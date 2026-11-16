//! R51 calibration through real observation and native draw paths.
use crate::{appearance_profile::Profile, color::light_ui::{DisplayLight, LEVELS}, render::{Painter, RetainedGame, View, tile_damage::TilePlanner}};
use corpse_core::{Field, Floor, FloorLightSourcesEntry, Run};
use std::sync::Arc;

const MATERIALS: [&str;5] = ["floor","wall","grass","water","shallow-water"];
fn room(illumination: f64, tile: &str, torch: bool) -> Run {
    let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    r.simulation_revision=51.0.into();r.player.x=8.;r.player.y=6.;
    r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
    r.player.ability_order=Field::Missing;r.player.equipment_abilities=Field::Missing;
    r.player.stats.perception=300.;r.inventory.clear();r.actors=Default::default();
    r.ground.clear();r.history.clear();r.log.clear();r.offers.clear();
    let mut tiles=vec!["floor".to_string();441];tiles[6*21+10]=tile.into();
    r.floors=vec![Arc::new(Floor{depth:r.depth,width:21.0.into(),height:21.0.into(),
        tiles:tiles.clone(),remembered_tiles:tiles.into(),lights:vec![if torch {0.}else{illumination};441],
        seen:vec![true;441],substrates:vec!["stone".into();441].into(),..Default::default()})];
    if torch {r.floor_mut().unwrap().light_sources=vec![FloorLightSourcesEntry{
        x:10.,y:4.,power:100.,falloff:10.,kind:"torch".to_owned().into()}].into();}
    r
}
fn center(v:&View)->&crate::render::Cell {v.cells.iter().take(v.terrain_count).find(|c|c.x==10&&c.y==6).unwrap()}
fn output()->Option<String>{std::env::var("CORPSE_R51_ILLUMINATION_OUTPUT").ok()}
fn png(name:&str,p:&Painter){if let Some(dir)=output(){std::fs::create_dir_all(&dir).unwrap();let file=std::fs::File::create(format!("{dir}/{name}.png")).unwrap();let mut png=png::Encoder::new(file,p.width as u32,p.height as u32);png.set_color(png::ColorType::Rgba);png.set_depth(png::BitDepth::Eight);png.write_header().unwrap().write_image_data(&p.pixels).unwrap();}}
fn crop(p:&Painter,v:&View)->(usize,usize,Vec<u8>){let(cw,ch,ox,oy)=p.map_layout(v.width,v.height);let r=p.logical_damage_rect(ox+9*cw,oy+5*ch,3*cw,3*ch).unwrap();(r.w,r.h,(r.y..r.bottom()).flat_map(|y|p.pixels[(y*p.width+r.x)*4..(y*p.width+r.right())*4].iter().copied()).collect())}
fn place(p:&mut Painter,c:&(usize,usize,Vec<u8>),x:i32,y:i32){let a=p.logical_damage_rect(x,y,1,1).unwrap();assert!(a.x+c.0<=p.width&&a.y+c.1<=p.height);for row in 0..c.1{let start=((a.y+row)*p.width+a.x)*4;p.pixels[start..start+c.0*4].copy_from_slice(&c.2[row*c.0*4..(row+1)*c.0*4]);}}
fn levels()->Vec<(f64,bool)>{LEVELS.into_iter().map(|l|(l,false)).chain([(80.,false),(80.,true)]).collect()}

#[test]
fn illumination_native_calibration_pixels_and_all_incremental_paths(){
    let mut sheet=Painter::new();sheet.resize(2200,1200);sheet.set_responsive_canvas(true);sheet.clear([15,16,22]);
    sheet.text("Illumination: native sRGB calibration",20,20,24,[239,229,211]);
    sheet.text("Same seed, tiles, position and animation time. Real R51 visibility; Perception 300 keeps every sample directly visible.",20,56,12,[180,184,195]);
    let mut rows=Vec::new();
    for (j,(level,torch)) in levels().iter().enumerate(){let x=122+j as i32*114;sheet.text(&format!("{level:+.0}"),x,88,17,[239,229,211]);sheet.text(if *torch{"torch"}else{"neutral"},x,112,10,[180,184,195]);}
    for (row,tile) in MATERIALS.iter().enumerate(){
        let mut full=Painter::new();full.resize(1280,800);
        let mut retained_p=Painter::new();retained_p.resize(1280,800);let mut retained=RetainedGame::default();
        let mut tiled=Painter::new();tiled.resize(1280,800);let mut planner=TilePlanner::default();let mut previous=None;
        let baseline=crate::observe::observe(&room(0.,tile,false));let base=center(&baseline).clone();
        let y=144+row as i32*115;sheet.text(tile,12,y+28,12,[210,214,225]);
        let mut sampled=Vec::new();
        for (column,(level,torch)) in levels().into_iter().enumerate(){
            let run=room(level,tile,torch);let frozen=serde_json::to_vec(&run).unwrap();let rng=run.rng;
            let view=crate::observe::observe(&run);let sample=center(&view);let o=&view.terrain_snapshot.as_ref().unwrap().cells[6*21+10];
            assert!(o.current&&o.direct&&o.direct_chance==100.,"{tile}/{level}: visibility held constant, {o:?}");
            assert_eq!(o.illumination,Some(level));
            if !torch {assert_eq!(sample.bg,DisplayLight::neutral(level).apply(base.bg),"{tile}/{level}: exact displayed background calibration");assert_eq!(sample.fg,expected_foreground(level,base.fg,sample.bg),"{tile}/{level}: glyph calibration");}
            full.game_at(&view,"",false,0.);
            retained.update(&mut retained_p,&view,"",0.,true,previous.is_none()).unwrap();assert_eq!(retained_p.pixels,full.pixels,"{tile}/{level}: retained pixels");
            planner.replace_view(previous.as_ref(),&view,&tiled);planner.finish_damage(&mut tiled);planner.paint_base(&mut tiled,&view,"",0.);tiled.tile_clip_enabled=false;planner.clear_frame();assert_eq!(tiled.pixels,full.pixels,"{tile}/{level}: tile pixels");
            let image=crop(&full,&view);place(&mut sheet,&image,122+column as i32*114,y);
            rows.push(serde_json::json!({"terrain":tile,"illumination":o.illumination,"ambient":o.ambient,"torch":torch,"foreground":sample.fg,"background":sample.bg,"zeroBackground":base.bg,"directChance":o.direct_chance,"seed":run.seed,"position":[run.player.x,run.player.y],"presentationMs":0,"nativeFrame":[1280,800],"crop":[image.0,image.1],"cropRgbaCrc32":crc32fast::hash(&image.2),"fullRetainedTileEqual":true}));
            png(&format!("{tile}-{}{}",level as i32,if torch{"-torch"}else{""}),&full);
            sampled.push(sample.bg);
            assert_eq!(serde_json::to_vec(&run).unwrap(),frozen);assert_eq!(run.rng,rng);previous=Some(view);
        }
        assert_ne!(sampled[4],sampled[6]);assert_ne!(sampled[6],sampled[8]);
        let neutral=crate::color::luminance(sampled[9]);let warm=crate::color::luminance(sampled[10]);
        assert!((neutral-warm).abs()<0.004,"{tile}: equal scalar light retains perceived brightness: {neutral}/{warm}");
    }
    sheet.text("Columns use copied native 3x3 map crops, with no recoloring or resizing. Neutral 80 is included beside torch 80 for comparison.",20,746,11,[180,184,195]);
    png("illumination-levels-native",&sheet);
    if let Some(dir)=output(){std::fs::write(format!("{dir}/native-calibration-evidence.json"),serde_json::to_vec_pretty(&rows).unwrap()).unwrap();}
}

#[test]
fn illumination_entities_effects_custom_profiles_and_unbounded_observations(){
    let p=Profile::default();let source=room(0.,"floor",false);let before=serde_json::to_vec(&source).unwrap();
    let observation=corpse_core::vision::observe_terrain(&source).unwrap();let at=6*21+10;
    for level in [-150.,-100.,-50.,0.,50.,100.,150.,250.] {
        let mut o=observation.clone();for c in &mut o.cells{c.illumination=Some(level);c.ambient=Some(level);}
        let frozen=format!("{o:?}");let light=crate::appearance_light::light_field(&o,&p,0.);
        assert_eq!(light[at].illumination,level,"never clamp observation values");
        for label in ["creature","item","Fire","gas"] {
            let mut cell=crate::render::Cell{x:10,y:6,fg:[80,120,60],bg:[34,28,32],label:label.into(),direct:true,current:true,
                animations:vec![crate::render::Animation::Gas{phase:0.,color:[21,61,28]},crate::render::Animation::Fire{phase:0.,speed:1.,colors:[[50,20,10];4]}],..Default::default()};
            let mut zero=cell.clone();crate::illumination_ui::finish_cell(&mut zero,crate::color::Light{display:Some(DisplayLight::neutral(0.)),..crate::color::Light::ambient(0.)},&p);
            crate::illumination_ui::finish_cell(&mut cell,light[at],&p);
            assert_eq!(cell.bg,DisplayLight::neutral(level).apply(zero.bg));
            assert_eq!(cell.fg,expected_foreground(level,zero.fg,cell.bg));
            let crate::render::Animation::Gas{color,..}=cell.animations[0] else{panic!()};assert_eq!(color,DisplayLight::neutral(level).apply([21,61,28]));
        }
        assert_eq!(format!("{o:?}"),frozen);
    }
    for tile in MATERIALS {let custom=p.set(&format!("terrain.{tile}.background"),serde_json::json!("#27412f")).unwrap();
        let zero=crate::observe::observe_with_profile(&room(0.,tile,false),&custom);let bright=crate::observe::observe_with_profile(&room(100.,tile,false),&custom);
        assert_eq!(center(&bright).bg,DisplayLight::neutral(100.).apply(center(&zero).bg));
    }
    assert_eq!(serde_json::to_vec(&source).unwrap(),before);
}

#[test]
#[ignore="Explicit isolated SDL process: SDL_VIDEODRIVER=offscreen, --test-threads=1; Linux software GL is not Windows hardware certification"]
fn illumination_native_gpu_matches_software_at_every_calibration_level(){
    let sdl=sdl2::init().unwrap();let video=sdl.video().unwrap();let window=video.window("Illumination GPU calibration",1280,800).hidden().build().unwrap();
    let(mut canvas,accelerated)=crate::gpu_render::create_canvas(window,false).unwrap();let creator=canvas.texture_creator();let mut renderer=crate::gpu_render::Renderer::new(&creator,2048);
    let mut cpu=Painter::new();let mut gpu=Painter::new();gpu.gpu=Some(crate::gpu_render::Frame::default());let mut evidence=Vec::new();
    for tile in MATERIALS {for (level,torch) in levels(){
        let run=room(level,tile,torch);let frozen=serde_json::to_vec(&run).unwrap();let view=crate::observe::observe(&run);
        cpu.game_at(&view,"",false,0.);gpu.gpu.as_mut().unwrap().begin();gpu.game_at(&view,"",false,0.);renderer.draw(&mut canvas,gpu.gpu.as_ref().unwrap()).unwrap();
        let actual=canvas.read_pixels(None,sdl2::pixels::PixelFormatEnum::RGBA32).unwrap();let mut maximum=0;let mut changed=0usize;
        for (a,b) in actual.chunks_exact(4).zip(cpu.pixels.chunks_exact(4)){for k in 0..3{let delta=a[k].abs_diff(b[k]);maximum=maximum.max(delta);changed+=usize::from(delta>0);}}
        assert!(maximum<=3,"{tile}/{level}/torch={torch}: GPU rounding delta {maximum}");
        evidence.push(serde_json::json!({"terrain":tile,"illumination":level,"torch":torch,"renderer":canvas.info().name,"acceleratedApi":accelerated,"maxChannelDelta":maximum,"differentChannels":changed}));
        assert_eq!(serde_json::to_vec(&run).unwrap(),frozen);
    }}
    if let Some(dir)=output(){std::fs::create_dir_all(&dir).unwrap();std::fs::write(format!("{dir}/gpu-calibration-evidence.json"),serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();}
}

#[test]
fn illumination_actual_creature_item_player_and_backgrounds_are_graded_once(){
    let mut results=Vec::new();
    for light in [-100.,0.,50.,100.] {
        let mut run=room(light,"floor",false);
        run.actors_at_mut(run.depth).push(corpse_core::population::create_actor_revision("bear",900.,10.,6.,false,51.).unwrap());
        run.ground.insert(run.depth.to_string(),Arc::new(vec![corpse_core::GroundItem{x:11.,y:6.,
            item:corpse_core::Item{id:901.,kind:"potion".into(),glyph:"!".into(),name:"Potion".into(),..Default::default()},..Default::default()}]));
        let frozen=serde_json::to_vec(&run).unwrap();let view=crate::observe::observe(&run);
        let actor=view.cells.iter().find(|c|c.actor_id==Some(900.)).unwrap();
        let item=view.cells.iter().find(|c|c.x==11&&c.y==6&&c.entity_ink.is_some()).unwrap();
        let player=view.cells.iter().find(|c|c.actor_id==Some(run.player.id)).unwrap();
        for cell in [actor,item,player]{assert!(cell.display_light.is_some(),"{} was not calibrated",cell.label);
            let ground=view.cells[..view.terrain_count].iter().find(|c|c.x==cell.x&&c.y==cell.y).unwrap();assert_eq!(cell.bg,ground.bg);
        }
        results.push((actor.fg,item.fg,player.fg));assert_eq!(serde_json::to_vec(&run).unwrap(),frozen);
    }
    assert_ne!(results[0],results[1]);assert_ne!(results[1].0,results[2].0);
    // Saturated bright glyphs may reach their gamut, but darkness always responds.
    assert_ne!(results[0].1,results[1].1);assert_ne!(results[0].2,results[1].2);
}

#[test]
fn illumination_unknown_and_telepathic_cells_never_reveal_hidden_light(){
    let run=room(0.,"floor",false);let p=Profile::default();let mut o=corpse_core::vision::observe_terrain(&run).unwrap();let at=6*21+10;
    for knowledge in ["remembered","layout","telepathic"] {
        let mut samples=Vec::new();
        for level in [-150.,0.,500.] {
            let c=&mut o.cells[at];c.direct=false;c.current=knowledge=="telepathic";c.knowledge=knowledge.into();c.illumination=Some(level);c.ambient=Some(level);
            let (cells,_,lights)=crate::observe::terrain_frame(&o,&p,17,0.);assert!(lights[at].display.is_none());
            let cell=cells.iter().find(|c|c.x==10&&c.y==6).unwrap();samples.push((cell.fg,cell.bg));
        }
        assert!(samples.iter().all(|c|*c==samples[0]),"hidden {knowledge} illumination changes must not change pixels");
    }
}

#[test]
fn illumination_custom_source_controls_are_opt_in_and_neutral_flicker_is_visible(){
    let p=Profile::default();let run=room(80.,"floor",true);let o=corpse_core::vision::observe_terrain(&run).unwrap();let at=6*21+10;
    let default=crate::appearance_light::light_field(&o,&p,0.);
    assert_eq!(default[at].illumination,80.);
    assert_eq!(default[at].display.unwrap().multiplier,DisplayLight::neutral(80.).multiplier);
    let disabled=p.set("lighting.sources.torch.intensity",serde_json::json!(0.)).unwrap();
    let dim=crate::appearance_light::light_field(&o,&disabled,0.);
    assert_eq!(dim[at].illumination,80.,"an appearance override never changes gameplay light");
    assert_eq!(dim[at].display.unwrap().multiplier,1.,"zero source intensity restores neutral ambient exposure");
    let edited=p.set("lighting.sources.torch.color",serde_json::json!("#ffffff")).unwrap()
        .set("lighting.sources.torch.flickerStrength",serde_json::json!(0.8)).unwrap();
    let first=crate::appearance_light::light_field(&o,&edited,0.);
    let later=crate::appearance_light::light_field(&o,&edited,1000.);
    assert_ne!(first[at].display.unwrap().multiplier,later[at].display.unwrap().multiplier);
    assert_eq!(first[at].display.unwrap().tint,[1.;3]);
    let zero=corpse_core::vision::observe_terrain(&room(0.,"floor",false)).unwrap();
    for time in [0.,1000.] {let fields=crate::appearance_light::light_field(&zero,&p,time);assert_eq!(fields[at].display,Some(DisplayLight::neutral(0.)));}
}

#[test]
fn illumination_transient_effects_use_the_same_detached_grading(){
    let mut results=Vec::new();
    for level in [-100.,0.,100.] {
        let run=room(level,"floor",false);let frozen=serde_json::to_vec(&run).unwrap();let view=crate::observe::observe(&run);
        let event:corpse_core::events::PresentationEvent=serde_json::from_value(serde_json::json!({"kind":"status","depth":run.depth,"to":{"x":10,"y":6},"color":"#c86428","glyph":"*"})).unwrap();
        let cue=crate::effects::Cue{event,started:0.,held:false,static_marks:false};
        let tiles=crate::effects::tiles(&cue,20.,false);assert!(!tiles.is_empty());
        let mut p=Painter::new();p.gpu=Some(crate::gpu_render::Frame::default());p.effects(&view,&[cue.clone()],20.,false);
        let expected=DisplayLight::neutral(level).foreground([200,100,40],false);
        let expected_field=DisplayLight::neutral(level).apply([64,32,13]);
        assert!(p.gpu.as_ref().unwrap().quads.iter().any(|q|q.color[..3]==expected_field),"effect background must use background exposure at {level}");
        assert!(tiles.iter().all(|t|t.glyph.is_none()),"local feedback must be light only");
        let mut projectile=cue.clone();projectile.event.kind="projectile".into();projectile.event.from=Some(corpse_core::events::EventPoint{x:9.,y:6.});
        let mut bolt=Painter::new();bolt.gpu=Some(crate::gpu_render::Frame::default());bolt.effects(&view,&[projectile],150.,false);
        assert!(bolt.gpu.as_ref().unwrap().quads.iter().any(|q|q.color[..3]==expected),"traveling glyph must use the scene's display calibration at {level}");
        let mut cpu=Painter::new();cpu.game_at(&view,"",false,0.);cpu.effects(&view,&[cue],20.,false);results.push(crop(&cpu,&view).2);
        assert_eq!(serde_json::to_vec(&run).unwrap(),frozen);
    }
    assert_ne!(results[0],results[1]);assert_ne!(results[1],results[2]);
}

#[test]
fn illumination_water_depth_hues_and_luminance_remain_distinct_at_all_knots(){
    for level in LEVELS {
        let deep=crate::observe::observe(&room(level,"water",false));let shallow=crate::observe::observe(&room(level,"shallow-water",false));
        let a=center(&deep);let b=center(&shallow);
        assert!(crate::color::luminance(b.bg)>crate::color::luminance(a.bg),"{level}: shallow background remains lighter");
        assert!(crate::color::luminance(b.fg)>crate::color::luminance(a.fg),"{level}: shallow foreground remains lighter");
        for c in [a,b] {// At .05 background exposure, 10% retained saturation may quantize
            // to neutral 8-bit RGB. Foreground contrast gain must retain blue.
            assert!(c.bg[2]>=c.bg[0]&&c.fg[2]>c.fg[0],"{level}: water keeps blue hue, allowing neutral near-black background quantization");
            if level>=0. {assert!(c.bg[2]>c.bg[0]);}}
        assert_ne!(a.bg,b.bg);assert_ne!(a.fg,b.fg);
    }
}

#[test]
fn illumination_deep_water_veil_keeps_darkness_and_invalidates_the_whole_frame(){
    let mut retained=RetainedGame::default();let mut cached=Painter::new();let mut full=Painter::new();let mut tiled=Painter::new();let mut plan=TilePlanner::default();let mut previous=None;
    for level in [0.,-100.,100.,-100.,0.] {
        let mut run=room(level,"water",false);run.player.x=10.;run.player.y=6.;
        let frozen=serde_json::to_vec(&run).unwrap();let view=crate::observe::observe(&run);assert!(view.deep_water);
        let veil=crate::illumination_ui::immersion_color(&view).unwrap();assert_eq!(veil,DisplayLight::neutral(level).apply([20,78,170]));
        if level == -100. {assert_eq!(veil,[3,4,4]);}
        let damage=retained.update(&mut cached,&view,"",0.,true,false).unwrap();assert_eq!(damage.area(),cached.width*cached.height);
        plan.replace_view(previous.as_ref(),&view,&tiled);plan.finish_damage(&mut tiled);assert_eq!(tiled.tile_mask.area(),tiled.width*tiled.height);
        plan.paint_base(&mut tiled,&view,"",0.);tiled.tile_clip_enabled=false;plan.clear_frame();full.game_at(&view,"",false,0.);
        assert_eq!(cached.pixels,full.pixels);assert_eq!(tiled.pixels,full.pixels);
        let(cw,ch,ox,oy)=full.map_layout(view.width,view.height);let pixel=full.logical_damage_rect(ox+10*cw,oy+6*ch,1,1).unwrap();let at=(pixel.y*full.width+pixel.x)*4;
        if level == -100. {assert!(full.pixels[at..at+3].iter().all(|v|*v<=5),"underwater darkness may not be relit by the blue veil");}
        png(&format!("underwater-{}",level as i32),&full);
        assert_eq!(serde_json::to_vec(&run).unwrap(),frozen);previous=Some(view);
    }
}

fn expected_foreground(level:f64,base:crate::render::Rgb,bg:crate::render::Rgb)->crate::render::Rgb {
    let fg=DisplayLight::neutral(level).foreground(base,false);
    if level<0. {let t=(-level/25.).clamp(0.,1.);crate::color::composite(fg,crate::color::light_ui::foreground_contrast(fg,bg,3.),t*t*(3.-2.*t))}else{fg}
}

#[test]
fn negative_light_keeps_admitted_creatures_readable_without_changing_knowledge() {
    let profile=Profile::default();
    for level in [-100.,-75.,-50.,-25.] {
        let mut run=room(level,"floor",false);
        let actor=corpse_core::population::create_actor_revision("rat",900.,9.,6.,false,51.).unwrap();
        run.actors_at_mut(run.depth).push(actor);
        let before=serde_json::to_vec(&run).unwrap();let view=crate::observe::observe(&run);
        assert!(corpse_core::vision::visible_actor(&run,&run.actors_at(run.depth)[0]));
        let c=view.cells.iter().find(|c|c.actor_id==Some(900.)).unwrap();
        assert!(crate::color::contrast(c.fg,c.bg)>=4.45,"{level}: {:?}/{:?}",c.fg,c.bg);
        let terrain=center(&view);assert!(crate::color::contrast(terrain.fg,terrain.bg)>=2.95);
        let mut p=Painter::new();p.game_at(&view,"",false,0.);png(&format!("negative-{level}-visible-rat"),&p);
        assert_eq!(serde_json::to_vec(&run).unwrap(),before);
    }
    assert_eq!(crate::illumination_ui::unexplored(&profile),[0;3]);
}

#[test]
fn chasm_uses_static_brogue_ink_on_black_and_memory_has_no_live_updates() {
    let mut run=room(0.,"chasm",false);let before=serde_json::to_vec(&run).unwrap();
    let view=crate::observe::observe(&run);let c=center(&view);
    assert_eq!(c.bg,[0;3]);assert!((18..=28).contains(&c.fg[0]));assert!((38..=59).contains(&c.fg[2]));
    let mut p=Painter::new();p.game_at(&view,"",false,0.);png("brogue-chasm-native",&p);
    p.game_at(&view,"",false,713.);assert_eq!(serde_json::to_vec(&run).unwrap(),before);
    run.player.stats.perception=0.;run.player.x=1.;run.player.y=1.;
    for y in 0..21 {run.floor_mut().unwrap().tiles[y*21+5]="wall".into();}
    let memory=crate::observe::observe(&run);let old=center(&memory).clone();
    assert!(!old.current);
    assert_eq!(crate::render::readable_cell_colors(&old), (crate::color::memory(c.fg), [0;3]),
        "remembered chasm keeps authored dim ink, not the ordinary memory readability floor");
    let at=6*21+10;run.floor_mut().unwrap().tiles[at]="grass".into();run.floor_mut().unwrap().lights[at]=100.;
    let actor=corpse_core::population::create_actor_revision("rat",901.,10.,6.,false,51.).unwrap();run.actors_at_mut(run.depth).push(actor);
    let hidden=crate::observe::observe(&run);let latest=center(&hidden);
    assert_eq!((old.glyph,old.fg,old.bg),(latest.glyph.clone(),latest.fg,latest.bg));
    assert!(!hidden.cells.iter().any(|c|c.actor_id==Some(901.)));
}

#[test]
fn remembered_decorations_are_neutral_and_chasm_custom_grading_survives() {
    let profile=Profile::default();
    let mut cell=crate::render::Cell{label:"water".into(),fg:[25,40,60],bg:[2,2,3],
        edges:[Some([25,40,60]),None,None,None],underlays:vec![("~".into(),[25,40,60],0.4)],
        background_overlay:Some(([30,80,100],0.2)),..Default::default()};
    crate::illumination_ui::finish_cell(&mut cell,crate::color::Light::ambient(-100.),&profile);
    for color in [cell.edges[0].unwrap(),cell.underlays[0].1,cell.background_overlay.unwrap().0] {
        assert_eq!(color[0],color[1]);assert_eq!(color[1],color[2]);
    }
    let original=cell.clone();crate::illumination_ui::finish_cell(&mut cell,crate::color::Light::ambient(100.),&profile);assert_eq!(cell,original);
    let run=room(0.,"chasm",false);
    for (key,value) in [("global.exposure",serde_json::json!(0.5)),("terrain.chasm.opacity",serde_json::json!(0.0))] {
        let custom=profile.set(key,value).unwrap();let view=crate::observe::observe_with_profile(&run,&custom);
        let default=crate::observe::observe(&run);assert_ne!(center(&view).fg,center(&default).fg,"{key} must not be overwritten by raw Brogue ink");
    }
}
