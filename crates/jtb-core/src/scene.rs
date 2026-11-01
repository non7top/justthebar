use std::sync::Arc;

use crate::buttons::{clock_rect, icon_rect, layout_buttons, stripe_split, title_left};
use crate::icon::Icon;
use crate::Edge;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn rgb(r: f32, g: f32, b: f32) -> Color {
        Color { r, g, b, a: 1.0 }
    }

    /// Parses `#rrggbb`.
    pub fn from_hex(text: &str) -> Result<Color, String> {
        let digits = text
            .strip_prefix('#')
            .filter(|d| d.len() == 6 && d.bytes().all(|b| b.is_ascii_hexdigit()))
            .ok_or_else(|| format!("color must look like #rrggbb, got \"{text}\""))?;
        let channel = |i: usize| {
            let value = u8::from_str_radix(&digits[i..i + 2], 16).unwrap_or(0);
            f32::from(value) / 255.0
        };
        Ok(Color::rgb(channel(0), channel(2), channel(4)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    pub rect: Rect,
    pub text: String,
    pub color: Color,
    /// Left-aligned (task buttons) instead of centered (clock).
    pub leading: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fill {
    pub rect: Rect,
    pub color: Color,
}

/// The part a button plays in a drag in progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drag {
    None,
    /// The button being dragged; drawn faint.
    Source,
    /// A drop here lands before this button.
    Before,
    /// A drop here lands after this button.
    After,
}

impl Drag {
    /// What the `index`th button shows while button `from` is dragged over button `over`.
    pub fn of(index: usize, from: usize, over: Option<usize>) -> Drag {
        if index == from {
            Drag::Source
        } else if over == Some(index) {
            if from < index {
                Drag::After
            } else {
                Drag::Before
            }
        } else {
            Drag::None
        }
    }
}

/// What one task button shows.
#[derive(Debug, Clone, PartialEq)]
pub struct ButtonView {
    pub title: String,
    pub active: bool,
    pub hovered: bool,
    /// Folded down to the taskbar; drawn dimmed.
    pub minimized: bool,
    /// Asking for attention; drawn in the flash color.
    pub flashing: bool,
    /// The title changed while the window was not active; drawn with a side stripe.
    pub title_changed: bool,
    /// A window under a group header; indented on side bars.
    pub child: bool,
    /// A group or pin header.
    pub header: bool,
    pub drag: Drag,
    pub icon: Option<Arc<Icon>>,
}

/// An icon to draw into `rect`.
#[derive(Debug, Clone, PartialEq)]
pub struct IconDraw {
    pub rect: Rect,
    pub icon: Arc<Icon>,
    pub opacity: f32,
}

/// Everything a renderer needs to draw one frame, in physical pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    pub background: Color,
    pub font_px: f32,
    pub fills: Vec<Fill>,
    pub icons: Vec<IconDraw>,
    pub labels: Vec<Label>,
}

/// How faint a minimized window's icon and title are drawn.
const MINIMIZED_OPACITY: f32 = 0.5;

/// How faint the button under the mouse button is drawn while it is dragged.
const DRAGGED_OPACITY: f32 = 0.4;

/// The line that shows where a dragged button will land.
const DROP_LINE: Color = Color {
    r: 0.35,
    g: 0.75,
    b: 1.0,
    a: 1.0,
};

pub fn clock_text(hour: u16, minute: u16) -> String {
    format!("{hour:02}:{minute:02}")
}

/// Lays out a bar of `width` x `height` pixels; `scale` is dpi / 96.
pub fn build_scene(
    edge: Edge,
    width: f32,
    height: f32,
    scale: f32,
    background: Color,
    (hour, minute): (u16, u16),
    buttons: &[ButtonView],
) -> Scene {
    let white = Color::rgb(1.0, 1.0, 1.0);
    let mut fills = Vec::new();
    let mut icons = Vec::new();
    let mut labels = vec![Label {
        rect: clock_rect(edge, width, height, scale),
        text: clock_text(hour, minute),
        color: white,
        leading: false,
    }];
    for (rect, button) in layout_buttons(edge, width, height, scale, buttons.len())
        .into_iter()
        .zip(buttons)
    {
        let rect = if button.child && !edge.is_horizontal() {
            Rect {
                left: rect.left + 14.0 * scale,
                ..rect
            }
        } else {
            rect
        };
        fills.push(Fill {
            rect,
            color: match (button.active, button.flashing, button.hovered) {
                (true, _, hovered) => Color {
                    r: 0.25,
                    g: 0.55,
                    b: 1.0,
                    a: if hovered { 0.90 } else { 0.75 },
                },
                (false, true, hovered) => Color {
                    r: 0.95,
                    g: 0.55,
                    b: 0.10,
                    a: if hovered { 0.95 } else { 0.80 },
                },
                (false, false, true) => Color { a: 0.22, ..white },
                (false, false, false) => Color {
                    a: if button.header { 0.17 } else { 0.10 },
                    ..white
                },
            },
        });
        let dragged = button.drag == Drag::Source;
        if dragged {
            if let Some(fill) = fills.last_mut() {
                fill.color.a *= DRAGGED_OPACITY;
            }
        }
        let drop_line = match button.drag {
            Drag::Before => Some(false),
            Drag::After => Some(true),
            _ => None,
        };
        if let Some(after) = drop_line {
            let thick = 3.0 * scale;
            fills.push(Fill {
                rect: match (edge.is_horizontal(), after) {
                    (true, false) => Rect {
                        right: rect.left + thick,
                        ..rect
                    },
                    (true, true) => Rect {
                        left: rect.right - thick,
                        ..rect
                    },
                    (false, false) => Rect {
                        bottom: rect.top + thick,
                        ..rect
                    },
                    (false, true) => Rect {
                        top: rect.bottom - thick,
                        ..rect
                    },
                },
                color: DROP_LINE,
            });
        }
        let content = if button.title_changed {
            let (stripe, rest) = stripe_split(edge, rect, scale);
            fills.push(Fill {
                rect: stripe,
                color: Color::rgb(1.0, 0.35, 0.0),
            });
            rest
        } else {
            rect
        };
        let opacity = if dragged {
            DRAGGED_OPACITY
        } else if button.minimized {
            MINIMIZED_OPACITY
        } else {
            1.0
        };
        if let Some(icon) = &button.icon {
            icons.push(IconDraw {
                rect: icon_rect(content, scale),
                icon: Arc::clone(icon),
                opacity,
            });
        }
        let left = title_left(content, scale).min(content.right);
        labels.push(Label {
            rect: Rect {
                left,
                right: (content.right - 6.0 * scale).max(left),
                ..content
            },
            text: button.title.clone(),
            color: Color {
                a: opacity,
                ..white
            },
            leading: true,
        });
    }
    Scene {
        background,
        font_px: 14.0 * scale,
        fills,
        icons,
        labels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_colors() {
        assert_eq!(Color::from_hex("#ff0000"), Ok(Color::rgb(1.0, 0.0, 0.0)));
        assert_eq!(Color::from_hex("#00FF00"), Ok(Color::rgb(0.0, 1.0, 0.0)));
        assert!(Color::from_hex("ff0000").is_err());
        assert!(Color::from_hex("#fff").is_err());
        assert!(Color::from_hex("#gg0000").is_err());
    }

    #[test]
    fn clock_is_zero_padded() {
        assert_eq!(clock_text(7, 5), "07:05");
        assert_eq!(clock_text(23, 59), "23:59");
    }

    #[test]
    fn clock_sits_at_the_far_end() {
        let bg = Color::rgb(1.0, 0.0, 0.0);
        let h = build_scene(Edge::Bottom, 1920.0, 40.0, 1.0, bg, (12, 0), &[]);
        assert_eq!(h.labels[0].rect.right, 1920.0);
        assert_eq!(h.labels[0].rect.left, 1840.0);
        let v = build_scene(Edge::Left, 40.0, 1080.0, 1.0, bg, (12, 0), &[]);
        assert_eq!(v.labels[0].rect.bottom, 1080.0);
        assert_eq!(v.labels[0].rect.top, 1050.0);
    }

    #[test]
    fn drag_marks_the_source_and_the_landing_side() {
        assert_eq!(Drag::of(1, 1, Some(3)), Drag::Source);
        assert_eq!(Drag::of(3, 1, Some(3)), Drag::After);
        assert_eq!(Drag::of(0, 2, Some(0)), Drag::Before);
        assert_eq!(Drag::of(2, 1, Some(3)), Drag::None);
        assert_eq!(Drag::of(2, 1, None), Drag::None);
    }

    #[test]
    fn a_drop_target_gets_a_line_on_its_landing_side() {
        let view = |drag| ButtonView {
            title: "t".into(),
            active: false,
            hovered: false,
            minimized: false,
            flashing: false,
            title_changed: false,
            child: false,
            header: false,
            drag,
            icon: None,
        };
        let bg = Color::rgb(0.0, 0.0, 0.0);
        let plain = build_scene(
            Edge::Bottom,
            1920.0,
            40.0,
            1.0,
            bg,
            (1, 1),
            &[view(Drag::None)],
        );
        let after = build_scene(
            Edge::Bottom,
            1920.0,
            40.0,
            1.0,
            bg,
            (1, 1),
            &[view(Drag::After)],
        );
        assert_eq!(after.fills.len(), plain.fills.len() + 1);
        let button = after.fills[0].rect;
        let line = after.fills[1].rect;
        assert_eq!(line.right, button.right);
        assert_eq!(line.right - line.left, 3.0);
        let before = build_scene(
            Edge::Left,
            240.0,
            600.0,
            1.0,
            bg,
            (1, 1),
            &[view(Drag::Before)],
        );
        assert_eq!(before.fills[1].rect.top, before.fills[0].rect.top);
        assert_eq!(before.fills[1].rect.bottom - before.fills[1].rect.top, 3.0);
        let source = build_scene(
            Edge::Bottom,
            1920.0,
            40.0,
            1.0,
            bg,
            (1, 1),
            &[view(Drag::Source)],
        );
        assert!(source.fills[0].color.a < plain.fills[0].color.a);
        assert_eq!(source.fills.len(), plain.fills.len());
    }

    #[test]
    fn each_button_gets_a_fill_and_a_label() {
        let bg = Color::rgb(0.0, 0.0, 0.0);
        let buttons = [
            ButtonView {
                title: "one".into(),
                active: false,
                hovered: false,
                minimized: false,
                flashing: false,
                title_changed: false,
                child: false,
                header: false,
                drag: Drag::None,
                icon: None,
            },
            ButtonView {
                title: "two".into(),
                active: true,
                hovered: true,
                minimized: false,
                flashing: false,
                title_changed: false,
                child: false,
                header: false,
                drag: Drag::None,
                icon: Some(Arc::new(Icon {
                    id: 1,
                    width: 1,
                    height: 1,
                    bgra: vec![0, 0, 0, 255],
                })),
            },
        ];
        let scene = build_scene(Edge::Bottom, 1920.0, 40.0, 1.0, bg, (12, 0), &buttons);
        assert_eq!(scene.fills.len(), 2);
        assert_eq!(scene.labels.len(), 3);
        assert!(scene.fills[1].color.a > scene.fills[0].color.a);
        assert_eq!(scene.icons.len(), 1);
        let plain = ButtonView {
            title: "two".into(),
            active: true,
            hovered: false,
            minimized: true,
            flashing: false,
            title_changed: false,
            child: false,
            header: false,
            drag: Drag::None,
            icon: None,
        };
        let calm = build_scene(Edge::Bottom, 1920.0, 40.0, 1.0, bg, (12, 0), &[plain]);
        assert!(scene.fills[1].color.a > calm.fills[0].color.a);
        assert!(calm.labels[1].color.a < scene.labels[2].color.a);
        let flash = ButtonView {
            title: "three".into(),
            active: false,
            hovered: false,
            minimized: false,
            flashing: true,
            title_changed: true,
            child: false,
            header: false,
            drag: Drag::None,
            icon: None,
        };
        let flashing = build_scene(Edge::Bottom, 1920.0, 40.0, 1.0, bg, (12, 0), &[flash]);
        assert!(flashing.fills[0].color.r > flashing.fills[0].color.b);
        assert_eq!(flashing.fills.len(), 2);
        assert!(scene.labels[1].leading);
        assert!(!scene.labels[0].leading);
    }
}
