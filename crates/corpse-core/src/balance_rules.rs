//! Live R52 divine balance changes and their historical command boundary.
use crate::{Field, Run};

pub fn current(run: &Run) -> bool {
    run.simulation_revision.value_or(29.) >= 52. && matches!(run.balance_rules_from, Field::Value(_))
}

pub(crate) fn validate(run: &Run) -> Result<(), String> {
    if !run.balance_rules_from.is_missing()
        && (run.simulation_revision.value_or(29.) < 52.
            || run.balance_rules_from.as_ref().is_some_and(|&n| n > run.replay.len()))
    {
        return Err("Invalid divine balance command boundary".into());
    }
    Ok(())
}

/// An old live save adopts current divine balance after its existing command prefix.
pub fn admit_current(run: &mut Run) -> Result<(), String> {
    validate(run)?;
    if run.simulation_revision.value_or(29.) >= 52. && run.balance_rules_from.is_missing() {
        run.balance_rules_from = run.replay.len().into();
    }
    Ok(())
}

pub(crate) fn apply_replay_policy(run: &mut Run, from: &Field<usize>, cursor: usize) {
    if run.simulation_revision.value_or(29.) < 52. { return; }
    if from.as_ref() == Some(&cursor) {
        run.balance_rules_from = cursor.into();
    } else if cursor == 0 {
        run.balance_rules_from = Field::Missing;
    }
}

/// Current cost to set the anchor; returning still costs nothing.
pub const DREAM_ANCHOR_COST: f64 = 20.;
pub fn dream_anchor_cost(run: &Run) -> f64 {
    if current(run) { DREAM_ANCHOR_COST } else { 50. }
}

/// Sudden Sleep advances its own activation turn before control returns,
/// so use an extra scheduler tick to deliver 25 turns after activation.
pub fn sudden_sleep_ticks(run: &Run) -> f64 {
    if current(run) { 26. } else { 51. }
}

/// Neither uncapped energy nor zero maximum has a meaningful percentage.
pub fn sleep_walk_active(run: &Run, actor: &crate::Actor) -> bool {
    if !actor.abilities.iter().any(|id| id == "ursa-sleep-walk") { return false; }
    let balanced = current(run);
    if balanced && crate::engine::condition(actor, "slowed") > 0. { return false; }
    if actor.energy == 0. { return true; }
    if !balanced || actor.abilities.iter().any(|id| id == "grisha-boundless-hunger") {
        return false;
    }
    let cap = crate::combat::effective_stats(run, actor).max_energy;
    cap.is_finite() && cap > 0. && actor.energy < 0.1 * cap
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Command, divine_rescues::{self, OVERFLOWING}, engine, temporal::Engine};

    fn room(marker: Field<usize>) -> Run {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 52.0.into();
        r.balance_rules_from = marker;
        r.player.abilities.clear();
        r.player.conditions.clear();
        r.player.active.clear();
        r.player.ability_reserves = Field::Missing;
        r.player.energy = 100.;
        r.player.blood = Field::Missing;
        r.player.damage_bank = Field::Missing;
        r.player.stats.max_energy = 100.;
        r.player.hp = 100.;
        r.player.max_hp = 100.;
        r.player.base_max_hp = 100.0.into();
        r.inventory.clear();
        r.offers.clear();
        for actors in r.actors.values_mut() { std::sync::Arc::make_mut(actors).clear(); }
        crate::revision50_ability_energy::reconcile_acquired(&mut r).unwrap();
        r
    }

    #[test]
    fn five_saves_then_death_damage_does_not_refill_and_level_up_does() {
        let mut r = room(0usize.into());
        r.player.abilities.push(OVERFLOWING.into());
        let id = r.player.id;
        for left in (0..5).rev() {
            engine::damage_environment(&mut r,id,1000.,None,"blood-weapon",true).unwrap();
            assert_eq!(r.player.hp,1.);
            assert_eq!(divine_rescues::saves(&r.player),left as f64);
        }
        assert_eq!(r.player.blood.value_or(0.),0.);
        assert_eq!(r.player.damage_bank.value_or(0.),0.);
        let mut no_saves = r.clone();
        engine::damage_environment(&mut no_saves,id,1000.,None,"blood-weapon",true).unwrap();
        assert_eq!(no_saves.status,"dead");

        r.exp = crate::progression::level_up_cost(&r);
        crate::progression::level_up(&mut r);
        assert_eq!(r.player.hp,1.);
        assert_eq!(divine_rescues::saves(&r.player),5.);
    }

    #[test]
    fn full_heal_refills_before_next_damage_but_not_one_hp_rescues() {
        let mut r = room(0usize.into());
        r.player.abilities.push(OVERFLOWING.into());
        let id = r.player.id;
        engine::damage_environment(&mut r,id,1000.,None,"blood-weapon",true).unwrap();
        assert_eq!(divine_rescues::saves(&r.player),4.);
        r.player.hp = r.player.max_hp;
        engine::damage_environment(&mut r,id,1000.,None,"blood-weapon",true).unwrap();
        assert_eq!(divine_rescues::saves(&r.player),4.);

        r.player.max_hp = 1.;
        engine::damage_environment(&mut r,id,1000.,None,"blood-weapon",true).unwrap();
        assert_eq!(divine_rescues::saves(&r.player),3.);
        let saved = serde_json::to_string(&r).unwrap();
        let loaded = Engine::load(&saved).unwrap();
        assert_eq!(divine_rescues::saves(&loaded.run.player),3.);
    }

    #[test]
    fn fight_long_only_changes_its_own_melee_component() {
        for (marker, delta) in [(Field::Null,20.), (0usize.into(),3.)] {
            let mut r = room(marker);
            r.player.hp=50.;
            r.player.abilities.push(crate::divine_combat::FIGHT_LONG.into());
            let attacker=r.player.clone();
            crate::divine_combat::heal_melee_damage(&mut r,&attacker,100.,false);
            assert_eq!(r.player.hp,50.);
            crate::divine_combat::heal_melee_damage(&mut r,&attacker,100.,true);
            assert_eq!(r.player.hp,50.+delta);
        }
    }

    #[test]
    fn ursa_low_energy_threshold_and_reserved_dream_payment() {
        let mut r=room(0usize.into());
        let id = "ursa-dream-of-time-gone-by";
        r.player.abilities=["ursa-sleep-walk",id].map(str::to_owned).to_vec();
        for (e,yes) in [(0.,true),(9.99,true),(10.,false),(11.,false)] {
            r.player.energy=e;
            assert_eq!(crate::ai::is_hasted_in(&r,&r.player),yes);
        }
        r.player.abilities.push("grisha-boundless-hunger".into());
        r.player.energy=9.;
        assert!(!sleep_walk_active(&r,&r.player));
        r.player.abilities=vec![id.into()];
        r.player.energy=20.;
        crate::revision50_ability_energy::reconcile_acquired(&mut r).unwrap();
        let mut eng=Engine::new(r).unwrap();
        let cmd=Command::Ability{id:id.into(),target:Field::Missing,x:Field::Missing,y:Field::Missing};
        eng.dispatch(&cmd).unwrap();
        assert!(eng.run.player.active.contains(&id.into()));
        let energy=eng.run.player.energy;
        eng.dispatch(&cmd).unwrap();
        assert!(!eng.run.player.active.contains(&id.into()));
        assert_eq!(eng.run.player.energy,energy);
    }

    #[test]
    fn sudden_sleep_leaves_twenty_five_paralysis_ticks_and_restores_saves() {
        let mut r=room(0usize.into());
        r.player.abilities=[OVERFLOWING,crate::abilities::SLEEP].map(str::to_owned).to_vec();
        let id=r.player.id;
        engine::damage_environment(&mut r,id,1000.,None,"blood-weapon",true).unwrap();
        assert_eq!(divine_rescues::saves(&r.player),4.);
        crate::revision50_ability_energy::reconcile_acquired(&mut r).unwrap();
        let mut eng=Engine::new(r).unwrap();
        eng.dispatch(&Command::Ability{id:crate::abilities::SLEEP.into(),target:Field::Missing,x:Field::Missing,y:Field::Missing}).unwrap();
        assert_eq!(engine::condition(&eng.run.player,"paralyzed"),25.);
        assert_eq!(divine_rescues::saves(&eng.run.player),5.);
    }

    #[test]
    fn replay_marker_preserves_old_activation_cost_and_live_adopts_at_eof() {
        let mut r=room(Field::Missing);
        let id="ursa-dream-of-time-gone-by";
        r.player.abilities=vec![id.into()];
        r.player.energy=100.;
        crate::revision50_ability_energy::reconcile_acquired(&mut r).unwrap();
        let mut legacy=Engine::new_for_replay(r.clone()).unwrap();
        let cmd=Command::Ability{id:id.into(),target:Field::Missing,x:Field::Missing,y:Field::Missing};
        legacy.dispatch(&cmd).unwrap();
        let record=crate::replay::ReplayRecord::try_from_run(&legacy.run).unwrap();
        assert!(record.balance_rules_from.is_missing());
        let mut session=crate::replay::ReplaySession::from_initial(r,record).unwrap();
        session.step().unwrap();
        assert_eq!(session.engine.save().unwrap(),legacy.save().unwrap());
        let live=Engine::load(&legacy.save().unwrap()).unwrap();
        assert_eq!(live.run.balance_rules_from,1usize.into());
        assert_eq!(dream_anchor_cost(&live.run),20.);
    }
}
