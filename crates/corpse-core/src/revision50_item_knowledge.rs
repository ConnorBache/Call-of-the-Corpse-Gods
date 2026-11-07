//! Read-only Revision 50 item knowledge. Saved discovery fields are retained for
//! old saves, but never gate R50 names or effects and are not changed by hovering.
use crate::{Item, Run};

/// Fixed authored lookup: O(effect bytes), constant scratch.
pub fn potion_name(effect: &str) -> Option<&'static str> {
    Some(match effect {
        "healing" => "Potion of Healing",
        "energy" => "Potion of Energy",
        "incineration" => "Potion of Incineration",
        "caustic gas" => "Potion of Caustic Gas",
        "paralysis" => "Potion of Paralysis",
        "confusion" => "Potion of Confusion",
        "telepathy" => "Potion of Telepathy",
        "levitation" => "Potion of Levitation",
        "speed" => "Potion of Speed",
        "invisibility" => "Potion of Invisibility",
        "descent" => "Potion of Descent",
        "cleansing" => "Potion of Cleansing",
        "fire immunity" => "Potion of Fire Immunity",
        "darkness" => "Potion of Darkness",
        "flooding" => "Potion of Flooding",
        "freezing" => "Potion of Freezing",
        "repulsion" => "Potion of Repulsion",
        "discord" => "Potion of Discord",
        "teleportation" => "Potion of Teleportation",
        "shattering" => "Potion of Shattering",
        "Veridian Sap" => "Verdania Sap",
        _ => return None,
    })
}

/// O(item identity bytes + output bytes), no actor, visibility, RNG or world query.
/// Callers must first admit the item through their own inventory/ground knowledge.
pub fn display_name(r: &Run, item: &Item) -> String {
    if !crate::revision50::enabled(r) { return item.name.clone(); }
    if crate::ahvamitt::is_mask(item) { return crate::ahvamitt::MASQUE_NAME.into(); }
    if let Some(name) = crate::revision50_rings::display_name(item)
        .or_else(|| crate::revision50_amulet_effects::display_name(item)) {
        return name.into();
    }
    if item.kind == "potion" {
        if let Some(name) = item.effect.as_ref().and_then(|effect| potion_name(effect)) {
            return format!("{name}{}", if item.diluted.value_or(false) { " (diluted)" } else { "" });
        }
    }
    crate::revision50_creature_identity::display_name(&item.name).into_owned()
}

/// Authored default copy with existing half-dose scalar/duration values; spatial
/// footprint rules remain fixed unless the effect explicitly scales them.
/// O(effect bytes + output bytes), constant scratch beyond the returned string.
pub fn potion_effect_description(effect: &str, diluted: bool) -> Option<String> {
    let strength = if diluted { 0.5 } else { 1.0 };
    let turns = |normal: f64| (normal * strength).ceil();
    Some(match effect {
        "healing" => format!("Restores {} HP, up to maximum HP.", 50.0 * strength),
        "energy" => format!("Restores {} energy, up to maximum energy.", 50.0 * strength),
        "incineration" => "Ignites the impact square and sets creatures there on fire.".into(),
        "caustic gas" => "Releases a cloud of caustic gas at the impact location.".into(),
        "paralysis" => format!("Releases paralysis gas and paralyzes an affected creature for {} turns.", turns(5.0)),
        "confusion" => "Releases a cloud of confusion gas at the impact location.".into(),
        "telepathy" => format!("Reveals creatures throughout the floor for {} turns.", turns(30.0)),
        "levitation" => format!("Grants flight for {} turns.", turns(20.0)),
        "speed" => format!("Grants haste for {} turns.", turns(10.0)),
        "invisibility" => format!("Grants invisibility for {} turns.", turns(20.0)),
        "descent" => "Collapses the ground around the impact location.".into(),
        "cleansing" => "Removes harmful temporary effects from the affected creature, leaving beneficial effects intact.".into(),
        "fire immunity" => format!("Grants immunity to fire for {} turns.", turns(20.0)),
        "darkness" => format!("Reduces illumination by {} in a 5×5 area for {} turns.", 50.0 * strength, turns(20.0)),
        "flooding" => format!("Turns a 3×3 area into shallow water for {} turns.", turns(20.0)),
        "freezing" => format!("Freezes water into ice and slows creatures in a 3×3 area for {} turns, and the ice remains until melted.", turns(3.0)),
        "repulsion" => format!("Pushes creatures within 3 tiles of the impact location up to {} tiles away.", turns(3.0)),
        "discord" => format!("Makes an affected creature attack other creatures regardless of allegiance for {} turns.", turns(10.0)),
        "teleportation" => "Teleports the affected creature to a random valid square on the current floor.".into(),
        "shattering" => "Destroys destructible walls and obstacles within 2 tiles of the impact location.".into(),
        "Veridian Sap" if diluted => "Restores half your missing health and energy and grants half the blessings still needed for your next level, rounded up.".into(),
        "Veridian Sap" => "Fully restores health and energy and grants the blessings still needed for your next level.".into(),
        _ => return None,
    })
}

pub fn potion_description(item: &Item) -> String {
    let effect = item.effect.as_ref().map_or("", String::as_str);
    let name: &str = match potion_name(effect) { Some(name) => name, None => &item.name };
    let description = potion_effect_description(effect, item.diluted.value_or(false))
        .unwrap_or_else(|| "This potion's effect is unavailable in this revision.".into());
    format!("{name}{}\n{description}", if item.diluted.value_or(false) { " (diluted)" } else { "" })
}
