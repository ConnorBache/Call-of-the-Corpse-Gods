//! R52 Darkness is an effective-light override. R50/R51 and raw sources survive.
use super::*;
use crate::{Command, Item, FloorLightSourcesEntry, movement::Point, vision,
    lighting_cache::LightingProjection, temporal::{self, Engine, History, Snapshot}};
use std::sync::Arc;

fn room(revision:f64,ambient:f64)->Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=revision.into();r.player.x=10.;r.player.y=10.;
    r.player.stats.perception=300.;r.player.energy=100.;r.player.stats.max_energy=100.;
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.equipment_abilities=Field::Missing;r.player.ability_order=Field::Missing;r.player.ability_reserves=Field::Missing;
    r.inventory.clear();r.offers.clear();r.replay.clear();r.history.clear();r.future=None;
    r.phase=Field::Missing;r.haste_snapshot=Field::Missing;r.replay_snapshot_from=Field::Missing;
    for a in r.actors.values_mut(){*a=Arc::new(vec![]);}
    for i in r.ground.values_mut(){*i=Arc::new(vec![]);}
    let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(ambient);f.seen.fill(false);
    f.environment=Field::Missing;f.doors=Field::Missing;f.portals=Field::Missing;f.non_euclidean=Field::Missing;
    f.vents=Field::Missing;f.currents=Field::Missing;f.mud=Field::Missing;f.slime=Field::Missing;
    f.roots=Field::Missing;f.fungal_roots=Field::Missing;f.ichor=Field::Missing;
    f.light_sources=Field::Missing;f.permanent_fire=Field::Missing;f.snow=Field::Missing;f.ice=Field::Missing;f.hellfire=Field::Missing;
    r
}
fn bottle(r:&mut Run,diluted:bool)->f64 {
    let id=r.next_id;r.next_id+=1.;
    r.inventory.push(Item{id,kind:"potion".into(),name:"Potion of Darkness".into(),glyph:"!".into(),
        effect:"darkness".to_string().into(),diluted:diluted.into(),..Default::default()});id
}
fn dose(r:&mut Run,x:f64,y:f64,diluted:bool){bottle(r,diluted);let at=r.inventory.len()-1;assert!(crate::potions::apply(r,at,x,y,true).unwrap());}
fn light(r:&Run,x:i32,y:i32,expected:f64){
    assert_eq!(vision::tile_light(r,x as usize,y as usize),expected);
    let p=LightingProjection::new(r);assert_eq!(p.current(Point{x,y}),expected);
    assert_eq!(p.current_excluding(Point{x,y},r.player.id),expected);
}
fn torch(r:&mut Run,x:f64,y:f64,power:f64){r.floor_mut().unwrap().light_sources.get_or_insert_with(Vec::new).push(
    FloorLightSourcesEntry{x,y,power,falloff:10.,kind:"torch".to_string().into()});}

#[test]
fn darkness_immediate_fixed_level_keeps_source_values_legacy_dilution_and_phases(){
    for revision in [50.,51.,52.] {for ambient in [-150.,-75.,0.,100.,250.] {for diluted in [false,true] {for thrown in [false,true] {for phase in ["normal","haste"] {
        let mut r=room(revision,ambient);r.phase=phase.to_string().into();
        let id=bottle(&mut r,diluted);let raw=r.floor().unwrap().lights.clone();let tick=r.tick;let rng=r.rng;
        let command=if thrown {Command::Throw{id,x:10.,y:10.}}else{Command::Use{id}};
        let used=if thrown {crate::projectiles::command(&mut r,&command)}else{crate::inventory::command(&mut r,&command)};
        assert_eq!(used.unwrap(),Some(true));
        let expected=if revision>=52.{-100.}else{(ambient.clamp(-100.,100.)-if diluted{25.}else{50.}).clamp(-100.,100.)};
        light(&r,10,10,expected);light(&r,8,8,expected);light(&r,7,10,if revision>=52.{expected}else{ambient.clamp(-100.,100.)});light(&r,7,9,ambient.clamp(-100.,100.));
        let mask=r.floor().unwrap().revision50_darkness.as_ref().unwrap();assert_eq!(mask.len(),if revision>=52.{29}else{25});
        assert!(mask.iter().all(|d|d.expires_at==tick+if diluted{10.}else{20.}));
        assert_eq!(r.floor().unwrap().lights,raw);assert_eq!(r.tick,tick);assert_eq!(r.rng,rng);
    }}}}}
}

#[test]
fn darkness_overrides_existing_mobile_and_new_lights_then_restores_current_sources(){
    let mut r=room(52.,30.);torch(&mut r,10.,10.,250.);
    r.player.abilities.push("anavani-shine".into());
    r.inventory.push(Item{id:500.,kind:"ring".into(),effect:"light".to_string().into(),..Default::default()});
    let raw=r.floor().unwrap().lights.clone();let sources=r.floor().unwrap().light_sources.clone();
    dose(&mut r,10.,10.,false);let until=r.tick+20.;light(&r,10,10,-100.);light(&r,12,10,-100.);light(&r,13,10,-100.);light(&r,14,10,100.);
    assert_eq!(r.floor().unwrap().light_sources,sources);assert_eq!(r.floor().unwrap().lights,raw);
    for x in [6.,10.,14.] {r.player.x=x;light(&r,10,10,-100.);light(&r,11,10,-100.);}
    torch(&mut r,11.,10.,600.);crate::environment::ignite(&mut r,10.,10.,Some(8.),None).unwrap();
    light(&r,10,10,-100.);light(&r,11,10,-100.);
    r.player.active.push("anavani-shine".into());light(&r,10,10,-100.);
    let mut expected=r.clone();expected.floor_mut().unwrap().revision50_darkness=Field::Missing;
    r.tick=until; // Reads expire immediately, even before mutable cleanup.
    for x in 7..=13 {light(&r,x,10,vision::tile_light(&expected,x as usize,10));}
    expire(&mut r);assert!(r.floor().unwrap().revision50_darkness.is_missing());
    assert_eq!(r.floor().unwrap().lights,raw);assert_eq!(r.floor().unwrap().light_sources.as_ref().unwrap()[1].power,600.);
}

#[test]
fn darkness_aura_and_ring_bonus_cannot_bypass_override(){
    let mut r=room(52.,100.);r.player.x=6.;
    r.inventory.push(Item{id:500.,kind:"ring".into(),effect:"light".to_string().into(),..Default::default()});
    let a=Actor{id:20.,x:10.,y:10.,hp:100.,max_hp:100.,abilities:vec!["enemy:beast:darkness-aura".into()],..Default::default()};
    r.actors_at_mut(r.depth).push(a.clone());assert_eq!(vision::light(&r,&a),-90.);
    dose(&mut r,10.,10.,false);assert_eq!(vision::light(&r,&a),-100.);
    let p=LightingProjection::new(&r);assert_eq!(p.ring_bonus_in_plane(Point{x:10,y:10},false),0.);
    assert_eq!(vision::perception_actor_light(&r,&a,p.current(Point{x:10,y:10}),p.ring_bonus_in_plane(Point{x:10,y:10},false)),-100.);
    r.tick+=20.;assert_eq!(vision::light(&r,&a),-90.);
}

#[test]
fn darkness_overlap_refresh_and_expiry_do_not_stack_or_revert_sources(){
    let mut r=room(52.,75.);dose(&mut r,10.,10.,false);let tick=r.tick;
    r.tick+=5.;dose(&mut r,12.,10.,true);assert_eq!(r.floor().unwrap().revision50_darkness.as_ref().unwrap().len(),41);
    light(&r,10,10,-100.);light(&r,14,10,-100.);
    r.tick=tick+15.;expire(&mut r);light(&r,10,10,-100.);light(&r,13,10,-100.);light(&r,14,10,75.);
    r.tick=tick+19.;dose(&mut r,10.,10.,true);r.tick=tick+20.;expire(&mut r);light(&r,10,10,-100.);
    r.tick=tick+29.;expire(&mut r);light(&r,10,10,75.);
    dose(&mut r,0.,0.,false);assert_eq!(r.floor().unwrap().revision50_darkness.as_ref().unwrap().len(),11);
}

#[test]
fn darkness_same_tick_revision_change_invalidates_cache_and_preserves_old_rules(){
    let mut r=room(51.,100.);dose(&mut r,10.,10.,false);
    let old=r.lighting_cache.field(&r,false).unwrap();assert_eq!(old.at(Point{x:10,y:10}),50.);
    r.simulation_revision=52.0.into();let new=r.lighting_cache.field(&r,false).unwrap();
    assert!(!Arc::ptr_eq(&old,&new));light(&r,10,10,-100.);
    r.simulation_revision=51.0.into();light(&r,10,10,50.);
}

#[test]
fn darkness_actual_perception_uses_literal_formula_and_hides_unknown_mask(){
    let mut r=room(52.,100.);r.player.stats.perception=0.;dose(&mut r,12.,10.,false);
    let a=Actor{id:20.,x:12.,y:10.,hp:100.,max_hp:100.,..Default::default()};r.actors_at_mut(r.depth).push(a.clone());
    light(&r,12,10,-100.);assert_eq!(vision::actor_chance(&r,&r.player,&a,0.),0.);
    assert_eq!(vision::actor_chance(&r,&r.player,&a,10.),0.);assert_eq!(vision::actor_chance(&r,&r.player,&a,20.),100.);
    let o=vision::observe_terrain(&r).unwrap();let at=10*o.width+12;
    assert!(!o.cells[at].current);assert!(o.light_sources.iter().all(|s|!s.cells.contains(&at)));
    r.player.stats.perception=100.;let before=serde_json::to_vec(&r).unwrap();let o=vision::observe_terrain(&r).unwrap();
    assert!(o.cells[at].direct);assert_eq!(o.cells[at].illumination,Some(-100.));assert_eq!(o.cells[at].ambient,Some(100.));
    let source=o.light_sources.iter().find(|s|s.kind=="potion-darkness-override").unwrap();
    assert!(source.cells.contains(&at));assert!(!source.origin_known&&source.actor_id.is_none());assert_eq!((source.x,source.y),(-1.,-1.));
    assert_eq!(serde_json::to_vec(&r).unwrap(),before);
}

#[test]
fn darkness_save_snapshot_future_restore_and_offscreen_expiry(){
    let mut r=room(52.,80.);dose(&mut r,10.,10.,false);let tick=r.tick;
    let saved=crate::storage::serialize(&r).unwrap();let loaded=crate::storage::deserialize(&saved).unwrap();light(&loaded,10,10,-100.);
    let snapshot=Snapshot::capture(&r);crate::revision50_ability_energy::initialize_reserve(&mut r.player,"morae-see-the-future",0.,0.).unwrap();
    temporal::remember_future(&mut r,&History::default()).unwrap();r.tick+=30.;expire(&mut r);light(&r,10,10,80.);
    assert!(temporal::restore_future(&mut r,&mut History::default()).unwrap());assert_eq!(r.tick,tick);light(&r,10,10,-100.);
    r.tick+=30.;expire(&mut r);snapshot.restore(&mut r).unwrap();light(&r,10,10,-100.);
    let depth=r.depth;r.depth=2.;r.tick+=21.;crate::offscreen::catch_up_floor(&mut r,depth).unwrap();
    assert!(r.floors.iter().find(|f|f.depth==depth).unwrap().revision50_darkness.is_missing());
}

#[test]
fn darkness_command_replay_haste_and_roundtrip_are_exact(){
    use crate::replay::{ReplayRecord,ReplaySession};
    for revision in [51.,52.] {
        let mut initial=room(revision,100.);initial.player.conditions.insert("hasted".into(),6.);let id=bottle(&mut initial,false);
        let mut e=Engine::new(initial.clone()).unwrap();
        for command in [Command::Use{id},Command::Move{dx:1.,dy:0.},Command::Wait,Command::Wait] {
            e.dispatch(&command).unwrap();e=Engine::load(&e.save().unwrap()).unwrap();
            light(&e.run,10,10,if revision>=52.{-100.}else{50.});
        }
        let saved=e.save().unwrap();let mut replay=ReplaySession::from_initial(initial,ReplayRecord::try_from_run(&e.run).unwrap()).unwrap();
        while replay.step().unwrap(){}assert_eq!(replay.engine.save().unwrap(),saved);
        replay.restart().unwrap();while replay.step().unwrap(){}assert_eq!(replay.engine.save().unwrap(),saved);
    }
}

#[test]
fn darkness_pack_description_is_revision_aware(){
    for revision in [51.,52.] {for diluted in [false,true] {
        let mut r=room(revision,100.);bottle(&mut r,diluted);let text=crate::inventory::potion_description(&r,&r.inventory[0]);
        assert!(text.contains(if diluted{"10 turns"}else{"20 turns"}));
        assert_eq!(text.contains("Immediately sets illumination to -100"),revision>=52.);
        assert_eq!(text.contains("Reduces illumination by"),revision<52.);
        assert_eq!(text.contains("radius-3 circle"),revision>=52.);assert_eq!(text.contains("5×5"),revision<52.);
    }}
}


#[test]
fn darkness_circle_boundaries_diagonals_edges_and_legacy_exact_masks(){
    for revision in [50.,51.,52.] {for diluted in [false,true] {for (cx,cy) in [(10,10),(0,0),(0,10),(19,19)] {
        let mut r=room(revision,100.);let w=r.floor().unwrap().width();let h=r.floor().unwrap().tiles.len()/w;
        dose(&mut r,cx as f64,cy as f64,diluted);
        let mask=r.floor().unwrap().revision50_darkness.as_ref().unwrap();
        let expected:Vec<_>=(0..h).flat_map(|y|(0..w).filter_map(move |x|{
            let dx=x as i32-cx;let dy=y as i32-cy;
            (if revision>=52.{dx*dx+dy*dy<=9}else{dx.abs()<=2&&dy.abs()<=2}).then_some(y*w+x)
        })).collect();
        assert_eq!(mask.iter().map(|d|d.cell).collect::<Vec<_>>(),expected,"revision={revision}, center=({cx},{cy})");
        let cached=LightingProjection::new(&r);
        for y in 0..h {for x in 0..w {
            let value=if expected.contains(&(y*w+x)){if revision>=52.{-100.}else{if diluted{75.}else{50.}}}else{100.};
            assert_eq!(vision::tile_light(&r,x,y),value);
            assert_eq!(cached.current(Point{x:x as i32,y:y as i32}),value);
        }}
    }}}
}

#[test]
fn darkness_circle_preserves_wall_impact_and_hidden_cell_policy(){
    for revision in [51.,52.] {
        let mut r=room(revision,100.);r.player.x=6.;let w=r.floor().unwrap().width();
        for y in 0..r.floor().unwrap().tiles.len()/w {r.floor_mut().unwrap().tiles[y*w+9]="wall".into();}
        let raw_tiles=r.floor().unwrap().tiles.clone();let raw_light=r.floor().unwrap().lights.clone();
        let landing=crate::projectiles::trace(&r,14.,10.);assert_eq!((landing.x,landing.y),(8.,10.));
        let id=bottle(&mut r,false);assert_eq!(crate::projectiles::command(&mut r,&Command::Throw{id,x:14.,y:10.}).unwrap(),Some(true));
        let last=if revision>=52.{11}else{10};let expected=if revision>=52.{-100.}else{50.};
        light(&r,9,10,expected);light(&r,10,10,expected);light(&r,last,10,expected);light(&r,last+1,10,100.);
        assert_eq!(r.floor().unwrap().tiles,raw_tiles);assert_eq!(r.floor().unwrap().lights,raw_light);
        let o=vision::observe_terrain(&r).unwrap();
        assert!(!o.cells[10*w+10].current,"The mask crosses walls but direct sight does not");
        assert!(o.light_sources.iter().all(|s|!s.cells.contains(&(10*w+10))),"Hidden darkened cells must not leak into appearance");
    }
}

#[test]
fn darkness_circle_save_preserves_exact_cells_and_expiry_boundary(){
    for diluted in [false,true] {
        let mut r=room(52.,100.);dose(&mut r,10.,10.,diluted);let expiry=r.tick+if diluted{10.}else{20.};
        let cells=r.floor().unwrap().revision50_darkness.clone();
        r.tick=expiry-1.;let mut loaded=crate::storage::deserialize(&crate::storage::serialize(&r).unwrap()).unwrap();
        assert_eq!(loaded.floor().unwrap().revision50_darkness,cells);light(&loaded,13,10,-100.);light(&loaded,12,12,-100.);light(&loaded,13,11,100.);
        loaded.tick=expiry;light(&loaded,13,10,100.);light(&loaded,12,12,100.);expire(&mut loaded);
        assert!(loaded.floor().unwrap().revision50_darkness.is_missing());
    }
}
