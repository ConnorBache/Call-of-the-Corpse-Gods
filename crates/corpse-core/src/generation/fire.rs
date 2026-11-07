//! Generation-only fuel containment. Never invoked by ticks, loading or temporal restore.
use super::{fire_cells, generate_substrates, generation_creature};
use crate::{Field, Floor};
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
fn layer(layer: &Field<Vec<f64>>, at: usize) -> f64 {
    layer
        .as_ref()
        .and_then(|v| v.get(at))
        .copied()
        .unwrap_or(0.0)
}
pub fn generated_terrain_is_fuel(f: &Floor, at: usize) -> bool {
    f.tiles[at] == "grass"
        || f.tiles[at] == "tree"
        || f.doors
            .as_ref()
            .is_some_and(|d| d.contains_key(&at.to_string()))
        || layer(&f.slime, at) > 0.0
        || layer(&f.roots, at) > 0.0
        || layer(&f.razorleaves, at) > 0.0
        || layer(&f.ichor, at) > 0.0
}
pub fn is_furnace_setup(id: i32) -> bool {
    [1, 2, 4, 12, 19].contains(&id)
}
pub fn fire_setup_weight(f: &Floor, id: i32) -> f64 {
    if !is_furnace_setup(id) {
        return 1.0;
    }
    let (mut fuel, mut ground) = (0, 0);
    for at in 0..f.tiles.len() {
        if ["wall", "chasm", "water", "shallow-water"].contains(&f.tiles[at].as_str()) {
            continue;
        }
        ground += 1;
        if generated_terrain_is_fuel(f, at) {
            fuel += 1
        }
    }
    (1.0 / (1.0
        + 6.0 * fuel as f64 / ground.max(1) as f64
        + if f.domain == "Forest" || f.domain == "Meadow" {
            2.0
        } else {
            0.0
        }))
    .max(0.12)
}
pub fn permanent_fire_safety_cells(f: &Floor, source: usize) -> Vec<usize> {
    let w = f.width();
    let h = f.height();
    let sx = (source % w) as i32;
    let sy = (source / w) as i32;
    let admit = |at: usize| {
        let x = (at % w) as i32;
        let y = (at / w) as i32;
        let dx = x - sx;
        let dy = y - sy;
        x > 0
            && x < w as i32 - 1
            && y > 0
            && y < h as i32 - 1
            && dx * dx + dy * dy <= 5
            && !["wall", "water", "shallow-water", "chasm"].contains(&f.tiles[at].as_str())
            && (!f
                .doors
                .as_ref()
                .is_some_and(|d| d.contains_key(&at.to_string()))
                || dx.abs().max(dy.abs()) <= 1)
    };
    if !admit(source) {
        return vec![];
    }
    let mut cells = vec![source];
    let mut seen = IndexSet::from([source]);
    let mut head = 0;
    while head < cells.len() {
        let at = cells[head];
        head += 1;
        let x = at % w;
        let y = at / w;
        for next in [
            if x > 0 { Some(at - 1) } else { None },
            if x + 1 < w { Some(at + 1) } else { None },
            if y > 0 { Some(at - w) } else { None },
            if y + 1 < h { Some(at + w) } else { None },
        ]
        .into_iter()
        .flatten()
        {
            if !seen.contains(&next) && admit(next) {
                seen.insert(next);
                cells.push(next)
            }
        }
    }
    cells
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GasComponent {
    pub cells: Vec<usize>,
    pub vegetation_cells: Vec<usize>,
    pub fuel_cells: Vec<usize>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerationGasComponents {
    pub has_vegetation: bool,
    pub component_at: Vec<i32>,
    pub initial_component_at: Vec<i32>,
    pub components: Vec<GasComponent>,
}
fn adjacent(at: usize, w: usize, h: usize) -> Vec<usize> {
    let x = (at % w) as i32;
    let y = (at / w) as i32;
    let mut result = vec![];
    for dy in -1..=1 {
        for dx in -1..=1 {
            let xx = x + dx;
            let yy = y + dy;
            if (dx != 0 || dy != 0) && xx >= 0 && yy >= 0 && xx < w as i32 && yy < h as i32 {
                result.push(yy as usize * w + xx as usize)
            }
        }
    }
    result
}
/// Two eight-neighbor graph labelings, durable and initially open. O(V), O(V) scratch.
pub fn build_generation_gas_components(f: &Floor) -> GenerationGasComponents {
    let w = f.width();
    let h = f.height();
    let n = f.tiles.len();
    let mut component_at = vec![-1; n];
    let mut components = vec![];
    for start in 0..n {
        if f.tiles[start] == "wall" || component_at[start] >= 0 {
            continue;
        }
        let id = components.len() as i32;
        let mut cells = vec![start];
        let mut vegetation_cells = vec![];
        let mut fuel_cells = vec![];
        component_at[start] = id;
        let mut head = 0;
        while head < cells.len() {
            let at = cells[head];
            head += 1;
            if f.tiles[at] == "grass" || f.tiles[at] == "tree" {
                vegetation_cells.push(at)
            }
            if generated_terrain_is_fuel(f, at) {
                fuel_cells.push(at)
            }
            for next in adjacent(at, w, h) {
                if component_at[next] >= 0 || f.tiles[next] == "wall" {
                    continue;
                }
                component_at[next] = id;
                cells.push(next)
            }
        }
        components.push(GasComponent {
            cells,
            vegetation_cells,
            fuel_cells,
        })
    }
    let mut initial_component_at = vec![-1; n];
    let mut initial_id = 0;
    let closed = |at: usize| {
        f.doors
            .as_ref()
            .and_then(|d| d.get(&at.to_string()))
            .is_some_and(|d| !d.open)
    };
    for start in 0..n {
        if component_at[start] < 0 || initial_component_at[start] >= 0 || closed(start) {
            continue;
        }
        let mut q = vec![start];
        initial_component_at[start] = initial_id;
        let mut head = 0;
        while head < q.len() {
            let at = q[head];
            head += 1;
            for next in adjacent(at, w, h) {
                if component_at[next] < 0 || initial_component_at[next] >= 0 || closed(next) {
                    continue;
                }
                initial_component_at[next] = initial_id;
                q.push(next)
            }
        }
        initial_id += 1;
    }
    GenerationGasComponents {
        has_vegetation: components.iter().any(|c| !c.vegetation_cells.is_empty()),
        component_at,
        initial_component_at,
        components,
    }
}
pub fn initial_gas_emitter(kind: &str) -> bool {
    [
        "fungus",
        "bastidion",
        "vent-fungus",
        "pale:Breather",
        "pale:Greater Breather",
    ]
    .contains(&kind)
}
pub fn gas_source_shares_combustible_flame_component(
    source: usize,
    fire: usize,
    g: &GenerationGasComponents,
) -> bool {
    let c = g.component_at[source];
    c >= 0
        && c == g.component_at[fire]
        && g.initial_component_at[source] >= 0
        && g.initial_component_at[source] == g.initial_component_at[fire]
        && !g.components[c as usize].vegetation_cells.is_empty()
}
pub fn ordinary_brazier_site_safe(f: &Floor, source: usize, g: &GenerationGasComponents) -> bool {
    let w = f.width();
    let x = (source % w) as i32;
    let y = (source / w) as i32;
    let near = |at: usize| ((at % w) as i32 - x).abs().max(((at / w) as i32 - y).abs()) <= 2;
    if f.doors
        .as_ref()
        .into_iter()
        .flat_map(|d| d.keys())
        .filter_map(|k| k.parse::<usize>().ok())
        .any(near)
        || f.vents
            .as_ref()
            .into_iter()
            .flatten()
            .any(|v| near(v.at as usize))
        || f.grates
            .as_ref()
            .into_iter()
            .flatten()
            .any(|&at| near(at as usize))
        || f.setup_spawns
            .as_ref()
            .into_iter()
            .flatten()
            .any(|p| near(p.y as usize * w + p.x as usize))
    {
        return false;
    }
    for v in f.vents.as_ref().into_iter().flatten() {
        if v.gas != "steam"
            && gas_source_shares_combustible_flame_component(v.at as usize, source, g)
        {
            return false;
        }
    }
    for s in f.setup_spawns.as_ref().into_iter().flatten() {
        if initial_gas_emitter(&s.kind)
            && gas_source_shares_combustible_flame_component(
                s.y as usize * w + s.x as usize,
                source,
                g,
            )
        {
            return false;
        }
    }
    if let Some(e) = f.godbeast_event.as_ref() {
        if x as f64 >= e.room.left - 2.0
            && x as f64 <= e.room.right + 2.0
            && y as f64 >= e.room.top - 2.0
            && y as f64 <= e.room.bottom + 2.0
        {
            return false;
        }
    }
    true
}
pub fn furnace_origins(id: i32, x: i32, y: i32, w: usize) -> Vec<i32> {
    if !is_furnace_setup(id) {
        return vec![];
    }
    let at = y * w as i32 + x;
    if id == 4 {
        vec![at - 3, at - 4, at + 4]
    } else if id == 19 {
        vec![at - 2, at + 1]
    } else {
        vec![at - 3, at - 2]
    }
}
/// Bounded radius-six apron; vegetation cardinality veto bounds candidate work by apron area.
pub fn furnace_containment_cells(
    f: &Floor,
    id: i32,
    x: i32,
    y: i32,
    g: &GenerationGasComponents,
    protected: &IndexSet<usize>,
) -> Option<Vec<usize>> {
    let w = f.width();
    let h = f.height();
    let origins = furnace_origins(id, x, y, w);
    if origins.is_empty() {
        return Some(vec![]);
    }
    if origins
        .iter()
        .any(|&at| at < 0 || at as usize >= f.tiles.len())
    {
        return None;
    }
    let origins: Vec<usize> = origins.into_iter().map(|at| at as usize).collect();
    if origins.iter().any(|&at| g.component_at[at] < 0) {
        return if g.has_vegetation { None } else { Some(vec![]) };
    }
    let near = |at: usize| {
        origins.iter().any(|&origin| {
            let dx = (at % w) as i32 - (origin % w) as i32;
            let dy = (at / w) as i32 - (origin / w) as i32;
            dx * dx + dy * dy <= 36
        })
    };
    let components: IndexSet<i32> = origins.iter().map(|&at| g.component_at[at]).collect();
    let max = origins.len() * 169;
    for &c in &components {
        let vegetation = &g.components[c as usize].vegetation_cells;
        if vegetation.len() > max
            || vegetation
                .iter()
                .any(|&at| !near(at) || protected.contains(&at))
        {
            return None;
        }
    }
    let mut cells = IndexSet::new();
    for origin in origins {
        let ox = (origin % w) as i32;
        let oy = (origin / w) as i32;
        for dy in -6..=6 {
            for dx in -6..=6 {
                let xx = ox + dx;
                let yy = oy + dy;
                if xx <= 0
                    || yy <= 0
                    || xx >= w as i32 - 1
                    || yy >= h as i32 - 1
                    || dx * dx + dy * dy > 36
                {
                    continue;
                }
                let at = yy as usize * w + xx as usize;
                if components.contains(&g.component_at[at]) {
                    cells.insert(at);
                }
            }
        }
    }
    Some(cells.into_iter().collect())
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FireSafetyResult {
    pub firebreak_cells: usize,
    pub cleared_fuel_cells: usize,
}
/// Bounded hearth/apron mutation plus O(V) gas-component labeling; no runtime fuel changes.
pub fn finalize_generated_fire_safety(f: &mut Floor) -> Result<FireSafetyResult, String> {
    let w = f.width();
    let fires = fire_cells(f);
    f.permanent_fire = Field::Value(fires.iter().map(|&at| at as f64).collect());
    if fires.is_empty() {
        return Ok(FireSafetyResult {
            firebreak_cells: 0,
            cleared_fuel_cells: 0,
        });
    }
    if f.substrates.as_ref().is_none() {
        f.substrates = Field::Value(generate_substrates(&f.tiles, &f.domain))
    }
    let mut cleared = IndexSet::new();
    let mut footprint = IndexSet::new();
    for &fire in &fires {
        let sx = (fire % w) as i32;
        let sy = (fire / w) as i32;
        for at in permanent_fire_safety_cells(f, fire) {
            footprint.insert(at);
            let fuel = generated_terrain_is_fuel(f, at);
            if fuel {
                cleared.insert(at);
            }
            if f.tiles[at] == "grass" || f.tiles[at] == "tree" {
                f.tiles[at] = "floor".into()
            }
            if let Some(doors) = f.doors.as_mut() {
                doors.shift_remove(&at.to_string());
            }
            for layer in [&mut f.slime, &mut f.roots, &mut f.razorleaves, &mut f.ichor] {
                if let Some(v) = layer.as_mut() {
                    v[at] = 0.0
                }
            }
            if f.tiles[at] == "floor" {
                let substrate = &mut f.substrates.as_mut().unwrap()[at];
                if fuel {
                    *substrate = "ash".into()
                } else if ((at % w) as i32 - sx)
                    .abs()
                    .max(((at / w) as i32 - sy).abs())
                    <= 1
                    && substrate != "ash"
                {
                    *substrate = "soot".into()
                }
            }
        }
    }
    let graph = build_generation_gas_components(f);
    let protected: IndexSet<usize> = f
        .generation
        .as_ref()
        .and_then(|g| g.terrain_spine.as_ref())
        .into_iter()
        .flatten()
        .map(|&at| at as usize)
        .collect();
    let setups = f.setups.as_ref().cloned().unwrap_or_default();
    for s in setups {
        if !is_furnace_setup(s.id as i32) {
            continue;
        }
        let apron =
            furnace_containment_cells(f, s.id as i32, s.x as i32, s.y as i32, &graph, &protected)
                .ok_or_else(|| {
                format!(
                    "Unsafe furnace containment: setup {} at {},{}",
                    s.id, s.x, s.y
                )
            })?;
        for at in apron {
            footprint.insert(at);
            let vegetation = f.tiles[at] == "grass" || f.tiles[at] == "tree";
            let overlay = layer(&f.slime, at) > 0.0
                || layer(&f.roots, at) > 0.0
                || layer(&f.razorleaves, at) > 0.0;
            if !vegetation && !overlay {
                continue;
            }
            cleared.insert(at);
            if vegetation {
                f.tiles[at] = "floor".into()
            }
            for layer in [&mut f.slime, &mut f.roots, &mut f.razorleaves] {
                if let Some(v) = layer.as_mut() {
                    v[at] = 0.0
                }
            }
            if f.tiles[at] == "floor" {
                f.substrates.as_mut().unwrap()[at] = "ash".into()
            }
        }
    }
    let final_graph = build_generation_gas_components(f);
    let sources: Vec<usize> = f
        .vents
        .as_ref()
        .into_iter()
        .flatten()
        .filter(|v| {
            v.gas != "steam"
                && !f
                    .basic_vent_cells
                    .as_ref()
                    .is_some_and(|c| c.contains(&v.at))
        })
        .map(|v| v.at as usize)
        .chain(
            f.setup_spawns
                .as_ref()
                .into_iter()
                .flatten()
                .filter(|s| initial_gas_emitter(&s.kind))
                .map(|s| s.y as usize * w + s.x as usize),
        )
        .collect();
    for source in sources {
        for &fire in &fires {
            if gas_source_shares_combustible_flame_component(source, fire, &final_graph) {
                return Err(format!(
                    "Unsafe furnace containment: autonomous source {source} shares vegetation with flame {fire}"
                ));
            }
        }
    }
    for &fire in &fires {
        if f.tiles[fire] == "floor" {
            f.substrates.as_mut().unwrap()[fire] = "soot".into()
        }
    }
    for root in f.fungal_roots.as_mut().into_iter().flatten() {
        if root.active && root.path.iter().any(|&at| cleared.contains(&(at as usize))) {
            root.active = false
        }
    }
    Ok(FireSafetyResult {
        firebreak_cells: footprint.len(),
        cleared_fuel_cells: cleared.len(),
    })
}
pub fn initial_fire_separation_radius(kind: &str) -> i32 {
    if generation_creature(kind)
        .is_some_and(|d| d.abilities.iter().any(|a| a == "enemy:beast:razorleaves"))
    {
        return 8;
    }
    if initial_gas_emitter(kind) { 6 } else { 0 }
}
pub fn initial_fuel_spawn_exclusions(f: &Floor, kind: &str) -> IndexSet<usize> {
    let slime = generation_creature(kind)
        .is_some_and(|d| d.abilities.iter().any(|a| a == "enemy:beast:slime-trail"));
    let wide = initial_fire_separation_radius(kind);
    let mut excluded = IndexSet::new();
    if wide == 0 && !slime {
        return excluded;
    }
    let w = f.width();
    let h = f.height();
    let fires = fire_cells(f);
    let radius = if wide != 0 { wide } else { 3 };
    if initial_gas_emitter(kind) {
        let g = build_generation_gas_components(f);
        for &fire in &fires {
            let c = g.component_at[fire];
            if c >= 0 && !g.components[c as usize].vegetation_cells.is_empty() {
                for &cell in &g.components[c as usize].cells {
                    if g.initial_component_at[cell] == g.initial_component_at[fire] {
                        excluded.insert(cell);
                    }
                }
            }
        }
    }
    for source in fires {
        let x = (source % w) as i32;
        let y = (source / w) as i32;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if x + dx < 0
                    || x + dx >= w as i32
                    || y + dy < 0
                    || y + dy >= h as i32
                    || (wide > 0 && dx * dx + dy * dy > radius * radius)
                    || (wide == 0
                        && (dx.abs() - 1).max(0).pow(2) + (dy.abs() - 1).max(0).pow(2) > 5)
                {
                    continue;
                }
                excluded.insert((y + dy) as usize * w + (x + dx) as usize);
            }
        }
    }
    excluded
}
