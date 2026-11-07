use corpse_core::{Field, Floor, FloorSetupSpawnsEntry, fungus_feasts, population, environment};
use std::collections::HashSet;

fn floor(depth: u32) -> Floor {
    // Geometry is derived from the tile array: use a supported square floor.
    let (w, h) = (29, 29);
    let mut tiles = vec!["floor".to_string(); w*h];
    for i in 0..w*h { if i%w==0 || i%w==w-1 || i/w==0 || i/w==h-1 { tiles[i]="wall".into(); } }
    let mut f = Floor {depth:depth as f64, width:(w as f64).into(), height:(h as f64).into(),
        domain:"Dungeon".into(), tiles, lights:vec![100.;w*h], seen:vec![true;w*h],
        setup_spawns:vec![FloorSetupSpawnsEntry {kind:"fungus".into(),x:14.,y:10.,
            ..Default::default()}].into(), ..Default::default()};
    assert_eq!((f.width(), f.height()), (w, h));
    environment::initialize(&mut f).unwrap(); f
}
fn born(f: &mut Floor, seed: &str, revision: f64) -> Vec<corpse_core::Actor> {
    let mut next = 100.;
    population::populate_machine_enemies_revision(seed, &mut next, f, &[], &[], revision).unwrap()
}
#[test]
fn generated_feasts_have_two_distinct_depth_eligible_victims_and_active_roots() {
    for depth in 1..=40 {
        let pools = fungus_feasts::victim_pools(depth, 51.).unwrap();
        assert!(!pools[0].is_empty(), "floor {depth}");
        // No authored introduction exists two or three floors after floor36.
        let impossible = [37, 38, 39, 40].contains(&depth);
        assert_eq!(pools[1].is_empty(), impossible, "floor {depth}");
        for sample in 0..4 {
            let seed = format!("fungus-feast-{sample}");
            let mut f = floor(depth); let before = f.clone();
            let actors = born(&mut f, &seed, 51.);
            if impossible {
                assert!(actors.is_empty());
                assert!(f.fungal_roots.as_ref().is_none_or(Vec::is_empty));
                let mut repeated = before;
                assert_eq!(born(&mut repeated, &seed, 51.), actors);
                assert_eq!(f, repeated);
                continue;
            }
            assert_eq!(actors.len(), 3, "floor {depth}");
            assert_eq!(actors.iter().filter(|a| a.dead.value_or(false)).count(), 2);
            let roots = f.fungal_roots.as_ref().unwrap(); assert_eq!(roots.len(), 2);
            for (slot, root) in roots.iter().enumerate() {
                let corpse = actors.iter().find(|a| a.id == root.corpse_id).unwrap();
                let kind = pools[slot].iter().find(|kind| population::create_actor_revision(kind,
                    0., 0., 0., false, 51.).unwrap().name == corpse.name).expect("victim is in the slot pool");
                assert!(pools[slot].contains(kind), "{depth} {slot} {kind}");
                assert!(corpse.dead.value_or(false) && corpse.heartless.value_or(false));
                assert_eq!(corpse.corpse_age, Field::Value(0.));
                assert!(environment::corpse_rooted(&f, &actors, corpse));
                assert!(root.path.len() <= 11); assert_eq!(root.fungus_id, actors[0].id);
            }
            assert_ne!(actors[1].name, actors[2].name);
            let mut repeated = before; let again = born(&mut repeated, &seed, 51.);
            assert_eq!(actors, again); assert_eq!(f, repeated);
        }
    }
}
#[test]
fn out_of_depth_means_first_eligible_two_or_three_floors_later() {
    for depth in 1..=36 {
        let pools = fungus_feasts::victim_pools(depth, 51.).unwrap();
        let now = corpse_core::ordinary_roster::candidates_revision(depth, true, 51.).unwrap();
        for kind in &pools[0] { assert!(now.iter().any(|recipe| recipe.kinds.contains(kind))); }
        for kind in &pools[1] {
            assert!(!now.iter().any(|recipe| recipe.kinds.contains(kind)), "{kind} at {depth}");
            let first = (1..=40).find(|&d| corpse_core::ordinary_roster::candidates_revision(d, true, 51.)
                .unwrap().iter().any(|recipe| recipe.kinds.contains(kind))).unwrap();
            assert!([depth+2, depth+3].contains(&first), "{kind} at {depth} first {first}");
        }
    }
}
#[test]
fn placement_finds_nearest_clear_cells_when_old_horizontal_slots_are_blocked() {
    let mut f = floor(10); let w = f.width();
    f.tiles[10*w+12] = "wall".into(); f.tiles[10*w+16] = "wall".into();
    let blocked = HashSet::from([9*w+14]);
    let plan = population::plan_machine_spawns_revision(&f, &blocked, 51.).unwrap();
    assert_eq!(plan.fungus_feasts.len(), 1);
    assert_eq!(plan.fungus_feasts[0].cells, [10*w+13, 10*w+15]);
    for cell in plan.fungus_feasts[0].cells { assert!(plan.reserved_cells.contains(&cell)); }
}
#[test]
fn impossible_layout_omits_fungus_and_both_victims_without_generation_failure() {
    let mut f = floor(10); let w = f.width();
    f.tiles.fill("wall".into()); f.tiles[10*w+14]="floor".into(); f.tiles[10*w+15]="floor".into();
    let plan = population::plan_machine_spawns_revision(&f, &HashSet::new(), 51.).unwrap();
    assert!(plan.groups.is_empty() && plan.fungus_feasts.is_empty());
    assert!(plan.reserved_cells.is_empty());
    assert!(born(&mut f, "no-room", 51.).is_empty());
}
#[test]
fn separate_fungi_never_share_generated_victim_cells() {
    let mut f = floor(10);
    f.setup_spawns.as_mut().unwrap().push(FloorSetupSpawnsEntry {kind:"fungus".into(),x:16.,y:10.,
        ..Default::default()});
    let plan = population::plan_machine_spawns_revision(&f, &HashSet::new(), 51.).unwrap();
    assert_eq!(plan.fungus_feasts.len(), 2);
    assert_eq!(plan.corpse_cells.iter().collect::<HashSet<_>>().len(), 4);
    let actors = born(&mut f, "neighbor-fungi", 51.); assert_eq!(actors.len(), 6);
    for fungus in actors.iter().filter(|a| !a.dead.value_or(false)) {
        assert_eq!(f.fungal_roots.as_ref().unwrap().iter().filter(|root| root.fungus_id==fungus.id).count(), 2);
    }
}
#[test]
fn legacy_fungus_generation_keeps_horizontal_canned_corpses_and_no_initial_roots() {
    for revision in [29., 46., 50.] {
        let mut f = floor(10); let w=f.width();
        let plan = population::plan_machine_spawns_revision(&f, &HashSet::new(), revision).unwrap();
        assert!(plan.fungus_feasts.is_empty());
        assert_eq!(plan.corpse_cells, vec![10*w+12, 10*w+16]);
        let actors = born(&mut f, "legacy-feast", revision);
        assert_eq!(actors.len(), 3); assert_eq!(actors[1].creature_kind, actors[2].creature_kind);
        assert!(f.fungal_roots.as_ref().is_none_or(Vec::is_empty));
    }
}
#[test]
fn no_late_fungi_or_godbeast_corpse_substitutions_when_no_deeper_species_exists() {
    for depth in 37..=40 {
        let pools = fungus_feasts::victim_pools(depth, 51.).unwrap();
        assert!(pools[1].is_empty());
        let mut f = floor(depth);
        assert!(born(&mut f, "no-late-fungus", 51.).is_empty());
    }
    for depth in 1..=40 {
        for pool in fungus_feasts::victim_pools(depth, 51.).unwrap() {
            for kind in pool {
                assert!(corpse_core::ordinary_roster::species_revision(51.).iter().any(|s|s.id==kind)
                    || population::catalog().pairs.values().any(|follower| follower == &kind));
                assert!(!["sphinx", "zillennia", "epidite", "borge", "mercuria", "veridian",
                    "verdania", "phantasmii", "flesh-administrator", "saidri-queen"].contains(&kind.as_str()));
                let a = population::create_actor_revision(&kind, 100., 1., 1., false, 51.).unwrap();
                assert!(!a.conditions.contains_key("godbeastDepth"), "{kind}");
            }
        }
    }
}
#[test]
fn root_glyphs_follow_next_segment_at_bends_and_preserve_last_segment_and_intersections() {
    use corpse_core::{FungalRoot, hazard_memory};
    let mut f = floor(1); let w=f.width();
    // East, northeast, east, south: each turn displays the outgoing segment.
    let path = vec![10*w+10, 10*w+11, 9*w+12, 9*w+13, 10*w+13];
    let mut roots = vec![0.; f.tiles.len()]; for &at in &path { roots[at]=1.; }
    f.roots=roots.into();
    f.fungal_roots=vec![FungalRoot {fungus_id:100.,corpse_id:101.,
        path:path.iter().map(|&at|at as f64).collect(),active:true,color:"#ee6cff".into()}].into();
    let glyphs: Vec<_> = path.iter().map(|&at|hazard_memory::observed_hazards_at_revision(&f,at,0.,51.)
        .root.as_ref().unwrap().glyph.clone()).collect();
    assert_eq!(glyphs, ["─", "╱", "─", "│", "│"]);
    assert_eq!(hazard_memory::observed_hazards_at_revision(&f,path[2],0.,50.)
        .root.as_ref().unwrap().glyph,"╱", "legacy replay keeps historical chord");
    f.fungal_roots.as_mut().unwrap().push(FungalRoot {fungus_id:102.,corpse_id:103.,
        path:vec![path[2] as f64,(path[2]+w) as f64],active:true,color:"#ee6cff".into()});
    assert_eq!(hazard_memory::observed_hazards_at_revision(&f,path[2],0.,51.)
        .root.as_ref().unwrap().glyph,"┼");
    // The one-cell degeneracy retains the existing vertical marker and never panics.
    f.fungal_roots.as_mut().unwrap()[0].path=vec![path[0] as f64];
    assert_eq!(hazard_memory::observed_hazards_at_revision(&f,path[0],0.,51.)
        .root.as_ref().unwrap().glyph,"│");
}
#[test]
fn hidden_root_memory_does_not_change_direction_until_observed_again() {
    use corpse_core::{FungalRoot, hazard_memory};
    let mut f=floor(1); let w=f.width(); let at=10*w+10;
    f.roots=vec![1.;f.tiles.len()].into();
    f.fungal_roots=vec![FungalRoot {fungus_id:100.,corpse_id:101.,path:vec![at as f64,(at+1) as f64],
        active:true,color:"#ee6cff".into()}].into();
    let mut mask=vec![false;f.tiles.len()];mask[at]=true;
    hazard_memory::remember_observed_hazards_revision(&mut f,&mask,0.,51.);
    f.fungal_roots.as_mut().unwrap()[0].path[1]=(at+w) as f64;
    mask[at]=false;hazard_memory::remember_observed_hazards_revision(&mut f,&mask,1.,51.);
    assert_eq!(f.remembered_hazards.as_ref().unwrap()[&at.to_string()].root.as_ref().unwrap().glyph,"─");
    mask[at]=true;hazard_memory::remember_observed_hazards_revision(&mut f,&mask,2.,51.);
    assert_eq!(f.remembered_hazards.as_ref().unwrap()[&at.to_string()].root.as_ref().unwrap().glyph,"│");
}

#[test]
fn dead_feast_pools_include_ordinary_pairs_followers_and_banshees() {
    // Current authored introduction is floor14, not the old roster's floor13.
    for depth in [11, 12] {
        let pools = fungus_feasts::victim_pools(depth, 51.).unwrap();
        assert!(pools[1].iter().any(|kind| kind == "hemogoblin-inventor"));
        assert!(pools[1].iter().any(|kind| kind == "hemogoblin-mech"));
        let actors = born(&mut floor(depth), "paired-feast", 51.);
        assert_eq!(actors.len(), 3);
        assert!(actors[1..].iter().all(|a| a.dead.value_or(false)));
    }
    assert!(fungus_feasts::victim_pools(27, 51.).unwrap()[1].iter().any(|kind| kind == "banshee"));
    assert!(fungus_feasts::victim_pools(34, 51.).unwrap()[1].iter().any(|kind| kind == "ursa-major"));
}

#[test]
fn dead_aquatic_victims_fill_the_floor13_gap_without_changing_spawn_depths() {
    assert_eq!(corpse_core::flavor_placement::bounds("kelpie"), Some((16, 25)));
    assert_eq!(fungus_feasts::victim_pools(13, 51.).unwrap()[1], ["kelpie"]);
    let actors = born(&mut floor(13), "aquatic-corpse-feast", 51.);
    assert_eq!(actors.len(), 3);
    assert!(actors[2].dead.value_or(false));
    assert_eq!(actors[2].creature_kind.as_ref().map(String::as_str), Some("kelpie"));
    let def = corpse_core::generation::setup_definitions().iter().find(|d|d.id==24).unwrap();
    for depth in 1..=40 {
        assert_eq!(corpse_core::generation::setup_eligible_at_depth_revision(def,depth,51.).unwrap(),
            corpse_core::generation::setup_eligible_at_depth_revision(def,depth,50.).unwrap());
    }
}
