//! R51 mechanics on independently regenerated, R52-selected candidate indices.
//! Only candidate/window numbers cross from the new run: no floor, actor, loot,
//! current or topology output is copied into this comparison world.
use corpse_core::{Run, generation, startup};

pub fn preceding_rules_at_selected_attempts(input_seed: &str, selected: &Run) -> Run {
    let mut old = startup::new_run_shell_revision(input_seed, "Avatar", false, 51.).unwrap();
    assert_eq!(old.seed, selected.seed);
    startup::ensure_floors_with(&mut old, 40, &|depth, seed, up| {
        let chosen = &selected.floors[depth as usize - 1];
        let recorded = chosen.generation.as_ref().unwrap().stages.iter()
            .find(|s| s.id == "cleanup").unwrap().details.as_ref().unwrap();
        let attempt = recorded["generationAttempt"] as u32;
        let window = recorded["populationAttempt"] as u32;
        assert!(window < startup::COMPLETE_FLOOR_ATTEMPT_LIMIT);
        assert_eq!(attempt / generation::GENERATION_CANDIDATE_LIMIT, window);
        let plan = generation::domain_plan_revision(seed, 51.);
        let domain = &plan[(depth as usize - 1) / 5];
        let (mut floor, quality, failed) = generation::generate_floor_candidate_revision(
            depth, seed, &domain.domain, depth as i32 == domain.godbeast_depth,
            79, up, attempt, 51.,
        )?;
        assert!(quality.acceptable, "Independent R51 candidate must satisfy its original geometry contract: floor {depth} attempt {attempt}");
        assert_eq!(failed, 0);
        // Reproduce the public generation wrapper's documented diagnostics from
        // the independent candidate, not from the selected floor's payload.
        let details = floor.generation.as_mut().unwrap().stages.iter_mut()
            .find(|s| s.id == "cleanup").unwrap().details.as_mut().unwrap();
        details.extend([
            ("generationAttempt".into(), attempt as f64),
            ("qualityAccepted".into(), 1.),
            ("qualityGroundFraction".into(), quality.ground_fraction),
            ("qualityGroundRatioDeficit".into(), quality.ground_ratio_deficit),
            ("qualityUnderusedSectors".into(), quality.underused_sectors as f64),
            ("topologyUnmetAlternatives".into(), quality.missing_alternatives as f64),
            ("topologyUnresolvedWidths".into(), quality.width_failures as f64),
            ("topologyBrokenPassages".into(), quality.broken_passages as f64),
            ("populationAttempt".into(), window as f64),
        ]);
        Ok(floor)
    }).unwrap();
    old
}
