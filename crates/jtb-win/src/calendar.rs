//! A popup window hosting the Windows month calendar control (`SysMonthCal32` from `comctl32.dll`),
//! opened next to the clock widget.
use std::cell::Cell;
use std::mem::size_of;
use std::time::{Duration, Instant};

use jtb_core::Edge;
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, SYSTEMTIME, WPARAM};
use windows::Win32::Graphics::Gdi::{COLOR_WINDOW, HBRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::UI::Controls::{InitCommonControlsEx, ICC_DATE_CLASSES, INITCOMMONCONTROLSEX};
use windows::Win32::UI::Input::KeyboardAndMouse::{SetFocus, VK_ESCAPE};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, IsChild, LoadCursorW, RegisterClassW,
    SendMessageW, SetWindowPos, ShowWindow, BS_PUSHBUTTON, CS_DROPSHADOW, HMENU, IDC_ARROW,
    SWP_NOACTIVATE, SWP_NOZORDER, SW_SHOW, WA_INACTIVE, WINDOW_STYLE, WM_ACTIVATE, WM_COMMAND,
    WM_DESTROY, WM_GETFONT, WM_KEYDOWN, WM_MOUSEWHEEL, WM_SETFOCUS, WM_SETFONT, WNDCLASSW,
    WS_CHILD, WS_CLIPCHILDREN, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_TABSTOP, WS_VISIBLE,
};

use crate::flyout::Anchor;

const CLASS_NAME: windows::core::PCWSTR = w!("JtbCalendarPopup");
const MONTHCAL_CLASS: windows::core::PCWSTR = w!("SysMonthCal32");
const BUTTON_CLASS: windows::core::PCWSTR = w!("BUTTON");

const MCM_FIRST: u32 = 0x1000;
const MCM_SETCURSEL: u32 = MCM_FIRST + 2;
const MCM_GETMINREQRECT: u32 = MCM_FIRST + 9;
const MCM_SETMONTHDELTA: u32 = MCM_FIRST + 20;
const MCM_SIZERECTTOMIN: u32 = MCM_FIRST + 29;

const ID_PREV: usize = 101;
const ID_TODAY: usize = 102;
const ID_NEXT: usize = 103;

const HEADER_HEIGHT: i32 = 32;

#[derive(Clone, Copy)]
struct State {
    popup: HWND,
    child: HWND,
    top_month: SYSTEMTIME,
}

thread_local! {
    static STATE: Cell<Option<State>> = const { Cell::new(None) };
    static LAST_CLOSED: Cell<Option<Instant>> = const { Cell::new(None) };
}

fn set_top_month(child: HWND, top: &SYSTEMTIME) {
    unsafe {
        // Jumping far away forces SysMonthCal32 to bring the target month into view
        // at the top-left (first) position when jumping back.
        let mut far = *top;
        far.wYear = far.wYear.wrapping_add(2);
        let _ = SendMessageW(
            child,
            MCM_SETCURSEL,
            Some(WPARAM(0)),
            Some(LPARAM(&mut far as *mut _ as isize)),
        );
        let mut target = *top;
        let _ = SendMessageW(
            child,
            MCM_SETCURSEL,
            Some(WPARAM(0)),
            Some(LPARAM(&mut target as *mut _ as isize)),
        );
    }
}

fn prev_month() {
    if let Some(mut state) = STATE.get() {
        if state.top_month.wMonth == 1 {
            state.top_month.wMonth = 12;
            state.top_month.wYear = state.top_month.wYear.saturating_sub(1);
        } else {
            state.top_month.wMonth -= 1;
        }
        state.top_month.wDay = 1;
        set_top_month(state.child, &state.top_month);
        STATE.set(Some(state));
    }
}

fn next_month() {
    if let Some(mut state) = STATE.get() {
        if state.top_month.wMonth == 12 {
            state.top_month.wMonth = 1;
            state.top_month.wYear = state.top_month.wYear.saturating_add(1);
        } else {
            state.top_month.wMonth += 1;
        }
        state.top_month.wDay = 1;
        set_top_month(state.child, &state.top_month);
        STATE.set(Some(state));
    }
}

fn today_month() {
    if let Some(mut state) = STATE.get() {
        let now = unsafe { GetLocalTime() };
        state.top_month = now;
        set_top_month(state.child, &state.top_month);
        STATE.set(Some(state));
    }
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
        WM_COMMAND => {
            let id = wparam.0 & 0xffff;
            match id {
                ID_PREV => prev_month(),
                ID_TODAY => today_month(),
                ID_NEXT => next_month(),
                _ => {}
            }
            return LRESULT(0);
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam.0 >> 16) & 0xffff) as i16;
            if delta > 0 {
                prev_month();
            } else if delta < 0 {
                next_month();
            }
            return LRESULT(0);
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
            HEADER_HEIGHT,
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
            HEADER_HEIGHT,
            width,
            height,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );

        let btn_style = WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as u32);
        let btn_prev = CreateWindowExW(
            Default::default(),
            BUTTON_CLASS,
            w!("<"),
            btn_style,
            4,
            4,
            36,
            HEADER_HEIGHT - 8,
            Some(popup),
            Some(HMENU(ID_PREV as isize as _)),
            Some(instance.into()),
            None,
        );
        let btn_today = CreateWindowExW(
            Default::default(),
            BUTTON_CLASS,
            w!("Today"),
            btn_style,
            (width - 70) / 2,
            4,
            70,
            HEADER_HEIGHT - 8,
            Some(popup),
            Some(HMENU(ID_TODAY as isize as _)),
            Some(instance.into()),
            None,
        );
        let btn_next = CreateWindowExW(
            Default::default(),
            BUTTON_CLASS,
            w!(">"),
            btn_style,
            width - 40,
            4,
            36,
            HEADER_HEIGHT - 8,
            Some(popup),
            Some(HMENU(ID_NEXT as isize as _)),
            Some(instance.into()),
            None,
        );

        let font = SendMessageW(child, WM_GETFONT, None, None);
        if font.0 != 0 {
            if let Ok(b) = btn_prev {
                let _ = SendMessageW(
                    b,
                    WM_SETFONT,
                    Some(WPARAM(font.0 as usize)),
                    Some(LPARAM(1)),
                );
            }
            if let Ok(b) = btn_today {
                let _ = SendMessageW(
                    b,
                    WM_SETFONT,
                    Some(WPARAM(font.0 as usize)),
                    Some(LPARAM(1)),
                );
            }
            if let Ok(b) = btn_next {
                let _ = SendMessageW(
                    b,
                    WM_SETFONT,
                    Some(WPARAM(font.0 as usize)),
                    Some(LPARAM(1)),
                );
            }
        }

        let total_height = height + HEADER_HEIGHT;
        const GAP: i32 = 8;
        let (mut x, mut y) = match anchor.edge {
            Edge::Bottom => (
                anchor.click.x - width / 2,
                anchor.bar.top - total_height - GAP,
            ),
            Edge::Top => (anchor.click.x - width / 2, anchor.bar.bottom + GAP),
            Edge::Left => (anchor.bar.right + GAP, anchor.click.y - total_height / 2),
            Edge::Right => (
                anchor.bar.left - width - GAP,
                anchor.click.y - total_height / 2,
            ),
        };

        // Clamp inside monitor bounds.
        x = x.clamp(
            anchor.monitor.left + GAP,
            anchor.monitor.right - width - GAP,
        );
        y = y.clamp(
            anchor.monitor.top + GAP,
            anchor.monitor.bottom - total_height - GAP,
        );

        let _ = SetWindowPos(
            popup,
            None,
            x,
            y,
            width,
            total_height,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );

        let now = GetLocalTime();
        set_top_month(child, &now);

        STATE.set(Some(State {
            popup,
            child,
            top_month: now,
        }));
        let _ = ShowWindow(popup, SW_SHOW);
        let _ = SetFocus(Some(child));
    }
}
