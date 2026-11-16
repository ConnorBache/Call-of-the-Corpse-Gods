//! Outcome receipts for inventory-only feedback. No new inventory mechanics.
use crate::revision50_amulet_effects::{self as effects, Effect};
use crate::revision50_amulets::{self as amulets, AmuletSpec, AmuletState};
use crate::{events, Actor, Command, Field, Item, Run};
fn room() -> Run {
    let mut r: Run =
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 52.0.into();
    r.player.x = 10.;
    r.player.y = 10.;
    r.player.abilities.clear();
    r.player.active.clear();
    r.player.conditions.clear();
    r.player.ability_reserves = Field::Missing;
    r.player.hp = r.player.max_hp;
    r.player.energy = 50.;
    r.actors.clear();
    r.inventory.clear();
    r.ground.clear();
    let f = r.floor_mut().unwrap();
    f.tiles.fill("floor".into());
    f.lights.fill(100.);
    r
}
fn potion(id: f64) -> Item {
    Item {
        id,
        name: "Potion of healing".into(),
        kind: "potion".into(),
        effect: "healing".to_string().into(),
        ..Default::default()
    }
}
fn marks(events: &[events::PresentationEvent], key: &str) -> Vec<(String, Option<f64>)> {
    events
        .iter()
        .filter_map(|e| e.animation.as_ref())
        .filter(|a| a.proposal == key && a.surface.as_deref() == Some("inventory"))
        .map(|a| (a.style.clone(), a.item_id))
        .collect()
}
#[test]
fn inventory_tools_marks_actual_free_consumption_only() {
    for tools in [false, true] {
        let mut r = room();
        if tools {
            r.player
                .abilities
                .push("isheilah-tools-of-the-trade".into());
        }
        r.inventory.push(potion(900.));
        events::begin();
        assert_eq!(
            crate::inventory::command(&mut r, &Command::Use { id: 900. }).unwrap(),
            Some(true)
        );
        assert_eq!(marks(&events::finish(), "P14").len(), usize::from(tools));
        events::begin();
        assert_eq!(
            crate::inventory::command(&mut r, &Command::Use { id: 900. }).unwrap(),
            Some(false)
        );
        assert!(marks(&events::finish(), "P14").is_empty());
        r.inventory.push(potion(901.));
        events::begin();
        crate::potions::apply(&mut r, 0, 10., 10., true).unwrap();
        assert!(marks(&events::finish(), "P14").is_empty());
    }
}
#[test]
fn inventory_tools_heart_consumption_and_invalid_potion_outcomes() {
    let mut r = room();
    r.player
        .abilities
        .push("isheilah-tools-of-the-trade".into());
    let rat = crate::population::create_actor_revision("rat", 910., 10., 10., false, 52.).unwrap();
    r.inventory
        .push(crate::inventory::heart_for_revision(&rat, 900., 52.).unwrap());
    events::begin();
    assert_eq!(
        crate::inventory::command(&mut r, &Command::Use { id: 900. }).unwrap(),
        Some(true)
    );
    assert_eq!(
        marks(&events::finish(), "P14"),
        vec![("speed".into(), Some(900.))]
    );
    let mut invalid = potion(901.);
    invalid.effect = Field::Missing;
    r.inventory.push(invalid);
    events::begin();
    crate::inventory::command(&mut r, &Command::Use { id: 901. }).unwrap();
    assert!(marks(&events::finish(), "P14").is_empty());
    assert!(r.inventory.iter().any(|i| i.id == 901.));
}
#[test]
fn inventory_bottomless_retains_then_consumes_actual_remainder() {
    let mut r = room();
    r.player.abilities.push("kative-bottomless-pack".into());
    r.inventory.push(potion(900.));
    let rng = r.rng;
    events::begin();
    crate::inventory::command(&mut r, &Command::Use { id: 900. }).unwrap();
    assert_eq!(
        marks(&events::finish(), "P66"),
        vec![("split-potion".into(), Some(900.))]
    );
    assert!(r.inventory[0].diluted.value_or(false));
    events::begin();
    crate::inventory::command(&mut r, &Command::Use { id: 900. }).unwrap();
    assert_eq!(
        marks(&events::finish(), "P66"),
        vec![("fade-potion".into(), Some(900.))]
    );
    assert!(r.inventory.is_empty());
    assert_eq!(r.rng, rng);
}
#[test]
fn inventory_payments_mark_only_displays_actually_charged_and_stale_plan_is_quiet() {
    for (reserve, cost, main_after, want) in [
        (10., 10., 50., vec![("reserve-drain".into(), Some(900.))]),
        // Pre-amendment R52 charge is capped before payment, not spent as 50.
        (
            50., 15., 45.,
            vec![
                ("reserve-drain".into(), Some(900.)),
                ("energy-drain".into(), None),
            ],
        ),
        (
            5., 15., 40.,
            vec![
                ("reserve-drain".into(), Some(900.)),
                ("energy-drain".into(), None),
            ],
        ),
        (0., 15., 35., vec![("energy-drain".into(), None)]),
    ] {
        let mut r = room();
        let mut item = effects::make_amulet(Effect::Poison, 900.).unwrap();
        item.amulet_state = AmuletState {
            instance_id: 900,
            energy: reserve,
            ready_at_half_tick: 0,
        }
        .into();
        r.inventory.push(item);
        let spec = AmuletSpec {
            capacity: 50.,
            activation_cost: cost,
        };
        let plan = amulets::plan_item_activation(&r, 900., Some(&spec)).unwrap();
        events::begin();
        amulets::commit_item_activation(&mut r, 900., &plan, Some(&spec)).unwrap();
        assert_eq!(marks(&events::finish(), "amulet"), want);
        assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy, 0.);
        assert_eq!(r.player.energy, main_after);
        let before = r.clone();
        events::begin();
        assert!(amulets::commit_item_activation(&mut r, 900., &plan, Some(&spec)).is_err());
        assert!(events::finish().is_empty());
        assert_eq!(r, before);
    }
}
#[test]
fn inventory_spellslinger_spark_only_first_free_activation_not_second_or_zero_cost_payment() {
    let mut r = room();
    r.player.abilities = vec!["isheilah-spellslinger".into(), "kative-appropriate".into()];
    r.inventory
        .push(effects::make_amulet(Effect::Poison, 900.).unwrap());
    let spec = AmuletSpec {
        capacity: 50.,
        activation_cost: 10.,
    };
    for first in [true, false] {
        let plan = amulets::plan_item_activation(&r, 900., Some(&spec)).unwrap();
        events::begin();
        amulets::commit_item_activation(&mut r, 900., &plan, Some(&spec)).unwrap();
        let out = events::finish();
        assert!(marks(&out, "amulet").is_empty());
        assert_eq!(marks(&out, "P18").len(), usize::from(first));
        assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy, 10.);
        assert_eq!(r.player.energy, 50.);
    }
}
#[test]
fn inventory_reaping_only_glints_reserves_that_increase_without_a_victim_trail() {
    let mut r = room();
    r.inventory.push(Item {
        id: 800.,
        kind: "ring".into(),
        effect: "reaping".to_string().into(),
        ..Default::default()
    });
    for (id, energy) in [(900., Some(9.)), (901., Some(10.)), (902., None), (903., Some(50.))] {
        let mut item = effects::make_amulet(Effect::Poison, id).unwrap();
        item.amulet_state = energy
            .map(|energy| AmuletState {
                instance_id: id as u64,
                energy,
                ready_at_half_tick: 0,
            })
            .map(Field::Value)
            .unwrap_or(Field::Missing);
        r.inventory.push(item);
    }
    let attacker = r.player.clone();
    let target = Actor {
        id: 999.,
        hp: 100.,
        max_hp: 100.,
        x: 1000.,
        y: 1000.,
        state: "hunting".into(),
        ..Default::default()
    };
    r.actors_at_mut(r.depth).push(target.clone());
    events::begin();
    amulets::reap_direct_hit(&mut r, &attacker, &target, 20.);
    let out = events::finish();
    assert_eq!(
        marks(&out, "reaping"),
        vec![("reserve-rise".into(), Some(900.))]
    );
    for id in [900., 901., 903.] {
        assert_eq!(r.inventory.iter().find(|i| i.id == id).unwrap().amulet_state.as_ref().unwrap().energy, 10.);
    }
    assert!(r.inventory.iter().find(|i| i.id == 902.).unwrap().amulet_state.is_missing());
    assert_eq!(r.player.energy, 50.);
    assert!(out
        .iter()
        .all(|e| e.animation.as_ref().unwrap().cells.is_empty() && e.from.is_none()));
    events::begin();
    amulets::reap_direct_hit(&mut r, &attacker, &target, 20.);
    assert!(events::finish().is_empty());
}
#[test]
fn inventory_tinker_shows_actual_granted_god_and_failed_reuse_is_quiet() {
    let mut base = room();
    base.player.energy = 150.;
    base.player.abilities.push("kative-tinker".into());
    crate::revision50_ability_energy::initialize_reserve(&mut base.player, "kative-tinker", 0., 0.)
        .unwrap();
    base.gods = (0..12).filter(|g| *g != 8).map(|g| g as f64).collect();
    base.inventory.push(Item {
        id: 900.,
        name: "Ring".into(),
        kind: "ring".into(),
        ..Default::default()
    });
    let mut success = None;
    for seed in 1..=1000 {
        let mut r = base.clone();
        r.rng = seed as f64;
        events::begin();
        let ok = crate::tinker::activate(&mut r, Some(900.)).is_ok_and(|v| v);
        let out = events::finish();
        if ok {
            success = Some((seed, r, out));
            break;
        }
        assert!(marks(&out, "P65").is_empty());
    }
    let (seed, mut r, out) = success.unwrap();
    let grant = crate::tinker::grants(&r.inventory[0]).next().unwrap();
    let god = crate::progression::definition_for_revision(52., grant)
        .unwrap()
        .god_id as u32;
    assert_eq!(
        marks(&out, "P65"),
        vec![
            ("gear".into(), Some(900.)),
            (format!("spark-god-{god}"), Some(900.))
        ]
    );
    let mut plain = base.clone();
    plain.rng = seed as f64;
    crate::tinker::activate(&mut plain, Some(900.)).unwrap();
    assert_eq!(plain, r);
    let before = r.clone();
    events::begin();
    assert!(!crate::tinker::activate(&mut r, Some(900.)).unwrap());
    assert!(events::finish().is_empty());
    assert_eq!(r.inventory, before.inventory);
    assert_eq!(r.player, before.player);
    assert_eq!(r.rng, before.rng);
}
#[test]
fn inventory_full_health_and_energy_only_do_not_fabricate_healing() {
    for effect in ["healing", "energy"] {
        let mut r = room();
        let mut item = potion(900.);
        item.effect = effect.to_owned().into();
        r.inventory.push(item);
        events::begin();
        crate::inventory::command(&mut r, &Command::Use { id: 900. }).unwrap();
        let out = events::finish();
        assert!(!out.iter().any(|e| e
            .animation
            .as_ref()
            .is_some_and(|a| a.stage == events::Stage::Health)));
    }
    let mut r = room();
    r.player.hp -= 10.;
    r.inventory.push(potion(900.));
    events::begin();
    crate::inventory::command(&mut r, &Command::Use { id: 900. }).unwrap();
    assert!(events::finish().iter().any(|e| e
        .animation
        .as_ref()
        .is_some_and(|a| a.stage == events::Stage::Health)));
}
#[test]
fn inventory_old_revision_has_no_added_presentation_or_rng_change() {
    let mut r = room();
    r.simulation_revision = 51.0.into();
    r.player.abilities = vec![
        "isheilah-tools-of-the-trade".into(),
        "kative-bottomless-pack".into(),
    ];
    r.inventory.push(potion(900.));
    let mut plain = r.clone();
    events::begin();
    crate::inventory::command(&mut r, &Command::Use { id: 900. }).unwrap();
    let out = events::finish();
    crate::inventory::command(&mut plain, &Command::Use { id: 900. }).unwrap();
    assert_eq!(r, plain);
    assert!(out.iter().all(|e| e.animation.is_none()));
}

#[test]
fn inventory_borge_reward_matches_new_item_and_rejected_offering_is_quiet() {
    let mut r = room();
    r.next_id = 1000.;
    r.floor_mut().unwrap().godbeast_event = crate::GodbeastEvent {
        kind: "Borge".into(),
        x: 10.,
        y: 7.,
        altar: crate::GodbeastEventTap { x: 10., y: 10. }.into(),
        activated: true.into(),
        sacrifice_actor_id: 999.0.into(),
        ..Default::default()
    }
    .into();
    events::begin();
    assert!(!crate::borge::interact(&mut r, 10., 10.).unwrap());
    assert!(events::finish().is_empty());
    r.inventory.push(Item {
        id: 900.,
        kind: "heart".into(),
        source_actor_id: 999.0.into(),
        borge_depth: r.depth.into(),
        ..Default::default()
    });
    events::begin();
    assert!(crate::borge::interact(&mut r, 10., 10.).unwrap());
    let out = events::finish();
    assert_eq!(marks(&out, "E74"), vec![("diamond".into(), Some(1000.))]);
    assert!(r
        .inventory
        .iter()
        .any(|i| i.id == 1000. && crate::borge::is_shard(i)));
    assert!(!r.inventory.iter().any(|i| i.id == 900.));
    assert!(r.ground.values().all(|g| g.is_empty()));
    events::begin();
    assert!(!crate::borge::interact(&mut r, 10., 10.).unwrap());
    assert!(events::finish().is_empty());
}
#[test]
fn inventory_verdanias_actual_gift_and_tap_break_have_no_false_heal() {
    for scenario in ["success", "far", "full", "repeat"] {
        let mut r = room();
        r.next_id = 1000.;
        r.floor_mut().unwrap().godbeast_event = crate::GodbeastEvent {
            kind: "Veridian".into(),
            x: 11.,
            y: 10.,
            ..Default::default()
        }
        .into();
        r.inventory.push(Item {
            id: 900.,
            name: "Tree Tap".into(),
            kind: "tool".into(),
            effect: "tree-tap".to_string().into(),
            ..Default::default()
        });
        if scenario == "far" {
            r.player.x = 2.;
        }
        if scenario == "full" {
            for id in 910..915 {
                r.inventory.push(potion(id as f64));
            }
        }
        if scenario == "repeat" {
            r.veridian_sap_harvested = true.into();
        }
        let hp = r.player.hp;
        events::begin();
        let result = crate::inventory::command(&mut r, &Command::Use { id: 900. }).unwrap();
        let out = events::finish();
        if scenario == "success" {
            assert_eq!(result, Some(true));
            assert_eq!(
                marks(&out, "E82"),
                vec![
                    ("droplet-leaf".into(), Some(1000.)),
                    ("fracture".into(), Some(900.))
                ]
            );
            assert!(r
                .inventory
                .iter()
                .any(|i| i.id == 1000. && i.effect.as_ref().is_some_and(|e| e == "Veridian Sap")));
            assert!(!r.inventory.iter().any(|i| i.id == 900.));
            assert!(!out.iter().any(|e| e
                .animation
                .as_ref()
                .is_some_and(|a| a.stage == events::Stage::Health)));
        } else {
            assert_eq!(result, Some(false));
            assert!(out.is_empty());
            assert!(r.inventory.iter().any(|i| i.id == 900.));
        }
        assert_eq!(r.player.hp, hp);
    }
}
