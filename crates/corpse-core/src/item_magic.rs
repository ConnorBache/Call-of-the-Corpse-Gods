//! Sensed magic and stale item observations; no identification or simulation RNG.
use crate::{
    Field, Item, Run, RunDetectedItemsEntryEntry as Detected,
    RunRememberedItemsEntryEntry as Remembered,
};
pub fn classify(i: &Item) -> Option<String> {
    if i.kind == "potion" {
        return i.effect.as_ref().filter(|s| !s.is_empty()).map(|s| {
            if matches!(
                s.as_str(),
                "incineration" | "caustic gas" | "paralysis" | "confusion" | "descent"
            ) {
                "bad"
            } else {
                "good"
            }
            .into()
        });
    }
    if !matches!(i.kind.as_str(), "weapon" | "armor" | "ring" | "amulet") {
        return None;
    }
    let mut copy = i.clone();
    copy.magic_nature = "good".to_string().into();
    crate::equipment::refresh_known_magic(&mut copy);
    copy.magic_nature.as_ref().cloned()
}
pub fn magical(i: &Item) -> bool {
    i.kind == "potion" || classify(i).is_some()
}
fn reveal(i: &mut Item, known: &mut Field<indexmap::IndexMap<String, String>>) {
    let Some(nature) = classify(i) else { return };
    i.magic_nature = nature.clone().into();
    if i.kind != "potion" || nature == "bizarre" {
        return;
    }
    if let Some(effect) = i.effect.as_ref().filter(|s| !s.is_empty()) {
        i.potion_nature = nature.clone().into();
        known
            .get_or_insert_with(Default::default)
            .insert(effect.clone(), nature);
    }
}
pub fn remember_detected_items(r: &mut Run) -> Result<(), String> {
    let sensitive = crate::half_here::origins(r).iter().any(|a| {
        !a.dead.value_or(false) && a.conditions.get("magicSensitive").copied().unwrap_or(0.0) > 0.0
    });
    let key = r.depth.to_string();
    let dream_view = crate::dreamwalking::dream_view(r);
    let remembered = r
        .detected_items
        .as_ref()
        .and_then(|m| m.get(&key))
        .cloned()
        .unwrap_or_default();
    if !sensitive && remembered.is_empty() {
        return Ok(());
    }
    r.detected_items.get_or_insert_with(Default::default);
    if sensitive {
        for i in &mut r.inventory {
            reveal(i, &mut r.known_potion_natures)
        }
        let mut magical_entries = vec![];
        if let Some(entries) = r.ground.get_mut(&key) {
            for entry in crate::mutation_journal::make_mut(entries) {
                if (!crate::dreamwalking::dream_ground_item(&entry.item) || dream_view) && magical(&entry.item) {
                    reveal(&mut entry.item, &mut r.known_potion_natures);
                    magical_entries.push(Detected {
                        x: entry.x,
                        y: entry.y,
                        item: entry.item.clone(),
                    });
                }
            }
        }
        let mut ordinary = indexmap::IndexMap::new();
        if let Some(entries) = r.remembered_items.as_ref().and_then(|m| m.get(&key)) {
            for entry in entries {
                ordinary.insert(entry.item.id.to_bits(), entry.clone());
            }
        }
        for entry in remembered {
            if !magical(&entry.item) {
                ordinary
                    .entry(entry.item.id.to_bits())
                    .or_insert(Remembered {
                        x: entry.x,
                        y: entry.y,
                        item: entry.item,
                    });
            }
        }
        if !ordinary.is_empty() {
            r.remembered_items
                .get_or_insert_with(Default::default)
                .insert(key.clone(), ordinary.values().cloned().collect());
        }
        r.detected_items
            .as_mut()
            .unwrap()
            .insert(key.clone(), magical_entries.clone());
        if let Some(entries) = r.remembered_items.as_mut().and_then(|m| m.get_mut(&key)) {
            entries.retain(|e| !magical(&e.item));
            entries.extend(
                magical_entries
                    .into_iter()
                    .filter(|e| ordinary.contains_key(&e.item.id.to_bits()))
                    .map(|e| Remembered {
                        x: e.x,
                        y: e.y,
                        item: e.item,
                    }),
            );
        }
        return Ok(());
    }
    let f = r.floor().ok_or("missing detection floor")?;
    let w = f.width();
    let mask = crate::vision::terrain_visibility(r);
    let visible = |x: f64, y: f64| {
        let at = (y * w as f64 + x) as usize;
        mask.get(at).copied().unwrap_or(false)
    };
    let entries = r.ground.get(&key).map(|e| e.as_slice()).unwrap_or_default();
    let observed_ids: std::collections::HashSet<_> = entries
        .iter()
        .filter(|e| (!crate::dreamwalking::dream_ground_item(&e.item) || dream_view) && visible(e.x, e.y))
        .map(|e| e.item.id.to_bits())
        .collect();
    let remembered_ids: std::collections::HashSet<_> =
        remembered.iter().map(|e| e.item.id.to_bits()).collect();
    let mut next: Vec<_> = remembered
        .into_iter()
        .filter(|e| (!crate::dreamwalking::dream_ground_item(&e.item) || dream_view) && !visible(e.x, e.y) && !observed_ids.contains(&e.item.id.to_bits()))
        .collect();
    next.extend(
        entries
            .iter()
            .filter(|e| (!crate::dreamwalking::dream_ground_item(&e.item) || dream_view) && visible(e.x, e.y) && remembered_ids.contains(&e.item.id.to_bits()))
            .map(|e| Detected {
                x: e.x,
                y: e.y,
                item: e.item.clone(),
            }),
    );
    r.detected_items.as_mut().unwrap().insert(key, next);
    Ok(())
}
#[cfg(test)]
mod tests {
    #[test]
    fn exact_magic_detection_memory() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/item-magic-v48.json");
        let cases: serde_json::Value =
            serde_json::from_reader(std::fs::File::open(path).unwrap()).unwrap();
        for c in cases.as_array().unwrap() {
            let mut r: crate::Run = serde_json::from_value(c["initial"].clone()).unwrap();
            super::remember_detected_items(&mut r).unwrap();
            assert!(crate::storage::canonical_equal(
                &serde_json::to_value(r).unwrap(),
                &c["expected"]
            ));
        }
    }
}
/// Remembered knowledge only: never consult physical hidden modifiers here.
pub fn displayed_item_magic_nature(r: &Run, i: &Item) -> Option<String> {
    i.magic_nature.as_ref().cloned().or_else(|| {
        if i.kind == "potion" {
            i.potion_nature.as_ref().cloned().or_else(|| {
                r.known_potion_natures
                    .as_ref()
                    .and_then(|m| m.get(i.effect.as_ref().map(String::as_str).unwrap_or("")))
                    .cloned()
            })
        } else {
            None
        }
    })
}
pub fn potion_nature(r: &Run, i: &Item) -> Option<String> {
    displayed_item_magic_nature(r, i).filter(|s| s != "bizarre")
}
const APPEARANCES: [&str; 18] = [
    "healing",
    "energy",
    "incineration",
    "caustic gas",
    "paralysis",
    "offense",
    "defense",
    "accuracy",
    "dodge",
    "dr",
    "perception",
    "detect magic",
    "telepathy",
    "levitation",
    "speed",
    "confusion",
    "invisibility",
    "descent",
];
const POOL: [&str; 16] = [
    "healing",
    "energy",
    "incineration",
    "caustic gas",
    "paralysis",
    "offense",
    "defense",
    "accuracy",
    "dodge",
    "detect magic",
    "telepathy",
    "levitation",
    "speed",
    "confusion",
    "invisibility",
    "descent",
];
pub fn potion_appearance(r: &Run, effect: &str) -> String {
    let mut names = vec![
        "crimson",
        "azure",
        "amber",
        "violet",
        "milky",
        "smoky",
        "emerald",
        "silver",
        "black",
        "iridescent",
        "ochre",
    ];
    let mut seed = 2166136261u32;
    for c in r.seed.chars() {
        let mut units = [0u16; 2];
        let first = c.encode_utf16(&mut units)[0];
        seed = (seed ^ first as u32).wrapping_mul(16777619);
    }
    for i in (1..names.len()).rev() {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        names.swap(i, seed as usize % (i + 1));
    }
    let mut added = ["teal", "golden", "rose", "indigo", "pearly", "chartreuse"];
    let mut suffix = seed ^ 0x9e3779b9;
    for i in (1..added.len()).rev() {
        suffix = suffix.wrapping_mul(1664525).wrapping_add(1013904223);
        added.swap(i, suffix as usize % (i + 1));
    }
    names.push("copper");
    names.extend(added);
    format!(
        "{} potion",
        names[APPEARANCES.iter().position(|s| *s == effect).unwrap_or(0)]
    )
}
pub fn potion_description(r: &Run, i: &Item) -> String {
    if crate::revision50::enabled(r) { return crate::revision50_item_knowledge::potion_description_for_run(r,i); }
    let nature = potion_nature(r, i);
    let suffix = nature
        .as_ref()
        .map(|n| format!(" ({n} magic)"))
        .unwrap_or_default();
    if i.effect.as_ref().is_some_and(|s| s == "Veridian Sap") {
        return format!(
            "{}{suffix}",
            if i.diluted.value_or(false) {
                "Diluted Veridian Sap: restores half your missing health and energy and grants half the blessings still needed for your next level (rounded up)."
            } else {
                "Veridian Sap: fully restores health and energy and grants the blessings still needed for your next level. Boundless Hunger preserves any energy above the ordinary equipped maximum."
            }
        );
    }
    let effect = i.effect.as_ref().map(String::as_str).unwrap_or("");
    if i.identified.value_or(false) || r.known_potions.iter().any(|s| s == effect) {
        return format!(
            "Potion of {effect}{}{suffix}",
            if i.diluted.value_or(false) {
                " (half efficacy)"
            } else {
                ""
            }
        );
    }
    let candidates: &[&str] = if r.simulation_revision.value_or(0.0) < 16.0 {
        &APPEARANCES
    } else {
        &POOL
    };
    let candidates = candidates
        .iter()
        .copied()
        .filter(|s| {
            !r.known_potions.iter().any(|known| known == s)
                && nature.as_ref().is_none_or(|n| {
                    *n == if matches!(
                        *s,
                        "incineration" | "caustic gas" | "paralysis" | "confusion" | "descent"
                    ) {
                        "bad"
                    } else {
                        "good"
                    }
                })
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{}{suffix}. Might be: {candidates}. Identified when used.",
        potion_appearance(r, effect)
    )
}
#[cfg(test)]
mod display_tests {
    #[test]
    fn exact_potion_display() {
        let mut r: crate::Run =
            serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let c: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/potion-display-v48.json"))
                .unwrap();
        for c in c.as_array().unwrap() {
            r.seed = c["seed"].as_str().unwrap().into();
            r.simulation_revision = c["revision"].as_f64().unwrap().into();
            r.known_potion_natures = if c["known"] == true {
                serde_json::from_value(serde_json::json!({"paralysis":"bad","healing":"good"}))
                    .unwrap()
            } else {
                crate::Field::Missing
            };
            let i: crate::Item = serde_json::from_value(c["item"].clone()).unwrap();
            assert_eq!(
                super::potion_appearance(&r, i.effect.as_ref().unwrap()),
                c["appearance"]
            );
            assert_eq!(
                serde_json::to_value(super::potion_nature(&r, &i)).unwrap(),
                c["nature"]
            );
            assert_eq!(
                serde_json::to_value(super::displayed_item_magic_nature(&r, &i)).unwrap(),
                c["magic"]
            );
            assert_eq!(super::potion_description(&r, &i), c["description"]);
        }
    }
}
