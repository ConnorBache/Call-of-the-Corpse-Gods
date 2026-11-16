//! Fixed authored R50 ring powers. No quality tiers or loot probabilities implied.
use corpse_core::{Actor, Field, Floor, Item, Run, lighting_cache::LightingProjection,
    movement::Point, portals_generation::PortalPair, vision};
use std::sync::Arc;

fn room() -> Run {
    let mut r=Run{depth:1.,simulation_revision:50.0.into(),..Default::default()};
    r.player=Actor{id:1.,x:2.,y:2.,hp:100.,max_hp:100.,..Default::default()};
    r.floors.push(Arc::new(Floor{depth:1.,tiles:vec!["floor".into();121],lights:vec![-50.;121],
        seen:vec![false;121],..Default::default()}));
    r
}
fn ring(id:f64,effect:&str)->Item {
    Item{id,name:format!("Ring of {effect}"),kind:"ring".into(),effect:effect.to_string().into(),
        ..Default::default()}
}
fn assert_cached(r:&Run) {
    let projected=LightingProjection::new(r);
    for y in 0..11 { for x in 0..11 {
        assert_eq!(projected.current(Point{x:x as i32,y:y as i32}).to_bits(),vision::tile_light(r,x,y).to_bits(),"{x},{y}");
    }}
    assert_eq!(vision::terrain_visibility(r),(0..121).map(|i|vision::visible(r,i%11,i/11)).collect::<Vec<_>>());
    let actors=vision::VisibilityProjection::new(r);
    for a in r.actors_at(r.depth) { assert_eq!(actors.directly_visible_actor(a),vision::directly_visible_actor(r,a)); }
}
#[test]
fn light_is_additive_without_falloff_and_excludes_entire_wearer_tile() {
    let mut r=room();r.inventory.push(ring(2.,"light"));
    assert_eq!(vision::tile_light(&r,2,2),-50.);
    assert_eq!(vision::light(&r,&r.player),-50.);
    assert_eq!(vision::tile_light(&r,3,2),-40.);
    assert_eq!(vision::tile_light(&r,10,10),-40.);
    let colocated=Actor{id:3.,x:2.,y:2.,hp:10.,..Default::default()};
    assert_eq!(vision::light(&r,&colocated),-50.);
    r.inventory.push(ring(4.,"light"));
    assert_eq!(vision::tile_light(&r,10,10),-30.,"each distinct carried Light ring adds ten");
    r.floor_mut().unwrap().lights.fill(95.);
    assert_eq!(vision::tile_light(&r,2,2),95.);
    assert_eq!(vision::tile_light(&r,10,10),100.);
    assert_cached(&r);
}
#[test]
fn light_respects_walls_doors_portals_and_same_tick_cache_changes() {
    let mut r=room();r.inventory.push(ring(2.,"light"));
    assert_cached(&r);
    for y in 0..11 {r.floor_mut().unwrap().tiles[y*11+5]="wall".into();}
    assert_eq!(vision::tile_light(&r,8,2),-50.);
    assert_eq!(vision::tile_light(&r,5,2),-40.,"blocking destination is visible");
    assert_cached(&r);
    r.floor_mut().unwrap().portals=vec![PortalPair{a:2*11+3,b:2*11+7,color_index:0}].into();
    assert_eq!(vision::tile_light(&r,8,2),-40.,"ordinary optical LOS through a portal");
    assert_cached(&r);
    r.floor_mut().unwrap().doors=std::iter::once(("25".into(),corpse_core::FloorDoorsEntry{open:false,..Default::default()})).collect::<indexmap::IndexMap<_,_>>().into();
    assert_eq!(vision::tile_light(&r,8,2),-50.,"blocked portal endpoint");assert_cached(&r);
    r.floor_mut().unwrap().doors.as_mut().unwrap().get_mut("25").unwrap().open=true;
    assert_eq!(vision::tile_light(&r,8,2),-40.);assert_cached(&r);
    for flag in [Field::Missing,Field::Null,Field::Value(String::new()),"ring1".to_string().into()] {
        r.inventory[0].equipped=flag;
        assert_eq!(vision::tile_light(&r,8,2),-40.);assert_cached(&r);
    }
    let item=r.inventory.pop().unwrap();
    assert_eq!(vision::tile_light(&r,8,2),-50.);assert_cached(&r);
    r.inventory.push(item);r.player.x=8.;
    assert_eq!(vision::tile_light(&r,8,2),-50.,"new source tile excluded immediately");assert_cached(&r);
    r.player.dead=true.into();
    assert_eq!(vision::tile_light(&r,9,2),-50.);assert_cached(&r);
}
#[test]
fn stealth_only_multiplies_negative_actor_light_once() {
    let mut r=room();r.inventory.extend([ring(2.,"stealth"),ring(3.,"stealth"),ring(4.,"light")]);
    assert_eq!(vision::light(&r,&r.player),-60.);
    assert_eq!(vision::tile_light(&r,2,2),-50.,"stealth does not darken the tile");
    let mut observer=Actor{id:5.,x:4.,y:2.,hp:100.,..Default::default()};observer.stats.perception=20.;
    let hidden=vision::detection_chance(&r,&observer,&r.player);
    r.inventory.retain(|i|i.effect.as_ref().is_none_or(|e|e!="stealth"));
    assert!(vision::detection_chance(&r,&observer,&r.player)>hidden);
    r.inventory.push(ring(6.,"stealth"));
    for ambient in [-100.,-90.,-5.,0.,25.,100.] {
        r.floor_mut().unwrap().lights.fill(ambient);
        assert_eq!(vision::light(&r,&r.player),if ambient<0. {(ambient*1.2).max(-100.)}else{ambient});
        assert_cached(&r);
    }
}
#[test]
fn light_and_stealth_restore_and_remain_revision_gated() {
    let mut r=room();r.inventory.extend([ring(2.,"light"),ring(3.,"stealth")]);
    assert_cached(&r);
    let bytes=serde_json::to_string(&r).unwrap();
    let restored:Run=serde_json::from_str(&bytes).unwrap();
    assert_eq!(vision::light(&restored,&restored.player),-60.);
    assert_eq!(vision::tile_light(&restored,10,10),-40.);assert_cached(&restored);
    assert_eq!(serde_json::to_string(&r).unwrap(),bytes,"read-only lighting does not mutate serialized state");
    r.simulation_revision=46.0.into();
    assert_eq!(vision::light(&r,&r.player),-50.);assert_eq!(vision::tile_light(&r,10,10),-50.);assert_cached(&r);
}
#[test]
fn light_stays_in_the_wearers_plane() {
    let mut r=room();r.inventory.push(ring(2.,"light"));
    corpse_core::dreamwalking::ensure_layer(&mut r);
    r.player.conditions.insert("phantasmiiPlane".into(),1.);
    let p=LightingProjection::new(&r);
    assert_eq!(p.in_plane(Point{x:3,y:2},false),-50.);
    assert_eq!(p.in_plane(Point{x:3,y:2},true),-40.);
    assert_eq!(p.in_plane(Point{x:2,y:2},true),-50.);
}

#[test]
fn additive_light_reduces_absolute_darkness_aura_and_does_not_duplicate_shared_bodies() {
    let mut r=room(); r.inventory.push(ring(2.,"light"));
    let mut aura=Actor{id:3.,x:4.,y:2.,hp:100.,abilities:vec!["enemy:beast:darkness-aura".into()],..Default::default()};
    r.actors_at_mut(1.).push(aura.clone());
    assert_eq!(vision::tile_light(&r,4,2),-90.);
    assert_eq!(vision::light(&r,&aura),-90.);
    r.inventory.push(ring(4.,"light"));
    assert_eq!(vision::light(&r,&aura),-80.);assert_cached(&r);
    let mut body=r.player.clone();body.id=5.;body.x=8.;
    body.conditions.insert("halfHereClone".into(),1.);
    body.conditions.insert("halfHereOwner".into(),r.player.id);
    r.actors_at_mut(1.).push(body);
    assert_eq!(vision::light(&r,&aura),-80.,"shared owner does not double the same two rings");
    assert_eq!(LightingProjection::new(&r).ring_bonus_in_plane(Point{x:2,y:2},false),0.);
    assert_eq!(LightingProjection::new(&r).ring_bonus_in_plane(Point{x:8,y:2},false),0.);
    assert_cached(&r);
    r.simulation_revision=46.0.into();assert_eq!(vision::light(&r,&aura),-100.);
    r.simulation_revision=50.0.into();
    aura.x=r.player.x;aura.y=r.player.y;
    assert_eq!(vision::light(&r,&aura),-100.,"wearer tile remains excluded even for colocated aura bearer");
}

#[test]
fn r52_stealth_stacks_but_light_still_raises_absolute_darkness() {
    let mut r=room();r.simulation_revision=52.0.into();
    r.inventory.extend([ring(2.,"stealth"),ring(3.,"stealth")]);
    assert!((vision::light(&r,&r.player)+72.).abs()<1e-9);
    assert_eq!(vision::tile_light(&r,2,2),-50.);
    r.inventory.clear();r.inventory.extend([ring(4.,"light"),ring(5.,"light")]);
    r.floor_mut().unwrap().lights.fill(-100.);
    assert_eq!(vision::tile_light(&r,3,2),-80.);
    assert_eq!(vision::tile_light(&r,2,2),-100.);
    assert_cached(&r);
}
