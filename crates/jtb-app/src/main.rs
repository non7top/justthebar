#![windows_subsystem = "windows"]

mod bar;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use jtb_core::config::Config;
use jtb_core::icon::Icon;
use jtb_core::scene::ButtonView;
use jtb_core::tasks::Tasks;
use jtb_core::Edge;
use jtb_win::{explorer, monitors, tasks};
use windows::core::{w, Result};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, KillTimer, PostQuitMessage,
    RegisterClassW, SetTimer, TranslateMessage, MSG, WM_DISPLAYCHANGE, WM_TIMER, WNDCLASSW,
    WS_EX_TOOLWINDOW, WS_POPUP,
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

thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    static CONTROL: Cell<Option<HWND>> = const { Cell::new(None) };
    static EXPLORER_HIDDEN: Cell<bool> = const { Cell::new(false) };
    static TASKS: RefCell<Tasks> = RefCell::new(Tasks::default());
    static ICONS: RefCell<HashMap<isize, Option<Arc<Icon>>>> = RefCell::new(HashMap::new());
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
    for bar_config in config.bars.iter().filter(|b| b.enabled) {
        match monitors::resolve(&monitors, bar_config.monitor) {
            Some((monitor_index, monitor_rect)) => planned.push(bar::Planned {
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
    bar::build(planned, hide && !explorer::autohide_active());
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

/// The tasks on the monitor with this rect, in order, with what their buttons show.
pub fn bar_tasks(monitor: Option<RECT>) -> Vec<(isize, ButtonView)> {
    TASKS.with_borrow(|tasks| {
        tasks
            .items()
            .iter()
            .filter(|task| {
                monitor.is_none_or(|rect| {
                    monitors::rect_of_window(tasks::hwnd_of(task.id)).is_none_or(|r| r == rect)
                })
            })
            .map(|task| {
                let view = ButtonView {
                    title: task.title.clone(),
                    active: tasks.active() == Some(task.id),
                    icon: ICONS.with_borrow(|icons| icons.get(&task.id).cloned().flatten()),
                };
                (task.id, view)
            })
            .collect()
    })
}

/// A click on the `index`th task button of the bar on `monitor`.
pub fn click_task(monitor: Option<RECT>, index: usize) {
    if let Some((id, view)) = bar_tasks(monitor).into_iter().nth(index) {
        tasks::activate_or_minimize(id, view.active);
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

fn on_shell_hook(code: usize, id: isize) {
    TASKS.with_borrow_mut(|list| match code {
        tasks::HSHELL_WINDOWCREATED | tasks::HSHELL_REDRAW => {
            if tasks::is_taskbar_window(id) {
                list.upsert(id, tasks::title(id));
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
    if args.restore_explorer {
        let state = explorer::restore();
        log(&format!("explorer restored, state={state}"));
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
                TIMER_ZORDER => bar::keep_above_explorer(),
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
