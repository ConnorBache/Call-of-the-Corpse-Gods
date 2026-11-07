//! Native water-depth presentation only; the canonical swimming rules stay untouched.
use crate::{appearance_profile::Profile, render::{Painter, RetainedGame, View, tile_damage::TilePlanner}};
use corpse_core::{Field, Floor, Item, Run};
use std::sync::Arc;

fn room() -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision = 51.0.into(); r.status = "playing".into();
    r.player.x = 7.; r.player.y = 9.;
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.ability_order = Field::Missing; r.player.equipment_abilities = Field::Missing;
    r.player.stats.perception = 300.;
    r.inventory.clear(); r.actors = Default::default(); r.ground.clear(); r.history.clear();
    r.offers.clear(); r.dreamwalking = Field::Missing;
    let (w,h) = (79,29); let mut tiles = vec!["floor".into();w*h];
    for y in 0..h { for x in 0..w {
        tiles[y*w+x] = if x==0||y==0||x==w-1||y==h-1 {"wall"}
            else if (4..15).contains(&y) && (10..17).contains(&x) {"shallow-water"}
            else if (4..15).contains(&y) && (17..29).contains(&x) {"water"}
            else {"floor"}.into();
    }}
    r.floors = vec![Arc::new(Floor { depth:r.depth, domain:"Dungeon".into(),
        width:(w as f64).into(), height:(h as f64).into(), remembered_tiles:tiles.clone().into(),
        tiles, lights:vec![100.;w*h], seen:vec![true;w*h], water_blood:vec![0.;w*h].into(),
        substrates:vec!["rock".into();w*h].into(), ..Default::default() })];
    r.log.clear();
    r
}
fn capture(name:&str,p:&Painter) {
    let Ok(dir)=std::env::var("CORPSE_R51_WATER_OUTPUT") else {return};
    std::fs::create_dir_all(&dir).unwrap();
    let file=std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
    let mut png=png::Encoder::new(file,p.width as u32,p.height as u32);
    png.set_color(png::ColorType::Rgba);png.set_depth(png::BitDepth::Eight);
    png.write_header().unwrap().write_image_data(&p.pixels).unwrap();
}
fn same(a:&Painter,b:&Painter,label:&str) {
    assert_eq!(a.pixels.len(),b.pixels.len());
    if let Some((i,(x,y)))=a.pixels.iter().zip(&b.pixels).enumerate().find(|(_, (x,y))|x!=y) {
        panic!("{label}: byte {i}: {x} != {y}");
    }
}
fn observed(r:&Run,expected:bool) -> View {
    let before=serde_json::to_vec(r).unwrap();let v=crate::observe::observe(r);
    assert_eq!(v.deep_water,expected);assert_eq!(serde_json::to_vec(r).unwrap(),before);
    v
}

#[test]
fn water_depth_observation_uses_true_swimming_not_glyph_hover_or_stale_status() {
    let mut r=room(); observed(&r,false);
    r.player.x=13.;observed(&r,false);
    r.player.x=20.;observed(&r,true);
    let at=9*79+20;Arc::make_mut(&mut r.floors[0]).ichor=vec![0.;79*29].into();
    Arc::make_mut(&mut r.floors[0]).ichor.as_mut().unwrap()[at]=1.;observed(&r,false);
    Arc::make_mut(&mut r.floors[0]).ichor.as_mut().unwrap()[at]=0.;observed(&r,true);
    r.player.dead=true.into();observed(&r,false);r.player.dead=false.into();
    r.player.x=7.;r.player.conditions.insert("underwater".into(),1.);observed(&r,false);
    let mut other=r.player.clone();other.id=987.;other.x=20.;other.conditions.clear();
    other.conditions.insert("halfHereClone".into(),1.);other.conditions.insert("halfHereOwner".into(),r.player.id);
    r.actors_at_mut(r.depth).push(other);let mut v=observed(&r,false);
    v.hover=Some((20,9));v.selected_actor=Some(987.);assert!(!v.deep_water,"hover/selection cannot change the viewpoint");
    let profile=Profile::default().set("terrain.floor.glyph",serde_json::json!("~")).unwrap();
    assert!(!crate::observe::observe_with_profile(&r,&profile).deep_water);
}

#[test]
fn water_depth_flight_phase_ascend_ring_and_dream_body_are_exempt() {
    let mut r=room();r.player.x=20.;observed(&r,true);
    r.player.conditions.insert("flying".into(),2.);observed(&r,false);
    r.player.conditions.clear();r.player.active.push("etill-phase-through".into());observed(&r,false);
    r.player.active.clear();r.player.abilities.push("anavani-ascend".into());observed(&r,false);
    r.player.abilities.clear();r.inventory.push(Item{id:789.,kind:"ring".into(),name:"Ring of Flight".into(),effect:"flight".to_owned().into(),..Default::default()});
    observed(&r,false);r.inventory.clear();observed(&r,true);
    let mut body=r.player.clone();body.id=987.;body.state="ally".into();body.name="Your sleeping body".into();
    body.conditions.insert("phantasmiiBody".into(),1.);body.conditions.insert("halfHereOwner".into(),r.player.id);
    r.actors_at_mut(r.depth).push(body);
    r.dreamwalking=corpse_core::dreamwalking::State{active:true,depth:r.depth,body_id:Some(987.),anchor_x:20.,anchor_y:9.,..Default::default()}.into();
    let dream=observed(&r,false);assert!(dream.dreamwalking,"wet sleeping body does not immerse its flying projection");
    r.dreamwalking=Field::Missing;observed(&r,true);
}

#[test]
fn water_depth_native_captures_separate_shallows_and_preserve_readable_tint() {
    let r=room();let frozen=serde_json::to_vec(&r).unwrap();
    let profile=Profile::default();
    // Native pre-change reconstruction at this room's 100 illumination:
    // zero-blood water used one shared illuminated anchor and neutral background.
    // The historical renderer had no immersion veil.
    // edgeColor is an imported material field, not an editable control path.
    // Exercise the supported complete-profile import rather than Profile::set.
    let mut legacy_json:serde_json::Value=serde_json::from_str(&profile.json()).unwrap();
    for tile in ["water","shallow-water"] {for (channel,color) in [("color","#39dfff"),("background","#182634"),("edgeColor","#a2bbca")] {
        legacy_json["terrain"][tile][channel]=serde_json::json!(color);
    }}
    let legacy=Profile::from_value(legacy_json).unwrap();
    let current=crate::observe::observe(&r);
    let shallow=current.cells.iter().take(current.terrain_count).find(|c|c.x==13&&c.y==9).unwrap();
    let deep=current.cells.iter().take(current.terrain_count).find(|c|c.x==20&&c.y==9).unwrap();
    assert!(crate::color_profile::luminance(shallow.bg)>crate::color_profile::luminance(deep.bg)*1.5);
    // Gamut-preserving brightness does not imply a larger red channel: shallow
    // water is lighter cyan-blue, while deep water is a darker saturated blue.
    assert!(crate::color_profile::luminance(shallow.fg)>crate::color_profile::luminance(deep.fg)*1.2,
        "light blue shallows versus rich blue depths: {:?} / {:?}",shallow.fg,deep.fg);
    assert!(shallow.fg[1] as u32 * deep.fg[2] as u32 > deep.fg[1] as u32 * shallow.fg[2] as u32);
    assert!(shallow.bg[2]>shallow.bg[1] && deep.bg[2]>deep.bg[1]);
    for (w,h) in [(1280,800),(1920,1080),(960,1200)] {
        let mut before=Painter::new();before.resize(w,h);before.profile=legacy.clone();
        let old=crate::observe::observe_with_profile(&r,&legacy);before.game_at(&old,"",false,731.);
        capture(&format!("{w}x{h}-before-depth-colors"),&before);
        let mut p=Painter::new();p.resize(w,h);p.game_at(&current,"",false,731.);
        capture(&format!("{w}x{h}-after-depth-colors"),&p);
        let mut wet=r.clone();wet.player.x=20.;let mut v=observed(&wet,true);
        v.deep_water=false;p.game_at(&v,"",false,731.);let dry_pixels=p.pixels.clone();
        capture(&format!("{w}x{h}-before-immersion-veil"),&p);
        v.deep_water=true;let veil=crate::illumination_ui::immersion_color(&v).unwrap();p.game_at(&v,"",false,731.);
        capture(&format!("{w}x{h}-after-immersion-veil"),&p);
        for (old,new) in dry_pixels.chunks_exact(4).zip(p.pixels.chunks_exact(4)) {
            for k in 0..3 {assert_eq!(new[k],((veil[k] as u32*34+old[k] as u32*221)/255)as u8);}
            // The modest veil retains over 86% of every foreground/background difference.
            assert!(old.iter().zip(new).take(3).all(|(&a,&b)|a.abs_diff(b)<=34));
        }
        let luminance=|c:[u8;3]|crate::color_profile::luminance(c);
        let tint=|c:[u8;3]|std::array::from_fn(|k|((veil[k] as u32*34+c[k] as u32*221)/255)as u8);
        let ink=tint([244,242,247]);let bg=tint([5,4,12]);
        assert!((luminance(ink)+0.05)/(luminance(bg)+0.05)>7.,"HUD text remains high contrast");
        let mut menu=Painter::new();menu.resize(w,h);menu.menu(0,0,true,"Normal");
        p.set_responsive_canvas(false);p.menu(0,0,true,"Normal");same(&p,&menu,"no gameplay tint leaks into menu");
    }
    assert_eq!(serde_json::to_vec(&r).unwrap(),frozen);
}

#[test]
fn water_depth_retained_and_tile_entry_exit_load_flight_and_animation_are_exact() {
    for (w,h) in [(1280,800),(1920,1080),(960,1200)] {
        let mut full=Painter::new();let mut retained=Painter::new();let mut tiled=Painter::new();
        full.resize(w,h);retained.resize(w,h);tiled.resize(w,h);
        let mut cache=RetainedGame::default();let mut plan=TilePlanner::default();let mut previous=None;
        let mut r=room();
        for step in 0..10 {
            let now=if step==3||step==4 {731.} else {0.};
            match step {
                1=>r.player.x=13.,2=>r.player.x=20.,
                5=>{r.player.conditions.insert("flying".into(),2.);},
                6=>r.player.conditions.clear(),7=>r.player.x=7.,
                8=>{r.player.x=20.;let raw=serde_json::to_string(&r).unwrap();r=serde_json::from_str(&raw).unwrap();},
                9=>r=room(),_=>{}
            }
            let next=crate::observe::observe(&r);let before=serde_json::to_vec(&r).unwrap();
            let damage=cache.update(&mut retained,&next,"",now,true,false).unwrap();
            plan.replace_view(previous.as_ref(),&next,&tiled);plan.mark_animation(&next,&tiled,now,false);plan.finish_damage(&mut tiled);
            if previous.as_ref().is_some_and(|v:&View|v.deep_water!=next.deep_water) {
                assert_eq!(damage.area(),w*h,"whole-screen tint transition must invalidate HUD as well as map");
                assert_eq!(tiled.tile_mask.area(),w*h);
            }
            plan.paint_base(&mut tiled,&next,"",now);tiled.tile_clip_enabled=false;
            full.game_at(&next,"",false,now);
            same(&retained,&full,&format!("retained {w}x{h} step {step}"));same(&tiled,&full,&format!("tile {w}x{h} step {step}"));
            assert_eq!(serde_json::to_vec(&r).unwrap(),before);
            plan.clear_frame();previous=Some(next);
        }
    }
}

#[test]
fn water_depth_gpu_records_only_one_clipped_veil_and_clears_on_exit() {
    let mut r=room();r.player.x=20.;let mut p=Painter::new();p.gpu=Some(crate::gpu_render::Frame::default());
    for (w,h) in [(1280,800),(1920,1080),(960,1200)] {
        p.resize(w,h);p.pixels.fill(17);
        for wet in [true,true,false] {
            r.player.x=if wet {20.}else{7.};let v=crate::observe::observe(&r);p.gpu.as_mut().unwrap().begin();
            p.game_at(&v,"",false,731.);
            let quads=&p.gpu.as_ref().unwrap().quads;
            let color=crate::illumination_ui::immersion_color(&v).unwrap_or([20,78,170]);
            let expected=[color[0],color[1],color[2],34];
            let veils:Vec<_>=quads.iter().filter(|q|q.color==expected).collect();
            assert_eq!(veils.len(),usize::from(wet));
            if wet {assert_eq!(veils[0].rect,(0,0,w as i32,h as i32));assert_eq!(quads.last().unwrap().color,expected);}
            assert!(p.pixels.iter().all(|&v|v==17));
        }
    }
}

#[test]
#[ignore="Isolated SDL native raster check: SDL_VIDEODRIVER=offscreen, --test-threads=1"]
fn water_depth_native_gpu_pixel_parity() {
    let sdl=sdl2::init().unwrap();let video=sdl.video().unwrap();let mut r=room();r.player.x=20.;
    let view=crate::observe::observe(&r);
    for (w,h) in [(1280,800),(1920,1080),(960,1200)] {
        let window=video.window("Water-depth native parity",w as u32,h as u32).hidden().build().unwrap();
        let (mut canvas,_)=crate::gpu_render::create_canvas(window,false).unwrap();let creator=canvas.texture_creator();
        let mut renderer=crate::gpu_render::Renderer::new(&creator,2048);
        let mut cpu=Painter::new();cpu.resize(w,h);let mut gpu=Painter::new();gpu.resize(w,h);gpu.gpu=Some(crate::gpu_render::Frame::default());
        cpu.game_at(&view,"",false,731.);gpu.game_at(&view,"",false,731.);
        renderer.draw(&mut canvas,gpu.gpu.as_ref().unwrap()).unwrap();
        let pixels=canvas.read_pixels(None,sdl2::pixels::PixelFormatEnum::RGBA32).unwrap();
        let max=pixels.chunks_exact(4).zip(cpu.pixels.chunks_exact(4)).flat_map(|(a,b)|(0..3).map(move|k|a[k].abs_diff(b[k]))).max().unwrap();
        eprintln!("Water native GPU {w}x{h}: max channel delta {max}");assert!(max<=8);
    }
}

#[test]
fn water_depth_real_engine_save_load_recomputes_and_does_not_persist_tint() {
    let mut engine=corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    engine.run.simulation_revision=51.0.into();
    engine.run.player.abilities.clear();engine.run.player.active.clear();engine.run.player.conditions.clear();
    engine.run.player.ability_order=Field::Missing;engine.run.player.equipment_abilities=Field::Missing;
    engine.run.inventory.clear();
    let at=engine.run.player.y as usize*engine.run.floor().unwrap().width()+engine.run.player.x as usize;
    engine.run.floor_mut().unwrap().tiles[at]="floor".into();
    let dry=engine.save().unwrap();
    engine.run.floor_mut().unwrap().tiles[at]="water".into();
    let wet=engine.save().unwrap();
    let mut painter=Painter::new();let mut full=Painter::new();let mut retained=RetainedGame::default();
    for (raw,submerged) in [(&dry,false),(&wet,true),(&wet,true),(&dry,false)] {
        let loaded=corpse_core::temporal::Engine::load(raw).unwrap();
        let before=loaded.save().unwrap();let v=observed(&loaded.run,submerged);
        retained.update(&mut painter,&v,"",0.,true,false).unwrap();full.game_at(&v,"",false,0.);
        same(&painter,&full,"actual Engine load must clear stale tint");
        assert_eq!(loaded.save().unwrap(),before);assert!(!before.contains("deep_water"));
    }
}
