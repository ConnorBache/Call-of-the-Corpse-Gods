use corpse_core::{Command, Field, Run, engine, population};
use std::sync::Arc;
fn room(revision:f64)->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=revision.into(); r.player.x=10.; r.player.y=10.;
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.stats.accuracy=1000.; r.player.stats.offense=0.;
    r.player.hp=1000.;r.player.max_hp=1000.;r.player.base_max_hp=1000.0.into();
    r.inventory.clear();r.offers.clear();r.history.clear();r.replay.clear();r.future=None;
    r.phase=Field::Missing;r.haste_snapshot=Field::Missing;
    for v in r.actors.values_mut(){*v=Arc::new(vec![]);}
    for v in r.ground.values_mut(){*v=Arc::new(vec![]);}
    let f=r.floor_mut().unwrap(); f.tiles.fill("floor".into()); f.lights.fill(1.);f.seen.fill(true);
    f.environment=Field::Missing;f.doors=Field::Missing;f.vents=Field::Missing;f.currents=Field::Missing;
    f.mud=Field::Missing;f.slime=Field::Missing;f.roots=Field::Missing;f.fungal_roots=Field::Missing;
    f.ichor=Field::Missing;f.light_sources=Field::Missing;f.permanent_fire=Field::Missing;
    f.snow=Field::Missing;f.ice=Field::Missing;f.hellfire=Field::Missing;
    let mut a=population::create_actor_revision("minotaur",r.next_id,11.,10.,false,revision).unwrap();
    a.stats.dr=0.;a.stats.defense=0.;a.stats.dodge=-1000.;
    r.next_id+=1.; a.state="hunting".into();a.hp=1000.;a.max_hp=1000.;a.base_max_hp=1000.0.into();
    a.conditions.insert("monsterChargeDx".into(),1.);a.conditions.insert("monsterChargeDy".into(),0.);
    a.conditions.insert("monsterChargeRemaining".into(),2.);
    a.last_seen=corpse_core::ActorLastSeen{x:10.,y:10.}.into();
    r.actors_at_mut(r.depth).push(a);r
}
#[test]
fn revision52_can_hit_displayed_minotaur_before_two_committed_steps() {
    for revision in [51.,52.] {
        let mut r=room(revision);
        engine::dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap();
        let a=&r.actors_at(r.depth)[0];
        assert_eq!(a.x,13.);assert_eq!(a.y,10.);
        assert_eq!(a.conditions["monsterChargeRemaining"],2.);
        if revision>=52. {assert!(a.hp<1000.,"hp={} log={:?}",a.hp,r.log);assert_eq!(r.player.x,10.);}
        else {assert_eq!(a.hp,1000.);assert_eq!(r.player.x,11.);}
    }
}
#[test]
fn complete_fast_action_owns_two_steps_but_player_haste_does_not_move_it() {
    for mode in 0..3 {
        let mut r=room(52.);
        let command=if mode==1 {r.player.conditions.insert("hasted".into(),5.);Command::Move{dx:0.,dy:1.}} else {Command::Wait};
        if mode==2 {r.phase="normal".to_string().into();}
        engine::dispatch(&mut r,&command).unwrap();
        let a=&r.actors_at(r.depth)[0];
        assert_eq!(a.x,if mode==1 {11.} else {13.});assert_eq!(a.y,10.);
        assert_eq!(a.conditions["monsterChargeRemaining"],2.);
    }
}
#[test]
fn rejected_move_does_not_spend_minotaur_commitment() {
    let mut r=room(52.);let old=r.actors_at(r.depth)[0].clone();
    assert!(!engine::dispatch(&mut r,&Command::Move{dx:2.,dy:0.}).unwrap().took_turn);
    assert_eq!(r.actors_at(r.depth)[0],old);
}
#[test]
fn arrival_floor_gets_one_complete_fast_action_not_an_old_deferred_pass() {
    let mut r=room(52.);
    r.floor_mut().unwrap().tiles[10*79+10]="down".into();
    let mut upper=r.floor().unwrap().clone();upper.depth=2.;upper.tiles[10*79+10]="up".into();
    if let Some(f)=r.floors.iter_mut().find(|f|f.depth==2.) {*f=Arc::new(upper);} else {r.floors.push(Arc::new(upper));}
    let mut a=r.actors_at(1.)[0].clone();a.id=r.next_id;r.next_id+=1.;
    r.actors.insert("2".into(),Arc::new(vec![a]));
    engine::dispatch(&mut r,&Command::Descend).unwrap();
    assert_eq!(r.depth,2.);
    assert_eq!(r.actors_at(2.)[0].x,13.);
    assert_eq!(r.actors_at(2.)[0].conditions["monsterChargeRemaining"],2.);
}
#[test]
fn player_can_kill_minotaur_before_deferred_charge() {
    let mut r=room(52.);r.actors_at_mut(r.depth)[0].hp=1.;
    engine::dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap();
    assert!(r.actors_at(r.depth).iter().all(|a|a.dead.value_or(false)||a.hp<=0.));
    assert_eq!(r.player.x,10.);
}

#[test]
fn only_a_real_charge_bump_can_attack() {
    let mut r=room(52.);
    r.minotaur_bump_from=0usize.into();
    r.player.x=13.;
    r.player.stats.dodge=-1000.;
    let victim=r.player.id;
    let id=r.actors_at(r.depth)[0].id;
    let hp=r.player.hp;
    let rng=r.rng.clone();
    // Even a direct attack aimed down the arrow is not a movement bump.
    engine::attack(&mut r,id,victim,1.).unwrap();
    assert_eq!(r.player.hp,hp);
    assert_eq!(r.rng,rng);
    assert_eq!(engine::attack_blocker(&r,engine::actor(&r,id).unwrap(),&r.player,0.,false),
        Some("minotaur-only-bumps"));
    engine::dispatch(&mut r,&Command::Wait).unwrap();
    assert_eq!(r.actors_at(r.depth)[0].x,12.);
    assert!(r.player.hp<hp,"a move followed by a real bump still lands");
}

#[test]
fn minotaur_cannot_bump_through_a_closed_diagonal_corner() {
    let mut r=room(52.);
    r.minotaur_bump_from=0usize.into();
    r.player.x=12.;r.player.y=11.;
    r.player.stats.dodge=-1000.;
    r.actors_at_mut(r.depth)[0].conditions.insert("monsterChargeDy".into(),1.);
    let f=r.floor_mut().unwrap();let w=f.width();
    f.tiles[10*w+12]="wall".into(); f.tiles[11*w+11]="wall".into();
    let hp=r.player.hp;
    engine::dispatch(&mut r,&Command::Wait).unwrap();
    assert_eq!(r.player.hp,hp);
    let a=&r.actors_at(r.depth)[0];
    assert_eq!((a.x,a.y),(11.,10.));
    assert_eq!(a.conditions["monsterChargeRemaining"],1.);
}

#[test]
fn discord_allows_the_cued_bump_but_not_a_side_attack() {
    for x in [10.,12.] {
        let mut r=room(52.);
        r.minotaur_bump_from=0usize.into();
        r.player.x=x;r.player.stats.dodge=-1000.;
        let depth=r.depth;
        let a=&mut r.actors_at_mut(depth)[0];
        a.conditions.insert(corpse_core::revision50_control_amulets::DISCORD.into(),5.);
        a.stats.perception=300.;
        let hp=r.player.hp;
        engine::dispatch(&mut r,&Command::Wait).unwrap();
        assert_eq!(r.player.hp<hp,x==12.);
    }
}

#[test]
fn live_adoption_persists_only_at_the_replay_boundary() {
    let mut old=corpse_core::startup::create_avatar_run("minotaur-bump-replay").unwrap();
    old.minotaur_bump_from=Field::Missing;
    engine::dispatch(&mut old,&Command::Wait).unwrap();
    let boundary=old.replay.len();
    let live=corpse_core::temporal::Engine::new(old).unwrap();
    assert_eq!(live.run.minotaur_bump_from.as_ref(),Some(&boundary));
    let saved=corpse_core::storage::serialize(&live.run).unwrap();
    assert_eq!(corpse_core::storage::deserialize(&saved).unwrap().minotaur_bump_from.as_ref(),Some(&boundary));
    let record=corpse_core::replay::ReplayRecord::from_run(&live.run);
    let mut replay=corpse_core::replay::ReplaySession::create(record,true).unwrap();
    assert!(!corpse_core::minotaur_rules::current(&replay.engine.run));
    while replay.step().unwrap() {}
    assert!(corpse_core::minotaur_rules::current(&replay.engine.run));
}

#[test]
fn blocked_counterattack_does_not_spend_energy() {
    let mut r=room(52.);
    r.minotaur_bump_from=0usize.into();
    let player=r.player.id;let depth=r.depth;
    let a=&mut r.actors_at_mut(depth)[0];
    let id=a.id;
    a.active.push("pelian-never-without-a-fight".into());
    a.abilities.push("pelian-never-without-a-fight".into());
    a.energy=100.;
    let hp=r.player.hp;
    engine::attack(&mut r,player,id,1.).unwrap();
    assert_eq!(r.player.hp,hp);
    assert_eq!(engine::actor(&r,id).unwrap().energy,100.);
}

fn straight_room() -> Run {
    let mut r=room(52.);
    r.minotaur_bump_from=0usize.into();
    r.minotaur_straight_from=0usize.into();
    r.player.stats.dodge=-1000.;
    r
}

#[test]
fn minotaur_finishing_the_last_step_cannot_attack_down_a_new_arrow() {
    let mut r=straight_room();
    r.player.x=12.;r.player.y=11.;
    let hp=r.player.hp;
    let a=&mut r.actors_at_mut(1.)[0];
    a.stats.perception=300.;
    a.conditions.insert("monsterChargeRemaining".into(),1.);
    let out=engine::dispatch(&mut r,&Command::Wait).unwrap();
    assert_eq!(r.player.hp,hp);
    let a=&r.actors_at(1.)[0];
    assert_eq!((a.x,a.y),(12.,10.));
    assert_eq!(a.conditions["monsterChargeRemaining"],2.);
    let windups:Vec<_>=out.events.iter().filter(|e|e.animation.as_ref().is_some_and(|a|a.style=="wave-charge-inward")).collect();
    assert_eq!(windups.len(),1);
    assert_eq!((windups[0].to.x,windups[0].to.y),(12.,10.));
}

#[test]
fn minotaur_charge_does_not_bump_a_portal_alias() {
    let mut r=straight_room();
    r.player.x=20.;
    let hp=r.player.hp;
    let f=r.floor_mut().unwrap();
    f.portals=vec![corpse_core::portals_generation::PortalPair{a:10*79+12,b:10*79+20,color_index:0}].into();
    engine::dispatch(&mut r,&Command::Wait).unwrap();
    let a=&r.actors_at(1.)[0];
    assert_eq!((a.x,a.y),(11.,10.));
    assert_eq!(r.player.hp,hp);
}

#[test]
fn minotaur_charge_crosses_fire_and_catches_on_first_step() {
    let mut r=straight_room();
    r.floor_mut().unwrap().tiles[10*79+12]="grass".into();
    corpse_core::environment::ignite(&mut r,12.,10.,Some(20.),None).unwrap();
    engine::dispatch(&mut r,&Command::Wait).unwrap();
    let a=&r.actors_at(1.)[0];
    assert_eq!((a.x,a.y),(13.,10.));
    assert!(engine::condition(a,"burning")>0.);
}

#[test]
fn minotaur_charge_falls_and_stops_before_the_second_step() {
    let mut r=straight_room();
    let mut below=r.floor().unwrap().clone();
    below.depth=2.;
    if let Some(f)=r.floors.iter_mut().find(|f|f.depth==2.) {*f=Arc::new(below);}
    else {r.floors.push(Arc::new(below));}
    r.floor_mut().unwrap().tiles[10*79+12]="chasm".into();
    let id=r.actors_at(1.)[0].id;
    assert!(engine::dispatch(&mut r,&Command::Wait).unwrap().took_turn);
    assert_eq!(r.depth,1.);
    assert!(r.actors_at(1.).iter().all(|a|a.id!=id));
    let landed=r.actors_at(2.).iter().find(|a|a.id==id).unwrap();
    assert_eq!((landed.x,landed.y),(12.,10.));
    assert!(landed.hp<1000. && landed.hp>0.);
}

#[test]
fn adopted_straight_charge_is_at_the_live_save_boundary_only() {
    let mut old=corpse_core::startup::create_avatar_run("minotaur-straight-replay").unwrap();
    old.minotaur_straight_from=Field::Missing;
    engine::dispatch(&mut old,&Command::Wait).unwrap();
    let boundary=old.replay.len();
    let live=corpse_core::temporal::Engine::new(old).unwrap();
    assert_eq!(live.run.minotaur_straight_from.as_ref(),Some(&boundary));
    let saved=corpse_core::storage::serialize(&live.run).unwrap();
    assert_eq!(corpse_core::storage::deserialize(&saved).unwrap().minotaur_straight_from.as_ref(),Some(&boundary));
    let record=corpse_core::replay::ReplayRecord::from_run(&live.run);
    let mut replay=corpse_core::replay::ReplaySession::create(record,true).unwrap();
    assert!(!corpse_core::minotaur_straight::current(&replay.engine.run));
    while replay.step().unwrap() {}
    assert!(corpse_core::minotaur_straight::current(&replay.engine.run));
}
