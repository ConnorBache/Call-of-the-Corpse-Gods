use super::*;
use crate::{Field, Floor};
use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
fn tile(f: &Floor, n: i64) -> Option<&str> {
    if n < 0 {
        None
    } else {
        f.tiles.get(n as usize).map(String::as_str)
    }
}
fn open(t: Option<&str>) -> bool {
    !matches!(t, Some("wall" | "tree" | "chasm"))
}
pub fn remove_nonsensical_doors(f: &mut Floor) -> usize {
    let w = f.width();
    let keys: Vec<String> = f
        .doors
        .as_ref()
        .map(|d| d.keys().cloned().collect())
        .unwrap_or_default();
    let mut removed = 0;
    for key in keys {
        let Ok(at) = key.parse::<usize>() else {
            continue;
        };
        if f.setups
            .as_ref()
            .into_iter()
            .flatten()
            .any(|s| setup_contains(s, (at % w) as i32, (at / w) as i32))
        {
            continue;
        }
        let i = at as i64;
        let stride = w as i64;
        let vertical = tile(f, i - 1) == Some("wall")
            && tile(f, i + 1) == Some("wall")
            && open(tile(f, i - stride))
            && open(tile(f, i + stride));
        let horizontal = tile(f, i - stride) == Some("wall")
            && tile(f, i + stride) == Some("wall")
            && open(tile(f, i - 1))
            && open(tile(f, i + 1));
        if matches!(tile(f, i), Some("floor" | "grass")) && (vertical || horizontal) {
            continue;
        }
        f.doors.as_mut().unwrap().shift_remove(&key);
        removed += 1;
    }
    let surviving: IndexSet<usize> = f
        .doors
        .as_ref()
        .into_iter()
        .flat_map(|d| d.keys().filter_map(|k| k.parse().ok()))
        .collect();
    if let Some(graph) = f.room_graph.as_mut() {
        for room in &mut graph.rooms {
            room.doors.retain(|at| surviving.contains(&(*at as usize)))
        }
        for c in &mut graph.connectors {
            c.doors.retain(|at| surviving.contains(&(*at as usize)))
        }
    } else {
        for room in f.dungeon_rooms.as_mut().into_iter().flatten() {
            room.doors.retain(|at| surviving.contains(&(*at as usize)))
        }
        for c in f.dungeon_connectors.as_mut().into_iter().flatten() {
            c.doors.retain(|at| surviving.contains(&(*at as usize)))
        }
    }
    removed
}
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagonalRepairs {
    pub diagonal_repairs: usize,
    pub protected_diagonal_contacts: usize,
}
/// Monotone geometry repair. At most V productive scans, O(V²) time/O(V) scratch.
pub fn repair_diagonal_openings(
    f: &mut Floor,
    supplied: Option<&IndexSet<usize>>,
) -> DiagonalRepairs {
    let w = f.width();
    let h = f.height();
    let mut protected = supplied
        .cloned()
        .unwrap_or_else(|| protected_generation_cells(f));
    protected.extend(protected_generation_cells(f));
    let geology = geological_repair_costs(f);
    let mut repairs = 0;
    let mut unresolved = IndexSet::new();
    for _ in 0..=f.tiles.len() {
        let mut changed = false;
        unresolved.clear();
        for y in 1..h - 2 {
            for x in 1..w - 2 {
                let a = y * w + x;
                let b = a + 1;
                let c = a + w;
                let d = c + 1;
                let candidates = if open(Some(&f.tiles[a]))
                    && open(Some(&f.tiles[d]))
                    && !open(Some(&f.tiles[b]))
                    && !open(Some(&f.tiles[c]))
                {
                    Some([b, c])
                } else if open(Some(&f.tiles[b]))
                    && open(Some(&f.tiles[c]))
                    && !open(Some(&f.tiles[a]))
                    && !open(Some(&f.tiles[d]))
                {
                    Some([a, d])
                } else if f.tiles[a] != "wall"
                    && f.tiles[d] != "wall"
                    && f.tiles[b] == "wall"
                    && f.tiles[c] == "wall"
                {
                    Some([b, c])
                } else if f.tiles[b] != "wall"
                    && f.tiles[c] != "wall"
                    && f.tiles[a] == "wall"
                    && f.tiles[d] == "wall"
                {
                    Some([a, d])
                } else {
                    None
                };
                let Some(candidates) = candidates else {
                    continue;
                };
                let mut candidates: Vec<usize> = candidates
                    .into_iter()
                    .filter(|n| !protected.contains(n))
                    .collect();
                candidates.sort_by_key(|&at| (terrain_repair_impact(f, at, &geology), at));
                let Some(&at) = candidates.first() else {
                    unresolved.insert(a);
                    continue;
                };
                f.tiles[at] = "floor".into();
                changed = true;
                repairs += 1;
            }
        }
        if !changed {
            break;
        }
    }
    DiagonalRepairs {
        diagonal_repairs: repairs,
        protected_diagonal_contacts: unresolved.len(),
    }
}
/// Conservative O(V³ log V) composition of <=V+1 passes and O(V² log V)
/// connectivity repair; O(V) scratch. Monotone carving can amortize productive work.
pub fn cleanup_generated_floor(f: &mut Floor) -> Result<IndexMap<String, usize>, String> {
    let protected = protected_generation_cells(f);
    let geology = geological_cells(f);
    let fragment_protection: IndexSet<usize> = protected
        .iter()
        .copied()
        .chain(geology.iter().copied())
        .collect();
    let mut bridges: IndexSet<usize> = f
        .generation_bridges
        .as_ref()
        .into_iter()
        .flatten()
        .map(|&n| n as usize)
        .collect();
    let w = f.width();
    let mut removed = remove_nonsensical_doors(f);
    let (mut fragments, mut diagonals, mut contacts, mut passes, mut paths, mut carved, mut visits) =
        (0, 0, 0, 0, 0, 0, 0);
    for _ in 0..=f.tiles.len() {
        passes += 1;
        let before = f.tiles.clone();
        remove_chasm_fragments(&mut f.tiles, &fragment_protection);
        fragments += before
            .iter()
            .enumerate()
            .filter(|(at, t)| t.as_str() == "chasm" && f.tiles[*at] == "floor")
            .count();
        let before_connections = f.tiles.clone();
        let c = connect_generated_rooms(f)?;
        paths += c.repair_paths;
        carved += c.carved_cells;
        visits += c.search_visits;
        let d = repair_diagonal_openings(f, Some(&protected));
        diagonals += d.diagonal_repairs;
        contacts = d.protected_diagonal_contacts;
        for (at, t) in before_connections.iter().enumerate() {
            if t == "chasm" && f.tiles[at] == "floor" {
                bridges.insert(at);
            }
        }
        if before == f.tiles {
            break;
        }
    }
    removed += remove_nonsensical_doors(f);
    let bridges: Vec<f64> = bridges
        .into_iter()
        .filter(|&at| {
            f.tiles[at] == "floor"
                && [
                    at as i64 - 1,
                    at as i64 + 1,
                    at as i64 - w as i64,
                    at as i64 + w as i64,
                ]
                .iter()
                .any(|&n| tile(f, n) == Some("chasm"))
        })
        .map(|at| at as f64)
        .collect();
    let bridge_cells = bridges.len();
    f.generation_bridges = Field::Value(bridges);
    generate_shallow_water(f);
    crate::currents::generate_currents(f);
    apply_water_setup_currents(f);
    let previous = f.substrates.clone();
    let mut substrates = generate_substrates(&f.tiles, &f.domain);
    for at in 0..f.tiles.len() {
        if f.tiles[at] == "floor" {
            if let Some(s) = previous.as_ref().and_then(|v| v.get(at)) {
                if s == "ash" || s == "soot" {
                    substrates[at] = s.clone()
                }
            }
        }
    }
    if let Some(natural) = f.generation.as_mut().and_then(|g| g.natural_walls.as_mut()) {
        natural.retain(|&at| f.tiles[at as usize] == "wall" && !protected.contains(&(at as usize)))
    }
    if f.domain == "Dungeon" {
        for &at in &geology {
            if !protected.contains(&at)
                && ["floor", "grass"].contains(&f.tiles[at].as_str())
                && !["sand", "ash", "soot"].contains(&substrates[at].as_str())
            {
                substrates[at] = "rock".into()
            }
        }
    }
    f.substrates = Field::Value(substrates);
    Ok([
        ("removedDoors", removed),
        ("fragmentCells", fragments),
        ("diagonalRepairs", diagonals),
        ("protectedDiagonalContacts", contacts),
        ("cleanupPasses", passes),
        ("accessRepairPaths", paths),
        ("accessCarvedCells", carved),
        ("accessSearchVisits", visits),
        (
            "preservedGeologicalChasmCells",
            geology.iter().filter(|&&at| f.tiles[at] == "chasm").count(),
        ),
        ("bridgeCells", bridge_cells),
        (
            "currentCells",
            f.currents
                .as_ref()
                .into_iter()
                .flatten()
                .filter(|&&v| v != 0.0)
                .count(),
        ),
    ]
    .into_iter()
    .map(|(k, v)| (k.into(), v))
    .collect())
}
