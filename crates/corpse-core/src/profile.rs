//! Pure local archive boundary; identities and timestamps are supplied by the UI.
use crate::{
    Command, Field, Run,
    storage::{Result, StorageError},
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeartTotal {
    pub creature: String,
    pub count: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    pub seed: String,
    pub mode: String,
    pub status: String,
    pub score: Option<f64>,
    pub observed_blessings: f64,
    pub hearts: Vec<HeartTotal>,
    pub hearts_complete: bool,
    pub depth: f64,
    pub tick: f64,
    pub death_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRecord {
    #[serde(flatten)]
    pub summary: RunSummary,
    pub id: String,
    pub saved_at: Option<String>,
    pub dev: bool,
    #[serde(flatten)]
    pub extra: indexmap::IndexMap<String, serde_json::Value>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub version: f64,
    pub run_archives: Vec<RunRecord>,
    pub high_scores: Vec<RunRecord>,
}
impl Default for Profile {
    fn default() -> Self {
        Self {
            version: 1.0,
            run_archives: vec![],
            high_scores: vec![],
        }
    }
}
fn error() -> StorageError {
    StorageError("Invalid descent profile. Existing records were not changed.".into())
}
fn count(n: f64) -> bool {
    n.is_finite() && n >= 0.0 && n <= 9007199254740991.0 && n.fract() == 0.0
}
fn valid_id(s: &str) -> bool {
    !s.is_empty() && s.encode_utf16().count() <= 200 && !s.chars().any(|c| (c as u32) < 32)
}
/// Detached read model, O(K log K) comparisons/O(K) memory. JS lexical order is UTF-16 order.
pub fn summarize_run(run: &Run) -> RunSummary {
    let stats = run.run_stats.as_ref();
    let death = stats.and_then(|s| s.death.as_ref());
    let mut hearts: Vec<_> = stats
        .into_iter()
        .flat_map(|s| s.hearts_eaten.iter())
        .map(|(creature, count)| HeartTotal {
            creature: creature.clone(),
            count: *count,
        })
        .collect();
    hearts.sort_by(|a, b| a.creature.encode_utf16().cmp(b.creature.encode_utf16()));
    RunSummary {
        seed: run.seed.clone(),
        mode: run.mode.clone(),
        status: run.status.clone(),
        score: stats
            .filter(|s| s.blessings_complete && run.playtest.is_missing())
            .map(|s| s.blessings_received),
        observed_blessings: stats.map_or(0.0, |s| s.blessings_received),
        hearts,
        hearts_complete: stats.is_some_and(|s| s.hearts_complete),
        depth: death.map_or(run.depth, |d| d.depth),
        tick: death.map_or(run.tick, |d| d.tick),
        death_reason: if run.status == "dead" {
            Some(
                death
                    .map_or("Cause not recorded", |d| d.reason.as_str())
                    .into(),
            )
        } else {
            None
        },
        ending: crate::finale::ending(run).map(str::to_owned),
    }
}
/// UI-supplied identity never consumes serialized gameplay randomness.
pub fn ensure_run_id(run: &mut Run, create_id: impl FnOnce() -> String) -> Result<String> {
    if let Some(id) = run.run_id.as_ref() {
        if !valid_id(id) {
            return Err(error());
        }
        return Ok(id.clone());
    }
    if !run.run_id.is_missing() {
        return Err(error());
    }
    let id = create_id();
    if !valid_id(&id) {
        return Err(error());
    }
    run.run_id = Field::Value(id.clone());
    Ok(id)
}
/// Stable ties preserve archive order; unknown legacy scores never rank. O(R log R).
pub fn ranked_run_records(records: &[RunRecord]) -> Vec<RunRecord> {
    let mut ranked: Vec<_> = records
        .iter()
        .filter(|r| r.summary.score.is_some())
        .cloned()
        .collect();
    ranked.sort_by(|a, b| {
        b.summary
            .score
            .unwrap()
            .total_cmp(&a.summary.score.unwrap())
    });
    ranked
}
// Current source exports ISO UTC timestamps. Other Date.parse legacy syntaxes fail clearly.
fn valid_timestamp(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 24
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || b[19] != b'.'
        || b[23] != b'Z'
    {
        return false;
    }
    let number = |a: usize, z: usize| s.get(a..z).and_then(|v| v.parse::<u32>().ok());
    matches!(number(0, 4), Some(0..=9999))
        && matches!(number(5, 7), Some(1..=12))
        && matches!(number(8, 10), Some(1..=31))
        && matches!(number(11, 13), Some(0..=23))
        && matches!(number(14, 16), Some(0..=59))
        && matches!(number(17, 19), Some(0..=59))
        && matches!(number(20, 23), Some(0..=999))
}
fn valid_record(r: &RunRecord) -> bool {
    let s = &r.summary;
    let mut hearts = std::collections::HashSet::new();
    valid_id(&r.id)
        && !r.dev
        && ["Unblessed", "Blessed", "Avatar"].contains(&s.mode.as_str())
        && ["dead", "won"].contains(&s.status.as_str())
        && count(s.observed_blessings)
        && s.score
            .is_none_or(|n| count(n) && n == s.observed_blessings)
        && count(s.depth)
        && (1.0..=40.0).contains(&s.depth)
        && count(s.tick)
        && r.saved_at.as_ref().is_none_or(|s| valid_timestamp(s))
        && s.death_reason
            .as_ref()
            .is_none_or(|s| !s.is_empty() && s.encode_utf16().count() <= 512)
        && s.ending
            .as_ref()
            .is_none_or(|s| !s.is_empty() && s.encode_utf16().count() <= 512)
        && s.hearts.iter().all(|h| {
            !h.creature.is_empty()
                && h.creature.encode_utf16().count() <= 200
                && count(h.count)
                && h.count > 0.0
                && hearts.insert(&h.creature)
        })
}
pub fn parse_profile(raw: Option<&str>) -> Result<Profile> {
    let Some(raw) = raw else {
        return Ok(Profile::default());
    };
    let v: serde_json::Value = serde_json::from_str(raw)?;
    let version = v["version"].as_f64().ok_or_else(error)?;
    if !v["runArchives"].as_array().is_some_and(|a| {
        a.iter().all(|r| {
            ["savedAt", "score", "deathReason"]
                .iter()
                .all(|k| r.get(*k).is_some())
        })
    }) {
        return Err(error());
    }
    let archives: Vec<RunRecord> = serde_json::from_value(v["runArchives"].clone())?;
    let mut ids = std::collections::HashSet::new();
    if version != 1.0
        || !archives
            .iter()
            .all(|r| valid_record(r) && ids.insert(&r.id))
    {
        return Err(error());
    }
    Ok(Profile {
        version,
        high_scores: ranked_run_records(&archives),
        run_archives: archives,
    })
}
/// Pure archive update; caller must additionally exclude controlled fixtures.
/// O(P + R log R + K log K), independent of world-floor count.
pub fn archive_run_record(
    profile: &Profile,
    run: &Run,
    replay: bool,
    saved_at: Option<&str>,
) -> Result<(Profile, bool)> {
    if !run.playtest.is_missing()
        || replay
        || run.dev
        || run.replay.iter().any(|c| matches!(c, Command::Dev { .. }))
        || run.status == "playing"
    {
        return Ok((profile.clone(), false));
    }
    let id = run.run_id.as_ref().filter(|s| valid_id(s)).ok_or_else(|| {
        StorageError("Assign a run identity before archiving this descent.".into())
    })?;
    if profile.run_archives.iter().any(|r| r.id == *id) {
        return Ok((profile.clone(), false));
    }
    let record = RunRecord {
        summary: summarize_run(run),
        id: id.clone(),
        saved_at: saved_at.map(str::to_owned),
        dev: false,
        extra: indexmap::IndexMap::new(),
    };
    if !valid_record(&record) {
        return Err(error());
    }
    let mut out = profile.clone();
    out.run_archives.push(record);
    out.high_scores = ranked_run_records(&out.run_archives);
    Ok((out, true))
}
