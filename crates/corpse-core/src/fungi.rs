//! Deterministic fungus/corpse conduits; R50 Bastidions emit hallucinogenic gas.
use crate::{Actor, Floor, FungalRoot, FungalRootGrowth, Run};
use std::collections::{HashMap, HashSet};
fn fungus(a: &Actor) -> bool {
    a.abilities.iter().any(|id| id == "enemy:beast:fungal-gas")
}
/// Serialized spawn marker. R50 emission also overrides old stored gas choices.
pub const HALLUCINOGENIC_KIND: f64 = 6.0;
pub const HALLUCINOGENIC_COLOR: &str = "#ee6cff";
/// Authored root material is independent of the emitted gas color.
pub const ROOT_COLOR: &str = "#8b9675";
pub const GROWTH_TURNS: u8 = 3;

impl FungalRoot {
    /// O(1). Missing growth state denotes a fully grown historical connection.
    pub fn grown_path(&self) -> &[f64] {
        &self.path[..self.growth.as_ref().map_or(self.path.len(), |g| g.grown.min(self.path.len()))]
    }
    pub fn reaches_corpse(&self) -> bool { self.grown_path().len() == self.path.len() }
}

pub(crate) fn new_link(source: &Actor, corpse: &Actor, path: Vec<f64>, revision: f64, color: &str) -> FungalRoot {
    FungalRoot {
        fungus_id: source.id, corpse_id: corpse.id, path, active: true, color: color.into(),
        growth: if revision >= 52. { FungalRootGrowth { grown: 1, turns: 0 }.into() }
            else { crate::Field::Missing },
    }
}
pub fn hallucination_emitter(run: &Run, source: &Actor) -> bool {
    crate::revision50::enabled(run) && (crate::corpses::is_bastidion(source)
        || source.creature_kind.as_ref().is_some_and(|kind|
            matches!(kind.as_str(), "bastidion" | "fungus" | "vent-fungus")))
}
/// O(name bytes) identity check and O(1) fixed-table lookup; no RNG.
pub fn emission(run: &Run, source: &Actor) -> (&'static str, &'static str) {
    if hallucination_emitter(run, source) {
        return ("hallucinogenic", HALLUCINOGENIC_COLOR);
    }
    let kind = source.conditions.get("fungalGasType").copied().unwrap_or(0.);
    let gases = ["caustic", "breather", "paralysis", "flammable", "explosive", "hydrogen"];
    let colors = ["#56ff37", "#23d59d", "#a13eff", "#ff9c45", "#ffd600", "#fff36d", "#b5efff"];
    if kind >= 0. && kind.fract() == 0. {
        (gases.get(kind as usize).copied().unwrap_or(gases[0]),
            colors.get(kind as usize).copied().unwrap_or(colors[0]))
    } else { (gases[0], colors[0]) }
}
pub(crate) fn root_path(width: usize, from: &Actor, to: &Actor) -> Vec<f64> {
    let (mut x, mut y) = (from.x as i64, from.y as i64);
    let dx = (to.x as i64 - x).abs();
    let dy = -(to.y as i64 - y).abs();
    let sx = if x < to.x as i64 { 1 } else { -1 };
    let sy = if y < to.y as i64 { 1 } else { -1 };
    let mut error = dx + dy;
    let mut path = vec![];
    loop {
        path.push((y * width as i64 + x) as f64);
        if x == to.x as i64 && y == to.y as i64 {
            break;
        }
        let twice = 2 * error;
        if twice >= dy {
            error += dy;
            x += sx
        }
        if twice <= dx {
            error += dx;
            y += sy
        }
    }
    path
}
/// Remove only material unsupported by a surviving connection. Never recreate
/// a missing/cut root cell, and preserve cells shared by another live path.
/// O(V+K+P) time, O(V) scratch: V<=3600 cells, K links, P path entries.
pub(crate) fn clear_orphaned_root_cells(f: &mut Floor) {
    let Some(roots) = f.roots.as_mut() else { return; };
    let mut retained = vec![false; roots.len()];
    for link in f.fungal_roots.as_ref().into_iter().flatten().filter(|link| link.active) {
        for &at in link.grown_path() {
            if let Some(cell) = retained.get_mut(at as usize) { *cell = true; }
        }
    }
    for (at, root) in roots.iter_mut().enumerate() {
        if *root != 0. && !retained[at] { *root = 0.; f.dirty = true; }
    }
}

/// R51 accepts one original shortest direct path per saved pair. R52 validates
/// only its grown material prefix while retaining the full canonical route.
/// Moved/dead endpoints, missing/burning material and malformed detours sever,
/// not reroute.
/// First pair history wins, including an inactive tombstone preventing regrowth.
/// O(A+E+V+P+K*R) expected time, O(A+V+K+R) scratch; E ability entries
/// scanned to classify A actors, R<=11 for admitted canonical paths.
/// Hash collision costs and COW/load callers are documented with the subsystem.
fn normalize_floor(f: &mut Floor, actors: &[Actor]) {
    let width = f.width();
    if width == 0 { return; }
    let sources: HashMap<_, _> = actors.iter().filter(|a| !a.dead.value_or(false) && fungus(a))
        .map(|a| (a.id as u64, a)).collect();
    let corpses: HashMap<_, _> = actors.iter().filter(|a| a.dead.value_or(false))
        .map(|a| (a.id as u64, a)).collect();
    let mut pairs = HashSet::new();
    if let Some(links) = f.fungal_roots.as_mut() {
        let roots = f.roots.as_ref();
        let fire = f.environment.as_ref();
        links.retain_mut(|link| {
            if !pairs.insert((link.fungus_id as u64, link.corpse_id as u64)) {
                f.dirty = true;
                return false;
            }
            if link.color != ROOT_COLOR { link.color = ROOT_COLOR.into(); f.dirty = true; }
            if link.active {
                let valid = sources.get(&(link.fungus_id as u64)).zip(corpses.get(&(link.corpse_id as u64)))
                    .is_some_and(|(source, corpse)| {
                        (source.x-corpse.x).hypot(source.y-corpse.y) <= 10.
                            && link.path == root_path(width, source, corpse)
                            && link.grown_path().iter().all(|&at| {
                                roots.and_then(|v|v.get(at as usize)).copied().unwrap_or(0.) > 0.
                                    && fire.and_then(|e|e.fire.get(at as usize)).copied().unwrap_or(0.) <= 0.
                            })
                    });
                if !valid { link.active = false; f.dirty = true; }
            }
            true
        });
    }
    clear_orphaned_root_cells(f);
}

/// Shared by active/offscreen ticks and each loaded temporal state. No RNG or
/// corpse replacement; compatibility is gated by the stored simulation revision.
pub(crate) fn normalize_world(run: &mut Run) {
    if run.simulation_revision.value_or(29.) < 51. { return; }
    for floor in &mut run.floors {
        if floor.roots.as_ref().is_none() && floor.fungal_roots.as_ref().is_none() { continue; }
        let actors = run.actors.get(&floor.depth.to_string()).map(|a|a.as_slice()).unwrap_or(&[]);
        normalize_floor(crate::mutation_journal::make_mut(floor), actors);
    }
}
fn normalize_current(run: &mut Run) {
    if run.simulation_revision.value_or(29.) < 51. { return; }
    let Some(floor) = run.floors.iter_mut().find(|f| f.depth == run.depth) else { return; };
    let actors = run.actors.get(&run.depth.to_string()).map(|a|a.as_slice()).unwrap_or(&[]);
    normalize_floor(crate::mutation_journal::make_mut(floor), actors);
}
/// Source death removes its material immediately, retaining saved pair history.
/// O(D+K+P+Vaffected) time plus COW detachment; no actor scan or RNG.
pub(crate) fn sever_source(run: &mut Run, id: f64) {
    if run.simulation_revision.value_or(29.) < 51. { return; }
    let animate=crate::events::enabled(run);
    let mut removed=Vec::new();
    for floor in &mut run.floors {
        if !floor.fungal_roots.as_ref().is_some_and(|links|links.iter().any(|link|link.active && link.fungus_id==id)) {
            continue;
        }
        let floor = crate::mutation_journal::make_mut(floor);
        for link in floor.fungal_roots.as_mut().unwrap() {
            if link.fungus_id == id { link.active = false; }
        }
        let prior:Vec<_>=if animate && floor.depth==run.depth {floor.fungal_roots.as_ref().unwrap().iter().filter(|link|link.fungus_id==id).flat_map(|link|link.path.iter().copied()).filter(|i|floor.roots.as_ref().and_then(|v|v.get(*i as usize)).is_some_and(|v|*v>0.)).collect()}else{vec![]};
        clear_orphaned_root_cells(floor);
        let w=floor.width();
        removed.extend(prior.into_iter().filter(|i|floor.roots.as_ref().is_none_or(|v|v[*i as usize]<=0.)).map(|i|crate::events::EventPoint{x:(i as usize%w)as f64,y:(i as usize/w)as f64}));
        floor.dirty = true;
    }
    crate::events::at(run,"E23","dissolve-root",crate::events::Stage::Special,&removed);
}

/// Original acquisition is O(F*C*(11+LOS)+P), plus actor scans/clones and gas/COW
/// work. R50 adds expected O(F+K+identity bytes) classification/recoloring and O(F)
/// scratch for K retained connections. See docs/revision50-cooldowns-bastidion.md.
/// R51 adds the normalize_current costs above before acquisition. No RNG is
/// used here; inactive catch-up uses this same emission path.
pub fn tick_fungi(run: &mut Run, frozen: &[bool]) -> Result<(), String> {
    normalize_current(run);
    if run.simulation_revision.value_or(29.) >= 52. {
        return tick_growing_fungi(run, frozen);
    }
    let direct_roots = run.simulation_revision.value_or(29.) >= 51.;
    let actors = run.actors_at(run.depth);
    let fungi: Vec<_> = actors
        .iter()
        .filter(|a| !a.dead.value_or(false) && fungus(a))
        .cloned()
        .collect();
    let corpses: Vec<_> = actors
        .iter()
        .filter(|a| a.dead.value_or(false))
        .cloned()
        .collect();
    let live: HashSet<_> = fungi.iter().map(|a| a.id as i64).collect();
    let dead: HashSet<_> = corpses.iter().map(|a| a.id as i64).collect();
    let hallucination_sources: HashSet<_> = fungi.iter()
        .filter(|a| hallucination_emitter(run, a)).map(|a| a.id as i64).collect();
    let f = run.floor_mut().ok_or("Missing floor")?;
    let roots = f.roots.as_ref();
    let fire = f.environment.as_ref();
    let connections = f.fungal_roots.get_or_insert_with(Vec::new);
    let mut pairs: HashSet<_> = connections
        .iter()
        .map(|c| (c.fungus_id as i64, c.corpse_id as i64))
        .collect();
    let mut counts: HashMap<i64, usize> = HashMap::new();
    for c in connections.iter_mut() {
        if !direct_roots && hallucination_sources.contains(&(c.fungus_id as i64))
            && c.color != HALLUCINOGENIC_COLOR {
            c.color = HALLUCINOGENIC_COLOR.into();
        }
        if !live.contains(&(c.fungus_id as i64))
            || !dead.contains(&(c.corpse_id as i64))
            || !c.path.iter().all(|&i| {
                roots.and_then(|v| v.get(i as usize)).copied().unwrap_or(0.) > 0.
                    && fire
                        .and_then(|e| e.fire.get(i as usize))
                        .copied()
                        .unwrap_or(0.)
                        <= 0.
            })
        {
            c.active = false
        }
        if c.active {
            *counts.entry(c.fungus_id as i64).or_default() += 1;
        }
    }
    for source in fungi {
        let (gas, gas_color) = emission(run, &source);
        let color = if direct_roots { ROOT_COLOR } else { gas_color };
        let f = run.floor().unwrap();
        let width = f.width();
        let at = source.y as usize * width + source.x as usize;
        if !frozen.get(at).copied().unwrap_or(false) {
            for corpse in &corpses {
                if (source.x - corpse.x).hypot(source.y - corpse.y) > 10.
                    || !crate::vision::line_of_sight(run, &source, corpse)
                    || pairs.contains(&(source.id as i64, corpse.id as i64))
                {
                    continue;
                }
                let path = root_path(width, &source, corpse);
                let f = run.floor().unwrap();
                if path.iter().any(|&i| {
                    frozen.get(i as usize).copied().unwrap_or(false)
                        || f.environment
                            .as_ref()
                            .and_then(|e| e.fire.get(i as usize))
                            .copied()
                            .unwrap_or(0.)
                            > 0.
                }) {
                    continue;
                }
                let cells:Vec<_>=path.iter().filter(|i|run.floor().unwrap().roots.as_ref().and_then(|v|v.get(**i as usize)).is_none_or(|v|*v<=0.)).map(|i|crate::events::EventPoint{x:(*i as usize%width) as f64,y:(*i as usize/width) as f64}).collect();
                crate::events::cue(run,&source,"E23","root",crate::events::Stage::Special,&cells);
                let f = run.floor_mut().unwrap();
                let n = f.tiles.len();
                let roots = f.roots.get_or_insert_with(|| vec![0.; n]);
                for &i in &path {
                    roots[i as usize] = 1.;
                }
                f.fungal_roots.as_mut().unwrap().push(FungalRoot {
                    fungus_id: source.id,
                    corpse_id: corpse.id,
                    path,
                    active: true,
                    color: color.into(),
                    growth: crate::Field::Missing,
                });
                pairs.insert((source.id as i64, corpse.id as i64));
                *counts.entry(source.id as i64).or_default() += 1;
            }
        }
        let count = counts.get(&(source.id as i64)).copied().unwrap_or(0);
        let before=run.floor().unwrap().environment.as_ref().and_then(|e|e.gas_volume.as_ref()).map_or(0.,|v|v[at]);
        let owner = crate::environment::source_id(run, Some(source.id));
        crate::environment::add_gas(
            run.floor_mut().unwrap(),
            at,
            gas,
            25. * (1 + count) as f64,
            owner,
        )?;
        let after=run.floor().unwrap().environment.as_ref().and_then(|e|e.gas_volume.as_ref()).map_or(0.,|v|v[at]);
        if after>before {crate::animation_enemy::mark(run,&source,"E23","puff-violet",crate::events::Stage::Special);}
    }
    Ok(())
}

/// R52 advances existing direct prefixes before acquiring new links. Called only
/// by the normal environment phase (also once per exact offscreen normal turn),
/// never by haste actions or rendering. No RNG. O(A+F*C*(R+LOS)+K*R) acquisition,
/// prefix checks and growth, plus existing normalization, gas and COW costs.
/// R<=11; retained pair history is unbounded by the initial two-corpse feast.
fn tick_growing_fungi(run: &mut Run, frozen: &[bool]) -> Result<(), String> {
    let actors = run.actors_at(run.depth);
    let fungi: Vec<_> = actors.iter().filter(|a| !a.dead.value_or(false) && fungus(a)).cloned().collect();
    let corpses: Vec<_> = actors.iter().filter(|a| a.dead.value_or(false)).cloned().collect();
    let width = run.floor().ok_or("Missing floor")?.width();
    let mut added = Vec::new();
    let mut counts = HashMap::<u64, usize>::new();
    let mut pairs = HashSet::new();
    {
        let f = run.floor_mut().unwrap();
        let n = f.tiles.len();
        let roots = &mut f.roots;
        for link in f.fungal_roots.get_or_insert_with(Vec::new) {
            pairs.insert((link.fungus_id as u64, link.corpse_id as u64));
            if !link.active { continue; }
            if let Some(growth) = link.growth.as_mut().filter(|g| g.grown < link.path.len()) {
                // Frozen roots retain their exact fractional progress. Fire on
                // grown cells already severed in normalization; an unbuilt next
                // square merely blocks growth, rather than burning phantom roots.
                let next = link.path[growth.grown] as usize;
                if !link.path[..=growth.grown].iter().any(|&at| frozen.get(at as usize).copied().unwrap_or(false))
                    && f.environment.as_ref().and_then(|e| e.fire.get(next)).copied().unwrap_or(0.) <= 0.
                {
                    growth.turns += 1;
                    if growth.turns == GROWTH_TURNS {
                        growth.turns = 0;
                        growth.grown += 1;
                        let roots = roots.get_or_insert_with(|| vec![0.; n]);
                        if roots[next] <= 0. {
                            roots[next] = 1.;
                            added.push(crate::events::EventPoint { x:(next%width) as f64, y:(next/width) as f64 });
                        }
                    }
                    f.dirty = true;
                }
            }
            if link.reaches_corpse() { *counts.entry(link.fungus_id as u64).or_default() += 1; }
        }
    }
    // Material growth is observable terrain, not an actor identity. The existing
    // at() gate keeps hidden cells private and leaves roots recognizable under
    // hallucination without exposing the fungus's true apparent form.
    crate::events::at(run, "E23", "root", crate::events::Stage::Special, &added);
    for source in fungi {
        let at = source.y as usize * width + source.x as usize;
        if !frozen.get(at).copied().unwrap_or(false) {
            for corpse in &corpses {
                if pairs.contains(&(source.id as u64, corpse.id as u64))
                    || (source.x-corpse.x).hypot(source.y-corpse.y) > 10.
                    || !crate::vision::line_of_sight(run, &source, corpse) { continue; }
                let path = root_path(width, &source, corpse);
                let f = run.floor().unwrap();
                if path.iter().any(|&cell| frozen.get(cell as usize).copied().unwrap_or(false)
                    || f.environment.as_ref().and_then(|e|e.fire.get(cell as usize)).copied().unwrap_or(0.) > 0.) { continue; }
                let link = new_link(&source, corpse, path, 52., ROOT_COLOR);
                // Acquisition establishes only the source prefix. Three later
                // normal turns must elapse before the first outward square.
                if link.reaches_corpse() { *counts.entry(source.id as u64).or_default() += 1; }
                let f = run.floor_mut().unwrap();
                let n = f.tiles.len();
                f.roots.get_or_insert_with(|| vec![0.; n])[at] = 1.;
                f.fungal_roots.as_mut().unwrap().push(link);
                f.dirty = true;
                pairs.insert((source.id as u64, corpse.id as u64));
            }
        }
        let (gas, _) = emission(run, &source);
        let count = counts.get(&(source.id as u64)).copied().unwrap_or(0);
        let before = run.floor().unwrap().environment.as_ref().and_then(|e|e.gas_volume.as_ref()).map_or(0., |v|v[at]);
        let owner = crate::environment::source_id(run, Some(source.id));
        crate::environment::add_gas(run.floor_mut().unwrap(), at, gas, 25. * (1+count) as f64, owner)?;
        let after = run.floor().unwrap().environment.as_ref().and_then(|e|e.gas_volume.as_ref()).map_or(0., |v|v[at]);
        if after > before { crate::animation_enemy::mark(run,&source,"E23","puff-violet",crate::events::Stage::Special); }
    }
    Ok(())
}
