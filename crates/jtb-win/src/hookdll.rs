//! Talks to `jtb_hook.dll` inside `ShellHost.exe`, which places the quick settings flyout before it
//! is shown (see the `jtb-hook` crate for the protocol).
use std::path::PathBuf;

use windows::core::{s, w, HSTRING};
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

fn message() -> u32 {
    unsafe { RegisterWindowMessageW(w!("JustTheBar.FlyoutPlacement")) }
}

fn send(window: HWND, command: usize, lparam: isize) -> isize {
    unsafe {
        SendMessageW(
            window,
            message(),
            Some(WPARAM(command)),
            Some(LPARAM(lparam)),
        )
    }
    .0
}

/// A copy of the DLL under a name made from its content, so a rebuilt DLL never has to replace one
/// that `ShellHost.exe` still has loaded.
fn staged_dll() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("own path: {e}"))?;
    let source = exe.with_file_name("jtb_hook.dll");
    let bytes = std::fs::read(&source).map_err(|e| format!("{}: {e}", source.display()))?;
    let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    });
    let staged = std::env::temp_dir().join(format!("jtb-hook-{hash:016x}.dll"));
    if !staged.exists() {
        std::fs::write(&staged, &bytes).map_err(|e| format!("{}: {e}", staged.display()))?;
    }
    Ok(staged)
}

/// Makes sure the flyout window runs the subclass; true when it does.
pub fn attach(window: HWND) -> Result<bool, String> {
    if send(window, PING, 0) == 1 {
        return Ok(true);
    }
    let path = staged_dll()?;
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
    send(window, ATTACH, 0);
    let _ = unsafe { UnhookWindowsHookEx(hook) };
    Ok(send(window, PING, 0) == 1)
}

/// The next time the flyout is positioned, it goes to (`x`, `y`).
pub fn set_position(window: HWND, x: i32, y: i32) {
    send(
        window,
        SET,
        ((i64::from(y) << 32) | i64::from(x as u32)) as isize,
    );
}

pub fn clear(window: HWND) {
    send(window, CLEAR, 0);
}
