use corpse_core::{Actor, Run, engine, revision50_combat as c};
fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 50.0.into();
    // Explicit exhausted test reserves preserve assertions about shared fallback.
    r.player.ability_reserves = indexmap::IndexMap::from([
        (c::EXECUTE.into(), corpse_core::revision50_ability_energy::AbilityReserve { capacity: 20., energy: 0. }),
        (c::COMMANDMENT.into(), corpse_core::revision50_ability_energy::AbilityReserve { capacity: 20., energy: 0. }),
    ]).into();
    r.player.x = 10.; r.player.y = 10.; r.player.hp = 100.; r.player.max_hp = 100.;
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.floor_mut().unwrap().tiles.fill("floor".into());
    r.floor_mut().unwrap().doors = Default::default();
    let mut b = r.actors_at(r.depth)[0].clone();
    b.id = 1.; b.x = 11.; b.y = 10.; b.hp = 100.; b.max_hp = 100.;
    b.state = "hunting".into(); b.dead = false.into(); b.abilities.clear(); b.conditions.clear();
    *r.actors_at_mut(r.depth) = vec![b];
    r
}
fn bodies(r: &Run) -> (Actor,Actor) { (r.player.clone(),r.actors_at(r.depth)[0].clone()) }
#[test]
fn execute_strict_threshold_and_exact_cost_and_revision_gate() {
    let mut r = room(); r.player.abilities.push(c::EXECUTE.into()); r.player.active.push(c::EXECUTE.into());
    let (a,mut b) = bodies(&r);
    b.hp = 25.; assert!(!c::lethal_hit(&mut r,&a,&b)); assert_eq!(r.player.energy,100.);
    // Live owner health is authoritative, not the attack snapshot.
    r.actors_at_mut(1.)[0].hp = 24.; b.hp = 24.;
    assert!(c::lethal_hit(&mut r,&a,&b)); assert_eq!(r.player.energy,80.);
    r.player.energy = 19.; assert!(!c::lethal_hit(&mut r,&a,&b)); assert_eq!(r.player.energy,19.);
    r.player.energy = 100.; r.simulation_revision = 49.0.into();
    assert!(!c::lethal_hit(&mut r,&a,&b)); assert_eq!(r.player.energy,100.);
}
#[test]
fn witch_hatred_uses_explicit_trait_and_does_not_charge_execute() {
    let mut r = room(); r.player.abilities.extend([c::WITCH_HATRED.into(),c::EXECUTE.into()]); r.player.active.push(c::EXECUTE.into());
    r.actors_at_mut(1.)[0].abilities.push(c::MAGICKAL_CASTER.into());
    let (a,b) = bodies(&r); assert!(c::witch_senses(&r,&a,&b));
    assert!(c::lethal_hit(&mut r,&a,&b)); assert_eq!(r.player.energy,100.);
    let mut unclassified = b.clone(); unclassified.abilities = vec!["enemy:beast:witch".into()];
    assert!(!c::magickal_caster(&unclassified));
}
#[test]
fn force_pushes_else_swaps_and_old_revision_is_inert() {
    let mut r = room(); r.player.abilities.push(c::FORCE.into()); r.player.active.push(c::FORCE.into());
    let (a,b) = bodies(&r); c::after_attack(&mut r,&a,&b).unwrap();
    assert_eq!(r.player.x,11.); assert_eq!(engine::actor(&r,1.).unwrap().x,12.);
    let mut r = room(); r.player.abilities.push(c::FORCE.into()); r.player.active.push(c::FORCE.into());
    let w = r.floor().unwrap().width(); r.floor_mut().unwrap().tiles[10*w+12] = "wall".into();
    let (a,b) = bodies(&r); c::after_attack(&mut r,&a,&b).unwrap();
    assert_eq!(r.player.x,11.); assert_eq!(engine::actor(&r,1.).unwrap().x,10.);
    r.simulation_revision = 49.0.into(); let (a,b) = bodies(&r);
    c::after_attack(&mut r,&a,&b).unwrap(); assert_eq!(r.player.x,11.);
}
#[test]
fn commandment_passive_one_active_max_and_insufficient_energy_fallback() {
    for (active,energy,expected,cost) in [(false,100.,12.,0.),(true,100.,14.,20.),(true,19.,12.,0.)] {
        let mut r = room(); r.player.abilities.push(c::COMMANDMENT.into()); r.player.energy = energy;
        if active { r.player.active.push(c::COMMANDMENT.into()); }
        let w = r.floor().unwrap().width(); r.floor_mut().unwrap().tiles[10*w+15] = "wall".into();
        let (a,b) = bodies(&r); c::after_attack(&mut r,&a,&b).unwrap();
        assert_eq!(engine::actor(&r,1.).unwrap().x,expected); assert_eq!(r.player.energy,energy-cost);
    }
}
#[test]
fn oblivious_blocks_unseen_enemy_but_not_environment_or_ally() {
    let mut r = room(); r.player.abilities.push(c::OBLIVIOUS.into());
    r.actors_at_mut(1.)[0].conditions.insert("invisible".into(),10.);
    let (a,b) = bodies(&r); assert!(c::oblivious_blocks(&r,&a,Some(&b)));
    assert!(!c::oblivious_blocks(&r,&a,None));
    let mut ally = b.clone(); ally.state = "ally".into(); assert!(!c::oblivious_blocks(&r,&a,Some(&ally)));
    r.simulation_revision = 49.0.into(); assert!(!c::oblivious_blocks(&r,&a,Some(&b)));
}
#[test]
fn missed_me_retreats_only_with_space_and_spends_no_reaction_if_trapped() {
    let mut r = room(); r.player.abilities.push("isheilah-missed-me".into()); r.player.active.push("isheilah-missed-me".into()); r.player.reactions = 1.0.into();
    let (a,b) = bodies(&r); assert!(c::missed_me(&mut r,&b,&a,true).unwrap());
    assert_eq!(r.player.reactions.value_or(0.),0.); assert!(r.player.x < 10.);
    r.player.x = 10.; r.player.y = 10.; r.player.reactions = 1.0.into();
    let w = r.floor().unwrap().width();
    for y in 9..=11 { for x in 9..=11 { if x!=10 || y!=10 {r.floor_mut().unwrap().tiles[y*w+x]="wall".into();} } }
    let (a,b) = bodies(&r); assert!(!c::missed_me(&mut r,&b,&a,true).unwrap()); assert_eq!(r.player.reactions.value_or(0.),1.);
}
#[test]
fn masochism_crossings_rearm_after_healing_and_are_strict() {
    assert_eq!(c::masochism_crossings(100.,100.,100.),0);
    assert_eq!(c::masochism_crossings(100.,99.,100.),1);
    assert_eq!(c::masochism_crossings(75.,74.,100.),1);
    assert_eq!(c::masochism_crossings(100.,24.,100.),4);
    assert_eq!(c::masochism_crossings(50.,75.,100.),0);
    assert_eq!(c::masochism_crossings(75.,74.,100.),1);
}
#[test]
fn replaced_debuff_and_counter_and_nightmare_are_inert() {
    let mut r = room(); r.player.conditions.insert("beatToDeathHits".into(),3.);
    let mut stats = r.player.stats.clone(); corpse_core::divine_combat::apply_stat_modifiers(&r,&r.player,&mut stats); assert_eq!(stats.dr,0.);
    r.player.abilities.push(corpse_core::divine_reactions::HIT_BACK.into());
    r.player.active.push(corpse_core::divine_reactions::NIGHTMARE.into()); r.player.nightmares = 1.0.into();
    let (a,b) = bodies(&r);
    assert_eq!(corpse_core::divine_reactions::prepare_hit_back(&mut r,&b,&a,10.,0.,0.,false),None);
    assert!(!corpse_core::divine_reactions::nightmare_absorbs(&r,&a,Some(&b)));
}
#[test]
fn commandment_cannot_push_nonflying_enemy_into_chasm() {
    let mut r = room(); r.player.abilities.push(c::COMMANDMENT.into()); r.player.active.push(c::COMMANDMENT.into());
    let w = r.floor().unwrap().width(); r.floor_mut().unwrap().tiles[10*w+12] = "chasm".into();
    let (a,b) = bodies(&r); c::after_attack(&mut r,&a,&b).unwrap();
    assert_eq!(engine::actor(&r,1.).unwrap().x,11.);
    assert_eq!(r.player.energy,80.);
}
#[test]
fn force_swaps_instead_of_pushing_nonflyer_over_chasm() {
    let mut r = room(); r.player.abilities.push(c::FORCE.into()); r.player.active.push(c::FORCE.into());
    let w = r.floor().unwrap().width(); r.floor_mut().unwrap().tiles[10*w+12] = "chasm".into();
    let (a,b) = bodies(&r); c::after_attack(&mut r,&a,&b).unwrap();
    assert_eq!(r.player.x,11.); assert_eq!(engine::actor(&r,1.).unwrap().x,10.);
    assert!(!engine::actor(&r,1.).unwrap().dead.value_or(false));
}
#[test]
fn commandment_restores_existing_forced_move_value() {
    let mut r = room(); r.player.abilities.push(c::COMMANDMENT.into());
    r.actors_at_mut(1.)[0].conditions.insert("forcedMove".into(),7.);
    let (a,b) = bodies(&r); c::after_attack(&mut r,&a,&b).unwrap();
    let b = engine::actor(&r,1.).unwrap();
    assert_eq!(b.x,12.); assert_eq!(engine::condition(b,"forcedMove"),7.);
}
#[test]
fn force_does_not_silently_limit_attack_to_melee_distance() {
    let mut r = room(); r.player.abilities.push(c::FORCE.into()); r.player.active.push(c::FORCE.into());
    r.actors_at_mut(1.)[0].x = 15.;
    let (a,b) = bodies(&r); c::after_attack(&mut r,&a,&b).unwrap();
    assert_eq!(r.player.x,15.); assert_eq!(engine::actor(&r,1.).unwrap().x,16.);
}
#[test]
fn force_into_killed_target_square_runs_corpse_stomp() {
    let mut r = room(); r.player.abilities.push(c::FORCE.into()); r.player.active.push(c::FORCE.into());
    let (a,b) = bodies(&r);
    let corpse = &mut r.actors_at_mut(1.)[0]; corpse.hp = 0.; corpse.dead = true.into(); corpse.corpse_age = 8.0.into();
    c::after_attack(&mut r,&a,&b).unwrap();
    assert_eq!(r.player.x,11.);
    assert_eq!(engine::actor(&r,1.).unwrap().corpse_age.value_or(-1.),0.);
    assert!(engine::actor(&r,1.).unwrap().dead.value_or(false));
}
#[test]
fn masochism_multiple_thresholds_grant_ten_total_and_healing_rearms() {
    let mut r = room(); r.player.abilities.push(c::MASOCHISM.into());
    r.player.hp = 24.; let id = r.player.id;
    c::masochism_after_hp_loss(&mut r,id,100.);
    assert_eq!(engine::condition(&r.player,"hasted"),10.);
    assert_eq!(engine::condition(&r.player,"hastedFresh"),1.);
    r.player.conditions.insert("hasted".into(),3.);
    // Healed above the 75% boundary, then damaged below it again.
    r.player.hp = 74.; c::masochism_after_hp_loss(&mut r,id,80.);
    assert_eq!(engine::condition(&r.player,"hasted"),10.);
    r.player.conditions.insert("hasted".into(),3.);
    r.player.hp = 73.; c::masochism_after_hp_loss(&mut r,id,74.);
    assert_eq!(engine::condition(&r.player,"hasted"),3.);
}
#[test]
fn masochism_uses_derived_maximum_and_does_not_shorten_longer_haste() {
    let mut r = room(); r.player.abilities.push(c::MASOCHISM.into());
    r.player.base_max_hp = 100.0.into(); r.player.stats.defense = 10.;
    // Derived maximum is 160 while serialized maxHp is deliberately stale 100.
    r.player.hp = 119.; let id = r.player.id;
    c::masochism_after_hp_loss(&mut r,id,120.);
    assert_eq!(engine::condition(&r.player,"hasted"),10.);
    r.player.conditions.insert("hasted".into(),20.);
    c::masochism_after_hp_loss(&mut r,id,120.);
    assert_eq!(engine::condition(&r.player,"hasted"),20.);
}
#[test]
fn masochism_normal_and_haste_phase_share_existing_duration_clock() {
    for phase in ["normal", "haste"] {
        for resolving in [false,true] {
            let mut r = room(); r.phase = phase.to_string().into(); r.resolving_tick = resolving.into();
            r.player.abilities.push(c::MASOCHISM.into()); r.player.hp = 99.; let id = r.player.id;
            c::masochism_after_hp_loss(&mut r,id,100.);
            assert_eq!(engine::condition(&r.player,"hasted"),10.);
            assert_eq!(engine::condition(&r.player,"hastedFresh"),if resolving {0.} else {1.});
            engine::tick_condition(&mut r.player,"hasted");
            assert_eq!(engine::condition(&r.player,"hasted"),if resolving {9.} else {10.});
        }
    }
}
#[test]
fn masochism_is_passive_revision_gated_and_never_triggers_on_healing() {
    let mut r = room(); let id = r.player.id; r.player.hp = 99.;
    c::masochism_after_hp_loss(&mut r,id,100.); assert_eq!(engine::condition(&r.player,"hasted"),0.);
    r.player.abilities.push(c::MASOCHISM.into()); r.simulation_revision = 49.0.into();
    c::masochism_after_hp_loss(&mut r,id,100.); assert_eq!(engine::condition(&r.player,"hasted"),0.);
    r.simulation_revision = 50.0.into();
    c::masochism_after_hp_loss(&mut r,id,75.); assert_eq!(engine::condition(&r.player,"hasted"),0.);
    assert_eq!(c::masochism_crossings(f64::NAN,24.,100.),0);
    assert_eq!(c::masochism_crossings(100.,24.,f64::INFINITY),0);
}
#[test]
fn masochism_common_damage_packet_hook_runs_once_for_four_crossings() {
    let mut r = room(); r.player.abilities.push(c::MASOCHISM.into()); let id = r.player.id;
    engine::damage_environment(&mut r,id,76.,None,"blood-weapon",true).unwrap();
    assert_eq!(r.player.hp,24.);
    assert_eq!(engine::condition(&r.player,"hasted"),10.);
}
#[test]
fn paid_combat_uses_its_own_reserve_and_missing_reserve_refuses_payment() {
    let mut r = room(); r.player.abilities.push(c::EXECUTE.into()); r.player.active.push(c::EXECUTE.into());
    r.player.energy = 0.;
    r.player.ability_reserves.as_mut().unwrap().get_mut(c::EXECUTE).unwrap().energy = 20.;
    r.actors_at_mut(1.)[0].hp = 24.;
    let (a,b) = bodies(&r); assert!(c::lethal_hit(&mut r,&a,&b));
    assert_eq!(r.player.energy,0.);
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()[c::EXECUTE].energy,0.);
    r.player.energy = 100.; r.player.ability_reserves.as_mut().unwrap().shift_remove(c::EXECUTE);
    assert!(!c::lethal_hit(&mut r,&a,&b)); assert_eq!(r.player.energy,100.);
}
#[test]
fn commandment_paid_push_uses_reserve_with_empty_shared_bar() {
    let mut r = room(); r.player.abilities.push(c::COMMANDMENT.into()); r.player.active.push(c::COMMANDMENT.into());
    r.player.energy = 0.;
    r.player.ability_reserves.as_mut().unwrap().get_mut(c::COMMANDMENT).unwrap().energy = 20.;
    let w = r.floor().unwrap().width(); r.floor_mut().unwrap().tiles[10*w+15] = "wall".into();
    let (a,b) = bodies(&r); c::after_attack(&mut r,&a,&b).unwrap();
    assert_eq!(engine::actor(&r,1.).unwrap().x,14.); assert_eq!(r.player.energy,0.);
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()[c::COMMANDMENT].energy,0.);
}
#[test]
fn surrender_admission_and_payment_both_use_the_ability_reserve() {
    use corpse_core::{divine_reactions as d, revision50_ability_energy::AbilityReserve};
    let mut r = room(); r.player.abilities.push(d::SURRENDER.into()); r.player.active.push(d::SURRENDER.into());
    r.player.energy = 0.;
    r.player.ability_reserves.as_mut().unwrap().insert(d::SURRENDER.into(),AbilityReserve {capacity:8.,energy:8.});
    let b = r.player.clone();
    assert_eq!(d::surrender_bonus(&r,&b,12.,true),12.);
    d::pay_surrender(&mut r,&b,5.);
    assert_eq!(r.player.energy,0.);
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()[d::SURRENDER].energy,0.);
    assert_eq!(d::surrender_bonus(&r,&b,12.,true),0.);
    r.tick += 1.; r.player.energy = 100.; r.player.ability_reserves.as_mut().unwrap().shift_remove(d::SURRENDER);
    assert_eq!(d::surrender_bonus(&r,&b,12.,true),0.);
}
#[test]
fn rage_free_first_rescue_then_uses_own_reserve_and_legacy_stays_unchanged() {
    use corpse_core::{divine_rescues as d, revision50_ability_energy::AbilityReserve};
    let mut r = room(); r.player.abilities.push(d::RAGE.into()); r.player.active.push(d::RAGE.into());
    r.player.energy = 0.; r.player.hp = 0.; let id = r.player.id;
    assert!(d::try_rescue(&mut r,id,false)); assert_eq!(r.player.hp,1.);
    r.player.ability_reserves.as_mut().unwrap().insert(d::RAGE.into(),AbilityReserve {capacity:1.,energy:1.});
    r.player.hp = 0.; assert!(d::try_rescue(&mut r,id,false));
    assert_eq!(r.player.energy,0.); assert_eq!(r.player.ability_reserves.as_ref().unwrap()[d::RAGE].energy,0.);
    assert_eq!(engine::condition(&r.player,"rageCost"),2.);
    r.simulation_revision = 49.0.into(); r.player.energy = 2.; r.player.hp = 0.;
    r.player.ability_reserves = Default::default();
    assert!(d::try_rescue(&mut r,id,false)); assert_eq!(r.player.energy,0.);
}
