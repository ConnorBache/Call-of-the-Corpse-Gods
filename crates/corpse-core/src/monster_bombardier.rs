//! Bombardier's authored all-energy, ignition-before-damage radius-five blast.
use crate::{
    Run,
    engine::{self, actor},
};
pub fn special_action(r: &mut Run, id: f64, target: f64) -> Result<bool, String> {
    let a = actor(r, id).ok_or("Bombardier actor missing")?.clone();
    let b = actor(r, target).ok_or("Bombardier target missing")?.clone();
    if !a.abilities.iter().any(|s| s == "enemy:beast:detonate")
        || (a.x - b.x).hypot(a.y - b.y) > 5.0
        || a.energy <= 0.0
        || !engine::enemy_spend(r, id, a.energy)
    {
        return Ok(false);
    }
    crate::environment::explode(
        r,
        a.x,
        a.y,
        crate::environment::ExplosionOptions {
            radius: 5.0,
            source: Some(id),
            direct_impact: true,
            ignite_first: true,
            ..Default::default()
        },
        None,
    )?;
    let a = actor(r, id).unwrap_or(&a).clone();
    let b = actor(r, target).unwrap_or(&b).clone();
    crate::monster_horn::report(
        r,
        &a,
        &b,
        "empties its reservoirs in an enormous explosion!",
    );
    Ok(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bombardier_matches_frozen_reference() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/bombardier-v48.json")).unwrap();
        for c in cases.as_array().unwrap() {
            let mut r: Run = serde_json::from_value(c["initial"].clone()).unwrap();
            crate::events::begin();
            let p = r.player.id;
            let result = if c["operation"] == "offscreen" {
                crate::offscreen::tick_inactive_floors(&mut r).unwrap();
                true
            } else {
                special_action(&mut r, 10.0, p).unwrap()
            };
            assert_eq!(result, c["result"].as_bool().unwrap(), "{}", c["kind"]);
            let actual = serde_json::to_value(r).unwrap();
            if !crate::storage::canonical_equal(&actual, &c["expected"]) {
                std::fs::write(
                    "/tmp/bombardier-native.json",
                    serde_json::to_vec(&actual).unwrap(),
                )
                .unwrap();
                std::fs::write(
                    "/tmp/bombardier-source.json",
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
