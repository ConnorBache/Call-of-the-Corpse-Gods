//! Native new-floor and seeded new-run integration with exact procedural geometry.
use crate::{
    Actor, Floor, GroundItem, Item, Run, population as p,
    spawn_heatmap::{SpawnSampler, build_spawn_heatmap},
};
use indexmap::IndexMap;
use std::{collections::HashSet, sync::Arc};
pub fn arrow_bundle_floors(seed: &str, depth: u32) -> Vec<u32> {
    let first = (depth - 1) / 5 * 5 + 1;
    let mut rng = p::population_random(&format!("{seed}:set:{first}:arrow-bundles"));
    let count = 2 + (rng.random() * 2.0).floor() as usize;
    let mut remaining: Vec<_> = (first..first + 5).collect();
    let mut chosen = Vec::new();
    for _ in 0..count {
        chosen.push(remaining.remove((rng.random() * remaining.len() as f64).floor() as usize));
    }
    chosen.sort();
    chosen
}
fn ray(f: &Floor, a: &Actor, x: i32, y: i32) -> bool {
    let (mut xx, mut yy) = (a.x as i32, a.y as i32);
    let (dx, dy) = ((x - xx).abs(), (y - yy).abs());
    let (sx, sy) = (if xx < x { 1 } else { -1 }, if yy < y { 1 } else { -1 });
    let mut e = dx - dy;
    while xx != x || yy != y {
        let e2 = e * 2;
        if e2 > -dy {
            e -= dy;
            xx += sx
        }
        if e2 < dx {
            e += dx;
            yy += sy
        }
        if xx == x && yy == y {
            return true;
        }
        let i = yy as usize * f.width() + xx as usize;
        if ["wall", "tree"].contains(&f.tiles[i].as_str()) || crate::environment::blocks_sight(f, i)
        {
            return false;
        }
    }
    true
}
pub fn refresh_razorleaves(f: &mut Floor, actors: &[Actor]) {
    let mut leaves = f.razorleaves.as_ref().map(|v| vec![0.0; v.len()]);
    for a in actors.iter().filter(|a| {
        !a.dead.value_or(false) && a.abilities.iter().any(|id| id == "enemy:beast:razorleaves")
    }) {
        let layer = leaves.get_or_insert_with(|| vec![0.0; f.tiles.len()]);
        for y in (a.y as i32 - 5).max(0)..=(a.y as i32 + 5).min(f.height() as i32 - 1) {
            for x in (a.x as i32 - 5).max(0)..=(a.x as i32 + 5).min(f.width() as i32 - 1) {
                let at = y as usize * f.width() + x as usize;
                if (x as f64 - a.x).hypot(y as f64 - a.y) <= 5.0
                    && f.tiles[at] != "wall"
                    && ray(f, a, x, y)
                {
                    layer[at] = 1.0
                }
            }
        }
    }
    if let Some(v) = leaves {
        f.razorleaves = v.into();
    }
}
fn place_generated(
    r: &mut Run,
    f: &mut Floor,
    ground: &mut Vec<GroundItem>,
    item: Item,
    at: usize,
) -> Result<(), String> {
    let (x, y) = ((at % f.width()) as f64, (at / f.width()) as f64);
    if at >= f.tiles.len()
        || !["floor", "grass"].contains(&f.tiles[at].as_str())
        || ground.iter().any(|g| g.x == x && g.y == y)
    {
        return Err(format!(
            "Required generated item invalid/occupied on floor{}",
            f.depth
        ));
    }
    *r.metrics.entry("itemPlacementCalls".into()).or_default() += 1.0;
    *r.metrics.entry("itemPlacementCells".into()).or_default() += f.tiles.len() as f64;
    *r.metrics
        .entry("itemPlacementOccupancyEntries".into())
        .or_default() += ground.len() as f64;
    ground.push(GroundItem {
        x,
        y,
        item,
        ..Default::default()
    });
    f.dirty = true;
    Ok(())
}
/// O(T_clone + T_population + T_index) inclusive time; clone/COW payload memory
/// follows Run and population costs. T_index is O((A+P+B)*d+F+V), documented
/// in creature_index.rs. Atomic boundary: errors preserve the supplied run.
pub fn spawn_floor(r: &mut Run, depth: u32) -> Result<(), String> {
    crate::creature_index::check_player(r)?;
    let mut draft = r.clone();
    spawn_floor_inner(&mut draft, depth)?;
    draft.actors.validate_allocator(draft.player.id,draft.depth,draft.next_id)?;
    draft.actors.adopt_epoch_from(&r.actors);
    *r = draft;
    Ok(())
}
fn spawn_floor_inner(r: &mut Run, depth: u32) -> Result<(), String> {
    let fi = r
        .floors
        .iter()
        .position(|f| f.depth == depth as f64)
        .ok_or("Missing generated floor")?;
    let mut f = (*r.floors[fi]).clone();
    let w = f.width();
    let mut ground = Vec::new();
    let seed = r.seed.clone();
    if let Some(sphinx) = crate::sphinx_generation::plan_sphinx_room(
        &mut f,
        &mut p::population_random(&format!("{seed}:floor:{depth}:sphinx")),
    ) {
        f.sphinx_room = sphinx.into();
    }
    let mut actors = p::populate_hordes_revision(
        &seed,
        &mut r.next_id,
        &mut f,
        &ground,
        None,
        r.simulation_revision.value_or(29.0),
    )?;
    refresh_razorleaves(&mut f, &actors);
    let machines = p::plan_machine_spawns_revision(&f, &p::blocked_cells(&f), r.simulation_revision.value_or(29.0))?;
    crate::verdania_guard::place(r, &mut f, &mut actors, &machines.reserved_cells)?;
    refresh_razorleaves(&mut f, &actors);
    let mut preview = f.clone();
    let mut preview_id = 0.0;
    let mut machine_rng = p::population_random(&format!("{seed}:floor:{depth}:machine-enemies"));
    let mut initial = actors.clone();
    for group in &machines.groups {
        for intent in &group.intents {
            initial.push(p::create_population_actor_revision(
                &mut preview_id,
                &mut preview,
                &intent.kind,
                intent.y as usize * w + intent.x as usize,
                false,
                &mut machine_rng,
                r.simulation_revision.value_or(29.0),
            )?);
        }
    }
    let mut hearts = 0.0;
    let mut carried = 0.0;
    for a in &initial {
        if !a.dead.value_or(false)
            && !a.heartless.value_or(false)
            && crate::inventory::heart_for(a, 0.0)
                .map_err(|e| e.0)?
                .exp
                .value_or(0.0)
                <= 0.0
        {
            hearts += 1.0
        }
        if let Some(items) = a
            .monster_state
            .as_ref()
            .and_then(|s| s.carried_items.as_ref())
        {
            for i in items {
                carried += i.quantity.value_or(1.0)
            }
        }
    }
    let budget = crate::generation::retained_loot_allocation(depth, &r.floors, hearts, carried)?;
    let mut excluded = r
        .generated_ability_grants
        .get_or_insert_with(Vec::new)
        .clone();
    excluded.extend(
        crate::progression::catalog_for_revision(r.simulation_revision.value_or(29.0))
            .iter()
            .filter(|a| r.gods.contains(&a.god_id))
            .map(|a| a.id.clone()),
    );
    let loot = crate::equipment::generate_floor_loot_for_run(
        r,
        depth,
        &mut p::population_random(&format!("{seed}:floor:{depth}:loot")),
        &excluded,
        budget["ordinaryLoot"] as usize,
    )?;
    // Keep the historical retained-slot ledger unchanged: early bonus density
    // must not reduce floors 5–6. Record actual scheduled drops separately.
    let early_loot_counts = if r.simulation_revision.value_or(29.) >= 51. && depth <= 4 {
        Some(["potion", "ring", "amulet"].map(|kind|
            loot.iter().filter(|item| item.kind == kind).count() as f64))
    } else { None };
    let reference_floor = crate::playtest::is_prior_floor(r, depth);
    let reference_loot = if reference_floor { loot.clone() } else { vec![] };
    let harvested_loot: HashSet<_> = reference_loot.iter().map(|i| i.id as u64).collect();
    r.generated_ability_grants.as_mut().unwrap().extend(
        loot.iter()
            .filter_map(|i| i.granted_ability.as_ref().cloned()),
    );
    let dais = f
        .setups
        .as_ref()
        .and_then(|s| s.iter().find(|s| s.id == 67.0))
        .map(|s| s.y as usize * w + s.x as usize + 4);
    let tap = f
        .godbeast_event
        .as_ref()
        .filter(|e| e.kind == "Veridian" && !r.veridian_sap_harvested.value_or(false))
        .and_then(|e| e.tap.as_ref())
        .map(|p| p.y as usize * w + p.x as usize);
    let heatmap = build_spawn_heatmap(&f);
    let land = heatmap.land.clone();
    let occupied: HashSet<_> = actors
        .iter()
        .map(|a| a.y as usize * w + a.x as usize)
        .chain(machines.reserved_cells.iter().copied())
        .chain(
            f.sphinx_room
                .as_ref()
                .into_iter()
                .flat_map(|s| s.cells.iter().map(|i| *i as usize)),
        )
        .collect();
    let mut sampler = SpawnSampler::new(
        heatmap,
        land,
        p::population_random(&format!("{seed}:floor:{depth}:loot-placement")),
        occupied,
    );
    for at in crate::pocket_biomes::reserved(&f) { sampler.reserve(at); }
    if let Some(at) = dais {
        sampler.reserve(at)
    }
    if let Some(at) = tap {
        sampler.reserve(at)
    }
    if r.simulation_revision.value_or(29.0) >= 34.0 {
        if let Some(at) = crate::phantasmii_generation::dagger_cell(&f) { sampler.reserve(at); }
    }
    let mut dais_assigned = false;
    for item in loot {
        let on_dais = !dais_assigned && dais.is_some() && item.kind != "potion";
        let at = if on_dais { dais } else { sampler.pick(None) }
            .ok_or("No legal space for generated loot")?;
        if on_dais {
            dais_assigned = true
        }
        place_generated(r, &mut f, &mut ground, item, at)?;
        sampler.reserve(at);
    }
    if arrow_bundle_floors(&seed, depth).contains(&depth) {
        let at = sampler
            .pick(None)
            .ok_or("No legal space for required arrows")?;
        let item = crate::equipment::make_arrows(r.next_id, 5.0);
        r.next_id += 1.0;
        place_generated(r, &mut f, &mut ground, item, at)?;
        sampler.reserve(at);
    }
    if let Some(at) = tap {
        let item = Item {
            id: r.next_id,
            name: "Tree tap".into(),
            glyph: "¬".into(),
            kind: "tool".into(),
            effect: "tree-tap".to_string().into(),
            identified: true.into(),
            ..Default::default()
        };
        r.next_id += 1.0;
        place_generated(r, &mut f, &mut ground, item, at)?;
    }
    let mut details = IndexMap::from([("items".into(), ground.len() as f64)]);
    details.extend(budget);
    if let Some([potions, rings, amulets]) = early_loot_counts {
        details.extend([
            ("ordinaryLootGenerated".into(), potions + rings + amulets),
            ("ordinaryPotionDrops".into(), potions),
            ("ordinaryRingDrops".into(), rings),
            ("ordinaryAmuletDrops".into(), amulets),
        ]);
    }
    crate::generation::record_generation_stage(&mut f, "items", Some(details))?;
    actors.extend(p::populate_machine_enemies_revision(
        &seed,
        &mut r.next_id,
        &mut f,
        &actors,
        &ground,
        r.simulation_revision.value_or(29.0),
    )?);
    let reference_ordinary_ids: HashSet<_> = if reference_floor && r.simulation_revision.value_or(29.0)>=37.0 { actors.iter().filter(|a|crate::playtest::ordinary_heart_actor(a)).map(|a|a.id as u64).collect() } else { HashSet::new() };
    let harvested_actors: HashSet<_> = if reference_floor && r.simulation_revision.value_or(29.0)<37.0 {
        let ordinary: Vec<_> = actors.iter().filter(|a| crate::playtest::ordinary_heart_actor(a)).cloned().collect();
        crate::playtest::collect_reference_floor(r, depth, &ordinary, &reference_loot)?;
        ordinary.iter().map(|a| a.id as u64).collect()
    } else { HashSet::new() };
    if let Some(room) = f.sphinx_room.as_ref().cloned() {
        let mut guardian = p::create_actor("sphinx", r.next_id, room.x, room.y, false)?;
        // Preserve the special room's legacy body, plus the explicit R52 birth baselines.
        guardian.stats.accuracy = crate::combat::creature_birth_accuracy(
            r.simulation_revision.value_or(29.0), guardian.stats.accuracy);
        guardian.stats.dodge = crate::combat::creature_birth_dodge(
            r.simulation_revision.value_or(29.0), guardian.stats.dodge);
        if r.simulation_revision.value_or(29.0) >= 50.0 {
            crate::revision50_creature_identity::apply_actor(&mut guardian);
        }
        r.next_id += 1.0;
        guardian.heartless = true.into();
        actors.push(guardian);
        let mut rng = p::population_random(&format!("{seed}:floor:{depth}:sphinx-rewards"));
        for n in 0..4 {
            let item = if n < 2 {
                crate::equipment::make_godbeast_permanent_for_run(r, depth, &mut rng)?
            } else {
                let effect = if n == 2 { "healing" } else { "energy" };
                Item {
                    id: r.next_id,
                    name: format!("potion of {effect}"),
                    kind: "potion".into(),
                    glyph: "!".into(),
                    effect: effect.to_string().into(),
                    identified: false.into(),
                    ..Default::default()
                }
            };
            r.next_id += 1.0;
            place_generated(r, &mut f, &mut ground, item, room.reward_cells[n] as usize)?;
        }
    }
    if let Some(e) = f.godbeast_event.as_ref().filter(|e| e.kind == "Zillennia") {
        let mut a = p::zillennia_revision(depth, r.simulation_revision.value_or(29.0))?;
        a.id = r.next_id;
        r.next_id += 1.0;
        a.x = e.x;
        a.y = e.y;
        actors.push(a);
    }
    if reference_floor && r.simulation_revision.value_or(29.0)<37.0 {
        actors.retain(|a| !harvested_actors.contains(&(a.id as u64)));
        ground.retain(|g| !harvested_loot.contains(&(g.item.id as u64)));
    }
    if r.simulation_revision.value_or(29.0)>=33.0 {
        crate::pocket_biomes::populate_revision(&mut f,&mut actors,&mut ground,&mut r.next_id,r.simulation_revision.value_or(29.0))?;
    }
    crate::finale::populate(r, &mut f, &mut actors, &ground)?;
    if r.simulation_revision.value_or(29.0) >= 34.0 {
        crate::ahvamitt::populate_revision(&f, &mut actors, &mut r.next_id, r.simulation_revision.value_or(29.0));
        crate::phantasmii_generation::populate_revision(&seed, &mut f, &mut actors, &mut ground, &mut r.next_id, r.simulation_revision.value_or(29.0))?;
    }
    crate::sub_biomes_generation::decorate(&seed, &mut f, &mut actors, &ground, &mut r.next_id, r.simulation_revision.value_or(29.0))?;
    if reference_floor && r.simulation_revision.value_or(29.0)>=37.0 {
        let ordinary: Vec<_> = actors.iter().filter(|a|reference_ordinary_ids.contains(&(a.id as u64)) || a.conditions.contains_key("cornDimensionResident")).cloned().collect();
        crate::playtest::collect_reference_floor(r,depth,&ordinary,&reference_loot)?;
        let harvested: HashSet<_> = ordinary.iter().map(|a|a.id as u64).collect();
        actors.retain(|a|!harvested.contains(&(a.id as u64)));
        ground.retain(|g|!harvested_loot.contains(&(g.item.id as u64)));
    }
    crate::portals_generation::install(&seed, &mut f, &actors, &ground, r.simulation_revision.value_or(29.0));
    crate::non_euclidean_generation::install(&seed, &mut f, &actors, &ground, r.simulation_revision.value_or(29.0));
    if r.simulation_revision.value_or(29.0)>=52.0 {
        crate::square_currents::generate(&mut f, crate::rng::seed_hash(&seed));
    } else if r.simulation_revision.value_or(29.0)>=40.0 {crate::shore_currents::repair(&mut f);}
    crate::sub_biomes_generation::install_links(&seed, &mut f, &actors, &ground, r.simulation_revision.value_or(29.0));
    refresh_razorleaves(&mut f, &actors);
    if r.simulation_revision.value_or(29.0) >= 52.0 {
        let cells = match crate::revision52_swimming::placement_cells(&f, &actors, &ground) {
            Ok(cells) => cells,
            Err(error) => {
                #[cfg(test)]
                export_island_failure(&seed, &f, &actors, &ground, &error);
                return Err(error);
            }
        };
        if let Some(cleanup) = f.generation.as_mut().and_then(|g|g.stages.iter_mut().find(|s|s.id == "cleanup")) {
            cleanup.details.get_or_insert_with(Default::default)
                .insert("islandSwimmingRings".into(), cells.len() as f64);
        }
        for at in cells {
            let ring = crate::revision52_swimming::make_ring(r.next_id);
            r.next_id += 1.0;
            place_generated(r, &mut f, &mut ground, ring, at)?;
        }
    }
    if r.simulation_revision.value_or(29.0) >= 38.0 {
        validate_complete_floor_transitions(&f)?;
    }
    if !reference_floor { crate::verdania_guard::validate(&f,&actors)?; }
    f.dirty = false;
    f.last_tick = 0.0;
    r.floors[fi] = Arc::new(f);
    r.actors.insert(depth.to_string(), Arc::new(actors));
    crate::water_blood::register_kelpie_sources(r, depth as f64);
    r.ground.insert(depth.to_string(), Arc::new(ground));
    crate::zillennia_ground::initialize_floor(r, depth as f64);
    // Initial contact is generation, not activation of an unseen floor's clock.
    crate::mutation_journal::make_mut(&mut r.floors[fi]).dirty = false;
    Ok(())
}

/// Last acceptance gate after population and every terrain/topology decorator.
/// O(V + fixtures) time and O(V) scratch; mirrors the geometry safety invariant.
fn validate_complete_floor_transitions(f: &Floor) -> Result<(), String> {
    let reachable = crate::generation::reachable_without_braziers(f, None);
    if !crate::generation::transitions_accessible(f, &reachable) {
        return Err("Generated floor has no safe route between transitions".into());
    }
    Ok(())
}

/// Keep source allocation IDs and legacy payloads; R50 has no unusable sword.
/// O(1) fixed item creation, no RNG. The caller keeps next_id=4 for compatibility.
pub(crate) fn starting_inventory(revision: f64) -> Result<Vec<Item>, String> {
    let mut items = vec![
            Item {
                id: 1.0,
                name: "Shortsword".into(),
                kind: "weapon".into(),
                glyph: ")".into(),
                damage: 30.0.into(),
                hands: 1.0.into(),
                equipped: "hand1".to_string().into(),
                ..Default::default()
            },
            crate::equipment::make_equipment("Bow", 2.0).map_err(|e| e.0)?,
            crate::equipment::make_arrows(3.0, 5.0),
        ];
    if revision >= 50.0 { items.retain(|item| item.id != 1.0); }
    Ok(items)
}

/// Historical/fixture construction with exact seed normalization and starter state.
/// Keep all recorded modes for save/replay compatibility; new playable runs use
/// `create_avatar_run`. No geometry or simulation is fabricated here.
pub fn new_run_shell(seed: &str, mode: &str, dev: bool) -> Result<Run, String> {
    new_run_shell_revision(seed, mode, dev, 39.0)
}
pub fn new_run_shell_revision(
    seed: &str,
    mode: &str,
    dev: bool,
    revision: f64,
) -> Result<Run, String> {
    new_run_shell_revision_policy(seed, mode, dev, revision, &crate::Field::Missing)
}
pub(crate) fn new_run_shell_revision_policy(seed: &str, mode: &str, dev: bool, revision: f64, starter_policy: &crate::Field<u8>) -> Result<Run,String> {
    if !starter_policy.is_missing() && (!matches!(starter_policy,crate::Field::Value(1)) || mode!="Avatar" || revision!=52.) {
        return Err("Invalid Avatar starter policy".into());
    }
    if ![29.0, 30.0, 31.0, 32.0, 33.0, 34.0, 35.0, 36.0, 37.0, 38.0, 39.0, 40.0, 41.0, 42.0, 43.0, 44.0, 45.0, 46.0, 50.0, 51.0, 52.0].contains(&revision) {
        return Err("Unsupported simulation revision".into());
    }
    if !["Avatar", "Blessed", "Unblessed"].contains(&mode) {
        return Err("Unknown mode".into());
    }
    let prefix = seed.split_once(':').and_then(|(p, _)| {
        let parts: Vec<_> = p.split('-').collect();
        if parts.len() == 3
            && parts
                .iter()
                .all(|s| !s.is_empty() && s.len() <= 2 && s.bytes().all(|b| b.is_ascii_digit()))
        {
            Some(
                parts
                    .iter()
                    .map(|s| s.parse::<u32>().unwrap())
                    .collect::<Vec<_>>(),
            )
        } else {
            None
        }
    });
    let numeric = prefix.is_some();
    let mut ids = prefix.unwrap_or_default();
    let mut h = crate::rng::seed_hash(seed);
    while ids.len() < 3 {
        h = h.wrapping_mul(1664525).wrapping_add(1013904223);
        let id = h % 12;
        if !ids.contains(&id) {
            ids.push(id)
        }
    }
    if ids.iter().any(|id| *id > 11) || ids.iter().collect::<HashSet<_>>().len() != 3 {
        return Err("Seed prefix must contain three distinct god IDs 0–11.".into());
    }
    let full_seed = if numeric {
        seed.into()
    } else {
        format!("{}-{}-{}:{seed}", ids[0], ids[1], ids[2])
    };
    let player = Actor {
        id: 0.0,
        x: 2.0,
        y: 2.0,
        name: "You".into(),
        glyph: "@".into(),
        color: "#fff0c9".into(),
        hp: 100.0,
        max_hp: 100.0,
        energy: 100.0,
        stats: crate::Stats {
            accuracy: if revision >= 52. { 15. } else { 0. },
            dodge: if revision >= 52. { 15. } else { 0. },
            perception: if revision >= 51. {0.} else {300.},
            max_energy: 100.0,
            ..Default::default()
        },
        state: "ally".into(),
        reactions: 0.0.into(),
        blood: 5.0.into(),
        nightmares: 0.0.into(),
        ..Default::default()
    };
    let mut r = Run {
        version: 1.0,
        gameplay_revision: 3.0.into(),
        divine_revision: 1.0.into(),
        enemy_health_revision: 1.0.into(),
        simulation_revision: revision.into(),
        amulet_rules_from: if revision >= 52. { 0usize.into() } else { crate::Field::Missing },
        potion_rules_from: if revision >= 52. { 0usize.into() } else { crate::Field::Missing },
        ring_rules_from: if revision >= 52. { 0usize.into() } else { crate::Field::Missing },
        gas_rules_from: if revision >= 52. { 0usize.into() } else { crate::Field::Missing },
        screamer_schedule: None,
        commandment_rules_from: if revision >= 50. { 0usize.into() } else { crate::Field::Missing },
        screamer_rules_from: if revision >= 50. { 0usize.into() } else { crate::Field::Missing },
        ember_rules_from: if revision >= 50. { 0usize.into() } else { crate::Field::Missing },
        ragelight_rules_from: if revision >= 52. { 0usize.into() } else { crate::Field::Missing },
        balance_rules_from: if revision >= 52. { 0usize.into() } else { crate::Field::Missing },
        steam_rules_from: if revision >= 50. { 0usize.into() } else { crate::Field::Missing },
        execute_rules_from: if revision >= 52. { 0usize.into() } else { crate::Field::Missing },
        beckoning_rules_from: if revision >= 52. { 0usize.into() } else { crate::Field::Missing },
        ichor_fire_rules_from: if revision >= 52. { 0usize.into() } else { crate::Field::Missing },
        future_floor_rules_from: if revision >= 52. { 0usize.into() } else { crate::Field::Missing },
        future_floor: None,
        firebolt_travel_from: if revision >= 52. { 0usize.into() } else { crate::Field::Missing },
        composite_bow_generated: if revision >= 51. { false.into() } else { crate::Field::Missing },
        rng: crate::rng::seed_hash(&full_seed) as f64,
        seed: full_seed,
        mode: mode.into(),
        gods: if mode == "Avatar" {
            ids.iter().map(|i| *i as f64).collect()
        } else if mode == "Blessed" {
            vec![ids[0] as f64]
        } else {
            vec![]
        },
        depth: 1.0,
        level: 1.0,
        run_stats: crate::RunStatistics {
            version: 1.0,
            blessings_complete: true,
            hearts_complete: true,
            ..Default::default()
        }
        .into(),
        player,
        inventory: starting_inventory(revision)?,
        log_kinds: vec![].into(),
        log_archive: crate::LogArchive {
            version: 1.0,
            ..Default::default()
        }
        .into(),
        next_id: 4.0,
        status: "playing".into(),
        metrics: IndexMap::from([
            ("commands".into(), 0.0),
            ("aiActors".into(), 0.0),
            ("sightTests".into(), 0.0),
        ]),
        dev,
        ..Default::default()
    };
    for god in r.gods.clone() {
        let pool = crate::progression::starter_abilities_for_revision(god as u32, revision)?;
        let index = (crate::rng::random(&mut r) * pool.len() as f64).floor() as usize;
        r.player.abilities.push(pool[index].clone());
    }
    if matches!(starter_policy,crate::Field::Value(1)) {
        // Preserve the legacy three global draws; isolated selection cannot shift
        // procedural generation solely because its rejection count differs.
        r.player.abilities=crate::avatar_starters::choose(&r.seed,&r.gods)?;
        r.avatar_starter_policy=starter_policy.clone();
    }
    crate::revision50_ability_energy::reconcile_acquired(&mut r)
        .map_err(|e| format!("Starter blessing reserve: {e:?}"))?;
    Ok(r)
}
/// Native revision38 retries complete, atomic floor candidates. The bound is
/// independent of geometry's own 32-candidate quality search.
pub const COMPLETE_FLOOR_ATTEMPT_LIMIT: u32 = 8;

/// Placement/packing failures may depend on geometry. Invalid models, unknown
/// catalogs, unsupported revisions and programming invariants must still fail.
fn retryable_generation_error(error: &str) -> bool {
    error.starts_with("Structural generation quality exhausted:")
        || error.starts_with("Required generated item invalid/occupied on floor")
        || error.starts_with("Isolated island")
        || error == "No guarded Verdania tap location"
        || [
            "Generated floor has no safe route between transitions",
            "No legal space for generated loot",
            "No legal space for required arrows",
            "No space for complete horde",
            "No safe inventor mech location",
            "Required setup creature cannot occupy coordinate",
            "Required machine reservation occupied",
            "Required setup overlaps permanent fire",
            "Machine hearts leave insufficient ordinary horde budget",
            "No balanced exact ordinary horde budget",
            "Phantasmii dagger has no clear adjacent altar tile",
            "Insufficient reachable cells for required dream population",
            "Administrator floor has no safe reachable boss position",
            "Nest reserved creature cell occupied",
            "Nest honey cell occupied",
            "Floor-one entrance conflicts with an authored fixture",
            "Floor-one entrance conflicts with unsupported lighting",
            "Floor-one entrance conflicts with a protected fixture",
            "Floor-one entrance cannot safely reach its generated room",
        ].contains(&error)
}

pub fn ensure_floors_with(
    r: &mut Run,
    to: u32,
    generate: &impl Fn(u32, u32, Option<crate::generation::Point>) -> Result<Floor, String>,
) -> Result<(), String> {
    // A supplied generator owns its contract: never silently call it again or
    // reinterpret invalid external fixtures as random packing failures.
    ensure_floors_attempts(r, to, 1, &|depth, seed, up, _| generate(depth, seed, up))
}

/// O(F*K*(G+P) + T_index) time, with K<=8, geometry G bounded to32 candidates,
/// T_index=O((A+pilots+B)*d+F+V) identity reconciliation after population.
/// population P documented in population.rs; one draft run and candidate extra.
/// Arc-backed prior floors/actors/loot remain shared until a trial mutates them.
fn ensure_floors_attempts(
    r: &mut Run,
    to: u32,
    attempts: u32,
    generate: &impl Fn(u32, u32, Option<crate::generation::Point>, u32) -> Result<Floor, String>,
) -> Result<(), String> {
    crate::creature_index::check_player(r)?;
    let mut draft = r.clone();
    let mut depth = draft.floors.last().map_or(if crate::fast_mode::enabled(&draft) { draft.depth as u32 } else { 1 }, |f| crate::fast_mode::below(&draft, f.depth) as u32);
    while depth <= 40.min(to) {
        let previous_depth = crate::fast_mode::above(&draft, depth as f64);
        let actual_up = if depth == 1 && draft.simulation_revision.value_or(29.0) >= 31.0 {
            Some(crate::entrance::generation_anchor())
        } else { draft
            .floors
            .iter()
            .find(|f| f.depth == previous_depth)
            .and_then(|f| {
                f.tiles
                    .iter()
                    .position(|t| t == "down")
                    .map(|at| crate::generation::Point {
                        x: (at % f.width()) as i32,
                        y: (at / f.width()) as i32,
                    })
            }) };
        let mut accepted = None;
        let mut last_error = String::new();
        for attempt in 0..attempts {
            let mut candidate = draft.clone();
            let result = (|| {
                let mut f = generate(depth, crate::rng::seed_hash(&candidate.seed), actual_up, attempt)?;
                if candidate.simulation_revision.value_or(29.0) >= 31.0 && depth == 1 {
                    crate::entrance::install_floor_one_entrance(&mut f)?;
                }
                candidate.floors.push(Arc::new(f));
                spawn_floor_inner(&mut candidate, depth)?;
                if attempts > 1 {
                    if let Some(stage) = crate::mutation_journal::make_mut(candidate.floors.last_mut().unwrap())
                        .generation.as_mut().and_then(|g| g.stages.iter_mut().find(|s| s.id == "cleanup")) {
                        stage.details.get_or_insert_with(Default::default)
                            .insert("populationAttempt".into(), attempt as f64);
                    }
                }
                Ok::<(), String>(())
            })();
            match result {
                Ok(()) => { accepted = Some(candidate); break; }
                Err(error) => {
                    if attempts == 1 || !retryable_generation_error(&error) { return Err(error); }
                    last_error = error;
                }
            }
        }
        draft = accepted.ok_or_else(|| format!(
            "Generation failed after {attempts} complete floor attempts on floor {depth}: {last_error}"
        ))?;
        depth = crate::fast_mode::below(&draft, depth as f64) as u32;
    }
    draft.actors.validate_allocator(draft.player.id,draft.depth,draft.next_id)?;
    draft.actors.adopt_epoch_from(&r.actors);
    *r = draft;
    Ok(())
}
pub fn create_run_with(
    seed: &str,
    mode: &str,
    dev: bool,
    generate: &impl Fn(u32, u32, Option<crate::generation::Point>) -> Result<Floor, String>,
) -> Result<Run, String> {
    create_run_with_revision(seed, mode, dev, generate, 39.0)
}
pub fn create_run_with_revision(
    seed: &str,
    mode: &str,
    dev: bool,
    generate: &impl Fn(u32, u32, Option<crate::generation::Point>) -> Result<Floor, String>,
    revision: f64,
) -> Result<Run, String> {
    let mut r = new_run_shell_revision(seed, mode, dev, revision)?;
    let to = initial_generation_target(revision, 1);
    ensure_floors_with(&mut r, to, generate)?;
    finish_new_run(r, revision)
}
fn finish_new_run(mut r: Run, revision: f64) -> Result<Run, String> {
    let f = &r.floors[0];
    let at = f
        .tiles
        .iter()
        .position(|t| t == "up")
        .ok_or("Generated floor has no entrance")?;
    r.player.x = (at % f.width()) as f64;
    r.player.y = (at / f.width()) as f64;
    if let Some(arrival) = crate::entrance::arrival(f) {
        r.player.x = arrival.x as f64;
        r.player.y = arrival.y as f64;
    }
    let opening = if revision >= 31.0 {
        "You begin to adventure down to end the corruption of the Flesh."
    } else { "The descent begins. Find the way below." };
    crate::engine::log(&mut r, opening, "environment");
    crate::engine::log(
        &mut r,
        "Creature and loot tuning is provisional and documented.",
        "environment",
    );
    for id in r.player.abilities.clone() {
        crate::progression::initialize_ability_state_for_revision(r.simulation_revision.value_or(29.), &mut r.player, &id, r.tick)?;
    }
    crate::engine::monster_floor_entered(&mut r);
    crate::engine::monster_prepare_player_turn(&mut r).map_err(|e| e.0)?;
    r.initial_player = r.player.clone().into();
    crate::engine::remember(&mut r);
    Ok(r)
}
/// Current playable startup is Avatar-only with twenty displayed floors.
/// Authored content coordinates remain internal; route and rewards are explicit.
/// Historical replays must use their explicit revision/mode constructor instead.
pub fn create_avatar_run(seed: &str) -> Result<Run, String> {
    create_avatar_run_replay_policies(seed, &0usize.into(), &0usize.into(), &0usize.into(), &0usize.into(), &0usize.into(), &crate::Field::Value(crate::avatar_starters::POLICY))
}
pub(crate) fn create_avatar_run_replay_policies(seed: &str, policy: &crate::Field<usize>, potion_policy: &crate::Field<usize>, ring_policy: &crate::Field<usize>, gas_policy: &crate::Field<usize>, future_policy: &crate::Field<usize>, starter_policy: &crate::Field<u8>) -> Result<Run, String> {
    let mut r = new_run_shell_revision_policy(seed, "Avatar", false, 52., starter_policy)?;
    crate::amulet_rules::apply_replay_policy(&mut r, policy, 0);
    crate::potion_rules::apply_replay_policy(&mut r, potion_policy, 0);
    crate::ring_rules::apply_replay_policy(&mut r, ring_policy, 0);
    crate::gas_rules::apply_replay_policy(&mut r, gas_policy, 0);
    crate::future_floor::apply_replay_policy(&mut r, future_policy, 0);
    r.fast_mode = true.into();
    r.compressed_depths = true.into();
    let to = crate::fast_mode::arrival_target(&r, r.depth as u32);
    ensure_floors(&mut r, to)?;
    finish_new_run(r, 52.)
}
/// Historical Fast startup is retained only for old seed replays.
pub fn create_avatar_run_fast(seed: &str) -> Result<Run, String> {
    create_avatar_run_fast_replay_policies(seed, &0usize.into(), &0usize.into(), &0usize.into(), &0usize.into(), &0usize.into(), &crate::Field::Missing)
}
pub(crate) fn create_avatar_run_fast_replay_policies(seed: &str, policy: &crate::Field<usize>, potion_policy: &crate::Field<usize>, ring_policy: &crate::Field<usize>, gas_policy: &crate::Field<usize>, future_policy: &crate::Field<usize>, starter_policy: &crate::Field<u8>) -> Result<Run, String> {
    let mut r = new_run_shell_revision_policy(seed, "Avatar", false, 52., starter_policy)?;
    crate::amulet_rules::apply_replay_policy(&mut r, policy, 0);
    crate::potion_rules::apply_replay_policy(&mut r, potion_policy, 0);
    crate::ring_rules::apply_replay_policy(&mut r, ring_policy, 0);
    crate::gas_rules::apply_replay_policy(&mut r, gas_policy, 0);
    crate::future_floor::apply_replay_policy(&mut r, future_policy, 0);
    r.fast_mode = true.into();
    let to = crate::fast_mode::arrival_target(&r, r.depth as u32);
    ensure_floors(&mut r, to)?;
    finish_new_run(r, 52.)
}

/// Historical seed-only startup uses exact procedural geometry and population.
/// Its default revision and mode argument remain available for compatibility;
/// user-facing new-game entry points must use `create_avatar_run`.
pub fn create_run(seed: &str, mode: &str, dev: bool) -> Result<Run, String> {
    create_run_revision(seed, mode, dev, 39.0)
}
pub fn create_run_revision(
    seed: &str,
    mode: &str,
    dev: bool,
    revision: f64,
) -> Result<Run, String> {
    create_run_revision_replay_policies(seed, mode, dev, revision, &0usize.into(), &0usize.into(), &0usize.into(), &0usize.into(), &0usize.into(), &crate::Field::Missing)
}
pub(crate) fn create_run_revision_replay_policies(seed: &str, mode: &str, dev: bool, revision: f64, policy: &crate::Field<usize>, potion_policy: &crate::Field<usize>, ring_policy: &crate::Field<usize>, gas_policy: &crate::Field<usize>, future_policy: &crate::Field<usize>, starter_policy: &crate::Field<u8>) -> Result<Run, String> {
    let mut r = new_run_shell_revision_policy(seed, mode, dev, revision, starter_policy)?;
    crate::amulet_rules::apply_replay_policy(&mut r, policy, 0);
    crate::potion_rules::apply_replay_policy(&mut r, potion_policy, 0);
    crate::ring_rules::apply_replay_policy(&mut r, ring_policy, 0);
    crate::gas_rules::apply_replay_policy(&mut r, gas_policy, 0);
    crate::future_floor::apply_replay_policy(&mut r, future_policy, 0);
    let to = initial_generation_target(revision, 1);
    ensure_floors(&mut r, to)?;
    finish_new_run(r, revision)
}
/// Current startup keeps the active floor plus one below it. DEBUG retains
/// prior floors needed for reconstruction. Existing saves are never pruned.
pub fn initial_generation_target(revision: f64, depth: u32) -> u32 {
    if revision >= 52. { depth.saturating_add(1).min(40) } else { depth.max(10) }
}

/// Speculative player-arrival horizon only. Explicit physical destinations and
/// authored generation sweeps still call ensure_floors with their exact target.
pub fn arrival_generation_target(revision: f64, depth: u32) -> u32 {
    if revision >= 52. { depth.saturating_add(1).min(40) }
    else { ((((depth as f64 + 5.) / 5.).ceil() * 5.).min(40.)) as u32 }
}

pub fn ensure_floors(r: &mut Run, to: u32) -> Result<(), String> {
    let revision = r.simulation_revision.value_or(29.0);
    if crate::fast_mode::enabled(r) {
        if revision < 52. { return Err("Fast mode requires revision52".into()); }
        let compressed = crate::fast_mode::compressed(r);
        return ensure_floors_attempts(r, to, COMPLETE_FLOOR_ATTEMPT_LIMIT, &|depth, seed, actual_up, attempt| {
            let previous = if depth == 1 { 0 } else if compressed { crate::fast_mode::content_depth((depth + 1) / 2 - 1) } else if depth == 40 { 39 } else { depth - 2 };
            crate::generation::generate_fast_floor_from_attempt(depth, seed, actual_up, revision,
                attempt * crate::generation::GENERATION_CANDIDATE_LIMIT,
                crate::fast_mode::boss_depth(previous, depth))
        });
    }
    if revision < 38.0 {
        return ensure_floors_with(r, to, &|depth, seed, actual_up|
            crate::generation::generate_floor_full_revision(depth, seed, actual_up, revision));
    }
    ensure_floors_attempts(r, to, COMPLETE_FLOOR_ATTEMPT_LIMIT, &|depth, seed, actual_up, attempt|
        crate::generation::generate_floor_full_revision_from_attempt(
            depth, seed, actual_up, revision,
            attempt * crate::generation::GENERATION_CANDIDATE_LIMIT))
}

#[cfg(test)]
mod recovery_tests_r38 {
    use super::*;
    use std::cell::Cell;

    fn initial() -> Run {
        let mut run = new_run_shell_revision("0-1-2:population-recovery-r38", "Unblessed", false, 38.).unwrap();
        ensure_floors(&mut run, 1).unwrap();
        run
    }
    fn generated(depth: u32, seed: u32, up: Option<crate::generation::Point>) -> Floor {
        crate::generation::generate_floor_full_revision(depth, seed, up, 38.).unwrap()
    }
    fn invalid_tap(mut floor: Floor) -> Floor {
        let at = floor.tiles.iter().position(|t| t == "up").unwrap();
        floor.godbeast_event = crate::GodbeastEvent {
            kind: "Veridian".into(),
            tap: crate::GodbeastEventTap { x: (at % floor.width()) as f64, y: (at / floor.width()) as f64 }.into(),
            ..Default::default()
        }.into();
        floor
    }
    fn without_attempt(mut run: Run) -> serde_json::Value {
        for floor in &mut run.floors {
            if let Some(g) = crate::mutation_journal::make_mut(floor).generation.as_mut() {
                for stage in &mut g.stages {
                    if let Some(details) = stage.details.as_mut() { details.shift_remove("populationAttempt"); }
                }
            }
        }
        serde_json::to_value(run).unwrap()
    }

    #[test]
    fn failed_population_trial_leaves_no_ids_metrics_loot_or_ability_grants() {
        let before = initial();
        let mut expected = before.clone();
        ensure_floors_attempts(&mut expected, 2, 8, &|d,s,u,_| Ok(generated(d,s,u))).unwrap();
        let mut actual = before;
        let calls = Cell::new(0);
        ensure_floors_attempts(&mut actual, 2, 8, &|d,s,u,attempt| {
            calls.set(calls.get()+1);
            let f=generated(d,s,u);
            Ok(if attempt == 0 { invalid_tap(f) } else { f })
        }).unwrap();
        assert_eq!(calls.get(),2);
        let cleanup=actual.floors[1].generation.as_ref().unwrap().stages.iter().find(|s|s.id=="cleanup").unwrap();
        assert_eq!(cleanup.details.as_ref().unwrap()["populationAttempt"],1.);
        assert_eq!(without_attempt(actual),without_attempt(expected));
    }

    #[test]
    fn finite_exhaustion_and_fatal_failure_are_atomic() {
        let before=initial(); let mut run=before.clone(); let calls=Cell::new(0);
        let error=ensure_floors_attempts(&mut run,2,8,&|d,s,u,_| {
            calls.set(calls.get()+1); Ok(invalid_tap(generated(d,s,u)))
        }).unwrap_err();
        assert_eq!(calls.get(),8);
        assert!(error.starts_with("Generation failed after 8 complete floor attempts on floor 2:"));
        assert_eq!(serde_json::to_value(&run).unwrap(),serde_json::to_value(&before).unwrap());
        calls.set(0);
        let error=ensure_floors_attempts(&mut run,2,8,&|_,_,_,_| {
            calls.set(calls.get()+1); Err("Unknown creature invalid-catalog-entry".into())
        }).unwrap_err();
        assert_eq!(calls.get(),1); assert_eq!(error,"Unknown creature invalid-catalog-entry");
        assert_eq!(serde_json::to_value(&run).unwrap(),serde_json::to_value(before).unwrap());
    }

    #[test]
    fn custom_generators_keep_single_attempt_contract() {
        let mut run=initial(); let before=serde_json::to_value(&run).unwrap(); let calls=Cell::new(0);
        assert_eq!(ensure_floors_with(&mut run,2,&|_,_,_| {
            calls.set(calls.get()+1); Err("No space for complete horde".into())
        }).unwrap_err(),"No space for complete horde");
        assert_eq!(calls.get(),1); assert_eq!(serde_json::to_value(run).unwrap(),before);
    }

    #[test]
    fn final_safety_gate_rejects_hazard_only_transition_routes() {
        let mut f = Floor { depth: 2., tiles: vec!["wall".into(); 2291], ..Default::default() };
        for x in 3..=12 { f.tiles[8*79+x]="floor".into(); }
        f.tiles[8*79+3]="up".into(); f.tiles[8*79+12]="down".into();
        validate_complete_floor_transitions(&f).unwrap();
        f.permanent_fire=vec![(8*79+7) as f64].into();
        let error=validate_complete_floor_transitions(&f).unwrap_err();
        assert_eq!(error,"Generated floor has no safe route between transitions");
        assert!(retryable_generation_error(&error));
        let mut run=initial(); let calls=Cell::new(0);
        ensure_floors_attempts(&mut run,2,8,&|d,s,u,attempt| {
            calls.set(calls.get()+1);
            if attempt==0 { Err(error.clone()) } else { Ok(generated(d,s,u)) }
        }).unwrap();
        assert_eq!(calls.get(),2);
    }

    #[test]
    fn retry_classification_does_not_mask_configuration_or_invariants() {
        for error in ["Unsupported generation revision","Invalid ordinary loot slot count", "Unknown Godbeast kind", "No feasible horde composition", "Required setup exceeds maximum machine group size", "Generation stage out of order: cleanup -> cleanup", "Dream population already generated"] {
            assert!(!retryable_generation_error(error),"{error}");
        }
        for error in ["No space for complete horde", "Phantasmii dagger has no clear adjacent altar tile", "Required generated item invalid/occupied on floor20", "Structural generation quality exhausted: Forest floor 20 seed 0: width"] {
            assert!(retryable_generation_error(error),"{error}");
        }
    }
}

/// Opt-in test diagnostic only. Production builds contain no export code.
/// Disabled tests only check the environment flag; enabled exports cost O(J)
/// serialization/extra memory and O(J) filesystem writes for J JSON bytes.
/// At most eight final candidates are considered for the selected floor.
#[cfg(test)]
fn export_island_failure(seed: &str, floor: &Floor, actors: &[Actor], ground: &[GroundItem], error: &str) {
    use std::io::Write;
    let Ok(dir) = std::env::var("COTCG_ISLAND_FAILURE_EXPORT_DIR") else { return; };
    if floor.depth != 37. { return; }
    let directory=std::path::Path::new(&dir);
    assert!(directory.is_absolute(),"Use an explicit absolute diagnostic export directory");
    let attempt=floor.generation.as_ref().and_then(|g|g.stages.iter().find(|s|s.id=="cleanup"))
        .and_then(|s|s.details.as_ref()).and_then(|d|d.get("generationAttempt")).copied().unwrap_or(0.);
    std::fs::create_dir_all(directory).expect("diagnostic export directory");
    let path=directory.join(format!("seed-{}-floor37-attempt{}.json",crate::rng::seed_hash(seed),attempt));
    let mut output=match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(file)=>file,
        Err(e) if e.kind()==std::io::ErrorKind::AlreadyExists=> {
            eprintln!("ISLAND_FAILURE_EXPORT retained existing {}; use a fresh directory for a new capture",path.display());return;
        }
        Err(e)=>panic!("Diagnostic candidate export {}: {e}",path.display()),
    };
    let value=serde_json::json!({"seed":seed,"generationAttempt":attempt,"error":error,"floor":floor,"actors":actors,"ground":ground});
    output.write_all(&serde_json::to_vec_pretty(&value).unwrap()).expect("diagnostic candidate export");
    eprintln!("ISLAND_FAILURE_EXPORT {}: {}",path.display(),error);
}

#[cfg(test)]
#[test]
#[ignore = "Opt-in candidate capture; set COTCG_ISLAND_FAILURE_EXPORT_DIR to a fresh absolute directory"]
fn export_rejected_swimming_candidate() {
    let directory=std::env::var("COTCG_ISLAND_FAILURE_EXPORT_DIR")
        .expect("Set COTCG_ISLAND_FAILURE_EXPORT_DIR explicitly to run this diagnostic");
    assert!(std::path::Path::new(&directory).is_absolute());
    let mut r=new_run_shell_revision("0-1-2:swimming-survey-b","Avatar",false,52.).unwrap();
    ensure_floors(&mut r,40).expect("Diagnostic generation failed; rejected final candidates were exported");
    assert_eq!(r.floors.len(),40);
    eprintln!("DIAGNOSTIC_GENERATION_ACCEPTED seed={} floors={}",r.seed,r.floors.len());
}

#[cfg(test)]
mod verdania_production_tests {
    use super::*;
    #[test]
    fn populated_guard_sweep_all_biomes_and_depths() {
        let mut retries=0;
        for domain in ["Dungeon","Cave","Meadow","Forest","Archipelago","Mountain"] {
            for depth in [5u32,25,35,37] {
                let mut r=new_run_shell_revision(&format!("0-1-2:guard-sweep-{domain}-{depth}"),"Avatar",false,52.).unwrap();
                r.fast_mode=true.into();r.compressed_depths=true.into();r.depth=depth as f64;
                ensure_floors_attempts(&mut r,depth,COMPLETE_FLOOR_ATTEMPT_LIMIT,&|d,seed,up,attempt|{
                    let mut f=crate::generation::generate_floor_revision(d,seed.wrapping_add(attempt*1031),domain,false,79,up,52.)?;
                    let at=f.tiles.iter().position(|t|t=="floor").unwrap();
                    f.godbeast=true;
                    f.godbeast_event=crate::GodbeastEvent{kind:"Veridian".into(),display_name:"Verdania".to_string().into(),x:(at%f.width()) as f64,y:(at/f.width()) as f64,..Default::default()}.into();
                    Ok(f)
                }).unwrap_or_else(|e|panic!("{domain} {depth}: {e}"));
                let f=r.floors.last().unwrap();
                crate::verdania_guard::validate(f,r.actors_at(depth as f64)).unwrap();
                let attempt=f.generation.as_ref().unwrap().stages.iter().find(|s|s.id=="cleanup").unwrap().details.as_ref().unwrap()["populationAttempt"];
                retries+=attempt as usize;
                eprintln!("VERDANIA_PRODUCTION domain={domain} depth={depth} accepted_attempt={attempt}");
                assert_eq!(r.ground[&depth.to_string()].iter().filter(|g|g.item.effect.as_ref().map(String::as_str)==Some("tree-tap")).count(),1);
                assert!(crate::population::validate_encounter_metadata(&f.encounters));
            }
        }
        eprintln!("VERDANIA_PRODUCTION 24 populated cases; total retries={retries}");
    }
}
