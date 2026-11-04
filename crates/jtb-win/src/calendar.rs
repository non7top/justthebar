//! A popup window hosting the Windows month calendar control (`SysMonthCal32` from `comctl32.dll`),
//! opened next to the clock widget.
use std::cell::Cell;
use std::mem::size_of;
use std::time::{Duration, Instant};

use jtb_core::Edge;
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{COLOR_WINDOW, HBRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{InitCommonControlsEx, ICC_DATE_CLASSES, INITCOMMONCONTROLSEX};
use windows::Win32::UI::Input::KeyboardAndMouse::{SetFocus, VK_ESCAPE};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, IsChild, LoadCursorW, RegisterClassW,
    SendMessageW, SetWindowPos, ShowWindow, CS_DROPSHADOW, IDC_ARROW, SWP_NOACTIVATE, SWP_NOZORDER,
    SW_SHOW, WA_INACTIVE, WM_ACTIVATE, WM_DESTROY, WM_KEYDOWN, WM_SETFOCUS, WNDCLASSW, WS_CHILD,
    WS_CLIPCHILDREN, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};

use crate::flyout::Anchor;

const CLASS_NAME: windows::core::PCWSTR = w!("JtbCalendarPopup");
const MONTHCAL_CLASS: windows::core::PCWSTR = w!("SysMonthCal32");
const MCM_FIRST: u32 = 0x1000;
const MCM_GETMINREQRECT: u32 = MCM_FIRST + 9;
const MCM_SETMONTHDELTA: u32 = MCM_FIRST + 20;
const MCM_SIZERECTTOMIN: u32 = MCM_FIRST + 29;

#[derive(Clone, Copy)]
struct State {
    popup: HWND,
    child: HWND,
}

thread_local! {
    static STATE: Cell<Option<State>> = const { Cell::new(None) };
    static LAST_CLOSED: Cell<Option<Instant>> = const { Cell::new(None) };
}

unsafe extern "system" fn popup_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_ACTIVATE => {
            let active = (wparam.0 & 0xffff) as u32;
            if active == WA_INACTIVE {
                let other = HWND(lparam.0 as *mut _);
                if other == hwnd || IsChild(hwnd, other).as_bool() {
                    return LRESULT(0);
                }
                let _ = DestroyWindow(hwnd);
                return LRESULT(0);
            }
        }
        WM_SETFOCUS => {
            if let Some(state) = STATE.get() {
                if state.popup == hwnd {
                    let _ = SetFocus(Some(state.child));
                }
            }
            return LRESULT(0);
        }
        WM_KEYDOWN => {
            if wparam.0 == VK_ESCAPE.0 as usize {
                let _ = DestroyWindow(hwnd);
                return LRESULT(0);
            }
        }
        WM_DESTROY => {
            LAST_CLOSED.set(Some(Instant::now()));
            STATE.set(None);
            return LRESULT(0);
        }
        _ => {}
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn register_class() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| unsafe {
        let _ = InitCommonControlsEx(&INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_DATE_CLASSES,
        });
        let Ok(instance) = GetModuleHandleW(None) else {
            return;
        };
        let class = WNDCLASSW {
            style: CS_DROPSHADOW,
            hInstance: instance.into(),
            lpszClassName: CLASS_NAME,
            lpfnWndProc: Some(popup_proc),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as isize as _),
            ..Default::default()
        };
        RegisterClassW(&class);
    });
}

/// Toggles the calendar popup next to the clock defined by `anchor`.
pub fn toggle(anchor: &Anchor, months: u32) {
    if let Some(state) = STATE.get() {
        unsafe {
            let _ = DestroyWindow(state.popup);
        }
        STATE.set(None);
        return;
    }
    if let Some(closed) = LAST_CLOSED.get() {
        if closed.elapsed() < Duration::from_millis(250) {
            return;
        }
    }
    register_class();

    unsafe {
        let Ok(instance) = GetModuleHandleW(None) else {
            return;
        };
        let popup = match CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            CLASS_NAME,
            w!("Calendar"),
            WS_POPUP | WS_CLIPCHILDREN,
            0,
            0,
            100,
            100,
            Some(anchor.window),
            None,
            Some(instance.into()),
            None,
        ) {
            Ok(hwnd) => hwnd,
            Err(_) => return,
        };

        let child = match CreateWindowExW(
            Default::default(),
            MONTHCAL_CLASS,
            None,
            WS_CHILD | WS_VISIBLE,
            0,
            0,
            0,
            0,
            Some(popup),
            None,
            Some(instance.into()),
            None,
        ) {
            Ok(hwnd) => hwnd,
            Err(_) => {
                let _ = DestroyWindow(popup);
                return;
            }
        };

        let mut req = RECT::default();
        SendMessageW(
            child,
            MCM_GETMINREQRECT,
            Some(WPARAM(0)),
            Some(LPARAM(&mut req as *mut _ as isize)),
        );
        let w = req.right - req.left;
        let h = req.bottom - req.top;

        let count = months.clamp(1, 12) as i32;
        let mut target = RECT {
            left: 0,
            top: 0,
            right: w + 16,
            bottom: h * count + 40,
        };
        SendMessageW(
            child,
            MCM_SIZERECTTOMIN,
            Some(WPARAM(0)),
            Some(LPARAM(&mut target as *mut _ as isize)),
        );
        let width = (target.right - target.left).max(w);
        let height = (target.bottom - target.top).max(h * count);

        let _ = SendMessageW(child, MCM_SETMONTHDELTA, Some(WPARAM(1)), None);

        let _ = SetWindowPos(
            child,
            None,
            0,
            0,
            width,
            height,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );

        const GAP: i32 = 8;
        let (mut x, mut y) = match anchor.edge {
            Edge::Bottom => (anchor.click.x - width / 2, anchor.bar.top - height - GAP),
            Edge::Top => (anchor.click.x - width / 2, anchor.bar.bottom + GAP),
            Edge::Left => (anchor.bar.right + GAP, anchor.click.y - height / 2),
            Edge::Right => (anchor.bar.left - width - GAP, anchor.click.y - height / 2),
        };

        // Clamp inside monitor bounds.
        x = x.clamp(
            anchor.monitor.left + GAP,
            anchor.monitor.right - width - GAP,
        );
        y = y.clamp(
            anchor.monitor.top + GAP,
            anchor.monitor.bottom - height - GAP,
        );

        let _ = SetWindowPos(
            popup,
            None,
            x,
            y,
            width,
            height,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );

        STATE.set(Some(State { popup, child }));
        let _ = ShowWindow(popup, SW_SHOW);
        let _ = SetFocus(Some(child));
    }
}
