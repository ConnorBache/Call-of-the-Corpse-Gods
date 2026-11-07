//! Deterministic seed-based replay and explicit canonical-fixture replay.
use crate::{Command, Field, Run, storage::StorageError, temporal::Engine};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplayRecord {
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub playtest: Field<crate::playtest::Profile>,
    pub version: f64,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub revert_policy: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub snapshot_from: Field<usize>,
    pub seed: String,
    pub mode: String,
    #[serde(default)]
    pub dev: bool,
    pub commands: Vec<Command>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub simulation_revision: Field<f64>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub depth: Field<f64>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub tick: Field<f64>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub status: Field<String>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub saved_at: Field<String>,
}
impl ReplayRecord {
    /// O(commands + metadata bytes), exports accepted commands and explicit policy.
    /// Unchecked compatibility helper: unknown provenance emits an invalid policy;
    /// user-facing exporters must call try_from_run instead.
    pub fn from_run(run: &Run) -> Self {
        Self {
            playtest: run.playtest.clone(),
            version: 2.0,
            revert_policy: Field::Value(match run.replay_snapshot_from {Field::Missing=>"legacy",Field::Value(0)=>"snapshot",Field::Value(_)=>"legacy",Field::Null=>"unknown"}.into()),
            snapshot_from: match run.replay_snapshot_from {Field::Value(n) if n>0=>Field::Value(n),_=>Field::Missing},
            seed: run.seed.clone(),
            mode: run.mode.clone(),
            dev: run.dev,
            commands: run.replay.clone(),
            simulation_revision: run.simulation_revision.clone(),
            depth: Field::Value(run.depth),
            tick: Field::Value(run.tick),
            status: Field::Value(run.status.clone()),
            saved_at: Field::Missing,
        }
    }
    /// O(commands + metadata bytes), rejects ambiguous old experimental ledgers.
    pub fn try_from_run(run:&Run)->Result<Self,StorageError> {
        if matches!(run.replay_snapshot_from,Field::Null) {return Err(StorageError("Seed replay export unavailable: this older experimental save has unknown Revert-policy provenance. The saved session remains usable.".into()));}
        let record=Self::from_run(run);record.validate_policy()?;Ok(record)
    }
    /// O(1) policy/schema checks. Boundary is a command INDEX, not game tick.
    fn validate_policy(&self)->Result<(),StorageError> {
        let valid=if self.version==1.0 {self.revert_policy.is_missing()&&self.snapshot_from.is_missing()}
        else if self.version==2.0 {match self.revert_policy.as_ref().map(String::as_str) {
            Some("snapshot")=>self.snapshot_from.is_missing(),
            Some("legacy")=>self.snapshot_from.is_missing()||matches!(self.snapshot_from,Field::Value(n) if n<=self.commands.len()),
            _=>false,
        }}else{false};
        if valid {Ok(())}else{Err(StorageError("Unsupported replay version/Revert policy or invalid transition boundary".into()))}
    }
    /// O(1), configure only at start or the single legacy-to-snapshot boundary.
    fn apply_policy(&self,engine:&mut Engine,cursor:usize) {
        if cursor==0 {engine.set_legacy_revert(self.version==1.0||self.revert_policy.as_ref().map(String::as_str)==Some("legacy"));}
        if self.version==2.0&&self.snapshot_from.as_ref()==Some(&cursor) {engine.set_legacy_revert(false);}
    }
}
pub struct ReplaySession {
    pub record: ReplayRecord,
    pub engine: Engine,
    pub cursor: usize,
    initial: Run,
    seed_based: bool,
}
impl ReplaySession {
    /// O(initial world generation/import), plus O(1) policy admission.
    pub fn create(record: ReplayRecord, require_revision: bool) -> Result<Self, StorageError> {
        if (require_revision || !record.simulation_revision.is_missing())
            && !matches!(record.simulation_revision.as_ref(), Some(29.0 | 30.0 | 31.0 | 32.0 | 33.0 | 34.0 | 35.0 | 36.0 | 37.0 | 38.0 | 39.0 | 40.0 | 41.0 | 42.0 | 43.0 | 44.0 | 45.0 | 46.0 | 50.0 | 51.0))
        {
            return Err(StorageError("This replay requires its original game build: simulation revision is missing or incompatible".into()));
        }
        record.validate_policy()?;
        if !["Unblessed", "Blessed", "Avatar"].contains(&record.mode.as_str())
        {
            return Err(StorageError("Unsupported replay".into()));
        }
        let initial = create_initial(&record)
        .map_err(StorageError)?;
        let mut engine=Engine::new(initial.clone())?;
        record.apply_policy(&mut engine,0);
        Ok(Self {
            engine,
            initial,
            record,
            cursor: 0,
            seed_based: true,
        })
    }

    /// O(initial world clone/validation/history import), O(1) policy admission.
    pub fn from_initial(initial: Run, record: ReplayRecord) -> Result<Self, StorageError> {
        record.validate_policy()?;
        if record.simulation_revision != initial.simulation_revision
            || !matches!(record.simulation_revision.as_ref(), Some(29.0 | 30.0 | 31.0 | 32.0 | 33.0 | 34.0 | 35.0 | 36.0 | 37.0 | 38.0 | 39.0 | 40.0 | 41.0 | 42.0 | 43.0 | 44.0 | 45.0 | 46.0 | 50.0 | 51.0))
            || record.seed != initial.seed
            || record.mode != initial.mode
            || record.dev != initial.dev
            || record.playtest != initial.playtest
        {
            return Err(StorageError(
                "Replay requires matching initial fixture and simulation revision 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 50 or 51".into(),
            ));
        }
        let mut engine=Engine::new(initial.clone())?;
        record.apply_policy(&mut engine,0);
        Ok(Self {
            engine,
            initial,
            record,
            cursor: 0,
            seed_based: false,
        })
    }
    pub fn step(&mut self) -> Result<bool, crate::engine::EngineError> {
        Ok(self.step_with_outcome()?.is_some())
    }
    /// One simulation command plus O(1) policy/cursor work; no ledger scan.
    /// Presentation events remain detached from authoritative replay state.
    pub fn step_with_outcome(
        &mut self,
    ) -> Result<Option<crate::engine::DispatchOutcome>, crate::engine::EngineError> {
        if self.finished() {
            return Ok(None);
        }
        self.record.apply_policy(&mut self.engine,self.cursor);
        let outcome = self.engine.dispatch(&self.record.commands[self.cursor])?;
        self.cursor += 1;
        // Boundary at end must persist even before another command is played.
        if self.record.snapshot_from.as_ref()==Some(&self.cursor) {self.record.apply_policy(&mut self.engine,self.cursor);}
        Ok(Some(outcome))
    }
    pub fn finished(&self) -> bool {
        self.cursor >= self.record.commands.len()
    }
    /// O(initial world generation/import); resets cursor and its exact policy.
    pub fn restart(&mut self) -> Result<(), StorageError> {
        if self.seed_based {
            self.initial = create_initial(&self.record)
            .map_err(StorageError)?;
        }
        self.record.validate_policy()?;
        self.engine = Engine::new(self.initial.clone())?;
        self.record.apply_policy(&mut self.engine,0);
        self.cursor = 0;
        Ok(())
    }
}

fn create_initial(record: &ReplayRecord) -> Result<Run, String> {
    if let Some(p) = record.playtest.as_ref() {
        if p.version == 2 {return Err("DEBUG loadout edits are not command-ledger replayable; use the saved session instead.".into());}
        let run = crate::playtest::create_run_revision(&record.seed, &record.mode, record.dev, p.start_floor, &p.profile, record.simulation_revision.value_or(29.))?;
        if run.playtest != record.playtest { return Err("Playtest replay starting profile does not match this build.".into()); }
        Ok(run)
    } else if record.playtest.is_missing() {
        crate::startup::create_run_revision(&record.seed, &record.mode, record.dev, record.simulation_revision.value_or(29.))
    } else { Err("Invalid null playtest replay profile.".into()) }
}

include!("replay_policy_tests.rs");
