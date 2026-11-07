//! Revision-31 floor-one entrance, independent of ordinary inter-floor stairs.
//!
//! The normal generator first reserves its existing entrance room at `generation_anchor`.
//! This final, RNG-free pass extends that safe room to the bottom interior row. Historical
//! revisions never call it, so their terrain, generation streams and population stay exact.
use crate::{Field, Floor, FloorLightSourcesEntry, generation};
use serde::{Deserialize, Serialize};
use std::{cmp::Reverse, collections::BinaryHeap};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entrance {
    pub x: f64,
    pub y: f64,
}

/// Native normal floors are 79 by 29. The generator requires a five-cell stair inset.
pub fn generation_anchor() -> generation::Point {
    generation::Point { x: 39, y: 23 }
}

/// Coordinate-only identity, suitable for projection against remembered terrain.
/// It never inspects a live tile that may be outside the player's knowledge.
pub fn metadata_cell(f: &Floor) -> Option<usize> {
    let e = f.entrance.as_ref()?;
    let (w, h) = (f.width(), f.height());
    if f.depth != 1.0
        || w < 3
        || h < 3
        || !e.x.is_finite()
        || !e.y.is_finite()
        || e.x != (w / 2) as f64
        || e.y != (h - 2) as f64
    {
        return None;
    }
    (h - 2).checked_mul(w)?.checked_add(w / 2).filter(|at| *at < f.tiles.len())
}

/// Persistent identity is metadata, not a new movement tile or an escape action.
pub fn cell(f: &Floor) -> Option<usize> {
    let at = metadata_cell(f)?;
    f.tiles.get(at).is_some_and(|t| t == "up").then_some(at)
}

/// Initial player landing keeps the door glyph visible immediately to its south.
pub fn arrival(f: &Floor) -> Option<generation::Point> {
    let at = cell(f)?;
    let north = at.checked_sub(f.width())?;
    (["floor", "grass"].contains(&f.tiles[north].as_str()) && !hazardous(f, north))
        .then_some(generation::Point { x: (at % f.width()) as i32, y: (at / f.width()) as i32 - 1 })
}

pub fn validate(f: &Floor) -> bool {
    f.entrance.is_missing() || cell(f).is_some()
}

fn layer_nonzero(layer: &Field<Vec<f64>>, at: usize) -> bool {
    layer.as_ref().and_then(|v| v.get(at)).is_some_and(|v| *v != 0.0)
}

fn hazardous(f: &Floor, at: usize) -> bool {
    [&f.mud, &f.slime, &f.roots, &f.razorleaves, &f.ichor]
        .iter()
        .any(|layer| layer_nonzero(layer, at))
        || f.environment.as_ref().is_some_and(|e| {
            [&e.fire, &e.caustic, &e.breather, &e.paralysis]
                .iter()
                .any(|v| v.get(at).is_some_and(|v| *v != 0.0))
                || [
                    &e.gas_volume,
                    &e.confusion,
                    &e.steam,
                    &e.flammable,
                    &e.explosive,
                    &e.hydrogen,
                ]
                .iter()
                .any(|layer| layer_nonzero(layer, at))
                || e.gas_type.as_ref().and_then(|v| v.get(at)).is_some_and(Option::is_some)
        })
        || f.bulwarks.as_ref().is_some_and(|v| v.contains_key(&at.to_string()))
}

fn authored_fixture(f: &Floor, at: usize) -> bool {
    let (x, y) = (at % f.width(), at / f.width());
    f.setups.as_ref().into_iter().flatten()
        .any(|s| generation::setup_contains(s, x as i32, y as i32))
        || f.setup_spawns.as_ref().into_iter().flatten()
            .any(|s| s.x == x as f64 && s.y == y as f64)
        || f.vents.as_ref().into_iter().flatten().any(|v| v.at == at as f64)
        || f.grates.as_ref().is_some_and(|v| v.contains(&(at as f64)))
        || f.godbeast_event.as_ref().is_some_and(|e| {
            x as f64 >= e.room.left && x as f64 <= e.room.right
                && y as f64 >= e.room.top && y as f64 <= e.room.bottom
        })
}

fn clear_brazier(f: &mut Floor, at: usize) {
    if let Some(v) = f.hellfire.as_mut().and_then(|v| v.get_mut(at)) { *v = false; }
    if let Some(fires) = f.permanent_fire.as_mut() { fires.retain(|v| *v != at as f64); }
    if let Some(e) = f.environment.as_mut() {
        if let Some(v) = e.fire.get_mut(at) { *v = 0.0; }
        if let Some(v) = e.fire_kind.as_mut().and_then(|v| v.get_mut(at)) { *v = None; }
        if let Some(v) = e.fire_fresh.as_mut().and_then(|v| v.get_mut(at)) { *v = false; }
        if let Some(v) = e.fire_reaction.as_mut().and_then(|v| v.get_mut(at)) { *v = 0.0; }
        if let Some(p) = e.provenance.as_mut().and_then(|v| v.get_mut("fire")) {
            if let Some(v) = p.owner.get_mut(at) { *v = -1.0; }
            if let Some(v) = p.player.get_mut(at) { *v = 0.0; }
        }
    }
}

fn clear_required_fixtures(f: &mut Floor) -> Result<Vec<FloorLightSourcesEntry>, String> {
    let w = f.width();
    let target = (f.height() - 2) * w + w / 2;
    let mut lights = Vec::new();
    for at in [target, target - w] {
        if authored_fixture(f, at) {
            return Err("Floor-one entrance conflicts with an authored fixture".into());
        }
        let sources: Vec<_> = f.light_sources.as_ref().into_iter().flatten()
            .filter(|s| s.y as usize * w + s.x as usize == at).cloned().collect();
        if sources.len() > 1 || sources.iter().any(|s| {
            !s.kind.as_ref().is_some_and(|k| ["torch", "brazier"].contains(&k.as_str()))
        }) {
            return Err("Floor-one entrance conflicts with unsupported lighting".into());
        }
        if let Some(source) = sources.into_iter().next() {
            f.light_sources.as_mut().unwrap().retain(|s| s.y as usize * w + s.x as usize != at);
            if source.kind.as_ref().is_some_and(|k| k == "brazier") { clear_brazier(f, at); }
            lights.push(source);
        }
        if let Some(doors) = f.doors.as_mut() { doors.shift_remove(&at.to_string()); }
        // Keep diagnostic room/connector references consistent with this narrow replacement.
        if let Some(graph) = f.room_graph.as_mut() {
            for r in &mut graph.rooms { r.doors.retain(|v| *v != at as f64); }
            for c in &mut graph.connectors { c.doors.retain(|v| *v != at as f64); }
        }
        for r in f.dungeon_rooms.as_mut().into_iter().flatten() { r.doors.retain(|v| *v != at as f64); }
        for c in f.dungeon_connectors.as_mut().into_iter().flatten() { c.doors.retain(|v| *v != at as f64); }
    }
    Ok(lights)
}

/// Relocation is exceptional (at most the two mandatory cells). Torch checks are local;
/// a brazier candidate additionally uses the existing O(V+M) safety/connectivity checks.
/// With L<=2 lights this is O(L * (V log V + V*(V+M))) worst-case, O(V+M) scratch.
fn relocate_lights(f: &mut Floor, lights: Vec<FloorLightSourcesEntry>) {
    let (w, h) = (f.width(), f.height());
    let entrance = cell(f).unwrap();
    for mut light in lights {
        let old = light.y as usize * w + light.x as usize;
        let brazier = light.kind.as_ref().is_some_and(|k| k == "brazier");
        let protected = generation::protected_generation_cells(f);
        let graph = brazier.then(|| generation::build_generation_gas_components(f));
        let mut candidates: Vec<_> = (0..f.tiles.len()).filter(|&at| {
            let (x, y) = (at % w, at / w);
            x > 0 && x < w - 1 && y > 0 && y < h - 1
                && !protected.contains(&at) && !hazardous(f, at)
                && x.abs_diff(entrance % w).max(y.abs_diff(entrance / w)) > 5
                && if brazier { f.tiles[at] == "floor" } else {
                    f.tiles[at] == "wall" && generation::cardinal_neighbors(at, w, h).iter()
                        .any(|&i| ["floor", "grass"].contains(&f.tiles[i].as_str()))
                }
        }).collect();
        candidates.sort_unstable_by_key(|&at| {
            ((at % w).abs_diff(old % w).pow(2) + (at / w).abs_diff(old / w).pow(2), at)
        });
        let destination = candidates.into_iter().find(|&at| {
            !brazier || (generation::ordinary_brazier_site_safe(f, at, graph.as_ref().unwrap())
                && generation::permanent_fire_safety_cells(f, at).iter()
                    .all(|&i| !generation::generated_terrain_is_fuel(f, i))
                && generation::brazier_preserves_component_access(f, at))
        });
        // Only an ordinary light is omitted if no safe existing terrain can accept it.
        let Some(at) = destination else { continue; };
        light.x = (at % w) as f64;
        light.y = (at / w) as f64;
        f.light_sources.get_or_insert_with(Vec::new).push(light);
        if brazier {
            f.permanent_fire.get_or_insert_with(Vec::new).push(at as f64);
            if let Some(e) = f.environment.as_mut() {
                let kinds = e.fire_kind.get_or_insert_with(|| e.fire.iter().map(|v| {
                    if *v > 0.0 { Some("creature".into()) } else { None }
                }).collect());
                if let Some(v) = kinds.get_mut(at) { *v = Some("brazier".into()); }
                if let Some(v) = e.fire.get_mut(at) { *v = 1.0; }
                if let Some(p) = e.provenance.as_mut().and_then(|v| v.get_mut("fire")) {
                    if let Some(v) = p.owner.get_mut(at) { *v = -1.0; }
                    if let Some(v) = p.player.get_mut(at) { *v = 0.0; }
                }
            }
        }
    }
}

/// Atomically replace only ordinary fixtures on the two required entrance cells, then
/// connect the room and safely relocate those lights without changing authored machines.
pub fn install_floor_one_entrance(f: &mut Floor) -> Result<(), String> {
    if f.depth != 1.0 { return Ok(()); }
    if !f.entrance.is_missing() {
        return if validate(f) { Ok(()) } else { Err("Invalid floor-one entrance identity".into()) };
    }
    let (w, h, n) = (f.width(), f.height(), f.tiles.len());
    if w < 5 || h < 5 || w.checked_mul(h) != Some(n) || n > 3600 {
        return Err("Invalid floor-one entrance geometry".into());
    }
    let mut draft = f.clone();
    let lights = clear_required_fixtures(&mut draft)?;
    install_inner(&mut draft)?;
    relocate_lights(&mut draft, lights);
    *f = draft;
    Ok(())
}

/// The only mutable terrain is the chosen cardinal path and the previous up tile.
/// A stable Dijkstra minimizes changed terrain, then length, then row-major cell index.
/// O((V + M) log V) time and O(V + M) temporary memory, including bounded fixture/setup
/// expansion M; grid degree is four. No random draws or simulation-time hooks are added.
fn install_inner(f: &mut Floor) -> Result<(), String> {
    if f.depth != 1.0 {
        return Ok(());
    }
    if !f.entrance.is_missing() {
        return if validate(f) { Ok(()) } else { Err("Invalid floor-one entrance identity".into()) };
    }
    let (w, h, n) = (f.width(), f.height(), f.tiles.len());
    if w < 5 || h < 5 || w.checked_mul(h) != Some(n) || n > 3600 {
        return Err("Invalid floor-one entrance geometry".into());
    }
    let old = f.tiles.iter().position(|t| t == "up").ok_or("Floor one has no generated entrance")?;
    if f.tiles.iter().filter(|t| *t == "up").count() != 1 {
        return Err("Floor one requires exactly one generated entrance".into());
    }
    let target = (h - 2) * w + w / 2;
    let mut protected = generation::protected_generation_cells(f);
    let mut old_torches = Vec::new();
    if let Some(recess) = f.generation.as_ref()
        .and_then(|g| g.stair_recesses.as_ref())
        .and_then(|rs| rs.iter().find(|r| r.at == old as f64))
    {
        let cells = generation::stair_recess_cells(recess, w)?;
        for at in cells.footprint {
            protected.shift_remove(&at);
        }
        old_torches = cells.torches;
    }
    let mut blocked = vec![false; n];
    for at in 0..n {
        blocked[at] = hazardous(f, at) || ["down", "pit"].contains(&f.tiles[at].as_str());
    }
    // Do not cut a machine footprint, a fixture, a containment wall, or its hardware.
    for setup in f.setups.as_ref().into_iter().flatten() {
        let (left, right, top, bottom) = setup.bounds.as_ref()
            .map(|b| (b.left, b.right, b.top, b.bottom))
            .unwrap_or((setup.x - 6.0, setup.x + 6.0, setup.y - 6.0, setup.y + 6.0));
        for y in top.max(0.0) as usize..=bottom.min((h - 1) as f64) as usize {
            for x in left.max(0.0) as usize..=right.min((w - 1) as f64) as usize {
                if generation::setup_contains(setup, x as i32, y as i32) { blocked[y * w + x] = true; }
            }
        }
    }
    for at in crate::spawn_heatmap::fixture_cells(f) {
        if at < n { blocked[at] = true; }
    }
    for source in f.light_sources.as_ref().into_iter().flatten() {
        let at = source.y as usize * w + source.x as usize;
        if at < n && !(old_torches.contains(&at) && source.kind.as_ref().is_some_and(|k| k == "torch")) {
            blocked[at] = true;
        }
    }
    if let Some(e) = f.godbeast_event.as_ref() {
        for y in e.room.top as usize..=e.room.bottom as usize {
            for x in e.room.left as usize..=e.room.right as usize {
                if let Some(b) = blocked.get_mut(y * w + x) { *b = true; }
            }
        }
    }
    let allowed = |at: usize| {
        let (x, y) = (at % w, at / w);
        x > 0 && x < w - 1 && y > 0 && y < h - 1 && !blocked[at]
            && (!protected.contains(&at) || ["floor", "grass", "up"].contains(&f.tiles[at].as_str()))
    };
    if !allowed(target) || !allowed(target - w) || !allowed(old) {
        return Err("Floor-one entrance conflicts with a protected fixture".into());
    }
    let mut costs = vec![usize::MAX; n];
    let mut previous = vec![usize::MAX; n];
    let mut queue = BinaryHeap::new();
    costs[target] = 0;
    queue.push(Reverse((0, target)));
    while let Some(Reverse((cost, at))) = queue.pop() {
        if costs[at] != cost { continue; }
        if at == old { break; }
        for next in generation::cardinal_neighbors(at, w, h) {
            // The door faces north: never leave the initial arrival tile isolated.
            if (at == target && next != target - w) || !allowed(next) { continue; }
            let change = usize::from(!["floor", "grass", "up"].contains(&f.tiles[next].as_str()));
            let candidate = cost + change * (n + 1) + 1;
            if candidate < costs[next] {
                costs[next] = candidate;
                previous[next] = at;
                queue.push(Reverse((candidate, next)));
            }
        }
    }
    if costs[old] == usize::MAX {
        return Err("Floor-one entrance cannot safely reach its generated room".into());
    }
    let mut path = vec![old];
    while *path.last().unwrap() != target {
        path.push(previous[*path.last().unwrap()]);
    }
    // Planning above is read-only, so any error leaves the caller's floor untouched.
    let mut corridor = vec![false; n];
    for at in path {
        corridor[at] = true;
        if !["floor", "grass"].contains(&f.tiles[at].as_str()) {
            f.tiles[at] = "floor".into();
            if let Some(v) = f.substrates.as_mut().and_then(|v| v.get_mut(at)) {
                *v = if f.domain == "Dungeon" { "tiled" } else { "rock" }.into();
            }
            if let Some(v) = f.lights.get_mut(at) { *v = 0.0; }
        }
        if let Some(v) = f.currents.as_mut().and_then(|v| v.get_mut(at)) { *v = 0.0; }
    }
    if let Some(sources) = f.light_sources.as_mut() {
        sources.retain(|s| {
            let at = s.y as usize * w + s.x as usize;
            !(old_torches.contains(&at) && corridor[at] && s.kind.as_ref().is_some_and(|k| k == "torch"))
        });
    }
    if let Some(recesses) = f.generation.as_mut().and_then(|g| g.stair_recesses.as_mut()) {
        recesses.retain(|r| r.at != old as f64);
    }
    f.tiles[target] = "up".into();
    f.entrance = Entrance { x: (w / 2) as f64, y: (h - 2) as f64 }.into();
    Ok(())
}
