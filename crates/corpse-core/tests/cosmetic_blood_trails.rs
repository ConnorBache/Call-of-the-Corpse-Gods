//! Cosmetic native trails must not change movement, RNG, hazard rules or legacy saves.
use corpse_core::{Run, storage, travel};
fn room(kind: &str, revision: f64) -> (Run, f64, usize) {
    let mut run = storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    run.simulation_revision = revision.into();
    let actor = &mut std::sync::Arc::make_mut(run.actors.get_mut("1").unwrap())[0];
    actor.creature_kind = kind.to_string().into();
    actor.name = kind.to_string();
    actor.abilities.clear();
    actor.conditions.clear();
    actor.x = 20.; actor.y = 10.; actor.dead = false.into();
    let id = actor.id;
    let at = 10 * run.floor().unwrap().width() + 20;
    let floor = run.floor_mut().unwrap();
    floor.tiles[at] = "floor".into();
    floor.blood = Default::default();
    (run,id,at)
}
#[test]
fn family_footprints_only_change_existing_cosmetic_layer() {
    for kind in ["hemogoblin", "hemogoblin-bloodmancer", "hemogoblin-inventor"] {
        let (mut current,id,at) = room(kind,31.);
        let mut legacy = current.clone(); legacy.simulation_revision = 30.0.into();
        travel::enemy_moved(&mut current,id).unwrap();
        travel::enemy_moved(&mut legacy,id).unwrap();
        assert!(current.floor().unwrap().blood.as_ref().unwrap()[at]);
        assert!(legacy.floor().unwrap().blood.is_missing());
        let saved = serde_json::to_string(&current).unwrap();
        let restored: Run = serde_json::from_str(&saved).unwrap();
        assert_eq!(restored,current);
        // Same gameplay and RNG bytes, including dirty/offscreen scheduling flags.
        current.floor_mut().unwrap().blood = Default::default();
        current.simulation_revision = 30.0.into();
        assert_eq!(current,legacy,"{kind}: noncosmetic mutation");
    }
}
#[test]
fn no_trails_for_mech_nonfamily_dead_or_nonfloor_tiles() {
    for kind in ["hemogoblin-mech", "rat"] {
        let (mut run,id,_) = room(kind,31.);
        travel::enemy_moved(&mut run,id).unwrap();
        assert!(run.floor().unwrap().blood.is_missing());
    }
    for tile in ["water", "shallow-water", "grass", "up", "down", "chasm"] {
        let (mut run,id,at) = room("hemogoblin",31.);
        run.floor_mut().unwrap().tiles[at] = tile.into();
        travel::enemy_moved(&mut run,id).unwrap();
        assert!(run.floor().unwrap().blood.is_missing(),"{tile}");
    }
    let (mut run,id,_) = room("hemogoblin",31.);
    corpse_core::engine::actor_mut(&mut run,id).unwrap().dead = true.into();
    travel::enemy_moved(&mut run,id).unwrap();
    assert!(run.floor().unwrap().blood.is_missing());
}
#[test]
fn repeated_movement_preserves_prior_stains_without_rng() {
    let (mut run,id,first) = room("hemogoblin",31.);
    travel::enemy_moved(&mut run,id).unwrap();
    corpse_core::engine::actor_mut(&mut run,id).unwrap().x += 1.;
    run.floor_mut().unwrap().tiles[first+1] = "floor".into();
    travel::enemy_moved(&mut run,id).unwrap();
    let once = run.clone();
    travel::enemy_moved(&mut run,id).unwrap();
    assert_eq!(run,once);
    let stains = run.floor().unwrap().blood.as_ref().unwrap();
    assert!(stains[first] && stains[first+1]);
    assert_eq!(stains.iter().filter(|v| **v).count(),2);
}
#[test]
fn normal_and_movement_only_steps_share_cosmetic_hook_without_other_changes() {
    for movement_only in [false,true] {
        let (mut native,id,from)=room("hemogoblin",31.);
        native.floor_mut().unwrap().tiles[from+1]="floor".into();
        let mut legacy=native.clone(); legacy.simulation_revision=30.0.into();
        let moved=corpse_core::engine::move_actor(&mut native,id,1.,0.,movement_only).unwrap();
        let old_moved=corpse_core::engine::move_actor(&mut legacy,id,1.,0.,movement_only).unwrap();
        assert!(moved && old_moved);
        assert!(native.floor().unwrap().blood.as_ref().unwrap()[from+1]);
        native.floor_mut().unwrap().blood=Default::default();
        native.simulation_revision=30.0.into();
        assert_eq!(native,legacy);
    }
}
#[test]
fn temporal_snapshot_restores_stain_layer_without_sharing_future_mutation() {
    let (mut run,id,at)=room("hemogoblin",31.);
    let before=corpse_core::temporal::Snapshot::capture(&run);
    travel::enemy_moved(&mut run,id).unwrap();
    let after=corpse_core::temporal::Snapshot::capture(&run);
    assert!(before.state.floor().unwrap().blood.is_missing());
    assert!(after.state.floor().unwrap().blood.as_ref().unwrap()[at]);
    before.restore(&mut run).unwrap();
    assert!(run.floor().unwrap().blood.is_missing());
    after.restore(&mut run).unwrap();
    assert!(run.floor().unwrap().blood.as_ref().unwrap()[at]);
}
