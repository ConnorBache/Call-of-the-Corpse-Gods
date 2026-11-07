//! One exact screen-space fade LUT entry. No framebuffer, glyph or game cache.
use super::{damage, tile_damage, Painter, Rgb};

type RowTable = [[u8; 256]; 3];
// At most 1,572,864 requested payload bytes. Taller fades retain the old loop.
const MAX_CACHED_ROWS: usize = 2048;

#[derive(Default)]
pub(super) struct FadeCache {
    key: Option<(usize, Rgb)>,
    rows: Vec<RowTable>,
    #[cfg(test)]
    rebuilds: usize,
    #[cfg(test)]
    generated_rows: usize,
}

/// Exact original arithmetic and operation order, including the historical
/// composite clamp/round conversion. Do not replace it with integer blending.
fn row_table(row: usize, height: usize, background: Rgb) -> RowTable {
    let position = row as f64 / height.max(1) as f64;
    let opacity = crate::chronicle::fade(position);
    let mut table = [[0u8; 256]; 3];
    for value in 0..256 {
        let result = crate::color::composite(background, [value as u8; 3], opacity);
        for k in 0..3 { table[k][value] = result[k]; }
    }
    table
}

impl FadeCache {
    /// O(1) warm lookup; O(256*H) rebuild when exact native height or RGB changes.
    /// One entry, reusing its row capacity. No per-frame allocation on cache hit.
    /// Height above the fixed budget or reserve failure uses the exact old path.
    fn prepare(&mut self, height: usize, background: Rgb) -> bool {
        if height == 0 || height > MAX_CACHED_ROWS { return false; }
        if self.key == Some((height,background)) { return true; }
        if self.rows.capacity() < height
            && self.rows.try_reserve_exact(height-self.rows.len()).is_err() { return false; }
        self.key = None;
        self.rows.resize(height,[[0;256];3]);
        for (row,table) in self.rows.iter_mut().enumerate() {
            *table = row_table(row,height,background);
        }
        self.key = Some((height,background));
        #[cfg(test)]
        { self.rebuilds += 1; self.generated_rows += height; }
        true
    }

    /// Resize invalidates the key, but bounded row capacity remains reusable.
    pub(super) fn invalidate(&mut self) { self.key=None; self.rows.clear(); }
}

impl Painter {
    /// Exact old Chronicle rectangle contract: supplied native bounds already
    /// include framebuffer clipping; logical clip is intentionally not consulted.
    /// Warm O(R+S+P) plus existing tile-span lookup, cold adds O(256*H), where R
    /// is clipped rows, S selected spans, P their pixels, H unclipped fade height.
    /// Alpha bytes are never touched. No pixel/row clock or collector lookup.
    pub(super) fn paint_chronicle_fade(&mut self, background: Rgb,
        bounds:(i32,i32,i32,i32)) {
        // GPU path keeps its native row quads; the LUT is CPU fallback only.
        if let Some(gpu)=self.gpu.as_mut(){
            let (left,top,right,bottom)=bounds;
            let d=self.damage_clip.unwrap_or(damage::Rect::new(0,0,self.width,self.height));
            for y in top.max(d.y as i32)..bottom.min(d.bottom()as i32){
                let opacity=crate::chronicle::fade((y-top)as f64/(bottom-top).max(1)as f64);
                gpu.fill((left.max(d.x as i32),y,right.min(d.right()as i32),y+1),background,((1.-opacity)*255.).round().clamp(0.,255.)as u8);
            }
            return;
        }
        #[cfg(test)]
        let mut profile=super::paint_profile_bench::span(super::paint_profile_bench::Kind::Fade);
        let (left,top,right,bottom)=bounds;
        let d=self.damage_clip.unwrap_or(damage::Rect::new(0,0,self.width,self.height));
        let fade_left=left.max(d.x as i32);let fade_right=right.min(d.right() as i32);
        let start=top.max(d.y as i32);let end=bottom.min(d.bottom() as i32);
        if start>=end || fade_left>=fade_right { return; }
        let height=(bottom-top).max(1) as usize;
        #[cfg(test)]
        let enabled=self.chronicle_fade_cache_enabled;
        #[cfg(not(test))]
        let enabled=true;
        #[cfg(test)]
        let prior_rebuilds=self.chronicle_fade_cache.rebuilds;
        let cached=enabled && self.chronicle_fade_cache.prepare(height,background);
        #[cfg(test)]
        if self.chronicle_fade_cache.rebuilds!=prior_rebuilds { profile.cache_miss(); }
        for y in start..end {
            let fallback;
            let table=if cached { &self.chronicle_fade_cache.rows[(y-top) as usize] }
                else { fallback=row_table((y-top) as usize,height,background); &fallback };
            for(l,r)in tile_damage::row_segments(self.tile_clip_enabled.then_some(&self.tile_mask),y,fade_left,fade_right){
                self.raster_stats.pixels((r-l) as usize);
                #[cfg(test)]
                profile.pixels((r-l) as usize);
                let row=&mut self.pixels[(y as usize*self.width+l as usize)*4
                    ..(y as usize*self.width+r as usize)*4];
                for pixel in row.chunks_exact_mut(4) {
                    for k in 0..3 { pixel[k]=table[k][pixel[k] as usize]; }
                }
            }
        }
    }
}

#[cfg(test)]
#[path="chronicle_fade_cache_tests.rs"]
mod tests;
