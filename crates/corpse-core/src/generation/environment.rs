//! Initial vent reservation and deterministic final relocation.
use super::*;
use crate::{Field, Floor, GasVent};
use indexmap::IndexSet;
use serde::Serialize;
pub const GENERATED_VENT_GASES: [&str; 4] = ["caustic", "paralysis", "flammable", "explosive"];
pub fn setup_count_range(depth: f64) -> (usize, usize) {
    let progress = (depth - 1.0).clamp(0.0, 19.0) / 19.0;
    (
        (2.0 + 18.0 * progress).round() as usize,
        (6.0 + 14.0 * progress).round() as usize,
    )
}
pub fn basic_vent_budget(depth: f64) -> usize {
    setup_count_range(depth).0.div_ceil(4).clamp(1, 5)
}
pub fn generated_gas_fire_separated(
    source: usize,
    fire: usize,
    gas: &str,
    graph: &GenerationGasComponents,
) -> bool {
    gas == "steam" || !gas_source_shares_combustible_flame_component(source, fire, graph)
}
fn near(a: usize, b: usize, w: usize) -> usize {
    (a % w).abs_diff(b % w).max((a / w).abs_diff(b / w))
}
fn site_allowed(
    f: &Floor,
    at: usize,
    fixtures: &IndexSet<usize>,
    stairs: &[usize],
    setups: bool,
) -> bool {
    let w = f.width();
    let x = (at % w) as f64;
    let y = (at / w) as f64;
    if !["floor", "grass"].contains(&f.tiles[at].as_str())
        || fixtures.contains(&at)
        || stairs.iter().any(|&s| near(s, at, w) <= 5)
    {
        return false;
    }
    if setups
        && f.setups
            .as_ref()
            .into_iter()
            .flatten()
            .any(|s| setup_contains(s, x as i32, y as i32))
    {
        return false;
    }
    if let Some(e) = f.godbeast_event.as_ref() {
        if x >= e.room.left - 1.0
            && x <= e.room.right + 1.0
            && y >= e.room.top - 1.0
            && y <= e.room.bottom + 1.0
        {
            return false;
        }
        if e.tap
            .as_ref()
            .is_some_and(|p| (x - p.x).abs().max((y - p.y).abs()) <= 1.0)
        {
            return false;
        }
    }
    [at - 1, at + 1, at - w, at + w]
        .iter()
        .filter(|&&n| ["floor", "grass"].contains(&f.tiles[n].as_str()))
        .count()
        >= 3
}
pub fn generate_basic_vents(f: &mut Floor, r: &mut impl FnMut() -> f64) -> usize {
    let w = f.width();
    let h = f.height();
    let stairs = automatic_transitions(f);
    let fires = fire_cells(f);
    let fixtures: IndexSet<usize> = fires
        .iter()
        .copied()
        .chain(
            f.doors
                .as_ref()
                .into_iter()
                .flat_map(|d| d.keys().filter_map(|k| k.parse::<usize>().ok())),
        )
        .chain(
            f.light_sources
                .as_ref()
                .into_iter()
                .flatten()
                .map(|s| s.y as usize * w + s.x as usize),
        )
        .collect();
    let graph = if fires.is_empty() {
        None
    } else {
        Some(build_generation_gas_components(f))
    };
    let mut candidates = vec![];
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let at = y * w + x;
            if site_allowed(f, at, &fixtures, &stairs, false) {
                candidates.push((at, r()))
            }
        }
    }
    candidates.sort_by(|a, b| a.1.total_cmp(&b.1));
    let mut selected = vec![];
    let mut gas: Option<&str> = None;
    for (at, _) in candidates {
        if selected.len() >= basic_vent_budget(f.depth) {
            break;
        }
        if selected.iter().any(|&other| near(other, at, w) < 6) {
            continue;
        }
        let chosen = *gas.get_or_insert_with(|| GENERATED_VENT_GASES[(r() * 4.0).floor() as usize]);
        if fires
            .iter()
            .any(|&fire| !generated_gas_fire_separated(at, fire, chosen, graph.as_ref().unwrap()))
        {
            continue;
        }
        f.vents.get_or_insert_with(Vec::new).push(GasVent {
            at: at as f64,
            gas: chosen.into(),
            phase: "waiting".into(),
            countdown: 30.0,
            amount: Field::Value(1000.0),
            pulse: Field::Missing,
        });
        selected.push(at);
        gas = None;
    }
    let count = selected.len();
    f.basic_vent_cells = Field::Value(selected.into_iter().map(|at| at as f64).collect());
    count
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VentReconciliation {
    pub relocated_basic_vents: usize,
    pub unresolved_basic_vents: usize,
}
/// O(V + B*C + B²), bounded B<=5 ordinary sources, C<=V candidates; no random draws.
pub fn reconcile_generated_basic_vents(f: &mut Floor) -> VentReconciliation {
    let mut cells: Vec<usize> = f
        .basic_vent_cells
        .as_ref()
        .into_iter()
        .flatten()
        .map(|&at| at as usize)
        .collect();
    let w = f.width();
    let h = f.height();
    let fires = fire_cells(f);
    let graph = build_generation_gas_components(f);
    let mut unsafe_components = IndexSet::new();
    for &fire in &fires {
        let component = graph.component_at[fire];
        let initial = graph.initial_component_at[fire];
        if component >= 0
            && initial >= 0
            && !graph.components[component as usize]
                .vegetation_cells
                .is_empty()
        {
            unsafe_components.insert(initial);
        }
    }
    let excluded: Vec<bool> = graph
        .initial_component_at
        .iter()
        .map(|id| unsafe_components.contains(id))
        .collect();
    let mut fixtures: IndexSet<usize> = f
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
                .map(|&at| at as usize),
        )
        .chain(fires.iter().copied())
        .chain(
            f.doors
                .as_ref()
                .into_iter()
                .flat_map(|d| d.keys().filter_map(|k| k.parse::<usize>().ok())),
        )
        .chain(
            f.light_sources
                .as_ref()
                .into_iter()
                .flatten()
                .map(|s| s.y as usize * w + s.x as usize),
        )
        .chain(
            f.setup_spawns
                .as_ref()
                .into_iter()
                .flatten()
                .map(|s| s.y as usize * w + s.x as usize),
        )
        .collect();
    let stairs = automatic_transitions(f);
    let mut candidates = vec![];
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let at = y * w + x;
            if site_allowed(f, at, &fixtures, &stairs, true) {
                candidates.push(at)
            }
        }
    }
    let (mut relocated, mut unresolved) = (0, 0);
    for index in 0..cells.len() {
        let source = cells[index];
        let vent_index = f
            .vents
            .as_ref()
            .and_then(|v| v.iter().position(|v| v.at == source as f64));
        let Some(vent_index) = vent_index else {
            unresolved += 1;
            continue;
        };
        if !excluded[source] {
            continue;
        }
        let mut best = None;
        let mut distance = usize::MAX;
        for &at in &candidates {
            if excluded[at]
                || fixtures.contains(&at)
                || cells
                    .iter()
                    .enumerate()
                    .any(|(j, &other)| j != index && near(other, at, w) < 6)
            {
                continue;
            }
            let d = (source % w).abs_diff(at % w).pow(2) + (source / w).abs_diff(at / w).pow(2);
            if d < distance {
                distance = d;
                best = Some(at)
            }
        }
        let Some(best) = best else {
            unresolved += 1;
            continue;
        };
        f.vents.as_mut().unwrap()[vent_index].at = best as f64;
        cells[index] = best;
        if let Some(original) = f.basic_vent_cells.as_mut() {
            original[index] = best as f64
        }
        fixtures.insert(best);
        relocated += 1;
    }
    VentReconciliation {
        relocated_basic_vents: relocated,
        unresolved_basic_vents: unresolved,
    }
}
