use super::*;
use crate::{Field, GroundItem, rng::GenerationRng};
use std::sync::Arc;

fn room() -> Run {
    let mut run: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    run.simulation_revision = 51.0.into();
    run.composite_bow_generated = false.into();
    run.inventory = crate::startup::starting_inventory(51.).unwrap();
    run.player.abilities.clear(); run.player.active.clear(); run.player.conditions.clear();
    run.player.equipment_abilities = Field::Missing; run.player.ability_order = Field::Missing;
    run.player.x = 10.; run.player.y = 10.; run.next_id = 9000.;
    run.actors_at_mut(run.depth).clear(); run.ground_at_mut(run.depth).clear();
    run.floor_mut().unwrap().lights.fill(100.); run.floor_mut().unwrap().seen.fill(true);
    run
}
fn loot(run: &mut Run, depth: u32, seed: u32) -> Vec<Item> {
    crate::equipment::generate_floor_loot_for_run(run, depth, &mut GenerationRng::new(seed), &[], 20).unwrap()
}
fn world_bows(run: &Run) -> usize {
    run.inventory.iter().filter(|i| is_composite(i)).count()
        + run.ground.values().flat_map(|items|items.iter()).filter(|g| is_composite(&g.item)).count()
        + crate::debug_floor::state(run).map_or(0, |s| s.stash.iter().filter(|g| is_composite(&g.item)).count())
}

#[test]
fn forty_floor_and_event_streams_share_one_budget_and_are_deterministic() {
    for seed in 0..64 {
        let mut run = crate::startup::new_run_shell_revision(&format!("0-5-11:unique-{seed}"), "Unblessed", false, 51.).unwrap();
        let mut mirror = run.clone();
        let simulation_rng = run.rng;
        let mut bows = 0;
        for depth in 1..=40 {
            let items = loot(&mut run, depth, seed * 40 + depth);
            assert_eq!(items, loot(&mut mirror, depth, seed * 40 + depth));
            if depth <= 4 {
                assert!((5..=7).contains(&items.len()));
                assert!(items.iter().all(|item| !is_composite(item)));
            } else {
                assert_eq!(items.len(), 20);
                assert_ne!(items[0].kind, items[1].kind);
            }
            bows += items.iter().filter(|i| is_composite(i)).count();
            for event in 0..3 {
                let mut rng = GenerationRng::new(seed * 120 + depth * 3 + event);
                let mut mirrored_rng = rng;
                let item = crate::equipment::make_godbeast_permanent_for_run(&mut run, depth, &mut rng).unwrap();
                assert_eq!(item, crate::equipment::make_godbeast_permanent_for_run(&mut mirror, depth, &mut mirrored_rng).unwrap());
                assert_eq!(rng.state, mirrored_rng.state);
                run.next_id += 1.; mirror.next_id += 1.;
                bows += usize::from(is_composite(&item));
            }
            assert!(bows <= 1, "seed {seed}, floor {depth}");
            // Deliberately retain no generated items: destroying every drop
            // cannot replenish the generation budget on the next floor.
        }
        assert_eq!(bows, 1); assert_eq!(run.rng, simulation_rng);
        assert_eq!(run.next_id, mirror.next_id);
        assert_eq!(run.composite_bow_generated, true.into());
    }
}

#[test]
fn pickup_drop_loss_save_and_future_keep_the_spawn_budget() {
    let mut run = room();
    let bow = (0..128).find_map(|seed| loot(&mut run, 5, seed).into_iter().find(is_composite)).unwrap();
    let id = bow.id;
    run.ground_at_mut(1.).push(GroundItem {x:10., y:10., item:bow, ..Default::default()});
    assert_eq!(crate::inventory::collect_underfoot(&mut run,20.).unwrap(),1);
    assert_eq!(crate::inventory::command(&mut run, &crate::Command::Drop {id}).unwrap(), Some(true));
    assert_eq!(world_bows(&run),1);
    run.ground_at_mut(1.).retain(|g|g.item.id != id);
    let snapshot = crate::temporal::Snapshot::capture(&run);
    let anchor = crate::future_anchor::FutureAnchor::capture41(&run).unwrap();
    let raw = crate::storage::serialize(&run).unwrap();
    let mut restored = crate::storage::deserialize_compatible(&raw).unwrap();
    assert_eq!(restored.composite_bow_generated, true.into());
    assert_eq!(world_bows(&restored),0);
    for depth in 2..=40 { assert!(loot(&mut restored, depth, depth).iter().all(|i| !is_composite(i))); }
    snapshot.restore(&mut restored).unwrap();
    assert_eq!(restored.composite_bow_generated, true.into());
    let mut future = anchor.prepare_restore().unwrap();
    assert_eq!(future.composite_bow_generated, true.into());
    assert!(loot(&mut future,2,10).iter().all(|i| !is_composite(i)));
    assert_eq!(crate::startup::starting_inventory(51.).unwrap(), crate::startup::starting_inventory(50.).unwrap());
}

#[test]
fn missing_marker_finds_all_stored_owners_without_removing_old_bows() {
    for source in 0..7 {
        let mut run = room(); run.composite_bow_generated = Field::Missing;
        let bow = crate::equipment::make_equipment("Composite bow", 100.).unwrap();
        match source {
            0 => run.inventory.push(bow),
            1 => { run.ground.insert("40".into(),Arc::new(vec![GroundItem{item:bow,..Default::default()}])); }
            2 => run.player.monster_state = crate::ActorMonsterState {carried_items:vec![bow].into(),..Default::default()}.into(),
            3 => run.player.copied_inventory = vec![bow].into(),
            4 => run.player.monster_state = crate::ActorMonsterState {pilot: Box::new(Actor {
                copied_inventory:vec![bow].into(), ..Default::default()
            }).into(),..Default::default()}.into(),
            5 => { run.remembered_items.get_or_insert_with(Default::default).insert("40".into(),
                vec![crate::RunRememberedItemsEntryEntry{item:bow,..Default::default()}]); }
            _ => run.playtest = crate::playtest::Profile {
                version:2, start_floor:6, profile:crate::debug_floor::PROFILE.into(), prior_hearts:0,
                prior_blessings:0., sources:vec![], candidates:vec![], debug:Some(crate::debug_floor::State {
                    pending:Default::default(), consumed:0, stash:vec![crate::debug_floor::Reward{depth:1,item:bow}],
                    entered:true, loadout_revision:0,
                }),
            }.into(),
        }
        let mut expected = run.clone(); expected.composite_bow_generated = true.into();
        initialize(&mut run);
        assert_eq!(run,expected,"source {source}");
        assert!(loot(&mut run,2,25).iter().all(|i| !is_composite(i)));
    }
    let mut old = room(); old.simulation_revision = 50.0.into(); old.composite_bow_generated = Field::Missing;
    for id in [101.,102.] { old.inventory.push(crate::equipment::make_equipment("Composite bow",id).unwrap()); }
    let before = old.clone(); initialize(&mut old); assert_eq!(old,before);
}

#[test]
fn legacy_loot_rewards_ids_and_rng_remain_exact() {
    for revision in [29.,43.,46.,50.] {
        for seed in 0..32 {
            let mut run = room(); run.simulation_revision = revision.into(); run.composite_bow_generated = Field::Missing;
            let mut rng = GenerationRng::new(seed); let mut old_rng = rng; let mut next_id = run.next_id;
            let old = crate::equipment::generate_floor_loot_revision(12,&mut old_rng,&mut next_id,&[],20,revision).unwrap();
            assert_eq!(old,crate::equipment::generate_floor_loot_for_run(&mut run,12,&mut rng,&[],20).unwrap());
            assert_eq!(rng.state,old_rng.state); assert_eq!(run.next_id,next_id);
            let old_reward = crate::equipment::make_godbeast_permanent_revision(12,next_id,&mut old_rng,revision).unwrap();
            assert_eq!(old_reward,crate::equipment::make_godbeast_permanent_for_run(&mut run,12,&mut rng).unwrap());
            assert_eq!(rng.state,old_rng.state); assert!(run.composite_bow_generated.is_missing());
        }
    }
}

#[test]
fn mercuria_cannot_make_another_bow_after_the_existing_bow_is_gone() {
    let mut run = room(); run.composite_bow_generated = true.into();
    run.floor_mut().unwrap().godbeast_event = crate::GodbeastEvent {
        kind:"Mercuria".into(),x:15.,y:14.,lever:crate::GodbeastEventLever{x:14.,y:14.}.into(),..Default::default()
    }.into();
    let old_rng = run.rng;
    assert!(crate::godbeast_interactions::pull_lever(&mut run,14.,14.).unwrap());
    assert_eq!(world_bows(&run),0); assert_eq!(run.ground.get("1").unwrap().len(),1);
    assert_eq!(run.rng,old_rng);
}

#[test]
fn normal_forty_floor_dungeon_has_one_bow_including_sphinx_prizes() {
    let mut run = crate::startup::new_run_shell_revision("0-5-11:r50-deep-public","Unblessed",false,51.).unwrap();
    crate::startup::ensure_floors(&mut run,10).unwrap();
    assert_eq!(world_bows(&run),1);
    let first = run.ground.values().flat_map(|items|items.iter()).find(|g|is_composite(&g.item)).unwrap().item.id;
    crate::startup::ensure_floors(&mut run,40).unwrap();
    assert_eq!(world_bows(&run),1);
    assert_eq!(run.ground.values().flat_map(|items|items.iter()).find(|g|is_composite(&g.item)).unwrap().item.id,first);
    assert!(run.floors.iter().any(|floor|floor.sphinx_room.as_ref().is_some()),"exercise actual Sphinx prizes");
    assert_eq!(run.composite_bow_generated,true.into());
}

#[test]
fn debug_and_synthetic_deep_starts_share_the_dungeon_budget() {
    let debug = crate::debug_floor::create_run_revision("0-5-11:r50-deep-public","Unblessed",6,51.).unwrap();
    assert_eq!(world_bows(&debug),1); assert_eq!(debug.composite_bow_generated,true.into());
    let synthetic = crate::playtest::create_run_revision("0-5-11:r50-deep-public","Unblessed",false,6,crate::playtest::PROFILE,51.).unwrap();
    assert_eq!(world_bows(&synthetic),1); assert_eq!(synthetic.composite_bow_generated,true.into());
}
