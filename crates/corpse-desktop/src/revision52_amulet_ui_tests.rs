//! Actual-instance projections and native CPU render oracles for R52 tiers.
//! These exercise keyboard/mouse handlers, not a platform SDL event loop.
//! Stale raw charges above ten deliberately test capped R52 projections;
//! Engine admission normalizes stored charges, while read-only UI does not mutate.
use super::*;
use corpse_core::revision52_amulet_tiers as tiers;
use corpse_core::revision50_amulet_effects as amulets;

const EFFECTS: [&str;12] = ["lightning","firebolt","poison","iceblast","acid","blinking",
    "beckoning","discord","obstruction","roots","teleportation","floods"];

fn room() -> Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=52.0.into();r.player.x=10.;r.player.y=10.;r.player.energy=100.;
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.conditions.insert("allSeeingReveal".into(),1.);
    r.player.conditions.insert("allSeeingDepth".into(),r.depth);
    r.inventory.clear();r.actors.clear();r.ground.clear();r.offers.clear();r.next_id=1000.;
    // Production save admission requires an actor and ground block for every
    // stored floor, even when the controlled scene has no occupants or loot.
    for floor in &r.floors {
        r.actors.insert(floor.depth.to_string(),std::sync::Arc::new(Vec::new()));
        r.ground.insert(floor.depth.to_string(),std::sync::Arc::new(Vec::new()));
    }
    let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.seen.fill(true);
    f.remembered_tiles=Field::Missing;f.environment=Field::Missing;f.light_sources=Field::Missing;
    f.doors=Field::Missing;f.portals=Field::Missing;f.non_euclidean=Field::Missing;
    f.speed_up=Field::Missing;f.godbeast_event=Field::Missing;
    r
}
fn item(effect:&str,id:f64,depth:u32,energy:f64) -> Item {
    let mut item=Item{id,kind:"amulet".into(),effect:effect.to_string().into(),glyph:"\"".into(),..Default::default()};
    item.name=amulets::display_name(&item).unwrap().into();
    corpse_core::revision50_amulets::initialize_item(&mut item).unwrap();
    tiers::initialize_birth(&mut item,52.,depth).unwrap();
    item.amulet_state.as_mut().unwrap().energy=energy;
    item
}
fn capture(p:&Painter,name:&str) {
    if let Ok(dir)=std::env::var("CORPSE_R52_AMULET_REVIEW") {
        std::fs::create_dir_all(&dir).unwrap();
        let file=std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
        let mut enc=png::Encoder::new(file,p.width as u32,p.height as u32);
        enc.set_color(png::ColorType::Rgba);enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(&p.pixels).unwrap();
    }
}
fn select_mouse(view:&mut PackView,id:f64) {
    let index=view.groups.iter().position(|g|g.item.id==id).unwrap();
    let yy=view.row_positions().into_iter().find(|(i,_)|*i==index).unwrap().1;
    view.list_scroll=(yy-view.layout.list.1).clamp(0,view.list_max_scroll());
    assert_eq!(view.click(view.layout.list.0+8,yy-view.list_scroll+4),PackAction::None);
    assert_eq!(view.selected,Some(id));
}

#[test]
fn r52_amulet_ui_all_effects_use_shared_profile_copy_and_unchanged_costs() {
    let mut r=room();let p=Painter::new();
    for depth in [1,20,40] {for effect in EFFECTS {
        let amulet=item(effect,900.,depth,17.);r.inventory=vec![amulet];
        let amulet=&r.inventory[0];let profile=tiers::profile(&r,amulet).unwrap();
        let title=format!("Tier {} {}",profile.tier,amulets::display_name(amulet).unwrap());
        let spec=amulets::item_spec(amulet).unwrap();
        let mut expected=vec![title.clone(),"Amulet".into(),tiers::description(&r,amulet).unwrap(),
            format!("Amulet energy: 10/10 · Activation: {} energy.",spec.activation_cost)];
        if let Some(details)=tiers::details(&r,amulet){expected.push(details);}
        let expected=expected.join("\n");let before=r.clone();
        assert_eq!(displayed_name(&r,amulet),title);
        assert_eq!(describe_item(&r,amulet),expected);
        for forbidden in ["Active while carried","active while carried","No cooldown","does not"] {
            assert!(!expected.contains(forbidden),"{effect}: {expected}");
        }
        let mut view=PackView::observe(&r);view.configure(&p,p.frame().map.tuple());
        view.hover(view.layout.list.0+8,view.row_positions()[0].1+4);
        assert_eq!(view.tooltip().as_deref(),Some(expected.as_str()));
        view.selected=Some(900.);
        assert_eq!(view.detail_lines().iter().filter(|(line,_)|line==&title).count(),1);
        assert_eq!(r,before);
        r.player.abilities.push("kative-appropriate".into());
        assert!(describe_item(&r,&r.inventory[0]).contains(&format!("Activation: {} energy",(spec.activation_cost-10.).max(0.))));
        r.player.abilities.clear();
    }}
}

#[test]
fn r52_amulet_ui_separate_instances_keep_manual_order_and_selected_identity() {
    let mut r=room();
    r.inventory=vec![item("lightning",900.,1,50.),item("lightning",901.,40,23.),
        item("lightning",902.,40,23.)];
    // Missing-state R52 items also represent separate physical reserves.
    let mut missing=item("lightning",903.,1,50.);missing.amulet_tier=Field::Missing;missing.amulet_state=Field::Missing;
    let mut other=missing.clone();other.id=904.;r.inventory.extend([missing,other]);
    r.inventory_display_order=vec![902.,900.,904.,901.,903.].into();
    let before=r.clone();let mut p=Painter::new();let mut view=PackView::observe(&r);
    view.configure(&p,p.frame().map.tuple());
    assert_eq!(view.groups.len(),5);assert!(view.groups.iter().all(|g|g.items.len()==1&&g.count==1.));
    assert_eq!(view.display_order(),vec![902.,900.,904.,901.,903.]);
    select_mouse(&mut view,901.);
    for (w,h) in [(1280,800),(960,1200)] {
        p.resize(w,h);p.set_responsive_canvas(true);view.configure(&p,p.frame().map.tuple());
        assert_eq!(view.selected_group().unwrap().item.id,901.);
        view.focus_action=None;
        for _ in 0..view.keyboard_tabs_for_action_label("Apply").unwrap(){view.key(Keycode::Tab,Mod::NOMOD);}
        assert_eq!(view.key(Keycode::Return,Mod::NOMOD),PackAction::Target(PackTarget::UseAt(901.)));
        assert_eq!(view.key(Keycode::Escape,Mod::NOMOD),PackAction::Close);
        view.cancel_selection();view.refresh(&r);view.configure(&p,p.frame().map.tuple());
        assert_eq!(view.selected_group().unwrap().item.id,901.);
        let index=view.selected_index().unwrap();
        let PackAction::Order(order)=view.move_group(index,-1) else {panic!("reorder within amulets")};
        assert_eq!(order.len(),5);assert_eq!(view.selected_group().unwrap().item.id,901.);
        view.refresh(&r);view.configure(&p,p.frame().map.tuple());
        assert_eq!(view.display_order(),vec![902.,900.,904.,901.,903.]);
    }
    assert_eq!(r,before);
    r.inventory[1].amulet_state.as_mut().unwrap().energy=3.;
    view.refresh(&r);assert_eq!(view.selected_group().unwrap().item.id,901.);
    assert!(view.selected_group().unwrap().description.contains("3/10"));
    assert_eq!(view.groups.iter().find(|g|g.item.id==902.).unwrap().item.amulet_state.as_ref().unwrap().energy,23.);
}

#[test]
fn r52_amulet_ui_save_load_names_and_missing_tier_are_not_inferred_from_floor() {
    let mut r=room();
    r.inventory=vec![item("blinking",900.,1,7.),item("blinking",901.,40,23.)];
    let mut old=item("lightning",902.,1,11.);old.amulet_tier=Field::Missing;r.inventory.push(old);
    let engine=corpse_core::temporal::Engine::new(r).unwrap();
    let saved=engine.save().unwrap();let loaded=corpse_core::temporal::Engine::load(&saved).unwrap();
    assert_eq!(loaded.run.inventory,engine.run.inventory);
    for (a,b) in loaded.run.inventory.iter().zip(&engine.run.inventory){
        assert_eq!(describe_item(&loaded.run,a),describe_item(&engine.run,b));
    }
    let mut deep=loaded.run.clone();deep.depth=40.;
    assert_eq!(displayed_name(&deep,&deep.inventory[0]),"Tier 1 Amulet of Blinking");
    assert_eq!(displayed_name(&deep,&deep.inventory[1]),"Tier 9 Amulet of Blinking");
    assert_eq!(displayed_name(&deep,&deep.inventory[2]),"Tier 1 Amulet of Lightning");
    assert_eq!(deep.inventory[2].amulet_tier,Field::Missing);
    assert!(crate::debug_floor_ui::item_row(&deep,&deep.inventory[0],Some(1)).contains("Tier 1 Amulet of Blinking · 7/10 energy · F1"));
    assert!(crate::debug_floor_ui::item_row(&deep,&deep.inventory[1],Some(40)).contains("Tier 9 Amulet of Blinking · 10/10 energy · F40"));
    for revision in [50.,51.] {
        let mut legacy=room();legacy.simulation_revision=revision.into();
        let mut amulet=item("blinking",900.,1,7.);amulet.amulet_tier=Field::Missing;legacy.inventory=vec![amulet];
        let engine=corpse_core::temporal::Engine::new(legacy).unwrap();
        let reload=corpse_core::temporal::Engine::load(&engine.save().unwrap()).unwrap();
        assert_eq!(displayed_name(&reload.run,&reload.run.inventory[0]),"Amulet of Blinking");
        assert!(describe_item(&reload.run,&reload.run.inventory[0]).contains("Blink along a 10 tile line."));
        assert!(!describe_item(&reload.run,&reload.run.inventory[0]).contains("Tier"));
        let mut view=PackView::observe(&reload.run);view.selected=Some(900.);
        assert_eq!(view.detail_lines().iter().filter(|(line,_)|line=="Amulet of Blinking").count(),1);
    }
}

#[test]
fn r52_amulet_ui_visible_ground_uses_actual_tier_and_charge() {
    let mut r=room();let amulet=item("discord",900.,40,12.);
    r.ground_at_mut(r.depth).push(corpse_core::GroundItem{x:11.,y:10.,item:amulet.clone(),..Default::default()});
    let before=r.clone();let report=crate::inspection_ui::observe_tile(&r,11,10);
    assert!(report.lines.join("\n").contains("Tier 9 Amulet of Discord"));
    assert!(report.item_hover.contains(&describe_item(&r,&amulet)));
    assert!(report.item_hover.join("\n").contains("10/10"));
    assert!(crate::observe::observe(&r).cells.iter().any(|c|c.x==11&&c.y==10&&c.label=="Tier 9 Amulet of Discord"));
    assert_eq!(r,before);
}

#[test]
fn r52_amulet_ui_memory_never_enriches_from_hidden_live_tier_or_charge() {
    let mut r=room();r.player.conditions.clear();let depth=r.depth;
    let f=r.floor_mut().unwrap();let (x,y)=(f.width()-2,f.height()-2);let at=y*f.width()+x;
    f.seen.fill(false);f.seen[at]=true;f.lights.fill(-100.);
    f.remembered_tiles=vec!["floor".into();f.tiles.len()].into();
    f.remembered_substrates=vec!["earth".into();f.tiles.len()].into();f.remembered_hazards=Default::default();
    assert!(!corpse_core::vision::observe_terrain(&r).unwrap().cells[at].current);
    r.remembered_items.get_or_insert_with(Default::default).insert(depth.to_string(),vec![
        corpse_core::RunRememberedItemsEntryEntry{x:x as f64,y:y as f64,item:item("poison",900.,1,17.)}]);
    let before=crate::inspection_ui::observe_tile(&r,x,y);
    assert!(before.item_hover.join("\n").contains("Tier 1 Amulet of Poison"));
    assert!(before.item_hover.join("\n").contains("10/10"));
    r.ground_at_mut(depth).push(corpse_core::GroundItem{x:x as f64,y:y as f64,item:item("poison",900.,40,2.),..Default::default()});
    assert_eq!(crate::inspection_ui::observe_tile(&r,x,y),before);
}

#[test]
fn r52_amulet_ui_tier_range_mouse_keyboard_and_known_collision_match() {
    let mut r=room();r.inventory=vec![item("blinking",900.,1,50.),item("blinking",901.,40,50.),
        item("iceblast",902.,40,50.),item("lightning",903.,40,50.),item("beckoning",904.,40,50.),item("teleportation",905.,40,50.)];
    let tier1=PackTarget::UseAt(900.);let tier9=PackTarget::UseAt(901.);
    assert!(tier1.target_blocker(&r,28.,10.).is_some());assert!(tier9.target_blocker(&r,28.,10.).is_none());
    assert_eq!(tier9.aim_preview(&r,Some((28,10))).impact,Some((28,10)));
    r.actors_at_mut(r.depth).push(corpse_core::Actor{id:950.,x:24.,y:10.,hp:100.,max_hp:100.,name:"Target".into(),glyph:"r".into(),state:"wandering".into(),..Default::default()});
    for (id,impact) in [(901.,(23,10)),(902.,(24,10))] {
        let before=r.clone();let target=PackTarget::UseAt(id);
        let mut mouse=crate::observe::observe(&r);crate::sync_pointer_view(&mut mouse,&r,Some((28,10)),vec![],Some(&target));
        let next=crate::aim_keyboard_tile(&r,Some((28,11)),Keycode::Up).unwrap();
        let mut keyboard=crate::observe::observe(&r);crate::sync_pointer_view(&mut keyboard,&r,Some(next),vec![],Some(&target));
        assert_eq!(mouse.target_impact,Some(impact));assert_eq!(keyboard.target_impact,mouse.target_impact);
        assert_eq!(mouse.target_path,keyboard.target_path);assert_eq!(mouse.target_area,keyboard.target_area);
        assert_eq!(r,before);
    }
    assert!(PackTarget::UseAt(903.).aim_preview(&r,Some((28,10))).delivery.contains(&(25,10)));
    for id in [904.,905.] {assert!(PackTarget::UseAt(id).target_blocker(&r,24.,10.).is_none());}
    for effect in ["beckoning","teleportation"] {
        r.inventory.push(item(effect,906.,1,50.));assert!(PackTarget::UseAt(906.).target_blocker(&r,24.,10.).is_some());r.inventory.pop();
    }
    // Hidden blockers cannot affect previews or UI admission.
    r.player.conditions.clear();r.actors_at_mut(r.depth)[0].x=14.;
    r.actors_at_mut(r.depth)[0].conditions.insert("invisible".into(),10.);
    assert!(!corpse_core::vision::visible_actor(&r,&r.actors_at(r.depth)[0]));
    let hidden=PackTarget::UseAt(902.).aim_preview(&r,Some((18,10)));
    let blocker=PackTarget::UseAt(902.).target_blocker(&r,18.,10.);
    assert!(!hidden.area.is_empty());assert!(blocker.is_none());
    r.actors.clear();assert_eq!(PackTarget::UseAt(902.).aim_preview(&r,Some((18,10))),hidden);
    assert_eq!(PackTarget::UseAt(902.).target_blocker(&r,18.,10.),blocker);
}

#[test]
fn r52_amulet_ui_selected_instance_matches_receipt_and_no_forward_is_free() {
    let mut r=room();r.inventory=vec![item("blinking",900.,1,50.),item("blinking",901.,40,30.)];
    let mut pack=PackView::observe(&r);pack.selected=Some(901.);
    let target=PackTarget::UseAt(901.);let expected=target.aim_preview(&r,Some((28,10)));
    assert_eq!(expected.impact,Some((28,10)));
    let result=corpse_core::engine::dispatch(&mut r,&target.command(28.,10.)).unwrap();
    assert!(result.took_turn);assert_eq!((r.player.x,r.player.y),(28.,10.));
    assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,50.);
    assert_eq!(r.inventory[1].amulet_state.as_ref().unwrap().energy,0.);
    assert!(result.events.iter().filter_map(|e|e.animation.as_ref()).any(|a|
        a.style=="reserve-drain"&&a.item_id==Some(901.)));
    assert!(result.events.iter().filter_map(|e|e.animation.as_ref()).any(|a|
        a.proposal=="A06"&&a.stage==corpse_core::events::Stage::Travel
        &&a.cells.last().is_some_and(|p|(p.x,p.y)==(28.,10.))));
    pack.refresh(&r);assert_eq!(pack.selected,Some(901.));
    assert!(pack.selected_group().unwrap().description.contains("0/10"));
    r.player.x=10.;r.player.y=10.;let width=r.floor().unwrap().width();
    r.floor_mut().unwrap().tiles[10*width+11]="wall".into();
    assert!(target.target_blocker(&r,28.,10.).is_some());let before=r.clone();
    let result=corpse_core::engine::dispatch(&mut r,&target.command(28.,10.));
    assert!(result.as_ref().map_or(true,|out|!out.took_turn));
    assert_eq!(r.inventory,before.inventory);assert_eq!(r.tick,before.tick);assert_eq!(r.player.energy,before.player.energy);
}

#[test]
fn r52_amulet_ui_pack_full_retained_tile_and_cancel_match_at_landscape_and_portrait() {
    let mut r=room();r.inventory=vec![item("lightning",900.,1,50.),item("lightning",901.,40,23.),item("blinking",902.,40,18.)];
    let before=r.clone();let base=crate::observe::observe(&r);
    let mut pack=PackView::observe(&r);let mut retained=crate::render::RetainedGame::default();
    let mut p=Painter::new();let mut full=Painter::new();let mut tile=Painter::new();
    let mut planner=crate::render::tile_damage::TilePlanner::default();
    for (w,h) in [(1280,800),(960,1200)] {
        for painter in [&mut p,&mut full,&mut tile] {painter.resize(w,h);painter.set_responsive_canvas(true);}
        pack.configure(&p,p.frame().map.tuple());pack.selected=None;select_mouse(&mut pack,901.);
        let mut old:Option<crate::render::View>=None;
        for (step,id) in [Some(901.),Some(902.),None].into_iter().enumerate() {
            let mut view=crate::observe::observe(&r);let target=id.map(PackTarget::UseAt);
            crate::sync_pointer_view(&mut view,&r,id.map(|_|(28,10)),vec![],target.as_ref());
            retained.update(&mut p,&view,"",0.,true,false).unwrap();full.game_at(&view,"",false,0.);
            assert!(p.pixels==full.pixels,"retained {w}x{h} step {step}");
            planner.replace_view(old.as_ref(),&view,&tile);planner.finish_damage(&mut tile);
            planner.paint_base(&mut tile,&view,"",0.);tile.tile_clip_enabled=false;planner.clear_frame();
            if tile.pixels!=full.pixels {
                capture(&tile,&format!("mismatch-actual-{step}-{w}x{h}"));
                capture(&full,&format!("mismatch-reference-{step}-{w}x{h}"));
                let changed:Vec<_>=tile.pixels.chunks_exact(4).zip(full.pixels.chunks_exact(4)).enumerate()
                    .filter_map(|(i,(a,b))|(a!=b).then_some((i%w,i/w))).collect();
                eprintln!("tile mismatch {w}x{h} step {step}: pixels={} bounds={:?}; first={:?}",changed.len(),
                    (changed.iter().map(|p|p.0).min(),changed.iter().map(|p|p.1).min(),changed.iter().map(|p|p.0).max(),changed.iter().map(|p|p.1).max()),changed.iter().take(16).collect::<Vec<_>>());
            }
            assert!(tile.pixels==full.pixels,"tile {w}x{h} step {step}");
            capture(&full,&format!("aim-{step}-{w}x{h}"));old=Some(view);
        }
        p.game_at(&base,"",false,0.);let clean=p.pixels.clone();
        pack.paint(&mut p);capture(&p,&format!("pack-tier9-{w}x{h}"));
        assert_eq!(pack.selected_group().unwrap().item.id,901.);
        let index=pack.selected_group().unwrap().actions.iter().position(|b|b.label=="Apply").unwrap();
        let rect=pack.button_rects().into_iter().find(|(i,_)|*i==index).unwrap().1;
        assert_eq!(pack.click(rect.0+4,rect.1+4),PackAction::Target(PackTarget::UseAt(901.)));
        assert_eq!(pack.key(Keycode::Escape,Mod::NOMOD),PackAction::Close);
        pack.cancel_selection();pack.refresh(&r);pack.configure(&p,p.frame().map.tuple());
        p.game_at(&base,"",false,0.);assert!(p.pixels==clean);assert_eq!(pack.selected,Some(901.));
    }
    assert_eq!(r,before);
}
