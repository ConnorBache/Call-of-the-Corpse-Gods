use corpse_core::{Actor, Run, ai, engine, revision50_nightmares as n};

fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 50.0.into();
    r.player.x = 10.; r.player.y = 10.;
    r.player.abilities = vec![n::ID.into()];
    r.player.active.clear(); r.player.conditions.clear();
    r.player.nightmares = 2.0.into();
    r.floor_mut().unwrap().tiles.fill("floor".into());
    r.floor_mut().unwrap().doors = Default::default();
    r.actors_at_mut(r.depth).clear();
    r
}

#[test]
fn independent_serialized_actor_and_multiple_decoys() {
    let mut r = room();
    let energy = r.player.energy;
    let first = n::create(&mut r, 11., 10.).unwrap();
    let second = n::create(&mut r, 12., 10.).unwrap();
    assert_ne!(first, second);
    assert_eq!(r.player.nightmares.value_or(-1.), 0.);
    assert_eq!(r.player.energy, energy);
    for id in [first, second] {
        let a = engine::actor(&r, id).unwrap();
        assert!(n::is_decoy(a));
        assert_eq!(a.conditions[n::OWNER], r.player.id);
        assert_eq!(a.state, "ally");
        assert!(a.abilities.is_empty() && a.active.is_empty());
        assert_eq!(corpse_core::half_here::owner_id(&r, a), id);
        assert!(!corpse_core::half_here::is_clone(a));
        assert!(a.heartless.value_or(false));
    }
    let restored: Run = serde_json::from_value(serde_json::to_value(&r).unwrap()).unwrap();
    assert!(n::is_decoy(engine::actor(&restored, first).unwrap()));
    assert!(ai::phase_order(restored.actors_at(restored.depth), false).is_empty());
    assert!(ai::phase_order(restored.actors_at(restored.depth), true).is_empty());
}

#[test]
fn invalid_activation_is_atomic_including_bad_birth_cursor() {
    for (x,y,charges,revision) in [(10.,10.,1.,50.), (f64::NAN,10.,1.,50.),
        (-1.,10.,1.,50.), (11.5,10.,1.,50.), (11.,10.,0.,50.), (11.,10.,1.,49.)] {
        let mut r = room(); r.player.nightmares = charges.into(); r.simulation_revision = revision.into();
        let before = serde_json::to_value(&r).unwrap();
        assert!(n::create(&mut r,x,y).is_err());
        assert_eq!(serde_json::to_value(&r).unwrap(), before);
    }
    let mut r = room(); r.next_id = r.player.id;
    let before = serde_json::to_value(&r).unwrap();
    assert!(n::create(&mut r,11.,10.).is_err());
    assert_eq!(serde_json::to_value(&r).unwrap(), before);
}

#[test]
fn occupied_decoy_square_is_rejected_before_spending() {
    let mut r = room(); n::create(&mut r,11.,10.).unwrap();
    let before = serde_json::to_value(&r).unwrap();
    assert!(n::preflight(&r,11.,10.).is_err());
    assert!(n::create(&mut r,11.,10.).is_err());
    assert_eq!(serde_json::to_value(&r).unwrap(),before);
}

#[test]
fn recharge_on_hundredth_normal_turn_persists_through_save() {
    let mut r = room(); r.player.nightmares = 0.0.into();
    for tick in 1..100 { r.tick = tick as f64; n::tick(&mut r); }
    assert_eq!(r.player.nightmares.value_or(-1.),0.);
    let mut r: Run = serde_json::from_value(serde_json::to_value(&r).unwrap()).unwrap();
    r.tick = 100.; n::tick(&mut r);
    assert_eq!(r.player.nightmares.value_or(-1.),1.);
    n::tick(&mut r); // Same tick cannot duplicate the recharge.
    assert_eq!(r.player.nightmares.value_or(-1.),1.);
    for tick in 101..=200 { r.tick = tick as f64; n::tick(&mut r); }
    assert_eq!(r.player.nightmares.value_or(-1.),2.);
    r.simulation_revision = 49.0.into(); r.tick = 300.; n::tick(&mut r);
    assert_eq!(r.player.nightmares.value_or(-1.),2.);
}

#[test]
fn strike_removes_only_decoy_without_rewards_or_shared_damage() {
    let mut r = room(); let first = n::create(&mut r,11.,10.).unwrap();
    let second = n::create(&mut r,12.,10.).unwrap();
    let player = r.player.clone(); let exp = r.exp;
    assert!(n::strike(&mut r,first));
    assert!(!n::strike(&mut r,first));
    assert!(engine::actor(&r,first).is_none());
    assert!(engine::actor(&r,second).is_some());
    assert_eq!(r.player,player); assert_eq!(r.exp,exp);
    let id = r.player.id; assert!(!n::strike(&mut r,id));
}

#[test]
fn decoys_never_execute_enemy_special_actions() {
    let mut r = room(); let id = n::create(&mut r,11.,10.).unwrap();
    let player = r.player.id;
    assert!(!ai::enemy_action(&mut r,id,player).unwrap());
    let enemy = Actor { id: r.next_id, x: 12., y: 10., state: "hunting".into(),
        hp: 10., max_hp: 10., ..Default::default() };
    r.next_id += 1.; r.actors_at_mut(r.depth).push(enemy);
    assert!(!ai::phase_order(r.actors_at(r.depth),false).contains(&id));
}

#[test]
fn decoys_do_not_follow_stairs_or_fall_through_terrain() {
    let mut r = room(); let id = n::create(&mut r,11.,10.).unwrap();
    corpse_core::travel::queue_ally_stair_followers(&mut r,2.,corpse_core::movement::Point{x:10,y:10});
    assert!(!corpse_core::travel::has_ally_stair_follow(engine::actor(&r,id).unwrap()));
    let depth = r.depth;
    assert!(!corpse_core::travel::arrive_ally_at_stair(&mut r,id,depth,corpse_core::movement::Point{x:12,y:10}).unwrap());
    let w = r.floor().unwrap().width();
    r.floor_mut().unwrap().tiles[10*w+11] = "chasm".into();
    corpse_core::travel::fall_actors_one_level(&mut r,depth,&[id],None).unwrap();
    let a = engine::actor(&r,id).unwrap();
    assert_eq!((a.x,a.y,a.hp),(11.,10.,1.));
}

#[test]
fn passive_fire_and_biological_statuses_do_not_affect_hallucination() {
    let mut r = room(); let id = n::create(&mut r,11.,10.).unwrap();
    corpse_core::environment::initialize(r.floor_mut().unwrap()).unwrap();
    let w = r.floor().unwrap().width();
    r.floor_mut().unwrap().environment.as_mut().unwrap().fire[10*w+11] = 10.;
    let before = engine::actor(&r,id).unwrap().clone();
    let rng = r.rng;
    corpse_core::environment::refresh_burning_from_fire(&mut r,id).unwrap();
    for key in ["slowed","poisoned","hemophilia"] {
        assert!(!corpse_core::monster_statuses::apply_status(&mut r,id,key,10.,None));
    }
    assert_eq!(engine::actor(&r,id).unwrap(),&before);
    assert_eq!(r.rng,rng);
}

fn enemy(r: &mut Run, ability: &str) -> f64 {
    let mut a = r.player.clone();
    a.id = r.next_id; r.next_id += 1.;
    a.x = 13.; a.y = 10.; a.state = "hunting".into();
    a.abilities = vec![ability.into()]; a.active.clear(); a.conditions.clear();
    a.energy = 100.;
    let id = a.id; r.actors_at_mut(r.depth).push(a); id
}

#[test]
fn direct_breath_and_ruin_pulse_dissolve_but_lingering_fire_does_not() {
    let mut r = room(); let id = n::create(&mut r,11.,10.).unwrap();
    let source = enemy(&mut r,"enemy:beast:fire-breath");
    engine::damage_environment(&mut r,id,24.,Some(source),"fire",true).unwrap();
    assert!(engine::actor(&r,id).is_some());
    assert!(corpse_core::monster_inventor::breathe_fire(&mut r,source,id).unwrap());
    assert!(engine::actor(&r,id).is_none());
    let mut r = room(); let id = n::create(&mut r,11.,10.).unwrap();
    let source = enemy(&mut r,"enemy:beast:ruin-pulse");
    assert!(corpse_core::monster_ruin::special_action(&mut r,source,id).unwrap());
    assert!(engine::actor(&r,id).is_none());
}

#[test]
fn resolved_curse_projectile_dissolves_and_preserves_caster_cost_and_recovery() {
    let mut r = room(); let id = n::create(&mut r,11.,10.).unwrap();
    let source = enemy(&mut r,"enemy:beast:witch");
    assert!(corpse_core::monster_spells::special_action(&mut r,source,id).unwrap());
    assert!(engine::actor(&r,id).is_none());
    let caster = engine::actor(&r,source).unwrap();
    assert_eq!(caster.energy,92.);
    assert_eq!(caster.conditions["rangedRecovery"],1.);
    assert_eq!(caster.conditions["monsterSpellPhase"],1.);
}

#[test]
fn blocked_curse_does_not_dissolve_and_immediate_blast_does() {
    let mut r = room(); let id = n::create(&mut r,11.,10.).unwrap();
    let source = enemy(&mut r,"enemy:beast:witch");
    let w = r.floor().unwrap().width();
    r.floor_mut().unwrap().tiles[10*w+12] = "wall".into();
    assert!(!corpse_core::monster_spells::special_action(&mut r,source,id).unwrap());
    assert!(engine::actor(&r,id).is_some());
    corpse_core::environment::explode(&mut r,11.,10.,corpse_core::environment::ExplosionOptions {
        ignite_area:false, ..Default::default()
    },None).unwrap();
    assert!(engine::actor(&r,id).is_none());
}
