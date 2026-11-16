use super::*;
use crate::{observe, render::{Painter, tile_damage::TilePlanner}, session_store::tracked_engine as temporal};
use corpse_core::{Field, Stats, revision50_amulet_effects::{self as amulets, Effect}};

pub(crate) fn room() -> Run {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=52.0.into(); r.player.x=10.;r.player.y=10.;r.player.energy=200.;
    r.player.stats=Stats {max_energy:200.,perception:300.,..Default::default()};
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.ability_reserves=Field::Missing;r.player.amulet_ledger=Field::Missing;
    r.player.equipment_abilities=Field::Missing;r.player.ability_order=Field::Missing;
    r.player.conditions.insert("allSeeingReveal".into(),1.);
    r.player.conditions.insert("allSeeingDepth".into(),r.depth);
    r.inventory=vec![amulets::make_amulet_for_revision(Effect::Firebolt,900.,52.).unwrap()];
    r.actors.clear();r.ground.clear();r.offers.clear();r.next_id=1000.;
    for depth in r.floors.iter().map(|f|f.depth).collect::<Vec<_>>() {r.actors_at_mut(depth);r.ground_at_mut(depth);}
    r.history.clear();r.replay.clear();r.haste_snapshot=Field::Missing;r.future=None;r.phase="normal".to_string().into();
    let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.seen.fill(true);
    f.remembered_tiles=Field::Missing;f.environment=Field::Missing;f.light_sources=Field::Missing;
    f.doors=Field::Missing;f.portals=Field::Missing;f.non_euclidean=Field::Missing;
    f.speed_up=Field::Missing;f.godbeast_event=Field::Missing;f.currents=Field::Missing;
    f.roots=Field::Missing;f.mud=Field::Missing;f.slime=Field::Missing;
    let w=f.width();for x in [12,15,20] {f.tiles[10*w+x]="grass".into();}f.tiles[10*w+25]="wall".into();
    r
}
fn cast()->(temporal::Engine,effects::Effects,View) {
    let mut engine=temporal::Engine::new(room()).unwrap();
    let mut effects=effects::Effects::default();
    crate::dispatch(&mut engine,&mut effects,0.,&Command::UseAt{id:900.,x:11.,y:10.}).unwrap();
    effects.begin_frame(1000.);
    let view=observe::observe(&engine.run);
    (engine,effects,view)
}
fn fire(cells:&[Cell],x:usize)->bool {cells.iter().any(|cell|cell.x==x&&cell.y==10&&cell.fire)}
fn arrival(effects:&effects::Effects,x:usize)->f64 {
    let cue=effects.cues.iter().find(|c|c.event.animation.as_ref().is_some_and(|a|a.proposal=="A02"&&a.stage==Stage::Travel)).unwrap();
    let position=cue.event.animation.as_ref().unwrap().cells.iter().position(|p|p.x==x as f64&&p.y==10.).unwrap();
    cue.started+effects::arrival_ms(&cue.event,position)
}

#[test]
fn firebolt_advances_terrain_without_reverting_current_actors_or_knowledge() {
    let (engine,effects,mut view)=cast();
    let saved=engine.save().unwrap();
    assert!(fire(&view.cells,12)&&fire(&view.cells,20));
    // An actor overlay is deliberately changed after simulation: staging must
    // retain exactly that post-turn layer, not resurrect the outgoing actor.
    let actor=Cell {x:20,y:10,glyph:"X".into(),actor_id:Some(991.),current:true,direct:true,fg:[210,230,250],bg:[12,13,14],..Default::default()};
    view.cells.push(actor.clone());
    let canonical=view.cells.clone();let snapshot=view.terrain_snapshot.as_ref().unwrap().clone();
    let mut stage=None;
    assert!(effects.firebolt_terrain.prepare(&view,&effects.cues,1000.,false,&mut stage));
    let first=stage.as_ref().unwrap();
    assert!(!fire(&first.cells,12)&&!fire(&first.cells,20));
    assert_eq!(first.cells.last(),Some(&actor));
    assert_eq!(view.cells,canonical);assert!(Arc::ptr_eq(view.terrain_snapshot.as_ref().unwrap(),&snapshot));
    let cells=first.cells.clone();
    assert!(!effects.firebolt_terrain.prepare(&view,&effects.cues,1001.,false,&mut stage));
    assert!(Arc::ptr_eq(&cells,&stage.as_ref().unwrap().cells),"unchanged display frame reuses detached paint cache");
    assert!(effects.firebolt_terrain.prepare(&view,&effects.cues,arrival(&effects,12),false,&mut stage));
    assert!(fire(&stage.as_ref().unwrap().cells,12));assert!(!fire(&stage.as_ref().unwrap().cells,20));
    effects.firebolt_terrain.prepare(&view,&effects.cues,arrival(&effects,20),false,&mut stage);
    assert!(fire(stage.as_ref().map(|s|s.cells.as_slice()).unwrap_or(&view.cells),20));
    assert!(effects.firebolt_terrain.prepare(&view,&effects.cues,10000.,false,&mut stage));
    assert!(stage.is_none());assert_eq!(view.cells,canonical);assert_eq!(engine.save().unwrap(),saved);
}

#[test]
fn interruption_reduced_instant_visibility_loss_and_nonfirebolt_stay_canonical() {
    let (_engine,mut effects,mut view)=cast();let canonical=view.cells.clone();let mut stage=None;
    effects.firebolt_terrain.prepare(&view,&effects.cues,1000.,false,&mut stage);assert!(stage.is_some());
    assert!(effects.firebolt_terrain.prepare(&view,&effects.cues,1000.,true,&mut stage));assert!(stage.is_none());
    effects.firebolt_terrain.prepare(&view,&effects.cues,1000.,false,&mut stage);
    effects.configure_motion(true,true);
    assert!(effects.firebolt_terrain.prepare(&view,&effects.cues,1000.,false,&mut stage));assert!(stage.is_none());
    effects.configure_motion(false,false);
    effects.firebolt_terrain.prepare(&view,&effects.cues,1000.,false,&mut stage);
    effects.clear();assert!(effects.firebolt_terrain.prepare(&view,&effects.cues,1000.,false,&mut stage));assert!(stage.is_none());
    assert_eq!(view.cells,canonical);
    let (_engine,effects,_)=cast();
    let at=10*view.width+20;
    let observation=Arc::make_mut(view.terrain_snapshot.as_mut().unwrap());
    observation.cells[at].current=false;observation.cells[at].direct=false;
    effects.firebolt_terrain.prepare(&view,&effects.cues,1000.,false,&mut stage);
    assert!(!stage.as_ref().unwrap().withheld.contains(&at));
    view.dreamwalking=!view.dreamwalking;
    assert!(effects.firebolt_terrain.prepare(&view,&effects.cues,1000.,false,&mut stage));
    assert!(stage.is_none(),"same-floor plane transitions discard the outgoing terrain mask");
    let mut r=room();
    for command in [Command::Wait,Command::Move{dx:1.,dy:0.},Command::UseAt{id:999.,x:11.,y:10.}] {assert!(!is_firebolt(&r,&command));}
    let command=Command::UseAt{id:900.,x:11.,y:10.};assert!(is_firebolt(&r,&command));
    r.inventory[0]=amulets::make_amulet_for_revision(Effect::Lightning,900.,52.).unwrap();assert!(!is_firebolt(&r,&command));
}

fn capture(p:&Painter,name:&str) {
    if let Ok(directory)=std::env::var("CORPSE_STAGED_FIREBOLT_REVIEW").or_else(|_|std::env::var("CORPSE_ACTION_REVIEW")) {
        std::fs::create_dir_all(&directory).unwrap();
        let file=std::fs::File::create(format!("{directory}/{name}.png")).unwrap();
        let mut encoder=png::Encoder::new(file,p.width as u32,p.height as u32);
        encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
    }
}
#[test]
fn staged_native_arrival_frames_match_full_and_keep_final_view_immutable() {
    let (engine,effects,view)=cast();let saved=engine.save().unwrap();let canonical=view.cells.clone();
    for animated in [false,true] {
        let mut tile=Painter::new();let mut full=Painter::new();let mut retained=Painter::new();let mut retained_game=crate::render::RetainedGame::default();
        if animated {tile.profile=tile.profile.set("lighting.flickerStrength",serde_json::json!(0.3)).unwrap();full.profile=tile.profile.clone();retained.profile=tile.profile.clone();}
        // Use actual profile-specific detached observations, including dynamic
        // reconstruction. Staging uses only already admitted route information.
        let view=observe::observe_with_profile(&engine.run,&tile.profile);
        let mut stage=Staging::default();stage.capture(observe::observe_with_profile(&room(),&tile.profile),&effects.cues);
        for painter in [&mut tile,&mut full,&mut retained] {painter.resize(960,600);painter.set_responsive_canvas(true);}
        let mut plan=TilePlanner::default();plan.replace_view(None,&view,&tile);
        for (frame,now) in [1000.,1001.,arrival(&effects,12),arrival(&effects,15),arrival(&effects,20),10000.].into_iter().enumerate() {
            if stage.prepare(&view,&effects.cues,now,false,&mut tile.firebolt_terrain) {plan.invalidate();plan.replace_view(None,&view,&tile);}
            stage.prepare(&view,&effects.cues,now,false,&mut full.firebolt_terrain);
            if let Some(stage)=tile.firebolt_terrain.as_ref() {for rect in stage.damage(&tile,&view) {plan.mark_native_rect(rect);}}
            let changed=stage.prepare(&view,&effects.cues,now,false,&mut retained.firebolt_terrain);
            retained_game.update(&mut retained,&view,"",now,false,changed).unwrap();
            plan.mark_animation(&view,&tile,now,false);plan.finish_damage(&mut tile);plan.paint_base(&mut tile,&view,"",now);tile.tile_clip_enabled=false;
            full.game_at(&view,"",false,now);
            assert_eq!(tile.pixels,full.pixels,"staged native frame {frame}, animated={animated}");
            assert_eq!(retained.pixels,full.pixels,"staged retained frame {frame}, animated={animated}");
            capture(&full,&format!("firebolt-terrain-{}-{frame}",if animated{"custom-light"}else{"default"}));
            // Capture the complete production composition only after the three
            // independent base-paint oracles agree. Next full paint clears it.
            full.effects(&view,&effects.cues,now,false);
            capture(&full,&format!("firebolt-staged-flight-{}-{frame}",if animated{"custom-light"}else{"default"}));
            plan.clear_frame();
        }
    }
    assert_eq!(view.cells,canonical);assert_eq!(engine.save().unwrap(),saved);
}

fn shot_room()->Run {
    let mut run=room();
    run.inventory=vec![
        corpse_core::Item{id:900.,name:"Bow".into(),kind:"weapon".into(),damage:15.0.into(),ranged_weapon:"bow".to_string().into(),hands:2.0.into(),equipped:"hand1".to_string().into(),..Default::default()},
        corpse_core::Item{id:901.,name:"Arrow".into(),kind:"ammo".into(),effect:"arrows".to_string().into(),quantity:5.0.into(),..Default::default()}];
    run
}
#[test]
fn exact_new_landed_item_waits_for_flight_without_hiding_actors_or_custom_glyphs() {
    let mut engine=temporal::Engine::new(shot_room()).unwrap();let mut effects=effects::Effects::default();
    crate::dispatch(&mut engine,&mut effects,0.,&Command::Shoot{id:900.,x:20.,y:10.}).unwrap();effects.begin_frame(1000.);
    let cue=effects.cues.iter().find(|c|c.event.animation.as_ref().is_some_and(|a|a.delivery==Delivery::Projectile&&a.item_id.is_some())).unwrap();
    let animation=cue.event.animation.as_ref().unwrap();let item=animation.item_id.unwrap().to_bits();
    let arrival=cue.started+effects::arrival_ms(&cue.event,animation.cells.len()-1);
    let mut view=observe::observe(&engine.run);let saved=engine.save().unwrap();
    assert!(view.cells.iter().any(|cell|cell.ground_item_id==Some(item)));
    // Identity is independent from canonical flight versus custom item glyphs.
    for cell in &mut view.cells {if cell.ground_item_id==Some(item){cell.glyph="$".into();}}
    for row in &mut view.ground_entities {if row.id==item{row.glyph="$".into();}}
    let actor=Cell{x:20,y:10,actor_id:Some(1999.),glyph:"M".into(),..Default::default()};view.cells.push(actor.clone());
    let canonical=view.cells.clone();let mut stage=None;
    assert!(effects.firebolt_terrain.prepare(&view,&effects.cues,1000.,false,&mut stage));
    assert!(!stage.as_ref().unwrap().cells.iter().any(|cell|cell.ground_item_id==Some(item)));
    assert_eq!(stage.as_ref().unwrap().cells.last(),Some(&actor));
    let first=stage.as_ref().unwrap().cells.clone();
    assert!(!effects.firebolt_terrain.prepare(&view,&effects.cues,arrival-1.,false,&mut stage));
    assert!(Arc::ptr_eq(&first,&stage.as_ref().unwrap().cells));
    assert!(effects.firebolt_terrain.prepare(&view,&effects.cues,arrival,false,&mut stage));assert!(stage.is_none());
    assert_eq!(view.cells,canonical);assert_eq!(engine.save().unwrap(),saved);
    // A pre-existing observed item is never hidden, even with the same exact ID
    // in a deliberately constructed detached presentation fixture.
    let mut before=observe::observe(&shot_room());
    before.ground_entities.push(view.ground_entities.iter().find(|row|row.id==item).unwrap().clone());
    let mut already_there=Staging::default();already_there.capture(before,&effects.cues);
    assert!(!already_there.prepare(&view,&effects.cues,1000.,false,&mut stage));assert!(stage.is_none());
}
#[test]
fn projectile_endpoint_mask_does_not_touch_existing_ground_or_scattered_landing() {
    let mut run=shot_room();let depth=run.depth;
    run.ground_at_mut(depth).push(corpse_core::model::GroundItem{x:20.,y:10.,item:corpse_core::Item{id:777.,name:"Ring of Stealth".into(),kind:"ring".into(),glyph:"o".into(),effect:"stealth".to_string().into(),..Default::default()},auto_pickup_blocked:Field::Missing});
    let mut engine=temporal::Engine::new(run).unwrap();let mut effects=effects::Effects::default();
    crate::dispatch(&mut engine,&mut effects,0.,&Command::Shoot{id:900.,x:20.,y:10.}).unwrap();effects.begin_frame(1000.);
    let view=observe::observe(&engine.run);let mut stage=None;
    assert!(view.cells.iter().any(|cell|cell.ground_item_id==Some(777.0f64.to_bits())));
    assert!(!effects.firebolt_terrain.prepare(&view,&effects.cues,1000.,false,&mut stage));assert!(stage.is_none());
    let shot=effects.cues.iter().find_map(|cue|cue.event.animation.as_ref().filter(|a|a.delivery==Delivery::Projectile).and_then(|a|a.item_id)).unwrap();
    assert!(view.ground_entities.iter().any(|row|row.id==shot.to_bits()&&(row.x,row.y)!=(20,10)),"occupied landing scatters the arrow; no endpoint inference is made");
    let mut potion=shot_room();potion.inventory.push(corpse_core::Item{id:990.,kind:"potion".into(),..Default::default()});
    assert!(!needs_snapshot(&potion,&Command::Throw{id:990.,x:20.,y:10.}));
}
