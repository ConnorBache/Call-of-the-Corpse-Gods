// Supplement to actual-action rows: cached-view hover/idle and deliberately
// edited paint-descriptor cases. No gameplay or full observation is timed here.
fn static_tile_frame(run:&Run,p:&Painter)->render::View {
    let mut v=observe::observe_with_profile(run,&p.profile);
    v.terrain_snapshot=None;v.dreamwalking=false;
    for c in &mut v.cells{c.animations.clear();c.pulse_speed=0.;}
    v
}
fn add_test_fire(v:&mut render::View,dense:bool){
    for(i,c)in v.cells.iter_mut().enumerate(){if dense||i==0{c.animations.push(render::Animation::Fire{phase:0.17+i as f64/100.,speed:1.,colors:[[255,40,0],[255,140,20],[255,220,80],[255,110,0]]});}}
}
/// Same descriptor/clock matrix for both paint modes. Setup and exact proofs are
/// outside intervals; all damage discovery/paint/upload planning/drop is inside.
#[test]
#[ignore="lead-controlled full/tile cached-frame diagnostic; no SDL timing"]
fn timeweave_tile_frame_matrix(){
    let options=Options::read();let mut rows=Vec::new();
    let cases=["unchanged-static","chronicle-hover","pack-hover","pack-close","memory-transition","changed-fg-bg","removed-layer","sparse-fire-idle","dense-fire-idle"];
    for repeat in 1..=options.repeats{for case in cases{
        let run=weave_room();let canonical=options.prove(repeat).then(||serde_json::to_value(&run).unwrap());
        let mut ui=Ui::new(&run);let mut initial=static_tile_frame(&run,&ui.painter);
        if case=="sparse-fire-idle"||case=="dense-fire-idle"{add_test_fire(&mut initial,case=="dense-fire-idle");}
        ui.tile_plan.invalidate();ui.paint_game(initial,0.);
        let point=if case.starts_with("pack-"){ui.reopen(&run);ui.paint_cached(0.);Some(ui.view.retained_test_hover_point(0).expect("authored pack must have visible item"))}else{None};
        let mut replacement=if matches!(case,"memory-transition"|"changed-fg-bg"|"removed-layer"){Some(static_tile_frame(&run,&ui.painter))}else{None};
        if let Some(v)=replacement.as_mut(){
            let at=v.cells.iter().position(|c|c.current).unwrap();
            match case{
                "memory-transition"=>{v.cells[at].current=false;v.cells[at].direct=false;},
                "changed-fg-bg"=>{v.cells[at].fg=[9,240,190];v.cells[at].bg=[36,7,81];},
                "removed-layer"=>{v.cells.remove(at);},_=>unreachable!()
            }
        }
        let now=if case.ends_with("fire-idle"){180.}else{0.};
        let t=start(options.timed);
        match case {
            "chronicle-hover"=>ui.painter.chronicle_hover=true,
            "pack-hover"=>{ui.view.configure(&ui.painter,modal_ui::Frame::from_profile(&ui.painter.profile).map.tuple());let(x,y)=point.unwrap();ui.view.hover(x,y);},
            "pack-close"=>ui.open=false,_=>{}
        }
        if let Some(v)=replacement.take(){ui.paint_game(v,now);}else{ui.paint_cached(now);}
        std::hint::black_box(&ui.painter.pixels);let paint_pipeline_ns=elapsed(t);
        let proof=options.prove(repeat);if proof{ui.prove_pixels(now);assert_eq!(serde_json::to_value(&run).unwrap(),canonical.unwrap(),"paint-only workload cannot mutate Run/RNG");}
        rows.push(ui.paint_metadata(serde_json::json!({"sample_id":format!("tile-frame-{case}-{repeat}"),"case":case,"repeat":repeat,"paint_pipeline_ns":paint_pipeline_ns,"whole_observe_timed":false,"gameplay_command_timed":false,"descriptor_setup_outside_timer":true,"changed_view_replacement_and_old_drop_timed":matches!(case,"memory-transition"|"changed-fg-bg"|"removed-layer"),"fixed_clock_ms":now,"preceding_presented_clock_ms":0.,"proof_performed":proof}),proof));
    }}
    let value=serde_json::json!({"schema":1,"suite":"timeweave-tile-frame-matrix","repeats":options.repeats,"timed":options.timed,"scope":"Cached-view hover/idle and synthetic descriptor edits, not gameplay observation. Descriptor construction/edit setup outside intervals in both arms. Planning/footprints/rowspan union/paint/checked borrowed upload sources/destruction inside. No SDL, texture upload or present calls. Optional counters OFF in headline rows. First/last full pixel and Run/RNG proofs outside intervals.","rows":rows});
    let out=options.root.join("timeweave-tile-frame-matrix.json");assert!(!out.exists());std::fs::write(&out,serde_json::to_vec_pretty(&value).unwrap()).unwrap();println!("TILE_FRAME_REPORT {}",out.display());
}
