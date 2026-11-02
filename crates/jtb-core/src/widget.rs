//! What the built-in bar widgets show, as plain data. Reading the system is up to the app.

/// One widget's current look: a symbol glyph and/or a short label, plus its tooltip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetState {
    /// A glyph of the symbol font; empty for a text-only widget.
    pub symbol: String,
    /// Short text next to the glyph, such as `87%` or `EN`; may be empty.
    pub label: String,
    pub tooltip: String,
}

/// How long (pixels at 100% scale) a widget on the bar is: its glyph and text with some padding.
pub fn bar_length(state: &WidgetState) -> f32 {
    let text = state.label.chars().count() as f32 * 8.0;
    let symbol = if state.symbol.is_empty() {
        0.0
    } else if text > 0.0 {
        20.0
    } else {
        16.0
    };
    (12.0 + symbol + text).max(28.0)
}

fn glyph(code: u32) -> String {
    char::from_u32(code).map(String::from).unwrap_or_default()
}

/// The battery glyph for `percent` (0 to 100), charging or not (11 levels), with the percentage.
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
        symbol: glyph(code),
        label: format!("{percent}%"),
        tooltip: format!("Battery: {percent}%{state}"),
    }
}

/// The speaker glyph for `percent` (0 to 100), or the muted one, with the percentage.
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
    let (label, tooltip) = if muted {
        (String::new(), "Speakers: muted".to_string())
    } else {
        (format!("{percent}%"), format!("Speakers: {percent}%"))
    };
    WidgetState {
        symbol: glyph(code),
        label,
        tooltip,
    }
}

/// How the machine is connected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Network {
    /// Wi-Fi: the network name and the signal quality in percent.
    Wifi {
        name: String,
        quality: u8,
    },
    Ethernet,
    Offline,
}

/// The network glyph: signal bars for Wi-Fi, a plug for a cable, a globe when offline.
pub fn network_state(network: &Network) -> WidgetState {
    let (code, tooltip) = match network {
        Network::Wifi { name, quality } => {
            let code = match quality {
                0..=24 => 0xE872,
                25..=49 => 0xE873,
                50..=74 => 0xE874,
                _ => 0xE701,
            };
            (code, format!("Wi-Fi: {name}, signal {quality}%"))
        }
        Network::Ethernet => (0xE839, "Network: wired".to_string()),
        Network::Offline => (0xE774, "Network: not connected".to_string()),
    };
    WidgetState {
        symbol: glyph(code),
        label: String::new(),
        tooltip,
    }
}

/// The input language indicator: the three letter language code in capitals, such as `ENG`.
pub fn language_state(iso_code: &str) -> WidgetState {
    let code: String = iso_code
        .chars()
        .take(3)
        .flat_map(char::to_uppercase)
        .collect();
    WidgetState {
        symbol: String::new(),
        tooltip: format!("Input language: {code}"),
        label: code,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_levels_map_to_eleven_glyphs() {
        assert_eq!(battery_state(0, false).symbol, "\u{E850}");
        assert_eq!(battery_state(52, false).symbol, "\u{E855}");
        assert_eq!(battery_state(100, false).symbol, "\u{E83F}");
        assert_eq!(battery_state(100, true).symbol, "\u{EA93}");
        assert_eq!(battery_state(0, true).symbol, "\u{E85A}");
        assert_eq!(battery_state(90, true).symbol, "\u{E862}");
    }

    #[test]
    fn battery_tooltip_says_when_charging() {
        assert_eq!(battery_state(80, true).tooltip, "Battery: 80%, charging");
        assert_eq!(battery_state(80, false).tooltip, "Battery: 80%");
    }

    #[test]
    fn volume_glyph_follows_the_level_and_mute() {
        assert_eq!(volume_state(0, false).symbol, "\u{E74F}");
        assert_eq!(volume_state(50, true).symbol, "\u{E74F}");
        assert_eq!(volume_state(20, false).symbol, "\u{E993}");
        assert_eq!(volume_state(50, false).symbol, "\u{E994}");
        assert_eq!(volume_state(90, false).symbol, "\u{E995}");
    }

    #[test]
    fn wifi_glyph_follows_the_signal() {
        let wifi = |quality| Network::Wifi {
            name: "home".to_string(),
            quality,
        };
        assert_eq!(network_state(&wifi(10)).symbol, "\u{E872}");
        assert_eq!(network_state(&wifi(60)).symbol, "\u{E874}");
        assert_eq!(network_state(&wifi(100)).symbol, "\u{E701}");
        assert_eq!(network_state(&wifi(60)).tooltip, "Wi-Fi: home, signal 60%");
        assert_eq!(network_state(&Network::Ethernet).symbol, "\u{E839}");
        assert_eq!(network_state(&Network::Offline).symbol, "\u{E774}");
    }

    #[test]
    fn language_is_three_capital_letters() {
        let state = language_state("eng");
        assert_eq!(state.label, "ENG");
        assert!(state.symbol.is_empty());
        assert_eq!(language_state("ru").label, "RU");
    }

    #[test]
    fn bar_length_grows_with_the_label() {
        let battery = bar_length(&battery_state(87, false));
        let language = bar_length(&language_state("eng"));
        assert!(battery > language);
        assert!(language >= 28.0);
    }
}
