//! Authored grips, held movement and River Drake water jets.
use crate::{
    Actor, Run,
    engine::{self, actor, actor_mut_trusted, condition},
};
fn has(a: &Actor, t: &str) -> bool {
    a.abilities.iter().any(|s| s == &format!("enemy:beast:{t}"))
}
fn distance(r: &Run, a: &Actor, b: &Actor) -> f64 {
    crate::portals::actor_distance(r, a, b)
}
pub fn release_invalid(r: &mut Run) {
    let ids: Vec<_> = std::iter::once(r.player.id)
        .chain(r.actors_at(r.depth).iter().map(|a| a.id))
        .collect();
    for id in ids {
        let a = actor(r, id).unwrap();
        if condition(a, "grabbed") <= 0.0 {
            continue;
        }
        let holder = a.conditions.get("grabbedBy").and_then(|id| actor(r, *id));
        if holder.is_none_or(|h| {
            h.dead.value_or(false) || h.hp <= 0.0
                || crate::revision50_combat::oblivious_blocks(r,a,Some(h))
                || !crate::dreamwalking::same_plane(r, a, h)
                || distance(r, a, h) > std::f64::consts::SQRT_2
        }) {
            let a = actor_mut_trusted(r, id).unwrap();
            a.conditions.shift_remove("grabbed");
            a.conditions.shift_remove("grabbedBy");
        }
    }
}
pub fn movement_held(r: &mut Run, id: f64) -> bool {
    let Some(a) = actor(r, id) else { return false };
    if condition(a, "forcedMove") != 0.0 {
        return false;
    }
    if condition(a, "grabbed") > 0.0 {
        release_invalid(r)
    }
    let a = actor(r, id).unwrap().clone();
    if condition(&a, "grabbed") <= 0.0 {
        return false;
    }
    if id == r.player.id || crate::vision::visible_actor(r, &a) {
        engine::log(
            r,
            format!(
                "{} held in place.",
                if id == r.player.id {
                    "You are".into()
                } else {
                    format!("{} is", crate::actor_presentation::name(r, &a))
                }
            ),
            "player",
        );
    }
    true
}
pub fn tick_actor(r: &mut Run, id: f64) {
    let Some(a) = actor(r, id) else { return };
    if condition(a, "grabbed") <= 0.0 {
        return;
    }
    let holder = a.conditions.get("grabbedBy").and_then(|id| {
        r.actors_at(r.depth)
            .iter()
            .find(|b| b.id == *id)
            .or_else(|| (*id == r.player.id).then_some(&r.player))
    });
    let valid = holder.is_some_and(|h| {
        !h.dead.value_or(false) && h.hp > 0.0
            && !crate::revision50_combat::oblivious_blocks(r,a,Some(h))
            && crate::dreamwalking::same_plane(r, a, h)
            && distance(r, a, h) <= std::f64::consts::SQRT_2
    });
    let a = actor_mut_trusted(r, id).unwrap();
    if valid {
        *a.conditions.get_mut("grabbed").unwrap() -= 1.0;
    }
    if !valid || condition(a, "grabbed") <= 0.0 {
        a.conditions.shift_remove("grabbed");
        a.conditions.shift_remove("grabbedBy");
    }
}
pub fn deep_water(r: &mut Run, id: f64, target: f64) -> bool {
    let a = actor(r, id).unwrap().clone();
    let b = actor(r, target).unwrap().clone();
    if !has(&a, "deep-water")
        || a.dead.value_or(false)
        || a.hp <= 0.0
        || b.dead.value_or(false)
        || b.hp <= 0.0
        || distance(r, &a, &b) > std::f64::consts::SQRT_2
        || condition(&b, "grabbed") > 0.0
        || !crate::monster_horn::permitted(r, &a, &b)
        || !crate::vision::line_of_sight(r, &a, &b)
    {
        return false;
    }
    let f = r.floor().unwrap();
    if f.tiles[b.y as usize * f.width() + b.x as usize] != "water" {
        return false;
    }
    let p = crate::projectiles::trace_from_actor(r, &a, b.x, b.y);
    if !crate::projectiles::reaches_actor(r, &p, &b) {
        return false;
    }
    if crate::revision50_nightmares::strike(r, target) { return true; }
    let b = actor_mut_trusted(r, target).unwrap();
    b.conditions.insert("grabbed".into(), 4.0);
    b.conditions.insert("grabbedBy".into(), id);
    let b = b.clone();
    crate::monster_horn::report(r, &a, &b, "grabs its prey in the deep water.");
    crate::events::actor_event(r, &a, "status", "Held", Some((&b).into()), None);
    true
}
pub fn hold_action(r: &mut Run, id: f64, target: f64) -> bool {
    let a = actor(r, id).unwrap().clone();
    let b = actor(r, target).unwrap().clone();
    if crate::revision50_combat::oblivious_blocks(r,&b,Some(&a)) {return false;}
    if !crate::dreamwalking::same_plane(r, &a, &b)
        || (crate::dreamwalking::enabled(r) && crate::dreamwalking::is_body(&a))
        || distance(r, &a, &b) > std::f64::consts::SQRT_2
        || (!has(&a, "hold-only")
            && (!has(&a, "grab") || has(&a, "deep-water") || condition(&b, "grabbed") > 0.0))
    {
        return false;
    }
    if crate::revision50_nightmares::strike(r, target) { return true; }
    let b = actor_mut_trusted(r, target).unwrap();
    b.conditions.insert("grabbed".into(), 3.0);
    b.conditions.insert("grabbedBy".into(), id);
    let b = b.clone();
    if crate::enemy_traits::definition(&a).is_some_and(|d| d.id == "maulchild") {
        let av = crate::vision::visible_actor(r, &a);
        let bv = crate::vision::visible_actor(r, &b);
        if r.player.x >= 0.0 && (id == r.player.id || target == r.player.id || av || bv) {
            let source = if av {
                if crate::actor_presentation::apparent_kind(r, &a).is_some() {
                    crate::actor_presentation::name(r, &a)
                } else if a.name == "Maulchild" {
                    "The Maulchild"
                } else {
                    &a.name
                }
            } else {
                "Something unseen"
            };
            let target = if target == r.player.id {
                "you"
            } else if bv {
                crate::actor_presentation::name(r, &b)
            } else {
                "something unseen"
            };
            engine::log(
                r,
                format!("{source} wraps {target} in a tight hug."),
                if id == r.player.id { "player" } else { "enemy" },
            );
        }
    } else {
        crate::monster_horn::report(
            r,
            &a,
            &b,
            if target == r.player.id {
                "holds you in place."
            } else {
                "holds its prey in place."
            },
        );
    }
    crate::events::actor_event(r, &a, "status", "Held", Some((&b).into()), None);
    true
}
pub fn water_jet(r: &mut Run, id: f64, target: f64) -> Result<bool, String> {
    let a = actor(r, id).unwrap().clone();
    let b = actor(r, target).unwrap().clone();
    let range = distance(r, &a, &b);
    if !has(&a, "water-jet")
        || range <= std::f64::consts::SQRT_2
        || range > 6.0
        || !crate::monster_horn::permitted(r, &a, &b)
    {
        return Ok(false);
    }
    if crate::projectiles::possible_shot_range(r,&a,&b,6.).is_none()||!engine::enemy_spend(r,id,8.){return Ok(false)}
    let p=crate::projectiles::trace_committed_from_actor(r,&a,b.x,b.y);
    let range=crate::projectiles::traveled_distance(r,&a,b.x,b.y,&p);
    if r.simulation_revision.value_or(29.)>=36.{actor_mut_trusted(r,id).unwrap().conditions.insert("rangedRecovery".into(),1.);}
    let Some(target)=p.actor_id.filter(|_|range<=6.) else{
        crate::events::actor_event(r,&a,"projectile","Water jet",Some(crate::events::EventPoint{x:p.x,y:p.y}),None);
        return Ok(true)
    };
    let hit = engine::attack_ranged_resolved(r, id, target, 22.0, range, 1.0, false).map_err(|e| e.0)?;
    if hit && actor(r, target).is_some_and(|b| !b.dead.value_or(false)) {
        let b = actor(r, target).unwrap();
        let (toward_x, toward_y) = crate::portals::direction_toward(r, b, a.x, a.y);
        let (dx, dy) = (-toward_x, -toward_y);
        actor_mut_trusted(r, target).unwrap().conditions.insert("forcedMove".into(), 1.0);
        engine::move_actor(r, target, dx, dy, false).map_err(|e| e.0)?;
        if let Some(b) = actor_mut_trusted(r, target) {
            b.conditions.shift_remove("forcedMove");
        }
    }
    let b = actor(r, target).unwrap_or(&b);
    crate::events::actor_event(r, &a, "projectile", "Water jet", Some(b.into()), None);
    Ok(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dream_plane_grabs_cannot_apply_or_survive_a_plane_change() {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 34.0.into();
        r.dreamwalking = crate::dreamwalking::State { active: true, depth: 1., ..Default::default() }.into();
        r.player.conditions.clear();
        let (x, y, player) = (r.player.x, r.player.y, r.player.id);
        let holder = r.actors_at_mut(1.).first_mut().unwrap();
        holder.x = x + 1.; holder.y = y;
        holder.abilities.push("enemy:beast:grab".into());
        let id = holder.id;
        assert!(!hold_action(&mut r, id, player));
        assert!(!r.player.conditions.contains_key("grabbed"));
        for tick in [false, true] {
            r.player.conditions.insert("grabbed".into(), 3.);
            r.player.conditions.insert("grabbedBy".into(), id);
            if tick { tick_actor(&mut r, player); } else { release_invalid(&mut r); }
            assert!(!r.player.conditions.contains_key("grabbed"));
            assert!(!r.player.conditions.contains_key("grabbedBy"));
        }
        r.simulation_revision = 33.0.into();
        assert!(hold_action(&mut r, id, player));
        assert_eq!(r.player.conditions.get("grabbed"), Some(&3.));
    }
    #[test]
    fn grabs_match_frozen_reference() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/grab-v48.json")).unwrap();
        for c in cases.as_array().unwrap() {
            let mut r: Run = serde_json::from_value(c["initial"].clone()).unwrap();
            crate::events::begin();
            let p = r.player.id;
            let result = match c["operation"].as_str().unwrap() {
                "special" => {
                    deep_water(&mut r, 10.0, p)
                        || hold_action(&mut r, 10.0, p)
                        || water_jet(&mut r, 10.0, p).unwrap()
                }
                "offscreen" => {
                    crate::offscreen::tick_inactive_floors(&mut r).unwrap();
                    true
                }
                "held" => movement_held(&mut r, p),
                "tick" => {
                    tick_actor(&mut r, p);
                    true
                }
                _ => {
                    release_invalid(&mut r);
                    true
                }
            };
            assert_eq!(result, c["result"].as_bool().unwrap(), "{}", c["kind"]);
            let actual = serde_json::to_value(r).unwrap();
            if !crate::storage::canonical_equal(&actual, &c["expected"]) {
                std::fs::write(
                    "/tmp/grab-native.json",
                    serde_json::to_vec(&actual).unwrap(),
                )
                .unwrap();
                std::fs::write(
                    "/tmp/grab-source.json",
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
#[cfg(test)]
mod venom_tests {
    #[test]
    fn terminal_venom_matches_frozen_reference() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/venom-terminal-v48.json"))
                .unwrap();
        for c in cases.as_array().unwrap() {
            let mut r: crate::Run = serde_json::from_value(c["initial"].clone()).unwrap();
            crate::events::begin();
            crate::ai::enemy_turn_effects(&mut r, c["id"].as_f64().unwrap()).unwrap();
            let actual = serde_json::to_value(r).unwrap();
            assert!(
                crate::storage::canonical_equal(&actual, &c["expected"]),
                "{} state",
                c["name"]
            );
            assert!(
                crate::storage::canonical_equal(
                    &serde_json::to_value(crate::events::finish()).unwrap(),
                    &c["events"]
                ),
                "{} events",
                c["name"]
            );
        }
    }
}
