//! Observation-only Dreamwalking ink. This layer never queries the world or RNG.
use crate::render::{Cell, Rgb};

pub const NORMAL_INK: Rgb = [40, 75, 156];
const NORMAL_GROUND: Rgb = [4, 10, 29];

/// Borrow ordinary views; materialize only an active projection's detached layer.
/// The core clone shares floor cells, but metadata/history cloning retains its
/// existing Run-clone cost; callers reuse this value through nested observations.
pub fn observation_run(run: &corpse_core::Run) -> std::borrow::Cow<'_, corpse_core::Run> {
    if corpse_core::dreamwalking::active(run) && !corpse_core::dreamwalking::context_dream(run) {
        std::borrow::Cow::Owned(corpse_core::dreamwalking::observation_run(run))
    } else { std::borrow::Cow::Borrowed(run) }
}

/// Quest shards and tagged dropped weapons belong to the dream plane, including
/// a dream Half Here viewpoint. Waking items remain observable from dreams.
pub fn item_visible(run: &corpse_core::Run, item: &corpse_core::Item) -> bool {
    corpse_core::dreamwalking::item_visible(run, item)
}

/// O(1 + U + E) for a cell's bounded underlays U and four edges E. Only already
/// observed cells enter this transform; frame time is purely cosmetic. Original
/// material/creature colors, lighting and user palettes cannot defeat the override.
pub fn override_cell(cell: &mut Cell, now: f64, seed: u32) {
    let phase = crate::color::cosmetic_hash(cell.x as i32, cell.y as i32, seed)
        * std::f64::consts::TAU;
    let flicker = if now <= 0. { 1. } else {
        0.78 + 0.22 * (0.5 + 0.5 * (now / 137. + phase).sin()
            * (now / 307. + phase * 0.7).sin())
    };
    let ink = NORMAL_INK.map(|n| (n as f64 * flicker).round() as u8);
    let ground = NORMAL_GROUND.map(|n| (n as f64 * flicker).round() as u8);
    cell.bg = ground;
    cell.material_background = cell.material_background.map(|_| ground);
    cell.background_overlay = cell.background_overlay.map(|(_, opacity)| (ground, opacity));
    if cell.dream_entity {
        return;
    }
    cell.fg = ink;
    if let Some(entity) = cell.entity_ink.as_mut() { entity.color = ink; }
    for (_, color, _) in &mut cell.underlays { *color = ink; }
    for color in cell.edges.iter_mut().flatten() { *color = ink; }
    // Fire, water, gas and torch effects otherwise repaint their authored hues.
    cell.animations.clear();
    cell.pulse_speed = 0.;
    cell.fire_flicker = false;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn override_replaces_every_normal_world_color_but_preserves_identity() {
        let mut cell = Cell { x: 3, y: 4, glyph: "r".into(), label: "Rat".into(),
            fg: [255, 0, 0], bg: [0, 255, 0], material_background: Some([255, 255, 0]),
            underlays: vec![("~".into(), [255, 0, 255], 0.4)],
            edges: [Some([0, 255, 0]); 4],
            animations: vec![crate::render::Animation::Torch],
            pulse_speed: 12., ..Cell::default() };
        override_cell(&mut cell, 0., 12);
        assert_eq!((&cell.glyph, &cell.label), (&"r".to_string(), &"Rat".to_string()));
        assert_eq!(cell.fg, NORMAL_INK); assert_eq!(cell.bg, NORMAL_GROUND);
        assert_eq!(cell.underlays[0].1, NORMAL_INK);
        assert!(cell.edges.iter().flatten().all(|c| *c == NORMAL_INK));
        assert!(cell.animations.is_empty()); assert_eq!(cell.pulse_speed, 0.);
    }
    #[test]
    fn cosmetic_flicker_is_repeatable_and_exempts_dream_entities() {
        let original = Cell { x: 7, y: 6, fg: [244, 225, 150], dream_entity: false,
            ..Cell::default() };
        let mut a = original.clone(); let mut b = original.clone();
        override_cell(&mut a, 421., 77); override_cell(&mut b, 421., 77);
        assert_eq!(a.fg, b.fg); assert_eq!(a.bg, b.bg);
        override_cell(&mut b, 824., 77); assert_ne!(a.fg, b.fg);
        let mut dream = Cell { dream_entity: true, ..original.clone() };
        override_cell(&mut dream, 421., 77); assert_eq!(dream.fg, original.fg);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use corpse_core::{Actor, Floor, GroundItem, Run};
    use std::sync::Arc;

    fn room(dreaming: bool) -> Run {
        let mut r = corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap().run;
        r.simulation_revision = 34.0.into(); r.player.x = 4.; r.player.y = 4.;
        r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
        r.offers.clear(); r.status = "playing".into(); r.depth = 1.; r.next_id = 40.;
        r.inventory.clear(); r.ground.clear(); r.actors.clear(); r.detected_items = Default::default();
        r.remembered_items = Default::default();
        let w = 79; let h = 29; let mut tiles = vec!["floor".into(); w*h];
        for y in 0..h { for x in 0..w {
            if x == 0 || y == 0 || x == w-1 || y == h-1 { tiles[y*w+x] = "wall".into(); }
            else if x > 13 && x < 22 && y > 4 && y < 12 { tiles[y*w+x] = "water".into(); }
        }}
        r.floors = vec![Arc::new(Floor { width: (w as f64).into(), height: (h as f64).into(), depth: 1.,
            domain: "Dungeon".into(), tiles, lights: vec![100.;w*h], seen: vec![true;w*h], ..Default::default() })];
        let rat = corpse_core::population::create_actor_revision("rat", 10., 8., 4., false, 34.).unwrap();
        let nightmare = corpse_core::phantasmii_generation::nightmare(1, 11., 8., 4.);
        r.actors_at_mut(1.).extend([rat, nightmare]);
        r.ground.insert("1".into(), Arc::new(vec![GroundItem { x: 10., y: 4.,
            item: corpse_core::phantasmii_generation::shard(20.), ..Default::default() }]));
        if dreaming { enter(&mut r); }
        r
    }
    fn enter(r: &mut Run) {
        let mut body = r.player.clone(); body.id = 12.; body.state = "ally".into();
        body.name = "Your sleeping body".into(); body.conditions.insert("phantasmiiBody".into(), 1.);
        body.conditions.insert("halfHereOwner".into(), r.player.id);
        r.actors_at_mut(1.).push(body);
        r.dreamwalking = corpse_core::dreamwalking::State { active: true, depth: 1., body_id: Some(12.),
            anchor_x: r.player.x, anchor_y: r.player.y, ..Default::default() }.into();
        r.player.x = 5.;
    }
    #[test]
    fn dream_ui_awake_hides_nightmares_and_shards_without_changing_state() {
        let r = room(false); let before = r.clone(); let view = crate::observe::observe(&r);
        assert!(!view.dreamwalking);
        assert!(!view.cells.iter().any(|c| c.actor_id == Some(11.) || c.label.contains("Shards of")));
        assert!(!crate::inspection_ui::observe_tile(&r, 10, 4).scene.contains("Shards of"));
        assert_eq!(crate::inspection_ui::observe_tile(&r, 8, 4).title, "Giant Rat");
        assert!(crate::ability_ui::visible_abilities(&r).is_empty());
        assert_eq!(r, before);
    }
    #[test]
    fn phantasmii_floor_shards_are_hidden_from_waking_map_list_and_inspector() {
        let mut r = room(false);
        r.simulation_revision = 52.0.into();
        r.dream_shards_from = 0usize.into();
        let shard = corpse_core::phantasmii_generation::floor_shard(23.);
        r.ground_at_mut(1.).push(GroundItem { x: 9., y: 4., item: shard.clone(), ..Default::default() });
        r.remembered_items = serde_json::from_value(serde_json::json!({"1":[{"x":9,"y":4,"item":shard}]})).unwrap();
        r.detected_items = serde_json::from_value(serde_json::json!({"1":[{"x":9,"y":4,"item":shard}]})).unwrap();
        for in_dream in [false, true, false] {
            if in_dream { enter(&mut r); }
            else if corpse_core::dreamwalking::active(&r) { assert!(corpse_core::dreamwalking::wake(&mut r)); }
            let view = crate::observe::observe(&r);
            assert_eq!(view.cells.iter().any(|c| c.label == shard.name && c.dream_entity), in_dream);
            assert_eq!(view.ground_entities.iter().any(|e| e.name == shard.name), in_dream);
            assert_eq!(crate::inspection_ui::observe_tile(&r,9,4).scene.contains(&shard.name), in_dream);
        }
    }
    #[test]
    fn borge_shard_r39_dream_ground_visibility_inspection_and_ink() {
        let mut r=room(false);r.simulation_revision=39.0.into();
        let mut shard=corpse_core::borge::shard(21.);shard.dream_ground=true.into();
        r.ground_at_mut(1.).push(GroundItem{x:9.,y:4.,item:shard.clone(),..Default::default()});
        let before=r.clone();
        assert!(!item_visible(&r,&shard));
        assert!(!crate::observe::observe(&r).cells.iter().any(|c|c.label==shard.name));
        assert!(!crate::inspection_ui::observe_tile(&r,9,4).scene.contains(&shard.name));
        assert_eq!(r,before);
        // Remembered/detected snapshots must not leak the hidden weapon either.
        r.detected_items=serde_json::from_value(serde_json::json!({"1":[{"x":9,"y":4,"item":shard}]})).unwrap();
        r.remembered_items=serde_json::from_value(serde_json::json!({"1":[{"x":9,"y":4,"item":shard}]})).unwrap();
        assert!(!crate::observe::observe(&r).cells.iter().any(|c|c.label==shard.name));
        assert!(!crate::inspection_ui::observe_tile(&r,9,4).scene.contains(&shard.name));
        enter(&mut r);
        let waking=corpse_core::equipment::make_equipment("Shortsword",22.).unwrap();
        r.ground_at_mut(1.).push(GroundItem{x:7.,y:4.,item:waking.clone(),..Default::default()});
        let before=r.clone();let view=crate::observe::observe(&r);
        assert!(item_visible(&r,&shard));
        assert!(view.cells.iter().any(|c|c.label==shard.name && c.dream_entity));
        assert!(view.cells.iter().any(|c|c.label==waking.name && !c.dream_entity && c.fg==NORMAL_INK));
        assert!(crate::inspection_ui::observe_tile(&r,9,4).scene.contains(&shard.name));
        assert_eq!(r,before);
    }
    #[test]
    fn dream_ui_plane_overlap_statuses_row_and_body_are_consistent() {
        let r = room(true); let before = r.clone(); let view = crate::observe::observe(&r);
        assert!(view.dreamwalking);
        let occupants: Vec<_> = view.cells.iter().filter(|c| c.x == 8 && c.y == 4 && c.actor_id.is_some()).collect();
        assert_eq!(occupants.len(), 2); assert_eq!(occupants.last().unwrap().actor_id, Some(11.));
        assert!(view.cells.iter().filter(|c| !c.dream_entity).all(|c| c.fg == NORMAL_INK));
        assert_eq!(view.creatures.iter().find(|c|c.id==11.).unwrap().state, "In mind");
        assert_eq!(view.creatures.iter().find(|c|c.id==12.).unwrap().state, "Sleeping");
        assert_eq!(view.player_statuses.iter().take(2).map(|s|s.name.as_str()).collect::<Vec<_>>(), ["Dreamwalking", "Flying"]);
        let report = crate::inspection_ui::observe_tile(&r, 8, 4);
        let detail = report.creature.unwrap();
        assert_eq!(detail.behavior, "In mind");
        assert!(detail.description.contains("In the dream world,") && detail.description.contains("In reality, a giant rat"), "{}", detail.description);
        let wake = view.pack_hud.items.iter().find(|row| row.source == crate::game_hud::QuickSource::Dreamwalk).unwrap();
        assert_eq!(wake.label, "Wake Up");
        assert!(matches!(crate::game_hud::activate_quick(&r, wake.source),
            crate::pack::PackAction::Command(corpse_core::Command::Ability { id, target, .. })
                if id == corpse_core::dreamwalking::WAKE_UP_ID && target.is_missing()));
        assert!(view.cells.iter().any(|c| c.label == "Shards of the Phantasmii" && c.dream_entity));
        assert_eq!(r, before, "observation must never mutate simulation or consume RNG");
    }
    #[test]
    fn dream_ui_overlap_preserves_disguise_and_override_beats_custom_palette() {
        let mut r=room(true);
        r.actors_at_mut(1.)[0]=corpse_core::population::create_actor_revision("skin-walker",10.,8.,4.,false,34.).unwrap();
        let shown=crate::creature_presentation::project(&r,&r.actors_at(1.)[0]).unwrap();
        let report=crate::inspection_ui::observe_tile(&r,8,4);
        assert!(report.scene.to_lowercase().contains(&shown.name.to_lowercase()));assert!(!report.scene.contains("Skin Walker"));
        let profile=crate::appearance_profile::Profile::default().set("terrain.floor.color",serde_json::json!("#ff0000")).unwrap();
        let view=crate::observe::observe_with_profile(&r,&profile);
        assert!(view.cells.iter().filter(|c|!c.dream_entity).all(|c|c.fg==NORMAL_INK));
    }
    #[test]
    fn dream_ui_permanent_toggle_replaces_temporary_row_without_duplicates() {
        let mut r = room(true); r.dreamwalking.as_mut().unwrap().unlocked = true;
        r.player.abilities.push(corpse_core::dreamwalking::DREAMWALK_ID.into());
        let rows = crate::ability_ui::visible_abilities(&r);
        assert_eq!(rows.len(), 1); assert_eq!(rows[0].name, "Dreamwalk"); assert_eq!(rows[0].state, "ON");
        assert_eq!(crate::ability_ui::observe(&r, &rows[0].id).name, rows[0].name);
        assert!(crate::ability_ui::deactivates_without_target(&r, &rows[0].id));
        corpse_core::dreamwalking::wake(&mut r);
        let rows = crate::ability_ui::visible_abilities(&r);
        assert_eq!(rows.len(), 1); assert_eq!(rows[0].name, "Dreamwalk"); assert_eq!(rows[0].state, "OFF");
    }
    #[test]
    fn dream_ui_map_inspection_path_and_targeting_use_the_detached_dream_layer() {
        let mut r=room(true);corpse_core::dreamwalking::ensure_layer(&mut r);
        Arc::make_mut(&mut r.dreamwalking.as_mut().unwrap().floors[0]).tiles[4*79+6]="wall".into();
        let before=r.clone();let view=crate::observe::observe(&r);
        assert_eq!(view.terrain_snapshot.as_ref().unwrap().cells[4*79+6].tile.as_deref(),Some("wall"));
        assert!(crate::inspection_ui::observe_tile(&r,6,4).scene.to_lowercase().contains("wall"));
        assert_eq!(crate::targeting_ui::observe(&r,Some(crate::targeting_ui::Point{x:7,y:4})).stop_reason,
            crate::targeting_ui::StopReason::Terrain);
        assert!(crate::travel_ui::path_to(&r,7,4,false).iter().all(|s|s.x!=6||s.y!=4));
        assert_eq!(r.floor().unwrap().tiles[4*79+6],"floor");assert_eq!(r,before);
    }
    #[test]
    fn dream_ui_nightmare_sensing_does_not_reveal_remote_terrain_or_hidden_normal_creatures() {
        let mut r = room(true);
        r.floor_mut().unwrap().seen.fill(false);
        r.player.stats.perception = 0.;
        for y in 1..28 { r.floor_mut().unwrap().tiles[y*79+20] = "wall".into(); }
        for a in r.actors_at_mut(1.).iter_mut().filter(|a| a.id == 10. || a.id == 11.) { a.x = 60.; a.y = 20.; }
        let view = crate::observe::observe(&r);
        assert!(view.cells.iter().any(|c|c.actor_id==Some(11.)));
        assert!(!view.cells.iter().any(|c|c.actor_id==Some(10.)));
        assert!(view.terrain_snapshot.as_ref().unwrap().cells[20*79+60].tile.is_none());
        assert!(!crate::inspection_ui::observe_tile(&r,60,20).scene.contains("Rat"));
    }
    #[test]
    fn dream_ui_half_here_retains_its_dream_perspective_after_waking() {
        let mut r = room(false);
        let mut clone: Actor = r.player.clone(); clone.id = 30.; clone.x=6.;
        clone.conditions.insert("halfHereClone".into(),1.); clone.conditions.insert("halfHereOwner".into(),r.player.id);
        clone.conditions.insert("phantasmiiPlane".into(),1.);
        r.player.abilities.push(corpse_core::half_here::ID.into()); r.player.active.push(corpse_core::half_here::ID.into());
        r.player.conditions.insert("halfHereId".into(),30.); r.actors_at_mut(1.).push(clone);
        let view = crate::observe::observe(&r);
        assert!(!view.dreamwalking);
        assert_eq!(view.creatures.iter().find(|a| a.id==11.).unwrap().state,"In mind");
        assert!(view.cells.iter().any(|c|c.label=="Shards of the Phantasmii"));
        let report = crate::inspection_ui::observe_tile(&r,8,4).creature.unwrap();
        assert!(report.your_attack.contains("another plane"));
    }
    #[test]
    fn dream_ui_nightmares_are_excluded_from_bestiary_and_disguises() {
        let mut r = room(true); r.player.conditions.insert("hallucinating".into(),9.);
        let nightmare = r.actors_at(1.).iter().find(|a|a.id==11.).unwrap();
        assert!(crate::creature_presentation::project(&r, nightmare).is_none());
        assert!(crate::manual::Manual::new().catalog.entries.iter().all(|e| !e.name.contains("Nightmare")));
    }
    #[test]
    fn dream_ui_actual_renderer_flickers_without_mutation_and_exports_review() {
        let r = room(true); let before = r.clone();
        let mut view = crate::observe::observe(&r);
        view.measure_conditions(&mut crate::statuses_ui::ConditionMeters::default());
        let mut painter = crate::render::Painter::new();
        painter.game_at(&view,"",false,421.); let first = painter.pixels.clone();
        painter.game_at(&view,"",false,421.); assert_eq!(painter.pixels, first);
        painter.game_at(&view,"",false,824.); assert_ne!(painter.pixels, first);
        assert_eq!(r,before);
        if let Ok(directory) = std::env::var("CORPSE_DREAM_UI_REVIEW") {
            std::fs::create_dir_all(&directory).unwrap();
            for (name, time, inspect) in [("dreamwalking",0.,false),("dreamwalking-flicker",824.,false),("nightmare-overlap",0.,true)] {
                painter.game_at(&view,"",false,time);
                if inspect { crate::inspection_ui::paint(&crate::inspection_ui::observe_tile(&r,8,4),&mut painter,0); }
                let file=std::fs::File::create(format!("{directory}/{name}.png")).unwrap();
                let mut encoder=png::Encoder::new(file,painter.width as u32,painter.height as u32);
                encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
                encoder.write_header().unwrap().write_image_data(&painter.pixels).unwrap();
            }
        }
    }
}
