use corpse_core::{ai,engine,population,startup,new_creatures as n,Field,Floor,Run};
use std::sync::Arc;
fn room(kind:&str,revision:f64)->Run {
    let mut r=startup::new_run_shell_revision("0-5-11:witch33","Unblessed",false,revision).unwrap();
    r.player.x=2.;r.player.y=2.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.ability_order=Field::Missing;
    let w=79;let h=29;let mut tiles=vec!["wall".into();w*h];for y in 1..h-1 {for x in 1..w-1 {tiles[y*w+x]="floor".into();}}
    let mut f=Floor{width:(w as f64).into(),height:(h as f64).into(),depth:1.,domain:"Dungeon".into(),tiles,lights:vec![100.;w*h],seen:vec![true;w*h],..Default::default()};
    corpse_core::environment::initialize(&mut f).unwrap();r.floors=vec![Arc::new(f)];
    let mut a=population::create_actor_revision(kind,4.,5.,5.,false,revision).unwrap();a.state="hunting".into();
    r.actors.clear();r.actors.insert("1".into(),Arc::new(vec![a]));r.ground.clear();r.ground.insert("1".into(),Arc::new(vec![]));r.next_id=5.;r.tick=3.;r.generated_ability_grants=Vec::new().into();r
}
fn cast(r:&mut Run)->bool {ai::enemy_action(r,4.,r.player.id).unwrap()}
#[test]
fn witch_spends_only_successful_summons_and_keeps_curses_between() {
    let mut r=room("witch",33.);assert!(cast(&mut r));
    assert_eq!(r.actors_at(1.).len(),2);assert_eq!(engine::actor(&r,4.).unwrap().energy,40.);
    assert_eq!(engine::condition(engine::actor(&r,4.).unwrap(),"monsterSummonReadyAt"),13.);
    let rng=r.rng;let target=r.player.id;assert!(!corpse_core::witch::summon(&mut r,4.,target).unwrap());assert_eq!(r.rng,rng);
    assert!(cast(&mut r));assert_eq!(r.actors_at(1.).len(),2);assert!(engine::condition(&r.player,"slowed")>0.);
    r.tick=12.;assert!(!corpse_core::witch::summon(&mut r,4.,target).unwrap());
    r.tick=13.;assert!(cast(&mut r));assert_eq!(r.actors_at(1.).len(),3);
    for a in r.actors_at(1.).iter().skip(1) {assert!(n::template(a.creature_kind.as_ref().unwrap()).is_some());assert_eq!(a.max_hp,244.);}
    let mut legacy=room("witch",32.);assert!(cast(&mut legacy));assert_eq!(legacy.actors_at(1.).len(),1);
}
#[test]
fn failed_summons_are_atomic_and_heartless_is_inherited() {
    for failure in ["energy","space","silence"] {
        let mut r=room("witch",33.);
        match failure {"energy"=>engine::actor_mut(&mut r,4.).unwrap().energy=9.,"silence"=>{engine::actor_mut(&mut r,4.).unwrap().conditions.insert("silenced".into(),2.);},_=>{for y in 4..=6 {for x in 4..=6 {if (x,y)!=(5,5) {r.floor_mut().unwrap().tiles[y*79+x]="water".into();}}}}}
        let before=r.clone();let target=r.player.id;assert!(!corpse_core::witch::summon(&mut r,4.,target).unwrap());assert_eq!(r,before,"{failure}");
    }
    let mut kinds=std::collections::BTreeSet::new();
    for seed in 1..=32 {let mut r=room("witch",33.);r.rng=seed as f64;engine::actor_mut(&mut r,4.).unwrap().heartless=true.into();assert!(cast(&mut r));let child=&r.actors_at(1.)[1];assert!(child.heartless.value_or(false));kinds.insert(child.creature_kind.as_ref().unwrap().clone());}
    assert_eq!(kinds.len(),2);
}
#[test]
fn disguise_reverses_at_chebyshev_two_and_preserves_true_state() {
    let mut r=room("skin-walker",33.);let id=4.;let a=engine::actor(&r,id).unwrap().clone();
    let disguise=n::disguise_kind(&r,&a).unwrap();assert!(n::DISGUISES.contains(&disguise));
    r.player.x=3.;r.player.y=3.;assert!(n::disguise_kind(&r,&a).is_none(),"diagonal distance2 reveals");
    r.player.x=2.;assert_eq!(n::disguise_kind(&r,&a),Some(disguise));
    r.player.x=3.;r.floor_mut().unwrap().tiles[4*79+4]="wall".into();assert_eq!(n::disguise_kind(&r,&a),Some(disguise),"wall prevents observation");
    assert_eq!(engine::actor(&r,id).unwrap(),&a);
}
#[test]
fn half_here_reveals_from_observing_body_only() {
    let mut r=room("skin-walker",33.);r.player.abilities.push(corpse_core::half_here::ID.into());r.player.active.push(corpse_core::half_here::ID.into());r.player.conditions.insert("halfHereId".into(),9.);
    let mut body=r.player.clone();body.id=9.;body.x=3.;body.y=3.;body.state="ally".into();body.conditions.insert("halfHereClone".into(),1.);body.conditions.insert("halfHereOwner".into(),r.player.id);r.actors_at_mut(1.).push(body);
    assert!(n::disguise_kind(&r,engine::actor(&r,4.).unwrap()).is_none());
    engine::actor_mut(&mut r,9.).unwrap().dead=true.into();assert!(n::disguise_kind(&r,engine::actor(&r,4.).unwrap()).is_some());
}
#[test]
fn new_species_have_bounded_revisioned_population_and_real_hearts() {
    for revision in [29.,30.,31.,32.] {assert!(population::create_actor_revision("skin-walker",1.,0.,0.,false,revision).is_err());for depth in [24,26,30,36] {assert!(corpse_core::ordinary_roster::candidates_revision(depth,true,revision).unwrap().iter().all(|c|c.kinds.iter().all(|k|n::template(k).is_none())));}}
    for (kind,first,last) in [("night-crawler",24,34),("skin-walker",26,36)] {
        let a=population::create_actor_revision(kind,17.,1.,1.,false,33.).unwrap();let heart=corpse_core::inventory::heart_for(&a,9.).unwrap();assert_eq!(heart.exp.value_or(0.),2.);assert_eq!(heart.energy.value_or(0.),25.);
        for depth in 1..=40 {let pool=corpse_core::ordinary_roster::candidates_revision(depth,true,33.).unwrap();assert_eq!(pool.iter().any(|c|c.kinds.iter().any(|k|k==kind)),(first..=last).contains(&depth));assert!(pool.iter().all(|c|c.weight>0. && c.weight<=1.));}
    }
    for depth in [24,26,27,34,36,40] {let plan=population::plan_budgeted_hordes_revision("0-5-11:r33-budget",depth,24,0,true,&[],33.).unwrap();assert_eq!(plan.hordes.iter().map(|h|population::heart_blessings(&h.kinds).unwrap()).sum::<i32>(),24);}
}
#[test]
fn new_state_survives_save_snapshot_revert_and_future() {
    let mut r=room("witch",33.);r.player.conditions.insert("revertCharges".into(),3.);assert!(cast(&mut r));
    let mut skin=population::create_actor_revision("skin-walker",r.next_id,6.,6.,false,33.).unwrap();skin.state="sleeping".into();r.next_id+=1.;r.actors_at_mut(1.).push(skin);
    let expected=r.actors_at(1.).to_vec();
    let raw=corpse_core::storage::serialize(&r).unwrap();r=corpse_core::storage::deserialize(&raw).unwrap();assert_eq!(r.actors_at(1.),expected);
    let opening=corpse_core::temporal::Snapshot::capture(&r);let mut history=corpse_core::temporal::History::default();history.record_completed(opening);
    r.tick+=1.;r.actors_at_mut(1.)[2].x=3.;history.closing=Some(corpse_core::temporal::Snapshot::capture(&r));assert!(corpse_core::temporal::revert(&mut r,&mut history).unwrap());assert_eq!(r.actors_at(1.),expected);
    corpse_core::temporal::remember_future(&mut r,&history).unwrap();r.actors_at_mut(1.)[2].x=2.;assert!(corpse_core::temporal::restore_future(&mut r,&mut history).unwrap());assert_eq!(r.actors_at(1.),expected);
    assert_eq!(startup::new_run_shell("0-5-11:r33","Unblessed",false).unwrap().simulation_revision.value_or(0.),39.);
}
#[test]
fn new_creatures_execute_native_turns_and_offscreen_catchup_without_identity_changes() {
    for kind in ["night-crawler","skin-walker"] {
        let mut r=room(kind,33.);r.player.stats.dr=1000.;r.player.hp=10000.;r.player.max_hp=10000.;r.player.base_max_hp=10000.0.into();
        let choice=engine::actor(&r,4.).unwrap().conditions.get("skinWalkerDisguise").copied();
        for _ in 0..3 {engine::dispatch(&mut r,&corpse_core::Command::Wait).unwrap();}
        assert_eq!(engine::actor(&r,4.).unwrap().creature_kind.as_ref().map(String::as_str),Some(kind));
        assert_eq!(engine::actor(&r,4.).unwrap().conditions.get("skinWalkerDisguise").copied(),choice);
        let mut f=(*r.floors[0]).clone();f.depth=2.;r.floors.push(Arc::new(f));r.depth=2.;r.actors.insert("2".into(),Arc::new(vec![]));r.ground.insert("2".into(),Arc::new(vec![]));r.tick+=5.;
        corpse_core::offscreen::catch_up_floor(&mut r,1.).unwrap();
        let a=r.actors_at(1.).iter().find(|a|a.id==4.).unwrap();assert_eq!(a.creature_kind.as_ref().map(String::as_str),Some(kind));assert_eq!(a.conditions.get("skinWalkerDisguise").copied(),choice);
    }
}
#[test]
fn revision33_replays_native_summons_exactly() {
    let initial=room("witch",33.);let mut engine=corpse_core::temporal::Engine::new(initial.clone()).unwrap();
    for _ in 0..4 {engine.dispatch(&corpse_core::Command::Wait).unwrap();}
    let record=corpse_core::replay::ReplayRecord::from_run(&engine.run);
    let mut replay=corpse_core::replay::ReplaySession::from_initial(initial,record).unwrap();
    while replay.step().unwrap() {}
    assert_eq!(replay.engine.run.actors,engine.run.actors);assert_eq!(replay.engine.run.rng,engine.run.rng);
}

#[test]
fn revived_skinwalker_retains_disguise_and_unspeak_prices_witch_summons() {
    let mut a=population::create_actor_revision("skin-walker",17.,5.,5.,false,33.).unwrap();
    let choice=a.conditions["skinWalkerDisguise"];a.dead=true.into();a.hp=0.;a.heartless=true.into();
    corpse_core::corpses::revive_corpse(&mut a,20.);assert!(!a.dead.value_or(false));assert_eq!(a.conditions["skinWalkerDisguise"],choice);
    for (revision,expected) in [(32.,8.),(33.,10.)] {let mut r=room("witch",revision);assert_eq!(corpse_core::abilities::highest_energy_cost(&mut r,4.).unwrap(),expected);}
}

#[test]
fn uncast_cancels_one_witch_attempt_without_double_charging_or_summon_cooldown() {
    let mut r=room("witch",33.);r.player.abilities.push("grisha-uncast".into());r.player.active.push("grisha-uncast".into());r.player.energy=100.;
    assert!(cast(&mut r));assert_eq!(r.player.energy,90.);assert_eq!(r.actors_at(1.).len(),1);assert_eq!(engine::actor(&r,4.).unwrap().energy,50.);assert_eq!(engine::condition(engine::actor(&r,4.).unwrap(),"monsterSummonReadyAt"),0.);assert_eq!(engine::condition(&r.player,"slowed"),0.);
}

#[test]
fn queen_is_conditional_only_with_explicit_guardian_heart_and_trait_cost() {
    let a=population::create_actor_revision("saidribee-queen",17.,5.,5.,false,33.).unwrap();
    assert_eq!(a.name,"Saidribee Queen");assert_eq!(a.max_hp,430.);assert_eq!(a.stats.offense,230.);assert_eq!(a.stats.defense,48.);assert_eq!(a.stats.max_energy,100.);
    let queen=corpse_core::inventory::heart_for(&a,77.).unwrap();let guardian=corpse_core::inventory::heart_for(&population::catalog().actors["saidri-guardian"],77.).unwrap();
    assert_eq!(queen.bonus,guardian.bonus);assert_eq!(queen.energy,guardian.energy);assert_eq!(queen.exp,guardian.exp);assert_eq!(queen.effect,guardian.effect);assert!(queen.name.contains("Queen"));
    assert_eq!(corpse_core::enemy_traits::catalog().abilities["enemy:beast:saidribee-queen"].cost,15.);
    for rev in [29.,30.,31.,32.] {assert!(population::create_actor_revision("saidribee-queen",17.,5.,5.,false,rev).is_err());}
    for depth in 1..=40 {assert!(corpse_core::ordinary_roster::candidates_revision(depth,true,33.).unwrap().iter().all(|c|c.kinds.iter().all(|k|k!="saidribee-queen")));}
    let mut r=room("witch",33.);r.actors_at_mut(1.).push(a);let raw=corpse_core::storage::serialize(&r).unwrap();assert_eq!(corpse_core::storage::deserialize(&raw).unwrap().actors,r.actors);
}

/// O(S + A*d) for state encoding and registry admission in this bounded fixture.
#[test]
fn birth_identity_collision_and_exhaustion_are_atomic() {
    for cursor in [4., 9_007_199_254_740_991.] {
        let mut r = room("witch", 33.);
        r.next_id = cursor;
        let target = r.player.id;
        let before = corpse_core::storage::canonical_value(&r).unwrap();
        let rng = r.rng;
        assert!(corpse_core::witch::summon(&mut r, 4., target).is_err());
        assert_eq!(r.rng, rng);
        assert_eq!(corpse_core::storage::canonical_value(&r).unwrap(), before);
    }
}

/// One ordinary turn plus O(S) canonical comparison; simulation costs remain real.
#[test]
fn birth_collision_dispatch_rolls_back_and_success_preserves_floor_handle() {
    let mut r = room("witch", 33.);
    r.next_id = 4.;
    let before = corpse_core::storage::canonical_value(&r).unwrap();
    let rng = r.rng;
    assert!(engine::dispatch(&mut r, &corpse_core::Command::Wait).is_err());
    assert_eq!(r.rng, rng);
    assert_eq!(corpse_core::storage::canonical_value(&r).unwrap(), before);

    let mut r = room("witch", 33.);
    let (floor, handle) = r.actors.floor_id(4., r.player.id, r.depth).unwrap().unwrap();
    engine::dispatch(&mut r, &corpse_core::Command::Wait).unwrap();
    assert!(r.actors_at(1.).len() > 1, "control turn must actually summon");
    assert_eq!(r.actors.resolve_floor_id(&floor, handle, r.player.id, r.depth).unwrap(), Some(4.));
}

/// O(S + A*d) admission and canonical comparison for a two-birth batch.
#[test]
fn twin_birth_rejects_second_identity_before_spend_or_rng() {
    let mut r = room("pale:Greater Birther", 33.);
    let parent = engine::actor_mut(&mut r, 4.).unwrap();
    parent.energy = 100.;
    if !parent.abilities.iter().any(|id| id == "enemy:pale:double-birth") {
        parent.abilities.push("enemy:pale:double-birth".into());
    }
    corpse_core::enemy_traits::initialize_enemy_traits(parent);
    let mut blocker = population::create_actor_revision("witch", 6., 12., 12., false, 33.).unwrap();
    blocker.state = "sleeping".into();
    r.actors_at_mut(1.).push(blocker);
    r.next_id = 5.;
    let target = r.player.id;
    let before = corpse_core::storage::canonical_value(&r).unwrap();
    let rng = r.rng;
    assert!(ai::enemy_action(&mut r, 4., target).is_err());
    assert_eq!(r.rng, rng);
    assert_eq!(corpse_core::storage::canonical_value(&r).unwrap(), before);
}
