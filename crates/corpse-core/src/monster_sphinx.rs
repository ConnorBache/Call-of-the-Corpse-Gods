//! Invincible guardian's one-time provocation, delayed pursuit and sight loss.
use crate::{
    Field, Run,
    engine::{actor, actor_mut_trusted, condition},
};
pub fn provoked(r: &mut Run, id: f64, source: Option<f64>) -> bool {
    let source = source.and_then(|id| actor(r, id)).cloned();
    provoked_with_source(r, id, source.as_ref())
}
pub fn provoked_with_source(r: &mut Run, id: f64, source: Option<&crate::Actor>) -> bool {
    let Some(a) = actor(r, id).cloned() else {
        return false;
    };
    let Some(b) = source else { return false };
    if !a.abilities.iter().any(|s| s == "enemy:beast:sphinx")
        || !crate::revision50_control_amulets::hostile(r,&a,b)
    {
        return false;
    }
    if !a.conditions.contains_key("monsterSphinxProvokedAt") {
        let tick = r.tick;
        let a = actor_mut_trusted(r, id).unwrap();
        a.conditions.insert("monsterSphinxProvokedAt".into(), tick);
        a.conditions
            .insert("monsterSphinxHuntAt".into(), tick + 4.0);
        a.conditions.insert("monsterSphinxProvokerId".into(), b.id);
        a.conditions.insert("targetId".into(), b.id);
        a.last_seen = Field::Value(crate::model::ActorLastSeen { x: b.x, y: b.y });
        let a = a.clone();
        crate::animation_enemy::mark(r, &a, "E12", "eye-gold", crate::events::Stage::Special);
        crate::monster_horn::report(
            r,
            &a,
            &b,
            "stares at its attacker. It will wait three turns before hunting.",
        );
    }
    true
}
pub fn turn(r: &mut Run, id: f64, target: Option<f64>, movement_only: bool) -> bool {
    let Some(a) = actor(r, id).cloned() else {
        return false;
    };
    if a.dead.value_or(false)
        || a.hp <= 0.0
        || id == r.player.id
        || crate::half_here::is_clone(&a)
        || !a.abilities.iter().any(|s| s == "enemy:beast:sphinx")
    {
        return false;
    }
    if crate::ai::sleeping(&a) {
        return true;
    }
    let hunt = a.conditions.get("monsterSphinxHuntAt").copied();
    if hunt.is_none_or(|t| r.tick < t) {
        actor_mut_trusted(r, id).unwrap().state = "neutral".into();
        return true;
    }
    let mut target = target.and_then(|id| actor(r, id)).cloned();
    if condition(&a, "monsterSphinxLostSight") == 0.0 {
        let provoker = a
            .conditions
            .get("monsterSphinxProvokerId")
            .and_then(|id| {
                std::iter::once(&r.player)
                    .chain(r.actors_at(r.depth))
                    .find(|b| b.id == *id && !b.dead.value_or(false) && b.hp > 0.0)
            })
            .cloned();
        if let Some(b) = provoker.filter(|b| crate::vision::line_of_sight(r, &a, b)) {
            target = Some(b)
        } else {
            actor_mut_trusted(r, id)
                .unwrap()
                .conditions
                .insert("monsterSphinxLostSight".into(), 1.0);
        }
    }
    if target.is_some() && a.state != "hunting" { crate::animation_enemy::mark(r, &a, "E12", "eye-gold", crate::events::Stage::Special); }
    let a = actor_mut_trusted(r, id).unwrap();
    if let Some(b) = target {
        a.state = "hunting".into();
        a.last_seen = Field::Value(crate::model::ActorLastSeen { x: b.x, y: b.y });
        a.conditions.insert("targetId".into(), b.id);
        false
    } else {
        a.state = "wandering".into();
        a.last_seen = Field::Missing;
        a.conditions.shift_remove("targetId");
        movement_only
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sphinx_ai_matches_frozen_reference() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/sphinx-ai-v48.json")).unwrap();
        for c in cases.as_array().unwrap() {
            let mut r: Run = serde_json::from_value(c["initial"].clone()).unwrap();
            crate::events::begin();
            let result = if c["operation"] == "offscreen" {
                crate::offscreen::tick_inactive_floors(&mut r).unwrap();
                true
            } else if c["operation"] == "provoke" {
                if c["sourceActor"].is_object() {
                    let source: crate::Actor =
                        serde_json::from_value(c["sourceActor"].clone()).unwrap();
                    provoked_with_source(&mut r, 10.0, Some(&source))
                } else {
                    provoked(&mut r, 10.0, c["source"].as_f64())
                }
            } else {
                turn(
                    &mut r,
                    10.0,
                    c["target"].as_f64(),
                    c["movementOnly"].as_bool().unwrap(),
                )
            };
            assert_eq!(result, c["result"].as_bool().unwrap(), "{}", c["kind"]);
            let actual = serde_json::to_value(r).unwrap();
            if !crate::storage::canonical_equal(&actual, &c["expected"]) {
                std::fs::write("/tmp/sphinx-a.json", serde_json::to_vec(&actual).unwrap()).unwrap();
                std::fs::write(
                    "/tmp/sphinx-b.json",
                    serde_json::to_vec(&c["expected"]).unwrap(),
                )
                .unwrap();
            }
            assert!(
                crate::storage::canonical_equal(&actual, &c["expected"]),
                "{} state",
                c["kind"]
            );
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
