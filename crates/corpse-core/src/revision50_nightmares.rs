//! R50 independent, serialized hallucination actors, not shared-health clones.
use crate::{Actor, Run};

pub const ID: &str = "ursa-waking-nightmares";
pub const MARKER: &str = "wakingNightmare";
pub const OWNER: &str = "wakingNightmareOwner";

pub fn is_decoy(a: &Actor) -> bool { a.conditions.get(MARKER) == Some(&1.0) }
pub fn enabled(r: &Run) -> bool { r.simulation_revision.value_or(29.) >= 50. }

/// Pure admission: O(A + visibility + registry admission); does not spend an ID,
/// nightmare, energy, turn, or random draw. Visibility includes ordinary portals.
pub fn preflight(r: &Run, x: f64, y: f64) -> Result<(), String> {
    preflight_for(r,r.player.id,x,y)
}
/// Explicit independent owner and target; caller chooses the square, not this API.
pub fn preflight_for(r: &Run, owner_id: f64, x: f64, y: f64) -> Result<(), String> {
    if !enabled(r) { return Err("Waking Nightmares requires revision 50.".into()); }
    let p=crate::engine::actor(r,owner_id).ok_or("Nightmare owner is missing.")?;
    if crate::half_here::is_clone(p) || is_decoy(p) {return Err("Nightmares require an independent owner.".into());}
    if !p.abilities.iter().any(|s| s == ID) {
        return Err("Waking Nightmares is not owned.".into());
    }
    if p.dead.value_or(false) || p.hp <= 0. {
        return Err("A living owner is required.".into());
    }
    let charges = p.nightmares.value_or(0.);
    if !charges.is_finite() || charges < 1. {
        return Err("You need a nightmare.".into());
    }
    let f = crate::dreamwalking::floor_for(r,p).ok_or("Active floor missing.")?;
    if !x.is_finite() || !y.is_finite() || x.fract() != 0. || y.fract() != 0.
        || x < 0. || y < 0. || x >= f.width() as f64 || y >= f.height() as f64
    { return Err("Select a visible unoccupied square.".into()); }
    if r.simulation_revision.value_or(29.)>=51. && p.id!=r.player.id && p.state!="ally"
        && (crate::revision51_stairs::stair_cell(f,y as usize*f.width()+x as usize) || (r.simulation_revision.value_or(29.)>=52. && crate::borge::reserved_cell(f,y as usize*f.width()+x as usize))) {
        return Err("Enemy hallucinations cannot appear on stairs.".into());
    }
    if std::iter::once(&r.player).chain(r.actors_at(r.depth)).any(|a| {
        !a.dead.value_or(false) && a.hp > 0.
            && crate::dreamwalking::same_plane(r, p, a)
            && crate::portals::same_position(r, a, x, y)
    }) || !crate::vision::visible_from(r, p, x as usize, y as usize) {
        return Err("Select a visible unoccupied square.".into());
    }
    crate::creature_index::check_next_creature_id(r)
}

/// Commit after rechecking admission. O(A + visibility + registry admission),
/// plus O(A) if the actor block is shared with a temporal snapshot.
pub fn create(r: &mut Run, x: f64, y: f64) -> Result<f64, String> {
    create_for(r,r.player.id,x,y)
}
pub fn create_for(r: &mut Run, owner_id: f64, x: f64, y: f64) -> Result<f64, String> {
    preflight_for(r, owner_id, x, y)?;
    let p = crate::engine::actor(r,owner_id).unwrap();
    let id = r.next_id;
    // Deliberately do not clone the player's powers, conditions, health pool,
    // equipment grants, species, or resurrection state.
    let mut decoy = Actor {
        id, x, y, name: format!("Hallucination of {}", p.name),
        glyph: p.glyph.clone(), color: p.color.clone(), hp: 1., max_hp: 1.,
        base_max_hp: 1.0.into(), state: if owner_id==r.player.id {"ally".into()} else {p.state.clone()},
        heartless: true.into(), perfect: false.into(), ..Default::default()
    };
    decoy.conditions.insert(MARKER.into(), 1.);
    decoy.conditions.insert(OWNER.into(), p.id);
    if crate::dreamwalking::enabled(r) {
        decoy.conditions.insert("phantasmiiPlane".into(), if crate::dreamwalking::dream_actor(r,p) { 1. } else { 0. });
    }
    let charges=p.nightmares.value_or(0.0);
    crate::engine::actor_mut_trusted(r,owner_id).unwrap().nightmares=(charges-1.0).into();
    r.next_id += 1.;
    crate::animation_player::local(r,&decoy,"P31","summon",crate::events::Stage::Special);
    r.actors_at_mut(r.depth).push(decoy);
    if let Some(f) = r.floor_mut() { f.dirty = true; }
    Ok(id)
}

/// Call once during a normal world turn, never during bonus/haste phases.
/// Uses existing serialized absolute recharge deadline; O(owned abilities).
pub fn tick(r: &mut Run) {
    tick_actor(r, r.player.id);
}
/// The normal-turn actor scheduler calls this once per independent owner.
/// Half Here/body projections do not earn a second charge for their owner.
/// O(actor lookup + B), O(1) scratch, plus actor-block COW on a recharge write.
/// Copied absolute deadlines remain independent serialized clocks.
pub fn tick_actor(r: &mut Run, actor_id: f64) {
    if !enabled(r) { return; }
    let Some(a)=crate::engine::actor(r,actor_id) else {return};
    if crate::half_here::is_clone(a) || is_decoy(a) || a.dead.value_or(false)
        || a.hp<=0.0 || !a.abilities.iter().any(|s|s==ID) {return;}
    let now=r.tick;
    let a=crate::engine::actor_mut_trusted(r,actor_id).unwrap();
    let at=*a.conditions.entry("nightmareAt".into()).or_insert(now+99.0);
    if now>=at {
        a.nightmares=(a.nightmares.value_or(0.0)+1.0).into();
        a.conditions.insert("nightmareAt".into(),now+100.0);
        if actor_id==r.player.id {crate::events::inventory(r,"P31","charge-spark",None);}
    }
}

/// A landed strike dissolves the actor even if the damage would be zero.
/// Caller must return before ordinary damage/death/reward/hit-effect handling.
/// O(A) stable removal; no corpse, resurrection, experience, or item drops.
pub fn strike(r: &mut Run, id: f64) -> bool {
    if !enabled(r) || !crate::engine::actor(r, id).is_some_and(is_decoy) { return false; }
    crate::animation_player::local(r,crate::engine::actor(r,id).unwrap(),"P31","dissolve",crate::events::Stage::Special);
    r.actors_at_mut(r.depth).retain(|a| a.id != id);
    crate::revision50::end_duels_with_target(r,id);
    if let Some(f) = r.floor_mut() { f.dirty = true; }
    true
}

#[cfg(test)]
mod independent_clock_tests {
    use super::*;
    fn room()->Run {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=50.0.into();r.inventory.clear();r.player.active.clear();
        r.player.conditions.clear();r.player.abilities=vec![ID.into()];
        r.player.nightmares=2.0.into();r.player.conditions.insert("nightmareAt".into(),100.0);r
    }
    fn independent_owner(r:&Run,x:f64,y:f64)->Actor {
        let mut a=r.player.clone();a.id=r.next_id;a.x=x;a.y=y;a.state="ally".into();a.conditions.insert("phantasmiiPlane".into(),0.0);a
    }
    fn insert_owner(r:&mut Run,a:Actor) {r.next_id+=1.0;r.actors_at_mut(r.depth).push(a);}
    #[test]
    fn independent_recharge_clock_and_charges_are_independent_and_persisted() {
        let mut r=room();let a=independent_owner(&r,7.0,7.0);let id=a.id;
        insert_owner(&mut r,a);
        // Spending one owner's charges must not consume the copied charges.
        r.player.nightmares=0.0.into();r.tick=99.0;
        tick_actor(&mut r,id);assert_eq!(crate::engine::actor(&r,id).unwrap().nightmares.value_or(0.0),2.0);
        r.tick=100.0;tick_actor(&mut r,id);tick_actor(&mut r,id);
        assert_eq!(crate::engine::actor(&r,id).unwrap().nightmares.value_or(0.0),3.0);
        assert_eq!(r.player.nightmares.value_or(0.0),0.0);
        tick(&mut r);assert_eq!(r.player.nightmares.value_or(0.0),1.0);
        let saved=serde_json::to_string(&r).unwrap();let mut restored:Run=serde_json::from_str(&saved).unwrap();
        restored.tick=200.0;tick_actor(&mut restored,id);
        assert_eq!(crate::engine::actor(&restored,id).unwrap().nightmares.value_or(0.0),4.0);
        assert_eq!(restored.player.nightmares.value_or(0.0),1.0);
    }
    #[test]
    fn shared_bodies_decoys_dead_and_legacy_actors_do_not_recharge() {
        for marker in ["halfHereClone","phantasmiiBody",MARKER,"dead","legacy"] {
            let mut r=room();let mut a=independent_owner(&r,7.0,7.0);let id=a.id;
            if marker=="dead" {a.dead=true.into();} else if marker=="legacy" {r.simulation_revision=49.0.into();}
            else {a.conditions.insert(marker.into(),1.0);}
            r.actors_at_mut(r.depth).push(a);r.tick=100.0;
            let before=serde_json::to_vec(&r).unwrap();tick_actor(&mut r,id);
            assert_eq!(serde_json::to_vec(&r).unwrap(),before,"{marker}");
        }
    }
    #[test]
    fn explicit_owner_activation_uses_own_visibility_charge_identity_and_plane() {
        let mut r=room();r.actors_at_mut(r.depth).clear();
        let a=independent_owner(&r,7.0,7.0);let owner=a.id;
        insert_owner(&mut r,a);
        r.floor_mut().unwrap().tiles.fill("wall".into());
        r.player.conditions.insert("allSeeingReveal".into(),1.0);
        r.player.conditions.insert("allSeeingDepth".into(),r.depth);
        let before=serde_json::to_vec(&r).unwrap();
        assert!(preflight_for(&r,owner,0.0,0.0).is_err(),"player reveal cannot reveal tiles to a copy");
        assert!(create_for(&mut r,owner,0.0,0.0).is_err());
        assert_eq!(serde_json::to_vec(&r).unwrap(),before);
        let depth=r.depth;
        let a=crate::engine::actor_mut_trusted(&mut r,owner).unwrap();
        a.conditions.insert("allSeeingReveal".into(),1.0);a.conditions.insert("allSeeingDepth".into(),depth);
        let player_charges=r.player.nightmares.clone();let next=r.next_id;
        let decoy=create_for(&mut r,owner,0.0,0.0).unwrap();assert_eq!(decoy,next);
        assert_eq!(r.player.nightmares,player_charges);
        assert_eq!(crate::engine::actor(&r,owner).unwrap().nightmares.value_or(0.0),1.0);
        let d=crate::engine::actor(&r,decoy).unwrap();
        assert_eq!(d.conditions.get(OWNER),Some(&owner));assert_eq!(d.state,"ally");
        assert_eq!(d.conditions.get("phantasmiiPlane"),crate::engine::actor(&r,owner).unwrap().conditions.get("phantasmiiPlane"));
        assert!(d.abilities.is_empty());assert!(d.copied_inventory.is_missing());
    }
}
