//! Native, un-clipped ink geometry shared by glyph painting and attached markers.
//! This is render-cache metadata only, never an observed or serialized field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Bounds {
    pub(super) left: i32,
    pub(super) top: i32,
    pub(super) right: i32,
    pub(super) bottom: i32,
}
impl Bounds {
    /// Once per final raster insertion: O(mask area), O(1) scratch. Transparent
    /// padding and font advances are excluded; pixel bounds are half-open.
    pub(super) fn from_mask(mask: &[u8], width: usize) -> Option<Self> {
        if width == 0 { return None; }
        let mut bounds = None;
        for (y, row) in mask.chunks_exact(width).enumerate() {
            if let (Some(left), Some(right)) = (row.iter().position(|&a| a != 0), row.iter().rposition(|&a| a != 0)) {
                Self::extend(&mut bounds, Self { left:left as i32, top:y as i32, right:right as i32+1, bottom:y as i32+1 });
            }
        }
        bounds
    }
    pub(super) fn extend(bounds: &mut Option<Self>, row: Self) {
        *bounds = Some(bounds.map_or(row, |b| Self {
            left:b.left.min(row.left), top:b.top.min(row.top),
            right:b.right.max(row.right), bottom:b.bottom.max(row.bottom),
        }));
    }
    pub(super) fn translated(self, x: i32, y: i32) -> Self {
        Self { left:self.left+x, top:self.top+y, right:self.right+x, bottom:self.bottom+y }
    }
    pub(super) fn center(self) -> (f64, f64) {
        ((self.left as f64+self.right as f64)/2., (self.top as f64+self.bottom as f64)/2.)
    }
}
