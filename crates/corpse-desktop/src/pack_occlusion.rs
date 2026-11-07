//! Bounded opaque Pack occlusion, not retained/damage rendering. Only a plain
//! ASCII Cell whose complete conservative footprint is covered may be omitted.
use super::{damage::Rect, Cell, Painter, View};

#[cfg(test)]
#[derive(Clone,Copy,Debug,Default)]
pub(super) struct Stats {pub checks:u64,pub eligible:u64,pub skipped:u64}

impl Painter {
    /// Paint the ordinary game beneath a Pack that the caller MUST subsequently
    /// paint opaquely at these same logical bounds, in the same canvas/damage state.
    /// Only map Cells are eligible; Chronicle, HUD, effects and Pack remain ordered.
    /// O(C) bounded containment checks plus remaining original raster work, O(1)
    /// scratch. No pixel buffer, Cell list, spatial index or geometry cache is added.
    pub fn game_at_behind_pack(&mut self,v:&View,notice:&str,now:f64,bounds:(i32,i32,i32,i32)) {
        self.set_responsive_canvas(true);
        let previous=self.pack_map_occlusion;
        // Keep tile/retained composition and unusual font/contrast paths unchanged.
        let supported=!self.tile_clip_enabled && self.paint_cell_indices.is_none()
            && !self.prose_contrast_outline
            && self.profile.value("typography.map")==crate::appearance_profile::Profile::defaults().value("typography.map");
        self.pack_map_occlusion=if supported {self.pack_opaque_native_bounds(bounds)} else {None};
        self.game_at(v,notice,false,now);
        self.pack_map_occlusion=previous;
    }

    // Pack overrides logical clip with its own bounds and begins with rect(...),
    // whose alpha is always 255. Ignore the incoming logical clip here. Native
    // damage clips both old map paint and later Pack alike, so containment in the
    // full backing is conservative even when only a smaller damage area repaints.
    fn pack_opaque_native_bounds(&self,(x,y,w,h):(i32,i32,i32,i32))->Option<Rect> {
        if w<=0 || h<=0 {return None;}
        let right=x.checked_add(w)?;let bottom=y.checked_add(h)?;
        let (l,t)=(self.native_x(x as f64),self.native_y(y as f64));
        let (r,b)=(self.native_x(right as f64).max(l.saturating_add(1)),
            self.native_y(bottom as f64).max(t.saturating_add(1)));
        let (l,t,r,b)=(l.max(0),t.max(0),r.min(self.width as i32),b.min(self.height as i32));
        (l<r && t<b).then(||Rect::new(l as usize,t as usize,(r-l) as usize,(b-t) as usize))
    }

    pub(super) fn pack_hides_plain_cell(&mut self,c:&Cell,x:i32,y:i32,cw:i32,ch:i32)->bool {
        let Some(cover)=self.pack_map_occlusion else{return false;};
        #[cfg(test)] {self.pack_occlusion_stats.checks+=1;}
        // Text annotations, compound/fallback identities, emphasized/italic glyphs
        // and underlays keep the original path. Do not estimate their overhang.
        if cw<=0 || ch<=0 || c.glyph.len()!=1 || !matches!(c.glyph.as_bytes()[0],32..=126)
            || c.italic || c.heartless || !c.underlays.is_empty()
            || !c.superscript.is_empty() || !c.subscript.is_empty() || !c.arrow.is_empty()
            || !c.emphasis.is_finite() || c.emphasis>1. {return false;}
        #[cfg(test)] {self.pack_occlusion_stats.eligible+=1;}
        // With exact default map typography and emphasis<=1, atlas glyph width is
        // <=cw and height<=2ch; center/rounding stays in this deliberately wider
        // three-cell rectangle. Other eligible strokes/overlays stay within the
        // tile (+one logical pixel for tiny positive rects). Add two logical and
        // one native pixels on every edge to preserve subpixel min-one strokes.
        let Some(l)=x.checked_sub(cw).and_then(|n|n.checked_sub(2)) else{return false;};
        let Some(t)=y.checked_sub(ch).and_then(|n|n.checked_sub(2)) else{return false;};
        let Some(r)=cw.checked_mul(2).and_then(|n|x.checked_add(n)).and_then(|n|n.checked_add(2)) else{return false;};
        let Some(b)=ch.checked_mul(2).and_then(|n|y.checked_add(n)).and_then(|n|n.checked_add(2)) else{return false;};
        let (l,t,r,b)=(self.native_x(l as f64).saturating_sub(1),self.native_y(t as f64).saturating_sub(1),
            self.native_x(r as f64).saturating_add(1),self.native_y(b as f64).saturating_add(1));
        // Do not clip the footprint before containment. Conservative rejection at
        // framebuffer/Pack edges avoids mistaking overhang for covered content.
        let hidden=l>=cover.x as i32 && t>=cover.y as i32
            && r<=cover.right() as i32 && b<=cover.bottom() as i32;
        #[cfg(test)] if hidden {self.pack_occlusion_stats.skipped+=1;}
        hidden
    }
}

#[cfg(test)]
#[path="pack_occlusion_tests.rs"]
mod tests;
