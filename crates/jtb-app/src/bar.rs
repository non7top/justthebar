use std::cell::{Cell, RefCell};

use jtb_core::buttons::{bar_layout_with_widgets, hit_test, tray_icon_rect, BarLayout};
use jtb_core::config::{BarConfig, Button, Config};
use jtb_core::layout::{corner_insets, edge_offsets, Slot};
use jtb_core::scene::{
    build_scene_with_widgets, start_art_size, ButtonView, Color, Drag, StartView,
};
use jtb_core::tray::{Mouse, Side};
use jtb_core::Edge;
use jtb_ui::{D2dRenderer, Renderer};
use jtb_win::appbar::AppBar;
use windows::core::{w, Result};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{ClientToScreen, InvalidateRect, ValidateRect};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetWindowRect, KillTimer,
    LoadCursorW, RegisterClassW, RegisterWindowMessageW, SetTimer, SetWindowPos, ShowWindow,
    HWND_TOPMOST, IDC_ARROW, MA_NOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SW_SHOWNOACTIVATE, WM_ACTIVATE, WM_DROPFILES, WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONDOWN,
    WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_PAINT,
    WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SIZE, WM_TIMER, WM_WINDOWPOSCHANGED, WNDCLASSW,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use crate::log;

const WM_MOUSELEAVE: u32 = 0x02A3;

/// Repaints the Start button while its hot frame fades in or out.
const VK_ESCAPE: usize = 0x1B;
const TIMER_START_FADE: usize = 1;
const FADE_TICK_MS: u32 = 30;
/// Fraction of the fade done per tick: 30 ms of 300 ms, as in Open-Shell.
const FADE_STEP: f32 = 0.1;

struct BarWindow {
    hwnd: HWND,
    /// Position in `Config::bars`.
    index: usize,
    bar: AppBar,
    background: Color,
    /// Show only the windows on this monitor; `None` shows all.
    task_monitor: Option<RECT>,
    /// Button under the mouse, and whether a `WM_MOUSELEAVE` is already requested.
    hover: Option<usize>,
    /// Notification area icon under the mouse.
    tray_hover: Option<usize>,
    tracking: bool,
    /// Button index and position where the left button went down, and whether it became a drag.
    press: Option<(usize, i32, i32)>,
    dragging: bool,
    /// How much of the Start button's hot frame is mixed in, 0 to 1.
    start_blend: f32,
    renderer: Option<D2dRenderer>,
}

thread_local! {
    static BARS: RefCell<Vec<BarWindow>> = const { RefCell::new(Vec::new()) };
    static BEHIND_FULLSCREEN: RefCell<Vec<HWND>> = const { RefCell::new(Vec::new()) };
    static LAST_FULLSCREEN: Cell<Option<std::time::Instant>> = const { Cell::new(None) };
    static RESERVE_TICKS: Cell<u32> = const { Cell::new(0) };
    static RESERVE_TRIES: Cell<u32> = const { Cell::new(0) };
}

/// A bar to create: its config, the monitor index it resolved to, and that monitor's rect.
pub struct Planned {
    /// Position in `Config::bars`.
    pub index: usize,
    pub config: BarConfig,
    pub monitor_index: usize,
    pub monitor_rect: RECT,
}

pub fn register_class() -> Result<()> {
    unsafe {
        let class = WNDCLASSW {
            hInstance: GetModuleHandleW(None)?.into(),
            lpszClassName: w!("JustTheBar"),
            lpfnWndProc: Some(wnd_proc),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        };
        RegisterClassW(&class);
    }
    Ok(())
}

/// Creates all bars. Windows are made first, at their monitor's origin, so each one reports that
/// monitor's dpi; only then are thicknesses known and the corners can be shared out.
/// With `flush`, bars sit at the monitor edge even though the system keeps Explorer's hidden
/// taskbar space reserved there; that strip cannot be reclaimed (SPI_SETWORKAREA is refused).
pub fn build(planned: Vec<Planned>, flush: bool, config: &Config) {
    let mut windows = Vec::new();
    for plan in planned {
        match create_window(&plan) {
            Ok(hwnd) => {
                let dpi = unsafe { GetDpiForWindow(hwnd) } as i32;
                let thickness = plan.config.size as i32 * dpi / 96;
                windows.push((hwnd, plan, thickness));
            }
            Err(e) => log(&format!("create bar window failed: {e}")),
        }
    }
    let slots: Vec<Slot> = windows
        .iter()
        .map(|(_, plan, thickness)| Slot {
            monitor: plan.monitor_index,
            edge: plan.config.edge,
            thickness: *thickness,
        })
        .collect();
    let insets = corner_insets(&slots);
    let offsets = edge_offsets(&slots);
    for (((hwnd, plan, thickness), insets), offset) in windows.into_iter().zip(insets).zip(offsets)
    {
        let (bar, info) = AppBar::register(
            hwnd,
            plan.config.edge,
            thickness,
            plan.monitor_rect,
            insets,
            flush.then_some(offset),
        );
        log(&format!(
            "bar monitor={} edge={:?} thickness={thickness} {info} work={:?}",
            plan.monitor_index,
            plan.config.edge,
            jtb_win::explorer::work_area()
        ));
        let (width, height) = client_size(hwnd);
        let renderer = match D2dRenderer::new(hwnd, width, height) {
            Ok(renderer) => Some(renderer),
            Err(e) => {
                log(&format!("renderer init failed: {e}"));
                None
            }
        };
        BARS.with_borrow_mut(|bars| {
            bars.push(BarWindow {
                hwnd,
                index: plan.index,
                bar,
                background: config.appearance.bar_color.0,
                task_monitor: config.tasks.per_monitor.then_some(plan.monitor_rect),
                hover: None,
                tray_hover: None,
                tracking: false,
                press: None,
                dragging: false,
                start_blend: 0.0,
                renderer,
            })
        });
        jtb_win::shell::accept_drops(hwnd);
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
    }
}

fn create_window(plan: &Planned) -> Result<HWND> {
    unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            w!("JustTheBar"),
            w!("justthebar"),
            WS_POPUP,
            plan.monitor_rect.left,
            plan.monitor_rect.top,
            1,
            1,
            None,
            None,
            Some(GetModuleHandleW(None)?.into()),
            None,
        )
    }
}

/// Unregisters every appbar and closes its window.
pub fn destroy_all() {
    let bars = BARS.take();
    for BarWindow {
        hwnd,
        bar,
        renderer,
        ..
    } in bars
    {
        drop(renderer);
        bar.remove();
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
    }
}

/// Explorer's taskbar and our bars are all topmost, so touching the taskbar raises it over a bar.
pub fn keep_above_explorer() {
    let hwnds: Vec<HWND> = BARS.with_borrow(|bars| bars.iter().map(|b| b.hwnd).collect());
    for &hwnd in &hwnds {
        if BEHIND_FULLSCREEN.with_borrow(|hidden| hidden.contains(&hwnd)) {
            continue;
        }
        if jtb_win::explorer::tray_above(hwnd, &hwnds) {
            unsafe {
                let _ = SetWindowPos(
                    hwnd,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
    }
}

/// Puts the bars on a monitor that a fullscreen window covers behind it, and back on top when
/// the window goes. Returns a line for the log when a bar changed sides.
pub fn follow_fullscreen() -> Option<String> {
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, GWL_EXSTYLE, HWND_BOTTOM, WS_EX_TOPMOST,
    };
    let hwnds: Vec<HWND> = BARS.with_borrow(|bars| bars.iter().map(|b| b.hwnd).collect());
    let covered = jtb_win::fullscreen::covered_monitor(&hwnds);
    let mut line = None;
    for &hwnd in &hwnds {
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
        if !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
            continue;
        }
        let behind = covered == Some(info.rcMonitor);
        let was = BEHIND_FULLSCREEN.with_borrow(|hidden| hidden.contains(&hwnd));
        let topmost = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32 & WS_EX_TOPMOST.0 != 0;
        if behind && (!was || topmost) {
            unsafe {
                let _ = SetWindowPos(
                    hwnd,
                    Some(HWND_BOTTOM),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        } else if !behind && was {
            unsafe {
                let _ = SetWindowPos(
                    hwnd,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }
        if behind != was {
            BEHIND_FULLSCREEN.with_borrow_mut(|hidden| {
                if behind {
                    hidden.push(hwnd);
                } else {
                    hidden.retain(|h| *h != hwnd);
                }
            });
            line = Some(format!(
                "fullscreen: bar {:#x} {} on monitor {:?}; {}",
                hwnd.0 as usize,
                if behind {
                    "stepped back"
                } else {
                    "back on top"
                },
                info.rcMonitor,
                jtb_win::fullscreen::describe_foreground()
            ));
        }
    }
    line
}

/// True while a fullscreen window covers a monitor that has a bar, and for a few seconds after:
/// a game's window keeps losing and regaining the foreground while it starts, and shell changes
/// made in those gaps (new work areas, bars made again) upset it.
pub fn fullscreen_active() -> bool {
    let now = BEHIND_FULLSCREEN.with_borrow(|hidden| !hidden.is_empty());
    if now {
        LAST_FULLSCREEN.set(Some(std::time::Instant::now()));
    }
    now || LAST_FULLSCREEN
        .get()
        .is_some_and(|at| at.elapsed() < std::time::Duration::from_secs(6))
}

/// Windows can reset the work area behind the bars' backs (a display change makes Explorer
/// recompute it), after which maximized windows cover the bars. Checks every few seconds that each
/// bar's strip is outside its monitor's work area; if not, reserves it again, first with a new
/// position and, when that did not help, by removing the registration and making it again. Returns a line for the log.
pub fn ensure_reserved() -> Option<String> {
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
    const CHECK_EVERY_TICKS: u32 = 10;
    if fullscreen_active() {
        return None;
    }
    let ticks = RESERVE_TICKS.get() + 1;
    RESERVE_TICKS.set(ticks % CHECK_EVERY_TICKS);
    if !ticks.is_multiple_of(CHECK_EVERY_TICKS) {
        return None;
    }
    let bars: Vec<(HWND, AppBar)> =
        BARS.with_borrow(|bars| bars.iter().map(|b| (b.hwnd, b.bar)).collect());
    let mut unreserved = Vec::new();
    for (hwnd, bar) in bars {
        let mut rect = RECT::default();
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
        if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err()
            || !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool()
        {
            continue;
        }
        let work = info.rcWork;
        let overlap_x = rect.right.min(work.right) - rect.left.max(work.left);
        let overlap_y = rect.bottom.min(work.bottom) - rect.top.max(work.top);
        if overlap_x > 8 && overlap_y > 8 {
            unreserved.push((bar, rect, work, info.rcMonitor));
        }
    }
    if unreserved.is_empty() {
        RESERVE_TRIES.set(0);
        return None;
    }
    let tries = RESERVE_TRIES.get();
    RESERVE_TRIES.set(tries + 1);
    // After a few failures, only look again every 30 s.
    if tries >= 6 && !tries.is_multiple_of(15) {
        return None;
    }
    let probe_window = BARS.with_borrow(|bars| bars.iter().map(|b| b.hwnd).collect::<Vec<_>>());
    let mut lines = Vec::new();
    for (bar, rect, work, monitor) in unreserved {
        let done = if tries == 0 {
            bar.set_pos()
        } else {
            bar.rebuild()
        };
        // Another window asks for the same edge: Explorer answers with a smaller rectangle only
        // when it holds this bar's registration.
        let other = probe_window
            .iter()
            .copied()
            .find(|w| *w != bar.window())
            .unwrap_or_default();
        let seen = bar.probe(other);
        lines.push(format!(
            "reserve: bar {rect:?} lies inside the work area {work:?}, reserved again (try {}): {done}; monitor {monitor:?}, another window asking for the edge gets {seen:?}",
            tries + 1
        ));
    }
    Some(lines.join("\n"))
}

pub fn invalidate_all() {
    let hwnds: Vec<HWND> = BARS.with_borrow(|bars| bars.iter().map(|b| b.hwnd).collect());
    for hwnd in hwnds {
        unsafe {
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
    }
}

/// Where the flyout of a widget clicked just now belongs: the bar under the cursor.
pub fn flyout_anchor() -> Option<jtb_win::flyout::Anchor> {
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetWindowRect};
    let mut click = POINT::default();
    unsafe { GetCursorPos(&mut click) }.ok()?;
    BARS.with_borrow(|bars| {
        bars.iter().find_map(|b| {
            let mut bar = RECT::default();
            unsafe { GetWindowRect(b.hwnd, &mut bar) }.ok()?;
            let inside = click.x >= bar.left
                && click.x < bar.right
                && click.y >= bar.top
                && click.y < bar.bottom;
            if !inside {
                return None;
            }
            let mut info = MONITORINFO {
                cbSize: size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };
            let monitor = unsafe { MonitorFromWindow(b.hwnd, MONITOR_DEFAULTTONEAREST) };
            unsafe { GetMonitorInfoW(monitor, &mut info) }.ok().ok()?;
            Some(jtb_win::flyout::Anchor {
                window: b.hwnd,
                edge: b.bar.edge(),
                bar,
                monitor: info.rcMonitor,
                click,
            })
        })
    })
}

/// A lone press of the Windows key: opens the Start menu from the Start button of the bar on the
/// monitor under the cursor, or of the first bar when none is there.
pub fn press_start_at_cursor() {
    use windows::Win32::Graphics::Gdi::{
        MonitorFromPoint, MonitorFromWindow, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
    let mut cursor = POINT::default();
    let _ = unsafe { GetCursorPos(&mut cursor) };
    let monitor = unsafe { MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST) };
    let Some((hwnd, bar)) = BARS.with_borrow(|bars| {
        bars.iter()
            .find(|b| unsafe { MonitorFromWindow(b.hwnd, MONITOR_DEFAULTTONEAREST) } == monitor)
            .or(bars.first())
            .map(|b| (b.hwnd, b.bar))
    }) else {
        return;
    };
    if crate::start_enabled() {
        note_start_rect(hwnd, bar);
        crate::click_row(task_monitor_of(hwnd), 0, Button::Left);
    }
}

fn bar_for(hwnd: HWND) -> Option<AppBar> {
    BARS.with_borrow(|bars| bars.iter().find(|b| b.hwnd == hwnd).map(|b| b.bar))
}

fn task_monitor_of(hwnd: HWND) -> Option<RECT> {
    BARS.with_borrow(|bars| {
        bars.iter()
            .find(|b| b.hwnd == hwnd)
            .and_then(|b| b.task_monitor)
    })
}

fn background_of(hwnd: HWND) -> Color {
    BARS.with_borrow(|bars| {
        bars.iter()
            .find(|b| b.hwnd == hwnd)
            .map_or(Color::rgb(0.0, 0.0, 0.0), |b| b.background)
    })
}

fn client_size(hwnd: HWND) -> (u32, u32) {
    let mut rc = RECT::default();
    unsafe {
        let _ = GetClientRect(hwnd, &mut rc);
    }
    (
        (rc.right - rc.left).max(1) as u32,
        (rc.bottom - rc.top).max(1) as u32,
    )
}

/// Runs `f` on the bar's renderer. The renderer is taken out of `BARS` meanwhile, because drawing
/// can dispatch nested messages (appbar notifications) that borrow `BARS` again.
fn with_renderer(hwnd: HWND, f: impl FnOnce(&mut D2dRenderer) -> Result<()>) {
    let taken = BARS.with_borrow_mut(|bars| {
        bars.iter_mut()
            .find(|b| b.hwnd == hwnd)
            .and_then(|b| b.renderer.take())
    });
    let Some(mut renderer) = taken else {
        return;
    };
    let result = f(&mut renderer);
    BARS.with_borrow_mut(|bars| {
        if let Some(bar) = bars.iter_mut().find(|b| b.hwnd == hwnd) {
            bar.renderer = Some(renderer);
        }
    });
    if let Err(e) = result {
        log(&format!("render failed: {e}"));
    }
}

fn paint(hwnd: HWND, bar: AppBar) {
    let (width, height) = client_size(hwnd);
    let scale = unsafe { GetDpiForWindow(hwnd) } as f32 / 96.0;
    let time = unsafe { GetLocalTime() };
    let clock_time = jtb_core::clock::Time {
        year: time.wYear,
        month: time.wMonth,
        weekday: time.wDayOfWeek,
        day: time.wDay,
        hour: time.wHour,
        minute: time.wMinute,
        second: time.wSecond,
    };
    let clock_fmt = crate::clock_format();
    let clock_lines = jtb_core::clock::render(&clock_fmt, &clock_time);
    let clock_extent = jtb_core::clock::extent(&clock_fmt, bar.edge().is_horizontal());
    let (hover, dragged_from, blend, pressed, index, tray_hover) = BARS.with_borrow(|bars| {
        bars.iter()
            .find(|b| b.hwnd == hwnd)
            .map(|b| {
                (
                    b.hover,
                    b.press.filter(|_| b.dragging).map(|(from, _, _)| from),
                    b.start_blend,
                    b.press.is_some_and(|(index, _, _)| index == 0),
                    b.index,
                    b.tray_hover,
                )
            })
            .unwrap_or((None, None, 0.0, false, 0, None))
    });
    let tray_views = crate::tray_views(index, tray_hover);
    let widget_views = crate::widget_views(index, tray_views.len(), tray_hover);
    let scene = build_scene_with_widgets(
        bar.edge(),
        width as f32,
        height as f32,
        scale,
        background_of(hwnd),
        &clock_lines,
        clock_extent,
        &crate::bar_views(task_monitor_of(hwnd))
            .into_iter()
            .enumerate()
            .map(|(i, view)| {
                let is_start = view.start.is_some();
                ButtonView {
                    hovered: hover == Some(i),
                    active: view.active || (is_start && pressed),
                    drag: if is_start {
                        Drag::None
                    } else {
                        dragged_from.map_or(Drag::None, |from| Drag::of(i, from, hover))
                    },
                    start: view.start.map(|s| StartView {
                        blend,
                        pressed,
                        ..s
                    }),
                    ..view
                }
            })
            .collect::<Vec<_>>(),
        &tray_views,
        &widget_views,
    );
    with_renderer(hwnd, |r| r.draw(&scene));
}

/// Moves the Start button's blend one tick toward hot (pointer on it) or normal, and stops the
/// timer once it got there.
fn fade_step(hwnd: HWND) {
    let settled = BARS.with_borrow_mut(|bars| {
        let Some(bar) = bars.iter_mut().find(|b| b.hwnd == hwnd) else {
            return true;
        };
        let target = if bar.hover == Some(0) { 1.0 } else { 0.0 };
        let next = if bar.start_blend < target {
            (bar.start_blend + FADE_STEP).min(target)
        } else {
            (bar.start_blend - FADE_STEP).max(target)
        };
        bar.start_blend = next;
        next == target
    });
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, false);
        if settled {
            let _ = KillTimer(Some(hwnd), TIMER_START_FADE);
        }
    }
}

fn point_of(lparam: LPARAM) -> (i32, i32) {
    (
        (lparam.0 & 0xFFFF) as i16 as i32,
        ((lparam.0 >> 16) & 0xFFFF) as i16 as i32,
    )
}

/// What is under the mouse: a button, or an icon of the notification area.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Hit {
    Nothing,
    Button(usize),
    Tray(usize),
    Clock,
}

fn layout_of(hwnd: HWND, bar: AppBar) -> BarLayout {
    let (width, height) = client_size(hwnd);
    let scale = unsafe { GetDpiForWindow(hwnd) } as f32 / 96.0;
    let views = crate::bar_views(task_monitor_of(hwnd));
    let index = bar_index(hwnd);
    let tray = crate::tray_views(index, None);
    let lengths: Vec<f32> = crate::widget_views(index, tray.len(), None)
        .iter()
        .map(|w| w.length)
        .collect();
    let clock_fmt = crate::clock_format();
    let clock_extent = jtb_core::clock::extent(&clock_fmt, bar.edge().is_horizontal());
    bar_layout_with_widgets(
        bar.edge(),
        width as f32,
        height as f32,
        scale,
        views.len(),
        start_art_size(&views),
        tray.len(),
        &lengths,
        clock_extent,
    )
}

fn clock_anchor(hwnd: HWND, bar: AppBar) -> Option<jtb_win::flyout::Anchor> {
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetWindowRect};
    flyout_anchor().or_else(|| {
        let mut bar_rect = RECT::default();
        unsafe { GetWindowRect(hwnd, &mut bar_rect) }.ok()?;
        let mut click = POINT::default();
        unsafe { GetCursorPos(&mut click) }.ok()?;
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
        unsafe { GetMonitorInfoW(monitor, &mut info) }.ok().ok()?;
        Some(jtb_win::flyout::Anchor {
            window: hwnd,
            edge: bar.edge(),
            bar: bar_rect,
            monitor: info.rcMonitor,
            click,
        })
    })
}

/// The screen rectangle of the icon drawn in cell `at` of the notification area on the bar with
/// config index `index`.
pub fn tray_icon_screen_rect(index: usize, at: usize) -> Option<RECT> {
    let (hwnd, bar) = BARS.with_borrow(|bars| {
        bars.iter()
            .find(|b| b.index == index)
            .map(|b| (b.hwnd, b.bar))
    })?;
    let cell = layout_of(hwnd, bar).tray.get(at).copied()?;
    let icon = tray_icon_rect(cell, unsafe { GetDpiForWindow(hwnd) } as f32 / 96.0);
    let mut origin = POINT { x: 0, y: 0 };
    unsafe {
        let _ = ClientToScreen(hwnd, &mut origin);
    }
    Some(RECT {
        left: origin.x + icon.left as i32,
        top: origin.y + icon.top as i32,
        right: origin.x + icon.right as i32,
        bottom: origin.y + icon.bottom as i32,
    })
}

/// Tells the app where the Start button is on screen, so the menu can be opened next to it.
fn note_start_rect(hwnd: HWND, bar: AppBar) {
    let Some(rect) = layout_of(hwnd, bar).rows.first().copied() else {
        return;
    };
    let mut origin = POINT { x: 0, y: 0 };
    unsafe {
        let _ = ClientToScreen(hwnd, &mut origin);
    }
    let mut window = RECT::default();
    unsafe {
        let _ = GetWindowRect(hwnd, &mut window);
    }
    crate::note_start_bar(bar.edge(), window, hwnd);
    crate::note_start_rect(RECT {
        left: origin.x + rect.left as i32,
        top: origin.y + rect.top as i32,
        right: origin.x + rect.right as i32,
        bottom: origin.y + rect.bottom as i32,
    });
}

fn hit_at(hwnd: HWND, bar: AppBar, lparam: LPARAM) -> Hit {
    let (x, y) = point_of(lparam);
    let (x, y) = (x as f32, y as f32);
    let layout = layout_of(hwnd, bar);
    if let Some(at) = hit_test(&layout.tray, x, y) {
        Hit::Tray(at)
    } else if let Some(at) = hit_test(&layout.rows, x, y) {
        Hit::Button(at)
    } else {
        let (width, height) = client_size(hwnd);
        let scale = unsafe { GetDpiForWindow(hwnd) } as f32 / 96.0;
        let clock_fmt = crate::clock_format();
        let clock_extent = jtb_core::clock::extent(&clock_fmt, bar.edge().is_horizontal());
        let clock = jtb_core::buttons::clock_rect_sized(
            bar.edge(),
            width as f32,
            height as f32,
            scale,
            clock_extent,
        );
        if x >= clock.left && x <= clock.right && y >= clock.top && y <= clock.bottom {
            Hit::Clock
        } else {
            Hit::Nothing
        }
    }
}

/// Shows the tooltip of tray icon `at` on the side of its cell that faces into the screen.
fn show_tray_tip(hwnd: HWND, bar: AppBar, at: usize) {
    let Some(text) = crate::tray_tooltip(at) else {
        jtb_win::tooltip::hide();
        return;
    };
    let Some(cell) = layout_of(hwnd, bar).tray.get(at).copied() else {
        return;
    };
    let mut origin = POINT { x: 0, y: 0 };
    unsafe {
        let _ = ClientToScreen(hwnd, &mut origin);
    }
    let (left, right) = (origin.x + cell.left as i32, origin.x + cell.right as i32);
    let (top, bottom) = (origin.y + cell.top as i32, origin.y + cell.bottom as i32);
    const GAP: i32 = 6;
    let shown = jtb_win::tooltip::show(hwnd, &text, |w, h| match bar.edge() {
        Edge::Bottom => ((left + right - w) / 2, top - h - GAP),
        Edge::Top => ((left + right - w) / 2, bottom + GAP),
        Edge::Left => (right + GAP, (top + bottom - h) / 2),
        Edge::Right => (left - w - GAP, (top + bottom - h) / 2),
    });
    if !shown {
        log("tray: tooltip window could not be created");
    }
}

fn button_at(hwnd: HWND, bar: AppBar, lparam: LPARAM) -> Option<usize> {
    match hit_at(hwnd, bar, lparam) {
        Hit::Button(at) => Some(at),
        _ => None,
    }
}

fn bar_index(hwnd: HWND) -> usize {
    BARS.with_borrow(|bars| bars.iter().find(|b| b.hwnd == hwnd).map_or(0, |b| b.index))
}

/// Records or clears the left-button press; returns what was recorded before.
fn set_press(hwnd: HWND, press: Option<(usize, i32, i32)>) -> (Option<(usize, i32, i32)>, bool) {
    BARS.with_borrow_mut(|bars| {
        let Some(bar) = bars.iter_mut().find(|b| b.hwnd == hwnd) else {
            return (None, false);
        };
        let before = (bar.press, bar.dragging);
        bar.press = press;
        bar.dragging = false;
        before
    })
}

/// A press that moved a few pixels is a drag, not a click.
fn note_drag(hwnd: HWND, lparam: LPARAM) {
    let (x, y) = point_of(lparam);
    BARS.with_borrow_mut(|bars| {
        if let Some(bar) = bars.iter_mut().find(|b| b.hwnd == hwnd) {
            if let Some((_, px, py)) = bar.press {
                if !bar.dragging && ((x - px).abs() > 6 || (y - py).abs() > 6) {
                    bar.dragging = true;
                    unsafe {
                        let _ = InvalidateRect(Some(hwnd), None, false);
                    }
                }
            }
        }
    });
}

fn click(hwnd: HWND, bar: AppBar, lparam: LPARAM, button: Button) {
    if let Some(index) = button_at(hwnd, bar, lparam) {
        crate::click_row(task_monitor_of(hwnd), index, button);
    }
}

/// Moves the hover highlight, and asks for `WM_MOUSELEAVE` once per entry into the window.
fn hover_to(hwnd: HWND, hover: Option<usize>, tray: Option<usize>) -> Option<usize> {
    let (changed, start_tracking, hot_changed, before) = BARS.with_borrow_mut(|bars| {
        let Some(bar) = bars.iter_mut().find(|b| b.hwnd == hwnd) else {
            return (false, false, false, None);
        };
        let start = (hover.is_some() || tray.is_some()) && !bar.tracking;
        bar.tracking |= start;
        let changed = bar.hover != hover || bar.tray_hover != tray;
        let hot_changed = (bar.hover == Some(0)) != (hover == Some(0));
        let before = bar.tray_hover;
        bar.hover = hover;
        bar.tray_hover = tray;
        (changed, start, hot_changed, before)
    });
    // Re-arming the timer on every mouse move would keep it from ever firing.
    if hot_changed && crate::start_enabled() {
        unsafe {
            SetTimer(Some(hwnd), TIMER_START_FADE, FADE_TICK_MS, None);
        }
    }
    if start_tracking {
        let mut track = TRACKMOUSEEVENT {
            cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
            dwFlags: TME_LEAVE,
            hwndTrack: hwnd,
            dwHoverTime: 0,
        };
        unsafe {
            let _ = TrackMouseEvent(&mut track);
        }
    }
    if changed {
        unsafe {
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
    }
    before
}

fn taskbar_created_message() -> u32 {
    static MESSAGE: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
    *MESSAGE.get_or_init(|| unsafe { RegisterWindowMessageW(w!("TaskbarCreated")) })
}

extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let bar = bar_for(hwnd);
    unsafe {
        match msg {
            _ if msg == taskbar_created_message() => {
                if let Some(bar) = bar {
                    log(&format!(
                        "TaskbarCreated: bar registered again {}",
                        bar.reregister()
                    ));
                    log(&jtb_win::explorer::hide_minimized_windows());
                }
                LRESULT(0)
            }
            _ if bar.is_some_and(|b| b.callback_message() == msg) => {
                if let Some(bar) = bar {
                    bar.on_callback(wparam.0, lparam.0);
                }
                LRESULT(0)
            }
            // A click must not take the foreground from the window the button stands for, or
            // clicking the active window's button could never minimize it.
            WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
            WM_ACTIVATE => {
                if let Some(bar) = bar {
                    bar.activate();
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_WINDOWPOSCHANGED => {
                if let Some(bar) = bar {
                    bar.window_pos_changed();
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            WM_LBUTTONDOWN => {
                if let Some(bar) = bar {
                    if let Hit::Tray(at) = hit_at(hwnd, bar, lparam) {
                        jtb_win::tooltip::hide();
                        crate::tray_press(at, Side::Left);
                        return LRESULT(0);
                    }
                    let index = button_at(hwnd, bar, lparam);
                    let (x, y) = point_of(lparam);
                    set_press(hwnd, index.map(|i| (i, x, y)));
                    if index.is_some() {
                        SetCapture(hwnd);
                    }
                }
                LRESULT(0)
            }
            WM_LBUTTONUP => {
                let _ = ReleaseCapture();
                if let Some(bar) = bar {
                    match hit_at(hwnd, bar, lparam) {
                        Hit::Tray(at) => {
                            crate::tray_mouse(at, Mouse::Up(Side::Left));
                            return LRESULT(0);
                        }
                        Hit::Clock => {
                            if let Some(anchor) = clock_anchor(hwnd, bar) {
                                jtb_win::calendar::toggle(&anchor, crate::clock_months());
                            }
                            return LRESULT(0);
                        }
                        _ => {}
                    }
                    let (press, dragging) = set_press(hwnd, None);
                    if dragging {
                        let _ = InvalidateRect(Some(hwnd), None, false);
                    }
                    let target = button_at(hwnd, bar, lparam);
                    let monitor = task_monitor_of(hwnd);
                    match press {
                        Some((from, _, _)) if dragging => {
                            log(&format!("drag from button {from} to {target:?}"));
                            if let Some(to) = target {
                                crate::reorder(monitor, from, to);
                            }
                        }
                        Some((from, _, _)) if Some(from) == target => {
                            if from == 0 && crate::start_enabled() {
                                note_start_rect(hwnd, bar);
                            }
                            crate::click_row(monitor, from, Button::Left);
                        }
                        _ => {}
                    }
                }
                LRESULT(0)
            }
            WM_RBUTTONDOWN => {
                if let Some(bar) = bar {
                    if let Hit::Tray(at) = hit_at(hwnd, bar, lparam) {
                        jtb_win::tooltip::hide();
                        crate::tray_press(at, Side::Right);
                    }
                }
                LRESULT(0)
            }
            WM_RBUTTONUP => {
                if let Some(bar) = bar {
                    match hit_at(hwnd, bar, lparam) {
                        Hit::Tray(at) => crate::tray_mouse(at, Mouse::Up(Side::Right)),
                        Hit::Button(index) => {
                            crate::context_menu(task_monitor_of(hwnd), index, hwnd);
                        }
                        Hit::Nothing | Hit::Clock => crate::bar_menu(hwnd),
                    }
                }
                LRESULT(0)
            }
            WM_MBUTTONUP => {
                if let Some(bar) = bar {
                    if let Hit::Tray(at) = hit_at(hwnd, bar, lparam) {
                        crate::tray_mouse(at, Mouse::Up(Side::Middle));
                    }
                }
                LRESULT(0)
            }
            WM_DROPFILES => {
                crate::pin_files(jtb_win::shell::dropped_files(wparam.0));
                LRESULT(0)
            }
            WM_MBUTTONDOWN => {
                if let Some(bar) = bar {
                    if let Hit::Tray(at) = hit_at(hwnd, bar, lparam) {
                        crate::tray_press(at, Side::Middle);
                    } else {
                        click(hwnd, bar, lparam, Button::Middle);
                    }
                }
                LRESULT(0)
            }
            WM_MOUSEMOVE => {
                note_drag(hwnd, lparam);
                if let Some(bar) = bar {
                    let (button, tray) = match hit_at(hwnd, bar, lparam) {
                        Hit::Button(at) => (Some(at), None),
                        Hit::Tray(at) => (None, Some(at)),
                        Hit::Nothing | Hit::Clock => (None, None),
                    };
                    let before = hover_to(hwnd, button, tray);
                    crate::tray_hover_events(before, tray);
                    if before != tray {
                        match tray {
                            Some(at) => show_tray_tip(hwnd, bar, at),
                            None => jtb_win::tooltip::hide(),
                        }
                    }
                }
                LRESULT(0)
            }
            WM_MOUSELEAVE => {
                BARS.with_borrow_mut(|bars| {
                    if let Some(bar) = bars.iter_mut().find(|b| b.hwnd == hwnd) {
                        bar.tracking = false;
                    }
                });
                let before = hover_to(hwnd, None, None);
                crate::tray_hover_events(before, None);
                jtb_win::tooltip::hide();
                LRESULT(0)
            }
            WM_TIMER if wparam.0 == TIMER_START_FADE => {
                fade_step(hwnd);
                LRESULT(0)
            }
            WM_ERASEBKGND => LRESULT(1),
            WM_SIZE => {
                let (width, height) = client_size(hwnd);
                with_renderer(hwnd, |r| r.resize(width, height));
                LRESULT(0)
            }
            WM_PAINT => {
                if let Some(bar) = bar {
                    paint(hwnd, bar);
                }
                let _ = ValidateRect(Some(hwnd), None);
                LRESULT(0)
            }
            WM_KEYDOWN if wparam.0 == VK_ESCAPE => {
                crate::quit();
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}
