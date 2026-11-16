//! Ground placement preserves stable entry order and exact fall/metric semantics.
use crate::{Floor, GroundItem, Run};
use std::{collections::HashSet, sync::Arc};
#[derive(Debug, Clone, PartialEq)]
pub enum PlacementResult {
    Placed { depth: f64, x: f64, y: f64 },
    Discarded { depth: f64 },
}
pub fn ground_destination(
    f: &Floor,
    existing: &[GroundItem],
    x: f64,
    y: f64,
) -> Option<(bool, f64, f64)> {
    let w = f.width();
    let occupied: HashSet<_> = existing
        .iter()
        .map(|e| (e.y * w as f64 + e.x).to_bits())
        .collect();
    let integer = x.fract() == 0.0 && y.fract() == 0.0;
    if integer && x >= 0.0 && y >= 0.0 && x < w as f64 && y < f.height() as f64 {
        let at = y as usize * w + x as usize;
        let tile = &f.tiles[at];
        if !["wall", "tree", "chasm", "pit"].contains(&tile.as_str())
            && !occupied.contains(&(at as f64).to_bits())
        {
            return Some((false, (at % w) as f64, (at / w) as f64));
        }
    }
    let (mut ground, mut gap) = (None, None);
    let (mut gd, mut pd) = (f64::INFINITY, f64::INFINITY);
    for (i, tile) in f.tiles.iter().enumerate() {
        if ["wall", "tree"].contains(&tile.as_str()) {
            continue;
        }
        let d = ((i % w) as f64 - x).powi(2) + ((i / w) as f64 - y).powi(2);
        if ["chasm", "pit"].contains(&tile.as_str()) {
            if d < pd {
                gap = Some(i);
                pd = d
            }
        } else if !occupied.contains(&(i as f64).to_bits()) && d < gd {
            ground = Some(i);
            gd = d
        }
    }
    ground
        .or(gap)
        .map(|i| (ground.is_none(), (i % w) as f64, (i / w) as f64))
}
fn metric(r: &mut Run, key: &str, n: f64) {
    *r.metrics.entry(key.into()).or_default() += n;
}
/// O(sum(V+I)) over at most40 visited floors, plus explicit generation callback cost.
pub fn place_ground_item(
    r: &mut Run,
    mut depth: f64,
    entry: GroundItem,
    ensure: impl Fn(&mut Run, f64) -> Result<(), String>,
) -> Result<PlacementResult, String> {
    if !depth.is_finite()
        || depth.fract() != 0.0
        || depth < 1.0
        || !entry.x.is_finite()
        || !entry.y.is_finite()
    {
        return Err("Invalid ground item placement coordinates/depth".into());
    }
    let (mut x, mut y) = (entry.x, entry.y);
    metric(r, "itemPlacementCalls", 1.0);
    while depth <= 40.0 {
        ensure(r, depth)?;
        let fi = r
            .floors
            .iter()
            .position(|f| f.depth == depth)
            .ok_or("Cannot place item: floor was not generated")?;
        let f = &r.floors[fi];
        let w = f.width();
        let direct = x.fract() == 0.0
            && y.fract() == 0.0
            && x >= 0.0
            && y >= 0.0
            && x < w as f64
            && y < f.height() as f64
            && ["chasm", "pit"].contains(&f.tiles[y as usize * w + x as usize].as_str());
        if direct && crate::finale::collapsed(r, depth, y as usize*w+x as usize) { break; }
        let key = depth.to_string();
        let ground = r
            .ground
            .entry(key.clone())
            .or_insert_with(|| Arc::new(vec![]));
        let count = ground.len();
        let cells = f.tiles.len();
        let destination = if direct {
            Some((true, x, y))
        } else {
            // Dream-owned Shards cannot occupy a waking item's placement cell.
            if ground.iter().any(|g| crate::dreamwalking::dream_ground_item(&g.item)) {
                let waking: Vec<_> = ground.iter().filter(|g| !crate::dreamwalking::dream_ground_item(&g.item)).cloned().collect();
                ground_destination(f, &waking, x, y)
            } else { ground_destination(f, ground, x, y) }
        };
        if !direct {
            metric(r, "itemPlacementCells", cells as f64);
            metric(r, "itemPlacementOccupancyEntries", count as f64)
        }
        let Some((fall, nx, ny)) = destination else {
            break;
        };
        crate::mutation_journal::make_mut(&mut r.floors[fi]).dirty = true;
        if !fall {
            let mut placed = entry.clone();
            placed.x = nx;
            placed.y = ny;
            crate::mutation_journal::make_mut(r.ground.get_mut(&key).unwrap()).push(placed);
            return Ok(PlacementResult::Placed {
                depth,
                x: nx,
                y: ny,
            });
        }
        if crate::finale::collapsed(r, depth, ny as usize*w+nx as usize) { break; }
        x = nx;
        y = ny;
        metric(r, "itemPlacementFalls", 1.0);
        depth = crate::fast_mode::below(r, depth);
    }
    metric(r, "itemPlacementDiscarded", 1.0);
    Ok(PlacementResult::Discarded { depth })
}
pub fn normalize_ground_items(
    r: &mut Run,
    ensure: impl Fn(&mut Run, f64) -> Result<(), String>,
) -> Result<(usize, usize), String> {
    let mut depths: Vec<_> = r.floors.iter().map(|f| (f.depth, f.width())).collect();
    depths.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut pending = Vec::new();
    for (d, w) in depths {
        if let Some(entries) = r.ground.get_mut(&d.to_string()) {
            let mut occupied = HashSet::new();
            crate::mutation_journal::make_mut(entries).retain(|e| {
                if crate::dreamwalking::dream_ground_item(&e.item) || occupied.insert((e.y * w as f64 + e.x).to_bits()) {
                    true
                } else {
                    pending.push((d, e.clone()));
                    false
                }
            })
        }
    }
    let (mut relocated, mut discarded) = (0, 0);
    for (d, e) in pending {
        match place_ground_item(r, d, e, &ensure)? {
            PlacementResult::Placed { .. } => relocated += 1,
            PlacementResult::Discarded { .. } => discarded += 1,
        }
    }
    Ok((relocated, discarded))
}
