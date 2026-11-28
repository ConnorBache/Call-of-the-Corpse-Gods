//! Presentation regressions driven by canonical feeding turns, not invented flags.
use super::feeding_description;
use corpse_core::{creature_feeding as food, engine, Actor, Field, Run};

const ID: f64 = 900.;
fn room(kind: &str) -> Run {
    let mut r: Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=52.0.into();r.tick=0.;r.phase=Field::Missing;
    r.floors.retain(|f|f.depth==1.);r.actors.clear();r.history.clear();r.future=None;
    r.inventory.clear();r.ground.clear();r.ground_at_mut(1.).clear();r.offers.clear();r.next_id=901.;
    r.player.x=10.;r.player.y=10.;r.player.abilities.clear();r.player.active.clear();
    r.player.ability_order=Field::Missing;r.player.equipment_abilities=Field::Missing;
    // Keep the template log and its matching logKinds together.
    r.player.conditions.clear();r.player.conditions.insert("invisible".into(),10000.);
    r.player.stats.perception=100.;
    let f=r.floor_mut().unwrap();let n=f.tiles.len();let w=f.width();let h=f.height();
    for y in 1..h-1 {for x in 1..w-1 {f.tiles[y*w+x]="floor".into();}}
    f.lights=vec![100.;n];f.seen=vec![true;n];f.last_tick=0.;
    f.environment=Field::Missing;f.roots=Field::Missing;f.fungal_roots=Field::Missing;
    f.ichor=Field::Missing;f.slime=Field::Missing;f.mud=Field::Missing;f.razorleaves=Field::Missing;
    f.vents=Field::Missing;f.permanent_fire=Field::Missing;f.currents=Field::Missing;f.doors=Field::Missing;
    f.water_blood=Field::Missing;f.kelpie_blood_sources=Field::Missing;
    let mut marks=vec![false;n];marks[10*w+13]=true;f.bloody_walls=marks.into();f.tiles[10*w+13]="wall".into();
    let mut a=corpse_core::population::create_actor_revision(kind,ID,12.,10.,false,52.).unwrap();
    a.state="wandering".into();a.dead=false.into();a.last_seen=Field::Missing;
    a.conditions.clear();a.conditions.insert(food::AGE.into(),99.);
    r.actors_at_mut(1.).push(a);
    corpse_core::storage::validate(&r).expect("valid feeding-room fixture");
    r
}
fn actor(r:&Run)->&Actor {engine::actor(r,ID).unwrap()}
fn actor_mut(r:&mut Run)->&mut Actor {engine::actor_mut(r,ID).unwrap()}
fn step(r:&mut Run) -> bool {
    r.tick+=1.;r.resolving_tick=true.into();food::begin_turn(r);
    let used=food::take_turn(r,ID,false,false).unwrap();food::end_turn(r);
    r.resolving_tick=Field::Missing;used
}
fn is_feeding(r:&Run)->bool {
    crate::observe::observe(r).creatures.iter().find(|a|a.id==ID)
        .is_some_and(|a|a.statuses.iter().any(|s|s.name=="Feeding"))
}
fn assert_visible_state(r:&Run,feeding:bool) {
    let before=serde_json::to_vec(r).unwrap();let view=crate::observe::observe(r);
    let row=view.creatures.iter().find(|a|a.id==ID).unwrap();
    assert_eq!(row.state,actor(r).state,"behavior identity is preserved");
    assert_eq!(row.statuses.iter().filter(|s|s.name=="Feeding").count(),usize::from(feeding));
    let report=crate::inspection_ui::observe_tile(r,actor(r).x as usize,actor(r).y as usize).creature.unwrap();
    assert_eq!(report.conditions.contains("Feeding"),feeding);
    assert_eq!(report.scene.contains("licks the bloody wall as it "), feeding, "{}", report.scene);
    if feeding {
        let posture = if corpse_core::travel::flying(r, actor(r)) { "flies" } else { "stands" };
        assert!(report.scene.contains(&format!("licks the bloody wall as it {posture}")), "{}", report.scene);
    }
    assert!(!report.behavior.contains("Feeding")&&!report.behavior.contains("Eating"),"no duplicated activity");
    assert_eq!(serde_json::to_vec(r).unwrap(),before,"presentation never mutates simulation or RNG");
}

#[test]
fn wall_feeding_starts_only_after_actual_meal_and_ends_on_tenth_turn() {
    let mut r=room("cave-bat");
    assert_visible_state(&r,false); // Beside a Bloody wall, not hungry yet.
    actor_mut(&mut r).conditions.insert(food::AGE.into(),100.);
    assert_eq!(feeding_description(&r,actor(&r),true,false),Some("Hungering"));
    assert_visible_state(&r,false); // Hunger and wall proximity are not activity.
    for count in 1..=9 {
        assert!(step(&mut r));
        assert_eq!(engine::condition(actor(&r),"creatureFeedingTurns"),count as f64);
        assert_eq!(engine::condition(actor(&r),"creatureFeedingKind"),1.);
        assert_eq!(food::description(&r,actor(&r)),Some("Eating"),"core terminology and state stay unchanged");
        assert_visible_state(&r,true);
    }
    let report=crate::inspection_ui::observe_tile(&r,12,10).creature.unwrap();
    assert!(report.conditions.split(", ").any(|s|s=="Flying"),"{}",report.conditions);
    assert!(report.conditions.ends_with(", Feeding"),"{}",report.conditions);
    let view=crate::observe::observe(&r);
    assert_eq!(crate::statuses_ui::summary(&view.creatures[0].statuses),report.conditions);
    assert!(step(&mut r));assert_eq!(engine::condition(actor(&r),food::AGE),0.);
    assert_visible_state(&r,false);
    for _ in 0..99 {assert!(!step(&mut r));}
    assert_visible_state(&r,false);assert!(step(&mut r));assert_visible_state(&r,true);
}

#[test]
fn interruptions_remove_feeding_before_the_next_observation() {
    for interruption in ["damage","hunting","movement","source","paralyzed","missed-turn"] {
        let mut r=room("cave-bat");for _ in 0..3 {assert!(step(&mut r));}
        assert_visible_state(&r,true);
        match interruption {
            "damage"=>{engine::damage_environment(&mut r,ID,1.,None,"attack",true).unwrap();},
            "hunting"=>actor_mut(&mut r).state="hunting".into(),
            "movement"=>actor_mut(&mut r).y=11.,
            "source"=>{let at=10*r.floor().unwrap().width()+13;r.floor_mut().unwrap().bloody_walls.as_mut().unwrap()[at]=false;},
            "paralyzed"=>{actor_mut(&mut r).conditions.insert("paralyzed".into(),2.);},
            _=>r.tick+=1.,
        }
        food::end_turn(&mut r);
        assert_eq!(engine::condition(actor(&r),"creatureFeedingTurns"),0.,"{interruption}");
        assert_visible_state(&r,false);
    }
}

#[test]
fn water_drinking_and_food_seeking_do_not_become_wall_feeding() {
    let mut r=room("rat");let at=10*r.floor().unwrap().width()+13;let n=r.floor().unwrap().tiles.len();
    r.floor_mut().unwrap().tiles[at]="water".into();
    let mut blood=vec![0.;n];blood[at]=3.;r.floor_mut().unwrap().water_blood=blood.into();
    assert!(step(&mut r));assert_eq!(engine::condition(actor(&r),"creatureFeedingKind"),2.);
    assert_eq!(feeding_description(&r,actor(&r),true,false),Some("Drinking"));assert_visible_state(&r,false);
    assert!(crate::inspection_ui::observe_tile(&r,12,10).creature.unwrap().behavior.contains("Drinking"));
    food::interrupt(actor_mut(&mut r));actor_mut(&mut r).x=11.;
    assert!(!step(&mut r));assert_visible_state(&r,false);
    assert!(crate::inspection_ui::observe_tile(&r,11,10).creature.unwrap().behavior.contains("Hungering"));
}

#[test]
fn hidden_and_sensed_creatures_never_reveal_feeding() {
    let mut r=room("rat");assert!(step(&mut r));assert_visible_state(&r,true);
    let w=r.floor().unwrap().width();let h=r.floor().unwrap().height();
    for y in 1..h-1 {r.floor_mut().unwrap().tiles[y*w+11]="wall".into();}
    r.player.stats.perception=0.;
    assert!(!corpse_core::vision::directly_visible_actor(&r,actor(&r)));
    assert!(crate::observe::observe(&r).creatures.is_empty());
    let hidden=crate::inspection_ui::observe_tile(&r,12,10);assert!(hidden.creature.is_none());
    r.player.conditions.insert("telepathic".into(),10.);
    let view=crate::observe::observe(&r);let row=view.creatures.iter().find(|a|a.id==ID).unwrap();
    assert_eq!(row.state,"sensed");assert!(!row.statuses.iter().any(|s|s.name=="Feeding"));
    let report=crate::inspection_ui::observe_tile(&r,12,10).creature.unwrap();
    assert!(!report.conditions.contains("Feeding")&&!report.behavior.contains("Eating"));
    assert!(!report.scene.contains("licks the bloody wall"));
    r.player.conditions.shift_remove("telepathic");food::interrupt(actor_mut(&mut r));
    assert_eq!(crate::inspection_ui::observe_tile(&r,12,10),hidden,"unseen activity does not update memory");
}

#[test]
fn disguised_and_all_six_hallucinated_presentations_hide_feeding() {
    let mut r=room("skin-walker");r.player.x=8.;assert!(step(&mut r));
    assert!(crate::creature_presentation::project(&r,actor(&r)).is_some());
    assert!(!is_feeding(&r));
    let report=crate::inspection_ui::observe_tile(&r,12,10).creature.unwrap();
    assert!(!report.conditions.contains("Feeding")&&!report.behavior.contains("Eating"));
    assert!(!report.scene.contains("licks the bloody wall"));
    r.player.x=10.;assert!(crate::creature_presentation::project(&r,actor(&r)).is_none());
    assert_visible_state(&r,true);
    r.player.conditions.insert("hallucinating".into(),10.);
    let before=r.clone();let view=crate::observe::observe(&r);
    let row=view.creatures.iter().find(|a|a.id==ID).unwrap();
    for variant in row.hallucination.as_ref().unwrap().iter() {
        assert!(!variant.state.contains("Feeding")&&!variant.statuses.iter().any(|s|s.name.contains("Feeding")));
    }
    let report=crate::inspection_ui::observe_tile(&r,12,10);
    for variant in report.hallucination.as_ref().unwrap().iter() {
        let c=variant.creature.as_ref().unwrap();
        assert!(!c.conditions.contains("Feeding")&&!c.behavior.contains("Eating"));
        assert!(!c.scene.contains("licks the bloody wall"));
    }
    assert_eq!(r,before);
}

#[test]
fn feeding_display_survives_storage_snapshot_and_keeps_legacy_gating() {
    let mut r=room("rat");assert!(step(&mut r));
    let raw=corpse_core::storage::serialize(&r).unwrap();
    let loaded=corpse_core::storage::deserialize(&raw).unwrap();assert_eq!(loaded,r);assert_visible_state(&loaded,true);
    let snapshot=corpse_core::temporal::Snapshot::capture(&r);
    for _ in 0..9 {step(&mut r);}assert_visible_state(&r,false);
    snapshot.restore(&mut r).unwrap();assert_eq!(r,loaded);assert_visible_state(&r,true);
    r.simulation_revision=32.0.into();assert_visible_state(&r,false);
}

fn capture(p:&crate::render::Painter,dir:&str,name:&str) {
    let file=std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
    let mut encoder=png::Encoder::new(file,p.width as u32,p.height as u32);
    encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
}
#[test]
fn feeding_native_rail_list_and_inspection_share_detached_comma_states() {
    let mut r=room("cave-bat");assert!(step(&mut r));assert_visible_state(&r,true);
    let before=r.clone();let mut view=crate::observe::observe(&r);
    view.provenance="CONTROLLED feeding fixture; actual canonical meal; native renderer".into();
    let report=crate::inspection_ui::observe_tile(&r,12,10);
    let dir=std::env::var("CORPSE_FEEDING_STATE_REVIEW").ok();
    if let Some(dir)=&dir {std::fs::create_dir_all(dir).unwrap();std::fs::write(format!("{dir}/feeding-run.json"),corpse_core::storage::serialize(&r).unwrap()).unwrap();}
    for (w,h) in [(1280,800),(960,720)] {
        let mut p=crate::render::Painter::new();p.resize(w,h);p.set_responsive_canvas(true);
        p.game_at(&view,"",false,0.);let first=p.pixels.clone();
        p.game_at(&view,"",false,0.);assert_eq!(p.pixels,first,"repeated idle paint is stable");
        if let Some(dir)=&dir {capture(&p,dir,&format!("feeding-rail-{w}x{h}"));}
        p.creatures(&view,None,0);
        if let Some(dir)=&dir {capture(&p,dir,&format!("feeding-list-{w}x{h}"));}
        p.game_at(&view,"",false,0.);crate::inspection_ui::paint(&report,&mut p,0);
        if let Some(dir)=&dir {capture(&p,dir,&format!("feeding-inspection-{w}x{h}"));}
    }
    assert_eq!(r,before,"painting does not advance the meal");
}
