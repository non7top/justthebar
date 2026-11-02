//! Moves the Windows quick settings flyout (class `ControlCenterWindow`, opened by Win+A) next to
//! the bar whose widget was clicked. Windows places it at the bottom right of the screen itself.
use std::cell::{Cell, RefCell};
use std::time::{Duration, Instant};

use jtb_core::Edge;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, GetWindowRect, IsWindowVisible, SetWindowPos, SWP_NOACTIVATE, SWP_NOOWNERZORDER,
    SWP_NOSIZE, SWP_NOZORDER,
};

/// Where the flyout should go: next to this bar, near the clicked point.
#[derive(Clone, Copy)]
pub struct Anchor {
    pub edge: Edge,
    pub bar: RECT,
    pub monitor: RECT,
    pub click: POINT,
}

struct Pending {
    anchor: Anchor,
    since: Instant,
    seen: bool,
}

thread_local! {
    static PENDING: RefCell<Option<Pending>> = const { RefCell::new(None) };
    static LOGGED: Cell<bool> = const { Cell::new(false) };
}

/// A flyout is about to open next to `anchor`.
pub fn expect(anchor: Anchor) {
    PENDING.set(Some(Pending {
        anchor,
        since: Instant::now(),
        seen: false,
    }));
}

fn cloaked(window: HWND) -> bool {
    let mut value = 0u32;
    unsafe {
        DwmGetWindowAttribute(
            window,
            DWMWA_CLOAKED,
            &mut value as *mut u32 as *mut _,
            size_of::<u32>() as u32,
        )
    }
    .is_ok()
        && value != 0
}

fn visible_flyout() -> Option<HWND> {
    let mut after: Option<HWND> = None;
    while let Ok(window) =
        unsafe { FindWindowExW(None, after, w!("ControlCenterWindow"), PCWSTR::null()) }
    {
        after = Some(window);
        if unsafe { IsWindowVisible(window) }.as_bool() && !cloaked(window) {
            return Some(window);
        }
    }
    None
}

fn target(anchor: &Anchor, width: i32, height: i32, margin: i32) -> (i32, i32) {
    let (bar, monitor, click) = (anchor.bar, anchor.monitor, anchor.click);
    let clamp = |value: i32, low: i32, high: i32| value.min(high).max(low);
    let across_x = |x: i32| clamp(x, monitor.left + margin, monitor.right - width - margin);
    let across_y = |y: i32| clamp(y, monitor.top + margin, monitor.bottom - height - margin);
    match anchor.edge {
        Edge::Right => (
            bar.left - width - margin,
            across_y(click.y + margin - height),
        ),
        Edge::Left => (bar.right + margin, across_y(click.y + margin - height)),
        Edge::Bottom => (across_x(click.x - width / 2), bar.top - height - margin),
        Edge::Top => (across_x(click.x - width / 2), bar.bottom + margin),
    }
}

/// Call often (every 100 ms); keeps the flyout at its place while it is open. Returns a line for
/// the log the first time it finds and moves the flyout.
pub fn tick() -> Option<String> {
    let (anchor, since, seen) =
        PENDING.with_borrow(|p| p.as_ref().map(|p| (p.anchor, p.since, p.seen)))?;
    let Some(window) = visible_flyout() else {
        let waited = since.elapsed();
        if (seen && waited > Duration::from_millis(500)) || waited > Duration::from_secs(3) {
            PENDING.set(None);
            if !seen && !LOGGED.replace(true) {
                return Some("flyout: no visible ControlCenterWindow after Win+A".to_string());
            }
        }
        return None;
    };
    let mut rect = RECT::default();
    unsafe { GetWindowRect(window, &mut rect) }.ok()?;
    let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
    let margin = 12 * unsafe { GetDpiForWindow(window) } as i32 / 96;
    let (x, y) = target(&anchor, width, height, margin);
    let first = PENDING.with_borrow_mut(|p| {
        p.as_mut().is_some_and(|p| {
            let first = !p.seen;
            p.seen = true;
            p.since = Instant::now();
            first
        })
    });
    if (rect.left, rect.top) != (x, y) {
        let moved = unsafe {
            SetWindowPos(
                window,
                None,
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_NOACTIVATE,
            )
        };
        if first {
            return Some(format!(
                "flyout: ControlCenterWindow was at {rect:?}, moved to ({x}, {y}): {moved:?}"
            ));
        }
    }
    None
}
