//! Revision51 ordinary/Sparkle fire contact with water. Hellfire and the existing
//! combustible Ichor overlay are preserved; no timers, RNG, or persisted fields.
use crate::Floor;

/// O(1) layer checks. Water contact means occupying the wet tile, not adjacency.
pub fn quenches(f: &Floor, i: usize, revision: f64) -> bool {
    revision >= 51.
        && matches!(f.tiles.get(i).map(String::as_str), Some("water" | "shallow-water"))
        && !f.ichor.as_ref().is_some_and(|v| v.get(i).is_some_and(|&v| v > 0.))
        && !crate::terrain_runtime::hellfire_at(f, i)
}
fn clear_flame(f: &mut Floor, i: usize) {
    if let Some(e) = f.environment.as_mut() {
        let changed = e.fire[i] != 0.
            || e.fire_kind.as_ref().is_some_and(|v| v[i].is_some())
            || e.fire_fresh.as_ref().is_some_and(|v| v[i])
            || e.fire_reaction.as_ref().is_some_and(|v| v[i] != 0.)
            || e.provenance.as_ref().and_then(|p| p.get("fire"))
                .is_some_and(|p| p.owner[i] != -1. || p.player[i] != 0.);
        f.dirty |= changed;
        e.fire[i] = 0.;
        if let Some(v) = e.fire_kind.as_mut() { v[i] = None; }
        if let Some(v) = e.fire_fresh.as_mut() { v[i] = false; }
        if let Some(v) = e.fire_reaction.as_mut() { v[i] = 0.; }
        if let Some(p) = e.provenance.as_mut().and_then(|p| p.get_mut("fire")) {
            p.owner[i] = -1.; p.player[i] = 0.;
        }
    }
}
/// O(P+L) source cleanup, O(1) scratch, plus the caller's existing floor COW.
/// Remove only brazier light on this cell; unrelated fixed lights survive.
pub fn extinguish_cell(f: &mut Floor, i: usize, revision: f64) -> bool {
    if !quenches(f, i, revision) { return false; }
    extinguish(f, i);
    true
}
/// Sparkle itself is quenched by water even where independent Ichor can burn.
pub fn extinguish_sparkle(f: &mut Floor, i: usize, revision: f64) -> bool {
    if revision < 51. || !matches!(f.tiles.get(i).map(String::as_str), Some("water" | "shallow-water"))
        || crate::terrain_runtime::hellfire_at(f, i) { return false; }
    extinguish(f, i);
    true
}
fn extinguish(f: &mut Floor, i: usize) {
    clear_flame(f, i);
    if let Some(v) = f.permanent_fire.as_mut() {
        let old=v.len(); v.retain(|&at| at != i as f64); f.dirty |= old != v.len();
    }
    let (x, y) = ((i % f.width()) as f64, (i / f.width()) as f64);
    if let Some(v) = f.light_sources.as_mut() {
        let old=v.len();
        v.retain(|s| !(s.x == x && s.y == y && s.kind.as_ref().is_some_and(|k| k == "brazier")));
        f.dirty |= old != v.len();
    }
}
/// O(V+P+L) time and O(V) scratch at fire-adapter entry, including offscreen
/// execution. Existing/restored wet sources cannot reignite during upkeep.
pub fn extinguish_floor(f: &mut Floor, revision: f64) {
    if revision < 51. { return; }
    let wet: Vec<_> = (0..f.tiles.len()).map(|i| quenches(f, i, revision)).collect();
    for (i, &wet) in wet.iter().enumerate() { if wet { clear_flame(f, i); } }
    if let Some(v) = f.permanent_fire.as_mut() {
        let old=v.len(); v.retain(|&i| !wet.get(i as usize).copied().unwrap_or(false)); f.dirty |= old != v.len();
    }
    let width = f.width();
    if let Some(v) = f.light_sources.as_mut() {
        let old=v.len();
        v.retain(|s| !(s.kind.as_ref().is_some_and(|k| k == "brazier")
            && wet.get(s.y as usize * width + s.x as usize).copied().unwrap_or(false)));
        f.dirty |= old != v.len();
    }
}
