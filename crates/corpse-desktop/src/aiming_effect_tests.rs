//! Full, retained, tile and GPU-command aiming oracles. No platform input claims.
use super::*;
use corpse_core::{Actor,Field,Run};
use corpse_core::revision50_amulet_effects::{self as amulets,Effect};
fn room()->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=51.0.into();r.player.x=10.;r.player.y=10.;r.player.conditions.clear();r.player.abilities.clear();r.player.active.clear();
    r.player.conditions.insert("allSeeingReveal".into(),1.);r.player.conditions.insert("allSeeingDepth".into(),r.depth);
    r.actors.clear();r.inventory.clear();r.ground.clear();r.offers.clear();
    let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(0.);f.seen.fill(true);f.remembered_tiles=Field::Missing;
    f.environment=Field::Missing;f.light_sources=Field::Missing;f.doors=Field::Missing;f.portals=Field::Missing;f.non_euclidean=Field::Missing;f.speed_up=Field::Missing;f.godbeast_event=Field::Missing;
    for (id,kind) in [(900.,Effect::Lightning),(901.,Effect::Firebolt),(902.,Effect::Iceblast),(903.,Effect::Blinking)] {r.inventory.push(amulets::make_amulet(kind,id).unwrap());}
    r.actors_at_mut(r.depth).push(Actor{id:910.,x:16.,y:10.,hp:100.,max_hp:100.,name:"Target".into(),glyph:"r".into(),state:"wandering".into(),..Default::default()});
    let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[10*w+21]="wall".into();r
}
#[test]fn r51_aim_mouse_keyboard_items_cancel_and_equipment_are_pure(){
    let r=room();let before=serde_json::to_vec(&r).unwrap();let mut v=observe::observe(&r);
    for target in [pack::PackTarget::Shoot(1.),pack::PackTarget::Throw(1.),pack::PackTarget::UseAt(901.)] {
        sync_pointer_view(&mut v,&r,Some((20,10)),vec![],Some(&target));sync_ability_preview(&mut v,&r,None,None,true);
        assert_eq!(v.target_path.last(),Some(&(16,10)));assert_eq!(v.target_impact,Some((16,10)));assert_eq!(v.target_color,Some([246,192,91]));
        let next=aim_keyboard_tile(&r,Some((20,11)),Keycode::Up).unwrap();assert_eq!(next,(20,10));
        let mut keyboard=observe::observe(&r);sync_pointer_view(&mut keyboard,&r,Some(next),vec![],Some(&target));
        assert_eq!(keyboard.target_path,v.target_path);assert_eq!(keyboard.target_area,v.target_area);assert_eq!(keyboard.target_impact,v.target_impact);
        sync_pointer_view(&mut v,&r,None,vec![],None);assert!(v.target_path.is_empty());assert!(v.target_area.is_empty());assert!(v.target_impact.is_none());assert!(v.target_color.is_none());
    }
    sync_pointer_view(&mut v,&r,Some((11,10)),vec![],Some(&pack::PackTarget::UseAt(900.)));
    assert!(v.target_path.contains(&(20,10)));assert_eq!(v.target_impact,Some((21,10)));
    sync_pointer_view(&mut v,&r,Some((14,13)),vec![],Some(&pack::PackTarget::UseAt(902.)));
    assert!(v.target_path.is_empty());assert_eq!(v.target_area.len(),9);assert_eq!(v.target_impact,Some((14,13)));
    let mut changed=r.clone();changed.inventory.clear();sync_item_preview(&mut v,&changed,Some(&pack::PackTarget::UseAt(902.)));
    assert!(v.target_path.is_empty());assert!(v.target_area.is_empty());assert!(v.target_impact.is_none());
    assert_eq!(aim_keyboard_tile(&r,Some((0,0)),Keycode::Left),Some((0,0)));
    assert_eq!(aim_keyboard_tile(&r,None,Keycode::Right),Some((11,10)));
    assert_eq!(aim_keyboard_tile(&r,None,Keycode::Escape),None);
    assert!(aim_modifiers_allowed(Mod::NOMOD));assert!(aim_modifiers_allowed(Mod::LSHIFTMOD));
    for modifiers in [Mod::LALTMOD,Mod::RCTRLMOD,Mod::LGUIMOD] {assert!(!aim_modifiers_allowed(modifiers));}
    assert_eq!(serde_json::to_vec(&r).unwrap(),before);
}
fn frames(r:&Run)->Vec<render::View> {
    [(Some(pack::PackTarget::UseAt(901.)),Some((20,10))),
     (Some(pack::PackTarget::UseAt(900.)),Some((11,10))),
     (Some(pack::PackTarget::UseAt(902.)),Some((14,13))),
     (None,None),(Some(pack::PackTarget::Shoot(1.)),Some((14,11))),
     (Some(pack::PackTarget::UseAt(903.)),Some((14,13))),(None,None)]
     .into_iter().map(|(target,hover)|{let mut v=observe::observe(r);sync_pointer_view(&mut v,r,hover,vec![],target.as_ref());v}).collect()
}
#[test]fn r52_aim_mouse_keyboard_first_impact_matches_committed_command(){
    for (id,endpoint) in [(902.,(16,10)),(903.,(15,10)),(901.,(16,10))] {
        let mut r=room();r.simulation_revision=52.0.into();let before=r.clone();
        let target=pack::PackTarget::UseAt(id);let mut mouse=observe::observe(&r);
        sync_pointer_view(&mut mouse,&r,Some((20,10)),vec![],Some(&target));
        let next=aim_keyboard_tile(&r,Some((20,11)),Keycode::Up).unwrap();let mut keyboard=observe::observe(&r);
        sync_pointer_view(&mut keyboard,&r,Some(next),vec![],Some(&target));
        assert_eq!(mouse.target_path,keyboard.target_path);assert_eq!(mouse.target_impact,keyboard.target_impact);
        assert_eq!(mouse.target_area,keyboard.target_area);assert_eq!(mouse.target_impact,Some(endpoint));
        assert_eq!(mouse.target_path.last(),Some(&endpoint));assert_eq!(r,before);
        assert!(target.target_blocker(&r,20.,10.).is_none());
        let out=corpse_core::engine::dispatch(&mut r,&target.command(20.,10.)).unwrap();
        let key=if id==902.{"A04"}else if id==903.{"A06"}else{"A02"};
        assert!(out.events.iter().filter_map(|e|e.animation.as_ref()).any(|a|a.proposal==key&&a.stage==corpse_core::events::Stage::Travel
            &&a.cells.last().is_some_and(|p|(p.x as usize,p.y as usize)==endpoint)));
        if id==903. {assert_eq!((r.player.x,r.player.y),(15.,10.));}
        sync_pointer_view(&mut mouse,&r,None,vec![],None);assert!(mouse.target_path.is_empty()&&mouse.target_area.is_empty()&&mouse.target_impact.is_none());
    }
}
#[test]fn r52_aim_shifted_blast_and_blink_full_retained_and_tile_match(){
    let mut r=room();r.simulation_revision=52.0.into();let mut views=Vec::new();
    for (id,hover) in [(902.,Some((20,10))),(903.,Some((20,10))),(902.,Some((14,13))),(903.,Some((16,10))),(902.,None)] {
        let mut v=observe::observe(&r);sync_pointer_view(&mut v,&r,hover,vec![],Some(&pack::PackTarget::UseAt(id)));views.push(v);
    }
    let mut retained=render::RetainedGame::default();let mut p=Painter::new();let mut full=Painter::new();
    for (w,h) in [(1280,800),(960,1200)] {p.resize(w,h);full.resize(w,h);p.set_responsive_canvas(true);full.set_responsive_canvas(true);
        for (i,v) in views.iter().enumerate() {retained.update(&mut p,v,"",0.,true,false).unwrap();full.game_at(v,"",false,0.);assert!(p.pixels==full.pixels);
            if let Ok(dir)=std::env::var("CORPSE_AIM_REVIEW") {
                std::fs::create_dir_all(&dir).unwrap();let name=["intercepted-iceblast","shortened-blink","clear-iceblast","occupied-blink","cancel"][i];
                let file=std::fs::File::create(format!("{dir}/r52-{name}-{w}x{h}.png")).unwrap();
                let mut enc=png::Encoder::new(file,p.width as u32,p.height as u32);enc.set_color(png::ColorType::Rgba);enc.set_depth(png::BitDepth::Eight);
                enc.write_header().unwrap().write_image_data(&p.pixels).unwrap();
            }
        }
    }
    let mut tile=Painter::new();let mut full=Painter::new();let mut plan=render::tile_damage::TilePlanner::default();let mut old=None;
    for v in &views {plan.replace_view(old,v,&tile);plan.finish_damage(&mut tile);plan.paint_base(&mut tile,v,"",0.);tile.tile_clip_enabled=false;plan.clear_frame();
        full.game_at(v,"",false,0.);assert!(tile.pixels==full.pixels);old=Some(v);}
    assert!(ability_ui::display_description(&r,"anavani-illuminate","").contains("Ichor-covered water does not"));
}
#[test]fn r51_aim_full_retained_tile_cancel_animation_and_resize_match(){
    let r=room();let views=frames(&r);let mut retained=render::RetainedGame::default();let mut p=Painter::new();let mut full=Painter::new();
    for (w,h) in [(1280,800),(1920,1080),(960,1200),(1537,911)] {
        p.resize(w,h);full.resize(w,h);p.set_responsive_canvas(true);full.set_responsive_canvas(true);
        for (i,v) in views.iter().enumerate(){
            let now=i as f64*180.;retained.update(&mut p,v,"",now,true,false).unwrap();full.game_at(v,"",false,now);
            assert!(p.pixels==full.pixels,"retained trail at {w}x{h}, frame {i}");
        }
    }
    let mut tile=Painter::new();let mut full=Painter::new();let mut plan=render::tile_damage::TilePlanner::default();let mut old=None;
    for(i,v)in views.iter().enumerate(){
        plan.replace_view(old,v,&tile);plan.finish_damage(&mut tile);plan.paint_base(&mut tile,v,"",0.);tile.tile_clip_enabled=false;plan.clear_frame();
        full.game_at(v,"",false,0.);assert!(tile.pixels==full.pixels,"tile trail at frame {i}, byte {:?}",tile.pixels.iter().zip(&full.pixels).position(|(a,b)|a!=b));old=Some(v);
    }
}
#[test]fn r51_aim_gpu_commands_and_native_captures(){
    let r=room();let views=frames(&r);let mut p=Painter::new();p.gpu=Some(gpu_render::Frame::default());p.pixels.fill(17);
    p.game_at(&views[0],"",false,0.);assert!(p.pixels.iter().all(|&b|b==17));
    assert!(p.gpu.as_ref().unwrap().quads.iter().any(|q|q.color[..3]==[246,192,91]),"gold delivery uses GPU path");
    assert!(p.gpu.as_ref().unwrap().quads.iter().any(|q|q.color[..3]==[244,127,101]),"separate impact uses GPU path");
    if let Ok(dir)=std::env::var("CORPSE_AIM_REVIEW") {
        std::fs::create_dir_all(&dir).unwrap();let mut p=Painter::new();
        for (name,v) in ["firebolt","lightning","iceblast","cancel","bow","blink","cancel-again"].into_iter().zip(&views) {
            p.game_at(v,"",false,0.);let file=std::fs::File::create(format!("{dir}/aim-{name}.png")).unwrap();
            let mut enc=png::Encoder::new(file,p.width as u32,p.height as u32);enc.set_color(png::ColorType::Rgba);enc.set_depth(png::BitDepth::Eight);enc.write_header().unwrap().write_image_data(&p.pixels).unwrap();
        }
    }
}

#[test]fn r51_aim_context_changes_cancel_actions_and_ink_but_pack_selection_survives(){
    let r=room();let before=r.clone();let mut v=observe::observe(&r);
    for _context in ["Pack","Menu","Inspect","History","Ability details"] {
        let mut item=Some(pack::PackTarget::UseAt(900.));let mut ability=Some("numo-implant-thought".into());let mut attacker=Some(910.);let mut keyboard=true;
        sync_pointer_view(&mut v,&r,Some((11,10)),vec![],item.as_ref());assert!(!v.target_path.is_empty());
        assert!(cancel_aim_for_context(true,&mut item,&mut ability,&mut attacker,&mut keyboard));
        assert!(item.is_none()&&ability.is_none()&&attacker.is_none()&&!keyboard);
        sync_pointer_view(&mut v,&r,None,vec![],item.as_ref());sync_ability_preview(&mut v,&r,ability.as_deref(),attacker,item.is_some());
        assert!(v.target_path.is_empty()&&v.target_area.is_empty()&&v.target_impact.is_none());
        // Selecting a new action closes Pack and starts just that item mode.
        item=Some(pack::PackTarget::Shoot(1.));
        assert!(!cancel_aim_for_context(false,&mut item,&mut ability,&mut attacker,&mut keyboard));assert!(item.is_some()&&ability.is_none());
    }
    assert_eq!(r,before);
}
