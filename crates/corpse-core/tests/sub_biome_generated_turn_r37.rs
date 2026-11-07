//! Independent regression: generated biome actors must pass real command admission.
use corpse_core::{startup, storage, temporal::Engine, Command};
#[test]
fn generated_corn_floor_wait_and_save_resume_execute_identically() {
    let mut run=startup::new_run_shell_revision("sub-biome-review-1","Unblessed",false,37.).unwrap();
    startup::ensure_floors(&mut run,40).unwrap();
    let (depth,cells,w)=run.floors.iter().find_map(|f|f.pocket_biomes.as_ref().into_iter().flatten()
        .find(|b|b.theme=="corn-dimension").map(|b|(f.depth,b.cells.clone(),f.width())))
        .expect("fixed review seed must exercise a generated Corn dimension");
    let free=cells.into_iter().find(|i|!run.actors_at(depth).iter()
        .any(|a|a.x==(*i%w)as f64&&a.y==(*i/w)as f64)).unwrap();
    assert!(run.actors_at(depth).iter().any(|a|a.conditions.get("cornDimensionResident")==Some(&1.)));
    run.depth=depth;run.player.x=(free%w)as f64;run.player.y=(free/w)as f64;
    // Keep this admission/replay fixture alive while deep monsters take real turns.
    run.player.hp=100000.;run.player.max_hp=100000.;
    let mut original=Engine::new(run).unwrap();
    let saved=original.save().unwrap();let mut restored=Engine::load(&saved).unwrap();
    for command in [Command::Wait,Command::Move{dx:1.,dy:0.},Command::Wait] {
        let before=original.run.tick;
        let a=original.dispatch(&command).unwrap();let b=restored.dispatch(&command).unwrap();
        assert_eq!(a.took_turn,b.took_turn);
        if matches!(command,Command::Wait){assert!(a.took_turn);assert!(original.run.tick>before);}
        assert!(storage::canonical_equal(&serde_json::to_value(original.canonical_run().unwrap()).unwrap(),
            &serde_json::to_value(restored.canonical_run().unwrap()).unwrap()));
    }
}
