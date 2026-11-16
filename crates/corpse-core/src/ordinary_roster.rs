//! Revision30 widens each ordinary species' frozen window by two floors.
//! Conditional encounters retain their own eligibility; pairs follow their leader.
use crate::population::{Candidate, catalog, encounter_danger, encounter_score, heart_blessings};
use serde::Deserialize;
use std::{collections::HashSet, sync::OnceLock};

#[derive(Clone, Debug, Deserialize)]
pub struct OrdinarySpecies {
    pub id: String,
    pub family: String,
    pub first: u32,
    pub last: u32,
}
impl OrdinarySpecies {
    pub fn widened_bounds(&self) -> (u32, u32) {
        (self.first.saturating_sub(2).max(1), (self.last + 2).min(40))
    }
}
pub fn species() -> &'static [OrdinarySpecies] {
    static SPECIES: OnceLock<Vec<OrdinarySpecies>> = OnceLock::new();
    SPECIES.get_or_init(|| {
        serde_json::from_str(include_str!("catalog/ordinary-roster-v48.json"))
            .expect("ordinary roster")
    })
}

pub fn species_revision(revision: f64) -> &'static [OrdinarySpecies] {
    if revision < 33.0 { return species(); }
    static SPECIES: OnceLock<Vec<OrdinarySpecies>> = OnceLock::new();
    SPECIES.get_or_init(|| {
        let mut result = species().to_vec();
        for (id, first, last) in [("night-crawler",24,34),("skin-walker",26,36)] {
            result.push(OrdinarySpecies {id:id.into(), family:"wild".into(),first,last});
        }
        result
    })
}
pub(crate) fn bounds(s: &OrdinarySpecies, revision: f64) -> (u32,u32) {
    if revision >= 33.0 && crate::new_creatures::template(&s.id).is_some() { (s.first,s.last) }
    else if revision >= 32.0 { crate::flavor_placement::bounds(&s.id).expect("complete revision32 roster") }
    else { s.widened_bounds() }
}
fn build(depth: u32, water: bool, revision: f64) -> Vec<Candidate> {
    let c = catalog();
    let kinds: Vec<_> = species_revision(revision)
        .iter()
        .filter(|s| {
            let (first, last) = bounds(s, revision);
            (first..=last).contains(&depth) && (water || c.aquatic.get(&s.id) != Some(&true))
        })
        .collect();
    let sizes: &[usize] = if depth == 1 {
        &[1, 2, 3]
    } else {
        &[1, 2, 3, 5]
    };
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    let mut add = |members: Vec<String>| {
        if members.iter().filter(|k| *k == "banshee").count() > 1 {
            return;
        }
        if members.len() == 2 && c.pairs.get(&members[0]) != Some(&members[1]) {
            return;
        }
        let mut key = members.clone();
        key.sort();
        if !seen.insert(key) {
            return;
        }
        let score = encounter_score(depth as f64, &members).expect("authored danger");
        let danger = encounter_danger(depth as f64, score);
        if depth == 1 && !["low", "medium"].contains(&danger) {
            return;
        }
        let weight = members
            .iter()
            .map(|id| {
                let leader = c
                    .pairs
                    .iter()
                    .find(|(_, follower)| *follower == id)
                    .map_or(id, |(leader, _)| leader);
                let first = bounds(species_revision(revision).iter().find(|s| &s.id == leader)
                    .expect("roster leader"), revision).0;
                (0.2 + 0.2 * (depth - first) as f64).min(1.0)
            })
            .fold(1.0, f64::min);
        let blessings = heart_blessings(&members).expect("authored hearts");
        result.push(Candidate {
            kinds: members,
            danger: danger.into(),
            score,
            weight,
            blessings,
        });
    };
    // Same ordered thematic recipes as revision29; O(K²*M² + C*K*M) including
    // dedup keys and introduction lookup, K<=55, M<=5, cached once per depth/habitat.
    for kind in &kinds {
        if kind.id == "dryad" {
            for other in &kinds {
                if other.id == kind.id
                    || c.pairs.contains_key(&other.id)
                    || c.aquatic.get(&other.id) == Some(&true)
                {
                    continue;
                }
                for &n in sizes.iter().filter(|&&n| n > 1) {
                    let mut members = vec![kind.id.clone()];
                    members.extend(vec![other.id.clone(); n - 1]);
                    add(members);
                }
            }
        } else if let Some(follower) = c.pairs.get(&kind.id) {
            add(vec![kind.id.clone(), follower.clone()]);
        } else {
            for &n in sizes {
                add(vec![kind.id.clone(); n]);
            }
            for other in &kinds {
                if other.id == kind.id
                    || other.id == "dryad"
                    || c.pairs.contains_key(&other.id)
                    || other.family != kind.family
                {
                    continue;
                }
                for &n in sizes.iter().filter(|&&n| n > 1) {
                    let mut members = vec![kind.id.clone()];
                    members.extend(vec![other.id.clone(); n - 1]);
                    add(members);
                }
            }
        }
    }
    result
}

pub fn candidates(depth: u32, water: bool) -> Result<&'static [Candidate], String> {
    if !(1..=40).contains(&depth) {
        return Err("Invalid population depth".into());
    }
    static POOLS: [OnceLock<Vec<Candidate>>; 80] = [const { OnceLock::new() }; 80];
    Ok(POOLS[(depth as usize - 1) * 2 + water as usize].get_or_init(|| build(depth, water, 30.0)))
}

/// Explicit revision32 pools leave revision29 frozen and revision30/31 widened
/// recipes unchanged. Separate immutable caches prevent cross-replay pollution.
pub fn candidates_revision(depth: u32, water: bool, revision: f64) -> Result<&'static [Candidate],String> {
    if !(1..=40).contains(&depth) { return Err("Invalid population depth".into()); }
    match revision {
        29.0 => crate::population::budgeted_candidates(depth, water),
        30.0 | 31.0 => candidates(depth, water),
        32.0 => {
            static POOLS: [OnceLock<Vec<Candidate>>;80] = [const { OnceLock::new() };80];
            Ok(POOLS[(depth as usize-1)*2+water as usize].get_or_init(|| build(depth,water,32.0)))
        },
        33.0 | 34.0 | 35.0 | 36.0 | 37.0 | 38.0 | 39.0 | 40.0 | 41.0 | 42.0 | 43.0 | 44.0 | 45.0 | 46.0 | 50.0 | 51.0 | 52.0 => {
            static POOLS: [OnceLock<Vec<Candidate>>;80] = [const { OnceLock::new() };80];
            Ok(POOLS[(depth as usize-1)*2+water as usize].get_or_init(|| build(depth,water,33.0)))
        },
        _ => Err("Unsupported population revision".into()),
    }
}
