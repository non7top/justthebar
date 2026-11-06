//! Finds the monitor taken over by a fullscreen window (a video in a browser, a game), so the
//! bars on it can step back.
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONULL,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetForegroundWindow, GetWindowLongPtrW, GetWindowRect, GetWindowTextW,
    GetWindowThreadProcessId, IsIconic, IsWindowVisible, GWL_STYLE, WS_CAPTION,
};

/// Window classes of the desktop and shell windows, which cover a monitor without being a
/// fullscreen app.
const SHELL_CLASSES: [&str; 5] = [
    "Progman",
    "WorkerW",
    "Shell_TrayWnd",
    "Shell_SecondaryTrayWnd",
    "ControlCenterWindow",
];

fn class_of(window: HWND) -> String {
    let mut buffer = [0u16; 64];
    let length = unsafe { GetClassNameW(window, &mut buffer) };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}

/// The monitor rectangle that the foreground window covers completely without a title bar, unless
/// the window is one of `ours` or a shell window.
pub fn covered_monitor(ours: &[HWND]) -> Option<RECT> {
    let window = unsafe { GetForegroundWindow() };
    if window.0.is_null() || ours.contains(&window) {
        return None;
    }
    unsafe {
        if !IsWindowVisible(window).as_bool() || IsIconic(window).as_bool() {
            return None;
        }
        let mut cloaked = 0u32;
        if DwmGetWindowAttribute(
            window,
            DWMWA_CLOAKED,
            &mut cloaked as *mut u32 as *mut _,
            size_of::<u32>() as u32,
        )
        .is_ok()
            && cloaked != 0
        {
            return None;
        }
        if GetWindowLongPtrW(window, GWL_STYLE) as u32 & WS_CAPTION.0 == WS_CAPTION.0 {
            return None;
        }
    }
    let class = class_of(window);
    if SHELL_CLASSES.contains(&class.as_str()) {
        return None;
    }
    let monitor = unsafe { MonitorFromWindow(window, MONITOR_DEFAULTTONULL) };
    if monitor.is_invalid() {
        return None;
    }
    let mut info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        return None;
    }
    let mut rc = RECT::default();
    unsafe { GetWindowRect(window, &mut rc) }.ok()?;
    let screen = info.rcMonitor;
    (rc.left <= screen.left
        && rc.top <= screen.top
        && rc.right >= screen.right
        && rc.bottom >= screen.bottom)
        .then_some(screen)
}

/// The foreground window's class, title and process, for the log.
pub fn describe_foreground() -> String {
    let window = unsafe { GetForegroundWindow() };
    if window.0.is_null() {
        return "no foreground window".to_string();
    }
    let mut title = [0u16; 80];
    let length = unsafe { GetWindowTextW(window, &mut title) };
    let mut process = 0u32;
    unsafe { GetWindowThreadProcessId(window, Some(&mut process)) };
    let mut rc = RECT::default();
    let _ = unsafe { GetWindowRect(window, &mut rc) };
    format!(
        "foreground {:#x} class {} title {:?} pid {process} at {rc:?}",
        window.0 as usize,
        class_of(window),
        String::from_utf16_lossy(&title[..length.max(0) as usize])
    )
}
