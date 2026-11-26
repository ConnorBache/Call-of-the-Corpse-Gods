//! Stable shared-owner endpoints. Body-local conditions are never merged into the owner.
use crate::{Actor, Field, Run};
pub const ID: &str = "etill-half-here";
pub const COLOR: &str = "#c4b1e8";
pub const MARKER: &str = "h";
const LOCK_UNTIL: &str = "halfHereLockedUntil";
const MINIMUM_TURNS: f64 = 5.;

pub fn lock_enabled(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 52. && matches!(r.half_here_lock_from, Field::Value(_))
}

pub(crate) fn validate_lock_policy(r: &Run) -> Result<(), String> {
    if !r.half_here_lock_from.is_missing()
        && (r.simulation_revision.value_or(29.) < 52.
            || !matches!(r.half_here_lock_from, Field::Value(n) if n <= r.replay.len()))
    {
        return Err("Invalid Half Here lock command boundary".into());
    }
    Ok(())
}

/// Live saves adopt the lock for new placements; do not re-time an existing clone.
pub fn admit_current(r: &mut Run) -> Result<(), String> {
    validate_lock_policy(r)?;
    if r.simulation_revision.value_or(29.) >= 52. && r.half_here_lock_from.is_missing() {
        r.half_here_lock_from = r.replay.len().into();
    }
    Ok(())
}

pub(crate) fn apply_replay_policy(r: &mut Run, from: &Field<usize>, cursor: usize) {
    if from.as_ref() == Some(&cursor) {
        r.half_here_lock_from = cursor.into();
    } else if cursor == 0 {
        r.half_here_lock_from = Field::Missing;
    }
}

/// Only elapsed world turns count; free toggles and bonus actions do not.
pub fn turns_until_dismissal(r: &Run) -> u8 {
    if !lock_enabled(r) { return 0; }
    r.player.conditions.get(LOCK_UNTIL)
        .map_or(0, |until| (until - r.tick).clamp(0., MINIMUM_TURNS).ceil() as u8)
}

pub(crate) fn position_locked(r: &Run, id: f64) -> bool {
    r.player.conditions.get("halfHereId") == Some(&id) && turns_until_dismissal(r) > 0
}

pub fn is_clone(a: &Actor) -> bool {
    a.conditions.get("halfHereClone") == Some(&1.0) || crate::dreamwalking::is_body(a)
}
pub fn normalize_presentation(a: &mut Actor) -> bool {
    if is_clone(a) && a.color.to_lowercase() == "#88d9cd" {
        a.color = COLOR.into();
        true
    } else {
        false
    }
}
pub fn owner_id(r: &Run, a: &Actor) -> f64 {
    if let Some(owner)=crate::dreamwalking::owner(r,a) {return owner;}
    if is_clone(a) && a.conditions.get("halfHereOwner") == Some(&r.player.id) {
        r.player.id
    } else {
        a.id
    }
}
pub fn clone_id(r: &Run) -> Option<f64> {
    let p = &r.player;
    if !p.abilities.iter().any(|s| s == ID) || !p.active.iter().any(|s| s == ID) {
        return None;
    }
    r.actors_at(r.depth)
        .iter()
        .find(|a| {
            !a.dead.value_or(false)
                && is_clone(a)
                && p.conditions.get("halfHereId") == Some(&a.id)
                && owner_id(r, a) == p.id
        })
        .map(|a| a.id)
}
pub fn origins(r: &Run) -> Vec<&Actor> {
    let mut bodies = vec![&r.player];
    if let Some(id) = clone_id(r) {
        bodies.push(r.actors_at(r.depth).iter().find(|a| a.id == id).unwrap())
    }
    bodies
}
pub fn within_range(r: &Run, x: f64, y: f64, range: f64) -> bool {
    origins(r)
        .iter()
        .any(|a| crate::portals::point_distance(r, a, x, y) <= range)
}
/// O(total actors + floors), with COW detach only for actor blocks containing owned endpoints.
pub fn dismiss(r: &mut Run) {
    let owner = r.player.id;
    for a in r.actors_at(r.depth).iter().filter(|a|is_clone(a)&&!crate::dreamwalking::is_body(a)&&a.conditions.get("halfHereOwner")==Some(&owner)) {crate::animation_player::local(r,a,"P29","dissolve",crate::events::Stage::Special);}
    let mut changed = Vec::new();
    for (d, actors) in &mut r.actors {
        if actors
            .iter()
            .any(|a| is_clone(a) && !crate::dreamwalking::is_body(a) && a.conditions.get("halfHereOwner") == Some(&owner))
        {
            crate::mutation_journal::make_mut(actors)
                .retain(|a| crate::dreamwalking::is_body(a) || !is_clone(a) || a.conditions.get("halfHereOwner") != Some(&owner));
            changed.push(d.clone());
        }
    }
    for f in &mut r.floors {
        if changed.iter().any(|d| d == &f.depth.to_string()) {
            crate::mutation_journal::make_mut(f).dirty = true
        }
    }
    r.player.conditions.shift_remove("halfHereId");
    r.player.conditions.shift_remove(LOCK_UNTIL);
    r.player.active.retain(|s| s != ID);
    crate::revision50::settle_duel_targets(r);
}
/// O(current actors + owner projection size); local status fields retain their endpoint identity.
pub fn sync(r: &mut Run) {
    crate::dreamwalking::sync(r);
    let id = clone_id(r);
    if id.is_none() || r.player.dead.value_or(false) || r.status != "playing" {
        if r.player.conditions.contains_key("halfHereId") || r.player.active.iter().any(|s| s == ID)
        {
            dismiss(r)
        }
        return;
    }
    let p = r.player.clone();
    let a = r
        .actors_slice_mut(r.depth)
        .iter_mut()
        .find(|a| Some(a.id) == id)
        .unwrap();
    a.abilities = p
        .abilities
        .iter()
        .filter(|s| s.as_str() != ID)
        .cloned()
        .collect();
    a.active = p
        .active
        .iter()
        .filter(|s| s.as_str() != ID)
        .cloned()
        .collect();
    a.ability_order = p
        .ability_order
        .as_ref()
        .unwrap_or(&p.abilities)
        .iter()
        .filter(|s| s.as_str() != ID)
        .cloned()
        .collect::<Vec<_>>()
        .into();
    a.equipment_abilities = p
        .equipment_abilities
        .as_ref()
        .into_iter()
        .flatten()
        .filter(|s| s.as_str() != ID)
        .cloned()
        .collect::<Vec<_>>()
        .into();
    a.stats = p.stats;
    a.base_max_hp = p.base_max_hp;
    let harmed=a.hp>p.hp;
    a.hp = p.hp;
    a.max_hp = p.max_hp;
    a.energy = p.energy;
    a.blood = p.blood;
    a.damage_bank = p.damage_bank;
    a.nightmares = p.nightmares;
    a.reactions = p.reactions;
    a.adaptive_dr = Field::Missing;
    a.adaptive_hearts = match p.adaptive_hearts {
        Field::Value(v) => Field::Value(v),
        _ => Field::Missing,
    };
    if harmed {
        let a=a.clone();
        crate::animation_player::local(r,&a,"P29","ripple",crate::events::Stage::Health);
        crate::animation_player::local(r,&r.player,"P29","ripple",crate::events::Stage::Health);
    }
}
/// Destination validation and activation cost/events belong to the caller.
/// Birth admission is O(d) clean, O(A*d + F + H) dirty; it precedes dismissal.
pub fn create(r: &mut Run, x: f64, y: f64) -> Result<f64, String> {
    if turns_until_dismissal(r) > 0 {
        return Err("Half Here cannot be moved yet.".into());
    }
    crate::creature_index::check_next_creature_id(r)?;
    dismiss(r);
    let p = &r.player;
    let id = r.next_id;
    r.next_id += 1.0;
    let mut a = Actor {
        id,
        x,
        y,
        name: "Half Here".into(),
        glyph: "@".into(),
        color: COLOR.into(),
        hp: p.hp,
        max_hp: p.max_hp,
        base_max_hp: p.base_max_hp.clone(),
        energy: p.energy,
        stats: p.stats.clone(),
        state: "ally".into(),
        perfect: false.into(),
        heartless: true.into(),
        ..Default::default()
    };
    if crate::dreamwalking::enabled(r) {a.conditions.insert("phantasmiiPlane".into(), if crate::dreamwalking::active(r) {1.} else {0.});}
    a.conditions.insert("halfHereClone".into(), 1.0);
    a.conditions.insert("halfHereOwner".into(), p.id);
    crate::animation_shapes::line(r,&r.player,(&a).into(),"P29","line-grow","#40ffff");
    r.actors_at_mut(r.depth).push(a);
    r.player.conditions.insert("halfHereId".into(), id);
    if lock_enabled(r) {
        r.player.conditions.insert(LOCK_UNTIL.into(), r.tick + MINIMUM_TURNS);
    }
    r.player.active.push(ID.into());
    sync(r);
    Ok(id)
}

#[cfg(test)]
mod identity_birth_tests {
    use super::*;
    /// O(S) serialized-state comparison for a fixed-size malformed-cursor fixture.
    #[test]
    fn collision_rejects_before_dismissing_existing_endpoint() {
        let mut r = Run { depth: 1., next_id: 0., ..Default::default() };
        r.player.active.push(ID.into());
        r.player.conditions.insert("halfHereId".into(), 1.);
        let mut clone = Actor { id: 1., ..Default::default() };
        clone.conditions.insert("halfHereClone".into(), 1.);
        clone.conditions.insert("halfHereOwner".into(), 0.);
        r.actors_at_mut(1.).push(clone);
        let before = serde_json::to_value(&r).unwrap();
        assert!(create(&mut r, 2., 2.).is_err());
        assert_eq!(serde_json::to_value(&r).unwrap(), before);
    }
}
