// Derived from ManagedShell.AppBar/AppBarManager.cs (Apache-2.0), ported to Rust.
use std::mem::size_of;

use jtb_core::layout::Insets;
use jtb_core::Edge;
use windows::core::w;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::Shell::{
    SHAppBarMessage, ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_ACTIVATE, ABM_NEW, ABM_QUERYPOS,
    ABM_REMOVE, ABM_SETPOS, ABM_WINDOWPOSCHANGED, ABN_FULLSCREENAPP, ABN_POSCHANGED, APPBARDATA,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowRect, RegisterWindowMessageW, SetWindowPos, HWND_BOTTOM, HWND_TOPMOST, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE,
};

#[derive(Clone, Copy)]
pub struct AppBar {
    hwnd: HWND,
    edge: Edge,
    thickness: i32,
    monitor: RECT,
    insets: Insets,
    /// `Some(distance from the screen edge)` places the bar flush at the monitor edge and ignores
    /// space the system keeps reserved there (Explorer's hidden taskbar).
    flush_offset: Option<i32>,
    callback_message: u32,
}

fn abe(edge: Edge) -> u32 {
    match edge {
        Edge::Top => ABE_TOP,
        Edge::Bottom => ABE_BOTTOM,
        Edge::Left => ABE_LEFT,
        Edge::Right => ABE_RIGHT,
    }
}

fn data(hwnd: HWND) -> APPBARDATA {
    APPBARDATA {
        cbSize: size_of::<APPBARDATA>() as u32,
        hWnd: hwnd,
        ..Default::default()
    }
}

impl AppBar {
    /// Registers `hwnd` as an appbar and reserves `thickness` pixels on `monitor`'s `edge`.
    pub fn register(
        hwnd: HWND,
        edge: Edge,
        thickness: i32,
        monitor: RECT,
        insets: Insets,
        flush_offset: Option<i32>,
    ) -> (AppBar, String) {
        let callback_message = unsafe { RegisterWindowMessageW(w!("AppBarMessage")) };
        let mut abd = data(hwnd);
        abd.uCallbackMessage = callback_message;
        unsafe { SHAppBarMessage(ABM_NEW, &mut abd) };
        let bar = AppBar {
            hwnd,
            edge,
            thickness,
            monitor,
            insets,
            flush_offset,
            callback_message,
        };
        let info = bar.set_pos();
        (bar, info)
    }

    pub fn edge(&self) -> Edge {
        self.edge
    }

    pub fn window(&self) -> HWND {
        self.hwnd
    }

    /// What Explorer offers `other` for this bar's edge of its monitor. Smaller than the monitor
    /// when Explorer holds this bar's registration.
    pub fn probe(&self, other: HWND) -> RECT {
        let mut abd = data(other);
        abd.uEdge = abe(self.edge);
        abd.rc = self.monitor;
        unsafe { SHAppBarMessage(ABM_QUERYPOS, &mut abd) };
        abd.rc
    }

    pub fn callback_message(&self) -> u32 {
        self.callback_message
    }

    /// The system trims appbars to avoid ones on perpendicular edges (Explorer's included), so the
    /// long side is set explicitly: the monitor minus the corners taken by earlier bars of ours.
    fn set_span(&self, rc: &mut RECT) {
        if self.edge.is_horizontal() {
            rc.left = self.monitor.left + self.insets.start;
            rc.right = self.monitor.right - self.insets.end;
        } else {
            rc.top = self.monitor.top + self.insets.start;
            rc.bottom = self.monitor.bottom - self.insets.end;
        }
    }

    /// QUERYPOS, fix up the thickness (the system only adjusts the edge it moved), then SETPOS.
    pub fn set_pos(&self) -> String {
        match self.flush_offset {
            Some(offset) => self.set_pos_flush(offset),
            None => self.set_pos_standard(),
        }
    }

    fn set_pos_standard(&self) -> String {
        let mut abd = data(self.hwnd);
        abd.uEdge = abe(self.edge);
        abd.rc = self.monitor;
        unsafe { SHAppBarMessage(ABM_QUERYPOS, &mut abd) };
        let queried = abd.rc;
        self.set_span(&mut abd.rc);
        match self.edge {
            Edge::Top => abd.rc.bottom = abd.rc.top + self.thickness,
            Edge::Bottom => abd.rc.top = abd.rc.bottom - self.thickness,
            Edge::Left => abd.rc.right = abd.rc.left + self.thickness,
            Edge::Right => abd.rc.left = abd.rc.right - self.thickness,
        }
        unsafe { SHAppBarMessage(ABM_SETPOS, &mut abd) };
        // SETPOS trims the long side again, so widen the window itself afterwards.
        let mut rc = abd.rc;
        self.set_span(&mut rc);
        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                rc.left,
                rc.top,
                rc.right - rc.left,
                rc.bottom - rc.top,
                SWP_NOACTIVATE,
            );
        }
        let mut window = RECT::default();
        unsafe {
            let _ = GetWindowRect(self.hwnd, &mut window);
        }
        format!(
            "monitor={:?} queried={:?} set={:?} window={:?}",
            self.monitor, queried, rc, window
        )
    }

    fn set_pos_flush(&self, offset: i32) -> String {
        let mut rc = self.monitor;
        self.set_span(&mut rc);
        match self.edge {
            Edge::Top => {
                rc.top = self.monitor.top + offset;
                rc.bottom = rc.top + self.thickness;
            }
            Edge::Bottom => {
                rc.bottom = self.monitor.bottom - offset;
                rc.top = rc.bottom - self.thickness;
            }
            Edge::Left => {
                rc.left = self.monitor.left + offset;
                rc.right = rc.left + self.thickness;
            }
            Edge::Right => {
                rc.right = self.monitor.right - offset;
                rc.left = rc.right - self.thickness;
            }
        }
        // SETPOS notifies every other appbar; repeating an unchanged one would ping-pong forever.
        let mut current = RECT::default();
        unsafe {
            let _ = GetWindowRect(self.hwnd, &mut current);
        }
        if current == rc {
            return format!("flush monitor={:?} unchanged={:?}", self.monitor, rc);
        }
        let mut abd = data(self.hwnd);
        abd.uEdge = abe(self.edge);
        abd.rc = rc;
        unsafe { SHAppBarMessage(ABM_SETPOS, &mut abd) };
        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                rc.left,
                rc.top,
                rc.right - rc.left,
                rc.bottom - rc.top,
                SWP_NOACTIVATE,
            );
        }
        format!("flush monitor={:?} set={:?}", self.monitor, rc)
    }

    /// Handles the appbar callback message: `notification` is the wParam, `value` the lParam.
    /// Returns a description when the position was recomputed.
    pub fn on_callback(&self, notification: usize, value: isize) -> Option<String> {
        match notification as u32 {
            ABN_POSCHANGED => Some(self.set_pos()),
            ABN_FULLSCREENAPP => {
                self.set_fullscreen(value != 0);
                None
            }
            _ => None,
        }
    }

    /// Fullscreen app entered (`true`) or left (`false`): drop to the bottom or return to topmost.
    fn set_fullscreen(&self, entered: bool) {
        let after = if entered { HWND_BOTTOM } else { HWND_TOPMOST };
        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                Some(after),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }
    }

    /// Registers again after Explorer restarted, which forgets every appbar, and reserves the
    /// edge once more.
    pub fn reregister(&self) -> String {
        let mut abd = data(self.hwnd);
        abd.uCallbackMessage = self.callback_message;
        let registered = unsafe { SHAppBarMessage(ABM_NEW, &mut abd) };
        format!("registration answer {registered}; {}", self.set_pos())
    }

    /// Removes the registration and makes it again, which makes Explorer work out the work area
    /// from scratch.
    pub fn rebuild(&self) -> String {
        self.remove();
        self.reregister()
    }

    pub fn activate(&self) {
        let mut abd = data(self.hwnd);
        unsafe { SHAppBarMessage(ABM_ACTIVATE, &mut abd) };
    }

    pub fn window_pos_changed(&self) {
        let mut abd = data(self.hwnd);
        unsafe { SHAppBarMessage(ABM_WINDOWPOSCHANGED, &mut abd) };
    }

    pub fn remove(&self) {
        let mut abd = data(self.hwnd);
        unsafe { SHAppBarMessage(ABM_REMOVE, &mut abd) };
    }
}
