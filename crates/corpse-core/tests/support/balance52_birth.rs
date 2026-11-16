//! Shared integration-test oracle. It deliberately does not call the production
//! profile applicator when computing expected state.
#![allow(dead_code)]
use corpse_core::{Actor, Floor, Run, balance52, inventory, population, startup};
use std::sync::Arc;

pub const ORIGINAL_FIGHTERS: &[(&str, u32, u32)] = &[
    ("bear", 6, 14),
    ("blorb", 1, 3),
    ("cancer", 35, 40),
    ("dire-bear", 20, 29),
    ("giant-beetle", 8, 17),
    ("glim", 7, 16),
    ("hemogoblin", 1, 5),
    ("killspawn", 25, 34),
    ("long-saidri", 13, 22),
    ("minotaur", 33, 40),
    ("needleling", 1, 7),
    ("pale:Biter", 3, 11),
    ("pale:Greater Biter", 18, 27),
    ("pale:Jackal", 1, 7),
    ("phoenix", 32, 40),
    ("rat", 1, 3),
    ("shellback-beetle", 1, 4),
    ("troll", 19, 28),
    ("ursa-major", 36, 40),
    ("ursa-minor", 36, 40),
];

pub fn canonical_kind(key: &str) -> &str {
    for prefix in ["nest:", "queen:", "witch:", "birther:", "greater-birther:"] {
        if let Some(kind) = key.strip_prefix(prefix) {
            return kind;
        }
    }
    key
}

pub fn natural_kind(a: &Actor) -> String {
    if let Some(kind) = a.creature_kind.as_ref() {
        return kind.clone();
    }
    if let Some(def) = corpse_core::enemy_traits::definition(a) {
        return def.id.clone();
    }
    let name = corpse_core::revision50_creature_identity::legacy_name(
        corpse_core::enemy_traits::base_name(&a.name),
    );
    if let Some((kind, _)) = population::catalog().actors.iter().find(|(_, template)| {
        template.name == name && template.abilities.iter().all(|id| a.abilities.contains(id))
    }) {
        return kind.clone();
    }
    // Non-catalog authored actors are allowed in whole-world comparisons, but
    // no guessed display-name key may accidentally admit a numeric profile.
    format!("unresolved:{name}")
}

pub fn generated_key(a: &Actor) -> String {
    let kind = natural_kind(a);
    if a.conditions.get("nestResident") == Some(&1.) {
        format!("nest:{kind}")
    } else {
        kind
    }
}

pub fn coefficients(row: &balance52::BirthProfile) -> [(&'static str, Option<f64>); 5] {
    [
        ("monsterGasPassiveVolume", row.gas_passive),
        ("monsterGasBoostVolume", row.gas_boost),
        ("monsterGasShotVolume", row.gas_shot),
        ("monsterHealAmount", row.heal_amount),
        ("monsterSpecialDamage", row.special_damage),
    ]
}

pub fn first_dodge_offset(key: &str, fallback: f64) -> f64 {
    balance52::profiles().iter().filter(|row| row.kind == key)
        .min_by_key(|row| row.floor).map_or(fallback, |row| row.dodge)
}
pub fn expected_generic_birth(old: &Actor, key: &str) -> Actor {
    let mut expected = old.clone();
    expected.stats.accuracy += 15.;
    expected.stats.dodge = (expected.stats.dodge + 10.).max(10.);
    let rows: Vec<balance52::BirthProfile> = serde_json::from_str(include_str!("../../src/catalog/species-fixed-v52.json")).unwrap();
    if let Some(row) = rows.iter().find(|r|r.kind == canonical_kind(key)) {
        let base=population::create_actor_revision(canonical_kind(key),0.,0.,0.,false,51.).unwrap();
        expected.stats=base.stats; expected.energy=if canonical_kind(key)!=key {base.energy}else{old.energy};
        expected.stats.offense=row.offense; expected.stats.defense=row.defense;
        expected.stats.accuracy=row.accuracy+15.;expected.stats.dodge=(row.dodge+10.).max(10.);expected.stats.dr=row.dr;
        expected.max_hp=100.+6.*row.defense;expected.hp=if old.dead.value_or(false){0.}else{expected.max_hp};
        for (key,value) in coefficients(row) {
            expected.conditions.shift_remove(key);
            if let Some(n)=value.or_else(||base.conditions.get(key).copied()){expected.conditions.insert(key.into(),n);}
        }
    }
    if canonical_kind(key) == "pale:Jackal" { expected.heartless = true.into(); }
    expected
}
pub fn expected_birth(old: &Actor, key: &str, _floor: f64) -> Actor {
    let mut expected = expected_generic_birth(old,key);
    // Floor-authored fungus is distinct from the generic bastidion template.
    // Its fixed body is the first authored fungus, while corpse provenance and
    // generated identity/coordinates remain from this occurrence.
    if old.name == "Corpse Fungus" {
        let first = &population::catalog().fungi[0];
        expected.stats = first.stats.clone();
        expected.stats.accuracy += 15.;
        expected.stats.dodge = (expected.stats.dodge + 10.).max(10.);
        expected.max_hp = first.max_hp;
        expected.hp = if old.dead.value_or(false) {0.} else {first.hp};
    }
    expected
}

pub fn assert_rewards(old: &Actor, new: &Actor) {
    // Some authored special bodies have no heart template. Compare the result
    // including the error, rather than unwrapping or treating an error as a heart.
    let old = inventory::heart_for_revision(old, 9999., 51.).map_err(|e| e.to_string());
    let new = inventory::heart_for_revision(new, 9999., 52.).map_err(|e| e.to_string());
    assert_eq!(
        new, old,
        "birth tuning changed an authored heart reward/result"
    );
}

pub fn floor(depth: u32) -> Floor {
    let (w, h) = (79, 29);
    let mut tiles = vec!["wall".to_string(); w * h];
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            tiles[y * w + x] = "floor".into();
        }
    }
    let mut f = Floor {
        depth: depth as f64,
        width: (w as f64).into(),
        height: (h as f64).into(),
        domain: "Dungeon".into(),
        tiles,
        lights: vec![100.; w * h],
        seen: vec![true; w * h],
        ..Default::default()
    };
    corpse_core::environment::initialize(&mut f).unwrap();
    f
}

pub fn scene(depth: u32, revision: f64, seed: u32) -> Run {
    let mut r =
        startup::new_run_shell_revision("0-5-11:r52-birth-hook", "Unblessed", false, revision)
            .unwrap();
    r.depth = depth as f64;
    r.rng = seed as f64;
    r.player.x = 2.;
    r.player.y = 2.;
    r.player.abilities.clear();
    r.player.active.clear();
    r.player.conditions.clear();
    r.inventory.clear();
    r.floors = (1..=depth.max(10)).map(|d| Arc::new(floor(d))).collect();
    r.actors.clear();
    r.ground.clear();
    for d in 1..=depth.max(10) {
        r.actors.insert(d.to_string(), Arc::new(vec![]));
        r.ground.insert(d.to_string(), Arc::new(vec![]));
    }
    r.next_id = 901.;
    r.tick = 0.;
    r
}

pub fn summon_once(
    parent_kind: &str,
    depth: u32,
    revision: f64,
    seed: u32,
    heartless: bool,
) -> Run {
    let mut r = scene(depth, revision, seed);
    let mut a =
        population::create_actor_revision(parent_kind, 900., 4., 4., false, revision).unwrap();
    a.state = "hunting".into();
    a.heartless = heartless.into();
    let parent_stats = a.stats.clone();
    r.actors_at_mut(r.depth).push(a);
    let player = r.player.id;
    assert!(
        corpse_core::ai::enemy_action(&mut r, 900., player).unwrap(),
        "{parent_kind} cast failed"
    );
    assert!(
        r.actors_at(r.depth).len() > 1,
        "{parent_kind} created no child"
    );
    assert_eq!(r.actors_at(r.depth)[0].stats, parent_stats,
        "{parent_kind} summoning must not change parent stats or reapply birth baselines");
    r
}

pub fn context_for_parent(parent: &str, child: &Actor) -> String {
    let context = match parent {
        "pale:Birther" => "birther",
        "pale:Greater Birther" => "greater-birther",
        "witch" => "witch",
        "saidribee-queen" => "queen",
        _ => panic!("Unknown native parent fixture {parent}"),
    };
    format!("{context}:{}", natural_kind(child))
}

pub fn paired_summon(parent: &str, depth: u32, seed: u32, heartless: bool) -> (Run, Run) {
    let old = summon_once(parent, depth, 51., seed, heartless);
    let new = summon_once(parent, depth, 52., seed, heartless);
    assert_eq!(old.rng, new.rng, "birth profiles must not consume RNG");
    assert_eq!(old.next_id, new.next_id);
    assert_eq!(
        old.actors_at(old.depth).len(),
        new.actors_at(new.depth).len()
    );
    // Parents use the generic constructor, not the floor-context overlay.
    // Their R52 birth differences are Accuracy and Dodge; summoning changes neither body.
    let expected_parent = expected_generic_birth(&old.actors_at(old.depth)[0], parent);
    assert_eq!(
        expected_parent,
        new.actors_at(new.depth)[0],
        "parent payment/cooldown/traits must stay legacy apart from birth Accuracy/Dodge"
    );
    for (a, b) in old.actors_at(old.depth)[1..]
        .iter()
        .zip(&new.actors_at(new.depth)[1..])
    {
        let key = context_for_parent(parent, a);
        assert_eq!(
            *b,
            expected_birth(a, &key, depth as f64),
            "{key} floor {depth}, seed {seed}"
        );
        assert_rewards(a, b);
    }
    (old, new)
}

pub fn nest(depth: u32, revision: f64) -> (Floor, Vec<Actor>, Vec<corpse_core::GroundItem>, f64) {
    let mut f = floor(depth);
    let w = f.width();
    let inhabitants: Vec<_> = (3..9).map(|x| 3 * w + x).collect();
    let queen = 4 * w + 4;
    let honey = 4 * w + 6;
    let mut cells = inhabitants.clone();
    cells.extend([queen, honey]);
    f.pocket_biomes = vec![corpse_core::pocket_biomes::PocketBiome {
        theme: "saidribee-nest".into(),
        queen_cell: Some(queen),
        honey_cell: Some(honey),
        inhabitants,
        cells,
        ..Default::default()
    }]
    .into();
    let (mut actors, mut ground, mut next) = (vec![], vec![], 900.);
    corpse_core::pocket_biomes::populate_revision(
        &mut f,
        &mut actors,
        &mut ground,
        &mut next,
        revision,
    )
    .unwrap();
    (f, actors, ground, next)
}

/// Return the actual new actor from its production birth boundary, never a
/// contextual string passed to the generic template constructor.
pub fn paired_birth(key: &str, depth: u32) -> (Actor, Actor) {
    if key.starts_with("nest:") {
        let (f, old, g, id) = nest(depth, 51.);
        let (new_f, new, new_g, new_id) = nest(depth, 52.);
        assert_eq!(f, new_f);
        assert_eq!(g, new_g);
        assert_eq!(id, new_id);
        assert_eq!(old.len(), 7);
        assert_eq!(new.len(), 7);
        for (a, b) in old.iter().zip(&new) {
            assert_eq!(*b, expected_birth(a, &generated_key(a), depth as f64));
            assert_rewards(a, b);
        }
        let i = old
            .iter()
            .position(|a| generated_key(a) == key)
            .expect("unknown Nest profile key");
        return (old[i].clone(), new[i].clone());
    }
    let parent = if key.starts_with("birther:") {
        Some("pale:Birther")
    } else if key.starts_with("greater-birther:") {
        Some("pale:Greater Birther")
    } else if key.starts_with("witch:") {
        Some("witch")
    } else if key.starts_with("queen:") {
        Some("saidribee-queen")
    } else {
        None
    };
    if let Some(parent) = parent {
        for seed in 0..64 {
            let (old, new) = paired_summon(parent, depth, seed, false);
            if let Some(i) = old
                .actors_at(old.depth)
                .iter()
                .position(|a| a.id != 900. && context_for_parent(parent, a) == key)
            {
                return (
                    old.actors_at(old.depth)[i].clone(),
                    new.actors_at(new.depth)[i].clone(),
                );
            }
        }
        panic!("No native birth for {key} in 64 deterministic seeds");
    }
    let (mut f, mut new_f) = (floor(depth), floor(depth));
    let (mut id, mut new_id) = (900., 900.);
    let (mut rng, mut new_rng) = (
        population::population_random("r52-exact-birth"),
        population::population_random("r52-exact-birth"),
    );
    let old = population::create_population_actor_revision(
        &mut id,
        &mut f,
        key,
        3 * 79 + 3,
        false,
        &mut rng,
        51.,
    )
    .unwrap();
    let new = population::create_population_actor_revision(
        &mut new_id,
        &mut new_f,
        key,
        3 * 79 + 3,
        false,
        &mut new_rng,
        52.,
    )
    .unwrap();
    assert_eq!(f, new_f);
    assert_eq!(id, new_id);
    assert_eq!(rng.random(), new_rng.random());
    (old, new)
}
