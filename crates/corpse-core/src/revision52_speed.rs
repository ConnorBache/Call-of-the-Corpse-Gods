//! Command-local action receipts. No game-clock, rendering or serialized state:
//! the scope encloses one synchronous NPC action and is discarded before return.
//! Nested reactive movement/attacks record only against the acting NPC identity.
use std::cell::RefCell;
#[derive(Default, Clone, Copy)]
pub(crate) struct Receipt { pub moved: bool, pub attacked: bool }
struct Action { actor: f64, bonus: bool, receipt: Receipt }
thread_local! { static ACTIONS: RefCell<Vec<Action>> = const { RefCell::new(Vec::new()) }; }
struct Scope;
impl Drop for Scope { fn drop(&mut self) { ACTIONS.with(|a| { a.borrow_mut().pop(); }); } }
pub(crate) fn record_move(id:f64) { ACTIONS.with(|a| { if let Some(scope)=a.borrow_mut().last_mut().filter(|a|a.actor==id) {scope.receipt.moved=true;} }); }
pub(crate) fn record_attack(id:f64) { ACTIONS.with(|a| { if let Some(scope)=a.borrow_mut().last_mut().filter(|a|a.actor==id) {scope.receipt.attacked=true;} }); }
/// The current complete action, not the player's saved input phase. Direct
/// action-kernel callers without a scope retain the ordinary-action default.
pub(crate) fn bonus_action(id:f64)->bool {
    ACTIONS.with(|a|a.borrow().last().is_some_and(|a|a.actor==id && a.bonus))
}
/// O(1) bookkeeping per action/movement/attack, O(nesting depth) transient memory.
/// The guard clears both receipt and phase on errors/unwinding; neither survives
/// a command, and a nested different actor cannot inherit the parent's phase.
pub(crate) fn measure<T>(id:f64, bonus:bool, action:impl FnOnce()->T)->(T,Receipt) {
    ACTIONS.with(|a|a.borrow_mut().push(Action{actor:id,bonus,receipt:Receipt::default()}));
    let guard=Scope;
    let result=action();
    let receipt=ACTIONS.with(|a|a.borrow().last().unwrap().receipt);
    drop(guard);
    (result,receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn phase_scope_is_identity_local_and_cleared_on_return_error_and_unwind() {
        assert!(!bonus_action(1.));
        let (out, receipt)=measure(1.,true,|| {
            assert!(bonus_action(1.)); assert!(!bonus_action(2.));
            let (_,nested)=measure(2.,false,|| {
                assert!(!bonus_action(1.)); assert!(!bonus_action(2.));
                record_move(1.); record_attack(2.);
            });
            assert!(!nested.moved && nested.attacked); assert!(bonus_action(1.));
            record_move(1.); Err::<(),_>("expected")
        });
        assert_eq!(out,Err("expected"));assert!(receipt.moved && !receipt.attacked);
        assert!(!bonus_action(1.));
        let panic=std::panic::catch_unwind(||measure(1.,true,||panic!("expected unwind")));
        assert!(panic.is_err());assert!(!bonus_action(1.));
        let (_,receipt)=measure(1.,false,||assert!(!bonus_action(1.)));
        assert!(!receipt.moved && !receipt.attacked);
    }
}
