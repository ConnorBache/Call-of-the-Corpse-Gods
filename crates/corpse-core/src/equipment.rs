//! Authored slots and resource settling; granted abilities/crafts remain gated.
use crate::{
    combat::{effective_stats, integer_stat},
    engine::{EngineError, log},
    model::*,
    resources,
};
#[derive(Clone, Copy, Debug)]
pub struct EquipmentBefore {
    pub max_energy: f64,
    pub boundless: bool,
    pub flying: bool,
    pub half_here_flying: bool,
}
pub fn capture_before(r: &Run) -> EquipmentBefore {
    EquipmentBefore {
        max_energy: effective_stats(r, &r.player).max_energy,
        boundless: r
            .player
            .abilities
            .iter()
            .any(|s| s == "grisha-boundless-hunger"),
        flying: crate::travel::flying(r, &r.player),
        half_here_flying: crate::half_here::clone_id(r)
            .and_then(|id| crate::engine::actor(r, id))
            .is_some_and(|a| crate::travel::flying(r, a)),
    }
}
pub fn equippable(i: &Item) -> bool {
    matches!(i.kind.as_str(), "weapon" | "armor" | "ring" | "amulet")
}
pub fn validate(i: &Item) -> Result<(), EngineError> {
    if i.enhancements
        .iter()
        .any(|e| !crate::crafting::supported(&e.craft))
        || crate::tinker::grants(i).any(|id| {
            !id.is_empty()
                && !crate::abilities::supported(id)
                && !matches!(id.as_str(), "numo-revert" | "morae-see-the-future")
        })
    {
        return Err(EngineError("equipment craft/granted-ability hooks".into()));
    }
    Ok(())
}
/// O(G*A*L) metadata preflight for G grant IDs, A=72 catalog entries and
/// maximum ID length L. It does not initialize powers or mutate item state.
pub fn validate_for_revision(i: &Item, revision: f64) -> Result<(), EngineError> {
    validate(i)?;
    if !crate::revision52_amulet_tiers::valid_item(i,revision) {return Err(EngineError("Invalid Revision 52 amulet tier".into()));}
    if revision >= 50.0 && crate::tinker::grants(i)
        .any(|id| crate::progression::definition_for_revision(revision, id).is_none()) {
        return Err(EngineError("Equipment contains an unknown or retired Revision 50 blessing grant".into()));
    }
    Ok(())
}
/// Forced health settlement at a mutation boundary. Time is the inclusive
/// effective-stat derivation + owner cloning + achievement/resource hooks;
/// O(A+B+C+I+E+owner data+hook cost), not O(1). Records O(1) cache state afterward.
/// `equipment=true` retains the original full-loss HP adjustment semantics.
pub fn sync_health(r: &mut Run, equipment: bool) {
    crate::divine_combat::unlock_blood_and_fury(r);
    let a = r.player.clone();
    let s = effective_stats(r, &a);
    let base = a.base_max_hp.value_or(100.0);
    r.player.base_max_hp = base.into();
    let max = (base + 6.0 * s.defense + s.max_hp.value_or(0.0)).max(1.0);
    if max == a.max_hp {
        crate::health_dirty::mark_clean(r);
        return;
    }
    let gain = max - a.max_hp;
    if !a.dead.value_or(false) && a.hp > 0.0 {
        let hp = if gain > 0.0 || equipment {
            a.hp + gain
        } else {
            a.hp.min(max)
        };
        resources::record_loss(r, &a, "hp", a.hp.min(a.hp - hp));
        r.player.hp = hp;
    }
    r.player.max_hp = max;
    crate::health_dirty::mark_clean(r);
}
/// Settle only a potential maximum-HP change at a repeated turn boundary.
/// Clean path: O(1) scalar/flag checks plus the still-preserved O(B+achievement
/// count) unlock hook; dirty path inherits `sync_health`'s inclusive cost.
/// Returns whether maximum-health derivation ran, not whether its value changed.
/// This is deliberately not advertised as a constant-time achievement system.
pub fn flush_health(r: &mut Run) -> bool {
    if crate::health_dirty::needs_sync(r) {
        sync_health(r, false);
        true
    } else {
        crate::divine_combat::unlock_blood_and_fury(r);
        false
    }
}
/// Source health synchronization for a potion's resolved resource owner.
pub fn sync_actor_health(r: &mut Run, id: f64) -> Result<(), EngineError> {
    let a = crate::engine::actor(r, id)
        .ok_or_else(|| EngineError("Missing health target".into()))?
        .clone();
    let owner = crate::half_here::owner_id(r, &a);
    if owner == r.player.id {
        sync_health(r, false);
        crate::half_here::sync(r);
        return Ok(());
    }
    if a.conditions.get("miniMe") != Some(&1.0) {
        crate::lifecycle::sync_enemy_health(r, id).map_err(EngineError)?;
        crate::half_here::sync(r);
        return Ok(());
    }
    let stats = effective_stats(r, &a);
    let base = a
        .base_max_hp
        .value_or(a.max_hp - 6.0 * a.stats.defense - a.stats.max_hp.value_or(0.0));
    let max = (base + 6.0 * stats.defense + stats.max_hp.value_or(0.0)).max(1.0);
    let next = crate::engine::actor_mut_trusted(r, id).unwrap();
    next.base_max_hp = Field::Value(base);
    if max != a.max_hp {
        let gain = max - a.max_hp;
        if !a.dead.value_or(false) && a.hp > 0.0 {
            let hp = if gain > 0.0 {
                a.hp + gain
            } else {
                a.hp.min(max)
            };
            resources::record_loss(r, &a, "hp", a.hp.min(a.hp - hp));
            crate::engine::actor_mut_trusted(r, id).unwrap().hp = hp;
        }
        crate::engine::actor_mut_trusted(r, id).unwrap().max_hp = max;
    }
    crate::half_here::sync(r);
    Ok(())
}
fn settle(r: &mut Run, before: EquipmentBefore) -> Result<(), EngineError> {
    let mut crushed = crate::equipment_abilities::reconcile(r, true).map_err(EngineError)?;
    let old = r.player.clone();
    let cap = effective_stats(r, &old).max_energy;
    let loss = (before.max_energy - cap).max(0.0);
    let mut e = (old.energy - loss).max(0.0);
    if (loss > 0.0 || before.boundless)
        && !r
            .player
            .abilities
            .iter()
            .any(|s| s == "grisha-boundless-hunger")
    {
        e = e.min(cap);
    }
    resources::record_loss(r, &old, "energy", (old.energy - e).max(0.0));
    r.player.energy = e;
    sync_health(r, true);
    let mut losses = vec![];
    if r.player.hp < old.hp {
        losses.push(format!("{} HP", old.hp - r.player.hp));
    }
    if r.player.energy < old.energy {
        losses.push(format!("{} energy", old.energy - r.player.energy));
    }
    if !losses.is_empty() {
        log(
            r,
            format!("The equipment change removes {}.", losses.join(" and ")),
            "player",
        );
    }
    if r.player.energy < old.energy && crate::abilities::remove_unfunded(r) {
        crushed |= crate::divine_environment::phase_crushing_endpoint(r).is_some();
    }
    if crushed {
        let p = r.player.clone();
        resources::record_loss(r, &p, "hp", p.hp.max(0.0));
        r.player.hp = 0.0;
        crate::engine::unpreventable_death(r, r.player.id, "phase-crushing")?;
    } else {
        crate::engine::settle_equipment_death(r)?;
    }
    crate::half_here::sync(r);
    settle_flight_loss(r, before)?;
    Ok(())
}
fn settle_flight_loss(r: &mut Run, before: EquipmentBefore) -> Result<(), EngineError> {
    if r.status != "playing" {
        return Ok(());
    }
    if let Some(id) = crate::half_here::clone_id(r) {
        if before.half_here_flying
            && !crate::travel::flying(r, crate::engine::actor(r, id).unwrap())
        {
            crate::travel::fall_actors_one_level(r, r.depth, &[id], None).map_err(EngineError)?;
        }
    }
    if before.flying
        && !crate::travel::flying(r, &r.player)
        && !r.player.dead.value_or(false)
        && r.status == "playing"
    {
        let f = r.floor().unwrap();
        let tile = f.tiles[r.player.y as usize * f.width() + r.player.x as usize].clone();
        if tile == "chasm" {
            crate::travel::travel(r, 1.0, true, false).map_err(EngineError)?;
        } else if tile == "pit" {
            r.status = "won".into();
            log(
                r,
                "Without flight, you fall into the bottomless pit. Victory.",
                "player",
            );
        }
    }
    Ok(())
}
/// O(I), preserves inventory identities and two-hand storage in hand1.
pub fn equip(r: &mut Run, id: f64, slot: &str) -> Result<bool, EngineError> {
    if let Some(message) = change_blocker(
        r,
        &Command::Equip {
            id,
            slot: slot.into(),
        },
    ) {
        log(r, message, "player");
        return Ok(false);
    }
    let Some(i) = r.inventory.iter().position(|i| i.id == id) else {
        return Ok(false);
    };
    let item = r.inventory[i].clone();
    if !equippable(&item) {
        return Ok(false);
    }
    validate_for_revision(&item, r.simulation_revision.value_or(29.0))?;
    let allowed = match item.kind.as_str() {
        "weapon" => matches!(slot, "hand1" | "hand2"),
        "ring" => matches!(slot, "ring1" | "ring2"),
        _ => slot == item.kind,
    };
    if !allowed {
        return Ok(false);
    }
    for i in &r.inventory {
        validate_for_revision(i, r.simulation_revision.value_or(29.0))?;
    }
    let before = capture_before(r);
    let two = item.kind == "weapon" && item.hands.value_or(1.0) == 2.0;
    let slot = if two { "hand1" } else { slot };
    for other in &mut r.inventory {
        if other.equipped.as_ref().is_some_and(|s| s == slot)
            || (item.kind == "weapon"
                && other.kind == "weapon"
                && other.equipped.as_ref().is_some()
                && (two || other.hands.value_or(1.0) == 2.0))
        {
            other.equipped = Field::Missing;
        }
    }
    r.inventory[i].equipped = slot.to_string().into();
    settle(r, before)?;
    Ok(true)
}
pub fn unequip(r: &mut Run, id: f64) -> Result<bool, EngineError> {
    if let Some(message) = change_blocker(r, &Command::Unequip { id }) {
        log(r, message, "player");
        return Ok(false);
    }
    let Some(i) = r.inventory.iter().position(|i| i.id == id) else {
        return Ok(false);
    };
    if !equippable(&r.inventory[i]) || r.inventory[i].equipped.as_ref().is_none() {
        return Ok(false);
    }
    validate(&r.inventory[i])?;
    let before = capture_before(r);
    r.inventory[i].equipped = Field::Missing;
    settle(r, before)?;
    Ok(true)
}
pub fn settle_removed(r: &mut Run, before: EquipmentBefore) -> Result<(), EngineError> {
    settle(r, before)
}
pub fn add_bonus(s: &mut Stats, b: &PartialStats) {
    macro_rules! add{($($k:ident),*)=>{$(if let Some(n)=b.$k.as_ref(){s.$k=integer_stat(s.$k)+integer_stat(*n);})*};}
    add!(
        offense, defense, accuracy, dodge, dr, perception, max_energy
    );
    if let Some(n) = b.max_hp.as_ref() {
        s.max_hp = (integer_stat(s.max_hp.value_or(0.0)) + integer_stat(*n)).into();
    }
}
/// Outer-command ownership reconciliation for the admitted no-grant subset.
pub fn sync_ownership(r: &mut Run) -> Result<(), EngineError> {
    for item in &r.inventory {
        validate_for_revision(item, r.simulation_revision.value_or(29.0))?;
    }
    let before = capture_before(r);
    let crushed = crate::equipment_abilities::reconcile(r, true).map_err(EngineError)?;
    if crushed {
        let p = r.player.clone();
        resources::record_loss(r, &p, "hp", p.hp.max(0.0));
        r.player.hp = 0.0;
        if r.status == "playing" {
            crate::engine::unpreventable_death(r, r.player.id, "phase-crushing")?;
        }
    }
    crate::half_here::sync(r);
    settle_flight_loss(r, before)?;
    Ok(())
}
/// Immutable authored base equipment; O(B) first catalog parse, O(item size) per clone.
pub fn make_equipment(name: &str, id: f64) -> Result<Item, EngineError> {
    static C: std::sync::OnceLock<serde_json::Map<String, serde_json::Value>> =
        std::sync::OnceLock::new();
    let c = C.get_or_init(|| {
        serde_json::from_str(include_str!("../catalog/equipment-v48.json")).unwrap()
    });
    let Some(def) = c.get(name) else {
        return Err(EngineError(format!("unknown equipment {name}")));
    };
    let mut value = def.clone();
    value["id"] = serde_json::json!(id);
    value["name"] = name.into();
    let bonus = value
        .get("bonus")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    value["bonus"] = bonus.clone();
    value["baseBonus"] = bonus;
    value["enhancements"] = serde_json::json!([]);
    serde_json::from_value(value).map_err(|e| EngineError(e.to_string()))
}
pub fn make_arrows(id: f64, count: f64) -> Item {
    Item {
        id,
        name: "Arrows".into(),
        kind: "ammo".into(),
        glyph: "/".into(),
        effect: "arrows".to_string().into(),
        quantity: count.into(),
        ..Default::default()
    }
}
#[cfg(test)]
mod catalog_tests {
    use super::*;
    #[test]
    fn authored_bow_and_arrow_bundle() {
        let bow = make_equipment("Bow", 2.0).unwrap();
        assert_eq!(bow.damage.value_or(0.0), 15.0);
        assert_eq!(bow.hands.value_or(0.0), 2.0);
        assert_eq!(bow.offense_multiplier.value_or(0.0), 0.5);
        assert_eq!(make_arrows(3.0, 5.0).quantity.value_or(0.0), 5.0);
    }
    #[test]
    fn every_equipment_definition_parses() {
        let c: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(include_str!("../catalog/equipment-v48.json")).unwrap();
        for name in c.keys() {
            assert!(make_equipment(name, 1.0).is_ok(), "{name}");
        }
    }
}
/// O(I+E) time/space preview; capacity-losing changes never strand occupied slots.
pub fn change_blocker(r: &Run, c: &Command) -> Option<String> {
    if crate::revision50_inventory::enabled(r) {
        return match c {
            Command::Equip { .. } | Command::Unequip { .. } =>
                Some("Carried rings and amulets are always active. Drop jewelry to stop its effects; apply bows directly.".into()),
            Command::Drop { id } | Command::Throw { id, .. } => removal_blocker(r, *id, false),
            _ => None,
        };
    }

    let (id, slot, remove) = match c {
        Command::Equip { id, slot } => (*id, Some(slot.as_str()), false),
        Command::Unequip { id } => (*id, None, false),
        Command::Drop { id } | Command::Throw { id, .. } => (*id, None, true),
        _ => return None,
    };
    let mut items = r.inventory.clone();
    let i = items.iter().position(|i| i.id == id)?;
    let item = items[i].clone();
    if let Some(slot) = slot {
        let allowed = match item.kind.as_str() {
            "weapon" => matches!(slot, "hand1" | "hand2"),
            "ring" => matches!(slot, "ring1" | "ring2"),
            "armor" | "amulet" => slot == item.kind,
            _ => false,
        };
        if !allowed {
            return None;
        }
        let two = item.kind == "weapon" && item.hands.value_or(1.0) == 2.0;
        let slot = if two { "hand1" } else { slot };
        for other in &mut items {
            if other.equipped.as_ref().is_some_and(|s| s == slot)
                || (item.kind == "weapon"
                    && other.kind == "weapon"
                    && other.equipped.as_ref().is_some()
                    && (two || other.hands.value_or(1.0) == 2.0))
            {
                other.equipped = Field::Missing;
            }
        }
        items[i].equipped = slot.to_string().into();
    } else if remove {
        if crate::inventory::quantity(&items[i]) > 1.0 {
            items[i].quantity = (crate::inventory::quantity(&items[i]) - 1.0).into();
        } else {
            items.remove(i);
        }
    } else {
        if !equippable(&item) || item.equipped.as_ref().is_none() {
            return None;
        }
        items[i].equipped = Field::Missing;
    }
    let capacity = crate::crafting::capacity(&items);
    let occupied: f64 = items
        .iter()
        .map(|i| {
            if crate::inventory::is_arrow(i) {
                1.0
            } else {
                crate::inventory::quantity(i)
            }
        })
        .sum();
    if capacity < crate::crafting::capacity(&r.inventory) && occupied > capacity {
        let n = occupied - capacity;
        Some(format!(
            "You cannot remove that carrying-capacity equipment: {occupied} items would need {capacity} slots. Free {n} more slot{} first.",
            if n == 1.0 { "" } else { "s" }
        ))
    } else {
        None
    }
}
/// R50 capacity-loss preview. Existing items are never discarded to make room.
/// O(I + G*A + B) plus item payload cloning; no mutations, energy refill or RNG.
/// Whole-stack removal is used by debug stash; ordinary drop/throw removes one.
pub fn removal_blocker(r: &Run, id: f64, whole_stack: bool) -> Option<String> {
    if !crate::revision50_inventory::enabled(r) { return None; }
    let mut items = r.inventory.clone();
    let index = items.iter().position(|i| i.id == id)?;
    if !whole_stack && crate::inventory::quantity(&items[index]) > 1.0 {
        items[index].quantity = (crate::inventory::quantity(&items[index]) - 1.0).into();
    } else { items.remove(index); }
    let abilities = crate::equipment_abilities::equipment_ability_state(r, &items).abilities;
    let before = crate::revision50_inventory::capacity(r);
    let after = crate::revision50_inventory::capacity_for_abilities_revision(&abilities, r.simulation_revision.value_or(29.));
    if (after.rings < before.rings || after.amulets < before.amulets || after.potions < before.potions || after.hearts < before.hearts)
        && !crate::revision50_inventory::finite_categories_fit_revision(&items, &abilities, r.simulation_revision.value_or(29.)) {
        Some("This jewelry grants carrying capacity. Free space in the affected category before removing it.".into())
    } else { None }
}
/// Reclassifies only already-sensed magic after a player-authored craft, never reveals unknown items.
pub fn refresh_known_magic(i: &mut Item) {
    if i.magic_nature.is_missing() {
        return;
    }
    let base = if let Some(b) = i.base_bonus.as_ref() {
        b.clone()
    } else {
        static C: std::sync::OnceLock<serde_json::Map<String, serde_json::Value>> =
            std::sync::OnceLock::new();
        let c = C.get_or_init(|| {
            serde_json::from_str(include_str!("../catalog/equipment-v48.json")).unwrap()
        });
        let item_name = if i.kind == "armor"
            && i.name.strip_prefix("Plate Armor").is_some_and(|s| {
                s.is_empty()
                    || s.starts_with(" of ")
                    || s == ", Cursed"
                    || s.starts_with(", Cursed ")
            }) {
            i.name.replacen("Plate Armor", "Plate armor", 1)
        } else {
            i.name.clone()
        };
        c.keys()
            .find_map(|name| {
                let d = make_equipment(name, 0.0).ok()?;
                if d.kind == i.kind
                    && (item_name == *name
                        || item_name.starts_with(&format!("{name} "))
                        || item_name.starts_with(&format!("{name},")))
                {
                    Some(d.bonus.value_or_default())
                } else {
                    None
                }
            })
            .unwrap_or_default()
    };
    let b = i.bonus.value_or_default();
    let mut positive = i.enchantment.value_or(0.0) > 0.0
        || crate::tinker::grants(i).next().is_some()
        || !i.enhancements.is_empty();
    let mut negative = i.enchantment.value_or(0.0) < 0.0;
    for (v, base) in [
        (&b.offense, &base.offense),
        (&b.defense, &base.defense),
        (&b.accuracy, &base.accuracy),
        (&b.dodge, &base.dodge),
        (&b.dr, &base.dr),
        (&b.perception, &base.perception),
        (&b.max_energy, &base.max_energy),
        (&b.max_hp, &base.max_hp),
    ] {
        let n = v.value_or(0.0) - base.value_or(0.0);
        positive |= n > 0.0;
        negative |= n < 0.0;
    }
    for e in &i.enhancements {
        for v in [
            &e.bonus.offense,
            &e.bonus.defense,
            &e.bonus.accuracy,
            &e.bonus.dodge,
            &e.bonus.dr,
            &e.bonus.perception,
            &e.bonus.max_energy,
            &e.bonus.max_hp,
        ] {
            positive |= v.value_or(0.0) > 0.0;
            negative |= v.value_or(0.0) < 0.0;
        }
    }
    i.magic_nature = match (positive, negative) {
        (true, true) => "bizarre".to_string().into(),
        (true, false) => "good".to_string().into(),
        (false, true) => "bad".to_string().into(),
        _ => Field::Missing,
    };
}
#[path = "equipment_generation.rs"]
mod generated_loot;
pub(crate) use generated_loot::make_mercuria_reward;
pub use generated_loot::{
    generate_floor_loot_for_run, make_godbeast_permanent_for_run,
    create_equipment, create_equipment_revision, generate_floor_loot, generate_floor_loot_revision, make_godbeast_permanent, make_godbeast_permanent_revision, modify_equipment,
};
