//! Exact optional guarded-room search; no runtime terrain approximation.
use crate::{Field, Floor, FloorSphinxRoom, rng::GenerationRng};
use indexmap::IndexSet;
use std::collections::HashSet;
const DIRS: [(i32, i32); 8] = [
    (0, -1),
    (1, 0),
    (0, 1),
    (-1, 0),
    (1, -1),
    (1, 1),
    (-1, 1),
    (-1, -1),
];
fn flood(f: &Floor, start: usize, blocked: usize, tiles: &[String]) -> IndexSet<usize> {
    let (w, h) = (f.width() as i32, f.height() as i32);
    let hazards: HashSet<_> = f
        .permanent_fire
        .as_ref()
        .into_iter()
        .flatten()
        .chain(f.grates.as_ref().into_iter().flatten())
        .map(|v| *v as usize)
        .chain(
            f.light_sources
                .as_ref()
                .into_iter()
                .flatten()
                .filter(|s| s.kind.as_ref().is_some_and(|k| k == "brazier"))
                .map(|s| s.y as usize * w as usize + s.x as usize),
        )
        .collect();
    let passable =
        |i: usize| !["wall", "tree", "chasm"].contains(&tiles[i].as_str()) && !hazards.contains(&i);
    let mut found = IndexSet::new();
    if start >= tiles.len() || start == blocked || !passable(start) {
        return found;
    }
    found.insert(start);
    let mut head = 0;
    while head < found.len() {
        let at = found[head];
        head += 1;
        if at != start && ["up", "down", "pit"].contains(&tiles[at].as_str()) {
            continue;
        }
        let (x, y) = ((at % w as usize) as i32, (at / w as usize) as i32);
        for (dx, dy) in DIRS {
            let (xx, yy) = (x + dx, y + dy);
            if xx < 0 || yy < 0 || xx >= w || yy >= h {
                continue;
            }
            let next = (yy * w + xx) as usize;
            if next == blocked
                || found.contains(&next)
                || !passable(next)
                || (dx != 0
                    && dy != 0
                    && (tiles[(y * w + xx) as usize] == "wall"
                        || tiles[(yy * w + x) as usize] == "wall"))
            {
                continue;
            }
            found.insert(next);
        }
    }
    found
}
fn protected(f: &Floor) -> IndexSet<usize> {
    let mut p = crate::generation::protected_generation_cells(f);
    let w = f.width();
    p.extend(
        f.doors
            .as_ref()
            .into_iter()
            .flat_map(|d| d.keys())
            .filter_map(|k| k.parse::<usize>().ok()),
    );
    p.extend(
        f.setup_spawns
            .as_ref()
            .into_iter()
            .flatten()
            .map(|s| s.y as usize * w + s.x as usize),
    );
    p.extend(
        f.generation
            .as_ref()
            .and_then(|g| g.terrain_spine.as_ref())
            .into_iter()
            .flatten()
            .map(|i| *i as usize),
    );
    for i in 0..f.tiles.len() {
        let layer = |a: &Field<Vec<f64>>| a.as_ref().is_some_and(|v| v[i] != 0.0);
        if layer(&f.roots)
            || layer(&f.slime)
            || layer(&f.mud)
            || layer(&f.ichor)
            || layer(&f.razorleaves)
            || f.environment.as_ref().is_some_and(|e| {
                e.fire[i] != 0.0 || e.gas_type.as_ref().is_some_and(|v| v[i].is_some())
            })
            || f.bulwarks
                .as_ref()
                .is_some_and(|v| v.contains_key(&i.to_string()))
        {
            p.insert(i);
        }
    }
    p
}
fn rewards(cells: &[usize], w: usize, x: usize, y: usize) -> Vec<f64> {
    let mut r: Vec<_> = cells
        .iter()
        .copied()
        .filter(|i| (i % w).abs_diff(x).max((i / w).abs_diff(y)) > 1)
        .collect();
    let score = |i: usize| (i % w).abs_diff(x).pow(2) + (i / w).abs_diff(y).pow(2);
    r.sort_by(|a, b| score(*b).cmp(&score(*a)).then(a.cmp(b)));
    r.truncate(4);
    r.into_iter().map(|i| i as f64).collect()
}
/// Existing doors cost O(D*V); fallback fixed wall stencils cost O(V); O(V) memory.
pub fn plan_sphinx_room(f: &mut Floor, rng: &mut GenerationRng) -> Option<FloorSphinxRoom> {
    if let Some(r) = f.sphinx_room.as_ref() {
        return Some(r.clone());
    }
    if f.depth < 12.0 || rng.random() >= 0.2 {
        return None;
    }
    let up = f.tiles.iter().position(|t| t == "up")?;
    let (w, h) = (f.width(), f.height());
    let baseline = flood(f, up, usize::MAX, &f.tiles);
    let protected = protected(f);
    let mut candidates = Vec::new();
    let mut doors: Vec<_> = f
        .doors
        .as_ref()
        .into_iter()
        .flat_map(|d| d.keys())
        .filter_map(|k| k.parse::<usize>().ok())
        .collect();
    doors.sort_unstable();
    for door in doors {
        if !baseline.contains(&door) {
            continue;
        }
        let main = flood(f, up, door, &f.tiles);
        let cells: Vec<_> = baseline
            .iter()
            .copied()
            .filter(|i| *i != door && !main.contains(i))
            .collect();
        if cells.len() < 6
            || cells.len() > 100
            || cells.iter().any(|i| {
                ["up", "down", "pit"].contains(&f.tiles[*i].as_str()) || protected.contains(i)
            })
        {
            continue;
        }
        let reward_cells = rewards(&cells, w, door % w, door / w);
        if reward_cells.len() == 4 {
            let mut all = vec![door as f64];
            all.extend(cells.into_iter().map(|v| v as f64));
            candidates.push(FloorSphinxRoom {
                x: (door % w) as f64,
                y: (door / w) as f64,
                cells: all,
                reward_cells,
            });
        }
    }
    if !candidates.is_empty() {
        return Some(candidates[(rng.random() * candidates.len() as f64).floor() as usize].clone());
    }
    let mut carvings = Vec::new();
    for approach in &baseline {
        if !["floor", "grass", "shallow-water"].contains(&f.tiles[*approach].as_str())
            || protected.contains(approach)
        {
            continue;
        }
        let (ax, ay) = ((*approach % w) as i32, (*approach / w) as i32);
        for (dx, dy) in &DIRS[..4] {
            for offset in [-1, 0] {
                let (x, y) = (ax + dx, ay + dy);
                let (sx, sy) = (-dy, *dx);
                let mut footprint = Vec::new();
                let mut cells = Vec::new();
                let mut valid = true;
                for depth in 0..=4 {
                    if !valid {
                        break;
                    }
                    for across in offset - 1..=offset + 2 {
                        let (px, py) = (x + dx * depth + sx * across, y + dy * depth + sy * across);
                        if px < 1 || py < 1 || px >= w as i32 - 1 || py >= h as i32 - 1 {
                            valid = false;
                            break;
                        }
                        let i = py as usize * w + px as usize;
                        if f.tiles[i] != "wall" || protected.contains(&i) {
                            valid = false;
                            break;
                        }
                        footprint.push(i);
                        if (depth == 0 && across == 0)
                            || ((1..=3).contains(&depth) && across >= offset && across < offset + 2)
                        {
                            cells.push(i)
                        }
                    }
                }
                if !valid {
                    continue;
                }
                let reward_cells = rewards(&cells, w, x as usize, y as usize);
                if reward_cells.len() == 4 {
                    carvings.push((
                        FloorSphinxRoom {
                            x: x as f64,
                            y: y as f64,
                            cells: cells.into_iter().map(|v| v as f64).collect(),
                            reward_cells,
                        },
                        footprint,
                    ));
                }
            }
        }
    }
    if carvings.is_empty() {
        return None;
    }
    let (chosen, footprint) =
        carvings.swap_remove((rng.random() * carvings.len() as f64).floor() as usize);
    let door = chosen.y as usize * w + chosen.x as usize;
    let mut proposed = f.tiles.clone();
    for i in &chosen.cells {
        proposed[*i as usize] = "floor".into()
    }
    let guarded = flood(f, up, door, &proposed);
    let unguarded = flood(f, up, usize::MAX, &proposed);
    if baseline.iter().any(|i| !guarded.contains(i))
        || chosen
            .reward_cells
            .iter()
            .any(|i| guarded.contains(&(*i as usize)) || !unguarded.contains(&(*i as usize)))
    {
        return None;
    }
    f.tiles = proposed;
    if let Some(s) = f.substrates.as_mut() {
        for i in &footprint {
            s[*i] = "tiled".into()
        }
    }
    if let Some(n) = f.generation.as_mut().and_then(|g| g.natural_walls.as_mut()) {
        let constructed: HashSet<_> = footprint.into_iter().collect();
        n.retain(|i| !constructed.contains(&(*i as usize)));
    }
    f.doors.get_or_insert_with(Default::default).insert(
        door.to_string(),
        crate::FloorDoorsEntry {
            open: false,
            ..Default::default()
        },
    );
    f.dirty = true;
    Some(chosen)
}
