//! Reads Start button art in Open-Shell's strip format: frames of equal height stacked vertically.
//! Derived from the button loading code of Open-Shell (MIT), see NOTICE.

use crate::icon::premultiply;

/// First two pixels of the optional header row, as 0xAARRGGBB without alpha ("MNA", "NTB").
const HEADER_TAG1: u32 = 0x41_4e_4d;
const HEADER_TAG2: u32 = 0x42_54_4e;

/// One frame as premultiplied BGRA pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartArt {
    pub normal: Frame,
    pub hot: Frame,
    pub pressed: Frame,
}

/// Decodes a PNG into 8-bit RGBA.
pub fn decode_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let data = &buf[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => data
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => data
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => data.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return Err("indexed PNG was not expanded".to_string()),
    };
    Ok((info.width, info.height, rgba))
}

fn pixel(rgba: &[u8], width: u32, x: u32, y: u32) -> u32 {
    let i = (y * width + x) as usize * 4;
    let p = &rgba[i..i + 4];
    (u32::from(p[3]) << 24) | (u32::from(p[0]) << 16) | (u32::from(p[1]) << 8) | u32::from(p[2])
}

/// Cuts the strip into the normal, hot and pressed frames. A header row (tags in its first two
/// pixels) lists the frame count and which frames those are; without one the strip is three
/// frames in that order.
pub fn parse_strip(width: u32, height: u32, rgba: &[u8]) -> Result<StartArt, String> {
    if width == 0 || height < 3 || rgba.len() != (width * height) as usize * 4 {
        return Err("the image is empty or its size does not match its pixels".to_string());
    }
    let mut header_rows = 0;
    let mut total = 3;
    let mut picks = [0, 1, 2];
    if width >= 10
        && pixel(rgba, width, 0, 0) & 0xFF_FFFF == HEADER_TAG1
        && pixel(rgba, width, 1, 0) & 0xFF_FFFF == HEADER_TAG2
    {
        header_rows = ((pixel(rgba, width, 2, 0) >> 16) & 255).min(height);
        total = (pixel(rgba, width, 2, 0) & 255).clamp(1, height.saturating_sub(1).max(1));
        for (n, pick) in picks.iter_mut().enumerate() {
            *pick = (pixel(rgba, width, 3 + n as u32, 0) & 255).min(total - 1);
        }
    }
    let frame_height = (height - header_rows) / total;
    if frame_height == 0 {
        return Err("the image has no room for its frames".to_string());
    }
    let frame = |index: u32| {
        let first = (header_rows + index * frame_height) as usize * width as usize * 4;
        let rows = &rgba[first..first + (frame_height * width) as usize * 4];
        let mut bgra = Vec::with_capacity(rows.len());
        for p in rows.as_chunks::<4>().0 {
            let alpha = p[3];
            bgra.extend([
                premultiply(p[2], alpha),
                premultiply(p[1], alpha),
                premultiply(p[0], alpha),
                alpha,
            ]);
        }
        Frame {
            width,
            height: frame_height,
            bgra,
        }
    };
    Ok(StartArt {
        normal: frame(picks[0]),
        hot: frame(picks[1]),
        pressed: frame(picks[2]),
    })
}

/// Decodes a PNG file's bytes and cuts it into frames.
pub fn load(bytes: &[u8]) -> Result<StartArt, String> {
    let (width, height, rgba) = decode_png(bytes)?;
    parse_strip(width, height, &rgba)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A strip of `rows` rows where every pixel of row `y` is `[y, 0, 0, 255]` (red = row number).
    fn strip(width: u32, rows: u32) -> Vec<u8> {
        (0..rows)
            .flat_map(|y| (0..width).flat_map(move |_| [y as u8, 0, 0, 255]))
            .collect()
    }

    fn red_of(frame: &Frame) -> u8 {
        frame.bgra[2]
    }

    #[test]
    fn a_plain_strip_is_three_equal_frames() {
        let art = parse_strip(4, 9, &strip(4, 9)).unwrap();
        assert_eq!((art.normal.width, art.normal.height), (4, 3));
        assert_eq!(red_of(&art.normal), 0);
        assert_eq!(red_of(&art.hot), 3);
        assert_eq!(red_of(&art.pressed), 6);
    }

    #[test]
    fn a_header_row_names_the_frames() {
        let width = 10;
        let mut rgba = strip(width, 1 + 4 * 2);
        let put = |rgba: &mut Vec<u8>, x: usize, value: u32| {
            rgba[x * 4..x * 4 + 4].copy_from_slice(&[
                (value >> 16) as u8,
                (value >> 8) as u8,
                value as u8,
                255,
            ]);
        };
        put(&mut rgba, 0, HEADER_TAG1);
        put(&mut rgba, 1, HEADER_TAG2);
        put(&mut rgba, 2, 0x01_00_04); // one header row, four frames
        put(&mut rgba, 3, 0); // normal
        put(&mut rgba, 4, 3); // hot
        put(&mut rgba, 5, 99); // pressed, out of range: the last frame
        let art = parse_strip(width, 9, &rgba).unwrap();
        assert_eq!(art.normal.height, 2);
        assert_eq!(red_of(&art.normal), 1);
        assert_eq!(red_of(&art.hot), 7);
        assert_eq!(red_of(&art.pressed), 7);
    }

    #[test]
    fn pixels_become_premultiplied_bgra() {
        let mut rgba = vec![0u8; 4 * 3];
        rgba[0..4].copy_from_slice(&[200, 100, 50, 128]);
        let art = parse_strip(1, 3, &rgba).unwrap();
        assert_eq!(art.normal.bgra, vec![25, 50, 100, 128]);
    }

    #[test]
    fn bad_sizes_are_rejected() {
        assert!(parse_strip(0, 3, &[]).is_err());
        assert!(parse_strip(2, 2, &strip(2, 2)).is_err());
        assert!(parse_strip(2, 3, &[0; 4]).is_err());
    }

    #[test]
    fn png_files_decode_to_rgba() {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 2, 3);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[7; 2 * 3 * 3]).unwrap();
        }
        let (w, h, rgba) = decode_png(&bytes).unwrap();
        assert_eq!((w, h), (2, 3));
        assert_eq!(&rgba[..4], &[7, 7, 7, 255]);
        assert!(load(&bytes).is_ok());
        assert!(decode_png(b"not a png").is_err());
    }
}
