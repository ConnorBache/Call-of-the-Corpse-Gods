// Actual SDL software texture transfer/display timing, separate from CPU paint.
// No desktop window, GPU, vsync, Windows runtime or hardware timing claim.
fn sdl_upload_readback_equal(got:&[u8],expected:&[u8],width:usize,label:&str){
    assert_eq!(got.len(),expected.len(),"SDL {label} readback length");
    if let Some((i,(a,b)))=got.iter().zip(expected).enumerate().find(|(_, (a,b))|a!=b){panic!("SDL {label}: first differing byte {i}, pixel({},{}), channel{}, got {a}, expected {b}",(i/4)%width,(i/4)/width,i%4);}
}
/// Surface/texture creation, initial full upload and initial presentation are
/// outside timers in BOTH arms. The next texture update uses the exact shared
/// production plan and checked borrowed byte ranges, never a copied prefix.
fn measure_software_display(p:&Painter,previous:&[u8],full:bool,case:&str,repeat:usize,order:usize)->serde_json::Value{
    use sdl2::pixels::PixelFormatEnum;
    let plan=tile_overlay::upload_plan(p,full);
    let surface=sdl2::surface::Surface::new(p.width as u32,p.height as u32,PixelFormatEnum::RGBA32).unwrap();
    let mut canvas=surface.into_canvas().unwrap();let creator=canvas.texture_creator();
    let mut texture=creator.create_texture_streaming(PixelFormatEnum::RGBA32,p.width as u32,p.height as u32).unwrap();
    texture.update(None,previous,p.width*4).unwrap();
    canvas.clear();canvas.copy(&texture,None,None).unwrap();canvas.present();
    // The setup texture/canvas is exactly the identical previous frame per arm.
    sdl_upload_readback_equal(&canvas.read_pixels(None,PixelFormatEnum::RGBA32).unwrap(),previous,p.width,"initial");
    let wall=Instant::now();let t=Instant::now();let mut actual_calls=0usize;let mut actual_bytes=0usize;
    if plan.full{texture.update(None,&p.pixels,p.width*4).unwrap();actual_calls=1;actual_bytes=p.width*p.height*4;}
    else{for &r in &plan.rectangles{let(bytes,pitch)=tile_overlay::upload_source(p,r).unwrap();texture.update(Some(sdl2::rect::Rect::new(r.x as i32,r.y as i32,r.w as u32,r.h as u32)),bytes,pitch).unwrap();actual_calls+=1;actual_bytes+=r.w*r.h*4;}}
    let update_ns=t.elapsed().as_nanos();let t=Instant::now();
    canvas.clear();canvas.copy(&texture,None,None).unwrap();canvas.present();
    let copy_present_ns=t.elapsed().as_nanos();let wall_ns=wall.elapsed().as_nanos();
    assert_eq!((actual_calls,actual_bytes),(plan.calls,plan.bytes));assert!(wall_ns>=update_ns+copy_present_ns);
    // Actual SDL readback, not CPU-buffer comparison, proves BOTH upload routes.
    sdl_upload_readback_equal(&canvas.read_pixels(None,PixelFormatEnum::RGBA32).unwrap(),&p.pixels,p.width,if full{"full"}else{"tile"});
    serde_json::json!({"case":case,"repeat":repeat,"order_in_pair":order,"arm":if full{"full-upload"}else{"tile-upload"},"width":p.width,"height":p.height,"previous_clock_ms":300.,"next_clock_ms":600.,"cpu_damage_pixels":p.tile_mask.area(),"raw_requested_calls":plan.requested_calls,"raw_requested_bytes":plan.requested_bytes,"chosen_full_texture":plan.full,"planned_calls":plan.calls,"planned_bytes":plan.bytes,"actual_update_calls":actual_calls,"actual_update_bytes":actual_bytes,"update_ns":update_ns,"copy_present_ns":copy_present_ns,"display_wall_ns":wall_ns,"unattributed_timer_ns":wall_ns-update_ns-copy_present_ns,"initial_and_final_sdl_readback_exact":true})
}
#[test]
#[ignore="lead-controlled SDL software upload/present n20; run with --test-threads=1"]
fn timeweave_tile_sdl_software_upload_present(){
    let options=Options::read();let repeats:usize=std::env::var("SDL_TILE_REPEATS").unwrap_or_else(|_|"20".into()).parse().unwrap();assert!((1..=50).contains(&repeats));
    let _sdl=sdl2::init().unwrap();let mut rows=Vec::new();
    for repeat in 1..=repeats {for animated in [false,true]{
        let case=if animated{"animated-freeback"}else{"static-freeback"};
        let mut engine=temporal::Engine::new(if animated{animated_weave_room()}else{weave_room()}).unwrap();
        let mut ui=Ui::new(&engine.run);
        // Establish tile candidate indices even when caller's action arm is full.
        // No environment mutation and no cloned View; all CPU setup is untimed.
        ui.tile_enabled=true;ui.diagnostics=false;let initial=ui.game.take().unwrap();ui.tile_plan.invalidate();ui.paint_game(initial,0.);
        let forward=dispatch(&mut engine,&mut ui.effects,300.,&Command::Move{dx:1.,dy:0.}).unwrap();assert!(forward.took_turn);
        let view=observe::observe_with_profile(&engine.run,&ui.painter.profile);ui.paint_game(view,300.);ui.prove_pixels(300.);
        let previous=ui.painter.pixels.clone();let before_animations=animation_counts(ui.game.as_ref().unwrap());
        let(x,y,rng,tick)=(engine.run.player.x,engine.run.player.y,engine.run.rng,engine.run.tick);
        let back=dispatch(&mut engine,&mut ui.effects,600.,&Command::Move{dx:-1.,dy:0.}).unwrap();assert!(!back.took_turn);assert_eq!((engine.run.player.x,engine.run.player.y),(x-1.,y));assert_eq!(engine.run.tick,tick);assert_eq!(engine.run.rng,rng);
        let view=observe::observe_with_profile(&engine.run,&ui.painter.profile);ui.paint_game(view,600.);ui.prove_pixels(600.);
        let after_animations=animation_counts(ui.game.as_ref().unwrap());if animated{assert!(before_animations.0>0&&after_animations.0>0);}else{assert_eq!((before_animations.0,after_animations.0),(0,0),"static control must genuinely have no animation candidates");}
        // Candidate next pixels already passed full paint oracle; both SDL arms
        // receive precisely these same old/new bytes and the real resulting mask.
        for(order,full)in (if repeat%2==0{[false,true]}else{[true,false]}).into_iter().enumerate(){
            let mut row=measure_software_display(&ui.painter,&previous,full,case,repeat,order);
            row["observed_animations_before"]=serde_json::json!({"all":before_animations.0,"fire":before_animations.1,"torch":before_animations.2,"reflection":before_animations.3});row["observed_animations_after"]=serde_json::json!({"all":after_animations.0,"fire":after_animations.1,"torch":after_animations.2,"reflection":after_animations.3});rows.push(row);
        }
    }}
    let value=serde_json::json!({"schema":1,"suite":"tile-sdl-software-upload-present","repeats":repeats,"renderer":"SDL software Surface canvas with RGBA32 streaming texture","scope":"One actual forward/freeback sequence derives identical previous/next CPU buffers and real tile masks per pair. CPU command/observation/paint, plan construction, texture setup/initial upload/presentation and both readbacks are outside display timers. Timed update includes checked borrowed upload sources and actual SDL texture.update calls. Timed copy/present includes canvas.clear,copy,present equally. Independent software renderer/texture per arm, alternating order. No SDL window, vsync, GPU, Windows runtime, hardware storage or total input latency claim. n20 nearest-rank p95 is second-highest and p99 is maximum, not a theoretical tail bound.","rows":rows});
    let out=options.root.join("tile-sdl-software-upload-present.json");assert!(!out.exists());std::fs::write(&out,serde_json::to_vec_pretty(&value).unwrap()).unwrap();println!("TILE_SDL_REPORT {}",out.display());
}
