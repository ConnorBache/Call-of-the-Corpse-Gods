use super::*;
use crate::{Field, Floor, FloorLightSourcesEntry};
use indexmap::IndexSet;
struct LightingContext {
    occupied: IndexSet<usize>,
    fixtures: IndexSet<usize>,
    stairs: Vec<usize>,
    gas_components: IndexSet<i32>,
    graph: GenerationGasComponents,
    freestanding_budget: usize,
    freestanding_count: usize,
}
fn dry(f: &Floor, at: usize) -> bool {
    f.tiles
        .get(at)
        .is_some_and(|t| t == "floor" || t == "grass")
}
fn sconce(f: &Floor, i: usize) -> bool {
    let w = f.width();
    f.tiles[i] == "wall"
        && [
            if i % w > 0 { Some(i - 1) } else { None },
            if i % w + 1 < w { Some(i + 1) } else { None },
            i.checked_sub(w),
            if i + w < f.tiles.len() {
                Some(i + w)
            } else {
                None
            },
        ]
        .into_iter()
        .flatten()
        .any(|j| {
            dry(f, j)
                || ["up", "down", "pit", "water", "shallow-water"].contains(&f.tiles[j].as_str())
        })
}
impl LightingContext {
    fn eligible(&self, f: &Floor, i: usize) -> bool {
        let w = f.width();
        let h = f.height();
        !self.occupied.contains(&i)
            && !self.fixtures.contains(&i)
            && !f
                .setups
                .as_ref()
                .into_iter()
                .flatten()
                .any(|s| setup_contains(s, (i % w) as i32, (i / w) as i32))
            && (sconce(f, i)
                || (i % w > 0
                    && i % w < w - 1
                    && i >= w
                    && i < (h - 1) * w
                    && dry(f, i)
                    && !(self
                        .gas_components
                        .contains(&self.graph.initial_component_at[i])
                        && self.graph.component_at[i] >= 0
                        && !self.graph.components[self.graph.component_at[i] as usize]
                            .vegetation_cells
                            .is_empty())))
    }
    fn add(&mut self, f: &mut Floor, at: usize, power: f64) -> bool {
        if !self.eligible(f, at) {
            return false;
        }
        let w = f.width();
        let brazier = f.tiles[at] != "wall";
        if brazier
            && (self.freestanding_count >= self.freestanding_budget
                || self
                    .stairs
                    .iter()
                    .any(|&s| (s % w).abs_diff(at % w).max((s / w).abs_diff(at / w)) <= 1)
                || !ordinary_brazier_site_safe(f, at, &self.graph)
                || !brazier_preserves_component_access(f, at))
        {
            return false;
        }
        self.occupied.insert(at);
        f.light_sources
            .get_or_insert_with(Vec::new)
            .push(FloorLightSourcesEntry {
                x: (at % w) as f64,
                y: (at / w) as f64,
                power,
                falloff: 10.0,
                kind: Field::Value(if brazier { "brazier" } else { "torch" }.into()),
            });
        if brazier {
            f.permanent_fire
                .get_or_insert_with(Vec::new)
                .push(at as f64);
            self.freestanding_count += 1;
        }
        true
    }
}
/// Geometry-driven weighted light placement. O(VR + S V²) conservative bound, S<=38;
/// O(V+R) scratch plus bounded source list, no runtime random stream consumption.
pub fn generate_habitation_lights(
    f: &mut Floor,
    r: &mut impl FnMut() -> f64,
) -> Vec<FloorLightSourcesEntry> {
    let w = f.width();
    let h = f.height();
    let graph = build_generation_gas_components(f);
    let gas_components = f
        .vents
        .as_ref()
        .into_iter()
        .flatten()
        .filter(|v| v.gas != "steam")
        .map(|v| graph.initial_component_at[v.at as usize])
        .collect();
    let sources = f.light_sources.get_or_insert_with(Vec::new);
    let initial_count = sources.len();
    let occupied = sources
        .iter()
        .map(|s| s.y as usize * w + s.x as usize)
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
        .chain(
            f.doors
                .as_ref()
                .into_iter()
                .flat_map(|d| d.keys().filter_map(|k| k.parse::<usize>().ok())),
        )
        .chain(
            f.setup_spawns
                .as_ref()
                .into_iter()
                .flatten()
                .map(|s| s.y as usize * w + s.x as usize),
        )
        .collect();
    if let Some(e) = f.godbeast_event.as_ref() {
        if let Some(p) = e.well.as_ref() {
            fixtures.insert(p.y as usize * w + p.x as usize);
        }
        if let Some(p) = e.lever.as_ref() {
            fixtures.insert(p.y as usize * w + p.x as usize);
        }
        if let Some(p) = e.tap.as_ref() {
            fixtures.insert(p.y as usize * w + p.x as usize);
        }
        fixtures.insert(e.y as usize * w + e.x as usize);
    }
    let stairs = automatic_transitions(f);
    let mut context = LightingContext {
        occupied,
        fixtures,
        stairs: stairs.clone(),
        gas_components,
        graph,
        freestanding_budget: usize::MAX,
        freestanding_count: 0,
    };
    let framed: IndexSet<usize> = f
        .generation
        .as_ref()
        .and_then(|g| g.stair_recesses.as_ref())
        .into_iter()
        .flatten()
        .map(|r| r.at as usize)
        .collect();
    for stair in stairs.into_iter().filter(|at| !framed.contains(at)) {
        for at in [stair + 1, stair - w, stair - 1, stair + w] {
            if f.tiles[at] == "wall" && context.add(f, at, 75.0) {
                break;
            }
        }
    }
    let rooms = f
        .room_graph
        .as_ref()
        .map(|g| g.rooms.as_slice())
        .or_else(|| f.dungeon_rooms.as_ref().map(Vec::as_slice))
        .unwrap_or(&[]);
    let doors: Vec<usize> = f
        .doors
        .as_ref()
        .into_iter()
        .flat_map(|d| d.keys().filter_map(|k| k.parse().ok()))
        .collect();
    let freestanding_weight = fire_setup_weight(f, 1);
    let mut candidates = vec![];
    for y in 0..h {
        for x in 0..w {
            let at = y * w + x;
            if !context.eligible(f, at) {
                continue;
            }
            let wall = sconce(f, at);
            let mut affinity = 0.3f64;
            for room in rooms {
                let d = (x as f64 - room.center.x).hypot(y as f64 - room.center.y);
                let constructed = room.kind == "rectangle" || room.kind == "open";
                affinity = affinity.max(if constructed { 5.0 } else { 1.8 } / (1.0 + d / 5.0));
            }
            if doors
                .iter()
                .any(|&d| ((d % w) as f64 - x as f64).hypot((d / w) as f64 - y as f64) <= 3.0)
            {
                affinity += 2.0
            }
            let boundary = [
                at.checked_sub(1),
                at.checked_add(1),
                at.checked_sub(w),
                at.checked_add(w),
            ]
            .into_iter()
            .flatten()
            .any(|n| {
                f.tiles
                    .get(n)
                    .is_some_and(|t| ["wall", "water", "shallow-water"].contains(&t.as_str()))
            });
            let weight = affinity
                * if wall {
                    3.0
                } else {
                    (if boundary { 0.8 } else { 0.25 }) * freestanding_weight
                };
            candidates.push((at, weight, r().max(f64::EPSILON)));
        }
    }
    let slots = (w - 2).div_ceil(10) * (h - 2).div_ceil(10);
    let mut target = 0usize;
    for _ in 0..slots {
        if r() >= 0.12 {
            target += 1
        }
    }
    target = 38.min(initial_count + target.saturating_sub(framed.len() * 2));
    context.freestanding_budget =
        ((target.saturating_sub(initial_count)) as f64 * freestanding_weight).ceil() as usize;
    while f.light_sources.as_ref().unwrap().len() < target && !candidates.is_empty() {
        let mut best = None;
        let mut score = f64::INFINITY;
        for (i, &(at, weight, tie)) in candidates.iter().enumerate() {
            if f.tiles[at] != "wall" && context.freestanding_count >= context.freestanding_budget {
                continue;
            }
            let x = (at % w) as f64;
            let y = (at / w) as f64;
            let distance = f
                .light_sources
                .as_ref()
                .unwrap()
                .iter()
                .fold(f64::INFINITY, |d, s| d.min((s.x - x).hypot(s.y - y)));
            if distance < 4.0 {
                continue;
            }
            let spacing = (distance / 9.0).min(1.0);
            let candidate = -tie.ln() / (weight * spacing);
            if candidate < score {
                score = candidate;
                best = Some(i)
            }
        }
        let Some(best) = best else { break };
        let (at, _, _) = candidates.remove(best);
        context.add(f, at, 60.0 + (r() * 26.0).floor());
    }
    f.light_sources.as_ref().unwrap()[initial_count..].to_vec()
}
