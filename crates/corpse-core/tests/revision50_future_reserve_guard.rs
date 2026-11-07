//! Future payment is implemented independently of the overall R50 release gate.
use corpse_core::{Command, Field, Run, revision50_ability_energy as energy, temporal::{self, Engine, History}};
const ID: &str = "morae-see-the-future";
const OTHER: &str = "numo-distract";
fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into();
    r.player.abilities = vec![ID.into()]; r.player.active.clear(); r.player.conditions.clear();
    r.player.ability_reserves = Field::Missing; r.player.ability_order = Field::Missing;
    r.player.equipment_abilities = Field::Missing;
    r.inventory.clear(); r.offers.clear();
    // Keep the floor block keys required by canonical save/anchor validation.
    // Emptying the maps removes the world schema, not just the fixture contents.
    for actors in r.actors.values_mut() { *actors = Default::default(); }
    for ground in r.ground.values_mut() { *ground = Default::default(); }
    r.history.clear(); r.haste_snapshot = Field::Missing; r.future = None;
    r
}
fn command() -> Command { Command::Ability { id: ID.into(), target: Field::Missing, x: Field::Missing, y: Field::Missing } }
fn charge(r: &mut Run, own: f64, shared: f64) {
    energy::initialize_reserve(&mut r.player, ID, 50., own).unwrap();
    energy::initialize_reserve(&mut r.player, OTHER, 50., 17.).unwrap();
    r.player.energy = shared;
}
#[test]
fn revision50_capture_drains_exactly_own_and_player_and_saves_paid_state() {
    for (own, shared) in [(50.,40.), (50.,0.), (0.,40.), (0.,0.)] {
        let mut r = room(50.); charge(&mut r,own,shared);
        temporal::remember_future(&mut r, &History::default()).unwrap();
        assert_eq!(r.player.energy,0.);
        assert_eq!(r.player.ability_reserves.as_ref().unwrap()[ID].energy,0.);
        assert_eq!(r.player.ability_reserves.as_ref().unwrap()[OTHER].energy,17.);
        let anchor = r.future.as_ref().unwrap().native_state().unwrap();
        assert_eq!(anchor.player.energy,0.);
        assert_eq!(anchor.player.ability_reserves,r.player.ability_reserves);
        assert_eq!(r.time_costs.as_ref().unwrap().energy,shared);
    }
}
#[test]
fn revision50_missing_or_invalid_payment_rejects_without_bookkeeping() {
    for invalid in [false,true] {
        let mut r = room(50.); r.player.energy=40.;
        if invalid {charge(&mut r,50.,40.);r.player.ability_reserves.as_mut().unwrap()[ID].energy=-1.;}
        let mut e=Engine {run:r,history:History::default()};
        let before=e.run.clone(); assert!(e.dispatch(&command()).is_err()); assert_eq!(e.run,before);
        assert!(temporal::remember_future(&mut e.run,&e.history).is_err()); assert_eq!(e.run,before);
    }
}
#[test]
fn revision50_reserve_only_capture_is_silenced_but_free_capture_and_return_are_not() {
    for (own, shared) in [(50.,0.),(0.,40.),(0.,0.)] {
        let mut r=room(50.);charge(&mut r,own,shared);r.player.conditions.insert("silenced".into(),2.);
        // Source-level harness leaves production Engine/startup admission unchanged.
        let mut e=Engine {run:r,history:History::default()};
        e.dispatch(&command()).unwrap();
        assert_eq!(e.run.future.is_some(),own+shared==0.);
        assert_eq!(e.run.player.energy,shared);
        assert_eq!(e.run.player.ability_reserves.as_ref().unwrap()[ID].energy,own);
        if own+shared==0. {
            e.run.player.energy=12.;e.dispatch(&command()).unwrap();
            assert!(e.run.future.is_none());assert_eq!(e.run.player.energy,0.);
        }
    }
}
#[test]
fn revision50_raw_save_restore_preserves_paid_and_other_pools() {
    let mut r=room(50.);charge(&mut r,32.,21.);
    r.player.reaping_ledger=corpse_core::revision50_amulets::ReapingLedger{damage_carry:19.}.into();
    temporal::remember_future(&mut r,&History::default()).unwrap();
    let wire=corpse_core::storage::serialize(&r).unwrap();
    let mut restored=corpse_core::storage::deserialize(&wire).unwrap();
    restored.player.energy=12.;
    restored.player.reaping_ledger.as_mut().unwrap().damage_carry=2.;
    restored.player.ability_reserves.as_mut().unwrap()[ID].energy=9.;
    restored.player.ability_reserves.as_mut().unwrap()[OTHER].energy=1.;
    assert!(temporal::restore_future(&mut restored,&mut History::default()).unwrap());
    assert_eq!(restored.player.energy,0.);
    assert_eq!(restored.player.ability_reserves.as_ref().unwrap()[ID].energy,0.);
    assert_eq!(restored.player.ability_reserves.as_ref().unwrap()[OTHER].energy,17.);
    assert_eq!(restored.player.reaping_ledger.as_ref().unwrap().damage_carry,19.);
}
#[test]
fn legacy_capture_preserves_all_shared_energy_payment() {
    for revision in [29.,46.] {
        let mut r=room(revision);charge(&mut r,50.,40.);
        temporal::remember_future(&mut r,&History::default()).unwrap();
        assert_eq!(r.player.energy,0.);assert!(r.future.is_some());
        assert_eq!(r.player.ability_reserves.as_ref().unwrap()[ID].energy,50.);
    }
}
