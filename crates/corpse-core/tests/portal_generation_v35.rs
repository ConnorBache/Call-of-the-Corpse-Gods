use corpse_core::{Actor,Field,Floor,GroundItem,Run,generation,portals_generation as p,startup,storage};
use std::collections::HashSet;

fn room(depth: u32) -> Floor {
    let mut f=Floor{depth:depth as f64,domain:"Dungeon".into(),tiles:vec!["floor".into();2291],lights:vec![0.;2291],seen:vec![false;2291],..Default::default()};
    for at in 0..2291 {if at%79==0 || at%79==78 || at/79==0 || at/79==28 {f.tiles[at]="wall".into();}}
    f.tiles[80]="up".into();f.tiles[27*79+77]="down".into();f
}
fn endpoint_set(f:&Floor)->HashSet<usize> {f.portals.as_ref().into_iter().flatten().flat_map(|p|[p.a,p.b]).collect()}
#[test]
fn depth_gate_determinism_and_legacy_nonmutation() {
    for depth in 1..=40 {
        let mut a=room(depth);let original=a.clone();
        p::install("repeatable",&mut a,&[],&[],34.);assert_eq!(a,original);
        p::install("repeatable",&mut a,&[],&[],35.);
        if depth<10 {assert_eq!(a,original);continue;}
        assert_eq!(a.portals.as_ref().unwrap().len(),1);assert!(p::all_endpoints_optional(&a));
        let mut b=original.clone();p::install("repeatable",&mut b,&[],&[],35.);assert_eq!(a,b);
        let saved=a.clone();p::install("different",&mut a,&[],&[],35.);assert_eq!(a,saved);
        a.portals=Field::Missing;assert_eq!(a,original);
    }
}
#[test]
fn seven_pairs_keep_all_endpoints_optional_with_ordered_colors() {
    assert_eq!(p::COLOR_NAMES,["blue","orange","red","yellow","green","indigo","violet"]);
    for seed in 0..64 {
        let mut f=room(10);p::install_count(&seed.to_string(),&mut f,&[],&[],99);
        assert_eq!(f.portals.as_ref().unwrap().len(),7);assert_eq!(endpoint_set(&f).len(),14);
        assert!(p::all_endpoints_optional(&f));assert!(p::validate_floor(&f));
        for (i,pair) in f.portals.as_ref().unwrap().iter().enumerate() {assert_eq!(pair.color_index as usize,i);}
    }
}
#[test]
fn never_occupies_stairs_fixture_actor_loot_or_required_single_width_route() {
    let mut f=room(10);
    // A wall divides the room except for a single mandatory walking cell.
    for y in 1..28 {f.tiles[y*79+39]="wall".into();} f.tiles[14*79+39]="floor".into();
    f.permanent_fire=vec![200.].into();f.grates=vec![201.].into();
    let actors=vec![Actor{x:10.,y:10.,..Default::default()}];
    let ground=vec![GroundItem{x:11.,y:10.,..Default::default()}];
    for seed in 0..64 {
        let mut g=f.clone();p::install_count(&seed.to_string(),&mut g,&actors,&ground,7);
        let endpoints=endpoint_set(&g);assert!(!endpoints.is_empty());
        for banned in [80,27*79+77,14*79+39,200,201,10*79+10,10*79+11] {assert!(!endpoints.contains(&banned));}
        assert!(p::all_endpoints_optional(&g));assert_eq!(g.tiles,f.tiles);
    }
}
#[test]
fn disconnected_if_both_avoided_is_rejected_even_if_individually_bypassable() {
    let mut f=room(10);
    for y in 1..28 {f.tiles[y*79+39]="wall".into();}
    let a=10*79+39;let b=20*79+39;f.tiles[a]="floor".into();f.tiles[b]="floor".into();
    f.portals=vec![p::PortalPair{a,b,color_index:0}].into();assert!(!p::all_endpoints_optional(&f));
    f.portals=Field::Missing;p::install_count("joint-check",&mut f,&[],&[],7);assert!(p::all_endpoints_optional(&f));
}
#[test]
fn unsuitable_maps_omit_instead_of_rewriting_or_forcing_a_portal() {
    let mut f=room(10);for t in &mut f.tiles {if t=="floor" {*t="water".into();}}
    let tiles=f.tiles.clone();p::install("wet",&mut f,&[],&[],35.);
    assert!(f.portals.as_ref().unwrap().is_empty());assert_eq!(f.tiles,tiles);
}
#[test]
fn unreachable_decorative_components_do_not_receive_endpoints() {
    let mut f=room(10);for y in 1..28 {f.tiles[y*79+39]="wall".into();}
    for seed in 0..16 {
        let mut g=f.clone();p::install_count(&seed.to_string(),&mut g,&[],&[],7);
        assert_eq!(g.portals.as_ref().unwrap().len(),7);
        assert!(endpoint_set(&g).iter().all(|at|at%79<39));
    }
}
#[test]
fn generated_six_biomes_and_multiple_seeds_preserve_walking_routes() {
    for domain in ["Dungeon","Cave","Meadow","Forest","Archipelago","Mountain"] {
        for seed in 0..8 {
            let mut f=generation::generate_floor_revision(10,seed,domain,true,79,None,34.).unwrap();
            let original=f.tiles.clone();p::install(&format!("{domain}:{seed}"),&mut f,&[],&[],35.);
            assert_eq!(f.portals.as_ref().unwrap().len(),1,"{domain} seed{seed}");
            assert!(p::all_endpoints_optional(&f),"{domain} seed{seed}");assert_eq!(f.tiles,original);
        }
    }
}
fn fixture()->Run {serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap()}
#[test]
fn optional_metadata_roundtrips_and_malformed_waking_or_dream_pairs_reject() {
    let mut r=fixture();r.simulation_revision=35.0.into();
    // Fixture floor10 is appended without simulation so metadata can be checked by both boundaries.
    while r.floors.len()<10 {let d=r.floors.len()+1;r.floors.push(std::sync::Arc::new(room(d as u32)));r.actors.insert(d.to_string(),std::sync::Arc::new(vec![]));r.ground.insert(d.to_string(),std::sync::Arc::new(vec![]));}
    p::install("save",std::sync::Arc::make_mut(&mut r.floors[9]),&[],&[],35.);
    let s=storage::serialize(&r).unwrap();let loaded=storage::deserialize(&s).unwrap();assert_eq!(loaded.floors[9].portals,r.floors[9].portals);
    for invalid in [serde_json::json!([{"a":80,"b":80,"colorIndex":0}]),serde_json::json!([{"a":80,"b":99999,"colorIndex":0}]),serde_json::json!([{"a":80,"b":81,"colorIndex":7}]),serde_json::json!([{"a":80,"b":81,"colorIndex":0},{"a":81,"b":82,"colorIndex":1}]),serde_json::Value::Null] {
        let mut v:serde_json::Value=serde_json::from_str(&s).unwrap();v["floors"][9]["portals"]=invalid;
        let raw=serde_json::to_string(&v).unwrap();assert!(storage::deserialize(&raw).is_err());assert!(storage::deserialize_compatible(&raw).is_err());
    }
    let mut legacy=r.clone();legacy.simulation_revision=34.0.into();assert!(storage::validate(&legacy).is_err());
    let mut dream=r.clone();dream.dreamwalking=corpse_core::dreamwalking::State{floors:vec![r.floors[9].clone()],..Default::default()}.into();
    let floor=std::sync::Arc::make_mut(&mut dream.dreamwalking.as_mut().unwrap().floors[0]);floor.portals.as_mut().unwrap()[0].b=99999;
    assert!(storage::validate(&dream).is_err());
}
#[test]
fn startup35_adds_portals_without_rebudgeting_actors_or_ground() {
    let f=generation::generate_floor_revision(10,7,"Dungeon",true,79,None,34.).unwrap();
    let mut old=startup::new_run_shell_revision("0-1-2:portal-budget","Unblessed",false,34.).unwrap();old.depth=10.;old.floors=vec![std::sync::Arc::new(f)];
    let mut new=old.clone();new.simulation_revision=35.0.into();
    startup::spawn_floor(&mut old,10).unwrap();startup::spawn_floor(&mut new,10).unwrap();
    assert_eq!(old.actors_at(10.).len(),new.actors_at(10.).len());
    for (a,b) in old.actors_at(10.).iter().zip(new.actors_at(10.)) {
        let av=serde_json::to_value(a).unwrap();let bv=serde_json::to_value(b).unwrap();
        for (k,v) in av.as_object().unwrap() {assert_eq!(v,&bv[k],"actor {} field {}",a.name,k);}
    }
    assert_eq!(old.ground,new.ground);assert_eq!(old.rng,new.rng);assert_eq!(old.next_id,new.next_id);
    let mut without=new.floor().unwrap().clone();without.portals=Field::Missing;assert_eq!(&without,old.floor().unwrap());
    assert_eq!(new.floor().unwrap().portals.as_ref().unwrap().len(),1);
}
#[test]
fn every_actor_profile_and_population_pool_remains_revision34() {
    for kind in corpse_core::population::catalog().actors.keys().map(String::as_str).chain(["night-crawler","skin-walker","saidribee-queen"]) {
        let old=corpse_core::population::create_actor_revision(kind,100.,8.,8.,false,34.);
        let new=corpse_core::population::create_actor_revision(kind,100.,8.,8.,false,35.);
        assert_eq!(serde_json::to_value(old.unwrap()).unwrap(),serde_json::to_value(new.unwrap()).unwrap(),"{kind}");
    }
    for depth in 1..=40 {for water in [false,true] {
        let old=corpse_core::ordinary_roster::candidates_revision(depth,water,34.).unwrap();
        let new=corpse_core::ordinary_roster::candidates_revision(depth,water,35.).unwrap();
        assert_eq!(format!("{old:?}"),format!("{new:?}"));
    }}
}
