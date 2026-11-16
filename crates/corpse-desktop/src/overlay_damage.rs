//! Bounded overlay invalidation from detached UI state, never framebuffer comparison.
//! O(displayed effect tiles + hovered tooltip text + pack hover rows), with <=8 regions.
use crate::{render::{Painter,View},render::damage::{Damage,Rect},pack::PackPaintState};
#[derive(Clone,PartialEq)]
pub struct Hover {pub report:crate::inspection_ui::Report,pub tile:(i32,i32,i32,i32),pub bounds:Rect}
#[derive(Clone,PartialEq)]
pub struct Effect {pub x:i32,pub y:i32,pub color:[u8;3],pub light:u64,pub glyph:Option<String>,pub bounds:Rect}
#[derive(Clone,PartialEq)]
pub struct Shutter {pub bounds:Rect,pub offset:usize}
#[derive(Default)]
pub struct Frame {pub pack:Option<PackPaintState>,pub hover:Option<Hover>,pub effects:Vec<Effect>,pub shutter:Option<Shutter>,pub fallback:bool,pub save_error:Option<String>}
#[derive(Default)]
pub struct Tracker{previous:Option<Frame>}
fn logical(d:&mut Damage,p:&Painter,r:crate::modal_ui::Rect){if let Some(r)=p.logical_damage_rect(r.0,r.1,r.2,r.3){d.add(r);}}
fn banner(p:&Painter)->Option<Rect>{p.logical_damage_rect(8,8,1264,76)}
impl Tracker{
    pub fn invalidate(&mut self){self.previous=None;}
    /// Regions may conservatively contain unchanged pixels, but unchanged layers are reused.
    pub fn changes(&self,p:&Painter,next:&Frame)->Damage{
        let mut d=Damage::empty(p.width,p.height);
        let Some(old)=&self.previous else{return Damage::full(p.width,p.height);};
        if next.fallback||old.fallback{return Damage::full(p.width,p.height);}
        match (&old.pack,&next.pack){
            (Some(a),Some(b))=>for r in b.changes(a){logical(&mut d,p,r);},
            (a,b)=>{if let Some(a)=a{logical(&mut d,p,a.bounds);}if let Some(b)=b{logical(&mut d,p,b.bounds);}}
        }
        if old.hover!=next.hover{if let Some(v)=&old.hover{d.add(v.bounds);}if let Some(v)=&next.hover{d.add(v.bounds);}}
        if old.effects!=next.effects{for e in old.effects.iter().chain(&next.effects){d.add(e.bounds);}}
        if old.shutter!=next.shutter{if let Some(v)=&old.shutter{d.add(v.bounds);}if let Some(v)=&next.shutter{d.add(v.bounds);}}
        if old.save_error!=next.save_error {if let Some(r)=banner(p){d.add(r);}}
        d
    }
    pub fn commit(&mut self,next:Frame){self.previous=Some(next);}
}
impl Frame{
    /// Contrast sampling can inspect a complete text line. Any intersected translucent
    /// hover therefore invalidates its entire conservative bounds before compositing.
    pub fn expand_dependencies(&self,p:&Painter,d:&mut Damage){
        if let Some(h)=&self.hover{if d.rects().iter().any(|r|r.intersect(h.bounds).is_some()){d.add(h.bounds);}}
        if let Some(pack)=&self.pack{
            // Native damage may straddle logical lines. Conservatively select every
            // logical dependency whose native footprint intersects any damaged region.
            let mut logical_regions=Vec::new();
            for r in d.rects(){let(a,b)=p.logical_point(r.x as i32,r.y as i32,(p.width as u32,p.height as u32));let(c,e)=p.logical_point(r.right()as i32,r.bottom()as i32,(p.width as u32,p.height as u32));logical_regions.push(crate::modal_ui::Rect(a-1,b-1,c-a+2,e-b+2));}
            pack.expand_damage(&mut logical_regions);
            for r in logical_regions{logical(d,p,r);}
        }
    }
    pub fn pack_intersects(&self,p:&Painter,r:Rect)->bool{self.pack.as_ref().and_then(|s|p.logical_damage_rect(s.bounds.0,s.bounds.1,s.bounds.2,s.bounds.3)).is_some_and(|b|b.intersect(r).is_some())}
    pub fn hover_intersects(&self,r:Rect)->bool{self.hover.as_ref().is_some_and(|s|s.bounds.intersect(r).is_some())}
    pub fn effects_intersect(&self,r:Rect)->bool{self.effects.iter().any(|e|e.bounds.intersect(r).is_some())}
    pub fn shutter_intersects(&self,r:Rect)->bool{self.shutter.as_ref().is_some_and(|s|s.bounds.intersect(r).is_some())}
    pub fn banner_intersects(&self,p:&Painter,r:Rect)->bool{self.save_error.is_some()&&banner(p).is_some_and(|b|b.intersect(r).is_some())}
}
/// Window expose/restore requires presentation of the retained valid texture even
/// with no changed pixels. This does not require software paint or texture upload.
pub fn present_required(damage:&Damage,force_present:bool)->bool{force_present||!damage.is_empty()}
/// Expose/restore changes presentation, not gameplay observations; resize is handled separately.
pub fn presentation_only_window(event:sdl2::event::WindowEvent)->bool{matches!(event,sdl2::event::WindowEvent::Exposed|sdl2::event::WindowEvent::Restored)}
/// Exact emitted effect tile descriptors, conservatively expanded for glyph overhang.
/// Visible-cell lookup uses one O(C) table, not one whole-view scan per effect tile.
pub fn effects(p:&Painter,v:&View,cues:&[crate::effects::Cue],now:f64,reduced:bool)->Vec<Effect>{
    if cues.is_empty(){return Vec::new();}
    let mut known=vec![false;v.width*v.height];let mut actors=vec![None;known.len()];for c in &v.cells{if c.x<v.width&&c.y<v.height{let at=c.y*v.width+c.x;known[at]|=c.current;if c.actor_id.is_some(){actors[at]=Some(c);}}}
    let(cw,ch,ox,oy)=p.map_layout(v.width,v.height);let mut tiles=std::collections::BTreeMap::new();
    for cue in cues.iter().filter(|c|c.event.depth==v.depth){for tile in crate::effects::tiles(cue,now,reduced){
        if tile.x<0||tile.y<0||tile.x>=v.width as i32||tile.y>=v.height as i32||!known[tile.y as usize*v.width+tile.x as usize]{continue;}
        let index=tile.y as usize*v.width+tile.x as usize;
        if tiles.get(&index).is_none_or(|old:&crate::effects::Tile|tile.light>=old.light){tiles.insert(index,tile);}
    }}
    tiles.into_values().filter_map(|t|{let pad=cw.max(ch).max(4).saturating_mul(t.glyph.as_ref().map_or(1,|g|g.chars().count().max(1)as i32));p.logical_damage_rect(ox+t.x*cw-pad,oy+t.y*ch-pad,cw+2*pad,ch+2*pad).map(|mut bounds|{if let Some(c)=actors[t.y as usize*v.width+t.x as usize]{if let Some(actor)=p.cell_damage_rect(c,v.width,v.height){bounds=bounds.union(actor);}}Effect{x:t.x,y:t.y,color:t.color,light:t.light.to_bits(),glyph:t.glyph,bounds}})}).collect()
}
/// Conservative maximum tooltip footprint, using its actual clipped viewport geometry.
/// Layout wrapping remains in the existing painter; no duplicate prose wrapping here.
pub fn hover(p:&Painter,report:&crate::inspection_ui::Report,tile:(i32,i32,i32,i32))->Option<Hover>{
    if !crate::inspection_ui::hover_available(report){return None;}
    let r=crate::modal_ui::hover_bounds_painter(p,tile,i32::MAX/4);
    p.logical_damage_rect(r.0,8,r.2,p.logical_height()-16).map(|bounds|Hover{report:report.clone(),tile,bounds})
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn expose_does_not_request_observation_or_resize_work(){use sdl2::event::WindowEvent::*;assert!(presentation_only_window(Exposed));assert!(presentation_only_window(Restored));assert!(!presentation_only_window(Resized(800,600)));assert!(!presentation_only_window(SizeChanged(800,600)));assert!(!presentation_only_window(FocusGained));}
    #[test]fn expose_presents_without_upload_damage(){let d=Damage::empty(1280,800);assert!(!present_required(&d,false));assert!(present_required(&d,true));assert_eq!(d.upload_bytes(),0);assert!(present_required(&Damage::full(1280,800),false));}
    #[test]fn empty_stable_overlay_does_not_damage(){let p=Painter::new();let mut t=Tracker::default();assert_eq!(t.changes(&p,&Frame::default()).area(),p.width*p.height);t.commit(Frame::default());assert!(t.changes(&p,&Frame::default()).is_empty());}
    #[test]fn shutter_closing_and_error_removal_restore_old_footprints(){let p=Painter::new();let mut t=Tracker::default();let r=Rect::new(12,6,100,80);t.commit(Frame{shutter:Some(Shutter{bounds:r,offset:0}),save_error:Some("failed".into()),..Default::default()});let d=t.changes(&p,&Frame::default());assert!(d.rects().iter().any(|a|a.intersect(r)==Some(r)));assert!(d.area()<p.width*p.height);}
}
