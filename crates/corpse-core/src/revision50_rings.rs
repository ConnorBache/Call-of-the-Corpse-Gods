//! Authored fixed-power Revision 50 rings. Item.effect is the identity, not its
//! display name, enchantment or Tinker grants. No tiers or loot rates are inferred.
use crate::{Actor, Item, Run};

pub const STEALTH: &str = "stealth";
pub const LIGHT: &str = "light";
pub const REGENERATION: &str = "regeneration";
pub const VAMPIRISM: &str = "vampirism";
/// Source compatibility for pre-rename draft callers, not a second power.
pub const TRANSFERENCE: &str = VAMPIRISM;
pub const REAPING: &str = "reaping";
pub const FLIGHT: &str = "flight";
pub const FIRE_RESISTANCE: &str = "fire-resistance";
pub const POISON_RESISTANCE: &str = "poison-resistance";
pub const SUSTAIN_STRENGTH: &str = "sustain-strength";
pub const WILLPOWER: &str = "willpower";
pub const PROTECTION: &str = "protection";
pub const SLAYING: &str = "slaying";
pub const ICE_IMMUNITY: &str = "ice-immunity";
pub const CORROSION_RESISTANCE: &str = "corrosion-resistance";
/// Swimming is ordinary loot under current rules and remains an island safeguard.
pub const SWIMMING: &str = "swimming";

/// Removed powers retain their serialized identity and inventory slot.
pub fn removed_effect(effect: &str) -> bool {
    matches!(effect, FIRE_RESISTANCE | POISON_RESISTANCE | SUSTAIN_STRENGTH | ICE_IMMUNITY | CORROSION_RESISTANCE)
}
pub fn retired(r: &Run, item: &Item) -> bool {
    crate::ring_rules::current(r) && effect(item).is_some_and(removed_effect)
}
pub fn description_for_run(r: &Run, item: &Item) -> Option<&'static str> {
    if retired(r, item) { return Some("Retired ring."); }
    if crate::ring_rules::current(r) && effect(item) == Some(WILLPOWER) {
        return Some("Halves the duration of all status effects.");
    }
    description(item)
}

/// Canonical named-ring identity, independent of wearing or run revision.
/// Only these explicit powers have been authored. Unknown/legacy effects are not
/// promoted into named rings by matching a display name or a granted ability.
pub fn effect(item: &Item) -> Option<&'static str> {
    if item.kind != "ring" { return None; }
    match item.effect.as_ref().map(String::as_str) {
        Some(STEALTH) => Some(STEALTH),
        Some(LIGHT) => Some(LIGHT),
        Some(REGENERATION) => Some(REGENERATION),
        Some(VAMPIRISM | "transference") => Some(VAMPIRISM),
        Some(REAPING) => Some(REAPING),
        Some(FLIGHT) => Some(FLIGHT),
        Some(FIRE_RESISTANCE) => Some(FIRE_RESISTANCE),
        Some(POISON_RESISTANCE) => Some(POISON_RESISTANCE),
        Some(SUSTAIN_STRENGTH) => Some(SUSTAIN_STRENGTH),
        Some(WILLPOWER) => Some(WILLPOWER),
        Some(PROTECTION) => Some(PROTECTION),
        Some(SLAYING) => Some(SLAYING),
        Some(ICE_IMMUNITY) => Some(ICE_IMMUNITY),
        Some(CORROSION_RESISTANCE) => Some(CORROSION_RESISTANCE),
        Some(SWIMMING) => Some(SWIMMING),
        _ => None,
    }
}

/// Canonical display name overrides stale draft names without mutating saves.
pub fn display_name(item: &Item) -> Option<&'static str> {
    Some(match effect(item)? {
        STEALTH => "Ring of Stealth",
        LIGHT => "Ring of Light",
        REGENERATION => "Ring of Regeneration",
        VAMPIRISM => "Ring of Vampirism",
        FLIGHT => "Ring of Flight",
        FIRE_RESISTANCE => "Ring of Fire Resistance",
        POISON_RESISTANCE => "Ring of Poison Resistance",
        CORROSION_RESISTANCE => "Ring of Corrosion Resistance",
        ICE_IMMUNITY => "Ring of Ice Immunity",
        SUSTAIN_STRENGTH => "Ring of Sustain Strength",
        WILLPOWER => "Ring of Willpower",
        PROTECTION => "Ring of Protection",
        SLAYING => "Ring of Slaying",
        REAPING => "Ring of Reaping",
        SWIMMING => "Ring of Swimming",
        _ => return None,
    })
}

/// Authored pack text; callers decide whether the R50 item view is active.
pub fn description(item: &Item) -> Option<&'static str> {
    match effect(item)? {
        STEALTH => Some("Negative illumination on your tile is multiplied by 1.2."),
        LIGHT => Some("Adds 10 illumination to surrounding tiles in your line of sight."),
        REGENERATION => Some("Doubles your natural HP regeneration."),
        REAPING => Some("Restores 1 energy to each amulet for every 20 direct damage you deal to enemies."),
        VAMPIRISM => Some("Melee attacks heal you for 5% of the damage dealt."),
        FLIGHT => Some("Grants flight."),
        FIRE_RESISTANCE => Some("Halves fire and burning damage."),
        POISON_RESISTANCE => Some("Halves poison and venom damage."),
        SUSTAIN_STRENGTH => Some("Halves all temporary reductions to your stats."),
        WILLPOWER => Some("Halves the duration of confusion, hallucination, and enemy curses."),
        PROTECTION => Some("Grants 4 DR."),
        SLAYING => Some("Grants +4 Damage."),
        ICE_IMMUNITY => Some("Prevents cold damage, cold-induced slowing, and slipping on ice."),
        CORROSION_RESISTANCE => Some("Halves acid, caustic gas, and ichor damage, and corrosion penalties to DR."),
        SWIMMING => Some("Grants haste while in shallow or deep water, but not while flying above it."),
        _ => None,
    }
}

/// Is an authored named-ring power carried by this equipment owner?
///
/// Numeric powers stack per carried copy. Inherently binary powers (flight,
/// ice immunity, swimming haste) use presence, without inventing extra effects.
/// This read-only query never removes items or their separate Tinker grants,
/// legacy stat bonuses or enhancements. Half Here/dream-body endpoints share
/// their owner's equipment; stats-only Mini-Me and decoys inherit none.
///
/// O(I + C) worst-case time, O(1) extra memory: I owner inventory entries,
/// C actor conditions inspected for ownership/clone exclusion. No RNG or cache.
pub fn has_effect(r: &Run, actor: &Actor, named_effect: &str) -> bool {
    count_effect(r, actor, named_effect) > 0
}

/// Count distinct carried items, not quantity, with the same ownership/exclusions.
/// O(I + C) worst-case time, O(1) additional memory. Quantity is not a ring count.
pub fn count_effect(r: &Run, actor: &Actor, named_effect: &str) -> usize {
    if !crate::revision50::enabled(r)
        || (crate::ring_rules::current(r) && removed_effect(named_effect))
        || !matches!(named_effect, STEALTH | LIGHT | REGENERATION | TRANSFERENCE | FLIGHT
            | FIRE_RESISTANCE | POISON_RESISTANCE | CORROSION_RESISTANCE | ICE_IMMUNITY
            | WILLPOWER | PROTECTION | SLAYING | REAPING | SUSTAIN_STRENGTH | SWIMMING)
        || (named_effect == SWIMMING && r.simulation_revision.value_or(29.) < 52.)
        || actor.dead.value_or(false)
        || crate::revision50_nightmares::is_decoy(actor)
        || actor.conditions.get("miniMe") == Some(&1.0)
    {
        return 0;
    }
    let owner = if crate::half_here::owner_id(r, actor) == r.player.id {
        &r.player
    } else {
        actor
    };
    if owner.dead.value_or(false) { return 0; }
    crate::actor_equipment::inventory(r, owner).iter().filter(|item| {
        crate::revision50_inventory::worn(item) && effect(item) == Some(named_effect)
    }).count()
}

/// Natural HP fraction per normal world turn, before existing creature/Drown
/// multipliers and suppression. Never advances time or regenerates any energy.
pub fn natural_hp_rate(r: &Run, actor: &Actor) -> f64 {
    0.002 * stack_multiplier(r, actor, REGENERATION, 2.0)
}

/// Compose a per-ring multiplier after one ownership-aware inventory scan.
/// O(I + C) time, O(1) memory; no RNG, save state, or per-copy effects loop.
pub fn stack_multiplier(r: &Run, actor: &Actor, effect: &str, per_copy: f64) -> f64 {
    per_copy.powf(stacking_count(r, actor, effect) as f64)
}

/// Apply after scaling; combat rounds the result for revision 51 and later.
/// Legacy packets retain fractional resistance. Status duration is unchanged.
/// Unknown damage kinds receive no resistance. Cold immunity uses the same
/// ownership gate; temporary acid penalties are handled by their status source.
pub fn resist_damage(r: &Run, actor: &Actor, kind: &str, damage: f64) -> f64 {
    let effect = match kind {
        "fire" | "steam" => FIRE_RESISTANCE,
        "poison" | "venom" => POISON_RESISTANCE,
        "caustic" | "ichor" | "breatherGas" | crate::death_gas_rules::GAS | "acid" => CORROSION_RESISTANCE,
        "cold" => return if has_effect(r, actor, ICE_IMMUNITY) { 0.0 } else { damage },
        _ => return damage,
    };
    damage * stack_multiplier(r, actor, effect, 0.5)
}

/// Preserve recorded pre-R52 fixed-power behavior; Light already stacked.
pub fn stacking_count(r: &Run, actor: &Actor, effect: &str) -> usize {
    let count = count_effect(r, actor, effect);
    if r.simulation_revision.value_or(29.) < 52. && effect != LIGHT { count.min(1) } else { count }
}
