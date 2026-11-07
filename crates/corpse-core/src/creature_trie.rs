//! Lazy decimal-key lookup and generation-checked reusable local slots.
//!
//! A trie starts with five decimal digits (00000..99999), then grows leftward
//! without renumbering existing keys. Only occupied branches allocate lists.
//! These containers allocate no game identities themselves: callers must reserve
//! identities and keep canonical actor iteration order separate from lookup.
use std::sync::Arc;

#[derive(Debug, Clone)]
enum Node<T: Clone> {
    Branch { children: [Option<Arc<Node<T>>>; 10], nodes: usize },
    Leaf { values: [Option<T>; 10] },
}

impl<T: Clone> Node<T> {
    /// O(1) time/space: each node has exactly ten entries.
    fn empty(divisor: u64) -> Self {
        if divisor == 1 { Self::Leaf { values: std::array::from_fn(|_| None) } }
        else { Self::Branch { children: std::array::from_fn(|_| None), nodes: 1 } }
    }

    /// O(1) time and extra space; counts this subtree's allocated lists.
    fn nodes(&self) -> usize {
        match self { Self::Branch { nodes, .. } => *nodes, Self::Leaf { .. } => 1 }
    }

    /// O(d) time, O(d) stack for d remaining decimal digits.
    fn get(&self, key: u64, divisor: u64) -> Option<&T> {
        let digit = ((key / divisor) % 10) as usize;
        match self {
            Self::Leaf { values } => values[digit].as_ref(),
            Self::Branch { children, .. } => children[digit].as_ref()?.get(key, divisor / 10),
        }
    }

    /// O(d * (1+C)) time, O(d) copied nodes/stack, C = T clone cost.
    fn get_mut(&mut self, key: u64, divisor: u64) -> Option<&mut T> {
        let digit = ((key / divisor) % 10) as usize;
        match self {
            Self::Leaf { values } => values[digit].as_mut(),
            Self::Branch { children, .. } => Arc::make_mut(children[digit].as_mut()?).get_mut(key, divisor / 10),
        }
    }

    /// O(d * (1+C)) time and O(d) nodes/stack worst case, where C is the
    /// cost of cloning T. Shared snapshots copy only the modified path; each
    /// copied leaf clones at most ten values. Caller has checked absence.
    fn insert(&mut self, key: u64, divisor: u64, value: T) {
        let digit = ((key / divisor) % 10) as usize;
        match self {
            Self::Leaf { values } => values[digit] = Some(value),
            Self::Branch { children, nodes } => {
                let old = children[digit].as_ref().map_or(0, |n| n.nodes());
                let child = children[digit].get_or_insert_with(|| Arc::new(Self::empty(divisor / 10)));
                Arc::make_mut(child).insert(key, divisor / 10, value);
                *nodes = *nodes - old + child.nodes();
            }
        }
    }

    /// O(d * (1+C)) time, O(d) copied nodes/stack, C = T clone cost.
    /// Returns the removed value and whether this list is now empty.
    fn remove(&mut self, key: u64, divisor: u64) -> (Option<T>, bool) {
        let digit = ((key / divisor) % 10) as usize;
        match self {
            Self::Leaf { values } => {
                let old = values[digit].take();
                (old, values.iter().all(Option::is_none))
            }
            Self::Branch { children, nodes } => {
                let Some(child) = children[digit].as_mut() else { return (None, false); };
                let before = child.nodes();
                let (old, empty) = Arc::make_mut(child).remove(key, divisor / 10);
                let after = if empty { 0 } else { child.nodes() };
                if empty { children[digit] = None; }
                *nodes = *nodes - before + after;
                (old, children.iter().all(Option::is_none))
            }
        }
    }
}

/// Decimal trie with O(1) snapshot cloning. Keys are full u64 integers; the
/// initial five-digit capacity is a minimum, not a truncation or hard cap.
#[derive(Debug)]
pub struct DecimalTrie<T: Clone> {
    root: Option<Arc<Node<T>>>,
    digits: u32,
    len: usize,
}

impl<T: Clone> Default for DecimalTrie<T> {
    /// O(1) time/space; an empty trie allocates no lists.
    fn default() -> Self { Self { root: None, digits: 5, len: 0 } }
}

impl<T: Clone> Clone for DecimalTrie<T> {
    /// O(1) time/space; later edits copy only their modified path.
    fn clone(&self) -> Self { Self { root: self.root.clone(), digits: self.digits, len: self.len } }
}

impl<T: Clone> DecimalTrie<T> {
    /// O(1) time/space.
    pub fn len(&self) -> usize { self.len }
    /// O(1) time/space.
    pub fn is_empty(&self) -> bool { self.len == 0 }
    /// O(1) time/space; counts logical reachable node lists, not unique physical
    /// allocations shared across separately cloned snapshots.
    pub fn allocated_nodes(&self) -> usize { self.root.as_ref().map_or(0, |r| r.nodes()) }
    /// O(1) time/space; width lies in 5..=20.
    pub fn digits(&self) -> u32 { self.digits }

    /// O(d) time and O(d) stack; fixed five-digit IDs need five list accesses.
    /// Out-of-width keys cannot alias a shorter key.
    pub fn get(&self, key: u64) -> Option<&T> {
        if self.digits < 20 && key >= 10u64.pow(self.digits) { return None; }
        self.root.as_ref()?.get(key, 10u64.pow(self.digits - 1))
    }

    /// O(d * (1+C)) time, O(d) copied nodes/stack, C = T clone cost.
    /// Missing keys do not allocate or copy lists.
    pub fn get_mut(&mut self, key: u64) -> Option<&mut T> {
        self.get(key)?;
        Arc::make_mut(self.root.as_mut()?).get_mut(key, 10u64.pow(self.digits - 1))
    }

    /// O(d * (1+C)) time, O(d) new nodes/stack, C = T clone cost. Duplicate
    /// rejection is non-mutating. Growth preserves all old keys and values.
    pub fn insert_new(&mut self, key: u64, value: T) -> Result<(), String> {
        if self.get(key).is_some() { return Err(format!("Duplicate decimal-trie key {key}")); }
        while self.digits < 20 && key >= 10u64.pow(self.digits) {
            if let Some(old) = self.root.take() {
                let nodes = 1 + old.nodes();
                let mut children = std::array::from_fn(|_| None);
                children[0] = Some(old);
                self.root = Some(Arc::new(Node::Branch { children, nodes }));
            }
            self.digits += 1;
        }
        let divisor = 10u64.pow(self.digits - 1);
        let root = self.root.get_or_insert_with(|| Arc::new(Node::empty(divisor)));
        Arc::make_mut(root).insert(key, divisor, value);
        self.len += 1;
        Ok(())
    }

    /// O(d * (1+C)) time, O(d) copied nodes/stack, C = T clone cost. Prunes
    /// empty lists; removing the last key restores the empty five-digit state.
    pub fn remove(&mut self, key: u64) -> Option<T> {
        self.get(key)?;
        let (old, empty) = Arc::make_mut(self.root.as_mut()?).remove(key, 10u64.pow(self.digits - 1));
        if old.is_some() { self.len -= 1; }
        if empty { self.root = None; self.digits = 5; }
        old
    }
}

/// A reusable floor-local slot is not a global creature identity. Retain both
/// fields when storing references: the same slot number may later be occupied
/// by a different creature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotHandle { pub slot: u32, pub generation: u64 }

#[derive(Debug, Clone)]
struct Slot<T: Clone> { generation: u64, value: Option<T> }

/// Reusable slot allocator with O(1) snapshot cloning. Slots themselves use
/// the persistent decimal trie; the free stack uses copy-on-write storage.
/// Iterating canonical creatures must not use slot order.
#[derive(Debug, Clone)]
pub struct SlotMap<T: Clone> {
    slots: DecimalTrie<Slot<T>>,
    free: Arc<Vec<u32>>,
    next_slot: u64,
    len: usize,
}

impl<T: Clone> Default for SlotMap<T> {
    /// O(1) time/space; no slot lists allocated.
    fn default() -> Self {
        Self { slots: DecimalTrie::default(), free: Arc::new(Vec::new()), next_slot: 0, len: 0 }
    }
}

impl<T: Clone> SlotMap<T> {
    /// O(1) time/space.
    pub fn len(&self) -> usize { self.len }
    /// O(1) time/space.
    pub fn is_empty(&self) -> bool { self.len == 0 }
    /// O(1) time/space; includes vacant and generation-exhausted slots.
    pub fn allocated_slots(&self) -> usize { self.slots.len() }

    /// O(d*(1+C)) trie work plus amortized O(1) free-stack work, where d<=10
    /// for u32 slots and C=T clone cost. After snapshot cloning, first free-stack
    /// mutation copies O(F) free entries. Trie edits copy only O(d) nodes.
    /// Full u32 capacity returns an error without changing any existing slot.
    pub fn allocate(&mut self, value: T) -> Result<SlotHandle, String> {
        let handle = if let Some(&slot) = self.free.last() {
            Arc::make_mut(&mut self.free).pop();
            let entry = self.slots.get_mut(u64::from(slot)).expect("free slot must exist");
            entry.value = Some(value);
            SlotHandle { slot, generation: entry.generation }
        } else {
            let slot = u32::try_from(self.next_slot).map_err(|_| "Floor-local slot capacity exhausted".to_string())?;
            self.slots.insert_new(self.next_slot, Slot { generation: 0, value: Some(value) })?;
            self.next_slot += 1;
            SlotHandle { slot, generation: 0 }
        };
        self.len += 1;
        Ok(handle)
    }

    /// O(d) time/O(d) stack, d<=10. Stale handles never resolve to new occupants.
    pub fn get(&self, handle: SlotHandle) -> Option<&T> {
        let entry = self.slots.get(u64::from(handle.slot))?;
        if entry.generation != handle.generation { return None; }
        entry.value.as_ref()
    }

    /// O(d*(1+C)) trie work and O(d) copied nodes, C=T clone cost. Free-stack
    /// push is amortized O(1), worst O(F) time/space for reallocation or first
    /// write after cloning. Exhausted generations retire rather than wrap.
    pub fn remove(&mut self, handle: SlotHandle) -> Option<T> {
        self.get(handle)?;
        let entry = self.slots.get_mut(u64::from(handle.slot))?;
        let old = entry.value.take()?;
        self.len -= 1;
        if let Some(next) = entry.generation.checked_add(1) {
            entry.generation = next;
            Arc::make_mut(&mut self.free).push(handle.slot);
        }
        Some(old)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// O(N*d) time/O(N) space, N=100000, d=5.
    #[test]
    fn hundred_thousand_unique_keys_and_pruning() {
        let mut trie = DecimalTrie::default();
        assert_eq!(trie.allocated_nodes(), 0);
        for key in 0..100_000 { trie.insert_new(key, key + 7).unwrap(); }
        assert_eq!(trie.len(), 100_000);
        assert_eq!(trie.digits(), 5);
        assert_eq!(trie.allocated_nodes(), 11_111);
        for key in 0..100_000 { assert_eq!(trie.get(key), Some(&(key + 7))); }
        assert_eq!(trie.get(100_000), None);
        assert!(trie.insert_new(99_999, 0).is_err());
        assert_eq!(trie.get(99_999), Some(&100_006));
        for key in 0..100_000 { assert_eq!(trie.remove(key), Some(key + 7)); }
        assert!(trie.is_empty());
        assert_eq!(trie.allocated_nodes(), 0);
    }

    /// O(d) time/space, d<=20; sparse extreme keys and persistent snapshots.
    #[test]
    fn sparse_growth_extremes_and_snapshot_isolation() {
        let mut trie = DecimalTrie::default();
        trie.insert_new(0, "player").unwrap();
        assert_eq!(trie.allocated_nodes(), 5);
        let snapshot = trie.clone();
        trie.insert_new(100_000, "six digits").unwrap();
        trie.insert_new(u64::MAX, "maximum").unwrap();
        assert_eq!(trie.digits(), 20);
        assert!(trie.allocated_nodes() <= 60);
        assert_eq!(trie.get(0), Some(&"player"));
        assert_eq!(trie.get(100_000), Some(&"six digits"));
        assert_eq!(trie.get(u64::MAX), Some(&"maximum"));
        assert_eq!(snapshot.get(100_000), None);
        assert_eq!(snapshot.get(0), Some(&"player"));
        assert_eq!(trie.remove(0), Some("player"));
        assert_eq!(snapshot.get(0), Some(&"player"));
        assert_eq!(trie.remove(0), None);
        trie.remove(100_000); trie.remove(u64::MAX);
        assert_eq!(trie.digits(), 5);
        assert_eq!(trie.allocated_nodes(), 0);
    }

    /// O(N*d) time/O(N) space, N=100000, d=5; stale generations never alias.
    #[test]
    fn hundred_thousand_slots_reuse_and_snapshot_independence() {
        let mut slots = SlotMap::default();
        let handles: Vec<_> = (0..100_000).map(|id| slots.allocate(id).unwrap()).collect();
        assert_eq!(slots.len(), 100_000);
        let snapshot = slots.clone();
        for (id, handle) in handles.iter().enumerate() {
            assert_eq!(slots.get(*handle), Some(&(id as i32)));
            assert_eq!(slots.remove(*handle), Some(id as i32));
            assert_eq!(slots.get(*handle), None);
            assert_eq!(slots.remove(*handle), None);
        }
        for id in 0..100_000 {
            let new = slots.allocate(id).unwrap();
            assert_eq!(new.generation, 1);
            assert_eq!(slots.get(handles[new.slot as usize]), None);
            assert_eq!(snapshot.get(handles[new.slot as usize]), Some(&(new.slot as i32)));
        }
        assert_eq!(slots.allocated_slots(), 100_000);
    }

    /// O(1) time/space; explicit counter exhaustion cannot resurrect a handle.
    #[test]
    fn exhausted_generation_retires_slot() {
        let mut slots = SlotMap::default();
        let old = slots.allocate(7).unwrap();
        slots.slots.get_mut(0).unwrap().generation = u64::MAX;
        let last = SlotHandle { slot: old.slot, generation: u64::MAX };
        assert_eq!(slots.remove(last), Some(7));
        let replacement = slots.allocate(8).unwrap();
        assert_ne!(replacement.slot, last.slot);
        assert_eq!(slots.get(last), None);
    }

    /// O(d) time/space, d=5; synthetic exhaustion checks the failure boundary
    /// without allocating billions of slots.
    #[test]
    fn capacity_error_preserves_existing_occupant_and_reuse() {
        let mut slots = SlotMap::default();
        let occupant = slots.allocate(7).unwrap();
        slots.next_slot = u64::from(u32::MAX) + 1;
        assert!(slots.allocate(8).is_err());
        assert_eq!(slots.len(), 1);
        assert_eq!(slots.get(occupant), Some(&7));
        assert_eq!(slots.remove(occupant), Some(7));
        let reused = slots.allocate(9).unwrap();
        assert_eq!(reused.slot, occupant.slot);
        assert_ne!(reused.generation, occupant.generation);
    }

    /// O(N*d) time/O(N) space; optional isolated microbenchmark, not gameplay
    /// timing. Logical node bytes omit allocator metadata and shared Arc headers.
    #[test]
    #[ignore = "run optimized with --ignored --nocapture for lookup microtimings"]
    fn decimal_trie_microtiming() {
        use std::hint::black_box;
        use std::time::Instant;
        let mut trie = DecimalTrie::default();
        let start = Instant::now();
        for key in 0..100_000 { trie.insert_new(key, key).unwrap(); }
        let build = start.elapsed();
        let start = Instant::now();
        for _ in 0..10 {
            for key in 0..100_000 { black_box(trie.get(black_box(key))); }
        }
        let lookup = start.elapsed();
        let start = Instant::now();
        for _ in 0..100_000 { black_box(trie.clone()); }
        let clone = start.elapsed();
        println!("100k insert {:?}; 1m lookup {:?}; 100k snapshot clone {:?}; logical lists {}; node payload bytes {}", build, lookup, clone, trie.allocated_nodes(), trie.allocated_nodes() * std::mem::size_of::<Node<u64>>());
    }


    /// O(K*(d+log K)) time/O(K*d) space for K deterministic mixed operations;
    /// compares against an independent ordered map and checks snapshot writes.
    #[test]
    fn mixed_operations_match_ordered_map() {
        let mut trie = DecimalTrie::default();
        let mut expected = std::collections::BTreeMap::new();
        let mut state = 0x91f0_789a_5432_1234u64;
        for step in 0..20_000u64 {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let key = (state >> 8) % 150_000;
            match state % 3 {
                0 => {
                    let absent = !expected.contains_key(&key);
                    assert_eq!(trie.insert_new(key, step).is_ok(), absent);
                    if absent { expected.insert(key, step); }
                }
                1 => assert_eq!(trie.remove(key), expected.remove(&key)),
                _ => {
                    if let Some(value) = trie.get_mut(key) { *value += 1; }
                    if let Some(value) = expected.get_mut(&key) { *value += 1; }
                }
            }
            assert_eq!(trie.len(), expected.len());
            assert_eq!(trie.get(key), expected.get(&key));
        }
        let mut snapshot = trie.clone();
        for (&key, &value) in &expected {
            assert_eq!(trie.get(key), Some(&value));
            *snapshot.get_mut(key).unwrap() = value.wrapping_add(1);
            assert_eq!(trie.get(key), Some(&value));
            assert_eq!(snapshot.get(key), Some(&value.wrapping_add(1)));
        }
    }

}
