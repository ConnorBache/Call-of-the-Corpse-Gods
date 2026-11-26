//! Rendering-only chasm regressions. Real detached observation and native pixels.
use crate::{appearance_profile::Profile, render::{Cell, Painter, View}};
use corpse_core::{Field, Floor, Run};
use std::sync::Arc;
use crate::color::{Light, light_ui::DisplayLight};

fn room(level: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 52.0.into();
    r.player.x = 8.; r.player.y = 6.; r.player.stats.perception = 300.;
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.ability_order = Field::Missing; r.player.equipment_abilities = Field::Missing;
    r.inventory.clear(); r.actors = Default::default(); r.ground.clear();
    r.history.clear(); r.log.clear(); r.offers.clear();
    let mut tiles = vec!["floor".to_owned(); 441];
    for y in 2..19 { for x in 10..16 { if y != 10 { tiles[y*21+x] = "chasm".into(); } } }
    r.floors = vec![Arc::new(Floor { depth:r.depth, width:21.0.into(), height:21.0.into(),
        tiles:tiles.clone(), remembered_tiles:tiles.into(), lights:vec![level;441],
        seen:vec![true;441], substrates:vec!["stone".into();441].into(), ..Default::default() })];
    r
}
fn remembered_room(level: f64) -> Run {
    let mut run = room(level);
    run.player.x = 1.; run.player.y = 6.; run.player.stats.perception = 0.;
    // Real LOS occlusion, not manually altered observation flags.
    for y in 0..21 { run.floor_mut().unwrap().tiles[y * 21 + 5] = "wall".into(); }
    run
}
fn tile_pixels(painter: &Painter, view: &View, cell: &Cell) -> Vec<u8> {
    let (cw, ch, ox, oy) = painter.map_layout(view.width, view.height);
    let rect = painter.logical_damage_rect(ox + cell.x as i32 * cw,
        oy + cell.y as i32 * ch, cw, ch).unwrap();
    (rect.y..rect.bottom()).flat_map(|y|
        painter.pixels[(y * painter.width + rect.x) * 4..(y * painter.width + rect.right()) * 4]
            .iter().copied()).collect()
}
fn sample(view: &View) -> &Cell {
    view.cells[..view.terrain_count].iter().find(|c| c.x == 10 && c.y == 6).unwrap()
}
fn png(name: &str, painter: &Painter) {
    if let Ok(dir) = std::env::var("CORPSE_CHASM_OUTPUT") {
        std::fs::create_dir_all(&dir).unwrap();
        let file = std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
        let mut png = png::Encoder::new(file, painter.width as u32, painter.height as u32);
        png.set_color(png::ColorType::Rgba); png.set_depth(png::BitDepth::Eight);
        png.write_header().unwrap().write_image_data(&painter.pixels).unwrap();
    }
}

// Reference the existing ordinary terrain finishing contract, not any chasm
// branch: exposure/desaturation, then the shared negative-light contrast floor.
fn ordinary_terrain_ink(display: DisplayLight, foreground: crate::render::Rgb,
    background: crate::render::Rgb) -> crate::render::Rgb {
    let ink = display.foreground(foreground, false);
    if display.illumination >= 0. { return ink; }
    let rescued = crate::color::light_ui::foreground_contrast(ink, display.apply(background), 3.);
    let t = (-display.illumination / 25.).clamp(0., 1.);
    crate::color::composite(ink, rescued, t * t * (3. - 2. * t))
}

#[test]
fn chasm_light_finishing_matches_ordinary_terrain_from_same_base_ink() {
    let profile = Profile::default();
    for remembered in [false, true] {
        for level in [-150., -100., -75., -50., -25., -5., 0., 25., 50., 75., 100., 150.] {
            for tint in [[1.; 3], crate::color::light_ui::WARM.map(|c| c as f64 / 255.)] {
                let display = DisplayLight { tint, ..DisplayLight::neutral(level) };
                let light = Light { display:Some(display), ..Light::ambient(level) };
                let mut chasm = Cell { label:"chasm".into(), glyph:"⛶".into(),
                    fg:if remembered {[12,11,16]} else {[23,20,56]}, bg:[0;3],
                    current:!remembered, direct:!remembered, ..Default::default() };
                let base = chasm.clone();
                // An empty label skips the separate neutral readability stage,
                // supplying exactly the same authored ink to ordinary lighting.
                let mut ordinary = chasm.clone(); ordinary.label.clear();
                super::finish_cell(&mut chasm,light,&profile);
                super::finish_cell(&mut ordinary,light,&profile);
                assert_eq!((chasm.fg,chasm.bg,chasm.display_light),
                    (ordinary.fg,ordinary.bg,ordinary.display_light),
                    "chasm-specific light suppression at {level}, remembered={remembered}");
                assert_eq!((chasm.current,chasm.direct,chasm.glyph.as_str()),
                    (base.current,base.direct,base.glyph.as_str()));
                if remembered {
                    assert_eq!(chasm,base,"hidden light must not change memory at {level}");
                } else {
                    assert_eq!(chasm.fg,ordinary_terrain_ink(display,base.fg,base.bg));
                }
            }
        }
    }
}

#[test]
fn chasm_native_capture_and_detached_state_are_stable() {
    let mut rows = Vec::new();
    for level in [-100., -50., -25., 0., 50., 100.] {
        let run = room(level); let frozen = serde_json::to_vec(&run).unwrap();
        let view = crate::observe::observe(&run); let c = sample(&view);
        assert!(c.current && c.direct); assert_eq!(c.label, "chasm");
        let mut painter = Painter::new(); painter.resize(1280,800);
        painter.game_at(&view, "", false, 0.); let pixels = painter.pixels.clone();
        png(&format!("chasm-{level}"), &painter);
        painter.game_at(&view, "", false, 731.);
        assert_eq!(pixels, painter.pixels, "static chasm scene at {level}");
        assert_eq!(serde_json::to_vec(&run).unwrap(), frozen);
        rows.push(serde_json::json!({"illumination":level,"foreground":c.fg,"background":c.bg,
            "edges":c.edges,"glyph":c.glyph,"current":c.current,"direct":c.direct,
            "nativeRgbaCrc32":crc32fast::hash(&pixels)}));
    }
    if let Ok(dir) = std::env::var("CORPSE_CHASM_OUTPUT") {
        std::fs::write(format!("{dir}/colors.json"), serde_json::to_vec_pretty(&rows).unwrap()).unwrap();
    }
}

#[test]
fn chasm_ink_and_boundary_use_source_palette_and_ordinary_illumination() {
    let neutral = crate::observe::observe(&room(0.));
    let source = sample(&neutral);
    for (channel, range) in [(0,18..=28),(1,18..=28),(2,38..=59)] {
        assert!(range.contains(&source.fg[channel]));
    }
    let edge = crate::color::composite([0;3], source.fg, 0.62);
    assert!(source.edges.iter().flatten().all(|c| *c == edge));
    assert!(source.edges.iter().any(Option::is_some));
    for level in [-100.,-75.,-50.,-25.,-5.,0.,25.,50.,100.] {
        let run = room(level); let before = serde_json::to_vec(&run).unwrap();
        let view = crate::observe::observe(&run); let c = sample(&view);
        assert_eq!(c.glyph, source.glyph); assert!(c.current && c.direct);
        assert_eq!(c.bg, [0;3]);
        let display = DisplayLight::neutral(level);
        assert_eq!(c.fg, ordinary_terrain_ink(display, source.fg, source.bg), "ordinary light at {level}");
        for (actual, raw) in c.edges.iter().zip(&source.edges) {
            assert_eq!(*actual, raw.map(|color| display.foreground(color, false)));
        }
        if level <= -25. { assert!(crate::color::contrast(c.fg,c.bg) >= 2.95); }
        assert_eq!(serde_json::to_vec(&run).unwrap(),before);
    }
}

#[test]
fn chasm_ordinary_illumination_preserves_entity_fire_and_other_terrain_readability() {
    let profile = Profile::default();
    let light = Light { display:Some(DisplayLight::neutral(-100.)), ..Light::ambient(-100.) };
    let base = Cell { label:"chasm".into(), glyph:"⛶".into(), fg:[23,23,48],
        bg:[0;3], current:true, direct:true, ..Default::default() };
    let mut bare = base.clone(); super::finish_cell(&mut bare,light,&profile);
    assert_eq!(bare.fg, ordinary_terrain_ink(DisplayLight::neutral(-100.),base.fg,base.bg));
    for role in ["creature", "item", "fire", "floor", "pit", "wall"] {
        let mut c = base.clone();
        match role {
            "creature" => c.actor_id = Some(900.),
            "item" => c.entity_ink = Some(crate::render::EntityInk { color:base.fg,
                player:false, telepathic:false, important:true, remembered:false,
                detected:false, identity_outline:None, plain_black:false }),
            "fire" => c.fire = true,
            _ => c.label = role.into(),
        }
        super::finish_cell(&mut c,light,&profile);
        assert!(crate::color::contrast(c.fg,c.bg) >= if role == "creature" {4.45} else {2.95}, "{role}");
    }
    // Exercise unbounded detached light separately: stored Floor.lights is a
    // bounded material input and cannot stand in for arbitrary observed light.
    for level in [-150.,150.] {
        let display = DisplayLight::neutral(level);
        let mut c = base.clone();
        super::finish_cell(&mut c,Light { display:Some(display), ..Light::ambient(level) },&profile);
        assert_eq!(c.fg,ordinary_terrain_ink(display,base.fg,base.bg));
    }
    let settled = bare.clone(); super::finish_cell(&mut bare,light,&profile);
    assert_eq!(bare,settled,"finishing is still idempotent");
    super::restore_source_ink(&mut bare); super::finish_cell(&mut bare,light,&profile);
    assert_eq!(bare,settled,"detached rebuild must not add brightness");
}

#[test]
fn chasm_custom_ink_and_grading_remain_authoritative() {
    let neutral = room(0.); let default = crate::observe::observe(&neutral);
    for (key,value) in [("terrain.chasm.color",serde_json::json!("#eebb77")),
        ("terrain.chasm.background",serde_json::json!("#152739")),
        ("terrain.chasm.opacity",serde_json::json!(0.25)),
        ("global.exposure",serde_json::json!(0.5))] {
        let profile = Profile::default().set(key,value).unwrap();
        let zero = crate::observe::observe_with_profile(&neutral,&profile);
        assert_ne!((sample(&zero).fg,sample(&zero).bg),(sample(&default).fg,sample(&default).bg),"{key}");
        let dark = crate::observe::observe_with_profile(&room(-50.),&profile);
        assert_eq!(sample(&dark).fg,ordinary_terrain_ink(DisplayLight::neutral(-50.),sample(&zero).fg,sample(&zero).bg),"{key}");
        assert_eq!(sample(&dark).bg,DisplayLight::neutral(-50.).apply(sample(&zero).bg),"{key}");
    }
}

#[test]
fn chasm_full_retained_tile_and_idle_paths_match_native_pixels() {
    use crate::render::{RetainedGame,tile_damage::TilePlanner};
    let mut full=Painter::new(); let mut retained=Painter::new(); let mut tiled=Painter::new();
    for p in [&mut full,&mut retained,&mut tiled] { p.resize(1280,800); }
    let mut cache=RetainedGame::default(); let mut planner=TilePlanner::default(); let mut previous=None;
    for level in [0.,-100.,-50.,50.,0.] {
        let view=crate::observe::observe(&room(level));
        for now in [0.,731.] {
            full.game_at(&view,"",false,now);
            cache.update(&mut retained,&view,"",now,true,previous.is_none()).unwrap();
            assert_eq!(retained.pixels,full.pixels,"retained at {level}/{now}");
            planner.replace_view(previous.as_ref(),&view,&tiled); planner.finish_damage(&mut tiled);
            planner.paint_base(&mut tiled,&view,"",now); tiled.tile_clip_enabled=false; planner.clear_frame();
            assert_eq!(tiled.pixels,full.pixels,"tiled at {level}/{now}");
        }
        previous=Some(view);
    }
}

#[test]
fn remembered_chasm_native_pixels_preserve_dark_authored_ink_and_black_background() {
    let neutral = crate::observe::observe(&room(0.));
    let expected_ink = crate::color::memory(sample(&neutral).fg);
    let mut evidence = Vec::new();
    for level in [100., 0., -100.] {
        for remembered in [false, true] {
            let run = if remembered { remembered_room(level) } else { room(level) };
            let before = serde_json::to_vec(&run).unwrap();
            let view = crate::observe::observe(&run);
            let cell = sample(&view);
            assert_eq!(cell.current, !remembered);
            assert_eq!(cell.direct, !remembered);
            assert_eq!(cell.bg, [0; 3]);
            if remembered {
                assert_eq!(cell.fg, expected_ink, "hidden illumination must not affect memory");
                assert!(cell.display_light.is_none());
            }
            let mut painter = Painter::new(); painter.resize(1280, 800);
            painter.game_at(&view, "", false, 0.);
            let name = format!("chasm-{}-{level}", if remembered { "remembered" } else { "visible" });
            png(&name, &painter);
            let pixels = tile_pixels(&painter, &view, cell);
            let painted = crate::render::readable_cell_colors(cell);
            assert_eq!(painted, (cell.fg, cell.bg), "{name}: never lift chasm memory to gray");
            let peak = *cell.fg.iter().max().unwrap();
            assert!(pixels.chunks_exact(4).all(|pixel| pixel[..3].iter().all(|value| *value <= peak)),
                "{name}: native pixel is brighter than authored terrain ink");
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[..3] == [0; 3]));
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[..3].iter().any(|value| *value > 0)));
            evidence.push(serde_json::json!({"remembered":remembered,"liveIllumination":level,
                "foreground":cell.fg,"background":cell.bg,"paintForeground":painted.0,
                "paintBackground":painted.1,"foregroundLuminance":crate::color::luminance(cell.fg),
                "contrast":crate::color::contrast(cell.fg,cell.bg),"tileRgbaCrc32":crc32fast::hash(&pixels),
                "nativeRgbaCrc32":crc32fast::hash(&painter.pixels)}));
            assert_eq!(serde_json::to_vec(&run).unwrap(), before);
        }
    }
    if let Ok(dir) = std::env::var("CORPSE_CHASM_OUTPUT") {
        std::fs::write(format!("{dir}/remembered-colors.json"), serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
}

#[test]
fn remembered_chasm_does_not_reveal_hidden_changes_or_unknown_terrain() {
    let mut run = remembered_room(0.);
    let unknown = 6 * 21 + 18;
    run.floor_mut().unwrap().seen[unknown] = false;
    run.floor_mut().unwrap().remembered_tiles.as_mut().unwrap()[unknown].clear();
    let baseline = crate::observe::observe(&run);
    let old = sample(&baseline).clone();
    let observation = baseline.terrain_snapshot.as_ref().unwrap();
    assert!(!old.current && !old.direct && old.label == "chasm");
    assert!(observation.cells[unknown].tile.is_none());
    assert!(!baseline.cells.iter().any(|c| c.x == 18 && c.y == 6));
    let at = 6 * 21 + 10;
    run.floor_mut().unwrap().tiles[at] = "grass".into();
    run.floor_mut().unwrap().lights.fill(100.);
    let actor = corpse_core::population::create_actor_revision("rat", 901., 10., 6., false, 52.).unwrap();
    run.actors_at_mut(run.depth).push(actor);
    let before = serde_json::to_vec(&run).unwrap();
    let hidden = crate::observe::observe(&run);
    assert_eq!(sample(&hidden), &old, "hidden actual terrain/light/actors cannot recolor known terrain");
    assert!(!hidden.cells.iter().any(|c| c.actor_id == Some(901.)));
    assert!(hidden.terrain_snapshot.as_ref().unwrap().cells[unknown].tile.is_none());
    assert_eq!(serde_json::to_vec(&run).unwrap(), before);
    // Rendering a deserialized save uses the same redacted remembered data.
    let loaded: Run = serde_json::from_slice(&before).unwrap();
    assert_eq!(sample(&crate::observe::observe(&loaded)), &old);
}

#[test]
fn remembered_chasm_exemption_preserves_entities_fire_other_terrain_and_custom_profiles() {
    let base = Cell { label:"chasm".into(), glyph:"⛶".into(), fg:[12,12,17],
        bg:[0;3], current:false, direct:false, ..Default::default() };
    assert_eq!(crate::render::readable_cell_colors(&base), (base.fg, base.bg));
    for remembered in [false, true] {
        for player in [false, true] {
            let entity = crate::render::EntityInk { color:[23,23,48], player,
                telepathic:false, important:true, remembered, detected:false, identity_outline:None, plain_black:false };
            for actor in [false, true] {
                let mut cell = base.clone(); cell.fg = [23,23,48];
                cell.entity_ink = Some(entity); cell.actor_id = actor.then_some(901.);
                let with_chasm_label = crate::render::readable_cell_colors(&cell);
                cell.label = if actor { "Rat" } else { "Item" }.into();
                assert_eq!(crate::render::readable_cell_colors(&cell), with_chasm_label,
                    "chasm terrain exemption must not change actor/item ink policy");
                if !remembered { assert!(crate::color::contrast(with_chasm_label.0, with_chasm_label.1) >= 3.75); }
            }
        }
    }
    let mut fire = base.clone(); fire.fire = true; fire.fg = [240, 100, 30];
    let painted_fire = crate::render::readable_cell_colors(&fire);
    fire.label = "Fire".into();
    assert_eq!(crate::render::readable_cell_colors(&fire), painted_fire);
    for (label, floor) in [("floor", 54), ("wall", 70), ("water", 64), ("pit", 64)] {
        let mut cell = base.clone(); cell.label = label.into();
        let (fg, bg) = crate::render::readable_cell_colors(&cell);
        assert_eq!(fg, [floor;3], "{label}: memory readability unchanged");
        assert_eq!(bg, [if label == "wall" {13} else {7};3]);
    }
    let run = remembered_room(100.);
    let default = crate::observe::observe(&run);
    for (key,value) in [("terrain.chasm.color",serde_json::json!("#eebb77")),
        ("terrain.chasm.background",serde_json::json!("#152739")),
        ("terrain.chasm.opacity",serde_json::json!(0.25)),
        ("fog.rememberedBrightness",serde_json::json!(0.25)),
        ("global.exposure",serde_json::json!(0.5))] {
        let profile = Profile::default().set(key,value).unwrap();
        let view = crate::observe::observe_with_profile(&run, &profile);
        let custom = sample(&view);
        assert_eq!(crate::render::readable_cell_colors(custom), (custom.fg, custom.bg), "{key}");
        assert_ne!((custom.fg,custom.bg), (sample(&default).fg,sample(&default).bg), "{key}");
    }
}

#[test]
fn remembered_chasm_full_retained_tile_and_idle_paths_match_visibility_transitions() {
    use crate::render::{RetainedGame,tile_damage::TilePlanner};
    let mut full = Painter::new(); let mut retained = Painter::new(); let mut tiled = Painter::new();
    for p in [&mut full, &mut retained, &mut tiled] { p.resize(1280,800); }
    let mut cache = RetainedGame::default(); let mut planner = TilePlanner::default(); let mut previous = None;
    for (level, remembered) in [(0.,false), (0.,true), (100.,true), (100.,false), (-100.,false), (-100.,true), (0.,false)] {
        let run = if remembered { remembered_room(level) } else { room(level) };
        let before = serde_json::to_vec(&run).unwrap();
        let view = crate::observe::observe(&run);
        for now in [0.,731.] {
            full.game_at(&view,"",false,now);
            cache.update(&mut retained,&view,"",now,true,previous.is_none()).unwrap();
            assert_eq!(retained.pixels,full.pixels,"retained at {level}/{remembered}/{now}");
            planner.replace_view(previous.as_ref(),&view,&tiled); planner.finish_damage(&mut tiled);
            planner.paint_base(&mut tiled,&view,"",now); tiled.tile_clip_enabled=false; planner.clear_frame();
            assert_eq!(tiled.pixels,full.pixels,"tiled at {level}/{remembered}/{now}");
        }
        assert_eq!(serde_json::to_vec(&run).unwrap(), before);
        previous = Some(view);
    }
}
