//! R52 root cadence is normal-turn state, independent of presentation and RNG.
use corpse_core::{Command, Field, Floor, Run, engine, environment, fungi,
    hazard_memory, offscreen, population, storage, temporal::{self, Engine, History, Snapshot}};
use std::sync::Arc;

fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=revision.into();r.tick=0.;r.depth=1.;r.next_id=1100.;
    r.player.x=27.;r.player.y=14.;r.player.abilities.clear();r.player.active.clear();
    r.player.ability_order=Field::Missing;r.player.ability_reserves=Field::Missing;r.player.conditions.clear();
    r.player.stats.perception=300.;r.inventory.clear();r.offers.clear();r.history.clear();r.future=None;
    r.haste_snapshot=Field::Missing;r.phase=Field::Missing;r.replay.clear();
    r.actors.clear();r.ground.clear();
    let (w,h)=(79,29);let mut tiles=vec!["floor".into();w*h];
    for i in 0..w*h {if i%w==0||i%w==w-1||i/w==0||i/w==h-1 {tiles[i]="wall".into();}}
    tiles[80]="up".into();tiles[w*h-w-2]="down".into();
    let mut f=Floor {depth:1.,domain:"Dungeon".into(),tiles,lights:vec![100.;w*h],seen:vec![true;w*h],
        width:(w as f64).into(),height:(h as f64).into(),dirty:true,..Default::default()};
    environment::initialize(&mut f).unwrap();r.floors=vec![Arc::new(f)];
    r.actors_at_mut(1.).push(population::create_actor_revision("bastidion",1000.,30.,14.,false,revision).unwrap());
    for (id,x,y) in [(1001.,34.,14.),(1002.,34.,16.)] {
        let mut a=population::create_actor_revision("rat",id,x,y,false,revision).unwrap();
        a.dead=true.into();a.hp=0.;a.heartless=true.into();a.corpse_age=0.0.into();r.actors_at_mut(1.).push(a);
    }
    r.ground_at_mut(1.).clear();r
}
fn tick(r:&mut Run) {r.tick+=1.;let frozen=vec![false;r.floor().unwrap().tiles.len()];fungi::tick_fungi(r,&frozen).unwrap();}
fn links(r:&Run)->&[corpse_core::FungalRoot] {r.floor().unwrap().fungal_roots.as_ref().map(Vec::as_slice).unwrap_or(&[])}
fn rooted(r:&Run,id:f64)->bool {environment::corpse_rooted(r.floor().unwrap(),r.actors_at(1.),engine::actor(r,id).unwrap())}
fn assert_prefixes(r:&Run) {
    let mut expected=vec![0.;r.floor().unwrap().tiles.len()];
    for link in links(r).iter().filter(|l|l.active) {for &at in link.grown_path() {expected[at as usize]=1.;}}
    assert_eq!(r.floor().unwrap().roots.as_ref().unwrap(),&expected);
}

#[test]
fn acquisition_waits_three_later_turns_per_tile_and_only_contact_feeds_or_roots() {
    let mut r=room(52.);let rng=r.rng;
    for turn in 1..=13 {
        tick(&mut r);let grown=1+(turn-1)/3;
        assert_eq!(links(&r).len(),2);
        for link in links(&r) {assert_eq!(link.path.len(),5);assert_eq!(link.grown_path().len(),grown);
            assert_eq!(link.growth.as_ref().unwrap().turns,if turn==13 {0}else{((turn-1)%3) as u8});}
        assert_prefixes(&r);assert_eq!(rooted(&r,1001.),turn==13);assert_eq!(rooted(&r,1002.),turn==13);
        // No phantom target or ungrown route appears in observation.
        let target=14*79+34;
        assert_eq!(hazard_memory::observed_hazards_at_revision(r.floor().unwrap(),target,r.tick,52.).root.is_missing(),turn<13);
        let emitted=r.floor().unwrap().environment.as_ref().unwrap().gas_volume.as_ref().unwrap()[14*79+30];
        assert_eq!(emitted,25.*turn as f64+if turn==13 {50.}else{0.});
    }
    assert_eq!(r.rng,rng);let full=links(&r).to_vec();tick(&mut r);assert_eq!(links(&r),full);
}

#[test]
fn normal_commands_haste_replay_and_save_resume_keep_identical_growth_boundaries() {
    let mut r=room(52.);r.player.conditions.insert("hasted".into(),100.);
    let mut live=Engine::new(r.clone()).unwrap();let mut replay=Engine::new(r).unwrap();
    let mut saw_extra=false;
    for command in 0..18 {
        let before_tick=live.run.tick;let before=links(&live.run).to_vec();
        let action=Command::Move {dx:0.,dy:if command%2==0 {1.}else{-1.}};
        live.dispatch(&action).unwrap();replay.dispatch(&action).unwrap();
        if live.run.tick==before_tick {saw_extra=true;assert_eq!(links(&live.run),before,"haste action {command}");}
        else {
            let turn=live.run.tick as usize;
            for link in links(&live.run) {assert_eq!(link.grown_path().len(),(1+(turn-1)/3).min(link.path.len()));}
        }
        assert_eq!(live.save().unwrap(),replay.save().unwrap(),"replayed command {command}");
        if command%3==1 {live=Engine::load(&live.save().unwrap()).unwrap();}
    }
    assert!(saw_extra);assert_eq!(live.run.rng,replay.run.rng);
}

#[test]
fn offscreen_uses_the_same_normal_turn_cadence_and_preserves_partial_progress() {
    let mut active=room(52.);tick(&mut active);tick(&mut active);
    let mut away=active.clone();
    active.floor_mut().unwrap().last_tick=2.;away.floor_mut().unwrap().last_tick=2.;
    let mut other=away.floor().unwrap().clone();other.depth=2.;other.roots=Field::Missing;other.fungal_roots=Field::Missing;
    away.floors.push(Arc::new(other));away.actors_at_mut(2.).clear();away.ground_at_mut(2.).clear();away.depth=2.;
    for turn in 3..=10 {tick(&mut active);away.tick=turn as f64;offscreen::catch_up_floor(&mut away,1.).unwrap();
        let floor=away.floors.iter().find(|f|f.depth==1.).unwrap();
        assert_eq!(floor.fungal_roots,active.floor().unwrap().fungal_roots,"offscreen turn {turn}");
        assert_eq!(floor.roots,active.floor().unwrap().roots);
    }
}

#[test]
fn snapshot_and_future_roundtrips_restore_partial_prefix_and_fractional_timer() {
    let mut r=room(52.);for _ in 0..6 {tick(&mut r);}let expected=links(&r).to_vec();
    assert_eq!(expected[0].growth.as_ref().unwrap().turns,2);
    let snap=Snapshot::from_value(Snapshot::capture(&r).value().unwrap()).unwrap();
    let rng=r.rng;tick(&mut r);assert_ne!(links(&r),expected);snap.restore(&mut r).unwrap();
    assert_eq!(links(&r),expected);assert_eq!(r.rng,rng);
    let id="morae-see-the-future";r.player.abilities.push(id.into());
    corpse_core::revision50_ability_energy::initialize_reserve(&mut r.player,id,50.,50.).unwrap();
    temporal::remember_future(&mut r,&History::default()).unwrap();
    tick(&mut r);let raw=storage::serialize(&r).unwrap();let mut r=storage::deserialize_compatible(&raw).unwrap();
    assert!(temporal::restore_future(&mut r,&mut History::default()).unwrap());assert_eq!(links(&r),expected);
    tick(&mut r);assert_eq!(links(&r)[0].grown_path().len(),3);
}

#[test]
fn old_completed_roots_are_not_erased_or_retroactively_retimed() {
    for revision in [50.,51.] {
        let mut r=room(revision);tick(&mut r);assert!(links(&r).iter().all(|l|l.growth.is_missing()&&l.reaches_corpse()));
        let old=links(&r).to_vec();r.simulation_revision=52.0.into();
        let mut loaded=storage::deserialize_compatible(&storage::serialize(&r).unwrap()).unwrap();
        assert!(links(&loaded).iter().all(|l|l.reaches_corpse()&&l.growth.is_missing()));assert_prefixes(&loaded);
        tick(&mut loaded);assert!(links(&loaded).iter().all(|l|l.path==old.iter().find(|v|v.corpse_id==l.corpse_id).unwrap().path));
        assert!(rooted(&loaded,1001.));
    }
}

#[test]
fn cuts_burn_only_real_material_keep_shared_prefixes_and_never_reacquire() {
    let mut r=room(52.);for _ in 0..4 {tick(&mut r);}assert_eq!(links(&r)[0].grown_path().len(),2);
    let unbuilt=links(&r)[0].path[3] as usize;
    environment::burn_roots_revision(r.floor_mut().unwrap(),unbuilt,52.);
    assert!(links(&r).iter().all(|l|l.active));
    let cut=links(&r)[1].path[1] as usize;
    environment::burn_roots_revision(r.floor_mut().unwrap(),cut,52.);
    assert!(!links(&r)[1].active);assert!(links(&r)[0].active);assert_prefixes(&r);
    for _ in 0..12 {tick(&mut r);}assert_eq!(links(&r).len(),2);assert!(!links(&r)[1].active);assert_prefixes(&r);
    engine::actor_mut(&mut r,1000.).unwrap().hp=0.;engine::death_if_nonpositive(&mut r,1000.).unwrap();
    assert!(links(&r).iter().all(|l|!l.active));assert_prefixes(&r);
}

#[test]
fn freezing_and_fire_ahead_pause_without_detours_or_phantom_severing() {
    let mut r=room(52.);tick(&mut r);tick(&mut r);let before=links(&r).to_vec();
    let next=links(&r)[0].path[1] as usize;let mut frozen=vec![false;r.floor().unwrap().tiles.len()];frozen[next]=true;
    fungi::tick_fungi(&mut r,&frozen).unwrap();assert_eq!(links(&r)[0],before[0]);
    frozen[next]=false;r.floor_mut().unwrap().environment.as_mut().unwrap().fire[next]=1.;
    fungi::tick_fungi(&mut r,&frozen).unwrap();assert_eq!(links(&r)[0],before[0]);
    r.floor_mut().unwrap().environment.as_mut().unwrap().fire[next]=0.;
    tick(&mut r);assert_eq!(links(&r)[0].grown_path().len(),1);tick(&mut r);assert_eq!(links(&r)[0].grown_path().len(),2);
    assert_eq!(links(&r)[0].path,before[0].path);
}

#[test]
fn fully_rooted_corpses_cannot_revive_and_unreached_corpses_can() {
    for complete in [false,true] {
        let mut r=room(52.);for _ in 0..if complete {13}else{1} {tick(&mut r);}
        let corpse=engine::actor_mut(&mut r,1001.).unwrap();corpse.corpse_age=1000.0.into();
        assert_eq!(rooted(&r,1001.),complete);
        let mut e=Engine::new(r).unwrap();e.dispatch(&Command::Wait).unwrap();
        assert_eq!(engine::actor(&e.run,1001.).unwrap().dead.value_or(false),complete);
    }
}

#[test]
fn malformed_growth_is_rejected_and_load_never_fills_missing_material() {
    let mut r=room(52.);for _ in 0..4 {tick(&mut r);}let raw=storage::serialize(&r).unwrap();
    for bad in [serde_json::json!({"grown":0,"turns":0}),serde_json::json!({"grown":6,"turns":0}),
        serde_json::json!({"grown":2,"turns":3}),serde_json::json!({"grown":1.5,"turns":0})] {
        let mut value:serde_json::Value=serde_json::from_str(&raw).unwrap();value["floors"][0]["fungalRoots"][0]["growth"]=bad;
        assert!(storage::deserialize_compatible(&serde_json::to_string(&value).unwrap()).is_err());
    }
    let missing=links(&r)[0].grown_path()[1] as usize;r.floor_mut().unwrap().roots.as_mut().unwrap()[missing]=0.;
    let loaded=storage::deserialize_compatible(&storage::serialize(&r).unwrap()).unwrap();
    assert!(!links(&loaded)[0].active);assert_prefixes(&loaded);
}

#[test]
fn visible_root_growth_cues_remain_roots_under_hallucination_deterministically() {
    let mut r=room(52.);for _ in 0..3 {tick(&mut r);}r.player.conditions.insert("hallucinating".into(),10.);
    let mut one=Engine::new(r.clone()).unwrap();let mut two=Engine::new(r).unwrap();
    let cues=one.dispatch(&Command::Wait).unwrap().events;
    assert_eq!(cues,two.dispatch(&Command::Wait).unwrap().events);assert_eq!(one.run,two.run);
    let growth=cues.iter().filter_map(|e|e.animation.as_ref()).find(|a|a.proposal=="E23"&&a.style=="root").unwrap();
    assert_eq!(growth.cells.len(),2);assert!(growth.cells.iter().all(|p|p.x==31.));
    // No generic trait puff reveals the hallucinated source identity.
    assert!(!cues.iter().filter_map(|e|e.animation.as_ref()).any(|a|a.style=="puff-violet"));
}

#[test]
fn native_root_growth_review_exports_actual_incremental_states() {
    let Ok(path)=std::env::var("CORPSE_ROOT_GROWTH_REVIEW") else{return;};std::fs::create_dir_all(&path).unwrap();
    let mut r=room(52.);for turn in 1..=13 {tick(&mut r);if [1,3,4,7,13].contains(&turn) {
        r.log=vec![format!("Native R52 root growth: normal turn {turn}, {} grown cells per branch.",links(&r)[0].grown_path().len())];
        r.log_kinds=vec!["environment".into()].into();
        std::fs::write(format!("{path}/turn-{turn:02}.json"),storage::serialize(&r).unwrap()).unwrap();
    }}
    r.player.conditions.insert("hallucinating".into(),100.);
    std::fs::write(format!("{path}/hallucinating.json"),storage::serialize(&r).unwrap()).unwrap();
}

#[test]
fn newly_generated_feasts_begin_at_the_source_and_extend_on_turn_three() {
    let mut r=room(52.);r.depth=10.;r.actors.clear();r.ground.clear();
    let f=r.floors.first_mut().unwrap();let f=Arc::make_mut(f);f.depth=10.;
    f.setup_spawns=vec![corpse_core::FloorSetupSpawnsEntry {kind:"fungus".into(),x:30.,y:14.,..Default::default()}].into();
    let mut next=1000.;let actors=population::populate_machine_enemies_revision("slow-feasts",&mut next,f,&[],&[],52.).unwrap();
    let mut aged=false;
    for link in f.fungal_roots.as_ref().unwrap() {assert_eq!(link.grown_path().len(),1);assert!(!link.reaches_corpse());
        let corpse=actors.iter().find(|a|a.id==link.corpse_id).unwrap();assert!(!environment::corpse_rooted(f,&actors,corpse));}
    r.next_id=next;r.actors.insert("10".into(),Arc::new(actors));r.ground_at_mut(10.).clear();
    for turn in 1..=3 {tick(&mut r);for link in links(&r) {assert_eq!(link.grown_path().len(),if turn<3 {1}else{2});
        if turn==3 {aged=true;}}assert_prefixes(&r);}
    assert!(aged);
}

#[test]
fn a_moved_target_severs_its_partial_branch_without_changing_the_saved_direct_route() {
    let mut r=room(52.);for _ in 0..4 {tick(&mut r);}let original=links(&r)[0].path.clone();
    engine::actor_mut(&mut r,1001.).unwrap().y+=1.;tick(&mut r);
    assert!(!links(&r)[0].active);assert_eq!(links(&r)[0].path,original);assert!(links(&r)[1].active);
    assert_prefixes(&r);for _ in 0..9 {tick(&mut r);}assert_eq!(links(&r).len(),2);assert!(!links(&r)[0].active);
}

#[test]
fn ungrown_routes_do_not_add_false_junctions_to_another_branches_visible_material() {
    let mut r=room(52.);tick(&mut r);
    // The horizontal branch has only its source. Independent vertical material
    // crosses a future horizontal square; observation must not add a junction.
    let at=14*79+32;let f=r.floor_mut().unwrap();
    f.roots.as_mut().unwrap()[at]=1.;f.roots.as_mut().unwrap()[at+79]=1.;
    f.fungal_roots.as_mut().unwrap().push(corpse_core::FungalRoot {
        fungus_id:2000.,corpse_id:2001.,path:vec![at as f64,(at+79)as f64],active:true,
        color:fungi::ROOT_COLOR.into(),..Default::default()
    });
    let h=hazard_memory::observed_hazards_at_revision(f,at,0.,52.);
    assert_eq!(h.root.as_ref().unwrap().glyph,"│");
}
