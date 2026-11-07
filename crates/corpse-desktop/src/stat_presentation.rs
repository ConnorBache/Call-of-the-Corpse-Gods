//! Display units only. Save fields and simulation arithmetic remain unchanged.
use corpse_core::Actor;
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
/// Ordinary enemy strike, before variance, DR and abilities, displayed to the nearest whole point.
/// Actor-specific weapon coefficients matter; do not use the player bonus scale.
pub fn enemy_damage(actor: &Actor) -> f64 {
    corpse_core::combat::nearest_damage((actor.conditions.get("weaponDamage").copied().unwrap_or(10.)
        + actor.stats.offense * actor.conditions.get("weaponOffenseMultiplier").copied().unwrap_or(1.)) * 0.5)
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
        assert_eq!(enemy_damage(&a),27.);
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
