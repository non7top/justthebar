// Derived from ManagedShell.AppBar/ExplorerHelper.cs (Apache-2.0), ported to Rust.
use std::ffi::c_void;
use std::mem::size_of;
use std::sync::atomic::{AtomicU32, Ordering};

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::UI::Shell::{
    SHAppBarMessage, ABM_GETSTATE, ABM_SETSTATE, ABS_ALWAYSONTOP, ABS_AUTOHIDE, APPBARDATA,
};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, GetWindow, GetWindowLongPtrW, GetWindowRect, IsWindowVisible, SetWindowPos,
    SystemParametersInfoW, GWL_EXSTYLE, GW_HWNDPREV, HWND_BOTTOM, HWND_TOPMOST, SPI_GETWORKAREA,
    SWP_HIDEWINDOW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, WS_EX_TOPMOST,
};

const UNSET: u32 = u32::MAX;
static STARTUP_STATE: AtomicU32 = AtomicU32::new(UNSET);

fn primary_tray() -> Option<HWND> {
    crate::tray::explorer_tray()
}

fn all_trays() -> Vec<HWND> {
    let mut trays: Vec<HWND> = primary_tray().into_iter().collect();
    let mut after: Option<HWND> = None;
    loop {
        let next = unsafe {
            FindWindowExW(None, after, w!("Shell_SecondaryTrayWnd"), PCWSTR::null()).ok()
        };
        match next {
            Some(hwnd) => {
                trays.push(hwnd);
                after = Some(hwnd);
            }
            None => break,
        }
    }
    trays
}

fn data(hwnd: HWND) -> APPBARDATA {
    APPBARDATA {
        cbSize: size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        ..Default::default()
    }
}

fn get_state() -> Option<u32> {
    let tray = primary_tray()?;
    let mut abd = data(tray);
    Some(unsafe { SHAppBarMessage(ABM_GETSTATE, &mut abd) } as u32)
}

fn set_state(state: u32) {
    if let Some(tray) = primary_tray() {
        let mut abd = data(tray);
        abd.lParam = LPARAM(state as isize);
        unsafe { SHAppBarMessage(ABM_SETSTATE, &mut abd) };
    }
}

fn set_visible(visible: bool) {
    let (after, flag) = if visible {
        (HWND_TOPMOST, SWP_SHOWWINDOW)
    } else {
        (HWND_BOTTOM, SWP_HIDEWINDOW)
    };
    for tray in all_trays() {
        let currently_visible = unsafe { IsWindowVisible(tray) }.as_bool();
        if currently_visible != visible {
            unsafe {
                let _ = SetWindowPos(
                    tray,
                    Some(after),
                    0,
                    0,
                    0,
                    0,
                    flag | SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
    }
}

/// Switches Explorer's taskbar to auto-hide and hides its windows. Remembers the startup state once.
pub fn hide() {
    if STARTUP_STATE.load(Ordering::SeqCst) == UNSET {
        if let Some(state) = get_state() {
            STARTUP_STATE.store(state, Ordering::SeqCst);
        }
    }
    set_state(ABS_AUTOHIDE);
    set_visible(false);
}

/// The taskbar state saved before the first `hide`.
pub fn startup_state() -> Option<u32> {
    Some(STARTUP_STATE.load(Ordering::SeqCst)).filter(|state| *state != UNSET)
}

/// True when Explorer's taskbar is in auto-hide, in which case it reserves no screen space.
pub fn autohide_active() -> bool {
    get_state().is_some_and(|state| state & ABS_AUTOHIDE != 0)
}

/// Hides the taskbar again if Explorer brought it back (it does so when appbars change).
pub fn enforce_hidden() -> bool {
    let visible = all_trays()
        .into_iter()
        .any(|tray| unsafe { IsWindowVisible(tray) }.as_bool());
    if visible {
        hide();
    }
    visible
}

/// Restores the state saved by `hide`, or the Windows default (always on top) if none was saved.
/// Returns the state it applied.
pub fn restore() -> u32 {
    let saved = STARTUP_STATE.load(Ordering::SeqCst);
    let state = if saved == UNSET {
        ABS_ALWAYSONTOP
    } else {
        saved
    };
    set_state(state);
    set_visible(true);
    crate::tray::give_back(false);
    state
}

/// Autohide state and primary work area, compared between ticks to log changes.
pub fn snapshot() -> String {
    format!(
        "state={:?} work={:?} monitors={}",
        get_state(),
        work_area(),
        crate::monitors::describe_work_areas()
    )
}

pub fn work_area() -> RECT {
    let mut work = RECT::default();
    unsafe {
        let _ = SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut work as *mut RECT as *mut c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
    }
    work
}

/// True when a visible Explorer taskbar window is above `hwnd` in the z-order, unless a topmost
/// window of another app that covers `hwnd` entirely (a screenshot overlay) sits between them.
pub fn tray_above(hwnd: HWND, ours: &[HWND]) -> bool {
    let trays: Vec<HWND> = all_trays()
        .into_iter()
        .filter(|tray| unsafe { IsWindowVisible(*tray) }.as_bool())
        .collect();
    if trays.is_empty() {
        return false;
    }
    let mut own = RECT::default();
    unsafe {
        let _ = GetWindowRect(hwnd, &mut own);
    }
    let mut current = hwnd;
    while let Ok(above) = unsafe { GetWindow(current, GW_HWNDPREV) } {
        if trays.contains(&above) {
            return true;
        }
        if !ours.contains(&above) && covers(above, &own) {
            return false;
        }
        current = above;
    }
    false
}

fn covers(hwnd: HWND, inner: &RECT) -> bool {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() {
            return false;
        }
        let topmost = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0;
        let mut rc = RECT::default();
        let _ = GetWindowRect(hwnd, &mut rc);
        topmost
            && rc.left <= inner.left
            && rc.top <= inner.top
            && rc.right >= inner.right
            && rc.bottom >= inner.bottom
    }
}
