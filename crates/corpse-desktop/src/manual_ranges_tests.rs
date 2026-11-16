use super::page_text;
use super::super::{Manual, Painter, resolve_page};
use corpse_core::{Field, Run, engine};

fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 52.0.into();
    r.actors.clear(); r.inventory.clear(); r.player.abilities.clear(); r.player.active.clear();
    r.player.conditions.clear(); r.player.hp = 1000.; r.player.max_hp = 1000.;
    r.player.stats.dr = 0.; r.player.energy = 100.; r.player.x = 13.; r.player.y = 10.;
    r.tick = 0.; r.status = "playing".into(); r.next_id = 1000.;
    let f = r.floor_mut().unwrap();
    f.tiles.fill("floor".into()); f.lights.fill(0.); f.environment = Field::Missing;
    f.light_sources = Field::Missing; f.portals = Field::Missing;
    f.non_euclidean = Field::Missing; f.speed_up = Field::Missing;
    f.roots = Field::Missing; f.fungal_roots = Field::Missing;
    f.monster_entry_tick = 0.0.into();
    r
}
fn add(r: &mut Run, id: f64, x: f64, y: f64, power: &str) {
    let mut a = r.player.clone(); a.id = id; a.x = x; a.y = y;
    a.name = "Range witness".into(); a.state = "hunting".into();
    a.abilities = if power.is_empty() {vec![]} else {vec![power.into()]};
    r.actors_at_mut(r.depth).push(a);
}
fn wall(r: &mut Run, x: usize, y: usize) {
    let f = r.floor_mut().unwrap(); let at = y * f.width() + x; f.tiles[at] = "wall".into();
}
fn dream(r: &mut Run, id: f64) {
    engine::actor_mut(r,id).unwrap().conditions.insert("phantasmiiPlane".into(),1.);
}
fn condition(r: &Run, id: f64, key: &str) -> f64 { engine::condition(engine::actor(r,id).unwrap(),key) }

#[test]
fn bestiary_effect_ranges_inventory_covers_every_nearby_catalog_entry() {
    let manual = Manual::new(); let mut changed = 0; let mut affected = 0;
    for entry in &manual.catalog.entries {
        let mut changes = 0;
        for m in &entry.mechanics {
            let text = page_text(entry,m);
            if m.text.to_ascii_lowercase().contains("nearby") {
                assert_ne!(text,m.text,"{} / {}",entry.id,m.name);
            }
            if text != m.text { changes += 1; assert!(!text.to_ascii_lowercase().contains("nearby")); }
        }
        changed += changes; affected += usize::from(changes > 0);
    }
    assert_eq!((changed,affected),(40,20));
}

#[test]
fn bestiary_effect_ranges_full_pages_fit_preserving_lore_art_and_preview() {
    let mut manual = Manual::new(); manual.preview_all(); let mut p = Painter::new();
    let r = room(); let before = serde_json::to_value(&r).unwrap();
    let mut pages = 0;
    for id in manual.ids().to_vec() {
        manual.select_id(&id); manual.prepare_art();
        let entry = manual.entry().unwrap();
        if !entry.mechanics.iter().any(|m|page_text(entry,m)!=m.text) {continue;}
        pages += 1;
        let original = entry.mechanics.iter().map(|m|(m.name.clone(),m.text.clone())).collect::<Vec<_>>();
        for (width,height) in [(1280,800),(1920,1080),(900,1200)] {
        p.try_resize(width,height).unwrap();p.set_responsive_canvas(false);
        let page = resolve_page(&p,entry,manual.art.as_ref().map(|(_,art)|art));
        assert!(page.content.height <= 610,"{id}: {} at {}",page.content.height,page.content.size);
        assert!(page.content.size >= 18,"{id}: text shrank to {}",page.content.size);
        assert!(page.extras.iter().all(|(_,_,extra)|extra.height<=140 && extra.size>=18));
        let body = page.content.lines.iter().chain(page.extras.iter().flat_map(|(_,_,e)|e.lines.iter()))
            .filter(|l|!l.heading).map(|l|l.text.as_str()).collect::<Vec<_>>().join(" ");
        for m in &entry.mechanics { assert!(body.contains(page_text(entry,m)),"{id}: {}",m.name); }
        assert!(body.contains(&entry.description));
        assert!(body.contains(&super::super::natural_history(entry).split_whitespace().collect::<Vec<_>>().join(" ")));
        }
        assert_eq!(original,entry.mechanics.iter().map(|m|(m.name.clone(),m.text.clone())).collect::<Vec<_>>());
        if let Some(actor) = super::super::birth_actor(entry,1,52.) {
            let brief = crate::creature_abilities_ui::describe(&r,&actor);
            assert!(!brief.0.contains("spatial shortcuts") && !brief.1.contains("spatial shortcuts"));
            // Exercise the actual full-page paint, then compare both preview strings byte for byte.
            manual.paint(&mut p);
            assert_eq!(crate::creature_abilities_ui::describe(&r,&actor),brief);
        }
    }
    assert_eq!(pages,20); assert_eq!(serde_json::to_value(&r).unwrap(),before);
}

#[test]
fn bestiary_effect_ranges_silence_is_inclusive_circular_live_sight_and_plane_local() {
    for (dx,dy,blocked,other_plane,expected) in [(3.,0.,false,false,true),(3.,1.,false,false,false),(2.,2.,false,false,true),(3.,0.,true,false,false),(3.,0.,false,true,false)] {
        let mut r=room(); r.player.x=10.+dx; r.player.y=10.+dy;
        add(&mut r,10.,10.,10.,"enemy:beast:silence-aura");
        if blocked {wall(&mut r,11,10);} if other_plane {dream(&mut r,0.);}
        assert_eq!(corpse_core::abilities::energy_activation_silenced(&r,&r.player),expected);
        corpse_core::monster_horn::world_turn(&mut r);
        assert_eq!(condition(&r,0.,"auraSilenced")>0.,expected);
        engine::actor_mut(&mut r,10.).unwrap().hp=0.;
        assert!(!corpse_core::abilities::energy_activation_silenced(&r,&r.player));
    }
}

#[test]
fn bestiary_effect_ranges_portal_shortcuts_count_only_for_spatial_ranges() {
    let mut r=room(); r.player.x=40.;
    add(&mut r,10.,10.,10.,"enemy:beast:silence-aura");
    let f=r.floor_mut().unwrap(); let w=f.width();
    f.portals=vec![corpse_core::portals_generation::PortalPair {a:10*w+11,b:10*w+39,color_index:0}].into();
    assert_eq!(corpse_core::portals::actor_distance(&r,engine::actor(&r,10.).unwrap(),&r.player),2.);
    assert!(corpse_core::abilities::energy_activation_silenced(&r,&r.player));
    engine::actor_mut(&mut r,10.).unwrap().abilities=vec!["enemy:beast:darkness-aura".into()];
    assert_eq!(corpse_core::vision::tile_light(&r,40,10),0.);
    engine::actor_mut(&mut r,10.).unwrap().abilities=vec!["enemy:beast:detonate".into()];
    assert!(!corpse_core::monster_bombardier::special_action(&mut r,10.,0.).unwrap());
}

#[test]
fn bestiary_effect_ranges_ruin_hits_all_sides_only_inside_three_with_sight() {
    for (dx,dy,blocked,other_plane,expected) in [(3.,0.,false,false,true),(3.,1.,false,false,false),(3.,0.,true,false,false),(3.,0.,false,true,false)] {
        let mut r=room(); r.player.x=11.;
        add(&mut r,10.,10.,10.,"enemy:beast:ruin-pulse");
        add(&mut r,20.,10.+dx,10.+dy,"");
        if blocked {wall(&mut r,12,10);} if other_plane {dream(&mut r,20.);}
        assert!(corpse_core::monster_ruin::special_action(&mut r,10.,0.).unwrap());
        assert_eq!(engine::actor(&r,20.).unwrap().hp<1000.,expected);
    }
}

#[test]
fn bestiary_effect_ranges_darkness_uses_strict_ten_orthogonal_steps_and_walls() {
    for (dx,dy,expected) in [(9,0,-10.),(10,0,0.),(5,4,-10.),(5,5,0.),(2,2,-60.)] {
        let mut r=room(); add(&mut r,10.,10.,10.,"enemy:beast:darkness-aura");
        assert_eq!(corpse_core::vision::tile_light(&r,10+dx,10+dy),expected);
    }
    let mut r=room(); add(&mut r,10.,10.,10.,"enemy:beast:darkness-aura");
    wall(&mut r,11,10); assert_eq!(corpse_core::vision::tile_light(&r,13,10),0.);
    let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[10*w+11]="floor".into();
    dream(&mut r,10.); assert_eq!(corpse_core::vision::tile_light(&r,13,10),0.);
}

#[test]
fn bestiary_effect_ranges_dryad_requires_injured_ally_six_and_sight() {
    for (dx,dy,blocked,other_plane,expected) in [(6.,0.,false,false,true),(6.,1.,false,false,false),(6.,0.,true,false,false),(6.,0.,false,true,false)] {
        let mut r=room(); r.player.x=10.+dx;r.player.y=10.+dy;r.player.hp=500.;
        add(&mut r,10.,10.,10.,"enemy:beast:dryad");
        let a=engine::actor_mut(&mut r,10.).unwrap();a.state="ally".into();a.hp=1000.;
        if blocked {wall(&mut r,11,10);}if other_plane {dream(&mut r,0.);}
        assert_eq!(corpse_core::monster_spells::heal_others(&mut r,10.).unwrap(),expected);
        assert_eq!(r.player.hp>500.,expected);
        if expected {assert!(!corpse_core::monster_spells::heal_others(&mut r,10.).unwrap());r.tick=2.;assert!(corpse_core::monster_spells::heal_others(&mut r,10.).unwrap());}
    }
}

#[test]
fn bestiary_effect_ranges_scream_crosses_walls_and_calls_beyond_ten() {
    let mut r=room();r.player.x=11.;add(&mut r,10.,10.,10.,"enemy:pale:scream");
    engine::actor_mut(&mut r,10.).unwrap().name="Pale Screamer".into();
    for (id,x,y) in [(20.,20.,10.),(21.,20.,11.),(22.,14.,10.),(23.,14.,11.),(30.,30.,10.)] {add(&mut r,id,x,y,"");}
    for id in [20.,21.,22.,23.] {engine::actor_mut(&mut r,id).unwrap().state="ally".into();}
    dream(&mut r,23.);wall(&mut r,12,10);
    engine::actor_mut(&mut r,30.).unwrap().state="sleeping".into();
    assert!(corpse_core::ai::enemy_action_options(&mut r,10.,0.,false).unwrap());
    for (id,expected) in [(20.,true),(21.,false),(22.,true),(23.,false)] {assert_eq!(condition(&r,id,"disoriented")>0.,expected,"{id}");}
    assert_eq!(condition(&r,20.,"disoriented"),5.);
    assert_eq!(condition(&r,30.,"screamEmboldened"),0.);
    assert_eq!(engine::actor(&r,30.).unwrap().state,"hunting");
    assert!(engine::actor(&r,30.).unwrap().last_seen.as_ref().is_some());
}

#[test]
fn bestiary_effect_ranges_blast_is_direct_five_with_sight() {
    for (dx,dy,blocked,other_plane,expected) in [(5.,0.,false,false,true),(5.,1.,false,false,false),(5.,0.,true,false,false),(5.,0.,false,true,false)] {
        let mut r=room();r.player.x=11.;add(&mut r,10.,10.,10.,"enemy:beast:detonate");
        add(&mut r,20.,10.+dx,10.+dy,"");
        if blocked {wall(&mut r,12,10);}if other_plane {dream(&mut r,20.);}
        assert!(corpse_core::monster_bombardier::special_action(&mut r,10.,0.).unwrap());
        assert_eq!(engine::actor(&r,20.).unwrap().hp<1000.,expected);
        assert_eq!(engine::actor(&r,10.).unwrap().energy,0.);
    }
}

#[test]
fn bestiary_effect_ranges_stone_seal_target_five_and_placement_ring() {
    for (dx,dy,blocked,other_plane,expected) in [(5.,0.,false,false,true),(5.,1.,false,false,false),(5.,0.,true,false,false),(5.,0.,false,true,false)] {
        let mut r=room();r.player.x=10.+dx;r.player.y=10.+dy;
        add(&mut r,10.,10.,10.,"enemy:beast:wall-seal");
        if blocked {wall(&mut r,11,10);}if other_plane {dream(&mut r,0.);}
        assert_eq!(corpse_core::ai::enemy_action_options(&mut r,10.,0.,false).unwrap(),expected);
        if expected {
            let at=condition(&r,10.,"raisedWallIndex") as usize;let w=r.floor().unwrap().width();
            assert_eq!(((at%w) as f64-r.player.x).abs().max(((at/w) as f64-r.player.y).abs()),1.);
            assert_eq!(condition(&r,10.,"raisedWallExpires"),5.);
        }
    }
}

#[test]
fn bestiary_effect_ranges_fungal_roots_use_direct_ten_sight_and_three_turn_growth() {
    for (dx,dy,blocked,frozen,expected) in [(10.,0.,false,false,true),(10.,1.,false,false,false),(10.,0.,true,false,false),(10.,0.,false,true,false)] {
        let mut r=room();add(&mut r,10.,10.,10.,"enemy:beast:fungal-gas");add(&mut r,20.,10.+dx,10.+dy,"");
        let a=engine::actor_mut(&mut r,20.).unwrap();a.dead=true.into();a.hp=0.;
        if blocked {wall(&mut r,11,10);}
        let f=r.floor().unwrap();let mut mask=vec![false;f.tiles.len()];if frozen {mask[10*f.width()+11]=true;}
        corpse_core::fungi::tick_fungi(&mut r,&mask).unwrap();
        let links=r.floor().unwrap().fungal_roots.as_ref().unwrap();
        assert_eq!(!links.is_empty(),expected);
        if expected {
            assert_eq!(links[0].grown_path().len(),1);assert!(!links[0].reaches_corpse());
            for _ in 0..2 {corpse_core::fungi::tick_fungi(&mut r,&mask).unwrap();}
            assert_eq!(r.floor().unwrap().fungal_roots.as_ref().unwrap()[0].grown_path().len(),1);
            corpse_core::fungi::tick_fungi(&mut r,&mask).unwrap();
            assert_eq!(r.floor().unwrap().fungal_roots.as_ref().unwrap()[0].grown_path().len(),2);
        }
    }
}

#[test]
fn bestiary_effect_ranges_razorleaves_are_direct_five_not_a_square() {
    let mut r=room();add(&mut r,10.,10.,10.,"enemy:beast:razorleaves");
    let w=r.floor().unwrap().width();
    corpse_core::environment::refresh_razorleaves(&mut r,&[10.]);
    let leaves=r.floor().unwrap().razorleaves.as_ref().unwrap();
    assert_eq!(leaves[10*w+15],1.);assert_eq!(leaves[11*w+15],0.);assert_eq!(leaves[13*w+14],1.);
    wall(&mut r,11,10);corpse_core::environment::refresh_razorleaves(&mut r,&[10.]);
    assert_eq!(r.floor().unwrap().razorleaves.as_ref().unwrap()[10*w+15],0.);
}

#[test]
fn bestiary_effect_ranges_yeti_includes_diagonal_without_sight() {
    for (dx,dy,blocked,other_plane,expected) in [(1.,1.,false,false,true),(2.,0.,false,false,false),(1.,1.,true,false,true),(1.,1.,false,true,false)] {
        let mut r=room();r.player.x=10.+dx;r.player.y=10.+dy;add(&mut r,10.,10.,10.,"enemy:beast:yeti");
        if blocked {wall(&mut r,11,10);wall(&mut r,10,11);}if other_plane {dream(&mut r,0.);}
        corpse_core::monster_chill::apply_adjacency(&mut r);
        assert_eq!(condition(&r,0.,"slowed")>0.,expected);
    }
}

#[test]
fn bestiary_effect_ranges_banshee_has_no_radius_but_requires_thirty_turns_sight_and_plane() {
    for (blocked,other_plane,expected) in [(false,false,true),(true,false,false),(false,true,false)] {
        let mut r=room();r.player.x=40.;add(&mut r,10.,10.,10.,"enemy:beast:banshee");
        engine::actor_mut(&mut r,10.).unwrap().creature_kind="banshee".to_owned().into();
        if blocked {wall(&mut r,11,10);}if other_plane {dream(&mut r,0.);}
        r.tick=29.;corpse_core::lifecycle::world_turn(&mut r).unwrap();assert_eq!(r.player.hp,1000.);
        r.tick=30.;corpse_core::lifecycle::world_turn(&mut r).unwrap();assert_eq!(r.player.hp<1000.,expected);
    }
}

#[test]
fn bestiary_effect_ranges_native_page_capture_matrix() {
    let Ok(directory)=std::env::var("CORPSE_BESTIARY_RANGE_REVIEW") else{return;};
    std::fs::create_dir_all(&directory).unwrap();
    let mut m=Manual::new();m.preview_all();let mut p=Painter::new();
    let ids:Vec<_>=m.catalog.entries.iter().filter(|e|e.mechanics.iter().any(|v|page_text(e,v)!=v.text)).map(|e|e.id.clone()).collect();
    assert_eq!(ids.len(),20);
    for id in ids {
        assert!(m.select_id(&id));
        for (aspect,width,height) in [("native",1280,800),("wide",1920,1080),("portrait",900,1200)] {
            p.try_resize(width,height).unwrap();p.set_responsive_canvas(false);m.paint(&mut p);
            let file=std::fs::File::create(format!("{directory}/bestiary-ranges-{aspect}-{}.png",id.replace(':',"-"))).unwrap();
            let mut encoder=png::Encoder::new(file,p.width as u32,p.height as u32);
            encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
        }
    }
}
