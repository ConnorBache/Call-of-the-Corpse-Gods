//! Independent DEBUG/temporal integration regressions.
use corpse_core::{Command, Run, debug_floor as debug, inventory, progression, temporal::Engine};
fn prepared() -> Run {
    let mut r = debug::create_run("0-5-11:independent-debug-temporal", "Unblessed", 4).unwrap();
    while !r.offers.is_empty() {
        let id = r.offers[0].clone();
        progression::choose_offer(&mut r, &id).unwrap(); debug::advance(&mut r).unwrap();
    }
    assert!(debug::choices_complete(&r));
    for id in ["numo-revert", "morae-see-the-future"] {
        r.player.abilities.push(id.into()); progression::initialize_ability_state(&mut r.player,id,r.tick).unwrap();
    }
    r.player.conditions.insert("revertCharges".into(), 3.0);
    r
}
fn ability(id: &str) -> Command { Command::Ability { id:id.into(), target:Default::default(), x:Default::default(), y:Default::default() } }
fn occurrences(r: &Run, id: f64) -> usize {
    r.inventory.iter().filter(|i| i.id == id).count()
        + debug::state(r).unwrap().stash.iter().filter(|v| v.item.id == id).count()
        + r.ground.values().flat_map(|v|v.iter()).filter(|g|g.item.id == id).count()
}
#[test]
fn live_stash_take_after_drop_then_revert_keeps_every_source_item_exactly_once() {
    let mut r = prepared();
    let ids:Vec<_> = debug::state(&r).unwrap().stash.iter().filter(|v| !inventory::is_arrow(&v.item)).take(2).map(|v|v.item.id).collect();
    assert_eq!(ids.len(),2);
    debug::take(&mut r,ids[0]).unwrap(); debug::enter(&mut r).unwrap();
    let mut e = Engine::new(r).unwrap();
    e.dispatch(&Command::Drop { id:ids[0] }).unwrap(); assert_eq!(e.run.tick,1.0);
    debug::take(&mut e.run, ids[1]).unwrap();
    assert_eq!(occurrences(&e.run,ids[0]),1);assert_eq!(occurrences(&e.run,ids[1]),1);
    e.dispatch(&ability("numo-revert")).unwrap();
    assert_eq!(e.run.tick,0.0);assert!(!debug::preparing(&e.run));
    assert_eq!(occurrences(&e.run,ids[0]),1);
    assert_eq!(occurrences(&e.run,ids[1]),1,"Revert must not lose or duplicate a directly moved DEBUG stash item");
}

#[test]
fn live_stash_put_after_drop_then_revert_cannot_duplicate_reconstructed_item() {
    let mut r = prepared();
    let ids:Vec<_> = debug::state(&r).unwrap().stash.iter().filter(|v| !inventory::is_arrow(&v.item)).take(2).map(|v|v.item.id).collect();
    for &id in &ids { debug::take(&mut r,id).unwrap(); } debug::enter(&mut r).unwrap();
    let mut e = Engine::new(r).unwrap();
    e.dispatch(&Command::Drop { id:ids[0] }).unwrap();
    debug::put(&mut e.run,ids[1]).unwrap();
    e.dispatch(&ability("numo-revert")).unwrap();
    assert_eq!(occurrences(&e.run,ids[0]),1);
    assert_eq!(occurrences(&e.run,ids[1]),1,"Revert must not duplicate an item returned to the live DEBUG stash");
}
#[test]
fn live_stash_then_explicit_save_load_revert_preserves_source_item_uniqueness() {
    let mut r = prepared();
    let ids:Vec<_> = debug::state(&r).unwrap().stash.iter().filter(|v| !inventory::is_arrow(&v.item)).take(2).map(|v|v.item.id).collect();
    debug::take(&mut r,ids[0]).unwrap(); debug::enter(&mut r).unwrap();
    let mut e = Engine::new(r).unwrap();e.dispatch(&Command::Drop { id:ids[0] }).unwrap();
    debug::take(&mut e.run,ids[1]).unwrap();
    let raw=e.save().unwrap();let mut loaded=Engine::load(&raw).unwrap();
    loaded.dispatch(&ability("numo-revert")).unwrap();
    assert!(!debug::preparing(&loaded.run));
    assert_eq!(occurrences(&loaded.run,ids[0]),1);assert_eq!(occurrences(&loaded.run,ids[1]),1);
}
#[test]
fn future_restore_returns_pack_and_stash_to_one_consistent_branch() {
    let mut r=prepared();debug::enter(&mut r).unwrap();
    let id=debug::state(&r).unwrap().stash.iter().find(|v|!inventory::is_arrow(&v.item)).unwrap().item.id;
    let mut e=Engine::new(r).unwrap();e.dispatch(&ability("morae-see-the-future")).unwrap();
    assert!(e.run.future.is_some());debug::take(&mut e.run,id).unwrap();e.dispatch(&Command::Wait).unwrap();
    e.dispatch(&ability("morae-see-the-future")).unwrap();
    assert_eq!(occurrences(&e.run,id),1);assert!(debug::state(&e.run).unwrap().stash.iter().any(|v|v.item.id==id));assert!(!debug::preparing(&e.run));
}

#[test]
fn reconstructed_loot_leaves_tree_taps_but_keeps_initial_honey_and_no_unearned_shard() {
    let r = debug::create_run("0-5-11:independent-debug-loot", "Unblessed", 40).unwrap();
    let stash = &debug::state(&r).unwrap().stash;
    assert!(stash.iter().any(|v| corpse_core::royal_honey::is_honey(&v.item)), "initial Nest Honey is recoverable loose food");
    assert!(!stash.iter().any(|v|v.item.effect.as_ref().is_some_and(|s|s=="tree-tap")), "Tree tap is a world fixture, not automatically recovered ordinary loot");
    assert!(!stash.iter().any(|v|corpse_core::borge::is_shard(&v.item)), "No unearned Borge activation reward");
    assert!(r.ground.iter().filter(|(k,_)|k.as_str()!="40").flat_map(|(_,v)|v.iter()).any(|g|g.item.effect.as_ref().is_some_and(|s|s=="tree-tap")), "prior-floor Tree tap remains in its world");
}

#[test]
fn floor_forty_blessed_all_gods_use_live_offer_queues_then_explicit_exhaustion() {
    for god in 0..12 {
        let seed=format!("{god}-{}-{}:independent-debug-offers",(god+1)%12,(god+2)%12);
        let mut r=debug::create_run(&seed,"Blessed",40).unwrap();
        let mut choices=0;let mut exhausted=0;
        while !r.offers.is_empty() {
            let offers=r.offers.clone();
            for id in &offers {
                if !id.starts_with("eclipsed") {
                    assert!(!r.player.abilities.contains(id));
                    assert!(progression::catalog().iter().any(|a|a.id==*id&&a.god_id==god as f64));
                }
            }
            let id=offers[0].clone();if id.starts_with("eclipsed") {exhausted+=1;}
            assert!(progression::choose_offer(&mut r,&id).unwrap());debug::advance(&mut r).unwrap();choices+=1;
            assert!(choices<20);assert_eq!(r.tick,0.0);
        }
        assert!(debug::choices_complete(&r));assert_eq!(choices as f64,r.level-1.0);
        assert!(exhausted>0,"Blessed god{god} latefloor should exercise real exhausted pool, never invented powers");
        assert_eq!(debug::state(&r).unwrap().consumed,r.playtest.as_ref().unwrap().prior_hearts);
        assert!(debug::state(&r).unwrap().pending.is_empty());
    }
}

#[test]
fn live_stash_equip_after_world_turn_then_revert_is_a_consistent_transaction() {
    let mut r=prepared();
    let equip_id=debug::state(&r).unwrap().stash.iter().find(|v|matches!(v.item.kind.as_str(),"weapon"|"armor")).unwrap().item.id;
    debug::enter(&mut r).unwrap();let mut e=Engine::new(r).unwrap();e.dispatch(&Command::Wait).unwrap();
    assert!(debug::equip(&mut e.run,equip_id).unwrap());
    assert!(e.run.inventory.iter().find(|i|i.id==equip_id).unwrap().equipped.as_ref().is_some());
    e.dispatch(&ability("numo-revert")).unwrap();
    assert_eq!(occurrences(&e.run,equip_id),1);assert!(!debug::preparing(&e.run));
}
#[test]
fn pending_haste_stash_edits_and_saved_resume_revert_both_restore_one_branch() {
    for resume in [false,true] {
        let mut r=prepared();debug::enter(&mut r).unwrap();let mut e=Engine::new(r).unwrap();e.dispatch(&Command::Wait).unwrap();
        e.run.player.conditions.insert("hasted".into(),5.0);
        let id=debug::state(&e.run).unwrap().stash.iter().find(|v|!inventory::is_arrow(&v.item)).unwrap().item.id;
        let tick=e.run.tick;
        let moved=corpse_core::movement::DIRECTIONS.iter().find_map(|d| {
            let mut candidate=e.clone();let out=candidate.dispatch(&Command::Move { dx:d.x as f64,dy:d.y as f64 }).ok()?;
            (!out.took_turn&&candidate.run.tick==tick&&candidate.history.pending_haste.is_some()).then_some(candidate)
        }).expect("generated stair arrival has a legal haste movement");
        e=moved;debug::take(&mut e.run,id).unwrap();
        if resume {e=Engine::load(&e.save().unwrap()).unwrap();}
        e.dispatch(&ability("numo-revert")).unwrap();
        assert_eq!(occurrences(&e.run,id),1,"resume={resume}");assert!(!debug::preparing(&e.run));
        assert!(debug::state(&e.run).unwrap().pending.is_empty());
    }
}
