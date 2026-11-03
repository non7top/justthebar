//! One tracking tooltip shared by all bars, shown at a position the caller picks.
use std::cell::Cell;
use std::mem::{offset_of, size_of};

use windows::core::PWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    DrawTextW, GetDC, GetStockObject, ReleaseDC, SelectObject, DEFAULT_GUI_FONT, DT_CALCRECT,
    DT_NOPREFIX, DT_WORDBREAK, HGDIOBJ,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    InitCommonControlsEx, ICC_WIN95_CLASSES, INITCOMMONCONTROLSEX, TOOLTIPS_CLASSW, TTF_ABSOLUTE,
    TTF_TRACK, TTM_ADDTOOLW, TTM_ADJUSTRECT, TTM_DELTOOLW, TTM_SETMAXTIPWIDTH, TTM_TRACKACTIVATE,
    TTM_TRACKPOSITION, TTM_UPDATETIPTEXTW, TTS_ALWAYSTIP, TTS_NOPREFIX, TTTOOLINFOW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, SendMessageW, WINDOW_STYLE, WM_GETFONT, WS_EX_TOPMOST, WS_POPUP,
};

const MAX_WIDTH: isize = 320;
const TOOL_ID: usize = 1;

struct Tip {
    window: HWND,
    /// The bar window the tool is registered for.
    owner: HWND,
}

thread_local! {
    static TIP: Cell<Option<Tip>> = const { Cell::new(None) };
}

fn tool_info(owner: HWND, text: Option<&mut Vec<u16>>) -> TTTOOLINFOW {
    TTTOOLINFOW {
        // The size without the newer fields, which older common controls refuse.
        cbSize: (offset_of!(TTTOOLINFOW, lpszText) + size_of::<PWSTR>()) as u32,
        uFlags: TTF_TRACK | TTF_ABSOLUTE,
        hwnd: owner,
        uId: TOOL_ID,
        lpszText: text.map_or(PWSTR::null(), |t| PWSTR(t.as_mut_ptr())),
        ..Default::default()
    }
}

fn send(
    window: HWND,
    message: u32,
    wparam: usize,
    info: Option<&TTTOOLINFOW>,
    lparam: isize,
) -> isize {
    let lparam = info.map_or(lparam, |i| i as *const TTTOOLINFOW as isize);
    unsafe { SendMessageW(window, message, Some(WPARAM(wparam)), Some(LPARAM(lparam))) }.0
}

fn create(owner: HWND) -> Option<HWND> {
    unsafe {
        let init = INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_WIN95_CLASSES,
        };
        let _ = InitCommonControlsEx(&init);
        let window = CreateWindowExW(
            WS_EX_TOPMOST,
            TOOLTIPS_CLASSW,
            None,
            WS_POPUP | WINDOW_STYLE(TTS_NOPREFIX | TTS_ALWAYSTIP),
            0,
            0,
            0,
            0,
            Some(owner),
            None,
            GetModuleHandleW(None).ok().map(Into::into),
            None,
        )
        .ok()?;
        send(window, TTM_SETMAXTIPWIDTH, 0, None, MAX_WIDTH);
        Some(window)
    }
}

/// The size of the tooltip window for `text`. TTM_GETBUBBLESIZE is not used: comctl32 faulted
/// inside it on this machine.
fn bubble_size(window: HWND, text: &[u16]) -> (i32, i32) {
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: MAX_WIDTH as i32 - 16,
        bottom: 0,
    };
    let mut text = text.to_vec();
    unsafe {
        let dc = GetDC(Some(window));
        let font = send(window, WM_GETFONT, 0, None, 0);
        let font = if font != 0 {
            HGDIOBJ(font as *mut _)
        } else {
            GetStockObject(DEFAULT_GUI_FONT)
        };
        let old = SelectObject(dc, font);
        DrawTextW(
            dc,
            &mut text,
            &mut rect,
            DT_CALCRECT | DT_NOPREFIX | DT_WORDBREAK,
        );
        SelectObject(dc, old);
        ReleaseDC(Some(window), dc);
    }
    send(
        window,
        TTM_ADJUSTRECT,
        1,
        None,
        &mut rect as *mut RECT as isize,
    );
    (rect.right - rect.left, rect.bottom - rect.top)
}

/// Shows `text` for `owner` (a bar window) at the screen position `place` returns for the size of
/// the tooltip. Returns false when the tooltip window could not be made.
pub fn show(owner: HWND, text: &str, place: impl FnOnce(i32, i32) -> (i32, i32)) -> bool {
    let current = TIP.take();
    let tip = match current {
        Some(tip) => tip,
        None => match create(owner) {
            Some(window) => {
                if send(window, TTM_ADDTOOLW, 0, Some(&tool_info(owner, None)), 0) == 0 {
                    unsafe {
                        let _ = DestroyWindow(window);
                    }
                    return false;
                }
                Tip { window, owner }
            }
            None => return false,
        },
    };
    if tip.owner != owner {
        send(
            tip.window,
            TTM_DELTOOLW,
            0,
            Some(&tool_info(tip.owner, None)),
            0,
        );
        send(
            tip.window,
            TTM_ADDTOOLW,
            0,
            Some(&tool_info(owner, None)),
            0,
        );
    }
    let mut wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let info = tool_info(owner, Some(&mut wide));
    send(tip.window, TTM_UPDATETIPTEXTW, 0, Some(&info), 0);
    let (width, height) = bubble_size(tip.window, &wide[..wide.len() - 1]);
    let (x, y) = place(width, height);
    send(
        tip.window,
        TTM_TRACKPOSITION,
        0,
        None,
        ((y as u16 as isize) << 16) | (x as u16 as isize),
    );
    send(tip.window, TTM_TRACKACTIVATE, 1, Some(&info), 0);
    TIP.set(Some(Tip {
        window: tip.window,
        owner,
    }));
    true
}

pub fn hide() {
    let Some(tip) = TIP.take() else { return };
    send(
        tip.window,
        TTM_TRACKACTIVATE,
        0,
        Some(&tool_info(tip.owner, None)),
        0,
    );
    TIP.set(Some(tip));
}
