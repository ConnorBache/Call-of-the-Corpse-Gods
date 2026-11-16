//! Reconstructed R52 Floods native CPU regression coverage from the release docs.
//! Public command/observation paths drive every scene. Not byte-exact recovery.
use crate::{appearance_profile::Profile, effects, observe, pack::PackTarget,
    render::{Painter, RetainedGame, View, damage::Damage, tile_damage::TilePlanner}, tile_overlay};
use corpse_core::{Command, Field, Floor, Item, Run, events::{PresentationEvent, Stage}};
use std::sync::Arc;

fn room(revision: f64) -> Run {
    let mut run: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    run.simulation_revision = revision.into(); run.status = "playing".into(); run.tick = 0.;
    run.player.x = 7.; run.player.y = 11.; run.player.hp = run.player.max_hp;
    run.player.energy = 500.; run.player.stats.max_energy = 500.;
    run.player.abilities.clear(); run.player.active.clear(); run.player.conditions.clear();
    run.player.ability_order = Field::Missing; run.player.equipment_abilities = Field::Missing;
    run.player.conditions.insert("allSeeingReveal".into(), 1.);
    run.player.conditions.insert("allSeeingDepth".into(), run.depth);
    run.player.stats.perception = 100.;
    run.inventory.clear(); run.actors.clear(); run.ground.clear(); run.history.clear();
    run.offers.clear(); run.log.clear(); run.log_kinds = Field::Missing; run.dreamwalking = Field::Missing;
    let size = 23;
    let mut tiles = vec!["floor".to_owned(); size * size];
    for y in 0..size { for x in 0..size {
        if x == 0 || y == 0 || x == size-1 || y == size-1 { tiles[y*size+x] = "wall".into(); }
    }}
    run.floors = vec![Arc::new(Floor { depth: run.depth, domain: "Dungeon".into(),
        width: (size as f64).into(), height: (size as f64).into(),
        remembered_tiles: tiles.clone().into(), tiles, lights: vec![100.; size*size],
        seen: vec![true; size*size], water_blood: vec![0.; size*size].into(),
        substrates: vec!["rock".into(); size*size].into(), ..Default::default() })];
    run.actors.insert(run.depth.to_string(), Arc::new(Vec::new()));
    run.ground.insert(run.depth.to_string(), Arc::new(Vec::new()));
    let mut item = Item { id: 900., kind: "amulet".into(), effect: "floods".to_string().into(),
        glyph: "\"".into(), name: "Amulet of Floods".into(), ..Default::default() };
    corpse_core::revision50_amulets::initialize_item_for_revision(&mut item,revision).unwrap();
    if revision == 52. { corpse_core::revision52_amulet_tiers::initialize_birth(&mut item, revision, 1).unwrap(); }
    run.inventory.push(item); run.next_id = 1000.;
    corpse_core::validation::validate(&run).unwrap();
    corpse_core::engine::validate_supported(&run).unwrap();
    run
}
fn set_tile(run: &mut Run, x: usize, y: usize, tile: &str) {
    let floor = run.floor_mut().unwrap(); let at = y*floor.width()+x;
    floor.tiles[at] = tile.into();
    if let Some(remembered) = floor.remembered_tiles.as_mut() { remembered[at] = tile.into(); }
}
fn tile(run: &Run, x: usize, y: usize) -> &str {
    let floor = run.floor().unwrap(); &floor.tiles[y*floor.width()+x]
}
fn count(run: &Run, tile: &str) -> usize { run.floor().unwrap().tiles.iter().filter(|t| *t == tile).count() }
fn cast(run: &mut Run) -> Vec<PresentationEvent> {
    let target = PackTarget::UseAt(900.);
    assert!(target.target_blocker(run, 17., 11.).is_none());
    let result = corpse_core::engine::dispatch(run, &target.command(17., 11.)).unwrap();
    assert!(result.took_turn);
    result.events
}
fn receipt(events: &[PresentationEvent]) -> Vec<(usize, usize)> {
    events.iter().find_map(|event| event.animation.as_ref().filter(|animation|
        animation.proposal == "A12" && animation.stage == Stage::Travel))
        .expect("real R52 A12 travel receipt").cells.iter()
        .map(|point| (point.x as usize, point.y as usize)).collect()
}
fn wait_until(run: &mut Run, tick: f64) {
    for _ in 0..100 {
        if run.tick >= tick { return; }
        let before = run.tick;
        assert!(corpse_core::engine::dispatch(run, &Command::Wait).unwrap().took_turn);
        assert!(run.tick > before, "normal-turn fixture must advance");
    }
    panic!("bounded expiry wait did not finish");
}
fn view(run: &Run) -> View {
    let before = serde_json::to_vec(run).unwrap();
    let view = observe::observe_with_profile(run, Profile::defaults());
    assert_eq!(serde_json::to_vec(run).unwrap(), before, "observation must be pure");
    view
}
fn aimed(run: &Run, keyboard: bool) -> View {
    let before = serde_json::to_vec(run).unwrap();
    let mut view = view(run); let target = PackTarget::UseAt(900.);
    let point = if keyboard { crate::aim_keyboard_tile(run, Some((17, 12)), sdl2::keyboard::Keycode::Up).unwrap() }
        else { (17, 11) };
    crate::sync_pointer_view(&mut view, run, Some(point), vec![], Some(&target));
    assert_eq!(serde_json::to_vec(run).unwrap(), before);
    view
}
fn capture(painter: &Painter, name: &str) {
    let Ok(directory) = std::env::var("CORPSE_R52_FLOODS_OUTPUT") else { return; };
    std::fs::create_dir_all(&directory).unwrap();
    let file = std::fs::File::create(format!("{directory}/{name}-{}x{}.png", painter.width, painter.height)).unwrap();
    let mut encoder = png::Encoder::new(file, painter.width as u32, painter.height as u32);
    encoder.set_color(png::ColorType::Rgba); encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(&painter.pixels).unwrap();
}
fn same(actual: &Painter, expected: &Painter, label: &str) {
    assert_eq!(actual.pixels.len(), expected.pixels.len());
    if let Some(index) = actual.pixels.iter().zip(&expected.pixels).position(|(a,b)| a != b) {
        capture(actual, &format!("mismatch-{label}")); capture(expected, &format!("reference-{label}"));
        panic!("{label}: exact RGBA mismatch at byte {index}, actual {}, expected {}", actual.pixels[index], expected.pixels[index]);
    }
}
/// O(F*P) byte-oracle work and O(P) buffers, in addition to production observation,
/// Scene/tile damage, glyph rasterization and effect costs. No forced-full flag.
fn render_frames(frames: &[(&str, View)], events: &[PresentationEvent], times: &[f64]) {
    assert!(events.is_empty() || frames.len() == times.len());
    for (width, height) in [(1280, 800), (960, 1200)] {
        let mut full = Painter::new(); let mut retained = Painter::new(); let mut tiled = Painter::new();
        for painter in [&mut full, &mut retained, &mut tiled] {
            painter.resize(width, height); painter.set_responsive_canvas(true);
        }
        let mut base = RetainedGame::default(); let mut planner = TilePlanner::default();
        let mut overlay = tile_overlay::Tracker::default(); let mut old: Option<&View> = None;
        let mut retained_overlay = tile_overlay::Tracker::default();
        let mut effects = effects::Effects::default();
        if !events.is_empty() { effects.add(events.to_vec(), 1., frames[0].1.depth, 0., false); }
        for (index, (name, view)) in frames.iter().enumerate() {
            let now = times.get(index).copied().unwrap_or(0.);
            effects.prune(now);
            // The ordinary three rendering paths share exactly one detached view.
            full.game_at(view, "", false, now); full.effects(view, &effects.cues, now, false);
            base.update(&mut retained, view, "", now, true, false).unwrap();
            let retained_state = tile_overlay::Frame {
                effects: tile_overlay::effects(&retained, view, &effects.cues, now, false), ..Default::default()
            };
            let mut restore = Damage::empty(width, height);
            for rect in retained_overlay.changes(&retained, &retained_state) { restore.add(rect); }
            base.restore(&mut retained, &restore).unwrap();
            retained.effects(view, &effects.cues, now, false);
            same(&retained, &full, &format!("retained-{name}"));
            retained_overlay.commit(retained_state);
            planner.replace_view(old, view, &tiled);
            let state = tile_overlay::Frame {
                effects: tile_overlay::effects(&tiled, view, &effects.cues, now, false), ..Default::default()
            };
            for rect in overlay.changes(&tiled, &state) { planner.mark_native_rect(rect); }
            planner.mark_animation(view, &tiled, now, false);
            tile_overlay::finish_dependencies(&mut tiled, &mut planner, &state);
            planner.paint_base(&mut tiled, view, "", now);
            if !tiled.tile_mask.is_empty() { tiled.effects(view, &effects.cues, now, false); }
            tiled.tile_clip_enabled = false;
            same(&tiled, &full, &format!("tile-{name}")); capture(&full, name);
            overlay.commit(state); planner.clear_frame(); old = Some(view);
        }
    }
}

#[test]
fn r52_floods_ui_repeated_casts_deepen_only_the_first_shallows_and_deep_water_stops() {
    let mut run = room(52.); let before = aimed(&run, false);
    assert_eq!(before.target_area.len(), 15);
    let first = receipt(&cast(&mut run)); assert_eq!(first.len(), 15);
    assert_eq!(count(&run, "shallow-water"), 15); assert_eq!(count(&run, "water"), 0);
    let shallow = view(&run); let second_aim = aimed(&run, false);
    assert_eq!(second_aim.target_area, vec![(8,10),(8,11),(8,12)]);
    assert_eq!(receipt(&cast(&mut run)), second_aim.target_area);
    assert_eq!(count(&run, "shallow-water"), 12); assert_eq!(count(&run, "water"), 3);
    let deep = view(&run); let blocked = aimed(&run, false);
    assert!(blocked.target_area.is_empty());
    let events = cast(&mut run);
    assert!(events.iter().filter_map(|e| e.animation.as_ref()).filter(|a|a.proposal=="A12"&&a.stage==Stage::Travel)
        .all(|a|a.cells.is_empty()));
    assert_eq!(count(&run, "water"), 3);
    let mut dismissed = aimed(&run, false);
    crate::sync_pointer_view(&mut dismissed, &run, None, vec![], None);
    assert!(dismissed.target_area.is_empty()); assert!(dismissed.target_impact.is_none());
    render_frames(&[("first-aim", before), ("first-shallows", shallow), ("second-aim", second_aim),
        ("second-deep", deep), ("third-blocked", blocked), ("aim-cleared", dismissed)], &[], &[]);
}

#[test]
fn r52_floods_ui_normal_turn_expiry_preserves_original_ground_and_refreshes_deep_cells() {
    let mut run = room(52.); cast(&mut run); let shallow = view(&run);
    let original_expiry = run.floor().unwrap().revision50_temporary_terrain.as_ref().unwrap()[0].expires_at;
    wait_until(&mut run, 4.); cast(&mut run); let deep = view(&run);
    let refreshed = run.floor().unwrap().revision50_temporary_terrain.as_ref().unwrap().iter()
        .filter(|t|t.tile=="water").map(|t|t.expires_at).fold(0., f64::max);
    assert!(refreshed > original_expiry);
    wait_until(&mut run, original_expiry); assert_eq!(count(&run, "shallow-water"), 0);
    assert_eq!(count(&run, "water"), 3); let overlapping = view(&run);
    wait_until(&mut run, refreshed); assert_eq!(count(&run, "water"), 0);
    for y in 10..=12 { for x in 8..=12 { assert_eq!(tile(&run, x, y), "floor"); } }
    assert!(run.floor().unwrap().revision50_temporary_terrain.is_missing());
    let restored = view(&run);
    let mut natural = room(52.); set_tile(&mut natural, 9, 11, "shallow-water"); cast(&mut natural);
    assert_eq!(tile(&natural,9,11),"water"); let natural_deep = view(&natural);
    wait_until(&mut natural, 12.); assert_eq!(tile(&natural,9,11),"shallow-water");
    render_frames(&[("expiry-shallow",shallow),("expiry-deep",deep),("expiry-original",overlapping),
        ("expiry-restored",restored),("natural-deep",natural_deep),("natural-restored",view(&natural))], &[], &[]);
}

#[test]
fn r52_floods_ui_mixed_lanes_mouse_keyboard_preview_and_receipt_agree() {
    let mut run = room(52.); set_tile(&mut run, 10, 10, "shallow-water"); set_tile(&mut run, 9, 11, "water");
    let initial = view(&run); let mouse = aimed(&run, false); let keyboard = aimed(&run, true);
    let expected = vec![(8,10),(8,11),(8,12),(9,10),(9,12),(10,10),(10,12),(11,12),(12,12)];
    assert_eq!(mouse.target_area, expected); assert_eq!(keyboard.target_area, expected);
    assert_eq!(mouse.target_path, keyboard.target_path); assert_eq!(mouse.target_impact, keyboard.target_impact);
    assert_eq!(receipt(&cast(&mut run)), expected);
    assert_eq!(tile(&run,10,10),"water"); assert_eq!(tile(&run,9,11),"water");
    assert_eq!(tile(&run,11,10),"floor"); assert_eq!(tile(&run,12,12),"shallow-water");
    render_frames(&[("mixed-before",initial),("mixed-mouse",mouse),("mixed-keyboard",keyboard),("mixed-after",view(&run))], &[], &[]);
}

#[test]
fn r52_floods_ui_hidden_live_water_does_not_change_known_aiming_or_admission() {
    let mut run = room(52.); run.player.conditions.clear(); run.player.stats.perception = 0.;
    run.floor_mut().unwrap().lights.fill(-100.);
    for y in 10..=12 { run.floor_mut().unwrap().lights[y*23+8] = 100.; }
    let target = PackTarget::UseAt(900.);
    let before = aimed(&run,false); let admitted = target.target_blocker(&run,17.,11.);
    assert!(!before.target_area.is_empty());
    assert!(before.target_area.iter().all(|&(x,y)|corpse_core::vision::directly_visible_tile(&run,x,y)));
    assert!(!corpse_core::vision::visible(&run,9,11));
    for material in ["shallow-water","water","floor"] {
        // Leave remembered terrain untouched; this models a hidden live mutation.
        let floor = run.floor_mut().unwrap(); let width = floor.width(); floor.tiles[11*width+9] = material.into();
        let after = aimed(&run,true);
        assert_eq!(after.target_area,before.target_area); assert_eq!(after.target_path,before.target_path);
        assert_eq!(after.target_impact,before.target_impact);
        assert_eq!(target.target_blocker(&run,17.,11.),admitted);
    }
    render_frames(&[("hidden-before",before),("hidden-after",aimed(&run,false))], &[], &[]);
}

#[test]
fn r52_floods_ui_actual_wave_animation_retained_and_tiled_pixels_match_through_removal() {
    let mut run = room(52.); set_tile(&mut run,10,10,"shallow-water"); set_tile(&mut run,9,11,"water");
    let expected = aimed(&run,false).target_area;
    let events = cast(&mut run); assert_eq!(receipt(&events),expected);
    // Use the untouched real A12 travel event, isolating the wave rather than
    // synthesizing coordinates or pretending reserve HUD effects are map effects.
    let travel: Vec<_> = events.into_iter().filter(|event|event.animation.as_ref()
        .is_some_and(|a|a.proposal=="A12"&&a.stage==Stage::Travel)).collect();
    assert_eq!(travel.len(),1);
    let mut effects = effects::Effects::default(); effects.add(travel.clone(),run.tick,run.depth,0.,false);
    for now in [0.,40.,80.,120.,180.,500.] {
        for cue in &effects.cues { for cell in effects::tiles(cue,now,false) {
            assert!(expected.contains(&(cell.x as usize,cell.y as usize)),"wave escaped actual receipt");
        }}
    }
    let before = serde_json::to_vec(&run).unwrap();
    let frames: Vec<_> = ["wave-0","wave-40","wave-80","wave-120","wave-180","wave-removed"]
        .into_iter().map(|name|(name,view(&run))).collect();
    render_frames(&frames,&travel,&[0.,40.,80.,120.,180.,500.]);
    assert_eq!(serde_json::to_vec(&run).unwrap(),before);
}

#[test]
fn r52_floods_ui_real_movement_immerses_and_clears_while_r51_retains_shallow_stopping() {
    let mut run = room(52.); cast(&mut run); cast(&mut run);
    let before = view(&run); assert!(!before.deep_water);
    assert!(corpse_core::engine::dispatch(&mut run,&Command::Move{dx:1.,dy:0.}).unwrap().took_turn);
    assert_eq!((run.player.x,run.player.y),(8.,11.)); let immersed = view(&run); assert!(immersed.deep_water);
    assert!(corpse_core::engine::dispatch(&mut run,&Command::Move{dx:-1.,dy:0.}).unwrap().took_turn);
    let dry = view(&run); assert_eq!((run.player.x,run.player.y),(7.,11.)); assert!(!dry.deep_water);
    let mut legacy = room(51.); set_tile(&mut legacy,8,11,"shallow-water");
    let legacy_aim = aimed(&legacy,false); assert!(!legacy_aim.target_area.contains(&(8,11)));
    assert_eq!(legacy_aim.target_area.len(),10); cast(&mut legacy);
    assert_eq!(tile(&legacy,8,11),"shallow-water"); assert_eq!(count(&legacy,"water"),0);
    render_frames(&[("immersion-before",before),("immersion-wet",immersed),("immersion-cleared",dry),
        ("legacy-aim",legacy_aim),("legacy-after",view(&legacy))],&[],&[]);
}

#[test]
fn r52_floods_ui_actual_pack_action_enters_target_mode_without_casting() {
    let mut run=room(52.);
    // Tracked sessions require admitted persisted geometry, not the typed-only
    // 23x23 rendering fixture. Keep the real storage gate.
    let f=run.floor_mut().unwrap();let size=50;let n=size*size;
    f.width=(size as f64).into();f.height=(size as f64).into();f.tiles=vec!["floor".into();n];
    f.lights=vec![100.;n];f.seen=vec![true;n];f.remembered_tiles=f.tiles.clone().into();
    f.water_blood=vec![0.;n].into();f.substrates=vec!["rock".into();n].into();
    let mut engine=crate::temporal::Engine::new(run).unwrap();
    let before=engine.run.clone();
    let mut pack=crate::pack::PackView::observe(&engine.run);let mut effects=effects::Effects::default();
    let mut open=true;let mut target=None;let mut notice=String::new();
    crate::apply_pack_action(crate::pack::PackAction::Target(PackTarget::UseAt(900.)),
        &mut pack,&mut engine,&mut effects,0.,&mut open,&mut target,&mut notice);
    assert_eq!(target,Some(PackTarget::UseAt(900.)));assert!(!open);
    assert_eq!(engine.run,before);
    assert!(notice.contains("target"));
}
