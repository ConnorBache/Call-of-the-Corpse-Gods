//! Exact v48 corpse and burning helper semantics. Scheduler owns revival gates.
use crate::{Actor, Field};
pub fn has_permanent_burning(a: &Actor) -> bool {
    a.abilities.iter().any(|x| x == "enemy:beast:everburn")
        && a.abilities.iter().any(|x| x == "enemy:beast:fire-immunity")
}
pub fn is_bastidion(a: &Actor) -> bool {
    let name = a
        .name
        .strip_prefix("Mercuria’s ")
        .or_else(|| a.name.strip_prefix("Mercuria's "))
        .unwrap_or(&a.name);
    matches!(name, "Bastidion" | "Vent Fungus" | "Corpse Fungus")
}
pub fn can_revive_corpse(a: &Actor) -> bool {
    !is_bastidion(a) && !a.abilities.iter().any(|x| x == "enemy:beast:no-revival")
}
pub fn corpse_revival_chance(age: f64) -> f64 {
    if age <= 80. {
        0.
    } else {
        ((1. + ((age - 81.) / 3.).floor()) / 100.).min(1.)
    }
}
pub fn reset_corpse_revival(a: &mut Actor) {
    a.corpse_age = Field::Value(0.);
    a.conditions.shift_remove("monsterPhoenixRolls");
}
pub fn corpse_roll_chance(a: &Actor) -> f64 {
    let age = a.corpse_age.value_or(0.);
    if age <= 80. {
        return 0.;
    }
    if a.abilities.iter().any(|x| x == "enemy:beast:phoenix") {
        (0.01
            * 2f64.powf(
                a.conditions
                    .get("monsterPhoenixRolls")
                    .copied()
                    .unwrap_or(0.)
                    .min(7.),
            ))
        .min(1.)
    } else {
        corpse_revival_chance(age)
    }
}
/// Exactly one sample even at certainty; no sample for ineligible corpses.
pub fn roll_corpse_revival(a: &mut Actor, random: impl FnOnce() -> f64) -> bool {
    if !a.dead.value_or(false) || !can_revive_corpse(a) {
        return false;
    }
    let chance = corpse_roll_chance(a);
    if chance <= 0. {
        return false;
    }
    let success = random() < chance;
    if a.abilities.iter().any(|x| x == "enemy:beast:phoenix") {
        let n = (a
            .conditions
            .get("monsterPhoenixRolls")
            .copied()
            .unwrap_or(0.)
            + 1.)
            .min(7.);
        a.conditions.insert("monsterPhoenixRolls".into(), n);
    }
    success
}
pub fn advance_corpse_burning(a: &mut Actor, elapsed: f64) {
    if !a.dead.value_or(false) {
        return;
    }
    if let Some(b) = a.conditions.get_mut("burning") {
        if *b > 0. {
            *b = (*b - elapsed).max(0.)
        }
    }
    if a.conditions.get("burning").copied().unwrap_or(0.) <= 0. {
        a.conditions.shift_remove("burningOwner");
        a.conditions.shift_remove("burningPlayer");
    }
}
/// Revisioned ignition keeps the public frozen helper and legacy samples intact.
pub fn apply_burning_revision(
    a: &mut Actor,
    revision: f64,
    random: f64,
    owner: Option<f64>,
    player: Option<f64>,
    strength: Option<f64>,
) -> f64 {
    if crate::revision51_ichor_fire::extinguish(revision, a) { return 0.; }
    apply_burning(a, random, owner, player, strength)
}
pub fn apply_burning(
    a: &mut Actor,
    random: f64,
    owner: Option<f64>,
    player: Option<f64>,
    strength: Option<f64>,
) -> f64 {
    if crate::revision50_potions::fire_immune(a) {return 0.;}
    let duration = ((6. + (random * 5.).floor()) * strength.unwrap_or(1.)).ceil();
    let old = a.conditions.get("burning").copied().unwrap_or(0.);
    a.conditions.insert("burning".into(), old.max(duration));
    if let Some(v) = owner {
        a.conditions.insert("burningOwner".into(), v);
    }
    if let Some(v) = player {
        let prior = a.conditions.get("burningPlayer").copied().unwrap_or(0.);
        a.conditions.insert("burningPlayer".into(), prior.max(v));
    }
    duration
}
pub fn revive_corpse(a: &mut Actor, tick: f64) {
    if !can_revive_corpse(a) {
        return;
    }
    let copy = a.id == 0.
        || a.conditions.get("miniMe") == Some(&1.)
        || a.conditions.get("halfHereClone") == Some(&1.);
    const KEEP: &[&str] = &[
        "paleTraitsInitialized",
        "bestiaryTraitsInitialized",
        "godbeastDepth",
        "ichorFishDepth",
        "fungalGasType",
        "enemyKind",
        "stealth",
        "weaponDamage",
        "offenseMultiplier",
        "tier",
        "maxAbilityCost",
        "monsterUrsaPartnerId",
        "monsterLinkedMechId",
        "monsterLinkedInventorId",
        "monsterWendigoLevel",
        "skinWalkerDisguise",
        "witchSummoned",
        "cornDimensionResident",
    ];
    a.conditions.retain(|k, _| KEEP.contains(&k.as_str()));
    a.dead = Field::Value(false);
    a.state = "wandering".into();
    a.perfect = Field::Value(false);
    a.order = Field::Missing;
    a.last_seen = Field::Missing;
    let round = |v: f64| (v + 0.5).floor();
    if !copy {
        a.max_hp = 100.
            + 6. * round(
                crate::combat::integer_stat(a.stats.defense)
                    + crate::combat::integer_stat(
                        crate::combat::integer_stat(a.stats.max_hp.value_or(0.)) / 6.,
                    ),
            );
        a.base_max_hp = Field::Missing;
    }
    a.hp = if a.abilities.iter().any(|x| x == "enemy:beast:phoenix") {
        a.max_hp
    } else {
        (a.max_hp / 2.).ceil()
    };
    if a.creature_kind.as_ref().is_some_and(|s| s == "banshee")
        || a.abilities.iter().any(|x| x == "enemy:beast:banshee")
    {
        a.conditions.insert("monsterBansheeRevivedAt".into(), tick);
    }
    if has_permanent_burning(a) {
        a.conditions.insert("burning".into(), 1.);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hazard_boundaries() {
        assert_eq!(corpse_revival_chance(80.), 0.);
        assert_eq!(corpse_revival_chance(81.), 0.01);
        assert_eq!(corpse_revival_chance(83.), 0.01);
        assert_eq!(corpse_revival_chance(84.), 0.02);
        assert_eq!(corpse_revival_chance(1000.), 1.);
    }
}
