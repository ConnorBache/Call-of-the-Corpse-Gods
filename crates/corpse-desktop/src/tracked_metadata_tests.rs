use super::*;
use corpse_core::Field;
fn ready_metadata() -> Engine {
    let mut core=corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    core.run.inventory_display_order=Field::Missing;core.run.run_id=Field::Missing;
    let e=Engine::from_core(core);assert!(e.acknowledge_persisted(e.token()));e
}
#[test]
fn metadata_fields_preserve_presence_order_bits_and_noop_tokens(){
    let mut e=ready_metadata();let mut oracle=e.core.clone();let original=Stamp::capture(&e);
    let token=e.token();assert!(!e.set_inventory_display_order(Field::Missing).unwrap());assert_eq!(token,e.token());
    for desired in [Field::Null,Field::Value(vec![]),Field::Value(vec![99999.,-0.,0.,7.,7.]),Field::Missing]{
        assert!(e.set_inventory_display_order(desired.clone()).unwrap());let token=e.token();assert!(!e.set_inventory_display_order(desired).unwrap());assert_eq!(e.token(),token);
        assert_eq!(Stamp::capture(&e),original);assert!(!e.pending_batch().checkpoint_required);
    }
    let batch=e.pending_batch();assert_eq!(batch.operations.len(),4);assert!(batch.receipts.is_empty());
    for op in batch.operations {let encoded=serde_json::to_string(&op).unwrap();let op:Operation=serde_json::from_str(&encoded).unwrap();op.apply(&mut oracle).unwrap();}
    assert_eq!(oracle.save().unwrap(),e.save().unwrap());
}
#[test]
fn known_metadata_cannot_clear_unknown_error_or_overflow_and_stale_ack(){
    let mut e=ready_metadata();let old=e.token();e.set_inventory_display_order(vec![1.].into()).unwrap();assert!(!e.acknowledge_persisted(old));
    e.require_checkpoint();e.set_inventory_display_order(vec![2.].into()).unwrap();assert!(e.pending_batch().checkpoint_required);assert_eq!(e.pending_batch().operations.len(),1);
    let mut e=ready_metadata();assert!(e.dispatch(&Command::Dev{action:"unrecognized".into()}).is_err());e.set_inventory_display_order(vec![2.].into()).unwrap();assert!(e.pending_batch().checkpoint_required);
    let mut e=ready_metadata();for i in 0..32{e.set_inventory_display_order(vec![i as f64].into()).unwrap();}assert_eq!(e.pending_batch().operations.len(),32);assert!(!e.pending_batch().checkpoint_required);
    e.set_inventory_display_order(vec![100.].into()).unwrap();assert!(e.pending_batch().checkpoint_required);assert_eq!(e.pending_batch().operations.len(),32);
}
#[test]
fn identity_and_invalid_order_refuse_before_mutation_or_epoch(){
    let mut e=ready_metadata();let t=e.token();assert!(e.set_inventory_display_order(vec![f64::INFINITY].into()).is_err());assert_eq!(e.token(),t);
    assert!(e.set_run_id(Field::Null).is_err());assert_eq!(e.token(),t);assert!(e.core.run.run_id.is_missing());
    assert!(e.ensure_run_id(||"bad\nidentity".into()).is_err());assert_eq!(e.token(),t);
    assert_eq!(e.ensure_run_id(||"metadata-record-id".into()).unwrap(),"metadata-record-id");let t=e.token();
    assert_eq!(e.ensure_run_id(||panic!("existing identity must not call generator")).unwrap(),"metadata-record-id");assert_eq!(e.token(),t);
    assert_eq!(e.pending_batch().operations.len(),1);assert!(e.pending_batch().receipts.is_empty());
}
#[test]
fn metadata_command_order_and_seed_ledger_are_separate(){
    let mut e=ready_metadata();let mut oracle=e.core.clone();let replay=e.run.replay.len();let rng=e.run.rng;
    e.set_inventory_display_order(vec![99.,3.].into()).unwrap();e.ensure_run_id(||"ordered-metadata".into()).unwrap();assert_eq!(e.run.rng,rng);assert_eq!(e.run.replay.len(),replay);
    e.dispatch(&Command::Wait).unwrap();e.set_inventory_display_order(Field::Null).unwrap();
    let batch=e.pending_batch();assert!(matches!(batch.operations[0],Operation::InventoryDisplayOrder{..}));assert!(matches!(batch.operations[1],Operation::RunId{..}));assert!(matches!(batch.operations[2],Operation::Command{..}));assert_eq!(batch.receipts.len(),1);
    for operation in batch.operations {operation.apply(&mut oracle).unwrap();}
    assert_eq!(e.save().unwrap(),oracle.save().unwrap());assert_eq!(e.run.replay.len(),replay+1);
    assert_eq!(serde_json::to_value(corpse_core::replay::ReplayRecord::from_run(&e.run)).unwrap(),serde_json::to_value(corpse_core::replay::ReplayRecord::from_run(&oracle.run)).unwrap());
}

#[test]
fn selected_order_is_bounded_by_wire_bytes_not_an_invented_item_count(){
    let mut engine=ready_metadata();
    let ids=Field::Value(vec![1.;20_000]);
    assert!(engine.set_inventory_display_order(ids.clone()).unwrap());
    assert_eq!(engine.run.inventory_display_order,ids);
    let token=engine.token();let before=engine.save().unwrap();let pending=engine.pending_batch().operations;
    assert!(engine.set_inventory_display_order(Field::Value(vec![1.;150_000])).is_err());
    assert_eq!(engine.token(),token);assert_eq!(engine.save().unwrap(),before);
    assert_eq!(engine.pending_batch().operations,pending);
}
