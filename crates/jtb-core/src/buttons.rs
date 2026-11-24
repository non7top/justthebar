use crate::scene::Rect;
use crate::widget::{SIDE_THICK, STACKED_THICK};
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
    clock_rect_sized(edge, width, height, scale, default_clock_extent(edge))
}

/// The clock's length along the bar (pixels at 100% scale) when it shows one short line.
pub fn default_clock_extent(edge: Edge) -> f32 {
    if edge.is_horizontal() {
        CLOCK_LONG
    } else {
        CLOCK_SHORT
    }
}

/// Like `clock_rect`, with the clock `extent` pixels (at 100% scale) long along the bar.
pub fn clock_rect_sized(edge: Edge, width: f32, height: f32, scale: f32, extent: f32) -> Rect {
    if edge.is_horizontal() {
        Rect {
            left: (width - extent * scale).max(0.0),
            top: 0.0,
            right: width,
            bottom: height,
        }
    } else {
        Rect {
            left: 0.0,
            top: (height - extent * scale).max(0.0),
            right: width,
            bottom: height,
        }
    }
}

/// One rect per task button, laid out from the start of the bar up to the clock. Horizontal bars
/// put buttons side by side (shrinking them to fit), vertical bars stack them.
pub fn layout_buttons(edge: Edge, width: f32, height: f32, scale: f32, count: usize) -> Vec<Rect> {
    layout_from(
        edge,
        width,
        height,
        scale,
        count,
        0.0,
        0.0,
        default_clock_extent(edge),
    )
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
    let clock = default_clock_extent(edge);
    layout_rows_sized(
        edge, width, height, scale, count, start_size, reserved, clock,
    )
}

/// Like `layout_rows`, with the clock `clock_extent` pixels (at 100% scale) long along the bar.
#[allow(clippy::too_many_arguments)]
pub fn layout_rows_sized(
    edge: Edge,
    width: f32,
    height: f32,
    scale: f32,
    count: usize,
    start_size: Option<f32>,
    reserved: f32,
    clock_extent: f32,
) -> Vec<Rect> {
    let Some(start_size) = start_size.filter(|_| count > 0) else {
        return layout_from(
            edge,
            width,
            height,
            scale,
            count,
            0.0,
            reserved,
            clock_extent,
        );
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
        clock_extent,
    ));
    rects
}

/// Buttons laid out after `lead` pixels reserved at the start of the bar.
#[allow(clippy::too_many_arguments)]
fn layout_from(
    edge: Edge,
    width: f32,
    height: f32,
    scale: f32,
    count: usize,
    lead: f32,
    reserved: f32,
    clock_extent: f32,
) -> Vec<Rect> {
    if count == 0 {
        return Vec::new();
    }
    let clock = clock_rect_sized(edge, width, height, scale, clock_extent);
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

/// How the widgets on a side bar are laid out: icon and label side by side, the label under the
/// icon, or the icon alone. The first that fits all of them in one row is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetTier {
    SideBySide,
    Stacked,
    GlyphOnly,
}

/// The smallest and largest factor the widgets are scaled to, icon and text together: down before
/// the layout falls back to stacking them, up so that a roomy bar is not left with gaps.
pub const MIN_FACTOR: f32 = 0.75;
pub const MAX_FACTOR: f32 = 1.5;

/// The widgets' layout: the tier, each one's length along the bar (pixels at 100% scale) and the
/// thickness of a cell on a side bar, both already multiplied by `factor`, the factor the icons
/// and text are drawn at (1.0 on top and bottom bars and in the stacked and icon layouts).
#[derive(Debug, Clone, PartialEq)]
pub struct WidgetPlan {
    pub tier: WidgetTier,
    pub lengths: Vec<f32>,
    pub thick: f32,
    pub factor: f32,
}

/// Picks the tier from each widget's `[side by side, stacked, icon only]` lengths: side by side
/// scaled to fill the bar (between `MIN_FACTOR` and `MAX_FACTOR`), else stacked, else icons only.
/// Top and bottom bars have the room for side by side.
pub fn plan_widgets(edge: Edge, width: f32, scale: f32, sizes: &[[f32; 3]]) -> WidgetPlan {
    let pick = |i: usize| sizes.iter().map(|s| s[i]).collect::<Vec<f32>>();
    if edge.is_horizontal() || sizes.is_empty() {
        return WidgetPlan {
            tier: WidgetTier::SideBySide,
            lengths: pick(0),
            thick: SIDE_THICK,
            factor: 1.0,
        };
    }
    let room = (width - 2.0 * PAD * scale).max(0.0);
    let gaps = |lengths: &[f32]| GAP * scale * (lengths.len() as f32 - 1.0);
    let total = |lengths: &[f32]| lengths.iter().map(|len| len * scale).sum::<f32>();
    let side = pick(0);
    let fit = ((room - gaps(&side)) / total(&side) * 100.0).floor() / 100.0;
    if fit >= MIN_FACTOR {
        let factor = fit.min(MAX_FACTOR);
        return WidgetPlan {
            tier: WidgetTier::SideBySide,
            lengths: side.iter().map(|len| len * factor).collect(),
            thick: SIDE_THICK * factor,
            factor,
        };
    }
    let stacked = pick(1);
    if total(&stacked) + gaps(&stacked) <= room {
        return WidgetPlan {
            tier: WidgetTier::Stacked,
            lengths: stacked,
            thick: STACKED_THICK,
            factor: 1.0,
        };
    }
    WidgetPlan {
        tier: WidgetTier::GlyphOnly,
        lengths: pick(2),
        thick: SIDE_THICK,
        factor: 1.0,
    }
}

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
    let clock = default_clock_extent(edge);
    bar_layout_with_widgets(
        edge,
        width,
        height,
        scale,
        buttons,
        start_size,
        tray,
        &[],
        clock,
    )
}

/// Like `bar_layout`, with bar widgets of the given lengths (pixels at 100% scale, along the bar),
/// laid out side by side.
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
    clock_extent: f32,
) -> BarLayout {
    let plan = WidgetPlan {
        tier: WidgetTier::SideBySide,
        lengths: widgets.to_vec(),
        thick: SIDE_THICK,
        factor: 1.0,
    };
    bar_layout_with_plan(
        edge,
        width,
        height,
        scale,
        buttons,
        start_size,
        tray,
        &plan,
        clock_extent,
    )
}

/// Like `bar_layout`, with the bar widgets laid out as `plan` says. On side bars they wrap onto
/// further rows when one row is too narrow, only a widget wider than the bar alone is shrunk, and
/// each row is stretched to the full width. They sit just before the tray cells, in order, and
/// their rects follow the tray cells in `tray`.
#[allow(clippy::too_many_arguments)]
pub fn bar_layout_with_plan(
    edge: Edge,
    width: f32,
    height: f32,
    scale: f32,
    buttons: usize,
    start_size: Option<f32>,
    tray: usize,
    plan: &WidgetPlan,
    clock_extent: f32,
) -> BarLayout {
    let widgets = plan.lengths.as_slice();
    let (mut cells, extent) = tray_cells(edge, width, height, scale, tray, clock_extent);
    let clock = clock_rect_sized(edge, width, height, scale, clock_extent);
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
            let each = plan.thick * scale;
            let room = (width - 2.0 * pad).max(0.0);
            let mut rows: Vec<Vec<f32>> = Vec::new();
            let mut used = 0.0;
            for len in widgets {
                let length = (len * scale).min(room);
                match rows.last_mut() {
                    Some(row) if used + gap + length <= room => {
                        row.push(length);
                        used += gap + length;
                    }
                    _ => {
                        rows.push(vec![length]);
                        used = length;
                    }
                }
            }
            let row_count = rows.len() as f32;
            let block = row_count * each + (row_count - 1.0) * gap;
            let first_top = (clock.top - extent - pad - block).max(0.0);
            for (i, row) in rows.iter().enumerate() {
                let top = first_top + i as f32 * (each + gap);
                let used: f32 = row.iter().sum::<f32>() + gap * (row.len() as f32 - 1.0);
                let extra = (room - used).max(0.0) / row.len() as f32;
                let mut left = pad;
                for length in row {
                    let right = left + length + extra;
                    cells.push(Rect {
                        left,
                        top,
                        right,
                        bottom: top + each,
                    });
                    left = right + gap;
                }
            }
            reserved += block + pad;
        }
    }
    BarLayout {
        rows: layout_rows_sized(
            edge,
            width,
            height,
            scale,
            buttons,
            start_size,
            reserved,
            clock_extent,
        ),
        tray: cells,
    }
}

fn tray_cells(
    edge: Edge,
    width: f32,
    height: f32,
    scale: f32,
    count: usize,
    clock_extent: f32,
) -> (Vec<Rect>, f32) {
    if count == 0 {
        return (Vec::new(), 0.0);
    }
    let clock = clock_rect_sized(edge, width, height, scale, clock_extent);
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
        let layout = bar_layout_with_widgets(
            Edge::Bottom,
            1920.0,
            40.0,
            1.0,
            3,
            None,
            2,
            &[50.0, 30.0],
            default_clock_extent(Edge::Bottom),
        );
        assert_eq!(layout.tray.len(), 4);
        let (first, second) = (layout.tray[2], layout.tray[3]);
        assert_eq!(first.right - first.left, 50.0);
        assert_eq!(second.right - second.left, 30.0);
        assert!(first.right <= second.left);
        assert!(second.right <= layout.tray[0].left);
        assert!(layout.rows[2].right <= first.left);
    }

    #[test]
    fn bar_widgets_form_one_row_above_the_tray_on_a_side_bar_when_they_fit() {
        let layout = bar_layout_with_widgets(
            Edge::Right,
            200.0,
            1080.0,
            1.0,
            2,
            None,
            3,
            &[40.0, 40.0, 40.0, 40.0],
            default_clock_extent(Edge::Right),
        );
        assert_eq!(layout.tray.len(), 7);
        let row = &layout.tray[3..];
        assert!(row.iter().all(|r| r.top == row[0].top));
        assert!(row.windows(2).all(|w| w[0].right <= w[1].left));
        assert!(row[3].right <= 200.0);
        assert_eq!(row[0].bottom - row[0].top, 28.0);
        assert!(row[0].bottom <= layout.tray[0].top);
        assert!(layout.rows[1].bottom <= row[0].top);
    }

    #[test]
    fn bar_widgets_wrap_to_a_second_row_instead_of_shrinking_on_a_narrow_side_bar() {
        let lengths = [36.0, 32.0, 60.0, 68.0];
        let layout = bar_layout_with_widgets(
            Edge::Right,
            168.0,
            1080.0,
            1.0,
            2,
            None,
            3,
            &lengths,
            default_clock_extent(Edge::Right),
        );
        let row = &layout.tray[3..];
        for (cell, length) in row.iter().zip(lengths) {
            assert!(cell.right - cell.left >= length);
            assert!(cell.right <= 168.0 - 4.0 + 0.01);
        }
        assert!((row[2].right - 164.0).abs() < 0.01);
        assert!((row[3].right - 164.0).abs() < 0.01);
        assert_eq!(row[0].top, row[2].top);
        assert!(row[3].top > row[2].top);
        assert_eq!(row[3].left, row[0].left);
        assert!(row[3].bottom <= layout.tray[0].top);
        assert!(layout.rows[1].bottom <= row[0].top);
    }

    #[test]
    fn widgets_scale_down_to_one_slim_row_before_they_stack() {
        // language, network, volume, battery on a 160 px bar: [side, stacked, icon only]
        let sizes = [
            [26.0, 26.0, 26.0],
            [24.0, 24.0, 24.0],
            [63.0, 38.0, 24.0],
            [63.0, 38.0, 24.0],
        ];
        let plan = plan_widgets(Edge::Right, 160.0, 1.0, &sizes);
        assert_eq!(plan.tier, WidgetTier::SideBySide);
        assert!(plan.factor >= MIN_FACTOR && plan.factor < 1.0);
        assert_eq!(plan.thick, SIDE_THICK * plan.factor);
        let layout = bar_layout_with_plan(
            Edge::Right,
            160.0,
            1080.0,
            1.0,
            2,
            None,
            3,
            &plan,
            default_clock_extent(Edge::Right),
        );
        let row = &layout.tray[3..];
        assert!(row.iter().all(|r| r.top == row[0].top));
        assert!(row[3].right <= 156.0 + 0.01 && row[3].right > 150.0);
        assert!((row[0].bottom - row[0].top - plan.thick).abs() < 0.001);
    }

    #[test]
    fn widgets_stack_when_even_the_smallest_scale_does_not_fit() {
        let sizes = [
            [60.0, 40.0, 24.0],
            [60.0, 40.0, 24.0],
            [60.0, 40.0, 24.0],
            [60.0, 40.0, 24.0],
        ];
        let plan = plan_widgets(Edge::Right, 190.0, 1.0, &sizes);
        assert_eq!(plan.tier, WidgetTier::Stacked);
        assert_eq!(plan.lengths, vec![40.0; 4]);
        assert_eq!(plan.thick, STACKED_THICK);
        assert_eq!(plan.factor, 1.0);
        let layout = bar_layout_with_plan(
            Edge::Right,
            190.0,
            1080.0,
            1.0,
            2,
            None,
            3,
            &plan,
            default_clock_extent(Edge::Right),
        );
        let row = &layout.tray[3..];
        assert!(row.iter().all(|r| r.top == row[0].top));
        assert_eq!(row[0].bottom - row[0].top, STACKED_THICK);
    }

    #[test]
    fn the_layout_tier_follows_the_room() {
        let sizes = [[50.0, 30.0, 24.0], [50.0, 30.0, 24.0]];
        let plan = |width| plan_widgets(Edge::Right, width, 1.0, &sizes);
        assert_eq!(plan(200.0).tier, WidgetTier::SideBySide);
        assert_eq!(plan(200.0).factor, MAX_FACTOR);
        assert!((plan(120.0).factor - 1.10).abs() < 0.001);
        assert_eq!(plan(100.0).tier, WidgetTier::SideBySide);
        assert!((plan(100.0).factor - 0.9).abs() < 0.001);
        assert_eq!(plan(70.0).tier, WidgetTier::Stacked);
        assert_eq!(plan(50.0).tier, WidgetTier::GlyphOnly);
        let bottom = plan_widgets(Edge::Bottom, 50.0, 1.0, &sizes);
        assert_eq!(bottom.tier, WidgetTier::SideBySide);
        assert_eq!(bottom.factor, 1.0);
    }

    #[test]
    fn a_widget_wider_than_a_side_bar_is_shrunk_to_it() {
        let layout = bar_layout_with_widgets(
            Edge::Right,
            60.0,
            1080.0,
            1.0,
            0,
            None,
            0,
            &[90.0],
            default_clock_extent(Edge::Right),
        );
        assert_eq!(layout.tray[0].right - layout.tray[0].left, 52.0);
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
