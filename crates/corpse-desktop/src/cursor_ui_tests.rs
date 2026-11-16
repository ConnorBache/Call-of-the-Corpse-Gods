use super::*;
use crate::{render::Painter, session_store::tracked_engine as temporal};
use corpse_core::{Command, Field, Run};

fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 52.0.into(); r.player.x = 10.; r.player.y = 10.;
    r.player.conditions.clear(); r.player.abilities.clear(); r.player.active.clear();
    r.player.stats.perception = 300.;
    r.player.conditions.insert("allSeeingReveal".into(), 1.);
    r.player.conditions.insert("allSeeingDepth".into(), r.depth);
    // The old room fixture contains legacy items rejected by revision-52 loading.
    r.inventory.clear();
    r.actors.clear(); r.ground.clear(); r.offers.clear(); r.history.clear(); r.replay.clear();
    r.future = None; r.haste_snapshot = Field::Missing; r.phase = "normal".to_string().into();
    r.player.hp = 100.; r.player.max_hp = 100.; r.player.base_max_hp = 100.0.into();
    for depth in r.floors.iter().map(|f| f.depth).collect::<Vec<_>>() { r.actors_at_mut(depth); r.ground_at_mut(depth); }
    let f = r.floor_mut().unwrap();
    f.tiles.fill("floor".into()); f.seen.fill(true); f.lights.fill(100.);
    f.remembered_tiles = Field::Missing; f.environment = Field::Missing;
    f.portals = Field::Missing; f.non_euclidean = Field::Missing;
    f.currents = Field::Missing; f.light_sources = Field::Missing; f.speed_up = Field::Missing;
    r
}
fn inspect(action: Action) -> (usize, usize) {
    let Action::Inspect(tile) = action else { panic!("Expected inspection, got {action:?}"); }; tile
}

#[test]
fn cursor_enter_directions_edges_repeat_and_escape_are_detached() {
    let r = room(); let engine = temporal::Engine::new(r).unwrap();
    let before = engine.save().unwrap(); let grid = crate::cursor_grid(&engine.run).unwrap();
    let mut cursor = Cursor::default();
    assert_eq!(cursor.key(Keycode::Return, Mod::NOMOD, true, grid, None), Action::Consumed);
    assert!(!cursor.active());
    let mut tile = inspect(cursor.key(Keycode::Return, Mod::NOMOD, false, grid, None));
    assert_eq!(tile, grid.player); assert!(cursor.active());
    assert_eq!(cursor.key(Keycode::Return, Mod::NOMOD, true, grid, Some(tile)), Action::Consumed);
    for (key, dx, dy) in [(Keycode::Kp8, 0, -1), (Keycode::Kp9, 1, -1),
        (Keycode::Kp6, 1, 0), (Keycode::Kp3, 1, 1), (Keycode::Kp2, 0, 1),
        (Keycode::Kp1, -1, 1), (Keycode::Kp4, -1, 0), (Keycode::Kp7, -1, -1)] {
        let next = inspect(cursor.key(key, Mod::NOMOD, true, grid, Some(tile)));
        assert_eq!(next, ((tile.0 as i32 + dx) as usize, (tile.1 as i32 + dy) as usize)); tile = next;
    }
    assert_eq!(tile, grid.player);
    for (key, at) in [(Keycode::Left, (0, 0)), (Keycode::Y, (0, 0)),
        (Keycode::Right, (grid.width - 1, grid.height - 1)),
        (Keycode::N, (grid.width - 1, grid.height - 1))] {
        assert_eq!(cursor.key(key, Mod::NOMOD, false, grid, Some(at)), Action::Inspect(at));
    }
    assert_eq!(cursor.key(Keycode::Escape, Mod::NOMOD, false, grid, Some(tile)), Action::Exit);
    assert!(!cursor.active());
    assert_eq!(cursor.key(Keycode::Escape, Mod::NOMOD, true, grid, None), Action::Consumed);
    assert_eq!(cursor.key(Keycode::Escape, Mod::NOMOD, false, grid, None), Action::Pass);
    assert_eq!(engine.save().unwrap(), before);
    assert_eq!(crate::key_command(Keycode::Kp6), Some(Command::Move { dx: 1., dy: 0. }));
}

#[test]
fn cursor_entry_prefers_valid_hover_and_world_replacement_clears_mode() {
    let grid = Grid { width: 20, height: 12, player: (5, 6) };
    for (hover, expected) in [(Some((8, 9)), (8, 9)), (None, (5, 6)), (Some((20, 12)), (5, 6))] {
        let mut c = Cursor::default();
        assert_eq!(c.key(Keycode::KpEnter, Mod::NOMOD, false, grid, hover), Action::Inspect(expected));
        assert_eq!(c.key(Keycode::KpEnter, Mod::NOMOD, false, grid, Some(expected)), Action::Confirm(expected));
        assert!(!c.active());
    }
    for key in [Keycode::F9, Keycode::R, Keycode::I, Keycode::M, Keycode::Num1, Keycode::X, Keycode::Z] {
        let mut c = Cursor::default(); c.key(Keycode::Return, Mod::NOMOD, false, grid, None);
        assert_eq!(c.key(key, Mod::NOMOD, false, grid, None), Action::Pass);
        assert!(!c.active(), "{key:?} must release cursor before its existing action");
    }
    let mut c = Cursor::default(); c.key(Keycode::Return, Mod::NOMOD, false, grid, None);
    assert!(c.clear(), "mouse or focus takeover releases cursor");
    assert!(!c.clear());
    assert_eq!(c.key(Keycode::Return, Mod::NOMOD, false, Grid { width: 0, ..grid }, None), Action::Consumed);
    assert!(!c.active());
}

#[test]
fn cursor_system_shortcuts_and_modal_admission_preserve_owners() {
    let grid = Grid { width: 20, height: 12, player: (5, 6) }; let mut c = Cursor::default();
    for modifier in [Mod::LALTMOD, Mod::LCTRLMOD, Mod::LGUIMOD] {
        assert_eq!(c.key(Keycode::Return, modifier, false, grid, None), Action::Pass);
        assert!(!c.active());
    }
    c.key(Keycode::Return, Mod::NOMOD, false, grid, None);
    assert_eq!(c.key(Keycode::Return, Mod::LALTMOD, false, grid, None), Action::Pass);
    assert!(c.active(), "fullscreen does not consume the inspected coordinate");
    let mut r = room();
    assert!(crate::cursor_context_available(crate::Screen::Game, false, Some(&r), false));
    assert!(!crate::cursor_context_available(crate::Screen::Game, true, Some(&r), false));
    assert!(!crate::cursor_context_available(crate::Screen::Game, false, Some(&r), true));
    for screen in [crate::Screen::Menu, crate::Screen::Start, crate::Screen::ImportSave,
        crate::Screen::Summary, crate::Screen::Manual, crate::Screen::Replays,
        crate::Screen::ReplayGame, crate::Screen::Scores, crate::Screen::Settings] {
        assert!(!crate::cursor_context_available(screen, false, Some(&r), false));
    }
    r.offers.push("numo-revert".into());
    assert!(!crate::cursor_context_available(crate::Screen::Game, false, Some(&r), false));
    r.offers.clear(); r.status = "dead".into();
    assert!(!crate::cursor_context_available(crate::Screen::Game, false, Some(&r), false));
    assert!(!crate::cursor_context_available(crate::Screen::Game, false, None, false));
}

#[test]
fn cursor_z_wait_matches_all_existing_aliases_across_action_phases_and_save_reload() {
    assert_eq!(rest_key(Keycode::Z, Mod::NOMOD, false), RestKey::Once);
    assert_eq!(rest_key(Keycode::Z, Mod::LSHIFTMOD, false), RestKey::UntilChange);
    for modifiers in [Mod::NOMOD, Mod::LSHIFTMOD, Mod::RSHIFTMOD] {
        assert_eq!(rest_key(Keycode::Z, modifiers, true), RestKey::None);
    }
    for modifiers in [Mod::LCTRLMOD, Mod::LALTMOD, Mod::LGUIMOD, Mod::LCTRLMOD | Mod::LSHIFTMOD] {
        assert_eq!(rest_key(Keycode::Z, modifiers, false), RestKey::None);
    }
    use corpse_core::model::FastActionPhase;
    for (phase, fast) in [("normal", None), ("haste", None),
        ("normal", Some(FastActionPhase::Haste)), ("normal", Some(FastActionPhase::Normal))] {
        let mut r = room(); r.phase = phase.to_string().into();
        r.player.conditions.insert("hasted".into(), 5.);
        if let Some(fast) = fast { r.player.abilities.push("enemy:beast:fast".into());
            r.fast_action_phase = fast.into(); }
        let mut expected = temporal::Engine::new(r.clone()).unwrap();
        crate::dispatch(&mut expected, &mut crate::effects::Effects::default(), 0., &Command::Wait).unwrap();
        for key in [Keycode::Z, Keycode::Space, Keycode::Period, Keycode::Kp5] {
            let command = crate::key_command(key).unwrap(); assert_eq!(command, Command::Wait);
            let mut actual = temporal::Engine::new(r.clone()).unwrap();
            crate::dispatch(&mut actual, &mut crate::effects::Effects::default(), 0., &command).unwrap();
            assert_eq!(actual.save().unwrap(), expected.save().unwrap(), "{phase}, {fast:?}, {key:?}");
            assert_eq!(temporal::Engine::load(&actual.save().unwrap()).unwrap().save().unwrap(), actual.save().unwrap());
        }
    }
}

#[test]
fn cursor_enter_click_geometry_tracks_resize_and_rejects_out_of_bounds() {
    let r = room(); let grid = crate::cursor_grid(&r).unwrap(); let mut p = Painter::new();
    for (w, h) in [(1280, 800), (1537, 911), (1920, 1080), (900, 1200)] {
        p.resize(w, h); p.set_responsive_canvas(true);
        for tile in [(0, 0), grid.player, (grid.width - 1, grid.height - 1)] {
            let sdl2::event::Event::MouseButtonDown { x, y, timestamp, window_id, .. } =
                crate::cursor_click(&p, grid, tile, 19, 23).unwrap() else { panic!(); };
            assert_eq!((timestamp, window_id), (19, 23));
            assert_eq!(crate::pointer_tile_dimensions(&p, grid.width, grid.height, x, y), Some(tile));
        }
        assert!(crate::cursor_click(&p, grid, (grid.width, 0), 0, 0).is_none());
        assert!(crate::cursor_click(&p, grid, (0, grid.height), 0, 0).is_none());
    }
}

#[test]
fn cursor_unknown_hover_never_reads_hidden_terrain_items_or_creatures() {
    let mut r = room(); r.player.conditions.clear(); r.player.stats.perception = 0.;
    let f = r.floor_mut().unwrap(); f.seen.fill(false); f.lights.fill(-100.);
    let tile = (f.width() - 2, f.height() - 2);
    assert!(!corpse_core::vision::visible(&r, tile.0, tile.1));
    let before = serde_json::to_vec(&r).unwrap(); let mut c = Cursor::default();
    let grid = crate::cursor_grid(&r).unwrap();
    assert_eq!(c.key(Keycode::Return, Mod::NOMOD, false, grid, Some(tile)), Action::Inspect(tile));
    let view = crate::observe::observe(&r);
    let report = crate::hover_tile_report(&r, tile.0, tile.1, None, view.terrain_snapshot.as_deref());
    let path = crate::travel_ui::path_to(&r, tile.0, tile.1, true);
    assert!(path.is_empty());
    assert!(matches!(crate::travel_ui::start(&r, tile.0, tile.1), crate::travel_ui::Start::NoPath));
    assert_eq!(serde_json::to_vec(&r).unwrap(), before);
    let mut hidden = r.player.clone(); hidden.id = 999.; hidden.x = tile.0 as f64; hidden.y = tile.1 as f64;
    hidden.name = "SECRET CREATURE".into(); hidden.hp = 9876.; hidden.state = "hunting".into();
    r.actors_at_mut(r.depth).push(hidden);
    let f = r.floor_mut().unwrap(); let at = tile.1 * f.width() + tile.0; f.tiles[at] = "water".into();
    r.ground_at_mut(r.depth).push(corpse_core::GroundItem { x: tile.0 as f64, y: tile.1 as f64,
        item: corpse_core::Item { id: 998., kind: "potion".into(), name: "SECRET ITEM".into(), ..Default::default() }, ..Default::default() });
    let view = crate::observe::observe(&r);
    assert_eq!(crate::hover_tile_report(&r, tile.0, tile.1, None, view.terrain_snapshot.as_deref()), report);
    assert!(crate::travel_ui::path_to(&r, tile.0, tile.1, true).is_empty());
    assert_eq!(c.key(Keycode::Return, Mod::NOMOD, false, grid, Some(tile)), Action::Confirm(tile));
    assert!(matches!(crate::travel_ui::start(&r, tile.0, tile.1), crate::travel_ui::Start::NoPath));
}

#[test]
fn cursor_confirm_known_travel_and_hazard_use_identical_mouse_admission() {
    let mut r = room(); let mut p = Painter::new(); p.set_responsive_canvas(true);
    let grid = crate::cursor_grid(&r).unwrap();
    let classify = |r: &Run, tile: (usize, usize)| match crate::travel_ui::start(r, tile.0, tile.1) {
        crate::travel_ui::Start::Direct(command) => (Some(command), vec![]),
        crate::travel_ui::Start::Walk(walk) => (None, walk.path().iter().map(|s| (s.x, s.y)).collect()),
        crate::travel_ui::Start::NoPath => (None, vec![]),
    };
    for tile in [(11, 10), (15, 10), (10, 10)] {
        let mut c = Cursor::default(); c.key(Keycode::Return, Mod::NOMOD, false, grid, Some(tile));
        assert_eq!(c.key(Keycode::Return, Mod::NOMOD, false, grid, Some(tile)), Action::Confirm(tile));
        let sdl2::event::Event::MouseButtonDown { x, y, .. } = crate::cursor_click(&p, grid, tile, 0, 0).unwrap() else { panic!(); };
        let clicked = crate::pointer_tile_dimensions(&p, grid.width, grid.height, x, y).unwrap();
        assert_eq!(classify(&r, tile), classify(&r, clicked));
    }
    let f = r.floor_mut().unwrap(); let at = 10 * f.width() + 11; f.tiles[at] = "chasm".into();
    let (Some(command), _) = classify(&r, (11, 10)) else { panic!("adjacent hazard uses direct admission"); };
    assert!(crate::hazards_ui::intentional(&r, &command).is_some());
}

#[test]
fn cursor_dream_plane_reports_match_existing_pointer_without_run_mutation() {
    let mut r = room(); let mut body = r.player.clone(); body.id = 901.;
    body.conditions.clear(); body.conditions.insert("phantasmiiBody".into(), 1.);
    body.conditions.insert("halfHereOwner".into(), r.player.id); r.actors_at_mut(r.depth).push(body);
    r.dreamwalking = corpse_core::dreamwalking::State { active: true, depth: r.depth,
        body_id: Some(901.), anchor_x: r.player.x, anchor_y: r.player.y, ..Default::default() }.into();
    corpse_core::dreamwalking::ensure_layer(&mut r);
    let before = serde_json::to_vec(&r).unwrap(); let grid = crate::cursor_grid(&r).unwrap();
    let f = corpse_core::dreamwalking::floor_in_plane(&r, true).unwrap();
    assert_eq!((grid.width, grid.height), (f.width(), f.height()));
    let mut c = Cursor::default(); let at = inspect(c.key(Keycode::Return, Mod::NOMOD, false, grid, None));
    let tile = inspect(c.key(Keycode::Right, Mod::NOMOD, false, grid, Some(at)));
    let view = crate::observe::observe(&r);
    assert_eq!(crate::hover_tile_report(&r, tile.0, tile.1, None, view.terrain_snapshot.as_deref()),
        crate::inspection_ui::observe_tile(&r, tile.0, tile.1));
    assert_eq!(serde_json::to_vec(&r).unwrap(), before);
}

#[test]
#[ignore = "Actual SDL event loop/readback: SDL_VIDEODRIVER=offscreen, --test-threads=1; not OS/Windows input"]
fn cursor_native_sdl_production_event_loop_and_captures() {
    let root = std::env::var_os("CORPSE_CURSOR_REVIEW").map(std::path::PathBuf::from)
        .unwrap_or_else(||std::env::temp_dir().join(format!("corpse-cursor-{}",std::process::id())));
    for software in [true,false] {
        crate::input_loop_tests::native_input_loop(room(),root.join(if software {"software"} else {"sdl-accelerated"}),software);
    }
}

#[test]
fn cursor_render_keeps_observed_footer_and_does_not_mutate_save() {
    let engine = temporal::Engine::new(room()).unwrap(); let before = engine.save().unwrap();
    let grid = crate::cursor_grid(&engine.run).unwrap(); let mut c = Cursor::default();
    let tile = inspect(c.key(Keycode::Return, Mod::NOMOD, false, grid, Some((11,10))));
    let mut view = crate::observe::observe(&engine.run); view.hover = Some(tile);
    let report = crate::hover_tile_report(&engine.run,tile.0,tile.1,None,view.terrain_snapshot.as_deref());
    assert!(!report.scene.is_empty());
    let footer = crate::inspection_ui::footer_text("",Some(&report));
    assert_eq!(footer,report.scene);
    let mut with_text = Painter::new(); let mut blank = Painter::new();
    with_text.game_at(&view,footer,false,0.); blank.game_at(&view,"",false,0.);
    let y = with_text.inspection_notice_y() as usize;
    let from = y * with_text.width * 4; let to = (y+24).min(with_text.height) * with_text.width * 4;
    assert_ne!(&with_text.pixels[from..to],&blank.pixels[from..to],"observed terrain text must be visibly painted");
    assert_eq!(crate::inspection_ui::footer_text("Save failed.",Some(&report)),"Save failed.");
    assert_eq!(engine.save().unwrap(),before);
}
