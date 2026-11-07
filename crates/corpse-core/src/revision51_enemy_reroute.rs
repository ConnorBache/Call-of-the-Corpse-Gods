//! A blocked pursuit can spend its existing action on a shorter legal route.
//! The caller supplies the already-known goal. This never acquires a target,
//! samples a portal, retries a failed action, or changes the actor's schedule.
use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::{Actor, Run, engine, movement::{self, Point}};

const UNREACHABLE: u32 = u32::MAX;

#[derive(Debug)]
struct Edge {
    from: usize,
    direction: Point,
    arrivals: Vec<usize>,
    actions: u32,
}

/// Reverse Dijkstra on movement outcomes. An edge is usable only after EVERY
/// exit has a finite cost: cost = departure actions + max(exit costs). This is
/// exact shortest guaranteed action cost, not a lucky stochastic-portal exit.
/// O((V+E+K) log V) time/O(V+E+K) scratch, K total edge outcomes; no RNG.
fn remaining_actions(n: usize, edges: &[Edge], goals: &[usize]) -> Vec<u32> {
    let mut incoming = vec![Vec::new(); n];
    for (edge, e) in edges.iter().enumerate() {
        for &arrival in &e.arrivals { incoming[arrival].push(edge); }
    }
    let mut pending: Vec<_> = edges.iter().map(|e| e.arrivals.len()).collect();
    let mut worst = vec![0; edges.len()];
    let mut costs = vec![UNREACHABLE; n];
    let mut settled = vec![false; n];
    let mut heap = BinaryHeap::new();
    for &goal in goals { costs[goal] = 0; heap.push(Reverse((0, goal))); }
    while let Some(Reverse((cost, at))) = heap.pop() {
        if settled[at] || cost != costs[at] { continue; }
        settled[at] = true;
        for &edge in &incoming[at] {
            pending[edge] -= 1;
            worst[edge] = worst[edge].max(cost);
            if pending[edge] != 0 { continue; }
            let e = &edges[edge];
            let next = worst[edge] + e.actions;
            if next < costs[e.from] {
                costs[e.from] = next;
                heap.push(Reverse((next, e.from)));
            }
        }
    }
    costs
}

fn eligible(r: &Run, a: &Actor, goal: Point) -> bool {
    r.simulation_revision.value_or(29.) >= 51.
        && a.id != r.player.id && a.state == "hunting"
        && a.hp > 0. && !a.dead.value_or(false)
        && a.last_seen.as_ref().is_some_and(|p| p.x == goal.x as f64 && p.y == goal.y as f64)
        && !["confused", "paralyzed", "forcedMove", "riptide", "monsterRetreating"]
            .iter().any(|k| engine::condition(a, k) != 0.)
        && !a.abilities.iter().any(|s| matches!(s.as_str(), "enemy:beast:rooted" | "enemy:beast:minotaur"))
        && !crate::revision50_control_amulets::rooted(r, a)
        && !crate::revision50_grappler::locked(r, a)
        && !crate::ichor::stranded(r, a)
        && !crate::dreamwalking::inert(r, a)
        && !crate::half_here::is_clone(a)
        && !crate::revision50_terrain::is_soldier(a)
        && !crate::revision50_nightmares::is_decoy(a)
}

fn sticky(f: &crate::Floor, a: &Actor, at: usize, flying: bool) -> bool {
    !flying && (f.mud.as_ref().is_some_and(|v| v[at] > 0.)
        || f.roots.as_ref().is_some_and(|v| v[at] > 0.)
        || (f.slime.as_ref().is_some_and(|v| v[at] > 0.)
            && !a.abilities.iter().any(|s| s == "enemy:beast:slime-walker")))
}

/// Only called after the normal planner has selected a step. The cheap probe
/// leaves clear steps, intentional wall attacks, confusion and other goals
/// unchanged. A blocked attempt is not executed first (no duplicate payments,
/// status rolls or sticky-ground struggles). See docs/revision51-enemy-reroute.md
/// for mask/helper costs and the existing full-floor AI topology contract.
pub(crate) fn replacement_step(r: &Run, id: f64, goal: Point, chosen: Point) -> Option<Point> {
    let a = engine::actor(r, id)?;
    if !eligible(r, a, goal) { return None; }
    let f = r.floor()?;
    let source = crate::portals::cell(f, a.x, a.y)?;
    let goal_at = crate::portals::cell(f, goal.x as f64, goal.y as f64)?;
    let target_id = a.conditions.get("targetId").copied();
    let blocked_by_actor = |x, y| r.actors_at(r.depth).iter().any(|b| {
        b.id != id && Some(b.id) != target_id && !b.dead.value_or(false)
            && crate::dreamwalking::same_plane(r, a, b)
            && crate::portals::same_position(r, b, x, y)
            // Discord is an intentional attack, not a blocked pursuit move.
            && !crate::revision50_control_amulets::discord_hostility(r, a, b)
    });
    let (x, y) = (a.x + chosen.x as f64, a.y + chosen.y as f64);
    let outcomes = crate::portals::step_options(r, a, x, y);
    let occupied = outcomes.iter().any(|&(x, y)| blocked_by_actor(x, y));
    // A newly solid ordinary cell can invalidate a phase-cached field. Keep a
    // deliberate Cogworks breach, whose strike consumes the original action.
    let solid = !engine::can_traverse(r, a, x, y)
        && crate::revision50_terrain::breach_step(r, a, goal) != Some(chosen);
    if !occupied && !solid { return None; }
    if f.width() == 0 || f.tiles.len() > 3600 { return None; }

    let _timing = crate::timing::span("path.blocked_reroute");
    let n = f.tiles.len();
    let w = f.width();
    let flying = engine::flying_in(r, a);
    let phase = a.active.iter().any(|s| s == "etill-phase-through");
    let target_cells = crate::portals::aliases(f, goal_at);
    let stair_goal = crate::revision51_stairs::blocked_cell(r, a, f, goal_at);
    let mut goals = if stair_goal { Vec::new() } else { target_cells.clone() };
    let mut terminal = vec![false; n];
    for &at in &goals { terminal[at] = true; }
    let mut occupied = vec![false; n];
    // Do not read the quarry's actual position into the route. A hidden player
    // or decoy may have moved away from last_seen; the old goal stays the goal.
    // Final movement still uses the authoritative collision/admission pipeline.
    for b in r.actors_at(r.depth).iter().filter(|b| b.id != id && Some(b.id) != target_id
        && !b.dead.value_or(false) && crate::dreamwalking::same_plane(r, a, b)) {
        if let Some(at) = crate::portals::cell(f, b.x, b.y) {
            for at in crate::portals::aliases(f, at) { occupied[at] = true; }
        }
    }
    let terrain: Vec<_> = (0..n).map(|at| {
        let (x, y) = ((at % w) as f64, (at / w) as f64);
        engine::actor_passable(r, a, x, y) && !engine::bulwark_blocks(r, a, x, y)
            && (terminal[at] || (crate::ichor::habitat_path_allowed(f, a, at)
                && (f.tiles[at] != "chasm" || flying)))
    }).collect();
    let allowed: Vec<_> = (0..n).map(|at| terrain[at] && (!occupied[at] || terminal[at] || at == source)).collect();
    if stair_goal {
        // Stairs cannot be occupied by this actor. Reach a legal, unoccupied
        // contact square instead, retaining the same known target and corner
        // rule. Unlike the target itself, these squares get no terrain or
        // occupancy exemption. At most eight probes per logical target alias.
        for target in target_cells {
            let to = crate::portals::point(f, target);
            for d in movement::DIRECTIONS {
                let from = Point { x: to.x + d.x, y: to.y + d.y };
                let Some(at) = crate::portals::cell(f, from.x as f64, from.y as f64) else { continue; };
                if allowed[at] && !terminal[at]
                    && movement::diagonal_step_allowed(&f.tiles, w, from, to, phase) {
                    terminal[at] = true;
                    goals.push(at);
                }
            }
        }
        if goals.is_empty() { return None; }
    }
    let mut edges = Vec::with_capacity(n * 8);
    for from in 0..n {
        if !allowed[from] || terminal[from] { continue; }
        let p = crate::portals::point(f, from);
        let actions = 1 + u32::from(sticky(f, a, from, flying)
            && !(from == source && engine::condition(a, "stuck") > 0.));
        for direction in movement::DIRECTIONS {
            let to = Point { x: p.x + direction.x, y: p.y + direction.y };
            let Some(entry) = crate::portals::cell(f, to.x as f64, to.y as f64) else { continue; };
            if !allowed[entry] || !movement::diagonal_step_allowed(&f.tiles, w, p, to, phase) { continue; }
            let mut arrivals = crate::portals::walking_exits(f, from, entry);
            arrivals.sort_unstable(); arrivals.dedup();
            if arrivals.is_empty() || arrivals.iter().any(|&at| !allowed[at]) { continue; }
            edges.push(Edge { from, direction, arrivals, actions });
        }
    }
    let costs = remaining_actions(n, &edges, &goals);
    if costs[source] == UNREACHABLE || costs[source] == 0 { return None; }
    // Edges retain DIRECTIONS order: N,E,S,W,NE,SE,SW,NW. Equal best routes
    // therefore stay deterministic and prefer an orthogonal step when tied.
    edges.iter().filter(|e| e.from == source)
        .filter_map(|e| {
            let remaining = e.arrivals.iter().map(|&at| costs[at]).max()?;
            if remaining >= costs[source] { return None; }
            let (x, y) = (a.x + e.direction.x as f64, a.y + e.direction.y as f64);
            if !engine::can_traverse(r, a, x, y) || e.arrivals.iter().any(|&at| occupied[at]) { return None; }
            Some((remaining + e.actions, e.direction))
        })
        .fold(None, |best: Option<(u32, Point)>, next| {
            if best.is_none_or(|old| next.0 < old.0) { Some(next) } else { best }
        }).map(|(_, step)| step)
}

#[cfg(test)]
#[path = "revision51_enemy_reroute_tests.rs"]
mod tests;
