use std::mem::size_of;

use jtb_core::config::MonitorSel;
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, MonitorFromRect, MonitorFromWindow, HDC, HMONITOR,
    MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::WindowsAndMessaging::{GetWindowPlacement, IsIconic, WINDOWPLACEMENT};

const MONITORINFOF_PRIMARY: u32 = 1;

#[derive(Debug, Clone, Copy)]
pub struct Monitor {
    pub rect: RECT,
    pub work: RECT,
    pub primary: bool,
}

unsafe extern "system" fn collect(
    monitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    data: LPARAM,
) -> BOOL {
    let list = &mut *(data.0 as *mut Vec<Monitor>);
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if GetMonitorInfoW(monitor, &mut info).as_bool() {
        list.push(Monitor {
            rect: info.rcMonitor,
            work: info.rcWork,
            primary: info.dwFlags & MONITORINFOF_PRIMARY != 0,
        });
    }
    BOOL(1)
}

/// All monitors with the primary one first, so index 0 is always the primary.
pub fn list() -> Vec<Monitor> {
    let mut monitors: Vec<Monitor> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(collect),
            LPARAM(&mut monitors as *mut Vec<Monitor> as isize),
        );
    }
    monitors.sort_by_key(|m| !m.primary);
    monitors
}

/// Work area of every monitor in `list()` order, for the log.
pub fn describe_work_areas() -> String {
    list()
        .iter()
        .map(|m| {
            let w = m.work;
            format!("({},{},{},{})", w.left, w.top, w.right, w.bottom)
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Returns the monitor's index in `list()` order and its rect.
pub fn resolve(monitors: &[Monitor], sel: MonitorSel) -> Option<(usize, RECT)> {
    let index = match sel {
        MonitorSel::Primary => 0,
        MonitorSel::Index(i) => i,
    };
    monitors.get(index).map(|m| (index, m.rect))
}

/// Rect of the monitor a window is on. A minimized window counts where it will restore to.
pub fn rect_of_window(hwnd: HWND) -> Option<RECT> {
    let monitor = unsafe {
        if IsIconic(hwnd).as_bool() {
            let mut placement = WINDOWPLACEMENT {
                length: size_of::<WINDOWPLACEMENT>() as u32,
                ..Default::default()
            };
            GetWindowPlacement(hwnd, &mut placement).ok()?;
            MonitorFromRect(&placement.rcNormalPosition, MONITOR_DEFAULTTONEAREST)
        } else {
            MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST)
        }
    };
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    unsafe { GetMonitorInfoW(monitor, &mut info) }
        .as_bool()
        .then_some(info.rcMonitor)
}
