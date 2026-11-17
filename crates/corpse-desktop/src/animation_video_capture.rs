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
