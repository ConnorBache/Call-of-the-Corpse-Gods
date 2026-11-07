//! Chronicle-only reductions keep turn, damage, corpse, RNG and replay behavior.
use corpse_core::{engine, population, Command, Field, Run};
use std::sync::Arc;

fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into();
    r.tick = 0.; r.depth = 1.; r.status = "playing".into();
    r.inventory.clear(); r.actors.clear(); r.ground.clear(); r.history.clear();
    r.haste_snapshot = Field::Missing; r.phase = Field::Missing; r.replay.clear(); r.offers.clear();
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.ability_order = Field::Missing; r.player.x = 8.; r.player.y = 8.;
    r.player.base_max_hp = 100.0.into(); r.player.max_hp = 100.; r.player.hp = 100.;
    r.player.stats.max_hp = Field::Missing; r.player.stats.perception = 300.;
    r.player.energy = 100.; r.player.stats.max_energy = 100.; r.next_id = 1000.;
    let mut f = r.floors[0].as_ref().clone();
    f.environment = Field::Missing; f.vents = Field::Missing; f.permanent_fire = Field::Missing;
    f.godbeast_event = Field::Missing; f.godbeast = false; f.doors = Field::Missing;
    f.dirty = true; f.last_tick = 0.; f.lights.fill(100.);
    let w = f.width(); let h = f.height();
    for y in 1..h-1 { for x in 1..w-1 { f.tiles[y*w+x] = "floor".into(); } }
    f.tiles[w+1] = "up".into(); f.tiles[(h-2)*w+w-2] = "down".into();
    r.floors = vec![Arc::new(f)];
    r.actors.insert("1".into(), Arc::new(vec![]));
    r.ground.insert("1".into(), Arc::new(vec![]));
    r.log.clear(); r.log_kinds = Vec::new().into(); r.log_archive = Field::Missing;
    r
}
fn npc(r: &mut Run, state: &str, x: f64) -> f64 {
    let id = r.next_id; r.next_id += 1.;
    let mut a = population::create_actor_revision("rat", id, x, 8., false, 46.).unwrap();
    a.state = state.into(); a.abilities.clear(); a.active.clear(); a.conditions.clear();
    r.actors_at_mut(1.).push(a); id
}
fn only_presentation_diff(a: &Run, b: &Run) {
    let strip = |r: &Run| {
        let mut r = r.clone();
        r.simulation_revision = 46.0.into(); r.log.clear();
        r.log_kinds = Field::Missing; r.log_archive = Field::Missing;
        serde_json::to_value(r).unwrap()
    };
    assert_eq!(strip(a), strip(b), "Chronicle suppression must not change serialized simulation state");
}
fn has(r: &Run, fragment: &str) -> bool { r.log.iter().any(|s| s.contains(fragment)) }

#[test]
fn wait_keeps_turn_reactions_and_events_without_chronicle_noise() {
    for phase in ["normal", "haste"] {
        let mut old = room(45.); old.phase = phase.to_string().into(); old.player.reactions = 0.0.into();
        let mut new = old.clone(); new.simulation_revision = 46.0.into();
        let a = engine::dispatch(&mut old, &Command::Wait).unwrap();
        let b = engine::dispatch(&mut new, &Command::Wait).unwrap();
        assert!(a.took_turn && b.took_turn); assert_eq!(new.tick, 1.);
        assert_eq!(a.events, b.events); assert!(b.events.iter().any(|e| e.label.as_deref() == Some("Wait")));
        assert!(has(&old, "You wait.")); assert!(!has(&new, "You wait."));
        only_presentation_diff(&old, &new);
    }
}

#[test]
fn player_slowness_still_takes_two_turns_without_delay_notice() {
    let mut old = room(45.); old.player.conditions.insert("slowed".into(), 4.);
    let mut new = old.clone(); new.simulation_revision = 46.0.into();
    engine::dispatch(&mut old, &Command::Wait).unwrap();
    engine::dispatch(&mut new, &Command::Wait).unwrap();
    assert_eq!(new.tick, 2.); assert_eq!(engine::condition(&new.player, "slowed"), 2.);
    assert!(has(&old, "Slowness delays your next action."));
    assert!(!has(&new, "Slowness delays your next action."));
    only_presentation_diff(&old, &new);
}

#[test]
fn other_creature_paralysis_keeps_skipped_action_and_decay_silently() {
    for state in ["hunting", "ally"] {
        let mut old = room(45.); let id = npc(&mut old, state, 9.);
        engine::actor_mut(&mut old, id).unwrap().conditions.insert("paralyzed".into(), 3.);
        let mut new = old.clone(); new.simulation_revision = 46.0.into();
        engine::dispatch(&mut old, &Command::Wait).unwrap();
        engine::dispatch(&mut new, &Command::Wait).unwrap();
        assert!(has(&old, "is paralyzed and cannot act."));
        assert!(!has(&new, "is paralyzed and cannot act."));
        assert_eq!(new.player.hp, 100.);
        assert_eq!(engine::condition(engine::actor(&new, id).unwrap(), "paralyzed"), 2.);
        only_presentation_diff(&old, &new);
    }
}

#[test]
fn slow_creature_keeps_even_turn_pause_without_notice() {
    for natural in [false, true] {
        let mut old = room(45.); old.tick = 1.; let id = npc(&mut old, "hunting", 9.);
        let a = engine::actor_mut(&mut old, id).unwrap();
        if natural { a.abilities.push("enemy:beast:slow".into()); }
        else { a.conditions.insert("slowed".into(), 4.); }
        let mut new = old.clone(); new.simulation_revision = 46.0.into();
        engine::dispatch(&mut old, &Command::Wait).unwrap();
        engine::dispatch(&mut new, &Command::Wait).unwrap();
        assert!(has(&old, "pauses; it is Slow.")); assert!(!has(&new, "pauses; it is Slow."));
        assert_eq!(new.player.hp, 100.); assert_eq!(new.tick, 2.);
        only_presentation_diff(&old, &new);
    }
}

#[test]
fn ordinary_player_and_other_actor_kills_keep_damage_and_corpses_without_followup() {
    for player_source in [false, true] {
        let mut old = room(45.); let id = npc(&mut old, "hunting", 9.);
        let source = if player_source { old.player.id } else { npc(&mut old, "ally", 10.) };
        engine::actor_mut(&mut old, source).unwrap().conditions.insert("lucky".into(), 1.);
        let mut new = old.clone(); new.simulation_revision = 46.0.into();
        engine::attack_ranged(&mut old, source, id, 10000., 1., 1., false).unwrap();
        engine::attack_ranged(&mut new, source, id, 10000., 1., 1., false).unwrap();
        assert!(has(&old, if player_source { "You kill " } else { " kills " }));
        assert!(!has(&new, " kill ") && !has(&new, " kills ") && !has(&new, " dies"));
        assert!(has(&new, " damage.")); assert!(engine::actor(&new, id).unwrap().dead.value_or(false));
        only_presentation_diff(&old, &new);
    }
}

#[test]
fn actor_owned_damage_suppresses_kill_followup_but_unowned_deaths_stay() {
    for owned in [false, true] {
        let mut old = room(45.); let id = npc(&mut old, "hunting", 9.);
        let source = owned.then_some(old.player.id);
        let mut new = old.clone(); new.simulation_revision = 46.0.into();
        engine::damage_environment(&mut old, id, 10000., source, "fire", true).unwrap();
        engine::damage_environment(&mut new, id, 10000., source, "fire", true).unwrap();
        assert!(has(&new, "Fire deals "));
        assert_eq!(has(&new, "is killed by fire."), !owned);
        assert!(!has(&new, "You kill "));
        only_presentation_diff(&old, &new);
    }
    for bypass in [false, true] {
        let mut r = room(46.); let id = npc(&mut r, "hunting", 9.);
        engine::actor_mut(&mut r, id).unwrap().hp = 0.;
        engine::resolve_death(&mut r, id, None, None, bypass).unwrap();
        assert!(has(&r, if bypass { "dies unpreventably." } else { "dies." }));
    }
}

#[test]
fn player_death_paralysis_and_contextual_kills_remain() {
    for bypass in [false, true] {
        let mut r = room(46.); let source = npc(&mut r, "hunting", 9.); let player = r.player.id;
        r.player.hp = 0.; engine::resolve_death(&mut r, player, Some(source), None, bypass).unwrap();
        assert!(has(&r, "You died.")); assert_eq!(r.status, "dead");
        assert_eq!(r.run_stats.as_ref().unwrap().death.as_ref().unwrap().unpreventable, bypass);
    }
    let mut r = room(46.); r.player.conditions.insert("paralyzed".into(), 2.);
    assert!(!engine::dispatch(&mut r, &Command::Move { dx: 1., dy: 0. }).unwrap().took_turn);
    assert!(has(&r, "You are paralyzed. Wait to advance time."));
    for state in ["sleeping", "wandering"] {
        let mut r = room(46.); let target = npc(&mut r, state, 9.);
        r.player.stats.offense = 10000.; engine::player_attack(&mut r, target).unwrap();
        assert!(has(&r, if state == "sleeping" { "while it is sleeping." } else { "while it is unaware." }));
    }
    let mut r = room(46.); let target = npc(&mut r, "hunting", 9.);
    let mut shard = corpse_core::borge::shard(2000.); shard.equipped = "hand1".to_string().into();
    r.inventory.push(shard); r.player.conditions.insert("lucky".into(), 1.);
    engine::player_attack(&mut r, target).unwrap();
    assert!(has(&r, "The corpse blackens and turns to stone."));
    assert!(engine::actor(&r, target).unwrap().dead.value_or(false));
}

#[test]
fn silent_waits_still_save_and_replay_exactly_without_archive_noise() {
    let mut initial = room(46.); initial.player.conditions.insert("slowed".into(), 4.);
    let mut e = corpse_core::temporal::Engine::new(initial.clone()).unwrap();
    for _ in 0..3 { e.dispatch(&Command::Wait).unwrap(); }
    assert!(!has(&e.run, "You wait.") && !has(&e.run, "Slowness delays"));
    assert!(e.run.log_archive.as_ref().is_none_or(|a| a.messages.iter().all(|s| !s.contains("You wait.") && !s.contains("Slowness delays"))));
    let saved = e.save().unwrap();
    assert_eq!(corpse_core::temporal::Engine::load(&saved).unwrap().save().unwrap(), saved);
    let record = corpse_core::replay::ReplayRecord::try_from_run(&e.run).unwrap();
    let mut replay = corpse_core::replay::ReplaySession::from_initial(initial, record).unwrap();
    while replay.step().unwrap() {}
    assert_eq!(replay.engine.save().unwrap(), saved);
}
