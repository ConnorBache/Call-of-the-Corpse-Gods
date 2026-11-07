// Diagnostic-only: existing nested spans around the real post-command paint.
// No new per-tile/per-lookup clocks. Collector setup/extraction and proofs excluded.
#[cfg(feature="runtime-timing")]
#[test]
#[ignore="lead-controlled n5 paint attribution, collector off/on; not headline production timing"]
fn timeweave_tile_planning_attribution(){
    let options=Options::read();let repeats:usize=std::env::var("TILE_PLAN_DIAGNOSTIC_REPEATS").unwrap_or_else(|_|"5".into()).parse().unwrap();assert!((1..=20).contains(&repeats));
    let dir=Sandbox::new(&options.root,"tile-paint-attribution");let mut rows=Vec::new();
    for repeat in 1..=repeats{
        let mut parity:std::collections::BTreeMap<usize,(String,serde_json::Value)>=Default::default();
        // Alternate collector order. Both traces start from the same authored Run.
        for enabled in if repeat%2==0{[true,false]}else{[false,true]} {
            corpse_core::timing::set_enabled(false);corpse_core::timing::reset();
            let mut engine=temporal::Engine::new(animated_weave_room()).unwrap();
            let path=dir.0.join(format!("trace-{repeat}-{enabled}.save"));let mut auto=session_store::Autosave::default();auto.flush_tracked(&path,&engine,session_store::Kind::Fixture,false).unwrap();let mut ui=Ui::new(&engine.run);ui.diagnostics=true;
            assert!(animation_counts(ui.game.as_ref().unwrap()).0>0);
            for(step,(dx,took_turn))in [(1.,true),(-1.,false)].into_iter().enumerate(){
                let now=(step+1)as f64*300.;let out=dispatch(&mut engine,&mut ui.effects,now,&Command::Move{dx,dy:0.}).unwrap();assert_eq!(out.took_turn,took_turn);
                auto.flush_tracked(&path,&engine,session_store::Kind::Fixture,false).unwrap();
                let game=observe::observe_with_profile(&engine.run,&ui.painter.profile);assert!(animation_counts(&game).0>0);
                // The natural preceding command+flush+view cache state is retained.
                // Only paint comparison/index/masks/raster/upload-plan/drop is timed.
                corpse_core::timing::reset();corpse_core::timing::set_enabled(enabled);
                let wall=Instant::now();{
                    let _total=corpse_core::timing::span("desktop.tile_pipeline_total");
                    ui.paint_game(game,now);std::hint::black_box(&ui.painter.pixels);
                }let wall_ns=wall.elapsed().as_nanos();
                corpse_core::timing::set_enabled(false);
                let spans=corpse_core::timing::take_report();let counts=corpse_core::timing::take_counts();
                if enabled{let root=spans.get("desktop.tile_pipeline_total").unwrap();assert!(root.inclusive_ns<=wall_ns);let exclusive_sum:u128=spans.values().map(|s|s.exclusive_ns).sum();assert_eq!(exclusive_sum,root.inclusive_ns,"exclusive descendants partition this one root; do not sum inclusive children");}else{assert!(spans.is_empty());}
                ui.prove_pixels(now);reload_proof(&engine,&path);
                let state=engine.save().unwrap();let events=serde_json::Value::Array(ui.effects.cues.iter().map(|c|serde_json::json!({"event":c.event,"started_bits":c.started.to_bits(),"held":c.held})).collect());
                if let Some((old_state,old_events))=parity.get(&step){assert!(old_state==&state,"collector changes gameplay/save");assert_eq!(old_events,&events,"collector changes emitted events");}else{parity.insert(step,(state,events));}
                rows.push(ui.paint_metadata(serde_json::json!({"case":if took_turn{"animated-forward"}else{"animated-freeback"},"repeat":repeat,"collector_enabled":enabled,"paint_wall_ns":wall_ns,"spans":spans,"counts":counts,"proof_performed":true,"counter_diagnostics_on_in_both_collector_arms":true}),true));
            }
        }
    }
    let value=serde_json::json!({"schema":1,"suite":"tile-paint-attribution","repeats":repeats,"scope":"Post-command real forward/freeback paint only, including old View drop. Natural command/flush/observe cache setup precedes timing. Collector reset/enable before timer; disabled/report taken immediately after timer; pixels/save/events parity afterward. Existing nested inclusive/exclusive spans only; index is nested under compare; repeated span_prepare calls accumulated. Sum exclusive labels once, never sum inclusive parent+children. Counter diagnostics enabled in both collector arms. Caller supplies actual build profile; opt1 numbers are diagnostic, not production speed.","rows":rows});
    let out=options.root.join("tile-paint-attribution.json");assert!(!out.exists());std::fs::write(&out,serde_json::to_vec_pretty(&value).unwrap()).unwrap();println!("TILE_ATTRIBUTION_REPORT {}",out.display());
}
