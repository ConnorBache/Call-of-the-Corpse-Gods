//! Individually ported load-time transformations. Full migration admission remains gated
//! until the entire ordered v48 normalization pipeline is available and verified.
use crate::{Actor, Field, Run};
fn stat(n: f64) -> f64 {
    n.signum() * (n.abs() + 0.5).floor() + 0.0
}
fn enemy_defense(n: f64) -> f64 {
    (n + 0.5).floor() + 0.0
}
fn enemy_health(d: f64) -> f64 {
    100.0 + 6.0 * enemy_defense(d)
}
/// Exact missing-health preservation, temporary buffs and player-copy exclusions.
/// O(all actors + nested pilots), no RNG/log/resource event changes.
pub fn migrate_enemy_health_revision(run: &mut Run) -> bool {
    if run.enemy_health_revision.value_or(0.0) >= 1.0 {
        return false;
    }
    fn migrate(a: &mut Actor) {
        if a.conditions.get("miniMe") != Some(&1.0)
            && a.conditions.get("halfHereClone") != Some(&1.0)
        {
            let missing = (a.max_hp - a.hp).max(0.0);
            let bd = a.conditions.get("buff:defense").copied().unwrap_or(0.0);
            let bh = a.conditions.get("buff:maxHp").copied().unwrap_or(0.0);
            a.stats.defense = enemy_defense((a.max_hp - 6.0 * bd - bh - 100.0) / 6.0).max(-16.0);
            a.stats.max_hp = Field::Missing;
            a.base_max_hp = Field::Missing;
            if a.conditions.contains_key("buff:defense") {
                a.conditions.insert("buff:defense".into(), stat(bd));
            }
            a.max_hp = enemy_health(
                a.stats.defense
                    + a.conditions.get("buff:defense").copied().unwrap_or(0.0)
                    + stat(stat(bh) / 6.0),
            );
            if !a.dead.value_or(false) && a.hp > 0.0 {
                a.hp = (a.max_hp - missing).min(a.max_hp).max(1.0);
            }
        }
        if let Some(p) = a.monster_state.as_mut().and_then(|m| m.pilot.as_mut()) {
            migrate(p);
        }
    }
    for actors in run.actors.values_mut() {
        for a in crate::mutation_journal::make_mut(actors) {
            migrate(a);
        }
    }
    run.enemy_health_revision = Field::Value(1.0);
    true
}
/// Generated-item ledger migration preserves first grant by ascending stable item ID.
pub fn migrate_generated_ability_grants(run: &mut Run) -> bool {
    if run.generated_ability_grants.as_ref().is_some() {
        return false;
    }
    let mut items: Vec<_> = run
        .inventory
        .iter()
        .chain(
            run.ground
                .values()
                .flat_map(|entries| entries.iter().map(|e| &e.item)),
        )
        .collect();
    items.sort_by(|a, b| a.id.total_cmp(&b.id));
    let mut grants = Vec::new();
    for i in items {
        if let Some(id) = i.granted_ability.as_ref() {
            if !grants.contains(id) {
                grants.push(id.clone());
            }
        }
    }
    run.generated_ability_grants = Field::Value(grants);
    true
}
/// Generic historical rebasing for independently verified normalization functions.
/// It normalizes each original timeline state, then reconstructs inverse groups.
/// O(H*S + normalization), H<=5, boundary-only JSON and COW block detachments.
pub fn rebase_history(
    run: &mut Run,
    normalize: fn(&mut Run) -> bool,
) -> crate::storage::Result<bool> {
    let history = crate::temporal::History::import(run)?;
    let mut changed = normalize(run);
    let mut rebuilt = history;
    for frame in &mut rebuilt.frames {
        changed |= normalize(crate::mutation_journal::make_mut(&mut frame.state));
    }
    if let Some(end) = &mut rebuilt.closing {
        changed |= normalize(crate::mutation_journal::make_mut(&mut end.state));
    }
    if let Some(p) = &mut rebuilt.pending_haste {
        changed |= normalize(crate::mutation_journal::make_mut(&mut p.state));
    }
    if changed {
        let out = rebuilt.export(run)?;
        run.history = out.history;
        run.haste_snapshot = out.haste_snapshot;
    }
    Ok(changed)
}

/// Legacy identity is the one-time marker. Preserves corpse state and health ratio.
/// O(ability count), no RNG or temporal boundary effects.
pub fn migrate_bastidion(actor: &mut Actor) -> bool {
    if ![
        "Vent Fungus",
        "Mercuria’s Vent Fungus",
        "Mercuria's Vent Fungus",
    ]
    .contains(&actor.name.as_str())
    {
        return false;
    }
    actor.name = actor.name.replace("Vent Fungus", "Bastidion");
    actor.hp *= 4.0;
    actor.max_hp *= 4.0;
    if let Some(base) = actor.base_max_hp.as_mut() {
        *base *= 4.0;
    }
    actor.stats.dr = 8.0;
    for id in ["enemy:beast:gas-immunity", "enemy:beast:no-revival"] {
        if !actor.abilities.iter().any(|s| s == id) {
            actor.abilities.push(id.into());
        }
    }
    true
}

fn equipped(item: &crate::Item, revision: f64) -> bool {
    if revision >= 50.0 { return crate::revision50_inventory::worn(item); }
    if !["weapon", "armor", "ring", "amulet"].contains(&item.kind.as_str()) {
        return false;
    }
    if item.kind == "weapon"
        && item
            .ranged_weapon
            .as_ref()
            .is_some_and(|s| s == "bow" || s == "composite-bow")
    {
        item.hands.as_ref() == Some(&2.0) && item.equipped.as_ref().is_some_and(|s| s == "hand1")
    } else {
        item.equipped.as_ref().is_some_and(|s| !s.is_empty())
    }
}
fn copy_body(a: &Actor) -> bool {
    a.conditions.get("miniMe") == Some(&1.0) || a.conditions.get("halfHereClone") == Some(&1.0)
}
fn shared_owner(a: &Actor, player: &Actor) -> bool {
    a.conditions.get("halfHereClone") == Some(&1.0)
        && a.conditions.get("halfHereOwner") == Some(&player.id)
}
/// Revision-aware health-only projection, avoiding combat side effects. R50
/// carried jewelry contributes without rewriting historical equipped flags.
pub(crate) fn maximum_health_for_revision(
    a: &Actor,
    is_player: bool,
    player: &Actor,
    inventory: &[crate::Item],
    revision: f64,
) -> f64 {
    if !is_player && shared_owner(a, player) {
        return maximum_health_for_revision(player, true, player, inventory, revision);
    }
    let mut defense =
        stat(a.stats.defense) + stat(a.conditions.get("buff:defense").copied().unwrap_or(0.0));
    let mut hp = stat(a.stats.max_hp.value_or(0.0))
        + stat(a.conditions.get("buff:maxHp").copied().unwrap_or(0.0));
    if is_player {
        for item in inventory.iter().filter(|i| equipped(i, revision)) {
            if let Some(bonus) = item.bonus.as_ref() {
                for (value, base, dest) in [
                    (
                        &bonus.defense,
                        item.base_bonus.as_ref().map(|b| &b.defense),
                        &mut defense,
                    ),
                    (
                        &bonus.max_hp,
                        item.base_bonus.as_ref().map(|b| &b.max_hp),
                        &mut hp,
                    ),
                ] {
                    if let Some(value) = value.as_ref() {
                        *dest += base.and_then(|b| b.as_ref()).map_or_else(
                            || stat(*value),
                            |base| stat(*base) + stat(*value - *base),
                        );
                    }
                }
            }
            for e in &item.enhancements {
                defense += stat(e.bonus.defense.value_or(0.0));
                hp += stat(e.bonus.max_hp.value_or(0.0));
            }
        }
    }
    if !is_player && !copy_body(a) {
        return enemy_health(defense + stat(hp / 6.0));
    }
    let baseline = a.base_max_hp.value_or(if is_player {
        100.0
    } else {
        a.max_hp - 6.0 * a.stats.defense - a.stats.max_hp.value_or(0.0)
    });
    (baseline + 6.0 * defense + hp).max(1.0)
}
/// Source divine-revision migration. Enemy caps wait for enemy-health migration.
/// O(A*(ability count + equipped items)), touches no logs, RNG or resource history.
pub fn migrate_divine_revision(run: &mut Run) -> bool {
    if run.divine_revision.value_or(0.0) >= 1.0 {
        return false;
    }
    fn actor(a: &mut Actor, is_player: bool, player: &Actor, inventory: &[crate::Item], revision: f64) {
        if is_player || copy_body(a) {
            if a.base_max_hp.as_ref().is_none() {
                a.base_max_hp = Field::Value(if is_player {
                    100.0
                } else {
                    a.max_hp - 6.0 * a.stats.defense - a.stats.max_hp.value_or(0.0)
                });
            }
            a.max_hp = maximum_health_for_revision(a, is_player, player, inventory, revision);
        }
        if !a.dead.value_or(false) && a.hp > 0.0 {
            a.hp = a.hp.min(a.max_hp);
        }
        a.adaptive_dr = Field::Missing;
        a.conditions.shift_remove("adaptiveAt");
        if a.abilities.iter().any(|id| id == "isheilah-quick-hand") {
            let cap = [
                "isheilah-missed-me",
                "isheilah-are-you-sure-about-that",
                "isheilah-quick-hand",
            ]
            .iter()
            .filter(|id| a.abilities.iter().any(|v| v == **id))
            .count() as f64;
            a.reactions = Field::Value((a.reactions.value_or(0.0) + 1.0).min(cap));
        }
    }
    let prior = run.player.clone();
    let revision = run.simulation_revision.value_or(29.0);
    actor(&mut run.player, true, &prior, &run.inventory, revision);
    for list in run.actors.values_mut() {
        for a in crate::mutation_journal::make_mut(list) {
            actor(a, false, &run.player, &run.inventory, revision);
        }
    }
    if let Some(initial) = run.initial_player.as_mut() {
        let prior = initial.clone();
        actor(initial, true, &prior, &[], revision);
    }
    run.divine_revision = Field::Value(1.0);
    true
}

/// Retain raw fractional provenance, updating only health whose projection changes.
/// O(actors * (equipped items + enhancements)), recursively includes nested pilots.
pub fn normalize_integer_stat_health(run: &mut Run) -> bool {
    fn actor(
        a: &mut Actor,
        is_player: bool,
        player: &mut Actor,
        inventory: &[crate::Item],
        revision: f64,
    ) -> bool {
        let shared = !is_player && shared_owner(a, player);
        let owner = if shared { player.clone() } else { a.clone() };
        let owner_is_player = is_player || shared;
        let enemy = !owner_is_player && !copy_body(&owner);
        let mut sources = vec![
            (
                Some(owner.stats.defense),
                owner.stats.max_hp.as_ref().copied(),
            ),
            (
                Some(owner.conditions.get("buff:defense").copied().unwrap_or(0.0)),
                Some(owner.conditions.get("buff:maxHp").copied().unwrap_or(0.0)),
            ),
        ];
        if owner_is_player {
            for item in inventory.iter().filter(|i| equipped(i, revision)) {
                for b in item
                    .bonus
                    .as_ref()
                    .into_iter()
                    .chain(item.base_bonus.as_ref())
                    .chain(item.enhancements.iter().map(|e| &e.bonus))
                {
                    sources.push((b.defense.as_ref().copied(), b.max_hp.as_ref().copied()));
                }
            }
        }
        let fractional = sources
            .iter()
            .any(|(d, h)| d.into_iter().chain(h).any(|n| n.fract() != 0.0));
        let flat = enemy
            && sources
                .iter()
                .any(|(_, h)| h.is_some_and(|n| (stat(n) / 6.0).fract() != 0.0));
        let mut changed = false;
        if fractional || flat {
            let baseline = owner.base_max_hp.value_or(if owner_is_player {
                100.0
            } else {
                owner.max_hp - 6.0 * owner.stats.defense - owner.stats.max_hp.value_or(0.0)
            });
            let max = maximum_health_for_revision(a, is_player, player, inventory, revision);
            if max != a.max_hp {
                if !enemy && owner.base_max_hp.is_missing() {
                    if shared {
                        player.base_max_hp = Field::Value(baseline);
                    } else {
                        a.base_max_hp = Field::Value(baseline);
                    }
                }
                a.max_hp = max;
                if !a.dead.value_or(false) && a.hp > 0.0 {
                    a.hp = if shared { player.hp } else { a.hp.min(max) };
                }
                changed = true;
            }
        }
        if let Some(pilot) = a.monster_state.as_mut().and_then(|m| m.pilot.as_mut()) {
            changed |= actor(pilot, false, player, inventory, revision);
        }
        changed
    }
    let mut context = run.player.clone();
    let revision = run.simulation_revision.value_or(29.0);
    let mut changed = actor(&mut run.player, true, &mut context, &run.inventory, revision);
    for list in run.actors.values_mut() {
        for a in crate::mutation_journal::make_mut(list) {
            changed |= actor(a, false, &mut run.player, &run.inventory, revision);
        }
    }
    if let Some(initial) = run.initial_player.as_mut() {
        let mut context = initial.clone();
        changed |= actor(initial, true, &mut context, &[], revision);
    }
    changed
}
