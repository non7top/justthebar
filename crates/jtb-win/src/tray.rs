// Derived from ManagedShell.WindowsTray/TrayService.cs (Apache-2.0), ported to Rust.
use std::cell::RefCell;
use std::sync::atomic::{AtomicIsize, Ordering};

use jtb_core::traypacket::{parse_appbar_call, AppBarCall, ABM_GETTASKBARPOS};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HANDLE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{SHLockShared, SHUnlockShared};
use windows::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, CreateWindowExW, DefWindowProcW, DestroyWindow, FindWindowExW,
    FindWindowW, GetSystemMetrics, GetWindowLongPtrW, GetWindowThreadProcessId, IsWindow,
    PostMessageW, RegisterClassW, RegisterWindowMessageW, SendMessageW, SendNotifyMessageW,
    SetWindowLongPtrW, SetWindowPos, CS_DBLCLKS, GWL_STYLE, HWND_BOTTOM, HWND_BROADCAST,
    HWND_TOPMOST, SM_CXSCREEN, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, WINDOWPOS,
    WM_ACTIVATEAPP, WM_COMMAND, WM_COPYDATA, WM_USER, WM_WINDOWPOSCHANGED, WNDCLASSW, WS_CHILD,
    WS_CLIPCHILDREN, WS_CLIPSIBLINGS, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};

/// The `Shell_TrayWnd` window of this process, 0 when we do not hold the role.
static OURS: AtomicIsize = AtomicIsize::new(0);

/// Explorer posts this one to its own tray window; sending it blocks both sides.
const POSTED_MESSAGE: u32 = WM_USER + 372;

type Handler = Box<dyn Fn(&[u8]) -> bool>;
type QueryHandler = Box<dyn Fn(&[u8]) -> Option<isize>>;
/// The edge (`ABE_*`) and screen rectangle to report as the taskbar position, if any.
type PosHandler = Box<dyn Fn() -> Option<(u32, RECT)>>;

thread_local! {
    static HANDLER: RefCell<Option<Handler>> = const { RefCell::new(None) };
    static QUERY_HANDLER: RefCell<Option<QueryHandler>> = const { RefCell::new(None) };
    static POS_HANDLER: RefCell<Option<PosHandler>> = const { RefCell::new(None) };
}

/// Writes the answer to an `ABM_GETTASKBARPOS` call into the caller's shared `APPBARDATAV2`
/// (`uEdge` at byte 12, `rc` at byte 16).
fn answer_taskbar_pos(call: AppBarCall, edge: u32, rc: RECT) -> bool {
    unsafe {
        let data = SHLockShared(HANDLE(call.shared as usize as *mut _), call.process);
        if data.is_null() {
            return false;
        }
        let bytes = data as *mut u8;
        (bytes.add(12) as *mut u32).write_unaligned(edge);
        for (i, value) in [rc.left, rc.top, rc.right, rc.bottom]
            .into_iter()
            .enumerate()
        {
            (bytes.add(16 + i * 4) as *mut i32).write_unaligned(value);
        }
        let _ = SHUnlockShared(data);
    }
    true
}

pub fn ours() -> Option<HWND> {
    let raw = OURS.load(Ordering::SeqCst);
    (raw != 0).then_some(HWND(raw as *mut _))
}

pub fn is_active() -> bool {
    ours().is_some()
}

/// The `Shell_TrayWnd` that is not ours (Explorer's).
pub fn explorer_tray() -> Option<HWND> {
    let own = ours();
    let mut after: Option<HWND> = None;
    loop {
        let next = unsafe { FindWindowExW(None, after, w!("Shell_TrayWnd"), PCWSTR::null()).ok() }?;
        if Some(next) != own {
            return Some(next);
        }
        after = Some(next);
    }
}

fn forward(msg: u32, wparam: WPARAM, lparam: LPARAM, fallback: HWND) -> LRESULT {
    match explorer_tray() {
        Some(target) if msg == POSTED_MESSAGE => unsafe {
            let _ = PostMessageW(Some(target), msg, wparam, lparam);
            DefWindowProcW(fallback, msg, wparam, lparam)
        },
        Some(target) => unsafe { SendMessageW(target, msg, Some(wparam), Some(lparam)) },
        None => unsafe { DefWindowProcW(fallback, msg, wparam, lparam) },
    }
}

extern "system" fn tray_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_COPYDATA => {
            // dwData 1 is a notify icon packet, 0 an AppBar call (ours included), 3 an icon
            // rectangle query; only the first is ours to handle.
            let data = lparam.0 as *const COPYDATASTRUCT;
            if !data.is_null() {
                let data = unsafe { &*data };
                if data.dwData == 1 && !data.lpData.is_null() {
                    let bytes = unsafe {
                        std::slice::from_raw_parts(data.lpData as *const u8, data.cbData as usize)
                    };
                    if HANDLER.with_borrow(|h| h.as_ref().is_some_and(|h| h(bytes))) {
                        return LRESULT(1);
                    }
                }
                if data.dwData == 0 && !data.lpData.is_null() {
                    let bytes = unsafe {
                        std::slice::from_raw_parts(data.lpData as *const u8, data.cbData as usize)
                    };
                    if let Some(call) = parse_appbar_call(bytes) {
                        if call.message == ABM_GETTASKBARPOS {
                            let pos = POS_HANDLER.with_borrow(|h| h.as_ref().and_then(|h| h()));
                            if let Some((edge, rc)) = pos {
                                if answer_taskbar_pos(call, edge, rc) {
                                    return LRESULT(1);
                                }
                            }
                        }
                    }
                }
                if data.dwData == 3 && !data.lpData.is_null() {
                    let bytes = unsafe {
                        std::slice::from_raw_parts(data.lpData as *const u8, data.cbData as usize)
                    };
                    if let Some(answer) =
                        QUERY_HANDLER.with_borrow(|h| h.as_ref().and_then(|h| h(bytes)))
                    {
                        return LRESULT(answer);
                    }
                }
            }
            forward(msg, wparam, lparam, hwnd)
        }
        WM_WINDOWPOSCHANGED => {
            let pos = unsafe { &*(lparam.0 as *const WINDOWPOS) };
            if pos.flags.0 & SWP_SHOWWINDOW.0 != 0 {
                unsafe {
                    let style = GetWindowLongPtrW(hwnd, GWL_STYLE) & !(WS_VISIBLE.0 as isize);
                    SetWindowLongPtrW(hwnd, GWL_STYLE, style);
                }
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_ACTIVATEAPP | WM_COMMAND => forward(msg, wparam, lparam, hwnd),
        _ if msg >= WM_USER => forward(msg, wparam, lparam, hwnd),
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

extern "system" fn notify_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn send_taskbar_created() {
    let msg = unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) };
    if msg != 0 {
        unsafe {
            let _ = SendNotifyMessageW(HWND_BROADCAST, msg, WPARAM(0), LPARAM(0));
        }
    }
}

fn set_bottom(hwnd: HWND) {
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_BOTTOM),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

fn set_topmost(hwnd: HWND) {
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

/// Takes the tray role: creates our `Shell_TrayWnd` above Explorer's, which goes to the bottom,
/// and tells every app to send its icons again. `handler` gets the notify icon packets (the
/// `SHELLTRAYDATA` bytes) and returns whether it took the packet. `query` gets the icon rectangle
/// queries and returns the answer, or none to leave it to Explorer. `taskbar_pos` answers the
/// AppBar call that asks where the taskbar is. Everything else goes to Explorer's window.
pub fn create(
    handler: impl Fn(&[u8]) -> bool + 'static,
    query: impl Fn(&[u8]) -> Option<isize> + 'static,
    taskbar_pos: impl Fn() -> Option<(u32, RECT)> + 'static,
) -> Result<(), String> {
    if is_active() {
        return Ok(());
    }
    let instance = unsafe { GetModuleHandleW(None) }.map_err(|e| e.to_string())?;
    let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let tray_class = WNDCLASSW {
        style: CS_DBLCLKS,
        hInstance: instance.into(),
        lpszClassName: w!("Shell_TrayWnd"),
        lpfnWndProc: Some(tray_proc),
        ..Default::default()
    };
    let notify_class = WNDCLASSW {
        style: CS_DBLCLKS,
        hInstance: instance.into(),
        lpszClassName: w!("TrayNotifyWnd"),
        lpfnWndProc: Some(notify_proc),
        ..Default::default()
    };
    // A second registration after a release fails with "class exists", which is fine.
    unsafe {
        RegisterClassW(&tray_class);
        RegisterClassW(&notify_class);
    }
    HANDLER.with_borrow_mut(|h| *h = Some(Box::new(handler)));
    QUERY_HANDLER.with_borrow_mut(|h| *h = Some(Box::new(query)));
    POS_HANDLER.with_borrow_mut(|h| *h = Some(Box::new(taskbar_pos)));
    let tray = unsafe {
        CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            w!("Shell_TrayWnd"),
            w!(""),
            WS_POPUP | WS_CLIPCHILDREN | WS_CLIPSIBLINGS,
            0,
            0,
            screen_width,
            23,
            None,
            None,
            Some(instance.into()),
            None,
        )
    }
    .map_err(|e| format!("creating Shell_TrayWnd: {e}"))?;
    unsafe {
        let _ = CreateWindowExW(
            Default::default(),
            w!("TrayNotifyWnd"),
            w!(""),
            WS_CHILD | WS_CLIPCHILDREN | WS_CLIPSIBLINGS,
            0,
            0,
            screen_width,
            23,
            Some(tray),
            None,
            Some(instance.into()),
            None,
        );
    }
    OURS.store(tray.0 as isize, Ordering::SeqCst);
    if let Some(explorer) = explorer_tray() {
        set_bottom(explorer);
    }
    set_topmost(tray);
    send_taskbar_created();
    Ok(())
}

/// Whether the window that owns a tray icon still exists.
pub fn owner_alive(owner: isize) -> bool {
    unsafe { IsWindow(Some(HWND(owner as *mut _))) }.as_bool()
}

/// Lets the owner's process take the foreground, so menus it opens close when clicked away.
pub fn allow_foreground(owner: isize) {
    let mut pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(HWND(owner as *mut _), Some(&mut pid));
        if pid != 0 {
            let _ = AllowSetForegroundWindow(pid);
        }
    }
}

/// Sends an icon's callback message to its owner without waiting for the app to handle it.
pub fn notify_owner(owner: isize, callback_message: u32, wparam: usize, lparam: isize) -> bool {
    unsafe {
        SendNotifyMessageW(
            HWND(owner as *mut _),
            callback_message,
            WPARAM(wparam),
            LPARAM(lparam),
        )
    }
    .is_ok()
}

/// Called on a timer: puts our window back on top if another `Shell_TrayWnd` got above it.
/// Returns true when it had to.
pub fn keep_on_top() -> bool {
    let Some(own) = ours() else { return false };
    let top = unsafe { FindWindowW(w!("Shell_TrayWnd"), PCWSTR::null()).ok() };
    if top == Some(own) {
        return false;
    }
    set_topmost(own);
    true
}

/// Gives the tray role back: destroys our window, raises Explorer's and tells every app to send
/// its icons again. With `force` the broadcast also goes out when we never held the role, which
/// is what recovers after a crash.
pub fn give_back(force: bool) -> bool {
    let held = OURS.swap(0, Ordering::SeqCst);
    if held != 0 {
        let hwnd = HWND(held as *mut _);
        if unsafe { IsWindow(Some(hwnd)) }.as_bool() {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }
        HANDLER.with_borrow_mut(|h| *h = None);
        QUERY_HANDLER.with_borrow_mut(|h| *h = None);
        POS_HANDLER.with_borrow_mut(|h| *h = None);
    }
    if held == 0 && !force {
        return false;
    }
    if let Some(explorer) = explorer_tray() {
        set_topmost(explorer);
    }
    send_taskbar_created();
    true
}
