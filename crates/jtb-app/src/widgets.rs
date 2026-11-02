//! The built-in bar widgets. Each one reads some system state and shows it either on the bar itself
//! or as a small cell in the notification area; `jtb.json` picks which ones and where.
use std::cell::RefCell;
use std::sync::Arc;

use jtb_core::config::{WidgetConfig, WidgetPlace};
use jtb_core::icon::Icon;
use jtb_core::scene::WidgetView;
use jtb_core::widget::{self, WidgetState};
use jtb_win::{glyph, system};

const SYMBOL_FONT: &str = "Segoe MDL2 Assets";
const TEXT_FONT: &str = "Segoe UI Semibold";
const BITMAP_SIZE: u32 = 32;
const WHITE: [u8; 3] = [255, 255, 255];

/// What a click asks the app to do, once the widget list is no longer borrowed.
pub enum Action {
    /// The quick settings panel (Win+A).
    QuickSettings,
    /// Switch to the next input language.
    NextLanguage,
}

pub trait Widget {
    /// The name used in `jtb.json`.
    fn name(&self) -> &'static str;
    /// What the widget shows now; none hides it (a battery widget on a desktop).
    fn state(&mut self) -> Option<WidgetState>;
    /// Whether the widget is cheap and changes fast enough to be read every 100 ms.
    fn quick(&self) -> bool {
        false
    }
    /// A left click on the widget.
    fn click(&mut self) -> Option<Action> {
        None
    }
}

struct Battery;

impl Widget for Battery {
    fn name(&self) -> &'static str {
        "battery"
    }

    fn state(&mut self) -> Option<WidgetState> {
        system::battery().map(|(percent, charging)| widget::battery_state(percent, charging))
    }

    fn click(&mut self) -> Option<Action> {
        Some(Action::QuickSettings)
    }
}

struct Language;

impl Widget for Language {
    fn name(&self) -> &'static str {
        "language"
    }

    fn state(&mut self) -> Option<WidgetState> {
        system::input_language().map(|code| widget::language_state(&code))
    }

    fn quick(&self) -> bool {
        true
    }

    fn click(&mut self) -> Option<Action> {
        Some(Action::NextLanguage)
    }
}

#[derive(Default)]
struct Volume {
    last_error: Option<String>,
}

impl Widget for Volume {
    fn name(&self) -> &'static str {
        "volume"
    }

    fn state(&mut self) -> Option<WidgetState> {
        match system::volume() {
            Ok((percent, muted)) => {
                self.last_error = None;
                Some(widget::volume_state(percent, muted))
            }
            Err(error) => {
                if self.last_error.as_ref() != Some(&error) {
                    crate::log(&format!("widgets: volume cannot be read: {error}"));
                    self.last_error = Some(error);
                }
                None
            }
        }
    }

    fn click(&mut self) -> Option<Action> {
        Some(Action::QuickSettings)
    }
}

#[derive(Default)]
struct Network {
    was_offline: bool,
}

impl Widget for Network {
    fn name(&self) -> &'static str {
        "network"
    }

    fn state(&mut self) -> Option<WidgetState> {
        let network = system::network();
        let state = widget::network_state(&network);
        let offline = network == widget::Network::Offline;
        if offline && !self.was_offline {
            crate::log(&format!(
                "widgets: no connection found among {}",
                system::network_report()
            ));
        }
        self.was_offline = offline;
        Some(state)
    }

    fn click(&mut self) -> Option<Action> {
        Some(Action::QuickSettings)
    }
}

fn create(name: &str) -> Option<Box<dyn Widget>> {
    match name {
        "battery" => Some(Box::new(Battery)),
        "volume" => Some(Box::new(Volume::default())),
        "network" => Some(Box::new(Network::default())),
        "language" => Some(Box::new(Language)),
        _ => None,
    }
}

/// A widget's state with its bitmaps: the glyph, and the label as a bitmap for the tray cell.
struct Shown {
    state: WidgetState,
    symbol: Option<Arc<Icon>>,
    label: Option<Arc<Icon>>,
}

struct Cell {
    widget: Box<dyn Widget>,
    place: WidgetPlace,
    shown: Option<Shown>,
}

thread_local! {
    static CELLS: RefCell<Vec<Cell>> = const { RefCell::new(Vec::new()) };
}

fn draw(font: &str, text: &str, em: u32, tint: [u8; 3]) -> Option<Arc<Icon>> {
    if text.is_empty() {
        return None;
    }
    let bgra = glyph::render(font, text, BITMAP_SIZE, em, tint)?;
    Some(Arc::new(Icon {
        id: crate::next_icon_id(),
        width: BITMAP_SIZE,
        height: BITMAP_SIZE,
        bgra,
    }))
}

fn show(state: WidgetState) -> Shown {
    Shown {
        symbol: draw(
            SYMBOL_FONT,
            &state.symbol,
            BITMAP_SIZE,
            state.tint.unwrap_or(WHITE),
        ),
        label: draw(TEXT_FONT, &state.label, BITMAP_SIZE * 3 / 4, WHITE),
        state,
    }
}

/// Replaces the widgets with those in `config`; unknown names are logged and skipped.
pub fn rebuild(config: &[WidgetConfig]) {
    let cells = config
        .iter()
        .filter_map(|entry| match create(&entry.name) {
            Some(widget) => Some(Cell {
                widget,
                place: entry.place,
                shown: None,
            }),
            None => {
                crate::log(&format!("widgets: unknown widget {:?}", entry.name));
                None
            }
        })
        .collect();
    CELLS.set(cells);
    refresh();
}

/// Reads every widget again; true when something on screen changed.
pub fn refresh() -> bool {
    refresh_where(|_| true)
}

/// Reads only the widgets that change fast (the input language); true when something changed.
pub fn refresh_quick() -> bool {
    refresh_where(|widget| widget.quick())
}

fn refresh_where(wanted: impl Fn(&dyn Widget) -> bool) -> bool {
    CELLS.with_borrow_mut(|cells| {
        let mut changed = false;
        for cell in cells {
            if !wanted(cell.widget.as_ref()) {
                continue;
            }
            let state = cell.widget.state();
            let same = match (&state, &cell.shown) {
                (Some(new), Some(old)) => *new == old.state,
                (None, None) => true,
                _ => false,
            };
            if same {
                continue;
            }
            changed = true;
            cell.shown = state.map(show);
            crate::log(&format!(
                "widgets: {} now {:?}",
                cell.widget.name(),
                cell.shown.as_ref().map(|shown| &shown.state.tooltip)
            ));
        }
        changed
    })
}

/// The `at`th shown widget in the order of the cells: those in the notification area, then those
/// on the bar.
fn nth_shown(cells: &mut [Cell], at: usize) -> Option<&mut Cell> {
    let mut order = Vec::new();
    for place in [WidgetPlace::Tray, WidgetPlace::Bar] {
        for (i, cell) in cells.iter().enumerate() {
            if cell.place == place && cell.shown.is_some() {
                order.push(i);
            }
        }
    }
    let index = *order.get(at)?;
    cells.get_mut(index)
}

/// The icons of the widgets shown in the notification area: the glyph, or the text when the widget
/// has no glyph.
pub fn tray_icons() -> Vec<Arc<Icon>> {
    CELLS.with_borrow_mut(|cells| {
        let mut icons = Vec::new();
        for cell in cells.iter().filter(|c| c.place == WidgetPlace::Tray) {
            if let Some(shown) = &cell.shown {
                if let Some(icon) = shown.symbol.as_ref().or(shown.label.as_ref()) {
                    icons.push(Arc::clone(icon));
                }
            }
        }
        icons
    })
}

/// The widgets shown on the bar; `hovered` is the index among them.
pub fn bar_views(hovered: Option<usize>) -> Vec<WidgetView> {
    CELLS.with_borrow(|cells| {
        cells
            .iter()
            .filter(|c| c.place == WidgetPlace::Bar)
            .filter_map(|c| c.shown.as_ref())
            .enumerate()
            .map(|(at, shown)| WidgetView {
                symbol: shown.symbol.clone(),
                label: shown.state.label.clone(),
                length: widget::bar_length(&shown.state),
                hovered: hovered == Some(at),
            })
            .collect()
    })
}

/// The tooltip of the `at`th shown widget (tray ones first, then bar ones).
pub fn tooltip(at: usize) -> Option<String> {
    CELLS.with_borrow_mut(|cells| {
        nth_shown(cells, at)
            .and_then(|cell| cell.shown.as_ref().map(|shown| shown.state.tooltip.clone()))
    })
}

/// A left click on the `at`th shown widget.
pub fn click(at: usize) {
    // An action can pump messages, which re-enter `refresh`, so the borrow must be over.
    let action =
        CELLS.with_borrow_mut(|cells| nth_shown(cells, at).and_then(|cell| cell.widget.click()));
    match action {
        Some(Action::QuickSettings) => {
            if let Some(anchor) = crate::bar::flyout_anchor() {
                jtb_win::flyout::expect(anchor);
            }
            system::open_quick_settings();
        }
        Some(Action::NextLanguage) => system::next_input_language(),
        None => {}
    }
}
