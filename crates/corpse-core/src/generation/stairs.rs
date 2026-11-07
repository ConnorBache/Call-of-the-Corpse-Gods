use super::{cardinal_neighbors, fire_cells, protected_generation_cells};
use crate::{Field, Floor, FloorLightSourcesEntry, StairRecess};
use indexmap::IndexSet;
use serde::Serialize;
const FACINGS: [(&str, i32, i32); 4] = [
    ("north", 0, -1),
    ("east", 1, 0),
    ("south", 0, 1),
    ("west", -1, 0),
];
#[derive(Debug, Serialize)]
pub struct RecessCells {
    pub walls: Vec<usize>,
    pub torches: Vec<usize>,
    pub landing: Vec<usize>,
    pub footprint: Vec<usize>,
}
pub fn stair_recess_cells(recess: &StairRecess, width: usize) -> Result<RecessCells, String> {
    let &(_, x, y) = FACINGS
        .iter()
        .find(|&&(s, _, _)| s == recess.facing)
        .ok_or("Unknown stair recess facing")?;
    let forward = y * width as i32 + x;
    let side = x * width as i32 - y;
    let at = recess.at as i32;
    let torches = vec![(at - side) as usize, (at + side) as usize];
    let walls = vec![
        (at - forward - side) as usize,
        (at - forward) as usize,
        (at - forward + side) as usize,
        torches[0],
        torches[1],
    ];
    let landing = vec![
        (at + forward - side) as usize,
        (at + forward) as usize,
        (at + forward + side) as usize,
    ];
    let footprint = walls
        .iter()
        .copied()
        .chain(std::iter::once(at as usize))
        .chain(landing.iter().copied())
        .collect();
    Ok(RecessCells {
        walls,
        torches,
        landing,
        footprint,
    })
}
fn components(f: &Floor, walls: &IndexSet<usize>) -> usize {
    let w = f.width();
    let h = f.height();
    let mut excluded = fire_cells(f);
    excluded.extend(f.grates.as_ref().into_iter().flatten().map(|&v| v as usize));
    let passable = |at: usize| {
        !walls.contains(&at)
            && !excluded.contains(&at)
            && !["wall", "tree", "chasm", "up", "down", "pit"].contains(&f.tiles[at].as_str())
    };
    let mut seen = vec![false; f.tiles.len()];
    let mut count = 0;
    for start in w + 1..(h - 1) * w {
        if start % w == 0 || start % w == w - 1 || seen[start] || !passable(start) {
            continue;
        }
        count += 1;
        let mut q = vec![start];
        seen[start] = true;
        let mut head = 0;
        while head < q.len() {
            let at = q[head];
            head += 1;
            for next in cardinal_neighbors(at, w, h) {
                if seen[next] || !passable(next) {
                    continue;
                }
                seen[next] = true;
                q.push(next)
            }
        }
    }
    count
}
/// Four proposed facings per stair, O(V) component scans each, O(V) scratch. All four draws
/// occur before candidate filtering. Protected frame/landing cells are never changed to fit.
pub fn generate_stair_recesses(
    f: &mut Floor,
    r: &mut impl FnMut() -> f64,
) -> Result<Vec<StairRecess>, String> {
    let w = f.width();
    let h = f.height();
    let mut protected = protected_generation_cells(f);
    let mut recesses = vec![];
    for at in 0..f.tiles.len() {
        if f.tiles[at] != "up" && f.tiles[at] != "down" {
            continue;
        }
        if at % w < 2 || at % w > w - 3 || at / w < 2 || at / w > h - 3 {
            return Err("Unsafe stair recess: insufficient interior space".into());
        }
        let mut candidates = vec![];
        for &(facing, x, y) in &FACINGS {
            let recess = StairRecess {
                at: at as f64,
                facing: facing.into(),
            };
            let cells = stair_recess_cells(&recess, w)?;
            let tie = r();
            if !cells.walls.iter().all(|cell| {
                !protected.contains(cell)
                    && !f
                        .doors
                        .as_ref()
                        .is_some_and(|d| d.contains_key(&cell.to_string()))
            }) || !cells.landing.iter().all(|&cell| f.tiles[cell] == "floor")
            {
                continue;
            }
            let forward = y * w as i32 + x;
            let side = x * w as i32 - y;
            let component_count = components(f, &cells.walls.iter().copied().collect());
            let dry_exit = cells
                .landing
                .iter()
                .map(|&cell| cell as i32 + forward)
                .chain([
                    at as i32 + forward - side * 2,
                    at as i32 + forward + side * 2,
                ])
                .filter(|&cell| {
                    cell >= 0
                        && f.tiles
                            .get(cell as usize)
                            .is_some_and(|t| t == "floor" || t == "grass")
                })
                .count();
            let backing = cells
                .walls
                .iter()
                .filter(|&&cell| {
                    let n = cell as i32 - forward;
                    n >= 0 && f.tiles.get(n as usize).is_some_and(|t| t == "wall")
                })
                .count();
            candidates.push((recess, cells, component_count, dry_exit, backing, tie));
        }
        candidates.sort_by(|a, b| {
            (b.3 > 0)
                .cmp(&(a.3 > 0))
                .then(a.2.cmp(&b.2))
                .then(b.3.cmp(&a.3))
                .then(b.4.cmp(&a.4))
                .then(a.5.total_cmp(&b.5))
        });
        let Some((recess, cells, _, _, _, _)) = candidates.into_iter().next() else {
            return Err("Unsafe stair recess: no unoccupied wall frame".into());
        };
        for &cell in &cells.walls {
            f.tiles[cell] = "wall".into()
        }
        let sources = f.light_sources.get_or_insert_with(Vec::new);
        for cell in cells.torches {
            sources.push(FloorLightSourcesEntry {
                x: (cell % w) as f64,
                y: (cell / w) as f64,
                kind: Field::Value("torch".into()),
                power: 75.0,
                falloff: 10.0,
            })
        }
        protected.extend(cells.footprint);
        recesses.push(recess);
    }
    if let Some(g) = f.generation.as_mut() {
        g.stair_recesses = Field::Value(recesses.clone())
    }
    Ok(recesses)
}
