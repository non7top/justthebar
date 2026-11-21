//! A low-level keyboard hook that reports a lone press of the Windows key and keeps it from
//! other programs, so the Start menu opens next to the bar's own Start button instead of where
//! Open-Shell or Windows would put it.
//!
//! The hook runs on its own thread so a busy bar thread cannot make Windows drop it. When the key
//! comes up without another key in between, the release is swallowed and replayed behind an
//! unassigned key press, which is how other programs learn it was not a lone press.
use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    VIRTUAL_KEY, VK_LWIN, VK_RWIN,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostMessageW, PostThreadMessageW,
    SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

/// A key that has no meaning, pressed between the Windows key's down and up.
const MASK_KEY: VIRTUAL_KEY = VIRTUAL_KEY(0xE8);

static THREAD: AtomicU32 = AtomicU32::new(0);
static TARGET: AtomicIsize = AtomicIsize::new(0);
static MESSAGE: AtomicU32 = AtomicU32::new(0);
static WIN_DOWN: AtomicBool = AtomicBool::new(false);
static CHORD: AtomicBool = AtomicBool::new(false);

fn key(code: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: code,
                dwFlags: flags,
                ..Default::default()
            },
        },
    }
}

unsafe extern "system" fn hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && lparam.0 != 0 {
        let event = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        if event.flags.0 & LLKHF_INJECTED.0 == 0 {
            let message = wparam.0 as u32;
            let down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
            let up = message == WM_KEYUP || message == WM_SYSKEYUP;
            let code = event.vkCode;
            if code == u32::from(VK_LWIN.0) || code == u32::from(VK_RWIN.0) {
                if down {
                    if !WIN_DOWN.swap(true, Ordering::SeqCst) {
                        CHORD.store(false, Ordering::SeqCst);
                    }
                } else if up
                    && WIN_DOWN.swap(false, Ordering::SeqCst)
                    && !CHORD.load(Ordering::SeqCst)
                {
                    let window = HWND(TARGET.load(Ordering::SeqCst) as *mut _);
                    let _ = unsafe {
                        PostMessageW(
                            Some(window),
                            MESSAGE.load(Ordering::SeqCst),
                            WPARAM(0),
                            LPARAM(0),
                        )
                    };
                    let inputs = [
                        key(MASK_KEY, KEYBD_EVENT_FLAGS(0)),
                        key(MASK_KEY, KEYEVENTF_KEYUP),
                        key(VIRTUAL_KEY(code as u16), KEYEVENTF_KEYUP),
                    ];
                    unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
                    return LRESULT(1);
                }
            } else if down && WIN_DOWN.load(Ordering::SeqCst) {
                CHORD.store(true, Ordering::SeqCst);
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// Starts watching the Windows key; `message` is posted to `window` on each lone press. Does
/// nothing when it is already running.
pub fn install(window: HWND, message: u32) -> Result<(), String> {
    if THREAD.load(Ordering::SeqCst) != 0 {
        return Ok(());
    }
    TARGET.store(window.0 as isize, Ordering::SeqCst);
    MESSAGE.store(message, Ordering::SeqCst);
    WIN_DOWN.store(false, Ordering::SeqCst);
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let module = unsafe { GetModuleHandleW(None) }.ok();
        let hooked = unsafe {
            SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(hook),
                module.map(|m| windows::Win32::Foundation::HINSTANCE(m.0)),
                0,
            )
        };
        match hooked {
            Ok(handle) => {
                THREAD.store(unsafe { GetCurrentThreadId() }, Ordering::SeqCst);
                let _ = sender.send(Ok(()));
                let mut msg = MSG::default();
                while unsafe { GetMessageW(&mut msg, None, 0, 0) }.0 > 0 {
                    unsafe {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
                let _ = unsafe { UnhookWindowsHookEx(handle) };
                THREAD.store(0, Ordering::SeqCst);
            }
            Err(error) => {
                let _ = sender.send(Err(error.to_string()));
            }
        }
    });
    receiver
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_else(|_| Err("the keyboard hook thread did not answer".to_string()))
}

/// Stops watching the Windows key.
pub fn uninstall() {
    let thread = THREAD.load(Ordering::SeqCst);
    if thread != 0 {
        let _ = unsafe { PostThreadMessageW(thread, WM_QUIT, WPARAM(0), LPARAM(0)) };
    }
}
