mod d2d;

pub use d2d::D2dRenderer;

use jtb_core::scene::Scene;
use windows::core::Result;

/// Draws a laid-out `Scene`; the only place that knows which graphics API is in use.
pub trait Renderer {
    fn resize(&mut self, width: u32, height: u32) -> Result<()>;
    fn draw(&mut self, scene: &Scene) -> Result<()>;
}
