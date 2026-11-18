use super::*;
use crate::{Command, Stats, ai, engine};

fn room() -> Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=52.0.into();r.player.x=10.;r.player.y=10.;
    r.player.hp=10000.;r.player.max_hp=10000.;r.player.base_max_hp=10000.0.into();
    r.player.stats=Stats{perception:300.,max_energy:100.,..Default::default()};
    r.player.energy=100.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.equipment_abilities=Field::Missing;r.player.ability_order=Field::Missing;
    r.player.ability_reserves=Field::Missing;r.player.amulet_ledger=Field::Missing;
    r.inventory=vec![make_ring(900.)];
    for a in r.actors.values_mut(){*a=Default::default();}
    for g in r.ground.values_mut(){*g=Default::default();}
    r.history.clear();r.replay.clear();r.haste_snapshot=Field::Missing;r.future=None;
    r.phase="normal".to_string().into();r.fast_action_phase=Field::Missing;
    r.next_id=1000.;r.offers.clear();
    let f=r.floor_mut().unwrap();
    f.tiles.fill("floor".into());f.lights.fill(100.);f.seen.fill(true);
    f.doors=Field::Missing;f.portals=Field::Missing;f.non_euclidean=Field::Missing;
    f.speed_up=Field::Missing;f.environment=Field::Missing;f.vents=Field::Missing;
    f.currents=Field::Missing;f.mud=Field::Missing;f.slime=Field::Missing;f.roots=Field::Missing;
    f.ichor=Field::Missing;f.grates=Field::Missing;f.permanent_fire=Field::Missing;
    f.razorleaves=Field::Missing;f.godbeast_event=Field::Missing;
    f.ice=Field::Missing;f.hellfire=Field::Missing;f.light_sources=Field::Missing;
    f.revision50_terrain=Field::Missing;f.revision50_temporary_terrain=Field::Missing;
    r
}
fn at(r:&Run,x:usize,y:usize)->usize {y*r.floor().unwrap().width()+x}
fn tile(r:&mut Run,x:usize,y:usize,value:&str) {let i=at(r,x,y);r.floor_mut().unwrap().tiles[i]=value.into();}
fn step(r:&mut Run,dx:i32,dy:i32)->crate::engine::DispatchOutcome {
    engine::dispatch(r,&Command::Move{dx:dx as f64,dy:dy as f64}).unwrap()
}

#[test]
fn named_ring_uses_existing_inventory_and_inspection_without_random_pool_change() {
    let mut r=room();let item=&r.inventory[0];
    crate::equipment::validate(item).unwrap();
    assert_eq!(crate::revision50_rings::effect(item),Some("swimming"));
    assert_eq!(crate::revision50_item_knowledge::display_name(&r,item),"Ring of Swimming");
    assert!(crate::revision50_rings::description(item).unwrap().contains("shallow or deep water"));
    assert_eq!(crate::revision50_inventory::space_for(&r,item),4.);
    // Named R50 groups remain fourteen, including their one-draw distribution.
    let rng=r.rng;let next=r.next_id;
    assert_eq!(make_ring(901.).effect,"swimming".to_string().into());
    assert_eq!(r.rng,rng);assert_eq!(r.next_id,next);
    r.inventory[0].name="stale display string".into();
    assert_eq!(crate::revision50_item_knowledge::display_name(&r,&r.inventory[0]),"Ring of Swimming");
}

#[test]
fn shallow_and_deep_contact_but_not_air_land_ichor_or_legacy() {
    for t in ["water","shallow-water"] {
        let mut r=room();tile(&mut r,10,10,t);
        assert!(ai::is_hasted_in(&r,&r.player));
        r.player.conditions.insert("flying".into(),1.);assert!(!ai::is_hasted_in(&r,&r.player));
        r.player.conditions.clear();let i=at(&r,10,10);let n=r.floor().unwrap().tiles.len();
        r.floor_mut().unwrap().ichor=vec![0.;n].into();r.floor_mut().unwrap().ichor.as_mut().unwrap()[i]=1.;
        assert!(!ai::is_hasted_in(&r,&r.player));r.floor_mut().unwrap().ichor=Field::Missing;
        r.simulation_revision=51.0.into();assert!(!ai::is_hasted_in(&r,&r.player));
    }
    let r=room();assert!(!ai::is_hasted_in(&r,&r.player));
}

#[test]
fn entry_and_exit_have_no_lingering_haste_status_or_extra_bonus() {
    for water in ["water","shallow-water"] {
        let mut r=room();tile(&mut r,11,10,water);
        let tick=r.tick;assert!(step(&mut r,1,0).took_turn);assert_eq!(r.tick,tick+1.);
        assert!(ai::is_hasted_in(&r,&r.player));
        assert!(!step(&mut r,1,0).took_turn);assert_eq!(r.tick,tick+1.);
        assert!(!ai::is_hasted_in(&r,&r.player));
        assert!(step(&mut r,1,0).took_turn);assert_eq!(r.tick,tick+2.);
        assert_eq!(engine::condition(&r.player,"hasted"),0.);
        assert_eq!(engine::condition(&r.player,"hastedFresh"),0.);
    }
}

#[test]
fn extra_copies_and_existing_haste_do_not_stack_slowing_still_wins() {
    let mut r=room();for x in 10..=14 {tile(&mut r,x,10,"water");}
    r.inventory.push(make_ring(901.));r.player.conditions.insert("hasted".into(),5.);
    r.phase="haste".to_string().into();let tick=r.tick;
    assert!(!step(&mut r,1,0).took_turn);assert_eq!(r.tick,tick);
    assert_eq!(engine::condition(&r.player,"hasted"),5.);
    assert!(step(&mut r,1,0).took_turn);assert_eq!(r.tick,tick+1.);
    assert_eq!(engine::condition(&r.player,"hasted"),4.);
    r.player.conditions.insert("slowed".into(),3.);assert!(!ai::is_hasted_in(&r,&r.player));
}

#[test]
fn full_haste_move_and_wait_skip_current_tick_but_normal_wait_drifts() {
    let mut r=room();for y in 9..=12 {for x in 9..=12 {tile(&mut r,x,y,"water");}}
    let n=r.floor().unwrap().tiles.len();r.floor_mut().unwrap().currents=vec![5.;n].into();
    r.phase="haste".to_string().into();let tick=r.tick;
    assert!(!step(&mut r,1,0).took_turn);assert_eq!((r.player.x,r.player.y),(11.,10.));assert_eq!(r.tick,tick);
    assert!(step(&mut r,1,0).took_turn);assert_eq!((r.player.x,r.player.y),(12.,11.));assert_eq!(r.tick,tick+1.);
    assert!(!engine::dispatch(&mut r,&Command::Wait).unwrap().took_turn);
    assert_eq!((r.player.x,r.player.y),(12.,11.));assert_eq!(r.tick,tick+1.);
    assert_eq!(r.phase.as_ref().map(String::as_str),Some("normal"));
    assert!(engine::dispatch(&mut r,&Command::Wait).unwrap().took_turn);
    assert_eq!((r.player.x,r.player.y),(12.,12.));assert_eq!(r.tick,tick+2.);
}

#[test]
fn every_baseline_move_and_wait_transition_matches_true_haste_engine() {
    for owned in [false,true] {for haste in [false,true] {for contact in ["floor","shallow-water","water"] {
        let mut base=room();if !owned {base.inventory.clear();}
        let w=base.floor().unwrap().width();
        // Mixed arrival terrain covers entering, staying in, and leaving water.
        for y in 9..=12 {for x in 9..=12 {
            tile(&mut base,x,y,if (x+y)%3==0 {"floor"} else if (x+y)%3==1 {"shallow-water"} else {"water"});
        }}
        tile(&mut base,10,10,contact);
        let n=base.floor().unwrap().tiles.len();base.floor_mut().unwrap().currents=vec![5.;n].into();
        base.phase=if haste {"haste"} else {"normal"}.to_string().into();
        let f=base.floor().unwrap();let g=Geometry::new(f).unwrap();let edges=mobility(f,&g,owned);
        let state=(10*w+10)*2+usize::from(haste);
        for expected in &edges[state] {
            let mut r=base.clone();let tick=r.tick;
            let command=if expected.dx==0&&expected.dy==0 {Command::Wait}
                else {Command::Move{dx:expected.dx as f64,dy:expected.dy as f64}};
            let result=engine::dispatch(&mut r,&command).unwrap();
            let actual=(r.player.y as usize*w+r.player.x as usize)*2
                +usize::from(r.phase.as_ref().is_some_and(|p|p=="haste"));
            let bonus=owned&&haste&&contact!="floor";
            assert_eq!(actual,expected.to,"owned={owned} haste={haste} contact={contact} command={command:?}");
            assert_eq!(result.took_turn,!bonus);assert_eq!(r.tick,tick+if bonus {0.} else {1.});
            assert!(r.fast_action_phase.is_missing(),"Swimming alone must not create a Fast continuation");
        }
        assert_eq!(edges[state].len(),9,"all eight legal moves and Wait are exercised");
    }}}
}

#[test]
fn carried_ring_does_not_float_away_and_capacity_requires_real_drop() {
    let mut r=room();tile(&mut r,10,10,"water");
    for _ in 0..32 {assert!(!crate::inventory::float_carried_item(&mut r).unwrap());}
    assert_eq!(r.inventory.len(),1);
    // The safeguard occupies the island's ONLY dry square. Freeing capacity
    // must not require another land cell or automatically reacquire the old ring.
    for y in 7..=13 {for x in 7..=13 {tile(&mut r,x,y,"water");}}
    tile(&mut r,10,10,"floor");r.inventory.clear();
    for id in 901..906 {let mut item=make_ring(id as f64);item.effect="protection".to_string().into();r.inventory.push(item);}
    r.ground_at_mut(r.depth).push(GroundItem{x:10.,y:10.,item:make_ring(900.),..Default::default()});
    assert_eq!(crate::inventory::collect_underfoot(&mut r,20.).unwrap(),0);
    assert!(!crate::equipment::unequip(&mut r,901.).unwrap());
    assert_eq!(crate::revision50_inventory::space_for(&r,&make_ring(900.)),0.);
    let tick=r.tick;assert!(engine::dispatch(&mut r,&Command::Drop{id:901.}).unwrap().took_turn);
    assert_eq!(r.tick,tick+1.);
    assert_eq!(r.inventory.len(),5);assert!(r.inventory.iter().any(|i|i.id==900.));
    assert!(!r.inventory.iter().any(|i|i.id==901.));
    assert_eq!((r.player.x,r.player.y),(10.,10.));
    assert!(r.ground[&r.depth.to_string()].iter().any(|g|g.item.id==901.&&(g.x,g.y)!=(10.,10.)));
    assert!(!engine::dispatch(&mut r,&Command::Pickup).unwrap().took_turn);
    assert_eq!(r.tick,tick+1.);assert_eq!(r.inventory.len(),5);
}

#[test]
fn owner_local_water_half_here_and_stats_only_copy_rules_are_preserved() {
    let mut r=room();tile(&mut r,12,10,"water");
    let mut clone=r.player.clone();clone.id=902.;clone.x=12.;
    clone.conditions.insert("halfHereClone".into(),1.);clone.conditions.insert("halfHereOwner".into(),r.player.id);
    assert!(ai::is_hasted_in(&r,&clone));assert!(!ai::is_hasted_in(&r,&r.player));
    clone.conditions.clear();clone.copied_inventory=vec![make_ring(903.)].into();
    assert!(ai::is_hasted_in(&r,&clone));
    clone.conditions.insert("miniMe".into(),1.);assert!(!ai::is_hasted_in(&r,&clone));
}

#[test]
fn save_load_and_snapshot_restore_recompute_conditional_haste() {
    let mut r=room();tile(&mut r,10,10,"water");r.phase="haste".to_string().into();
    let before=crate::temporal::Snapshot::capture(&r);
    let raw=crate::storage::serialize(&r).unwrap();let mut loaded=crate::storage::deserialize(&raw).unwrap();
    assert!(ai::is_hasted_in(&loaded,&loaded.player));
    let tick=loaded.tick;assert!(!step(&mut loaded,1,0).took_turn);assert_eq!(loaded.tick,tick);
    assert!(!ai::is_hasted_in(&loaded,&loaded.player));
    before.restore(&mut loaded).unwrap();assert!(ai::is_hasted_in(&loaded,&loaded.player));
    assert_eq!(loaded.phase.as_ref().map(String::as_str),Some("haste"));
    loaded.inventory.clear();assert!(!ai::is_hasted_in(&loaded,&loaded.player));
}

#[test]
fn see_the_future_roundtrip_restores_ring_contact_and_original_haste_phase() {
    let mut r=room();r.future_floor_rules_from=Field::Null;tile(&mut r,10,10,"water");r.phase="haste".to_string().into();
    r.player.abilities.push("morae-see-the-future".into());
    crate::revision50_ability_energy::reconcile_acquired(&mut r).unwrap();
    let mut e=crate::temporal::Engine::new(r).unwrap();
    let future=Command::Ability{id:"morae-see-the-future".into(),target:Field::Missing,x:Field::Missing,y:Field::Missing};
    e.dispatch(&future).unwrap();assert!(e.run.future.is_some());
    let tick=e.run.tick;
    assert!(!e.dispatch(&Command::Move{dx:1.,dy:0.}).unwrap().took_turn);
    assert!(!ai::is_hasted_in(&e.run,&e.run.player));
    let mut loaded=crate::temporal::Engine::load_replay_checkpoint(&e.save().unwrap()).unwrap();
    loaded.dispatch(&future).unwrap();assert!(loaded.run.future.is_none());
    assert_eq!((loaded.run.player.x,loaded.run.player.y),(10.,10.));assert_eq!(loaded.run.tick,tick);
    assert_eq!(loaded.run.phase.as_ref().map(String::as_str),Some("haste"));
    assert!(ai::is_hasted_in(&loaded.run,&loaded.run.player));
    assert!(!loaded.dispatch(&Command::Move{dx:0.,dy:1.}).unwrap().took_turn);
    assert_eq!(loaded.run.tick,tick);
}

#[test]
fn timeweave_return_to_water_preserves_phase_without_minting_timed_haste() {
    let mut r=room();r.player.abilities.push(crate::progression::WEAVE_ID.into());
    tile(&mut r,10,10,"water");r.phase="haste".to_string().into();
    let tick=r.tick;assert!(!step(&mut r,1,0).took_turn);
    assert!(!ai::is_hasted_in(&r,&r.player));
    let phase=r.phase.clone();let rng=r.rng;
    assert!(!step(&mut r,-1,0).took_turn);
    assert_eq!((r.player.x,r.player.y),(10.,10.));assert_eq!(r.phase,phase);
    assert_eq!(r.tick,tick);assert_eq!(r.rng,rng);assert!(ai::is_hasted_in(&r,&r.player));
    assert_eq!(engine::condition(&r.player,"hasted"),0.);
    // Returning by Weave retained normal phase; the ring does not create a
    // second bonus opportunity merely because the destination is wet again.
    assert!(step(&mut r,0,1).took_turn);assert_eq!(r.tick,tick+1.);
}

fn island_floor() -> Floor {
    let mut f=Floor{depth:1.,tiles:vec!["floor".into();225],..Default::default()};
    for y in 0..15 {for x in 0..15 {if x==0||y==0||x==14||y==14 {f.tiles[y*15+x]="wall".into();}}}
    for y in 3..12 {for x in 3..12 {f.tiles[y*15+x]="water".into();}}
    for y in 6..9 {for x in 6..9 {f.tiles[y*15+x]="floor".into();}}
    f.tiles[16]="up".into();f
}

// The one-cell entry channel carries westward attempts back east. The carrier
// can remain in water at x=4, use a bonus step to x=3, then reach shore at x=2.
fn trapped_island() -> Floor {
    let mut f=Floor{depth:1.,tiles:vec!["wall".into();225],..Default::default()};
    f.tiles[7*15+1]="up".into();f.tiles[7*15+2]="floor".into();
    f.tiles[7*15+3]="water".into();f.tiles[7*15+4]="water".into();
    for y in 6..9 {for x in 5..8 {f.tiles[y*15+x]="floor".into();}}
    let mut currents=vec![0.;225];currents[7*15+3]=3.;f.currents=currents.into();f
}

#[test]
fn only_trapped_island_gets_safe_ring_without_terrain_or_loot_mutation() {
    assert!(placement_cells(&island_floor(),&[],&[]).unwrap().is_empty());
    let f=trapped_island();let before=serde_json::to_value(&f).unwrap();
    let placements=placement_cells(&f,&[],&[]).unwrap();assert_eq!(placements,vec![6*15+5]);
    assert_eq!(placement_cells(&f,&[],&[]).unwrap(),placements);assert_eq!(serde_json::to_value(&f).unwrap(),before);
    let ground=vec![GroundItem{x:5.,y:6.,item:make_ring(900.),..Default::default()}];
    assert!(placement_cells(&f,&[],&ground).unwrap().is_empty());
    let occupied=vec![GroundItem{x:5.,y:6.,item:Item::default(),..Default::default()}];
    assert_eq!(placement_cells(&f,&[],&occupied).unwrap(),vec![6*15+6]);
}

#[test]
fn land_and_shallow_bridges_and_legal_diagonals_prevent_false_islands() {
    for bridge in ["floor","shallow-water"] {
        let mut f=island_floor();for x in 2..=6 {f.tiles[7*15+x]=bridge.into();}
        assert_eq!(Geometry::new(&f).unwrap().components.len(),1);
        assert!(placement_cells(&f,&[],&[]).unwrap().is_empty());
    }
    let mut f=island_floor();for (x,y) in [(3,3),(4,4),(5,5)] {f.tiles[y*15+x]="floor".into();}
    assert_eq!(Geometry::new(&f).unwrap().components.len(),1);
    f.tiles[4*15+5]="wall".into();
    assert_eq!(Geometry::new(&f).unwrap().components.len(),2);
    // Disconnected land is still ordinarily swimmable, so no ring is due.
    assert!(placement_cells(&f,&[],&[]).unwrap().is_empty());
}

#[test]
fn single_shore_and_shallow_speck_are_not_trapped_islands() {
    let mut f=island_floor();for y in 6..9 {for x in 6..9 {f.tiles[y*15+x]="water".into();}}
    assert!(placement_cells(&f,&[],&[]).unwrap().is_empty());
    f.tiles[7*15+7]="shallow-water".into();
    assert!(placement_cells(&f,&[],&[]).unwrap().is_empty());
}

#[test]
fn sealed_dry_pocket_is_not_island_but_inaccessible_water_island_rejects() {
    let mut f=island_floor();f.tiles.fill("wall".into());f.tiles[16]="up".into();f.tiles[17]="floor".into();
    f.tiles[7*15+7]="floor".into();assert!(placement_cells(&f,&[],&[]).unwrap().is_empty());
    f.tiles[7*15+8]="water".into();
    assert!(placement_cells(&f,&[],&[]).unwrap_err().starts_with("Isolated island"));
}

#[test]
fn occupied_hazardous_and_boundary_targets_are_not_silently_relocated() {
    let f=trapped_island();let g=Geometry::new(&f).unwrap();let island=&g.components[g.component[6*15+5]];
    let actors:Vec<_>=island.iter().enumerate().map(|(i,&at)|Actor{id:100.+i as f64,x:(at%15) as f64,y:(at/15) as f64,..Default::default()}).collect();
    assert!(placement_cells(&f,&actors,&[]).unwrap_err().starts_with("Isolated island"));
    let mut hazardous=f.clone();hazardous.permanent_fire=island.iter().map(|&i|i as f64).collect::<Vec<_>>().into();
    assert!(placement_cells(&hazardous,&[],&[]).unwrap_err().starts_with("Isolated island"));
    for at in placement_cells(&f,&[],&[]).unwrap(){assert!((1..14).contains(&(at%15))&&(1..14).contains(&(at/15)));}
}

fn distances_to_other(g:&Geometry,edges:&[Vec<Step>],own:usize)->Vec<usize> {
    let mut incoming=vec![Vec::new();edges.len()];
    for (from,steps) in edges.iter().enumerate(){for step in steps{incoming[step.to].push(from);}}
    let mut distances=vec![usize::MAX;edges.len()];let mut queue=VecDeque::new();
    for (at,&label) in g.component.iter().enumerate(){if g.passable[at]&&label!=usize::MAX&&label!=own {
        for state in [at*2,at*2+1]{distances[state]=0;queue.push_back(state);}
    }}
    while let Some(to)=queue.pop_front(){for &from in &incoming[to]{if distances[from]==usize::MAX{distances[from]=distances[to]+1;queue.push_back(from);}}}
    distances
}

#[test]
fn two_label_proof_matches_independent_per_island_bfs_across_square_currents() {
    for seed in 0..32 {
        let mut f=island_floor();crate::square_currents::generate(&mut f,seed);
        let g=Geometry::new(&f).unwrap();
        for owned in [false,true] {
            let edges=mobility(&f,&g,owned);let labels=escape_labels(&f,&g,&edges);
            for (own,cells) in g.components.iter().enumerate() {
                let distances=distances_to_other(&g,&edges,own);
                for &at in cells {for phase in [0,1] {
                    assert_eq!(reaches_other(labels[at*2+phase],own),distances[at*2+phase]!=usize::MAX);
                }}
            }
        }
        // Ordinary square circulation in this broad lake has escape routes.
        assert!(placement_cells(&f,&[],&[]).unwrap().is_empty());
    }
}

#[test]
fn validated_escape_actions_replay_through_real_engine_normal_and_haste_paths() {
    for phase in [0,1] {
        let mut r=room();let w=r.floor().unwrap().width();
        {let f=r.floor_mut().unwrap();f.tiles.fill("wall".into());
            f.tiles[10*w+1]="up".into();f.tiles[10*w+2]="floor".into();
            f.tiles[10*w+3]="water".into();f.tiles[10*w+4]="water".into();f.tiles[10*w+5]="floor".into();
            let mut currents=vec![0.;f.tiles.len()];currents[10*w+3]=3.;f.currents=currents.into();
        }
        let f=r.floor().unwrap();let g=Geometry::new(f).unwrap();let edges=mobility(f,&g,true);
        let at=placement_cells(f,&[],&[]).unwrap()[0];let own=g.component[at];let distances=distances_to_other(&g,&edges,own);
        r.player.x=(at%w) as f64;r.player.y=(at/w) as f64;
        r.phase=if phase==1 {"haste"} else {"normal"}.to_string().into();let mut state=at*2+phase;
        while distances[state]>0 {
            let next=*edges[state].iter().find(|s|distances[s.to]<distances[state]).unwrap();
            if next.dx==0&&next.dy==0 {engine::dispatch(&mut r,&Command::Wait).unwrap();} else {step(&mut r,next.dx,next.dy);}
            let actual=(r.player.y as usize*w+r.player.x as usize)*2+usize::from(r.phase.as_ref().is_some_and(|p|p=="haste"));
            assert_eq!(actual,next.to);state=actual;
        }
        assert_ne!(g.component[state/2],own);assert_ne!(g.component[state/2],usize::MAX);
    }
}

#[test]
fn multi_step_spatial_interiors_reach_ordinary_ground_without_lucky_branches() {
    use crate::non_euclidean_generation::{NonEuclideanGroup,NonEuclideanPocket};
    let mut f=island_floor();f.depth=20.;
    // A broad dry shore, with a 3x3 stochastic pocket away from the lake.
    f.tiles.fill("floor".into());f.tiles[16]="up".into();
    let cells:Vec<_>=(4..7).flat_map(|y|(4..7).map(move |x|y*15+x)).collect();
    let groups=(4..7).map(|y|NonEuclideanGroup{sources:vec![y*15+4,y*15+5],destination:y*15+6}).collect();
    f.non_euclidean=vec![NonEuclideanPocket{cells:cells.clone(),groups}].into();
    assert!(crate::non_euclidean_generation::validate_floor(&f));
    let before=serde_json::to_value(&f).unwrap();let g=Geometry::new(&f).unwrap();
    let center=5*15+5;
    assert!(!neighbors(&f,center).any(|(to,_,_)|g.passable[to]));
    assert!(cells.iter().all(|&at|g.land_return[at]));
    assert!(g.ring_accessible_across_island(&f,&g.components[g.component[center]]));
    assert_eq!(serde_json::to_value(&f).unwrap(),before);
}

#[test]
fn stochastic_land_return_rejects_a_lucky_exit_with_an_unbounded_self_loop() {
    use crate::non_euclidean_generation::{NonEuclideanGroup,NonEuclideanPocket};
    let mut f=Floor{depth:20.,tiles:vec!["wall".into();81],..Default::default()};
    for x in 1..5 {f.tiles[4*9+x]="floor".into();}
    let cells=vec![4*9+2,4*9+3,4*9+4];
    f.non_euclidean=vec![NonEuclideanPocket{cells:cells.clone(),groups:vec![
        NonEuclideanGroup{sources:vec![cells[0],cells[2]],destination:cells[1]}]}].into();
    assert!(crate::non_euclidean_generation::validate_floor(&f));
    let g=Geometry::new(&f).unwrap();
    assert!(g.land_return[cells[0]]);
    assert_eq!(crate::portals::walking_exits(&f,cells[2],cells[1]),vec![cells[0],cells[2]]);
    // One outcome can leave, but the other returns to the same trapped state.
    assert!(!g.land_return[cells[2]]);
    assert!(!g.ring_accessible_across_island(&f,&g.components[0]));
}

#[test]
#[ignore="Full-world deterministic frequency survey; run explicitly for generation changes"]
fn swimming_safeguard_full_world_survey() {
    for seed in ["0-1-2:swimming-survey-a","0-1-2:swimming-survey-b"] {
        let mut r=crate::startup::new_run_shell_revision(seed,"Avatar",false,52.).unwrap();
        let rng=r.rng;crate::startup::ensure_floors(&mut r,40).unwrap();assert_eq!(r.rng,rng);
        let mut total=0;let mut maximum=0;let mut floors=0;let mut population_retries=0;let mut accepted_attempt_index_sum=0;
        let mut island_components=0;let mut max_components=0;
        let mut squares=0;let mut covered=0;let mut visits=0;let mut max_visits=0;let mut godbeasts=0;
        let seed_hash=crate::rng::seed_hash(&r.seed);let plan=crate::generation::domain_plan_revision(seed_hash,52.);
        for f in &r.floors {
            let entry=&plan[(f.depth as usize-1)/5];
            assert_eq!(f.domain,entry.domain);assert_eq!(f.godbeast,f.depth==entry.godbeast_depth as f64);
            if f.godbeast {godbeasts+=1;assert_eq!(f.godbeast_event.as_ref().unwrap().kind,entry.godbeast_kind);}
            let mut copy=(**f).clone();let coverage=crate::square_currents::generate(&mut copy,seed_hash);
            assert_eq!(copy.currents,f.currents,"final current reconstruction floor {}",f.depth);
            squares+=coverage.squares;covered+=coverage.covered_tiles;visits+=coverage.square_cell_visits;
            max_visits=max_visits.max(coverage.square_cell_visits);
            let geometry=Geometry::new(f).unwrap();let components=geometry.islands(f).count();
            island_components+=components;max_components=max_components.max(components);
            let rings=r.ground[&f.depth.to_string()].iter().filter(|g|crate::revision50_rings::effect(&g.item)==Some("swimming")).count();
            assert!(placement_cells(f,r.actors_at(f.depth),&r.ground[&f.depth.to_string()]).unwrap().is_empty());
            total+=rings;maximum=maximum.max(rings);floors+=usize::from(rings>0);
            let cleanup=f.generation.as_ref().unwrap().stages.iter().find(|s|s.id=="cleanup").unwrap().details.as_ref().unwrap();
            accepted_attempt_index_sum+=cleanup.get("generationAttempt").copied().unwrap_or(0.) as usize;
            population_retries+=cleanup.get("populationAttempt").copied().unwrap_or(0.) as usize;
        }
        assert_eq!(godbeasts,7);
        eprintln!("SWIMMING_SURVEY seed={seed} floors={} rings={total} floors_with_rings={floors} max_rings_per_floor={maximum} water_separated_components={island_components} max_components_per_floor={max_components} accepted_attempt_index_sum={accepted_attempt_index_sum} population_retry_sum={population_retries}",r.floors.len());
        eprintln!("CURRENT_SURVEY seed={seed} squares={squares} covered_tiles={covered} square_cell_visits={visits} max_floor_visits={max_visits} godbeasts={godbeasts}");
    }
}
