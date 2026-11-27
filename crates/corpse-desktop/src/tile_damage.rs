//! Separate tile-rule compositor: borrowed paint comparisons, flat reusable indices,
//! exact native row spans. No Scene clones, hashes, tree index or extra framebuffer.
use super::{Painter,View,Cell,Animation};
pub use super::damage::Rect;
#[derive(Default,Debug)]
pub struct RowSpans {
 width:usize,height:usize,rows:Vec<Vec<(usize,usize)>>,area:usize,raw_entries:usize,
 words:Vec<u64>,words_per_row:usize,diagnostics:bool,words_touched:usize,words_scanned:usize,
}
impl RowSpans {
 /// Reuse storage; O(H + H*ceil(W/64)) clear, no RGBA copy.
 pub fn reset(&mut self,w:usize,h:usize){
  self.width=w;self.height=h;self.rows.resize_with(h,Vec::new);
  for row in &mut self.rows{row.clear();}
  self.words_per_row=w/64+usize::from(w%64!=0);
  self.words.resize(self.words_per_row.checked_mul(h).expect("native mask dimensions overflow"),0);
  self.words.fill(0);self.area=0;self.raw_entries=0;self.words_touched=0;self.words_scanned=0;
 }
 /// Exact union: O(clipped rectangle height * covered words). Duplicate rectangles
 /// still OR words, but never allocate/push duplicate intervals or require sorting.
 pub fn add(&mut self,r:Rect){
  let Some(r)=r.intersect(Rect::new(0,0,self.width,self.height))else{return;};
  let first=r.x/64;let last=(r.right()-1)/64;
  let first_mask=u64::MAX<<(r.x%64);
  let end=r.right()%64;let last_mask=if end==0{u64::MAX}else{(1u64<<end)-1};
  self.raw_entries+=r.h;
  if self.diagnostics{self.words_touched+=r.h*(last-first+1);}
  for y in r.y..r.bottom(){let base=y*self.words_per_row;
   if first==last{self.words[base+first]|=first_mask&last_mask;}
   else{self.words[base+first]|=first_mask;for word in &mut self.words[base+first+1..base+last]{*word=u64::MAX;}self.words[base+last]|=last_mask;}
  }
 }
 /// O(H*ceil(W/64) + emitted word-runs); adjacent word-runs join without sorting.
 /// Final partial-word bits are never set by add; the width clamp is defensive.
 pub fn finish(&mut self){
  self.area=0;if self.diagnostics{self.words_scanned=self.words.len();}
  for(y,row)in self.rows.iter_mut().enumerate(){row.clear();
   for (i,&word)in self.words[y*self.words_per_row..(y+1)*self.words_per_row].iter().enumerate(){
    let mut remaining=word;let mut offset=0usize;
    while remaining!=0{
     let zeros=remaining.trailing_zeros()as usize;remaining>>=zeros;offset+=zeros;
     let ones=remaining.trailing_ones()as usize;
     let a=i*64+offset;let b=(a+ones).min(self.width);
     if a<b{if let Some(last)=row.last_mut().filter(|last|last.1==a){last.1=b;}else{row.push((a,b));}self.area+=b-a;}
     if ones==64{break;}remaining>>=ones;offset+=ones;
    }
   }
  }
 }
 pub fn rows(&self)->&[Vec<(usize,usize)>]{&self.rows}
 pub fn width(&self)->usize{self.width}pub fn height(&self)->usize{self.height}
 pub fn area(&self)->usize{self.area}pub fn is_empty(&self)->bool{self.area==0}
 pub fn contains(&self,x:usize,y:usize)->bool{self.rows.get(y).is_some_and(|r|{let i=r.partition_point(|&(_,b)|b<=x);r.get(i).is_some_and(|&(a,b)|a<=x&&x<b)})}
 pub fn intersects(&self,r:Rect)->bool{let Some(r)=r.intersect(Rect::new(0,0,self.width,self.height))else{return false;};(r.y..r.bottom()).any(|y|{let row=&self.rows[y];let i=row.partition_point(|&(_,b)|b<=r.x);row.get(i).is_some_and(|&(a,_)|a<r.right())})}
 /// O(H+S) output; exact vertical merging only for identical consecutive row spans.
 pub fn upload_rectangles(&self)->Vec<Rect>{let mut out=Vec::new();let mut y=0;while y<self.height{let mut end=y+1;while end<self.height&&self.rows[end]==self.rows[y]{end+=1;}for &(a,b)in &self.rows[y]{out.push(Rect::new(a,y,b-a,end-y));}y=end;}out}
}
pub struct Segments<'a>{row:Option<&'a[(usize,usize)]>,at:usize,l:i32,r:i32,done:bool}
impl Iterator for Segments<'_>{type Item=(i32,i32);fn next(&mut self)->Option<Self::Item>{if let Some(row)=self.row{while let Some(&(a,b))=row.get(self.at){self.at+=1;if a>=self.r.max(0)as usize{return None;}let l=self.l.max(a as i32);let r=self.r.min(b as i32);if l<r{return Some((l,r));}}None}else if !self.done&&self.l<self.r{self.done=true;Some((self.l,self.r))}else{None}}}
pub fn row_segments(mask:Option<&RowSpans>,y:i32,l:i32,r:i32)->Segments<'_>{let row=mask.map(|m|m.rows.get(y as usize).map(Vec::as_slice).unwrap_or(&[]));let at=row.map_or(0,|row|row.partition_point(|&(_,b)|b<=l.max(0)as usize));Segments{row,at,l,r,done:false}}
#[derive(Default,Clone,Copy,Debug)]
pub struct Eligibility{pub enabled:bool,pub both_no_paint_layers:u64,/// Equal paint with no LOCAL Cell animation/pulse; global animation can override.
 pub same_paint_nonanimated:u64,pub memory_transition:u64,pub changed_colors:u64,pub changed_other_paint:u64,pub animated_candidates:u64,pub global_animation_candidates:u64,pub overlay_marks:u64,pub spill_marks:u64,pub compared_layers:u64,pub footprint_entries:u64,pub index_capacity_bytes:u64,pub capacity_growth_events:u64,pub span_capacity_bytes:u64,pub raw_span_entries:u64,pub merged_span_entries:u64,pub bitset_words_touched:u64,pub bitset_words_scanned:u64,pub bitset_capacity_bytes:u64}
#[derive(Default)]struct Index{head:Vec<usize>,tail:Vec<usize>,next:Vec<usize>,bounds:Vec<Option<Rect>>,animated:Vec<usize>}
#[derive(Clone,Copy,PartialEq,Default)]struct Geometry{w:usize,h:usize,nw:usize,nh:usize,cw:i32,ch:i32,ox:i32,oy:i32}
impl Geometry{fn new(p:&Painter,v:&View)->Self{let(cw,ch,ox,oy)=p.map_layout(v.width,v.height);Self{w:v.width,h:v.height,nw:p.width,nh:p.height,cw,ch,ox,oy}}}
impl Index{
 fn capacities(&self)->[usize;5]{[self.head.capacity()*std::mem::size_of::<usize>(),self.tail.capacity()*std::mem::size_of::<usize>(),self.next.capacity()*std::mem::size_of::<usize>(),self.bounds.capacity()*std::mem::size_of::<Option<Rect>>(),self.animated.capacity()*std::mem::size_of::<usize>()]}
 fn build(&mut self,p:&Painter,v:&View,g:Geometry){let n=g.w*g.h;self.head.resize(n,usize::MAX);self.head.fill(usize::MAX);self.tail.resize(n,usize::MAX);self.tail.fill(usize::MAX);self.next.resize(v.cells.len(),usize::MAX);self.next.fill(usize::MAX);self.bounds.clear();self.animated.clear();
  let ax=p.profile.number("typography.map.alignX").abs();let ay=p.profile.number("typography.map.alignY").abs();
  for(i,c)in v.cells.iter().enumerate(){
   let spill=c.marker_spill();let extra=(spill as i32).saturating_mul(g.ch.max(7));let e=c.emphasis.max(1.);
   let gx=((g.cw as f64*(e-1.)/2.).ceil()as i32).max(0)+(ax*g.cw as f64/15.).ceil()as i32+3;
   let gy=((g.ch as f64*(e-1.)/2.).ceil()as i32).max(0)+(ay*g.ch as f64/15.).ceil()as i32+3;
   self.bounds.push(p.logical_damage_rect(g.ox+c.x as i32*g.cw-gx-extra,g.oy+c.y as i32*g.ch-gy,g.cw+2*gx+2*extra,g.ch+2*gy+(g.ch/2).max(7)));
   if !c.animations.is_empty()||c.pulse_speed!=0.||c.fire_flicker{self.animated.push(i);}
   if c.x>=g.w||c.y>=g.h{continue;}let at=c.y*g.w+c.x;if self.tail[at]!=usize::MAX{self.next[self.tail[at]]=i;}else{self.head[at]=i;}self.tail[at]=i;
  }
 }
}
fn bits(a:f64,b:f64)->bool{a.to_bits()==b.to_bits()}
fn animation_eq(a:&Animation,b:&Animation)->bool{match(a,b){(Animation::Fire{phase:a,speed:as_,colors:ac},Animation::Fire{phase:b,speed:bs,colors:bc})=>bits(*a,*b)&&bits(*as_,*bs)&&ac==bc,(Animation::Water{phase:a,speed:as_},Animation::Water{phase:b,speed:bs})=>bits(*a,*b)&&bits(*as_,*bs),(Animation::Gas{phase:a,color:ac},Animation::Gas{phase:b,color:bc})|(Animation::Reflection{phase:a,color:ac},Animation::Reflection{phase:b,color:bc})=>bits(*a,*b)&&ac==bc,(Animation::Razorleaves{phase:a,speed:as_,color:ac,opacity:ao},Animation::Razorleaves{phase:b,speed:bs,color:bc,opacity:bo})=>bits(*a,*b)&&bits(*as_,*bs)&&ac==bc&&bits(*ao,*bo),(Animation::WardPulse{speed:a},Animation::WardPulse{speed:b})=>bits(*a,*b),(Animation::Torch,Animation::Torch)=>true,_=>false}}
/// Exact already-observed paint values. No hashes, live state or float normalization.
fn paint_eq(a:&Cell,b:&Cell)->bool{
 if a.cogworks!=b.cogworks || a.bottom_aligned!=b.bottom_aligned { return false; }
 a.display_light==b.display_light&&a.unlit_ink==b.unlit_ink&&a.timeweave==b.timeweave&&a.duel_edges==b.duel_edges&&a.italic==b.italic&&a.non_euclidean==b.non_euclidean&&a.dream_entity==b.dream_entity&&a.fire==b.fire&&a.fire_flicker==b.fire_flicker&&a.material_background==b.material_background&&a.entity_ink==b.entity_ink&&a.x==b.x&&a.y==b.y&&a.glyph==b.glyph&&a.label==b.label&&a.edges==b.edges&&a.current==b.current&&a.direct==b.direct&&a.controllable_ally==b.controllable_ally&&a.heartless==b.heartless&&a.corpse==b.corpse&&a.superscript==b.superscript&&a.number_of_blood==b.number_of_blood&&a.subscript==b.subscript&&a.arrow==b.arrow&&a.fg==b.fg&&a.bg==b.bg&&bits(a.emphasis,b.emphasis)&&bits(a.pulse_speed,b.pulse_speed)&&a.actor_id.map(f64::to_bits)==b.actor_id.map(f64::to_bits)&&match(a.background_overlay,b.background_overlay){(None,None)=>true,(Some((ac,aa)),Some((bc,ba)))=>ac==bc&&bits(aa,ba),_=>false}&&a.underlays.len()==b.underlays.len()&&a.underlays.iter().zip(&b.underlays).all(|((ag,ac,aa),(bg,bc,ba))|ag==bg&&ac==bc&&bits(*aa,*ba))&&a.animations.len()==b.animations.len()&&a.animations.iter().zip(&b.animations).all(|(a,b)|animation_eq(a,b))
}
#[cfg(test)]
mod cogworks_damage_tests {
 use super::*;
 #[test] fn detached_cogworks_marker_invalidates_tile_paint() {
  let a=Cell::default();let mut b=a.clone();b.cogworks=true;
  assert!(!paint_eq(&a,&b));assert!(!paint_eq(&b,&a));
 }
 #[test] fn grass_alignment_invalidates_tile_paint() {
  let a=Cell::default();let mut b=a.clone();b.bottom_aligned=true;
  assert!(!paint_eq(&a,&b));assert!(!paint_eq(&b,&a));
 }
}
#[derive(Default)]pub struct TilePlanner{current:Index,scratch:Index,geometry:Geometry,valid:bool,full:bool,dirty:Vec<bool>,tiles:Vec<usize>,rects:Vec<Rect>,last_time:Option<u64>,pub eligibility:Eligibility}
impl TilePlanner{
 pub fn invalidate(&mut self){self.valid=false;self.full=true;}
 pub fn mark_native_rect(&mut self,r:Rect){self.rects.push(r);if self.eligibility.enabled{self.eligibility.overlay_marks+=1;}}
 fn mark_tile(&mut self,i:usize){if i<self.dirty.len()&&!self.dirty[i]{self.dirty[i]=true;self.tiles.push(i);}}
 fn footprint(&mut self,r:Option<Rect>){if let Some(r)=r{self.rects.push(r);if self.eligibility.enabled{self.eligibility.spill_marks+=1;}}}
 /// O(N+C+B) borrowed comparison plus cached footprint setup; indices only retained.
 pub fn replace_view(&mut self,old:Option<&View>,new:&View,p:&Painter){
  let _compare_timing=corpse_core::timing::span("desktop.tile.compare");let g=Geometry::new(p,new);let old_capacity=self.eligibility.enabled.then(||self.scratch.capacities());{let _index_timing=corpse_core::timing::span("desktop.tile.index_footprints");self.scratch.build(p,new,g);}if self.eligibility.enabled{let after=self.scratch.capacities();self.eligibility.capacity_growth_events+=old_capacity.unwrap().iter().zip(after).filter(|(a,b)|**a<*b).count()as u64;self.eligibility.index_capacity_bytes=(after.iter().sum::<usize>()+self.current.capacities().iter().sum::<usize>())as u64;self.eligibility.footprint_entries+=new.cells.len()as u64;}
  self.dirty.resize(g.w*g.h,false);
  if !self.valid||g!=self.geometry||old.is_none(){self.full=true;}else{let old=old.unwrap();
   for at in 0..self.scratch.head.len(){let mut a=self.current.head[at];let mut b=self.scratch.head[at];let mut equal=true;let mut animated=false;
    if a==usize::MAX&&b==usize::MAX{if self.eligibility.enabled{self.eligibility.both_no_paint_layers+=1;}continue;}
    while a!=usize::MAX&&b!=usize::MAX{let ac=&old.cells[a];let bc=&new.cells[b];animated|=!ac.animations.is_empty()||!bc.animations.is_empty()||ac.pulse_speed!=0.||ac.fire_flicker||bc.pulse_speed!=0.||bc.fire_flicker;
     if self.eligibility.enabled{self.eligibility.compared_layers+=1;if ac.current!=bc.current||ac.direct!=bc.direct{self.eligibility.memory_transition+=1;}if ac.fg!=bc.fg||ac.bg!=bc.bg{self.eligibility.changed_colors+=1;}}
     if !paint_eq(ac,bc){equal=false;}a=self.current.next[a];b=self.scratch.next[b];}
    equal&=a==usize::MAX&&b==usize::MAX;
    if equal {if !animated&&self.eligibility.enabled{self.eligibility.same_paint_nonanimated+=1;}}else{self.mark_tile(at);if self.eligibility.enabled{self.eligibility.changed_other_paint+=1;}
      let mut a=self.current.head[at];while a!=usize::MAX{self.footprint(self.current.bounds[a]);a=self.current.next[a];}let mut b=self.scratch.head[at];while b!=usize::MAX{self.footprint(self.scratch.bounds[b]);b=self.scratch.next[b];}}
   }
   for i in 0..old.cells.len().max(new.cells.len()){let a=old.cells.get(i).map(|c|(c.x,c.y));let b=new.cells.get(i).map(|c|(c.x,c.y));if a!=b{self.footprint(self.current.bounds.get(i).copied().flatten());self.footprint(self.scratch.bounds.get(i).copied().flatten());}}
   // At a fixed presentation clock, custom global lighting/material animation
   // can derive different pixels from different immutable observations even when
   // time-zero Cell paint fields compare equal. Identity is conservative O(1);
   // mark cached old/new footprints only, never re-observe the world here.
   let global_animation=crate::appearance_light::animated(&p.profile)||crate::appearance_material::animated(&p.profile);
   let same_snapshot=match(&old.terrain_snapshot,&new.terrain_snapshot){(None,None)=>true,(Some(a),Some(b))=>std::sync::Arc::ptr_eq(a,b),_=>false};
   if global_animation&&(!same_snapshot||old.terrain_count!=new.terrain_count){
    for i in 0..self.current.bounds.len(){self.footprint(self.current.bounds[i]);}
    for i in 0..self.scratch.bounds.len(){self.footprint(self.scratch.bounds[i]);}
   }
   if old.deep_water != new.deep_water || crate::illumination_ui::screen_tint(old) != crate::illumination_ui::screen_tint(new) { self.full=true; }
   // UI compares borrowed fields; no cloned panel descriptor or Scene.
   let left=p.hud_left();let top=super::chronicle_height(&p.profile)+20;
   if old.log_groups!=new.log_groups||old.chronicle_offset!=new.chronicle_offset{if let Some(r)=p.logical_damage_rect(0,0,left,top){self.mark_native_rect(r);}}
   if !hud_eq(old,new){for r in p.hud_damage(){self.mark_native_rect(r);}}
   if !enemies_eq(old,new){if let Some(r)=p.logical_damage_rect(p.frame().map.2,0,left-p.frame().map.2,p.logical_height()){self.mark_native_rect(r);}}
   if old.provenance!=new.provenance{if let Some(r)=p.logical_damage_rect(0,775,super::WIDTH as i32,25){self.mark_native_rect(r);}}
   // O(K) old/new interaction cells and scratch. Include minimum native strokes,
   // not only tile interiors; fractional-scale corners can cross their boundary.
   if old.path!=new.path||old.target_path!=new.target_path||old.target_area!=new.target_area||old.target_impact!=new.target_impact||old.target_color!=new.target_color||old.possible_exits!=new.possible_exits||old.hover!=new.hover{for v in [old,new]{for &(x,y)in v.path.iter().chain(&v.target_path).chain(&v.target_area).chain(v.target_impact.iter()).chain(&v.possible_exits).chain(v.hover.iter()){if x<g.w&&y<g.h{self.mark_tile(y*g.w+x);self.footprint(p.interaction_damage_rect(g.ox+x as i32*g.cw,g.oy+y as i32*g.ch,g.cw,g.ch));}}}}
   if old.ability_overlays!=new.ability_overlays {self.full=true;}
   if old.perception_boundaries!=new.perception_boundaries {
    for v in [old,new] {for b in &v.perception_boundaries {for r in crate::perception_ui::bounds(p,v,b){self.mark_native_rect(r);}}}
   }
   if old.selected_actor.map(f64::to_bits)!=new.selected_actor.map(f64::to_bits){
    if let Some(id)=old.selected_actor.map(f64::to_bits){for(i,c)in old.cells.iter().enumerate(){if c.actor_id.map(f64::to_bits)==Some(id){self.footprint(self.current.bounds[i]);}}}
    if let Some(id)=new.selected_actor.map(f64::to_bits){for(i,c)in new.cells.iter().enumerate(){if c.actor_id.map(f64::to_bits)==Some(id){self.footprint(self.scratch.bounds[i]);}}}
   }
   if old.dreamwalking!=new.dreamwalking||old.cosmetic_seed!=new.cosmetic_seed||old.fire_seed!=new.fire_seed{for i in 0..g.w*g.h{self.mark_tile(i);}}
  }
  std::mem::swap(&mut self.current,&mut self.scratch);self.geometry=g;self.valid=true;
 }
 pub fn mark_animation(&mut self,v:&View,p:&Painter,now:f64,reduced:bool){let time=if reduced{0.}else{now};if self.last_time==Some(time.to_bits()){return;}if v.dreamwalking||crate::appearance_light::animated(&p.profile)||crate::appearance_material::animated(&p.profile){for i in 0..self.current.bounds.len(){self.footprint(self.current.bounds[i]);if self.eligibility.enabled{self.eligibility.global_animation_candidates+=1;}}}else{for j in 0..self.current.animated.len(){let i=self.current.animated[j];self.footprint(self.current.bounds[i]);if self.eligibility.enabled{self.eligibility.animated_candidates+=1;}}}
  use crate::persistent_ability_overlays::{mark_mind_read, mind_read_ink};
  if !v.ability_overlays.mind_read.is_empty()
    && self.last_time.map(|t| mind_read_ink(f64::from_bits(t))) != Some(mind_read_ink(time)) {
   mark_mind_read(p, v, |rect| self.mark_native_rect(rect));
  }
  if let Some(rect)=crate::game_hud::status_animation_damage(p,v,self.last_time.map(f64::from_bits),time) {
   self.mark_native_rect(rect);
  }
  self.last_time=Some(time.to_bits());
 }
 /// Rebuild exact mask from retained frame marks; repeated calls preserve marks.
 pub fn finish_damage(&mut self,p:&mut Painter){let _span_timing=corpse_core::timing::span("desktop.tile.span_prepare");p.tile_mask.diagnostics=self.eligibility.enabled;p.tile_mask.reset(p.width,p.height);if self.full{p.tile_mask.add(Rect::new(0,0,p.width,p.height));}else{let g=self.geometry;for &i in &self.tiles{if let Some(r)=p.logical_damage_rect(g.ox+(i%g.w)as i32*g.cw,g.oy+(i/g.w)as i32*g.ch,g.cw,g.ch){p.tile_mask.add(r);}}for &r in &self.rects{p.tile_mask.add(r);}}p.tile_mask.finish();if self.eligibility.enabled{self.eligibility.raw_span_entries=p.tile_mask.raw_entries as u64;self.eligibility.bitset_words_touched=p.tile_mask.words_touched as u64;self.eligibility.bitset_words_scanned=p.tile_mask.words_scanned as u64;self.eligibility.bitset_capacity_bytes=(p.tile_mask.words.capacity()*std::mem::size_of::<u64>())as u64;self.eligibility.merged_span_entries=p.tile_mask.rows.iter().map(Vec::len).sum::<usize>()as u64;self.eligibility.span_capacity_bytes=(p.tile_mask.rows.capacity()*std::mem::size_of::<Vec<(usize,usize)>>()+p.tile_mask.rows.iter().map(|r|r.capacity()*std::mem::size_of::<(usize,usize)>()).sum::<usize>())as u64;}p.tile_clip_enabled=true;}
 /// One global-order cell pass. Only footprint-intersecting layers rasterize.
 pub fn paint_base(&mut self,p:&mut Painter,v:&View,notice:&str,now:f64){let _paint_timing=corpse_core::timing::span("desktop.tile.paint_base");if p.tile_mask.is_empty(){return;}p.clip=None;p.damage_clip=None;p.paint_cell_indices=None;std::mem::swap(&mut p.tile_cell_bounds,&mut self.current.bounds);p.game_at(v,notice,false,now);std::mem::swap(&mut p.tile_cell_bounds,&mut self.current.bounds);p.clip=None;}
 pub fn clear_frame(&mut self){for i in self.tiles.drain(..){if i<self.dirty.len(){self.dirty[i]=false;}}self.rects.clear();self.full=false;}
}
fn hud_eq(a:&View,b:&View)->bool{a.pack_hud==b.pack_hud&&a.cogworks_mode==b.cogworks_mode&&a.selected_ability==b.selected_ability&&a.pending_energy_loss.to_bits()==b.pending_energy_loss.to_bits()&&a.resource_changes==b.resource_changes&&a.player_stats.map(f64::to_bits)==b.player_stats.map(f64::to_bits)&&a.domain==b.domain&&a.mode==b.mode&&a.god_labels==b.god_labels&&[a.depth,a.level,a.hp,a.max_hp,a.energy,a.max_energy,a.blessings,a.blessing_goal].map(f64::to_bits)==[b.depth,b.level,b.hp,b.max_hp,b.energy,b.max_energy,b.blessings,b.blessing_goal].map(f64::to_bits)&&a.creatures.len()==b.creatures.len()&&a.player_meters.len()==b.player_meters.len()&&a.player_meters.iter().zip(&b.player_meters).all(|(a,b)|a.name==b.name&&a.duration.map(f64::to_bits)==b.duration.map(f64::to_bits)&&bits(a.reference,b.reference)&&a.reference_known==b.reference_known&&bits(a.percent,b.percent))&&a.player_status_offset==b.player_status_offset&&a.total_abilities==b.total_abilities&&a.abilities.len()==b.abilities.len()&&a.abilities.iter().zip(&b.abilities).all(|(a,b)|a.indicator==b.indicator&&a.fortunes==b.fortunes&&a.reserve==b.reserve&&bits(a.pending_reserve_loss,b.pending_reserve_loss)&&a.name==b.name&&a.cost==b.cost&&a.state==b.state&&a.mode==b.mode&&a.color==b.color&&a.active==b.active&&a.supported==b.supported)}
fn enemies_eq(a:&View,b:&View)->bool{a.ground_entities==b.ground_entities&&a.creatures.len()==b.creatures.len()&&a.creatures.iter().zip(&b.creatures).all(|(a,b)|bits(a.id,b.id)&&a.has_heart==b.has_heart&&a.name==b.name&&a.glyph==b.glyph&&a.state==b.state&&a.perception_range==b.perception_range&&a.color==b.color&&a.identity_outline==b.identity_outline&&bits(a.distance,b.distance)&&[a.hp,a.max_hp,a.energy,a.max_energy].map(f64::to_bits)==[b.hp,b.max_hp,b.energy,b.max_energy].map(f64::to_bits)&&a.shared_resources==b.shared_resources&&a.uses_energy==b.uses_energy&&a.invulnerable==b.invulnerable&&a.statuses==b.statuses&&a.meters==b.meters)}
#[cfg(test)]
#[path="tile_damage_tests.rs"]
mod tests;
