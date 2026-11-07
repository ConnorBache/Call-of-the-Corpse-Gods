use super::{geological_repair_costs, protected_generation_cells, terrain_repair_impact};
use crate::Floor;
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
pub fn terrain_passable(t: &str) -> bool {
    !["wall", "tree", "chasm"].contains(&t)
}
pub fn cardinal_neighbors(i: usize, w: usize, h: usize) -> Vec<usize> {
    let x = i % w;
    let y = i / w;
    [
        if x > 1 { Some(i - 1) } else { None },
        if x < w - 2 { Some(i + 1) } else { None },
        if y > 1 { Some(i - w) } else { None },
        if y < h - 2 { Some(i + w) } else { None },
    ]
    .into_iter()
    .flatten()
    .collect()
}
pub fn automatic_transitions(f: &Floor) -> Vec<usize> {
    f.tiles
        .iter()
        .enumerate()
        .filter_map(|(i, t)| {
            if ["up", "down", "pit"].contains(&t.as_str()) {
                Some(i)
            } else {
                None
            }
        })
        .collect()
}
pub fn transitions_accessible(f: &Floor, reachable: &IndexSet<usize>) -> bool {
    let transitions: IndexSet<usize> = automatic_transitions(f).into_iter().collect();
    transitions.iter().all(|&i| {
        cardinal_neighbors(i, f.width(), f.height())
            .iter()
            .any(|j| reachable.contains(j) && !transitions.contains(j))
    })
}
pub fn fire_cells(f: &Floor) -> IndexSet<usize> {
    let mut cells: IndexSet<usize> = f
        .permanent_fire
        .as_ref()
        .into_iter()
        .flatten()
        .map(|&n| n as usize)
        .collect();
    for s in f.light_sources.as_ref().into_iter().flatten() {
        if s.kind.as_ref().is_some_and(|s| s == "brazier") {
            cells.insert(s.y as usize * f.width() + s.x as usize);
        }
    }
    cells
}
pub fn sealed_vent_hardware(f: &Floor) -> IndexSet<usize> {
    let vents: IndexSet<usize> = f
        .vents
        .as_ref()
        .into_iter()
        .flatten()
        .map(|v| v.at as usize)
        .collect();
    f.setups
        .as_ref()
        .into_iter()
        .flatten()
        .filter(|s| s.id == 18.0)
        .map(|s| s.y as usize * f.width() + s.x as usize + 1)
        .filter(|at| vents.contains(at))
        .collect()
}
pub fn reachable_without_braziers(f: &Floor, extra: Option<usize>) -> IndexSet<usize> {
    let w = f.width();
    let h = f.height();
    let mut blocked = fire_cells(f);
    if let Some(at) = extra {
        blocked.insert(at);
    }
    blocked.extend(f.grates.as_ref().into_iter().flatten().map(|&n| n as usize));
    blocked.extend(automatic_transitions(f));
    let Some(upstairs) = f.tiles.iter().position(|t| t == "up") else {
        return IndexSet::new();
    };
    let mut seen = IndexSet::from([upstairs]);
    let start = cardinal_neighbors(upstairs, w, h)
        .into_iter()
        .find(|i| !blocked.contains(i) && terrain_passable(&f.tiles[*i]));
    let Some(start) = start else { return seen };
    let mut queue = vec![start];
    seen.insert(start);
    let mut head = 0;
    while head < queue.len() {
        let at = queue[head];
        head += 1;
        for j in cardinal_neighbors(at, w, h) {
            if !seen.contains(&j) && !blocked.contains(&j) && terrain_passable(&f.tiles[j]) {
                seen.insert(j);
                queue.push(j)
            }
        }
    }
    seen
}
#[derive(Default, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectivityRepairs {
    pub repair_paths: usize,
    pub carved_cells: usize,
    pub bridge_cells: usize,
    pub search_visits: usize,
}
/// Stable Dijkstra ordering by (impact*(V+1)+distance, cell). O(V log V), O(V) scratch.
fn minimum_impact_route(
    f: &Floor,
    starts: &[usize],
    target: &IndexSet<usize>,
    protected: &IndexSet<usize>,
    blocked: &IndexSet<usize>,
    geology: &[u8],
    stats: &mut ConnectivityRepairs,
) -> Option<Vec<usize>> {
    let v = f.tiles.len();
    let mut distance = vec![usize::MAX; v];
    let mut previous = vec![usize::MAX; v];
    let mut heap = BinaryHeap::new();
    let allowed = |at: usize| {
        !blocked.contains(&at) && (!protected.contains(&at) || terrain_passable(&f.tiles[at]))
    };
    for &at in starts {
        if !allowed(at) {
            continue;
        }
        let cost = terrain_repair_impact(f, at, geology) * (v + 1);
        distance[at] = cost;
        previous[at] = at;
        heap.push(Reverse((cost, at)));
    }
    while let Some(Reverse((cost, at))) = heap.pop() {
        if distance[at] != cost {
            continue;
        }
        stats.search_visits += 1;
        if target.contains(&at) {
            let mut path = vec![at];
            while previous[*path.last().unwrap()] != *path.last().unwrap() {
                path.push(previous[*path.last().unwrap()])
            }
            return Some(path);
        }
        for next in cardinal_neighbors(at, f.width(), f.height()) {
            if !allowed(next) {
                continue;
            }
            let candidate = cost + terrain_repair_impact(f, next, geology) * (v + 1) + 1;
            if candidate >= distance[next] {
                continue;
            }
            distance[next] = candidate;
            previous[next] = at;
            heap.push(Reverse((candidate, next)));
        }
    }
    None
}
fn carve(f: &mut Floor, path: &[usize], stats: &mut ConnectivityRepairs) -> Result<(), String> {
    let mut changed = 0;
    for &at in path {
        if !terrain_passable(&f.tiles[at]) {
            if f.tiles[at] == "chasm" {
                stats.bridge_cells += 1
            }
            f.tiles[at] = "floor".into();
            stats.carved_cells += 1;
            changed += 1
        }
    }
    if changed == 0 {
        return Err("Access repair did not consume solid terrain".into());
    }
    stats.repair_paths += 1;
    Ok(())
}
/// At most V productive terrain-consuming searches: O(V² log V), O(V) scratch.
pub fn connect_generated_rooms(f: &mut Floor) -> Result<ConnectivityRepairs, String> {
    let w = f.width();
    let h = f.height();
    let protected = protected_generation_cells(f);
    let geology = geological_repair_costs(f);
    let mut blocked = fire_cells(f);
    let mut stats = ConnectivityRepairs::default();
    blocked.extend(f.grates.as_ref().into_iter().flatten().map(|&n| n as usize));
    blocked.extend(automatic_transitions(f));
    if let Some(e) = f.godbeast_event.as_ref() {
        if e.kind == "Veridian" {
            blocked.insert(e.y as usize * w + e.x as usize);
        }
    }
    blocked.extend((0..f.tiles.len()).filter(|i| crate::borge::solid_at(f,(i%w) as f64,(i/w) as f64)));
    let upstairs = f
        .tiles
        .iter()
        .position(|t| t == "up")
        .ok_or("Generated floor has no entrance staircase")?;
    let approaches: Vec<usize> = cardinal_neighbors(upstairs, w, h)
        .into_iter()
        .filter(|at| !blocked.contains(at))
        .collect();
    if !approaches.iter().any(|&at| terrain_passable(&f.tiles[at])) {
        let mut candidates: Vec<usize> = approaches
            .into_iter()
            .filter(|at| !protected.contains(at))
            .collect();
        candidates.sort_by_key(|&at| (terrain_repair_impact(f, at, &geology), at));
        let at = *candidates
            .first()
            .ok_or("Automatic staircase has no repairable approach")?;
        carve(f, &[at], &mut stats)?;
    }
    let mut reachable = reachable_without_braziers(f, None);
    let mut excluded = IndexSet::new();
    let hardware = sealed_vent_hardware(f);
    for start in w + 1..f.tiles.len() - w - 1 {
        if start % w == 0
            || start % w == w - 1
            || reachable.contains(&start)
            || blocked.contains(&start)
            || excluded.contains(&start)
            || !terrain_passable(&f.tiles[start])
        {
            continue;
        }
        let mut component = vec![start];
        let mut seen = IndexSet::from([start]);
        let mut head = 0;
        while head < component.len() {
            let at = component[head];
            head += 1;
            for next in cardinal_neighbors(at, w, h) {
                if !seen.contains(&next)
                    && !blocked.contains(&next)
                    && terrain_passable(&f.tiles[next])
                {
                    seen.insert(next);
                    component.push(next)
                }
            }
        }
        if component.iter().all(|at| hardware.contains(at)) {
            excluded.extend(component);
            continue;
        }
        let path = minimum_impact_route(
            f, &component, &reachable, &protected, &blocked, &geology, &mut stats,
        )
        .ok_or("Authored geometry leaves an inaccessible room")?;
        carve(f, &path, &mut stats)?;
        reachable = reachable_without_braziers(f, None);
    }
    for transition in automatic_transitions(f) {
        let candidates = cardinal_neighbors(transition, w, h);
        if candidates
            .iter()
            .any(|at| reachable.contains(at) && !blocked.contains(at))
        {
            continue;
        }
        let path = minimum_impact_route(
            f,
            &candidates,
            &reachable,
            &protected,
            &blocked,
            &geology,
            &mut stats,
        )
        .ok_or("Automatic staircase has no repairable approach")?;
        carve(f, &path, &mut stats)?;
        reachable = reachable_without_braziers(f, None);
    }
    if !transitions_accessible(f, &reachable) {
        return Err("Automatic staircase has no accessible approach".into());
    }
    Ok(stats)
}
pub fn brazier_preserves_access(f: &Floor, at: usize) -> bool {
    let before = reachable_without_braziers(f, None);
    if !before.contains(&at) {
        return false;
    }
    let after = reachable_without_braziers(f, Some(at));
    after.len() == before.len() - 1 && transitions_accessible(f, &after)
}
pub fn brazier_preserves_component_access(f: &Floor, at: usize) -> bool {
    let w = f.width();
    let h = f.height();
    let mut blocked = fire_cells(f);
    blocked.extend(f.grates.as_ref().into_iter().flatten().map(|&n| n as usize));
    blocked.extend(automatic_transitions(f));
    let usable = |i: usize| terrain_passable(&f.tiles[i]) && !blocked.contains(&i);
    if !usable(at) {
        return false;
    }
    for stair in automatic_transitions(f) {
        if cardinal_neighbors(stair, w, h).contains(&at)
            && !cardinal_neighbors(stair, w, h)
                .into_iter()
                .any(|i| i != at && usable(i))
        {
            return false;
        }
    }
    let adj: Vec<usize> = cardinal_neighbors(at, w, h)
        .into_iter()
        .filter(|&i| usable(i))
        .collect();
    if adj.is_empty() {
        return false;
    }
    blocked.insert(at);
    let mut seen = IndexSet::from([adj[0]]);
    let mut q = vec![adj[0]];
    let mut head = 0;
    while head < q.len() {
        let cur = q[head];
        head += 1;
        for next in cardinal_neighbors(cur, w, h) {
            if !seen.contains(&next) && terrain_passable(&f.tiles[next]) && !blocked.contains(&next)
            {
                seen.insert(next);
                q.push(next)
            }
        }
    }
    adj.iter().all(|i| seen.contains(i))
}
