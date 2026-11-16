//! Frozen v48 monster-lifecycle.ts clocks and Ursa's linked death alarm.
use crate::{
    Actor, Field, Run,
    engine::{actor, actor_mut_trusted, condition},
};
fn is_kind(a: &Actor, kind: &str, name: &str) -> bool {
    a.creature_kind.as_ref().is_some_and(|v| v == kind)
        || (a.creature_kind.as_ref().is_none_or(|v| v.is_empty())
            && a.name.strip_prefix("Mercurial ").unwrap_or(&a.name) == name)
}
fn trait_(a: &Actor, id: &str) -> bool {
    a.abilities.iter().any(|v| v == id)
}
pub fn has_banshee_shriek(a: &Actor) -> bool {
    is_kind(a, "banshee", "Banshee") && trait_(a, "enemy:beast:banshee")
}
pub fn has_ursa_death_cry(a: &Actor) -> bool {
    is_kind(a, "ursa-minor", "Ursa Minor") && trait_(a, "enemy:beast:ursa-minor")
}
pub fn is_ursa_companion(a: &Actor) -> bool {
    is_kind(a, "ursa-major", "Ursa Major")
}
pub fn regen_multiplier(a: &Actor) -> f64 {
    if trait_(a, "enemy:beast:regeneration") {
        10.
    } else {
        1.
    }
}
pub fn floor_entered(r: &mut Run) {
    let tick = r.tick;
    if let Some(f) = r.floor_mut() {
        if f.monster_entry_tick.as_ref().is_none() {
            f.monster_entry_tick = tick.into();
        }
    }
}
fn started_at(r: &Run, a: &Actor) -> Option<f64> {
    a.conditions
        .get("monsterBansheeRevivedAt")
        .copied()
        .or_else(|| {
            r.floor()
                .and_then(|f| f.monster_entry_tick.as_ref().copied())
        })
}
pub fn banshee_turns_until(r: &Run, a: &Actor) -> Option<f64> {
    if a.dead.value_or(false) || !has_banshee_shriek(a) {
        return None;
    }
    Some(30. - ((r.tick - started_at(r, a)?).max(0.) % 30.))
}
pub fn banshee_warning(r: &Run, a: &Actor) -> Option<&'static str> {
    let remaining = banshee_turns_until(r, a)?;
    if remaining <= 5. {
        Some("It's about to shriek!")
    } else if remaining <= 10. {
        Some("It looks like the banshee will shriek soon...")
    } else {
        None
    }
}
pub fn audible_banshee(r: &Run, a: &Actor) -> bool {
    is_kind(a, "banshee", "Banshee")
        && r.tick < condition(a, "monsterAudibleUntil")
        && ["monsterAudibleX", "monsterAudibleY"].iter().all(|key| {
            a.conditions
                .get(*key)
                .is_some_and(|v| v.is_finite() && v.fract() == 0.)
        })
}
pub fn world_turn(r: &mut Run) -> Result<(), String> {
    let ids: Vec<_> = r.actors_at(r.depth).iter().map(|a| a.id).collect();
    for id in ids {
        let tick = r.tick;
        let Some(a) = actor_mut_trusted(r, id) else { continue };
        if condition(a, "monsterAudibleUntil") <= tick {
            for key in ["monsterAudibleUntil", "monsterAudibleX", "monsterAudibleY"] {
                a.conditions.shift_remove(key);
            }
        }
        let a = actor(r, id).unwrap();
        if !has_banshee_shriek(a) {
            continue;
        }
        let Some(started) = started_at(r, a) else {
            continue;
        };
        if tick <= started
            || (tick - started) % 30. != 0.
            || a.conditions.get("monsterBansheeLastShriek") == Some(&tick)
            || a.dead.value_or(false)
            || a.hp <= 0.
            || r.player.dead.value_or(false)
            || r.player.hp <= 0.
            || r.status != "playing"
        {
            continue;
        }
        let a = actor_mut_trusted(r, id).unwrap();
        for (key, value) in [
            ("monsterBansheeLastShriek", tick),
            ("monsterAudibleUntil", tick + 1.),
            ("monsterAudibleX", a.x),
            ("monsterAudibleY", a.y),
        ] {
            a.conditions.insert(key.into(), value);
        }
        crate::engine::log(
            r,
            "A banshee's shriek pierces through the dungeon!",
            "enemy",
        );
        let a = actor(r, id).unwrap();
        crate::events::audible_banshee(r,a);
        if crate::vision::line_of_sight(r, a, &r.player) {
            crate::engine::damage_environment(r, r.player.id, crate::balance52::local_amount(r, a, "monsterSpecialDamage", 100.), Some(id), "banshee", true)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
pub fn ursa_death_cry(r: &mut Run, id: f64, origin: crate::movement::Point) {
    let Some(a) = actor(r, id) else { return };
    if !a.dead.value_or(false) || !has_ursa_death_cry(a) {
        return;
    }
    crate::animation_enemy::mark(r, a, "E19", "sound-pale", crate::events::Stage::Special);
    let partner = a.conditions.get("monsterUrsaPartnerId").copied();
    let major = r
        .actors_at(r.depth)
        .iter()
        .find(|a| {
            Some(a.id) == partner && !a.dead.value_or(false) && a.hp > 0. && is_ursa_companion(a)
        })
        .map(|a| a.id);
    if !r.player.dead.value_or(false) && r.player.x >= 0. {
        crate::engine::log(r, "Ursa Minor lets out a terrible scream!", "enemy");
    }
    if let Some(id) = major {
        let a = actor_mut_trusted(r, id).unwrap();
        crate::ai::wake(a, true);
        a.state = "hunting".into();
        a.last_seen = Field::Value(crate::ActorLastSeen {
            x: origin.x as f64,
            y: origin.y as f64,
        });
        a.conditions.shift_remove("targetId");
        crate::travel::clear_floor_chase(a);
        let a=a.clone(); crate::animation_enemy::mark(r, &a, "E20", "eye", crate::events::Stage::Special);
    }
}
/// Ordinary enemy branch of combat.syncHealth; linked and copied player bodies
/// remain explicit scheduler gates, rather than being treated as enemies.
pub fn sync_enemy_health(r: &mut Run, id: f64) -> Result<(), String> {
    let a = actor(r, id).ok_or("Missing health-sync actor")?.clone();
    if id == r.player.id || condition(&a, "miniMe") == 1. || condition(&a, "halfHereClone") == 1. {
        return Err("Unsupported copied/shared-body enemy health sync".into());
    }
    let stats = crate::combat::effective_stats(r, &a);
    actor_mut_trusted(r, id).unwrap().base_max_hp = Field::Missing;
    let max = 100. + 6. * (stats.defense + 0.5).floor();
    if max == a.max_hp {
        return Ok(());
    }
    let gain = max - a.max_hp;
    if !a.dead.value_or(false) && a.hp > 0. {
        let hp = if gain > 0. {
            a.hp + gain
        } else {
            a.hp.min(max)
        };
        crate::resources::record_loss(r, &a, "hp", a.hp.min(a.hp - hp));
        actor_mut_trusted(r, id).unwrap().hp = hp;
    }
    actor_mut_trusted(r, id).unwrap().max_hp = max;
    if r.simulation_revision.value_or(29.) >= 52. && actor(r, id).unwrap().hp <= 0. {
        crate::engine::death_if_nonpositive(r, id).map_err(|e| e.to_string())?;
    }
    Ok(())
}
/// R52 boundary repair for unresolved nonpositive health, before actions/regen.
/// O(A) scan and O(Z) IDs, plus existing death/rescue/corpse hooks for Z candidates.
/// Fractional positive HP is alive; never use displayed HP as the death threshold.
pub(crate) fn settle_nonpositive_health(r: &mut Run) -> Result<(), crate::engine::EngineError> {
    if r.simulation_revision.value_or(29.) < 52. { return Ok(()); }
    let ids: Vec<_> = std::iter::once(&r.player).chain(r.actors_at(r.depth))
        .filter(|a| !a.dead.value_or(false) && a.hp <= 0.)
        .map(|a| a.id).collect();
    for id in ids {
        // Earlier death hooks can remove or rescue another endpoint.
        if actor(r, id).is_some_and(|a| !a.dead.value_or(false) && a.hp <= 0.) {
            crate::engine::death_if_nonpositive(r, id)?;
        }
    }
    Ok(())
}
