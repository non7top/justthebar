//! A popup menu listing the installed keyboard layouts, opened next to the language widget.
//! Choosing one asks the window that had the focus to switch; when it ignores the request, Win+Space
//! is pressed as many times as it takes.
use std::cell::RefCell;
use std::time::{Duration, Instant};

use jtb_core::Edge;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Globalization::{GetLocaleInfoW, LOCALE_SISO639LANGNAME2, LOCALE_SLANGUAGE};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyboardLayout, GetKeyboardLayoutList, SendInput, HKL, INPUT, INPUT_0, INPUT_KEYBOARD,
    KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_LWIN, VK_SPACE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetForegroundWindow, GetGUIThreadInfo,
    GetWindowThreadProcessId, PostMessageW, SetForegroundWindow, TrackPopupMenu, GUITHREADINFO,
    MF_CHECKED, MF_STRING, TPM_BOTTOMALIGN, TPM_CENTERALIGN, TPM_LEFTALIGN, TPM_NONOTIFY,
    TPM_RETURNCMD, TPM_RIGHTALIGN, TPM_TOPALIGN, TPM_VCENTERALIGN, WM_INPUTLANGCHANGEREQUEST,
    WM_NULL,
};

use crate::flyout::Anchor;

struct Pending {
    window: HWND,
    target: HKL,
    steps: usize,
    since: Instant,
}

thread_local! {
    static PENDING: RefCell<Option<Pending>> = const { RefCell::new(None) };
}

fn locale_text(language: u32, kind: u32) -> String {
    let mut buffer = [0u16; 96];
    let len = unsafe { GetLocaleInfoW(language, kind, Some(&mut buffer)) };
    if len > 1 {
        String::from_utf16_lossy(&buffer[..len as usize - 1])
    } else {
        String::new()
    }
}

fn layout_of(window: HWND) -> HKL {
    let thread = unsafe { GetWindowThreadProcessId(window, None) };
    unsafe { GetKeyboardLayout(thread) }
}

fn layouts() -> Vec<HKL> {
    let count = unsafe { GetKeyboardLayoutList(None) };
    if count <= 0 {
        return Vec::new();
    }
    let mut list = vec![HKL::default(); count as usize];
    let count = unsafe { GetKeyboardLayoutList(Some(&mut list)) } as usize;
    list.truncate(count);
    list
}

/// Shows the menu at `anchor` and acts on the choice. Returns a line for the log.
pub fn show(anchor: &Anchor) -> String {
    let previous = unsafe { GetForegroundWindow() };
    let current = layout_of(previous);
    let list = layouts();
    if list.is_empty() {
        return "language: no keyboard layouts found".to_string();
    }
    let Ok(menu) = (unsafe { CreatePopupMenu() }) else {
        return "language: cannot create the menu".to_string();
    };
    for (i, layout) in list.iter().enumerate() {
        let language = layout.0 as usize as u32 & 0xFFFF;
        let code = locale_text(language, LOCALE_SISO639LANGNAME2).to_uppercase();
        let text = format!("{code}   {}", locale_text(language, LOCALE_SLANGUAGE));
        let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
        let flags = if *layout == current {
            MF_STRING | MF_CHECKED
        } else {
            MF_STRING
        };
        let _ = unsafe { AppendMenuW(menu, flags, i + 1, PCWSTR(wide.as_ptr())) };
    }
    let (x, y, align) = match anchor.edge {
        Edge::Right => (
            anchor.bar.left,
            anchor.click.y,
            TPM_RIGHTALIGN | TPM_VCENTERALIGN,
        ),
        Edge::Left => (
            anchor.bar.right,
            anchor.click.y,
            TPM_LEFTALIGN | TPM_VCENTERALIGN,
        ),
        Edge::Bottom => (
            anchor.click.x,
            anchor.bar.top,
            TPM_CENTERALIGN | TPM_BOTTOMALIGN,
        ),
        Edge::Top => (
            anchor.click.x,
            anchor.bar.bottom,
            TPM_CENTERALIGN | TPM_TOPALIGN,
        ),
    };
    // The menu closes when you click elsewhere only if its owner is the foreground window.
    let _ = unsafe { SetForegroundWindow(anchor.window) };
    let chosen = unsafe {
        TrackPopupMenu(
            menu,
            align | TPM_RETURNCMD | TPM_NONOTIFY,
            x,
            y,
            None,
            anchor.window,
            None,
        )
    }
    .0;
    let _ = unsafe { PostMessageW(Some(anchor.window), WM_NULL, WPARAM(0), LPARAM(0)) };
    let _ = unsafe { DestroyMenu(menu) };
    if !previous.0.is_null() {
        let _ = unsafe { SetForegroundWindow(previous) };
    }
    let Some(target) = usize::try_from(chosen)
        .ok()
        .and_then(|id| id.checked_sub(1))
        .and_then(|at| list.get(at).copied())
    else {
        return "language: menu closed without a choice".to_string();
    };
    if target == current {
        return "language: that layout is already active".to_string();
    }
    let from = list.iter().position(|l| *l == current).unwrap_or(0);
    let to = list.iter().position(|l| *l == target).unwrap_or(0);
    let mut info = GUITHREADINFO {
        cbSize: size_of::<GUITHREADINFO>() as u32,
        ..Default::default()
    };
    let thread = unsafe { GetWindowThreadProcessId(previous, None) };
    let _ = unsafe { GetGUIThreadInfo(thread, &mut info) };
    for window in [previous, info.hwndFocus] {
        if !window.0.is_null() {
            let _ = unsafe {
                PostMessageW(
                    Some(window),
                    WM_INPUTLANGCHANGEREQUEST,
                    WPARAM(0),
                    LPARAM(target.0 as isize),
                )
            };
        }
    }
    PENDING.set(Some(Pending {
        window: previous,
        target,
        steps: (to + list.len() - from) % list.len(),
        since: Instant::now(),
    }));
    format!(
        "language: asked window {:#x} to switch {from} -> {to}",
        previous.0 as usize
    )
}

fn key(code: VIRTUAL_KEY, flags: KEYBD_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: code,
                dwFlags: flags,
                ..Default::default()
            },
        },
    }
}

/// Call often. When the window did not take the layout request, presses Win+Space instead.
pub fn tick() -> Option<String> {
    let (window, target, steps, since) =
        PENDING.with_borrow(|p| p.as_ref().map(|p| (p.window, p.target, p.steps, p.since)))?;
    if since.elapsed() < Duration::from_millis(300) {
        return None;
    }
    PENDING.set(None);
    if layout_of(window) == target {
        return Some("language: the window switched".to_string());
    }
    let mut inputs = vec![key(VK_LWIN, KEYBD_EVENT_FLAGS(0))];
    for _ in 0..steps.max(1) {
        inputs.push(key(VK_SPACE, KEYBD_EVENT_FLAGS(0)));
        inputs.push(key(VK_SPACE, KEYEVENTF_KEYUP));
    }
    inputs.push(key(VK_LWIN, KEYEVENTF_KEYUP));
    unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
    Some(format!(
        "language: the window ignored the request, pressed Win+Space {steps} times"
    ))
}
