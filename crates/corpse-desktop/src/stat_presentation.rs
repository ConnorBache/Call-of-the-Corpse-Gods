//! Display units only. Save fields and simulation arithmetic remain unchanged.
use corpse_core::Actor;
/// Round rendered quantities only, avoiding fractional noise and negative zero.
/// O(1) numeric work plus output digits; simulation values are never changed.
pub fn whole_number(value: f64) -> String {
    let rounded = value.round();
    if rounded == 0. { "0".into() } else { format!("{rounded:.0}") }
}

/// Current HP labels use whole units without depicting a living fraction as zero.
/// O(1), display only: exact HP, bar proportions and death thresholds are untouched.
pub fn current_health(hp: f64) -> f64 { hp.ceil().max(0.) }
/// O(1). Input is already rounded by its mechanic; never round before conversion.
/// Damage is the player's nominal melee bonus, not final damage or a ranged bonus.
pub fn bonus(key: &str, value: f64) -> (&str, f64) {
    match key {
        "offense" | "Offense" => ("Damage", value),
        "defense" | "Defense" => ("HP", value * 6.),
        _ => (key, value),
    }
}
/// Signed nominal Damage display only; preserve fractional simulation bonuses.
pub fn damage_bonus(run: &corpse_core::Run, value: f64) -> f64 {
    if corpse_core::combat::nearest_damage_enabled(run) { corpse_core::combat::integer_stat(value) }
    else { value }
}
/// Current-revision ordinary enemy strike, before variance, DR and abilities.
/// Actor-specific weapon coefficients matter; never use the player bonus scale.
pub fn enemy_damage(actor: &Actor) -> f64 {
    enemy_damage_revision(actor, 52.)
}
/// O(1) display conversion, using the originating run's rounding policy.
/// R51+ rounds to nearest; legacy profiles retain ceiling. No Actor/Run mutation.
pub fn enemy_damage_revision(actor: &Actor, revision: f64) -> f64 {
    let amount = corpse_core::combat::creature_attack_base(revision, actor, actor.stats.offense);
    if corpse_core::combat::raw_attack_damage_enabled(revision) {
        corpse_core::combat::nearest_damage(amount)
    } else if revision >= 51. {
        corpse_core::combat::nearest_damage(amount * 0.5)
    } else {
        corpse_core::combat::tuned_damage(amount, Some("attack"))
    }
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn bonuses_preserve_sign_and_convert_after_mechanical_rounding() {
        assert_eq!(bonus("offense", 4.), ("Damage", 4.));
        assert_eq!(bonus("defense", -3.), ("HP", -18.));
        assert_eq!(bonus("defense", 0.5), ("HP", 3.));
        assert_eq!(bonus("dr", 4.), ("dr", 4.));
    }
    #[test] fn enemy_damage_uses_its_own_attack_coefficients() {
        let mut a=Actor::default();a.stats.offense=4.;
        assert_eq!(enemy_damage(&a),7.);
        a.conditions.insert("weaponDamage".into(),45.);
        a.conditions.insert("weaponOffenseMultiplier".into(),2.);
        assert_eq!(enemy_damage(&a),53.);
    }
}

#[cfg(test)]
mod revision51_hud_tests {
    #[test]
    fn revision51_hud_damage_rounds_signed_bonus_without_altering_stats() {
        let mut r:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=51.0.into();r.player.stats.offense=0.;r.player.conditions.clear();r.player.abilities.clear();r.player.active.clear();r.inventory.clear();
        r.player.conditions.insert("buff:offense".into(),-3.);r.player.conditions.insert("buffTicks:offense".into(),5.);
        r.inventory.push(corpse_core::Item{id:800.,name:"Ring of Sustain Strength".into(),kind:"ring".into(),effect:corpse_core::revision50_rings::SUSTAIN_STRENGTH.to_string().into(),..Default::default()});
        let before=r.clone();assert_eq!(corpse_core::combat::effective_stats(&r,&r.player).offense,-1.5);
        assert_eq!(crate::observe::observe(&r).player_stats[0],-2.);assert_eq!(r,before);
        r.simulation_revision=50.0.into();assert_eq!(crate::observe::observe(&r).player_stats[0],-1.5);
    }
}

#[cfg(test)]
mod revision52_bestiary_damage_tests {
    use super::*;
    #[test]
    fn enemy_damage_revision_keeps_legacy_rounding_and_current_coefficients() {
        let mut actor = Actor::default();
        actor.stats.offense = 2.2;
        let before = actor.clone();
        for revision in [29., 33., 42., 50.] {
            assert_eq!(enemy_damage_revision(&actor, revision), 7.);
        }
        for revision in [51., 52.] {
            assert_eq!(enemy_damage_revision(&actor, revision), 6.);
        }
        assert_eq!(actor, before);
        actor.conditions.insert("weaponDamage".into(), 45.);
        actor.conditions.insert("weaponOffenseMultiplier".into(), 2.);
        actor.stats.offense = 4.;
        assert_eq!(enemy_damage_revision(&actor, 50.), 27.);
        assert_eq!(enemy_damage_revision(&actor, 52.), 53.);
        actor.stats.offense = -100.;
        for revision in [29., 50., 51., 52.] {
            assert_eq!(enemy_damage_revision(&actor, revision), 0.);
        }
    }
}

#[cfg(test)]
mod zero_health_display_tests {
    #[test]
    fn whole_health_never_rounds_a_living_fraction_to_zero() {
        for hp in [f64::MIN_POSITIVE, 0.01, 0.25, 0.4, 0.5, 1.] {
            assert_eq!(super::current_health(hp), 1.);
        }
        for hp in [-10., -0.25, -0., 0.] { assert_eq!(super::current_health(hp), 0.); }
        assert_eq!(super::current_health(1.25), 2.);
    }
}

#[cfg(test)] mod integer_quantity_tests {
    #[test] fn display_rounds_signed_quantities_without_binary_noise() {
        for (value,expected) in [(2.49,"2"),(2.5,"3"),(-2.5,"-3"),(-0.01,"0"),(1e9,"1000000000")] {
            assert_eq!(super::whole_number(value),expected);
        }
    }
}
