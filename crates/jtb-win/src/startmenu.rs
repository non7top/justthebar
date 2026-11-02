//! Open-Shell opens its menu next to its own button, a popup window owned by Explorer's taskbar.
//! With that taskbar hidden the button sits at the bottom-left, so the menu would open there;
//! moving the button under our Start button just before the toggle makes the menu follow it.
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, GetWindow, GetWindowRect, GetWindowThreadProcessId, PostMessageW,
    RegisterWindowMessageW, SetWindowPos, GW_OWNER, SWP_NOACTIVATE, SWP_NOZORDER,
};

fn buttons() -> Vec<HWND> {
    let Some(owner) = crate::tray::explorer_tray() else {
        return Vec::new();
    };
    let mut found = Vec::new();
    let mut after: Option<HWND> = None;
    while let Ok(button) =
        unsafe { FindWindowExW(None, after, w!("OpenShell.CStartButton"), PCWSTR::null()) }
    {
        after = Some(button);
        if unsafe { GetWindow(button, GW_OWNER) }.ok() == Some(owner) {
            found.push(button);
        }
    }
    found
}

/// The process id and screen rectangle of Explorer's own `Shell_TrayWnd`.
pub fn explorer_tray_info() -> Option<(u32, RECT)> {
    let tray = crate::tray::explorer_tray()?;
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(tray, Some(&mut pid)) };
    Some((pid, rect_of(tray)?))
}

fn rect_of(button: HWND) -> Option<RECT> {
    let mut rect = RECT::default();
    unsafe { GetWindowRect(button, &mut rect) }
        .ok()
        .map(|_| rect)
}

/// Where Open-Shell's button of the main taskbar is now (screen coordinates).
pub fn button_rect() -> Option<RECT> {
    buttons().into_iter().filter_map(rect_of).next_back()
}

/// Moves Open-Shell's button of the main taskbar to `to` (screen coordinates). Returns where the
/// button ended up, or `None` when there is no such button.
pub fn move_button(to: RECT) -> Option<RECT> {
    for button in buttons() {
        unsafe {
            let _ = SetWindowPos(
                button,
                None,
                to.left,
                to.top,
                to.right - to.left,
                to.bottom - to.top,
                SWP_NOACTIVATE | SWP_NOZORDER,
            );
        }
    }
    button_rect()
}

/// Asks the Open-Shell hook inside Explorer to toggle its menu, the same message
/// `StartMenu.exe -toggle` posts to Explorer's taskbar after it has started up. Returns whether
/// the message was posted.
pub fn post_toggle() -> bool {
    let Some(tray) = crate::tray::explorer_tray() else {
        return false;
    };
    let message = unsafe { RegisterWindowMessageW(w!("OpenShellMenu.StartMenuMsg")) };
    message != 0 && unsafe { PostMessageW(Some(tray), message, WPARAM(0), LPARAM(0)) }.is_ok()
}
