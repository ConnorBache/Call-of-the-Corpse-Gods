//! R50 presentation-only identities. Frozen catalogs and internal species keys
//! remain stable for replay, heart accounting and authored mechanics.
use crate::{Actor, Item, Run};
use std::borrow::Cow;

pub const BLORB_COLOR: &str = "#28703c";
pub const BEAR_COLOR: &str = "#a47c55";
pub const DIRE_BEAR_COLOR: &str = "#000000";
pub const INVENTOR_COLOR: &str = "#d600ff";
pub const SAMARA_COLOR: &str = "#8a5a32";
pub const SAIDRIBEE_YELLOW: &str = "#f8d100";
pub const CHIMERA_GOLD: &str = "#ddb435";
pub const SPHINX_GOLD: &str = "#ffd34a";

/// O(L), fixed exact-name aliases; custom names and internal IDs are untouched.
pub fn legacy_name(name: &str) -> &str {
    match name {
        "Corpse Fungus" => "Bastidion",
        "Devil's Samara Tree" => "Devil's Samara",
        "Pale Chomper" => "Pale Biter",
        "Greater Pale Chomper" => "Greater Pale Biter",
        "Pale Gasser" => "Pale Breather",
        "Greater Pale Gasser" => "Greater Pale Breather",
        _ => name,
    }
}
/// O(L), fixed exact-name aliases, borrowing unless a prefix/suffix needs copying.
pub fn display_name(name: &str) -> Cow<'_, str> {
    let (prefix, rest) = if let Some(rest) = name.strip_prefix("Mercuria’s ") { ("Mercuria’s ", rest) }
        else if let Some(rest) = name.strip_prefix("Mercuria's ") { ("Mercuria's ", rest) }
        else { ("", name) };
    let (base, suffix) = if let Some(base) = rest.strip_suffix(" <heart>") { (base, " <heart>") }
        else if let Some(base) = rest.strip_suffix(" heart") { (base, " heart") }
        else { (rest, "") };
    let renamed = match base {
        "Bastidion" | "Vent Fungus" => "Corpse Fungus",
        "Devil's Samara" => "Devil's Samara Tree",
        "Pale Biter" => "Pale Chomper",
        "Greater Pale Biter" => "Greater Pale Chomper",
        "Pale Breather" => "Pale Gasser",
        "Greater Pale Breather" => "Greater Pale Gasser",
        _ => return Cow::Borrowed(name),
    };
    if prefix.is_empty() && suffix.is_empty() { Cow::Borrowed(renamed) }
    else { Cow::Owned(format!("{prefix}{renamed}{suffix}")) }
}
pub fn name_for_revision(name: &str, revision: f64) -> Cow<'_, str> {
    if revision >= 50. { display_name(name) } else { Cow::Borrowed(name) }
}
/// O(L) on construction/load and existing trait refresh. Explicit non-default actor glyph/color survive.
/// Recursive pilot/item payload P contributes O(P) work, with no RNG/stat edits.
pub fn apply_actor(a: &mut Actor) {
    let renamed=display_name(&a.name);if renamed.as_ref()!=a.name {a.name=renamed.into_owned();}
    let name=crate::enemy_traits::base_name(&a.name);
    let kind=a.creature_kind.as_ref().map(String::as_str).unwrap_or(name);
    let defaults = match kind {
        "blorb" | "Blorb" => Some((&["o","B","¤","a"][..], "b", &["#b1a286"][..], BLORB_COLOR)),
        "bear" | "Bear" => Some((&["u"][..], "B", &["#a47c55"][..], BEAR_COLOR)),
        "dire-bear" | "Dire Bear" => Some((&["U"][..], "B", &["#e0a368","#bc9164"][..], DIRE_BEAR_COLOR)),
        "bastidion" | "Bastidion" | "Corpse Fungus" | "vent-fungus" | "Vent Fungus" => Some((&["&","B","F"][..], "F", &[][..], "#9fc875")),
        "hemogoblin-inventor" | "Hemogoblin Inventor" => Some((&["i"][..], "h", &["#eab567"][..], INVENTOR_COLOR)),
        "yeti" | "Yeti" => Some((&["A"][..], "Y", &[][..], "#D7E8F1")),
        "samara" | "Devil's Samara" | "Devil's Samara Tree" => Some((&["Y"][..], "T", &["#85b382","#70946a"][..], SAMARA_COLOR)),
        "vampire" | "Vampire" => Some((&["v"][..], "V", &[][..], "#D54B5C")),
        "banshee" | "Banshee" => Some((&["b"][..], "B", &["#a2e0da"][..], INVENTOR_COLOR)),
        "short-saidri" | "Short-legged Saidribee" => Some((&["v","b"][..], "b", &["#e3c55b","#d3b88a"][..], SAIDRIBEE_YELLOW)),
        "long-saidri" | "Long-legged Saidribee" => Some((&["w","b"][..], "b", &["#75cb9d","#d6a374","#7fca9d"][..], SAMARA_COLOR)),
        "saidri-guardian" | "Saidribee Guardian" => Some((&["V","B"][..], "B", &["#ed9de0","#efc88e","#dd9de0"][..], SAIDRIBEE_YELLOW)),
        "pale:Birther" | "Pale Birther" => Some((&["(≬)"][..], "b", &[][..], "#b9b8b1")),
        "pale:Greater Birther" | "Greater Pale Birther" => Some((&["(≬)"][..], "B", &[][..], "#f0e8dc")),
        "chimera" | "Chimera" => Some((&["C"][..], "C", &["#d99270"][..], CHIMERA_GOLD)),
        "sphinx" | "Sphinx" => Some((&["S"][..], "S", &["#e9cf78"][..], SPHINX_GOLD)),
        _ => None,
    };
    if let Some((old_glyphs,glyph,old_colors,color))=defaults {
        if old_glyphs.contains(&a.glyph.as_str()) {a.glyph=glyph.into();}
        if old_colors.iter().any(|old|a.color.eq_ignore_ascii_case(old)) {a.color=color.into();}
    }
    if let Some(state)=a.monster_state.as_mut() {
        if let Some(pilot)=state.pilot.as_mut() {apply_actor(pilot);}
        if let Some(items)=state.carried_items.as_mut() {for item in items {apply_item(item);}}
    }
}
pub fn apply_item(item: &mut Item) {
    if item.kind=="heart" {let name=display_name(&item.name);if name.as_ref()!=item.name {item.name=name.into_owned();}}
}
/// O(A+I+L) over all floors, pilots, held/remembered/detected hearts and name bytes.
/// Called by the existing timeline/anchor migration orchestrator independently
/// for each state. Existing logs stay historical; subsequent logs use new names.
pub fn normalize_world(r: &mut Run) {
    if r.simulation_revision.value_or(29.)<50. {return;}
    for block in r.actors.values_mut() {for actor in crate::mutation_journal::make_mut(block) {apply_actor(actor);}}
    for item in &mut r.inventory {apply_item(item);}
    for block in r.ground.values_mut() {for ground in crate::mutation_journal::make_mut(block) {apply_item(&mut ground.item);}}
    if let Some(map)=r.remembered_items.as_mut() {for entries in map.values_mut() {for ground in entries {apply_item(&mut ground.item);}}}
    if let Some(map)=r.detected_items.as_mut() {for entries in map.values_mut() {for ground in entries {apply_item(&mut ground.item);}}}
}
