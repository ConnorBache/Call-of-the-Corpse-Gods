//! Frozen v48 monster-statuses.ts. Normal-clock packets and action replacement.
use crate::{
    Actor, Run,
    engine::{actor, actor_mut_trusted, condition},
};
fn source_key(key: &str) -> Option<&'static str> {
    match key {
        "hemophilia" => Some("hemophiliaSourceId"),
        "poisoned" => Some("poisonSourceId"),
        "slowed" => Some("slowSourceId"),
        _ => None,
    }
}
pub fn status_source(r: &Run, a: &Actor, key: &str) -> Option<f64> {
    let id = *a.conditions.get(source_key(key)?)?;
    if id == 0. || id == r.player.id {
        return Some(r.player.id);
    }
    r.actors
        .values()
        .flat_map(|a| a.iter())
        .find(|a| a.id == id)
        .map(|a| a.id)
}
/// Raw ring scaling shared by historical mental/curse admission and the current
/// all-status admission below. Never pass an existing countdown through this.
/// O(I + C) ring query, O(1) scratch; no RNG or retained resistance state.
pub fn willpower_duration(r: &Run, target: &Actor, duration: f64) -> f64 {
    if duration.is_finite() && duration > 0.
        && crate::revision50_rings::has_effect(r, target, crate::revision50_rings::WILLPOWER)
    {
        (duration * crate::revision50_rings::stack_multiplier(r, target, crate::revision50_rings::WILLPOWER, 0.5)).ceil()
    } else {
        duration
    }
}

/// Scale a newly authored finite status duration, never an existing countdown.
/// Current rings affect beneficial and harmful statuses equally. Call before
/// max/refresh composition; cooldowns, state flags and indefinite traits are not
/// durations. Owner-aware ring lookup is O(I + C), O(1) additional memory.
pub fn status_duration(r: &Run, target: &Actor, duration: f64) -> f64 {
    if crate::ring_rules::current(r) {
        willpower_duration(r, target, duration)
    } else {
        duration
    }
}

/// Curse provenance belongs to the spell, not to the condition name: Witch
/// poison/slowness are curses; an ordinary poison or slow application is not.
/// Discord-aware hostility makes this incoming enemy magic, not a self/ally buff.
/// Shared Half Here endpoints never turn a self application into an enemy curse.
pub fn apply_curse(
    r: &mut Run,
    target: f64,
    key: &str,
    duration: f64,
    source: f64,
) -> bool {
    let duration = if crate::ring_rules::current(r) { duration } else { match (actor(r, target), crate::engine::actor_any(r, source)) {
        (Some(a), Some(b)) if crate::half_here::owner_id(r, a) != crate::half_here::owner_id(r, b)
            && (crate::revision50_control_amulets::discord_hostility(r,a,b)
                || (crate::half_here::owner_id(r, a) == r.player.id || a.state == "ally")
                    != (crate::half_here::owner_id(r, b) == r.player.id || b.state == "ally")) =>
            willpower_duration(r, a, duration),
        _ => duration,
    }};
    apply_status(r, target, key, duration, Some(source))
}

pub fn apply_status(
    r: &mut Run,
    target: f64,
    key: &str,
    duration: f64,
    source: Option<f64>,
) -> bool {
    if actor(r,target).is_some_and(crate::revision50_nightmares::is_decoy) { return false; }
    let Some(source_key) = source_key(key) else {
        return false;
    };
    if actor(r,target).is_some_and(|a|crate::revision50_combat::oblivious_blocks(r,a,source.and_then(|id|crate::engine::actor_any(r,id)))) {return false;}
    // Admission scans O(total actors) for attribution, O(1) extra state; existing
    // condition edits/COW costs follow. Only infliction checks the source plane.
    if crate::dreamwalking::enabled(r) {
        let Some(target_actor) = actor(r, target) else { return false };
        let reachable = match source {
            Some(id) => crate::engine::actor_any(r, id)
                .is_some_and(|s| crate::dreamwalking::same_plane(r, s, target_actor)),
            None => crate::dreamwalking::contact_plane(r, target_actor),
        };
        if !reachable { return false; }
    }
    let duration = actor(r,target).map_or(duration, |a| status_duration(r,a,duration));
    let prior=actor(r,target).map_or(0.,|a|condition(a,key));
    let curse_key=source.and_then(|id|crate::engine::actor_any(r,id)).and_then(|a| {
        if a.abilities.iter().any(|s|s=="enemy:beast:bloodmancy") {Some("E04")}
        else if a.abilities.iter().any(|s|s=="enemy:beast:witch") {Some("E08")} else {None}
    });
    let Some(a) = actor_mut_trusted(r, target) else {
        return false;
    };
    if a.dead.value_or(false) || a.hp <= 0. || !duration.is_finite() || duration <= 0. {
        return false;
    }
    if key=="poisoned" && condition(a,"poisoned")<=0. {
        a.conditions.shift_remove(crate::revision50_amulet_effects::POISON_DAMAGE);
    }
    a.conditions
        .insert(key.into(), condition(a, key).max(duration.ceil().max(0.)));
    a.conditions.shift_remove(&format!("{key}Fresh"));
    if let Some(source) = source {
        a.conditions.insert(source_key.into(), source);
    } else {
        a.conditions.shift_remove(source_key);
    }
    let a=a.clone();
    if prior<=0. {if let Some(proposal)=curse_key {crate::animation_enemy::mark(r,&a,proposal,match key {"hemophilia"=>"split-ring-red","slowed"=>"bracket-blue",_=>"droplet-green"},crate::events::Stage::Special);}}
    r.floor_mut().expect("current floor").dirty = true;
    true
}
pub fn hemophilia_episode_damage(a: &Actor) -> f64 {
    (a.max_hp * 0.02).ceil().max(0.)
}
pub fn hemophilia_episode_damage_for_run(r: &Run, a: &Actor) -> f64 {
    if crate::combat::nearest_damage_enabled(r) { crate::combat::nearest_damage(a.max_hp * 0.02) }
    else { hemophilia_episode_damage(a) }
}
pub fn try_hemophilia_episode(r: &mut Run, id: f64) -> Result<bool, String> {
    let Some(a) = actor(r, id) else {
        return Ok(false);
    };
    if r.status != "playing"
        || a.dead.value_or(false)
        || a.hp <= 0.
        || condition(a, "forcedMove") != 0.
        || condition(a, "hemophilia") <= 0.
    {
        return Ok(false);
    }
    if crate::rng::random(r) >= 0.25 {
        return Ok(false);
    }
    let a = actor(r, id).unwrap().clone();
    let source = status_source(r, &a, "hemophilia");
    if id == r.player.id || (r.player.x >= 0. && crate::vision::visible_actor(r, &a)) {
        crate::engine::log(
            r,
            if id == r.player.id {
                "You vomit blood.".into()
            } else {
                format!("{} vomits blood.", crate::actor_presentation::name(r, &a))
            },
            if source == Some(r.player.id) {
                "player"
            } else if source.is_some() {
                "enemy"
            } else {
                "environment"
            },
        );
        crate::events::actor_event(r, &a, "impact", "Vomit blood", None, None);
    }
    let f = r.floor_mut().unwrap();
    let i = a.y as usize * f.width() + a.x as usize;
    if f.tiles[i] == "floor" {
        let n = f.tiles.len();
        f.blood.get_or_insert_with(|| vec![false; n])[i] = true;
        f.dirty = true;
    }
    crate::engine::damage_environment(
        r,
        id,
        hemophilia_episode_damage_for_run(r, &a),
        source,
        "hemophilia",
        true,
    )
    .map_err(|e| e.to_string())?;
    Ok(true)
}
pub fn tick_poisoned(r: &mut Run, id: f64) -> Result<(), String> {
    let Some(a) = actor(r, id) else { return Ok(()) };
    if a.dead.value_or(false) || a.hp <= 0. || condition(a, "poisoned") <= 0. {
        return Ok(());
    }
    let source = status_source(r, a, "poisoned");
    let damage = if crate::revision50::enabled(r) {
        a.conditions.get(crate::revision50_amulet_effects::POISON_DAMAGE).copied().unwrap_or(2.)
    } else {2.};
    crate::engine::damage_environment(r, id, damage, source, "poison", true).map_err(|e| e.to_string())
}
pub fn slow_skips_action(r: &Run, a: &Actor) -> bool {
    condition(a, "slowed") > 0. && r.tick % 2. == 0.
}
#[cfg(test)]
mod parity {
    fn compare(a: &serde_json::Value, b: &serde_json::Value, path: &str) {
        match (a, b) {
            (serde_json::Value::Object(a), serde_json::Value::Object(b)) => {
                assert_eq!(a.len(), b.len(), "keys {path}");
                for (k, v) in a {
                    compare(
                        v,
                        b.get(k).unwrap_or_else(|| panic!("missing {path}.{k}")),
                        &format!("{path}.{k}"),
                    );
                }
            }
            (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
                assert_eq!(a.len(), b.len(), "length {path}");
                for (i, (a, b)) in a.iter().zip(b).enumerate() {
                    compare(a, b, &format!("{path}[{i}]"));
                }
            }
            _ => assert_eq!(a, b, "{path}"),
        }
    }
    #[test]
    fn status_and_lifecycle_reference() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/monster-clocks-v48.json"))
                .unwrap();
        for case in cases.as_array().unwrap() {
            let mut r: crate::Run = serde_json::from_value(case["initial"].clone()).unwrap();
            let mut results = vec![];
            crate::events::begin();
            for o in case["operations"].as_array().unwrap() {
                let id = o["target"].as_f64().unwrap_or(0.);
                match o["kind"].as_str().unwrap() {
                    "apply" => results.push(super::apply_status(
                        &mut r,
                        id,
                        o["key"].as_str().unwrap(),
                        o["duration"].as_f64().unwrap(),
                        o["source"].as_f64(),
                    )),
                    "episode" => results.push(super::try_hemophilia_episode(&mut r, id).unwrap()),
                    "poison" => super::tick_poisoned(&mut r, id).unwrap(),
                    "world" => crate::lifecycle::world_turn(&mut r).unwrap(),
                    "ursa" => crate::lifecycle::ursa_death_cry(
                        &mut r,
                        id,
                        crate::movement::Point {
                            x: o["x"].as_i64().unwrap() as i32,
                            y: o["y"].as_i64().unwrap() as i32,
                        },
                    ),
                    _ => panic!("operation"),
                }
            }
            let expected: crate::Run = serde_json::from_value(case["expected"].clone()).unwrap();
            let actual = serde_json::to_value(&r).unwrap();
            let expected = serde_json::to_value(expected).unwrap();
            compare(&actual, &expected, &case["name"].to_string());
            assert_eq!(serde_json::to_value(results).unwrap(), case["results"]);
            let events: Vec<crate::events::PresentationEvent> =
                serde_json::from_value(case["events"].clone()).unwrap();
            assert_eq!(crate::events::finish(), events, "{} events", case["name"]);
        }
    }
}

#[cfg(test)]
mod plane_tests {
    use super::*;
    use crate::dreamwalking as dream;
    fn room() -> Run {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 34.0.into(); r.actors_at_mut(r.depth).clear();
        r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
        r.player.hp = 100.; r.player.max_hp = 100.;
        let mut source = r.player.clone(); source.id = 20.; source.state = "hunting".into();
        r.actors_at_mut(r.depth).push(source);
        r
    }
    #[test]
    fn r34_status_infliction_rejects_cross_plane_without_mutation() {
        let mut r = room();
        actor_mut_trusted(&mut r, 20.).unwrap().conditions.insert("phantasmiiPlane".into(), 1.);
        for key in ["poisoned", "hemophilia", "slowed"] {
            let before = serde_json::to_value(&r).unwrap();
            assert!(!apply_status(&mut r, 0., key, 5., Some(20.)));
            assert_eq!(serde_json::to_value(&r).unwrap(), before);
        }
        actor_mut_trusted(&mut r, 20.).unwrap().conditions.insert("phantasmiiPlane".into(), 0.);
        for key in ["poisoned", "hemophilia", "slowed"] {
            assert!(apply_status(&mut r, 0., key, 5., Some(20.)));
        }
    }
    #[test]
    fn r34_source_less_status_uses_contact_plane_and_legacy_is_unchanged() {
        let mut r = room();
        r.player.conditions.insert("phantasmiiPlane".into(), 1.);
        assert!(!apply_status(&mut r, 0., "slowed", 5., None));
        assert!(dream::with_plane(&mut r, true, |r| apply_status(r, 0., "slowed", 5., None)));
        r.simulation_revision = 33.0.into();
        assert!(apply_status(&mut r, 0., "poisoned", 5., Some(20.)));
    }
    #[test]
    fn r34_inflicted_dot_keeps_source_credit_after_caster_changes_plane() {
        let mut r = room();
        assert!(apply_status(&mut r, 0., "poisoned", 5., Some(20.)));
        assert!(apply_status(&mut r, 0., "hemophilia", 5., Some(20.)));
        actor_mut_trusted(&mut r, 20.).unwrap().conditions.insert("phantasmiiPlane".into(), 1.);
        tick_poisoned(&mut r, 0.).unwrap();
        assert_eq!(r.player.hp, 98.);
        r.rng = 1.;
        assert!(try_hemophilia_episode(&mut r, 0.).unwrap());
        assert_eq!(r.player.hp, 96.);
        assert_eq!(status_source(&r, &r.player, "poisoned"), Some(20.));
        assert_eq!(status_source(&r, &r.player, "hemophilia"), Some(20.));
    }
}
