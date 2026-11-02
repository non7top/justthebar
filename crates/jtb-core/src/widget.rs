//! What the built-in bar widgets show, as plain data. Reading the system is up to the app.

/// Which font a widget's text is drawn with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Face {
    /// Segoe Fluent Icons (or Segoe MDL2 Assets), the glyphs the stock taskbar uses.
    Symbols,
    /// The normal UI font, for short text such as a language code.
    Text,
}

/// One widget's current look: a glyph or a short text in one tray cell, plus its tooltip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetState {
    pub face: Face,
    pub text: String,
    pub tooltip: String,
}

fn glyph(code: u32) -> String {
    char::from_u32(code).map(String::from).unwrap_or_default()
}

/// The battery glyph for `percent` (0 to 100), charging or not (Segoe Fluent Icons, 11 levels).
pub fn battery_state(percent: u8, charging: bool) -> WidgetState {
    let level = (u32::from(percent.min(100)) + 5) / 10;
    let code = match (charging, level) {
        (false, 10) => 0xE83F,
        (false, level) => 0xE850 + level,
        (true, 10) => 0xEA93,
        (true, 9) => 0xE862,
        (true, level) => 0xE85A + level,
    };
    let state = if charging { ", charging" } else { "" };
    WidgetState {
        face: Face::Symbols,
        text: glyph(code),
        tooltip: format!("Battery: {percent}%{state}"),
    }
}

/// The speaker glyph for `percent` (0 to 100), or the muted one.
pub fn volume_state(percent: u8, muted: bool) -> WidgetState {
    let code = if muted || percent == 0 {
        0xE74F
    } else if percent < 34 {
        0xE993
    } else if percent < 67 {
        0xE994
    } else {
        0xE995
    };
    let tooltip = if muted {
        "Speakers: muted".to_string()
    } else {
        format!("Speakers: {percent}%")
    };
    WidgetState {
        face: Face::Symbols,
        text: glyph(code),
        tooltip,
    }
}

/// The input language indicator: the two letter language code in capitals, such as `EN`.
pub fn language_state(iso_code: &str) -> WidgetState {
    let code: String = iso_code
        .chars()
        .take(2)
        .flat_map(char::to_uppercase)
        .collect();
    WidgetState {
        face: Face::Text,
        tooltip: format!("Input language: {code}"),
        text: code,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_levels_map_to_eleven_glyphs() {
        assert_eq!(battery_state(0, false).text, "\u{E850}");
        assert_eq!(battery_state(52, false).text, "\u{E855}");
        assert_eq!(battery_state(100, false).text, "\u{E83F}");
        assert_eq!(battery_state(100, true).text, "\u{EA93}");
        assert_eq!(battery_state(0, true).text, "\u{E85A}");
        assert_eq!(battery_state(90, true).text, "\u{E862}");
    }

    #[test]
    fn battery_tooltip_says_when_charging() {
        assert_eq!(battery_state(80, true).tooltip, "Battery: 80%, charging");
        assert_eq!(battery_state(80, false).tooltip, "Battery: 80%");
    }

    #[test]
    fn volume_glyph_follows_the_level_and_mute() {
        assert_eq!(volume_state(0, false).text, "\u{E74F}");
        assert_eq!(volume_state(50, true).text, "\u{E74F}");
        assert_eq!(volume_state(20, false).text, "\u{E993}");
        assert_eq!(volume_state(50, false).text, "\u{E994}");
        assert_eq!(volume_state(90, false).text, "\u{E995}");
    }

    #[test]
    fn language_is_two_capital_letters() {
        let state = language_state("en");
        assert_eq!(state.text, "EN");
        assert_eq!(state.face, Face::Text);
        assert_eq!(language_state("rus").text, "RU");
    }
}
