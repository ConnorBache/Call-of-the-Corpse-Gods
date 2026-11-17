//! Native population planning. Catalog JSON is immutable authored data, never live state.
use crate::{Actor, Field, Floor, Item, rng::GenerationRng};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::OnceLock,
};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candidate {
    pub kinds: Vec<String>,
    pub danger: String,
    pub score: f64,
    pub weight: f64,
    pub blessings: i32,
}
#[derive(Deserialize)]
pub struct SingleChoice {
    pub id: String,
    pub family: String,
    pub weight: f64,
}
#[derive(Deserialize)]
pub struct PopulationCatalog {
    pub singles: IndexMap<String, Vec<SingleChoice>>,
    pub actors: IndexMap<String, Actor>,
    pub hearts: IndexMap<String, i32>,
    pub awake: IndexMap<String, bool>,
    pub aquatic: IndexMap<String, bool>,
    pub candidates: IndexMap<String, Vec<Candidate>>,
    pub fungi: Vec<Actor>,
    pub zillennia: Vec<Actor>,
    pub corpses: Vec<String>,
    pub pairs: IndexMap<String, String>,
    pub danger: IndexMap<String, f64>,
    pub potions: Vec<String>,
}
pub fn catalog() -> &'static PopulationCatalog {
    static C: OnceLock<PopulationCatalog> = OnceLock::new();
    C.get_or_init(|| {
        serde_json::from_str(include_str!("catalog/population-v48.json"))
            .expect("frozen population catalog")
    })
}
/// Population FNV differs from geometry only for the zero-hash case.
pub fn population_random(seed: &str) -> GenerationRng {
    let mut h = 2166136261u32;
    for c in seed.chars() {
        let n = c as u32;
        let code = if n > 65535 {
            0xd800 + ((n - 65536) >> 10)
        } else {
            n
        };
        h = (h ^ code).wrapping_mul(16777619);
    }
    GenerationRng::new(h)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Remainder {
    pub reward: i32,
    pub groups: i32,
    pub actors: i32,
    pub positive: i32,
    pub banshees: i32,
}
#[derive(Clone, Copy)]
struct Cost {
    reward: i32,
    size: i32,
    banshee: i32,
}
pub struct HordeFeasibility {
    costs: Vec<Cost>,
    memo: HashMap<Remainder, bool>,
    minimum: i32,
    maximum: i32,
}
impl HordeFeasibility {
    pub fn new(candidates: &[Candidate]) -> Self {
        let mut seen = HashSet::new();
        let mut costs = Vec::new();
        for c in candidates {
            let key = (
                c.blessings,
                c.kinds.len() as i32,
                c.kinds.iter().any(|x| x == "banshee") as i32,
            );
            if seen.insert(key) {
                costs.push(Cost {
                    reward: key.0,
                    size: key.1,
                    banshee: key.2,
                });
            }
        }
        costs.sort_by_key(|c| (c.size, c.reward));
        let minimum = costs.iter().map(|c| c.reward).min().unwrap_or(i32::MAX);
        let maximum = costs.iter().map(|c| c.reward).max().unwrap_or(i32::MIN);
        Self {
            costs,
            memo: HashMap::new(),
            minimum,
            maximum,
        }
    }
    pub fn after(s: Remainder, c: &Candidate) -> Remainder {
        Self::subtract(
            s,
            Cost {
                reward: c.blessings,
                size: c.kinds.len() as i32,
                banshee: c.kinds.iter().any(|x| x == "banshee") as i32,
            },
        )
    }
    fn subtract(s: Remainder, c: Cost) -> Remainder {
        Remainder {
            reward: s.reward - c.reward,
            groups: s.groups - 1,
            actors: s.actors - c.size,
            positive: 0.max(s.positive - (c.reward > 0) as i32),
            banshees: s.banshees - c.banshee,
        }
    }
    // Finite memoized DAG: groups decreases on every edge; bounded by budget48.
    pub fn can_finish(&mut self, s: Remainder) -> bool {
        if s.groups == 0 {
            return s.reward == 0 && s.actors == 0 && s.positive == 0 && s.banshees >= 0;
        }
        if s.groups < 0
            || s.reward < 0
            || s.banshees < 0
            || s.positive > s.groups
            || s.actors < s.groups
            || s.actors > s.groups * 5
            || self.costs.is_empty()
            || s.reward < self.minimum * s.groups
            || s.reward < s.positive
            || s.reward > self.maximum * s.groups
        {
            return false;
        }
        if let Some(v) = self.memo.get(&s) {
            return *v;
        }
        let mut result = false;
        for i in 0..self.costs.len() {
            if self.can_finish(Self::subtract(s, self.costs[i])) {
                result = true;
                break;
            }
        }
        self.memo.insert(s, result);
        result
    }
}
fn sizes(depth: u32) -> &'static [(usize, f64)] {
    if depth == 1 {
        &[(1, 0.5), (3, 0.5)]
    } else {
        &[(1, 0.6), (3, 0.3), (5, 0.1)]
    }
}
pub fn budgeted_candidates(depth: u32, water: bool) -> Result<&'static [Candidate], String> {
    catalog()
        .candidates
        .get(&format!("{depth}:{water}"))
        .map(|x| x.as_slice())
        .ok_or("Invalid population depth".into())
}
fn balanced_target(
    candidates: &[Candidate],
    budget: i32,
    depth: u32,
    banshees: i32,
) -> Option<(HordeFeasibility, Remainder)> {
    if !(2..=48).contains(&budget) || banshees > 1 {
        return None;
    }
    let usable: Vec<_> = candidates
        .iter()
        .filter(|c| {
            c.kinds.len() != 2
                && c.blessings <= budget
                && (banshees == 0 || !c.kinds.iter().any(|k| k == "banshee"))
        })
        .cloned()
        .collect();
    let mut solver = HordeFeasibility::new(&usable);
    let (mut expected, mut total) = (0.0, 0.0);
    for &(size, sw) in sizes(depth) {
        let pool: Vec<_> = usable.iter().filter(|c| c.kinds.len() == size).collect();
        let weight: f64 = pool.iter().map(|c| c.weight).sum();
        if weight == 0.0 {
            continue;
        }
        expected += sw
            * pool
                .iter()
                .map(|c| c.weight * c.blessings as f64)
                .sum::<f64>()
            / weight;
        total += sw;
    }
    let expected_groups = budget as f64 / (expected / total.max(0.01)).max(0.5);
    let mut groups: Vec<i32> = (2..=2 * budget).collect();
    groups.sort_by(|a, b| {
        ((*a as f64 - expected_groups).abs())
            .total_cmp(&(*b as f64 - expected_groups).abs())
            .then(a.cmp(b))
    });
    for difference in [0, -1, 1, -2, 2] {
        for &g in &groups {
            let actors = 2 * g + difference;
            if (actors as f64) < 1.5 * g as f64 || (actors as f64) > 2.5 * g as f64 {
                continue;
            }
            let state = Remainder {
                reward: budget,
                groups: g,
                actors,
                positive: 2.max((g + 1) / 2),
                banshees: 1 - banshees,
            };
            if solver.can_finish(state) {
                return Some((solver, state));
            }
        }
    }
    None
}
pub fn ordinary_horde_budget_feasible(depth: u32, budget: i32, water: bool, banshees: i32) -> bool {
    budgeted_candidates(depth, water)
        .ok()
        .and_then(|c| balanced_target(c, budget, depth, banshees))
        .is_some()
}
/// Revision-aware feasibility for generation's machine-budget admission. Frozen
/// callers retain revision29 recipes; revision38 stamps use the live roster.
pub fn ordinary_horde_budget_feasible_revision(depth: u32, budget: i32, water: bool, banshees: i32, revision: f64) -> bool {
    crate::ordinary_roster::candidates_revision(depth, water, revision)
        .ok()
        .and_then(|c| balanced_target(c, budget, depth, banshees))
        .is_some()
}
pub fn heart_blessings(kinds: &[String]) -> Result<i32, String> {
    kinds.iter().try_fold(0, |n, k| {
        catalog()
            .hearts
            .get(k)
            .copied()
            .or_else(|| crate::new_creatures::heart_reward(k))
            .map(|v| n + v)
            .ok_or_else(|| format!("Unknown ordinary heart reward: {k}"))
    })
}
fn weighted_index(weights: &[f64], rng: &mut GenerationRng) -> Result<usize, String> {
    if weights.is_empty() {
        return Err("No feasible horde composition".into());
    }
    let mut t = rng.random() * weights.iter().sum::<f64>();
    for (i, w) in weights.iter().enumerate() {
        t -= w;
        if t < 0.0 {
            return Ok(i);
        }
    }
    Ok(weights.len() - 1)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HordeSpawn {
    pub kinds: Vec<String>,
    #[serde(
        default,
        rename = "alwaysAwake",
        skip_serializing_if = "Field::is_missing"
    )]
    pub always_awake: Field<bool>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HordePlan {
    pub hordes: Vec<HordeSpawn>,
}
/// Frozen revision29 planner for differential callers; live startup uses the explicit revision API.
pub fn plan_budgeted_hordes(
    seed: &str,
    depth: u32,
    total: i32,
    machines: i32,
    water: bool,
    reserved: &[String],
) -> Result<HordePlan, String> {
    plan_budgeted_hordes_revision(seed, depth, total, machines, water, reserved, 29.0)
}
/// Explicit revision keeps frozen replay generation separate from current tuning.
pub fn plan_budgeted_hordes_revision(
    seed: &str,
    depth: u32,
    total: i32,
    machines: i32,
    water: bool,
    reserved: &[String],
    revision: f64,
) -> Result<HordePlan, String> {
    let budget = total - machines;
    if machines < 0 || budget < 2 {
        return Err("Machine hearts leave insufficient ordinary horde budget".into());
    }
    let candidates = match revision {
        29.0 => budgeted_candidates(depth, water)?,
        30.0 | 31.0 | 32.0 | 33.0 | 34.0 | 35.0 | 36.0 | 37.0 | 38.0 | 39.0 | 40.0 | 41.0 | 42.0 | 43.0 | 44.0 | 45.0 | 46.0 | 50.0 | 51.0 | 52.0 => crate::ordinary_roster::candidates_revision(depth, water, revision)?,
        _ => return Err("Unsupported population revision".into()),
    };
    let banshees = reserved.iter().filter(|k| *k == "banshee").count() as i32;
    let (mut solver, mut state) = balanced_target(candidates, budget, depth, banshees)
        .ok_or("No balanced exact ordinary horde budget")?;
    let mut rng = population_random(&format!("{seed}:floor:{depth}:horde-composition"));
    let mut hordes = Vec::new();
    let mut planned = state.actors;
    let original = state.groups;
    while state.groups > 0 {
        let feasible: Vec<_> = candidates
            .iter()
            .filter(|c| c.kinds.len() != 2 && solver.can_finish(HordeFeasibility::after(state, c)))
            .collect();
        let mut pairs = Vec::new();
        for c in candidates
            .iter()
            .filter(|c| c.kinds.len() == 2 && catalog().pairs.get(&c.kinds[0]) == Some(&c.kinds[1]))
        {
            let after = HordeFeasibility::after(state, c);
            for adjustment in [0, -1, 1, -2, 2] {
                let actors = planned + adjustment;
                if (actors - 2 * original).abs() > 2
                    || (actors as f64) < 1.5 * original as f64
                    || (actors as f64) > 2.5 * original as f64
                {
                    continue;
                }
                let next = Remainder {
                    actors: after.actors + adjustment,
                    ..after
                };
                if solver.can_finish(next) {
                    pairs.push((c, next, actors));
                    break;
                }
            }
        }
        let ordinary: Vec<_> = sizes(depth)
            .iter()
            .filter(|(s, _)| feasible.iter().any(|c| c.kinds.len() == *s))
            .collect();
        let pair_weight = pairs.iter().map(|(c, _, _)| c.weight).fold(0.0, f64::max);
        let selected = if !pairs.is_empty() && rng.random() < 0.2 * pair_weight {
            let i = weighted_index(
                &pairs.iter().map(|(c, _, _)| c.weight).collect::<Vec<_>>(),
                &mut rng,
            )?;
            state = pairs[i].1;
            planned = pairs[i].2;
            pairs[i].0
        } else {
            let i = weighted_index(
                &ordinary.iter().map(|(_, w)| *w).collect::<Vec<_>>(),
                &mut rng,
            )?;
            let pool: Vec<_> = feasible
                .into_iter()
                .filter(|c| c.kinds.len() == ordinary[i].0)
                .collect();
            let j = weighted_index(&pool.iter().map(|c| c.weight).collect::<Vec<_>>(), &mut rng)?;
            state = HordeFeasibility::after(state, pool[j]);
            pool[j]
        };
        hordes.push(HordeSpawn {
            kinds: selected.kinds.clone(),
            always_awake: Field::Missing,
        });
    }
    if !solver.can_finish(state) {
        return Err("Incomplete ordinary horde plan".into());
    }
    Ok(HordePlan { hordes })
}
/// Exact per-depth Godbeast template, with revisioned numerical overlay only.
pub fn zillennia_revision(depth: u32, revision: f64) -> Result<Actor, String> {
    let mut a = catalog().zillennia.get(depth.checked_sub(1).ok_or("Invalid Zillennia depth")? as usize)
        .cloned().ok_or("Invalid Zillennia depth")?;
    if revision == 52. {
        a = catalog().zillennia.iter().find(|first| first.zillennia_quality == a.zillennia_quality)
            .expect("requested quality exists").clone();
    }
    if revision >= 32.0 {
        let quality = a.zillennia_quality.as_ref().map_or("unqualified", String::as_str);
        let kind = format!("zillennia:{quality}");
        crate::physical_balance::apply_new_actor_kind(&mut a, &kind, revision);
    }
    Ok(a)
}

/// Same named fungus uses its weakest first template on every R52 floor.
/// Legacy depth tiers remain exact. O(1), no RNG or actor mutation.
pub fn fungus_revision(depth: u32, revision: f64) -> Result<Actor, String> {
    if !(1..=40).contains(&depth) { return Err("Invalid fungus depth".into()); }
    let index = if revision == 52. { 0 } else { depth as usize - 1 };
    let mut a = catalog().fungi.get(index).cloned().ok_or("Invalid fungus depth")?;
    a.stats.accuracy = crate::combat::creature_birth_accuracy(revision, a.stats.accuracy);
    a.stats.dodge = crate::combat::creature_birth_dodge(revision, a.stats.dodge);
    Ok(a)
}

pub fn create_actor(kind: &str, id: f64, x: f64, y: f64, sleeping: bool) -> Result<Actor, String> {
    create_actor_revision(kind, id, x, y, sleeping, 29.0)
}
/// Explicit actor revision preserves frozen differential callers and saved-run generation.
pub fn create_actor_revision(kind: &str, id: f64, x: f64, y: f64, sleeping: bool, revision: f64) -> Result<Actor, String> {
    if ![29.0, 30.0, 31.0, 32.0, 33.0, 34.0, 35.0, 36.0, 37.0, 38.0, 39.0, 40.0, 41.0, 42.0, 43.0, 44.0, 45.0, 46.0, 50.0, 51.0, 52.0].contains(&revision) { return Err("Unsupported actor revision".into()); }
    let mut a = catalog()
        .actors
        .get(kind)
        .cloned()
        .or_else(|| if revision >= 33.0 { crate::new_creatures::template(kind).cloned() } else { None })
        .ok_or_else(|| format!("Unknown creature {kind}"))?;
    crate::physical_balance::apply_new_actor_kind(&mut a, kind, revision);
    crate::revision50_combat::mark_new_caster(&mut a, kind, revision);
    a.id = id;
    a.x = x;
    a.y = y;
    crate::new_creatures::initialize_disguise(&mut a);
    if !["sphinx", "hemogoblin-mech", "ursa-minor", "ursa-major"].contains(&kind) {
        a.state = if sleeping { "sleeping" } else { "wandering" }.into();
    }
    if revision >= 50.0 && crate::corpses::is_bastidion(&a) {
        a.conditions.insert("fungalGasType".into(), crate::fungi::HALLUCINOGENIC_KIND);
    }
    if revision >= 50. { crate::revision50_creature_identity::apply_actor(&mut a); }
    Ok(a)
}
pub fn create_population_actor(
    next_id: &mut f64,
    f: &mut Floor,
    kind: &str,
    at: usize,
    sleeping: bool,
    rng: &mut GenerationRng,
) -> Result<Actor, String> {
    create_population_actor_revision(next_id, f, kind, at, sleeping, rng, 29.0)
}
pub fn create_population_actor_revision(
    next_id: &mut f64,
    f: &mut Floor,
    kind: &str,
    at: usize,
    sleeping: bool,
    rng: &mut GenerationRng,
    revision: f64,
) -> Result<Actor, String> {
    if ![29.0, 30.0, 31.0, 32.0, 33.0, 34.0, 35.0, 36.0, 37.0, 38.0, 39.0, 40.0, 41.0, 42.0, 43.0, 44.0, 45.0, 46.0, 50.0, 51.0, 52.0].contains(&revision) { return Err("Unsupported actor revision".into()); }
    if revision >= 51. && crate::revision51_stairs::stair_cell(f,at) {
        return Err("Enemies cannot spawn on stairs".into());
    }
    if revision >=52. && crate::borge::reserved_cell(f,at) {
        return Err("Enemies cannot spawn on the Borge or its altar".into());
    }
    let w = f.width();
    let (x, y) = ((at % w) as f64, (at / w) as f64);
    let mut a = if ["fungus", "bastidion", "vent-fungus"].contains(&kind) {
        let mut a = fungus_revision(f.depth as u32, revision)?;
        // R50's fixed identity needs no random draw; earlier seeds retain exactly
        // their original six-way selection and generation RNG advancement.
        let gas = if revision >= 50.0 { crate::fungi::HALLUCINOGENIC_KIND }
            else { (rng.random() * 6.0).floor() };
        a.conditions.insert("fungalGasType".into(), gas);
        a.id = *next_id;
        a.x = x;
        a.y = y;
        a.state = if sleeping { "sleeping" } else { "wandering" }.into();
        a
    } else {
        create_actor_revision(kind, *next_id, x, y, sleeping, revision)?
    };
    if revision >= 50. { crate::revision50_creature_identity::apply_actor(&mut a); }
    *next_id += 1.0;
    if kind == "hemogoblin-inventor" && rng.random() < 0.75 {
        let effect =
            &catalog().potions[(rng.random() * catalog().potions.len() as f64).floor() as usize];
        let item = Item {
            id: *next_id,
            name: "Unidentified potion".into(),
            kind: "potion".into(),
            glyph: "!".into(),
            effect: Field::Value(effect.clone()),
            identified: Field::Value(false),
            enhancements: vec![],
            ..Default::default()
        };
        *next_id += 1.0;
        a.monster_state = Field::Value(crate::ActorMonsterState {
            carried_items: Field::Value(vec![item]),
            ..Default::default()
        });
    }
    if a.abilities.iter().any(|k| k == "enemy:beast:slime-trail") {
        let mut slime = f.slime.value_or(vec![0.0; f.tiles.len()]);
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (xx, yy) = (x as i32 + dx, y as i32 + dy);
                if xx >= 0 && yy >= 0 && (xx as usize) < w && (yy as usize) < f.height() {
                    let i = yy as usize * w + xx as usize;
                    if !["wall", "tree", "chasm"].contains(&f.tiles[i].as_str()) {
                        slime[i] = 1.0
                    }
                }
            }
        }
        f.slime = Field::Value(slime);
    }
    crate::balance52::apply_new_actor_kind(&mut a, kind, f.depth, revision);
    Ok(a)
}

pub fn floor_threat(depth: f64) -> f64 {
    2.0 + (16.0 / 15.0) * (depth - 1.0).clamp(0.0, 15.0) + 0.25 * (depth - 16.0).max(0.0)
}
pub fn creature_danger(kind: &str, depth: f64) -> Result<f64, String> {
    if ["fungus", "vent-fungus", "bastidion"].contains(&kind) {
        return Ok(3.0 + depth * 0.025);
    }
    if kind.starts_with("pale:Greater ") {
        return Ok(1.8 * creature_danger(&kind.replacen("Greater ", "", 1), depth)?);
    }
    catalog()
        .danger
        .get(kind)
        .copied()
        .or_else(|| crate::new_creatures::danger(kind))
        .ok_or_else(|| format!("No encounter danger for {kind}"))
}
pub fn encounter_score(depth: f64, kinds: &[String]) -> Result<f64, String> {
    let sum = kinds
        .iter()
        .try_fold(0.0, |n, k| creature_danger(k, depth).map(|s| n + s))?;
    let crowd = 1.0 + 0.06 * (kinds.len().saturating_sub(1).min(5) as f64);
    let synergy = if (kinds.iter().any(|k| k == "maulchild")
        && kinds.iter().any(|k| k != "maulchild"))
        || (kinds
            .iter()
            .any(|k| k.starts_with("pale:") && k.contains("Breather"))
            && kinds
                .iter()
                .any(|k| k.starts_with("pale:") && (k.contains("Biter") || k.contains("Spitter"))))
    {
        1.1
    } else {
        1.0
    };
    Ok((sum * crowd * synergy * 1000.0).round() / 1000.0)
}
pub fn encounter_danger(depth: f64, score: f64) -> &'static str {
    let r = score / floor_threat(depth);
    if r < 1.0 {
        "low"
    } else if r < 2.0 {
        "medium"
    } else if r < 3.5 {
        "high"
    } else {
        "extreme"
    }
}
pub fn validate_encounter_metadata(records: &Field<Vec<crate::EncounterRecord>>) -> bool {
    let Some(records) = records.as_ref() else {
        return records.is_missing();
    };
    if records.len() > 3600 {
        return false;
    }
    let mut ids = HashSet::new();
    let mut actors = HashSet::new();
    let int = |v: f64| v.is_finite() && v.fract() == 0.0 && v.abs() <= 9007199254740991.0;
    for e in records {
        if !int(e.id)
            || e.id < 1.0
            || !ids.insert(e.id as u64)
            || !["low", "medium", "high", "extreme"].contains(&e.danger.as_str())
            || !e.score.is_finite()
            || e.score <= 0.0
            || !e.floor_threat.is_finite()
            || e.floor_threat <= 0.0
            || e.actor_ids.is_empty()
            || e.actor_ids.len() > 12
            || e.setup_indices.len() > 20
            || e.setup_indices.iter().any(|v| !int(*v) || *v < 0.0)
        {
            return false;
        }
        if let Some(kind) = e.kind.as_ref() {
            if !["horde", "machine", "verdania-guard"].contains(&kind.as_str()) {
                return false;
            }
        } else if !e.kind.is_missing() {
            return false;
        }
        if e.kind.as_ref().is_some_and(|k| k == "horde" || k == "verdania-guard") {
            if e.leader_id.as_ref() != e.actor_ids.first()
                || e.follower_ids.as_ref().map(|v| v.as_slice()) != Some(&e.actor_ids[1..])
                || !e.setup_indices.is_empty()
            {
                return false;
            }
        } else if !e.leader_id.is_missing() || !e.follower_ids.is_missing() {
            return false;
        }
        for id in &e.actor_ids {
            if !int(*id) || *id < 1.0 || !actors.insert(*id as u64) {
                return false;
            }
        }
    }
    true
}
pub fn blocked_cells(f: &Floor) -> HashSet<usize> {
    let mut blocked: HashSet<_> = crate::spawn_heatmap::fixture_cells(f).into_iter().collect();
    blocked.extend(crate::pocket_biomes::reserved(f));
    if let Some(s) = f.sphinx_room.as_ref() {
        blocked.insert(s.y as usize * f.width() + s.x as usize);
        blocked.extend(s.cells.iter().chain(&s.reward_cells).map(|v| *v as usize));
    }
    blocked
}
#[derive(Clone)]
pub struct MachineGroup {
    pub intents: Vec<crate::FloorSetupSpawnsEntry>,
    pub setup_indices: Vec<f64>,
}
pub struct MachinePlan {
    pub groups: Vec<MachineGroup>,
    pub corpse_cells: Vec<usize>,
    pub reserved_cells: HashSet<usize>,
    pub fungus_feasts: Vec<crate::fungus_feasts::Feast>,
}
pub fn plan_machine_spawns(f: &Floor, blocked: &HashSet<usize>) -> Result<MachinePlan, String> {
    plan_machine_spawns_revision(f, blocked, 29.0)
}
pub fn plan_machine_spawns_revision(
    f: &Floor, blocked: &HashSet<usize>, revision: f64,
) -> Result<MachinePlan, String> {
    let (w, h) = (f.width(), f.height());
    let mut groups: IndexMap<String, MachineGroup> = IndexMap::new();
    let mut reserved = HashSet::new();
    let mut corpses = Vec::new();
    for (i, intent) in f.setup_spawns.as_ref().into_iter().flatten().enumerate() {
        let setup = intent
            .setup_index
            .as_ref()
            .copied()
            .or_else(|| {
                f.setups
                    .as_ref()
                    .and_then(|ss| {
                        ss.iter().position(|s| {
                            crate::spawn_heatmap::setup_contains(s, intent.x, intent.y)
                        })
                    })
                    .map(|i| i as f64)
            })
            .unwrap_or(-1.0);
        let key = if setup >= 0.0 {
            format!("setup:{setup}")
        } else {
            format!("legacy:{i}")
        };
        let at = intent.y as usize * w + intent.x as usize;
        if intent.x < 0.0
            || intent.y < 0.0
            || intent.x.fract() != 0.0
            || intent.y.fract() != 0.0
            || intent.x >= w as f64
            || intent.y >= h as f64
            || reserved.contains(&at)
            || blocked.contains(&at)
            || ["wall", "tree", "chasm", "pit", "up", "down"].contains(&f.tiles[at].as_str())
        {
            return Err("Required setup creature cannot occupy coordinate".into());
        }
        let group = groups.entry(key).or_insert_with(|| MachineGroup {
            intents: vec![],
            setup_indices: if setup >= 0.0 { vec![setup] } else { vec![] },
        });
        group.intents.push(intent.clone());
        if group.intents.len() > 12 {
            return Err("Required setup exceeds maximum machine group size".into());
        }
        reserved.insert(at);
    }
    let mut fungus_feasts = Vec::new();
    if revision >= 51.0 {
        fungus_feasts = crate::fungus_feasts::plan(f, blocked, &mut reserved, revision)?;
        let supplied: HashSet<_> = fungus_feasts.iter().map(|feast| feast.source).collect();
        for group in groups.values_mut() {
            group.intents.retain(|intent| {
                let at = intent.y as usize * w + intent.x as usize;
                let keep = !crate::fungus_feasts::is_fungus(&intent.kind) || supplied.contains(&at);
                if !keep { reserved.remove(&at); }
                keep
            });
        }
        groups.retain(|_, group| !group.intents.is_empty());
        corpses.extend(fungus_feasts.iter().flat_map(|feast| feast.cells));
    } else {
    for intent in f.setup_spawns.as_ref().into_iter().flatten() {
        if ["fungus", "bastidion", "vent-fungus"].contains(&intent.kind.as_str()) {
            for delta in [-2.0, 2.0] {
                let x = intent.x + delta;
                if x < 0.0 || x >= w as f64 {
                    continue;
                }
                let at = intent.y as usize * w + x as usize;
                if reserved.contains(&at) || blocked.contains(&at) || f.tiles[at] != "floor" {
                    continue;
                }
                reserved.insert(at);
                corpses.push(at);
            }
        }
    }
    }
    Ok(MachinePlan {
        groups: groups.into_values().collect(),
        corpse_cells: corpses,
        reserved_cells: reserved,
        fungus_feasts,
    })
}
pub fn choose_sleep_states(always: &[bool], rng: &mut GenerationRng) -> Vec<bool> {
    let mut sleeping: Vec<_> = always
        .iter()
        .map(|a| {
            let roll = rng.random();
            !*a && roll < 0.3
        })
        .collect();
    let mut awake = sleeping.iter().filter(|v| !**v).count();
    let mut eligible: Vec<_> = sleeping
        .iter()
        .enumerate()
        .filter_map(|(i, s)| if *s { Some(i) } else { None })
        .collect();
    while awake < 2.min(sleeping.len()) {
        let i = (rng.random() * eligible.len() as f64).floor() as usize;
        sleeping[eligible.remove(i)] = false;
        awake += 1;
    }
    sleeping
}
pub fn connected_horde_cells(
    f: &Floor,
    anchor: usize,
    allowed: &HashSet<usize>,
    radius: i32,
    limit: usize,
) -> Vec<usize> {
    if !allowed.contains(&anchor) {
        return vec![];
    }
    let (w, h) = (f.width() as i32, f.height() as i32);
    let (x, y) = ((anchor % f.width()) as i32, (anchor / f.width()) as i32);
    let mut cells = vec![anchor];
    let mut seen = HashSet::from([anchor]);
    let mut head = 0;
    if cells.len() >= limit {
        return cells;
    }
    while head < cells.len() {
        let at = cells[head];
        head += 1;
        let (xx, yy) = ((at % f.width()) as i32, (at / f.width()) as i32);
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let (nx, ny) = (xx + dx, yy + dy);
                if nx < 0
                    || ny < 0
                    || nx >= w
                    || ny >= h
                    || (nx - x).pow(2) + (ny - y).pow(2) > radius.pow(2)
                {
                    continue;
                }
                let next = (ny * w + nx) as usize;
                if seen.contains(&next)
                    || !allowed.contains(&next)
                    || (dx != 0
                        && dy != 0
                        && (!allowed.contains(&((yy * w + nx) as usize))
                            || !allowed.contains(&((ny * w + xx) as usize))))
                {
                    continue;
                }
                seen.insert(next);
                cells.push(next);
                if cells.len() >= limit {
                    return cells;
                }
            }
        }
    }
    cells
}
fn record_group(
    f: &Floor,
    kinds: &[String],
    sleeping: bool,
    members: &[Actor],
    kind: &str,
    setup_indices: Vec<f64>,
) -> Result<crate::EncounterRecord, String> {
    let score = encounter_score(f.depth, kinds)?;
    let ids: Vec<_> = members.iter().map(|a| a.id).collect();
    Ok(crate::EncounterRecord {
        id: f.encounters.as_ref().map_or(0, |v| v.len()) as f64 + 1.0,
        danger: encounter_danger(f.depth, score).into(),
        score,
        floor_threat: floor_threat(f.depth),
        sleeping,
        leader_id: if kind == "horde" {
            ids[0].into()
        } else {
            Field::Missing
        },
        follower_ids: if kind == "horde" {
            ids[1..].to_vec().into()
        } else {
            Field::Missing
        },
        actor_ids: ids,
        setup_indices,
        kind: kind.to_string().into(),
    })
}
fn habitats(f: &Floor) -> (Vec<usize>, Vec<usize>) {
    let map = crate::spawn_heatmap::build_spawn_heatmap(f);
    let (w, h) = (f.width(), f.height());
    let up = f.tiles.iter().position(|t| t == "up");
    let room = f.godbeast_event.as_ref().map(|e| &e.room);
    let rewards: HashSet<_> = f
        .setups
        .as_ref()
        .into_iter()
        .flatten()
        .filter(|s| s.id == 67.0)
        .map(|s| s.y as usize * w + s.x as usize + 4)
        .collect();
    let safe = |at: &usize| {
        let (x, y) = ((at % w) as f64, (at / w) as f64);
        up.is_none_or(|u| (x - (u % w) as f64).powi(2) + (y - (u / w) as f64).powi(2) > 25.0)
            && !rewards.contains(at)
            && !room.is_some_and(|r| x >= r.left && x <= r.right && y >= r.top && y <= r.bottom)
    };
    let wet: HashSet<_> = map.water.into_iter().collect();
    let mut seen = vec![false; f.tiles.len()];
    let mut water = Vec::new();
    for at in 0..f.tiles.len() {
        if seen[at] || !["water", "shallow-water"].contains(&f.tiles[at].as_str()) {
            continue;
        }
        seen[at] = true;
        let mut component = vec![at];
        let mut head = 0;
        while head < component.len() {
            let c = component[head];
            head += 1;
            let (x, y) = ((c % w) as i32, (c / w) as i32);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (xx, yy) = (x + dx, y + dy);
                    if xx < 0 || yy < 0 || xx as usize >= w || yy as usize >= h {
                        continue;
                    }
                    let next = yy as usize * w + xx as usize;
                    if !seen[next] && ["water", "shallow-water"].contains(&f.tiles[next].as_str()) {
                        seen[next] = true;
                        component.push(next)
                    }
                }
            }
        }
        if component.len() >= 24 {
            water.extend(component.into_iter().filter(|c| wet.contains(c) && safe(c)));
        }
    }
    (map.land.into_iter().filter(safe).collect(), water)
}
/// Whole groups are selected before placement. O(H*V*r² + H*M*V), r<=6, M<=12.
/// Frozen revision29 population for differential callers; live startup supplies its saved revision.
pub fn populate_hordes(
    seed: &str,
    next_id: &mut f64,
    f: &mut Floor,
    ground: &[crate::GroundItem],
    plan: Option<&HordePlan>,
) -> Result<Vec<Actor>, String> {
    populate_hordes_revision(seed, next_id, f, ground, plan, 29.0)
}
pub fn populate_hordes_revision(
    seed: &str,
    next_id: &mut f64,
    f: &mut Floor,
    ground: &[crate::GroundItem],
    plan: Option<&HordePlan>,
    revision: f64,
) -> Result<Vec<Actor>, String> {
    if ![29.0, 30.0, 31.0, 32.0, 33.0, 34.0, 35.0, 36.0, 37.0, 38.0, 39.0, 40.0, 41.0, 42.0, 43.0, 44.0, 45.0, 46.0, 50.0, 51.0, 52.0].contains(&revision) { return Err("Unsupported population revision".into()); }
    let (land, water) = habitats(f);
    let blocked = blocked_cells(f);
    let machines = plan_machine_spawns_revision(f, &blocked, revision)?;
    let reserved: Vec<_> = machines.groups.iter()
        .flat_map(|group| group.intents.iter().map(|intent| intent.kind.clone())).collect();
    let generated;
    let plan = if let Some(p) = plan {
        p
    } else {
        let budget = f
            .enemy_blessing_budget
            .as_ref()
            .copied()
            .map(|v| v as i32)
            .unwrap_or(crate::generation::ordinary_blessing_budget(
                crate::generation::BudgetSeed::Text(seed),
                f.depth as u32,
            )? as i32);
        generated = plan_budgeted_hordes_revision(
            seed,
            f.depth as u32,
            budget,
            heart_blessings(&reserved)?,
            water.len() >= 24,
            &reserved,
            revision,
        )?;
        &generated
    };
    if plan.hordes.len() < 2 || plan.hordes.len() > f.tiles.len() {
        return Err("An ordinary population plan requires at least two bounded hordes".into());
    }
    for h in &plan.hordes {
        if h.kinds.is_empty() || h.kinds.len() > 12 {
            return Err("Invalid explicit horde composition".into());
        }
        let dryads = h.kinds.iter().filter(|k| *k == "dryad").count();
        if dryads > 0 && (dryads != 1 || h.kinds.len() < 2) {
            return Err("A Dryad horde requires one Dryad grouped with other enemies".into());
        }
        for k in &h.kinds {
            if (catalog().pairs.contains_key(k) || catalog().pairs.values().any(|v| v == k))
                && (h.kinds.len() != 2 || catalog().pairs.get(&h.kinds[0]) != Some(&h.kinds[1]))
            {
                return Err("Paired creatures require complete authored composition".into());
            }
            if k == "sphinx" {
                return Err("Sphinx requires structural guarded room".into());
            }
        }
        encounter_score(f.depth, &h.kinds)?;
        let wet = h
            .kinds
            .iter()
            .filter(|k| catalog().aquatic.get(*k) == Some(&true))
            .count();
        if wet > 0 && wet != h.kinds.len() {
            return Err("Horde cannot mix incompatible habitats".into());
        }
    }
    if plan
        .hordes
        .iter()
        .flat_map(|h| h.kinds.iter())
        .chain(reserved.iter())
        .filter(|k| *k == "banshee")
        .count()
        > 1
    {
        return Err("A floor cannot spawn more than one Banshee".into());
    }
    let w = f.width();
    let map = crate::spawn_heatmap::build_spawn_heatmap(f);
    let mut sampler = crate::spawn_heatmap::SpawnSampler::new(
        map,
        land.iter().chain(&water).copied().collect(),
        population_random(&format!("{seed}:floor:{}:horde-placement", f.depth)),
        machines.reserved_cells,
    );
    sampler
        .occupied
        .extend(ground.iter().map(|g| g.y as usize * w + g.x as usize));
    let always: Vec<_> = plan
        .hordes
        .iter()
        .map(|h| {
            h.always_awake.value_or(false)
                || h.kinds
                    .iter()
                    .any(|k| catalog().awake.get(k) == Some(&true))
        })
        .collect();
    let sleeping = choose_sleep_states(
        &always,
        &mut population_random(&format!("{seed}:floor:{}:horde-sleep", f.depth)),
    );
    let mut rng = population_random(&format!("{seed}:floor:{}:horde-actors", f.depth));
    let mut actors = Vec::new();
    f.encounters = vec![].into();
    let mut fuel = HashMap::new();
    for k in plan.hordes.iter().flat_map(|h| h.kinds.iter()) {
        fuel.entry(k.clone())
            .or_insert_with(|| crate::generation::initial_fuel_spawn_exclusions(f, k));
    }
    for (index, horde) in plan.hordes.iter().enumerate() {
        let candidates = if catalog().aquatic.get(&horde.kinds[0]) == Some(&true) {
            &water
        } else {
            &land
        };
        let available: Vec<_> = candidates
            .iter()
            .copied()
            .filter(|at| {
                !blocked.contains(at)
                    && !(revision>=52. && crate::borge::reserved_cell(f,*at))
                    && !sampler.occupied.contains(at)
                    && horde.kinds.iter().all(|k| !fuel[k].contains(at))
            })
            .collect();
        let allowed: HashSet<_> = available.iter().copied().collect();
        let mut chosen = None;
        for radius in [4, 6] {
            let anchors: Vec<_> = available
                .iter()
                .copied()
                .filter(|at| {
                    horde.kinds.len() == 1
                        || connected_horde_cells(f, *at, &allowed, radius, horde.kinds.len()).len()
                            >= horde.kinds.len()
                })
                .collect();
            let Some(anchor) = sampler.pick(Some(&anchors)) else {
                continue;
            };
            let mut local = if horde.kinds.len() == 1 {
                vec![]
            } else {
                connected_horde_cells(f, anchor, &allowed, radius, usize::MAX)
                    .into_iter()
                    .filter(|at| *at != anchor)
                    .collect()
            };
            let mut cells = vec![anchor];
            while cells.len() < horde.kinds.len() {
                let next = sampler
                    .pick(Some(&local))
                    .ok_or("Validated horde lost follower cells")?;
                cells.push(next);
                local.remove(local.iter().position(|at| *at == next).unwrap());
            }
            chosen = Some(cells);
            break;
        }
        let mut cells = chosen.ok_or("No space for complete horde")?;
        if horde.kinds == ["hemogoblin-inventor", "hemogoblin-mech"] {
            let (x, y) = ((cells[0] % w) as f64, (cells[0] / w) as f64);
            let distant: Vec<_> = available
                .iter()
                .copied()
                .filter(|at| {
                    *at != cells[0] && ((at % w) as f64 - x).hypot((at / w) as f64 - y) >= 8.0
                })
                .collect();
            let fallback: Vec<_> = available
                .iter()
                .copied()
                .filter(|at| *at != cells[0])
                .collect();
            cells[1] = sampler
                .pick(Some(if distant.is_empty() {
                    &fallback
                } else {
                    &distant
                }))
                .ok_or("No safe inventor mech location")?;
        }
        let mut members = Vec::new();
        for (k, at) in horde.kinds.iter().zip(&cells) {
            let a = create_population_actor_revision(next_id, f, k, *at, sleeping[index], &mut rng, revision)?;
            sampler.occupied.insert(*at);
            members.push(a);
        }
        if horde.kinds.len() == 2 && catalog().pairs.get(&horde.kinds[0]) == Some(&horde.kinds[1]) {
            let ids = [members[0].id, members[1].id];
            if horde.kinds[0] == "hemogoblin-inventor" {
                members[0]
                    .conditions
                    .insert("monsterLinkedMechId".into(), ids[1]);
                members[1]
                    .conditions
                    .insert("monsterLinkedInventorId".into(), ids[0]);
            } else {
                members[0]
                    .conditions
                    .insert("monsterUrsaPartnerId".into(), ids[1]);
                members[1]
                    .conditions
                    .insert("monsterUrsaPartnerId".into(), ids[0]);
            }
        }
        let record = record_group(f, &horde.kinds, sleeping[index], &members, "horde", vec![])?;
        f.encounters.as_mut().unwrap().push(record);
        actors.extend(members);
        sampler.add_spacing_anchor(cells[0]);
    }
    crate::generation::record_generation_stage(
        f,
        "hordes",
        Some(IndexMap::from([
            ("hordes".into(), plan.hordes.len() as f64),
            ("actors".into(), actors.len() as f64),
            (
                "sleeping".into(),
                sleeping.iter().filter(|s| **s).count() as f64,
            ),
            (
                "awake".into(),
                sleeping.iter().filter(|s| !**s).count() as f64,
            ),
        ])),
    )?;
    Ok(actors)
}
pub fn populate_machine_enemies(
    seed: &str,
    next_id: &mut f64,
    f: &mut Floor,
    ordinary: &[Actor],
    ground: &[crate::GroundItem],
) -> Result<Vec<Actor>, String> {
    populate_machine_enemies_revision(seed, next_id, f, ordinary, ground, 29.0)
}
pub fn populate_machine_enemies_revision(
    seed: &str,
    next_id: &mut f64,
    f: &mut Floor,
    ordinary: &[Actor],
    ground: &[crate::GroundItem],
    revision: f64,
) -> Result<Vec<Actor>, String> {
    if ![29.0, 30.0, 31.0, 32.0, 33.0, 34.0, 35.0, 36.0, 37.0, 38.0, 39.0, 40.0, 41.0, 42.0, 43.0, 44.0, 45.0, 46.0, 50.0, 51.0, 52.0].contains(&revision) { return Err("Unsupported actor revision".into()); }
    let w = f.width();
    let machines = plan_machine_spawns_revision(f, &blocked_cells(f), revision)?;
    if f.encounters.as_ref().is_some_and(|rs| {
        rs.iter()
            .any(|r| r.kind.as_ref().is_some_and(|k| k == "machine"))
    }) {
        return Err("Machine enemies already populated".into());
    }
    let occupied: HashSet<_> = ordinary
        .iter()
        .map(|a| a.y as usize * w + a.x as usize)
        .chain(ground.iter().map(|g| g.y as usize * w + g.x as usize))
        .collect();
    if machines
        .reserved_cells
        .iter()
        .any(|at| occupied.contains(at))
    {
        return Err("Required machine reservation occupied".into());
    }
    for group in &machines.groups {
        for intent in &group.intents {
            if crate::generation::initial_fuel_spawn_exclusions(f, &intent.kind)
                .contains(&(intent.y as usize * w + intent.x as usize))
            {
                return Err("Required setup overlaps permanent fire".into());
            }
        }
    }
    let mut actors = Vec::new();
    let mut rng = population_random(&format!("{seed}:floor:{}:machine-enemies", f.depth));
    f.encounters.get_or_insert_with(Vec::new);
    for group in &machines.groups {
        let mut members = Vec::new();
        for intent in &group.intents {
            members.push(create_population_actor_revision(
                next_id,
                f,
                &intent.kind,
                intent.y as usize * w + intent.x as usize,
                false,
                &mut rng,
                revision,
            )?);
        }
        let kinds = group
            .intents
            .iter()
            .map(|i| i.kind.clone())
            .collect::<Vec<_>>();
        let record = record_group(
            f,
            &kinds,
            false,
            &members,
            "machine",
            group.setup_indices.clone(),
        )?;
        f.encounters.as_mut().unwrap().push(record);
        actors.extend(members);
    }
    if revision >= 51.0 {
        crate::fungus_feasts::populate(seed, next_id, f, &mut actors, &machines.fungus_feasts, revision)?;
    } else {
    for at in &machines.corpse_cells {
        let mut a = create_actor_revision(
            if revision >= 32.0 { crate::flavor_placement::corpse_kind(f.depth as u32) } else { &catalog().corpses[f.depth as usize - 1] },
            *next_id,
            (*at % w) as f64,
            (*at / w) as f64,
            false,
            revision,
        )?;
        *next_id += 1.0;
        a.dead = true.into();
        a.hp = 0.0;
        a.heartless = true.into();
        a.corpse_age = 0.0.into();
        actors.push(a);
    }
    }
    crate::generation::record_generation_stage(
        f,
        "machine-enemies",
        Some(IndexMap::from([
            ("groups".into(), machines.groups.len() as f64),
            (
                "actors".into(),
                actors.iter().filter(|a| !a.dead.value_or(false)).count() as f64,
            ),
            ("corpses".into(), machines.corpse_cells.len() as f64),
        ])),
    )?;
    Ok(actors)
}
pub fn setup_encounter_rating(depth: f64, kinds: &[String]) -> Result<Option<String>, String> {
    if kinds.is_empty() {
        return Ok(None);
    }
    let danger = encounter_danger(depth, encounter_score(depth, kinds)?);
    Ok(
        if danger == "extreme" || (depth <= 3.0 && danger == "high") {
            None
        } else {
            Some(danger.into())
        },
    )
}

pub fn select_enemy_kind(
    depth: u32,
    families: &[String],
    rng: &mut GenerationRng,
    permitted: impl Fn(&str) -> bool,
) -> Result<String, String> {
    let choices: Vec<_> = catalog()
        .singles
        .get(&depth.to_string())
        .ok_or("Invalid single creature depth")?
        .iter()
        .filter(|c| families.contains(&c.family) && permitted(&c.id))
        .collect();
    let i = weighted_index(&choices.iter().map(|c| c.weight).collect::<Vec<_>>(), rng)?;
    Ok(choices[i].id.clone())
}
pub fn low_end_water_setup_kind(
    depth: u32,
    families: Option<&[String]>,
) -> Result<(String, bool), String> {
    let all = &crate::generation::generation_bestiary().families;
    let families = families.unwrap_or(all);
    let choices = catalog()
        .singles
        .get(&depth.to_string())
        .ok_or("Invalid single creature depth")?;
    let pool: Vec<_> = choices
        .iter()
        .filter(|c| {
            families.contains(&c.family)
                && c.id != "dryad"
                && !(c.id.starts_with("pale:") && c.id.contains("Breather"))
                && !catalog().actors.get(&c.id).is_some_and(|a| {
                    a.abilities
                        .iter()
                        .any(|id| id == "enemy:beast:slime-trail" || id == "enemy:beast:fungal-gas")
                })
        })
        .collect();
    let mut pick = 0.0;
    for c in &pool {
        pick -= c.weight;
        if pick < 0.0 {
            return Ok((c.id.clone(), catalog().aquatic.get(&c.id) == Some(&true)));
        }
    }
    let c = pool.last().ok_or("No eligible single creature")?;
    Ok((c.id.clone(), catalog().aquatic.get(&c.id) == Some(&true)))
}
