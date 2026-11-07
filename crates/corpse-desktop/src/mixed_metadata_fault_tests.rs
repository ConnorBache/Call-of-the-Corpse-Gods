// Included inside metadata_operation_tests; uses its test-local Directory/fixture.
#[test]
fn mixed_metadata_command_faults_preserve_pending_and_retry_exactly_once(){
 for(fault,complete)in[(AppendFault::Partial(13),false),(AppendFault::BeforeSync,true),(AppendFault::AfterSync,true)]{
  let dir=Directory::new();let path=dir.0.join("mixed.save");let mut e=tracked_engine::Engine::from_core(fixture(41.));let mut a=Autosave::default();
  a.flush_tracked(&path,&e,Kind::Fixture,false).unwrap();let prior=std::fs::read(&path).unwrap();let prior_state=decode(&prior).unwrap().0.save().unwrap();
  e.set_inventory_display_order(vec![99999.,99999.,12345.].into()).unwrap();
  e.ensure_run_id(||"mixed-fault-identity".into()).unwrap();e.dispatch(&Command::Wait).unwrap();
  e.set_inventory_display_order(Field::Null).unwrap();
  let pending=e.pending_batch();assert_eq!(pending.operations.len(),4);let expected=e.save().unwrap();let replay=e.run.replay.clone();
  inject_append_fault(fault);assert!(a.flush_tracked(&path,&e,Kind::Fixture,false).is_err());
  assert_eq!(e.pending_batch().operations,pending.operations);assert_eq!(e.token(),pending.token);
  let interrupted=std::fs::read(&path).unwrap();let recovered=decode(&interrupted).unwrap().0;
  assert_eq!(recovered.save().unwrap(),if complete{expected.clone()}else{prior_state});
  a.flush_tracked(&path,&e,Kind::Fixture,false).unwrap();
  assert!(e.pending_batch().operations.is_empty());assert_eq!(e.run.replay,replay);
  let bytes=std::fs::read(&path).unwrap();let loaded=decode(&bytes).unwrap().0;
  assert_eq!(loaded.save().unwrap(),expected);assert_eq!(loaded.run.inventory_display_order,Field::Null);assert_eq!(loaded.run.run_id,e.run.run_id);
  a.flush_tracked(&path,&e,Kind::Fixture,true).unwrap();assert_eq!(std::fs::read(&path).unwrap(),bytes);
 }
}
#[test]
fn mixed_metadata_torn_record_never_partially_replays_operations(){
 let dir=Directory::new();let path=dir.0.join("mixed-torn.save");let mut e=tracked_engine::Engine::from_core(fixture(41.));let mut a=Autosave::default();
 a.flush_tracked(&path,&e,Kind::Fixture,false).unwrap();let prior=std::fs::read(&path).unwrap();let prior_state=decode(&prior).unwrap().0.save().unwrap();
 e.set_inventory_display_order(vec![99999.,99999.].into()).unwrap();e.dispatch(&Command::Wait).unwrap();e.ensure_run_id(||"mixed-tail-identity".into()).unwrap();
 a.flush_tracked(&path,&e,Kind::Fixture,false).unwrap();let full=std::fs::read(&path).unwrap();let header=log::inspect_header(&full).unwrap();
 let previous=log::parse(&prior,&header,|s|parse_operations(s,true).map(|_|())).unwrap();let fullparsed=log::parse(&full,&header,|s|parse_operations(s,true).map(|_|())).unwrap();
 assert_eq!(fullparsed.records.len(),previous.records.len()+1);assert_eq!(parse_operations(fullparsed.records.last().unwrap().payload,true).unwrap().len(),3);
 // Exhaustive framing-only cuts; representative full decodes avoid repeated world imports.
 for cut in prior.len()..full.len(){let p=log::parse(&full[..cut],&header,|s|parse_operations(s,true).map(|_|())).unwrap();assert_eq!(p.metadata.valid_len,prior.len());assert_eq!(p.records.len(),previous.records.len());assert_eq!(p.checkpoint,previous.checkpoint);}
 for cut in [prior.len(),prior.len()+1,prior.len()+(full.len()-prior.len())/2,full.len()-1]{assert_eq!(decode(&full[..cut]).unwrap().0.save().unwrap(),prior_state);}
 let restored=decode(&full).unwrap().0;assert_eq!(restored.save().unwrap(),e.save().unwrap());assert_eq!(restored.run.inventory_display_order,e.run.inventory_display_order);assert_eq!(restored.run.run_id,e.run.run_id);
}
