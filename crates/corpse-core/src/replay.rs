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
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub firebolt_travel_from: Field<usize>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub amulet_rules_from: Field<usize>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub potion_rules_from: Field<usize>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub ring_rules_from: Field<usize>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub gas_rules_from: Field<usize>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub commandment_rules_from: Field<usize>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub screamer_rules_from: Field<usize>,
    #[serde(rename = "ragelightRulesFrom", default, skip_serializing_if = "Field::is_missing")]
    pub ragelight_rules_from: Field<usize>,
    #[serde(rename = "balanceRulesFrom", default, skip_serializing_if = "Field::is_missing")]
    pub balance_rules_from: Field<usize>,
    #[serde(rename = "collapseRulesFrom", default, skip_serializing_if = "Field::is_missing")]
    pub collapse_rules_from: Field<usize>,
    #[serde(rename = "bloodMistFireRulesFrom", default, skip_serializing_if = "Field::is_missing")]
    pub blood_mist_fire_rules_from: Field<usize>,
    #[serde(rename = "halfHereLockFrom", default, skip_serializing_if = "Field::is_missing")]
    pub half_here_lock_from: Field<usize>,
    #[serde(rename = "halfHereEnergyFrom", default, skip_serializing_if = "Field::is_missing")]
    pub half_here_energy_from: Field<usize>,
    #[serde(rename = "screamerRangeFrom", default, skip_serializing_if = "Field::is_missing")]
    pub screamer_range_from: Field<usize>,
    #[serde(rename = "deathGasRulesFrom", default, skip_serializing_if = "Field::is_missing")]
    pub death_gas_rules_from: Field<usize>,
    #[serde(rename = "minotaurBumpFrom", default, skip_serializing_if = "Field::is_missing")]
    pub minotaur_bump_from: Field<usize>,
    #[serde(rename = "borgeThrowFrom", default, skip_serializing_if = "Field::is_missing")]
    pub borge_throw_from: Field<usize>,
    #[serde(rename = "bansheeShriekFrom", default, skip_serializing_if = "Field::is_missing")]
    pub banshee_shriek_from: Field<usize>,
    #[serde(rename = "localFloorTicksFrom", default, skip_serializing_if = "Field::is_missing")]
    pub local_floor_ticks_from: Field<usize>,
    #[serde(rename = "executeRulesFrom", default, skip_serializing_if = "Field::is_missing")]
    pub execute_rules_from: Field<usize>,
    #[serde(rename = "beckoningRulesFrom", default, skip_serializing_if = "Field::is_missing")]
    pub beckoning_rules_from: Field<usize>,
    #[serde(rename = "ichorFireRulesFrom", default, skip_serializing_if = "Field::is_missing")]
    pub ichor_fire_rules_from: Field<usize>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub avatar_starter_policy: Field<u8>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub ember_rules_from: Field<usize>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub steam_rules_from: Field<usize>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub future_floor_rules_from: Field<usize>,
    pub seed: String,
    pub mode: String,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub fast_mode: Field<bool>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub compressed_depths: Field<bool>,
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
            amulet_rules_from: match run.amulet_rules_from { Field::Value(n) => n.into(), _ => Field::Missing },
            potion_rules_from: match run.potion_rules_from { Field::Value(n) => n.into(), _ => Field::Missing },
            ring_rules_from: match run.ring_rules_from { Field::Value(n) => n.into(), _ => Field::Missing },
            gas_rules_from: match run.gas_rules_from { Field::Value(n) => n.into(), _ => Field::Missing },
            commandment_rules_from: match run.commandment_rules_from { Field::Value(n) => n.into(), _ => Field::Missing },
            screamer_rules_from: match run.screamer_rules_from { Field::Value(n) => n.into(), _ => Field::Missing },
            ember_rules_from: match run.ember_rules_from { Field::Value(n) => n.into(), _ => Field::Missing },
            ragelight_rules_from: match run.ragelight_rules_from { Field::Value(n) => n.into(), _ => Field::Missing },
            balance_rules_from: match run.balance_rules_from { Field::Value(n) => n.into(), _ => Field::Missing },
            collapse_rules_from: match run.collapse_rules_from { Field::Value(n) => n.into(), _ => Field::Missing },
            blood_mist_fire_rules_from: match run.blood_mist_fire_rules_from { Field::Value(n) => n.into(), _ => Field::Missing },
            half_here_lock_from: match run.half_here_lock_from { Field::Value(n) => n.into(), _ => Field::Missing },
            half_here_energy_from: match run.half_here_energy_from { Field::Value(n) => n.into(), _ => Field::Missing },
            screamer_range_from: match run.screamer_range_from { Field::Value(n) => n.into(), _ => Field::Missing },
            death_gas_rules_from: match run.death_gas_rules_from { Field::Value(n) => n.into(), _ => Field::Missing },
            minotaur_bump_from: match run.minotaur_bump_from { Field::Value(n) => n.into(), _ => Field::Missing },
            borge_throw_from: match run.borge_throw_from { Field::Value(n) => n.into(), _ => Field::Missing },
            banshee_shriek_from: match run.banshee_shriek_from { Field::Value(n) => n.into(), _ => Field::Missing },
            local_floor_ticks_from: match run.local_floor_ticks_from { Field::Value(n) => n.into(), _ => Field::Missing },
            steam_rules_from: match run.steam_rules_from { Field::Value(n) => n.into(), _ => Field::Missing },
            execute_rules_from: match run.execute_rules_from { Field::Value(n) => n.into(), _ => Field::Missing },
            beckoning_rules_from: match run.beckoning_rules_from { Field::Value(n) => n.into(), _ => Field::Missing },
            ichor_fire_rules_from: match run.ichor_fire_rules_from { Field::Value(n) => n.into(), _ => Field::Missing },
            future_floor_rules_from: match run.future_floor_rules_from { Field::Value(n) => n.into(), _ => Field::Missing },
            firebolt_travel_from: match run.firebolt_travel_from { Field::Value(n) => n.into(), _ => Field::Missing },
            avatar_starter_policy: run.avatar_starter_policy.clone(),
            seed: run.seed.clone(),
            mode: run.mode.clone(),
            fast_mode: run.fast_mode.clone(),
            compressed_depths: run.compressed_depths.clone(),
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
        crate::storage::validate_avatar_starter_policy(&self.avatar_starter_policy, self.simulation_revision.value_or(29.), &self.mode).map_err(StorageError)?;
        if !self.playtest.is_missing() && !self.avatar_starter_policy.is_missing() { return Err(StorageError("Unsupported synthetic playtest starter policy".into())); }
        if matches!(self.compressed_depths, Field::Null) || self.compressed_depths.value_or(false) && !self.fast_mode.value_or(false) { return Err(StorageError("Invalid dungeon route".into())); }
        if matches!(self.fast_mode, Field::Null) || self.fast_mode.value_or(false) && (self.simulation_revision.as_ref() != Some(&52.) || self.mode != "Avatar" || self.dev || !self.playtest.is_missing()) { return Err(StorageError("Unsupported Fast mode replay configuration".into())); }
        if !self.firebolt_travel_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 52.
            && matches!(self.firebolt_travel_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Invalid Firebolt travel replay boundary".into()));
        }
        if !self.amulet_rules_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 52.
            && matches!(self.amulet_rules_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Invalid amulet rules replay boundary".into()));
        }
        if !self.potion_rules_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 52.
            && matches!(self.potion_rules_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Invalid potion rules replay boundary".into()));
        }
        if !self.ring_rules_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 52.
            && matches!(self.ring_rules_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Invalid ring rules replay boundary".into()));
        }
        if !self.gas_rules_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 52.
            && matches!(self.gas_rules_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Invalid gas rules replay boundary".into()));
        }
        if !self.commandment_rules_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 50.
            && matches!(self.commandment_rules_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Invalid Commandment rules replay boundary".into()));
        }
        if !self.ember_rules_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 50.
            && matches!(self.ember_rules_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Invalid ember rules command boundary".into()));
        }
        if !self.steam_rules_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 50.
            && matches!(self.steam_rules_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Invalid steam rules command boundary".into()));
        }
        if !self.screamer_rules_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 50.
            && matches!(self.screamer_rules_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Invalid Screamer rules replay boundary".into()));
        }
        if !self.ragelight_rules_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 52.
            && matches!(self.ragelight_rules_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Invalid Ragelight rules replay boundary".into()));
        }
        if !self.balance_rules_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 52.
            && matches!(self.balance_rules_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Unsupported divine balance replay boundary".into()));
        }
        if !self.collapse_rules_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 52.
            && matches!(self.collapse_rules_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Unsupported collapse rules replay boundary".into()));
        }
        if !self.blood_mist_fire_rules_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 52.
            && matches!(self.blood_mist_fire_rules_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Unsupported Blood Mist fire rules replay boundary".into()));
        }
        if !self.half_here_lock_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 52.
            && matches!(self.half_here_lock_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Unsupported Half Here lock replay boundary".into()));
        }
        if !self.half_here_energy_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 52.
            && matches!(self.half_here_energy_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Unsupported Half Here energy replay boundary".into()));
        }
        if !self.screamer_range_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 52.
            && matches!(self.screamer_range_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Unsupported Screamer range replay boundary".into()));
        }
        if !self.minotaur_bump_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 52.
            && matches!(self.minotaur_bump_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Invalid Minotaur bump replay boundary".into()));
        }
        if !self.borge_throw_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 52.
            && matches!(self.borge_throw_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Invalid Borge throw replay boundary".into()));
        }
        if !self.banshee_shriek_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 52.
            && matches!(self.banshee_shriek_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Invalid Banshee shriek replay boundary".into()));
        }
        if !self.local_floor_ticks_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 52.
            && matches!(self.local_floor_ticks_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Invalid Local floor ticks replay boundary".into()));
        }
        if !self.death_gas_rules_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 52.
            && matches!(self.death_gas_rules_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Unsupported Death Gas rules replay boundary".into()));
        }
        if !self.execute_rules_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 52.
            && matches!(self.execute_rules_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Invalid Execute rules replay boundary".into()));
        }
        if !self.beckoning_rules_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 52.
            && matches!(self.beckoning_rules_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Invalid Beckoning rules replay boundary".into()));
        }
        if !self.ichor_fire_rules_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 52.
            && matches!(self.ichor_fire_rules_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Invalid Ichor fire rules replay boundary".into()));
        }
        if !self.future_floor_rules_from.is_missing() && !(self.simulation_revision.value_or(29.) >= 52.
            && matches!(self.future_floor_rules_from, Field::Value(n) if n <= self.commands.len())) {
            return Err(StorageError("Invalid See the Future rules replay boundary".into()));
        }
        let valid=if self.version==1.0 {self.revert_policy.is_missing()&&self.snapshot_from.is_missing()}
        else if self.version==2.0 {match self.revert_policy.as_ref().map(String::as_str) {
            Some("snapshot")=>self.snapshot_from.is_missing(),
            Some("legacy")=>self.snapshot_from.is_missing()||matches!(self.snapshot_from,Field::Value(n) if n<=self.commands.len()),
            _=>false,
        }}else{false};
        if valid {Ok(())}else{Err(StorageError("Unsupported replay version/Revert policy or invalid transition boundary".into()))}
    }
    /// O(1) ordinary policy checks; O(world state) once when adopting a floor checkpoint.
    fn apply_policy(&self,engine:&mut Engine,cursor:usize) -> Result<(), String> {
        crate::amulet_rules::apply_replay_policy(&mut engine.run, &self.amulet_rules_from, cursor);
        crate::potion_rules::apply_replay_policy(&mut engine.run, &self.potion_rules_from, cursor);
        crate::ring_rules::apply_replay_policy(&mut engine.run, &self.ring_rules_from, cursor);
        crate::gas_rules::apply_replay_policy(&mut engine.run, &self.gas_rules_from, cursor);
        crate::commandment_rules::apply_replay_policy(&mut engine.run, &self.commandment_rules_from, cursor);
        crate::screamer_rules::apply_replay_policy(&mut engine.run, &self.screamer_rules_from, cursor);
        crate::ember_rules::apply_replay_policy(&mut engine.run, &self.ember_rules_from, cursor);
        crate::ragelight_rules::apply_replay_policy(&mut engine.run, &self.ragelight_rules_from, cursor);
        crate::balance_rules::apply_replay_policy(&mut engine.run, &self.balance_rules_from, cursor);
        crate::finale::apply_replay_policy(&mut engine.run, &self.collapse_rules_from, cursor);
        crate::blood_mist::apply_replay_policy(&mut engine.run, &self.blood_mist_fire_rules_from, cursor);
        crate::half_here::apply_replay_policy(&mut engine.run, &self.half_here_lock_from, cursor);
        crate::half_here::apply_energy_replay_policy(&mut engine.run, &self.half_here_energy_from, cursor);
        crate::screamer_rules::apply_range_replay_policy(&mut engine.run, &self.screamer_range_from, cursor);
        crate::death_gas_rules::apply_replay_policy(&mut engine.run, &self.death_gas_rules_from, cursor);
        crate::minotaur_rules::apply_replay_policy(&mut engine.run, &self.minotaur_bump_from, cursor);
        crate::borge::apply_replay_policy(&mut engine.run, &self.borge_throw_from, cursor);
        crate::banshee_rules::apply_replay_policy(&mut engine.run, &self.banshee_shriek_from, cursor);
        crate::local_floor_rules::apply_replay_policy(&mut engine.run, &self.local_floor_ticks_from, cursor);
        crate::steam_rules::apply_replay_policy(&mut engine.run, &self.steam_rules_from, cursor);
        crate::execute_rules::apply_replay_policy(&mut engine.run, &self.execute_rules_from, cursor);
        crate::beckoning_rules::apply_replay_policy(&mut engine.run, &self.beckoning_rules_from, cursor);
        crate::ichor_fire_rules::apply_replay_policy(&mut engine.run, &self.ichor_fire_rules_from, cursor);
        crate::future_floor::apply_replay_policy(&mut engine.run, &self.future_floor_rules_from, cursor);
        crate::firebolt_travel::apply_replay_policy(&mut engine.run, &self.firebolt_travel_from, cursor);
        if cursor==0 {engine.set_legacy_revert(self.version==1.0||self.revert_policy.as_ref().map(String::as_str)==Some("legacy"));}
        if self.version==2.0&&self.snapshot_from.as_ref()==Some(&cursor) {engine.set_legacy_revert(false);}
        crate::future_floor::ensure_entry(&mut engine.run)?;
        Ok(())
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
            && !matches!(record.simulation_revision.as_ref(), Some(29.0 | 30.0 | 31.0 | 32.0 | 33.0 | 34.0 | 35.0 | 36.0 | 37.0 | 38.0 | 39.0 | 40.0 | 41.0 | 42.0 | 43.0 | 44.0 | 45.0 | 46.0 | 50.0 | 51.0 | 52.0))
        {
            return Err(StorageError("This replay requires its original game build: simulation revision is missing or incompatible".into()));
        }
        record.validate_policy()?;
        if !["Unblessed", "Blessed", "Avatar"].contains(&record.mode.as_str())
        {
            return Err(StorageError("Unsupported replay".into()));
        }
        let mut initial = create_initial(&record)
        .map_err(StorageError)?;
        crate::firebolt_travel::apply_replay_policy(&mut initial, &record.firebolt_travel_from, 0);
        crate::amulet_rules::apply_replay_policy(&mut initial, &record.amulet_rules_from, 0);
        crate::potion_rules::apply_replay_policy(&mut initial, &record.potion_rules_from, 0);
        crate::ring_rules::apply_replay_policy(&mut initial, &record.ring_rules_from, 0);
        crate::gas_rules::apply_replay_policy(&mut initial, &record.gas_rules_from, 0);
        crate::commandment_rules::apply_replay_policy(&mut initial, &record.commandment_rules_from, 0);
        crate::screamer_rules::apply_replay_policy(&mut initial, &record.screamer_rules_from, 0);
        crate::ember_rules::apply_replay_policy(&mut initial, &record.ember_rules_from, 0);
        crate::ragelight_rules::apply_replay_policy(&mut initial, &record.ragelight_rules_from, 0);
        crate::balance_rules::apply_replay_policy(&mut initial, &record.balance_rules_from, 0);
        crate::finale::apply_replay_policy(&mut initial, &record.collapse_rules_from, 0);
        crate::blood_mist::apply_replay_policy(&mut initial, &record.blood_mist_fire_rules_from, 0);
        crate::half_here::apply_replay_policy(&mut initial, &record.half_here_lock_from, 0);
        crate::half_here::apply_energy_replay_policy(&mut initial, &record.half_here_energy_from, 0);
        crate::screamer_rules::apply_range_replay_policy(&mut initial, &record.screamer_range_from, 0);
        crate::death_gas_rules::apply_replay_policy(&mut initial, &record.death_gas_rules_from, 0);
        crate::minotaur_rules::apply_replay_policy(&mut initial, &record.minotaur_bump_from, 0);
        crate::borge::apply_replay_policy(&mut initial, &record.borge_throw_from, 0);
        crate::banshee_rules::apply_replay_policy(&mut initial, &record.banshee_shriek_from, 0);
        crate::local_floor_rules::apply_replay_policy(&mut initial, &record.local_floor_ticks_from, 0);
        crate::steam_rules::apply_replay_policy(&mut initial, &record.steam_rules_from, 0);
        crate::execute_rules::apply_replay_policy(&mut initial, &record.execute_rules_from, 0);
        crate::beckoning_rules::apply_replay_policy(&mut initial, &record.beckoning_rules_from, 0);
        crate::ichor_fire_rules::apply_replay_policy(&mut initial, &record.ichor_fire_rules_from, 0);
        crate::future_floor::apply_replay_policy(&mut initial, &record.future_floor_rules_from, 0);
        let mut engine=Engine::new_for_replay(initial.clone())?;
        record.apply_policy(&mut engine,0).map_err(StorageError)?;
        Ok(Self {
            engine,
            initial,
            record,
            cursor: 0,
            seed_based: true,
        })
    }

    /// O(initial world clone/validation/history import), O(1) policy admission.
    pub fn from_initial(mut initial: Run, record: ReplayRecord) -> Result<Self, StorageError> {
        record.validate_policy()?;
        if record.simulation_revision != initial.simulation_revision
            || !matches!(record.simulation_revision.as_ref(), Some(29.0 | 30.0 | 31.0 | 32.0 | 33.0 | 34.0 | 35.0 | 36.0 | 37.0 | 38.0 | 39.0 | 40.0 | 41.0 | 42.0 | 43.0 | 44.0 | 45.0 | 46.0 | 50.0 | 51.0 | 52.0))
            || record.seed != initial.seed
            || record.mode != initial.mode
            || record.fast_mode.value_or(false) != crate::fast_mode::enabled(&initial)
            || record.compressed_depths.value_or(false) != crate::fast_mode::compressed(&initial)
            || record.dev != initial.dev
            || record.playtest != initial.playtest
            || record.avatar_starter_policy != initial.avatar_starter_policy
        {
            return Err(StorageError(
                "Replay requires matching initial fixture and simulation revision 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 50, 51 or 52".into(),
            ));
        }
        crate::firebolt_travel::apply_replay_policy(&mut initial, &record.firebolt_travel_from, 0);
        crate::amulet_rules::apply_replay_policy(&mut initial, &record.amulet_rules_from, 0);
        crate::potion_rules::apply_replay_policy(&mut initial, &record.potion_rules_from, 0);
        crate::ring_rules::apply_replay_policy(&mut initial, &record.ring_rules_from, 0);
        crate::gas_rules::apply_replay_policy(&mut initial, &record.gas_rules_from, 0);
        crate::commandment_rules::apply_replay_policy(&mut initial, &record.commandment_rules_from, 0);
        crate::screamer_rules::apply_replay_policy(&mut initial, &record.screamer_rules_from, 0);
        crate::ember_rules::apply_replay_policy(&mut initial, &record.ember_rules_from, 0);
        crate::ragelight_rules::apply_replay_policy(&mut initial, &record.ragelight_rules_from, 0);
        crate::balance_rules::apply_replay_policy(&mut initial, &record.balance_rules_from, 0);
        crate::finale::apply_replay_policy(&mut initial, &record.collapse_rules_from, 0);
        crate::blood_mist::apply_replay_policy(&mut initial, &record.blood_mist_fire_rules_from, 0);
        crate::half_here::apply_replay_policy(&mut initial, &record.half_here_lock_from, 0);
        crate::half_here::apply_energy_replay_policy(&mut initial, &record.half_here_energy_from, 0);
        crate::screamer_rules::apply_range_replay_policy(&mut initial, &record.screamer_range_from, 0);
        crate::death_gas_rules::apply_replay_policy(&mut initial, &record.death_gas_rules_from, 0);
        crate::minotaur_rules::apply_replay_policy(&mut initial, &record.minotaur_bump_from, 0);
        crate::borge::apply_replay_policy(&mut initial, &record.borge_throw_from, 0);
        crate::banshee_rules::apply_replay_policy(&mut initial, &record.banshee_shriek_from, 0);
        crate::local_floor_rules::apply_replay_policy(&mut initial, &record.local_floor_ticks_from, 0);
        crate::steam_rules::apply_replay_policy(&mut initial, &record.steam_rules_from, 0);
        crate::execute_rules::apply_replay_policy(&mut initial, &record.execute_rules_from, 0);
        crate::beckoning_rules::apply_replay_policy(&mut initial, &record.beckoning_rules_from, 0);
        crate::ichor_fire_rules::apply_replay_policy(&mut initial, &record.ichor_fire_rules_from, 0);
        crate::future_floor::apply_replay_policy(&mut initial, &record.future_floor_rules_from, 0);
        let mut engine=Engine::new_for_replay(initial.clone())?;
        record.apply_policy(&mut engine,0).map_err(StorageError)?;
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
        self.record.apply_policy(&mut self.engine,self.cursor).map_err(crate::engine::EngineError)?;
        let outcome = self.engine.dispatch(&self.record.commands[self.cursor])?;
        self.cursor += 1;
        // Boundary at end must persist even before another command is played.
        if self.record.local_floor_ticks_from.as_ref()==Some(&self.cursor) || self.record.banshee_shriek_from.as_ref()==Some(&self.cursor) || self.record.borge_throw_from.as_ref()==Some(&self.cursor) || self.record.minotaur_bump_from.as_ref()==Some(&self.cursor) || self.record.death_gas_rules_from.as_ref()==Some(&self.cursor) || self.record.screamer_range_from.as_ref()==Some(&self.cursor) || self.record.half_here_energy_from.as_ref()==Some(&self.cursor) || self.record.half_here_lock_from.as_ref()==Some(&self.cursor) || self.record.blood_mist_fire_rules_from.as_ref()==Some(&self.cursor) || self.record.future_floor_rules_from.as_ref()==Some(&self.cursor) || self.record.gas_rules_from.as_ref()==Some(&self.cursor) || self.record.commandment_rules_from.as_ref()==Some(&self.cursor) || self.record.screamer_rules_from.as_ref()==Some(&self.cursor) || self.record.ragelight_rules_from.as_ref()==Some(&self.cursor) || self.record.collapse_rules_from.as_ref()==Some(&self.cursor) || self.record.balance_rules_from.as_ref()==Some(&self.cursor) || self.record.beckoning_rules_from.as_ref()==Some(&self.cursor) || self.record.ichor_fire_rules_from.as_ref()==Some(&self.cursor) || self.record.execute_rules_from.as_ref()==Some(&self.cursor) || self.record.steam_rules_from.as_ref()==Some(&self.cursor) || self.record.ember_rules_from.as_ref()==Some(&self.cursor) || self.record.ring_rules_from.as_ref()==Some(&self.cursor) || self.record.potion_rules_from.as_ref()==Some(&self.cursor) || self.record.amulet_rules_from.as_ref()==Some(&self.cursor) || self.record.snapshot_from.as_ref()==Some(&self.cursor) || self.record.firebolt_travel_from.as_ref()==Some(&self.cursor) {self.record.apply_policy(&mut self.engine,self.cursor).map_err(crate::engine::EngineError)?;}
        Ok(Some(outcome))
    }
    pub fn finished(&self) -> bool {
        self.cursor >= self.record.commands.len()
    }
    /// O(initial world generation/import); resets cursor and its exact policy.
    pub fn restart(&mut self) -> Result<(), StorageError> {
        self.record.validate_policy()?;
        if self.seed_based {
            self.initial = create_initial(&self.record)
            .map_err(StorageError)?;
        }
        if self.record.avatar_starter_policy != self.initial.avatar_starter_policy { return Err(StorageError("Replay starter policy does not match initial state".into())); }
        let mut initial = self.initial.clone();
        crate::firebolt_travel::apply_replay_policy(&mut initial, &self.record.firebolt_travel_from, 0);
        crate::amulet_rules::apply_replay_policy(&mut initial, &self.record.amulet_rules_from, 0);
        crate::potion_rules::apply_replay_policy(&mut initial, &self.record.potion_rules_from, 0);
        crate::ring_rules::apply_replay_policy(&mut initial, &self.record.ring_rules_from, 0);
        crate::gas_rules::apply_replay_policy(&mut initial, &self.record.gas_rules_from, 0);
        crate::commandment_rules::apply_replay_policy(&mut initial, &self.record.commandment_rules_from, 0);
        crate::screamer_rules::apply_replay_policy(&mut initial, &self.record.screamer_rules_from, 0);
        crate::ember_rules::apply_replay_policy(&mut initial, &self.record.ember_rules_from, 0);
        crate::ragelight_rules::apply_replay_policy(&mut initial, &self.record.ragelight_rules_from, 0);
        crate::balance_rules::apply_replay_policy(&mut initial, &self.record.balance_rules_from, 0);
        crate::finale::apply_replay_policy(&mut initial, &self.record.collapse_rules_from, 0);
        crate::blood_mist::apply_replay_policy(&mut initial, &self.record.blood_mist_fire_rules_from, 0);
        crate::half_here::apply_replay_policy(&mut initial, &self.record.half_here_lock_from, 0);
        crate::half_here::apply_energy_replay_policy(&mut initial, &self.record.half_here_energy_from, 0);
        crate::screamer_rules::apply_range_replay_policy(&mut initial, &self.record.screamer_range_from, 0);
        crate::death_gas_rules::apply_replay_policy(&mut initial, &self.record.death_gas_rules_from, 0);
        crate::minotaur_rules::apply_replay_policy(&mut initial, &self.record.minotaur_bump_from, 0);
        crate::borge::apply_replay_policy(&mut initial, &self.record.borge_throw_from, 0);
        crate::banshee_rules::apply_replay_policy(&mut initial, &self.record.banshee_shriek_from, 0);
        crate::local_floor_rules::apply_replay_policy(&mut initial, &self.record.local_floor_ticks_from, 0);
        crate::steam_rules::apply_replay_policy(&mut initial, &self.record.steam_rules_from, 0);
        crate::execute_rules::apply_replay_policy(&mut initial, &self.record.execute_rules_from, 0);
        crate::beckoning_rules::apply_replay_policy(&mut initial, &self.record.beckoning_rules_from, 0);
        crate::ichor_fire_rules::apply_replay_policy(&mut initial, &self.record.ichor_fire_rules_from, 0);
        crate::future_floor::apply_replay_policy(&mut initial, &self.record.future_floor_rules_from, 0);
        self.engine = Engine::new_for_replay(initial)?;
        self.record.apply_policy(&mut self.engine,0).map_err(StorageError)?;
        self.cursor = 0;
        Ok(())
    }
}

fn create_initial(record: &ReplayRecord) -> Result<Run, String> {
    if record.compressed_depths.value_or(false) { return crate::startup::create_avatar_run_replay_policies(&record.seed, &record.amulet_rules_from, &record.potion_rules_from, &record.ring_rules_from, &record.gas_rules_from, &record.future_floor_rules_from, &record.avatar_starter_policy); }
    if record.fast_mode.value_or(false) { return crate::startup::create_avatar_run_fast_replay_policies(&record.seed, &record.amulet_rules_from, &record.potion_rules_from, &record.ring_rules_from, &record.gas_rules_from, &record.future_floor_rules_from, &record.avatar_starter_policy); }
    if let Some(p) = record.playtest.as_ref() {
        if p.version == 2 {return Err("DEBUG loadout edits are not command-ledger replayable; use the saved session instead.".into());}
        let run = crate::playtest::create_run_revision_replay_policies(&record.seed, &record.mode, record.dev, p.start_floor, &p.profile, record.simulation_revision.value_or(29.), &record.amulet_rules_from, &record.potion_rules_from, &record.ring_rules_from, &record.gas_rules_from, &record.future_floor_rules_from)?;
        if run.playtest != record.playtest { return Err("Playtest replay starting profile does not match this build.".into()); }
        Ok(run)
    } else if record.playtest.is_missing() {
        crate::startup::create_run_revision_replay_policies(&record.seed, &record.mode, record.dev, record.simulation_revision.value_or(29.), &record.amulet_rules_from, &record.potion_rules_from, &record.ring_rules_from, &record.gas_rules_from, &record.future_floor_rules_from, &record.avatar_starter_policy)
    } else { Err("Invalid null playtest replay profile.".into()) }
}

include!("replay_policy_tests.rs");

#[cfg(test)]
mod avatar_starter_policy_tests {
    use super::*;

    #[test]
    fn avatar_starter_policy_survives_save_export_and_rewind() {
        let mut run: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        run.simulation_revision = 52.0.into();
        let legacy = serde_json::to_value(ReplayRecord::from_run(&run)).unwrap();
        assert!(legacy.get("avatarStarterPolicy").is_none());

        let before = crate::temporal::Snapshot::capture(&run);
        run.avatar_starter_policy = 1u8.into();
        assert!(crate::temporal::Snapshot::capture(&run).value().unwrap().get("avatarStarterPolicy").is_none());
        before.restore(&mut run).unwrap();
        assert_eq!(run.avatar_starter_policy, Field::Value(1));

        let saved = serde_json::to_string(&run).unwrap();
        let resumed: Run = serde_json::from_str(&saved).unwrap();
        let record = ReplayRecord::from_run(&resumed);
        assert_eq!(record.avatar_starter_policy, Field::Value(1));
        let json = serde_json::to_string(&record).unwrap();
        let mut restored: ReplayRecord = serde_json::from_str(&json).unwrap();
        assert!(restored.validate_policy().is_ok());
        restored.avatar_starter_policy = Field::Value(2);
        assert!(restored.validate_policy().is_err());
        restored.avatar_starter_policy = Field::Null;
        assert!(restored.validate_policy().is_err());
        restored.avatar_starter_policy = Field::Value(1);
        restored.mode = "Blessed".into();
        assert!(restored.validate_policy().is_err());
        restored.mode = "Avatar".into();
        restored.simulation_revision = 51.0.into();
        assert!(restored.validate_policy().is_err());
    }
}
