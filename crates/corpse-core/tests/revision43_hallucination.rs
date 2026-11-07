//! Revision43 exposure is player-owned; legacy identity projection stays versioned.
use corpse_core::{Actor, Field, Run, engine, environment};
fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into();
    r.player.x = 20.; r.player.y = 10.;
    r.player.conditions.clear(); r.player.abilities.clear(); r.player.active.clear();
    r.inventory.clear(); r.actors_at_mut(1.).clear(); r.phase = Field::Missing;
    let f = r.floor_mut().unwrap();
    f.environment = Field::Missing; f.fungal_roots = Field::Missing;
    f.roots = Field::Missing; f.mud = Field::Missing; f.slime = Field::Missing;
    f.ichor = Field::Missing; f.razorleaves = Field::Missing; f.water_blood = Field::Missing;
    f.kelpie_blood_sources = Field::Missing; f.currents = Field::Missing;
    f.vents = Field::Missing; f.permanent_fire = Field::Missing;
    let w=f.width(); let h=f.height();
    for y in 1..h-1 { for x in 1..w-1 { f.tiles[y*w+x]="floor".into(); } }
    r
}
fn add_actor(r: &mut Run, id: f64, x: f64, state: &str) -> Actor {
    let mut a = r.player.clone(); a.id=id; a.name="Gas witness".into();
    a.x=x; a.state=state.into(); a.dead=false.into();
    a.conditions.insert("bestiaryTraitsInitialized".into(),1.);
    a.conditions.insert("paleTraitsInitialized".into(),1.);
    r.actors_at_mut(1.).push(a.clone()); a
}
fn gas(r: &mut Run, x: usize) {
    let i=10*r.floor().unwrap().width()+x;
    environment::add_gas(r.floor_mut().unwrap(),i,"hallucinogenic",1000.,-1.).unwrap();
}
#[test]
fn revision43_excludes_enemies_and_allies_but_legacy_exposes_them() {
    let mut diffusion_results = Vec::new();
    for revision in [42.,43.] {
        let mut r=room(revision); add_actor(&mut r,200.,21.,"hunting"); add_actor(&mut r,201.,22.,"ally");
        for x in [20,21,22] { gas(&mut r,x); }
        let i=10*r.floor().unwrap().width()+21;
        assert_eq!(environment::creature_hazard_at(&r,engine::actor(&r,200.).unwrap(),i),revision<43.);
        assert!(environment::creature_hazard_at(&r,&r.player,i));
        // The existing gas propagation consumes RNG (GasPass::random_int).
        // Compare identical42/43 streams, not the invalid zero-draw assumption.
        let rng_before=r.rng; environment::tick_environment(&mut r).unwrap();
        diffusion_results.push((rng_before,r.rng,r.floor().unwrap().environment.clone()));
        assert_eq!(engine::condition(&r.player,"hallucinating"),2.);
        for id in [200.,201.] {
            assert_eq!(engine::condition(engine::actor(&r,id).unwrap(),"hallucinating"),if revision<43. {2.} else {0.});
        }
    }
    assert_eq!(diffusion_results[0],diffusion_results[1],
        "the player-only eligibility guard adds no RNG draws or gas propagation changes");
}
#[test]
fn revision43_retains_shared_owner_endpoint_exposure_and_immunity() {
    let mut r=room(43.); let owner=r.player.id;
    r.player.abilities.push(corpse_core::half_here::ID.into());
    r.player.active.push(corpse_core::half_here::ID.into());
    r.player.conditions.insert("halfHereId".into(),200.);
    add_actor(&mut r,200.,23.,"ally");
    let a=engine::actor_mut(&mut r,200.).unwrap();
    a.conditions.insert("halfHereClone".into(),1.); a.conditions.insert("halfHereOwner".into(),owner);
    gas(&mut r,23);
    let i=10*r.floor().unwrap().width()+23;
    assert!(environment::creature_hazard_at(&r,engine::actor(&r,200.).unwrap(),i));
    environment::tick_environment(&mut r).unwrap();
    assert_eq!(engine::condition(engine::actor(&r,200.).unwrap(),"hallucinating"),2.);
    assert_eq!(engine::condition(&r.player,"hallucinating"),0.,"body-local status is not transferred");
    let mut immune=room(43.); immune.player.abilities.push("enemy:pale:gas-immunity".into()); gas(&mut immune,20);
    environment::tick_environment(&mut immune).unwrap();
    assert_eq!(engine::condition(&immune.player,"hallucinating"),0.);
}
#[test]
fn revision43_world_turn_decay_and_save_restore_remain_exact() {
    let mut r=room(43.); gas(&mut r,20); environment::tick_environment(&mut r).unwrap();
    r.floor_mut().unwrap().environment=Field::Missing;
    let saved=corpse_core::storage::serialize(&r).unwrap();
    let mut restored=corpse_core::storage::deserialize(&saved).unwrap();
    assert_eq!(engine::condition(&restored.player,"hallucinating"),2.);
    environment::tick_environment(&mut restored).unwrap();
    assert_eq!(engine::condition(&restored.player,"hallucinating"),1.);
    environment::tick_environment(&mut restored).unwrap();
    assert_eq!(engine::condition(&restored.player,"hallucinating"),0.);
}
#[test]
fn revision43_sleeping_body_remains_player_owned_without_admitting_other_clones() {
    let mut r=room(43.); add_actor(&mut r,200.,23.,"ally"); add_actor(&mut r,201.,24.,"ally");
    r.dreamwalking=corpse_core::dreamwalking::State { active:true, unlocked:true, depth:r.depth,
        body_id:Some(200.), ..Default::default() }.into();
    engine::actor_mut(&mut r,200.).unwrap().conditions.insert("phantasmiiBody".into(),1.);
    let foreign=engine::actor_mut(&mut r,201.).unwrap();
    foreign.conditions.insert("halfHereClone".into(),1.);
    foreign.conditions.insert("halfHereOwner".into(),999.);
    gas(&mut r,23);
    let i=10*r.floor().unwrap().width()+23;
    assert!(environment::creature_hazard_at(&r,engine::actor(&r,200.).unwrap(),i));
    assert!(!environment::creature_hazard_at(&r,engine::actor(&r,201.).unwrap(),i));
}
