//! Frozen-v48 full world generation orchestration, with 32 fixed quality candidates.
use super::*;
use crate::{
    model::*,
    rng::{GenerationRng, mix},
};
use indexmap::IndexMap;
use serde::{Serialize, de::DeserializeOwned};
use std::collections::BTreeSet;
fn convert<T: Serialize, U: DeserializeOwned>(value: &T) -> Result<U, String> {
    serde_json::from_value(serde_json::to_value(value).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
fn details<const N: usize>(v: [(&str, f64); N]) -> IndexMap<String, f64> {
    v.into_iter().map(|(k, v)| (k.into(), v)).collect()
}
fn layout_stage(
    id: &str,
    g: &DungeonLayout,
    d: Option<IndexMap<String, f64>>,
) -> GenerationStageRecord {
    generation_stage_record(
        id,
        &g.tiles,
        &convert::<_, Vec<RoomMetadata>>(&g.rooms).expect("native room schema"),
        &convert::<_, Vec<ConnectorMetadata>>(&g.connectors).expect("native connector schema"),
        d,
    )
}
pub const GENERATION_CANDIDATE_LIMIT: u32 = 32;
pub fn generate_floor_full(
    depth: u32,
    seed: u32,
    actual_up: Option<Point>,
) -> Result<Floor, String> {
    generate_floor_full_revision(depth, seed, actual_up,29.0)
}
pub fn generate_floor_full_revision(
    depth: u32,
    seed: u32,
    actual_up: Option<Point>,
    revision: f64,
) -> Result<Floor, String> {
    generate_floor_full_revision_from_attempt(depth, seed, actual_up, revision, 0)
}

/// Continue the same floor plan in a bounded candidate window. Only R38 native
/// startup recovery uses nonzero windows; original seed/domain/stairs remain fixed.
pub fn generate_floor_full_revision_from_attempt(
    depth: u32, seed: u32, actual_up: Option<Point>, revision: f64, first_attempt: u32,
) -> Result<Floor, String> {
    if first_attempt > 224 || first_attempt % GENERATION_CANDIDATE_LIMIT != 0
        || (first_attempt != 0 && revision < 38.0) {
        return Err("Unsupported generation recovery window".into());
    }
    if ![29.0,30.0,31.0,32.0,33.0,34.0,35.0,36.0,37.0,38.0,39.0,40.0,41.0,42.0,43.0,44.0,45.0,46.0,50.0,51.0,52.0].contains(&revision) { return Err("Unsupported generation revision".into()); }
    if !(1..=40).contains(&depth) {
        return Err("Depth must be an integer from 1 to 40".into());
    }
    let plan = domain_plan_revision(seed, revision);
    let d = &plan[(depth as usize - 1) / 5];
    generate_floor_revision_window(
        depth,
        seed,
        &d.domain,
        depth as i32 == d.godbeast_depth,
        79,
        actual_up,
        revision,
        first_attempt,
        None,
    )
}
/// Fast mode changes only the encounter schedule. Destination depth still owns
/// the domain, terrain, population, loot and every depth-scaled creature stat.
/// The normal generator never enters this path, preserving its exact seed stream.
pub(crate) fn generate_fast_floor_from_attempt(
    depth: u32, seed: u32, actual_up: Option<Point>, revision: f64,
    first_attempt: u32, boss_depth: Option<u32>,
) -> Result<Floor, String> {
    if revision < 52. || !(1..=40).contains(&depth)
        || first_attempt > 224 || first_attempt % GENERATION_CANDIDATE_LIMIT != 0
        || boss_depth.is_some_and(|boss| boss == 0 || boss >= 40 || boss % 5 != 0
            || boss > depth || depth - boss > 1) {
        return Err("Unsupported Fast mode floor configuration".into());
    }
    let plan = domain_plan_revision(seed, revision);
    let domain = &plan[(depth as usize - 1) / 5].domain;
    generate_floor_revision_window(depth, seed, domain, boss_depth.is_some(), 79,
        actual_up, revision, first_attempt, boss_depth)
}
pub fn generate_floor(
    depth: u32,
    seed: u32,
    domain: &str,
    godbeast: bool,
    size: usize,
    actual_up: Option<Point>,
) -> Result<Floor, String> {
    generate_floor_revision(depth, seed, domain, godbeast, size, actual_up,29.0)
}
pub fn generate_floor_revision(
    depth: u32,
    seed: u32,
    domain: &str,
    godbeast: bool,
    size: usize,
    actual_up: Option<Point>,
    revision: f64,
) -> Result<Floor, String> {
    generate_floor_revision_window(depth, seed, domain, godbeast, size, actual_up, revision, 0, None)
}

fn generate_floor_revision_window(
    depth: u32, seed: u32, domain: &str, godbeast: bool, size: usize,
    actual_up: Option<Point>, revision: f64, first_attempt: u32, boss_depth: Option<u32>,
) -> Result<Floor, String> {
    if ![29.0,30.0,31.0,32.0,33.0,34.0,35.0,36.0,37.0,38.0,39.0,40.0,41.0,42.0,43.0,44.0,45.0,46.0,50.0,51.0,52.0].contains(&revision) { return Err("Unsupported generation revision".into()); }
    if ![79, 50, 60].contains(&size) {
        return Err("Supported floor geometries are 79×29, 50×50 and 60×60".into());
    }
    if !(1..=40).contains(&depth) {
        return Err("Depth must be an integer from 1 to 40".into());
    }
    if ![
        "Dungeon",
        "Cave",
        "Meadow",
        "Forest",
        "Archipelago",
        "Mountain",
    ]
    .contains(&domain)
    {
        return Err(format!("Unknown domain: {domain}"));
    }
    let mut last = "No candidate generated".to_string();
    for attempt in first_attempt..first_attempt + GENERATION_CANDIDATE_LIMIT {
        match generate_floor_candidate_with_boss(depth, seed, domain, godbeast, size, actual_up, attempt, revision, boss_depth) {
            Ok((mut f, q, failed)) => {
                let stage = f
                    .generation
                    .as_mut()
                    .unwrap()
                    .stages
                    .iter_mut()
                    .find(|s| s.id == "cleanup")
                    .unwrap();
                let d = stage.details.get_or_insert_with(Default::default);
                d.extend(details([
                    ("generationAttempt", attempt as f64),
                    ("qualityAccepted", if q.acceptable { 1. } else { 0. }),
                    ("qualityGroundFraction", q.ground_fraction),
                    ("qualityGroundRatioDeficit", q.ground_ratio_deficit),
                    ("qualityUnderusedSectors", q.underused_sectors as f64),
                    ("topologyUnmetAlternatives", q.missing_alternatives as f64),
                    ("topologyUnresolvedWidths", q.width_failures as f64),
                    ("topologyBrokenPassages", q.broken_passages as f64),
                ]));
                if q.acceptable {
                    return Ok(f);
                }
                let mut val = serde_json::to_value(&q).unwrap();
                val.as_object_mut()
                    .unwrap()
                    .insert("failedLandmarks".into(), failed.into());
                // JSON.stringify emits integral number values without a decimal suffix.
                // Preserve source diagnostic text when the final candidate is rejected.
                for value in val.as_object_mut().unwrap().values_mut() {
                    if let Some(number) = value.as_f64() {
                        if number.fract() == 0.0 && number >= 0.0 && number <= u32::MAX as f64 {
                            *value = serde_json::Value::from(number as u32);
                        }
                    }
                }
                last = serde_json::to_string(&val).unwrap();
            }
            Err(e) => {
                if ![
                    "Dungeon exit cannot reach",
                    "Floor has insufficient space",
                    "Planned terrain leaves no entrance",
                    "Insufficient accretion scaffold",
                    "Unsafe furnace containment",
                    "Insufficient ordinary vent sites",
                    "Unsafe ordinary vent containment",
                    "Unsafe stair recess",
                    "No connected Saidribee Nest",
                ]
                .iter()
                .any(|s| e.contains(s))
                    && !(revision >= 52.0 && e.starts_with("Isolated island"))
                    && !(revision >= 38.0 && [
                        "Automatic staircase has no repairable approach",
                        "Automatic staircase has no accessible approach",
                        "Authored geometry leaves an inaccessible room",
                        "Unowned exit route",
                        "Diagonal doorway has no cardinal approach",
                        "Unowned cardinal room bridge",
                        "No safe Godbeast tap location",
                        "Access repair did not consume solid terrain",
                    ].iter().any(|s| e.contains(s)))
                {
                    return Err(e);
                }
                last = e;
            }
        }
    }
    Err(format!(
        "Structural generation quality exhausted: {domain} floor {depth} seed {seed}: {last}"
    ))
}
pub fn generate_floor_candidate(
    depth: u32,
    seed: u32,
    domain: &str,
    godbeast: bool,
    size: usize,
    actual_up: Option<Point>,
    attempt: u32,
) -> Result<(Floor, TopologyAcceptance, usize), String> {
    generate_floor_candidate_revision(depth, seed, domain, godbeast, size, actual_up, attempt,29.0)
}
pub fn generate_floor_candidate_revision(
    depth: u32,
    seed: u32,
    domain: &str,
    godbeast: bool,
    size: usize,
    actual_up: Option<Point>,
    attempt: u32,
    revision: f64,
) -> Result<(Floor, TopologyAcceptance, usize), String> {
    generate_floor_candidate_with_boss(depth, seed, domain, godbeast, size, actual_up, attempt, revision, None)
}
fn generate_floor_candidate_with_boss(
    depth: u32, seed: u32, domain: &str, godbeast: bool, size: usize,
    actual_up: Option<Point>, attempt: u32, revision: f64, boss_depth: Option<u32>,
) -> Result<(Floor, TopologyAcceptance, usize), String> {
    if ![29.0,30.0,31.0,32.0,33.0,34.0,35.0,36.0,37.0,38.0,39.0,40.0,41.0,42.0,43.0,44.0,45.0,46.0,50.0,51.0,52.0].contains(&revision) { return Err("Unsupported generation revision".into()); }
    if ![79, 50, 60].contains(&size)
        || !(1..=40).contains(&depth)
        || ![
            "Dungeon",
            "Cave",
            "Meadow",
            "Forest",
            "Archipelago",
            "Mountain",
        ]
        .contains(&domain)
    {
        return Err("Unsupported generation candidate configuration".into());
    }
    if revision >= 38.0 && attempt >= 256 {
        return Err("Unsupported generation candidate attempt".into());
    }
    let godbeast = godbeast && (boss_depth.is_some() || revision < 33.0 || (depth < 40 && depth % 5 == 0));
    let generation_seed = if attempt == 0 {
        seed
    } else {
        mix(seed, 730000 + attempt)
    };
    let w = size;
    let h = if w == 79 { 29 } else { w };
    let n = w * h;
    let stairs = floor_stairs(depth, seed, size as i32, actual_up)?;
    let mut protection = BTreeSet::new();
    let fx = (stairs.up.x - 3).clamp(1, w as i32 - 8);
    let fy = (stairs.up.y - 3).clamp(1, h as i32 - 8);
    for dy in 0..7 {
        for dx in 0..7 {
            protection.insert(((fy + dy) * w as i32 + fx + dx) as usize);
        }
    }
    for dy in -1..=1 {
        for dx in -1..=1 {
            protection.insert(((stairs.down.y + dy) * w as i32 + stairs.down.x + dx) as usize);
        }
    }
    let mut rng = GenerationRng::new(mix(generation_seed, depth + 30000));
    let geology = plan_geology(w, h, domain, &mut || rng.random(), &protection)?;
    let mut stages = vec![];
    let mut rng = GenerationRng::new(mix(generation_seed, depth));
    let mut layout = generate_dungeon_layout(
        w,
        stairs.up,
        stairs.down,
        &mut || rng.random(),
        &geology.constraints(),
        Some(&mut |id, g| stages.push(layout_stage(id, g, None))),
    )?;
    let scaffold = layout.tiles.iter().filter(|t| *t == "floor").count();
    if layout.rooms.len() < 8 || scaffold as f64 / (((w - 2) * (h - 2)) as f64) < 0.22 {
        return Err(format!(
            "Insufficient accretion scaffold: {} rooms, {scaffold} cells",
            layout.rooms.len()
        ));
    }
    let plan = domain_plan_revision(seed, revision);
    let d = (depth as usize - 1) / 5;
    let boss_block = (boss_depth.unwrap_or(depth) as usize - 1) / 5;
    let mut rng = GenerationRng::new(mix(generation_seed, depth + 21000));
    let loops = add_room_loops(
        &mut layout,
        &mut || rng.random(),
        &LoopOptions {
            constraints: geology.constraints(),
            ..Default::default()
        },
        None,
    )?;
    stages.push(layout_stage(
        "loops",
        &layout,
        Some(details([("addedLoops", loops as f64)])),
    ));
    if domain != "Dungeon" {
        layout.doors.clear();
        for room in &mut layout.rooms {
            room.doors.clear()
        }
        for c in &mut layout.connectors {
            c.doors.clear()
        }
    }
    let mut architectural = BTreeSet::new();
    for p in [stairs.up, stairs.down] {
        for dy in -1..=1 {
            for dx in -1..=1 {
                architectural.insert(((p.y + dy) * w as i32 + p.x + dx) as usize);
            }
        }
    }
    let mut transitions = vec![];
    if domain != "Dungeon" {
        if depth % 5 == 1 && d > 0 {
            transitions.push(GeologyTransition {
                domain: plan[d - 1].domain.clone(),
                top: true,
            })
        }
        if depth % 5 == 0 && d < 7 {
            transitions.push(GeologyTransition {
                domain: plan[d + 1].domain.clone(),
                top: false,
            })
        }
    }
    let mut tiles = layout.tiles.clone();
    let mut rng = GenerationRng::new(mix(generation_seed, depth + 31000));
    let natural = generate_geology(
        &mut tiles,
        domain,
        &mut || rng.random(),
        &layout,
        &architectural,
        &transitions,
        Some(&geology),
    )?;
    let rooms: Vec<RoomMetadata> = convert(&layout.rooms)?;
    let connectors: Vec<ConnectorMetadata> = convert(&layout.connectors)?;
    let mut f = Floor {
        width: Field::Value(w as f64),
        height: Field::Value(h as f64),
        doors: Field::Value(convert(&layout.doors)?),
        generation: Field::Value(GenerationMetadata {
            version: 1.,
            stages,
            landmarks: Field::Value(convert(&natural.landmarks)?),
            terrain: Field::Value(convert(&natural.terrain)?),
            natural_walls: Field::Value(natural.natural_walls.iter().map(|&i| i as f64).collect()),
            terrain_spine: Field::Value(natural.terrain_spine.iter().map(|&i| i as f64).collect()),
            ..Default::default()
        }),
        enemy_blessing_budget: Field::Value(ordinary_blessing_budget(
            BudgetSeed::Number(seed),
            depth,
        )? as f64),
        light_sources: Field::Value(vec![]),
        substrates: Field::Value(generate_substrates(&tiles, domain)),
        depth: depth as f64,
        domain: domain.into(),
        tiles,
        lights: vec![0.; n],
        seen: vec![false; n],
        godbeast,
        ..Default::default()
    };
    if domain == "Dungeon" {
        f.dungeon_rooms = Field::Value(rooms);
        f.dungeon_connectors = Field::Value(connectors)
    } else {
        f.room_graph = Field::Value(FloorRoomGraph { rooms, connectors })
    }
    generate_shallow_water(&mut f);
    record_generation_stage(
        &mut f,
        "natural-terrain",
        Some(
            natural
                .details
                .iter()
                .map(|(k, &v)| (k.clone(), v as f64))
                .collect(),
        ),
    )?;
    let baseline = capture_landmark_quality_baseline(&f);
    if godbeast {
        let mut rng = GenerationRng::new(mix(generation_seed, depth + 73000));
        f.godbeast_event = Field::Value(reserve_godbeast_room_revision(
            &mut f.tiles,
            &plan[boss_block].godbeast_kind,
            &mut || rng.random(),
            &[stairs.up, stairs.down],
            &layout.rooms.iter().map(|r| r.center).collect::<Vec<_>>(),
            &natural.terrain_spine.iter().copied().collect(),
            revision,
        )?);
        if revision >= 33.0 && plan[boss_block].godbeast_kind == "Veridian" {
            f.godbeast_event.as_mut().unwrap().display_name = "Verdania".to_string().into();
        }
    }
    for p in [stairs.up, stairs.down] {
        for dy in -1..=1 {
            for dx in -1..=1 {
                f.tiles[((p.y + dy) * w as i32 + p.x + dx) as usize] = "floor".into();
            }
        }
    }
    f.tiles[stairs.up.y as usize * w + stairs.up.x as usize] = "up".into();
    f.tiles[stairs.down.y as usize * w + stairs.down.x as usize] =
        if depth == 40 { "pit" } else { "down" }.into();
    let mut rng = GenerationRng::new(mix(generation_seed, depth + 93000));
    let recesses = generate_stair_recesses(&mut f, &mut || rng.random())?;
    f.enemy_families = Field::Value(generation_bestiary().families.clone());
    if domain != "Dungeon" {
        let mut rng = GenerationRng::new(mix(generation_seed, depth + 48200));
        generate_room_doors(&mut f, &mut || rng.random());
    }
    let mut rng = GenerationRng::new(mix(generation_seed, depth + 92000));
    let vents = generate_basic_vents(&mut f, &mut || rng.random());
    if vents != basic_vent_budget(depth as f64) {
        return Err("Insufficient ordinary vent sites".into());
    }
    let mut rng = GenerationRng::new(mix(generation_seed, depth + 91000));
    let lights = generate_habitation_lights(&mut f, &mut || rng.random());
    record_generation_stage(
        &mut f,
        "environment",
        Some(details([
            ("ordinaryLights", lights.len() as f64),
            ("basicVents", vents as f64),
            ("basicVentTarget", basic_vent_budget(depth as f64) as f64),
            ("stairRecesses", recesses.len() as f64),
            ("stairTorches", (recesses.len() * 2) as f64),
        ])),
    )?;
    let mut rng = GenerationRng::new(mix(generation_seed, depth + 48100));
    generate_setups_revision(&mut f, &mut || rng.random(), revision)?;
    let setups = f.setups.as_ref().map(Vec::len).unwrap_or(0);
    record_generation_stage(
        &mut f,
        "trap-setups",
        Some(details([("setups", setups as f64)])),
    )?;
    let initial_fire = finalize_generated_fire_safety(&mut f)?;
    let cleanup = cleanup_generated_floor(&mut f)?;
    let topology = finalize_generated_topology(&mut f)?;
    if topology.changed_cells > 0 {
        cleanup_generated_floor(&mut f)?;
    }
    // Local grass/snow pockets retain the established final fire/vent safety pass.
    if revision >= 33.0 { crate::pocket_biomes::decorate_revision(&mut f, seed, revision)?; }
    let fire = finalize_generated_fire_safety(&mut f)?;
    let vent = reconcile_generated_basic_vents(&mut f);
    if vent.unresolved_basic_vents > 0 {
        return Err("Unsafe ordinary vent containment".into());
    }
    if revision >= 52.0 {
        crate::square_currents::generate_for_attempt(&mut f, seed, attempt);
    } else if revision >= 40.0 {
        crate::shore_currents::repair(&mut f);
        apply_water_setup_currents_revision(&mut f, revision);
        crate::shore_currents::repair(&mut f);
    }
    let final_topology = measure_generated_topology(&f);
    let landmark = measure_landmark_quality(&f, &baseline)?;
    f.lights = f
        .tiles
        .iter()
        .map(|t| {
            (-99f64).max(
                -2. * (depth as f64 - 1.)
                    + if t == "wall" {
                        -35.
                    } else if t == "chasm" {
                        -45.
                    } else {
                        0.
                    },
            )
        })
        .collect();
    for s in f
        .setups
        .as_ref()
        .into_iter()
        .flatten()
        .filter(|s| s.id == 67.)
    {
        for dx in -4..=1 {
            f.lights[(s.y as i32 * w as i32 + s.x as i32 + dx) as usize] =
                (-99f64).max(-70. - 2. * (depth as f64 - 1.));
        }
    }
    let mut d: IndexMap<String, f64> = cleanup.into_iter().map(|(k, v)| (k, v as f64)).collect();
    d.extend(convert::<_, IndexMap<String, f64>>(&fire)?);
    d.extend(convert::<_, IndexMap<String, f64>>(&vent)?);
    d.extend(details([
        ("landmarkFailedMajors", landmark.failed_majors as f64),
        (
            "landmarkSubstantialMajors",
            landmark.substantial_majors as f64,
        ),
        (
            "landmarkMinimumMatchingFraction",
            landmark.minimum_matching_fraction,
        ),
        (
            "landmarkMinimumConnectedFraction",
            landmark.minimum_largest_component_fraction,
        ),
        ("scaffoldRooms", layout.rooms.len() as f64),
        ("scaffoldCells", scaffold as f64),
    ]));
    for (k, v) in convert::<_, IndexMap<String, f64>>(&final_topology)? {
        d.insert(format!("topology_{k}"), v);
    }
    d.extend(details([
        ("topologyAddedLoops", topology.added_loops as f64),
        ("topologyWidenedCells", topology.widened_cells as f64),
        (
            "topologyUnmetAlternatives",
            topology.unmet_alternatives as f64,
        ),
        (
            "topologyUnresolvedWidths",
            topology.unresolved_widths as f64,
        ),
        (
            "clearedFuelCells",
            (initial_fire.cleared_fuel_cells + fire.cleared_fuel_cells) as f64,
        ),
    ]));
    record_generation_stage(&mut f, "cleanup", Some(d))?;
    let mut q = evaluate_generated_topology(&f, &final_topology);
    q.acceptable &= landmark.acceptable;
    if revision >= 33.0 {
        crate::creature_feeding::decorate_floor(&mut f, generation_seed);
        crate::water_blood::register_river_sources(&mut f);
    }
    if revision >= 52.0 && q.acceptable {
        crate::revision52_swimming::placement_cells(&f, &[], &[])?;
    }
    Ok((f, q, landmark.failed_majors))
}
