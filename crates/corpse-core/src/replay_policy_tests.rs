#[cfg(test)]
mod policy_tests {
    use super::*;
    /// O(fixture decode), controlled genuine Revert-capable initial state.
    fn initial()->Run {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=39.0.into();for a in r.actors.values_mut(){*a=std::sync::Arc::new(vec![]);}
        r.player.hp=100_000.;r.player.max_hp=100_000.;r.player.base_max_hp=100_000.0.into();r.player.energy=100.;r.player.stats.max_energy=100.;
        r.player.abilities=vec!["numo-revert".into(),"morae-see-the-future".into()];r.player.active.clear();r.player.conditions.clear();r.offers.clear();r.replay.clear();r.history.clear();r.future=None;r.haste_snapshot=Field::Missing;r.replay_snapshot_from=Field::Missing;
        crate::progression::initialize_ability_state(&mut r.player,"numo-revert",r.tick).unwrap();r.player.conditions.insert("revertCharges".into(),20.);r
    }
    fn revert()->Command {Command::Ability{id:"numo-revert".into(),target:Field::Missing,x:Field::Missing,y:Field::Missing}}
    /// O(commands*simulation+export), exact current+retained history after restart.
    fn compare(initial:Run, expected:&Engine,record:ReplayRecord){
        let mut replay=ReplaySession::from_initial(initial,record).unwrap();
        while replay.step().unwrap(){}
        assert_eq!(replay.engine.save().unwrap(),expected.save().unwrap());
        replay.restart().unwrap();while replay.step().unwrap(){}
        assert_eq!(replay.engine.save().unwrap(),expected.save().unwrap());
    }
    #[test]
    fn v2_snapshot_three_wait_two_reverts_roundtrips_and_restarts(){
        let initial=initial();let mut e=Engine::new(initial.clone()).unwrap();
        for c in [Command::Wait,Command::Wait,Command::Wait,revert(),revert()]{e.dispatch(&c).unwrap();}
        assert_eq!(e.run.replay_snapshot_from,Field::Value(0));let record=ReplayRecord::try_from_run(&e.run).unwrap();assert_eq!(record.version,2.);assert_eq!(record.revert_policy.as_ref().map(String::as_str),Some("snapshot"));
        let restored=Engine::load(&e.save().unwrap()).unwrap();assert_eq!(restored.run.replay_snapshot_from,Field::Value(0));compare(initial,&e,ReplayRecord::try_from_run(&restored.run).unwrap());
    }
    #[test]
    fn v1_legacy_and_v2_mixed_transition_reproduce_retained_history(){
        let initial=initial();let mut e=Engine::new(initial.clone()).unwrap();e.set_legacy_revert(true);
        for c in [Command::Wait,Command::Wait,Command::Wait,revert()]{e.dispatch(&c).unwrap();}
        let mut v1=ReplayRecord::from_run(&e.run);v1.version=1.;v1.revert_policy=Field::Missing;v1.snapshot_from=Field::Missing;compare(initial.clone(),&e,v1);
        let boundary=e.run.replay.len();e.set_legacy_revert(false);assert_eq!(e.run.replay_snapshot_from,Field::Value(boundary));
        // A switch at the end is preserved even with no following command.
        compare(initial.clone(),&e,ReplayRecord::try_from_run(&e.run).unwrap());
        for c in [Command::Wait,revert(),Command::Wait,revert()]{e.dispatch(&c).unwrap();}
        let loaded=Engine::load(&e.save().unwrap()).unwrap();assert_eq!(loaded.run.replay_snapshot_from,Field::Value(boundary));compare(initial,&e,ReplayRecord::try_from_run(&loaded.run).unwrap());
    }
    #[test]
    fn snapshot_and_future_restore_preserve_policy_with_live_ledger(){
        let mut e=Engine::new(initial()).unwrap();e.set_legacy_revert(true);e.dispatch(&Command::Wait).unwrap();e.set_legacy_revert(false);
        let boundary=e.run.replay_snapshot_from.clone();e.dispatch(&Command::Wait).unwrap();
        crate::temporal::remember_future(&mut e.run,&e.history).unwrap();e.dispatch(&Command::Wait).unwrap();let ledger=e.run.replay.clone();
        assert!(crate::temporal::restore_future(&mut e.run,&mut e.history).unwrap());assert_eq!(e.run.replay_snapshot_from,boundary);assert_eq!(e.run.replay,ledger);
        let snapshot=crate::temporal::Snapshot::capture(&e.run);assert!(snapshot.state.replay_snapshot_from.is_missing());assert!(snapshot.value().unwrap().get("replaySnapshotFrom").is_none());
        snapshot.restore(&mut e.run).unwrap();assert_eq!(e.run.replay_snapshot_from,boundary);
        let mut imported=snapshot.value().unwrap();imported["replaySnapshotFrom"]=serde_json::json!(123);
        let reconstructed=crate::temporal::Snapshot::from_value(imported).unwrap();
        assert!(reconstructed.state.replay_snapshot_from.is_missing());assert!(reconstructed.state.replay.is_empty());
    }
    #[test]
    fn actual_future_commands_keep_mixed_policy_after_restore_and_restart(){
        let initial=initial();let mut e=Engine::new(initial.clone()).unwrap();e.set_legacy_revert(true);
        let future=Command::Ability{id:"morae-see-the-future".into(),target:Field::Missing,x:Field::Missing,y:Field::Missing};
        e.dispatch(&Command::Wait).unwrap();e.dispatch(&future).unwrap();assert!(e.run.future.is_some());
        let boundary=e.run.replay.len();e.set_legacy_revert(false);
        e.dispatch(&Command::Wait).unwrap();e.dispatch(&future).unwrap();assert!(e.run.future.is_none());
        assert_eq!(e.run.replay_snapshot_from,Field::Value(boundary));
        e.dispatch(&Command::Wait).unwrap();e.dispatch(&revert()).unwrap();
        compare(initial,&e,ReplayRecord::try_from_run(&e.run).unwrap());
    }
    #[test]
    fn unknown_and_invalid_policy_is_rejected_without_guessing(){
        let mut e=Engine::new(initial()).unwrap();e.run.replay_snapshot_from=Field::Null;
        assert!(ReplayRecord::try_from_run(&e.run).unwrap_err().to_string().contains("unknown"));
        assert!(ReplaySession::from_initial(initial(),ReplayRecord::from_run(&e.run)).is_err());
        let mut base=ReplayRecord::from_run(&initial());base.version=2.;base.revert_policy=Field::Value("legacy".into());
        for bad in [Field::Null,Field::Value(1)]{let mut r=base.clone();r.snapshot_from=bad;assert!(r.validate_policy().is_err());}
        for policy in ["unknown","future-policy"]{let mut r=base.clone();r.revert_policy=Field::Value(policy.into());assert!(r.validate_policy().is_err());}
        let mut r=base.clone();r.version=3.;assert!(r.validate_policy().is_err());r.version=1.;assert!(r.validate_policy().is_err());
        let mut run=initial();run.replay_snapshot_from=Field::Value(1);assert!(Engine::new(run).is_err());
    }
}
