//! Revision33 God-Obelisk: one selected actor, one exact heart, one single-use weapon.
//! All state lives in the ordinary Floor/Item snapshots; no external quest cache or RNG.
use crate::{Actor, Field, Floor, GodbeastEvent, GodbeastEventRoom, GodbeastEventTap, GroundItem, Item, Run};
use crate::engine::{self, EngineError};

pub const SHARD_NAME: &str = "Shard of the Borge";
pub const SHARD_DESCRIPTION: &str = "A shard of the dark obelisk. Any creature which it bleeds will fall into an endless nightmare. It can only be used once.";
pub const DEMAND: &str = "The Borge demands a sacrifice...";
pub const RADIUS: i32 = 2;

pub fn throw_offering_enabled(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 52. && matches!(r.borge_throw_from, Field::Value(_))
}
pub(crate) fn validate(r: &Run) -> Result<(), String> {
    if !r.borge_throw_from.is_missing() && (r.simulation_revision.value_or(29.) < 52.
        || !matches!(r.borge_throw_from, Field::Value(n) if n <= r.replay.len())) {
        return Err("Invalid Borge throw command boundary".into());
    }
    Ok(())
}
/// Adopt after the historical command prefix, not while replaying a save journal.
pub fn admit(r: &mut Run) -> Result<(), String> {
    validate(r)?;
    if r.simulation_revision.value_or(29.) >= 52. && r.borge_throw_from.is_missing() {
        r.borge_throw_from = r.replay.len().into();
    }
    Ok(())
}
pub(crate) fn apply_replay_policy(r: &mut Run, from: &Field<usize>, cursor: usize) {
    if from.as_ref() == Some(&cursor) { r.borge_throw_from = cursor.into(); }
    else if cursor == 0 { r.borge_throw_from = Field::Missing; }
}

pub fn weapon_enabled(r: &Run) -> bool { r.simulation_revision.value_or(29.) >= 39. }
pub fn is_shard(item: &Item) -> bool { item.kind == "weapon" && item.name == SHARD_NAME }
pub fn shard(id: f64) -> Item {
    Item { id, name: SHARD_NAME.into(), kind: "weapon".into(), glyph: "/".into(),
        hands: 1.0.into(), damage: 0.0.into(), offense_multiplier: 0.0.into(),
        identified: true.into(), effect: SHARD_DESCRIPTION.to_string().into(), ..Default::default() }
}
pub fn contains(e: &GodbeastEvent, x: f64, y: f64) -> bool {
    e.kind == "Borge" && (x-e.x).abs() <= RADIUS as f64 && (y-e.y).abs() <= RADIUS as f64
}
pub fn solid_at(f: &Floor, x: f64, y: f64) -> bool {
    f.godbeast_event.as_ref().is_some_and(|e| contains(e,x,y))
        && x>=0. && y>=0. && x<f.width() as f64 && y<f.height() as f64
        && f.tiles[y as usize*f.width()+x as usize] != "chasm"
}
/// Body and altar are reserved against hostile occupancy, independently of
/// terrain replacement. O(1), no RNG or serialized state.
pub fn reserved_cell(f: &Floor, at: usize) -> bool {
    if at >= f.tiles.len() || f.width() == 0 { return false; }
    let (x,y)=((at%f.width()) as f64,(at/f.width()) as f64);
    f.godbeast_event.as_ref().is_some_and(|e| e.kind=="Borge"
        && (contains(e,x,y) || e.altar.as_ref().is_some_and(|p|p.x==x&&p.y==y)))
}
pub fn interaction_at(r: &Run, x: f64, y: f64) -> bool {
    if !x.is_finite() || !y.is_finite() || x.fract()!=0. || y.fract()!=0. { return false; }
    r.floor().is_some_and(|f| f.godbeast_event.as_ref().is_some_and(|e| e.kind=="Borge"
        && (solid_at(f,x,y) || e.altar.as_ref().is_some_and(|p| p.x==x && p.y==y
            && f.tiles[y as usize*f.width()+x as usize] != "chasm"))))
}
/// O(V*A) for A<=2 avoided stair cells, O(1) scratch. Nearest safe central placement;
/// stable y/x ties preserve determinism. The 25-cell solid body has a clear two-cell ring.
pub fn reserve_room(tiles: &mut [String], avoid: &[crate::generation::Point]) -> Result<GodbeastEvent,String> {
    let w = if tiles.len()==2291 {79} else {(tiles.len() as f64).sqrt() as usize};
    if w==0 || tiles.len()%w!=0 { return Err("Invalid Borge grid".into()); }
    let h=tiles.len()/w;
    let mut best=None;
    for y in 5..h as i32-5 { for x in 5..w as i32-5 {
        if avoid.iter().any(|p| (p.x-x).abs()<=6 && (p.y-y).abs()<=6) {continue;}
        let score=(x-w as i32/2).pow(2)+(y-h as i32/2).pow(2);
        if best.is_none_or(|(old,_,_)| score<old) { best=Some((score,x,y)); }
    }}
    let (_,x,y)=best.ok_or("Grid too small for central Borge room")?;
    for yy in y-4..=y+4 { for xx in x-4..=x+4 {
        tiles[yy as usize*w+xx as usize]=if (xx-x).abs()<=RADIUS && (yy-y).abs()<=RADIUS {"wall"} else {"floor"}.into();
    }}
    Ok(GodbeastEvent { kind:"Borge".into(), x:x as f64,y:y as f64,
        room:GodbeastEventRoom {left:(x-4) as f64,right:(x+4) as f64,top:(y-4) as f64,bottom:(y+4) as f64},
        altar:GodbeastEventTap {x:x as f64,y:(y+3) as f64}.into(), ..Default::default() })
}
fn eligible(r: &Run, a: &Actor) -> bool {
    a.id!=r.player.id && !a.dead.value_or(false) && a.hp>0. && !a.heartless.value_or(false)
        && !crate::half_here::is_clone(a) && a.conditions.get("miniMe")!=Some(&1.) && a.name!="Mini-Me"
        && !is_marked_target(r,a)
        && !crate::monster_inventor::is_mech(a)
        && !a.abilities.iter().any(|s| s=="enemy:beast:sphinx")
        && !a.active.iter().any(|s| s=="morae-turning-the-wheel")
        && crate::inventory::heart_for(a,0.).is_ok()
}
/// O(A*(F+B+H)) selection, F<=40 event scans, B actor trait keys, H heart-template clone. No sorting/RNG. Max HP, then DR, then Defense; lowest actor ID wins ties.
pub fn strongest(r: &Run) -> Option<&Actor> {
    r.actors_at(r.depth).iter().filter(|a| eligible(r,a)).max_by(|a,b| {
        a.max_hp.total_cmp(&b.max_hp).then_with(||a.stats.dr.total_cmp(&b.stats.dr))
            .then_with(||a.stats.defense.total_cmp(&b.stats.defense)).then_with(||b.id.total_cmp(&a.id))
    })
}
pub fn is_marked_target(r: &Run, a: &Actor) -> bool {
    r.floors.iter().any(|f| f.godbeast_event.as_ref().is_some_and(|e| e.kind=="Borge"
        && !e.reward_created.value_or(false) && e.sacrifice_actor_id.as_ref()==Some(&a.id)))
}
/// O(F), F<=40. Only a chosen heart receives provenance, preserving ordinary Jackal stacks.
pub fn mark_heart(r: &Run, a: &Actor, item: &mut Item) {
    if let Some(f)=r.floors.iter().find(|f| f.godbeast_event.as_ref().is_some_and(|e| e.kind=="Borge"
        && e.sacrifice_actor_id.as_ref()==Some(&a.id))) {
        item.source_actor_id=a.id.into(); item.borge_depth=f.depth.into();
    }
}
pub fn matching_heart(e: &GodbeastEvent, depth: f64, item: &Item) -> bool {
    item.kind=="heart" && e.sacrifice_actor_id.as_ref().is_some()
        && e.sacrifice_actor_id==item.source_actor_id && item.borge_depth.as_ref()==Some(&depth)
}
fn altar_at(f: &Floor, e: &GodbeastEvent, x: f64, y: f64) -> bool {
    e.kind == "Borge" && e.altar.as_ref().is_some_and(|p| p.x == x && p.y == y)
        && crate::portals::cell(f, x, y).is_some_and(|at| f.tiles[at] != "chasm")
}
pub(crate) fn walk_over_satisfied_altar(r: &Run, x: f64, y: f64) -> bool {
    throw_offering_enabled(r) && r.floor().is_some_and(|f| f.godbeast_event.as_ref().is_some_and(|e|
        e.reward_created.value_or(false) && !contains(e, x, y) && altar_at(f, e, x, y)))
}
pub(crate) fn accept_thrown_heart(r: &mut Run, index: usize, item: &Item, landing: crate::projectiles::Landing) -> Result<bool,EngineError> {
    let (x, y) = (landing.x, landing.y);
    if item.kind != "heart" || !throw_offering_enabled(r) || crate::dreamwalking::active(r)
        || landing.actor_id.is_some() || (x, y) == (r.player.x, r.player.y)
        || !r.floor().is_some_and(|f| f.godbeast_event.as_ref().is_some_and(|e|
            e.activated.value_or(false) && !e.reward_created.value_or(false)
                && altar_at(f, e, x, y) && matching_heart(e, r.depth, item))) {
        return Ok(false);
    }
    let heart = crate::inventory::take_one(r, index);
    crate::events::projectile_item(r, heart.id);
    complete_offering(r, x, y, true)?;
    Ok(true)
}

fn complete_offering(r: &mut Run, x: f64, y: f64, on_altar: bool) -> Result<(),EngineError> {
    let reward = shard(r.next_id);
    let reward_id = reward.id;
    r.next_id += 1.;
    if on_altar {
        // A completed ritual puts its reward on the exact altar tile, even if
        // another item is already there. Normal scattered-drop rules don't apply.
        r.ground_at_mut(r.depth).push(GroundItem {x, y, item: reward, auto_pickup_blocked: Field::Missing});
        if crate::remains_rules::current(r) {
            crate::corpse_placement::vacate_for_item(r,r.depth,x,y,false,None).map_err(EngineError)?;
        }
        r.floor_mut().unwrap().dirty = true;
    } else {
        r.inventory.push(reward);
    }
    r.floor_mut().unwrap().godbeast_event.as_mut().unwrap().reward_created=true.into();
    engine::log(r,"The offered heart transforms into a Shard of the Borge.","player");
    crate::events::actor_event(r,&r.player,"pulse","Shard of the Borge",Some(crate::events::EventPoint{x,y}),None);
    if !on_altar { crate::events::inventory(r,"E74","diamond",Some(reward_id)); }
    Ok(())
}
pub fn interact(r: &mut Run,x:f64,y:f64) -> Result<bool,EngineError> {
    if !interaction_at(r,x,y) || (x-r.player.x).abs().max((y-r.player.y).abs())>1. {return Ok(false);}
    let e=r.floor().unwrap().godbeast_event.as_ref().unwrap().clone();
    if contains(&e,x,y) {
        if e.activated.value_or(false) {
            engine::log(r,if e.reward_created.value_or(false) {"The Borge has accepted its sacrifice."}
                else {"The Borge waits for the heart it marked."},"environment");
            return Ok(false);
        }
        let target=strongest(r).map(|a|(a.id,a.name.clone()));
        let next=r.floor_mut().unwrap().godbeast_event.as_mut().unwrap();
        next.activated=true.into();
        if let Some((id,name))=target { next.sacrifice_actor_id=id.into();next.sacrifice_name=name.into(); }
        crate::events::at(r,"E73","fracture-amber",crate::events::Stage::Special,&[crate::events::EventPoint{x,y}]);
        engine::log(r,DEMAND,"environment");
        engine::log(r,if r.floor().unwrap().godbeast_event.as_ref().unwrap().sacrifice_actor_id.as_ref().is_some() {
            "The Borge marks its sacrifice. Bring that creature's heart to the altar."
        } else {"No living creature on this floor can yield the sacrifice."},"environment");
        return Ok(true);
    }
    if !e.activated.value_or(false) {
        engine::log(r,"Strike the Borge to learn its demand.","environment");return Ok(false);
    }
    if e.reward_created.value_or(false) {
        engine::log(r,"The Borge is satisfied.","environment");return Ok(false);
    }
    let Some(index)=r.inventory.iter().position(|i|matching_heart(&e,r.depth,i)) else {
        engine::log(r,"The altar accepts only the exact heart marked by the Borge.","environment");return Ok(false);
    };
    crate::inventory::take_one(r,index);
    complete_offering(r, x, y, false)?;
    Ok(true)
}
/// Ordinary adjacent melee geometry, but no accuracy roll or creature immunity.
/// Targets include allies/neutral creatures; environmental fixtures are not actors.
pub fn use_blocker(r:&Run,item:&Item,x:f64,y:f64)->Option<&'static str> {
    if weapon_enabled(r) || !is_shard(item) {return Some("This item has no targeted use.");}
    if r.status!="playing" || r.player.dead.value_or(false) || r.player.hp<=0. {return Some("You cannot use the shard now.");}
    if !x.is_finite() || !y.is_finite() || x.fract()!=0. || y.fract()!=0.
        || (x-r.player.x).abs().max((y-r.player.y).abs())!=1. {return Some("Choose an adjacent creature.");}
    let Some(f)=r.floor() else {return Some("No floor.");};
    if x<0. || y<0. || x>=f.width() as f64 || y>=f.height() as f64 || solid_at(f,x,y) {return Some("Choose an adjacent creature.");}
    if !engine::actor_passable(r,&r.player,x,y) { return Some("That creature is blocked from melee reach."); }
    if !crate::movement::diagonal_step_allowed(&f.tiles,f.width(),
        crate::movement::Point{x:r.player.x as i32,y:r.player.y as i32},
        crate::movement::Point{x:x as i32,y:y as i32},
        r.player.active.iter().any(|s|s=="etill-phase-through")) {return Some("That creature is blocked from melee reach.");}
    if !r.actors_at(r.depth).iter().any(|a|a.x==x && a.y==y && !a.dead.value_or(false) && a.hp>0.) {return Some("Choose a living creature.");}
    None
}
pub fn use_at(r:&mut Run,id:f64,x:f64,y:f64)->Result<bool,EngineError> {
    let Some(index)=r.inventory.iter().position(|i|i.id==id) else {return Ok(false);};
    if use_blocker(r,&r.inventory[index],x,y).is_some() {return Ok(false);}
    let target=r.actors_at(r.depth).iter().find(|a|a.x==x && a.y==y && !a.dead.value_or(false) && a.hp>0.).unwrap().clone();
    let owner=crate::half_here::owner_id(r,&target);
    let before=crate::equipment::capture_before(r);
    crate::inventory::take_one(r,index);
    let victim=engine::actor(r,owner).unwrap().clone();
    crate::animation_enemy::mark(r,&target,"E74","puncture",crate::events::Stage::Special);
    crate::resources::record_loss(r,&victim,"hp",victim.hp.max(0.));
    let a=engine::actor_mut_trusted(r,owner).unwrap();
    a.hp=0.;
    if !a.abilities.iter().any(|s|s=="enemy:beast:no-revival") {a.abilities.push("enemy:beast:no-revival".into());}
    engine::resolve_death(r,owner,Some(r.player.id),Some("borge-shard"),true)?;
    crate::equipment::settle_removed(r,before)?;
    crate::animation_enemy::mark(r,&r.player,"E74","dissolve-shard",crate::events::Stage::Special);
    engine::log(r,"The Shard of the Borge disintegrates.","player");
    crate::events::actor_event(r,&r.player,"attack","Shard of the Borge",Some(crate::events::EventPoint{x,y}),None);
    Ok(true)
}
pub fn equipped_shard_id(r: &Run) -> Option<f64> {
    r.inventory.iter().filter(|i|is_shard(i))
        .filter(|i|i.equipped.as_ref().is_some_and(|s|s=="hand1" || s=="hand2"))
        .min_by_key(|i|if i.equipped.as_ref().is_some_and(|s|s=="hand1") {0} else {1}).map(|i|i.id)
}
pub fn equipped_attack(r:&mut Run,target:f64)->Result<bool,EngineError> {
    let Some(id)=equipped_shard_id(r) else {return Ok(false);};
    let Some(a)=engine::actor(r,target) else {return Ok(false);};
    use_at(r,id,a.x,a.y)
}

/// A successful weapon hit is terminal even for regenerative creatures. Shared
/// bodies resolve to their owner once; ordinary loot and terminal boss hooks run.
pub fn kill_on_hit(r:&mut Run,target:f64,source:f64)->Result<(),EngineError> {
    let Some(target_actor)=engine::actor(r,target).cloned() else {return Ok(())};
    let owner=crate::half_here::owner_id(r,&target_actor);
    let Some(victim)=engine::actor(r,owner).cloned() else {return Ok(())};
    if victim.dead.value_or(false)||victim.hp<=0. {return Ok(())}
    crate::animation_enemy::mark(r,&target_actor,"E74","puncture",crate::events::Stage::Special);
    crate::resources::record_loss(r,&victim,"hp",victim.hp.max(0.));
    let a=engine::actor_mut_trusted(r,owner).unwrap();a.hp=0.;
    if !a.abilities.iter().any(|s|s=="enemy:beast:no-revival") {a.abilities.push("enemy:beast:no-revival".into());}
    engine::resolve_death(r,owner,Some(source),Some("borge-shard"),true)?;
    crate::half_here::sync(r);
    Ok(())
}
pub fn consume_weapon(r:&mut Run,id:f64)->Result<(),EngineError> {
    let Some(index)=r.inventory.iter().position(|i|i.id==id && is_shard(i)) else {return Ok(())};
    let before=crate::equipment::capture_before(r);
    crate::inventory::take_one(r,index);
    crate::equipment::settle_removed(r,before)?;
    crate::animation_enemy::mark(r,&r.player,"E74","dissolve-shard",crate::events::Stage::Special);
    engine::log(r,"The Shard of the Borge disintegrates.","player");
    Ok(())
}

#[cfg(test)]
#[path="borge_occupancy_tests.rs"]
mod occupancy_tests;

/// Admission-only repair for saves made before hostile Borge occupancy was
/// forbidden. Plan all relocations first, stable distance/y/x ties, no RNG.
/// O(F+A+T*V*(P+A)) time and O(A+T+P) scratch (T trapped actors, V cells,
/// P portal aliases/topology); restricted to Borge floors at admission.
pub fn repair_hostile_occupancy(r: &mut Run) -> Result<(), String> {
    if r.simulation_revision.value_or(29.)<52. {return Ok(());}
    let mut plans=Vec::new();
    for f in &r.floors {
        if !f.godbeast_event.as_ref().is_some_and(|e|e.kind=="Borge") {continue;}
        let w=f.width();
        let mut occupied:std::collections::HashSet<usize>=r.actors_at(f.depth).iter()
            .filter(|a|!a.dead.value_or(false)&&a.hp>0.)
            .filter_map(|a|crate::portals::cell(f,a.x,a.y)).collect();
        if f.depth==r.depth {if let Some(at)=crate::portals::cell(f,r.player.x,r.player.y){occupied.insert(at);}}
        for a in r.actors_at(f.depth) {
            if a.dead.value_or(false)||a.hp<=0.||!crate::revision51_stairs::restricted(r,a)
                || !crate::portals::cell(f,a.x,a.y).is_some_and(|at|reserved_cell(f,at)) {continue;}
            let target=(0..f.tiles.len()).filter(|&at| {
                !reserved_cell(f,at)&&!occupied.contains(&at)
                    && !crate::portals::aliases(f,at).iter().any(|alias|occupied.contains(alias))
                    && matches!(f.tiles[at].as_str(),"floor"|"grass"|"shallow-water"|"water")
                    && !crate::environment::blocks_movement(f,at)
                    && !crate::revision51_stairs::stair_cell(f,at)
                    && crate::ichor::habitat_path_allowed(f,a,at)
            }).min_by_key(|&at|(((at%w) as i32-a.x as i32).abs().max(((at/w) as i32-a.y as i32).abs()),at));
            let at=target.ok_or("No valid square to relocate a creature off the Borge")?;
            occupied.insert(at);plans.push((f.depth,a.id,(at%w) as f64,(at/w) as f64));
        }
    }
    for (depth,id,x,y) in plans {
        if let Some(a)=r.actors_at_mut(depth).iter_mut().find(|a|a.id==id) {a.x=x;a.y=y;}
    }
    Ok(())
}
