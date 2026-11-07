//! Independent narrow revision32 regressions. These establish mechanics, not full-game balance.
use corpse_core::{ai, engine, population, startup, storage, temporal, Command, Field, Floor, Run};
use std::sync::Arc;

fn room(kind: &str, revision: f64) -> Run {
    let mut r = startup::new_run_shell_revision("0-5-11:qa-monster32", "Unblessed", false, revision).unwrap();
    r.player.x = 2.; r.player.y = 2.;
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.ability_order = Field::Missing;
    r.player.hp = 10000.; r.player.max_hp = 10000.; r.player.base_max_hp = 10000.0.into();
    let w = 79; let h = 29;
    let mut tiles = vec!["wall".to_string(); w*h];
    for y in 1..h-1 { for x in 1..w-1 { tiles[y*w+x] = "floor".into(); } }
    let mut f = Floor { width: (w as f64).into(), height: (h as f64).into(), depth: 1., domain: "Dungeon".into(), tiles,
        lights: vec![100.;w*h], seen: vec![true;w*h], ..Default::default() };
    corpse_core::environment::initialize(&mut f).unwrap();
    r.floors = vec![Arc::new(f)];
    let mut a = population::create_actor_revision(kind,4.,5.,5.,false,revision).unwrap();
    a.state = "hunting".into(); a.last_seen = corpse_core::ActorLastSeen {x:2.,y:2.}.into();
    r.actors.clear(); r.actors.insert("1".into(),Arc::new(vec![a]));
    r.ground.clear(); r.ground.insert("1".into(),Arc::new(vec![]));
    r.next_id = 5.; r.tick = 3.; r.generated_ability_grants=Vec::new().into();
    r
}
fn cast(r: &mut Run) -> bool { ai::enemy_action(r,4.,r.player.id).unwrap() }
fn ready(r: &Run) -> f64 { engine::condition(engine::actor(r,4.).unwrap(),"monsterSummonReadyAt") }
fn children(r: &Run) -> usize { r.actors_at(1.).iter().filter(|a|a.born_from_birther.value_or(false)).count() }

#[test]
fn qa_three_casts_require_twenty_world_turns_and_use_finite_resource_budget() {
    for kind in ["pale:Birther", "pale:Greater Birther"] {
        let mut r = room(kind,32.);
        assert_eq!(ready(&r),0.);
        assert!(cast(&mut r)); assert_eq!(ready(&r),13.); assert_eq!(children(&r),2);
        let after = engine::actor(&r,4.).unwrap().energy;
        assert_eq!(after,20.,"first twin cast costs30 of50");
        for t in [3.,4.,12.] { r.tick=t; let rng=r.rng; assert!(!cast(&mut r)); assert_eq!(children(&r),2); assert_eq!(r.rng,rng); assert_eq!(engine::actor(&r,4.).unwrap().energy,after); }
        r.tick=13.; assert!(cast(&mut r)); assert_eq!(ready(&r),23.); assert_eq!(children(&r),3);
        r.tick=22.; assert!(!cast(&mut r));
        r.tick=23.; assert!(cast(&mut r)); assert_eq!(ready(&r),33.); assert_eq!(children(&r),4);
        assert_eq!(engine::actor(&r,4.).unwrap().energy,0.);
        r.tick=33.; assert!(!cast(&mut r)); assert_eq!(ready(&r),33.); assert_eq!(children(&r),4);
    }
}

#[test]
fn qa_failed_birth_does_not_spend_energy_rng_or_start_timer() {
    for failure in ["space","energy","silenced"] {
        let mut r=room("pale:Birther",32.);
        if failure=="space" {
            // Water has clear sight but is not an admissible Pale birth destination.
            for y in 4..=6 { for x in 4..=6 { if (x,y)!=(5,5) {r.floor_mut().unwrap().tiles[y*79+x]="water".into();} } }
        } else if failure=="energy" {engine::actor_mut(&mut r,4.).unwrap().energy=9.;}
        else {engine::actor_mut(&mut r,4.).unwrap().conditions.insert("silenced".into(),2.);}
        let energy=engine::actor(&r,4.).unwrap().energy; let rng=r.rng;
        assert!(!cast(&mut r),"{failure}"); assert_eq!(ready(&r),0.); assert_eq!(children(&r),0);
        assert_eq!(engine::actor(&r,4.).unwrap().energy,energy); assert_eq!(r.rng,rng);
    }
}

#[test]
fn qa_legacy_birth_is_not_retroactively_cooled_down() {
    let mut r=room("pale:Birther",31.);
    assert!(cast(&mut r)); assert!(cast(&mut r)); assert_eq!(r.tick,3.);
    assert_eq!(children(&r),3); assert_eq!(ready(&r),0.);
    // Normal gameplay still respects the authored Slow phase, even without a birth timer.
    let mut r=room("pale:Birther",31.);r.tick=2.;r.player.stats.dr=1000.;
    let mut casts=vec![];let mut energy=50.;
    for _ in 0..5 {
        engine::dispatch(&mut r,&Command::Wait).unwrap();
        let next=engine::actor(&r,4.).unwrap().energy;
        if next<energy {casts.push(r.tick);energy=next;}
    }
    assert_eq!(casts,vec![3.,5.,7.]);assert_eq!(energy,0.);assert_eq!(children(&r),4);
}

#[test]
fn qa_summon_timer_survives_storage_snapshot_and_future_restore() {
    let mut r=room("pale:Birther",32.); assert!(cast(&mut r));
    engine::validate_supported(&r).unwrap();
    let raw=storage::serialize(&r).unwrap();
    let mut r=storage::deserialize_compatible(&raw).unwrap();
    assert_eq!(ready(&r),13.); assert!(!cast(&mut r));
    let snapshot=temporal::Snapshot::capture(&r);
    r.tick=13.; assert!(cast(&mut r)); assert_eq!(ready(&r),23.);
    snapshot.restore(&mut r).unwrap(); assert_eq!(r.tick,3.); assert_eq!(ready(&r),13.); assert_eq!(children(&r),2);
    let mut history=temporal::History::import(&r).unwrap();
    temporal::remember_future(&mut r,&history).unwrap();
    r.tick=13.; assert!(cast(&mut r));
    assert!(temporal::restore_future(&mut r,&mut history).unwrap());
    assert_eq!(r.tick,3.); assert_eq!(ready(&r),13.); assert_eq!(children(&r),2);
}

#[test]
fn qa_actual_revert_and_see_future_commands_restore_summons_and_timer() {
    let mut r=room("pale:Birther",32.); r.tick=2.; r.player.stats.dr=1000.;
    r.player.abilities=vec!["numo-revert".into(),"morae-see-the-future".into()];
    r.player.conditions.insert("revertCharges".into(),10.);
    r.player.energy=100.; r.player.stats.max_energy=100.;
    let mut e=temporal::Engine::new(r).unwrap();
    e.dispatch(&Command::Wait).unwrap(); assert_eq!(e.run.tick,3.); assert_eq!(ready(&e.run),13.); assert_eq!(children(&e.run),2);
    e.dispatch(&Command::Ability{id:"numo-revert".into(),target:Field::Missing,x:Field::Missing,y:Field::Missing}).unwrap();
    assert_eq!(e.run.tick,2.); assert_eq!(ready(&e.run),0.); assert_eq!(children(&e.run),0);
    e.dispatch(&Command::Wait).unwrap(); assert_eq!(ready(&e.run),13.);
    let future=Command::Ability{id:"morae-see-the-future".into(),target:Field::Missing,x:Field::Missing,y:Field::Missing};
    e.dispatch(&future).unwrap();
    for _ in 0..10 {e.dispatch(&Command::Wait).unwrap();}
    assert!(ready(&e.run)>=23.);
    e.dispatch(&future).unwrap();
    assert_eq!(e.run.tick,3.); assert_eq!(ready(&e.run),13.); assert_eq!(children(&e.run),2);
}

#[test]
fn qa_actual_dispatch_haste_save_resume_replay_share_world_turn_timer() {
    let mut initial=room("pale:Birther",32.);
    initial.player.conditions.insert("hasted".into(),40.);
    initial.player.stats.dr=1000.;
    let mut e=temporal::Engine::new(initial.clone()).unwrap();
    let mut first=None; let mut changes=vec![]; let mut prior=0.;
    for i in 0..52 {
        e.dispatch(&Command::Wait).unwrap();
        let now=ready(&e.run);
        if now!=prior {changes.push((e.run.tick,now)); first.get_or_insert(e.run.tick); prior=now;}
        if i==8 {e=temporal::Engine::load(&e.save().unwrap()).unwrap();}
    }
    assert!(first.is_some(),"actual AI must summon");
    assert!(changes.len()>=2,"actual AI must recast after cooldown");
    for pair in changes.windows(2) {assert!(pair[1].0-pair[0].0>=10.,"haste shortened clock: {changes:?}");}
    assert!(!e.run.player.conditions.contains_key("monsterSummonReadyAt"));
    let record=corpse_core::replay::ReplayRecord::from_run(&e.run);
    let mut replay=corpse_core::replay::ReplaySession::from_initial(initial,record).unwrap();
    while replay.step().unwrap() {}
    let resumed=serde_json::to_value(e.canonical_run().unwrap()).unwrap();
    let replayed=serde_json::to_value(replay.engine.canonical_run().unwrap()).unwrap();
    if !storage::canonical_equal(&resumed,&replayed) {
        std::fs::write("/tmp/qa32-resumed.json",serde_json::to_vec(&resumed).unwrap()).unwrap();
        std::fs::write("/tmp/qa32-replayed.json",serde_json::to_vec(&replayed).unwrap()).unwrap();
    }
    assert!(storage::canonical_equal(&resumed,&replayed));
}

#[test]
fn qa_summoned_hearts_remain_existing_reward_risk_not_silently_removed() {
    let mut r=room("pale:Birther",32.); assert!(cast(&mut r));
    let child=r.actors_at(1.).iter().find(|a|a.born_from_birther.value_or(false)).unwrap().clone();
    let heart=corpse_core::inventory::heart_for(&child,99.).unwrap();
    assert_eq!(heart.exp.value_or(0.),0.); assert_eq!(heart.energy.value_or(0.),5.);
    engine::actor_mut(&mut r,child.id).unwrap().hp=0.;
    engine::death_if_nonpositive(&mut r,child.id).unwrap();
    assert!(engine::actor(&r,child.id).is_none());
    assert_eq!(r.ground.get("1").unwrap().iter().filter(|g|g.item.kind=="heart").count(),1);
    // Timer is a rate cap, not a lifetime cast cap: energy restoration can reopen birth.
    r.tick=13.; engine::actor_mut(&mut r,4.).unwrap().energy=50.;
    assert!(cast(&mut r)); assert_eq!(ready(&r),23.);
}

#[test]
fn qa_player_mini_me_toggle_does_not_gain_monster_birth_cooldown() {
    let mut r=room("pale:Birther",32.); r.actors_at_mut(1.).clear();
    r.initial_player=r.player.clone().into();
    r.player.abilities.push("kative-mini-me".into()); r.player.energy=100.; r.player.stats.max_energy=100.;
    let c=Command::Ability{id:"kative-mini-me".into(),target:Field::Missing,x:Field::Missing,y:Field::Missing};
    engine::dispatch(&mut r,&c).unwrap();
    assert!(r.actors_at(1.).iter().any(|a|a.name=="Mini-Me"));
    engine::dispatch(&mut r,&c).unwrap();
    assert!(!r.actors_at(1.).iter().any(|a|a.name=="Mini-Me"));
    engine::dispatch(&mut r,&c).unwrap();
    assert!(r.actors_at(1.).iter().any(|a|a.name=="Mini-Me"));
    assert!(!r.player.conditions.contains_key("monsterSummonReadyAt"));
    assert!(r.tick<13.,"player summon reactivation was not forced to wait10worldturns");
}

#[test]
fn qa_every_catalog_heart_has_exact_additive_growth_and_six_hp_per_defense() {
    let mut checked=0;
    for kind in population::catalog().actors.keys() {
        let a=population::create_actor_revision(kind,4.,3.,3.,false,32.).unwrap();
        let Ok(mut item)=corpse_core::inventory::heart_for(&a,99.) else {continue;};
        let mut r=room("pale:Birther",32.);
        r.actors_at_mut(1.).clear(); r.player.hp=100.; r.player.max_hp=100.; r.player.base_max_hp=100.0.into();
        let before=r.player.stats.clone(); let bonus=item.bonus.as_ref().cloned().unwrap_or_default();
        let id=item.id; item.exp=0.0.into(); // isolate heart arithmetic from optional level grants
        r.inventory.push(item);
        assert_eq!(corpse_core::inventory::command(&mut r,&Command::Use{id}).unwrap(),Some(true),"{kind}");
        let pairs=[(r.player.stats.offense,before.offense+bonus.offense.value_or(0.)),
            (r.player.stats.defense,before.defense+bonus.defense.value_or(0.)),
            (r.player.stats.accuracy,before.accuracy+bonus.accuracy.value_or(0.)),
            (r.player.stats.dodge,before.dodge+bonus.dodge.value_or(0.)),
            (r.player.stats.dr,before.dr+bonus.dr.value_or(0.)),
            (r.player.stats.perception,before.perception+bonus.perception.value_or(0.)),
            (r.player.stats.max_energy,before.max_energy+bonus.max_energy.value_or(0.))];
        for (actual,expected) in pairs {assert_eq!(actual,expected,"{kind}");}
        let hp=100.+6.*corpse_core::combat::integer_stat(r.player.stats.defense)+corpse_core::combat::integer_stat(r.player.stats.max_hp.value_or(0.));
        assert_eq!(r.player.max_hp,hp,"{kind}"); checked+=1;
    }
    assert!(checked>=70,"broad catalog coverage expected: {checked}");
}

#[test]
fn qa_heartless_parents_and_revived_parents_birth_heartless_children_only_in32() {
    for revision in [31.,32.] { for kind in ["pale:Birther","pale:Greater Birther"] { for revived in [false,true] {
        let mut r=room(kind,revision);
        let a=engine::actor_mut(&mut r,4.).unwrap(); a.heartless=true.into();
        if revived {
            a.dead=true.into(); a.hp=0.; corpse_core::corpses::revive_corpse(a,3.);
            assert!(a.heartless.value_or(false)); a.state="hunting".into();
        }
        assert!(cast(&mut r));
        let ids:Vec<_>=r.actors_at(1.).iter().filter(|a|a.born_from_birther.value_or(false)).map(|a|a.id).collect();
        assert_eq!(ids.len(),2);
        for id in ids {
            let a=engine::actor_mut(&mut r,id).unwrap(); assert_eq!(a.heartless.value_or(false),revision>=32.);
            a.hp=0.; engine::death_if_nonpositive(&mut r,id).unwrap();
        }
        assert_eq!(r.ground.get("1").unwrap().iter().filter(|g|g.item.kind=="heart").count(),if revision>=32.{0}else{2});
    } } }
}

#[test]
fn qa_greater_birther_descendant_inherits_heartlessness_and_own_timer() {
    let mut found=false;
    for seed in 1..=100 {
        let mut r=room("pale:Greater Birther",32.); r.rng=seed as f64;
        engine::actor_mut(&mut r,4.).unwrap().heartless=true.into();
        assert!(cast(&mut r));
        let Some(id)=r.actors_at(1.).iter().find(|a|a.id!=4.&&a.abilities.iter().any(|id|id=="enemy:pale:birth")).map(|a|a.id)else{continue};
        let child=engine::actor_mut(&mut r,id).unwrap(); assert!(child.heartless.value_or(false)); child.state="hunting".into();
        assert_eq!(engine::condition(child,"monsterSummonReadyAt"),0.);
        let next=r.next_id; let player=r.player.id;
        assert!(ai::enemy_action(&mut r,id,player).unwrap());
        assert_eq!(engine::condition(engine::actor(&r,id).unwrap(),"monsterSummonReadyAt"),13.);
        assert!(!ai::enemy_action(&mut r,id,player).unwrap());
        assert!(r.actors_at(1.).iter().filter(|a|a.id>=next).all(|a|a.heartless.value_or(false)
            && !a.abilities.iter().any(|id|id=="enemy:pale:birth")),"basic Birther grandchildren are Jackals, not an unbounded free birth chain");
        found=true;break;
    }
    assert!(found,"fixed seeds must exercise a real randomly selected child Birther");
}

#[test]
fn qa_unseen_sleep_refills_energy_but_active_detection_does_not() {
    let mut r=room("pale:Birther",32.);
    for cycle in 0..3 {
        engine::actor_mut(&mut r,4.).unwrap().state="hunting".into();
        for _ in 0..3 {
            assert!(cast(&mut r)); r.tick+=10.;
            let ids:Vec<_>=r.actors_at(1.).iter().filter(|a|a.born_from_birther.value_or(false)).map(|a|a.id).collect();
            for id in ids {engine::actor_mut(&mut r,id).unwrap().hp=0.;engine::death_if_nonpositive(&mut r,id).unwrap();}
        }
        assert_eq!(engine::actor(&r,4.).unwrap().energy,0.); assert!(!cast(&mut r));
        let mut a=engine::actor(&r,4.).unwrap().clone();
        for _ in 0..80 {assert!(!ai::advance_rest(&r,&mut a,true));}
        assert_eq!(a.energy,0.,"detected exhausted summoner cannot refill during combat");
        for _ in 0..19 {assert!(!ai::advance_rest(&r,&mut a,false));assert_eq!(a.energy,0.);}
        assert!(ai::advance_rest(&r,&mut a,false));assert_eq!(a.energy,1.);
        for _ in 0..49 {assert!(ai::advance_rest(&r,&mut a,false));}
        assert_eq!(a.energy,50.); assert_eq!(a.state,"wandering");
        *engine::actor_mut(&mut r,4.).unwrap()=a;r.tick+=69.;
        assert_eq!(r.ground.get("1").unwrap().iter().filter(|g|g.item.kind=="heart").count(),4*(cycle+1));
    }
    // This intentionally exposes existing live-parent refill potential, not a claim of a lifetime cap.
}

#[test]
fn qa_offscreen_catch_up_preserves_absolute_world_turn_deadline() {
    let mut r=room("pale:Birther",32.);assert!(cast(&mut r));
    r.actors_at_mut(1.).retain(|a|a.id==4.);
    engine::actor_mut(&mut r,4.).unwrap().state="neutral".into();
    let mut second=(*r.floors[0]).clone();second.depth=2.;
    r.floors.push(Arc::new(second));r.actors.insert("2".into(),Arc::new(vec![]));r.ground.insert("2".into(),Arc::new(vec![]));
    Arc::make_mut(&mut r.floors[0]).last_tick=3.;Arc::make_mut(&mut r.floors[0]).dirty=false;
    r.depth=2.;r.tick=12.;corpse_core::offscreen::validate_supported(&r).unwrap();
    corpse_core::offscreen::catch_up_floor(&mut r,1.).unwrap();
    assert_eq!(engine::condition(&r.actors_at(1.)[0],"monsterSummonReadyAt"),13.);
    assert_eq!(r.floors[0].last_tick,12.);
    r.depth=1.;engine::actor_mut(&mut r,4.).unwrap().state="hunting".into();assert!(!cast(&mut r));
    r.tick=13.;assert!(cast(&mut r));assert_eq!(ready(&r),23.);
}

#[test]
fn qa_single_open_cell_allows_five_paid_single_casts_not_free_unbounded_births() {
    let mut r=room("pale:Birther",32.);
    for y in 4..=6 {for x in 4..=6 {if (x,y)!=(5,5)&&(x,y)!=(4,4){r.floor_mut().unwrap().tiles[y*79+x]="water".into();}}}
    for n in 0..5 {
        r.tick=3.+10.*n as f64;assert!(cast(&mut r));assert_eq!(children(&r),1);
        assert_eq!(engine::actor(&r,4.).unwrap().energy,40.-10.*n as f64);
        let id=r.actors_at(1.).iter().find(|a|a.id!=4.).unwrap().id;
        engine::actor_mut(&mut r,id).unwrap().hp=0.;engine::death_if_nonpositive(&mut r,id).unwrap();
    }
    r.tick=53.;assert!(!cast(&mut r));assert_eq!(children(&r),0);
    assert_eq!(r.ground.get("1").unwrap().iter().filter(|g|g.item.kind=="heart").count(),5);
}
