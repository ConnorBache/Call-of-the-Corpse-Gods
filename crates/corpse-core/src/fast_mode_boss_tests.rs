//! Production Fast startup/population regressions, not synthetic boss metadata.
//! Each fixture uses the ordinary seed plan and all intervening retained floors.
use crate::{Actor, Command, GroundItem, Run, generation, startup, storage};

const SCHEDULE: [u32; 7] = [5, 10, 15, 20, 25, 30, 35];
const ARRIVALS: [u32; 7] = [5, 11, 15, 21, 25, 31, 35];

fn strict_round_trip(r: &Run, stage: &str) -> Run {
    let wire = storage::serialize(r).unwrap();
    let loaded = storage::deserialize(&wire)
        .unwrap_or_else(|error| panic!("{} depth {} {stage}: strict load: {error}", r.seed, r.depth));
    assert!(storage::serialize(&loaded).unwrap() == wire,
        "{} depth {} {stage}: strict load changed authoritative state", r.seed, r.depth);
    loaded
}

fn assert_body(actual: &Actor, expected: &Actor) {
    assert_eq!(actual.name, expected.name);
    assert_eq!(actual.creature_kind, expected.creature_kind);
    assert_eq!(actual.zillennia_quality, expected.zillennia_quality);
    assert_eq!(actual.stats, expected.stats);
    assert_eq!(actual.max_hp, expected.max_hp);
    assert_eq!(actual.hp, expected.hp);
    assert_eq!(actual.energy, expected.energy);
    assert_eq!(actual.abilities, expected.abilities);
    assert_eq!(actual.conditions, expected.conditions);
    assert_eq!(actual.state, expected.state);
    assert_eq!(actual.heartless, expected.heartless);
}

fn assert_population(r: &Run, depth: u32, kind: &str) {
    let f = r.floors.iter().find(|f| f.depth == depth as f64).unwrap();
    let event = f.godbeast_event.as_ref().unwrap();
    assert!(f.godbeast);
    assert_eq!(event.kind, kind);
    let seed = crate::rng::seed_hash(&r.seed);
    let plan = generation::domain_plan_revision(seed, 52.);
    assert_eq!(f.domain, plan[(depth as usize - 1) / 5].domain,
        "Terrain belongs to destination depth, even across a domain boundary");
    assert_eq!(f.enemy_blessing_budget.value_or(0.),
        generation::ordinary_blessing_budget(generation::BudgetSeed::Number(seed), depth).unwrap() as f64,
        "Fast does not compensate the destination's ordinary population budget");
    let ordinary_ids: std::collections::HashSet<_> = f.encounters.as_ref().unwrap().iter()
        .flat_map(|e| e.actor_ids.iter().map(|id| *id as u64)).collect();
    let ordinary_rewards: f64 = r.actors_at(depth as f64).iter()
        .filter(|a| ordinary_ids.contains(&(a.id as u64)) && !a.heartless.value_or(false))
        .map(|a| crate::inventory::heart_for(a, 0.).unwrap().exp.value_or(0.)).sum();
    assert_eq!(ordinary_rewards, f.enemy_blessing_budget.value_or(0.));

    // These four bodies exist on every production floor. The displaced Phantasmii
    // encounter must use destination31's Gargantuan tier, not milestone30's tier.
    let nightmares: Vec<_> = r.actors_at(depth as f64).iter()
        .filter(|a| crate::dreamwalking::is_nightmare(a)).collect();
    assert_eq!(nightmares.len(), crate::phantasmii_generation::NIGHTMARE_COUNT);
    for a in nightmares {
        assert_body(a, &crate::phantasmii_generation::nightmare_revision(depth, a.id, a.x, a.y, 52.));
    }
    let zillennia: Vec<_> = r.actors_at(depth as f64).iter()
        .filter(|a| a.creature_kind.as_ref().is_some_and(|k| k == "zillennia")).collect();
    assert_eq!(zillennia.len(), usize::from(kind == "Zillennia"));
    if let Some(a) = zillennia.first() {
        assert_eq!((a.x, a.y), (event.x, event.y));
        assert_body(a, &crate::population::zillennia_revision(depth, 52.).unwrap());
    }
    let tricksters: Vec<_> = r.actors_at(depth as f64).iter().filter(|a| crate::ahvamitt::is_actor(a)).collect();
    assert_eq!(tricksters.len(), usize::from(kind == "Ahvamitt"));
    if let Some(a) = tricksters.first() {
        assert_body(a, &crate::ahvamitt::actor_revision(a.id, event.x, event.y, 52.));
        assert_eq!((a.x, a.y), (event.x, event.y));
    }
    let ground = &r.ground[&depth.to_string()];
    assert_eq!(ground.iter().filter(|g| crate::phantasmii_generation::is_dagger(&g.item)).count(),
        usize::from(kind == "Phantasmii"));
    assert_eq!(ground.iter().filter(|g| crate::phantasmii_generation::is_shard(&g.item)).count(),
        if kind == "Phantasmii" { 3 } else { 0 });
    if kind == "Veridian" {
        assert_eq!(event.display_name.as_ref().map(String::as_str), Some("Verdania"));
        let tap = event.tap.as_ref().unwrap();
        assert_eq!(ground.iter().filter(|g| g.item.name == "Tree tap" && g.x == tap.x && g.y == tap.y).count(), 1);
    }
    assert!(generation::transitions_accessible(f, &generation::reachable_without_braziers(f, None)));
}

// Skip pathfinding/combat approach only. Keep generated terrain, ordinary actors,
// boss bodies, items and special-event data intact throughout activation tests.
fn stand_next_to(r: &mut Run, x: f64, y: f64) {
    let f = r.floor().unwrap();
    let candidates: Vec<_> = [-1., 0., 1.].into_iter().flat_map(|dy|
        [-1., 0., 1.].into_iter().map(move |dx| (x + dx, y + dy)))
        .filter(|&(xx, yy)| xx != x || yy != y)
        .map(|(xx, yy)| {
            let tile = (xx >= 0. && yy >= 0. && xx < f.width() as f64 && yy < f.height() as f64)
                .then(|| f.tiles[yy as usize * f.width() + xx as usize].as_str());
            (xx, yy, tile, crate::travel::passable(r, &r.player, xx, yy),
                crate::travel::habitat_valid(f, &r.player, xx, yy),
                r.actors_at(r.depth).iter().any(|a| !a.dead.value_or(false) && a.x == xx && a.y == yy))
        }).collect();
    let old_fixture_had_approach=candidates.iter().any(|(_,_,tile,passable,habitat,occupied)|
        *passable && *habitat && !*occupied && matches!(tile,Some("floor"|"grass"|"up"|"down")));
    if !old_fixture_had_approach {
        eprintln!("FAST_BOSS_PREVIOUS_FIXTURE_REJECTION seed={} depth={} kind={} target=({x},{y}) candidates={candidates:?}",
            r.seed,r.depth,f.godbeast_event.as_ref().unwrap().kind);
    }
    // Water and shallow water are legal player terrain, including around a real
    // Ahvamitt teleport destination. Exclude falling cells, not wet approaches.
    let at = candidates.iter().find(|(_, _, tile, passable, habitat, occupied)|
        *passable && *habitat && !*occupied
            && matches!(tile, Some("floor" | "grass" | "up" | "down" | "water" | "shallow-water" | "speed-up")))
        .unwrap_or_else(|| panic!(
            "{} depth {} {} target ({x},{y}) has no non-falling approach; (x,y,tile,passable,habitat,occupied)={candidates:?}",
            r.seed, r.depth, f.godbeast_event.as_ref().unwrap().kind));
    eprintln!("FAST_BOSS_CONTROLLED_APPROACH seed={} depth={} kind={} target=({x},{y}) cell=({},{}) tile={:?}",
        r.seed, r.depth, f.godbeast_event.as_ref().unwrap().kind, at.0, at.1, at.2);
    let (xx, yy) = (at.0, at.1);
    r.player.x = xx;
    r.player.y = yy;
}

fn collect_generated(r: &mut Run, id: f64) {
    let g = r.ground[&r.depth.to_string()].iter().find(|g| g.item.id == id).unwrap();
    r.player.x = g.x;
    r.player.y = g.y;
    let capacity = crate::inventory::carrying_capacity(r).unwrap();
    assert!(crate::inventory::collect_underfoot(r, capacity).unwrap() > 0);
    assert!(r.inventory.iter().any(|i| i.id == id || crate::phantasmii_generation::is_shard(i)));
}

fn activate_generated(mut r: Run, depth: u32) {
    assert!(crate::travel::travel_to(&mut r, depth as f64, false, false).unwrap());
    assert_eq!(r.depth, depth as f64);
    assert_eq!(r.floor().unwrap().monster_entry_tick.as_ref(), Some(&r.tick));
    r = strict_round_trip(&r, "entered boss floor");
    let e = r.floor().unwrap().godbeast_event.as_ref().unwrap().clone();
    match e.kind.as_str() {
        "Ahvamitt" => {
            let id = r.actors_at(r.depth).iter().find(|a| crate::ahvamitt::is_actor(a)).unwrap().id;
            for finds in 1..=3 {
                let a = crate::engine::actor(&r, id).unwrap().clone();
                stand_next_to(&mut r, a.x, a.y);
                assert!(crate::ahvamitt::attack(&mut r, id).unwrap());
                if finds < 3 {
                    let a = crate::engine::actor(&r, id).unwrap();
                    assert_eq!(a.conditions.get("ahvamittFinds"), Some(&(finds as f64)));
                    assert_eq!(a.hp, a.max_hp);
                    assert_eq!(a.state, "neutral");
                }
                r = strict_round_trip(&r, "Ahvamitt search phase");
            }
            assert!(crate::engine::actor(&r, id).is_none());
            assert!(r.floor().unwrap().godbeast_event.as_ref().unwrap().reward_created.value_or(false));
            assert_eq!(r.ground[&depth.to_string()].iter().filter(|g| g.item.name == crate::ahvamitt::MASQUE_NAME).count(), 1);
        }
        "Zillennia" => {
            let a = r.actors_at(r.depth).iter().find(|a| a.creature_kind.as_ref().is_some_and(|k| k == "zillennia")).unwrap().clone();
            if depth == 21 {
                assert_eq!(a.name, "Greater Zillennia");
                assert_ne!(a.zillennia_quality, crate::population::zillennia_revision(20, 52.).unwrap().zillennia_quality);
                assert!(crate::zillennia_ground::clears_ground(&r, &a));
            }
            stand_next_to(&mut r, a.x, a.y);
            // Keep the test player alive through a real, fully scaled pulse.
            r.player.max_hp = 10_000.; r.player.hp = 10_000.; r.player.base_max_hp = 10_000.0.into();
            let player = r.player.id;
            assert!(crate::monster_ruin::special_action(&mut r, a.id, player).unwrap());
            let after = crate::engine::actor(&r, a.id).unwrap();
            assert_eq!(after.conditions.get("beastCooldown"), Some(&3.));
            assert_eq!(after.energy, a.energy - 20.);
            assert!(r.player.hp < 10_000.);
            assert!(!crate::monster_ruin::special_action(&mut r, a.id, player).unwrap());
        }
        "Mercuria" => {
            let lever = e.lever.as_ref().unwrap();
            let mut expected_run = r.clone();
            let mut rng = crate::population::population_random(&format!("{}:floor:{depth}:Mercuria", r.seed));
            let expected_reward = crate::equipment::make_godbeast_permanent_for_run(&mut expected_run, depth, &mut rng).unwrap();
            assert!(crate::godbeast_interactions::pull_lever(&mut r, lever.x, lever.y).unwrap());
            assert_eq!(r.ground[&depth.to_string()].iter().find(|g| g.item.id == expected_reward.id).unwrap().item, expected_reward);
            assert!(!crate::godbeast_interactions::pull_lever(&mut r, lever.x, lever.y).unwrap());
            for _ in 0..7 {
                crate::ichor::tick_ichor_revision(r.floor_mut().unwrap(), 52.);
                crate::ichor::tick_fish(&mut r).unwrap();
            }
            assert_eq!(r.floor().unwrap().godbeast_event.as_ref().unwrap().fish_spawned.value_or(0.), 0.);
            r = strict_round_trip(&r, "Mercuria before first fish");
            crate::ichor::tick_ichor_revision(r.floor_mut().unwrap(), 52.);
            let id = r.next_id;
            let mut forecast = r.clone();
            let random = crate::rng::random(&mut forecast);
            crate::ichor::tick_fish(&mut r).unwrap();
            let fish = crate::engine::actor(&r, id).expect("Activated production well spawns its first fish at age eight");
            assert_body(fish, &crate::ichor::make_fish_revision(id, fish.x, fish.y, depth as f64, random, 52.));
            assert_eq!(r.floor().unwrap().godbeast_event.as_ref().unwrap().fish_spawned.as_ref(), Some(&1.));
            assert_eq!(r.floor().unwrap().godbeast_event.as_ref().unwrap().next_fish_age.as_ref(), Some(&16.));
        }
        "Veridian" => {
            let id = r.ground[&depth.to_string()].iter().find(|g| g.item.name == "Tree tap").unwrap().item.id;
            collect_generated(&mut r, id);
            stand_next_to(&mut r, e.x, e.y);
            assert_eq!(crate::inventory::command(&mut r, &Command::Use { id }).unwrap(), Some(true));
            assert!(r.veridian_sap_harvested.value_or(false));
            assert!(r.floor().unwrap().godbeast_event.as_ref().unwrap().tapped.value_or(false));
            assert!(!r.inventory.iter().any(|i| i.id == id));
            assert_eq!(r.inventory.iter().filter(|i| i.name == "Verdania Sap").count(), 1);
            assert!(!crate::godbeast_interactions::tap_veridian(&mut r).success);
        }
        "Borge" => {
            let target = crate::borge::strongest(&r).expect("Production Borge has an eligible ordinary sacrifice").clone();
            r.player.x = e.x - 3.; r.player.y = e.y;
            assert!(crate::borge::interact(&mut r, e.x - 2., e.y).unwrap());
            let activated = r.floor().unwrap().godbeast_event.as_ref().unwrap();
            assert!(activated.activated.value_or(false));
            assert_eq!(activated.sacrifice_actor_id.as_ref(), Some(&target.id));
            assert_eq!(activated.sacrifice_name.as_ref(), Some(&target.name));
            r = strict_round_trip(&r, "Borge marked actual generated sacrifice");
            let mut heart = crate::inventory::heart_for(&target, r.next_id).unwrap();
            r.next_id += 1.;
            crate::borge::mark_heart(&r, &target, &mut heart);
            assert_eq!(heart.borge_depth.as_ref(), Some(&(depth as f64)));
            r.inventory.push(heart);
            let altar = e.altar.as_ref().unwrap();
            r.player.x = altar.x; r.player.y = altar.y;
            assert!(crate::borge::interact(&mut r, altar.x, altar.y).unwrap());
            assert!(r.floor().unwrap().godbeast_event.as_ref().unwrap().reward_created.value_or(false));
            assert_eq!(r.inventory.iter().filter(|i| crate::borge::is_shard(i)).count(), 1);
            assert!(!crate::borge::interact(&mut r, altar.x, altar.y).unwrap());
        }
        "Phantasmii" => {
            let id = r.ground[&depth.to_string()].iter().find(|g| crate::phantasmii_generation::is_dagger(&g.item)).unwrap().item.id;
            collect_generated(&mut r, id);
            let altar = e.altar.as_ref().unwrap();
            r.player.x = altar.x; r.player.y = altar.y;
            assert_eq!(crate::inventory::command(&mut r, &Command::Use { id }).unwrap(), Some(true));
            assert!(crate::dreamwalking::active(&r));
            let body = crate::dreamwalking::body(&r).unwrap();
            assert_eq!(body.name, "Your sleeping body");
            assert_eq!(body.conditions.get("phantasmiiBody"), Some(&1.));
            assert_eq!((body.x, body.y), (altar.x, altar.y));
            r = strict_round_trip(&r, "Phantasmii sleeping body and dream floor");
            let shards: Vec<_> = r.ground[&depth.to_string()].iter().filter(|g| crate::phantasmii_generation::is_shard(&g.item)).map(|g| g.item.id).collect();
            assert_eq!(shards.len(), 3);
            for id in shards { collect_generated(&mut r, id); }
            assert!(crate::dreamwalking::unlocked(&r));
            r = strict_round_trip(&r, "Phantasmii unlocked");
            assert!(crate::dreamwalking::wake(&mut r));
            assert!(crate::dreamwalking::body(&r).is_none());
        }
        "Epidite" => {
            assert!(crate::epidite::solid_at(r.floor().unwrap(), e.x, e.y));
            stand_next_to(&mut r, e.x, e.y);
            r.player.max_hp = 10_000.; r.player.hp = 10_000.; r.player.base_max_hp = 10_000.0.into();
            let body = r.actors_at(r.depth).iter().find(|a| !a.heartless.value_or(false)).unwrap();
            let heart = crate::inventory::heart_for(body, r.next_id).unwrap();
            let id = heart.id; r.next_id += 1.;
            let mut expected = heart.clone(); assert!(crate::epidite::bless(&mut expected));
            let (x, y) = (r.player.x, r.player.y);
            r.ground_at_mut(depth as f64).push(GroundItem { x, y, item: heart, ..Default::default() });
            for _ in 0..29 { crate::epidite::world_turn(&mut r).unwrap(); }
            assert_eq!(crate::epidite::countdown(r.floor().unwrap().godbeast_event.as_ref().unwrap()), 1);
            assert!(!r.ground[&depth.to_string()].iter().find(|g| g.item.id == id).unwrap().item.blessed.value_or(false));
            r = strict_round_trip(&r, "Epidite before first pulse");
            crate::epidite::world_turn(&mut r).unwrap();
            assert_eq!(crate::epidite::countdown(r.floor().unwrap().godbeast_event.as_ref().unwrap()), 30);
            assert_eq!(r.ground[&depth.to_string()].iter().find(|g| g.item.id == id).unwrap().item, expected);
            assert!(r.player.hp < 10_000.);
        }
        other => panic!("Uncovered authored boss: {other}"),
    }
    strict_round_trip(&r, "activated boss");
}

fn production_route(seed: &str, expected: [&str; 7], through: u32, activate: &[u32]) -> Run {
    let plan = generation::domain_plan_revision(crate::rng::seed_hash(seed), 52.);
    assert_eq!(plan[..7].iter().map(|p| p.godbeast_kind.as_str()).collect::<Vec<_>>(), expected);
    let mut r = startup::create_avatar_run_fast(seed).unwrap();
    assert_eq!(r.depth, 1.);
    assert_eq!(r.floors.iter().map(|f| f.depth as u32).collect::<Vec<_>>(), [1, 3]);
    for ((scheduled, destination), kind) in SCHEDULE.into_iter().zip(ARRIVALS).zip(expected) {
        if destination > through { break; }
        startup::ensure_floors(&mut r, destination).unwrap_or_else(|e| panic!("{seed} {scheduled}->{destination}: {e}"));
        assert_population(&r, destination, kind);
        r = strict_round_trip(&r, &format!("populated milestone {scheduled} at {destination}"));
        if activate.contains(&destination) { activate_generated(r.clone(), destination); }
        assert!(!r.actors.contains_key(&scheduled.to_string()) || scheduled == destination);
    }
    r
}

#[test]
fn fast_production_all_bosses_activate_and_strict_load_with_destination_scaling() {
    // All seven authored kinds. Every actor-bearing encounter crosses a skipped
    // milestone: Ahvamitt10->11, Zillennia20->21, Phantasmii30->31.
    let mut r = production_route("0-1-2:fast-boss-production-287",
        ["Mercuria", "Ahvamitt", "Borge", "Zillennia", "Epidite", "Phantasmii", "Veridian"], 35, &ARRIVALS);
    startup::ensure_floors(&mut r, 40).unwrap();
    assert_eq!(r.floors.iter().map(|f| f.depth as u32).collect::<Vec<_>>(),
        (1..40).step_by(2).chain(std::iter::once(40)).collect::<Vec<_>>());
    assert_eq!(r.floors.iter().filter(|f| f.godbeast).map(|f| f.depth as u32).collect::<Vec<_>>(), ARRIVALS);
    assert_eq!(r.actors_at(40.).iter().filter(|a| crate::finale::is_boss(a)).count(), 1);
    strict_round_trip(&r, "complete physical route including final40");
}

#[test]
fn fast_production_displaced_fixture_bosses_keep_rewards_and_phases() {
    production_route("0-1-2:fast-boss-production-29",
        ["Epidite", "Mercuria", "Ahvamitt", "Borge", "Zillennia", "Veridian", "Phantasmii"],
        31, &[11, 21, 31]);
}

#[test]
fn fast_production_displaced_epidite_keeps_thirty_turn_pulse_and_strict_load() {
    production_route("0-1-2:fast-boss-production-3",
        ["Veridian", "Epidite", "Zillennia", "Mercuria", "Ahvamitt", "Borge", "Phantasmii"],
        11, &[11]);
}
