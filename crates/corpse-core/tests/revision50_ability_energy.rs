use corpse_core::{abilities, Field, Run, revision50_ability_energy as energy};
fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 50.0.into();
    r.player.energy = 0.; r.player.active.clear(); r.player.conditions.clear();
    r
}
#[test]
fn paid_activation_uses_its_reserve_without_player_energy() {
    let mut r = room(); let id = abilities::ALL_SEEING;
    r.player.abilities = vec![id.into()];
    energy::initialize_reserve(&mut r.player, id, 30., 30.).unwrap();
    // The return value means a turn was spent, not that activation succeeded.
    assert!(!abilities::activate(&mut r, id, &Field::Missing, &Field::Missing, &Field::Missing).unwrap());
    assert_eq!(r.player.energy, 0.);
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()[id].energy, 0.);
    assert_eq!(r.player.conditions.get("allSeeingReveal"), Some(&1.));
}
#[test]
fn missing_capacity_rejects_paid_activation_without_spending_player_energy() {
    let mut r = room(); let id = abilities::ALL_SEEING;
    r.player.abilities = vec![id.into()]; r.player.energy = 100.;
    assert!(!abilities::activate(&mut r, id, &Field::Missing, &Field::Missing, &Field::Missing).unwrap());
    assert_eq!(r.player.energy, 100.);
    assert!(!r.player.conditions.contains_key("allSeeingReveal"));
}
#[test]
fn repeated_initialization_cannot_refresh_charge() {
    let mut r = room();
    energy::initialize_reserve(&mut r.player, "test:ability", 30., 0.).unwrap();
    assert_eq!(energy::initialize_reserve(&mut r.player, "test:ability", 30., 30.), Err(energy::EnergyError::AlreadyInitialized));
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()["test:ability"].energy, 0.);
}
#[test]
fn upkeep_uses_reserve_then_exhausted_player_fallback() {
    let mut r = room(); let id = "dawel-shadow-in-the-dark";
    let cost = abilities::upkeep_cost(id); assert!(cost > 0.);
    r.player.abilities = vec![id.into()]; r.player.active = vec![id.into()];
    energy::initialize_reserve(&mut r.player, id, cost, cost).unwrap();
    r.player.energy = cost;
    let actor_id = r.player.id;
    abilities::pay_upkeep(&mut r, actor_id).unwrap();
    assert_eq!(r.player.energy, cost);
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()[id].energy, 0.);
    r.tick += 1.;
    abilities::pay_upkeep(&mut r, actor_id).unwrap(); assert_eq!(r.player.energy, 0.);
    assert!(r.player.active.iter().any(|x| x == id));
    r.tick += 1.;
    abilities::pay_upkeep(&mut r, actor_id).unwrap(); assert!(!r.player.active.iter().any(|x| x == id));
}
