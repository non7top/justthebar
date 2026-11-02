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

/// What a click asks the app to do, once the widget list is no longer borrowed.
pub enum Action {
    /// Open a URI such as `ms-settings:sound`.
    Open(&'static str),
}

pub trait Widget {
    /// The name used in `jtb.json`.
    fn name(&self) -> &'static str;
    /// What the widget shows now; none hides it (a battery widget on a desktop).
    fn state(&mut self) -> Option<WidgetState>;
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
        Some(Action::Open(QUICK_SETTINGS))
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

    fn click(&mut self) -> Option<Action> {
        Some(Action::Open("ms-settings:regionlanguage"))
    }
}

const QUICK_SETTINGS: &str = "ms-actioncenter:controlcenter/&showFooter=true";

struct Volume;

impl Widget for Volume {
    fn name(&self) -> &'static str {
        "volume"
    }

    fn state(&mut self) -> Option<WidgetState> {
        system::volume().map(|(percent, muted)| widget::volume_state(percent, muted))
    }

    fn click(&mut self) -> Option<Action> {
        Some(Action::Open(QUICK_SETTINGS))
    }
}

struct Network;

impl Widget for Network {
    fn name(&self) -> &'static str {
        "network"
    }

    fn state(&mut self) -> Option<WidgetState> {
        Some(widget::network_state(&system::network()))
    }

    fn click(&mut self) -> Option<Action> {
        Some(Action::Open(QUICK_SETTINGS))
    }
}

fn create(name: &str) -> Option<Box<dyn Widget>> {
    match name {
        "battery" => Some(Box::new(Battery)),
        "volume" => Some(Box::new(Volume)),
        "network" => Some(Box::new(Network)),
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

fn draw(font: &str, text: &str) -> Option<Arc<Icon>> {
    if text.is_empty() {
        return None;
    }
    let bgra = glyph::render(font, text, BITMAP_SIZE)?;
    Some(Arc::new(Icon {
        id: crate::next_icon_id(),
        width: BITMAP_SIZE,
        height: BITMAP_SIZE,
        bgra,
    }))
}

fn show(state: WidgetState) -> Shown {
    Shown {
        symbol: draw(SYMBOL_FONT, &state.symbol),
        label: draw(TEXT_FONT, &state.label),
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
    CELLS.with_borrow_mut(|cells| {
        let mut changed = false;
        for cell in cells {
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
    // Opening a URI can pump messages, which re-enter `refresh`, so the borrow must be over.
    let action =
        CELLS.with_borrow_mut(|cells| nth_shown(cells, at).and_then(|cell| cell.widget.click()));
    if let Some(Action::Open(uri)) = action {
        crate::open_uri(uri);
    }
}
