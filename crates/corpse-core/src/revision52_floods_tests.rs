//! Reconstructed R52 Floods regressions from the released behavior contract.
//! These tests are new verification, not a recovered copy of the missing suite.
use super::*;
use crate::{Command, Field, Item, Stats, revision50_amulet_effects as effects,
    revision50_amulets as energy, revision50_control_amulets as control,
    revision52_amulet_tiers as tiers};

fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 52.0.into();
    r.player.x = 10.; r.player.y = 10.;
    r.player.hp = 1000.; r.player.max_hp = 1000.; r.player.base_max_hp = 1000.0.into();
    r.player.energy = 10000.;
    r.player.stats = Stats { perception: 300., max_energy: 10000., ..Default::default() };
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.equipment_abilities = Field::Missing; r.player.ability_order = Field::Missing;
    r.player.ability_reserves = Field::Missing; r.player.amulet_ledger = Field::Missing;
    r.player.conditions.insert("allSeeingReveal".into(), 1.);
    r.player.conditions.insert("allSeeingDepth".into(), r.depth);
    r.inventory.clear();
    // Preserve required empty per-floor blocks for real storage validation.
    for a in r.actors.values_mut() { *a = Default::default(); }
    for g in r.ground.values_mut() { *g = Default::default(); }
    r.history.clear(); r.haste_snapshot = Field::Missing; r.future = None;
    r.next_id = 1000.; r.phase = "normal".to_string().into();
    r.offers.clear(); r.replay.clear();
    let f = r.floor_mut().unwrap();
    f.tiles.fill("floor".into()); f.lights.fill(100.); f.seen.fill(true);
    f.remembered_tiles = vec!["floor".into(); f.tiles.len()].into();
    f.doors = Field::Missing; f.portals = Field::Missing; f.non_euclidean = Field::Missing;
    f.speed_up = Field::Missing; f.environment = Field::Missing; f.hellfire = Field::Missing;
    f.permanent_fire = Field::Missing; f.grates = Field::Missing; f.razorleaves = Field::Missing;
    f.godbeast_event = Field::Missing;
    f.revision50_terrain = Field::Missing; f.revision50_temporary_terrain = Field::Missing;
    let mut item = Item { id: 901., kind: "amulet".into(), effect: "floods".to_string().into(),
        glyph: "\"".into(), identified: true.into(), ..Default::default() };
    energy::initialize_item_for_revision(&mut item,52.).unwrap();
    tiers::initialize_birth(&mut item, 52., 1).unwrap();
    r.inventory.push(item);
    r
}
fn cell(r: &Run, x: usize, y: usize) -> usize { y * r.floor().unwrap().width() + x }
fn tile(r: &mut Run, x: usize, y: usize, value: &str) {
    let at = cell(r, x, y); r.floor_mut().unwrap().tiles[at] = value.into();
}
fn cast(r: &mut Run) {
    assert_eq!(effects::command(r, &Command::UseAt { id: 901., x: 15., y: 10. }).unwrap(), Some(true));
}
fn record(r: &Run, x: usize, y: usize) -> &control::TemporaryTerrain {
    let at = cell(r, x, y);
    r.floor().unwrap().revision50_temporary_terrain.as_ref().unwrap().iter().find(|t| t.cell == at).unwrap()
}
fn enemy(r: &mut Run, id: f64, x: f64, y: f64) {
    let mut a = Actor { id, x, y, name: "Giant Rat".into(), glyph: "r".into(), color: "#ffffff".into(),
        hp: 1000., max_hp: 1000., base_max_hp: 1000.0.into(), energy: 100., state: "hunting".into(),
        stats: Stats { max_energy: 100., dr: 900., ..Default::default() }, ..Default::default() };
    a.conditions.insert("paralyzed".into(), 100.);
    r.actors_at_mut(r.depth).push(a);
}
fn wave_receipt(r: &mut Run) -> Vec<(usize, usize)> {
    crate::events::begin(); cast(r);
    let events = crate::events::finish();
    events.into_iter().filter_map(|e| e.animation)
        .filter(|a| a.proposal == "A12" && a.stage == crate::events::Stage::Travel)
        .flat_map(|a| a.cells).map(|p| (p.x as usize, p.y as usize)).collect()
}

#[test]
fn mixed_lanes_include_consumed_shallow_and_exclude_deep_without_self_stopping() {
    let mut r = room();
    tile(&mut r, 13, 9, "shallow-water"); tile(&mut r, 12, 10, "water");
    let expected = vec![(11,9),(11,10),(11,11),(12,9),(12,11),(13,9),(13,11),(14,11),(15,11)];
    assert_eq!(preview(&r, false, 15., 10.), expected);
    let rng = r.rng;
    assert_eq!(wave_receipt(&mut r), expected);
    assert_eq!(r.rng, rng);
    assert_eq!(r.floor().unwrap().tiles[cell(&r,13,9)], "water");
    assert_eq!(r.floor().unwrap().tiles[cell(&r,14,9)], "floor");
    assert_eq!(r.floor().unwrap().tiles[cell(&r,12,10)], "water");
    assert_eq!(r.floor().unwrap().tiles[cell(&r,13,10)], "floor");
    assert_eq!(record(&r,13,9).previous, "shallow-water");
    assert_eq!(r.floor().unwrap().revision50_temporary_terrain.as_ref().unwrap().len(), 9);
}

#[test]
fn diagonal_lanes_consume_their_own_terminal_shallows() {
    let mut r = room();
    tile(&mut r,12,10,"shallow-water"); tile(&mut r,12,12,"shallow-water"); tile(&mut r,10,12,"water");
    let expected = vec![(12,10),(11,11),(12,12)];
    assert_eq!(preview(&r,false,15.,15.), expected);
    let rng = r.rng;
    effects::command(&mut r,&Command::UseAt{id:901.,x:15.,y:15.}).unwrap();
    for (x,y) in [(12,10),(12,12)] { assert_eq!(r.floor().unwrap().tiles[cell(&r,x,y)],"water"); }
    for (x,y) in [(13,11),(13,13),(11,13)] { assert_eq!(r.floor().unwrap().tiles[cell(&r,x,y)],"floor"); }
    assert_eq!(r.rng,rng);
}

#[test]
fn repeated_casts_pay_reserve_then_player_and_third_wave_is_paid_empty() {
    let mut r = room(); let rng = r.rng; let energy = r.player.energy;
    assert_eq!(wave_receipt(&mut r).len(),15);
    assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,0.);
    assert_eq!(r.player.energy,energy-20.);
    assert_eq!(wave_receipt(&mut r),vec![(11,9),(11,10),(11,11)]);
    assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,0.);
    assert_eq!(r.player.energy,energy-50.);
    let terrain = r.floor().unwrap().revision50_temporary_terrain.clone();
    assert!(preview(&r,false,15.,10.).is_empty());
    assert!(wave_receipt(&mut r).is_empty());
    assert_eq!(r.player.energy,energy-80.);
    assert_eq!(r.floor().unwrap().revision50_temporary_terrain,terrain);
    assert_eq!(r.rng,rng);
}

#[test]
fn promotion_refreshes_one_record_and_restores_first_true_ground() {
    for ground in ["floor","grass","snow","sand"] {
        let mut r=room(); let first_tick=r.tick;
        for y in 9..=11 { tile(&mut r,11,y,ground); }
        cast(&mut r); r.tick+=4.; cast(&mut r);
        let entries=r.floor().unwrap().revision50_temporary_terrain.as_ref().unwrap();
        assert_eq!(entries.len(),15);
        let unique:std::collections::HashSet<_>=entries.iter().map(|t|t.cell).collect();
        assert_eq!(unique.len(),entries.len());
        for y in 9..=11 {
            let t=record(&r,11,y); assert_eq!(t.previous,ground); assert_eq!(t.tile,"water");
            assert_eq!(t.expires_at,first_tick+16.);
        }
        r.tick=first_tick+12.;control::expire_terrain(&mut r);
        assert_eq!(r.floor().unwrap().revision50_temporary_terrain.as_ref().unwrap().len(),3);
        assert_eq!(r.floor().unwrap().tiles[cell(&r,12,10)],"floor");
        assert_eq!(r.floor().unwrap().tiles[cell(&r,11,10)],"water");
        r.tick=first_tick+16.;control::expire_terrain(&mut r);
        assert_eq!(r.floor().unwrap().tiles[cell(&r,11,10)],ground);
        assert!(r.floor().unwrap().revision50_temporary_terrain.is_missing());
    }
}

#[test]
fn natural_shallow_restores_and_later_independent_terrain_wins() {
    let mut r=room();let tick=r.tick;
    for y in 9..=11 {tile(&mut r,11,y,"shallow-water");}
    cast(&mut r);
    tile(&mut r,11,9,"grass");tile(&mut r,11,11,"wall");
    r.tick=tick+11.;control::expire_terrain(&mut r);
    assert_eq!(r.floor().unwrap().tiles[cell(&r,11,10)],"water");
    r.tick+=1.;control::expire_terrain(&mut r);
    assert_eq!(r.floor().unwrap().tiles[cell(&r,11,9)],"grass");
    assert_eq!(r.floor().unwrap().tiles[cell(&r,11,10)],"shallow-water");
    assert_eq!(r.floor().unwrap().tiles[cell(&r,11,11)],"wall");
    assert!(r.floor().unwrap().revision50_temporary_terrain.is_missing());
}

#[test]
fn deep_water_records_roundtrip_raw_save_and_snapshot_without_migration() {
    let mut r=room();cast(&mut r);r.tick+=3.;cast(&mut r);
    crate::validation::validate(&r).unwrap();
    let before=r.clone();
    let raw=crate::storage::serialize(&r).unwrap();let mut loaded=crate::storage::deserialize(&raw).unwrap();
    assert_eq!(loaded.floor().unwrap().revision50_temporary_terrain,r.floor().unwrap().revision50_temporary_terrain);
    let snapshot=crate::temporal::Snapshot::from_value(crate::temporal::Snapshot::capture(&loaded).value().unwrap()).unwrap();
    loaded.tick+=12.;control::expire_terrain(&mut loaded);
    assert!(loaded.floor().unwrap().revision50_temporary_terrain.is_missing());
    snapshot.restore(&mut loaded).unwrap();
    assert_eq!(loaded.floor().unwrap().tiles,before.floor().unwrap().tiles);
    assert_eq!(loaded.floor().unwrap().revision50_temporary_terrain,before.floor().unwrap().revision50_temporary_terrain);
    assert_eq!((loaded.tick,loaded.rng),(before.tick,before.rng));
    assert_eq!(loaded.inventory,before.inventory);
}

#[test]
fn offscreen_normal_catchup_expires_shallow_and_refreshed_deep_separately() {
    let mut r=room();let depth=r.depth;let tick=r.tick;
    cast(&mut r);r.tick+=3.;cast(&mut r);
    r.floor_mut().unwrap().last_tick=r.tick;r.depth=2.;r.tick=tick+12.;
    crate::offscreen::catch_up_floor(&mut r,depth).unwrap();
    let f=r.floors.iter().find(|f|f.depth==depth).unwrap();
    assert_eq!(f.tiles[10*f.width()+11],"water");assert_eq!(f.tiles[10*f.width()+12],"floor");
    assert_eq!(f.revision50_temporary_terrain.as_ref().unwrap().len(),3);
    r.tick=tick+15.;crate::offscreen::catch_up_floor(&mut r,depth).unwrap();
    let f=r.floors.iter().find(|f|f.depth==depth).unwrap();
    assert_eq!(f.tiles[10*f.width()+11],"floor");assert!(f.revision50_temporary_terrain.is_missing());
}

#[test]
fn all_nine_packets_hit_consumed_cell_once_and_push_before_conversion() {
    for tier in 1..=9 {
        let mut r=room();r.inventory[0].amulet_tier=(tier as f64).into();
        tile(&mut r,12,10,"shallow-water");enemy(&mut r,100.,12.,10.);enemy(&mut r,101.,15.,10.);
        let rng=r.rng;cast(&mut r);
        let a=engine::actor(&r,100.).unwrap();assert_eq!(a.hp,1000.-tiers::DAMAGE[tier-1][4]);
        assert_eq!((a.x,a.y),(14.,10.));assert_eq!(engine::actor(&r,101.).unwrap().hp,1000.);
        assert_eq!(r.floor().unwrap().tiles[cell(&r,12,10)],"water");assert_eq!(r.rng,rng);
        assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,0.);
    }
}

#[test]
fn normal_haste_and_free_cast_profiles_use_normal_tick_expiry() {
    for phase in ["normal","haste"] {
        let mut r=room();r.phase=phase.to_string().into();
        tile(&mut r,11,10,"shallow-water");let start=r.tick;
        assert!(engine::dispatch(&mut r,&Command::UseAt{id:901.,x:15.,y:10.}).unwrap().took_turn);
        assert_eq!(record(&r,11,10).expires_at,start+12.);
        while r.tick<start+11. {engine::dispatch(&mut r,&Command::Wait).unwrap();}
        assert_eq!(r.floor().unwrap().tiles[cell(&r,11,10)],"water");
        while r.tick<start+12. {engine::dispatch(&mut r,&Command::Wait).unwrap();}
        assert_eq!(r.floor().unwrap().tiles[cell(&r,11,10)],"shallow-water");
    }
    let mut r=room();r.player.abilities.push("isheilah-spellslinger".into());
    tile(&mut r,11,10,"shallow-water");let (tick,phase)=(r.tick,r.phase.clone());
    let out=engine::dispatch(&mut r,&Command::UseAt{id:901.,x:15.,y:10.}).unwrap();
    assert!(!out.took_turn);assert_eq!((r.tick,r.phase.clone()),(tick,phase));
    assert_eq!(record(&r,11,10).expires_at,tick+12.);
}

#[test]
fn hidden_terrain_and_actors_do_not_change_preview_or_animation_bounds() {
    let mut r=room();r.player.conditions.clear();r.player.stats.perception=0.;
    // An opaque barrier makes every cell beyond it unknown without touching memory.
    for y in 0..r.floor().unwrap().height() {tile(&mut r,12,y,"wall");}
    let expected=preview(&r,false,15.,10.);let memory=r.floor().unwrap().remembered_tiles.clone();
    assert!(!crate::vision::visible(&r,13,10));
    for hidden in ["floor","shallow-water","water"] {
        let mut test=r.clone();tile(&mut test,13,10,hidden);enemy(&mut test,100.,13.,10.);
        assert_eq!(preview(&test,false,15.,10.),expected);
        assert!(effects::admission(&test,&Command::UseAt{id:901.,x:15.,y:10.}).unwrap().is_ok());
        let rng=test.rng;let receipt=wave_receipt(&mut test);
        assert!(receipt.len()<=15);assert!(receipt.iter().all(|&(x,y)|crate::vision::visible(&r,x,y)));
        assert!(!receipt.contains(&(13,10)));assert_eq!(test.rng,rng);
        assert_eq!(test.floor().unwrap().remembered_tiles,memory);
    }
    // Darkness hides reachable cells without a physical wall. Live hidden water
    // changes commit geometry, but never preview/admission or observed travel cells.
    let mut dark=room();dark.player.conditions.clear();dark.player.stats.perception=0.;
    dark.floor_mut().unwrap().lights.fill(-100.);
    dark.floor_mut().unwrap().light_sources=Field::Missing;
    assert!(!crate::vision::visible(&dark,13,10));
    let expected=preview(&dark,false,15.,10.);
    for hidden in ["floor","shallow-water","water"] {
        let mut test=dark.clone();tile(&mut test,13,10,hidden);
        let before=wave_cells(&test,15.,10.,false,WaterStop::Lanes);
        assert_eq!(before.len(),if hidden=="floor"{15}else if hidden=="shallow-water"{13}else{12});
        assert_eq!(preview(&test,false,15.,10.),expected);
        assert!(effects::admission(&test,&Command::UseAt{id:901.,x:15.,y:10.}).unwrap().is_ok());
        let receipt=wave_receipt(&mut test);
        assert!(receipt.iter().all(|&(x,y)|crate::vision::visible(&dark,x,y)));
        assert!(!receipt.contains(&(13,10)));
    }
    // An invisible occupant in a reached shallow tile is hit without leaking its identity into aiming.
    let mut r=room();r.player.conditions.clear();tile(&mut r,11,10,"shallow-water");
    let p=preview(&r,false,15.,10.);enemy(&mut r,100.,11.,10.);
    engine::actor_mut_trusted(&mut r,100.).unwrap().conditions.insert("invisible".into(),100.);
    assert_eq!(preview(&r,false,15.,10.),p);cast(&mut r);assert_eq!(engine::actor(&r,100.).unwrap().hp,980.);
}

#[test]
fn protected_ground_doors_portals_grates_and_unrelated_edits_are_not_promoted() {
    for kind in ["closed-door","open-door","grate","portal","permanent","borge","epidite","unrelated"] {
        let mut r=room();tile(&mut r,11,10,"shallow-water");let at=cell(&r,11,10);
        match kind {
            "closed-door"|"open-door"=>{r.floor_mut().unwrap().doors.get_or_insert_with(Default::default)
                .insert(at.to_string(),crate::FloorDoorsEntry{open:kind=="open-door",..Default::default()});},
            "grate"=>r.floor_mut().unwrap().grates=vec![at as f64].into(),
            "portal"=>r.floor_mut().unwrap().portals=vec![crate::portals_generation::PortalPair{a:at,b:at+20,color_index:0}].into(),
            "permanent"=>{
                r.floor_mut().unwrap().revision50_terrain=vec![crate::revision50_terrain::Modification{
                    kind:crate::revision50_terrain::Kind::Wall,cells:vec![crate::revision50_terrain::Cell{x:11,y:10}],hp:Some(200.),previous_tiles:vec!["shallow-water".into()]}].into();
            },
            "borge"|"epidite"=>r.floor_mut().unwrap().godbeast_event=crate::GodbeastEvent{
                kind:if kind=="borge"{"Borge"}else{"Epidite"}.into(),x:11.,y:10.,..Default::default()}.into(),
            _=>r.floor_mut().unwrap().revision50_temporary_terrain=vec![control::TemporaryTerrain{
                cell:at,previous:"floor".into(),tile:"wall".into(),expires_at:100.}].into(),
        }
        assert!(!control::can_deepen_floods_terrain(&r,at),"{kind}");
        let prior=r.floor().unwrap().revision50_temporary_terrain.clone();
        // Geometry stays the released R50 physical wave; protection gates restrict terrain mutation.
        flood(&mut r,15.,10.,WaterStop::Lanes).unwrap();
        assert_eq!(r.floor().unwrap().tiles[at],"shallow-water","{kind}");
        let records=r.floor().unwrap().revision50_temporary_terrain.as_ref().unwrap();
        assert_eq!(records.iter().filter(|t|t.cell==at).count(),if prior.is_missing(){0}else{1},"{kind}");
        if kind=="closed-door" {assert!(!preview(&r,false,15.,10.).contains(&(11,10)));}
    }
}

#[test]
fn duel_protection_preserves_statuses_and_position_but_not_shallow_ground() {
    // Keep the chosen target inside the square throughout this shielding case.
    let mut r=room();tile(&mut r,12,10,"shallow-water");enemy(&mut r,100.,12.,10.);enemy(&mut r,101.,11.,9.);
    let depth=r.depth;let a=engine::actor_mut(&mut r,100.).unwrap();a.active.push(crate::revision50::DUEL.into());
    for (k,v) in [("r50DuelTarget",101.),("r50DuelX",12.),("r50DuelY",10.),("r50DuelDepth",depth),
        ("poisoned",3.),(control::ROOTS,4.)] {a.conditions.insert(k.into(),v);}
    let conditions=a.conditions.clone();flood(&mut r,15.,10.,WaterStop::Lanes).unwrap();
    let a=engine::actor(&r,100.).unwrap();assert_eq!((a.hp,a.x,a.y),(1000.,12.,10.));assert_eq!(a.conditions,conditions);
    assert_eq!(r.floor().unwrap().tiles[cell(&r,12,10)],"water");
    assert_eq!(engine::actor(&r,101.).unwrap().hp,980.);
    let mut r=room();tile(&mut r,12,10,"shallow-water");enemy(&mut r,100.,12.,10.);
    engine::actor_mut_trusted(&mut r,100.).unwrap().conditions.insert("phantasmiiPlane".into(),1.);
    cast(&mut r);let a=engine::actor(&r,100.).unwrap();assert_eq!((a.hp,a.x),(1000.,12.));
    assert_eq!(r.floor().unwrap().tiles[cell(&r,12,10)],"water");
}

#[test]
fn invalid_water_records_reject_before_dispatch_or_snapshot_mutation() {
    for bad in ["legacy","duplicate","predecessor","tile","expiry","cell"] {
        let mut r=room();tile(&mut r,11,10,"shallow-water");cast(&mut r);
        match bad {
            "legacy"=>{r.simulation_revision=51.0.into();r.inventory[0].amulet_tier=Field::Missing;},
            "duplicate"=>{let entry=record(&r,11,10).clone();r.floor_mut().unwrap().revision50_temporary_terrain.as_mut().unwrap().push(entry);},
            _=>{let at=cell(&r,11,10);let t=r.floor_mut().unwrap().revision50_temporary_terrain.as_mut().unwrap().iter_mut().find(|t|t.cell==at).unwrap();
                match bad {"predecessor"=>t.previous="water".into(),"tile"=>t.tile="lava".into(),"expiry"=>t.expires_at=-1.,_=>t.cell=usize::MAX,}
            },
        }
        assert!(control::validate_terrain(&r).is_err(),"{bad}");
        let before=r.clone();assert!(engine::dispatch(&mut r,&Command::Wait).is_err(),"{bad}");assert_eq!(r,before);
        let snapshot=crate::temporal::Snapshot::capture(&r);let mut valid=room();let original=valid.clone();
        assert!(snapshot.restore(&mut valid).is_err(),"{bad}");assert_eq!(valid,original);
        assert!(crate::storage::deserialize(&crate::storage::serialize(&r).unwrap()).is_err(),"{bad}");
    }
}

#[test]
fn legacy_shallow_stop_and_r52_only_description_preserve_all_other_profiles() {
    for revision in [50.,51.] {
        let mut r=room();r.simulation_revision=revision.into();r.inventory[0].amulet_tier=Field::Missing;
        tile(&mut r,12,10,"shallow-water");let before=preview(&r,false,15.,10.);
        assert!(!before.contains(&(12,10)));assert!(!before.contains(&(13,10)));assert_eq!(before.len(),11);
        let rng=r.rng;cast(&mut r);assert_eq!(r.rng,rng);
        assert_eq!(r.floor().unwrap().tiles[cell(&r,12,10)],"shallow-water");
        assert_eq!(r.floor().unwrap().tiles[cell(&r,13,10)],"floor");
        assert!(!tiers::description(&r,&r.inventory[0]).unwrap().contains("deep water"));
        assert!(r.floor().unwrap().revision50_temporary_terrain.as_ref().unwrap().iter().all(|t|t.tile=="shallow-water"));
    }
    let r=room();assert!(tiers::description(&r,&r.inventory[0]).unwrap().contains("deep water"));
    assert_eq!(tiers::DAMAGE.iter().map(|d|d[4]).collect::<Vec<_>>(),vec![20.,35.,42.,43.,61.,62.,63.,77.,84.]);
    assert_eq!(effects::item_spec(&r.inventory[0]).unwrap().activation_cost,30.);
}

#[test]
fn selected_target_wave_exit_ends_duel_before_a_later_wave_hit() {
    let mut r=room();tile(&mut r,12,10,"shallow-water");enemy(&mut r,100.,12.,10.);enemy(&mut r,101.,14.,9.);
    let depth=r.depth;let a=engine::actor_mut(&mut r,100.).unwrap();a.active.push(crate::revision50::DUEL.into());
    for (k,v) in [("r50DuelTarget",101.),("r50DuelX",12.),("r50DuelY",10.),("r50DuelDepth",depth)] {a.conditions.insert(k.into(),v);}
    flood(&mut r,15.,10.,WaterStop::Lanes).unwrap();
    assert_eq!((engine::actor(&r,101.).unwrap().x,engine::actor(&r,101.).unwrap().y),(16.,9.));
    let owner=engine::actor(&r,100.).unwrap();assert_eq!((owner.hp,owner.x,owner.y),(980.,14.,10.));
    assert!(!owner.active.iter().any(|s|s==crate::revision50::DUEL));
    for key in ["r50DuelTarget","r50DuelX","r50DuelY","r50DuelDepth"] {assert!(!owner.conditions.contains_key(key));}
}

#[test]
fn released_floods_rejects_nontargeted_use_without_payment_or_terrain_change() {
    for revision in [50.,51.,52.] {
        let mut r=room();r.simulation_revision=revision.into();
        if revision<52. {r.inventory[0].amulet_tier=Field::Missing;}
        let before=r.clone();
        let error=effects::command(&mut r,&Command::Use{id:901.}).unwrap_err();
        assert!(error.to_string().contains("Choose a target"));assert_eq!(r,before);
        let result=engine::dispatch(&mut r,&Command::Use{id:901.}).unwrap();
        assert!(!result.took_turn);
        assert_eq!((r.tick,r.player.energy,r.rng),(before.tick,before.player.energy,before.rng));
        assert_eq!(r.inventory,before.inventory);
        assert_eq!(r.floor().unwrap().tiles,before.floor().unwrap().tiles);
        assert_eq!(r.floor().unwrap().currents,before.floor().unwrap().currents);
        assert!(r.floor().unwrap().revision50_temporary_terrain.is_missing());
    }
}

#[test]
fn released_floods_rejects_unpublished_current_metadata_in_saves_and_snapshots() {
    let mut r=room();cast(&mut r);
    let mut raw=crate::storage::canonical_value(&r).unwrap();
    let floor=raw["floors"].as_array_mut().unwrap().iter_mut()
        .find(|f|f["depth"].as_f64()==Some(r.depth)).unwrap();
    floor["revision50TemporaryTerrain"][0]["floodCurrent"]=serde_json::json!({
        "previous":0.,"applied":3.,"layerWasMissing":true});
    let encoded=serde_json::to_string(&raw).unwrap();
    for error in [crate::storage::deserialize(&encoded).unwrap_err(),
        crate::storage::deserialize_compatible(&encoded).unwrap_err(),
        crate::storage::from_canonical(&encoded).unwrap_err(),
        crate::temporal::Snapshot::from_value(raw).unwrap_err()] {
        assert!(error.to_string().contains("floodCurrent"),"{error}");
    }
}

#[test]
fn released_floods_recorded_use_at_replays_with_save_load_recasts_and_expiry() {
    use crate::replay::{ReplayRecord,ReplaySession};
    let initial=room();let mut e=crate::temporal::Engine::new(initial.clone()).unwrap();
    for command in [Command::UseAt{id:901.,x:15.,y:10.},Command::Wait,
        Command::UseAt{id:901.,x:15.,y:10.}] {
        e.dispatch(&command).unwrap();e=crate::temporal::Engine::load(&e.save().unwrap()).unwrap();
    }
    assert_eq!(e.run.floor().unwrap().tiles[cell(&e.run,11,10)],"water");
    for _ in 0..12 {e.dispatch(&Command::Wait).unwrap();}
    assert!(e.run.floor().unwrap().revision50_temporary_terrain.is_missing());
    let saved=e.save().unwrap();
    let mut replay=ReplaySession::from_initial(initial,ReplayRecord::try_from_run(&e.run).unwrap()).unwrap();
    while replay.step().unwrap(){}assert_eq!(replay.engine.save().unwrap(),saved);
}
