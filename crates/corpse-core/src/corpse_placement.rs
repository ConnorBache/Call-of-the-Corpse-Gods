//! Physical corpse space independent of living actors and items.
use crate::{
    Actor, GroundItem, Run,
    item_placement::{PlacementResult, ground_destination},
};
use std::collections::HashSet;
fn metric(r: &mut Run, k: &str, n: f64) {
    *r.metrics.entry(k.into()).or_default() += n
}
fn sever(r: &mut Run, id: f64) {
    let direct_roots = r.simulation_revision.value_or(29.) >= 51.;
    for f in &mut r.floors {
        if f.fungal_roots
            .as_ref()
            .is_some_and(|roots| roots.iter().any(|root| root.corpse_id == id))
        {
            let f = crate::mutation_journal::make_mut(f);
            for root in f.fungal_roots.as_mut().unwrap() {
                if root.corpse_id == id {
                    root.active = false
                }
            }
            if direct_roots { crate::fungi::clear_orphaned_root_cells(f); }
        }
    }
}
/// O(sum(V+A)) over <=40 floors, plus native generation requested by callback.
pub fn place_corpse(
    r: &mut Run,
    mut depth: f64,
    mut corpse: Actor,
    ensure: impl Fn(&mut Run, f64) -> Result<(), String>,
) -> Result<PlacementResult, String> {
    if !corpse.dead.value_or(false)
        || !depth.is_finite()
        || depth.fract() != 0.0
        || depth < 1.0
        || !corpse.x.is_finite()
        || !corpse.y.is_finite()
    {
        return Err("Invalid corpse placement coordinates/depth/state".into());
    }
    let from = depth;
    let origin = (corpse.x, corpse.y);
    let (mut x, mut y) = origin;
    metric(r, "corpsePlacementCalls", 1.0);
    while depth <= 40.0 {
        ensure(r, depth)?;
        let fi = r
            .floors
            .iter()
            .position(|f| f.depth == depth)
            .ok_or("Corpse placement floor missing")?;
        let f = &r.floors[fi];
        let w = f.width();
        let corpses: Vec<_> = r
            .actors_at(depth)
            .iter()
            .filter(|a| a.dead.value_or(false) && a.id != corpse.id)
            .map(|a| GroundItem {
                x: a.x,
                y: a.y,
                ..Default::default()
            })
            .collect();
        let direct = x.fract() == 0.0
            && y.fract() == 0.0
            && x >= 0.0
            && y >= 0.0
            && x < w as f64
            && y < f.height() as f64
            && ["chasm", "pit"].contains(&f.tiles[y as usize * w + x as usize].as_str());
        if direct && crate::finale::collapsed(r, depth, y as usize*w+x as usize) { break; }
        let destination = if direct {
            Some((true, x, y))
        } else {
            ground_destination(f, &corpses, x, y)
        };
        let cells = f.tiles.len();
        if !direct {
            metric(r, "corpsePlacementCells", cells as f64);
            metric(r, "corpsePlacementOccupancyEntries", corpses.len() as f64);
        }
        crate::mutation_journal::make_mut(&mut r.floors[fi]).dirty = true;
        let Some((fall, nx, ny)) = destination else {
            break;
        };
        if !fall {
            corpse.x = nx;
            corpse.y = ny;
            if depth != from {
                r.actors_at_mut(from).retain(|a| a.id != corpse.id);
            }
            if let Some(existing) = r
                .actors_at_mut(depth)
                .iter_mut()
                .find(|a| a.id == corpse.id)
            {
                *existing = corpse.clone();
            } else {
                r.actors_at_mut(depth).push(corpse.clone());
            }
            if depth != from || corpse.x != origin.0 || corpse.y != origin.1 {
                sever(r, corpse.id)
            }
            return Ok(PlacementResult::Placed {
                depth,
                x: nx,
                y: ny,
            });
        }
        if crate::finale::collapsed(r, depth, ny as usize*w+nx as usize) { break; }
        x = nx;
        y = ny;
        metric(r, "corpsePlacementFalls", 1.0);
        depth += 1.0;
    }
    r.actors_at_mut(from).retain(|a| a.id != corpse.id);
    sever(r, corpse.id);
    metric(r, "corpsePlacementDiscarded", 1.0);
    Ok(PlacementResult::Discarded { depth })
}
pub fn normalize_corpses(
    r: &mut Run,
    ensure: impl Fn(&mut Run, f64) -> Result<(), String>,
) -> Result<(usize, usize), String> {
    let mut depths: Vec<_> = r.floors.iter().map(|f| (f.depth, f.width())).collect();
    depths.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut pending = Vec::new();
    for (depth, w) in depths {
        let mut occupied = HashSet::new();
        r.actors_at_mut(depth).retain(|a| {
            if a.dead.value_or(false) && !occupied.insert((a.y * w as f64 + a.x).to_bits()) {
                pending.push((depth, a.clone()));
                false
            } else {
                true
            }
        });
    }
    let (mut relocated, mut discarded) = (0, 0);
    for (d, a) in pending {
        match place_corpse(r, d, a, &ensure)? {
            PlacementResult::Placed { .. } => relocated += 1,
            PlacementResult::Discarded { .. } => discarded += 1,
        }
    }
    Ok((relocated, discarded))
}
