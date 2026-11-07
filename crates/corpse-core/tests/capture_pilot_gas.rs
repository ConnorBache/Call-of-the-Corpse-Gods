//! Selected four-column gas pilot oracles; no persistence or whole-world coverage claim.
use corpse_core::{Run,environment,capture_pilot::{self,Scope,Change}};

/// O(generated cells), normalize outside collection so scalar tests do not hide hydration.
fn fixture()->Run {
    let mut r=corpse_core::startup::create_run("02-04-09:timing-r39-1","Blessed",false).unwrap();
    let e=environment::initialize(r.floor_mut().unwrap()).unwrap();
    e.gas_type.as_mut().unwrap().fill(None);
    e.gas_volume.as_mut().unwrap().fill(0.);
    e.gas_owner.as_mut().unwrap().fill(-1.);
    e.gas_player.as_mut().unwrap().fill(0.);
    environment::refresh_gas_views(e);
    r
}
/// O(N) setup-only search for a valid known producer target.
fn cell(r:&Run)->usize {let f=r.floor().unwrap();(0..f.tiles.len()).find(|&i|f.tiles[i]=="floor" && environment::gas_passable(f,i) && f.environment.as_ref().unwrap().fire[i]==0.).unwrap()}
/// O(selected changes * tuple bytes), stable diagnostic ordering for comparisons.
fn only_gas(changes:Vec<Change>)->Vec<Change>{changes.into_iter().filter(|c|matches!(c,Change::Gas{..})).collect()}

#[test]
fn direct_deposits_count_attempts_coalesce_and_read_final_master_tuple(){
    let before=fixture();let at=cell(&before);let mut after=before.clone();
    let scope=Scope::begin();
    environment::add_gas(after.floor_mut().unwrap(),at,"caustic",5.,-2.).unwrap();
    environment::add_gas(after.floor_mut().unwrap(),at,"caustic",7.,-2.).unwrap();
    let report=scope.finish(&before,&after);
    assert!(report.unsupported.is_empty(),"{:?}",report.unsupported);
    assert_eq!(report.counters.gas_write_attempts,8);
    assert_eq!(report.counters.gas_candidates,1);
    assert_eq!(only_gas(capture_pilot::normalize_changes(&report.changes).unwrap()),only_gas(capture_pilot::selected_oracle(&before,&after)));
    assert_eq!(after.floor().unwrap().environment.as_ref().unwrap().gas_volume.as_ref().unwrap()[at],12.);
}

#[test]
fn diffusion_bulk_owners_and_player_columns_match_selected_fullscan(){
    let mut before=fixture();let at=cell(&before);
    environment::add_gas(before.floor_mut().unwrap(),at,"caustic",100.,-2.).unwrap();
    let mut after=before.clone();let n=after.floor().unwrap().tiles.len();let mut rng=1u32;
    let scope=Scope::begin();
    environment::propagate_gas(after.floor_mut().unwrap(),&mut rng,&vec![false;n]).unwrap();
    let report=scope.finish(&before,&after);
    assert!(report.unsupported.is_empty(),"{:?}",report.unsupported);
    assert!(report.counters.gas_write_attempts>=6*n as u64,"two passes each write volume+owner+player");
    assert!(report.counters.gas_candidates<=n);
    assert_eq!(only_gas(capture_pilot::normalize_changes(&report.changes).unwrap()),only_gas(capture_pilot::selected_oracle(&before,&after)));
}

#[test]
fn discarded_physical_proxy_marks_cannot_export_its_uncommitted_values(){
    let before=fixture();let at=cell(&before);let mut discarded=before.clone();
    let scope=Scope::begin();environment::add_gas(discarded.floor_mut().unwrap(),at,"steam",50.,-1.).unwrap();
    let report=scope.finish(&before,&before);
    assert_eq!(report.counters.gas_candidates,1);
    assert!(report.changes.is_empty());
}

#[test]
fn enabled_collection_does_not_change_gas_or_rng_results(){
    let mut before=fixture();let at=cell(&before);
    environment::add_gas(before.floor_mut().unwrap(),at,"steam",75.,-2.).unwrap();
    let n=before.floor().unwrap().tiles.len();let mut off=before.clone();let mut on=before.clone();let(mut off_rng,mut on_rng)=(77u32,77u32);
    environment::propagate_gas(off.floor_mut().unwrap(),&mut off_rng,&vec![false;n]).unwrap();
    let scope=Scope::begin();environment::propagate_gas(on.floor_mut().unwrap(),&mut on_rng,&vec![false;n]).unwrap();
    let report=scope.finish(&before,&on);
    assert_eq!(off_rng,on_rng);
    assert_eq!(serde_json::to_vec(&off).unwrap(),serde_json::to_vec(&on).unwrap());
    assert_eq!(only_gas(capture_pilot::normalize_changes(&report.changes).unwrap()),only_gas(capture_pilot::selected_oracle(&before,&on)));
}

#[test]
fn gas_primary_hydration_is_explicitly_unsupported_not_claimed_as_indexed_capture(){
    let mut before=fixture();before.floor_mut().unwrap().environment.as_mut().unwrap().gas_owner=corpse_core::Field::Missing;
    let mut after=before.clone();let scope=Scope::begin();environment::initialize(after.floor_mut().unwrap()).unwrap();
    let report=scope.finish(&before,&after);
    assert!(report.unsupported.iter().any(|s|s=="gas_primary_column_initialization"));
}

#[test]
fn gas_consumption_by_fire_marks_the_two_known_primary_writes(){
    let mut before=fixture();let at=cell(&before);
    environment::add_gas(before.floor_mut().unwrap(),at,"flammable",50.,-2.).unwrap();
    let mut after=before.clone();let n=after.floor().unwrap().tiles.len();
    let scope=Scope::begin();
    {let mut adapter=corpse_core::fire_adapter::FireAdapter::new(&mut after,vec![false;n]).unwrap();adapter.ignite_creature(at,-1.,0.);adapter.resolve().unwrap();}
    let report=scope.finish(&before,&after);
    assert!(report.unsupported.is_empty(),"{:?}",report.unsupported);
    assert!(report.counters.gas_write_attempts>=2);
    assert_eq!(after.floor().unwrap().environment.as_ref().unwrap().gas_volume.as_ref().unwrap()[at],0.);
    assert_eq!(only_gas(capture_pilot::normalize_changes(&report.changes).unwrap()),only_gas(capture_pilot::selected_oracle(&before,&after)));
}

/// O(N) setup/oracle, O(1) selected collector; test each Field presence and exact IEEE bit case.
#[test]
fn typed_comparison_matches_frozen_verbose_presence_and_ieee_oracle(){
    use corpse_core::Field;
    for mode in 0..10 {
        let before=fixture();let at=cell(&before);let depth=before.depth.to_bits();let mut after=before.clone();
        match mode {
            0=>after.floor_mut().unwrap().environment=Field::Missing,
            1=>after.floor_mut().unwrap().environment=Field::Null,
            2=>after.floor_mut().unwrap().environment.as_mut().unwrap().gas_type=Field::Missing,
            3=>after.floor_mut().unwrap().environment.as_mut().unwrap().gas_type=Field::Null,
            4=>after.floor_mut().unwrap().environment.as_mut().unwrap().gas_type.as_mut().unwrap()[at]=Some("steam".into()),
            5=>after.floor_mut().unwrap().environment.as_mut().unwrap().gas_volume=Field::Missing,
            6=>after.floor_mut().unwrap().environment.as_mut().unwrap().gas_volume=Field::Null,
            7=>after.floor_mut().unwrap().environment.as_mut().unwrap().gas_volume.as_mut().unwrap()[at]=-0.,
            8=>after.floor_mut().unwrap().environment.as_mut().unwrap().gas_owner.as_mut().unwrap()[at]=f64::from_bits(0x7ff8_0000_0000_1234),
            _=>after.floor_mut().unwrap().environment.as_mut().unwrap().gas_player.as_mut().unwrap()[at]=f64::INFINITY,
        }
        let scope=Scope::begin();capture_pilot::mark_gas(depth,at);let report=scope.finish(&before,&after);
        assert_eq!(report.changes.len(),1,"mode {mode}");assert!(report.unsupported.is_empty());
        let normalized=capture_pilot::normalize_changes(&report.changes).unwrap();let expected:Vec<_>=capture_pilot::selected_oracle(&before,&after).into_iter().filter(|c|matches!(c,Change::Gas{index,..} if *index==at)).collect();assert_eq!(normalized,expected,"mode {mode}");
        let encoded=serde_json::to_vec(&report.changes).unwrap();assert_eq!(capture_pilot::normalize_encoded_changes(&encoded).unwrap(),expected);
    }
}
/// O(N) fixture, O(1) collection; unchanged borrowed reads emit no JSON values.
#[test]
fn typed_noop_nan_payload_and_shape_rejection(){
    let mut before=fixture();let at=cell(&before);let depth=before.depth.to_bits();before.floor_mut().unwrap().environment.as_mut().unwrap().gas_volume.as_mut().unwrap()[at]=f64::from_bits(0x7ff8_0000_0000_4567);
    let scope=Scope::begin();capture_pilot::mark_gas(depth,at);let report=scope.finish(&before,&before);assert!(report.changes.is_empty());assert!(report.unsupported.is_empty());
    let mut after=before.clone();after.floor_mut().unwrap().environment.as_mut().unwrap().gas_volume.as_mut().unwrap().clear();let scope=Scope::begin();capture_pilot::mark_gas(depth,at);let report=scope.finish(&before,&after);assert!(report.changes.is_empty());assert!(report.unsupported.iter().any(|s|s.starts_with("gas_address_or_shape")));
}
/// O(payload bytes), malformed diagnostic tuples fail instead of guessing presence/bits.
#[test]
fn compact_normalizer_rejects_invalid_tags_types_and_lengths(){
    for value in [serde_json::json!([9]),serde_json::json!([0,1]),serde_json::json!([2,[2,5],[0],[0],[0]]),serde_json::json!([2,[2,null],[2,-1],[0],[0]])]{let change=Change::Gas{depth_bits:1f64.to_bits(),index:0,value};assert!(capture_pilot::normalize_changes(&[change]).is_err());}
}
