//! Authored R50 potion additions. Impact geometry is plane-local and deterministic;
//! the existing potion dispatcher owns consumption, dilution, resource ownership,
//! logs and transactional command rollback.
use crate::{Actor, Field, Floor, Run, engine::{self, EngineError}};
use serde::{Deserialize, Serialize};
pub const ORDINARY: [&str;20] = ["healing","energy","incineration","caustic gas","paralysis","confusion","telepathy","levitation","speed","invisibility","descent","cleansing","fire immunity","darkness","flooding","freezing","repulsion","discord","teleportation","shattering"];
pub const CURRENT_ORDINARY: [&str;17] = ["healing","energy","incineration","caustic gas","paralysis","confusion","telepathy","levitation","speed","invisibility","descent","fire immunity","darkness","freezing","discord","teleportation","shattering"];
pub const RETIRED_MESSAGE: &str = "This potion has been retired.";
pub fn retired(r:&Run,item:&crate::Item)->bool {
    crate::potion_rules::current(r) && item.kind=="potion" && matches!(item.effect.as_ref().map(String::as_str),Some("cleansing"|"flooding"|"repulsion"))
}
pub fn roster(current:bool)->&'static [&'static str] {if current {&CURRENT_ORDINARY} else {&ORDINARY}}
pub const FIRE_IMMUNITY:&str="r50FireImmunity";
pub fn new_effect(effect:&str)->bool {ORDINARY[11..].contains(&effect)}
pub fn condition_key(key:&str)->bool {matches!(key,FIRE_IMMUNITY|"r50FireImmunityFresh")}
pub fn fire_immune(a:&Actor)->bool {engine::condition(a,FIRE_IMMUNITY)>0.}
#[derive(Clone,Debug,PartialEq,Serialize,Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct Darkness {pub cell:usize,pub amount:f64,pub expires_at:f64}
/// Fixed area scan, O(radius²), O(radius²) scratch. No portal propagation or RNG.
fn square(f:&Floor,x:f64,y:f64,radius:i32)->Vec<usize> {
    (-radius..=radius).flat_map(|dy|(-radius..=radius).filter_map(move |dx|
        crate::portals::cell(f,x+dx as f64,y+dy as f64))).collect()
}
pub fn darkness_at(f:&Floor,at:usize,tick:f64)->f64 {
    f.revision50_darkness.as_ref().and_then(|v|v.iter().find(|d|d.cell==at&&d.expires_at>tick)).map_or(0.,|d|d.amount)
}
/// R52 changes effective illumination and dose geometry, not source powers or clocks.
pub fn absolute_darkness(r:&Run)->bool {r.simulation_revision.value_or(29.)>=52.}
/// O(D) time, O(1) scratch; D is the bounded, unique-per-cell potion mask.
pub fn absolute_darkness_at(r:&Run,f:&Floor,at:usize)->bool {
    absolute_darkness(r)&&darkness_at(f,at,r.tick)>0.
}
pub fn expire(r:&mut Run) {
    if !crate::revision50::enabled(r){return}
    let tick=r.tick;
    if r.floor().is_none_or(|f|f.revision50_darkness.as_ref().is_none_or(|v|!v.iter().any(|d|d.expires_at<=tick))){return}
    let f=r.floor_mut().unwrap();
    f.revision50_darkness.as_mut().unwrap().retain(|d|d.expires_at>tick);
    if f.revision50_darkness.as_ref().unwrap().is_empty(){f.revision50_darkness=Field::Missing;}
    f.dirty=true;
}
pub fn has_darkness(r:&Run,depth:f64)->bool {
    crate::revision50::enabled(r)&&r.floors.iter().chain(r.dreamwalking.as_ref().into_iter().flat_map(|s|s.floors.iter()))
        .any(|f|f.depth==depth&&f.revision50_darkness.as_ref().is_some_and(|v|!v.is_empty()))
}
pub fn validate(r:&Run)->Result<(),String> {
    for f in r.floors.iter().chain(r.dreamwalking.as_ref().into_iter().flat_map(|s|s.floors.iter())) {
        if let Some(v)=f.revision50_darkness.as_ref() {
            if !crate::revision50::enabled(r){return Err("Potion darkness requires revision 50".into())}
            let mut seen=std::collections::HashSet::new();
            if v.iter().any(|d|d.cell>=f.tiles.len()||!seen.insert(d.cell)||!d.amount.is_finite()||d.amount<=0.||d.amount>50.||!d.expires_at.is_finite()||d.expires_at<0.) {
                return Err("Invalid potion darkness".into())
            }
        }
    }Ok(())
}
fn mutable_target(r:&Run,id:f64)->bool {
    engine::actor(r,id).is_some_and(|a|a.hp>0.&&!a.dead.value_or(false)
        && !crate::dreamwalking::inert(r,a)&&!crate::revision50_nightmares::is_decoy(a)
        && !crate::revision50_terrain::is_soldier(a)&&crate::dreamwalking::same_plane(r,&r.player,a))
}
fn harmful_target(r:&Run,id:f64)->bool {
    mutable_target(r,id)&&engine::actor(r,id).is_some_and(|a|
        !crate::revision50::duel_blocks(r,&r.player,a)&&!crate::revision50_combat::oblivious_blocks(r,a,Some(&r.player)))
}
/// Clean only named temporary effects and their provenance; never blanket-retain
/// condition names, which also carry identity, clocks, allegiance and abilities.
fn cleanse(r:&mut Run,id:f64) {
    let owner=crate::half_here::owner_id(r,engine::actor(r,id).unwrap());
    // Capacity/defense are shared-owner stats; other timed penalties stay local.
    for recipient in [id,owner] {
        let a=engine::actor_mut_trusted(r,recipient).unwrap();
        let keys:Vec<_>=a.conditions.iter().filter_map(|(key,&value)| {
            let stat=key.strip_prefix("buff:")?;
            (value<0.&&engine::condition(a,&format!("buffTicks:{stat}"))>0.
                && (recipient==id||matches!(stat,"defense"|"maxHp"|"maxEnergy"))).then(||stat.to_string())
        }).collect();
        for stat in keys {a.conditions.shift_remove(&format!("buff:{stat}"));a.conditions.shift_remove(&format!("buffTicks:{stat}"));}
        if recipient==owner {break}
    }
    crate::revision50_control_amulets::break_roots(r,id);
    let a=engine::actor_mut_trusted(r,id).unwrap();
    for key in ["poisoned","slowed","hemophilia","bleeding","paralyzed","confused","hallucinating","weakened","unlucky","distracted","envenomed","venomTurns","silenced","riptide","disoriented","screamWeakened","gasVulnerableTicks","grabbed","stuck",crate::revision50_control_amulets::DISCORD,crate::revision50_amulet_effects::ACID_TURNS] {
        a.conditions.shift_remove(key);a.conditions.shift_remove(&format!("{key}Fresh"));
    }
    for key in ["poisonSourceId","slowSourceId","hemophiliaSourceId","bleedSourceId","disorientedPenalty","screamPenalty","gasVulnerable","grabbedBy","venomSourceId",crate::revision50_amulet_effects::POISON_DAMAGE,crate::revision50_amulet_effects::ACID_PENALTY] {a.conditions.shift_remove(key);}
    a.conditions.retain(|key,_|!key.starts_with("venom:"));
    crate::revision52_enemy_followups::clear_bleed(a);
    if !a.abilities.iter().any(|id|id=="enemy:beast:everburn") {
        for key in ["burning","burningOwner","burningPlayer"] {a.conditions.shift_remove(key);}
    }
}
/// R52 uses cell-center Euclidean radius 3: scan 49 positions, keep at most 29.
/// R50/R51 retain the exact square/order. Both ignore walls and portal topology;
/// the projectile dispatcher has already selected the real impact coordinate.
fn darkness_footprint(r:&Run,x:f64,y:f64)->Vec<usize> {
    let f=r.floor().unwrap();
    if !absolute_darkness(r) {return square(f,x,y,2)}
    (-3i32..=3).flat_map(|dy|(-3i32..=3).filter_map(move |dx| {
        if dx*dx+dy*dy>9 {None} else {crate::portals::cell(f,x+dx as f64,y+dy as f64)}
    })).collect()
}
/// O(49 + 29D), O(29) scratch for R52; D <= floor cells. Reapplication
/// refreshes the greater magnitude/duration and never stacks darkness.
fn darkness(r:&mut Run,x:f64,y:f64,strength:f64) {
    let cells=darkness_footprint(r,x,y);let expiry=r.tick+(20.*strength).ceil();
    let f=r.floor_mut().unwrap();f.dirty=true;
    let entries=f.revision50_darkness.get_or_insert_with(Vec::new);
    for cell in cells {
        if let Some(d)=entries.iter_mut().find(|d|d.cell==cell) {d.amount=d.amount.max(50.*strength);d.expires_at=d.expires_at.max(expiry);}
        else {entries.push(Darkness{cell,amount:50.*strength,expires_at:expiry});}
    }
}
/// Existing ice is an overlay: freeze/melt preserve the exact water/ground tile.
/// Exclude structures and non-Euclidean cells rather than overwrite topology.
pub fn ice_cell_valid(r:&Run,at:usize)->bool {
    let Some(f)=r.floor() else{return false};let w=f.width();
    f.tiles.get(at).is_some_and(|t|matches!(t.as_str(),"floor"|"grass"|"water"|"shallow-water"|"snow"|"sand"))
        && !f.doors.as_ref().is_some_and(|d|d.contains_key(&at.to_string()))
        && !f.grates.as_ref().is_some_and(|v|v.contains(&(at as f64)))
        && !crate::borge::solid_at(f,(at%w) as f64,(at/w) as f64)
        && !crate::epidite::solid_at(f,(at%w) as f64,(at/w) as f64)
        && crate::portals::pair_at(f,at).is_none()&&crate::portals::exits(f,at)==vec![at]&&crate::portals::aliases(f,at)==vec![at]
        && f.revision50_terrain.as_ref().is_none_or(|v|!v.iter().any(|m|m.cells.iter().any(|c|c.y as usize*w+c.x as usize==at)))
}
pub fn freeze_cells(r:&mut Run,cells:&[usize]) {
    if !crate::revision50::enabled(r){return}
    let cells:Vec<_>=cells.iter().copied().filter(|&at|ice_cell_valid(r,at)).collect();
    if cells.is_empty(){return}
    let f=r.floor_mut().unwrap();let n=f.tiles.len();f.dirty=true;
    let ice=f.ice.get_or_insert_with(||vec![false;n]);for at in cells {ice[at]=true;}
}
/// Provisional center default: sample one of the eight first-step-valid vectors.
/// O(8*Q) admission, Q shared collision/topology queries; at most eight choices.
fn repulsion_center_direction(r:&mut Run,id:f64)->Option<(i32,i32)> {
    let a=engine::actor(r,id).filter(|a|a.hp>0.&&!a.dead.value_or(false))?;
    let choices:Vec<_>=crate::movement::DIRECTIONS.into_iter().filter(|d| {
        let (x,y)=(a.x+d.x as f64,a.y+d.y as f64);
        crate::revision50_wave_teleport::push_step_open(r,a,d.x,d.y)
            && !engine::bulwark_blocks(r,a,x,y)
            // These player-only interaction cells also refuse forced movement.
            && (id!=r.player.id || if crate::dreamwalking::active(r) {
                !matches!(r.floor().unwrap().tiles[y as usize*r.floor().unwrap().width()+x as usize].as_str(),"up"|"down"|"pit")
            } else {
                !crate::borge::interaction_at(r,x,y)&&!crate::godbeast_interactions::lever_pending(r,x,y)
            })
    }).collect();
    if choices.is_empty(){return None}
    let d=choices[(crate::rng::random(r)*choices.len() as f64).floor() as usize];
    Some((d.x,d.y))
}
fn repulsion(r:&mut Run,x:f64,y:f64,strength:f64)->Result<(),EngineError> {
    let mut targets:Vec<_>=std::iter::once(&r.player).chain(r.actors_at(r.depth))
        .filter(|a|harmful_target(r,a.id)&&(a.x-x).hypot(a.y-y)<=3.)
        .map(|a|(a.id,a.x,a.y)).collect();
    // Furthest first, ID tie-break, so storage order cannot alter collisions.
    targets.sort_by(|a,b|((b.1-x).hypot(b.2-y)).total_cmp(&((a.1-x).hypot(a.2-y))).then(a.0.total_cmp(&b.0)));
    for (id,ax,ay) in targets {
        let sign=|v:f64|if v==0.{0}else{v.signum() as i32};let (mut dx,mut dy)=(sign(ax-x),sign(ay-y));
        if dx==0&&dy==0 {
            let Some(direction)=repulsion_center_direction(r,id) else{continue};
            (dx,dy)=direction;
        }
        crate::revision50_wave_teleport::push_steps(r,id,dx,dy,(3.*strength).ceil() as usize)?;
    }Ok(())
}
/// O((2R+1)²*(P+M)) topology/Claywall admission, R=6 current and 2 historical; doors/grates use existing small maps.
/// Ordinary walls, trees, doors, grates and Obstruction are destructible. Event
/// structures, boundary walls, portal cells and Claywalls are never changed.
fn shatter(r:&mut Run,x:f64,y:f64) {
    let revision = r.simulation_revision.value_or(29.);
    let radius=if crate::potion_rules::current(r) {6} else {2};
    let f=r.floor().unwrap();let w=f.width();let h=f.height();
    let cells:Vec<_>=square(f,x,y,radius).into_iter().filter(|&at| {
        let (xx,yy)=(at%w,at/w);
        (xx as f64-x).hypot(yy as f64-y)<=radius as f64&&xx>0&&yy>0&&xx+1<w&&yy+1<h
            &&!crate::borge::solid_at(f,xx as f64,yy as f64)&&!crate::epidite::solid_at(f,xx as f64,yy as f64)
            &&crate::portals::pair_at(f,at).is_none()&&crate::portals::exits(f,at)==vec![at]&&crate::portals::aliases(f,at)==vec![at]
            && f.permanent_fire.as_ref().is_none_or(|v|!v.contains(&(at as f64)))
            && f.vents.as_ref().is_none_or(|v|!v.iter().any(|v|v.at==at as f64))
            && f.light_sources.as_ref().is_none_or(|v|!v.iter().any(|v|v.x==xx as f64&&v.y==yy as f64))
            && f.godbeast_event.as_ref().is_none_or(|e| {
                !(e.x==xx as f64&&e.y==yy as f64)
                    && [e.altar.as_ref().map(|p|(p.x,p.y)),e.well.as_ref().map(|p|(p.x,p.y)),e.lever.as_ref().map(|p|(p.x,p.y))].into_iter().flatten().all(|(x,y)|x!=xx as f64||y!=yy as f64)
            })
            && f.revision50_terrain.as_ref().is_none_or(|v|!crate::revision50_terrain::clay_blocks(v,crate::revision50_terrain::Cell{x:xx as i32,y:yy as i32}))
    }).collect();
    // Capture whole shared objects before mutation so later impact cells cannot
    // re-shatter the original terrain restored by an earlier wall destruction.
    let wall_cells:Vec<_>=f.revision50_terrain.as_ref().into_iter().flatten()
        .filter(|m|m.kind==crate::revision50_terrain::Kind::Wall
            && m.cells.iter().any(|c|cells.contains(&(c.y as usize*w+c.x as usize))))
        .flat_map(|m|m.cells.iter().map(|c|c.y as usize*w+c.x as usize)).collect();
    let f=r.floor_mut().unwrap();
    for &at in &wall_cells {
        let result=crate::revision50_terrain::damage_wall_revision(f,crate::revision50_terrain::Cell{x:(at%w) as i32,y:(at/w) as i32},crate::revision50_terrain::WALL_HP,revision);
        debug_assert!(result.is_ok());
    }
    for at in cells {
        if wall_cells.contains(&at) {continue;}
        let mut changed=false;
        if matches!(f.tiles[at].as_str(),"wall"|"tree") {
            // Cancel temporary wall restoration before opening the terrain.
            crate::stone_seal_rules::forget_where(f, |cell| cell == at);
            let prior=f.revision50_temporary_terrain.as_ref().and_then(|v|v.iter().find(|t|t.cell==at&&t.tile=="wall")).map(|t|t.previous.clone());
            if let Some(v)=f.revision50_temporary_terrain.as_mut(){v.retain(|t|t.cell!=at||t.tile!="wall");}
            f.tiles[at]=prior.unwrap_or_else(||"floor".into());changed=true;
        }
        if let Some(d)=f.doors.as_mut(){changed|=d.shift_remove(&at.to_string()).is_some();}
        if let Some(g)=f.grates.as_mut(){let before=g.len();g.retain(|v|*v!=at as f64);changed|=before!=g.len();}
        if changed {f.dirty=true;crate::ichor::enqueue_opening_revision(f,at,revision);}
    }
}
/// A five-cell-diameter circle. O(25 + A) world queries and O(A) target IDs;
/// fire admission retains water, walls, immunity and source ownership rules.
pub fn incinerate(r:&mut Run,x:f64,y:f64,strength:f64)->Result<(),EngineError> {
    let cells:Vec<_>=square(r.floor().unwrap(),x,y,2).into_iter().filter(|&at| {
        let w=r.floor().unwrap().width();
        ((at%w) as f64-x).hypot((at/w) as f64-y)<=2.
    }).collect();
    let w=r.floor().unwrap().width();
    let targets:Vec<_>=std::iter::once(&r.player).chain(r.actors_at(r.depth))
        .filter(|a|mutable_target(r,a.id)&&cells.contains(&(a.y as usize*w+a.x as usize)))
        .map(|a|a.id).collect();
    for at in cells {
        crate::environment::ignite(r,(at%w) as f64,(at/w) as f64,Some((8.*strength).ceil()),Some(r.player.id)).map_err(EngineError)?;
    }
    for id in targets {
        let random=crate::rng::random(r);let revision=r.simulation_revision.value_or(29.);
        crate::corpses::apply_burning_run(r,id,revision,random,Some(-2.),Some(1.),Some(strength));
    }
    Ok(())
}
pub fn apply(r:&mut Run,effect:&str,target:Option<f64>,x:f64,y:f64,strength:f64,fresh:bool)->Result<bool,EngineError> {
    if !new_effect(effect){return Ok(false)}
    if crate::potion_rules::current(r)&&matches!(effect,"cleansing"|"flooding"|"repulsion") {return Err(EngineError(RETIRED_MESSAGE.into()))}
    if !crate::revision50::enabled(r){return Err(EngineError("This potion requires revision 50.".into()))}
    let target=target.filter(|&id|mutable_target(r,id));
    match effect {
        "cleansing"=>{if let Some(id)=target {cleanse(r,id)}},
        "fire immunity"=>{if let Some(id)=target {crate::potions::timed_status(r,id,FIRE_IMMUNITY,20.*strength,fresh);}},
        "darkness"=>darkness(r,x,y,strength),
        "flooding"=>{
            let cells:Vec<_>=square(r.floor().unwrap(),x,y,1).into_iter().filter(|&at|crate::revision50_control_amulets::can_replace_terrain(r,at)).collect();
            for &at in &cells {crate::revision50_wave_teleport::quench(r.floor_mut().unwrap(),at);}
            crate::revision50_control_amulets::add_temporary_terrain(r,&cells,"shallow-water",(20.*strength).ceil());
        },
        "freezing"=>{
            let f=r.floor().unwrap();let cells=square(f,x,y,1);let w=f.width();
            let targets:Vec<_>=std::iter::once(&r.player).chain(r.actors_at(r.depth)).filter(|a|harmful_target(r,a.id)&&cells.contains(&(a.y as usize*w+a.x as usize))
                &&!crate::revision50_rings::has_effect(r,a,crate::revision50_rings::ICE_IMMUNITY)).map(|a|a.id).collect();
            let water:Vec<_>=cells.into_iter().filter(|&at|matches!(f.tiles[at].as_str(),"water"|"shallow-water")).collect();
            freeze_cells(r,&water);
            for id in targets {crate::monster_statuses::apply_status(r,id,"slowed",3.*strength,Some(r.player.id));}
        },
        "repulsion"=>repulsion(r,x,y,strength)?,
        "discord"=>{if let Some(id)=target.filter(|&id|harmful_target(r,id)){crate::potions::timed_status(r,id,crate::revision50_control_amulets::DISCORD,10.*strength,fresh);}},
        "teleportation"=>{if let Some(id)=target.filter(|&id|harmful_target(r,id)){crate::revision50_wave_teleport::teleport(r,id)?;}},
        "shattering"=>shatter(r,x,y),
        _=>unreachable!(),
    }Ok(true)
}

#[cfg(test)]
#[path="revision50_potions_tests.rs"]
mod tests;
#[cfg(test)]
#[path="revision52_darkness_tests.rs"]
mod revision52_darkness_tests;
