use serde::{Deserialize, Serialize};

use crate::activity::NotificationRule;
use crate::config::PinConfig;

/// What the app itself changes while running, kept apart from the user's `jtb.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    /// Keys of groups folded by the user.
    pub folded: Vec<String>,
    /// Group keys in the order the user dragged them.
    pub order: Vec<String>,
    /// Pins added from the bar.
    pub pins: Vec<PinConfig>,
    /// Paths of config pins the user removed from the bar.
    pub unpinned: Vec<String>,
    /// User-created notification/activity rules.
    pub rules: Vec<NotificationRule>,
}

impl State {
    pub fn parse(text: &str) -> Result<State, String> {
        serde_json::from_str(text).map_err(|e| e.to_string())
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn toggle_fold(&mut self, key: &str) {
        match self.folded.iter().position(|k| k == key) {
            Some(index) => {
                self.folded.remove(index);
            }
            None => self.folded.push(key.to_string()),
        }
    }

    /// Moves `from` to where `to` is within `sequence` (the keys on screen, in order). Keys of
    /// other bars that are not in `sequence` keep their relative order, after it.
    pub fn move_key(&mut self, sequence: &[String], from: &str, to: &str) {
        let mut moved = sequence.to_vec();
        let (Some(from_at), Some(to_at)) = (
            moved.iter().position(|k| k == from),
            moved.iter().position(|k| k == to),
        ) else {
            return;
        };
        let key = moved.remove(from_at);
        moved.insert(to_at, key);
        let rest: Vec<String> = self
            .order
            .iter()
            .filter(|k| !moved.contains(k))
            .cloned()
            .collect();
        self.order = moved;
        self.order.extend(rest);
    }

    /// Adds a pin unless one with the same path exists. Returns whether it was added.
    pub fn add_pin(&mut self, pin: PinConfig) -> bool {
        if self
            .pins
            .iter()
            .any(|p| p.path.eq_ignore_ascii_case(&pin.path))
        {
            return false;
        }
        self.unpinned.retain(|p| !p.eq_ignore_ascii_case(&pin.path));
        self.pins.push(pin);
        true
    }

    /// Removes a pin from the bar, whether it came from the config or from the bar.
    pub fn remove_pin(&mut self, path: &str) {
        self.pins.retain(|p| !p.path.eq_ignore_ascii_case(path));
        if !self.unpinned.iter().any(|p| p.eq_ignore_ascii_case(path)) {
            self.unpinned.push(path.to_string());
        }
    }

    /// The config pins that were not removed, then the pins added from the bar, one per path.
    pub fn effective_pins(&self, config_pins: &[PinConfig]) -> Vec<PinConfig> {
        let mut pins: Vec<PinConfig> = config_pins
            .iter()
            .filter(|p| {
                !self
                    .unpinned
                    .iter()
                    .any(|u| u.eq_ignore_ascii_case(&p.path))
            })
            .cloned()
            .collect();
        for pin in &self.pins {
            if !pins.iter().any(|p| p.path.eq_ignore_ascii_case(&pin.path)) {
                pins.push(pin.clone());
            }
        }
        pins
    }

    /// Adds a notification rule for an executable, replacing an existing rule for that executable.
    pub fn add_rule(&mut self, rule: NotificationRule) {
        self.rules
            .retain(|r| !r.exe.eq_ignore_ascii_case(&rule.exe));
        self.rules.push(rule);
    }

    /// Returns the active rules: config rules plus user-created rules from state.
    /// Falls back to default coding-agent rules if none are configured.
    pub fn effective_rules(&self, config_rules: &[NotificationRule]) -> Vec<NotificationRule> {
        let mut rules = config_rules.to_vec();
        for rule in &self.rules {
            if !rules.iter().any(|r| r.exe.eq_ignore_ascii_case(&rule.exe)) {
                rules.push(rule.clone());
            }
        }
        if rules.is_empty() {
            crate::activity::default_rules()
        } else {
            rules
        }
    }
}

/// Splits a command line the way a shortcut stores it: spaces separate, double quotes group.
pub fn split_args(line: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut started = false;
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            c if c.is_whitespace() && !quoted => {
                if started {
                    args.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            c => {
                current.push(c);
                started = true;
            }
        }
    }
    if started {
        args.push(current);
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(list: &[&str]) -> Vec<String> {
        list.iter().map(|k| k.to_string()).collect()
    }

    #[test]
    fn folding_toggles() {
        let mut state = State::default();
        state.toggle_fold("a");
        assert_eq!(state.folded, keys(&["a"]));
        state.toggle_fold("a");
        assert!(state.folded.is_empty());
    }

    #[test]
    fn moving_a_key_down_and_up() {
        let mut state = State::default();
        let seq = keys(&["a", "b", "c"]);
        state.move_key(&seq, "a", "c");
        assert_eq!(state.order, keys(&["b", "c", "a"]));
        state.move_key(&seq, "c", "a");
        assert_eq!(state.order, keys(&["c", "a", "b"]));
    }

    #[test]
    fn keys_of_other_bars_stay_after_the_moved_ones() {
        let mut state = State {
            order: keys(&["x", "a", "y"]),
            ..State::default()
        };
        state.move_key(&keys(&["a", "b"]), "b", "a");
        assert_eq!(state.order, keys(&["b", "a", "x", "y"]));
    }

    #[test]
    fn unknown_keys_do_nothing() {
        let mut state = State::default();
        state.move_key(&keys(&["a"]), "a", "zzz");
        assert!(state.order.is_empty());
    }

    #[test]
    fn pins_are_unique_by_path() {
        let mut state = State::default();
        let pin = PinConfig {
            name: "n".into(),
            path: r"C:\a.exe".into(),
            args: vec![],
        };
        assert!(state.add_pin(pin.clone()));
        assert!(!state.add_pin(PinConfig {
            path: r"c:\A.EXE".into(),
            ..pin
        }));
        state.remove_pin(r"C:\A.exe");
        assert!(state.pins.is_empty());
    }

    #[test]
    fn a_config_pin_can_be_hidden_and_pinned_again() {
        let config_pin = PinConfig {
            name: "n".into(),
            path: r"C:\a.exe".into(),
            args: vec![],
        };
        let config_pins = [config_pin.clone()];
        let mut state = State::default();
        assert_eq!(state.effective_pins(&config_pins), vec![config_pin.clone()]);
        state.remove_pin(r"c:\A.EXE");
        assert!(state.effective_pins(&config_pins).is_empty());
        assert!(state.add_pin(config_pin.clone()));
        assert!(state.unpinned.is_empty());
        assert_eq!(state.effective_pins(&config_pins), vec![config_pin]);
    }

    #[test]
    fn state_round_trips_and_tolerates_missing_keys() {
        let mut state = State::default();
        state.toggle_fold("a");
        assert_eq!(State::parse(&state.to_json()), Ok(state));
        assert_eq!(State::parse("{}"), Ok(State::default()));
    }

    #[test]
    fn args_split_on_spaces_outside_quotes() {
        assert_eq!(
            split_args(r#"-load "my session" -x"#),
            keys(&["-load", "my session", "-x"])
        );
        assert!(split_args("   ").is_empty());
        assert_eq!(split_args(r#""""#), keys(&[""]));
    }
}
