use corpse_core::{Actor, Field, Floor, Run, engine, environment, fungi, hazard_memory, population, storage};
use std::{collections::HashSet, sync::Arc};

fn room(revision: f64) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=revision.into();r.tick=0.;r.depth=1.;r.next_id=1100.;
    r.player.x=34.;r.player.y=13.;r.player.abilities.clear();r.player.active.clear();
    r.player.ability_order=Field::Missing;r.player.ability_reserves=Field::Missing;r.player.conditions.clear();
    r.player.stats.perception=300.;r.inventory.clear();r.offers.clear();r.history.clear();r.future=None;
    r.actors.clear();r.ground.clear();
    let (w,h)=(79,29);let mut tiles=vec!["floor".into();w*h];
    for i in 0..w*h {if i%w==0||i%w==w-1||i/w==0||i/w==h-1 {tiles[i]="wall".into();}}
    tiles[80]="up".into();tiles[w*h-w-2]="down".into();
    let mut f=Floor {depth:1.,domain:"Dungeon".into(),tiles,lights:vec![100.;w*h],seen:vec![true;w*h],
        width:(w as f64).into(),height:(h as f64).into(),dirty:true,..Default::default()};
    environment::initialize(&mut f).unwrap();r.floors=vec![Arc::new(f)];
    r.actors_at_mut(1.).push(population::create_actor_revision("bastidion",1000.,39.,14.,false,revision).unwrap());
    for (id,x,y) in [(1001.,47.,12.),(1002.,45.,19.),(1003.,31.,12.),(1004.,35.,20.)] {
        let mut a=population::create_actor_revision("rat",id,x,y,false,revision).unwrap();
        a.dead=true.into();a.hp=0.;a.heartless=true.into();a.corpse_age=0.0.into();r.actors_at_mut(1.).push(a);
    }
    r.ground_at_mut(1.).clear();r
}
fn tick(r:&mut Run) {let frozen=vec![false;r.floor().unwrap().tiles.len()];fungi::tick_fungi(r,&frozen).unwrap();}
fn roots(r:&Run)->&[corpse_core::FungalRoot] {r.floor().unwrap().fungal_roots.as_ref().unwrap()}
fn live_cells(r:&Run)->HashSet<usize> {roots(r).iter().filter(|link|link.active).flat_map(|link|link.path.iter().map(|&at|at as usize)).collect()}
fn assert_exact_material(r:&Run) {
    let expected=live_cells(r);
    let actual:HashSet<_>=r.floor().unwrap().roots.as_ref().unwrap().iter().enumerate()
        .filter_map(|(at,&value)|(value>0.).then_some(at)).collect();
    assert_eq!(actual,expected,"material must be exactly surviving connection cells");
}
fn occupant(r:&Run,id:f64)->&Actor {engine::actor(r,id).unwrap()}

#[test]
fn every_target_gets_one_shortest_direct_olive_path_without_capping_acquisition() {
    let mut r=room(51.);let rng=r.rng;tick(&mut r);
    assert_eq!(roots(&r).len(),4,"two initial meals is not an ongoing acquisition cap");
    for link in roots(&r) {
        let source=occupant(&r,link.fungus_id);let target=occupant(&r,link.corpse_id);
        let (dx,dy)=(target.x as i32-source.x as i32,target.y as i32-source.y as i32);
        let longest=dx.abs().max(dy.abs());
        assert_eq!(link.path.len(),longest as usize+1);assert_eq!(link.color,fungi::ROOT_COLOR);
        assert_eq!(link.path.first(),Some(&(source.y*79.+source.x)));
        assert_eq!(link.path.last(),Some(&(target.y*79.+target.x)));
        for &at in &link.path {
            let (x,y)=(at as i32%79-source.x as i32,at as i32/79-source.y as i32);
            assert!((2*(dx*y-dy*x)).abs()<=longest,"path deviates beyond raster rounding");
        }
    }
    let before=roots(&r).to_vec();for _ in 0..4 {tick(&mut r);}
    assert_eq!(roots(&r),before);assert_eq!(r.rng,rng);assert_exact_material(&r);
    assert_eq!(fungi::emission(&r,occupant(&r,1000.)).0,"hallucinogenic");
}
#[test]
fn severing_removes_the_entire_orphan_branch_but_preserves_shared_live_cells_and_history() {
    let mut r=room(51.);tick(&mut r);
    let cut=roots(&r)[0].path[4] as usize;
    environment::burn_roots_revision(r.floor_mut().unwrap(),cut,51.);
    assert!(!roots(&r)[0].active);assert!(roots(&r)[1..].iter().all(|link|link.active));
    assert_exact_material(&r);assert!(r.floor().unwrap().roots.as_ref().unwrap()[14*79+39]>0.);
    let severed=roots(&r)[0].clone();tick(&mut r);
    assert_eq!(roots(&r).len(),4);assert_eq!(roots(&r)[0],severed,"saved pair prevents reacquisition");
    assert_exact_material(&r);
}
#[test]
fn loaded_stale_material_and_duplicate_pairs_are_cleaned_idempotently_without_rng() {
    let mut r=room(51.);tick(&mut r);let first=roots(&r)[0].clone();
    let mut duplicate=first.clone();duplicate.path.insert(1,(14*79+40) as f64);
    r.floor_mut().unwrap().fungal_roots.as_mut().unwrap().push(duplicate);
    let stale=roots(&r)[2].path.clone();r.floor_mut().unwrap().fungal_roots.as_mut().unwrap()[2].active=false;
    let orphan=5*79+5;r.floor_mut().unwrap().roots.as_mut().unwrap()[orphan]=1.;
    for link in r.floor_mut().unwrap().fungal_roots.as_mut().unwrap() {link.color="#ee6cff".into();}
    let rng=r.rng;
    let loaded=storage::deserialize_compatible(&storage::serialize(&r).unwrap()).unwrap();
    assert_eq!(roots(&loaded).len(),4);assert!(!roots(&loaded)[2].active);
    assert_eq!(roots(&loaded)[0].path,first.path);assert_exact_material(&loaded);assert_eq!(loaded.rng,rng);
    assert!(roots(&loaded).iter().all(|link|link.color==fungi::ROOT_COLOR));
    assert!(stale.iter().any(|&at|!live_cells(&loaded).contains(&(at as usize))));
    let again=storage::deserialize_compatible(&storage::serialize(&loaded).unwrap()).unwrap();
    assert_eq!(roots(&loaded),roots(&again));assert_eq!(loaded.floor().unwrap().roots,again.floor().unwrap().roots);
    assert_eq!(again.rng,rng);
}
#[test]
fn an_inactive_first_pair_cannot_be_revived_by_a_duplicate_saved_active_pair() {
    let mut r=room(51.);tick(&mut r);let duplicate=roots(&r)[0].clone();
    let links=r.floor_mut().unwrap().fungal_roots.as_mut().unwrap();links[0].active=false;links.push(duplicate);
    tick(&mut r);assert_eq!(roots(&r).len(),4);assert!(!roots(&r)[0].active);assert_exact_material(&r);
}
#[test]
fn moved_or_revived_targets_and_moved_sources_sever_old_paths_instead_of_rerouting() {
    for mutation in ["target-move","target-revive","source-move"] {
        let mut r=room(51.);tick(&mut r);let paths=roots(&r).iter().map(|link|link.path.clone()).collect::<Vec<_>>();
        match mutation {
            "target-move"=>engine::actor_mut(&mut r,1001.).unwrap().x+=1.,
            "target-revive"=>{let a=engine::actor_mut(&mut r,1001.).unwrap();a.dead=false.into();a.hp=10.;},
            _=>engine::actor_mut(&mut r,1000.).unwrap().x+=1.,
        }
        tick(&mut r);assert_eq!(roots(&r).len(),4);assert!(!roots(&r)[0].active);
        if mutation=="source-move" {assert!(roots(&r).iter().all(|link|!link.active));}
        else {assert!(roots(&r)[1..].iter().all(|link|link.active));}
        assert_eq!(roots(&r).iter().map(|link|link.path.clone()).collect::<Vec<_>>(),paths);
        assert_exact_material(&r);
    }
}
#[test]
fn fungal_source_death_clears_roots_immediately_before_another_environment_tick() {
    let mut r=room(51.);tick(&mut r);engine::actor_mut(&mut r,1000.).unwrap().hp=0.;
    engine::death_if_nonpositive(&mut r,1000.).unwrap();
    assert!(roots(&r).iter().all(|link|!link.active));assert_exact_material(&r);
    assert!(r.floor().unwrap().roots.as_ref().unwrap().iter().all(|&value|value==0.));
}
#[test]
fn hidden_root_memory_is_retained_until_observation_without_live_stale_glyphs() {
    let mut r=room(51.);tick(&mut r);let at=roots(&r)[0].path[5] as usize;
    let mut mask=vec![false;r.floor().unwrap().tiles.len()];mask[at]=true;
    hazard_memory::remember_observed_hazards_revision(r.floor_mut().unwrap(),&mask,0.,51.);
    let remembered=r.floor().unwrap().remembered_hazards.as_ref().unwrap()[&at.to_string()].clone();
    let cut=roots(&r)[0].path[4] as usize;environment::burn_roots_revision(r.floor_mut().unwrap(),cut,51.);
    mask[at]=false;hazard_memory::remember_observed_hazards_revision(r.floor_mut().unwrap(),&mask,1.,51.);
    assert_eq!(r.floor().unwrap().remembered_hazards.as_ref().unwrap()[&at.to_string()],remembered);
    assert!(hazard_memory::observed_hazards_at_revision(r.floor().unwrap(),at,1.,51.).root.is_missing());
    mask[at]=true;hazard_memory::remember_observed_hazards_revision(r.floor_mut().unwrap(),&mask,2.,51.);
    assert!(r.floor().unwrap().remembered_hazards.as_ref().unwrap().get(&at.to_string()).is_none_or(|h|h.root.is_missing()));
}
#[test]
fn legacy_roots_keep_old_color_and_severed_material() {
    let mut r=room(50.);tick(&mut r);assert!(roots(&r).iter().all(|link|link.color==fungi::HALLUCINOGENIC_COLOR));
    let path=roots(&r)[0].path.clone();environment::burn_roots_revision(r.floor_mut().unwrap(),path[4] as usize,50.);
    assert!(r.floor().unwrap().roots.as_ref().unwrap()[path[5] as usize]>0.);
    assert!(hazard_memory::observed_hazards_at_revision(r.floor().unwrap(),path[5] as usize,0.,50.).root.as_ref().is_some());
}
#[test]
fn native_root_review_exports_actual_simulation_states_when_requested() {
    let Ok(path)=std::env::var("CORPSE_ROOT_REVIEW") else{return;};
    std::fs::create_dir_all(&path).unwrap();
    let mut r=room(51.);tick(&mut r);
    r.log=vec!["Native R51: four shortest direct roots; authored olive material; gas unchanged.".into()];
    r.log_kinds=vec!["environment".into()].into();
    std::fs::write(format!("{path}/after-four-live.json"),storage::serialize(&r).unwrap()).unwrap();
    let cut=roots(&r)[0].path[4] as usize;environment::burn_roots_revision(r.floor_mut().unwrap(),cut,51.);
    r.log[0]="Native R51: severed branch removed; three surviving branches preserved.".into();
    std::fs::write(format!("{path}/after-severed.json"),storage::serialize(&r).unwrap()).unwrap();
    engine::actor_mut(&mut r,1000.).unwrap().hp=0.;engine::death_if_nonpositive(&mut r,1000.).unwrap();
    r.log=vec!["Native R51: dead fungus leaves no stale root geometry.".into()];r.log_kinds=vec!["environment".into()].into();
    std::fs::write(format!("{path}/after-source-death.json"),storage::serialize(&r).unwrap()).unwrap();
}
