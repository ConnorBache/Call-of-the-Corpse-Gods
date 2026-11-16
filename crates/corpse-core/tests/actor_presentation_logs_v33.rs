//! Ranged and status narration must obey the same observation identity as the map.
use corpse_core::{actor_presentation as presentation, engine, population, startup, Actor, Command, Field, Floor, Run};
use std::sync::Arc;

fn room() -> Run {
    let mut r = startup::new_run_shell_revision("0-5-11:skin-log-repro", "Unblessed", false, 33.).unwrap();
    r.player.x=2.; r.player.y=2.; r.player.abilities.clear(); r.player.active.clear();
    r.player.conditions.clear(); r.player.ability_order=Field::Missing;
    let w=79; let h=29; let mut tiles=vec!["wall".into();w*h];
    for y in 1..h-1 { for x in 1..w-1 { tiles[y*w+x]="floor".into(); } }
    let mut f=Floor {width:(w as f64).into(),height:(h as f64).into(),depth:1.,domain:"Dungeon".into(),
        tiles,lights:vec![100.;w*h],seen:vec![true;w*h],..Default::default()};
    corpse_core::environment::initialize(&mut f).unwrap(); r.floors=vec![Arc::new(f)];
    let mut a=population::create_actor_revision("skin-walker",4.,6.,2.,false,33.).unwrap(); a.state="sleeping".into();
    r.actors.clear(); r.actors.insert("1".into(),Arc::new(vec![a]));
    r.ground.clear(); r.ground.insert("1".into(),Arc::new(vec![])); r.next_id=95.; r.tick=3.;
    r.generated_ability_grants=Vec::new().into(); r.log.clear(); r.log_kinds=Vec::new().into();
    let mut bow=corpse_core::equipment::make_equipment("Bow",90.).unwrap(); bow.equipped="hand1".to_string().into();
    r.inventory=vec![bow,corpse_core::equipment::make_arrows(91.,5.)]; r
}
fn skin(r:&Run)->&Actor {engine::actor(r,4.).unwrap()}
fn shoot(r:&mut Run, hit:bool) -> corpse_core::engine::DispatchOutcome {
    r.player.conditions.insert(if hit {"lucky"} else {"unlucky"}.into(),1.);
    let (x,y)=(skin(r).x,skin(r).y);
    engine::dispatch(r,&Command::Shoot{id:90.,x,y}).unwrap()
}
#[test]
fn actual_bow_hit_and_miss_use_far_disguise_and_near_true_name() {
    for hit in [false,true] { for x in [6.,4.] {
        let mut r=room(); engine::actor_mut(&mut r,4.).unwrap().x=x;
        let expected=presentation::name(&r,skin(&r)).to_string();
        assert_eq!(expected=="Skin Walker",x==4.);
        let out=shoot(&mut r,hit); assert!(out.took_turn);
        let line=r.log.iter().find(|s|s.starts_with("You shoot ")).unwrap();
        assert!(line.starts_with(&format!("You shoot {expected}")),"{line}");
        if !hit { assert!(line.ends_with("but miss.")); }
        if x==6. { assert!(!r.log.iter().any(|s|s.contains("Skin Walker"))); }
        assert!(out.events.iter().all(|e|e.label.as_ref().is_none_or(|s|s!="Skin Walker")));
        assert_eq!(skin(&r).name,"Skin Walker"); assert_eq!(skin(&r).stats.dodge,92.);
    }}
}
#[test]
fn distinct_actors_status_damage_death_and_attacking_subjects_do_not_leak() {
    let mut r=room(); let second=population::create_actor_revision("skin-walker",5.,6.,4.,false,33.).unwrap();
    r.actors_at_mut(1.).push(second); let a=presentation::name(&r,skin(&r)).to_string();
    let b=presentation::name(&r,engine::actor(&r,5.).unwrap()).to_string(); assert_ne!(a,b);
    let before=r.clone();for _ in 0..3 {assert_eq!(presentation::name(&r,skin(&r)),a);assert_eq!(presentation::name(&r,engine::actor(&r,5.).unwrap()),b);}assert_eq!(r,before);
    let player=r.player.id;
    engine::damage_environment(&mut r,5.,1.,Some(player),"fire",false).unwrap();
    assert!(r.log.iter().any(|s|s==&format!("Fire deals 1 damage to {b}.")));
    let mut ally=population::create_actor_revision("rat",6.,7.,2.,false,33.).unwrap();ally.state="ally".into();r.actors_at_mut(1.).push(ally);
    engine::actor_mut(&mut r,4.).unwrap().conditions.insert("unlucky".into(),1.);
    engine::attack_ranged(&mut r,4.,6.,20.,1.,1.,false).unwrap();
    assert!(r.log.iter().any(|s|s.starts_with(&format!("{a} fires at "))));
    r.player.conditions.insert("lucky".into(),1.);
    engine::attack_ranged(&mut r,player,4.,10000.,4.,1.,true).unwrap();
    assert!(r.log.iter().any(|s|s.starts_with("You kill ") && s.contains(&a)));
    assert!(!r.log.iter().any(|s|s.contains("Skin Walker")),"{:?}",r.log);
    assert_eq!(skin(&r).name,"Skin Walker");assert!(skin(&r).dead.value_or(false));
}
#[test]
fn half_here_reveal_and_hallucination_use_shared_read_only_identity() {
    let mut r=room();let disguise=presentation::name(&r,skin(&r)).to_string();
    r.player.abilities.push(corpse_core::half_here::ID.into());r.player.active.push(corpse_core::half_here::ID.into());r.player.conditions.insert("halfHereId".into(),9.);
    let mut body=r.player.clone();body.id=9.;body.x=4.;body.y=3.;body.state="ally".into();
    body.conditions.insert("halfHereClone".into(),1.);body.conditions.insert("halfHereOwner".into(),r.player.id);r.actors_at_mut(1.).push(body);
    assert_eq!(presentation::name(&r,skin(&r)),"Skin Walker");
    let out=shoot(&mut r,false);assert!(out.took_turn);assert!(r.log.iter().any(|s|s=="You shoot Skin Walker, but miss."));
    let mut r=room();r.player.conditions.insert("hallucinating".into(),2.);let expected=presentation::name(&r,skin(&r)).to_string();
    let before=r.clone();assert!(presentation::apparent_kind(&r,skin(&r)).is_some());assert_eq!(r,before);
    shoot(&mut r,false);assert!(r.log.iter().any(|s|s==&format!("You shoot {expected}, but miss.")));assert!(!r.log.iter().any(|s|s.contains("Skin Walker")));
    assert_ne!(disguise,"Skin Walker");
}
#[test]
fn save_and_replay_preserve_display_labels_without_changing_real_identity() {
    let mut initial=room();initial.player.conditions.insert("unlucky".into(),1.);
    let expected=presentation::name(&initial,skin(&initial)).to_string();
    let restored=corpse_core::storage::deserialize(&corpse_core::storage::serialize(&initial).unwrap()).unwrap();assert_eq!(presentation::name(&restored,skin(&restored)),expected);
    let mut e=corpse_core::temporal::Engine::new(restored).unwrap();e.dispatch(&Command::Shoot{id:90.,x:6.,y:2.}).unwrap();
    let record=corpse_core::replay::ReplayRecord::from_run(&e.run);let mut replay=corpse_core::replay::ReplaySession::from_initial(initial,record).unwrap();
    while replay.step().unwrap() {} assert_eq!(replay.engine.run.log,e.run.log);assert_eq!(replay.engine.run.rng,e.run.rng);assert_eq!(replay.engine.run.actors,e.run.actors);
    assert!(e.run.log.iter().any(|s|s==&format!("You shoot {expected}, but miss.")));assert_eq!(skin(&e.run).name,"Skin Walker");
}
#[test]
fn legacy_labels_and_true_death_provenance_are_preserved() {
    for revision in [29.,30.,31.,32.] {
        let mut r=room();r.simulation_revision=revision.into();r.player.conditions.insert("hallucinating".into(),2.);
        assert_eq!(presentation::name(&r,skin(&r)),"Skin Walker");
        r.player.conditions.insert("unlucky".into(),1.);let player=r.player.id;
        engine::attack_ranged(&mut r,player,4.,20.,4.,1.,true).unwrap();
        assert!(r.log.iter().any(|s|s=="You shoot Skin Walker, but miss."));
    }
    for x in [6.,4.] { for physical in [false,true] {
        let mut r=room();r.player.hp=1.;engine::actor_mut(&mut r,4.).unwrap().x=x;
        let expected=presentation::name(&r,skin(&r)).to_string();let player=r.player.id;
        if physical {engine::actor_mut(&mut r,4.).unwrap().conditions.insert("lucky".into(),1.);
            engine::attack_ranged(&mut r,4.,player,10000.,x-2.,1.,true).unwrap();}
        else {engine::damage_environment(&mut r,player,10000.,Some(4.),"poison",false).unwrap();}
        let death=r.run_stats.as_ref().unwrap().death.as_ref().unwrap();
        assert_eq!(death.source_name.as_ref().map(String::as_str),Some("Skin Walker"));
        assert!(death.reason.contains(&expected),"{} expected {expected}",death.reason);
        assert_eq!(death.reason.contains("Skin Walker"),x==4.);
    }}
}

#[test]
fn revert_and_future_restore_observed_identity_and_existing_labels() {
    let mut r=room();r.player.conditions.insert("revertCharges".into(),3.);
    let expected=presentation::name(&r,skin(&r)).to_string();shoot(&mut r,false);
    let log=r.log.clone();let mut history=corpse_core::temporal::History::default();
    history.record_completed(corpse_core::temporal::Snapshot::capture(&r));
    r.tick+=1.;engine::actor_mut(&mut r,4.).unwrap().x=3.;
    history.closing=Some(corpse_core::temporal::Snapshot::capture(&r));
    assert!(corpse_core::temporal::revert(&mut r,&mut history).unwrap());
    assert_eq!(presentation::name(&r,skin(&r)),expected);assert!(r.log.iter().any(|s|log.contains(s)&&s.starts_with("You shoot ")));
    corpse_core::temporal::remember_future(&mut r,&history).unwrap();
    engine::actor_mut(&mut r,4.).unwrap().x=3.;
    assert!(corpse_core::temporal::restore_future(&mut r,&mut history).unwrap());
    assert_eq!(presentation::name(&r,skin(&r)),expected);assert_eq!(skin(&r).name,"Skin Walker");
}

#[test]
fn revision52_telepathy_overrides_hallucination_per_target_and_survives_save() {
    let mut r=room();r.simulation_revision=52.0.into();
    r.player.conditions.insert("hallucinating".into(),100.);
    r.player.conditions.insert("cameTurns".into(),10.);
    r.player.conditions.insert("cameTarget".into(),4.);
    let other=population::create_actor_revision("rat",5.,7.,2.,false,52.).unwrap();
    r.actors_at_mut(1.).push(other);
    let before=r.clone();
    assert_eq!(presentation::name(&r,skin(&r)),"Skin Walker");
    assert!(presentation::apparent_kind(&r,engine::actor(&r,5.).unwrap()).is_some(),"nearby sensed terrain does not reveal another identity");
    assert_eq!(r,before);
    let restored=corpse_core::storage::deserialize(&corpse_core::storage::serialize(&r).unwrap()).unwrap();
    assert_eq!(presentation::name(&restored,skin(&restored)),"Skin Walker");
    let player=r.player.id;
    engine::damage_environment(&mut r,4.,10.,Some(player),"fire",true).unwrap();
    assert!(r.log.iter().any(|s|s=="Fire deals 10 damage to Skin Walker."),"{:?}",r.log);
    r.player.conditions.shift_remove("cameTurns");
    assert!(presentation::apparent_kind(&r,skin(&r)).is_some());
}

#[test]
fn revision52_telepathy_sources_and_range_control_true_identity() {
    let mut base=room();base.simulation_revision=52.0.into();
    base.player.conditions.insert("hallucinating".into(),100.);
    for source in ["condition","mind-read","dream-see","half-here"] {
        let mut r=base.clone();
        match source {
            "condition"=>{r.player.conditions.insert("telepathic".into(),10.);},
            "mind-read"=>r.player.abilities.push("numo-mind-read".into()),
            "dream-see"=>r.player.abilities.push("ursa-dream-see".into()),
            _=>{
                r.player.abilities.push(corpse_core::half_here::ID.into());
                r.player.active.push(corpse_core::half_here::ID.into());
                r.player.conditions.insert("halfHereId".into(),9.);
                let mut body=r.player.clone();body.id=9.;
                body.conditions.insert("halfHereClone".into(),1.);
                body.conditions.insert("halfHereOwner".into(),r.player.id);
                body.conditions.insert("telepathic".into(),10.);r.actors_at_mut(1.).push(body);
            }
        }
        assert_eq!(presentation::name(&r,skin(&r)),"Skin Walker","{source}");
    }
    base.player.abilities.push("numo-mind-read".into());
    engine::actor_mut(&mut base,4.).unwrap().x=8.;
    assert!(presentation::apparent_kind(&base,skin(&base)).is_some());
    base.player.active.push("numo-mind-read".into());
    assert_eq!(presentation::name(&base,skin(&base)),"Skin Walker");
    engine::actor_mut(&mut base,4.).unwrap().dead=true.into();
    assert!(presentation::apparent_kind(&base,skin(&base)).is_some(),"dead creatures are not telepathic targets");
}
