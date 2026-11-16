//! Production input/admission/dispatch tests with actual native pixel captures.
//! No OS event-delivery or Windows platform claim is made by these CPU tests.
use super::*;
use crate::{effects, observe, pack, render::Painter, session_store::tracked_engine as temporal};
use corpse_core::{Actor, Field, Item, Stats, revision50_amulet_effects::Effect};
use sdl2::keyboard::Mod;

fn room(kind: Effect) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 52.0.into();
    r.player.x = 10.; r.player.y = 10.; r.player.energy = 200.;
    r.player.hp = 100.; r.player.max_hp = 100.; r.player.base_max_hp = 100.0.into();
    r.player.stats = Stats { max_energy: 200., perception: 300., ..Default::default() };
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.ability_reserves = Field::Missing; r.player.amulet_ledger = Field::Missing;
    r.player.equipment_abilities = Field::Missing; r.player.ability_order = Field::Missing;
    r.player.conditions.insert("allSeeingReveal".into(), 1.);
    r.player.conditions.insert("allSeeingDepth".into(), r.depth);
    r.inventory = vec![amulets::make_amulet_for_revision(kind, 900., 52.).unwrap()];
    r.actors.clear(); r.ground.clear(); r.offers.clear(); r.next_id = 1000.;
    for depth in r.floors.iter().map(|floor| floor.depth).collect::<Vec<_>>() {
        r.actors_at_mut(depth); r.ground_at_mut(depth);
    }
    r.history.clear(); r.replay.clear(); r.haste_snapshot = Field::Missing; r.future = None;
    r.phase = "normal".to_string().into();
    let f = r.floor_mut().unwrap();
    f.tiles.fill("floor".into()); f.lights.fill(100.); f.seen.fill(true);
    f.remembered_tiles = Field::Missing; f.environment = Field::Missing;
    f.light_sources = Field::Missing; f.doors = Field::Missing; f.portals = Field::Missing;
    f.non_euclidean = Field::Missing; f.speed_up = Field::Missing;
    f.godbeast_event = Field::Missing; f.currents = Field::Missing;
    f.roots = Field::Missing; f.mud = Field::Missing; f.slime = Field::Missing;
    let width = f.width(); f.tiles[10 * width + 25] = "wall".into();
    r
}
fn capture(p: &Painter, name: &str) {
    if let Ok(dir) = std::env::var("CORPSE_DIRECTIONAL_ZAP_REVIEW") {
        std::fs::create_dir_all(&dir).unwrap();
        let f = std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
        let mut enc = png::Encoder::new(f, p.width as u32, p.height as u32);
        enc.set_color(png::ColorType::Rgba); enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(&p.pixels).unwrap();
    }
}
struct Cast {
    took_turn: bool,
    changed: bool,
    feedback: effects::Effects,
}
fn dispatch_aim(engine: &mut temporal::Engine, target: &PackTarget, tile: (usize, usize)) -> Cast {
    let view = observe::observe(&engine.run);
    let command = command(&engine.run, &view, target, tile).unwrap();
    let mut feedback = effects::Effects::default();
    let out = crate::dispatch(engine, &mut feedback, 0., &command).unwrap();
    // The production wrapper transfers event ownership into Effects. Inspect
    // those actual displayed receipts, not the deliberately emptied out.events.
    assert!(out.events.is_empty());
    Cast { took_turn: out.took_turn, changed: out.changed, feedback }
}

#[test]
fn normal_first_direction_ignores_stale_mouse_then_cursor_moves_and_cancel_is_free() {
    let r = room(Effect::Firebolt); let before = r.clone(); let target = PackTarget::UseAt(900.);
    // This is the reproduced old behavior, not the desired eastward direction.
    assert_eq!(crate::aim_keyboard_tile(&r, Some((30, 4)), Keycode::Right), Some((31, 4)));
    for (key, expected) in [(Keycode::Right, (11, 10)), (Keycode::Y, (9, 9)),
        (Keycode::Kp3, (11, 11)), (Keycode::Up, (10, 9)), (Keycode::Left, (9, 10)),
        (Keycode::Down, (10, 11)), (Keycode::U, (11, 9)), (Keycode::B, (9, 11))] {
        assert_eq!(keyboard_tile(&r, Some(&target), Some((30, 4)), false, key), Some(expected));
        assert_eq!(keyboard_tile(&r, Some(&target), None, false, key), Some(expected));
    }
    assert_eq!(keyboard_tile(&r, Some(&target), Some((11, 10)), true, Keycode::Right), Some((12, 10)));
    assert_eq!(keyboard_tile(&r, Some(&target), Some((11, 10)), true, Keycode::Return), None);
    let mut view = observe::observe(&r); let mut pending = Some(target);
    crate::sync_pointer_view(&mut view, &r, Some((11, 10)), vec![], pending.as_ref());
    let (mut ability, mut attacker, mut keyboard) = (None, None, true);
    assert!(crate::cancel_aim_for_context(true, &mut pending, &mut ability, &mut attacker, &mut keyboard));
    crate::sync_pointer_view(&mut view, &r, None, vec![], pending.as_ref());
    assert!(view.target_path.is_empty() && view.target_area.is_empty() && view.target_impact.is_none());
    assert_eq!(r, before);
    let mut unavailable = observe::observe(&r);
    unavailable.cells.retain(|cell| (cell.x, cell.y) != (11, 10));
    assert_eq!(command(&r, &unavailable, &PackTarget::UseAt(900.), (11, 10)), Err("Select a known map location."));
    assert_eq!(r, before);
    let mut edge = r.clone(); edge.player.x = 0.; edge.player.y = 0.;
    let tile = keyboard_tile(&edge, Some(&PackTarget::UseAt(900.)), Some((30, 4)), false, Keycode::Left).unwrap();
    assert_eq!(tile, (0, 0));
    assert!(command(&edge, &observe::observe(&edge), &PackTarget::UseAt(900.), tile).is_err());
    assert!(crate::aim_modifiers_allowed(Mod::NOMOD));
    for modifiers in [Mod::LALTMOD, Mod::LCTRLMOD, Mod::LGUIMOD] {
        assert!(!crate::aim_modifiers_allowed(modifiers), "system shortcuts remain unhandled");
    }
}

#[test]
fn actual_pack_mouse_and_normal_keyboard_cast_empty_rays_identically_and_save_reload() {
    for kind in [Effect::Lightning, Effect::Firebolt, Effect::Poison, Effect::Acid] {
        let r = room(kind); let target = PackTarget::UseAt(900.);
        let mut keyboard = temporal::Engine::new(r.clone()).unwrap();
        let mut mouse = temporal::Engine::new(r).unwrap();
        let mut painter = Painter::new(); let mut pack_view = pack::PackView::observe(&keyboard.run);
        pack_view.configure(&painter, painter.frame().map.tuple());
        pack_view.key(Keycode::Down, Mod::NOMOD);
        for _ in 0..pack_view.keyboard_tabs_for_action_label("Apply").unwrap() {
            pack_view.key(Keycode::Tab, Mod::NOMOD);
        }
        let action = pack_view.key(Keycode::Return, Mod::NOMOD);
        assert_eq!(action, pack::PackAction::Target(target.clone()));
        let mut open = true; let mut pending = None; let mut notice = String::new();
        crate::apply_pack_action(action, &mut pack_view, &mut keyboard, &mut effects::Effects::default(),
            0., &mut open, &mut pending, &mut notice);
        assert!(!open && pending == Some(target.clone()));
        let before = keyboard.save().unwrap();
        let next = keyboard_tile(&keyboard.run, pending.as_ref(), Some((30, 4)), false, Keycode::Right).unwrap();
        assert_eq!(next, (11, 10));
        let mut preview = observe::observe(&keyboard.run);
        assert!(preview.cells.iter().any(|cell| cell.x == 11 && cell.y == 10 && cell.actor_id.is_none()));
        crate::sync_pointer_view(&mut preview, &keyboard.run, Some(next), vec![], pending.as_ref());
        assert!(preview.target_path.contains(&(11, 10)));
        assert_eq!(keyboard.save().unwrap(), before, "aiming must not spend resources or RNG");
        if kind == Effect::Firebolt {
            painter.game_at(&preview, &notice, false, 0.); capture(&painter, "empty-east-before-zap");
        }
        let outcome = dispatch_aim(&mut keyboard, &target, next);
        assert!(outcome.took_turn);
        let mouse_outcome = dispatch_aim(&mut mouse, &target, (11, 10));
        assert_eq!(outcome.took_turn, mouse_outcome.took_turn);
        assert_eq!(outcome.changed, mouse_outcome.changed);
        assert_eq!(outcome.feedback.cues.iter().map(|cue| (&cue.event, cue.started, cue.held, cue.static_marks)).collect::<Vec<_>>(),
            mouse_outcome.feedback.cues.iter().map(|cue| (&cue.event, cue.started, cue.held, cue.static_marks)).collect::<Vec<_>>());
        assert_eq!(keyboard.save().unwrap(), mouse.save().unwrap());
        assert_eq!(keyboard.run.inventory[0].amulet_state.as_ref().unwrap().energy, 0.);
        let loaded = temporal::Engine::load(&keyboard.save().unwrap()).unwrap();
        assert_eq!(loaded.save().unwrap(), keyboard.save().unwrap());
        assert!(outcome.feedback.cues.iter().filter_map(|cue| cue.event.animation.as_ref())
            .any(|cue| cue.proposal == corpse_core::animation_amulets::key(kind)
                && cue.stage == corpse_core::events::Stage::Travel && cue.cells.len() > 1));
        if kind == Effect::Firebolt {
            let view = observe::observe(&keyboard.run);
            painter.game_at(&view, "", false, 0.); painter.effects(&view, &outcome.feedback.cues, 50., false);
            capture(&painter, "empty-east-launched");
        }
    }
}

#[test]
fn endpoint_creature_and_legacy_items_keep_existing_target_contracts() {
    let mut r = room(Effect::Firebolt); let target = PackTarget::UseAt(900.);
    for kind in [Effect::Iceblast, Effect::Blinking] {
        r.inventory[0] = amulets::make_amulet_for_revision(kind, 900., 52.).unwrap();
        assert!(!directional(&r, &target));
        assert_eq!(keyboard_tile(&r, Some(&target), Some((17, 10)), false, Keycode::Right), Some((18, 10)));
        assert!(command(&r, &observe::observe(&r), &target, (18, 10)).is_ok());
    }
    for effect in ["beckoning", "discord", "roots", "teleportation"] {
        let mut item = Item { id: 900., kind: "amulet".into(), effect: effect.to_string().into(), ..Default::default() };
        corpse_core::revision50_amulets::initialize_item_for_revision(&mut item, 52.).unwrap();
        r.inventory[0] = item; assert!(!directional(&r, &target));
        let before = r.clone();
        assert!(command(&r, &observe::observe(&r), &target, (11, 10)).is_err(), "{effect}");
        assert!(amulets::admission(&r, &target.command(11., 10.)).unwrap().is_err());
        assert_eq!(r, before);
    }
    for revision in [50., 51.] {
        r = room(Effect::Firebolt); r.simulation_revision = revision.into();
        assert!(!directional(&r, &target));
        assert_eq!(keyboard_tile(&r, Some(&target), Some((30, 4)), false, Keycode::Right), Some((31, 4)));
    }
    assert!(!directional(&r, &PackTarget::Throw(900.)));
    assert!(!directional(&r, &PackTarget::Shoot(900.)));
}

#[test]
fn empty_direction_still_respects_terrain_and_acid_range() {
    // The R52 command boundary changes Firebolt only: combustible blockers
    // open during travel; old replay prefixes still trace before burning.
    for legacy in [false, true] {
        for blocker in ["wall", "tree", "door", "door-on-wall", "Borge", "Epidite"] {
            let mut r = room(Effect::Firebolt); let width = r.floor().unwrap().width();
            r.firebolt_travel_from = if legacy { Field::Null } else { 0usize.into() };
            match blocker {
                "wall" | "tree" => r.floor_mut().unwrap().tiles[10 * width + 14] = blocker.into(),
                "door" | "door-on-wall" => {
                    if blocker == "door-on-wall" { r.floor_mut().unwrap().tiles[10 * width + 14] = "wall".into(); }
                    let doors = r.floor_mut().unwrap().doors.get_or_insert_with(Default::default);
                    doors.insert((10 * width + 14).to_string(), corpse_core::FloorDoorsEntry { open: false, ..Default::default() });
                }
                kind => {
                    // Keep generated, admitted godbeast-room metadata rather
                    // than weakening the fixture into a fabricated solid flag.
                    let event = corpse_core::generation::reserve_godbeast_room_revision(
                        &mut r.floor_mut().unwrap().tiles, kind, &mut || 0.5, &[], &[], &Default::default(), 52.).unwrap();
                    r.player.x = event.x - 5.; r.player.y = event.y;
                    r.floor_mut().unwrap().godbeast_event = event.into();
                },
            }
            let tile = (r.player.x as usize + 1, r.player.y as usize);
            let blocker_x = r.floor().unwrap().godbeast_event.as_ref()
                .map_or(14., |event| event.x - if blocker == "Borge" { 2. } else { 0. });
            let burns_open = !legacy && matches!(blocker, "tree" | "door");
            let last_allowed = if burns_open { 25. } else { blocker_x };
            let target = PackTarget::UseAt(900.); let p = target.aim_preview(&r, Some(tile));
            assert_eq!(p.delivery.contains(&(blocker_x as usize, tile.1)), burns_open, "{blocker}, legacy={legacy}");
            assert!(!p.delivery.contains(&(last_allowed as usize, tile.1)), "remaining blocker: {blocker}, legacy={legacy}");
            assert_eq!(p.impact, Some((last_allowed as usize, tile.1)), "{blocker}, legacy={legacy}");
            let mut engine = temporal::Engine::new(r).unwrap();
            let out = dispatch_aim(&mut engine, &target, tile);
            let cue = out.feedback.cues.iter().filter_map(|cue| cue.event.animation.as_ref())
                .find(|a| a.proposal == "A02" && a.stage == corpse_core::events::Stage::Travel).unwrap();
            assert_eq!(cue.cells.last().unwrap().x, last_allowed, "{blocker}, legacy={legacy}");
            assert!(cue.cells.iter().all(|p| p.x <= last_allowed), "{blocker}, legacy={legacy}: {:?}", cue.cells);
            let floor = engine.run.floor().unwrap();
            if blocker == "tree" { assert_eq!(floor.tiles[10 * width + 14], "floor"); }
            if matches!(blocker, "door" | "door-on-wall") {
                assert!(!floor.doors.as_ref().unwrap().contains_key(&(10 * width + 14).to_string()));
            }
            if blocker == "door-on-wall" { assert_eq!(floor.tiles[10 * width + 14], "wall"); }
        }
    }
    let r = room(Effect::Acid); let target = PackTarget::UseAt(900.);
    assert_eq!(target.aim_preview(&r, Some((11, 10))).delivery.last(), Some(&(18, 10)));
    let mut engine = temporal::Engine::new(r).unwrap();
    let out = dispatch_aim(&mut engine, &target, (11, 10));
    let cue = out.feedback.cues.iter().filter_map(|cue| cue.event.animation.as_ref())
        .find(|a| a.proposal == "A05" && a.stage == corpse_core::events::Stage::Travel).unwrap();
    assert_eq!(cue.cells.last().unwrap().x, 18.);
}
fn actor(id: f64, x: f64) -> Actor {
    let mut actor = corpse_core::population::create_actor_revision("rat", id, x, 10., false, 52.).unwrap();
    actor.hp = 500.; actor.max_hp = 500.; actor.base_max_hp = 500.0.into();
    actor.conditions.insert("paralyzed".into(), 100.); actor
}
#[test]
fn hidden_creatures_unknown_terrain_and_other_planes_do_not_leak_into_empty_aim() {
    let mut r = room(Effect::Firebolt);
    r.player.conditions.shift_remove("allSeeingReveal"); r.player.conditions.shift_remove("allSeeingDepth");
    let target = PackTarget::UseAt(900.); let before = target.aim_preview(&r, Some((11, 10)));
    let mut hidden = actor(910., 14.); hidden.conditions.insert("invisible".into(), 20.);
    r.actors_at_mut(r.depth).push(hidden);
    assert!(!corpse_core::vision::visible_actor(&r, &r.actors_at(r.depth)[0]));
    assert_eq!(target.aim_preview(&r, Some((11, 10))), before);
    assert!(command(&r, &observe::observe(&r), &target, (11, 10)).is_ok());
    let mut engine = temporal::Engine::new(r.clone()).unwrap();
    let out = dispatch_aim(&mut engine, &target, (11, 10));
    assert!(out.took_turn); assert!(corpse_core::engine::actor(&engine.run, 910.).unwrap().hp < 500.);
    assert_eq!(engine.run.inventory[0].amulet_state.as_ref().unwrap().energy, 0.);
    // A creature on another plane is not an interceptor, even when coordinates match.
    r.actors_at_mut(r.depth)[0].conditions.insert("phantasmiiNightmare".into(), 1.);
    assert_eq!(target.aim_preview(&r, Some((11, 10))), before);
    let mut engine = temporal::Engine::new(r).unwrap(); dispatch_aim(&mut engine, &target, (11, 10));
    assert_eq!(corpse_core::engine::actor(&engine.run, 910.).unwrap().hp, 500.);
    // Unseen geometry cannot change the disclosed prefix or become an admission probe.
    let mut dark = room(Effect::Firebolt); dark.player.conditions.clear(); dark.player.stats.perception = -1000.;
    let f = dark.floor_mut().unwrap(); f.lights.fill(-100.); let width = f.width();
    let preview = target.aim_preview(&dark, Some((11, 10)));
    let reason = target.target_blocker(&dark, 11., 10.);
    dark.floor_mut().unwrap().tiles[10 * width + 20] = "wall".into();
    dark.actors_at_mut(dark.depth).push(actor(911., 19.));
    assert_eq!(target.aim_preview(&dark, Some((11, 10))), preview);
    assert_eq!(target.target_blocker(&dark, 11., 10.), reason);
}

#[test]
fn empty_direction_uses_existing_haste_spellslinger_and_future_accounting() {
    for phase in ["normal", "haste"] {
        let mut r = room(Effect::Poison); r.phase = phase.to_string().into();
        r.player.conditions.insert("hasted".into(), 10.);
        let mut engine = temporal::Engine::new(r).unwrap(); let tick = engine.run.tick;
        let out = dispatch_aim(&mut engine, &PackTarget::UseAt(900.), (11, 10));
        assert_eq!(out.took_turn, phase == "normal");
        assert_eq!(engine.run.inventory[0].amulet_state.as_ref().unwrap().energy, 0.);
        assert_eq!(engine.run.tick, tick + if phase == "normal" { 1. } else { 0. });
    }
    let mut r = room(Effect::Firebolt); r.player.abilities.push("isheilah-spellslinger".into());
    let mut engine = temporal::Engine::new(r).unwrap(); let tick = engine.run.tick;
    let target = PackTarget::UseAt(900.); let first = dispatch_aim(&mut engine, &target, (11, 10));
    assert!(!first.took_turn); assert_eq!(engine.run.tick, tick);
    let second = dispatch_aim(&mut engine, &target, (11, 10));
    assert!(second.took_turn); assert_eq!(engine.run.tick, tick + 1.);
    for appropriate in [false, true] {
        let mut r = room(Effect::Poison);
        r.player.abilities.push("morae-see-the-future".into());
        if appropriate { r.player.abilities.push("kative-appropriate".into()); }
        corpse_core::revision50_ability_energy::acquire_for_revision(&mut r.player,
            &["morae-see-the-future".into()], 52.).unwrap();
        let mut engine = temporal::Engine::new(r).unwrap();
        let future = Command::Ability { id: "morae-see-the-future".into(), target: Field::Missing, x: Field::Missing, y: Field::Missing };
        crate::dispatch(&mut engine, &mut effects::Effects::default(), 0., &future).unwrap();
        assert!(engine.run.future.is_some()); assert_eq!(engine.run.player.energy, 0.);
        let initial_item = engine.run.inventory[0].clone(); let initial_rng = engine.run.rng;
        let cast = dispatch_aim(&mut engine, &PackTarget::UseAt(900.), (11, 10));
        // Future drains the main bar. Only Appropriate's authored 15 -> 5 cost
        // makes this shot affordable from the ten-point amulet reserve alone.
        assert_eq!(cast.took_turn, appropriate);
        if appropriate { assert!(cast.changed); }
        assert_eq!(engine.run.inventory[0].amulet_state.as_ref().unwrap().energy, if appropriate { 5. } else { 10. });
        crate::dispatch(&mut engine, &mut effects::Effects::default(), 0., &future).unwrap();
        assert!(engine.run.future.is_none());
        assert_eq!(engine.run.inventory[0], initial_item); assert_eq!(engine.run.rng, initial_rng);
    }
}
