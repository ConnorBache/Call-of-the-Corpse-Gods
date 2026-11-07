//! Revision33 deeper Zillennia clear only loose ground contents they occupy.
//! No aura, damage, inventory/corpse mutation, liquid mutation or topology removal.
use crate::{Actor, Floor, Run};
use std::collections::HashSet;

/// The canonical quality bands start Greater at authored depth21. Follow the
/// actor's quality, never its current floor (including stair/fall transfers).
pub fn clears_ground(r: &Run, a: &Actor) -> bool {
    r.simulation_revision.value_or(29.0) >= 33.0
        && !a.dead.value_or(false)
        && a.hp > 0.0
        && a.creature_kind.as_ref().is_some_and(|k| k == "zillennia")
        && a.zillennia_quality.as_ref().is_some_and(|q| {
            matches!(q.as_str(), "greater" | "hulking" | "ancient" | "timeless" | "transcendent")
        })
}

fn cell(f: &Floor, x: f64, y: f64) -> Option<usize> {
    (x.is_finite() && y.is_finite() && x.fract() == 0.0 && y.fract() == 0.0
        && x >= 0.0 && y >= 0.0 && x < f.width() as f64 && y < f.height() as f64)
        .then(|| y as usize * f.width() + x as usize)
}

/// Ordinary floor/grass only: other topology, water and Ichor-bearing cells retain
/// their covers. Slime here is the land surface layer, never a liquid tile.
fn land_cover(f: &Floor, at: usize) -> bool {
    matches!(f.tiles[at].as_str(), "floor" | "grass")
        && !f.ichor.as_ref().is_some_and(|v| v[at] > 0.0)
}
fn covered(f: &Floor, at: usize) -> bool {
    land_cover(f, at)
        && (f.tiles[at] == "grass"
            || f.snow.as_ref().is_some_and(|v| v[at])
            || f.slime.as_ref().is_some_and(|v| v[at] != 0.0)
            || f.blood.as_ref().is_some_and(|v| v[at]))
}

/// O(F + G + Z) expected time for F floors, G ground entries and Z touched cells;
/// O(Z) scratch, plus existing Arc COW copies only for changed floor/ground data.
/// Retain preserves surviving item order. Knowledge is updated only by remember.
fn erase_cells(r: &mut Run, depth: f64, cells: &HashSet<usize>) -> bool {
    if cells.is_empty() { return false; }
    let Some(fi) = r.floors.iter().position(|f| f.depth == depth) else { return false };
    let f = &r.floors[fi];
    let key = depth.to_string();
    let dream = crate::dreamwalking::context_dream(r);
    let plane_items = crate::borge::weapon_enabled(r);
    let has_items = r.ground.get(&key).is_some_and(|ground| ground.iter().any(|g| {
        (!plane_items || crate::dreamwalking::dream_ground_item(&g.item) == dream) && cell(f, g.x, g.y).is_some_and(|at| cells.contains(&at))
    }));
    let has_cover = cells.iter().any(|&at| covered(f, at));
    if !has_items && !has_cover { return false; }
    if has_items {
        crate::mutation_journal::make_mut(r.ground.get_mut(&key).unwrap()).retain(|g| {
            (plane_items && crate::dreamwalking::dream_ground_item(&g.item) != dream) || !cell(f, g.x, g.y).is_some_and(|at| cells.contains(&at))
        });
    }
    let f = crate::mutation_journal::make_mut(&mut r.floors[fi]);
    if has_cover {
        for &at in cells {
            if !land_cover(f, at) { continue; }
            if f.tiles[at] == "grass" { f.tiles[at] = "floor".into(); }
            if let Some(v) = f.snow.as_mut() { v[at] = false; }
            if let Some(v) = f.slime.as_mut() { v[at] = 0.0; }
            if let Some(v) = f.blood.as_mut() { v[at] = false; }
        }
    }
    f.dirty = true;
    true
}

/// Arrival hook shared by normal, forced, haste and offscreen movement.
pub fn on_arrival(r: &mut Run, id: f64) -> bool {
    if r.simulation_revision.value_or(29.0) < 33.0 { return false; }
    let Some(a) = crate::engine::actor(r, id).filter(|a| clears_ground(r, a)) else { return false };
    let Some(at) = r.floor().and_then(|f| cell(f, a.x, a.y)) else { return false };
    erase_cells(r, r.depth, &HashSet::from([at]))
}

/// Initial generated placement, including floors not currently being observed.
/// O(A + F + G + Z) expected time, plus the explicitly documented COW cost.
pub fn initialize_floor(r: &mut Run, depth: f64) -> bool {
    if r.simulation_revision.value_or(29.0) < 33.0 { return false; }
    let Some(f) = r.floors.iter().find(|f| f.depth == depth) else { return false };
    let cells = r.actors_at(depth).iter().filter(|a| clears_ground(r, a))
        .filter_map(|a| cell(f, a.x, a.y)).collect();
    erase_cells(r, depth, &cells)
}
