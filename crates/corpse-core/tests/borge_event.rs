use corpse_core::{Command, Field, Run, borge, engine, generation, inventory, storage, temporal::{Engine, Snapshot}};
use std::sync::Arc;
fn room() -> Run {
    let mut r: Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.floors.truncate(1);r.actors.retain(|k,_|k=="1");r.ground.retain(|k,_|k=="1");
    r.simulation_revision=33.0.into();r.actors_at_mut(1.).clear();r.inventory.clear();r.ground_at_mut(1.).clear();
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.x=19.;r.player.y=14.;
    r.player.energy=200.;r.player.stats.max_energy=200.;r.phase=Field::Missing;r.offers.clear();
    let f=r.floor_mut().unwrap();f.environment=Field::Missing;f.vents=Field::Missing;f.permanent_fire=Field::Missing;
    f.godbeast_event=Field::Missing;f.godbeast=true;f.doors=Field::Missing;
    let w=f.width();let h=f.height();
    for y in 1..h-1 {for x in 1..w-1 {f.tiles[y*w+x]="floor".into();}}
    f.tiles[w+1]="up".into();f.tiles[(h-2)*w+w-2]="down".into();
    let e=borge::reserve_room(&mut f.tiles,&[]).unwrap();f.godbeast_event=e.into();
    r.next_id=1000.;r
}
fn add(r:&mut Run,kind:&str,x:f64,y:f64)->f64 {
    let id=r.next_id;r.next_id+=1.;
    let mut a=corpse_core::population::create_actor(kind,id,x,y,false).unwrap();
    a.state="sleeping".into();r.actors_at_mut(r.depth).push(a);id
}
fn summon(r:&mut Run)->f64 {
    let e=r.floor().unwrap().godbeast_event.as_ref().unwrap().clone();
    r.player.x=e.x-3.;r.player.y=e.y;
    assert!(borge::interact(r,e.x-2.,e.y).unwrap());
    r.floor().unwrap().godbeast_event.as_ref().unwrap().sacrifice_actor_id.value_or(-1.)
}
fn altar(r:&mut Run)->(f64,f64) {
    let p=r.floor().unwrap().godbeast_event.as_ref().unwrap().altar.as_ref().unwrap();let (x,y)=(p.x,p.y);
    r.player.x=x;r.player.y=y+1.;(x,y)
}
fn kill(r:&mut Run,id:f64) {
    engine::actor_mut(r,id).unwrap().hp=0.;engine::resolve_death(r,id,Some(r.player.id),None,true).unwrap();
}
fn take_marked(r:&mut Run)->f64 {
    let index=r.ground[&r.depth.to_string()].iter().position(|g|g.item.source_actor_id.as_ref().is_some()).unwrap();
    let item=r.ground_at_mut(r.depth).remove(index).item;let id=item.id;r.inventory.push(item);id
}
fn ability(id:&str)->Command {Command::Ability{id:id.into(),target:Field::Missing,x:Field::Missing,y:Field::Missing}}
#[test]
fn seven_exact_milestones_four_authored_types_and_legacy_plan_unchanged() {
    for seed in 0..80 {
        let legacy=generation::domain_plan(seed);
        for rev in [29.,30.,31.,32.] {assert_eq!(generation::domain_plan_revision(seed,rev),legacy);}
        let plan=generation::domain_plan_revision(seed,33.);
        assert_eq!(plan.iter().map(|p|p.godbeast_depth).collect::<Vec<_>>(),vec![5,10,15,20,25,30,35,-1]);
        assert_eq!(plan.iter().map(|p|&p.domain).collect::<Vec<_>>(),legacy.iter().map(|p|&p.domain).collect::<Vec<_>>());
        assert!(plan[7].godbeast_kind.is_empty());
        assert_eq!(plan[..4].iter().map(|p|p.godbeast_kind.as_str()).collect::<std::collections::BTreeSet<_>>(),["Borge","Mercuria","Veridian","Zillennia"].into_iter().collect());
        assert!(plan[..7].windows(2).all(|w|w[0].godbeast_kind!=w[1].godbeast_kind));
    }
}
#[test]
fn generated_borge_has_25_solid_cells_clear_accessible_ring_altar_and_stairs() {
    let seed=(0..1000).find(|s|generation::domain_plan_revision(*s,33.)[0].godbeast_kind=="Borge").unwrap();
    for domain in ["Dungeon","Cave","Meadow","Forest","Archipelago","Mountain"] {
        let f=generation::generate_floor_revision(5,seed,domain,true,79,None,33.).unwrap();
        let e=f.godbeast_event.as_ref().unwrap();assert_eq!(e.kind,"Borge");
        assert!((e.x-39.).abs()<=10. && (e.y-14.).abs()<=10.);
        assert_eq!(f.tiles.iter().enumerate().filter(|(i,_)|borge::solid_at(&f,(i%79) as f64,(i/79) as f64)).count(),25);
        for y in e.y as i32-2..=e.y as i32+2 {for x in e.x as i32-2..=e.x as i32+2 {assert_eq!(f.tiles[y as usize*79+x as usize],"wall");}}
        let reachable=generation::reachable_without_braziers(&f,None);
        let p=e.altar.as_ref().unwrap();assert!(reachable.contains(&(p.y as usize*79+p.x as usize)),"{domain}");
        for y in e.y as i32-3..=e.y as i32+3 {for x in e.x as i32-3..=e.x as i32+3 {if (x-e.x as i32).abs()==3 || (y-e.y as i32).abs()==3 {assert!(reachable.contains(&(y as usize*79+x as usize)),"{domain} ring {x},{y}");}}}
        assert!(generation::transitions_accessible(&f,&reachable));
    }
}
#[test]
fn strongest_is_alive_eligible_and_stable_hp_dr_defense_id_tiebreak() {
    let mut r=room();let a=add(&mut r,"rat",8.,8.);let b=add(&mut r,"rat",9.,8.);
    for id in [a,b] {let a=engine::actor_mut(&mut r,id).unwrap();a.max_hp=900.;a.stats.dr=40.;a.stats.defense=80.;}
    let dead=add(&mut r,"dragon",10.,8.);engine::actor_mut(&mut r,dead).unwrap().dead=true.into();
    let heartless=add(&mut r,"dragon",11.,8.);engine::actor_mut(&mut r,heartless).unwrap().heartless=true.into();
    let invulnerable=add(&mut r,"sphinx",12.,8.);engine::actor_mut(&mut r,invulnerable).unwrap().max_hp=10000.;
    let clone=add(&mut r,"rat",13.,8.);let c=engine::actor_mut(&mut r,clone).unwrap();c.max_hp=10000.;c.conditions.insert("miniMe".into(),1.);
    assert_eq!(borge::strongest(&r).unwrap().id,a);
    engine::actor_mut(&mut r,b).unwrap().stats.defense+=1.;assert_eq!(borge::strongest(&r).unwrap().id,b);
    engine::actor_mut(&mut r,a).unwrap().stats.dr+=1.;assert_eq!(borge::strongest(&r).unwrap().id,a);
    engine::actor_mut(&mut r,b).unwrap().max_hp+=1.;assert_eq!(borge::strongest(&r).unwrap().id,b);
    let hp=engine::actor(&r,b).unwrap().hp;let rng=r.rng;assert_eq!(summon(&mut r),b);
    assert_eq!(engine::actor(&r,b).unwrap().hp,hp);assert_eq!(r.rng,rng);assert!(r.log.iter().any(|s|s==borge::DEMAND));
    let e=r.floor().unwrap().godbeast_event.as_ref().unwrap().clone();engine::actor_mut(&mut r,a).unwrap().max_hp=99999.;
    assert!(!borge::interact(&mut r,e.x-2.,e.y).unwrap());assert_eq!(r.floor().unwrap().godbeast_event.as_ref().unwrap(),&e);
}
#[test]
fn exact_heart_not_same_species_one_shard_and_no_food_stats_when_offered() {
    let mut r=room();let a=add(&mut r,"rat",8.,8.);let b=add(&mut r,"rat",9.,8.);assert_eq!(summon(&mut r),a);
    kill(&mut r,b);let wrong=r.ground_at_mut(1.).remove(0).item;r.inventory.push(wrong);
    let (x,y)=altar(&mut r);assert!(!borge::interact(&mut r,x,y).unwrap());
    kill(&mut r,a);let heart=take_marked(&mut r);assert_eq!(r.inventory.iter().find(|i|i.id==heart).unwrap().source_actor_id.as_ref(),Some(&a));
    let stats=r.player.stats.clone();let exp=r.exp;let energy=r.player.energy;
    assert!(borge::interact(&mut r,x,y).unwrap());assert_eq!(r.player.stats,stats);assert_eq!(r.exp,exp);assert_eq!(r.player.energy,energy);
    assert!(!r.inventory.iter().any(|i|i.id==heart));assert_eq!(r.inventory.iter().filter(|i|borge::is_shard(i)).count(),1);
    assert_eq!(r.inventory.iter().find(|i|borge::is_shard(i)).unwrap().effect.as_ref().unwrap(),borge::SHARD_DESCRIPTION);
    assert!(!borge::interact(&mut r,x,y).unwrap());assert_eq!(r.inventory.len(),2);
    storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();
}
#[test]
fn marked_heart_is_still_food_and_cannot_reselect_after_consumption_or_empty_floor() {
    let mut r=room();let id=add(&mut r,"rat",8.,8.);summon(&mut r);kill(&mut r,id);let heart=take_marked(&mut r);
    let bonus=r.inventory[0].bonus.as_ref().unwrap().clone();let old=r.player.stats.clone();
    inventory::command(&mut r,&Command::Use{id:heart}).unwrap();
    assert_eq!(r.player.stats.offense,old.offense+bonus.offense.value_or(0.));
    let (x,y)=altar(&mut r);assert!(!borge::interact(&mut r,x,y).unwrap());assert!(!r.inventory.iter().any(borge::is_shard));
    let mut empty=room();assert_eq!(summon(&mut empty),-1.);add(&mut empty,"dragon",8.,8.);assert_eq!(borge::strongest(&empty).unwrap().name,"Dragon");
    let e=empty.floor().unwrap().godbeast_event.as_ref().unwrap().clone();assert!(!borge::interact(&mut empty,e.x-2.,e.y).unwrap());assert!(empty.floor().unwrap().godbeast_event.as_ref().unwrap().sacrifice_actor_id.is_missing());
}
#[test]
fn shard_invalid_targets_do_not_consume_and_phoenix_cannot_revive_no_accuracy_roll() {
    let mut r=room();r.player.x=8.;r.player.y=8.;let id=add(&mut r,"phoenix",9.,8.);
    r.inventory.push(borge::shard(500.));r.player.conditions.insert("unlucky".into(),1.);r.player.stats.accuracy=-1000.;
    let before=r.clone();for (x,y) in [(8.,8.),(7.,8.),(12.,8.),(f64::NAN,8.)] {assert!(!borge::use_at(&mut r,500.,x,y).unwrap());assert_eq!(r,before);}
    assert_eq!(inventory::command(&mut r,&Command::Throw{id:500.,x:9.,y:8.}).unwrap(),Some(false));
    let rng=r.rng;assert!(borge::use_at(&mut r,500.,9.,8.).unwrap());assert_eq!(r.rng,rng);assert!(!r.inventory.iter().any(borge::is_shard));
    let a=engine::actor_mut(&mut r,id).unwrap();assert!(a.dead.value_or(false));a.corpse_age=100000.0.into();
    assert!(!corpse_core::corpses::roll_corpse_revival(a,||panic!("permanent death must not roll")));
    corpse_core::corpses::revive_corpse(a,100001.);assert!(a.dead.value_or(false));
    storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();
}
#[test]
fn equipped_shard_kills_any_adjacent_creature_including_sphinx_and_allies() {
    for (kind,state) in [("sphinx","hunting"),("rat","ally"),("dragon","neutral")] {
        let mut r=room();r.player.x=8.;r.player.y=8.;let id=add(&mut r,kind,9.,8.);engine::actor_mut(&mut r,id).unwrap().state=state.into();
        let mut shard=borge::shard(500.);shard.equipped="hand2".to_string().into();r.inventory.push(shard);
        let rng=r.rng;engine::player_attack(&mut r,id).unwrap();assert_eq!(r.rng,rng);
        assert!(engine::actor(&r,id).unwrap().dead.value_or(false));assert!(r.inventory.is_empty());
    }
}
#[test]
fn provenance_survives_actor_floor_transfer_carry_drop_save_and_snapshot() {
    let mut r=room();let id=add(&mut r,"rat",8.,8.);summon(&mut r);
    let mut f=r.floor().unwrap().clone();f.depth=2.;f.godbeast=false;f.godbeast_event=Field::Missing;
    r.floors.push(Arc::new(f));r.actors.insert("2".into(),Arc::new(vec![]));r.ground.insert("2".into(),Arc::new(vec![]));
    let a=r.actors_at_mut(1.).remove(0);r.actors_at_mut(2.).push(a);r.depth=2.;kill(&mut r,id);let heart=take_marked(&mut r);
    assert_eq!(r.inventory[0].borge_depth.as_ref(),Some(&1.));let snap=Snapshot::capture(&r);
    inventory::command(&mut r,&Command::Drop{id:heart}).unwrap();let ground=r.ground["2"][0].item.clone();assert_eq!(ground.source_actor_id.as_ref(),Some(&id));
    let mut r=storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();snap.restore(&mut r).unwrap();
    r.depth=1.;let (x,y)=altar(&mut r);assert!(borge::interact(&mut r,x,y).unwrap());
}
#[test]
fn save_replay_and_both_time_abilities_preserve_consumption_and_permanent_death() {
    let mut r=room();r.player.x=8.;r.player.y=8.;let id=add(&mut r,"rat",9.,8.);r.inventory.push(borge::shard(500.));
    for name in ["numo-revert","morae-see-the-future"] {r.player.abilities.push(name.into());corpse_core::progression::initialize_ability_state(&mut r.player,name,0.).unwrap();}
    r.player.conditions.insert("revertCharges".into(),3.);let initial=r.clone();let mut e=Engine::new(r).unwrap();
    e.dispatch(&Command::UseAt{id:500.,x:9.,y:8.}).unwrap();assert!(engine::actor(&e.run,id).unwrap().dead.value_or(false));
    let saved=e.save().unwrap();let loaded=Engine::load(&saved).unwrap();assert_eq!(loaded.run.inventory,e.run.inventory);
    let mut replay=corpse_core::replay::ReplaySession::from_initial(initial,corpse_core::replay::ReplayRecord::from_run(&e.run)).unwrap();while replay.step().unwrap() {}assert_eq!(replay.engine.save().unwrap(),saved);
    e.dispatch(&ability("numo-revert")).unwrap();assert!(!engine::actor(&e.run,id).unwrap().dead.value_or(false));assert!(e.run.inventory.iter().any(borge::is_shard));
    e.dispatch(&ability("morae-see-the-future")).unwrap();e.dispatch(&Command::UseAt{id:500.,x:9.,y:8.}).unwrap();
    let mut e=Engine::load(&e.save().unwrap()).unwrap();e.dispatch(&ability("morae-see-the-future")).unwrap();assert!(!engine::actor(&e.run,id).unwrap().dead.value_or(false));assert!(e.run.inventory.iter().any(borge::is_shard));
}
#[test]
fn bumping_obelisk_interacts_before_collision_in_normal_and_haste_phases() {
    for hasted in [false,true] {
        let mut r=room();add(&mut r,"rat",8.,8.);
        let e=r.floor().unwrap().godbeast_event.as_ref().unwrap().clone();r.player.x=e.x-3.;r.player.y=e.y;
        if hasted {r.player.conditions.insert("hasted".into(),10.);}
        let before=(r.player.x,r.player.y);let outcome=engine::dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap();
        assert!(outcome.took_turn);assert_eq!((r.player.x,r.player.y),before);assert_eq!(r.tick,1.);
        assert!(r.floor().unwrap().godbeast_event.as_ref().unwrap().activated.value_or(false));
        r.player.active.push("etill-phase-through".into());assert!(!engine::actor_passable(&r,&r.player,e.x-2.,e.y));
        assert!(!corpse_core::travel::passable(&r,&r.player,e.x-2.,e.y));
    }
}
#[test]
fn offering_is_atomic_across_actual_revert_future_and_save() {
    let mut r=room();let id=add(&mut r,"rat",8.,8.);summon(&mut r);kill(&mut r,id);let heart=take_marked(&mut r);let (x,y)=altar(&mut r);
    for name in ["numo-revert","morae-see-the-future"] {r.player.abilities.push(name.into());corpse_core::progression::initialize_ability_state(&mut r.player,name,0.).unwrap();}
    r.player.conditions.insert("revertCharges".into(),3.);let mut e=Engine::new(r).unwrap();
    e.dispatch(&Command::Interact{x,y}).unwrap();assert!(e.run.inventory.iter().any(borge::is_shard));
    e.dispatch(&ability("numo-revert")).unwrap();assert!(e.run.inventory.iter().any(|i|i.id==heart && i.source_actor_id.as_ref()==Some(&id)));
    assert!(!e.run.inventory.iter().any(borge::is_shard));assert!(!e.run.floor().unwrap().godbeast_event.as_ref().unwrap().reward_created.value_or(false));
    e.dispatch(&ability("morae-see-the-future")).unwrap();e.dispatch(&Command::Interact{x,y}).unwrap();
    let mut e=Engine::load(&e.save().unwrap()).unwrap();e.dispatch(&ability("morae-see-the-future")).unwrap();
    assert!(e.run.inventory.iter().any(|i|i.id==heart));assert!(!e.run.inventory.iter().any(borge::is_shard));
    e.dispatch(&Command::Interact{x,y}).unwrap();assert_eq!(e.run.inventory.iter().filter(|i|borge::is_shard(i)).count(),1);
}
#[test]
fn shard_administrator_uses_existing_terminal_boss_hook_not_immunity_or_corpse() {
    let mut r=room();r.floor_mut().unwrap().godbeast_event=Field::Missing;r.floor_mut().unwrap().godbeast=false;
    for depth in 2..=40 {let mut f=r.floors[0].as_ref().clone();f.depth=depth as f64;r.floors.push(Arc::new(f));r.actors.insert(depth.to_string(),Arc::new(vec![]));r.ground.insert(depth.to_string(),Arc::new(vec![]));}
    r.depth=40.;r.player.x=8.;r.player.y=8.;r.actors_at_mut(40.).push(corpse_core::finale::administrator(200.,9.,8.));r.inventory.push(borge::shard(500.));
    let rng=r.rng;assert!(borge::use_at(&mut r,500.,9.,8.).unwrap());assert_eq!(r.rng,rng);
    assert!(r.actors_at(40.).is_empty());assert!(r.ground["40"].is_empty());assert!(r.collapse.as_ref().is_some());assert!(!r.inventory.iter().any(borge::is_shard));
    storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();
}
#[test]
fn malformed_provenance_and_half_initialized_event_are_rejected_at_load() {
    let mut r=room();let id=add(&mut r,"rat",8.,8.);summon(&mut r);kill(&mut r,id);take_marked(&mut r);
    let mut v=serde_json::to_value(&r).unwrap();v["inventory"][0]["sourceActorId"]=serde_json::json!(-1);
    assert!(storage::deserialize(&v.to_string()).is_err());
    let mut v=serde_json::to_value(&r).unwrap();v["floors"][0]["godbeastEvent"].as_object_mut().unwrap().remove("altar");
    assert!(storage::deserialize(&v.to_string()).is_err());
}
#[test]
fn one_heart_cannot_be_pledged_to_two_borge_events_after_actor_travel() {
    let mut r=room();let id=add(&mut r,"rat",8.,8.);summon(&mut r);
    let mut f=r.floor().unwrap().clone();f.depth=2.;let e=f.godbeast_event.as_mut().unwrap();e.activated=Field::Missing;e.sacrifice_actor_id=Field::Missing;e.sacrifice_name=Field::Missing;
    r.floors.push(Arc::new(f));r.actors.insert("2".into(),Arc::new(vec![]));r.ground.insert("2".into(),Arc::new(vec![]));
    let mut a=r.actors_at_mut(1.).remove(0);a.max_hp=10000.;r.actors_at_mut(2.).push(a);r.depth=2.;
    let other=add(&mut r,"rat",9.,8.);assert_eq!(summon(&mut r),other);assert_ne!(other,id);
}
#[test]
fn verdania_is_existing_tree_with_legacy_internal_identity_and_r33_display() {
    let seed=(0..1000).find(|s|generation::domain_plan_revision(*s,33.)[0].godbeast_kind=="Veridian").unwrap();
    let f=generation::generate_floor_full_revision(5,seed,None,33.).unwrap();let e=f.godbeast_event.as_ref().unwrap();
    assert_eq!(e.kind,"Veridian");assert_eq!(e.display_name.as_ref().map(String::as_str),Some("Verdania"));
    let mut r=room();r.floor_mut().unwrap().godbeast_event=e.clone().into();r.player.x=e.x-1.;r.player.y=e.y;
    let tap=corpse_core::godbeast_interactions::tap_veridian(&mut r);assert!(tap.success);let item=tap.sap.unwrap();assert_eq!(item.name,"Verdania Sap");assert_eq!(item.effect.as_ref().map(String::as_str),Some("Veridian Sap"));
}
#[test]
fn generated_floor40_and_nonmilestones_never_add_an_extra_godbeast() {
    for depth in [1,4,6,9,11,36,39,40] {
        let f=generation::generate_floor_full_revision(depth,613,None,33.).unwrap();
        assert!(!f.godbeast,"floor {depth}");assert!(f.godbeast_event.is_missing(),"floor {depth}");
    }
    // Explicit callers cannot force a revision33 encounter on the Administrator floor.
    let f=generation::generate_floor_revision(40,613,"Dungeon",true,79,None,33.).unwrap();
    assert!(!f.godbeast);assert!(f.godbeast_event.is_missing());
}
#[test]
fn all_25_body_cells_have_obelisk_feature_and_altar_survives_hazard_memory() {
    let r=room();let f=r.floor().unwrap();let e=f.godbeast_event.as_ref().unwrap();
    for y in e.y as usize-2..=e.y as usize+2 {for x in e.x as usize-2..=e.x as usize+2 {
        let h=corpse_core::hazard_memory::observed_hazards_at(f,y*f.width()+x,0.);
        assert_eq!(h.feature.as_ref().unwrap().name,"The Borge");
    }}
    let p=e.altar.as_ref().unwrap();let at=p.y as usize*f.width()+p.x as usize;
    let h=corpse_core::hazard_memory::observed_hazards_at(f,at,0.);assert_eq!(h.feature.as_ref().unwrap().name,"Borge sacrifice altar");
}
#[test]
fn actual_stair_travel_preserves_the_marked_heart_and_event() {
    let mut r=room();let id=add(&mut r,"rat",8.,8.);summon(&mut r);kill(&mut r,id);let heart=take_marked(&mut r);
    for depth in 2..=10 {let mut f=r.floors[0].as_ref().clone();f.depth=depth as f64;f.godbeast=false;f.godbeast_event=Field::Missing;r.floors.push(Arc::new(f));r.actors.insert(depth.to_string(),Arc::new(vec![]));r.ground.insert(depth.to_string(),Arc::new(vec![]));}
    let down=r.floor().unwrap().tiles.iter().position(|t|t=="down").unwrap();let w=r.floor().unwrap().width();r.player.x=(down%w) as f64;r.player.y=(down/w) as f64;
    assert!(corpse_core::travel::travel(&mut r,1.,false,false).unwrap());assert_eq!(r.depth,2.);assert!(r.inventory.iter().any(|i|i.id==heart && i.source_actor_id.as_ref()==Some(&id)));
    assert!(corpse_core::travel::travel(&mut r,-1.,false,false).unwrap());let (x,y)=altar(&mut r);assert!(borge::interact(&mut r,x,y).unwrap());
}
#[test]
fn blocked_ordinary_melee_geometry_preserves_the_shard() {
    let mut r=room();r.player.x=8.;r.player.y=8.;add(&mut r,"rat",9.,9.);r.inventory.push(borge::shard(500.));
    let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[8*w+9]="wall".into();r.floor_mut().unwrap().tiles[9*w+8]="wall".into();
    let before=r.clone();assert!(!borge::use_at(&mut r,500.,9.,9.).unwrap());assert_eq!(r,before);
    let e=r.floor().unwrap().godbeast_event.as_ref().unwrap().clone();r.player.x=e.x-3.;r.player.y=e.y;assert!(!borge::interact(&mut r,e.x-2.5,e.y).unwrap());
}

#[test]
fn altar_is_a_fixed_fixture_like_existing_godbeast_hardware() {
    let r=room();let f=r.floor().unwrap();let p=f.godbeast_event.as_ref().unwrap().altar.as_ref().unwrap();
    assert!(!corpse_core::travel::descent_cells(f,p.x,p.y,false).contains(&(p.y as usize*f.width()+p.x as usize)));
}
