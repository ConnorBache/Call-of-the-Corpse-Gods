use super::*;
use crate::engine::condition;

fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 52.0.into();
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.hp = 500.; r.player.max_hp = 500.;
    r.actors_at_mut(r.depth).clear(); r.inventory.clear();
    r
}
fn source(kind: &str) -> Actor { Actor { id: 800., creature_kind: kind.to_string().into(), name: kind.into(), hp: 100., max_hp: 100., ..Default::default() } }
fn advance(r: &mut Run) {
    r.tick += 1.; tick_bleed(r, r.player.id).unwrap();
}
#[test]
fn additive_wounds_deliver_half_once_in_three_normal_ticks() {
    let mut r = room(); let id = r.player.id; let a = source("killspawn");
    after_melee(&mut r, &a, id, 11., true).unwrap();
    after_melee(&mut r, &a, id, 8., true).unwrap();
    assert_eq!(condition(&r.player, BLEED), 3.);
    let start = r.player.hp;
    advance(&mut r); assert_eq!(start - r.player.hp, 4.);
    let hp = r.player.hp; tick_bleed(&mut r, id).unwrap(); assert_eq!(r.player.hp, hp);
    // A new hit adds its complete budget without extending the old wound.
    after_melee(&mut r, &a, id, 10., true).unwrap();
    advance(&mut r); advance(&mut r); advance(&mut r);
    assert_eq!(start - r.player.hp, 15.);
    assert!(!r.player.conditions.keys().any(|k| k == BLEED || k.starts_with(BLEED_PREFIX)));
}
#[test]
fn no_followup_for_legacy_zero_ranged_or_terminal_targets() {
    for kind in ["killspawn", "blorb"] {
        for (revision, amount, melee, dead) in [(51.,20.,true,false),(52.,0.,true,false),(52.,20.,false,false),(52.,20.,true,true)] {
            let mut r = room(); r.simulation_revision = revision.into(); r.player.dead = dead.into();
            let id = r.player.id; let before = r.clone();
            after_melee(&mut r, &source(kind), id, amount, melee).unwrap();
            assert_eq!(r, before);
        }
    }
}
#[test]
fn caustic_packet_is_one_actual_damage_and_respects_immunity() {
    let mut r = room(); let id = r.player.id; let a = source("blorb");
    after_melee(&mut r, &a, id, 5., true).unwrap(); assert_eq!(r.player.hp, 499.);
    r.player.abilities.push("enemy:pale:gas-immunity".into());
    after_melee(&mut r, &a, id, 5., true).unwrap(); assert_eq!(r.player.hp, 499.);
}
#[test]
fn save_roundtrip_and_cleanse_keep_budgets_exact() {
    let mut r = room(); let id = r.player.id;
    after_melee(&mut r, &source("killspawn"), id, 21., true).unwrap(); advance(&mut r);
    let mut loaded: Run = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    for _ in 0..2 { advance(&mut r); advance(&mut loaded); }
    assert_eq!(r, loaded); assert_eq!(r.player.hp, 489.);
    after_melee(&mut r, &source("killspawn"), id, 20., true).unwrap();
    crate::abilities::clear_status_conditions(&mut r.player);
    let hp = r.player.hp; for _ in 0..3 { advance(&mut r); } assert_eq!(r.player.hp, hp);
}
#[test]
fn actual_melee_pipeline_uses_final_hp_damage_and_terminal_kills_leave_no_wound() {
    for terminal in [false,true] {
        let mut r=room();r.player.x=10.;r.player.y=10.;r.player.stats.dr=20.;r.player.stats.dodge=-100.;
        if terminal {r.player.hp=1.;}
        let mut a=source("killspawn");a.x=11.;a.y=10.;a.state="hunting".into();a.stats.offense=80.;a.stats.accuracy=100.;
        r.actors_at_mut(r.depth).push(a);let id=r.player.id;let hp=r.player.hp;
        engine::attack(&mut r,800.,id,1.).unwrap();
        let pending:f64=r.player.conditions.iter().filter(|(k,_)|k.starts_with(BLEED_PREFIX)).map(|(_,v)|v).sum();
        if terminal {assert_eq!(pending,0.);} else {assert_eq!(pending,crate::combat::nearest_damage((hp-r.player.hp)*0.5));assert!(pending>0.);}
    }
}
#[test]
fn offscreen_wounds_finish_after_source_disappears_and_do_not_repeat() {
    let mut r=room();let mut target=source("rat");target.id=802.;target.x=10.;target.y=10.;target.hp=400.;target.max_hp=500.;target.base_max_hp=500.0.into();target.state="sleeping".into();
    r.actors_at_mut(r.depth).push(target);after_melee(&mut r,&source("killspawn"),802.,30.,true).unwrap();
    let depth=r.depth;let tick=r.tick;let f=r.floor_mut().unwrap();f.dirty=true;f.last_tick=tick;
    let mut control=r.clone();clear_bleed(engine::actor_mut_trusted(&mut control,802.).unwrap());
    for run in [&mut r,&mut control] {run.depth=2.;run.tick+=3.;crate::offscreen::catch_up_floor(run,depth).unwrap();}
    let body=|run:&Run|run.actors_at(depth).iter().find(|a|a.id==802.).unwrap().clone();
    assert_eq!(body(&control).hp-body(&r).hp,15.);assert_eq!(condition(&body(&r),BLEED),0.);
    let before=r.clone();crate::offscreen::catch_up_floor(&mut r,depth).unwrap();assert_eq!(r,before);
}
#[test]
fn temporal_snapshots_and_historical_cleansing_preserve_or_remove_whole_wounds() {
    let mut r=room();r.potion_rules_from=crate::Field::Null;let id=r.player.id;after_melee(&mut r,&source("killspawn"),id,13.,true).unwrap();
    let original=r.player.conditions.clone();
    let snapshot=crate::temporal::Snapshot::capture(&r);
    let future=crate::future_anchor::FutureAnchor::capture41(&r).unwrap();
    advance(&mut r);snapshot.restore(&mut r).unwrap();assert_eq!(r.player.conditions,original);
    assert_eq!(future.native_state().unwrap().player.conditions,original);
    r.inventory.push(crate::Item{id:900.,kind:"potion".into(),effect:"cleansing".to_string().into(),..Default::default()});
    let (x,y)=(r.player.x,r.player.y);assert!(crate::potions::apply(&mut r,0,x,y,false).unwrap());
    assert!(!r.player.conditions.keys().any(|k|k==BLEED||k.starts_with(BLEED_PREFIX)));
}
#[test]
fn malformed_saved_wounds_reject_before_any_dispatch_mutation() {
    for (key,value) in [("killspawnBleed:bad:1",1.),("killspawnBleed:3:NaN",1.),
        ("killspawnBleed:-1:1",1.),("killspawnBleed:2.5:1",1.),("killspawnBleed:3:1:extra",1.),
        ("killspawnBleed:3:1",0.),("killspawnBleed:3:1",-1.),("killspawnBleed:3:1",1.5),
        (BLEED,4.),(FLIGHT,7.),(FLIGHT,0.5)] {
        let mut r=room();r.player.conditions.insert(BLEED.into(),3.);r.player.conditions.insert(key.into(),value);
        let before=r.clone();assert!(validate_run(&r).is_err(),"{key}={value}");
        assert!(crate::storage::validate(&r).is_err());assert!(engine::dispatch(&mut r,&crate::Command::Wait).is_err());assert_eq!(r,before);
    }
    let mut r=room();r.player.conditions.insert("killspawnBleed:3:800".into(),1.);
    assert!(validate_run(&r).is_err(),"a budget must have its visible timer");
    r.player.conditions.clear();r.player.conditions.insert(BLEED.into(),3.);
    assert!(validate_run(&r).is_err(),"a visible timer must have an outstanding budget");
}
#[test]
fn lazy_inactive_wounds_and_temporal_admission_use_their_retained_clock() {
    let mut r=room();let mut target=source("rat");target.id=802.;target.x=10.;target.y=10.;target.state="sleeping".into();
    r.actors_at_mut(r.depth).push(target);after_melee(&mut r,&source("killspawn"),802.,30.,true).unwrap();
    r.next_id=900.;r.depth=2.;r.tick+=50.;
    assert!(validate_run(&r).is_ok());crate::storage::validate(&r).unwrap();
    let restored=crate::storage::from_canonical(&serde_json::to_string(&r).unwrap()).unwrap();assert_eq!(r,restored);
    let snapshot=crate::temporal::Snapshot::capture(&r);let future=crate::future_anchor::FutureAnchor::capture41(&r).unwrap();
    r.tick+=100.;snapshot.restore(&mut r).unwrap();validate_run(&r).unwrap();
    validate_run(future.native_state().unwrap()).unwrap();
}
