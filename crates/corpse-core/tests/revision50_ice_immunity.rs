//! Explicit R50 fixtures. No new cold damage, freezing status or amulet effect
//! is inferred, and these tests do not open the R50 release-admission gate.
use corpse_core::{Actor, Command, Field, Item, Run, Stats, engine, half_here,
    monster_chill, revision50_rings as rings, terrain_runtime, temporal::Snapshot};

fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into(); r.tick = 0.;
    r.player.x = 10.; r.player.y = 10.;
    r.player.hp = 100.; r.player.max_hp = 100.; r.player.base_max_hp = 100.0.into();
    r.player.energy = 100.;
    r.player.stats = Stats { perception: 300., max_energy: 100., ..Default::default() };
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.equipment_abilities = Field::Missing; r.player.ability_order = Field::Missing;
    r.player.ability_reserves = Field::Missing;
    r.inventory.clear(); r.actors = Default::default(); r.ground.clear();
    r.history.clear(); r.haste_snapshot = Field::Missing; r.future = None;
    r.phase = "haste".to_string().into(); r.next_id = 900.; r.dreamwalking = Field::Missing;
    let f = r.floor_mut().unwrap();
    f.tiles.fill("floor".into()); f.lights.fill(100.);
    f.doors = Field::Missing; f.portals = Field::Missing;
    f.non_euclidean = Field::Missing; f.speed_up = Field::Missing;
    f.ice = Field::Missing; f.snow = Field::Missing; f.puddles = Field::Missing;
    f.environment = Field::Missing;
    corpse_core::environment::initialize(f).unwrap();
    r
}
fn ring(id: f64) -> Item {
    Item { id, name: "Ring of Ice Immunity".into(), kind: "ring".into(), glyph: "=".into(),
        effect: rings::ICE_IMMUNITY.to_string().into(),
        ..Default::default() }
}
fn creature(id: f64, x: f64, y: f64, kind: &str) -> Actor {
    let mut a = Actor { id, x, y, name: kind.into(), creature_kind: kind.to_string().into(),
        glyph: "e".into(), color: "#ffffff".into(), hp: 100., max_hp: 100.,
        base_max_hp: 100.0.into(), energy: 100., state: "hunting".into(),
        stats: Stats { max_energy: 100., ..Default::default() },
        abilities: vec![format!("enemy:beast:{kind}")], ..Default::default() };
    // Keep action-clock fixtures stationary; Yeti adjacency is passive.
    a.conditions.insert("paralyzed".into(), 100.); a
}
fn ice(r: &mut Run, x: usize, y: usize) {
    let f = r.floor_mut().unwrap(); let n = f.tiles.len(); let at = y * f.width() + x;
    f.ice.get_or_insert_with(|| vec![false; n])[at] = true;
}
fn slip_seed(r: &Run) -> f64 {
    (1..1000).map(|seed| seed as f64).find(|seed| {
        let mut probe = r.clone(); probe.rng = *seed; corpse_core::rng::random(&mut probe) < 1. / 3.
    }).unwrap()
}

#[test]
fn every_carried_authored_r50_ring_prevents_yeti_chill_regardless_of_flags() {
    for mode in ["active", "duplicate", "missing", "null", "empty", "old-slot", "legacy", "wrong-kind", "old-name"] {
        let mut r = room(if mode == "legacy" { 49. } else { 50. });
        let mut item = ring(1.);
        match mode {
            "missing" => item.equipped = Field::Missing,
            "null" => item.equipped = Field::Null,
            "empty" => item.equipped = String::new().into(),
            "old-slot" => item.equipped = "ring2".to_string().into(),
            "wrong-kind" => item.kind = "amulet".into(),
            "old-name" => item.effect = "cold-resistance".to_string().into(),
            _ => {}
        }
        r.inventory.push(item);
        if mode == "duplicate" { r.inventory.push(ring(2.)); }
        r.actors_at_mut(r.depth).push(creature(901., 11., 11., "yeti"));
        let rng = r.rng;
        monster_chill::apply_adjacency(&mut r);
        assert_eq!(engine::condition(&r.player, "slowed"),
            if matches!(mode, "active" | "duplicate" | "missing" | "null" | "empty" | "old-slot") { 0. } else { 1. }, "{mode}");
        assert_eq!(r.rng, rng);
        if matches!(mode, "active" | "duplicate" | "missing" | "null" | "empty" | "old-slot") {
            assert!(!r.player.conditions.contains_key("slowSourceId"));
        } else {
            assert_eq!(r.player.conditions.get("slowSourceId"), Some(&901.));
        }
    }
}

#[test]
fn ice_immunity_prevents_slip_without_rng_or_terrain_changes_and_removal_restores_it() {
    let mut r = room(50.); r.inventory.push(ring(1.)); ice(&mut r, 10, 10);
    r.rng = slip_seed(&r); let rng = r.rng; let layer = r.floor().unwrap().ice.clone();
    let id = r.player.id;
    assert!(engine::move_actor(&mut r, id, 1., 0., true).unwrap());
    assert_eq!(r.player.x, 11.); assert_eq!(r.rng, rng);
    assert_eq!(r.floor().unwrap().ice, layer);
    r.player.x = 10.;
    assert_eq!(corpse_core::inventory::command(&mut r, &Command::Drop { id: 1. }).unwrap(), Some(true));
    assert!(r.inventory.is_empty());
    assert!(engine::move_actor(&mut r, id, 1., 0., true).unwrap());
    assert_eq!(r.player.x, 10.); assert_ne!(r.rng, rng);
    assert!(r.log.iter().any(|s| s == "You slip on the ice."));
    // Historical revisions keep their original footing roll despite the item.
    r.simulation_revision = 49.0.into();
    let mut old_ring = ring(1.); old_ring.equipped = "ring1".to_string().into(); r.inventory.push(old_ring);
    r.rng = rng;
    assert!(terrain_runtime::slip(&mut r, id));
}

#[test]
fn half_here_and_actor_local_footing_use_the_correct_equipment_owner() {
    let mut r = room(50.); r.inventory.push(ring(1.));
    r.player.abilities.push(half_here::ID.into());
    let clone = half_here::create(&mut r, 20., 10.).unwrap();
    r.actors_at_mut(r.depth).push(creature(901., 21., 10., "yeti"));
    monster_chill::apply_adjacency(&mut r);
    assert_eq!(engine::condition(&r.player, "slowed"), 0.);
    assert_eq!(engine::condition(engine::actor(&r, clone).unwrap(), "slowed"), 0.);
    ice(&mut r, 20, 10); let rng = r.rng;
    assert!(!terrain_runtime::slip(&mut r, clone)); assert_eq!(r.rng, rng);
    r.inventory.clear(); monster_chill::apply_adjacency(&mut r);
    assert_eq!(engine::condition(&r.player, "slowed"), 1.);
    assert_eq!(engine::condition(engine::actor(&r, clone).unwrap(), "slowed"), 1.);
    assert_eq!(r.player.conditions.get("slowSourceId"), Some(&901.));

    let mut other = creature(902., 30., 10., "rat"); other.abilities.clear();
    other.copied_inventory = vec![ring(3.)].into(); r.actors_at_mut(r.depth).push(other);
    ice(&mut r, 30, 10); r.rng = slip_seed(&r); let rng = r.rng;
    assert!(!terrain_runtime::slip(&mut r, 902.)); assert_eq!(r.rng, rng);
    engine::actor_mut(&mut r, 902.).unwrap().copied_inventory = Field::Missing;
    r.inventory.push(ring(4.));
    assert!(terrain_runtime::slip(&mut r, 902.), "ordinary actors cannot borrow player rings");
}

#[test]
fn chill_preserves_portal_distance_and_plane_rules() {
    use corpse_core::{dreamwalking, portals_generation::PortalPair};
    let mut r = room(50.); let w = r.floor().unwrap().width();
    r.floor_mut().unwrap().portals = vec![PortalPair { a: 10 * w + 10, b: 10 * w + 30, color_index: 0 }].into();
    r.actors_at_mut(r.depth).push(creature(901., 31., 10., "yeti"));
    monster_chill::apply_adjacency(&mut r);
    assert_eq!(r.player.conditions.get("slowSourceId"), Some(&901.));
    r.player.conditions.clear(); r.inventory.push(ring(1.));
    monster_chill::apply_adjacency(&mut r);
    assert_eq!(engine::condition(&r.player, "slowed"), 0.);

    let mut r = room(50.);
    r.dreamwalking = dreamwalking::State { active: true, depth: r.depth, floors: r.floors.clone(), ..Default::default() }.into();
    r.actors_at_mut(r.depth).push(creature(901., 11., 10., "yeti"));
    monster_chill::apply_adjacency(&mut r);
    assert_eq!(engine::condition(&r.player, "slowed"), 0., "waking Yeti cannot chill dream player");
    let mut dream_yeti = creature(902., 11., 10., "yeti");
    dream_yeti.conditions.insert("phantasmiiPlane".into(), 1.); r.actors_at_mut(r.depth).push(dream_yeti);
    monster_chill::apply_adjacency(&mut r);
    assert_eq!(r.player.conditions.get("slowSourceId"), Some(&902.));
    r.player.conditions.clear(); r.inventory.push(ring(1.));
    monster_chill::apply_adjacency(&mut r);
    assert_eq!(engine::condition(&r.player, "slowed"), 0.);

    // A waking Half Here endpoint can chill the shared dream-player stream,
    // but its owner's carried ring prevents both endpoint and owner infliction.
    r.player.abilities.push(half_here::ID.into());
    let clone = half_here::create(&mut r, 20., 10.).unwrap();
    engine::actor_mut(&mut r, clone).unwrap().conditions.insert("phantasmiiPlane".into(), 0.);
    engine::actor_mut(&mut r, 901.).unwrap().x = 21.;
    r.actors_at_mut(r.depth).retain(|a| a.id != 902.);
    monster_chill::apply_adjacency(&mut r);
    assert_eq!(engine::condition(&r.player, "slowed"), 0.);
    assert_eq!(engine::condition(engine::actor(&r, clone).unwrap(), "slowed"), 0.);
    r.inventory.clear(); monster_chill::apply_adjacency(&mut r);
    assert_eq!(engine::condition(&r.player, "slowed"), 1.);
    assert_eq!(engine::condition(engine::actor(&r, clone).unwrap(), "slowed"), 1.);
}

#[test]
fn protected_action_and_haste_clocks_do_not_receive_yeti_delay() {
    for immune in [false, true] {
        let mut r = room(50.);
        if immune { r.inventory.push(ring(1.)); }
        r.actors_at_mut(r.depth).push(creature(901., 11., 10., "yeti"));
        assert!(engine::dispatch(&mut r, &Command::Wait).unwrap().took_turn);
        assert_eq!(r.tick, if immune { 1. } else { 2. });
    }
    let mut r = room(50.); r.inventory.push(ring(1.));
    r.player.conditions.insert("hasted".into(), 5.);
    r.actors_at_mut(r.depth).push(creature(901., 11., 10., "yeti")); ice(&mut r, 10, 10);
    r.rng = slip_seed(&r);
    assert!(!engine::dispatch(&mut r, &Command::Move { dx: 0., dy: 1. }).unwrap().took_turn);
    assert_eq!((r.player.x, r.player.y), (10., 11.)); assert_eq!(r.tick, 0.);
    assert_eq!(r.phase.as_ref().map(String::as_str), Some("normal"));
    assert!(engine::dispatch(&mut r, &Command::Wait).unwrap().took_turn);
    assert_eq!(r.tick, 1.);
}

#[test]
fn witchcraft_and_other_statuses_are_not_ice_and_are_not_cleared() {
    let mut r = room(50.); r.inventory.push(ring(1.));
    let mut witch = creature(901., 13., 10., "witch"); witch.conditions.clear();
    r.actors_at_mut(r.depth).push(witch); let player = r.player.id;
    assert!(corpse_core::monster_spells::special_action(&mut r, 901., player).unwrap());
    assert_eq!(engine::condition(&r.player, "slowed"), 5.);
    assert_eq!(r.player.conditions.get("slowSourceId"), Some(&901.));
    r.player.conditions.insert("confused".into(), 8.);
    r.player.conditions.insert("paralyzed".into(), 2.);
    let conditions = r.player.conditions.clone();
    r.actors_at_mut(r.depth).push(creature(902., 11., 10., "yeti"));
    monster_chill::apply_adjacency(&mut r);
    assert_eq!(r.player.conditions, conditions);
    assert!(corpse_core::monster_statuses::apply_status(&mut r, player, "slowed", 7., None));
    assert_eq!(engine::condition(&r.player, "slowed"), 7.);
}

#[test]
fn immunity_keeps_beneficial_snow_quenching_and_does_not_melt_ice() {
    let mut r = room(50.); r.inventory.push(ring(1.)); ice(&mut r, 10, 10);
    let f = r.floor_mut().unwrap(); let n = f.tiles.len(); let at = 10 * f.width() + 10;
    f.snow = vec![false; n].into(); f.snow.as_mut().unwrap()[at] = true;
    r.player.conditions.insert("burning".into(), 8.);
    let player = r.player.id;
    assert!(terrain_runtime::snow_contact(&mut r, player));
    assert_eq!(engine::condition(&r.player, "burning"), 0.);
    assert!(!terrain_runtime::snow_at(r.floor().unwrap(), at));
    assert!(r.floor().unwrap().puddles.as_ref().unwrap()[at]);
    assert!(terrain_runtime::ice_at(r.floor().unwrap(), at));
}

#[test]
fn raw_save_and_snapshot_restore_keep_both_immunities_and_determinism() {
    let mut r = room(50.); r.inventory.push(ring(1.)); ice(&mut r, 10, 10);
    r.actors_at_mut(r.depth).push(creature(901., 11., 10., "yeti"));
    r.rng = slip_seed(&r);
    let snapshot = Snapshot::from_value(Snapshot::capture(&r).value().unwrap()).unwrap();
    r.inventory.clear(); r.player.x = 15.; snapshot.restore(&mut r).unwrap();
    let mut restored: Run = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
    for r in [&mut r, &mut restored] {
        let rng = r.rng; monster_chill::apply_adjacency(r);
        assert_eq!(engine::condition(&r.player, "slowed"), 0.);
        let player = r.player.id;
        assert!(engine::move_actor(r, player, 0., 1., true).unwrap());
        assert_eq!(r.player.y, 11.); assert_eq!(r.rng, rng);
    }
    assert_eq!(r, restored);
}
