use corpse_core::{Floor, Run, engine, environment, population, startup, terrain_runtime};
use std::sync::Arc;
const W:usize=79;
const H:usize=29;
fn room(revision:f64)->Run {
    let mut r=startup::new_run_shell_revision("0-5-11:r37-terrain", "Unblessed",false,revision).unwrap();
    r.player.x=10.;r.player.y=10.;r.player.conditions.clear();r.player.abilities.clear();r.player.active.clear();
    let mut tiles=vec!["wall".into();W*H];
    for y in 1..H-1 {for x in 1..W-1 {tiles[y*W+x]="floor".into();}}
    let mut f=Floor {width:(W as f64).into(),height:(H as f64).into(),depth:1.,domain:"Dungeon".into(),tiles,lights:vec![100.;W*H],seen:vec![true;W*H],..Default::default()};
    environment::initialize(&mut f).unwrap();r.floors=vec![Arc::new(f)];r.actors.clear();r.actors.insert("1".into(),Arc::new(vec![]));r.ground.clear();r.ground.insert("1".into(),Arc::new(vec![]));r.inventory.clear();r
}
fn cover(r:&mut Run,which:&str,at:usize) {
    let f=r.floor_mut().unwrap();let n=f.tiles.len();
    let layer=match which {"ice"=>&mut f.ice,"snow"=>&mut f.snow,"hellfire"=>&mut f.hellfire,_=>panic!()};
    layer.get_or_insert_with(||vec![false;n])[at]=true;
}
fn actor(r:&mut Run,kind:&str)->f64 {
    let mut a=population::create_actor_revision(kind,20.,10.,10.,false,37.).unwrap();
    a.conditions.clear();a.active.clear();a.state="wandering".into();
    r.player.x=5.;r.player.y=5.;r.actors_at_mut(1.).push(a);20.
}
#[test]
fn slip_uses_one_serialized_coin_only_on_valid_departure() {
    let base=room(37.);let mut slips=0;
    for seed in 1..=3000 {
        let mut r=base.clone();r.rng=corpse_core::rng::seed_hash(&format!("ice-{seed}")) as f64;cover(&mut r,"ice",10*W+10);
        let mut expected=r.clone();let roll=corpse_core::rng::random(&mut expected);
        let id=r.player.id;assert!(engine::move_actor(&mut r,id,1.,0.,true).unwrap());
        assert_eq!(r.rng,expected.rng);
        assert_eq!(r.player.x,if roll<1./3.{10.}else{11.});
        if r.player.x==10.{slips+=1;}
    }
    assert!((900..=1100).contains(&slips),"{slips}");
    let mut r=base.clone();cover(&mut r,"ice",10*W+10);r.floor_mut().unwrap().tiles[10*W+11]="wall".into();
    let rng=r.rng;let id=r.player.id;assert!(!engine::move_actor(&mut r,id,1.,0.,true).unwrap());assert_eq!(rng,r.rng);
    // Entering ice from ordinary ground does not roll.
    let mut r=base;cover(&mut r,"ice",10*W+11);let rng=r.rng;let id=r.player.id;
    assert!(engine::move_actor(&mut r,id,1.,0.,true).unwrap());assert_eq!(rng,r.rng);assert_eq!(r.player.x,11.);
}
#[test]
fn ice_exempts_flight_ice_species_forced_displacement_and_legacy() {
    for kind in ["wendigo","yeti"] {
        let mut r=room(37.);let id=actor(&mut r,kind);cover(&mut r,"ice",10*W+10);let rng=r.rng;
        assert!(engine::move_actor(&mut r,id,1.,0.,true).unwrap());assert_eq!(engine::actor(&r,id).unwrap().x,11.);assert_eq!(rng,r.rng);
    }
    for mode in ["flying","forcedMove","legacy"] {
        let mut r=room(if mode=="legacy"{36.}else{37.});cover(&mut r,"ice",10*W+10);
        if mode!="legacy"{r.player.conditions.insert(mode.into(),1.);}
        let rng=r.rng;let id=r.player.id;assert!(engine::move_actor(&mut r,id,1.,0.,true).unwrap());assert_eq!(r.player.x,11.);assert_eq!(rng,r.rng);
    }
}
#[test]
fn npc_slips_and_failed_or_successful_moves_replay_identically() {
    let mut r=room(37.);let id=actor(&mut r,"rat");cover(&mut r,"ice",10*W+10);
    for seed in 1..50 {let mut a=r.clone();a.rng=seed as f64;let mut b=a.clone();
        engine::move_actor(&mut a,id,1.,0.,true).unwrap();engine::move_actor(&mut b,id,1.,0.,true).unwrap();assert_eq!(a,b);
    }
}
#[test]
fn direct_fire_and_burning_body_melt_snow_without_burning_underlying_fuel() {
    let mut r=room(37.);let at=10*W+11;cover(&mut r,"snow",at);r.floor_mut().unwrap().tiles[at]="grass".into();
    environment::ignite(&mut r,11.,10.,None,None).unwrap();
    let f=r.floor().unwrap();assert!(!terrain_runtime::snow_at(f,at));assert!(f.puddles.as_ref().unwrap()[at]);assert_eq!(f.tiles[at],"grass");assert_eq!(f.environment.as_ref().unwrap().fire[at],0.);
    cover(&mut r,"snow",at);r.player.conditions.insert("burning".into(),7.);r.player.conditions.insert("burningOwner".into(),40.);
    let id=r.player.id;assert!(engine::move_actor(&mut r,id,1.,0.,true).unwrap());
    assert_eq!(engine::condition(&r.player,"burning"),0.);assert!(!r.player.conditions.contains_key("burningOwner"));assert!(!terrain_runtime::snow_at(r.floor().unwrap(),at));
}
#[test]
fn flying_body_does_not_touch_snow_and_everburn_is_not_removed() {
    let mut r=room(37.);cover(&mut r,"snow",10*W+10);r.player.conditions.insert("burning".into(),7.);r.player.conditions.insert("flying".into(),2.);let id=r.player.id;
    assert!(!terrain_runtime::snow_contact(&mut r,id));assert!(terrain_runtime::snow_at(r.floor().unwrap(),10*W+10));
    r.player.conditions.shift_remove("flying");r.player.abilities.push("enemy:beast:everburn".into());
    assert!(terrain_runtime::snow_contact(&mut r,id));assert!(engine::condition(&r.player,"burning")>0.);
}
#[test]
fn hellfire_persists_ignites_entrants_and_spreads_ordinary_fire() {
    let mut r=room(37.);let at=10*W+11;cover(&mut r,"hellfire",at);environment::initialize(r.floor_mut().unwrap()).unwrap();
    let id=r.player.id;assert!(engine::move_actor(&mut r,id,1.,0.,true).unwrap());assert!(engine::condition(&r.player,"burning")>0.);
    r.player.abilities.push("enemy:beast:fire-immunity".into());r.player.x=5.;r.player.y=5.;r.player.conditions.clear();
    r.floor_mut().unwrap().tiles[at+1]="grass".into();let mut spread=false;
    for _ in 0..80 {environment::tick_environment(&mut r).unwrap();let f=r.floor().unwrap();let e=f.environment.as_ref().unwrap();assert!(e.fire[at]>0.);assert_eq!(e.fire_kind.as_ref().unwrap()[at].as_deref(),Some("brazier"));
        if e.fire[at+1]>0.{spread=true;assert!(!terrain_runtime::hellfire_at(f,at+1));assert_ne!(e.fire_kind.as_ref().unwrap()[at+1].as_deref(),Some("brazier"));}
    }
    assert!(spread);
}
#[test]
fn snow_quenches_a_spreading_fire_contact_and_records_puddle() {
    let mut r=room(37.);let at=10*W+11;cover(&mut r,"hellfire",at);cover(&mut r,"snow",at+1);r.floor_mut().unwrap().tiles[at+1]="grass".into();
    environment::initialize(r.floor_mut().unwrap()).unwrap();environment::tick_environment(&mut r).unwrap();
    let f=r.floor().unwrap();assert!(!terrain_runtime::snow_at(f,at+1));assert!(f.puddles.as_ref().unwrap()[at+1]);assert_eq!(f.environment.as_ref().unwrap().fire[at+1],0.);assert_eq!(f.tiles[at+1],"grass");
}
#[test]
fn occupied_destination_does_not_attempt_ice_footing() {
    let mut r=room(37.);cover(&mut r,"ice",10*W+10);
    let a=population::create_actor_revision("rat",20.,11.,10.,false,37.).unwrap();r.actors_at_mut(1.).push(a);
    let id=r.player.id;let before=r.rng;assert!(!engine::move_actor(&mut r,id,1.,0.,true).unwrap());assert_eq!(r.rng,before);
}
#[test]
fn snapshot_and_serialization_preserve_terrain_and_rng() {
    let mut r=room(37.);cover(&mut r,"ice",10*W+10);cover(&mut r,"snow",10*W+11);cover(&mut r,"hellfire",12*W+12);
    let snap=corpse_core::temporal::Snapshot::capture(&r);
    let mut expected=r.clone();let id=r.player.id;engine::move_actor(&mut expected,id,1.,0.,true).unwrap();
    let mut loaded:Run=serde_json::from_slice(&serde_json::to_vec(&r).unwrap()).unwrap();engine::move_actor(&mut loaded,id,1.,0.,true).unwrap();assert_eq!(loaded,expected);
    terrain_runtime::melt_snow(r.floor_mut().unwrap(),10*W+11);snap.restore(&mut r).unwrap();engine::move_actor(&mut r,id,1.,0.,true).unwrap();assert_eq!(r,expected);
}
#[test]
fn hellfire_cannot_be_quenched_by_water_or_fire_state_reset() {
    let mut r=room(37.);let at=10*W+11;cover(&mut r,"hellfire",at);r.floor_mut().unwrap().tiles[at]="water".into();
    for _ in 0..5 {let e=r.floor_mut().unwrap().environment.as_mut().unwrap();e.fire[at]=0.;e.fire_kind.as_mut().unwrap()[at]=None;
        environment::tick_environment(&mut r).unwrap();assert!(r.floor().unwrap().environment.as_ref().unwrap().fire[at]>0.);}
}
#[test]
fn offscreen_burning_creature_melts_and_quenches_snow() {
    let mut r=room(37.);let mut second=r.floor().unwrap().clone();second.depth=2.;second.dirty=true;second.snow=vec![false;W*H].into();second.snow.as_mut().unwrap()[10*W+10]=true;
    r.floors.push(Arc::new(second));let mut a=population::create_actor_revision("rat",20.,10.,10.,false,37.).unwrap();a.state="sleeping".into();a.conditions.insert("burning".into(),7.);
    r.actors.insert("2".into(),Arc::new(vec![a]));r.ground.insert("2".into(),Arc::new(vec![]));r.tick=1.;
    corpse_core::offscreen::catch_up_floor(&mut r,2.).unwrap();
    let f=&r.floors[1];assert!(!terrain_runtime::snow_at(f,10*W+10));assert!(f.puddles.as_ref().unwrap()[10*W+10]);assert_eq!(engine::condition(&r.actors_at(2.)[0],"burning"),0.);
}
#[test]
fn haste_move_contact_quenches_before_normal_turn() {
    let mut r=room(37.);cover(&mut r,"snow",10*W+11);r.player.conditions.insert("burning".into(),7.);r.player.conditions.insert("hasted".into(),5.);
    let tick=r.tick;let outcome=engine::dispatch(&mut r,&corpse_core::Command::Move{dx:1.,dy:0.});
    assert!(outcome.is_ok(),"{outcome:?}");assert_eq!(r.tick,tick);assert_eq!(r.phase.as_ref().map(String::as_str),Some("normal"));assert_eq!(r.player.x,11.);assert_eq!(engine::condition(&r.player,"burning"),0.);
}
fn speed_strip(r:&mut Run,corridor:bool) {
    use corpse_core::non_euclidean_generation::{NonEuclideanGroup as G,NonEuclideanPocket as P};
    let f=r.floor_mut().unwrap();
    if corridor {f.tiles.fill("wall".into());for x in 8..=19 {f.tiles[10*W+x]="floor".into();}}
    f.speed_up=vec![P {cells:(10..18).map(|x|10*W+x).collect(),groups:(10..18).step_by(2).map(|x|G {sources:vec![10*W+x],destination:10*W+x+1}).collect()}].into();
}
#[test]
fn ordered_strip_every_interior_start_can_walk_both_directions() {
    for start in 10..18 {for dx in [-1.,1.] {
        let mut r=room(37.);speed_strip(&mut r,true);r.player.x=start as f64;r.player.y=10.;let id=r.player.id;let rng=r.rng;let target=if dx<0.{9.}else{18.};
        for _ in 0..8 {if r.player.x==target{break;}let before=r.player.x;assert!(engine::move_actor(&mut r,id,dx,0.,true).unwrap());assert_ne!(before,r.player.x,"stalled at {start}, {dx}");}
        assert_eq!(r.player.x,target,"{start} {dx}");assert_eq!(r.rng,rng);
    }}
}
#[test]
fn ordered_strip_forward_reverse_side_entry_and_two_cell_progress() {
    let mut r=room(37.);speed_strip(&mut r,false);r.player.x=9.;let id=r.player.id;
    for x in [11.,13.,15.,17.,18.] {engine::move_actor(&mut r,id,1.,0.,true).unwrap();assert_eq!(r.player.x,x);}
    for x in [16.,14.,12.,10.,9.] {engine::move_actor(&mut r,id,-1.,0.,true).unwrap();assert_eq!(r.player.x,x);}
    engine::move_actor(&mut r,id,1.,0.,true).unwrap();assert_eq!(r.player.x,11.);
    engine::move_actor(&mut r,id,-1.,0.,true).unwrap();assert_eq!(r.player.x,10.);
    engine::move_actor(&mut r,id,-1.,0.,true).unwrap();assert_eq!(r.player.x,9.);
    r.player.x=12.;r.player.y=9.;engine::move_actor(&mut r,id,0.,1.,true).unwrap();assert_eq!((r.player.x,r.player.y),(13.,10.));
    engine::move_actor(&mut r,id,-1.,0.,true).unwrap();assert_eq!(r.player.x,12.);
    engine::move_actor(&mut r,id,-1.,0.,true).unwrap();assert_eq!(r.player.x,10.);
}
#[test]
fn ordered_pathfields_execute_exact_predicted_cost_for_npc_from_every_cell() {
    use corpse_core::{movement::{self,Point},portals};
    for start in 10..18 {for goal in [9,18] {for stochastic in [false,true] {
        let mut r=room(37.);speed_strip(&mut r,true);let id=actor(&mut r,"rat");engine::actor_mut(&mut r,id).unwrap().x=start as f64;
        // Add an unreachable random group to exercise the stochastic-aware
        // next_step branch without altering the straight corridor's route.
        if stochastic {use corpse_core::non_euclidean_generation::{NonEuclideanGroup as G,NonEuclideanPocket as P};r.floor_mut().unwrap().non_euclidean=vec![P{cells:vec![3*W+3,3*W+4,3*W+5],groups:vec![G{sources:vec![3*W+3,3*W+4],destination:3*W+5}]}].into();}
        let f=r.floor().unwrap();let field=movement::distance_field_ordered(&f.tiles,W,Point{x:goal,y:10},false,false,None,None,&portals::links(f),&portals::ordered_pairs(f));
        let expected=field.steps[10*W+start];assert!(expected>0);let mut count=0;
        while engine::actor(&r,id).unwrap().x != goal as f64 && count<20 {
            let a=engine::actor(&r,id).unwrap();let p=Point{x:a.x as i32,y:a.y as i32};let d=movement::next_step(&field,&r.floor().unwrap().tiles,p).expect("pathstep");
            let options=portals::step_options(&r,a,a.x+d.x as f64,a.y+d.y as f64);assert_eq!(options.len(),1);
            assert!(engine::move_actor(&mut r,id,d.x as f64,d.y as f64,true).unwrap());let a=engine::actor(&r,id).unwrap();assert_eq!((a.x,a.y),options[0]);count+=1;
        }
        assert_eq!(count,expected,"start{start} goal{goal} stochastic{stochastic}");
    }}}
}
#[test]
fn ordered_exception_does_not_change_random_links_or_projectile_transfer() {
    use corpse_core::{movement::Point,portals,non_euclidean_generation::{NonEuclideanGroup as G,NonEuclideanPocket as P}};
    let mut r=room(37.);speed_strip(&mut r,false);r.player.x=11.;let at=10*W+10;
    assert_eq!(portals::step_options(&r,&r.player,10.,10.),vec![(10.,10.)]);
    assert_eq!(portals::exits(r.floor().unwrap(),at),vec![at+1]);
    let ray=portals::directed_ray(r.floor().unwrap(),Point{x:11,y:10},Point{x:9,y:10},|_|true);assert!(ray.windows(2).any(|w|w==[Point{x:10,y:10},Point{x:11,y:10}]));
    r.floor_mut().unwrap().speed_up=corpse_core::Field::Missing;r.floor_mut().unwrap().non_euclidean=vec![P{cells:vec![at,at+1],groups:vec![G{sources:vec![at],destination:at+1}]}].into();
    assert_eq!(portals::step_options(&r,&r.player,10.,10.),vec![(11.,10.)]);
}
#[test]
fn ordered_local_step_preserves_shared_enemy_occupancy() {
    let mut r=room(37.);speed_strip(&mut r,false);r.player.x=11.;
    let a=population::create_actor_revision("rat",20.,10.,10.,false,37.).unwrap();r.actors_at_mut(1.).push(a);let id=r.player.id;
    assert!(!engine::move_actor(&mut r,id,-1.,0.,true).unwrap());assert_eq!(r.player.x,11.);assert_eq!(engine::actor(&r,20.).unwrap().x,10.);
}
