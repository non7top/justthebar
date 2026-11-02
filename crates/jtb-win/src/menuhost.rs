//! Runs Open-Shell's menu code inside this process, the way `StartMenu.exe -nohook` does, so the
//! menu placement can be given directly (`StartMenuParams`) instead of being worked out from
//! Explorer's hidden taskbar.
use std::cell::{Cell, RefCell};
use std::ffi::c_void;

use windows::core::{s, w, PCWSTR};
use windows::Win32::Foundation::{HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::System::LibraryLoader::{
    GetProcAddress, LoadLibraryExW, LOAD_WITH_ALTERED_SEARCH_PATH,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetClassNameW, SendMessageW, SetWindowsHookExW, UnhookWindowsHookEx, CWPSTRUCT,
    WH_CALLWNDPROC, WM_WINDOWPOSCHANGING,
};

#[link(name = "ole32")]
extern "system" {
    fn OleInitialize(reserved: *mut c_void) -> i32;
}

/// The placement Open-Shell takes instead of asking the taskbar (`MenuContainer.h`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct StartMenuParams {
    pub start_button: HWND,
    pub taskbar: HWND,
    pub start_button_rect: RECT,
    pub taskbar_rect: RECT,
    pub monitor_rect: RECT,
    pub edge: u32,
}

type ToggleFn = unsafe extern "C" fn(i32, bool) -> HWND;
type Proc = unsafe extern "system" fn() -> isize;

struct Host {
    toggle: ToggleFn,
    message: u32,
}

thread_local! {
    static HOST: RefCell<Option<Host>> = const { RefCell::new(None) };
    static FAILED: Cell<bool> = const { Cell::new(false) };
    static PENDING: Cell<Option<(u32, StartMenuParams)>> = const { Cell::new(None) };
}

fn load(dll: &str) -> Result<Host, String> {
    let wide: Vec<u16> = dll.encode_utf16().chain(Some(0)).collect();
    let module: HMODULE =
        unsafe { LoadLibraryExW(PCWSTR(wide.as_ptr()), None, LOAD_WITH_ALTERED_SEARCH_PATH) }
            .map_err(|e| format!("cannot load {dll}: {e}"))?;
    unsafe {
        let wait = GetProcAddress(module, s!("?WaitDllInitThread@@YAXXZ"));
        let init = GetProcAddress(module, s!("?InitManagers@@YAX_N@Z"));
        let toggle = GetProcAddress(module, s!("?ToggleStartMenu@@YAPEAUHWND__@@H_N@Z"));
        let (Some(wait), Some(init), Some(toggle)) = (wait, init, toggle) else {
            return Err("the library does not export the expected functions".to_string());
        };
        OleInitialize(std::ptr::null_mut());
        let wait = std::mem::transmute::<Proc, unsafe extern "C" fn()>(wait);
        let init = std::mem::transmute::<Proc, unsafe extern "C" fn(bool)>(init);
        wait();
        init(true);
        Ok(Host {
            toggle: std::mem::transmute::<Proc, ToggleFn>(toggle),
            message: windows::Win32::UI::WindowsAndMessaging::RegisterWindowMessageW(w!(
                "OpenShellMenu.StartMenuMsg"
            )),
        })
    }
}

extern "system" fn hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let call = unsafe { &*(lparam.0 as *const CWPSTRUCT) };
        if call.message == WM_WINDOWPOSCHANGING {
            if let Some((message, params)) = PENDING.get() {
                let mut class = [0u16; 64];
                let len = unsafe { GetClassNameW(call.hwnd, &mut class) } as usize;
                if String::from_utf16_lossy(&class[..len]) == "OpenShell.CMenuContainer" {
                    PENDING.set(None);
                    unsafe {
                        SendMessageW(
                            call.hwnd,
                            message,
                            Some(WPARAM(0)),
                            Some(LPARAM(&params as *const StartMenuParams as isize)),
                        );
                    }
                }
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// Toggles Open-Shell's menu in this process with the given placement. Loads the library from
/// `dll` on first use. Returns an error text when it cannot, so the caller can fall back.
pub fn toggle(dll: &str, params: StartMenuParams) -> Result<(), String> {
    if FAILED.get() {
        return Err("in-process menu failed earlier".to_string());
    }
    if HOST.with_borrow(|h| h.is_none()) {
        match load(dll) {
            Ok(host) => HOST.with_borrow_mut(|h| *h = Some(host)),
            Err(e) => {
                FAILED.set(true);
                return Err(e);
            }
        }
    }
    let (toggle, message) = HOST.with_borrow(|h| {
        let host = h.as_ref().expect("host loaded");
        (host.toggle, host.message)
    });
    PENDING.set(Some((message, params)));
    let thread = unsafe { windows::Win32::System::Threading::GetCurrentThreadId() };
    let hook = unsafe { SetWindowsHookExW(WH_CALLWNDPROC, Some(hook), None, thread) }
        .map_err(|e| format!("cannot hook the menu window: {e}"))?;
    unsafe { toggle(-1, false) };
    PENDING.set(None);
    unsafe {
        let _ = UnhookWindowsHookEx(hook);
    }
    Ok(())
}
