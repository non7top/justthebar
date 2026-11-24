//! What the built-in bar widgets show, as plain data. Reading the system is up to the app.
use crate::widgeticon::WidgetIcon;

/// One widget's current look: an icon (drawn by the bar, or a glyph of the symbol font) and/or a
/// short label, plus its tooltip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WidgetState {
    /// An icon the bar draws itself; takes the place of `symbol`.
    pub icon: Option<WidgetIcon>,
    /// A glyph of the symbol font; empty for a text-only widget or one with a drawn icon.
    pub symbol: String,
    /// Short text next to the icon, such as `87%` or `EN`; may be empty.
    pub label: String,
    pub tooltip: String,
    /// Color of the glyph (red, green, blue); none draws it in the bar's text color.
    pub tint: Option<[u8; 3]>,
}

impl WidgetState {
    /// Whether there is an icon or glyph, as opposed to text alone.
    pub fn has_art(&self) -> bool {
        self.icon.is_some() || !self.symbol.is_empty()
    }

    /// Height of the icon on the bar in pixels at 100% scale. The battery is as tall as the ink of
    /// the signal glyph beside it; the volume bars are drawn larger than the glyphs.
    pub fn icon_size(&self) -> f32 {
        match self.icon {
            Some(WidgetIcon::Battery { .. }) => BATTERY_ICON,
            Some(WidgetIcon::Volume { .. }) => VOLUME_ICON,
            None => ICON,
        }
    }

    /// Width over height of the icon.
    pub fn icon_aspect(&self) -> f32 {
        self.icon.map_or(1.0, |icon| icon.aspect())
    }

    /// Whether the label is drawn on the icon, taking no room of its own.
    pub fn label_on_icon(&self) -> bool {
        matches!(self.icon, Some(WidgetIcon::Battery { .. }))
    }
}

/// Size of an icon on the bar, in pixels at 100% scale.
pub const ICON: f32 = 20.0;
const VOLUME_ICON: f32 = 26.0;
const BATTERY_ICON: f32 = 18.0;
/// Space around a widget's content, and between its icon and label.
pub const PAD: f32 = 2.0;
pub const GAP: f32 = 3.0;
/// Height of a label line under the icon in the stacked layout.
pub const LABEL_LINE: f32 = 19.0;
/// Extra width on a measured label so rounding never trims it.
const SLACK: f32 = 2.0;
/// Thickness of a widget cell on a side bar: icon and label side by side, or stacked.
pub const SIDE_THICK: f32 = 28.0;
pub const STACKED_THICK: f32 = PAD + ICON + 1.0 + LABEL_LINE + PAD;

/// How long a widget is along the bar in each layout: icon and label side by side, the label under
/// the icon, and the icon alone (a text-only widget keeps its text).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WidgetSizes {
    pub side: f32,
    pub stacked: f32,
    pub compact: f32,
}

impl WidgetSizes {
    pub fn array(self) -> [f32; 3] {
        [self.side, self.stacked, self.compact]
    }
}

/// The lengths (pixels at 100% scale) of a widget whose label measures `label_width` pixels.
pub fn sizes(state: &WidgetState, label_width: f32) -> WidgetSizes {
    let text = if state.label.is_empty() || state.label_on_icon() {
        0.0
    } else {
        label_width + SLACK
    };
    let icon = state.icon_size() * state.icon_aspect();
    let floor = ICON + 2.0 * PAD;
    if !state.has_art() {
        let own = (text + 2.0 * PAD).max(floor);
        return WidgetSizes {
            side: own,
            stacked: own,
            compact: own,
        };
    }
    let side = if text > 0.0 { icon + GAP + text } else { icon };
    WidgetSizes {
        side: side + 2.0 * PAD,
        stacked: icon.max(text) + 2.0 * PAD,
        compact: (icon + 2.0 * PAD).max(floor),
    }
}

fn glyph(code: u32) -> String {
    char::from_u32(code).map(String::from).unwrap_or_default()
}

/// Where the machine gets its power from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Power {
    Battery,
    /// The power cable is connected and the battery is charging.
    Charging,
    /// The power cable is connected and the battery is not charging, as when it is full.
    Plugged,
}

/// The battery icon for `percent` (0 to 100) and the power source. The number is drawn on the
/// icon; the percent sign is left for the tooltip.
pub fn battery_state(percent: u8, power: Power) -> WidgetState {
    let note = match power {
        Power::Battery => "",
        Power::Charging => ", charging",
        Power::Plugged => ", plugged in",
    };
    WidgetState {
        icon: Some(WidgetIcon::Battery { percent, power }),
        symbol: String::new(),
        label: percent.to_string(),
        tooltip: format!("Battery: {percent}%{note}"),
        tint: None,
    }
}

/// The speaker for `percent` (0 to 100), or the muted one; the tooltip has the percentage.
pub fn volume_state(percent: u8, muted: bool) -> WidgetState {
    let silent = muted || percent == 0;
    let level = if silent {
        0
    } else if percent < 34 {
        1
    } else if percent < 67 {
        2
    } else {
        3
    };
    let tooltip = if muted {
        "Speakers: muted".to_string()
    } else {
        format!("Speakers: {percent}%")
    };
    WidgetState {
        icon: Some(WidgetIcon::Volume {
            level,
            muted: silent,
        }),
        symbol: String::new(),
        label: String::new(),
        tooltip,
        tint: None,
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

/// Signal quality in percent from an RSSI in dBm, linear from -100 dBm (0%) to -50 dBm (100%) as
/// Windows defines its own quality value.
pub fn quality_from_rssi(dbm: i32) -> u8 {
    ((dbm + 100) * 2).clamp(0, 100) as u8
}

/// The Wi-Fi tooltip; the network name is left out when it is not known.
pub fn wifi_tooltip(name: Option<&str>, quality: u8) -> String {
    match name.filter(|n| !n.is_empty()) {
        Some(name) => format!("Wi-Fi: {name}, signal {quality}%"),
        None => format!("Wi-Fi: signal {quality}%"),
    }
}

/// The network glyph: signal bars for Wi-Fi, a plug for a cable, a red crossed globe when offline.
pub fn network_state(network: &Network) -> WidgetState {
    let mut tint = None;
    let (code, tooltip) = match network {
        Network::Wifi { name, quality } => {
            let code = match quality {
                0..=24 => 0xE872,
                25..=49 => 0xE873,
                50..=74 => 0xE874,
                _ => 0xE701,
            };
            (code, wifi_tooltip(Some(name), *quality))
        }
        Network::Ethernet => (0xE839, "Network: wired".to_string()),
        Network::Offline => {
            tint = Some([0xFF, 0x99, 0xA4]);
            (0xF384, "Network: not connected".to_string())
        }
    };
    WidgetState {
        icon: None,
        symbol: glyph(code),
        label: String::new(),
        tooltip,
        tint,
    }
}

/// The input language indicator: the language code in capitals, two letters such as `EN` (three
/// for a language that has no two letter code).
pub fn language_state(iso_code: &str) -> WidgetState {
    let code: String = iso_code
        .chars()
        .take(3)
        .flat_map(char::to_uppercase)
        .collect();
    WidgetState {
        icon: None,
        symbol: String::new(),
        tooltip: format!("Input language: {code}"),
        label: code,
        tint: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_state_carries_the_level_and_power() {
        assert_eq!(
            battery_state(60, Power::Charging).icon,
            Some(WidgetIcon::Battery {
                percent: 60,
                power: Power::Charging
            })
        );
        assert!(battery_state(60, Power::Battery).symbol.is_empty());
    }

    #[test]
    fn battery_tooltip_says_how_it_is_powered() {
        assert_eq!(
            battery_state(80, Power::Charging).tooltip,
            "Battery: 80%, charging"
        );
        assert_eq!(
            battery_state(100, Power::Plugged).tooltip,
            "Battery: 100%, plugged in"
        );
        assert_eq!(battery_state(80, Power::Battery).tooltip, "Battery: 80%");
        assert_eq!(battery_state(80, Power::Battery).label, "80");
    }

    #[test]
    fn volume_icon_follows_the_level_and_mute() {
        let icon = |percent, muted| volume_state(percent, muted).icon;
        let volume = |level, muted| Some(WidgetIcon::Volume { level, muted });
        assert_eq!(icon(0, false), volume(0, true));
        assert_eq!(icon(50, true), volume(0, true));
        assert_eq!(icon(20, false), volume(1, false));
        assert_eq!(icon(50, false), volume(2, false));
        assert_eq!(icon(90, false), volume(3, false));
        assert_eq!(volume_state(50, true).label, "");
        assert_eq!(volume_state(50, false).label, "");
        assert_eq!(volume_state(50, false).tooltip, "Speakers: 50%");
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
        assert_eq!(network_state(&Network::Offline).symbol, "\u{F384}");
        assert_eq!(
            network_state(&Network::Offline).tint,
            Some([0xFF, 0x99, 0xA4])
        );
    }

    #[test]
    fn rssi_maps_linearly_to_quality() {
        assert_eq!(quality_from_rssi(-100), 0);
        assert_eq!(quality_from_rssi(-75), 50);
        assert_eq!(quality_from_rssi(-50), 100);
        assert_eq!(quality_from_rssi(-30), 100);
        assert_eq!(quality_from_rssi(-120), 0);
    }

    #[test]
    fn the_wifi_tooltip_leaves_out_an_unknown_name() {
        assert_eq!(wifi_tooltip(Some("home"), 60), "Wi-Fi: home, signal 60%");
        assert_eq!(wifi_tooltip(None, 60), "Wi-Fi: signal 60%");
        assert_eq!(wifi_tooltip(Some(""), 60), "Wi-Fi: signal 60%");
    }

    #[test]
    fn language_is_two_capital_letters() {
        let state = language_state("en");
        assert_eq!(state.label, "EN");
        assert_eq!(state.tooltip, "Input language: EN");
        assert!(!state.has_art());
        assert_eq!(language_state("ru").label, "RU");
        assert_eq!(language_state("fil").label, "FIL");
    }

    #[test]
    fn sizes_grow_with_the_label_and_shrink_in_the_stacked_and_compact_layouts() {
        let state = WidgetState {
            label: "100%".to_string(),
            ..volume_state(80, false)
        };
        let labeled = sizes(&state, 26.0);
        assert_eq!(labeled.side, 2.0 + 26.0 + 3.0 + 28.0 + 2.0);
        assert_eq!(labeled.stacked, 2.0 + 28.0 + 2.0);
        assert_eq!(labeled.compact, 2.0 + 26.0 + 2.0);
        assert!(labeled.compact < labeled.stacked && labeled.stacked < labeled.side);
        let language = sizes(&language_state("en"), 20.0);
        assert_eq!(language.side, language.compact);
        assert_eq!(language.side, 2.0 + 22.0 + 2.0);
    }

    #[test]
    fn the_volume_bars_are_larger_than_glyphs_and_the_battery_as_tall_as_the_signal_ink() {
        let battery = battery_state(80, Power::Battery);
        assert_eq!(battery.icon_size(), 18.0);
        assert_eq!(volume_state(80, false).icon_size(), 26.0);
        assert_eq!(language_state("en").icon_size(), 20.0);
        assert!(battery.label_on_icon());
        assert!(!volume_state(80, false).label_on_icon());
        assert_eq!(battery.icon_aspect(), 1.75);
        let battery_sizes = sizes(&battery, 26.0);
        assert_eq!(battery_sizes.side, 2.0 + 31.5 + 2.0);
        assert_eq!(battery_sizes.stacked, battery_sizes.side);
        assert_eq!(battery_sizes.compact, battery_sizes.side);
    }

    #[test]
    fn a_widget_without_a_label_is_icon_sized_in_every_layout() {
        let network = sizes(
            &network_state(&Network::Wifi {
                name: "home".to_string(),
                quality: 100,
            }),
            0.0,
        );
        assert_eq!([network.side, network.stacked, network.compact], [24.0; 3]);
    }
}
