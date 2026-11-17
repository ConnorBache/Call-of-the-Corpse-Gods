use corpse_core::{Field, Run, Stats, engine, environment};
fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into();
    r.player.x = 10.; r.player.y = 10.; r.player.hp = 100.; r.player.max_hp = 100.;
    r.player.base_max_hp = 100.0.into(); r.player.energy = 37.;
    r.player.stats = Stats { offense: 10., dr: 2., accuracy: 100., perception: 300., max_energy: 100., ..Default::default() };
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.equipment_abilities = Field::Missing; r.player.ability_order = Field::Missing;
    r.player.ability_reserves = Field::Missing;
    r.inventory.clear();
    // Empty floor contents without removing blocks required by save validation.
    for actors in r.actors.values_mut() { *actors = Default::default(); }
    for ground in r.ground.values_mut() { *ground = Default::default(); }
    r.history.clear(); r.haste_snapshot = Field::Missing; r.future = None;
    r.phase = "normal".to_string().into(); r.next_id = 900.;
    let f = r.floor_mut().unwrap();
    f.tiles.fill("floor".into()); f.lights.fill(100.);
    f.doors = Field::Missing; f.portals = Field::Missing; f.environment = Field::Missing;
    f.non_euclidean = Field::Missing; f.speed_up = Field::Missing; f.currents = Field::Missing;
    f.fungal_roots = Field::Missing; f.roots = Field::Missing; f.mud = Field::Missing;
    f.slime = Field::Missing; f.ichor = Field::Missing; f.razorleaves = Field::Missing;
    f.water_blood = Field::Missing; f.kelpie_blood_sources = Field::Missing;
    f.vents = Field::Missing; f.permanent_fire = Field::Missing;
    r
}

fn exposed_room() -> Run {
    let mut r = room(52.);
    let mut tree = r.player.clone();
    tree.id = 777.; tree.x = 12.; tree.y = 10.;
    tree.name = "Devil's Samara Tree".into();
    tree.abilities = vec!["enemy:beast:razorleaves".into(), "enemy:beast:rooted".into()];
    tree.state = "sleeping".into();
    r.actors_at_mut(r.depth).push(tree);
    r
}

#[test]
fn razorleaves_use_maximum_not_remaining_health_for_player_and_enemy() {
    for hp in [100., 50., 10.] {
        let mut r = exposed_room();
        r.player.hp = hp;
        let mut enemy = r.player.clone();
        enemy.id = 778.; enemy.x = 11.; enemy.max_hp = 200.; enemy.hp = hp;
        enemy.state = "sleeping".into();
        r.actors_at_mut(r.depth).push(enemy);
        environment::tick_environment(&mut r).unwrap();
        assert_eq!(r.player.hp, hp - 2.);
        assert_eq!(engine::actor(&r, 778.).unwrap().hp, hp - 4.);
        assert_eq!(engine::actor(&r, 777.).unwrap().hp, 100., "Samara stays immune");
    }
}

#[test]
fn razorleaves_maximum_health_rule_survives_save_roundtrip_and_rounds_once() {
    let mut r = exposed_room();
    r.player.max_hp = 126.; r.player.base_max_hp = 126.0.into(); r.player.hp = 20.;
    let encoded = serde_json::to_string(&r).unwrap();
    let mut restored: Run = serde_json::from_str(&encoded).unwrap();
    environment::tick_environment(&mut r).unwrap();
    environment::tick_environment(&mut restored).unwrap();
    assert_eq!(r.player.hp, 17.); // round(126 * .02), not round(20 * .02)
    assert_eq!(restored.player.hp, r.player.hp);
}
