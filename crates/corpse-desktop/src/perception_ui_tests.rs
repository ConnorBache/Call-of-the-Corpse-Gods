use super::*;
use crate::{observe,render,appearance_profile};
fn room()->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=51.0.into();r.player.x=20.;r.player.y=14.;r.player.stats.perception=0.;
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.actors.clear();r.inventory.clear();r.ground.clear();r.offers.clear();
    let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(0.);f.seen.fill(false);
    f.remembered_tiles=corpse_core::Field::Missing;f.portals=corpse_core::Field::Missing;f.doors=corpse_core::Field::Missing;
    f.environment=corpse_core::Field::Missing;f.light_sources=corpse_core::Field::Missing;f.non_euclidean=corpse_core::Field::Missing;f.speed_up=corpse_core::Field::Missing;
    let mut enemy=corpse_core::population::create_actor_revision("rat",900.,23.,14.,false,51.).unwrap();
    enemy.stats.perception=0.;enemy.conditions.clear();enemy.abilities.clear();enemy.state="wandering".into();
    r.actors_at_mut(r.depth).push(enemy);r
}
fn boundary(v:&View)->&Boundary {v.perception_boundaries.iter().find(|b|b.actor==900f64.to_bits()).unwrap()}
fn on(v:&View,x:usize,y:usize,vertical:bool)->bool {boundary(v).edges.contains(&Edge{x,y,vertical})}
#[test]fn revision51_perception_labels_legacy_gate_and_heart_preview(){
    let mut r=room();let v=observe::observe(&r);assert_eq!(v.creatures[0].perception_range.as_deref(),Some("Perception range: 20"));
    let c=crate::inspection_ui::observe_tile(&r,23,14).creature.unwrap();
    assert!(c.behavior.contains("Perception range: 20"));assert!(!c.behavior.contains("Illumination"));
    let w=r.floor().unwrap().width();r.floor_mut().unwrap().lights[14*w+20]=-15.;
    let report=crate::inspection_ui::observe_tile(&r,20,14);assert!(report.lines.iter().any(|s|s=="−3 perception range"));
    assert!(!report.lines.iter().any(|s|s.contains("Illumination")));
    r.floor_mut().unwrap().lights[14*w+20]=0.;assert!(!crate::inspection_ui::observe_tile(&r,20,14).lines.iter().any(|s|s.contains("perception range")));
    r.simulation_revision=50.0.into();let v=observe::observe(&r);assert!(v.perception_boundaries.is_empty());assert!(v.creatures[0].perception_range.is_none());
    assert!(crate::inspection_ui::observe_tile(&r,23,14).creature.unwrap().behavior.contains("Illumination"));
    let bat=corpse_core::population::create_actor_revision("cave-bat",901.,23.,14.,false,51.).unwrap();
    r.simulation_revision=51.0.into();r.actors_at_mut(r.depth)[0]=bat;
    assert!(crate::inspection_ui::observe_tile(&r,23,14).creature.unwrap().heart.contains("+1 Perception"));
}
#[test]fn revision51_perception_range_is_hover_only_with_stable_card_geometry(){
    let r=room();let v=observe::observe(&r);let mut other=observe::observe(&r);
    other.creatures[0].perception_range=Some("Perception range: 99".into());
    let mut p=Painter::new();let mut changed=Painter::new();
    p.game_at(&v,"",false,0.);changed.game_at(&other,"",false,0.);
    assert!(p.pixels==changed.pixels,"unhovered range has no ink");
    let(_,_,ox,_)=p.map_layout(v.width,v.height);let layout=render::enemy_list::Layout::resolve_frame(&p.profile,ox,p.frame());
    let point=(layout.rows.0+10,layout.rows.1+10);let before=layout.content(&p,&v.creatures).hit(point.0,point.1,0);
    p.enemy_hover=Some(900f64.to_bits());changed.enemy_hover=p.enemy_hover;
    p.game_at(&v,"",false,0.);changed.game_at(&other,"",false,0.);
    assert!(p.pixels!=changed.pixels,"hover reveals the correct range");
    assert_eq!(layout.content(&p,&v.creatures).hit(point.0,point.1,0),before);
}
#[test]fn revision51_perception_contour_uses_creature_distance_stealth_and_light(){
    let mut r=room();let before=serde_json::to_vec(&r).unwrap();let v=observe::observe(&r);
    assert!(on(&v,34,14,true),"distance 10 seen, 11 not; never tile-radius 20");
    assert_eq!(serde_json::to_vec(&r).unwrap(),before);
    r.player.conditions.insert("stealth".into(),0.5);assert!(on(&observe::observe(&r),33,14,true),"fractional Stealth is not rounded away");
    r.player.conditions.insert("stealth".into(),5.);let v=observe::observe(&r);assert!(on(&v,29,14,true));assert!(!on(&v,34,14,true));
    r.player.conditions.clear();r.player.conditions.insert("submergedStealth".into(),5.);assert!(on(&observe::observe(&r),29,14,true));
    r.player.conditions.clear();r.floor_mut().unwrap().lights.fill(-15.);let v=observe::observe(&r);assert!(on(&v,32,14,true));
    assert!(crate::inspection_ui::observe_tile(&r,23,14).creature.unwrap().behavior.contains("−3 perception range"));
    r.player.stats.perception=100.;r.actors_at_mut(r.depth)[0].stats.perception=100.;r.floor_mut().unwrap().lights.fill(-100.);
    let v=observe::observe(&r);assert!(on(&v,34,14,true),"literal -100 light formula with positive Perception");
    assert!(crate::inspection_ui::observe_tile(&r,20,14).lines.iter().any(|s|s=="−20 perception range"));
    r.floor_mut().unwrap().lights.fill(1.);let v=observe::observe(&r);assert!(boundary(&v).edges.is_empty(),"positive light bypasses distance, no fake radius");
    r.player.conditions.insert("invisible".into(),1.);assert!(boundary(&observe::observe(&r)).edges.is_empty());
}
#[test]fn revision51_perception_known_wall_blocks_but_unknown_ray_never_closes(){
    let r=room();let v=observe::observe(&r);let mut o=(**v.terrain_snapshot.as_ref().unwrap()).clone();let w=o.width;
    o.cells[14*w+26].tile=Some("wall".into());
    let b=super::observe(&r,Some(&o),&v.creatures);assert!(b[0].edges.contains(&Edge{x:27,y:14,vertical:true}));
    // Neither the actual hidden wall nor an invented wall closes a safe contour.
    o.cells[14*w+26].current=false;o.cells[14*w+26].direct=false;o.cells[14*w+26].tile=None;
    let b=super::observe(&r,Some(&o),&v.creatures);assert!(!b[0].edges.iter().any(|e|e.vertical&&e.y==14&&e.x>=26&&e.x<=34));
}
#[test]fn revision51_perception_hidden_wall_and_light_do_not_change_detached_preview(){
    let mut r=room();let w=r.floor().unwrap().width();let h=r.floor().unwrap().height();
    for y in 0..h {r.floor_mut().unwrap().tiles[y*w+45]="wall".into();}
    let v=observe::observe(&r);let o=v.terrain_snapshot.as_deref().unwrap();
    let before=super::observe(&r,Some(o),&v.creatures);
    r.floor_mut().unwrap().tiles[14*w+60]="wall".into();r.floor_mut().unwrap().lights[14*w+60]=100.;
    r.floor_mut().unwrap().light_sources=vec![corpse_core::model::FloorLightSourcesEntry{x:60.,y:14.,power:100.,falloff:10.,kind:"torch".to_string().into(),..Default::default()}].into();
    assert_eq!(super::observe(&r,Some(o),&v.creatures),before);
    assert_eq!(observe::observe(&r).perception_boundaries,before,"fresh hidden alcove mutations do not affect observed light/rays");
    // Even a mutated live wall between known endpoints cannot influence a frozen observation.
    r.floor_mut().unwrap().tiles[14*w+26]="wall".into();
    assert_eq!(super::observe(&r,Some(o),&v.creatures),before);
}
#[test]fn revision51_perception_portals_only_use_known_links(){
    let mut r=room();let v=observe::observe(&r);let mut o=(**v.terrain_snapshot.as_ref().unwrap()).clone();let w=o.width;
    r.floor_mut().unwrap().portals=vec![corpse_core::portals_generation::PortalPair{a:14*w+25,b:14*w+37,color_index:0}].into();
    let b=super::observe(&r,Some(&o),&v.creatures);assert!(!b[0].edges.contains(&Edge{x:34,y:14,vertical:true}),"known portal shortens optical route");
    o.cells[14*w+37].current=false;o.cells[14*w+37].direct=false;assert!(super::observe(&r,Some(&o),&v.creatures).is_empty(),"unknown mate has no promised safe region");
}
#[test]fn revision51_perception_special_senses_and_moving_light_are_omitted(){
    let mut r=room();let v=observe::observe(&r);let o=v.terrain_snapshot.as_deref();
    let original=r.actors_at(r.depth)[0].clone();
    for kind in ["telepathic","allSeeing","came","numo-mind-read","ursa-dream-see","witch"] {
        r.actors_at_mut(r.depth)[0]=original.clone();r.player.abilities.clear();
        let depth=r.depth;let player=r.player.id;let a=&mut r.actors_at_mut(depth)[0];
        match kind {
            "telepathic"=>{a.conditions.insert("telepathic".into(),10.);},
            "allSeeing"=>{a.conditions.insert("allSeeingReveal".into(),1.);a.conditions.insert("allSeeingDepth".into(),depth);},
            "came"=>{a.conditions.insert("cameTurns".into(),10.);a.conditions.insert("cameTarget".into(),player);},
            "witch"=>{a.abilities.push(corpse_core::revision50_combat::WITCH_HATRED.into());r.player.abilities.push(corpse_core::revision50_combat::MAGICKAL_CASTER.into());},
            ability=>a.abilities.push(ability.into()),
        }
        assert!(super::observe(&r,o,&v.creatures).is_empty(),"{kind} cannot imply an optical safe zone");
    }
    r.actors_at_mut(r.depth)[0]=original;r.player.abilities=vec!["anavani-shine".into()];
    assert!(super::observe(&r,o,&v.creatures).is_empty());
    r.player.active.push("anavani-shine".into());assert!(!super::observe(&r,o,&v.creatures).is_empty(),"suppressed R51 Shine no longer moves an emitter");
    r.player.abilities.clear();r.player.active.clear();r.inventory.push(corpse_core::Item{id:990.,kind:"ring".into(),name:"Ring of Light".into(),effect:"light".to_string().into(),equipped:"ring1".to_string().into(),..Default::default()});
    assert!(super::observe(&r,o,&v.creatures).is_empty());
}
fn paint_pair(p:&mut Painter,full:&mut Painter,v:&View,retained:&mut render::RetainedGame,changed:bool,now:f64) {
    retained.update(p,v,"",now,changed,false).unwrap();full.enemy_hover=p.enemy_hover;full.enemy_list_offset=p.enemy_list_offset;
    full.game_at(v,"",false,now);assert!(p.pixels==full.pixels,"retained pixels differ at {:?}",p.pixels.iter().zip(&full.pixels).position(|(a,b)|a!=b));
}
#[test]fn revision51_perception_two_way_hover_cancel_scroll_resize_and_repaint(){
    let r=room();let mut v=observe::observe(&r);let mut p=Painter::new();let mut full=Painter::new();let mut retained=render::RetainedGame::default();
    for(w,h)in[(1280,800),(1920,1080),(960,1200),(1537,911)] {
        for p in [&mut p,&mut full]{p.resize(w,h);p.set_responsive_canvas(true);}
        let(cw,ch,ox,oy)=p.map_layout(v.width,v.height);
        crate::refresh_panel_hover(&mut p,&v,Some((ox+23*cw+cw/2,oy+14*ch+ch/2)),true);
        assert_eq!(p.enemy_hover,Some(900f64.to_bits()));paint_pair(&mut p,&mut full,&v,&mut retained,true,0.);
        let layout=render::enemy_list::Layout::resolve_frame(&p.profile,ox,p.frame());
        crate::refresh_panel_hover(&mut p,&v,Some((layout.rows.0+10,layout.rows.1+10)),true);assert_eq!(p.enemy_hover,Some(900f64.to_bits()));
        paint_pair(&mut p,&mut full,&v,&mut retained,false,0.);
        crate::refresh_panel_hover(&mut p,&v,None,false);assert_eq!(p.enemy_hover,None);paint_pair(&mut p,&mut full,&v,&mut retained,false,0.);
        p.enemy_hover=Some(900f64.to_bits());paint_pair(&mut p,&mut full,&v,&mut retained,false,420.);
        p.enemy_list_offset=50;paint_pair(&mut p,&mut full,&v,&mut retained,false,820.);
    }
    let old=observe::observe(&r);v.perception_boundaries[0].edges.pop();
    paint_pair(&mut p,&mut full,&v,&mut retained,true,820.);
    let mut gone=observe::observe(&r);gone.creatures.clear();gone.cells.retain(|c|c.actor_id!=Some(900.));gone.perception_boundaries.clear();
    p.enemy_hover=None;paint_pair(&mut p,&mut full,&gone,&mut retained,true,820.);
    // Tile path hover invalidation shares the same stored contour bounds.
    let mut tile=Painter::new();let mut full=Painter::new();let mut plan=render::tile_damage::TilePlanner::default();let mut keys=None;
    plan.replace_view(None,&old,&tile);
    for hover in [None,Some(900f64.to_bits()),None,Some(900f64.to_bits())] {
        tile.enemy_hover=hover;full.enemy_hover=hover;
        let next=crate::tile_overlay::PanelKeys::read(&tile);next.mark_changes(keys,&tile,&old,&mut plan);keys=Some(next);
        plan.finish_damage(&mut tile);plan.paint_base(&mut tile,&old,"",0.);tile.tile_clip_enabled=false;plan.clear_frame();
        full.game_at(&old,"",false,0.);assert!(tile.pixels==full.pixels,"tile pixels differ at {:?}",tile.pixels.iter().zip(&full.pixels).position(|(a,b)|a!=b));
    }
    plan.replace_view(Some(&old),&v,&tile);plan.finish_damage(&mut tile);plan.paint_base(&mut tile,&v,"",0.);tile.tile_clip_enabled=false;
    full.game_at(&v,"",false,0.);assert!(tile.pixels==full.pixels,"tile pixels differ at {:?}",tile.pixels.iter().zip(&full.pixels).position(|(a,b)|a!=b));
    plan.clear_frame();tile.enemy_hover=None;full.enemy_hover=None;
    plan.replace_view(Some(&v),&gone,&tile);let next=crate::tile_overlay::PanelKeys::read(&tile);next.mark_changes(keys,&tile,&gone,&mut plan);
    plan.finish_damage(&mut tile);plan.paint_base(&mut tile,&gone,"",0.);tile.tile_clip_enabled=false;
    full.game_at(&gone,"",false,0.);assert!(tile.pixels==full.pixels,"removed actor leaves no old contour");
}
#[test]fn revision51_perception_gpu_quads_and_native_captures(){
    let r=room();let v=observe::observe(&r);let mut p=Painter::new();
    p.gpu=Some(crate::gpu_render::Frame::default());p.enemy_hover=Some(900f64.to_bits());p.pixels.fill(19);p.game_at(&v,"",false,0.);
    assert!(p.pixels.iter().all(|&b|b==19));assert!(p.gpu.as_ref().unwrap().quads.iter().any(|q|q.color==[INK[0],INK[1],INK[2],255]));
    if let Ok(dir)=std::env::var("CORPSE_PERCEPTION_REVIEW") {
        std::fs::create_dir_all(&dir).unwrap();let mut p=Painter::new();p.profile=appearance_profile::Profile::default();p.enemy_hover=Some(900f64.to_bits());
        for (w,h) in [(1280,800),(1920,1080),(960,1200),(1537,911)] {
            p.resize(w,h);p.game_at(&v,"",false,0.);
            let file=std::fs::File::create(format!("{dir}/perception-{w}x{h}.png")).unwrap();
            let mut enc=png::Encoder::new(file,w as u32,h as u32);enc.set_color(png::ColorType::Rgba);enc.set_depth(png::BitDepth::Eight);enc.write_header().unwrap().write_image_data(&p.pixels).unwrap();
        }
        p.resize(1280,800);p.game_at(&v,"",false,0.);
        let(cw,ch,ox,oy)=p.map_layout(v.width,v.height);
        let report=crate::inspection_ui::observe_tile(&r,23,14);
        crate::inspection_ui::paint_hover_at(&report,&mut p,(ox+23*cw,oy+14*ch,ox+24*cw,oy+15*ch));
        fn save(p:&Painter,path:String){let file=std::fs::File::create(path).unwrap();let mut enc=png::Encoder::new(file,p.width as u32,p.height as u32);enc.set_color(png::ColorType::Rgba);enc.set_depth(png::BitDepth::Eight);enc.write_header().unwrap().write_image_data(&p.pixels).unwrap();}
        save(&p,format!("{dir}/perception-map-hover.png"));
        let mut dark=r.clone();dark.floor_mut().unwrap().lights.fill(-15.);let dark_view=observe::observe(&dark);p.enemy_hover=None;p.game_at(&dark_view,"",false,0.);
        let report=crate::inspection_ui::observe_tile(&dark,20,14);p.centered_sheet(|p|crate::inspection_ui::paint(&report,p,0));
        save(&p,format!("{dir}/perception-tile-inspection.png"));
    }
}
