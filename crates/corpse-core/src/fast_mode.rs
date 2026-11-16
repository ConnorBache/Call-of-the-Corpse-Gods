//! Content-depth routing. New games have twenty physical floors; old saves keep their route.
use crate::Run;

pub fn compressed(r: &Run) -> bool { r.compressed_depths.value_or(false) }
/// Content coordinates preserve authored balance. Milestone floors retain exact content.
pub fn content_depth(floor: u32) -> u32 { if floor % 5 == 0 { floor * 2 } else { floor * 2 - 1 } }
pub fn display_depth(r: &Run, depth: f64) -> f64 { if compressed(r) { (depth / 2.).ceil() } else { depth } }
pub fn maximum_display_depth(r: &Run) -> u32 { if compressed(r) { 20 } else { 40 } }
pub fn enabled(r: &Run) -> bool { r.fast_mode.value_or(false) }
/// Current Fast startup is deliberately limited to the R52 Avatar contract.
pub fn valid_configuration(r: &Run) -> bool {
    !matches!(r.compressed_depths, crate::Field::Null)
        && (!compressed(r) || enabled(r))
        && !matches!(r.fast_mode, crate::Field::Null)
        && (!enabled(r) || r.simulation_revision.as_ref() == Some(&52.) && r.mode == "Avatar" && !r.dev)
}

/// Legacy Fast visits 1,3,...,39,40; new runs use twenty content representatives.
/// A depth beyond40 is a sentinel for no lower floor, never a generated floor.
pub fn below(r: &Run, depth: f64) -> f64 {
    if compressed(r) { return content_depth((depth / 2.).ceil() as u32 + 1) as f64; }
    if !enabled(r) { return depth + 1.; }
    if depth == 39. { 40. } else { depth + 2. }
}

/// O(F), F<=40: return to the previous actual physical floor.
/// Normal mode deliberately retains its exact legacy arithmetic.
pub fn above(r: &Run, depth: f64) -> f64 {
    if compressed(r) { let floor = (depth / 2.).ceil() as u32; return if floor <= 1 { 0. } else { content_depth(floor - 1) as f64 }; }
    if !enabled(r) { return depth - 1.; }
    r.floors.iter().filter(|f| f.depth < depth).map(|f| f.depth)
        .max_by(f64::total_cmp).unwrap_or(if depth == 40. { 39. } else { depth - 2. })
}
pub fn adjacent(r: &Run, from: f64, to: f64) -> bool {
    to == below(r, from) || to == above(r, from)
}
/// Only physical single-floor travel is accelerated. Explicit teleport targets
/// use travel_to and are already expressed in effective floor coordinates.
pub fn destination(r: &Run, from: f64, delta: f64) -> f64 {
    if delta == 1. { below(r, from) }
    else if delta == -1. { above(r, from) }
    else { from + delta }
}
pub fn arrival_target(r: &Run, depth: u32) -> u32 {
    if enabled(r) { (below(r, depth as f64) as u32).min(40) }
    else { crate::startup::arrival_generation_target(r.simulation_revision.value_or(29.), depth) }
}
/// At most one scheduled boss can be crossed by a two-depth step. Its identity
/// comes from the crossed block; terrain, population and rewards use destination.
pub fn boss_depth(previous: u32, destination: u32) -> Option<u32> {
    let next = (previous / 5 + 1) * 5;
    (next <= destination && next < 40).then_some(next)
}

/// O(1). Every explicit target must name a physical floor on this run's route.
/// The route is fixed, including the final Administrator floor.
pub fn playable(r: &Run, depth: f64) -> bool {
    if !depth.is_finite() || depth.fract() != 0. || !(1. ..=40.).contains(&depth) { return false; }
    if compressed(r) { return content_depth((depth / 2.).ceil() as u32) as f64 == depth; }
    if !enabled(r) { return true; }
    depth == 40. || depth % 2. == 1.
}
