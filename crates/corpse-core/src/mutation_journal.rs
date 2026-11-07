//! Transaction-local first-write COW evidence, not a complete field-write log.
//! Only Floor / actor-vector / ground-vector Arcs are intercepted. Scalars,
//! replacements and unrestricted public edits require separate coverage/fallback.
//! No JSON serialization happens in mutable access. Retained original Arcs are
//! authoritative keys; raw post-COW addresses are *only* coalescing hints. An
//! allocator-reused hint may suppress capture, so missing exact original keys
//! MUST fall back to the immutable opening snapshot, never imply "unchanged".
use crate::{Actor, Floor, GroundItem, Run};
use std::{any::{Any, TypeId}, cell::RefCell, collections::{HashMap, HashSet}, marker::PhantomData, rc::Rc, sync::Arc};

#[derive(Clone, Copy, Debug, Default)]
pub struct Counters { pub accesses: usize, pub captured_blocks: usize, pub coalesced_accesses: usize }
#[derive(Clone, Debug)]
struct Blocks<T> { originals: HashMap<usize, Arc<T>>, aliases: HashSet<usize> }
impl<T> Default for Blocks<T> {
    /// O(1) empty allocation-free maps.
    fn default() -> Self { Self { originals: HashMap::new(), aliases: HashSet::new() } }
}
impl<T> Blocks<T> {
    /// Expected O(1), retaining only first-old Arc; actual COW payload cost is in make_mut.
    fn before(&mut self, block: &Arc<T>) -> bool {
        let key=Arc::as_ptr(block) as usize;
        if self.originals.contains_key(&key) || self.aliases.contains(&key) { return false; }
        self.originals.insert(key,block.clone()); true
    }
    /// Expected O(1). Hints do not provide authoritative mutation coverage.
    fn after(&mut self, address: usize) { self.aliases.insert(address); }
    /// Expected O(1); retained original allocation makes pointer reuse impossible here.
    fn old(&self, expected: &Arc<T>) -> Option<&Arc<T>> {
        self.originals.get(&(Arc::as_ptr(expected) as usize)).filter(|old|Arc::ptr_eq(old,expected))
    }
    /// Expected O(K) entries/hints, O(K) extra map storage; Arc payload remains shared.
    fn merge(&mut self, other: &Self) {
        for (&key,value) in &other.originals {self.originals.entry(key).or_insert_with(||value.clone());}
        self.aliases.extend(other.aliases.iter().copied());
    }
}
#[derive(Clone, Debug, Default)]
pub struct Journal {
    floors: Blocks<Floor>, actors: Blocks<Vec<Actor>>, ground: Blocks<Vec<GroundItem>>,
    counters: Counters,
}
impl Journal {
    /// O(1) counters; accesses are supported COW borrows, not scalar write counts.
    pub fn counters(&self) -> Counters { self.counters }
    /// Expected O(1), exact first-old evidence; None requires snapshot fallback.
    pub fn old_floor(&self, expected: &Arc<Floor>) -> Option<&Arc<Floor>> {self.floors.old(expected)}
    /// Expected O(1), exact first-old evidence; None requires snapshot fallback.
    pub fn old_actors(&self, expected: &Arc<Vec<Actor>>) -> Option<&Arc<Vec<Actor>>> {self.actors.old(expected)}
    /// Expected O(1), exact first-old evidence; None requires snapshot fallback.
    pub fn old_ground(&self, expected: &Arc<Vec<GroundItem>>) -> Option<&Arc<Vec<GroundItem>>> {self.ground.old(expected)}
    /// Expected O(K) block roots/hints. Merge evidence across free/haste scopes;
    /// lookup remains keyed to the exact requested opening Arc, never a later value.
    pub fn merge(&mut self, other: &Self) {
        self.floors.merge(&other.floors);self.actors.merge(&other.actors);self.ground.merge(&other.ground);
        self.counters.accesses+=other.counters.accesses;
        self.counters.coalesced_accesses+=other.counters.coalesced_accesses;
        self.counters.captured_blocks+=other.counters.captured_blocks;
    }
    /// O(1), distinct retained original roots, unlike historical capture counters.
    pub fn retained_blocks(&self) -> usize {self.floors.originals.len()+self.actors.originals.len()+self.ground.originals.len()}
    /// Expected O(R + K) root-handle scan/filter, with O(R) scratch, R opening
    /// physical/dream floor + actor/ground groups, K retained entries. Releasing
    /// uniquely-owned discarded payload can additionally cost its destruction size.
    pub fn retain_opening(&mut self, opening:&Run) {
        let mut floors:HashSet<usize>=opening.floors.iter().map(|v|Arc::as_ptr(v) as usize).collect();
        if let Some(dream)=opening.dreamwalking.as_ref() {floors.extend(dream.floors.iter().map(|v|Arc::as_ptr(v) as usize));}
        let actors:HashSet<usize>=opening.actors.values().map(|v|Arc::as_ptr(v) as usize).collect();
        let ground:HashSet<usize>=opening.ground.values().map(|v|Arc::as_ptr(v) as usize).collect();
        self.floors.originals.retain(|key,_|floors.contains(key));
        self.actors.originals.retain(|key,_|actors.contains(key));
        self.ground.originals.retain(|key,_|ground.contains(key));
        self.clear_aliases();
    }
    /// O(A) alias table release; aliases are never retained across finished scopes.
    fn clear_aliases(&mut self) {self.floors.aliases.clear();self.actors.aliases.clear();self.ground.aliases.clear();}
    /// Expected O(1), safe Any downcasts only for the three supported types.
    fn before<T: Clone+'static>(&mut self, value:&Arc<T>) {
        let any=value as &dyn Any;
        let captured=if let Some(v)=any.downcast_ref::<Arc<Floor>>() {self.floors.before(v)}
        else if let Some(v)=any.downcast_ref::<Arc<Vec<Actor>>>() {self.actors.before(v)}
        else if let Some(v)=any.downcast_ref::<Arc<Vec<GroundItem>>>() {self.ground.before(v)}
        else {return};
        self.counters.accesses+=1;
        if captured {self.counters.captured_blocks+=1;} else {self.counters.coalesced_accesses+=1;}
    }
    /// Expected O(1), hint-only post-COW address registration.
    fn after<T:'static>(&mut self, address:usize) {
        let kind=TypeId::of::<T>();
        if kind==TypeId::of::<Floor>() {self.floors.after(address);}
        else if kind==TypeId::of::<Vec<Actor>>() {self.actors.after(address);}
        else if kind==TypeId::of::<Vec<GroundItem>>() {self.ground.after(address);}
    }
}
#[derive(Default)]
struct Scopes { next:u64, stack:Vec<(u64,Journal)> }
thread_local! {static SCOPES:RefCell<Scopes>=RefCell::new(Scopes::default());}
/// Thread-bound transaction scope. Dropping without finish discards its evidence.
#[must_use]
pub struct Scope { id:Option<u64>, _thread_bound:PhantomData<Rc<()>> }
/// Amortized O(1) scope push; nested scopes own separate evidence.
pub fn begin() -> Scope {
    let id=SCOPES.with(|s| {let mut s=s.borrow_mut();s.next=s.next.checked_add(1).expect("mutation scope id exhausted");let id=s.next;s.stack.push((id,Journal::default()));id});
    Scope{id:Some(id),_thread_bound:PhantomData}
}
impl Scope {
    /// O(nesting depth + alias count) removal/cleanup; LIFO scope removal is O(1).
    pub fn finish(mut self) -> Journal { let mut journal=self.remove().unwrap_or_default();journal.clear_aliases();journal }
    /// O(nesting depth); out-of-order scope teardown cannot consume another scope.
    fn remove(&mut self) -> Option<Journal> {
        let id=self.id.take()?;
        SCOPES.with(|s| {let mut s=s.borrow_mut();let i=s.stack.iter().rposition(|(key,_)|*key==id)?;Some(s.stack.remove(i).1)})
    }
}
impl Drop for Scope {
    /// O(depth + released journal payload); error/unwind paths discard evidence.
    fn drop(&mut self) {self.remove();}
}
/// Expected O(1) journal bookkeeping plus Arc COW O(payload) when shared.
/// The first active capture retains one original Arc and therefore causes one
/// COW clone; repeated access only keeps raw alias hints, not strong/weak after
/// references, so does not itself force repeated clones/allocations. Inactive
/// access and unsupported types retain ordinary Arc::make_mut behavior.
pub fn make_mut<T:Clone+'static>(value:&mut Arc<T>) -> &mut T {
    let _timing = crate::timing::span("history.journal_mutable_access");
    SCOPES.with(|s| {if let Some((_,journal))=s.borrow_mut().stack.last_mut() {journal.before(value);}});
    let result=Arc::make_mut(value);
    let address=result as *mut T as usize;
    SCOPES.with(|s| {if let Some((_,journal))=s.borrow_mut().stack.last_mut() {journal.after::<T>(address);}});
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    /// O(1) small floor fixture.
    fn floor() -> Arc<Floor> {Arc::new(Floor::default())}
    /// O(1) payload per access; validates original evidence and pointer stability.
    #[test] fn first_old_coalesces_and_keeps_original_even_after_net_zero() {
        let mut v=floor();let old=v.clone();let scope=begin();
        make_mut(&mut v).depth=2.;let after=Arc::as_ptr(&v);
        make_mut(&mut v).depth=0.;assert_eq!(after,Arc::as_ptr(&v));
        let journal=scope.finish();assert_eq!(journal.counters().captured_blocks,1);
        assert_eq!(journal.counters().accesses,2);assert_eq!(journal.old_floor(&old).unwrap().depth,0.);
        assert!(journal.old_floor(&v).is_none());
    }
    /// O(1) fixtures; inactive scope does not add an artificial owner/COW clone.
    #[test] fn inactive_and_unsupported_access_do_not_force_cow() {
        let mut v=floor();let pointer=Arc::as_ptr(&v);make_mut(&mut v).depth=1.;assert_eq!(pointer,Arc::as_ptr(&v));
        let scope=begin();let mut number=Arc::new(1u64);let pointer=Arc::as_ptr(&number);*make_mut(&mut number)=2;assert_eq!(pointer,Arc::as_ptr(&number));assert_eq!(scope.finish().counters().accesses,0);
    }
    /// O(1) fixtures; nested evidence stays separate and dropped scopes clean up.
    #[test] fn nested_drop_and_merge_preserve_exact_opening_keys() {
        let mut v=floor();let old=v.clone();let outer=begin();make_mut(&mut v).depth=1.;
        let middle=v.clone();let inner=begin();make_mut(&mut v).depth=2.;let child=inner.finish();
        make_mut(&mut v).depth=3.;let mut parent=outer.finish();parent.merge(&child);
        assert_eq!(parent.old_floor(&old).unwrap().depth,0.);assert_eq!(parent.old_floor(&middle).unwrap().depth,1.);
        {let _discard=begin();make_mut(&mut v).depth=4.;}
        let pointer=Arc::as_ptr(&v);make_mut(&mut v).depth=5.;assert_eq!(pointer,Arc::as_ptr(&v));
    }
    /// O(1) fixtures; cloned branches may mutate without changing first-old payload.
    #[test] fn cloned_branch_alias_chain_has_exact_original_fallback() {
        let mut a=floor();let original=a.clone();let scope=begin();make_mut(&mut a).depth=1.;
        let mut b=a.clone();make_mut(&mut b).depth=2.;make_mut(&mut b).depth=3.;
        let journal=scope.finish();assert_eq!(original.depth,0.);assert_eq!(a.depth,1.);
        assert_eq!(journal.old_floor(&original).unwrap().depth,0.);assert!(journal.old_floor(&a).is_none());
    }
    /// O(1) fixture; simulates allocator alias reuse without relying on allocator behavior.
    #[test] fn alias_collision_is_only_missing_evidence_never_wrong_old_value() {
        let mut v=floor();let opening=v.clone();let scope=begin();
        SCOPES.with(|s| {s.borrow_mut().stack.last_mut().unwrap().1.floors.aliases.insert(Arc::as_ptr(&v) as usize);});
        make_mut(&mut v).depth=9.;let journal=scope.finish();
        assert!(journal.old_floor(&opening).is_none());
        let authoritative=journal.old_floor(&opening).unwrap_or(&opening);
        assert_eq!(authoritative.depth,0.);assert_eq!(v.depth,9.);
        assert_eq!(journal.counters().captured_blocks,0);
    }

    /// O(root count) pruning; transient probes are not retained as history evidence.
    #[test] fn retain_opening_prunes_probe_roots_and_keeps_historical_counts() {
        let mut opening=Run::default();opening.floors.push(floor());
        let mut live=opening.floors[0].clone();let mut probe=floor();
        let scope=begin();make_mut(&mut live).depth=1.;make_mut(&mut probe).depth=2.;
        let mut journal=scope.finish();assert_eq!(journal.retained_blocks(),2);
        journal.retain_opening(&opening);assert_eq!(journal.retained_blocks(),1);
        assert_eq!(journal.counters().captured_blocks,2);
        assert!(journal.old_floor(&opening.floors[0]).is_some());assert!(journal.floors.aliases.is_empty());
    }

}
