use super::*;
use crate::{ConnectorMetadata, Field, Floor, FloorRoomGraph, RoomMetadata};
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologyMetrics {
    pub traversable_cells: usize,
    pub connected_components: usize,
    pub significant_spaces: usize,
    pub bottleneck_regions: usize,
    pub useful_alternatives: usize,
    pub barrier_circuits: usize,
    pub recorded_useful_loops: usize,
    pub ground_useful_alternatives: usize,
    pub ground_connected_components: usize,
    pub ground_cells: usize,
    pub ground_occupied_sectors: usize,
    pub important_passages: usize,
    pub narrow_important_passages: usize,
    pub protected_narrow_passages: usize,
    pub broken_passages: usize,
    pub deliberately_modified_passages: usize,
    pub occupied_sectors: usize,
    pub total_sectors: usize,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologyQualityResult {
    pub before: TopologyMetrics,
    pub after: TopologyMetrics,
    pub changed_cells: usize,
    pub widened_cells: usize,
    pub added_loops: usize,
    pub unmet_alternatives: usize,
    pub unresolved_widths: usize,
}
#[derive(Clone)]
struct Network {
    w: usize,
    h: usize,
    open: Vec<bool>,
    adjacent: Vec<Vec<usize>>,
}
impl Network {
    fn is_open(&self, at: i64) -> bool {
        at >= 0 && self.open.get(at as usize).copied().unwrap_or(false)
    }
}
fn graph(f: &Floor) -> (&[RoomMetadata], &[ConnectorMetadata]) {
    if let Some(g) = f.room_graph.as_ref() {
        (&g.rooms, &g.connectors)
    } else {
        (
            f.dungeon_rooms.as_ref().map(Vec::as_slice).unwrap_or(&[]),
            f.dungeon_connectors
                .as_ref()
                .map(Vec::as_slice)
                .unwrap_or(&[]),
        )
    }
}
fn network(f: &Floor, water: bool) -> Network {
    let w = f.width();
    let h = f.height();
    let mut excluded = fire_cells(f);
    excluded.extend(automatic_transitions(f));
    excluded.extend(
        f.grates
            .as_ref()
            .into_iter()
            .flatten()
            .map(|&at| at as usize),
    );
    excluded.extend(sealed_vent_hardware(f));
    if let Some(e) = f.godbeast_event.as_ref() {
        if e.kind == "Veridian" {
            excluded.insert(e.y as usize * w + e.x as usize);
        }
    }
    let open = f
        .tiles
        .iter()
        .enumerate()
        .map(|(at, t)| !excluded.contains(&at) && terrain_passable(t) && (water || t != "water"))
        .collect();
    let adjacent = (0..f.tiles.len())
        .map(|at| cardinal_neighbors(at, w, h))
        .collect();
    Network {
        w,
        h,
        open,
        adjacent,
    }
}
fn path(c: &ConnectorMetadata) -> &[f64] {
    c.path.as_ref().map(Vec::as_slice).unwrap_or(&c.cells)
}
fn important(c: &ConnectorMetadata) -> bool {
    c.purpose != "corner" && path(c).len() >= 4
}
fn broad(at: usize, n: &Network) -> bool {
    if !n.open[at] {
        return false;
    }
    [
        at as i64,
        at as i64 - 1,
        at as i64 - n.w as i64,
        at as i64 - n.w as i64 - 1,
    ]
    .iter()
    .any(|&a| {
        n.is_open(a)
            && n.is_open(a + 1)
            && n.is_open(a + n.w as i64)
            && n.is_open(a + n.w as i64 + 1)
    })
}
fn passage_cells(f: &Floor, c: &ConnectorMetadata) -> Vec<usize> {
    path(c)
        .iter()
        .map(|&v| v as usize)
        .filter(|at| {
            !f.doors
                .as_ref()
                .is_some_and(|d| d.contains_key(&at.to_string()))
                && !["up", "down", "pit"].contains(&f.tiles[*at].as_str())
        })
        .collect()
}
fn component_count(mask: &[bool], n: &Network, minimum: usize) -> usize {
    let mut seen = vec![false; mask.len()];
    let mut count = 0;
    for start in 0..mask.len() {
        if !mask[start] || seen[start] {
            continue;
        }
        let mut q = vec![start];
        seen[start] = true;
        let mut head = 0;
        while head < q.len() {
            let at = q[head];
            head += 1;
            for &to in &n.adjacent[at] {
                if mask[to] && !seen[to] {
                    seen[to] = true;
                    q.push(to)
                }
            }
        }
        if q.len() >= minimum {
            count += 1
        }
    }
    count
}
/// Iterative Tarjan articulation pass; O(V+E) time/storage, E<=4V.
fn bottlenecks(n: &Network) -> usize {
    let v = n.open.len();
    let mut discovered = vec![0usize; v];
    let mut low = vec![0usize; v];
    let mut parent = vec![usize::MAX; v];
    let mut subtree = vec![0usize; v];
    let mut next_neighbor = vec![0usize; v];
    let mut separated = vec![Vec::<usize>::new(); v];
    let mut marked = vec![false; v];
    let mut time = 0;
    for start in 0..v {
        if !n.open[start] || discovered[start] != 0 {
            continue;
        }
        let mut stack = vec![start];
        let mut members = vec![start];
        time += 1;
        discovered[start] = time;
        low[start] = time;
        subtree[start] = 1;
        while let Some(&at) = stack.last() {
            let adj = &n.adjacent[at];
            if next_neighbor[at] < adj.len() {
                let to = adj[next_neighbor[at]];
                next_neighbor[at] += 1;
                if !n.open[to] || to == parent[at] {
                    continue;
                }
                if discovered[to] != 0 {
                    low[at] = low[at].min(discovered[to]);
                    continue;
                }
                parent[to] = at;
                time += 1;
                discovered[to] = time;
                low[to] = time;
                subtree[to] = 1;
                members.push(to);
                stack.push(to);
            } else {
                stack.pop();
                let p = parent[at];
                if p == usize::MAX {
                    continue;
                }
                subtree[p] += subtree[at];
                low[p] = low[p].min(low[at]);
                if low[at] >= discovered[p] {
                    separated[p].push(subtree[at]);
                }
            }
        }
        for &at in &members {
            let rest = members.len() - 1 - separated[at].iter().sum::<usize>();
            if separated[at]
                .iter()
                .copied()
                .chain([rest])
                .filter(|&size| size >= 8)
                .count()
                >= 2
            {
                marked[at] = true
            }
        }
    }
    component_count(&marked, n, 1)
}
fn useful_loop(c: &ConnectorMetadata, n: &Network) -> bool {
    let path: Vec<usize> = path(c).iter().map(|&v| v as usize).collect();
    if path.is_empty() || path.iter().any(|&at| !n.open[at]) {
        return false;
    }
    let removed: IndexSet<usize> = c.cells.iter().map(|&v| v as usize).collect();
    let pair = if let Some(e) = c.route_endpoints.as_ref() {
        Some((e.0 as i64, e.1 as i64))
    } else if path.len() == 1 {
        let at = path[0] as i64;
        [(at - 1, at + 1), (at - n.w as i64, at + n.w as i64)]
            .into_iter()
            .find(|&(a, b)| {
                n.is_open(a)
                    && n.is_open(b)
                    && !removed.contains(&(a as usize))
                    && !removed.contains(&(b as usize))
            })
    } else {
        Some((
            2 * path[0] as i64 - path[1] as i64,
            2 * path[path.len() - 1] as i64 - path[path.len() - 2] as i64,
        ))
    };
    let Some((from, to)) = pair else { return false };
    if !n.is_open(from)
        || !n.is_open(to)
        || removed.contains(&(from as usize))
        || removed.contains(&(to as usize))
    {
        return false;
    }
    let (from, to) = (from as usize, to as usize);
    let mut distance = vec![-1i32; n.open.len()];
    let mut q = vec![from];
    distance[from] = 0;
    let mut head = 0;
    while head < q.len() {
        let at = q[head];
        head += 1;
        if at == to {
            return distance[at] >= (path.len() + 7) as i32;
        }
        for &next in &n.adjacent[at] {
            if n.open[next] && !removed.contains(&next) && distance[next] < 0 {
                distance[next] = distance[at] + 1;
                q.push(next)
            }
        }
    }
    false
}
fn shortest_path(
    n: &Network,
    from: usize,
    to: usize,
    removed: &IndexSet<usize>,
) -> Option<Vec<usize>> {
    let mut prev = vec![usize::MAX; n.open.len()];
    let mut q = vec![from];
    prev[from] = from;
    let mut head = 0;
    while head < q.len() {
        let at = q[head];
        head += 1;
        if at == to {
            let mut path = vec![to];
            while *path.last().unwrap() != from {
                path.push(prev[*path.last().unwrap()])
            }
            path.reverse();
            return Some(path);
        }
        for &next in &n.adjacent[at] {
            if n.open[next] && !removed.contains(&next) && prev[next] == usize::MAX {
                prev[next] = at;
                q.push(next)
            }
        }
    }
    None
}
#[derive(Clone, Copy)]
struct Alternatives {
    circuits: usize,
    useful: usize,
    components: usize,
}
/// Planar dual obstacle components plus at most six bank probes each: O(V²), O(V) scratch.
fn alternatives(n: &Network) -> Alternatives {
    let v = n.open.len();
    let mut components = vec![-1i32; v];
    let mut seen = vec![false; v];
    let (mut next_id, mut circuits, mut useful) = (0, 0, 0);
    for start in 0..v {
        if !n.open[start] || components[start] >= 0 {
            continue;
        }
        let mut q = vec![start];
        components[start] = next_id;
        let mut head = 0;
        while head < q.len() {
            let at = q[head];
            head += 1;
            for &to in &n.adjacent[at] {
                if n.open[to] && components[to] < 0 {
                    components[to] = next_id;
                    q.push(to)
                }
            }
        }
        next_id += 1;
    }
    for start in 0..v {
        if n.open[start] || seen[start] {
            continue;
        }
        let mut q = vec![start];
        let mut surrounding = IndexSet::new();
        let (mut boundary, mut left, mut right, mut top, mut bottom) = (false, n.w, 0, n.h, 0);
        seen[start] = true;
        let mut head = 0;
        while head < q.len() {
            let at = q[head];
            head += 1;
            let x = at % n.w;
            let y = at / n.w;
            left = left.min(x);
            right = right.max(x);
            top = top.min(y);
            bottom = bottom.max(y);
            if x == 0 || x == n.w - 1 || y == 0 || y == n.h - 1 {
                boundary = true
            }
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let xx = x as i32 + dx;
                    let yy = y as i32 + dy;
                    if (dx == 0 && dy == 0)
                        || xx < 0
                        || xx >= n.w as i32
                        || yy < 0
                        || yy >= n.h as i32
                    {
                        continue;
                    }
                    let to = yy as usize * n.w + xx as usize;
                    if n.open[to] {
                        surrounding.insert(components[to]);
                        continue;
                    }
                    if !seen[to] {
                        seen[to] = true;
                        q.push(to)
                    }
                }
            }
        }
        if boundary || q.len() < 4 || right - left + bottom - top + 2 < 6 || surrounding.len() != 1
        {
            continue;
        }
        circuits += 1;
        let cx = (left + right) as f64 / 2.0;
        let cy = (top + bottom) as f64 / 2.0;
        let score = |at: usize| ((at % n.w) as f64 - cx).abs() + ((at / n.w) as f64 - cy).abs();
        let mut probes = vec![];
        for at in q {
            let mut pos = 0;
            while pos < probes.len()
                && (score(probes[pos]) < score(at)
                    || (score(probes[pos]) == score(at) && probes[pos] < at))
            {
                pos += 1
            }
            if pos < 3 {
                probes.insert(pos, at);
                if probes.len() > 3 {
                    probes.pop();
                }
            }
        }
        let mut verified = false;
        for at in probes {
            for step in [1, n.w] {
                if verified {
                    break;
                }
                let bank = |direction: i64| -> Option<usize> {
                    let mut next = at as i64;
                    for _ in 0..if step == 1 { n.w } else { n.h } {
                        if next < 0 {
                            return None;
                        }
                        let x = next % n.w as i64;
                        let y = next / n.w as i64;
                        if x <= 0 || x >= n.w as i64 - 1 || y <= 0 || y >= n.h as i64 - 1 {
                            return None;
                        }
                        if n.open[next as usize] {
                            return Some(next as usize);
                        }
                        next += direction * step as i64;
                    }
                    None
                };
                let (Some(a), Some(b)) = (bank(-1), bank(1)) else {
                    continue;
                };
                let direct = (a % n.w).abs_diff(b % n.w) + (a / n.w).abs_diff(b / n.w);
                let Some(route) = shortest_path(n, a, b, &IndexSet::new()) else {
                    continue;
                };
                if route.len() - 1 < direct + 6 {
                    continue;
                }
                let removed = route[1..route.len() - 1].iter().copied().collect();
                if shortest_path(n, a, b, &removed).is_some_and(|p| p.len() - 1 >= direct + 6) {
                    verified = true
                }
            }
        }
        if verified {
            useful += 1
        }
    }
    Alternatives {
        circuits,
        useful,
        components: next_id as usize,
    }
}
fn protected_topology(f: &Floor) -> IndexSet<usize> {
    protected_generation_cells(f)
        .into_iter()
        .chain(
            geological_cells(f)
                .into_iter()
                .filter(|&at| ["wall", "tree", "chasm", "water"].contains(&f.tiles[at].as_str())),
        )
        .collect()
}
/// Final live-grid measurements, O((L+1)V + V²), O(V) scratch. No graph-label shortcuts.
pub fn measure_generated_topology(f: &Floor) -> TopologyMetrics {
    let n = network(f, true);
    let mut ground = n.clone();
    let mut water = false;
    for at in 0..ground.open.len() {
        if ground.open[at] && f.tiles[at] == "water" {
            ground.open[at] = false;
            water = true
        }
    }
    let (_, connectors) = graph(f);
    let protected = protected_topology(f);
    let mut cores = vec![false; n.open.len()];
    let mut sectors = [0usize; 12];
    let mut ground_sectors = [0usize; 12];
    let (
        mut traversable,
        mut important_passages,
        mut narrow_important,
        mut protected_narrow,
        mut broken,
        mut modified,
    ) = (0, 0, 0, 0, 0, 0);
    for at in 0..n.open.len() {
        if !n.open[at] {
            continue;
        }
        traversable += 1;
        let x = at % n.w;
        let y = at / n.w;
        let sector = 2.min(y * 3 / n.h) * 4 + 3.min(x * 4 / n.w);
        sectors[sector] += 1;
        if ground.open[at] {
            ground_sectors[sector] += 1
        }
        if x > 1
            && x < n.w - 2
            && y > 1
            && y < n.h - 2
            && [
                -(n.w as i64) - 1,
                -(n.w as i64),
                -(n.w as i64) + 1,
                -1,
                0,
                1,
                n.w as i64 - 1,
                n.w as i64,
                n.w as i64 + 1,
            ]
            .iter()
            .all(|&d| n.is_open(at as i64 + d))
        {
            cores[at] = true
        }
    }
    for c in connectors.iter().filter(|c| important(c)) {
        important_passages += 1;
        let cells = passage_cells(f, c);
        let narrow: Vec<usize> = cells
            .iter()
            .copied()
            .filter(|&at| !broad(at, &ground))
            .collect();
        if cells.iter().any(|&at| !n.open[at]) {
            if cells
                .iter()
                .filter(|&&at| !n.open[at])
                .all(|at| protected.contains(at))
            {
                modified += 1
            } else {
                broken += 1
            }
        }
        if !narrow.is_empty() {
            if narrow.iter().all(|&at| {
                protected.contains(&at)
                    || [
                        at as i64,
                        at as i64 - 1,
                        at as i64 - n.w as i64,
                        at as i64 - n.w as i64 - 1,
                    ]
                    .iter()
                    .all(|&a| {
                        [a, a + 1, a + n.w as i64, a + n.w as i64 + 1]
                            .iter()
                            .any(|&cell| {
                                !ground.is_open(cell)
                                    && cell >= 0
                                    && protected.contains(&(cell as usize))
                            })
                    })
            }) {
                protected_narrow += 1
            } else {
                narrow_important += 1
            }
        }
    }
    let alt = alternatives(&n);
    let ga = if water { alternatives(&ground) } else { alt };
    TopologyMetrics {
        traversable_cells: traversable,
        connected_components: alt.components,
        significant_spaces: component_count(&cores, &n, 12),
        bottleneck_regions: bottlenecks(&n),
        useful_alternatives: alt.useful,
        barrier_circuits: alt.circuits,
        recorded_useful_loops: connectors
            .iter()
            .filter(|c| c.purpose == "loop" && useful_loop(c, &n))
            .count(),
        ground_useful_alternatives: ga.useful,
        ground_connected_components: ga.components,
        ground_cells: ground.open.iter().filter(|&&v| v).count(),
        ground_occupied_sectors: ground_sectors.iter().filter(|&&v| v >= 8).count(),
        important_passages,
        narrow_important_passages: narrow_important,
        protected_narrow_passages: protected_narrow,
        broken_passages: broken,
        deliberately_modified_passages: modified,
        occupied_sectors: sectors.iter().filter(|&&v| v >= 8).count(),
        total_sectors: 12,
    }
}
fn widening_patch(
    f: &Floor,
    seeds: &[usize],
    protected: &IndexSet<usize>,
    remaining: usize,
) -> Option<Vec<usize>> {
    let w = f.width();
    let h = f.height();
    let mut patch = IndexSet::new();
    let allowed = |at: usize| {
        at % w > 0
            && at % w < w - 1
            && at >= w
            && at < (h - 1) * w
            && ["wall", "tree"].contains(&f.tiles[at].as_str())
            && !protected.contains(&at)
    };
    if seeds.is_empty() || seeds.len() > remaining || seeds.iter().any(|&at| !allowed(at)) {
        return None;
    }
    patch.extend(seeds.iter().copied());
    for _ in 0..6 {
        let mut changed = false;
        for at in patch.clone() {
            for a in [
                at as i64,
                at as i64 - 1,
                at as i64 - w as i64,
                at as i64 - w as i64 - 1,
            ] {
                let b = a + 1;
                let c = a + w as i64;
                let d = c + 1;
                let open = |n: i64| {
                    n >= 0
                        && (patch.contains(&(n as usize))
                            || f.tiles.get(n as usize).is_some_and(|t| terrain_passable(t)))
                };
                let pair = if open(a) && open(d) && !open(b) && !open(c) {
                    Some([b, c])
                } else if open(b) && open(c) && !open(a) && !open(d) {
                    Some([a, d])
                } else {
                    None
                };
                let Some(pair) = pair else { continue };
                let bridge = pair.into_iter().find(|&at| at >= 0 && allowed(at as usize));
                let Some(bridge) = bridge else { return None };
                if patch.len() >= remaining.min(6) {
                    return None;
                }
                patch.insert(bridge as usize);
                changed = true;
            }
        }
        if !changed {
            return Some(patch.into_iter().collect());
        }
    }
    None
}
fn sync_graph(f: &mut Floor, g: &FloorRoomGraph) {
    if f.room_graph.as_ref().is_some() {
        f.room_graph = Field::Value(g.clone())
    } else {
        f.dungeon_rooms = Field::Value(g.rooms.clone());
        f.dungeon_connectors = Field::Value(g.connectors.clone())
    }
}
fn widen(
    f: &mut Floor,
    g: &mut FloorRoomGraph,
    protected: &IndexSet<usize>,
    budget: usize,
    widened: &mut usize,
) {
    let mut n = network(f, false);
    let mut passages: Vec<usize> = g
        .connectors
        .iter()
        .enumerate()
        .filter_map(|(i, c)| if important(c) { Some(i) } else { None })
        .collect();
    passages.sort_by(|&a, &b| {
        let a = &g.connectors[a];
        let b = &g.connectors[b];
        (b.purpose == "loop")
            .cmp(&(a.purpose == "loop"))
            .then(path(b).len().cmp(&path(a).len()))
            .then(a.id.total_cmp(&b.id))
    });
    for id in passages {
        let saved_path = path(&g.connectors[id]).to_vec();
        for &value in &saved_path {
            if *widened >= budget {
                break;
            }
            let at = value as usize;
            if f.doors
                .as_ref()
                .is_some_and(|d| d.contains_key(&at.to_string()))
                || ["up", "down", "pit"].contains(&f.tiles[at].as_str())
                || protected.contains(&at)
                || broad(at, &n)
            {
                continue;
            }
            let mut patches = vec![];
            for a in [
                at as i64,
                at as i64 - 1,
                at as i64 - n.w as i64,
                at as i64 - n.w as i64 - 1,
            ] {
                let missing: Vec<usize> = [a, a + 1, a + n.w as i64, a + n.w as i64 + 1]
                    .into_iter()
                    .filter(|&cell| !n.is_open(cell))
                    .map(|cell| cell as usize)
                    .collect();
                if missing.iter().any(|&cell| cell >= f.tiles.len()) {
                    continue;
                }
                if let Some(p) = widening_patch(f, &missing, protected, budget - *widened) {
                    patches.push(p)
                }
            }
            patches.sort_by(|a, b| a.len().cmp(&b.len()).then(a[0].cmp(&b[0])));
            let Some(patch) = patches.into_iter().next() else {
                continue;
            };
            for &cell in &patch {
                f.tiles[cell] = "floor".into();
                if !g.connectors[id].cells.contains(&(cell as f64)) {
                    g.connectors[id].cells.push(cell as f64)
                }
            }
            if g.connectors[id].path.as_ref().is_none() {
                g.connectors[id].path = Field::Value(saved_path.clone())
            }
            *widened += patch.len();
            for cell in patch {
                n.open[cell] = true
            }
        }
    }
    sync_graph(f, g)
}
fn as_layout(f: &Floor, g: &FloorRoomGraph, tiles: Vec<String>) -> Result<DungeonLayout, String> {
    let rooms = g
        .rooms
        .iter()
        .map(|r| -> Result<LayoutRoom, String> {
            Ok(LayoutRoom {
                id: r.id as usize,
                kind: serde_json::from_value(serde_json::Value::String(r.kind.clone())).map_err(
                    |_| format!("Unsupported legacy room kind for topology: {}", r.kind),
                )?,
                cells: r.cells.iter().map(|&v| v as usize).collect(),
                size_class: serde_json::from_value(serde_json::Value::String(
                    r.size_class.as_ref().cloned().unwrap_or("medium".into()),
                ))
                .map_err(|e| e.to_string())?,
                center: Point {
                    x: r.center.x as i32,
                    y: r.center.y as i32,
                },
                bounds: Bounds {
                    x: r.bounds.x as i32,
                    y: r.bounds.y as i32,
                    width: r.bounds.width as usize,
                    height: r.bounds.height as usize,
                },
                parent_room_id: r.parent_room_id.map(|v| v as usize),
                connector_id: r.connector_id.map(|v| v as usize),
                doors: r.doors.iter().map(|&v| v as usize).collect(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let connectors = g
        .connectors
        .iter()
        .map(|c| -> Result<LayoutConnector, String> {
            Ok(LayoutConnector {
                id: c.id as usize,
                kind: c.kind.clone(),
                cells: c.cells.iter().map(|&v| v as usize).collect(),
                room_ids: c.room_ids.iter().map(|&v| v as usize).collect(),
                doors: c.doors.iter().map(|&v| v as usize).collect(),
                parent_room_id: c.parent_room_id as usize,
                purpose: c.purpose.clone(),
                style: c
                    .style
                    .as_ref()
                    .map(|s| serde_json::from_value(serde_json::Value::String(s.clone())))
                    .transpose()
                    .map_err(|e| e.to_string())?,
                path: c
                    .path
                    .as_ref()
                    .map(|p| p.iter().map(|&v| v as usize).collect()),
                parent_connector_id: c.parent_connector_id.as_ref().map(|&v| v as usize),
                route_before: c.route_before.as_ref().map(|&v| v as usize),
                route_after: c.route_after.as_ref().map(|&v| v as usize),
                route_endpoints: c
                    .route_endpoints
                    .as_ref()
                    .map(|&(a, b)| [a as usize, b as usize]),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let doors = f
        .doors
        .as_ref()
        .into_iter()
        .flat_map(|d| {
            d.iter().filter_map(|(key, value)| {
                key.parse::<usize>()
                    .ok()
                    .map(|at| (at, LayoutDoor { open: value.open }))
            })
        })
        .collect();
    Ok(DungeonLayout {
        tiles,
        doors,
        rooms,
        connectors,
    })
}
/// Bounded 96-cell widening and at most two 12-cell loops. Graph conversion O(V+metadata),
/// loop acceptance invokes O(V²) alternatives; <=48 checks, O(V²) overall bounded factor.
pub fn finalize_generated_topology(f: &mut Floor) -> Result<TopologyQualityResult, String> {
    let before = measure_generated_topology(f);
    let (rooms, connectors) = graph(f);
    let mut g = FloorRoomGraph {
        rooms: rooms.to_vec(),
        connectors: connectors.to_vec(),
    };
    let original = f.tiles.clone();
    let protected = protected_topology(f);
    let budget = 96.min(f.tiles.len() / 24);
    let mut widened = 0;
    let mut added = 0;
    widen(f, &mut g, &protected, budget, &mut widened);
    let live = if widened > 0 {
        measure_generated_topology(f)
    } else {
        before.clone()
    };
    let widened_before = widened;
    let desired = 2.min(g.rooms.len().saturating_sub(2));
    if live.ground_useful_alternatives < desired && !g.rooms.is_empty() {
        let n = network(f, false);
        let mut blocked = protected.clone();
        for at in 0..f.tiles.len() {
            if !n.open[at] && !["wall", "tree"].contains(&f.tiles[at].as_str()) {
                blocked.insert(at);
            }
        }
        let mut view = as_layout(
            f,
            &g,
            n.open
                .iter()
                .map(|&open| if open { "floor" } else { "wall" }.into())
                .collect(),
        )?;
        let mut verified = live.ground_useful_alternatives;
        let mut accept = |candidate: &DungeonLayout, _: &LayoutConnector| {
            let mut floor = f.clone();
            floor.tiles = candidate.tiles.clone();
            let next = alternatives(&network(&floor, false)).useful;
            if next <= verified {
                return false;
            }
            verified = next;
            true
        };
        added = add_room_loops(
            &mut view,
            &mut || 0.5,
            &LoopOptions {
                constraints: LayoutConstraints {
                    reserved_cells: blocked.into_iter().collect(),
                    ..Default::default()
                },
                allow_bent: true,
                max_loops: Some(2.min(desired - live.ground_useful_alternatives)),
                maximum_candidate_checks: Some(48),
            },
            Some(&mut accept),
        )?;
        for at in 0..f.tiles.len() {
            if view.tiles[at] == "floor" && !n.open[at] {
                f.tiles[at] = "floor".into()
            }
        }
        for connector in view.connectors.iter().skip(g.connectors.len()) {
            g.connectors.push(
                serde_json::from_value(serde_json::to_value(connector).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?,
            )
        }
        sync_graph(f, &g);
        if added > 0 {
            widen(f, &mut g, &protected, budget, &mut widened)
        }
    }
    let after = if added > 0 || widened != widened_before {
        measure_generated_topology(f)
    } else {
        live
    };
    Ok(TopologyQualityResult {
        changed_cells: original
            .iter()
            .zip(&f.tiles)
            .filter(|(a, b)| a != b)
            .count(),
        unmet_alternatives: desired.saturating_sub(after.ground_useful_alternatives),
        unresolved_widths: after.narrow_important_passages,
        before,
        after,
        widened_cells: widened,
        added_loops: added,
    })
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TopologyAcceptance {
    pub acceptable: bool,
    pub ground_fraction: f64,
    pub ground_ratio_deficit: f64,
    pub underused_sectors: usize,
    pub missing_alternatives: usize,
    pub width_failures: usize,
    pub broken_passages: usize,
}
pub fn evaluate_generated_topology(f: &Floor, m: &TopologyMetrics) -> TopologyAcceptance {
    let (min, max) = match f.domain.as_str() {
        "Cave" => (0.35, 0.68),
        "Meadow" => (0.42, 0.75),
        "Forest" => (0.40, 0.74),
        "Archipelago" => (0.28, 0.67),
        "Mountain" => (0.33, 0.66),
        _ => (0.30, 0.66),
    };
    let fraction = m.ground_cells as f64 / ((f.width() - 2) * (f.height() - 2)) as f64;
    let deficit = (min - fraction).max(fraction - max).max(0.0);
    let underused = 9usize.saturating_sub(m.ground_occupied_sectors);
    let missing = 2usize.saturating_sub(m.ground_useful_alternatives);
    TopologyAcceptance {
        acceptable: deficit == 0.0
            && underused == 0
            && missing == 0
            && m.narrow_important_passages == 0
            && m.broken_passages == 0
            && m.connected_components == 1,
        ground_fraction: fraction,
        ground_ratio_deficit: deficit,
        underused_sectors: underused,
        missing_alternatives: missing,
        width_failures: m.narrow_important_passages,
        broken_passages: m.broken_passages,
    }
}
