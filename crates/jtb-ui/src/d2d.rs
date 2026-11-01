use std::collections::HashMap;
use std::ffi::c_void;

use jtb_core::scene::{Color, Scene};
use windows::core::{w, Result};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_RECT_F, D2D_SIZE_U,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Bitmap, ID2D1Factory, ID2D1HwndRenderTarget,
    D2D1_BITMAP_INTERPOLATION_MODE_LINEAR, D2D1_BITMAP_PROPERTIES,
    D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT, D2D1_FACTORY_TYPE_SINGLE_THREADED,
    D2D1_HWND_RENDER_TARGET_PROPERTIES, D2D1_PRESENT_OPTIONS_NONE, D2D1_RENDER_TARGET_PROPERTIES,
};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat, DWRITE_FACTORY_TYPE_SHARED,
    DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL,
    DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_CENTER,
    DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TRIMMING, DWRITE_TRIMMING_GRANULARITY_CHARACTER,
    DWRITE_WORD_WRAPPING_NO_WRAP,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;

use crate::Renderer;

pub struct D2dRenderer {
    target: ID2D1HwndRenderTarget,
    text: IDWriteFactory,
    /// Uploaded icon images by `Icon::id`.
    bitmaps: HashMap<u64, ID2D1Bitmap>,
}

fn color(c: Color) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn rect(r: jtb_core::scene::Rect) -> D2D_RECT_F {
    D2D_RECT_F {
        left: r.left,
        top: r.top,
        right: r.right,
        bottom: r.bottom,
    }
}

impl D2dRenderer {
    /// Single-line text that ends in an ellipsis when it does not fit.
    fn text_format(&self, font_px: f32, leading: bool) -> Result<IDWriteTextFormat> {
        unsafe {
            let format = self.text.CreateTextFormat(
                w!("Segoe UI"),
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                font_px,
                w!("en-us"),
            )?;
            format.SetTextAlignment(if leading {
                DWRITE_TEXT_ALIGNMENT_LEADING
            } else {
                DWRITE_TEXT_ALIGNMENT_CENTER
            })?;
            format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
            format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
            let sign = self.text.CreateEllipsisTrimmingSign(&format)?;
            let trimming = DWRITE_TRIMMING {
                granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                delimiter: 0,
                delimiterCount: 0,
            };
            format.SetTrimming(&trimming, &sign)?;
            Ok(format)
        }
    }

    /// Uploads the scene's icons that are not cached yet and drops cached ones no longer shown.
    fn sync_bitmaps(&mut self, scene: &Scene) -> Result<()> {
        self.bitmaps
            .retain(|id, _| scene.icons.iter().any(|draw| draw.icon.id == *id));
        for draw in &scene.icons {
            let icon = &draw.icon;
            if self.bitmaps.contains_key(&icon.id) {
                continue;
            }
            let props = D2D1_BITMAP_PROPERTIES {
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                dpiX: 96.0,
                dpiY: 96.0,
            };
            let bitmap = unsafe {
                self.target.CreateBitmap(
                    D2D_SIZE_U {
                        width: icon.width,
                        height: icon.height,
                    },
                    Some(icon.bgra.as_ptr() as *const c_void),
                    icon.width * 4,
                    &props,
                )?
            };
            self.bitmaps.insert(icon.id, bitmap);
        }
        Ok(())
    }

    pub fn new(hwnd: HWND, width: u32, height: u32) -> Result<D2dRenderer> {
        unsafe {
            let factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let text: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let target = factory.CreateHwndRenderTarget(
                &D2D1_RENDER_TARGET_PROPERTIES::default(),
                &D2D1_HWND_RENDER_TARGET_PROPERTIES {
                    hwnd,
                    pixelSize: D2D_SIZE_U { width, height },
                    presentOptions: D2D1_PRESENT_OPTIONS_NONE,
                },
            )?;
            // Scene coordinates are physical pixels, so make one DIP equal one pixel.
            target.SetDpi(96.0, 96.0);
            Ok(D2dRenderer {
                target,
                text,
                bitmaps: HashMap::new(),
            })
        }
    }
}

impl Renderer for D2dRenderer {
    fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        unsafe { self.target.Resize(&D2D_SIZE_U { width, height }) }
    }

    fn draw(&mut self, scene: &Scene) -> Result<()> {
        self.sync_bitmaps(scene)?;
        unsafe {
            let centered = self.text_format(scene.font_px, false)?;
            let leading = self.text_format(scene.font_px, true)?;

            self.target.BeginDraw();
            self.target.Clear(Some(&color(scene.background)));
            for fill in &scene.fills {
                let brush = self
                    .target
                    .CreateSolidColorBrush(&color(fill.color), None)?;
                self.target.FillRectangle(&rect(fill.rect), &brush);
            }
            for draw in &scene.icons {
                if let Some(bitmap) = self.bitmaps.get(&draw.icon.id) {
                    self.target.DrawBitmap(
                        bitmap,
                        Some(&rect(draw.rect)),
                        1.0,
                        D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
                        None,
                    );
                }
            }
            for label in &scene.labels {
                let brush = self
                    .target
                    .CreateSolidColorBrush(&color(label.color), None)?;
                let wide: Vec<u16> = label.text.encode_utf16().collect();
                self.target.DrawText(
                    &wide,
                    if label.leading { &leading } else { &centered },
                    &rect(label.rect),
                    &brush,
                    D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
                    DWRITE_MEASURING_MODE_NATURAL,
                );
            }
            self.target.EndDraw(None, None)
        }
    }
}
