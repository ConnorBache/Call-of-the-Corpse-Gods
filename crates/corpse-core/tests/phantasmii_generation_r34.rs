use corpse_core::{Field, Floor, GroundItem, Run, dreamwalking, generation, inventory, phantasmii_generation as p, population, startup, storage};
use std::collections::BTreeSet;

fn floor(depth: u32, wet: bool, altar: bool) -> Floor {
    let mut f = Floor { depth: depth as f64, domain: "Dungeon".into(), tiles: vec![if wet { "water".into() } else { "floor".into() }; 2291], ..Default::default() };
    for at in 0..f.tiles.len() { if at % 79 == 0 || at % 79 == 78 || at / 79 == 0 || at / 79 == 28 { f.tiles[at] = "wall".into(); } }
    f.tiles[80] = "up".into(); f.tiles[27*79+77] = "down".into();
    if altar { f.godbeast_event = generation::reserve_godbeast_room(&mut f.tiles, "Phantasmii", &mut || 0.5, &[], &[], &Default::default()).unwrap().into(); f.godbeast = true; }
    f
}
fn room() -> Run {
    let mut r: Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.floors.truncate(1); r.actors.retain(|k,_| k == "1"); r.ground.retain(|k,_| k == "1");
    r.simulation_revision = 34.0.into(); r.inventory.clear(); r.actors_at_mut(1.).clear(); r.ground_at_mut(1.).clear();
    r.player.x = 8.; r.player.y = 8.; r.next_id = 1000.; r.offers.clear(); r.phase = Field::Missing;
    r
}
#[test]
fn revision34_bag_has_seven_distinct_designs_at_unchanged_seven_milestones() {
    for seed in 0..128 {
        let old = generation::domain_plan_revision(seed, 33.0);
        let new = generation::domain_plan_revision(seed, 34.0);
        assert_eq!(new.iter().map(|e|e.godbeast_depth).collect::<Vec<_>>(), vec![5,10,15,20,25,30,35,-1]);
        assert_eq!(new.iter().map(|e|&e.domain).collect::<Vec<_>>(), old.iter().map(|e|&e.domain).collect::<Vec<_>>());
        assert_eq!(new[..7].iter().map(|e|e.godbeast_kind.as_str()).collect::<BTreeSet<_>>(), ["Ahvamitt","Borge","Epidite","Mercuria","Phantasmii","Veridian","Zillennia"].into_iter().collect());
        assert!(new[..7].windows(2).all(|a|a[0].godbeast_kind != a[1].godbeast_kind));
        assert!(!old.iter().any(|e|e.godbeast_kind == "Phantasmii"));
    }
}
#[test]
fn persistent_four_on_all_forty_floors_and_three_quest_items_only_at_altar() {
    for depth in 1..=40 {
        for wet in [false, true] {
            let altar = depth == 5;
            let mut f = floor(depth, wet, altar);
            let original = f.tiles.clone(); let mut actors = Vec::new(); let mut ground = Vec::new(); let mut id = 100.;
            p::populate("dream-seed", &mut f, &mut actors, &mut ground, &mut id).unwrap();
            assert_eq!(f.tiles, original); assert_eq!(actors.len(), 4);
            assert!(actors.iter().all(|a|dreamwalking::is_nightmare(a) && a.heartless.value_or(false)));
            assert_eq!(actors.iter().map(|a|(a.x as usize,a.y as usize)).collect::<BTreeSet<_>>().len(),4);
            assert_eq!(ground.iter().filter(|g|p::is_shard(&g.item)).count(), if altar {3} else {0});
            assert_eq!(ground.iter().filter(|g|p::is_dagger(&g.item)).count(), if altar {1} else {0});
            if altar {
                let at=f.godbeast_event.as_ref().unwrap().altar.as_ref().unwrap();
                let dagger=ground.iter().find(|g|p::is_dagger(&g.item)).unwrap();
                assert_eq!((dagger.x-at.x).abs().max((dagger.y-at.y).abs()),1.);
                assert_eq!(dagger.item.effect.as_ref().unwrap(),p::DAGGER_DESCRIPTION);
                assert_eq!(f.tiles[at.y as usize*79+at.x as usize],"floor");
            }
            let snapshot=(actors.clone(),ground.clone(),id);
            assert!(p::populate("dream-seed", &mut f, &mut actors, &mut ground, &mut id).is_err());
            assert_eq!((actors,ground,id),snapshot);
        }
    }
}
#[test]
fn eight_fixed_next_band_body_profiles_have_no_inherited_combat_traits() {
    for band in 0..8 {
        let a=p::nightmare(band*5+1, 100., 8., 9.);
        let baseline=population::create_actor_revision(p::NIGHTMARE_BASELINES[band as usize], 100., 8., 9., false, 33.).unwrap();
        assert_eq!(a.stats, baseline.stats); assert_eq!(a.max_hp, baseline.max_hp); assert_eq!(a.hp,a.max_hp);
        assert_eq!(a.conditions["nightmareTier"],(band+1) as f64);
        assert_eq!(a.abilities,vec!["enemy:beast:flight","enemy:beast:no-revival"]);
        assert!(a.monster_state.is_missing()); assert!(a.heartless.value_or(false));
        assert_eq!(p::nightmare(band*5+5,101.,8.,9.).name,a.name);
    }
    assert_eq!(p::nightmare(16,0.,0.,0.).name,"Nightmare");
    assert_eq!(p::nightmare(40,0.,0.,0.).name,"Terrible Nightmare");
}
#[test]
fn shard_pickup_is_dream_only_one_real_stack_and_unlocks_without_consuming() {
    let mut r=room();
    let ordinary=corpse_core::equipment::make_equipment("Shortsword",100.).unwrap();
    for item in [ordinary,p::shard(101.),p::shard(102.),p::shard(103.)] {
        r.ground_at_mut(1.).push(GroundItem{x:8.,y:8.,item,..Default::default()});
    }
    assert_eq!(inventory::collect_underfoot(&mut r,20.).unwrap(),1);
    assert_eq!(r.ground["1"].len(),3);
    let body_id=104.; let mut body=r.player.clone();body.id=body_id;body.conditions.insert("phantasmiiBody".into(),1.);r.actors_at_mut(1.).push(body);
    r.dreamwalking=dreamwalking::State{active:true,depth:1.,body_id:Some(body_id),anchor_x:8.,anchor_y:8.,..Default::default()}.into();
    assert_eq!(inventory::collect_underfoot(&mut r,2.).unwrap(),3);
    assert_eq!(r.inventory.len(),2);assert_eq!(inventory::occupancy(&r),2.);
    let shards=r.inventory.iter().find(|i|p::is_shard(i)).unwrap();assert_eq!(inventory::quantity(shards),3.);
    assert!(dreamwalking::unlocked(&r));assert!(r.ground["1"].is_empty());
    let saved=storage::serialize(&r).unwrap(); let loaded=storage::deserialize(&saved).unwrap();
    assert_eq!(loaded.inventory,r.inventory);
}
#[test]
fn revision34_natural_actor_profiles_are_still_revision33_and_legacy_shell_is_explicit() {
    for kind in ["rat","witch","skin-walker","dragon"] {
        assert_eq!(population::create_actor_revision(kind,10.,8.,8.,false,34.).unwrap(),population::create_actor_revision(kind,10.,8.,8.,false,33.).unwrap());
    }
    assert_eq!(startup::new_run_shell("dream-generation","Unblessed",false).unwrap().simulation_revision.value_or(0.),39.);
    assert_eq!(startup::new_run_shell_revision("dream-generation","Unblessed",false,33.).unwrap().simulation_revision.value_or(0.),33.);
}

#[test]
fn generated_event_is_walkable_and_population_additions_do_not_rebudget_ordinary_content() {
    let seed=(0..1000).find(|seed|generation::domain_plan_revision(*seed,34.)[0].godbeast_kind=="Phantasmii").unwrap();
    for domain in ["Dungeon","Cave","Meadow","Forest","Archipelago","Mountain"] {
        let f=generation::generate_floor_revision(5,seed,domain,true,79,None,34.).unwrap();
        let altar=f.godbeast_event.as_ref().unwrap().altar.as_ref().unwrap();
        assert!(generation::reachable_without_braziers(&f,None).contains(&(altar.y as usize*79+altar.x as usize)),"{domain}");
        let mut r=startup::new_run_shell_revision("0-1-2:dream-population-budget","Unblessed",false,34.).unwrap();
        r.floors=vec![std::sync::Arc::new(f.clone())]; r.depth=5.;
        let mut legacy=r.clone();legacy.simulation_revision=33.0.into();
        startup::spawn_floor(&mut legacy,5).unwrap();
        startup::spawn_floor(&mut r,5).unwrap();
        assert_eq!(r.actors_at(5.).iter().filter(|a|dreamwalking::is_nightmare(a)).count(),4,"{domain}");
        assert_eq!(r.actors_at(5.).len(),legacy.actors_at(5.).len()+4,"{domain}");
        let budget=|r:&Run|r.floor().unwrap().generation.as_ref().unwrap().stages.iter().find(|s|s.id=="items").unwrap().details.clone();
        assert_eq!(budget(&r),budget(&legacy),"{domain}");
        assert_eq!(r.ground["5"].iter().filter(|g|p::is_shard(&g.item)).count(),3,"{domain}");
        assert_eq!(r.ground["5"].iter().filter(|g|p::is_dagger(&g.item)).count(),1,"{domain}");
        assert_eq!(r.ground["5"].len(),legacy.ground["5"].len()+4,"{domain}");
    }
}

#[test]
fn debug_reconstruction_leaves_prior_dream_quest_and_nightmares_unearned() {
    let seed=(0..1000).map(|n|format!("0-1-2:debug-phantasmii-{n}"))
        .find(|seed|generation::domain_plan_revision(corpse_core::rng::seed_hash(seed),34.)[0].godbeast_kind=="Phantasmii").unwrap();
    let r=corpse_core::debug_floor::create_run(&seed,"Unblessed",6).unwrap();
    assert_eq!(r.simulation_revision.value_or(0.),39.);
    assert!(!dreamwalking::unlocked(&r));
    assert!(!r.inventory.iter().any(|i|p::is_dagger(i)||p::is_shard(i)));
    let s=corpse_core::debug_floor::state(&r).unwrap();
    assert!(!s.stash.iter().any(|v|p::is_dagger(&v.item)||p::is_shard(&v.item)));
    assert_eq!(r.ground["5"].iter().filter(|g|p::is_dagger(&g.item)).count(),1);
    assert_eq!(r.ground["5"].iter().filter(|g|p::is_shard(&g.item)).count(),3);
    for depth in 1..=6 {assert_eq!(r.actors_at(depth as f64).iter().filter(|a|dreamwalking::is_nightmare(a)).count(),4);}
    storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();
    let legacy=corpse_core::debug_floor::create_run_revision("0-1-2:legacy-debug","Unblessed",1,33.).unwrap();
    assert_eq!(legacy.simulation_revision.value_or(0.),33.);
    assert!(!legacy.actors.values().flat_map(|actors| actors.iter()).any(dreamwalking::is_nightmare));
    storage::deserialize(&storage::serialize(&legacy).unwrap()).unwrap();
}
