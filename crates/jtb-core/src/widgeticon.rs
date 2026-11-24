//! Icons the bar draws itself, from shapes on a 32 x 32 grid, so they line up with each other
//! and with the bar's text; the stock symbol font places every glyph differently. Pure pixels.
use crate::widget::Power;

/// Width and height of the bitmaps; the renderer shrinks them to the icon size.
pub const SIZE: u32 = 64;
const GRID: f32 = 32.0;
const SAMPLES: u32 = 3;

type Color = [f32; 3];
const WHITE: Color = [1.0, 1.0, 1.0];
const GREEN: Color = [0.23, 0.88, 0.33];
const YELLOW: Color = [0.99, 0.84, 0.09];
const ORANGE: Color = [1.0, 0.42, 0.17];
const RED: Color = [0.97, 0.13, 0.20];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetIcon {
    /// A vertical battery filled to `percent`; the bolt shows the power cable is connected.
    Battery { percent: u8, power: Power },
    /// Three bars lit by level, left to right; all dim with a slash when muted.
    Volume { level: u8, muted: bool },
}

/// Premultiplied BGRA pixels, `SIZE` x `SIZE`.
pub fn render(icon: &WidgetIcon) -> Vec<u8> {
    let mut canvas = Canvas::new();
    match icon {
        WidgetIcon::Battery { percent, power } => battery(&mut canvas, *percent, *power),
        WidgetIcon::Volume { level, muted } => volume(&mut canvas, *level, *muted),
    }
    canvas.into_bgra()
}

/// The fill color: green while the cable is connected, else by level.
pub fn battery_fill(percent: u8, power: Power) -> [u8; 3] {
    let color = if power != Power::Battery || percent >= 50 {
        GREEN
    } else if percent >= 25 {
        YELLOW
    } else if percent > 15 {
        ORANGE
    } else {
        RED
    };
    color.map(|c| (c * 255.0).round() as u8)
}

const FILL_LEFT: f32 = 11.5;
const FILL_RIGHT: f32 = 20.5;
const FILL_TOP: f32 = 9.0;
const FILL_BOTTOM: f32 = 26.5;

fn battery(canvas: &mut Canvas, percent: u8, power: Power) {
    canvas.paint(WHITE, 0.95, &rrect(13.0, 2.5, 19.0, 7.0, 1.5));
    canvas.paint(WHITE, 0.95, &rrect(8.0, 5.5, 24.0, 30.0, 4.0));
    canvas.erase(&rrect(10.0, 7.5, 22.0, 28.0, 2.2));
    let percent = percent.min(100);
    if percent > 0 {
        let height = ((FILL_BOTTOM - FILL_TOP) * f32::from(percent) / 100.0).max(1.8);
        let [r, g, b] = battery_fill(percent, power).map(|c| f32::from(c) / 255.0);
        canvas.paint(
            [r, g, b],
            1.0,
            &rrect(
                FILL_LEFT,
                FILL_BOTTOM - height,
                FILL_RIGHT,
                FILL_BOTTOM,
                1.2,
            ),
        );
    }
    if power != Power::Battery {
        let bolt = [
            (18.6, 10.0),
            (12.0, 19.0),
            (15.4, 19.0),
            (13.4, 26.0),
            (20.0, 16.5),
            (16.6, 16.5),
        ];
        canvas.paint(WHITE, 1.0, &polygon(&bolt));
    }
}

const DIM: f32 = 0.28;

// The three bars of "Volume Level Indicator" from SVG Repo (CC0), a 47 x 47 drawing.
const BAR_LOW: [(f32, f32); 7] = [
    (1.215, 33.459),
    (10.494, 28.156),
    (14.1, 30.247),
    (14.1, 35.551),
    (11.69, 37.961),
    (2.41, 37.961),
    (0.081, 36.169),
];
const BAR_MID: [(f32, f32); 8] = [
    (16.512, 26.115),
    (17.726, 24.023),
    (26.884, 18.79),
    (30.49, 20.883),
    (30.49, 35.551),
    (28.08, 37.961),
    (18.922, 37.961),
    (16.512, 35.551),
];
const BAR_HIGH: [(f32, f32); 8] = [
    (32.9, 16.752),
    (34.115, 14.659),
    (43.394, 9.357),
    (47.0, 11.449),
    (47.0, 35.551),
    (44.589, 37.961),
    (35.31, 37.961),
    (32.9, 35.551),
];

/// A point of the 47 x 47 drawing on the grid, the bars centered.
fn on_grid(points: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let scale = GRID / 47.0;
    points
        .iter()
        .map(|(x, y)| (x * scale, (y - 23.66) * scale + GRID / 2.0))
        .collect()
}

/// Three bars, lit from the left by level: red, green, orange. Muted shows them all dim with a
/// red slash.
fn volume(canvas: &mut Canvas, level: u8, muted: bool) {
    let bars = [
        (&BAR_LOW[..], RED),
        (&BAR_MID[..], GREEN),
        (&BAR_HIGH[..], ORANGE),
    ];
    for (i, (points, color)) in bars.iter().enumerate() {
        let shape = on_grid(points);
        if i < usize::from(level) && !muted {
            canvas.paint(*color, 1.0, &polygon(&shape));
        } else {
            canvas.paint(WHITE, DIM, &polygon(&shape));
        }
    }
    if muted {
        canvas.paint(RED, 0.95, &segment((4.0, 8.0), (28.0, 27.0), 1.3));
    }
}

struct Canvas {
    /// Premultiplied RGBA.
    px: Vec<[f32; 4]>,
}

impl Canvas {
    fn new() -> Canvas {
        Canvas {
            px: vec![[0.0; 4]; (SIZE * SIZE) as usize],
        }
    }

    fn coverage(x: u32, y: u32, inside: &dyn Fn(f32, f32) -> bool) -> f32 {
        let scale = GRID / SIZE as f32;
        let mut hits = 0;
        for sy in 0..SAMPLES {
            for sx in 0..SAMPLES {
                let gx = (x as f32 + (sx as f32 + 0.5) / SAMPLES as f32) * scale;
                let gy = (y as f32 + (sy as f32 + 0.5) / SAMPLES as f32) * scale;
                if inside(gx, gy) {
                    hits += 1;
                }
            }
        }
        hits as f32 / (SAMPLES * SAMPLES) as f32
    }

    fn paint(&mut self, color: Color, alpha: f32, inside: &dyn Fn(f32, f32) -> bool) {
        for y in 0..SIZE {
            for x in 0..SIZE {
                let a = Self::coverage(x, y, inside) * alpha;
                if a > 0.0 {
                    let dst = &mut self.px[(y * SIZE + x) as usize];
                    for (d, c) in dst.iter_mut().zip([color[0], color[1], color[2], 1.0]) {
                        *d = c * a + *d * (1.0 - a);
                    }
                }
            }
        }
    }

    fn erase(&mut self, inside: &dyn Fn(f32, f32) -> bool) {
        for y in 0..SIZE {
            for x in 0..SIZE {
                let keep = 1.0 - Self::coverage(x, y, inside);
                for d in &mut self.px[(y * SIZE + x) as usize] {
                    *d *= keep;
                }
            }
        }
    }

    fn into_bgra(self) -> Vec<u8> {
        let byte = |v: f32| (v * 255.0).round().clamp(0.0, 255.0) as u8;
        self.px
            .iter()
            .flat_map(|p| [byte(p[2]), byte(p[1]), byte(p[0]), byte(p[3])])
            .collect()
    }
}

fn rrect(x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> impl Fn(f32, f32) -> bool {
    let r = r.min((x1 - x0) / 2.0).min((y1 - y0) / 2.0);
    move |x, y| {
        if x < x0 || x > x1 || y < y0 || y > y1 {
            return false;
        }
        let cx = x.clamp(x0 + r, x1 - r);
        let cy = y.clamp(y0 + r, y1 - r);
        (x - cx).hypot(y - cy) <= r
    }
}

fn segment(a: (f32, f32), b: (f32, f32), half_width: f32) -> impl Fn(f32, f32) -> bool {
    move |x, y| {
        let (vx, vy) = (b.0 - a.0, b.1 - a.1);
        let t = (((x - a.0) * vx + (y - a.1) * vy) / (vx * vx + vy * vy)).clamp(0.0, 1.0);
        (x - (a.0 + t * vx)).hypot(y - (a.1 + t * vy)) <= half_width
    }
}

fn polygon(points: &[(f32, f32)]) -> impl Fn(f32, f32) -> bool + '_ {
    move |x, y| {
        let mut inside = false;
        let mut j = points.len() - 1;
        for i in 0..points.len() {
            let (xi, yi) = points[i];
            let (xj, yj) = points[j];
            if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
                inside = !inside;
            }
            j = i;
        }
        inside
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha_at(bgra: &[u8], gx: f32, gy: f32) -> u8 {
        let x = (gx / GRID * SIZE as f32) as u32;
        let y = (gy / GRID * SIZE as f32) as u32;
        bgra[((y * SIZE + x) * 4 + 3) as usize]
    }

    fn rgb_at(bgra: &[u8], gx: f32, gy: f32) -> [u8; 3] {
        let x = (gx / GRID * SIZE as f32) as u32;
        let y = (gy / GRID * SIZE as f32) as u32;
        let at = ((y * SIZE + x) * 4) as usize;
        [bgra[at + 2], bgra[at + 1], bgra[at]]
    }

    fn battery_pixels(percent: u8, power: Power) -> Vec<u8> {
        render(&WidgetIcon::Battery { percent, power })
    }

    #[test]
    fn the_bitmap_has_the_declared_size() {
        assert_eq!(
            battery_pixels(50, Power::Battery).len(),
            (SIZE * SIZE * 4) as usize
        );
    }

    #[test]
    fn the_fill_rises_with_the_level() {
        let high = |p| alpha_at(&battery_pixels(p, Power::Battery), 16.0, 12.0);
        assert_eq!(high(100), 255);
        assert_eq!(high(50), 0);
        let low = |p| alpha_at(&battery_pixels(p, Power::Battery), 16.0, 25.0);
        assert_eq!(low(10), 255);
        assert_eq!(low(0), 0);
    }

    #[test]
    fn the_fill_color_follows_the_level_and_the_cable() {
        assert_eq!(battery_fill(80, Power::Battery), [59, 224, 84]);
        assert_eq!(battery_fill(40, Power::Battery), [252, 214, 23]);
        assert_eq!(battery_fill(20, Power::Battery), [255, 107, 43]);
        assert_eq!(battery_fill(15, Power::Battery), [247, 33, 51]);
        assert_eq!(battery_fill(15, Power::Charging), [59, 224, 84]);
        assert_eq!(battery_fill(100, Power::Plugged), [59, 224, 84]);
        assert_eq!(
            rgb_at(&battery_pixels(100, Power::Battery), 16.0, 24.0),
            [59, 224, 84]
        );
    }

    #[test]
    fn the_bolt_shows_only_with_the_cable() {
        let at_bolt = |power| rgb_at(&battery_pixels(100, power), 15.0, 21.0);
        assert_eq!(at_bolt(Power::Battery), [59, 224, 84]);
        assert_eq!(at_bolt(Power::Charging), [255, 255, 255]);
        assert_eq!(at_bolt(Power::Plugged), [255, 255, 255]);
    }

    fn volume_pixels(level: u8, muted: bool) -> Vec<u8> {
        render(&WidgetIcon::Volume { level, muted })
    }

    // The middle of each bar on the grid.
    const LOW_BAR: (f32, f32) = (4.8, 22.7);
    const MID_BAR: (f32, f32) = (16.0, 21.0);
    const HIGH_BAR: (f32, f32) = (27.2, 16.5);

    #[test]
    fn the_volume_bars_light_up_from_the_left_in_red_green_and_orange() {
        let at = |level, bar: (f32, f32)| rgb_at(&volume_pixels(level, false), bar.0, bar.1);
        assert_eq!(at(1, LOW_BAR), [247, 33, 51]);
        assert_eq!(at(2, MID_BAR), [59, 224, 84]);
        assert_eq!(at(3, HIGH_BAR), [255, 107, 43]);
    }

    #[test]
    fn the_unlit_volume_bars_are_dim() {
        let alpha = |level, bar: (f32, f32)| alpha_at(&volume_pixels(level, false), bar.0, bar.1);
        assert_eq!(alpha(1, LOW_BAR), 255);
        assert!(alpha(1, MID_BAR) > 0 && alpha(1, MID_BAR) < 100);
        assert!(alpha(2, HIGH_BAR) > 0 && alpha(2, HIGH_BAR) < 100);
        assert!(alpha(0, LOW_BAR) > 0 && alpha(0, LOW_BAR) < 100);
    }

    #[test]
    fn a_muted_volume_has_dim_bars_and_a_red_slash() {
        let pixels = volume_pixels(3, true);
        assert!(alpha_at(&pixels, LOW_BAR.0, LOW_BAR.1) < 100);
        assert!(alpha_at(&pixels, HIGH_BAR.0, HIGH_BAR.1) < 100);
        assert!(alpha_at(&pixels, 16.0, 17.5) > 200);
    }

    #[test]
    fn the_outline_and_cap_are_drawn() {
        let pixels = battery_pixels(0, Power::Battery);
        assert!(alpha_at(&pixels, 9.0, 18.0) > 200);
        assert!(alpha_at(&pixels, 16.0, 4.0) > 200);
        assert_eq!(alpha_at(&pixels, 16.0, 18.0), 0);
        assert_eq!(alpha_at(&pixels, 2.0, 2.0), 0);
    }
}
