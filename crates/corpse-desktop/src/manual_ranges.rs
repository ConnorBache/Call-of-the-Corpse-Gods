//! Full-page range copy only. Never imported by the creature hover/preview.
//! Geometry and admission audit: docs/revision52-bestiary-effect-ranges.md.
use super::{Entry, Mechanic};

/// O(M) fixed-name comparisons for M mechanics; output is borrowed static prose.
/// The caller lays it out with the existing page metrics, never edits the catalog.
pub(super) fn page_text<'a>(entry: &Entry, mechanic: &'a Mechanic) -> &'a str {
    range_override(entry, mechanic).unwrap_or(&mechanic.text)
}

fn range_override(entry: &Entry, mechanic: &Mechanic) -> Option<&'static str> {
    Some(match (entry.id.as_str(), mechanic.name.as_str()) {
        ("bastidion", "Fungal gas") => "Emits hallucinogenic gas. Roots grow 1 tile every 3 turns toward visible corpses within 10 tiles. Connected corpses increase the gas and stay dead.",
        ("dryad", "Healing Grove") => "Heals a visible wounded ally within 6 tiles; 2-turn cooldown.",
        (id, "Ruin Pulse") if id.starts_with("zillennia:") => "Hits friends and foes in sight within 3 tiles. 3-turn cooldown.",
        (id, "Mouth of Silence") if id.starts_with("zillennia:") => "Blocks new energy-paid abilities of foes in sight within 3 tiles.",
        (id, "Darkness Aura") if id.starts_with("zillennia:") => "Darkens ground within 9 orthogonal steps; walls block it.",
        ("bombardier", "Reservoir Explosion") => "Ignites the area, then hits creatures in sight within 5 tiles for 50% max HP. Fire and gas reactions can spread farther.",
        ("horn-5" | "horn-7", "Stone Seal") => "Raises a wall beside visible prey within 5 tiles if 3 adjacent squares are open. Lasts 5 turns; 3-turn cooldown.",
        ("pale:Screamer", "Scream") => "Screams with you within 10 tiles, even through walls: Disoriented (-5 Accuracy) to foes, +5 Damage and +20 Perception to allies for 5 turns. Wakes and rallies allies floorwide. Waits for the effects to end before screaming again.",
        ("pale:Greater Screamer", "Scream") => "Screams with you within 10 tiles, even through walls: Disoriented (-10 Accuracy) to foes, +10 Damage and +40 Perception to allies for 5 turns. Wakes and rallies allies floorwide. Waits for the effects to end before screaming again.",
        ("samara", "Razorleaves") => "Fills visible ground within 5 tiles with razorleaves: 2% max HP per turn inside. Immune to razorleaves.",
        ("yeti", "Winter's Trail") => "Leaves snow. Slows adjacent player bodies at turn start, including through walls; ice immunity protects.",
        ("banshee", "Banshee Shriek") => "Every 30 turns, a floorwide shriek reveals it. Creatures in sight take 50% current HP + 100 damage through armor.",
        _ => return None,
    })
}

#[cfg(test)]
#[path = "manual_ranges_tests.rs"]
mod tests;
