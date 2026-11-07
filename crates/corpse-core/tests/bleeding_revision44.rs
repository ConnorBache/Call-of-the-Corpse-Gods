//! Bleed Them revision44 packets; duration and normal damage handling are shared.
use corpse_core::{Actor, Command, Field, Run, divine_reactions as d, engine};
use std::sync::Arc;
fn room(revision:f64)->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=revision.into();r.tick=0.;r.depth=1.;r.status="playing".into();
    r.inventory.clear();r.actors.clear();r.ground.clear();r.history.clear();r.haste_snapshot=Field::Missing;
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.ability_order=Field::Missing;
    r.player.x=8.;r.player.y=8.;r.player.base_max_hp=100.0.into();r.player.max_hp=100.;r.player.hp=100.;
    r.player.stats.max_hp=Field::Missing;r.player.energy=100.;r.player.stats.max_energy=100.;
    r.phase=Field::Missing;r.replay.clear();r.offers.clear();r.next_id=1000.;
    let mut f=r.floors[0].as_ref().clone();f.environment=Field::Missing;f.vents=Field::Missing;
    f.permanent_fire=Field::Missing;f.godbeast_event=Field::Missing;f.godbeast=false;f.doors=Field::Missing;
    f.dirty=true;f.last_tick=0.;let w=f.width();let h=f.height();
    for y in 1..h-1 {for x in 1..w-1 {f.tiles[y*w+x]="floor".into();}}
    f.tiles[w+1]="up".into();f.tiles[(h-2)*w+w-2]="down".into();
    r.floors=(1..=10).map(|depth|{let mut f=f.clone();f.depth=depth as f64;Arc::new(f)}).collect();
    for depth in 1..=10 {r.actors.insert(depth.to_string(),Arc::new(vec![]));r.ground.insert(depth.to_string(),Arc::new(vec![]));}
    r
}
fn npc(r:&mut Run)->f64 {
    let id=900.;let mut a=corpse_core::population::create_actor_revision("rat",id,30.,20.,false,43.).unwrap();
    a.state="sleeping".into();a.abilities.clear();a.active.clear();a.conditions.clear();
    a.base_max_hp=100.0.into();a.hp=100.;a.max_hp=100.;a.stats.max_hp=Field::Missing;
    a.conditions.insert("paralyzed".into(),100.);r.actors_at_mut(1.).push(a);id
}
fn packet(r:&mut Run,id:f64,source:Option<f64>) {
    let a=engine::actor(r,id).unwrap();let (n,exact)=d::bleeding_packet(r.simulation_revision.value_or(29.),a);
    engine::damage_environment(r,id,n,source,"bleeding",exact).unwrap();
}
#[test]
fn thresholds_fractional_stacks_zero_and_legacy_max_hp_formula() {
    let mut a=Actor::default();
    for max_hp in [10.,100.,10000.] {a.max_hp=max_hp;
        for (stacks,damage) in [(0.,0.),(-1.,0.),(0.5,1.),(1.,1.),(4.999,1.),(5.,2.),(9.,2.),(10.,3.),(15.,4.)] {
            a.conditions.insert("bleeding".into(),stacks);
            assert_eq!(d::bleeding_packet(44.,&a),(damage,true));
            assert_eq!(d::bleeding_packet(43.,&a),(max_hp*0.02,false));
        }
    }
}
#[test]
fn packets_ignore_armor_as_before_but_keep_nightmare_protection_and_source_death() {
    for revision in [43.,44.] {for player in [false,true] {
        let mut r=room(revision);let enemy=npc(&mut r);let id=if player{r.player.id}else{enemy};
        let a=engine::actor_mut(&mut r,id).unwrap();a.stats.defense=10000.;a.stats.dr=10000.;a.conditions.insert("bleeding".into(),10.);
        packet(&mut r,id,None);assert_eq!(engine::actor(&r,id).unwrap().hp,if revision==43.{99.}else{97.});
    }}
    let mut r=room(44.);let enemy=npc(&mut r);engine::actor_mut(&mut r,enemy).unwrap().state="hunting".into();
    r.player.conditions.insert("bleeding".into(),10.);r.player.active.push(d::NIGHTMARE.into());r.player.nightmares=1.0.into();
    let player=r.player.id;packet(&mut r,player,Some(enemy));assert_eq!(r.player.hp,100.);assert_eq!(r.player.nightmares.value_or(0.),0.);
    r.player.active.clear();r.player.hp=1.;packet(&mut r,player,Some(enemy));
    // Player terminal state uses Run.status/HP; Actor.dead is the NPC corpse flag.
    assert_eq!(r.status,"dead");assert_eq!(r.player.hp,0.);
    let death=r.run_stats.as_ref().unwrap().death.as_ref().unwrap();
    assert_eq!(death.cause.as_ref().map(String::as_str),Some("bleeding"));assert!(death.reason.contains("Bleeding"));assert_eq!(death.source_name.as_ref(),Some(&engine::actor(&r,enemy).unwrap().name));
}
#[test]
fn application_retains_max_duration_and_decay_crosses_thresholds_without_stacking_addition() {
    let mut r=room(44.);let id=npc(&mut r);r.player.abilities.push(d::BLEED.into());let a=r.player.clone();
    d::apply_bleeding(&mut r,&a,id,1.,10.);d::apply_bleeding(&mut r,&a,id,1.,3.5);
    assert_eq!(engine::condition(engine::actor(&r,id).unwrap(),"bleeding"),10.);
    assert_eq!(engine::condition(engine::actor(&r,id).unwrap(),"bleedSourceId"),a.id);
    for (stacks,expected) in [(10.,3.),(9.,2.),(8.,2.),(7.,2.),(6.,2.),(5.,2.),(4.,1.),(3.,1.),(2.,1.),(1.,1.)] {
        let b=engine::actor(&r,id).unwrap();assert_eq!(engine::condition(b,"bleeding"),stacks);assert_eq!(d::bleeding_packet(44.,b),(expected,true));
        engine::tick_condition(engine::actor_mut(&mut r,id).unwrap(),"bleeding");
    }
    d::apply_bleeding(&mut r,&a,id,1.,0.5);assert_eq!(d::bleeding_packet(44.,engine::actor(&r,id).unwrap()),(1.,true));
    engine::tick_condition(engine::actor_mut(&mut r,id).unwrap(),"bleeding");assert_eq!(d::bleeding_packet(44.,engine::actor(&r,id).unwrap()),(0.,true));
    let before=engine::actor(&r,id).unwrap().conditions.clone();d::apply_bleeding(&mut r,&a,id,0.,100.);assert_eq!(engine::actor(&r,id).unwrap().conditions,before);
}
#[test]
fn foreground_and_offscreen_actual_ticks_share_damage_and_duration() {
    for revision in [43.,44.] {
        let mut foreground=room(revision);let id=npc(&mut foreground);
        engine::actor_mut(&mut foreground,id).unwrap().conditions.insert("bleeding".into(),10.);
        let mut off=foreground.clone();
        for tick in 1..=10 {
            engine::dispatch(&mut foreground,&Command::Wait).unwrap();
            off.tick=tick as f64;corpse_core::offscreen::catch_up_floor(&mut off,1.).unwrap();
            let a=engine::actor(&foreground,id).unwrap();let b=engine::actor(&off,id).unwrap();
            assert!((a.hp-b.hp).abs()<1e-9,"revision={revision} tick={tick}: {} vs {}",a.hp,b.hp);
            assert_eq!(engine::condition(a,"bleeding"),engine::condition(b,"bleeding"));
        }
    }
}
#[test]
fn haste_free_move_does_not_tick_bleed_and_save_replay_preserve_stack_rules() {
    let mut r=room(44.);r.player.conditions.insert("bleeding".into(),10.);
    r.player.conditions.insert("hasted".into(),10.);r.phase="haste".to_string().into();
    let out=engine::dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap();
    assert!(!out.took_turn);assert_eq!(r.player.hp,100.);assert_eq!(r.player.conditions["bleeding"],10.);
    r.player.conditions.shift_remove("hasted");r.phase=Field::Missing;r.replay.clear();
    let initial=r.clone();let mut e=corpse_core::temporal::Engine::new(r).unwrap();
    for _ in 0..6 {e.dispatch(&Command::Wait).unwrap();}
    assert_eq!(e.run.player.conditions["bleeding"],4.);
    let saved=e.save().unwrap();assert_eq!(corpse_core::temporal::Engine::load(&saved).unwrap().save().unwrap(),saved);
    let record=corpse_core::replay::ReplayRecord::try_from_run(&e.run).unwrap();
    let mut replay=corpse_core::replay::ReplaySession::from_initial(initial,record).unwrap();
    while replay.step().unwrap(){}assert_eq!(replay.engine.save().unwrap(),saved);
}
