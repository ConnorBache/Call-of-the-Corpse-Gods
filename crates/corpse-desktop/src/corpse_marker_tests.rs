use super::*;
#[test]
fn corpse_cross_preserves_glyph_and_observed_death_semantics() {
    let mut r=corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let before=serde_json::to_string(&r).unwrap();let mut v=crate::observe::observe(&r);
    assert!(!v.cells.iter().find(|c|c.actor_id==Some(r.player.id)).unwrap().corpse);
    let id=r.actors_at(r.depth)[0].id;let a=r.actors_at_mut(r.depth).iter_mut().find(|a|a.id==id).unwrap();
    a.dead=true.into();a.hp=0.;a.heartless=true.into();
    v=crate::observe::observe(&r);
    let corpse=v.cells.iter().find(|c|c.actor_id==Some(id)).expect("visible corpse");
    assert!(corpse.corpse);assert!(!corpse.heartless);assert!(!corpse.glyph.is_empty());
    assert!(!v.creatures.iter().any(|a|a.id==id),"dead creatures do not become live previews");
    let frozen=serde_json::to_string(&r).unwrap();let mut p=Painter::new();p.game_at(&v,"",false,0.);
    assert_eq!(serde_json::to_string(&r).unwrap(),frozen);assert_ne!(before,frozen);
}
#[test]
fn corpse_cross_is_clipped_and_gpu_recording_does_not_paint_cpu_pixels() {
    for scale in [1.,1.5,2.] {
        let mut p=Painter::new();p.resize((1280.*scale)as usize,(800.*scale)as usize);p.clear([10,20,30]);let before=p.pixels.clone();
        p.corpse_marker(40,50,15,24,None);
        let bounds=(p.native_x(40.),p.native_y(50.),p.native_x(55.),p.native_y(74.));let mut changed=0;
        for (i,(a,b)) in p.pixels.chunks_exact(4).zip(before.chunks_exact(4)).enumerate(){if a!=b {changed+=1;let(x,y)=((i%p.width)as i32,(i/p.width)as i32);assert!(x>=bounds.0&&x<bounds.2&&y>=bounds.1&&y<bounds.3);}}
        assert!(changed>0);
        p.gpu=Some(crate::gpu_render::Frame::default());p.pixels.fill(17);p.corpse_marker(40,50,15,24,None);
        assert!(p.pixels.iter().all(|&b|b==17));assert!(!p.gpu.as_ref().unwrap().quads.is_empty());
    }
}
#[test]
fn corpse_toggle_repairs_retained_and_tile_frames() {
    let engine=corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let mut v=crate::observe::observe(&engine.run);let mut p=Painter::new();let mut full=Painter::new();let mut retained=RetainedGame::default();
    retained.update(&mut p,&v,"",0.,true,true).unwrap();
    let cell=v.cells.iter_mut().find(|c|c.actor_id==Some(engine.run.player.id)).unwrap();cell.corpse=true;cell.heartless=false;
    retained.update(&mut p,&v,"",0.,true,false).unwrap();full.game_at(&v,"",false,0.);assert_eq!(p.pixels,full.pixels);
    let old=crate::observe::observe(&engine.run);let mut plan=tile_damage::TilePlanner::default();let mut p=Painter::new();
    plan.replace_view(None,&old,&p);plan.finish_damage(&mut p);plan.paint_base(&mut p,&old,"",0.);p.tile_clip_enabled=false;plan.clear_frame();
    plan.replace_view(Some(&old),&v,&p);plan.finish_damage(&mut p);plan.paint_base(&mut p,&v,"",0.);p.tile_clip_enabled=false;assert_eq!(p.pixels,full.pixels);
}
