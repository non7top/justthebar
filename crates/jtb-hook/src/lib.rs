//! Loaded into `ShellHost.exe` by `jtb-app` to place the quick settings flyout before it is shown.
//!
//! The app installs `jtb_hook_proc` as a one-shot WH_CALLWNDPROC hook on the flyout's thread and
//! sends the attach command (1); the hook subclasses the window and the app removes the hook again.
//! From then on the app talks to the subclass directly with the same registered message: 2 sets
//! the base position (x in the low 32 bits of lparam, y in the high 32, `i32::MIN` to keep the y
//! the window asks for), 3 clears it, 5 sets the x limits (low and high 32 bits), 6 sets how many
//! halves of the window's width and height are subtracted from the base (x in the low 32 bits, y
//! in the high 32), and any command is answered with 1, so the app can tell the subclass is
//! there. The window's size is read when it is placed, because a new flyout window has none
//! before its first show.
//!
//! A loaded DLL stays in `ShellHost.exe`, so every build is a file named `jtb-hook-<hash>.dll` and
//! takes its message name and subclass id from that hash: a newer build attaches next to an older
//! one instead of talking to it.
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::OnceLock;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::System::LibraryLoader::{
    GetModuleFileNameW, GetModuleHandleExW, GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
    GET_MODULE_HANDLE_EX_FLAG_PIN,
};
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetClassNameW, GetWindowRect, RegisterWindowMessageW, CWPSTRUCT, SWP_NOMOVE,
    SWP_NOSIZE, WINDOWPOS, WM_NCDESTROY, WM_WINDOWPOSCHANGING,
};

const ATTACH: usize = 1;
const SET: usize = 2;
const CLEAR: usize = 3;
const LIMITS: usize = 5;
const HALVES: usize = 6;
/// Sent as y when only x is wanted.
const KEEP: i32 = i32::MIN;

static ANCHOR: u8 = 0;
static ACTIVE: AtomicBool = AtomicBool::new(false);
static X: AtomicI32 = AtomicI32::new(0);
static Y: AtomicI32 = AtomicI32::new(0);
static LOW_X: AtomicI32 = AtomicI32::new(i32::MIN);
static HIGH_X: AtomicI32 = AtomicI32::new(i32::MAX);
static HALVES_X: AtomicI32 = AtomicI32::new(0);
static HALVES_Y: AtomicI32 = AtomicI32::new(0);

/// The registered message and the subclass id of this build.
fn identity() -> (u32, usize) {
    static IDENTITY: OnceLock<(u32, usize)> = OnceLock::new();
    *IDENTITY.get_or_init(|| {
        let mut module = HMODULE::default();
        let mut path = [0u16; 520];
        let found = unsafe {
            GetModuleHandleExW(
                GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS,
                PCWSTR(&ANCHOR as *const u8 as *const u16),
                &mut module,
            )
        }
        .is_ok();
        let len = if found {
            (unsafe { GetModuleFileNameW(Some(module), &mut path) }) as usize
        } else {
            0
        };
        let file = String::from_utf16_lossy(&path[..len.min(path.len())]);
        let hash = file
            .rsplit('\\')
            .next()
            .and_then(|name| name.strip_prefix("jtb-hook-"))
            .and_then(|name| name.strip_suffix(".dll"))
            .unwrap_or("unstaged")
            .to_string();
        let id = u64::from_str_radix(&hash, 16).unwrap_or(0x4A54_4231) as usize;
        let name: Vec<u16> = format!("JustTheBar.FlyoutPlacement.{hash}\0")
            .encode_utf16()
            .collect();
        (unsafe { RegisterWindowMessageW(PCWSTR(name.as_ptr())) }, id)
    })
}

fn is_flyout(window: HWND) -> bool {
    let mut class = [0u16; 32];
    let len = unsafe { GetClassNameW(window, &mut class) };
    len > 0 && String::from_utf16_lossy(&class[..len as usize]) == "ControlCenterWindow"
}

unsafe extern "system" fn subclass(
    window: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if msg == identity().0 {
        match wparam.0 {
            SET => {
                X.store(lparam.0 as i32, Ordering::Relaxed);
                Y.store((lparam.0 >> 32) as i32, Ordering::Relaxed);
                ACTIVE.store(true, Ordering::Release);
            }
            LIMITS => {
                LOW_X.store(lparam.0 as i32, Ordering::Relaxed);
                HIGH_X.store((lparam.0 >> 32) as i32, Ordering::Relaxed);
            }
            HALVES => {
                HALVES_X.store(lparam.0 as i32, Ordering::Relaxed);
                HALVES_Y.store((lparam.0 >> 32) as i32, Ordering::Relaxed);
            }
            CLEAR => ACTIVE.store(false, Ordering::Release),
            _ => {}
        }
        return LRESULT(1);
    }
    if msg == WM_NCDESTROY {
        let _ = unsafe { RemoveWindowSubclass(window, Some(subclass), identity().1) };
    }
    let result = unsafe { DefSubclassProc(window, msg, wparam, lparam) };
    // The window's own procedure applies its size and position rules first; only then is the
    // requested position replaced.
    if msg == WM_WINDOWPOSCHANGING && ACTIVE.load(Ordering::Acquire) && lparam.0 != 0 {
        let pos = unsafe { &mut *(lparam.0 as *mut WINDOWPOS) };
        if pos.flags.0 & SWP_NOMOVE.0 == 0 {
            let mut current = RECT::default();
            let _ = unsafe { GetWindowRect(window, &mut current) };
            let width = if pos.flags.0 & SWP_NOSIZE.0 == 0 && pos.cx > 0 {
                pos.cx
            } else {
                current.right - current.left
            };
            let height = if pos.flags.0 & SWP_NOSIZE.0 == 0 && pos.cy > 0 {
                pos.cy
            } else {
                current.bottom - current.top
            };
            let x = X.load(Ordering::Relaxed) - width * HALVES_X.load(Ordering::Relaxed) / 2;
            pos.x = x
                .min(HIGH_X.load(Ordering::Relaxed).saturating_sub(width))
                .max(LOW_X.load(Ordering::Relaxed));
            let y = Y.load(Ordering::Relaxed);
            if y != KEEP {
                pos.y = y - height * HALVES_Y.load(Ordering::Relaxed) / 2;
            }
        }
    }
    result
}

fn attach(window: HWND) {
    if !is_flyout(window) {
        return;
    }
    // The subclass procedure lives in this DLL, so it must stay loaded after the hook is removed.
    let mut pinned = HMODULE::default();
    let _ = unsafe {
        GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_PIN,
            PCWSTR(&ANCHOR as *const u8 as *const u16),
            &mut pinned,
        )
    };
    let _ = unsafe { SetWindowSubclass(window, Some(subclass), identity().1, 0) };
}

/// # Safety
/// Called by Windows as a WH_CALLWNDPROC hook; `lparam` points to a `CWPSTRUCT` when `code` is 0.
#[no_mangle]
pub unsafe extern "system" fn jtb_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == 0 && lparam.0 != 0 {
        let call = unsafe { &*(lparam.0 as *const CWPSTRUCT) };
        if call.message == identity().0 && call.wParam.0 == ATTACH {
            attach(call.hwnd);
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}
