//! Revisioned Ichor-fish family immunity, including current imported actors.
use crate::Actor;

/// Fixed identity set: O(L) identity bytes, O(1) scratch, no catalog scan or RNG.
/// A saved kind is authoritative; only kind-less actors use their display name.
pub fn immune(revision: f64, actor: &Actor) -> bool {
    if revision < 51. { return false; }
    let identity = actor.creature_kind.as_ref().map(String::as_str)
        .unwrap_or_else(|| crate::enemy_traits::base_name(&actor.name));
    matches!(identity,
        "lesser-ichor-fish" | "ichor-fish-standard" | "greater-ichor-fish"
        | "lesser-ichor-shark" | "ichor-shark" | "greater-ichor-shark"
        | "lesser-ichor-leviathan" | "ichor-leviathan" | "greater-ichor-leviathan"
        | "ichor-fish" | "Ichor Fish"
        | "Lesser Ichor-Fish" | "Ichor-Fish" | "Greater Ichor-Fish"
        | "Lesser Ichor-Shark" | "Ichor Shark" | "Greater Ichor Shark"
        | "Lesser Ichor Leviathan" | "Ichor Leviathan" | "Greater Ichor Leviathan")
}

/// Clear an inherited burning status before emission, damage or fresh ignition.
/// Three ordered-map removals are O(C) for C conditions; no new actor scan.
pub fn extinguish(revision: f64, actor: &mut Actor) -> bool {
    if !immune(revision, actor) { return false; }
    for key in ["burning", "burningOwner", "burningPlayer"] {
        actor.conditions.shift_remove(key);
    }
    true
}
