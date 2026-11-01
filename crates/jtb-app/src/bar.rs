use std::cell::RefCell;

use jtb_core::buttons::{hit_test, layout_buttons};
use jtb_core::config::BarConfig;
use jtb_core::layout::{corner_insets, edge_offsets, Slot};
use jtb_core::scene::{build_scene, Color};
use jtb_ui::{D2dRenderer, Renderer};
use jtb_win::appbar::AppBar;
use windows::core::{w, Result};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{InvalidateRect, ValidateRect};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, LoadCursorW, RegisterClassW,
    SetWindowPos, ShowWindow, HWND_TOPMOST, IDC_ARROW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SW_SHOWNOACTIVATE, WM_ACTIVATE, WM_ERASEBKGND, WM_KEYDOWN, WM_LBUTTONDOWN, WM_PAINT, WM_SIZE,
    WM_WINDOWPOSCHANGED, WNDCLASSW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use crate::log;

const VK_ESCAPE: usize = 0x1B;
const BAR_COLOR: Color = Color::rgb(0.09, 0.10, 0.12);

struct BarWindow {
    hwnd: HWND,
    bar: AppBar,
    monitor: RECT,
    renderer: Option<D2dRenderer>,
}

thread_local! {
    static BARS: RefCell<Vec<BarWindow>> = const { RefCell::new(Vec::new()) };
}

/// A bar to create: its config, the monitor index it resolved to, and that monitor's rect.
pub struct Planned {
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
pub fn build(planned: Vec<Planned>, flush: bool) {
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
                bar,
                monitor: plan.monitor_rect,
                renderer,
            })
        });
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

pub fn invalidate_all() {
    let hwnds: Vec<HWND> = BARS.with_borrow(|bars| bars.iter().map(|b| b.hwnd).collect());
    for hwnd in hwnds {
        unsafe {
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
    }
}

fn bar_for(hwnd: HWND) -> Option<AppBar> {
    BARS.with_borrow(|bars| bars.iter().find(|b| b.hwnd == hwnd).map(|b| b.bar))
}

fn monitor_of(hwnd: HWND) -> Option<RECT> {
    BARS.with_borrow(|bars| bars.iter().find(|b| b.hwnd == hwnd).map(|b| b.monitor))
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
    let scene = build_scene(
        bar.edge(),
        width as f32,
        height as f32,
        scale,
        BAR_COLOR,
        (time.wHour, time.wMinute),
        &crate::bar_tasks(monitor_of(hwnd))
            .into_iter()
            .map(|(_, view)| view)
            .collect::<Vec<_>>(),
    );
    with_renderer(hwnd, |r| r.draw(&scene));
}

fn click(hwnd: HWND, bar: AppBar, lparam: LPARAM) {
    let x = (lparam.0 & 0xFFFF) as i16 as f32;
    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32;
    let (width, height) = client_size(hwnd);
    let scale = unsafe { GetDpiForWindow(hwnd) } as f32 / 96.0;
    let monitor = monitor_of(hwnd);
    let count = crate::bar_tasks(monitor).len();
    let rects = layout_buttons(bar.edge(), width as f32, height as f32, scale, count);
    if let Some(index) = hit_test(&rects, x, y) {
        crate::click_task(monitor, index);
    }
}

extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let bar = bar_for(hwnd);
    unsafe {
        match msg {
            _ if bar.is_some_and(|b| b.callback_message() == msg) => {
                if let Some(bar) = bar {
                    bar.on_callback(wparam.0, lparam.0);
                }
                LRESULT(0)
            }
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
                    click(hwnd, bar, lparam);
                }
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
