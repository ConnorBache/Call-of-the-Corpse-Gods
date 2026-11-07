//! Source-ordered persistence normalization. Called only on detached load state.
use crate::{
    Field, Item, Run,
    storage::{Result, StorageError},
    temporal::Snapshot,
};
use serde_json::{Value, json};
use std::collections::HashMap;
fn err(e: String) -> StorageError {
    StorageError(e)
}
fn is_bow(i: &Item) -> bool {
    i.kind == "weapon"
        && i.ranged_weapon
            .as_ref()
            .is_some_and(|s| matches!(s.as_str(), "bow" | "composite-bow"))
}
fn for_items(r: &mut Run, mut f: impl FnMut(&mut Item)) {
    for i in &mut r.inventory {
        f(i)
    }
    for entries in r.ground.values_mut() {
        for e in crate::mutation_journal::make_mut(entries) {
            f(&mut e.item)
        }
    }
    if let Some(map) = r.remembered_items.as_mut() {
        for entries in map.values_mut() {
            for e in entries {
                f(&mut e.item)
            }
        }
    }
    if let Some(map) = r.detected_items.as_mut() {
        for entries in map.values_mut() {
            for e in entries {
                f(&mut e.item)
            }
        }
    }
}
pub fn normalize_bows(r: &mut Run) -> Result<bool> {
    let valid: Vec<bool> = r
        .inventory
        .iter()
        .enumerate()
        .map(|(n, i)| {
            is_bow(i)
                && i.hands.as_ref() == Some(&2.0)
                && i.equipped.as_ref().is_some_and(|s| s == "hand1")
                && !r.inventory.iter().enumerate().any(|(m, o)| {
                    m != n
                        && o.equipped
                            .as_ref()
                            .is_some_and(|s| s == "hand1" || s == "hand2")
                })
        })
        .collect();
    let mut changed = false;
    let mut stale = false;
    fn fix(i: &mut Item, valid: bool) -> (bool, bool) {
        if !is_bow(i) {
            return (false, false);
        }
        let stale = !i.equipped.is_missing() && !valid;
        let changed = stale || i.hands.as_ref() != Some(&2.0);
        if stale {
            i.equipped = Field::Missing;
        }
        i.hands = Field::Value(2.0);
        (changed, stale)
    }
    for (i, valid) in r.inventory.iter_mut().zip(valid) {
        let (c, s) = fix(i, valid);
        changed |= c;
        stale |= s;
    }
    // Inventory has already been handled; process other ownership locations separately.
    for entries in r.ground.values_mut() {
        for e in crate::mutation_journal::make_mut(entries) {
            changed |= fix(&mut e.item, false).0;
        }
    }
    if let Some(map) = r.remembered_items.as_mut() {
        for entries in map.values_mut() {
            for e in entries {
                changed |= fix(&mut e.item, false).0;
            }
        }
    }
    if let Some(map) = r.detected_items.as_mut() {
        for entries in map.values_mut() {
            for e in entries {
                changed |= fix(&mut e.item, false).0;
            }
        }
    }
    for list in r.actors.values_mut() {
        for a in crate::mutation_journal::make_mut(list) {
            if let Some(items) = a
                .monster_state
                .as_mut()
                .and_then(|m| m.carried_items.as_mut())
            {
                for i in items {
                    changed |= fix(i, false).0;
                }
            }
        }
    }
    if stale {
        crate::equipment_abilities::reconcile(r, false).map_err(err)?;
        r.player.max_hp =
            crate::migrations::maximum_health_for_revision(&r.player, true, &r.player, &r.inventory, r.simulation_revision.value_or(29.0));
        r.player.hp = r.player.hp.min(r.player.max_hp);
        if !r
            .player
            .abilities
            .iter()
            .any(|id| id == "grisha-boundless-hunger")
        {
            r.player.energy = r
                .player
                .energy
                .min(crate::combat::effective_stats(r, &r.player).max_energy);
        }
        crate::half_here::sync(r);
    }
    Ok(changed)
}
fn amount(s: &str) -> bool {
    let mut split = s.split('.');
    let a = split.next().unwrap_or("");
    !a.is_empty()
        && a.bytes().all(|b| b.is_ascii_digit())
        && split
            .next()
            .is_none_or(|b| !b.is_empty() && b.bytes().all(|b| b.is_ascii_digit()))
        && split.next().is_none()
}
fn stat_modifier(s: &str, sign: Option<char>) -> bool {
    let Some((n, k)) = s.split_once(' ') else {
        return false;
    };
    let Some(c) = n.chars().next() else {
        return false;
    };
    matches!(c, '+' | '-')
        && sign.is_none_or(|s| s == c)
        && amount(&n[1..])
        && [
            "offense",
            "defense",
            "accuracy",
            "dodge",
            "dr",
            "perception",
            "maxEnergy",
            "maxHp",
        ]
        .contains(&k)
}
fn plate_name(s: &str) -> bool {
    let Some(s) = s
        .strip_prefix("Plate Armor")
        .or_else(|| s.strip_prefix("Plate armor"))
    else {
        return false;
    };
    if s.is_empty() {
        return true;
    }
    if let Some(s) = s.strip_prefix(" of ") {
        return ["double", "triple", "quadruple", "quintuple", "sextuple"].contains(&s)
            || stat_modifier(s, None);
    }
    if let Some(s) = s.strip_prefix(", Cursed of ") {
        return s
            .split_once(" and ")
            .is_some_and(|(a, b)| stat_modifier(a, Some('+')) && stat_modifier(b, Some('-')));
    }
    false
}
fn normalize_plate(i: &mut Item) {
    if i.kind != "armor" || !i.tinkered.value_or(false) {
        return;
    }
    let Some(perk) = i
        .granted_ability
        .as_ref()
        .and_then(|id| crate::progression::definition(id))
        .map(|a| a.name.clone())
    else {
        return;
    };
    let mut prior = i.name.as_str();
    for suffix in [format!(" of {perk}"), format!(" and {perk}")] {
        if let Some(s) = prior.strip_suffix(&suffix) {
            prior = s;
            break;
        }
    }
    if !plate_name(prior) {
        return;
    }
    if i.name.starts_with("Plate armor") {
        i.name.replace_range(..11, "Plate Armor");
    }
    if !i.name.ends_with(&format!(" of {perk}")) && !i.name.ends_with(&format!(" and {perk}")) {
        i.name += &format!(
            "{}{}",
            if i.name.contains(" of ") {
                " and "
            } else {
                " of "
            },
            perk
        );
    }
}
fn replace_ascii_ignore_case(s: &str, needle: &str, replacement: &str) -> String {
    let lower = s.to_ascii_lowercase();
    let mut out = String::new();
    let mut at = 0;
    while let Some(pos) = lower[at..].find(needle) {
        let pos = at + pos;
        out.push_str(&s[at..pos]);
        out.push_str(replacement);
        at = pos + needle.len();
    }
    out.push_str(&s[at..]);
    out
}
fn has_spitter_word(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    lower.match_indices("pale spitter").any(|(i, _)| {
        let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
        (i == 0 || !word(lower.as_bytes()[i - 1]))
            && (i + 12 == lower.len() || !word(lower.as_bytes()[i + 12]))
    })
}
fn js_space(c: char) -> bool {
    matches!(c, '\u{0009}'..='\u{000d}' | ' ' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}
fn legacy_heart_type(i: &Item) -> Option<String> {
    if i.kind != "heart" {
        return None;
    }
    if i.heart_creature_type
        .as_ref()
        .is_some_and(|s| !s.is_empty())
    {
        return crate::inventory::heart_creature_type(i);
    }
    let prefix = i
        .name
        .strip_suffix("<heart>")
        .or_else(|| i.name.strip_suffix("heart"))?;
    if !prefix.chars().last().is_some_and(js_space) {
        return None;
    }
    let prefix = prefix.trim_end_matches(js_space);
    // JS dot does not cross line terminators; non-JS whitespace is part of identity.
    if prefix
        .chars()
        .any(|c| matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
        || prefix
            .chars()
            .last()
            .is_some_and(|c| c.is_whitespace() && !js_space(c))
    {
        return None;
    }
    let mut item = i.clone();
    item.name = format!("{prefix} heart");
    crate::inventory::heart_creature_type(&item)
}
fn normalize_items(r: &mut Run, divine: bool) {
    for_items(r, |i| {
        normalize_plate(i);
        if divine {
            if let Some(kind) = legacy_heart_type(i) {
                i.heart_creature_type = Field::Value(kind);
            }
        }
        if i.kind == "heart" {
            i.name = i.name.replacen("Vent Fungus", "Bastidion", 1);
            if has_spitter_word(&i.name) {
                if let Some(b) = i.bonus.as_mut() {
                    if let Some(p) = b.perception.as_ref().copied() {
                        b.accuracy = Field::Value(b.accuracy.value_or(0.0) + p);
                        b.perception = Field::Missing;
                        if let Some(e) = i.effect.as_mut() {
                            *e = replace_ascii_ignore_case(e, "perception", "accuracy");
                        }
                    }
                }
            }
            i.glyph = ";".into();
        }
    });
}
fn revisions(r: &mut Run) {
    if r.gameplay_revision.value_or(1.0) < 2.0 {
        for list in r.actors.values_mut() {
            for a in crate::mutation_journal::make_mut(list) {
                if a.name.ends_with("Pale Birther")
                    && (a.name.len() == 12
                        || (!a.name.as_bytes()[a.name.len() - 13].is_ascii_alphanumeric()
                            && a.name.as_bytes()[a.name.len() - 13] != b'_'))
                    && !a.abilities.iter().any(|s| s == "enemy:pale:slow")
                {
                    a.abilities.push("enemy:pale:slow".into());
                }
            }
        }
        let mut inventory: Vec<Item> = vec![];
        for i in std::mem::take(&mut r.inventory) {
            let found = crate::inventory::stack_key(&i).and_then(|key| {
                inventory
                    .iter()
                    .position(|p| crate::inventory::stack_key(p).as_ref() == Some(&key))
            });
            if let Some(n) = found {
                inventory[n].quantity =
                    Field::Value(inventory[n].quantity.value_or(1.0) + i.quantity.value_or(1.0));
            } else {
                inventory.push(i)
            }
        }
        r.inventory = inventory;
        r.gameplay_revision = Field::Value(2.0);
    }
    if r.gameplay_revision.value_or(1.0) < 3.0 {
        for list in r.actors.values_mut() {
            for a in crate::mutation_journal::make_mut(list) {
                if (a.creature_kind.as_ref().is_some_and(|s| s == "zillennia")
                    || crate::enemy_traits::catalog()
                        .zillennia_names
                        .iter()
                        .any(|n| n == crate::enemy_traits::base_name(&a.name)))
                    && !a
                        .abilities
                        .iter()
                        .any(|id| id == "enemy:beast:darkness-aura")
                {
                    a.abilities.push("enemy:beast:darkness-aura".into());
                }
            }
        }
        r.gameplay_revision = Field::Value(3.0);
    }
}
/// Core ordered transform; timeline/anchor orchestration calls this independently per state.
pub fn normalize_world(r: &mut Run) -> Result<()> {
    let divine = r.divine_revision.value_or(0.0) < 1.0;
    crate::migrations::migrate_divine_revision(r);
    normalize_bows(r)?;
    crate::unique_composite_bow::initialize(r);
    crate::migrations::migrate_generated_ability_grants(r);
    for floor in &mut r.floors {
        if let Some(e) = crate::mutation_journal::make_mut(floor).environment.as_mut() {
            crate::environment::normalize(e).map_err(err)?;
            crate::environment::refresh_gas_views(e);
            crate::environment::refresh_fire_views(e);
        }
    }
    normalize_items(r, divine);
    for list in r.actors.values_mut() {
        for a in crate::mutation_journal::make_mut(list) {
            if let Some(items) = a
                .monster_state
                .as_mut()
                .and_then(|m| m.carried_items.as_mut())
            {
                for i in items {
                    normalize_plate(i)
                }
            }
            crate::migrations::migrate_bastidion(a);
            crate::enemy_traits::normalize_presentation(a);
            if let Some(p) = a.monster_state.as_mut().and_then(|m| m.pilot.as_mut()) {
                crate::enemy_traits::normalize_presentation(p);
            }
        }
    }
    crate::migrations::migrate_enemy_health_revision(r);
    crate::migrations::normalize_integer_stat_health(r);
    crate::maintained_links::normalize_dead_maintained_links(r);
    revisions(r);
    let ensure = |r: &mut Run, d: f64| crate::startup::ensure_floors(r, d as u32);
    crate::item_placement::normalize_ground_items(r, ensure).map_err(err)?;
    crate::corpse_placement::normalize_corpses(r, ensure).map_err(err)?;
    crate::revision50_creature_identity::normalize_world(r);
    crate::fungi::normalize_world(r);
    Ok(())
}
#[derive(Default)]
pub struct Context {
    anchors: HashMap<String, String>,
}
fn restore_pending(r: &mut Run) -> Result<()> {
    if let Some(raw) = r.haste_snapshot.as_ref().filter(|s| !s.is_empty()) {
        let snap = Snapshot::from_value(serde_json::from_str(raw)?)?;
        snap.restore(r)?;
    }
    Ok(())
}
fn normalize_state(r: &mut Run, context: &mut Context, depth: usize) -> Result<bool> {
    if depth > 32 {
        return Err(StorageError(
            "Legacy save anchor nesting exceeds native admission limit32".into(),
        ));
    }
    crate::storage::validate(r)?;
    let before = crate::storage::canonical_value(r)?;
    normalize_world(r)?;
    if let Some(anchor) = r.future.clone() {
        let raw=anchor.wire_text()?.to_owned();
        if raw.is_empty() {
            r.future = None;
        } else if let Some(cached) = context.anchors.get(&raw) {
            r.future = Some(cached.clone().into());
        } else {
            let normalized = crate::storage::decode_compatible_with(&raw, context, depth + 1)?;
            let encoded = crate::storage::serialize(&normalized)?;
            context.anchors.insert(raw, encoded.clone());
            r.future = Some(encoded.into());
        }
    }
    if let Some(raw) = r.haste_snapshot.clone().as_ref().filter(|s| !s.is_empty()) {
        let opening: Value = serde_json::from_str(raw)?;
        let object = opening
            .as_object()
            .ok_or_else(|| StorageError("Invalid haste opening".into()))?;
        let mut scaffold = crate::storage::canonical_value(r)?;
        for (k, v) in object {
            scaffold[k] = v.clone();
        }
        scaffold["gameplayRevision"] = object
            .get("gameplayRevision")
            .cloned()
            .filter(|v| !v.is_null())
            .unwrap_or(json!(1));
        for key in ["divineRevision", "enemyHealthRevision", "runStats"] {
            if !object.contains_key(key) {
                scaffold.as_object_mut().unwrap().shift_remove(key);
            }
        }
        scaffold["history"] = json!([]);
        scaffold["replay"] = json!([]);
        scaffold
            .as_object_mut()
            .unwrap()
            .shift_remove("hasteSnapshot");
        if let Some(cursor) = object.get("logArchiveCursor") {
            let cursor: crate::temporal::ArchiveCursor = serde_json::from_value(cursor.clone())?;
            let archive = r
                .log_archive
                .as_ref()
                .ok_or_else(|| StorageError("Missing haste archive".into()))?;
            if cursor.messages > archive.messages.len() || cursor.events > archive.events.len() {
                return Err(StorageError("Invalid haste archive cursor".into()));
            }
            let mut archive = archive.clone();
            archive.messages.truncate(cursor.messages);
            archive.events.truncate(cursor.events);
            scaffold["logArchive"] = serde_json::to_value(archive)?;
            scaffold
                .as_object_mut()
                .unwrap()
                .shift_remove("logArchiveCursor");
        } else if !object.contains_key("logArchive") {
            scaffold.as_object_mut().unwrap().shift_remove("logArchive");
        }
        let mut scaffold: Run = serde_json::from_value(scaffold)?;
        if normalize_state(&mut scaffold, context, depth + 1)? {
            // Source migration snapshot retains runId; ordinary gameplay snapshots omit it.
            let mut value = Snapshot::capture(&scaffold).value()?;
            if !scaffold.run_id.is_missing() {
                value["runId"] = serde_json::to_value(&scaffold.run_id)?;
            }
            r.haste_snapshot = Field::Value(crate::temporal::js_stringify(&value));
        }
    }
    crate::storage::validate(r)?;
    Ok(!crate::storage::canonical_equal(
        &before,
        &crate::storage::canonical_value(r)?,
    ))
}
/// Reconstruct original sparse history before normalizing independently. O(H*S), H<=5.
pub fn migrate_timeline(r: &mut Run, context: &mut Context, depth: usize) -> Result<()> {
    // Revision41 has no historical timeline to rebuild. Explicit Future anchors
    // still receive ordinary normalization, O(current state + anchor state).
    if !crate::temporal::retains_revert_history(r) {
        crate::temporal::validate_retired_history(r)?;
        normalize_state(r, context, depth)?;
        return Ok(());
    }
    let mut original = r.clone();
    let mut changed = normalize_state(r, context, depth)?;
    let mut newer = r.clone();
    if newer.haste_snapshot.as_ref().is_some_and(|s| !s.is_empty()) {
        restore_pending(&mut newer)?;
        changed |= normalize_state(&mut newer, context, depth)?;
    }
    let mut rebuilt = r.history.clone();
    restore_pending(&mut original)?;
    for i in (0..original.history.len()).rev() {
        let frame: crate::temporal::Frame =
            serde_json::from_str(original.history.get(i).ok_or_else(|| {
                StorageError("Legacy inverse mutated the history ledger".into())
            })?)?;
        let mut value = crate::storage::canonical_value(&original)?;
        for group in frame.groups.iter().rev() {
            crate::temporal::apply(&mut value, group)?;
        }
        value["future"] = frame.future.map(Value::String).unwrap_or(Value::Null);
        original = serde_json::from_value(value)?;
        let mut earlier = original.clone();
        changed |= normalize_state(&mut earlier, context, depth)?;
        let patches = crate::temporal::inverse(
            &Snapshot::capture(&earlier),
            &Snapshot::capture(&newer),
            true,
        )?;
        rebuilt[i] =
            crate::temporal::js_stringify(&serde_json::to_value(crate::temporal::Frame {
                groups: vec![patches],
                future: crate::future_anchor::wire_option(&earlier.future)?,
            })?);
        newer = earlier;
    }
    if changed {
        r.history = rebuilt;
    }
    Ok(())
}
