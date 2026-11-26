//! Legacy (revision29–40) Revert snapshots and shared explicit Future anchors.
//! Revision41 bypasses retained history; ordinary command rollback remains in engine.rs.
//! Free actions belong to the latest completed frame; a pending haste phase has its own opening.
use crate::storage::{self, StorageError};
use crate::{Field, Run, RunTimeCosts};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::VecDeque, sync::{Arc, Mutex, Weak}};
type Result<T> = std::result::Result<T, StorageError>;
#[cfg(test)]
thread_local! { static SNAPSHOT_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
include!("snapshot_clone.rs");
/// O(1) revision policy. Only legacy rules retain Revert history.
pub fn retains_revert_history(run: &Run) -> bool {
    run.simulation_revision.value_or(39.0) < 41.0
}
/// O(1) retired-history admission; no world traversal or serialization.
pub(crate) fn validate_retired_history(run: &Run) -> Result<()> {
    if !retains_revert_history(run) && (!run.history.is_empty() || run.haste_snapshot.as_ref().is_some_and(|s| !s.is_empty())) {
        return Err(StorageError("Revision41 does not admit legacy Revert history".into()));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ArchiveCursor {
    pub messages: usize,
    pub events: usize,
}
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub state: Arc<Run>,
    pub cursor: Option<ArchiveCursor>,
    /// Runtime-only evidence for the transition ending here; never serialized.
    incoming: Option<Arc<crate::mutation_journal::Journal>>,
}
impl Snapshot {
    /// O(F+A+I+M) scalar/collection copy, excluding floor cells, floor actors, ground and archive.
    /// A here is the player's nested state; I inventory; M non-floor metadata. Anchor strings still copy.
    pub fn capture(run: &Run) -> Self {
        #[cfg(test)] SNAPSHOT_CALLS.with(|n| n.set(n.get()+1));
    let _timing_function = crate::timing::span("history.snapshot");
        Self {
            state: Arc::new(clone_snapshot_state(run)),
            incoming: None,
            cursor: run.log_archive.as_ref().map(|a| ArchiveCursor {
                messages: a.messages.len(),
                events: a.events.len(),
            }),
        }
    }
    pub fn value(&self) -> Result<Value> {
        let mut v = serde_json::to_value(&*self.state)?;
        let o = v.as_object_mut().unwrap();
        for k in ["history", "replay", "hasteSnapshot", "logArchive", "runId", "replaySnapshotFrom", "fireboltTravelFrom", "amuletRulesFrom", "potionRulesFrom", "ringRulesFrom", "gasRulesFrom", "commandmentRulesFrom", "screamerRulesFrom", "avatarStarterPolicy", "emberRulesFrom", "ragelightRulesFrom", "balanceRulesFrom", "collapseRulesFrom", "bloodMistFireRulesFrom", "halfHereLockFrom", "halfHereEnergyFrom", "screamerRangeFrom", "steamRulesFrom", "executeRulesFrom", "beckoningRulesFrom", "ichorFireRulesFrom", "futureFloorRulesFrom"] {
            o.shift_remove(k);
        }
        if let Some(c) = self.cursor {
            o.insert("logArchiveCursor".into(), serde_json::to_value(c)?);
        }
        Ok(v)
    }
    /// Exact snapshot-equivalence guard for identity reuse. O(R + M) time and
    /// O(R + M) scratch for R large-block handles and M serialized metadata bytes.
    /// Never descends into physical/dream floors or actor/ground block payloads.
    /// Different COW identities conservatively miss even when their values match.
    fn same_boundary(&self, other: &Snapshot) -> Result<bool> {
        let _timing = crate::timing::span("history.opening_identity_guard");
        if self.cursor != other.cursor
            || !same_arc_array(&self.state.floors, &other.state.floors)
            || !same_arc_map(&self.state.actors, &other.state.actors)
            || !same_arc_map(&self.state.ground, &other.state.ground) {
            return Ok(false);
        }
        let both_dream = match (self.state.dreamwalking.as_ref(), other.state.dreamwalking.as_ref()) {
            (Some(a), Some(b)) if same_arc_array(&a.floors, &b.floors) => true,
            (None, None) => false,
            _ => return Ok(false),
        };
        // Future compares shared immutable identity or exact legacy text without
        // encoding. Distinct native identities conservatively miss; None and
        // empty imported text remain different. Other fields stay serialized.
        if self.state.future != other.state.future { return Ok(false); }
        let mut a = snapshot_header(self, both_dream);
        let mut b = snapshot_header(other, both_dream);
        a.future = None;
        b.future = None;
        // Byte comparison also checks ordered maps and floating spelling. Pointer
        // identity alone is insufficient: Engine.run permits external header edits.
        // Immutable Future handles clone in O(1); distinct native handles conservatively miss.
        Ok(serde_json::to_vec(&a)? == serde_json::to_vec(&b)?)
    }

    pub fn from_value(mut v: Value) -> Result<Self> {
        let o = v
            .as_object_mut()
            .ok_or_else(|| StorageError("Invalid snapshot".into()))?;
        let cursor = o
            .shift_remove("logArchiveCursor")
            .map(serde_json::from_value)
            .transpose()?;
        o.insert("history".into(), json!([]));
        o.insert("replay".into(), json!([]));
        o.shift_remove("replaySnapshotFrom");
        o.shift_remove("fireboltTravelFrom");
        o.shift_remove("amuletRulesFrom");
        o.shift_remove("potionRulesFrom");
        o.shift_remove("ringRulesFrom");
        o.shift_remove("gasRulesFrom");
        o.shift_remove("commandmentRulesFrom");
        o.shift_remove("screamerRulesFrom");
        o.shift_remove("avatarStarterPolicy");
        o.shift_remove("emberRulesFrom");
        o.shift_remove("ragelightRulesFrom");
        o.shift_remove("balanceRulesFrom");
        o.shift_remove("collapseRulesFrom");
        o.shift_remove("bloodMistFireRulesFrom");
        o.shift_remove("halfHereLockFrom");
        o.shift_remove("halfHereEnergyFrom");
        o.shift_remove("screamerRangeFrom");
        o.shift_remove("borgeThrowFrom");
        o.shift_remove("steamRulesFrom");
        o.shift_remove("executeRulesFrom");
        o.shift_remove("beckoningRulesFrom");
        o.shift_remove("ichorFireRulesFrom");
        o.shift_remove("futureFloorRulesFrom");
        let state = serde_json::from_value(v)?;
        crate::revision52_amulet_tiers::validate_run(&state).map_err(StorageError)?;
        crate::revision50_control_amulets::validate_terrain(&state).map_err(StorageError)?;
        Ok(Self {
            state: Arc::new(state),
            cursor,
            incoming: None,
        })
    }
    pub fn restore(&self, run: &mut Run) -> Result<()> {
        crate::revision52_amulet_tiers::validate_run(&self.state).map_err(StorageError)?;
        crate::revision50_control_amulets::validate_terrain(&self.state).map_err(StorageError)?;
        let mut restored = (*self.state).clone();
        restored.firebolt_travel_from = run.firebolt_travel_from.clone();
        restored.amulet_rules_from = run.amulet_rules_from.clone();
        restored.potion_rules_from = run.potion_rules_from.clone();
        restored.ring_rules_from = run.ring_rules_from.clone();
        restored.gas_rules_from = run.gas_rules_from.clone();
        restored.commandment_rules_from = run.commandment_rules_from.clone();
        restored.screamer_rules_from = run.screamer_rules_from.clone();
        restored.ragelight_rules_from = run.ragelight_rules_from.clone();
        restored.balance_rules_from = run.balance_rules_from.clone();
        restored.collapse_rules_from = run.collapse_rules_from.clone();
        restored.blood_mist_fire_rules_from = run.blood_mist_fire_rules_from.clone();
        restored.half_here_lock_from = run.half_here_lock_from.clone();
        restored.half_here_energy_from = run.half_here_energy_from.clone();
        restored.screamer_range_from = run.screamer_range_from.clone();
        restored.death_gas_rules_from = run.death_gas_rules_from.clone();
        restored.minotaur_bump_from = run.minotaur_bump_from.clone();
        restored.borge_throw_from = run.borge_throw_from.clone();
        restored.execute_rules_from = run.execute_rules_from.clone();
        restored.beckoning_rules_from = run.beckoning_rules_from.clone();
        restored.ichor_fire_rules_from = run.ichor_fire_rules_from.clone();
        restored.avatar_starter_policy = run.avatar_starter_policy.clone();
        restored.ember_rules_from = run.ember_rules_from.clone();
        restored.steam_rules_from = run.steam_rules_from.clone();
        restored.future_floor_rules_from = run.future_floor_rules_from.clone();
        restored.future_floor = run.future_floor.clone();
        crate::revision52_reserves::normalize_run(&mut restored).map_err(StorageError)?;
        crate::revision50::settle_current_duels(&mut restored);
        if let Some(c) = self.cursor {
            let a = run
                .log_archive
                .as_ref()
                .ok_or_else(|| StorageError("Snapshot archive prefix is missing".into()))?;
            if c.messages > a.messages.len() || c.events > a.events.len() {
                return Err(StorageError(
                    "Snapshot archive cursor exceeds current archive".into(),
                ));
            }
        }
        let mut archive = std::mem::take(&mut run.log_archive);
        if let Some(c) = self.cursor {
            let a = archive
                .as_mut()
                .ok_or_else(|| StorageError("Snapshot archive prefix is missing".into()))?;
            if c.messages > a.messages.len() || c.events > a.events.len() {
                return Err(StorageError(
                    "Snapshot archive cursor exceeds current archive".into(),
                ));
            }
            a.messages.truncate(c.messages);
            a.events.truncate(c.events);
        } else {
            archive = Field::Missing;
        }
        let replay = std::mem::take(&mut run.replay);
        let replay_policy = std::mem::take(&mut run.replay_snapshot_from);
        let history = std::mem::take(&mut run.history);
        let run_id = std::mem::take(&mut run.run_id);
        *run = restored;
        run.replay = replay;
        run.replay_snapshot_from = replay_policy;
        run.history = history;
        run.run_id = run_id;
        run.log_archive = archive;
        run.haste_snapshot = Field::Missing;
        Ok(())
    }
}
/// An exact encoded edge keyed by immutable endpoints, not addresses alone.
/// Weak references neither retain snapshot state nor allow address reuse to hit.
#[derive(Debug, Clone)]
struct ExportedFrame {
    before: Weak<Run>,
    after: Weak<Run>,
    before_cursor: Option<ArchiveCursor>,
    after_cursor: Option<ArchiveCursor>,
    encoded: Arc<str>,
}
impl ExportedFrame {
    /// O(1): Weak identity remains unique even after an allocation is dropped;
    /// Arc::make_mut dissociates Weak pointers before any in-place mutation.
    fn matches(&self, before: &Snapshot, after: &Snapshot) -> bool {
        self.before.ptr_eq(&Arc::downgrade(&before.state))
            && self.after.ptr_eq(&Arc::downgrade(&after.state))
            && self.before_cursor == before.cursor
            && self.after_cursor == after.cursor
    }
}
#[derive(Debug, Clone, Default)]
pub struct History {
    /// Runtime policy only; old command journals explicitly select legacy replay.
    full_snapshot: bool,
    pub frames: VecDeque<Snapshot>,
    pub pending_haste: Option<Snapshot>,
    pub closing: Option<Snapshot>,
    // Runtime-only, bounded to five edges. Shared clones may evict each other's
    // entries but cannot hit different endpoints. Never held during serialization.
    exported: Arc<Mutex<Vec<ExportedFrame>>>,
    pending_writes: crate::mutation_journal::Journal,
}
impl History {
    /// DEBUG stash/entry edits are zero-turn actions performed outside dispatch.
    /// Only a changed serialized revision requires a new closing snapshot; normal
    /// runs and normal free-action ownership are untouched. Pending haste keeps
    /// its existing opening snapshot, which already restores both pack and stash.
    fn debug_loadout_changed(&self, run: &Run) -> bool {
        crate::debug_floor::state(run).is_some_and(|current| {
            self.closing.as_ref().and_then(|s| crate::debug_floor::state(&s.state))
                .is_none_or(|prior| prior.loadout_revision != current.loadout_revision)
        })
    }
    fn sync_debug_loadout(&mut self, run: &Run) {
        if !retains_revert_history(run) { return; }
        if self.pending_haste.is_none()
            && !run.haste_snapshot.as_ref().is_some_and(|s| !s.is_empty())
            && self.debug_loadout_changed(run) {
            self.closing = Some(Snapshot::capture(run));
        }
    }
    /// Amortized O(1) frame bookkeeping, plus destruction of evicted state/evidence.
    pub fn record_completed(&mut self, before: Snapshot) {
        self.closing = None;
        self.frames.push_back(before);
        if self.frames.len() > 5 {
            self.frames.pop_front();
        }
        // No retained transition ends at the oldest opening; release its evidence.
        if let Some(first) = self.frames.front_mut() { first.incoming = None; }
    }
    pub fn record_free(&mut self, _before: Snapshot) {} // Current world already includes free mutations; opening baseline remains exact.
    pub fn import(run: &Run) -> Result<Self> {
        if !retains_revert_history(run) {
            validate_retired_history(run)?;
            return Ok(Self::default());
        }
        let mut original = run.clone();
        let mut h = Self::default();
        if let Some(raw) = run.haste_snapshot.as_ref().filter(|raw| !raw.is_empty()) {
            let snap = Snapshot::from_value(serde_json::from_str(raw)?)?;
            snap.restore(&mut original)?;
            h.pending_haste = Some(snap);
        }
        h.closing = Some(Snapshot::capture(&original));
        let mut v = serde_json::to_value(&original)?;
        for raw in run.history.iter().rev() {
            let frame: Frame = serde_json::from_str(raw)?;
            for group in frame.groups.iter().rev() {
                apply(&mut v, group)?;
            }
            v["future"] = frame.future.map(Value::String).unwrap_or(Value::Null);
            let previous: Run = serde_json::from_value(v.clone())?;
            h.frames.push_front(Snapshot::capture(&previous));
        }
        Ok(h)
    }
    /// Temporal commands do not append free inverse groups. Rebase untouched paths
    /// onto their new current values before a later completed turn links snapshots.
    /// O(H*S), only at temporal command boundaries, H <= 5.
    fn rebase_temporal(&mut self, run: &Run) -> Result<()> {
        if !retains_revert_history(run) { return Ok(()); }
        // O(snapshot header): full overwrite retains original openings, not selective rebases.
        if self.full_snapshot {
            if self.pending_haste.is_none() { self.closing = Some(Snapshot::capture(run)); }
            self.pending_writes = Default::default();
            return Ok(());
        }
        if self.pending_haste.is_some() {
            return Ok(());
        }
        let mut old_newer = self
            .closing
            .clone()
            .unwrap_or_else(|| Snapshot::capture(run));
        let mut current = run.clone();
        let mut rebased = VecDeque::new();
        for older in self.frames.iter().rev() {
            let patches = inverse(older, &old_newer, true)?;
            let mut value = serde_json::to_value(&current)?;
            apply(&mut value, &patches)?;
            current = serde_json::from_value(value)?;
            current.future = older.state.future.clone();
            rebased.push_front(Snapshot::capture(&current));
            old_newer = older.clone();
        }
        self.frames = rebased;
        self.pending_writes = Default::default();
        self.closing = Some(Snapshot::capture(run));
        Ok(())
    }
    /// Consume an already sealed exact edge for Revert. O(encoded frame bytes)
    /// parsing on a hit; otherwise the documented selective inverse fallback.
    /// Identity + archive cursors match before reuse; no write-history assumptions.
    fn recorded_inverse(&self, before: &Snapshot, after: &Snapshot) -> Result<Vec<Patch>> {
        let encoded = self.exported.lock().unwrap_or_else(|e| e.into_inner())
            .iter().find(|edge| edge.matches(before, after)).map(|edge| edge.encoded.clone());
        if let Some(encoded) = encoded {
            let _timing = crate::timing::span("history.recorded_inverse");
            let frame: Frame = serde_json::from_str(&encoded)?;
            // Runtime seals always emit a single net-difference group. Do not
            // accidentally reinterpret a different grouping contract in future.
            if frame.groups.len() == 1 { return Ok(frame.groups.into_iter().next().unwrap()); }
        }
        let _timing = crate::timing::span("history.recorded_inverse_miss");
        inverse(before, after, true)
    }

    /// O(H) exact identity/cursor comparisons, H <= 5; no payload traversal.
    /// Cloned histories may share the cache, so only locally retained edges survive.
    fn retains_edge(&self, entry: &ExportedFrame) -> bool {
        let Some(mut newer) = self.pending_haste.as_ref().or(self.closing.as_ref()) else { return false; };
        for older in self.frames.iter().rev() {
            if entry.matches(older, newer) { return true; }
            newer = older;
        }
        false
    }

    /// Seal the current completed edge when a command finishes, not at a later save.
    /// O(changed block bytes * nesting + metadata + patch bytes + H*C),
    /// H,C <= 5 for retained-edge pruning; O(C) on a cache hit.
    /// Pending haste keeps the prior completed edge's opening, never partial state.
    fn seal_current(&self) -> Result<()> {
        let _timing = crate::timing::span("history.journal_seal");
        let Some(before) = self.frames.back() else { return Ok(()); };
        let Some(after) = self.pending_haste.as_ref().or(self.closing.as_ref()) else { return Ok(()); };
        if self.exported.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|e| e.matches(before, after)) {
            return Ok(());
        }
        let patches = inverse(before, after, true)?;
        let encoded = js_stringify(&serde_json::to_value(Frame {
            groups: vec![patches], future: crate::future_anchor::wire_option(&before.state.future)?,
        })?);
        let entry = ExportedFrame {
            before: Arc::downgrade(&before.state), after: Arc::downgrade(&after.state),
            before_cursor: before.cursor, after_cursor: after.cursor,
            encoded: Arc::from(encoded),
        };
        let mut cache = self.exported.lock().unwrap_or_else(|e| e.into_inner());
        // Export stores newest-first, while seals append: index zero is not a
        // reliable oldest-entry policy. Prune by actual retained transitions.
        if cache.iter().any(|edge| edge.matches(before, after)) { return Ok(()); }
        cache.retain(|edge| self.retains_edge(edge));
        if cache.len() >= 5 { cache.remove(0); }
        cache.push(entry);
        Ok(())
    }

    /// Boundary-only compatible legacy inverse history (H <= 5).
    /// Cold O(H*S + D + B), warm O(H² + B + run clone), where S is snapshot
    /// JSON size, D recursive diff/path/compaction work and B encoded history bytes.
    /// Cache is at most five strings; cold scratch is two JSON endpoints + patches.
    /// Dispatch reuses a previous closing identity only after exact root/header
    /// checks, preserving the previously sealed edge without trusting public writes.
    /// Shared unchanged payloads are skipped; no world/disk work is skipped.
    pub fn export(&self, run: &Run) -> Result<Run> {
        if !retains_revert_history(run) { validate_retired_history(run)?; crate::timeweave::validate_run(run).map_err(StorageError)?; return Ok(run.clone()); }
    let _timing_function = crate::timing::span("history.export");
        let mut out = run.clone();
        let mut newer = self
            .pending_haste
            .clone()
            .or_else(|| if self.debug_loadout_changed(run) { Some(Snapshot::capture(run)) } else { self.closing.clone() })
            .unwrap_or_else(|| Snapshot::capture(run));
        let cached = self.exported.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let mut next_cache = Vec::new();
        let mut frames = Vec::new();
        for older in self.frames.iter().rev() {
            let entry = if let Some(entry) = cached.iter().find(|e| e.matches(older, &newer)) {
                let _timing = crate::timing::span("history.export_cache_hit");
                entry.clone()
            } else {
                let patches = if self.full_snapshot { snapshot_overwrite_patches(older, &newer)? } else { inverse(older, &newer, true)? };
                let encoded = js_stringify(&serde_json::to_value(Frame {
                    groups: vec![patches],
                    future: crate::future_anchor::wire_option(&older.state.future)?,
                })?);
                ExportedFrame {
                    before: Arc::downgrade(&older.state),
                    after: Arc::downgrade(&newer.state),
                    before_cursor: older.cursor,
                    after_cursor: newer.cursor,
                    encoded: Arc::from(encoded),
                }
            };
            frames.push(entry.encoded.to_string());
            if next_cache.len() < 5 { next_cache.push(entry); }
            newer = older.clone();
        }
        *self.exported.lock().unwrap_or_else(|e| e.into_inner()) = next_cache;
        frames.reverse();
        out.history = frames;
        if let Some(p) = &self.pending_haste {
            out.haste_snapshot = Field::Value(js_stringify(&p.value()?));
        }
        Ok(out)
    }
}
fn costs(run: &Run) -> RunTimeCosts {
    run.time_costs.value_or_default()
}
fn retain_costs(run: &mut Run, current: RunTimeCosts) {
    let restored = costs(run);
    let next = (run.player.energy - (current.energy - restored.energy).max(0.0)).max(0.0);
    let loss = (run.player.energy - next).max(0.0);
    record_energy_loss(run, loss);
    run.player.energy = next;
    if retains_revert_history(run) {
        let charges = run.player.conditions.entry("revertCharges".into()).or_default();
        *charges = (*charges - (current.charges - restored.charges).max(0.0)).max(0.0);
    }
    run.time_costs = Field::Value(current);
}
fn record_energy_loss(run: &mut Run, amount: f64) {
    crate::resources::record_player_energy_loss(run, amount);
}

pub fn revert(run: &mut Run, history: &mut History) -> Result<bool> {
    if !retains_revert_history(run) { return Err(StorageError("Revert is unavailable under revision41 rules".into())); }
    history.sync_debug_loadout(run);
    if history.frames.is_empty()
        || run
            .player
            .conditions
            .get("revertCharges")
            .copied()
            .unwrap_or(0.0)
            < 1.0
        || run.player.energy < 2.0
    {
        return Ok(false);
    }
    let mut ledger = costs(run);
    ledger.energy += 2.0;
    ledger.charges += 1.0;
    if let Some(raw) = run.haste_snapshot.as_ref().filter(|raw| !raw.is_empty()) {
        let pending = Snapshot::from_value(serde_json::from_str(raw)?)?;
        pending.restore(run)?;
    } else if let Some(pending) = &history.pending_haste {
        pending.restore(run)?;
    }
    let frame = history.frames.back().unwrap().clone();
    if history.full_snapshot {
        // O(snapshot header + discarded state); large floor/actor blocks remain COW.
        // Preserve live replay/runId, rewind logs/archive, then reapply cumulative costs.
        frame.restore(run)?;
        crate::health_dirty::invalidate(run);
        history.frames.pop_back();
        history.closing = Some(frame);
        history.pending_haste = None;
        history.pending_writes = Default::default();
        retain_costs(run, ledger);
        return Ok(true);
    }
    let closing = history
        .closing
        .clone()
        .unwrap_or_else(|| Snapshot::capture(run));
    // Revert must apply only the original transition's changed paths. Whole-state restore
    // would incorrectly erase resource telemetry introduced by an earlier Revert cost.
    let patches = history.recorded_inverse(&frame, &closing)?;
    let mut value = serde_json::to_value(&*run)?;
    apply(&mut value, &patches)?;
    let mut restored: Run = serde_json::from_value(value)?;
    restored.future = frame.state.future.clone();
    restored.haste_snapshot = Field::Missing;
    *run = restored;
    history.frames.pop_back();
    history.closing = Some(frame);
    history.pending_haste = None;
    history.pending_writes = Default::default();
    retain_costs(run, ledger);
    Ok(true)
}
/// Expected O(ID bytes), read-only admission for the two pools paid by Future.
pub(crate) fn validate_future_capture(run: &Run) -> Result<()> {
    if crate::revision50::enabled(run) {
        crate::revision50_ability_energy::plan_future_payment_for_revision(&run.player, run.simulation_revision.value_or(29.))
            .map_err(|e| StorageError(format!("Invalid See The Future payment: {e:?}")))?;
    }
    Ok(())
}
fn future_spends_energy(run: &Run) -> bool {
    run.player.energy > 0.0 || crate::revision50::enabled(run)
        && run.player.ability_reserves.as_ref()
            .and_then(|m| m.get(crate::revision50_ability_energy::FUTURE_ID))
            .is_some_and(|reserve| reserve.energy > 0.0)
}
pub fn remember_future(run: &mut Run, history: &History) -> Result<()> {
    validate_future_capture(run)?;
    if crate::revision50::enabled(run) {
        let plan = crate::revision50_ability_energy::plan_future_payment_for_revision(&run.player, run.simulation_revision.value_or(29.))
            .map_err(|e| StorageError(format!("Invalid See The Future payment: {e:?}")))?;
        // O(owned Run data + COW mutations + anchor scan): prepare paid state and
        // capture off to the side so every fallible step precedes publication.
        let mut paid = run.clone();
        let mut ledger = costs(&paid);
        ledger.energy += plan.actor_energy_spent();
        if !ledger.energy.is_finite() { return Err(StorageError("See The Future time cost overflow".into())); }
        paid.time_costs = ledger.into();
        record_energy_loss(&mut paid, plan.actor_energy_spent());
        crate::revision50_ability_energy::commit_payment(&plan,
            paid.player.ability_reserves.as_mut().unwrap(), &mut paid.player.energy)
            .map_err(|e| StorageError(format!("Invalid See The Future payment: {e:?}")))?;
        crate::half_here::sync(&mut paid);
        paid.future = Some(crate::future_anchor::FutureAnchor::capture41(&paid)?);
        *run = paid;
        return Ok(());
    }
    let mut ledger = costs(run);
    ledger.energy += run.player.energy;
    run.time_costs = Field::Value(ledger);
    record_energy_loss(run, run.player.energy.max(0.0));
    run.player.energy = 0.0;
    if !retains_revert_history(run) {
        run.future=Some(crate::future_anchor::FutureAnchor::capture41(run)?);
        return Ok(());
    }
    // The anchor intentionally excludes completed history. Avoid encoding five
    // full snapshot frames merely to discard them in the new policy.
    let mut copy = if history.full_snapshot {
        let mut copy = run.clone();
        if let Some(p) = &history.pending_haste { copy.haste_snapshot = Field::Value(js_stringify(&p.value()?)); }
        copy
    } else { history.export(run)? };
    copy.future = None;
    copy.history.clear();
    copy.replay.clear();
    copy.replay_snapshot_from = Field::Missing;
    copy.firebolt_travel_from = Field::Missing;
    copy.amulet_rules_from = Field::Missing;
    copy.potion_rules_from = Field::Missing;
    copy.ring_rules_from = Field::Missing;
    copy.gas_rules_from = Field::Missing;
    copy.commandment_rules_from = Field::Missing;
    copy.screamer_rules_from = Field::Missing;
    copy.ragelight_rules_from = Field::Missing;
    copy.balance_rules_from = Field::Missing;
    copy.collapse_rules_from = Field::Missing;
    copy.blood_mist_fire_rules_from = Field::Missing;
    copy.half_here_lock_from = Field::Missing;
    copy.half_here_energy_from = Field::Missing;
    copy.screamer_range_from = Field::Missing;
    copy.death_gas_rules_from = Field::Missing;
    copy.minotaur_bump_from = Field::Missing;
    copy.borge_throw_from = Field::Missing;
    copy.execute_rules_from = Field::Missing;
    copy.beckoning_rules_from = Field::Missing;
    copy.ichor_fire_rules_from = Field::Missing;
    copy.avatar_starter_policy = Field::Missing;
    copy.ember_rules_from = Field::Missing;
    copy.steam_rules_from = Field::Missing;
    copy.future_floor_rules_from = Field::Missing;
    copy.future_floor = None;
    run.future = Some(storage::serialize(&copy)?.into());
    Ok(())
}
/// O(anchor bytes + admission); prepare all fallible new-rule work before live mutation.
fn prepare_future41(run: &Run) -> Result<(Run,History)> {
    let raw=run.future.as_ref().ok_or_else(||StorageError("Future anchor is missing".into()))?;
    let restored=raw.prepare_restore()?;
    if restored.simulation_revision != run.simulation_revision {
        return Err(StorageError("Future anchor mechanics revision differs from current run".into()));
    }
    crate::timeweave::validate_run(&restored).map_err(StorageError)?;
    let mut history=History::import(&restored)?;
    history.full_snapshot=true;
    Ok((restored,history))
}
/// O(discarded current state + retained replay transfer); admission already succeeded.
fn commit_future41(run:&mut Run, history:&mut History, mut restored:Run, next_history:History) {
    let ledger=costs(run);
    restored.replay=std::mem::take(&mut run.replay);
    restored.replay_snapshot_from=run.replay_snapshot_from.clone();
    restored.firebolt_travel_from=run.firebolt_travel_from.clone();
    restored.amulet_rules_from=run.amulet_rules_from.clone();
    restored.potion_rules_from=run.potion_rules_from.clone();
    restored.ring_rules_from=run.ring_rules_from.clone();
    restored.gas_rules_from=run.gas_rules_from.clone();
    restored.commandment_rules_from=run.commandment_rules_from.clone();
    restored.screamer_rules_from=run.screamer_rules_from.clone();
    restored.ragelight_rules_from=run.ragelight_rules_from.clone();
    restored.balance_rules_from=run.balance_rules_from.clone();
    restored.collapse_rules_from=run.collapse_rules_from.clone();
    restored.blood_mist_fire_rules_from=run.blood_mist_fire_rules_from.clone();
    restored.half_here_lock_from=run.half_here_lock_from.clone();
    restored.half_here_energy_from=run.half_here_energy_from.clone();
    restored.screamer_range_from=run.screamer_range_from.clone();
    restored.death_gas_rules_from=run.death_gas_rules_from.clone();
    restored.minotaur_bump_from=run.minotaur_bump_from.clone();
    restored.borge_throw_from=run.borge_throw_from.clone();
    restored.execute_rules_from=run.execute_rules_from.clone();
    restored.beckoning_rules_from=run.beckoning_rules_from.clone();
    restored.ichor_fire_rules_from=run.ichor_fire_rules_from.clone();
    restored.avatar_starter_policy=run.avatar_starter_policy.clone();
    restored.ember_rules_from=run.ember_rules_from.clone();
    restored.steam_rules_from=run.steam_rules_from.clone();
    restored.future_floor_rules_from=run.future_floor_rules_from.clone();
    restored.future_floor=run.future_floor.clone();
    restored.run_id=run.run_id.clone();restored.future=None;
    *history=next_history;*run=restored;retain_costs(run,ledger);crate::health_dirty::invalidate(run);
}
pub fn restore_future(run: &mut Run, history: &mut History) -> Result<bool> {
    if !retains_revert_history(run) {
        if run.future.is_none() {return Ok(false);}
        let (restored,next_history)=prepare_future41(run)?;
        commit_future41(run,history,restored,next_history);return Ok(true);
    }
    let Some(raw) = run.future.as_ref() else {
        return Ok(false);
    };
    let mut restored = storage::deserialize(raw.wire_text()?)?;
    let ledger = costs(run);
    restored.replay = std::mem::take(&mut run.replay);
    restored.replay_snapshot_from = run.replay_snapshot_from.clone();
    restored.firebolt_travel_from = run.firebolt_travel_from.clone();
        restored.amulet_rules_from = run.amulet_rules_from.clone();
        restored.potion_rules_from = run.potion_rules_from.clone();
        restored.ring_rules_from = run.ring_rules_from.clone();
        restored.gas_rules_from = run.gas_rules_from.clone();
        restored.commandment_rules_from = run.commandment_rules_from.clone();
        restored.screamer_rules_from = run.screamer_rules_from.clone();
        restored.ragelight_rules_from = run.ragelight_rules_from.clone();
        restored.balance_rules_from = run.balance_rules_from.clone();
        restored.collapse_rules_from = run.collapse_rules_from.clone();
        restored.blood_mist_fire_rules_from = run.blood_mist_fire_rules_from.clone();
        restored.half_here_lock_from = run.half_here_lock_from.clone();
        restored.half_here_energy_from = run.half_here_energy_from.clone();
        restored.screamer_range_from = run.screamer_range_from.clone();
        restored.death_gas_rules_from = run.death_gas_rules_from.clone();
        restored.minotaur_bump_from = run.minotaur_bump_from.clone();
        restored.borge_throw_from = run.borge_throw_from.clone();
        restored.execute_rules_from = run.execute_rules_from.clone();
        restored.beckoning_rules_from = run.beckoning_rules_from.clone();
        restored.ichor_fire_rules_from = run.ichor_fire_rules_from.clone();
        restored.avatar_starter_policy = run.avatar_starter_policy.clone();
        restored.ember_rules_from = run.ember_rules_from.clone();
        restored.steam_rules_from = run.steam_rules_from.clone();
        restored.future_floor_rules_from = run.future_floor_rules_from.clone();
        restored.future_floor = run.future_floor.clone();
    restored.run_id = run.run_id.clone();
    restored.future = None;
    let policy = history.full_snapshot;
    *history = History::import(&restored)?;
    history.full_snapshot = policy;
    *run = restored;
    retain_costs(run, ledger);
    Ok(true)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    pub path: Vec<Value>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub value: Field<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remove: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<usize>,
}
impl Patch {
    fn value(path: Vec<Value>, value: Value) -> Self {
        Self {
            path,
            value: Field::Value(value),
            remove: None,
            length: None,
        }
    }
    fn remove(path: Vec<Value>) -> Self {
        Self {
            path,
            value: Field::Missing,
            remove: Some(true),
            length: None,
        }
    }
    fn length(path: Vec<Value>, n: usize) -> Self {
        Self {
            path,
            value: Field::Missing,
            remove: None,
            length: Some(n),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub groups: Vec<Vec<Patch>>,
    pub future: Option<String>,
}
fn walk(a: &Value, b: Option<&Value>, path: Vec<Value>, patches: &mut Vec<Patch>) {
    if Some(a) == b {
        return;
    }
    if let (Value::Array(aa), Some(Value::Array(bb))) = (a, b) {
        let start = patches.len();
        if aa.len() != bb.len() {
            patches.push(Patch::length(path.clone(), aa.len()));
        }
        for (i, av) in aa.iter().enumerate() {
            let mut p = path.clone();
            p.push(json!(i));
            walk(av, bb.get(i), p, patches);
        }
        if patches.len() > start + 1 && aa.iter().all(|v| !v.is_array() && !v.is_object()) {
            let whole = Patch::value(path, a.clone());
            let sparse: usize = patches[start..]
                .iter()
                .map(|p| {
                    js_stringify(&serde_json::to_value(p).unwrap())
                        .encode_utf16()
                        .count()
                        + 1
                })
                .sum();
            if js_stringify(&serde_json::to_value(&whole).unwrap())
                .encode_utf16()
                .count()
                < sparse
            {
                patches.truncate(start);
                patches.push(whole);
            }
        }
        return;
    }
    if let (Value::Object(aa), Some(Value::Object(bb))) = (a, b) {
        for (k, av) in aa {
            let mut p = path.clone();
            p.push(json!(k));
            walk(av, bb.get(k), p, patches);
        }
        for k in bb.keys().filter(|k| !aa.contains_key(*k)) {
            let mut p = path.clone();
            p.push(json!(k));
            patches.push(Patch::remove(p));
        }
        return;
    }
    patches.push(Patch::value(path, a.clone()));
}
/// O((M+D)*L + P + J) conservative selective diff bound: M metadata, D changed
/// block bytes, L depth, P patches, J compaction candidates. Memory O(M+D+P+L).
pub fn inverse(before: &Snapshot, after: &Snapshot, retain_log: bool) -> Result<Vec<Patch>> {
    let (a, b) = inverse_pair_values(before, after)?;
    Ok(inverse_values(&a, &b, before.cursor, after.cursor, retain_log))
}
/// Materialize only potentially changed COW blocks and the ordinary scalar header.
/// O(F + actor/ground groups + header bytes + changed block bytes), not O(writes).
/// Shared immutable blocks are masked equally, so the original ordered walker emits
/// exactly the same selective leaf patches. Structural changes keep real old values.
fn inverse_pair_values(before: &Snapshot, after: &Snapshot) -> Result<(Value, Value)> {
    let _timing = crate::timing::span("history.journal_project");
    let both_dream = before.state.dreamwalking.as_ref().is_some()
        && after.state.dreamwalking.as_ref().is_some();
    let mut a = inverse_header(before, both_dream)?;
    let mut b = inverse_header(after, both_dream)?;
    let journal = after.incoming.as_deref();
    let (af, bf) = projected_array(&before.state.floors, &after.state.floors, journal)?;
    a["floors"] = af; b["floors"] = bf;
    let (aa, ba) = projected_map(&before.state.actors, &after.state.actors, journal)?;
    a["actors"] = aa; b["actors"] = ba;
    let (ag, bg) = projected_map(&before.state.ground, &after.state.ground, journal)?;
    a["ground"] = ag; b["ground"] = bg;
    if both_dream {
        let (ad, bd) = projected_array(
            &before.state.dreamwalking.as_ref().unwrap().floors,
            &after.state.dreamwalking.as_ref().unwrap().floors, journal)?;
        a["dreamwalking"]["floors"] = ad;
        b["dreamwalking"]["floors"] = bd;
    }
    Ok((a, b))
}

/// O(R) exact ordered handle comparison, O(1) scratch; no payload equality.
fn same_arc_array<T>(a: &[Arc<T>], b: &[Arc<T>]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a,b)| Arc::ptr_eq(a,b))
}
/// O(R + key bytes) exact ordered keys/handles, O(1) scratch; no actor/item scan.
fn same_arc_map<T>(a: &indexmap::IndexMap<String, Arc<T>>, b: &indexmap::IndexMap<String, Arc<T>>) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|((ak,av),(bk,bv))| ak == bk && Arc::ptr_eq(av,bv))
}
/// O(R + M) clone/drop of handles and metadata; large COW payloads stay shared.
fn snapshot_header(snapshot: &Snapshot, mask_dream_floors: bool) -> Run {
    let mut header = clone_snapshot_state(&snapshot.state);
    header.floors.clear();
    header.actors = Default::default();
    header.ground.clear();
    if mask_dream_floors {
        header.dreamwalking.as_mut().unwrap().floors.clear();
    }
    header
}
/// O(header bytes + number of shared block handles), excluding big COW payloads.
/// Dream-plane creation/removal retains its complete value for a structural patch.
fn inverse_header(snapshot: &Snapshot, mask_dream_floors: bool) -> Result<Value> {
    let _timing = crate::timing::span("history.journal_header");
    let mut value = serde_json::to_value(snapshot_header(snapshot, mask_dream_floors))?;
    for key in ["history", "replay", "hasteSnapshot", "logArchive", "runId", "replaySnapshotFrom", "fireboltTravelFrom", "amuletRulesFrom", "potionRulesFrom", "ringRulesFrom", "gasRulesFrom", "commandmentRulesFrom", "screamerRulesFrom", "avatarStarterPolicy", "emberRulesFrom", "ragelightRulesFrom", "balanceRulesFrom", "collapseRulesFrom", "bloodMistFireRulesFrom", "halfHereLockFrom", "halfHereEnergyFrom", "screamerRangeFrom", "steamRulesFrom", "executeRulesFrom", "beckoningRulesFrom", "ichorFireRulesFrom", "futureFloorRulesFrom"] {
        value.as_object_mut().unwrap().shift_remove(key);
    }
    Ok(value)
}

/// O(block count + changed payload bytes); unchanged Arc blocks use equal null
/// sentinels. Call only for arrays of structured objects (floors), never scalar
/// arrays: their legacy compactness selection must see the original values.
fn projected_array<T: Serialize + 'static>(a: &[Arc<T>], b: &[Arc<T>], journal: Option<&crate::mutation_journal::Journal>) -> Result<(Value, Value)> {
    let mut av = Vec::with_capacity(a.len());
    let mut bv = Vec::with_capacity(b.len());
    for i in 0..a.len().max(b.len()) {
        let unchanged = a.get(i).zip(b.get(i)).is_some_and(|(a,b)| Arc::ptr_eq(a,b));
        if let Some(v) = a.get(i) {
            av.push(if unchanged { Value::Null } else { journal_old_value(v, journal)? });
        }
        if let Some(v) = b.get(i) {
            bv.push(if unchanged { Value::Null } else { journal_new_value(v)? });
        }
    }
    Ok((Value::Array(av), Value::Array(bv)))
}

/// Expected O(group count + changed payload bytes), preserving each map's order.
/// Removed/inserted groups retain real data; only shared values on both sides mask.
fn projected_map<T: Serialize + 'static>(
    a: &indexmap::IndexMap<String, Arc<T>>,
    b: &indexmap::IndexMap<String, Arc<T>>,
    journal: Option<&crate::mutation_journal::Journal>,
) -> Result<(Value, Value)> {
    let mut av = serde_json::Map::new();
    let mut bv = serde_json::Map::new();
    for (key, value) in a {
        let unchanged = b.get(key).is_some_and(|other| Arc::ptr_eq(value, other));
        av.insert(key.clone(), if unchanged { Value::Null } else { journal_old_value(value, journal)? });
    }
    for (key, value) in b {
        let unchanged = a.get(key).is_some_and(|other| Arc::ptr_eq(value, other));
        bv.insert(key.clone(), if unchanged { Value::Null } else { journal_new_value(value)? });
    }
    Ok((Value::Object(av), Value::Object(bv)))
}

/// O(changed block bytes); current endpoint serialization, not a world scan.
fn journal_new_value<T: Serialize>(value: &Arc<T>) -> Result<Value> {
    let _span = crate::timing::span("history.journal_after_block");
    Ok(serde_json::to_value(&**value)?)
}

/// O(block bytes) serialization, expected O(1) authoritative first-write lookup.
/// Missing coverage (imports/public replacement/temporal rebase) uses the exact
/// immutable opening snapshot and is counted, never silently treated as unchanged.
fn journal_old_value<T: Serialize + 'static>(value: &Arc<T>, journal: Option<&crate::mutation_journal::Journal>) -> Result<Value> {
    use std::any::Any;
    if let Some(journal) = journal {
        let any = value as &dyn Any;
        if let Some(v) = any.downcast_ref::<Arc<crate::Floor>>().and_then(|v| journal.old_floor(v)) {
            let _span = crate::timing::span("history.journal_first_old");
            return Ok(serde_json::to_value(&**v)?);
        }
        if let Some(v) = any.downcast_ref::<Arc<Vec<crate::Actor>>>().and_then(|v| journal.old_actors(v)) {
            let _span = crate::timing::span("history.journal_first_old");
            return Ok(serde_json::to_value(&**v)?);
        }
        if let Some(v) = any.downcast_ref::<Arc<Vec<crate::GroundItem>>>().and_then(|v| journal.old_ground(v)) {
            let _span = crate::timing::span("history.journal_first_old");
            return Ok(serde_json::to_value(&**v)?);
        }
    }
    let _span = crate::timing::span("history.journal_snapshot_fallback");
    Ok(serde_json::to_value(&**value)?)
}

// Audited against Run serde fields; tools/check-snapshot-roots.py is a release gate.
const SNAPSHOT_OPTIONAL_ROOTS: &[&str] = &[
    "dreamwalking",
    "timeweave",
    "collapse",
    "playtest",
    "runStats",
    "achievements",
    "veridianSapHarvested",
    "compositeBowGenerated",
    "gameplayRevision",
    "divineRevision",
    "enemyHealthRevision",
    "simulationRevision",
    "resolvingTick",
    "timeCosts",
    "phase",
    "fastActionPhase",
    "fastMode",
    "damageSinceLastTurn",
    "damageSinceLastTurnMaxHp",
    "resourceHistory",
    "initialPlayer",
    "inventoryDisplayOrder",
    "logKinds",
    "generatedAbilityGrants",
    "knownPotionNatures",
    "rememberedItems",
    "detectedItems",
];

/// Full snapshot checkpoint encoding in the existing Frame/Patch JSON schema.
/// O(S + R) time/space: S old snapshot bytes and R fixed canonical root fields.
/// No recursive endpoint diff; this runs at export/checkpoint, never each ordinary turn.
fn snapshot_overwrite_patches(before: &Snapshot, _after: &Snapshot) -> Result<Vec<Patch>> {
    let Value::Object(mut object) = before.value()? else { unreachable!("Run serializes as object") };
    object.shift_remove("logArchiveCursor");
    // Explicit absence is independent of closing/live state and cached edge identity.
    let removals: Vec<Patch> = SNAPSHOT_OPTIONAL_ROOTS.iter().filter(|k|
        // The new root is absent by contract in29–40. Preserve old history bytes.
        !(**k == "timeweave" && retains_revert_history(&before.state))
        && !(**k == "fastActionPhase" && before.state.simulation_revision.value_or(29.) < 52.)
        && !(**k == "fastMode" && before.state.simulation_revision.value_or(29.) < 52.)
        && !(**k == "compositeBowGenerated" && before.state.simulation_revision.value_or(29.) < 51.)
        && !object.contains_key(**k))
        .map(|k| Patch::remove(vec![json!(k)])).collect();
    let mut patches: Vec<Patch> = object.into_iter().map(|(k,v)| Patch::value(vec![json!(k)],v)).collect();
    patches.extend(removals);
    match before.cursor {
        Some(c) => {
            patches.push(Patch::length(vec![json!("logArchive"),json!("messages")],c.messages));
            patches.push(Patch::length(vec![json!("logArchive"),json!("events")],c.events));
        }
        None => patches.push(Patch::remove(vec![json!("logArchive")])),
    }
    Ok(patches)
}

/// Same ordered patch construction as inverse, borrowing already serialized endpoints.
/// O(S + D) time, O(patch bytes + traversal depth) additional memory; see inverse.
fn inverse_values(
    a: &Value,
    b: &Value,
    before_cursor: Option<ArchiveCursor>,
    after_cursor: Option<ArchiveCursor>,
    retain_log: bool,
) -> Vec<Patch> {
    let _timing = crate::timing::span("history.inverse_diff");
    let mut p = Vec::new();
    walk(a, Some(b), vec![], &mut p);
    if retain_log {
        p.retain(|p| {
            !matches!(
                p.path.first().and_then(Value::as_str),
                Some("log" | "logKinds")
            )
        });
        p.push(Patch::value(vec![json!("log")], a["log"].clone()));
        p.push(if let Some(k) = a.get("logKinds") {
            Patch::value(vec![json!("logKinds")], k.clone())
        } else {
            Patch::remove(vec![json!("logKinds")])
        });
    }
    match (before_cursor, after_cursor) {
        (Some(a), Some(b)) => {
            for (k, x, y) in [
                ("events", a.events, b.events),
                ("messages", a.messages, b.messages),
            ] {
                if retain_log || x != y {
                    p.push(Patch::length(vec![json!("logArchive"), json!(k)], x));
                }
            }
        }
        (None, b) if b.is_some() || retain_log => p.push(Patch::remove(vec![json!("logArchive")])),
        _ => {}
    }
    p
}
fn descend<'a>(mut v: &'a mut Value, path: &[Value], create: bool) -> Result<&'a mut Value> {
    for key in path {
        if let Some(i) = key.as_u64() {
            let a = v
                .as_array_mut()
                .ok_or_else(|| StorageError("History array path mismatch".into()))?;
            let i = i as usize;
            if i >= a.len() {
                if !create {
                    return Err(StorageError("History index exceeds array".into()));
                }
                a.resize(i + 1, Value::Null);
            }
            v = &mut a[i];
        } else {
            let k = key
                .as_str()
                .ok_or_else(|| StorageError("Invalid history key".into()))?;
            let o = v
                .as_object_mut()
                .ok_or_else(|| StorageError("History object path mismatch".into()))?;
            if !o.contains_key(k) {
                if !create {
                    return Err(StorageError("Missing history path".into()));
                }
                o.insert(k.into(), json!({}));
            }
            v = o.get_mut(k).unwrap();
        }
    }
    Ok(v)
}
pub fn apply(v: &mut Value, patches: &[Patch]) -> Result<()> {
    for p in patches.iter().filter(|p| p.length.is_none()) {
        let Some((key, parent)) = p.path.split_last() else {
            return Err(StorageError("Empty legacy patch path".into()));
        };
        let target = descend(v, parent, true)?;
        if p.remove == Some(true) || matches!(p.value, Field::Missing) {
            if let Some(k) = key.as_str() {
                target
                    .as_object_mut()
                    .ok_or_else(|| StorageError("Remove nonobject".into()))?
                    .shift_remove(k);
            } else if let Some(i) = key.as_u64() {
                if let Some(t) = target.as_array_mut().and_then(|a| a.get_mut(i as usize)) {
                    *t = Value::Null;
                }
            }
        } else {
            let value = p.value.as_ref().cloned().unwrap_or(Value::Null);
            let t = descend(target, std::slice::from_ref(key), true)?;
            *t = value;
        }
    }
    for p in patches.iter().filter(|p| p.length.is_some()) {
        let target = descend(v, &p.path, false)?
            .as_array_mut()
            .ok_or_else(|| StorageError("Length patch requires array".into()))?;
        target.resize(p.length.unwrap(), Value::Null);
    }
    Ok(())
}
/// JS-compatible integer-valued number spelling for embedded inverse JSON. Ordinary export compares parsed JSON.
pub fn js_stringify(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => {
            let s = n.to_string();
            s.strip_suffix(".0").unwrap_or(&s).to_string()
        }
        Value::String(s) => serde_json::to_string(s).unwrap(),
        Value::Array(a) => format!(
            "[{}]",
            a.iter().map(js_stringify).collect::<Vec<_>>().join(",")
        ),
        Value::Object(o) => format!(
            "{{{}}}",
            o.iter()
                .map(|(k, v)| format!("{}:{}", serde_json::to_string(k).unwrap(), js_stringify(v)))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

/// Session owns transient COW history; legacy JSON materializes only at boundaries.
#[derive(Debug, Clone)]
pub struct Engine {
    pub run: Run,
    pub history: History,
}
impl Engine {
    /// O(world validation + history import); replay provenance checks/addition O(1).
    pub fn new(mut run: Run) -> Result<Self> {
        crate::balance_rules::admit_current(&mut run).map_err(StorageError)?;
        crate::finale::admit_current(&mut run).map_err(StorageError)?;
        crate::blood_mist::admit_current(&mut run).map_err(StorageError)?;
        crate::half_here::admit_current(&mut run).map_err(StorageError)?;
        crate::screamer_rules::admit_range(&mut run).map_err(StorageError)?;
        crate::death_gas_rules::admit(&mut run).map_err(StorageError)?;
        crate::minotaur_rules::admit(&mut run).map_err(StorageError)?;
        crate::borge::admit(&mut run).map_err(StorageError)?;
        Self::new_for_replay(run)
    }

    /// Replay already carries authoritative policy; a missing balance marker is legacy,
    /// and is omitted again when serializing that historic checkpoint.
    pub(crate) fn new_for_replay(mut run: Run) -> Result<Self> {
        if run.replay_snapshot_from.as_ref().is_some_and(|&n|n>run.replay.len()) {
            return Err(StorageError("Replay policy boundary exceeds command ledger".into()));
        }
        // Old plain checkpoints contain legacy command prefixes. Future commands
        // use snapshot semantics; explicit old-journal replay overrides this below.
        if run.replay_snapshot_from.is_missing() {run.replay_snapshot_from=Field::Value(run.replay.len());}
        crate::amulet_rules::admit_current(&mut run).map_err(StorageError)?;
        crate::potion_rules::admit_current(&mut run).map_err(StorageError)?;
        crate::ring_rules::admit_current(&mut run).map_err(StorageError)?;
        crate::gas_rules::admit_current(&mut run).map_err(StorageError)?;
        crate::commandment_rules::admit_current(&mut run).map_err(StorageError)?;
        crate::screamer_rules::admit_current(&mut run).map_err(StorageError)?;
        crate::ember_rules::admit_current(&mut run).map_err(StorageError)?;
        crate::ragelight_rules::admit_current(&mut run).map_err(StorageError)?;
        crate::balance_rules::validate(&run).map_err(StorageError)?;
        crate::steam_rules::admit_current(&mut run).map_err(StorageError)?;
        crate::execute_rules::admit_current(&mut run).map_err(StorageError)?;
        crate::beckoning_rules::admit_current(&mut run).map_err(StorageError)?;
        crate::ichor_fire_rules::admit_current(&mut run).map_err(StorageError)?;
        crate::future_floor::admit_current(&mut run).map_err(StorageError)?;
        crate::firebolt_travel::admit_current(&mut run).map_err(StorageError)?;
        storage::validate(&run)?;
        storage::validate_current_revision(&run)?;
        crate::revision52_reserves::normalize_run(&mut run).map_err(StorageError)?;
        crate::unique_composite_bow::initialize(&mut run);
        crate::hanged::normalize_world(&mut run);
        let mut history = History::import(&run)?;
        history.full_snapshot = true;
        if !retains_revert_history(&run) {
            let revision=run.simulation_revision.clone();
            if let Some(anchor)=run.future.as_mut(){anchor.promote41(&revision);}
        }
        crate::future_floor::validate_anchor(&run).map_err(StorageError)?;
        crate::future_floor::ensure_entry(&mut run).map_err(StorageError)?;
        Ok(Self { run, history })
    }
    /// O(1) policy selection, clearing at most five encoded cache handles.
    /// Used only to replay old persisted command tails under their original rules.
    pub fn set_legacy_revert(&mut self, legacy: bool) {
        if !retains_revert_history(&self.run) { return; }
        self.history.full_snapshot = !legacy;
        if legacy {self.run.replay_snapshot_from=Field::Missing;}
        else if self.run.replay_snapshot_from.is_missing() {self.run.replay_snapshot_from=Field::Value(self.run.replay.len());}
        self.history.exported = Default::default();
    }
    /// Replay-only checkpoint admission preserves explicit legacy policy markers.
    pub fn load_replay_checkpoint(raw: &str) -> Result<Self> {
        Self::new_for_replay(storage::deserialize_compatible(raw)?)
    }
    pub fn load(raw: &str) -> Result<Self> {
        let mut run = storage::deserialize_compatible(raw)?;
        // Loading for live play adopts current rules even from an old playback checkpoint.
        if matches!(run.amulet_rules_from, Field::Null) { run.amulet_rules_from = Field::Missing; }
        if matches!(run.potion_rules_from, Field::Null) { run.potion_rules_from = Field::Missing; }
        if matches!(run.ring_rules_from, Field::Null) { run.ring_rules_from = Field::Missing; }
        if matches!(run.gas_rules_from, Field::Null) { run.gas_rules_from = Field::Missing; }
        if matches!(run.commandment_rules_from, Field::Null) { run.commandment_rules_from = Field::Missing; }
        if matches!(run.screamer_rules_from, Field::Null) { run.screamer_rules_from = Field::Missing; }
        if matches!(run.ember_rules_from, Field::Null) { run.ember_rules_from = Field::Missing; }
        if matches!(run.ragelight_rules_from, Field::Null) { run.ragelight_rules_from = Field::Missing; }
        if matches!(run.balance_rules_from, Field::Null) { run.balance_rules_from = Field::Missing; }
        if matches!(run.collapse_rules_from, Field::Null) { run.collapse_rules_from = Field::Missing; }
        if matches!(run.blood_mist_fire_rules_from, Field::Null) { run.blood_mist_fire_rules_from = Field::Missing; }
        if matches!(run.half_here_lock_from, Field::Null) { run.half_here_lock_from = Field::Missing; }
        if matches!(run.half_here_energy_from, Field::Null) { run.half_here_energy_from = Field::Missing; }
        if matches!(run.screamer_range_from, Field::Null) { run.screamer_range_from = Field::Missing; }
        if matches!(run.death_gas_rules_from, Field::Null) { run.death_gas_rules_from = Field::Missing; }
        if matches!(run.minotaur_bump_from, Field::Null) { run.minotaur_bump_from = Field::Missing; }
        if matches!(run.borge_throw_from, Field::Null) { run.borge_throw_from = Field::Missing; }
        if matches!(run.steam_rules_from, Field::Null) { run.steam_rules_from = Field::Missing; }
        if matches!(run.execute_rules_from, Field::Null) { run.execute_rules_from = Field::Missing; }
        if matches!(run.beckoning_rules_from, Field::Null) { run.beckoning_rules_from = Field::Missing; }
        if matches!(run.ichor_fire_rules_from, Field::Null) { run.ichor_fire_rules_from = Field::Missing; }
        if matches!(run.future_floor_rules_from, Field::Null) { run.future_floor_rules_from = Field::Missing; }
        Self::new(run)
    }
    /// New41 borrows current state after O(F+L) trail/retired-history validation.
    /// Legacy owns its exported history; no new41 Run/trail clone is performed.
    pub fn checkpoint_state(&self) -> Result<std::borrow::Cow<'_,Run>> {
        if !retains_revert_history(&self.run) {
            validate_retired_history(&self.run)?;
            crate::timeweave::validate_run(&self.run).map_err(StorageError)?;
            Ok(std::borrow::Cow::Borrowed(&self.run))
        } else { Ok(std::borrow::Cow::Owned(self.history.export(&self.run)?)) }
    }
    pub fn save(&self) -> Result<String> {
    let _timing_function = crate::timing::span("storage.engine_save");
        storage::serialize(self.checkpoint_state()?.as_ref())
    }
    pub fn canonical_run(&self) -> Result<Run> {
        self.history.export(&self.run)
    }
    pub fn dispatch(
        &mut self,
        command: &crate::Command,
    ) -> std::result::Result<crate::engine::DispatchOutcome, crate::engine::EngineError> {
    let _timing_function = crate::timing::span("command.temporal_dispatch");
        let err = |e: StorageError| crate::engine::EngineError(e.to_string());
        if !retains_revert_history(&self.run) && matches!(command, crate::Command::Ability { id, .. } if id == "numo-revert") {
            return Err(crate::engine::EngineError("Revert is unavailable under revision41 rules".into()));
        }
        if !crate::future_floor::enabled(&self.run) && self.run.status == "playing" && self.run.future.is_none()
            && matches!(command, crate::Command::Ability { id, .. }
                if id == "morae-see-the-future" && self.run.player.abilities.contains(id))
        {
            validate_future_capture(&self.run).map_err(err)?;
        }
        crate::future_floor::ensure_entry(&mut self.run).map_err(crate::engine::EngineError)?;
        if crate::future_floor::enabled(&self.run) && matches!(command, crate::Command::Ability{id,..} if id==crate::future_floor::ID) {
            if self.run.status!="playing" || !self.run.offers.is_empty() || crate::debug_floor::preparing(&self.run)
                || crate::engine::condition(&self.run.player,"paralyzed")>0. {return Ok(Default::default());}
            crate::engine::validate_supported(&self.run)?;
            let mut candidate=self.run.clone();candidate.replay.push(command.clone());
            crate::events::begin();
            let restored=crate::future_floor::restore(&mut candidate).map_err(crate::engine::EngineError)?;
            if restored {let id=candidate.player.id;crate::dreamwalking::with_actor(&mut candidate,id,|r|crate::animation_shapes::nova(r,&r.player,"P71","nova-pink-large",6));}
            let events=crate::events::finish();self.run=candidate;
            return Ok(crate::engine::DispatchOutcome{changed:restored,events,..Default::default()});
        }
        self.history.sync_debug_loadout(&self.run);
        // No temporal escape from the debug reconstruction/choice boundary.
        if crate::debug_floor::preparing(&self.run) && !matches!(command, crate::Command::Choose { .. }) {
            return crate::engine::dispatch(&mut self.run, command);
        }
        if let crate::Command::Ability { id, target, x, y } = command {
            if id == "numo-revert" || id == "morae-see-the-future" {
                if self.run.status != "playing"
                    || self
                        .run
                        .player
                        .conditions
                        .get("deathResolutionBlocked")
                        .copied()
                        .unwrap_or(0.0)
                        != 0.0
                {
                    return Ok(Default::default());
                }
                // New41 restore admission occurs before command bookkeeping. Do not
                // consume live replay or publish a failed anchor's command prefix.
                let mut prepared_future41 = if !retains_revert_history(&self.run)
                    && id == "morae-see-the-future" && self.run.future.is_some()
                    && self.run.player.abilities.contains(id) && self.run.offers.is_empty()
                    && self.run.player.conditions.get("paralyzed").copied().unwrap_or(0.0)<=0.0 {
                        Some(prepare_future41(&self.run).map_err(err)?)
                    } else {None};
                // Validate outside the temporal ownership hooks; other unported state must not execute.
                let mut probe = self.run.clone();
                probe
                    .player
                    .abilities
                    .retain(|x| x != "numo-revert" && x != "morae-see-the-future");
                probe.player.conditions.shift_remove("revertCharges");
                probe.player.conditions.shift_remove("acquired:numo-revert");
                probe
                    .player
                    .conditions
                    .shift_remove("acquired:morae-see-the-future");
                crate::engine::validate_supported(&probe)?;
                *self.run.metrics.entry("commands".into()).or_default() += 1.0;
                self.run.replay.push(command.clone());
                crate::resources::observe_resource_history(&mut self.run);
                if !self.run.offers.is_empty() {
                    crate::engine::log(
                        &mut self.run,
                        "Choose your level-up ability first.",
                        "player",
                    );
                    crate::equipment::sync_ownership(&mut self.run)?;
                    crate::abilities::disable_unfunded(&mut self.run)?;
                    crate::engine::settle_doors(&mut self.run);
                    self.history.rebase_temporal(&self.run).map_err(err)?;
                    return Ok(crate::engine::DispatchOutcome {
                        pending_snapshot: None,
                        events: vec![],
                        changed: true,
                        took_turn: false,
                        used_item: false,
                    });
                }
                if self
                    .run
                    .player
                    .conditions
                    .get("paralyzed")
                    .copied()
                    .unwrap_or(0.0)
                    > 0.0
                {
                    crate::engine::log(
                        &mut self.run,
                        "You are paralyzed. Wait to advance time.",
                        "player",
                    );
                    crate::equipment::sync_ownership(&mut self.run)?;
                    crate::abilities::disable_unfunded(&mut self.run)?;
                    crate::engine::settle_doors(&mut self.run);
                    self.history.rebase_temporal(&self.run).map_err(err)?;
                    return Ok(crate::engine::DispatchOutcome {
                        events: vec![],
                        pending_snapshot: None,
                        changed: true,
                        took_turn: false,
                        used_item: false,
                    });
                }
                let signature = |r: &Run| {
                    serde_json::json!([
                        r.tick,
                        r.depth,
                        r.player,
                        r.inventory,
                        r.actors_at(r.depth)
                            .iter()
                            .find(|a| target.as_ref().is_some_and(|t| *t == a.id)),
                        r.actors_at(r.depth).len()
                    ])
                };
                let before_signature = signature(&self.run);
                let future_was_active=self.run.future.is_some();
                crate::events::begin();
                if self.run.player.abilities.contains(id) {
                    if (id == "numo-revert"
                        || (self.run.future.is_none() && future_spends_energy(&self.run)))
                        && crate::abilities::energy_activation_silenced(&self.run, &self.run.player)
                    {
                        crate::engine::log(
                            &mut self.run,
                            "Silence prevents this energy-paid activation.",
                            "player",
                        );
                    } else if id == "numo-revert" {
                        if !revert(&mut self.run, &mut self.history).map_err(err)? {
                            crate::engine::log(
                                &mut self.run,
                                "Revert needs a prior turn, one charge and 2 energy.",
                                "player",
                            );
                        }
                    } else if self.run.future.is_some() {
                        if let Some((restored,next_history))=prepared_future41.take() {
                            commit_future41(&mut self.run,&mut self.history,restored,next_history);
                        } else {restore_future(&mut self.run, &mut self.history).map_err(err)?;}
                    } else {
                        remember_future(&mut self.run, &self.history).map_err(err)?;
                    }
                }
                if crate::events::enabled(&self.run) && id=="morae-see-the-future" && future_was_active!=self.run.future.is_some() {
                    let player_id=self.run.player.id;
                    crate::dreamwalking::with_actor(&mut self.run,player_id,|r|crate::animation_shapes::nova(r,&r.player,"P71","nova-pink-large",6));
                }
                if signature(&self.run) != before_signature {
                    let label = if id == "numo-revert" {
                        "Revert"
                    } else {
                        "See The Future"
                    };
                    crate::engine::log(&mut self.run, &format!("You use {label}."), "player");
                    let point = self
                        .run
                        .actors_at(self.run.depth)
                        .iter()
                        .find(|a| target.as_ref().is_some_and(|t| *t == a.id))
                        .map(crate::events::EventPoint::from)
                        .or_else(|| match (x.as_ref(), y.as_ref()) {
                            (Some(x), Some(y)) => Some(crate::events::EventPoint { x: *x, y: *y }),
                            _ => None,
                        });
                    if !crate::events::enabled(&self.run) || id!="morae-see-the-future" { crate::events::actor_event(
                        &self.run,
                        &self.run.player,
                        "pulse",
                        label,
                        point,
                        None,
                    ); }
                }
                crate::equipment::sync_ownership(&mut self.run)?;
                crate::abilities::disable_unfunded(&mut self.run)?;
                crate::engine::settle_doors(&mut self.run);
                self.history.rebase_temporal(&self.run).map_err(err)?;
                return Ok(crate::engine::DispatchOutcome {
                    events: crate::events::finish(),
                    pending_snapshot: None,
                    changed: true,
                    took_turn: false,
                    used_item: false,
                });
            }
        }
        if self.run.status == "playing" && crate::projectiles::blocker(&self.run, command).is_some()
        {
            return crate::engine::dispatch(&mut self.run, command);
        }
        // Revision41 has no completed/pending rewind state, journal scope or closing capture.
        if !retains_revert_history(&self.run) {
            let mut out=crate::engine::dispatch(&mut self.run, command)?;
            if crate::future_floor::enabled(&self.run) && self.run.status=="dead" && crate::future_floor::available(&self.run) {
                crate::events::begin();
                crate::future_floor::restore(&mut self.run).map_err(crate::engine::EngineError)?;
                let id=self.run.player.id;crate::dreamwalking::with_actor(&mut self.run,id,|r|crate::animation_shapes::nova(r,&r.player,"P71","nova-pink-large",6));
                out.events=crate::events::finish();out.pending_snapshot=None;out.changed=true;
            }
            crate::future_floor::ensure_entry(&mut self.run).map_err(crate::engine::EngineError)?;
            return Ok(out);
        }
        let mut before = Snapshot::capture(&self.run);
        before.incoming = self.history.closing.as_ref().and_then(|s| s.incoming.clone());
        let writes_scope = (!self.history.full_snapshot).then(crate::mutation_journal::begin);
        let outcome = crate::engine::dispatch(&mut self.run, command)?;
        let writes = writes_scope.map(crate::mutation_journal::Scope::finish).unwrap_or_default();
        if self.run.tick != before.state.tick {
            if let Some(closing) = &self.history.closing {
                // Compare the immutable pre-command capture only when it will become
                // a completed opening. Free commands never pay this metadata guard.
                // There is no benefit with a pending opening or no preceding seal.
                // Release the cache lock before comparing metadata; no lock
                // spans serialization or simulation work.
                let sealed = self.history.pending_haste.is_none()
                    && self.history.frames.back().is_some_and(|older| {
                        self.history.exported.lock().unwrap_or_else(|e| e.into_inner())
                            .iter().any(|edge| edge.matches(older, closing))
                    });
                if sealed && closing.same_boundary(&before).map_err(err)? {
                    let _reuse = crate::timing::span("history.opening_identity_reuse");
                    // Preserve the previously sealed edge endpoint only after exact
                    // projected equality, never merely because dispatch was last writer.
                    before = closing.clone();
                }
            }
            let opening = self.history.pending_haste.take().unwrap_or(before);
            let mut incoming = std::mem::take(&mut self.history.pending_writes);
            incoming.merge(&writes);
            incoming.retain_opening(&opening.state);
            self.history.record_completed(opening);
            let mut closing = Snapshot::capture(&self.run);
            closing.incoming = Some(Arc::new(incoming));
            self.history.closing = Some(closing);
        } else if self
            .run
            .haste_snapshot
            .as_ref()
            .is_some_and(|raw| !raw.is_empty())
            || self.history.pending_haste.is_some()
            || self.run.phase.as_ref().is_some_and(|p| p == "normal")
        {
            self.history.pending_writes.merge(&writes);
            if self.history.pending_haste.is_none() {
                let incoming = before.incoming.clone();
                self.history.pending_haste =
                    Some(if let Some(snapshot) = outcome.pending_snapshot.clone() {
                        snapshot
                    } else if let Some(raw) = self
                        .run
                        .haste_snapshot
                        .as_ref()
                        .filter(|raw| !raw.is_empty())
                    {
                        Snapshot::from_value(
                            serde_json::from_str(raw)
                                .map_err(|e| crate::engine::EngineError(e.to_string()))?,
                        )
                        .map_err(err)?
                    } else {
                        before
                    });
                self.history.pending_haste.as_mut().unwrap().incoming = incoming;
            }
            self.history.pending_writes.retain_opening(&self.history.pending_haste.as_ref().unwrap().state);
        } else {
            let mut incoming = self.history.closing.as_ref().and_then(|s| s.incoming.as_ref())
                .map(|j| (**j).clone()).unwrap_or_default();
            incoming.merge(&writes);
            if let Some(opening) = self.history.frames.back() { incoming.retain_opening(&opening.state); }
            else { incoming = Default::default(); }
            self.history.record_free(before);
            let mut closing = Snapshot::capture(&self.run);
            closing.incoming = Some(Arc::new(incoming));
            self.history.closing = Some(closing);
        }
        if !self.history.full_snapshot { self.history.seal_current().map_err(err)?; }
        Ok(outcome)
    }
}

#[cfg(test)]
mod legacy_undefined_tests {
    use super::*;
    #[test]
    fn missing_patch_value_restores_undefined_not_null() {
        let mut state = json!({"actor":{"baseMaxHp":100,"nullable":7},"array":[9]});
        let patches: Vec<Patch> = serde_json::from_value(json!([
            {"path":["actor","baseMaxHp"]},
            {"path":["actor","nullable"],"value":null},
            {"path":["array",0]}
        ]))
        .unwrap();
        apply(&mut state, &patches).unwrap();
        assert!(state["actor"].get("baseMaxHp").is_none());
        assert_eq!(state["actor"].get("nullable"), Some(&Value::Null));
        assert_eq!(state["array"], json!([null]));
    }
}

#[cfg(test)]
mod export_reuse_tests {
    use super::*;
    // These frozen selective-export tests certify the retained v19 replay policy.
    // New whole-snapshot semantics have separate preservation/roundtrip tests.
    fn legacy_load(raw: &str) -> Result<Engine> { let mut e=Engine::load(raw)?;e.set_legacy_revert(true);Ok(e) }
    fn legacy_new(run: Run) -> Result<Engine> { let mut e=Engine::new(run)?;e.set_legacy_revert(true);Ok(e) }


    /// Frozen pre-optimization export oracle, O(H*S + patch construction).
    fn legacy_export(history: &History, run: &Run) -> Result<Run> {
        let mut out = run.clone();
        let mut newer = history.pending_haste.clone()
            .or_else(|| if history.debug_loadout_changed(run) {
                Some(Snapshot::capture(run))
            } else { history.closing.clone() })
            .unwrap_or_else(|| Snapshot::capture(run));
        let mut frames = Vec::new();
        for older in history.frames.iter().rev() {
            let mut a = older.value()?;
            let mut b = newer.value()?;
            a.as_object_mut().unwrap().shift_remove("logArchiveCursor");
            b.as_object_mut().unwrap().shift_remove("logArchiveCursor");
            let patches = inverse_values(&a, &b, older.cursor, newer.cursor, true);
            frames.push(js_stringify(&serde_json::to_value(Frame {
                groups: vec![patches], future: crate::future_anchor::wire_option(&older.state.future)?,
            })?));
            newer = older.clone();
        }
        frames.reverse();
        out.history = frames;
        if let Some(p) = &history.pending_haste {
            out.haste_snapshot = Field::Value(js_stringify(&p.value()?));
        }
        Ok(out)
    }

    /// O(H*S + output bytes), including deliberately inconsistent public snapshots.
    fn assert_export_bytes(engine: &Engine) {
        let expected = storage::serialize(&legacy_export(&engine.history, &engine.run).unwrap()).unwrap();
        assert!(engine.save().unwrap() == expected, "export differs from legacy bytes");
    }

    /// O(H*S + output bytes); exact bytes include embedded patches and key order.
    fn assert_exact(engine: &Engine) {
        let expected = storage::serialize(&legacy_export(&engine.history, &engine.run).unwrap()).unwrap();
        assert!(engine.save().unwrap() == expected, "export differs from legacy bytes");
        let loaded = legacy_load(&expected).unwrap();
        assert!(loaded.save().unwrap() == engine.save().unwrap(), "valid timeline reload differs");
    }

    /// O(R + M) per guard plus fixed fixture construction; checks all guard classes.
    #[test]
    fn opening_guard_checks_external_headers_roots_order_dream_and_cursors() {
        let mut run = Run::default();
        run.floors = vec![Arc::new(crate::Floor::default()), Arc::new(crate::Floor::default())];
        run.actors.insert("1".into(), Arc::new(vec![]));
        run.actors.insert("2".into(), Arc::new(vec![]));
        run.ground.insert("1".into(), Arc::new(vec![]));
        run.player.conditions.insert("first".into(), 1.);
        run.player.conditions.insert("second".into(), 2.);
        let old = Snapshot::capture(&run);
        assert!(old.same_boundary(&Snapshot::capture(&run)).unwrap());
        let mut edited = run.clone(); edited.player.energy = 1.;
        assert!(!old.same_boundary(&Snapshot::capture(&edited)).unwrap());
        let mut with_future = run.clone(); with_future.future = Some("anchor\"bytes".repeat(32).into());
        let future = Snapshot::capture(&with_future);
        assert!(future.same_boundary(&Snapshot::capture(&with_future)).unwrap());
        assert!(!old.same_boundary(&future).unwrap());
        let mut changed=with_future.future.as_ref().unwrap().wire_text().unwrap().to_owned();changed.push('x');with_future.future=Some(changed.into());
        assert!(!future.same_boundary(&Snapshot::capture(&with_future)).unwrap());
        with_future.future = Some(String::new().into());
        assert!(!old.same_boundary(&Snapshot::capture(&with_future)).unwrap());
        let mut edited = run.clone(); edited.tick = -0.;
        assert!(!old.same_boundary(&Snapshot::capture(&edited)).unwrap());
        let mut edited = run.clone();
        let value = edited.player.conditions.shift_remove("first").unwrap();
        edited.player.conditions.insert("first".into(), value);
        assert!(!old.same_boundary(&Snapshot::capture(&edited)).unwrap());
        let mut edited = run.clone();
        let value = edited.actors.shift_remove("1").unwrap(); edited.actors.insert("1".into(), value);
        assert!(!old.same_boundary(&Snapshot::capture(&edited)).unwrap());
        let mut edited = run.clone(); edited.floors.swap(0,1);
        assert!(!old.same_boundary(&Snapshot::capture(&edited)).unwrap());
        let mut edited = run.clone(); edited.floors[0] = Arc::new((*edited.floors[0]).clone());
        assert!(!old.same_boundary(&Snapshot::capture(&edited)).unwrap(), "equal values with new identities conservatively miss");
        let mut edited = run.clone(); Arc::make_mut(edited.ground.get_mut("1").unwrap()).push(crate::GroundItem::default());
        assert!(!old.same_boundary(&Snapshot::capture(&edited)).unwrap());
        let mut cursor = Snapshot::capture(&run); cursor.cursor = Some(ArchiveCursor {messages:0,events:0});
        assert!(!old.same_boundary(&cursor).unwrap());
        run.dreamwalking = crate::dreamwalking::State { floors:run.floors.clone(), ..Default::default() }.into();
        let dream = Snapshot::capture(&run);
        assert!(dream.same_boundary(&Snapshot::capture(&run)).unwrap());
        let mut edited=run.clone(); edited.dreamwalking.as_mut().unwrap().floors.swap(0,1);
        assert!(!dream.same_boundary(&Snapshot::capture(&edited)).unwrap());
        let mut edited=run.clone(); edited.dreamwalking.as_mut().unwrap().active=true;
        assert!(!dream.same_boundary(&Snapshot::capture(&edited)).unwrap());
        let mut edited=run.clone(); edited.dreamwalking=Field::Null;
        assert!(!dream.same_boundary(&Snapshot::capture(&edited)).unwrap());
    }

    /// O(commands * (dispatch + H*S)), proving actual cache identity/retention and fallback.
    #[test]
    fn next_completed_turn_retains_prior_sealed_edge_but_external_edits_miss() {
        let mut engine = legacy_load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        for _ in 0..5 { engine.dispatch(&crate::Command::Wait).unwrap(); }
        assert_exact(&engine);
        let older=engine.history.frames.back().unwrap().clone();
        let closing=engine.history.closing.as_ref().unwrap().clone();
        assert!(engine.history.exported.lock().unwrap().iter().any(|e|e.matches(&older,&closing)));
        engine.dispatch(&crate::Command::Wait).unwrap();
        let opening=engine.history.frames.back().unwrap();
        assert!(Arc::ptr_eq(&opening.state,&closing.state));
        assert!(engine.history.exported.lock().unwrap().iter().any(|e|e.matches(&older,opening)),
            "new seal must not evict the preceding still-retained edge");
        {
            let cache=engine.history.exported.lock().unwrap();
            let mut newer=engine.history.closing.as_ref().unwrap();
            for frame in engine.history.frames.iter().rev() {
                assert!(cache.iter().any(|edge|edge.matches(frame,newer)),
                    "all five retained edges should be cached before exporting");
                newer=frame;
            }
        }
        assert_exact(&engine);
        engine.dispatch(&crate::Command::Pickup).unwrap();
        let free_closing=engine.history.closing.as_ref().unwrap().clone();
        engine.dispatch(&crate::Command::Wait).unwrap();
        assert!(Arc::ptr_eq(&engine.history.frames.back().unwrap().state,&free_closing.state));
        assert_exact(&engine);
        let closing=engine.history.closing.as_ref().unwrap().clone();
        engine.run.player.energy -= 0.5;
        engine.dispatch(&crate::Command::Wait).unwrap();
        assert!(!Arc::ptr_eq(&engine.history.frames.back().unwrap().state,&closing.state));
        assert_exact(&engine);
    }

    #[test]
    fn reused_endpoints_match_legacy_export_for_zero_to_five_frames_and_reload() {
        let mut engine = legacy_load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        assert_exact(&engine);
        for _ in 0..7 {
            engine.dispatch(&crate::Command::Wait).unwrap();
            assert_exact(&engine);
            engine.dispatch(&crate::Command::Pickup).unwrap();
            assert_exact(&engine);
        }
        assert_eq!(engine.history.frames.len(), 5);
        // Repeated exports and changes after an export must never reuse stale JSON.
        engine.run.player.energy -= 0.25;
        engine.history.closing = None;
        assert_exact(&engine);
        engine.history.pending_haste = Some(Snapshot::capture(&engine.run));
        assert_exact(&engine);
    }

    #[test]
    fn export_cache_misses_on_public_endpoint_cursor_and_cow_mutations() {
        let mut engine = legacy_load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        for _ in 0..5 { engine.dispatch(&crate::Command::Wait).unwrap(); }
        assert_export_bytes(&engine);
        assert_eq!(engine.history.exported.lock().unwrap().len(), 5);
        // Single-strong-owner Arc::make_mut must dissociate its cached Weak key.
        let frame = engine.history.frames.front_mut().unwrap();
        Arc::make_mut(&mut frame.state).player.energy -= 0.5;
        assert_export_bytes(&engine);
        // Archive metadata is outside Run and therefore explicitly part of key.
        let frame = engine.history.frames.front_mut().unwrap();
        frame.cursor = None;
        assert_export_bytes(&engine);
        // A cloned timeline shares the bounded cache but not a key for changed state.
        let mut cloned = engine.clone();
        let closing = cloned.history.closing.as_mut().unwrap();
        Arc::make_mut(&mut closing.state).player.hp -= 0.25;
        assert_export_bytes(&cloned);
        assert_export_bytes(&engine);
        engine.history.frames.pop_front();
        assert_export_bytes(&engine);
        assert!(engine.history.exported.lock().unwrap().len() <= 5);
    }

    /// O(trace steps * (dispatch + complete save bytes)); hit versus forced-miss oracle.
    #[test]
    fn recorded_revert_matches_uncached_inverse_for_all_temporal_traces() {
        let cases: Value = serde_json::from_str(include_str!("../../../fixtures/temporal-command-traces-v48.json")).unwrap();
        for case in cases.as_array().unwrap() {
            let mut cached = legacy_new(serde_json::from_value(case["initial"].clone()).unwrap()).unwrap();
            let mut uncached = cached.clone();
            uncached.history.exported = Default::default();
            for step in case["steps"].as_array().unwrap() {
                let command: crate::Command = serde_json::from_value(step["command"].clone()).unwrap();
                cached.save().unwrap();
                uncached.history.exported.lock().unwrap().clear();
                let a = cached.dispatch(&command).unwrap();
                let b = uncached.dispatch(&command).unwrap();
                assert_eq!(serde_json::to_value(a.events).unwrap(), serde_json::to_value(b.events).unwrap());
                assert!(cached.save().unwrap() == uncached.save().unwrap(), "cached inverse changed {}", case["name"]);
                if step["resume"] == true {
                    cached = legacy_load(&cached.save().unwrap()).unwrap();
                    uncached = legacy_load(&uncached.save().unwrap()).unwrap();
                }
            }
        }
    }

    /// O(cases * snapshot/patch bytes); embedded JS numbers must restore exact typed values.
    #[test]
    fn recorded_inverse_number_spellings_and_cursor_misses_are_exact() {
        let source = legacy_load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap().run;
        for value in [-0.0, 1.0, 1.0000000000000002, 9007199254740992.0, 1e100, 1e-100] {
            let mut before_run = source.clone();
            before_run.player.energy = value;
            let before = Snapshot::capture(&before_run);
            let mut after_run = before_run.clone(); after_run.player.energy = 42.;
            let after = Snapshot::capture(&after_run);
            let mut history = History::default();
            history.frames.push_back(before.clone()); history.closing = Some(after.clone());
            history.seal_current().unwrap();
            let cached = history.recorded_inverse(&before, &after).unwrap();
            let direct = inverse(&before, &after, true).unwrap();
            let mut a = serde_json::to_value(&after_run).unwrap(); let mut b = a.clone();
            apply(&mut a, &cached).unwrap(); apply(&mut b, &direct).unwrap();
            let a: Run = serde_json::from_value(a).unwrap(); let b: Run = serde_json::from_value(b).unwrap();
            assert_eq!(a.player.energy.to_bits(), b.player.energy.to_bits());
            assert_eq!(storage::serialize(&a).unwrap(), storage::serialize(&b).unwrap());
            let mut changed_cursor = after.clone(); changed_cursor.cursor = None;
            let fallback = history.recorded_inverse(&before, &changed_cursor).unwrap();
            assert_eq!(serde_json::to_value(fallback).unwrap(), serde_json::to_value(inverse(&before, &changed_cursor, true).unwrap()).unwrap());
        }
    }

    /// O(workers * exports * (S*L + patch bytes)); concurrency correctness, not timing.
    #[test]
    fn shared_export_cache_concurrent_divergent_clones_are_exact() {
        let mut engine = legacy_load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        for _ in 0..5 { engine.dispatch(&crate::Command::Wait).unwrap(); }
        assert_export_bytes(&engine);
        let barrier = Arc::new(std::sync::Barrier::new(4));
        let mut threads = Vec::new();
        for i in 0..4 {
            let mut fork = engine.clone();
            let barrier = barrier.clone();
            Arc::make_mut(&mut fork.history.frames.front_mut().unwrap().state).player.energy -= i as f64;
            threads.push(std::thread::spawn(move || {
                barrier.wait();
                for _ in 0..3 { assert_export_bytes(&fork); }
            }));
        }
        for thread in threads { thread.join().unwrap(); }
        assert_export_bytes(&engine);
        assert!(engine.history.exported.lock().unwrap().len() <= 5);
    }

    /// O(generation + turns + exports), exercising real zero-turn debug loadout hooks.
    #[test]
    fn warm_export_cache_detects_debug_loadout_revision_changes() {
        let mut run = crate::debug_floor::create_run("save-cache-debug", "Unblessed", 1).unwrap();
        crate::debug_floor::enter(&mut run).unwrap();
        let mut engine = legacy_new(run).unwrap();
        engine.dispatch(&crate::Command::Wait).unwrap();
        assert_exact(&engine);
        let id = engine.run.inventory.iter().find(|i| i.id <= 3.).unwrap().id;
        assert!(crate::debug_floor::put(&mut engine.run, id).unwrap());
        assert!(engine.history.debug_loadout_changed(&engine.run));
        assert_exact(&engine);
        assert!(crate::debug_floor::take(&mut engine.run, id).unwrap());
        assert_exact(&engine);
        engine.dispatch(&crate::Command::Wait).unwrap();
        assert_exact(&engine);
    }

    /// O(exports * (S*L + patch bytes)); synthetic metadata variants need not reload.
    #[test]
    fn warm_export_cache_checks_both_archive_cursor_keys() {
        let mut engine = legacy_load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        engine.dispatch(&crate::Command::Wait).unwrap();
        for cursor in [None, Some(ArchiveCursor {messages: 0, events: 0}),
            Some(ArchiveCursor {messages: 1, events: 1}), None] {
            engine.history.frames.front_mut().unwrap().cursor = cursor;
            assert_export_bytes(&engine);
            engine.history.closing.as_mut().unwrap().cursor = cursor;
            assert_export_bytes(&engine);
        }
    }

    #[test]
    fn reused_endpoints_keep_all_legacy_patches_in_temporal_reference_traces() {
        let cases: Value = serde_json::from_str(include_str!("../../../fixtures/temporal-command-traces-v48.json")).unwrap();
        for case in cases.as_array().unwrap() {
            let run: Run = serde_json::from_value(case["initial"].clone()).unwrap();
            let mut engine = legacy_new(run).unwrap();
            assert_exact(&engine);
            for step in case["steps"].as_array().unwrap() {
                let command: crate::Command = serde_json::from_value(step["command"].clone()).unwrap();
                engine.dispatch(&command).unwrap();
                assert_exact(&engine);
                if step["resume"] == true { engine = legacy_load(&engine.save().unwrap()).unwrap(); }
            }
        }
    }
}

#[cfg(test)]
#[path = "timeweave_history_tests.rs"]
mod timeweave_history_tests;
