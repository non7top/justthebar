//! The built-in bar widgets. Each one reads some system state and shows it as one cell next to the
//! tray icons; `jtb.json` picks which ones with `widgets`.
use std::cell::RefCell;
use std::sync::Arc;

use jtb_core::icon::Icon;
use jtb_core::widget::{self, Face, WidgetState};
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
        Some(Action::Open("ms-settings:batterysaver"))
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

fn create(name: &str) -> Option<Box<dyn Widget>> {
    match name {
        "battery" => Some(Box::new(Battery)),
        "language" => Some(Box::new(Language)),
        _ => None,
    }
}

struct Cell {
    widget: Box<dyn Widget>,
    shown: Option<(WidgetState, Arc<Icon>)>,
}

thread_local! {
    static CELLS: RefCell<Vec<Cell>> = const { RefCell::new(Vec::new()) };
}

fn draw(state: &WidgetState) -> Option<Arc<Icon>> {
    let font = match state.face {
        Face::Symbols => SYMBOL_FONT,
        Face::Text => TEXT_FONT,
    };
    let bgra = glyph::render(font, &state.text, BITMAP_SIZE)?;
    Some(Arc::new(Icon {
        id: crate::next_icon_id(),
        width: BITMAP_SIZE,
        height: BITMAP_SIZE,
        bgra,
    }))
}

/// Replaces the widgets with those named in `names`; unknown names are logged and skipped.
pub fn rebuild(names: &[String]) {
    let cells = names
        .iter()
        .filter_map(|name| match create(name) {
            Some(widget) => Some(Cell {
                widget,
                shown: None,
            }),
            None => {
                crate::log(&format!("widgets: unknown widget {name:?}"));
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
                (Some(new), Some((old, _))) => new == old,
                (None, None) => true,
                _ => false,
            };
            if same {
                continue;
            }
            changed = true;
            cell.shown = state.and_then(|state| draw(&state).map(|icon| (state, icon)));
            crate::log(&format!(
                "widgets: {} now {:?}",
                cell.widget.name(),
                cell.shown.as_ref().map(|(state, _)| &state.tooltip)
            ));
        }
        changed
    })
}

/// The icons of the widgets that have something to show, in order.
pub fn icons() -> Vec<Arc<Icon>> {
    CELLS.with_borrow(|cells| {
        cells
            .iter()
            .filter_map(|cell| cell.shown.as_ref().map(|(_, icon)| Arc::clone(icon)))
            .collect()
    })
}

fn with_shown<T>(at: usize, f: impl FnOnce(&mut Cell) -> T) -> Option<T> {
    CELLS.with_borrow_mut(|cells| {
        cells
            .iter_mut()
            .filter(|cell| cell.shown.is_some())
            .nth(at)
            .map(f)
    })
}

/// The tooltip of the `at`th shown widget.
pub fn tooltip(at: usize) -> Option<String> {
    with_shown(at, |cell| {
        cell.shown.as_ref().map(|(s, _)| s.tooltip.clone())
    })
    .flatten()
}

/// A left click on the `at`th shown widget.
pub fn click(at: usize) {
    // Opening a URI can pump messages, which re-enter `refresh`, so the borrow must be over.
    if let Some(Some(Action::Open(uri))) = with_shown(at, |cell| cell.widget.click()) {
        crate::open_uri(uri);
    }
}
