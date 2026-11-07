//! Designer-authored R51 optical range and awareness primitives. Pure queries;
//! no RNG, world mutation, caches or dependency on presentation brightness.
use crate::Run;

pub fn enabled(r: &Run) -> bool { r.simulation_revision.value_or(29.) >= 51. }

/// Explicit policy captures the designer's clarification: trust the formula.
/// Positive Perception can see into -100 light; no special darkness cutoff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FullDarknessRule { Formula, Opaque }
/// Designer-confirmed literal formula (2026-11-07).
pub const FULL_DARKNESS_RULE: FullDarknessRule = FullDarknessRule::Formula;

/// Neutral-light terrain range. Creature range is half before target stealth.
pub fn perception_range(perception: f64) -> f64 { (20. + perception / 5.).max(0.) }
/// The signed terrain contribution shown by inspection, not raw illumination.
pub fn tile_modifier(illumination: f64) -> f64 { illumination / 5. }
/// Finite formula component; light >= +1 bypasses distance in tile_in_range.
pub fn tile_range(perception: f64, illumination: f64) -> f64 {
    (20. + tile_modifier(illumination) + perception / 5.).max(0.)
}
pub fn tile_in_range_with_rule(perception: f64, distance: f64, illumination: f64,
    rule: FullDarknessRule) -> bool {
    if !distance.is_finite() || distance < 0. || !perception.is_finite()
        || !illumination.is_finite() || (rule == FullDarknessRule::Opaque && illumination <= -100.) {
        return false;
    }
    if illumination >= 1. { return true; }
    let range = tile_range(perception, illumination);
    range > 0. && distance <= range
}
pub fn tile_in_range(perception: f64, distance: f64, illumination: f64) -> bool {
    tile_in_range_with_rule(perception, distance, illumination, FULL_DARKNESS_RULE)
}
pub fn functional_distance(distance: f64, stealth: f64, submerged_stealth: f64) -> f64 {
    (distance + stealth + submerged_stealth).max(0.)
}
pub fn creature_in_range(perception: f64, distance: f64, stealth: f64,
    submerged_stealth: f64, illumination: f64) -> bool {
    distance.is_finite() && tile_in_range(perception,
        2. * functional_distance(distance, stealth, submerged_stealth), illumination)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inclusive_tile_and_creature_boundaries() {
        assert!(tile_in_range(0.,20.,0.)); assert!(!tile_in_range(0.,20.001,0.));
        assert!(creature_in_range(0.,10.,0.,0.,0.));
        assert!(!creature_in_range(0.,10.001,0.,0.,0.));
        assert!(tile_in_range(100.,40.,0.));
        assert!(creature_in_range(100.,20.,0.,0.,0.));
    }
    #[test]
    fn positive_light_bypasses_distance_at_one_not_small_fraction() {
        assert!(tile_in_range(0.,1000.,1.));
        assert!(creature_in_range(0.,1000.,900.,0.,1.));
        assert!(!tile_in_range(0.,1000.,0.999));
        assert!(!tile_in_range(0.,f64::INFINITY,1.));
    }
    #[test]
    fn darkness_decision_is_explicit_and_both_policies_are_tested() {
        for rule in [FullDarknessRule::Formula,FullDarknessRule::Opaque] {
            assert!(!tile_in_range_with_rule(0.,1.,-100.,rule));
        }
        assert!(tile_in_range_with_rule(100.,20.,-100.,FullDarknessRule::Formula));
        assert!(!tile_in_range_with_rule(100.,1.,-100.,FullDarknessRule::Opaque));
        assert!(tile_in_range(100.,20.,-100.));
        assert!(creature_in_range(100.,10.,0.,0.,-100.));
    }
    #[test]
    fn stealth_adds_before_doubling_without_changing_terrain() {
        assert_eq!(functional_distance(25.,5.,0.),30.);
        assert!(creature_in_range(0.,5.,5.,0.,0.));
        assert!(!creature_in_range(0.,5.001,5.,0.,0.));
        assert!(tile_in_range(0.,10.,0.));
        assert!(creature_in_range(0.,14.,-4.,0.,0.));
        assert!(creature_in_range(0.,4.,0.,6.,0.));
        assert!(!creature_in_range(0.,4.001,0.,6.,0.));
    }
    #[test]
    fn range_modifiers_are_not_rounded_away() {
        assert_eq!(tile_modifier(-15.),-3.);
        assert_eq!(tile_range(0.,-15.),17.);
        assert_eq!(tile_range(3.,-15.),17.6);
        assert!(creature_in_range(0.,8.5,0.,0.,-15.));
        assert_eq!(functional_distance(8.,0.5,0.25),8.75);
        assert!(creature_in_range(0.,9.5,0.5,0.,0.));
        assert!(!creature_in_range(0.,9.501,0.5,0.,0.));
        assert!(creature_in_range(0.,10.5,-0.5,0.,0.));
    }
}

/// Awareness is separate from optical eligibility. Sleeping range uses actual
/// route distance, not doubled creature distance or target stealth a second time.
pub fn awareness_chance(plausible: bool, player_or_ally: bool, sleeping: bool,
    hunting: bool, distance: f64) -> f64 {
    if !plausible { return 0.; }
    if player_or_ally { return 100.; }
    if sleeping { return if distance.is_finite() && distance <= 10. { 10. } else { 0. }; }
    if hunting { 100. } else { 25. }
}
