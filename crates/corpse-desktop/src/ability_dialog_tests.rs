use super::*;
fn ability(description: &str) -> AbilityView {
    AbilityView { reserve:None,indicator:None,fortunes:None,pending_reserve_loss:0., id: "test".into(), name: "Observe".into(), description: description.into(),
        color: super::super::GOLD, cost: "10 Energy".into(), state: "Ready".into(),
        active: false, mode: "Active".into(), supported: true }
}
fn capture(name: &str, p: &Painter) {
    if let Ok(dir) = std::env::var("CORPSE_ABILITY_DIALOG_OUTPUT") {
        std::fs::create_dir_all(&dir).unwrap();
        let file = std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
        let mut enc = png::Encoder::new(file, p.width as u32, p.height as u32);
        enc.set_color(png::ColorType::Rgba); enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(&p.pixels).unwrap();
    }
}
#[test]
fn ability_dialog_fits_content_and_touches_map_corner_at_every_resolution() {
    let short = ability("See nearby creatures.");
    let long = ability(&format!("FIRST. {} FINAL.", "A complete description remains available by scrolling. ".repeat(80)));
    let mut p = Painter::new();
    for (w,h) in [(1280,800),(1920,1080),(1537,911),(960,1200),(960,720)] {
        p.resize(w,h); p.set_responsive_canvas(true);
        for a in [&short, &long] {
            let layout = Layout::resolve(&p,a);
            let b = bounds(&p,a).unwrap();
            let map=map_menu_area(p.frame().map);
            assert_eq!(b.right(),p.native_x((map.0+map.2) as f64) as usize);
            assert_eq!(b.bottom(),p.native_y((map.1+map.3) as f64) as usize);
            assert!(b.y>=p.native_y(map.1 as f64) as usize);
            assert!(b.bottom() < p.native_y(p.inspection_notice_y() as f64) as usize);
            let prior = (p.scale,p.origin,p.responsive_canvas,p.clip);
            p.clear([1,2,3]); p.ability_detail(a,usize::MAX);
            assert_eq!((p.scale,p.origin,p.responsive_canvas,p.clip),prior);
            for (i,pixel) in p.pixels.chunks_exact(4).enumerate() {
                if !b.contains(i%p.width,i/p.width) { assert_eq!(&pixel[..3],&[1,2,3],"outside exact dialog bounds"); }
            }
            if a.id == short.id && a.description == short.description {
                assert!(!layout.scrolling);
                assert_eq!(layout.height,layout.lines.iter().map(|l|l.height+l.gap).sum::<i32>()+2*PAD+FOOTER);
                assert!(layout.height < 200,"short prose must not retain the old 500-pixel box");
                assert!(layout.width < 516,"short prose shrinks horizontally too");
            } else {
                assert!(layout.scrolling);
                assert!(layout.first(usize::MAX)>0);
                let first=layout.first(usize::MAX);
                let visible_height=layout.lines.iter().skip(first).enumerate().map(|(i,l)|l.height+if i==0{0}else{l.gap}).sum::<i32>();
                assert!(visible_height<=layout.body_height,"last line remains accessible");
            }
        }
    }
}
#[test]
fn ability_dialog_wraps_headers_and_restores_after_resize_scroll_and_dismissal() {
    let run:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let before=run.clone(); let view=crate::observe::observe(&run);
    let short=ability("See nearby creatures.");
    let mut long=ability(&format!("FIRST. {} FINAL.","No authored ability content is lost. ".repeat(100)));
    long.name="A very long observed ability name that wraps instead of being truncated".into();
    long.cost="Reserve 50/50 · 15 NRG + action".into();
    let mut p=Painter::new(); let mut full=Painter::new();
    for (w,h) in [(1280,800),(1537,911),(960,1200)] {
        for p in [&mut p,&mut full] {p.resize(w,h);p.set_responsive_canvas(true);}
        let mut plan=crate::render::tile_damage::TilePlanner::default();
        let mut tracker=crate::tile_overlay::Tracker::default();
        plan.replace_view(None,&view,&p);
        let mut clear=None;
        for (step,show) in [None,Some((&short,0)),Some((&long,0)),Some((&long,usize::MAX)),None,Some((&short,0)),None].into_iter().enumerate() {
            let state=crate::tile_overlay::Frame {panels: show.and_then(|(a,_)|p.ability_detail_damage(a)).into_iter().collect(),redraw_panels:true,..Default::default()};
            for rect in tracker.changes(&p,&state) { plan.mark_native_rect(rect); }
            crate::tile_overlay::finish_dependencies(&mut p,&mut plan,&state);
            plan.paint_base(&mut p,&view,"Inspect: a remembered floor.",0.);
            if !p.tile_mask.is_empty() { if let Some((a,offset))=show {p.ability_detail(a,offset);} }
            p.tile_clip_enabled=false;
            full.game_at(&view,"Inspect: a remembered floor.",false,0.);
            if let Some((a,offset))=show {full.ability_detail(a,offset);}
            assert!(p.pixels==full.pixels,"tile/full equality {w}x{h} step {step}");
            if show.is_none() { if let Some(base)=&clear {assert_eq!(&p.pixels,base);}else{clear=Some(p.pixels.clone());} }
            else {capture(&format!("ability-{w}x{h}-{step}"),&p);}
            tracker.commit(state); plan.clear_frame();
        }
    }
    assert_eq!(run,before);
}
#[test]
fn ability_dialog_respects_custom_fonts_and_preserves_all_rows() {
    let mut p=Painter::new();
    p.profile=p.profile.set("typography.tooltips.fontSize",serde_json::json!(26)).unwrap();
    p.profile=p.profile.set("typography.tooltips.lineHeight",serde_json::json!(1.8)).unwrap();
    let mut a=ability(&"Large fonts also wrap all their text. ".repeat(30));
    a.name="A full long ability name that must not vanish".into();
    a.supported=false;
    let l=Layout::resolve(&p,&a);
    assert!(l.scrolling);
    assert!(l.lines.iter().map(|line|line.text.as_str()).collect::<Vec<_>>().join(" ").ends_with("Native execution coverage pending"));
    assert!(l.lines.iter().all(|line|p.measure_text_role(Role::Tooltips,&line.text,line.size)<=f64::from(l.width-2*PAD)));
    let b=p.ability_detail_damage(&a).unwrap();
    let map=map_menu_area(p.frame().map);
    assert_eq!(b.right(),p.native_x((map.0+map.2) as f64)as usize);
    assert_eq!(b.bottom(),p.native_y((map.1+map.3) as f64)as usize);
}
#[test]
fn footer_damage_covers_text_on_wide_and_portrait_windows() {
    let run:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let view=crate::observe::observe(&run);
    for(w,h)in[(1280,800),(1920,1080),(960,1200)] {
        let(mut p,mut full)=(Painter::new(),Painter::new());
        for p in [&mut p,&mut full]{p.resize(w,h);p.set_responsive_canvas(true);}
        let mut plan=crate::render::tile_damage::TilePlanner::default();plan.replace_view(None,&view,&p);
        let mut retained=crate::render::RetainedGame::default();
        let mut rp=Painter::new();rp.resize(w,h);rp.set_responsive_canvas(true);
        let mut empty=None;
        for text in ["","Inspect: stone floor","Select an enemy.","Inspect: floor with a heart","","Inspect: floor",""] {
            plan.mark_native_rect(p.footer_damage().unwrap());plan.finish_damage(&mut p);
            plan.paint_base(&mut p,&view,text,0.);p.tile_clip_enabled=false;
            full.game_at(&view,text,false,0.);
            retained.update(&mut rp,&view,text,0.,true,false).unwrap();
            assert!(rp.pixels==full.pixels,"retained footer {w}x{h} {text}");
            assert!(p.pixels==full.pixels,"footer updates {w}x{h} {text}");
            if text.is_empty(){if let Some(base)=&empty{assert_eq!(&p.pixels,base);}else{empty=Some(p.pixels.clone());}}
            else {assert_ne!(Some(&p.pixels),empty.as_ref());}
            plan.clear_frame();
        }
    }
}
#[test]
fn ability_dialog_authored_catalog_is_fully_wrapped_and_observation_only() {
    let mut run:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    run.simulation_revision=52.0.into(); run.offers.clear();
    run.player.abilities=["morae-see-the-future","anavani-illuminate","ursa-dream-see"].into_iter().map(str::to_owned).collect();
    run.player.active.clear();run.player.ability_reserves=corpse_core::Field::Missing;
    for id in run.player.abilities.clone(){corpse_core::revision50_ability_energy::initialize_reserve(&mut run.player,&id,50.,50.).unwrap();}
    let frozen=run.clone(); let view=crate::observe::observe(&run);
    let mut p=Painter::new(); p.set_responsive_canvas(true);
    for entry in corpse_core::progression::catalog_for_revision(52.) {
        let a=crate::ability_ui::observe(&run,&entry.id);
        let l=Layout::resolve(&p,&a);
        assert!(l.lines.iter().all(|line|p.measure_text_role(Role::Tooltips,&line.text,line.size)<=f64::from(l.width-2*PAD)),"{}",a.id);
        let b=p.ability_detail_damage(&a).unwrap();
        let map=map_menu_area(p.frame().map);
        assert_eq!(b.right(),p.native_x((map.0+map.2) as f64)as usize,"{}",a.id);
        assert_eq!(b.bottom(),p.native_y((map.1+map.3) as f64)as usize,"{}",a.id);
        assert!(b.bottom()<p.native_y(p.inspection_notice_y()as f64)as usize,"{}",a.id);
        if matches!(a.id.as_str(),"anavani-illuminate"|"morae-see-the-future"|"ursa-dream-see") {
            p.game_at(&view,"You see the floor and a Hemogoblin.",false,0.);
            p.ability_detail(&a,0);capture(&format!("authored-{}",a.id),&p);
        }
    }
    assert_eq!(run,frozen);
}

#[test]
fn inspection_strip_has_own_bounds_and_text_cannot_touch_border(){
    let run:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let view=crate::observe::observe(&run);
    for(w,h)in[(1280,800),(1920,1080),(960,1200)]{
        let mut p=Painter::new();p.resize(w,h);p.set_responsive_canvas(true);
        let strip=p.hud_tracks().inspector;
        let top=strip.1;
        let bottom=strip.1+strip.3;
        assert!(p.inspection_notice_y()>top);assert!(p.inspection_notice_y()+15<bottom);
        p.game_at(&view,"",false,0.);let empty=p.pixels.clone();
        p.game_at(&view,&"Inspect: stone floor, water and a very long creature name. ".repeat(10),false,0.);
        assert_ne!(empty,p.pixels);
        for y in [top,bottom-1]{
            let y=p.native_y(y as f64)as usize;
            let start=y*p.width as usize*4;let end=start+p.width as usize*4;
            assert_eq!(&empty[start..end],&p.pixels[start..end],"text must not overwrite border");
        }
    }
}
