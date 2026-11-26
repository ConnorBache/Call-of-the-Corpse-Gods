//! Stable pickup order, quantity identities, basic equipment and consumables.
use crate::{
    combat,
    engine::{EngineError, log},
    equipment,
    model::*,
    progression,
};
pub fn is_arrow(i: &Item) -> bool {
    i.kind == "ammo" && i.effect.as_ref().is_some_and(|s| s == "arrows")
}
pub fn quantity(i: &Item) -> f64 {
    i.quantity.value_or(1.0)
}
pub fn occupancy(r: &Run) -> f64 {
    if crate::revision50_inventory::enabled(r) {
        return r.inventory.iter().map(crate::revision50_inventory::units).sum();
    }
    r.inventory
        .iter()
        .map(|i| if is_arrow(i) || crate::phantasmii_generation::is_shard(i) { 1.0 } else { quantity(i) })
        .sum()
}
pub fn carrying_capacity(r: &Run) -> Result<f64, EngineError> {
    for i in &r.inventory {
        equipment::validate(i)?;
    }
    if crate::revision50_inventory::enabled(r) {
        if r.inventory.iter().any(|i| !crate::revision50_inventory::classified(i)) {
            return Err(EngineError("This item has no Revision 50 backpack category.".into()));
        }
        return Ok(crate::revision50_inventory::capacity(r).total());
    }
    Ok(crate::crafting::capacity(&r.inventory))
}
pub fn stack_key(i: &Item) -> Option<String> {
    if i.source_actor_id.as_ref().is_some() { return None; }
    if is_arrow(i) {
        return Some("arrows".into());
    }
    if crate::phantasmii_generation::is_shard(i) {
        return Some("phantasmii-shards".into());
    }
    if i.kind != "heart"
        || !i
            .name
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .any(|s| s == "Jackal")
    {
        return None;
    }
    let bonus = serde_json::to_value(i.bonus.as_ref()).unwrap();
    let mut entries: Vec<_> = bonus
        .as_object()
        .into_iter()
        .flat_map(|m| m.iter())
        .collect();
    entries.sort_by(|(a, _), (b, _)| a.cmp(b));
    Some(crate::temporal::js_stringify(&serde_json::json!([
        i.name,
        heart_creature_type(i),
        i.energy.value_or(0.0),
        i.exp.value_or(0.0),
        entries
            .into_iter()
            .map(|(k, v)| serde_json::json!([k, v]))
            .collect::<Vec<_>>(),
        i.enhancements,
        i.blessed.value_or(false)
    ])))
}

/// O(I) removal shifts for I carried entries; stacked split clones item data.
/// Health invalidation is O(1), including consume-then-replace at equal length.
pub fn take_one(r: &mut Run, index: usize) -> Item {
    if crate::revision50_inventory::active(r, &r.inventory[index]) || r.inventory[index].equipped.as_ref().is_some() {
        crate::health_dirty::invalidate(r);
    }
    if quantity(&r.inventory[index]) > 1.0 {
        r.inventory[index].quantity = (quantity(&r.inventory[index]) - 1.0).into();
        let mut item = r.inventory[index].clone();
        item.id = r.next_id;
        r.next_id += 1.0;
        item.quantity = 1.0.into();
        item
    } else {
        r.inventory.remove(index)
    }
}
/// O(G(I+B)) category admission plus full equipment settlement per jewelry pickup;
/// G ground entries, I carried entries, B blessings. Stable order, O(G) output.
pub fn collect_underfoot(r: &mut Run, capacity: f64) -> Result<usize, EngineError> {
    if r.status != "playing" {
        return Ok(0);
    }
    let ground = r
        .ground
        .get(&r.depth.to_string())
        .map(|v| v.as_ref().clone())
        .unwrap_or_default();
    let mut remaining = vec![];
    let mut collected = 0;
    let mut collected_shards = false;
    let mut collected_hearts = std::collections::HashSet::new();
    for mut e in ground {
        let shard = crate::phantasmii_generation::is_shard(&e.item);
        if !crate::dreamwalking::item_in_plane(r,&e.item) { remaining.push(e); continue; }
        if !crate::portals::same_position(r,&r.player,e.x,e.y) {
            e.auto_pickup_blocked = Field::Missing;
            remaining.push(e);
            continue;
        }
        if e.auto_pickup_blocked.value_or(false) {
            remaining.push(e);
            continue;
        }
        let key = stack_key(&e.item);
        let stack = key.as_ref().and_then(|k| {
            r.inventory
                .iter()
                .position(|i| stack_key(i).as_ref() == Some(k))
        });
        if crate::revision50_inventory::enabled(r) && !crate::revision50_inventory::classified(&e.item) {
            remaining.push(e);
            continue;
        }
        let space = if crate::revision50_inventory::enabled(r) {
            crate::revision50_inventory::space_for(r, &e.item)
        } else { (capacity - occupancy(r)).max(0.0) };
        if space == 0.0 && !((is_arrow(&e.item) || shard) && stack.is_some()) {
            remaining.push(e);
            continue;
        }
        let available = quantity(&e.item);
        let taken = if is_arrow(&e.item) || shard {
            available
        } else {
            available.min(space)
        };
        let before = (crate::revision50::enabled(r) && crate::revision50_inventory::worn(&e.item))
            .then(|| equipment::capture_before(r));
        if let Some(i) = stack {
            r.inventory[i].quantity = (quantity(&r.inventory[i]) + taken).into();
        } else {
            let mut item = e.item.clone();
            item.dream_ground=Field::Missing;
            if taken != available {
                item.id = r.next_id;
                r.next_id += 1.0;
                item.quantity = taken.into();
            }
            if crate::revision50::enabled(r) {
                crate::revision50_ability_energy::acquire_item_for_revision(&mut r.player, &mut item, r.simulation_revision.value_or(29.))
                    .map_err(EngineError)?;
            }
            r.inventory.push(item);
        }
        if let Some(before) = before { equipment::settle_removed(r, before)?; }
        if r.simulation_revision.value_or(29.) >= 52. && e.item.kind == "heart" && taken > 0. {
            collected_hearts.insert(e.item.id.to_bits());
        }
        let name = crate::revision50_item_knowledge::display_name(r, &e.item);
        if taken < available {
            e.item.quantity = (available - taken).into();
            remaining.push(e);
        }
        collected += taken as usize;
        collected_shards |= shard;
        log(
            r,
            format!(
                "You pick up {name}{}.",
                if taken > 1.0 {
                    format!(" ×{taken}")
                } else {
                    String::new()
                }
            ),
            "player",
        );
    }
    *r.ground_at_mut(r.depth) = remaining;
    crate::corpses::record_heart_pickups(r, &collected_hearts);
    if collected_shards { crate::dreamwalking::shards_collected(r); }
    Ok(collected)
}
pub fn command(r: &mut Run, c: &Command) -> Result<Option<bool>, EngineError> {
    if let Some(result) = crate::revision50_amulet_effects::command(r,c)? { return Ok(Some(result)); }
    if let Command::UseAt { id, x, y } = c {
        if crate::revision50_inventory::enabled(r)
            && r.inventory.iter().any(|i| i.id == *id && crate::projectiles::is_bow(i))
        {
            return crate::projectiles::command(r, &Command::Shoot { id: *id, x: *x, y: *y });
        }
        return crate::borge::use_at(r, *id, *x, *y).map(Some);
    }
    if let Command::Throw { id, .. } | Command::Shoot { id, .. } = c {
        if r.inventory.iter().any(|i|i.id == *id && ((crate::borge::is_shard(i) && (!crate::borge::weapon_enabled(r) || matches!(c,Command::Shoot{..}))) || crate::phantasmii_generation::is_shard(i))) { return Ok(Some(false)); }
    }
    if matches!(c, Command::Shoot { .. } | Command::Throw { .. }) {
        return crate::projectiles::command(r, c);
    }
    let id = match c {
        Command::Drop { id }
        | Command::Use { id }
        | Command::Equip { id, .. }
        | Command::Unequip { id } => *id,
        _ => return Ok(None),
    };
    let Some(index) = r.inventory.iter().position(|i| i.id == id) else {
        if matches!(c,Command::Use {..}) {
            let ground=r.ground.get(&r.depth.to_string()).and_then(|gs|gs.iter().position(|g|g.item.id==id && crate::portals::same_position(r,&r.player,g.x,g.y) && crate::royal_honey::is_honey(&g.item)));
            if let Some(index)=ground {
                if crate::royal_honey::eat(r) { r.ground_at_mut(r.depth).remove(index);return Ok(Some(true)); }
            }
        }
        return Ok(Some(false));
    };
    let item = r.inventory[index].clone();
    // Capture before acquisition: a heart that grants a new blessing cannot make
    // the already admitted action retroactively free.
    let hp_before_item_use = r.player.hp;
    let energy_before_item_use = r.player.energy;
    let tools_free = matches!(c, Command::Use { .. })
        && matches!(item.kind.as_str(), "heart" | "potion")
        && r.player.abilities.iter().any(|a| a == "isheilah-tools-of-the-trade");
    let display_name = crate::revision50_item_knowledge::display_name(r, &item);
    if crate::phantasmii_generation::is_shard(&item) { return Ok(Some(false)); }
    if crate::phantasmii_generation::is_dagger(&item) && matches!(c, Command::Use { .. }) {
        return crate::dreamwalking::apply_dagger(r, id).map(Some);
    }
    match c {
        Command::Equip { slot, .. } => {
            if !equipment::equip(r, id, slot)? {
                return Ok(Some(false));
            }
            log(r, format!("You equip {}.", display_name), "player");
            crate::events::actor_event(
                r,
                &r.player,
                "status",
                &format!("Equip {}", display_name),
                None,
                None,
            );
            equipment::sync_health(r, false);
        }
        Command::Unequip { .. } => {
            if !equipment::unequip(r, id)? {
                return Ok(Some(false));
            }
            log(r, format!("You unequip {}.", display_name), "player");
            crate::events::actor_event(
                r,
                &r.player,
                "status",
                &format!("Unequip {}", display_name),
                None,
                None,
            );
            equipment::sync_health(r, false);
        }
        Command::Drop { .. } => {
            if crate::revision50::enabled(r) {
                if let Some(message) = equipment::change_blocker(r, c) { log(r, message, "player"); return Ok(Some(false)); }
            }
            equipment::validate(&item)?;
            let cap = equipment::capture_before(r);
            let mut dropped = take_one(r, index);
            dropped.equipped = Field::Missing;
            crate::item_placement::place_ground_item(
                r,
                r.depth,
                GroundItem {
                    x: r.player.x,
                    y: r.player.y,
                    item: dropped,
                    auto_pickup_blocked: true.into(),
                },
                |r, d| crate::startup::ensure_floors(r, d as u32),
            )
            .map_err(EngineError)?;
            equipment::settle_removed(r, cap)?;
            log(r, format!("You drop {}.", display_name), "player");
            crate::events::actor_event(
                r,
                &r.player,
                "status",
                &format!("Drop {}", display_name),
                None,
                None,
            );
        }
        Command::Use { .. } => {
            if crate::royal_honey::is_honey(&item) {
                if !crate::royal_honey::eat(r) {return Ok(Some(false));}
                take_one(r,index);
            } else if item.kind == "heart" {
                equipment::validate(&item)?;
                let armor_gain = if r
                    .player
                    .abilities
                    .iter()
                    .any(|s| s == "kative-adaptive-armor")
                {
                    if let Some(kind) = heart_creature_type(&item) {
                        let prior = r
                            .player
                            .adaptive_hearts
                            .as_ref()
                            .and_then(|m| m.get(&kind))
                            .copied()
                            .unwrap_or(0.0);
                        let gain = (3.0 - prior.floor().max(0.0)).max(0.0);
                        if gain > 0.0 {
                            r.player
                                .adaptive_hearts
                                .get_or_insert_with(Default::default)
                                .insert(kind, prior + 1.0);
                        }
                        gain
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };
                let rewards = effective_heart_rewards(r, &item);
                if let Some(b) = rewards.bonus.as_ref() {
                    equipment::add_bonus(&mut r.player.stats, b);
                }
                let energy = r.player.energy + item.energy.value_or(0.0);
                r.player.energy = if r
                    .player
                    .abilities
                    .iter()
                    .any(|s| s == "grisha-boundless-hunger")
                {
                    energy
                } else {
                    energy.min(combat::effective_stats(r, &r.player).max_energy)
                };
                progression::award(r, rewards.blessings);
                let creature = heart_creature_type(&item).unwrap_or("Unknown creature".into());
                let s = r.run_stats.get_or_insert_with(|| RunStatistics {
                    version: 1.0,
                    ..Default::default()
                });
                *s.hearts_eaten.entry(creature).or_default() += 1.0;
                take_one(r, index);
                equipment::sync_health(r, false);
                progression::level_up(r);
                log(r, format!("You eat {}.", item.name), "player");
                if armor_gain > 0.0 {
                    log(
                        r,
                        format!(
                            "Adaptive Armor gains {armor_gain} DR against {}.",
                            crate::revision50_creature_identity::name_for_revision(&heart_creature_type(&item).unwrap(), r.simulation_revision.value_or(29.))
                        ),
                        "player",
                    );
                }
                if !crate::events::enabled(r) || r.player.hp>hp_before_item_use {
                    crate::events::actor_event(r,&r.player,"heal",&format!("Eat {}",item.name),None,None);
                }
                if r.player.energy>energy_before_item_use {
                    crate::events::inventory(r,"shared","energy-rise",None);
                }
            } else if item.kind == "potion" {
                potion(r, index)?;
            } else if item.kind == "tool" && item.effect.as_ref().is_some_and(|v| v == "tree-tap") {
                let result = crate::godbeast_interactions::tap_veridian(r);
                log(r, result.message, "player");
                if !result.success {
                    return Ok(Some(false));
                }
                if let Some(sap) = result.sap {
                    let sap_id=sap.id;
                    take_one(r, index);
                    r.inventory.push(sap);
                    crate::events::inventory(r,"E82","droplet-leaf",Some(sap_id));
                    crate::events::inventory(r,"E82","fracture",Some(item.id));
                    if !crate::events::enabled(r) {
                        crate::events::actor_event(r, &r.player, "heal", if r.simulation_revision.value_or(29.)>=33. {"Verdania Sap"} else {"Veridian Sap"}, None, None);
                    }
                } else {
                    return Ok(Some(false));
                }
            } else {
                return Err(EngineError(format!("item use {}", item.name)));
            }
        }
        _ => unreachable!(),
    }
    if tools_free && r.inventory.iter().find(|i|i.id==item.id).is_none_or(|after|
        quantity(after)<quantity(&item) || after.diluted.value_or(false)!=item.diluted.value_or(false)) {
        crate::events::inventory(r,"P14","speed",Some(item.id));
    }
    Ok(Some(true))
}
fn potion(r: &mut Run, index: usize) -> Result<(), EngineError> {
    crate::potions::apply(r, index, r.player.x, r.player.y, false)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reference_inventory_boundaries() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/inventory-v48.json")).unwrap();
        for case in cases.as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let mut r: Run = serde_json::from_value(case["before"].clone()).unwrap();
            match name {
                "equip capacity armor" => {
                    equipment::equip(&mut r, 300.0, "armor").unwrap();
                }
                "unequip capacity armor fatal" => {
                    equipment::unequip(&mut r, 300.0).unwrap();
                }
                "two-handed bow clears shortsword" => {
                    equipment::equip(&mut r, 2.0, "hand2").unwrap();
                }
                "one-handed weapon replaces two-handed" => {
                    equipment::equip(&mut r, 1.0, "hand2").unwrap();
                }
                "unequip bow" => {
                    equipment::unequip(&mut r, 2.0).unwrap();
                }
                n if n.starts_with("pickup") => {
                    collect_underfoot(&mut r, 20.0).unwrap();
                }
                _ => potion(&mut r, 3).unwrap(),
            }
            let actual = serde_json::to_value(&r).unwrap();
            let expected_run: Run = serde_json::from_value(case["after"].clone()).unwrap();
            let expected = serde_json::to_value(expected_run).unwrap();
            for key in [
                "player",
                "inventory",
                "ground",
                "log",
                "logKinds",
                "logArchive",
                "nextId",
                "knownPotions",
                "resourceHistory",
                "status",
                "runStats",
            ] {
                assert_eq!(actual.get(key), expected.get(key), "{name}: {key}");
            }
        }
    }
}
pub fn heart_creature_type(i: &Item) -> Option<String> {
    if i.kind != "heart" {
        return None;
    }
    static TYPES: std::sync::OnceLock<std::collections::BTreeMap<String, String>> =
        std::sync::OnceLock::new();
    let types = TYPES.get_or_init(|| {
        serde_json::from_str(include_str!("../catalog/heart-types-v48.json")).unwrap()
    });
    let explicit = i.heart_creature_type.as_ref().filter(|s| !s.is_empty());
    let raw = if let Some(s) = explicit {
        s.as_str()
    } else {
        i.name
            .strip_suffix(" <heart>")
            .or_else(|| i.name.strip_suffix(" heart"))?
            .trim_end()
    };
    let raw = raw
        .strip_prefix("Mercuria’s ")
        .or_else(|| raw.strip_prefix("Mercuria's "))
        .unwrap_or(raw);
    let raw = crate::revision50_creature_identity::legacy_name(raw);
    if let Some(name) = types.get(raw) {
        return Some(name.clone());
    }
    let pale = raw
        .strip_prefix("Greater ")
        .unwrap_or(raw)
        .strip_prefix("Pale ")
        .is_some_and(|s| {
            matches!(
                s,
                "Biter" | "Breather" | "Spitter" | "Birther" | "Screamer" | "Jackal"
            )
        });
    if explicit.is_some()
        || pale
        || matches!(
            raw,
            "Zillennia" | "Ichor Fish" | "Bastidion" | "Vent Fungus"
        )
    {
        Some(raw.into())
    } else {
        None
    }
}
/// O(I+V+G), one fair coin even with no eligible carried unit; then weighted unit draw.
pub fn float_carried_item(r: &mut Run) -> Result<bool, String> {
    let f = r.floor().ok_or("active floor missing")?;
    let at = r.player.y as usize * f.width() + r.player.x as usize;
    if r.status != "playing"
        || r.player.dead.value_or(false)
        || f.tiles[at] != "water"
        || f.ichor
            .as_ref()
            .and_then(|v| v.get(at))
            .copied()
            .unwrap_or(0.0)
            != 0.0
        || crate::environment::is_flying_in(r, &r.player)
        || r.player.abilities.iter().any(|s| s == "etill-drown")
    {
        return Ok(false);
    }
    if crate::rng::random(r) >= 0.5 {
        return Ok(false);
    }
    let carried_jewelry_active = crate::revision50::enabled(r);
    let eligible = |i: &Item| {
        if carried_jewelry_active && crate::revision50_inventory::worn(i) { return false; }
        !equipment::equippable(i)
            || i.equipped.as_ref().is_none()
            || (i.kind == "weapon"
                && i.ranged_weapon
                    .as_ref()
                    .is_some_and(|s| matches!(s.as_str(), "bow" | "composite-bow"))
                && (i.hands.value_or(0.0) != 2.0
                    || i.equipped.as_ref().is_none_or(|s| s != "hand1")))
    };
    let total: f64 = r
        .inventory
        .iter()
        .filter(|i| eligible(i))
        .map(quantity)
        .sum();
    if total == 0.0 {
        return Ok(false);
    }
    let mut selected = (crate::rng::random(r) * total).floor();
    let index = r
        .inventory
        .iter()
        .position(|i| {
            if !eligible(i) {
                return false;
            }
            selected -= quantity(i);
            selected < 0.0
        })
        .ok_or("invalid physical item quantities")?;
    let item = take_one(r, index);
    let name = crate::revision50_item_knowledge::display_name(r, &item);
    crate::item_placement::place_ground_item(
        r,
        r.depth,
        GroundItem {
            x: r.player.x,
            y: r.player.y,
            item,
            auto_pickup_blocked: true.into(),
        },
        |r, d| crate::startup::ensure_floors(r, d as u32),
    )?;
    log(
        r,
        format!("{name} floats away from your pack."),
        "environment",
    );
    Ok(true)
}
#[cfg(test)]
mod water_tests {
    use super::*;
    #[test]
    fn exact_float_item_rng_and_quantities() {
        let f: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/water-inventory-v48.json"))
                .unwrap();
        let base: Run = serde_json::from_value(f["base"].clone()).unwrap();
        for c in f["cases"].as_array().unwrap() {
            let mut r = base.clone();
            r.rng = c["rng"].as_f64().unwrap();
            if c["empty"] == true {
                r.inventory.clear();
            }
            assert_eq!(
                float_carried_item(&mut r).unwrap(),
                c["result"].as_bool().unwrap()
            );
            let actual = serde_json::to_value(&r).unwrap();
            let mut e = f["base"].clone();
            for (k, v) in c["after"].as_object().unwrap() {
                e[k] = v.clone()
            }
            let e: Run = serde_json::from_value(e).unwrap();
            let expected = serde_json::to_value(e).unwrap();
            for k in c["after"].as_object().unwrap().keys() {
                assert_eq!(
                    actual[k], expected[k],
                    "rng {} empty {} {k}",
                    c["rng"], c["empty"]
                );
            }
        }
    }
}
#[path = "heart_items.rs"]
mod heart_items;
pub use heart_items::{heart_for, heart_for_revision, effective_heart_rewards, HeartRewards};
#[path = "item_magic.rs"]
mod item_magic;
pub use item_magic::magical as is_magical;
pub use item_magic::remember_detected_items;
pub use item_magic::{
    displayed_item_magic_nature, potion_appearance, potion_description, potion_nature,
};
