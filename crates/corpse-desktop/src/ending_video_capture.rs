//! Opt-in capture of genuine terminal outcomes through the SDL event loop.
//! The two saves prepare short review scenes, but never prescribe their outcome.
use super::*;
use sdl2::{render::Canvas, video::Window};
use std::{cell::RefCell, io::Write, path::Path, process::{Child, Command as Process, Stdio}};

struct Capture {
    destination: std::path::PathBuf,
    expected: &'static str,
    action: Keycode,
    continued: bool,
    action_sent: bool,
    space_sent: bool,
    started: Instant,
    game_at: Option<Instant>,
    prompt_at: Option<Instant>,
    summary_at: Option<Instant>,
    next_frame: f64,
    encoder: Option<Child>,
    previous_corner: Option<u8>,
    changing_frames: usize,
}
thread_local! { static CAPTURE: RefCell<Option<Capture>> = const { RefCell::new(None) }; }

pub fn inject(sdl: &sdl2::Sdl, canvas: &Canvas<Window>) -> Result<(), String> {
    CAPTURE.with(|capture| {
        let mut capture=capture.borrow_mut();
        let Some(c)=capture.as_mut() else{return Ok(());};
        if c.started.elapsed().as_secs() > 12 {return Err(format!("{} capture timed out (continued={}, action={}, prompt={})",c.expected,c.continued,c.action_sent,c.prompt_at.is_some()));}
        let key = if !c.continued { c.continued=true; Keycode::C }
            else if !c.action_sent && c.game_at.is_some_and(|at| at.elapsed().as_millis() >= 550) {
                c.action_sent=true; c.action
            } else if !c.space_sent && c.prompt_at.is_some_and(|at| at.elapsed().as_millis() >= 650) {
                c.space_sent=true; Keycode::Space
            } else {return Ok(());};
        sdl.event()?.push_event(Event::KeyDown {timestamp:0,window_id:canvas.window().id(),
            keycode:Some(key),scancode:None,keymod:Mod::NOMOD,repeat:false})
    })
}

pub fn observe(_: &mut Canvas<Window>, p: &Painter, screen: Screen, condition: &str, prompt: bool) -> Result<bool, String> {
    CAPTURE.with(|capture| {
        let mut capture=capture.borrow_mut();
        let Some(c)=capture.as_mut() else{return Ok(false);};
        if screen==Screen::Game {c.game_at.get_or_insert_with(Instant::now);}
        let Some(game_at)=c.game_at else{return Ok(false);};
        let elapsed=game_at.elapsed().as_secs_f64();
        if c.encoder.is_none() {
            let size=format!("{}x{}",p.width,p.height);
            c.encoder=Some(Process::new("ffmpeg").args(["-hide_banner","-loglevel","error","-y",
                "-f","rawvideo","-pix_fmt","rgba","-s",&size,"-r","30","-i","-","-an",
                "-c:v","libx264","-threads","2","-crf","18","-pix_fmt","yuv420p","-movflags","+faststart"])
                .arg(&c.destination).stdin(Stdio::piped()).spawn().map_err(|e|e.to_string())?);
        }
        if condition==c.expected && prompt {
            c.prompt_at.get_or_insert_with(Instant::now);
            let expected_ink=if condition=="won" {255} else {0};
            assert_eq!(&p.pixels[..4], &[expected_ink,expected_ink,expected_ink,255]);
            assert_eq!(&p.pixels[p.pixels.len()-4..], &[expected_ink,expected_ink,expected_ink,255]);
        }
        if elapsed >= c.next_frame {
            let corner=p.pixels[0];
            if condition==c.expected && !prompt && c.previous_corner.is_some_and(|prev| prev!=corner) {c.changing_frames+=1;}
            c.previous_corner=Some(corner);
            let out=c.encoder.as_mut().unwrap().stdin.as_mut().unwrap();
            while elapsed >= c.next_frame {
                out.write_all(&p.pixels).map_err(|e|e.to_string())?;
                c.next_frame+=1./30.;
            }
        }
        if screen==Screen::Summary {
            if !c.space_sent {return Err("Summary appeared without a fresh Space press".into());}
            let at=c.summary_at.get_or_insert_with(Instant::now);
            if at.elapsed().as_millis()>=800 {
                let mut encoder=c.encoder.take().unwrap();
                drop(encoder.stdin.take());
                if !encoder.wait().map_err(|e|e.to_string())?.success(){return Err("ffmpeg failed".into());}
                if c.changing_frames<3{return Err(format!("{} did not visibly animate",c.expected));}
                return Ok(true);
            }
        }
        Ok(false)
    })
}

fn capture(directory:&Path, escape:bool) {
    let stem=if escape {"escape-fade-to-white"} else {"death-fade-to-black"};
    let scene=directory.join(format!("{stem}-scratch"));
    std::fs::create_dir_all(&scene).unwrap();
    let mut run=corpse_core::startup::create_avatar_run("0-1-2:ending-fade-review").unwrap();
    let action=if escape {
        // Review scene: place the Avatar one ordinary step from the real
        // surface exit, after the Administrator has already been defeated.
        let floor=run.floor().unwrap();
        let door=corpse_core::entrance::cell(floor).unwrap();
        let w=floor.width();
        let n=floor.tiles.len();
        (run.player.x,run.player.y)=((door%w)as f64,(door/w-1)as f64);
        let wave=corpse_core::finale::CollapseFloor{depth:1,up:Some(door),cells:vec![false;n],..Default::default()};
        run.collapse=std::sync::Arc::new(corpse_core::finale::Collapse {
            boss_id:run.next_id,started_tick:run.tick,next_pulse_tick:run.tick+2.,waves:vec![wave],escaped:false,
        }).into();
        corpse_core::engine::log(&mut run,"The Flesh Administrator dies. She falls into the opening flesh. Escape to the blue door!", "environment");
        Keycode::Kp2
    } else {
        // Wait one turn and let the real periodic fire packet kill the Avatar.
        run.player.hp=1.;
        run.player.active.clear();
        run.player.conditions.insert("burning".into(),3.);
        Keycode::Kp5
    };
    let checkpoint=temporal::Engine::new(run).unwrap();
    let save=scene.join("save.json");
    std::fs::write(&save,session_store::encode(&checkpoint,session_store::Kind::Genuine).unwrap()).unwrap();
    CAPTURE.with(|capture| *capture.borrow_mut()=Some(Capture{
        destination:directory.join(format!("{stem}.mp4")),expected:if escape{"won"}else{"dead"},action,
        continued:false,action_sent:false,space_sent:false,started:Instant::now(),game_at:None,
        prompt_at:None,summary_at:None,next_frame:0.,encoder:None,previous_corner:None,changing_frames:0,
    }));
    let mut args=vec!["corpse-desktop".into(),"--software-renderer".into(),"--save".into(),save.to_string_lossy().into_owned()];
    for (flag,name) in [("--profile","profile.json"),("--appearance","appearance.json"),("--replay-dir","replays")] {
        args.push(flag.into());args.push(scene.join(name).to_string_lossy().into_owned());
    }
    let result=run_app(args);
    let c=CAPTURE.with(|capture| capture.borrow_mut().take().unwrap());
    result.unwrap();
    assert!(c.summary_at.is_some());
    std::fs::remove_dir_all(scene).unwrap();
}

#[test]
#[ignore = "explicit native SDL capture; requires ffmpeg"]
fn record_both_ending_fades() {
    let directory=std::path::PathBuf::from(std::env::var("CORPSE_ENDING_CAPTURE_DIR").unwrap());
    std::fs::create_dir_all(&directory).unwrap();
    capture(&directory,false);
    capture(&directory,true);
}
