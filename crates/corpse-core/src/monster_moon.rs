//! Valta/Moonspawn approach blink and charged moonbeam in authored priority.
use crate::{
    Run,
    engine::{self, actor, actor_mut_trusted, condition},
};
pub fn special_action(
    r: &mut Run,
    id: f64,
    target: f64,
    allow_approach_blink: bool,
) -> Result<bool, String> {
    let a = actor(r, id).ok_or("Moon actor missing")?.clone();
    let b = actor(r, target).ok_or("Moon target missing")?.clone();
    let has = |s: &str| a.abilities.iter().any(|v| v == &format!("enemy:beast:{s}"));
    let range = crate::portals::actor_distance(r, &a, &b);
    let possible=crate::projectiles::possible_shot_range(r,&a,&b,f64::INFINITY);
    let ray_range=possible.unwrap_or(f64::INFINITY);
    let permitted = crate::monster_horn::permitted(r, &a, &b);
    let clear = |_r: &Run| possible.is_some();
    if has("moon-laser") && condition(&a, "laserCharge") != 0.0 {
        if !clear(r) {
            return Ok(false);
        }
        actor_mut_trusted(r, id)
            .unwrap()
            .conditions
            .shift_remove("laserCharge");
        if !permitted || ray_range > 16.0 {
            return Ok(true);
        }
        if r.simulation_revision.value_or(29.)>=36. {
            let actual=crate::projectiles::trace_committed_from_actor(r,&a,b.x,b.y);
    crate::events::tag_path(&a,"E49","bolt-blue");
            let traveled=crate::projectiles::traveled_distance(r,&a,b.x,b.y,&actual);
            actor_mut_trusted(r,id).unwrap().conditions.insert("rangedRecovery".into(),1.);
            if let Some(victim)=actual.actor_id.filter(|_|traveled<=16.){engine::attack_ranged_resolved(r,id,victim,crate::balance52::local_amount(r,&a,"monsterSpecialDamage",if crate::combat::raw_attack_damage_enabled(r.simulation_revision.value_or(29.)) { 60. } else { 120. }),traveled,1.,false).map_err(|e|e.0)?;}
            crate::events::actor_event(r,&a,"projectile","Moonbeam",Some(crate::events::EventPoint{x:actual.x,y:actual.y}),None);
        }else{
            engine::attack_ranged(r, id, target, 120.0, ray_range, 1.0, false).map_err(|e| e.0)?;
            crate::events::actor_event(r, &a, "projectile", "Moonbeam", Some((&b).into()), None);
        }
        crate::monster_horn::report(r, &a, &b, "releases its charged moonbeam.");
        return Ok(true);
    }
    if allow_approach_blink && has("blink") && range > 3.0 && condition(&a, "beastCooldown") == 0.0
    {
        let f = r.floor().unwrap();
        let mut choices = Vec::new();
        for y in (b.y as i32 - 3).max(1)..=(b.y as i32 + 3).min(f.height() as i32 - 2) {
            for x in (b.x as i32 - 3).max(1)..=(b.x as i32 + 3).min(f.width() as i32 - 2) {
                let mut q = a.clone();
                q.x = x as f64;
                q.y = y as f64;
                let near = crate::portals::actor_distance(r, &q, &b);
                if near <= 3.0
                    && near < range
                    && crate::portals::actor_distance(r, &q, &a) <= 5.0
                    && crate::monster_horn::free(r, q.x, q.y, Some(&a))
                    && crate::ichor::movement_allowed(r, &a, q.x, q.y)
                    && crate::vision::line_of_sight(r, &a, &q)
                {
                    choices.push(q);
                }
            }
        }
        if !choices.is_empty() && crate::monster_horn::spell_spend(r, id, 10.0) {
            let q = &choices[(crate::rng::random(r) * choices.len() as f64).floor() as usize];
            crate::events::actor_event(r, &a, "pulse", "Vanish", None, None);
            let a = actor_mut_trusted(r, id).unwrap();
            a.x = q.x;
            a.y = q.y;
            crate::travel::enemy_moved(r, id)?;
            let a = actor(r, id).unwrap().clone();
            crate::events::actor_event(r, &a, "pulse", "Appear", None, None);
            r.floor_mut().unwrap().dirty = true;
            actor_mut_trusted(r, id)
                .unwrap()
                .conditions
                .insert("beastCooldown".into(), 3.0);
            crate::monster_horn::report(r, &a, &b, "folds through moonlight.");
            return Ok(true);
        }
    }
    if has("moon-laser")
        && range > std::f64::consts::SQRT_2
        && ray_range <= 16.0
        && permitted
        && clear(r)
        && crate::monster_horn::spell_spend(r, id, 20.0)
    {
        actor_mut_trusted(r, id)
            .unwrap()
            .conditions
            .insert("laserCharge".into(), 1.0);
        crate::events::actor_event(r, &a, "pulse", "Charging moonbeam", None, None);
        crate::monster_horn::report(r, &a, &b, "charges a searing moonbeam.");
        return Ok(true);
    }
    Ok(false)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moon_matches_frozen_reference() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/moon-v48.json")).unwrap();
        for c in cases.as_array().unwrap() {
            let mut r: Run = serde_json::from_value(c["initial"].clone()).unwrap();
            crate::events::begin();
            let p = r.player.id;
            let result = if c["operation"] == "offscreen" {
                crate::offscreen::tick_inactive_floors(&mut r).unwrap();
                true
            } else {
                special_action(&mut r, 10.0, p, c["allowApproachBlink"].as_bool().unwrap()).unwrap()
            };
            assert_eq!(result, c["result"].as_bool().unwrap(), "{}", c["kind"]);
            let actual = serde_json::to_value(r).unwrap();
            if !crate::storage::canonical_equal(&actual, &c["expected"]) {
                std::fs::write(
                    "/tmp/moon-native.json",
                    serde_json::to_vec(&actual).unwrap(),
                )
                .unwrap();
                std::fs::write(
                    "/tmp/moon-source.json",
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
