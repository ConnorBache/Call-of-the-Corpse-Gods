//! Opt-in review videos from real core receipt fixtures, using production paint paths.
use super::*;
use corpse_core::Run;
use std::{io::Write,process::{Command as Process,Stdio}};
#[test]
#[ignore = "explicit review capture; requires ffmpeg and exported actual core fixtures"]
fn render_animation_review_videos() {
 let input=std::env::var("CORPSE_ANIMATION_EXPORT_DIR").expect("fixture directory");
 let output=std::env::var("CORPSE_ANIMATION_VIDEO_DIR").expect("output directory");
 std::fs::create_dir_all(&output).unwrap();
 let mut paths=std::fs::read_dir(&input).unwrap().map(|e|e.unwrap().path()).filter(|p|p.extension().is_some_and(|e|e=="json")).collect::<Vec<_>>();paths.sort();assert!(!paths.is_empty());
 let mut index=Vec::new();
 for path in paths {
  let value:serde_json::Value=serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
  let before:Run=serde_json::from_value(value["before"].clone()).unwrap();
  let after:Run=serde_json::from_value(value["after"].clone()).unwrap();
  let events:Vec<corpse_core::events::PresentationEvent>=serde_json::from_value(value["events"].clone()).unwrap();
  let name=path.file_stem().unwrap().to_str().unwrap();
  let mut fx=effects::Effects::default();fx.add(events,after.tick,after.depth,500.,false);
  let frames=if let Some(frames)=value["frames"].as_array().filter(|f|!f.is_empty()){frames.iter().map(|f|corpse_core::events::PresentationFrame{action:f["action"].as_u64().unwrap()as u32,observation:std::rc::Rc::new(serde_json::from_value::<Run>(f["run"].clone()).unwrap())}).collect()}else{vec![corpse_core::events::PresentationFrame{action:fx.cues.iter().filter_map(|c|c.event.animation.as_ref().map(|a|a.action)).max().unwrap_or(0),observation:std::rc::Rc::new(after.clone())}]};
  let mut painter=Painter::new();painter.resize(960,600);painter.set_responsive_canvas(true);
  let pre=observe::observe_with_profile(&before,&painter.profile);let post=observe::observe_with_profile(&after,&painter.profile);
  fx.sequence.capture(pre,frames,&fx.cues,&painter.profile);
  if let Some(inward)=fx.cues.iter().find(|c|c.event.animation.as_ref()
      .is_some_and(|a|a.style==corpse_core::animation_shapes::BECKON_BEAM)) {
   let receipt=inward.event.animation.as_ref().unwrap();
   if let Some(actor)=receipt.hide_actor {
    let outbound=fx.cues.iter().find(|c|c.event.animation.as_ref()
        .is_some_and(|a|a.style==corpse_core::animation_shapes::BECKON_OUTBOUND&&a.action==receipt.action)).unwrap();
    let location=|now|fx.sequence.view(now).unwrap_or(&post).cells.iter()
        .find(|c|c.actor_id==Some(actor)).map(|c|(c.x,c.y));
    let origin=receipt.cells.first().unwrap();
    assert_eq!(location(outbound.started),Some((origin.x as usize,origin.y as usize)));
    assert_eq!(location(inward.started-1.),location(outbound.started));
    assert_eq!(location(inward.started),None);
    let lands=inward.started+effects::duration(&inward.event);
    assert_eq!(location(lands-1.),None);
    assert!(location(lands).is_some(),"Beckoned actor must reappear at beam completion");
    assert_ne!(location(lands),location(outbound.started));
   }
  }
  let end=fx.cues.iter().map(|c|c.started+effects::duration(&c.event)).fold(500f64,f64::max);
  let duration=(end+700.).max(2000.);let count=(duration*60./1000.).ceil()as usize;
  let destination=format!("{output}/{name}.mp4");
  let mut encoder=Process::new("ffmpeg").args(["-hide_banner","-loglevel","error","-y","-f","rawvideo","-pix_fmt","rgba","-s","960x600","-r","60","-i","-","-an","-c:v","libx264","-preset","fast","-crf","19","-pix_fmt","yuv420p","-movflags","+faststart",&destination]).stdin(Stdio::piped()).spawn().unwrap();
  let mut base_id=0usize;let mut base_pixels=Vec::new();let mut stdin=encoder.stdin.take().unwrap();let mut hashes=std::collections::BTreeSet::new();
  for frame in 0..count {let now=frame as f64*1000./60.;let view=fx.sequence.view(now).unwrap_or(&post);let view_id=view as *const _ as usize;if view_id!=base_id {painter.game_at(view,"",false,0.);base_pixels=painter.pixels.clone();base_id=view_id;}else{painter.pixels.copy_from_slice(&base_pixels);}painter.effects(view,&fx.cues,now,false);effects::paint_inventory_hud(&mut painter,view,&fx.cues,now,false);hashes.insert(crc32fast::hash(&painter.pixels));stdin.write_all(&painter.pixels).unwrap();if frame==count/2 {let f=std::fs::File::create(format!("{output}/{name}-middle.png")).unwrap();let mut enc=png::Encoder::new(f,960,600);enc.set_color(png::ColorType::Rgba);enc.set_depth(png::BitDepth::Eight);enc.write_header().unwrap().write_image_data(&painter.pixels).unwrap();}}
  drop(stdin);assert!(encoder.wait().unwrap().success());assert!(hashes.len()>1,"no visual change: {name}");
  index.push(serde_json::json!({"name":name,"frames":count,"duration_seconds":count as f64/60.,"distinct_raster_frames":hashes.len(),"receipt_count":fx.cues.len(),"source":"real core trigger fixture; production native Painter and presentation sequence"}));
 }
 std::fs::write(format!("{output}/index.json"),serde_json::to_vec_pretty(&index).unwrap()).unwrap();
}

fn razorleaves_scene() -> Run {
    let mut run=crate::non_euclidean_ui::tests::scene();
    run.simulation_revision=52.0.into();
    run.player.x=9.;
    run.player.y=10.;
    run.player.stats.perception=100.;
    let floor=run.floor_mut().unwrap();
    floor.non_euclidean=corpse_core::Field::Missing;
    floor.godbeast_event=corpse_core::Field::Missing;
    floor.light_sources=corpse_core::Field::Missing;
    let samara=corpse_core::population::create_actor_revision("samara",901.,13.,10.,false,52.).unwrap();
    run.actors_at_mut(run.depth).push(samara);
    corpse_core::environment::refresh_razorleaves(&mut run,&[901.]);
    run
}

#[test]
fn razorleaves_animate_only_observed_tiles_without_mutating_gameplay() {
    let run=razorleaves_scene();let before=run.clone();
    let view=observe::observe(&run);
    let visible:Vec<_>=view.cells.iter().filter(|c|c.animations.iter()
        .any(|a|matches!(a,crate::render::Animation::Razorleaves{..}))).collect();
    assert!(visible.len()>50);
    assert!(visible.iter().any(|c|(c.x,c.y)==(9,10)),"effect remains over occupied tiles");
    assert!(visible.iter().all(|c|c.current&&(c.x as i32-13).pow(2)+(c.y as i32-10).pow(2)<=25));
    let mut painter=Painter::new();
    painter.game_at(&view,"",false,130.);
    let first=crc32fast::hash(&painter.pixels);
    painter.game_at(&view,"",false,730.);
    assert_ne!(first,crc32fast::hash(&painter.pixels));
    painter.game_at(&view,"",false,130.);
    assert_eq!(first,crc32fast::hash(&painter.pixels));
    assert_eq!(before,run);
}

#[test]
#[ignore = "explicit native gameplay capture; requires ffmpeg"]
fn capture_razorleaves() {
    let output=std::env::var("CORPSE_RAZORLEAVES_VIDEO").expect("output path");
    let run=razorleaves_scene();
    let mut painter=Painter::new();
    painter.resize(1280,800);
    painter.set_responsive_canvas(true);
    let view=observe::observe_with_profile(&run,&painter.profile);
    let (w,h,ox,oy)=painter.map_layout(view.width,view.height);
    let crop=format!("crop={}:{}:{}:{},scale=720:-2:flags=neighbor",w*14,h*13,ox+w*6,oy+h*4);
    let mut encoder=Process::new("ffmpeg").args(["-hide_banner","-loglevel","error","-y",
        "-f","rawvideo","-pix_fmt","rgba","-s","1280x800","-r","30","-i","-",
        "-vf",&crop,"-an","-c:v","libx264","-crf","17","-pix_fmt","yuv420p",
        "-movflags","+faststart",&output]).stdin(Stdio::piped()).spawn().unwrap();
    let mut sink=encoder.stdin.take().unwrap();
    for frame in 0..120 {
        painter.game_at(&view,"Falling razorleaves fill the air.",false,frame as f64*1000./30.);
        sink.write_all(&painter.pixels).unwrap();
    }
    drop(sink);assert!(encoder.wait().unwrap().success());
}
