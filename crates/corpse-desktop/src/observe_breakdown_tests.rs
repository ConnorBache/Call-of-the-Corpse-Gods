// Synthetic observation-only population, not a generated encounter or AI timing.
// O(N log N + 32*actor creation + candidate visibility queries) SETUP, untimed.
#[cfg(feature="runtime-timing")]
fn observe_crowded32_fixture()->Run {
    let mut r=weave_room();
    let (w,h)={let f=r.floor().unwrap();(f.width(),f.height())};
    let px=r.player.x as usize;let py=r.player.y as usize;
    let mut positions:Vec<_>=(0..h).flat_map(|y|(0..w).map(move|x|(x,y)))
        .filter(|&(x,y)|(x,y)!=(px,py)&&x.abs_diff(px)<=6&&y.abs_diff(py)<=6).collect();
    positions.sort_by_key(|&(x,y)|(x.abs_diff(px).max(y.abs_diff(py)),x.abs_diff(px)+y.abs_diff(py),y,x));
    let depth=r.depth;
    for(x,y)in positions {
        if r.actors_at(depth).len()==32 {break;}
        assert_eq!(r.floor().unwrap().tiles[y*w+x],"floor");
        let mut a=corpse_core::population::create_actor_revision("rat",r.next_id,x as f64,y as f64,false,41.).unwrap();
        corpse_core::ai::wake(&mut a,true);a.state="hunting".into();
        a.last_seen=corpse_core::ActorLastSeen{x:r.player.x,y:r.player.y}.into();
        a.conditions.insert("targetId".into(),r.player.id);
        // Keep authored stealth, player senses and original lighting. Pick only
        // genuinely visible nearby positions; never force reveal/all-seeing.
        if !corpse_core::vision::visible_actor(&r,&a){continue;}
        r.next_id+=1.;r.actors_at_mut(depth).push(a);
    }
    assert_eq!(r.actors_at(depth).len(),32,"fixture cannot supply 32 genuinely visible nearby rats");
    let locations:std::collections::HashSet<_>=r.actors_at(depth).iter().map(|a|(a.x as usize,a.y as usize)).collect();
    assert_eq!(locations.len(),32);assert!(!locations.contains(&(px,py)));
    assert!(r.actors_at(depth).iter().all(|a|a.state=="hunting"&&!a.dead.value_or(false)));
    r
}

// Include from timeweave_action_tests.rs. Uses its unchanged exact fixture builders.
// Diagnostic only: full-state checks/painting/output occur OUTSIDE observation timers.
#[cfg(feature="runtime-timing")]
#[test]
#[ignore="lead-controlled view decomposition, no parallel workloads"]
fn timeweave_observe_breakdown() {
    use corpse_core::timing;
    let options=Options::read();
    let repeats:usize=std::env::var("OBSERVE_REPEATS").unwrap_or_else(|_|"20".into()).parse().unwrap();assert!((1..=50).contains(&repeats));
    let selected=std::env::var("OBSERVE_FIXTURES").unwrap_or_else(|_|"eat10,eat40,weave10,future40".into());
    let mut rows=Vec::new();
    for label in selected.split(',') {
        let mut engine=temporal::Engine::new(match label {
            "eat10"=>eat_fixture(10,41,false),"eat40"=>eat_fixture(40,41,false),
            "weave10"=>weave_room(),"future40"=>future_fixture(40),
            "actors10"=>ordinary_fixture(41),"crowded32"=>observe_crowded32_fixture(),_=>panic!("unknown fixture {label}")
        }).unwrap();
        let mut ui=Ui::new(&engine.run);
        match label {
            "eat10"|"eat40"=> {
                for _ in 0..5 {assert!(engine.dispatch(&Command::Wait).unwrap().took_turn);}
                ui.reopen(&engine.run);let id=engine.run.inventory[0].id;let action=select_eat(&mut ui.view,id);
                let before=engine.run.player.stats.offense;
                apply_pack_action(action,&mut ui.view,&mut engine,&mut ui.effects,300.,&mut ui.open,&mut ui.target,&mut ui.notice);
                assert_eq!(engine.run.player.stats.offense,before+1.);
            },
            "weave10"=>{assert!(dispatch(&mut engine,&mut ui.effects,300.,&Command::Move{dx:1.,dy:0.}).unwrap().took_turn);},
            "future40"=>{assert!(!dispatch(&mut engine,&mut ui.effects,300.,&future_command()).unwrap().took_turn);assert!(engine.run.future.is_some());},
            "actors10"=>{assert!(dispatch(&mut engine,&mut ui.effects,300.,&Command::Wait).unwrap().took_turn);},
            "crowded32"=>{}, // deliberately no AI/world turn; post-setup observation stress
            _=>unreachable!()
        }
        // Same post-action canonical fixtures as previous matrix; cache states are
        // CONTROLLED below rather than pretending to reproduce an unknown hit state.
        let before=engine.save().unwrap();let mut reference_pixels=None;let mut reference_view=None;
        for cache in ["cold","warm"] {for repeat in 1..=repeats {
            let arms=if repeat%2==1 {[false,true]}else{[true,false]};
            for enabled in arms {
                timing::set_enabled(false);timing::reset();
                engine.run.optical_cache=Default::default();engine.run.lighting_cache=Default::default();
                if cache=="warm" {let v=observe::observe_with_profile(&engine.run,&ui.painter.profile);std::hint::black_box(v);}
                timing::reset();timing::set_enabled(enabled);
                let start=Instant::now();let v=observe::observe_with_profile(&engine.run,&ui.painter.profile);let elapsed=start.elapsed().as_nanos();
                timing::set_enabled(false);let spans=timing::take_report();let counts=timing::take_counts();
                if !enabled {assert!(spans.is_empty());assert!(counts.is_empty());}
                if label=="crowded32" {
                    assert_eq!(v.creatures.len(),32,"all 32 catalog rats must actually be visible, not merely spawned");
                    let ids:std::collections::HashSet<_>=v.creatures.iter().map(|a|a.id.to_bits()).collect();assert_eq!(ids.len(),32);
                    assert!(v.creatures.iter().all(|a|a.state=="hunting"));
                }
                let terrain=v.terrain_snapshot.as_ref().unwrap();
                let metadata=serde_json::json!({"floor_cells":v.width*v.height,"known_cells":terrain.cells.iter().filter(|c|c.tile.is_some()).count(),"current_cells":terrain.cells.iter().filter(|c|c.current).count(),"direct_cells":terrain.cells.iter().filter(|c|c.direct).count(),"sources":terrain.light_sources.len(),"footprint_entries":terrain.light_sources.iter().map(|s|s.cells.len()).sum::<usize>(),"render_cells":v.cells.len(),"creatures":v.creatures.len(),"floor_actors":engine.run.actors_at(engine.run.depth).len()});
                // Fixed-time full paint is a differential OUTPUT oracle, never timed
                // or used as a substitute for a view construction measurement.
                let exact=format!("{v:?}");if let Some(old)=reference_view.as_ref(){assert_eq!(old,&exact,"cold/warm profiling changed full View");}else{reference_view=Some(exact);}
                ui.painter.game_at(&v,"",false,300.);
                if let Some(pixels)=reference_pixels.as_ref(){assert_eq!(&ui.painter.pixels,pixels,"instrumentation/cold/warm changed output");}
                else {reference_pixels=Some(ui.painter.pixels.clone());}
                rows.push(serde_json::json!({"fixture":label,"fixture_scope":if label=="crowded32"{"Synthetic post-setup open room with32 catalog rats at distinct nearby genuinely visible cells, awake/hunting; no simulation command, reveal override or light override"}else{"Existing post-action fixture"},"cache":cache,"repeat":repeat,"collection_enabled":enabled,"observe_ns":elapsed,"spans":spans,"counts":counts,"workload":metadata}));
            }
        }}
        assert_eq!(engine.save().unwrap(),before,"observing must not change canonical state");
    }
    let out=options.root.join("observe-breakdown.json");assert!(!out.exists());
    std::fs::write(&out,serde_json::to_vec_pretty(&serde_json::json!({"schema":1,"scope":"Observation only, no save/paint/SDL in timed interval. Same fixture builders and post-action state as previous matrix, but explicitly reset/primed derived caches. Alternating enabled/disabled collection in one instrumented binary; compile-disabled baseline remains a separate build control.","repeats":repeats,"rows":rows})).unwrap()).unwrap();
    println!("OBSERVE_BREAKDOWN {}",out.display());
}
