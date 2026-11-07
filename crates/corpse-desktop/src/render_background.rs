//! Exact replacement for two consecutive opaque passes: clear(base), rect(fog).
//! No retained pixels, draw-list discovery, cache, or simulation dependencies.
use super::{damage, tile_damage, Painter, Rgb};

#[inline]
fn fill_rgba(row: &mut [u8], ink: &[u8; 4]) {
    for pixel in row.chunks_exact_mut(4) { pixel.copy_from_slice(ink); }
}

impl Painter {
    /// Clear Q selected native pixels exactly once, choosing the rectangle color
    /// only where the old subsequent rect would cover them. O(H + S + Q) time,
    /// O(1) scratch: H clear rows, S tile-selected spans. Full frame Q=P instead
    /// of the old P+A_fog writes. Logical clipping affects rect, never clear.
    pub(super) fn clear_with_rect(&mut self, base: Rgb, rect: (i32,i32,i32,i32), color: Rgb) {
        if self.gpu.is_some(){self.clear(base);self.rect(rect.0,rect.1,rect.2,rect.3,color);return;}

        #[cfg(test)]
        if !self.background_fused_enabled {
            self.clear(base);
            self.rect(rect.0,rect.1,rect.2,rect.3,color);
            return;
        }
        // This is one opaque fill primitive inside the existing Background span.
        // Compare total Background time; the former separate Clear call is gone.
        #[cfg(test)]
        let mut profile=super::paint_profile_bench::span(super::paint_profile_bench::Kind::FillOpaque);
        let framebuffer=damage::Rect::new(0,0,self.width,self.height);
        let Some(area)=self.damage_clip.unwrap_or(framebuffer).intersect(framebuffer) else{return;};
        let (x,y,w,h)=rect;
        let fog=if w>0 && h>0 {
            // These are exactly rect's rounded endpoints/minimum positive stroke,
            // intersected with native_fill's framebuffer/logical/damage bounds.
            let (left,top)=(self.native_x(x as f64),self.native_y(y as f64));
            let right=self.native_x((x+w) as f64).max(left+1);
            let bottom=self.native_y((y+h) as f64).max(top+1);
            let (l,t,r,b)=self.native_bounds();
            let (l,t,r,b)=(l.max(left),t.max(top),r.min(right),b.min(bottom));
            (l<r && t<b).then_some((l,t,r,b))
        } else {None};
        let base=[base[0],base[1],base[2],255];
        let color=[color[0],color[1],color[2],255];
        for y in area.y..area.bottom() {
            for (l,r) in tile_damage::row_segments(self.tile_clip_enabled.then_some(&self.tile_mask),
                y as i32,area.x as i32,area.right() as i32) {
                self.raster_stats.pixels((r-l) as usize);
                #[cfg(test)]
                profile.pixels((r-l) as usize);
                let row=&mut self.pixels[(y*self.width+l as usize)*4..(y*self.width+r as usize)*4];
                let middle=fog.and_then(|(fl,ft,fr,fb)| {
                    let (a,b)=(l.max(fl),r.min(fr));
                    (y as i32>=ft && (y as i32)<fb && a<b).then_some((a,b))
                });
                if let Some((a,b))=middle {
                    let (before,rest)=row.split_at_mut((a-l) as usize*4);
                    let (inside,after)=rest.split_at_mut((b-a) as usize*4);
                    fill_rgba(before,&base);
                    fill_rgba(inside,&color);
                    fill_rgba(after,&base);
                } else { fill_rgba(row,&base); }
            }
        }
    }
}

#[cfg(test)]
#[path="render_background_tests.rs"]
mod tests;
