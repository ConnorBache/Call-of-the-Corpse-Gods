//! Deterministic, keyed deadlines; no wall-clock time, RNG, or actor scanning.
//!
//! The caller owns phase semantics and chooses `order` (for example, the actor's
//! order in the original phase snapshot). Equal orders execute in insertion
//! order. A replacement is a new insertion. `pop_at` never executes overdue work
//! implicitly; `pop_through` is the explicit catch-up operation. A caller that
//! reschedules a recurring event must advance its deadline, or otherwise bound
//! its dispatch loop. This queue does not silently reorder gameplay phases.
//!
//! One queue may be owned by each floor, using that floor's simulation tick.
//! Alternatively, use `scope` in keys for identity and a shared world clock. Scope
//! does NOT filter dispatch: never combine independent clocks in one queue.
//!
//! Complexity variables: N = live events, D = occupied deadline buckets, K =
//! events in the selected bucket. All key comparisons are fixed-size O(1).
//! BTreeMap operations are worst-case logarithmic; no expected-hash claim is
//! needed. The queue and cancellation index contain O(N + D) entries. Cancelling
//! or replacing physically removes entries, so repeated renewal has no stale
//! queue growth. Payload clone/drop/serde costs are additional where applicable.
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet};

/// Stable identity for one logical effect, independent of its next deadline.
/// `owner` is a caller-assigned stable actor/object ID, not its vector position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventKey {
    pub scope: u64,
    pub owner: u64,
    pub kind: u32,
    pub channel: u32,
}

/// Logical tick and caller-defined phase. Numeric phase order is execution order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Deadline {
    pub tick: u64,
    pub phase: u16,
}

/// Generation-qualified reference. An old handle cannot cancel a renewed event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Handle {
    pub key: EventKey,
    pub generation: u64,
}

/// An owned due event, returned before its gameplay callback executes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scheduled<E> {
    pub handle: Handle,
    pub deadline: Deadline,
    pub order: u64,
    pub payload: E,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct OrderKey {
    order: u64,
    generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Location {
    deadline: Deadline,
    order_key: OrderKey,
}

/// No event is changed if the generation counter is exhausted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationExhausted;

/// A deterministic queue and exact cancellation index.
/// Derived Clone costs O(N + D + sum(payload clone costs)); equality compares
/// O(N + D) entries and payload equality costs. Neither is an O(1) snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct Schedule<E> {
    next_generation: u64,
    buckets: BTreeMap<Deadline, BTreeMap<OrderKey, Scheduled<E>>>,
    index: BTreeMap<EventKey, Location>,
}

impl<E> Default for Schedule<E> {
    /// O(1) time and space; no actor/floor inspection.
    fn default() -> Self {
        Self { next_generation: 1, buckets: BTreeMap::new(), index: BTreeMap::new() }
    }
}

impl<E> Schedule<E> {
    /// O(1) time/space; counts live events only, not historical renewals.
    pub fn len(&self) -> usize { self.index.len() }

    /// O(1) time/space. This does not inspect actors or payloads.
    pub fn is_empty(&self) -> bool { self.index.is_empty() }

    /// Schedule or replace a key. O(log N + log D + log K_old + log K_new)
    /// time and O(1) new tree entries, plus drop cost of a replaced payload.
    /// Counter exhaustion leaves the entire queue unchanged.
    pub fn schedule(
        &mut self, key: EventKey, deadline: Deadline, order: u64, payload: E,
    ) -> Result<Handle, GenerationExhausted> {
        let generation = self.next_generation;
        let next = generation.checked_add(1).ok_or(GenerationExhausted)?;
        self.cancel_key(key);
        self.next_generation = next;
        let handle = Handle { key, generation };
        let order_key = OrderKey { order, generation };
        self.buckets.entry(deadline).or_default().insert(order_key,
            Scheduled { handle, deadline, order, payload });
        self.index.insert(key, Location { deadline, order_key });
        Ok(handle)
    }

    /// Generation-safe cancellation. An absent/stale handle costs O(log N).
    /// Success costs O(log N + log D + log K), returns the payload without
    /// cloning it, and releases O(1) queue/index entries.
    pub fn cancel(&mut self, handle: Handle) -> Option<Scheduled<E>> {
        let location = *self.index.get(&handle.key)?;
        if location.order_key.generation != handle.generation { return None; }
        self.remove(handle.key, location)
    }

    /// Cancel the current generation by logical identity. O(log N + log D +
    /// log K) time, O(1) scratch; no queue scan and no payload clone.
    pub fn cancel_key(&mut self, key: EventKey) -> Option<Scheduled<E>> {
        let location = *self.index.get(&key)?;
        self.remove(key, location)
    }

    /// Read the current scheduled event. O(log N + log D + log K) time,
    /// O(1) scratch. A miss in the identity index costs O(log N).
    pub fn get(&self, key: EventKey) -> Option<&Scheduled<E>> {
        let location = self.index.get(&key)?;
        self.buckets.get(&location.deadline)?.get(&location.order_key)
    }

    /// Remove the first event at precisely this deadline. O(log D) on an absent
    /// bucket; otherwise O(log D + log K + log N), O(1) scratch. Older deadlines
    /// remain untouched, making phase/catch-up choices explicit at the caller.
    pub fn pop_at(&mut self, deadline: Deadline) -> Option<Scheduled<E>> {
        let bucket = self.buckets.get(&deadline)?;
        let (&order_key, event) = bucket.first_key_value()?;
        let key = event.handle.key;
        self.remove(key, Location { deadline, order_key })
    }

    /// Pop the chronologically first event only if <= the supplied cutoff.
    /// O(log D) to find the first bucket, then O(log D + log K + log N) on a
    /// due event; O(1) scratch. Includes older ticks' later phases explicitly.
    /// Does not loop through all elapsed ticks or run callbacks.
    pub fn pop_through(&mut self, cutoff: Deadline) -> Option<Scheduled<E>> {
        let (&deadline, _) = self.buckets.first_key_value()?;
        if deadline > cutoff { return None; }
        self.pop_at(deadline)
    }

    /// The earliest scheduled logical deadline. O(log D) time, O(1) scratch.
    /// An empty queue is O(1); no search over actors or all event buckets.
    pub fn next_deadline(&self) -> Option<Deadline> {
        self.buckets.first_key_value().map(|(&deadline, _)| deadline)
    }

    /// Exact index-directed removal. O(log N + log D + log K) time, O(1)
    /// scratch, no payload clone. Removing the last event removes its bucket.
    fn remove(&mut self, key: EventKey, location: Location) -> Option<Scheduled<E>> {
        let bucket = self.buckets.get_mut(&location.deadline)?;
        let event = bucket.remove(&location.order_key)?;
        let empty = bucket.is_empty();
        self.index.remove(&key);
        if empty { self.buckets.remove(&location.deadline); }
        Some(event)
    }
}

impl<E: Serialize> Serialize for Schedule<E> {
    /// O(N + D + payload serialization) time, O(N) reference scratch plus
    /// serializer output. Ordered records give deterministic JSON, unlike map
    /// keys made from structs. No payload cloning and no serialized redundant
    /// cancellation index; it is validated/rebuilt on load.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Wire<'a, E> {
            version: u8,
            next_generation: u64,
            events: Vec<&'a Scheduled<E>>,
        }
        let events = self.buckets.values().flat_map(|bucket| bucket.values()).collect();
        Wire { version: 1, next_generation: self.next_generation, events }.serialize(serializer)
    }
}

impl<'de, E: Deserialize<'de>> Deserialize<'de> for Schedule<E> {
    /// O(N log N + payload deserialization) time and O(N + payload size)
    /// storage/scratch. Rejects duplicates and invalid generations rather than
    /// silently dropping or overwriting gameplay events.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire<E> { version: u8, next_generation: u64, events: Vec<Scheduled<E>> }
        let wire = Wire::<E>::deserialize(deserializer)?;
        if wire.version != 1 || wire.next_generation == 0 {
            return Err(serde::de::Error::custom("unsupported schedule version or invalid counter"));
        }
        let mut schedule = Self { next_generation: wire.next_generation, ..Self::default() };
        let mut generations = BTreeSet::new();
        for event in wire.events {
            let generation = event.handle.generation;
            if generation == 0 || generation >= schedule.next_generation || !generations.insert(generation) {
                return Err(serde::de::Error::custom("invalid or duplicate schedule generation"));
            }
            let key = event.handle.key;
            let deadline = event.deadline;
            let order_key = OrderKey { order: event.order, generation };
            if schedule.index.insert(key, Location { deadline, order_key }).is_some() {
                return Err(serde::de::Error::custom("duplicate schedule event key"));
            }
            schedule.buckets.entry(deadline).or_default().insert(order_key, event);
        }
        Ok(schedule)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// O(1) time/space fixed-size fixture key.
    fn key(owner: u64) -> EventKey { EventKey { scope: 1, owner, kind: 2, channel: 0 } }
    /// O(1) time/space fixed-size logical deadline.
    fn at(tick: u64, phase: u16) -> Deadline { Deadline { tick, phase } }

    /// O(N log N) queue setup; checks explicit actor order then insertion tie
    /// order for N=4 fixed fixtures. O(N) storage.
    #[test]
    fn stable_order_is_explicit_not_actor_id() {
        let mut s = Schedule::default();
        s.schedule(key(10), at(5, 2), 9, "last").unwrap();
        s.schedule(key(30), at(5, 2), 1, "first").unwrap();
        s.schedule(key(20), at(5, 2), 1, "second").unwrap();
        s.schedule(key(1), at(5, 2), 4, "third").unwrap();
        let values: Vec<_> = std::iter::from_fn(|| s.pop_at(at(5, 2))).map(|e| e.payload).collect();
        assert_eq!(values, ["first", "second", "third", "last"]);
        assert!(s.is_empty());
        assert!(s.buckets.is_empty());
    }

    /// O(R log N) for R=1000 replacements, N=1 live event; O(1) live storage.
    /// Tests physically removed renewals and generation-safe stale cancellation.
    #[test]
    fn renewal_does_not_accumulate_or_accept_stale_handle() {
        let mut s = Schedule::default();
        let stale = s.schedule(key(1), at(1, 0), 0, 0).unwrap();
        for n in 1..1000 {
            s.schedule(key(1), at(n + 1, 0), 0, n).unwrap();
            assert_eq!(s.len(), 1);
            assert_eq!(s.buckets.len(), 1);
            assert!(s.cancel(stale).is_none());
        }
        let current = s.get(key(1)).unwrap().handle;
        assert_eq!(s.cancel(current).unwrap().payload, 999);
        assert!(s.is_empty());
        assert!(s.buckets.is_empty());
    }

    /// O(N log N), N=3 events; verifies precise phases and explicit overdue
    /// processing without advancing or searching every intervening tick.
    #[test]
    fn exact_phase_and_explicit_catch_up() {
        let mut s = Schedule::default();
        s.schedule(key(1), at(2, 9), 0, "old").unwrap();
        s.schedule(key(2), at(8, 1), 0, "now").unwrap();
        s.schedule(key(3), at(8, 2), 0, "later").unwrap();
        assert!(s.pop_at(at(8, 0)).is_none());
        assert_eq!(s.pop_at(at(8, 1)).unwrap().payload, "now");
        assert_eq!(s.pop_through(at(8, 1)).unwrap().payload, "old");
        assert!(s.pop_through(at(8, 1)).is_none());
        assert_eq!(s.next_deadline(), Some(at(8, 2)));
        assert_eq!(s.pop_through(at(8, 2)).unwrap().payload, "later");
    }

    /// O(N log N + serialized bytes), N=3, and O(N) scratch. Tests clone and
    /// serde restore identical cancellation identity, ordering and next counter.
    #[test]
    fn deterministic_roundtrip_and_clone() {
        let mut s = Schedule::default();
        s.schedule(key(4), at(8, 4), 0, "far").unwrap();
        s.schedule(key(2), at(3, 2), 5, "early").unwrap();
        let handle = s.schedule(key(9), at(3, 2), 1, "first").unwrap();
        let json = serde_json::to_string(&s).unwrap();
        let mut restored: Schedule<String> = serde_json::from_str(&json).unwrap();
        let mut cloned = s.clone();
        assert_eq!(serde_json::to_string(&restored).unwrap(), json);
        assert_eq!(restored.cancel(handle).unwrap().payload, cloned.cancel(handle).unwrap().payload);
        while let Some(a) = restored.pop_through(at(u64::MAX, u16::MAX)) {
            let b = cloned.pop_through(at(u64::MAX, u16::MAX)).unwrap();
            assert_eq!(a.payload, b.payload);
            assert_eq!(a.handle, b.handle);
        }
        assert!(cloned.is_empty());
    }

    /// O(N log N + serialized bytes), N=2. Invalid serialized aliases must not
    /// replace an event silently or permit stale generations.
    #[test]
    fn rejects_invalid_serialized_state() {
        let mut s = Schedule::default();
        s.schedule(key(1), at(1, 0), 0, 7).unwrap();
        let original = serde_json::to_value(s).unwrap();
        let mut duplicate = original.clone();
        let event = duplicate["events"][0].clone();
        duplicate["events"].as_array_mut().unwrap().push(event);
        assert!(serde_json::from_value::<Schedule<u32>>(duplicate).is_err());
        let mut invalid = original.clone();
        invalid["events"][0]["handle"]["generation"] = 0.into();
        assert!(serde_json::from_value::<Schedule<u32>>(invalid).is_err());
        let mut future = original.clone();
        future["events"][0]["handle"]["generation"] = 100.into();
        assert!(serde_json::from_value::<Schedule<u32>>(future).is_err());
        let mut version = original;
        version["version"] = 2.into();
        assert!(serde_json::from_value::<Schedule<u32>>(version).is_err());
    }

    /// O(1) fixture size. Generation exhaustion must not cancel the old event
    /// or otherwise mutate state before returning the error.
    #[test]
    fn exhaustion_is_transactional() {
        let mut s = Schedule::default();
        s.schedule(key(1), at(1, 0), 0, 1).unwrap();
        s.next_generation = u64::MAX;
        let before = s.clone();
        assert_eq!(s.schedule(key(1), at(2, 0), 0, 2), Err(GenerationExhausted));
        assert_eq!(s, before);
    }

    /// O(N log N) setup for N=1024; absent due access uses only tree lookup,
    /// with no payload clones/comparisons/drop and no queue mutation. O(N) space.
    #[test]
    fn absent_due_does_not_touch_payloads_or_events() {
        #[derive(Debug)]
        struct Opaque;
        let mut s = Schedule::default();
        for n in 0..1024 { s.schedule(key(n), at(n + 100, 2), n, Opaque).unwrap(); }
        let counter = s.next_generation;
        for phase in 0..100 { assert!(s.pop_at(at(50, phase)).is_none()); }
        assert_eq!(s.len(), 1024);
        assert_eq!(s.next_generation, counter);
        assert_eq!(s.next_deadline(), Some(at(100, 2)));
    }
}
