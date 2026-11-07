//! Exact inverse-byte regressions for the staged coarse mutation journal.
//! Synthetic projection fixtures test serialization only, not valid gameplay admission.
#[allow(dead_code)]
#[path = "../../corpse-desktop/examples/save_envelope_bench/frozen_history.rs"]
mod frozen;
use corpse_core::{Actor, Command, Field, Floor, GroundItem, Run, storage,
    temporal::{self, Engine, Snapshot}};
use serde_json::{Value, json};
use std::sync::Arc;

/// O(N) small structured block construction, N=64 cells in these tests.
fn floor(depth: f64) -> Arc<Floor> {
    Arc::new(Floor { depth, width: 8.0.into(), height: 8.0.into(),
        tiles: vec!["floor".into();64], lights: vec![1.0;64], seen: vec![false;64],
        ..Floor::default() })
}
/// O(F+A+G) fixed-size synthetic root setup; no simulation is dispatched on it.
fn roots() -> Run {
    let mut run=Run::default();
    run.floors=vec![floor(1.),floor(2.),floor(3.)];
    for i in 1..=3 {
        run.actors.insert(i.to_string(),Arc::new(vec![Actor {id:i as f64,name:format!("actor {i}"),hp:10.,..Actor::default()}]));
        let mut ground=GroundItem::default();ground.item.id=(100+i) as f64;
        run.ground.insert(i.to_string(),Arc::new(vec![ground]));
    }
    run
}
/// O(full snapshot bytes * nesting + patches + archive cursor lengths).
/// Independent frozen walk oracle; real archive contents are tested by exact_engine.
/// Tests both logging modes, exact embedded JSON bytes and patch application.
fn exact(before: &Snapshot, after: &Snapshot) -> Value {
    let mut last=Value::Null;
    for retain_log in [false,true] {
        let expected=frozen::inverse(before,after,retain_log).unwrap();
        let actual=temporal::inverse(before,after,retain_log).unwrap();
        let expected_value=serde_json::to_value(&expected).unwrap();
        let actual_value=serde_json::to_value(&actual).unwrap();
        assert_eq!(frozen::js_stringify(&actual_value),frozen::js_stringify(&expected_value));
        let mut reference=serde_json::to_value(&*after.state).unwrap();
        // Snapshot intentionally retains only archive cursors, not archive payload.
        // Supply the structural prefix expected by BOTH patch interpreters here;
        // valid Engine save/reload checks below still use the real archive data.
        if let Some(cursor)=after.cursor {
            reference["logArchive"]=json!({
                "messages":vec![Value::Null;cursor.messages],
                "events":vec![Value::Null;cursor.events],
            });
        }
        let mut candidate=reference.clone();
        let reference_patches:Vec<temporal::Patch>=serde_json::from_value(expected_value).unwrap();
        temporal::apply(&mut reference,&reference_patches).unwrap();
        temporal::apply(&mut candidate,&actual).unwrap();
        assert_eq!(candidate,reference);
        last=actual_value;
    }
    last
}
/// O(F*S*L) across a fixed number of structural cases; no runtime journal required.
#[test]
fn shared_floor_append_truncate_reorder_and_replacement_match_full_inverse() {
    let base=roots();let before=Snapshot::capture(&base);
    exact(&before,&Snapshot::capture(&base));
    let mut after=base.clone();after.floors.push(floor(4.));
    exact(&before,&Snapshot::capture(&after));
    let mut after=base.clone();after.floors.truncate(1);
    exact(&before,&Snapshot::capture(&after));
    let mut after=base.clone();after.floors.swap(0,2);
    exact(&before,&Snapshot::capture(&after));
    let mut after=base.clone();after.floors[1]=floor(9.);
    exact(&before,&Snapshot::capture(&after));
    let mut after=base.clone();after.floors.clear();
    exact(&before,&Snapshot::capture(&after));
    exact(&Snapshot::capture(&after),&before);
}
/// O(N*L + patch candidates), verifying real scalar arrays retain compactness rules.
#[test]
fn changed_floor_scalar_arrays_keep_whole_array_choice_and_net_zero_semantics() {
    let base=roots();let before=Snapshot::capture(&base);let mut after=base.clone();
    Arc::make_mut(&mut after.floors[1]).tiles.fill("wall".into());
    let patches=exact(&before,&Snapshot::capture(&after));
    assert!(patches.as_array().unwrap().iter().any(|p|p["path"]==json!(["floors",1,"tiles"])&&p.get("value").is_some()),
        "fixture must exercise legacy whole scalar-array compaction");
    Arc::make_mut(&mut after.floors[1]).tiles.fill("floor".into());
    let patches=exact(&before,&Snapshot::capture(&after));
    assert!(!patches.as_array().unwrap().iter().any(|p|p["path"].as_array().unwrap().first()==Some(&json!("floors"))));
}
/// O(group bytes * nesting) over fixed map/vector structure cases; keeps map order.
#[test]
fn actor_and_ground_map_structure_order_and_net_zero_match_full_inverse() {
    let base=roots();let before=Snapshot::capture(&base);
    let mut after=base.clone();
    after.actors.insert("4".into(),Arc::new(vec![Actor{id:4.,..Actor::default()}]));
    after.ground.insert("4".into(),Arc::new(vec![]));
    exact(&before,&Snapshot::capture(&after));
    let mut after=base.clone();after.actors.shift_remove("2");after.ground.shift_remove("2");
    exact(&before,&Snapshot::capture(&after));
    let mut after=base.clone();
    let actors=after.actors.shift_remove("1").unwrap();after.actors.insert("1".into(),actors);
    let ground=after.ground.shift_remove("1").unwrap();after.ground.insert("1".into(),ground);
    exact(&before,&Snapshot::capture(&after));
    let mut after=base.clone();
    let actors=Arc::make_mut(after.actors.get_mut("1").unwrap());
    actors[0].hp=5.;actors[0].hp=10.;actors.push(Actor{id:90.,..Actor::default()});actors.pop();
    let ground=Arc::make_mut(after.ground.get_mut("1").unwrap());
    ground[0].x=7.;ground[0].x=0.;ground.push(GroundItem::default());ground.pop();
    exact(&before,&Snapshot::capture(&after));
    let mut after=base.clone();
    Arc::make_mut(after.actors.get_mut("1").unwrap()).insert(0,Actor{id:99.,..Actor::default()});
    Arc::make_mut(after.ground.get_mut("1").unwrap()).clear();
    exact(&before,&Snapshot::capture(&after));
}
/// O(dream floor bytes * nesting), covering structural Field presence and dirty blocks.
#[test]
fn dream_creation_removal_and_shared_or_changed_layers_match_full_inverse() {
    let absent=roots();let mut present=absent.clone();
    present.dreamwalking=corpse_core::dreamwalking::State{
        floors:vec![floor(1.),floor(2.)],depth:1.,unlocked:true,
        ..Default::default()
    }.into();
    let no=Snapshot::capture(&absent);let yes=Snapshot::capture(&present);
    exact(&no,&yes);exact(&yes,&no);exact(&yes,&yes);
    let mut changed=present.clone();
    changed.dreamwalking.as_mut().unwrap().anchor_x=8.;
    exact(&yes,&Snapshot::capture(&changed));
    Arc::make_mut(&mut changed.dreamwalking.as_mut().unwrap().floors[0]).seen.fill(true);
    exact(&yes,&Snapshot::capture(&changed));
    changed.dreamwalking.as_mut().unwrap().floors.push(floor(3.));
    exact(&yes,&Snapshot::capture(&changed));
    changed.dreamwalking.as_mut().unwrap().floors.swap(0,1);
    exact(&yes,&Snapshot::capture(&changed));
    changed.dreamwalking=Field::Null;
    exact(&yes,&Snapshot::capture(&changed));
}
/// O(H*S*L + encode/reload), validating captured journals and every retained edge.
fn exact_engine(engine: &Engine) {
    let mut newer=engine.history.pending_haste.as_ref().or(engine.history.closing.as_ref()).unwrap();
    for older in engine.history.frames.iter().rev() {exact(older,newer);newer=older;}
    let expected=storage::serialize(&frozen::export(engine)).unwrap();
    assert!(engine.save().unwrap()==expected,"journal save differs from frozen export");
    let mut resumed=Engine::load(&expected).unwrap();
    resumed.set_legacy_revert(true); // This test compares the frozen legacy export codec.
    assert!(resumed.save().unwrap()==expected,"valid timeline reload differs");
}
/// O(commands * (dispatch + H*S*L)), real writes, free edits, eviction and five undos.
#[test]
fn actual_captured_command_journals_save_and_five_undos_match_frozen_reference() {
    let mut engine=Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    engine.run.player.abilities.push("numo-revert".into());
    engine.run.player.conditions.insert("revertCharges".into(),10.);
    engine=Engine::new(engine.run).unwrap();
    engine.set_legacy_revert(true); // Keep the explicitly frozen inverse/export oracle.
    for command in [Command::Wait,Command::Unequip{id:1.},Command::Equip{id:1.,slot:"hand1".into()},
        Command::Drop{id:2.},Command::Pickup,Command::Wait,Command::Wait,Command::Wait,Command::Wait,Command::Wait] {
        engine.dispatch(&command).unwrap();exact_engine(&engine);
    }
    assert_eq!(engine.history.frames.len(),5);
    for expected_remaining in (0..5).rev() {
        engine.dispatch(&Command::Ability{id:"numo-revert".into(),target:Field::Missing,x:Field::Missing,y:Field::Missing}).unwrap();
        assert_eq!(engine.history.frames.len(),expected_remaining);
        exact_engine(&engine);
    }
    assert!(engine.history.frames.is_empty());
}
