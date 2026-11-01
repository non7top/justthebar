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

fn premultiply(channel: u8, alpha: u8) -> u8 {
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
}
