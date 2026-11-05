//! Moves the Windows quick settings flyout (class `ControlCenterWindow`, opened by Win+A) next to
//! the bar whose widget was clicked. Windows places it at the bottom right of the screen itself.
use std::cell::{Cell, RefCell};
use std::time::{Duration, Instant};

use jtb_core::Edge;

use crate::hookdll;
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Dwm::{
    DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS,
};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowExW, GetWindowRect, IsWindowVisible, SetWindowPos, SWP_NOACTIVATE, SWP_NOOWNERZORDER,
    SWP_NOSIZE, SWP_NOZORDER,
};

/// Where the flyout should go: next to this bar, near the clicked point.
#[derive(Clone, Copy)]
pub struct Anchor {
    /// The bar window the widget is on.
    pub window: HWND,
    pub edge: Edge,
    pub bar: RECT,
    pub monitor: RECT,
    pub click: POINT,
}

struct Pending {
    anchor: Anchor,
    since: Instant,
    seen: bool,
    repositioned: u32,
}

thread_local! {
    static PENDING: RefCell<Option<Pending>> = const { RefCell::new(None) };
    static LOGGED: Cell<bool> = const { Cell::new(false) };
    static LAST_OPEN: Cell<Option<Instant>> = const { Cell::new(None) };
    static CLOSING: Cell<Option<Instant>> = const { Cell::new(None) };
    static HOOKED: Cell<bool> = const { Cell::new(false) };
    static LINES: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static HOOK_STATE: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Lines for the log queued by `expect` and `leave_alone`.
pub fn take_lines() -> Vec<String> {
    LINES.take()
}

fn all_flyouts() -> Vec<HWND> {
    let mut found = Vec::new();
    let mut after: Option<HWND> = None;
    while let Ok(window) =
        unsafe { FindWindowExW(None, after, w!("ControlCenterWindow"), PCWSTR::null()) }
    {
        after = Some(window);
        found.push(window);
    }
    found
}

/// Tells the code inside `ShellHost.exe` where the flyout goes, so it is placed before it is
/// shown. Logs the first outcome and each change of it.
fn aim_hook(anchor: &Anchor) {
    HOOKED.set(false);
    let windows = all_flyouts();
    for window in &windows {
        let window = *window;
        let attached = hookdll::attach(window);
        let state = match &attached {
            Ok(true) => {
                let mut rect = RECT::default();
                if unsafe { GetWindowRect(window, &mut rect) }.is_ok() {
                    let (x, y) = target_for(anchor, &rect);
                    let side = matches!(anchor.edge, Edge::Left | Edge::Right);
                    hookdll::set_position(window, x, (!side).then_some(y));
                    HOOKED.set(true);
                    let visible = unsafe { IsWindowVisible(window) }.as_bool();
                    let hidden = cloaked(window);
                    let mut note = format!(
                        "window {:#x} of {}: at {rect:?}, visible {visible}, cloaked {hidden}, target ({x}, {y})",
                        window.0 as usize,
                        windows.len()
                    );
                    if !visible || hidden {
                        // On the first open the shell shows the window where it was parked, so
                        // the hidden window is parked at the target.
                        let moved = unsafe {
                            SetWindowPos(
                                window,
                                None,
                                x,
                                if side { rect.top } else { y },
                                0,
                                0,
                                SWP_NOSIZE | SWP_NOZORDER | SWP_NOOWNERZORDER | SWP_NOACTIVATE,
                            )
                        };
                        let mut after = RECT::default();
                        let _ = unsafe { GetWindowRect(window, &mut after) };
                        note += &format!(", pre-move {moved:?}, now at {after:?}");
                    }
                    LINES.with_borrow_mut(|lines| lines.push(format!("flyout: {note}")));
                }
                "placing the flyout from inside ShellHost".to_string()
            }
            Ok(false) => "hook loaded but the flyout window did not take it".to_string(),
            Err(error) => format!("hook unavailable ({error}), moving the flyout from outside"),
        };
        HOOK_STATE.with_borrow_mut(|last| {
            if *last != state {
                LINES.with_borrow_mut(|lines| lines.push(format!("flyout: {state}")));
                *last = state;
            }
        });
    }
}

/// Switches off the target held inside `ShellHost.exe`; call on exit so a later Win+A is not moved.
pub fn release_hook() {
    for window in all_flyouts() {
        hookdll::clear(window);
    }
}

/// True when the flyout closed a moment ago: a click on the bar takes focus and dismisses it, so a
/// Win+A sent for that same click would open it again instead of closing it.
pub fn just_dismissed() -> bool {
    visible_flyout().is_none()
        && LAST_OPEN
            .get()
            .is_some_and(|at| at.elapsed() < Duration::from_millis(600))
}

/// True while the flyout is on screen.
pub fn is_open() -> bool {
    visible_flyout().is_some()
}

/// The flyout is about to be closed: stop moving it, or the closing window gets pulled back.
pub fn leave_alone() {
    PENDING.set(None);
    CLOSING.set(Some(Instant::now()));
    release_hook();
}

/// The flyout's state at a click, for the log.
pub fn describe() -> String {
    format!(
        "visible {}, last seen {:?} ago",
        visible_flyout().is_some(),
        LAST_OPEN.get().map(|at| at.elapsed())
    )
}

/// A flyout is about to open next to `anchor`.
pub fn expect(anchor: Anchor) {
    aim_hook(&anchor);
    PENDING.set(Some(Pending {
        anchor,
        since: Instant::now(),
        seen: false,
        repositioned: 0,
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

fn target_for(anchor: &Anchor, rect: &RECT) -> (i32, i32) {
    let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
    let margin = 12 * unsafe { GetDpiForWindow(anchor.window) } as i32 / 96;
    target(anchor, width, height, margin)
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
    let visible = visible_flyout();
    if visible.is_some() {
        LAST_OPEN.set(Some(Instant::now()));
    }
    if let Some(clicked) = CLOSING.get() {
        let waited = clicked.elapsed();
        if visible.is_none() {
            CLOSING.set(None);
            return Some(format!("flyout: closed {waited:?} after the click"));
        }
        if waited > Duration::from_millis(800) {
            CLOSING.set(None);
            return Some(format!("flyout: still open {waited:?} after the click"));
        }
    }
    let (anchor, since, seen, repositioned) = PENDING.with_borrow(|p| {
        p.as_ref()
            .map(|p| (p.anchor, p.since, p.seen, p.repositioned))
    })?;
    let Some(window) = visible else {
        let waited = since.elapsed();
        if (seen && waited > Duration::from_millis(500)) || waited > Duration::from_secs(3) {
            PENDING.set(None);
            release_hook();
            if !seen && !LOGGED.replace(true) {
                return Some("flyout: no visible ControlCenterWindow after Win+A".to_string());
            }
            if repositioned > 0 {
                return Some(format!(
                    "flyout: pushed back {repositioned} times while it was open"
                ));
            }
        }
        return None;
    };
    let mut rect = RECT::default();
    unsafe { GetWindowRect(window, &mut rect) }.ok()?;
    let (x, y) = target_for(&anchor, &rect);
    let first = PENDING.with_borrow_mut(|p| {
        p.as_mut().is_some_and(|p| {
            let first = !p.seen;
            p.seen = true;
            p.since = Instant::now();
            first
        })
    });
    if HOOKED.get() {
        // Moving it after it is shown makes its buttons ignore clicks for a while.
        return first.then(|| {
            format!("flyout: shown at {rect:?} with the hook aiming at x {x}, not moving it")
        });
    }
    if (rect.left, rect.top) != (x, y) {
        if !first {
            PENDING.with_borrow_mut(|p| p.iter_mut().for_each(|p| p.repositioned += 1));
        }
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
            let mut visual = RECT::default();
            let visual = unsafe {
                DwmGetWindowAttribute(
                    window,
                    DWMWA_EXTENDED_FRAME_BOUNDS,
                    &mut visual as *mut RECT as *mut _,
                    size_of::<RECT>() as u32,
                )
            }
            .map(|()| visual);
            return Some(format!(
                "flyout: ControlCenterWindow was at {rect:?}, moved to ({x}, {y}): {moved:?}, frame bounds after: {visual:?}"
            ));
        }
    }
    None
}
