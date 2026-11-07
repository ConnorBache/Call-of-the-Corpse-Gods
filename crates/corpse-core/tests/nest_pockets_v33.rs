use corpse_core::{Command,Field,Floor,Run,engine,population,startup,pocket_biomes as b,royal_honey as honey};
use std::{sync::Arc,collections::{HashSet,VecDeque}};
fn room()->Run {
    let mut r=startup::new_run_shell_revision("0-5-11:nest33","Unblessed",false,33.).unwrap();
    r.player.x=2.;r.player.y=2.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.ability_order=Field::Missing;
    let w=79;let h=29;let mut tiles=vec!["wall".into();w*h];for y in 1..h-1 {for x in 1..w-1 {tiles[y*w+x]="floor".into();}}
    tiles[2*w+2]="up".into();tiles[25*w+75]="down".into();
    let mut f=Floor{width:(w as f64).into(),height:(h as f64).into(),depth:1.,domain:"Dungeon".into(),tiles,lights:vec![100.;w*h],seen:vec![true;w*h],..Default::default()};
    corpse_core::environment::initialize(&mut f).unwrap();r.floors=vec![Arc::new(f)];
    r.actors.clear();r.actors.insert("1".into(),Arc::new(vec![]));r.ground.clear();r.ground.insert("1".into(),Arc::new(vec![]));r.next_id=50.;r.tick=3.;r.generated_ability_grants=Vec::new().into();r
}
fn queen()->Run {let mut r=room();let mut q=population::create_actor_revision("saidribee-queen",4.,5.,5.,false,33.).unwrap();q.state="hunting".into();r.actors_at_mut(1.).push(q);r}
fn cast(r:&mut Run)->bool {let target=r.player.id;corpse_core::saidribee_queen::summon(r,4.,target).unwrap()}
#[test]
fn queen_three_casts_are_energy_gated_world_clocked_and_heartless() {
    let mut r=queen();engine::actor_mut(&mut r,4.).unwrap().heartless=true.into();
    for tick in [3.,13.,23.] {r.tick=tick;assert!(cast(&mut r));let before=r.clone();assert!(!cast(&mut r));assert_eq!(r,before);r.tick=tick+9.;assert!(!cast(&mut r));}
    assert_eq!(r.actors_at(1.).len(),4);assert_eq!(engine::actor(&r,4.).unwrap().energy,55.);
    assert!(r.actors_at(1.).iter().skip(1).all(|a|a.heartless.value_or(false)&&["short-saidri","saidri-guardian"].contains(&a.creature_kind.as_ref().unwrap().as_str())));
    engine::actor_mut(&mut r,4.).unwrap().energy=14.;r.tick=100.;let before=r.clone();assert!(!cast(&mut r));assert_eq!(r,before);
}
#[test]
fn queen_failed_blocked_silenced_or_legacy_casts_do_not_mutate() {
    for failure in ["space","silence","paralyzed","legacy"] {let mut r=queen();match failure {
        "space"=>for y in 4..=6 {for x in 4..=6 {if(x,y)!=(5,5) {r.floor_mut().unwrap().tiles[y*79+x]="water".into();}}},
        "silence"=>{engine::actor_mut(&mut r,4.).unwrap().conditions.insert("silenced".into(),2.);},
        "paralyzed"=>{engine::actor_mut(&mut r,4.).unwrap().conditions.insert("paralyzed".into(),2.);},
        _=>r.simulation_revision=32.0.into(),
    }let before=r.clone();assert!(!cast(&mut r),"{failure}");assert_eq!(r,before,"{failure}");}
    let mut kinds=HashSet::new();for seed in 1..=64 {let mut r=queen();r.rng=seed as f64;assert!(cast(&mut r));kinds.insert(r.actors_at(1.)[1].creature_kind.clone().into_option().unwrap());}assert_eq!(kinds.len(),2);
}
#[test]
fn honey_floor_inventory_effective_max_full_and_blocked_actions() {
    for floor in [false,true] {let mut r=room();r.player.stats.defense=5.;r.player.stats.max_energy=145.;r.player.hp=1.;r.player.energy=2.;
        if floor {let (x,y)=(r.player.x,r.player.y);r.ground_at_mut(1.).push(corpse_core::GroundItem{x,y,item:honey::item(44.),..Default::default()});} else {r.inventory.push(honey::item(44.));}
        assert_eq!(corpse_core::inventory::command(&mut r,&Command::Use{id:44.}).unwrap(),Some(true));
        assert_eq!(r.player.hp,130.);assert_eq!(r.player.energy,145.);assert!(!r.inventory.iter().any(|i|i.id==44.));assert!(r.ground["1"].is_empty());
        let before=r.clone();assert_eq!(corpse_core::inventory::command(&mut r,&Command::Use{id:44.}).unwrap(),Some(false));assert_eq!(r,before);
    }
    let mut r=room();r.inventory.push(honey::item(44.));r.player.hp=r.player.max_hp;r.player.energy=100.;assert!(corpse_core::inventory::command(&mut r,&Command::Use{id:44.}).unwrap().unwrap());assert!(r.inventory.iter().all(|i|i.id!=44.));assert_eq!(r.player.hp,r.player.max_hp);assert_eq!(r.player.energy,100.);
    let mut r=room();r.inventory.push(honey::item(44.));r.player.hp=9.;r.player.energy=7.;r.player.conditions.insert("paralyzed".into(),2.);engine::dispatch(&mut r,&Command::Use{id:44.}).unwrap();assert!(r.inventory.iter().any(|i|i.id==44.));assert_eq!((r.player.hp,r.player.energy),(9.,7.));
    r.player.conditions.clear();r.player.dead=true.into();r.player.hp=0.;assert!(!honey::eat(&mut r));assert_eq!(r.player.hp,0.);
}
#[test]
fn honey_save_revert_future_and_replay_preserve_exact_once_consumption() {
    let mut r=room();r.inventory.push(honey::item(44.));r.player.hp=9.;r.player.energy=7.;r.player.conditions.insert("revertCharges".into(),3.);
    let raw=corpse_core::storage::serialize(&r).unwrap();r=corpse_core::storage::deserialize(&raw).unwrap();
    let opening=corpse_core::temporal::Snapshot::capture(&r);let mut history=corpse_core::temporal::History::default();history.record_completed(opening);
    corpse_core::inventory::command(&mut r,&Command::Use{id:44.}).unwrap();r.tick+=1.;history.closing=Some(corpse_core::temporal::Snapshot::capture(&r));assert!(corpse_core::temporal::revert(&mut r,&mut history).unwrap());assert!(r.inventory.iter().any(honey::is_honey));assert_eq!((r.player.hp,r.player.energy),(9.,5.)); // Existing Revert costs two energy.
    corpse_core::temporal::remember_future(&mut r,&history).unwrap();corpse_core::inventory::command(&mut r,&Command::Use{id:44.}).unwrap();assert!(corpse_core::temporal::restore_future(&mut r,&mut history).unwrap());assert!(r.inventory.iter().any(honey::is_honey));
    let initial=r.clone();let mut e=corpse_core::temporal::Engine::new(r).unwrap();e.dispatch(&Command::Use{id:44.}).unwrap();let record=corpse_core::replay::ReplayRecord::from_run(&e.run);let mut replay=corpse_core::replay::ReplaySession::from_initial(initial,record).unwrap();while replay.step().unwrap(){}assert_eq!(replay.engine.run.inventory,e.run.inventory);assert_eq!(replay.engine.run.player,e.run.player);
}
fn reachable(f:&Floor,start:usize,goal:usize,subset:Option<&HashSet<usize>>)->bool {
    let w=f.width();let mut seen=HashSet::from([start]);let mut q=VecDeque::from([start]);while let Some(i)=q.pop_front(){if i==goal{return true;}for (dx,dy) in [(0,-1),(1,0),(0,1),(-1,0)]{let(x,y)=((i%w)as i32+dx,(i/w)as i32+dy);if x<0||y<0||x>=w as i32||y>=f.height()as i32{continue;}let j=y as usize*w+x as usize;if corpse_core::generation::terrain_passable(&f.tiles[j])&&subset.is_none_or(|s|s.contains(&j))&&seen.insert(j){q.push_back(j);}}}false
}
#[test]
fn seeded_nests_have_connected_rooms_stairs_and_guaranteed_reward() {
    for seed in [1,42,123456] {let depth=b::nest_depth(seed);assert!((28..=34).contains(&depth)&&depth!=30);
        let mut f=corpse_core::generation::generate_floor_revision(depth,seed,"Dungeon",false,79,None,33.).unwrap();let same=corpse_core::generation::generate_floor_revision(depth,seed,"Dungeon",false,79,None,33.).unwrap();assert_eq!(f,same);
        let nest=b::nest(&f).unwrap().clone();assert_eq!(nest.rooms.len(),3);assert!(b::validate(&f));let cells:HashSet<_>=nest.cells.iter().copied().collect();let up=f.tiles.iter().position(|t|t=="up").unwrap();let down=f.tiles.iter().position(|t|t=="down").unwrap();assert!(reachable(&f,up,down,None));assert!(reachable(&f,up,nest.honey_cell.unwrap(),None));for &at in &nest.cells {assert!(reachable(&f,nest.queen_cell.unwrap(),at,Some(&cells)));}assert!(!cells.contains(&up)&&!cells.contains(&down));
        let tiles=f.tiles.clone();let mut actors=vec![];let mut ground=vec![];let mut id=100.;b::populate(&mut f,&mut actors,&mut ground,&mut id).unwrap();assert_eq!(actors.len(),7);assert_eq!(ground.len(),1);assert!(honey::is_honey(&ground[0].item));assert_eq!(f.tiles,tiles);
        assert_eq!(actors.iter().filter(|a|a.creature_kind.as_ref().is_some_and(|k|k=="saidribee-queen")).count(),1);
    }
}
#[test]
fn pocket_depth_theme_habitat_and_legacy_guards() {
    let mut count=[0usize;2];for (group,depth) in [8,32].into_iter().enumerate(){for seed in 1..=50 {let mut r=room();let f=r.floor_mut().unwrap();f.depth=depth as f64;
        // Plain three-room connected scaffold isolates occurrence from expensive geology.
        f.dungeon_rooms=vec![(8,8),(22,8),(36,8),(50,8)].into_iter().enumerate().map(|(id,(x,y))|corpse_core::RoomMetadata{id:id as f64,cells:(y..y+6).flat_map(|yy|(x..x+8).map(move|xx|(yy*79+xx)as f64)).collect(),center:corpse_core::DungeonPoint{x:x as f64+3.,y:y as f64+3.},..Default::default()}).collect::<Vec<_>>().into();
        b::decorate(f,seed).unwrap();count[group]+=f.pocket_biomes.as_ref().map_or(0,Vec::len);
        if let Some(bs)=f.pocket_biomes.as_ref(){for biome in bs {if biome.theme=="permafrost" {assert!((17..=36).contains(&depth));assert!(biome.cells.iter().all(|i|f.snow.as_ref().unwrap()[*i]));}if biome.theme=="flower-grove" {assert!(!biome.flowers.is_empty());}}}
    }}assert!(count[1]>count[0],"deeper local habitats become more common: {count:?}");
    let mut r=room();r.floor_mut().unwrap().godbeast=true;b::decorate(r.floor_mut().unwrap(),42).unwrap();assert!(r.floor().unwrap().pocket_biomes.is_missing());
    let f=corpse_core::generation::generate_floor_revision(28,42,"Dungeon",false,79,None,32.).unwrap();assert!(f.pocket_biomes.is_missing());
}
#[test]
fn queen_cooldown_children_save_and_replay_are_deterministic() {
    let mut r=queen();r.player.conditions.insert("revertCharges".into(),3.);assert!(cast(&mut r));
    let expected=r.actors_at(1.).to_vec();let rng=r.rng;let raw=corpse_core::storage::serialize(&r).unwrap();r=corpse_core::storage::deserialize(&raw).unwrap();assert_eq!(r.actors_at(1.),expected);assert_eq!(r.rng,rng);
    let opening=corpse_core::temporal::Snapshot::capture(&r);let mut history=corpse_core::temporal::History::default();history.record_completed(opening);r.tick=13.;assert!(cast(&mut r));history.closing=Some(corpse_core::temporal::Snapshot::capture(&r));assert!(corpse_core::temporal::revert(&mut r,&mut history).unwrap());assert_eq!(r.actors_at(1.),expected);assert_eq!(r.rng,rng);
    corpse_core::temporal::remember_future(&mut r,&history).unwrap();r.tick=13.;assert!(cast(&mut r));assert!(corpse_core::temporal::restore_future(&mut r,&mut history).unwrap());assert_eq!(r.actors_at(1.),expected);
    let initial=queen();let mut e=corpse_core::temporal::Engine::new(initial.clone()).unwrap();for _ in 0..3 {e.dispatch(&Command::Wait).unwrap();}let record=corpse_core::replay::ReplayRecord::from_run(&e.run);let mut replay=corpse_core::replay::ReplaySession::from_initial(initial,record).unwrap();while replay.step().unwrap(){}assert_eq!(replay.engine.run.actors,e.run.actors);assert_eq!(replay.engine.run.rng,e.run.rng);
}
#[test]
fn habitat_relocation_preserves_whole_hordes_and_machine_actors() {
    let mut r=room();let f=r.floor_mut().unwrap();f.depth=27.;let cells:Vec<_>=(8..12).flat_map(|y|(10..15).map(move|x|y*79+x)).collect();
    f.pocket_biomes=vec![b::PocketBiome{theme:"permafrost".into(),cells:cells.clone(),rooms:vec![1],..Default::default()}].into();
    let mut actors:Vec<_>=(0..4).map(|i|population::create_actor_revision("yeti",10.+i as f64,30.+i as f64,15.,false,33.).unwrap()).collect();
    f.encounters=vec![corpse_core::EncounterRecord{kind:"horde".to_string().into(),actor_ids:vec![10.,11.,12.],..Default::default()},corpse_core::EncounterRecord{kind:"machine".to_string().into(),actor_ids:vec![13.],..Default::default()}].into();
    let machine=actors[3].clone();b::populate(f,&mut actors,&mut vec![],&mut 100.).unwrap();assert_eq!(actors[3],machine);assert!(actors[..3].iter().all(|a|cells.contains(&(a.y as usize*79+a.x as usize))));
}
#[test]
fn seeded_nest_atomic_startup_reserves_populations_and_reward() {
    let mut r=startup::new_run_shell_revision("0-5-11:nest-startup","Unblessed",false,33.).unwrap();
    let seed=corpse_core::rng::seed_hash(&r.seed);let d=b::nest_depth(seed);
    // Prior-floor entries are only needed for retained-loot allocation; the
    // isolated target uses the actual finalized generator and full spawn path.
    let mut f=corpse_core::generation::generate_floor_full_revision(d,seed,None,33.).unwrap();
    f.depth=d as f64;r.floors=vec![Arc::new(f)];r.depth=d as f64;
    startup::spawn_floor(&mut r,d).unwrap();let f=r.floor().unwrap();let nest=b::nest(f).unwrap();
    assert_eq!(r.actors_at(d as f64).iter().filter(|a|a.conditions.contains_key("nestResident")).count(),7);
    let rewards:Vec<_>=r.ground[&d.to_string()].iter().filter(|g|honey::is_honey(&g.item)).collect();assert_eq!(rewards.len(),1);assert_eq!(rewards[0].y as usize*f.width()+rewards[0].x as usize,nest.honey_cell.unwrap());
    assert!(r.actors_at(d as f64).iter().filter(|a|!a.conditions.contains_key("nestResident")).all(|a|!nest.cells.contains(&(a.y as usize*f.width()+a.x as usize))));
}
#[test]
fn nest_guarantee_holds_across_seeded_natural_domains() {
    let mut domains=HashSet::new();
    for seed in 1..=12 {let d=b::nest_depth(seed);let f=corpse_core::generation::generate_floor_full_revision(d,seed,None,33.).unwrap_or_else(|e|panic!("seed{seed} depth{d}: {e}"));domains.insert(f.domain.clone());assert!(b::nest(&f).is_some(),"seed{seed} depth{d}");assert!(b::validate(&f));}
    assert!(domains.len()>=3,"sample must exercise actual natural domain variation: {domains:?}");
}
#[test]
fn rare_all_water_window_seed155_gets_bounded_platform_nest_without_domain_override() {
    for seed in [6,155] {
        let depth=b::nest_depth(seed);let plan=corpse_core::generation::domain_plan_revision(seed,33.);
        if seed==6 {assert_ne!(plan[(depth as usize-1)/5].domain,"Archipelago");}
        else {assert_eq!(plan[5].domain,"Archipelago");assert_eq!(plan[6].domain,"Archipelago");}
        let f=corpse_core::generation::generate_floor_full_revision(depth,seed,None,33.).unwrap();let nest=b::nest(&f).unwrap();
        assert_eq!(f.domain,plan[(depth as usize-1)/5].domain);assert_eq!(nest.rooms.len(),3);
        let bridges=f.generation_bridges.as_ref().into_iter().flatten().filter(|i|nest.cells.contains(&(**i as usize))).count();assert!(bridges<=160,"seed{seed}: {bridges}");
        let up=f.tiles.iter().position(|t|t=="up").unwrap();let down=f.tiles.iter().position(|t|t=="down").unwrap();assert!(reachable(&f,up,down,None));assert!(reachable(&f,up,nest.honey_cell.unwrap(),None));
        assert!(nest.cells.iter().all(|i|matches!(f.tiles[*i].as_str(),"floor"|"grass")));assert!(corpse_core::generation::validate_floor_generation(&f));
    }
}
