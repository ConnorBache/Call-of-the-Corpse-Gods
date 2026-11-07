//! Source-only regression coverage; native execution is still required.
use corpse_core::{Command, Run, engine, revision50_ability_energy as energy};

fn room(id: &str, revision: f64, reserve: f64, shared: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into();
    r.player.abilities = vec![id.into()];
    r.player.active = vec![id.into()];
    r.player.conditions.clear();
    r.player.energy = shared;
    r.inventory.clear();
    r.ground.clear();
    r.offers.clear();
    let depth = r.depth;
    r.actors_at_mut(depth).clear();
    energy::initialize_reserve(&mut r.player, id, reserve.max(20.), reserve).unwrap();
    r
}

fn free_finish(r: &mut Run) {
    // Rally reaches the real dispatch finish boundary without world-turn upkeep.
    let command = Command::Rally { x: r.player.x, y: r.player.y };
    let tick = r.tick;
    let outcome = engine::dispatch(r, &command).unwrap();
    assert!(!outcome.took_turn);
    assert_eq!(r.tick, tick);
}

#[test]
fn reserve_funded_active_modes_survive_real_free_command_finish() {
    for id in ["etill-phase-through", "dawel-shadow-in-the-dark"] {
        let mut r = room(id, 50., 20., 0.);
        free_finish(&mut r);
        assert!(r.player.active.iter().any(|a| a == id), "{id} lost its funded activation");
        assert_eq!(r.player.energy, 0.);
        assert_eq!(r.player.ability_reserves.as_ref().unwrap()[id].energy, 20.);
    }
}

#[test]
fn exhausted_reserve_uses_shared_affordability_at_finish() {
    let id = "dawel-shadow-in-the-dark";
    let mut r = room(id, 50., 0., 2.);
    free_finish(&mut r);
    assert!(r.player.active.iter().any(|a| a == id));
    assert_eq!(r.player.energy, 2., "free finish must not charge upkeep");
    r.player.energy = 0.;
    free_finish(&mut r);
    assert!(!r.player.active.iter().any(|a| a == id));
}

#[test]
fn legacy_finish_ignores_future_reserve_fields() {
    let id = "dawel-shadow-in-the-dark";
    let mut r = room(id, 49., 20., 0.);
    free_finish(&mut r);
    assert!(!r.player.active.iter().any(|a| a == id));
    assert_eq!(r.player.ability_reserves.as_ref().unwrap()[id].energy, 20.);
}
