use crate::scene::Rect;
use crate::Edge;

const CLOCK_LONG: f32 = 80.0;
const CLOCK_SHORT: f32 = 30.0;
const BUTTON_MAX_LONG: f32 = 180.0;
const BUTTON_VERTICAL_LONG: f32 = 30.0;
const PAD: f32 = 4.0;
const GAP: f32 = 2.0;
/// Side of one notification area cell, and of the icon in it.
const TRAY_CELL: f32 = 24.0;
const TRAY_ICON: f32 = 16.0;

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
    layout_from(edge, width, height, scale, count, 0.0, 0.0)
}

/// Like `layout_buttons`, but with `start_size` (pixels at 100% scale) the first rect is a square
/// for button art, as large as that but never thicker than the bar, centered across the bar. The
/// other buttons share what is left. `reserved` pixels at the end of the bar, before the clock,
/// are kept free (the notification area).
pub fn layout_rows(
    edge: Edge,
    width: f32,
    height: f32,
    scale: f32,
    count: usize,
    start_size: Option<f32>,
    reserved: f32,
) -> Vec<Rect> {
    let Some(start_size) = start_size.filter(|_| count > 0) else {
        return layout_from(edge, width, height, scale, count, 0.0, reserved);
    };
    let pad = PAD * scale;
    let thickness = if edge.is_horizontal() { height } else { width };
    let side = (thickness - 2.0 * pad).clamp(0.0, start_size * scale);
    let across = (thickness - side) / 2.0;
    let (left, top) = if edge.is_horizontal() {
        (pad, across)
    } else {
        (across, pad)
    };
    let first = Rect {
        left,
        top,
        right: left + side,
        bottom: top + side,
    };
    let mut rects = vec![first];
    rects.extend(layout_from(
        edge,
        width,
        height,
        scale,
        count - 1,
        side + GAP * scale,
        reserved,
    ));
    rects
}

/// Buttons laid out after `lead` pixels reserved at the start of the bar.
fn layout_from(
    edge: Edge,
    width: f32,
    height: f32,
    scale: f32,
    count: usize,
    lead: f32,
    reserved: f32,
) -> Vec<Rect> {
    if count == 0 {
        return Vec::new();
    }
    let clock = clock_rect(edge, width, height, scale);
    let pad = PAD * scale;
    let gap = GAP * scale;
    let n = count as f32;
    if edge.is_horizontal() {
        let available = (clock.left - 2.0 * pad - lead - reserved).max(0.0);
        let each = (BUTTON_MAX_LONG * scale)
            .min((available - gap * (n - 1.0)) / n)
            .max(0.0);
        (0..count)
            .map(|i| {
                let left = pad + lead + i as f32 * (each + gap);
                Rect {
                    left,
                    top: pad,
                    right: left + each,
                    bottom: (height - pad).max(pad),
                }
            })
            .collect()
    } else {
        let available = (clock.top - 2.0 * pad - lead - reserved).max(0.0);
        let each = (BUTTON_VERTICAL_LONG * scale)
            .min((available - gap * (n - 1.0)) / n)
            .max(0.0);
        (0..count)
            .map(|i| {
                let top = pad + lead + i as f32 * (each + gap);
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

/// The rects of a bar: one per button and one per notification area icon.
#[derive(Debug, Clone, PartialEq)]
pub struct BarLayout {
    pub rows: Vec<Rect>,
    pub tray: Vec<Rect>,
}

/// Thickness of a widget on a side bar.
const WIDGET_THICK: f32 = 28.0;

/// Lays out `buttons` rows and `tray` icon cells. The cells sit at the end of the bar before the
/// clock: one row on top and bottom bars, a grid on side bars. Cells that do not fit are left out.
pub fn bar_layout(
    edge: Edge,
    width: f32,
    height: f32,
    scale: f32,
    buttons: usize,
    start_size: Option<f32>,
    tray: usize,
) -> BarLayout {
    bar_layout_with_widgets(edge, width, height, scale, buttons, start_size, tray, &[])
}

/// Like `bar_layout`, with bar widgets of the given lengths (pixels at 100% scale, along the bar;
/// ignored on side bars, where each takes a row). They sit just before the tray cells, in order,
/// and their rects follow the tray cells in `tray`.
#[allow(clippy::too_many_arguments)]
pub fn bar_layout_with_widgets(
    edge: Edge,
    width: f32,
    height: f32,
    scale: f32,
    buttons: usize,
    start_size: Option<f32>,
    tray: usize,
    widgets: &[f32],
) -> BarLayout {
    let (mut cells, extent) = tray_cells(edge, width, height, scale, tray);
    let clock = clock_rect(edge, width, height, scale);
    let pad = PAD * scale;
    let gap = GAP * scale;
    let count = widgets.len() as f32;
    let mut reserved = extent;
    if !widgets.is_empty() {
        if edge.is_horizontal() {
            let total: f32 =
                widgets.iter().map(|len| len * scale).sum::<f32>() + gap * (count - 1.0);
            let mut left = (clock.left - extent - pad - total).max(0.0);
            let top = pad;
            let bottom = (height - pad).max(pad);
            for len in widgets {
                let right = left + len * scale;
                cells.push(Rect {
                    left,
                    top,
                    right,
                    bottom,
                });
                left = right + gap;
            }
            reserved += total + pad;
        } else {
            let each = WIDGET_THICK * scale;
            let total = each * count + gap * (count - 1.0);
            let mut top = (clock.top - extent - pad - total).max(0.0);
            for _ in widgets {
                cells.push(Rect {
                    left: pad,
                    top,
                    right: (width - pad).max(pad),
                    bottom: top + each,
                });
                top += each + gap;
            }
            reserved += total + pad;
        }
    }
    BarLayout {
        rows: layout_rows(edge, width, height, scale, buttons, start_size, reserved),
        tray: cells,
    }
}

fn tray_cells(edge: Edge, width: f32, height: f32, scale: f32, count: usize) -> (Vec<Rect>, f32) {
    if count == 0 {
        return (Vec::new(), 0.0);
    }
    let clock = clock_rect(edge, width, height, scale);
    let pad = PAD * scale;
    let cell = TRAY_CELL * scale;
    if edge.is_horizontal() {
        let room = ((clock.left - 2.0 * pad) / cell).floor().max(0.0) as usize;
        let shown = count.min(room);
        let right = clock.left - pad;
        let top = ((height - cell) / 2.0).max(0.0);
        let cells = (0..shown)
            .map(|i| {
                let left = right - (shown - i) as f32 * cell;
                Rect {
                    left,
                    top,
                    right: left + cell,
                    bottom: top + cell,
                }
            })
            .collect();
        (cells, shown as f32 * cell + pad)
    } else {
        let columns = (((width - 2.0 * pad) / cell).floor() as usize).max(1);
        let rows = count.div_ceil(columns);
        let bottom = clock.top - pad;
        let room = ((clock.top - 2.0 * pad) / cell).floor().max(0.0) as usize;
        let rows = rows.min(room);
        let shown = count.min(rows * columns);
        let first = (width - columns.min(shown) as f32 * cell) / 2.0;
        let cells = (0..shown)
            .map(|i| {
                let (row, col) = (i / columns, i % columns);
                let left = first + col as f32 * cell;
                let top = bottom - (rows - row) as f32 * cell;
                Rect {
                    left,
                    top,
                    right: left + cell,
                    bottom: top + cell,
                }
            })
            .collect();
        (cells, rows as f32 * cell + pad)
    }
}

/// The icon square inside a notification area cell.
pub fn tray_icon_rect(cell: Rect, scale: f32) -> Rect {
    let size = TRAY_ICON * scale;
    let left = (cell.left + cell.right - size) / 2.0;
    let top = (cell.top + cell.bottom - size) / 2.0;
    Rect {
        left,
        top,
        right: left + size,
        bottom: top + size,
    }
}

const STRIPE_SIDE: f32 = 15.0;
const STRIPE_THIN: f32 = 6.0;
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

/// Splits a button into a marker stripe on the side facing the screen edge the bar sits on, and
/// the rest. Stripes on side bars are wide, on top and bottom bars thin.
pub fn stripe_split(edge: Edge, button: Rect, scale: f32) -> (Rect, Rect) {
    let wide = STRIPE_SIDE * scale;
    let thin = STRIPE_THIN * scale;
    match edge {
        Edge::Left => (
            Rect {
                right: button.left + wide,
                ..button
            },
            Rect {
                left: button.left + wide,
                ..button
            },
        ),
        Edge::Right => (
            Rect {
                left: button.right - wide,
                ..button
            },
            Rect {
                right: button.right - wide,
                ..button
            },
        ),
        Edge::Top => (
            Rect {
                bottom: button.top + thin,
                ..button
            },
            Rect {
                top: button.top + thin,
                ..button
            },
        ),
        Edge::Bottom => (
            Rect {
                top: button.bottom - thin,
                ..button
            },
            Rect {
                bottom: button.bottom - thin,
                ..button
            },
        ),
    }
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
    fn a_square_first_button_pushes_the_rest_along() {
        let plain = layout_buttons(Edge::Bottom, 1920.0, 40.0, 1.0, 3);
        let rows = layout_rows(Edge::Bottom, 1920.0, 40.0, 1.0, 4, Some(54.0), 0.0);
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].right - rows[0].left, rows[0].bottom - rows[0].top);
        assert_eq!(rows[0].bottom - rows[0].top, 32.0);
        assert!(rows[1].left >= rows[0].right);
        assert!(rows[1].left > plain[0].left);
        assert_eq!(
            layout_rows(Edge::Bottom, 1920.0, 40.0, 1.0, 3, None, 0.0),
            plain
        );
        let side = layout_rows(Edge::Left, 40.0, 600.0, 1.0, 2, Some(54.0), 0.0);
        assert_eq!(side[0].right - side[0].left, 32.0);
        assert!(side[1].top >= side[0].bottom);
        assert!(layout_rows(Edge::Left, 40.0, 600.0, 1.0, 0, Some(54.0), 0.0).is_empty());
    }

    #[test]
    fn the_square_first_button_stops_growing_and_is_centered_on_a_thick_bar() {
        let rows = layout_rows(Edge::Right, 200.0, 1080.0, 1.0, 2, Some(54.0), 0.0);
        assert_eq!(rows[0].right - rows[0].left, 54.0);
        assert_eq!(rows[0].bottom - rows[0].top, 54.0);
        assert_eq!(rows[0].left, 73.0);
        assert!(rows[1].top >= rows[0].bottom);
        let scaled = layout_rows(Edge::Bottom, 1920.0, 200.0, 2.0, 2, Some(54.0), 0.0);
        assert_eq!(scaled[0].bottom - scaled[0].top, 108.0);
        assert_eq!(scaled[0].top, 46.0);
    }

    #[test]
    fn tray_cells_sit_before_the_clock_and_push_the_buttons_back() {
        let plain = layout_buttons(Edge::Bottom, 1920.0, 40.0, 1.0, 3);
        let layout = bar_layout(Edge::Bottom, 1920.0, 40.0, 1.0, 3, None, 4);
        let clock = clock_rect(Edge::Bottom, 1920.0, 40.0, 1.0);
        assert_eq!(layout.tray.len(), 4);
        assert!(layout.tray[3].right <= clock.left);
        assert!(layout.tray[0].left < layout.tray[1].left);
        assert_eq!(layout.tray[0].top, 8.0);
        assert!(layout.rows[2].right <= layout.tray[0].left);
        let none = bar_layout(Edge::Bottom, 1920.0, 40.0, 1.0, 3, None, 0);
        assert_eq!(none.rows, plain);
        assert!(none.tray.is_empty());
    }

    #[test]
    fn bar_widgets_sit_before_the_tray_cells_in_order() {
        let layout =
            bar_layout_with_widgets(Edge::Bottom, 1920.0, 40.0, 1.0, 3, None, 2, &[50.0, 30.0]);
        assert_eq!(layout.tray.len(), 4);
        let (first, second) = (layout.tray[2], layout.tray[3]);
        assert_eq!(first.right - first.left, 50.0);
        assert_eq!(second.right - second.left, 30.0);
        assert!(first.right <= second.left);
        assert!(second.right <= layout.tray[0].left);
        assert!(layout.rows[2].right <= first.left);
    }

    #[test]
    fn bar_widgets_stack_above_the_tray_on_a_side_bar() {
        let layout =
            bar_layout_with_widgets(Edge::Right, 200.0, 1080.0, 1.0, 2, None, 3, &[50.0, 50.0]);
        assert_eq!(layout.tray.len(), 5);
        let (first, second) = (layout.tray[3], layout.tray[4]);
        assert_eq!(first.bottom - first.top, 28.0);
        assert!(first.bottom <= second.top);
        assert!(second.bottom <= layout.tray[0].top);
        assert!(layout.rows[1].bottom <= first.top);
    }

    #[test]
    fn tray_cells_form_a_grid_above_the_clock_on_a_side_bar() {
        let layout = bar_layout(Edge::Right, 200.0, 1080.0, 1.0, 2, None, 10);
        let clock = clock_rect(Edge::Right, 200.0, 1080.0, 1.0);
        // (200 - 8) / 24 = 8 columns, so 10 icons take two rows.
        assert_eq!(layout.tray.len(), 10);
        assert_eq!(layout.tray[0].top, layout.tray[7].top);
        assert!(layout.tray[8].top > layout.tray[0].top);
        assert!(layout.tray[9].bottom <= clock.top);
        assert!(layout.rows[1].bottom <= layout.tray[0].top);
    }

    #[test]
    fn tray_cells_that_do_not_fit_are_left_out() {
        let layout = bar_layout(Edge::Bottom, 200.0, 40.0, 1.0, 0, None, 50);
        let clock = clock_rect(Edge::Bottom, 200.0, 40.0, 1.0);
        assert!(layout.tray.len() < 50);
        assert!(layout
            .tray
            .iter()
            .all(|c| c.left >= 0.0 && c.right <= clock.left));
    }

    #[test]
    fn the_tray_icon_is_centered_in_its_cell() {
        let cell = Rect {
            left: 10.0,
            top: 0.0,
            right: 34.0,
            bottom: 24.0,
        };
        let icon = tray_icon_rect(cell, 1.0);
        assert_eq!(
            (icon.left, icon.top, icon.right, icon.bottom),
            (14.0, 4.0, 30.0, 20.0)
        );
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
    fn stripe_sits_on_the_outer_side() {
        let button = Rect {
            left: 4.0,
            top: 10.0,
            right: 196.0,
            bottom: 40.0,
        };
        let (right_stripe, right_rest) = stripe_split(Edge::Right, button, 1.0);
        assert_eq!(right_stripe.right, button.right);
        assert_eq!(right_stripe.right - right_stripe.left, 15.0);
        assert_eq!(right_rest.right, right_stripe.left);
        let (left_stripe, left_rest) = stripe_split(Edge::Left, button, 2.0);
        assert_eq!(left_stripe.left, button.left);
        assert_eq!(left_rest.left, left_stripe.right);
        let (bottom_stripe, _) = stripe_split(Edge::Bottom, button, 1.0);
        assert_eq!(bottom_stripe.bottom, button.bottom);
    }

    #[test]
    fn no_tasks_no_buttons() {
        assert!(layout_buttons(Edge::Bottom, 1920.0, 40.0, 1.0, 0).is_empty());
    }
}
