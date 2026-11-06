//! Talks to `jtb_hook.dll` inside `ShellHost.exe`, which places the quick settings flyout before it
//! is shown (see the `jtb-hook` crate for the protocol).
use std::cell::RefCell;
use std::path::PathBuf;

use windows::core::{s, HSTRING, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, WPARAM};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowThreadProcessId, RegisterWindowMessageW, SendMessageW, SetWindowsHookExW,
    UnhookWindowsHookEx, HOOKPROC, WH_CALLWNDPROC,
};

const ATTACH: usize = 1;
const SET: usize = 2;
const CLEAR: usize = 3;
const PING: usize = 4;
const LIMITS: usize = 5;
const HALVES: usize = 6;

/// The staged DLL and the message its subclass listens to.
struct Staged {
    path: PathBuf,
    message: u32,
}

thread_local! {
    static STAGED: RefCell<Option<Staged>> = const { RefCell::new(None) };
}

fn register(name: &str) -> u32 {
    let wide: Vec<u16> = format!("{name}\0").encode_utf16().collect();
    unsafe { RegisterWindowMessageW(PCWSTR(wide.as_ptr())) }
}

fn send(window: HWND, message: u32, command: usize, lparam: isize) -> isize {
    unsafe { SendMessageW(window, message, Some(WPARAM(command)), Some(LPARAM(lparam))) }.0
}

/// A copy of the DLL under a name made from its content, so a rebuilt DLL never has to replace one
/// that `ShellHost.exe` still has loaded; the name also tells the DLL which message to use.
fn staged() -> Result<(PathBuf, u32), String> {
    if let Some(known) = STAGED.with_borrow(|s| s.as_ref().map(|s| (s.path.clone(), s.message))) {
        return Ok(known);
    }
    let exe = std::env::current_exe().map_err(|e| format!("own path: {e}"))?;
    let source = exe.with_file_name("jtb_hook.dll");
    let bytes = std::fs::read(&source).map_err(|e| format!("{}: {e}", source.display()))?;
    let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    });
    let path = std::env::temp_dir().join(format!("jtb-hook-{hash:016x}.dll"));
    if !path.exists() {
        std::fs::write(&path, &bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    let message = register(&format!("JustTheBar.FlyoutPlacement.{hash:016x}"));
    STAGED.set(Some(Staged {
        path: path.clone(),
        message,
    }));
    Ok((path, message))
}

/// Switches off the target an older build of the DLL may still hold in `ShellHost.exe`.
fn release_legacy(window: HWND) {
    send(window, register("JustTheBar.FlyoutPlacement"), CLEAR, 0);
}

/// Makes sure the flyout window runs this build's subclass; true when it does.
pub fn attach(window: HWND) -> Result<bool, String> {
    let (path, message) = staged()?;
    if send(window, message, PING, 0) == 1 {
        return Ok(true);
    }
    release_legacy(window);
    let module = unsafe { LoadLibraryW(&HSTRING::from(path.as_os_str())) }
        .map_err(|e| format!("load {}: {e}", path.display()))?;
    let hook_proc: HOOKPROC = unsafe {
        std::mem::transmute(
            GetProcAddress(module, s!("jtb_hook_proc")).ok_or("jtb_hook_proc not found")?,
        )
    };
    let thread = unsafe { GetWindowThreadProcessId(window, None) };
    let hook =
        unsafe { SetWindowsHookExW(WH_CALLWNDPROC, hook_proc, Some(HINSTANCE(module.0)), thread) }
            .map_err(|e| format!("hook thread {thread}: {e}"))?;
    send(window, message, ATTACH, 0);
    let _ = unsafe { UnhookWindowsHookEx(hook) };
    Ok(send(window, message, PING, 0) == 1)
}

/// Where the flyout goes the next time it is positioned: `x_base` minus `x_halves` halves of its
/// width, kept between `x_low` and `x_high` minus its width; `y_base` minus `y_halves` halves of
/// its height, or the y it asks for when `None`.
#[derive(Clone, Copy)]
pub struct Placement {
    pub x_base: i32,
    pub x_halves: i32,
    pub x_low: i32,
    pub x_high: i32,
    pub y_base: Option<i32>,
    pub y_halves: i32,
}

pub fn set_position(window: HWND, placement: Placement) {
    if let Ok((_, message)) = staged() {
        let pack = |low: i32, high: i32| ((i64::from(high) << 32) | i64::from(low as u32)) as isize;
        send(
            window,
            message,
            LIMITS,
            pack(placement.x_low, placement.x_high),
        );
        send(
            window,
            message,
            HALVES,
            pack(placement.x_halves, placement.y_halves),
        );
        send(
            window,
            message,
            SET,
            pack(placement.x_base, placement.y_base.unwrap_or(i32::MIN)),
        );
    }
}

pub fn clear(window: HWND) {
    if let Ok((_, message)) = staged() {
        send(window, message, CLEAR, 0);
    }
}
