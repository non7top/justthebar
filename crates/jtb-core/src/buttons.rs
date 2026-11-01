use crate::scene::Rect;
use crate::Edge;

const CLOCK_LONG: f32 = 80.0;
const CLOCK_SHORT: f32 = 30.0;
const BUTTON_MAX_LONG: f32 = 180.0;
const BUTTON_VERTICAL_LONG: f32 = 30.0;
const PAD: f32 = 4.0;
const GAP: f32 = 2.0;

/// The clock sits at the far end of the bar. `scale` is dpi / 96.
pub fn clock_rect(edge: Edge, width: f32, height: f32, scale: f32) -> Rect {
    if edge.is_horizontal() {
        Rect {
            left: (width - CLOCK_LONG * scale).max(0.0),
            top: 0.0,
            right: width,
            bottom: height,
        }
    } else {
        Rect {
            left: 0.0,
            top: (height - CLOCK_SHORT * scale).max(0.0),
            right: width,
            bottom: height,
        }
    }
}

/// One rect per task button, laid out from the start of the bar up to the clock. Horizontal bars
/// put buttons side by side (shrinking them to fit), vertical bars stack them.
pub fn layout_buttons(edge: Edge, width: f32, height: f32, scale: f32, count: usize) -> Vec<Rect> {
    if count == 0 {
        return Vec::new();
    }
    let clock = clock_rect(edge, width, height, scale);
    let pad = PAD * scale;
    let gap = GAP * scale;
    let n = count as f32;
    if edge.is_horizontal() {
        let available = (clock.left - 2.0 * pad).max(0.0);
        let each = (BUTTON_MAX_LONG * scale)
            .min((available - gap * (n - 1.0)) / n)
            .max(0.0);
        (0..count)
            .map(|i| {
                let left = pad + i as f32 * (each + gap);
                Rect {
                    left,
                    top: pad,
                    right: left + each,
                    bottom: (height - pad).max(pad),
                }
            })
            .collect()
    } else {
        let available = (clock.top - 2.0 * pad).max(0.0);
        let each = (BUTTON_VERTICAL_LONG * scale)
            .min((available - gap * (n - 1.0)) / n)
            .max(0.0);
        (0..count)
            .map(|i| {
                let top = pad + i as f32 * (each + gap);
                Rect {
                    left: pad,
                    top,
                    right: (width - pad).max(pad),
                    bottom: top + each,
                }
            })
            .collect()
    }
}

const ICON_SIZE: f32 = 16.0;
const ICON_INSET: f32 = 6.0;

/// The icon square inside a button, vertically centered at the button's start.
pub fn icon_rect(button: Rect, scale: f32) -> Rect {
    let size = ICON_SIZE * scale;
    let left = button.left + ICON_INSET * scale;
    let top = (button.top + button.bottom - size) / 2.0;
    Rect {
        left,
        top,
        right: left + size,
        bottom: top + size,
    }
}

/// Where a button's title starts, after the icon.
pub fn title_left(button: Rect, scale: f32) -> f32 {
    button.left + (2.0 * ICON_INSET + ICON_SIZE) * scale
}

/// Index of the rect containing the point.
pub fn hit_test(rects: &[Rect], x: f32, y: f32) -> Option<usize> {
    rects
        .iter()
        .position(|r| x >= r.left && x < r.right && y >= r.top && y < r.bottom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_buttons_run_left_to_right_before_the_clock() {
        let rects = layout_buttons(Edge::Bottom, 1920.0, 40.0, 1.0, 3);
        assert_eq!(rects.len(), 3);
        assert!(rects[0].left < rects[1].left);
        assert!(rects[2].right <= clock_rect(Edge::Bottom, 1920.0, 40.0, 1.0).left);
        assert_eq!(rects[0].right - rects[0].left, 180.0);
    }

    #[test]
    fn many_buttons_shrink_to_fit() {
        let rects = layout_buttons(Edge::Top, 800.0, 40.0, 1.0, 20);
        let clock = clock_rect(Edge::Top, 800.0, 40.0, 1.0);
        assert!(rects.last().unwrap().right <= clock.left);
        assert!(rects[0].right - rects[0].left < 180.0);
    }

    #[test]
    fn vertical_buttons_stack_above_the_clock() {
        let rects = layout_buttons(Edge::Left, 40.0, 1080.0, 1.0, 4);
        assert!(rects[0].top < rects[1].top);
        assert!(rects[3].bottom <= clock_rect(Edge::Left, 40.0, 1080.0, 1.0).top);
        assert_eq!(rects[0].left, 4.0);
    }

    #[test]
    fn hit_test_finds_the_button() {
        let rects = layout_buttons(Edge::Bottom, 1920.0, 40.0, 1.0, 2);
        assert_eq!(hit_test(&rects, 10.0, 20.0), Some(0));
        assert_eq!(hit_test(&rects, rects[1].left + 1.0, 20.0), Some(1));
        assert_eq!(hit_test(&rects, 1900.0, 20.0), None);
    }

    #[test]
    fn icon_is_centered_and_title_starts_after_it() {
        let button = Rect {
            left: 10.0,
            top: 4.0,
            right: 190.0,
            bottom: 36.0,
        };
        let icon = icon_rect(button, 1.0);
        assert_eq!(icon.right - icon.left, 16.0);
        assert_eq!(icon.top - button.top, button.bottom - icon.bottom);
        assert!(title_left(button, 1.0) > icon.right);
    }

    #[test]
    fn no_tasks_no_buttons() {
        assert!(layout_buttons(Edge::Bottom, 1920.0, 40.0, 1.0, 0).is_empty());
    }
}
