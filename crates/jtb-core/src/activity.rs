use regex::RegexBuilder;
use serde::{Deserialize, Serialize};

/// The activity state of a task window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActivityState {
    #[default]
    Unknown,
    Working,
    Idle,
    Prompt,
}

/// A rule that maps window titles and executables to activity states.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationRule {
    /// Executable name or substring (e.g. "WindowsTerminal.exe", "code.exe"), or "*" for any.
    pub exe: String,
    /// Optional title filter substring or regex (e.g. "Claude", "Aider").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_filter: Option<String>,
    /// Substring or regex indicating a prompt (triggers blinking/flashing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Substring or regex indicating the app is actively working.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working: Option<String>,
    /// Substring or regex indicating the app is idle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle: Option<String>,
}

/// Tests whether `pattern` matches `text`, trying regex first and falling back to case-insensitive substring.
pub fn pattern_matches(pattern: &str, text: &str) -> bool {
    if pattern.is_empty() {
        return false;
    }
    if let Ok(re) = RegexBuilder::new(pattern).case_insensitive(true).build() {
        if re.is_match(text) {
            return true;
        }
    }
    text.to_lowercase().contains(&pattern.to_lowercase())
}

/// Checks whether `rule` applies to a window with `exe` path and `title`.
pub fn rule_applies(rule: &NotificationRule, exe: &str, title: &str) -> bool {
    if !rule.exe.is_empty() && rule.exe != "*" {
        let exe_name = std::path::Path::new(exe)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(exe);
        if !exe_name.eq_ignore_ascii_case(&rule.exe)
            && !exe.to_lowercase().contains(&rule.exe.to_lowercase())
        {
            return false;
        }
    }
    if let Some(filter) = &rule.title_filter {
        if !pattern_matches(filter, title) {
            return false;
        }
    }
    true
}

/// Classifies a window's title into an activity state based on rules.
pub fn classify_title(rules: &[NotificationRule], exe: &str, title: &str) -> Option<ActivityState> {
    for rule in rules {
        if !rule_applies(rule, exe, title) {
            continue;
        }
        if let Some(pat) = &rule.prompt {
            if pattern_matches(pat, title) {
                return Some(ActivityState::Prompt);
            }
        }
        if let Some(pat) = &rule.working {
            if pattern_matches(pat, title) {
                return Some(ActivityState::Working);
            }
        }
        if let Some(pat) = &rule.idle {
            if pattern_matches(pat, title) {
                return Some(ActivityState::Idle);
            }
        }
    }
    None
}

/// Built-in notification rules targeting common coding agent titles.
pub fn default_rules() -> Vec<NotificationRule> {
    vec![NotificationRule {
        exe: "*".into(),
        title_filter: None,
        prompt: Some(r"(?i)(waiting for (user|input)|prompt:|\[y/n\]|\bapprove\b|\? )".into()),
        working: Some(r"(?i)(\bworking\b|\bthinking\b|\bgenerating\b|running\s+(bash|tool|command)|\bprocessing\b)".into()),
        idle: Some(r"(?i)(\bidle\b|\bready\b|\bdone\b|\bfinished\b)".into()),
    }]
}

/// Parses user input string from the rule creation dialog into a `NotificationRule`.
/// Format: `[optional_title_filter] prompt_pattern [| working_pattern [| idle_pattern]]`.
pub fn parse_rule_input(exe: &str, input: &str) -> Option<NotificationRule> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }
    let (title_filter, rest) = if input.starts_with('[') {
        if let Some(close) = input.find(']') {
            let filter = input[1..close].trim().to_string();
            let rest = input[close + 1..].trim();
            (Some(filter), rest)
        } else {
            (None, input)
        }
    } else {
        (None, input)
    };
    let parts: Vec<&str> = rest
        .split('|')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    let prompt = parts.first().map(|s| s.to_string());
    let working = parts.get(1).map(|s| s.to_string());
    let idle = parts.get(2).map(|s| s.to_string());
    Some(NotificationRule {
        exe: exe.to_string(),
        title_filter,
        prompt,
        working,
        idle,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_with_regex_or_substring() {
        assert!(pattern_matches(
            r"Claude - (working|busy)",
            "Claude - working on task"
        ));
        assert!(pattern_matches(r"[y/n]", "Do you want to proceed? [y/n]"));
        assert!(pattern_matches("waiting", "Agent waiting for input"));
        assert!(!pattern_matches("running", "Agent waiting for input"));
    }

    #[test]
    fn classifies_prompt_working_and_idle() {
        let rules = default_rules();
        assert_eq!(
            classify_title(&rules, "wt.exe", "Claude - thinking..."),
            Some(ActivityState::Working)
        );
        assert_eq!(
            classify_title(&rules, "wt.exe", "Claude - idle"),
            Some(ActivityState::Idle)
        );
        assert_eq!(
            classify_title(&rules, "wt.exe", "Claude - waiting for input"),
            Some(ActivityState::Prompt)
        );
        assert_eq!(
            classify_title(&rules, "notepad.exe", "Untitled - Notepad"),
            None
        );
    }

    #[test]
    fn rule_applies_matches_exe_and_filter() {
        let rule = NotificationRule {
            exe: "WindowsTerminal.exe".into(),
            title_filter: Some("Claude".into()),
            prompt: Some("prompt".into()),
            working: None,
            idle: None,
        };
        assert!(rule_applies(
            &rule,
            r"C:\path\WindowsTerminal.exe",
            "Claude session"
        ));
        assert!(!rule_applies(&rule, r"C:\path\cmd.exe", "Claude session"));
        assert!(!rule_applies(
            &rule,
            r"C:\path\WindowsTerminal.exe",
            "powershell"
        ));
    }

    #[test]
    fn parses_rule_input_variations() {
        let r1 = parse_rule_input("wt.exe", "waiting").unwrap();
        assert_eq!(r1.exe, "wt.exe");
        assert_eq!(r1.prompt.as_deref(), Some("waiting"));
        assert_eq!(r1.working, None);
        assert_eq!(r1.idle, None);

        let r2 = parse_rule_input("wt.exe", "[Claude] prompt | working | idle").unwrap();
        assert_eq!(r2.title_filter.as_deref(), Some("Claude"));
        assert_eq!(r2.prompt.as_deref(), Some("prompt"));
        assert_eq!(r2.working.as_deref(), Some("working"));
        assert_eq!(r2.idle.as_deref(), Some("idle"));
    }
}
