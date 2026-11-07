//! Admission for the revision-37 terrain overlays; old saves retain missing fields.
use crate::Run;

/// O(total floor cells) serialized input validation, no mutation or RNG.
pub fn validate_run(run: &Run) -> bool {
    let enabled = run.simulation_revision.value_or(29.) >= 37.;
    run.floors.iter().chain(run.dreamwalking.as_ref().into_iter().flat_map(|s| s.floors.iter()))
        .all(|f| (enabled || !f.pocket_biomes.as_ref().into_iter().flatten().any(|b| matches!(b.theme.as_str(), "mountainous" | "forest" | "corn-dimension"))) && [&f.ice, &f.puddles, &f.hellfire, &f.remembered_ice, &f.remembered_puddles, &f.remembered_hellfire]
            .into_iter().all(|layer| layer.is_missing() || enabled && layer.as_ref().is_some_and(|v| v.len() == f.tiles.len())))
}
