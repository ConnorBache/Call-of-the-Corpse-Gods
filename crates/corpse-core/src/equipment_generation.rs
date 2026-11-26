//! Exact authored loot draws; streams are supplied by population, never wall-clock seeded.
use super::make_equipment;
use crate::{combat::integer_stat, model::*, rng::GenerationRng};
#[path = "equipment_generation_revision50.rs"]
mod revision50;
#[path = "equipment_generation_revision51.rs"]
mod revision51;
const STATS: [&str; 8] = [
    "offense",
    "defense",
    "accuracy",
    "dodge",
    "dr",
    "perception",
    "maxEnergy",
    "maxHp",
];
const KINDS: [&str; 4] = ["weapon", "armor", "ring", "amulet"];
const POTIONS: [&str; 16] = [
    "healing",
    "energy",
    "incineration",
    "caustic gas",
    "paralysis",
    "offense",
    "defense",
    "accuracy",
    "dodge",
    "detect magic",
    "telepathy",
    "levitation",
    "speed",
    "confusion",
    "invisibility",
    "descent",
];
fn catalog() -> &'static serde_json::Map<String, serde_json::Value> {
    static C: std::sync::OnceLock<serde_json::Map<String, serde_json::Value>> =
        std::sync::OnceLock::new();
    C.get_or_init(|| serde_json::from_str(include_str!("../catalog/equipment-v48.json")).unwrap())
}
fn pick<'a, T>(values: &'a [T], rng: &mut GenerationRng) -> &'a T {
    &values[(rng.random() * values.len() as f64).floor() as usize]
}
fn stat<'a>(s: &'a PartialStats, k: &str) -> &'a Field<f64> {
    match k {
        "offense" => &s.offense,
        "defense" => &s.defense,
        "accuracy" => &s.accuracy,
        "dodge" => &s.dodge,
        "dr" => &s.dr,
        "perception" => &s.perception,
        "maxEnergy" => &s.max_energy,
        _ => &s.max_hp,
    }
}
fn stat_mut<'a>(s: &'a mut PartialStats, k: &str) -> &'a mut Field<f64> {
    match k {
        "offense" => &mut s.offense,
        "defense" => &mut s.defense,
        "accuracy" => &mut s.accuracy,
        "dodge" => &mut s.dodge,
        "dr" => &mut s.dr,
        "perception" => &mut s.perception,
        "maxEnergy" => &mut s.max_energy,
        _ => &mut s.max_hp,
    }
}
fn scaled(k: &str, n: f64) -> f64 {
    integer_stat(
        n * match k {
            "maxHp" | "maxEnergy" => 6.0,
            "dr" => 0.5,
            _ => 1.0,
        },
    )
}
fn magnitude(depth: u32, r: &mut GenerationRng) -> f64 {
    1.0 + (r.random() * (depth as f64 / 2.0).ceil().max(1.0)).floor()
}
fn add(i: &mut Item, k: &str, n: f64) {
    let value = i
        .bonus
        .as_ref()
        .map(|b| stat(b, k).value_or(0.0))
        .unwrap_or(0.0);
    let rounded = if let Some(base) = i.base_bonus.as_ref().and_then(|b| stat(b, k).as_ref()) {
        integer_stat(*base) + integer_stat(value - *base)
    } else {
        integer_stat(value)
    };
    *stat_mut(i.bonus.get_or_insert_with(Default::default), k) = (rounded + n).into();
}
pub fn modify_equipment(item: &mut Item, depth: u32, rng: &mut GenerationRng) {
    let choices: Vec<_> = [
        (2, 2.0, "double"),
        (4, 3.0, "triple"),
        (8, 4.0, "quadruple"),
        (16, 5.0, "quintuple"),
        (32, 6.0, "sextuple"),
    ]
    .into_iter()
    .filter(|(d, _, _)| item.kind == "armor" && depth >= *d)
    .collect();
    let category = (rng.random() * if choices.is_empty() { 2.0 } else { 3.0 }).floor() as u8;
    if category == 2 {
        let (_, mult, name) = pick(&choices, rng);
        let b = item
            .base_bonus
            .as_ref()
            .or(item.bonus.as_ref())
            .cloned()
            .unwrap_or_default();
        for k in STATS {
            if let Some(n) = stat(&b, k).as_ref() {
                *stat_mut(item.bonus.get_or_insert_with(Default::default), k) =
                    (integer_stat(*n) * mult).into();
            }
        }
        item.name += &format!(" of {name}");
        return;
    }
    let plus = *pick(&STATS, rng);
    let amount = scaled(plus, magnitude(depth, rng));
    add(item, plus, amount);
    if category == 0 {
        item.name += &format!(" of +{amount} {plus}")
    } else {
        let other: Vec<_> = STATS.into_iter().filter(|k| *k != plus).collect();
        let minus = *pick(&other, rng);
        let penalty = scaled(minus, magnitude(depth, rng));
        add(item, minus, -penalty);
        item.name += &format!(", Cursed of +{amount} {plus} and -{penalty} {minus}");
    }
}
/// O(C + A + item payload), with exact legacy RNG order; compatibility entry point.
pub fn create_equipment(
    kind: &str,
    rng: &mut GenerationRng,
    next_id: &mut f64,
    depth: u32,
    owned: &[String],
) -> Result<Item, String> {
    create_equipment_revision(kind,rng,next_id,depth,owned,29.0)
}
/// Generated jewelry has no intrinsic base stats. Check effective integer
/// contributions, not merely a present/empty bonus object or a name suffix.
/// O(8) time, no allocation; only called at creation, never on saved/admin items.
fn jewelry_has_property(item: &Item) -> bool {
    item.granted_ability.as_ref().is_some_and(|id| !id.is_empty())
        || item.bonus.as_ref().is_some_and(|b| STATS.iter().any(|k|
            integer_stat(stat(b,k).value_or(0.0)) != 0.0))
}
/// O(C + A + item payload); revision43 adds at most one bounded existing
/// modification roll for otherwise plain jewelry, with no retry loop.
pub fn create_equipment_revision(
    kind: &str, rng: &mut GenerationRng, next_id: &mut f64,
    depth: u32, owned: &[String], revision: f64,
) -> Result<Item, String> {
    create_equipment_with_amulet_rules(kind,rng,next_id,depth,owned,revision,true,revision>=52.)
}
fn create_equipment_with_amulet_rules(
    kind: &str, rng: &mut GenerationRng, next_id: &mut f64,
    depth: u32, owned: &[String], revision: f64, current:bool,current_rings:bool,
) -> Result<Item,String> {
    if revision >= 50.0 {
        let mut item = revision50::create_with_item_rules(kind, rng, *next_id, revision,current,current_rings)?;
        crate::revision52_amulet_tiers::initialize_birth(&mut item, revision, depth)?;
        *next_id += 1.0;
        return Ok(item);
    }
    let names: Vec<_> = catalog()
        .iter()
        .filter(|(n, d)| n.as_str() != "Bow" && d["kind"] == kind)
        .map(|(n, _)| n.as_str())
        .collect();
    if names.is_empty() {
        return Err(format!("unknown equipment kind {kind}"));
    }
    let name = pick(&names, rng);
    let id = *next_id;
    *next_id += 1.0;
    let mut item = make_equipment(name, id).map_err(|e| e.0)?;
    if matches!(kind, "ring" | "amulet") && rng.random() < 0.5 {
        let pool: Vec<_> = crate::progression::catalog_for_revision(revision)
            .iter()
            .filter(|a| !owned.contains(&a.id))
            .collect();
        if !pool.is_empty() {
            let a = pick(&pool, rng);
            item.granted_ability = a.id.clone().into();
            item.name += &format!(" of {}", a.name);
            if depth <= 15 {
                let k = *pick(&STATS, rng);
                let penalty = scaled(k, magnitude(depth, rng));
                add(&mut item, k, -penalty);
                item.name += &format!(", Cursed -{penalty} {k}");
            }
            return Ok(item);
        }
    }
    if rng.random() < 0.5 {
        modify_equipment(&mut item, depth, rng)
    }
    // Preserve all successful rolls, and every legacy draw. Even when the
    // ability pool is exhausted, one existing modifier guarantees a nonzero
    // positive stat (an optional curse affects a different stat).
    if revision >= 43.0 && matches!(kind,"ring"|"amulet") && !jewelry_has_property(&item) {
        modify_equipment(&mut item, depth, rng);
    }
    Ok(item)
}
/// O(L*(C+A)) time with L<=20 loot slots, C equipment catalog, A=72 ability definitions.
pub fn generate_floor_loot(
    depth: u32,
    rng: &mut GenerationRng,
    next_id: &mut f64,
    excluded: &[String],
    count: usize,
) -> Result<Vec<Item>, String> {
    generate_floor_loot_revision(depth,rng,next_id,excluded,count,29.0)
}
/// O(L*(C+A)+item payloads), L<=20; no extra RNG draw for revision selection.
pub fn generate_floor_loot_revision(
    depth: u32, rng: &mut GenerationRng, next_id: &mut f64,
    excluded: &[String], count: usize, revision: f64,
) -> Result<Vec<Item>, String> {
    let mut generated = false;
    generate_floor_loot_with_budget(depth, rng, next_id, excluded, count, revision,
        if revision >= 51. { Some(&mut generated) } else { None },true,revision>=52.,revision>=52.)
}
/// Production entry point: all generated floors share the persisted run budget.
/// Same bounded loot cost, plus a one-time import scan documented in unique_composite_bow.
pub fn generate_floor_loot_for_run(
    run: &mut Run, depth: u32, rng: &mut GenerationRng, excluded: &[String], count: usize,
) -> Result<Vec<Item>, String> {
    let revision = run.simulation_revision.value_or(29.);
    if revision < 51. {
        return generate_floor_loot_revision(depth, rng, &mut run.next_id, excluded, count, revision);
    }
    if !(2..=20).contains(&count) {
        return Err("Invalid ordinary loot slot count".into());
    }
    let current=crate::amulet_rules::current(run);
    let current_potions=crate::potion_rules::current(run);
    let current_rings=crate::ring_rules::current(run);
    crate::unique_composite_bow::initialize(run);
    if let Some(primary) = revision51::primary_kind(&run.seed, depth) {
        let mut items=revision51::generate_with_item_rules(primary, rng, &mut run.next_id, revision,current,current_potions,current_rings)?;
        for item in &mut items {crate::revision52_amulet_tiers::initialize_birth(item,revision,depth)?;}
        return Ok(items);
    }
    let mut generated = run.composite_bow_generated.value_or(false);
    let items = generate_floor_loot_with_budget(depth, rng, &mut run.next_id, excluded, count,
        revision, Some(&mut generated),current,current_potions,current_rings)?;
    run.composite_bow_generated = generated.into();
    Ok(items)
}
fn generate_floor_loot_with_budget(
    depth: u32, rng: &mut GenerationRng, next_id: &mut f64,
    excluded: &[String], count: usize, revision: f64, mut bow_generated: Option<&mut bool>, current:bool,current_potions:bool,current_rings:bool,
) -> Result<Vec<Item>, String> {
    if !(2..=20).contains(&count) {
        return Err("Invalid ordinary loot slot count".into());
    }
    let max = (if depth <= 3 { 2 } else { 4 }).min((count - 1).max(2));
    let permanents = 2 + (rng.random() * (max - 1) as f64).floor() as usize;
    let kinds = if bow_generated.as_deref() == Some(&true) { &revision50::JEWELRY_KINDS[..] }
        else if revision >= 50.0 { &revision50::KINDS[..] } else { &KINDS[..] };
    let first = *pick(kinds, rng);
    let others: Vec<_> = kinds.iter().copied().filter(|k| *k != first).collect();
    let second = *pick(&others, rng);
    let mut items = vec![];
    let mut excluded = excluded.to_vec();
    for n in 0..permanents {
        let kind = match n {
            0 => first,
            1 => second,
            _ => *pick(if bow_generated.as_deref() == Some(&true) {
                &revision50::JEWELRY_KINDS[..]
            } else { kinds }, rng),
        };
        let i = create_equipment_with_amulet_rules(kind, rng, next_id, depth, &excluded, revision,current,current_rings)?;
        if crate::unique_composite_bow::is_composite(&i) {
            if let Some(generated) = bow_generated.as_deref_mut() { *generated = true; }
        }
        if let Some(a) = i.granted_ability.as_ref() {
            excluded.push(a.clone())
        }
        items.push(i)
    }
    for _ in permanents..count {
        let id = *next_id;
        *next_id += 1.0;
        let effect = *pick(if revision>=50. {crate::revision50_potions::roster(current_potions)} else {&POTIONS[..]}, rng);
        items.push(Item {
            id,
            name: "Unidentified potion".into(),
            kind: "potion".into(),
            glyph: "!".into(),
            effect: effect.to_string().into(),
            identified: false.into(),
            ..Default::default()
        });
    }
    Ok(items)
}
pub fn make_godbeast_permanent(
    depth: u32,
    id: f64,
    rng: &mut GenerationRng,
) -> Result<Item, String> {
    make_godbeast_permanent_revision(depth, id, rng, 29.0)
}
/// Shared budget for Sphinx and Mercuria, including already generated later floors.
/// O(1) named selection/payload allocation after the one-time import scan.
pub fn make_godbeast_permanent_for_run(
    run: &mut Run, depth: u32, rng: &mut GenerationRng,
) -> Result<Item, String> {
    let revision = run.simulation_revision.value_or(29.);
    let id = run.next_id;
    if revision < 51. { return make_godbeast_permanent_revision(depth, id, rng, revision); }
    crate::unique_composite_bow::initialize(run);
    let kinds = if run.composite_bow_generated.value_or(false) {
        &revision50::JEWELRY_KINDS[..]
    } else { &revision50::KINDS[..] };
    let mut item = revision50::create_with_item_rules(pick(kinds, rng), rng, id, revision,crate::amulet_rules::current(run),crate::ring_rules::current(run))?;
    crate::revision52_amulet_tiers::initialize_birth(&mut item, revision, depth)?;
    if crate::unique_composite_bow::is_composite(&item) { run.composite_bow_generated = true.into(); }
    Ok(item)
}
/// The well always uses the named amulet pool at its maximum tier. Keep the
/// old group draw's stream position even though Mercuria no longer uses it.
pub(crate) fn make_mercuria_reward(run: &mut Run, depth: u32, rng: &mut GenerationRng) -> Result<Item, String> {
    if !crate::mercuria_rules::current(run) {
        return make_godbeast_permanent_for_run(run, depth, rng);
    }
    let _ = rng.random();
    let revision = run.simulation_revision.value_or(29.);
    let mut item = revision50::create_with_item_rules("amulet", rng, run.next_id, revision,
        crate::amulet_rules::current(run), crate::ring_rules::current(run))?;
    crate::revision52_amulet_tiers::initialize_max_birth(&mut item, revision)?;
    Ok(item)
}
/// Existing event catalog filtered to Composite bow, Ring and Amulet: one equal
/// candidate per useful group, then the same uniform named-jewelry draw as loot.
/// O(C) legacy catalog scan; O(1) R50 lookup plus returned item allocation.
pub fn make_godbeast_permanent_revision(
    depth: u32, id: f64, rng: &mut GenerationRng, revision: f64,
) -> Result<Item, String> {
    if revision >= 50.0 {
        let kind = *pick(&revision50::KINDS, rng);
        let mut item=revision50::create(kind, rng, id, revision)?;
        crate::revision52_amulet_tiers::initialize_birth(&mut item, revision, depth)?;
        return Ok(item);
    }
    let names: Vec<_> = catalog()
        .keys()
        .filter(|n| n.as_str() != "Bow")
        .map(String::as_str)
        .collect();
    let name = pick(&names, rng);
    let mut item = make_equipment(name, id).map_err(|e| e.0)?;
    let k = *pick(&STATS, rng);
    let amount = scaled(k, depth as f64);
    add(&mut item, k, amount);
    item.name += &format!(" of +{amount} {k}");
    Ok(item)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_loot_and_reward_draws() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/loot-v48.json")).unwrap();
        for c in cases.as_array().unwrap() {
            let depth = c["depth"].as_u64().unwrap() as u32;
            let mut rng = GenerationRng::new(c["seed"].as_u64().unwrap() as u32);
            let mut next = 100.0;
            let excluded: Vec<String> = serde_json::from_value(c["excluded"].clone()).unwrap();
            let items = generate_floor_loot(
                depth,
                &mut rng,
                &mut next,
                &excluded,
                c["count"].as_u64().unwrap() as usize,
            )
            .unwrap();
            let expected: Vec<Item> = serde_json::from_value(c["items"].clone()).unwrap();
            assert_eq!(
                items, expected,
                "seed {} depth{depth} count{}",
                c["seed"], c["count"]
            );
            let reward = make_godbeast_permanent(depth, next, &mut rng).unwrap();
            next += 1.0;
            let expected: Item = serde_json::from_value(c["reward"].clone()).unwrap();
            assert_eq!(reward, expected);
            assert_eq!(next, c["nextId"].as_f64().unwrap());
            assert_eq!(rng.state, c["state"].as_u64().unwrap() as u32);
        }
    }
}

#[cfg(test)]
#[path = "equipment_generation_revision43_tests.rs"]
mod revision43_tests;

#[cfg(test)]
#[path = "equipment_generation_revision50_tests.rs"]
mod revision50_tests;

#[cfg(test)]
#[path = "equipment_generation_revision51_tests.rs"]
mod revision51_tests;
