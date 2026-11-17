//! Ruin Pulse snapshots endpoint order; every physical body takes its own packet.
use crate::{
    Actor, Run,
    engine::{self, actor, actor_mut_trusted, condition},
};
pub fn ruin_damage(a: &Actor) -> f64 {
    const QUALITIES: [&str; 10] = [
        "infant",
        "child",
        "lesser",
        "unqualified",
        "mature",
        "greater",
        "hulking",
        "ancient",
        "timeless",
        "transcendent",
    ];
    a.zillennia_quality
        .as_ref()
        .and_then(|q| QUALITIES.iter().position(|x| *x == q))
        .map(|i| 28.0 + 8.0 * i as f64)
        .unwrap_or_else(|| 20.0 + a.conditions.get("godbeastDepth").copied().unwrap_or(1.0) * 2.0)
}
/// Revision42 early Zillennia pulse trial. O(1) fixed quality dispatch; no RNG,
/// geometry, cooldown or energy changes. Raw packet is halved here ONCE; existing
/// attack-channel tuning still applies afterward through R51, yielding7/9.
/// Current R52 authors7/9 directly and applies no final attack multiplier.
pub fn ruin_damage_revision(a: &Actor, revision: f64) -> f64 {
    if crate::combat::raw_attack_damage_enabled(revision) {
        return match a.zillennia_quality.as_ref().map(String::as_str) {
            Some("infant") => 7.0,
            Some("child") => 9.0,
            _ => a.zillennia_quality.as_ref().and_then(|q| ["infant", "child", "lesser", "unqualified", "mature", "greater", "hulking", "ancient", "timeless", "transcendent"].iter().position(|x| *x == q))
                .map(|i| 14.0 + 4.0 * i as f64)
                .unwrap_or_else(|| 10.0 + a.conditions.get("godbeastDepth").copied().unwrap_or(1.0)),
        };
    }
    if matches!(revision, 42.0 | 43.0 | 44.0 | 45.0 | 46.0 | 50.0 | 51.0 | 52.0) {
        match a.zillennia_quality.as_ref().map(String::as_str) {
            Some("infant") => return 14.0,
            Some("child") => return 18.0,
            _ => {}
        }
    }
    ruin_damage(a)
}
pub fn special_action(r: &mut Run, id: f64, target: f64) -> Result<bool, String> {
    let a = actor(r, id).ok_or("Ruin caster missing")?.clone();
    let b = actor(r, target).ok_or("Ruin target missing")?.clone();
    if !a.abilities.iter().any(|s| s == "enemy:beast:ruin-pulse")
        || crate::portals::actor_distance(r, &a, &b) > 3.0
        || condition(&a, "beastCooldown") != 0.0
        || !crate::monster_horn::spell_spend(r, id, 20.0)
    {
        return Ok(false);
    }
    // Presentation reuses the exact point-distance and LOS admission rules.
    // O(N*(portal distance+LOS)) on a successful cast, no RNG or geometry writes.
    crate::animation_enemy::wave(r,&a,"E51",false,Some(3.),true,false);
        let wave_action=crate::events::current_action();
    // O(A * (A + LOS + damage)), stable source array snapshot, no target RNG.
    let ids: Vec<f64> = std::iter::once(&r.player)
        .chain(r.actors_at(r.depth))
        .map(|v| v.id)
        .collect();
    for victim in ids {
        let a = if crate::revision50::enabled(r) {a.clone()}
            else {engine::actor_any(r, id).unwrap_or(&a).clone()};
        let pulse = ruin_damage_revision(&a, r.simulation_revision.value_or(29.0));
        if let Some(v) = actor(r, victim).cloned() {
            if victim != id
                && !v.dead.value_or(false)
                && crate::portals::actor_distance(r, &a, &v) <= 3.0
                && crate::vision::line_of_sight(r, &a, &v)
            {
                // Range and LOS have resolved this fresh ability activation impact.
                // This is not a periodic aura/environment callback; a retained
                // player source may earn Reaping, an allied caster may not.
                if crate::dreamwalking::same_plane(r, &a, &v)
                    && crate::revision50_nightmares::strike(r, victim) { continue; }
                let packets=crate::events::damage_count(victim);
                engine::damage_direct_with_source_snapshot(
                    r,
                    victim,
                    pulse,
                    &a,
                    "attack",
                    false,
                )
                .map_err(|e| e.0)?;
                if crate::events::damage_count(victim)>packets {
                    crate::animation_enemy::mark(r,&v,"E51","puncture-violet",crate::events::Stage::Health);
                }
            }
        }
    }
    if let Some(a) = actor_mut_trusted(r, id) {
        a.conditions.insert("beastCooldown".into(), 3.0);
    }
    let a = actor(r, id).unwrap_or(&a).clone();
    if !crate::events::enabled(r) {crate::events::actor_event(r, &a, "burst", "Ruin pulse", None, None);}
    let b = actor(r, target).unwrap_or(&b);
    let b = b.clone();
    crate::monster_horn::report(r, &a, &b, "unleashes a pulse of ruin.");
    crate::events::checkpoint_for_action(r,wave_action);
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn revision42_early_pulse_is_local_and_once_scaled() {
        let mut actor=Actor::default();
        for (quality,old,new) in [("infant",28.,14.),("child",36.,18.),("lesser",44.,44.)] {
            actor.zillennia_quality=quality.to_string().into();
            for rev in [29.,30.,31.,32.,33.,34.,35.,36.,37.,38.,39.,40.,41.] {
                assert_eq!(ruin_damage_revision(&actor,rev),old);
            }
            assert_eq!(ruin_damage_revision(&actor,42.),new);
            assert_eq!(crate::combat::tuned_damage(new,Some("attack")),new/2.);
        }
    }
    #[test]
    fn ruin_matches_frozen_reference() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/ruin-v48.json")).unwrap();
        for c in cases.as_array().unwrap() {
            let mut r: Run = serde_json::from_value(c["initial"].clone()).unwrap();
            crate::events::begin();
            let p = r.player.id;
            let result = if c["operation"] == "world" {
                crate::monster_horn::world_turn(&mut r);
                true
            } else if c["operation"] == "offscreen" {
                crate::offscreen::tick_inactive_floors(&mut r).unwrap();
                true
            } else {
                special_action(&mut r, 10.0, p).unwrap()
            };
            assert_eq!(result, c["result"].as_bool().unwrap(), "{}", c["kind"]);
            let actual = serde_json::to_value(r).unwrap();
            if !crate::storage::canonical_equal(&actual, &c["expected"]) {
                std::fs::write(
                    "/tmp/ruin-native.json",
                    serde_json::to_vec(&actual).unwrap(),
                )
                .unwrap();
                std::fs::write(
                    "/tmp/ruin-source.json",
                    serde_json::to_vec(&c["expected"]).unwrap(),
                )
                .unwrap();
                panic!("{}", c["kind"]);
            }
            assert!(
                crate::storage::canonical_equal(
                    &serde_json::to_value(crate::events::finish()).unwrap(),
                    &c["events"]
                ),
                "{} events",
                c["kind"]
            );
        }
    }
}
