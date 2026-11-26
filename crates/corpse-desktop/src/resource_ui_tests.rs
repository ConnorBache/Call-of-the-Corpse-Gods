use super::*;
use corpse_core::{Field,Run};
use crate::render::{RetainedGame,tile_damage::TilePlanner};
const ID:&str="pelian-never-surrender";
fn room()->Run{
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=52.0.into();r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.hp=191.9;r.player.max_hp=202.;r.player.base_max_hp=202.0.into();r.player.energy=80.;
    r.player.stats.max_energy=100.;r.inventory.clear();r.offers.clear();
    for a in r.actors.values_mut(){*a=Default::default();}
    r.player.abilities.extend([ID.into(),"shrail-rage-at-the-dying-of-the-light".into()]);
    corpse_core::revision50_ability_energy::reconcile_acquired(&mut r).unwrap();
    corpse_core::resources::begin_resource_turn(&mut r);r.player.hp=202.;r.player.energy=60.;
    for id in [ID,"shrail-rage-at-the-dying-of-the-light"] {r.player.ability_reserves.as_mut().unwrap().get_mut(id).unwrap().energy=0.;}r
}
fn save_png(p:&Painter,name:&str){
    if let Ok(dir)=std::env::var("CORPSE_RESOURCE_REVIEW"){
        std::fs::create_dir_all(&dir).unwrap();
        let mut encoder=png::Encoder::new(std::fs::File::create(format!("{dir}/{name}.png")).unwrap(),p.width as u32,p.height as u32);
        encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
    }
}
#[test]
fn resource_ui_formats_capped_positive_negative_zero_and_small_changes(){
    let r=room();assert_eq!(player_changes(&r)[0],"+5%");assert_eq!(player_changes(&r)[1],"-20%");
    assert_eq!(value_text("Health",202.,202.,"+5%"),"202/202 - +5%");
    assert_eq!(value_text("Health",0.,202.,"-100%"),"0/202 - -100%");
    assert_eq!(value_text("Health",-12.,202.,""),"-12 / 202");
    assert_eq!(value_text("Health",-0.25,202.,""),"-1 / 202");
    assert_eq!(bar_fractions(-12.,202.,0.),(0.,0.));
    assert_eq!(percent_text(Some(0.)),"");assert_eq!(percent_text(None),"");
    assert_eq!(percent_text(Some(0.04)),"+0%");assert_eq!(percent_text(Some(-0.5)),"-1%");
    assert_eq!(percent_text(Some(1e-12)),"+0%");
    assert_eq!(percent_text(Some(1e9)),"+1000000000%");
    assert_eq!(value_text("Health",0.25,202.,"-99%"),"1/202 - -99%");
    assert_eq!(crate::ability_ui::reserve_summary(&r,ID).unwrap(),"Reserve 0/10 - -100%");
    let before=r.clone();let _=crate::observe::observe(&r);assert!(r==before,"observation mutated the Run");
}
#[test]
fn resource_ui_full_zero_and_only_delta_changes_repaint_at_normal_and_portrait_sizes(){
    for(w,h)in[(1280,800),(960,1200)]{
        let mut r=room();let mut old=crate::observe::observe(&r);
        let(mut retained_p,mut tile,mut full)=(Painter::new(),Painter::new(),Painter::new());
        for p in [&mut retained_p,&mut tile,&mut full]{p.try_resize(w,h).unwrap();p.set_responsive_canvas(true);}
        let mut retained=RetainedGame::default();let mut plan=TilePlanner::default();
        retained.update(&mut retained_p,&old,"",0.,true,true).unwrap();
        plan.replace_view(None,&old,&tile);plan.finish_damage(&mut tile);plan.paint_base(&mut tile,&old,"",0.);tile.tile_clip_enabled=false;plan.clear_frame();
        full.game_at(&old,"",false,0.);assert!(retained_p.pixels==full.pixels,"initial retained frame mismatch");assert!(tile.pixels==full.pixels,"initial tile frame mismatch");
        save_png(&full,&format!("resource-gains-{w}x{h}"));
        for step in 0..4{
            match step{
                0=>corpse_core::resources::begin_resource_turn(&mut r), // Values stay full/zero; only percentages disappear.
                1=>{r.player.hp=0.;r.player.energy=0.;},
                2=>corpse_core::resources::begin_resource_turn(&mut r), // Values stay zero; only percentages disappear.
                _=>{r.player.hp=202.;r.player.energy=100.;r.player.ability_reserves.as_mut().unwrap().get_mut(ID).unwrap().energy=10.;},
            }
            let v=crate::observe::observe(&r);full.game_at(&v,"",false,0.);
            retained.update(&mut retained_p,&v,"",0.,true,false).unwrap();
            plan.replace_view(Some(&old),&v,&tile);plan.finish_damage(&mut tile);plan.paint_base(&mut tile,&v,"",0.);tile.tile_clip_enabled=false;plan.clear_frame();
            assert!(retained_p.pixels==full.pixels,"retained {w}x{h} step {step}");
            assert!(tile.pixels==full.pixels,"tile {w}x{h} step {step}");old=v;
            save_png(&full,&format!("resource-step-{step}-{w}x{h}"));
        }
        let left=full.hud_left()+12;let right=full.logical_width()-14;
        for label in ["Health","Energy","Blessings"]{
            let start=left+text_width(&full,label,14)+8;
            for value in ["202/202 - +5%","0/202 - -100%","10000/10000 - +100%"]{
                let(text,size,x)=fit_value(&full,value,start,right);
                assert!(x>=start);assert!(x+text_width(&full,&text,size)<=right,"{value} at {w}x{h}");
            }
        }
        let huge="1234567890123456789/1234567890123456789 - +100%";
        let start=left+text_width(&full,"Blessings",14)+8;
        let(text,size,x)=fit_value(&full,huge,start,right);
        assert!(text.starts_with('…'));assert!(text.ends_with(" - +100%"));
        assert!(x>=start&&x+text_width(&full,&text,size)<=right);
        old.hp=1e18;old.max_hp=1e18;old.energy=1e18;old.max_energy=1e18;
        old.blessings=1e18;old.blessing_goal=1e18;old.resource_changes=["+1000000000%".into(),"-100%".into(),"+100%".into()];
        full.game_at(&old,"",false,0.);save_png(&full,&format!("resource-large-{w}x{h}"));
    }
}
#[test]
fn resource_ui_amulet_display_tracks_identity_and_pack_refresh_at_reset(){
    let mut r=room();let mut item=corpse_core::Item{id:900.,kind:"amulet".into(),effect:"lightning".to_string().into(),name:"Amulet of Lightning".into(),..Default::default()};
    corpse_core::revision50_amulets::initialize_item_for_revision(&mut item,52.).unwrap();r.inventory.push(item);
    corpse_core::resources::begin_resource_turn(&mut r);r.inventory[0].amulet_state.as_mut().unwrap().energy=0.;
    let text=crate::pack::describe_item(&r,&r.inventory[0]);assert!(text.contains("Amulet energy: 0/10 - -100%"));
    let mut ground=r.clone();let dropped=ground.inventory.pop().unwrap();
    assert!(!crate::pack::describe_item(&ground,&dropped).contains("-100%")); // Ground/memory prose is not a player meter.
    for(w,h)in[(1280,800),(960,1200)] {
        let mut painter=Painter::new();painter.try_resize(w,h).unwrap();painter.set_responsive_canvas(true);
        let mut shown=crate::pack::PackView::observe(&r);shown.configure(&painter,painter.frame().map.tuple());
        assert_eq!(shown.key(sdl2::keyboard::Keycode::Home,sdl2::keyboard::Mod::NOMOD),crate::pack::PackAction::None);
        painter.game_at(&crate::observe::observe(&r),"",false,0.);shown.paint(&mut painter);
        save_png(&painter,&format!("resource-amulet-pack-{w}x{h}"));
    }
    let mut pack=crate::pack::PackView::observe(&r);let p=Painter::new();pack.configure(&p,p.frame().map.tuple());
    let before=pack.paint_state(&p);corpse_core::resources::begin_resource_turn(&mut r);pack.refresh(&r);
    assert!(!crate::pack::describe_item(&r,&r.inventory[0]).contains("-100%"));
    // Same item and tick: the baseline-only change must still refresh projected prose.
    let after=pack.paint_state(&p);assert!(!after.changes(&before).is_empty());
    r.resource_history=Field::Missing;assert!(!crate::pack::describe_item(&r,&r.inventory[0]).contains("-100%"));
}

#[test]
fn thick_resource_labels_are_inside_and_respect_parent_clip_at_all_fill_levels() {
    for fraction in [0.,0.02,0.5,1.] {
        let mut p=Painter::new();p.clear([3,4,5]);
        let original=p.pixels.clone();
        p.clip=Some((104,101,210,22));
        paint_bar(&mut p,"Health","50/100",100,100,220,24,[255,48,85],fraction,13);
        assert_eq!(p.clip,Some((104,101,210,22)));
        assert_ne!(p.pixels,original);
        for y in 0..p.height {for x in 0..p.width {
            if !(104..314).contains(&x)||!(101..123).contains(&y) {
                let i=(y*p.width+x)*4;assert_eq!(&p.pixels[i..i+4],&original[i..i+4]);
            }
        }}
    }
}

#[test]
fn pending_upkeep_hatch_and_forecast_only_changes_repaint_both_paths() {
    let r=room();let mut old=crate::observe::observe(&r);
    let(mut full,mut retained_p,mut tile)=(Painter::new(),Painter::new(),Painter::new());
    let mut retained=RetainedGame::default();let mut plan=TilePlanner::default();
    retained.update(&mut retained_p,&old,"",0.,true,true).unwrap();
    plan.replace_view(None,&old,&tile);plan.finish_damage(&mut tile);plan.paint_base(&mut tile,&old,"",0.);tile.tile_clip_enabled=false;plan.clear_frame();
    for loss in [20.,1.,0.,old.energy] {
        let mut v=crate::observe::observe(&r);v.pending_energy_loss=loss;
        if let Some(a)=v.abilities.first_mut(){a.mode="toggle".into();a.reserve=Some((10.,10.));a.pending_reserve_loss=loss.min(10.);}
        full.game_at(&v,"",false,0.);retained.update(&mut retained_p,&v,"",0.,true,false).unwrap();
        plan.replace_view(Some(&old),&v,&tile);plan.finish_damage(&mut tile);plan.paint_base(&mut tile,&v,"",0.);tile.tile_clip_enabled=false;plan.clear_frame();
        assert_eq!(retained_p.pixels,full.pixels);assert_eq!(tile.pixels,full.pixels);old=v;
    }
    let mut p=Painter::new();p.clear([1,2,3]);paint_bar_pending(&mut p,"Energy","60/100",100,100,200,24,[0,229,255],0.6,13,0.2);
    let at=(100*p.width+180)*4;assert_eq!(&p.pixels[at..at+3],&[110,90,235]);
    let at=(100*p.width+219)*4;assert_eq!(&p.pixels[at..at+3],&[110,90,235]);
    let at=(100*p.width+220)*4;assert_eq!(&p.pixels[at..at+3],&[36,41,53]);
}

#[test]
fn ability_bar_height_tracks_enclosed_names_and_totals_across_sizes() {
    for (w,h) in [(1280,800),(1920,1080),(960,1200),(2560,1600)] {
        let mut p=Painter::new();p.resize(w,h);p.set_responsive_canvas(true);
        let width=p.logical_width()-p.hud_left()-24;
        for name in ["Execute","I Came For You","Ragelight","Turning the Wheel","Never Ending Flesh"] {
            let (label,height)=ability_bar_layout(&p,name,"10/10 NRG",width,13);
            assert_eq!(label.lines().collect::<Vec<_>>().join(" "),name);
            assert!(height<=crate::ability_ui::row(&p.profile));
            assert_eq!(height,if label.contains('\n'){(label.lines().count() as i32+1)*15+4}else{19});
            for line in label.lines(){assert!(text_width(&p,line,13)<=width-12);}
            p.clear([1,2,3]);paint_bar_pending(&mut p,&label,"10/10 NRG",100,100,width,height,[255,51,77],1.,13,0.5);
            // Text is strictly enclosed by the exact content-sized bar clip.
            let bottom=p.native_y((100+height)as f64) as usize;
            let left=p.native_x(100.)as usize;let right=p.native_x((100+width)as f64)as usize;
            for y in bottom..bottom+3 {for x in left..right {let at=(y*p.width+x)*4;assert_eq!(&p.pixels[at..at+3],&[1,2,3]);}}
        }
    }
}

#[test]
fn boundless_above_100_keeps_infinity_and_fixed_scale_with_truthful_upkeep() {
    let mut r=room();r.player.abilities.push("grisha-boundless-hunger".into());
    let id="morae-turning-the-wheel";r.player.abilities.push(id.into());r.player.active.push(id.into());
    corpse_core::revision50_ability_energy::initialize_reserve(&mut r.player,id,10.,0.).unwrap();
    r.player.energy=250.;
    r.player.ability_reserves.as_mut().unwrap().get_mut(id).unwrap().energy=5.;
    assert_eq!(corpse_core::abilities::projected_player_upkeep(&r),15.);
    assert_eq!(bar_fractions(250.,f64::INFINITY,15.),(1.,0.15));
    r.player.ability_reserves.as_mut().unwrap().get_mut(id).unwrap().energy=0.;
    for energy in [250.,110.,90.] {
        r.player.energy=energy;let before=r.clone();let v=crate::observe::observe(&r);
        assert!(v.max_energy.is_infinite());assert_eq!(v.pending_energy_loss,20.);assert_eq!(r,before);
        let(fill,pending)=bar_fractions(energy,v.max_energy,v.pending_energy_loss);
        assert_eq!(fill,if energy>=100.{1.}else{0.9});
        assert!((pending-0.2).abs()<1e-12);
        assert_eq!(bar_fractions(energy,f64::INFINITY,150.).1,fill);
        let label=value_text("Energy",energy,v.max_energy,"");assert!(label.contains('∞'));assert!(label.starts_with(&format!("{energy:.0}")));
        for(w,h)in[(1280,800),(1920,1080),(960,1200)] {
            let mut p=Painter::new();p.resize(w,h);p.set_responsive_canvas(true);
            let left=p.hud_left()+12;let width=p.logical_width()-p.hud_left()-24;
            let(text,size,x)=fit_value(&p,&label,left+text_width(&p,"Energy",13)+14,left+width-6);
            assert_eq!(text,label);assert!(x+text_width(&p,&text,size)<=left+width-6);
            p.game_at(&v,"",false,0.);save_png(&p,&format!("boundless-{energy:.0}-{w}x{h}"));
        }
    }
}

#[test]
fn upcoming_hatch_preserves_player_and_reserve_fill_between_indigo_strokes(){
 for color in [[0,229,255],[255,51,77],[255,209,57]] {
  let mut p=Painter::new();p.clear([1,2,3]);
  paint_bar_pending(&mut p,"","",100,100,200,24,color,1.,13,0.5);
  // Sample below text: warning area must contain both unchanged fill and indigo strokes.
  let mut base=0;let mut stroke=0;
  for y in 117..122 {for x in 201..298 {let at=(y*p.width+x)*4;let rgb=&p.pixels[at..at+3];if rgb==color{base+=1;}else if rgb==[110,90,235]{stroke+=1;}else{panic!("unexpected tint {rgb:?}");}}}
  assert!(base>stroke&&stroke>0);
 }
}

#[test]
fn adjacent_reserve_bar_is_one_third_width_and_preserves_count_and_name() {
    for(w,h)in[(1280,800),(1920,1080),(960,1200),(2560,1600)]{
        let mut p=Painter::new();p.resize(w,h);p.set_responsive_canvas(true);
        let left=p.hud_left();let rail=p.logical_width()-left-24;let width=rail/3;
        let x=left+12+rail-width;let name_x=left+18;let available=x-name_x-8;
        assert!(width*3<=rail&&rail-width*3<3);
        for name in ["Ragelight","I Came For You","Turning the Wheel","Never Ending Flesh"] {
            let lines=p.wrap_text_role(crate::typography_ui::Role::Ability,name,available,13);
            assert_eq!(lines.join(" "),name);
            for line in lines{assert!(text_width(&p,&line,13)<=available);}
        }
        let(value,size,value_x)=fit_value(&p,"10/10 NRG",x+14,x+width-6);
        assert_eq!(value,"10/10 NRG");assert!(value_x+text_width(&p,&value,size)<=x+width-6);
    }
}

#[test]
fn ability_enable_circle_for_reserve_and_no_reserve_but_not_passive(){
    let r=room();
    for(w,h)in[(1280,800),(1920,1080),(960,1200)]{
        let mut p=Painter::new();p.resize(w,h);p.set_responsive_canvas(true);
        let mut v=crate::observe::observe_with_profile(&r,&p.profile);v.abilities.truncate(1);
        for reserve in [None,Some((10.,10.))] {
            for passive in [false,true] {
                v.abilities[0].mode=if passive{"passive"}else{"toggle"}.into();v.abilities[0].reserve=reserve;
                v.abilities[0].active=false;p.game_at(&v,"",false,0.);let off=p.pixels.clone();
                v.abilities[0].active=true;p.game_at(&v,"",false,0.);
                if passive {assert_eq!(off,p.pixels,"passive must have no enabled decoration");}
                else {
                    assert_ne!(off,p.pixels,"both reserve kinds need on/off indication");
                    assert!(p.pixels.chunks_exact(4).any(|c|c[..3]==[100,255,114]));
                }
            }
        }
    }
}

#[test]
fn fortune_boxes_track_charges_single_countdown_and_retained_updates(){
    let mut r=room();r.player.abilities=vec!["morae-fortune-teller".into()];r.player.ability_order=corpse_core::Field::Missing;
    for(w,h)in[(1280,800),(1920,1080),(960,1200)]{
        let(mut p,mut full,mut tile)=(Painter::new(),Painter::new(),Painter::new());
        for p in [&mut p,&mut full,&mut tile]{p.resize(w,h);p.set_responsive_canvas(true);}
        r.player.fortune_schedule=corpse_core::Field::Missing;
        let mut retained=RetainedGame::default();let mut plan=TilePlanner::default();let mut old=None;
        for(count,tick,until)in[(3.,0.,0),(2.,13.,20),(1.,19.,14),(0.,20.,13),(1.,21.,12)] {
            r.tick=tick;r.player.conditions.insert("fortunes".into(),count);
            corpse_core::fortune_schedule::ensure(&mut r.player,tick);
            let before=r.clone();let v=crate::observe::observe_with_profile(&r,&p.profile);assert!(r==before);
            assert_eq!(v.abilities[0].fortunes,Some((count as u8,until)));
            full.game_at(&v,"",false,0.);retained.update(&mut p,&v,"",0.,true,false).unwrap();
            plan.replace_view(old.as_ref(),&v,&tile);plan.finish_damage(&mut tile);plan.paint_base(&mut tile,&v,"",0.);tile.tile_clip_enabled=false;plan.clear_frame();
            assert!(p.pixels==full.pixels);assert!(tile.pixels==full.pixels);old=Some(v);
        }
    }
}

#[test]
fn amulet_compact_meter_reads_physical_reserve_and_pack_refreshes(){
    let mut r=room();let mut item=corpse_core::Item{id:901.,kind:"amulet".into(),effect:"lightning".to_string().into(),name:"Tier 5 Amulet of Lightning with a long name".into(),..Default::default()};
    corpse_core::revision50_amulets::initialize_item_for_revision(&mut item,52.).unwrap();
    assert_eq!(amulet_reserve(&item,52.),Some((10.,10.)));assert_eq!(amulet_reserve(&item,49.),None);
    assert_eq!(amulet_reserve(&corpse_core::Item::default(),52.),None);
    for(w,h)in[(1280,800),(1920,1080),(960,1200)]{
        let mut p=Painter::new();p.resize(w,h);p.set_responsive_canvas(true);
        r.inventory=vec![item.clone()];let mut pack=crate::pack::PackView::observe(&r);pack.configure(&p,p.frame().map.tuple());
        let mut old=None;
        for energy in [10.,5.,0.] {
            r.inventory[0].amulet_state.as_mut().unwrap().energy=energy;
            assert_eq!(amulet_reserve(&r.inventory[0],52.),Some((energy,10.)));
            pack.refresh(&r);pack.configure(&p,p.frame().map.tuple());p.clear([8,7,13]);pack.paint(&mut p);
            if let Some(previous)=old {assert_ne!(previous,p.pixels);}old=Some(p.pixels.clone());
            save_png(&p,&format!("amulet-pack-{w}x{h}-{energy}"));
        }
    }
}

#[test]
fn targeting_orb_orange_and_retained_clear_at_multiple_sizes(){
 let mut r=room();r.player.abilities=vec!["morae-fortune-teller".into()];r.player.ability_order=Field::Missing;
 for(w,h)in[(1280,800),(1920,1080),(960,1200)] {
  let(mut p,mut full,mut tile)=(Painter::new(),Painter::new(),Painter::new());for p in [&mut p,&mut full,&mut tile]{p.resize(w,h);p.set_responsive_canvas(true);}
  let mut retained=RetainedGame::default();let mut plan=TilePlanner::default();let mut old=None;
  for selected in [false,true,false] {
   let mut v=crate::observe::observe_with_profile(&r,&p.profile);assert_ne!(v.abilities[0].mode.to_lowercase(),"passive");
   crate::sync_ability_preview(&mut v,&r,if selected{Some("morae-fortune-teller")}else{None},None,false);
   assert_eq!(v.selected_ability.is_some(),selected);
   full.game_at(&v,"",false,0.);retained.update(&mut p,&v,"",0.,true,false).unwrap();
   plan.replace_view(old.as_ref(),&v,&tile);plan.finish_damage(&mut tile);plan.paint_base(&mut tile,&v,"",0.);tile.tile_clip_enabled=false;plan.clear_frame();
   assert_eq!(p.pixels,full.pixels);assert_eq!(tile.pixels,full.pixels);
   if selected{assert!(p.pixels.chunks_exact(4).any(|c|c[..3]==[255,155,56]));save_png(&p,&format!("target-orb-{w}x{h}"));}
   old=Some(v);
  }
 }
}

#[test]
fn cogworks_modes_and_rally_native_projection(){
 let mut r=room();r.player.abilities=vec!["kative-cogworks".into()];r.player.ability_order=Field::Missing;r.rally=Some(corpse_core::RunRally{x:r.player.x,y:r.player.y});
 for(w,h)in[(1280,800),(1920,1080),(960,1200)]{
  let(mut p,mut full,mut tile)=(Painter::new(),Painter::new(),Painter::new());for p in [&mut p,&mut full,&mut tile]{p.resize(w,h);p.set_responsive_canvas(true);}let mut retained=RetainedGame::default();let mut plan=TilePlanner::default();let mut old=None;
  for mode in 1..=3 {
   assert!(corpse_core::revision50_terrain::select_mode(&mut r,mode as f64));let mut v=crate::observe::observe_with_profile(&r,&p.profile);v.selected_ability=Some("kative-cogworks".into());assert_eq!(v.cogworks_mode,mode);assert!(v.cells.iter().any(|c|c.glyph=="@"&&c.superscript.contains('⚑')));
   full.game_at(&v,"",false,0.);retained.update(&mut p,&v,"",0.,true,false).unwrap();plan.replace_view(old.as_ref(),&v,&tile);plan.finish_damage(&mut tile);plan.paint_base(&mut tile,&v,"",0.);tile.tile_clip_enabled=false;plan.clear_frame();assert_eq!(p.pixels,full.pixels);assert_eq!(tile.pixels,full.pixels);if mode==2{save_png(&p,&format!("cogworks-rally-{w}x{h}"));}old=Some(v);
  }
 }
}
