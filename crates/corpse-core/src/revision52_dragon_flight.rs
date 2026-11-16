//! Paid, bounded Dragon shortcuts. Pure route evaluation never samples portals.
use std::{cmp::Reverse, collections::BinaryHeap};
use crate::{Actor, Run, engine::{self, EngineError, condition}, movement::{self, Point}};
use crate::revision52_enemy_followups::{self as followups, FLIGHT, FLIGHT_COST, FLIGHT_TURNS};
const INF: u32 = u32::MAX;
struct Edge { from: usize, direction: Point, arrivals: Vec<usize>, cost: u32 }
struct Routes { edges: Vec<Edge>, costs: Vec<u32> }

/// Reverse minimax Dijkstra, O((V+E+K) log V) time/O(V+E+K) memory. Every
/// possible portal exit must have a finite route: a lucky exit is not safety.
fn costs(n: usize, edges: &[Edge], goals: &[usize]) -> Vec<u32> {
    let mut incoming = vec![Vec::new(); n];
    for (i, e) in edges.iter().enumerate() { for &at in &e.arrivals { incoming[at].push(i); } }
    let mut pending: Vec<_> = edges.iter().map(|e| e.arrivals.len()).collect();
    let mut worst = vec![0; edges.len()]; let mut settled = vec![false; n];
    let mut result = vec![INF; n]; let mut heap = BinaryHeap::new();
    for &at in goals { result[at] = 0; heap.push(Reverse((0, at))); }
    while let Some(Reverse((cost, at))) = heap.pop() {
        if settled[at] || result[at] != cost { continue; }
        settled[at] = true;
        for &i in &incoming[at] {
            pending[i] -= 1; worst[i] = worst[i].max(cost);
            if pending[i] != 0 { continue; }
            let next = worst[i].saturating_add(edges[i].cost);
            if next < result[edges[i].from] {
                result[edges[i].from] = next; heap.push(Reverse((next, edges[i].from)));
            }
        }
    }
    result
}
fn safe_landing(r: &Run, ground: &Actor, at: usize) -> bool {
    let f = r.floor().unwrap();
    f.tiles[at] != "chasm" && !crate::environment::creature_hazard_at(r, ground, at)
        && crate::environment::current_destination_revision(f, at, r.simulation_revision.value_or(29.)).is_none()
}
/// At most 3600 cells, eight departures each. Occupancy is snapshotted once;
/// per-edge authoritative terrain/portal checks retain corner, stair, habitat
/// and bulwark rules. See complexity ledger for helper costs.
fn routes(r: &Run, a: &Actor, ground: &Actor, goal: Option<Point>, land_only: bool) -> Option<Routes> {
    let f = r.floor()?; let n = f.tiles.len(); let w = f.width();
    if n > 3600 || w == 0 { return None; }
    let mut occupied = vec![false; n];
    for b in std::iter::once(&r.player).chain(r.actors_at(r.depth)) {
        if b.id == a.id || b.dead.value_or(false) || b.hp <= 0. || !crate::dreamwalking::same_plane(r, a, b) { continue; }
        if let Some(at) = crate::portals::cell(f, b.x, b.y) {
            for alias in crate::portals::aliases(f, at) { occupied[alias] = true; }
        }
    }
    let allowed: Vec<_> = (0..n).map(|at| !occupied[at]
        && engine::actor_passable(r, a, (at % w) as f64, (at / w) as f64)
        && !engine::bulwark_blocks(r, a, (at % w) as f64, (at / w) as f64)
        && crate::ichor::habitat_path_allowed(f, a, at)
        && (f.tiles[at] != "chasm" || engine::flying_in(r, a))).collect();
    let target_cells = goal.and_then(|p| crate::portals::cell(f, p.x as f64, p.y as f64))
        .map(|at| crate::portals::aliases(f, at)).unwrap_or_default();
    let goals: Vec<_> = (0..n).filter(|&at| allowed[at] && safe_landing(r, ground, at)
        && (land_only || target_cells.iter().any(|&target| {
            let p = crate::portals::point(f, at); let q = crate::portals::point(f, target);
            (p.x-q.x).abs().max((p.y-q.y).abs()) <= 1
                && movement::diagonal_step_allowed(&f.tiles, w, p, q, false)
        }))).collect();
    if goals.is_empty() { return None; }
    let mut edges = Vec::with_capacity(n * 8);
    let flying = engine::flying_in(r, a);
    let mut origin = a.clone();
    for at in 0..n {
        if !allowed[at] { continue; }
        origin.x = (at % w) as f64; origin.y = (at / w) as f64;
        let sticky = !flying && !(origin.x==a.x && origin.y==a.y && condition(a,"stuck")>0.)
            && (f.mud.as_ref().is_some_and(|v|v[at]>0.)
            || f.roots.as_ref().is_some_and(|v|v[at]>0.)
            || f.slime.as_ref().is_some_and(|v|v[at]>0.));
        for direction in movement::DIRECTIONS {
            let (x,y) = (origin.x + direction.x as f64, origin.y + direction.y as f64);
            let Some(entry) = crate::portals::cell(f,x,y) else { continue; };
            if !allowed[entry] || !engine::can_traverse(r, &origin, x, y) { continue; }
            let mut arrivals = crate::portals::walking_exits(f, at, entry);
            arrivals.sort_unstable(); arrivals.dedup();
            if arrivals.is_empty() || arrivals.iter().any(|&i| !allowed[i]) { continue; }
            edges.push(Edge { from: at, direction, arrivals, cost: 1 + u32::from(sticky) });
        }
    }
    let costs = costs(n, &edges, &goals);
    Some(Routes { edges, costs })
}
/// Convert movement attempts into complete actions without changing the route
/// safety field. Fast has two stages: a fresh action and an earned continuation.
/// A first sticky struggle earns no continuation; a struggle on the continuation
/// spends it, then the next action can move. This doubles the existing bounded
/// minimax graph, retaining O((V+E+K) log V) time/O(V+E+K) transient storage.
fn full_actions(routes: &Routes, at: usize, fast: bool) -> u32 {
    if !fast { return routes.costs[at]; }
    let mut edges=Vec::with_capacity(routes.edges.len()*2);
    for edge in &routes.edges {
        for continuation in [false,true] {
            let sticky=edge.cost>1;
            let next_continuation=!continuation || sticky;
            edges.push(Edge {
                from:edge.from*2+usize::from(continuation), direction:edge.direction,
                arrivals:edge.arrivals.iter().map(|at|at*2+usize::from(next_continuation)).collect(),
                cost:if continuation {edge.cost-1} else {edge.cost},
            });
        }
    }
    let goals=routes.costs.iter().enumerate().filter(|(_,cost)|**cost==0)
        .flat_map(|(at,_)|[at*2,at*2+1]).collect::<Vec<_>>();
    costs(routes.costs.len()*2,&edges,&goals)[at*2]
}
fn normal_turns(r:&Run,a:&Actor,actions:u32)->u32 {
    if actions==0 || actions==INF {return actions;}
    let hasted=crate::ai::is_hasted_in(r,a)
        && !a.abilities.iter().any(|s|s=="enemy:beast:slow" || s=="enemy:pale:slow");
    if !hasted {return actions;}
    // Starting in the normal action leaves one slot this turn, not two.
    let used=u32::from(!crate::revision52_speed::bonus_action(a.id));
    actions.saturating_add(used).div_ceil(2)
}
fn next(routes: &Routes, at: usize) -> Option<Point> {
    if routes.costs[at] == INF || routes.costs[at] == 0 { return None; }
    routes.edges.iter().filter(|e|e.from==at).find(|e|
        e.arrivals.iter().map(|&i|routes.costs[i]).max().unwrap_or(INF)
            .saturating_add(e.cost) == routes.costs[at]).map(|e|e.direction)
}
fn land(r: &mut Run, id: f64) {
    let a = engine::actor_mut_trusted(r,id).unwrap();
    // A later, longer levitation grant owns the shared flight timer. Ending
    // this paid shortcut must not delete somebody else's remaining flight.
    if condition(a,"flying") <= condition(a,FLIGHT) { a.conditions.shift_remove("flying"); }
    a.conditions.shift_remove(FLIGHT);
    let a = a.clone();
    crate::animation_enemy::mark(r,&a,"E29","fold",crate::events::Stage::Special);
}
pub fn turn(r: &mut Run, id: f64, target: Option<f64>, movement_only: bool) -> Result<bool, EngineError> {
    turn_inner(r,id,target,movement_only,false)
}
/// Offscreen allies and interrupted orders finish only their already-paid
/// landing; this grants no pursuit, attack or new activation while absent.
pub fn landing_turn(r: &mut Run, id: f64) -> Result<bool, EngineError> {
    if !followups::enabled(r) || engine::actor(r,id).is_none_or(|a|condition(a,FLIGHT)<=0.) { return Ok(false); }
    turn_inner(r,id,None,true,true)
}
fn turn_inner(r: &mut Run, id: f64, target: Option<f64>, movement_only: bool, landing_only: bool) -> Result<bool, EngineError> {
    if !followups::enabled(r) { return Ok(false); }
    let Some(a) = engine::actor(r,id).cloned() else { return Ok(false); };
    if id == r.player.id || !followups::species(&a,"dragon") || a.dead.value_or(false) || a.hp <= 0.
        || crate::half_here::is_clone(&a) || crate::dreamwalking::inert(r,&a) { return Ok(false); }
    let active = condition(&a,FLIGHT)>0.;
    if active && condition(&a,"flying")<=0. {
        engine::actor_mut_trusted(r,id).unwrap().conditions.shift_remove(FLIGHT); return Ok(false);
    }
    if ["confused","paralyzed","forcedMove","riptide"].iter().any(|k|condition(&a,k)>0.)
        || crate::revision50_grappler::locked(r,&a) || crate::revision50_control_amulets::rooted(r,&a) {
        return Ok(false);
    }
    // An already-airborne hunter follows the actual chasm exit using ordinary
    // pursuit. Do not fold its wings on adjacent ground or buy a new shortcut;
    // the existing finite flight timer still expires on normal upkeep.
    if !landing_only && active && crate::travel::flying_chasm_chase(r,&a) { return Ok(false); }
    let goal = if landing_only {None} else {target.and_then(|tid|engine::actor(r,tid)).map(|b|Point{x:b.x as i32,y:b.y as i32})
        .or_else(||a.last_seen.as_ref().map(|p|Point{x:p.x as i32,y:p.y as i32}))};
    let mut ground = a.clone(); ground.conditions.shift_remove("flying");
    let mut air = a.clone(); air.conditions.insert("flying".into(),FLIGHT_TURNS);
    let Some(at) = r.floor().and_then(|f|crate::portals::cell(f,a.x,a.y)) else { return Ok(false); };
    if !active {
        if movement_only || !matches!(a.state.as_str(),"hunting"|"ally") || goal.is_none()
            || engine::flying_in(r,&a) || condition(&a,"slowed")>0. || a.energy<FLIGHT_COST
            || crate::abilities::energy_activation_silenced(r,&a) { return Ok(false); }
        let Some(air_routes) = routes(r,&air,&ground,goal,false) else { return Ok(false); };
        let airborne = air_routes.costs[at];
        // Takeoff earns no movement. Before the next ordinary action, upkeep
        // spends one flight tick; active pursuit still reserves one landing
        // tick. Do not admit a route that the next action would abandon, and
        // never rely on future Fast/Haste movement to satisfy this safety cap.
        if airborne == 0 || airborne > (FLIGHT_TURNS - 2.) as u32 { return Ok(false); }
        let fast=crate::ai::fast_movement_enabled(r,&a);
        let ground_actions=routes(r,&ground,&ground,goal,false).map_or(INF,|p|full_actions(&p,at,fast));
        // Takeoff owns one full action and earns no Fast movement. Haste's
        // current action position matters when both routes reach the same tick.
        let air_actions=full_actions(&air_routes,at,fast).saturating_add(1);
        let air_turns=normal_turns(r,&a,air_actions);
        let ground_turns=normal_turns(r,&a,ground_actions);
        if air_turns >= ground_turns || !engine::enemy_spend(r,id,FLIGHT_COST) { return Ok(false); }
        let a = engine::actor_mut_trusted(r,id).unwrap();
        a.conditions.insert("flying".into(),FLIGHT_TURNS); a.conditions.insert(FLIGHT.into(),FLIGHT_TURNS);
        let a = a.clone();
        crate::animation_enemy::mark(r,&a,"E29","unfold",crate::events::Stage::Special);
        if crate::vision::visible_actor(r,&a) { engine::log(r,format!("{} takes flight.",crate::actor_presentation::name(r,&a)),"enemy"); }
        return Ok(true);
    }
    // Re-plan each real action for changing occupancy and target position.
    // The timer is advanced only by ordinary upkeep, never by bonus movement.
    let budget = (condition(&a,FLIGHT).min(condition(&a,"flying"))-1.).max(0.) as u32;
    let budget = if condition(&a,"slowed")>0. { budget/2 } else { budget };
    let toward = goal.and_then(|_|routes(r,&air,&ground,goal,false));
    if toward.as_ref().is_some_and(|p|p.costs[at]==0) { land(r,id); return Ok(false); }
    let route = if toward.as_ref().is_some_and(|p|p.costs[at]<=budget) { toward }
        else { routes(r,&air,&ground,None,true) };
    let Some(route) = route else { return Ok(true); };
    if route.costs[at]==0 { land(r,id); return Ok(false); }
    let Some(step) = next(&route,at) else { return Ok(true); };
    engine::move_actor(r,id,step.x as f64,step.y as f64,movement_only)?;
    let Some(after) = engine::actor(r,id).cloned() else { return Ok(true); };
    let Some(now) = r.floor().and_then(|f|crate::portals::cell(f,after.x,after.y)) else { return Ok(true); };
    if safe_landing(r,&ground,now) && (route.costs[now]==0 || condition(&after,"flying")<=1.) { land(r,id); }
    Ok(true)
}

#[cfg(test)]
#[path="revision52_dragon_flight_tests.rs"]
mod tests;
