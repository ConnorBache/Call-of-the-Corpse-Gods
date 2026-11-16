//! R52 species-fixed birth profiles. Saved actors are never retuned.
//! Authored traits, hearts, population recipes and RNG are deliberately independent.
use crate::Actor;
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BirthProfile {
    pub kind: String,
    pub floor: u32,
    pub offense: f64,
    pub defense: f64,
    pub accuracy: f64,
    pub dodge: f64,
    pub dr: f64,
    #[serde(default)] pub gas_passive: Option<f64>,
    #[serde(default)] pub gas_boost: Option<f64>,
    #[serde(default)] pub gas_shot: Option<f64>,
    #[serde(default)] pub heal_amount: Option<f64>,
    #[serde(default)] pub special_damage: Option<f64>,
}

pub fn profiles() -> &'static [BirthProfile] {
    static ROWS: OnceLock<Vec<BirthProfile>> = OnceLock::new();
    ROWS.get_or_init(|| {
        serde_json::from_str(include_str!("catalog/species-fixed-v52.json"))
            .expect("validated R52 contextual birth profiles")
    })
}

/// Birth context is provenance, not a separate species. O(L), fixed prefixes.
pub fn canonical_kind(kind: &str) -> &str {
    for prefix in ["nest:", "queen:", "witch:", "birther:", "greater-birther:"] {
        if let Some(kind) = kind.strip_prefix(prefix) { return kind; }
    }
    kind
}
/// O(R*L), fixed catalog. Depth validates the caller but never scales a species.
/// The row's floor records its introduction/reference, not an alternate tier.
pub fn profile(kind: &str, depth: f64) -> Option<&'static BirthProfile> {
    if !depth.is_finite() { return None; }
    let kind = canonical_kind(kind);
    profiles().iter().find(|row| row.kind == kind)
}

/// R52 new bodies keep one species Dodge across all depths and birth contexts.
/// Non-profiled special types retain their authored values.
/// O(R) fixed catalog scan, O(1) scratch, no RNG or saved-actor mutation.
pub fn birth_dodge_offset(kind: &str, revision: f64, fallback: f64) -> f64 {
    if revision == 52. { profile(kind, 1.).map_or(fallback, |row| row.dodge) }
    else { fallback }
}

/// Apply only at an explicit NEW actor construction boundary, after legacy overlays
/// and before wounds/growth/corpse admission. O(R) fixed lookup, O(1) writes, no RNG.
/// Integral Defense is essential: native health sync/revival derives 100+6*Defense.
/// No calls belong in load, save, resurrection, copying, travel or temporal restore.
pub fn apply_new_actor_kind(actor: &mut Actor, kind: &str, depth: f64, revision: f64) -> bool {
    if revision != 52. {
        return false;
    }
    let Some(row) = profile(kind, depth) else {
        return false;
    };
    // Reset context-only numeric payloads (not provenance, loyalty, loot or wounds).
    // Revision51 constructor is deliberately used to avoid this R52 overlay recursing.
    // The fixed legacy template carries the species' Perception and energy capacities.
    if let Ok(base) = crate::population::create_actor_revision(canonical_kind(kind), 0., 0., 0., false, 51.) {
        actor.stats = base.stats;
        actor.energy = base.energy;
        for key in ["monsterGasPassiveVolume", "monsterGasBoostVolume", "monsterGasShotVolume",
            "monsterHealAmount", "monsterSpecialDamage"] {
            actor.conditions.shift_remove(key);
            if let Some(value) = base.conditions.get(key) { actor.conditions.insert(key.into(), *value); }
        }
    }
    actor.stats.offense = row.offense;
    actor.stats.defense = row.defense;
    actor.stats.accuracy = crate::combat::creature_birth_accuracy(revision, row.accuracy);
    actor.stats.dodge = crate::combat::creature_birth_dodge(revision, birth_dodge_offset(kind, revision, row.dodge));
    actor.stats.dr = row.dr;
    actor.hp = 100. + 6. * row.defense;
    actor.max_hp = actor.hp;
    for (key, value) in [
        ("monsterGasPassiveVolume", row.gas_passive),
        ("monsterGasBoostVolume", row.gas_boost),
        ("monsterGasShotVolume", row.gas_shot),
        ("monsterHealAmount", row.heal_amount),
        ("monsterSpecialDamage", row.special_damage),
    ] {
        if let Some(value) = value { actor.conditions.insert(key.into(), value); }
    }
    true
}

/// Saved local spell magnitudes: O(1) lookup, no depth lookup or RNG at action time.
/// Old snapshots and all non-R52 runs keep their authored default.
pub fn local_amount(run: &crate::Run, actor: &Actor, key: &str, fallback: f64) -> f64 {
    if run.simulation_revision.value_or(29.) != 52. { return fallback; }
    actor.conditions.get(key).copied().filter(|v| valid_condition(key, *v)).unwrap_or(fallback)
}
pub fn condition_key(key: &str) -> bool {
    matches!(key, "monsterHealAmount" | "monsterSpecialDamage")
}
pub fn valid_condition(key: &str, value: f64) -> bool {
    condition_key(key) && value.is_finite() && value >= 0.
        && if key == "monsterSpecialDamage" { (value * 2.).is_finite() && (value * 2.).fract() == 0. } else { value.fract() == 0. }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_is_unique_integral_finite_and_has_positive_health() {
        let mut keys = std::collections::BTreeSet::new();
        assert!(!profiles().is_empty());
        for row in profiles() {
            assert!(keys.insert((&row.kind, row.floor)));
            assert!((1..=40).contains(&row.floor));
            for value in [row.offense, row.defense, row.accuracy, row.dodge, row.dr] {
                assert!(value.is_finite() && value.fract() == 0.);
            }
            assert!(100. + 6. * row.defense > 0.);
            for value in [row.gas_passive, row.gas_boost, row.gas_shot].into_iter().flatten() {
                assert!(value.is_finite() && value >= 0.);
            }
            if let Some(value)=row.heal_amount { assert!(valid_condition("monsterHealAmount",value)); }
            if let Some(value)=row.special_damage { assert!(valid_condition("monsterSpecialDamage",value)); }
        }
    }
    #[test]
    fn every_other_revision_and_unknown_context_is_exact_noop() {
        let mut actor = Actor::default();
        actor.hp = 17.;
        actor.max_hp = 90.;
        actor.stats.offense = 123.;
        let before = actor.clone();
        for revision in [
            29.,
            31.,
            32.,
            33.,
            41.,
            42.,
            46.,
            50.,
            51.,
            52.5,
            53.,
            f64::NAN,
        ] {
            assert!(!apply_new_actor_kind(&mut actor, "rat", 2., revision));
            assert_eq!(actor, before);
        }
        for kind in ["unlisted-context:rat", "unknown"] {
            assert!(!apply_new_actor_kind(&mut actor, kind, 20., 52.));
            assert_eq!(actor, before);
        }
        assert!(profile("rat", f64::NAN).is_none());
    }
    #[test]
    fn every_depth_uses_one_species_tier_not_unrelated_species() {
        let first = profiles()
            .iter()
            .filter(|r| r.kind == "rat")
            .min_by_key(|r| r.floor)
            .unwrap();
        let last = profiles()
            .iter()
            .filter(|r| r.kind == "rat")
            .max_by_key(|r| r.floor)
            .unwrap();
        assert_eq!(profile("rat", -1.).unwrap().floor, first.floor);
        assert_eq!(profile("rat", 40.).unwrap().floor, last.floor);
        assert_eq!(profile("ursa-major", 1.).unwrap().floor, 36);
    }
}
