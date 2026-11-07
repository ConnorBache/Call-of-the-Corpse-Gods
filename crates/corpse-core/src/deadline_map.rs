//! Ordered sparse deadlines with a mutation-invalidated scheduling index.
//!
//! Serialized state is exactly the existing `IndexMap<String, f64>` object. The
//! scheduler is derived state: clone may preserve it; load rebuilds it lazily.
//! Every mutable map borrow invalidates it before writes become possible. Thus
//! direct public-field writes through `insert`, `entry`, `get_mut`, indexing,
//! `values_mut`, or `retain` cannot leave an unnoticed stale deadline.
//!
//! B = deadline entries, D = distinct deadlines, K = due entries, Z = total key
//! bytes. An unchanged `has_due` is O(1) apart from an uncontended mutex lock.
//! The first check after arbitrary mutations rebuilds in O(B log B + Z) time
//! and O(B + Z) storage. Due processing costs O(K log B + B + dropped key bytes),
//! preserving IndexMap retention order. It never scans B entries on an unchanged
//! no-expiry check. Lock contention is an external scheduling cost, not claimed
//! constant wall time. The mutex preserves Send/Sync for shared immutable Run
//! observations; no simulation or wall-clock decision depends on it.
use crate::turn_schedule::{Deadline, EventKey, Schedule};
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::sync::{Mutex, MutexGuard};

#[derive(Debug, Clone, Default)]
struct Cache {
    valid: bool,
    queue: Schedule<String>,
    next: Option<u64>,
    nan_count: usize,
}

/// Drop-in ordered-map data with a strictly derived schedule. Equality and serde
/// intentionally ignore cache state, so reads do not alter canonical snapshots.
pub struct DeadlineMap {
    values: IndexMap<String, f64>,
    cache: Mutex<Cache>,
}

impl Default for DeadlineMap {
    /// O(1) time/space. Empty map has an already-valid empty schedule.
    fn default() -> Self {
        Self {
            values: IndexMap::new(),
            cache: Mutex::new(Cache { valid: true, ..Cache::default() }),
        }
    }
}

impl Clone for DeadlineMap {
    /// O(B + Z) map copy plus O(B + D + Z) when the schedule is valid. The clone
    /// has its own mutex/cache; mutation and expiration cannot affect its source.
    fn clone(&self) -> Self {
        let cache = self.lock();
        Self {
            values: self.values.clone(),
            cache: Mutex::new(if cache.valid { cache.clone() } else { Cache::default() }),
        }
    }
}

impl PartialEq for DeadlineMap {
    /// Expected O(B) map comparisons and key-hash costs, O(1) scratch; derived
    /// schedules are irrelevant. Retains IndexMap/f64 equality semantics.
    fn eq(&self, other: &Self) -> bool { self.values == other.values }
}

impl fmt::Debug for DeadlineMap {
    /// O(B + Z + number-formatting) time/output; no cache rebuild or inspection.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.values.fmt(f) }
}

impl Deref for DeadlineMap {
    type Target = IndexMap<String, f64>;
    /// O(1) time/space; subsequent map operation retains its own complexity.
    fn deref(&self) -> &Self::Target { &self.values }
}

impl DerefMut for DeadlineMap {
    /// O(1) invalidation plus uncontended mutex access. Subsequent map operation
    /// retains its own cost. Invalidation happens before returning a mutable
    /// reference, including no-op/failed writes, so no mutable API bypass exists.
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.cache.get_mut().unwrap_or_else(|e| e.into_inner()).valid = false;
        &mut self.values
    }
}

impl From<IndexMap<String, f64>> for DeadlineMap {
    /// O(1) ownership transfer. First scheduling query pays O(B log B + Z) build.
    fn from(values: IndexMap<String, f64>) -> Self {
        Self { values, cache: Mutex::new(Cache::default()) }
    }
}

impl FromIterator<(String, f64)> for DeadlineMap {
    /// Expected O(B + Z) construction, O(B + Z) storage; duplicate-key behavior
    /// and insertion order are exactly IndexMap's. Schedule remains lazy.
    fn from_iter<T: IntoIterator<Item = (String, f64)>>(iter: T) -> Self {
        IndexMap::from_iter(iter).into()
    }
}

impl Serialize for DeadlineMap {
    /// O(B + Z + numeric formatting) time/output, same serialized ordered object
    /// as IndexMap. No scheduler/cache fields or mutation are serialized.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.values.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DeadlineMap {
    /// Expected O(B + Z) decoding/storage; rebuild is deferred until a due check.
    /// The enclosing existing save validator retains deadline admission rules.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        IndexMap::<String, f64>::deserialize(deserializer).map(Self::from)
    }
}

impl DeadlineMap {
    /// O(1) on an unchanged map; O(B log B + Z) after mutable access. Like the
    /// original `.any(|expiry| expiry <= tick)`, NaN never satisfies this check.
    pub fn has_due(&self, tick: f64) -> bool {
        let Some(cutoff) = time_key(tick) else { return false };
        let mut cache = self.lock();
        ensure_cache(&self.values, &mut cache);
        cache.next.is_some_and(|next| next <= cutoff)
    }

    /// Remove exactly the entries rejected by `retain(|_, expiry| *expiry > tick)`.
    /// No-due unchanged finite map: O(1). Due map: O(K log B + B + dropped key
    /// bytes), O(1) additional scratch beyond queue-owned payloads. Initial dirty
    /// rebuild adds O(B log B + Z). One ordered retain avoids K separate O(B)
    /// IndexMap shifts. Expiration updates the queue directly, not via dirty
    /// invalidation, so the next unchanged tick has no rebuild cost.
    pub fn expire(&mut self, tick: f64) -> usize {
        let cache = self.cache.get_mut().unwrap_or_else(|e| e.into_inner());
        let Some(cutoff) = time_key(tick) else {
            // Original retention rejects every value when the comparison is NaN.
            let removed = self.values.len();
            self.values.clear();
            *cache = Cache { valid: true, ..Cache::default() };
            return removed;
        };
        ensure_cache(&self.values, cache);
        if cache.next.is_none_or(|next| next > cutoff) && cache.nan_count == 0 {
            return 0;
        }
        while cache.queue.pop_through(Deadline { tick: cutoff, phase: 0 }).is_some() {}
        let before = self.values.len();
        self.values.retain(|_, expiry| *expiry > tick);
        cache.nan_count = 0;
        cache.next = cache.queue.next_deadline().map(|deadline| deadline.tick);
        before - self.values.len()
    }

    /// O(1) uncontended locking. A prior panic cannot poison authoritative map
    /// data through this immutable cache lock; rebuild validity is set last.
    fn lock(&self) -> MutexGuard<'_, Cache> {
        self.cache.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// O(1) time/space, exact monotone encoding of all non-NaN f64 times. This queue's
/// integer coordinate is ordered floating-point time, not rounded world ticks.
/// Thus fractions, infinities, negatives and signed zero preserve `<=` semantics.
fn time_key(value: f64) -> Option<u64> {
    if value.is_nan() { return None; }
    let bits = if value == 0.0 { 0.0f64.to_bits() } else { value.to_bits() };
    Some(if bits & (1 << 63) != 0 { !bits } else { bits ^ (1 << 63) })
}

/// Valid-cache path O(1). Dirty path O(B log B + Z) time/O(B + Z) storage;
/// the replacement cache is built separately and installed only on success.
/// Event payloads retain original keys, though expiry uses an ordered batch
/// retain to preserve existing key order and avoid repeated vector shifts.
fn ensure_cache(values: &IndexMap<String, f64>, cache: &mut Cache) {
    if cache.valid { return; }
    let mut rebuilt = Cache::default();
    for (order, (key, value)) in values.iter().enumerate() {
        let Some(tick) = time_key(*value) else { rebuilt.nan_count += 1; continue };
        // No practical map can contain u64::MAX entries, so this fresh queue
        // cannot exhaust its generation counter during a rebuild.
        rebuilt.queue.schedule(
            EventKey { scope: 0, owner: order as u64, kind: 0, channel: 0 },
            Deadline { tick, phase: 0 }, order as u64, key.clone(),
        ).expect("a map cannot contain u64::MAX deadline entries");
    }
    rebuilt.next = rebuilt.queue.next_deadline().map(|deadline| deadline.tick);
    rebuilt.valid = true;
    *cache = rebuilt;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// O(B + Z), B=3 fixed fixture, authoritative order deliberately not sorted.
    fn fixture() -> DeadlineMap {
        [("late".into(), 10.0), ("early".into(), 2.0), ("middle".into(), 5.0)].into_iter().collect()
    }

    /// O(B log B + Z) initialization, then O(1) no-due checks, B=3. Cache counter
    /// stability proves repeated checks do not reconstruct a new queue.
    #[test]
    fn due_checks_cache_and_expiry_preserves_order() {
        let mut map = fixture();
        assert!(!map.has_due(1.0));
        let initial_json = serde_json::to_string(&map.lock().queue).unwrap();
        for tick in [0.0, 1.0, 1.5, 1.9] {
            assert!(!map.has_due(tick));
            assert_eq!(map.expire(tick), 0);
        }
        assert_eq!(serde_json::to_string(&map.lock().queue).unwrap(), initial_json);
        assert!(map.has_due(2.0));
        assert_eq!(map.expire(2.0), 1);
        assert_eq!(map.keys().map(String::as_str).collect::<Vec<_>>(), ["late", "middle"]);
        assert!(!map.has_due(4.0));
        assert_eq!(map.expire(20.0), 2);
        assert!(map.is_empty());
        assert!(map.lock().valid);
        assert!(map.lock().queue.is_empty());
    }

    /// O(B log B) per mutation batch, B<=3; verifies all mutable-access families
    /// invalidate the cache, including references returned by IndexMap methods.
    #[test]
    fn arbitrary_mutable_access_cannot_leave_stale_deadlines() {
        let mut map = fixture();
        assert!(map.has_due(2.0));
        map.insert("early".into(), 20.0);
        assert!(!map.has_due(2.0));
        *map.get_mut("middle").unwrap() = 1.0;
        assert!(map.has_due(1.0));
        map["middle"] = 30.0;
        assert!(!map.has_due(1.0));
        *map.entry("late".into()).or_insert(0.0) = -1.0;
        assert!(map.has_due(0.0));
        map.shift_remove("late");
        assert!(!map.has_due(0.0));
        for value in map.values_mut() { *value = 0.0; }
        assert_eq!(map.expire(0.0), 2);
    }

    /// O(B log B + serialized bytes), B=3. Cache activity does not alter JSON,
    /// key order, clone independence or equality; load rebuilds without extras.
    #[test]
    fn transparent_serde_and_clone_are_canonical() {
        let map = fixture();
        let json = serde_json::to_string(&map).unwrap();
        assert_eq!(json, r#"{"late":10.0,"early":2.0,"middle":5.0}"#);
        assert!(map.has_due(2.0));
        assert_eq!(serde_json::to_string(&map).unwrap(), json);
        let mut clone = map.clone();
        assert!(clone.lock().valid);
        assert_eq!(serde_json::to_string(&clone.lock().queue).unwrap(), serde_json::to_string(&map.lock().queue).unwrap());
        assert_eq!(clone, map);
        assert_eq!(clone.expire(5.0), 2);
        assert_eq!(map.len(), 3);
        let restored: DeadlineMap = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, map);
        assert!(restored.has_due(2.0));
        assert_eq!(serde_json::to_string(&restored).unwrap(), json);
    }

    /// O(T * B log B), T=10 ticks/B=8 edge-case numbers. Compares results with
    /// the exact legacy retain predicate, including fractions/infinities/NaN.
    #[test]
    fn floating_deadlines_match_legacy_comparisons() {
        let values = [f64::NEG_INFINITY, -2.5, -0.0, 0.0, 0.25, 1.5, f64::INFINITY, f64::NAN];
        for tick in [f64::NEG_INFINITY, -3.0, -0.0, 0.0, 0.1, 0.25, 1.0, 1.5, f64::INFINITY, f64::NAN] {
            let original: IndexMap<_, _> = values.iter().enumerate().map(|(i, v)| (i.to_string(), *v)).collect();
            let mut map: DeadlineMap = original.clone().into();
            assert_eq!(map.has_due(tick), original.values().any(|v| *v <= tick));
            let mut expected = original;
            expected.retain(|_, expiry| *expiry > tick);
            map.expire(tick);
            assert_eq!(map.keys().collect::<Vec<_>>(), expected.keys().collect::<Vec<_>>());
            for (key, value) in expected { assert_eq!(map[&key].to_bits(), value.to_bits()); }
        }
    }

    /// O(B log B) per renewal, fixed B=1. Cancelled and renewed old deadlines
    /// cannot remove a new barrier at the old deadline.
    #[test]
    fn renewal_and_cancel_do_not_fire_old_events() {
        let mut map = DeadlineMap::default();
        map.insert("0".into(), 4.0);
        assert!(!map.has_due(3.0));
        map.insert("0".into(), 9.0);
        assert_eq!(map.expire(4.0), 0);
        assert_eq!(map["0"], 9.0);
        map.shift_remove("0");
        assert!(!map.has_due(9.0));
        map.insert("0".into(), 12.0);
        assert_eq!(map.expire(9.0), 0);
        assert_eq!(map.expire(12.0), 1);
    }

    /// O(1) fixture and serde work; outer optional/absent semantics are not
    /// collapsed by the transparent wrapper's serializer.
    #[test]
    fn empty_and_optional_shape_stay_unchanged() {
        #[derive(Serialize, Deserialize)]
        struct Outer { #[serde(default, skip_serializing_if = "Option::is_none")] map: Option<DeadlineMap> }
        let absent: Outer = serde_json::from_str("{}").unwrap();
        assert_eq!(serde_json::to_string(&absent).unwrap(), "{}");
        let null: Outer = serde_json::from_str(r#"{"map":null}"#).unwrap();
        assert!(null.map.is_none());
        let empty: Outer = serde_json::from_str(r#"{"map":{}}"#).unwrap();
        assert_eq!(serde_json::to_string(&empty).unwrap(), r#"{"map":{}}"#);
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DeadlineMap>();
    }
    /// O(T * B log B), T=1000/B<=17 fixture; differential mutation/expiration
    /// sequences preserve ordered keys and exact value bits after every step.
    #[test]
    fn differential_mutation_sequence_matches_index_map() {
        let mut map = DeadlineMap::default();
        let mut reference: IndexMap<String, f64> = IndexMap::new();
        let mut state = 42u64;
        for step in 0..1000 {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let key = ((state >> 16) % 17).to_string();
            let value = ((state >> 32) % 80) as f64 * 0.25 - 3.0;
            match state % 5 {
                0 | 1 => { map.insert(key.clone(), value); reference.insert(key, value); }
                2 => { map.shift_remove(&key); reference.shift_remove(&key); }
                3 => {
                    if let Some(v) = map.get_mut(&key) { *v = value; }
                    if let Some(v) = reference.get_mut(&key) { *v = value; }
                }
                _ => {
                    map.retain(|_, v| *v != value);
                    reference.retain(|_, v| *v != value);
                }
            }
            let tick = (step % 40) as f64 * 0.25;
            assert_eq!(map.has_due(tick), reference.values().any(|v| *v <= tick));
            if step % 3 == 0 {
                let old_len = reference.len();
                reference.retain(|_, v| *v > tick);
                assert_eq!(map.expire(tick), old_len - reference.len());
            }
            assert_eq!(map.keys().collect::<Vec<_>>(), reference.keys().collect::<Vec<_>>());
            for (key, value) in &reference { assert_eq!(map[key].to_bits(), value.to_bits()); }
        }
    }

    /// O(B log B), B=2 fixtures. Global guarded expiration keeps NaN alone;
    /// unconditional catch-up removes it, matching the two old caller patterns.
    #[test]
    fn nan_guarded_and_unconditional_expiration_are_distinct() {
        let mut map = DeadlineMap::default();
        map.insert("nan".into(), f64::NAN);
        assert!(!map.has_due(2.0));
        if map.has_due(2.0) { map.expire(2.0); }
        assert_eq!(map.len(), 1);
        map.insert("due".into(), 2.0);
        assert!(map.has_due(2.0));
        assert_eq!(map.expire(2.0), 2);
        map.insert("nan".into(), f64::NAN);
        assert_eq!(map.expire(2.0), 1);
    }

}
