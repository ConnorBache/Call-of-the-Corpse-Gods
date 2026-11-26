use corpse_core::{ Command, Field, Run, dreamwalking as d, engine, phantasmii_generation as p, storage};
fn room()->Run {
 let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
 r.floors.truncate(1);r.actors.retain(|k,_|k=="1");r.ground.retain(|k,_|k=="1");
 r.simulation_revision=34.0.into();r.actors_at_mut(1.).clear();r.ground_at_mut(1.).clear();r.inventory.clear();
 r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.x=8.;r.player.y=8.;
 r.player.hp=500.;r.player.max_hp=500.;r.player.energy=1000.;r.player.stats.max_energy=1000.;r.next_id=1000.;r.offers.clear();r.phase=Field::Missing;
 let f=r.floor_mut().unwrap();f.environment=Field::Missing;f.vents=Field::Missing;f.permanent_fire=Field::Missing;f.godbeast_event=Field::Missing;f.doors=Field::Missing;
 let w=f.width();let h=f.height();for y in 1..h-1{for x in 1..w-1{f.tiles[y*w+x]="floor".into();}}
 f.tiles[w+1]="up".into();f.tiles[(h-2)*w+w-2]="down".into();r
}
fn start(r:&mut Run) {
 r.dreamwalking=d::State{unlocked:true,..Default::default()}.into();
 assert_eq!(d::ability(r,d::DREAMWALK_ID).unwrap(),Some(true));
}
fn add(r:&mut Run,night:bool,x:f64,y:f64)->f64 {
 let id=r.next_id;r.next_id+=1.;let mut a=if night{p::nightmare(1,id,x,y)}else{corpse_core::population::create_actor_revision("rat",id,x,y,false,34.).unwrap()};a.state="hunting".into();r.actors_at_mut(1.).push(a);id
}
fn ability(id:&str)->Command{Command::Ability{id:id.into(),target:Field::Missing,x:Field::Missing,y:Field::Missing}}
#[test]fn begin_body_anchor_and_wake_are_persistent(){let mut r=room();start(&mut r);let id=d::body(&r).unwrap().id;assert_eq!((d::body(&r).unwrap().x,d::body(&r).unwrap().y),(8.,8.));assert!(engine::flying_in(&r,&r.player));r.player.x=12.;let saved=storage::serialize(&r).unwrap();let mut r=storage::deserialize(&saved).unwrap();assert!(d::active(&r));assert!(d::wake(&mut r));assert_eq!((r.player.x,r.player.y),(8.,8.));assert!(engine::actor(&r,id).is_none());assert!(d::unlocked(&r));}
#[test]fn projection_overlaps_normal_enemy_without_attack(){let mut r=room();start(&mut r);let id=add(&mut r,false,9.,8.);let hp=engine::actor(&r,id).unwrap().hp;assert!(engine::move_actor(&mut r,0.,1.,0.,false).unwrap());assert_eq!(r.player.x,9.);assert_eq!(engine::actor(&r,id).unwrap().hp,hp);}
#[test]fn plane_attacks_and_body_damage_notify_without_waking(){let mut r=room();let normal=add(&mut r,false,9.,8.);start(&mut r);let body=d::body(&r).unwrap().id;let hp=r.player.hp;engine::damage_environment(&mut r,0.,10.,Some(normal),"attack",true).unwrap();assert_eq!(r.player.hp,hp);engine::damage_environment(&mut r,body,10.,Some(normal),"attack",true).unwrap();assert_eq!(r.player.hp,hp-10.);assert!(d::active(&r));assert!(r.log.iter().any(|s|s.contains("sleeping body takes")));}
#[test]fn nightmares_hidden_awake_seen_dream_and_leave_nothing(){let mut r=room();let id=add(&mut r,true,10.,8.);assert!(!corpse_core::vision::visible_actor(&r,engine::actor(&r,id).unwrap()));start(&mut r);assert!(corpse_core::vision::visible_actor(&r,engine::actor(&r,id).unwrap()));assert!(!corpse_core::vision::directly_visible_actor(&r,engine::actor(&r,id).unwrap()));let exp=r.exp;engine::actor_mut(&mut r,id).unwrap().hp=0.;engine::resolve_death(&mut r,id,Some(0.),None,true).unwrap();assert!(engine::actor(&r,id).is_none());assert!(r.ground["1"].is_empty());assert_eq!(r.exp,exp);}
#[test]fn rejected_travel_and_consumption_are_turnless(){let mut r=room();start(&mut r);for c in [Command::Descend,Command::Ascend,Command::Use{id:999.},Command::UseAt{id:999.,x:8.,y:8.}]{let t=r.tick;let out=engine::dispatch(&mut r,&c).unwrap();assert!(!out.took_turn);assert_eq!(r.tick,t);assert!(d::active(&r));}assert!(r.log.iter().any(|s|s==d::STAIRS_MESSAGE));}
#[test]fn dream_floor_mutations_do_not_change_waking_floor_and_restore_on_error(){let mut r=room();start(&mut r);let at=9*79+9;let normal=r.floor().unwrap().tiles[at].clone();let result:Result<(),&str>=d::with_plane(&mut r,true,|r|{r.floor_mut().unwrap().tiles[at]="chasm".into();Err("test")});assert!(result.is_err());assert!(!d::context_dream(&r));assert_eq!(r.floor().unwrap().tiles[at],normal);assert_eq!(d::floor_for(&r,&r.player).unwrap().tiles[at],"chasm");}
#[test]fn half_here_keeps_creation_plane_across_sleep_and_wake(){let mut r=room();r.player.abilities.push(corpse_core::half_here::ID.into());r.player.active.push(corpse_core::half_here::ID.into());let clone=corpse_core::half_here::create(&mut r,10.,8.).unwrap();start(&mut r);assert!(!d::dream_actor(&r,engine::actor(&r,clone).unwrap()));assert!(corpse_core::half_here::clone_id(&r).is_some());d::wake(&mut r);start(&mut r);let clone=corpse_core::half_here::create(&mut r,12.,8.).unwrap();assert!(d::dream_actor(&r,engine::actor(&r,clone).unwrap()));d::wake(&mut r);assert!(d::dream_view(&r));assert!(d::dream_actor(&r,engine::actor(&r,clone).unwrap()));}
#[test]fn blood_costs_still_apply_in_dream(){let mut r=room();start(&mut r);let hp=r.player.hp;engine::damage_environment(&mut r,0.,10.,None,"blood-letting",true).unwrap();assert_eq!(r.player.hp,hp-10.);}
#[test]fn real_dispatch_wait_keeps_body_and_projection_valid(){let mut r=room();start(&mut r);r.player.x=12.;engine::dispatch(&mut r,&Command::Wait).unwrap();assert!(d::active(&r));assert_eq!(d::body(&r).unwrap().x,8.);assert_eq!(r.player.x,12.);engine::dispatch(&mut r,&ability(d::DREAMWALK_ID)).unwrap();assert!(!d::active(&r));assert_eq!(r.player.x,8.);}
#[test]
fn debug_seed_replay_rejects_unrecorded_stash_edits_with_session_guidance() {
 let r=corpse_core::debug_floor::create_run("r34-debug-replay", "Avatar",1).unwrap();
 let record=corpse_core::replay::ReplayRecord::from_run(&r);
 let err=corpse_core::replay::ReplaySession::create(record,true).err().expect("DEBUG is session-based");
 assert!(err.to_string().contains("saved session"));
 assert!(storage::deserialize(&storage::serialize(&r).unwrap()).is_ok());
}

#[test]
fn current_dreamwalking_does_not_stomp_on_arrival_or_wait() {
    let mut r=room();
    r.simulation_revision=52.0.into();
    r.death_gas_rules_from=0usize.into();
    let id=r.next_id;
    r.next_id+=1.;
    let mut rat=corpse_core::population::create_actor_revision("rat",id,9.,8.,false,52.).unwrap();
    rat.hp=0.;
    r.actors_at_mut(r.depth).push(rat);
    engine::death_if_nonpositive(&mut r,id).unwrap();
    let heart=r.ground["1"].iter().find(|g|g.item.kind=="heart").unwrap().item.id;
    let corpse=engine::actor_mut(&mut r,id).unwrap();
    corpse.corpse_age=32.0.into();
    corpse.conditions.insert("monsterPhoenixRolls".into(),3.);
    start(&mut r);
    for command in [Command::Move{dx:1.,dy:0.},Command::Wait] {
        assert!(engine::dispatch(&mut r,&command).unwrap().took_turn);
        let corpse=engine::actor(&r,id).unwrap();
        assert!(corpse.corpse_age.value_or(0.)>=32.);
        assert_eq!(corpse.conditions.get("monsterPhoenixRolls"),Some(&3.));
        assert!(!corpse.conditions.contains_key("corpseStomped"));
        assert!(r.ground["1"].iter().any(|g|g.item.id==heart));
        assert!(!r.inventory.iter().any(|i|i.id==heart));
        assert!(!r.log.iter().any(|s|s.contains("stomp on")));
    }
    assert_eq!((r.player.x,r.player.y),(9.,8.));
    assert!(d::wake(&mut r));
    assert!(engine::dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap().took_turn);
    let corpse=engine::actor(&r,id).unwrap();
    assert!(corpse.corpse_age.value_or(99.)<=1.);
    assert!(!corpse.conditions.contains_key("monsterPhoenixRolls"));
    assert!(corpse_core::corpses::heart_harvested(corpse));
    assert!(r.inventory.iter().any(|i|i.id==heart));
    assert!(r.log.iter().any(|s|s.contains("stomp on")));
}

#[test]
fn historical_dreamwalking_replay_without_new_policy_keeps_stomp() {
    for revision in [34.,52.] {
        let mut r=room();
        r.simulation_revision=revision.into();
        let id=add(&mut r,false,9.,8.);
        let corpse=engine::actor_mut(&mut r,id).unwrap();
        corpse.hp=0.;
        corpse.dead=true.into();
        corpse.corpse_age=32.0.into();
        start(&mut r);
        let player=r.player.id;
        assert!(engine::move_actor(&mut r,player,1.,0.,false).unwrap());
        assert_eq!(engine::actor(&r,id).unwrap().corpse_age.as_ref(),Some(&0.));
    }
}
