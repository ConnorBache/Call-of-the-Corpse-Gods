//! R50 control effects. Pure target admission precedes shared resource payment.
//! Temporary hostility never rewrites allegiance; terrain edits retain exact
//! predecessors and expire only on the normal world clock, including offscreen.
use crate::{Actor, Field, Item, Run, engine::{self, EngineError}, movement::{self, Point}};
use serde::{Deserialize, Serialize};
use crate::revision52_amulet_tiers::{self as tiers,Profile};
pub const DISCORD: &str = "r50Discord";
pub const ROOTS: &str = "r50Roots";
const ROOT_X:&str="r50RootsX";
const ROOT_Y:&str="r50RootsY";
const ROOT_DEPTH:&str="r50RootsDepth";
const ROOT_SOURCE:&str="r50RootsSource";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlEffect { Beckoning, Discord, Obstruction, Roots, Teleportation, Floods }
pub fn effect(item:&Item)->Option<ControlEffect> {
    if item.kind!="amulet" {return None}
    Some(match item.effect.as_ref()?.as_str() {
        "beckoning"=>ControlEffect::Beckoning,"discord"=>ControlEffect::Discord,
        "obstruction"=>ControlEffect::Obstruction,"roots"=>ControlEffect::Roots,
        "teleportation"=>ControlEffect::Teleportation,"floods"=>ControlEffect::Floods,_=>return None,
    })
}
fn base_profile(kind:ControlEffect)->Profile {
    Profile {tier:1,direct_damage:if kind==ControlEffect::Floods{20.}else{0.},
        periodic_damage:if kind==ControlEffect::Roots{3.}else{0.},
        range:match kind {ControlEffect::Roots=>Some(6.),ControlEffect::Beckoning|ControlEffect::Discord|ControlEffect::Teleportation=>Some(8.),_=>None},
        duration:match kind {ControlEffect::Discord=>Some(6.),ControlEffect::Obstruction=>Some(8.),ControlEffect::Roots=>Some(4.),ControlEffect::Floods=>Some(12.),_=>None}}
}
pub fn activation_cost(kind:ControlEffect)->f64 {match kind {
    ControlEffect::Beckoning=>15.,ControlEffect::Discord|ControlEffect::Floods=>30.,
    ControlEffect::Obstruction=>25.,ControlEffect::Roots=>20.,ControlEffect::Teleportation=>50.,
}}
pub fn display_name(item:&Item)->Option<&'static str> {Some(match effect(item)? {
    ControlEffect::Beckoning=>"Amulet of Beckoning",ControlEffect::Discord=>"Amulet of Discord",
    ControlEffect::Obstruction=>"Amulet of Obstruction",ControlEffect::Roots=>"Amulet of Roots",
    ControlEffect::Teleportation=>"Amulet of Teleportation",ControlEffect::Floods=>"Amulet of Floods",
})}
pub fn description(item:&Item)->Option<&'static str> {Some(match effect(item)? {
    ControlEffect::Beckoning=>"Pulls a targeted creature next to you.",
    ControlEffect::Discord=>"Makes a creature and all other creatures hostile to each other for 6 turns.",
    ControlEffect::Obstruction=>"Creates a three-tile wall perpendicular to your aim for 8 turns.",
    ControlEffect::Roots=>"Roots a creature in place for 4 turns, dealing 3 damage each turn.",
    ControlEffect::Teleportation=>"Teleports another creature to a random valid square on the current floor.",
    ControlEffect::Floods=>"Sends a 3×5 wave that deals 20 damage, pushes creatures up to 2 tiles, extinguishes fire, and leaves shallow water for 12 turns.",
})}
pub fn details(item:&Item)->Option<&'static str> {Some(match effect(item)? {
    ControlEffect::Beckoning=>"Targets a visible creature within 8 tiles with a clear path to you.",
    ControlEffect::Discord | ControlEffect::Teleportation=>"Targets a visible creature within 8 tiles.",
    ControlEffect::Obstruction=>"Targets visible open ground along a cardinal or diagonal aim.",
    ControlEffect::Roots=>"Targets a grounded, visible creature within 6 tiles. Fire, teleporting, or forced movement breaks the roots.",
    ControlEffect::Floods=>return None,
})}

pub fn condition_key(k:&str)->bool {matches!(k,DISCORD|"r50DiscordFresh"|ROOTS|ROOT_X|ROOT_Y|ROOT_DEPTH|ROOT_SOURCE|tiers::ROOTS_DAMAGE)}
pub fn visible_target(r:&Run,x:f64,y:f64,range:f64)->Option<&Actor> {
    let f=crate::dreamwalking::floor_for(r,&r.player)?;
    crate::portals::cell(f,x,y)?;
    r.actors_at(r.depth).iter().find(|a|crate::half_here::owner_id(r,a)!=r.player.id&&a.hp>0.&&!a.dead.value_or(false)
        && !crate::dreamwalking::inert(r,a) && !crate::revision50_terrain::is_soldier(a) && !crate::revision50_nightmares::is_decoy(a)
        && crate::dreamwalking::same_plane(r,&r.player,a)
        && crate::portals::same_position(r,a,x,y)&&crate::vision::visible_actor(r,a)
        && crate::portals::actor_distance(r,&r.player,a)<=range)
}
fn beckoning_destinations(player:&Actor,a:&Actor)->Vec<Point> {
    let mut destinations:Vec<_>=movement::DIRECTIONS.iter().map(|d|Point{x:player.x as i32+d.x,y:player.y as i32+d.y}).collect();
    destinations.sort_by(|p,q|((p.x as f64-a.x).hypot(p.y as f64-a.y)).total_cmp(&((q.x as f64-a.x).hypot(q.y as f64-a.y))).then(p.y.cmp(&q.y)).then(p.x.cmp(&q.x)));
    destinations
}
/// Known-only pull route in the same deterministic candidate order as commit.
/// O(observation + A*visibility + 8*(B+L*(A*P+M+H))), O(V+A+L+B)
/// scratch; L bounded by floor dimensions, B cloned actor fields, M terrain modifications, H habitat/traits.
/// A route is shown only when every cell is currently observed. Hidden actors
/// never participate; preview is geometry, not a promise that admission succeeds.
pub fn beckoning_preview(r:&Run,x:f64,y:f64)->Vec<(usize,usize)> {beckoning_preview_with_range(r,x,y,8.)}
pub fn beckoning_preview_with_range(r:&Run,x:f64,y:f64,range:f64)->Vec<(usize,usize)> {
    let Some(a)=visible_target(r,x,y,range) else{return vec![]};
    let Some(f)=crate::dreamwalking::floor_for(r,a) else{return vec![]};
    let Some(o)=crate::vision::observe_terrain(r) else{return vec![]};
    let visible:Vec<_>=std::iter::once(&r.player).chain(r.actors_at(r.depth)).filter(|b|
        b.id!=a.id&&b.hp>0.&&!b.dead.value_or(false)&&crate::dreamwalking::same_plane(r,a,b)
        &&(b.id==r.player.id||crate::vision::visible_actor(r,b))).collect();
    let current=|p:Point|crate::portals::cell(f,p.x as f64,p.y as f64).is_some_and(|at|o.cells[at].current);
    if !current(Point{x:a.x as i32,y:a.y as i32}) {return vec![];}
    for to in beckoning_destinations(&r.player,a) {
        if (a.x,a.y)==(to.x as f64,to.y as f64){continue}
        let cells=crate::portals::linear_cells(Point{x:a.x as i32,y:a.y as i32},to);
        let mut prior=cells[0];let mut simulated=a.clone();
        let valid=cells.iter().skip(1).all(|&p| {
            if !current(p){return false}
            let at=p.y as usize*f.width()+p.x as usize;
            // Corner cells are knowledge-gated before movement tests, too.
            if p.x!=prior.x&&p.y!=prior.y {
                let c1=Point{x:p.x,y:prior.y};let c2=Point{x:prior.x,y:p.y};
                if !current(c1)||!current(c2){return false}
            }
            let options=crate::portals::step_options(r,&simulated,p.x as f64,p.y as f64);
            let open=options.len()==1&&options[0]==(p.x as f64,p.y as f64)
                && engine::actor_passable(r,&simulated,p.x as f64,p.y as f64)
                && !engine::bulwark_blocks(r,&simulated,p.x as f64,p.y as f64)
                && crate::ichor::movement_allowed(r,&simulated,p.x as f64,p.y as f64)
                && !crate::environment::blocks_sight(f,at)
                && !matches!(f.tiles[at].as_str(),"pit"|"chasm"|"up"|"down")
                && !visible.iter().any(|b|crate::portals::same_position(r,b,p.x as f64,p.y as f64))
                && movement::diagonal_step_allowed(&f.tiles,f.width(),prior,p,false);
            prior=p;simulated.x=p.x as f64;simulated.y=p.y as f64;open
        });
        if valid {return cells.into_iter().map(|p|(p.x as usize,p.y as usize)).collect()}
    }
    vec![]
}
/// At most eight ray checks (bounded by floor dimensions), each scans same-plane
/// occupancy and portal destinations. O(8*L*(A+P+M)), O(L) scratch.
fn beckoning_plan(r:&Run,x:f64,y:f64,range:f64)->Option<(f64,Vec<Point>)> {
    let a=visible_target(r,x,y,range)?;
    let f=crate::dreamwalking::floor_for(r,a)?;
    let destinations=beckoning_destinations(&r.player,a);
    for to in destinations {
        if (a.x,a.y)==(to.x as f64,to.y as f64) {continue}
        let cells=crate::portals::linear_cells(Point{x:a.x as i32,y:a.y as i32},to);
        let mut simulated=a.clone();
        let valid=cells.iter().skip(1).all(|p| {
            let (xx,yy)=(p.x as f64,p.y as f64);
            let Some(at)=crate::portals::cell(f,xx,yy) else{return false};
            let options=crate::portals::step_options(r,&simulated,xx,yy);
            if options.len()!=1||options[0]!=(xx,yy)
                || !engine::actor_passable(r,&simulated,xx,yy)
                || engine::bulwark_blocks(r,&simulated,xx,yy)
                || !crate::ichor::movement_allowed(r,&simulated,xx,yy)
                || matches!(f.tiles[at].as_str(),"pit"|"chasm"|"up"|"down")
                || crate::environment::blocks_sight(f,at)
                || crate::portals::occupied_at(r,&simulated,xx,yy,Some(a.id)).is_some()
                || !movement::diagonal_step_allowed(&f.tiles,f.width(),Point{x:simulated.x as i32,y:simulated.y as i32},*p,false) {return false}
            simulated.x=xx;simulated.y=yy;true
        });
        if valid {return Some((a.id,cells))}
    }
    None
}
/// Uses Cogworks' authored exact grid orientation: cardinal or exact diagonal.
/// No arbitrary-slope rounding rule is invented; the three-cell footprint must fit.
fn obstruction_cells(r:&Run,x:f64,y:f64)->Option<Vec<usize>> {
    let f=crate::dreamwalking::floor_for(r,&r.player)?;
    crate::portals::cell(f,x,y)?;
    let (dx,dy)=(x-r.player.x,y-r.player.y);
    if dx==0.&&dy==0. || dx!=0.&&dy!=0.&&dx.abs()!=dy.abs() {return None}
    let sign=|v:f64|if v==0. {0.}else{v.signum()};
    let (px,py)=(-sign(dy),sign(dx));
    (-1..=1).map(|k|crate::portals::cell(f,x+px*k as f64,y+py*k as f64)).collect()
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct TemporaryTerrain {pub cell:usize,pub previous:String,pub tile:String,pub expires_at:f64}
/// Only ordinary replaceable ground. Structures, topology, authored terrain and
/// overlapping temporary edits are rejected instead of losing restoration state.
pub fn can_replace_terrain(r:&Run,at:usize)->bool {
    replaceable_ground(r,at) && crate::dreamwalking::floor_for(r,&r.player).is_some_and(|f|
        f.revision50_temporary_terrain.as_ref().is_none_or(|v|!v.iter().any(|t|t.cell==at)))
}
// Shared protection gates, without granting general temporary-terrain stacking.
fn replaceable_ground(r:&Run,at:usize)->bool {
    let Some(f)=crate::dreamwalking::floor_for(r,&r.player) else{return false};
    f.tiles.get(at).is_some_and(|t|matches!(t.as_str(),"floor"|"grass"|"shallow-water"|"snow"|"sand"))
        && !crate::stone_seal_rules::contains(f,at)
        && f.doors.as_ref().is_none_or(|d|!d.contains_key(&at.to_string()))
        && f.grates.as_ref().is_none_or(|v|!v.contains(&(at as f64)))
        && !crate::borge::solid_at(f,(at%f.width())as f64,(at/f.width())as f64)
        && !crate::epidite::solid_at(f,(at%f.width())as f64,(at/f.width())as f64)
        && crate::portals::pair_at(f,at).is_none()
        && crate::portals::exits(f,at)==vec![at]
        && crate::portals::aliases(f,at)==vec![at]
        && f.revision50_terrain.as_ref().is_none_or(|v|!v.iter().any(|m|m.cells.iter().any(|c|c.y as usize*f.width()+c.x as usize==at)))
}
/// R52 alone may promote natural or Floods-created shallow water. Unrelated
/// temporary edits are never admitted, and all structural protections still apply.
pub(crate) fn can_deepen_floods_terrain(r:&Run,at:usize)->bool {
    tiers::enabled(r) && replaceable_ground(r,at)
        && crate::dreamwalking::floor_for(r,&r.player).is_some_and(|f|
            f.tiles.get(at).is_some_and(|t|t=="shallow-water")
            && f.revision50_temporary_terrain.as_ref().is_none_or(|v|
                v.iter().filter(|t|t.cell==at).all(|t|t.tile=="shallow-water")))
}
/// O(C*(T+M+P+G+B)) admission/merge, C<=15; see the R52 Floods ledger.
/// Replace the single record in place, retaining its first true predecessor.
/// A refreshed expiry must not allow the earlier shallow entry to resurrect water.
pub(crate) fn deepen_floods_terrain(r:&mut Run,cells:&[usize]) {
    let expires_at=r.tick+12.;
    for &cell in cells {
        if !can_deepen_floods_terrain(r,cell) {continue}
        let f=r.floor_mut().unwrap();
        let previous=std::mem::replace(&mut f.tiles[cell],"water".into());
        let entries=f.revision50_temporary_terrain.get_or_insert_with(Vec::new);
        if let Some(t)=entries.iter_mut().find(|t|t.cell==cell) {
            t.tile="water".into();t.expires_at=expires_at;
        } else {
            entries.push(TemporaryTerrain{cell,previous,tile:"water".into(),expires_at});
        }
        f.dirty=true;
    }
}
pub fn add_temporary_terrain(r:&mut Run,cells:&[usize],tile:&str,duration:f64) {
    let expires_at=r.tick+duration;
    let f=r.floor_mut().unwrap();f.dirty=true;
    for &cell in cells {
        let previous=std::mem::replace(&mut f.tiles[cell],tile.into());
        f.revision50_temporary_terrain.get_or_insert_with(Vec::new).push(TemporaryTerrain{cell,previous,tile:tile.into(),expires_at});
    }
}
/// O(T) normal-turn scan; restoration is conditional so a later independent
/// terrain edit is never erased. No actor is overwritten or moved by expiry.
pub fn expire_terrain(r:&mut Run) {
    crate::revision50_potions::expire(r);
    if !crate::revision50::enabled(r) {return}
    let tick=r.tick;
    if r.floor().is_none_or(|f|f.revision50_temporary_terrain.as_ref().is_none_or(|v|!v.iter().any(|t|t.expires_at<=tick))) {return}
    let f=r.floor_mut().unwrap();
    let entries=std::mem::take(f.revision50_temporary_terrain.as_mut().unwrap());
    let mut retained=Vec::new();
    let mut restored=Vec::new();
    for t in entries {if t.expires_at>tick {retained.push(t)} else if f.tiles.get(t.cell)==Some(&t.tile) {restored.push((t.cell,t.tile));f.tiles[t.cell]=t.previous}}
    f.revision50_temporary_terrain=if retained.is_empty(){Field::Missing}else{retained.into()};
    for (cell,tile) in restored {crate::events::at(r,if tile=="wall"{"A09"}else{"A12"},if tile=="wall"{"dissolve"}else{"wave"},crate::events::Stage::Special,&crate::animation_amulets::cell_points(r,&[cell]));}
}
pub fn has_temporary_terrain(r:&Run,depth:f64)->bool {
    crate::revision50_potions::has_darkness(r,depth) || crate::revision50::enabled(r)&&r.floors.iter().chain(r.dreamwalking.as_ref().into_iter().flat_map(|s|s.floors.iter()))
        .any(|f|f.depth==depth&&f.revision50_temporary_terrain.as_ref().is_some_and(|v|!v.is_empty()))
}
pub fn obstruction_blocks(r:&Run,a:&Actor,x:f64,y:f64)->bool {
    crate::revision50::enabled(r)&&crate::dreamwalking::floor_for(r,a).is_some_and(|f|
        crate::portals::cell(f,x,y).is_some_and(|at|f.revision50_temporary_terrain.as_ref().is_some_and(|v|
            v.iter().any(|t|t.cell==at&&t.tile=="wall"&&f.tiles[at]=="wall"))))
}
pub fn validate_terrain(r:&Run)->Result<(),String> {
    crate::revision50_potions::validate(r)?;
    crate::revision50_terrain::validate_run(r)?;
    for f in r.floors.iter().chain(r.dreamwalking.as_ref().into_iter().flat_map(|s|s.floors.iter())) {
        if let Some(v)=f.revision50_temporary_terrain.as_ref() {
            if !crate::revision50::enabled(r) {return Err("Temporary amulet terrain requires revision 50".into())}
            let mut seen=std::collections::HashSet::new();
            if v.iter().any(|t|t.cell>=f.tiles.len()||!seen.insert(t.cell)||!t.expires_at.is_finite()||t.expires_at<0.
                || !(matches!(t.tile.as_str(),"wall"|"shallow-water") || tiers::enabled(r)&&t.tile=="water")||!matches!(t.previous.as_str(),"floor"|"grass"|"shallow-water"|"snow"|"sand")) {return Err("Invalid temporary amulet terrain".into())}
        }
    }Ok(())
}
fn run_profile(r:&Run,kind:ControlEffect)->Profile {
    let mut p=base_profile(kind);
    if crate::amulet_rules::current(r) {match kind {
        ControlEffect::Beckoning|ControlEffect::Discord=>p.range=None,
        ControlEffect::Roots=>{p.duration=Some(8.);p.periodic_damage=0.;}, _=>{},
    }}
    p
}
pub fn target_valid(r:&Run,kind:ControlEffect,x:f64,y:f64)->bool {target_valid_with_profile(r,kind,x,y,run_profile(r,kind))}
pub fn target_valid_with_profile(r:&Run,kind:ControlEffect,x:f64,y:f64,p:Profile)->bool {
    if !crate::revision50::enabled(r) || (crate::amulet_rules::current(r)&&kind==ControlEffect::Floods) {return false}
    if crate::dreamwalking::dream_actor(r,&r.player)!=crate::dreamwalking::context_dream(r) {
        return target_valid_with_profile(&crate::dreamwalking::observation_run(r),kind,x,y,p)
    }
    if matches!(kind,ControlEffect::Beckoning|ControlEffect::Discord|ControlEffect::Roots)
        && visible_target(r,x,y,p.range.unwrap_or(f64::INFINITY))
            .is_some_and(|a|crate::revision50::duel_blocks(r,&r.player,a)) {return false}
    match kind {
        ControlEffect::Beckoning=>beckoning_plan(r,x,y,p.range.unwrap_or(f64::INFINITY)).is_some(),
        ControlEffect::Discord=>visible_target(r,x,y,p.range.unwrap_or(f64::INFINITY)).is_some(),
        ControlEffect::Roots=>visible_target(r,x,y,6.).is_some_and(|a|!engine::flying_in(r,a)),
        ControlEffect::Obstruction=>obstruction_cells(r,x,y).is_some_and(|cells| {
            let f=crate::dreamwalking::floor_for(r,&r.player).unwrap();
            crate::vision::visible(r,x as usize,y as usize) && cells.into_iter().all(|at|can_replace_terrain(r,at)
                && crate::portals::occupied_at(r,&r.player,(at%f.width())as f64,(at/f.width())as f64,None).is_none())
        }),
        ControlEffect::Teleportation|ControlEffect::Floods=>crate::revision50_wave_teleport::target_valid_with_range(r,kind==ControlEffect::Teleportation,x,y,p.range.unwrap_or(f64::INFINITY)),
    }
}
pub fn execute(r:&mut Run,kind:ControlEffect,x:f64,y:f64)->Result<(),EngineError> {execute_with_profile(r,kind,x,y,run_profile(r,kind))}
pub fn execute_with_profile(r:&mut Run,kind:ControlEffect,x:f64,y:f64,p:Profile)->Result<(),EngineError> {
    if !target_valid_with_profile(r,kind,x,y,p) {return Err(EngineError("The control amulet target is no longer valid.".into()))}
    // R52 casts use the same committed collision ray as other targeted magic.
    // Admission stays aim-only: hidden blockers may intercept after payment,
    // never leak their existence through preflight validity.
    let (mut x,mut y)=(x,y);
    let mut intercepted=None;
    if r.simulation_revision.value_or(29.)>=52. && matches!(kind,ControlEffect::Beckoning|ControlEffect::Discord|ControlEffect::Roots|ControlEffect::Obstruction) {
        // Beckoning retains the same collision checks. Its round-trip
        // cue is emitted only when a creature was actually pulled.
        let key=match kind {ControlEffect::Beckoning=>None,ControlEffect::Discord=>Some("A08"),ControlEffect::Roots=>Some("A10"),_=>Some("A09")};
        let hit=crate::animation_amulets::control_cast(r,key,x,y);
        x=hit.x;y=hit.y;intercepted=hit.actor_id;
        if kind==ControlEffect::Obstruction {
            // Terrain cannot be erected on the intercepting creature.
            if hit.actor_id.is_some() || !target_valid_with_profile(r,kind,x,y,p) {return Ok(())}
        } else {
            let Some(a)=hit.actor_id.and_then(|id|engine::actor(r,id)) else{return Ok(())};
            if a.id==r.player.id || a.hp<=0. || a.dead.value_or(false)
                || crate::dreamwalking::inert(r,a) || crate::revision50_terrain::is_soldier(a)
                || crate::revision50_nightmares::is_decoy(a) || crate::revision50::duel_blocks(r,&r.player,a)
                || (kind==ControlEffect::Roots && engine::flying_in(r,a)) {return Ok(())}
            // Beckoning retains its separately authored legal pull-path checks.
            if kind==ControlEffect::Beckoning && beckoning_plan(r,x,y,p.range.unwrap_or(f64::INFINITY)).is_none(){return Ok(())}
        }
    }
    match kind {
        ControlEffect::Beckoning=>{
            let (id,path)=beckoning_plan(r,x,y,p.range.unwrap_or(f64::INFINITY)).unwrap();
            let original=engine::actor(r,id).unwrap().clone();
            let mut traveled=vec![crate::events::EventPoint::from(&original)];
            for p in path.iter().skip(1) {
                let a=engine::actor(r,id).ok_or_else(||EngineError("Beckoning target left the floor".into()))?;
                let (dx,dy)=(p.x as f64-a.x,p.y as f64-a.y);
                let old=engine::actor_mut_trusted(r,id).unwrap().conditions.insert("forcedMove".into(),1.);
                let moved=engine::move_actor(r,id,dx,dy,true);
                if let Some(a)=engine::actor_mut_trusted(r,id) {a.conditions.shift_remove("forcedMove");if let Some(old)=old {a.conditions.insert("forcedMove".into(),old);}}
                if !moved? {return Err(EngineError("The Beckoning path became blocked.".into()))}
                if let Some(a)=engine::actor(r,id){traveled.push(a.into());}
            }
            if traveled.len()>1 {crate::beckoning_skip::after_pull(r,r.player.id,id);}
            if crate::events::enabled(r) && traveled.len()>1 {
                traveled.push((&r.player).into());
                crate::animation_shapes::beckon_round_trip(r,&r.player,&original,"A07",&traveled,"#b78ee8");
            }
        },
        ControlEffect::Discord=>{let id=intercepted.unwrap_or_else(||visible_target(r,x,y,p.range.unwrap_or(f64::INFINITY)).unwrap().id);let duration=crate::monster_statuses::status_duration(r,engine::actor(r,id).unwrap(),p.duration.unwrap());engine::actor_mut_trusted(r,id).unwrap().conditions.insert(DISCORD.into(),duration);let a=engine::actor(r,id).unwrap();crate::events::cue(r,a,"A08","crossed-ring",crate::events::Stage::Special,&[a.into()]);},
        ControlEffect::Roots=>{
            let id=intercepted.unwrap_or_else(||visible_target(r,x,y,6.).unwrap().id);let depth=r.depth;let source=r.player.id;let legacy_tier=tiers::enabled(r)&&!crate::amulet_rules::current(r);
            let duration=crate::monster_statuses::status_duration(r,engine::actor(r,id).unwrap(),p.duration.unwrap());
            let a=engine::actor_mut_trusted(r,id).unwrap();
            for (key,value) in [(ROOTS,duration),(ROOT_X,a.x),(ROOT_Y,a.y),(ROOT_DEPTH,depth),(ROOT_SOURCE,source)] {a.conditions.insert(key.into(),value);}
            if legacy_tier {a.conditions.insert(tiers::ROOTS_DAMAGE.into(),p.periodic_damage);} else {a.conditions.shift_remove(tiers::ROOTS_DAMAGE);}
            let a=engine::actor(r,id).unwrap();crate::events::cue(r,a,"A10","root",crate::events::Stage::Special,&[a.into()]);
        },
        ControlEffect::Obstruction=>{let cells=obstruction_cells(r,x,y).unwrap();add_temporary_terrain(r,&cells,"wall",p.duration.unwrap());crate::events::at(r,"A09","slab",crate::events::Stage::Special,&crate::animation_amulets::cell_points(r,&cells));},
        ControlEffect::Teleportation|ControlEffect::Floods=>crate::revision50_wave_teleport::execute_with_profile(r,kind==ControlEffect::Teleportation,x,y,p.range.unwrap_or(f64::INFINITY),p.direct_damage)?,
    }Ok(())
}
pub fn discordant(r:&Run,a:&Actor)->bool {crate::revision50::enabled(r)&&engine::condition(a,DISCORD)>0.}
pub fn discord_hostility(r:&Run,a:&Actor,b:&Actor)->bool {
    crate::revision50::enabled(r)&&a.id!=b.id&&crate::half_here::owner_id(r,a)!=crate::half_here::owner_id(r,b)&&(discordant(r,a)||discordant(r,b))&&crate::dreamwalking::same_plane(r,a,b)
}
pub fn hostile(r:&Run,a:&Actor,b:&Actor)->bool {
    discord_hostility(r,a,b)||((a.id==r.player.id||a.state=="ally")!=(b.id==r.player.id||b.state=="ally"))
}
/// A narrow AI overlay handles the discordant actor and creatures selecting it.
/// Permanent state/order/perfect/allegiance are untouched, including allies and
/// neutral creatures. Legacy Haste is movement-only; R52 Haste is a full action.
/// O(A*(visibility+distance))
/// plus existing ability/combat/pathfinding costs; no new target memories.
pub fn discord_turn(r:&mut Run,id:f64,haste:bool)->Result<bool,EngineError> {
    discord_action(r,id,haste,false)
}
pub(crate) fn discord_fast_followup(r:&mut Run,id:f64)->Result<bool,EngineError> {
    discord_action(r,id,true,true)
}
fn discord_action(r:&mut Run,id:f64,haste:bool,fast:bool)->Result<bool,EngineError> {
    if !crate::revision50::enabled(r) {return Ok(false)}
    crate::dreamwalking::with_actor(r,id,|r| {
        let Some(a)=engine::actor(r,id).cloned() else{return Ok(false)};
        if a.dead.value_or(false)||a.hp<=0.||crate::dreamwalking::inert(r,&a)||crate::revision50_nightmares::is_decoy(&a) {return Ok(false)}
        let own=discordant(r,&a);
        if !own&&!std::iter::once(&r.player).chain(r.actors_at(r.depth)).any(|b|b.hp>0.&&!b.dead.value_or(false)&&discordant(r,b)&&discord_hostility(r,&a,b)) {return Ok(false)}
        let target=std::iter::once(&r.player).chain(r.actors_at(r.depth)).filter(|b|
            b.id!=a.id&&!b.dead.value_or(false)&&b.hp>0.&&crate::dreamwalking::same_plane(r,&a,b)
            && (discord_hostility(r,&a,b)||(a.state!="neutral"&&b.state!="neutral"&&hostile(r,&a,b)))
            && (engine::condition(b,"invisible")<=0.||engine::condition(&a,"telepathic")>0.)
            && crate::vision::line_of_sight(r,&a,b))
            .min_by(|b,c|crate::portals::actor_distance(r,&a,b).total_cmp(&crate::portals::actor_distance(r,&a,c)).then(b.id.total_cmp(&c.id))).cloned();
        let Some(b)=target else{return Ok(own)};
        if !own&&!discordant(r,&b) {return Ok(false)}
        if engine::condition(&a,"paralyzed")>0.||crate::ichor::stranded(r,&a)||(!haste&&crate::monster_statuses::slow_skips_action(r,&a)) {return Ok(true)}
        if crate::ai::sleeping(&a) {crate::ai::wake(engine::actor_mut_trusted(r,id).unwrap(),true);return Ok(true)}
        if crate::minotaur_rules::only_bump(r,&a) {
            return engine::monster_ai_turn(r,id,false);
        }
        if !haste || r.simulation_revision.value_or(29.)>=52. {
            if !fast && crate::ai::recover_ranged(r,id) {return Ok(true)}
            if !fast && crate::ai::enemy_action(r,id,b.id)? {return Ok(true)}
            if crate::portals::actor_distance(r,&a,&b)<=std::f64::consts::SQRT_2 {
                if !crate::monster_statuses::try_hemophilia_episode(r,id).map_err(EngineError)? {engine::attack(r,id,b.id,1.)?;}return Ok(true)
            }
        }
        if let Some(step)=engine::creature_path_step(r,id,Point{x:b.x as i32,y:b.y as i32}) {engine::move_actor(r,id,step.x as f64,step.y as f64,true)?;}
        Ok(true)
    })
}
pub fn break_roots(r:&mut Run,id:f64) {if let Some(a)=engine::actor(r,id).filter(|a|(engine::condition(a,ROOTS)>0.||a.conditions.contains_key(ROOT_X))){crate::events::cue(r,a,"A10","dissolve-root",crate::events::Stage::Special,&[a.into()]);}if let Some(a)=engine::actor_mut_trusted(r,id) {for k in [ROOTS,ROOT_X,ROOT_Y,ROOT_DEPTH,ROOT_SOURCE,tiers::ROOTS_DAMAGE] {a.conditions.shift_remove(k);}}}
pub fn rooted(r:&Run,a:&Actor)->bool {
    if !crate::revision50::enabled(r)||engine::condition(a,ROOTS)<=0.||engine::condition(a,"burning")!=0.
        ||a.conditions.get(ROOT_X)!=Some(&a.x)||a.conditions.get(ROOT_Y)!=Some(&a.y)||a.conditions.get(ROOT_DEPTH)!=Some(&r.depth) {return false}
    crate::dreamwalking::floor_for(r,a).is_some_and(|f| {
        let Some(at)=crate::portals::cell(f,a.x,a.y) else{return false};
        !f.environment.as_ref().is_some_and(|e|e.fire.get(at).copied().unwrap_or(0.)>0.)
            &&!f.hellfire.as_ref().is_some_and(|v|v.get(at)==Some(&true))
    })
}
pub fn tick_actor(r:&mut Run,id:f64)->Result<(),EngineError> {
    if !crate::revision50::enabled(r) {return Ok(())}
    if let Some(a)=engine::actor_mut_trusted(r,id) {engine::tick_condition(a,crate::revision50_potions::FIRE_IMMUNITY);}
    if engine::actor(r,id).is_some_and(|a|engine::condition(a,ROOTS)>0.) {
        if !engine::actor(r,id).is_some_and(|a|rooted(r,a)) {break_roots(r,id)} else {
            // Like ordinary status provenance, player source zero maps to the
            // inactive-floor proxy's player identity for indirect kill credit.
            let source=engine::actor(r,id).unwrap().conditions.get(ROOT_SOURCE).copied().map(|source|if source==0. {r.player.id}else{source});
            // Current Roots immobilizes only, including old captured damage rates.
            // Do not send a zero packet through combat: that could trigger procs.
            if !crate::amulet_rules::current(r) {
                let damage=if tiers::enabled(r){engine::actor(r,id).unwrap().conditions.get(tiers::ROOTS_DAMAGE).copied().unwrap_or(3.)}else{3.};
                engine::damage_environment(r,id,damage,source,"roots",true)?;
            }
            if let Some(a)=engine::actor_mut_trusted(r,id) {engine::tick_condition(a,ROOTS);}
            if engine::actor(r,id).is_some_and(|a|engine::condition(a,ROOTS)<=0.) {break_roots(r,id)}
        }
    }
    if let Some(a)=engine::actor(r,id).filter(|a|engine::condition(a,DISCORD)==1.){crate::events::cue(r,a,"A08","unfold",crate::events::Stage::Special,&[a.into()]);}
    if let Some(a)=engine::actor_mut_trusted(r,id) {engine::tick_condition(a,DISCORD);if engine::condition(a,DISCORD)<=0. {a.conditions.shift_remove(DISCORD);}}
    Ok(())
}
/// UI checks only visible actors and observed terrain. Hidden occupants never
/// change previews or error prose; full transactional admission checks them.
pub fn target_blocker_ui(r:&Run,item:&Item,x:f64,y:f64)->Option<&'static str> {
    if tiers::retired(r,item) {return Some(tiers::RETIRED_MESSAGE);}
    let kind=effect(item)?;
    let p=tiers::profile(r,item)?;
    if matches!(kind,ControlEffect::Teleportation|ControlEffect::Floods) {return crate::revision50_wave_teleport::target_blocker_ui_with_range(r,kind==ControlEffect::Teleportation,x,y,p.range.unwrap_or(f64::INFINITY))}
    if kind!=ControlEffect::Obstruction {
        let range=p.range.unwrap_or(f64::INFINITY);
        let Some(a)=visible_target(r,x,y,range) else{return Some("Choose a visible creature within this amulet's range.")};
        if kind==ControlEffect::Roots&&engine::flying_in(r,a) {return Some("Roots require a grounded creature.")}
        return None
    }
    let Some(cells)=obstruction_cells(r,x,y) else{return Some("Choose a cardinal or diagonal direction with room for all three wall tiles.")};
    let Some(o)=crate::vision::observe_terrain(r) else{return Some("No floor is visible.")};
    let center=crate::dreamwalking::floor_for(r,&r.player).and_then(|f|crate::portals::cell(f,x,y)).unwrap();
    if !o.cells[center].current {return Some("Choose a currently visible target tile.")}
    if cells.iter().any(|&at|o.cells.get(at).is_some_and(|c|c.current&&!matches!(c.tile.as_deref(),Some("floor"|"grass"|"shallow-water"|"snow"|"sand")))) {return Some("Choose a valid target for this amulet.")}
    let f=crate::dreamwalking::floor_for(r,&r.player).unwrap();
    if cells.iter().any(|&at|std::iter::once(&r.player).chain(r.actors_at(r.depth)).any(|a|a.hp>0.&&!a.dead.value_or(false)&&crate::dreamwalking::same_plane(r,&r.player,a)&&crate::vision::visible_actor(r,a)&&crate::portals::same_position(r,a,(at%f.width())as f64,(at/f.width())as f64))) {return Some("The wall cannot replace a visible creature.")}
    None
}
pub fn preview(r:&Run,item:&Item,x:f64,y:f64)->Vec<(usize,usize)> {
    if tiers::retired(r,item) {return vec![];}
    let Some(kind)=effect(item) else{return vec![]};
    let Some(p)=tiers::profile(r,item) else{return vec![]};
    if matches!(kind,ControlEffect::Teleportation|ControlEffect::Floods) {return crate::revision50_wave_teleport::preview_with_range(r,kind==ControlEffect::Teleportation,x,y,p.range.unwrap_or(f64::INFINITY))}
    if target_blocker_ui(r,item,x,y).is_some() {return vec![]}
    if kind==ControlEffect::Obstruction {
        let f=crate::dreamwalking::floor_for(r,&r.player).unwrap();let Some(o)=crate::vision::observe_terrain(r) else{return vec![]};
        return obstruction_cells(r,x,y).unwrap().into_iter().filter(|&at|o.cells[at].current).map(|at|(at%f.width(),at/f.width())).collect()
    }
    vec![(x as usize,y as usize)]
}

#[cfg(test)]
mod tests {
    use super::*;
    fn room()->Run {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=50.0.into();r.player.x=12.;r.player.y=14.;r.player.hp=100.;r.player.max_hp=100.;
        r.player.conditions.clear();r.player.abilities.clear();r.player.active.clear();
        r.player.equipment_abilities=Field::Missing;r.player.ability_reserves=Field::Missing;
        r.player.conditions.insert("allSeeingReveal".into(),1.);
        r.player.conditions.insert("allSeeingDepth".into(),r.depth);
        r.inventory.clear();r.actors_at_mut(r.depth).clear();
        let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.doors=Field::Missing;
        f.environment=Field::Missing;f.hellfire=Field::Missing;f.portals=Field::Missing;
        f.non_euclidean=Field::Missing;f.speed_up=Field::Missing;f.revision50_terrain=Field::Missing;
        f.permanent_fire=Field::Missing;f.razorleaves=Field::Missing;r
    }
    fn creature(r:&mut Run,id:f64,x:f64,y:f64,state:&str) {
        r.actors_at_mut(r.depth).push(Actor{id,x,y,hp:100.,max_hp:100.,base_max_hp:100.0.into(),state:state.into(),
            stats:crate::Stats{accuracy:100.,offense:10.,max_hp:100.0.into(),..Default::default()},..Default::default()});
    }
    fn item(effect:&str)->Item {Item{id:900.,kind:"amulet".into(),effect:effect.to_string().into(),equipped:"amulet".to_string().into(),..Default::default()}}
    #[test] fn authored_control_magic_draws_actual_ray_and_intercepts_first_creature() {
        for (kind,key,condition) in [(ControlEffect::Discord,"A08",DISCORD),(ControlEffect::Roots,"A10",ROOTS)] {
            let mut r=room();r.simulation_revision=52.0.into();
            creature(&mut r,100.,14.,14.,"hunting");creature(&mut r,101.,17.,14.,"hunting");
            crate::events::begin();execute(&mut r,kind,17.,14.).unwrap();let events=crate::events::finish();
            assert!(engine::condition(engine::actor(&r,100.).unwrap(),condition)>0.);
            assert_eq!(engine::condition(engine::actor(&r,101.).unwrap(),condition),0.);
            let cue=events.iter().filter_map(|e|e.animation.as_ref()).find(|a|a.proposal==key&&a.stage==crate::events::Stage::Travel).unwrap();
            assert_eq!(cue.delivery,crate::events::Delivery::Magic);
            assert_eq!(cue.cells.last().unwrap().x,14.);
        }
    }
    #[test] fn authored_control_magic_wall_stops_effect_without_hidden_admission_probe() {
        for (kind,condition) in [(ControlEffect::Discord,DISCORD),(ControlEffect::Roots,ROOTS)] {
            let mut r=room();r.simulation_revision=52.0.into();creature(&mut r,101.,17.,14.,"hunting");
            let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[14*w+14]="wall".into();
            assert!(target_valid(&r,kind,17.,14.));
            execute(&mut r,kind,17.,14.).unwrap();
            assert_eq!(engine::condition(engine::actor(&r,101.).unwrap(),condition),0.);
        }
    }
    #[test] fn authored_control_preview_stops_at_known_blocker_without_scouting() {
        let mut r=room();r.simulation_revision=52.0.into();creature(&mut r,100.,14.,14.,"hunting");creature(&mut r,101.,17.,14.,"hunting");
        let i=item("discord");let before=r.clone();
        let preview=crate::revision50_amulet_effects::aim_preview(&r,&i,17.,14.);
        assert_eq!(preview.delivery.last(),Some(&(14,14)));assert_eq!(preview.area,vec![(14,14)]);assert_eq!(r,before);
    }
    #[test] fn authored_obstruction_never_builds_beyond_a_creature() {
        let mut r=room();r.simulation_revision=52.0.into();creature(&mut r,100.,14.,14.,"hunting");
        execute(&mut r,ControlEffect::Obstruction,18.,14.).unwrap();
        let w=r.floor().unwrap().width();assert_eq!(r.floor().unwrap().tiles[14*w+18],"floor");
    }
    #[test] fn beckoning_pulls_nearest_adjacent_and_breaks_roots() {
        let mut r=room();creature(&mut r,100.,17.,14.,"hunting");
        execute(&mut r,ControlEffect::Roots,17.,14.).unwrap();
        assert!(target_valid(&r,ControlEffect::Beckoning,17.,14.));
        execute(&mut r,ControlEffect::Beckoning,17.,14.).unwrap();
        let a=engine::actor(&r,100.).unwrap();assert_eq!((a.x,a.y),(13.,14.));assert!(!a.conditions.contains_key(ROOTS));
        assert!(!a.conditions.contains_key("forcedMove"));
    }
    #[test] fn beckoning_failure_has_no_resource_or_state_mutation() {
        let mut r=room();creature(&mut r,100.,17.,14.,"hunting");r.inventory.push(item("beckoning"));
        let w=r.floor().unwrap().width();for d in movement::DIRECTIONS {r.floor_mut().unwrap().tiles[(14+d.y)as usize*w+(12+d.x)as usize]="wall".into();}
        let before=r.clone();
        assert!(!target_valid(&r,ControlEffect::Beckoning,17.,14.));
        assert!(crate::revision50_amulet_effects::admission(&r,&crate::Command::UseAt{id:900.,x:17.,y:14.}).unwrap().is_err());
        assert_eq!(r,before);
    }
    #[test] fn obstruction_exact_perpendicular_unlimited_range_and_safe_expiry() {
        let mut r=room();let cells=obstruction_cells(&r,32.,14.).unwrap();let w=r.floor().unwrap().width();
        assert_eq!(cells,vec![13*w+32,14*w+32,15*w+32]);
        assert!(target_valid(&r,ControlEffect::Obstruction,32.,14.));
        execute(&mut r,ControlEffect::Obstruction,32.,14.).unwrap();
        let expiry=r.tick+8.;r.tick=expiry-1.;expire_terrain(&mut r);assert_eq!(r.floor().unwrap().tiles[cells[0]],"wall");
        r.player.active.push("etill-phase-through".into());assert!(!engine::actor_passable(&r,&r.player,32.,14.));
        r.floor_mut().unwrap().tiles[cells[0]]="grass".into();r.tick=expiry;expire_terrain(&mut r);
        assert_eq!(r.floor().unwrap().tiles[cells[0]],"grass");assert_eq!(r.floor().unwrap().tiles[cells[1]],"floor");
        assert!(r.floor().unwrap().revision50_temporary_terrain.is_missing());
    }
    #[test] fn obstruction_rejects_occupied_stairs_portals_bounds_and_unknown_orientation() {
        let mut r=room();creature(&mut r,100.,20.,14.,"hunting");assert!(!target_valid(&r,ControlEffect::Obstruction,20.,14.));
        r.actors_at_mut(r.depth).clear();let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[14*w+20]="down".into();
        assert!(!target_valid(&r,ControlEffect::Obstruction,20.,14.));
        assert!(obstruction_cells(&r,20.,15.).is_none());assert!(obstruction_cells(&r,0.,2.).is_none());
        assert!(!can_replace_terrain(&r,14*w+20));
    }
    #[test] fn temporary_terrain_and_statuses_roundtrip_and_rewind_together() {
        let mut r=room();creature(&mut r,100.,15.,14.,"ally");execute(&mut r,ControlEffect::Discord,15.,14.).unwrap();
        execute(&mut r,ControlEffect::Obstruction,20.,14.).unwrap();
        let serialized=serde_json::to_string(&r).unwrap();let mut r:Run=serde_json::from_str(&serialized).unwrap();
        let snapshot=crate::temporal::Snapshot::from_value(crate::temporal::Snapshot::capture(&r).value().unwrap()).unwrap();
        r.tick+=8.;expire_terrain(&mut r);tick_actor(&mut r,100.).unwrap();snapshot.restore(&mut r).unwrap();
        assert_eq!(r.floor().unwrap().revision50_temporary_terrain.as_ref().unwrap().len(),3);
        assert_eq!(engine::condition(engine::actor(&r,100.).unwrap(),DISCORD),6.);assert!(validate_terrain(&r).is_ok());
    }
    #[test] fn discord_is_mutual_preserves_allegiance_and_expires_after_six_ticks() {
        let mut r=room();creature(&mut r,100.,15.,14.,"ally");creature(&mut r,101.,16.,14.,"ally");
        execute(&mut r,ControlEffect::Discord,15.,14.).unwrap();
        let a=engine::actor(&r,100.).unwrap();let b=engine::actor(&r,101.).unwrap();
        assert!(hostile(&r,a,b)&&hostile(&r,b,a));assert!(hostile(&r,a,&r.player));
        assert!(discord_turn(&mut r,101.,false).unwrap());assert!(engine::actor(&r,100.).unwrap().hp<100.);
        for _ in 0..6 {tick_actor(&mut r,100.).unwrap();}
        let a=engine::actor(&r,100.).unwrap();let b=engine::actor(&r,101.).unwrap();
        assert_eq!(a.state,"ally");assert_eq!(b.state,"ally");assert!(!hostile(&r,a,b));
    }
    #[test] fn discord_haste_moves_only_without_ticking_status() {
        let mut r=room();creature(&mut r,100.,17.,14.,"neutral");creature(&mut r,101.,19.,14.,"neutral");
        execute(&mut r,ControlEffect::Discord,17.,14.).unwrap();let before=engine::condition(engine::actor(&r,100.).unwrap(),DISCORD);
        assert!(discord_turn(&mut r,100.,true).unwrap());let a=engine::actor(&r,100.).unwrap();
        assert_eq!((a.x,a.y),(18.,14.));assert_eq!(a.state,"neutral");assert_eq!(engine::condition(a,DISCORD),before);
        assert_eq!(engine::actor(&r,101.).unwrap().hp,100.);
    }
    #[test] fn roots_tick_four_times_block_only_movement_and_break_on_fire() {
        let mut r=room();creature(&mut r,100.,15.,14.,"hunting");execute(&mut r,ControlEffect::Roots,15.,14.).unwrap();
        assert!(engine::movement_held(&mut r,100.));assert!(engine::move_actor(&mut r,100.,1.,0.,true).unwrap());
        let held=engine::actor(&r,100.).unwrap();assert_eq!((held.x,held.y),(15.,14.));
        for _ in 0..4 {tick_actor(&mut r,100.).unwrap();}
        assert_eq!(engine::actor(&r,100.).unwrap().hp,88.);assert!(!engine::movement_held(&mut r,100.));
        execute(&mut r,ControlEffect::Roots,15.,14.).unwrap();engine::actor_mut_trusted(&mut r,100.).unwrap().conditions.insert("burning".into(),1.);
        tick_actor(&mut r,100.).unwrap();assert!(!engine::actor(&r,100.).unwrap().conditions.contains_key(ROOTS));
        assert_eq!(engine::actor(&r,100.).unwrap().hp,88.);
    }
    #[test] fn target_plane_visibility_range_and_revision_are_enforced() {
        let mut r=room();creature(&mut r,100.,19.,14.,"hunting");
        assert!(target_valid(&r,ControlEffect::Discord,19.,14.));assert!(!target_valid(&r,ControlEffect::Roots,19.,14.));
        engine::actor_mut_trusted(&mut r,100.).unwrap().conditions.insert("phantasmiiPlane".into(),1.);
        assert!(!target_valid(&r,ControlEffect::Discord,19.,14.));
        r.simulation_revision=49.0.into();assert!(!target_valid(&r,ControlEffect::Obstruction,20.,14.));
    }
    #[test] fn obstruction_preview_does_not_probe_hidden_creature() {
        let mut r=room();let i=item("obstruction");let before=preview(&r,&i,20.,14.);
        creature(&mut r,100.,20.,14.,"hunting");engine::actor_mut_trusted(&mut r,100.).unwrap().conditions.insert("invisible".into(),10.);
        r.player.conditions.shift_remove("allSeeingReveal");r.player.conditions.shift_remove("allSeeingDepth");
        // Compare the same terrain/senses with and without the unseen occupant.
        let with=preview(&r,&i,20.,14.);r.actors_at_mut(r.depth).clear();assert_eq!(with,preview(&r,&i,20.,14.));
        assert_eq!(before.len(),3);
    }
    #[test] fn offscreen_normal_ticks_expire_terrain_and_control_statuses() {
        let mut r=room();creature(&mut r,100.,16.,14.,"neutral");
        creature(&mut r,101.,16.,15.,"hunting");
        // Isolate temporary hostility from player-inflicted damage: Roots
        // normally provokes a neutral, independent of Discord. Paralysis keeps
        // these two fixture actors from attacking during the catch-up window.
        for id in [100.,101.] {engine::actor_mut_trusted(&mut r,id).unwrap().conditions.insert("paralyzed".into(),10.);}
        execute(&mut r,ControlEffect::Roots,16.,15.).unwrap();execute(&mut r,ControlEffect::Discord,16.,14.).unwrap();
        execute(&mut r,ControlEffect::Obstruction,20.,14.).unwrap();
        let original_depth=r.depth;let original_tick=r.tick;let w=r.floor().unwrap().width();
        // Move the player elsewhere; the visited source floor catches up exactly.
        r.depth=2.;r.tick=original_tick+8.;r.actors_at_mut(2.).clear();
        crate::offscreen::catch_up_floor(&mut r,original_depth).unwrap();
        let a=r.actors_at(original_depth).iter().find(|a|a.id==100.).unwrap();
        assert!(!a.conditions.contains_key(ROOTS));assert!(!a.conditions.contains_key(DISCORD));assert_eq!(a.state,"neutral");
        let rooted=r.actors_at(original_depth).iter().find(|a|a.id==101.).unwrap();
        assert!(!rooted.conditions.contains_key(ROOTS));assert_eq!(rooted.state,"hunting");
        let f=r.floors.iter().find(|f|f.depth==original_depth).unwrap();
        assert_eq!(f.tiles[14*w+20],"floor");assert!(f.revision50_temporary_terrain.is_missing());
    }
    #[test] fn roots_player_provenance_provokes_neutrals_on_and_offscreen() {
        let mut r=room();creature(&mut r,100.,16.,14.,"neutral");
        execute(&mut r,ControlEffect::Roots,16.,14.).unwrap();
        let mut active=r.clone();tick_actor(&mut active,100.).unwrap();
        let active=engine::actor(&active,100.).unwrap();assert_eq!(active.state,"hunting");assert_eq!(active.hp,97.);
        let original_depth=r.depth;let tick=r.tick;
        r.floor_mut().unwrap().last_tick=tick;r.floor_mut().unwrap().dirty=true;
        r.depth=2.;r.tick=tick+1.;r.actors_at_mut(2.).clear();
        crate::offscreen::catch_up_floor(&mut r,original_depth).unwrap();
        let offscreen=r.actors_at(original_depth).iter().find(|a|a.id==100.).unwrap();
        assert_eq!(offscreen.state,active.state);assert_eq!(offscreen.hp,active.hp);
        assert_eq!(engine::condition(offscreen,ROOTS),engine::condition(active,ROOTS));
    }
    #[test] fn free_command_spends_cost_but_not_status_or_world_clocks() {
        let mut r=room();r.player.abilities.push("isheilah-spellslinger".into());r.inventory.push(item("discord"));
        creature(&mut r,100.,15.,14.,"ally");let tick=r.tick;
        let result=crate::revision50_amulet_effects::command(&mut r,&crate::Command::UseAt{id:900.,x:15.,y:14.}).unwrap();
        assert_eq!(result,Some(false));assert_eq!(r.tick,tick);
        assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,20.);
        assert_eq!(engine::condition(engine::actor(&r,100.).unwrap(),DISCORD),6.);
    }

    #[test] fn direct_magic_honors_fated_duel_but_hazards_keep_existing_rules() {
        let mut r=room();creature(&mut r,100.,15.,14.,"hunting");creature(&mut r,101.,16.,14.,"hunting");
        r.player.active.push(crate::revision50::DUEL.into());
        for (k,v) in [("r50DuelX",12.),("r50DuelY",14.),("r50DuelDepth",r.depth),("r50DuelTarget",100.)] {r.player.conditions.insert(k.into(),v);}
        let player=r.player.id;let rng=r.rng;
        engine::damage_direct(&mut r,player,10.,101.,"acid",true).unwrap();assert_eq!(r.player.hp,100.);assert_eq!(r.rng,rng);
        engine::damage_direct(&mut r,player,10.,100.,"acid",true).unwrap();assert_eq!(r.player.hp,90.);
        engine::damage_environment(&mut r,player,10.,Some(101.),"fire",true).unwrap();assert_eq!(r.player.hp,80.);
        engine::damage_environment(&mut r,player,3.,Some(101.),"roots",true).unwrap();assert_eq!(r.player.hp,77.);
    }
    #[test] fn discord_curses_respect_willpower_and_distinct_shared_owners() {
        let mut r=room();creature(&mut r,100.,15.,14.,"ally");
        let player=r.player.id;r.inventory.push(Item{id:901.,kind:"ring".into(),effect:crate::revision50_rings::WILLPOWER.to_string().into(),equipped:"ring1".to_string().into(),..Default::default()});
        engine::actor_mut_trusted(&mut r,100.).unwrap().conditions.insert(DISCORD.into(),6.);
        assert!(crate::monster_statuses::apply_curse(&mut r,player,"slowed",5.,100.));assert_eq!(engine::condition(&r.player,"slowed"),3.);
        r.player.conditions.shift_remove("slowed");
        engine::actor_mut_trusted(&mut r,100.).unwrap().conditions.insert("halfHereClone".into(),1.);
        engine::actor_mut_trusted(&mut r,100.).unwrap().conditions.insert("halfHereOwner".into(),player);
        assert!(crate::monster_statuses::apply_curse(&mut r,player,"slowed",5.,100.));assert_eq!(engine::condition(&r.player,"slowed"),5.);
    }
    #[test] fn offscreen_discord_does_not_act_on_last_paralyzed_turn() {
        let mut r=room();creature(&mut r,100.,15.,14.,"neutral");creature(&mut r,101.,16.,14.,"neutral");
        execute(&mut r,ControlEffect::Discord,15.,14.).unwrap();
        engine::actor_mut_trusted(&mut r,100.).unwrap().conditions.insert("paralyzed".into(),1.);
        engine::actor_mut_trusted(&mut r,101.).unwrap().conditions.insert("paralyzed".into(),10.);
        let original=r.depth;r.floor_mut().unwrap().last_tick=r.tick;r.depth=2.;r.tick+=1.;r.actors_at_mut(2.).clear();
        crate::offscreen::catch_up_floor(&mut r,original).unwrap();
        let a=r.actors_at(original).iter().find(|a|a.id==100.).unwrap();let b=r.actors_at(original).iter().find(|a|a.id==101.).unwrap();
        assert_eq!(a.hp,100.);assert_eq!(b.hp,100.);assert_eq!(engine::condition(a,DISCORD),5.);
        assert_eq!((a.x,a.y),(15.,14.));
    }

    #[test] fn obstruction_only_center_must_be_visible_and_unknown_outer_cell_does_not_change_preview() {
        let mut r=room();let i=item("obstruction");let w=r.floor().unwrap().width();let hidden=13*w+20;
        r.player.conditions.shift_remove("allSeeingReveal");r.player.conditions.shift_remove("allSeeingDepth");
        let f=r.floor_mut().unwrap();f.lights.fill(100.);f.light_sources=Field::Missing;f.lights[hidden]=-100.;
        assert!(crate::vision::visible(&r,20,14));assert!(!crate::vision::visible(&r,20,13));
        assert!(target_valid(&r,ControlEffect::Obstruction,20.,14.));
        let before=preview(&r,&i,20.,14.);assert_eq!(before.len(),2);assert!(!before.contains(&(20,13)));
        r.floor_mut().unwrap().tiles[hidden]="wall".into();
        assert!(!target_valid(&r,ControlEffect::Obstruction,20.,14.));assert_eq!(preview(&r,&i,20.,14.),before);
        r.floor_mut().unwrap().tiles[hidden]="floor".into();execute(&mut r,ControlEffect::Obstruction,20.,14.).unwrap();
        assert_eq!(r.floor().unwrap().tiles[hidden],"wall");
    }

    #[test] fn duel_rejects_new_control_impacts_before_payment_but_existing_roots_persist() {
        let mut r=room();creature(&mut r,100.,16.,14.,"hunting");
        execute(&mut r,ControlEffect::Roots,16.,14.).unwrap();let depth=r.depth;
        let a=engine::actor_mut_trusted(&mut r,100.).unwrap();a.active.push(crate::revision50::DUEL.into());
        for (k,v) in [("r50DuelX",16.),("r50DuelY",14.),("r50DuelDepth",depth),("r50DuelTarget",101.)] {a.conditions.insert(k.into(),v);}
        for (kind,name) in [(ControlEffect::Beckoning,"beckoning"),(ControlEffect::Discord,"discord"),(ControlEffect::Roots,"roots")] {
            r.inventory=vec![item(name)];let before=r.clone();assert!(!target_valid(&r,kind,16.,14.));
            assert!(crate::revision50_amulet_effects::admission(&r,&crate::Command::UseAt{id:900.,x:16.,y:14.}).unwrap().is_err());
            assert_eq!(r,before);
        }
        tick_actor(&mut r,100.).unwrap();assert_eq!(engine::actor(&r,100.).unwrap().hp,97.);
        assert_eq!(engine::condition(engine::actor(&r,100.).unwrap(),ROOTS),3.);
    }

}
