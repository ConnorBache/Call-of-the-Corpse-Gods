//! Haste is conditional on current water contact, not an escape assumption.
use corpse_core::{Floor, revision52_swimming};

fn neck(current:f64) -> Floor {
    let mut f=Floor{depth:1.,tiles:vec!["wall".into();81],..Default::default()};
    f.tiles[4*9+1]="up".into();
    f.tiles[4*9+2]="floor".into(); // Entrance shore.
    f.tiles[4*9+3]="water".into(); // Only entry/exit of the one-cell island.
    f.tiles[4*9+4]="floor".into();
    let mut currents=vec![0.;81];currents[4*9+3]=current;f.currents=currents.into();f
}

#[test]
fn inward_neck_current_rejects_ring_that_cannot_create_a_haste_opportunity() {
    // East pushes every attempted departure from the island straight back onto
    // dry land during the normal tick. No water-origin bonus is ever available.
    let f=neck(3.);
    let before=serde_json::to_value(&f).unwrap();
    assert!(revision52_swimming::placement_cells(&f,&[],&[]).unwrap_err().starts_with("Isolated island"));
    assert_eq!(serde_json::to_value(&f).unwrap(),before);
}

#[test]
fn still_water_needs_no_ring_but_unfixable_current_traps_either_shore_reject() {
    assert!(revision52_swimming::placement_cells(&neck(0.),&[],&[]).unwrap().is_empty());
    assert!(revision52_swimming::placement_cells(&neck(7.),&[],&[]).unwrap_err().starts_with("Isolated island"));
}

fn lake(cells:&[usize])->Floor {
    let mut f=Floor{depth:20.,tiles:vec!["floor".into();225],..Default::default()};
    for y in 3..12 {for x in 3..12 {f.tiles[y*15+x]="water".into();}}
    f.tiles[16]="up".into();
    for &at in cells {f.tiles[at]="floor".into();}
    f
}
fn pocket(cells:&[usize])->corpse_core::non_euclidean_generation::NonEuclideanPocket {
    use corpse_core::non_euclidean_generation::{NonEuclideanGroup,NonEuclideanPocket};
    NonEuclideanPocket{cells:cells.to_vec(),groups:cells.chunks(2).map(|pair|
        NonEuclideanGroup{sources:vec![pair[0]],destination:pair[1]}).collect()}
}

#[test]
fn portal_covered_one_cell_island_cannot_disappear_from_safeguard_detection() {
    let at=7*15+7;let mut f=lake(&[at]);
    f.portals=vec![corpse_core::portals_generation::PortalPair{a:at,b:17,color_index:0}].into();
    assert!(corpse_core::portals_generation::validate_floor(&f));
    assert!(revision52_swimming::placement_cells(&f,&[],&[]).unwrap_err().starts_with("Isolated island"));
}

#[test]
fn fully_covered_spatial_or_speed_up_island_rejects_instead_of_disappearing() {
    let cells=[7*15+7,7*15+8];
    for ordered in [false,true] {
        let mut f=lake(&cells);
        if ordered {f.speed_up=vec![pocket(&cells)].into();f.speed_up_terrain=true.into();for &at in &cells {f.tiles[at]="speed-up".into();}}
        else {f.non_euclidean=vec![pocket(&cells)].into();}
        assert!(corpse_core::non_euclidean_generation::validate_floor(&f));
        assert!(revision52_swimming::placement_cells(&f,&[],&[]).unwrap_err().starts_with("Isolated island"));
    }
}

#[test]
fn transport_splitting_trapped_island_fragments_cannot_hide_inaccessible_ring() {
    let mut f=Floor{depth:20.,tiles:vec!["wall".into();225],..Default::default()};
    f.tiles[7*15+1]="up".into();f.tiles[7*15+2]="floor".into();
    f.tiles[7*15+3]="water".into();f.tiles[7*15+4]="water".into();
    let cells=[7*15+5,7*15+6,7*15+7,7*15+8];for &at in &cells{f.tiles[at]="floor".into();}
    let mut currents=vec![0.;225];currents[7*15+3]=3.;f.currents=currents.into();
    f.non_euclidean=vec![pocket(&cells[1..3])].into();
    assert!(corpse_core::non_euclidean_generation::validate_floor(&f));
    assert!(revision52_swimming::placement_cells(&f,&[],&[]).unwrap_err().contains("split or covered"));
}

#[test]
fn trapped_starting_island_gets_ring_and_can_acquire_it_before_reaching_exit() {
    let mut f=neck(3.);f.tiles[4*9+4]="water".into();f.tiles[4*9+5]="floor".into();
    f.tiles[4*9+6]="up".into();f.tiles[4*9+1]="down".into();
    assert_eq!(revision52_swimming::placement_cells(&f,&[],&[]).unwrap(),vec![4*9+5]);
    // The opposite shore is ordinarily escapable and must not receive a ring.
    f.currents.as_mut().unwrap()[4*9+3]=0.;
    assert!(revision52_swimming::placement_cells(&f,&[],&[]).unwrap().is_empty());
}

#[test]
fn ringless_water_corridor_still_requires_unassisted_downstairs_access() {
    let mut f=neck(7.);f.tiles[4*9+4]="water".into();f.tiles[4*9+5]="down".into();
    // There is no detached dry island, hence no safeguard item to assume owned.
    assert!(revision52_swimming::placement_cells(&f,&[],&[]).unwrap_err().contains("without a preowned"));
    f.currents.as_mut().unwrap()[4*9+3]=0.;
    assert!(revision52_swimming::placement_cells(&f,&[],&[]).unwrap().is_empty());
}

#[test]
fn captured_floor37_multi_step_pockets_have_obtainable_ordinary_escape() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/revision52-swimming-floor37-pocket.json")).unwrap();
    assert_eq!(fixture["sourceSeed"],"0-1-2:swimming-survey-b");
    assert_eq!(fixture["sourceGenerationAttempt"],4.);
    assert!(fixture["originalError"].as_str().unwrap().contains("split or covered"));
    let f:Floor=serde_json::from_value(fixture["floor"].clone()).unwrap();
    // Preserve exactly the actor fields used by placement without retaining
    // unrelated combat/stat payloads in the reduced topology fixture.
    let actors:Vec<_>=fixture["actors"].as_array().unwrap().iter().map(|v|corpse_core::Actor{
        id:v["id"].as_f64().unwrap(),x:v["x"].as_f64().unwrap(),y:v["y"].as_f64().unwrap(),
        dead:v.get("dead").and_then(serde_json::Value::as_bool).map(corpse_core::Field::Value).unwrap_or_default(),
        ..Default::default()}).collect();
    let ground:Vec<corpse_core::GroundItem>=serde_json::from_value(fixture["ground"].clone()).unwrap();
    assert_eq!(f.depth,37.);assert!(corpse_core::non_euclidean_generation::validate_floor(&f));
    let before=serde_json::to_value(&f).unwrap();
    assert!(revision52_swimming::placement_cells(&f,&actors,&ground).unwrap().is_empty());
    assert_eq!(serde_json::to_value(&f).unwrap(),before);
}
