//! R51 perception preview. All geometry/light comes from the detached current
//! observation. Unknown rays produce no boundary, never an invented safe zone.
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
        let optimistic=o.cells.iter().map(|c|current(c.index)&&
            (matches!(c.tile.as_deref(),Some("wall"|"tree"))||c.door_open==Some(false))).collect::<Vec<_>>();
        let blocked=o.cells.iter().map(|c|!current(c.index)||optimistic[c.index]).collect();
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
/// O(A*V*(K²R+B)+V+K), O(V+K+A*E) extra memory. A admitted creatures,
/// V cells, K known portal endpoints (<=14), R max ray length, B target actor
/// modifiers, E contour edges (<=2V). Runs only on world observation, not paint.
pub fn observe(run:&Run,o:Option<&vision::TerrainObservation>,creatures:&[Creature])->Vec<Boundary> {
    if !rules::enabled(run) {return Vec::new();}
    let Some(o)=o else{return Vec::new();};
    // TerrainObservation may combine planes under these powers but carries no
    // per-cell plane tag. Omit uncertain geometry rather than mix their walls.
    if corpse_core::dreamwalking::active(run)
        ||creatures.iter().any(|c|run.actors_at(run.depth).iter().find(|a|a.id==c.id)
            .is_some_and(|a|a.perfect.value_or(false)
                &&!corpse_core::dreamwalking::same_plane(run,&run.player,a)))
        ||run.player.abilities.iter().any(|a|a=="enemy:beast:darkness-aura")
        ||(run.player.abilities.iter().any(|a|a=="anavani-shine")
            &&!run.player.active.iter().any(|a|a=="anavani-shine"))
        ||corpse_core::revision50_rings::has_effect(run,&run.player,"light") {return Vec::new();}
    let Some(g)=Geometry::new(run,o) else{return Vec::new();};
    let light=o.cells.iter().map(|c|c.illumination.map(|l|
        vision::perception_actor_light(run,&run.player,l,0.))).collect::<Vec<_>>();
    let stealth=corpse_core::engine::condition(&run.player,"stealth");
    let submerged=corpse_core::engine::condition(&run.player,"submergedStealth");
    let invisible=corpse_core::engine::condition(&run.player,"invisible")>0.;
    creatures.iter().filter_map(|row| {
        let actor=run.actors_at(run.depth).iter().find(|a|a.id==row.id)?;
        // A disguise/hallucination must never disclose its underlying senses.
        if row.shared_resources||actor.state=="ally"||row.hallucination.is_some()
            ||crate::creature_presentation::project(run,actor).is_some(){return None;}
        // Optical range is not a safe boundary for alternate senses. Do not
        // imply safety beyond it while these observer-specific overrides apply.
        if (corpse_core::engine::condition(actor,"allSeeingReveal")==1.
                &&corpse_core::engine::condition(actor,"allSeeingDepth")==run.depth)
            ||corpse_core::engine::condition(actor,"telepathic")>0.
            ||(corpse_core::engine::condition(actor,"cameTurns")>0.
                &&corpse_core::engine::condition(actor,"cameTarget")==run.player.id)
            ||actor.abilities.iter().any(|a|matches!(a.as_str(),"numo-mind-read"|"ursa-dream-see"))
            ||corpse_core::revision50_combat::witch_senses(run,actor,&run.player) {return None;}
        let from=Point{x:actor.x as i32,y:actor.y as i32};
        let at=actor.y as usize*o.width+actor.x as usize;
        if !o.cells.get(at).is_some_and(|c|c.current&&c.direct)
            ||!corpse_core::dreamwalking::same_plane(run,&run.player,actor)
            ||corpse_core::dreamwalking::is_nightmare(actor) {return None;}
        let perception=corpse_core::combat::effective_stats(run,actor).perception;
        let states=o.cells.iter().map(|c| {
            if !c.current||!c.direct||c.tile.is_none(){return None;}
            let illumination=light[c.index]?;
            if invisible {return Some(false);}
            let to=Point{x:(c.index%o.width)as i32,y:(c.index/o.width)as i32};
            // Without portals, range alone can prove an excluded tile. Avoid
            // ray walks outside the finite creature radius. Positive light still
            // requires LOS, and known portal shortcuts must use route distance.
            if g.floor.portals.as_ref().is_none_or(|p|p.is_empty())
                &&!rules::creature_in_range(perception,vision::distance(from,to),stealth,submerged,illumination){return Some(false);}
            let definite=rules::creature_in_range(perception,g.distance(from,to,&g.blocked),stealth,submerged,illumination);
            let possible=rules::creature_in_range(perception,g.distance(from,to,&g.optimistic),stealth,submerged,illumination);
            (definite==possible).then_some(definite)
        }).collect::<Vec<_>>();
        Some(Boundary{actor:row.id.to_bits(),edges:edges(o.width,o.height,&states)})
    }).collect()
}
/// Emit each known true/false interface once. Never close a contour against
/// fog, remembered cells, map limits or a path whose hidden segment matters.
fn edges(w:usize,h:usize,states:&[Option<bool>])->Vec<Edge> {
    let mut out=Vec::new();
    for y in 0..h {for x in 0..w {
        let Some(a)=states[y*w+x] else{continue;};
        if x+1<w && states[y*w+x+1].is_some_and(|b|a!=b) {out.push(Edge{x:x+1,y,vertical:true});}
        if y+1<h && states[(y+1)*w+x].is_some_and(|b|a!=b) {out.push(Edge{x,y:y+1,vertical:false});}
    }}out
}
pub fn selected(view:&View,id:Option<u64>)->Option<&Boundary> {
    id.and_then(|id|view.perception_boundaries.iter().find(|b|b.actor==id))
}
fn logical(p:&Painter,v:&View,e:Edge)->(i32,i32,i32,i32) {
    let(cw,ch,ox,oy)=p.map_layout(v.width,v.height);
    (ox+e.x as i32*cw,oy+e.y as i32*ch,if e.vertical{1}else{cw},if e.vertical{ch}else{1})
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
