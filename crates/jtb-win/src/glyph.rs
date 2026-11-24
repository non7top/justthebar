//! Draws a glyph or a short text into a white-on-transparent bitmap, so a widget can be shown
//! like a tray icon.
use windows::core::PCWSTR;
use windows::Win32::Foundation::{RECT, SIZE};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateFontW, DeleteDC, DeleteObject, DrawTextW, GetDC,
    GetTextExtentPoint32W, ReleaseDC, SelectObject, SetBkColor, SetBkMode, SetTextColor,
    ANTIALIASED_QUALITY, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CLIP_DEFAULT_PRECIS,
    DEFAULT_CHARSET, DIB_RGB_COLORS, DT_CENTER, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, FW_NORMAL,
    HGDIOBJ, OPAQUE, OUT_DEFAULT_PRECIS,
};

/// Width in pixels of `text` in `font` at `em` pixels high, as GDI measures it.
pub fn text_width(font: &str, text: &str, em: u32) -> u32 {
    let face: Vec<u16> = font.encode_utf16().chain(Some(0)).collect();
    let wide: Vec<u16> = text.encode_utf16().collect();
    unsafe {
        let screen = GetDC(None);
        let dc = CreateCompatibleDC(Some(screen));
        ReleaseDC(None, screen);
        let font_handle = CreateFontW(
            -(em as i32),
            0,
            0,
            0,
            FW_NORMAL.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            ANTIALIASED_QUALITY,
            0,
            PCWSTR(face.as_ptr()),
        );
        let old_font = SelectObject(dc, HGDIOBJ(font_handle.0));
        let mut size = SIZE::default();
        let _ = GetTextExtentPoint32W(dc, &wide, &mut size);
        SelectObject(dc, old_font);
        let _ = DeleteObject(HGDIOBJ(font_handle.0));
        let _ = DeleteDC(dc);
        size.cx.max(0) as u32
    }
}

/// Premultiplied BGRA pixels, `size` x `size`, white where `text` is drawn in `font`, shifted so
/// that the ink of `center_on` (the text itself, or a fuller form of it) is centered: the glyphs sit
/// at different places in their boxes.
pub fn render(
    font: &str,
    text: &str,
    size: u32,
    em: u32,
    tint: [u8; 3],
    center_on: &str,
) -> Option<Vec<u8>> {
    let reference = draw_at(font, center_on, size, em, [255; 3], (0, 0))?;
    let shift = centering_shift(&reference, size);
    draw_at(font, text, size, em, tint, shift)
}

/// How far to move the drawn pixels to center their ink in the bitmap.
fn centering_shift(bgra: &[u8], size: u32) -> (i32, i32) {
    let (mut left, mut top, mut right, mut bottom) = (u32::MAX, u32::MAX, 0, 0);
    for (i, px) in bgra.as_chunks::<4>().0.iter().enumerate() {
        if px[3] > 8 {
            let (x, y) = (i as u32 % size, i as u32 / size);
            (left, top, right, bottom) = (left.min(x), top.min(y), right.max(x), bottom.max(y));
        }
    }
    if left == u32::MAX {
        return (0, 0);
    }
    let middle = size as i32 / 2;
    (
        middle - ((left + right) / 2) as i32,
        middle - ((top + bottom) / 2) as i32,
    )
}

fn draw_at(
    font: &str,
    text: &str,
    size: u32,
    em: u32,
    tint: [u8; 3],
    shift: (i32, i32),
) -> Option<Vec<u8>> {
    let face: Vec<u16> = font.encode_utf16().chain(Some(0)).collect();
    let mut wide: Vec<u16> = text.encode_utf16().collect();
    unsafe {
        let screen = GetDC(None);
        let dc = CreateCompatibleDC(Some(screen));
        ReleaseDC(None, screen);
        let header = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: size as i32,
            biHeight: -(size as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };
        let info = BITMAPINFO {
            bmiHeader: header,
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0).ok()?;
        if bits.is_null() {
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
            let _ = DeleteDC(dc);
            return None;
        }
        let old_bitmap = SelectObject(dc, HGDIOBJ(bitmap.0));
        let font_handle = CreateFontW(
            -(em as i32),
            0,
            0,
            0,
            FW_NORMAL.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            ANTIALIASED_QUALITY,
            0,
            PCWSTR(face.as_ptr()),
        );
        let old_font = SelectObject(dc, HGDIOBJ(font_handle.0));
        SetBkMode(dc, OPAQUE);
        SetBkColor(dc, windows::Win32::Foundation::COLORREF(0));
        SetTextColor(dc, windows::Win32::Foundation::COLORREF(0x00FF_FFFF));
        let mut rect = RECT {
            left: shift.0,
            top: shift.1,
            right: size as i32 + shift.0,
            bottom: size as i32 + shift.1,
        };
        DrawTextW(
            dc,
            &mut wide,
            &mut rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
        let pixels = std::slice::from_raw_parts(bits as *const u8, (size * size * 4) as usize);
        let mut out = Vec::with_capacity(pixels.len());
        for px in pixels.as_chunks::<4>().0 {
            let alpha = px[0].max(px[1]).max(px[2]);
            let scaled = |channel: u8| (u16::from(channel) * u16::from(alpha) / 255) as u8;
            out.extend_from_slice(&[scaled(tint[2]), scaled(tint[1]), scaled(tint[0]), alpha]);
        }
        SelectObject(dc, old_font);
        SelectObject(dc, old_bitmap);
        let _ = DeleteObject(HGDIOBJ(font_handle.0));
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(dc);
        Some(out)
    }
}
