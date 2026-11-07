//! Revision 36 contiguous cellular-automaton pockets and their literal assignments.
//! Generation uses its own seed/floor stream, never the simulation RNG. Failed
//! proposals are discarded whole: no self-links, rewiring or terrain repair.
use crate::{Actor, Field, Floor, GroundItem, Run, rng::GenerationRng};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};

/// Conservative initial defaults, not designer-authored density scaling.
pub const MAX_POCKETS: usize = 8;
pub const MAX_SPEED_UP_AREAS: usize = 8;
pub const MIN_GENERATED_CELLS: usize = 6;
pub const MAX_CELLS: usize = 48;
pub const SHAPE_ATTEMPTS: usize = 48;
pub const ASSIGNMENT_ATTEMPTS: usize = 64;
const CA_SIDE: usize = 11;
const CA_STEPS: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NonEuclideanGroup {
    /// One or two source squares. Order retains the forced next-source trace.
    pub sources: Vec<usize>,
    pub destination: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct NonEuclideanPocket {
    /// Sorted, unique, cardinally contiguous CA component.
    pub cells: Vec<usize>,
    /// Sequential assignment order is serialized, not reconstructed on load.
    pub groups: Vec<NonEuclideanGroup>,
}

pub fn requested_pockets(depth: u32) -> usize { usize::from(depth >= 20) }
pub fn pocket_at(f: &Floor, at: usize) -> Option<&NonEuclideanPocket> {
    f.non_euclidean.as_ref()?.iter().find(|p| p.cells.binary_search(&at).is_ok())
}
pub fn speedup_at(f: &Floor, at: usize) -> Option<&NonEuclideanPocket> {
    f.speed_up.as_ref()?.iter().find(|p| p.cells.binary_search(&at).is_ok())
}
pub fn has_links(f: &Floor) -> bool {
    f.non_euclidean.as_ref().is_some_and(|p| !p.is_empty()) || f.speed_up.as_ref().is_some_and(|p| !p.is_empty())
}
/// Saved assignment, including temporarily replaced terrain. Presentation must
/// separately gate this on observed terrain, never hidden live replacements.
pub fn assigned_group_at(f: &Floor, at: usize) -> Option<&NonEuclideanGroup> {
    pocket_at(f, at).or_else(||speedup_at(f,at))?.groups.iter().find(|g| g.destination == at || g.sources.contains(&at))
}
/// O(S), S<=2. Replacing either endpoint removes this tile link until exact
/// terrain restoration. Metadata is retained for stable identity, not physics.
pub fn speedup_group_active(f: &Floor, group: &NonEuclideanGroup) -> bool {
    !f.speed_up_terrain.value_or(false) || group.sources.iter().copied()
        .chain(std::iter::once(group.destination))
        .all(|at| f.tiles.get(at).is_some_and(|t| t == "speed-up"))
}
pub fn group_at(f: &Floor, at: usize) -> Option<&NonEuclideanGroup> {
    let group = assigned_group_at(f, at)?;
    (pocket_at(f, at).is_some() || speedup_group_active(f, group)).then_some(group)
}
/// Stable floor-wide display IDs. Count every saved group, including hidden or
/// replaced ones, so observing/replacing a tile never renumbers another group.
/// O(G*S), G<=384 and S<=2; no RNG and no world mutation.
pub fn group_number(f: &Floor, at: usize) -> Option<usize> {
    f.non_euclidean.as_ref().into_iter().flatten().chain(f.speed_up.as_ref().into_iter().flatten())
        .flat_map(|p| &p.groups).enumerate()
        .find(|(_, g)| g.destination == at || g.sources.contains(&at)).map(|(i, _)| i + 1)
}
/// One-time r51 generation conversion, O(K) writes for K<=384 speed-up cells.
/// Ordinary terrain is replaced, not retained beneath a modifier. Existing
/// hazard layers remain independent, as on every other ground tile.
pub fn install_speedup_terrain(f: &mut Floor) {
    for pocket in f.speed_up.as_ref().into_iter().flatten() {
        for &at in &pocket.cells {
            f.tiles[at] = "speed-up".into();
            if let Some(currents) = f.currents.as_mut() { currents[at] = 0.; }
        }
    }
    f.speed_up_terrain = true.into();
}
/// Possible arrivals only. This query is deliberately deterministic and read-only.
pub fn exits(f: &Floor, at: usize) -> Vec<usize> {
    group_at(f, at).map_or_else(Vec::new, |g| {
        if at == g.destination { g.sources.clone() } else { vec![g.destination] }
    })
}
pub fn destinations(f: &Floor, at: usize) -> Vec<usize> { exits(f, at) }
fn neighbors(at: usize, w: usize, n: usize) -> [Option<usize>; 4] {
    [if at % w > 0 { Some(at - 1) } else { None },
     if at % w + 1 < w { Some(at + 1) } else { None },
     if at >= w { Some(at - w) } else { None },
     if at + w < n { Some(at + w) } else { None }]
}
/// Assignment adjacency is the live eight-way grid, including its existing
/// no-wall-corner diagonal rule. CA contiguity and safety remain cardinal.
fn assignment_neighbors(f: &Floor, at: usize) -> Vec<usize> {
    let w = f.width();
    if w == 0 || at >= f.tiles.len() { return vec![]; }
    let from = crate::movement::Point { x: (at % w) as i32, y: (at / w) as i32 };
    crate::movement::DIRECTIONS.iter().filter_map(|direction| {
        let to = crate::movement::Point { x: from.x + direction.x, y: from.y + direction.y };
        if to.x < 0 || to.y < 0 || to.x >= w as i32 || to.y >= f.height() as i32
            || !crate::movement::diagonal_step_allowed(&f.tiles, w, from, to, false) { return None; }
        Some(to.y as usize * w + to.x as usize)
    }).collect()
}
fn adjacent(f: &Floor, a: usize, b: usize) -> bool { assignment_neighbors(f, a).contains(&b) }
fn eligible_destinations(f: &Floor, remaining: &BTreeSet<usize>) -> Vec<usize> {
    remaining.iter().copied().filter(|&at|
        assignment_neighbors(f, at).iter().any(|other| remaining.contains(other))).collect()
}
fn pick<T: Copy>(values: &[T], random: &mut impl FnMut() -> f64) -> T {
    values[(random() * values.len() as f64).floor() as usize]
}

/// One literal proposal. O(K^2 log K) time, O(K) scratch for K<=48.
/// Removing the sources BEFORE destination eligibility is important. A terminal
/// singleton or stranded next-source chain rejects this whole proposal.
fn assignment_with_random(f: &Floor, cells: &[usize], random: &mut impl FnMut() -> f64)
    -> Option<Vec<NonEuclideanGroup>>
{
    let (w, n) = (f.width(), f.tiles.len());
    if w == 0 || cells.len() < 2 { return None; }
    let mut remaining: BTreeSet<_> = cells.iter().copied().collect();
    if remaining.len() != cells.len() || remaining.iter().any(|&at| at >= n) { return None; }
    let mut next_source = None;
    let mut groups = Vec::new();
    while !remaining.is_empty() {
        if remaining.len() < 2 { return None; }
        let two = remaining.len() > 2 && random() < 0.30;
        let first = next_source.take().unwrap_or_else(|| pick(&remaining.iter().copied().collect::<Vec<_>>(), random));
        if !remaining.remove(&first) { return None; }
        let mut sources = vec![first];
        if two {
            let second = pick(&remaining.iter().copied().collect::<Vec<_>>(), random);
            remaining.remove(&second);
            // The explicit fallback restores only the proposed second source;
            // it does not alter any completed assignment.
            if eligible_destinations(f, &remaining).is_empty() {
                remaining.insert(second);
            } else {
                sources.push(second);
            }
        }
        let candidates = eligible_destinations(f, &remaining);
        let destination = if candidates.is_empty() {
            if remaining.len() != 1 { return None; }
            *remaining.first().unwrap()
        } else { pick(&candidates, random) };
        remaining.remove(&destination);
        groups.push(NonEuclideanGroup { sources, destination });
        if remaining.is_empty() { break; }
        let adjacent: Vec<_> = assignment_neighbors(f, destination).into_iter()
            .filter(|at| remaining.contains(at)).collect();
        if adjacent.is_empty() { return None; }
        next_source = Some(pick(&adjacent, random));
    }
    Some(groups)
}
pub fn assignment_once(f: &Floor, cells: &[usize], rng: &mut GenerationRng)
    -> Option<Vec<NonEuclideanGroup>>
{
    assignment_with_random(f, cells, &mut || rng.random())
}
fn contiguous(cells: &[usize], w: usize, n: usize) -> bool {
    let Some(&start) = cells.first() else { return false; };
    let members: HashSet<_> = cells.iter().copied().collect();
    let mut seen = HashSet::from([start]);
    let mut queue = vec![start];
    let mut head = 0;
    while head < queue.len() {
        let at = queue[head]; head += 1;
        for next in neighbors(at, w, n).into_iter().flatten() {
            if members.contains(&next) && seen.insert(next) { queue.push(next); }
        }
    }
    seen.len() == cells.len()
}
/// Structural/save validation. Terrain and generation-history heuristics are
/// intentionally not checked: later gameplay may alter a valid pocket and
/// controlled fixtures can describe structurally valid authored links.
pub fn validate_floor(f: &Floor) -> bool {
    if (!f.non_euclidean.is_missing() && f.non_euclidean.as_ref().is_none()) || (!f.speed_up.is_missing() && f.speed_up.as_ref().is_none()) { return false; }
    let random = f.non_euclidean.as_ref().map(Vec::as_slice).unwrap_or(&[]);
    let ordered = f.speed_up.as_ref().map(Vec::as_slice).unwrap_or(&[]);
    if random.len() > MAX_POCKETS || ordered.len() > MAX_SPEED_UP_AREAS || ((!random.is_empty() || !ordered.is_empty()) && !(20.0..=40.0).contains(&f.depth)) { return false; }
    let pockets:Vec<_> = random.iter().chain(ordered).collect();
    let (w, n) = (f.width(), f.tiles.len());
    if w == 0 { return pockets.is_empty(); }
    let mut occupied: HashSet<_> = f.portals.as_ref().into_iter().flatten().flat_map(|p| [p.a, p.b]).collect();
    for pocket in pockets {
        if !(2..=MAX_CELLS).contains(&pocket.cells.len())
            || pocket.cells.windows(2).any(|p| p[0] >= p[1])
            || pocket.cells.iter().any(|&at| at >= n || !occupied.insert(at))
            || !contiguous(&pocket.cells, w, n) { return false; }
        let mut remaining: BTreeSet<_> = pocket.cells.iter().copied().collect();
        for group in &pocket.groups {
            if !(1..=2).contains(&group.sources.len()) { return false; }
            for source in &group.sources { if !remaining.remove(source) { return false; } }
            if !remaining.remove(&group.destination) { return false; }
        }
        if !remaining.is_empty() { return false; }
    }
    true
}
/// Checks the generator's ordered trace independently from save admission.
/// Controlled/edited metadata need only satisfy structural `validate_floor`.
pub fn assignment_follows_rules(f: &Floor, pocket: &NonEuclideanPocket) -> bool {
    if f.width() == 0 { return false; }
    let mut remaining: BTreeSet<_> = pocket.cells.iter().copied().collect();
    let mut previous_destination = None;
    for group in &pocket.groups {
        if !(1..=2).contains(&group.sources.len()) || remaining.len() < 2
            || (remaining.len() == 2 && group.sources.len() != 1)
            || previous_destination.is_some_and(|prev| !adjacent(f, prev, group.sources[0])) { return false; }
        for source in &group.sources { if !remaining.remove(source) { return false; } }
        let candidates = eligible_destinations(f, &remaining);
        if group.sources.len() == 2 && candidates.is_empty() { return false; }
        if !(candidates.contains(&group.destination)
            || candidates.is_empty() && remaining.len() == 1 && remaining.contains(&group.destination)) { return false; }
        remaining.remove(&group.destination);
        previous_destination = Some(group.destination);
    }
    remaining.is_empty()
}
pub fn validate_run(r: &Run) -> bool {
    let revision = r.simulation_revision.value_or(29.0);
    r.floors.iter().chain(r.dreamwalking.as_ref().into_iter().flat_map(|s| s.floors.iter()))
        .all(|f| validate_floor(f) && (revision >= 36.0 || f.non_euclidean.is_missing())
            && (revision >= 37.0 || (f.speed_up.is_missing() && f.non_euclidean.as_ref().is_none_or(|v|v.len()<=1)))
            && (f.speed_up_terrain.is_missing() || revision >= 51.0 && f.speed_up_terrain.as_ref() == Some(&true))
            && f.tiles.iter().enumerate().all(|(at, t)| t != "speed-up"
                || revision >= 51.0 && f.speed_up_terrain.value_or(false) && speedup_at(f, at).is_some())
            && f.remembered_tiles.as_ref().is_none_or(|v| v.iter().all(|t|t != "speed-up" || revision >= 51.0 && f.speed_up_terrain.value_or(false)))
            && f.altered_chasms.as_ref().is_none_or(|v|v.values().all(|m|m.tile != "speed-up" || revision >= 51.0 && f.speed_up_terrain.value_or(false)))
            && f.revision50_terrain.as_ref().is_none_or(|v|v.iter().all(|m|m.previous_tiles.iter().all(|t|t != "speed-up" || revision >= 51.0 && f.speed_up_terrain.value_or(false))))
            && f.revision50_temporary_terrain.as_ref().is_none_or(|v|v.iter().all(|m|m.previous != "speed-up" || revision >= 51.0 && f.speed_up_terrain.value_or(false))))
}
fn walking(f: &Floor) -> Vec<bool> {
    let w = f.width();
    let mut open: Vec<_> = f.tiles.iter().enumerate().map(|(at, t)|
        !["wall", "tree", "chasm", "pit"].contains(&t.as_str())
        && !crate::borge::solid_at(f, (at % w) as f64, (at / w) as f64)
        && !crate::epidite::solid_at(f, (at % w) as f64, (at / w) as f64)).collect();
    for &at in f.grates.as_ref().into_iter().flatten().chain(f.permanent_fire.as_ref().into_iter().flatten()) {
        if let Some(v) = open.get_mut(at as usize) { *v = false; }
    }
    open
}
fn reachable(start: usize, edges: &[Vec<usize>]) -> Vec<bool> {
    let mut seen = vec![false; edges.len()];
    let mut queue = vec![start]; seen[start] = true;
    let mut head = 0;
    while head < queue.len() {
        let at = queue[head]; head += 1;
        for &next in &edges[at] { if !seen[next] { seen[next] = true; queue.push(next); } }
    }
    seen
}
/// Proves actual directed movement, not just physical CA contiguity. Every
/// originally reachable walking cell must remain reachable FROM and TO upstairs.
/// Both stochastic exits are edges, and each resulting state is checked. Thus
/// either outcome has a route out; random return choices cannot create a closed
/// trap. Repeat with every portal endpoint forbidden to preserve optional
/// portal traversal. Four-neighbor proof is conservative for the eight-way live walker.
/// O(V+E+K log K+P) time, O(V+E+K+P) scratch, E<=8V, K<=48, P<=7;
/// once per complete proposal, including structural admission.
pub fn traversable(f: &Floor) -> bool {
    let (w, n) = (f.width(), f.tiles.len());
    if w == 0 || n == 0 || !validate_floor(f) || !crate::portals_generation::validate_floor(f) { return false; }
    traversable_graph(f, false) && traversable_graph(f, true)
}
/// The second proof forbids every portal endpoint together, preserving r35's
/// optional-portal guarantee after the r36 movement graph changes.
fn traversable_graph(f: &Floor, avoid_portals: bool) -> bool {
    let (w, n) = (f.width(), f.tiles.len());
    let mut open = walking(f);
    if avoid_portals {
        for p in f.portals.as_ref().into_iter().flatten() { open[p.a] = false; open[p.b] = false; }
    }
    let Some(start) = f.tiles.iter().position(|t| t == "up").filter(|&at| open[at]) else { return false; };
    let ordinary: Vec<Vec<_>> = (0..n).map(|at| if open[at] {
        neighbors(at, w, n).into_iter().flatten().filter(|&next| open[next]).collect()
    } else { vec![] }).collect();
    let original = reachable(start, &ordinary);
    let mut arrivals: Vec<Vec<usize>> = (0..n).map(|at| vec![at]).collect();
    for p in f.portals.as_ref().into_iter().flatten().filter(|_| !avoid_portals) {
        if !open[p.a] || !open[p.b] { return false; }
        arrivals[p.a] = vec![p.b]; arrivals[p.b] = vec![p.a];
    }
    for pocket in f.non_euclidean.as_ref().into_iter().flatten().chain(f.speed_up.as_ref().into_iter().flatten()) {
        if pocket.cells.iter().any(|&at| !original[at]) { return false; }
        for group in &pocket.groups {
            arrivals[group.destination] = group.sources.clone();
            for &source in &group.sources { arrivals[source] = vec![group.destination]; }
        }
    }
    let mut forward = vec![vec![]; n];
    let mut reverse = vec![vec![]; n];
    for at in 0..n {
        for &entry in &ordinary[at] {
            let own_speed_pair=speedup_at(f,at).is_some_and(|p|p.groups.iter().any(|g| (g.destination==at||g.sources.contains(&at))&&(g.destination==entry||g.sources.contains(&entry))));
            let local=[entry];
            let options=if own_speed_pair { &local[..] } else { &arrivals[entry][..] };
            for &arrival in options {
                if !open[arrival] { return false; }
                forward[at].push(arrival); reverse[arrival].push(at);
            }
        }
    }
    let outbound = reachable(start, &forward);
    let homebound = reachable(start, &reverse);
    (0..n).all(|at| !original[at] || outbound[at] && homebound[at])
}
/// A local 11x11 binary CA (initial live probability .55, three Moore-neighbor
/// birth>=5/survival>=4 steps); only whole cardinal components of 6..48 cells may be selected.
/// Eligibility is a fixed dead mask and terrain is never edited. O(CA_SIDE^2)
/// time/memory because the number of CA rounds is fixed.
fn ca_component(f: &Floor, eligible: &[bool], center: usize, rng: &mut GenerationRng) -> Option<Vec<usize>> {
    let (w, h) = (f.width(), f.height());
    let radius = (CA_SIDE / 2) as i32;
    let (cx, cy) = ((center % w) as i32, (center / w) as i32);
    let mut indices = vec![None; CA_SIDE * CA_SIDE];
    let mut alive = vec![false; CA_SIDE * CA_SIDE];
    for y in 1..CA_SIDE - 1 { for x in 1..CA_SIDE - 1 {
        let (xx, yy) = (cx + x as i32 - radius, cy + y as i32 - radius);
        if xx >= 0 && yy >= 0 && xx < w as i32 && yy < h as i32 {
            let at = yy as usize * w + xx as usize;
            if eligible[at] { indices[y * CA_SIDE + x] = Some(at); alive[y * CA_SIDE + x] = rng.random() < 0.55; }
        }
    }}
    for _ in 0..CA_STEPS {
        let mut next = vec![false; alive.len()];
        for y in 1..CA_SIDE - 1 { for x in 1..CA_SIDE - 1 {
            let i = y * CA_SIDE + x;
            if indices[i].is_none() { continue; }
            let mut count = 0;
            for yy in y-1..=y+1 { for xx in x-1..=x+1 {
                if (xx != x || yy != y) && alive[yy * CA_SIDE + xx] { count += 1; }
            }}
            next[i] = count >= if alive[i] { 4 } else { 5 };
        }}
        alive = next;
    }
    let mut components = Vec::new();
    let mut seen = vec![false; alive.len()];
    for i in 0..alive.len() {
        if !alive[i] || seen[i] { continue; }
        let mut queue = vec![i]; seen[i] = true;
        let mut head = 0;
        while head < queue.len() {
            let at = queue[head]; head += 1;
            for next in neighbors(at, CA_SIDE, alive.len()).into_iter().flatten() {
                if alive[next] && !seen[next] { seen[next] = true; queue.push(next); }
            }
        }
        if (MIN_GENERATED_CELLS..=MAX_CELLS).contains(&queue.len()) {
            let mut component: Vec<_> = queue.into_iter().map(|j| indices[j].unwrap()).collect();
            component.sort_unstable(); components.push(component);
        }
    }
    if components.is_empty() { None } else {
        Some(components.swap_remove((rng.random() * components.len() as f64).floor() as usize))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationReport {
    pub shape_attempts: usize,
    pub shape_candidates: usize,
    pub assignment_attempts: usize,
    pub assignment_rejections: usize,
    pub safety_rejections: usize,
    pub accepted_pockets: usize,
    pub accepted_cells: usize,
    pub accepted_groups: usize,
    pub accepted_two_source_groups: usize,
}
pub fn install(seed: &str, f: &mut Floor, actors: &[Actor], ground: &[GroundItem], revision: f64) {
    let _ = install_with_report(seed, f, actors, ground, revision);
}
/// Bounded rejection: S<=48 CA shapes, R<=64 assignments per shape. Worst-case
/// O(V+H+A+I + S*(121+R*(K^2 log K+V+E+P))) time and
/// O(V+E+H+A+I+K+P) scratch: K<=48, E<=8V, P<=7, H fixture footprints,
/// A actors, I ground entries. No art/terrain/population/budget changes.
/// Diagnostic counts do not add RNG draws or serialized gameplay state.
pub fn install_with_report(seed: &str, f: &mut Floor, actors: &[Actor], ground: &[GroundItem], revision: f64) -> GenerationReport {
    let mut report = GenerationReport::default();
    if revision < 36.0 || !f.non_euclidean.is_missing() || requested_pockets(f.depth as u32) == 0 { return report; }
    let (w, n) = (f.width(), f.tiles.len());
    if w == 0 || n % w != 0 { return report; }
    let mut blocked: HashSet<usize> = crate::generation::protected_generation_cells(f).into_iter()
        .chain(crate::spawn_heatmap::fixture_cells(f))
        .chain(actors.iter().map(|a| a.y as usize * w + a.x as usize))
        .chain(ground.iter().map(|g| g.y as usize * w + g.x as usize))
        .chain(f.portals.as_ref().into_iter().flatten().flat_map(|p| [p.a, p.b]))
        .chain(f.speed_up.as_ref().into_iter().flatten().flat_map(|p|p.cells.iter().copied())).collect();
    if let Some(connectors) = f.room_graph.as_ref().map(|g| &g.connectors).or(f.dungeon_connectors.as_ref()) {
        blocked.extend(connectors.iter().flat_map(|c| c.cells.iter().map(|v| *v as usize)));
    }
    if let Some(s) = f.sphinx_room.as_ref() { blocked.extend(s.cells.iter().map(|v| *v as usize)); }
    let open = walking(f);
    let eligible: Vec<_> = (0..n).map(|at| open[at] && !blocked.contains(&at)
        && ["floor", "grass", "shallow-water"].contains(&f.tiles[at].as_str())).collect();
    let centers: Vec<_> = (0..n).filter(|&at| eligible[at]).collect();
    f.non_euclidean = Field::Value(vec![]);
    if centers.is_empty() { return report; }
    let mut rng = crate::population::population_random(&format!("{seed}:floor:{}:non-euclidean-r36", f.depth));
    for _ in 0..SHAPE_ATTEMPTS {
        report.shape_attempts += 1;
        let center = centers[(rng.random() * centers.len() as f64).floor() as usize];
        let Some(cells) = ca_component(f, &eligible, center, &mut rng) else { continue; };
        report.shape_candidates += 1;
        for _ in 0..ASSIGNMENT_ATTEMPTS {
            report.assignment_attempts += 1;
            let Some(groups) = assignment_once(f, &cells, &mut rng) else { report.assignment_rejections += 1; continue; };
            f.non_euclidean = vec![NonEuclideanPocket { cells: cells.clone(), groups }].into();
            if traversable(f) {
                let pocket = &f.non_euclidean.as_ref().unwrap()[0];
                report.accepted_pockets = 1;
                report.accepted_cells = pocket.cells.len();
                report.accepted_groups = pocket.groups.len();
                report.accepted_two_source_groups = pocket.groups.iter().filter(|g| g.sources.len() == 2).count();
                return report;
            }
            report.safety_rejections += 1;
            f.non_euclidean = vec![].into();
        }
    }
    report
}

#[cfg(test)]
mod proposal_tests {
    use super::*;
    fn floor() -> Floor {Floor {depth:20.,tiles:vec!["floor".into();2291],..Default::default()}}
    #[test]
    fn last_two_always_form_one_pair_and_terminal_singletons_reject() {
        let f=floor();let mut draws=0;
        let result=assignment_with_random(&f,&[80,81],&mut || {draws+=1;0.0}).unwrap();
        assert_eq!(result,vec![NonEuclideanGroup{sources:vec![80],destination:81}]);
        assert_eq!(draws,1,"last pair has only initial-source draw, no group-size draw");
        for seed in 0..128 { assert!(assignment_once(&f,&[80,81,82],&mut GenerationRng::new(seed)).is_none()); }
    }
    #[test]
    fn proposal_threshold_and_removed_source_eligibility_are_literal() {
        let f=floor();
        assert_eq!(eligible_destinations(&f,&BTreeSet::from([80,82,83])),vec![82,83]);
        let cells=vec![80,81,82,83,84];
        let mut first=true;
        let groups=assignment_with_random(&f,&cells,&mut || {if first {first=false;0.299999} else {0.0}}).unwrap();
        assert_eq!(groups,vec![NonEuclideanGroup{sources:vec![80,81],destination:82},NonEuclideanGroup{sources:vec![83],destination:84}]);
        let mut first=true;
        assert!(assignment_with_random(&f,&cells,&mut || {if first {first=false;0.30} else {0.0}}).is_none(),"one-source initial proposal leaves this scripted five-cell trace stranded; do not silently rewire");
    }
    #[test]
    fn assignment_uses_diagonal_neighbors_and_forced_next_source_but_never_cuts_wall_corners() {
        let mut f=floor();
        // (1,1) and (2,2) have only diagonal adjacency in this unassigned set.
        let remaining=BTreeSet::from([80,160]);
        assert_eq!(eligible_destinations(&f,&remaining),vec![80,160]);
        let mut draws=[0.99,0.0,0.0,0.99].into_iter();
        let groups=assignment_with_random(&f,&[80,81,160,161],&mut || draws.next().unwrap_or(0.0)).unwrap();
        assert_eq!(groups,vec![NonEuclideanGroup{sources:vec![80],destination:81},NonEuclideanGroup{sources:vec![161],destination:160}]);
        assert!(adjacent(&f,81,161)); // The forced next source is diagonal.
        f.tiles[81]="wall".into();
        assert!(eligible_destinations(&f,&remaining).is_empty(),"one blocking orthogonal wall forbids the diagonal, matching live movement");
        assert!(!adjacent(&f,80,160));
        f.tiles[81]="floor".into();f.tiles[159]="wall".into();
        assert!(!adjacent(&f,80,160),"either orthogonal wall blocks the corner");
        f.tiles[159]="tree".into();
        assert!(adjacent(&f,80,160),"retain the existing wall-only corner rule rather than inventing a stricter tree rule");
    }
    #[test]
    fn source_removal_changes_destination_eligibility_and_forced_chain_is_preserved() {
        let f=floor();let cells=(80..=87).collect::<Vec<_>>();let mut successes=0;let mut triples=0;
        for seed in 0..1000 {
            if let Some(groups)=assignment_once(&f,&cells,&mut GenerationRng::new(seed)) {
                let pocket=NonEuclideanPocket{cells:cells.clone(),groups};
                assert!(assignment_follows_rules(&f,&pocket));successes+=1;
                triples+=pocket.groups.iter().filter(|g|g.sources.len()==2).count();
            }
        }
        assert!(successes>0&&triples>0);
    }
}

/// Corn dimensions use the same literal CA/70:30 assignment algorithm with
/// denser bounded placement. Existing links are never replaced or overlapped.
/// Up to 3*48*64 proposals: O(S*R*(K² log K+V+E)), O(V+E+K) scratch.
pub fn append_in_cells(f:&mut Floor,actors:&[Actor],ground:&[GroundItem],cells:&[usize],wanted:usize,rng:&mut GenerationRng){
    let (w,n)=(f.width(),f.tiles.len());let static_blocked:HashSet<_>=crate::generation::protected_generation_cells(f).into_iter().chain(crate::spawn_heatmap::fixture_cells(f)).chain(actors.iter().map(|a|a.y as usize*w+a.x as usize)).chain(ground.iter().map(|g|g.y as usize*w+g.x as usize)).collect();
    for _ in 0..wanted.min(3){if f.non_euclidean.as_ref().map_or(0,Vec::len)>=MAX_POCKETS{break}let mut used=static_blocked.clone();used.extend(f.portals.as_ref().into_iter().flatten().flat_map(|p|[p.a,p.b]));used.extend(f.non_euclidean.as_ref().into_iter().flatten().chain(f.speed_up.as_ref().into_iter().flatten()).flat_map(|p|p.cells.iter().copied()));
        let mut eligible=vec![false;n];for &at in cells{eligible[at]=!used.contains(&at)&&matches!(f.tiles[at].as_str(),"floor"|"grass"|"shallow-water");}let centers:Vec<_>=cells.iter().copied().filter(|i|eligible[*i]).collect();if centers.is_empty(){break}let mut accepted=false;
        'shape:for _ in 0..SHAPE_ATTEMPTS{let center=centers[(rng.random()*centers.len()as f64).floor()as usize];let Some(cells)=ca_component(f,&eligible,center,rng)else{continue};if cells.len()>16{continue}for _ in 0..ASSIGNMENT_ATTEMPTS{let Some(groups)=assignment_once(f,&cells,rng)else{continue};f.non_euclidean.get_or_insert_with(Vec::new).push(NonEuclideanPocket{cells:cells.clone(),groups});if traversable(f){accepted=true;break 'shape}f.non_euclidean.as_mut().unwrap().pop();}}
        if !accepted{break}
    }
}
