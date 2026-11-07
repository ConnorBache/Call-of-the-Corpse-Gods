//! Optional, non-authoritative packet observer for deterministic analysis harnesses.
//! Disabled unless explicitly begun by a caller; never serialized or read by rules.
//! One O(1) append per applied packet when enabled, O(P) memory per captured batch.
//! Amount is the already-mitigated incoming packet; actual is clamped to live HP.
use std::cell::RefCell;

#[derive(Clone, Debug, PartialEq)]
pub struct DamagePacket {
    pub tick: f64,
    pub victim: f64,
    pub amount: f64,
    pub actual: f64,
}
thread_local! {static PENDING:RefCell<Option<Vec<DamagePacket>>>=const{RefCell::new(None)};}

/// Start a command-scoped analysis batch. A second begin discards an unfinished batch.
pub fn begin() { PENDING.with(|p|*p.borrow_mut()=Some(Vec::new())); }
/// Finish analysis and disable observation. No active batch produces an empty list.
pub fn take()->Vec<DamagePacket> {PENDING.with(|p|p.borrow_mut().take().unwrap_or_default())}
/// Called only after actual combat immunity/mitigation, before lethal HP clamping.
pub(crate) fn record(tick:f64,victim:f64,amount:f64,actual:f64) {
    PENDING.with(|p|{if let Some(batch)=p.borrow_mut().as_mut(){batch.push(DamagePacket{tick,victim,amount,actual});}});
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn disabled_by_default_and_take_disables() {
        take();record(1.,0.,14.,10.);assert!(take().is_empty());
        begin();record(1.,0.,14.,10.);assert_eq!(take(),vec![DamagePacket{tick:1.,victim:0.,amount:14.,actual:10.}]);
        record(2.,0.,1.,1.);assert!(take().is_empty());
    }
}
