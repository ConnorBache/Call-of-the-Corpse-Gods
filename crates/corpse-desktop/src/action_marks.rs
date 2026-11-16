//! Semantic HUD anchors use the same light-only feedback as map cells.
//! Styles select the existing resource/item anchor, never decorative geometry.
use crate::render::{Painter, Rgb};
#[derive(Clone, Debug, PartialEq)]
pub struct Mark {
    pub style: String,
    pub progress: u16,
    pub direction: Option<(i8, i8)>,
}
/// O(w*h) native raster work / O(1) scratch. Fade is supplied by the detached
/// cue clock; the highlight stays inside this one resource/item tile.
pub fn paint(p: &mut Painter, _mark: &Mark, x: i32, y: i32, w: i32, h: i32, color: Rgb, alpha: f64) {
    p.blend_rect(x, y, w, h, color, (alpha.clamp(0., 1.) * 64.).round() as u8);
}
