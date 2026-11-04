//! The advanced options window: a table of every option in `jtb_core::settings::SCHEMA` with its
//! value and description. Double click (or Enter) edits a value, Save writes `jtb.json` and the
//! running bars pick the file up.
use std::cell::{Cell, RefCell};
use std::path::PathBuf;

use jtb_core::settings::{self, Kind, Row};
use jtb_win::shell;
use serde_json::Value;
use windows::core::{w, HSTRING, PCWSTR, PWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::COLOR_WINDOW;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    InitCommonControlsEx, ICC_LISTVIEW_CLASSES, INITCOMMONCONTROLSEX, LVCFMT_LEFT, LVCF_FMT,
    LVCF_TEXT, LVCF_WIDTH, LVCOLUMNW, LVIF_TEXT, LVITEMW, LVM_GETNEXTITEM, LVM_INSERTCOLUMNW,
    LVM_INSERTITEMW, LVM_SETCOLUMNWIDTH, LVM_SETEXTENDEDLISTVIEWSTYLE, LVM_SETITEMTEXTW,
    LVNI_SELECTED, LVS_EX_FULLROWSELECT, LVS_EX_GRIDLINES, LVS_REPORT, LVS_SHOWSELALWAYS,
    LVS_SINGLESEL, NMHDR, NMITEMACTIVATE, NM_DBLCLK, WC_LISTVIEWW,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, SetFocus};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetMessageW,
    GetWindowTextLengthW, GetWindowTextW, IsDialogMessageW, LoadCursorW, MoveWindow,
    RegisterClassW, SendMessageW, SetForegroundWindow, SetWindowTextW, ShowWindow,
    TranslateMessage, BS_DEFPUSHBUTTON, BS_PUSHBUTTON, CW_USEDEFAULT, ES_AUTOHSCROLL, HMENU,
    IDC_ARROW, MSG, SW_SHOW, WM_CLOSE, WM_COMMAND, WM_CREATE, WM_DESTROY, WM_NOTIFY, WM_SIZE,
    WNDCLASSW, WS_BORDER, WS_CAPTION, WS_CHILD, WS_CLIPCHILDREN, WS_EX_CLIENTEDGE,
    WS_OVERLAPPEDWINDOW, WS_POPUP, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE,
};

use crate::log;

const ID_LIST: isize = 100;
const ID_SAVE: usize = 101;
const ID_CLOSE: usize = 102;
const ID_OK: usize = 1;
const ID_CANCEL: usize = 2;
const COLUMN_WIDTHS: [i32; 3] = [230, 260, 520];
const LVN_KEYDOWN: u32 = (-155i32) as u32;
const VK_RETURN: u16 = 0x0D;
const SS_LEFT: u32 = 0;

struct State {
    window: HWND,
    list: HWND,
    status: HWND,
    path: PathBuf,
    root: Value,
    rows: Vec<Row>,
    dirty: bool,
}

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    static INPUT: RefCell<Option<String>> = const { RefCell::new(None) };
    static INPUT_DONE: Cell<bool> = const { Cell::new(false) };
}

fn wide(text: &str) -> HSTRING {
    HSTRING::from(text)
}

/// Opens the window (or brings the open one forward) for the config file at `path`.
pub fn open(path: PathBuf) {
    if let Some(window) = STATE.with_borrow(|s| s.as_ref().map(|s| s.window)) {
        let _ = unsafe { SetForegroundWindow(window) };
        return;
    }
    let root = match std::fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<Value>(&text) {
            Ok(root) => root,
            Err(e) => {
                log(&format!(
                    "settings: {} is not valid JSON: {e}",
                    path.display()
                ));
                return;
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            serde_json::json!({ "version": jtb_core::config::CONFIG_VERSION })
        }
        Err(e) => {
            log(&format!("settings: cannot read {}: {e}", path.display()));
            return;
        }
    };
    unsafe {
        let _ = InitCommonControlsEx(&INITCOMMONCONTROLSEX {
            dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_LISTVIEW_CLASSES,
        });
        let Ok(instance) = GetModuleHandleW(None) else {
            return;
        };
        let class = WNDCLASSW {
            hInstance: instance.into(),
            lpszClassName: w!("JtbSettings"),
            lpfnWndProc: Some(wnd_proc),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH(
                (COLOR_WINDOW.0 + 1) as isize as _,
            ),
            ..Default::default()
        };
        RegisterClassW(&class);
        let rows = settings::rows(&root);
        STATE.set(Some(State {
            window: HWND::default(),
            list: HWND::default(),
            status: HWND::default(),
            path,
            root,
            rows,
            dirty: false,
        }));
        let created = CreateWindowExW(
            Default::default(),
            w!("JtbSettings"),
            w!("just the bar - advanced options"),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1100,
            640,
            None,
            None,
            Some(instance.into()),
            None,
        );
        match created {
            Ok(window) => {
                let _ = ShowWindow(window, SW_SHOW);
                let _ = SetForegroundWindow(window);
            }
            Err(e) => {
                log(&format!("settings: cannot create the window: {e}"));
                STATE.set(None);
            }
        }
    }
}

fn set_status(text: &str) {
    if let Some(status) = STATE.with_borrow(|s| s.as_ref().map(|s| s.status)) {
        let _ = unsafe { SetWindowTextW(status, &wide(text)) };
    }
}

/// Sets the text of one cell; `insert` creates the row (column 0) first.
fn put(list: HWND, index: usize, column: i32, text: &str, insert: bool) {
    let mut buffer: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    let cell = LVITEMW {
        mask: LVIF_TEXT,
        iItem: index as i32,
        iSubItem: column,
        pszText: PWSTR(buffer.as_mut_ptr()),
        ..Default::default()
    };
    let (message, wparam) = if insert {
        (LVM_INSERTITEMW, 0)
    } else {
        (LVM_SETITEMTEXTW, index)
    };
    unsafe {
        SendMessageW(
            list,
            message,
            Some(WPARAM(wparam)),
            Some(LPARAM(&cell as *const LVITEMW as isize)),
        );
    }
}

fn fill_list() {
    let Some((list, lines)) = STATE.with_borrow(|s| {
        s.as_ref().map(|s| {
            let lines: Vec<(String, String, String)> = s
                .rows
                .iter()
                .map(|row| {
                    (
                        row.path.clone(),
                        settings::get(&s.root, row),
                        row.spec.description.to_string(),
                    )
                })
                .collect();
            (s.list, lines)
        })
    }) else {
        return;
    };
    for (index, (path, value, description)) in lines.iter().enumerate() {
        put(list, index, 0, path, true);
        put(list, index, 1, value, false);
        put(list, index, 2, description, false);
    }
}

fn refresh_value(index: usize) {
    let Some((list, value)) = STATE.with_borrow(|s| {
        s.as_ref().and_then(|s| {
            s.rows
                .get(index)
                .map(|row| (s.list, settings::get(&s.root, row)))
        })
    }) else {
        return;
    };
    put(list, index, 1, &value, false);
}

fn change(index: usize, text: &str) {
    let result = STATE.with_borrow_mut(|s| {
        let state = s.as_mut()?;
        let row = state.rows.get(index)?.clone();
        let result = settings::set(&mut state.root, &row, text).map(|()| row.path);
        if result.is_ok() {
            state.dirty = true;
        }
        Some(result)
    });
    match result {
        Some(Ok(path)) => {
            refresh_value(index);
            set_status(&format!("{path} changed, not saved yet"));
        }
        Some(Err(e)) => set_status(&e),
        None => {}
    }
}

/// Edits the value of row `index`: a switch toggles, a choice opens a menu, anything else asks.
fn edit(index: usize) {
    let Some((window, row, current)) = STATE.with_borrow(|s| {
        s.as_ref().and_then(|s| {
            s.rows
                .get(index)
                .map(|row| (s.window, row.clone(), settings::get(&s.root, row)))
        })
    }) else {
        return;
    };
    match row.spec.kind {
        Kind::Bool => change(index, if current == "true" { "false" } else { "true" }),
        Kind::Choice(words) => {
            let items: Vec<(&str, bool)> = words.iter().map(|word| (*word, true)).collect();
            if let Some(chosen) = shell::popup_menu(window, &items) {
                change(index, words[chosen]);
            }
        }
        _ => {
            let prompt = format!("{}: {}", row.path, row.spec.description);
            if let Some(text) = input_box(window, &prompt, &current) {
                change(index, &text);
            }
        }
    }
}

fn save() {
    let outcome = STATE.with_borrow(|s| {
        let state = s.as_ref()?;
        if let Err(e) = settings::validate(&state.root) {
            return Some(Err(format!("not saved: {e}")));
        }
        let text = serde_json::to_string_pretty(&state.root).ok()?;
        let temp = state.path.with_extension("json.tmp");
        let written = std::fs::write(&temp, text + "\n")
            .and_then(|()| std::fs::rename(&temp, &state.path))
            .map_err(|e| format!("not saved: {}: {e}", state.path.display()));
        Some(written.map(|()| state.path.display().to_string()))
    });
    match outcome {
        Some(Ok(path)) => {
            STATE.with_borrow_mut(|s| {
                if let Some(state) = s.as_mut() {
                    state.dirty = false;
                }
            });
            log(&format!("settings: saved {path}"));
            set_status(&format!("Saved {path}; the bars apply it within a second"));
        }
        Some(Err(e)) => set_status(&e),
        None => {}
    }
}

fn layout(window: HWND) {
    let Some((list, status)) = STATE.with_borrow(|s| s.as_ref().map(|s| (s.list, s.status))) else {
        return;
    };
    let mut client = RECT::default();
    unsafe {
        let _ = GetClientRect(window, &mut client);
        let (width, height) = (client.right, client.bottom);
        let bar = 40;
        let _ = MoveWindow(list, 0, 0, width, (height - bar).max(0), true);
        let _ = MoveWindow(status, 8, height - bar + 12, (width - 220).max(0), 22, true);
        for (id, x) in [(ID_SAVE, width - 190), (ID_CLOSE, width - 100)] {
            if let Ok(button) =
                windows::Win32::UI::WindowsAndMessaging::GetDlgItem(Some(window), id as i32)
            {
                let _ = MoveWindow(button, x, height - bar + 6, 84, 28, true);
            }
        }
        let rest = (width - COLUMN_WIDTHS[0] - COLUMN_WIDTHS[1] - 30).max(200);
        SendMessageW(
            list,
            LVM_SETCOLUMNWIDTH,
            Some(WPARAM(2)),
            Some(LPARAM(rest as isize)),
        );
    }
}

fn create_controls(window: HWND) {
    unsafe {
        let Ok(instance) = GetModuleHandleW(None) else {
            return;
        };
        let list = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            WC_LISTVIEWW,
            PCWSTR::null(),
            WS_CHILD
                | WS_VISIBLE
                | WS_TABSTOP
                | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(
                    LVS_REPORT | LVS_SINGLESEL | LVS_SHOWSELALWAYS,
                ),
            0,
            0,
            100,
            100,
            Some(window),
            Some(HMENU(ID_LIST as _)),
            Some(instance.into()),
            None,
        )
        .unwrap_or_default();
        SendMessageW(
            list,
            LVM_SETEXTENDEDLISTVIEWSTYLE,
            Some(WPARAM(0)),
            Some(LPARAM((LVS_EX_FULLROWSELECT | LVS_EX_GRIDLINES) as isize)),
        );
        for (column, (title, width)) in ["Option", "Value", "Description"]
            .iter()
            .zip(COLUMN_WIDTHS)
            .enumerate()
        {
            let mut text: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
            let spec = LVCOLUMNW {
                mask: LVCF_FMT | LVCF_WIDTH | LVCF_TEXT,
                fmt: LVCFMT_LEFT,
                cx: width,
                pszText: PWSTR(text.as_mut_ptr()),
                ..Default::default()
            };
            SendMessageW(
                list,
                LVM_INSERTCOLUMNW,
                Some(WPARAM(column)),
                Some(LPARAM(&spec as *const LVCOLUMNW as isize)),
            );
        }
        let status = CreateWindowExW(
            Default::default(),
            w!("STATIC"),
            w!("Double click a value to change it, then Save."),
            WS_CHILD | WS_VISIBLE | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(SS_LEFT),
            0,
            0,
            100,
            20,
            Some(window),
            None,
            Some(instance.into()),
            None,
        )
        .unwrap_or_default();
        for (id, label) in [(ID_SAVE, w!("Save")), (ID_CLOSE, w!("Close"))] {
            let _ = CreateWindowExW(
                Default::default(),
                w!("BUTTON"),
                label,
                WS_CHILD
                    | WS_VISIBLE
                    | WS_TABSTOP
                    | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(BS_PUSHBUTTON as u32),
                0,
                0,
                84,
                28,
                Some(window),
                Some(HMENU(id as _)),
                Some(instance.into()),
                None,
            );
        }
        STATE.with_borrow_mut(|s| {
            if let Some(state) = s.as_mut() {
                state.window = window;
                state.list = list;
                state.status = status;
            }
        });
    }
    fill_list();
    layout(window);
}

unsafe extern "system" fn wnd_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_CREATE => {
            create_controls(window);
            LRESULT(0)
        }
        WM_SIZE => {
            layout(window);
            LRESULT(0)
        }
        WM_COMMAND => {
            match wparam.0 & 0xFFFF {
                ID_SAVE => save(),
                ID_CLOSE => {
                    let _ = unsafe { DestroyWindow(window) };
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_NOTIFY => {
            let header = unsafe { &*(lparam.0 as *const NMHDR) };
            if header.idFrom == ID_LIST as usize {
                if header.code == NM_DBLCLK {
                    let activate = unsafe { &*(lparam.0 as *const NMITEMACTIVATE) };
                    if activate.iItem >= 0 {
                        edit(activate.iItem as usize);
                    }
                } else if header.code == LVN_KEYDOWN {
                    let key =
                        unsafe { &*(lparam.0 as *const windows::Win32::UI::Controls::NMLVKEYDOWN) };
                    if key.wVKey == VK_RETURN {
                        let list = header.hwndFrom;
                        let selected = unsafe {
                            SendMessageW(
                                list,
                                LVM_GETNEXTITEM,
                                Some(WPARAM(usize::MAX)),
                                Some(LPARAM(LVNI_SELECTED as isize)),
                            )
                        };
                        if selected.0 >= 0 {
                            edit(selected.0 as usize);
                        }
                    }
                }
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            let dirty = STATE.with_borrow(|s| s.as_ref().is_some_and(|s| s.dirty));
            if dirty {
                set_status("There are unsaved changes: Save, or press Close again to drop them");
                STATE.with_borrow_mut(|s| {
                    if let Some(state) = s.as_mut() {
                        state.dirty = false;
                    }
                });
                return LRESULT(0);
            }
            let _ = unsafe { DestroyWindow(window) };
            LRESULT(0)
        }
        WM_DESTROY => {
            STATE.set(None);
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
    }
}

/// A small modal window with one edit field. Returns the text on OK, none on cancel.
pub fn input_box(owner: HWND, prompt: &str, initial: &str) -> Option<String> {
    unsafe {
        let instance = GetModuleHandleW(None).ok()?;
        let class = WNDCLASSW {
            hInstance: instance.into(),
            lpszClassName: w!("JtbSettingsInput"),
            lpfnWndProc: Some(input_proc),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH(
                (COLOR_WINDOW.0 + 1) as isize as _,
            ),
            ..Default::default()
        };
        RegisterClassW(&class);
        INPUT.set(None);
        INPUT_DONE.set(false);
        let dialog = CreateWindowExW(
            Default::default(),
            w!("JtbSettingsInput"),
            &wide(prompt),
            WS_POPUP | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            720,
            140,
            Some(owner),
            None,
            Some(instance.into()),
            None,
        )
        .ok()?;
        let edit = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            w!("EDIT"),
            &wide(initial),
            WS_CHILD
                | WS_VISIBLE
                | WS_TABSTOP
                | WS_BORDER
                | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(ES_AUTOHSCROLL as u32),
            12,
            14,
            680,
            24,
            Some(dialog),
            Some(HMENU(200 as _)),
            Some(instance.into()),
            None,
        )
        .ok()?;
        for (id, label, x, default) in [
            (ID_OK, w!("OK"), 520, true),
            (ID_CANCEL, w!("Cancel"), 612, false),
        ] {
            let style = if default {
                BS_DEFPUSHBUTTON
            } else {
                BS_PUSHBUTTON
            };
            let _ = CreateWindowExW(
                Default::default(),
                w!("BUTTON"),
                label,
                WS_CHILD
                    | WS_VISIBLE
                    | WS_TABSTOP
                    | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(style as u32),
                x,
                52,
                84,
                28,
                Some(dialog),
                Some(HMENU(id as _)),
                Some(instance.into()),
                None,
            );
        }
        let _ = EnableWindow(owner, false);
        let _ = SetFocus(Some(edit));
        let mut message = MSG::default();
        while !INPUT_DONE.get() && GetMessageW(&mut message, None, 0, 0).as_bool() {
            if !IsDialogMessageW(dialog, &message).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
        let _ = EnableWindow(owner, true);
        let _ = SetForegroundWindow(owner);
        INPUT.take()
    }
}

unsafe extern "system" fn input_proc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_COMMAND => {
            match wparam.0 & 0xFFFF {
                ID_OK => {
                    if let Ok(edit) = unsafe {
                        windows::Win32::UI::WindowsAndMessaging::GetDlgItem(Some(window), 200)
                    } {
                        let length = unsafe { GetWindowTextLengthW(edit) } as usize;
                        let mut buffer = vec![0u16; length + 1];
                        let copied = unsafe { GetWindowTextW(edit, &mut buffer) } as usize;
                        INPUT.set(Some(String::from_utf16_lossy(&buffer[..copied])));
                    }
                    INPUT_DONE.set(true);
                    let _ = unsafe { DestroyWindow(window) };
                }
                ID_CANCEL => {
                    INPUT_DONE.set(true);
                    let _ = unsafe { DestroyWindow(window) };
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_CLOSE => {
            INPUT_DONE.set(true);
            let _ = unsafe { DestroyWindow(window) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(window, message, wparam, lparam) },
    }
}
