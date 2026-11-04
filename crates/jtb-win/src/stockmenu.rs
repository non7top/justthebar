//! A built-in stock flat start menu popup widget, styled matching the classic two-column layout.
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use jtb_core::Edge;
use windows::core::{w, BOOL, HSTRING, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::DwmGetColorizationColor;
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreatePen,
    CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, EndPaint, FillRect, LineTo, MoveToEx,
    RoundRect, SelectObject, SetBkMode, SetTextColor, ANTIALIASED_QUALITY, CLIP_DEFAULT_PRECIS,
    DEFAULT_CHARSET, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, FW_BOLD, FW_NORMAL, HBRUSH,
    HDC, HFONT, HGDIOBJ, HPEN, OUT_DEFAULT_PRECIS, PAINTSTRUCT, PS_SOLID, SRCCOPY, TRANSPARENT,
};
use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    keybd_event, SetFocus, TrackMouseEvent, KEYEVENTF_KEYUP, TME_LEAVE, TRACKMOUSEEVENT, VK_ESCAPE,
    VK_LWIN,
};
use windows::Win32::UI::Shell::{
    ExtractIconExW, SHGetFileInfoW, ShellExecuteW, SHFILEINFOW, SHGFI_ICON, SHGFI_SMALLICON,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyWindow, DrawIconEx, GetClientRect,
    LoadCursorW, RegisterClassW, SetForegroundWindow, SetWindowPos, ShowWindow, CS_DROPSHADOW,
    DI_NORMAL, HICON, IDC_ARROW, SWP_NOZORDER, SW_HIDE, SW_SHOW, SW_SHOWNORMAL, WA_INACTIVE,
    WM_ACTIVATE, WM_DESTROY, WM_KEYDOWN, WM_KILLFOCUS, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
    WM_MOUSEWHEEL, WM_PAINT, WNDCLASSW, WS_CLIPCHILDREN, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

const CLASS_NAME: windows::core::PCWSTR = w!("JtbStockMenuPopup");
const WM_MOUSELEAVE: u32 = 0x02A3;

const BASE_WIDTH: i32 = 410;
const BASE_LEFT_WIDTH: i32 = 236;
const BASE_HEIGHT: i32 = 420;
const BASE_ITEM_HEIGHT: i32 = 36;
const BASE_RIGHT_ITEM_HEIGHT: i32 = 28;
const BASE_AVATAR_SIZE: i32 = 46;

#[derive(Clone, Debug)]
pub enum LeftAction {
    Launch { target: String, args: String },
    WindowsStart,
    AllPrograms,
    Back,
}

#[derive(Clone, Debug)]
pub struct LeftItem {
    pub name: String,
    pub action: LeftAction,
    pub has_chevron: bool,
    pub icon_path: Option<String>,
    pub is_separator_after: bool,
}

#[derive(Clone, Debug)]
pub enum RightAction {
    Shell(String),
    Command { exe: String, args: String },
    RunDialog,
    ShutDown,
    Restart,
    Lock,
    SignOut,
}

#[derive(Clone, Debug)]
pub struct RightItem {
    pub label: String,
    pub action: RightAction,
    pub has_chevron: bool,
    pub is_separator_after: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum HoverTarget {
    Left(usize),
    Right(usize),
    ShutDown,
    ShutDownArrow,
}

struct MenuState {
    popup: HWND,
    dpi_scale: f32,
    all_programs: bool,
    scroll_offset: i32,
    hovered: Option<HoverTarget>,
    username: String,
    left_items: Vec<LeftItem>,
    all_programs_items: Vec<LeftItem>,
    right_items: Vec<RightItem>,
    icons: HashMap<String, isize>,
}

thread_local! {
    static STATE: RefCell<Option<MenuState>> = const { RefCell::new(None) };
    static LAST_CLOSED: RefCell<Option<Instant>> = const { RefCell::new(None) };
}

fn current_username() -> String {
    std::env::var("USERNAME").unwrap_or_else(|_| "User".to_string())
}

fn appdata_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(PathBuf::from)
}

fn programdata_dir() -> Option<PathBuf> {
    std::env::var_os("ProgramData")
        .or_else(|| std::env::var_os("ALLUSERSPROFILE"))
        .map(PathBuf::from)
}

fn scan_pinned_items() -> Vec<LeftItem> {
    let mut items = Vec::new();
    if let Some(appdata) = appdata_dir() {
        let pinned = appdata.join("OpenShell").join("Pinned");
        if pinned.is_dir() {
            if let Ok(entries) = std::fs::read_dir(pinned) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("lnk") {
                        let stem = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_string();
                        if stem.eq_ignore_ascii_case("startscreen") {
                            items.push(LeftItem {
                                name: "Start Menu (Windows)".to_string(),
                                action: LeftAction::WindowsStart,
                                has_chevron: false,
                                icon_path: None,
                                is_separator_after: false,
                            });
                        } else {
                            items.push(LeftItem {
                                name: stem,
                                action: LeftAction::Launch {
                                    target: path.to_string_lossy().to_string(),
                                    args: String::new(),
                                },
                                has_chevron: false,
                                icon_path: Some(path.to_string_lossy().to_string()),
                                is_separator_after: false,
                            });
                        }
                    }
                }
            }
        }
    }

    if items.is_empty() {
        items.push(LeftItem {
            name: "PlanetSide 2".to_string(),
            action: LeftAction::Launch {
                target: "PlanetSide 2.lnk".to_string(),
                args: String::new(),
            },
            has_chevron: false,
            icon_path: None,
            is_separator_after: false,
        });
        items.push(LeftItem {
            name: "Start Menu (Windows)".to_string(),
            action: LeftAction::WindowsStart,
            has_chevron: false,
            icon_path: None,
            is_separator_after: false,
        });
    }

    if let Some(last) = items.last_mut() {
        last.is_separator_after = true;
    }
    items
}

fn build_normal_left_items() -> Vec<LeftItem> {
    let mut items = scan_pinned_items();

    items.push(LeftItem {
        name: "Calculator".to_string(),
        action: LeftAction::Launch {
            target: "calc.exe".to_string(),
            args: String::new(),
        },
        has_chevron: true,
        icon_path: Some(r"C:\Windows\System32\calc.exe".to_string()),
        is_separator_after: false,
    });
    items.push(LeftItem {
        name: "Paint".to_string(),
        action: LeftAction::Launch {
            target: "mspaint.exe".to_string(),
            args: String::new(),
        },
        has_chevron: false,
        icon_path: Some(r"C:\Windows\System32\mspaint.exe".to_string()),
        is_separator_after: false,
    });
    items.push(LeftItem {
        name: "Settings".to_string(),
        action: LeftAction::Launch {
            target: "ms-settings:".to_string(),
            args: String::new(),
        },
        has_chevron: false,
        icon_path: None,
        is_separator_after: false,
    });
    items.push(LeftItem {
        name: "Snipping Tool".to_string(),
        action: LeftAction::Launch {
            target: "snippingtool.exe".to_string(),
            args: String::new(),
        },
        has_chevron: false,
        icon_path: Some(r"C:\Windows\System32\SnippingTool.exe".to_string()),
        is_separator_after: false,
    });
    items.push(LeftItem {
        name: "Notepad".to_string(),
        action: LeftAction::Launch {
            target: "notepad.exe".to_string(),
            args: String::new(),
        },
        has_chevron: false,
        icon_path: Some(r"C:\Windows\notepad.exe".to_string()),
        is_separator_after: false,
    });
    items.push(LeftItem {
        name: "Windows PowerShell".to_string(),
        action: LeftAction::Launch {
            target: "powershell.exe".to_string(),
            args: String::new(),
        },
        has_chevron: true,
        icon_path: Some(r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe".to_string()),
        is_separator_after: true,
    });
    items.push(LeftItem {
        name: "All Programs".to_string(),
        action: LeftAction::AllPrograms,
        has_chevron: true,
        icon_path: None,
        is_separator_after: false,
    });

    items
}

fn scan_all_programs() -> Vec<LeftItem> {
    let mut dirs_to_scan = Vec::new();
    if let Some(appdata) = appdata_dir() {
        dirs_to_scan.push(
            appdata
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }
    if let Some(progdata) = programdata_dir() {
        dirs_to_scan.push(
            progdata
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs"),
        );
    }

    let mut found = Vec::new();
    for base in dirs_to_scan {
        if !base.is_dir() {
            continue;
        }
        collect_programs(&base, &mut found);
    }

    found.sort_by_key(|a| a.name.to_lowercase());
    found.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));

    found.push(LeftItem {
        name: "Back".to_string(),
        action: LeftAction::Back,
        has_chevron: false,
        icon_path: None,
        is_separator_after: false,
    });

    found
}

fn collect_programs(dir: &Path, out: &mut Vec<LeftItem>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if name.eq_ignore_ascii_case("Startup") || name.starts_with('.') {
                continue;
            }
            out.push(LeftItem {
                name,
                action: LeftAction::Launch {
                    target: path.to_string_lossy().to_string(),
                    args: String::new(),
                },
                has_chevron: true,
                icon_path: None,
                is_separator_after: false,
            });
        } else if path.extension().and_then(|s| s.to_str()) == Some("lnk") {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if name.eq_ignore_ascii_case("desktop") {
                continue;
            }
            out.push(LeftItem {
                name,
                action: LeftAction::Launch {
                    target: path.to_string_lossy().to_string(),
                    args: String::new(),
                },
                has_chevron: false,
                icon_path: Some(path.to_string_lossy().to_string()),
                is_separator_after: false,
            });
        }
    }
}

fn build_right_items() -> Vec<RightItem> {
    vec![
        RightItem {
            label: "Documents".to_string(),
            action: RightAction::Shell("shell:Personal".to_string()),
            has_chevron: false,
            is_separator_after: false,
        },
        RightItem {
            label: "Pictures".to_string(),
            action: RightAction::Shell("shell:My Pictures".to_string()),
            has_chevron: false,
            is_separator_after: false,
        },
        RightItem {
            label: "Music".to_string(),
            action: RightAction::Shell("shell:My Music".to_string()),
            has_chevron: false,
            is_separator_after: true,
        },
        RightItem {
            label: "Recent Items".to_string(),
            action: RightAction::Shell("shell:Recent".to_string()),
            has_chevron: true,
            is_separator_after: false,
        },
        RightItem {
            label: "This PC".to_string(),
            action: RightAction::Shell("shell:MyComputerFolder".to_string()),
            has_chevron: false,
            is_separator_after: true,
        },
        RightItem {
            label: "Settings".to_string(),
            action: RightAction::Command {
                exe: "ms-settings:".to_string(),
                args: String::new(),
            },
            has_chevron: false,
            is_separator_after: false,
        },
        RightItem {
            label: "&Control Panel".to_string(),
            action: RightAction::Command {
                exe: "control.exe".to_string(),
                args: String::new(),
            },
            has_chevron: true,
            is_separator_after: false,
        },
        RightItem {
            label: "Devices and Printers".to_string(),
            action: RightAction::Shell("shell:PrintersFolder".to_string()),
            has_chevron: false,
            is_separator_after: false,
        },
        RightItem {
            label: "Default Programs".to_string(),
            action: RightAction::Command {
                exe: "control.exe".to_string(),
                args: "/name Microsoft.DefaultPrograms".to_string(),
            },
            has_chevron: false,
            is_separator_after: false,
        },
        RightItem {
            label: "&Help and Support".to_string(),
            action: RightAction::Command {
                exe: "ms-get-help:".to_string(),
                args: String::new(),
            },
            has_chevron: false,
            is_separator_after: false,
        },
        RightItem {
            label: "&Run...".to_string(),
            action: RightAction::RunDialog,
            has_chevron: false,
            is_separator_after: false,
        },
    ]
}

fn load_icon(path: &str) -> Option<HICON> {
    let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
    let mut large = HICON::default();
    let count = unsafe { ExtractIconExW(PCWSTR(wide.as_ptr()), 0, Some(&mut large), None, 1) };
    if count > 0 && !large.is_invalid() {
        return Some(large);
    }
    let mut sfi = SHFILEINFOW::default();
    let res = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut sfi),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_SMALLICON,
        )
    };
    if res != 0 && !sfi.hIcon.is_invalid() {
        return Some(sfi.hIcon);
    }
    None
}

fn execute_left(action: &LeftAction) {
    match action {
        LeftAction::WindowsStart => unsafe {
            keybd_event(VK_LWIN.0 as u8, 0, Default::default(), 0);
            keybd_event(VK_LWIN.0 as u8, 0, KEYEVENTF_KEYUP, 0);
        },
        LeftAction::Launch { target, args } => {
            let target_w = HSTRING::from(target.as_str());
            let args_w = if args.is_empty() {
                None
            } else {
                Some(HSTRING::from(args.as_str()))
            };
            unsafe {
                ShellExecuteW(
                    None,
                    w!("open"),
                    &target_w,
                    args_w
                        .as_ref()
                        .map(|s| PCWSTR(s.as_ptr()))
                        .unwrap_or(PCWSTR::null()),
                    PCWSTR::null(),
                    SW_SHOWNORMAL,
                );
            }
        }
        LeftAction::AllPrograms | LeftAction::Back => {}
    }
}

fn execute_right(action: &RightAction) {
    match action {
        RightAction::Shell(path) => {
            let wide = HSTRING::from(path.as_str());
            unsafe {
                ShellExecuteW(
                    None,
                    w!("open"),
                    &wide,
                    PCWSTR::null(),
                    PCWSTR::null(),
                    SW_SHOWNORMAL,
                );
            }
        }
        RightAction::Command { exe, args } => {
            let exe_w = HSTRING::from(exe.as_str());
            let args_w = if args.is_empty() {
                None
            } else {
                Some(HSTRING::from(args.as_str()))
            };
            unsafe {
                ShellExecuteW(
                    None,
                    w!("open"),
                    &exe_w,
                    args_w
                        .as_ref()
                        .map(|s| PCWSTR(s.as_ptr()))
                        .unwrap_or(PCWSTR::null()),
                    PCWSTR::null(),
                    SW_SHOWNORMAL,
                );
            }
        }
        RightAction::RunDialog => unsafe {
            keybd_event(VK_LWIN.0 as u8, 0, Default::default(), 0);
            keybd_event(b'R', 0, Default::default(), 0);
            keybd_event(b'R', 0, KEYEVENTF_KEYUP, 0);
            keybd_event(VK_LWIN.0 as u8, 0, KEYEVENTF_KEYUP, 0);
        },
        RightAction::ShutDown => {
            let cmd = HSTRING::from("shutdown.exe");
            let args = HSTRING::from("/s /t 0");
            unsafe {
                ShellExecuteW(None, w!("open"), &cmd, &args, PCWSTR::null(), SW_HIDE);
            }
        }
        RightAction::Restart => {
            let cmd = HSTRING::from("shutdown.exe");
            let args = HSTRING::from("/r /t 0");
            unsafe {
                ShellExecuteW(None, w!("open"), &cmd, &args, PCWSTR::null(), SW_HIDE);
            }
        }
        RightAction::Lock => {
            let cmd = HSTRING::from("rundll32.exe");
            let args = HSTRING::from("user32.dll,LockWorkStation");
            unsafe {
                ShellExecuteW(None, w!("open"), &cmd, &args, PCWSTR::null(), SW_HIDE);
            }
        }
        RightAction::SignOut => {
            let cmd = HSTRING::from("shutdown.exe");
            let args = HSTRING::from("/l");
            unsafe {
                ShellExecuteW(None, w!("open"), &cmd, &args, PCWSTR::null(), SW_HIDE);
            }
        }
    }
}

fn accent_color() -> COLORREF {
    let mut color = 0u32;
    let mut opaque = BOOL(0);
    if unsafe { DwmGetColorizationColor(&mut color, &mut opaque) }.is_ok() && color != 0 {
        let r = (color >> 16) & 0xff;
        let g = (color >> 8) & 0xff;
        let b = color & 0xff;
        COLORREF(r | (g << 8) | (b << 16))
    } else {
        COLORREF(122 | (69 << 8) | (73 << 16))
    }
}

fn register_class() {
    static REGISTERED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if REGISTERED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    let Ok(instance) = (unsafe { GetModuleHandleW(None) }) else {
        return;
    };
    let wnd_class = WNDCLASSW {
        style: CS_DROPSHADOW,
        lpfnWndProc: Some(wnd_proc),
        hInstance: instance.into(),
        hCursor: unsafe { LoadCursorW(None, IDC_ARROW) }.unwrap_or_default(),
        lpszClassName: CLASS_NAME,
        ..Default::default()
    };
    unsafe {
        let _ = RegisterClassW(&wnd_class);
    }
}

fn close_popup(hwnd: HWND) {
    STATE.with_borrow_mut(|s| {
        if let Some(state) = s.take() {
            for (_, handle) in state.icons {
                unsafe {
                    let _ = DestroyIcon(HICON(handle as *mut _));
                }
            }
        }
    });
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
    LAST_CLOSED.with_borrow_mut(|c| *c = Some(Instant::now()));
}

pub fn toggle(window: HWND, edge: Edge, _bar: RECT, button: RECT, monitor: RECT) {
    if let Some(hwnd) = STATE.with_borrow(|s| s.as_ref().map(|s| s.popup)) {
        close_popup(hwnd);
        return;
    }
    if let Some(closed) = LAST_CLOSED.with_borrow(|c| *c) {
        if closed.elapsed() < Duration::from_millis(250) {
            return;
        }
    }

    register_class();

    let Ok(instance) = (unsafe { GetModuleHandleW(None) }) else {
        return;
    };
    let popup = match unsafe {
        CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            CLASS_NAME,
            w!("StartMenu"),
            WS_POPUP | WS_CLIPCHILDREN,
            0,
            0,
            BASE_WIDTH,
            BASE_HEIGHT,
            Some(window),
            None,
            Some(instance.into()),
            None,
        )
    } {
        Ok(h) => h,
        Err(_) => return,
    };

    let dpi_scale = unsafe { GetDpiForWindow(popup) } as f32 / 96.0;
    let width = (BASE_WIDTH as f32 * dpi_scale).round() as i32;
    let height = (BASE_HEIGHT as f32 * dpi_scale).round() as i32;

    let (mut x, mut y) = match edge {
        Edge::Bottom => (button.left, button.top - height),
        Edge::Top => (button.left, button.bottom),
        Edge::Left => (button.right, button.top),
        Edge::Right => (button.left - width, button.top),
    };

    x = x.clamp(monitor.left, monitor.right - width);
    y = y.clamp(monitor.top, monitor.bottom - height);

    unsafe {
        let _ = SetWindowPos(popup, None, x, y, width, height, SWP_NOZORDER);
    }

    let left_items = build_normal_left_items();
    let right_items = build_right_items();
    let username = current_username();

    let mut icons = HashMap::new();
    for item in &left_items {
        if let Some(path) = &item.icon_path {
            if !icons.contains_key(path) {
                if let Some(hicon) = load_icon(path) {
                    icons.insert(path.clone(), hicon.0 as isize);
                }
            }
        }
    }

    STATE.with_borrow_mut(|s| {
        *s = Some(MenuState {
            popup,
            dpi_scale,
            all_programs: false,
            scroll_offset: 0,
            hovered: None,
            username,
            left_items,
            all_programs_items: Vec::new(),
            right_items,
            icons,
        });
    });

    unsafe {
        let _ = ShowWindow(popup, SW_SHOW);
        let _ = SetForegroundWindow(popup);
        let _ = SetFocus(Some(popup));
    }
}

fn hit_test(
    pt: POINT,
    left_items: &[LeftItem],
    right_items: &[RightItem],
    all_programs: bool,
    scroll: i32,
    scale: f32,
    size: (i32, i32),
) -> Option<HoverTarget> {
    let (width, height) = size;
    let left_w = (BASE_LEFT_WIDTH as f32 * scale).round() as i32;
    let item_h = (BASE_ITEM_HEIGHT as f32 * scale).round() as i32;
    let right_item_h = (BASE_RIGHT_ITEM_HEIGHT as f32 * scale).round() as i32;
    let avatar_h = (BASE_AVATAR_SIZE as f32 * scale).round() as i32 + (24.0 * scale) as i32;
    let bottom_h = (36.0 * scale).round() as i32;

    if pt.x < left_w {
        if all_programs && !left_items.is_empty() {
            let back_top = height - item_h;
            if pt.y >= back_top {
                return Some(HoverTarget::Left(left_items.len() - 1));
            }
            let list_y = pt.y + scroll - (4.0 * scale) as i32;
            let idx = (list_y / item_h) as usize;
            if idx < left_items.len().saturating_sub(1) {
                return Some(HoverTarget::Left(idx));
            }
        } else {
            let mut cur_y = (4.0 * scale) as i32;
            for (i, item) in left_items.iter().enumerate() {
                if pt.y >= cur_y && pt.y < cur_y + item_h {
                    return Some(HoverTarget::Left(i));
                }
                cur_y += item_h;
                if item.is_separator_after {
                    cur_y += (6.0 * scale) as i32;
                }
            }
        }
    } else {
        if pt.y >= height - bottom_h {
            let arrow_w = (28.0 * scale) as i32;
            if pt.x >= width - arrow_w {
                return Some(HoverTarget::ShutDownArrow);
            } else {
                return Some(HoverTarget::ShutDown);
            }
        }

        let mut cur_y = avatar_h;
        for (i, item) in right_items.iter().enumerate() {
            if pt.y >= cur_y && pt.y < cur_y + right_item_h {
                return Some(HoverTarget::Right(i));
            }
            cur_y += right_item_h;
            if item.is_separator_after {
                cur_y += (6.0 * scale) as i32;
            }
        }
    }
    None
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_ACTIVATE => {
            if (wparam.0 & 0xffff) == WA_INACTIVE as usize {
                close_popup(hwnd);
            }
            LRESULT(0)
        }
        WM_KILLFOCUS => {
            close_popup(hwnd);
            LRESULT(0)
        }
        WM_KEYDOWN => {
            if wparam.0 == VK_ESCAPE.0 as usize {
                close_popup(hwnd);
            }
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam.0 >> 16) as i16) as i32;
            STATE.with_borrow_mut(|s| {
                if let Some(state) = s.as_mut() {
                    if state.all_programs {
                        let step = (BASE_ITEM_HEIGHT as f32 * state.dpi_scale * 2.0) as i32;
                        let item_h = (BASE_ITEM_HEIGHT as f32 * state.dpi_scale).round() as i32;
                        let total_h = state.all_programs_items.len() as i32 * item_h;
                        let max_scroll =
                            (total_h - (BASE_HEIGHT as f32 * state.dpi_scale) as i32).max(0);
                        if delta > 0 {
                            state.scroll_offset = (state.scroll_offset - step).max(0);
                        } else {
                            state.scroll_offset = (state.scroll_offset + step).min(max_scroll);
                        }
                        let _ =
                            windows::Win32::Graphics::Gdi::InvalidateRect(Some(hwnd), None, false);
                    }
                }
            });
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let x = (lparam.0 & 0xffff) as i16 as i32;
            let y = ((lparam.0 >> 16) & 0xffff) as i16 as i32;
            let pt = POINT { x, y };

            let mut tme = TRACKMOUSEEVENT {
                cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                dwFlags: TME_LEAVE,
                hwndTrack: hwnd,
                dwHoverTime: 0,
            };
            let _ = TrackMouseEvent(&mut tme);

            let mut changed = false;
            STATE.with_borrow_mut(|s| {
                if let Some(state) = s.as_mut() {
                    let mut rc = RECT::default();
                    let _ = GetClientRect(hwnd, &mut rc);
                    let w = rc.right - rc.left;
                    let h = rc.bottom - rc.top;
                    let active_items = if state.all_programs {
                        &state.all_programs_items
                    } else {
                        &state.left_items
                    };
                    let target = hit_test(
                        pt,
                        active_items,
                        &state.right_items,
                        state.all_programs,
                        state.scroll_offset,
                        state.dpi_scale,
                        (w, h),
                    );
                    if target != state.hovered {
                        state.hovered = target;
                        changed = true;
                    }
                }
            });
            if changed {
                let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(hwnd), None, false);
            }
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            let mut changed = false;
            STATE.with_borrow_mut(|s| {
                if let Some(state) = s.as_mut() {
                    if state.hovered.is_some() {
                        state.hovered = None;
                        changed = true;
                    }
                }
            });
            if changed {
                let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(hwnd), None, false);
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => LRESULT(0),
        WM_LBUTTONUP => {
            let x = (lparam.0 & 0xffff) as i16 as i32;
            let y = ((lparam.0 >> 16) & 0xffff) as i16 as i32;
            let pt = POINT { x, y };

            let mut action_left = None;
            let mut action_right = None;
            let mut toggle_all = false;
            let mut toggle_back = false;

            STATE.with_borrow_mut(|s| {
                if let Some(state) = s.as_mut() {
                    let mut rc = RECT::default();
                    let _ = GetClientRect(hwnd, &mut rc);
                    let w = rc.right - rc.left;
                    let h = rc.bottom - rc.top;
                    let active_items = if state.all_programs {
                        &state.all_programs_items
                    } else {
                        &state.left_items
                    };
                    if let Some(target) = hit_test(
                        pt,
                        active_items,
                        &state.right_items,
                        state.all_programs,
                        state.scroll_offset,
                        state.dpi_scale,
                        (w, h),
                    ) {
                        match target {
                            HoverTarget::Left(idx) => {
                                if let Some(item) = active_items.get(idx) {
                                    match &item.action {
                                        LeftAction::AllPrograms => toggle_all = true,
                                        LeftAction::Back => toggle_back = true,
                                        act => action_left = Some(act.clone()),
                                    }
                                }
                            }
                            HoverTarget::Right(idx) => {
                                if let Some(item) = state.right_items.get(idx) {
                                    action_right = Some(item.action.clone());
                                }
                            }
                            HoverTarget::ShutDown => {
                                action_right = Some(RightAction::ShutDown);
                            }
                            HoverTarget::ShutDownArrow => {
                                action_right = Some(RightAction::Restart);
                            }
                        }
                    }
                }
            });

            if toggle_all {
                STATE.with_borrow_mut(|s| {
                    if let Some(state) = s.as_mut() {
                        if state.all_programs_items.is_empty() {
                            state.all_programs_items = scan_all_programs();
                        }
                        state.all_programs = true;
                        state.scroll_offset = 0;
                    }
                });
                let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(hwnd), None, false);
                return LRESULT(0);
            }

            if toggle_back {
                STATE.with_borrow_mut(|s| {
                    if let Some(state) = s.as_mut() {
                        state.all_programs = false;
                        state.scroll_offset = 0;
                    }
                });
                let _ = windows::Win32::Graphics::Gdi::InvalidateRect(Some(hwnd), None, false);
                return LRESULT(0);
            }

            if let Some(act) = action_left {
                close_popup(hwnd);
                execute_left(&act);
                return LRESULT(0);
            }

            if let Some(act) = action_right {
                close_popup(hwnd);
                execute_right(&act);
                return LRESULT(0);
            }

            LRESULT(0)
        }
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut client = RECT::default();
            let _ = GetClientRect(hwnd, &mut client);
            let width = client.right - client.left;
            let height = client.bottom - client.top;

            let mem_dc = CreateCompatibleDC(Some(hdc));
            let mem_bm = CreateCompatibleBitmap(hdc, width, height);
            let old_bm = SelectObject(mem_dc, HGDIOBJ(mem_bm.0));

            paint_menu(mem_dc, width, height);

            let _ = BitBlt(hdc, 0, 0, width, height, Some(mem_dc), 0, 0, SRCCOPY);

            let _ = SelectObject(mem_dc, old_bm);
            let _ = DeleteObject(HGDIOBJ(mem_bm.0));
            let _ = DeleteDC(mem_dc);
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        WM_DESTROY => {
            close_popup(hwnd);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn paint_menu(hdc: HDC, width: i32, height: i32) {
    STATE.with_borrow(|s| {
        let Some(state) = s.as_ref() else {
            return;
        };
        let scale = state.dpi_scale;
        let left_w = (BASE_LEFT_WIDTH as f32 * scale).round() as i32;
        let item_h = (BASE_ITEM_HEIGHT as f32 * scale).round() as i32;
        let right_item_h = (BASE_RIGHT_ITEM_HEIGHT as f32 * scale).round() as i32;
        let pad_x = (8.0 * scale).round() as i32;

        unsafe {
            let white_brush = CreateSolidBrush(COLORREF(0x00FFFFFF));
            let left_rect = RECT {
                left: 0,
                top: 0,
                right: left_w,
                bottom: height,
            };
            FillRect(hdc, &left_rect, white_brush);
            let _ = DeleteObject(HGDIOBJ(white_brush.0));

            let accent = accent_color();
            let right_brush = CreateSolidBrush(accent);
            let right_rect = RECT {
                left: left_w,
                top: 0,
                right: width,
                bottom: height,
            };
            FillRect(hdc, &right_rect, right_brush);
            let _ = DeleteObject(HGDIOBJ(right_brush.0));

            let border_pen = CreatePen(PS_SOLID, 1, COLORREF(0x00A0A0A0));
            let old_pen = SelectObject(hdc, HGDIOBJ(border_pen.0));
            let _ = MoveToEx(hdc, left_w, 0, None);
            let _ = LineTo(hdc, left_w, height);

            let _ = SetBkMode(hdc, TRANSPARENT);

            let font_h = -(12.0 * scale).round() as i32;
            let normal_face: Vec<u16> = "Segoe UI\0".encode_utf16().collect();
            let normal_font = CreateFontW(
                font_h,
                0,
                0,
                0,
                FW_NORMAL.0 as i32,
                0,
                0,
                0,
                DEFAULT_CHARSET,
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                ANTIALIASED_QUALITY,
                0,
                PCWSTR(normal_face.as_ptr()),
            );
            let bold_face: Vec<u16> = "Segoe UI\0".encode_utf16().collect();
            let bold_font = CreateFontW(
                font_h - 2,
                0,
                0,
                0,
                FW_BOLD.0 as i32,
                0,
                0,
                0,
                DEFAULT_CHARSET,
                OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS,
                ANTIALIASED_QUALITY,
                0,
                PCWSTR(bold_face.as_ptr()),
            );
            let old_font = SelectObject(hdc, HGDIOBJ(normal_font.0));

            let hover_brush = CreateSolidBrush(COLORREF(0x00FBE1E5));
            let hover_pen = CreatePen(PS_SOLID, 1, COLORREF(0x00E7C070));
            let hover_style = (hover_brush, hover_pen);

            let active_left = if state.all_programs {
                &state.all_programs_items
            } else {
                &state.left_items
            };

            if state.all_programs && !active_left.is_empty() {
                let back_top = height - item_h;
                let back_idx = active_left.len() - 1;
                let mut cur_y = (4.0 * scale) as i32 - state.scroll_offset;

                for (i, item) in active_left.iter().enumerate().take(back_idx) {
                    if cur_y + item_h > 0 && cur_y < back_top {
                        let item_rect = RECT {
                            left: pad_x,
                            top: cur_y,
                            right: left_w - pad_x,
                            bottom: cur_y + item_h,
                        };
                        draw_left_item(
                            hdc,
                            item,
                            &item_rect,
                            state.hovered == Some(HoverTarget::Left(i)),
                            hover_style,
                            scale,
                            &state.icons,
                        );
                    }
                    cur_y += item_h;
                }

                let sep_pen = CreatePen(PS_SOLID, 1, COLORREF(0x00D8D8D8));
                let _ = SelectObject(hdc, HGDIOBJ(sep_pen.0));
                let _ = MoveToEx(hdc, pad_x, back_top - 2, None);
                let _ = LineTo(hdc, left_w - pad_x, back_top - 2);
                let _ = DeleteObject(HGDIOBJ(sep_pen.0));

                let back_rect = RECT {
                    left: pad_x,
                    top: back_top,
                    right: left_w - pad_x,
                    bottom: height - 4,
                };
                if let Some(back_item) = active_left.get(back_idx) {
                    draw_left_item(
                        hdc,
                        back_item,
                        &back_rect,
                        state.hovered == Some(HoverTarget::Left(back_idx)),
                        hover_style,
                        scale,
                        &state.icons,
                    );
                }
            } else {
                let mut cur_y = (4.0 * scale) as i32;
                for (i, item) in active_left.iter().enumerate() {
                    let item_rect = RECT {
                        left: pad_x,
                        top: cur_y,
                        right: left_w - pad_x,
                        bottom: cur_y + item_h,
                    };
                    draw_left_item(
                        hdc,
                        item,
                        &item_rect,
                        state.hovered == Some(HoverTarget::Left(i)),
                        hover_style,
                        scale,
                        &state.icons,
                    );
                    cur_y += item_h;
                    if item.is_separator_after {
                        let sep_pen = CreatePen(PS_SOLID, 1, COLORREF(0x00D8D8D8));
                        let _ = SelectObject(hdc, HGDIOBJ(sep_pen.0));
                        let _ = MoveToEx(hdc, pad_x, cur_y + 2, None);
                        let _ = LineTo(hdc, left_w - pad_x, cur_y + 2);
                        let _ = DeleteObject(HGDIOBJ(sep_pen.0));
                        cur_y += (6.0 * scale) as i32;
                    }
                }
            }

            let _ = DeleteObject(HGDIOBJ(hover_brush.0));
            let _ = DeleteObject(HGDIOBJ(hover_pen.0));

            draw_right_pane(
                hdc,
                state,
                (left_w, width, height),
                right_item_h,
                scale,
                bold_font,
            );

            let outer_pen = CreatePen(PS_SOLID, 1, COLORREF(0x00808080));
            let _ = SelectObject(hdc, HGDIOBJ(outer_pen.0));
            let _ = MoveToEx(hdc, 0, 0, None);
            let _ = LineTo(hdc, width - 1, 0);
            let _ = LineTo(hdc, width - 1, height - 1);
            let _ = LineTo(hdc, 0, height - 1);
            let _ = LineTo(hdc, 0, 0);

            let _ = SelectObject(hdc, old_pen);
            let _ = DeleteObject(HGDIOBJ(outer_pen.0));
            let _ = DeleteObject(HGDIOBJ(border_pen.0));
            let _ = SelectObject(hdc, old_font);
            let _ = DeleteObject(HGDIOBJ(normal_font.0));
            let _ = DeleteObject(HGDIOBJ(bold_font.0));
        }
    });
}

unsafe fn draw_left_item(
    hdc: HDC,
    item: &LeftItem,
    rect: &RECT,
    hovered: bool,
    hover_style: (HBRUSH, HPEN),
    scale: f32,
    icons: &HashMap<String, isize>,
) {
    let (hover_brush, hover_pen) = hover_style;
    if hovered {
        let old_p = SelectObject(hdc, HGDIOBJ(hover_pen.0));
        let old_b = SelectObject(hdc, HGDIOBJ(hover_brush.0));
        let _ = RoundRect(hdc, rect.left, rect.top, rect.right, rect.bottom, 6, 6);
        let _ = SelectObject(hdc, old_b);
        let _ = SelectObject(hdc, old_p);
    }

    let icon_size = (20.0 * scale).round() as i32;
    let icon_x = rect.left + (6.0 * scale) as i32;
    let icon_y = rect.top + (rect.bottom - rect.top - icon_size) / 2;

    if let Some(path) = &item.icon_path {
        if let Some(&handle) = icons.get(path) {
            let hicon = HICON(handle as *mut _);
            let _ = DrawIconEx(
                hdc, icon_x, icon_y, hicon, icon_size, icon_size, 0, None, DI_NORMAL,
            );
        } else {
            draw_app_glyph(hdc, icon_x, icon_y, icon_size);
        }
    } else if matches!(item.action, LeftAction::WindowsStart) {
        draw_windows_glyph(hdc, icon_x, icon_y, icon_size);
    } else if matches!(item.action, LeftAction::Back) {
        draw_back_glyph(hdc, icon_x, icon_y, icon_size);
    } else {
        draw_app_glyph(hdc, icon_x, icon_y, icon_size);
    }

    SetTextColor(hdc, COLORREF(0x00111111));
    let text_x = icon_x + icon_size + (8.0 * scale) as i32;
    let mut text_rect = RECT {
        left: text_x,
        top: rect.top,
        right: rect.right - (16.0 * scale) as i32,
        bottom: rect.bottom,
    };
    let wide: Vec<u16> = item.name.encode_utf16().collect();
    let _ = DrawTextW(
        hdc,
        &mut wide.clone(),
        &mut text_rect,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );

    if item.has_chevron {
        let mut chev_rect = RECT {
            left: rect.right - (16.0 * scale) as i32,
            top: rect.top,
            right: rect.right - 4,
            bottom: rect.bottom,
        };
        let chev: Vec<u16> = "\u{25b8}".encode_utf16().collect();
        SetTextColor(hdc, COLORREF(0x00666666));
        let _ = DrawTextW(
            hdc,
            &mut chev.clone(),
            &mut chev_rect,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
    }
}

unsafe fn draw_right_pane(
    hdc: HDC,
    state: &MenuState,
    dims: (i32, i32, i32),
    item_h: i32,
    scale: f32,
    bold_font: HFONT,
) {
    let (left_w, width, height) = dims;
    let pad_x = left_w + (12.0 * scale).round() as i32;
    let avatar_size = (BASE_AVATAR_SIZE as f32 * scale).round() as i32;
    let avatar_top = (12.0 * scale).round() as i32;
    let avatar_right = width - (16.0 * scale).round() as i32;
    let avatar_left = avatar_right - avatar_size;

    let white_pen = CreatePen(PS_SOLID, 2, COLORREF(0x00FFFFFF));
    let old_pen = SelectObject(hdc, HGDIOBJ(white_pen.0));
    let _ = RoundRect(
        hdc,
        avatar_left,
        avatar_top,
        avatar_right,
        avatar_top + avatar_size,
        8,
        8,
    );

    let head_size = avatar_size / 3;
    let head_x = avatar_left + (avatar_size - head_size) / 2;
    let head_y = avatar_top + (avatar_size / 6);
    let white_brush = CreateSolidBrush(COLORREF(0x00FFFFFF));
    let old_b = SelectObject(hdc, HGDIOBJ(white_brush.0));
    let _ = RoundRect(
        hdc,
        head_x,
        head_y,
        head_x + head_size,
        head_y + head_size,
        head_size,
        head_size,
    );

    let body_w = (avatar_size * 2) / 3;
    let body_h = avatar_size / 3;
    let body_x = avatar_left + (avatar_size - body_w) / 2;
    let body_y = head_y + head_size + 2;
    let _ = RoundRect(hdc, body_x, body_y, body_x + body_w, body_y + body_h, 6, 6);
    let _ = SelectObject(hdc, old_b);
    let _ = DeleteObject(HGDIOBJ(white_brush.0));
    let _ = SelectObject(hdc, old_pen);
    let _ = DeleteObject(HGDIOBJ(white_pen.0));

    let old_font = SelectObject(hdc, HGDIOBJ(bold_font.0));
    SetTextColor(hdc, COLORREF(0x00FFFFFF));
    let mut name_rect = RECT {
        left: pad_x,
        top: avatar_top + avatar_size + (4.0 * scale) as i32,
        right: width - (12.0 * scale) as i32,
        bottom: avatar_top + avatar_size + (24.0 * scale) as i32,
    };
    let wide_user: Vec<u16> = state.username.encode_utf16().collect();
    let _ = DrawTextW(
        hdc,
        &mut wide_user.clone(),
        &mut name_rect,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
    let _ = SelectObject(hdc, old_font);

    let mut cur_y = name_rect.bottom + (6.0 * scale) as i32;
    let r_hover_brush = CreateSolidBrush(COLORREF(0x00A05860));
    let r_hover_pen = CreatePen(PS_SOLID, 1, COLORREF(0x00C08088));

    for (i, item) in state.right_items.iter().enumerate() {
        let is_hover = state.hovered == Some(HoverTarget::Right(i));
        let item_rect = RECT {
            left: left_w + (6.0 * scale) as i32,
            top: cur_y,
            right: width - (6.0 * scale) as i32,
            bottom: cur_y + item_h,
        };
        if is_hover {
            let op = SelectObject(hdc, HGDIOBJ(r_hover_pen.0));
            let ob = SelectObject(hdc, HGDIOBJ(r_hover_brush.0));
            let _ = RoundRect(
                hdc,
                item_rect.left,
                item_rect.top,
                item_rect.right,
                item_rect.bottom,
                4,
                4,
            );
            let _ = SelectObject(hdc, ob);
            let _ = SelectObject(hdc, op);
        }

        let mut text_rect = RECT {
            left: item_rect.left + (8.0 * scale) as i32,
            top: item_rect.top,
            right: item_rect.right - (16.0 * scale) as i32,
            bottom: item_rect.bottom,
        };
        SetTextColor(hdc, COLORREF(0x00FFFFFF));
        let wide: Vec<u16> = item.label.encode_utf16().collect();
        let _ = DrawTextW(
            hdc,
            &mut wide.clone(),
            &mut text_rect,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE,
        );

        if item.has_chevron {
            let mut chev_rect = RECT {
                left: item_rect.right - (16.0 * scale) as i32,
                top: item_rect.top,
                right: item_rect.right - 4,
                bottom: item_rect.bottom,
            };
            let chev: Vec<u16> = "\u{25b8}".encode_utf16().collect();
            let _ = DrawTextW(
                hdc,
                &mut chev.clone(),
                &mut chev_rect,
                DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
            );
        }

        cur_y += item_h;
        if item.is_separator_after {
            let sep_pen = CreatePen(PS_SOLID, 1, COLORREF(0x00603038));
            let old_p = SelectObject(hdc, HGDIOBJ(sep_pen.0));
            let _ = MoveToEx(hdc, left_w + (8.0 * scale) as i32, cur_y + 2, None);
            let _ = LineTo(hdc, width - (8.0 * scale) as i32, cur_y + 2);
            let _ = SelectObject(hdc, old_p);
            let _ = DeleteObject(HGDIOBJ(sep_pen.0));
            cur_y += (6.0 * scale) as i32;
        }
    }

    let _ = DeleteObject(HGDIOBJ(r_hover_brush.0));
    let _ = DeleteObject(HGDIOBJ(r_hover_pen.0));

    let bottom_h = (34.0 * scale).round() as i32;
    let arrow_w = (26.0 * scale).round() as i32;
    let btn_top = height - bottom_h - (4.0 * scale) as i32;
    let btn_bottom = height - (4.0 * scale) as i32;
    let btn_left = left_w + (8.0 * scale) as i32;
    let btn_right = width - (8.0 * scale) as i32;
    let split_x = btn_right - arrow_w;

    let shutdown_hover = state.hovered == Some(HoverTarget::ShutDown);
    let arrow_hover = state.hovered == Some(HoverTarget::ShutDownArrow);

    let btn_brush = CreateSolidBrush(if shutdown_hover {
        COLORREF(0x00B06068)
    } else {
        COLORREF(0x00804048)
    });
    let btn_pen = CreatePen(PS_SOLID, 1, COLORREF(0x00A06068));
    let op = SelectObject(hdc, HGDIOBJ(btn_pen.0));
    let ob = SelectObject(hdc, HGDIOBJ(btn_brush.0));
    let _ = RoundRect(hdc, btn_left, btn_top, split_x, btn_bottom, 4, 4);
    let _ = SelectObject(hdc, ob);
    let _ = DeleteObject(HGDIOBJ(btn_brush.0));

    let arr_brush = CreateSolidBrush(if arrow_hover {
        COLORREF(0x00B06068)
    } else {
        COLORREF(0x00804048)
    });
    let ob = SelectObject(hdc, HGDIOBJ(arr_brush.0));
    let _ = RoundRect(hdc, split_x + 2, btn_top, btn_right, btn_bottom, 4, 4);
    let _ = SelectObject(hdc, ob);
    let _ = SelectObject(hdc, op);
    let _ = DeleteObject(HGDIOBJ(arr_brush.0));
    let _ = DeleteObject(HGDIOBJ(btn_pen.0));

    SetTextColor(hdc, COLORREF(0x00FFFFFF));
    let mut shut_rect = RECT {
        left: btn_left + 6,
        top: btn_top,
        right: split_x - 4,
        bottom: btn_bottom,
    };
    let shut_txt: Vec<u16> = "\u{23fb} Shut down".encode_utf16().collect();
    let _ = DrawTextW(
        hdc,
        &mut shut_txt.clone(),
        &mut shut_rect,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );

    let mut arr_rect = RECT {
        left: split_x + 2,
        top: btn_top,
        right: btn_right,
        bottom: btn_bottom,
    };
    let arr_txt: Vec<u16> = "\u{25b8}".encode_utf16().collect();
    let _ = DrawTextW(
        hdc,
        &mut arr_txt.clone(),
        &mut arr_rect,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
}

unsafe fn draw_app_glyph(hdc: HDC, x: i32, y: i32, size: i32) {
    let pen = CreatePen(PS_SOLID, 1, COLORREF(0x000078D7));
    let brush = CreateSolidBrush(COLORREF(0x000078D7));
    let op = SelectObject(hdc, HGDIOBJ(pen.0));
    let ob = SelectObject(hdc, HGDIOBJ(brush.0));
    let _ = RoundRect(hdc, x, y, x + size, y + size, 4, 4);
    let _ = SelectObject(hdc, ob);
    let _ = SelectObject(hdc, op);
    let _ = DeleteObject(HGDIOBJ(brush.0));
    let _ = DeleteObject(HGDIOBJ(pen.0));
}

unsafe fn draw_windows_glyph(hdc: HDC, x: i32, y: i32, size: i32) {
    let half = size / 2 - 1;
    let colors = [
        COLORREF(0x00235BE8),
        COLORREF(0x002BB770),
        COLORREF(0x00E88E00),
        COLORREF(0x000CB4F4),
    ];
    let rects = [
        (x, y),
        (x + half + 2, y),
        (x, y + half + 2),
        (x + half + 2, y + half + 2),
    ];
    for (i, &(rx, ry)) in rects.iter().enumerate() {
        let brush = CreateSolidBrush(colors[i]);
        let pen = CreatePen(PS_SOLID, 1, colors[i]);
        let op = SelectObject(hdc, HGDIOBJ(pen.0));
        let ob = SelectObject(hdc, HGDIOBJ(brush.0));
        let _ = RoundRect(hdc, rx, ry, rx + half, ry + half, 2, 2);
        let _ = SelectObject(hdc, ob);
        let _ = SelectObject(hdc, op);
        let _ = DeleteObject(HGDIOBJ(brush.0));
        let _ = DeleteObject(HGDIOBJ(pen.0));
    }
}

unsafe fn draw_back_glyph(hdc: HDC, x: i32, y: i32, size: i32) {
    let pen = CreatePen(PS_SOLID, 2, COLORREF(0x00333333));
    let op = SelectObject(hdc, HGDIOBJ(pen.0));
    let mid_y = y + size / 2;
    let _ = MoveToEx(hdc, x + size - 2, mid_y, None);
    let _ = LineTo(hdc, x + 2, mid_y);
    let _ = LineTo(hdc, x + 6, mid_y - 4);
    let _ = MoveToEx(hdc, x + 2, mid_y, None);
    let _ = LineTo(hdc, x + 6, mid_y + 4);
    let _ = SelectObject(hdc, op);
    let _ = DeleteObject(HGDIOBJ(pen.0));
}
