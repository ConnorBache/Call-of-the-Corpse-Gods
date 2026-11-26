//! Full-page range copy only. Never imported by the creature hover/preview.
//! Geometry and admission audit: docs/revision52-bestiary-effect-ranges.md.
use super::{Entry, Mechanic};

/// O(M) fixed-name comparisons for M mechanics; output is borrowed static prose.
/// The caller lays it out with the existing page metrics, never edits the catalog.
pub(super) fn page_text<'a>(entry: &Entry, mechanic: &'a Mechanic) -> &'a str {
    match (entry.id.as_str(), mechanic.name.as_str()) {
        ("bastidion", "Fungal gas") => "Roots reach corpses up to 10 straight-line tiles away, with line of sight. No spatial shortcuts. Revival stops once roots reach the corpse. Gas spreads from the fungus and has no fixed radius.",
        ("dryad", "Healing Grove") => "Heals an injured ally up to 6 straight-line tiles away, including spatial shortcuts. Same plane and line of sight required. Cannot heal itself or a healing-suppressed ally. Successful casts have a 2-world-turn cooldown.",
        (id, "Ruin Pulse") if id.starts_with("zillennia:") => "Hits every other creature within 3 straight-line tiles, counting spatial shortcuts. Same plane and line of sight required; allies are also hit.",
        (id, "Mouth of Silence") if id.starts_with("zillennia:") => "Blocks new energy-paid activations within 3 straight-line tiles, counting spatial shortcuts. Enemies only; same plane and line of sight required.",
        (id, "Darkness Aura") if id.starts_with("zillennia:") => "Darkens ground fewer than 10 orthogonal steps away (diagonals count 2). Same plane; walls block it. No spatial shortcuts.",
        ("bombardier", "Reservoir Explosion") => "Empties all Energy in a blast up to 5 straight-line tiles away. Same plane and line of sight required; no spatial shortcuts. Ignites the area before blast damage. Fire and gas reactions can spread farther.",
        ("horn-5" | "horn-7", "Stone Seal") => "Targets prey up to 5 straight-line tiles away, including spatial shortcuts, on the same plane with line of sight. Raises one wall in the prey's 8 adjacent squares if at least 3 are free floor. Wall lasts 5 world turns; cooldown is 3 world turns.",
        ("pale:Screamer", "Scream") => "Within 10 straight-line tiles, including spatial shortcuts: foes lose 5 Accuracy to Disoriented for 5 turns; allies gain Damage and Perception for 5 turns. Same plane; walls do not block it. Calls same-side monsters across the whole floor.",
        ("pale:Greater Screamer", "Scream") => "Within 10 straight-line tiles, including spatial shortcuts: foes lose 10 Accuracy to Disoriented for 5 turns; allies gain Damage and Perception for 5 turns. Same plane; walls do not block it. Calls same-side monsters across the whole floor.",
        ("samara", "Razorleaves") => "Razorleaves cover squares up to 5 straight-line tiles away with line of sight; no spatial shortcuts. At normal turn end, exposed creatures on that plane lose 2% maximum HP. Creatures with Razorleaves are immune.",
        ("yeti", "Winter's Trail") => "Slows you if either body starts a turn within the 8 adjacent squares (diagonals included), also counting spatial shortcuts. Same plane; walls do not block it. Ice immunity protects that body.",
        ("banshee", "Banshee Shriek") => "Every 30 world turns, the white shriek crosses the whole floor, even through walls. Purple marks its line of sight. Creatures on the same plane in that line of sight take 100 damage plus half their current HP, ignoring armor. No distance cap.",
        _ => &mechanic.text,
    }
}

#[cfg(test)]
#[path = "manual_ranges_tests.rs"]
mod tests;
