//! Ichor species resist fire in current runs without changing legacy packets or RNG.
use corpse_core::{Actor, Command, Field, Item, Run, Stats, engine, environment,
    population, revision51_ichor_fire as immunity, storage};

const KINDS: [&str; 9] = [
    "lesser-ichor-fish", "ichor-fish-standard", "greater-ichor-fish",
    "lesser-ichor-shark", "ichor-shark", "greater-ichor-shark",
    "lesser-ichor-leviathan", "ichor-leviathan", "greater-ichor-leviathan",
];
fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into();
    r.player.x = 10.; r.player.y = 10.; r.player.hp = 100.; r.player.max_hp = 100.;
    r.player.base_max_hp = 100.0.into(); r.player.energy = 100.;
    r.player.stats = Stats { accuracy: 100., perception: 300., max_energy: 100., ..Default::default() };
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.ability_order = Field::Missing; r.player.equipment_abilities = Field::Missing;
    r.player.ability_reserves = Field::Missing; r.inventory.clear();
    for actors in r.actors.values_mut() { *actors = Default::default(); }
    for ground in r.ground.values_mut() { *ground = Default::default(); }
    r.history.clear(); r.haste_snapshot = Field::Missing; r.future = None;
    r.phase = "normal".to_string().into(); r.next_id = 1000.;
    let f = r.floor_mut().unwrap(); f.tiles.fill("floor".into()); f.lights.fill(100.); f.seen.fill(true);
    f.doors = Field::Missing; f.portals = Field::Missing; f.environment = Field::Missing;
    f.non_euclidean = Field::Missing; f.speed_up = Field::Missing; f.currents = Field::Missing;
    f.fungal_roots = Field::Missing; f.roots = Field::Missing; f.mud = Field::Missing;
    f.slime = Field::Missing; f.ichor = Field::Missing; f.razorleaves = Field::Missing;
    f.water_blood = Field::Missing; f.kelpie_blood_sources = Field::Missing;
    f.vents = Field::Missing; f.permanent_fire = Field::Missing; f.hellfire = Field::Missing;
    f.ice = Field::Missing; f.snow = Field::Missing; f.puddles = Field::Missing;
    f.godbeast_event = Field::Missing;
    r
}
fn fish(kind: &str, id: f64, x: f64, y: f64, revision: f64) -> Actor {
    let mut a = population::create_actor_revision(kind, id, x, y, false, revision).unwrap();
    a.hp = 100.; a.max_hp = 100.; a.base_max_hp = 100.0.into(); a.state = "neutral".into();
    a
}
fn insert(r: &mut Run, kind: &str) {
    let a = fish(kind, 901., 12., 10., r.simulation_revision.value_or(29.));
    r.actors_at_mut(r.depth).push(a);
}
fn clean(a: &Actor) {
    assert_eq!(a.hp, 100., "{}", a.name);
    assert!(engine::condition(a, "burning") <= 0., "{} still burns", a.name);
    assert!(!a.conditions.contains_key("burningOwner"));
    assert!(!a.conditions.contains_key("burningPlayer"));
}
fn set_burning(a: &mut Actor) {
    for (key, n) in [("burning", 8.), ("burningOwner", -2.), ("burningPlayer", 1.)] {
        a.conditions.insert(key.into(), n);
    }
}
fn incinerate(r: &mut Run, x: f64, y: f64, thrown: bool) {
    r.inventory.push(Item { id: 800., kind: "potion".into(), name: "Potion of incineration".into(),
        glyph: "!".into(), effect: "incineration".to_string().into(), ..Default::default() });
    let at = r.inventory.len() - 1;
    assert!(corpse_core::potions::apply(r, at, x, y, thrown).unwrap());
}

#[test]
fn every_tier_rejects_all_fire_packet_routes_only_in_revision51() {
    for revision in [29., 40., 50., 51.] {
        for kind in KINDS {
            let mut r = room(revision); insert(&mut r, kind); let source = r.player.clone(); let rng = r.rng;
            engine::damage_environment(&mut r, 901., 8., None, "fire", true).unwrap();
            engine::damage_direct(&mut r, 901., 8., source.id, "fire", true).unwrap();
            engine::damage_environment_with_source_snapshot(&mut r, 901., 8., &source, "fire", true).unwrap();
            engine::damage_direct_with_source_snapshot(&mut r, 901., 8., &source, "fire", true).unwrap();
            assert_eq!(engine::actor(&r, 901.).unwrap().hp, if revision >= 51. {100.} else {68.}, "{revision} {kind}");
            assert_eq!(r.rng, rng);
        }
    }
}
#[test]
fn fire_immunity_does_not_block_other_damage_or_general_mercurial_creatures() {
    for kind in KINDS {
        for damage in ["attack", "cold", "lightning", "acid", "poison", "bleeding", "explosion"] {
            let mut r = room(51.); insert(&mut r, kind);
            engine::damage_environment(&mut r, 901., 8., None, damage, true).unwrap();
            assert_eq!(engine::actor(&r, 901.).unwrap().hp, if damage == "explosion" {50.} else {92.}, "{kind} {damage}");
        }
    }
    let mut r = room(51.); let mut other = population::create_actor_revision("rat", 901., 12., 10., false, 51.).unwrap();
    other.hp = 100.; other.max_hp = 100.; corpse_core::ichor::mercurialize_actor(&mut other);
    assert!(!immunity::immune(51., &other)); r.actors_at_mut(1.).push(other);
    engine::damage_environment(&mut r, 901., 8., None, "fire", true).unwrap();
    assert_eq!(engine::actor(&r, 901.).unwrap().hp, 92.);
}
#[test]
fn identity_fallback_is_exact_and_revisioned() {
    for kind in KINDS {
        let a = fish(kind, 901., 12., 10., 51.);
        let mut nameless_kind = a.clone(); nameless_kind.creature_kind = Field::Missing;
        assert!(immunity::immune(51., &nameless_kind)); assert!(!immunity::immune(50., &nameless_kind));
        nameless_kind.name = format!("Mercuria’s {}", nameless_kind.name);
        assert!(immunity::immune(51., &nameless_kind));
        nameless_kind.creature_kind = "rat".to_string().into();
        assert!(!immunity::immune(51., &nameless_kind));
    }
    let mut alias = Actor { creature_kind: "ichor-fish".to_string().into(), ..Default::default() };
    assert!(immunity::immune(51., &alias)); alias.creature_kind = "not-ichor-fish".to_string().into();
    assert!(!immunity::immune(51., &alias));
}
#[test]
fn hazard_fire_hellfire_and_existing_burning_are_harmless_for_every_tier() {
    for kind in KINDS {
        for hellfire in [false, true] {
            let mut r = room(51.); insert(&mut r, kind);
            let at = 10 * r.floor().unwrap().width() + 12;
            if hellfire {
                let n = r.floor().unwrap().tiles.len();
                r.floor_mut().unwrap().hellfire = vec![false; n].into();
                r.floor_mut().unwrap().hellfire.as_mut().unwrap()[at] = true;
            } else {
                environment::ignite(&mut r, 12., 10., Some(8.), None).unwrap();
            }
            set_burning(engine::actor_mut(&mut r, 901.).unwrap());
            assert!(!environment::creature_hazard_at(&r, engine::actor(&r, 901.).unwrap(), at));
            let rng = r.rng; environment::refresh_burning_from_fire(&mut r, 901.).unwrap();
            assert_eq!(r.rng, rng); clean(engine::actor(&r, 901.).unwrap());
            set_burning(engine::actor_mut(&mut r, 901.).unwrap());
            environment::tick_environment(&mut r).unwrap(); clean(engine::actor(&r, 901.).unwrap());
        }
    }
}
#[test]
fn inherited_burning_does_not_spread_or_melt_ice() {
    let mut r = room(51.); insert(&mut r, KINDS[0]);
    let at = 10 * r.floor().unwrap().width() + 12; let n = r.floor().unwrap().tiles.len();
    r.floor_mut().unwrap().ice = vec![false; n].into();
    r.floor_mut().unwrap().ice.as_mut().unwrap()[at] = true;
    set_burning(engine::actor_mut(&mut r, 901.).unwrap());
    environment::tick_environment(&mut r).unwrap(); clean(engine::actor(&r, 901.).unwrap());
    assert!(r.floor().unwrap().environment.as_ref().unwrap().fire.iter().all(|v| *v == 0.));
    assert!(r.floor().unwrap().ice.as_ref().unwrap()[at]);
    r.floor_mut().unwrap().snow = vec![false; n].into();
    r.floor_mut().unwrap().snow.as_mut().unwrap()[at] = true;
    set_burning(engine::actor_mut(&mut r, 901.).unwrap());
    assert!(!corpse_core::terrain_runtime::snow_contact(&mut r, 901.));
    clean(engine::actor(&r, 901.).unwrap());
    assert!(r.floor().unwrap().snow.as_ref().unwrap()[at]);
}
#[test]
fn incineration_drunk_or_thrown_and_breath_aoe_cannot_ignite_any_tier() {
    for revision in [50., 51.] {
        for kind in KINDS {
            let mut r = room(revision); insert(&mut r, kind);
            incinerate(&mut r, 12., 10., true);
            assert_eq!(engine::condition(engine::actor(&r, 901.).unwrap(), "burning") > 0., revision < 51., "{kind}");
            let mut r = room(revision); insert(&mut r, kind);
            // Use the real player as source so the same cone/AoE path includes all victims.
            let source = r.player.id;
            assert!(corpse_core::monster_inventor::breathe_fire(&mut r, source, 901.).unwrap());
            let a = engine::actor(&r, 901.).unwrap();
            assert_eq!(a.hp, if revision >= 51. {100.} else {76.});
            assert_eq!(engine::condition(a, "burning") > 0., revision < 51.);
            let mut r = room(revision); r.player.creature_kind = kind.to_string().into();
            incinerate(&mut r, 10., 10., false);
            assert_eq!(engine::condition(&r.player, "burning") > 0., revision < 51.);
        }
    }
}
#[test]
fn firebolt_ignition_respects_current_and_legacy_immunity() {
    for revision in [50., 51.] {
        for kind in KINDS {
            let mut r = room(revision); insert(&mut r, kind);
            let amulet = corpse_core::revision50_amulet_effects::make_amulet(
                corpse_core::revision50_amulet_effects::Effect::Firebolt, 800.).unwrap();
            r.inventory.push(amulet);
            let used = corpse_core::revision50_amulet_effects::command(&mut r, &Command::UseAt {id:800.,x:12.,y:10.}).unwrap();
            assert!(used.is_some());
            let a = engine::actor(&r, 901.).unwrap();
            assert_eq!(a.hp, if revision >= 51. {100.} else {20.}, "{kind}");
            assert_eq!(engine::condition(a, "burning") > 0., revision < 51.);
        }
    }
}
#[test]
fn offscreen_burning_and_hellfire_use_same_immunity_for_every_tier() {
    for revision in [50., 51.] {
        for kind in KINDS {
            let mut r = room(revision); r.tick = 1.;
            let mut f = r.floor().unwrap().clone(); f.depth = 2.; f.last_tick = 0.; f.dirty = true;
            let at = 10 * f.width() + 12; f.hellfire = vec![false; f.tiles.len()].into();
            f.hellfire.as_mut().unwrap()[at] = true;
            let fi = r.floors.iter().position(|f| f.depth == 2.).unwrap(); r.floors[fi] = std::sync::Arc::new(f);
            let mut a = fish(kind, 901., 12., 10., revision); set_burning(&mut a); r.actors_at_mut(2.).push(a);
            corpse_core::offscreen::catch_up_floor(&mut r, 2.).unwrap();
            let a = &r.actors_at(2.)[0];
            if revision >= 51. {clean(a)} else {assert!(a.hp < 100.); assert!(engine::condition(a, "burning") > 0.);}
            let state = storage::serialize(&r).unwrap(); corpse_core::offscreen::catch_up_floor(&mut r, 2.).unwrap();
            assert_eq!(storage::serialize(&r).unwrap(), state); assert_eq!(r.floors[fi].last_tick, 1.);
        }
    }
}
#[test]
fn save_snapshot_future_and_replay_retain_revisioned_protection() {
    for revision in [50., 51.] {
        let mut initial = room(revision); insert(&mut initial, KINDS[4]);
        set_burning(engine::actor_mut(&mut initial, 901.).unwrap());
        let saved = storage::serialize(&initial).unwrap(); let mut loaded = storage::deserialize_compatible(&saved).unwrap();
        let snapshot = corpse_core::temporal::Snapshot::capture(&initial);
        let mut future_state = initial.clone();
        future_state.player.abilities.push(corpse_core::revision50_ability_energy::FUTURE_ID.into());
        corpse_core::revision50_ability_energy::acquire(&mut future_state.player,
            &[corpse_core::revision50_ability_energy::FUTURE_ID.into()]).unwrap();
        corpse_core::temporal::remember_future(&mut future_state, &Default::default()).unwrap();
        for r in [&mut initial, &mut loaded, &mut future_state] {
            engine::damage_environment(r, 901., 8., None, "fire", true).unwrap();
            assert_eq!(engine::actor(r, 901.).unwrap().hp, if revision >= 51. {100.} else {92.});
        }
        assert!(corpse_core::temporal::restore_future(&mut future_state, &mut Default::default()).unwrap());
        assert_eq!(engine::actor(&future_state, 901.).unwrap().hp, 100.);
        engine::damage_environment(&mut future_state, 901., 8., None, "fire", true).unwrap();
        assert_eq!(engine::actor(&future_state, 901.).unwrap().hp, if revision >= 51. {100.} else {92.});
        snapshot.restore(&mut initial).unwrap(); assert_eq!(initial.simulation_revision, Field::Value(revision));
        let mut run = corpse_core::temporal::Engine::new(initial.clone()).unwrap(); run.dispatch(&Command::Wait).unwrap();
        let record = corpse_core::replay::ReplayRecord::try_from_run(&run.run).unwrap();
        let mut replay = corpse_core::replay::ReplaySession::from_initial(initial, record).unwrap();
        while replay.step().unwrap() {}
        assert_eq!(storage::serialize(&run.run).unwrap(), storage::serialize(&replay.engine.run).unwrap());
        if revision >= 51. { clean(engine::actor(&run.run, 901.).unwrap()); }
    }
}
