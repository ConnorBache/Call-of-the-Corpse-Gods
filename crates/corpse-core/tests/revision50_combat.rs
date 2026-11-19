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
    let (a,b) = bodies(&r); c::after_attack(&mut r,&a,&b,true).unwrap();
    assert_eq!(r.player.x,11.); assert_eq!(engine::actor(&r,1.).unwrap().x,12.);
    let mut r = room(); r.player.abilities.push(c::FORCE.into()); r.player.active.push(c::FORCE.into());
    let w = r.floor().unwrap().width(); r.floor_mut().unwrap().tiles[10*w+12] = "wall".into();
    let (a,b) = bodies(&r); c::after_attack(&mut r,&a,&b,true).unwrap();
    assert_eq!(r.player.x,11.); assert_eq!(engine::actor(&r,1.).unwrap().x,10.);
    r.simulation_revision = 49.0.into(); let (a,b) = bodies(&r);
    c::after_attack(&mut r,&a,&b,true).unwrap(); assert_eq!(r.player.x,11.);
}
#[test]
fn commandment_passive_one_active_max_and_insufficient_energy_fallback() {
    for (active,energy,expected,cost) in [(false,100.,12.,0.),(true,100.,14.,20.),(true,19.,12.,0.)] {
        let mut r = room(); r.player.abilities.push(c::COMMANDMENT.into()); r.player.energy = energy;
        if active { r.player.active.push(c::COMMANDMENT.into()); }
        let w = r.floor().unwrap().width(); r.floor_mut().unwrap().tiles[10*w+15] = "wall".into();
        let (a,b) = bodies(&r); c::after_attack(&mut r,&a,&b,true).unwrap();
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
    let (a,b) = bodies(&r); c::after_attack(&mut r,&a,&b,true).unwrap();
    assert_eq!(engine::actor(&r,1.).unwrap().x,11.);
    assert_eq!(r.player.energy,80.);
}
#[test]
fn force_swaps_instead_of_pushing_nonflyer_over_chasm() {
    let mut r = room(); r.player.abilities.push(c::FORCE.into()); r.player.active.push(c::FORCE.into());
    let w = r.floor().unwrap().width(); r.floor_mut().unwrap().tiles[10*w+12] = "chasm".into();
    let (a,b) = bodies(&r); c::after_attack(&mut r,&a,&b,true).unwrap();
    assert_eq!(r.player.x,11.); assert_eq!(engine::actor(&r,1.).unwrap().x,10.);
    assert!(!engine::actor(&r,1.).unwrap().dead.value_or(false));
}
#[test]
fn commandment_restores_existing_forced_move_value() {
    let mut r = room(); r.player.abilities.push(c::COMMANDMENT.into());
    r.actors_at_mut(1.)[0].conditions.insert("forcedMove".into(),7.);
    let (a,b) = bodies(&r); c::after_attack(&mut r,&a,&b,true).unwrap();
    let b = engine::actor(&r,1.).unwrap();
    assert_eq!(b.x,12.); assert_eq!(engine::condition(b,"forcedMove"),7.);
}
#[test]
fn force_does_not_silently_limit_attack_to_melee_distance() {
    let mut r = room(); r.player.abilities.push(c::FORCE.into()); r.player.active.push(c::FORCE.into());
    r.actors_at_mut(1.)[0].x = 15.;
    let (a,b) = bodies(&r); c::after_attack(&mut r,&a,&b,true).unwrap();
    assert_eq!(r.player.x,15.); assert_eq!(engine::actor(&r,1.).unwrap().x,16.);
}
#[test]
fn force_into_killed_target_square_runs_corpse_stomp() {
    let mut r = room(); r.player.abilities.push(c::FORCE.into()); r.player.active.push(c::FORCE.into());
    let (a,b) = bodies(&r);
    let corpse = &mut r.actors_at_mut(1.)[0]; corpse.hp = 0.; corpse.dead = true.into(); corpse.corpse_age = 8.0.into();
    c::after_attack(&mut r,&a,&b,true).unwrap();
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
    let (a,b) = bodies(&r); c::after_attack(&mut r,&a,&b,true).unwrap();
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

#[test]
fn commandment_hit_only_preserves_old_misses_and_unstoppable_force() {
    for (landed, historical, force, expected_x, expected_spend) in [
        (false, false, false, 11., 0.),
        (false, false, true, 12., 0.),
        (false, true, false, 14., 20.),
        (true, false, false, 14., 20.),
    ] {
        let mut r = room();
        if historical { r.commandment_rules_from = corpse_core::Field::Null; }
        r.player.abilities.push(c::COMMANDMENT.into());
        r.player.active.push(c::COMMANDMENT.into());
        if force {
            r.player.abilities.push(c::FORCE.into());
            r.player.active.push(c::FORCE.into());
        }
        r.player.conditions.insert(if landed { "lucky" } else { "unlucky" }.into(), 1.);
        let w = r.floor().unwrap().width();
        r.floor_mut().unwrap().tiles[10*w+15] = "wall".into();
        let player_id = r.player.id;
        let initial_energy = r.player.energy;
        engine::attack(&mut r, player_id, 1., 1.).unwrap();
        let enemy = engine::actor(&r,1.).unwrap();
        assert_eq!(enemy.hp < 100., landed);
        assert_eq!(enemy.x, expected_x);
        assert_eq!(r.player.x, if force { 11. } else { 10. });
        assert_eq!(r.player.energy, initial_energy - expected_spend);
    }
}

#[test]
fn commandment_does_not_spend_or_move_on_a_landed_friendly_strike() {
    let mut r = room();
    r.player.abilities.push(c::COMMANDMENT.into());
    r.player.active.push(c::COMMANDMENT.into());
    r.actors_at_mut(1.)[0].state = "ally".into();
    let (a,b) = bodies(&r);
    c::after_attack(&mut r, &a, &b, true).unwrap();
    assert_eq!(r.player.energy, 100.);
    assert_eq!(engine::actor(&r,1.).unwrap().x, 11.);
}

#[test]
fn commandment_policy_adopts_at_end_of_history_and_survives_replay_restart() {
    use corpse_core::{Command, Field, replay::{ReplayRecord,ReplaySession}, temporal::Engine};
    let mut initial = room();
    initial.inventory.clear();
    let mut record = ReplayRecord::from_run(&initial);
    record.commands = vec![Command::Wait];
    let mut old = ReplaySession::from_initial(initial.clone(), record.clone()).unwrap();
    old.step().unwrap();
    assert_eq!(old.engine.run.commandment_rules_from, Field::Null);
    let live = Engine::load(&old.engine.save().unwrap()).unwrap();
    assert_eq!(live.run.commandment_rules_from, Field::Value(1));

    let mut mixed = ReplaySession::from_initial(initial.clone(), ReplayRecord::try_from_run(&live.run).unwrap()).unwrap();
    for _ in 0..2 {
        assert_eq!(mixed.engine.run.commandment_rules_from, Field::Null);
        mixed.step().unwrap();
        assert_eq!(mixed.engine.save().unwrap(), live.save().unwrap());
        mixed.restart().unwrap();
    }
    for invalid in [Field::Null, Field::Value(2)] {
        record.commandment_rules_from = invalid;
        assert!(ReplaySession::from_initial(initial.clone(), record.clone()).is_err());
    }
}

fn execute_room(reserve:f64, energy:f64) -> Run {
    let mut r=room();
    r.simulation_revision=52.0.into();
    r.execute_rules_from=0usize.into();
    r.player.stats.accuracy=1000.;
    r.player.energy=energy;
    r.player.abilities=vec![c::EXECUTE.into()];
    r.player.active=r.player.abilities.clone();
    r.player.ability_reserves=Default::default();
    r.inventory=vec![corpse_core::Item{id:120.,kind:"ring".into(),name:"Ring of Reaping".into(),
        effect:corpse_core::revision50_rings::REAPING.to_string().into(),..Default::default()}];
    corpse_core::revision50_ability_energy::initialize_reserve(&mut r.player,c::EXECUTE,10.,reserve).unwrap();
    r.actors_at_mut(1.)[0].stats.dodge=10.;
    r
}

fn execute_melee(r:&mut Run) -> bool {
    let before=r.player.ability_reserves.as_ref().unwrap()[c::EXECUTE].energy;
    let player=r.player.id;
    engine::attack(r,player,1.,1.).unwrap();
    r.player.ability_reserves.as_ref().unwrap()[c::EXECUTE].energy < before
}

#[test]
fn execute_crossing_uses_actual_ordinary_damage_and_reaps_only_that_packet() {
    let mut baseline=execute_room(10.,5.);
    baseline.player.active.clear();
    assert!(!execute_melee(&mut baseline));
    let packet=100.-engine::actor(&baseline,1.).unwrap().hp;
    assert!(packet>0. && packet<75.);
    let carry=baseline.player.reaping_ledger.as_ref().unwrap().damage_carry;

    for (start,enabled,reserve,energy,expected_proc,expected_hp) in [
        (packet+24.,true,10.,5.,true,0.),
        (packet+24.,true,5.,5.,true,0.),
        (packet+24.,false,10.,5.,false,24.),
        (packet+24.,true,5.,4.,false,24.),
        (packet+25.,true,10.,5.,false,25.),
        (24.,true,10.,5.,false,(24.-packet).max(0.)),
        (packet,true,10.,5.,false,0.),
    ] {
        let mut r=execute_room(reserve,energy);
        if !enabled {r.player.active.clear();}
        r.actors_at_mut(1.)[0].hp=start;
        assert_eq!(execute_melee(&mut r),expected_proc,"start {start}, reserve {reserve}, energy {energy}");
        let hp=r.actors_at(1.).iter().find(|a|a.id==1.).map_or(0.,|a|a.hp.max(0.));
        assert_eq!(hp,expected_hp);
        assert_eq!(r.player.ability_reserves.as_ref().unwrap()[c::EXECUTE].energy,
            if expected_proc {0.} else {reserve});
        assert_eq!(r.player.energy,if expected_proc {energy-(10.-reserve)} else {energy});
        if expected_proc { assert_eq!(r.player.reaping_ledger.as_ref().unwrap().damage_carry,carry); }
    }
}
#[test]
fn execute_crossing_bypasses_death_rescues_but_natural_lethality_does_not() {
    let mut baseline=execute_room(10.,5.);
    baseline.player.active.clear();
    execute_melee(&mut baseline);
    let packet=100.-engine::actor(&baseline,1.).unwrap().hp;
    for (start,proc) in [(packet+24.,true),(packet,false)] {
        let mut r=execute_room(10.,5.);
        let b=&mut r.actors_at_mut(1.)[0];
        b.hp=start;b.blood=1.0.into();b.abilities.push(corpse_core::divine_rescues::OVERFLOWING.into());
        assert_eq!(execute_melee(&mut r),proc);
        if proc {
            assert!(r.actors_at(1.).iter().all(|a|a.id!=1.||a.hp<=0.));
        } else {
            assert_eq!(engine::actor(&r,1.).unwrap().hp,1.);
            assert_eq!(r.player.ability_reserves.as_ref().unwrap()[c::EXECUTE].energy,10.);
        }
    }
}
#[test]
fn execute_crossing_never_charges_on_a_miss_or_witch_hatred() {
    let mut r=execute_room(10.,5.);
    r.player.stats.accuracy=0.;
    r.actors_at_mut(1.)[0].hp=40.;
    assert!(!execute_melee(&mut r));
    assert_eq!(engine::actor(&r,1.).unwrap().hp,40.);
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()[c::EXECUTE].energy,10.);

    let mut r=execute_room(10.,5.);
    r.player.abilities.push(c::WITCH_HATRED.into());
    r.actors_at_mut(1.)[0].hp=40.;
    r.actors_at_mut(1.)[0].abilities.push(c::MAGICKAL_CASTER.into());
    assert!(!execute_melee(&mut r));
    assert!(r.actors_at(1.).iter().all(|a|a.id!=1.||a.hp<=0.));
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()[c::EXECUTE].energy,10.);
}

#[test]
fn execute_crossing_admits_bows_and_weapon_throws_but_not_improvised_objects() {
    use corpse_core::{Command, Item, projectiles};
    for kind in ["bow shot","weapon","tool"] {
        let fire=|r:&mut Run| {
            if kind=="bow shot" {
                assert!(engine::attack_bow_normal_accuracy_resolved(r,1.,10.,1.).unwrap());
            } else {
                let command=Command::Throw{id:123.,x:11.,y:10.};
                assert_eq!(projectiles::command(r,&command).unwrap(),Some(true));
            }
        };
        let mut initial=execute_room(10.,5.);
        initial.player.stats.offense=20.;
        let b=&mut initial.actors_at_mut(1.)[0];
        b.conditions.insert("paralyzed".into(),5.);
        if kind!="bow shot" {
            initial.inventory.push(Item{
                id:123.,kind:kind.into(),name:"Test thrown item".into(),hands:1.0.into(),
                damage:0.0.into(),offense_multiplier:1.0.into(),thrown:true.into(),..Default::default()
            });
        }
        let mut baseline=initial.clone();
        baseline.player.active.clear();
        fire(&mut baseline);
        let packet=100.-engine::actor(&baseline,1.).unwrap().hp;
        assert!(packet>0. && packet<75.,"{kind}: baseline {packet}");
        initial.actors_at_mut(1.)[0].hp=24.+packet;
        fire(&mut initial);
        let target_hp=initial.actors_at(1.).iter().find(|a|a.id==1.).map_or(0.,|a|a.hp.max(0.));
        let eligible=kind!="tool";
        assert_eq!(target_hp,if eligible {0.} else {24.},"{kind}");
        assert_eq!(initial.player.ability_reserves.as_ref().unwrap()[c::EXECUTE].energy,
            if eligible {0.} else {10.},"{kind}");
    }
}
