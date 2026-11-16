//! Player-only presentation regressions; no simulation revision or fixture rewrite.
use super::*;
use crate::{appearance_profile::Profile, render::Painter};
use corpse_core::{Actor, Field};
use serde_json::json;

fn room() -> Run {
    let mut run = corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    run.simulation_revision = 52.0.into();
    run.player.abilities.clear();
    run.player.active.clear();
    run.player.conditions.clear();
    run.actors.clear();
    run.ground.clear();
    run.offers.clear();
    run.player.glyph = "(≬)".into();
    run
}

fn assert_player(cell: &Cell) {
    assert_eq!(cell.glyph, "@");
    assert!(cell.superscript.is_empty(), "player superscript: {:?}", cell.superscript);
    assert!(cell.subscript.is_empty());
}

fn player(view: &View) -> &Cell {
    view.cells.iter().find(|cell| cell.actor_id == Some(view.player_id)).unwrap()
}

fn dream_room() -> Run {
    let mut run = room();
    let mut body = run.player.clone();
    body.id = 12.;
    body.name = "Your sleeping body".into();
    body.conditions.insert("phantasmiiBody".into(), 1.);
    body.conditions.insert("halfHereOwner".into(), run.player.id);
    run.actors_at_mut(1.).push(body);
    run.dreamwalking = corpse_core::dreamwalking::State {
        active: true, depth: 1., body_id: Some(12.),
        anchor_x: run.player.x, anchor_y: run.player.y,
        ..Default::default()
    }.into();
    run.player.x += 2.;
    corpse_core::dreamwalking::ensure_layer(&mut run);
    run
}

#[test]
fn player_glyph_all_modes_revisions_and_terminal_states_are_plain_at() {
    let backgrounds = vec![[0; 3]; 79 * 29];
    for revision in [29., 34., 43., 50., 51., 52.] {
        for mode in ["Unblessed", "Blessed", "Avatar"] {
            for dead in [false, true] {
                let mut run = room();
                run.simulation_revision = revision.into();
                run.mode = mode.into();
                run.player.dead = dead.into();
                let before = serde_json::to_vec(&run).unwrap();
                let cell = actor_cell(&run, &run.player, 79, &backgrounds, true);
                assert_player(&cell);
                assert_eq!(cell.corpse, dead, "existing death marker remains independent");
                assert_eq!(serde_json::to_vec(&run).unwrap(), before);
            }
        }
    }
}

#[test]
fn player_glyph_custom_profile_cannot_replace_identity_but_keeps_styling() {
    let run = room();
    let backgrounds = vec![[0; 3]; 79 * 29];
    let profile = Profile::default()
        .set("entities.objects.player.inherited", json!(false)).unwrap()
        .set("entities.objects.player.glyph", json!("P")).unwrap()
        .set("entities.objects.player.color", json!("#123456")).unwrap()
        .set("entities.objects.player.background", json!("#654321")).unwrap()
        .set("entities.objects.player.emphasis", json!(1.5)).unwrap();
    let before = serde_json::to_vec(&run).unwrap();
    let cell = actor_cell_profile(&profile, &run, &run.player, 79, &backgrounds, true);
    assert_player(&cell);
    assert_eq!(cell.fg, [0x12, 0x34, 0x56]);
    assert_eq!(cell.background_overlay, Some(([0x65, 0x43, 0x21], 1.)));
    assert_eq!(cell.emphasis, 1.5);
    let global = Profile::default().set("entities.defaults.glyph", json!("G")).unwrap();
    assert_player(&actor_cell_profile(&global, &run, &run.player, 79, &backgrounds, true));
    assert_eq!(profile.value("entities.objects.player.glyph"), Some(&json!("P")));
    assert_eq!(serde_json::to_vec(&run).unwrap(), before);
}

#[test]
fn player_glyph_does_not_replace_creature_ally_rank_or_half_here_annotations() {
    let run = room();
    let backgrounds = vec![[0; 3]; 79 * 29];
    let mut creature = Actor {
        id: 10., x: 13., y: 14., glyph: "(≬)".into(), color: "#abcabc".into(),
        state: "ally".into(), order: "f".to_string().into(), ..Default::default()
    };
    let cell = actor_cell(&run, &creature, 79, &backgrounds, false);
    assert_eq!((&*cell.glyph, &*cell.superscript, &*cell.subscript), ("(≬)", "f", ""));
    creature.creature_kind = "zillennia".to_string().into();
    creature.zillennia_quality = "transcendent".to_string().into();
    let cell = actor_cell(&run, &creature, 79, &backgrounds, false);
    assert_eq!((&*cell.glyph, &*cell.superscript, &*cell.subscript), ("(≬)", "10", "f"));
    creature.creature_kind = Field::Missing;
    creature.glyph = "@".into();
    creature.conditions.insert("halfHereClone".into(), 1.);
    creature.conditions.insert("halfHereOwner".into(), run.player.id);
    let before = creature.clone();
    let cell = actor_cell(&run, &creature, 79, &backgrounds, false);
    assert_eq!((&*cell.glyph, &*cell.superscript), ("@", "h"));
    assert_eq!(creature, before);
}

#[test]
fn player_glyph_dream_and_sleeping_body_share_at_without_changing_identity_or_inspection() {
    let run = dream_room();
    let before = serde_json::to_vec(&run).unwrap();
    let view = observe(&run);
    assert_player(player(&view));
    assert!(player(&view).dream_entity);
    let body = view.cells.iter().find(|cell| cell.actor_id == Some(12.)).unwrap();
    assert_player(body);
    assert!(!body.dream_entity);
    assert_eq!(body.fg, crate::dream_ui::NORMAL_INK);
    let row = view.creatures.iter().find(|row| row.id == 12.).unwrap();
    assert_eq!(row.glyph, "@");
    assert_eq!(row.name, "Your sleeping body");
    assert_eq!(row.state, "Sleeping");
    let report = crate::inspection_ui::observe_tile(&run, body.x, body.y);
    assert_eq!(report.title, "Your sleeping body");
    assert!(report.creature.unwrap().resources.contains("Shared with you"));
    assert_eq!(serde_json::to_vec(&run).unwrap(), before);
    assert_eq!(run.player.glyph, "(≬)");
    assert_eq!(corpse_core::dreamwalking::body(&run).unwrap().glyph, "(≬)");
}

#[test]
fn player_glyph_portal_aliases_and_hallucination_keep_actual_player_unannotated() {
    let mut run = room();
    let w = run.floor().unwrap().width();
    let at = run.player.y as usize * w + run.player.x as usize;
    run.player.conditions.insert("hallucinating".into(), 10.);
    run.player.conditions.insert("allSeeingReveal".into(), 1.);
    run.player.conditions.insert("allSeeingDepth".into(), run.depth);
    run.floor_mut().unwrap().portals = vec![corpse_core::portals_generation::PortalPair {
        a: at, b: at + 2, color_index: 0,
    }].into();
    let before = serde_json::to_vec(&run).unwrap();
    let mut view = observe(&run);
    for phase in 0..3 {
        crate::hallucination_ui::apply_view(&mut view, phase);
        let cells: Vec<_> = view.cells.iter().filter(|cell| cell.actor_id == Some(run.player.id)).collect();
        assert_eq!(cells.len(), 2);
        assert!(cells.iter().any(|cell| cell.label.ends_with("(through portal)")));
        for cell in cells { assert_player(cell); assert!(cell.hallucination.is_none()); }
    }
    assert_eq!(serde_json::to_vec(&run).unwrap(), before);
}

fn capture(painter: &Painter, name: &str) {
    let Ok(directory) = std::env::var("CORPSE_PLAYER_GLYPH_REVIEW") else { return; };
    std::fs::create_dir_all(&directory).unwrap();
    let file = std::fs::File::create(format!("{directory}/{name}.png")).unwrap();
    let mut encoder = png::Encoder::new(file, painter.width as u32, painter.height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(&painter.pixels).unwrap();
}

#[test]
fn player_glyph_native_raster_and_inspection_backdrop_have_no_mode_superscript() {
    for (width, height) in [(1280, 800), (1920, 1080)] {
        let run = room();
        let before = serde_json::to_vec(&run).unwrap();
        let view = observe(&run);
        assert_player(player(&view));
        let mut painter = Painter::new();
        painter.resize(width, height);
        painter.game_at(&view, "", false, 0.);
        let plain = painter.pixels.clone();
        capture(&painter, &format!("player-plain-at-{width}x{height}"));
        let mut old = observe(&run);
        old.cells.iter_mut().find(|cell| cell.actor_id == Some(run.player.id)).unwrap().superscript = "3".into();
        painter.game_at(&old, "", false, 0.);
        assert_ne!(painter.pixels, plain, "the legacy marker really paints at {width}x{height}");
        painter.game_at(&view, "", false, 0.);
        assert_eq!(painter.pixels, plain);
        crate::inspection_ui::paint(&crate::inspection_ui::observe_tile(&run, run.player.x as usize, run.player.y as usize), &mut painter, 0);
        capture(&painter, &format!("player-inspect-{width}x{height}"));
        assert_eq!(serde_json::to_vec(&run).unwrap(), before);
    }
    let run = dream_room();
    let view = observe(&run);
    let mut painter = Painter::new();
    painter.game_at(&view, "", false, 0.);
    capture(&painter, "player-dream-and-body-1280x800");
}


#[test]
fn player_invisibility_is_spectral_blue_and_restores_ordinary_ink() {
    let backgrounds = vec![[13, 9, 22]; 79 * 29];
    let profile = Profile::default()
        .set("entities.objects.player.inherited", json!(false)).unwrap()
        .set("entities.objects.player.color", json!("#ff2200")).unwrap()
        .set("entities.objects.player.background", json!("#654321")).unwrap();
    for revision in [29., 52.] {
        for lighting in [-100., 0., 100.] {
            let mut run = room();
            run.simulation_revision = revision.into();
            run.floor_mut().unwrap().lights.fill(lighting);
            for profile in [Profile::defaults(), &profile] {
                let ordinary = actor_cell_profile(profile, &run, &run.player, 79, &backgrounds, true);
                run.player.conditions.insert("invisible".into(), 5.);
                let before = serde_json::to_vec(&run).unwrap();
                let light = corpse_core::vision::light(&run, &run.player);
                let brightness = 0.56 + 0.44 * ((light.clamp(-100., 100.) + 100.) / 200.);
                let invisible = actor_cell_profile(profile, &run, &run.player, 79, &backgrounds, true);
                assert_player(&invisible);
                assert_eq!(invisible.fg, [135., 183., 243.].map(|c| (c * brightness).round() as u8));
                assert_eq!(invisible.entity_ink.unwrap().color, invisible.fg);
                assert_eq!(invisible.bg, ordinary.bg);
                assert_eq!(invisible.background_overlay, ordinary.background_overlay);
                assert_eq!(invisible.emphasis, ordinary.emphasis);
                assert_eq!(corpse_core::vision::light(&run, &run.player), light);
                assert_eq!(serde_json::to_vec(&run).unwrap(), before);
                run.player.conditions.insert("invisible".into(), 0.);
                assert_eq!(actor_cell_profile(profile, &run, &run.player, 79, &backgrounds, true).fg, ordinary.fg);
                run.player.conditions.remove("invisible");
            }
        }
    }
}

#[test]
fn player_invisibility_native_raster_is_blue_and_reversible() {
    let mut run = room();
    let ordinary = observe(&run);
    let mut painter = Painter::new();
    painter.game_at(&ordinary, "", false, 0.);
    let pixels = painter.pixels.clone();
    run.player.conditions.insert("invisible".into(), 5.);
    let invisible = observe(&run);
    let cell = player(&invisible);
    assert_player(cell);
    let (ink, _) = crate::render::readable_cell_colors(cell);
    assert!(ink[2] > ink[1] && ink[1] > ink[0]);
    painter.game_at(&invisible, "", false, 0.);
    assert_ne!(painter.pixels, pixels);
    capture(&painter, "player-invisible-spectral-blue-1280x800");
    run.player.conditions.remove("invisible");
    painter.game_at(&observe(&run), "", false, 0.);
    assert_eq!(painter.pixels, pixels);
}
