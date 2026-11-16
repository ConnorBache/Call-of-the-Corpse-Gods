use super::*;
fn scene() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 38.0.into();
    r.offers.clear(); r.player.conditions.clear();
    r
}
#[test]
fn authored_copy_r38_dream_descriptions_are_exact() {
    let mut r = scene();
    r.dreamwalking = corpse_core::dreamwalking::State { unlocked: true, ..Default::default() }.into();
    assert_eq!(dream_row(&r).unwrap().description, "Leave your sleeping body behind to enter the Realm of Dreams. Toggle again to awake.");
    r.dreamwalking = corpse_core::dreamwalking::State { active: true, depth: r.depth, ..Default::default() }.into();
    assert_eq!(dream_row(&r).unwrap().description, "Awake from the realm of dreams.");
}
#[test]
fn authored_copy_r38_mask_and_teleport_are_separate_and_readiness_survives() {
    let mut r = scene(); let mask = corpse_core::ahvamitt::mask(100.);
    assert_eq!(mask.effect.as_ref().unwrap(), "Grants Teleport while in your inventory.");
    assert_eq!(crate::pack::describe_item(&r, &mask), "Mask of Trickery\nGrants Teleport while in your inventory.");
    r.inventory.push(mask.clone());
    assert_eq!(corpse_core::ahvamitt::COOLDOWN, 100.);
    for remaining in [0., 100., 1.] {
        r.player.conditions.insert("ahvamittTeleportReadyAt".into(), r.tick + remaining);
        let row = teleport_row(&r).unwrap();
        assert_eq!(row.description, "Teleport to a random location on the floor.");
        assert_eq!(row.state, if remaining > 0. {"Unavailable"} else {"Ready"});
        assert!(row.cost.contains("cooldown"));
        assert_eq!(crate::pack::describe_item(&r, &mask), "Mask of Trickery\nGrants Teleport while in your inventory.");
    }
}
#[test]
fn authored_copy_r38_queen_catalog_matches_runtime() {
    let expected = "Queens such as this take care of their nests, spawning new young and weaving royal jelly to nourish them.";
    assert_eq!(corpse_core::new_creatures::QUEEN_HISTORY, expected);
    let catalog: serde_json::Value = serde_json::from_str(include_str!("../assets/manual.json")).unwrap();
    assert!(catalog.to_string().contains(expected));
}
