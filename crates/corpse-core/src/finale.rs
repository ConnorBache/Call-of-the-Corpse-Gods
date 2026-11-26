//! Revision-31 Administrator and deterministic staircase-propagating escape collapse.
//! No simulation RNG is used by placement or collapse. The original stairs are captured
//! before their tiles disappear; each pulse uses only the previous frontier.
use crate::{Actor, Field, Floor, Run, Stats};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};

pub const BOSS_KIND: &str = "flesh-administrator";
pub const BOSS_NAME: &str = "The Flesh Administrator";
pub const FALL_MESSAGE: &str = "Fell while escaping the collapse of the flesh.";
pub const WIN_MESSAGE: &str = "Collapsed the flesh!";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CollapseFloor {
    pub depth: u32,
    pub up: Option<usize>,
    pub upper_seeded: bool,
    pub cells: Vec<bool>,
    pub frontier: Vec<usize>,
    /// R52 alternates cardinal-only and diagonal-inclusive pulses per reached floor.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub diagonal_next: bool,
    /// Missing for a collapse already started under the original rules.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fire: Option<FireWave>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FireWave {
    origin: usize,
    down: Option<usize>,
    cells: Vec<bool>,
    frontier: Vec<usize>,
    upper_seeded: bool,
    diagonal_next: bool,
}

impl CollapseFloor {
    fn protected_stair(&self, at: usize) -> bool {
        self.fire.as_ref().is_some_and(|fire| Some(at) == self.up || Some(at) == fire.down)
    }
    fn gap(&self, at: usize) -> bool {
        self.cells.get(at) == Some(&true) && !self.protected_stair(at)
    }
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Collapse {
    pub boss_id: f64,
    pub started_tick: f64,
    pub next_pulse_tick: f64,
    pub escaped: bool,
    pub waves: Vec<CollapseFloor>,
}

pub fn enabled(r: &Run) -> bool { r.simulation_revision.value_or(29.) >= 31. }
fn current(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 52. && matches!(r.collapse_rules_from, Field::Value(_))
}
fn validate_policy(r: &Run) -> Result<(), String> {
    if !r.collapse_rules_from.is_missing() && (r.simulation_revision.value_or(29.) < 52.
        || r.collapse_rules_from.as_ref().is_some_and(|&n| n > r.replay.len())) {
        return Err("Invalid collapse rules command boundary".into());
    }
    Ok(())
}
pub fn admit_current(r: &mut Run) -> Result<(), String> {
    validate_policy(r)?;
    if r.simulation_revision.value_or(29.) >= 52. && r.collapse_rules_from.is_missing() {
        r.collapse_rules_from = r.replay.len().into();
    }
    Ok(())
}
pub(crate) fn apply_replay_policy(r: &mut Run, from: &Field<usize>, cursor: usize) {
    if r.simulation_revision.value_or(29.) < 52. { return; }
    if from.as_ref() == Some(&cursor) {
        r.collapse_rules_from = cursor.into();
    } else if cursor == 0 {
        r.collapse_rules_from = Field::Missing;
    }
}

pub fn is_boss(a: &Actor) -> bool { a.creature_kind.as_ref().is_some_and(|k| k == BOSS_KIND) }
pub fn ending(r: &Run) -> Option<&'static str> {
    (r.status == "won" && r.collapse.as_ref().is_some_and(|c| c.escaped)).then_some(WIN_MESSAGE)
}
pub fn collapsed(r: &Run, depth: f64, at: usize) -> bool {
    r.collapse.as_ref().and_then(|c| c.waves.iter().find(|w| w.depth as f64 == depth))
        .is_some_and(|w| w.gap(at))
}
fn at(f: &Floor, a: &Actor) -> Option<usize> {
    (a.x >= 0. && a.y >= 0. && a.x < f.width() as f64 && a.y < f.height() as f64)
        .then(|| a.y as usize * f.width() + a.x as usize)
}
fn stair(f: &Floor, kind: &str) -> Option<usize> {
    f.tiles.iter().position(|t| t == kind).or_else(|| f.altered_chasms.as_ref()
        .and_then(|m| m.iter().find(|(_, v)| v.tile == kind).and_then(|(k, _)| k.parse().ok())))
}

/// Provisional standalone boss tuning, independent of ordinary species/reward budgets.
/// HP follows the existing 100+6*Defense law so ordinary health synchronization is exact.
pub fn administrator(id: f64, x: f64, y: f64) -> Actor {
    Actor {
        id, x, y, name: BOSS_NAME.into(), creature_kind: BOSS_KIND.to_string().into(),
        glyph: "A".into(), color: "#f56f91".into(), hp: 1450., max_hp: 1450., energy: 100.,
        stats: Stats { offense: 550., defense: 225., accuracy: 115., dodge: 55., dr: 60.,
            perception: 16., max_energy: 100., ..Default::default() },
        state: "wandering".into(), heartless: true.into(), ..Default::default()
    }
}

pub fn administrator_revision(id: f64, x: f64, y: f64, revision: f64) -> Actor {
    let mut a = administrator(id, x, y);
    crate::physical_balance::apply_new_actor_kind(&mut a, BOSS_KIND, revision);
    a
}

fn initial_hazard(f: &Floor, i: usize) -> bool {
    [&f.ichor,&f.roots,&f.razorleaves].iter().any(|v| v.as_ref().is_some_and(|v| v[i] > 0.))
        || f.environment.as_ref().is_some_and(|e| {
            [&e.fire,&e.caustic,&e.breather,&e.paralysis].iter().any(|v| v[i] > 0.)
                || [&e.confusion,&e.flammable,&e.explosive,&e.hydrogen,&e.steam].iter()
                    .any(|v| v.as_ref().is_some_and(|v| v[i] > 0.))
        })
}

/// Once per new floor40: O(V+A+I) plus a bounded-degree O(V) distance field, O(V+A+I) scratch.
/// A deterministic farthest reachable safe floor cell avoids stairs, arrival, actors and loot.
/// This is separate from all ordinary horde choices and does not consume either RNG stream.
pub(crate) fn populate(r: &mut Run, f: &mut Floor, actors: &mut Vec<Actor>, ground: &[crate::GroundItem]) -> Result<(), String> {
    if !enabled(r) || f.depth != 40. { return Ok(()); }
    // The old bottomless-pit victory is not part of the new boss-and-ascent objective.
    for tile in &mut f.tiles { if tile == "pit" { *tile = "floor".into(); } }
    if r.collapse.as_ref().is_some() || actors.iter().any(is_boss) { return Ok(()); }
    let up = stair(f, "up").ok_or("Administrator floor has no arrival stair")?;
    let w = f.width();
    let mut blocked: HashSet<usize> = actors.iter().filter_map(|a| at(f, a)).collect();
    blocked.extend(ground.iter().map(|g| g.y as usize * w + g.x as usize));
    blocked.extend(f.grates.as_ref().into_iter().flatten().map(|i| *i as usize));
    blocked.extend(f.vents.as_ref().into_iter().flatten().map(|v| v.at as usize));
    blocked.extend(f.light_sources.as_ref().into_iter().flatten().map(|s| s.y as usize * w + s.x as usize));
    let allowed: Vec<_> = f.tiles.iter().enumerate().map(|(i,t)| {
        !["wall", "tree", "chasm", "pit"].contains(&t.as_str())
            && !f.grates.as_ref().is_some_and(|v| v.contains(&(i as f64)))
            && !initial_hazard(f, i)
    }).collect();
    let distance = crate::movement::distance_field(&f.tiles, w, crate::movement::Point {
        x: (up % w) as i32, y: (up / w) as i32,
    }, false, false, Some(&allowed), None);
    let chosen = (0..f.tiles.len()).filter(|i| distance.steps[*i] >= 3 && allowed[*i]
        && ["floor", "grass"].contains(&f.tiles[*i].as_str()) && !blocked.contains(i)
        && !f.doors.as_ref().is_some_and(|d| d.contains_key(&i.to_string())))
        .max_by_key(|i| (distance.steps[*i], std::cmp::Reverse(*i)))
        .ok_or("Administrator floor has no safe reachable boss position")?;
    actors.push(administrator_revision(r.next_id, (chosen % w) as f64, (chosen / w) as f64, r.simulation_revision.value_or(29.0)));
    r.next_id += 1.;
    Ok(())
}

/// Called only after the ordinary actual-death/rescue gate. No corpse or extra heart is made.
pub fn actual_death(r: &mut Run, a: &Actor) -> Result<bool, String> {
    if !enabled(r) || !is_boss(a) { return Ok(false); }
    if !r.actors_at(r.depth).iter().any(|b| b.id == a.id && b.hp <= 0. && b.dead.value_or(false)) {
        return Ok(false);
    }
    r.actors_at_mut(r.depth).retain(|b| b.id != a.id);
    if r.collapse.as_ref().is_some() { return Ok(true); }
    let f = r.floor().ok_or("Administrator death floor missing")?;
    let origin = at(f, a).ok_or("Administrator death cell invalid")?;
    let depth = r.depth as u32;
    r.collapse = Arc::new(Collapse { boss_id: a.id, started_tick: r.tick,
        next_pulse_tick: r.tick + 2., escaped: false, waves: vec![] }).into();
    seed(r, depth, origin, current(r))?;
    let newly_reached = propagate_stairs(r)?;
    announce_start(r);
    settle_floor(r, depth)?;
    for reached in newly_reached { settle_floor(r, reached)?; }
    Ok(true)
}

pub fn announce_start(r: &mut Run) {
    crate::engine::log(r, "The Flesh Administrator dies. She falls into the opening flesh. Escape to the blue door!", "environment");
}

fn seed(r: &mut Run, depth: u32, cell: usize, with_fire: bool) -> Result<(), String> {
    if r.collapse.as_ref().unwrap().waves.iter().any(|w| w.depth == depth) { return Ok(()); }
    if !r.floors.iter().any(|f| f.depth == depth as f64) { crate::startup::ensure_floors(r, depth)?; }
    let f = r.floors.iter().find(|f| f.depth == depth as f64).ok_or("Collapse floor missing")?;
    if cell >= f.tiles.len() { return Err("Collapse stair seed is outside floor".into()); }
    let wave = CollapseFloor { depth, up: stair(f, "up"), upper_seeded: false,
        cells: vec![false; f.tiles.len()], frontier: vec![cell], diagonal_next: false,
        fire: with_fire.then(|| FireWave { origin: cell, down: stair(f,"down"), ..Default::default() }) };
    let c = crate::mutation_journal::make_mut(r.collapse.as_mut().unwrap());
    c.waves.push(wave);
    c.waves.last_mut().unwrap().cells[cell] = true;
    erode(r, depth, &[cell]);
    Ok(())
}

/// Stair seeds may connect immediately, but never expand a new spatial layer this pulse.
/// At most 40 iterations: each reached floor is inserted once and links only upward.
fn propagate_stairs(r: &mut Run) -> Result<Vec<u32>, String> {
    let mut reached = Vec::new();
    let mut index = 0;
    let window = crate::local_floor_rules::window(r);
    while index < r.collapse.as_ref().unwrap().waves.len() {
        let source = r.collapse.as_ref().unwrap().waves[index].depth as f64;
        let upper = crate::fast_mode::above(r, source);
        let c = crate::mutation_journal::make_mut(r.collapse.as_mut().unwrap());
        let wave = &mut c.waves[index];
        index += 1;
        if !window.contains(source) || !window.contains(upper) || wave.upper_seeded
            || wave.depth <= 1 || !wave.up.is_some_and(|i| wave.cells[i]) { continue; }
        wave.upper_seeded = true;
        let depth = upper as u32;
        let origin = wave.up.unwrap();
        let with_fire = wave.fire.is_some();
        if !r.floors.iter().any(|f| f.depth == depth as f64) { crate::startup::ensure_floors(r,depth)?; }
        let f = r.floors.iter().find(|f| f.depth == depth as f64).ok_or("Collapse upper floor missing")?;
        let cell = stair(f,"down").unwrap_or(origin);
        seed(r,depth,cell,with_fire)?;
        reached.push(depth);
    }
    Ok(reached)
}

/// Only the normal-turn scheduler calls this. Each pulse is O(sum(frontier+V+A+I) + L*N)
/// where L is removed actors and N is world actors (maintained-link cleanup), including
/// COW terrain copies and fixture/entity scans. Every cell
/// enters one frontier once. No recursion, RNG, or within-pulse expansion of new stair seeds.
pub fn world_turn(r: &mut Run) -> Result<(), String> {
    if !enabled(r) || r.status != "playing" || r.collapse.as_ref().is_none() { return Ok(()); }
    // A hidden-floor death can seed beneath the real player while its proxy has no player.
    if !crate::travel::flying(r,&r.player) && r.floor().and_then(|f| at(f,&r.player)).is_some_and(|i| collapsed(r,r.depth,i)) {
        fall_player(r)?;
    }
    let window = crate::local_floor_rules::window(r);
    while r.status == "playing" && r.collapse.as_ref().is_some_and(|c| r.tick >= c.next_pulse_tick) {
        let alternating = r.simulation_revision.value_or(29.) >= 52.;
        let mut changes = Vec::new();
        let c = crate::mutation_journal::make_mut(r.collapse.as_mut().unwrap());
        let fire_pulse = (c.next_pulse_tick - c.started_tick) % 4. == 0.;
        c.next_pulse_tick += 2.;
        for wave in &mut c.waves {
            if !window.contains(wave.depth as f64) { continue; }
            let f = r.floors.iter().find(|f| f.depth == wave.depth as f64).ok_or("Reached collapse floor missing")?;
            let w = f.width();
            let h = f.height();
            let next = expand(&mut wave.cells, &wave.frontier, w, h, !alternating || wave.diagonal_next);
            if alternating { wave.diagonal_next = !wave.diagonal_next; }
            wave.frontier = next.clone();
            changes.push((wave.depth,next));
        }
        for (depth,cells) in &changes { erode(r,*depth,cells); }
        let seeded = propagate_stairs(r)?;
        if fire_pulse { advance_fire(r)?; }
        for depth in changes.iter().map(|(d,_)| *d).chain(seeded) {
            settle_floor(r,depth)?;
        }
        if r.status != "playing" { break; }
    }
    Ok(())
}

/// Bounded-degree expansion shared by both waves. Stairs are graph cells even
/// when they are physically preserved, so neither wave stalls at a staircase.
fn expand(cells: &mut [bool], frontier: &[usize], w: usize, h: usize, diagonal: bool) -> Vec<usize> {
    let mut next = Vec::new();
    for &cell in frontier {
        let (x, y) = ((cell % w) as i32, (cell / w) as i32);
        for d in crate::movement::DIRECTIONS {
            if !diagonal && d.x != 0 && d.y != 0 { continue; }
            let (nx, ny) = (x + d.x, y + d.y);
            if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 { continue; }
            let i = ny as usize * w + nx as usize;
            if !cells[i] {
                cells[i] = true;
                next.push(i);
            }
        }
    }
    next
}

fn ignite_seed(fire: &mut FireWave, size: usize) -> Vec<usize> {
    fire.cells = vec![false; size];
    fire.cells[fire.origin] = true;
    fire.frontier = vec![fire.origin];
    fire.frontier.clone()
}

/// Fire starts behind the first collapse layer and expands once every four
/// turns. It follows the same route; upstairs fire starts only when the fire
/// reaches the lower stairs, not when collapse first reaches those stairs.
fn advance_fire(r: &mut Run) -> Result<(), String> {
    let mut changes = Vec::new();
    let window = crate::local_floor_rules::window(r);
    let upper_depths: Vec<_> = r.collapse.as_ref().unwrap().waves.iter()
        .map(|w| window.contains(w.depth as f64)
            .then(|| crate::fast_mode::above(r, w.depth as f64))
            .filter(|&d| window.contains(d)).map(|d| d as u32)).collect();
    let c = crate::mutation_journal::make_mut(r.collapse.as_mut().unwrap());
    for (index, wave) in c.waves.iter_mut().enumerate() {
        if !window.contains(wave.depth as f64) { continue; }
        let Some(fire) = &mut wave.fire else { continue };
        let f = r.floors.iter().find(|f| f.depth == wave.depth as f64).ok_or("Hellfire floor missing")?;
        let next = if fire.cells.is_empty() {
            if index != 0 { continue; }
            ignite_seed(fire, f.tiles.len())
        } else {
            let next = expand(&mut fire.cells, &fire.frontier, f.width(), f.height(), fire.diagonal_next);
            if !fire.frontier.is_empty() { fire.diagonal_next = !fire.diagonal_next; }
            fire.frontier = next.clone();
            next
        };
        if !next.is_empty() { changes.push((wave.depth, next)); }
    }
    // Waves are ordered from the boss floor upward. Seed any freshly reached
    // floors, but give each new seed its first spatial expansion next pulse.
    for (i, upper) in upper_depths.into_iter().enumerate() {
        let Some(upper) = upper else { continue };
        let (depth, up) = (c.waves[i].depth, c.waves[i].up);
        let Some(fire) = &mut c.waves[i].fire else { continue };
        if depth <= 1 || fire.upper_seeded || !up.is_some_and(|at| fire.cells.get(at) == Some(&true)) { continue; }
        fire.upper_seeded = true;
        let w = c.waves.iter_mut().find(|w| w.depth == upper).ok_or("Hellfire overtook collapse")?;
        let next = ignite_seed(w.fire.as_mut().ok_or("Hellfire upper wave missing")?, w.cells.len());
        changes.push((upper, next));
    }
    for (depth, cells) in changes {
        let f = crate::mutation_journal::make_mut(r.floors.iter_mut()
            .find(|f| f.depth == depth as f64).ok_or("Hellfire floor missing")?);
        let layer = f.hellfire.get_or_insert_with(|| vec![false; f.tiles.len()]);
        for i in cells { layer[i] = true; }
        f.dirty = true;
    }
    Ok(())
}

/// Permanently remove solid terrain and physical fixtures, never remembered knowledge.
fn erode(r: &mut Run, depth: u32, cells: &[usize]) {
    if cells.is_empty() { return; }
    let direct_roots = r.simulation_revision.value_or(29.) >= 51.;
    let wave = r.collapse.as_ref().unwrap().waves.iter().find(|w| w.depth == depth).unwrap();
    let preserved: Vec<_>;
    let cells = if wave.fire.is_some() {
        preserved = cells.iter().copied().filter(|&i| !wave.protected_stair(i)).collect();
        preserved.as_slice()
    } else { cells };
    if cells.is_empty() { return; }
    if r.depth == depth as f64 {
        let f = r.floor().unwrap();
        if stair(f, "up").is_some_and(|up| cells.contains(&up)) {
            crate::engine::log(r, if depth == 1 { "The collapse consumes the blue entrance. The way out is lost." }
                else { "The collapse consumes the upward staircase on this floor." }, "environment");
        }
    }
    let f = crate::mutation_journal::make_mut(r.floors.iter_mut().find(|f| f.depth == depth as f64).unwrap());
    let w = f.width();
    let changed: HashSet<_> = cells.iter().copied().collect();
    for &i in cells {
        f.tiles[i] = "chasm".into();
        for layer in [&mut f.ichor,&mut f.slime,&mut f.mud,&mut f.roots,&mut f.razorleaves,&mut f.currents,&mut f.water_blood] {
            if let Some(v) = layer.as_mut() { v[i] = 0.; }
        }
        for layer in [&mut f.blood,&mut f.snow,&mut f.ice,&mut f.puddles,&mut f.hellfire] { if let Some(v) = layer.as_mut() { v[i] = false; } }
        if let Some(v) = f.doors.as_mut() { v.shift_remove(&i.to_string()); }
        if let Some(v) = f.bulwarks.as_mut() { v.shift_remove(&i.to_string()); }
        if let Some(v) = f.altered_chasms.as_mut() { v.shift_remove(&i.to_string()); }
        if let Some(e) = f.environment.as_mut() {
            e.fire[i] = 0.;
            if let Some(v) = e.fire_kind.as_mut() { v[i] = None; }
            if let Some(v) = e.fire_fresh.as_mut() { v[i] = false; }
            if let Some(v) = e.fire_reaction.as_mut() { v[i] = 0.; }
            if let Some(p) = e.provenance.as_mut().and_then(|m| m.get_mut("fire")) { p.owner[i]=-1.; p.player[i]=0.; }
        }
    }
    for v in [&mut f.grates,&mut f.permanent_fire] { if let Some(v) = v.as_mut() { v.retain(|i| !changed.contains(&(*i as usize))); } }
    if let Some(v) = f.vents.as_mut() { v.retain(|v| !changed.contains(&(v.at as usize))); }
    if let Some(v) = f.light_sources.as_mut() { v.retain(|v| !changed.contains(&(v.y as usize*w+v.x as usize))); }
    for root in f.fungal_roots.as_mut().into_iter().flatten() {
        if root.path.iter().any(|i| changed.contains(&(*i as usize))) { root.active = false; }
    }
    if direct_roots { crate::fungi::clear_orphaned_root_cells(f); }
    f.dirty = true;
}

/// Existing flight semantics apply, including equipment, Ascend and Phase Through.
/// A collapse chasm is bottomless: loose objects/corpses and non-flying actors leave play.
fn settle_floor(r: &mut Run, depth: u32) -> Result<(), String> {
    let f = r.floors.iter().find(|f| f.depth == depth as f64).unwrap();
    let w = f.width();
    let lost: Vec<_> = r.actors_at(depth as f64).iter().filter(|a| {
        at(f,a).is_some_and(|i| collapsed(r,depth as f64,i)) && (a.dead.value_or(false) || !crate::travel::flying(r,a))
    }).map(|a| a.id).collect();
    let lose_player = r.depth == depth as f64 && at(f,&r.player).is_some_and(|i| collapsed(r,r.depth,i)) && !crate::travel::flying(r,&r.player);
    for id in &lost { crate::maintained_links::end_maintained_links_to_dead_target(r,*id); }
    if lost.iter().any(|id| r.actors_at(depth as f64).iter().any(|a| a.id == *id && crate::half_here::is_clone(a))) { crate::half_here::dismiss(r); }
    let lost: HashSet<_> = lost.iter().map(|i| i.to_bits()).collect();
    r.actors_at_mut(depth as f64).retain(|a| !lost.contains(&a.id.to_bits()));
    let collapse = Arc::clone(r.collapse.as_ref().unwrap());
    let wave = collapse.waves.iter().find(|c| c.depth == depth).unwrap();
    let dream_shards_only = crate::phantasmii_blessing_rules::dream_shards_only(r);
    r.ground_at_mut(depth as f64).retain(|g| crate::dreamwalking::protected_ground(&g.item, dream_shards_only) || !wave.gap(g.y as usize*w+g.x as usize));
    if lose_player { fall_player(r)?; }
    Ok(())
}

/// Settle a cohort that is actually falling, including flight expiry off screen.
/// False means the ordinary chasm/pit semantics still apply to that candidate.
pub fn intercept_actor_falls(r: &mut Run, depth: f64, candidates: &[f64]) -> Result<(), String> {
    if r.collapse.as_ref().is_none() { return Ok(()); }
    let Some(f) = r.floors.iter().find(|f| f.depth == depth) else { return Ok(()); };
    let lost: Vec<_> = r.actors_at(depth).iter().filter(|a| candidates.contains(&a.id)
        && !crate::travel::flying(r,a) && at(f,a).is_some_and(|i| collapsed(r,depth,i)))
        .map(|a| a.id).collect();
    for id in &lost { crate::maintained_links::end_maintained_links_to_dead_target(r,*id); }
    if lost.iter().any(|id| r.actors_at(depth).iter().any(|a| a.id == *id && crate::half_here::is_clone(a))) { crate::half_here::dismiss(r); }
    r.actors_at_mut(depth).retain(|a| !lost.contains(&a.id));
    Ok(())
}

pub fn fall_player(r: &mut Run) -> Result<(), String> {
    if r.status != "playing" { return Ok(()); }
    let p = r.player.clone();
    crate::resources::record_loss(r,&p,"hp",p.hp.max(0.));
    r.player.hp = 0.;
    crate::engine::unpreventable_death(r,r.player.id,"flesh-collapse").map_err(|e| e.to_string())?;
    crate::engine::log(r,FALL_MESSAGE,"environment");
    Ok(())
}
/// True only for an actual fall attempt at a collapse-created gap, not hovering above it.
pub fn intercept_fall(r: &mut Run) -> Result<bool, String> {
    let yes = r.floor().and_then(|f| at(f,&r.player)).is_some_and(|i| collapsed(r,r.depth,i));
    if yes { fall_player(r)?; }
    Ok(yes)
}
pub fn try_escape(r: &mut Run) -> bool {
    if !enabled(r) || r.status != "playing" || r.depth != 1. || r.collapse.as_ref().is_none() { return false; }
    let Some(f) = r.floor() else { return false; };
    if at(f,&r.player) != crate::entrance::cell(f) { return false; }
    crate::mutation_journal::make_mut(r.collapse.as_mut().unwrap()).escaped = true;
    r.status = "won".into();
    crate::engine::log(r,WIN_MESSAGE,"player");
    true
}

pub fn validate(r: &Run) -> Result<(), String> {
    validate_policy(r)?;
    let bosses: Vec<_> = r.actors.values().flat_map(|v| v.iter()).filter(|a| is_boss(a)).collect();
    if bosses.len() > 1 || (!bosses.is_empty() && !enabled(r))
        || (r.collapse.as_ref().is_some() && !bosses.is_empty()) {
        return Err("Invalid Administrator identity or defeat state".into());
    }
    let Some(c) = r.collapse.as_ref() else { return if r.collapse.is_missing() { Ok(()) } else { Err("Invalid collapse state".into()) }; };
    if !enabled(r) || !c.boss_id.is_finite() || c.boss_id < 0. || c.boss_id.fract()!=0.
        || !c.started_tick.is_finite() || c.started_tick < 0. || c.started_tick.fract()!=0.
        || !c.next_pulse_tick.is_finite() || c.next_pulse_tick<c.started_tick+2.
        || (c.next_pulse_tick-c.started_tick)%2.!=0. || c.waves.is_empty() || c.waves.len()>40
        || (c.escaped && r.status!="won") { return Err("Invalid collapse clocks or ending".into()); }
    let mut depths = HashSet::new();
    for wave in &c.waves {
        let f = r.floors.iter().find(|f| f.depth==wave.depth as f64).ok_or("Collapse floor missing")?;
        let n = f.tiles.len();
        if !(1..=40).contains(&wave.depth) || !depths.insert(wave.depth) || wave.cells.len()!=n
            || wave.up.is_some_and(|i| i>=n) || wave.frontier.len()>n
            || wave.frontier.iter().any(|i| *i>=n || !wave.cells[*i])
            || wave.frontier.iter().copied().collect::<HashSet<_>>().len()!=wave.frontier.len()
            || wave.cells.iter().enumerate().any(|(i,b)| *b && !wave.protected_stair(i) && f.tiles[i]!="chasm") {
            return Err("Invalid collapse frontier or terrain".into());
        }
    }
    Ok(())
}
