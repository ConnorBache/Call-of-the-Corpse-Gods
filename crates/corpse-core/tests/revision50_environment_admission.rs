use corpse_core::{Actor, Run, environment, progression, revision50, revision50_combat};

fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into();
    r.player.abilities.clear();
    r.player.active.clear();
    r.player.conditions.clear();
    let depth = r.depth;
    r.actors_at_mut(depth).clear();
    r
}

fn npc(id: &str, state: &str) -> Actor {
    Actor { id: 99., x: 13., y: 14., hp: 10., max_hp: 10.,
        state: state.into(), abilities: vec![id.into()], ..Default::default() }
}

fn assert_rejected_without_mutation(r: &Run) {
    let before = r.clone();
    assert!(environment::validate_supported(r).is_err());
    assert_eq!(*r, before);
    assert!(environment::validate_supported_with_falls(r, &[r.player.id]).is_err());
    assert_eq!(*r, before);
}

#[test]
fn all_integrated_revision50_player_powers_pass_environment_admission() {
    let ids: Vec<_> = progression::catalog_for_revision(50.).iter()
        .filter(|a| revision50::supported(&a.id)).map(|a| a.id.clone()).collect();
    assert!(!ids.is_empty());
    for id in ids {
        let mut r = room(50.);
        r.player.abilities.push(id.clone());
        if revision50::toggle(&id) { r.player.active.push(id.clone()); }
        let before = r.clone();
        assert!(environment::validate_supported(&r).is_ok(), "{id}");
        assert_eq!(r, before);
    }
}

#[test]
fn revision50_power_admission_is_not_backported_to_legacy_revisions() {
    for revision in [29., 41., 46., 49.] {
        for id in ["shrail-execute", "ursa-oblivious", "kative-clay-army"] {
            let mut r = room(revision);
            r.player.abilities.push(id.into());
            assert_rejected_without_mutation(&r);
        }
    }
}

#[test]
fn new_player_powers_do_not_admit_unrelated_hostile_or_allied_npcs() {
    for state in ["hunting", "ally", "neutral"] {
        for id in ["shrail-execute", "ursa-oblivious", "kative-clay-army"] {
            let mut r = room(50.);
            let depth = r.depth;
            r.actors_at_mut(depth).push(npc(id, state));
            assert_rejected_without_mutation(&r);
        }
    }
}

#[test]
fn shared_player_endpoint_is_admitted_but_foreign_owner_is_not() {
    let mut r = room(50.);
    let mut endpoint = npc("shrail-execute", "ally");
    endpoint.conditions.insert("halfHereClone".into(), 1.);
    endpoint.conditions.insert("halfHereOwner".into(), r.player.id);
    endpoint.active.push("shrail-execute".into());
    let depth = r.depth;
    r.actors_at_mut(depth).push(endpoint);
    assert!(environment::validate_supported(&r).is_ok());
    r.actors_at_mut(depth)[0].conditions.insert("halfHereOwner".into(), 123.);
    assert_rejected_without_mutation(&r);
}

#[test]
fn explicit_caster_marker_is_inert_admission_only_at_revision50() {
    for revision in [46., 49., 50.] {
        let mut r = room(revision);
        let depth = r.depth;
        r.actors_at_mut(depth).push(npc(revision50_combat::MAGICKAL_CASTER, "hunting"));
        if revision < 50. {
            assert_rejected_without_mutation(&r);
        } else {
            let before = r.clone();
            assert!(environment::validate_supported(&r).is_ok());
            assert_eq!(r, before);
        }
    }
}

#[test]
fn valid_revision50_power_does_not_mask_an_unknown_later_power() {
    let mut r = room(50.);
    r.player.abilities = vec!["shrail-execute".into(), "test:unsupported-power".into()];
    assert_rejected_without_mutation(&r);
    r.player.abilities.reverse();
    assert_rejected_without_mutation(&r);
}
