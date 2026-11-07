//! Opt-in wall-clock diagnostics. Never used to decide simulation behavior.
//! Disabled builds compile spans to a zero-sized no-op. Enabled collection is
//! thread-local: O(log L) aggregation per closed scope, O(depth + L) memory.
//! L is the number of distinct static labels. Inclusive totals overlap; exclusive
//! totals subtract direct child wall time and can be summed within a root sample.

/// Fixed diagnostic counters: no clock, hashing or allocation per increment.
#[derive(Clone, Copy)]
#[repr(usize)]
pub enum Count { ProfileValue, ProfileRgb, ProfileMaterial, HexParse, KnownCells, OutputCells, ActorQueries, LightSources, LightCandidates, LightRayTests, LightFootprintEntries, LightingHit, LightingMiss, OpticalHit, OpticalMiss, ObserverHit, ObserverMiss, ProfileResolvedMaterial, ProfileResolvedScalar }
const COUNT_NAMES: [&str; 19] = ["profile_value", "profile_rgb", "profile_material", "hex_parse", "known_cells", "output_cells", "actor_queries", "light_sources", "light_candidates", "light_ray_tests", "light_footprint_entries", "lighting_hit", "lighting_miss", "optical_hit", "optical_miss", "observer_hit", "observer_miss", "profile_resolved_material", "profile_resolved_scalar"];

#[cfg(feature = "runtime-timing")]
mod enabled {
    use super::{Count,COUNT_NAMES};
    use std::{cell::RefCell, collections::BTreeMap, time::Instant};
    use serde::Serialize;
    #[derive(Clone, Debug, Default, Serialize)]
    pub struct Totals { pub calls: u64, pub inclusive_ns: u128, pub exclusive_ns: u128 }
    struct Frame { label: &'static str, start: Instant, children: u128 }
    #[derive(Default)]
    struct State { enabled: bool, stack: Vec<Frame>, totals: BTreeMap<&'static str, Totals>, counts: [u64; 19] }
    thread_local! { static STATE: RefCell<State> = RefCell::new(State::default()); }
    pub struct Span { depth: Option<usize>, _not_send: std::marker::PhantomData<std::rc::Rc<()>> }
    /// O(1) amortized stack push; no simulation state is observed or mutated.
    pub fn span(label: &'static str) -> Span {
        let depth = STATE.with(|s| { let mut s=s.borrow_mut(); if !s.enabled { return None; }
            let depth=s.stack.len(); s.stack.push(Frame {label,start:Instant::now(),children:0}); Some(depth) });
        Span { depth, _not_send: std::marker::PhantomData }
    }
    impl Drop for Span {
        /// O(log L) accumulation plus O(1) parent accounting. Requires lexical LIFO scopes.
        fn drop(&mut self) { if let Some(depth)=self.depth { STATE.with(|s| {
            let mut s=s.borrow_mut(); assert_eq!(s.stack.len(),depth+1,"timing scopes must close LIFO");
            let f=s.stack.pop().unwrap(); let elapsed=f.start.elapsed().as_nanos();
            if let Some(parent)=s.stack.last_mut() {parent.children+=elapsed;}
            let t=s.totals.entry(f.label).or_default();t.calls+=1;t.inclusive_ns+=elapsed;
            t.exclusive_ns+=elapsed.saturating_sub(f.children);
        }); } }
    }
    /// O(1); call only outside measured scopes. Collection starts disabled.
    pub fn set_enabled(enabled: bool) { STATE.with(|s| {let mut s=s.borrow_mut();assert!(s.stack.is_empty());s.enabled=enabled;}); }
    /// O(L), drops prior counters; call outside spans between independent samples.
    pub fn reset() { STATE.with(|s| {let mut s=s.borrow_mut();assert!(s.stack.is_empty());s.totals.clear();s.counts.fill(0);}); }
    /// O(1) map transfer (caller serialization O(L)); call after root scope closes.
    pub fn take_report() -> BTreeMap<&'static str, Totals> { STATE.with(|s| {let mut s=s.borrow_mut();assert!(s.stack.is_empty());std::mem::take(&mut s.totals)}) }
    /// O(1) skip, with no clock/TLS access for cells outside the sample.
    #[inline]
    pub fn sampled_span(label: &'static str, sample: bool) -> Span {
        if sample { span(label) } else { Span { depth:None, _not_send:std::marker::PhantomData } }
    }
    /// O(1) thread-local fixed slot, disabled collection performs no update.
    pub fn count(which: Count, amount: u64) { STATE.with(|s| { let mut s=s.borrow_mut(); if s.enabled {s.counts[which as usize]=s.counts[which as usize].saturating_add(amount);} }); }
    /// O(number of fixed counters), call outside measured scopes.
    pub fn take_counts() -> BTreeMap<&'static str,u64> { STATE.with(|s| {let mut s=s.borrow_mut();assert!(s.stack.is_empty()); let c=std::mem::replace(&mut s.counts,[0;19]);COUNT_NAMES.into_iter().zip(c).filter(|(_,v)|*v!=0).collect()}) }
    #[cfg(test)] mod tests {
        use super::*;
        /// O(1) time/space: fixed-size nested and disabled-collection cases.
        #[test] fn nested_accounting_and_disabled_collection() {
            reset();set_enabled(true);{let _p=span("parent");{let _c=span("child");std::hint::black_box(17);}}
            let r=take_report();assert_eq!(r["parent"].calls,1);assert_eq!(r["child"].calls,1);
            assert_eq!(r["parent"].inclusive_ns,r["parent"].exclusive_ns+r["child"].inclusive_ns);
            set_enabled(false);{let _s=span("disabled");}assert!(take_report().is_empty());
        }
    }
}
#[cfg(feature = "runtime-timing")]
pub use enabled::*;
#[cfg(not(feature = "runtime-timing"))]
pub struct Span;
/// O(1), optimized away entirely in normal nonprofiling builds.
#[cfg(not(feature = "runtime-timing"))]
#[inline(always)]
pub fn span(_: &'static str) -> Span { Span }

#[cfg(not(feature = "runtime-timing"))]
#[inline(always)]
pub fn count(_: Count, _: u64) {}

#[cfg(not(feature = "runtime-timing"))]
#[inline(always)]
pub fn sampled_span(_: &'static str, _: bool) -> Span { Span }
