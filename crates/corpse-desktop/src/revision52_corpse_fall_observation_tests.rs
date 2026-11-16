use super::observe;
use corpse_core::{Actor, Command, Field, Run, Stats, engine};

#[test]
fn command_settled_flying_death_disappears_from_visible_chasm_and_appears_below() {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 52.0.into(); r.floors.truncate(3); r.depth = 1.; r.tick = 0.; r.next_id = 2000.;
    r.actors = Default::default(); r.ground.clear(); r.inventory.clear();
    r.player.x = 10.; r.player.y = 10.; r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.stats = Stats { perception: 300., max_energy: 100., ..Default::default() };
    r.player.ability_order = Field::Missing; r.player.equipment_abilities = Field::Missing;
    r.player.ability_reserves = Field::Missing; r.player.amulet_ledger = Field::Missing;
    for floor in &mut r.floors {
        let f = std::sync::Arc::make_mut(floor);
        f.tiles.fill("floor".into()); f.lights.fill(100.); f.seen.fill(true);
        f.environment = Field::Missing; f.currents = Field::Missing; f.vents = Field::Missing;
        f.roots = Field::Missing; f.fungal_roots = Field::Missing; f.ichor = Field::Missing;
        f.mud = Field::Missing; f.slime = Field::Missing; f.razorleaves = Field::Missing;
        f.doors = Field::Missing; f.grates = Field::Missing; f.portals = Field::Missing;
        f.non_euclidean = Field::Missing; f.light_sources = Field::Missing;
        f.permanent_fire = Field::Missing; f.godbeast_event = Field::Missing;
    }
    for d in 1..=3 { r.actors_at_mut(d as f64); r.ground_at_mut(d as f64); }
    let i = 10 * r.floor().unwrap().width() + 12; r.floor_mut().unwrap().tiles[i] = "chasm".into();
    let mut a = Actor { id: 800., x: 12., y: 10., name: "Cave Bat".into(), glyph: "b".into(),
        hp: 1., max_hp: 100., energy: 100., state: "sleeping".into(), heartless: true.into(),
        abilities: vec!["enemy:beast:flight".into()], stats: Stats { max_energy: 100., ..Default::default() },
        ..Default::default() };
    a.conditions.insert("bestiaryTraitsInitialized".into(), 1.);
    a.conditions.insert("paleTraitsInitialized".into(), 1.);
    r.actors_at_mut(1.).push(a);
    let before = observe(&r); let live = before.cells.iter().find(|c| c.actor_id == Some(800.)).unwrap();
    assert!(!live.corpse); assert!(before.creatures.iter().any(|a| a.id == 800.));
    // Exercise full command-boundary death settlement and fresh detached view,
    // rather than directly editing the corpse flag or just checking storage.
    r.actors_at_mut(1.)[0].hp = 0.;
    assert!(engine::dispatch(&mut r, &Command::Wait).unwrap().took_turn);
    let upper = observe(&r);
    assert!(upper.cells.iter().any(|c| c.x == 12 && c.y == 10 && c.current));
    assert!(!upper.cells.iter().any(|c| c.actor_id == Some(800.)));
    assert!(!upper.creatures.iter().any(|a| a.id == 800.));
    assert!(r.actors_at(1.).is_empty()); assert_eq!(r.actors_at(2.).len(), 1);
    r.depth = 2.; let lower = observe(&r);
    let fallen = lower.cells.iter().find(|c| c.actor_id == Some(800.)).unwrap();
    assert!(fallen.corpse); assert_eq!((fallen.x, fallen.y), (12, 10));
    let dead = engine::actor(&r, 800.).unwrap(); assert!(dead.dead.value_or(false));
    assert!(!engine::flying_in(&r, dead));
    assert_eq!(r.actors.values().flat_map(|v| v.iter()).filter(|a| a.id == 800.).count(), 1);
}
