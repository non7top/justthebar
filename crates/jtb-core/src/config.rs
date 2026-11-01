use serde::Deserialize;

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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub hide_explorer_taskbar: bool,
    pub bars: Vec<BarConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            hide_explorer_taskbar: false,
            bars: vec![BarConfig::default()],
        }
    }
}

impl Config {
    pub fn parse(text: &str) -> Result<Config, String> {
        let config: Config = serde_json::from_str(text).map_err(|e| e.to_string())?;
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
    fn rejects_bad_input() {
        assert!(Config::parse("{").is_err());
        assert!(Config::parse(r#"{"bars": [{"edge": "middle"}]}"#).is_err());
        assert!(Config::parse(r#"{"bars": [{"monitor": "second"}]}"#).is_err());
        assert!(Config::parse(r#"{"bars": [{"size": 0}]}"#).is_err());
        assert!(Config::parse(r#"{"colour": 1}"#).is_err());
    }
}
