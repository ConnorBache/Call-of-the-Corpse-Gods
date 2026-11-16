//! Explicit floor-testing reconstruction. Generation is normal; collection is not combat.
use crate::{Actor, Command, Field, Item, Run};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, sync::Arc};

pub const PROFILE: &str = "debug-all-natural-prior-collection-v2";
pub const ASSUMPTIONS: &str = "DEBUG: all naturally generated prior-floor hearts and ordinary ground/carried loot collected. No combat consumption or survival simulated. No optional Sphinx rewards, event activations or summoned extras. Full health/energy at entry. Autosave and high scores disabled.";
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reward { pub depth: u32, pub item: Item }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct State {
    pub pending: VecDeque<Reward>,
    pub consumed: u32,
    pub stash: Vec<Reward>,
    pub entered: bool,
    /// Zero-turn external loadout edits must be part of the temporal closing frame.
    #[serde(default)]
    pub loadout_revision: u64,
}
// The same queue handles generated actor, carried and ground hearts. Only the
// metadata total is derived; the stored item remains untouched for real consumption.
fn queue_heart(r: &Run, state: &mut State, blessings: &mut f64, depth: u32, item: Item) {
    *blessings += crate::inventory::effective_heart_rewards(r, &item).blessings;
    state.pending.push_back(Reward { depth, item });
}
pub fn state(r: &Run) -> Option<&State> { r.playtest.as_ref().and_then(|p| p.debug.as_ref()) }
fn state_mut(r: &mut Run) -> Option<&mut State> { r.playtest.as_mut().and_then(|p| p.debug.as_mut()) }
fn loadout_changed(r: &mut Run) { let s = state_mut(r).unwrap(); s.loadout_revision = s.loadout_revision.wrapping_add(1); }
pub fn preparing(r: &Run) -> bool { state(r).is_some_and(|s| !s.entered) }
pub fn choices_complete(r: &Run) -> bool { state(r).is_some_and(|s| s.pending.is_empty()) && r.offers.is_empty() }
pub fn natural_heart(a: &Actor) -> bool { !a.dead.value_or(false) && !a.heartless.value_or(false)
    && !crate::dreamwalking::is_nightmare(a) && !crate::dreamwalking::is_body(a) }
fn phantasmii_quest(i: &Item) -> bool { crate::phantasmii_generation::is_dagger(i) || crate::phantasmii_generation::is_shard(i) }
/// Normal ground loot excludes gated Sphinx prizes and unearned Phantasmii quest items.
pub fn ordinary_ground(r: &Run, depth: u32, g: &crate::GroundItem) -> bool {
    if phantasmii_quest(&g.item) { return false; }
    if g.item.kind == "tool" && g.item.effect.as_ref().is_some_and(|e| e == "tree-tap") { return false; }
    let Some(f) = r.floors.iter().find(|f| f.depth == depth as f64) else { return false; };
    !f.sphinx_room.as_ref().is_some_and(|room| room.reward_cells.contains(&(g.y * f.width() as f64 + g.x)))
}
pub fn validate(r: &Run, p: &crate::playtest::Profile) -> Result<(), String> {
    let s = p.debug.as_ref().ok_or("Missing debug reconstruction state")?;
    if p.version != 2 || p.profile != PROFILE || ![33.,34.,35.,36.,37.,38.,39.,40.,41.,42.,43.,44.,45.,46.,50.,51.,52.].contains(&r.simulation_revision.value_or(0.))
        || s.consumed > p.prior_hearts || s.pending.len() + s.consumed as usize != p.prior_hearts as usize
        || s.entered && (!s.pending.is_empty())
        || !crate::fast_mode::playable(r, p.start_floor as f64)
        || s.pending.iter().any(|v| v.depth == 0 || v.depth >= p.start_floor || !crate::fast_mode::playable(r, v.depth as f64))
        || s.stash.iter().any(|v| v.depth >= p.start_floor || v.depth != 0 && !crate::fast_mode::playable(r, v.depth as f64))
        || s.pending.iter().any(|v| v.item.kind != "heart") {
        return Err("Invalid debug reconstruction profile".into());
    }
    for v in s.pending.iter().chain(&s.stash) { crate::equipment::validate(&v.item).map_err(|e| e.0)?; }
    Ok(())
}
/// Current DEBUG input is a displayed floor 1–20; reconstruct only visited floors.
pub fn create_avatar_run(seed: &str, floor: u32) -> Result<Run, String> {
    if !(1..=20).contains(&floor) { return Err("Debug floor must be 1–20.".into()); }
    create_run_revision_route(seed, "Avatar", crate::fast_mode::content_depth(floor), 52.0, true, true)
}
/// Historical Fast DEBUG reconstruction retained for compatibility.
pub fn create_avatar_run_fast(seed: &str, floor: u32) -> Result<Run, String> {
    create_run_revision_mode(seed, "Avatar", floor, 52.0, true)
}

/// Historical reconstruction helper; new DEBUG games use `create_avatar_run`.
pub fn create_run(seed: &str, mode: &str, floor: u32) -> Result<Run, String> {
    create_run_revision(seed, mode, floor, 39.)
}
/// Explicit revision keeps existing revision33 debug reconstructions reproducible.
pub fn create_run_revision(seed: &str, mode: &str, floor: u32, revision: f64) -> Result<Run, String> {
    create_run_revision_mode(seed, mode, floor, revision, false)
}
fn create_run_revision_mode(seed: &str, mode: &str, floor: u32, revision: f64, fast: bool) -> Result<Run, String> {
    create_run_revision_route(seed, mode, floor, revision, fast, false)
}
fn create_run_revision_route(seed: &str, mode: &str, floor: u32, revision: f64, fast: bool, compressed: bool) -> Result<Run, String> {
    if ![33.,34.,35.,36.,37.,38.,39.,40.,41.,42.,43.,44.,45.,46.,50.,51.,52.].contains(&revision) { return Err("Unsupported debug simulation revision".into()); }
    if !(1..=40).contains(&floor) { return Err("Debug floor must be 1–40.".into()); }
    if fast && !compressed && floor != 40 && floor % 2 == 0 { return Err("Fast DEBUG requires an odd floor from 1 to 39, or floor 40.".into()); }
    if seed.trim().is_empty() || seed.split_once(':').map_or(seed, |(_, suffix)| suffix).len() > 160 || seed.chars().any(char::is_control) {
        return Err("Enter a seed of 1–160 bytes without control characters.".into());
    }
    let mut r = crate::startup::new_run_shell_revision(seed, mode, false, revision)?;
    if fast { r.fast_mode = true.into(); }
    if compressed { r.compressed_depths = true.into(); }
    // No playtest marker during generation: IDs, ground, carried loot, rare placement,
    // floor populations and target-floor rewards are exactly the normal seed's world.
    let to = if fast { crate::fast_mode::arrival_target(&r, floor) } else { crate::startup::initial_generation_target(revision, floor) };
    crate::startup::ensure_floors(&mut r, to)?;
    for id in r.player.abilities.clone() { crate::progression::initialize_ability_state_for_revision(r.simulation_revision.value_or(29.), &mut r.player, &id, r.tick)?; }
    let mut state = State { pending: VecDeque::new(), consumed: 0, stash: vec![], entered: false, loadout_revision: 0 };
    let mut sources = vec![];
    let mut blessings = 0.;
    // O(H + G + C) enumeration after the real generation pipeline; real heart
    // consumption/offer work is deferred at every earned level until user choice.
    let prior_depths: Vec<u32> = r.floors.iter().map(|f| f.depth as u32).filter(|&d| d < floor).collect();
    for depth in prior_depths {
        let actors = r.actors_at(depth as f64).to_vec();
        for a in &actors {
            if natural_heart(a) {
                let item = crate::inventory::heart_for_revision(a, r.next_id, r.simulation_revision.value_or(29.)).map_err(|e| e.0)?;
                r.next_id += 1.; queue_heart(&r, &mut state, &mut blessings, depth, item);
            }
            if let Some(items) = a.monster_state.as_ref().and_then(|s| s.carried_items.as_ref()) {
                for item in items.iter().filter(|item| !phantasmii_quest(item)) {
                    if item.kind == "heart" { queue_heart(&r, &mut state, &mut blessings, depth, item.clone()); }
                    else { state.stash.push(Reward { depth, item: item.clone() }); }
                }
            }
        }
        let ground = r.ground.get(&depth.to_string()).map(|v|v.as_ref().clone()).unwrap_or_default();
        for g in ground.iter().filter(|g| ordinary_ground(&r, depth, g)) {
            if g.item.kind == "heart" {
                queue_heart(&r, &mut state, &mut blessings, depth, g.item.clone());
            } else { state.stash.push(Reward { depth, item: g.item.clone() }); }
        }
        let remaining = ground.into_iter().filter(|g| !ordinary_ground(&r, depth, g)).collect();
        r.ground.insert(depth.to_string(), Arc::new(remaining));
        // A collected heart represents a defeated initial actor, not a renewable
        // alive/heartless source. Other actors lose recovered carried items once.
        let prior = r.actors_at_mut(depth as f64);
        prior.retain(|a| !natural_heart(a));
        for a in prior { if let Some(s) = a.monster_state.as_mut() {
            if let Some(items) = s.carried_items.as_mut() { items.retain(phantasmii_quest); }
            if s.carried_items.as_ref().is_some_and(Vec::is_empty) { s.carried_items = Field::Missing; }
        } }
    }
    // Reconstruction removes defeated fungi directly instead of resolving combat
    // deaths. Apply the same R51 root cleanup now as save/load, including dirty
    // flags, so resuming cannot activate extra hidden floors and change turn RNG.
    // O(all root floors' actors + root/path material), with existing COW costs.
    crate::fungi::normalize_world(&mut r);
    for reward in &state.stash { sources.push(crate::playtest::EquipmentSource { depth: reward.depth, item_id: reward.item.id }); }
    r.playtest = crate::playtest::Profile { version: 2, start_floor: floor, profile: PROFILE.into(), prior_hearts: state.pending.len() as u32, prior_blessings: blessings, sources, candidates: vec![], debug: Some(state) }.into();
    r.depth = floor as f64;
    let f = r.floor().ok_or("Missing debug target floor")?;
    let at = f.tiles.iter().position(|t| t == "up").ok_or("Debug target has no up stair")?;
    let arrival = crate::entrance::arrival(f);
    (r.player.x, r.player.y) = ((at % f.width()) as f64, (at / f.width()) as f64);
    if let Some(at) = arrival { (r.player.x, r.player.y) = (at.x as f64, at.y as f64); }
    crate::engine::log(&mut r, ASSUMPTIONS, "environment");
    let identity = format!("DEBUG • floor {} • exact seed {}. Choose each earned level before entering.", crate::fast_mode::display_depth(&r, floor as f64), r.seed);
    crate::engine::log(&mut r, identity, "environment");
    // Mini-Me uses the real starter snapshot, not the reconstructed high-level body.
    r.initial_player = r.player.clone().into();
    advance(&mut r)?;
    crate::engine::remember(&mut r);
    Ok(r)
}
/// Consume actual hearts in stable floor/actor order, stopping immediately at
/// each level offer. In particular Adaptive Armor cannot gain retroactive hearts.
pub fn advance(r: &mut Run) -> Result<(), String> {
    if !preparing(r) { return Ok(()); }
    while r.offers.is_empty() {
        let Some(reward) = state_mut(r).unwrap().pending.pop_front() else { break; };
        let target = r.depth; r.depth = reward.depth as f64;
        let id = reward.item.id; r.inventory.push(reward.item);
        if crate::inventory::command(r, &Command::Use { id }).map_err(|e| e.0)? != Some(true) {
            return Err("Debug heart consumption failed".into());
        }
        state_mut(r).unwrap().consumed += 1;
        r.depth = target;
    }
    crate::equipment::sync_health(r, true);
    r.player.hp = r.player.max_hp;
    r.player.energy = crate::combat::effective_stats(r, &r.player).max_energy;
    Ok(())
}
/// Debug-only stash does not expand normal Pack slots or fabricate/replace items.
pub fn take(r: &mut Run, id: f64) -> Result<bool, String> {
    if !choices_complete(r) { return Err("Finish every earned level choice first.".into()); }
    let Some(at) = state(r).and_then(|s|s.stash.iter().position(|v|v.item.id == id)) else { return Ok(false); };
    if crate::revision50::enabled(r) {
        let mut draft = r.clone();
        let result = take_revision50(&mut draft, at)?;
        if result { *r = draft; }
        return Ok(result);
    }
    let item = &state(r).unwrap().stash[at].item;
    let arrow = crate::inventory::is_arrow(item);
    let merge = arrow.then(||r.inventory.iter().position(crate::inventory::is_arrow)).flatten();
    let needed = if arrow { if merge.is_some() { 0. } else { 1. } } else { crate::inventory::quantity(item) };
    if crate::inventory::occupancy(r) + needed > 20. { return Err("Pack is full (20 slots). Return an item to the debug stash first.".into()); }
    let reward = state_mut(r).unwrap().stash.remove(at);
    if let Some(at) = merge { r.inventory[at].quantity = (crate::inventory::quantity(&r.inventory[at]) + crate::inventory::quantity(&reward.item)).into(); }
    else { r.inventory.push(reward.item); }
    loadout_changed(r);
    Ok(true)
}
/// R50 uses ordinary category, acquisition and carried-jewelry settlement rules.
/// The caller's detached Run keeps stash, ownership and reserves atomic on error.
/// O(I*N+B+G*A+S+R+settlement) time and O(R) detached payload: item count I,
/// identity bytes N, blessings B, grants G, catalog A=72, stash S, owned Run R.
/// Settlement includes grant/stat, environment and linked-actor reconciliation.
fn take_revision50(r: &mut Run, at: usize) -> Result<bool, String> {
    let mut item = state(r).unwrap().stash[at].item.clone();
    crate::equipment::validate_for_revision(&item, r.simulation_revision.value_or(29.)).map_err(|e|e.0)?;
    if !crate::revision50_inventory::classified(&item) {
        return Err("This item has no Revision 50 backpack category.".into());
    }
    if crate::revision50_inventory::space_for(r, &item) < crate::inventory::quantity(&item) {
        return Err("This backpack category is full. Return an item from that category to the debug stash first.".into());
    }
    let key = crate::inventory::stack_key(&item);
    let merge = key.as_ref().and_then(|key| r.inventory.iter().position(|i|crate::inventory::stack_key(i).as_ref()==Some(key)));
    let before = crate::equipment::capture_before(r);
    let mut player = r.player.clone();
    crate::revision50_ability_energy::acquire_item_for_revision(&mut player, &mut item, r.simulation_revision.value_or(29.))?;
    state_mut(r).unwrap().stash.remove(at);
    r.player = player;
    if let Some(index) = merge {
        r.inventory[index].quantity = (crate::inventory::quantity(&r.inventory[index]) + crate::inventory::quantity(&item)).into();
    } else { r.inventory.push(item); }
    crate::equipment::settle_removed(r, before).map_err(|e|e.0)?;
    loadout_changed(r);
    Ok(true)
}
pub fn put(r: &mut Run, id: f64) -> Result<bool, String> {
    let mut draft = r.clone();
    let result = put_inner(&mut draft, id)?;
    if result { *r = draft; }
    Ok(result)
}
fn put_inner(r: &mut Run, id: f64) -> Result<bool, String> {
    if !choices_complete(r) { return Err("Finish every earned level choice first.".into()); }
    let Some(at) = r.inventory.iter().position(|v|v.id == id) else { return Ok(false); };
    if id > 3. && !r.playtest.as_ref().unwrap().sources.iter().any(|v|v.item_id == id) { return Err("Only starter and reconstructed loot can return to this debug stash.".into()); }
    let revision50 = crate::revision50::enabled(r);
    if revision50 {
        if let Some(reason) = crate::equipment::removal_blocker(r, id, true) { return Err(reason); }
    } else if r.inventory[at].equipped.as_ref().is_some() && !crate::equipment::unequip(r, id).map_err(|e|e.0)? {
        return Err("Normal equipment rules prevent removing this equipped item.".into());
    }
    let before = revision50.then(|| crate::equipment::capture_before(r));
    let item = r.inventory.remove(at);
    let depth = r.playtest.as_ref().unwrap().sources.iter().find(|v|v.item_id == id).map_or(0, |v|v.depth);
    // Starter items may be stashed even on floor one (depth zero denotes starter).
    state_mut(r).unwrap().stash.push(Reward { depth, item });
    if let Some(before) = before { crate::equipment::settle_removed(r, before).map_err(|e|e.0)?; }
    else { crate::equipment::sync_health(r, false); }
    loadout_changed(r);
    Ok(true)
}
pub fn equip(r: &mut Run, id: f64) -> Result<bool, String> {
    let mut draft = r.clone();
    let result = equip_inner(&mut draft, id)?;
    if result { *r = draft; }
    Ok(result)
}
fn equip_inner(r: &mut Run, id: f64) -> Result<bool, String> {
    if state(r).is_none() { return Ok(false); }
    if !r.inventory.iter().any(|i|i.id == id) && !take(r, id)? { return Ok(false); }
    let item = r.inventory.iter().find(|i|i.id == id).ok_or("Item not in pack")?;
    if crate::revision50::enabled(r) {
        if !matches!(item.kind.as_str(), "ring" | "amulet") {
            return Err("Carry this item with Enter. Rings and amulets are automatically active while carried.".into());
        }
        // Compatibility shortcut: take already enforced the category limit and
        // settled ownership. Carried jewelry never needs a separate equip step.
        return Ok(true);
    }
    let slot = match item.kind.as_str() { "weapon" => "hand1", "armor" => "armor", "amulet" => "amulet", "ring" => if r.inventory.iter().any(|i|i.equipped.as_ref().is_some_and(|s|s == "ring1")) { "ring2" } else { "ring1" }, _ => return Err("This item cannot be equipped. Carry it instead.".into()) };
    let equipped = crate::equipment::equip(r, id, slot).map_err(|e|e.0)?;
    if equipped { loadout_changed(r); }
    Ok(equipped)
}
pub fn enter(r: &mut Run) -> Result<(), String> {
    if !choices_complete(r) { return Err("Finish every earned level choice first.".into()); }
    if state(r).unwrap().entered { return Ok(()); }
    crate::equipment::sync_health(r, true);
    r.player.hp = r.player.max_hp;
    r.player.energy = crate::combat::effective_stats(r, &r.player).max_energy;
    state_mut(r).unwrap().entered = true;
    loadout_changed(r);
    let p = r.playtest.as_ref().unwrap();
    let summary = format!("DEBUG floor {}: {} prior hearts, {} blessings, level {}. {} items remain in stash (F4).", crate::fast_mode::display_depth(r, r.depth), p.prior_hearts, p.prior_blessings, r.level, state(r).unwrap().stash.len());
    crate::engine::log(r, summary, "environment");
    crate::engine::log(r, format!("Exact seed: {} (Ctrl+C copies). Full HP/energy at entry; autosave disabled.",r.seed), "environment");
    crate::engine::monster_floor_entered(r);
    crate::engine::monster_prepare_player_turn(r).map_err(|e|e.0)?;
    crate::engine::remember(r);
    Ok(())
}

#[cfg(test)]
#[path = "revision50_debug_loadout_tests.rs"]
mod revision50_loadout_tests;

#[cfg(test)]
mod fast_mode_queue_tests {
    use super::*;
    #[test]
    fn fast_mode_debug_shared_queue_counts_actual_rewards_without_mutating_items() {
        for fast in [false,true] {
            let mut r=crate::startup::new_run_shell_revision("0-1-2:fast-debug-queue","Avatar",false,52.).unwrap();
            if fast {r.fast_mode=true.into();}
            let mut s=State{pending:VecDeque::new(),consumed:0,stash:vec![],entered:false,loadout_revision:0};
            let mut total=0.;
            let heart=Item{id:401.,kind:"heart".into(),name:"Rat heart".into(),exp:3.0.into(),..Default::default()};
            // Actor, carried and ground collection all call this same queue.
            for depth in [1,3,5] {queue_heart(&r,&mut s,&mut total,depth,heart.clone());}
            assert_eq!(total,if fast {18.}else{9.});assert_eq!(s.pending.len(),3);
            assert!(s.pending.iter().all(|reward|reward.item==heart));
        }
    }
}
