//! Direct Run projections only: no production R50 admission or SDL event loop.
use super::*;
use corpse_core::revision50_amulet_effects::{self as amulets, Effect};

fn run() -> Run {
    let mut r: Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=50.0.into();r.inventory.clear();r.actors=Default::default();
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.x=10.;r.player.y=10.;r.player.energy=100.;
    let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.seen.fill(true);
    f.doors=Field::Missing;f.portals=Field::Missing;f.environment=Field::Missing;
    r
}
fn capture(p:&Painter,name:&str) {
    if let Ok(dir)=std::env::var("CORPSE_R50_JEWELRY_OUTPUT") {
        std::fs::create_dir_all(&dir).unwrap();
        let file=std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
        let mut enc=png::Encoder::new(file,p.width as u32,p.height as u32);
        enc.set_color(png::ColorType::Rgba);enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(&p.pixels).unwrap();
    }
}
fn selected(r:&Run,p:&mut Painter,id:f64)->PackView {
    p.game_at(&crate::observe::observe(r),"",false,0.);
    let mut pack=PackView::observe(r);pack.configure(p,p.frame().map.tuple());
    pack.selected=Some(id);pack
}
fn paint_capture(r:&Run,p:&mut Painter,pack:&PackView,name:&str) {
    p.game_at(&crate::observe::observe(r),"",false,0.);pack.paint(p);
    let first=p.pixels.clone();capture(p,name);
    p.game_at(&crate::observe::observe(r),"",false,0.);pack.paint(p);
    assert_eq!(p.pixels,first,"repeatable {name}");
}
#[test]
fn revision50_jewelry_capture_five_carried_categories() {
    let mut r=run();
    r.inventory=vec![
        Item{id:905.,kind:"heart".into(),name:"Rat heart".into(),..Default::default()},
        Item{id:904.,kind:"potion".into(),effect:"healing".to_string().into(),quantity:2.0.into(),..Default::default()},
        Item{id:903.,kind:"ring".into(),effect:"light".to_string().into(),name:"Ring of Light".into(),..Default::default()},
        amulets::make_amulet(Effect::Lightning,902.).unwrap(),
        equipment::make_arrows(901.,500.),
    ];
    let before=serde_json::to_vec(&r).unwrap();let mut p=Painter::new();let pack=selected(&r,&mut p,903.);
    assert_eq!(pack.stamp.category_headers,vec!["Trinkets 1 · unlimited","Amulets 1/5","Rings 1/5","Potions 2/5","Hearts 1/5"]);
    assert_eq!(pack.display_order(),vec![901.,902.,903.,904.,905.]);
    assert!(pack.capacity_heading().is_empty());
    assert!(pack.section_positions().iter().all(|(_,y)|*y+24<=pack.layout.list.1+pack.layout.list.3));
    assert!(pack.groups.iter().filter(|g|matches!(g.item.kind.as_str(),"ring"|"amulet"))
        .all(|g|g.description.lines().nth(1)==Some(titlecase(&g.item.kind).as_str())
            &&!g.description.contains("while carried")&&!g.actions.iter().any(|b|matches!(b.label.as_str(),"Wear"|"Take off"))));
    paint_capture(&r,&mut p,&pack,"five-carried-categories");
    assert_eq!(serde_json::to_vec(&r).unwrap(),before);
}
#[test]
fn revision50_jewelry_capture_concise_ring_copy_and_long_grants() {
    let mut r=run();r.player.abilities.push("isheilah-tools-of-the-trade".into());
    for (id,effect) in [(901.,"light"),(902.,"light"),(903.,"regeneration"),(904.,"regeneration"),(905.,"transference"),(906.,"ice-immunity")] {
        r.inventory.push(Item{id,kind:"ring".into(),name:format!("Ring of {effect}"),effect:effect.to_string().into(),..Default::default()});
    }
    r.inventory[2].granted_ability="shrail-execute".to_string().into();
    r.inventory[2].additional_granted_abilities=vec!["kative-appropriate".into(),"numo-implant-thought".into()];
    let before=serde_json::to_vec(&r).unwrap();let mut p=Painter::new();
    for (id,name,fragment) in [(901.,"light-duplicates","Adds 10 illumination"),(903.,"long-ring-top","Doubles your natural HP regeneration."),(905.,"vampirism","Ring of Vampirism"),(906.,"ice-immunity","Ring of Ice Immunity")] {
        let mut pack=selected(&r,&mut p,id);
        assert!(pack.selected_group().unwrap().description.contains(fragment));
        paint_capture(&r,&mut p,&pack,name);
        if id==903. {
            assert_eq!(pack.detail_max_scroll(),0,"grant names fit without duplicated ability prose");
            let text=&pack.selected_group().unwrap().description;
            for name in ["Execute","Appropriate","Implant Thought"] {assert!(text.contains(&format!("Grants {name}.")));}
            assert!(!text.contains("energy costs from amulets by 10"));
            let Rect(x,y,_,_)=pack.layout.detail;pack.scroll(-1000,x+2,y+2);
            assert_eq!(pack.detail_scroll,pack.detail_max_scroll());
            paint_capture(&r,&mut p,&pack,"long-ring-bottom");
        }
    }
    assert_eq!(serde_json::to_vec(&r).unwrap(),before);
}
#[test]
fn revision50_jewelry_capture_amulet_refresh_and_apply_cancel() {
    let mut r=run();let item=amulets::make_amulet(Effect::Lightning,901.).unwrap();
    assert!(item.equipped.is_missing());r.inventory.push(item);
    let mut p=Painter::new();let mut pack=selected(&r,&mut p,901.);
    assert!(pack.selected_group().unwrap().description.contains("Activation: 25 energy"));
    assert_eq!(pack.selected_group().unwrap().description.lines().nth(1),Some("Amulet"));
    assert!(pack.selected_group().unwrap().actions.iter().any(|b|b.label=="Apply"&&b.blocker.is_none()));
    assert!(!pack.selected_group().unwrap().actions.iter().any(|b|matches!(b.label.as_str(),"Wear"|"Take off")));
    paint_capture(&r,&mut p,&pack,"amulet-full");
    r.inventory[0].amulet_state.as_mut().unwrap().energy=25.;r.player.abilities.push("kative-appropriate".into());
    pack.refresh(&r);pack.configure(&p,p.frame().map.tuple());
    let text=&pack.selected_group().unwrap().description;
    assert!(text.contains("25/50")&&text.contains("Activation: 15 energy")&&!text.contains("No cooldown"));
    paint_capture(&r,&mut p,&pack,"amulet-spent-appropriate");
    let before=serde_json::to_vec(&r).unwrap();let base=crate::observe::observe(&r);
    p.game_at(&base,"",false,0.);let baseline=p.pixels.clone();
    for cycle in 0..3 {
        // Reopen after owner cancellation: no engine command is dispatched.
        let mut pack=selected(&r,&mut p,901.);
        for _ in 0..pack.keyboard_tabs_for_action_label("Apply").unwrap() {pack.key(Keycode::Tab,Mod::NOMOD);}
        let PackAction::Target(target)=pack.key(Keycode::Return,Mod::NOMOD) else {panic!("Apply must enter targeting")};
        assert_eq!(target,PackTarget::UseAt(901.));
        let mut preview=crate::observe::observe(&r);preview.target_path=target.application_preview(&r,Some((12,10)));
        assert!(!preview.target_path.is_empty());
        p.game_at(&preview,"",false,0.);if cycle==0 {capture(&p,"amulet-target-preview");}
        assert_eq!(pack.key(Keycode::Escape,Mod::NOMOD),PackAction::Close);
        p.game_at(&base,"",false,0.);assert_eq!(p.pixels,baseline,"cancel {cycle}");
        assert_eq!(serde_json::to_vec(&r).unwrap(),before);
    }
    capture(&p,"amulet-cancel-restored");
}
#[test]
fn revision50_jewelry_capture_status_meters_remaining_turns() {
    let mut r=run();let mut meters=crate::statuses_ui::ConditionMeters::default();
    meters.measure(r.player.id,&[]);
    for (key,turns) in [("r50Discord",6.),("r50Roots",4.),("r50AcidTurns",6.),("r50FireImmunity",20.)] {r.player.conditions.insert(key.into(),turns);}
    let mut first=crate::observe::observe(&r);first.measure_conditions(&mut meters);
    for name in ["Discord: hostile to all","Rooted","Corroded","Fire Immunity"] {
        let m=first.player_meters.iter().find(|m|m.name==name).unwrap();assert!(m.reference_known);assert_eq!(m.percent,100.);
    }
    let mut p=Painter::new();p.game_at(&first,"",false,0.);capture(&p,"statuses-full");
    first.player_status_offset=2;
    assert_eq!(first.player_meters[2].name,"Corroded");
    p.game_at(&first,"",false,0.);capture(&p,"statuses-full-page2");
    for (key,turns) in [("r50Discord",3.),("r50Roots",2.),("r50AcidTurns",3.),("r50FireImmunity",10.)] {r.player.conditions.insert(key.into(),turns);}
    let before=serde_json::to_vec(&r).unwrap();let mut half=crate::observe::observe(&r);half.measure_conditions(&mut meters);
    for name in ["Discord: hostile to all","Rooted","Corroded","Fire Immunity"] {assert_eq!(half.player_meters.iter().find(|m|m.name==name).unwrap().percent,50.);}
    p.game_at(&half,"",false,0.);capture(&p,"statuses-half");assert_eq!(serde_json::to_vec(&r).unwrap(),before);
    half.player_status_offset=2;p.game_at(&half,"",false,0.);capture(&p,"statuses-half-page2");
}

#[test]
fn revision50_jewelry_capture_actual_item_hovers() {
    let mut r=run();r.known_potions.clear();r.known_potion_natures=Field::Missing;r.ground.clear();
    let potion=Item{id:950.,kind:"potion".into(),name:"Unidentified potion".into(),glyph:"!".into(),
        effect:"freezing".to_string().into(),identified:false.into(),..Default::default()};
    r.inventory=vec![potion.clone()];
    r.ground_at_mut(r.depth).push(corpse_core::GroundItem{x:11.,y:10.,item:potion,..Default::default()});
    let mut p=Painter::new();let base=crate::observe::observe(&r);p.game_at(&base,"",false,0.);let baseline=p.pixels.clone();
    let report=crate::inspection_ui::observe_tile(&r,11,10);
    assert!(crate::inspection_ui::hover_available(&report));assert!(report.creature.is_none());
    let (cw,ch,ox,oy)=p.map_layout(base.width,base.height);
    let tile=(ox+11*cw,oy+10*ch,ox+12*cw,oy+11*ch);
    let before=serde_json::to_vec(&r).unwrap();
    crate::inspection_ui::paint_hover_at(&report,&mut p,tile);assert_ne!(p.pixels,baseline);
    capture(&p,"freezing-ground-hover");
    p.game_at(&base,"",false,0.);assert_eq!(p.pixels,baseline);
    assert_eq!(serde_json::to_vec(&r).unwrap(),before);
    for ring in [false,true] {
        if ring {
            r.inventory=vec![Item{id:951.,kind:"ring".into(),name:"Old ring label".into(),
                effect:"light".to_string().into(),granted_ability:"shrail-execute".to_string().into(),
                additional_granted_abilities:vec!["kative-appropriate".into(),"numo-implant-thought".into()],..Default::default()}];
        }
        let before=serde_json::to_vec(&r).unwrap();
        p.game_at(&crate::observe::observe(&r),"",false,0.);
        let mut pack=PackView::observe(&r);pack.configure(&p,p.frame().map.tuple());
        let y=pack.row_positions()[0].1-pack.list_scroll+4;pack.hover(pack.layout.list.0+12,y);
        assert!(pack.selected.is_none());let tip=pack.tooltip().unwrap();
        assert!(tip.contains(if ring {"Ring of Light"}else{"Potion of Freezing"}));
        paint_capture(&r,&mut p,&pack,if ring {"granted-ring-pack-hover"}else{"freezing-pack-hover"});
        assert_eq!(serde_json::to_vec(&r).unwrap(),before);
    }
}
