//! Controlled full-window visual QA. Include beside timeweave_ui.rs only for tests.
#[cfg(test)]
mod scene_qa {
    use corpse_core::{Command, Field, Run, temporal::Engine};
    use std::{path::Path, sync::Arc};

    /// O(fixture size): deterministic, admitted control fixture. Original floor,
    /// lights, seen/remembered cells and perception are not enhanced for the image.
    fn controlled_room()->Run {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=41.0.into();
        r.player.x=12.;r.player.y=14.;
        r.player.abilities=vec![corpse_core::progression::WEAVE_ID.into()];
        r.player.active.clear();r.player.conditions.clear();
        r.offers.clear();r.replay.clear();r.history.clear();r.future=None;
        r.haste_snapshot=Field::Missing;r.phase=Field::Missing;r.timeweave=Field::Missing;
        r.replay_snapshot_from=Field::Missing;
        for actors in r.actors.values_mut(){*actors=Arc::new(vec![]);}
        for items in r.ground.values_mut(){*items=Arc::new(vec![]);}
        let f=r.floor_mut().unwrap();
        f.environment=Field::Missing;f.doors=Field::Missing;f.vents=Field::Missing;
        f.currents=Field::Missing;f.mud=Field::Missing;f.slime=Field::Missing;f.roots=Field::Missing;
        // The authored room already has floor on the intended path; do not carve
        // or reveal terrain merely to make a screenshot pass.
        let w=f.width();for y in 14..=17 {for x in 12..=17 {assert_eq!(f.tiles[y*w+x],"floor");}}
        r
    }
    /// O(native output pixels), actual game paint including unchanged HUD/layout.
    fn screenshot(e:&Engine, destination:&Path, scale:f64)->usize {
        let view=crate::observe::observe(&e.run);
        let visible=view.cells.iter().take(view.terrain_count).filter(|c|c.timeweave.is_some()).count();
        assert!(visible>0,"the real observation must contain at least one directly visible Timeweave");
        for c in view.cells.iter().filter(|c|c.timeweave.is_some()) {assert!(c.direct&&c.current);}
        let mut painter=crate::render::Painter::new();
        painter.resize((crate::render::WIDTH as f64*scale) as usize,(crate::render::HEIGHT as f64*scale) as usize);
        painter.game_at(&view,"Controlled QA: 12 ordinary steps, then one free backward step. NPCs/items removed; no reveal.",false,0.);
        let file=std::fs::File::create(destination).unwrap();
        let mut encoder=png::Encoder::new(file,painter.width as u32,painter.height as u32);
        encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(&painter.pixels).unwrap();
        visible
    }
    /// O(12 normal turns + one actual free command + three software paints).
    /// Ignored so ordinary gates never create screenshots or add this QA workload.
    #[test]
    #[ignore = "explicit full-scene screenshot QA; set CORPSE_TIMEWEAVE_SCENE_REVIEW"]
    fn actual_walked_path_and_free_backstep_full_window() {
        let directory=std::env::var("CORPSE_TIMEWEAVE_SCENE_REVIEW").expect("set an existing workspace reports/timeweave directory");
        let directory=Path::new(&directory);assert!(directory.is_dir());
        let initial=controlled_room();corpse_core::storage::validate(&initial).unwrap();
        corpse_core::engine::validate_supported(&initial).unwrap();
        let mut engine=Engine::new(initial).unwrap();let initial_tick=engine.run.tick;
        std::fs::write(directory.join("timeweave-scene-initial.json"),engine.save().unwrap()).unwrap();
        let commands=[(1.,0.);5].into_iter().chain([(0.,1.);3]).chain([(-1.,0.);4]);
        for (dx,dy) in commands {
            let before=engine.run.tick;let outcome=engine.dispatch(&Command::Move{dx,dy}).unwrap();
            assert!(outcome.took_turn);assert_eq!(engine.run.tick,before+1.);
        }
        assert_eq!((engine.run.player.x,engine.run.player.y),(13.,17.));
        assert_eq!(engine.run.tick,initial_tick+12.);
        assert_eq!(engine.run.timeweave.as_ref().unwrap().len(),12);
        let before_visible=screenshot(&engine,&directory.join("timeweave-scene-before-backstep-1x.png"),1.);
        let tick=engine.run.tick;let rng=engine.run.rng;let trail=engine.run.timeweave.clone();
        let outcome=engine.dispatch(&Command::Move{dx:1.,dy:0.}).unwrap();
        assert!(outcome.changed&&!outcome.took_turn);assert_eq!(engine.run.tick,tick);assert_eq!(engine.run.rng,rng);
        assert_eq!((engine.run.player.x,engine.run.player.y),(14.,17.));assert_eq!(engine.run.timeweave,trail);
        assert!(engine.history.frames.is_empty());assert!(engine.history.pending_haste.is_none());
        let after_visible=screenshot(&engine,&directory.join("timeweave-scene-after-backstep-1x.png"),1.);
        screenshot(&engine,&directory.join("timeweave-scene-after-backstep-1_5x.png"),1.5);
        std::fs::write(directory.join("timeweave-scene-after-free.json"),engine.save().unwrap()).unwrap();
        let receipt=serde_json::json!({
            "fixture":"avatar-room-v1, controlled NPC/ground/hazard removal; authored terrain/light/perception retained",
            "revision":41,"ordinary_moves":12,"free_backward_moves":1,"trail_segments":12,
            "tick_before_free":tick,"tick_after_free":engine.run.tick,"rng_unchanged_on_free":true,
            "visible_arrows_before":before_visible,"visible_arrows_after":after_visible,
            "initial_position":[12,14],"before_free_position":[13,17],"after_free_position":[14,17],
            "reveal_or_visibility_override":false,"windows_runtime_tested":false,
            "left_rail_layout":"unchanged; pending separate user decision",
            "pngs":["timeweave-scene-before-backstep-1x.png","timeweave-scene-after-backstep-1x.png","timeweave-scene-after-backstep-1_5x.png"]
        });
        std::fs::write(directory.join("timeweave-scene-receipt.json"),serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    }
}
