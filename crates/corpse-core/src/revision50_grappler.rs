//! Revision 50 Grappler: translate a captured group without overlap or squeezing.
use crate::{Actor, Run, engine::{self, EngineError}, movement::{self, Point}};
pub const ID: &str = "shrail-grappler";
fn active(r: &Run) -> bool {
    crate::revision50::enabled(r) && r.player.hp > 0. && !r.player.dead.value_or(false)
        && r.player.active.iter().any(|id| id == ID)
}
fn eligible(r: &Run, a: &Actor) -> bool {
    a.id != r.player.id && a.hp > 0. && !a.dead.value_or(false)
        && a.state != "ally" && a.state != "neutral"
        && crate::dreamwalking::same_plane(r, &r.player, a)
        && crate::portals::actor_chebyshev(r, &r.player, a) <= 1.
}
/// Place at the actual translation boundary, after attack/interact branches.
/// Forced movement remains available for dragging and existing displacement.
pub fn locked(r: &Run, a: &Actor) -> bool {
    active(r) && eligible(r, a) && engine::condition(a, "forcedMove") == 0.
}
#[derive(Debug, Clone)]
pub struct DragSnapshot { depth: f64, x: f64, y: f64, actors: Vec<(f64, f64, f64)> }
pub fn snapshot(r: &Run) -> Option<DragSnapshot> {
    active(r).then(|| DragSnapshot { depth: r.depth, x: r.player.x, y: r.player.y,
        actors: r.actors_at(r.depth).iter().filter(|a| eligible(r,a)).map(|a|(a.id,a.x,a.y)).collect() })
}
#[derive(Clone, Copy, Debug)]
struct Translation { id: f64, from: (f64,f64), to: (f64,f64), allowed: bool }
/// Pure simultaneous collision plan. All targets share a translation vector,
/// so descending projection is a dependency order: each destination occupant
/// precedes the actor that needs its source cell. Blocked chains stay in place.
/// O(K log K + K*A), O(A+K) scratch for K captured actors and A occupants.
fn plan(mut candidates: Vec<Translation>, mut occupied: Vec<(f64,f64,f64)>, dx:f64, dy:f64) -> Vec<Translation> {
    candidates.sort_by(|a,b| {
        let ap=a.from.0*dx+a.from.1*dy; let bp=b.from.0*dx+b.from.1*dy;
        bp.total_cmp(&ap).then_with(||a.id.total_cmp(&b.id))
    });
    let mut moves=Vec::new();
    for c in candidates {
        if !c.allowed || occupied.iter().any(|&(id,x,y)| id!=c.id && (x,y)==c.to) { continue; }
        if let Some(p)=occupied.iter_mut().find(|p|p.0==c.id) { p.1=c.to.0; p.2=c.to.1; }
        moves.push(c);
    }
    moves
}
fn traversable(r:&Run,a:&Actor,x:f64,y:f64)->bool {
    let Some(f)=r.floor() else{return false};
    if x<0. || y<0. || x>=f.width() as f64 || y>=f.tiles.len() as f64/f.width() as f64 {return false}
    let options=crate::portals::step_options(r,a,x,y);
    // A portal redirection is not the requested translation. Do not invent a
    // destination or draw RNG to reroll a blocked carried actor.
    if options.len()!=1 || options[0]!=(x,y) {return false}
    let cell=y as usize*f.width()+x as usize;
    engine::actor_passable(r,a,x,y) && !engine::bulwark_blocks(r,a,x,y)
        && crate::ichor::movement_allowed(r,a,x,y)
        && (f.tiles[cell]!="chasm" || engine::flying_in(r,a))
        && !f.doors.as_ref().and_then(|doors|doors.get(&cell.to_string())).is_some_and(|door|!door.open)
        && movement::diagonal_step_allowed(&f.tiles,f.width(),Point{x:a.x as i32,y:a.y as i32},Point{x:x as i32,y:y as i32},a.active.iter().any(|id|id=="etill-phase-through"))
}
/// Call once after the player commits a move, using the pre-move snapshot.
/// No mutation for failed movement, revision <50, depth transitions, or teleport.
/// Planning O(K log K + K*A), plus K existing forced-movement calls and their
/// terrain/travel effects (not constant time). Every actual move rechecks current
/// collision state, so an earlier terrain failure cannot produce an overlap.
pub fn finish(r:&mut Run,before:Option<DragSnapshot>)->Result<(),EngineError> {
    let Some(before)=before else{return Ok(())};
    if r.depth!=before.depth || !active(r) {return Ok(())}
    let (dx,dy)=(r.player.x-before.x,r.player.y-before.y);
    if (dx==0. && dy==0.) || dx.abs()>1. || dy.abs()>1. {return Ok(())}
    let candidates=before.actors.iter().filter_map(|&(id,x,y)| {
        let a=engine::actor(r,id)?;
        if a.hp<=0. || a.dead.value_or(false) || (a.x,a.y)!=(x,y)
            || !crate::dreamwalking::same_plane(r,&r.player,a) {return None}
        Some(Translation{id,from:(x,y),to:(x+dx,y+dy),allowed:traversable(r,a,x+dx,y+dy)})
    }).collect();
    let occupied=std::iter::once(&r.player).chain(r.actors_at(r.depth)).filter(|a|
        !a.dead.value_or(false) && crate::dreamwalking::same_plane(r,&r.player,a)
    ).map(|a|(a.id,a.x,a.y)).collect();
    for step in plan(candidates,occupied,dx,dy) {
        if r.depth!=before.depth {break}
        let Some(a)=engine::actor(r,step.id) else{continue};
        if a.hp<=0. || a.dead.value_or(false) || (a.x,a.y)!=step.from || !traversable(r,a,step.to.0,step.to.1) {continue}
        let observed_before=a.clone();
        let old=engine::actor_mut_trusted(r,step.id).unwrap().conditions.insert("forcedMove".into(),1.);
        let result=engine::move_actor(r,step.id,dx,dy,true);
        if let Some(a)=engine::actor_mut_trusted(r,step.id) {
            a.conditions.shift_remove("forcedMove");
            if let Some(old)=old {a.conditions.insert("forcedMove".into(),old);}
        }
        result?;
        if r.depth==before.depth {if let Some(after)=engine::actor(r,step.id) {if (after.x,after.y)!=step.from {
            crate::events::cue(r,after,"P03","bracket",crate::events::Stage::Travel,&[after.into()]);
            crate::events::link(r,&observed_before,after,"P03","stitch-grapple",crate::events::Stage::Travel);
        }}}
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn c(id:f64,x:f64,y:f64,allowed:bool)->Translation {Translation{id,from:(x,y),to:(x+1.,y),allowed}}
    #[test] fn chains_move_front_first_independent_of_input_order() {
        let p=plan(vec![c(2.,0.,0.,true),c(1.,1.,0.,true)],vec![(2.,0.,0.),(1.,1.,0.)],1.,0.);
        assert_eq!(p.iter().map(|c|c.id).collect::<Vec<_>>(),vec![1.,2.]);
    }
    #[test] fn blocked_front_keeps_entire_chain_and_other_lane_moves() {
        let p=plan(vec![c(2.,0.,0.,true),c(1.,1.,0.,false),c(3.,0.,1.,true)],vec![(2.,0.,0.),(1.,1.,0.),(3.,0.,1.)],1.,0.);
        assert_eq!(p.iter().map(|c|c.id).collect::<Vec<_>>(),vec![3.]);
    }
    #[test] fn outside_occupant_blocks_chain() {
        assert!(plan(vec![c(1.,1.,0.,true),c(2.,0.,0.,true)],vec![(1.,1.,0.),(2.,0.,0.),(3.,2.,0.)],1.,0.).is_empty());
    }
    #[test] fn lock_filters_dead_allied_neutral_distant_and_old_revision() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=50.0.into();r.player.active.push(ID.into());
        let mut a=r.player.clone();a.id=100.;a.x+=1.;a.state="hunting".into();
        assert!(locked(&r,&a));
        a.state="ally".into();assert!(!locked(&r,&a));
        a.state="neutral".into();assert!(!locked(&r,&a));
        a.state="hunting".into();a.hp=0.;assert!(!locked(&r,&a));a.hp=1.;
        a.x+=2.;assert!(!locked(&r,&a));a.x-=2.;
        r.simulation_revision=49.0.into();assert!(!locked(&r,&a));assert!(snapshot(&r).is_none());
    }
    #[test] fn carried_actor_uses_forced_movement_and_restores_condition() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=50.0.into();r.player.active.push(ID.into());
        let mut a=r.actors_at(r.depth)[0].clone();a.id=100.;a.x=r.player.x-1.;a.y=r.player.y;
        a.conditions.insert("forcedMove".into(),7.);
        let depth=r.depth;r.actors_at_mut(depth).clear();r.actors_at_mut(depth).push(a);
        let before=snapshot(&r);let old=(r.player.x,r.player.y);r.player.x+=1.;
        finish(&mut r,before).unwrap();
        let a=engine::actor(&r,100.).unwrap();assert_eq!((a.x,a.y),old);
        assert_eq!(engine::condition(a,"forcedMove"),7.);
    }
    #[test] fn diagonal_wall_prevents_squeezing_despite_forced_flag() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=50.0.into();r.player.active.push(ID.into());
        let mut a=r.actors_at(r.depth)[0].clone();a.id=100.;a.x=r.player.x-1.;a.y=r.player.y;
        let original=(a.x,a.y);let depth=r.depth;
        r.actors_at_mut(depth).clear();r.actors_at_mut(depth).push(a);
        let width=r.floor().unwrap().width();let at=(original.1 as usize+1)*width+original.0 as usize;
        r.floor_mut().unwrap().tiles[at]="wall".into();
        let before=snapshot(&r);r.player.x+=1.;r.player.y+=1.;finish(&mut r,before).unwrap();
        let a=engine::actor(&r,100.).unwrap();assert_eq!((a.x,a.y),original);
    }

}

#[cfg(test)] mod animation_grapple_tether_tests {
 use super::*;
 #[test] fn tether_records_only_actual_admitted_drag(){
  for blocked in [false,true] {
   let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();r.player.x=12.;r.player.y=12.;r.player.conditions.clear();r.player.abilities=vec![ID.into()];r.player.active=r.player.abilities.clone();r.inventory.clear();
   let mut enemy=r.player.clone();enemy.id=900.;enemy.y=11.;enemy.state="hunting".into();enemy.active.clear();enemy.abilities.clear();r.actors_at_mut(r.depth).clear();r.actors_at_mut(r.depth).push(enemy);
   let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.portals=crate::Field::Missing;f.non_euclidean=crate::Field::Missing;if blocked {let w=f.width();f.tiles[11*w+13]="wall".into();}
   let before=snapshot(&r);r.player.x+=1.;crate::events::begin();finish(&mut r,before).unwrap();let events=crate::events::finish();let tether=events.iter().find(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="P03"&&a.style=="stitch-grapple"));
   assert_eq!(tether.is_some(),!blocked);if let Some(e)=tether {assert_eq!(e.from,Some(crate::events::EventPoint{x:12.,y:11.}));assert_eq!(e.to,crate::events::EventPoint{x:13.,y:11.});}
  }
 }
}
