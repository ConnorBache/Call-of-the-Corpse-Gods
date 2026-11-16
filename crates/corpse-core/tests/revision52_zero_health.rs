//! Exact death thresholds remain separate from whole-number HP presentation.
use corpse_core::{Command, Field, Run, engine, temporal::{Engine, Snapshot}, storage};
fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 52.0.into(); r.next_id = 1000.;
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.hp = 100.; r.player.max_hp = 100.; r.player.base_max_hp = 100.0.into();
    r.player.stats.defense = 0.; r.player.energy = 100.; r.player.stats.max_energy = 100.;
    r.player.x = 10.; r.player.y = 10.; r.player.dead = false.into();
    r.inventory.clear(); r.offers.clear(); r.actors.clear(); r.ground.clear();
    for depth in r.floors.iter().map(|f| f.depth).collect::<Vec<_>>() { r.actors_at_mut(depth); r.ground_at_mut(depth); }
    let f = r.floor_mut().unwrap(); f.tiles.fill("floor".into()); f.lights.fill(100.);
    f.doors = Field::Missing; f.environment = Field::Missing; f.portals = Field::Missing;
    f.non_euclidean = Field::Missing; f.godbeast_event = Field::Missing;
    r.history.clear(); r.replay.clear(); r.future = None; r.haste_snapshot = Field::Missing;
    r
}
fn enemy(r: &mut Run, hp: f64) -> f64 {
    let mut a = corpse_core::population::create_actor_revision("rat", 900., 12., 10., false, 52.).unwrap();
    a.hp = hp; a.max_hp = 100.; a.stats.defense = 0.; a.abilities.clear(); a.active.clear();
    a.conditions.clear(); a.state = "neutral".into(); a.dead = false.into();
    r.actors_at_mut(r.depth).push(a); 900.
}
fn packet(r: &mut Run, id: f64, n: f64) { engine::damage_environment(r, id, n, None, "", true).unwrap(); }
#[test]
fn exact_zero_and_overkill_finish_death_for_player_and_enemy_once() {
    for player in [false, true] { for (hp, n) in [(1., 1.), (1., 2.), (0.25, 1.)] {
        let mut r = room(); let id = if player { r.player.hp = hp; r.player.id } else { enemy(&mut r, hp) };
        packet(&mut r, id, n);
        let a = engine::actor(&r, id).unwrap(); assert_eq!(a.hp, 0.);
        if player { assert_eq!(r.status, "dead"); assert!(r.run_stats.as_ref().unwrap().death.as_ref().is_some()); }
        else { assert!(a.dead.value_or(false)); assert_eq!(a.corpse_age.value_or(-1.), 0.); }
        let once = r.clone(); packet(&mut r, id, n); assert_eq!(r, once, "dead target cannot die twice");
    }}
}
#[test]
fn stale_zero_cannot_act_or_regenerate_in_normal_or_haste_commands() {
    for phase in ["normal", "haste"] { for hp in [0., -1.] { for player in [false, true] {
        let mut r = room(); r.phase = phase.to_string().into();
        let id = if player { r.player.hp = hp; r.player.id } else { enemy(&mut r, hp) };
        let tick = r.tick; engine::dispatch(&mut r, &Command::Wait).unwrap();
        let a = engine::actor(&r, id).unwrap(); assert_eq!(a.hp, 0.);
        if player { assert_eq!(r.status, "dead"); assert_eq!(r.tick, tick); }
        else { assert!(a.dead.value_or(false)); }
    }}}
}
#[test]
fn positive_fraction_and_hanged_quarter_hp_survive_without_minimum_death_threshold() {
    let mut r = room(); let id = enemy(&mut r, 1.25); packet(&mut r, id, 1.);
    assert_eq!(engine::actor(&r, id).unwrap().hp, 0.25); assert!(!engine::actor(&r, id).unwrap().dead.value_or(false));
    r.player.abilities.push(corpse_core::hanged::ID.into()); r.player.hp = 1.; r.player.energy = 1.;
    let player = r.player.id; packet(&mut r, player, 1.);
    assert_eq!((r.player.hp, r.player.energy), (0.25, 0.)); assert_eq!(r.status, "playing");
    engine::dispatch(&mut r, &Command::Wait).unwrap();
    assert!(r.player.hp >= 0.25); assert_eq!(r.status, "playing");
    assert!(engine::actor(&r, id).unwrap().hp >= 0.25); assert!(!engine::actor(&r, id).unwrap().dead.value_or(false));
    packet(&mut r, player, 1.); assert_eq!(r.status, "dead");
}
#[test]
fn nonpositive_boundary_retains_ordered_rescue_and_actual_corpse_revival() {
    let mut r = room(); r.player.hp = 0.; r.player.abilities.push(corpse_core::divine_rescues::OVERFLOWING.into()); r.player.blood = 1.0.into();
    engine::dispatch(&mut r, &Command::Wait).unwrap();
    assert_eq!(r.status, "playing"); assert!(r.player.hp >= 1.); assert_eq!(r.player.blood.value_or(-1.), 0.);
    let id = enemy(&mut r, 1.); packet(&mut r, id, 1.);
    let tick = r.tick; let a = engine::actor_mut(&mut r, id).unwrap();
    assert!(corpse_core::corpses::can_revive_corpse(a)); corpse_core::corpses::revive_corpse(a, tick);
    assert!(!a.dead.value_or(false)); assert!(a.hp > 0.);
    engine::dispatch(&mut r, &Command::Wait).unwrap(); assert!(!engine::actor(&r, id).unwrap().dead.value_or(false));
}
#[test]
fn health_sync_resolves_newly_nonpositive_enemy_immediately() {
    let mut r = room(); let id = enemy(&mut r, 1.);
    engine::actor_mut(&mut r, id).unwrap().stats.defense = -17.;
    corpse_core::equipment::sync_actor_health(&mut r, id).unwrap();
    let a = engine::actor(&r, id).unwrap(); assert_eq!(a.hp, 0.); assert!(a.dead.value_or(false));
}
#[test]
fn offscreen_zero_uses_death_not_clean_floor_bulk_regeneration() {
    let mut r = room(); let id = enemy(&mut r, 0.);
    let mut other = r.floor().unwrap().clone(); other.depth = 2.; other.last_tick = 1.; other.dirty = false; other.seen.fill(true);
    r.floors.retain(|f| f.depth != 2.); r.floors.push(other.into()); let a = r.actors_at_mut(1.).pop().unwrap(); r.actors_at_mut(2.).push(a); r.ground_at_mut(2.);
    r.tick = 3.; corpse_core::offscreen::catch_up_floor(&mut r, 2.).unwrap();
    let a = r.actors_at(2.).iter().find(|a| a.id == id).unwrap(); assert_eq!(a.hp, 0.); assert!(a.dead.value_or(false));
    assert_eq!(r.player.hp, 100.); assert_eq!(r.status, "playing");
}
#[test]
fn save_resume_and_snapshot_retain_fractional_hp_and_settle_stale_zero_deterministically() {
    let mut r = room(); let id = enemy(&mut r, 0.); r.player.hp = 0.25;
    let snap = Snapshot::capture(&r); let mut restored = r.clone(); snap.restore(&mut restored).unwrap();
    assert_eq!(restored.player.hp, 0.25); assert_eq!(engine::actor(&restored, id).unwrap().hp, 0.);
    let text = storage::serialize(&r).unwrap(); let mut loaded = Engine::load(&text).unwrap();
    engine::dispatch(&mut r, &Command::Wait).unwrap(); loaded.dispatch(&Command::Wait).unwrap();
    assert_eq!(loaded.run.player.hp, r.player.hp); assert_eq!(loaded.run.rng, r.rng);
    assert_eq!(engine::actor(&loaded.run, id).unwrap().hp, 0.); assert!(engine::actor(&loaded.run, id).unwrap().dead.value_or(false));
}

#[test]
fn actual_melee_and_poison_exact_zero_finish_lifecycle() {
    for player in [false, true] {
        let mut r = room(); let id = enemy(&mut r, 100.);
        engine::actor_mut(&mut r, id).unwrap().x = 11.;
        let (attacker, victim) = if player { (id, r.player.id) } else { (r.player.id, id) };
        engine::actor_mut(&mut r, attacker).unwrap().conditions.insert("lucky".into(), 1.);
        let mut probe = r.clone(); engine::attack(&mut probe, attacker, victim, 1.).unwrap();
        let damage = 100. - engine::actor(&probe, victim).unwrap().hp; assert!(damage > 0.);
        engine::actor_mut(&mut r, victim).unwrap().hp = damage;
        engine::attack(&mut r, attacker, victim, 1.).unwrap();
        assert_eq!(engine::actor(&r, victim).unwrap().hp, 0.);
        assert!(if player {r.status == "dead"} else {engine::actor(&r, victim).unwrap().dead.value_or(false)});
        let mut r = room(); let victim = if player {r.player.hp=1.;r.player.id} else {enemy(&mut r,1.)};
        let a=engine::actor_mut(&mut r,victim).unwrap();a.conditions.insert("poisoned".into(),3.);
        a.conditions.insert(corpse_core::revision50_amulet_effects::POISON_DAMAGE.into(),1.);
        corpse_core::monster_statuses::tick_poisoned(&mut r,victim).unwrap();
        assert_eq!(engine::actor(&r,victim).unwrap().hp,0.);
        assert!(if player {r.status == "dead"} else {engine::actor(&r,victim).unwrap().dead.value_or(false)});
    }
}
