//! One serialized deadline per successful scream; no per-turn actor/status scan.
use crate::{Actor, Run};
use crate::turn_schedule::{Deadline, EventKey};
pub const TAG: &str = "dontScream";
fn key(id: f64) -> EventKey { EventKey { scope: 0, owner: id.to_bits(), kind: 2, channel: 0 } }
fn now(tick: f64) -> u64 { tick.max(0.).floor() as u64 }
// Keep a constant-size historical receipt after tag cleanup: inactive floors
// may later simulate ticks preceding the current global clock. Owner identity
// prevents copied conditions from suppressing a clone that never screamed.
const UNTIL: &str = "lastScreamUntil";
const OWNER: &str = "lastScreamOwner";
pub fn blocked(r: &Run, a: &Actor) -> bool {
    crate::screamer_rules::current(r)
        && a.conditions.get(OWNER) == Some(&a.id)
        && a.conditions.get(UNTIL).is_some_and(|&due| r.tick < due)
}
/// Called only after the paid scream has actually applied its effects. Duration
/// is the max of actual recipient durations, accumulated in that existing pass.
pub fn after_scream(r: &mut Run, id: f64, duration: f64) {
    if !crate::screamer_rules::current(r) || duration <= 0. { return; }
    let due = now(r.tick).saturating_add(duration.ceil() as u64);
    r.screamer_schedule.get_or_insert_with(Default::default).schedule(
        key(id), Deadline { tick: due, phase: 0 }, 0, ())
        .expect("screamer schedule generation exhausted");
    if let Some(a)=crate::engine::actor_any_mut(r,id) {
        a.conditions.insert(TAG.into(),due as f64);
        a.conditions.insert(UNTIL.into(),due as f64);
        a.conditions.insert(OWNER.into(),id);
    }
}
/// One queue peek each normal world turn; only due actors are looked up. The
/// previous turn's environment has already decremented scream statuses. Haste,
/// Fast movement and free commands do not advance this deadline.
pub fn expire(r: &mut Run) {
    if !crate::screamer_rules::current(r) {return;}
    let cutoff=Deadline {tick:now(r.tick),phase:0};
    while let Some(e)=r.screamer_schedule.as_mut().and_then(|q|q.pop_through(cutoff)) {
        let id=f64::from_bits(e.handle.key.owner);
        if let Some(a)=crate::engine::actor_any_mut(r,id) {a.conditions.shift_remove(TAG);}
    }
    if r.screamer_schedule.as_ref().is_some_and(|q|q.is_empty()) {r.screamer_schedule=None;}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Field, engine};
    fn room(greater: bool) -> Run {
        let mut r: Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();r.screamer_rules_from=0usize.into();
        r.floor_mut().unwrap().tiles.fill("floor".into());
        r.inventory.clear();r.player.conditions.clear();r.player.active.clear();r.player.abilities.clear();
        r.player.x=12.;r.player.y=14.;
        let mut a=r.actors_at(r.depth).iter().next().unwrap().clone();
        a.id=900.;a.x=14.;a.y=14.;a.hp=100.;a.max_hp=100.;a.energy=100.;
        a.creature_kind=Field::Missing;
        a.name=if greater {"Greater Pale Screamer"}else{"Pale Screamer"}.into();
        a.abilities=vec!["enemy:pale:scream".into()];a.conditions.clear();
        a.conditions.insert("bestiaryTraitsInitialized".into(),1.);
        r.actors_at_mut(r.depth).clear();r.actors_at_mut(r.depth).push(a);r
    }
    fn scream(r:&mut Run)->bool {crate::ai::enemy_action(r,900.,r.player.id).unwrap()}
    #[test]
    fn scream_requires_player_in_effect_range_before_spending_or_scheduling() {
        for greater in [false,true] {
            let mut r = room(greater);
            r.screamer_range_from = 0usize.into();
            r.player.x = 3.; // 11 tiles from the screamer.
            crate::events::begin();
            assert!(!scream(&mut r));
            assert!(crate::events::finish().is_empty());
            assert_eq!(engine::actor(&r,900.).unwrap().energy,100.);
            assert!(r.screamer_schedule.is_none());
            assert_eq!(engine::condition(&r.player,"disoriented"),0.);

            r.player.x = 4.; // Exactly 10 tiles.
            assert!(scream(&mut r));
            assert_eq!(engine::actor(&r,900.).unwrap().energy,85.);
            assert_eq!(engine::condition(&r.player,"disoriented"),5.);
            assert!(!scream(&mut r));

            let mut legacy = room(greater);
            legacy.player.x = 3.;
            assert!(scream(&mut legacy));
            assert_eq!(engine::condition(&legacy.player,"disoriented"),0.);
        }
    }

    #[test]
    fn range_gate_checks_the_player_not_a_nearer_target() {
        let mut r = room(false);
        r.screamer_range_from = 0usize.into();
        r.player.x = 3.;
        let caster = engine::actor_mut_trusted(&mut r,900.).unwrap();
        caster.conditions.insert(crate::revision50_control_amulets::DISCORD.into(), 100.);
        let mut target = caster.clone();
        target.conditions.clear();
        target.abilities.clear();
        target.id = 901.;
        target.x = 16.;
        r.actors_at_mut(r.depth).push(target);
        assert!(!crate::ai::enemy_action(&mut r,900.,901.).unwrap());
        assert_eq!(engine::actor(&r,900.).unwrap().energy,100.);
        assert!(r.screamer_schedule.is_none());
        r.player.x = 4.;
        assert!(crate::ai::enemy_action(&mut r,900.,901.).unwrap());
    }

    #[test]
    fn screamer_success_blocks_both_variants_for_actual_five_turns() {
        for greater in [false,true] {
            let mut r=room(greater);let tick=r.tick;
            assert!(scream(&mut r));
            assert_eq!(engine::condition(engine::actor(&r,900.).unwrap(),"screamBonus"),if greater {10.}else{5.});
            assert_eq!(engine::condition(engine::actor(&r,900.).unwrap(),TAG),tick+5.);
            assert_eq!(engine::actor(&r,900.).unwrap().energy,85.);
            crate::events::begin();
            assert!(!scream(&mut r));assert_eq!(engine::actor(&r,900.).unwrap().energy,85.);
            assert!(crate::events::finish().is_empty());
            for t in 0..5 {
                r.tick=tick+t as f64;expire(&mut r);
                assert!(blocked(&r,engine::actor(&r,900.).unwrap()));
                crate::environment::tick_environment(&mut r).unwrap();
            }
            assert_eq!(engine::condition(engine::actor(&r,900.).unwrap(),"screamEmboldened"),0.);
            r.tick=tick+5.;expire(&mut r);
            assert!(!blocked(&r,engine::actor(&r,900.).unwrap()));assert!(scream(&mut r));
        }
    }
    #[test]
    fn screamer_failed_gates_never_schedule_and_expiry_does_not_bypass_them() {
        let mut r=room(false);engine::actor_mut_trusted(&mut r,900.).unwrap().energy=0.;
        assert!(!scream(&mut r));assert!(r.screamer_schedule.is_none());
        engine::actor_mut_trusted(&mut r,900.).unwrap().energy=100.;assert!(scream(&mut r));
        r.tick+=5.;expire(&mut r);engine::actor_mut_trusted(&mut r,900.).unwrap().conditions.insert("paralyzed".into(),1.);
        assert!(!scream(&mut r));assert!(r.screamer_schedule.is_none());
    }
    #[test]
    fn screamer_queue_roundtrip_rewind_identity_and_offscreen_expiry() {
        let mut r=room(false);assert!(scream(&mut r));
        let snapshot=crate::temporal::Snapshot::capture(&r);
        let saved=serde_json::to_string(&r).unwrap();let mut loaded:Run=serde_json::from_str(&saved).unwrap();
        assert!(blocked(&loaded,engine::actor(&loaded,900.).unwrap()));
        let mut copy=engine::actor(&r,900.).unwrap().clone();copy.id=901.;
        assert!(!blocked(&r,&copy));r.actors_at_mut(r.depth).push(copy);
        after_scream(&mut r,901.,3.);r.tick+=3.;expire(&mut r);
        assert!(!blocked(&r,engine::actor(&r,901.).unwrap()));assert!(blocked(&r,engine::actor(&r,900.).unwrap()));
        r.tick+=2.;expire(&mut r);assert!(r.screamer_schedule.is_none());
        snapshot.restore(&mut r).unwrap();assert!(blocked(&r,engine::actor(&r,900.).unwrap()));
        loaded.depth=2.;loaded.tick+=5.;expire(&mut loaded);assert!(loaded.screamer_schedule.is_none());
        assert_eq!(engine::condition(loaded.actors_at(1.).iter().next().unwrap(),TAG),0.);
        r.actors_at_mut(r.depth).clear();r.tick+=5.;expire(&mut r);assert!(r.screamer_schedule.is_none());
    }
    #[test]
    fn screamer_legacy_policy_preserves_repeated_screams() {
        let mut r=room(false);r.screamer_rules_from=Field::Null;
        assert!(scream(&mut r));assert!(scream(&mut r));assert!(r.screamer_schedule.is_none());
        crate::screamer_rules::apply_replay_policy(&mut r,&Field::Value(2),2);
        assert!(scream(&mut r));assert!(!scream(&mut r));
    }
    #[test]
    fn screamer_commands_replay_and_restart_match_new_and_legacy_runs() {
        for legacy in [false,true] {
            let mut initial=room(false);
            initial.player.hp=10000.;initial.player.max_hp=10000.;
            initial.screamer_rules_from=if legacy {Field::Null}else{0usize.into()};
            let mut core=crate::temporal::Engine::new(initial.clone()).unwrap();
            for _ in 0..7 {core.dispatch(&crate::Command::Wait).unwrap();}
            let record=crate::replay::ReplayRecord::try_from_run(&core.run).unwrap();
            let mut replay=crate::replay::ReplaySession::from_initial(initial,record).unwrap();
            for restart in [false,true] {
                if restart {replay.restart().unwrap();}
                while replay.step().unwrap() {}
                assert_eq!(replay.engine.save().unwrap(),core.save().unwrap());
            }
        }
    }

    #[test]
    fn screamer_historical_catchup_retains_deadline_after_global_expiry() {
        let mut r=room(false);r.tick=10.;assert!(scream(&mut r));
        // Leave the caster's floor and advance global time beyond the deadline.
        r.depth=2.;r.tick=30.;expire(&mut r);
        assert!(r.screamer_schedule.is_none());
        assert_eq!(engine::condition(engine::actor_any(&r,900.).unwrap(),TAG),0.);
        // Same indexed actor, now simulated through historical inactive ticks.
        r.depth=1.;
        for tick in 11..15 {r.tick=tick as f64;assert!(!scream(&mut r));}
        r.tick=15.;assert!(scream(&mut r));
        assert_eq!(engine::condition(engine::actor(&r,900.).unwrap(),TAG),20.);
        r.tick=19.;assert!(!scream(&mut r));r.tick=20.;expire(&mut r);assert!(scream(&mut r));
    }

    #[test]
    fn screamer_mixed_willpower_duration_keeps_longest_applied_status() {
        let mut r=room(false);r.ring_rules_from=0usize.into();
        r.inventory.push(crate::Item {id:998.,kind:"ring".into(),effect:"willpower".to_string().into(),..Default::default()});
        assert!(scream(&mut r));
        assert_eq!(engine::condition(&r.player,"disoriented"),3.);
        assert_eq!(engine::condition(engine::actor(&r,900.).unwrap(),"screamEmboldened"),5.);
        assert_eq!(engine::condition(engine::actor(&r,900.).unwrap(),TAG),r.tick+5.);
    }

    #[test]
    fn screamer_real_offscreen_proxy_catchup_retains_historical_lockout() {
        let mut r=room(false);r.tick=10.;assert!(scream(&mut r));
        r.floor_mut().unwrap().last_tick=10.;r.floor_mut().unwrap().dirty=true;
        let a=engine::actor_mut_trusted(&mut r,900.).unwrap();
        a.conditions.insert(crate::revision50_control_amulets::DISCORD.into(),100.);
        a.stats.offense=-100.;a.stats.max_energy=100.;
        let mut target=a.clone();target.id=901.;target.name="Target".into();target.abilities.clear();
        target.conditions.clear();target.conditions.insert("paralyzed".into(),100.);
        target.x=16.;target.hp=10000.;target.max_hp=10000.;r.actors_at_mut(1.).push(target);
        r.depth=2.;r.tick=16.;expire(&mut r);
        crate::offscreen::catch_up_floor(&mut r,1.).unwrap();
        let a=engine::actor_any(&r,900.).unwrap();
        // A premature scream at historical tick 11 would shift this to 21.
        assert_eq!(engine::condition(a,UNTIL),20.);
        assert_eq!(engine::condition(a,TAG),20.);
        assert!(r.screamer_schedule.as_ref().unwrap().get(key(900.)).is_some());
    }

}
