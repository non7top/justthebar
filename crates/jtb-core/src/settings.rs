//! The options shown in the settings window, as data: a path into `jtb.json`, a type, a text and
//! the default. The window is built from this table, so a new option is one more entry here.
//!
//! Values are read from and written to the parsed JSON tree of the file, so keys this table does
//! not know (pins, groups) are kept as they are.
use serde_json::{Map, Value};

use crate::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Bool,
    Int {
        min: i64,
        max: i64,
    },
    Text,
    /// One of these words.
    Choice(&'static [&'static str]),
    /// `#rrggbb`.
    Color,
    /// `primary` or a 0-based number.
    Monitor,
    /// Comma separated words. A widget may carry its place: `battery:tray`.
    List,
}

#[derive(Debug)]
pub struct OptionSpec {
    /// Dotted path into the file; a `*` stands for every entry of an array (`bars.*.edge`).
    pub path: &'static str,
    pub kind: Kind,
    pub description: &'static str,
    /// What the program uses when the file has no value, as it would be typed.
    pub default: &'static str,
}

const MOUSE_ACTIONS: &[&str] = &["activate", "launch", "toggle_fold", "none"];

pub const SCHEMA: &[OptionSpec] = &[
    OptionSpec {
        path: "hide_explorer_taskbar",
        kind: Kind::Bool,
        description: "Hide Explorer's own taskbar while the bars run.",
        default: "false",
    },
    OptionSpec {
        path: "bars.*.monitor",
        kind: Kind::Monitor,
        description: "Monitor of this bar: primary, or a 0-based number.",
        default: "primary",
    },
    OptionSpec {
        path: "bars.*.edge",
        kind: Kind::Choice(&["top", "bottom", "left", "right"]),
        description: "Screen edge of this bar.",
        default: "bottom",
    },
    OptionSpec {
        path: "bars.*.size",
        kind: Kind::Int { min: 1, max: 1000 },
        description: "Thickness of this bar in pixels at 100% scale.",
        default: "40",
    },
    OptionSpec {
        path: "bars.*.enabled",
        kind: Kind::Bool,
        description: "Show this bar.",
        default: "true",
    },
    OptionSpec {
        path: "appearance.bar_color",
        kind: Kind::Color,
        description: "Background color of the bars, #rrggbb.",
        default: "#171a1f",
    },
    OptionSpec {
        path: "tasks.per_monitor",
        kind: Kind::Bool,
        description: "Each bar lists only the windows on its own monitor.",
        default: "true",
    },
    OptionSpec {
        path: "grouping.mode",
        kind: Kind::Choice(&["off", "accordion"]),
        description: "off: one button per window; accordion: windows of one app fold under a header.",
        default: "accordion",
    },
    OptionSpec {
        path: "mouse.header_left",
        kind: Kind::Choice(MOUSE_ACTIONS),
        description: "Left click on a group header.",
        default: "toggle_fold",
    },
    OptionSpec {
        path: "mouse.header_middle",
        kind: Kind::Choice(MOUSE_ACTIONS),
        description: "Middle click on a group header.",
        default: "launch",
    },
    OptionSpec {
        path: "mouse.window_left",
        kind: Kind::Choice(MOUSE_ACTIONS),
        description: "Left click on a window button.",
        default: "activate",
    },
    OptionSpec {
        path: "mouse.window_middle",
        kind: Kind::Choice(MOUSE_ACTIONS),
        description: "Middle click on a window button.",
        default: "none",
    },
    OptionSpec {
        path: "start.enabled",
        kind: Kind::Bool,
        description: "Draw a Start button at the start of the bar.",
        default: "false",
    },
    OptionSpec {
        path: "start.menu",
        kind: Kind::Choice(&["stock", "openshell"]),
        description: "Start menu provider: stock built-in flat widget or Open-Shell.",
        default: "stock",
    },
    OptionSpec {
        path: "start.command",
        kind: Kind::Text,
        description: "Program the Start button runs (Open-Shell's StartMenu.exe).",
        default: r"C:\Program Files\Open-Shell\StartMenu.exe",
    },
    OptionSpec {
        path: "start.args",
        kind: Kind::List,
        description: "Arguments of that program, comma separated.",
        default: "-toggle",
    },
    OptionSpec {
        path: "start.label",
        kind: Kind::Text,
        description: "Text of the Start button when it has no art.",
        default: "Start",
    },
    OptionSpec {
        path: "start.image",
        kind: Kind::Text,
        description: "Start button art in Open-Shell's strip format, next to the exe.",
        default: "start-button.png",
    },
    OptionSpec {
        path: "start.size",
        kind: Kind::Int { min: 8, max: 512 },
        description: "Longest side of the Start button art in pixels at 100% scale.",
        default: "54",
    },
    OptionSpec {
        path: "start.in_process",
        kind: Kind::Bool,
        description: "Open Open-Shell's menu from a copy of its library inside this app, which places it next to the button.",
        default: "true",
    },
    OptionSpec {
        path: "tray.enabled",
        kind: Kind::Bool,
        description: "Show the notification area icons on one of the bars.",
        default: "false",
    },
    OptionSpec {
        path: "tray.bar",
        kind: Kind::Int { min: 0, max: 64 },
        description: "Number of the bar (0-based, in the list above) that shows the icons.",
        default: "0",
    },
    OptionSpec {
        path: "clock.format",
        kind: Kind::Text,
        description: "Clock format string (e.g. 'hh:mm:ss\\nDDD, dd MMM' or '%H:%M:%S\\n%a, %d %b').",
        default: "%H:%M",
    },
    OptionSpec {
        path: "clock.months",
        kind: Kind::Int { min: 1, max: 12 },
        description: "Number of calendar months to display stacked vertically in the popup.",
        default: "3",
    },
    OptionSpec {
        path: "widgets",
        kind: Kind::List,
        description: "Widgets in order: battery, language, network, volume. Add :tray to a name to put it in the notification area.",
        default: "",
    },
];

/// One line of the settings window: an option with its path resolved to an array entry.
#[derive(Debug, Clone)]
pub struct Row {
    pub path: String,
    pub spec: &'static OptionSpec,
}

/// The options to show for the file `root`: the table, with `*` replaced by every entry of the
/// array in the file (one entry when the file has none, as the program then has one default bar).
pub fn rows(root: &Value) -> Vec<Row> {
    let mut rows = Vec::new();
    for spec in SCHEMA {
        match spec.path.split_once(".*.") {
            None => rows.push(Row {
                path: spec.path.to_string(),
                spec,
            }),
            Some((array, rest)) => {
                let count = root
                    .get(array)
                    .and_then(Value::as_array)
                    .map_or(1, |a| a.len().max(1));
                for index in 0..count {
                    rows.push(Row {
                        path: format!("{array}.{index}.{rest}"),
                        spec,
                    });
                }
            }
        }
    }
    // Keep the entries of one array together: bar 0 with all its options, then bar 1.
    rows.sort_by_key(|row| array_key(&row.path));
    rows
}

fn array_key(path: &str) -> (usize, usize) {
    let mut parts = path.split('.');
    let first = parts.next().unwrap_or("");
    let index = parts.next().and_then(|p| p.parse().ok());
    let order = SCHEMA
        .iter()
        .position(|spec| spec.path.split(".*.").next() == Some(first) || spec.path == path)
        .unwrap_or(usize::MAX);
    (order, index.unwrap_or(0))
}

fn lookup<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.')
        .try_fold(root, |value, part| match part.parse::<usize>() {
            Ok(index) if value.is_array() => value.get(index),
            _ => value.get(part),
        })
}

/// The value as it is typed in the window: the file's value, else the default.
pub fn get(root: &Value, row: &Row) -> String {
    match lookup(root, &row.path) {
        None | Some(Value::Null) => row.spec.default.to_string(),
        Some(value) => show(value, row.spec.kind),
    }
}

fn show(value: &Value, kind: Kind) -> String {
    match (kind, value) {
        (_, Value::String(text)) => text.clone(),
        (_, Value::Bool(flag)) => flag.to_string(),
        (_, Value::Number(number)) => number.to_string(),
        (Kind::List, Value::Array(items)) => {
            items.iter().map(show_item).collect::<Vec<_>>().join(", ")
        }
        (_, other) => other.to_string(),
    }
}

fn show_item(item: &Value) -> String {
    match item {
        Value::String(text) => text.clone(),
        Value::Object(map) => {
            let name = map.get("name").and_then(Value::as_str).unwrap_or("");
            match map.get("place").and_then(Value::as_str) {
                Some(place) if place != "bar" => format!("{name}:{place}"),
                _ => name.to_string(),
            }
        }
        other => other.to_string(),
    }
}

/// Checks `text` for the option's type and returns the JSON value to store.
pub fn parse(kind: Kind, text: &str) -> Result<Value, String> {
    let text = text.trim();
    match kind {
        Kind::Bool => match text {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            _ => Err("must be true or false".to_string()),
        },
        Kind::Int { min, max } => {
            let number: i64 = text
                .parse()
                .map_err(|_| "must be a whole number".to_string())?;
            if (min..=max).contains(&number) {
                Ok(Value::from(number))
            } else {
                Err(format!("must be {min} to {max}"))
            }
        }
        Kind::Text => Ok(Value::String(text.to_string())),
        Kind::Choice(words) => {
            if words.contains(&text) {
                Ok(Value::String(text.to_string()))
            } else {
                Err(format!("must be one of: {}", words.join(", ")))
            }
        }
        Kind::Color => {
            let digits = text.strip_prefix('#').unwrap_or("");
            if digits.len() == 6 && digits.chars().all(|c| c.is_ascii_hexdigit()) {
                Ok(Value::String(text.to_lowercase()))
            } else {
                Err("must look like #rrggbb".to_string())
            }
        }
        Kind::Monitor => {
            if text == "primary" {
                Ok(Value::String(text.to_string()))
            } else {
                text.parse::<u64>()
                    .map(Value::from)
                    .map_err(|_| "must be primary or a 0-based number".to_string())
            }
        }
        Kind::List => Ok(Value::Array(
            text.split(',')
                .map(str::trim)
                .filter(|word| !word.is_empty())
                .map(|word| match word.split_once(':') {
                    Some((name, place)) if place != "bar" => {
                        let mut map = Map::new();
                        map.insert("name".to_string(), Value::String(name.trim().to_string()));
                        map.insert("place".to_string(), Value::String(place.trim().to_string()));
                        Value::Object(map)
                    }
                    Some((name, _)) => Value::String(name.trim().to_string()),
                    None => Value::String(word.to_string()),
                })
                .collect(),
        )),
    }
}

/// Stores `text` at the row's path, creating the objects on the way.
pub fn set(root: &mut Value, row: &Row, text: &str) -> Result<(), String> {
    let value = parse(row.spec.kind, text)?;
    let mut node = root;
    let parts: Vec<&str> = row.path.split('.').collect();
    for (at, part) in parts.iter().enumerate() {
        let last = at + 1 == parts.len();
        if let Ok(index) = part.parse::<usize>() {
            let array = node
                .as_array_mut()
                .ok_or_else(|| format!("{} is not a list", parts[..at].join(".")))?;
            while array.len() <= index {
                array.push(Value::Object(Map::new()));
            }
            node = &mut array[index];
        } else {
            if !node.is_object() {
                *node = Value::Object(Map::new());
            }
            let map = node.as_object_mut().ok_or("not an object")?;
            if last {
                map.insert((*part).to_string(), value);
                return Ok(());
            }
            let next_is_index = parts
                .get(at + 1)
                .is_some_and(|p| p.parse::<usize>().is_ok());
            node = map.entry((*part).to_string()).or_insert_with(|| {
                if next_is_index {
                    Value::Array(vec![Value::Object(Map::new())])
                } else {
                    Value::Object(Map::new())
                }
            });
        }
    }
    *node = value;
    Ok(())
}

/// Whether the whole file would still load, for example `tray.bar` pointing at an existing bar.
pub fn validate(root: &Value) -> Result<(), String> {
    Config::parse(&root.to_string()).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row(root: &Value, path: &str) -> Row {
        rows(root)
            .into_iter()
            .find(|row| row.path == path)
            .unwrap_or_else(|| panic!("no row {path}"))
    }

    #[test]
    fn every_default_in_the_table_matches_the_program() {
        let root = json!({});
        for row in rows(&root) {
            let mut copy = root.clone();
            set(&mut copy, &row, &get(&root, &row)).unwrap_or_else(|e| panic!("{}: {e}", row.path));
        }
        let mut filled = json!({});
        for row in rows(&filled.clone()) {
            let text = get(&filled, &row);
            set(&mut filled, &row, &text).unwrap();
        }
        assert_eq!(
            Config::parse(&filled.to_string()).unwrap(),
            Config::parse("{}").unwrap(),
            "a default in SCHEMA differs from the program's default"
        );
    }

    #[test]
    fn bars_expand_to_one_row_per_entry_in_order() {
        let root = json!({"bars": [{"edge": "right"}, {"edge": "left"}]});
        let paths: Vec<String> = rows(&root).into_iter().map(|r| r.path).collect();
        let first = paths.iter().position(|p| p == "bars.0.monitor").unwrap();
        assert_eq!(paths[first + 3], "bars.0.enabled");
        assert_eq!(paths[first + 4], "bars.1.monitor");
    }

    #[test]
    fn reads_the_file_value_and_falls_back_to_the_default() {
        let root = json!({"bars": [{"edge": "right", "size": 200}]});
        assert_eq!(get(&root, &row(&root, "bars.0.edge")), "right");
        assert_eq!(get(&root, &row(&root, "bars.0.size")), "200");
        assert_eq!(get(&root, &row(&root, "bars.0.enabled")), "true");
        assert_eq!(get(&root, &row(&root, "start.size")), "54");
    }

    #[test]
    fn sets_a_value_and_keeps_unknown_keys() {
        let mut root = json!({"pins": [{"name": "a", "path": "b"}], "tray": {"enabled": false}});
        let target = row(&root, "tray.enabled");
        set(&mut root, &target, "true").unwrap();
        let target = row(&root, "start.label");
        set(&mut root, &target, "Go").unwrap();
        assert_eq!(root["tray"]["enabled"], json!(true));
        assert_eq!(root["start"]["label"], json!("Go"));
        assert_eq!(root["pins"][0]["name"], json!("a"));
    }

    #[test]
    fn saving_keeps_key_order_and_groups() {
        let text = r#"{"version":1,"zeta":1,"groups":[{"name":"g","exes":["a.exe"]}],"alpha":2}"#;
        let mut root: Value = serde_json::from_str(text).unwrap();
        let target = row(&root, "tray.enabled");
        set(&mut root, &target, "false").unwrap();
        let saved = serde_json::to_string(&root).unwrap();
        assert!(saved.starts_with(
            r#"{"version":1,"zeta":1,"groups":[{"name":"g","exes":["a.exe"]}],"alpha":2"#
        ));
        assert!(saved.ends_with(r#""tray":{"enabled":false}}"#));
    }

    #[test]
    fn rejects_values_of_the_wrong_type() {
        let root = json!({});
        let size = row(&root, "bars.0.size");
        assert!(parse(size.spec.kind, "0").is_err());
        assert!(parse(size.spec.kind, "abc").is_err());
        assert!(parse(row(&root, "bars.0.edge").spec.kind, "middle").is_err());
        assert!(parse(row(&root, "appearance.bar_color").spec.kind, "red").is_err());
        assert!(parse(row(&root, "bars.0.monitor").spec.kind, "second").is_err());
        assert_eq!(
            parse(row(&root, "bars.0.monitor").spec.kind, "1").unwrap(),
            json!(1)
        );
        assert_eq!(
            parse(row(&root, "appearance.bar_color").spec.kind, "#AABBCC").unwrap(),
            json!("#aabbcc")
        );
    }

    #[test]
    fn widgets_keep_their_place() {
        let root = json!({"widgets": ["language", {"name": "battery", "place": "tray"}]});
        let widgets = row(&root, "widgets");
        assert_eq!(get(&root, &widgets), "language, battery:tray");
        let mut copy = root.clone();
        set(&mut copy, &widgets, "network, volume:tray, battery:bar").unwrap();
        assert_eq!(
            copy["widgets"],
            json!(["network", {"name": "volume", "place": "tray"}, "battery"])
        );
        Config::parse(&copy.to_string()).unwrap();
    }

    #[test]
    fn validate_catches_a_tray_bar_that_does_not_exist() {
        let mut root = json!({"bars": [{"edge": "right"}]});
        let tray = row(&root, "tray.enabled");
        set(&mut root, &tray, "true").unwrap();
        let bar = row(&root, "tray.bar");
        set(&mut root, &bar, "3").unwrap();
        assert!(validate(&root).is_err());
        set(&mut root, &bar, "0").unwrap();
        assert!(validate(&root).is_ok());
    }
}
