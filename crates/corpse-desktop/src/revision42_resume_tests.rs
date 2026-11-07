// Include inside recording_resume::tests after core/header42 are merged.
#[test]fn both_41_and_42_clean_recordings_resume_without_mechanics_upgrade(){
 for revision in [41.0,42.0]{
  let dir=Sandbox::new();let path=dir.path();let base=fixture();let mut run=base.run.clone();run.simulation_revision=revision.into();let e=Engine::new(run).unwrap();
  Autosave::default().flush_tracked(&path,&e,Kind::Fixture,false).unwrap();let before=std::fs::read(&path).unwrap();
  let(mut loaded,k)=load(&path).unwrap();assert!(!loaded.pending_batch().checkpoint_required);assert_eq!(loaded.run.simulation_revision,revision.into());
  loaded.dispatch(&Command::Wait).unwrap();Autosave::default().flush_tracked(&path,&loaded,k,false).unwrap();
  let after=std::fs::read(&path).unwrap();assert!(after.starts_with(&before));assert_eq!(log::inspect_header(&after).unwrap().simulation_revision,revision as u32);equal_loaded(&path,&loaded);
 }
}
