//! Revision 33 dissolved blood, separate from cosmetic ground stains and all gases.
//! No RNG, currents, feeding, water geometry, damage rules, or global fire changes.
use crate::{Actor, Floor, Run};
pub const MAX_CONCENTRATION: f64 = 100.0;
pub const KELPIE_SOURCE: f64 = 3.0;
pub const RIVER_SOURCE: f64 = 3.0;
pub const NEIGHBOR_TRANSFER: f64 = 0.10;
pub const RETENTION: f64 = 0.94;
pub const CUTOFF: f64 = 0.05;
pub fn enabled(r: &Run) -> bool { r.simulation_revision.value_or(29.0) >= 33.0 }
pub fn water_cell(f: &Floor, at: usize) -> bool {
    f.tiles.get(at).is_some_and(|t| t == "water" || t == "shallow-water")
        && f.ichor.as_ref().and_then(|v| v.get(at)).copied().unwrap_or(0.0) <= 0.0
}
pub fn active(f: &Floor) -> bool {
    f.water_blood.as_ref().is_some_and(|v| v.iter().any(|&n| n > 0.0))
        || [&f.kelpie_blood_sources, &f.blood_river_sources].iter()
            .any(|sources| sources.as_ref().is_some_and(|v| v.iter().any(|&at| water_cell(f, at as usize))))
}
/// Initial population only; the persisted locations do not follow or require living Kelpies.
/// O(A+N) time and O(N+K) storage on first blood allocation, K <= N unique sources.
pub fn register_kelpie_sources(r: &mut Run, depth: f64) {
    if !enabled(r) { return; }
    let Some(fi) = r.floors.iter().position(|f| f.depth == depth) else { return; };
    let f = &r.floors[fi];
    let width = f.width();
    let sources: Vec<_> = r.actors_at(depth).iter().filter(|a| {
        !a.dead.value_or(false) && a.hp > 0.0
            && (a.creature_kind.as_ref().is_some_and(|s| s == "kelpie") || a.name == "Kelpie")
    }).map(|a| a.y as usize * width + a.x as usize).filter(|&at| water_cell(f, at)).collect();
    let f = crate::mutation_journal::make_mut(&mut r.floors[fi]);
    let n = f.tiles.len();
    f.water_blood.get_or_insert_with(|| vec![0.0; n]);
    if sources.is_empty() { return; }
    let mut registered = vec![false; n];
    for &at in f.kelpie_blood_sources.as_ref().into_iter().flatten() {
        if let Some(cell) = registered.get_mut(at as usize) { *cell = true; }
    }
    for at in sources {
        let points = f.kelpie_blood_sources.get_or_insert_with(Vec::new);
        if !registered[at] { points.push(at as f64); registered[at] = true; }
        let blood = f.water_blood.get_or_insert_with(|| vec![0.0; n]);
        blood[at] = blood[at].max(KELPIE_SOURCE);
    }
}
/// Revision-33 generation boundary only: actual `river` landmark wet cells,
/// never lake landmarks, ordinary cosmetic-red water, or Ichor. Caller gates revision.
/// O(N+L+K) time/O(N+L) transient space for L landmark cells, K existing sources;
/// the source/grid buffers persist through creature death and save/rewind.
pub fn register_river_sources(f: &mut Floor) {
    let sources: Vec<_> = f.generation.as_ref().and_then(|g| g.landmarks.as_ref())
        .into_iter().flatten().filter(|landmark| landmark.kind == "river")
        .flat_map(|landmark| landmark.cells.iter()).copied()
        .filter(|&at| at.is_finite() && at >= 0. && at.fract() == 0.)
        .map(|at| at as usize).filter(|&at| water_cell(f, at)).collect();
    if sources.is_empty() { return; }
    let n = f.tiles.len();
    let mut registered = vec![false; n];
    for &at in f.blood_river_sources.as_ref().into_iter().flatten() {
        if let Some(cell) = registered.get_mut(at as usize) { *cell = true; }
    }
    for at in sources {
        let points = f.blood_river_sources.get_or_insert_with(Vec::new);
        if !registered[at] { points.push(at as f64); registered[at] = true; }
        let blood = f.water_blood.get_or_insert_with(|| vec![0.; n]);
        blood[at] = blood[at].max(RIVER_SOURCE);
    }
}
/// All positive actual HP damage deposits one unit/HP at the physical body tile.
/// O(1), or O(N) for first lazy layer allocation; no RNG or movement hook.
pub fn deposit_damage(r: &mut Run, body: &Actor, actual: f64) {
    if !enabled(r) || !actual.is_finite() || actual <= 0.0 { return; }
    let Some(f) = r.floor() else { return; };
    let at = body.y as usize * f.width() + body.x as usize;
    if !water_cell(f, at) { return; }
    let f = r.floor_mut().unwrap();
    let n = f.tiles.len();
    let blood = f.water_blood.get_or_insert_with(|| vec![0.0; n]);
    blood[at] = (blood[at] + actual).min(MAX_CONCENTRATION);
    f.dirty = true;
}
/// One normal environmental turn. O(N+K) time, O(N) temporary memory; eight
/// bounded transfers per wet cell. Double buffering conserves mass before
/// dilution/cutoff; dry/ichor cells and frozen boundaries cannot carry blood.
pub fn diffuse(f: &mut Floor, frozen: &[bool]) {
    if !active(f) { return; }
    let n = f.tiles.len();
    let width = f.width();
    let height = f.height();
    let wet: Vec<_> = (0..n).map(|i| water_cell(f, i)).collect();
    let old = f.water_blood.as_ref().cloned().unwrap_or_else(|| vec![0.0; n]);
    let mut next = vec![0.0; n];
    for i in 0..n {
        if !wet[i] || old[i] <= 0.0 { continue; }
        if frozen.get(i).copied().unwrap_or(false) { next[i] += old[i]; continue; }
        let mut remaining = old[i];
        for dy in -1isize..=1 {
            for dx in -1isize..=1 {
                if dx == 0 && dy == 0 { continue; }
                let x = (i % width) as isize + dx;
                let y = (i / width) as isize + dy;
                if x < 0 || y < 0 || x >= width as isize || y >= height as isize { continue; }
                let j = y as usize * width + x as usize;
                if !wet[j] || frozen.get(j).copied().unwrap_or(false) { continue; }
                // Dissolved blood cannot jump across a dry corner.
                if dx != 0 && dy != 0 && (!wet[i / width * width + x as usize]
                    || !wet[y as usize * width + i % width]) { continue; }
                let sent = old[i] * NEIGHBOR_TRANSFER;
                next[j] += sent;
                remaining -= sent;
            }
        }
        next[i] += remaining.max(0.0);
    }
    for (i, value) in next.iter_mut().enumerate() {
        if !frozen.get(i).copied().unwrap_or(false) {
            *value = (*value * RETENTION).clamp(0.0, MAX_CONCENTRATION);
            if *value < CUTOFF { *value = 0.0; }
        }
    }
    for (sources, strength) in [(&f.kelpie_blood_sources, KELPIE_SOURCE), (&f.blood_river_sources, RIVER_SOURCE)] {
        for &at in sources.as_ref().into_iter().flatten() {
            let i = at as usize;
            if wet.get(i).copied().unwrap_or(false) && !frozen.get(i).copied().unwrap_or(false) {
                next[i] = next[i].max(strength);
            }
        }
    }
    f.water_blood = next.into();
    f.dirty = true;
}
