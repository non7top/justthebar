use std::sync::Arc;

use crate::buttons::{clock_rect, icon_rect, layout_buttons, title_left};
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

/// What one task button shows.
#[derive(Debug, Clone, PartialEq)]
pub struct ButtonView {
    pub title: String,
    pub active: bool,
    pub hovered: bool,
    pub icon: Option<Arc<Icon>>,
}

/// An icon to draw into `rect`.
#[derive(Debug, Clone, PartialEq)]
pub struct IconDraw {
    pub rect: Rect,
    pub icon: Arc<Icon>,
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
        fills.push(Fill {
            rect,
            color: match (button.active, button.hovered) {
                (true, hovered) => Color {
                    r: 0.25,
                    g: 0.55,
                    b: 1.0,
                    a: if hovered { 0.90 } else { 0.75 },
                },
                (false, true) => Color { a: 0.22, ..white },
                (false, false) => Color { a: 0.10, ..white },
            },
        });
        if let Some(icon) = &button.icon {
            icons.push(IconDraw {
                rect: icon_rect(rect, scale),
                icon: Arc::clone(icon),
            });
        }
        let left = title_left(rect, scale).min(rect.right);
        labels.push(Label {
            rect: Rect {
                left,
                right: (rect.right - 6.0 * scale).max(left),
                ..rect
            },
            text: button.title.clone(),
            color: white,
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
    fn each_button_gets_a_fill_and_a_label() {
        let bg = Color::rgb(0.0, 0.0, 0.0);
        let buttons = [
            ButtonView {
                title: "one".into(),
                active: false,
                hovered: false,
                icon: None,
            },
            ButtonView {
                title: "two".into(),
                active: true,
                hovered: true,
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
            icon: None,
        };
        let calm = build_scene(Edge::Bottom, 1920.0, 40.0, 1.0, bg, (12, 0), &[plain]);
        assert!(scene.fills[1].color.a > calm.fills[0].color.a);
        assert!(scene.labels[1].leading);
        assert!(!scene.labels[0].leading);
    }
}
