#![windows_subsystem = "windows"]

mod bar;

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use jtb_core::config::PinConfig;
use jtb_core::config::{Action, Button, Config, StartConfig, Target};
use jtb_core::grouping::{build_rows, Row, Rules};
use jtb_core::icon::Icon;
use jtb_core::scene::{ButtonView, Drag, StartView, TrayView};
use jtb_core::startart;
use jtb_core::state::State as UiState;
use jtb_core::tasks::{Task, Tasks};
use jtb_core::tray::{Mouse, Side, TrayIcon, TrayIcons, TrayKey};
use jtb_core::traypacket;
use jtb_core::Edge;
use jtb_win::{explorer, monitors, shell, startmenu, tasks, tray};
use windows::core::{w, Result};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetCursorPos, GetMessageW, KillTimer,
    PostQuitMessage, RegisterClassW, SetTimer, TranslateMessage, MSG, WM_DISPLAYCHANGE, WM_TIMER,
    WNDCLASSW, WS_EX_TOOLWINDOW, WS_POPUP,
};

const TIMER_HIDE_EXPLORER: usize = 1;
const TIMER_QUIT: usize = 2;
const TIMER_CLOCK: usize = 3;
const TIMER_CONFIG: usize = 4;
const TIMER_ZORDER: usize = 7;

struct State {
    config: Config,
    /// Config file to watch; `None` when the layout was fixed on the command line.
    config_path: Option<PathBuf>,
    config_mtime: Option<SystemTime>,
    /// `--hide-explorer` forces hiding regardless of the config.
    force_hide_explorer: bool,
}

/// Normal, hot and pressed frames of the Start button.
type StartFrames = [Arc<Icon>; 3];

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    static CONTROL: Cell<Option<HWND>> = const { Cell::new(None) };
    static EXPLORER_HIDDEN: Cell<bool> = const { Cell::new(false) };
    static TASKS: RefCell<Tasks> = RefCell::new(Tasks::default());
    static ICONS: RefCell<HashMap<isize, Option<Arc<Icon>>>> = RefCell::new(HashMap::new());
    static EXE_ICONS: RefCell<HashMap<String, Option<Arc<Icon>>>> = RefCell::new(HashMap::new());
    static UI_STATE: RefCell<UiState> = RefCell::new(UiState::default());
    static TRAY_ICONS: RefCell<TrayIcons> = RefCell::new(TrayIcons::default());
    static START_ART: RefCell<Option<(String, Option<StartFrames>)>> = const { RefCell::new(None) };
    static LAST_PRESS: Cell<[Option<std::time::Instant>; 3]> = const { Cell::new([None; 3]) };
    /// Where the Start button was clicked, in screen coordinates, until the menu is toggled.
    static START_RECT: Cell<Option<RECT>> = const { Cell::new(None) };
    /// The edge (`ABE_*`) and screen rectangle of the bar whose Start button was clicked last.
    static START_BAR: Cell<Option<(u32, RECT)>> = const { Cell::new(None) };
    static NEXT_ICON_ID: Cell<u64> = const { Cell::new(1) };
    static SHELLHOOK_MESSAGE: Cell<u32> = const { Cell::new(0) };
    static LAST_SNAPSHOT: RefCell<String> = const { RefCell::new(String::new()) };
}

fn exe_sibling(name: &str) -> Option<PathBuf> {
    std::env::current_exe().ok().map(|p| p.with_file_name(name))
}

/// Appends a line, prefixed with milliseconds since start, to `jtb.log` next to the exe;
/// failures are ignored.
fn log(line: &str) {
    use std::io::Write;
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    let ms = START
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_millis();
    let Some(path) = exe_sibling("jtb.log") else {
        return;
    };
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{ms:>7} {line}");
    }
}

struct Args {
    edge: Option<Edge>,
    hide_explorer: bool,
    restore_explorer: bool,
    quit_after_secs: Option<u32>,
}

impl Args {
    fn parse() -> Args {
        let mut args = Args {
            edge: None,
            hide_explorer: false,
            restore_explorer: false,
            quit_after_secs: None,
        };
        let mut iter = std::env::args().skip(1);
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--hide-explorer" => args.hide_explorer = true,
                "--restore-explorer" => args.restore_explorer = true,
                "--edge" => args.edge = iter.next().and_then(|v| Edge::parse(&v)),
                "--quit-after" => {
                    args.quit_after_secs = iter.next().and_then(|v| v.parse().ok());
                }
                _ => {}
            }
        }
        args
    }
}

/// Puts Explorer's taskbar back when the app exits, including on panic unwinding.
struct ExplorerGuard;

impl Drop for ExplorerGuard {
    fn drop(&mut self) {
        if EXPLORER_HIDDEN.get() {
            let state = explorer::restore();
            log(&format!("explorer restored, state={state}"));
        }
        if tray::give_back(false) {
            log("tray: role given back on exit");
        }
    }
}

fn mtime(path: &PathBuf) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Loads `jtb.json`; a missing file means defaults, a broken one is an error.
fn load_config(path: &PathBuf) -> std::result::Result<Config, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Config::parse(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(e.to_string()),
    }
}

/// Applies `config`: explorer visibility first, then every bar is rebuilt.
fn apply(config: &Config, force_hide_explorer: bool) {
    let hide = config.hide_explorer_taskbar || force_hide_explorer;
    let control = CONTROL.get();
    if hide != EXPLORER_HIDDEN.get() {
        EXPLORER_HIDDEN.set(hide);
        if hide {
            explorer::hide();
            log(&format!(
                "explorer hidden, startup state={:?}",
                explorer::startup_state()
            ));
        } else {
            explorer::restore();
        }
    }
    if let Some(control) = control {
        unsafe {
            if hide {
                SetTimer(Some(control), TIMER_HIDE_EXPLORER, 100, None);
            } else {
                let _ = KillTimer(Some(control), TIMER_HIDE_EXPLORER);
            }
        }
    }

    bar::destroy_all();
    let monitors = monitors::list();
    let mut planned = Vec::new();
    for (index, bar_config) in config.bars.iter().enumerate().filter(|(_, b)| b.enabled) {
        match monitors::resolve(&monitors, bar_config.monitor) {
            Some((monitor_index, monitor_rect)) => planned.push(bar::Planned {
                index,
                config: bar_config.clone(),
                monitor_index,
                monitor_rect,
            }),
            None => log(&format!(
                "skipped bar: monitor {:?} not found ({} monitors)",
                bar_config.monitor,
                monitors.len()
            )),
        }
    }
    bar::build(planned, hide && !explorer::autohide_active(), config);
    sync_tray(config.tray.enabled);
}

/// Takes or gives back the tray role to match the config.
fn sync_tray(enabled: bool) {
    if enabled == tray::is_active() {
        return;
    }
    if enabled {
        match tray::create(on_tray_packet, on_icon_query, on_taskbar_pos) {
            Ok(()) => log("tray: role taken, Explorer's Shell_TrayWnd sent to the bottom"),
            Err(e) => log(&format!("tray: could not take the role: {e}")),
        }
    } else {
        tray::give_back(false);
        TRAY_ICONS.with_borrow_mut(|icons| *icons = TrayIcons::default());
        log("tray: role given back");
    }
}

/// A notify icon packet from the tray window. Returns whether it was taken.
fn on_tray_packet(data: &[u8]) -> bool {
    let Some(packet) = traypacket::parse_notify(data) else {
        log(&format!("tray: packet of {} bytes ignored", data.len()));
        return false;
    };
    let owner = tasks::process_path(packet.hwnd)
        .and_then(|path| path.rsplit('\\').next().map(str::to_string))
        .unwrap_or_default();
    let valid = packet.is_valid();
    if packet.is_shell_status_icon() {
        return false;
    }
    // An app that deleted its icon may keep modifying it; Explorer has no such icon, so refuse.
    if packet.is_modify() && TRAY_ICONS.with_borrow(|icons| icons.was_deleted(packet.key())) {
        log(&format!(
            "tray: modify of a deleted icon refused owner={owner} id={}",
            packet.id
        ));
        return false;
    }
    let icon = if valid && packet.wants_icon() {
        let id = next_icon_id();
        tasks::icon_from_handle(packet.icon_handle, id).map(Arc::new)
    } else {
        None
    };
    let drawn = icon.is_some();
    let changes = packet.changes(icon);
    let (total, redraw) = TRAY_ICONS.with_borrow_mut(|icons| {
        let mut redraw = false;
        for change in changes {
            redraw |= icons.apply(change);
        }
        (icons.len(), redraw)
    });
    log(&format!(
        "tray: message={} owner={owner} id={} flags={:#x} tip={:?} valid={valid} icon={drawn} icons={total}",
        packet.message, packet.id, packet.flags, packet.tip
    ));
    if redraw {
        bar::invalidate_all();
    }
    valid
}

/// Answers where an icon is on screen; none when it is not one of the bar's, so Explorer answers.
fn on_icon_query(data: &[u8]) -> Option<isize> {
    let query = traypacket::parse_icon_query(data)?;
    let key = TrayKey {
        owner: query.hwnd,
        id: query.id,
    };
    let at = TRAY_ICONS.with_borrow(|icons| {
        icons
            .visible()
            .filter(|i| i.icon.is_some())
            .position(|i| i.key == key)
    })?;
    let rect = bar::tray_icon_screen_rect(current_config().tray.bar, at)?;
    let (x, y) = match query.message {
        1 => (rect.left, rect.top),
        2 => (rect.right, rect.bottom),
        _ => return None,
    };
    log(&format!(
        "tray: icon rect asked message={} hwnd={:#x} id={} -> ({x}, {y})",
        query.message, query.hwnd, query.id
    ));
    Some(traypacket::pack_point(x, y))
}

fn next_icon_id() -> u64 {
    let id = NEXT_ICON_ID.get();
    NEXT_ICON_ID.set(id + 1);
    id
}

/// The icons the bar with this config index shows in its notification area; none for other bars.
pub fn tray_views(bar_index: usize, hovered: Option<usize>) -> Vec<TrayView> {
    let config = current_config();
    if !config.tray.enabled || config.tray.bar != bar_index || !tray::is_active() {
        return Vec::new();
    }
    TRAY_ICONS.with_borrow(|icons| {
        icons
            .visible()
            .filter_map(|i| i.icon.clone())
            .enumerate()
            .map(|(at, icon)| TrayView {
                icon,
                hovered: hovered == Some(at),
            })
            .collect()
    })
}

/// The icon drawn in cell `at` of the notification area (the cells follow `tray_views`).
fn tray_icon_at(at: usize) -> Option<TrayIcon> {
    TRAY_ICONS.with_borrow(|icons| {
        icons
            .visible()
            .filter(|i| i.icon.is_some())
            .nth(at)
            .cloned()
    })
}

/// The tooltip of the icon in cell `at`; none when the app gave it no text.
pub fn tray_tooltip(at: usize) -> Option<String> {
    tray_icon_at(at)
        .map(|i| i.tooltip)
        .filter(|tip| !tip.is_empty())
}

/// The cursor in screen coordinates as the callback messages carry it: x low, y high.
fn cursor_word() -> u32 {
    let mut at = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut at);
    }
    (at.x as u16 as u32) | ((at.y as u16 as u32) << 16)
}

/// Passes a mouse event on an icon to the app that owns it.
pub fn tray_mouse(at: usize, event: Mouse) {
    let Some(icon) = tray_icon_at(at) else { return };
    let owner = icon.key.owner;
    if icon.callback_message == 0 || !tray::owner_alive(owner) {
        return;
    }
    if matches!(event, Mouse::Down { .. }) {
        tray::allow_foreground(owner);
    }
    let mouse = cursor_word();
    for message in icon.mouse_messages(event) {
        let (wparam, lparam) = icon.callback_params(message, mouse);
        if !tray::notify_owner(owner, icon.callback_message, wparam, lparam) {
            log(&format!(
                "tray: owner {owner:#x} did not take message {message:#x}"
            ));
        }
    }
}

/// A button press on an icon; two presses within the double click time make a double click.
pub fn tray_press(at: usize, side: Side) {
    let slot = side as usize;
    let now = std::time::Instant::now();
    let limit = std::time::Duration::from_millis(u64::from(unsafe { GetDoubleClickTime() }));
    let mut last = LAST_PRESS.get();
    let double = last[slot].is_some_and(|t| now.duration_since(t) <= limit);
    last[slot] = if double { None } else { Some(now) };
    LAST_PRESS.set(last);
    tray_mouse(at, Mouse::Down { side, double });
}

/// The pointer moved from icon `before` to icon `now` (either may be none) or within one.
pub fn tray_hover_events(before: Option<usize>, now: Option<usize>) {
    if before == now {
        if let Some(at) = now {
            tray_mouse(at, Mouse::Move);
        }
        return;
    }
    if let Some(at) = before {
        tray_mouse(at, Mouse::Leave);
    }
    if let Some(at) = now {
        tray_mouse(at, Mouse::Enter);
    }
}

fn current_settings() -> Option<(Config, bool)> {
    STATE.with_borrow(|s| {
        s.as_ref()
            .map(|s| (s.config.clone(), s.force_hide_explorer))
    })
}

/// Reloads the config file when it changed; a bad file keeps the last good config.
fn poll_config() {
    let Some((path, seen)) = STATE.with_borrow(|s| {
        s.as_ref()
            .and_then(|s| s.config_path.clone().map(|p| (p, s.config_mtime)))
    }) else {
        return;
    };
    let now = mtime(&path);
    if now == seen {
        return;
    }
    let loaded = load_config(&path);
    let changed = STATE.with_borrow_mut(|s| {
        let state = s.as_mut()?;
        state.config_mtime = now;
        match loaded {
            Ok(config) if config != state.config => {
                state.config = config;
                Some(())
            }
            Ok(_) => None,
            Err(e) => {
                log(&format!("config error, keeping the previous one: {e}"));
                None
            }
        }
    });
    if changed.is_some() {
        log("config reloaded");
        if let Some((config, force)) = current_settings() {
            apply(&config, force);
        }
    }
}

/// What a bar button stands for.
enum Click {
    Window {
        id: isize,
        key: String,
    },
    Header {
        key: String,
        windows: Vec<isize>,
        pin: Option<usize>,
    },
    Start,
}

impl Click {
    /// Key of the group the button belongs to.
    fn key(&self) -> &str {
        match self {
            Click::Window { key, .. } | Click::Header { key, .. } => key,
            Click::Start => "start",
        }
    }
}

fn current_config() -> Config {
    STATE.with_borrow(|s| s.as_ref().map(|s| s.config.clone()).unwrap_or_default())
}

fn state_file() -> Option<PathBuf> {
    exe_sibling("jtb.state.json")
}

fn load_state() {
    let Some(path) = state_file() else { return };
    match std::fs::read_to_string(&path) {
        Ok(text) => match UiState::parse(&text) {
            Ok(state) => UI_STATE.set(state),
            Err(e) => log(&format!("state file ignored: {e}")),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => log(&format!("state file not read: {e}")),
    }
}

fn save_state() {
    let Some(path) = state_file() else { return };
    let json = UI_STATE.with_borrow(|s| s.to_json());
    if let Err(e) = std::fs::write(&path, json) {
        log(&format!("state file not written: {e}"));
    }
}

fn effective_pins(config: &Config) -> Vec<PinConfig> {
    UI_STATE.with_borrow(|s| s.effective_pins(&config.pins))
}

/// The icon of an executable, loaded once.
fn exe_icon(path: &str) -> Option<Arc<Icon>> {
    EXE_ICONS.with_borrow_mut(|cache| {
        cache
            .entry(path.to_string())
            .or_insert_with(|| {
                let id = NEXT_ICON_ID.get();
                let icon = tasks::exe_icon(path, id).map(Arc::new);
                if icon.is_some() {
                    NEXT_ICON_ID.set(id + 1);
                }
                icon
            })
            .clone()
    })
}

/// The Start button art, read once per configured file name; `None` gives the drawn button.
fn start_frames(image: &str) -> Option<StartFrames> {
    if image.trim().is_empty() {
        return None;
    }
    START_ART.with_borrow_mut(|cache| {
        if cache.as_ref().is_none_or(|(loaded, _)| loaded != image) {
            *cache = Some((image.to_string(), load_start_art(image)));
        }
        cache.as_ref().and_then(|(_, frames)| frames.clone())
    })
}

fn load_start_art(image: &str) -> Option<StartFrames> {
    let path = if std::path::Path::new(image).is_absolute() {
        PathBuf::from(image)
    } else {
        exe_sibling(image)?
    };
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) => {
            log(&format!("start art not read, drawing the button: {e}"));
            return None;
        }
    };
    let art = match startart::load(&bytes) {
        Ok(art) => art,
        Err(e) => {
            log(&format!("start art ignored, drawing the button: {e}"));
            return None;
        }
    };
    log(&format!(
        "start art loaded, frames {}x{}",
        art.normal.width, art.normal.height
    ));
    let icon = |frame: startart::Frame| {
        let id = NEXT_ICON_ID.get();
        NEXT_ICON_ID.set(id + 1);
        Arc::new(Icon {
            id,
            width: frame.width,
            height: frame.height,
            bgra: frame.bgra,
        })
    };
    Some([icon(art.normal), icon(art.hot), icon(art.pressed)])
}

fn start_row(config: &StartConfig) -> (ButtonView, Click) {
    let view = ButtonView {
        title: config.label.clone(),
        active: false,
        hovered: false,
        minimized: false,
        flashing: false,
        title_changed: false,
        child: false,
        header: false,
        drag: Drag::None,
        start: Some(StartView {
            frames: start_frames(&config.image),
            blend: 0.0,
            pressed: false,
            size: config.size as f32,
        }),
        icon: None,
    };
    (view, Click::Start)
}

/// Remembers where the Start button the user just clicked is, for `run_start`.
pub fn note_start_rect(rect: RECT) {
    START_RECT.set(Some(rect));
}

/// Remembers which bar the Start button was clicked on: Open-Shell asks where the taskbar is and
/// opens the menu on that side.
pub fn note_start_bar(edge: Edge, rect: RECT) {
    let abe = match edge {
        Edge::Left => 0,
        Edge::Top => 1,
        Edge::Right => 2,
        Edge::Bottom => 3,
    };
    START_BAR.set(Some((abe, rect)));
}

fn on_taskbar_pos() -> Option<(u32, RECT)> {
    let pos = START_BAR.get()?;
    log(&format!("start: taskbar position asked -> {pos:?}"));
    Some(pos)
}

fn run_start(config: &StartConfig) {
    log(&format!(
        "start button: {} {:?}",
        config.command, config.args
    ));
    if let Some((_, bar)) = START_BAR.get() {
        let fitted = startmenu::fit_explorer_tray(bar);
        log(&format!(
            "start: Explorer taskbar window asked to cover {bar:?}, was/now {fitted:?}"
        ));
    }
    if let Some(rect) = START_RECT.take() {
        let moved = startmenu::move_button(rect);
        log(&format!(
            "start: Open-Shell button asked to move to {rect:?}, now at {moved:?}"
        ));
    }
    launch_app(&config.command, &config.args);
}

/// The buttons of the bar on `monitor` (`None` shows every window), with what a click on each does.
fn build_bar(
    monitor: Option<RECT>,
    config: &Config,
    pins: &[PinConfig],
) -> Vec<(ButtonView, Click)> {
    let rules = Rules::new(&config.groups);
    let (folded, order) = UI_STATE.with_borrow(|s| {
        (
            s.folded.iter().cloned().collect::<HashSet<String>>(),
            s.order.clone(),
        )
    });
    let mut bar = TASKS.with_borrow(|tasks| {
        let visible: Vec<Task> = tasks
            .items()
            .iter()
            .filter(|task| {
                monitor.is_none_or(|rect| {
                    monitors::rect_of_window(tasks::hwnd_of(task.id)).is_none_or(|r| r == rect)
                })
            })
            .cloned()
            .collect();
        let icon_of = |id: isize| ICONS.with_borrow(|icons| icons.get(&id).cloned().flatten());
        let rows = build_rows(
            &visible,
            pins,
            &rules,
            &folded,
            &order,
            config.grouping.mode,
        );
        rows.into_iter()
            .map(|row| match row {
                Row::Window { id, child } => {
                    let task = tasks.get(id).cloned().unwrap_or(Task {
                        id,
                        title: String::new(),
                        exe: String::new(),
                    });
                    let view = ButtonView {
                        title: task.title.clone(),
                        active: tasks.active() == Some(id),
                        hovered: false,
                        minimized: tasks::is_minimized(id),
                        flashing: tasks.is_flashing(id),
                        title_changed: tasks.has_new_title(id),
                        child,
                        header: false,
                        drag: Drag::None,
                        start: None,
                        icon: icon_of(id),
                    };
                    let key = rules.group_of(&task).key;
                    (view, Click::Window { id, key })
                }
                Row::Header {
                    key,
                    name,
                    pin,
                    windows,
                    folded,
                } => {
                    let active = windows.iter().any(|id| tasks.active() == Some(*id));
                    let title = if windows.is_empty() {
                        name
                    } else {
                        let marker = if folded { "\u{25B8}" } else { "\u{25BE}" };
                        format!("{marker} {name} ({})", windows.len())
                    };
                    let view = ButtonView {
                        title,
                        active,
                        hovered: false,
                        minimized: windows.iter().all(|id| tasks::is_minimized(*id)),
                        flashing: windows.iter().any(|id| tasks.is_flashing(*id)),
                        title_changed: !active && windows.iter().any(|id| tasks.has_new_title(*id)),
                        child: false,
                        header: true,
                        drag: Drag::None,
                        start: None,
                        icon: pin
                            .and_then(|i| pins.get(i))
                            .and_then(|p| exe_icon(&p.path))
                            .or_else(|| windows.first().and_then(|id| icon_of(*id))),
                    };
                    (view, Click::Header { key, windows, pin })
                }
            })
            .collect::<Vec<_>>()
    });
    if config.start.enabled {
        bar.insert(0, start_row(&config.start));
    }
    bar
}

/// Whether the bars show a Start button.
pub fn start_enabled() -> bool {
    current_config().start.enabled
}

/// What the bar on `monitor` shows, one entry per button.
pub fn bar_views(monitor: Option<RECT>) -> Vec<ButtonView> {
    let config = current_config();
    let pins = effective_pins(&config);
    build_bar(monitor, &config, &pins)
        .into_iter()
        .map(|(view, _)| view)
        .collect()
}

fn toggle_fold(key: &str) {
    UI_STATE.with_borrow_mut(|s| s.toggle_fold(key));
    save_state();
    bar::invalidate_all();
}

fn launch_app(path: &str, args: &[String]) {
    if path.is_empty() || !tasks::launch(path, args) {
        log(&format!("launch failed: {path}"));
    }
}

fn exe_of(id: isize) -> String {
    TASKS.with_borrow(|t| t.get(id).map(|t| t.exe.clone()).unwrap_or_default())
}

/// Starts the app a button stands for: the pin's program, else the window's executable.
fn launch_click(click: &Click, pins: &[PinConfig]) {
    match click {
        Click::Window { id, .. } => launch_app(&exe_of(*id), &[]),
        Click::Start => {}
        Click::Header { windows, pin, .. } => match pin.and_then(|i| pins.get(i)) {
            Some(pin) => launch_app(&pin.path, &pin.args),
            None => launch_app(
                &windows.first().map(|id| exe_of(*id)).unwrap_or_default(),
                &[],
            ),
        },
    }
}

/// A click of `button` on the `index`th button of the bar on `monitor`.
pub fn click_row(monitor: Option<RECT>, index: usize, button: Button) {
    let config = current_config();
    let pins = effective_pins(&config);
    let Some((view, click)) = build_bar(monitor, &config, &pins).into_iter().nth(index) else {
        return;
    };
    let target = match click {
        Click::Window { .. } => Target::Window,
        Click::Header { .. } => Target::Header,
        Click::Start => {
            if button == Button::Left {
                run_start(&config.start);
            }
            return;
        }
    };
    match config.mouse.action(target, button) {
        Action::None => {}
        Action::Activate => match &click {
            Click::Window { id, .. } => {
                let foreground = tasks::foreground();
                let active = view.active || foreground == Some(*id);
                log(&format!(
                    "click: window {id} active={} foreground={foreground:?}",
                    view.active
                ));
                tasks::activate_or_minimize(*id, active);
            }
            Click::Header { windows, .. } => match windows.first() {
                Some(id) => tasks::activate_or_minimize(*id, false),
                None => launch_click(&click, &pins),
            },
            Click::Start => {}
        },
        Action::Launch => launch_click(&click, &pins),
        Action::ToggleFold => match &click {
            Click::Header { windows, .. } if windows.is_empty() => launch_click(&click, &pins),
            _ => toggle_fold(click.key()),
        },
    }
}

/// Drags the `from`th button of the bar on `monitor` to where the `to`th is.
pub fn reorder(monitor: Option<RECT>, from: usize, to: usize) {
    let config = current_config();
    let pins = effective_pins(&config);
    let bar = build_bar(monitor, &config, &pins);
    let (Some(source), Some(target)) = (bar.get(from), bar.get(to)) else {
        return;
    };
    if matches!(source.1, Click::Start) || matches!(target.1, Click::Start) {
        return;
    }
    let (from_key, to_key) = (source.1.key().to_string(), target.1.key().to_string());
    if source.0.child {
        if let (Click::Window { id: from_id, .. }, Click::Window { id: to_id, .. }) =
            (&source.1, &target.1)
        {
            if from_key == to_key && from_id != to_id {
                log(&format!("window {from_id} moved to window {to_id}"));
                TASKS.with_borrow_mut(|t| t.move_to(*from_id, *to_id));
                bar::invalidate_all();
            }
        }
        return;
    }
    if from_key == to_key {
        return;
    }
    log(&format!("group {from_key} moved to {to_key}"));
    let mut sequence: Vec<String> = Vec::new();
    for (_, click) in &bar {
        if !sequence.iter().any(|k| k == click.key()) {
            sequence.push(click.key().to_string());
        }
    }
    UI_STATE.with_borrow_mut(|s| s.move_key(&sequence, &from_key, &to_key));
    save_state();
    bar::invalidate_all();
}

/// Files dropped on a bar become pins.
pub fn pin_files(paths: Vec<String>) {
    let mut added = false;
    for path in paths {
        let (program, args) = if path.to_lowercase().ends_with(".lnk") {
            match shell::resolve_shortcut(&path) {
                Some(resolved) => resolved,
                None => {
                    log(&format!("shortcut not resolved: {path}"));
                    continue;
                }
            }
        } else {
            (path, Vec::new())
        };
        added |= pin_program(&program, args);
    }
    if added {
        save_state();
        bar::invalidate_all();
    }
}

fn pin_program(path: &str, args: Vec<String>) -> bool {
    let name = std::path::Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    UI_STATE.with_borrow_mut(|s| {
        s.add_pin(PinConfig {
            name,
            path: path.to_string(),
            args,
        })
    })
}

/// The right-click menu of the `index`th button of the bar on `monitor`.
pub fn context_menu(monitor: Option<RECT>, index: usize, hwnd: HWND) {
    let config = current_config();
    let pins = effective_pins(&config);
    let Some((_, click)) = build_bar(monitor, &config, &pins).into_iter().nth(index) else {
        return;
    };
    match &click {
        Click::Window { id, .. } => {
            let exe = exe_of(*id);
            let items = [("Pin this app", !exe.is_empty()), ("Close window", true)];
            match shell::popup_menu(hwnd, &items) {
                Some(0) => {
                    if pin_program(&exe, Vec::new()) {
                        save_state();
                        bar::invalidate_all();
                    }
                }
                Some(1) => tasks::close(*id),
                _ => {}
            }
        }
        Click::Start => {}
        Click::Header { key, pin, .. } => {
            let items = [
                ("Launch new", true),
                ("Fold or unfold", true),
                ("Unpin", pin.is_some()),
            ];
            match shell::popup_menu(hwnd, &items) {
                Some(0) => launch_click(&click, &pins),
                Some(1) => toggle_fold(key),
                Some(2) => {
                    if let Some(pin) = pin.and_then(|i| pins.get(i)) {
                        UI_STATE.with_borrow_mut(|s| s.remove_pin(&pin.path));
                        save_state();
                        bar::invalidate_all();
                    }
                }
                _ => {}
            }
        }
    }
}

/// Re-reads the window list so windows the shell hook missed show up and closed ones go away.
fn sync_tasks() {
    TASKS.with_borrow_mut(|list| list.replace_all(tasks::enumerate()));
    refresh_icons();
}

/// Loads the icon of `id`, keeping the old image (and its cached upload) if the pixels match.
fn load_icon(id: isize) {
    let next = NEXT_ICON_ID.get();
    let fresh = tasks::icon(id, next);
    ICONS.with_borrow_mut(|icons| {
        let kept = icons
            .get(&id)
            .cloned()
            .flatten()
            .filter(|old| fresh.as_ref().is_some_and(|new| old.same_pixels(new)));
        if kept.is_none() && fresh.is_some() {
            NEXT_ICON_ID.set(next + 1);
        }
        icons.insert(id, kept.or_else(|| fresh.map(Arc::new)));
    });
}

/// Loads icons for windows that have none yet and forgets those of closed windows.
fn refresh_icons() {
    let ids: Vec<isize> = TASKS.with_borrow(|list| list.items().iter().map(|t| t.id).collect());
    for id in &ids {
        if !ICONS.with_borrow(|icons| icons.contains_key(id)) {
            load_icon(*id);
        }
    }
    ICONS.with_borrow_mut(|icons| icons.retain(|id, _| ids.contains(id)));
}

fn fill_exe(list: &mut Tasks, id: isize) {
    if list.get(id).is_some_and(|task| task.exe.is_empty()) {
        list.set_exe(id, tasks::process_path(id).unwrap_or_default());
    }
}

fn on_shell_hook(code: usize, id: isize) {
    TASKS.with_borrow_mut(|list| match code {
        tasks::HSHELL_FLASH => {
            if tasks::is_taskbar_window(id) {
                list.upsert(id, tasks::title(id));
                fill_exe(list, id);
                list.set_flashing(id, true);
            }
        }
        tasks::HSHELL_WINDOWCREATED | tasks::HSHELL_REDRAW => {
            list.set_flashing(id, false);
            if tasks::is_taskbar_window(id) {
                list.upsert(id, tasks::title(id));
                fill_exe(list, id);
            } else {
                list.remove(id);
            }
        }
        tasks::HSHELL_WINDOWDESTROYED | tasks::HSHELL_WINDOWREPLACED => list.remove(id),
        tasks::HSHELL_WINDOWACTIVATED | tasks::HSHELL_RUDEAPPACTIVATED => {
            if id == 0 {
                list.set_active(None);
            } else if tasks::is_taskbar_window(id) {
                list.upsert(id, tasks::title(id));
                fill_exe(list, id);
                list.set_active(Some(id));
            }
        }
        _ => {}
    });
    if code == tasks::HSHELL_REDRAW && TASKS.with_borrow(|list| list.contains(id)) {
        load_icon(id);
    }
    refresh_icons();
    bar::invalidate_all();
}

/// Removes all appbars and ends the message loop.
pub fn quit() {
    bar::destroy_all();
    unsafe { PostQuitMessage(0) };
}

fn main() -> Result<()> {
    std::panic::set_hook(Box::new(|info| log(&format!("panic: {info}"))));
    let args = Args::parse();
    log(&format!(
        "start quit_after={:?} restore={}",
        args.quit_after_secs, args.restore_explorer
    ));
    load_state();
    if args.restore_explorer {
        let state = explorer::restore();
        log(&format!("explorer restored, state={state}"));
        tray::give_back(true);
        log("tray: sent TaskbarCreated");
        return Ok(());
    }

    let (config, config_path) = match args.edge {
        Some(edge) => (Config::single(edge, false), None),
        None => {
            let path = exe_sibling("jtb.json");
            let config = match path.as_ref().map(load_config) {
                Some(Ok(config)) => config,
                Some(Err(e)) => {
                    log(&format!("config error, using defaults: {e}"));
                    Config::default()
                }
                None => Config::default(),
            };
            (config, path)
        }
    };
    let config_mtime = config_path.as_ref().and_then(mtime);
    STATE.set(Some(State {
        config: config.clone(),
        config_path,
        config_mtime,
        force_hide_explorer: args.hide_explorer,
    }));

    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        bar::register_class()?;

        let instance = GetModuleHandleW(None)?;
        let control_class = WNDCLASSW {
            hInstance: instance.into(),
            lpszClassName: w!("JustTheBarControl"),
            lpfnWndProc: Some(control_proc),
            ..Default::default()
        };
        RegisterClassW(&control_class);
        // A hidden top-level window: it gets the display-change broadcast and owns the timers.
        let control = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            w!("JustTheBarControl"),
            w!("justthebar control"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance.into()),
            None,
        )?;
        CONTROL.set(Some(control));

        let _guard = ExplorerGuard;
        SHELLHOOK_MESSAGE.set(tasks::register_shell_hook(control));
        sync_tasks();
        TASKS.with_borrow_mut(|list| {
            let active = tasks::foreground().filter(|id| list.contains(*id));
            list.set_active(active);
        });
        apply(&config, args.hide_explorer);
        SetTimer(Some(control), TIMER_CLOCK, 1000, None);
        SetTimer(Some(control), TIMER_CONFIG, 1000, None);
        SetTimer(Some(control), TIMER_ZORDER, 200, None);
        if let Some(secs) = args.quit_after_secs {
            SetTimer(Some(control), TIMER_QUIT, secs * 1000, None);
        }

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    Ok(())
}

extern "system" fn control_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_TIMER => {
            match wparam.0 {
                TIMER_HIDE_EXPLORER => {
                    if explorer::enforce_hidden() {
                        log("explorer taskbar was visible, hid it again");
                    }
                }
                TIMER_CLOCK => {
                    sync_tasks();
                    bar::invalidate_all();
                    let now = explorer::snapshot();
                    LAST_SNAPSHOT.with_borrow_mut(|last| {
                        if *last != now {
                            log(&format!("changed: {now}"));
                            *last = now;
                        }
                    });
                }
                TIMER_ZORDER => {
                    bar::keep_above_explorer();
                    if tray::keep_on_top() {
                        log("tray: raised our Shell_TrayWnd");
                    }
                }
                TIMER_CONFIG => poll_config(),
                TIMER_QUIT => {
                    unsafe {
                        let _ = KillTimer(Some(hwnd), TIMER_QUIT);
                    }
                    log("quit timer fired");
                    quit();
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_DISPLAYCHANGE => {
            log("display change");
            if let Some((config, force)) = current_settings() {
                apply(&config, force);
            }
            LRESULT(0)
        }
        _ if msg != 0 && msg == SHELLHOOK_MESSAGE.get() => {
            on_shell_hook(wparam.0, lparam.0);
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
