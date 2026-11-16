//! R52 admission preserves the preceding simulation and changes only its new rules.
use corpse_core::replay::{ReplayRecord, ReplaySession};
use corpse_core::{Actor, Command, Field, Run, startup, storage, temporal::Engine};

fn generated(revision: f64) -> Run {
    startup::create_run_revision("0-5-11:r52-admission", "Avatar", false, revision).unwrap()
}

#[path = "support/balance52_birth.rs"]
mod birth;
#[path = "support/generation52_comparison.rs"]
mod generation_comparison;

fn loot_details(f: &corpse_core::Floor) -> &indexmap::IndexMap<String, f64> {
    f.generation.as_ref().unwrap().stages.iter().find(|s|s.id=="items").unwrap().details.as_ref().unwrap()
}
#[test]
fn revision52_generation_changes_only_approved_births_reserves_currents_and_island_loot() {
    for seed in ["0-5-11:r52-admission", "r52-birth-delta-second"] {
        let mut new = startup::new_run_shell_revision(seed, "Avatar", false, 52.).unwrap();
        startup::ensure_floors(&mut new, 40).unwrap();
        // New safety acceptance can legitimately reject an R51 layout. Hold
        // the chosen candidate indices fixed, independently regenerate R51
        // terrain and population, then retain every unrelated-field assertion.
        let old = generation_comparison::preceding_rules_at_selected_attempts(seed, &new);
        let ids:std::collections::HashMap<u64,f64>=(1..=40).flat_map(|d|
            old.actors_at(d as f64).iter().zip(new.actors_at(d as f64)).map(|(a,b)|(b.id as u64,a.id)))
            .collect();
        let old_id=|id:f64| *ids.get(&(id as u64)).expect("recorded actor reference must resolve");
        let mut changed=0; let mut nest_bodies=0; let mut feast_bodies=0; let mut tiered=0;
        let mut added_items=0; let mut budget_added_items=0.; let mut shared_root_sources=0; let mut island_rings=0;
        for depth in 1..=40 {
            let d=depth as f64;
            assert_eq!(old.actors_at(d).len(),new.actors_at(d).len(),"{seed} floor {depth}");
            let before=loot_details(&old.floors[depth-1]);
            let after=loot_details(&new.floors[depth-1]);
            let jackals=old.actors_at(d).iter().filter(|a|birth::natural_kind(a)=="pale:Jackal"
                && !a.dead.value_or(false) && !a.heartless.value_or(false)).count() as f64;
            assert_eq!(after["retainedHeartSlots"],before["retainedHeartSlots"]-jackals);
            assert_eq!(after["carriedItemSlots"],before["carriedItemSlots"]);
            let first=depth-(depth-1)%3;
            let prior:f64=(first..depth).map(|p|loot_details(&new.floors[p-1])["retainedSlots"]).sum();
            let target=if depth%3==0 {20.}else{7.*(depth-first+1) as f64};
            assert_eq!(after["ordinaryLoot"],(target-prior-after["retainedHeartSlots"]-after["carriedItemSlots"]).max(2.));
            let retained=after["ordinaryLoot"]+after["retainedHeartSlots"]+after["carriedItemSlots"];
            assert_eq!(after["retainedSlots"],retained,"retained slots {seed} floor {depth}");
            assert_eq!(after["retainedBlockTotal"],prior+retained,"retained block {seed} floor {depth}");
            assert_eq!(after["retainedBlockTarget"],target);
            assert_eq!(after["retainedBlockOvershoot"],(prior+retained-target).max(0.));
            let ground_changed=old.ground[&depth.to_string()].len()!=new.ground[&depth.to_string()].len();
            added_items+=new.ground[&depth.to_string()].len() as i64-old.ground[&depth.to_string()].len() as i64;
            let feast_ids: Vec<_>=old.floors[depth-1].fungal_roots.as_ref().into_iter().flatten().map(|x|x.corpse_id).collect();
            for (a,b) in old.actors_at(d).iter().zip(new.actors_at(d)) {
                let kind=birth::generated_key(a);
                if a.conditions.get("nestResident")==Some(&1.) {nest_bodies+=1;}
                if feast_ids.contains(&a.id) {feast_bodies+=1;assert!(a.dead.value_or(false)&&a.heartless.value_or(false));}
                let expected=birth::expected_birth(a,&kind,d);
                let mut actual=b.clone(); actual.id=old_id(actual.id);
                for key in ["monsterLinkedMechId","monsterLinkedInventorId","monsterUrsaPartnerId"] {
                    if let Some(id)=actual.conditions.get_mut(key) {*id=old_id(*id);}
                }
                if let Some(value)=actual.conditions.get_mut("skinWalkerDisguise") {
                    assert_eq!(*value,((b.id as u64).wrapping_mul(2_654_435_761)%6) as f64);
                    *value=a.conditions["skinWalkerDisguise"];
                }
                if let (Some(old_items),Some(new_items))=(a.monster_state.as_ref().and_then(|m|m.carried_items.as_ref()),
                    actual.monster_state.as_mut().and_then(|m|m.carried_items.as_mut())) {
                    assert_eq!(old_items.len(),new_items.len());
                    for (x,y) in old_items.iter().zip(new_items) {y.id=x.id;}
                }
                // Nightmare placement excludes ground loot. Only the floors with
                // changed loot counts may move those existing authored bodies.
                if ground_changed && b.name.contains("Nightmare") {actual.x=a.x;actual.y=a.y;}
                assert_eq!(actual,expected,"{seed} floor {depth} {}",a.name);
                if corpse_core::balance52::profile(&kind,d).is_some(){changed+=1;}
            }
            let old_ground=&old.ground[&depth.to_string()];
            let new_ground=&new.ground[&depth.to_string()];
            let rings:Vec<_>=new_ground.iter().filter(|g|corpse_core::revision50_rings::effect(&g.item)==Some("swimming")).collect();
            for g in &rings {assert_eq!(g.item,corpse_core::revision52_swimming::make_ring(g.item.id));}
            assert!(corpse_core::revision52_swimming::placement_cells(&new.floors[depth-1],new.actors_at(d),new_ground).unwrap().is_empty());
            island_rings+=rings.len() as i64;
            // Fixed early R51+ drops ignore the retained-slot ledger; later
            // drops follow the independently asserted ordinary-loot budget.
            // Accepted geometry can change Jackal counts, so a golden total
            // from the old default layouts is not a valid oracle here.
            let ordinary_added=if depth<=4 {
                for key in ["ordinaryLootGenerated","ordinaryPotionDrops","ordinaryRingDrops","ordinaryAmuletDrops"] {
                    assert_eq!(before[key],after[key],"fixed early schedule {seed} floor {depth}");
                }
                0.
            } else {after["ordinaryLoot"]-before["ordinaryLoot"]};
            assert_eq!(after["items"]-before["items"],ordinary_added,"items-stage budget {seed} floor {depth}");
            assert_eq!(new_ground.len() as f64-old_ground.len() as f64,ordinary_added+rings.len() as f64,
                "final ground budget plus guaranteed rings {seed} floor {depth}");
            budget_added_items+=ordinary_added;
            for g in new_ground.iter() {
                if corpse_core::revision50_amulet_effects::is_amulet(&g.item) {
                    assert_eq!(g.item.amulet_tier.value_or(0.),corpse_core::revision52_amulet_tiers::tier_for_depth(depth as u32) as f64); tiered+=1;
                }
            }
            if !ground_changed {
                for (a,b) in old_ground.iter().zip(new_ground.iter()) {
                    let mut expected=a.clone();let mut actual=b.clone();
                    if let Some(state)=actual.item.amulet_state.as_mut(){assert_eq!(state.instance_id,b.item.id as u64);state.instance_id=a.item.id as u64;}
                    actual.item.id=a.item.id;
                    if corpse_core::revision50_amulet_effects::is_amulet(&a.item) {
                        corpse_core::revision52_amulet_tiers::initialize_birth(&mut expected.item,52.,depth as u32).unwrap();
                        if let Some(state)=expected.item.amulet_state.as_mut(){state.energy=state.energy.min(10.);}
                    }
                    assert_eq!(actual,expected,"unchanged loot count: {seed} floor {depth}");
                }
            }
            let mut floor=(*new.floors[depth-1]).clone();
            for e in floor.encounters.as_mut().into_iter().flatten() {
                for id in &mut e.actor_ids {*id=old_id(*id);}
                if let Some(id)=e.leader_id.as_mut(){*id=old_id(*id);}
                for id in e.follower_ids.as_mut().into_iter().flatten(){*id=old_id(*id);}
            }
            for root in floor.fungal_roots.as_mut().into_iter().flatten() {root.fungus_id=old_id(root.fungus_id);root.corpse_id=old_id(root.corpse_id);}
            // Root acquisition now starts at the source. Prove the approved
            // delta independently before restoring only it for the unchanged-
            // floor oracle below; never replace the floor with current output.
            let old_floor=&old.floors[depth-1];
            let old_roots=old_floor.fungal_roots.as_ref().map(Vec::as_slice).unwrap_or(&[]);
            let new_roots=floor.fungal_roots.as_ref().map(Vec::as_slice).unwrap_or(&[]);
            assert_eq!(new_roots.len(),old_roots.len(),"root count {seed} floor {depth}");
            if !old_roots.is_empty() {
                let mut expected_prefix=vec![0.;floor.tiles.len()];
                let mut expected_full=vec![0.;floor.tiles.len()];
                let mut source_cells=std::collections::HashMap::new();
                for (before,after) in old_roots.iter().zip(new_roots) {
                    assert_eq!((after.fungus_id,after.corpse_id),(before.fungus_id,before.corpse_id),
                        "root endpoints {seed} floor {depth}");
                    assert_eq!(after.path,before.path,"full root route {seed} floor {depth}");
                    assert!(before.growth.is_missing() && before.active);
                    assert_eq!(after.growth,corpse_core::FungalRootGrowth {grown:1,turns:0}.into(),
                        "new root progress {seed} floor {depth}");
                    let source=old.actors_at(d).iter().find(|a|a.id==before.fungus_id).unwrap();
                    let source_cell=source.y as usize*floor.width()+source.x as usize;
                    assert_eq!(after.path.first(),Some(&(source_cell as f64)));
                    assert_eq!(after.grown_path(),&[source_cell as f64]);
                    assert!(!after.reaches_corpse(),"generated corpse is not yet contacted");
                    expected_prefix[source_cell]=1.;
                    for &cell in &before.path {expected_full[cell as usize]=1.;}
                    let count=source_cells.entry(source_cell).or_insert(0usize);*count+=1;
                }
                // Both initial targets share one physical source cell, rather
                // than duplicating or prematurely filling either outward path.
                assert!(source_cells.values().all(|&count|count==2));
                shared_root_sources+=source_cells.len();
                assert_eq!(floor.roots.as_ref(),Some(&expected_prefix),"initial root material {seed} floor {depth}");
                assert_eq!(expected_prefix.iter().filter(|&&v|v>0.).count(),source_cells.len());
                assert_eq!(old_floor.roots.as_ref(),Some(&expected_full),"historical root material {seed} floor {depth}");
                for (before,after) in old_roots.iter().zip(floor.fungal_roots.as_mut().unwrap()) {
                    after.growth=Field::Missing;
                    assert_eq!(&*after,before,"remaining root metadata {seed} floor {depth}");
                }
                floor.roots=expected_full.into();
            }
            // Independently reconstruct the newly specified square-current
            // layer from the final terrain and accepted attempt. Then restore
            // only that verified delta for the whole-floor compatibility oracle.
            let mut expected_currents=floor.clone();
            corpse_core::square_currents::generate(&mut expected_currents,corpse_core::rng::seed_hash(&new.seed));
            assert_eq!(floor.currents,expected_currents.currents,"square currents {seed} floor {depth}");
            floor.currents=old_floor.currents.clone();
            let cleanup=floor.generation.as_mut().unwrap().stages.iter_mut().find(|s|s.id=="cleanup").unwrap().details.as_mut().unwrap();
            assert_eq!(cleanup.shift_remove("islandSwimmingRings"),Some(rings.len() as f64));
            let stage=floor.generation.as_mut().unwrap().stages.iter_mut().find(|s|s.id=="items").unwrap();
            stage.details=before.clone().into();
            assert_eq!(floor,*old.floors[depth-1],"floor geometry and non-loot metadata {seed} {depth}");
        }
        assert!(changed>0 && feast_bodies>0 && tiered>0);assert_eq!(nest_bodies,7);
        assert!(shared_root_sources>0);
        assert_eq!(new.next_id-old.next_id,added_items as f64);
        assert_eq!(added_items as f64,budget_added_items+island_rings as f64);
        let mut expected_player=old.player.clone();expected_player.stats.accuracy=15.;expected_player.stats.dodge=15.;
        if let Some(reserves)=expected_player.ability_reserves.as_mut() {
            for reserve in reserves.values_mut(){reserve.capacity=reserve.capacity.min(10.);reserve.energy=reserve.energy.min(reserve.capacity);}
        }
        expected_player.active.retain(|id|id!="isheilah-missed-me");
        let reaction_cap=["isheilah-quick-hand","isheilah-are-you-sure-about-that"].into_iter()
            .filter(|id|expected_player.abilities.iter().any(|owned|owned==id)).count() as f64;
        if let Some(n)=expected_player.reactions.as_mut(){*n=n.min(reaction_cap);}
        assert_eq!(new.player,expected_player);
        assert!(old.firebolt_travel_from.is_missing(),"legacy generation retains its original policy");
        assert_eq!(new.firebolt_travel_from.as_ref(),Some(&0),"new R52 generation records current Firebolt from command zero");
        let mut a=serde_json::to_value(&old).unwrap();let mut b=serde_json::to_value(&new).unwrap();
        for key in ["simulationRevision","player","floors","actors","ground","nextId","fireboltTravelFrom"] {a.as_object_mut().unwrap().remove(key);b.as_object_mut().unwrap().remove(key);}
        // Existing instrumentation counts additional item placement work; it is
        // not world state or a new random stream.
        for key in ["itemPlacementCalls","itemPlacementCells","itemPlacementOccupancyEntries"] {
            a["metrics"].as_object_mut().unwrap().remove(key);b["metrics"].as_object_mut().unwrap().remove(key);
        }
        assert_eq!(a,b,"unrelated run state {seed}");
    }
}

#[test]
fn revision52_normal_save_seed_replay_and_restart_are_exact() {
    let mut e = Engine::new(generated(52.)).unwrap();
    assert_eq!((e.run.player.stats.accuracy, e.run.player.stats.dodge), (15., 15.));
    for command in [Command::Pickup, Command::Wait, Command::Wait] {
        e.dispatch(&command).unwrap();
        let saved = e.save().unwrap();
        e = Engine::load(&saved).unwrap();
        assert_eq!(e.save().unwrap(), saved);
        assert_eq!(e.run.simulation_revision, Field::Value(52.));
    }
    let expected = e.save().unwrap();
    let mut replay =
        ReplaySession::create(ReplayRecord::try_from_run(&e.run).unwrap(), true).unwrap();
    while replay.step().unwrap() {}
    assert_eq!(replay.engine.save().unwrap(), expected);
    replay.restart().unwrap();
    while replay.step().unwrap() {}
    assert_eq!(replay.engine.save().unwrap(), expected);
}

#[test]
fn revision52_debug_and_playtest_keep_their_provenance_boundaries() {
    let r = corpse_core::debug_floor::create_run_revision("0-5-11:r52-debug", "Unblessed", 1, 52.)
        .unwrap();
    corpse_core::playtest::validate_profile(&r).unwrap();
    let mut e = Engine::new(r).unwrap();
    assert!(ReplaySession::create(ReplayRecord::try_from_run(&e.run).unwrap(), true).is_err());
    corpse_core::debug_floor::enter(&mut e.run).unwrap();
    e.dispatch(&Command::Wait).unwrap();
    let saved = e.save().unwrap();
    assert_eq!(Engine::load(&saved).unwrap().save().unwrap(), saved);
    let p = corpse_core::playtest::create_run_revision(
        "0-5-11:r52-playtest",
        "Unblessed",
        false,
        1,
        corpse_core::playtest::PROFILE,
        52.,
    )
    .unwrap();
    corpse_core::playtest::validate_profile(&p).unwrap();
    assert_eq!(p.simulation_revision, Field::Value(52.));
}

#[test]
fn revision52_generic_births_use_fixed_profiles_and_corrected_raw_damage() {
    for kind in [
        "rat",
        "cave-bat",
        "hemogoblin",
        "night-crawler",
        "skin-walker",
        "tiger",
        "dragon",
        "bastidion",
    ] {
        let a = corpse_core::population::create_actor_revision(kind, 900., 10., 10., false, 51.)
            .unwrap();
        let b = corpse_core::population::create_actor_revision(kind, 900., 10., 10., false, 52.)
            .unwrap();
        let expected = birth::expected_generic_birth(&a, kind);
        assert_eq!(expected, b, "{kind}: fixed numerical birth profile");
    }
    for row in corpse_core::balance42::profiles() {
        let mut a = Actor::default();
        let mut b = a.clone();
        corpse_core::balance42::apply_new_actor_kind(&mut a, &row.kind, 51.);
        corpse_core::balance42::apply_new_actor_kind(&mut b, &row.kind, 52.);
        assert_eq!(a, b, "{}", row.kind);
    }
    for (quality, amount) in [("infant", 7.), ("child", 9.)] {
        let a = Actor {
            zillennia_quality: quality.to_string().into(),
            ..Default::default()
        };
        assert_eq!(
            corpse_core::monster_ruin::ruin_damage_revision(&a, 52.),
            amount
        );
    }
}

#[test]
fn revision52_admits_no_future_fractional_or_skipped_revision() {
    let initial = generated(52.);
    for revision in [
        47.,
        48.,
        49.,
        53.,
        52.5,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ] {
        assert!(startup::new_run_shell_revision("invalid", "Unblessed", false, revision).is_err());
        let mut r = initial.clone();
        r.simulation_revision = revision.into();
        assert!(storage::validate_current_revision(&r).is_err());
        let record = ReplayRecord::from_run(&r);
        assert!(ReplaySession::from_initial(r, record).is_err());
    }
}

#[test]
fn legacy_revision51_is_never_upgraded_on_save_load() {
    let e = Engine::new(generated(51.)).unwrap();
    let saved = e.save().unwrap();
    let loaded = Engine::load(&saved).unwrap();
    assert_eq!(loaded.run.simulation_revision, Field::Value(51.));
    assert_eq!(loaded.save().unwrap(), saved);
    assert_eq!(
        startup::new_run_shell("historical-default", "Unblessed", false)
            .unwrap()
            .simulation_revision,
        Field::Value(39.)
    );
}
