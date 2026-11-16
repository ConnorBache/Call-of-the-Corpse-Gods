use corpse_core::{Actor, Command, Field, Run, engine, population, storage, temporal};
fn room(revision:f64)->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=revision.into();r.player.x=10.;r.player.y=10.;r.player.hp=10000.;r.player.max_hp=10000.;r.player.base_max_hp=10000.0.into();
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.stats.dodge=-1000.;r.player.stats.defense=0.;r.player.stats.dr=0.;
    r.inventory.clear();r.offers.clear();r.actors=Default::default();r.ground.clear();r.history.clear();r.replay.clear();r.future=None;r.haste_snapshot=Field::Missing;r.phase=Field::Missing;r.tick=10.;
    let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.seen.fill(true);f.environment=Field::Missing;f.doors=Field::Missing;f.vents=Field::Missing;f.currents=Field::Missing;f.mud=Field::Missing;f.slime=Field::Missing;f.roots=Field::Missing;f.ichor=Field::Missing;f.fungal_roots=Field::Missing;f.portals=Field::Missing;
    for depth in r.floors.iter().map(|f|f.depth).collect::<Vec<_>>() {r.actors_at_mut(depth);r.ground_at_mut(depth);}
    r
}
fn add(r:&mut Run,kind:&str,x:f64,y:f64)->f64 {
    let id=r.next_id;r.next_id+=1.;let mut a=population::create_actor_revision(kind,id,x,y,false,r.simulation_revision.value_or(29.)).unwrap();
    a.state="hunting".into();a.last_seen=corpse_core::ActorLastSeen{x:r.player.x,y:r.player.y}.into();a.conditions.insert("targetId".into(),r.player.id);a.stats.perception=1000.;a.stats.accuracy=1000.;a.stats.offense=0.;a.stats.dr=0.;a.stats.defense=0.;a.hp=10000.;a.max_hp=10000.;a.base_max_hp=10000.0.into();r.actors_at_mut(r.depth).push(a);id
}
fn act(r:&mut Run,c:Command) {engine::dispatch(r,&c).unwrap();}
fn strikes(out:&engine::DispatchOutcome)->usize {out.events.iter().filter(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="shared"&&a.stage==corpse_core::events::Stage::Impact)).count()}
fn a(r:&Run,id:f64)->&Actor {engine::actor(r,id).unwrap()}
#[test]
fn cave_bat_catches_running_player_and_hits_after_first_step() {
    let mut r=room(52.);let bat=add(&mut r,"cave-bat",9.,10.);
    for x in 11..=15 {let out=engine::dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap();assert_eq!(r.player.x,x as f64);assert_eq!(a(&r,bat).x,(x-1)as f64);assert_eq!(strikes(&out),1,"bat must move then attack on flee step {x}");}
}
#[test]
fn fast_two_moves_or_one_attack_and_revision51_compatibility() {
    for revision in [51.,52.] {let mut r=room(revision);let bat=add(&mut r,"cave-bat",6.,10.);act(&mut r,Command::Wait);assert_eq!(a(&r,bat).x,8.);}
    let mut fast=room(52.);let id=add(&mut fast,"cave-bat",9.,10.);let mut ordinary=fast.clone();engine::actor_mut(&mut ordinary,id).unwrap().abilities.retain(|s|s!="enemy:beast:fast");
    act(&mut fast,Command::Wait);act(&mut ordinary,Command::Wait);assert_eq!(fast.player.hp,ordinary.player.hp,"Fast never adds a second attack when adjacent");assert_eq!(fast.rng,ordinary.rng);
}
#[test]
fn haste_has_two_complete_attacks_and_fast_composes_per_action() {
    let mut one=room(52.);let id=add(&mut one,"cave-bat",9.,10.);let mut two=one.clone();engine::actor_mut(&mut two,id).unwrap().conditions.insert("hasted".into(),5.);
    let single=engine::dispatch(&mut one,&Command::Wait).unwrap();let double=engine::dispatch(&mut two,&Command::Wait).unwrap();assert_eq!(strikes(&single),1);assert_eq!(strikes(&double),2);assert_eq!(engine::condition(a(&two,id),"hasted"),4.);
    let mut r=room(52.);let id=add(&mut r,"cave-bat",4.,10.);engine::actor_mut(&mut r,id).unwrap().conditions.insert("hasted".into(),5.);act(&mut r,Command::Wait);assert_eq!(a(&r,id).x,8.,"two complete Fast actions give four moves");
}
#[test]
fn slowed_and_innate_slow_suppress_fast_and_haste_bonuses() {
    for innate in [false,true] {for tick in [10.,11.] {let mut r=room(52.);r.tick=tick;let id=add(&mut r,"cave-bat",5.,10.);let enemy=engine::actor_mut(&mut r,id).unwrap();enemy.conditions.insert("hasted".into(),5.);if innate {enemy.abilities.push("enemy:beast:slow".into());}else{enemy.conditions.insert("slowed".into(),5.);}
        act(&mut r,Command::Wait);assert_eq!(a(&r,id).x,if tick==10.{6.}else{5.});}}
}
#[test]
fn blocked_and_held_first_moves_do_not_earn_a_fast_followup() {
    let mut r=room(52.);let id=add(&mut r,"cave-bat",5.,10.);let w=r.floor().unwrap().width();for y in 0..r.floor().unwrap().height(){r.floor_mut().unwrap().tiles[y*w+6]="wall".into();}act(&mut r,Command::Wait);assert_eq!(a(&r,id).x,5.);
    let mut r=room(52.);let id=add(&mut r,"horn-2",5.,10.);let n=r.floor().unwrap().tiles.len();let w=r.floor().unwrap().width();let mut mud=vec![0.;n];mud[10*w+5]=1.;r.floor_mut().unwrap().mud=mud.into();act(&mut r,Command::Wait);assert_eq!(a(&r,id).x,5.);assert!(engine::condition(a(&r,id),"stuck")>0.);
}
#[test]
fn player_true_haste_wait_and_attack_are_complete_bonus_actions() {
    let mut r=room(52.);r.player.conditions.insert("hasted".into(),5.);let tick=r.tick;act(&mut r,Command::Wait);assert_eq!(r.tick,tick);assert_eq!(r.phase.as_ref().unwrap(),"normal");act(&mut r,Command::Wait);assert_eq!(r.tick,tick+1.);
    let mut r=room(52.);r.player.conditions.insert("hasted".into(),5.);r.player.stats.accuracy=1000.;let id=add(&mut r,"rat",11.,10.);let hp=a(&r,id).hp;let tick=r.tick;act(&mut r,Command::Move{dx:1.,dy:0.});assert!(a(&r,id).hp<hp);assert_eq!(r.tick,tick);act(&mut r,Command::Move{dx:1.,dy:0.});assert_eq!(r.tick,tick+1.);
}
#[test]
fn player_fast_haste_four_inputs_save_and_snapshot_every_phase() {
    let mut r=room(52.);r.player.abilities.push("enemy:beast:fast".into());r.player.conditions.insert("hasted".into(),5.);let tick=r.tick;
    for step in 1..=4 {act(&mut r,Command::Move{dx:1.,dy:0.});assert_eq!(r.player.x,10.+step as f64);assert_eq!(r.tick,tick+if step==4{1.}else{0.});
        assert_eq!(r.fast_action_phase.as_ref().is_some(),step==1||step==3);
        let encoded=storage::serialize(&r).unwrap();let loaded=storage::deserialize_compatible(&encoded).unwrap();assert_eq!(loaded.fast_action_phase,r.fast_action_phase);assert_eq!(loaded.phase,r.phase);
        let snapshot=temporal::Snapshot::capture(&r);let mut restored=r.clone();restored.fast_action_phase=Field::Missing;snapshot.restore(&mut restored).unwrap();assert_eq!(restored.fast_action_phase,r.fast_action_phase);
        if step<4 {let c=Command::Move{dx:1.,dy:0.};let mut direct=r.clone();let mut resumed=loaded;act(&mut direct,c.clone());act(&mut resumed,c);assert_eq!(direct.player,resumed.player);assert_eq!(direct.tick,resumed.tick);assert_eq!(direct.phase,resumed.phase);assert_eq!(direct.fast_action_phase,resumed.fast_action_phase);}
    }
}
#[test]
fn player_fast_preserves_move_then_attack_and_blocked_command_credit() {
    let mut r=room(52.);r.player.abilities.push("enemy:beast:fast".into());let tick=r.tick;let id=add(&mut r,"rat",12.,10.);act(&mut r,Command::Move{dx:1.,dy:0.});assert_eq!(r.tick,tick);let saved=r.fast_action_phase.clone();act(&mut r,Command::Move{dx:2.,dy:0.});assert_eq!(r.fast_action_phase,saved);r.player.stats.accuracy=1000.;let hp=a(&r,id).hp;act(&mut r,Command::Move{dx:1.,dy:0.});assert_eq!(r.tick,tick+1.);assert!(a(&r,id).hp<hp);assert!(r.fast_action_phase.is_missing());
}

#[test]
fn future_restores_each_fast_phase_and_clears_a_later_marker() {
    for steps in 0..=3 {
        let mut r=room(52.);r.player.abilities=vec!["enemy:beast:fast".into(),"morae-see-the-future".into()];r.player.conditions.insert("hasted".into(),9.);r.player.energy=100.;r.player.stats.max_energy=100.;
        corpse_core::revision50_ability_energy::reconcile_acquired(&mut r).unwrap();
        let mut e=temporal::Engine::new(r).unwrap();for _ in 0..steps {e.dispatch(&Command::Move{dx:1.,dy:0.}).unwrap();}
        let (phase,fast,x,tick)=(e.run.phase.clone(),e.run.fast_action_phase.clone(),e.run.player.x,e.run.tick);
        let future=Command::Ability{id:"morae-see-the-future".into(),target:Field::Missing,x:Field::Missing,y:Field::Missing};
        e.dispatch(&future).unwrap();assert!(e.run.future.is_some());e.dispatch(&Command::Move{dx:1.,dy:0.}).unwrap();e.dispatch(&future).unwrap();
        assert!(e.run.future.is_none());assert_eq!((e.run.phase.clone(),e.run.fast_action_phase.clone(),e.run.player.x,e.run.tick),(phase,fast,x,tick));
    }
}
#[test]
fn legacy_or_malformed_fast_markers_are_rejected_and_missing_phase_loads() {
    for revision in [51.,52.] {let r=room(revision);let raw=storage::serialize(&r).unwrap();assert!(storage::deserialize_compatible(&raw).is_ok());
        let mut value=serde_json::to_value(&r).unwrap();value["fastActionPhase"]="haste".into();value["phase"]="haste".into();assert!(storage::deserialize_compatible(&value.to_string()).is_err());
        value["phase"]="normal".into();assert_eq!(storage::deserialize_compatible(&value.to_string()).is_ok(),revision>=52.);
    }
}
#[test]
fn lethal_enemy_haste_does_not_execute_dead_players_requested_move() {
    let mut r=room(52.);r.player.hp=1.;let id=add(&mut r,"cave-bat",9.,10.);engine::actor_mut(&mut r,id).unwrap().conditions.insert("hasted".into(),9.);
    act(&mut r,Command::Move{dx:1.,dy:0.});assert_ne!(r.status,"playing");assert_eq!(r.player.x,10.);
}
#[test]
fn yeti_slows_before_player_haste_eligibility_is_sampled() {
    let mut r=room(52.);r.player.conditions.insert("hasted".into(),9.);add(&mut r,"yeti",9.,10.);let tick=r.tick;act(&mut r,Command::Wait);assert!(r.tick>tick);assert_ne!(r.phase.as_ref().map(String::as_str),Some("normal"));
}

#[test]
fn fast_portal_move_is_not_limited_to_timeweave_edges_and_resumes_exactly() {
    for haste in [false,true] {for weave in [false,true] {
        let mut r=room(52.);
        // Persisted portals are admitted from floor ten onward. Keep a complete
        // valid floor/actor/ground sequence rather than bypassing that contract.
        let template=r.floor().unwrap().clone();
        while r.floors.len()<10 {let depth=(r.floors.len()+1)as f64;let mut f=template.clone();f.depth=depth;r.floors.push(std::sync::Arc::new(f));r.actors_at_mut(depth);r.ground_at_mut(depth);}
        r.depth=10.;r.player.abilities.push("enemy:beast:fast".into());
        if haste {r.player.conditions.insert("hasted".into(),9.);}
        if weave {r.player.abilities.push(corpse_core::progression::WEAVE_ID.into());}
        let w=r.floor().unwrap().width();r.floor_mut().unwrap().portals=vec![corpse_core::portals_generation::PortalPair{a:10*w+11,b:10*w+30,color_index:0}].into();
        assert!(storage::deserialize_compatible(&storage::serialize(&r).unwrap()).is_ok(),"portal fixture must validate before the first command");
        let tick=r.tick;let trail=r.timeweave.clone();
        let out=engine::dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap();
        assert!(!out.took_turn);assert_eq!((r.player.x,r.player.y),(30.,10.));assert_eq!(r.tick,tick);
        assert_eq!(r.fast_action_phase.as_ref(),Some(&if haste {corpse_core::model::FastActionPhase::Haste}else{corpse_core::model::FastActionPhase::Normal}));
        assert_eq!(r.timeweave,trail,"portal movement must not create a local Timeweave edge");
        let mut resumed=storage::deserialize_compatible(&storage::serialize(&r).unwrap()).unwrap();
        for run in [&mut r,&mut resumed] {act(run,Command::Move{dx:1.,dy:0.});}
        assert_eq!(r.player,resumed.player);assert_eq!(r.phase,resumed.phase);assert_eq!(r.fast_action_phase,resumed.fast_action_phase);assert_eq!(r.tick,resumed.tick);assert_eq!(r.timeweave,resumed.timeweave);assert_eq!(r.rng,resumed.rng);
        assert_eq!(r.player.x,31.);assert_eq!(r.tick,tick+if haste{0.}else{1.});assert!(r.fast_action_phase.is_missing());
    }}
}
#[test]
fn same_floor_stair_and_flying_special_tile_moves_grant_fast_not_timeweave() {
    for tile in ["up","chasm","pit"] {
        let mut r=room(52.);r.player.abilities=vec!["enemy:beast:fast".into(),corpse_core::progression::WEAVE_ID.into()];
        if tile!="up" {r.player.conditions.insert("flying".into(),9.);}
        let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[10*w+11]=tile.into();let tick=r.tick;let trail=r.timeweave.clone();
        let out=engine::dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap();
        assert_eq!((r.depth,r.player.x,r.player.y),(1.,11.,10.),"{tile}");assert_eq!(r.status,"playing");assert!(!out.took_turn);assert_eq!(r.tick,tick);
        assert_eq!(r.fast_action_phase.as_ref(),Some(&corpse_core::model::FastActionPhase::Normal));assert_eq!(r.timeweave,trail);
    }
}
#[test]
fn fast_arrival_does_not_create_credit_after_floor_transfer_or_terminal_state() {
    for (tile,lethal,haste) in [("down",false,false),("down",false,true),("chasm",false,false),("chasm",true,false),("pit",false,false)] {
        let mut r=room(52.);r.player.abilities.push("enemy:beast:fast".into());if haste {r.player.conditions.insert("hasted".into(),9.);}
        let mut destination=r.floor().unwrap().clone();destination.depth=2.;destination.last_tick=r.tick;let w=destination.width();destination.tiles[10*w+10]="up".into();
        if let Some(f)=r.floors.iter_mut().find(|f|f.depth==2.) {*f=std::sync::Arc::new(destination);}else{r.floors.push(std::sync::Arc::new(destination));}
        r.floor_mut().unwrap().tiles[10*w+11]=tile.into();if lethal {r.player.hp=1.;}
        act(&mut r,Command::Move{dx:1.,dy:0.});
        assert!(r.fast_action_phase.is_missing(),"{tile}, lethal={lethal}, haste={haste}");
        if tile=="pit" {assert_eq!(r.status,"won");}else{assert_eq!(r.depth,2.);assert_eq!(r.status,if lethal{"dead"}else{"playing"});}
    }
}
#[test]
fn failed_or_forced_movement_and_attack_advance_do_not_mint_fast_credit() {
    let mut r=room(52.);r.player.abilities.push("enemy:beast:fast".into());let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[10*w+11]="wall".into();let tick=r.tick;
    act(&mut r,Command::Move{dx:1.,dy:0.});assert_eq!(r.player.x,10.);assert_eq!(r.tick,tick);assert!(r.fast_action_phase.is_missing());
    r.floor_mut().unwrap().tiles[10*w+11]="floor".into();r.player.conditions.insert("forcedMove".into(),1.);let id=r.player.id;
    assert!(engine::move_actor(&mut r,id,1.,0.,true).unwrap());assert_eq!(r.player.x,11.);assert!(r.fast_action_phase.is_missing());
    let mut r=room(52.);r.player.abilities=vec!["enemy:beast:fast".into(),corpse_core::revision50_combat::FORCE.into()];r.player.active=vec![corpse_core::revision50_combat::FORCE.into()];r.player.stats.accuracy=1000.;add(&mut r,"rat",11.,10.);let tick=r.tick;
    act(&mut r,Command::Move{dx:1.,dy:0.});assert_eq!(r.player.x,11.);assert_eq!(r.tick,tick+1.);assert!(r.fast_action_phase.is_missing());
}
