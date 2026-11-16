use super::*;
use crate::{temporal::{History, Snapshot}};
const TARGET: f64 = 1000.0;
const OTHER: f64 = 1001.0;
fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=52.0.into();r.next_id=2000.0;r.player.x=10.;r.player.y=10.;
    r.player.abilities=vec![DUEL.into()];r.player.active.clear();r.player.conditions.clear();
    r.player.ability_reserves=Field::Missing;r.inventory.clear();for block in r.actors.values_mut() {*block=std::sync::Arc::new(vec![]);}
    r.player.hp=1000.;r.player.max_hp=1000.;r.player.base_max_hp=1000.0.into();
    crate::revision50_ability_energy::initialize_reserve(&mut r.player,DUEL,100.,100.).unwrap();
    let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.portals=Field::Missing;f.non_euclidean=Field::Missing;
    f.environment=Field::Missing;f.doors=Field::Missing;f.grates=Field::Missing;
    for (id,x,y) in [(TARGET,11.,10.),(OTHER,10.,11.)] {
        let a=Actor{id,x,y,hp:100.,max_hp:100.,base_max_hp:100.0.into(),name:"Giant Rat".into(),creature_kind:"rat".to_string().into(),state:"hunting".into(),..Default::default()};
        r.actors_at_mut(r.depth).push(a);
    }
    assert_eq!(activate(&mut r,DUEL,&TARGET.into(),&Field::Missing,&Field::Missing).unwrap(),Some(false));
    assert!(active(&r.player));r
}
fn active(a:&Actor)->bool {a.active.iter().any(|s|s==DUEL)}
fn ended(a:&Actor) {assert!(!active(a));for key in ["r50DuelTarget","r50DuelX","r50DuelY","r50DuelDepth"] {assert!(!a.conditions.contains_key(key),"{key}");}}
fn blocked(r:&Run)->bool {duel_blocks(r,engine::actor(r,OTHER).unwrap(),&r.player)}
fn copy_owner(r:&mut Run,id:f64) {
    let mut a=r.player.clone();a.id=id;a.state="ally".into();a.y=9.;
    r.actors_at_mut(r.depth).push(a);
}
#[test]
fn selected_target_uses_inclusive_anchored_square_not_owner_distance() {
    for dx in -2..=2 {for dy in -2..=2 {
        let mut r=room();let a=engine::actor_mut(&mut r,TARGET).unwrap();a.x=11.+dx as f64;a.y=10.+dy as f64;
        settle_duel(&mut r);assert!(active(&r.player));assert!(blocked(&r));
    }}
    for (x,y) in [(14.,10.),(8.,10.),(11.,13.),(11.,7.)] {
        let mut r=room();let a=engine::actor_mut(&mut r,TARGET).unwrap();a.x=x;a.y=y;
        assert!(!blocked(&r));settle_duel(&mut r);ended(&r.player);
    }
}
#[test]
fn ordinary_exit_is_final_before_reentry_and_does_not_charge_upkeep() {
    let mut r=room();let reserve=r.player.ability_reserves.as_ref().unwrap()[DUEL].energy;
    for _ in 0..3 {assert!(engine::move_actor(&mut r,TARGET,1.,0.,true).unwrap());}
    ended(&r.player);assert!(!blocked(&r));
    assert!(engine::move_actor(&mut r,TARGET,-1.,0.,true).unwrap());ended(&r.player);
    r.tick+=1.;let id=r.player.id;crate::abilities::pay_upkeep(&mut r,id).unwrap();
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()[DUEL].energy,reserve);
}
#[test]
fn stale_dead_missing_or_off_floor_target_has_no_immunity_or_upkeep() {
    for mode in 0..4 {
        let mut r=room();let energy=r.player.ability_reserves.as_ref().unwrap()[DUEL].energy;
        match mode {
            0=>engine::actor_mut(&mut r,TARGET).unwrap().hp=0.,
            1=>engine::actor_mut(&mut r,TARGET).unwrap().dead=true.into(),
            2=>r.actors_at_mut(r.depth).retain(|a|a.id!=TARGET),
            _=>{let a=engine::actor(&r,TARGET).unwrap().clone();r.actors_at_mut(r.depth).retain(|a|a.id!=TARGET);r.actors_at_mut(r.depth+1.).push(a);}
        }
        assert!(!blocked(&r));r.tick+=1.;let id=r.player.id;crate::abilities::pay_upkeep(&mut r,id).unwrap();
        ended(&r.player);assert_eq!(r.player.ability_reserves.as_ref().unwrap()[DUEL].energy,energy);
    }
}
#[test]
fn direct_periodic_environment_and_explicit_death_clear_only_selected_target() {
    for mode in 0..4 {
        let mut r=room();let player=r.player.id;
        engine::damage_environment(&mut r,OTHER,1000.,None,"fire",true).unwrap();assert!(active(&r.player));
        match mode {
            0=>engine::damage_direct(&mut r,TARGET,1000.,player,"fire",true).unwrap(),
            1=>engine::damage_environment(&mut r,TARGET,1000.,Some(player),"poison",true).unwrap(),
            2=>engine::damage_environment(&mut r,TARGET,1000.,None,"fire",true).unwrap(),
            _=>{engine::actor_mut_trusted(&mut r,TARGET).unwrap().hp=0.;engine::resolve_death(&mut r,TARGET,None,None,true).unwrap();}
        }
        ended(&r.player);
    }
}
#[test]
fn successful_death_prevention_does_not_end_duel() {
    let mut r=room();let a=engine::actor_mut(&mut r,TARGET).unwrap();
    a.abilities.push(crate::divine_rescues::PELIAN.into());a.active.push(crate::divine_rescues::PELIAN.into());
    engine::damage_environment(&mut r,TARGET,1000.,None,"fire",true).unwrap();
    assert!(engine::actor(&r,TARGET).unwrap().hp>0.);assert!(active(&r.player));
    engine::damage_environment(&mut r,TARGET,1000.,None,"fire",true).unwrap();ended(&r.player);
}
#[test]
fn independent_owners_and_shared_endpoints_end_together_without_coupling_reserves() {
    let mut r=room();copy_owner(&mut r,1002.);
    let before=r.player.ability_reserves.as_ref().unwrap()[DUEL].energy;
    // Warm candidate cache before shared endpoint insertion and owner departure.
    duel_actor_changed(&mut r,OTHER);
    r.player.abilities.push(crate::half_here::ID.into());
    let endpoint=crate::half_here::create(&mut r,18.,18.).unwrap();
    let source=engine::actor(&r,OTHER).unwrap().clone();
    assert!(duel_blocks(&r,&source,engine::actor(&r,endpoint).unwrap()));
    r.player.x=8.;let player=r.player.id;duel_actor_changed(&mut r,player);ended(&r.player);
    assert!(active(engine::actor(&r,1002.).unwrap()));
    for _ in 0..3 {engine::move_actor(&mut r,TARGET,1.,0.,true).unwrap();}
    ended(engine::actor(&r,1002.).unwrap());
    assert!(!duel_blocks(&r,&source,engine::actor(&r,endpoint).unwrap()));
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()[DUEL].energy,before);
}
#[test]
fn cache_rebuild_covers_public_mutation_birth_clone_load_and_removal() {
    let mut r=room();duel_actor_changed(&mut r,OTHER);assert!(r.actors.duel_owner_ids(r.player.id,r.depth).unwrap().is_empty());
    let template=r.player.clone();
    let a=engine::actor_mut(&mut r,OTHER).unwrap();a.active=template.active.clone();a.abilities=template.abilities.clone();a.conditions=template.conditions.clone();
    assert!(r.actors.duel_owner_ids(r.player.id,r.depth).unwrap().contains(&OTHER));
    copy_owner(&mut r,1002.);
    assert!(r.actors.duel_owner_ids(r.player.id,r.depth).unwrap().contains(&1002.));
    for mut world in [r.clone(),serde_json::from_str::<Run>(&serde_json::to_string(&r).unwrap()).unwrap()] {
        let a=engine::actor_mut(&mut world,TARGET).unwrap();a.x=14.;duel_actor_changed(&mut world,TARGET);
        ended(&world.player);ended(engine::actor(&world,OTHER).unwrap());ended(engine::actor(&world,1002.).unwrap());
    }
    r.actors_at_mut(r.depth).retain(|a|a.id!=TARGET);duel_actor_changed(&mut r,TARGET);
    ended(&r.player);ended(engine::actor(&r,OTHER).unwrap());
}
#[test]
fn push_and_teleport_commit_expiry_at_arrival() {
    let mut r=room();crate::revision50_wave_teleport::push_steps(&mut r,TARGET,1,0,3).unwrap();ended(&r.player);
    let mut r=room();
    // Teleport uses deterministic serialized RNG. Every eligible destination is
    // outside the Duel, so this checks the production arrival rather than a seed guess.
    let f=r.floor_mut().unwrap();let w=f.width();f.tiles.fill("wall".into());f.tiles[20*w+20]="floor".into();
    crate::revision50_wave_teleport::teleport(&mut r,TARGET).unwrap();ended(&r.player);
}
#[test]
fn snapshot_and_future_restore_their_own_live_target_state() {
    let mut r=room();copy_owner(&mut r,1002.);let saved=Snapshot::capture(&r);
    let anchor=crate::future_anchor::FutureAnchor::capture41(&r).unwrap();
    engine::actor_mut(&mut r,TARGET).unwrap().x=14.;duel_actor_changed(&mut r,TARGET);ended(&r.player);
    saved.restore(&mut r).unwrap();assert!(active(&r.player));assert!(blocked(&r));assert!(active(engine::actor(&r,1002.).unwrap()));
    engine::actor_mut(&mut r,TARGET).unwrap().x=14.;duel_actor_changed(&mut r,TARGET);
    r.future=Some(anchor);assert!(crate::temporal::restore_future(&mut r,&mut History::default()).unwrap());assert!(active(&r.player));assert!(blocked(&r));
}
#[test]
fn old_revisions_keep_owner_only_behavior() {
    for revision in [50.,51.] {
        let mut r=room();r.simulation_revision=revision.into();engine::actor_mut(&mut r,TARGET).unwrap().dead=true.into();
        let before=r.clone();duel_actor_changed(&mut r,TARGET);settle_current_duels(&mut r);assert_eq!(r,before);
        settle_duel(&mut r);assert!(active(&r.player));assert!(blocked(&r));
    }
}
fn clear_floor(r:&mut Run,depth:f64) {
    let f=r.floors.iter_mut().find(|f|f.depth==depth).unwrap();let f=crate::mutation_journal::make_mut(f);
    f.tiles.fill("floor".into());f.lights.fill(100.);f.environment=Field::Missing;f.portals=Field::Missing;
    f.non_euclidean=Field::Missing;f.doors=Field::Missing;f.grates=Field::Missing;
    f.dirty=true;f.last_tick=0.;
}
#[test]
fn target_fall_ends_source_floor_player_and_independent_duels() {
    let mut r=room();copy_owner(&mut r,1002.);clear_floor(&mut r,2.);
    let f=r.floor_mut().unwrap();let at=10*f.width()+11;f.tiles[at]="chasm".into();
    let depth=r.depth;crate::travel::fall_actors_one_level(&mut r,depth,&[TARGET],None).unwrap();
    assert_eq!(r.depth,1.);assert!(r.actors_at(2.).iter().any(|a|a.id==TARGET));
    ended(&r.player);ended(engine::actor(&r,1002.).unwrap());
}
#[test]
fn player_floor_travel_preserves_existing_owner_exit_rule() {
    let mut r=room();clear_floor(&mut r,2.);
    assert!(crate::travel::travel(&mut r,1.,false,true).unwrap());
    assert_eq!(r.depth,2.);ended(&r.player);
}
#[test]
fn dreamwalking_body_uses_owner_and_target_death_clears_it() {
    let mut r=room();r.dreamwalking=crate::dreamwalking::State{unlocked:true,..Default::default()}.into();
    crate::dreamwalking::ability(&mut r,crate::dreamwalking::DREAMWALK_ID).unwrap();
    let body=crate::dreamwalking::body(&r).unwrap().id;
    let source=engine::actor(&r,OTHER).unwrap().clone();
    assert!(duel_blocks(&r,&source,engine::actor(&r,body).unwrap()));
    engine::damage_environment(&mut r,TARGET,1000.,None,"fire",true).unwrap();ended(&r.player);
    assert!(!duel_blocks(&r,&source,engine::actor(&r,body).unwrap()));
}
#[test]
fn nightmare_removal_ends_duel_without_a_corpse() {
    let mut r=room();engine::actor_mut(&mut r,TARGET).unwrap().conditions.insert("phantasmiiNightmare".into(),1.);
    engine::damage_environment(&mut r,TARGET,1000.,None,"fire",true).unwrap();
    assert!(engine::actor(&r,TARGET).is_none());ended(&r.player);
}
#[test]
fn offscreen_target_death_copyback_clears_the_actual_player_duel() {
    let mut r=room();clear_floor(&mut r,2.);
    let mut target=engine::actor(&r,TARGET).unwrap().clone();target.hp=0.1;target.state="sleeping".into();target.conditions.insert("bleeding".into(),100.);
    let energy=r.player.ability_reserves.as_ref().unwrap()[DUEL].energy;
    // Deliberately retain an old off-floor target link to exercise actual-world
    // copyback, without introducing a separately unsupported offscreen owner.
    r.actors_at_mut(1.).retain(|a|a.id!=TARGET);r.actors_at_mut(2.).push(target);
    assert!(!blocked(&r));r.tick=1.;crate::offscreen::catch_up_floor(&mut r,2.).unwrap();
    let target=r.actors_at(2.).iter().find(|a|a.id==TARGET).unwrap();assert!(target.dead.value_or(false));ended(&r.player);
    r.tick=2.;let id=r.player.id;crate::abilities::pay_upkeep(&mut r,id).unwrap();
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()[DUEL].energy,energy);
}
#[test]
fn independent_owner_upkeep_removes_a_stale_duel_without_another_charge() {
    let mut r=room();copy_owner(&mut r,1002.);r.tick=2.;
    let energy=engine::actor(&r,1002.).unwrap().ability_reserves.as_ref().unwrap()[DUEL].energy;
    engine::actor_mut(&mut r,TARGET).unwrap().hp=0.;
    crate::abilities::pay_upkeep(&mut r,1002.).unwrap();let owner=engine::actor(&r,1002.).unwrap();ended(owner);
    assert_eq!(owner.ability_reserves.as_ref().unwrap()[DUEL].energy,energy);
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()[DUEL].energy,energy);
}
#[test]
fn loaded_stale_duel_is_normalized_but_valid_world_and_legacy_are_preserved() {
    for revision in [51.,52.] {
        let mut r=room();r.simulation_revision=revision.into();
        let valid=crate::storage::deserialize(&crate::storage::serialize(&r).unwrap()).unwrap();assert!(active(&valid.player));
        engine::actor_mut(&mut r,TARGET).unwrap().x=14.;
        let raw=crate::storage::serialize(&r).unwrap();
        let strict=crate::storage::deserialize(&raw).unwrap();
        let compatible=crate::storage::deserialize_compatible(&raw).unwrap();
        if revision==52. {ended(&strict.player);ended(&compatible.player);}else{assert!(active(&strict.player));assert!(active(&compatible.player));}
    }
}
#[test]
fn haste_departure_and_replay_use_same_cleanup_before_next_turn() {
    let mut r=room();r.tick=1.;
    let a=engine::actor_mut(&mut r,TARGET).unwrap();a.x=13.;a.conditions.insert("hasted".into(),4.);
    a.conditions.insert("targetId".into(),OTHER);a.last_seen=crate::ActorLastSeen{x:15.,y:10.}.into();
    // Full Haste includes perception. Keep the intended ally strictly closer
    // than the player rather than relying on the old movement-only target lock.
    let other=engine::actor_mut(&mut r,OTHER).unwrap();other.x=15.;other.y=10.;other.state="ally".into();
    let energy=r.player.ability_reserves.as_ref().unwrap()[DUEL].energy;
    let mut replay=serde_json::from_str::<Run>(&serde_json::to_string(&r).unwrap()).unwrap();
    let command=crate::Command::Wait;
    assert!(engine::dispatch(&mut r,&command).unwrap().took_turn);
    assert!(engine::dispatch(&mut replay,&command).unwrap().took_turn);
    assert_eq!(engine::actor(&r,TARGET).unwrap().x,14.);
    ended(&r.player);assert_eq!(r,replay);
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()[DUEL].energy,energy);
}
#[test]
fn charm_status_or_plane_changes_alone_do_not_end_the_duel() {
    let mut r=room();let a=engine::actor_mut(&mut r,TARGET).unwrap();
    a.state="ally".into();a.conditions.insert("paralyzed".into(),2.);a.conditions.insert("phantasmiiPlane".into(),1.);
    settle_current_duels(&mut r);assert!(active(&r.player));
}
