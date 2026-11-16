use super::*;
// O(P) output/time per frame; opt-in evidence harness never writes user profiles.
fn record(p: &Painter, name: &str) {
    let dir = std::env::var("PAINT_EVIDENCE_DIR").expect("isolated output directory");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(format!("{dir}/{name}-{}x{}.rgba",p.width,p.height),&p.pixels).unwrap();
}
#[test]
#[ignore = "opt-in cross-build exact RGBA and release timing evidence"]
fn exhaustive_paint_evidence() {
    let engine=corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let mut view=crate::observe::observe(&engine.run);
    view.path=vec![(12,14),(13,14)]; view.hover=Some((13,14));
    for (width,height) in [(1280,800),(1920,1080),(900,720),(720,900),(320,200)] {
        let mut p=Painter::new(); p.resize(width,height);
        for (name,now) in [("game",0.),("animated",420.)] {
            p.game_at(&view,"Parity test",false,now); record(&p,name);
        }
        for frame in [0,18,37,83] { p.menu(frame,0,true,"Blessed"); record(&p,&format!("menu-{frame}")); }
        p.start("parity", "Avatar", true, None, ""); record(&p,"start");
        p.blessing_choices(&[],0); record(&p,"blessing");
        p.history(&["A long history. Ω ♥".into()],0); record(&p,"history");
        let mut m=crate::manual::Manual::new();
        for id in ["blorb","wendigo","bastidion","zillennia:transcendent"] {
            assert!(m.select_id(id)); m.paint_at_depth(&mut p,83.,14); record(&p,&format!("manual-{id}"));
        }
        p.game_at(&view,"",false,420.);
        let mut pack=crate::pack::PackView::observe(&engine.run);
        pack.configure(&p,p.map_layout(view.width,view.height)); pack.paint(&mut p); record(&p,"pack");
        let report=crate::inspection_ui::observe_tile(&engine.run,engine.run.player.x as usize,engine.run.player.y as usize);
        p.game_at(&view,"",false,420.); crate::inspection_ui::paint(&report,&mut p,0);record(&p,"inspection");
        p.game_at(&view,"",false,420.); crate::inspection_ui::paint_hover(&report,&mut p,300,300);record(&p,"hover");
        p.profile=p.profile.set("typography.chronicle.fontWeight",serde_json::json!(700)).unwrap();
        p.profile=p.profile.set("typography.map.fontFamily",serde_json::json!("IBM Plex Mono")).unwrap();
        p.profile=p.profile.set("typography.map.letterSpacing",serde_json::json!(0.7)).unwrap();
        p.game_at(&view,"Styled",false,420.);record(&p,"styled");
        p.profile=crate::appearance_profile::Profile::default();
        for c in &mut view.cells {
            c.animations=vec![Animation::Gas{phase:0.7,color:[18,233,91]},Animation::Reflection{phase:0.4,color:[219,45,192]}];
        }
        p.game_at(&view,"Effects",false,420.);record(&p,"gas-reflections");
        for c in &mut view.cells {c.animations.clear();}
        let cue=crate::effects::Cue{event:corpse_core::events::PresentationEvent{
            kind:"burst".into(),from:None,to:corpse_core::events::EventPoint{x:engine.run.player.x,y:engine.run.player.y},
            glyph:Some("人".into()),color:None,label:None,depth:view.depth, animation: None, },started:0.,held:true,static_marks:false};
        p.effects(&view,&[cue],83.,false);record(&p,"explosion-overlay");
        // All alpha values on arbitrary destination RGBA, clipped/downscaled strokes,
        // font fallback, compound/italic map glyphs, picture scaling and zero dimensions.
        for (i,v) in p.pixels.iter_mut().enumerate() { *v=(i.wrapping_mul(37)%256) as u8; }
        for clip in [None,Some((-2,3,100,100)),Some((30,30,0,10)),Some((100,120,97,83))] {
            p.clip=clip;
            for a in 0..=255u8 { p.pixel(a as i32-30,a as i32/2,[a,255-a,79],a); }
            for (x,y,w,h) in [(-10,-10,30,30),(1,2,1,1),(1275,795,20,20),(20,20,0,2),(10,10,200,150)] {p.rect(x,y,w,h,[73,126,211]);}
            p.text("AΩ♥漢",-2,22,19,[183,19,244]);
            p.prose("AΩ♥",30,55,19,[112,223,37],false);
            p.prose_contrast_outline=true; p.prose("Ink",30,80,19,[12,23,37],true); p.prose_contrast_outline=false;
            p.map_glyph_styled('@',90,90,21,27,[232,84,17],0.42,true);
            p.map_identity("Ω*",120,100,30,30,[97,129,231]);
            let image=Picture{w:2,h:2,rgba:vec![12,43,72,0,21,212,73,127,255,40,19,255,38,92,12,1]};
            p.picture(&image,-3,40,94,86);
        }
        p.clip=None; record(&p,"primitives");
        if width>=1280 {
            for _ in 0..4 {p.game_at(&view,"",false,420.);}
            let mut times=Vec::new();
            for _ in 0..31 {let start=std::time::Instant::now();p.game_at(&view,"",false,420.);std::hint::black_box(&p.pixels);times.push(start.elapsed().as_secs_f64()*1000.);}
            times.sort_by(f64::total_cmp); eprintln!("PAINT_TIMING game {width}x{height} median_ms={} p95_ms={}",times[15],times[29]);
        }
    }
}
/// O(samples log samples + samples * draw cost), O(samples) timing storage.
fn bench(label: &str, mut draw: impl FnMut()) {
    for _ in 0..8 {draw();}
    let mut values=Vec::new();
    for _ in 0..51 {let start=std::time::Instant::now();draw();values.push(start.elapsed().as_nanos() as u64);}
    let raw=values.clone();values.sort();
    eprintln!("PAINT_BENCH {}",serde_json::json!({"label":label,"median_ns":values[25],"p95_ns":values[48],"raw_ns":raw}));
}
/// Fixed input detached paint benchmark; no simulation mutation or save writes.
#[test]
#[ignore = "quiet-host optimized baseline/candidate timing only"]
fn bulk_paint_benchmark() {
    let engine=corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let mut view=crate::observe::observe(&engine.run);
    for (width,height) in [(1280,800),(1920,1080)] {
        let mut p=Painter::new();p.resize(width,height);
        let prefix=format!("{width}x{height}");
        bench(&format!("{prefix}/clear"),||{p.clear([9,13,17]);std::hint::black_box(&p.pixels);});
        bench(&format!("{prefix}/opaque_background"),||{p.rect(0,100,950,640,[8,13,19]);std::hint::black_box(&p.pixels);});
        bench(&format!("{prefix}/text"),||{for y in (10..700).step_by(22) {p.text("Sphinx Ω @ creature description",20,y,18,[223,139,44]);}std::hint::black_box(&p.pixels);});
        bench(&format!("{prefix}/prose"),||{for y in (10..700).step_by(22) {p.prose("Sphinx Ω @ creature description",20,y,18,[223,139,44],false);}std::hint::black_box(&p.pixels);});
        bench(&format!("{prefix}/compound_glyphs"),||{for y in (120..600).step_by(24) {for x in (10..900).step_by(24) {p.map_identity("Ω*",x,y,24,24,[87,231,149]);}}std::hint::black_box(&p.pixels);});
        bench(&format!("{prefix}/game_avatar"),||{p.game_at(&view,"",false,420.);std::hint::black_box(&p.pixels);});
        bench(&format!("{prefix}/menu"),||{p.menu(37,0,true,"Blessed");std::hint::black_box(&p.pixels);});
        let mut m=crate::manual::Manual::new();assert!(m.select_id("wendigo"));
        bench(&format!("{prefix}/manual_wendigo"),||{m.paint_at_depth(&mut p,83.,14);std::hint::black_box(&p.pixels);});
        for c in &mut view.cells {c.glyph="Ω*".into();c.animations=vec![Animation::Gas{phase:0.7,color:[18,233,91]},Animation::Reflection{phase:0.4,color:[219,45,192]}];}
        bench(&format!("{prefix}/synthetic_dense_animated"),||{p.game_at(&view,"",false,420.);std::hint::black_box(&p.pixels);});
        view=crate::observe::observe(&engine.run);
    }
}
/// Fixed detached menu component diagnostics; separate microbenchmarks are not additive spans.
#[test]
#[ignore = "quiet-host menu component timing only"]
fn menu_component_benchmark() {
    let mut p=Painter::new();p.resize(1920,1080);
    let frame=37;
    let ambient_frame=18+((frame as u64-18)*33/83) as u32;
    let fire=hellfire(ambient_frame);
    let left=-(p.origin.0 as f64/p.scale).ceil() as i32;
    let top=-(p.origin.1 as f64/p.scale).ceil() as i32;
    let wide=(p.width as f64/p.scale).ceil() as i32;
    let high=(p.height as f64/p.scale).ceil() as i32;
    let art_h=(500. * p.logo.h as f64/p.logo.w as f64).round() as i32;
    bench("menu1080/fire_generation",||{std::hint::black_box(hellfire(ambient_frame));});
    bench("menu1080/fire_picture",||{p.picture(&fire,left,top,wide,high);std::hint::black_box(&p.pixels);});
    bench("menu1080/embers",||{p.menu_embers(ambient_frame);std::hint::black_box(&p.pixels);});
    bench("menu1080/anatomy",||{p.beating_emblem(676,58,500,art_h,frame);std::hint::black_box(&p.pixels);});
    if let Some(cache)=&p.menu_raster {eprintln!("MENU_CACHE_STORAGE {:?}",cache.storage());}
    bench("menu1080/title_shapes",||{
        crate::menu_ui::lettering(&mut p,"CALL OF THE",113,61,337,35,[238,229,207],false);
        crate::menu_ui::lettering(&mut p,"CORPSE",103,116,559,129,[238,229,207],true);
        crate::menu_ui::lettering(&mut p,"GODS",177,266,463,145,[197,37,54],true);
        std::hint::black_box(&p.pixels);
    });
}
