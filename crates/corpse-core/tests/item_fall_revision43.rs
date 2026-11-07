//! Revision43 item/fall changes, with explicit revision42 preservation.
use corpse_core::{borge, engine, inventory, royal_honey, travel, Command, Field, Run};
use std::sync::Arc;

fn room(revision:f64)->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=revision.into();r.status="playing".into();r.depth=1.;r.tick=0.;
    r.inventory.clear();r.actors.clear();r.ground.clear();r.history.clear();r.haste_snapshot=Field::Missing;
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.ability_order=Field::Missing;
    r.player.x=8.;r.player.y=8.;r.player.base_max_hp=100.0.into();r.player.hp=100.;r.player.max_hp=100.;
    r.player.stats.defense=0.;r.player.stats.max_hp=Field::Missing;r.player.energy=100.;r.player.stats.max_energy=100.;
    r.phase=Field::Missing;r.offers.clear();r.replay.clear();r.next_id=1000.;
    let mut f=r.floors[0].as_ref().clone();f.environment=Field::Missing;f.vents=Field::Missing;
    f.permanent_fire=Field::Missing;f.godbeast_event=Field::Missing;f.godbeast=false;f.doors=Field::Missing;
    let w=f.width();let h=f.height();
    for y in 1..h-1 {for x in 1..w-1 {f.tiles[y*w+x]="floor".into();}}
    f.tiles[w+1]="up".into();f.tiles[(h-2)*w+w-2]="down".into();
    r.floors=(1..=10).map(|depth|{let mut f=f.clone();f.depth=depth as f64;Arc::new(f)}).collect();
    for depth in 1..=10 {r.actors.insert(depth.to_string(),Arc::new(vec![]));r.ground.insert(depth.to_string(),Arc::new(vec![]));}
    r
}
fn chasm(r:&mut Run,x:usize,y:usize){let f=r.floor_mut().unwrap();let w=f.width();f.tiles[y*w+x]="chasm".into();}
fn add(r:&mut Run,kind:&str)->f64 {
    let id=r.next_id;r.next_id+=1.;
    let mut a=corpse_core::population::create_actor(kind,id,9.,8.,false).unwrap();a.state="hunting".into();
    r.actors_at_mut(1.).push(a);id
}
#[test]
fn honey_adds_9999_instead_of_full_restore_only_from43_and_never_increases_caps(){
    for revision in [42.,43.] {
        let mut r=room(revision);r.player.base_max_hp=50000.0.into();r.player.max_hp=50000.;r.player.hp=3.;
        r.player.stats.max_energy=60000.;r.player.energy=4.;let rng=r.rng;
        assert!(royal_honey::eat(&mut r));
        assert_eq!(r.player.hp,if revision==42.{50000.}else{10002.});
        assert_eq!(r.player.energy,if revision==42.{60000.}else{10003.});
        assert_eq!((r.player.max_hp,r.player.stats.max_energy),(50000.,60000.));assert_eq!(r.rng,rng);
    }
    let mut r=room(43.);r.player.hp=1.;r.player.energy=2.;assert!(royal_honey::eat(&mut r));
    assert_eq!((r.player.hp,r.player.energy),(100.,100.));
    r.player.dead=true.into();r.player.hp=0.;let before=r.clone();assert!(!royal_honey::eat(&mut r));assert_eq!(r,before);
}
#[test]
fn honey_ground_and_pack_consumption_are_once_and_save_replay_exact(){
    for ground in [false,true] {
        let mut r=room(43.);r.player.hp=1.;r.player.energy=2.;
        if ground {r.ground_at_mut(1.).push(corpse_core::GroundItem{x:8.,y:8.,item:royal_honey::item(900.),..Default::default()});}
        else {r.inventory.push(royal_honey::item(900.));}
        assert_eq!(inventory::command(&mut r,&Command::Use{id:900.}).unwrap(),Some(true));
        assert!(!r.inventory.iter().any(royal_honey::is_honey));assert!(r.ground["1"].is_empty());
        let before=r.clone();assert_eq!(inventory::command(&mut r,&Command::Use{id:900.}).unwrap(),Some(false));assert_eq!(r,before);
        let loaded=corpse_core::storage::deserialize(&corpse_core::storage::serialize(&r).unwrap()).unwrap();
        assert_eq!(loaded.player,r.player);assert_eq!(loaded.inventory,r.inventory);
    }
    let mut r=room(43.);r.player.hp=1.;r.player.energy=2.;r.inventory.push(royal_honey::item(900.));
    let initial=r.clone();let mut e=corpse_core::temporal::Engine::new(r).unwrap();
    e.dispatch(&Command::Use{id:900.}).unwrap();let record=corpse_core::replay::ReplayRecord::from_run(&e.run);
    let mut replay=corpse_core::replay::ReplaySession::from_initial(initial,record).unwrap();while replay.step().unwrap(){}
    assert_eq!(replay.engine.save().unwrap(),e.save().unwrap());
    let mut r=room(43.);r.player.hp=1.;r.player.energy=2.;r.inventory.push(royal_honey::item(900.));
    let mut history=corpse_core::temporal::History::default();
    corpse_core::temporal::remember_future(&mut r,&history).unwrap();
    // Future spends current energy before capturing its anchor; Honey must not
    // refund that persistent cost when the anchor is restored.
    assert_eq!(r.player.energy,0.);
    let captured_costs=r.time_costs.clone();
    assert_eq!(inventory::command(&mut r,&Command::Use{id:900.}).unwrap(),Some(true));
    assert!(corpse_core::temporal::restore_future(&mut r,&mut history).unwrap());
    assert_eq!((r.player.hp,r.player.energy),(1.,0.));assert_eq!(r.time_costs,captured_costs);assert!(r.inventory.iter().any(royal_honey::is_honey));
}
#[test]
fn voluntary_chasm_cost_is25_percent_max_rounded_up_but_forced_fall_is_unchanged(){
    for revision in [42.,43.] {for voluntary in [false,true] {
        let mut r=room(revision);r.player.base_max_hp=101.0.into();r.player.max_hp=101.;r.player.hp=101.;chasm(&mut r,8,8);
        assert!(if voluntary{travel::jump_chasm(&mut r)}else{travel::travel(&mut r,1.,true,false)}.unwrap());
        assert_eq!(r.depth,2.);assert_eq!(r.player.hp,if revision>=43.&&voluntary{75.}else{94.});
    }}
    let mut r=room(43.);chasm(&mut r,8,8);let f=Arc::make_mut(&mut r.floors[1]);let w=f.width();f.tiles[8*w+8]="water".into();
    assert!(travel::jump_chasm(&mut r).unwrap());assert_eq!(r.player.hp,100.);
    let mut r=room(43.);let before=r.clone();assert!(!travel::jump_chasm(&mut r).unwrap());assert_eq!(r,before);
}
#[test]
fn actual_descend_move_and_forced_move_route_distinct_costs_without_weave(){
    for command in [Command::Descend,Command::Move{dx:1.,dy:0.}] {
        let mut r=room(43.);chasm(&mut r,if matches!(command,Command::Descend){8}else{9},8);
        let out=engine::dispatch(&mut r,&command).unwrap();assert!(out.took_turn);assert_eq!(r.depth,2.);
        // Paid dispatch runs ordinary 0.2%-maxHP regeneration after the 25 HP
        // fall packet. The direct-travel test above isolates packet rounding.
        assert_eq!(r.player.hp,75.+100.*0.002);assert!(r.timeweave.is_missing());
    }
    let mut r=room(43.);chasm(&mut r,9,8);r.player.conditions.insert("forcedMove".into(),1.);
    let id=r.player.id;assert!(engine::move_actor(&mut r,id,1.,0.,false).unwrap());assert_eq!(r.depth,2.);assert_eq!(r.player.hp,93.);
    let mut r=room(43.);let f=r.floor_mut().unwrap();let w=f.width();f.tiles[8*w+9]="wall".into();
    let before=(r.depth,r.player.hp,r.player.x,r.player.y,r.tick);let out=engine::dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap();
    assert!(!out.took_turn);assert_eq!((r.depth,r.player.hp,r.player.x,r.player.y,r.tick),before);
}
#[test]
fn non_weave43_command_trace_keeps_haste_and_came_free_phase_behavior(){
    for condition in ["hasted","cameTurns"] {
        let mut a=room(42.);a.player.conditions.insert(condition.into(),10.);
        if condition=="hasted" {a.phase="haste".to_string().into();}
        else {let target=add(&mut a,"rat");engine::actor_mut(&mut a,target).unwrap().x=12.;a.player.conditions.insert("cameTarget".into(),target);}
        let mut b=a.clone();b.simulation_revision=43.0.into();
        let command=Command::Move{dx:1.,dy:0.};let ao=engine::dispatch(&mut a,&command).unwrap();let bo=engine::dispatch(&mut b,&command).unwrap();
        assert!(!ao.took_turn);assert_eq!(ao.took_turn,bo.took_turn);assert_eq!(a.player,b.player);assert_eq!(a.phase,b.phase);assert_eq!(a.tick,b.tick);assert_eq!(a.rng,b.rng);assert_eq!(a.log,b.log);assert!(b.timeweave.is_missing());
    }
}
#[test]
fn shard43_is_equippable_melee_success_consumes_miss_and_protection_do_not(){
    for outcome in ["hit","miss","protected"] {
        let mut r=room(43.);let target=add(&mut r,"phoenix");r.inventory.push(borge::shard(900.));
        assert_eq!(inventory::command(&mut r,&Command::Equip{id:900.,slot:"hand1".into()}).unwrap(),Some(true));
        assert_eq!(r.inventory.iter().find(|i|i.id==900.).unwrap().equipped.as_ref().map(String::as_str),Some("hand1"));
        if outcome=="miss" {r.player.conditions.insert("unlucky".into(),1.);} else {r.player.conditions.insert("lucky".into(),1.);}
        if outcome=="protected" {engine::actor_mut(&mut r,target).unwrap().active.push("morae-turning-the-wheel".into());}
        engine::player_attack(&mut r,target).unwrap();
        let a=engine::actor(&r,target).unwrap();assert_eq!(a.dead.value_or(false),outcome=="hit");
        assert_eq!(r.inventory.iter().any(|i|i.id==900.),outcome!="hit");
        if outcome=="hit" {assert!(a.abilities.iter().any(|x|x=="enemy:beast:no-revival"));}
    }
}
// Append to tests/item_fall_revision43.rs; reuses its room/chasm helpers.
// PortalPair construction follows portal_integration_v35.rs. Generated endpoints
// start on floor/grass/shallow-water, but stored endpoints may later have changed
// terrain: validate_floor deliberately validates geometry, not endpoint terrain.
#[test]
fn portal_resolved_chasm_entry_keeps_legacy_packet_while_direct_entry_costs25_percent() {
    for (revision, portal) in [(42.,true),(43.,true),(43.,false)] {
        let mut r=room(revision);
        // A portal is admitted only from floor10. Precreate the full lookahead
        // batch so this routing regression does not generate unrelated floors.
        let template=r.floors[0].as_ref().clone();
        for depth in 11..=20 {
            let mut f=template.clone();f.depth=depth as f64;
            r.floors.push(Arc::new(f));
            r.actors.insert(depth.to_string(),Arc::new(vec![]));
            r.ground.insert(depth.to_string(),Arc::new(vec![]));
        }
        r.depth=10.;
        let landing=if portal {(40,15)}else{(9,8)};
        chasm(&mut r,landing.0,landing.1);
        if portal {
            let f=r.floor_mut().unwrap();let w=f.width();
            f.portals=vec![corpse_core::portals_generation::PortalPair {
                a:8*w+9,b:landing.1*w+landing.0,color_index:0,
            }].into();
            assert!(corpse_core::portals_generation::validate_floor(f));
        }
        assert_eq!(corpse_core::portals::step_options(&r,&r.player,9.,8.),
            vec![(landing.0 as f64,landing.1 as f64)]);
        let out=engine::dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap();
        assert!(out.took_turn);assert_eq!(r.depth,11.);
        assert_eq!((r.player.x,r.player.y),(landing.0 as f64,landing.1 as f64));
        // ceil(100 * .25 * legacy fall scale .25)=7 vs exact new25,
        // followed by the same normal paid-turn .2HP regeneration.
        assert_eq!(r.player.hp,if portal {93.2}else{75.2},"revision={revision}, portal={portal}");
        assert!(r.timeweave.is_missing());
    }
}
