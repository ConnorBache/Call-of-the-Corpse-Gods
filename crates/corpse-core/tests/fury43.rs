//! Revision43 Fury is one stronger packet, not another attack/proc or RNG draw.
use corpse_core::{crafting, engine, rng, combat, Run, ItemEnhancementsEntry};
fn fixture(revision:f64)->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=revision.into(); r.history.clear(); r.haste_snapshot=corpse_core::Field::Missing;
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.energy=100.; r.player.stats.offense=85.; r.player.stats.accuracy=68.;
    r.inventory.retain(|i|i.id==1.);
    r.inventory[0].enhancements=vec![ItemEnhancementsEntry {craft:"shrail-fury-crafting".into(),active:true.into(),id:"craft:1:0".to_string().into(),..Default::default()}];
    let a=&mut r.actors_at_mut(1.)[0];a.x=13.;a.y=14.;a.hp=1_000_000.;a.max_hp=1_000_000.;a.abilities.clear();a.active.clear();a.conditions.clear();a.stats.dodge=40.;a.stats.dr=22.;a.state="hunting".into();
    r
}
#[test]
fn preview_and_payment_share_revision_bonus_and_funding_order() {
    for (revision,bonus) in [(42.,10.),(43.,114.)] {
        for (energy,paid) in [(9.,0.),(10.,1.),(19.,1.),(20.,2.)] {
            let mut r=fixture(revision);r.player.energy=energy;let extra=r.inventory[0].enhancements[0].clone();r.inventory[0].enhancements.push(extra);
            let before=r.rng;
            assert_eq!(crafting::preview_fury_bonus(&r,energy),(bonus*paid,10.*paid));
            assert_eq!(r.rng,before);
            assert_eq!(crafting::fury_bonus(&mut r),bonus*paid);
            assert_eq!(r.player.energy,energy-10.*paid);assert_eq!(r.rng,before);
            assert_eq!(r.inventory[0].enhancements.iter().filter(|e|e.active.value_or(false)).count(),paid as usize);
        }
    }
}
#[test]
fn actual_single_strikes_match_hit_variance_mitigation_and_rounding() {
    let base=fixture(43.);let mut checked=0;
    for accuracy in [35.,68.] { for offense in [0.,85.,170.] { for dr in [0.,22.,100.] {for seed in 0..32 {
        for (revision,bonus) in [(42.,10.),(43.,114.)] {
            let mut r=base.clone();r.simulation_revision=revision.into();r.player.stats.offense=offense;r.player.stats.accuracy=accuracy;r.actors_at_mut(1.)[0].stats.dr=dr;r.rng=rng::seed_hash(&format!("fury43-packet-{seed}")) as f64;
            let mut prediction=r.clone();let hit=rng::random(&mut prediction)<combat::hit_chance(accuracy,40.,0.);
            let expected=if hit {combat::ordinary_strike_damage(30.+offense+bonus,0.75+rng::random(&mut prediction)*0.5,dr,false,false)} else {0.};
            let pid=r.player.id;let enemy=r.actors_at(1.)[0].id;let hp=r.actors_at(1.)[0].hp;
            engine::attack(&mut r,pid,enemy,1.).unwrap();
            assert_eq!(hp-r.actors_at(1.)[0].hp,expected,"revision{revision} offense{offense} dr{dr} seed{seed}");
            assert_eq!(r.player.energy,90.);checked+=1;
        }
    }}}}
    assert_eq!(checked,1152);
}
#[test]
fn assault_and_offhand_do_not_duplicate_fury_payment_or_on_hit_procs() {
    for offhand in [false,true] {
        let mut old=fixture(42.);old.player.stats.accuracy=1000.;old.player.energy=40.;
        old.player.abilities.push(corpse_core::divine_combat::BEAT_TO_DEATH.into());
        if offhand {let mut weapon=old.inventory[0].clone();weapon.id=99.;weapon.equipped="hand2".to_string().into();weapon.enhancements.clear();old.inventory.push(weapon);}
        else {old.player.abilities.push(corpse_core::abilities::ASSAULT.into());old.player.active.push(corpse_core::abilities::ASSAULT.into());}
        let mut new=old.clone();new.simulation_revision=43.0.into();let id=old.actors_at(1.)[0].id;
        engine::player_attack(&mut old,id).unwrap();engine::player_attack(&mut new,id).unwrap();
        assert_eq!(old.rng,new.rng,"no added hit/variance/proc rolls");
        assert_eq!(old.player.energy,new.player.energy);
        assert_eq!(new.player.energy,if offhand {30.} else {20.});
        assert_eq!(old.actors_at(1.)[0].conditions.get("beatToDeathHits"),new.actors_at(1.)[0].conditions.get("beatToDeathHits"));
        assert!(new.actors_at(1.)[0].hp<old.actors_at(1.)[0].hp);
        assert_eq!(new.log.iter().filter(|s|s.starts_with("You strike ")).count(),old.log.iter().filter(|s|s.starts_with("You strike ")).count());
    }
}
#[test]
fn miss_still_pays_once_and_does_not_gain_a_second_attempt() {
    let mut old=fixture(42.);old.player.stats.accuracy=-10_000.;let mut new=old.clone();new.simulation_revision=43.0.into();let id=old.actors_at(1.)[0].id;
    engine::player_attack(&mut old,id).unwrap();engine::player_attack(&mut new,id).unwrap();
    assert_eq!(old.rng,new.rng);assert_eq!(new.player.energy,90.);assert_eq!(new.actors_at(1.)[0].hp,1_000_000.);
    assert_eq!(new.log.iter().filter(|s|s.contains(", but miss.")).count(),1);
}
