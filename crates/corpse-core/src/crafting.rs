//! Authored stat/capacity/healing crafts. Fury and Dream require separate combat/flight hooks.
use crate::{
    combat,
    engine::{EngineError, log},
    equipment, inventory,
    model::*,
};
pub fn craft_name(s: &str) -> Option<&'static str> {
    let resolved = crate::progression::catalog()
        .iter()
        .find(|a| a.id == s)
        .map(|a| a.name.as_str())
        .unwrap_or(s);
    for (name, ability) in [
        ("Fury", "Fury Crafting"),
        ("Zeal", "Zeal Craft"),
        ("Guile", "Guile Craft"),
        ("Dream", "Dream Craft"),
        ("Artifice", "Artifice Craft"),
    ] {
        if resolved == name
            || resolved == ability
            || resolved.to_lowercase() == format!("{} craft", name.to_lowercase())
            || s.ends_with(&format!("-{}-craft", name.to_lowercase()))
        {
            return Some(name);
        }
    }
    None
}
pub fn supported(s: &str) -> bool {
    matches!(
        craft_name(s),
        Some("Guile" | "Artifice" | "Zeal" | "Fury" | "Dream")
    )
}
pub fn is_equipped(i: &Item) -> bool {
    equipment::equippable(i)
        && i.equipped.as_ref().is_some_and(|s| !s.is_empty())
        && (!i
            .ranged_weapon
            .as_ref()
            .is_some_and(|s| matches!(s.as_str(), "bow" | "composite-bow"))
            || (i.hands.value_or(0.0) == 2.0 && i.equipped.as_ref().is_some_and(|s| s == "hand1")))
}
pub fn capacity(items: &[Item]) -> f64 {
    20.0 + 5.0
        * items
            .iter()
            .filter(|i| is_equipped(i))
            .flat_map(|i| &i.enhancements)
            .filter(|e| craft_name(&e.craft) == Some("Guile"))
            .count() as f64
}
fn owner_id(name: &str) -> &'static str {
    match name {
        "Fury" => "shrail-fury-crafting",
        "Dream" => "ursa-dream-craft",
        "Guile" => "isheilah-guile-craft",
        "Artifice" => "kative-artifice-craft",
        "Zeal" => "pelian-zeal-craft",
        _ => "",
    }
}
/// O(I+A+E); whole heart unit consumed, never energy/blessings; stable craft slot ID.
pub fn craft(r: &mut Run, heart: f64, item: f64, kind: &str) -> Result<bool, EngineError> {
    let name = craft_name(kind);
    let hi = r
        .inventory
        .iter()
        .position(|i| i.id == heart && i.kind == "heart");
    let ii = r
        .inventory
        .iter()
        .position(|i| i.id == item && equipment::equippable(i));
    let (Some(name), Some(hi), Some(ii)) = (name, hi, ii) else {
        log(r, "Invalid.", "player");
        return Ok(false);
    };
    if !supported(kind) {
        return Err(EngineError(format!("craft {kind}")));
    }
    let id = owner_id(name);
    if !r.player.abilities.iter().any(|s| s == id) {
        let label = crate::progression::catalog()
            .iter()
            .find(|a| a.id == id)
            .unwrap()
            .name
            .clone();
        log(r, format!("You have not acquired {label}."), "player");
        return Ok(false);
    }
    let limit = if r
        .player
        .abilities
        .iter()
        .any(|s| s == "kative-divine-craftsman")
    {
        2
    } else {
        1
    };
    if r.inventory[ii].enhancements.len() >= limit {
        log(
            r,
            format!(
                "Invalid. This item already has {limit} enhancement{}.",
                if limit == 1 { "" } else { "s" }
            ),
            "player",
        );
        return Ok(false);
    }
    let mut b = r.inventory[hi].bonus.value_or_default();
    let scale = if name == "Artifice" { 5.0 } else { 1.0 };
    for n in [
        &mut b.offense,
        &mut b.defense,
        &mut b.accuracy,
        &mut b.dodge,
        &mut b.dr,
        &mut b.perception,
        &mut b.max_energy,
        &mut b.max_hp,
    ] {
        if let Some(v) = n.as_mut() {
            *v = combat::integer_stat(*v) * scale;
        }
    }
    let index = r.inventory[ii].enhancements.len();
    r.inventory[ii].enhancements.push(ItemEnhancementsEntry {
        craft: kind.into(),
        bonus: b,
        active: false.into(),
        id: format!("craft:{item}:{index}").into(),
    });
    equipment::refresh_known_magic(&mut r.inventory[ii]);
    let item_name = r.inventory[ii].name.clone();
    inventory::take_one(r, hi);
    log(
        r,
        format!("{item_name} receives a {name} craft. Its benefits apply while {}.",
            if crate::revision50::enabled(r) { "carried" } else { "equipped" }),
        "player",
    );
    crate::events::actor_event(r, &r.player, "pulse", &format!("{kind} craft"), None, None);
    equipment::sync_health(r, false);
    Ok(true)
}
pub fn activate(r: &mut Run, id: &str) -> Result<bool, EngineError> {
    let parts: Vec<_> = id.split(':').collect();
    if parts.len() != 3
        || parts[0] != "craft"
        || parts[1].is_empty()
        || parts[2].is_empty()
        || !parts[1]
            .bytes()
            .chain(parts[2].bytes())
            .all(|b| b.is_ascii_digit())
    {
        return Ok(false);
    }
    let item_id = parts[1].parse::<f64>().unwrap_or(-1.0);
    let slot = parts[2].parse::<usize>().unwrap_or(usize::MAX);
    let Some(item) = r
        .inventory
        .iter()
        .find(|i| i.id == item_id && crate::revision50_inventory::active(r,i))
    else {
        return Ok(false);
    };
    let Some(e) = item.enhancements.get(slot) else {
        return Ok(false);
    };
    match craft_name(&e.craft) {
        Some("Fury") => {
            let name = item.name.clone();
            let active = !e.active.value_or(false);
            let item = r.inventory.iter_mut().find(|i| i.id == item_id).unwrap();
            item.enhancements[slot].active = active.into();
            log(
                r,
                format!(
                    "Fury craft {} on {name}.",
                    if active { "active" } else { "inactive" }
                ),
                "player",
            );
            Ok(true)
        }

        Some("Zeal") => {
            if crate::abilities::energy_activation_silenced(r, &r.player) {
                log(r, "Silence prevents this energy-paid activation.", "player");
                return Ok(false);
            }
            let name = item.name.clone();
            if r.player.energy < 15.0 {
                log(r, "Not enough energy for Zeal craft.", "player");
                return Ok(false);
            }
            let p = r.player.clone();
            crate::resources::record_loss(r, &p, "energy", 15.0);
            r.player.energy -= 15.0;
            r.player.hp = (r.player.hp + r.player.max_hp * 0.25).min(r.player.max_hp);
            log(
                r,
                format!("Zeal restores your health through {name}."),
                "player",
            );
            Ok(true)
        }
        Some("Guile" | "Artifice" | "Dream") => Ok(false),
        _ => Err(EngineError("unimplemented craft activation".into())),
    }
}

/// O(1), no mutation/RNG: revision43's fixed bonus matches two expected ordinary
/// Shortsword attacks in the documented floor15 cohort; older recordings keep10.
pub fn fury_damage_per_craft(r: &Run) -> f64 {
    if r.simulation_revision.value_or(29.0) >= 43.0 { 114.0 } else { 10.0 }
}

/// Pure per-craft funding forecast; no mutation or random draws.
pub fn preview_fury_bonus(r: &Run, mut energy: f64) -> (f64, f64) {
    let mut damage = 0.0;
    let mut spent = 0.0;
    for item in r.inventory.iter().filter(|i| crate::revision50_inventory::active(r,i)) {
        for e in &item.enhancements {
            if craft_name(&e.craft) == Some("Fury") && e.active.value_or(false) && energy >= 10.0 {
                damage += fury_damage_per_craft(r);
                energy -= 10.0;
                spent += 10.0;
            }
        }
    }
    (damage, spent)
}
/// Call once at source attack-action boundary; never once per strike in an assault.
pub fn fury_bonus(r: &mut Run) -> f64 {
    let bonus = fury_damage_per_craft(r);
    let entries: Vec<_> = r
        .inventory
        .iter()
        .enumerate()
        .filter(|(_, i)| crate::revision50_inventory::active(r,i))
        .flat_map(|(i, item)| {
            item.enhancements
                .iter()
                .enumerate()
                .filter(|(_, e)| craft_name(&e.craft) == Some("Fury") && e.active.value_or(false))
                .map(move |(j, _)| (i, j))
        })
        .collect();
    let mut damage = 0.0;
    for (i, j) in entries {
        if r.player.energy >= 10.0 {
            let p = r.player.clone();
            crate::resources::record_loss(r, &p, "energy", 10.0);
            r.player.energy -= 10.0;
            damage += bonus;
        } else {
            r.inventory[i].enhancements[j].active = false.into();
        }
    }
    damage
}
pub fn has_dream_craft(r: &Run) -> bool {
    r.inventory.iter().any(|i| {
        crate::revision50_inventory::active(r,i)
            && i.enhancements
                .iter()
                .any(|e| craft_name(&e.craft) == Some("Dream"))
    })
}
