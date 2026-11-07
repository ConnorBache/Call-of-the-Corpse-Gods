use super::*;
#[test]
fn compact_dialog_input_damage_and_canvas_restore_share_one_transform() {
    let mut p=Painter::new();
    for(w,h)in[(1280,800),(1920,1080),(1537,911),(960,1200),(960,720)] {
        p.resize(w,h);p.set_responsive_canvas(true);let old=(p.scale,p.origin,p.logical_width(),p.logical_height());
        let expected=p.sheet_damage_rect(337,125,636,579).unwrap();
        let window=((w/2)as u32,(h/2)as u32);
        let point=p.centered_sheet(|s|{
            assert_eq!(s.logical_damage_rect(337,125,636,579),Some(expected));
            assert!(s.scale<old.0);
            assert!((s.scale / old.0 - DIALOG_SCALE).abs() < 1e-12);
            let pixel=(s.native_x(500.5),s.native_y(300.5));
            ((pixel.0 as f64*window.0 as f64/w as f64).round()as i32,(pixel.1 as f64*window.1 as f64/h as f64).round()as i32)
        });
        let got=p.logical_point_for_sheet(point.0,point.1,window);assert!((got.0-500).abs()<=2&&(got.1-300).abs()<=2);
        assert_eq!((p.scale,p.origin,p.logical_width(),p.logical_height()),old);
        let choice=p.centered_sheet(|s|(s.native_x(500.),s.native_y(220.)));
        let choice=p.logical_point_for_sheet(choice.0,choice.1,(w as u32,h as u32));assert_eq!(blessing_choice_at(choice.0,choice.1),Some(0));
    }
}
#[test]
fn compact_dialogs_render_repeatably_and_restore_background_after_dismissal() {
    let engine=corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();let view=crate::observe::observe(&engine.run);let before=engine.save().unwrap();
    let mut p=Painter::new();p.set_responsive_canvas(true);p.game_at(&view,"",false,0.);let base=p.pixels.clone();
    let ability=crate::ability_ui::AbilityView{id:"test".into(),name:"A long observed ability name".into(),description:"Full ability details remain available by scrolling. ".repeat(45),color:GOLD,cost:"6 Energy".into(),state:"Ready".into(),active:false,mode:"Active".into(),supported:true};
    for name in ["creatures","ability","hazard","inspection","pack","blessing"] {
        p.pixels.copy_from_slice(&base);
        match name {
            "creatures"=>p.creatures(&view,None,0),
            "ability"=>p.ability_detail(&ability,usize::MAX),
            "hazard"=>p.hazard_prompt("Walk amid razorleaves? Ending your turn there costs 2% of your maximum health (rounded up)."),
            "inspection"=>{let report=crate::inspection_ui::observe_tile(&engine.run,engine.run.player.x as usize,engine.run.player.y as usize);p.centered_sheet(|p|crate::inspection_ui::paint(&report,p,0));},
            "pack"=>{let mut pack=crate::pack::PackView::observe(&engine.run);pack.configure(&p,p.frame().map.tuple());assert!(pack.bounds().2<=760&&pack.bounds().3<=500);pack.paint(&mut p);},
            _=>p.blessing_choices(&[ability.clone(),ability.clone(),ability.clone()],0),
        }
        assert_ne!(p.pixels,base);
        if let Ok(dir)=std::env::var("CORPSE_COMPACT_DIALOG_OUTPUT") {std::fs::create_dir_all(&dir).unwrap();let f=std::fs::File::create(format!("{dir}/{name}.png")).unwrap();let mut enc=png::Encoder::new(f,p.width as u32,p.height as u32);enc.set_color(png::ColorType::Rgba);enc.set_depth(png::BitDepth::Eight);enc.write_header().unwrap().write_image_data(&p.pixels).unwrap();}
        p.game_at(&view,"",false,0.);assert_eq!(p.pixels,base,"closing {name}");
    }
    assert_eq!(engine.save().unwrap(),before);
}
