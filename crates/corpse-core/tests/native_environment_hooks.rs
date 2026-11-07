//! Designer hooks 2–5, revision 33: focused deterministic regressions.
use corpse_core::{Actor, Command, Field, FungalRoot, Run, ai, combat, engine, environment, storage, water_blood as blood};
use std::sync::Arc;
fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into();
    r.player.x = 20.; r.player.y = 10.; r.player.hp = 500.; r.player.max_hp = 500.;
    r.player.conditions.clear(); r.player.abilities.clear(); r.player.active.clear();
    r.player.stats.max_hp = 500.0.into();
    r.inventory.clear(); r.actors_at_mut(1.).clear(); r.phase = Field::Missing;
    let f = r.floor_mut().unwrap();
    f.environment = Field::Missing; f.fungal_roots = Field::Missing;
    f.roots = Field::Missing; f.mud = Field::Missing; f.slime = Field::Missing;
    f.ichor = Field::Missing; f.razorleaves = Field::Missing; f.water_blood = Field::Missing;
    f.kelpie_blood_sources = Field::Missing; f.currents = Field::Missing;
    f.vents = Field::Missing; f.permanent_fire = Field::Missing;
    let w=f.width(); let h=f.height();
    for y in 1..h-1 { for x in 1..w-1 { f.tiles[y*w+x]="floor".into(); } }
    r
}
fn at(r: &Run, x: usize, y: usize) -> usize { y*r.floor().unwrap().width()+x }
fn actor(r: &mut Run, name: &str, x: f64, y: f64, abilities: &[&str]) -> f64 {
    let mut a: Actor = serde_json::from_str::<Run>(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap().actors_at(1.)[0].clone();
    a.id=r.next_id; r.next_id+=1.; a.name=name.into(); a.creature_kind=Field::Missing;
    a.x=x; a.y=y; a.hp=500.; a.max_hp=500.; a.energy=100.;
    a.stats.max_hp=500.0.into(); a.stats.max_energy=100.;
    a.abilities=abilities.iter().map(|s| s.to_string()).collect();
    a.active.clear(); a.conditions.clear(); a.state="hunting".into(); a.dead=false.into();
    a.conditions.insert("bestiaryTraitsInitialized".into(),1.);
    a.conditions.insert("paleTraitsInitialized".into(),1.);
    let id=a.id; r.actors_at_mut(1.).push(a); id
}
fn concentration(r: &Run, i: usize) -> f64 { r.floor().unwrap().water_blood.as_ref().map_or(0.,|v|v[i]) }
fn gas_clear(r: &mut Run) { r.floor_mut().unwrap().environment=Field::Missing; }
#[test]
fn only_actual_positive_hp_damage_in_water_deposits_including_forced_and_dot_packets() {
    for tile in ["floor","water","shallow-water","grass"] {
        let mut r=room(33.); let p=r.player.id; let i=at(&r,20,10);
        r.floor_mut().unwrap().tiles[i]=tile.into();
        let rng=r.rng;
        engine::damage_environment(&mut r,p,0.,None,"attack",true).unwrap();
        assert_eq!(concentration(&r,i),0.);
        r.player.conditions.insert("forcedMove".into(),1.);
        for kind in ["attack","bleeding","poison","fire"] {
            engine::damage_environment(&mut r,p,4.,None,kind,true).unwrap();
        }
        assert_eq!(concentration(&r,i),if tile.contains("water"){16.}else{0.},"{tile}");
        assert_eq!(r.rng,rng);
    }
    let mut r=room(33.);let p=r.player.id;let i=at(&r,20,10);
    r.floor_mut().unwrap().tiles[i]="water".into();
    r.player.abilities.push("enemy:beast:fire-immunity".into());
    engine::damage_environment(&mut r,p,4.,None,"fire",true).unwrap();
    assert_eq!(concentration(&r,i),0.);
}
#[test]
fn blood_deposits_at_half_here_damaged_body_not_owner_pool() {
    let mut r=room(33.); r.player.abilities.push("etill-half-here".into());
    let clone=corpse_core::half_here::create(&mut r,23.,10.).unwrap();
    let body=at(&r,23,10);let owner=at(&r,20,10);
    r.floor_mut().unwrap().tiles[body]="water".into();
    r.floor_mut().unwrap().tiles[owner]="water".into();
    engine::damage_environment(&mut r,clone,7.,None,"attack",true).unwrap();
    assert_eq!(concentration(&r,body),7.);assert_eq!(concentration(&r,owner),0.);
}
#[test]
fn ordinary_movement_is_not_a_water_blood_trail() {
    let mut r=room(33.);let p=r.player.id;let from=at(&r,20,10);
    for i in [from,from+1] {r.floor_mut().unwrap().tiles[i]="water".into();}
    let rng=r.rng;
    assert!(engine::move_actor(&mut r,p,1.,0.,true).unwrap());
    assert_eq!(concentration(&r,from),0.);assert_eq!(concentration(&r,from+1),0.);
    assert_eq!(r.rng,rng);
}
#[test]
fn diffusion_is_bounded_water_only_deterministic_and_respects_frozen_cells() {
    let mut r=room(33.);let i=at(&r,20,10);
    for j in [i,i-1,i+1] {r.floor_mut().unwrap().tiles[j]="water".into();}
    let p=r.player.clone();blood::deposit_damage(&mut r,&p,1000.);
    assert_eq!(concentration(&r,i),100.);
    let rng=r.rng;let mut same=r.clone();
    blood::diffuse(r.floor_mut().unwrap(),&[]);blood::diffuse(same.floor_mut().unwrap(),&[]);
    assert_eq!(r,same);assert_eq!(r.rng,rng);
    assert!((concentration(&r,i)-75.2).abs()<1e-9);
    assert!((concentration(&r,i-1)-9.4).abs()<1e-9);
    assert_eq!(concentration(&r,i-r.floor().unwrap().width()),0.);
    let mut frozen=vec![false;r.floor().unwrap().tiles.len()];frozen[i]=true;
    let fixed=concentration(&r,i);blood::diffuse(r.floor_mut().unwrap(),&frozen);
    assert_eq!(concentration(&r,i),fixed);
    for _ in 0..200 { blood::diffuse(r.floor_mut().unwrap(),&[]); }
    assert!(!blood::active(r.floor().unwrap()));
}
#[test]
fn kelpie_spawn_source_is_mild_and_persists_at_spawn_after_movement_and_death() {
    let mut r=room(33.);let i=at(&r,22,10);
    for j in [i,i+1] {r.floor_mut().unwrap().tiles[j]="water".into();}
    let id=actor(&mut r,"Kelpie",22.,10.,&[]);
    engine::actor_mut(&mut r,id).unwrap().creature_kind="kelpie".to_string().into();
    blood::register_kelpie_sources(&mut r,1.);
    assert_eq!(concentration(&r,i),blood::KELPIE_SOURCE);
    engine::actor_mut(&mut r,id).unwrap().x=23.;
    engine::actor_mut(&mut r,id).unwrap().dead=true.into();
    for _ in 0..40 {blood::diffuse(r.floor_mut().unwrap(),&[]);}
    assert_eq!(r.floor().unwrap().kelpie_blood_sources.as_ref().unwrap(),&vec![i as f64]);
    assert_eq!(concentration(&r,i),blood::KELPIE_SOURCE);
    assert!(concentration(&r,i+1)>0. && concentration(&r,i+1)<blood::KELPIE_SOURCE);
}
#[test]
fn water_and_status_round_trip_and_temporal_snapshots() {
    let mut r=room(33.);let i=at(&r,20,10);r.floor_mut().unwrap().tiles[i]="water".into();
    let before=corpse_core::temporal::Snapshot::capture(&r);
    let p=r.player.clone();blood::deposit_damage(&mut r,&p,12.);
    r.player.conditions.insert("hallucinating".into(),2.);
    r.player.conditions.insert("disoriented".into(),5.);
    let after=corpse_core::temporal::Snapshot::capture(&r);
    let restored=storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();assert_eq!(restored,r);
    before.restore(&mut r).unwrap();assert_eq!(concentration(&r,i),0.);
    after.restore(&mut r).unwrap();assert_eq!(concentration(&r,i),12.);
    assert_eq!(engine::condition(&r.player,"hallucinating"),2.);
}
#[test]
fn hallucination_exposure_refreshes_exactly_two_world_turns_without_damage_or_confusion() {
    let mut r=room(33.);let i=at(&r,20,10);let hp=r.player.hp;
    r.player.conditions.insert("hallucinating".into(), 100.); // Refresh is exact, not max/add.
    for _ in 0..4 {
        environment::add_gas(r.floor_mut().unwrap(),i,"hallucinogenic",1000.,-1.).unwrap();
        environment::tick_environment(&mut r).unwrap();
        assert_eq!(engine::condition(&r.player,"hallucinating"),2.);
        assert_eq!(engine::condition(&r.player,"confused"),0.);
        assert_eq!(r.player.hp,hp);
    }
    gas_clear(&mut r);environment::tick_environment(&mut r).unwrap();
    assert_eq!(engine::condition(&r.player,"hallucinating"),1.);
    environment::tick_environment(&mut r).unwrap();assert_eq!(engine::condition(&r.player,"hallucinating"),0.);
}
#[test]
fn haste_movement_does_not_tick_hallucinations_or_diffusion() {
    let mut r=room(33.);r.player.conditions.insert("hasted".into(),5.);
    r.player.conditions.insert("hallucinating".into(),2.);
    let i=at(&r,20,10);r.floor_mut().unwrap().tiles[i]="water".into();
    let p=r.player.clone();blood::deposit_damage(&mut r,&p,10.);
    let tick=r.tick;let out=engine::dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap();
    assert!(!out.took_turn);assert_eq!(r.tick,tick);
    assert_eq!(engine::condition(&r.player,"hallucinating"),2.);assert_eq!(concentration(&r,i),10.);
    engine::dispatch(&mut r,&Command::Wait).unwrap();
    assert_eq!(engine::condition(&r.player,"hallucinating"),1.);
}
#[test]
fn greater_breather_uses_hallucinogenic_for_passive_and_boost_and_is_immune() {
    for greater in [false,true] {
        let mut r=room(33.);
        let name=if greater{"Greater Pale Breather"}else{"Pale Breather"};
        let id=actor(&mut r,name,22.,10.,&["enemy:pale:breath","enemy:pale:breath-boost","enemy:pale:gas-immunity"]);
        let p=r.player.id;let i=at(&r,22,10);
        assert!(ai::enemy_action(&mut r,id,p).unwrap());
        let gas=if greater{"hallucinogenic"}else{"caustic"};
        assert_eq!(r.floor().unwrap().environment.as_ref().unwrap().gas_type.as_ref().unwrap()[i].as_deref(),Some(gas));
        gas_clear(&mut r);environment::tick_environment(&mut r).unwrap();
        let e=r.floor().unwrap().environment.as_ref().unwrap();
        assert!(e.gas_type.as_ref().unwrap().iter().any(|g|g.as_deref()==Some(gas)));
        assert_eq!(engine::condition(engine::actor(&r,id).unwrap(),"hallucinating"),0.);
        assert_eq!(engine::actor(&r,id).unwrap().hp,500.);
    }
}
#[test]
fn scream_disorients_accuracy_only_refreshes_and_expires_allied_boost_unchanged() {
    for greater in [false,true] {
        let mut r=room(33.);let p=r.player.id;
        let name=if greater{"Greater Pale Screamer"}else{"Pale Screamer"};
        let id=actor(&mut r,name,22.,10.,&["enemy:pale:scream"]);
        let before=combat::effective_stats(&r,&r.player);
        assert!(ai::enemy_action(&mut r,id,p).unwrap());
        let after=combat::effective_stats(&r,&r.player);
        assert_eq!(after.accuracy,before.accuracy-if greater{10.}else{5.});
        assert_eq!(after.offense,before.offense);assert_eq!(after.defense,before.defense);
        assert_eq!(engine::condition(&r.player,"screamWeakened"),0.);
        assert_eq!(engine::condition(&r.player,"disoriented"),5.);
        assert_eq!(engine::condition(engine::actor(&r,id).unwrap(),"screamEmboldened"),5.);
        assert!(ai::enemy_action(&mut r,id,p).unwrap());assert_eq!(engine::condition(&r.player,"disoriented"),5.);
        for _ in 0..5 {environment::tick_environment(&mut r).unwrap();}
        assert_eq!(engine::condition(&r.player,"disoriented"),0.);
        assert_eq!(combat::effective_stats(&r,&r.player).accuracy,before.accuracy);
    }
}
#[test]
fn successful_escape_breaks_binding_root_only_failed_or_blocked_attempt_does_not() {
    let mut r=room(33.);let p=r.player.id;let i=at(&r,20,10);let other=at(&r,30,10);
    let n=r.floor().unwrap().tiles.len();let mut roots=vec![0.;n];
    for j in [i,i-1,other,other-1] {roots[j]=1.;}
    r.floor_mut().unwrap().roots=roots.into();
    r.floor_mut().unwrap().fungal_roots=vec![
        FungalRoot{fungus_id:100.,corpse_id:101.,path:vec![(i-1)as f64,i as f64],active:true,color:"#00ff00".into()},
        FungalRoot{fungus_id:200.,corpse_id:201.,path:vec![(other-1)as f64,other as f64],active:true,color:"#00ff00".into()},
    ].into();
    engine::move_actor(&mut r,p,1.,0.,false).unwrap();
    assert_eq!(r.player.x,20.);assert_eq!(r.floor().unwrap().roots.as_ref().unwrap()[i],1.);
    assert!(r.floor().unwrap().fungal_roots.as_ref().unwrap()[0].active);
    r.floor_mut().unwrap().tiles[i+1]="wall".into();
    assert!(!engine::move_actor(&mut r,p,1.,0.,false).unwrap());
    assert!(r.floor().unwrap().fungal_roots.as_ref().unwrap()[0].active);
    r.floor_mut().unwrap().tiles[i+1]="floor".into();
    assert!(engine::move_actor(&mut r,p,1.,0.,false).unwrap());assert_eq!(r.player.x,21.);
    assert_eq!(r.floor().unwrap().roots.as_ref().unwrap()[i],0.);
    let links=r.floor().unwrap().fungal_roots.as_ref().unwrap();assert!(!links[0].active);assert!(links[1].active);
    engine::move_actor(&mut r,p,-1.,0.,false).unwrap();
    engine::move_actor(&mut r,p,1.,0.,false).unwrap();assert_eq!(r.player.x,21.);
}
#[test]
fn legacy_revisions_keep_no_blood_caustic_breath_and_offense_weakening() {
    for revision in [29.,30.,31.,32.] {
        let mut r=room(revision);let p=r.player.id;let i=at(&r,20,10);
        r.floor_mut().unwrap().tiles[i]="water".into();
        engine::damage_environment(&mut r,p,4.,None,"attack",true).unwrap();
        assert!(r.floor().unwrap().water_blood.is_missing());
        let id=actor(&mut r,"Greater Pale Breather",22.,10.,&["enemy:pale:breath-boost"]);
        ai::enemy_action(&mut r,id,p).unwrap();
        assert!(r.floor().unwrap().environment.as_ref().unwrap().hallucinogenic.is_missing());
        assert_eq!(environment::breather_gas(&r,engine::actor(&r,id).unwrap()),"caustic");
        r.actors_at_mut(1.).clear();
        let id=actor(&mut r,"Pale Screamer",22.,10.,&["enemy:pale:scream"]);
        let before=combat::effective_stats(&r,&r.player);ai::enemy_action(&mut r,id,p).unwrap();
        let after=combat::effective_stats(&r,&r.player);
        assert_eq!(after.offense,before.offense-5.);assert_eq!(after.accuracy,before.accuracy);
        assert_eq!(engine::condition(&r.player,"disoriented"),0.);
    }
}
#[test]
fn offscreen_world_turns_expire_status_and_diffuse_sources() {
    let mut r=room(33.);let id=actor(&mut r,"Test creature",22.,10.,&[]);
    engine::actor_mut(&mut r,id).unwrap().state="sleeping".into();
    engine::actor_mut(&mut r,id).unwrap().conditions.insert("hallucinating".into(),2.);
    engine::actor_mut(&mut r,id).unwrap().conditions.insert("disoriented".into(),2.);
    let mut f=r.floor().unwrap().clone();f.depth=2.;f.last_tick=0.;f.dirty=true;
    let i=10*f.width()+22;f.tiles[i]="water".into();f.tiles[i+1]="water".into();
    let mut blood=vec![0.;f.tiles.len()];blood[i]=3.;f.water_blood=blood.into();f.kelpie_blood_sources=vec![i as f64].into();
    r.floors[1]=Arc::new(f);let actors=std::mem::take(r.actors_at_mut(1.));r.actors.insert("2".into(),Arc::new(actors));
    r.ground.insert("2".into(),Arc::new(vec![]));r.tick=1.;
    corpse_core::offscreen::catch_up_floor(&mut r,2.).unwrap();
    assert_eq!(engine::condition(&r.actors_at(2.)[0],"hallucinating"),1.);
    assert_eq!(engine::condition(&r.actors_at(2.)[0],"disoriented"),1.);
    assert!(r.floors[1].water_blood.as_ref().unwrap()[i+1]>0.);
    r.tick=2.;corpse_core::offscreen::catch_up_floor(&mut r,2.).unwrap();
    assert_eq!(engine::condition(&r.actors_at(2.)[0],"hallucinating"),0.);
}
#[test]
fn reflected_hit_back_blood_uses_the_reflected_victims_water_tile() {
    let mut r=room(33.);let p=r.player.id;let i=at(&r,20,10);
    r.floor_mut().unwrap().tiles[i]="water".into();
    r.player.stats.accuracy=100.;r.player.stats.dr=0.;
    let id=actor(&mut r,"Reflector",21.,10.,&["shrail-i-hit-back"]);
    engine::actor_mut(&mut r,id).unwrap().stats.offense=20.;
    engine::actor_mut(&mut r,id).unwrap().stats.dodge=0.;
    let hp=r.player.hp;engine::attack(&mut r,p,id,1.).unwrap();
    assert!(r.player.hp<hp);assert_eq!(concentration(&r,i),hp-r.player.hp);
}
#[test]
fn severed_fungus_corpse_pair_cannot_reacquire_but_unrelated_link_and_fire_survive() {
    let mut r=room(33.);r.player.x=22.;let p=r.player.id;let i=at(&r,22,10);
    let fungus=actor(&mut r,"Bastidion",20.,10.,&["enemy:beast:fungal-gas","enemy:beast:rooted"]);
    for (x,y) in [(24.,10.),(20.,14.)] {
        let id=actor(&mut r,"Corpse",x,y,&[]);
        let a=engine::actor_mut(&mut r,id).unwrap();a.dead=true.into();a.hp=0.;
    }
    corpse_core::fungi::tick_fungi(&mut r,&[]).unwrap();
    assert_eq!(r.floor().unwrap().fungal_roots.as_ref().unwrap().len(),2);
    assert!(engine::movement_held(&mut r,p));assert!(!engine::movement_held(&mut r,p));
    corpse_core::fungi::tick_fungi(&mut r,&[]).unwrap();
    let links=r.floor().unwrap().fungal_roots.as_ref().unwrap();
    assert_eq!(links.len(),2);assert_eq!(links.iter().filter(|l|l.active).count(),1);
    assert_eq!(r.floor().unwrap().roots.as_ref().unwrap()[i],0.);
    assert!(links.iter().all(|l|l.fungus_id==fungus));
    // Existing fire clearing remains exactly the same tile/link operation.
    let fire_at=at(&r,20,12);environment::burn_roots(r.floor_mut().unwrap(),fire_at);
    assert!(r.floor().unwrap().fungal_roots.as_ref().unwrap().iter().all(|l|!l.active));
}
#[test]
fn hallucination_gas_schema_provenance_and_hazard_memory_round_trip() {
    let mut r=room(33.);let i=at(&r,20,10);
    environment::add_gas(r.floor_mut().unwrap(),i,"hallucinogenic",100.,-2.).unwrap();
    let observed=corpse_core::hazard_memory::observed_hazards_at(r.floor().unwrap(),i,r.tick);
    assert_eq!(observed.gas.as_ref().unwrap().r#type,"hallucinogenic");
    assert_eq!(r.floor().unwrap().environment.as_ref().unwrap().provenance.as_ref().unwrap()["hallucinogenic"].owner[i],-2.);
    let restored=storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();assert_eq!(restored,r);
    let mut invalid=serde_json::to_value(&r).unwrap();invalid["floors"][0]["waterBlood"]=serde_json::json!([101.]);
    assert!(storage::deserialize(&invalid.to_string()).is_err());
}
#[test]
fn source_registration_never_adds_legacy_fields_and_water_memory_distinguishes_zero() {
    for revision in [29.,30.,31.,32.,33.] {
        let mut r=room(revision);let i=at(&r,20,10);r.floor_mut().unwrap().tiles[i]="water".into();
        let before=r.clone();blood::register_kelpie_sources(&mut r,1.);
        let h=corpse_core::hazard_memory::observed_hazards_at(r.floor().unwrap(),i,r.tick);
        if revision<33. { assert_eq!(r,before);assert!(h.water_blood.is_missing()); }
        else { assert_eq!(h.water_blood.as_ref(),Some(&0.)); }
    }
}
#[test]
fn revert_and_see_the_future_restore_new_layers_and_world_turn_status_clocks() {
    let mut r=room(33.);let i=at(&r,20,10);r.floor_mut().unwrap().tiles[i]="water".into();
    let p=r.player.clone();blood::deposit_damage(&mut r,&p,10.);
    r.player.abilities.extend(["numo-revert".into(),"morae-see-the-future".into()]);
    r.player.energy=100.;r.player.stats.max_energy=100.;
    r.player.conditions.insert("revertCharges".into(),5.);
    r.player.conditions.insert("hallucinating".into(),2.);
    r.player.conditions.insert("disoriented".into(),2.);
    let mut session=corpse_core::temporal::Engine::new(r).unwrap();
    let ability=|id:&str|Command::Ability{id:id.into(),target:Field::Missing,x:Field::Missing,y:Field::Missing};
    session.dispatch(&Command::Wait).unwrap();
    assert_eq!(engine::condition(&session.run.player,"hallucinating"),1.);
    assert!(concentration(&session.run,i)<10.);
    session.dispatch(&ability("numo-revert")).unwrap();
    assert_eq!(engine::condition(&session.run.player,"hallucinating"),2.);
    assert_eq!(engine::condition(&session.run.player,"disoriented"),2.);
    assert_eq!(concentration(&session.run,i),10.);
    session.dispatch(&ability("morae-see-the-future")).unwrap();
    session.dispatch(&Command::Wait).unwrap();
    assert_eq!(engine::condition(&session.run.player,"hallucinating"),1.);
    session=corpse_core::temporal::Engine::load(&session.save().unwrap()).unwrap();
    session.dispatch(&ability("morae-see-the-future")).unwrap();
    assert_eq!(engine::condition(&session.run.player,"hallucinating"),2.);
    assert_eq!(engine::condition(&session.run.player,"disoriented"),2.);
    assert_eq!(concentration(&session.run,i),10.);
}
#[test]
fn exposed_enemy_status_does_not_change_real_stats_ai_hostility_or_health() {
    let mut r=room(33.);let id=actor(&mut r,"Ordinary foe",21.,10.,&[]);let i=at(&r,21,10);
    let player_id=r.player.id;
    engine::actor_mut(&mut r,id).unwrap().conditions.insert("targetId".into(),player_id);
    let before=engine::actor(&r,id).unwrap().clone();let stats=combat::effective_stats(&r,&before);
    environment::add_gas(r.floor_mut().unwrap(),i,"hallucinogenic",1000.,-1.).unwrap();
    environment::tick_environment(&mut r).unwrap();
    let after=engine::actor(&r,id).unwrap();assert_eq!(engine::condition(after,"hallucinating"),2.);
    assert_eq!((after.x,after.y,after.hp,after.energy,after.state.as_str()),(before.x,before.y,before.hp,before.energy,before.state.as_str()));
    assert_eq!(after.conditions.get("targetId"),before.conditions.get("targetId"));
    assert_eq!(combat::effective_stats(&r,after),stats);assert_eq!(after.abilities,before.abilities);
}
#[test]
fn hidden_water_never_leaks_live_blood_through_observed_or_unseen_hazards() {
    let mut r=room(33.);let seen=at(&r,30,10);let unseen=seen+1;
    for i in [seen,unseen] {r.floor_mut().unwrap().tiles[i]="water".into();}
    blood::register_kelpie_sources(&mut r,1.);
    let n=r.floor().unwrap().tiles.len();let mut mask=vec![false;n];mask[seen]=true;
    corpse_core::hazard_memory::remember_observed_hazards(r.floor_mut().unwrap(),&mask,0.);
    let f=r.floor_mut().unwrap();f.seen[seen]=true;f.seen[unseen]=false;
    f.remembered_tiles=f.tiles.clone().into();
    let w=f.width();for y in 1..f.height()-1 {f.tiles[y*w+25]="wall".into();}
    f.water_blood.as_mut().unwrap()[seen]=70.;f.water_blood.as_mut().unwrap()[unseen]=90.;
    let before=r.clone();let view=corpse_core::vision::observe_terrain(&r).unwrap();
    assert!(!view.cells[seen].current);assert!(view.cells[seen].remembered);
    assert_eq!(view.cells[seen].hazards.as_ref().unwrap().water_blood.as_ref(),Some(&0.));
    assert!(!view.cells[unseen].current);assert!(!view.cells[unseen].remembered);
    assert!(view.cells[unseen].hazards.is_none());assert_eq!(r,before);
}
#[test]
fn authored_river_sources_exclude_lakes_dry_cells_and_ichor_and_persist_independently() {
    let mut r=room(33.);let river=at(&r,20,10);let lake=at(&r,25,10);let ordinary=at(&r,30,10);
    for i in [river,river+1,lake,ordinary] {r.floor_mut().unwrap().tiles[i]="water".into();}
    let n=r.floor().unwrap().tiles.len();let mut ichor=vec![0.;n];ichor[river+1]=1.;r.floor_mut().unwrap().ichor=ichor.into();
    r.floor_mut().unwrap().generation=corpse_core::GenerationMetadata{
        landmarks:vec![
            corpse_core::GeologicalLandmark{kind:"river".into(),cells:vec![river as f64,river as f64,(river+1)as f64,(river+2)as f64],..Default::default()},
            corpse_core::GeologicalLandmark{kind:"lake".into(),cells:vec![lake as f64],..Default::default()},
        ].into(),..Default::default()
    }.into();
    let rng=r.rng;blood::register_river_sources(r.floor_mut().unwrap());
    assert_eq!(r.floor().unwrap().blood_river_sources.as_ref().unwrap(),&vec![river as f64]);
    assert!(r.floor().unwrap().kelpie_blood_sources.is_missing());
    assert_eq!(concentration(&r,river),blood::RIVER_SOURCE);
    for i in [river+1,river+2,lake,ordinary] {assert_eq!(concentration(&r,i),0.);}
    let once=r.clone();blood::register_river_sources(r.floor_mut().unwrap());assert_eq!(r,once);assert_eq!(r.rng,rng);
    // Source lifetime is independent of generation metadata and living creatures.
    r.floor_mut().unwrap().generation=Field::Missing;
    for _ in 0..20 {blood::diffuse(r.floor_mut().unwrap(),&[]);}
    assert_eq!(concentration(&r,river),blood::RIVER_SOURCE);
    assert_eq!(storage::deserialize(&storage::serialize(&r).unwrap()).unwrap(),r);
}
