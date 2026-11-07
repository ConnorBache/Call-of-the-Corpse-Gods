//! Actual generated-floor, save/load and deterministic seed replay coverage.
use corpse_core::{Command, equipment, replay::{ReplayRecord, ReplaySession}, startup, temporal::Engine};


fn has_property(item:&corpse_core::Item)->bool {
    item.granted_ability.as_ref().is_some_and(|id| !id.is_empty()
        && corpse_core::progression::catalog_for_revision(43.).iter().any(|a|a.id==*id))
        || item.bonus.as_ref().is_some_and(|b| [
            &b.offense,&b.defense,&b.accuracy,&b.dodge,&b.dr,
            &b.perception,&b.max_energy,&b.max_hp,
        ].iter().any(|n|corpse_core::combat::integer_stat(n.value_or(0.0))!=0.0))
}

#[test]
fn generated_jewelry_is_special_and_seed_replay_save_reload_remain_exact() {
    let initial=startup::create_run_revision("0-5-11:revision43-jewelry", "Unblessed", false, 43.).unwrap();
    let mut jewelry=0;
    for entries in initial.ground.values() {for entry in entries.iter() {
        if matches!(entry.item.kind.as_str(),"ring"|"amulet") {
            jewelry+=1;assert!(has_property(&entry.item));
        }
    }}
    assert!(jewelry>0,"fixture must exercise actual generated jewelry");
    let mut engine=Engine::new(initial).unwrap();
    for command in [Command::Pickup,Command::Wait] {engine.dispatch(&command).unwrap();}
    let saved=engine.save().unwrap();
    assert_eq!(Engine::load(&saved).unwrap().save().unwrap(),saved);
    let mut replay=ReplaySession::create(ReplayRecord::try_from_run(&engine.run).unwrap(),true).unwrap();
    while replay.step().unwrap(){}
    assert_eq!(replay.engine.save().unwrap(),saved);
    // Creation guarantee is not a retroactive migration of authored/admin items.
    engine.run.inventory.push(equipment::make_equipment("Ring",900000.).unwrap());
    engine.run.next_id=900001.;
    let loaded=Engine::load(&engine.save().unwrap()).unwrap();
    let item=loaded.run.inventory.iter().find(|i|i.id==900000.).unwrap();
    assert_eq!(item.name,"Ring");assert!(!has_property(item));
}
