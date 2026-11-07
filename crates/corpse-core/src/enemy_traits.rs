//! Frozen intrinsic metadata and conservative legacy presentation normalization.
use crate::Actor;
use indexmap::IndexMap;
use serde::Deserialize;
use std::{collections::HashSet, sync::OnceLock};
#[derive(Debug, Deserialize)]
pub struct EnemyAbility {
    pub description: String,
    pub name: String,
    pub cost: f64,
    pub passive: bool,
}
#[derive(Debug, Deserialize)]
pub struct Definition {
    pub bleeds: Option<bool>,
    pub id: String,
    pub name: String,
    pub glyph: String,
    pub color: String,
    pub abilities: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub abilities: IndexMap<String, EnemyAbility>,
    pub definitions: Vec<Definition>,
    pub zillennia_names: Vec<String>,
    pub zillennia_color: String,
    pub pale_glyphs: IndexMap<String, String>,
    pub pale_colors: IndexMap<String, String>,
    pub legacy_colors: IndexMap<String, Vec<String>>,
    pub legacy_glyphs: IndexMap<String, Vec<String>>,
}
pub fn catalog() -> &'static Catalog {
    static C: OnceLock<Catalog> = OnceLock::new();
    C.get_or_init(|| {
        let mut data: Catalog=serde_json::from_str(include_str!("../catalog/enemy-traits-v48.json"))
            .expect("frozen enemy trait catalog");
        data.abilities.insert("enemy:beast:saidribee-queen".into(),EnemyAbility {
            name:"Summon Saidribee".into(),description:"Summons a Saidribee into an adjacent free square; successful casts wait ten world turns.".into(),cost:15.,passive:false,
        });
        data
    })
}
pub fn base_name(name: &str) -> &str {
    name.strip_prefix("Mercuria’s ")
        .or_else(|| name.strip_prefix("Mercuria's "))
        .unwrap_or(name)
}
fn definition_name(name: &str) -> Option<&'static Definition> {
    let name = crate::revision50_creature_identity::legacy_name(base_name(name));
    let name = if matches!(name, "vent-fungus" | "Vent Fungus") {
        "bastidion"
    } else {
        name
    };
    catalog()
        .definitions
        .iter()
        .find(|d| d.id == name || d.name == name)
}
pub fn definition(a: &Actor) -> Option<&'static Definition> {
    crate::new_creatures::definition(a).or_else(|| definition_name(
        a.creature_kind
            .as_ref()
            .map(String::as_str)
            .unwrap_or(&a.name),
    ))
}
fn zillennia_name(name: &str) -> bool {
    catalog()
        .zillennia_names
        .iter()
        .any(|s| s == base_name(name))
}
fn zillennia(a: &Actor) -> bool {
    a.creature_kind.as_ref().is_some_and(|s| s == "zillennia") || zillennia_name(&a.name)
}
fn pale_word(name: &str) -> bool {
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    name.match_indices("Pale").any(|(i, _)| {
        !name
            .as_bytes()
            .get(i.wrapping_sub(1))
            .is_some_and(|c| word(*c))
            && !name.as_bytes().get(i + 4).is_some_and(|c| word(*c))
    })
}
fn initialized(a: &Actor) -> bool {
    crate::engine::condition(a, "paleTraitsInitialized") != 0.0
        || crate::engine::condition(a, "bestiaryTraitsInitialized") != 0.0
}
pub fn intrinsic(a: &Actor) -> Vec<String> {
    if let Some(d) = definition(a) {
        return d.abilities.clone();
    }
    if zillennia(a) {
        return [
            "enemy:beast:ruin-pulse",
            "enemy:beast:silence-aura",
            "enemy:beast:darkness-aura",
        ]
        .map(String::from)
        .to_vec();
    }
    if !pale_word(&a.name) {
        return vec![];
    }
    let mut ids = vec!["enemy:pale:gas-immunity".into()];
    for (role, extra) in [
        (
            "Breather",
            &[
                "enemy:pale:breath",
                "enemy:pale:breath-boost",
                "enemy:pale:gas-vulnerability",
            ][..],
        ),
        (
            "Biter",
            &["enemy:pale:bite-repeat", "enemy:pale:lifesteal"][..],
        ),
        ("Spitter", &["enemy:pale:spit", "enemy:pale:gas-shot"][..]),
        (
            "Birther",
            &[
                "enemy:pale:birth",
                "enemy:pale:double-birth",
                "enemy:pale:slow",
            ][..],
        ),
        ("Screamer", &["enemy:pale:scream"][..]),
    ] {
        if crate::revision50_creature_identity::legacy_name(base_name(&a.name)).ends_with(role) {
            ids.extend(extra.iter().map(|s| s.to_string()));
        }
    }
    ids
}
pub fn enemy_traits(a: &Actor) -> Vec<String> {
    let inferred = if initialized(a) { vec![] } else { intrinsic(a) };
    let mut seen = HashSet::new();
    a.abilities
        .iter()
        .chain(&inferred)
        .filter(|id| seen.insert((*id).clone()) && catalog().abilities.contains_key(*id))
        .cloned()
        .collect()
}
pub fn has_enemy_ability(a: &Actor, id: &str) -> bool {
    a.abilities.iter().any(|s| s == id) || (!initialized(a) && intrinsic(a).iter().any(|s| s == id))
}
pub fn energy_ability_costs(a: &Actor) -> Vec<f64> {
    enemy_traits(a)
        .iter()
        .map(|id| {
            if id == "enemy:beast:detonate" {
                a.energy
            } else {
                catalog().abilities[id].cost
            }
        })
        .filter(|n| *n > 0.0)
        .collect()
}
pub fn maximum_cost(a: &Actor) -> f64 {
    energy_ability_costs(a).into_iter().fold(0.0, f64::max)
}
/// Revisioned wrapper keeps R50 defaults after frozen legacy glyph normalization.
/// O(actor name/trait/pilot/item bytes), no RNG or numerical rebalance.
pub fn initialize_enemy_traits_revision(a: &mut Actor, revision:f64) {
    initialize_enemy_traits(a);
    if revision>=50. {crate::revision50_creature_identity::apply_actor(a);}
}
pub fn initialize_enemy_traits(a: &mut Actor) {
    normalize_presentation(a);
    if initialized(a) {
        a.conditions
            .insert("maxAbilityCost".into(), maximum_cost(a));
        return;
    }
    let ids = intrinsic(a);
    if ids.is_empty() {
        return;
    }
    for id in ids {
        if !a.abilities.contains(&id) {
            a.abilities.push(id);
        }
    }
    a.conditions.insert(
        if pale_word(&a.name) {
            "paleTraitsInitialized"
        } else {
            "bestiaryTraitsInitialized"
        }
        .into(),
        1.0,
    );
    a.conditions
        .insert("maxAbilityCost".into(), maximum_cost(a));
}
pub fn pale_glyph(name: &str) -> Option<String> {
    let original = crate::revision50_creature_identity::legacy_name(base_name(name));
    let role = original
        .strip_prefix("Greater Pale ")
        .or_else(|| original.strip_prefix("Pale "))?;
    let glyph = catalog().pale_glyphs.get(role)?;
    Some(if original.starts_with("Greater ") {
        glyph.to_uppercase()
    } else {
        glyph.clone()
    })
}
pub fn enemy_glyph(name: &str) -> Option<String> {
    if let Some(p) = pale_glyph(name) {
        return Some(p);
    }
    let original = crate::revision50_creature_identity::legacy_name(base_name(name));
    if zillennia_name(original) {
        return Some("Z".into());
    }
    definition_name(original)
        .filter(|d| d.name == original)
        .map(|d| d.glyph.clone())
}
pub fn enemy_color(name: &str) -> Option<String> {
    let original = crate::revision50_creature_identity::legacy_name(base_name(name));
    if pale_glyph(original).is_some() {
        return Some(
            catalog().pale_colors[if original.starts_with("Greater ") {
                "greater"
            } else {
                "normal"
            }]
            .clone(),
        );
    }
    if zillennia_name(original) {
        return Some(catalog().zillennia_color.clone());
    }
    definition_name(original)
        .filter(|d| d.name == original)
        .map(|d| d.color.clone())
}
fn legacy_glyphs(name: &str) -> Option<Vec<String>> {
    if pale_glyph(name).is_some() {
        let role = name
            .strip_prefix("Greater Pale ")
            .or_else(|| name.strip_prefix("Pale "))
            .unwrap_or(name);
        return Some(if role == "Birther" {
            vec!["b".into(), "B".into(), "{*}".into()]
        } else {
            let c = role.chars().next()?.to_string();
            vec![c.clone(), c.to_lowercase()]
        });
    }
    catalog().legacy_glyphs.get(name).cloned()
}
pub fn normalize_presentation(a: &mut Actor) -> bool {
    if crate::half_here::is_clone(a) {
        return crate::half_here::normalize_presentation(a);
    }
    let original = base_name(&a.name).to_string();
    let mut changed = false;
    if let Some(glyph) = enemy_glyph(&a.name) {
        if a.glyph != glyph && legacy_glyphs(&original).is_some_and(|old| old.contains(&a.glyph)) {
            a.glyph = glyph;
            changed = true;
        }
    }
    let legacy = if pale_glyph(&original).is_some() {
        Some(vec!["#d2c9c4".into()])
    } else {
        catalog()
            .legacy_colors
            .get(if zillennia_name(&original) {
                "Zillennia"
            } else {
                &original
            })
            .cloned()
    };
    if let Some(color) = enemy_color(&a.name) {
        if original == a.name && legacy.is_some_and(|old| old.contains(&a.color.to_lowercase())) {
            a.color = color;
            changed = true;
        }
    }
    changed
}
