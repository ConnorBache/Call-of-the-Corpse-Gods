//! R50 Teleportation and Floods. Geometry is shared by preview and execution;
//! committed displacement uses the engine arrival path and serialized RNG.
use crate::{Actor, Floor, Run, engine::{self, EngineError}, movement::Point};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WaterStop { Lanes, WholeFront }
// Each lane stops independently at existing water, per the designer decision.
const WATER_STOP: WaterStop = WaterStop::Lanes;

fn target(r: &Run, x: f64, y: f64) -> Option<&Actor> {
    if crate::portals::cell(crate::dreamwalking::floor_for(r,&r.player)?,x,y).is_none() {return None}
    r.actors_at(r.depth).iter().find(|a| crate::half_here::owner_id(r,a)!=r.player.id && a.hp>0. && !a.dead.value_or(false)
        && !crate::dreamwalking::inert(r,a) && !crate::revision50_terrain::is_soldier(a)
        && !crate::revision50_nightmares::is_decoy(a)
        && crate::dreamwalking::same_plane(r,&r.player,a)
        && crate::portals::same_position(r,a,x,y) && crate::vision::visible_actor(r,a))
}
pub fn target_valid(r:&Run,teleport:bool,x:f64,y:f64)->bool {
    if !crate::revision50::enabled(r) || !x.is_finite() || !y.is_finite() || x.fract()!=0. || y.fract()!=0. {return false}
    if crate::dreamwalking::floor_for(r,&r.player).and_then(|f|crate::portals::cell(f,x,y)).is_none()
        || crate::portals::same_position(r,&r.player,x,y) {return false}
    if teleport { target(r,x,y).is_some_and(|a|crate::portals::actor_chebyshev(r,&r.player,a)<=8.) }
    else {let dx=x-r.player.x;let dy=y-r.player.y;dx==0. || dy==0. || dx.abs()==dy.abs()}
}
pub fn target_blocker_ui(r:&Run,teleport:bool,x:f64,y:f64)->Option<&'static str> {
    if target_valid(r,teleport,x,y) {None}
    else if teleport {Some("Choose another visible creature within 8 tiles.")}
    else {Some("Choose a cardinal or diagonal direction for the wave.")}
}
fn direction(r:&Run,x:f64,y:f64)->(i32,i32) {
    let sign=|v:f64|if v==0. {0}else{v.signum() as i32};
    (sign(x-r.player.x),sign(y-r.player.y))
}
fn stops(f:&Floor,at:usize)->bool {
    matches!(f.tiles[at].as_str(),"water"|"shallow-water"|"wall"|"tree"|"chasm"|"pit")
        || f.doors.as_ref().and_then(|d|d.get(&at.to_string())).is_some_and(|d|!d.open)
}
/// At most 15 cells, with explicit independent-lane or whole-front water stop. Stop tests use pre-wave terrain,
/// never newly-created water. Preview masks unknown cells before terrain reads.
fn wave_cells(r:&Run,x:f64,y:f64,known_only:bool,policy:WaterStop)->Vec<usize> {
    let Some(f)=crate::dreamwalking::floor_for(r,&r.player) else{return vec![]};
    let (dx,dy)=direction(r,x,y);if (dx,dy)==(0,0){return vec![]}
    let mut cells=Vec::with_capacity(15);
    let mut blocked=[false;3];
    for step in 1..=5 {
        let mut front=Vec::with_capacity(3);
        let mut water=false;
        for lane in -1..=1 {
            if blocked[(lane+1) as usize] {continue}
            let px=r.player.x as i32+dx*step-dy*lane;
            let py=r.player.y as i32+dy*step+dx*lane;
            let Some(at)=crate::portals::cell(f,px as f64,py as f64) else{blocked[(lane+1) as usize]=true;continue};
            if known_only && !crate::vision::visible(r,px as usize,py as usize) {blocked[(lane+1) as usize]=true;continue}
            if matches!(f.tiles[at].as_str(),"water"|"shallow-water"){water=true;}
            if stops(f,at) {blocked[(lane+1) as usize]=true;continue}
            front.push(at);
        }
        if water && policy==WaterStop::WholeFront {break}
        cells.extend(front);
    }
    cells
}
pub fn preview(r:&Run,teleport:bool,x:f64,y:f64)->Vec<(usize,usize)> {
    if !target_valid(r,teleport,x,y){return vec![]}
    if teleport{return vec![(x as usize,y as usize)]}
    let w=crate::dreamwalking::floor_for(r,&r.player).unwrap().width();
    wave_cells(r,x,y,true,WATER_STOP).into_iter().map(|at|(at%w,at/w)).collect()
}
/// O(V*(A+P+B+H)) time and O(V) scratch. Destination safety includes standing
/// habitat, hazards, plane-local occupancy and terrain, but not explored state.
/// H is creature hazard-query work; P is portal lookup; B is active traits.
fn destinations(r:&Run,a:&Actor)->Vec<usize> {
    let Some(f)=r.floor() else{return vec![]};let w=f.width();
    (0..f.tiles.len()).filter(|&at| {
        let x=(at%w) as f64;let y=(at/w) as f64;
        !crate::portals::same_position(r,a,x,y)
            && !matches!(f.tiles[at].as_str(),"chasm"|"pit")
            && engine::actor_passable(r,a,x,y)
            && !engine::bulwark_blocks(r,a,x,y)
            && crate::travel::habitat_valid(f,a,x,y)
            && !crate::environment::creature_hazard_at(r,a,at)
            && !f.doors.as_ref().and_then(|d|d.get(&at.to_string())).is_some_and(|d|!d.open)
            && crate::portals::occupied_at(r,a,x,y,Some(a.id)).is_none()
    }).collect()
}
pub(crate) fn teleport(r:&mut Run,id:f64)->Result<(),EngineError> {
    crate::dreamwalking::with_actor(r,id,|r| {
        let a=engine::actor(r,id).unwrap().clone();
        let choices=destinations(r,&a);
        // A cramped floor fizzles without RNG. Do not reveal destination counts
        // through target admission, which would scout unexplored terrain.
        if choices.is_empty(){return Ok(())}
        let at=choices[(crate::rng::random(r)*choices.len() as f64).floor() as usize];
        let w=r.floor().unwrap().width();let old=a.y as usize*w+a.x as usize;
        let a=engine::actor_mut_trusted(r,id).unwrap();a.x=(at%w) as f64;a.y=(at/w) as f64;
        engine::finish_actor_arrival(r,id,old,false,None)?;
        Ok(())
    })
}
pub(crate) fn quench(f:&mut Floor,at:usize) {
    if crate::terrain_runtime::hellfire_at(f,at)
        || f.permanent_fire.as_ref().is_some_and(|v|v.contains(&(at as f64))) {return}
    if let Some(e)=f.environment.as_mut() {
        // Gas explosions and divine/persistent flames are not ordinary fire.
        if e.fire_kind.as_ref().and_then(|v|v[at].as_deref())
            .is_some_and(|kind|!matches!(kind,"plain"|"creature"|"ichor")) {return}
        e.fire[at]=0.;
        if let Some(v)=e.fire_kind.as_mut(){v[at]=None}
        if let Some(v)=e.fire_fresh.as_mut(){v[at]=false}
        if let Some(v)=e.fire_reaction.as_mut(){v[at]=0.}
        if let Some(p)=e.provenance.as_mut().and_then(|p|p.get_mut("fire")){p.owner[at]=-1.;p.player[at]=0.;}
    }
    f.dirty=true;
}
fn push(r:&mut Run,id:f64,dx:i32,dy:i32)->Result<(),EngineError> {push_steps(r,id,dx,dy,2)}
/// Read-only physical admission shared by pushes and Repulsion's center lottery.
/// Call in the target's floor context; movement/arrival still owns live effects.
pub(crate) fn push_step_open(r:&Run,a:&Actor,dx:i32,dy:i32)->bool {
    let (x,y)=(a.x+dx as f64,a.y+dy as f64);
    let Some(f)=r.floor() else{return false};
    let Some(at)=crate::portals::cell(f,x,y) else{return false};
    // No opening doors, diagonal squeezing or portal redirection.
    !matches!(f.tiles[at].as_str(),"wall"|"tree"|"chasm"|"pit")
        && !f.doors.as_ref().and_then(|d|d.get(&at.to_string())).is_some_and(|d|!d.open)
        && engine::actor_passable(r,a,x,y)
        && crate::portals::occupied_at(r,a,x,y,Some(a.id)).is_none()
        && crate::portals::step_options(r,a,x,y).iter().all(|&p|p==(x,y))
        && crate::movement::diagonal_step_allowed(&f.tiles,f.width(),Point{x:a.x as i32,y:a.y as i32},Point{x:x as i32,y:y as i32},false)
}
pub(crate) fn push_steps(r:&mut Run,id:f64,dx:i32,dy:i32,steps:usize)->Result<(),EngineError> {
    for _ in 0..steps {
        let Some(a)=engine::actor(r,id).filter(|a|a.hp>0.&&!a.dead.value_or(false)).cloned() else{break};
        if !push_step_open(r,&a,dx,dy) {break}
        let old=engine::actor_mut_trusted(r,id).unwrap().conditions.insert("forcedMove".into(),1.);
        let moved=engine::move_actor(r,id,dx as f64,dy as f64,true);
        if let Some(a)=engine::actor_mut_trusted(r,id) {
            a.conditions.shift_remove("forcedMove");
            if let Some(old)=old{a.conditions.insert("forcedMove".into(),old);}
        }
        moved?;
        if engine::actor(r,id).is_none_or(|b|(b.x,b.y)==(a.x,a.y)){break}
    }
    Ok(())
}
fn flood(r:&mut Run,x:f64,y:f64,policy:WaterStop)->Result<(),EngineError> {
    let source=r.player.clone();let (dx,dy)=direction(r,x,y);let cells=wave_cells(r,x,y,false,policy);
    let mut targets:Vec<_>=r.actors_at(r.depth).iter().filter(|a| a.hp>0. && !a.dead.value_or(false)
        && crate::dreamwalking::same_plane(r,&source,a)
        && crate::portals::cell(r.floor().unwrap(),a.x,a.y).is_some_and(|at|cells.contains(&at)))
        .map(|a|(a.id,a.x*dx as f64+a.y*dy as f64)).collect();
    // Front first, ID tie-break: deterministic collision behavior independent
    // of actor storage order. Damage uses the activation-time source for Reaping.
    targets.sort_by(|a,b|b.1.total_cmp(&a.1).then_with(||a.0.total_cmp(&b.0)));
    for (id,_) in targets {
        if engine::actor(r,id).is_none_or(|a|crate::revision50::duel_blocks(r,&source,a)) {continue}
        engine::damage_direct_with_source_snapshot(r,id,20.,&source,"water",true)?;
        push(r,id,dx,dy)?;
    }
    for &at in &cells {quench(r.floor_mut().unwrap(),at);}
    let replace:Vec<_>=cells.into_iter().filter(|&at|crate::revision50_control_amulets::can_replace_terrain(r,at)).collect();
    crate::revision50_control_amulets::add_temporary_terrain(r,&replace,"shallow-water",12.);
    Ok(())
}
pub fn execute(r:&mut Run,teleportation:bool,x:f64,y:f64)->Result<(),EngineError> {
    if !target_valid(r,teleportation,x,y){return Err(EngineError("Invalid amulet target.".into()))}
    if teleportation {let id=target(r,x,y).unwrap().id;teleport(r,id)}
    else {let id=r.player.id;crate::dreamwalking::with_actor(r,id,|r|flood(r,x,y,WATER_STOP))}
}

#[cfg(test)]
mod tests {
    use super::*;
    fn room()->Run {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=50.0.into();r.player.x=12.;r.player.y=14.;
        // This is an amulet-only R50 fixture, not a migration of legacy starters.
        r.player.abilities.clear();r.player.active.clear();
        r.player.conditions.insert("allSeeingReveal".into(),1.);
        r.player.conditions.insert("allSeeingDepth".into(),r.depth);
        let depth=r.depth;r.actors_at_mut(depth).clear();
        let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.doors=Default::default();
        f.environment=Default::default();f.permanent_fire=Default::default();f.hellfire=Default::default();
        f.razorleaves=Default::default();
        r
    }
    fn creature(r:&mut Run,id:f64,x:f64,y:f64) {
        let a=Actor{id,x,y,hp:100.,max_hp:100.,state:"hunting".into(),..Default::default()};
        let d=r.depth;r.actors_at_mut(d).push(a);
    }
    #[test] fn teleport_rejects_self_and_wrong_plane_without_rng() {
        let mut r=room();let rng=r.rng;
        assert!(!target_valid(&r,true,r.player.x,r.player.y));
        creature(&mut r,100.,14.,14.);
        engine::actor_mut_trusted(&mut r,100.).unwrap().conditions.insert("phantasmiiPlane".into(),1.);
        assert!(!target_valid(&r,true,14.,14.));assert_eq!(r.rng,rng);
    }
    #[test] fn teleport_selection_is_repeatable_unexplored_and_unoccupied() {
        let mut r=room();creature(&mut r,100.,14.,14.);
        let w=r.floor().unwrap().width();let desired=10*w+10;
        r.floor_mut().unwrap().tiles.fill("wall".into());
        for at in [14*w+14,14*w+12,desired]{r.floor_mut().unwrap().tiles[at]="floor".into();}
        r.floor_mut().unwrap().seen[desired]=false;
        let a=engine::actor(&r,100.).unwrap().clone();assert_eq!(destinations(&r,&a),vec![desired]);
        let mut other=r.clone();teleport(&mut r,100.).unwrap();teleport(&mut other,100.).unwrap();
        assert_eq!(r.rng,other.rng);
        let a=engine::actor(&r,100.).unwrap();assert_eq!((a.x,a.y),(10.,10.));
        assert_eq!(r.depth,other.depth);
    }
    #[test] fn teleport_no_destination_consumes_no_rng_or_position() {
        let mut r=room();creature(&mut r,100.,14.,14.);r.floor_mut().unwrap().tiles.fill("wall".into());
        let before=r.rng;teleport(&mut r,100.).unwrap();assert_eq!(r.rng,before);
        let a=engine::actor(&r,100.).unwrap();assert_eq!((a.x,a.y),(14.,14.));
    }
    #[test] fn wave_is_fifteen_cells_and_never_overwrites_stop_tile() {
        let mut r=room();assert_eq!(wave_cells(&r,20.,14.,false,WaterStop::Lanes).len(),15);
        let w=r.floor().unwrap().width();let at=14*w+15;r.floor_mut().unwrap().tiles[at]="water".into();
        let cells=wave_cells(&r,20.,14.,false,WaterStop::Lanes);
        assert!(!cells.contains(&at));assert!(!cells.contains(&(at+1)));
        assert_eq!(cells.len(),12);
        assert_eq!(wave_cells(&r,20.,14.,false,WaterStop::WholeFront).len(),6);
    }
    #[test] fn quench_preserves_permanent_flame() {
        let mut r=room();let f=r.floor_mut().unwrap();let at=14*f.width()+14;
        crate::environment::initialize(f).unwrap();
        let e=f.environment.as_mut().unwrap();e.fire[at]=1.;e.fire_kind.as_mut().unwrap()[at]=Some("plain".into());
        quench(f,at);assert_eq!(f.environment.as_ref().unwrap().fire[at],0.);
        f.permanent_fire=vec![at as f64].into();f.environment.as_mut().unwrap().fire[at]=1.;
        quench(f,at);assert_eq!(f.environment.as_ref().unwrap().fire[at],1.);
    }
    #[test] fn flood_hits_once_and_pushes_two_without_extra_rng() {
        let mut r=room();creature(&mut r,100.,14.,14.);let rng=r.rng;
        flood(&mut r,20.,14.,WaterStop::Lanes).unwrap();let a=engine::actor(&r,100.).unwrap();
        assert_eq!(a.hp,80.);assert_eq!((a.x,a.y),(16.,14.));assert_eq!(r.rng,rng);
    }
    #[test] fn teleport_dispatch_rejects_immutable_endpoints_before_payment() {
        for marker in [crate::revision50_nightmares::MARKER,crate::revision50_terrain::SOLDIER_KEY,"ahvamitt","phantasmiiBody"] {
            let mut r=room();creature(&mut r,100.,14.,14.);
            r.inventory.clear();
            let mut item=crate::Item{id:900.,kind:"amulet".into(),effect:"teleportation".to_string().into(),
                equipped:"amulet".to_string().into(),..Default::default()};
            crate::revision50_amulets::initialize_item(&mut item).unwrap();r.inventory.push(item);
            r.player.abilities.push("isheilah-spellslinger".into());
            engine::actor_mut_trusted(&mut r,100.).unwrap().conditions.insert(marker.into(),1.);
            let before=(r.rng,r.tick,r.player.energy,r.inventory[0].amulet_state.clone(),r.player.amulet_ledger.clone());
            assert!(!target_valid(&r,true,14.,14.),"{marker}");
            let command=crate::Command::UseAt{id:900.,x:14.,y:14.};
            assert!(crate::revision50_amulet_effects::admission(&r,&command).unwrap().is_err(),"{marker}");
            let result=engine::dispatch(&mut r,&command);
            if marker==crate::revision50_terrain::SOLDIER_KEY {
                // Soldiers are ephemeral collision endpoints, not serializable
                // actor records. Dispatch rejects this synthetic fixture even
                // before amulet admission; never relax persisted-state gates.
                assert!(result.is_err(),"{marker}");
            } else {
                assert!(!result.unwrap().took_turn,"{marker}");
            }
            assert_eq!((r.rng,r.tick,r.player.energy,r.inventory[0].amulet_state.clone(),r.player.amulet_ledger.clone()),before,"{marker}");
            let a=engine::actor(&r,100.).unwrap();assert_eq!((a.x,a.y),(14.,14.),"{marker}");
        }
    }
    #[test] fn teleport_still_admits_an_independent_mini_me() {
        let mut r=room();creature(&mut r,100.,14.,14.);
        engine::actor_mut_trusted(&mut r,100.).unwrap().conditions.insert("miniMe".into(),1.);
        assert!(target_valid(&r,true,14.,14.));
    }
    #[test] fn flood_duel_guard_skips_damage_and_push_without_curing_status_or_shielding_ground() {
        let mut r=room();creature(&mut r,100.,14.,14.);creature(&mut r,101.,14.,13.);
        let depth=r.depth;
        let a=engine::actor_mut_trusted(&mut r,100.).unwrap();a.active.push(crate::revision50::DUEL.into());
        for (key,value) in [("r50DuelTarget",101.),("r50DuelX",14.),("r50DuelY",14.),("r50DuelDepth",depth),("poisoned",3.),(crate::revision50_control_amulets::ROOTS,4.)] {
            a.conditions.insert(key.into(),value);
        }
        flood(&mut r,20.,14.,WaterStop::Lanes).unwrap();
        let a=engine::actor(&r,100.).unwrap();assert_eq!(a.hp,100.);assert_eq!((a.x,a.y),(14.,14.));
        assert_eq!(engine::condition(a,"poisoned"),3.);assert_eq!(engine::condition(a,crate::revision50_control_amulets::ROOTS),4.);
        let b=engine::actor(&r,101.).unwrap();assert_eq!(b.hp,80.);assert_eq!((b.x,b.y),(16.,13.));
        let f=r.floor().unwrap();assert_eq!(f.tiles[14*f.width()+14],"shallow-water");
    }

    #[test] fn production_flood_activation_pays_once_and_each_lane_stops_at_old_water() {
        for water in ["water","shallow-water"] {
            let mut r=room();r.inventory.clear();r.player.abilities.clear();r.player.active.clear();
            let mut item=crate::Item{id:900.,kind:"amulet".into(),effect:"floods".to_string().into(),equipped:"amulet".to_string().into(),..Default::default()};
            crate::revision50_amulets::initialize_item(&mut item).unwrap();item.amulet_state.as_mut().unwrap().energy=50.;r.inventory.push(item);
            let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[14*w+15]=water.into();
            creature(&mut r,100.,14.,13.);creature(&mut r,101.,16.,14.);
            let expected=wave_cells(&r,20.,14.,false,WaterStop::Lanes);assert_eq!(expected.len(),12);
            assert_eq!(preview(&r,false,20.,14.),expected.iter().map(|at|(at%w,at/w)).collect::<Vec<_>>());
            let (rng,energy,tick)=(r.rng,r.player.energy,r.tick);
            let command=crate::Command::UseAt{id:900.,x:20.,y:14.};
            assert_eq!(crate::revision50_amulet_effects::command(&mut r,&command).unwrap(),Some(true));
            assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,20.);assert_eq!((r.rng,r.player.energy,r.tick),(rng,energy,tick));
            let a=engine::actor(&r,100.).unwrap();assert_eq!((a.hp,a.x,a.y),(80.,16.,13.));
            let b=engine::actor(&r,101.).unwrap();assert_eq!((b.hp,b.x,b.y),(100.,16.,14.));
            let f=r.floor().unwrap();assert_eq!(f.tiles[14*w+15],water);assert_eq!(f.tiles[14*w+16],"floor");
            for at in expected {assert_eq!(f.tiles[at],"shallow-water");}
            assert_eq!(f.revision50_temporary_terrain.as_ref().unwrap().len(),12);
            r.tick+=12.;crate::revision50_control_amulets::expire_terrain(&mut r);
            assert_eq!(r.floor().unwrap().tiles[13*w+17],"floor");assert_eq!(r.floor().unwrap().tiles[14*w+15],water);
        }
    }
    #[test] fn production_flood_diagonal_stop_and_invalid_target_are_rng_atomic() {
        let mut r=room();let w=r.floor().unwrap().width();
        r.floor_mut().unwrap().tiles[16*w+14]="water".into();
        let cells=preview(&r,false,20.,22.);assert!(!cells.contains(&(14,16)));assert!(!cells.contains(&(15,17)));
        assert!(cells.contains(&(15,19)));assert!(cells.contains(&(18,18)));
        assert!(target_valid(&r,false,20.,22.));let before=r.clone();assert!(execute(&mut r,false,20.,21.).is_err());assert_eq!(r,before);
        execute(&mut r,false,20.,22.).unwrap();assert_eq!(r.rng,before.rng);assert_eq!(r.floor().unwrap().tiles[16*w+14],"water");
    }

}
