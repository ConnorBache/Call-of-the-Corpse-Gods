//! Native, deterministic evidence for the R50 visual follow-up. No game rules.
use super::*;
use crate::appearance_profile::Profile;
use serde_json::json;

fn fixture() -> corpse_core::Run {
    let mut run = corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    run.simulation_revision = 50.0.into();
    run
}

fn capture(name: &str, painter: &Painter) {
    let Ok(dir) = std::env::var("CORPSE_R50_VISUAL_OUTPUT") else { return; };
    std::fs::create_dir_all(&dir).unwrap();
    let file = std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
    let mut encoder = png::Encoder::new(file, painter.width as u32, painter.height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(&painter.pixels).unwrap();
}

#[test]
fn revision50_corpse_markers_blood_preferences_and_renderer_parity() {
    let mut run = fixture();
    let original = run.actors_at(run.depth)[0].clone();
    let dead = &mut run.actors_at_mut(run.depth)[0];
    dead.dead = true.into(); dead.hp = 0.; dead.heartless = true.into();
    let mut living = original.clone(); living.id = 5.; living.x = 18.;
    let mut heartless = original.clone(); heartless.id = 6.; heartless.x = 20.; heartless.heartless = true.into();
    let mut body = original.clone(); body.id = 7.; body.x = 22.; body.heartless = true.into();
    body.conditions.insert("phantasmiiBody".into(), 1.);
    run.actors_at_mut(run.depth).extend([living, heartless, body]);
    let mut dry_run = run.clone();
    let width = run.floor().unwrap().width();
    let mut blood = vec![false; run.floor().unwrap().tiles.len()];
    for x in 13..16 { blood[14 * width + x] = true; }
    run.floor_mut().unwrap().blood = blood.into();
    dry_run.floor_mut().unwrap().blood = vec![false; run.floor().unwrap().tiles.len()].into();
    let frozen = run.clone();
    let custom = Profile::default()
        .set("terrain.feature:blood.color", json!("#42ccaa")).unwrap()
        .set("terrain.feature:blood.background", json!("#00ff00")).unwrap()
        .set("terrain.feature:blood.glyph", json!("X")).unwrap();
    let mut default_dot = None;
    for (name, profile) in [("default", Profile::default()), ("custom-blood", custom)] {
        let view = crate::observe::observe_with_profile(&run, &profile);
        let dry = crate::observe::observe_with_profile(&dry_run, &profile);
        let corpse = view.cells.iter().find(|c| c.actor_id == Some(original.id)).unwrap();
        assert!(corpse.corpse); assert!(!corpse.heartless); assert_eq!(corpse.glyph, original.glyph);
        for (id, ring) in [(5., false), (6., true), (7., false)] {
            let cell = view.cells.iter().find(|c| c.actor_id == Some(id)).unwrap();
            assert!(!cell.corpse, "living/sleeping bodies never gain an X");
            assert_eq!(cell.heartless, ring);
        }
        for x in 13..16 {
            let dot = view.cells.iter().find(|c| c.x == x && c.y == 14).unwrap();
            let dry_dot = dry.cells.iter().find(|c| c.x == x && c.y == 14).unwrap();
            assert_eq!(dot.bg, dry_dot.bg); assert_eq!(dot.glyph, dry_dot.glyph);
            assert_ne!(dot.fg, dry_dot.fg);
            if x == 13 {
                if let Some(default) = default_dot { assert_ne!(dot.fg, default, "custom blood foreground survives"); }
                else { default_dot = Some(dot.fg); }
            }
        }
        let mut full = Painter::new(); full.profile = profile.clone();
        full.resize(1920, 1200); full.game_at(&view, "", false, 0.);
        capture(&format!("markers-{name}"), &full);
        let mut retained_painter = Painter::new(); retained_painter.profile = profile.clone(); retained_painter.resize(1920, 1200);
        let mut retained = RetainedGame::default();
        retained.update(&mut retained_painter, &dry, "", 0., true, true).unwrap();
        retained.update(&mut retained_painter, &view, "", 0., true, false).unwrap();
        assert_eq!(retained_painter.pixels, full.pixels, "retained blood repaint");
        let mut tiled = Painter::new(); tiled.profile = profile; tiled.resize(1920, 1200);
        let mut plan = tile_damage::TilePlanner::default();
        plan.replace_view(None, &dry, &tiled); plan.finish_damage(&mut tiled); plan.paint_base(&mut tiled, &dry, "", 0.);
        tiled.tile_clip_enabled = false; plan.clear_frame();
        plan.replace_view(Some(&dry), &view, &tiled); plan.finish_damage(&mut tiled); plan.paint_base(&mut tiled, &view, "", 0.);
        tiled.tile_clip_enabled = false;
        assert_eq!(tiled.pixels, full.pixels, "tile-enabled blood repaint");
    }
    assert_eq!(run, frozen, "all observation and painting are read-only");
}

#[test]
fn revision50_corpse_markers_never_reveal_hidden_or_forgotten_actors() {
    let mut run = fixture();
    run.player.abilities.clear(); run.player.active.clear(); run.player.conditions.clear();
    let id = run.actors_at(run.depth)[0].id;
    run.actors_at_mut(run.depth)[0].dead = true.into();
    run.actors_at_mut(run.depth)[0].hp = 0.;
    assert!(crate::observe::observe(&run).cells.iter().any(|c| c.actor_id == Some(id) && c.corpse));
    run.actors_at_mut(run.depth)[0].conditions.insert("invisible".into(), 10.);
    assert!(!crate::observe::observe(&run).cells.iter().any(|c| c.actor_id == Some(id)));
    run.actors_at_mut(run.depth)[0].conditions.remove("invisible");
    let width = run.floor().unwrap().width();
    let height = run.floor().unwrap().height();
    for y in 1..height-1 { run.floor_mut().unwrap().tiles[y * width + 14] = "wall".into(); }
    run.player.stats.perception = 0.;
    assert!(!corpse_core::vision::visible_actor(&run, &run.actors_at(run.depth)[0]));
    let fog = crate::observe::observe(&run);
    assert!(!fog.cells.iter().any(|c| c.actor_id == Some(id)));
    let mut hidden_changed = run.clone();
    let hidden = &mut hidden_changed.actors_at_mut(run.depth)[0];
    hidden.dead = false.into(); hidden.hp = 100.; hidden.glyph = "Z".into(); hidden.color = "#00ff00".into();
    assert_eq!(fog.cells, crate::observe::observe(&hidden_changed).cells, "hidden death/identity changes cannot leak through X marks");
    let mut p = Painter::new(); p.resize(1920, 1200); p.game_at(&fog, "", false, 0.); capture("markers-fog", &p);
    let frozen = run.clone();
    let mut unknown = run.clone(); unknown.floor_mut().unwrap().seen.fill(false);
    unknown.floor_mut().unwrap().remembered_tiles = corpse_core::Field::Missing;
    assert!(!crate::observe::observe(&unknown).cells.iter().any(|c| c.actor_id == Some(id)));
    assert_eq!(run, frozen);
}

/// Test-only reconstruction of the former native 88% transform, not a scaled
/// screenshot or a runtime option. Both captures paint the same source content.
fn before_sheet(p: &mut Painter, paint: impl FnOnce(&mut Painter)) {
    let prior = (p.scale, p.origin, p.responsive_canvas);
    p.scale = (p.width as f64 / WIDTH as f64).min(p.height as f64 / HEIGHT as f64) * 0.88;
    p.origin = (((p.width as f64 - WIDTH as f64 * p.scale) / 2.).round() as i32,
        ((p.height as f64 - HEIGHT as f64 * p.scale) / 2.).round() as i32);
    p.responsive_canvas = false;
    paint(p);
    (p.scale, p.origin, p.responsive_canvas) = prior;
}

#[test]
fn revision50_smaller_dialogs_native_before_after_and_repeat_close() {
    assert_eq!(DIALOG_SCALE, 0.84);
    let run = fixture(); let frozen = run.clone(); let view = crate::observe::observe(&run);
    let ability = crate::ability_ui::AbilityView {
        id: "visual-fixture".into(), name: "An observed ability".into(),
        description: format!("FIRST LINE. {} FINAL LINE.", "Full observed ability details remain available by scrolling. ".repeat(30)),
        color: GOLD, cost: "6 Energy".into(), state: "Ready".into(), active: false, mode: "Active".into(), supported: true,
    };
    let report = crate::inspection_ui::observe_tile(&run, run.player.x as usize, run.player.y as usize);
    for (w, h) in [(1280, 800), (1920, 1080), (960, 720)] {
        let mut p = Painter::new(); p.resize(w, h); p.set_responsive_canvas(true);
        p.game_at(&view, "", false, 0.); let base = p.pixels.clone();
        let canvas = (p.scale, p.origin, p.logical_width(), p.logical_height());
        for name in ["inspection", "ability-top", "ability-bottom", "hazard"] {
            for before in [true, false] {
                p.pixels.copy_from_slice(&base);
                let paint = |s: &mut Painter| match name {
                    "inspection" => crate::inspection_ui::paint(&report, s, 0),
                    "ability-top" => s.ability_detail_sheet(&ability, 0),
                    "ability-bottom" => s.ability_detail_sheet(&ability, usize::MAX),
                    _ => s.hazard_prompt_sheet("Walk amid razorleaves? Ending your turn there costs 2% of your maximum health (rounded up)."),
                };
                if before { before_sheet(&mut p, paint); } else { p.centered_sheet(paint); }
                assert_ne!(p.pixels, base);
                assert_eq!((p.scale, p.origin, p.logical_width(), p.logical_height()), canvas);
                capture(&format!("dialog-{name}-{}-{w}x{h}", if before { "before-88" } else { "after-84" }), &p);
                p.game_at(&view, "", false, 0.);
                assert_eq!(p.pixels, base, "close and repeat leaves Chronicle, HUD and map unchanged");
            }
        }
    }
    assert_eq!(run, frozen);
}

#[test]
fn revision50_dialog_borders_survive_their_own_fractional_clip() {
    for (w, h) in [(1280, 800), (960, 720), (1920, 1080), (1537, 911), (960, 1200)] {
        let mut p = Painter::new(); p.resize(w, h); p.clear([1, 2, 3]);
        let background = p.pixels.clone();
        let ink = [53, 55, 68];
        let bounds = p.centered_sheet(|s| {
            let (x, y, w, h) = crate::inspection_ui::bounds(&s.profile);
            let bounds = s.logical_damage_rect(x, y, w, h).unwrap();
            s.clip = Some((x, y, w, h));
            s.border(x, y, w, h, ink);
            s.clip = None;
            bounds
        });
        let pixel = |x: usize, y: usize| &p.pixels[(y * p.width + x) * 4..(y * p.width + x) * 4 + 3];
        for x in bounds.x..bounds.right() {
            assert_eq!(pixel(x, bounds.y), ink, "top {w}x{h}");
            assert_eq!(pixel(x, bounds.bottom() - 1), ink, "bottom {w}x{h}");
        }
        for y in bounds.y..bounds.bottom() {
            assert_eq!(pixel(bounds.x, y), ink, "left {w}x{h}");
            assert_eq!(pixel(bounds.right() - 1, y), ink, "right {w}x{h}");
        }
        for (i, (actual, original)) in p.pixels.chunks_exact(4).zip(background.chunks_exact(4)).enumerate() {
            if !bounds.contains(i % p.width, i / p.width) { assert_eq!(actual, original, "outside panel footprint"); }
        }
    }
}
