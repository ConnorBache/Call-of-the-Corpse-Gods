//! Deterministic synthetic prior-floor collection, not a survival simulation.
//! Rewards and offers use their real gameplay paths; no stats are invented.
use crate::{Actor, Command, Field, Item, Run};
use serde::{Deserialize, Serialize};

pub const PROFILE: &str = "all-prior-ordinary-hearts-v1";
pub const DESCRIPTION: &str = "Synthetic full collection of prior ordinary hearts, including budgeted machine enemies; excludes Godbeasts and rare rewards. Seeded prior-floor equipment and up to four potions. Fully restored; unranked. Not simulated survival.";
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EquipmentSource {
    pub depth: u32,
    pub item_id: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Profile {
    pub version: u32,
    pub start_floor: u32,
    pub profile: String,
    pub prior_hearts: u32,
    pub prior_blessings: f64,
    pub sources: Vec<EquipmentSource>,
    /// R50 preparation-only loot, not owned/carried until the final sample is chosen.
    /// Emptied before publication and excluded from the persistent profile schema.
    #[serde(skip)]
    pub candidates: Vec<Item>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debug: Option<crate::debug_floor::State>,
}
pub fn parse_floor(input: &str) -> Result<u32, String> {
    let floor = input.trim().parse::<u32>().map_err(|_| "Enter a whole floor number from 1 to 40.".to_string())?;
    if !(1..=40).contains(&floor) { return Err("Enter a whole floor number from 1 to 40.".into()); }
    Ok(floor)
}
pub fn validate_profile(r: &Run) -> Result<(), String> {
    if let Some(p) = r.playtest.as_ref() {
        if r.simulation_revision.value_or(0.) < 31. || !(p.version == 1 && p.profile == PROFILE || p.version == 2 && p.profile == crate::debug_floor::PROFILE)
            || !(1..=40).contains(&p.start_floor) || !p.prior_blessings.is_finite()
            || p.prior_blessings < 0. || p.prior_blessings.fract() != 0. || !p.candidates.is_empty()
            || p.sources.iter().any(|s| s.depth == 0 || s.depth >= p.start_floor || !s.item_id.is_finite() || s.item_id < 0. || s.item_id.fract() != 0.) {
            return Err("Invalid or unsupported playtest starting profile.".into());
        }
        if p.version == 2 { crate::debug_floor::validate(r, p)?; }
        else if p.debug.is_some() { return Err("Unexpected debug state in legacy playtest".into()); }
    } else if !r.playtest.is_missing() { return Err("Invalid null playtest profile.".into()); }
    Ok(())
}
pub(crate) fn is_prior_floor(r: &Run, depth: u32) -> bool {
    r.playtest.as_ref().is_some_and(|p| p.version == 1 && depth < p.start_floor)
}
pub(crate) fn ordinary_heart_actor(a: &Actor) -> bool {
    !a.dead.value_or(false) && !a.heartless.value_or(false)
        && a.creature_kind.as_ref().is_none_or(|k| k != "zillennia")
        && !a.conditions.contains_key("godbeastDepth")
}
/// O(H*(A²+I) + L), H actual ordinary actors, A authored ability count, L ordinary loot.
/// Called once per prior generated floor, before rare rewards. No turns/AI are advanced.
pub(crate) fn collect_reference_floor(r: &mut Run, depth: u32, actors: &[Actor], loot: &[Item]) -> Result<(), String> {
    for a in actors {
        let heart = crate::inventory::heart_for_revision(a, r.next_id, r.simulation_revision.value_or(29.)).map_err(|e| e.0)?;
        r.next_id += 1.;
        let id = heart.id;
        let exp = heart.exp.value_or(0.);
        r.inventory.push(heart);
        if crate::inventory::command(r, &Command::Use { id }).map_err(|e| e.0)? != Some(true) {
            return Err("Generated prior heart could not be consumed.".into());
        }
        let p = r.playtest.as_mut().unwrap();
        p.prior_hearts += 1;
        p.prior_blessings += exp;
        // The real offer constructor consumes serialized gameplay RNG. A separate,
        // stable seed/floor/heart choice selects only legal offered entries.
        while !r.offers.is_empty() {
            let legal: Vec<_> = r.offers.iter().filter(|id| id.starts_with("eclipsed") || crate::abilities::supported(id) || matches!(id.as_str(), "numo-revert" | "morae-see-the-future")).cloned().collect();
            if legal.is_empty() { return Err("No implemented legal blessing in the generated playtest offer.".into()); }
            let key = format!("{}:playtest:v1:{depth}:heart:{}:level:{}", r.seed, r.playtest.as_ref().unwrap().prior_hearts, r.level);
            let mut rng = crate::population::population_random(&key);
            let id = &legal[(rng.random() * legal.len() as f64).floor() as usize];
            crate::progression::choose_offer(r, id).map_err(|e| e.0)?;
        }
    }
    for item in loot {
        if crate::revision50::enabled(r) {
            r.playtest.as_mut().unwrap().candidates.push(item.clone());
        } else { r.inventory.push(item.clone()); }
        r.playtest.as_mut().unwrap().sources.push(EquipmentSource { depth, item_id: item.id });
    }
    Ok(())
}
fn shuffle<T>(values: &mut [T], rng: &mut crate::rng::GenerationRng) {
    for n in (1..values.len()).rev() {
        let at = (rng.random() * (n + 1) as f64).floor() as usize;
        values.swap(n, at);
    }
}
/// Legacy: at most six legal worn slots, starter bow/arrows and four potions.
/// R50: same two-ring/one-amulet sample and potion count, unlimited trinkets;
/// carried jewelry is active without legacy slot flags. Acquisition never refills.
/// O(I + S*I + acquisition + settlement), bounded I<=20*(start_floor-1)+3
/// candidates and S source entries. R50 performs one final ownership settlement;
/// legacy keeps six slot settlements. No arbitrary target-depth loot draws.
fn select_loadout(r: &mut Run) -> Result<(), String> {
    let start = r.playtest.as_ref().unwrap().start_floor;
    if start == 1 { return Ok(()); }
    let revision50 = crate::revision50::enabled(r);
    let mut candidates: Vec<_> = if revision50 {
        std::mem::take(&mut r.playtest.as_mut().unwrap().candidates)
    } else { r.inventory.drain(3..).collect() };
    let mut rng = crate::population::population_random(&format!("{}:playtest:v1:{start}:loadout", r.seed));
    shuffle(&mut candidates, &mut rng);
    let mut selected = Vec::new();
    for (kind, slots) in [("weapon", vec!["hand1", "hand2"]), ("armor", vec!["armor"]), ("ring", vec!["ring1", "ring2"]), ("amulet", vec!["amulet"])] {
        if revision50 && matches!(kind, "weapon" | "armor") { continue; }
        let mut two_handed = false;
        for slot in slots {
            if two_handed { break; }
            let Some(at) = candidates.iter().position(|i| i.kind == kind && !(slot == "hand2" && i.hands.value_or(1.) == 2.) && crate::equipment::validate(i).is_ok()) else { break; };
            let item = candidates.remove(at);
            two_handed = item.hands.value_or(1.) == 2.;
            selected.push((item, Some(slot)));
        }
    }
    if revision50 {
        // The synthetic profile keeps its existing two-ring/one-amulet sample,
        // not a new random loadout rule. All useful special items fit as trinkets.
        let mut potions = 0;
        for item in candidates {
            if crate::revision50_inventory::category(&item) == Some(crate::revision50_inventory::Category::Trinket) {
                selected.push((item, None));
            } else if item.kind == "potion" && potions < 4 {
                selected.push((item, None)); potions += 1;
            }
        }
    } else {
        for item in candidates.into_iter().filter(|i| i.kind == "potion").take(4) { selected.push((item, None)); }
    }
    // Add only final choices before ordinary carried-ownership reconciliation.
    let before = revision50.then(|| crate::equipment::capture_before(r));
    for (item, _) in &selected {
        let mut acquired = item.clone();
        if revision50 { crate::revision50_ability_energy::acquire_item(&mut r.player, &mut acquired)?; }
        r.inventory.push(acquired);
    }
    if let Some(before) = before {
        crate::equipment::settle_removed(r, before).map_err(|e|e.0)?;
    }
    for (item, slot) in &selected {
        if revision50 { continue; }
        if let Some(slot) = slot {
            if !crate::equipment::equip(r, item.id, slot).map_err(|e| e.0)? { return Err("Generated playtest equipment could not be equipped.".into()); }
        }
    }
    // Keep the starter shortsword only when no generated hand item replaced it.
    r.inventory.retain(|i| i.id != 1. || i.equipped.as_ref().is_some());
    r.playtest.as_mut().unwrap().sources.retain(|s| selected.iter().any(|(i, _)| i.id == s.item_id));
    Ok(())
}
pub fn create_run(seed: &str, mode: &str, start_floor: u32) -> Result<Run, String> {
    create_run_revision(seed, mode, false, start_floor, PROFILE, 33.)
}
pub fn create_run_revision(seed: &str, mode: &str, dev: bool, start_floor: u32, profile: &str, revision: f64) -> Result<Run, String> {
    if !(1..=40).contains(&start_floor) || profile != PROFILE || ![31., 32., 33., 34., 35., 36., 37., 38., 39., 40., 41., 42., 43., 44., 45., 46., 50., 51.].contains(&revision) {
        return Err("Unsupported playtest floor, profile or simulation revision.".into());
    }
    if seed.trim().is_empty() || seed.split_once(':').map_or(seed, |(_, suffix)| suffix).len() > 160 || seed.chars().any(char::is_control) {
        return Err("Enter a nonempty seed of at most 160 bytes without control characters.".into());
    }
    let mut r = crate::startup::new_run_shell_revision(seed, mode, dev, revision)?;
    r.playtest = Profile { version: 1, start_floor, profile: PROFILE.into(), prior_hearts: 0, prior_blessings: 0., sources: vec![], candidates: vec![], debug: None }.into();
    for id in r.player.abilities.clone() { crate::progression::initialize_ability_state(&mut r.player, &id, r.tick)?; }
    crate::startup::ensure_floors(&mut r, start_floor.max(10))?;
    r.depth = start_floor as f64;
    let f = r.floor().ok_or("Missing playtest starting floor")?;
    let at = f.tiles.iter().position(|t| t == "up").ok_or("Playtest floor has no arrival stair")?;
    let arrival = crate::entrance::arrival(f);
    (r.player.x, r.player.y) = ((at % f.width()) as f64, (at / f.width()) as f64);
    if let Some(arrival) = arrival { (r.player.x, r.player.y) = (arrival.x as f64, arrival.y as f64); }
    select_loadout(&mut r)?;
    crate::equipment::sync_health(&mut r, true);
    r.player.hp = r.player.max_hp;
    r.player.energy = crate::combat::effective_stats(&r, &r.player).max_energy;
    r.log.clear(); r.log_kinds = vec![].into();
    r.log_archive = crate::LogArchive { version: 1., ..Default::default() }.into();
    r.resource_history = Field::Missing;
    crate::engine::log(&mut r, "You begin to adventure down to end the corruption of the Flesh.", "environment");
    let identity = format!("PLAYTEST • Floor {start_floor} • Seed {} • {PROFILE}", r.seed);
    crate::engine::log(&mut r, identity, "environment");
    crate::engine::log(&mut r, DESCRIPTION, "environment");
    let p = r.playtest.as_ref().unwrap();
    let equipment = if crate::revision50::enabled(&r) {
        format!("{} automatically active rings and amulets", r.inventory.iter().filter(|i| matches!(i.kind.as_str(), "ring" | "amulet")).count())
    } else { format!("{} equipped items", r.inventory.iter().filter(|i| i.equipped.as_ref().is_some()).count()) };
    let note = format!("Prior floors: {} ordinary hearts, {} blessings; level {}. Seeded loadout ready: {equipment} and {} potions. Press I to inspect the Pack.", p.prior_hearts, p.prior_blessings, r.level, r.inventory.iter().filter(|i| i.kind == "potion").count());
    crate::engine::log(&mut r, note, "environment");
    crate::engine::monster_floor_entered(&mut r);
    crate::engine::monster_prepare_player_turn(&mut r).map_err(|e| e.0)?;
    r.initial_player = r.player.clone().into();
    crate::engine::remember(&mut r);
    Ok(r)
}

#[cfg(test)]
#[path = "revision50_playtest_loadout_tests.rs"]
mod revision50_loadout_tests;
