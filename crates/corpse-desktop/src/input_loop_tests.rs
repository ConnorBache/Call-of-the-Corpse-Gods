//! Test-only SDL injection into the actual production event loop. Nothing here
//! is compiled into ordinary builds. Offscreen delivery is not OS keyboard QA.
use super::*;
use std::{cell::RefCell, path::PathBuf};
use sdl2::{mouse::MouseButton, video::Window, render::Canvas};

pub struct State<'a> {
    pub screen: Screen,
    pub engine: Option<&'a temporal::Engine>,
    pub cursor: bool,
    pub tile: Option<(usize, usize)>,
    pub inventory: bool,
    pub help: bool,
    pub resting: bool,
    pub item: bool,
    pub ability: bool,
    pub history: bool,
    pub context: bool,
}
#[derive(Clone)]
enum Input {
    Key(Keycode, Mod, bool),
    Motion((usize, usize)),
    Click((usize, usize)),
    Toolbar(usize),
    Resize(u32, u32),
    ClickHud(i32,i32),
    RightHud(i32,i32),
}
struct Step {
    label: &'static str,
    inputs: Vec<Input>,
    screen: Screen,
    cursor: bool,
    tile: Option<(usize, usize)>,
    inventory: bool,
    help: bool,
    resting: bool,
    item: bool,
    ability: bool,
    readonly: bool,
    capture: bool,
    cogworks_wall: Option<(usize, usize)>,
    history:Option<bool>,
    context:Option<bool>,
}
impl Step {
    fn game(label: &'static str, inputs: Vec<Input>) -> Self {
        Self { label, inputs, screen: Screen::Game, cursor: false, tile: None,
            inventory: false, help: false, resting: false, item: false, ability: false,
            readonly: true, capture: false, cogworks_wall: None,history:None,context:None }
    }
}
struct Script {
    directory: PathBuf,
    grid: cursor_ui::Grid,
    steps: Vec<Step>,
    index: usize,
    pushed: bool,
    previous: Option<String>,
    reports: Vec<serde_json::Value>,
    record: Option<Recording>,
}
// Optional test-only SDL readback. This records the production event loop,
 // including real command and animation frames, rather than redraw a mockup.
struct Recording {
    pipe: std::process::ChildStdin,
    since: std::time::Instant,
    next: std::time::Instant,
    frames: usize,
}
thread_local! { static SCRIPT: RefCell<Option<Script>> = const { RefCell::new(None) }; }

pub fn inject(sdl: &sdl2::Sdl, canvas: &mut Canvas<Window>, p: &Painter) -> Result<(), String> {
    SCRIPT.with(|slot| {
        let mut slot = slot.borrow_mut(); let Some(script) = slot.as_mut() else { return Ok(()); };
        if script.pushed {
            return if script.record.is_some() {Ok(())} else {Err("Input script observed no completed frame".into())};
        }
        let step = &script.steps[script.index];
        let events = sdl.event()?;
        for input in &step.inputs {
            let event = match input {
                Input::Key(key, keymod, repeat) => Event::KeyDown { timestamp: 0, window_id: canvas.window().id(),
                    keycode: Some(*key), scancode: None, keymod: *keymod, repeat: *repeat },
                Input::Motion(tile) => {
                    let Event::MouseButtonDown { x, y, .. } = cursor_click(p, script.grid, *tile, 0, canvas.window().id()).unwrap() else { unreachable!(); };
                    // SDL events are window pixels; inverse the current responsive mapping.
                    let (ww, wh) = canvas.window().size();
                    let scale = render::canvas_scale(p.width,p.height,true);
                    Event::MouseMotion { timestamp: 0, window_id: canvas.window().id(), which: 0,
                        mousestate: sdl2::mouse::MouseState::from_sdl_state(0),
                        x: (x as f64 * scale * ww as f64 / p.width as f64).round() as i32,
                        y: (y as f64 * scale * wh as f64 / p.height as f64).round() as i32, xrel: 1, yrel: 1 }
                },
                Input::Click(tile) => {
                    let Event::MouseButtonDown { x, y, .. } = cursor_click(p, script.grid, *tile, 0, canvas.window().id()).unwrap() else { unreachable!(); };
                    let (ww, wh) = canvas.window().size();
                    let scale = render::canvas_scale(p.width,p.height,true);
                    Event::MouseButtonDown { timestamp: 0, window_id: canvas.window().id(), which: 0,
                        mouse_btn: MouseButton::Left, clicks: 1,
                        x: (x as f64 * scale * ww as f64 / p.width as f64).round() as i32,
                        y: (y as f64 * scale * wh as f64 / p.height as f64).round() as i32 }
                },
                Input::Toolbar(action) => {
                    let slot = render::TOOLBAR_VISIBLE_ACTIONS.iter().position(|id| id == action).unwrap();
                    let rect=p.hud_tracks().button(slot);
                    let (x,w)=(rect.0,rect.2);
                    let (ww, wh) = canvas.window().size();
                    let scale = render::canvas_scale(p.width,p.height,true);
                    Event::MouseButtonDown { timestamp: 0, window_id: canvas.window().id(), which: 0,
                        mouse_btn: MouseButton::Left, clicks: 1,
                        x: ((x + w / 2) as f64 * scale * ww as f64 / p.width as f64).round() as i32,
                        y: ((rect.1+rect.3/2) as f64 * scale * wh as f64 / p.height as f64).round() as i32 }
                },
                Input::ClickHud(x,y)|Input::RightHud(x,y) => {
                    let (ww,wh)=canvas.window().size();
                    let scale=render::canvas_scale(p.width,p.height,true);
                    Event::MouseButtonDown { timestamp:0,window_id:canvas.window().id(),which:0,
                        mouse_btn:if matches!(input,Input::RightHud(..)){MouseButton::Right}else{MouseButton::Left},clicks:1,
                        x:(*x as f64*scale*ww as f64/p.width as f64).round() as i32,
                        y:(*y as f64*scale*wh as f64/p.height as f64).round() as i32 }
                }
                Input::Resize(w, h) => {
                    canvas.window_mut().set_size(*w, *h).map_err(|e| e.to_string())?;
                    Event::Window { timestamp: 0, window_id: canvas.window().id(),
                        win_event: sdl2::event::WindowEvent::Resized(*w as i32, *h as i32) }
                },
            };
            events.push_event(event)?;
        }
        script.pushed = true;
        if let Some(rec)=script.record.as_mut(){rec.since=std::time::Instant::now();}
        Ok(())
    })
}
fn capture(path: &std::path::Path, p: &Painter, pixels: &[u8]) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut encoder = png::Encoder::new(file, p.width as u32, p.height as u32);
    encoder.set_color(png::ColorType::Rgba); encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().map_err(|e| e.to_string())?.write_image_data(pixels).map_err(|e| e.to_string())
}
pub fn observe(canvas: &mut Canvas<Window>, p: &Painter, state: State<'_>) -> Result<bool, String> {
    SCRIPT.with(|slot| {
        let mut slot = slot.borrow_mut(); let Some(script) = slot.as_mut() else { return Ok(false); };
        let step = &script.steps[script.index];
        if let Some(rec)=script.record.as_mut() {
            use std::{io::Write,time::{Duration,Instant}};
            let now=Instant::now();
            if p.width==2560 && p.height==1440 && now>=rec.next {
                let rgba=canvas.read_pixels(None,PixelFormatEnum::RGBA32)?;
                let copies=if rec.frames==0 {1} else { ((now-rec.next).as_millis()/40+1).min(30) as usize };
                for _ in 0..copies { rec.pipe.write_all(&rgba).map_err(|e|e.to_string())?; }
                rec.frames+=copies;
                rec.next=now+Duration::from_millis(40);
            }
            let hold=if step.label=="bow-aim" {1400} else if step.inventory || step.label=="showcase" {1500} else {855};
            if rec.since.elapsed()<Duration::from_millis(hold) {return Ok(false);}
        }
        let check = |condition: bool, what: &str| if condition { Ok(()) } else { Err(format!("{}: unexpected {what}", step.label)) };
        check(state.screen == step.screen, "screen")?;
        check(state.cursor == step.cursor, "cursor ownership")?;
        if let Some(tile) = step.tile { check(state.tile == Some(tile), &format!("cursor tile {:?}, expected {tile:?}", state.tile))?; }
        check(state.inventory == step.inventory, "inventory")?;
        check(state.help == step.help, "help")?;
        check(state.resting == step.resting, "rest")?;
        check(state.item == step.item, "item selector")?;
        check(state.ability == step.ability, "ability selector")?;
        if let Some(expected)=step.history{check(state.history==expected,"Chronicle drawer")?;}
        if let Some(expected)=step.context{check(state.context==expected,"context detail")?;}
        let engine = state.engine.ok_or_else(|| format!("{}: missing run", step.label))?;
        if step.label=="bow-fire"{
            let arrows=engine.run.inventory.iter().filter(|i|corpse_core::inventory::is_arrow(i))
                .map(corpse_core::inventory::quantity).sum::<f64>();
            check(arrows==7.,"bow shot did not spend one arrow")?;
        }
        if let Some((x,y)) = step.cogworks_wall {
            use corpse_core::revision50_terrain::{Cell, Kind};
            let floor = engine.run.floor().unwrap();
            check(floor.revision50_terrain.as_ref().is_some_and(|mods| mods.iter().any(|m|
                m.kind == Kind::Wall && m.hp == Some(200.) &&
                m.cells == (-2..=2).map(|dy| Cell { x: x as i32, y: y as i32 + dy }).collect::<Vec<_>>()
            )), "five-cell shared-HP wall")?;
            check((y-2..=y+2).all(|row| floor.tiles[row*floor.width()+x] == "wall"), "wall tiles")?;
            check(engine.run.player.energy == 80., "20-energy payment")?;
        }
        let save = engine.save().map_err(|e| e.to_string())?;
        if step.readonly {
            if let Some(previous) = &script.previous { check(*previous == save, "simulation/save mutation")?; }
        }
        script.previous = Some(save);
        if step.capture {
            let pixels = canvas.read_pixels(None, PixelFormatEnum::RGBA32)?;
            capture(&script.directory.join(format!("{}.png", step.label)), p, &pixels)?;
        }
        script.reports.push(serde_json::json!({"step":step.label,"cursor":state.cursor,"tile":state.tile,
            "help":state.help,"inventory":state.inventory,"resting":state.resting,"item":state.item,"ability":state.ability,
            "tick":engine.run.tick,"player":[engine.run.player.x,engine.run.player.y],"read_only_save_equal":step.readonly,
            "renderer":canvas.info().name,"size":[p.width,p.height]}));
        script.index += 1; script.pushed = false;
        if script.index == script.steps.len() {
            std::fs::write(script.directory.join("event-loop-report.json"),
                serde_json::to_vec_pretty(&script.reports).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            return Ok(true);
        }
        Ok(false)
    })
}

pub fn native_input_loop(mut r: corpse_core::Run, directory: PathBuf, software: bool) {
    std::fs::create_dir_all(&directory).unwrap();
    r.player.abilities = vec!["pelian-i-came-for-you".into()];
    assert!(observe::ability_needs_target_for_run(&r, &r.player.abilities[0]));
    assert!(ability_ui::command_binding_allowed(&r, &r.player.abilities[0]));
    r.inventory = vec![corpse_core::revision50_amulet_effects::make_amulet_for_revision(
        corpse_core::revision50_amulet_effects::Effect::Firebolt, 900., 52.).unwrap()];
    r.player.energy = 100.; r.player.stats.max_energy = 100.;
    let fixture = directory.join("input-scene.json");
    std::fs::write(&fixture, temporal::Engine::new(r.clone()).unwrap().save().unwrap()).unwrap();
    let grid = cursor_grid(&r).unwrap(); let key = |key| Input::Key(key, Mod::NOMOD, false);
    let mut steps = vec![];
    let mut step = Step::game("start", vec![key(Keycode::Return)]); step.readonly = false; steps.push(step);
    let mut step = Step::game("cursor-entry", vec![key(Keycode::Return)]); step.cursor = true; step.tile = Some((10,10)); step.capture = true; steps.push(step);
    let mut step = Step::game("cursor-repeat", vec![Input::Key(Keycode::Return,Mod::NOMOD,true)]); step.cursor = true; step.tile = Some((10,10)); steps.push(step);
    let mut step = Step::game("cursor-navigate", vec![key(Keycode::Kp6),key(Keycode::Kp3)]); step.cursor = true; step.tile = Some((12,11)); step.capture = true; steps.push(step);
    let mut step = Step::game("help-open", vec![Input::Toolbar(help_ui::ACTION)]); step.cursor = true; step.help = true; step.capture = true; steps.push(step);
    let mut step = Step::game("help-consumes-actions", vec![key(Keycode::Z),Input::Key(Keycode::Z,Mod::LSHIFTMOD,false),key(Keycode::Kp6)]); step.cursor = true; step.help = true; step.tile = Some((12,11)); steps.push(step);
    let mut step = Step::game("help-close", vec![key(Keycode::Escape)]); step.cursor = true; steps.push(step);
    steps.push(Step::game("cursor-exit", vec![key(Keycode::Escape)]));
    steps.push(Step::game("held-escape-stays-game", vec![Input::Key(Keycode::Escape,Mod::NOMOD,true)]));
    steps.push(Step::game("bare-escape-stays-in-run", vec![key(Keycode::Escape)]));
    let mut step = Step::game("menu-button-leaves-run", vec![Input::Toolbar(9)]);
    step.screen = Screen::Menu; steps.push(step);
    steps.push(Step::game("continue", vec![key(Keycode::C)]));
    let mut step = Step::game("queued-pointer-enter", vec![Input::Motion((12,10)),key(Keycode::Return),key(Keycode::Left)]); step.cursor = true; step.tile = Some((11,10)); steps.push(step);
    let mut step = Step::game("mouse-takeover", vec![Input::Motion((13,10))]); step.tile = Some((13,10)); steps.push(step);
    let mut step = Step::game("cursor-to-pack", vec![key(Keycode::Return),key(Keycode::I)]); step.inventory = true; steps.push(step);
    let mut step = Step::game("pack-consumes-rest", vec![key(Keycode::Z),Input::Key(Keycode::Z,Mod::LSHIFTMOD,false)]); step.inventory = true; steps.push(step);
    steps.push(Step::game("pack-close", vec![key(Keycode::Escape)]));
    let mut step = Step::game("ability-selector", vec![key(Keycode::Return),key(Keycode::Num1)]); step.ability = true; steps.push(step);
    let mut step = Step::game("ability-consumes-rest", vec![key(Keycode::Z),Input::Key(Keycode::Z,Mod::LSHIFTMOD,false)]); step.ability = true; steps.push(step);
    steps.push(Step::game("ability-cancel", vec![key(Keycode::Escape)]));
    // Actual Pack navigation, not direct injection of a selected target.
    let mut pack = pack::PackView::observe(&r); let p = Painter::new(); pack.configure(&p,p.frame().map.tuple());
    pack.key(Keycode::Down,Mod::NOMOD);
    let tabs = pack.keyboard_tabs_for_action_label("Apply").unwrap();
    let mut inputs = vec![key(Keycode::I),key(Keycode::Down)]; inputs.extend((0..tabs).map(|_|key(Keycode::Tab))); inputs.push(key(Keycode::Return));
    let mut step = Step::game("item-selector", inputs); step.item = true; steps.push(step);
    let mut step = Step::game("item-consumes-rest", vec![key(Keycode::Z),Input::Key(Keycode::Z,Mod::LSHIFTMOD,false)]); step.item = true; steps.push(step);
    let mut step = Step::game("item-direction", vec![key(Keycode::Right)]); step.item = true; step.tile = Some((11,10)); step.capture = true; steps.push(step);
    let mut step = Step::game("item-cancel", vec![key(Keycode::Escape)]); step.inventory = true; steps.push(step);
    steps.push(Step::game("item-pack-close", vec![key(Keycode::Escape)]));
    let mut step = Step::game("cursor-rest-once", vec![key(Keycode::Return),key(Keycode::Z)]); step.readonly = false; steps.push(step);
    let mut step = Step::game("cursor-rest-until-change", vec![key(Keycode::Return),Input::Key(Keycode::Z,Mod::LSHIFTMOD,false)]); step.resting = true; step.readonly = false; steps.push(step);
    steps.push(Step::game("rest-cancel", vec![key(Keycode::Escape)]));
    let mut step = Step::game("cursor-reload", vec![key(Keycode::Return),key(Keycode::F9)]); step.readonly = false; steps.push(step);
    let mut step = Step::game("cursor-resize-before", vec![Input::Motion((11,10)),key(Keycode::Return)]); step.cursor = true; step.tile = Some((11,10)); steps.push(step);
    let mut step = Step::game("cursor-resize", vec![Input::Resize(900,1200)]); step.cursor = true; step.tile = Some((11,10)); steps.push(step);
    let mut step = Step::game("cursor-portrait", vec![key(Keycode::Up)]); step.cursor = true; step.tile = Some((11,9)); step.capture = true; steps.push(step);
    let mut step = Step::game("help-portrait", vec![key(Keycode::F1)]); step.cursor = true; step.help = true; step.capture = true; steps.push(step);
    let mut step = Step::game("help-portrait-close", vec![key(Keycode::Escape)]); step.cursor = true; steps.push(step);
    let mut step = Step::game("cursor-confirm", vec![key(Keycode::Return)]); step.readonly = false; steps.push(step);
    run_script(directory, software, Some(fixture), grid, steps);
}

pub fn native_cogworks_wall(mut r: corpse_core::Run, directory: PathBuf) {
    std::fs::create_dir_all(&directory).unwrap();
    r.player.abilities = vec!["kative-cogworks".into()];
    r.player.ability_reserves = Field::Missing;
    corpse_core::revision50_ability_energy::initialize_reserve(
        &mut r.player, "kative-cogworks", 10., 0.,
    ).unwrap();
    r.player.energy = 100.; r.player.stats.max_energy = 100.;
    let engine = corpse_core::temporal::Engine::new(r.clone()).unwrap();
    std::fs::write(directory.join("save.json"),
        session_store::encode(&engine, session_store::Kind::Genuine).unwrap()).unwrap();
    let grid = cursor_grid(&r).unwrap();
    let key = |key| Input::Key(key, Mod::NOMOD, false);
    let mut start = Step::game("continue-save", vec![key(Keycode::C)]);
    start.readonly = false;
    let mut aim = Step::game("aim-wall", vec![key(Keycode::Num1), key(Keycode::Num2)]);
    aim.ability = true;
    aim.readonly = false;
    let mut invalid = Step::game("reject-off-axis-wall", vec![Input::Click((14,11))]);
    invalid.ability = true;
    let mut cast = Step::game("click-valid-wall", vec![Input::Click((14,10))]);
    cast.readonly = false;
    cast.cogworks_wall = Some((14,10));
    run_script(directory, true, None, grid, vec![start, aim, invalid, cast]);
}

fn run_script(directory: PathBuf, software: bool, fixture: Option<PathBuf>, grid: cursor_ui::Grid, steps: Vec<Step>) {
    run_script_record(directory,software,fixture,grid,steps,None);
}
fn run_script_record(directory: PathBuf, software: bool, fixture: Option<PathBuf>, grid: cursor_ui::Grid, steps: Vec<Step>, record:Option<Recording>) {
    let expected = steps.len();
    SCRIPT.with(|slot| *slot.borrow_mut() = Some(Script { directory:directory.clone(), grid, steps,
        index:0,pushed:false,previous:None,reports:vec![],record }));
    let mut args = vec!["corpse-desktop".into()];
    if let Some(fixture) = fixture {
        args.extend(["--fixture".into(), fixture.to_string_lossy().into_owned()]);
    }
    for (flag,name) in [("--save","save.json"),("--profile","profile.json"),("--appearance","appearance.json"),("--replay-dir","replays")] {
        args.push(flag.into()); args.push(directory.join(name).to_string_lossy().into_owned());
    }
    if software {args.push("--software-renderer".into());}
    let result = run_app(args);
    let script = SCRIPT.with(|slot|slot.borrow_mut().take().unwrap());
    result.unwrap(); assert_eq!(script.index,expected);
}


#[test]
#[ignore = "Optional native HUD walkthrough, SDL offscreen and ffmpeg"]
fn record_mockup_hud_walkthrough() {
    let Some(dir)=std::env::var_os("CORPSE_MOCKUP_CAPTURE") else{return;};
    let dir=PathBuf::from(dir);
    let run=crate::session_store::load_tracked(&dir.join("showcase-save.json")).unwrap().0;
    let grid=cursor_grid(&run.run).unwrap();
    let staging=dir.join("recording");
    std::fs::create_dir_all(&staging).unwrap();
    std::fs::copy(dir.join("showcase-save.json"),staging.join("save.json")).unwrap();
    let mut p=Painter::new();
    p.resize(2560,1440);
    p.set_responsive_canvas(true);
    let tracks=p.hud_tracks();
    let click=|b:crate::modal_ui::Rect|Input::ClickHud(b.0+b.2/2,b.1+b.3/2);
    let key=|key|Input::Key(key,Mod::NOMOD,false);
    let mut steps=Vec::new();
    let mut step=Step::game("showcase",vec![Input::Resize(2560,1440),key(Keycode::C)]);
    step.readonly=false;step.capture=true;steps.push(step);
    // A visible stair has inspector prose but no item/creature popup.
    let stair=observe::observe(&run.run).ground_entities.iter()
        .find(|row|row.kind==render::enemy_list::GroundKind::Stair).map(|row|(row.x,row.y)).unwrap();
    let mut step=Step::game("showcase-inspector",vec![Input::Motion(stair)]);
    step.capture=true;steps.push(step);
    let (jx,jy): (usize,usize) = serde_json::from_slice(&std::fs::read(dir.join("jewelry-location.json")).unwrap()).unwrap();
    let mut step=Step::game("look-at-flight-ring",vec![Input::Motion((jx,jy))]);step.capture=true;steps.push(step);
    let v=observe::observe(&run.run);
    let (x,y)=v.cells.iter().find_map(|c|c.actor_id.is_some_and(|id|Some(id)==v.creatures.first().map(|a|a.id)).then_some((c.x,c.y)))
        .unwrap_or((grid.player.0+3,grid.player.1));
    let mut step=Step::game("look-at-enemy",vec![Input::Motion((x,y))]);step.capture=true;steps.push(step);
    let mut step=Step::game("bow-aim",vec![click(tracks.item_geometry().row(v.pack_hud.items.iter().position(|a|a.label=="Bow").unwrap())),Input::Motion((x,y))]);
    step.item=true;steps.push(step);
    let mut step=Step::game("bow-fire",vec![Input::Click((x,y))]);step.readonly=false;steps.push(step);
    let mut step=Step::game("open-pot-inventory",vec![click(tracks.potions)]);step.inventory=true;steps.push(step);
    steps.push(Step::game("close-pot-inventory",vec![key(Keycode::Escape)]));
    let mut step=Step::game("open-heart-inventory",vec![click(tracks.hearts)]);step.inventory=true;steps.push(step);
    steps.push(Step::game("close-heart-inventory",vec![key(Keycode::Escape)]));
    let mut step=Step::game("open-full-pack-by-key",vec![key(Keycode::I)]);step.inventory=true;steps.push(step);
    steps.push(Step::game("close-full-pack",vec![key(Keycode::Escape)]));
    let mut step=Step::game("wait-turn",vec![key(Keycode::Z)]);step.readonly=false;steps.push(step);
    let slot=render::TOOLBAR_VISIBLE_ACTIONS.iter().position(|id|*id==help_ui::ACTION).unwrap();
    let mut step=Step::game("help-from-header",vec![click(tracks.button(slot))]);step.help=true;steps.push(step);
    steps.push(Step::game("back-to-dungeon",vec![key(Keycode::Escape)]));
    let path=dir.join("Call-of-the-Corpse-Gods-HUD-1440p.mp4");
    let mut ffmpeg=std::process::Command::new("ffmpeg")
        .args(["-hide_banner","-loglevel","error","-y","-f","rawvideo","-pixel_format","rgba",
               "-video_size","2560x1440","-framerate","25","-i","-","-an","-c:v","libx264",
               "-preset","ultrafast","-threads","2","-crf","15","-pix_fmt","yuv420p","-movflags","+faststart"])
        .arg(path).stdin(std::process::Stdio::piped()).spawn().unwrap();
    let now=std::time::Instant::now();
    let record=Recording {pipe:ffmpeg.stdin.take().unwrap(),since:now,next:now,frames:0};
    run_script_record(staging,true,None,grid,steps,Some(record));
    assert!(ffmpeg.wait().unwrap().success());
}

#[test]
#[ignore = "Optional native Hearts and Potions stills; use SDL offscreen"]
fn capture_supply_panels() {
    use corpse_core::{Item,PartialStats};
    let Some(root)=std::env::var_os("CORPSE_SUPPLY_CAPTURE").map(PathBuf::from) else{return;};
    std::fs::create_dir_all(&root).unwrap();
    let mut run=corpse_core::startup::create_avatar_run("02-03-09:layout-review").unwrap();
    run.inventory.retain(|item|!matches!(item.kind.as_str(),"heart"|"potion"));
    for (name,n,exp,bonus) in [
        ("Giant Rat heart",2,2.,PartialStats{offense:2.0.into(),..Default::default()}),
        ("Cave Bat heart",1,2.,PartialStats{dodge:2.0.into(),..Default::default()}),
        ("Pale Jackal heart",1,4.,PartialStats{max_hp:4.0.into(),..Default::default()}),
    ] {
        for _ in 0..n {
            let id=run.next_id;run.next_id+=1.;
            run.inventory.push(Item{id,name:name.into(),kind:"heart".into(),glyph:"♥".into(),exp:exp.into(),bonus:bonus.clone().into(),..Default::default()});
        }
    }
    for (name,effect,n) in [
        ("Potion of Healing","healing",2),("Potion of Levitation","levitation",1),("Potion of Confusion","confusion",1),
        ("Verdania Sap","Veridian Sap",1),
    ] {
        for _ in 0..n {
            let id=run.next_id;run.next_id+=1.;
            run.inventory.push(Item{id,name:name.into(),kind:"potion".into(),glyph:"!".into(),effect:effect.to_owned().into(),identified:true.into(),..Default::default()});
        }
    }
    let grid=cursor_grid(&run).unwrap();
    let e=temporal::Engine::new(run).unwrap();
    std::fs::write(root.join("save.json"),crate::session_store::encode(&e,crate::session_store::Kind::Genuine).unwrap()).unwrap();
    let key=|k|Input::Key(k,Mod::NOMOD,false);
    let mut entry=Step::game("resume",vec![Input::Resize(3840,2160),key(Keycode::C)]);entry.readonly=false;
    let mut resize=Step::game("resize",vec![Input::Resize(3840,2160)]);resize.readonly=false;
    let mut steps=vec![entry,resize];
    let mut p=Painter::new();
    p.resize(3840,2160);p.set_responsive_canvas(true);
    let tracks=p.hud_tracks();
    for (rect,unselected,selected) in [
        (tracks.hearts,"hearts-unselected","hearts"),
        (tracks.potions,"potions-unselected","potions"),
    ] {
        let mut step=Step::game(unselected,vec![Input::ClickHud(rect.0+rect.2/2,rect.1+rect.3/2)]);step.inventory=true;step.capture=true;steps.push(step);
        let mut step=Step::game(selected,vec![key(Keycode::Down)]);step.inventory=true;step.capture=true;steps.push(step);
        steps.push(Step::game("esc-close",vec![key(Keycode::Escape)]));
    }
    let gods=tracks.gods;
    let mut god=Step::game("right-click-god",vec![Input::RightHud(gods.0+12,gods.1+8)]);god.context=Some(true);steps.push(god);
    let mut close=Step::game("close-god-detail",vec![key(Keycode::Escape)]);close.context=Some(false);steps.push(close);
    let chronicle=p.hud_tracks().chronicle;
    let mut open=Step::game("click-chronicle",vec![Input::ClickHud(chronicle.0+4,chronicle.1+4)]);open.history=Some(true);steps.push(open);
    run_script(root.clone(),true,None,grid,steps);
    for name in ["hearts-unselected","hearts","potions-unselected","potions"] {
        let png=std::fs::read(root.join(format!("{name}.png"))).unwrap();
        assert_eq!(&png[16..24],&[0,0,15,0,0,0,8,112],"full native 3840×2160");
    }
}
