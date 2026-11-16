//! R50/R51/R52 potion command -> detached observation -> native pixels. No SDL loop.
use crate::{appearance_profile::Profile, render::{Painter, RetainedGame, View, tile_damage}};
use corpse_core::{Command, Field, Floor, FloorLightSourcesEntry, Item, Run};
use std::sync::Arc;

fn room(revision: f64, ambient: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into();
    r.player.x = 8.; r.player.y = 6.;
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.ability_order = Field::Missing; r.player.equipment_abilities = Field::Missing;
    r.player.stats.perception = 300.;
    r.inventory.clear(); r.actors = Default::default(); r.ground.clear(); r.history.clear();
    r.floors = vec![Arc::new(Floor { depth: r.depth, width: 21.0.into(), height: 21.0.into(),
        tiles: vec!["floor".into(); 441], lights: vec![ambient; 441], seen: vec![true; 441],
        substrates: vec!["stone".into(); 441].into(), remembered_tiles: vec!["floor".into(); 441].into(),
        ..Default::default() })];
    r
}
fn throw(r: &mut Run, diluted: bool) {
    r.inventory.push(Item { id: 900., kind: "potion".into(), name: "Potion of Darkness".into(),
        glyph: "!".into(), effect: "darkness".to_owned().into(), diluted: diluted.into(), ..Default::default() });
    assert_eq!(corpse_core::projectiles::command(r, &Command::Throw { id: 900., x: 10., y: 6. }).unwrap(), Some(true));
    assert!(r.inventory.iter().all(|item| item.id != 900.));
}
fn capture(name: &str, p: &Painter) {
    let Ok(dir) = std::env::var("CORPSE_R51_DARKNESS_OUTPUT") else { return; };
    std::fs::create_dir_all(&dir).unwrap();
    let file = std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
    let mut encoder = png::Encoder::new(file, p.width as u32, p.height as u32);
    encoder.set_color(png::ColorType::Rgba); encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
}
fn paint(view: &View, name: &str) -> Painter {
    let mut p = Painter::new(); p.resize(1920, 1200); p.game_at(view, "", false, 0.);
    capture(name, &p); p
}
fn region(p: &Painter, v: &View, x: i32, y: i32) -> Vec<u8> {
    let (cw, ch, ox, oy) = p.map_layout(v.width, v.height);
    let rect = p.logical_damage_rect(ox + x * cw, oy + y * ch, cw, ch).unwrap();
    (rect.y..rect.bottom()).flat_map(|py| p.pixels[(py * p.width + rect.x) * 4..(py * p.width + rect.right()) * 4].iter().copied()).collect()
}

#[test]
fn darkness_native_throw_before_after_expiry_and_incremental_pixels() {
    for revision in [50., 51., 52.] {
        let profile = Profile::default();
        let mut r = room(revision, 100.);
        let before = crate::observe::observe_with_profile(&r, &profile);
        let before_p = paint(&before, &format!("r{revision}-before-dose"));
        let rng = r.rng;
        throw(&mut r, false);
        assert_eq!(r.rng, rng, "darkness never draws gameplay RNG");
        let frozen = serde_json::to_vec(&r).unwrap();
        let after = crate::observe::observe_with_profile(&r, &profile);
        let snapshot = after.terrain_snapshot.as_ref().unwrap();
        let kind=if revision>=52.{"potion-darkness-override"}else{"potion-darkness"};
        let source = snapshot.light_sources.iter().find(|s| s.kind == kind).unwrap();
        assert_eq!(source.cells.len(), if revision>=52.{29}else{25});
        assert!(source.strengths.iter().all(|v| *v == 50.));
        assert!(!source.origin_known && source.x == -1. && source.y == -1. && source.actor_id.is_none());
        let at = 6 * 21 + 10;
        assert_eq!(snapshot.cells[at].illumination, Some(if revision>=52.{-100.}else{50.}));
        assert_eq!(snapshot.cells[at].ambient, Some(100.), "raw ambient semantics stay unchanged");
        let (_, _, lights) = crate::observe::terrain_frame(snapshot, &profile, after.cosmetic_seed, 0.);
        assert_eq!(lights[at].ambient_attenuation, if revision>=52.{0.}else{0.75});
        if revision>=52. {assert_eq!(lights[at].display.unwrap().multiplier,0.05);}
        assert_eq!(lights[6 * 21 + 14].ambient_attenuation, 1.);
        for cell in after.cells.iter().take(after.terrain_count) {
            let old = before.cells.iter().find(|old| old.x == cell.x && old.y == cell.y).unwrap();
            let dx=cell.x as i32-10;let dy=cell.y as i32-6;
            if if revision>=52.{dx*dx+dy*dy<=9}else{dx.abs()<=2&&dy.abs()<=2} {
                assert!(crate::color_profile::luminance(cell.bg) < crate::color_profile::luminance(old.bg));
            } else { assert_eq!((cell.fg, cell.bg), (old.fg, old.bg), "unaffected ground"); }
        }
        let player = after.cells.iter().find(|c| c.actor_id == Some(r.player.id)).unwrap();
        let ground = after.cells.iter().take(after.terrain_count).find(|c| c.x == 8 && c.y == 6).unwrap();
        assert_eq!(player.bg, ground.bg, "self tile uses its actual terrain background");

        // Reconstruct precisely the former missing source payload. This is a
        // post-dose observation with the real 50 illumination, not a mock dose.
        let mut broken_snapshot = (**snapshot).clone();
        broken_snapshot.light_sources.retain(|s| s.kind != kind);
        let (old_ground, old_backgrounds, _) = crate::observe::terrain_frame(&broken_snapshot, &profile, after.cosmetic_seed, 0.);
        assert_eq!(old_ground, after.cells[..after.terrain_count], "calibration follows resolved illumination even without a darkness hue payload");
        let mut broken = crate::observe::observe_with_profile(&r, &profile);
        broken.cells[..broken.terrain_count].clone_from_slice(&old_ground);
        for entity in broken.cells.iter_mut().skip(broken.terrain_count) {
            entity.bg = old_backgrounds[entity.y * 21 + entity.x];
            if let Some(old) = before.cells.iter().skip(before.terrain_count).find(|old| old.actor_id == entity.actor_id) { entity.fg = old.fg; }
        }
        broken.terrain_snapshot = Some(Arc::new(broken_snapshot));
        let old_p = paint(&broken, &format!("r{revision}-post-dose-before-fix"));
        let after_p = paint(&after, &format!("r{revision}-post-dose-fixed"));
        assert_eq!(region(&after_p, &after, 10, 6), region(&old_p, &broken, 10, 6));
        assert_ne!(region(&before_p, &before, 10, 6), region(&after_p, &after, 10, 6));
        assert_eq!(region(&before_p, &before, 14, 6), region(&after_p, &after, 14, 6));

        let mut retained = RetainedGame::default();
        let mut retained_p = Painter::new(); retained_p.resize(1920, 1200);
        retained.update(&mut retained_p, &before, "", 0., true, true).unwrap();
        retained.update(&mut retained_p, &after, "", 0., true, false).unwrap();
        assert_eq!(retained_p.pixels, after_p.pixels, "retained repaint matches full native pixels");
        let mut tiled = Painter::new(); tiled.resize(1920, 1200);
        let mut planner = tile_damage::TilePlanner::default();
        planner.replace_view(None, &before, &tiled); planner.finish_damage(&mut tiled); planner.paint_base(&mut tiled, &before, "", 0.);
        tiled.tile_clip_enabled = false; planner.clear_frame();
        planner.replace_view(Some(&before), &after, &tiled); planner.finish_damage(&mut tiled); planner.paint_base(&mut tiled, &after, "", 0.);
        tiled.tile_clip_enabled = false;
        assert_eq!(tiled.pixels, after_p.pixels, "tile repaint matches full native pixels");
        assert_eq!(serde_json::to_vec(&r).unwrap(), frozen, "observation and painting never mutate saved state");

        r.tick += 20.; corpse_core::revision50_potions::expire(&mut r);
        let expired = crate::observe::observe(&r);
        assert_eq!(expired.cells[..expired.terrain_count], before.cells[..before.terrain_count]);
        let expired_p = paint(&expired, &format!("r{revision}-expired"));
        retained.update(&mut retained_p, &expired, "", 0., true, false).unwrap();
        assert_eq!(retained_p.pixels, expired_p.pixels, "expiry repaints the darkness away");
        assert_eq!(region(&before_p, &before, 10, 6), region(&expired_p, &expired, 10, 6));
    }
}

#[test]
fn darkness_materials_glow_dilution_profiles_and_ring_exclusion() {
    for ambient in [-30., 0., 100.] { for tile in ["floor", "wall", "grass", "water", "shallow-water"] {
        let mut r = room(51., ambient);
        let at = 6 * 21 + 10;
        r.floor_mut().unwrap().tiles[at] = tile.into();
        let custom = Profile::default().set(&format!("terrain.{tile}.background"), serde_json::json!("#725f46")).unwrap();
        for profile in [Profile::default(), custom] {
            let original = crate::observe::observe_with_profile(&r, &profile);
            let mut dark = r.clone();
            // Direct impact exercises material response even when a wall would
            // intercept a thrown projectile one cell earlier.
            dark.inventory.push(Item { id: 901., kind: "potion".into(), effect: "darkness".to_owned().into(), ..Default::default() });
            assert!(corpse_core::potions::apply(&mut dark, 0, 10., 6., true).unwrap());
            let observed = crate::observe::observe_with_profile(&dark, &profile);
            let a = original.cells.iter().take(original.terrain_count).find(|c| c.x == 10 && c.y == 6).unwrap();
            let b = observed.cells.iter().take(observed.terrain_count).find(|c| c.x == 10 && c.y == 6).unwrap();
            assert!(crate::color_profile::luminance(b.bg) < crate::color_profile::luminance(a.bg), "{tile} ambient={ambient}");
        }
    }}
    let mut r = room(51., 0.);
    r.floor_mut().unwrap().light_sources = vec![FloorLightSourcesEntry { x: 10., y: 6., power: 100., falloff: 10., kind: "torch".to_owned().into() }].into();
    let o = corpse_core::vision::observe_terrain(&r).unwrap();
    let initial = crate::appearance_light::light_field(&o, &Profile::default(), 0.);
    throw(&mut r, true);
    let o = corpse_core::vision::observe_terrain(&r).unwrap();
    let dark = crate::appearance_light::light_field(&o, &Profile::default(), 0.);
    let at = 6 * 21 + 10;
    assert_eq!(o.cells[at].illumination, Some(75.));
    assert_eq!(dark[at].ambient_attenuation, 0.875);
    for i in 0..3 { assert!(dark[at].energy[i] < initial[at].energy[i]); }
    r.tick += 10.; corpse_core::revision50_potions::expire(&mut r);
    assert!(!corpse_core::vision::observe_terrain(&r).unwrap().light_sources.iter().any(|s| s.kind == "potion-darkness"));

    let mut r = room(51., 100.);
    r.inventory.push(Item { id: 902., kind: "ring".into(), effect: "light".to_owned().into(), ..Default::default() });
    throw(&mut r, false);
    assert_eq!(corpse_core::vision::tile_light(&r, 8, 6), 50., "own Light ring still excludes its tile");
    assert_eq!(corpse_core::vision::tile_light(&r, 9, 6), 60., "ring still adds after potion darkness");
}

#[test]
fn darkness_r52_materials_sources_custom_profiles_and_dilution(){
    let at=6*21+10;
    for ambient in [-30.,0.,100.,250.] {for tile in ["floor","wall","grass","water","shallow-water"] {for diluted in [false,true] {
        let mut r=room(52.,ambient);r.floor_mut().unwrap().tiles[at]=tile.into();
        r.floor_mut().unwrap().light_sources=vec![FloorLightSourcesEntry{x:10.,y:6.,power:250.,falloff:10.,kind:"torch".to_owned().into()}].into();
        let raw=r.floor().unwrap().lights.clone();let sources=r.floor().unwrap().light_sources.clone();
        let custom=Profile::default().set("lighting.ambient",serde_json::json!(1.5)).unwrap()
            .set("lighting.sources.torch.intensity",serde_json::json!(2.)).unwrap()
            .set("lighting.visualRadius",serde_json::json!(2.)).unwrap()
            .set(&format!("terrain.{tile}.background"),serde_json::json!("#725f46")).unwrap();
        for profile in [Profile::default(),custom] {
            let before=crate::observe::observe_with_profile(&r,&profile);let mut dark=r.clone();
            dark.inventory.push(Item{id:901.,kind:"potion".into(),effect:"darkness".to_owned().into(),diluted:diluted.into(),..Default::default()});
            assert!(corpse_core::potions::apply(&mut dark,0,10.,6.,true).unwrap());
            let saved=serde_json::to_vec(&dark).unwrap();let rng=dark.rng;
            for time in [0.,400.,900.] {
                let observation=corpse_core::vision::observe_terrain(&dark).unwrap();
                assert_eq!(observation.cells[at].illumination,Some(-100.));
                // The inherited gameplay observation bounds its ambient sample;
                // saved floor/source values below must remain unmodified. The
                // separate unbounded-observation test covers display >100 tails.
                assert_eq!(observation.cells[at].ambient,Some(ambient.clamp(-100.,100.)));
                let lights=crate::appearance_light::light_field(&observation,&profile,time);
                assert_eq!(lights[at].display,Some(crate::color::light_ui::DisplayLight::neutral(-100.)));
                assert_eq!(lights[at].energy,[0.;3]);assert_eq!(lights[at].wall_energy,[0.;3]);
            }
            let after=crate::observe::observe_with_profile(&dark,&profile);
            let a=before.cells.iter().take(before.terrain_count).find(|c|c.x==10&&c.y==6).unwrap();
            let b=after.cells.iter().take(after.terrain_count).find(|c|c.x==10&&c.y==6).unwrap();
            assert!(crate::color_profile::luminance(b.bg)<crate::color_profile::luminance(a.bg),"{tile} ambient={ambient}");
            assert_eq!(serde_json::to_vec(&dark).unwrap(),saved);assert_eq!(dark.rng,rng);
            assert_eq!(dark.floor().unwrap().lights,raw);assert_eq!(dark.floor().unwrap().light_sources,sources);
            dark.tick+=if diluted{10.}else{20.};corpse_core::revision50_potions::expire(&mut dark);
            let expired=crate::observe::observe_with_profile(&dark,&profile);
            assert_eq!(expired.cells[..expired.terrain_count],before.cells[..before.terrain_count]);
        }
    }}}
}

#[test]
fn darkness_r52_zero_perception_uses_memory_not_live_hidden_illumination(){
    let mut r=room(52.,100.);r.player.stats.perception=0.;
    let before=crate::observe::observe(&r);let before_p=paint(&before,"r52-perception-zero-before-dose");
    let rng=r.rng;throw(&mut r,false);
    let saved=serde_json::to_vec(&r).unwrap();let after=crate::observe::observe(&r);
    let o=after.terrain_snapshot.as_ref().unwrap();let at=6*21+10;
    let mut affected=0;
    for y in 3usize..=9 {for x in 7usize..=13 {
        let dx=x as i32-10;let dy=y as i32-6;
        if dx*dx+dy*dy>9 {continue;} affected+=1;
        let cell=&o.cells[y*21+x];assert!(!cell.current&&!cell.direct&&cell.remembered);
        assert_eq!(cell.knowledge,"remembered");assert_eq!(cell.illumination,None);assert_eq!(cell.ambient,None);
    }}
    assert_eq!(affected,29);
    assert!(o.light_sources.iter().all(|source|source.kind!="potion-darkness-override"),"No live mask may cross into remembered terrain");
    assert_eq!(corpse_core::vision::tile_light(&r,10,6),-100.);
    let after_p=paint(&after,"r52-perception-zero-after-dose");
    let original=region(&before_p,&before,10,6);let remembered=region(&after_p,&after,10,6);
    let mean=|pixels:&[u8]|pixels.chunks_exact(4).map(|p|p[..3].iter().map(|v|*v as f64).sum::<f64>()).sum::<f64>()/(pixels.len()/4*3)as f64;
    assert_ne!(original,remembered,"The normal P=0 dose must visibly change the affected terrain");
    assert!(mean(&remembered)<mean(&original),"Remembered terrain is visibly dimmer than the previously lit terrain");
    assert_eq!(region(&before_p,&before,14,6),region(&after_p,&after,14,6));
    for (x,y) in [(13,6),(7,6),(10,3),(10,9),(12,8)] {
        assert_ne!(region(&before_p,&before,x,y),region(&after_p,&after,x,y),"Inclusive circle boundary changes native pixels at ({x},{y})");
    }
    for (x,y) in [(13,7),(13,9),(7,3),(14,6)] {
        assert_eq!(region(&before_p,&before,x,y),region(&after_p,&after,x,y),"Outside circle pixels remain unchanged at ({x},{y})");
    }
    assert_eq!(serde_json::to_vec(&r).unwrap(),saved);assert_eq!(r.rng,rng);
    let mut retained=RetainedGame::default();let mut retained_p=Painter::new();retained_p.resize(1920,1200);
    retained.update(&mut retained_p,&before,"",0.,true,true).unwrap();
    retained.update(&mut retained_p,&after,"",0.,true,false).unwrap();assert_eq!(retained_p.pixels,after_p.pixels);
    if let Ok(dir)=std::env::var("CORPSE_R51_DARKNESS_OUTPUT") {
        std::fs::write(format!("{dir}/r52-perception-zero-evidence.json"),serde_json::to_vec_pretty(&serde_json::json!({
            "perception":0,"radius":3,"distanceRule":"dx*dx+dy*dy<=9","affectedCells":29,"effectiveLightAfter":-100,"observedLightAfter":o.cells[at].illumination,
            "knowledgeAfter":o.cells[at].knowledge,"liveMaskProjected":false,
            "beforeCenterMeanRgb":mean(&original),"afterCenterMeanRgb":mean(&remembered),
            "beforeCenterCrc32":crc32fast::hash(&original),"afterCenterCrc32":crc32fast::hash(&remembered),
            "outsidePixelsUnchanged":true,"savedStateAndRngUnchangedByPainting":true,
            "nativeFrame":[1920,1200],"presentationMs":0})).unwrap()).unwrap();
    }
    r.tick+=20.;corpse_core::revision50_potions::expire(&mut r);
    let expired=crate::observe::observe(&r);let expired_p=paint(&expired,"r52-perception-zero-expired");
    assert_eq!(expired.cells[..expired.terrain_count],before.cells[..before.terrain_count]);
    assert_eq!(region(&before_p,&before,10,6),region(&expired_p,&expired,10,6));
}

#[derive(Debug, PartialEq)]
struct NativeCrop { width: usize, height: usize, pixels: Vec<u8> }
/// O(native crop pixels), copied without scaling, interpolation or recoloring.
fn native_crop(p: &Painter, v: &View, x: i32, y: i32, w: i32, h: i32) -> NativeCrop {
    let (cw, ch, ox, oy) = p.map_layout(v.width, v.height);
    let rect = p.logical_damage_rect(ox + x * cw, oy + y * ch, w * cw, h * ch).unwrap();
    NativeCrop { width: rect.w, height: rect.h,
        pixels: (rect.y..rect.bottom()).flat_map(|py| p.pixels[(py * p.width + rect.x) * 4..(py * p.width + rect.right()) * 4].iter().copied()).collect() }
}
fn place_native_crop(sheet: &mut Painter, crop: &NativeCrop, x: i32, y: i32) {
    let anchor = sheet.logical_damage_rect(x, y, 1, 1).unwrap();
    assert!(anchor.x + crop.width <= sheet.width && anchor.y + crop.height <= sheet.height);
    for row in 0..crop.height {
        let dest = ((anchor.y + row) * sheet.width + anchor.x) * 4;
        sheet.pixels[dest..dest + crop.width * 4].copy_from_slice(&crop.pixels[row * crop.width * 4..(row + 1) * crop.width * 4]);
        assert_eq!(&sheet.pixels[dest..dest + crop.width * 4], &crop.pixels[row * crop.width * 4..(row + 1) * crop.width * 4]);
    }
}
fn example_sheet(title: &str, subtitle: &str) -> Painter {
    let mut sheet = Painter::new(); sheet.resize(1920, 1200); sheet.set_responsive_canvas(true);
    sheet.clear([15, 16, 22]);
    sheet.text(title, 22, 20, 24, [239, 229, 211]);
    sheet.text(subtitle, 22, 54, 12, [180, 184, 195]);
    sheet
}

#[test]
fn illumination_native_examples_same_tiles_levels_sources_and_potion_expiry() {
    let materials = [("floor", "Floor"), ("wall", "Wall"), ("grass", "Grass"),
        ("water", "Deep water"), ("shallow-water", "Shallow water")];
    // The final pair deliberately has the same scalar illumination: emitter
    // color/energy, not just the number, also participates in appearance.
    let levels = [(-75., false), (-25., false), (0., false), (50., false),
        (100., false), (80., false), (80., true)];
    let mut levels_sheet = example_sheet("Illumination and tile appearance",
        "Same seed, position and animation time. Centre tile in each native 3 x 3 crop. No HUD or log.");
    let mut potion_sheet = example_sheet("Darkness Potion: before, after and expiry",
        "Same native 7 x 3 map crop. The real throw dispatcher applies the potion; expiry restores the original pixels.");
    for (column, (value, torch)) in levels.iter().enumerate() {
        let x = 145 + column as i32 * 155;
        levels_sheet.text(&format!("{value:.0}"), x, 88, 20, [239, 229, 211]);
        levels_sheet.text(if *torch { "torch light" } else { "ambient" }, x, 112, 11, [180, 184, 195]);
    }
    for (column, label) in ["Before: 100", "Potion active: 50", "Expired: 100"].iter().enumerate() {
        potion_sheet.text(label, 175 + column as i32 * 345, 94, 18, [239, 229, 211]);
    }
    let mut source = Painter::new(); source.resize(2560, 1600);
    let mut evidence = Vec::new();
    for (row, (tile, label)) in materials.iter().enumerate() {
        let y = 139 + row as i32 * 126;
        levels_sheet.text(label, 20, y + 37, 13, [210, 214, 225]);
        potion_sheet.text(label, 20, y + 37, 13, [210, 214, 225]);
        let mut cell_pixels = Vec::new();
        let mut backgrounds = Vec::new();
        for (column, (value, torch)) in levels.iter().copied().enumerate() {
            let mut r = room(51., if torch { 0. } else { value });
            let at = 6 * 21 + 10;
            r.floor_mut().unwrap().tiles[at] = (*tile).into();
            if torch {
                r.floor_mut().unwrap().light_sources = vec![FloorLightSourcesEntry {
                    x: 10., y: 4., power: 100., falloff: 10., kind: "torch".to_owned().into()
                }].into();
            }
            let frozen = serde_json::to_vec(&r).unwrap();
            let view = crate::observe::observe(&r);
            let o = &view.terrain_snapshot.as_ref().unwrap().cells[at];
            assert!(o.current && o.direct && o.direct_chance == 100., "sample visibility is held constant: {tile}/{value}");
            assert_eq!(o.illumination, Some(value));
            source.game_at(&view, "", false, 0.);
            let crop = native_crop(&source, &view, 9, 5, 3, 3);
            let center = region(&source, &view, 10, 6);
            let cell = view.cells.iter().take(view.terrain_count).find(|c| c.x == 10 && c.y == 6).unwrap();
            backgrounds.push(cell.bg);
            evidence.push(serde_json::json!({"sheet":"illumination-levels-native.png", "terrain":tile,
                "illumination":o.illumination,"ambient":o.ambient,"torch":torch,"directChance":o.direct_chance,
                "foreground":cell.fg,"background":cell.bg,"nativeCropWidth":crop.width,"nativeCropHeight":crop.height,
                "nativeCropRgbaCrc32":crc32fast::hash(&crop.pixels),"centerRgbaCrc32":crc32fast::hash(&center)}));
            cell_pixels.push(center);
            place_native_crop(&mut levels_sheet, &crop, 145 + column as i32 * 155, y);
            assert_eq!(serde_json::to_vec(&r).unwrap(), frozen, "level examples never mutate simulation");
        }
        assert!(crate::color_profile::luminance(backgrounds[0]) < crate::color_profile::luminance(backgrounds[2]), "negative illumination shades {tile}");
        assert_ne!(cell_pixels[2], cell_pixels[3], "{tile}: zero and +50 must differ");
        assert_ne!(cell_pixels[2], cell_pixels[4], "{tile}: zero and +100 must differ");
        assert_ne!(backgrounds[2], backgrounds[3]);
        assert_ne!(backgrounds[2], backgrounds[4]);
        assert!(cell_pixels[5] != cell_pixels[6], "equal scalar light still distinguishes real emitter glow on {tile}");

        let mut r = room(51., 100.); let at = 6 * 21 + 10;
        r.floor_mut().unwrap().tiles[at] = (*tile).into();
        let rng = r.rng;
        let mut crops = Vec::new(); let mut centers = Vec::new();
        for stage in 0..3 {
            if stage == 1 { throw(&mut r, false); }
            if stage == 2 { r.tick += 20.; corpse_core::revision50_potions::expire(&mut r); }
            let frozen = serde_json::to_vec(&r).unwrap();
            let view = crate::observe::observe(&r);
            let o = &view.terrain_snapshot.as_ref().unwrap().cells[at];
            assert_eq!(o.illumination, Some(if stage == 1 { 50. } else { 100. }));
            assert!(o.current && o.direct && o.direct_chance == 100.);
            source.game_at(&view, "", false, 0.);
            let crop = native_crop(&source, &view, 7, 5, 7, 3);
            let center = region(&source, &view, 10, 6);
            let cell = view.cells.iter().take(view.terrain_count).find(|c| c.x == 10 && c.y == 6).unwrap();
            evidence.push(serde_json::json!({"sheet":"darkness-potion-native-comparison.png", "terrain":tile,
                "stage":(["before","active","expired"][stage]),"illumination":o.illumination,"ambient":o.ambient,
                "foreground":cell.fg,"background":cell.bg,"nativeCropWidth":crop.width,"nativeCropHeight":crop.height,
                "nativeCropRgbaCrc32":crc32fast::hash(&crop.pixels),"centerRgbaCrc32":crc32fast::hash(&center)}));
            place_native_crop(&mut potion_sheet, &crop, 175 + stage as i32 * 345, y);
            crops.push(crop); centers.push(center);
            assert_eq!(serde_json::to_vec(&r).unwrap(), frozen, "capture never mutates potion state");
            assert_eq!(r.rng, rng, "examples consume no gameplay randomness");
        }
        assert!(centers[0] != centers[1], "active darkness changes actual {tile} pixels");
        assert!(centers[0] == centers[2], "expired {tile} pixels restore exactly");
        assert!(crops[0] == crops[2], "entire native crop restores, independent of tick/log/HUD changes");
    }
    levels_sheet.text("Last two columns both read 80: a nearby torch supplies a luminance-normalized warm tint. Samples are copied 1:1 from 2560 x 1600 game frames.", 22, 780, 11, [180, 184, 195]);
    potion_sheet.text("Only map pixels are compared. No rescaling, recoloring, mock tiles, or HUD/log changes are used as evidence.", 22, 780, 11, [180, 184, 195]);
    capture("illumination-levels-native", &levels_sheet);
    capture("darkness-potion-native-comparison", &potion_sheet);
    if let Ok(dir) = std::env::var("CORPSE_R51_DARKNESS_OUTPUT") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/native-example-evidence.json"), serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
}
