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
    r.floor_mut().unwrap().lights.fill(1.);let v=observe::observe(&r);assert_eq!(boundary(&v).edges.len(),2*(v.width+v.height),"positive light bypasses distance; enclose known map limits without a fake radius");
    r.player.conditions.insert("invisible".into(),1.);assert!(boundary(&observe::observe(&r)).edges.is_empty());
}
#[test]fn revision51_perception_known_wall_and_unknown_ray_enclose_only_known_sight(){
    let r=room();let v=observe::observe(&r);let mut o=(**v.terrain_snapshot.as_ref().unwrap()).clone();let w=o.width;
    o.cells[14*w+26].tile=Some("wall".into());
    let b=super::observe(&r,Some(&o),&v.creatures);assert!(b[0].edges.contains(&Edge{x:27,y:14,vertical:true}));
    // Close the known area at the unknown cell; do not invent sight beyond it.
    o.cells[14*w+26].current=false;o.cells[14*w+26].direct=false;o.cells[14*w+26].tile=None;
    let b=super::observe(&r,Some(&o),&v.creatures);
    assert!(b[0].edges.contains(&Edge{x:26,y:14,vertical:true}));
    assert!(!b[0].edges.iter().any(|e|e.vertical&&e.y==14&&e.x>26&&e.x<=34));
    assert_closed(&b[0].edges);
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
#[test]fn revision51_perception_moving_light_omits_optical_only_preview(){
    let mut r=room();let v=observe::observe(&r);let o=v.terrain_snapshot.as_deref();
    r.player.abilities=vec!["anavani-shine".into()];
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

// Odd/even crossings at cell centers reconstruct the exact enclosed cell union.
fn enclosed(edges:&[Edge],x:usize,y:usize)->bool {
    edges.iter().filter(|e|e.vertical&&e.y==y&&e.x>x).count()%2==1
}
fn assert_closed(edges:&[Edge]) {
    let mut degree=std::collections::BTreeMap::new();
    let mut unique=std::collections::BTreeSet::new();
    for e in edges {
        assert!(unique.insert((e.x,e.y,e.vertical)),"duplicate edge {e:?}");
        for point in [(e.x,e.y),if e.vertical{(e.x,e.y+1)}else{(e.x+1,e.y)}] {
            *degree.entry(point).or_insert(0)+=1;
        }
    }
    assert!(degree.values().all(|d|d%2==0),"open endpoint: {degree:?}");
}
#[test]
fn revision52_perception_encloses_all_masks_holes_and_map_edges() {
    // Exhaustive ternary 3x3 masks cover unknown holes, excluded occluder cells,
    // disconnected islands, diagonally touching corners and every map border.
    for mut code in 0..3usize.pow(9) {
        let states=(0..9).map(|_|{let state=[None,Some(false),Some(true)][code%3];code/=3;state}).collect::<Vec<_>>();
        let e=edges(3,3,&states);assert_closed(&e);
        assert!(e.len()<=4*9);
        for y in 0..3 {for x in 0..3 {assert_eq!(enclosed(&e,x,y),states[y*3+x]==Some(true));}}
    }
    let mut donut=vec![Some(true);25];donut[12]=Some(false);
    let e=edges(5,5,&donut);assert_eq!(e.len(),24);assert_closed(&e);
    for side in [Edge{x:2,y:2,vertical:true},Edge{x:3,y:2,vertical:true},Edge{x:2,y:2,vertical:false},Edge{x:2,y:3,vertical:false}] {
        assert!(e.contains(&side),"internal occluder hole side missing {side:?}");
    }
}
#[test]
fn revision52_perception_remembered_sight_uses_only_known_geometry_and_light_bounds() {
    let mut r=room();r.simulation_revision=52.0.into();r.actors_at_mut(r.depth)[0].stats.perception=200.;
    let f=r.floor_mut().unwrap();f.seen.fill(true);f.remembered_tiles=f.tiles.clone().into();
    let v=observe::observe(&r);let o=v.terrain_snapshot.as_deref().unwrap();let w=o.width;
    let remembered=&o.cells[14*w+43];
    assert!(!remembered.current&&remembered.remembered&&remembered.tile.as_deref()==Some("floor"));
    assert_eq!(remembered.illumination,None);
    assert!(enclosed(&boundary(&v).edges,43,14),"P200 guarantees distance20 even at unknown -100 light");
    assert!(!enclosed(&boundary(&v).edges,44,14),"remembered uncertain light must not be assumed neutral");
    assert!(enclosed(&boundary(&v).edges,40,14),"current observed neutral light stays exact");
    assert_closed(&boundary(&v).edges);
    let before=v.perception_boundaries.clone();
    for x in [43,44] {r.floor_mut().unwrap().lights[14*w+x]=-100.;r.floor_mut().unwrap().tiles[14*w+x]="wall".into();}
    assert_eq!(super::observe(&r,Some(o),&v.creatures),before,"frozen knowledge ignores live remembered-cell mutations");
    assert_eq!(observe::observe(&r).perception_boundaries,before,"fresh hidden wall/light mutations do not change remembered knowledge");
    let mut unknown=o.clone();unknown.cells[14*w+43].remembered=false;unknown.cells[14*w+43].tile=None;
    let b=super::observe(&r,Some(&unknown),&v.creatures);
    assert!(!enclosed(&b[0].edges,43,14),"unknown destination never enters enclosed sight");assert_closed(&b[0].edges);
    // A remembered opaque wall affects only the known route, not hidden live state.
    let mut blocked=o.clone();blocked.cells[14*w+41].tile=Some("wall".into());
    let b=super::observe(&r,Some(&blocked),&v.creatures);
    assert!(enclosed(&b[0].edges,41,14),"visible blocker face retains core ray semantics");
    assert!(!enclosed(&b[0].edges,42,14),"remembered wall occludes the known optical route");assert_closed(&b[0].edges);
}
fn capture_perception(p:&Painter,name:&str) {
    if let Ok(dir)=std::env::var("CORPSE_PERCEPTION_REVIEW") {
        std::fs::create_dir_all(&dir).unwrap();
        let file=std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
        let mut enc=png::Encoder::new(file,p.width as u32,p.height as u32);
        enc.set_color(png::ColorType::Rgba);enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(&p.pixels).unwrap();
    }
}
fn ink_at(p:&Painter,x:usize,y:usize)->bool {p.pixels[(y*p.width+x)*4..(y*p.width+x)*4+3]==INK}
#[test]
fn revision52_perception_native_map_border_corners_damage_and_hole_are_closed() {
    let mut r=room();r.floor_mut().unwrap().lights.fill(1.);let mut v=observe::observe(&r);
    let mut p=Painter::new();let mut full=Painter::new();let mut retained=render::RetainedGame::default();
    for (w,h) in [(1280,800),(1920,1080),(960,1200),(1537,911),(2560,1600)] {
        for painter in [&mut p,&mut full] {painter.resize(w,h);painter.set_responsive_canvas(true);}
        p.enemy_hover=Some(900f64.to_bits());paint_pair(&mut p,&mut full,&v,&mut retained,true,0.);
        let(cw,ch,ox,oy)=p.map_layout(v.width,v.height);
        let map=p.logical_damage_rect(ox,oy,cw*v.width as i32,ch*v.height as i32).unwrap();
        // Every perimeter pixel, not just the existence of an edge descriptor.
        for x in map.x..map.right() {assert!(ink_at(&p,x,map.y),"top {w}x{h} {x}");assert!(ink_at(&p,x,map.bottom()-1),"bottom {w}x{h} {x}");}
        for y in map.y..map.bottom() {assert!(ink_at(&p,map.x,y),"left {w}x{h} {y}");assert!(ink_at(&p,map.right()-1,y),"right {w}x{h} {y}");}
        let boxes=bounds(&p,&v,boundary(&v));
        for (x,y) in [(map.x,map.y),(map.right()-1,map.y),(map.x,map.bottom()-1),(map.right()-1,map.bottom()-1)] {
            assert!(boxes.iter().any(|b|b.contains(x,y)),"new map corner missing from dirty damage");
        }
        capture_perception(&p,&format!("perception-enclosed-map-border-{w}x{h}"));
        p.enemy_hover=None;paint_pair(&mut p,&mut full,&v,&mut retained,false,0.);
    }
    // One confidently excluded dark tile creates an internal hole, without
    // hiding the adjacent currently observed cells behind a pessimistic light bound.
    r.player.stats.perception=100.;let w=r.floor().unwrap().width();r.floor_mut().unwrap().lights[14*w+27]=-100.;
    v=observe::observe(&r);assert!(!enclosed(&boundary(&v).edges,27,14));assert!(enclosed(&boundary(&v).edges,26,14));assert_closed(&boundary(&v).edges);
    p.resize(1280,800);p.enemy_hover=Some(900f64.to_bits());p.game_at(&v,"",false,0.);
    let(cw,ch,ox,oy)=p.map_layout(v.width,v.height);
    let hole=p.logical_damage_rect(ox+27*cw,oy+14*ch,cw+1,ch+1).unwrap();
    for (x,y) in [(hole.x,hole.y),(hole.right()-1,hole.y),(hole.x,hole.bottom()-1),(hole.right()-1,hole.bottom()-1)] {assert!(ink_at(&p,x,y),"hole corner missing");}
    let center=p.logical_damage_rect(ox+27*cw+cw/2,oy+14*ch+ch/2,1,1).unwrap();assert!(!ink_at(&p,center.x,center.y),"excluded hole is not filled red");
    capture_perception(&p,"perception-enclosed-dark-hole");
}
#[test]
fn revision52_perception_native_known_fog_and_wall_contours_match_tile_repaints() {
    let mut r=room();r.simulation_revision=52.0.into();r.actors_at_mut(r.depth)[0].stats.perception=200.;
    let mut previous=None;let mut p=Painter::new();let mut full=Painter::new();let mut plan=render::tile_damage::TilePlanner::default();let mut keys=None;
    for scenario in ["fog-limit","remembered-sight","known-wall"] {
        if scenario=="remembered-sight" {let f=r.floor_mut().unwrap();f.seen.fill(true);f.remembered_tiles=f.tiles.clone().into();}
        if scenario=="known-wall" {let w=r.floor().unwrap().width();for y in 7..22 {r.floor_mut().unwrap().tiles[y*w+28]="wall".into();}}
        let v=observe::observe(&r);assert_closed(&boundary(&v).edges);
        if scenario=="fog-limit" {assert!(on(&v,41,14,true),"enclose the player knowledge limit");assert!(!enclosed(&boundary(&v).edges,41,14));}
        plan.replace_view(previous.as_ref(),&v,&p);
        for hover in [Some(900f64.to_bits()),None,Some(900f64.to_bits())] {
            p.enemy_hover=hover;full.enemy_hover=hover;
            let next=crate::tile_overlay::PanelKeys::read(&p);next.mark_changes(keys,&p,&v,&mut plan);keys=Some(next);
            plan.finish_damage(&mut p);plan.paint_base(&mut p,&v,"",0.);p.tile_clip_enabled=false;plan.clear_frame();
            full.game_at(&v,"",false,0.);assert!(p.pixels==full.pixels,"tile mismatch in {scenario} hover {hover:?}");
        }
        capture_perception(&p,&format!("perception-enclosed-{scenario}"));previous=Some(v);
    }
}
#[test]
#[ignore="Isolated SDL contour readback; SDL_VIDEODRIVER=offscreen --test-threads=1"]
fn revision52_perception_sdl_native_closed_contour_parity() {
    let mut r=room();r.player.stats.perception=100.;r.floor_mut().unwrap().lights.fill(1.);
    let w=r.floor().unwrap().width();r.floor_mut().unwrap().lights[14*w+27]=-100.;let v=observe::observe(&r);
    let sdl=sdl2::init().unwrap();let video=sdl.video().unwrap();
    for (w,h) in [(1280,800),(1537,911),(2560,1600)] {
        let window=video.window("Enclosed enemy sight",w,h).hidden().build().unwrap();
        let (mut canvas,_)=crate::gpu_render::create_canvas(window,false).unwrap();
        let creator=canvas.texture_creator();let mut renderer=crate::gpu_render::Renderer::new(&creator,2048);
        let mut cpu=Painter::new();let mut gpu=Painter::new();
        for p in [&mut cpu,&mut gpu] {p.resize(w as usize,h as usize);p.set_responsive_canvas(true);p.enemy_hover=Some(900f64.to_bits());}
        gpu.gpu=Some(crate::gpu_render::Frame::default());cpu.game_at(&v,"",false,0.);gpu.game_at(&v,"",false,0.);
        renderer.draw(&mut canvas,gpu.gpu.as_ref().unwrap()).unwrap();
        let actual=canvas.read_pixels(None,sdl2::pixels::PixelFormatEnum::RGBA32).unwrap();
        let maximum=actual.chunks_exact(4).zip(cpu.pixels.chunks_exact(4))
            .flat_map(|(a,b)|(0..3).map(move|k|a[k].abs_diff(b[k]))).max().unwrap();
        assert!(maximum<=3,"CPU/SDL channel delta {maximum} at {w}x{h}");
        for (a,b) in actual.chunks_exact(4).zip(cpu.pixels.chunks_exact(4)) {
            if b[..3]==INK {assert_eq!(&a[..3],&INK,"contour pixel differs at {w}x{h}");}
        }
        cpu.pixels=actual;capture_perception(&cpu,&format!("perception-enclosed-sdl-{w}x{h}"));
    }
}

/// Full native input/compositor regression using the authored room, not a
/// hand-assigned hover ID or a floor carved/revealed for the screenshot.
#[test]
fn revision52_perception_authored_room_input_overlay_and_core_truth() {
    let mut run:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    // The original fixture predates this presentation. Only opt into R52;
    // retain its actors, stairs, terrain, Perception, light and memory.
    run.simulation_revision=52.0.into();
    let unchanged=serde_json::to_vec(&run).unwrap();
    let view=observe::observe(&run);
    let actor=&run.actors_at(run.depth)[0];
    let id=actor.id.to_bits();
    let contour=selected(&view,Some(id)).expect("authored enemy has optical preview");
    assert!(!contour.edges.is_empty());assert_closed(&contour.edges);
    let observation=view.terrain_snapshot.as_ref().unwrap();
    let unknown=observation.cells.iter().map(|c|!known(c)).collect::<Vec<_>>();
    let from=Point{x:actor.x as i32,y:actor.y as i32};
    let perception=corpse_core::combat::effective_stats(&run,actor).perception;
    let mut checked=0;let mut admitted=0;
    for cell in &observation.cells {
        let (x,y)=(cell.index%view.width,cell.index/view.width);
        let inside=enclosed(&contour.edges,x,y);
        if inside {assert!(known(cell),"unknown tile entered preview at {x},{y}");admitted+=1;}
        if !cell.current||!cell.direct {continue;}
        let mut target=run.player.clone();target.x=x as f64;target.y=y as f64;
        // Independent actual-world scalar query is test-only. Production never
        // reads this mask and clips it: hidden blockers must not shape a preview.
        let actual=vision::actor_chance(&run,actor,&target,perception)>0.;
        if inside {assert!(actual,"preview claims impossible enemy sight at {x},{y}");}
        if vision::ray(view.width,view.height,&unknown,from,Point{x:x as i32,y:y as i32}) {
            assert_eq!(inside,actual,"all known direct ray at {x},{y}");checked+=1;
        }
    }
    assert!(checked>100&&admitted>100,"fixture must test a substantive region");
    let report=crate::inspection_ui::observe_tile(&run,actor.x as usize,actor.y as usize);
    let mut capture_names=Vec::new();
    for (w,h) in [(1280,800),(1537,911),(960,1200)] {
        let mut tile=Painter::new();let mut full=Painter::new();
        for p in [&mut tile,&mut full] {p.resize(w,h);p.set_responsive_canvas(true);}
        let mut plan=render::tile_damage::TilePlanner::default();
        let mut track=crate::tile_overlay::Tracker::default();let mut keys=None;
        plan.replace_view(None,&view,&tile);
        let (cw,ch,ox,oy)=tile.map_layout(view.width,view.height);
        let map_point=(ox+actor.x as i32*cw+cw/2,oy+actor.y as i32*ch+ch/2);
        let anchor=(ox+actor.x as i32*cw,oy+actor.y as i32*ch,ox+(actor.x as i32+1)*cw,oy+(actor.y as i32+1)*ch);
        let layout=render::enemy_list::Layout::resolve_frame(&tile.profile,ox,tile.frame());
        let content=layout.content_view(&tile,&view);
        assert!(render::enemy_list::Content::stair_count(&view)>0,"real stair row precedes enemy");
        let side_x=layout.rows.0+10;
        let side_y=(layout.rows.1..layout.rows.1+layout.rows.3).find(|&y|
            content.hit(side_x,y,0).and_then(|i|render::enemy_list::Content::actor(&view,i))==Some(id)).unwrap();
        let side_point=(side_x,side_y);
        for (step,(point,allowed,popup)) in [
            (None,true,false),(Some(map_point),true,true),(Some(map_point),true,true),
            (Some(side_point),true,false),(Some(side_point),false,false),
            (Some(side_point),true,false),(None,true,false),(Some(map_point),true,true),
        ].into_iter().enumerate() {
            crate::refresh_panel_hover(&mut tile,&view,point,allowed);
            assert_eq!(tile.enemy_hover,if point.is_some()&&allowed{Some(id)}else{None},"hover step {step}");
            let now=step as f64*210.;
            let next=crate::tile_overlay::PanelKeys::read(&tile);
            next.mark_changes(keys,&tile,&view,&mut plan);keys=Some(next);
            plan.mark_animation(&view,&tile,now,false);
            let state=crate::tile_overlay::Frame{
                hover:if popup{crate::tile_overlay::hover(&tile,&report,anchor)}else{None},
                ..Default::default()
            };
            if let Some(hover)=&state.hover {
                assert_eq!(Some(hover.bounds),crate::overlay_damage::hover(&tile,&report,anchor).map(|h|h.bounds),
                    "retained and tile tooltips use the same responsive footprint");
            }
            for rect in track.changes(&tile,&state) {plan.mark_native_rect(rect);}
            crate::tile_overlay::finish_dependencies(&mut tile,&mut plan,&state);
            plan.paint_base(&mut tile,&view,"",now);
            if !tile.tile_mask.is_empty() {if let Some(hover)=&state.hover {
                crate::inspection_ui::paint_hover_at(&hover.report,&mut tile,hover.tile);
            }}
            tile.tile_clip_enabled=false;
            full.enemy_hover=tile.enemy_hover;full.enemy_list_offset=tile.enemy_list_offset;
            full.game_at(&view,"",false,now);
            let backdrop_sample=(popup&&w==960).then(|| {
                let r=crate::modal_ui::hover_bounds_painter(&full,anchor,620);
                assert!(r.1>render::HEIGHT as i32,"exercise background below old fixed viewport");
                let point=full.logical_damage_rect(r.0+3,r.1+3,2,2).unwrap();
                let at=(point.y*full.width+point.x)*4;
                let expected=full.profile.rgb("ui.colors.tooltip").map(|c|c as u32*245);
                (at,std::array::from_fn::<_,3,_>(|k|((expected[k]+full.pixels[at+k]as u32*10)/255)as u8))
            });
            if let Some(hover)=&state.hover {crate::inspection_ui::paint_hover_at(&hover.report,&mut full,hover.tile);}
            if let Some((at,expected))=backdrop_sample {
                assert_eq!(&full.pixels[at..at+3],&expected,"portrait hover must paint its translucent background");
            }
            assert!(tile.pixels==full.pixels,"actual input/tooltip tile repaint differs at {w}x{h}, step {step}");
            if tile.enemy_hover.is_some() {
                let map=tile.logical_damage_rect(ox,oy,cw*view.width as i32,ch*view.height as i32).unwrap();
                let mut checked_pixels=0;
                for &edge in &contour.edges {
                    let (x,y,ww,hh)=logical(&tile,&view,edge);
                    if let Some(rect)=tile.logical_damage_rect(x,y,ww,hh).and_then(|r|r.intersect(map)) {
                        for y in rect.y..rect.bottom() {for x in rect.x..rect.right() {
                            if state.hover.as_ref().is_none_or(|h|!h.bounds.contains(x,y)) {
                                assert!(ink_at(&tile,x,y),"missing contour pixel {x},{y}, step {step}");checked_pixels+=1;
                            }
                        }}
                    }
                }
                // Tooltip damage bounds deliberately overestimate the full column.
                // Require full pixel coverage on the unobscured sidebar pathway.
                if state.hover.is_none() {assert!(checked_pixels>100);}
            }
            if step==1||step==3||step==4 {
                let name=format!("perception-authored-{}-{w}x{h}",match step{1=>"map-hover",3=>"sidebar-hover",_=>"cancel"});
                capture_perception(&tile,&name);capture_names.push(name);
            }
            track.commit(state);plan.clear_frame();
        }
    }
    assert_eq!(serde_json::to_vec(&run).unwrap(),unchanged,"hover does not mutate state, RNG or turns");
    if let Ok(dir)=std::env::var("CORPSE_PERCEPTION_REVIEW") {
        std::fs::write(format!("{dir}/perception-authored-evidence.json"),serde_json::to_vec_pretty(&serde_json::json!({
            "fixture":"avatar-room-v1.json","fixture_change":"simulationRevision 29 to 52 only",
            "enemy":actor.name,"checked_known_ray_cells":checked,"enclosed_known_cells":admitted,
            "edges":contour.edges.len(),"closed":true,"input":"refresh_panel_hover, map and actual sidebar after stair row",
            "compositor":"TilePlanner plus actual hover tooltip, compared byte-for-byte with full paint",
            "steps_per_size":8,"run_unchanged":true,"captures":capture_names,
            "windows_runtime_tested":false,"physical_gpu_tested":false
        })).unwrap()).unwrap();
    }
}

fn special_room(kind:&str)->Run {
    let mut r=room();r.simulation_revision=52.0.into();r.player.stats.perception=500.;
    // Optical visibility of the hypothetical target is disabled, exposing any
    // wrong special-sense radius or qualification instead of hiding it in a union.
    r.player.conditions.insert("invisible".into(),1.);
    let depth=r.depth;let player=r.player.id;
    let a=&mut r.actors_at_mut(depth)[0];
    match kind {
        "telepathic"=>{a.conditions.insert("telepathic".into(),10.);},
        "all-seeing"|"all-seeing-wrong-depth"=>{
            a.conditions.insert("allSeeingReveal".into(),1.);
            a.conditions.insert("allSeeingDepth".into(),depth+if kind.ends_with("wrong-depth"){1.}else{0.});
        },
        "came"|"came-wrong-target"|"came-expired"=>{
            a.conditions.insert("cameTurns".into(),if kind=="came-expired"{0.}else{10.});
            a.conditions.insert("cameTarget".into(),player+if kind=="came-wrong-target"{1.}else{0.});
        },
        "mind-read"|"mind-read-active"=>{
            a.abilities.push("numo-mind-read".into());
            if kind.ends_with("active") {a.active.push("numo-mind-read".into());}
        },
        "dream-inactive"|"dream-active"|"dream-sleeping"|"dream-resting"|"dream-recovering"=>{
            a.abilities.push("ursa-dream-see".into());
            if kind=="dream-active" {a.active.push("ursa-dream-see".into());}
            if kind=="dream-sleeping" {r.player.state="sleeping".into();}
            if kind=="dream-resting" {r.player.conditions.insert("monsterAllyRest".into(),1.);}
            if kind=="dream-recovering" {r.player.state="neutral".into();r.player.conditions.insert("monsterRecoverySleep".into(),1.);}
        },
        "witch"|"witch-noncaster"|"witch-zero-health"=>{
            a.abilities.push(corpse_core::revision50_combat::WITCH_HATRED.into());
            if kind!="witch-noncaster" {r.player.abilities.push(corpse_core::revision50_combat::MAGICKAL_CASTER.into());}
            if kind=="witch-zero-health" {r.player.hp=0.;}
        },
        _=>panic!("unrecognized special fixture {kind}"),
    }
    r
}

#[test]
fn revision52_perception_special_senses_match_actual_rules_over_every_known_cell() {
    for kind in ["telepathic","all-seeing","all-seeing-wrong-depth","came","came-wrong-target","came-expired",
        "mind-read","mind-read-active","dream-inactive","dream-active","dream-sleeping","dream-resting",
        "dream-recovering","witch","witch-noncaster","witch-zero-health"] {
        let r=special_room(kind);let before=serde_json::to_vec(&r).unwrap();let v=observe::observe(&r);
        let o=v.terrain_snapshot.as_ref().unwrap();let b=boundary(&v);assert_closed(&b.edges);
        let a=&r.actors_at(r.depth)[0];let mut admitted=0;
        for cell in &o.cells {
            assert!(cell.current&&cell.direct,"whole open fixture must be observed");
            let (x,y)=(cell.index%o.width,cell.index/o.width);
            let mut target=r.player.clone();target.x=x as f64;target.y=y as f64;
            let actual=vision::plausibly_visible_actor(&r,a,&target);
            assert_eq!(enclosed(&b.edges,x,y),actual,"special {kind} at {x},{y}");
            admitted+=usize::from(actual);
        }
        if ["telepathic","all-seeing","came","dream-active","dream-sleeping","dream-resting","dream-recovering","witch"].contains(&kind) {
            assert_eq!(admitted,o.cells.len());
        }
        if kind=="mind-read" {assert!(enclosed(&b.edges,28,14));assert!(!enclosed(&b.edges,29,14));}
        if kind=="mind-read-active" {assert!(enclosed(&b.edges,43,14));assert!(!enclosed(&b.edges,44,14));}
        assert_eq!(serde_json::to_vec(&r).unwrap(),before,"special projection mutated run {kind}");
    }
}

#[test]
fn revision52_perception_special_senses_keep_unknown_holes_and_identity_private() {
    let mut r=special_room("telepathic");let v=observe::observe(&r);
    let mut o=(**v.terrain_snapshot.as_ref().unwrap()).clone();let hidden=14*o.width+27;
    o.cells[hidden].tile=None;o.cells[hidden].current=false;o.cells[hidden].direct=false;o.cells[hidden].remembered=false;
    let preview=super::observe(&r,Some(&o),&v.creatures);assert_closed(&preview[0].edges);
    assert!(!enclosed(&preview[0].edges,27,14));assert!(enclosed(&preview[0].edges,26,14));
    r.floor_mut().unwrap().tiles[hidden]="wall".into();r.floor_mut().unwrap().lights[hidden]=-100.;
    assert_eq!(super::observe(&r,Some(&o),&v.creatures),preview,"special contour cannot expose hidden mutations");
    // Remembered known geometry is enough for geometry-independent sensing;
    // no remembered light or current hidden geometry is needed.
    o.cells[hidden].tile=Some("floor".into());o.cells[hidden].remembered=true;o.cells[hidden].illumination=None;
    let remembered=super::observe(&r,Some(&o),&v.creatures);assert!(enclosed(&remembered[0].edges,27,14));
    let mut concealed=v.creatures.clone();concealed[0].hallucination=Some(std::sync::Arc::new(std::array::from_fn(|_|v.creatures[0].clone())));
    assert!(super::observe(&r,Some(&o),&concealed).is_empty(),"hallucination must not disclose real senses");
    o.cells[14*o.width+23].direct=false;
    assert!(super::observe(&r,Some(&o),&v.creatures).is_empty(),"sensed-only enemy has no disclosed optical identity");
    // Directly observed terrain does not imply the actor standing there has
    // been directly identified. Exercise observe(), not an edited tile flag.
    for (hidden,amount) in [("invisible",1.),("stealth",1000.)] {
        let mut sensed=special_room("telepathic");
        sensed.player.conditions.insert("telepathic".into(),10.);
        sensed.actors_at_mut(sensed.depth)[0].conditions.insert(hidden.into(),amount);
        let v=observe::observe(&sensed);
        assert_eq!(v.creatures[0].state,"sensed");
        let tile=&v.terrain_snapshot.as_ref().unwrap().cells[14*v.width+23];
        assert!(tile.current&&tile.direct,"terrain remains directly observed around the sensed actor");
        assert!(v.perception_boundaries.is_empty(),"{hidden} sensed actor cannot disclose underlying senses");
        let depth=sensed.depth;let a=&mut sensed.actors_at_mut(depth)[0];
        a.conditions.remove("telepathic");a.conditions.insert("allSeeingReveal".into(),1.);a.conditions.insert("allSeeingDepth".into(),depth);
        assert!(observe::observe(&sensed).perception_boundaries.is_empty(),"hidden sense changes must remain undisclosed");
    }
}

#[test]
fn revision52_perception_special_senses_ignore_uncertain_light_and_portals_only_when_valid() {
    for kind in ["telepathic","all-seeing","came","mind-read","mind-read-active","dream-sleeping","witch"] {
        let mut r=special_room(kind);let v=observe::observe(&r);let mut o=(**v.terrain_snapshot.as_ref().unwrap()).clone();
        // A known endpoint with unknown mate disables the optical geometry;
        // it cannot erase a special sense that ignores walls and light.
        let w=o.width;let mate=14*w+70;
        r.floor_mut().unwrap().portals=vec![corpse_core::portals_generation::PortalPair{a:14*w+25,b:mate,color_index:0}].into();
        o.cells[mate].current=false;o.cells[mate].direct=false;o.cells[mate].remembered=false;o.cells[mate].tile=None;
        r.player.abilities.push("anavani-shine".into());
        r.inventory.push(corpse_core::Item{id:990.,kind:"ring".into(),name:"Ring of Light".into(),effect:"light".to_string().into(),equipped:"ring1".to_string().into(),..Default::default()});
        let b=super::observe(&r,Some(&o),&v.creatures);assert_eq!(b.len(),1,"known sense lost for {kind}");assert_closed(&b[0].edges);
        assert!(enclosed(&b[0].edges,24,14),"local guaranteed sense lost for {kind}");
        assert!(!enclosed(&b[0].edges,70,14),"unknown destination disclosed by {kind}");
    }
}

#[test]
fn revision52_perception_dream_los_and_special_union_native_captures() {
    let mut r=special_room("dream-active");let w=r.floor().unwrap().width();
    for y in 5..24 {r.floor_mut().unwrap().tiles[y*w+28]="wall".into();}
    let v=observe::observe(&r);let b=boundary(&v);assert_closed(&b.edges);
    assert!(enclosed(&b.edges,28,14),"known wall face is visible to Dream See");
    assert!(!enclosed(&b.edges,29,14),"Dream See cannot see through a wall while target awake");
    let mut p=Painter::new();p.enemy_hover=Some(900f64.to_bits());p.game_at(&v,"",false,0.);
    capture_perception(&p,"perception-special-dream-see-wall");
    for kind in ["telepathic","mind-read","mind-read-active","witch"] {
        let mut r=special_room(kind);
        // Restore ordinary optical eligibility, so the bounded sense and optical
        // area must be unioned rather than replacing each other.
        r.player.conditions.remove("invisible");r.player.stats.perception=0.;
        let w=r.floor().unwrap().width();for y in 6..23 {r.floor_mut().unwrap().tiles[y*w+26]="wall".into();}
        let f=r.floor_mut().unwrap();f.seen.fill(true);f.remembered_tiles=f.tiles.clone().into();
        let v=observe::observe(&r);let b=boundary(&v);assert_closed(&b.edges);
        assert!(enclosed(&b.edges,26,14),"optical wall face must remain included for {kind}");
        assert!(enclosed(&b.edges,27,14),"special sense must extend beyond the wall for {kind}");
        if kind=="mind-read" {assert!(!enclosed(&b.edges,29,14),"radius5 plus occluded optical route");}
        let mut p=Painter::new();p.set_responsive_canvas(true);
        let (cw,ch,ox,oy)=p.map_layout(v.width,v.height);
        crate::refresh_panel_hover(&mut p,&v,Some((ox+23*cw+cw/2,oy+14*ch+ch/2)),true);
        assert_eq!(p.enemy_hover,Some(900f64.to_bits()));p.game_at(&v,"",false,0.);
        capture_perception(&p,&format!("perception-special-{kind}-known-wall"));
    }
}
