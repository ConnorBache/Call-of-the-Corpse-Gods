//! Steam scalds on contact from a recorded command boundary.
use crate::{Field, Run};

pub fn current(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 50. && !matches!(r.steam_rules_from, Field::Null)
}

pub fn validate(r: &Run) -> Result<(), String> {
    if !r.steam_rules_from.is_missing() && (r.simulation_revision.value_or(29.) < 50.
        || r.steam_rules_from.as_ref().is_some_and(|&n| n > r.replay.len())) {
        return Err("Invalid steam rules command boundary".into());
    }
    Ok(())
}

/// Old live saves adopt the rule after their existing command ledger.
pub fn admit_current(r: &mut Run) -> Result<(), String> {
    validate(r)?;
    if r.simulation_revision.value_or(29.) >= 50. && r.steam_rules_from.is_missing() {
        r.steam_rules_from = r.replay.len().into();
    }
    Ok(())
}

pub fn apply_replay_policy(r: &mut Run, from: &Field<usize>, cursor: usize) {
    if r.simulation_revision.value_or(29.) < 50. { return; }
    if from.as_ref() == Some(&cursor) { r.steam_rules_from = cursor.into(); }
    else if cursor == 0 { r.steam_rules_from = Field::Null; }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, Item, engine, environment, temporal};

    fn room() -> Run {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 52.0.into();
        r.steam_rules_from = 0usize.into();
        r.player.x = 10.; r.player.y = 10.;
        r.player.hp = 100.; r.player.max_hp = 100.; r.player.base_max_hp = 100.0.into();
        r.player.conditions.clear(); r.player.abilities.clear(); r.player.active.clear();
        r.actors.clear(); r.ground.clear(); r.inventory.clear();
        for depth in r.floors.iter().map(|f| f.depth).collect::<Vec<_>>() {
            r.actors_at_mut(depth); r.ground_at_mut(depth);
        }
        let f = r.floor_mut().unwrap();
        f.tiles.fill("floor".into()); f.lights.fill(100.);
        f.doors = Field::Missing; f.portals = Field::Missing; f.non_euclidean = Field::Missing;
        f.environment = Field::Missing; f.vents = Field::Missing;
        f.hellfire = Field::Missing; f.permanent_fire = Field::Missing;
        r
    }

    fn steam(r: &mut Run) -> usize {
        let f = r.floor_mut().unwrap();
        let at = 10 * f.width() + 10;
        assert!(environment::add_gas(f, at, "steam", 250., -1.).unwrap());
        at
    }

    #[test]
    fn steam_scales_like_burning_and_does_not_follow_the_actor() {
        let mut r = room();
        let at = steam(&mut r);
        assert!(environment::creature_hazard_at(&r, &r.player, at));
        environment::tick_environment(&mut r).unwrap();
        assert_eq!(r.player.hp, 96.);
        assert_eq!(engine::condition(&r.player, "burning"), 0.);
        assert!(r.log.iter().any(|line| line.contains("Steam scalds you")));
        r.player.x = 17.;
        environment::tick_environment(&mut r).unwrap();
        assert_eq!(r.player.hp, 96.);
        assert_eq!(engine::condition(&r.player, "burning"), 0.);
    }

    #[test]
    fn steam_preserves_fire_protection_and_does_not_double_existing_heat() {
        for (mode, hp) in [("burning", 96.), ("fire immune", 100.), ("legacy fire ring", 98.), ("retired fire ring", 96.), ("gas immune", 96.)] {
            let mut r = room();
            steam(&mut r);
            match mode {
                "burning" => { r.player.conditions.insert("burning".into(), 2.); }
                "fire immune" => r.player.abilities.push("enemy:beast:fire-immunity".into()),
                "gas immune" => r.player.abilities.push("enemy:beast:gas-immunity".into()),
                "legacy fire ring" | "retired fire ring" => {
                    if mode == "legacy fire ring" { r.ring_rules_from = Field::Null; }
                    r.inventory.push(Item {
                        id: 1001., kind: "ring".into(),
                        name: "Ring of Fire Resistance".into(),
                        effect: "fire-resistance".to_string().into(),
                        ..Default::default()
                    });
                }
                _ => unreachable!(),
            }
            environment::tick_environment(&mut r).unwrap();
            assert_eq!(r.player.hp, hp, "{mode}");
            assert_eq!(engine::condition(&r.player, "burning"), if mode == "burning" {1.} else {0.}, "{mode}");
        }
    }

    #[test]
    fn steam_generated_by_quenching_on_deep_water_still_scalds() {
        let mut r = room();
        let at = {
            let f = r.floor_mut().unwrap();
            let at = 10 * f.width() + 10;
            f.tiles[at] = "water".into();
            at
        };
        let n = r.floor().unwrap().tiles.len();
        {
            let mut fire = crate::fire_adapter::FireAdapter::new(&mut r, vec![false; n]).unwrap();
            fire.ignite_creature(at, -1., 0.);
            fire.resolve().unwrap();
        }
        let h = crate::hazard_memory::observed_hazards_at_revision(r.floor().unwrap(), at, r.tick, 52.);
        assert_eq!(h.gas.as_ref().map(|g| g.r#type.as_str()), Some("steam"));
        assert!(h.fire.is_missing());
        environment::tick_environment(&mut r).unwrap();
        assert_eq!(r.player.hp, 96.);
        assert_eq!(engine::condition(&r.player, "burning"), 0.);
    }

    #[test]
    fn steam_uses_the_active_gas_and_records_its_own_lethal_cause() {
        let mut r = room();
        let at = steam(&mut r);
        environment::add_gas(r.floor_mut().unwrap(), at, "hydrogen", 30., -1.).unwrap();
        assert!(!environment::creature_hazard_at(&r, &r.player, at));
        environment::tick_environment(&mut r).unwrap();
        assert_eq!(r.player.hp, 100.);

        steam(&mut r);
        r.player.hp = 3.;
        environment::tick_environment(&mut r).unwrap();
        assert_eq!(r.status, "dead");
        let death = r.run_stats.as_ref().unwrap().death.as_ref().unwrap();
        assert_eq!(death.cause.as_ref().map(String::as_str), Some("steam"));
        assert_eq!(death.reason, "Steam");
    }

    #[test]
    fn steam_historical_prefix_is_harmless_and_live_adoption_replays() {
        let mut initial = room();
        steam(&mut initial);
        initial.steam_rules_from = Field::Null;
        let mut old = temporal::Engine::new(initial.clone()).unwrap();
        old.dispatch(&Command::Wait).unwrap();
        assert_eq!(old.run.player.hp, 100.);
        let mut live = temporal::Engine::load(&old.save().unwrap()).unwrap();
        assert_eq!(live.run.steam_rules_from, Field::Value(1));
        live.dispatch(&Command::Wait).unwrap();
        assert!(live.run.player.hp < 100.);
        let record = crate::replay::ReplayRecord::from_run(&live.run);
        let mut replay = crate::replay::ReplaySession::from_initial(initial, record).unwrap();
        assert!(replay.step().unwrap());
        assert_eq!(replay.engine.run.player.hp, 100.);
        assert!(replay.step().unwrap());
        assert_eq!(replay.engine.run.player.hp, live.run.player.hp);
        assert_eq!(replay.engine.run.steam_rules_from, live.run.steam_rules_from);
        replay.restart().unwrap();
        while replay.step().unwrap() {}
        assert_eq!(replay.engine.run.player.hp, live.run.player.hp);
    }
}
