//! Designer-confirmed revision51 corrections, preserving retained R46/R50 rules.
use corpse_core::{abilities, dreamwalking, engine, environment, fire_adapter::FireAdapter,
    movement::Point, progression, revision50_ability_energy as energy, storage,
    temporal::{self, Engine, History, Snapshot}, vision, Command, Field, Item, Run};
use std::sync::Arc;
const CAME: &str = "pelian-i-came-for-you";
const SHINE: &str = "anavani-shine";
fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = revision.into();
    r.player.x = 10.; r.player.y = 10.; r.player.energy = 100.; r.player.stats.max_energy = 100.;
    r.player.hp = 1000.; r.player.max_hp = 1000.; r.player.base_max_hp = 1000.0.into();
    r.player.abilities = vec![CAME.into(), SHINE.into(), abilities::SPARKLE.into(), abilities::ALL_SEEING.into()];
    r.player.active.clear(); r.player.conditions.clear(); r.player.ability_reserves = Field::Missing;
    r.inventory.clear(); r.offers.clear(); r.replay.clear(); r.history.clear(); r.future = None;
    r.phase = Field::Missing; r.haste_snapshot = Field::Missing; r.replay_snapshot_from = Field::Missing;
    for actors in r.actors.values_mut() { *actors = Arc::new(vec![]); }
    for items in r.ground.values_mut() { *items = Arc::new(vec![]); }
    let f = r.floor_mut().unwrap();
    f.tiles.fill("floor".into()); f.lights.fill(-50.); f.seen.fill(false);
    f.environment = Field::Missing; f.doors = Field::Missing; f.vents = Field::Missing;
    f.currents = Field::Missing; f.mud = Field::Missing; f.slime = Field::Missing;
    f.roots = Field::Missing; f.fungal_roots = Field::Missing; f.ichor = Field::Missing;
    f.light_sources = Field::Missing; f.permanent_fire = Field::Missing;
    f.snow = Field::Missing; f.ice = Field::Missing; f.hellfire = Field::Missing;
    if revision >= 50. { for id in [CAME, abilities::SPARKLE, abilities::ALL_SEEING, abilities::SLEEP, "morae-see-the-future"] {
        energy::initialize_reserve(&mut r.player, id, 0., 0.).unwrap();
    }}
    r
}
fn ability(id: &str) -> Command { Command::Ability { id: id.into(), target: Field::Missing, x: Field::Missing, y: Field::Missing } }
fn at(id: &str, x: f64, y: f64) -> Command { Command::Ability { id: id.into(), target: Field::Missing, x: x.into(), y: y.into() } }
fn target(r: &mut Run) -> f64 {
    let id = r.next_id; r.next_id += 1.;
    let mut a = corpse_core::population::create_actor_revision("rat", id, 20., 10., false, r.simulation_revision.value_or(51.)).unwrap();
    a.conditions.insert("paralyzed".into(), 100.); a.hp = 1000.; a.max_hp = 1000.; a.base_max_hp = 1000.0.into();
    r.actors_at_mut(r.depth).push(a); id
}
fn reveal(r: &mut Run) { assert!(!engine::dispatch(r, &ability(abilities::ALL_SEEING)).unwrap().took_turn); assert!(vision::all_seeing(r)); }
fn no_fire(r: &Run, i: usize) {
    let f = r.floor().unwrap();
    assert!(f.environment.as_ref().is_none_or(|e| e.fire[i] == 0. && e.fire_kind.as_ref().unwrap()[i].is_none()
        && !e.fire_fresh.as_ref().unwrap()[i] && e.fire_reaction.as_ref().unwrap()[i] == 0.));
    assert!(f.permanent_fire.as_ref().is_none_or(|v| !v.contains(&(i as f64))));
    assert!(f.light_sources.as_ref().is_none_or(|v| !v.iter().any(|s| s.x == (i % f.width()) as f64 && s.y == (i / f.width()) as f64 && s.kind.as_ref().is_some_and(|k| k == "brazier"))));
}
#[test]
fn sparkle_has_no_terrain_limit_but_water_including_ichor_quenches_immediately() {
    for tile in ["floor", "wall", "chasm", "pit", "water", "shallow-water", "grass", "tree"] {
        for ichor in [false, true] {
            let mut r = room(51.); reveal(&mut r);
            let i = 10 * r.floor().unwrap().width() + 15;
            r.floor_mut().unwrap().tiles[i] = tile.into();
            if ichor { let n=r.floor().unwrap().tiles.len(); r.floor_mut().unwrap().ichor=vec![0.;n].into(); r.floor_mut().unwrap().ichor.as_mut().unwrap()[i]=1.; }
            let (tick, rng, energy) = (r.tick, r.rng, r.player.energy);
            assert!(!engine::dispatch(&mut r, &at(abilities::SPARKLE, 15., 10.)).unwrap().took_turn);
            assert_eq!((r.tick, r.rng, r.player.energy), (tick, rng, energy - 20.));
            if matches!(tile, "water" | "shallow-water") { no_fire(&r, i); }
            else { assert!(r.floor().unwrap().environment.as_ref().unwrap().fire[i] > 0.); }
            assert!(vision::all_seeing(&r));
            let restored = storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();
            assert_eq!(restored.floor().unwrap().permanent_fire, r.floor().unwrap().permanent_fire);
        }
    }
}
#[test]
fn paid_abilities_reject_silence_or_insufficient_energy_before_world_mutation() {
    for id in [CAME, abilities::ALL_SEEING, abilities::SPARKLE] { for silenced in [false, true] {
        let mut r=room(51.); let tid=target(&mut r); r.player.energy=if silenced {100.} else {0.};
        if silenced {r.player.conditions.insert("silenced".into(),1.);}
        let player=r.player.clone(); let floor=r.floor().unwrap().clone(); let rng=r.rng;
        abilities::activate(&mut r,id,&tid.into(),&15.0.into(),&10.0.into()).unwrap();
        assert_eq!(r.player,player); assert_eq!(r.floor().unwrap(),&floor); assert_eq!(r.rng,rng);
    }}
    let mut r=room(51.);r.player.energy=0.;r.player.conditions.insert("silenced".into(),1.);
    assert!(!engine::dispatch(&mut r,&ability(SHINE)).unwrap().took_turn);
    assert!(r.player.active.iter().any(|id|id==SHINE)); assert_eq!(r.player.energy,0.);
}
#[test]
fn came_is_unlimited_after_exact_ten_payment_and_preserves_legacy_countdown() {
    for revision in [46.,50.,51.] {
        let mut r=room(revision);let tid=target(&mut r);r.player.energy=4.;
        if revision>=50. {
            *r.player.ability_reserves.as_mut().unwrap().get_mut(CAME).unwrap() = energy::AbilityReserve { capacity: 50., energy: 6. };
        }
        else {r.player.energy=10.;}
        abilities::activate(&mut r,CAME,&tid.into(),&Field::Missing,&Field::Missing).unwrap();
        assert_eq!(r.player.energy,0.);assert!(vision::telepathic(&r,engine::actor(&r,tid).unwrap()));
        for _ in 0..25 {engine::dispatch(&mut r,&Command::Wait).unwrap();}
        assert_eq!(r.player.conditions.contains_key("cameTarget"),revision>=50.);
        if revision>=50. {assert_eq!(r.player.conditions.get("cameTurns"),Some(&1.));}
        let snapshot=Snapshot::capture(&r);let saved=storage::serialize(&r).unwrap();
        let mut restored=storage::deserialize(&saved).unwrap();
        snapshot.restore(&mut restored).unwrap();
        assert_eq!(restored.player.conditions.get("cameTurns"),r.player.conditions.get("cameTurns"));
        assert_eq!(restored.simulation_revision,r.simulation_revision);
    }
}
#[test]
fn reveal_survives_all_bonus_move_modes_then_ends_after_wait() {
    for revision in [50.,51.] {for mode in ["haste","came","pursuit","weave"] {
        let mut r=room(revision);
        match mode {
            "haste"=>{r.player.conditions.insert("hasted".into(),5.);},
            "came"=>{let id=target(&mut r);r.player.conditions.insert("cameTarget".into(),id);r.player.conditions.insert("cameTurns".into(),1.);},
            "pursuit"=>{r.player.conditions.insert("r50PursuitCredit".into(),1.);},
            "weave"=>{r.player.abilities.push(progression::WEAVE_ID.into());engine::dispatch(&mut r,&Command::Move{dx:-1.,dy:0.}).unwrap();},
            _=>unreachable!(),
        }
        reveal(&mut r);let tick=r.tick;
        assert!(!engine::dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap().took_turn,"{mode}");
        assert_eq!(r.tick,tick);assert_eq!(vision::all_seeing(&r),revision>=51.,"{mode}");
        engine::dispatch(&mut r,&Command::Wait).unwrap();assert!(!vision::all_seeing(&r));
    }}
}
#[test]
fn reveal_normal_actions_expire_and_no_action_or_failed_commands_preserve() {
    for command in [Command::Wait,Command::Move{dx:1.,dy:0.},Command::Use{id:9001.},ability(abilities::SLEEP)] {
        let mut r=room(51.);r.player.abilities.push(abilities::SLEEP.into());
        r.inventory.push(Item{id:9001.,name:"Heart".into(),kind:"heart".into(),glyph:"%".into(),..Default::default()});
        reveal(&mut r);assert!(engine::dispatch(&mut r,&command).unwrap().took_turn);assert!(!vision::all_seeing(&r));
    }
    let mut r=room(51.);reveal(&mut r);let i=10*r.floor().unwrap().width()+11;r.floor_mut().unwrap().tiles[i]="wall".into();
    for command in [Command::Move{dx:1.,dy:0.},ability(SHINE),Command::Pickup,at(abilities::SPARKLE,-1.,10.)] {
        assert!(!engine::dispatch(&mut r,&command).unwrap().took_turn);assert!(vision::all_seeing(&r));
    }
}
#[test]
fn future_snapshot_save_load_and_replay_preserve_new_reveal_boundary() {
    use corpse_core::replay::{ReplayRecord,ReplaySession};
    let mut initial=room(51.);initial.player.abilities.push("morae-see-the-future".into());initial.player.conditions.insert("hasted".into(),5.);
    let mut e=Engine::new(initial.clone()).unwrap();
    let commands=[ability(abilities::ALL_SEEING),ability("morae-see-the-future"),Command::Move{dx:1.,dy:0.},Command::Wait,ability("morae-see-the-future"),Command::Move{dx:0.,dy:1.},Command::Wait];
    for (i,c) in commands.iter().enumerate() {e.dispatch(c).unwrap();assert_eq!(vision::all_seeing(&e.run),!matches!(i,3|6));e=Engine::load(&e.save().unwrap()).unwrap();assert_eq!(vision::all_seeing(&e.run),!matches!(i,3|6));}
    let expected=e.save().unwrap();let mut replay=ReplaySession::from_initial(initial,ReplayRecord::try_from_run(&e.run).unwrap()).unwrap();
    while replay.step().unwrap() {} assert_eq!(replay.engine.save().unwrap(),expected);
    replay.restart().unwrap();while replay.step().unwrap() {}assert_eq!(replay.engine.save().unwrap(),expected);
    let mut r=room(51.);reveal(&mut r);let snapshot=Snapshot::capture(&r);temporal::remember_future(&mut r,&History::default()).unwrap();
    engine::dispatch(&mut r,&Command::Wait).unwrap();assert!(!vision::all_seeing(&r));
    assert!(temporal::restore_future(&mut r,&mut History::default()).unwrap());assert!(vision::all_seeing(&r));
    engine::dispatch(&mut r,&Command::Wait).unwrap();snapshot.restore(&mut r).unwrap();assert!(vision::all_seeing(&r));
}
#[test]
fn shine_r50_to_r51_cache_transition_and_free_toggle_are_exact() {
    let mut r=room(50.);r.player.active.push(SHINE.into());
    let legacy=r.lighting_cache.field(&r,false).unwrap();assert_eq!(legacy.at(Point{x:10,y:10}),100.);
    r.simulation_revision=51.0.into();let suppressed=r.lighting_cache.field(&r,false).unwrap();
    assert!(!Arc::ptr_eq(&legacy,&suppressed));assert_eq!(suppressed.at(Point{x:10,y:10}),-50.);
    assert!(vision::observe_terrain(&r).unwrap().light_sources.iter().all(|s|s.kind!="shine"));
    r.player.x=11.;assert!(Arc::ptr_eq(&suppressed,&r.lighting_cache.field(&r,false).unwrap()));
    engine::dispatch(&mut r,&ability(SHINE)).unwrap();let lit=r.lighting_cache.field(&r,false).unwrap();assert_eq!(lit.at(Point{x:11,y:10}),100.);
    engine::dispatch(&mut r,&ability(SHINE)).unwrap();let loaded=storage::deserialize(&storage::serialize(&r).unwrap()).unwrap();
    assert_eq!(vision::tile_light(&loaded,11,10),-50.);assert_eq!(loaded.lighting_cache.field(&loaded,false).unwrap().at(Point{x:11,y:10}),-50.);
}
#[test]
fn water_contact_clears_existing_source_cache_and_spread_fire_but_preserves_hellfire() {
    use corpse_core::propagation::FireScan;
    for revision in [50.,51.] {for tile in ["water","shallow-water"] {
        let mut r=room(revision);r.player.abilities.clear();let i=10*r.floor().unwrap().width()+15;
        environment::place_sparkle(&mut r,15.,10.).unwrap();let lit=r.lighting_cache.field(&r,false).unwrap();assert_eq!(lit.at(Point{x:15,y:10}),100.);
        r.floor_mut().unwrap().tiles[i]=tile.into();let n=r.floor().unwrap().tiles.len();
        let mut adapter=FireAdapter::new(&mut r,vec![false;n]).unwrap();adapter.advance().unwrap();
        if revision>=51. {no_fire(&r,i);assert_eq!(r.lighting_cache.field(&r,false).unwrap().at(Point{x:15,y:10}),-50.);}
        else {assert!(r.floor().unwrap().environment.as_ref().unwrap().fire[i]>0.);}
    }}
    let mut r=room(51.);let n=r.floor().unwrap().tiles.len();let width=r.floor().unwrap().width();let source=10*width+15;let wet=source+1;
    environment::place_sparkle(&mut r,15.,10.).unwrap();r.floor_mut().unwrap().tiles[wet]="water".into();
    r.floor_mut().unwrap().slime=vec![0.;n].into();r.floor_mut().unwrap().slime.as_mut().unwrap()[wet]=1.;
    let mut adapter=FireAdapter::new(&mut r,vec![false;n]).unwrap();FireScan::expose(&mut adapter,wet,source);adapter.resolve().unwrap();no_fire(&r,wet);
    r.floor_mut().unwrap().hellfire=vec![false;n].into();r.floor_mut().unwrap().hellfire.as_mut().unwrap()[wet]=true;
    environment::place_sparkle(&mut r,16.,10.).unwrap();let mut adapter=FireAdapter::new(&mut r,vec![false;n]).unwrap();adapter.advance().unwrap();
    assert!(r.floor().unwrap().hellfire.as_ref().unwrap()[wet]);assert!(r.floor().unwrap().environment.as_ref().unwrap().fire[wet]>0.);
}
#[test]
fn dream_sparkle_water_contact_does_not_change_waking_sources() {
    let mut r=room(51.);let i=10*r.floor().unwrap().width()+15;r.floor_mut().unwrap().tiles[i]="water".into();
    r.dreamwalking=dreamwalking::State{unlocked:true,..Default::default()}.into();
    dreamwalking::ability(&mut r,dreamwalking::DREAMWALK_ID).unwrap();reveal(&mut r);let waking=r.floor().unwrap().clone();
    engine::dispatch(&mut r,&at(abilities::SPARKLE,15.,10.)).unwrap();
    assert_eq!(r.floor().unwrap().tiles,waking.tiles);
    assert_eq!(r.floor().unwrap().environment,waking.environment);
    assert_eq!(r.floor().unwrap().light_sources,waking.light_sources);
    assert_eq!(r.floor().unwrap().permanent_fire,waking.permanent_fire);
    dreamwalking::with_plane(&mut r,true,|r|no_fire(r,i));
}
#[test]
fn restored_floor_sources_and_shine_rebuild_after_future_and_snapshot() {
    let mut r=room(51.);r.player.active.push(SHINE.into());
    environment::place_sparkle(&mut r,15.,10.).unwrap();
    let i=10*r.floor().unwrap().width()+15;let n=r.floor().unwrap().tiles.len();
    let source=r.lighting_cache.field(&r,false).unwrap();assert_eq!(source.at(Point{x:15,y:10}),100.);
    let snapshot=Snapshot::capture(&r);temporal::remember_future(&mut r,&History::default()).unwrap();
    r.floor_mut().unwrap().tiles[i]="water".into();
    {let mut adapter=FireAdapter::new(&mut r,vec![false;n]).unwrap();adapter.resolve().unwrap();}
    no_fire(&r,i);assert_eq!(vision::tile_light(&r,15,10),-50.);
    r.player.active.clear();assert!(vision::tile_light(&r,15,10)>0.);
    assert!(temporal::restore_future(&mut r,&mut History::default()).unwrap());
    assert!(r.player.active.iter().any(|id|id==SHINE));assert_eq!(vision::tile_light(&r,15,10),100.);
    assert_eq!(r.lighting_cache.field(&r,false).unwrap().at(Point{x:15,y:10}),100.);
    r.floor_mut().unwrap().tiles[i]="water".into();
    corpse_core::revision51_fire_water::extinguish_floor(r.floor_mut().unwrap(),51.);no_fire(&r,i);
    snapshot.restore(&mut r).unwrap();assert_eq!(vision::tile_light(&r,15,10),100.);
    assert_eq!(r.lighting_cache.field(&r,false).unwrap().at(Point{x:15,y:10}),100.);
}
#[test]
fn inactive_floor_water_cleanup_uses_same_rule_without_relighting() {
    let mut r=room(51.);r.player.abilities.clear();environment::place_sparkle(&mut r,15.,10.).unwrap();
    let i=10*r.floor().unwrap().width()+15;
    let mut floor=r.floor().unwrap().clone();floor.depth=2.;floor.last_tick=r.tick;floor.tiles[i]="water".into();floor.dirty=true;
    r.floors.retain(|f|f.depth!=2.);r.floors.push(Arc::new(floor));r.actors_at_mut(2.).clear();r.ground_at_mut(2.).clear();r.tick+=2.;
    corpse_core::offscreen::catch_up_floor(&mut r,2.).unwrap();
    let native_depth=r.depth;r.depth=2.;no_fire(&r,i);assert_eq!(vision::tile_light(&r,15,10),-50.);r.depth=native_depth;
}
