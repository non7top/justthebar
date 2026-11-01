use serde::Deserialize;

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

/// Reserved: an app pinned to the bar. Parsed, not applied yet.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PinConfig {
    pub name: String,
    pub path: String,
    pub args: Vec<String>,
}

/// Reserved: a user-defined group of executables. Parsed, not applied yet.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GroupConfig {
    pub name: String,
    pub exes: Vec<String>,
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
    pub pins: Vec<PinConfig>,
    pub groups: Vec<GroupConfig>,
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
            pins: Vec::new(),
            groups: Vec::new(),
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
        assert_eq!(config.tasks, TasksConfig::default());
        assert_eq!(config.grouping, GroupingConfig::default());
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
    fn rejects_bad_input() {
        assert!(Config::parse(r#"{"version": 2}"#).is_err());
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
