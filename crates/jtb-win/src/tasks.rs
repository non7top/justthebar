// Derived from ManagedShell.WindowsTasks/ApplicationWindow.cs and TasksService.cs (Apache-2.0),
// ported to Rust.
use std::ffi::c_void;

use jtb_core::icon::{finish_pixels, Icon};
use jtb_core::tasks::Task;
use windows::core::{w, BOOL, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, GetLastError, HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits, GetObjectW, BITMAP, BITMAPINFO,
    BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Shell::{ExtractIconExW, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::{
    AllowSetForegroundWindow, DestroyIcon, EnumWindows, GetClassLongPtrW, GetClassNameW,
    GetForegroundWindow, GetGUIThreadInfo, GetIconInfo, GetPropW, GetWindow, GetWindowLongPtrW,
    GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, PostMessageW,
    RegisterShellHookWindow, RegisterWindowMessageW, SendMessageTimeoutW, SetForegroundWindow,
    ShowWindowAsync, GCLP_HICON, GCLP_HICONSM, GUITHREADINFO, GWL_EXSTYLE, GW_OWNER, HICON,
    ICONINFO, ICON_BIG, ICON_SMALL, ICON_SMALL2, SC_MINIMIZE, SC_RESTORE, SMTO_ABORTIFHUNG,
    SW_MINIMIZE, SW_RESTORE, SW_SHOWNORMAL, WM_CLOSE, WM_GETICON, WM_SYSCOMMAND, WS_EX_APPWINDOW,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_WINDOWEDGE,
};

// Shell hook codes (wParam of the SHELLHOOK message; lParam is the window).
pub const HSHELL_WINDOWCREATED: usize = 1;
pub const HSHELL_WINDOWDESTROYED: usize = 2;
pub const HSHELL_WINDOWACTIVATED: usize = 4;
pub const HSHELL_REDRAW: usize = 6;
pub const HSHELL_WINDOWREPLACED: usize = 13;
pub const HSHELL_RUDEAPPACTIVATED: usize = 0x8004;
pub const HSHELL_FLASH: usize = 0x8006;

pub fn hwnd_of(id: isize) -> HWND {
    HWND(id as *mut c_void)
}

fn class_name(hwnd: HWND) -> String {
    let mut buf = [0u16; 128];
    let len = unsafe { GetClassNameW(hwnd, &mut buf) };
    String::from_utf16_lossy(&buf[..len.max(0) as usize])
}

/// Full path of the executable that owns the window.
pub fn process_path(id: isize) -> Option<String> {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd_of(id), Some(&mut pid)) };
    if pid == 0 {
        return None;
    }
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }.ok()?;
    let mut buf = [0u16; 1024];
    let mut len = buf.len() as u32;
    let read = unsafe {
        QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        )
    };
    unsafe {
        let _ = CloseHandle(process);
    }
    read.is_ok()
        .then(|| String::from_utf16_lossy(&buf[..len as usize]))
}

/// Starts a program or opens a file the way Explorer would. Returns whether Windows accepted it.
pub fn launch(path: &str, args: &[String]) -> bool {
    let params = args
        .iter()
        .map(|arg| {
            if arg.contains(' ') {
                format!("\"{arg}\"")
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    let file = HSTRING::from(path);
    let parameters = HSTRING::from(params);
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            &file,
            &parameters,
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    result.0 as isize > 32
}

pub fn title(id: isize) -> String {
    let mut buf = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd_of(id), &mut buf) };
    String::from_utf16_lossy(&buf[..len.max(0) as usize])
}

/// Whether a window belongs on the taskbar. `CanAddToTaskbar` and the cloaked and immersive-shell
/// checks of ManagedShell's `ApplicationWindow`.
pub fn is_taskbar_window(id: isize) -> bool {
    let hwnd = hwnd_of(id);
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() || !IsWindowVisible(hwnd).as_bool() {
            return false;
        }
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let tool = ex & WS_EX_TOOLWINDOW.0 != 0;
        let app = ex & WS_EX_APPWINDOW.0 != 0;
        let no_activate = ex & WS_EX_NOACTIVATE.0 != 0;
        let has_owner = GetWindow(hwnd, GW_OWNER).is_ok_and(|owner| !owner.is_invalid());
        let deleted = !GetPropW(hwnd, w!("ITaskList_Deleted")).is_invalid();
        if tool || deleted || (has_owner && !app) || (no_activate && !app) {
            return false;
        }

        // Suspended UWP windows still exist but are cloaked.
        let mut cloaked = 0u32;
        let _ = DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut u32 as *mut c_void,
            size_of::<u32>() as u32,
        );
        if cloaked != 0 {
            return false;
        }

        // Shell-owned UWP windows have no window edge.
        let class = class_name(hwnd);
        let immersive = matches!(
            class.as_str(),
            "ApplicationFrameWindow"
                | "Windows.UI.Core.CoreWindow"
                | "StartMenuSizingFrame"
                | "Shell_LightDismissOverlay"
        );
        !(immersive && ex & WS_EX_WINDOWEDGE.0 == 0)
    }
}

unsafe extern "system" fn collect(hwnd: HWND, data: LPARAM) -> BOOL {
    let list = &mut *(data.0 as *mut Vec<Task>);
    let id = hwnd.0 as isize;
    if is_taskbar_window(id) {
        list.push(Task {
            id,
            title: title(id),
            exe: process_path(id).unwrap_or_default(),
            activity: Default::default(),
        });
    }
    BOOL(1)
}

/// All windows that belong on the taskbar right now, in z-order.
pub fn enumerate() -> Vec<Task> {
    let mut list: Vec<Task> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(collect), LPARAM(&mut list as *mut Vec<Task> as isize));
    }
    list
}

pub fn is_minimized(id: isize) -> bool {
    unsafe { IsIconic(hwnd_of(id)) }.as_bool()
}

pub fn foreground() -> Option<isize> {
    let hwnd = unsafe { GetForegroundWindow() };
    (!hwnd.is_invalid()).then_some(hwnd.0 as isize)
}

/// The active window of the thread that has the input focus, which is set even when
/// `GetForegroundWindow` answers null.
pub fn gui_active() -> Option<isize> {
    let mut info = GUITHREADINFO {
        cbSize: size_of::<GUITHREADINFO>() as u32,
        ..Default::default()
    };
    unsafe { GetGUIThreadInfo(0, &mut info) }.ok()?;
    (!info.hwndActive.is_invalid()).then_some(info.hwndActive.0 as isize)
}

/// Asks the shell to send `hwnd` the shell hook messages; returns the message id to listen for.
pub fn register_shell_hook(hwnd: HWND) -> u32 {
    unsafe {
        let _ = RegisterShellHookWindow(hwnd);
        RegisterWindowMessageW(w!("SHELLHOOK"))
    }
}

/// Asks the window to close, as its close button would.
pub fn close(id: isize) {
    unsafe {
        let _ = PostMessageW(Some(hwnd_of(id)), WM_CLOSE, WPARAM(0), LPARAM(0));
    }
}

/// Button click: restore a minimized window, minimize the active one, otherwise bring to front.
/// Returns what was tried, for the log.
///
/// The window gets the system command a title bar button sends, as Cairo does. A window running
/// elevated may refuse `ShowWindowAsync` from this process.
pub fn activate_or_minimize(id: isize, is_active: bool) -> String {
    let hwnd = hwnd_of(id);
    let mut process = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut process));
        let _ = AllowSetForegroundWindow(process);
        if IsIconic(hwnd).as_bool() {
            let mut result = 0usize;
            let sent = SendMessageTimeoutW(
                hwnd,
                WM_SYSCOMMAND,
                WPARAM(SC_RESTORE as usize),
                LPARAM(0),
                SMTO_ABORTIFHUNG,
                200,
                Some(&mut result),
            );
            let note = if sent.0 == 0 {
                let error = GetLastError();
                let _ = ShowWindowAsync(hwnd, SW_RESTORE);
                format!("restore command failed ({error:?}), asked ShowWindowAsync instead")
            } else {
                "restore command sent".to_string()
            };
            let _ = SetForegroundWindow(hwnd);
            note
        } else if is_active {
            match PostMessageW(
                Some(hwnd),
                WM_SYSCOMMAND,
                WPARAM(SC_MINIMIZE as usize),
                LPARAM(0),
            ) {
                Ok(()) => "minimize command posted".to_string(),
                Err(error) => {
                    let _ = ShowWindowAsync(hwnd, SW_MINIMIZE);
                    format!("minimize command failed ({error}), asked ShowWindowAsync instead")
                }
            }
        } else {
            let _ = SetForegroundWindow(hwnd);
            "brought to front".to_string()
        }
    }
}

/// The window's own icon, else its class icon. These handles belong to the window's app.
fn icon_handle(hwnd: HWND) -> Option<HICON> {
    for kind in [ICON_BIG, ICON_SMALL2, ICON_SMALL] {
        let mut result = 0usize;
        let sent = unsafe {
            SendMessageTimeoutW(
                hwnd,
                WM_GETICON,
                WPARAM(kind as usize),
                LPARAM(0),
                SMTO_ABORTIFHUNG,
                50,
                Some(&mut result),
            )
        };
        if sent.0 != 0 && result != 0 {
            return Some(HICON(result as *mut c_void));
        }
    }
    for index in [GCLP_HICON, GCLP_HICONSM] {
        let handle = unsafe { GetClassLongPtrW(hwnd, index) };
        if handle != 0 {
            return Some(HICON(handle as *mut c_void));
        }
    }
    None
}

fn read_pixels(dc: HDC, bitmap: HBITMAP, width: i32, height: i32) -> Option<Vec<u8>> {
    let mut info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let lines = unsafe {
        GetDIBits(
            dc,
            bitmap,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut c_void),
            &mut info,
            DIB_RGB_COLORS,
        )
    };
    (lines == height).then_some(pixels)
}

fn decode_icon(hicon: HICON, icon_id: u64) -> Option<Icon> {
    let mut info = ICONINFO::default();
    unsafe { GetIconInfo(hicon, &mut info).ok()? };
    let icon = (|| {
        if info.hbmColor.is_invalid() {
            return None;
        }
        let mut bitmap = BITMAP::default();
        let got = unsafe {
            GetObjectW(
                HGDIOBJ(info.hbmColor.0),
                size_of::<BITMAP>() as i32,
                Some(&mut bitmap as *mut BITMAP as *mut c_void),
            )
        };
        if got == 0 || bitmap.bmWidth <= 0 || bitmap.bmHeight <= 0 {
            return None;
        }
        let (width, height) = (bitmap.bmWidth, bitmap.bmHeight);
        let dc = unsafe { CreateCompatibleDC(None) };
        let mut color = read_pixels(dc, info.hbmColor, width, height);
        let mask = if info.hbmMask.is_invalid() {
            None
        } else {
            read_pixels(dc, info.hbmMask, width, height)
        };
        unsafe {
            let _ = DeleteDC(dc);
        }
        let pixels = color.as_mut()?;
        finish_pixels(pixels, mask.as_deref());
        Some(Icon {
            id: icon_id,
            width: width as u32,
            height: height as u32,
            bgra: color?,
        })
    })();
    unsafe {
        let _ = DeleteObject(HGDIOBJ(info.hbmColor.0));
        let _ = DeleteObject(HGDIOBJ(info.hbmMask.0));
    }
    icon
}

/// Loads the window's icon as pixels, tagged with `icon_id`.
pub fn icon(id: isize, icon_id: u64) -> Option<Icon> {
    decode_icon(icon_handle(hwnd_of(id))?, icon_id)
}

/// Decodes an icon handle an app sent to the tray; such handles are valid in every process.
pub fn icon_from_handle(handle: isize, icon_id: u64) -> Option<Icon> {
    decode_icon(HICON(handle as *mut c_void), icon_id)
}

/// The icon of an executable or file, tagged with `icon_id`.
pub fn exe_icon(path: &str, icon_id: u64) -> Option<Icon> {
    let file = HSTRING::from(path);
    let mut large = HICON::default();
    let count = unsafe { ExtractIconExW(&file, 0, Some(&mut large), None, 1) };
    if count == 0 || large.is_invalid() {
        return None;
    }
    let icon = decode_icon(large, icon_id);
    unsafe {
        let _ = DestroyIcon(large);
    }
    icon
}
