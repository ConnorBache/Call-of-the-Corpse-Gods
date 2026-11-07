//! R50 gas identity is fixed at birth and at emission, including retained saves.
use corpse_core::{Actor, Field, Floor, Run, engine, environment, fungi, population,
    rng::GenerationRng, storage, temporal};
use std::sync::Arc;

fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into(); r.tick = 0.; r.depth = 1.;
    r.player.x = 10.; r.player.y = 10.; r.player.hp = 1000.; r.player.max_hp = 1000.;
    r.player.base_max_hp = 1000.0.into(); r.player.energy = 100.;
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.ability_order = Field::Missing; r.player.ability_reserves = Field::Missing;
    r.inventory.clear(); r.actors.clear(); r.ground.clear(); r.offers.clear();
    r.next_id = 1000.; r.phase = Field::Missing; r.history.clear(); r.future = None;
    let (w,h) = (79,29);
    let mut tiles = vec!["floor".into(); w*h];
    for i in 0..w*h { if i%w==0 || i%w==w-1 || i/w==0 || i/w==h-1 {tiles[i]="wall".into();} }
    tiles[80]="up".into(); tiles[27*w+77]="down".into();
    let mut floor = Floor {width:(w as f64).into(),height:(h as f64).into(),depth:1.,
        domain:"Dungeon".into(),tiles,lights:vec![100.;w*h],seen:vec![true;w*h],
        dirty:true,..Default::default()};
    environment::initialize(&mut floor).unwrap();
    r.floors=vec![Arc::new(floor)];
    // Every stored floor needs actor and ground blocks, including empty ones.
    r.actors_at_mut(1.).clear(); r.ground_at_mut(1.).clear(); r
}
fn bastidion(r: &mut Run, depth: f64, old_gas: f64) -> f64 {
    let mut a=population::create_actor_revision("bastidion",r.next_id,14.,10.,false,
        r.simulation_revision.value_or(29.)).unwrap();
    r.next_id+=1.; a.conditions.insert("fungalGasType".into(),old_gas);
    let id=a.id; r.actors_at_mut(depth).push(a); id
}
fn gas(f: &Floor, at: usize) -> Option<&str> {
    f.environment.as_ref().unwrap().gas_type.as_ref().unwrap()[at].as_deref()
}

#[test]
fn r50_population_all_aliases_depths_and_seeds_skip_random_gas_selection() {
    for revision in [29.,46.,50.] { for depth in [1.,20.,40.] { for seed in 1..=32 {
        for alias in ["fungus","bastidion","vent-fungus"] {
            let mut r=room(revision); let f=r.floor_mut().unwrap(); f.depth=depth;
            let mut rng=GenerationRng::new(seed); let mut expected=rng;
            let expected_gas=if revision>=50. {fungi::HALLUCINOGENIC_KIND}
                else {(expected.random()*6.).floor()};
            let mut next=100.;
            let a=population::create_population_actor_revision(&mut next,f,alias,10*79+14,
                false,&mut rng,revision).unwrap();
            assert_eq!(a.conditions["fungalGasType"],expected_gas,"{revision}/{depth}/{seed}/{alias}");
            assert_eq!(rng.state,expected.state,"R50 must not discard a random draw");
            assert_eq!(next,101.);
        }
    }}}
    let a=population::create_actor_revision("bastidion",2.,3.,4.,false,50.).unwrap();
    assert_eq!(a.conditions["fungalGasType"],fungi::HALLUCINOGENIC_KIND);
}

#[test]
fn r50_live_emission_overrides_every_saved_gas_and_legacy_keeps_exact_mapping() {
    let legacy=["caustic","breather","paralysis","flammable","explosive","hydrogen","caustic"];
    for revision in [29.,46.,50.] { for old_gas in 0..=6 {
        let mut r=room(revision); let id=bastidion(&mut r,1.,old_gas as f64);
        let rng=r.rng; let frozen=vec![false; r.floor().unwrap().tiles.len()];
        fungi::tick_fungi(&mut r,&frozen).unwrap();
        let expected=if revision>=50. {"hallucinogenic"} else {legacy[old_gas]};
        assert_eq!(gas(r.floor().unwrap(),10*79+14),Some(expected));
        assert_eq!(r.rng,rng,"emission chooses no random gas");
        assert_eq!(engine::actor(&r,id).unwrap().conditions["fungalGasType"],old_gas as f64,
            "retained saved actor values need not be rewritten to emit correctly");
    }}
}

#[test]
fn r50_root_connections_keep_volume_and_recolor_old_saved_roots() {
    let mut r=room(50.); let id=bastidion(&mut r,1.,0.);
    let corpse=Actor{id:r.next_id,x:16.,y:10.,name:"Corpse".into(),hp:0.,max_hp:10.,
        dead:true.into(),state:"dead".into(),..Default::default()};
    r.next_id+=1.;r.actors_at_mut(1.).push(corpse);
    let frozen=vec![false;r.floor().unwrap().tiles.len()];
    fungi::tick_fungi(&mut r,&frozen).unwrap();
    let f=r.floor().unwrap(); let roots=f.fungal_roots.as_ref().unwrap();
    assert_eq!(roots.len(),1);assert!(roots[0].active);assert_eq!(roots[0].fungus_id,id);
    assert_eq!(roots[0].color,fungi::HALLUCINOGENIC_COLOR);
    assert_eq!(f.environment.as_ref().unwrap().gas_volume.as_ref().unwrap()[10*79+14],50.);
    r.floor_mut().unwrap().fungal_roots.as_mut().unwrap()[0].color="#56ff37".into();
    fungi::tick_fungi(&mut r,&frozen).unwrap();
    assert_eq!(r.floor().unwrap().fungal_roots.as_ref().unwrap()[0].color,fungi::HALLUCINOGENIC_COLOR);
    assert_eq!(r.floor().unwrap().environment.as_ref().unwrap().gas_volume.as_ref().unwrap()[10*79+14],100.);
}

#[test]
fn r50_offscreen_save_snapshot_and_future_replay_emit_hallucinogenic_gas() {
    let mut r=room(50.);let mut inactive=r.floor().unwrap().clone();inactive.depth=2.;
    r.floors.push(Arc::new(inactive));bastidion(&mut r,2.,4.);
    r.ground_at_mut(2.).clear();r.tick=1.;
    let raw=storage::serialize(&r).unwrap();r=storage::deserialize(&raw).unwrap();
    let initial=temporal::Snapshot::capture(&r);
    let mut history=temporal::History::import(&r).unwrap();
    // Test an already persisted anchor; new R50 Future capture remains gated
    // until its all-energy ability-reserve payment rule is authored.
    r.future=Some(storage::serialize(&r).unwrap().into());
    corpse_core::offscreen::catch_up_floor(&mut r,2.).unwrap();
    let first=r.floors[1].clone();let rng=r.rng;
    assert!(first.environment.as_ref().unwrap().gas_type.as_ref().unwrap().iter()
        .any(|g|g.as_deref()==Some("hallucinogenic")));
    assert!(!first.environment.as_ref().unwrap().gas_type.as_ref().unwrap().iter()
        .any(|g|g.as_deref()==Some("explosive")));
    assert!(temporal::restore_future(&mut r,&mut history).unwrap());
    corpse_core::offscreen::catch_up_floor(&mut r,2.).unwrap();
    assert_eq!(r.floors[1],first);assert_eq!(r.rng,rng);
    initial.restore(&mut r).unwrap();
    corpse_core::offscreen::catch_up_floor(&mut r,2.).unwrap();
    assert_eq!(r.floors[1],first);assert_eq!(r.rng,rng);
}

#[test]
fn amulet_only_cooldown_clarification_preserves_mask_and_enemy_lockouts() {
    let mut r=room(50.);r.inventory.push(corpse_core::ahvamitt::mask(100.));
    r.player.conditions.insert("ahvamittTeleportReadyAt".into(),100.);
    assert_eq!(corpse_core::ahvamitt::cooldown(&r),100);
    let before=r.clone();assert!(!corpse_core::ahvamitt::teleport(&mut r).unwrap());assert_eq!(r,before);
    let mut witch=population::create_actor_revision("witch",200.,14.,10.,false,50.).unwrap();
    witch.energy=100.;witch.state="hunting".into();
    witch.conditions.insert("monsterSummonReadyAt".into(),100.);
    witch.conditions.insert("monsterSpellReadyAt".into(),100.);
    r.actors_at_mut(1.).push(witch);let target=r.player.id;let before=r.clone();
    assert!(!corpse_core::witch::summon(&mut r,200.,target).unwrap());
    assert!(!corpse_core::monster_spells::special_action(&mut r,200.,target).unwrap());
    assert_eq!(r,before);
}

fn mature_rooted_corpse(r: &mut Run) -> (f64, f64) {
    mature_rooted_corpse_kind(r, "rat")
}
fn mature_rooted_corpse_kind(r: &mut Run, kind: &str) -> (f64, f64) {
    let depth = r.depth;
    let fungus = bastidion(r, depth, 6.);
    let mut corpse = population::create_actor_revision(kind, r.next_id, 16., 10., false,
        r.simulation_revision.value_or(29.)).unwrap();
    r.next_id += 1.;
    corpse.dead = true.into(); corpse.hp = 0.; corpse.heartless = true.into();
    corpse.corpse_age = 500.0.into();
    let id = corpse.id; r.actors_at_mut(depth).push(corpse);
    let frozen = vec![false; r.floor().unwrap().tiles.len()];
    fungi::tick_fungi(r, &frozen).unwrap();
    assert!(environment::corpse_rooted(r.floor().unwrap(), r.actors_at(depth),
        engine::actor(r, id).unwrap()));
    (fungus, id)
}

#[test]
fn rooted_corpses_never_revive_during_normal_or_haste_turns_and_severing_frees_them() {
    for revision in [50., 51.] {
        let mut r = room(revision);
        let (_, corpse) = mature_rooted_corpse(&mut r);
        r.player.conditions.insert("hasted".into(), 8.);
        for _ in 0..4 {
            engine::dispatch(&mut r, &corpse_core::Command::Wait).unwrap();
            assert!(engine::actor(&r, corpse).unwrap().dead.value_or(false));
        }
        environment::burn_roots(r.floor_mut().unwrap(), 10 * 79 + 15);
        let frozen = vec![false; r.floor().unwrap().tiles.len()];
        fungi::tick_fungi(&mut r, &frozen).unwrap();
        assert!(!environment::corpse_rooted(r.floor().unwrap(), r.actors_at(1.),
            engine::actor(&r, corpse).unwrap()), "severed pair must not reconnect");
        for _ in 0..3 { engine::dispatch(&mut r, &corpse_core::Command::Wait).unwrap(); }
        assert!(!engine::actor(&r, corpse).unwrap().dead.value_or(false),
            "mature unbound corpse revives on its next ordinary phase");
    }
}

#[test]
fn rooted_corpses_remain_dead_offscreen_after_save_and_both_temporal_restores() {
    let mut r = room(51.);
    let mut inactive = r.floor().unwrap().clone(); inactive.depth = 2.;
    r.floors.push(Arc::new(inactive)); r.actors_at_mut(2.).clear(); r.ground_at_mut(2.).clear();
    r.depth = 2.; let (_, corpse) = mature_rooted_corpse(&mut r); r.depth = 1.;
    r = storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();
    let initial = temporal::Snapshot::capture(&r);
    let mut history = temporal::History::import(&r).unwrap();
    r.future = Some(storage::serialize(&r).unwrap().into());
    r.tick = 4.; corpse_core::offscreen::catch_up_floor(&mut r, 2.).unwrap();
    assert!(r.actors_at(2.).iter().find(|a| a.id == corpse).unwrap().dead.value_or(false));
    let first_floor = r.floors[1].clone(); let first_actors = r.actors_at(2.).to_vec(); let rng = r.rng;
    assert!(temporal::restore_future(&mut r, &mut history).unwrap());
    r.tick = 4.; corpse_core::offscreen::catch_up_floor(&mut r, 2.).unwrap();
    assert_eq!(r.floors[1], first_floor); assert_eq!(r.actors_at(2.), first_actors); assert_eq!(r.rng, rng);
    initial.restore(&mut r).unwrap();
    r.tick = 4.; corpse_core::offscreen::catch_up_floor(&mut r, 2.).unwrap();
    assert_eq!(r.floors[1], first_floor); assert_eq!(r.actors_at(2.), first_actors); assert_eq!(r.rng, rng);
}

#[test]
fn dead_fungus_cannot_revive_even_after_all_roots_are_removed() {
    let mut r = room(51.); let fungus = bastidion(&mut r, 1., 6.);
    let a = engine::actor_mut(&mut r, fungus).unwrap();
    a.dead = true.into(); a.hp = 0.; a.corpse_age = 500.0.into();
    assert!(!corpse_core::corpses::roll_corpse_revival(a, || panic!("fungus never rolls revival")));
    for _ in 0..3 { engine::dispatch(&mut r, &corpse_core::Command::Wait).unwrap(); }
    assert!(engine::actor(&r, fungus).unwrap().dead.value_or(false));
}

#[test]
fn solitary_inventor_and_mech_feast_corpses_validate_and_revive_safely_after_release() {
    for kind in ["hemogoblin-inventor", "hemogoblin-mech"] {
        let mut r = room(51.);
        let (_, corpse) = mature_rooted_corpse_kind(&mut r, kind);
        r = storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();
        assert!(!engine::actor(&r, corpse).unwrap().conditions.contains_key("monsterLinkedMechId"));
        assert!(!engine::actor(&r, corpse).unwrap().conditions.contains_key("monsterLinkedInventorId"));
        engine::dispatch(&mut r, &corpse_core::Command::Wait).unwrap();
        assert!(engine::actor(&r, corpse).unwrap().dead.value_or(false));
        environment::burn_roots(r.floor_mut().unwrap(), 10 * 79 + 15);
        engine::dispatch(&mut r, &corpse_core::Command::Wait).unwrap();
        assert!(!engine::actor(&r, corpse).unwrap().dead.value_or(false));
        let target = r.player.id;
        // An Inventor without a shell can flee; an unpiloted Mech stays neutral.
        assert!(corpse_core::monster_inventor::turn(&mut r, corpse, Some(target), false).unwrap());
        r = storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();
        if kind == "hemogoblin-mech" { assert_eq!(engine::actor(&r, corpse).unwrap().state, "neutral"); }
    }
}

#[test]
fn dead_aquatic_feast_validates_without_water_sources_and_revives_stranded() {
    let mut r = room(51.);
    let (_, corpse) = mature_rooted_corpse_kind(&mut r, "kelpie");
    r = storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();
    corpse_core::water_blood::register_kelpie_sources(&mut r, 1.);
    assert!(r.floor().unwrap().kelpie_blood_sources.as_ref().is_none_or(Vec::is_empty));
    assert!(!corpse_core::water_blood::active(r.floor().unwrap()));
    engine::dispatch(&mut r, &corpse_core::Command::Wait).unwrap();
    assert!(engine::actor(&r, corpse).unwrap().dead.value_or(false));
    environment::burn_roots_revision(r.floor_mut().unwrap(), 10 * 79 + 15, 51.);
    engine::dispatch(&mut r, &corpse_core::Command::Wait).unwrap();
    assert!(!engine::actor(&r, corpse).unwrap().dead.value_or(false));
    assert!(corpse_core::ichor::stranded(&r, engine::actor(&r, corpse).unwrap()));
    let position = (engine::actor(&r, corpse).unwrap().x, engine::actor(&r, corpse).unwrap().y);
    assert!(corpse_core::ichor::squirm_if_stranded(&mut r, corpse));
    engine::dispatch(&mut r, &corpse_core::Command::Wait).unwrap();
    assert_eq!((engine::actor(&r, corpse).unwrap().x, engine::actor(&r, corpse).unwrap().y), position);
    r = storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();
    assert!(!engine::actor(&r, corpse).unwrap().dead.value_or(false));
    assert!(corpse_core::ichor::stranded(&r, engine::actor(&r, corpse).unwrap()));
}
