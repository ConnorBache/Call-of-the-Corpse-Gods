//! Raw HP attack coefficients replace the old post-armor halving in current R52.
use corpse_core::{combat, engine, rng, Actor, Field, Run, Stats};
fn room(revision: f64) -> Run {
    let mut r: Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=revision.into();r.rng=123456.;r.next_id=1000.;
    r.player.x=10.;r.player.y=10.;r.player.hp=1000.;r.player.max_hp=1000.;r.player.base_max_hp=1000.0.into();
    r.player.stats=Stats{accuracy:1000.,perception:100.,..Default::default()};
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.inventory.clear();
    r.player.ability_reserves=Field::Missing;r.player.ability_order=Field::Missing;r.player.equipment_abilities=Field::Missing;
    for actors in r.actors.values_mut(){*actors=Default::default();}
    r.history.clear();r.haste_snapshot=Field::Missing;r.future=None;
    let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);
    f.environment=Field::Missing;f.doors=Field::Missing;f.portals=Field::Missing;f.non_euclidean=Field::Missing;
    r.actors_at_mut(r.depth).push(Actor{id:901.,name:"Target".into(),x:11.,y:10.,hp:1000.,max_hp:1000.,base_max_hp:1000.0.into(),state:"hunting".into(),heartless:true.into(),stats:Stats{offense:20.,accuracy:1000.,..Default::default()},..Default::default()});r
}
#[test]
fn full_dr_subtraction_and_zero_armor_equivalence() {
    let now=room(52.);let old=room(51.);
    for offense in [-20.,0.,1.,20.,100.] { for variance in [0.75,1.,1.25] { for surprise in [false,true] {
        let actor=Actor::default();
        let before=combat::creature_attack_base(51.,&actor,offense);
        let after=combat::creature_attack_base(52.,&actor,offense);
        assert_eq!(after,before/2.);
        assert_eq!(combat::ordinary_strike_damage_for_run(&now,after,variance,0.,surprise,false),combat::ordinary_strike_damage_for_run(&old,before,variance,0.,surprise,false));
    }}}
    for dr in 0..=20 {
        assert_eq!(combat::ordinary_strike_damage_for_run(&now,20.,1.,dr as f64,false,false),20.-dr as f64);
    }
    assert_eq!(combat::ordinary_strike_damage_for_run(&now,15.,1.,10.,false,false),5.);
    assert_eq!(combat::ordinary_strike_damage_for_run(&old,30.,1.,10.,false,false),10.);
    assert_eq!(combat::ordinary_strike_damage_for_run(&now,15.,1.,100.,false,false),0.);
}
#[test]
fn actual_enemy_hits_apply_full_dr_in_both_phases() {
    for phase in ["normal","haste"] {for dr in [0.,1.,10.,100.] {for paralyzed in [false,true] {
        let mut r=room(52.);r.phase=phase.to_string().into();r.player.stats.dr=dr;
        if paralyzed {r.player.conditions.insert("paralyzed".into(),1.);}
        engine::actor_mut(&mut r,901.).unwrap().conditions.insert("lucky".into(),1.);
        let mut predicted=r.clone();rng::random(&mut predicted);let variance=0.75+0.5*rng::random(&mut predicted);
        let expected=(15.*variance*if paralyzed{3.}else{1.}-dr).max(0.).round();
        let player=r.player.id;engine::attack(&mut r,901.,player,1.).unwrap();
        assert_eq!(1000.-r.player.hp,expected,"{phase}, DR{dr}, paralysis{paralyzed}");
    }}}
}
#[test]
fn explicit_profiles_are_already_raw_and_survive_save_and_snapshot() {
    let mut r=room(52.);let a=engine::actor_mut(&mut r,901.).unwrap();
    a.conditions.insert("weaponDamage".into(),18.9);a.conditions.insert("weaponOffenseMultiplier".into(),1.);
    assert_eq!(combat::creature_attack_base(52.,a,20.),38.9);
    let snapshot=corpse_core::temporal::Snapshot::capture(&r);
    let loaded=corpse_core::storage::deserialize_compatible(&corpse_core::storage::serialize(&r).unwrap()).unwrap();
    assert_eq!(loaded.actors,r.actors);snapshot.restore(&mut r).unwrap();
    assert_eq!(combat::creature_attack_base(52.,engine::actor(&r,901.).unwrap(),20.),38.9);
}
#[test]
fn attack_packet_is_unscaled_but_other_effects_and_legacy_packets_are_unchanged() {
    let current=room(52.);let legacy=room(51.);
    assert_eq!(combat::damage_packet(&current,&current.player,9.,"attack",false),9.);
    assert_eq!(combat::damage_packet(&legacy,&legacy.player,9.,"attack",false),5.);
    for kind in ["fire","caustic","bleeding","poison","banshee","hemophilia","blood-letting","blood-weapon","explosion"] {
        assert_eq!(combat::damage_packet(&current,&current.player,20.,kind,false),combat::damage_packet(&legacy,&legacy.player,20.,kind,false),"{kind}");
    }
}
#[test]
fn copied_and_thrown_attack_components_are_half_sized_without_changing_stats() {
    let mut r=room(52.);r.player.stats.offense=20.;
    let mini=corpse_core::mini_me::preview(&r,12.,10.);
    assert_eq!(mini.stats.offense,20.);
    assert_eq!(mini.conditions["weaponDamage"],18.9);
    assert_eq!(mini.conditions["weaponOffenseMultiplier"],1.);
    assert_eq!(combat::creature_attack_base(52.,&mini,20.),corpse_core::revision50_player_attack::base(&r));
    for offense in [-20.,0.,1.5,20.,100.] {for thrown in [false,true] {
        let item=corpse_core::Item{kind:"weapon".into(),hands:1.0.into(),damage:30.0.into(),enchantment:3.0.into(),offense_multiplier:1.5.into(),thrown:thrown.into(),..Default::default()};
        assert_eq!(corpse_core::projectiles::player_throw_base_for_revision(&item,offense,52.),corpse_core::projectiles::player_throw_base_for_revision(&item,offense,51.)/2.);
    }}
}
#[test]
fn ruin_pulse_authored_packets_preserve_real_damage_without_attack_scaling() {
    for quality in ["infant","child","lesser","unqualified","mature","greater","hulking","ancient","timeless","transcendent"] {
        let a=Actor{zillennia_quality:quality.to_string().into(),..Default::default()};
        let old=corpse_core::monster_ruin::ruin_damage_revision(&a,51.);
        let new=corpse_core::monster_ruin::ruin_damage_revision(&a,52.);
        assert_eq!(new,old/2.);
        assert_eq!(combat::tuned_damage_for_run(&room(52.),new,Some("attack")),combat::tuned_damage_for_run(&room(51.),old,Some("attack")));
    }
}
