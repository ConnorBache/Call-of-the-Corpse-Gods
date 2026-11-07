use corpse_core::{Command, Field, Run, abilities, combat, engine, mini_me,
    revision50_ability_energy as energy};

fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 50.0.into();
    r.inventory.clear();
    // Empty floor contents without removing blocks required by save validation.
    for actors in r.actors.values_mut() { *actors = Default::default(); }
    for ground in r.ground.values_mut() { *ground = Default::default(); }
    r.player.x = 10.; r.player.y = 10.;
    r.player.abilities = vec![mini_me::ID.into()]; r.player.active.clear();
    r.player.conditions.clear(); r.player.ability_order = Field::Missing;
    r.player.stats.defense = 20.; r.player.base_max_hp = 100.0.into();
    r.player.max_hp = 220.; r.player.hp = 77.; r.player.energy = 61.;
    r.player.ability_reserves = Field::Missing;
    energy::initialize_reserve(&mut r.player, mini_me::ID, 30., 30.).unwrap();
    let f = r.floor_mut().unwrap(); f.tiles.fill("floor".into());
    f.lights.fill(100.); f.environment = Field::Missing; f.doors = Field::Missing;
    r
}
fn summon() -> Command {
    Command::Ability { id: mini_me::ID.into(), target: Field::Missing, x: Field::Missing, y: Field::Missing }
}

#[test]
fn real_activation_spends_own_reserve_and_creates_stats_only_ally() {
    let mut r = room();
    r.player.nightmares = 3.0.into(); r.player.reactions = 2.0.into();
    r.player.conditions.insert("hasted".into(), 5.);
    let stats = combat::effective_stats(&r, &r.player);
    let next = r.next_id;
    assert!(abilities::supported_for_run(&r, mini_me::ID));
    assert!(!engine::dispatch(&mut r, &summon()).unwrap().took_turn);
    assert_eq!(r.player.energy, 61.);
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()[mini_me::ID].energy, 0.);
    let a = engine::actor(&r, next).unwrap();
    assert_eq!((a.hp, a.max_hp, a.energy), (77., 220., 61.));
    assert_eq!(combat::effective_stats(&r, a), stats);
    assert!(a.abilities.is_empty() && a.active.is_empty());
    assert!(a.ability_reserves.is_missing() && a.copied_inventory.is_missing());
    assert!(a.nightmares.is_missing() && a.reactions.is_missing());
    assert!(!a.conditions.contains_key("hasted"));
    assert_eq!(r.next_id, next + 1.);
    assert_eq!(r.player.conditions["miniMeId"], next);
    assert!(a.perfect.value_or(false)); assert_eq!(a.state, "ally");
}

#[test]
fn actual_wait_and_dismiss_keep_copy_independent_without_ability_upkeep() {
    let mut r = room(); engine::dispatch(&mut r, &summon()).unwrap();
    let id = r.player.conditions["miniMeId"];
    let stats = engine::actor(&r, id).unwrap().stats.clone();
    r.player.stats.offense += 100.;
    assert!(engine::dispatch(&mut r, &Command::Wait).unwrap().took_turn);
    let a = engine::actor(&r, id).unwrap();
    assert_eq!(a.stats, stats); assert!(a.abilities.is_empty());
    assert!(a.ability_reserves.is_missing());
    let before = r.player.energy;
    assert!(!engine::dispatch(&mut r, &summon()).unwrap().took_turn);
    assert!(engine::actor(&r, id).is_none());
    assert!(!r.player.conditions.contains_key("miniMeId"));
    assert!(!r.player.active.iter().any(|s| s == mini_me::ID));
    assert_eq!(r.player.energy, before);
}

#[test]
fn missing_reserve_and_blocked_spawn_never_create_or_charge() {
    for blocked in [false, true] {
        let mut r = room();
        if blocked {
            r.floor_mut().unwrap().tiles.fill("wall".into());
            let width = r.floor().unwrap().width();
            r.floor_mut().unwrap().tiles[10 * width + 10] = "floor".into();
        } else { r.player.ability_reserves = Field::Missing; }
        let before = (r.next_id, r.player.energy, r.rng, r.tick);
        let reserves_before = r.player.ability_reserves.clone();
        if !blocked {
            // The effect's own admission still rejects missing funds without
            // inventing a reserve, charging energy or allocating a creature.
            abilities::activate(&mut r,mini_me::ID,&Field::Missing,&Field::Missing,&Field::Missing).unwrap();
            assert!(r.player.ability_reserves.is_missing());
            assert_eq!((r.next_id,r.player.energy,r.rng,r.tick),before);
            assert!(r.actors_at(r.depth).is_empty());
        }
        assert!(!engine::dispatch(&mut r, &summon()).unwrap().took_turn);
        assert_eq!((r.next_id, r.player.energy, r.rng, r.tick), before);
        if blocked {
            assert_eq!(r.player.ability_reserves,reserves_before);
        } else {
            // The command finish reconciles this already-owned blessing once.
            // That authored50 acquisition is not payment or a successful spawn.
            let reserves=r.player.ability_reserves.as_ref().unwrap();
            assert_eq!(reserves.len(),1);
            assert_eq!(reserves[mini_me::ID],energy::authored_reserve(50.,50.).unwrap());
        }
        assert!(r.actors_at(r.depth).is_empty());
        assert!(!r.player.conditions.contains_key("miniMeId"));
        assert!(!r.player.active.iter().any(|id|id==mini_me::ID));
    }
}

#[test]
fn real_activation_copies_exact_fractional_stats_without_live_sustain_or_rerounding() {
    use corpse_core::{Item, Stats, revision50_rings as rings, temporal::Snapshot};
    let mut r = room();
    r.player.stats = Stats { offense: 10., defense: 20., accuracy: 20., dodge: 20., dr: 20.,
        perception: 20., max_energy: 100., max_hp: 20.0.into() };
    r.inventory.push(Item { id: 9000., name: "Ring of Sustain Strength".into(), kind: "ring".into(),
        effect: rings::SUSTAIN_STRENGTH.to_string().into(), equipped: "ring1".to_string().into(),
        ..Default::default() });
    r.player.conditions.insert("weakened".into(), 5.);
    for key in ["defense", "accuracy", "dodge", "dr", "perception", "maxEnergy", "maxHp"] {
        r.player.conditions.insert(format!("buff:{key}"), -3.);
        r.player.conditions.insert(format!("buffTicks:{key}"), 5.);
    }
    corpse_core::equipment::sync_health(&mut r, false);
    let expected = Stats { offense: 7.5, defense: 18.5, accuracy: 18.5, dodge: 18.5, dr: 18.5,
        perception: 18.5, max_energy: 98.5, max_hp: 18.5.into() };
    assert_eq!(combat::effective_stats(&r, &r.player), expected);
    let max_hp = r.player.max_hp;
    let id = r.next_id;
    assert!(!engine::dispatch(&mut r, &summon()).unwrap().took_turn);
    let copy = engine::actor(&r, id).unwrap();
    assert_eq!(copy.stats, expected);
    assert_eq!(combat::effective_stats(&r, copy), expected);
    assert_eq!(copy.max_hp, max_hp);
    assert!(copy.abilities.is_empty() && copy.active.is_empty());
    assert!(copy.copied_inventory.is_missing() && copy.ability_reserves.is_missing());
    assert!(!rings::has_effect(&r, copy, rings::SUSTAIN_STRENGTH));
    assert!(!copy.conditions.contains_key("weakened"));

    // Later owner changes cannot re-evaluate the captured contributions.
    r.inventory.clear(); r.player.conditions.shift_remove("weakened");
    r.player.stats.offense = 100.;
    for key in ["defense", "accuracy", "dodge", "dr", "perception", "maxEnergy", "maxHp"] {
        r.player.conditions.shift_remove(&format!("buff:{key}"));
        r.player.conditions.shift_remove(&format!("buffTicks:{key}"));
    }
    assert!(engine::dispatch(&mut r, &Command::Wait).unwrap().took_turn);
    corpse_core::equipment::sync_actor_health(&mut r, id).unwrap();
    assert_eq!(combat::effective_stats(&r, engine::actor(&r, id).unwrap()), expected);
    assert_eq!(engine::actor(&r, id).unwrap().max_hp, max_hp);

    // Newly inflicted penalties still apply normally: the snapshot carries no ring.
    engine::actor_mut(&mut r, id).unwrap().conditions.insert("weakened".into(), 3.);
    assert_eq!(combat::effective_stats(&r, engine::actor(&r, id).unwrap()).offense, 2.5);
    engine::actor_mut(&mut r, id).unwrap().conditions.shift_remove("weakened");
    let snapshot = Snapshot::from_value(Snapshot::capture(&r).value().unwrap()).unwrap();
    engine::actor_mut(&mut r, id).unwrap().stats.offense = 999.;
    snapshot.restore(&mut r).unwrap();
    assert_eq!(combat::effective_stats(&r, engine::actor(&r, id).unwrap()), expected);
    let restored = corpse_core::storage::deserialize(&corpse_core::storage::serialize(&r).unwrap()).unwrap();
    assert_eq!(combat::effective_stats(&restored, engine::actor(&restored, id).unwrap()), expected);
}

#[test]
fn fractional_snapshot_exception_does_not_change_ordinary_or_historical_base_rounding() {
    let mut r = room();
    r.player.stats.offense = 7.5; r.player.stats.max_hp = 2.5.into();
    assert_eq!(combat::effective_stats(&r, &r.player).offense, 8.);
    assert_eq!(combat::effective_stats(&r, &r.player).max_hp, 3.0.into());
    let mut a = r.player.clone(); a.id = r.next_id; a.state = "ally".into();
    a.abilities.clear(); a.active.clear(); a.conditions.clear();
    a.conditions.insert("miniMe".into(), 1.);
    assert_eq!(combat::effective_stats(&r, &a).offense, 7.5);
    assert_eq!(combat::effective_stats(&r, &a).max_hp, 2.5.into());
    for revision in [29., 33., 43., 46., 49.] {
        r.simulation_revision = revision.into();
        assert_eq!(combat::effective_stats(&r, &a).offense, 8.);
        assert_eq!(combat::effective_stats(&r, &a).max_hp, 3.0.into());
    }
    r.simulation_revision = 50.0.into(); a.conditions.shift_remove("miniMe");
    // Other creatures retain their normal rounding and enemy-health conversion.
    assert_eq!(combat::effective_stats(&r, &a).offense, 8.);
}
