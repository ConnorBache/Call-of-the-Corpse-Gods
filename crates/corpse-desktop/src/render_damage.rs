//! Native-pixel damage and retained base buffer. No SDL, world or pixel-diff discovery.
//! Rectangles are half-open, clipped to a known framebuffer, deterministically coalesced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect { pub x:usize, pub y:usize, pub w:usize, pub h:usize }
impl Rect {
    pub fn new(x:usize,y:usize,w:usize,h:usize)->Self {Self{x,y,w,h}}
    pub fn right(self)->usize {self.x.saturating_add(self.w)}
    pub fn bottom(self)->usize {self.y.saturating_add(self.h)}
    pub fn area(self)->usize {self.w.saturating_mul(self.h)}
    pub fn intersect(self,b:Self)->Option<Self>{let x=self.x.max(b.x);let y=self.y.max(b.y);let r=self.right().min(b.right());let d=self.bottom().min(b.bottom());(x<r&&y<d).then(||Self::new(x,y,r-x,d-y))}
    pub fn union(self,b:Self)->Self{let x=self.x.min(b.x);let y=self.y.min(b.y);Self::new(x,y,self.right().max(b.right())-x,self.bottom().max(b.bottom())-y)}
    pub fn touches(self,b:Self)->bool {self.x<=b.right()&&b.x<=self.right()&&self.y<=b.bottom()&&b.y<=self.bottom()}
    pub fn contains(self,x:usize,y:usize)->bool{x>=self.x&&x<self.right()&&y>=self.y&&y<self.bottom()}
}
/// At most8 non-overlapping regions. Coalescing can include an unchanged gap;
/// fallback is explicit and counted, not a full-frame byte comparison.
#[derive(Clone,Debug)]
pub struct Damage { width:usize,height:usize,rects:Vec<Rect>,pub coalesced_to_bounds:bool }
impl Damage {
    pub fn empty(width:usize,height:usize)->Self{Self{width,height,rects:Vec::new(),coalesced_to_bounds:false}}
    pub fn full(width:usize,height:usize)->Self{let mut s=Self::empty(width,height);s.add(Rect::new(0,0,width,height));s}
    /// O(R²) usual insertion, R<=8. Actual overlap is conservatively bounded as
    /// before; mere contact merges only when it adds no unchanged pixels.
    /// At nine regions the bounded pair search is O(R^4), R<=9, with O(1) scratch.
    pub fn add(&mut self,rect:Rect){
        let Some(mut rect)=rect.intersect(Rect::new(0,0,self.width,self.height)) else{return;};
        let mut i=0;
        while i<self.rects.len(){
            let other=self.rects[i];
            let overlaps=rect.intersect(other).is_some();
            let aligned_touch=rect.touches(other)&&rect.union(other).area()==rect.area().saturating_add(other.area());
            if overlaps||aligned_touch{rect=rect.union(self.rects.remove(i));i=0;}else{i+=1;}
        }
        self.rects.push(rect);
        if self.rects.len()>8 {
            // Choose the smallest extra-area union, including every region that
            // its enlarged bounds would overlap. Closure preserves disjointness.
            let mut best:Option<(usize,usize,u16,Rect)>=None;
            for a in 0..self.rects.len(){for b in a+1..self.rects.len(){
                let mut mask=(1u16<<a)|(1u16<<b);
                let mut bounds=self.rects[a].union(self.rects[b]);
                loop {
                    let previous=mask;
                    for (i,r) in self.rects.iter().enumerate(){
                        if mask&(1u16<<i)==0&&bounds.intersect(*r).is_some(){mask|=1u16<<i;bounds=bounds.union(*r);}
                    }
                    if mask==previous{break;}
                }
                let covered:usize=self.rects.iter().enumerate().filter(|(i,_)|mask&(1u16<<*i)!=0).map(|(_,r)|r.area()).sum();
                let added=bounds.area().saturating_sub(covered);
                let candidate=(added,bounds.area(),mask,bounds);
                if best.as_ref().is_none_or(|old|(added,bounds.area(),mask)<(old.0,old.1,old.2)){best=Some(candidate);}
            }}
            let (_,_,mask,bounds)=best.expect("nine regions contain a pair");
            let mut i=0;self.rects.retain(|_|{let keep=mask&(1u16<<i)==0;i+=1;keep});
            self.rects.push(bounds);self.coalesced_to_bounds=true;
        }
        self.rects.sort_unstable_by_key(|r|(r.y,r.x,r.h,r.w));
    }
    pub fn extend(&mut self,other:&Self){assert_eq!((self.width,self.height),(other.width,other.height));for r in &other.rects{self.add(*r);}self.coalesced_to_bounds|=other.coalesced_to_bounds;}
    pub fn rects(&self)->&[Rect]{&self.rects}
    pub fn is_empty(&self)->bool{self.rects.is_empty()}
    pub fn area(&self)->usize{self.rects.iter().map(|r|r.area()).sum()}
    pub fn upload_bytes(&self)->usize{self.area().saturating_mul(4)}
}
/// Exact cached game composition beneath transient overlays. One extra4P-byte
/// framebuffer; capture/restore copy only requested Q pixels, never compare pixels.
#[derive(Default)]
pub struct RetainedBase { width:usize,height:usize,pixels:Vec<u8>,valid:bool }
impl RetainedBase {
    pub fn invalidate(&mut self){self.valid=false;}
    pub fn ready(&self,width:usize,height:usize)->bool{self.valid&&self.width==width&&self.height==height}
    /// O(P) allocation only on framebuffer change; initial capture must be full.
    pub fn capture(&mut self,pixels:&[u8],damage:&Damage)->Result<usize,&'static str>{
        let expected=damage.width.checked_mul(damage.height).and_then(|n|n.checked_mul(4)).ok_or("frame size overflow")?;
        if pixels.len()!=expected{return Err("frame byte length mismatch");}
        if !self.ready(damage.width,damage.height){
            if damage.area()!=damage.width*damage.height{return Err("initial base capture must cover the full frame");}
            self.pixels.resize(expected,0);self.width=damage.width;self.height=damage.height;
        }
        copy_regions(pixels,&mut self.pixels,damage)?;self.valid=true;Ok(damage.area())
    }
    /// O(Q) copy; restoring from the clean lower layer prevents alpha accumulation.
    pub fn restore(&self,pixels:&mut[u8],damage:&Damage)->Result<usize,&'static str>{
        if !self.ready(damage.width,damage.height){return Err("retained base is invalid");}
        copy_regions(&self.pixels,pixels,damage)?;Ok(damage.area())
    }
}
fn copy_regions(source:&[u8],target:&mut[u8],damage:&Damage)->Result<(),&'static str>{
    let expected=damage.width.checked_mul(damage.height).and_then(|n|n.checked_mul(4)).ok_or("frame size overflow")?;
    if source.len()!=expected||target.len()!=expected{return Err("frame byte length mismatch");}
    for r in damage.rects(){for y in r.y..r.bottom(){let start=(y*damage.width+r.x)*4;let end=start+r.w*4;target[start..end].copy_from_slice(&source[start..end]);}}
    Ok(())
}
/// Checked zero-copy source for SDL texture.update(Some(native_rect),slice,pitch).
/// Last row needs only rect width bytes; pitch stays the full framebuffer stride.
pub fn upload_source(pixels:&[u8],width:usize,height:usize,rect:Rect)->Result<(&[u8],usize),&'static str>{
    if rect.w==0||rect.h==0||rect.right()>width||rect.bottom()>height{return Err("invalid upload rectangle");}
    let pitch=width.checked_mul(4).ok_or("pitch overflow")?;
    let expected=pitch.checked_mul(height).ok_or("frame size overflow")?;
    if pixels.len()!=expected{return Err("frame byte length mismatch");}
    let start=rect.y.checked_mul(pitch).and_then(|v|v.checked_add(rect.x*4)).ok_or("upload offset overflow")?;
    let len=(rect.h-1).checked_mul(pitch).and_then(|v|v.checked_add(rect.w*4)).ok_or("upload extent overflow")?;
    Ok((&pixels[start..start+len],pitch))
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn empty_clip_merge_and_bounded_fragmentation(){let mut d=Damage::empty(100,80);assert!(d.is_empty());d.add(Rect::new(99,79,99,99));assert_eq!(d.rects(),&[Rect::new(99,79,1,1)]);d.add(Rect::new(98,78,1,1));assert_eq!(d.area(),2);for i in 0..20{d.add(Rect::new(i*4,0,1,1));}assert!(d.rects().len()<=8);assert!(d.coalesced_to_bounds);for(i,a)in d.rects().iter().enumerate(){for b in &d.rects()[..i]{assert!(a.intersect(*b).is_none());}}}
    #[test]fn restore_only_damage_and_empty_is_zero_work(){let full=Damage::full(7,5);let original:Vec<_>=(0..140).map(|n|n as u8).collect();let mut b=RetainedBase::default();assert_eq!(b.capture(&original,&full),Ok(35));let mut changed=vec![255;140];let mut d=Damage::empty(7,5);d.add(Rect::new(2,1,3,2));assert_eq!(b.restore(&mut changed,&d),Ok(6));for y in 0..5{for x in 0..7{let i=(y*7+x)*4;assert_eq!(&changed[i..i+4],if d.rects()[0].contains(x,y){&original[i..i+4]}else{&[255;4]});}}assert_eq!(b.restore(&mut changed,&Damage::empty(7,5)),Ok(0));b.invalidate();assert!(b.restore(&mut changed,&d).is_err());}
    #[test]fn partial_initial_capture_rejected_and_upload_math_exact(){let pixels=vec![0;7*5*4];let mut d=Damage::empty(7,5);d.add(Rect::new(6,4,1,1));assert!(RetainedBase::default().capture(&pixels,&d).is_err());let(s,p)=upload_source(&pixels,7,5,d.rects()[0]).unwrap();assert_eq!((s.len(),p),(4,28));let(s,p)=upload_source(&pixels,7,5,Rect::new(2,1,3,3)).unwrap();assert_eq!((s.len(),p),(68,28));assert!(upload_source(&pixels,7,5,Rect::new(7,4,1,1)).is_err());assert!(upload_source(&pixels,usize::MAX,5,Rect::new(0,0,1,1)).is_err());}
}

/// Diagnostic clipped-raster loop counts, not unique changed pixels or every
/// upstream glyph/coverage sample. Disabled by default;
/// one branch per clipped primitive/row, never a framebuffer comparison scan.
#[derive(Default,Clone,Copy,Debug)]
pub struct RasterStats {pub enabled:bool,pub pixel_visits:u64,pub pixel_writes:u64,pub cells_considered:u64,pub cells_painted:u64}
impl RasterStats {
    pub fn pixels(&mut self,n:usize){if self.enabled{self.pixel_visits=self.pixel_visits.saturating_add(n as u64);self.pixel_writes=self.pixel_writes.saturating_add(n as u64);}}
}

use super::{Cell as PaintCell, Painter, View, Rgb};
use std::collections::{BTreeMap,BTreeSet};
#[derive(Clone,PartialEq)]
struct AbilityInk {indicator:Option<crate::ability_ui::CustomIndicator>,fortunes:Option<(u8,u8)>,reserve:Option<(u64,u64)>,pending:u64,name:String,cost:String,state:String,mode:String,color:Rgb,active:bool,supported:bool}
#[derive(Clone,PartialEq)]
struct HudKey {cogworks_mode:u8,selected_ability:Option<String>,pending_energy_loss:u64,resource_changes:[String;3],domain:String,mode:String,gods:Vec<(String,Rgb)>,numbers:[u64;10],stats:[u64;5],creatures:usize,meters:Vec<crate::statuses_ui::Meter>,status_offset:usize,abilities:Vec<AbilityInk>,total:usize}
impl HudKey {fn from(v:&View)->Self{Self{cogworks_mode:v.cogworks_mode,selected_ability:v.selected_ability.clone(),pending_energy_loss:v.pending_energy_loss.to_bits(),resource_changes:v.resource_changes.clone(),domain:v.domain.clone(),mode:v.mode.clone(),gods:v.god_labels.clone(),numbers:[v.depth.to_bits(),v.display_depth.to_bits(),v.max_depth as u64,v.level.to_bits(),v.hp.to_bits(),v.max_hp.to_bits(),v.energy.to_bits(),v.max_energy.to_bits(),v.blessings.to_bits(),v.blessing_goal.to_bits()],stats:v.player_stats.map(f64::to_bits),creatures:v.creatures.len(),meters:v.player_meters.clone(),status_offset:v.player_status_offset,abilities:v.abilities.iter().map(|a|AbilityInk{indicator:a.indicator,fortunes:a.fortunes,reserve:a.reserve.map(|(n,m)|(n.to_bits(),m.to_bits())),pending:a.pending_reserve_loss.to_bits(),name:a.name.clone(),cost:a.cost.clone(),state:a.state.clone(),mode:a.mode.clone(),color:a.color,active:a.active,supported:a.supported}).collect(),total:v.total_abilities}}}
#[derive(Clone,PartialEq)]
struct EnemyInk {perception_range:Option<String>,identity_outline:Option<Rgb>,id:u64,name:String,glyph:String,state:String,color:Rgb,distance:u64,resources:[u64;4],shared:bool,uses_energy:bool,invulnerable:bool,statuses:Vec<crate::statuses_ui::Status>,meters:Vec<crate::statuses_ui::Meter>}
#[derive(Clone,PartialEq)]
struct Selection {toolbar:Option<usize>,bestiary:bool,enemy:Option<u64>,ground:Option<(usize,usize)>,ability:Option<usize>,chronicle:bool,enemy_offset:usize}
impl Selection {fn from(p:&Painter)->Self{Self{toolbar:p.toolbar_focus,bestiary:p.bestiary_unread,enemy:p.enemy_hover,ground:p.ground_hover,ability:p.ability_hover,chronicle:p.chronicle_hover,enemy_offset:p.enemy_list_offset}}}
struct Scene {
    order:Vec<(usize,usize)>, stacks:BTreeMap<(usize,usize),Vec<PaintCell>>, bounds:BTreeMap<(usize,usize),Rect>,
    buckets:BTreeMap<(usize,usize),Vec<usize>>, animated:Vec<Rect>,
    chronicle:Vec<crate::chronicle::Group>,chronicle_offset:usize,hud:HudKey,enemies:Vec<EnemyInk>,ground:Vec<super::enemy_list::GroundEntity>,
    paths:Vec<(usize,usize)>,targets:Vec<(usize,usize)>,target_area:Vec<(usize,usize)>,target_impact:Option<(usize,usize)>,target_color:Option<[u8;3]>,exits:Vec<(usize,usize)>,hover:Option<(usize,usize)>,
    perception_boundaries:Vec<crate::perception_ui::Boundary>,
    deep_water:bool,immersion_color:Option<Rgb>,dream:bool,seeds:(u32,u32),provenance:String,
}
#[cfg(test)]
mod cogworks_scene_tests {
 use super::*;
 #[test] fn detached_cogworks_marker_changes_retained_cell_stack() {
  let run:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
  let mut view=crate::observe::observe(&run);let painter=Painter::new();
  let before=Scene::capture(&painter,&view);view.cells[0].cogworks=true;
  let after=Scene::capture(&painter,&view);assert_ne!(before.stacks,after.stacks);
 }
}
impl Scene {
    /// O(C log C + copied paint data + spatial bucket entries), only on a new View.
    /// No world state or pixel raster is inspected to discover changes.
    fn capture(p:&Painter,v:&View)->Self {
        let mut stacks=BTreeMap::<_,Vec<_>>::new();let mut bounds=BTreeMap::new();let mut buckets=BTreeMap::<_,Vec<_>>::new();let mut animated=Vec::new();
        for(i,c)in v.cells.iter().enumerate(){
            let at=(c.x,c.y);stacks.entry(at).or_default().push(c.clone());
            if let Some(b)=p.cell_damage_rect(c,v.width,v.height){
                bounds.entry(at).and_modify(|r:&mut Rect|*r=r.union(b)).or_insert(b);
                for by in b.y/32..=(b.bottom()-1)/32{for bx in b.x/32..=(b.right()-1)/32{buckets.entry((bx,by)).or_default().push(i);}}
                if !c.animations.is_empty()||c.pulse_speed!=0. {animated.push(b);}
            }
        }
        Self{order:v.cells.iter().map(|c|(c.x,c.y)).collect(),stacks,bounds,buckets,animated,chronicle:v.log_groups.clone(),chronicle_offset:v.chronicle_offset,hud:HudKey::from(v),ground:v.ground_entities.clone(),enemies:v.creatures.iter().map(|a|EnemyInk{perception_range:a.perception_range.clone(),identity_outline:a.identity_outline,id:a.id.to_bits(),name:a.name.clone(),glyph:a.glyph.clone(),state:a.state.clone(),color:a.color,distance:a.distance.to_bits(),resources:[a.hp.to_bits(),a.max_hp.to_bits(),a.energy.to_bits(),a.max_energy.to_bits()],shared:a.shared_resources,uses_energy:a.uses_energy,invulnerable:a.invulnerable,statuses:a.statuses.clone(),meters:a.meters.clone()}).collect(),paths:v.path.clone(),targets:v.target_path.clone(),target_area:v.target_area.clone(),target_impact:v.target_impact,target_color:v.target_color,exits:v.possible_exits.clone(),hover:v.hover,perception_boundaries:v.perception_boundaries.clone(),deep_water:v.deep_water,immersion_color:crate::illumination_ui::immersion_color(v),dream:v.dreamwalking,seeds:(v.cosmetic_seed,v.fire_seed),provenance:v.provenance.clone()}
    }
    /// O(covered32px buckets + K log K), K intersecting cell stack indices.
    /// Sorted indices preserve global terrain/item/actor z-order exactly.
    fn indices(&self,r:Rect)->Vec<usize>{let mut out=BTreeSet::new();for by in r.y/32..=(r.bottom()-1)/32{for bx in r.x/32..=(r.right()-1)/32{if let Some(ids)=self.buckets.get(&(bx,by)){out.extend(ids);}}}out.into_iter().copied().collect()}
    fn add_interactions(&self,p:&Painter,v:&View,d:&mut Damage){
        let(cw,ch,ox,oy)=p.map_layout(v.width,v.height);
        for &(x,y) in self.paths.iter().chain(&self.targets).chain(&self.target_area).chain(self.target_impact.iter()).chain(&self.exits).chain(self.hover.iter()){
            if let Some(r)=p.interaction_damage_rect(ox+x as i32*cw,oy+y as i32*ch,cw,ch){d.add(r);}
        }
    }
}
/// Retained GAME base only. Caller composes transient overlays separately in
/// original z-order after restoring their old/new damage from this clean base.
#[derive(Default)]
pub struct RetainedGame {base:RetainedBase,scene:Option<Scene>,selection:Option<Selection>,size:(usize,usize),layout:(i32,i32,i32,i32),now:Option<u64>,notice:String}
impl RetainedGame {
    pub fn invalidate(&mut self){self.base.invalidate();self.scene=None;self.selection=None;self.now=None;}
    pub fn restore(&self,p:&mut Painter,d:&Damage)->Result<(),&'static str>{self.base.restore(&mut p.pixels,d).map(|_|())}
    /// Discover damage from detached paint descriptors on view changes and cached
    /// animated candidates on time changes. No full repaint/pixel diff is used.
    /// force_full is mandatory for font/palette/profile generation changes.
    pub fn update(&mut self,p:&mut Painter,v:&View,notice:&str,now:f64,view_changed:bool,force_full:bool)->Result<Damage,&'static str>{
        let discovery_timing=corpse_core::timing::span("desktop.paint.damage_discovery");
        let layout=p.map_layout(v.width,v.height);let size=(p.width,p.height);
        let full=force_full||!self.base.ready(size.0,size.1)||self.scene.is_none()||self.size!=size||self.layout!=layout;
        let mut damage=if full{Damage::full(size.0,size.1)}else{Damage::empty(size.0,size.1)};
        let next=(full||view_changed).then(||Scene::capture(p,v));
        let left=p.hud_left();let top=super::chronicle_height(&p.profile)+20;let entities=p.frame().map.2;
        let add=|d:&mut Damage,x,y,w,h|{if let Some(r)=p.logical_damage_rect(x,y,w,h){d.add(r);}};
        if !full {
            let previous=self.scene.as_ref().unwrap();
            if let Some(next)=&next {
                if previous.deep_water != next.deep_water || previous.immersion_color != next.immersion_color { damage = Damage::full(size.0,size.1); }
                // Cross-coordinate glyph overhang means global layer ordering matters.
                // Mark changed slots only; moving one actor must not dirty the whole map.
                for i in 0..previous.order.len().max(next.order.len()) {
                    let old=previous.order.get(i);let new=next.order.get(i);
                    if old!=new {if let Some(r)=old.and_then(|at|previous.bounds.get(at)){damage.add(*r);}if let Some(r)=new.and_then(|at|next.bounds.get(at)){damage.add(*r);}}
                }
                for(at,stack)in &previous.stacks{if next.stacks.get(at)!=Some(stack){if let Some(r)=previous.bounds.get(at){damage.add(*r);}if let Some(r)=next.bounds.get(at){damage.add(*r);}}}
                for(at,_)in &next.stacks{if !previous.stacks.contains_key(at){if let Some(r)=next.bounds.get(at){damage.add(*r);}}}
                if previous.chronicle!=next.chronicle||previous.chronicle_offset!=next.chronicle_offset{add(&mut damage,0,0,left,top);}
                if previous.hud!=next.hud{add(&mut damage,left,0,p.logical_width()-left,p.logical_height());}
                if previous.enemies!=next.enemies||previous.ground!=next.ground{add(&mut damage,entities,0,left-entities,p.logical_height());}
                if previous.provenance!=next.provenance{add(&mut damage,0,775,super::WIDTH as i32,25);}
                if previous.paths!=next.paths||previous.targets!=next.targets||previous.target_area!=next.target_area||previous.target_impact!=next.target_impact||previous.target_color!=next.target_color||previous.exits!=next.exits||previous.hover!=next.hover{previous.add_interactions(p,v,&mut damage);next.add_interactions(p,v,&mut damage);}
                if previous.perception_boundaries!=next.perception_boundaries {
                    for scene in [previous,next] {for b in scene.perception_boundaries.iter().filter(|b|[p.enemy_hover,self.selection.as_ref().and_then(|s|s.enemy)].contains(&Some(b.actor))){for r in crate::perception_ui::bounds(p,v,b){damage.add(r);}}}
                }
                if previous.dream!=next.dream||previous.seeds!=next.seeds{add(&mut damage,layout.2,layout.3,layout.0*v.width as i32,layout.1*v.height as i32);}
            }
            let selection=Selection::from(p);
            if let Some(old)=&self.selection{
                if old.toolbar!=selection.toolbar||old.bestiary!=selection.bestiary{add(&mut damage,0,p.toolbar_y()-24,p.logical_width(),p.logical_height()-p.toolbar_y()+24);}
                if old.ground!=selection.ground||old.enemy!=selection.enemy||old.enemy_offset!=selection.enemy_offset{add(&mut damage,entities,0,left-entities,p.logical_height());}
                if old.enemy!=selection.enemy {
                    for id in [old.enemy,selection.enemy] {
                        for b in previous.perception_boundaries.iter().filter(|b|Some(b.actor)==id){for r in crate::perception_ui::bounds(p,v,b){damage.add(r);}}
                        if let Some(b)=crate::perception_ui::selected(v,id){for r in crate::perception_ui::bounds(p,v,b){damage.add(r);}}
                    }
                    // Two visible IDs at most; repaint both old/new map footprints on hover.
                    for cell in &v.cells {if cell.actor_id.is_some_and(|id|[old.enemy,selection.enemy].contains(&Some(id.to_bits()))) {
                        if let Some(rect)=p.cell_damage_rect(cell,v.width,v.height){damage.add(rect);}
                    }}
                }
                if old.ground!=selection.ground {
                    for (x,y) in [old.ground,selection.ground].into_iter().flatten() {
                        if let Some(r)=p.interaction_damage_rect(layout.2+x as i32*layout.0,layout.3+y as i32*layout.1,layout.0,layout.1){damage.add(r);}
                    }
                }
                if old.ability!=selection.ability{add(&mut damage,left,crate::ability_ui::top(&p.profile)-4,p.logical_width()-left,p.logical_height()-crate::ability_ui::top(&p.profile)+4);}
                if old.chronicle!=selection.chronicle{add(&mut damage,0,0,left,top);}
            }
            if self.notice!=notice{if let Some(rect)=p.footer_damage(){damage.add(rect);}}
            if self.now!=Some(now.to_bits()) || (view_changed && (crate::appearance_light::animated(&p.profile)||crate::appearance_material::animated(&p.profile))) {
                let scene=next.as_ref().unwrap_or(previous);
                if v.dreamwalking||crate::appearance_light::animated(&p.profile)||crate::appearance_material::animated(&p.profile){
                    // These existing paths change every affected terrain descriptor.
                    for r in scene.bounds.values(){damage.add(*r);}
                }else{for r in &scene.animated{damage.add(*r);}}
                if p.profile.number("path.animationSpeed")>0. {scene.add_interactions(p,v,&mut damage);}
            }
        }
        if let Some(next)=next{self.scene=Some(next);}
        self.size=size;self.layout=layout;
        drop(discovery_timing);
        let paint_timing=corpse_core::timing::span("desktop.paint.base_paint");
        // Recompose each disjoint region once; only indexed intersecting cells are rasterized.
        for rect in damage.rects(){
            p.clip=None;
            p.damage_clip=Some(*rect);
            p.paint_cell_indices=Some(self.scene.as_ref().unwrap().indices(*rect));
            p.game_at(v,notice,false,now);
        }
        p.clip=None;p.damage_clip=None;p.paint_cell_indices=None;
        drop(paint_timing);
        if !damage.is_empty(){let _capture_timing=corpse_core::timing::span("desktop.paint.base_capture");self.base.capture(&p.pixels,&damage)?;}
        self.selection=Some(Selection::from(p));self.now=Some(now.to_bits());self.notice.clear();self.notice.push_str(notice);
        Ok(damage)
    }
}

#[cfg(test)]
#[path="render_damage_tests.rs"]
mod pixel_oracles;

#[cfg(test)]
#[path="render_damage_coalescing_tests.rs"]
mod coalescing_tests;

#[cfg(test)]
#[path="render_damage_metadata_bench.rs"]
mod metadata_bench;
