//! Direct draft projection/application tests, not production R50 admission.
use corpse_core::{Field, Item, Run, revision50_item_knowledge as knowledge};

fn run(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into();
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.equipment_abilities = Field::Missing;
    r.inventory.clear(); r.actors = Default::default(); r.ground.clear();
    r.history.clear(); r.haste_snapshot = Field::Missing; r.future = None;
    r.known_potions.clear(); r.known_potion_natures = Field::Missing;
    r
}
fn potion(id: f64, effect: &str) -> Item {
    Item { id, kind: "potion".into(), name: "Unidentified potion".into(),
        effect: effect.to_string().into(), identified: false.into(), glyph: "!".into(), ..Default::default() }
}
#[test]
fn r50_item_knowledge_all_potions_are_known_without_mutating_save_or_rng() {
    let r = run(50.0);
    let before = serde_json::to_vec(&r).unwrap();
    for effect in corpse_core::revision50_potions::ORDINARY.into_iter().chain(["Veridian Sap"]) {
        let item = potion(1.0, effect);
        let item_before = item.clone();
        let name = knowledge::display_name(&r, &item);
        let description = corpse_core::inventory::potion_description(&r, &item);
        assert_eq!(name, knowledge::potion_name(effect).unwrap());
        assert!(description.starts_with(&name), "{effect}: {description}");
        assert!(description.contains(&knowledge::potion_effect_description(effect, false).unwrap()));
        for hidden in ["Unidentified", "Might be:", "Identified when used"] {
            assert!(!description.contains(hidden), "{effect}: {description}");
        }
        assert_eq!(item, item_before);
    }
    assert_eq!(serde_json::to_vec(&r).unwrap(), before);
}
#[test]
fn r50_item_knowledge_diluted_values_and_fixed_footprints_match_effects() {
    for (effect, fragments) in [
        ("healing", vec!["25 HP"]), ("energy", vec!["25 energy"]),
        ("paralysis", vec!["3 turns"]), ("telepathy", vec!["15 turns"]),
        ("levitation", vec!["10 turns"]), ("speed", vec!["5 turns"]),
        ("invisibility", vec!["10 turns"]), ("fire immunity", vec!["10 turns"]),
        ("darkness", vec!["by 25", "5×5", "10 turns"]),
        ("flooding", vec!["3×3", "10 turns"]),
        ("freezing", vec!["3×3", "2 turns", "until melted"]),
        ("repulsion", vec!["within 3 tiles", "up to 2 tiles"]),
        ("discord", vec!["5 turns"]), ("shattering", vec!["within 2 tiles"]),
    ] {
        let detail = knowledge::potion_effect_description(effect, true).unwrap();
        for fragment in fragments { assert!(detail.contains(fragment), "{effect}: {detail}"); }
    }
}
#[test]
fn r50_item_knowledge_drink_throw_and_restore_never_discover_items() {
    let mut r = run(50.0);
    r.known_potions.push("invisibility".into());
    r.inventory = vec![potion(900.0, "healing"), potion(901.0, "healing")];
    let snapshot = corpse_core::temporal::Snapshot::capture(&r);
    let peer = r.inventory[1].clone();
    let (x, y) = (r.player.x, r.player.y);
    assert!(corpse_core::potions::apply(&mut r, 0, x, y, false).unwrap());
    assert_eq!(r.known_potions, ["invisibility"]);
    assert_eq!(r.inventory[0], peer);
    assert!(!r.log.iter().any(|line| line.contains("Identified:")));
    assert!(r.log.iter().any(|line| line == "You drink Potion of Healing."));
    assert!(corpse_core::potions::apply(&mut r, 0, x, y, true).unwrap());
    assert_eq!(r.known_potions, ["invisibility"]);
    assert!(!r.log.iter().any(|line| line.contains("Identified:")));
    assert!(r.log.iter().any(|line| line == "You throw Potion of Healing."));
    snapshot.restore(&mut r).unwrap();
    assert!(!r.inventory[0].identified.value_or(false));
    assert_eq!(knowledge::display_name(&r, &r.inventory[0]), "Potion of Healing");
}
#[test]
fn r50_item_knowledge_legacy_discovery_remains_unchanged() {
    let mut r = run(49.0);
    r.inventory = vec![potion(900.0, "healing"), potion(901.0, "healing")];
    assert!(corpse_core::inventory::potion_description(&r, &r.inventory[0]).contains("Identified when used"));
    let (x,y) = (r.player.x, r.player.y);
    assert!(corpse_core::potions::apply(&mut r, 0, x, y, false).unwrap());
    assert_eq!(r.known_potions, ["healing"]);
    assert!(r.inventory[0].identified.value_or(false));
    assert!(r.log.iter().any(|line| line == "Identified: Potion of healing."));
}
#[test]
fn r50_item_knowledge_names_saved_jewelry_without_renaming_it() {
    let r = run(50.0);
    for (kind, effect, name) in [("ring", "transference", "Ring of Vampirism"),
        ("amulet", "lightning", "Amulet of Lightning")] {
        let item = Item { kind: kind.into(), effect: effect.to_string().into(),
            name: "Old draft label".into(), ..Default::default() };
        assert_eq!(knowledge::display_name(&r,&item),name);
        assert_eq!(item.name,"Old draft label");
        let legacy = run(49.0);
        assert_eq!(knowledge::display_name(&legacy,&item),"Old draft label");
    }
}
#[test]
fn r50_item_knowledge_pickup_and_drop_logs_use_the_canonical_name() {
    let mut r=run(50.0);
    let item=potion(900.0,"healing");let (x,y)=(r.player.x,r.player.y);
    r.ground_at_mut(r.depth).push(corpse_core::GroundItem{x,y,item,..Default::default()});
    assert_eq!(corpse_core::inventory::collect_underfoot(&mut r,22.0).unwrap(),1);
    assert!(r.log.iter().any(|line|line=="You pick up Potion of Healing."));
    assert_eq!(corpse_core::inventory::command(&mut r,&corpse_core::Command::Drop{id:900.0}).unwrap(),Some(true));
    assert!(r.log.iter().any(|line|line=="You drop Potion of Healing."));
    assert!(r.known_potions.is_empty());
}
