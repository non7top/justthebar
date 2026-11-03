/// A window icon as premultiplied BGRA pixels. `id` is unique per loaded image so renderers can
/// cache their copy.
#[derive(Debug, PartialEq, Eq)]
pub struct Icon {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

impl Icon {
    pub fn same_pixels(&self, other: &Icon) -> bool {
        self.width == other.width && self.height == other.height && self.bgra == other.bgra
    }
}

/// Shrinks premultiplied BGRA pixels to `dw` x `dh` by averaging the source area under each new
/// pixel. Bilinear sampling skips most source pixels once the shrink passes about 2x and shimmers.
/// Returns `None` unless the sizes are valid and the result is not larger than the source.
pub fn downscale(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32) -> Option<Vec<u8>> {
    if dw == 0 || dh == 0 || dw > sw || dh > sh || src.len() != (sw * sh) as usize * 4 {
        return None;
    }
    let (xs, ys) = (sw as f32 / dw as f32, sh as f32 / dh as f32);
    let overlap = |from: f32, to: f32, cell: u32| to.min(cell as f32 + 1.0) - from.max(cell as f32);
    let mut out = Vec::with_capacity((dw * dh) as usize * 4);
    for dy in 0..dh {
        let (y0, y1) = (dy as f32 * ys, (dy + 1) as f32 * ys);
        for dx in 0..dw {
            let (x0, x1) = (dx as f32 * xs, (dx + 1) as f32 * xs);
            let mut sum = [0.0f32; 4];
            let mut total = 0.0f32;
            for sy in y0 as u32..(y1.ceil() as u32).min(sh) {
                let wy = overlap(y0, y1, sy);
                for sx in x0 as u32..(x1.ceil() as u32).min(sw) {
                    let weight = wy * overlap(x0, x1, sx);
                    let at = (sy * sw + sx) as usize * 4;
                    for (acc, channel) in sum.iter_mut().zip(&src[at..at + 4]) {
                        *acc += f32::from(*channel) * weight;
                    }
                    total += weight;
                }
            }
            out.extend(
                sum.iter()
                    .map(|acc| (acc / total).round().clamp(0.0, 255.0) as u8),
            );
        }
    }
    Some(out)
}

pub(crate) fn premultiply(channel: u8, alpha: u8) -> u8 {
    ((channel as u16 * alpha as u16 + 127) / 255) as u8
}

/// Turns the BGRA pixels of an icon's color bitmap into premultiplied ones. Icons without an
/// alpha channel take their transparency from the AND mask (32-bit rows, non-zero = transparent).
pub fn finish_pixels(color: &mut [u8], mask: Option<&[u8]>) {
    let has_alpha = color.as_chunks::<4>().0.iter().any(|px| px[3] != 0);
    for (i, px) in color.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let alpha = if has_alpha {
            px[3]
        } else if mask.is_some_and(|m| m.get(i * 4).copied().unwrap_or(0) != 0) {
            0
        } else {
            255
        };
        for channel in &mut px[..3] {
            *channel = premultiply(*channel, alpha);
        }
        px[3] = alpha;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_channel_is_premultiplied() {
        let mut px = [200, 100, 50, 128];
        finish_pixels(&mut px, None);
        assert_eq!(px, [100, 50, 25, 128]);
    }

    #[test]
    fn mask_decides_transparency_without_alpha() {
        let mut px = [10, 20, 30, 0, 40, 50, 60, 0];
        let mask = [255, 255, 255, 0, 0, 0, 0, 0];
        finish_pixels(&mut px, Some(&mask));
        assert_eq!(px, [0, 0, 0, 0, 40, 50, 60, 255]);
    }

    #[test]
    fn no_alpha_and_no_mask_is_opaque() {
        let mut px = [1, 2, 3, 0];
        finish_pixels(&mut px, None);
        assert_eq!(px, [1, 2, 3, 255]);
    }

    #[test]
    fn downscale_averages_every_source_pixel() {
        let src = [
            0, 0, 0, 255, 100, 100, 100, 255, 200, 200, 200, 255, 100, 100, 100, 255,
        ];
        assert_eq!(downscale(&src, 2, 2, 1, 1).unwrap(), [100, 100, 100, 255]);
    }

    #[test]
    fn downscale_by_a_fraction_keeps_a_flat_image_flat() {
        let src: Vec<u8> = [30, 60, 90, 255].repeat(9);
        let out = downscale(&src, 3, 3, 2, 2).unwrap();
        assert_eq!(out, [30, 60, 90, 255].repeat(4));
    }

    #[test]
    fn downscale_weighs_partly_covered_pixels() {
        // 3 -> 2 wide: the middle pixel is shared half and half between the two outputs.
        let src = [0, 0, 0, 0, 90, 90, 90, 90, 0, 0, 0, 0];
        let out = downscale(&src, 3, 1, 2, 1).unwrap();
        assert_eq!(out, [30, 30, 30, 30, 30, 30, 30, 30]);
    }

    #[test]
    fn downscale_refuses_growing_or_mismatched_input() {
        let src = [0u8; 16];
        assert!(downscale(&src, 2, 2, 4, 4).is_none());
        assert!(downscale(&src, 2, 2, 0, 1).is_none());
        assert!(downscale(&src, 3, 2, 1, 1).is_none());
    }
}
