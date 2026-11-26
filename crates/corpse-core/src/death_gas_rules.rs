//! Optional command boundary for Greater Gassers' combined environmental gas.
use crate::{Field, Run};

pub const GAS: &str = "deathGas";
pub fn current(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 52. && matches!(r.death_gas_rules_from, Field::Value(_))
}

pub(crate) fn validate(r: &Run) -> Result<(), String> {
    if !r.death_gas_rules_from.is_missing() && (r.simulation_revision.value_or(29.) < 52.
        || !matches!(r.death_gas_rules_from, Field::Value(n) if n <= r.replay.len())) {
        return Err("Invalid Death Gas rules command boundary".into());
    }
    Ok(())
}

pub(crate) fn admit(r: &mut Run) -> Result<(), String> {
    validate(r)?;
    if r.simulation_revision.value_or(29.) >= 52. && r.death_gas_rules_from.is_missing() {
        r.death_gas_rules_from = r.replay.len().into();
    }
    Ok(())
}

pub(crate) fn apply_replay_policy(r: &mut Run, from: &Field<usize>, cursor: usize) {
    if from.as_ref() == Some(&cursor) {
        r.death_gas_rules_from = cursor.into();
    } else if cursor == 0 {
        r.death_gas_rules_from = Field::Missing;
    }
}

/// Preserve any partial immunity inherited from each component.
pub fn effects(a: &crate::Actor) -> (bool, bool) {
    let has = |id| a.abilities.iter().any(|s| s == id);
    if has("enemy:beast:gas-immunity") || has("enemy:pale:gas-immunity") {
        (false, false)
    } else {
        (!has("grisha-entropy"), true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ai, combat, engine, environment, events, population, replay, storage, temporal};

    fn room() -> Run {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 52.0.into();
        r.death_gas_rules_from = 0usize.into();
        r.gas_rules_from = 0usize.into();
        r.player.x = 10.;
        r.player.y = 10.;
        r.player.hp = 150.;
        r.player.max_hp = 150.;
        r.player.base_max_hp = 150.0.into();
        r.player.abilities.clear();
        r.player.active.clear();
        r.player.conditions.clear();
        r.player.ability_reserves = Field::Missing;
        r.player.ability_order = Field::Missing;
        r.inventory.clear();
        r.actors_at_mut(r.depth).clear();
        let f = r.floor_mut().unwrap();
        f.tiles.fill("floor".into());
        f.lights.fill(100.);
        f.environment = Field::Missing;
        f.doors = Field::Missing;
        f.portals = Field::Missing;
        f.non_euclidean = Field::Missing;
        f.vents = Field::Missing;
        r
    }

    #[test]
    fn death_gas_shows_both_effects_and_partial_immunity() {
        for (ability, hurts, hallucinates) in [("", true, true), ("grisha-entropy", false, true),
            ("enemy:pale:gas-immunity", false, false), ("enemy:beast:gas-immunity", false, false)] {
            let mut r = room();
            if !ability.is_empty() { r.player.abilities.push(ability.into()); }
            // Switch off Entropy's self-emission; it otherwise displaces the
            // test cloud in the engine's one-gas-per-cell terrain.
            if ability == "grisha-entropy" { r.player.active.push(ability.into()); }
            // Isolate the test cell so propagation leaves exposure deterministic.
            let f = r.floor_mut().unwrap();
            let w = f.width();
            let at = 10*w+10;
            for y in 9..=11 { for x in 9..=11 { if x!=10 || y!=10 { f.tiles[y*w+x]="wall".into(); }}}
            assert!(environment::add_gas(f, at, GAS, 100., -1.).unwrap());
            assert_eq!(environment::creature_hazard_at(&r, &r.player, at), hurts || hallucinates);
            let expected = combat::damage_packet(&r, &r.player, r.player.max_hp / 15., GAS, true);
            let health = r.player.hp;
            environment::tick_environment(&mut r).unwrap();
            assert_eq!(r.player.hp, health - if hurts { expected } else { 0. }, "{ability}");
            assert_eq!(engine::condition(&r.player, "hallucinating"), if hallucinates {5.} else {0.}, "{ability}");
            if hurts { assert!(r.log.iter().any(|s| s.starts_with("Death Gas deals"))); }
        }
    }

    #[test]
    fn death_gas_paid_cast_passive_emission_and_legacy_gate() {
        for (kind, rules, gas) in [("pale:Breather", true, "caustic"),
            ("pale:Greater Breather", false, "hallucinogenic"), ("pale:Greater Breather", true, GAS)] {
            let mut r = room();
            if !rules { r.death_gas_rules_from = Field::Missing; }
            let mut a = population::create_actor_revision(kind, 901., 12., 10., false, 52.).unwrap();
            a.state = "hunting".into();
            a.energy = 50.;
            a.conditions.insert("monsterGasPassiveVolume".into(), 180.);
            r.actors_at_mut(r.depth).push(a);
            let at = 10*r.floor().unwrap().width()+12;
            let p = r.player.id;
            events::begin();
            assert!(ai::enemy_action_options(&mut r, 901., p, false).unwrap());
            let out = events::finish();
            assert!(out.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="E57")));
            assert_eq!(engine::actor(&r,901.).unwrap().energy,40.);
            assert_eq!(environment::gas_kind_at(r.floor().unwrap().environment.as_ref().unwrap(),at), Some(gas));
            if gas==GAS { assert!(r.log.iter().any(|s|s.contains("cloud of Death Gas"))); }
            r.floor_mut().unwrap().environment=Field::Missing;
            environment::tick_environment(&mut r).unwrap();
            let e = r.floor().unwrap().environment.as_ref().unwrap();
            assert!(e.gas_type.as_ref().unwrap().iter().filter(|s|s.as_deref()==Some(gas)).count()>1, "diffusion after {kind} emits {gas}");
            assert_eq!(e.death_gas.as_ref().is_some(),gas==GAS);
            if gas == GAS {
                for (i, (kind, volume)) in e.gas_type.as_ref().unwrap().iter().zip(e.gas_volume.as_ref().unwrap()).enumerate() {
                    assert_eq!(e.death_gas.as_ref().unwrap()[i], if kind.as_deref() == Some(GAS) { *volume } else { 0. });
                }
            }
        }
    }

    #[test]
    fn death_gas_save_and_replay_boundaries() {
        let mut r = room();
        r.death_gas_rules_from=Field::Missing;
        assert!(!current(&r));
        let mut r=temporal::Engine::new(r).unwrap().run;
        assert!(current(&r));
        let w=r.floor().unwrap().width();
        environment::add_gas(r.floor_mut().unwrap(),10*w+10,GAS,100.,-1.).unwrap();
        let raw=storage::serialize(&r).unwrap();
        let restored=storage::deserialize(&raw).unwrap();
        assert_eq!(restored.death_gas_rules_from,r.death_gas_rules_from);
        assert_eq!(restored.floor().unwrap().environment,r.floor().unwrap().environment);
        let record=replay::ReplayRecord::from_run(&r);
        assert_eq!(record.death_gas_rules_from,r.death_gas_rules_from);
        let snapshot=temporal::Snapshot::capture(&r);
        assert!(snapshot.value().unwrap().get("deathGasRulesFrom").is_none());
        let mut prefix=restored;
        apply_replay_policy(&mut prefix,&Field::Value(2),0);
        assert!(!current(&prefix));
        apply_replay_policy(&mut prefix,&Field::Value(2),1);
        assert!(!current(&prefix));
        apply_replay_policy(&mut prefix,&Field::Value(2),2);
        assert!(current(&prefix));
    }
}
