use crate::{Floor, FloorSetupsEntry};
use indexmap::IndexSet;
pub fn setup_contains(s: &FloorSetupsEntry, x: i32, y: i32) -> bool {
    if let Some(footprint) = s.footprint.as_ref() {
        let dx = x - s.x as i32 + 6;
        let dy = y - s.y as i32 + 6;
        if !(0..13).contains(&dx) || !(0..13).contains(&dy) {
            return false;
        }
        let local = (dy * 13 + dx) as f64;
        return footprint.binary_search_by(|n| n.total_cmp(&local)).is_ok();
    }
    let (left, right, top, bottom) = if let Some(b) = s.bounds.as_ref() {
        (b.left, b.right, b.top, b.bottom)
    } else {
        (s.x - 6.0, s.x + 6.0, s.y - 6.0, s.y + 6.0)
    };
    x as f64 >= left && x as f64 <= right && y as f64 >= top && y as f64 <= bottom
}
/// O(setup footprint + features + doors) with bounded 13x13 setup scans.
/// Explicit IndexSet order matches insertion provenance; membership does not use hash iteration.
pub fn protected_generation_cells(f: &Floor) -> IndexSet<usize> {
    let w = f.width() as i32;
    let mut result = IndexSet::new();
    result.extend(crate::pocket_biomes::reserved(f));
    if let Some(g) = f.generation.as_ref() {
        if let Some(recesses) = g.stair_recesses.as_ref() {
            for recess in recesses {
                for y in -1..=1 {
                    for x in -1..=1 {
                        result.insert((recess.at as i32 + y * w + x) as usize);
                    }
                }
            }
        }
    }
    for s in f.setups.as_ref().into_iter().flatten() {
        if let Some(footprint) = s.footprint.as_ref() {
            for &local in footprint {
                let local = local as i32;
                result.insert(
                    ((s.y as i32 + local / 13 - 6) * w + s.x as i32 + local % 13 - 6) as usize,
                );
            }
            continue;
        }
        let (left, right, top, bottom) = if let Some(b) = s.bounds.as_ref() {
            (b.left as i32, b.right as i32, b.top as i32, b.bottom as i32)
        } else {
            (
                s.x as i32 - 6,
                s.x as i32 + 6,
                s.y as i32 - 6,
                s.y as i32 + 6,
            )
        };
        for y in top..=bottom {
            for x in left..=right {
                if setup_contains(s, x, y) {
                    result.insert((y * w + x) as usize);
                }
            }
        }
    }
    if let Some(e) = f.godbeast_event.as_ref() {
        for y in e.room.top as i32..=e.room.bottom as i32 {
            for x in e.room.left as i32..=e.room.right as i32 {
                result.insert((y * w + x) as usize);
            }
        }
        if let Some(p) = e.tap.as_ref() {
            result.insert((p.y as i32 * w + p.x as i32) as usize);
        }
        if let Some(p) = e.well.as_ref() {
            result.insert((p.y as i32 * w + p.x as i32) as usize);
        }
        if let Some(p) = e.lever.as_ref() {
            result.insert((p.y as i32 * w + p.x as i32) as usize);
        }
    }
    for &at in f
        .grates
        .as_ref()
        .into_iter()
        .flatten()
        .chain(f.permanent_fire.as_ref().into_iter().flatten())
    {
        result.insert(at as usize);
    }
    for v in f.vents.as_ref().into_iter().flatten() {
        result.insert(v.at as usize);
    }
    for s in f.light_sources.as_ref().into_iter().flatten() {
        result.insert((s.y as i32 * w + s.x as i32) as usize);
    }
    if let Some(doors) = f.doors.as_ref() {
        let mut keys: Vec<usize> = doors.keys().filter_map(|k| k.parse().ok()).collect();
        keys.sort_unstable();
        for at in keys {
            for n in [
                at as i64 - 1,
                at as i64 + 1,
                at as i64 - w as i64,
                at as i64 + w as i64,
            ] {
                if n >= 0 && f.tiles.get(n as usize).is_some_and(|t| t == "wall") {
                    result.insert(n as usize);
                }
            }
        }
    }
    result
}
pub fn geological_cells(f: &Floor) -> IndexSet<usize> {
    f.generation
        .as_ref()
        .and_then(|g| g.landmarks.as_ref())
        .into_iter()
        .flatten()
        .flat_map(|l| l.cells.iter().map(|&n| n as usize))
        .collect()
}
pub fn geological_repair_costs(f: &Floor) -> Vec<u8> {
    let mut costs = vec![0; f.tiles.len()];
    for l in f
        .generation
        .as_ref()
        .and_then(|g| g.landmarks.as_ref())
        .into_iter()
        .flatten()
    {
        let c = match l.scale.as_str() {
            "major" => 12,
            "medium" => 6,
            "minor" => 3,
            _ => 0,
        };
        for &at in &l.cells {
            if let Some(v) = costs.get_mut(at as usize) {
                *v = c
            }
        }
    }
    costs
}
pub fn terrain_repair_impact(f: &Floor, at: usize, geology: &[u8]) -> usize {
    let base = match f.tiles[at].as_str() {
        "wall" => 4,
        "tree" => 2,
        "chasm" => 12,
        _ => return 0,
    };
    base + geology[at] as usize
}
