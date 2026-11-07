use corpse_core::{Command, Run, debug_floor as debug, startup, inventory, progression, storage, temporal::Engine};
const SEED:&str="0-5-11:debug-floor-regression";
fn complete(r:&mut Run) { while !r.offers.is_empty() { let id=r.offers[0].clone();progression::choose_offer(r,&id).unwrap();debug::advance(r).unwrap(); } assert!(debug::choices_complete(r)); }
#[test]
fn strict_floor_bounds_and_floor_one_starter_only() {
    assert!(debug::create_run(SEED,"Avatar",0).is_err());assert!(debug::create_run(SEED,"Avatar",41).is_err());
    assert!(debug::create_run("","Avatar",1).is_err());assert!(debug::create_run("a\nb","Avatar",1).is_err());
    let mut r=debug::create_run(SEED,"Avatar",1).unwrap();
    assert_eq!(r.level,1.);assert!(r.offers.is_empty());assert_eq!(r.inventory.len(),3);
    assert_eq!(r.playtest.as_ref().unwrap().prior_hearts,0);assert!(debug::state(&r).unwrap().stash.is_empty());
    debug::put(&mut r,1.).unwrap();storage::serialize(&r).unwrap();debug::take(&mut r,1.).unwrap();debug::enter(&mut r).unwrap();Engine::new(r).unwrap();
}
#[test]
fn actual_prior_hearts_loot_stats_and_target_are_exact_and_no_backtrack_duplicates() {
    let mut normal=startup::new_run_shell_revision(SEED,"Unblessed",false,33.).unwrap();startup::ensure_floors(&mut normal,10).unwrap();
    let mut expected=normal.clone();let mut hearts=0;let mut blessings=0.;let mut loot=vec![];
    for depth in 1..6 {
        for a in normal.actors_at(depth as f64) {
            if let Some(items)=a.monster_state.as_ref().and_then(|s|s.carried_items.as_ref()){loot.extend(items.iter().cloned());}
            if !debug::natural_heart(a){continue;}
            hearts+=1;let h=inventory::heart_for(a,expected.next_id).unwrap();expected.next_id+=1.;blessings+=h.exp.value_or(0.);let id=h.id;expected.inventory.push(h);expected.depth=depth as f64;
            inventory::command(&mut expected,&Command::Use{id}).unwrap();while !expected.offers.is_empty(){let id=expected.offers[0].clone();progression::choose_offer(&mut expected,&id).unwrap();}
        }
        loot.extend(normal.ground[&depth.to_string()].iter().filter(|g|debug::ordinary_ground(&normal,depth,g)&&g.item.kind!="heart").map(|g|g.item.clone()));
    }
    let mut r=debug::create_run_revision(SEED,"Unblessed",6,33.).unwrap();
    assert_eq!(r.actors["6"],normal.actors["6"]);assert_eq!(r.ground["6"],normal.ground["6"]);
    let p=r.playtest.as_ref().unwrap();assert_eq!(p.prior_hearts,hearts);assert_eq!(p.prior_blessings,blessings);
    let recovered:Vec<_>=debug::state(&r).unwrap().stash.iter().map(|v|v.item.clone()).collect();assert_eq!(recovered,loot);
    assert!(recovered.len()>20);assert_eq!(r.inventory.len(),3);
    complete(&mut r);assert_eq!(r.player.stats,expected.player.stats);assert_eq!((r.level,r.exp),(expected.level,expected.exp));
    assert_eq!(r.run_stats.as_ref().unwrap().blessings_received,blessings);
    for d in 1..6 {assert!(r.actors_at(d as f64).iter().all(|a|!debug::natural_heart(a)));assert!(r.ground[&d.to_string()].iter().all(|g|!debug::ordinary_ground(&r,d,g)));}
    assert_eq!(r.actors["6"],normal.actors["6"]);assert_eq!(r.ground["6"],normal.ground["6"]);
    assert!(normal.playtest.is_missing());
}
#[test]
fn same_seed_and_repeated_load_never_accumulate_and_choices_are_real() {
    for mode in ["Blessed","Avatar","Unblessed"] {
        let a=debug::create_run(SEED,mode,4).unwrap();let b=debug::create_run(SEED,mode,4).unwrap();
        assert_eq!(storage::serialize(&a).unwrap(),storage::serialize(&b).unwrap());
        let mut e=Engine::new(a).unwrap();let target=e.run.actors["4"].clone();let mut count=0;
        while !e.run.offers.is_empty() {
            let level=e.run.level;let old=e.run.player.abilities.clone();let consumed=debug::state(&e.run).unwrap().consumed;
            let before=e.save().unwrap();e.dispatch(&Command::Choose{id:"illegal-ability".into()}).unwrap();assert_eq!(e.run.player.abilities,old);assert_eq!(debug::state(&e.run).unwrap().consumed,consumed);
            // Explicit user/test selection of a current legal offer, never catalog-wide grant.
            let id=e.run.offers[0].clone();assert!(!old.contains(&id));e.dispatch(&Command::Choose{id:id.clone()}).unwrap();count+=1;
            if !id.starts_with("eclipsed"){assert!(e.run.player.abilities.contains(&id));let order=e.run.player.ability_order.as_ref().unwrap();assert_eq!(order.iter().filter(|v|**v==id).count(),1);}
            assert!(e.run.level==level||e.run.level==level+1.);assert_eq!(e.run.tick,0.);assert_eq!(e.run.actors["4"],target);
            let saved=e.save().unwrap();e=Engine::load(&saved).unwrap();assert_eq!(e.save().unwrap(),saved);assert_ne!(saved,before);
        }
        assert_eq!(count as f64,e.run.level-1.);assert!(debug::choices_complete(&e.run));
        let before=e.run.tick;e.dispatch(&Command::Wait).unwrap();assert_eq!(e.run.tick,before);
        debug::enter(&mut e.run).unwrap();assert_eq!(e.run.player.hp,e.run.player.max_hp);
    }
}
#[test]
fn overflow_is_retained_with_fixed_twenty_slots_and_target_forty_valid() {
    let mut r=debug::create_run(SEED,"Unblessed",40).unwrap();assert_eq!(r.floors.len(),40);complete(&mut r);
    let total=debug::state(&r).unwrap().stash.len()+r.inventory.len();
    let ids:Vec<_>=debug::state(&r).unwrap().stash.iter().filter(|v|!inventory::is_arrow(&v.item)).map(|v|v.item.id).collect();
    let mut refused=0;
    for id in ids {if debug::take(&mut r,id).is_err(){refused+=1;}}
    assert!(refused>0);assert_eq!(inventory::occupancy(&r),20.);assert_eq!(debug::state(&r).unwrap().stash.len()+r.inventory.len(),total);
    let before=storage::serialize(&r).unwrap();assert!(!debug::take(&mut r,-1.).unwrap());assert_eq!(storage::serialize(&r).unwrap(),before);
    let id=r.inventory.last().unwrap().id;debug::put(&mut r,id).unwrap();assert_eq!(inventory::occupancy(&r),19.);debug::take(&mut r,id).unwrap();assert_eq!(inventory::occupancy(&r),20.);
    debug::enter(&mut r).unwrap();Engine::new(r).unwrap();
}
#[test]
fn temporal_abilities_cannot_skip_preparation_or_duplicate_reconstruction() {
    let mut r=debug::create_run(SEED,"Unblessed",3).unwrap();complete(&mut r);
    assert_eq!(r.initial_player.as_ref().unwrap().stats.offense,0.);
    for id in ["numo-revert","morae-see-the-future"] {r.player.abilities.push(id.into());progression::initialize_ability_state(&mut r.player,id,r.tick).unwrap();}
    r.player.conditions.insert("revertCharges".into(),2.);
    let mut e=Engine::new(r).unwrap();let ability=|id:&str|Command::Ability{id:id.into(),target:Default::default(),x:Default::default(),y:Default::default()};
    e.dispatch(&ability("morae-see-the-future")).unwrap();assert!(e.run.future.is_none());assert!(debug::preparing(&e.run));
    debug::enter(&mut e.run).unwrap();let origin=e.run.playtest.clone();
    e.dispatch(&Command::Wait).unwrap();e.dispatch(&ability("numo-revert")).unwrap();assert_eq!(e.run.tick,0.);assert_eq!(e.run.playtest,origin);assert!(!debug::preparing(&e.run));
    e.dispatch(&ability("morae-see-the-future")).unwrap();assert!(e.run.future.is_some());e.dispatch(&Command::Wait).unwrap();e.dispatch(&ability("morae-see-the-future")).unwrap();assert_eq!(e.run.tick,0.);assert_eq!(e.run.playtest,origin);
}
