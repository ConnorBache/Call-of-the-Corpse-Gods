//! Command/runtime regressions for Revision 50. The fixed fixture and explicit
//! lucky/distracted/paralyzed conditions make hit outcomes independent of RNG luck.
//! These tests complement helper/data-contract tests; they require Cargo to run.
use corpse_core::{Actor, ActorLastSeen, Command, Field, Run, Stats, abilities, ai, engine,
    revision50, revision50_ability_energy as ability_energy,
    revision50_nightmares as nightmares, revision50_pursuit as pursuit};

const IMPLANT: &str = "numo-implant-thought";
const CREDIT: &str = "r50PursuitCredit";

fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 50.0.into();
    r.player.x = 10.;
    r.player.y = 10.;
    r.player.hp = 100.;
    r.player.max_hp = 100.;
    r.player.base_max_hp = 100.0.into();
    r.player.energy = 100.;
    r.player.abilities.clear();
    r.player.active.clear();
    r.player.conditions.clear();
    r.player.ability_order = Field::Missing;
    r.player.ability_reserves = Field::Missing;
    // Test-only exhausted reserves exercise the explicit player-energy fallback.
    // Zero capacities here are fixture inputs, never production balance defaults.
    for id in [IMPLANT, pursuit::ID, revision50::DUEL] {
        ability_energy::initialize_reserve(&mut r.player, id, 0., 0.).unwrap();
    }
    r.inventory.clear();
    r.actors_at_mut(r.depth).clear();
    r.ground_at_mut(r.depth).clear();
    r.phase = "normal".to_string().into();
    let f = r.floor_mut().unwrap();
    f.tiles.fill("floor".into());
    f.lights.fill(100.);
    f.doors = Field::Missing;
    f.portals = Field::Missing;
    f.non_euclidean = Field::Missing;
    f.speed_up = Field::Missing;
    r
}

/// Paralyze ordinary fixture actors so only the requested attack or movement
/// can occur. No species passives, random AI movement, or haste are implicit.
fn enemy(r: &mut Run, x: f64, y: f64) -> f64 {
    let id = r.next_id;
    r.next_id += 1.;
    let mut a = Actor {
        id, x, y, name: format!("Test enemy {id}"), glyph: "e".into(), color: "#ffffff".into(),
        hp: 100., max_hp: 100., base_max_hp: 100.0.into(), energy: 100.,
        state: "hunting".into(),
        stats: Stats { offense: 10., perception: 300., max_energy: 100., ..Default::default() },
        ..Default::default()
    };
    a.conditions.insert("paralyzed".into(), 100.);
    r.actors_at_mut(r.depth).push(a);
    id
}

fn ability(id: &str, target: Option<f64>, point: Option<(f64, f64)>) -> Command {
    Command::Ability {
        id: id.into(), target: target.map_or(Field::Missing, Field::Value),
        x: point.map_or(Field::Missing, |p| Field::Value(p.0)),
        y: point.map_or(Field::Missing, |p| Field::Value(p.1)),
    }
}
fn step(r: &mut Run, dx: f64, dy: f64) -> engine::DispatchOutcome {
    engine::dispatch(r, &Command::Move { dx, dy }).unwrap()
}
fn has_active(r: &Run, id: &str) -> bool { r.player.active.iter().any(|a| a == id) }
fn credit(r: &Run) -> f64 { engine::condition(&r.player, CREDIT) }

fn pursuit_room(target_x: f64) -> (Run, f64) {
    let mut r = room();
    r.player.abilities.push(pursuit::ID.into());
    let target = enemy(&mut r, target_x, 10.);
    let f = r.floor_mut().unwrap();
    let w = f.width();
    f.tiles.fill("wall".into());
    for x in 10..=target_x as usize { f.tiles[10 * w + x] = "floor".into(); }
    let outcome = engine::dispatch(&mut r, &ability(pursuit::ID, Some(target), None)).unwrap();
    assert!(!outcome.took_turn);
    assert_eq!(r.player.energy, 90.);
    assert_eq!(r.tick, 0.);
    (r, target)
}

#[test]
fn pursuit_dispatch_grants_exactly_one_movement_only_credit_per_world_turn() {
    let (mut r, _) = pursuit_room(18.);
    assert!(step(&mut r, 1., 0.).took_turn);
    assert_eq!((r.player.x, r.tick, credit(&r)), (11., 1., 1.));
    let bonus_tick = r.tick;
    let rng_before = r.rng;
    let ai_before = r.metrics.get("aiActors").copied();
    assert!(!step(&mut r, 1., 0.).took_turn);
    assert_eq!((r.player.x, r.tick, credit(&r)), (12., bonus_tick, 0.));
    assert_eq!(r.rng, rng_before);
    assert_eq!(r.metrics.get("aiActors").copied(), ai_before);
    assert_eq!(r.player.energy, 90.);
    // The next move advances the world instead of chaining another free move.
    assert!(step(&mut r, 1., 0.).took_turn);
    assert_eq!((r.player.x, r.tick, credit(&r)), (13., 2., 1.));
}

#[test]
fn pursuit_blocked_bonus_move_preserves_credit_and_does_not_tick_or_draw_rng() {
    let (mut r, _) = pursuit_room(18.);
    step(&mut r, 1., 0.);
    let (tick, rng, energy) = (r.tick, r.rng, r.player.energy);
    assert!(!step(&mut r, 0., 1.).took_turn); // A wall beside the corridor.
    assert_eq!((r.player.x, r.player.y, credit(&r)), (11., 10., 1.));
    assert_eq!((r.tick, r.rng, r.player.energy), (tick, rng, energy));
    assert!(!step(&mut r, 1., 0.).took_turn);
    assert_eq!((r.player.x, r.tick, credit(&r)), (12., tick, 0.));
}

#[test]
fn pursuit_bonus_cannot_make_an_attack_free_or_resolve_it_twice() {
    let (mut r, target) = pursuit_room(12.);
    step(&mut r, 1., 0.);
    assert_eq!(credit(&r), 1.);
    let before = engine::actor(&r, target).unwrap().hp;
    let out = step(&mut r, 1., 0.);
    assert!(out.took_turn);
    assert_eq!((r.player.x, r.tick, credit(&r)), (11., 2., 0.));
    assert!(engine::actor(&r, target).unwrap().hp < before);
    assert_eq!(out.events.iter().filter(|e| e.kind == "impact"
        && e.from.as_ref().is_some_and(|p| p.x == 11. && p.y == 10.)
        && e.to.x == 12. && e.to.y == 10.).count(), 1);
}

#[test]
fn pursuit_wait_is_an_ordinary_turn_and_discards_unused_credit() {
    let (mut r, _) = pursuit_room(18.);
    step(&mut r, 1., 0.);
    assert!(engine::dispatch(&mut r, &Command::Wait).unwrap().took_turn);
    assert_eq!((r.tick, credit(&r)), (2., 0.));
}

#[test]
fn pursuit_bonus_round_trips_and_does_not_repeat_a_world_or_haste_phase() {
    let (mut r, _) = pursuit_room(18.);
    r.player.conditions.insert("hasted".into(), 10.);
    r.phase = "haste".to_string().into();
    assert!(!step(&mut r, 1., 0.).took_turn); // Ordinary haste movement first.
    assert_eq!((r.tick, credit(&r)), (0., 0.));
    assert!(step(&mut r, 1., 0.).took_turn); // Normal movement earns Pursuit.
    assert_eq!((r.tick, credit(&r)), (1., 1.));
    let mut restored: Run = serde_json::from_value(serde_json::to_value(&r).unwrap()).unwrap();
    let tick = restored.tick;
    let haste = engine::condition(&restored.player, "hasted");
    let phase = restored.phase.clone();
    assert!(!step(&mut restored, 1., 0.).took_turn);
    assert_eq!((restored.tick, credit(&restored)), (tick, 0.));
    assert_eq!(engine::condition(&restored.player, "hasted"), haste);
    assert_eq!(restored.phase, phase);
}

#[test]
fn nightmare_actual_melee_miss_retains_decoy_and_landed_zero_damage_strike_dissolves_it() {
    let mut r = room();
    r.player.abilities.push(nightmares::ID.into());
    r.player.nightmares = 1.0.into();
    let attacker = enemy(&mut r, 12., 10.);
    let decoy = nightmares::create(&mut r, 11., 10.).unwrap();
    // A huge DR must not prevent a landed strike from dissolving the image.
    engine::actor_mut(&mut r, decoy).unwrap().stats.dr = 1_000_000.;
    engine::actor_mut(&mut r, attacker).unwrap().conditions.insert("distracted".into(), 1.);
    let player = r.player.clone();
    let (exp, next_id) = (r.exp, r.next_id);
    let ground = r.ground.clone();
    engine::attack(&mut r, attacker, decoy, 1.).unwrap();
    assert!(engine::actor(&r, decoy).is_some());
    assert!(r.log.iter().any(|message| message.contains("miss")));
    engine::actor_mut(&mut r, attacker).unwrap().conditions.insert("lucky".into(), 1.);
    engine::attack(&mut r, attacker, decoy, 1.).unwrap();
    assert!(engine::actor(&r, decoy).is_none());
    assert_eq!(r.actors_at(r.depth).len(), 1); // No corpse actor remains.
    assert_eq!(r.player, player); // No shared health, reward, or on-hit effect.
    assert_eq!((r.exp, r.next_id), (exp, next_id));
    assert_eq!(r.ground, ground);
}

#[test]
fn nightmare_dispatch_creates_an_independent_decoy_without_advancing_actor_clocks() {
    let mut r = room();
    r.player.abilities.push(nightmares::ID.into());
    r.player.nightmares = 1.0.into();
    let energy = r.player.energy;
    let tick = r.tick;
    let out = engine::dispatch(&mut r, &ability(nightmares::ID, None, Some((11., 10.)))).unwrap();
    assert!(!out.took_turn);
    assert_eq!((r.tick, r.player.energy, r.player.nightmares.value_or(-1.)), (tick, energy, 0.));
    let decoy = r.actors_at(r.depth).iter().find(|a| nightmares::is_decoy(a)).unwrap().clone();
    let ai_before = r.metrics.get("aiActors").copied().unwrap_or(0.);
    assert!(ai::phase_order(r.actors_at(r.depth), false).is_empty());
    assert!(ai::phase_order(r.actors_at(r.depth), true).is_empty());
    assert!(engine::dispatch(&mut r, &Command::Wait).unwrap().took_turn);
    assert_eq!(engine::actor(&r, decoy.id).unwrap(), &decoy);
    assert_eq!(r.metrics.get("aiActors").copied().unwrap_or(0.), ai_before);
    assert!(!engine::move_actor(&mut r, decoy.id, 1., 0., false).unwrap());
    let hp = r.player.hp;
    let player_id = r.player.id;
    engine::attack(&mut r, decoy.id, player_id, 1.).unwrap();
    assert_eq!(r.player.hp, hp);
}

#[test]
fn nightmare_is_targeted_by_actual_enemy_turn_and_disappears_without_a_reward() {
    let mut r = room();
    r.player.abilities.push(nightmares::ID.into());
    r.player.nightmares = 1.0.into();
    let attacker = enemy(&mut r, 14., 10.);
    let decoy = nightmares::create(&mut r, 13., 10.).unwrap();
    let a = engine::actor_mut(&mut r, attacker).unwrap();
    a.conditions.shift_remove("paralyzed");
    a.conditions.insert("targetId".into(), decoy);
    a.conditions.insert("lucky".into(), 1.);
    a.last_seen = ActorLastSeen { x: 13., y: 10. }.into();
    let (hp, exp) = (r.player.hp, r.exp);
    let ground = r.ground.clone();
    let out = engine::dispatch(&mut r, &Command::Wait).unwrap();
    assert!(out.took_turn);
    assert!(engine::actor(&r, decoy).is_none());
    assert_eq!((r.player.hp, r.exp), (hp, exp));
    assert_eq!(r.ground, ground);
    assert!(out.events.iter().any(|e| e.kind == "impact"
        && e.from.as_ref().is_some_and(|p| p.x == 14. && p.y == 10.)
        && e.to.x == 13. && e.to.y == 10.));
}

fn implant_room() -> (Run, f64, f64, f64) {
    let mut r = room();
    r.player.abilities.push(IMPLANT.into());
    let attacker = enemy(&mut r, 13., 10.);
    let victim = enemy(&mut r, 14., 10.);
    let untouched = enemy(&mut r, 13., 11.);
    (r, attacker, victim, untouched)
}

#[test]
fn implant_dispatch_spends_fifteen_and_one_action_for_the_selected_immediate_attack() {
    let (mut r, attacker, victim, untouched) = implant_room();
    let hp = engine::actor(&r, victim).unwrap().hp;
    let untouched_hp = engine::actor(&r, untouched).unwrap().hp;
    let out = engine::dispatch(&mut r, &ability(IMPLANT, Some(attacker), Some((14., 10.)))).unwrap();
    assert!(out.took_turn);
    assert_eq!((r.player.energy, r.tick), (85., 1.));
    assert!(engine::actor(&r, victim).unwrap().hp < hp);
    assert_eq!(engine::actor(&r, untouched).unwrap().hp, untouched_hp);
    assert_eq!(engine::actor(&r, attacker).unwrap().state, "hunting");
    assert!(!engine::actor(&r, attacker).unwrap().conditions.contains_key("temporaryAlly"));
    let attacks: Vec<_> = out.events.iter().filter(|e| e.kind == "impact").collect();
    assert_eq!(attacks.len(), 1); // Paralyzed actors cannot supply a later AI hit.
    assert_eq!(attacks[0].from.as_ref().unwrap().x, 13.);
    assert_eq!((attacks[0].to.x, attacks[0].to.y), (14., 10.));
}

#[test]
fn implant_can_choose_the_player_as_the_adjacent_creature() {
    let mut r = room();
    r.player.abilities.push(IMPLANT.into());
    let attacker = enemy(&mut r, 11., 10.);
    engine::actor_mut(&mut r, attacker).unwrap().conditions.insert("lucky".into(), 1.);
    let out = engine::dispatch(&mut r, &ability(IMPLANT, Some(attacker), Some((10., 10.)))).unwrap();
    assert!(out.took_turn);
    assert_eq!((r.player.energy, r.tick), (85., 1.));
    assert!(r.player.hp < 100.);
    assert!(out.events.iter().any(|e| e.kind == "impact" && e.to.x == 10. && e.to.y == 10.));
}

/// Activation preflight must be fully atomic. Dispatch may record an attempted
/// command and refresh observation, but must not spend resources, RNG or a turn.
fn assert_implant_rejected(mut r: Run, attacker: Option<f64>, point: Option<(f64, f64)>) {
    let cmd = ability(IMPLANT, attacker, point);
    let Command::Ability { id, target, x, y } = &cmd else { unreachable!() };
    let before = serde_json::to_value(&r).unwrap();
    assert!(!abilities::activate(&mut r, id, target, x, y).unwrap());
    assert_eq!(serde_json::to_value(&r).unwrap(), before);
    let (energy, tick, rng, hp) = (r.player.energy, r.tick, r.rng, r.player.hp);
    let actors = r.actors_at(r.depth).to_vec();
    let out = engine::dispatch(&mut r, &cmd).unwrap();
    assert!(!out.took_turn);
    assert_eq!((r.player.energy, r.tick, r.rng, r.player.hp), (energy, tick, rng, hp));
    assert_eq!(r.actors_at(r.depth), actors.as_slice());
    assert!(!out.events.iter().any(|e| e.kind == "impact" || e.kind == "projectile"));
}

#[test]
fn implant_invalid_target_or_nonadjacent_victim_is_atomic() {
    let (r, attacker, _, _) = implant_room();
    assert_implant_rejected(r.clone(), None, Some((14., 10.)));
    assert_implant_rejected(r.clone(), Some(-99.), Some((14., 10.)));
    assert_implant_rejected(r.clone(), Some(attacker), None);
    assert_implant_rejected(r.clone(), Some(attacker), Some((13., 10.))); // Self.
    assert_implant_rejected(r.clone(), Some(attacker), Some((15., 10.))); // Empty.
    assert_implant_rejected(r.clone(), Some(attacker), Some((10., 10.))); // Player too far.
    assert_implant_rejected(r.clone(), Some(attacker), Some((14.5, 10.)));
    let mut hidden = r.clone();
    engine::actor_mut(&mut hidden, attacker).unwrap().conditions.insert("invisible".into(), 10.);
    assert_implant_rejected(hidden, Some(attacker), Some((14., 10.)));
    for state in ["ally", "neutral"] {
        let mut friendly = r.clone();
        engine::actor_mut(&mut friendly, attacker).unwrap().state = state.into();
        assert_implant_rejected(friendly, Some(attacker), Some((14., 10.)));
    }
}

#[test]
fn implant_insufficient_energy_and_silence_reject_before_payment_or_attack() {
    let (r, attacker, _, _) = implant_room();
    let mut poor = r.clone();
    poor.player.energy = 14.;
    assert_implant_rejected(poor, Some(attacker), Some((14., 10.)));
    let mut silenced = r.clone();
    silenced.player.conditions.insert("silenced".into(), 1.);
    assert_implant_rejected(silenced, Some(attacker), Some((14., 10.)));
    let mut aura = r;
    engine::actor_mut(&mut aura, attacker).unwrap().abilities.push("enemy:beast:silence-aura".into());
    assert_implant_rejected(aura, Some(attacker), Some((14., 10.)));
}

fn duel_room() -> (Run, f64, f64) {
    let mut r = room();
    r.player.abilities.push(revision50::DUEL.into());
    let chosen = enemy(&mut r, 11., 10.);
    let outsider = enemy(&mut r, 10., 11.);
    let out = engine::dispatch(&mut r, &ability(revision50::DUEL, Some(chosen), None)).unwrap();
    assert!(!out.took_turn);
    assert!(has_active(&r, revision50::DUEL));
    (r, chosen, outsider)
}

#[test]
fn duel_dispatch_uses_fixed_five_by_five_square_and_only_exempts_chosen_enemy() {
    let (mut r, chosen, outsider) = duel_room();
    assert_eq!((r.player.energy, r.tick), (97., 0.));
    // The selected enemy's activation cell (11,10) is the fixed center.
    for dx in -2..=2 {
        for dy in -2..=2 {
            r.player.x = 11. + f64::from(dx);
            r.player.y = 10. + f64::from(dy);
            assert_eq!(engine::attack_blocker(&r, engine::actor(&r, outsider).unwrap(), &r.player, 0., false), Some("fated-duel"));
            assert_eq!(engine::attack_blocker(&r, engine::actor(&r, outsider).unwrap(), &r.player, 8., false), Some("fated-duel"));
            assert_eq!(engine::attack_blocker(&r, engine::actor(&r, chosen).unwrap(), &r.player, 0., false), None);
        }
    }
    engine::actor_mut(&mut r, chosen).unwrap().x = 18.;
    r.player.x = 13.;
    r.player.y = 12.;
    assert!(revision50::duel_blocks(&r, engine::actor(&r, outsider).unwrap(), &r.player));
    r.player.x = 14.;
    assert!(!revision50::duel_blocks(&r, engine::actor(&r, outsider).unwrap(), &r.player));
}

#[test]
fn duel_actual_attack_blocks_outsider_without_rng_but_allows_selected_enemy() {
    let (mut r, chosen, outsider) = duel_room();
    let player = r.player.id;
    let rng = r.rng;
    engine::attack(&mut r, outsider, player, 1.).unwrap();
    assert_eq!((r.player.hp, r.rng), (100., rng));
    engine::actor_mut(&mut r, chosen).unwrap().conditions.insert("lucky".into(), 1.);
    engine::attack(&mut r, chosen, player, 1.).unwrap();
    assert!(r.player.hp < 100.);
}

#[test]
fn duel_pays_three_per_world_turn_without_energy_regeneration_and_expires_unfunded() {
    let (mut r, _, _) = duel_room();
    r.player.energy = 6.;
    assert!(engine::dispatch(&mut r, &Command::Wait).unwrap().took_turn);
    assert_eq!(r.player.energy, 6., "activation prepaid the first normal upkeep");
    assert!(engine::dispatch(&mut r, &Command::Wait).unwrap().took_turn);
    assert_eq!(r.player.energy, 3.);
    assert!(has_active(&r, revision50::DUEL));
    assert!(engine::dispatch(&mut r, &Command::Wait).unwrap().took_turn);
    assert_eq!(r.player.energy, 0.);
    assert!(has_active(&r, revision50::DUEL), "exact funding covers the paid turn");
    engine::dispatch(&mut r, &Command::Wait).unwrap();
    assert!(!has_active(&r, revision50::DUEL));
    assert_eq!(r.player.energy, 0.);
}

#[test]
fn duel_leaving_area_expires_before_exit_turn_upkeep_and_does_not_reactivate_on_return() {
    let (mut r, _, outsider) = duel_room();
    r.player.x = 9.; // Center x=11, so x=9 is the inclusive left edge.
    let energy = r.player.energy;
    assert!(step(&mut r, -1., 0.).took_turn);
    assert_eq!(r.player.x, 8.);
    assert!(!has_active(&r, revision50::DUEL));
    assert_eq!(r.player.energy, energy); // No charge after leaving the square.
    for key in ["r50DuelTarget", "r50DuelX", "r50DuelY", "r50DuelDepth"] {
        assert!(!r.player.conditions.contains_key(key));
    }
    assert_eq!(engine::attack_blocker(&r, engine::actor(&r, outsider).unwrap(), &r.player, 0., false), None);
    step(&mut r, 1., 0.);
    assert!(!has_active(&r, revision50::DUEL));
    assert_eq!(r.player.energy, energy);
}

#[test]
fn duel_invalid_admission_and_floor_change_do_not_leave_protection_active() {
    let mut r = room();
    r.player.abilities.push(revision50::DUEL.into());
    let distant = enemy(&mut r, 13., 10.);
    let before = serde_json::to_value(&r).unwrap();
    assert!(!abilities::activate(&mut r, revision50::DUEL, &distant.into(), &Field::Missing, &Field::Missing).unwrap());
    assert_eq!(serde_json::to_value(&r).unwrap(), before);
    let (mut r, _, _) = duel_room();
    let mut restored: Run = serde_json::from_value(serde_json::to_value(&r).unwrap()).unwrap();
    assert!(has_active(&restored, revision50::DUEL));
    restored.depth = 2.;
    revision50::settle_duel(&mut restored);
    assert!(!has_active(&restored, revision50::DUEL));
    // After prepaid coverage expires, activation requires the next upkeep cost.
    r.tick += 2.;
    r.player.active.clear();
    r.player.energy = 2.;
    let target = r.actors_at(r.depth)[0].id;
    let before = serde_json::to_value(&r).unwrap();
    assert!(!abilities::activate(&mut r, revision50::DUEL, &target.into(), &Field::Missing, &Field::Missing).unwrap());
    assert_eq!(serde_json::to_value(&r).unwrap(), before);
}

#[test]
fn pursuit_attack_displacement_from_unstoppable_force_never_earns_credit() {
    let (mut r, _) = pursuit_room(18.);
    let blocker = enemy(&mut r, 11., 10.);
    r.player.abilities.push("shrail-unstoppable-force".into());
    r.player.active.push("shrail-unstoppable-force".into());
    assert_eq!(pursuit::route_for_run(&r).get(1), Some(&(11., 10.)));
    let hp = engine::actor(&r, blocker).unwrap().hp;
    assert!(step(&mut r, 1., 0.).took_turn);
    assert!(engine::actor(&r, blocker).unwrap().hp < hp);
    assert_eq!(r.player.x, 11.); // Forced advance, not a voluntary move.
    assert_eq!(engine::actor(&r, blocker).unwrap().x, 12.);
    assert_eq!((r.tick, credit(&r)), (1., 0.));
}

#[test]
fn nightmare_ranged_miss_retains_and_landed_projectile_dissolves_decoy() {
    let mut r = room();
    r.player.abilities.push(nightmares::ID.into());
    r.player.nightmares = 1.0.into();
    let attacker = enemy(&mut r, 15., 10.);
    let decoy = nightmares::create(&mut r, 12., 10.).unwrap();
    engine::actor_mut(&mut r, attacker).unwrap().conditions.insert("distracted".into(), 1.);
    assert!(!engine::attack_ranged(&mut r, attacker, decoy, 0., 3., 1., false).unwrap());
    assert!(engine::actor(&r, decoy).is_some());
    let (hp, exp, next_id) = (r.player.hp, r.exp, r.next_id);
    let ground = r.ground.clone();
    engine::actor_mut(&mut r, attacker).unwrap().conditions.insert("lucky".into(), 1.);
    assert!(engine::attack_ranged(&mut r, attacker, decoy, 0., 3., 1., false).unwrap());
    assert!(engine::actor(&r, decoy).is_none());
    assert_eq!((r.player.hp, r.exp, r.next_id), (hp, exp, next_id));
    assert_eq!(r.ground, ground);
}

#[test]
fn implant_exact_fifteen_energy_is_sufficient_and_leaves_zero() {
    let (mut r, attacker, victim, _) = implant_room();
    r.player.energy = 15.;
    let hp = engine::actor(&r, victim).unwrap().hp;
    let out = engine::dispatch(&mut r, &ability(IMPLANT, Some(attacker), Some((14., 10.)))).unwrap();
    assert!(out.took_turn);
    assert_eq!((r.player.energy, r.tick), (0., 1.));
    assert!(engine::actor(&r, victim).unwrap().hp < hp);
}

#[test]
fn duel_haste_movement_does_not_charge_an_extra_upkeep_turn() {
    let (mut r, _, _) = duel_room();
    r.player.conditions.insert("hasted".into(), 10.);
    r.phase = "haste".to_string().into();
    assert!(!step(&mut r, -1., 0.).took_turn);
    assert_eq!((r.player.x, r.tick, r.player.energy), (9., 0., 97.));
    assert!(has_active(&r, revision50::DUEL));
    assert!(engine::dispatch(&mut r, &Command::Wait).unwrap().took_turn);
    assert_eq!((r.tick, r.player.energy), (1., 97.));
    assert!(has_active(&r, revision50::DUEL));
}

#[test]
fn implant_uses_its_explicit_own_reserve_even_with_zero_player_energy() {
    let (mut r, attacker, victim, _) = implant_room();
    r.player.energy = 0.;
    // This charge is explicit test data, not an authored capacity assumption.
    r.player.ability_reserves.as_mut().unwrap().insert(
        IMPLANT.into(), ability_energy::authored_reserve(15., 15.).unwrap());
    let hp = engine::actor(&r, victim).unwrap().hp;
    let command = ability(IMPLANT, Some(attacker), Some((14., 10.)));
    assert!(engine::dispatch(&mut r, &command).unwrap().took_turn);
    assert_eq!((r.player.energy, r.tick), (0., 1.));
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()[IMPLANT].energy, 0.);
    assert!(engine::actor(&r, victim).unwrap().hp < hp);
    // A world turn must not regenerate this spent reserve or the player bar.
    let hp = engine::actor(&r, victim).unwrap().hp;
    assert!(!engine::dispatch(&mut r, &command).unwrap().took_turn);
    assert_eq!((r.player.energy, r.tick), (0., 1.));
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()[IMPLANT].energy, 0.);
    assert_eq!(engine::actor(&r, victim).unwrap().hp, hp);
}

#[test]
fn implant_missing_reserve_never_silently_falls_back_to_player_energy() {
    let (mut r, attacker, _, _) = implant_room();
    r.player.ability_reserves.as_mut().unwrap().shift_remove(IMPLANT);
    assert_implant_rejected(r, Some(attacker), Some((14., 10.)));
}
