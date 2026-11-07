//! Exact generation-only terrain cleanup; no runtime movement changes.
use super::setup_contains;
use crate::Floor;
use indexmap::IndexSet;
pub fn generate_substrates(tiles: &[String], domain: &str) -> Vec<String> {
    let w = if tiles.len() == 2291 {
        79
    } else {
        (tiles.len() as f64).sqrt() as usize
    };
    let base = if domain == "Dungeon" {
        "tiled"
    } else if ["Cave", "Mountain"].contains(&domain) {
        "rock"
    } else {
        "earth"
    };
    tiles
        .iter()
        .enumerate()
        .map(|(i, t)| {
            if t == "floor"
                && [
                    i.checked_sub(1),
                    i.checked_add(1),
                    i.checked_sub(w),
                    i.checked_add(w),
                ]
                .into_iter()
                .flatten()
                .any(|j| {
                    tiles
                        .get(j)
                        .is_some_and(|t| t == "water" || t == "shallow-water")
                })
            {
                "sand"
            } else {
                base
            }
            .into()
        })
        .collect()
}
fn dry(t: &str) -> bool {
    ["floor", "grass", "up", "down", "pit"].contains(&t)
}
fn adjacent(i: usize, w: usize, h: usize) -> Vec<usize> {
    let x = (i % w) as i32;
    let y = (i / w) as i32;
    let mut out = vec![];
    for dy in -1..=1 {
        for dx in -1..=1 {
            if (dx != 0 || dy != 0)
                && x + dx > 0
                && x + dx < w as i32 - 1
                && y + dy > 0
                && y + dy < h as i32 - 1
            {
                out.push((y + dy) as usize * w + (x + dx) as usize)
            }
        }
    }
    out
}
/// Snapshot-based one-cell shore apron: O(V+F), O(V+F) scratch, F authored fixtures.
pub fn generate_shallow_water(f: &mut Floor) {
    let w = f.width();
    let h = f.height();
    let before = f.tiles.clone();
    let mut protected = IndexSet::new();
    for c in f.dungeon_connectors.as_ref().into_iter().flatten() {
        protected.extend(c.cells.iter().map(|&i| i as usize));
    }
    protected.extend(
        f.vents
            .as_ref()
            .into_iter()
            .flatten()
            .map(|v| v.at as usize),
    );
    protected.extend(f.grates.as_ref().into_iter().flatten().map(|&i| i as usize));
    protected.extend(
        f.permanent_fire
            .as_ref()
            .into_iter()
            .flatten()
            .map(|&i| i as usize),
    );
    if let Some(doors) = f.doors.as_ref() {
        protected.extend(doors.keys().filter_map(|k| k.parse::<usize>().ok()));
    }
    protected.extend(
        f.light_sources
            .as_ref()
            .into_iter()
            .flatten()
            .map(|s| s.y as usize * w + s.x as usize),
    );
    protected.extend(
        f.setup_spawns
            .as_ref()
            .into_iter()
            .flatten()
            .map(|s| s.y as usize * w + s.x as usize)
            .filter(|&i| dry(&before[i])),
    );
    for (at, t) in before.iter().enumerate() {
        if ["up", "down", "pit"].contains(&t.as_str()) {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    protected.insert((at as i64 + dy * w as i64 + dx) as usize);
                }
            }
        }
    }
    let mut owner = vec![-1i32; f.tiles.len()];
    for (id, s) in f.setups.as_ref().into_iter().flatten().enumerate() {
        let Some(b) = s.bounds.as_ref() else { continue };
        for y in b.top as i32..=b.bottom as i32 {
            for x in b.left as i32..=b.right as i32 {
                if !setup_contains(s, x, y) {
                    continue;
                }
                let i = y as usize * w + x as usize;
                owner[i] = if owner[i] == -1 { id as i32 } else { -2 };
                if dry(&before[i])
                    && (s.id < 101.0
                        || s.id > 108.0
                        || !setup_contains(s, x - 1, y)
                        || !setup_contains(s, x + 1, y)
                        || !setup_contains(s, x, y - 1)
                        || !setup_contains(s, x, y + 1))
                {
                    protected.insert(i);
                }
            }
        }
    }
    if let Some(e) = f.godbeast_event.as_ref() {
        for y in e.room.top as usize..=e.room.bottom as usize {
            for x in e.room.left as usize..=e.room.right as usize {
                protected.insert(y * w + x);
            }
        }
        protected.insert(e.y as usize * w + e.x as usize);
        if let Some(p) = e.well.as_ref() {
            protected.insert(p.y as usize * w + p.x as usize);
        }
        if let Some(p) = e.lever.as_ref() {
            protected.insert(p.y as usize * w + p.x as usize);
        }
        if let Some(p) = e.tap.as_ref() {
            protected.insert(p.y as usize * w + p.x as usize);
        }
    }
    for i in 0..before.len() {
        if before[i] != "water" {
            continue;
        }
        for j in adjacent(i, w, h) {
            if (before[j] == "floor" || before[j] == "grass")
                && !protected.contains(&j)
                && (owner[j] == -1 || owner[j] == owner[i])
            {
                f.tiles[j] = "shallow-water".into();
            }
        }
    }
    let fringe: Vec<usize> = (0..f.tiles.len())
        .filter(|&i| {
            f.tiles[i] == "water" && adjacent(i, w, h).into_iter().any(|j| dry(&f.tiles[j]))
        })
        .collect();
    for i in fringe {
        f.tiles[i] = "shallow-water".into()
    }
    if let Some(currents) = f.currents.as_mut() {
        for i in 0..f.tiles.len() {
            if f.tiles[i] == "shallow-water" {
                currents[i] = 0.0
            }
        }
    }
}
/// Eight-connected components; protected or >=12-cell chasms survive. O(V) time/storage.
pub fn remove_chasm_fragments(tiles: &mut [String], protected: &IndexSet<usize>) {
    let w = if tiles.len() == 2291 {
        79
    } else {
        (tiles.len() as f64).sqrt() as usize
    };
    let h = tiles.len() / w;
    let mut seen = vec![false; tiles.len()];
    for i in 0..tiles.len() {
        if tiles[i] != "chasm" || seen[i] {
            continue;
        }
        let mut cells = vec![i];
        seen[i] = true;
        let mut head = 0;
        while head < cells.len() {
            let at = cells[head];
            head += 1;
            let x = (at % w) as i32;
            let y = (at / w) as i32;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let xx = x + dx;
                    let yy = y + dy;
                    if xx < 0 || yy < 0 || xx >= w as i32 || yy >= h as i32 {
                        continue;
                    }
                    let j = yy as usize * w + xx as usize;
                    if seen[j] || tiles[j] != "chasm" {
                        continue;
                    }
                    seen[j] = true;
                    cells.push(j)
                }
            }
        }
        if cells.len() < 12 && !cells.iter().any(|at| protected.contains(at)) {
            for at in cells {
                tiles[at] = "floor".into()
            }
        }
    }
}
/// Applies the authored 101..108 current lanes after natural drainage. O(V+S*169).
pub fn apply_water_setup_currents(f: &mut Floor) {
    apply_water_setup_currents_revision(f,39.0)
}
/// Existing bounded authored setup work plus O(V × destination eligibility).
pub fn apply_water_setup_currents_revision(f: &mut Floor, revision:f64) {
    let w = f.width();
    let n = f.tiles.len();
    f.currents.get_or_insert_with(|| vec![0.0; n]);
    for s in f.setups.as_ref().into_iter().flatten() {
        if s.id < 101.0 || s.id > 108.0 {
            continue;
        }
        let at = |dx: i32, dy: i32| ((s.y as i32 + dy) * w as i32 + s.x as i32 + dx) as usize;
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
        let currents = f.currents.as_mut().unwrap();
        for yy in top..=bottom {
            for xx in left..=right {
                if setup_contains(s, xx, yy) {
                    currents[yy as usize * w + xx as usize] = 0.0;
                }
            }
        }
        let mut flow = |dx: i32, dy: i32, code: f64| {
            let i = at(dx, dy);
            if f.tiles[i] == "water" {
                currents[i] = code
            }
        };
        if s.id == 102.0 {
            for dx in -2..=2 {
                flow(dx, -1, 3.0);
                flow(dx, 1, 7.0)
            }
        } else {
            for dx in -2..=2 {
                flow(dx, 0, 3.0)
            }
        }
        if s.id == 105.0 {
            flow(1, 0, 5.0);
            flow(1, 1, 5.0);
            flow(1, 2, 5.0);
            for dx in 1..=3 {
                flow(dx, 3, 3.0)
            }
            flow(4, 3, 1.0);
            flow(4, 2, 1.0)
        }
        if s.id == 106.0 {
            for dy in [-3, 3] {
                for dx in -2..0 {
                    flow(dx, dy, 3.0)
                }
            }
            for dy in -3..0 {
                flow(0, dy, 5.0)
            }
            for dy in 1..=3 {
                flow(0, dy, 1.0)
            }
        }
        if s.id == 108.0 {
            for dx in -4..=1 {
                for dy in -2..=1 {
                    f.lights[at(dx, dy)] = (-70.0 - 2.0 * (f.depth - 1.0)).max(-99.0)
                }
            }
        }
    }
    for i in 0..n {
        if f.currents.as_ref().unwrap()[i] != 0.0
            && crate::environment::current_destination_revision(f, i, revision).is_none()
        {
            f.currents.as_mut().unwrap()[i] = 0.0
        }
    }
}
