//! Observation-only snapshots: hidden cells never consult current hazard state.
use crate::{
    Field, Floor, GasPresence, HazardSnapshot, HazardSnapshotFeature, HazardSnapshotVent,
    RootObservation,
};
use indexmap::IndexMap;
use std::collections::{HashMap, HashSet};
fn value(v: &Field<Vec<f64>>, i: usize) -> f64 {
    v.as_ref().and_then(|v| v.get(i)).copied().unwrap_or(0.)
}
pub struct ObservationContext<'a> {
    roots: HashMap<usize, RootObservation>,
    vents: HashMap<usize, &'a crate::GasVent>,
    sources: HashMap<usize, &'a crate::FloorLightSourcesEntry>,
    permanent: HashSet<usize>,
}
pub fn observation_context(f: &Floor) -> ObservationContext<'_> {
    observation_context_revision(f, 29.0)
}
/// O(P) expected root-index time/space for P saved path entries. R51 uses each
/// forward segment; a terminal cell continues the final segment, never a chord.
pub fn observation_context_revision(f: &Floor, revision: f64) -> ObservationContext<'_> {
    let width = f.width();
    let mut roots: HashMap<usize, RootObservation> = HashMap::new();
    for root in f.fungal_roots.as_ref().into_iter().flatten() {
        if revision >= 51.0 && !root.active { continue; }
        for (n, &at) in root.path.iter().enumerate() {
            let at = at as usize;
            if value(&f.roots, at) == 0. {
                continue;
            }
            let before = if revision >= 51.0 && n + 1 < root.path.len() { at }
                else { root.path[n.saturating_sub(1)] as usize };
            let after = root.path[(n + 1).min(root.path.len() - 1)] as usize;
            let dx = (after % width) as i64 - (before % width) as i64;
            let dy = (after / width) as i64 - (before / width) as i64;
            let glyph = if dx == 0 {
                "│"
            } else if dy == 0 {
                "─"
            } else if dx * dy > 0 {
                "╲"
            } else {
                "╱"
            };
            let prior = roots.get(&at);
            roots.insert(
                at,
                RootObservation {
                    glyph: if prior.is_some_and(|p| p.glyph != glyph) {
                        "┼"
                    } else {
                        glyph
                    }
                    .into(),
                    color: if revision >= 51.0 {
                        crate::fungi::ROOT_COLOR.into()
                    } else if root.active {
                        root.color.clone()
                    } else {
                        "#82746b".into()
                    },
                    active: root.active || prior.is_some_and(|p| p.active),
                },
            );
        }
    }
    let vents: HashMap<_, _> = f
        .vents
        .as_ref()
        .into_iter()
        .flatten()
        .map(|v| (v.at as usize, v))
        .collect();
    let sources: HashMap<_, _> = f
        .light_sources
        .as_ref()
        .into_iter()
        .flatten()
        .map(|s| (s.y as usize * width + s.x as usize, s))
        .collect();
    let permanent: HashSet<_> = f
        .permanent_fire
        .as_ref()
        .into_iter()
        .flatten()
        .map(|&v| v as usize)
        .collect();
    ObservationContext {
        roots,
        vents,
        sources,
        permanent,
    }
}
pub fn observed_hazards_at(f: &Floor, at: usize, tick: f64) -> HazardSnapshot {
    observed_hazards_at_revision(f, at, tick, 29.0)
}
pub fn observed_hazards_at_revision(f: &Floor, at: usize, tick: f64, revision: f64) -> HazardSnapshot {
    let context = observation_context_revision(f, revision);
    observed_hazards_with_context(f, at, tick, &context)
}
pub fn observed_hazards_with_context(
    f: &Floor,
    at: usize,
    tick: f64,
    context: &ObservationContext<'_>,
) -> HazardSnapshot {
    let width = f.width();
    let ObservationContext {
        roots,
        vents,
        sources,
        permanent,
    } = context;
    let mut out = HazardSnapshot::default();
    if let Some(e) = f.environment.as_ref() {
        if let (Some(types), Some(volumes)) = (e.gas_type.as_ref(), e.gas_volume.as_ref()) {
            if let Some(Some(gas)) = types.get(at) {
                out.gas = GasPresence {
                    r#type: gas.clone(),
                    concentration: volumes[at],
                }
                .into();
            }
        } else {
            let mut max = 0.;
            let mut total = 0.;
            let mut gas = None;
            for kind in crate::environment::GAS_TYPES {
                let v = crate::environment::legacy_channel(e, kind)
                    .get(at)
                    .copied()
                    .unwrap_or(0.)
                    .max(0.);
                total += v;
                if v > max {
                    max = v;
                    gas = Some(kind);
                }
            }
            if let Some(gas) = gas {
                out.gas = GasPresence {
                    r#type: gas.into(),
                    concentration: (crate::rng::to_u32(total.floor()) & 65535) as f64,
                }
                .into();
            }
        }
        let kind = e
            .fire_kind
            .as_ref()
            .and_then(|v| v.get(at))
            .and_then(|v| v.as_deref());
        if kind == Some("embers") {
            out.fire = Field::Value("embers".into())
        } else if e.fire.get(at).copied().unwrap_or(0.) > 0. {
            out.fire = Field::Value(kind.unwrap_or("creature").into());
        }
    }
    if permanent.contains(&at) {
        out.permanent_fire = true.into();
    }
    for (field, target) in [
        (&f.slime, &mut out.slime),
        (&f.mud, &mut out.mud),
        (&f.ichor, &mut out.ichor),
        (&f.razorleaves, &mut out.razorleaves),
        (&f.roots, &mut out.roots),
    ] {
        if value(field, at) != 0. {
            *target = true.into();
        }
    }
    if f.blood
        .as_ref()
        .and_then(|v| v.get(at))
        .copied()
        .unwrap_or(false)
    {
        out.blood = true.into();
    }
    if crate::creature_feeding::bloody_wall(f, at) { out.bloody_wall = true.into(); }
    let dissolved = value(&f.water_blood, at);
    if f.water_blood.as_ref().is_some() && crate::water_blood::water_cell(f, at) {
        out.water_blood = dissolved.into();
    }
    if let Some(root) = roots.get(&at) {
        out.root = root.clone().into();
    }
    let current = value(&f.currents, at);
    if f.tiles[at] == "water" && !out.ichor.value_or(false) && (1.0..=8.).contains(&current) {
        out.current = current.into();
    }
    if let Some(vent) = vents.get(&at) {
        out.vent = HazardSnapshotVent {
            gas: vent.gas.clone(),
            phase: vent.phase.clone(),
            countdown: vent.countdown,
        }
        .into();
    }
    if let Some(&expiry) = f.bulwarks.as_ref().and_then(|v| v.get(&at.to_string())) {
        if expiry >= tick {
            out.bulwark = (expiry - tick).into();
        }
    }
    if let Some(source) = sources.get(&at) {
        out.light_source = source
            .kind
            .value_or(if f.tiles[at] == "wall" {
                "torch".into()
            } else {
                "brazier".into()
            })
            .into();
    }
    if let Some(e) = f.godbeast_event.as_ref() {
        let x = (at % width) as f64;
        let y = (at / width) as f64;
        let feature = if crate::epidite::solid_at(f,x,y) {
            Some(("T", "#fff7d6", "Epidite", "A tower of cleansing light. Every 30 turns it blesses exposed hearts and deals 100 damage through DR to creatures in sight."))
        } else if e.kind=="Phantasmii" && e.altar.as_ref().is_some_and(|p|p.x==x && p.y==y) {
            Some(("_", "#9295de", "Altar of the Phantasmii", "Stand here and apply the Dagger of the Phantasmii to begin the ritual."))
        } else if crate::borge::solid_at(f,x,y) {
            Some((if x==e.x { "┃" } else { "█" }, "#796484", "The Borge", "A huge dark obelisk. Strike its edge to learn its demand."))
        } else if e.kind=="Borge" && f.tiles[at]!="chasm" && e.altar.as_ref().is_some_and(|p|p.x==x && p.y==y) {
            Some(("π", "#c5ad63", "Borge sacrifice altar", if e.reward_created.value_or(false) {"The Borge has accepted its one sacrifice."} else {"Bring the exact marked heart here and walk into the altar to offer it."}))
        } else if e.kind == "Mercuria"
            && e.lever.as_ref().is_some_and(|p| p.x == x && p.y == y)
        {
            Some((
                "/",
                "#d8be79",
                "Mercuria’s lever",
                if e.activated.value_or(false) {
                    "The lever has opened the ichor well."
                } else {
                    "Walk into the lever to open the ichor well."
                },
            ))
        } else if e.kind == "Mercuria" && e.well.as_ref().is_some_and(|p| p.x == x && p.y == y) {
            Some((
                "O",
                "#c5ad63",
                "Mercuria’s well",
                if e.activated.value_or(false) {
                    "Ichor rises from the well."
                } else {
                    "An empty well waits beside its lever."
                },
            ))
        } else if e.kind == "Veridian" && e.x == x && e.y == y {
            Some((
                "♣",
                "#c0b488",
                e.display_name.as_ref().map(String::as_str).unwrap_or("Veridian"),
                if e.tapped.value_or(false) {
                    "The ancient tree has yielded its one gift."
                } else if e.display_name.as_ref().is_some_and(|name|name=="Verdania") {
                    "An ancient tree. Apply a tree tap while adjacent to collect Verdania Sap."
                } else {
                    "An ancient tree. Apply a tree tap while adjacent to collect Veridian Sap."
                },
            ))
        } else {
            None
        };
        if let Some((glyph, color, name, description)) = feature {
            out.feature = HazardSnapshotFeature {
                glyph: glyph.into(),
                color: color.into(),
                name: name.into(),
                description: description.into(),
            }
            .into();
        }
    }
    out
}
pub fn remember_observed_hazards(f: &mut Floor, mask: &[bool], tick: f64) {
    remember_observed_hazards_revision(f, mask, tick, 29.0);
}
pub fn remember_observed_hazards_revision(f: &mut Floor, mask: &[bool], tick: f64, revision: f64) {
    let context = observation_context_revision(f, revision);
    let updates: Vec<_> = (0..f.tiles.len())
        .filter(|&at| mask.get(at).copied().unwrap_or(false))
        .map(|at| (at, observed_hazards_with_context(f, at, tick, &context)))
        .collect();
    let memory = f.remembered_hazards.get_or_insert_with(IndexMap::new);
    for (at, out) in updates {
        if out == HazardSnapshot::default() {
            memory.shift_remove(&at.to_string());
        } else {
            memory.insert(at.to_string(), out);
        }
    }
}
