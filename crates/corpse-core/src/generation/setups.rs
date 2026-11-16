//! Finite authored stencil packing, preserving selection order and suffix repacks.
use super::*;
use crate::model::*;
use indexmap::{IndexMap, IndexSet};
use serde::Deserialize;
use std::sync::OnceLock;
#[derive(Debug, Clone, Deserialize)]
pub struct SetupDefinition {
    pub id: u32,
    pub kinds: Vec<String>,
    pub biomes: Vec<String>,
    pub eligible: Vec<bool>,
    pub introduced: Vec<bool>,
}
pub fn setup_definitions() -> &'static [SetupDefinition] {
    static C: OnceLock<Vec<SetupDefinition>> = OnceLock::new();
    C.get_or_init(|| {
        serde_json::from_str(include_str!("../../data/setup-catalog-v48.json")).unwrap()
    })
}
pub fn setup_eligible_at_depth(d: &SetupDefinition, depth: u32) -> Result<bool, String> {
    if !(1..=40).contains(&depth) {
        return Err("Setup eligibility supports depths1..40".into());
    }
    Ok(d.eligible[depth as usize - 1])
}
pub fn setup_eligible_at_depth_revision(d: &SetupDefinition, depth: u32, revision: f64) -> Result<bool,String> {
    if !(1..=40).contains(&depth) { return Err("Setup eligibility supports depths1..40".into()); }
    match revision {
        29.0 | 30.0 | 31.0 => setup_eligible_at_depth(d,depth),
        32.0 | 33.0 | 34.0 | 35.0 | 36.0 | 37.0 | 38.0 | 39.0 | 40.0 | 41.0 | 42.0 | 43.0 | 44.0 | 45.0 | 46.0 | 50.0 | 51.0 | 52.0 => Ok(crate::flavor_placement::setup_eligible(d,depth)),
        _ => Err("Unsupported setup revision".into()),
    }
}
pub fn generate_setups(f: &mut Floor, r: &mut dyn FnMut() -> f64) -> Result<(), String> {
    generate_setups_revision(f,r,29.0)
}
pub fn generate_setups_revision(f: &mut Floor, r: &mut dyn FnMut() -> f64, revision: f64) -> Result<(), String> {
    if ![29.0,30.0,31.0,32.0,33.0,34.0,35.0,36.0,37.0,38.0,39.0,40.0,41.0,42.0,43.0,44.0,45.0,46.0,50.0,51.0,52.0].contains(&revision) { return Err("Unsupported setup revision".into()); }
    if !(1.0..=40.0).contains(&f.depth) || f.depth.fract() != 0. {
        return Err("Unsupported setup depth".into());
    }
    let pool: Vec<_> = setup_definitions()
        .iter()
        .filter(|d| d.biomes.contains(&f.domain) && setup_eligible_at_depth_revision(d,f.depth as u32,revision).unwrap_or(false))
        .filter(|d| !(if revision>=32.0 { crate::flavor_placement::setup_introduced(d,f.depth as u32) } else { d.introduced[f.depth as usize-1] }) || r() < 0.2)
        .cloned()
        .map(|mut d| {
            if revision>=32.0 && d.id==108 {
                d.kinds=vec![crate::flavor_placement::water_watcher_kind(f.depth as u32,
                    f.enemy_families.as_ref().map(Vec::as_slice))?];
            }
            Ok(d)
        })
        .collect::<Result<Vec<_>,String>>()?;
    place_setups_revision(f, &pool, r, revision)
}
fn layer(a: &Field<Vec<f64>>, i: usize) -> f64 {
    a.as_ref().map(|a| a[i]).unwrap_or(0.)
}
fn add(points: &mut IndexSet<usize>, x: i32, y: i32) {
    if (0..13).contains(&x) && (0..13).contains(&y) {
        points.insert((y * 13 + x) as usize);
    }
}
fn jsround(n: f64) -> i32 {
    let f = n.floor();
    (if n - f < 0.5 { f } else { f + 1. }) as i32
}
pub fn setup_interaction_cells(p: &Floor, id: u32) -> Vec<usize> {
    let hub = if id == 1 || id == 2 { 4 } else { 6 };
    let mut points = IndexSet::new();
    let mut anchors = IndexSet::from([6 * 13 + hub as usize]);
    for i in 0..169 {
        if p.tiles[i] != "floor"
            || p.lights[i] != 0.
            || layer(&p.slime, i) != 0.
            || layer(&p.mud, i) != 0.
        {
            points.insert(i);
        }
    }
    let mut doors: Vec<usize> = p
        .doors
        .as_ref()
        .into_iter()
        .flat_map(|a| a.keys())
        .map(|k| k.parse().unwrap())
        .collect();
    doors.sort_unstable();
    for i in p
        .grates
        .as_ref()
        .into_iter()
        .flatten()
        .chain(p.permanent_fire.as_ref().into_iter().flatten())
        .map(|&i| i as usize)
        .chain(doors)
        .chain(
            p.vents
                .as_ref()
                .into_iter()
                .flatten()
                .map(|v| v.at as usize),
        )
    {
        points.insert(i);
        anchors.insert(i);
    }
    for i in p
        .setup_spawns
        .as_ref()
        .into_iter()
        .flatten()
        .map(|a| a.y as usize * 13 + a.x as usize)
        .chain(
            p.light_sources
                .as_ref()
                .into_iter()
                .flatten()
                .map(|a| a.y as usize * 13 + a.x as usize),
        )
    {
        points.insert(i);
        anchors.insert(i);
    }
    if id == 24 {
        anchors.insert(6 * 13 + 4);
        anchors.insert(6 * 13 + 8);
    }
    if id == 108 {
        for y in 4..=7 {
            for x in 2..=7 {
                add(&mut points, x, y)
            }
        }
    }
    if id == 36 {
        for y in 2..=8 {
            for x in 5..=7 {
                add(&mut points, x, y)
            }
        }
    }
    for end in anchors {
        let dx = (end % 13) as i32 - hub;
        let dy = (end / 13) as i32 - 6;
        let steps = dx.abs().max(dy.abs());
        for step in 0..=steps {
            add(
                &mut points,
                hub + jsround(dx as f64 * step as f64 / steps.max(1) as f64),
                6 + jsround(dy as f64 * step as f64 / steps.max(1) as f64),
            );
        }
    }
    let mut cells = points.clone();
    for i in points {
        let x = (i % 13) as i32;
        let y = (i / 13) as i32;
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            add(&mut cells, x + dx, y + dy)
        }
        if p.tiles[i] == "wall" {
            for dx in [-1, 1] {
                for dy in [-1, 1] {
                    add(&mut cells, x + dx, y + dy)
                }
            }
        }
    }
    let mut out: Vec<_> = cells.into_iter().collect();
    out.sort_unstable();
    out
}
struct Stencil {
    definition: SetupDefinition,
    floor: Floor,
    left: i32,
    right: i32,
    top: i32,
    bottom: i32,
    cells: Vec<usize>,
    fires: Vec<usize>,
    fuel: Vec<usize>,
    aprons: Vec<usize>,
    vents: Vec<usize>,
    emitters: Vec<(usize, i32)>,
    actors: usize,
    blessings: i32,
    rating: Option<String>,
    weight: f64,
    banshees: i32,
}
fn stencil(f: &Floor, d: &SetupDefinition) -> Result<Stencil, String> {
    let mut p = Floor {
        depth: f.depth,
        domain: f.domain.clone(),
        enemy_families: f.enemy_families.clone(),
        tiles: vec!["floor".into(); 169],
        lights: vec![0.; 169],
        seen: vec![false; 169],
        ..Default::default()
    };
    stamp_setup(&mut p, d.id, 6, 6, &d.kinds)?;
    let cells = setup_interaction_cells(&p, d.id);
    let left = cells.iter().map(|i| i % 13).min().unwrap() as i32;
    let right = cells.iter().map(|i| i % 13).max().unwrap() as i32;
    let top = cells.iter().map(|i| i / 13).min().unwrap() as i32;
    let bottom = cells.iter().map(|i| i / 13).max().unwrap() as i32;
    let fires: IndexSet<_> = p
        .permanent_fire
        .as_ref()
        .into_iter()
        .flatten()
        .map(|&i| i as usize)
        .chain(
            p.light_sources
                .as_ref()
                .into_iter()
                .flatten()
                .filter(|s| s.kind.as_ref().is_some_and(|s| s == "brazier"))
                .map(|s| s.y as usize * 13 + s.x as usize),
        )
        .collect();
    let kinds: Vec<_> = p
        .setup_spawns
        .as_ref()
        .into_iter()
        .flatten()
        .map(|s| s.kind.clone())
        .collect();
    Ok(Stencil {
        definition: d.clone(),
        left,
        right,
        top,
        bottom,
        fires: fires.into_iter().collect(),
        fuel: cells
            .iter()
            .copied()
            .filter(|&i| layer(&p.slime, i) > 0. || p.tiles[i] == "grass" || p.tiles[i] == "tree")
            .collect(),
        aprons: furnace_origins(d.id as i32, 6, 6, 13)
            .into_iter()
            .map(|i| i as usize)
            .collect(),
        vents: p
            .vents
            .as_ref()
            .into_iter()
            .flatten()
            .map(|v| v.at as usize)
            .collect(),
        emitters: p
            .setup_spawns
            .as_ref()
            .into_iter()
            .flatten()
            .filter_map(|s| {
                let r = initial_fire_separation_radius(&s.kind);
                (r > 0).then_some((s.y as usize * 13 + s.x as usize, r))
            })
            .collect(),
        actors: kinds.len(),
        blessings: crate::population::heart_blessings(&kinds)?,
        rating: crate::population::setup_encounter_rating(f.depth, &kinds)?,
        weight: fire_setup_weight(f, d.id as i32),
        banshees: kinds.iter().filter(|k| *k == "banshee").count() as i32,
        floor: p,
        cells,
    })
}
#[derive(Clone, Copy)]
struct Placement {
    t: usize,
    x: i32,
    y: i32,
}
#[derive(Clone)]
struct Packing {
    revision: f64,
    reserved: Vec<bool>,
    fire: Vec<bool>,
    fuel: Vec<bool>,
    furnace_fuel: Vec<bool>,
    fire_emitter: Vec<bool>,
    emitter_fire: IndexMap<i32, Vec<bool>>,
    planned_fires: IndexSet<usize>,
    placements: Vec<Placement>,
    remaining: i32,
    banshees: i32,
    used: IndexSet<u32>,
}
fn mark(mask: &mut [bool], at: usize, w: usize, h: usize, r: i32, r2: i32) {
    let x = (at % w) as i32;
    let y = (at / w) as i32;
    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy <= r2
                && x + dx >= 0
                && x + dx < w as i32
                && y + dy >= 0
                && y + dy < h as i32
            {
                mask[(y + dy) as usize * w + (x + dx) as usize] = true;
            }
        }
    }
}
impl Packing {
    fn mark_fire(&mut self, at: usize, w: usize, h: usize) {
        mark(&mut self.fire, at, w, h, 2, 5);
        for (&r, mask) in &mut self.emitter_fire {
            mark(mask, at, w, h, r, r * r)
        }
    }
    fn fits(&self, t: &Stencil, depth: u32) -> bool {
        t.blessings <= self.remaining
            && self.banshees + t.banshees <= 1
            && crate::population::ordinary_horde_budget_feasible_revision(
                depth,
                self.remaining - t.blessings + 2,
                false,
                self.banshees + t.banshees,
                if self.revision >= 38.0 { self.revision } else { 29.0 },
            )
    }
    fn reserve(&mut self, t: &Stencil, p: Placement, w: usize, h: usize) {
        self.placements.push(p);
        self.remaining -= t.blessings;
        self.banshees += t.banshees;
        self.used.insert(t.definition.id);
        let map = |i: usize| {
            ((p.y + (i / 13) as i32 - t.top) * w as i32 + p.x + (i % 13) as i32 - t.left) as usize
        };
        for &i in &t.cells {
            self.reserved[map(i)] = true
        }
        for &i in &t.fires {
            self.mark_fire(map(i), w, h);
            self.planned_fires.insert(map(i));
        }
        for &(i, r) in &t.emitters {
            mark(&mut self.fire_emitter, map(i), w, h, r, r * r)
        }
        for &i in &t.aprons {
            mark(&mut self.fire, map(i), w, h, 6, 36)
        }
        for &i in &t.fuel {
            mark(&mut self.fuel, map(i), w, h, 2, 5);
            mark(&mut self.furnace_fuel, map(i), w, h, 6, 36)
        }
    }
}
fn position(
    f: &Floor,
    t: &Stencil,
    s: &Packing,
    flip_x: bool,
    flip_y: bool,
    cost: &[f64],
    gas: &GenerationGasComponents,
    core: &IndexSet<usize>,
) -> Option<(i32, i32)> {
    let w = f.width() as i32;
    let h = f.height() as i32;
    let width = t.right - t.left + 1;
    let height = t.bottom - t.top + 1;
    let reverse_x = flip_x != (s.placements.len() % 2 == 1);
    let reverse_y = flip_y != [1, 2].contains(&(s.placements.len() % 4));
    let off = |i: usize| ((i / 13) as i32 - t.top) * w + (i % 13) as i32 - t.left;
    let mut best = None;
    let mut best_cost = f64::INFINITY;
    for yy in 1..=h - 1 - height {
        for xx in 1..=w - 1 - width {
            let x = if reverse_x { w - width - xx } else { xx };
            let y = if reverse_y { h - height - yy } else { yy };
            let map = |i: usize| (y * w + x + off(i)) as usize;
            let mut c = 0.;
            let mut clear = true;
            for &i in &t.cells {
                let at = map(i);
                if s.reserved[at] {
                    clear = false;
                    break;
                }
                c += cost[at];
            }
            if clear && t.fuel.iter().any(|&i| s.fire[map(i)]) {
                clear = false
            }
            if clear
                && t.fires
                    .iter()
                    .any(|&i| s.fuel[map(i)] || s.fire_emitter[map(i)])
            {
                clear = false
            }
            if clear && t.emitters.iter().any(|&(i, r)| s.emitter_fire[&r][map(i)]) {
                clear = false
            }
            if clear && t.aprons.iter().any(|&i| s.furnace_fuel[map(i)]) {
                clear = false
            }
            if clear
                && is_furnace_setup(t.definition.id as i32)
                && furnace_containment_cells(
                    f,
                    t.definition.id as i32,
                    x - t.left + 6,
                    y - t.top + 6,
                    gas,
                    core,
                )
                .is_none()
            {
                clear = false
            }
            if clear
                && !is_furnace_setup(t.definition.id as i32)
                && t.vents.iter().any(|&i| {
                    s.planned_fires.iter().any(|&fire| {
                        gas_source_shares_combustible_flame_component(map(i), fire, gas)
                    })
                })
            {
                clear = false
            }
            if clear
                && t.floor
                    .setup_spawns
                    .as_ref()
                    .into_iter()
                    .flatten()
                    .filter(|a| initial_gas_emitter(&a.kind))
                    .any(|a| {
                        s.planned_fires.iter().any(|&fire| {
                            gas_source_shares_combustible_flame_component(
                                map(a.y as usize * 13 + a.x as usize),
                                fire,
                                gas,
                            )
                        })
                    })
            {
                clear = false
            }
            if clear {
                if f.generation
                    .as_ref()
                    .and_then(|g| g.landmarks.as_ref())
                    .is_none()
                    || c == 0.
                {
                    return Some((x, y));
                }
                if c < best_cost {
                    best_cost = c;
                    best = Some((x, y));
                }
            }
        }
    }
    best
}
/// O(S²*T*V*A), S<=20 and A<=169, with O(V+T*A) working memory.
pub fn place_setups(
    f: &mut Floor,
    eligible: &[SetupDefinition],
    r: &mut dyn FnMut() -> f64,
) -> Result<(), String> {
    place_setups_revision(f, eligible, r, 29.0)
}

fn place_setups_revision(
    f: &mut Floor, eligible: &[SetupDefinition], r: &mut dyn FnMut() -> f64, revision: f64,
) -> Result<(), String> {
    let w = f.width();
    let h = f.height();
    let n = f.tiles.len();
    let (min, max) = setup_count_range(f.depth);
    let target = min + (r() * (max - min + 1) as f64).floor() as usize;
    let templates: Vec<_> = eligible
        .iter()
        .map(|d| stencil(f, d))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|t| t.actors == 0 || f.depth >= 2. || t.rating.is_some())
        .collect();
    let mut reserved = vec![false; n];
    let up = f
        .tiles
        .iter()
        .position(|t| t == "up")
        .map(|i| (i % w, i / w))
        .unwrap_or((5, 5));
    let down = f
        .tiles
        .iter()
        .position(|t| t == "down")
        .or_else(|| f.tiles.iter().position(|t| t == "pit"))
        .map(|i| (i % w, i / w))
        .unwrap_or((w - 6, h - 6));
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let xx = x as f64;
            let yy = y as f64;
            if (xx - up.0 as f64).hypot(yy - up.1 as f64) <= 5.
                || (xx - down.0 as f64).hypot(yy - down.1 as f64) <= 3.
                || f.godbeast_event.as_ref().is_some_and(|e| {
                    (xx >= e.room.left - 1.
                        && xx <= e.room.right + 1.
                        && yy >= e.room.top - 1.
                        && yy <= e.room.bottom + 1.)
                        || e.tap
                            .as_ref()
                            .is_some_and(|p| (xx - p.x).abs() <= 1. && (yy - p.y).abs() <= 1.)
                })
            {
                reserved[y * w + x] = true;
            }
        }
    }
    let landmarks = f.generation.as_ref().and_then(|g| g.landmarks.as_ref());
    if let Some(rooms) = f.dungeon_rooms.as_ref() {
        if landmarks.is_none() {
            let cells: IndexSet<_> = rooms
                .iter()
                .flat_map(|r| r.cells.iter().map(|&i| i as usize))
                .collect();
            for i in 0..n {
                if !cells.contains(&i) || ["wall", "tree", "chasm"].contains(&f.tiles[i].as_str()) {
                    reserved[i] = true;
                }
            }
        }
    }
    if landmarks.is_some() {
        let connectors = f
            .room_graph
            .as_ref()
            .map(|g| &g.connectors)
            .or_else(|| f.dungeon_connectors.as_ref());
        for c in connectors.into_iter().flatten() {
            for &i in &c.cells {
                let i = i as usize;
                if !["floor", "grass"].contains(&f.tiles[i].as_str()) {
                    continue;
                }
                let degree = [
                    i as isize - 1,
                    i as isize + 1,
                    i as isize - w as isize,
                    i as isize + w as isize,
                ]
                .into_iter()
                .filter(|&j| {
                    j < 0
                        || j >= n as isize
                        || !["wall", "tree", "chasm"].contains(&f.tiles[j as usize].as_str())
                })
                .count();
                if degree <= 2 {
                    reserved[i] = true;
                }
            }
        }
    }
    if landmarks.is_some() || f.dungeon_rooms.as_ref().is_some() {
        for k in f.doors.as_ref().into_iter().flat_map(|d| d.keys()) {
            let i: usize = k.parse().map_err(|_| "Invalid door index")?;
            let x = i % w;
            let y = i / w;
            for yy in y.saturating_sub(1).max(1)..=(y + 1).min(h - 2) {
                for xx in x.saturating_sub(1).max(1)..=(x + 1).min(w - 2) {
                    reserved[yy * w + xx] = true;
                }
            }
        }
    }
    let mut cost = vec![0.; n];
    for l in landmarks.into_iter().flatten() {
        for &at in &l.cells {
            cost[at as usize] = if ["river", "lake", "fissure"].contains(&l.kind.as_str()) {
                24.
            } else if l.scale == "major" {
                5.
            } else {
                2.
            };
        }
    }
    for i in 0..n {
        cost[i] += if ["water", "shallow-water", "chasm"].contains(&f.tiles[i].as_str()) {
            12.
        } else if f.tiles[i] == "wall" {
            3.
        } else {
            0.
        };
    }
    for at in f
        .vents
        .as_ref()
        .into_iter()
        .flatten()
        .map(|v| v.at as usize)
        .chain(
            f.grates
                .as_ref()
                .into_iter()
                .flatten()
                .chain(f.permanent_fire.as_ref().into_iter().flatten())
                .map(|&i| i as usize),
        )
        .chain(
            f.light_sources
                .as_ref()
                .into_iter()
                .flatten()
                .map(|s| s.y as usize * w + s.x as usize),
        )
    {
        reserved[at] = true;
    }
    let core: IndexSet<usize> = f
        .generation
        .as_ref()
        .and_then(|g| g.terrain_spine.as_ref())
        .into_iter()
        .flatten()
        .map(|&i| i as usize)
        .collect();
    for &i in &core {
        reserved[i] = true
    }
    let flip_x = r() < 0.5;
    let flip_y = r() < 0.5;
    let mut s = Packing {
        revision,
        reserved,
        fire: vec![false; n],
        fuel: vec![false; n],
        furnace_fuel: vec![false; n],
        fire_emitter: vec![false; n],
        emitter_fire: IndexMap::new(),
        planned_fires: f
            .permanent_fire
            .as_ref()
            .into_iter()
            .flatten()
            .map(|&i| i as usize)
            .collect(),
        placements: vec![],
        remaining: (f
            .enemy_blessing_budget
            .as_ref()
            .copied()
            .unwrap_or(f.depth + 6.)
            - 2.) as i32,
        banshees: f
            .setup_spawns
            .as_ref()
            .into_iter()
            .flatten()
            .filter(|a| a.kind == "banshee")
            .count() as i32,
        used: IndexSet::new(),
    };
    for radius in templates
        .iter()
        .flat_map(|t| t.emitters.iter().map(|e| e.1))
        .chain(
            f.setup_spawns
                .as_ref()
                .into_iter()
                .flatten()
                .map(|a| initial_fire_separation_radius(&a.kind))
                .filter(|&r| r > 0),
        )
    {
        s.emitter_fire
            .entry(radius)
            .or_insert_with(|| vec![false; n]);
    }
    for at in f
        .permanent_fire
        .as_ref()
        .into_iter()
        .flatten()
        .map(|&i| i as usize)
        .chain(
            f.light_sources
                .as_ref()
                .into_iter()
                .flatten()
                .filter(|s| s.kind.as_ref().is_some_and(|k| k == "brazier"))
                .map(|s| s.y as usize * w + s.x as usize),
        )
    {
        s.mark_fire(at, w, h)
    }
    for a in f.setup_spawns.as_ref().into_iter().flatten() {
        let r = initial_fire_separation_radius(&a.kind);
        if r > 0 {
            mark(
                &mut s.fire_emitter,
                a.y as usize * w + a.x as usize,
                w,
                h,
                r,
                r * r,
            )
        }
    }
    for i in 0..n {
        if layer(&f.slime, i) != 0. {
            mark(&mut s.fuel, i, w, h, 2, 5);
            mark(&mut s.furnace_fuel, i, w, h, 6, 36)
        }
    }
    let initial = s.clone();
    let gas = build_generation_gas_components(f);
    for _ in 0..target {
        let choices: Vec<usize> = templates
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                s.fits(t, f.depth as u32) && !(t.definition.id == 67 && s.used.contains(&67))
            })
            .map(|(i, _)| i)
            .collect();
        let fresh: Vec<_> = choices
            .iter()
            .copied()
            .filter(|&i| !s.used.contains(&templates[i].definition.id))
            .collect();
        let mut order = if fresh.is_empty() {
            choices.clone()
        } else {
            fresh
        };
        let mut placed = false;
        while !order.is_empty() {
            let mut roll = r() * order.iter().map(|&i| templates[i].weight).sum::<f64>();
            let mut choice = 0;
            while choice < order.len() - 1 {
                roll -= templates[order[choice]].weight;
                if roll < 0. {
                    break;
                }
                choice += 1;
            }
            let t = order.remove(choice);
            if let Some((x, y)) = position(f, &templates[t], &s, flip_x, flip_y, &cost, &gas, &core)
            {
                s.reserve(&templates[t], Placement { t, x, y }, w, h);
                placed = true;
                break;
            }
        }
        if !placed {
            let mut sorted = choices;
            sorted.sort_by_key(|&i| templates[i].cells.len());
            for t in sorted {
                if let Some((x, y)) =
                    position(f, &templates[t], &s, flip_x, flip_y, &cost, &gas, &core)
                {
                    s.reserve(&templates[t], Placement { t, x, y }, w, h);
                    placed = true;
                    break;
                }
            }
        }
        if !placed {
            let previous = s.placements.clone();
            let mut compact: Vec<usize> = (0..templates.len()).collect();
            compact.sort_by(|&a, &b| {
                (templates[a].cells.len() as f64 / templates[a].weight)
                    .total_cmp(&(templates[b].cells.len() as f64 / templates[b].weight))
            });
            let mut packed = false;
            for keep in (0..=previous.len()).rev() {
                s = initial.clone();
                for &p in &previous[..keep] {
                    s.reserve(&templates[p.t], p, w, h)
                }
                for _ in keep..target {
                    let mut next = None;
                    for &t in &compact {
                        if !s.fits(&templates[t], f.depth as u32)
                            || (templates[t].definition.id == 67 && s.used.contains(&67))
                        {
                            continue;
                        }
                        if let Some((x, y)) =
                            position(f, &templates[t], &s, flip_x, flip_y, &cost, &gas, &core)
                        {
                            next = Some(Placement { t, x, y });
                            break;
                        }
                    }
                    let Some(p) = next else { break };
                    s.reserve(&templates[p.t], p, w, h);
                }
                if s.placements.len() == target {
                    packed = true;
                    break;
                }
            }
            if !packed {
                return Err(
                    "Floor has insufficient space for approved environmental setups".into(),
                );
            }
            break;
        }
    }
    for placement in s.placements {
        let t = &templates[placement.t];
        let p = &t.floor;
        let sx = placement.x - t.left;
        let sy = placement.y - t.top;
        let map = |i: usize| (((i / 13) as i32 + sy) * w as i32 + (i % 13) as i32 + sx) as usize;
        for &old in &t.cells {
            let at = map(old);
            f.tiles[at] = p.tiles[old].clone();
            if p.lights[old] != 0. {
                f.lights[at] = p.lights[old]
            }
            for (a, b) in [(&mut f.slime, &p.slime), (&mut f.mud, &p.mud)] {
                if layer(b, old) != 0. {
                    a.get_or_insert_with(|| vec![0.; n])[at] = layer(b, old);
                }
            }
        }
        for (a, b) in [
            (&mut f.grates, &p.grates),
            (&mut f.permanent_fire, &p.permanent_fire),
        ] {
            if let Some(b) = b.as_ref().filter(|b| !b.is_empty()) {
                a.get_or_insert_with(Vec::new)
                    .extend(b.iter().map(|&i| map(i as usize) as f64));
            }
        }
        let mut doors: Vec<_> = p
            .doors
            .as_ref()
            .into_iter()
            .flat_map(|d| d.iter())
            .collect();
        doors.sort_by_key(|(k, _)| k.parse::<usize>().unwrap());
        for (k, v) in doors {
            f.doors
                .get_or_insert_with(Default::default)
                .insert(map(k.parse().unwrap()).to_string(), v.clone());
        }
        for v in p.vents.as_ref().into_iter().flatten() {
            let mut v = v.clone();
            v.at = map(v.at as usize) as f64;
            if t.definition.id == 8 {
                v.gas = GENERATED_VENT_GASES[(12 + sx + sy) as usize % 4].into()
            }
            f.vents.get_or_insert_with(Vec::new).push(v);
        }
        let setup_index = f.setups.as_ref().map(Vec::len).unwrap_or(0);
        for a in p.setup_spawns.as_ref().into_iter().flatten() {
            let mut a = a.clone();
            a.x += sx as f64;
            a.y += sy as f64;
            a.setup_index = Field::Value(setup_index as f64);
            f.setup_spawns.get_or_insert_with(Vec::new).push(a);
        }
        for a in p.light_sources.as_ref().into_iter().flatten() {
            let mut a = a.clone();
            a.x += sx as f64;
            a.y += sy as f64;
            f.light_sources.get_or_insert_with(Vec::new).push(a);
        }
        f.setups
            .get_or_insert_with(Vec::new)
            .push(FloorSetupsEntry {
                id: t.definition.id as f64,
                x: (6 + sx) as f64,
                y: (6 + sy) as f64,
                footprint: Field::Value(t.cells.iter().map(|&i| i as f64).collect()),
                bounds: Field::Value(SetupBounds {
                    left: placement.x as f64,
                    right: (placement.x + t.right - t.left) as f64,
                    top: placement.y as f64,
                    bottom: (placement.y + t.bottom - t.top) as f64,
                }),
            });
    }
    Ok(())
}
