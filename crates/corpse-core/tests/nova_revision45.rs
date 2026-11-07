//! Controlled Nova funding/ownership checks, not natural-world balance samples.
use corpse_core::{Actor, Command, Field, Item, Run, abilities, engine, storage};
const NOVA: &str = "anavani-nova";

fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into();
    r.floors.truncate(1); r.actors.retain(|k, _| k == "1"); r.ground.retain(|k, _| k == "1");
    r.depth = 1.; r.tick = 0.; r.status = "playing".into(); r.rng = 1.; r.next_id = 1000.;
    r.actors_at_mut(1.).clear(); r.ground_at_mut(1.).clear(); r.inventory.clear();
    r.offers.clear(); r.history.clear(); r.replay.clear(); r.future = None;
    r.phase = Field::Missing; r.haste_snapshot = Field::Missing; r.resource_history = Field::Missing;
    r.log.clear(); r.log_kinds = Field::Missing; r.log_archive = Field::Missing;
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.ability_order = Field::Missing; r.player.equipment_abilities = Field::Missing;
    r.player.x = 8.; r.player.y = 8.; r.player.hp = 1000.; r.player.max_hp = 1000.;
    r.player.base_max_hp = 1000.0.into(); r.player.stats.max_hp = Field::Missing;
    r.player.energy = 1000.; r.player.stats.max_energy = 1000.; r.player.stats.accuracy = 10000.;
    r.player.stats.offense = 0.; r.player.stats.dr = 10000.;
    let f = r.floor_mut().unwrap();
    f.environment = Field::Missing; f.vents = Field::Missing; f.permanent_fire = Field::Missing;
    f.godbeast_event = Field::Missing; f.godbeast = false; f.doors = Field::Missing;
    f.light_sources = Field::Missing; f.blood = Field::Missing; f.roots = Field::Missing;
    f.fungal_roots = Field::Missing; f.razorleaves = Field::Missing; f.grates = Field::Missing;
    f.portals = Field::Missing; f.non_euclidean = Field::Missing;
    let (w, h) = (f.width(), f.height());
    for y in 1..h-1 { for x in 1..w-1 { f.tiles[y*w+x] = "floor".into(); } }
    f.tiles[w+1] = "up".into(); f.tiles[(h-2)*w+w-2] = "down".into(); f.lights.fill(100.);
    r
}
fn npc(r: &mut Run, x: f64) -> f64 {
    let id = r.next_id; r.next_id += 1.;
    let mut a = corpse_core::population::create_actor_revision("rat", id, x, 8., false, 44.).unwrap();
    a.state = "hunting".into(); a.abilities.clear(); a.active.clear(); a.conditions.clear();
    a.conditions.insert("bestiaryTraitsInitialized".into(), 1.);
    a.hp = 10000.; a.max_hp = 10000.; a.base_max_hp = 10000.0.into(); a.stats.max_hp = Field::Missing;
    a.stats.dr = 10000.; a.stats.dodge = -10000.; a.stats.offense = 0.; a.stats.accuracy = 10000.;
    a.stats.max_energy = 1000.; a.energy = 1000.;
    r.actors_at_mut(1.).push(a); id
}
fn nova(a: &mut Actor, energy: f64) {
    a.abilities.push(NOVA.into()); a.active.push(NOVA.into()); a.energy = energy;
}
fn immune_target(r: &mut Run, id: f64) {
    engine::actor_mut(r, id).unwrap().abilities.push("enemy:beast:explosion-immunity".into());
}
fn energy_loss(r: &Run, id: f64) -> f64 {
    r.resource_history.as_ref().and_then(|h| h.actors.get(&id.to_string()))
        .and_then(|m| m.get("energy")).map_or(0., |m| m.loss)
}
fn offhand(r: &mut Run) {
    r.inventory.push(Item { id: 500., name: "QA offhand".into(), kind: "weapon".into(),
        hands: 1.0.into(), damage: 0.0.into(), offense_multiplier: 0.0.into(),
        equipped: "hand2".to_string().into(), ..Default::default() });
}

#[test]
fn all_legacy_revision_costs_and_catalog_remain_frozen() {
    for revision in 1..=44 { assert_eq!(abilities::nova_attack_cost(revision as f64), 30.); }
    assert_eq!(abilities::nova_attack_cost(45.), 40.);
    let catalog = corpse_core::progression::catalog().iter().find(|a| a.id == NOVA).unwrap();
    assert!(catalog.description.contains("30 energy"));
}

#[test]
fn player_melee_and_ranged_hit_or_miss_pay_only_when_fully_funded() {
    for revision in [29., 39., 44., 45.] { for ranged in [false, true] { for hit in [false, true] {
        let cost = abilities::nova_attack_cost(revision);
        for energy in [cost - 1., cost] {
            let mut r = room(revision); let id = npc(&mut r, if ranged { 11. } else { 9. });
            nova(&mut r.player, energy);
            r.player.conditions.insert(if hit { "lucky" } else { "distracted" }.into(), 1.);
            if ranged { engine::attack_ranged(&mut r, 0., id, 0., 3., 1., false).unwrap(); }
            else { engine::player_attack(&mut r, id).unwrap(); }
            let funded = energy >= cost;
            assert_eq!(r.player.energy, if funded { 0. } else { energy });
            assert_eq!(r.player.active.iter().any(|s| s == NOVA), funded);
            assert_eq!(engine::actor(&r, id).unwrap().hp, if funded { 5000. } else { 10000. },
                "revision={revision}, ranged={ranged}, hit={hit}, energy={energy}");
            assert_eq!(energy_loss(&r, 0.), if funded { cost } else { 0. });
            assert_eq!(r.player.hp, 1000., "passive explosion immunity is unchanged");
        }
    }}}
}

#[test]
fn missing_revision_still_uses_thirty_energy() {
    let mut r = room(29.); r.simulation_revision = Field::Missing;
    let id = npc(&mut r, 9.); nova(&mut r.player, 30.);
    engine::player_attack(&mut r, id).unwrap(); assert_eq!(r.player.energy, 0.);
    assert_eq!(engine::actor(&r, id).unwrap().hp, 5000.);
}

#[test]
fn toggle_is_free_even_when_no_blast_can_be_funded_and_has_no_upkeep() {
    for revision in [44., 45.] { for energy in [0., 29., 30., 39., 40.] {
        let mut r = room(revision); r.player.abilities.push(NOVA.into()); r.player.energy = energy;
        let rng = r.rng;
        assert!(!abilities::activate(&mut r, NOVA, &Field::Missing, &Field::Missing, &Field::Missing).unwrap());
        assert!(r.player.active.iter().any(|s| s == NOVA)); assert_eq!(r.player.energy, energy);
        abilities::pay_upkeep(&mut r, 0.).unwrap(); assert_eq!(r.player.energy, energy);
        abilities::activate(&mut r, NOVA, &Field::Missing, &Field::Missing, &Field::Missing).unwrap();
        assert!(!r.player.active.iter().any(|s| s == NOVA)); assert_eq!(r.player.energy, energy);
        assert_eq!(r.rng, rng);
    }}
}

#[test]
fn independent_attacker_melee_and_ranged_routes_share_funding() {
    // Lower-level paths only: full run admission still rejects independent NPC toggles.
    for ranged in [false, true] { for energy in [39., 40.] {
        let mut r = room(45.); let id = npc(&mut r, if ranged { 11. } else { 9. });
        nova(engine::actor_mut(&mut r, id).unwrap(), energy);
        r.player.abilities.push(NOVA.into());
        if ranged { engine::attack_ranged(&mut r, id, 0., 0., 3., 1., false).unwrap(); }
        else { engine::attack(&mut r, id, 0., 1.).unwrap(); }
        let a = engine::actor(&r, id).unwrap();
        assert_eq!(a.energy, if energy == 40. { 0. } else { energy });
        assert_eq!(a.active.iter().any(|s| s == NOVA), energy == 40.);
        assert_eq!(r.player.energy, 1000.);
    }}
}

#[test]
fn ai_hornlight_pays_spell_before_nova_and_cannot_borrow_missing_energy() {
    for energy in [47., 48.] {
        let mut r = room(45.); let id = npc(&mut r, 11.);
        let a = engine::actor_mut(&mut r, id).unwrap(); nova(a, energy);
        a.abilities.push("enemy:beast:horn-bolt".into());
        r.player.abilities.push(NOVA.into());
        assert!(corpse_core::ai::enemy_action(&mut r, id, 0.).unwrap());
        let a = engine::actor(&r, id).unwrap();
        assert_eq!(a.energy, if energy == 48. { 0. } else { 39. });
        assert_eq!(a.active.iter().any(|s| s == NOVA), energy == 48.);
    }
}

#[test]
fn assault_and_both_hands_pay_per_resolved_attack_until_funds_run_out() {
    for (energy, remaining, active) in [(49.,39.,false),(50.,0.,false),(90.,0.,false),(169.,39.,false),(170.,0.,true)] {
        let mut r = room(45.); let id = npc(&mut r, 9.); immune_target(&mut r, id);
        nova(&mut r.player, energy); offhand(&mut r);
        r.player.abilities.push(abilities::ASSAULT.into()); r.player.active.push(abilities::ASSAULT.into());
        engine::player_attack(&mut r, id).unwrap();
        assert_eq!(r.player.energy, remaining, "starting energy={energy}");
        assert_eq!(r.player.active.iter().any(|s| s == NOVA), active, "starting energy={energy}");
        assert_eq!(energy_loss(&r, 0.), energy - remaining);
    }
}

#[test]
fn recursive_npc_offhand_and_bite_repeat_use_same_cost() {
    for mini_me in [false, true] {
        let mut r = room(45.); let id = npc(&mut r, 9.); r.player.abilities.push(NOVA.into());
        let a = engine::actor_mut(&mut r, id).unwrap(); nova(a, if mini_me { 80. } else { 88. });
        if mini_me { a.conditions.insert("miniMe".into(),1.); a.conditions.insert("offhandDamage".into(),1.); }
        else { a.abilities.push("enemy:pale:bite-repeat".into()); }
        engine::attack(&mut r, id, 0., 1.).unwrap();
        assert_eq!(engine::actor(&r, id).unwrap().energy, 0.);
    }
}

#[test]
fn redirected_miss_funds_both_resolved_attacks() {
    let mut r = room(45.); let id = npc(&mut r, 9.); immune_target(&mut r, id);
    nova(&mut r.player, 80.); r.player.conditions.insert("distracted".into(),1.);
    let a = engine::actor_mut(&mut r, id).unwrap();
    a.abilities.push(corpse_core::divine_reactions::DEFLECT.into());
    a.active.push(corpse_core::divine_reactions::DEFLECT.into()); a.reactions = 1.0.into();
    engine::player_attack(&mut r, id).unwrap(); assert_eq!(r.player.energy, 0.);
    assert_eq!(energy_loss(&r, 0.), 80.);
}

#[test]
fn half_here_counterattack_spends_owner_energy_not_the_endpoint_copy() {
    for energy in [49., 50.] {
        let mut r = room(45.); let id = npc(&mut r, 10.); immune_target(&mut r, id);
        nova(&mut r.player, energy);
        r.player.abilities.extend([corpse_core::half_here::ID.into(), "pelian-never-without-a-fight".into()]);
        r.player.active.push("pelian-never-without-a-fight".into());
        let clone = corpse_core::half_here::create(&mut r, 9., 8.).unwrap();
        engine::actor_mut(&mut r, clone).unwrap().energy = 999.;
        engine::attack(&mut r, id, clone, 1.).unwrap();
        assert_eq!(r.player.energy, if energy == 50. { 0. } else { 39. });
        assert_eq!(r.player.active.iter().any(|s| s == NOVA), energy == 50.);
        assert_eq!(engine::actor(&r, id).unwrap().energy, 1000.);
    }
}

#[test]
fn thrown_and_bow_inventory_paths_charge_exactly_one_blast() {
    for shoot in [false, true] {
        let mut r = room(45.); let id = npc(&mut r, 11.); immune_target(&mut r, id);
        nova(&mut r.player, 40.); r.player.conditions.insert("lucky".into(),1.);
        let mut item = Item { id:500., name:"QA weapon".into(), kind:"weapon".into(),
            damage:0.0.into(), hands:1.0.into(), ..Default::default() };
        let command = if shoot {
            item.hands = 2.0.into(); item.ranged_weapon = "bow".to_string().into();
            item.equipped = "hand1".to_string().into();
            r.inventory.push(Item { id:501., name:"Arrow".into(), kind:"ammo".into(), effect:"arrows".to_string().into(), ..Default::default() });
            Command::Shoot { id:500., x:11., y:8. }
        } else { Command::Throw { id:500., x:11., y:8. } };
        r.inventory.push(item);
        assert_eq!(corpse_core::inventory::command(&mut r, &command).unwrap(), Some(true));
        assert_eq!(r.player.energy, 0.); assert_eq!(energy_loss(&r, 0.), 40.);
    }
}

#[test]
fn blocked_attacks_and_projectiles_without_a_victim_do_not_charge() {
    let mut r = room(45.); let id = npc(&mut r, 9.); nova(&mut r.player, 40.);
    engine::actor_mut(&mut r, id).unwrap().conditions.insert("phantasmiiPlane".into(),1.);
    let rng = r.rng; engine::player_attack(&mut r, id).unwrap();
    assert_eq!(r.player.energy, 40.); assert_eq!(r.rng, rng);
    r.inventory.push(Item { id:500., name:"QA stone".into(), kind:"tool".into(), ..Default::default() });
    assert_eq!(corpse_core::inventory::command(&mut r, &Command::Throw { id:500.,x:8.,y:11. }).unwrap(), Some(true));
    assert_eq!(r.player.energy, 40.); assert!(r.player.active.iter().any(|s| s == NOVA));
}

#[test]
fn unspeak_uses_runtime_nova_cost_without_rewriting_catalog_or_other_costs() {
    for revision in [29., 44., 45.] {
        let mut r = room(revision); let id = npc(&mut r, 9.);
        engine::actor_mut(&mut r, id).unwrap().abilities.push(NOVA.into());
        assert_eq!(abilities::highest_energy_cost(&mut r, id).unwrap(), abilities::nova_attack_cost(revision));
        engine::actor_mut(&mut r, id).unwrap().abilities.push("numo-mind-slave".into());
        assert_eq!(abilities::highest_energy_cost(&mut r, id).unwrap(), 100.);
    }
}

#[test]
fn save_load_snapshot_restore_and_command_replay_keep_revision_and_energy() {
    for revision in [44., 45.] {
        let mut r = room(revision); let id = npc(&mut r, 9.); immune_target(&mut r, id);
        engine::actor_mut(&mut r, id).unwrap().conditions.insert("paralyzed".into(), 100.);
        nova(&mut r.player, 80.);
        let initial = r.clone(); let snapshot = corpse_core::temporal::Snapshot::capture(&r);
        let mut e = corpse_core::temporal::Engine::new(r).unwrap();
        for _ in 0..2 { e.dispatch(&Command::Move { dx:1., dy:0. }).unwrap(); }
        assert_eq!(e.run.player.energy, 80. - 2. * abilities::nova_attack_cost(revision));
        let saved = e.save().unwrap(); let loaded = corpse_core::temporal::Engine::load(&saved).unwrap();
        assert_eq!(loaded.save().unwrap(), saved);
        let record = corpse_core::replay::ReplayRecord::try_from_run(&e.run).unwrap();
        let mut replay = corpse_core::replay::ReplaySession::from_initial(initial.clone(), record).unwrap();
        while replay.step().unwrap() {}
        assert_eq!(replay.engine.save().unwrap(), saved);
        let mut restored = loaded.run; snapshot.restore(&mut restored).unwrap();
        assert_eq!(restored.player.energy, 80.); assert_eq!(restored.simulation_revision, initial.simulation_revision);
        engine::player_attack(&mut restored, id).unwrap();
        assert_eq!(restored.player.energy, 80. - abilities::nova_attack_cost(revision));
        assert_eq!(storage::deserialize(&storage::serialize(&restored).unwrap()).unwrap().player.energy, restored.player.energy);
    }
}
