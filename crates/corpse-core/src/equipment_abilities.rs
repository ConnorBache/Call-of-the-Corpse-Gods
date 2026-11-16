//! Stable learned/equipment/Remember ownership and dormant budget retention.
//! Resource settlement, terminal death and flight loss are caller-owned phases.
use crate::{Item, Run, engine, progression};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EquipmentAbilityState {
    pub base: Vec<String>,
    pub grants: Vec<String>,
    pub order: Vec<String>,
    pub abilities: Vec<String>,
    pub active: Vec<String>,
}
fn unique(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .filter(|s| seen.insert(s.clone()))
        .collect()
}
fn equipped(i: &Item) -> bool {
    if !matches!(i.kind.as_str(), "weapon" | "armor" | "ring" | "amulet") {
        return false;
    }
    let bow = i.kind == "weapon"
        && i.ranged_weapon
            .as_ref()
            .is_some_and(|s| matches!(s.as_str(), "bow" | "composite-bow"));
    if bow {
        i.hands.value_or(0.0) == 2.0 && i.equipped.as_ref().is_some_and(|s| s == "hand1")
    } else {
        i.equipped.as_ref().is_some_and(|s| !s.is_empty())
    }
}
pub fn equipment_ability_state(r: &Run, inventory: &[Item]) -> EquipmentAbilityState {
    let p = &r.player;
    let previous: HashSet<&str> = p
        .equipment_abilities
        .as_ref()
        .map(|ids| ids.iter().map(String::as_str).collect())
        .unwrap_or_default();
    let base: Vec<String> = p
        .abilities
        .iter()
        .filter(|id| {
            !previous.contains(id.as_str())
                || p.conditions
                    .get(&format!("remembered:{id}"))
                    .is_some_and(|n| *n > r.tick)
        })
        .cloned()
        .collect();
    let grants = unique(
        inventory
            .iter()
            .filter(|i| if crate::revision50::enabled(r) { crate::revision50_inventory::worn(i) } else { equipped(i) })
            .flat_map(|i| i.granted_ability.as_ref().into_iter().chain(i.additional_granted_abilities.iter()))
            .filter(|id| {
                progression::catalog_for_revision(r.simulation_revision.value_or(29.0))
                    .iter()
                    .any(|a| a.id.as_str() == id.as_str())
            })
            .cloned(),
    );
    let next = unique(base.iter().chain(&grants).cloned());
    let next_set: HashSet<&str> = next.iter().map(String::as_str).collect();
    let order = unique(
        p.ability_order
            .as_ref()
            .unwrap_or(&p.abilities)
            .iter()
            .cloned()
            .chain(next.iter().cloned()),
    );
    let abilities = order
        .iter()
        .filter(|id| next_set.contains(id.as_str()))
        .cloned()
        .collect();
    let active = p
        .active
        .iter()
        .filter(|id| next_set.contains(id.as_str()))
        .cloned()
        .collect();
    EquipmentAbilityState {
        base,
        grants,
        order,
        abilities,
        active,
    }
}
pub fn learned_abilities(r: &Run) -> Vec<String> {
    let equipment: HashSet<&str> = r
        .player
        .equipment_abilities
        .as_ref()
        .map(|ids| ids.iter().map(String::as_str).collect())
        .unwrap_or_default();
    r.player
        .abilities
        .iter()
        .filter(|id| {
            !equipment.contains(id.as_str())
                && !r
                    .player
                    .conditions
                    .get(&format!("remembered:{id}"))
                    .is_some_and(|n| *n > r.tick)
        })
        .cloned()
        .collect()
}
fn remove_linked_effect(r: &mut Run, id: &str, settle_environment: bool) -> bool {
    let was_active = r.player.active.iter().any(|s| s == id);
    let player_id = r.player.id;
    crate::maintained_links::clear_maintained_link(r, player_id, id);
    r.player.active.retain(|s| s != id);
    let keys: &[&str] = match id {
        "lichor-blood-weapon" => &["bloodWeaponReady"],
        "ursa-dream-of-time-gone-by" => &["dreamX", "dreamY", "dreamDepth"],
        _ => &[],
    };
    for key in keys {
        r.player.conditions.shift_remove(*key);
    }
    if id == "kative-mini-me" {
        if let Some(target) = r.player.conditions.get("miniMeId").copied() {
            if r.actors
                .values()
                .any(|actors| actors.iter().any(|a| a.id == target))
            {
                crate::maintained_links::end_maintained_links_to_dead_target(r, target);
            }
            for actors in r.actors.values_mut() {
                if actors.iter().any(|a| a.id == target) {
                    crate::mutation_journal::make_mut(actors).retain(|a| a.id != target);
                }
            }
        }
        r.player.conditions.shift_remove("miniMeId");
    }
    settle_environment
        && was_active
        && id == "etill-phase-through"
        && crate::divine_environment::phase_crushing_endpoint(r).is_some()
}
/// Source ownership phase only. Equip calls true, then settles *all* resources,
/// death and clone/player falls in source order. Load normalization calls false.
pub fn reconcile(r: &mut Run, settle_environment: bool) -> Result<bool, String> {
    crate::revision50_ability_energy::reconcile_acquired(r)
        .map_err(|e| format!("Blessing acquisition: {e:?}"))?;
    let previous = unique(r.player.equipment_abilities.value_or(Vec::new()));
    let reactions_before = engine::max_reactions_for_revision(r.simulation_revision.value_or(29.), &r.player);
    let state = equipment_ability_state(r, &r.inventory);
    r.player.ability_order = state.order.into();
    r.player.abilities = state.abilities;
    let mut crushed = false;
    for id in previous {
        if r.player.abilities.contains(&id) {
            continue;
        }
        r.player
            .conditions
            .insert(format!("unequipped:{id}"), r.tick);
        crushed = remove_linked_effect(r, &id, settle_environment) || crushed;
    }
    for id in &state.grants {
        if let Some(absent_at) = r
            .player
            .conditions
            .get(&format!("unequipped:{id}"))
            .copied()
        {
            if id == "ursa-waking-nightmare" {
                if let Some(clock) = r.player.conditions.get_mut("nightmareAt") {
                    *clock += r.tick - absent_at;
                }
            }
            r.player
                .conditions
                .shift_remove(&format!("unequipped:{id}"));
        }
        progression::initialize_ability_state_for_revision(r.simulation_revision.value_or(29.), &mut r.player, id, r.tick)?;
    }
    r.player.equipment_abilities = state
        .grants
        .into_iter()
        .filter(|id| !state.base.contains(id))
        .collect::<Vec<_>>()
        .into();
    let reactions_after = engine::max_reactions_for_revision(r.simulation_revision.value_or(29.), &r.player);
    r.player.reactions = (r.player.reactions.value_or(0.0)
        - (reactions_before - reactions_after).max(0.0))
    .min(reactions_after)
    .max(0.0)
    .into();
    Ok(crushed)
}
/// Promotion changes ownership without refilling existing counters. Caller follows
/// reconciliation with its ordinary environment/health settlement boundary.
pub fn promote_learned(r: &mut Run, id: &str) -> Result<bool, String> {
    if crate::revision50::enabled(r) {
        crate::revision50_ability_energy::acquire_for_revision(&mut r.player, &[id.into()], r.simulation_revision.value_or(29.))
            .map_err(|e| format!("Blessing acquisition: {e:?}"))?;
    }
    let ids = r.player.equipment_abilities.get_or_insert_with(Vec::new);
    ids.retain(|s| s != id);
    r.player
        .conditions
        .shift_remove(&format!("remembered:{id}"));
    if !r.player.abilities.iter().any(|s| s == id) {
        r.player.abilities.push(id.into());
    }
    progression::initialize_ability_state_for_revision(r.simulation_revision.value_or(29.), &mut r.player, id, r.tick)?;
    reconcile(r, true)
}
