//! R51+ perception preview. Enclose the complete confidently known sight area
//! from detached current/remembered observation, never hidden world geometry.
use corpse_core::{Actor, Floor, Run, movement::Point, revision51_perception as rules, vision};
use crate::render::{Creature, Painter, View};

pub const INK: [u8;3] = [242,65,79];
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Edge { pub x:usize, pub y:usize, pub vertical:bool }
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct Boundary { pub actor:u64, pub edges:Vec<Edge> }

pub fn range_label(run:&Run,actor:&Actor)->Option<String> {
    (rules::enabled(run)&&!corpse_core::dreamwalking::is_nightmare(actor)).then(||format!("Perception range: {}",number(rules::perception_range(
        corpse_core::combat::effective_stats(run,actor).perception))))
}
fn number(n:f64)->String {
    // Keep fractional modifiers meaningful, but avoid floating point noise.
    let s=format!("{n:.2}");s.trim_end_matches('0').trim_end_matches('.').to_owned()
}
pub fn tile_label(run:&Run,cell:&vision::ObservedCell)->Option<String> {
    if !rules::enabled(run)||!cell.current { return None; }
    let n=rules::tile_modifier(cell.illumination?);
    (n<0.).then(||format!("−{} perception range",number(-n)))
}

/// Remembered terrain is usable knowledge; telepathic-only current projections
/// retain the existing optical-preview exclusion. Unknown cells have no tile.
fn known(cell:&vision::ObservedCell)->bool {
    cell.tile.is_some() && ((cell.current&&cell.direct)||(!cell.current&&cell.remembered))
}
/// Current light is exact. Remembered light is not serialized, so only sight
/// that agrees across its full possible interval can extend into remembered cells.
fn light_bounds(run:&Run,cell:&vision::ObservedCell)->Option<(f64,f64)> {
    if !known(cell) {return None;}
    let (low,high)=if cell.current {let l=cell.illumination?;(l,l)} else {(-100.,100.)};
    Some((vision::perception_actor_light(run,&run.player,low,0.),
        vision::perception_actor_light(run,&run.player,high,0.)))
}
/// Two detached opacity bounds. A result is certain only if the known-blocked
/// and unknown-transparent route agree. O(V) setup and storage; no live rays.
struct Geometry { floor:Floor, blocked:Vec<bool>, optimistic:Vec<bool> }
impl Geometry {
    fn new(run:&Run,o:&vision::TerrainObservation)->Option<Self> {
        let current=|i:usize|o.cells.get(i).is_some_and(|c|c.current&&c.direct&&c.tile.is_some());
        let mut floor=Floor::default();floor.tiles=vec!["floor".into();o.width*o.height];
        // Portal identity is admitted only when both endpoints are currently
        // observed. An exposed endpoint with an unknown mate is ambiguous.
        if let Some(f)=corpse_core::dreamwalking::floor_for(run,&run.player) {
            let mut pairs=Vec::new();
            for pair in f.portals.as_ref().into_iter().flatten() {
                let (a,b)=(current(pair.a),current(pair.b));
                if a!=b {return None;}
                if a&&b {pairs.push(pair.clone());}
            }
            floor.portals=pairs.into();
        }
        let optimistic=o.cells.iter().map(|c|known(c)&&
            (matches!(c.tile.as_deref(),Some("wall"|"tree"))||c.door_open==Some(false))).collect::<Vec<_>>();
        let blocked=o.cells.iter().map(|c|!known(c)||optimistic[c.index]
            ||(c.tile.as_deref()==Some("door")&&c.door_open.is_none())).collect();
        Some(Self{floor,blocked,optimistic})
    }
    fn distance(&self,from:Point,to:Point,opaque:&[bool])->f64 {
        let (w,h)=(self.floor.width(),self.floor.height());
        if self.floor.portals.as_ref().is_none_or(|p|p.is_empty()) {
            return if vision::ray(w,h,opaque,from,to) {vision::distance(from,to)} else {f64::INFINITY};
        }
        corpse_core::portals::route_filtered(&self.floor,from,to,false,
            |a,b|vision::ray(w,h,opaque,a,b),
            |a,b|[a,b].into_iter().all(|p|!opaque[p.y as usize*w+p.x as usize]))
            .map_or(f64::INFINITY,|r|r.distance)
    }
}
/// Non-optical rules that depend only on the observed actor and the player's
/// own state, never on hidden terrain/light. Their union is safe over known
/// same-plane cells even when an optical preview is unavailable.
#[derive(Default)]
struct Senses { all_known:bool, mind_radius:Option<f64>, dream_los:bool }
impl Senses {
    fn observe(run:&Run,actor:&Actor)->Self {
        let condition=corpse_core::engine::condition;
        let player=&run.player;
        let all_seeing=condition(actor,"allSeeingReveal")==1.
            &&condition(actor,"allSeeingDepth")==run.depth;
        let alive=!player.dead.value_or(false);
        // Match vision::telepathic_for_owner's target sleep predicate, including
        // resting allies/recovering neutrals. The player's state is not hidden.
        let sleeping=player.state=="sleeping"
            ||(player.state=="ally"&&condition(player,"monsterAllyRest")==1.)
            ||(player.state=="neutral"&&condition(player,"monsterRecoverySleep")==1.);
        let dream=alive&&actor.abilities.iter().any(|a|a=="ursa-dream-see");
        Self {
            all_known:all_seeing||corpse_core::revision50_combat::witch_senses(run,actor,player)
                ||(alive&&(condition(actor,"telepathic")>0.
                    ||(condition(actor,"cameTurns")>0.&&condition(actor,"cameTarget")==player.id)))
                ||(dream&&sleeping),
            mind_radius:(alive&&actor.abilities.iter().any(|a|a=="numo-mind-read"))
                .then(||if actor.active.iter().any(|a|a=="numo-mind-read"){20.}else{5.}),
            dream_los:dream&&actor.active.iter().any(|a|a=="ursa-dream-see"),
        }
    }
    fn any(&self)->bool {self.all_known||self.mind_radius.is_some()||self.dream_los}
    fn sees(&self,from:Point,to:Point,g:Option<&Geometry>)->bool {
        self.all_known||self.mind_radius.is_some_and(|r|vision::distance(from,to)<=r)
            ||(self.dream_los&&g.is_some_and(|g|g.distance(from,to,&g.blocked).is_finite()))
    }
}
/// World-observation-only projection. The additional known-mask/perimeter work
/// is O(V) per creature with E<=4V output edges; detached rays retain their
/// existing O(V*R) / O(V*K²R) cost. Full actor/modifier and memory bounds are in
/// docs/revision52-enemy-perception-overlay.md. Painting never queries the run.
pub fn observe(run:&Run,o:Option<&vision::TerrainObservation>,creatures:&[Creature])->Vec<Boundary> {
    if !rules::enabled(run) {return Vec::new();}
    let Some(o)=o else{return Vec::new();};
    // TerrainObservation may combine planes under these powers but carries no
    // per-cell plane tag. Omit uncertain geometry rather than mix their walls.
    if corpse_core::dreamwalking::active(run)
        ||creatures.iter().any(|c|run.actors_at(run.depth).iter().find(|a|a.id==c.id)
            .is_some_and(|a|a.perfect.value_or(false)
                &&!corpse_core::dreamwalking::same_plane(run,&run.player,a))) {return Vec::new();}
    // Moving player light cannot be reconstructed from the detached field. It
    // does not invalidate a known special sense which ignores illumination.
    let optical_supported=!(run.player.abilities.iter().any(|a|a=="enemy:beast:darkness-aura")
        ||(run.player.abilities.iter().any(|a|a=="anavani-shine")
            &&!run.player.active.iter().any(|a|a=="anavani-shine"))
        ||corpse_core::revision50_rings::has_effect(run,&run.player,"light"));
    let geometry=Geometry::new(run,o);
    let light=o.cells.iter().map(|c|light_bounds(run,c)).collect::<Vec<_>>();
    let stealth=corpse_core::engine::condition(&run.player,"stealth");
    let submerged=corpse_core::engine::condition(&run.player,"submergedStealth");
    let invisible=corpse_core::engine::condition(&run.player,"invisible")>0.;
    creatures.iter().filter_map(|row| {
        let actor=run.actors_at(run.depth).iter().find(|a|a.id==row.id)?;
        // A disguise/hallucination must never disclose its underlying senses.
        if row.shared_resources||actor.state=="ally"||row.state=="sensed"||row.hallucination.is_some()
            ||crate::creature_presentation::project(run,actor).is_some(){return None;}
        let senses=Senses::observe(run,actor);
        if (!optical_supported||geometry.is_none())&&!senses.any() {return None;}
        let from=Point{x:actor.x as i32,y:actor.y as i32};
        let at=actor.y as usize*o.width+actor.x as usize;
        if !o.cells.get(at).is_some_and(|c|c.current&&c.direct)
            ||!corpse_core::dreamwalking::same_plane(run,&run.player,actor)
            ||corpse_core::dreamwalking::is_nightmare(actor) {return None;}
        let perception=corpse_core::combat::effective_stats(run,actor).perception;
        let states=o.cells.iter().map(|c| {
            if !known(c) {return None;}
            let to=Point{x:(c.index%o.width)as i32,y:(c.index/o.width)as i32};
            if senses.sees(from,to,geometry.as_ref()) {return Some(true);}
            if !optical_supported {return None;}
            let g=geometry.as_ref()?;
            let (darkest,brightest)=light[c.index]?;
            if invisible {return Some(false);}
            // Without portals, range alone can prove an excluded tile. Avoid
            // ray walks outside the finite creature radius. Positive light still
            // requires LOS, and known portal shortcuts must use route distance.
            if g.floor.portals.as_ref().is_none_or(|p|p.is_empty())
                &&!rules::creature_in_range(perception,vision::distance(from,to),stealth,submerged,brightest){return Some(false);}
            let definite=rules::creature_in_range(perception,g.distance(from,to,&g.blocked),stealth,submerged,darkest);
            let possible=rules::creature_in_range(perception,g.distance(from,to,&g.optimistic),stealth,submerged,brightest);
            (definite==possible).then_some(definite)
        }).collect::<Vec<_>>();
        Some(Boundary{actor:row.id.to_bits(),edges:edges(o.width,o.height,&states)})
    }).collect()
}
/// Perimeter of the confirmed-visible union, including holes and map limits.
/// Unknown/ambiguous cells are outside this knowledge area, not promised safe.
/// O(V) time, O(E) output (E<=4V); each exposed cell side is emitted once.
fn edges(w:usize,h:usize,states:&[Option<bool>])->Vec<Edge> {
    let mut out=Vec::new();
    let inside=|x:usize,y:usize|states[y*w+x]==Some(true);
    for y in 0..h {for x in 0..w {
        if !inside(x,y) {continue;}
        if x==0||!inside(x-1,y) {out.push(Edge{x,y,vertical:true});}
        if x+1==w||!inside(x+1,y) {out.push(Edge{x:x+1,y,vertical:true});}
        if y==0||!inside(x,y-1) {out.push(Edge{x,y,vertical:false});}
        if y+1==h||!inside(x,y+1) {out.push(Edge{x,y:y+1,vertical:false});}
    }}out
}
pub fn selected(view:&View,id:Option<u64>)->Option<&Boundary> {
    id.and_then(|id|view.perception_boundaries.iter().find(|b|b.actor==id))
}
fn logical(p:&Painter,v:&View,e:Edge)->(i32,i32,i32,i32) {
    let(cw,ch,ox,oy)=p.map_layout(v.width,v.height);
    // Keep right/bottom map-limit strokes inside the clip. Include both grid
    // endpoints so concave and convex corners meet without a missing pixel.
    let x=ox+e.x as i32*cw-i32::from(e.x==v.width);
    let y=oy+e.y as i32*ch-i32::from(e.y==v.height);
    (x,y,if e.vertical{1}else{cw+1},if e.vertical{ch+1}else{1})
}
pub fn bounds(p:&Painter,v:&View,b:&Boundary)->Vec<crate::render::damage::Rect> {
    b.edges.iter().filter_map(|&e|{let(x,y,w,h)=logical(p,v,e);p.logical_damage_rect(x-1,y-1,w+2,h+2)}).collect()
}
/// Same thin solid rectangles for software/GPU, after map glyphs/path overlays.
pub fn paint(p:&mut Painter,v:&View) {
    let Some(b)=selected(v,p.enemy_hover) else{return;};
    let old=p.clip;let(cw,ch,ox,oy)=p.map_layout(v.width,v.height);
    p.clip=Some((ox,oy,cw*v.width as i32,ch*v.height as i32));
    for &e in &b.edges {let(x,y,w,h)=logical(p,v,e);p.rect(x,y,w,h,INK);}
    p.clip=old;
}

#[cfg(test)]
#[path="perception_ui_tests.rs"]
mod tests;
