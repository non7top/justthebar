use std::ffi::c_void;

use jtb_core::state::split_args;
use windows::core::{Interface, HSTRING};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
    STGM_READ,
};
use windows::Win32::UI::Shell::{
    DragAcceptFiles, DragFinish, DragQueryFileW, IShellLinkW, ShellLink, HDROP,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, SetForegroundWindow, TrackPopupMenu,
    MF_ENABLED, MF_GRAYED, MF_STRING, TPM_NONOTIFY, TPM_RETURNCMD,
};

/// Lets files be dropped on the window; they arrive as `WM_DROPFILES`.
pub fn accept_drops(hwnd: HWND) {
    unsafe { DragAcceptFiles(hwnd, true) };
}

/// Paths of the files in a `WM_DROPFILES` message (its `wParam`); releases the drop.
pub fn dropped_files(wparam: usize) -> Vec<String> {
    let drop = HDROP(wparam as *mut c_void);
    let count = unsafe { DragQueryFileW(drop, u32::MAX, None) };
    let mut files = Vec::new();
    for index in 0..count {
        let mut buf = [0u16; 1024];
        let len = unsafe { DragQueryFileW(drop, index, Some(&mut buf)) } as usize;
        files.push(String::from_utf16_lossy(&buf[..len.min(buf.len())]));
    }
    unsafe { DragFinish(drop) };
    files
}

fn until_nul(buf: &[u16]) -> String {
    let end = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

/// Target program and arguments of a `.lnk` shortcut.
pub fn resolve_shortcut(path: &str) -> Option<(String, Vec<String>)> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        let file: IPersistFile = link.cast().ok()?;
        file.Load(&HSTRING::from(path), STGM_READ).ok()?;
        let mut target = [0u16; 1024];
        link.GetPath(&mut target, std::ptr::null_mut(), 0).ok()?;
        let mut args = [0u16; 1024];
        link.GetArguments(&mut args).ok()?;
        let target = until_nul(&target);
        (!target.is_empty()).then(|| (target, split_args(&until_nul(&args))))
    }
}

/// Shows a context menu at the cursor and returns the index of the chosen entry.
pub fn popup_menu(hwnd: HWND, items: &[(&str, bool)]) -> Option<usize> {
    unsafe {
        let menu = CreatePopupMenu().ok()?;
        for (index, (label, enabled)) in items.iter().enumerate() {
            let flags = MF_STRING | if *enabled { MF_ENABLED } else { MF_GRAYED };
            let _ = AppendMenuW(menu, flags, index + 1, &HSTRING::from(*label));
        }
        let mut at = POINT::default();
        let _ = GetCursorPos(&mut at);
        let _ = SetForegroundWindow(hwnd);
        let chosen = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY,
            at.x,
            at.y,
            None,
            hwnd,
            None,
        );
        let _ = DestroyMenu(menu);
        let id = chosen.0 as usize;
        (id > 0).then(|| id - 1)
    }
}
