use corpse_core::{generation, startup, storage};

#[test]
fn known_tap_stair_seed_generates_all_required_content_without_changing_budget() {
    for input in ["r37-event-count-0047", "生成-é-🌱-35", "heart-audit-000071", "r37-event-count-0078", "93", "生成-é-🌱-113", "0-1-2:qa-120", "11-10-9:qa-127"] {
    let mut run = startup::new_run_shell_revision(input, "Unblessed", false, 38.).unwrap();
    startup::ensure_floors(&mut run, 40).unwrap();
    assert_eq!(run.floors.len(),40);
    let seed=corpse_core::rng::seed_hash(&run.seed);
    let plan=generation::domain_plan_revision(seed,38.);
    assert_eq!(run.floors.iter().filter(|f|f.godbeast).count(),7);
    assert_eq!(run.floors.iter().flat_map(|f|f.pocket_biomes.as_ref().into_iter().flatten()).filter(|b|b.theme=="saidribee-nest").count(),1);
    for floor in &run.floors {
        let depth=floor.depth as u32;
        assert!(generation::transitions_accessible(floor,&generation::reachable_without_braziers(floor,None)),"{input} floor {depth}");
        assert_eq!(floor.enemy_blessing_budget.value_or(0.), generation::ordinary_blessing_budget(generation::BudgetSeed::Number(seed),depth).unwrap() as f64);
        let records=floor.encounters.as_ref().unwrap();
        let ordinary_ids:std::collections::HashSet<_>=records.iter().flat_map(|r|r.actor_ids.iter().map(|n|*n as u64)).collect();
        let rewards:f64=run.actors_at(floor.depth).iter().filter(|a|ordinary_ids.contains(&(a.id as u64)))
            .map(|a|if a.heartless.value_or(false) {0.} else {corpse_core::inventory::heart_for(a,0.).unwrap().exp.value_or(0.)}).sum();
        assert_eq!(rewards,floor.enemy_blessing_budget.value_or(0.),"{input} floor {depth}");
        assert_eq!(run.actors_at(floor.depth).iter().filter(|a|corpse_core::dreamwalking::is_nightmare(a)).count(),4);
        if let Some(event)=floor.godbeast_event.as_ref() {
            assert_eq!(event.kind,plan[(depth as usize-1)/5].godbeast_kind);
            if event.kind=="Veridian" {
                let tap=event.tap.as_ref().unwrap(); let at=tap.y as usize*floor.width()+tap.x as usize;
                assert!(["floor","grass"].contains(&floor.tiles[at].as_str()));
                assert_eq!(run.ground[&depth.to_string()].iter().filter(|g|g.item.name=="Tree tap"&&g.x==tap.x&&g.y==tap.y).count(),1);
            }
            if event.kind=="Phantasmii" {
                assert_eq!(run.ground[&depth.to_string()].iter().filter(|g|corpse_core::phantasmii_generation::is_dagger(&g.item)).count(),1);
                assert_eq!(run.ground[&depth.to_string()].iter().filter(|g|corpse_core::phantasmii_generation::is_shard(&g.item)).count(),3);
            }
        }
    }
    assert_eq!(run.actors_at(40.).iter().filter(|a|corpse_core::finale::is_boss(a)).count(),1);
    let encoded=storage::serialize(&run).unwrap(); assert_eq!(storage::serialize(&storage::deserialize(&encoded).unwrap()).unwrap(),encoded);
    }
}

#[test]
fn historical_failed_seed_is_not_silently_repaired() {
    let mut run=startup::new_run_shell_revision("r37-event-count-0047","Unblessed",false,37.).unwrap();
    let before=serde_json::to_value(&run).unwrap();
    assert_eq!(startup::ensure_floors(&mut run,20).unwrap_err(),"Required generated item invalid/occupied on floor20");
    assert_eq!(serde_json::to_value(run).unwrap(),before);
}

#[test]
fn generation_batching_does_not_change_result_or_recovery_metadata() {
    let mut all=startup::new_run_shell_revision("0-1-2:recovery-batching-r38","Unblessed",false,38.).unwrap();
    let mut incremental=all.clone();
    startup::ensure_floors(&mut all,40).unwrap();
    for depth in 1..=40 {startup::ensure_floors(&mut incremental,depth).unwrap();}
    assert_eq!(serde_json::to_value(all).unwrap(),serde_json::to_value(incremental).unwrap());
}
