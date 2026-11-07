//! Exact requested wording; old recordings keep their original canonical logs.
use corpse_core::{engine, Field, Run};
fn fixture(revision:f64)->Run {
    let mut r=corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=revision.into();r.history.clear();r.haste_snapshot=Field::Missing;
    r.log.clear();r.log_kinds=Vec::new().into();r.log_archive=Field::Missing;r
}
#[test]
fn canonical_messages_change_only_in_revision43_and_archive_matches() {
    for revision in [42.,43.] {
        let mut r=fixture(revision);
        for (old,new) in [
            ("Silence prevents this energy-paid activation.","Your ability is silenced and does not work."),
            ("No eligible outside-god abilities remain for Tinker.","You can't think of any new ideas to tinker with..."),
            ("You died unpreventably.","You died."),
        ] {engine::log(&mut r,old,"player");assert_eq!(r.log.last().unwrap(),if revision==43.{new}else{old});}
        let saved=corpse_core::storage::serialize(&r).unwrap();let restored=corpse_core::storage::deserialize(&saved).unwrap();
        assert_eq!(restored.log,r.log);assert_eq!(restored.log_archive,r.log_archive);
    }
}
#[test]
fn silenced_actual_activation_uses_requested_message_without_payment() {
    for revision in [42.,43.] {
        let mut r=fixture(revision);r.player.conditions.insert("silenced".into(),2.);
        r.player.abilities.push("anavani-ascend".into());let energy=r.player.energy;
        assert!(!corpse_core::abilities::activate(&mut r,"anavani-ascend",&Field::Missing,&Field::Missing,&Field::Missing).unwrap());
        assert_eq!(r.player.energy,energy);
        assert_eq!(r.log.last().unwrap(),if revision==43.{"Your ability is silenced and does not work."}else{"Silence prevents this energy-paid activation."});
    }
}
#[test]
fn normal_death_text_does_not_remove_unpreventable_death_provenance() {
    let mut r=fixture(43.);let id=r.player.id;r.player.hp=0.;
    engine::unpreventable_death(&mut r,id,"borge-shard").unwrap();
    assert_eq!(r.log.last().unwrap(),"You died.");
    assert!(r.run_stats.as_ref().unwrap().death.as_ref().unwrap().unpreventable);
    assert_ne!(r.status,"playing");
}
#[test]
fn exhausted_tinker_and_success_use_exact_requested_text() {
    for revision in [42.,43.] {
        let mut r=fixture(revision);r.gods=(0..12).map(|x|x as f64).collect();
        let rng=r.rng;let energy=r.player.energy;
        assert!(!corpse_core::tinker::activate(&mut r,Some(1.)).unwrap());
        assert_eq!(r.rng,rng);assert_eq!(r.player.energy,energy);
        assert_eq!(r.log.last().unwrap(),if revision==43.{"You can't think of any new ideas to tinker with..."}else{"No eligible outside-god abilities remain for Tinker."});
        r.gods.clear();r.player.energy=100.;
        assert!(corpse_core::tinker::activate(&mut r,Some(1.)).unwrap());
        let message=r.log.last().unwrap();assert!(message.contains(" is blessed with "));
        assert_eq!(message.ends_with(" It grants this ability while equipped."),revision==42.);
    }
}
