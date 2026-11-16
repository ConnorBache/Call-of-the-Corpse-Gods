//! Proposed exact-pixel and counter tests. Staged only, not compiled or run.
use super::super::*;

fn fixture() -> (corpse_core::temporal::Engine,View) {
    let e=corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let v=crate::observe::observe(&e.run);(e,v)
}
fn seed_pixels(p:&mut Painter,seed:usize) {
    for (i,b) in p.pixels.iter_mut().enumerate(){*b=i.wrapping_mul(37).wrapping_add(seed*61) as u8;}
}
fn paint(p:&mut Painter,v:&View,pack:&crate::pack::PackView,state:&crate::pack::PackPaintState,now:f64,occluded:bool) {
    if occluded {p.game_at_behind_pack(v,"Pack occlusion parity",now,pack.bounds());}
    else {p.game_at(v,"Pack occlusion parity",false,now);}
    // Match the real order: effects precede the opaque Pack. A cue may reach
    // beyond its backing, so no effect draw itself is suppressed by this patch.
    let c=&v.cells[v.cells.len()/2];
    let cue=crate::effects::Cue {event:corpse_core::events::PresentationEvent {
        kind:"burst".into(),from:None,to:corpse_core::events::EventPoint{x:c.x as f64,y:c.y as f64},
        glyph:Some("Ω*".into()),color:Some("#af3187".into()),label:None,depth:v.depth, animation: None, },started:0.,held:true,static_marks:false};
    p.effects(v,&[cue],now,false);
    pack.paint_prepared(p,state);
    // A later translucent layer and text stay ordered above Pack, including
    // outside the panel. This layer is intentionally independent of map state.
    p.blend_rect(0,p.logical_height()-35,p.logical_width(),27,[53,127,211],87);
    p.text("Later overlay Ω",17,p.logical_height()-31,13,[219,211,87]);
}

#[test]
fn pack_occlusion_matches_full_composition_at_multiple_resolutions_and_damage() {
    let (engine,mut view)=fixture();
    let save=engine.save().unwrap();
    // Detached dense paint fixture, not reachable gameplay or a performance claim.
    view.width=40;view.height=30;view.terrain_snapshot=None;view.terrain_count=0;
    view.cells=(0..view.width*view.height).map(|i| {
        let mut c=Cell{x:i%view.width,y:i/view.width,glyph:if i%7==0{"@"}else{"."}.into(),
            fg:[221,151,43],bg:[17,(i%173) as u8,39],current:true,direct:true,..Default::default()};
        if i%11==0 {c.animations.push(Animation::Water{phase:0.7,speed:1.});}
        if i%13==0 {c.animations.push(Animation::Gas{phase:0.3,color:[17,219,51]});}
        if i%17==0 {c.background_overlay=Some(([119,31,87],0.43));}
        if i%37==0 {c.animations.push(Animation::Reflection{phase:0.9,color:[193,29,173]});c.emphasis=-1.;}
        if i%41==0 {c.animations.push(Animation::Torch);c.emphasis=0.01;}
        if i%43==0 {c.animations.push(Animation::Fire{phase:0.4,speed:1.,colors:[[99,13,1],[211,73,4],[241,173,9],[255,219,37]]});c.emphasis=0.999;}
        if i%19==0 {c.timeweave=Some((1,-1));}
        // These must never be culled using the plain-cell footprint.
        if i%23==0 {c.superscript="long overhang Ω".into();}
        if i%29==0 {c.glyph="Ω*".into();c.emphasis=1.5;}
        if i%31==0 {c.underlays.push(("*".into(),[87,211,53],0.7));}
        c
    }).collect();
    view.path=vec![(12,14),(13,14),(14,14)];view.hover=Some((14,14));
    let mut p=Painter::new();let mut pack=crate::pack::PackView::observe(&engine.run);
    let mut total_skipped=0;
    for (w,h) in [(1280,800),(1920,1080),(3840,2160),(1279,803),(719,1201),(320,200)] {
        p.resize(w,h);p.set_responsive_canvas(true);
        p.profile=p.profile.set("ui.colors.panel",serde_json::json!("#23195c")).unwrap();
        pack.configure(&p,p.frame().map.tuple());
        let state=pack.paint_state(&p); // prepared once and reused in both arms
        for (case,damage) in [None,Some(damage::Rect::new(w/4,h/5,w/2,h/2)),
            Some(damage::Rect::new(w-7,0,7,h)),Some(damage::Rect::new(0,0,0,h))].into_iter().enumerate() {
            p.damage_clip=damage;
            p.clip=Some((-3,5,1281,791));
            seed_pixels(&mut p,case);p.raster_stats=damage::RasterStats{enabled:true,..Default::default()};
            paint(&mut p,&view,&pack,&state,420.,false);
            let expected=p.pixels.clone();let old_visits=p.raster_stats.pixel_visits;
            p.clip=Some((-3,5,1281,791));
            seed_pixels(&mut p,case);p.raster_stats=damage::RasterStats{enabled:true,..Default::default()};
            paint(&mut p,&view,&pack,&state,420.,true);
            assert_eq!(p.pixels,expected,"{w}x{h} damage={damage:?}");
            assert!(p.pack_map_occlusion.is_none(),"occlusion must not survive the call");
            assert!(p.pack_occlusion_stats.skipped<=p.pack_occlusion_stats.eligible);
            assert!(p.pack_occlusion_stats.eligible<=p.pack_occlusion_stats.checks);
            assert!(p.raster_stats.pixel_visits<=old_visits);
            total_skipped+=p.pack_occlusion_stats.skipped;
            if case==0 && w>=1280 {
                assert!(p.pack_occlusion_stats.skipped>0,"oracle must exercise actual omissions");
                assert!(p.raster_stats.pixel_visits<old_visits,"omissions must avoid raster work");
            }
        }
        p.damage_clip=None;p.clip=None;
        // Close/reopen does not leave occlusion behind or require a retained frame.
        p.game_at(&view,"",false,420.);let expected=p.pixels.clone();
        p.game_at_behind_pack(&view,"",420.,pack.bounds());pack.paint(&mut p);
        p.game_at(&view,"",false,420.);assert_eq!(p.pixels,expected);
    }
    assert!(total_skipped>0);
    assert_eq!(engine.save().unwrap(),save,"paint must not mutate simulation");
}

#[test]
fn pack_occlusion_falls_back_for_custom_map_style_and_tile_paint() {
    let (engine,view)=fixture();let mut p=Painter::new();p.resize(1920,1080);p.set_responsive_canvas(true);
    let mut pack=crate::pack::PackView::observe(&engine.run);pack.configure(&p,p.frame().map.tuple());
    for profile in [crate::appearance_profile::Profile::default().set("typography.map.glyphScale",serde_json::json!(1.2)).unwrap(),
        crate::appearance_profile::Profile::default().set("typography.map.fontFamily",serde_json::json!("IBM Plex Mono")).unwrap()] {
        p.profile=profile;pack.configure(&p,p.frame().map.tuple());let state=pack.paint_state(&p);
        paint(&mut p,&view,&pack,&state,420.,false);let expected=p.pixels.clone();
        paint(&mut p,&view,&pack,&state,420.,true);
        assert_eq!(p.pixels,expected);assert_eq!(p.pack_occlusion_stats.skipped,0);
    }
    p.profile=crate::appearance_profile::Profile::default();
    pack.configure(&p,p.frame().map.tuple());let state=pack.paint_state(&p);
    p.tile_mask.reset(p.width,p.height);p.tile_mask.add(damage::Rect::new(0,0,p.width,p.height));p.tile_mask.finish();
    p.tile_cell_bounds=view.cells.iter().map(|c|p.cell_damage_rect(c,view.width,view.height)).collect();
    p.tile_clip_enabled=true;
    paint(&mut p,&view,&pack,&state,420.,false);let expected=p.pixels.clone();
    paint(&mut p,&view,&pack,&state,420.,true);
    assert_eq!(p.pixels,expected);assert_eq!(p.pack_occlusion_stats.skipped,0);
}

#[test]
fn conservative_plain_cell_filter_keeps_overhang_and_complex_cells() {
    let mut p=Painter::new();p.pack_map_occlusion=Some(damage::Rect::new(100,100,900,600));
    let plain=Cell{glyph:"@".into(),..Default::default()};
    assert!(p.pack_hides_plain_cell(&plain,400,300,15,24));
    assert!(!p.pack_hides_plain_cell(&plain,100,100,15,24),"boundary overhang stays painted");
    for kind in 0..8 {
        let mut c=plain.clone();
        match kind {0=>c.glyph="Ω*".into(),1=>c.superscript="123".into(),2=>c.subscript="long".into(),
            3=>c.arrow="→".into(),4=>c.underlays.push(("*".into(),[255,0,0],0.5)),5=>c.italic=true,
            6=>c.heartless=true,_=>c.emphasis=1.01}
        assert!(!p.pack_hides_plain_cell(&c,400,300,15,24),"complex kind={kind}");
    }
}

/// Quiet-lead-only matched benchmark. Same-binary whole composition, counters
/// disabled during timing; one separate diagnostic paint reports actual savings.
#[test]
#[ignore="lead-controlled release timing only; no SDL/GPU/Windows claim"]
fn pack_occlusion_renderer_benchmark() {
    let (engine,view)=fixture();let mut p=Painter::new();let mut pack=crate::pack::PackView::observe(&engine.run);
    for (w,h) in [(1280,800),(1920,1080),(3840,2160)] {
        p.resize(w,h);p.set_responsive_canvas(true);pack.configure(&p,p.frame().map.tuple());let state=pack.paint_state(&p);
        p.raster_stats=damage::RasterStats::default();
        paint(&mut p,&view,&pack,&state,420.,false);let expected=p.pixels.clone();
        paint(&mut p,&view,&pack,&state,420.,true);assert_eq!(p.pixels,expected);
        for _ in 0..3 {for mode in [false,true] {paint(&mut p,&view,&pack,&state,420.,mode);}}
        let mut times:[Vec<u64>;2]=[Vec::new(),Vec::new()];
        for pair in 0..20 {
            for mode in if pair%2==0 {[false,true]}else{[true,false]} {
                let start=std::time::Instant::now();paint(&mut p,&view,&pack,&state,420.,mode);
                std::hint::black_box(&p.pixels);
                times[usize::from(mode)].push(start.elapsed().as_nanos() as u64);
                assert_eq!(p.pixels,expected); // outside timing
            }
        }
        let mut diagnostics=Vec::new();
        for mode in [false,true] {
            p.raster_stats=damage::RasterStats{enabled:true,..Default::default()};
            paint(&mut p,&view,&pack,&state,420.,mode);
            diagnostics.push(serde_json::json!({"occluded":mode,"checks":p.pack_occlusion_stats.checks,
                "eligible":p.pack_occlusion_stats.eligible,"skipped":p.pack_occlusion_stats.skipped,
                "pixel_visits":p.raster_stats.pixel_visits,"cells_painted":p.raster_stats.cells_painted}));
        }
        p.raster_stats=damage::RasterStats::default();
        eprintln!("PACK_OCCLUSION {}",serde_json::json!({"width":w,"height":h,"full_ns":times[0],
            "occluded_ns":times[1],"diagnostics":diagnostics,"scope":"detached avatar fixture plus Pack/effects/test overlay; CPU software paint only; paired alternating order; counters off for timing"}));
    }
}
