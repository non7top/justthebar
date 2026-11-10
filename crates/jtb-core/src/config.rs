use serde::{Deserialize, Serialize};

use crate::activity::NotificationRule;
use crate::scene::Color;
use crate::Edge;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorSel {
    Primary,
    Index(usize),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawMonitor {
    Index(usize),
    Name(String),
}

impl TryFrom<RawMonitor> for MonitorSel {
    type Error = String;

    fn try_from(raw: RawMonitor) -> Result<Self, String> {
        match raw {
            RawMonitor::Index(i) => Ok(MonitorSel::Index(i)),
            RawMonitor::Name(n) if n == "primary" => Ok(MonitorSel::Primary),
            RawMonitor::Name(n) => Err(format!(
                "monitor must be \"primary\" or a 0-based number, got \"{n}\""
            )),
        }
    }
}

impl<'de> Deserialize<'de> for MonitorSel {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        MonitorSel::try_from(RawMonitor::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BarConfig {
    pub monitor: MonitorSel,
    pub edge: Edge,
    /// Thickness in pixels at 96 dpi.
    pub size: u32,
    pub enabled: bool,
}

impl Default for BarConfig {
    fn default() -> Self {
        BarConfig {
            monitor: MonitorSel::Primary,
            edge: Edge::Bottom,
            size: 40,
            enabled: true,
        }
    }
}

/// A `#rrggbb` color in the config file.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HexColor(pub Color);

impl<'de> Deserialize<'de> for HexColor {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let text = String::deserialize(d)?;
        Color::from_hex(&text)
            .map(HexColor)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Appearance {
    pub bar_color: HexColor,
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance {
            bar_color: HexColor(Color::rgb(23.0 / 255.0, 26.0 / 255.0, 31.0 / 255.0)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TasksConfig {
    /// Each bar lists only the windows on its own monitor.
    pub per_monitor: bool,
}

impl Default for TasksConfig {
    fn default() -> Self {
        TasksConfig { per_monitor: true }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GroupingMode {
    /// One plain button per window.
    Off,
    /// Windows of one app share a header button that folds.
    #[default]
    Accordion,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GroupingConfig {
    pub mode: GroupingMode,
}

/// An app pinned to the bar: its header launches it and groups its windows.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct PinConfig {
    pub name: String,
    pub path: String,
    pub args: Vec<String>,
}

/// A user-defined group of executables whose windows share one header.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GroupConfig {
    pub name: String,
    pub exes: Vec<String>,
}

/// Where a widget is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WidgetPlace {
    /// On the bar itself, between the task buttons and the notification area.
    #[default]
    Bar,
    /// As a small icon cell in the notification area.
    Tray,
}

/// A built-in widget to show: `"battery"`, or `{ "name": "battery", "place": "tray" }`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(from = "WidgetEntry")]
pub struct WidgetConfig {
    pub name: String,
    pub place: WidgetPlace,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum WidgetEntry {
    Name(String),
    Full {
        name: String,
        #[serde(default)]
        place: WidgetPlace,
    },
}

impl From<WidgetEntry> for WidgetConfig {
    fn from(entry: WidgetEntry) -> Self {
        match entry {
            WidgetEntry::Name(name) => WidgetConfig {
                name,
                place: WidgetPlace::default(),
            },
            WidgetEntry::Full { name, place } => WidgetConfig { name, place },
        }
    }
}

/// What opens when the Start button is clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StartMenu {
    /// Open-Shell when `start.command` exists, else the built-in menu.
    #[default]
    Auto,
    Stock,
    OpenShell,
}

/// The Start button, drawn by the bar; a click runs `command`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StartConfig {
    pub enabled: bool,
    pub menu: StartMenu,
    pub command: String,
    pub args: Vec<String>,
    pub label: String,
    /// Button art in Open-Shell's strip format, relative to the exe folder; a missing file gives
    /// the drawn button.
    pub image: String,
    /// Longest side of the art button in pixels at 100% scale; a thinner bar makes it smaller.
    pub size: u32,
    /// Opens Open-Shell's menu from a copy of its library loaded into this app, which lets the
    /// bar say exactly where the menu goes; falls back to `command` when that fails.
    pub in_process: bool,
    /// A lone press of the Windows key opens the menu from the Start button of the bar on the
    /// monitor under the cursor.
    pub win_key: bool,
}

impl Default for StartConfig {
    fn default() -> Self {
        StartConfig {
            enabled: false,
            menu: StartMenu::Auto,
            command: r"C:\Program Files\Open-Shell\StartMenu.exe".to_string(),
            args: vec!["-toggle".to_string()],
            label: "Start".to_string(),
            image: "start-button.png".to_string(),
            size: 54,
            in_process: true,
            win_key: true,
        }
    }
}

/// The notification area: when enabled we own the `Shell_TrayWnd` role and show the icons on
/// bar number `bar`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TrayConfig {
    pub enabled: bool,
    pub bar: usize,
}

/// What a click does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Bring the window forward, minimize it if active; on a header, the group's first window.
    Activate,
    /// Start the app (pin or executable of the clicked button).
    Launch,
    /// Fold or unfold the clicked button's group.
    ToggleFold,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Left,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Window,
    Header,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MouseConfig {
    pub header_left: Action,
    pub header_middle: Action,
    pub window_left: Action,
    pub window_middle: Action,
}

impl Default for MouseConfig {
    fn default() -> Self {
        MouseConfig {
            header_left: Action::ToggleFold,
            header_middle: Action::Launch,
            window_left: Action::Activate,
            window_middle: Action::None,
        }
    }
}

impl MouseConfig {
    pub fn action(&self, target: Target, button: Button) -> Action {
        match (target, button) {
            (Target::Header, Button::Left) => self.header_left,
            (Target::Header, Button::Middle) => self.header_middle,
            (Target::Window, Button::Left) => self.window_left,
            (Target::Window, Button::Middle) => self.window_middle,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ClockConfig {
    pub format: String,
    pub months: u32,
}

impl Default for ClockConfig {
    fn default() -> Self {
        ClockConfig {
            format: crate::clock::DEFAULT_FORMAT.to_string(),
            months: 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ActivityConfig {
    pub enabled: bool,
    pub record_title_changes: bool,
    pub rules: Vec<NotificationRule>,
}

impl Default for ActivityConfig {
    fn default() -> Self {
        ActivityConfig {
            enabled: true,
            record_title_changes: false,
            rules: Vec::new(),
        }
    }
}

/// The only config file version this build understands.
pub const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    pub hide_explorer_taskbar: bool,
    pub bars: Vec<BarConfig>,
    pub appearance: Appearance,
    pub tasks: TasksConfig,
    pub grouping: GroupingConfig,
    pub mouse: MouseConfig,
    pub start: StartConfig,
    pub tray: TrayConfig,
    pub clock: ClockConfig,
    /// Built-in widgets (`battery`, `language`, ...) shown in the notification area of the tray
    /// bar, left to right, next to the clock.
    pub widgets: Vec<WidgetConfig>,
    pub pins: Vec<PinConfig>,
    pub groups: Vec<GroupConfig>,
    pub activity: ActivityConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            version: CONFIG_VERSION,
            hide_explorer_taskbar: false,
            bars: vec![BarConfig::default()],
            appearance: Appearance::default(),
            tasks: TasksConfig::default(),
            grouping: GroupingConfig::default(),
            mouse: MouseConfig::default(),
            start: StartConfig::default(),
            tray: TrayConfig::default(),
            clock: ClockConfig::default(),
            widgets: Vec::new(),
            pins: Vec::new(),
            groups: Vec::new(),
            activity: ActivityConfig::default(),
        }
    }
}

impl Config {
    pub fn parse(text: &str) -> Result<Config, String> {
        let config: Config = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if config.version != CONFIG_VERSION {
            return Err(format!(
                "config version {} is not supported, this build reads version {CONFIG_VERSION}",
                config.version
            ));
        }
        for group in &config.groups {
            if group.name.trim().is_empty() || group.exes.is_empty() {
                return Err("every group needs a name and at least one exe".to_string());
            }
        }
        for pin in &config.pins {
            if pin.path.trim().is_empty() {
                return Err("every pin needs a path".to_string());
            }
        }
        if config.start.enabled && config.start.command.trim().is_empty() {
            return Err("the start button needs a command".to_string());
        }
        if !(8..=512).contains(&config.start.size) {
            return Err(format!(
                "start.size must be 8..512, got {}",
                config.start.size
            ));
        }
        if config.tray.enabled && config.tray.bar >= config.bars.len() {
            return Err(format!(
                "tray.bar is {} but there are only {} bars",
                config.tray.bar,
                config.bars.len()
            ));
        }
        for bar in &config.bars {
            if bar.size == 0 || bar.size > 1000 {
                return Err(format!("bar size must be 1..1000, got {}", bar.size));
            }
        }
        Ok(config)
    }

    pub fn single(edge: Edge, hide_explorer_taskbar: bool) -> Config {
        Config {
            hide_explorer_taskbar,
            bars: vec![BarConfig {
                edge,
                ..BarConfig::default()
            }],
            ..Config::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_object_is_the_default() {
        assert_eq!(Config::parse("{}").unwrap(), Config::default());
    }

    #[test]
    fn parses_two_adjacent_bars() {
        let config = Config::parse(
            r#"{
                "hide_explorer_taskbar": true,
                "bars": [
                    {"edge": "bottom", "size": 48},
                    {"monitor": 1, "edge": "left", "enabled": false}
                ]
            }"#,
        )
        .unwrap();
        assert!(config.hide_explorer_taskbar);
        assert_eq!(config.bars[0].size, 48);
        assert_eq!(config.bars[0].monitor, MonitorSel::Primary);
        assert_eq!(config.bars[1].monitor, MonitorSel::Index(1));
        assert_eq!(config.bars[1].edge, Edge::Left);
        assert!(!config.bars[1].enabled);
    }

    #[test]
    fn example_file_parses_and_matches_the_defaults() {
        let example = include_str!("../../../config/jtb.example.json");
        let config = Config::parse(example).unwrap();
        assert_eq!(config.version, CONFIG_VERSION);
        assert_eq!(config.appearance, Appearance::default());
        assert_eq!(config.start, StartConfig::default());
        assert_eq!(config.tray, TrayConfig::default());
        assert_eq!(config.tasks, TasksConfig::default());
        assert_eq!(config.grouping, GroupingConfig::default());
        assert_eq!(config.mouse, MouseConfig::default());
    }

    #[test]
    fn appearance_and_reserved_sections_parse() {
        let config = Config::parse(
            r##"{
                "appearance": {"bar_color": "#102030"},
                "tasks": {"per_monitor": false},
                "grouping": {"mode": "off"},
                "pins": [{"name": "Putty", "path": "C:\\putty.exe", "args": ["-load", "x"]}],
                "groups": [{"name": "Putty", "exes": ["putty.exe", "PuTTYPortable.exe"]}]
            }"##,
        )
        .unwrap();
        assert!(!config.tasks.per_monitor);
        assert_eq!(
            config.appearance.bar_color.0,
            Color::from_hex("#102030").unwrap()
        );
        assert_eq!(config.pins[0].args.len(), 2);
        assert_eq!(config.groups[0].exes.len(), 2);
    }

    #[test]
    fn mouse_actions_default_and_override() {
        let defaults = MouseConfig::default();
        assert_eq!(
            defaults.action(Target::Header, Button::Left),
            Action::ToggleFold
        );
        assert_eq!(
            defaults.action(Target::Header, Button::Middle),
            Action::Launch
        );
        let config =
            Config::parse(r#"{"mouse": {"header_left": "activate", "window_middle": "launch"}}"#)
                .unwrap();
        assert_eq!(config.mouse.header_left, Action::Activate);
        assert_eq!(config.mouse.window_middle, Action::Launch);
        assert_eq!(config.mouse.window_left, Action::Activate);
        assert!(Config::parse(r#"{"mouse": {"header_left": "explode"}}"#).is_err());
    }

    #[test]
    fn start_button_defaults_and_validation() {
        let defaults = Config::parse("{}").unwrap().start;
        assert!(!defaults.enabled);
        assert_eq!(defaults.args, vec!["-toggle".to_string()]);
        assert_eq!(defaults.menu, StartMenu::Auto);
        let shell = Config::parse(r#"{"start": {"menu": "openshell"}}"#).unwrap();
        assert_eq!(shell.start.menu, StartMenu::OpenShell);
        let stock = Config::parse(r#"{"start": {"menu": "stock"}}"#).unwrap();
        assert_eq!(stock.start.menu, StartMenu::Stock);
        let config =
            Config::parse(r#"{"start": {"enabled": true, "label": "Go", "image": ""}}"#).unwrap();
        assert!(config.start.enabled);
        assert_eq!(config.start.label, "Go");
        assert_eq!(config.start.command, StartConfig::default().command);
        assert!(Config::parse(r#"{"start": {"enabled": true, "command": " "}}"#).is_err());
        assert!(Config::parse(r#"{"start": {"enabled": false, "command": ""}}"#).is_ok());
        assert!(Config::parse(r#"{"start": {"colour": "red"}}"#).is_err());
        assert_eq!(defaults.size, 54);
        assert_eq!(
            Config::parse(r#"{"start": {"size": 80}}"#)
                .unwrap()
                .start
                .size,
            80
        );
        assert!(Config::parse(r#"{"start": {"size": 7}}"#).is_err());
        assert!(Config::parse(r#"{"start": {"size": 513}}"#).is_err());
    }

    #[test]
    fn tray_defaults_off_and_needs_a_real_bar() {
        assert!(!Config::parse("{}").unwrap().tray.enabled);
        let on = r#"{"tray": {"enabled": true, "bar": 0}}"#;
        assert!(Config::parse(on).unwrap().tray.enabled);
        assert!(Config::parse(r#"{"tray": {"enabled": true, "bar": 1}}"#).is_err());
        assert!(Config::parse(r#"{"tray": {"enabled": false, "bar": 5}}"#).is_ok());
        assert!(Config::parse(r#"{"tray": {"icons": 1}}"#).is_err());
    }

    #[test]
    fn widgets_take_a_name_or_a_name_with_a_place() {
        let config = Config::parse(
            r#"{"widgets": ["battery", {"name": "language", "place": "tray"}, {"name": "volume"}]}"#,
        )
        .unwrap();
        let places: Vec<(&str, WidgetPlace)> = config
            .widgets
            .iter()
            .map(|w| (w.name.as_str(), w.place))
            .collect();
        assert_eq!(
            places,
            [
                ("battery", WidgetPlace::Bar),
                ("language", WidgetPlace::Tray),
                ("volume", WidgetPlace::Bar)
            ]
        );
        assert!(Config::parse(r#"{"widgets": [{"name": "x", "place": "left"}]}"#).is_err());
    }

    #[test]
    fn rejects_bad_input() {
        assert!(Config::parse(r#"{"version": 2}"#).is_err());
        assert!(Config::parse(r#"{"groups": [{"name": "x"}]}"#).is_err());
        assert!(Config::parse(r#"{"groups": [{"exes": ["a.exe"]}]}"#).is_err());
        assert!(Config::parse(r#"{"pins": [{"name": "x"}]}"#).is_err());
        assert!(Config::parse(r##"{"appearance": {"bar_color": "red"}}"##).is_err());
        assert!(Config::parse(r#"{"grouping": {"mode": "stack"}}"#).is_err());
        assert!(Config::parse(r#"{"tasks": {"permonitor": true}}"#).is_err());
        assert!(Config::parse("{").is_err());
        assert!(Config::parse(r#"{"bars": [{"edge": "middle"}]}"#).is_err());
        assert!(Config::parse(r#"{"bars": [{"monitor": "second"}]}"#).is_err());
        assert!(Config::parse(r#"{"bars": [{"size": 0}]}"#).is_err());
        assert!(Config::parse(r#"{"colour": 1}"#).is_err());
    }
}
