//! Independent integration QA: real command/temporal paths plus controlled damage
//! and environment boundaries. These are fixture replays, not seeded-run parity.
use corpse_core::{
    Command, Field, GroundItem, Run, abilities, dreamwalking as dream, engine,
    environment, half_here, inventory, phantasmii_generation as quest, population,
    replay::{ReplayRecord, ReplaySession}, storage, temporal::Engine,
};

fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.floors.truncate(1); r.actors.retain(|k, _| k == "1"); r.ground.retain(|k, _| k == "1");
    r.simulation_revision = 34.0.into(); r.actors_at_mut(1.).clear(); r.ground_at_mut(1.).clear();
    r.inventory.clear(); r.offers.clear(); r.history.clear(); r.replay.clear(); r.future = None;
    r.phase = Field::Missing; r.haste_snapshot = Field::Missing; r.log.clear();
    r.log_kinds = Field::Missing; r.log_archive = Field::Missing;
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.x = 8.; r.player.y = 8.; r.player.hp = 100.; r.player.max_hp = 100.;
    r.player.energy = 1000.; r.player.stats.max_energy = 1000.;
    r.next_id = 1000.; r.rng = 1.;
    let f = r.floor_mut().unwrap();
    f.environment = Field::Missing; f.vents = Field::Missing; f.permanent_fire = Field::Missing;
    f.godbeast_event = Field::Missing; f.doors = Field::Missing; f.light_sources = Field::Missing;
    f.blood = Field::Missing; f.roots = Field::Missing; f.fungal_roots = Field::Missing;
    f.razorleaves = Field::Missing; f.grates = Field::Missing;
    let w = f.width(); let h = f.height();
    for y in 1..h-1 { for x in 1..w-1 { f.tiles[y*w+x] = "floor".into(); } }
    f.tiles[w+1] = "up".into(); f.tiles[(h-2)*w+w-2] = "down".into(); f.lights.fill(0.);
    r
}
fn ability(id: &str) -> Command {
    Command::Ability { id: id.into(), target: Field::Missing, x: Field::Missing, y: Field::Missing }
}
fn at(id: &str, x: f64, y: f64) -> Command {
    Command::Ability { id: id.into(), target: Field::Missing, x: x.into(), y: y.into() }
}
fn step(e: &mut Engine, dx: f64, dy: f64) {
    assert!(e.dispatch(&Command::Move { dx, dy }).unwrap().took_turn);
}
fn sleep(r: &mut Run) {
    r.dreamwalking = dream::State { unlocked: true, ..Default::default() }.into();
    assert_eq!(dream::ability(r, dream::DREAMWALK_ID).unwrap(), Some(true));
}
fn rat(r: &mut Run, x: f64, y: f64) -> f64 {
    let id = r.next_id; r.next_id += 1.;
    let mut a = population::create_actor_revision("rat", id, x, y, false, 34.).unwrap();
    a.state = "sleeping".into(); a.hp = 100.; a.max_hp = 100.;
    a.stats.accuracy = 10000.; a.stats.offense = 20.;
    r.actors_at_mut(r.depth).push(a); id
}
fn tile(r: &Run, x: usize, y: usize) -> usize { y*r.floor().unwrap().width()+x }
fn quest_room() -> Run {
    let mut r = room();
    let f = r.floor_mut().unwrap();
    let event = corpse_core::generation::reserve_godbeast_room(
        &mut f.tiles, "Phantasmii", &mut || 0.5, &[], &[], &Default::default(),
    ).unwrap();
    let altar = event.altar.as_ref().unwrap(); let (x, y) = (altar.x, altar.y);
    f.godbeast_event = event.into(); f.godbeast = true;
    r.player.x = x+1.; r.player.y = y;
    r.ground_at_mut(1.).push(GroundItem { x: x+1., y, item: quest::dagger(500.), ..Default::default() });
    for n in 0..3 {
        r.ground_at_mut(1.).push(GroundItem { x: x+1.+n as f64, y: y+1.,
            item: quest::shard(501.+n as f64), ..Default::default() });
    }
    r
}
fn unlock_commands() -> Vec<Command> {
    vec![Command::Pickup, Command::Move { dx: -1., dy: 0. }, Command::Use { id: 500. },
        Command::Move { dx: 1., dy: 1. }, Command::Move { dx: 1., dy: 0. },
        Command::Move { dx: 1., dy: 0. }, ability(dream::DREAMWALK_ID),
        Command::Move { dx: -1., dy: 0. }, ability(dream::DREAMWALK_ID)]
}

#[test]
fn dagger_altar_three_shard_stack_unlock_save_and_toggle_are_real_commands() {
    let mut e = Engine::new(quest_room()).unwrap();
    e.dispatch(&Command::Pickup).unwrap();
    assert!(e.run.inventory.iter().any(quest::is_dagger));
    let tick = e.run.tick;
    assert!(!e.dispatch(&Command::Use { id: 500. }).unwrap().took_turn);
    assert_eq!(e.run.tick, tick); assert!(!dream::active(&e.run));
    step(&mut e, -1., 0.);
    let anchor = (e.run.player.x, e.run.player.y);
    assert!(e.dispatch(&Command::Use { id: 500. }).unwrap().took_turn);
    assert!(dream::active(&e.run)); assert!(!dream::unlocked(&e.run));
    assert_eq!((dream::body(&e.run).unwrap().x, dream::body(&e.run).unwrap().y), anchor);
    step(&mut e, 1., 1.); step(&mut e, 1., 0.);
    assert_eq!(e.run.inventory.iter().filter(|i| quest::is_shard(i)).map(inventory::quantity).sum::<f64>(), 2.);
    e = Engine::load(&e.save().unwrap()).unwrap();
    assert!(dream::active(&e.run)); assert!(!dream::unlocked(&e.run));
    step(&mut e, 1., 0.);
    let shards: Vec<_> = e.run.inventory.iter().filter(|i| quest::is_shard(i)).collect();
    assert_eq!(shards.len(), 1); assert_eq!(inventory::quantity(shards[0]), 3.);
    assert!(dream::unlocked(&e.run)); assert_eq!(inventory::occupancy(&e.run), 2.);
    assert!(e.run.inventory.iter().any(quest::is_dagger));
    assert!(!e.dispatch(&ability(dream::WAKE_UP_ID)).unwrap().took_turn);
    assert!(e.dispatch(&ability(dream::DREAMWALK_ID)).unwrap().took_turn);
    assert_eq!((e.run.player.x, e.run.player.y), anchor); assert!(dream::body(&e.run).is_none());
    step(&mut e, -1., 0.);
    let next_anchor = (e.run.player.x, e.run.player.y);
    assert!(e.dispatch(&ability(dream::DREAMWALK_ID)).unwrap().took_turn);
    assert_eq!((dream::body(&e.run).unwrap().x, dream::body(&e.run).unwrap().y), next_anchor);
    assert!(dream::unlocked(&e.run));
}

#[test]
fn quest_fixture_replay_reproduces_events_and_complete_dream_state() {
    let initial = quest_room(); let mut expected = Engine::new(initial.clone()).unwrap();
    let outcomes: Vec<_> = unlock_commands().iter().map(|c| expected.dispatch(c).unwrap().events).collect();
    let mut replay = ReplaySession::from_initial(initial, ReplayRecord::from_run(&expected.run)).unwrap();
    for events in outcomes { assert_eq!(replay.step_with_outcome().unwrap().unwrap().events, events); }
    assert!(replay.finished());
    assert_eq!(replay.engine.save().unwrap(), expected.save().unwrap());
    assert!(dream::active(&replay.engine.run)); assert!(dream::unlocked(&replay.engine.run));
}

#[test]
fn early_wake_keeps_partial_shards_and_reentry_still_requires_the_altar() {
    let mut e = Engine::new(quest_room()).unwrap();
    e.dispatch(&Command::Pickup).unwrap(); step(&mut e, -1., 0.);
    let anchor = (e.run.player.x, e.run.player.y);
    e.dispatch(&Command::Use { id: 500. }).unwrap(); step(&mut e, 1., 1.);
    assert!(!dream::unlocked(&e.run));
    assert!(e.dispatch(&ability(dream::WAKE_UP_ID)).unwrap().took_turn);
    assert_eq!((e.run.player.x, e.run.player.y), anchor);
    assert!(!dream::active(&e.run)); assert!(dream::body(&e.run).is_none());
    assert_eq!(e.run.inventory.iter().filter(|i| quest::is_shard(i)).map(inventory::quantity).sum::<f64>(), 1.);
    assert!(!e.dispatch(&ability(dream::DREAMWALK_ID)).unwrap().took_turn);
    step(&mut e, -1., 0.);
    assert!(!e.dispatch(&Command::Use { id: 500. }).unwrap().took_turn);
    step(&mut e, 1., 0.);
    assert!(e.dispatch(&Command::Use { id: 500. }).unwrap().took_turn);
    step(&mut e, 1., 1.); step(&mut e, 1., 0.); step(&mut e, 1., 0.);
    assert!(dream::unlocked(&e.run));
    assert_eq!(e.run.inventory.iter().filter(|i| quest::is_shard(i)).map(inventory::quantity).sum::<f64>(), 3.);
    assert_eq!(e.run.inventory.iter().filter(|i| quest::is_shard(i)).count(), 1);
}

#[test]
fn body_melee_damages_shared_pool_without_waking_or_cross_plane_projection_hit() {
    let mut r = room(); let attacker = rat(&mut r, 9., 8.); sleep(&mut r);
    let body = dream::body(&r).unwrap().id; r.player.x = 12.;
    engine::attack(&mut r, attacker, 0., 1.).unwrap(); assert_eq!(r.player.hp, 100.);
    engine::actor_mut(&mut r, attacker).unwrap().conditions.insert("lucky".into(), 1.);
    engine::attack(&mut r, attacker, body, 1.).unwrap();
    assert!(r.player.hp < 100.); assert_eq!(dream::body(&r).unwrap().hp, r.player.hp);
    assert!(dream::active(&r)); assert_eq!(r.player.x, 12.);
    assert_eq!(r.log.iter().filter(|s| s.contains("sleeping body takes")).count(), 1);
}

#[test]
fn lethal_sleeping_body_packets_resolve_shared_death_exactly_once() {
    let mut r = room(); let attacker = rat(&mut r, 9., 8.); sleep(&mut r);
    let body = dream::body(&r).unwrap().id;
    engine::damage_environment(&mut r, body, 1000., Some(attacker), "attack", true).unwrap();
    assert_eq!(r.status, "dead"); assert_eq!(r.player.hp, 0.);
    let death = r.run_stats.as_ref().unwrap().death.clone();
    if engine::actor(&r, body).is_some() {
        engine::damage_environment(&mut r, body, 4., None, "fire", true).unwrap();
    }
    assert_eq!(r.player.hp, 0.);
    assert_eq!(r.run_stats.as_ref().unwrap().death, death,
        "terminal provenance changed; death-log count={}",
        r.log.iter().filter(|s| s.as_str() == "You died.").count());
    assert_eq!(r.log.iter().filter(|s| s.as_str() == "You died.").count(), 1);
    let before = storage::serialize(&r).unwrap();
    assert!(!engine::dispatch(&mut r, &Command::Wait).unwrap().changed);
    assert_eq!(storage::serialize(&r).unwrap(), before);
}

#[test]
fn nightmare_actual_kill_dissolves_without_corpse_loot_or_rewards() {
    let mut r = room(); sleep(&mut r);
    let id = r.next_id; r.next_id += 1.;
    let mut n = quest::nightmare(1, id, 9., 8.); n.hp = 1.; n.stats.dr = 0.;
    r.actors_at_mut(1.).push(n); r.player.conditions.insert("lucky".into(), 1.);
    let exp = r.exp; let inventory = r.inventory.clone(); let ground = r.ground.clone();
    engine::attack(&mut r, 0., id, 1.).unwrap();
    assert!(engine::actor(&r, id).is_none()); assert_eq!(r.exp, exp);
    assert_eq!(r.inventory, inventory); assert_eq!(r.ground, ground);
    assert_eq!(r.log.iter().filter(|s| s.contains("dissolves")).count(), 1);
}

#[test]
fn waking_fire_keeps_provenance_and_hits_waking_actor_not_overlapping_projection() {
    let mut r = room(); let victim = rat(&mut r, 14., 8.); let i = tile(&r, 14, 8);
    environment::ignite(&mut r, 14., 8., Some(10.), Some(0.)).unwrap();
    let before = r.floor().unwrap().environment.clone();
    let provenance = before.as_ref().unwrap().provenance.as_ref().unwrap()["fire"].clone();
    sleep(&mut r); r.player.x = 14.;
    assert_eq!(r.floor().unwrap().environment, before);
    assert!(dream::floor_for(&r, &r.player).unwrap().environment.is_missing());
    dream::tick_environments(&mut r, &[]).unwrap();
    assert_eq!(r.player.hp, 100., "waking fire cannot damage an overlapping projection");
    assert!(engine::actor(&r, victim).unwrap().hp < 100., "normal-plane fire still damages its normal-plane victim");
    let current = &r.floor().unwrap().environment.as_ref().unwrap().provenance.as_ref().unwrap()["fire"];
    assert_eq!(current.owner[i], provenance.owner[i]); assert_eq!(current.player[i], provenance.player[i]);
    assert_eq!(engine::condition(&r.player, "burning"), 0.);
    assert!(!dream::context_dream(&r)); assert!(!dream::environment_context(&r));
}

#[test]
fn dream_sparkle_real_dispatch_never_changes_waking_terrain_fire_or_lights() {
    let mut r = room(); r.player.abilities.push(abilities::SPARKLE.into());
    let i = tile(&r, 11, 8); r.floor_mut().unwrap().tiles[i] = "grass".into(); sleep(&mut r);
    let before_tiles = r.floor().unwrap().tiles.clone();
    let before_fire = r.floor().unwrap().environment.clone();
    let before_lights = r.floor().unwrap().light_sources.clone();
    let mut e = Engine::new(r).unwrap(); e.dispatch(&at(abilities::SPARKLE, 11., 8.)).unwrap();
    assert_eq!(e.run.floor().unwrap().tiles, before_tiles);
    assert_eq!(e.run.floor().unwrap().environment, before_fire);
    assert_eq!(e.run.floor().unwrap().light_sources, before_lights);
    let f = dream::floor_for(&e.run, &e.run.player).unwrap();
    assert_eq!(f.tiles[i], "floor");
    assert!(f.permanent_fire.as_ref().unwrap().contains(&(i as f64)));
    assert_eq!(f.environment.as_ref().unwrap().provenance.as_ref().unwrap()["fire"].player[i], 1.);
}

fn temporal_room() -> (Run, f64) {
    let mut r = room();
    r.player.abilities.extend([half_here::ID.into(), abilities::SPARKLE.into(),
        "numo-revert".into(), "morae-see-the-future".into()]);
    r.player.conditions.insert("revertCharges".into(), 5.);
    let clone = half_here::create(&mut r, 9., 8.).unwrap();
    sleep(&mut r);
    (r, clone)
}
#[test]
fn save_load_and_actual_revert_restore_both_layers_and_half_here_creation_plane() {
    let (r, clone) = temporal_room(); let mut e = Engine::new(r).unwrap();
    e.dispatch(&at(abilities::SPARKLE, 12., 8.)).unwrap();
    let baseline = e.run.clone();
    e.dispatch(&Command::Wait).unwrap();
    e.dispatch(&at(abilities::SPARKLE, 13., 8.)).unwrap();
    let changed = e.run.dreamwalking.clone();
    e = Engine::load(&e.save().unwrap()).unwrap();
    assert_eq!(e.run.dreamwalking, changed);
    assert!(!dream::dream_actor(&e.run, engine::actor(&e.run, clone).unwrap()));
    e.dispatch(&ability("numo-revert")).unwrap();
    assert_eq!(e.run.tick, baseline.tick);
    assert_eq!(e.run.dreamwalking, baseline.dreamwalking);
    assert_eq!(e.run.floor().unwrap(), baseline.floor().unwrap());
    assert!(!dream::dream_actor(&e.run, engine::actor(&e.run, clone).unwrap()));
    assert!(dream::active(&e.run)); assert!(!dream::context_dream(&e.run));
}

#[test]
fn actual_see_future_restores_dream_layer_and_half_here_plane_after_waking() {
    let (r, clone) = temporal_room(); let mut e = Engine::new(r).unwrap();
    e.dispatch(&at(abilities::SPARKLE, 12., 8.)).unwrap();
    let baseline = e.run.dreamwalking.clone(); let normal = e.run.floor().unwrap().clone();
    e.dispatch(&ability("morae-see-the-future")).unwrap();
    assert!(e.run.future.is_some());
    e.dispatch(&ability(dream::DREAMWALK_ID)).unwrap(); assert!(!dream::active(&e.run));
    e.dispatch(&Command::Wait).unwrap();
    e = Engine::load(&e.save().unwrap()).unwrap();
    e.dispatch(&ability("morae-see-the-future")).unwrap();
    assert!(dream::active(&e.run)); assert_eq!(e.run.dreamwalking, baseline);
    assert_eq!(e.run.floor().unwrap(), &normal);
    assert!(!dream::dream_actor(&e.run, engine::actor(&e.run, clone).unwrap()));
    assert!(e.run.future.is_none());
}

#[test]
fn fixture_replay_with_revert_preserves_both_layers_and_half_here_plane() {
    let (initial, clone) = temporal_room(); let mut expected = Engine::new(initial.clone()).unwrap();
    let commands = [at(abilities::SPARKLE, 12., 8.), Command::Wait,
        at(abilities::SPARKLE, 13., 8.), ability("numo-revert"),
        ability(dream::DREAMWALK_ID), ability(dream::DREAMWALK_ID)];
    let events: Vec<_> = commands.iter().map(|c| expected.dispatch(c).unwrap().events).collect();
    let mut replay = ReplaySession::from_initial(initial, ReplayRecord::from_run(&expected.run)).unwrap();
    for expected_events in events {
        assert_eq!(replay.step_with_outcome().unwrap().unwrap().events, expected_events);
    }
    assert!(replay.finished()); assert_eq!(replay.engine.save().unwrap(), expected.save().unwrap());
    assert!(!dream::dream_actor(&replay.engine.run, engine::actor(&replay.engine.run, clone).unwrap()));
    assert!(dream::active(&replay.engine.run));
    let i = tile(&replay.engine.run, 12, 8);
    let j = tile(&replay.engine.run, 13, 8);
    let floor = dream::floor_for(&replay.engine.run, &replay.engine.run.player).unwrap();
    assert!(floor.permanent_fire.as_ref().unwrap().contains(&(i as f64)));
    assert!(!floor.permanent_fire.as_ref().unwrap().contains(&(j as f64)));
    assert!(replay.engine.run.floor().unwrap().permanent_fire.is_missing());
}

#[test]
fn dream_created_half_here_remains_dream_side_after_real_wake_and_load() {
    let mut r = room(); r.player.abilities.push(half_here::ID.into()); sleep(&mut r);
    let nightmare = r.next_id; r.next_id += 1.;
    let mut a = quest::nightmare(1, nightmare, 30., 8.);
    a.state = "sleeping".into(); r.actors_at_mut(1.).push(a);
    let mut e = Engine::new(r).unwrap();
    e.dispatch(&at(half_here::ID, 9., 8.)).unwrap();
    let clone = half_here::clone_id(&e.run).unwrap();
    assert!(dream::dream_actor(&e.run, engine::actor(&e.run, clone).unwrap()));
    e.dispatch(&ability(dream::DREAMWALK_ID)).unwrap(); assert!(!dream::active(&e.run));
    e = Engine::load(&e.save().unwrap()).unwrap();
    let endpoint = engine::actor(&e.run, clone).unwrap();
    assert!(dream::dream_actor(&e.run, endpoint)); assert!(!dream::same_plane(&e.run, &e.run.player, endpoint));
    assert!(dream::dream_view(&e.run));
    assert!(corpse_core::vision::visible_actor(&e.run, engine::actor(&e.run, nightmare).unwrap()));
    assert!(dream::same_plane(&e.run, endpoint, engine::actor(&e.run, nightmare).unwrap()));
}

#[test]
fn staircase_bumps_cancel_coordinates_and_time_in_normal_and_haste_paths() {
    for stairs in ["up", "down", "pit"] { for haste in [false, true] {
        let mut r = room(); let i = tile(&r, 9, 8); r.floor_mut().unwrap().tiles[i] = stairs.into();
        if haste { r.player.conditions.insert("hasted".into(), 3.); }
        sleep(&mut r); let before = (r.player.x, r.player.y, r.depth, r.tick);
        let out = engine::dispatch(&mut r, &Command::Move { dx: 1., dy: 0. }).unwrap();
        assert!(!out.took_turn, "{stairs} haste={haste}");
        assert_eq!((r.player.x, r.player.y, r.depth, r.tick), before, "{stairs} haste={haste}");
        assert!(r.log.iter().any(|s| s == dream::STAIRS_MESSAGE));
        assert_eq!((dream::body(&r).unwrap().x, dream::body(&r).unwrap().y), (8., 8.));
    } }
}

#[test]
fn synthetic_dream_controls_cannot_bypass_terminal_choice_debug_or_paralysis_gates() {
    for mode in ["begin", "toggle-awake", "early-wake"] {
        for gate in ["dead", "won", "zero-hp", "offers", "debug-preparation", "paralyzed"] {
            let mut r = room();
            if mode == "begin" {
                r.dreamwalking = dream::State { unlocked: true, ..Default::default() }.into();
            } else {
                sleep(&mut r);
                if mode == "early-wake" { r.dreamwalking.as_mut().unwrap().unlocked = false; }
                r.player.x = 10.;
            }
            let id = if mode == "early-wake" { dream::WAKE_UP_ID } else { dream::DREAMWALK_ID };
            match gate {
                "dead" | "won" => r.status = gate.into(),
                "zero-hp" => { r.player.hp = 0.; dream::sync(&mut r); }
                "offers" => r.offers.push(abilities::ASSAULT.into()),
                "debug-preparation" => {
                    r.playtest = corpse_core::playtest::Profile {
                        version: 2, start_floor: 1, profile: corpse_core::debug_floor::PROFILE.into(),
                        prior_hearts: 0, prior_blessings: 0., sources: vec![], candidates: vec![],
                        debug: Some(corpse_core::debug_floor::State {
                            pending: Default::default(), consumed: 0, stash: vec![], entered: false,
                            loadout_revision: 0,
                        }),
                    }.into();
                }
                "paralyzed" => { r.player.conditions.insert("paralyzed".into(), 2.); }
                _ => unreachable!(),
            }
            let before = r.clone();
            assert_eq!(dream::ability(&mut r, id).unwrap(), Some(false), "{mode}/{gate}");
            assert_eq!(r, before, "direct synthetic action mutated {mode}/{gate}");
            let mut e = Engine::new(r).unwrap();
            let outcome = e.dispatch(&ability(id)).unwrap();
            assert!(!outcome.took_turn, "{mode}/{gate}");
            // Normal finish() may refresh remembered terrain even for a refused
            // free command; the authoritative toggle/endpoints must not change.
            let identity = |r: &Run| r.dreamwalking.as_ref().map(|s|
                (s.unlocked, s.active, s.depth, s.body_id, s.anchor_x, s.anchor_y,
                    s.context_dream, s.resolving_environment, s.floors.len()));
            assert_eq!(identity(&e.run), identity(&before), "{mode}/{gate}");
            assert_eq!((e.run.player.x, e.run.player.y, e.run.tick, e.run.next_id),
                (before.player.x, before.player.y, before.tick, before.next_id), "{mode}/{gate}");
            assert_eq!(e.run.player.energy, before.player.energy, "{mode}/{gate}");
            assert_eq!(e.run.offers, before.offers, "{mode}/{gate}");
            assert_eq!(e.run.playtest, before.playtest, "{mode}/{gate}");
            assert_eq!(e.run.actors_at(1.).iter().map(|a|a.id).collect::<Vec<_>>(),
                before.actors_at(1.).iter().map(|a|a.id).collect::<Vec<_>>(), "{mode}/{gate}");
            if gate == "zero-hp" { assert!(e.run.player.hp <= 0., "{mode}/{gate} revived the player"); }
        }
    }
}

#[test]
fn nested_dream_floor_geometry_environment_and_transient_context_corruption_is_rejected() {
    let mut r = room();
    let mut second = r.floor().unwrap().clone(); second.depth = 2.;
    r.floors.push(std::sync::Arc::new(second));
    r.actors.insert("2".into(), Default::default()); r.ground.insert("2".into(), Default::default());
    sleep(&mut r);
    dream::with_plane(&mut r, true, |r| environment::ignite(r, 11., 8., Some(10.), Some(0.))).unwrap();
    let valid = serde_json::to_value(&r).unwrap();
    storage::deserialize(&valid.to_string()).unwrap();
    let mut accepted = Vec::new();
    for case in ["duplicate-depth", "missing-waking-counterpart", "mismatched-geometry", "invalid-tile",
        "truncated-lights", "truncated-seen", "truncated-fire", "truncated-owner",
        "swapped-context", "resolving-context"] {
        let mut malformed = valid.clone();
        let state = &mut malformed["dreamwalking"];
        match case {
            "duplicate-depth" => {
                let duplicate = state["floors"][0].clone();
                state["floors"].as_array_mut().unwrap().push(duplicate);
            }
            "missing-waking-counterpart" => state["floors"][0]["depth"] = 3.into(),
            "mismatched-geometry" => { state["floors"][0]["tiles"].as_array_mut().unwrap().resize(2500, "floor".into()); }
            "invalid-tile" => state["floors"][0]["tiles"][0] = "invalid-terrain".into(),
            "truncated-lights" => { state["floors"][0]["lights"].as_array_mut().unwrap().pop(); }
            "truncated-seen" => { state["floors"][0]["seen"].as_array_mut().unwrap().pop(); }
            "truncated-fire" => { state["floors"][0]["environment"]["fire"].as_array_mut().unwrap().pop(); }
            "truncated-owner" => { state["floors"][0]["environment"]["provenance"]["fire"]["owner"].as_array_mut().unwrap().pop(); }
            "swapped-context" => state["context_dream"] = true.into(),
            "resolving-context" => state["resolving_environment"] = true.into(),
            _ => unreachable!(),
        }
        let raw = malformed.to_string();
        if storage::deserialize(&raw).is_ok() { accepted.push(format!("canonical/{case}")); }
        if Engine::load(&raw).is_ok() { accepted.push(format!("compatible/{case}")); }
    }
    assert!(accepted.is_empty(), "malformed nested dream saves accepted: {accepted:?}");
}

#[test]
fn report_eight_nightmare_numerical_profiles() {
    let r = room(); let mut rows = Vec::new();
    for band in 0..8 {
        let a = quest::nightmare(band*5+1, 2000.+band as f64, 10., 8.);
        let stats = corpse_core::combat::effective_stats(&r, &a);
        let base = 10. + stats.offense;
        assert!(!corpse_core::ai::is_hasted(&a));
        assert!(!a.abilities.iter().any(|id| matches!(id.as_str(), "enemy:beast:slow" | "enemy:pale:slow")));
        rows.push(serde_json::json!({
            "floors": format!("{}-{}",band*5+1,band*5+5), "name":a.name,
            "baseline":quest::NIGHTMARE_BASELINES[band as usize], "hp":a.max_hp,
            "offense":stats.offense, "defense":stats.defense, "raw_melee_base":base,
            "ordinary_melee_damage_vs_zero_dr": [
                corpse_core::combat::ordinary_strike_damage(base,0.75,0.,false,false),
                corpse_core::combat::ordinary_strike_damage(base,1.25-f64::EPSILON,0.,false,false)],
            "dr":stats.dr, "accuracy":stats.accuracy, "dodge":stats.dodge,
            "perception":stats.perception, "energy":stats.max_energy,
            "speed":"normal: one normal action per world turn; no innate haste or slow"
        }));
    }
    println!("NIGHTMARE_NUMERICAL_PROFILES={}",serde_json::to_string(&rows).unwrap());
}
