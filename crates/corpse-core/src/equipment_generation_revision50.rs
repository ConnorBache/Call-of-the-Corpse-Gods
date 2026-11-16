//! Fixed named powers for the first R50 build, with no jewelry quality tiers.
//! Existing ordinary group weights are filtered to useful permanent kinds.
use crate::{Item, rng::GenerationRng};
use super::pick;

pub const KINDS: [&str; 3] = ["weapon", "ring", "amulet"];
pub const JEWELRY_KINDS: [&str; 2] = ["ring", "amulet"];
pub const RINGS: [&str; 14] = [
    "stealth", "light", "regeneration", "vampirism", "reaping", "flight",
    "fire-resistance", "poison-resistance", "sustain-strength", "willpower",
    "protection", "slaying", "ice-immunity", "corrosion-resistance",
];
pub const AMULETS: [&str; 12] = [
    "lightning", "firebolt", "poison", "iceblast", "acid", "blinking",
    "beckoning", "discord", "obstruction", "roots", "teleportation", "floods",
];
/// One uniform draw within the requested group; no depth/tier/stat roll.
/// O(1) lookup and O(item bytes) allocation. The returned ID is supplied by caller.
pub fn create(kind: &str, rng: &mut GenerationRng, id: f64, revision: f64) -> Result<Item, String> {
    match kind {
        "ring" => {
            let mut item = Item { id, kind: "ring".into(), glyph: "=".into(),
                effect: (*pick(&RINGS, rng)).to_string().into(), identified: true.into(),
                ..Default::default() };
            item.name = crate::revision50_rings::display_name(&item)
                .ok_or("Missing authored ring name")?.into();
            Ok(item)
        }
        "amulet" => make_amulet(pick(&AMULETS, rng), id, revision),
        // Bow remains the starting bow; Composite bow was the only non-melee
        // ordinary weapon in the existing drop pool. No unusable equip bonuses.
        "weapon" => crate::equipment::make_equipment("Composite bow", id).map_err(|e| e.0),
        _ => Err(format!("No Revision 50 permanent group {kind}")),
    }
}
fn make_amulet(effect: &str, id: f64, revision: f64) -> Result<Item, String> {
    use crate::revision50_amulet_effects::{Effect, make_amulet_for_revision};
    let projectile = match effect {
        "lightning" => Some(Effect::Lightning), "firebolt" => Some(Effect::Firebolt),
        "poison" => Some(Effect::Poison), "iceblast" => Some(Effect::Iceblast),
        "acid" => Some(Effect::Acid), "blinking" => Some(Effect::Blinking), _ => None,
    };
    if let Some(kind) = projectile {
        return make_amulet_for_revision(kind, id, revision).ok_or_else(|| "Invalid generated amulet".into());
    }
    let mut item = Item { id, kind: "amulet".into(), glyph: "\"".into(),
        effect: effect.to_string().into(), identified: true.into(), ..Default::default() };
    item.name = crate::revision50_control_amulets::display_name(&item)
        .ok_or("Missing authored amulet name")?.into();
    crate::revision50_amulets::initialize_item_for_revision(&mut item, revision)
        .map_err(|e| format!("Invalid generated amulet state: {e:?}"))?;
    Ok(item)
}
