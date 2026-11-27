//! Designer-requested quiet HUD and bidirectional observed-creature highlights.
use super::*;
fn run()->corpse_core::Run {
    let mut r=corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=50.0.into();r.offers.clear();
    r.player.abilities=["numo-implant-thought","anavani-illuminate","etill-phase-through","shrail-fight-long","ursa-dream-see"].into_iter().map(str::to_string).collect();
    r.player.active.clear();r.player.ability_reserves=corpse_core::Field::Missing;
    for id in r.player.abilities.clone(){corpse_core::revision50_ability_energy::initialize_reserve(&mut r.player,&id,50.,50.).unwrap();}
    r
}
fn capture(name:&str,p:&Painter){
    if let Ok(dir)=std::env::var("CORPSE_R51_VISUAL_OUTPUT"){
        std::fs::create_dir_all(&dir).unwrap();
        let file=std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
        let mut encoder=png::Encoder::new(file,p.width as u32,p.height as u32);
        encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
    }
}
#[test]
fn quiet_ui_toolbar_adds_help_with_four_evenly_spaced_labels(){
    assert_eq!(TOOLBAR_ACTIONS,["Pack","Bestiary","?","Menu"]);
    assert_eq!(TOOLBAR_VISIBLE_ACTIONS,[4,7,crate::help_ui::ACTION,9]);
    let mut p=Painter::new();
    for(w,h)in [(1280,800),(1920,1080),(1537,911),(960,1200)]{
        p.resize(w,h);p.set_responsive_canvas(true);
        for(slot,id)in TOOLBAR_VISIBLE_ACTIONS.iter().enumerate(){
            let r=p.hud_tracks().button(slot);
            assert_eq!(p.toolbar_action(r.0+r.2/2,r.1+r.3/2),Some(*id));
            assert_eq!(p.toolbar_action(r.0,r.1+r.3),None);
        }
    }
}
#[test]
fn quiet_ui_ability_metadata_uses_available_width_and_right_edge(){
    let p=Painter::new();let role=crate::typography_ui::Role::Secondary;
    let text="Reserve 50/50 · 15 NRG + action";
    let(state,cost,x)=p.ability_metadata("ON",text,100,352);
    assert_eq!(state,"ON");assert_eq!(cost,text);
    assert_eq!(x+p.measure_text_role(role,&cost,10).ceil()as i32,352);
    let(_,short,x)=p.ability_metadata("UNAVAILABLE",&text.repeat(6),100,352);
    assert!(short.ends_with('…'));assert!(x>=100);
    let(_,cost,x)=p.ability_metadata("",text,100,352);assert_eq!(cost,text);
    assert_eq!(x+p.measure_text_role(role,&cost,10).ceil()as i32,352);
    let full=ability_cost_text("3/3 fortunes · 1 fortune", "ON");
    let(_,cost,_)=p.ability_metadata("ON",&full,100,352);
    assert_eq!(cost,"3/3 fortunes · 1 fortune");
}
#[test]
fn quiet_ui_unused_reserves_are_hidden_without_mutating_them(){
    let mut r=run();r.player.ability_reserves=corpse_core::Field::Missing;
    for a in corpse_core::progression::catalog_for_revision(50.){
        corpse_core::revision50_ability_energy::initialize_reserve(&mut r.player,&a.id,50.,50.).unwrap();
    }
    let before=r.clone();
    for a in corpse_core::progression::catalog_for_revision(50.){
        let row=crate::ability_ui::observe(&r,&a.id);
        if a.id=="morae-see-the-future" {
            assert!(!row.cost.contains("Reserve"));
            assert!(matches!(row.indicator,Some(crate::ability_ui::CustomIndicator::Future(_))));
        } else { assert_eq!(row.cost.contains("Reserve"),crate::ability_ui::r50_uses_energy(&a.id),"{}",a.name); }
        assert!(!row.description.contains("Ability energy is spent first"),"{}",a.name);
    }
    for id in ["isheilah-quick-hand","ursa-sleep-walk","lichor-blood-letting","kative-appropriate"]{
        assert!(!crate::ability_ui::observe(&r,id).cost.contains("Reserve"));
    }
    assert_eq!(r,before);
}
#[test]
fn quiet_ui_condition_names_are_short_and_meaningful(){
    for(full,short)in [("Slow: acts every other turn","Slow"),("Slowed: acts every other turn","Slowed"),("Lucky: next attack","Lucky"),("Discord: hostile to all","Discord"),("Envenomed: 8 damage/turn","Envenomed"),("Remember: Slow","Remember: Slow"),("Squirming helplessly: outside habitat","Squirming helplessly"),("Shine · active","Shine"),("Bonded · linked to you","Bonded")]{
        assert_eq!(crate::statuses_ui::display_name(full),short);
    }
}
#[test]
fn quiet_ui_two_way_hover_restores_full_retained_and_tile_frames(){
    let r=run();let v=crate::observe::observe(&r);let frozen=r.clone();
    let id=v.creatures[0].id;let cell=v.cells.iter().find(|c|c.actor_id==Some(id)).unwrap();
    for(w,h)in [(1280,800),(1920,1080),(1537,911),(960,1200)]{
        let(mut p,mut full)=(Painter::new(),Painter::new());
        for p in [&mut p,&mut full]{p.resize(w,h);p.set_responsive_canvas(true);}
        let(cw,ch,ox,oy)=p.map_layout(v.width,v.height);
        assert_eq!(p.enemy_at_map_point(&v,ox+cell.x as i32*cw+cw/2,oy+cell.y as i32*ch+ch/2),Some(id.to_bits()));
        assert_eq!(p.enemy_at_map_point(&v,0,0),None);
        let mut retained=RetainedGame::default();let mut clear=None;
        for(step,selected)in [None,Some(id.to_bits()),None,Some(id.to_bits()),None].into_iter().enumerate(){
            p.enemy_hover=selected;full.enemy_hover=selected;
            retained.update(&mut p,&v,"",0.,step==0,step==0).unwrap();full.game_at(&v,"",false,0.);
            assert!(p.pixels==full.pixels,"retained {w}x{h}, step {step}");
            if selected.is_none(){if let Some(base)=&clear{assert_eq!(&p.pixels,base);}else{clear=Some(p.pixels.clone());}}
            else{
                let base=clear.as_ref().unwrap();assert_ne!(&p.pixels,base);
                let bounds=p.cell_damage_rect(cell,v.width,v.height).unwrap();
                assert!((bounds.y..bounds.bottom()).any(|yy|(bounds.x..bounds.right()).any(|xx|{
                    let at=(yy*p.width+xx)*4;p.pixels[at..at+4]!=base[at..at+4]
                })),"profile hover highlights the actual monster tile");
                capture(&format!("hover-{w}x{h}"),&p);
            }
        }
        let mut plan=tile_damage::TilePlanner::default();let mut keys=None;
        plan.replace_view(None,&v,&p);
        for selected in [None,Some(id.to_bits()),None]{
            p.enemy_hover=selected;full.enemy_hover=selected;
            let current=crate::tile_overlay::PanelKeys::read(&p);current.mark_changes(keys,&p,&v,&mut plan);
            plan.finish_damage(&mut p);plan.paint_base(&mut p,&v,"",0.);p.tile_clip_enabled=false;
            full.game_at(&v,"",false,0.);assert!(p.pixels==full.pixels,"tile hover {w}x{h}");
            plan.clear_frame();keys=Some(current);
        }
    }
    assert_eq!(r,frozen);
}
#[test]
fn quiet_ui_large_red_corpse_marker_and_pale_heart_rings_stay_tile_bounded(){
    for scale in [1.,1.5,2.] {
        let mut p=Painter::new();p.resize((1280.*scale)as usize,(800.*scale)as usize);
        let bounds=|p:&Painter|{
            let coords:Vec<_>=p.pixels.chunks_exact(4).enumerate().filter(|(_,v)|v[..3]!=[0,0,0]).map(|(i,_)|(i%p.width,i/p.width)).collect();
            let tile=(p.native_x(40.)as usize,p.native_y(50.)as usize,p.native_x(55.)as usize,p.native_y(74.)as usize);
            assert!(coords.iter().all(|&(x,y)|x>=tile.0&&x<tile.2&&y>=tile.1&&y<tile.3),"marker ink must stay inside its native tile at scale {scale}");
            (coords.iter().map(|v|v.0).max().unwrap()-coords.iter().map(|v|v.0).min().unwrap()+1,
             coords.iter().map(|v|v.1).max().unwrap()-coords.iter().map(|v|v.1).min().unwrap()+1)
        };
        let red=|v:&[u8]|v[0]>v[1].saturating_add(45)&&v[0]>v[2].saturating_add(45);
        p.clear([0,0,0]);{ let bounds=corpse_marker_tests::marker_test_bounds(&mut p); p.corpse_marker(bounds,40,50,15,24,None); };let cross=bounds(&p);
        // Latest design replaces the former small-X cap: endpoints span 60%
        // of the tile width, with a narrow antialiased stroke and outline.
        for extent in [cross.0,cross.1] {
            assert!(extent as f64>=9.*scale&&extent as f64<=12.*scale+1.,"large cross extent at scale {scale}: {cross:?}");
        }
        assert!(p.pixels.chunks_exact(4).any(red),"corpse cross has red ink at scale {scale}");
        for glyph in ["r","^","y","^^"] {
            p.clear([0,0,0]);p.heartless_marker(glyph,40,50,15,24,None);let ring=bounds(&p);
            assert!(ring.0<cross.0&&ring.1<cross.1,"heartless ring stays smaller: {scale} {glyph}: {ring:?} vs {cross:?}");
            assert!(!p.pixels.chunks_exact(4).any(red),"heartless ring must not inherit the corpse red: {scale} {glyph}");
            assert!(p.pixels.chunks_exact(4).any(|v|v[0]>80&&v[1]>70&&v[2]>50),"pale ring remains visible: {scale} {glyph}");
        }
    }
}

#[test]
fn quiet_ui_prepaid_activation_remains_available_at_exact_funding(){
    let mut r=run();let id="etill-phase-through";r.player.energy=0.;
    r.player.ability_reserves.as_mut().unwrap().get_mut(id).unwrap().energy=5.;
    assert!(corpse_core::abilities::prepay_activation_upkeep(&mut r,id,5.));
    let row=crate::ability_ui::observe(&r,id);
    assert!(!row.state.contains("Unavailable"));assert!(row.cost.contains("5 NRG/t"));
    assert!(row.cost.contains("Reserve 0/50"));
}

#[test]
fn quiet_ui_map_hover_can_reveal_a_scrolled_off_profile(){
    let r=run();let mut v=crate::observe::observe(&r);let template=v.creatures[0].clone();
    v.creatures=(0..25).map(|i|{let mut a=template.clone();a.id=1000.+i as f64;a}).collect();
    let p=Painter::new();let(_,_,ox,_)=p.map_layout(v.width,v.height);
    let layout=enemy_list::Layout::resolve_frame(&p.profile,ox,p.frame());let content=layout.content(&p,&v.creatures);
    let offset=content.reveal(24,0);assert!(offset>0);
    assert_eq!(content.hit(layout.rows.0+4,layout.rows.1+layout.rows.3-10,offset),Some(24));
    assert_eq!(content.reveal(0,offset),0);
}
#[test]
fn quiet_ui_player_stats_use_effective_bonus_units_without_world_changes(){
    let mut r=run();r.inventory.clear();r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.stats.offense=5.;r.player.stats.defense=2.;r.player.stats.max_hp=12.0.into();
    r.player.stats.dr=3.;r.player.stats.accuracy=4.;r.player.stats.dodge=6.;
    let before=r.clone();let v=crate::observe::observe(&r);
    assert_eq!(v.player_stats,[5.,12.,3.,4.,6.]);assert_eq!(r,before);
}
#[test]
fn quiet_ui_unread_ability_and_stat_changes_repaint_all_responsive_modes(){
    let r=run();
    for(w,h)in [(1280,800),(1920,1080),(960,1200)]{
        let(mut p,mut full)=(Painter::new(),Painter::new());for p in [&mut p,&mut full]{p.resize(w,h);p.set_responsive_canvas(true);}
        let mut v=crate::observe::observe(&r);let mut retained=RetainedGame::default();
        for(step,(hover,unread))in [(None,false),(Some(0),false),(Some(0),true),(None,true),(None,false)].into_iter().enumerate(){
            p.ability_hover=hover;p.bestiary_unread=unread;full.ability_hover=hover;full.bestiary_unread=unread;
            v.player_stats[0]=step as f64;
            retained.update(&mut p,&v,"",0.,true,step==0).unwrap();full.game_at(&v,"",false,0.);
            assert!(p.pixels==full.pixels,"retained panels {w}x{h} step {step}");
        }
        let mut plan=tile_damage::TilePlanner::default();let mut keys=None;plan.replace_view(None,&v,&p);
        for(hover,unread)in [(None,false),(Some(0),false),(Some(0),true),(None,false)]{
            p.ability_hover=hover;p.bestiary_unread=unread;full.ability_hover=hover;full.bestiary_unread=unread;
            let current=crate::tile_overlay::PanelKeys::read(&p);current.mark_changes(keys,&p,&v,&mut plan);
            plan.finish_damage(&mut p);plan.paint_base(&mut p,&v,"",0.);p.tile_clip_enabled=false;
            full.game_at(&v,"",false,0.);assert!(p.pixels==full.pixels,"tile panels {w}x{h}");
            plan.clear_frame();keys=Some(current);
        }
    }
    assert_eq!(crate::chronicle::color("Bestiary Pages Added!"),[193,138,255]);
}

#[test]
fn water_squiggle_is_cosmetic_bounded_and_repaints_without_trails(){
    let r=run();let frozen=r.clone();let mut v=crate::observe::observe(&r);
    let cell=v.cells.iter_mut().find(|c|c.actor_id.is_none()&&c.current).unwrap();
    cell.glyph="~".into();cell.animations=vec![Animation::Water{phase:0.7,speed:1.}];
    for(w,h)in [(1280,800),(1920,1080),(960,1200)]{
        let(mut p,mut full)=(Painter::new(),Painter::new());
        for p in [&mut p,&mut full]{p.resize(w,h);p.set_responsive_canvas(true);}
        let mut retained=RetainedGame::default();let mut first=None;
        for(i,now)in [100.,800.,1500.,2200.].into_iter().enumerate(){
            retained.update(&mut p,&v,"",now,i==0,i==0).unwrap();full.game_at(&v,"",false,now);
            assert!(p.pixels==full.pixels,"water retained {w}x{h} {now}");
            if let Some(first)=first.as_ref(){assert_ne!(&p.pixels,first);}else{first=Some(p.pixels.clone());}
            capture(&format!("water-{w}x{h}-{i}"),&p);
        }
        let mut plan=tile_damage::TilePlanner::default();plan.replace_view(None,&v,&p);
        for now in [100.,800.,1500.]{plan.mark_animation(&v,&p,now,false);plan.finish_damage(&mut p);plan.paint_base(&mut p,&v,"",now);p.tile_clip_enabled=false;full.game_at(&v,"",false,now);assert!(p.pixels==full.pixels,"water tile {w}x{h} {now}");plan.clear_frame();}
        p.clear([0,0,0]);p.water_wave(100,100,15,24,[255;3],1.);
        let bounds=p.logical_damage_rect(100,100,15,24).unwrap();
        for(i,pixel)in p.pixels.chunks_exact(4).enumerate(){if pixel[..3]!=[0,0,0]{assert!(bounds.contains(i%p.width,i/p.width));}}
    }
    assert_eq!(r,frozen);
}
