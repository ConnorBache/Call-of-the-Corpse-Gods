//! One serialized refill event per Fortune Teller. Spending never resets an
//! existing deadline. Full capacity has no scheduled work; old depleted saves
//! start a fresh interval on first upkeep instead of aligning to run milestones.
use crate::model::Actor;
use crate::turn_schedule::{Deadline, EventKey};
const KEY: EventKey = EventKey { scope: 0, owner: 0, kind: 1, channel: 0 };
const PERIOD: u64 = 20;
fn now(tick: f64) -> u64 { tick.max(0.).floor() as u64 }
fn count(a: &Actor) -> f64 { a.conditions.get("fortunes").copied().unwrap_or(3.).clamp(0.,3.) }

pub fn ensure(a: &mut Actor, tick: f64) {
    if count(a) >= 3. {
        if let Some(q)=a.fortune_schedule.as_mut() { q.cancel_key(KEY); }
        return;
    }
    let q=a.fortune_schedule.get_or_insert_with(Default::default);
    if q.get(KEY).is_none() {
        q.schedule(KEY,Deadline {tick:now(tick).saturating_add(PERIOD),phase:0},0,())
            .expect("fortune schedule generation exhausted");
    }
}

/// Called in the existing normal actor upkeep phase, not on free/haste inputs.
pub fn upkeep(a: &mut Actor,tick:f64) {
    ensure(a,tick);
    let cutoff=Deadline {tick:now(tick),phase:0};
    while let Some(event)=a.fortune_schedule.as_mut().and_then(|q|q.pop_through(cutoff)) {
        let n=(count(a)+1.).min(3.);
        a.conditions.insert("fortunes".into(),n);
        if n<3. {
            a.fortune_schedule.as_mut().unwrap().schedule(KEY,
                Deadline {tick:event.deadline.tick.saturating_add(PERIOD),phase:0},0,())
                .expect("fortune schedule generation exhausted");
        }
    }
}

pub fn turns_until(a:&Actor,tick:f64)->u8 {
    if count(a)>=3. {return 0;}
    a.fortune_schedule.as_ref().and_then(|q|q.get(KEY))
        .map(|e|e.deadline.tick.saturating_sub(now(tick)).min(PERIOD) as u8)
        .unwrap_or(PERIOD as u8)
}

#[cfg(test)] mod tests {
    use super::*;
    fn actor(n:f64)->Actor {let mut a=Actor::default();a.conditions.insert("fortunes".into(),n);a}
    #[test] fn spend_starts_relative_deadline_and_further_spends_do_not_reset() {
        let mut a=actor(2.);ensure(&mut a,19.);assert_eq!(turns_until(&a,19.),20);
        upkeep(&mut a,20.);assert_eq!(count(&a),2.);
        a.conditions.insert("fortunes".into(),1.);ensure(&mut a,25.);
        assert_eq!(turns_until(&a,25.),14);
        upkeep(&mut a,38.);assert_eq!(count(&a),1.);
        upkeep(&mut a,39.);assert_eq!(count(&a),2.);assert_eq!(turns_until(&a,39.),20);
        upkeep(&mut a,59.);assert_eq!(count(&a),3.);assert!(a.fortune_schedule.as_ref().unwrap().is_empty());
        a.conditions.insert("fortunes".into(),2.);ensure(&mut a,74.);assert_eq!(turns_until(&a,74.),20);
    }
    #[test] fn schedule_survives_save_and_catches_up_only_to_cap() {
        let mut a=actor(0.);ensure(&mut a,7.);
        let text=serde_json::to_string(&a).unwrap();let mut a:Actor=serde_json::from_str(&text).unwrap();
        assert_eq!(turns_until(&a,12.),15);upkeep(&mut a,100.);assert_eq!(count(&a),3.);
        assert_eq!(a.fortune_schedule.as_ref().unwrap().len(),0);
    }
    #[test] fn old_save_depleted_migrates_without_global_phase_and_full_has_no_work() {
        let mut a=actor(2.);upkeep(&mut a,40.);assert_eq!(count(&a),2.);assert_eq!(turns_until(&a,40.),20);
        let mut full=actor(3.);upkeep(&mut full,40.);assert!(full.fortune_schedule.is_missing());
    }
    #[test] fn real_activation_schedules_once_and_invalid_use_does_not() {
        use crate::{Run,Field};
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();r.tick=19.;
        r.player.abilities=vec!["morae-fortune-teller".into()];r.player.conditions.clear();
        let id=r.player.id;
        assert!(!crate::abilities::activate(&mut r,"morae-fortune-teller",&Field::Missing,&Field::Missing,&Field::Missing).unwrap());
        assert!(r.player.fortune_schedule.is_missing());
        crate::abilities::activate(&mut r,"morae-fortune-teller",&id.into(),&Field::Missing,&Field::Missing).unwrap();
        assert_eq!(count(&r.player),2.);assert_eq!(turns_until(&r.player,r.tick),20);
        r.tick=22.;crate::abilities::activate(&mut r,"morae-fortune-teller",&id.into(),&Field::Missing,&Field::Missing).unwrap();
        assert_eq!(count(&r.player),1.);assert_eq!(turns_until(&r.player,r.tick),17);
        let json=serde_json::to_string(&r).unwrap();let restored:Run=serde_json::from_str(&json).unwrap();
        assert_eq!(turns_until(&restored.player,22.),17);
        let mut old=r.clone();old.simulation_revision=51.0.into();old.player.fortune_schedule=Field::Missing;
        crate::abilities::activate(&mut old,"morae-fortune-teller",&id.into(),&Field::Missing,&Field::Missing).unwrap();
        assert!(old.player.fortune_schedule.is_missing());
    }

}
