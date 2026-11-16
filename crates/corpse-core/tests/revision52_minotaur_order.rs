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
