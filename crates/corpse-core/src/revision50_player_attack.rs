//! Revision 50's unequipped player melee baseline, authored as a Greatsword.
//! R51 reduces fixed components; R52 retains the 20% innate increase while
//! authoring attack bases and Offense coefficients in actual pre-DR HP units.
//! Older revisions retain their arithmetic; no item is manufactured.
use crate::{combat, Run};

pub const DAMAGE: f64 = 45.0;
pub const OFFENSE_MULTIPLIER: f64 = 2.0;
pub const REVISION51_BASE_PERCENT: f64 = 70.0;

/// O(1), revision-aware fixed attack component; bonuses and rounding are separate.
pub fn fixed_damage_for_revision(revision: f64, damage: f64) -> f64 {
    // Percentage arithmetic preserves exact authored halves (R51 31.5; R52 15.75).
    if combat::raw_attack_damage_enabled(revision) { damage * 35.0 / 100.0 }
    else if revision >= 51.0 { damage * REVISION51_BASE_PERCENT / 100.0 } else { damage }
}

/// R52 keeps the innate 20% increase, authored at half raw strength:18.9.
/// R52 bows share this innate component; thrown items remain item-defined.
/// O(1), no RNG or saved-state mutation.
pub fn innate_damage_for_revision(revision: f64) -> f64 {
    let base = fixed_damage_for_revision(revision, DAMAGE);
    if revision >= 52.0 { base * 120.0 / 100.0 } else { base }
}

/// Already-derived Offense stays unchanged; R52 authors its attack coefficient as1.
pub fn with_offense_for_run(r: &Run, offense: f64) -> f64 {
    with_offense_for_revision(r.simulation_revision.value_or(29.0), offense)
}

/// Shared innate melee/bow formula, O(1), without item bonuses or early rounding.
pub fn with_offense_for_revision(revision: f64, offense: f64) -> f64 {
    innate_damage_for_revision(revision) + offense * if combat::raw_attack_damage_enabled(revision) { 1.0 } else { OFFENSE_MULTIPLIER }
}

/// Constant arithmetic on already-derived Offense. No rounding, RNG or procs.
pub fn with_offense(offense: f64) -> f64 {
    DAMAGE + OFFENSE_MULTIPLIER * offense
}

/// Current live player stats, without attack blessings, enchantment or offhand
/// strikes. Clay copies this base, not an activation-time attack snapshot.
/// Inherits effective_stats' actor/inventory/condition traversal; the additional
/// baseline calculation is O(1) time and space and never mutates the run.
pub fn base(r: &Run) -> f64 {
    with_offense_for_run(r, combat::effective_stats(r, &r.player).offense)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Item, PartialStats};

    #[test]
    fn baseline_matches_authored_greatsword_not_shortsword() {
        let catalog: serde_json::Value = serde_json::from_str(include_str!(
            "../catalog/equipment-v48.json"
        )).unwrap();
        assert_eq!(catalog["Greatsword"]["damage"].as_f64(), Some(DAMAGE));
        assert_eq!(catalog["Greatsword"]["offenseMultiplier"].as_f64(), Some(OFFENSE_MULTIPLIER));
        for (offense, expected) in [(0.0, 45.0), (5.0, 55.0), (25.0, 95.0), (100.0, 245.0)] {
            assert_eq!(with_offense(offense), expected);
        }
        // Penalties remain real; damage mitigation/rounding belongs to combat.
        assert_eq!(with_offense(-5.0), 35.0);
        assert_eq!(with_offense(1.5), 48.0);
    }

    #[test]
    fn base_reads_live_effective_offense_without_attack_side_effects() {
        let mut r = Run::default();
        r.simulation_revision = 50.0.into();
        r.player.stats.offense = 12.5;
        r.player.conditions.insert("buff:offense".into(), 2.5);
        r.inventory.push(Item {
            id: 100.0, kind: "ring".into(), equipped: "ring1".to_string().into(),
            bonus: PartialStats { offense: 4.5.into(), ..Default::default() }.into(),
            ..Default::default()
        });
        r.player.abilities = vec!["lichor-blood-weapon".into(), "lichor-the-number-of-blood".into(),
            "anavani-nova".into(), "etill-drown".into()];
        r.player.conditions.insert("bloodWeaponReady".into(), 1.0);
        r.player.active.push("anavani-nova".into());
        let before = serde_json::to_vec(&r).unwrap();
        assert_eq!(base(&r), 87.0); // integerStat(12.5) + 3 buff + 5 jewelry = 21.
        assert_eq!(serde_json::to_vec(&r).unwrap(), before);
        r.player.stats.offense = 20.0;
        assert_eq!(base(&r), 101.0);
        r.player.conditions.insert("weakened".into(), 1.0);
        assert_eq!(base(&r), 91.0);
    }

    #[test]
    fn explicit_bow_and_thrown_item_bases_stay_item_defined() {
        let bow = Item { kind: "weapon".into(), ranged_weapon: "bow".to_string().into(),
            damage: 15.0.into(), offense_multiplier: 0.5.into(), ..Default::default() };
        assert_eq!(crate::engine::player_weapon_strike_base_with_offense(&bow, 20.0), 25.0);
        let hatchet = Item { kind: "weapon".into(), damage: 30.0.into(), hands: 1.0.into(),
            thrown: true.into(), ..Default::default() };
        assert_eq!(crate::projectiles::player_throw_base_with_offense(&hatchet, 20.0), 50.0);
        assert_eq!(with_offense(20.0), 85.0);
    }
}
