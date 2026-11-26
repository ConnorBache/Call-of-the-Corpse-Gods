#![cfg_attr(all(target_os = "windows", not(debug_assertions), not(feature = "dev-tools")), windows_subsystem = "windows")]

mod gameplay_pause;
mod health_warning;
mod death_pause;
mod gpu_render;
#[cfg(feature = "runtime-timing")]
mod timing_test;
mod ability_ui;
mod cogworks_ui;
mod appearance_light;
mod illumination_ui;
#[cfg(test)]
mod illumination_render_tests;
mod appearance_material;
mod appearance_profile;
mod appearance_store;
mod appearance_ui;
mod chronicle;
mod chronicle_overlay;
mod tile_overlay;
#[cfg(test)]
mod overlay_damage;
mod color;
mod color_profile;
mod creature_abilities_ui;
mod creature_presentation;
mod hallucination_ui;
mod effects;
mod divine_animation_shapes;
mod staged_firebolt;
mod action_sequence;
mod action_marks;
mod embedded_assets;
mod entity_profile;
mod hazards_ui;
mod future_confirmation;
mod inspection_ui;
mod manual;
mod bestiary_progress;
mod display_ui;
mod dream_ui;
mod menu_ui;
mod help_ui;
mod cursor_ui;
#[cfg(test)] mod input_loop_tests;
mod avatar_menu;
mod menu_visit;
#[cfg(test)] mod avatar_menu_tests;
mod modal_ui;
mod observe;
mod perception_ui;
mod persistent_ability_overlays;
mod pack;
mod portal_ui;
mod non_euclidean_ui;
mod sub_biomes_ui;
mod profiles;
mod render;
mod replays;
mod session_store;
mod startup_ui;
mod debug_floor_ui;
mod release_policy;
mod statuses_ui;
mod monster_status_ui;
mod stat_presentation;
mod targeting_ui;
mod directional_zap_ui;
#[cfg(test)] mod aiming_effect_tests;
mod terminal_ui;
mod travel_ui;
mod typography_ui;
mod wait_ui;
mod world_features;
use corpse_core::{
    model::{Command, Field},
};
use session_store::tracked_engine as temporal;
use render::{Painter, HEIGHT, WIDTH};
use sdl2::{
    event::Event,
    keyboard::{Keycode, Mod},
    pixels::PixelFormatEnum,
};
use std::time::{Duration, Instant};
#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Menu,
    Start,
    ImportSave,
    Summary,
    Manual,
    Game,
    Replays,
    ReplayGame,
    Scores,
    Settings,
}
/// O(1) idle scheduling: Bestiary content is static; its input/window events
/// already set dirty. Keep timed redraws only for animated non-game screens.
fn screen_has_timed_paint(screen: Screen) -> bool {
    matches!(screen, Screen::Menu | Screen::ReplayGame)
}

#[cfg(test)]
mod static_bestiary_schedule_tests {
    use super::*;
    #[test]
    fn static_bestiary_never_requests_clock_only_frames() {
        assert!(!screen_has_timed_paint(Screen::Manual));
        assert!(screen_has_timed_paint(Screen::Menu));
        assert!(screen_has_timed_paint(Screen::ReplayGame));
        // Explicit event dirtiness must still admit a repaint at an unchanged clock.
        for dirty in [false, true] {
            let draw = dirty || (screen_has_timed_paint(Screen::Manual) && 19 != 18);
            assert_eq!(draw, dirty);
        }
    }
}

/// Shared menu layout: pointer hover and activation use the same visible rows.
/// O(1), reuse exact painted/clicked card bounds; only changes UI selection, never dispatches.
fn update_blessing_hover(selected:&mut usize,count:usize,x:i32,y:i32)->bool{
    if let Some(index)=render::blessing_choice_at(x,y).filter(|i|*i<count){if *selected!=index{*selected=index;return true;}}false
}
fn menu_item_at(x: i32, y: i32) -> Option<usize> {
    menu_ui::item_at(x, y)
}

// Render-test captures use the live first-appearance color unless a preview override was explicit.
fn paint_manual_render_test(
    manual: &mut manual::Manual,
    painter: &mut Painter,
    time_ms: f64,
    depth_override: Option<f64>,
) {
    if let Some(depth) = depth_override {
        manual.paint_at_visual_depth(painter, time_ms, depth);
    } else {
        manual.paint_at(painter, time_ms);
    }
}

/// O(1): pass only the active originating view's revision to the Bestiary.
/// Cached inactive games/replays must not change the menu's current-rule pages.
fn manual_simulation_revision(
    origin: Screen,
    game: Option<&corpse_core::Run>,
    replay: Option<&corpse_core::Run>,
) -> Option<f64> {
    match origin {
        Screen::Game => game,
        Screen::ReplayGame => replay,
        _ => None,
    }.map(|run| run.simulation_revision.value_or(29.))
}

#[cfg(test)]
mod bestiary_revision_routing_tests {
    use super::*;
    #[test]
    fn originating_game_or_replay_supplies_only_its_revision() {
        let mut game: corpse_core::Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let mut replay = game.clone();
        game.simulation_revision = 50.0.into();
        replay.simulation_revision = 51.0.into();
        assert_eq!(manual_simulation_revision(Screen::Game, Some(&game), Some(&replay)), Some(50.));
        assert_eq!(manual_simulation_revision(Screen::ReplayGame, Some(&game), Some(&replay)), Some(51.));
        game.simulation_revision = Field::Missing;
        assert_eq!(manual_simulation_revision(Screen::Game, Some(&game), None), Some(29.));
        for screen in [Screen::Menu, Screen::Manual, Screen::Replays, Screen::Scores, Screen::Settings, Screen::Summary, Screen::Start, Screen::ImportSave] {
            assert_eq!(manual_simulation_revision(screen, Some(&game), Some(&replay)), None);
        }
        assert_eq!(manual_simulation_revision(Screen::Game, None, Some(&replay)), None);
        assert_eq!(manual_simulation_revision(Screen::ReplayGame, Some(&game), None), None);
    }
}

fn can_continue(
    run: &Option<temporal::Engine>,
    saved: &Option<(temporal::Engine, session_store::Kind)>,
) -> bool {
    // The active run supersedes disk state, including after death: never resurrect a stale save.
    run.as_ref().filter(|r| r.run.playtest.is_missing())
        .map(|r| r.run.status == "playing")
        .unwrap_or_else(|| {
            saved
                .as_ref()
                .is_some_and(|(r, _)| r.run.status == "playing")
        })
}
fn continue_run(
    run: &mut Option<temporal::Engine>,
    saved: &mut Option<(temporal::Engine, session_store::Kind)>,
    kind: &mut session_store::Kind,
) -> bool {
    if !can_continue(run, saved) {
        return false;
    }
    if run.as_ref().is_none_or(|r| !r.run.playtest.is_missing()) {
        if let Some((engine, saved_kind)) = saved.take() {
            *run = Some(engine);
            *kind = saved_kind;
        }
    }
    true
}

/// O(file bytes + checkpoint decode + tail replay), bounded32MiB input; wrapper setup O(1).
fn load_with_kind(path: &str) -> Result<(temporal::Engine, session_store::Kind), String> {
    if std::fs::metadata(path)
        .map_err(|e| format!("Cannot load {path}: {e}"))?
        .len()
        > 32 * 1024 * 1024
    {
        return Err("Cannot load save larger than 32 MiB.".into());
    }
    let (engine, kind) = session_store::load_tracked(std::path::Path::new(path))?;
    release_policy::check_session(engine.run.dev, kind != session_store::Kind::Genuine
        || !engine.run.playtest.is_missing(), &engine.run.replay)?;
    Ok((engine, kind))
}
fn load(path: &str) -> Result<temporal::Engine, String> {
    load_with_kind(path).map(|(run, _)| run)
}

fn main() -> Result<(), String> {
    let result = run_app(std::env::args().collect());
    #[cfg(all(target_os = "windows", not(debug_assertions), not(feature = "dev-tools")))]
    if let Err(message) = &result {
        // A player opening the GUI executable has no stderr console.
        let _ = sdl2::messagebox::show_simple_message_box(
            sdl2::messagebox::MessageBoxFlag::ERROR, "Call of the Corpse Gods", message, None::<&sdl2::video::Window>,
        );
    }
    result
}
fn run_app(args: Vec<String>) -> Result<(), String> {
    release_policy::check_args(&args)?;
    #[cfg(feature = "runtime-timing")]
    if args.iter().any(|s| s == "--timing-test") {
        return timing_test::run(&args);
    }
    #[cfg(not(feature = "runtime-timing"))]
    if args.iter().any(|s| s == "--timing-test") {
        return Err("Timing tests require a build with --features runtime-timing".into());
    }
    let fixture_session = release_policy::DEV_TOOLS && args.iter().any(|s| s == "--fixture");
    let mut seed = args
        .iter()
        .position(|s| s == "--seed")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_default();
    let mode = args
        .iter()
        .position(|s| s == "--mode")
        .map(|i| args.get(i + 1).cloned().ok_or("--mode requires Avatar"))
        .transpose()?
        .unwrap_or_else(|| "Avatar".into());
    // A mode flag cannot create a new non-Avatar run; historical fixture/replay
    // loading still uses its recorded mode without rewriting the saved state.
    if args.iter().any(|s|s=="--generated") || (!fixture_session && !args.iter().any(|s|s=="--replay")) {validate_new_mode(&mode)?;}
    let mut avatar_selection = avatar_menu::Selection::default();
    avatar_selection.sync_seed(&seed);
    let mut seed_focus = true;
    let mut floor_focus = false;
    let mut pending_start = startup_ui::Loader::<temporal::Engine>::default();
    let mut playtest_floor = args.iter().position(|s| release_policy::DEV_TOOLS && (s == "--playtest-floor" || s == "--debug-floor"))
        .map(|i| args.get(i + 1).cloned().ok_or("--playtest-floor requires a floor from 1 to 20"))
        .transpose()?;
    if let Some(floor) = &playtest_floor {
        corpse_core::playtest::parse_floor(floor)?;
        seed=avatar_selection.prepared_seed(&seed,fresh_seed)?;
        avatar_selection.sync_seed(&seed);
    }
    let mut session_kind = if fixture_session {
        session_store::Kind::Fixture
    } else {
        session_store::Kind::Genuine
    };
    let mut provenance = if fixture_session {
        "Native migration test scene / incomplete port"
    } else {
        ""
    };
    let replay_dir = args
        .iter()
        .position(|s| s == "--replay-dir")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| {
            appearance_store::default_path()
                .with_file_name("replays")
                .to_string_lossy()
                .into_owned()
        });
    let mut replays = replays::Replays::new(std::path::Path::new(&replay_dir));
    let profile_path = args
        .iter()
        .position(|s| s == "--profile")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| {
            appearance_store::default_path()
                .with_file_name("profile.json")
                .to_string_lossy()
                .into_owned()
        });
    let mut profiles = profiles::Profiles::new(std::path::Path::new(&profile_path));
    let explicit_appearance = args
        .iter()
        .position(|a| a == "--appearance")
        .and_then(|i| args.get(i + 1));
    let appearance_path = explicit_appearance
        .map(std::path::PathBuf::from)
        .unwrap_or_else(appearance_store::default_path);
    let appearance_directory_error = if explicit_appearance.is_none() {
        appearance_store::prepare_default_directory(&appearance_path).err()
    } else {
        None
    };
    let mut settings = appearance_ui::SettingsView::load(&appearance_path);
    if let Some(error) = appearance_directory_error {
        settings.notice = error;
    }
    let mut painter = Painter::new();
    #[cfg(feature = "dev-tools")]
    if let Some(i) = args.iter().position(|s| s == "--render-size") {
        let value = args
            .get(i + 1)
            .ok_or("--render-size requires WIDTHxHEIGHT")?;
        let (w, h) = value
            .split_once('x')
            .ok_or("--render-size requires WIDTHxHEIGHT")?;
        let (w, h): (usize, usize) = (
            w.parse().map_err(|_| "invalid render width")?,
            h.parse().map_err(|_| "invalid render height")?,
        );
        if !(1..=8192).contains(&w) || !(1..=8192).contains(&h) {
            return Err("render dimensions must be 1..8192".into());
        }
        painter.try_resize(w,h)?;
    }
    painter.profile = settings.profile().clone();
    let mut manual = manual::Manual::new();
    manual.progress=bestiary_progress::Progress::load(bestiary_progress::Progress::path_for_profile(std::path::Path::new(&profile_path)));
    #[cfg(feature = "dev-tools")]
    if args.iter().any(|s|s=="--bestiary-preview-all") {manual.preview_all();}
    let mut bestiary_notices=bestiary_progress::Notices::default();
    #[cfg(feature = "dev-tools")]
    if let Some(id) = args
        .iter()
        .position(|s| s == "--manual-entry")
        .and_then(|i| args.get(i + 1))
    {
        if !manual.select_id(id) {
            return Err(format!("Unknown bestiary entry: {id}"));
        }
    }
    #[cfg(feature = "dev-tools")]
    let manual_depth = if let Some(i) = args.iter().position(|s| s == "--manual-depth") {
        let depth: f64 = args.get(i + 1).ok_or("--manual-depth requires 1..40")?
            .parse().map_err(|_| "--manual-depth requires 1..40")?;
        if !depth.is_finite() || !(1.0..=40.0).contains(&depth) { return Err("--manual-depth requires 1..40".into()); }
        Some(depth - 1.)
    } else { None };
    #[cfg(feature = "dev-tools")]
    let animation_frame: f64 = if let Some(i) = args.iter().position(|s| s == "--animation-frame") {
        let value: f64 = args.get(i + 1).ok_or("--animation-frame requires milliseconds")?
            .parse().map_err(|_| "invalid animation frame")?;
        if !value.is_finite() || value < 0. { return Err("animation frame must be finite and nonnegative".into()); }
        value
    } else { 0. };
    #[cfg(feature = "dev-tools")]
    if let Some(i) = args.iter().position(|s| s == "--render-test") {
        let path = args.get(i + 1).ok_or("--render-test requires PNG path")?;
        if args.iter().any(|s| s == "--render-help") {
            painter.clear([12, 12, 18]);
            let mut help = help_ui::Help::default(); help.open(&painter); help.paint(&mut painter);
        } else if args
            .iter()
            .any(|s| s == "--manual" || s == "--manual-entry")
        {
            paint_manual_render_test(&mut manual, &mut painter,
                if settings.reduced_motion { 0. } else { animation_frame }, manual_depth)
        } else if args.iter().any(|s|s=="--render-chronicle-shutter"){
            painter.clear([24,27,32]);let mut shutter=chronicle_overlay::Shutter::default();shutter.open(0.,false);
            let lines=(1..=45).map(|i|format!("Event {i}: You remember the winding corridors and distant footsteps.")).collect::<Vec<_>>();
            chronicle_overlay::paint(&mut painter,&lines,&shutter,animation_frame,false);
        } else if args.iter().any(|s| s == "--render-start") {
            painter.start(&seed, &mode, true, playtest_floor.as_deref().map(|s| (s, false)), "");
        } else {
            {painter.menu(18, 0, false, &mode);painter.menu_seed(&seed,false,false);painter.menu_fast_mode(false,false);avatar_selection.paint_slots(&mut painter,0);
             if let Some(i)=args.iter().position(|s|s=="--render-god-picker") {
                 let slot=args.get(i+1).and_then(|s|s.parse::<usize>().ok()).unwrap_or(1).clamp(1,3)-1;
                 avatar_selection.open(slot);avatar_selection.paint_picker(&mut painter);
             }}
        };
        let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
        let mut encoder = png::Encoder::new(file, painter.width as u32, painter.height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .map_err(|e| e.to_string())?
            .write_image_data(&painter.pixels)
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    let fixture = args
        .iter()
        .position(|s| s == "--fixture")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| {
            let relative = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.join("fixtures/avatar-room-v1.json")));
            relative
                .filter(|p| p.is_file())
                .unwrap_or_else(|| {
                    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("../../fixtures/avatar-room-v1.json")
                })
                .to_string_lossy()
                .into_owned()
        });
    #[cfg(feature = "dev-tools")]
    if let Some(i) = args.iter().position(|s| s == "--render-game") {
        let path = args
            .get(i + 1)
            .ok_or("--render-game requires output path")?;
        let r = if args.iter().any(|a| a == "--generated") {
            new_selected_run(&avatar_selection.prepared_seed(&seed,fresh_seed)?, &mode, playtest_floor.as_deref())?
        } else {
            load(&fixture)?
        };
        let mut view = observe::observe_with_profile(&r.run, &painter.profile);
        if args.iter().any(|a| a == "--generated") || !r.run.playtest.is_missing() {
            view.provenance = if r.run.playtest.is_missing() { "" } else { "DEBUG • Prior-floor reconstruction • No autosave / scores • F4 stash" }.into();
        }
        view.measure_conditions(&mut statuses_ui::ConditionMeters::default());
        painter.game_at(
            &view,
            if args.iter().any(|a| a == "--generated") || !r.run.playtest.is_missing() {
                ""
            } else {
                "Native migration test scene; port coverage incomplete."
            },
            false,
            if settings.reduced_motion { 0. } else { animation_frame },
        );
        if args.iter().any(|a| a == "--pack") {
            let mut pack = pack::PackView::observe(&r.run);
            pack.configure(
                &painter,
                painter.frame().map.tuple(),
            );
            // Render-only QA navigation uses the same non-dispatching Home key
            // path as the live Pack, so item details/actions can be inspected.
            if args.iter().any(|a| a == "--pack-select-first") {
                let _ = pack.key(Keycode::Home, Mod::NOMOD);
            }
            pack.paint(&mut painter);
        }
        if args.iter().any(|a| a == "--inspect") {
            painter.creatures(&view, None, 0);
        }
        if let Some(i) = args.iter().position(|a| a == "--inspect-tile") {
            let (x, y) = args
                .get(i + 1)
                .and_then(|a| a.split_once(','))
                .ok_or("--inspect-tile requires x,y")?;
            let report = inspection_ui::observe_tile(
                &r.run,
                x.parse().map_err(|_| "Invalid tile x")?,
                y.parse().map_err(|_| "Invalid tile y")?,
            );
            painter.centered_sheet(|p| inspection_ui::paint(&report,p,0));
        }
        if !view.offers.is_empty() {
            painter.blessing_choices(&view.offers, 0);
            painter.centered_sheet(|p|debug_floor_ui::choice_banner(&r.run,p));
        }
        let debug_view = debug_floor_ui::View { open: args.iter().any(|a| a == "--debug-stash"), ..Default::default() };
        if debug_view.active(&r.run) { painter.centered_sheet(|p| debug_view.paint(&r.run,p)); }
        let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
        let mut encoder = png::Encoder::new(file, painter.width as u32, painter.height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .map_err(|e| e.to_string())?
            .write_image_data(&painter.pixels)
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(feature = "dev-tools")]
    if let Some(i) = args.iter().position(|s| s == "--dump-observation") {
        let path = args
            .get(i + 1)
            .ok_or("--dump-observation requires output path")?;
        let r = if args.iter().any(|a| a == "--generated") {
            new_selected_run(&avatar_selection.prepared_seed(&seed,fresh_seed)?, &mode, playtest_floor.as_deref())?
        } else {
            load(&fixture)?
        };
        let v = observe::observe(&r.run);
        let cells:Vec<_>=v.cells.iter().map(|c|serde_json::json!({"x":c.x,"y":c.y,"glyph":c.glyph,"fg":c.fg,"bg":c.bg,"heartless":c.heartless,"superscript":c.superscript,"arrow":c.arrow})).collect();
        let data = serde_json::json!({"width":v.width,"height":v.height,"cells":cells,"hp":v.hp,"maxHp":v.max_hp,"energy":v.energy,"maxEnergy":v.max_energy,"tick":v.turn,"depth":v.display_depth,"log":v.log,"inventory":v.inventory});
        std::fs::write(path, serde_json::to_string_pretty(&data).unwrap())
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    #[cfg(feature = "dev-tools")]
    if args.iter().any(|s| s == "--check-fixture") {
        let r = if args.iter().any(|a| a == "--generated") {
            new_selected_run(&avatar_selection.prepared_seed(&seed,fresh_seed)?, &mode, playtest_floor.as_deref())?
        } else {
            load(&fixture)?
        };
        println!("Fixture validated: {} at turn {}", r.run.mode, r.run.tick);
        return Ok(());
    }
    let save_path = args
        .iter()
        .position(|s| s == "--save")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| {
            let base = appearance_store::default_path();
            let path = base.with_file_name(if fixture_session {
                "fixture-save.json"
            } else {
                "native-save.json"
            });
            path.to_string_lossy().into_owned()
        });
    let mut run: Option<temporal::Engine> = None;
    // Validate once, off the render/input hot path. Do not activate or autosave until Continue.
    let mut saved_run = if std::path::Path::new(&save_path).exists() {
        load_with_kind(&save_path)
            .ok()
            .filter(|(r, _)| r.run.status == "playing")
    } else {
        None
    };
    let mut notice = if cfg!(feature = "append-recording-trial") {
        "Experimental append-recording trial. Separate saves; keep your original game.".to_string()
    } else { String::new() };
    if !args.iter().any(|a| a == "--save") {
        if let Err(e) =
            appearance_store::prepare_default_directory(std::path::Path::new(&save_path))
        {
            notice = e;
        }
    }
    manual.progress.report_error(&mut notice);
    let sdl = sdl2::init()?;
    let video = sdl.video()?;
    let mut window = video
        .window("CALL OF THE CORPSE GODS", WIDTH as u32, HEIGHT as u32)
        .position_centered()
        .resizable()
        .allow_highdpi()
        .build()
        .map_err(|e| e.to_string())?;
    embedded_assets::set_window_icon(&mut window)?;
    // SDL's default selector prefers accelerated drivers and falls back to software.
    // Explicit software mode retains the original renderer for troubleshooting.
    let software_requested=args.iter().any(|a|a=="--software-renderer") || std::env::var("CORPSE_RENDERER").as_deref()==Ok("software");
    let (mut canvas,mut gpu_available)=gpu_render::create_canvas(window,software_requested)?;
    eprintln!("Drawing renderer: {} ({}); simulation remains deterministic CPU work",canvas.info().name,if gpu_available{"GPU command path"}else{"software fallback"});
    if let Err(error) = change_window_display(canvas.window_mut(), &video, settings.display_mode, settings.store.resolution) {
        settings.notice = format!("Cannot apply display mode: {error}");
    }
    let actual_display = appearance_store::DisplayMode::from_sdl(canvas.window().fullscreen_state());
    if actual_display != settings.display_mode && settings.notice.is_empty() {
        settings.notice = "Saved display mode is unavailable on this display.".into();
    }
    settings.observe_display_mode(actual_display);
    if !settings.notice.is_empty() { notice = settings.notice.clone(); }
    let mut creator = canvas.texture_creator();
    let mut gpu_failed=false;
    let mut pending_gpu_failure:Option<String>=None;
    let mut gpu_renderer=gpu_render::Renderer::new(&creator,canvas.info().max_texture_width.min(canvas.info().max_texture_height));
    let mut texture = creator
        .create_texture_streaming(PixelFormatEnum::RGBA32, WIDTH as u32, HEIGHT as u32)
        .map_err(|e| e.to_string())?;
    let mut events = sdl.event_pump()?;
    video.text_input().start();
    let start = Instant::now();
    let mut effects = effects::Effects::default();
    effects.gameplay_pause.enabled=true;
    let mut screen = if args
        .iter()
        .any(|s| s == "--manual" || s == "--manual-entry")
    {
        Screen::Manual
    } else if playtest_floor.is_some() {
        seed_focus = true;
        Screen::Start
    } else {
        Screen::Menu
    };
    if let Some(path) = args
        .iter()
        .position(|s| s == "--replay")
        .and_then(|i| args.get(i + 1))
    {
        replays.load(std::path::Path::new(path))?;
        screen = Screen::ReplayGame;
    }
    let mut previous = Screen::Menu;

    let mut last_frame = u32::MAX;
    let mut dirty = true;
    let mut cosmetic_dirty = false;
    let mut help = help_ui::Help::default();
    let mut inventory = false;
    let mut pack_view: Option<pack::PackView> = None;
    let mut pending_item: Option<pack::PackTarget> = None;
    let mut ability_detail: Option<String> = None;
    let mut ability_detail_scroll = 0usize;
    let mut travel: Option<(travel_ui::Travel, f64)> = None;
    let mut autoexplore = travel_ui::AutoExplore::default();
    let mut hover_tile: Option<(usize, usize)> = None;
    let mut keyboard_aim=false;
    let mut keyboard_cursor=cursor_ui::Cursor::default();
    let mut pending_hover_position: Option<Option<(i32,i32)>> = None;
    let mut pending_hover_raw: Option<Option<(i32,i32)>> = None;
    let mut hover_path: Vec<(usize, usize)> = Vec::new();
    let mut hover_report: Option<inspection_ui::Report> = None;
    let mut resting: Option<wait_ui::Rest> = None;
    let mut hazard_prompt: Option<(Command, String)> = None;
    let mut condition_meters = statuses_ui::ConditionMeters::default();
    let mut meter_identity = String::new();
    let mut meter_tick = 0.;
    let mut terminal_record_key: Option<String> = None;
    let mut record_warning = String::new();
    let mut summary: Option<corpse_core::profile::RunSummary> = None;
    let mut summary_scroll = 0usize;
    let mut death_pause=death_pause::DeathPause::default();
    let mut health_warning=health_warning::Warning::default();
    let mut tile_report: Option<inspection_ui::Report> = None;
    let mut tile_report_scroll = 0usize;
    let mut inspecting = false;
    let mut inspected_actor = None;
    let mut creature_offset = 0usize;
    let mut player_status_offset = 0usize;
    let mut chronicle_offset = 0usize;
    let mut menu_selected = 0usize;
    let menu_fast_mode = true;
    let mut menu_seed_focus = false;
    let mut observed_seed = seed.clone();
    let mut menu_visits = menu_visit::Visits::default();
    let mut display_form: Option<display_ui::DisplaySettings> = None;
    let mut choice_selected = 0usize;
    let mut debug_loadout = debug_floor_ui::View::default();
    let mut reduced = settings.reduced_motion;
    let mut history: Option<Vec<String>> = None;
    let mut shutter = chronicle_overlay::Shutter::default();
    let mut ability_offset = 0usize;
    let mut pending_ability: Option<String> = None;
    let mut implant_attacker: Option<f64> = None;
    let mut riptide_target = targeting_ui::RiptideTarget::default();
    let mut cached_view: Option<render::View> = None;
    let mut hallucination_phase=0usize;
    // Separate opt-in prototype. Unset/0 remains the frozen full-paint policy.
    let tile_requested=release_policy::DEV_TOOLS && std::env::var("CORPSE_TILE_PAINT").as_deref()==Ok("1");
    let mut tile_plan=render::tile_damage::TilePlanner::default();
    tile_plan.eligibility.enabled=release_policy::DEV_TOOLS && std::env::var("CORPSE_TILE_COUNTS").as_deref()==Ok("1");
    let mut tile_overlays=tile_overlay::Tracker::default();
    let mut tile_panels=None;
    let mut tile_notice=String::new();
    let mut force_texture_upload=true;
    let mut force_present=true;
    let mut autosave = session_store::Autosave::default();
    let mut rejected_drawable=None;
    let mut resize_refusal_notice=None;
    let mut minimized=false;
    let mut menu_help_hover = false;
    'running: loop {
        if let Some(error)=pending_gpu_failure.take(){
            eprintln!("GPU drawing failed; recreating a software renderer: {error}");
            // Drop every GPU resource before replacing its SDL context.
            // Keep the same window and game session; invalidate all paint state.
            painter.gpu=None;
            drop(gpu_renderer);drop(texture);drop(creator);
            let window=canvas.window().clone();drop(canvas);
            (canvas,gpu_available)=gpu_render::create_canvas(window,true)?;
            creator=canvas.texture_creator();
            texture=creator.create_texture_streaming(PixelFormatEnum::RGBA32,painter.width as u32,painter.height as u32).map_err(|e|e.to_string())?;
            gpu_renderer=gpu_render::Renderer::new(&creator,canvas.info().max_texture_width.min(canvas.info().max_texture_height));
            gpu_failed=true;tile_plan.invalidate();tile_overlays.invalidate();tile_panels=None;
            force_texture_upload=true;force_present=true;dirty=true;cosmetic_dirty=true;
        }
        painter.set_responsive_canvas(matches!(screen,Screen::Game|Screen::ReplayGame));
        let (drawable_w, drawable_h) = canvas.output_size()?;
        if !minimized&&drawable_w>0&&drawable_h>0&&rejected_drawable!=Some((drawable_w,drawable_h)) {
            let info=canvas.info();
            let resized=(||->Result<bool,String>{
                if (info.max_texture_width>0&&drawable_w>info.max_texture_width)||(info.max_texture_height>0&&drawable_h>info.max_texture_height){return Err("Window exceeds renderer texture limits; resize smaller".into())}
                let Some(plan)=painter.prepare_resize(drawable_w as usize,drawable_h as usize)? else{return Ok(false)};
                let replacement=creator.create_texture_streaming(PixelFormatEnum::RGBA32,drawable_w,drawable_h).map_err(|e|format!("Cannot resize display texture: {e}; resize smaller"))?;
                painter.commit_resize(plan);texture=replacement;Ok(true)
            })();
            if resized.is_ok(){
                rejected_drawable=None;
                cosmetic_dirty|=clear_resize_refusal(&mut notice,&mut resize_refusal_notice);
            }
            match resized{
                Ok(true)=>{
                    if screen==Screen::Game && !keyboard_aim && !keyboard_cursor.active() {
                        let mouse=events.mouse_state();
                        let raw=canvas.window().has_mouse_focus().then_some((mouse.x(),mouse.y()));
                        let position=resize_hover_position(&painter,raw,canvas.window().size(),resize_hover_blocked(run.as_ref().map(|r| &r.run),inventory||inspecting||history.is_some()||ability_detail.is_some()||hazard_prompt.is_some()),tile_report.is_some());
                        pending_hover_raw=Some(raw.filter(|_|position.is_some()));
                        if let Some(r)=run.as_ref(){defer_hover_position(&painter,&r.run,position,&mut pending_hover_position,&mut hover_tile,&mut hover_path,&mut hover_report);}
                    }
                    tile_plan.invalidate();tile_overlays.invalidate();tile_panels=None;force_texture_upload=true;force_present=true;dirty=true;},
                Ok(false)=>{},
                Err(error)=>{rejected_drawable=Some((drawable_w,drawable_h));resize_refusal_notice=Some(error.clone());notice=error;cosmetic_dirty=true;force_present=true;},
            }
        }
        if rejected_drawable.is_some_and(|size|size!=(drawable_w,drawable_h)){rejected_drawable=None;}
        // Use committed geometry even if this requested resize was rejected.
        let mut tile_enabled=!help.active()&&!gpu_available&&!gpu_failed&&tile_requested&&(painter.width as u64*HEIGHT as u64==painter.height as u64*WIDTH as u64);
        if health_warning.observe(run.as_ref().filter(|r|screen==Screen::Game&&r.run.status=="playing").map(|r|(r.run.player.hp,r.run.player.max_hp))){dirty=true;tile_plan.invalidate();tile_overlays.invalidate();force_texture_upload=true;}
        #[cfg(test)] input_loop_tests::inject(&sdl,&mut canvas,&painter)?;
        for mut event in events.poll_iter() {
            effects.gameplay_pause.release(&event);health_warning.release(&event);
            // Never expose final-state inspection or admit another turn while a
            // detached action sequence is still being presented. Window/quit
            // events remain responsive; held/queued gameplay input is discarded.
            if screen == Screen::Game && effects.presentation_pending(start.elapsed().as_millis() as f64)
                && !matches!(event, Event::Quit { .. } | Event::Window { .. }) { continue; }
            if health_warning.observe(run.as_ref().filter(|r|screen==Screen::Game&&r.run.status=="playing").map(|r|(r.run.player.hp,r.run.player.max_hp))){dirty=true;tile_plan.invalidate();tile_overlays.invalidate();force_texture_upload=true;}
            effects.gameplay_pause.observe(run.as_ref().filter(|_|screen==Screen::Game).map(|r|&r.run));
            if health_warning.active()&&!effects.gameplay_pause.fall_active() {
                effects.gameplay_pause.stop_waiting();
                match health_warning.event(&event){
                    health_warning::Decision::Dismiss=>{dirty=true;tile_plan.invalidate();tile_overlays.invalidate();force_texture_upload=true;continue;},
                    health_warning::Decision::Block=>continue,
                    health_warning::Decision::Pass=>{},
                }
            }
            match effects.gameplay_pause.event(&event) {
                gameplay_pause::Decision::Continue(command)=>{
                    if let (Some(command),Some(r))=(command,run.as_mut()) {
                        notice=match dispatch(r,&mut effects,start.elapsed().as_millis() as f64,&command){Ok(_)=>String::new(),Err(e)=>e.to_string()};
                    }
                    dirty=true;tile_plan.invalidate();tile_overlays.invalidate();force_texture_upload=true;continue;
                },
                gameplay_pause::Decision::Block=>continue,
                gameplay_pause::Decision::Pass=>{},
            }
            let terminal_status=run.as_ref().filter(|_|screen==Screen::Game).map(|r|r.run.status.as_str()).unwrap_or("playing");
            match death_pause.event_for_status(terminal_status,&event) {
                death_pause::Decision::Advance=>{summary=run.as_ref().map(|r|corpse_core::profile::summarize_run(&r.run));summary_scroll=0;screen=Screen::Summary;dirty=true;continue;},
                death_pause::Decision::Block=>continue,
                death_pause::Decision::Pass=>{},
            }
            match health_warning.event(&event){
                health_warning::Decision::Dismiss=>{dirty=true;tile_plan.invalidate();tile_overlays.invalidate();force_texture_upload=true;continue;},
                health_warning::Decision::Block=>continue,
                health_warning::Decision::Pass=>{},
            }
            // A held X is one press, not preview confirmation or cancellation.
            if matches!(event, Event::KeyDown { keycode: Some(Keycode::X), repeat: true, .. }) { continue; }
            let cancel_explore = cancels_autoexplore(&event);
            let was_exploring = cancel_explore && (autoexplore.path().is_some()
                || travel.as_ref().is_some_and(|(walk, _)| walk.exploring()));
            if cancel_explore {
                if autoexplore.cancel() { dirty = true; }
                if travel.as_ref().is_some_and(|(walk, _)| walk.exploring()) { travel = None; dirty = true; }
            }
            if observed_seed!=seed {avatar_selection.sync_seed(&seed);observed_seed=seed.clone();}
            dirty |= menu_visits.observe(screen,pending_start.busy(),&mut avatar_selection,&mut seed,&mut notice);
            if screen!=Screen::Menu {avatar_selection.close();}
            dirty|=cancel_aim_for_context(screen!=Screen::Game||inventory||inspecting||history.is_some()||ability_detail.is_some()||tile_report.is_some()||hazard_prompt.is_some(),
                &mut pending_item,&mut pending_ability,&mut implant_attacker,&mut keyboard_aim);
            painter.set_responsive_canvas(matches!(screen,Screen::Game|Screen::ReplayGame));
            // Keep both spaces: game clicks outside a modal still target the
            // responsive map/toolbar; only the authored sheet consumes sheet input.
            let raw_pointer=match &event{Event::MouseMotion{x,y,..}|Event::MouseButtonDown{x,y,..}|Event::MouseButtonUp{x,y,..}=>Some((*x,*y)),_=>None};
            let sheet_pointer=match &event{
                Event::MouseMotion{x,y,..}|Event::MouseButtonDown{x,y,..}|Event::MouseButtonUp{x,y,..}=>Some(painter.logical_point_for_sheet(*x,*y,canvas.window().size())),
                Event::MouseWheel{mouse_x,mouse_y,..}=>Some(painter.logical_point_for_sheet(*mouse_x,*mouse_y,canvas.window().size())),_=>None};
            let responsive_input=matches!(screen,Screen::Game|Screen::ReplayGame);
            // Observe transitions between queued events as well as between frames.
            if pending_start.busy() && !matches!(screen,Screen::Start|Screen::Menu) { pending_start.cancel(); }
            // SDL events use window points, while the framebuffer uses drawable pixels.
            match &mut event {
                Event::MouseMotion { x, y, .. }
                | Event::MouseButtonDown { x, y, .. }
                | Event::MouseButtonUp { x, y, .. } => {
                    (*x, *y) = painter.logical_point_for_canvas(*x,*y,canvas.window().size(),responsive_input);
                }
                Event::MouseWheel {
                    mouse_x, mouse_y, ..
                } => {
                    (*mouse_x, *mouse_y) =
                        painter.logical_point_for_canvas(*mouse_x,*mouse_y,canvas.window().size(),responsive_input);
                }
                _ => {}
            }
            // Help owns input before aiming, modal shortcuts or command dispatch.
            // A close click/key is consumed, never forwarded to the underlying view.
            let help_available = (screen == Screen::Menu && !pending_start.busy()
                && !avatar_selection.active() && !menu_seed_focus)
                || (screen == Screen::Game && !inventory && !inspecting && history.is_none()
                    && ability_detail.is_none() && tile_report.is_none() && hazard_prompt.is_none()
                    && run.as_ref().is_some_and(|r| r.run.offers.is_empty() && !debug_loadout.active(&r.run)));
            if matches!(event, Event::Window { win_event: sdl2::event::WindowEvent::Leave | sdl2::event::WindowEvent::FocusLost, .. })
                && painter.toolbar_focus == Some(help_ui::ACTION) {
                painter.toolbar_focus = None;
                cosmetic_dirty = true;
            }
            let next_help_hover = match &event {
                Event::MouseMotion { x, y, .. } => screen == Screen::Menu && help_available
                    && help_ui::menu_button_at(*x, *y),
                Event::Window { win_event: sdl2::event::WindowEvent::Leave | sdl2::event::WindowEvent::FocusLost, .. } => false,
                _ => menu_help_hover && screen == Screen::Menu && help_available,
            };
            cosmetic_dirty |= menu_help_hover != next_help_hover;
            menu_help_hover = next_help_hover;
            let help_button = match &event {
                Event::MouseButtonDown { x, y, .. } => match screen {
                    Screen::Menu => help_ui::menu_button_at(*x, *y),
                    Screen::Game => painter.toolbar_action(*x, *y) == Some(help_ui::ACTION),
                    _ => false,
                },
                _ => false,
            };
            match help.event(&event, &painter, help_available, help_button) {
                help_ui::Response::Opened | help_ui::Response::Closed => {
                    resting = None; travel = None; autoexplore.cancel();
                    pending_hover_position = None; pending_hover_raw = None;
                    hover_path.clear(); hover_report = None;
                    if let Some(view) = cached_view.as_mut() { view.path.clear(); }
                    tile_plan.invalidate(); tile_overlays.invalidate(); tile_panels = None;
                    tile_enabled = false; // Also repaint the opening/closing frame in full.
                    force_texture_upload = true; cosmetic_dirty = true; continue;
                },
                help_ui::Response::Consumed => { cosmetic_dirty = true; continue; },
                help_ui::Response::Ignored => {},
            }
            if pending_item.is_none()&&pending_ability.is_none(){keyboard_aim=false;}
            let cursor_available = cursor_context_available(screen,
                inventory||inspecting||history.is_some()||ability_detail.is_some()||tile_report.is_some()||hazard_prompt.is_some(),
                run.as_ref().map(|r| &r.run),run.as_ref().is_some_and(|r|debug_loadout.active(&r.run)));
            if !cursor_available || pending_item.is_some() || pending_ability.is_some() {
                keyboard_cursor.clear();
            }
            if matches!(event,Event::MouseMotion{..}|Event::MouseButtonDown{..}
                |Event::Window{win_event:sdl2::event::WindowEvent::FocusLost,..}) {
                keyboard_aim=false;keyboard_cursor.clear();
            }
            if cursor_available && pending_item.is_none() && pending_ability.is_none() {
                if let Event::KeyDown{keycode:Some(key),keymod,repeat,timestamp,window_id,..}=&event {
                    if let Some(r)=run.as_ref() {
                        if let Some(grid)=cursor_grid(&r.run) {
                            match keyboard_cursor.key(*key,*keymod,*repeat,grid,hover_tile) {
                                cursor_ui::Action::Inspect(tile) => {
                                    resting=None;travel=None;autoexplore.cancel();hover_tile=Some(tile);
                                    hover_path.clear();hover_report=None;pending_hover_position=None;pending_hover_raw=None;
                                    dirty=true;continue;
                                },
                                cursor_ui::Action::Confirm(tile) => {
                                    if let Some(click)=cursor_click(&painter,grid,tile,*timestamp,*window_id) {event=click;}
                                    else {continue;}
                                },
                                cursor_ui::Action::Exit => {
                                    resting=None;travel=None;hover_tile=None;hover_path.clear();hover_report=None;
                                    pending_hover_position=None;pending_hover_raw=None;dirty=true;continue;
                                },
                                cursor_ui::Action::Consumed => continue,
                                cursor_ui::Action::Pass => {},
                            }
                        }
                    }
                }
            }
            if screen==Screen::Game&&!inventory&&!inspecting&&history.is_none()&&ability_detail.is_none()
                &&tile_report.is_none()&&hazard_prompt.is_none()&&(pending_item.is_some()||pending_ability.is_some()) {
                if let Event::KeyDown{keycode:Some(key),keymod,timestamp,window_id,..}=&event {
                    if !aim_modifiers_allowed(*keymod) { /* Preserve fullscreen/system shortcuts. */ }
                    else if let Some(r)=run.as_ref() {
                        if *key==Keycode::Z {continue;}
                        if matches!(*key,Keycode::Return|Keycode::KpEnter)
                            && matches!(event,Event::KeyDown{repeat:true,..}) {continue;}
                        if let Some(next)=directional_zap_ui::keyboard_tile(&r.run,pending_item.as_ref(),hover_tile,keyboard_aim,*key) {
                            keyboard_aim=true;resting=None;travel=None;hover_tile=Some(next);hover_path.clear();hover_report=None;
                            pending_hover_position=None;pending_hover_raw=None;dirty=true;continue;
                        }
                        if matches!(*key,Keycode::Return|Keycode::KpEnter) {
                            if let Some((tx,ty))=hover_tile {
                                // General cursor and targeting share the exact map-click
                                // admission/dispatch branch and displayed-plane geometry.
                                if let Some(click)=cursor_grid(&r.run).and_then(|grid|cursor_click(&painter,grid,(tx,ty),*timestamp,*window_id)) {
                                    event=click;
                                } else {continue;}
                            } else {continue;}
                        }
                    }
                }
            }
            if let Some(r) = run.as_ref() {
                if let Err(e) =
                    autosave.flush_tracked(std::path::Path::new(&save_path), r, session_kind, false)
                {
                    notice = format!("Save failed: {e}. Export your run before closing.");
                    dirty = true;
                }
            }
            if let Some(pack) = pack_view.as_mut() {
                pack.configure(
                    &painter,
                    painter.frame().map.tuple(),
                );
            }
            if screen==Screen::Game && history.is_some(){
                match &event {
                    Event::MouseButtonDown{x,y,..}=>{if chronicle_overlay::header_hit_painter(&painter,*x,*y){shutter.close(start.elapsed().as_millis()as f64,reduced);}cosmetic_dirty=true;continue;},
                    Event::MouseWheel{y,..}=>{shutter.scroll_for_height(*y,history.as_ref().unwrap().len(),painter.logical_height());cosmetic_dirty=true;continue;},
                    Event::MouseMotion{..}=>continue,
                    _=>{},
                }
            }
            match event {
                Event::Quit { .. } => {
                    if let Some(r) = run.as_ref() {
                        if let Err(e) =
                            autosave.flush_tracked(std::path::Path::new(&save_path), r, session_kind, true)
                        {
                            notice = format!("Save failed: {e}. Export your run before closing.");
                            dirty = true;
                            continue;
                        }
                    }
                    break 'running;
                }
                Event::KeyDown { keycode: None, .. } => {
                    resting = None;
                    travel = None;
                }
                Event::KeyDown {
                    keycode: Some(key),
                    keymod,
                    repeat,
                    ..
                } => {
                    let key=if key==Keycode::KpEnter {Keycode::Return}else{key};
                    travel = None;
                    if screen==Screen::Game && history.is_some(){
                        match key{Keycode::Escape=>shutter.close(start.elapsed().as_millis()as f64,reduced),Keycode::Up=>shutter.scroll_for_height(1,history.as_ref().unwrap().len(),painter.logical_height()),Keycode::Down=>shutter.scroll_for_height(-1,history.as_ref().unwrap().len(),painter.logical_height()),Keycode::PageUp=>shutter.scroll_for_height(20,history.as_ref().unwrap().len(),painter.logical_height()),Keycode::PageDown=>shutter.scroll_for_height(-20,history.as_ref().unwrap().len(),painter.logical_height()),_=>{}}
                        cosmetic_dirty=true;continue;
                    }
                    if repeat
                        && !(screen == Screen::Game
                            && !inventory
                            && !inspecting
                            && ability_detail.is_none()
                            && history.is_none()
                            && matches!(
                                key_command(key),
                                Some(Command::Move { .. } | Command::Wait)
                            ))
                    {
                        continue;
                    }
                    dirty = true;
                    if screen == Screen::Menu && pending_start.busy() {
                        if key==Keycode::Escape{pending_start.cancel();notice="Play preparation cancelled.".into();}
                        continue;
                    }
                    if screen == Screen::Menu && avatar_selection.active() && !(key==Keycode::Return && keymod.intersects(Mod::LALTMOD|Mod::RALTMOD)) {
                        avatar_selection.key(key,keymod,&mut seed);
                        continue;
                    }
                    if screen == Screen::Menu && menu_seed_focus && !(key==Keycode::Return && keymod.intersects(Mod::LALTMOD|Mod::RALTMOD)) {
                        match key {
                            Keycode::Escape=>{menu_seed_focus=false;menu_selected=0;},
                            Keycode::Tab=>{menu_seed_focus=false;menu_selected=menu_ui::next_item(menu_ui::SEED_INDEX,!keymod.intersects(Mod::LSHIFTMOD|Mod::RSHIFTMOD),can_continue(&run,&saved_run));},
                            Keycode::Down|Keycode::Up=>{menu_seed_focus=false;menu_selected=menu_ui::next_item(menu_ui::SEED_INDEX,key==Keycode::Down,can_continue(&run,&saved_run));},
                            Keycode::Backspace=>{seed.pop();},
                            Keycode::A if keymod.intersects(Mod::LCTRLMOD|Mod::RCTRLMOD)=>seed.clear(),
                            Keycode::V if keymod.intersects(Mod::LCTRLMOD|Mod::RCTRLMOD)=>match video.clipboard().clipboard_text(){Ok(text)=>{if !menu_ui::edit_seed(&mut seed,&text,true){notice="Seed is too long or contains control characters.".into();}},Err(e)=>notice=e},
                            Keycode::Return|Keycode::KpEnter=>{playtest_floor=None;begin_selected_descent_with_fast(&mut pending_start,&seed,&avatar_selection,None,menu_fast_mode,&mut notice);},
                            _=>{},
                        }
                        continue;
                    }
                    if screen == Screen::Game && !inventory && ability_detail.is_none() && history.is_none() {
                        let shift = keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD);
                        painter.toolbar_focus = match key {
                            Keycode::I => Some(4),
                            Keycode::M => Some(7),
                            Keycode::Slash if shift => Some(help_ui::ACTION),
                            _ => None,
                        };
                    }
                    // Display control stays available without a settings screen.
                    if key == Keycode::Return && keymod.intersects(Mod::LALTMOD | Mod::RALTMOD) {
                        settings.notice.clear();
                        apply_settings(appearance_ui::Action::Fullscreen, &mut settings,
                            &mut canvas, &video, &mut painter, &mut reduced)?;
                        tile_plan.invalidate();tile_overlays.invalidate();tile_panels=None;
                        notice = settings.notice.clone();
                        display_form = None;
                        continue;
                    }
                    if screen == Screen::Settings {
                        if let Some(form) = display_form.as_mut() {
                            let action = form.key(key);
                            if apply_basic_display(action, form, &mut settings, &mut canvas, &video) {
                                screen = Screen::Menu;
                                display_form = None;
                            }
                        }
                        continue;
                    }
                    if screen == Screen::Game {
                        if let Some(r) = run.as_mut().filter(|r| release_policy::DEV_TOOLS && corpse_core::debug_floor::state(&r.run).is_some()) {
                            if key == Keycode::F7 {
                                let path=format!("{save_path}.debug.json");
                                notice=r.save().map_err(|e|e.to_string()).and_then(|raw|session_store::atomic_write(std::path::Path::new(&path),raw.as_bytes())).map(|_|format!("Exported separate DEBUG state to {path}")).unwrap_or_else(|e|e);
                                debug_loadout.notice=notice.clone(); continue;
                            }
                            if key == Keycode::C && keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD) { notice=video.clipboard().set_clipboard_text(&r.run.seed).map(|_|"Exact DEBUG seed copied.".into()).unwrap_or_else(|e|e); continue; }
                            if debug_loadout.active(&r.run) {
                                debug_loadout.key(&mut r.run, key); dirty = true; continue;
                            }
                            if key == Keycode::F4 && corpse_core::debug_floor::choices_complete(&r.run) {
                                debug_loadout.open = true; inventory = false; dirty = true; continue;
                            }
                        }
                    }
                    if screen == Screen::Game
                        && run
                            .as_ref()
                            .is_some_and(|r| r.run.status == "playing" && !r.run.offers.is_empty())
                    {
                        if key == Keycode::Escape && run.as_ref().is_some_and(|r| corpse_core::debug_floor::preparing(&r.run)) { screen = Screen::Menu; continue; }
                        let r = run.as_mut().unwrap();
                        resting = None;
                        let count = r.run.offers.len();
                        choice_selected = choice_selected.min(count - 1);
                        match key {
                            Keycode::Up => choice_selected = (choice_selected + count - 1) % count,
                            Keycode::Down | Keycode::Tab => {
                                choice_selected = (choice_selected + 1) % count
                            }
                            _ => {}
                        }
                        let chosen = match key {
                            Keycode::Num1 => Some(0),
                            Keycode::Num2 => Some(1),
                            Keycode::Num3 => Some(2),
                            Keycode::Return => Some(choice_selected),
                            _ => None,
                        };
                        if let Some(id) = chosen.and_then(|i| r.run.offers.get(i)).cloned() {
                            notice = match dispatch(
                                r,
                                &mut effects,
                                start.elapsed().as_millis() as f64,
                                &Command::Choose { id },
                            ) {
                                Ok(_) => String::new(),
                                Err(e) => e.to_string(),
                            };
                            choice_selected = 0;
                        }
                        continue;
                    }
                    if hazard_prompt.is_some() {
                        if let Some(command) = future_confirmation::answer(&mut hazard_prompt, key) {
                            if matches!(&command, Command::Ability { id, .. } if id == "morae-see-the-future" || id == "numo-revert") {
                                condition_meters.reset();
                            }
                            if let Some(r) = run.as_mut() {
                                notice = match dispatch(
                                    r,
                                    &mut effects,
                                    start.elapsed().as_millis() as f64,
                                    &command,
                                ) {
                                    Ok(_) => String::new(),
                                    Err(e) => e.to_string(),
                                };
                            }
                        }
                        continue;
                    }
                    let was_resting = resting.take().is_some();
                    if (was_resting || was_exploring) && key == Keycode::Escape {
                        continue;
                    }
                    if tile_report.is_some() && screen == Screen::Game {
                        match key {
                            Keycode::Escape => tile_report = None,
                            Keycode::Return | Keycode::Space => {
                                if let Some(report) = tile_report.as_mut() {
                                    inspection_ui::toggle_details(report);
                                }
                            }
                            Keycode::Down | Keycode::PageDown => tile_report_scroll += 1,
                            Keycode::Up | Keycode::PageUp => {
                                tile_report_scroll = tile_report_scroll.saturating_sub(1)
                            }
                            _ => {}
                        }
                        continue;
                    }
                    if key == Keycode::Escape {
                        if pending_start.busy() && matches!(screen,Screen::Start|Screen::Menu) {
                            pending_start.cancel();
                            screen = Screen::Menu;
                            notice = "Play preparation cancelled.".into();
                            continue;
                        }
                        if screen == Screen::Manual {
                            screen = previous;
                            continue;
                        }
                        if pending_item.take().is_some() {
                            inventory=run.as_ref().is_some_and(|r|r.run.status=="playing");
                            notice.clear();
                            continue;
                        }
                        if pending_ability.take().is_some() {
                            implant_attacker = None; riptide_target.cancel();
                            notice.clear();
                            continue;
                        }
                        if ability_detail.take().is_some() {
                            continue;
                        }
                        if inspecting {
                            inspecting = false;
                            continue;
                        }
                        if inventory {
                            if let (Some(v), Some(r)) = (pack_view.as_mut(), run.as_mut()) {
                                let action = v.key(key, keymod);
                                apply_pack_action(
                                    action,
                                    v,
                                    r,
                                    &mut effects,
                                    start.elapsed().as_millis() as f64,
                                    &mut inventory,
                                    &mut pending_item,
                                    &mut notice,
                                );
                            } else {
                                inventory = false;
                            }
                            continue;
                        }
                        // With no panel or targeting mode to cancel, keep the
                        // live run open. The toolbar Menu button still leaves it.
                        if screen == Screen::Game && run.as_ref().is_some_and(|r|r.run.status=="playing") {
                            continue;
                        }
                        if screen != Screen::Menu {
                            screen = Screen::Menu;
                        } else {
                            if let Some(r) = run.as_ref() {
                                if let Err(e) = autosave.flush_tracked(
                                    std::path::Path::new(&save_path),
                                    r,
                                    session_kind,
                                    true,
                                ) {
                                    notice = format!(
                                        "Save failed: {e}. Export your run before closing."
                                    );
                                    dirty = true;
                                    continue;
                                }
                            }
                            break 'running;
                        }
                        continue;
                    }
                    if ability_detail.is_some() && screen == Screen::Game {
                        match key {
                            Keycode::Up => {
                                ability_detail_scroll = ability_detail_scroll.saturating_sub(1)
                            }
                            Keycode::Down => ability_detail_scroll += 1,
                            _ => {}
                        }
                        continue;
                    }
                    if inspecting && screen == Screen::Game {
                        match key {
                            Keycode::Up => creature_offset = creature_offset.saturating_sub(1),
                            Keycode::Down => creature_offset += 1,
                            _ => {}
                        }
                        continue;
                    }
                    if screen == Screen::Scores {
                        match key {
                            Keycode::Up => profiles.selected = profiles.selected.saturating_sub(1),
                            Keycode::Down => {
                                profiles.selected = (profiles.selected + 1)
                                    .min(profiles.profile.high_scores.len().saturating_sub(1))
                            }
                            Keycode::Return => profiles.detail = !profiles.detail,
                            _ => {}
                        }
                        continue;
                    }
                    if screen == Screen::ReplayGame {
                        match key {
                            Keycode::Space | Keycode::Right => {
                                replays.step(start.elapsed().as_millis() as f64)
                            }
                            Keycode::P => replays.playing = !replays.playing,
                            Keycode::R => replays.restart(),
                            _ => {}
                        }
                        continue;
                    }
                    if screen == Screen::Replays {
                        match key {
                            Keycode::Up => replays.selected = replays.selected.saturating_sub(1),
                            Keycode::Down => {
                                replays.selected = (replays.selected + 1)
                                    .min(replays.files.len().saturating_sub(1))
                            }
                            Keycode::Return => {
                                if let Some(path) = replays.files.get(replays.selected).cloned() {
                                    match replays.load(&path) {
                                        Ok(()) => screen = Screen::ReplayGame,
                                        Err(e) => replays.notice = e,
                                    }
                                }
                            }
                            _ => {}
                        }
                        continue;
                    }
                    if screen == Screen::Start {
                        if pending_start.busy() { continue; }
                        match key {
                            Keycode::Tab => {
                                if playtest_floor.is_some() {
                                    seed_focus = !seed_focus; floor_focus = !seed_focus;
                                } else { seed_focus = true; floor_focus = false; }
                            },
                            Keycode::Backspace if floor_focus => { if let Some(floor) = playtest_floor.as_mut() { floor.pop(); } },
                            Keycode::A if floor_focus && keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD) => { if let Some(floor) = playtest_floor.as_mut() { floor.clear(); } },
                            Keycode::Left | Keycode::Right if floor_focus => {
                                if let Some(floor) = playtest_floor.as_mut() {
                                    *floor = menu_ui::step_start_floor(floor,key==Keycode::Right,menu_fast_mode);
                                }
                            },
                            Keycode::Backspace if seed_focus => {
                                seed.pop();
                            }
                            Keycode::A
                                if seed_focus
                                    && keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD) =>
                            {
                                seed.clear()
                            }
                            Keycode::C if playtest_floor.is_some() && keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD) => { notice=video.clipboard().set_clipboard_text(&seed).map(|_|"Exact seed copied.".into()).unwrap_or_else(|e|e); },
                            Keycode::V if playtest_floor.is_some() && keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD) => { match video.clipboard().clipboard_text() { Ok(text) if text.len() <= 180 && !text.chars().any(char::is_control) => seed=text, Ok(_) => notice="Seed must fit 180 total bytes and contain no control characters.".into(), Err(e) => notice=e } },
                            Keycode::F3 | Keycode::F4 | Keycode::F8 if playtest_floor.is_some() => debug_floor_ui::cycle_god(&mut seed, if key == Keycode::F3 { 0 } else if key == Keycode::F4 { 1 } else { 2 }),
                            Keycode::F2 => {if let Ok(next)=avatar_selection.prepared_seed("",fresh_seed){seed=next;}},
                            Keycode::Return => {
                                if !pending_start.busy() {
                                    begin_selected_descent_with_fast(&mut pending_start,&seed,&avatar_selection,playtest_floor.clone(),menu_fast_mode,&mut notice);
                                }
                            },
                            _ => {}
                        }
                        continue;
                    }
                    if screen == Screen::Summary {
                        match key {
                            Keycode::R => terminal_record_key = None,
                            Keycode::Return => {
                                seed.clear();playtest_floor=None;screen=Screen::Menu;
                                menu_visits.observe(screen,false,&mut avatar_selection,&mut seed,&mut notice);
                                begin_selected_descent_with_fast(&mut pending_start,&seed,&avatar_selection,None,menu_fast_mode,&mut notice);
                            }
                            Keycode::Down => summary_scroll += 1,
                            Keycode::Up => summary_scroll = summary_scroll.saturating_sub(1),
                            _ => {}
                        }
                        continue;
                    }
                    if screen == Screen::Game
                        && matches!(key_command(key), Some(Command::Move { .. }))
                        && run.as_ref().is_some_and(|r| r.run.status != "playing")
                    {
                        summary = run
                            .as_ref()
                            .map(|r| corpse_core::profile::summarize_run(&r.run));
                        summary_scroll = 0;
                        screen = Screen::Summary;
                        continue;
                    }
                    if screen == Screen::ImportSave {
                        continue;
                    }
                    if screen == Screen::Menu {
                        match key {
                            Keycode::Space if menu_selected==0 && !fixture_session=>{playtest_floor=None;begin_selected_descent_with_fast(&mut pending_start,&seed,&avatar_selection,None,menu_fast_mode,&mut notice);continue;},
                            Keycode::Down | Keycode::Up | Keycode::Tab => {
                                let forward = key != Keycode::Up && !(key == Keycode::Tab && keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD));
                                menu_selected = menu_ui::next_item(menu_selected, forward, can_continue(&run, &saved_run));
                                menu_seed_focus=menu_selected==menu_ui::SEED_INDEX;
                                continue;
                            }
                            Keycode::Left | Keycode::Right if (avatar_menu::GOD_START..avatar_menu::GOD_START+3).contains(&menu_selected) => {
                                let slot=menu_selected-avatar_menu::GOD_START;
                                menu_selected=avatar_menu::GOD_START+(slot+if key==Keycode::Right{1}else{2})%3;
                                continue;
                            }
                            Keycode::Right if menu_selected == 0 => {menu_selected=avatar_menu::GOD_START;continue;}
                            Keycode::Return | Keycode::Space if menu_selected == menu_ui::FAST_INDEX => {
                                notice.clear(); continue;
                            }
                            Keycode::Return | Keycode::Space if menu_selected == menu_ui::SEED_INDEX => {menu_seed_focus=true;continue;}
                            Keycode::Return | Keycode::Space if (avatar_menu::GOD_START..avatar_menu::GOD_START+3).contains(&menu_selected) => {
                                avatar_selection.sync_seed(&seed);avatar_selection.open(menu_selected-avatar_menu::GOD_START);continue;
                            }
                            Keycode::I => {
                                screen = Screen::ImportSave;
                                notice.clear();
                                continue;
                            }
                            Keycode::C | Keycode::F9 => menu_selected = 1,
                            _ => {}
                        }
                        if (matches!(key,Keycode::Return|Keycode::Space) && menu_selected == 1)
                            || matches!(key, Keycode::C | Keycode::F9)
                        {
                            if continue_run(&mut run, &mut saved_run, &mut session_kind) {
                                if fixture_session {
                                    session_kind = session_store::Kind::Fixture;
                                }
                                provenance = if session_kind == session_store::Kind::Genuine {
                                    ""
                                } else {
                                    "Native migration test scene / incomplete port / unranked"
                                };
                                screen = Screen::Game;
                                notice.clear();
                            }
                            continue;
                        }
                        if matches!(key,Keycode::Return|Keycode::Space) && menu_selected > 1 {
                            screen = match menu_selected {
                                2 => Screen::Replays,
                                3 => Screen::Scores,
                                4 => {
                                    previous = Screen::Menu;
                                    Screen::Manual
                                }
                                5 => Screen::Settings,
                                6 if release_policy::DEV_TOOLS => { if let Ok(next)=avatar_selection.prepared_seed(&seed,fresh_seed){seed=next;} playtest_floor = Some("1".into()); seed_focus = true; floor_focus = false; Screen::Start },
                                _ => Screen::Menu
                            };
                            continue;
                        }
                    }
                    if key == Keycode::M {
                        if screen == Screen::Manual {
                            screen = previous;
                        } else {
                            previous = screen;
                            screen = Screen::Manual;
                        }
                        continue;
                    }
                    if screen == Screen::Manual {
                        match key {
                            Keycode::Right => manual.step(1),
                            Keycode::Left => manual.step(-1),
                            Keycode::PageUp => manual.page(-1),
                            Keycode::PageDown => manual.page(1),
                            Keycode::Tab => manual.step(1),
                            Keycode::Home => manual.step(-1000),
                            Keycode::End => manual.step(1000),
                            Keycode::Down => manual.step(1),
                            Keycode::Up => manual.step(-1),
                            _ => {}
                        }
                        continue;
                    }
                    if key == Keycode::Return
                        && (screen == Screen::Menu
                            || run.as_ref().is_some_and(|r| r.run.status == "dead"))
                    {
                        if !fixture_session {
                            playtest_floor = None;
                            screen=Screen::Menu;
                            menu_visits.observe(screen,pending_start.busy(),&mut avatar_selection,&mut seed,&mut notice);
                            begin_selected_descent_with_fast(&mut pending_start,&seed,&avatar_selection,None,menu_fast_mode,&mut notice);
                            continue;
                        }
                        match load(&fixture) {
                            Ok(r) => {
                                effects.clear();
                                condition_meters.reset();
                                run = Some(r);
                                pack_view = None;
                                pending_item = None;
                                inventory = false;
                                inspecting = false;
                                screen = Screen::Game;
                                notice.clear();
                            }
                            Err(e) => notice = e,
                        }
                        continue;
                    }
                    if screen != Screen::Game {
                        continue;
                    }
                    let Some(r) = run.as_mut() else {
                        continue;
                    };
                    if key == Keycode::Slash && keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD) {
                        continue; // Help is admitted before gameplay input above.
                    }
                    if key == Keycode::PageDown {
                        ability_offset = (ability_offset + ability_ui::capacity(&painter.profile))
                            .min(
                                ability_ui::visible_abilities(&r.run)
                                    .len()
                                    .saturating_sub(1),
                            );
                        continue;
                    }
                    if key == Keycode::PageUp {
                        ability_offset =
                            ability_offset.saturating_sub(ability_ui::capacity(&painter.profile));
                        continue;
                    }
                    if key == Keycode::I {
                        inventory = !inventory;
                        continue;
                    }
                    if key == Keycode::F6 {
                        if corpse_core::debug_floor::state(&r.run).is_some() { notice="DEBUG seed-only replay export is unavailable; use F7 for an explicit debug state export.".into(); continue; }
                        notice = if session_kind == session_store::Kind::Genuine {
                            (|| -> Result<String, String> {
                                let mut record = corpse_core::replay::ReplayRecord::try_from_run(&r.run).map_err(|e|e.to_string())?;
                                let date = session_store::utc_now();
                                record.saved_at = date.clone().into();
                                let path = session_store::replay_archive::write(
                                    std::path::Path::new(&replay_dir), &r.run, &date,
                                    &serde_json::to_vec_pretty(&record).map_err(|e| e.to_string())?,
                                )?;
                                replays = replays::Replays::new(std::path::Path::new(&replay_dir));
                                Ok(format!("Replay exported: {}", path.display()))
                            })()
                            .unwrap_or_else(|e| e)
                        } else {
                            "Seed-only replay export is unavailable for controlled or unknown-origin sessions.".into()
                        };
                        continue;
                    }
                    if key == Keycode::F7 {
                        if session_kind != session_store::Kind::Genuine {
                            notice = "Source-format export is unavailable for controlled sessions; F5 preserves their native provenance.".into();
                            continue;
                        }
                        let path = if corpse_core::debug_floor::state(&r.run).is_some() { format!("{save_path}.debug.json") } else { format!("{save_path}.v48.json") };
                        notice = r
                            .save()
                            .map_err(|e| e.to_string())
                            .and_then(|raw| {
                                session_store::atomic_write(
                                    std::path::Path::new(&path),
                                    raw.as_bytes(),
                                )
                            })
                            .map(|_| format!("Exported source-format save to {path}"))
                            .unwrap_or_else(|e| e);
                        continue;
                    }
                    if key == Keycode::F5 {
                        if !r.run.playtest.is_missing() { notice="DEBUG autosave is disabled; normal Continue is preserved. F7 explicitly exports a separate .debug.json file.".into(); continue; }
                        notice = match autosave.flush_tracked(
                            std::path::Path::new(&save_path),
                            r,
                            session_kind,
                            true,
                        ) {
                            Ok(()) => {
                                if r.run.status == "dead"
                                    && !session_store::unknown_seed_replay(&r.run)
                                    && !r.run.dev
                                    && session_kind == session_store::Kind::Genuine
                                {
                                    "Death recorded. No resumable save.".into()
                                } else {
                                    format!("Saved native state to {save_path}")
                                }
                            }
                            Err(e) => e,
                        };
                        continue;
                    }
                    if key == Keycode::F9 {
                        match load_with_kind(&save_path) {
                            Ok((saved, kind)) => {
                                session_kind = if fixture_session {
                                    session_store::Kind::Fixture
                                } else {
                                    kind
                                };
                                effects.clear();
                                condition_meters.reset();
                                provenance = if session_kind == session_store::Kind::Genuine {
                                    ""
                                } else {
                                    "Native migration test scene / incomplete port / unranked"
                                };
                                *r = saved;
                                pack_view = None;
                                pending_item = None;
                                inspecting = false;
                                notice =
                                    "Reloaded native save; RNG and temporal state restored.".into();
                            }
                            Err(e) => notice = e,
                        }
                        continue;
                    }
                    if key == Keycode::X && !inventory && pending_item.is_none()
                        && pending_ability.is_none() && r.run.status == "playing" && r.run.offers.is_empty()
                    {
                        travel = autoexplore.press(&r.run).map(|walk| (walk, start.elapsed().as_millis() as f64));
                        notice.clear();
                        continue;
                    }
                    if cursor_ui::rest_key(key,keymod,pending_item.is_some()||pending_ability.is_some()) == cursor_ui::RestKey::UntilChange
                        && !inventory
                        && r.run.status == "playing"
                        && r.run.offers.is_empty()
                    {
                        resting = Some(wait_ui::Rest {
                            knowledge: wait_ui::observe(&r.run),
                            next: start.elapsed().as_millis() as f64,
                        });
                        continue;
                    }
                    let mut command = if (key == Keycode::R && !corpse_core::temporal::retains_revert_history(&r.run))
                        || (key == Keycode::Z && cursor_ui::rest_key(key,keymod,pending_item.is_some()||pending_ability.is_some()) != cursor_ui::RestKey::Once)
                        { None } else { key_command(key) };
                    if inventory {
                        let view = pack_view.get_or_insert_with(|| pack::PackView::observe(&r.run));
                        view.refresh(&r.run);
                        view.configure(
                            &painter,
                            painter.frame().map.tuple(),
                        );
                        let action = view.key(key, keymod);
                        apply_pack_action(
                            action,
                            view,
                            r,
                            &mut effects,
                            start.elapsed().as_millis() as f64,
                            &mut inventory,
                            &mut pending_item,
                            &mut notice,
                        );
                        continue;
                    }
                    if !inventory {
                        if r.run.simulation_revision.value_or(29.) >= 50. && pending_ability.as_deref()==Some("kative-cogworks") {
                            let mode=match key {Keycode::Num1=>Some(1.),Keycode::Num2=>Some(2.),Keycode::Num3=>Some(3.),_=>None};
                            if let Some(mode)=mode {
                                notice=match dispatch(r,&mut effects,start.elapsed().as_millis() as f64,
                                    &Command::Ability{id:"kative-cogworks".into(),target:Field::Value(mode),x:Field::Missing,y:Field::Missing}) {
                                    Ok(_)=>targeting_ui::cogworks_prompt(&r.run),Err(e)=>e.to_string(),
                                };
                                continue;
                            }
                        }
                        let n = match key {
                            Keycode::Num1 => Some(0),
                            Keycode::Num2 => Some(1),
                            Keycode::Num3 => Some(2),
                            Keycode::Num4 => Some(3),
                            Keycode::Num5 => Some(4),
                            Keycode::Num6 => Some(5),
                            _ => None,
                        };
                        if let Some(i) = n {
                            if let Some(row) =
                                ability_ui::visible_abilities(&r.run).get(i + ability_offset)
                            {
                                if ability_ui::command_binding_allowed(&r.run, &row.id) {
                                    command = Some(Command::Ability {
                                        id: row.id.clone(), target: Field::Missing,
                                        x: Field::Missing, y: Field::Missing,
                                    });
                                }
                            }
                        }
                    }
                    if key == Keycode::Period && keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD)
                    {
                        command = Some(Command::Descend);
                    }
                    if key == Keycode::Comma && keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD) {
                        command = Some(Command::Ascend);
                    }
                    if let Some(Command::Ability { id, .. }) = &command {
                        if targeting_ui::cancel_reselected_ability(id, &mut pending_ability, &mut implant_attacker, &mut riptide_target) {
                            notice.clear();
                            continue;
                        }
                        if r.run.simulation_revision.value_or(29.) >= 50. && !ability_ui::runtime_supported(&r.run, id) { notice = "This ability is not yet available in this build.".into(); continue; }
                        if id == "kative-tinker" {
                            let view =
                                pack_view.get_or_insert_with(|| pack::PackView::observe(&r.run));
                            let action = view.begin_tinker(&r.run, id);
                            inventory = true;
                            apply_pack_action(
                                action,
                                view,
                                r,
                                &mut effects,
                                start.elapsed().as_millis() as f64,
                                &mut inventory,
                                &mut pending_item,
                                &mut notice,
                            );
                            continue;
                        }
                        if observe::ability_needs_target_for_run(&r.run, id)
                            && !ability_ui::deactivates_without_target(&r.run, id)
                        {
                            implant_attacker = None; riptide_target.cancel();pending_item=None;
                            pending_ability = Some(id.clone());
                            notice = if r.run.simulation_revision.value_or(29.) >= 50. { targeting_ui::ability_prompt(id) } else { "Select a target. Escape cancels." }.into();
                            if id=="kative-cogworks" { notice=targeting_ui::cogworks_prompt(&r.run); }
                            continue;
                        }
                    }
                    if let Some(c) = command {
                        if let Some(message) = future_confirmation::message(&r.run, &c) {
                            hazard_prompt = Some((c, message.into()));
                            continue;
                        }
                        if matches!(&c,Command::Ability{id,..} if id=="numo-revert"||id=="morae-see-the-future")
                        {
                            condition_meters.reset();
                        }
                        if let Some(message) = hazards_ui::intentional(&r.run, &c).filter(|_|!gameplay_pause::needs_ack(&r.run,&c)) {
                            hazard_prompt = Some((c, message));
                            continue;
                        }
                        notice =
                            match dispatch(r, &mut effects, start.elapsed().as_millis() as f64, &c)
                            {
                                Ok(_) => String::new(),
                                Err(e) => e.to_string(),
                            };
                    }
                }
                Event::MouseButtonDown {
                    x,
                    y,
                    mouse_btn: sdl2::mouse::MouseButton::Right,
                    ..
                } if screen == Screen::Game && !inventory && !inspecting => {
                    resting = None;
                    travel = None;
                    if hazard_prompt.is_some()
                        || tile_report.is_some()
                        || history.is_some()
                        || ability_detail.is_some()
                        || !run
                            .as_ref()
                            .is_some_and(|r| r.run.offers.is_empty() && r.run.status == "playing")
                    {
                        continue;
                    }
                    if let Some(view) = cached_view.as_ref() {
                        let (_,_,ox,_) = painter.map_layout(view.width,view.height);
                        let layout=render::enemy_list::Layout::resolve_frame(&painter.profile,ox,painter.frame());
                        if let Some(index)=layout.rows.contains(x,y).then(||layout.content_view(&painter,view).hit(x,y,painter.enemy_list_offset)).flatten() {
                            if let Some((tx,ty))=render::enemy_list::Content::ground(view,index) {
                                if let Some(r)=run.as_ref(){tile_report=Some(inspection_ui::observe_tile(&r.run,tx,ty));tile_report_scroll=0;}
                                painter.enemy_hover=None;painter.ground_hover=None;cosmetic_dirty=true;continue;
                            }
                            let Some(index)=render::enemy_list::Content::creature_index(view,index) else{continue;};
                            inspected_actor=view.creatures.get(index).map(|a|a.id);
                            creature_offset=index;
                            inspecting=true;
                            painter.enemy_hover=None;
                            // This branch is cosmetic: update the detached presentation inputs
                            // that a world rebuild would assign, without observing the Run again.
                            if let Some(view)=cached_view.as_mut(){
                                if tile_enabled{tile_overlay::mark_presentation(&painter,view,&mut tile_plan);}
                                sync_enemy_inspection_view(view,inspected_actor,&hover_path);
                                if tile_enabled{tile_overlay::mark_presentation(&painter,view,&mut tile_plan);}
                            }
                            cosmetic_dirty=true;
                            continue;
                        }
                    }
                    if cached_view.as_ref().and_then(|v| painter.ability_inspect_at(v.abilities.len(),x,y)).is_some() {
                        if let Some(r) = &run {
                            ability_detail = ability_ui::visible_abilities(&r.run)
                                .get(
                                    ability_offset
                                        + ((y - ability_ui::top(&painter.profile))
                                            / ability_ui::row(&painter.profile))
                                            as usize,
                                )
                                .map(|a| a.id.clone());
                            ability_detail_scroll = 0;
                            dirty = true;
                        }
                    } else if let (Some(r), Some(view)) = (run.as_mut(), cached_view.as_ref()) {
                        let (cw, ch, ox, oy) = painter.map_layout(view.width, view.height);
                        if x >= ox
                            && y >= oy
                            && x < ox + cw * view.width as i32
                            && y < oy + ch * view.height as i32
                        {
                            let (tx, ty) = (((x - ox) / cw) as usize, ((y - oy) / ch) as usize);
                            if let Some(cell) =
                                view.cells.iter().rev().find(|c| c.x == tx && c.y == ty)
                            {
                                let command = if cell.controllable_ally
                                    && view
                                        .cells
                                        .iter()
                                        .any(|c| c.x == tx && c.y == ty && c.current)
                                {
                                    Command::Ally {
                                        id: cell.actor_id.unwrap(),
                                    }
                                } else {
                                    Command::Rally {
                                        x: tx as f64,
                                        y: ty as f64,
                                    }
                                };
                                notice = dispatch(
                                    r,
                                    &mut effects,
                                    start.elapsed().as_millis() as f64,
                                    &command,
                                )
                                .map(|_| String::new())
                                .unwrap_or_else(|e| e.to_string());
                                dirty = true;
                            }
                        }
                    }
                }
                Event::MouseButtonDown {
                    x,
                    y,
                    mouse_btn: sdl2::mouse::MouseButton::Left,
                    ..
                } => {
                    dirty = true;
                    resting = None;
                    travel = None;
                    if hazard_prompt.is_some() {
                        continue;
                    }
                    if screen == Screen::Settings {
                        if let Some(form) = display_form.as_mut() {
                            let action = form.click(x, y);
                            if apply_basic_display(action, form, &mut settings, &mut canvas, &video) {
                                screen = Screen::Menu;
                                display_form = None;
                            }
                        }
                        continue;
                    }
                    if screen == Screen::Summary {
                        if (690..740).contains(&y) {
                            if (180..610).contains(&x) {
                                seed.clear();playtest_floor=None;screen=Screen::Menu;
                                menu_visits.observe(screen,false,&mut avatar_selection,&mut seed,&mut notice);
                                begin_selected_descent_with_fast(&mut pending_start,&seed,&avatar_selection,None,menu_fast_mode,&mut notice);
                            } else if (710..1100).contains(&x) {
                                screen = Screen::Menu;
                            }
                        }
                        continue;
                    }
                    if screen == Screen::Menu {
                        if pending_start.busy(){continue;}
                        if avatar_selection.active(){avatar_selection.click(x,y,&mut seed);continue;}
                        let Some(i) = menu_item_at(x, y) else {
                            continue;
                        };
                        menu_selected = i;
                        menu_seed_focus=i==menu_ui::SEED_INDEX;
                        match i {
                            menu_ui::FAST_INDEX => { notice.clear(); },
                            0 if !fixture_session => { playtest_floor=None;begin_selected_descent_with_fast(&mut pending_start,&seed,&avatar_selection,None,menu_fast_mode,&mut notice); },
                            0 => match load(&fixture) {
                                Ok(r) => {
                                    effects.clear();
                                    condition_meters.reset();
                                    run = Some(r);
                                    pack_view = None;
                                    pending_item = None;
                                    inventory = false;
                                    inspecting = false;
                                    screen = Screen::Game;
                                    notice.clear();
                                }
                                Err(e) => notice = e,
                            },
                            1 => {
                                if continue_run(&mut run, &mut saved_run, &mut session_kind) {
                                    if fixture_session {
                                        session_kind = session_store::Kind::Fixture;
                                    }
                                    provenance = if session_kind == session_store::Kind::Genuine {
                                        ""
                                    } else {
                                        "Native migration test scene / incomplete port / unranked"
                                    };
                                    screen = Screen::Game;
                                    notice.clear();
                                }
                            }
                            2 => screen = Screen::Replays,
                            3 => screen = Screen::Scores,
                            4 => {
                                previous = Screen::Menu;
                                screen = Screen::Manual
                            }
                            5 => screen = Screen::Settings,
                            i if (avatar_menu::GOD_START..avatar_menu::GOD_START+3).contains(&i) => {avatar_selection.sync_seed(&seed);avatar_selection.open(i-avatar_menu::GOD_START);},
                            6 if release_policy::DEV_TOOLS => { if let Ok(next)=avatar_selection.prepared_seed(&seed,fresh_seed){seed=next;} playtest_floor = Some("1".into()); seed_focus = true; floor_focus = false; screen = Screen::Start; },
                            _ => {}
                        }
                    } else if screen == Screen::Start {
                        if pending_start.busy() {
                            if (284..410).contains(&x) && (610..650).contains(&y) { pending_start.cancel(); screen = Screen::Menu; notice = "Play preparation cancelled.".into(); }
                            continue;
                        }
                        if (284..994).contains(&x) && (340..392).contains(&y) {
                            seed_focus = true; floor_focus = false;
                        } else if playtest_floor.is_some() && (450..994).contains(&x) && (475..517).contains(&y) {
                            seed_focus = false; floor_focus = true;

                        } else if (284..994).contains(&x) && (if playtest_floor.is_some() { 548..603 } else { 505..560 }).contains(&y) {
                            {
                                if !pending_start.busy() {
                                    begin_selected_descent_with_fast(&mut pending_start,&seed,&avatar_selection,playtest_floor.clone(),menu_fast_mode,&mut notice);
                                }
                            }
                        } else if (892..994).contains(&x) && (303..338).contains(&y) {
                            if let Ok(next)=avatar_selection.prepared_seed("",fresh_seed){seed=next;}
                        } else if (284..410).contains(&x) && (610..650).contains(&y) {
                            screen = Screen::Menu;
                        }
                    } else if screen == Screen::Manual {
                        manual.click(x, y);
                    } else if screen == Screen::Game {
                        if let Some(r) = run.as_mut() {
                            if debug_loadout.active(&r.run) { debug_loadout.click(&mut r.run, sheet_pointer.unwrap().0, sheet_pointer.unwrap().1); dirty = true; continue; }
                            if r.run.status == "playing" && !r.run.offers.is_empty() {
                                if let Some(i) = render::blessing_choice_at(sheet_pointer.unwrap().0,sheet_pointer.unwrap().1) {
                                    if let Some(id) = r.run.offers.get(i).cloned() {
                                        notice = match dispatch(
                                            r,
                                            &mut effects,
                                            start.elapsed().as_millis() as f64,
                                            &Command::Choose { id },
                                        ) {
                                            Ok(_) => String::new(),
                                            Err(e) => e.to_string(),
                                        };
                                        choice_selected = 0;
                                    }
                                }
                                continue;
                            }
                            if let Some(report) = tile_report.as_mut().filter(|_| {
                                let (xx, yy, w, h) = inspection_ui::bounds(&painter.profile);
                                let(x,y)=sheet_pointer.unwrap();
                                x >= xx && x < xx + w && y >= yy && y < yy + h
                            }) {
                                if inspection_ui::close_hit_profile(&painter,sheet_pointer.unwrap().0,sheet_pointer.unwrap().1) {
                                    tile_report = None;
                                } else {
                                    inspection_ui::click(
                                        report,
                                        &painter,
                                        tile_report_scroll,
                                        sheet_pointer.unwrap().0,
                                        sheet_pointer.unwrap().1,
                                    );
                                }
                                continue;
                            }
                            if history.is_some() { continue; }
                            if ability_detail.is_some() {
                                ability_detail = None;
                                continue;
                            }
                            if inventory {
                                let view = pack_view
                                    .get_or_insert_with(|| pack::PackView::observe(&r.run));
                                view.refresh(&r.run);
                                view.configure(
                                    &painter,
                                    painter.frame().map.tuple(),
                                );
                                let action = view.click(x, y);
                                apply_pack_action(
                                    action,
                                    view,
                                    r,
                                    &mut effects,
                                    start.elapsed().as_millis() as f64,
                                    &mut inventory,
                                    &mut pending_item,
                                    &mut notice,
                                );
                                continue;
                            }
                            if !inspecting && tile_report.is_none() && hazard_prompt.is_none()
                                && painter.chronicle_hit_rect().contains(x,y) {
                                history=Some(bestiary_notices.history(&r.run));shutter.open(start.elapsed().as_millis()as f64,reduced);
                                resting=None;travel=None;painter.chronicle_hover=false;
                                cosmetic_dirty=true;continue;
                            }
                            let view = observe::observe(&r.run);
                            if inspecting
                                && painter.toolbar_action(x,y).is_none()
                            {
                                if let Some(cached) = cached_view.as_ref() {
                                    inspected_actor =
                                        render::creature_at(&painter,cached,creature_offset,sheet_pointer.unwrap().0,sheet_pointer.unwrap().1)
                                            .or(inspected_actor);
                                }
                                continue;
                            }
                            if let Some(action) =
                                painter.toolbar_action(x,y)
                            {
                                painter.toolbar_focus = Some(action);
                                match action {
                                    1 => {
                                        if r.run.status == "playing" && r.run.offers.is_empty() {
                                            resting = Some(wait_ui::Rest {
                                                knowledge: wait_ui::observe(&r.run),
                                                next: start.elapsed().as_millis() as f64,
                                            });
                                        }
                                    }
                                    0 => {
                                        notice = match dispatch(
                                            r,
                                            &mut effects,
                                            start.elapsed().as_millis() as f64,
                                            &Command::Wait,
                                        ) {
                                            Ok(_) => String::new(),
                                            Err(e) => e.to_string(),
                                        };
                                    }
                                    2 => {
                                        notice = match dispatch(
                                            r,
                                            &mut effects,
                                            start.elapsed().as_millis() as f64,
                                            &Command::Descend,
                                        ) {
                                            Ok(_) => String::new(),
                                            Err(e) => e.to_string(),
                                        };
                                    }
                                    3 => {
                                        notice = match dispatch(
                                            r,
                                            &mut effects,
                                            start.elapsed().as_millis() as f64,
                                            &Command::Ascend,
                                        ) {
                                            Ok(_) => String::new(),
                                            Err(e) => e.to_string(),
                                        };
                                    }
                                    4 => inventory = !inventory,
                                    5 => {
                                        if tile_report.is_some() {
                                            tile_report = None;
                                        } else {
                                            let (x, y) = hover_tile.unwrap_or((
                                                r.run.player.x as usize,
                                                r.run.player.y as usize,
                                            ));
                                            tile_report =
                                                Some(inspection_ui::observe_tile(&r.run, x, y));
                                            tile_report_scroll = 0;
                                        }
                                        inspecting = false;
                                        inventory = false;
                                    }
                                    6 => {
                                        history = Some(bestiary_notices.history(&r.run));
                                        shutter.open(start.elapsed().as_millis()as f64,reduced);resting=None;travel=None;
                                    }
                                    7 => {
                                        previous = Screen::Game;
                                        screen = Screen::Manual
                                    }
                                    help_ui::ACTION => {}, // Help is routed before command dispatch.
                                    _ => screen = Screen::Menu,
                                }
                            } else if x >= painter.hud_left() + 10
                                && y >= painter.frame().map.1+painter.frame().map.3-22
                                && y < painter.frame().map.1+painter.frame().map.3
                            {
                                inspected_actor = view.creatures.first().map(|a| a.id);
                                inspecting = true;
                                creature_offset = 0;
                                inventory = false;
                            } else if x >= painter.hud_left() + 10
                                && y >= ability_ui::top(&painter.profile)
                                && y < ability_ui::top(&painter.profile)
                                    + ability_ui::row(&painter.profile)
                                        * ability_ui::capacity(&painter.profile) as i32
                            {
                                let i = ((y - ability_ui::top(&painter.profile))
                                    / ability_ui::row(&painter.profile))
                                    as usize;
                                if let Some(id) = ability_ui::visible_abilities(&r.run)
                                    .get(i + ability_offset)
                                    .map(|a| a.id.clone())
                                {
                                    if !ability_ui::command_binding_allowed(&r.run, &id) { continue; }
                                    if targeting_ui::cancel_reselected_ability(&id, &mut pending_ability, &mut implant_attacker, &mut riptide_target) {
                                        notice.clear();
                                        continue;
                                    }
                                    if id=="kative-cogworks" {
                                        if let Some(mode)=cogworks_ui::bubble_hit(x,y,painter.hud_left(),ability_ui::top(&painter.profile)+i as i32*ability_ui::row(&painter.profile)) {
                                            notice=match dispatch(r,&mut effects,start.elapsed().as_millis() as f64,&Command::Ability{id:id.clone(),target:Field::Value(mode as f64),x:Field::Missing,y:Field::Missing}){Ok(_)=>targeting_ui::cogworks_prompt(&r.run),Err(e)=>e.to_string()};
                                        }
                                    }
                                    if id == "kative-tinker" {
                                        let pv = pack_view
                                            .get_or_insert_with(|| pack::PackView::observe(&r.run));
                                        let action = pv.begin_tinker(&r.run, &id);
                                        inventory = true;
                                        apply_pack_action(
                                            action,
                                            pv,
                                            r,
                                            &mut effects,
                                            start.elapsed().as_millis() as f64,
                                            &mut inventory,
                                            &mut pending_item,
                                            &mut notice,
                                        );
                                        continue;
                                    }
                                    if observe::ability_needs_target_for_run(&r.run, &id)
                                        && !ability_ui::deactivates_without_target(&r.run, &id)
                                    {
                                        implant_attacker = None; riptide_target.cancel();pending_item=None;
                                        notice = if r.run.simulation_revision.value_or(29.) >= 50. { targeting_ui::ability_prompt(&id) } else { "Select a target. Escape cancels." }.into();
                                        if id=="kative-cogworks" { notice=targeting_ui::cogworks_prompt(&r.run); }
                                        pending_ability = Some(id);
                                        continue;
                                    }
                                    let command = Command::Ability { id: id.clone(), target: Field::Missing, x: Field::Missing, y: Field::Missing };
                                    if let Some(message) = future_confirmation::message(&r.run, &command) {
                                        hazard_prompt = Some((command, message.into()));
                                        continue;
                                    }
                                    if id == "numo-revert" || id == "morae-see-the-future" {
                                        condition_meters.reset();
                                    }
                                    notice = match dispatch(
                                        r,
                                        &mut effects,
                                        start.elapsed().as_millis() as f64,
                                        &Command::Ability {
                                            id,
                                            target: Field::Missing,
                                            x: Field::Missing,
                                            y: Field::Missing,
                                        },
                                    ) {
                                        Ok(_) => String::new(),
                                        Err(e) => e.to_string(),
                                    };
                                }
                            } else {
                                let (cw, ch, ox, oy) = painter.map_layout(view.width, view.height);
                                if x >= ox
                                    && y >= oy
                                    && x < ox + cw * view.width as i32
                                    && y < oy + ch * view.height as i32
                                {
                                    let (tx, ty) =
                                        (((x - ox) / cw) as usize, ((y - oy) / ch) as usize);
                                    if let Some(target) = pending_item.take() {
                                        match directional_zap_ui::command(&r.run,&view,&target,(tx,ty)) {
                                            Ok(command) => {
                                                let outcome=dispatch(r,&mut effects,start.elapsed().as_millis() as f64,&command);
                                                notice=finish_pack_dispatch(&mut inventory,&r.run.status,outcome);
                                                if let Some(pack)=pack_view.as_mut(){pack.refresh(&r.run);}
                                            },
                                            Err(message) => {notice=message.into();inventory=r.run.status=="playing";},
                                        }
                                    } else if let Some(id) = pending_ability.take() {
                                        let target = view
                                            .cells
                                            .iter()
                                            .rev()
                                            .filter(|c| c.x == tx && c.y == ty)
                                            .filter_map(|c| c.actor_id)
                                            .find(|id| corpse_core::engine::actor(&r.run, *id)
                                                .is_some_and(|a| corpse_core::dreamwalking::same_plane(&r.run, &r.run.player, a)))
                                            .map(Field::Value)
                                            .unwrap_or(Field::Missing);
                                        let target = if id == "etill-riptide" {
                                            match riptide_target.select(&r.run, target.as_ref().copied(), tx, ty) {
                                                Ok(Some(actor)) => Field::Value(actor),
                                                Ok(None) => { pending_ability=Some(id); notice="Select the rally point. Escape cancels.".into(); continue; },
                                                Err(reason) => { pending_ability=Some(id); notice=reason.into(); continue; },
                                            }
                                        } else { target };
                                        let target = if r.run.simulation_revision.value_or(29.) >= 50. && id == "numo-implant-thought" {
                                            if let Some(attacker) = implant_attacker {
                                                if let Some(reason) = targeting_ui::implant_victim_blocker(&r.run, attacker, tx, ty) {
                                                    pending_ability = Some(id); notice = reason.into(); continue;
                                                }
                                                implant_attacker = None; riptide_target.cancel();
                                                Field::Value(attacker)
                                            } else {
                                                let attacker = target.as_ref().copied();
                                                if !attacker.is_some_and(|actor| targeting_ui::visible_enemy(&r.run, actor)) {
                                                    pending_ability = Some(id); notice = "Select a visible enemy first.".into(); continue;
                                                }
                                                implant_attacker = attacker;
                                                pending_ability = Some(id);
                                                notice = "Now select a creature adjacent to that enemy. Escape cancels.".into(); continue;
                                            }
                                        } else { target };
                                        if r.run.simulation_revision.value_or(29.) >= 50. && id == "pelian-fated-duel" {
                                            if !target.as_ref().is_some_and(|actor| targeting_ui::adjacent_enemy(&r.run, *actor)) {
                                                pending_ability = Some(id); notice = "Select a visible adjacent enemy.".into(); continue;
                                            }
                                        }
                                        if r.run.simulation_revision.value_or(29.) >= 50. && id == "pelian-relentless-pursuit"
                                            && !target.as_ref().is_some_and(|actor| targeting_ui::visible_enemy(&r.run, *actor)) {
                                            pending_ability = Some(id); notice = "Select a visible enemy to pursue.".into(); continue;
                                        }
                                        if r.run.simulation_revision.value_or(29.) >= 50. && id == "ursa-waking-nightmares" {
                                            if let Some(reason)=targeting_ui::nightmare_blocker(&r.run,tx,ty) {
                                                pending_ability = Some(id); notice = reason; continue;
                                            }
                                        }
                                        if id==corpse_core::revision50_terrain::CLAY_ARMY {
                                            if let Some(reason)=targeting_ui::clay_blocker(&r.run,tx,ty) {
                                                pending_ability=Some(id);notice=reason;continue;
                                            }
                                        }
                                        let target=if id=="kative-cogworks" {
                                            if corpse_core::revision50_terrain::mode(&r.run)==2 {
                                                pending_ability=Some(id);notice=targeting_ui::cogworks_prompt(&r.run);continue;
                                            }
                                            if !corpse_core::vision::visible(&r.run,tx,ty)
                                                || corpse_core::revision50_terrain::preview_for_run(&r.run,tx as f64,ty as f64).is_err() {
                                                pending_ability=Some(id);notice="Select a visible valid placement or one of your terrain modifications.".into();continue;
                                            }
                                            Field::Missing
                                        } else if id==corpse_core::revision50_terrain::CLAY_ARMY {Field::Missing} else {target};
                                        notice = match dispatch(
                                            r,
                                            &mut effects,
                                            start.elapsed().as_millis() as f64,
                                            &Command::Ability {
                                                id,
                                                target,
                                                x: Field::Value(tx as f64),
                                                y: Field::Value(ty as f64),
                                            },
                                        ) {
                                            Ok(_) => String::new(),
                                            Err(e) => e.to_string(),
                                        };
                                    } else {
                                        if tile_report.is_some() {
                                            tile_report =
                                                Some(inspection_ui::observe_tile(&r.run, tx, ty));
                                            tile_report_scroll = 0;
                                        } else if r.run.status != "playing" {
                                            summary =
                                                Some(corpse_core::profile::summarize_run(&r.run));
                                            summary_scroll = 0;
                                            screen = Screen::Summary;
                                        } else {
                                            match travel_ui::start(&r.run, tx, ty) {
                                                travel_ui::Start::NoPath => {
                                                    notice = "No known walkable path.".into()
                                                }
                                                travel_ui::Start::Walk(t) => {
                                                    travel = Some((
                                                        t,
                                                        start.elapsed().as_millis() as f64,
                                                    ));
                                                    notice.clear();
                                                }
                                                travel_ui::Start::Direct(c) => {
                                                    if let Some(message) =
                                                        hazards_ui::intentional(&r.run, &c)
                                                            .filter(|_| !gameplay_pause::needs_ack(&r.run, &c))
                                                    {
                                                        hazard_prompt = Some((c, message));
                                                    } else {
                                                        notice = dispatch(
                                                            r,
                                                            &mut effects,
                                                            start.elapsed().as_millis() as f64,
                                                            &c,
                                                        )
                                                        .map(|_| String::new())
                                                        .unwrap_or_else(|e| e.to_string());
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Event::MouseMotion { x, y, .. } if screen == Screen::Menu && avatar_selection.active() => {
                    dirty |= avatar_selection.hover(x,y);
                }
                Event::MouseMotion { x, y, .. } if screen == Screen::Menu && !menu_seed_focus && !pending_start.busy() => {
                    if let Some(i) = menu_item_at(x, y) {
                        if i != menu_selected {
                            menu_selected = i;
                            dirty = true;
                        }
                    }
                }
                Event::MouseMotion { x, y, .. } if screen==Screen::Game && run.as_ref().is_some_and(|r|r.run.status=="playing"&&!r.run.offers.is_empty())=>{
                    let r=run.as_ref().unwrap();
                    if !debug_loadout.active(&r.run){cosmetic_dirty|=update_blessing_hover(&mut choice_selected,r.run.offers.len(),sheet_pointer.unwrap().0,sheet_pointer.unwrap().1);}
                    continue;
                }
                Event::MouseMotion { x, y, .. } if screen == Screen::Game && inventory => {
                    painter.toolbar_focus = None;
                    if let Some(view) = pack_view.as_mut() {
                        cosmetic_dirty |= view.hover(x,y);
                    }
                }
                Event::MouseMotion { x, y, .. } if screen == Screen::Game => {
                    let blocked = inventory
                        || inspecting
                        || history.is_some()
                        || ability_detail.is_some()
                        || hazard_prompt.is_some()
                        || tile_report.as_ref().is_some_and(|_| {
                            let (xx, yy, w, h) = inspection_ui::bounds(&painter.profile);
                            let(x,y)=sheet_pointer.unwrap();
                            x >= xx && x < xx + w && y >= yy && y < yy + h
                        });
                    let chronicle_hover=!blocked && painter.chronicle_hit_rect().contains(x,y)
                        && run.as_ref().is_some_and(|r|r.run.offers.is_empty());
                    if painter.chronicle_hover!=chronicle_hover {painter.chronicle_hover=chronicle_hover;cosmetic_dirty=true;}
                    if let Some(v)=cached_view.as_ref(){
                        cosmetic_dirty|=refresh_panel_hover(&mut painter,v,Some((x,y)),!blocked
                            &&run.as_ref().is_some_and(|r|r.run.offers.is_empty()&&r.run.status=="playing"));
                    }
                    let toolbar_focus = if blocked { None } else {
                        painter.toolbar_action(x,y)
                    };
                    if painter.toolbar_focus != toolbar_focus {
                        painter.toolbar_focus = toolbar_focus;
                        cosmetic_dirty = true;
                    }
                    // Defer hover after queued gameplay/resize until fresh map geometry exists.
                    if dirty || cached_view.is_none() {
                        let position=(!blocked).then_some((x,y));
                        pending_hover_raw=Some(if blocked{None}else{raw_pointer});
                        if let Some(r)=run.as_ref(){defer_hover_position(&painter,&r.run,position,
                            &mut pending_hover_position,&mut hover_tile,&mut hover_path,&mut hover_report);}
                        else {pending_hover_position=Some(None);hover_tile=None;hover_path.clear();hover_report=None;}
                        cosmetic_dirty=true;
                        continue;
                    }
                    let next = if !blocked {
                        cached_view.as_ref().and_then(|v|pointer_tile(&painter,v,x,y))
                    } else { None };
                    if next != hover_tile {
                        hover_tile = next;
                        hover_path.clear();
                        hover_report = None;
                        if let (Some((x, y)), Some(r)) = (next, run.as_ref()) {
                            hover_report = Some(hover_tile_report(&r.run,x,y,pending_item.as_ref(),
                                cached_view.as_ref().and_then(|v|v.terrain_snapshot.as_deref())));
                            if pending_item.is_none()
                                && pending_ability.is_none()
                                && tile_report.is_none()
                                && r.run.status == "playing"
                            {
                                hover_path = travel_ui::path_to(&r.run, x, y, true)
                                    .iter()
                                    .map(|s| (s.x, s.y))
                                    .collect();
                            }
                        }
                        if let (Some(view),Some(r))=(cached_view.as_mut(),run.as_ref()){
                            if tile_enabled{tile_overlay::mark_presentation(&painter,view,&mut tile_plan);}
                            let path=travel.as_ref().map(|(t,_)|t.path()).or_else(||autoexplore.path())
                                .map(|path|path.iter().map(|s|(s.x,s.y)).collect()).unwrap_or_else(||hover_path.clone());
                            sync_pointer_view(view,&r.run,hover_tile,path,pending_item.as_ref());
                            sync_ability_preview(view,&r.run,pending_ability.as_deref(),if pending_ability.as_deref()==Some("etill-riptide"){riptide_target.enemy()}else{implant_attacker},pending_item.is_some());
                            if tile_enabled{tile_overlay::mark_presentation(&painter,view,&mut tile_plan);}
                        }
                        cosmetic_dirty = true;
                    }
                }
                Event::MouseWheel {
                    y,
                    mouse_x,
                    mouse_y,
                    ..
                } if screen == Screen::Game => {
                    if let Some(r) = run.as_ref().filter(|r| debug_loadout.active(&r.run)) { debug_loadout.scroll(&r.run,y); dirty=true; continue; }
                    if let Some(report) = tile_report.as_ref() {
                        if y < 0 {
                            tile_report_scroll = (tile_report_scroll + (-y) as usize)
                                .min(inspection_ui::scroll_line_count(report, &painter));
                        } else {
                            tile_report_scroll = tile_report_scroll.saturating_sub(y as usize);
                        }
                    } else if ability_detail.is_some() {
                        if y < 0 {
                            ability_detail_scroll += (-y) as usize;
                        } else {
                            ability_detail_scroll =
                                ability_detail_scroll.saturating_sub(y as usize);
                        }
                        dirty = true;
                        continue;
                    }
                    if inventory {
                        if let Some(view) = pack_view.as_mut() {
                            view.scroll(y, mouse_x, mouse_y);
                        }
                        dirty = true;
                        continue;
                    }
                    if !inspecting && tile_report.is_none() && history.is_none() && hazard_prompt.is_none() {
                        if let Some(v)=cached_view.as_ref() {
                            let (_,_,ox,_)=painter.map_layout(v.width,v.height);
                            let layout=render::enemy_list::Layout::resolve_frame(&painter.profile,ox,painter.frame());
                            if layout.bounds.contains(mouse_x,mouse_y) {
                                let content=layout.content_view(&painter,v);
                                painter.enemy_list_offset=content.scroll(painter.enemy_list_offset,y);
                                let index=content.hit(mouse_x,mouse_y,painter.enemy_list_offset);
                                painter.enemy_hover=index.and_then(|i|render::enemy_list::Content::actor(v,i));
                                painter.ground_hover=index.and_then(|i|render::enemy_list::Content::ground(v,i));
                                cosmetic_dirty=true;continue;
                            }
                        }
                    }
                    if mouse_x >= painter.hud_left() + 10
                        && (painter.frame().map.1+painter.frame().map.3-22..painter.frame().map.1+painter.frame().map.3).contains(&mouse_y)
                    {
                        if y < 0 {
                            player_status_offset += 2;
                        } else {
                            player_status_offset = player_status_offset.saturating_sub(2);
                        }
                    } else if mouse_x >= painter.hud_left() + 10
                        && mouse_y >= ability_ui::top(&painter.profile)
                        && mouse_y < ability_ui::top(&painter.profile)+ability_ui::row(&painter.profile)*ability_ui::capacity(&painter.profile) as i32
                    {
                        if y < 0 {
                            ability_offset += 1;
                        } else {
                            ability_offset = ability_offset.saturating_sub(1);
                        }
                        if let Some(r) = run.as_ref() {
                            ability_offset = ability_offset.min(
                                ability_ui::visible_abilities(&r.run)
                                    .len()
                                    .saturating_sub(1),
                            );
                        }
                    } else if painter.chronicle_hit_rect().contains(mouse_x,mouse_y)
                    {
                        if y > 0 {
                            chronicle_offset += y as usize;
                        } else {
                            chronicle_offset = chronicle_offset.saturating_sub((-y) as usize);
                        }
                    } else if inspecting {
                        if y < 0 {
                            creature_offset += (-y) as usize;
                        } else {
                            creature_offset = creature_offset.saturating_sub(y as usize);
                        }
                    }
                    dirty = true;
                }
                Event::MouseWheel { y, .. } if screen == Screen::Summary => {
                    summary_scroll = if y < 0 {
                        (summary_scroll + (-y) as usize).min(
                            summary
                                .as_ref()
                                .map(|s| s.hearts.len().saturating_sub(9))
                                .unwrap_or(0),
                        )
                    } else {
                        summary_scroll.saturating_sub(y as usize)
                    };
                    dirty = true;
                }
                Event::MouseWheel { y, .. } if screen == Screen::Manual => {
                    // Entire entry fits on one page; wheel browses entries, never hides text.
                    if y != 0 {
                        manual.step(if y < 0 { 1 } else { -1 });
                    }
                    dirty = true;
                }
                Event::DropFile { filename, .. } if screen == Screen::ImportSave => {
                    match load_with_kind(&filename) {
                        Ok((saved, kind)) => {
                            if !saved.run.playtest.is_missing() {
                                if let Some(previous) = run.take() {
                                    if previous.run.playtest.is_missing() && previous.run.status == "playing" { saved_run = Some((previous, session_kind)); }
                                }
                            }
                            debug_loadout = debug_floor_ui::View::default();
                            session_kind = if fixture_session {
                                session_store::Kind::Fixture
                            } else {
                                kind
                            };
                            effects.clear();
                            condition_meters.reset();
                            provenance = if session_kind == session_store::Kind::Genuine {
                                ""
                            } else {
                                "Native migration test scene / incomplete port / unranked"
                            };
                            run = Some(saved);
                            pack_view = None;
                            inventory = false;
                            inspecting = false;
                            pending_item = None;
                            pending_ability = None;
                            screen = Screen::Game;
                            notice = "Imported complete save.".into();
                        }
                        Err(e) => notice = e,
                    }
                    dirty = true;
                }
                Event::DropFile { filename, .. } if screen == Screen::Scores => {
                    if let Err(e) = profiles.load(std::path::Path::new(&filename)) {
                        profiles.notice = e;
                    }
                    dirty = true;
                }
                Event::DropFile { filename, .. }
                    if screen == Screen::Replays || screen == Screen::Menu =>
                {
                    match replays.load(std::path::Path::new(&filename)) {
                        Ok(()) => screen = Screen::ReplayGame,
                        Err(e) => {
                            replays.notice = e;
                            screen = Screen::Replays;
                        }
                    }
                    dirty = true;
                }
                Event::TextInput { text, .. } if screen==Screen::Menu && menu_seed_focus && !avatar_selection.active() && !pending_start.busy()=>{
                    if menu_ui::edit_seed(&mut seed,&text,false){dirty=true;}
                }
                Event::TextInput { text, .. } if screen == Screen::Start && !pending_start.busy() && floor_focus => {
                    if let Some(floor) = playtest_floor.as_mut() {
                        if floor.len() + text.len() <= 3 && text.chars().all(|c| c.is_ascii_digit()) { floor.push_str(&text); dirty = true; }
                    }
                }
                Event::TextInput { text, .. } if screen == Screen::Start && !pending_start.busy() && seed_focus => {
                    if menu_ui::edit_seed(&mut seed,&text,false) {dirty=true;}
                }
                Event::Window {
                    win_event: sdl2::event::WindowEvent::Leave,
                    ..
                } => {
                    hover_tile = None;
                    pending_hover_position=Some(None);
                    pending_hover_raw=Some(None);
                    hover_path.clear();
                    hover_report = None;
                    dirty = true;
                }
                // Recreate texture storage, then request a full paint/upload without
                // unnecessarily rebuilding the cached gameplay observation.
                Event::RenderDeviceReset { .. } => {
                    gpu_renderer.reset();
                    force_texture_upload=true;force_present=true;
                    if gpu_available{pending_gpu_failure=Some("SDL graphics device reset".into());}else{
                        texture=creator.create_texture_streaming(PixelFormatEnum::RGBA32,painter.width as u32,painter.height as u32).map_err(|e|e.to_string())?;
                    }
                    cosmetic_dirty=true;
                }
                Event::RenderTargetsReset { .. } => {
                    gpu_renderer.reset();
                    force_texture_upload=true;force_present=true;
                    // A full paint/upload conservatively restores reset contents.
                    cosmetic_dirty=true;
                }
                Event::Window { win_event:sdl2::event::WindowEvent::Minimized, .. }=>{minimized=true;},
                Event::Window { win_event:sdl2::event::WindowEvent::Restored, .. } => {minimized=false;cosmetic_dirty=true;force_present=true;},
                Event::Window { win_event:sdl2::event::WindowEvent::Exposed, .. } => {cosmetic_dirty=true;force_present=true;},
                Event::Window { .. } => {dirty = true;},
                _ => {}
            }
        }
        if observed_seed!=seed {avatar_selection.sync_seed(&seed);observed_seed=seed.clone();}
        dirty |= menu_visits.observe(screen,pending_start.busy(),&mut avatar_selection,&mut seed,&mut notice);
        if screen!=Screen::Menu {avatar_selection.close();}
        dirty|=cancel_aim_for_context(screen!=Screen::Game||inventory||inspecting||history.is_some()||ability_detail.is_some()||tile_report.is_some()||hazard_prompt.is_some(),
            &mut pending_item,&mut pending_ability,&mut implant_attacker,&mut keyboard_aim);
        if pending_gpu_failure.is_some(){continue 'running;}
        // Leaving the form, including importing another run, invalidates its result.
        let was_preparing = pending_start.busy();
        if let Some(result) = pending_start.poll(matches!(screen,Screen::Start|Screen::Menu)) {
            match result {
                Ok(r) => {
                    if !r.run.playtest.is_missing() {
                        if let Some(previous) = run.take() {
                            if previous.run.playtest.is_missing() && previous.run.status == "playing" { saved_run = Some((previous, session_kind)); }
                        }
                    }
                    session_kind = session_store::Kind::Genuine;
                    provenance = "";
                    effects.clear(); condition_meters.reset();
                    debug_loadout = debug_floor_ui::View::default();
                    run = Some(r); pack_view = None; pending_item = None;
                    inventory = false; inspecting = false;
                    screen = Screen::Game; notice.clear();
                }
                Err(error) => notice = error,
            }
            dirty = true;
        }
        dirty |= menu_visits.observe(screen,pending_start.busy(),&mut avatar_selection,&mut seed,&mut notice);
        if was_preparing && !pending_start.busy() { dirty = true; }
        // DEBUG never supersedes the normal Continue slot, even in this process.
        if run.as_ref().is_some_and(|r| r.run.playtest.is_missing()) {
            saved_run = None;
        }
        if run.as_ref().is_some_and(|r| !r.run.playtest.is_missing()) {
            provenance = "DEBUG • Prior-floor reconstruction • No autosave / scores • F4 stash";
        }
        if let Some(r) = run.as_mut() {
            if r.run.status != "playing" {
                let key = format!("{}:{}:{}", r.run.seed, r.run.tick, r.run.status);
                if terminal_record_key.as_ref() != Some(&key) {
                    terminal_record_key = Some(key);
                    record_warning.clear();
                    if session_store::unknown_seed_replay(&r.run){record_warning=session_store::UNKNOWN_REPLAY_WARNING.into();}
                    if session_store::RELEASE_RANKING_ENABLED
                        && session_kind == session_store::Kind::Genuine
                        && r.run.playtest.is_missing()
                    {
                        let id = r.ensure_run_id(|| {
                            format!("native-{}", fresh_seed())
                        });
                        match id.map_err(|e| e.to_string()).and_then(|_| {
                            session_store::archive(
                                std::path::Path::new(&profile_path),
                                std::path::Path::new(&replay_dir),
                                &r.run,
                                session_kind,
                                Some(&session_store::utc_now()),
                            )
                        }) {
                            Ok(true) => {
                                let _ = profiles.load(std::path::Path::new(&profile_path));
                                replays = replays::Replays::new(std::path::Path::new(&replay_dir));
                            }
                            Ok(false) => {}
                            Err(e) => record_warning = e,
                        }
                    }
                }
            } else {
                terminal_record_key = None;
            }
        }
        if screen == Screen::Settings && display_form.is_none() {
            let (desktop, modes) = display_choices(canvas.window(), &video);
            display_form = Some(display_ui::DisplaySettings::new(settings.display_mode,
                settings.store.resolution, desktop, modes).with_action_animation_mode(settings.store.action_animation_mode));
        } else if screen != Screen::Settings {
            display_form = None;
        }
        let now = start.elapsed().as_millis() as f64;
        let next_hallucination_phase=hallucination_ui::sample(now,reduced);
        let hallucination_changed=next_hallucination_phase!=hallucination_phase
            && cached_view.as_ref().is_some_and(hallucination_ui::view_active);
        if screen==Screen::Game&&hallucination_changed {cosmetic_dirty=true;}
        if history.is_some(){
            if screen!=Screen::Game||shutter.closed(now,reduced){history=None;cosmetic_dirty=true;}
            else if shutter.animating(now,reduced){cosmetic_dirty=true;}
        }
        if health_warning.observe(run.as_ref().filter(|r|screen==Screen::Game&&r.run.status=="playing").map(|r|(r.run.player.hp,r.run.player.max_hp))){dirty=true;tile_plan.invalidate();tile_overlays.invalidate();force_texture_upload=true;}
        effects.gameplay_pause.observe(run.as_ref().filter(|_|screen==Screen::Game).map(|r|&r.run));
        if screen==Screen::Game&&(effects.gameplay_pause.active()||effects.gameplay_pause.waiting||health_warning.active()||run.as_ref().is_some_and(|r|death_pause::terminal(&r.run.status))) {
            travel=None;resting=None;autoexplore.cancel();pending_item=None;pending_ability=None;implant_attacker=None;riptide_target.cancel();keyboard_aim=false;
            inventory=false;inspecting=false;ability_detail=None;tile_report=None;hazard_prompt=None;history=None;hover_report=None;
        }
        if !effects.presentation_pending(now) && !effects.gameplay_pause.active() && !effects.gameplay_pause.waiting && !help.active() && screen == Screen::Game && travel.as_ref().is_some_and(|(_, next)| now >= *next) {
            if let (Some((mut walk, _)), Some(r)) = (travel.take(), run.as_mut()) {
                let quiet = walk.exploring();
                match walk.next_command(&r.run) {
                    Ok(Some(command)) => {
                        if let Some(message) = hazards_ui::intentional(&r.run, &command).filter(|_|!gameplay_pause::needs_ack(&r.run,&command)) {
                            hazard_prompt = Some((command, message));
                        } else {
                            match dispatch(r, &mut effects, now, &command) {
                                Ok(_) => match walk.after_step(&r.run, false) {
                                    Ok(true) => travel = Some((walk, now + 45.)),
                                    Ok(false) => {}
                                    Err(e) => { if !quiet { notice = e; } },
                                },
                                Err(e) => { if !quiet { notice = e.to_string(); } },
                            }
                        }
                    }
                    Ok(None) => {}
                    Err(e) => { if !quiet { notice = e; } },
                }
                dirty = true;
            }
        }
        if health_warning.observe(run.as_ref().filter(|r|screen==Screen::Game&&r.run.status=="playing").map(|r|(r.run.player.hp,r.run.player.max_hp))){dirty=true;tile_plan.invalidate();tile_overlays.invalidate();force_texture_upload=true;}
        if !effects.presentation_pending(now) && !effects.gameplay_pause.active() && !effects.gameplay_pause.waiting && !help.active() && screen == Screen::Game && !health_warning.active() && resting.as_ref().is_some_and(|rest| now >= rest.next) {
            if let Some(r) = run.as_mut() {
                let before = resting.take().unwrap();
                if r.run.status == "playing" && r.run.offers.is_empty() {
                    match dispatch(r, &mut effects, now, &Command::Wait) {
                        Ok(_) => {
                            let next = wait_ui::observe(&r.run);
                            if let Some(reason) = wait_ui::changed(&before.knowledge, &next) {
                                notice = format!("Waiting paused: {reason}");
                            } else {
                                resting = Some(wait_ui::Rest {
                                    knowledge: next,
                                    next: now + 45.,
                                });
                            }
                        }
                        Err(e) => notice = e.to_string(),
                    }
                }
                dirty = true;
            }
        }

        // One normal Wait per displayed animation boundary, never a tight simulation loop.
        if screen==Screen::Game && effects.gameplay_pause.waiting && !effects.presentation_pending(now) && !health_warning.active() {
            if let Some(r)=run.as_mut() {
                if r.run.status=="playing" && r.run.offers.is_empty() && corpse_core::engine::condition(&r.run.player,"paralyzed")>0. {
                    match dispatch(r,&mut effects,now,&Command::Wait) {
                        Ok(_)=>{if r.run.status!="playing"||!r.run.offers.is_empty(){effects.gameplay_pause.stop_waiting();}},
                        Err(e)=>{notice=e.to_string();effects.gameplay_pause.stop_waiting();},
                    }
                }else{effects.gameplay_pause.stop_waiting();}
                effects.gameplay_pause.observe(Some(&r.run));dirty=true;
            }
        }
        if health_warning.active(){effects.gameplay_pause.stop_waiting();}

        if let Some(r) = run.as_ref() {
            if let Err(e) = autosave.flush_tracked(std::path::Path::new(&save_path), r, session_kind, false)
            {
                notice = format!("Save failed: {e}. Export your run before closing.");
                dirty = true;
            }
        }

        if health_warning.observe(run.as_ref().filter(|r|screen==Screen::Game&&r.run.status=="playing").map(|r|(r.run.player.hp,r.run.player.max_hp))){dirty=true;tile_plan.invalidate();tile_overlays.invalidate();force_texture_upload=true;}
        if health_warning.active(){travel=None;resting=None;autoexplore.cancel();pending_item=None;pending_ability=None;implant_attacker=None;riptide_target.cancel();keyboard_aim=false;inventory=false;inspecting=false;ability_detail=None;tile_report=None;hazard_prompt=None;history=None;hover_report=None;}
        let action_reduced=reduced || settings.store.action_animation_mode!=appearance_store::ActionAnimationMode::Normal;
        let action_instant=settings.store.action_animation_mode==appearance_store::ActionAnimationMode::Instant;
        replays.configure_motion(action_reduced,action_instant);
        if screen == Screen::ReplayGame && replays.playing && now - replays.last_step >= 120. && replays.ready_for_step(now) {
            replays.step(now);
            dirty = true;
        }
        // Resample after synchronous command/replay/autosave work, so the first
        // presented frame starts at the queued action rather than skipping it.
        let now = start.elapsed().as_millis() as f64;
        if !minimized&&drawable_w>0&&drawable_h>0 {effects.begin_frame(now);}
        effects.configure_motion(action_reduced,action_instant);
        if effects.profile.as_ref()!=Some(&painter.profile){effects.profile=Some(painter.profile.clone());}
        let old_cues = (effects.cues.len(),effects.plane_transition.is_some(),effects.sequence.active());
        if !minimized&&drawable_w>0&&drawable_h>0 {effects.prune(now);}
        dirty |= (effects.cues.len(),effects.plane_transition.is_some(),effects.sequence.active()) != old_cues;
        let transient = effects.animate(now) || (screen==Screen::ReplayGame&&replays.animating(now));
        let frame = if reduced {
            18
        } else {
            18 + (start.elapsed().as_millis() / if screen == Screen::Menu { 33 } else if transient && screen != Screen::Manual { 16 } else { 83 }) as u32
        };
        if !minimized&&drawable_w>0&&drawable_h>0 && (dirty || cosmetic_dirty
            || (screen_has_timed_paint(screen) && frame != last_frame)
            || (screen == Screen::Game
                && !reduced
                && frame != last_frame
                && (transient
                    || cached_view.as_ref().is_some_and(|v| {
                        cached_view_needs_idle_paint(v,&painter.profile)
                    })))
        ) {
            painter.set_responsive_canvas(matches!(screen,Screen::Game|Screen::ReplayGame));
            let mut tile_frame=false;
            painter.tile_clip_enabled=false;
            let gpu_frame=gpu_available && rejected_drawable.is_none() && matches!(screen,Screen::Game|Screen::ReplayGame);
            if gpu_frame{painter.gpu.get_or_insert_with(Default::default).begin();}else{painter.gpu=None;gpu_renderer.reset();}

            if screen!=Screen::Game{painter.firebolt_terrain=None;tile_plan.invalidate();tile_overlays.invalidate();tile_panels=None;}
            painter.bestiary_unread=manual.progress.has_unread();
            match screen {
                Screen::Menu => {
                    painter.menu(frame, menu_selected, can_continue(&run, &saved_run), &mode);
                    painter.menu_seed(&seed,menu_seed_focus,pending_start.busy());
                    painter.menu_fast_mode(menu_fast_mode,menu_selected==menu_ui::FAST_INDEX);
                    avatar_selection.paint_slots(&mut painter,menu_selected);
                    if !notice.is_empty() {
                        painter.rect(103,415,559,24,[24,10,12]);
                        let label=painter.fit_text_role(typography_ui::Role::Secondary,&notice,545,12);
                        painter.text_role(typography_ui::Role::Secondary,&label,110,420,12,[240,170,145]);
                    }
                    avatar_selection.paint_picker(&mut painter);
                    help_ui::paint_menu_button(&mut painter, menu_help_hover && !help.active());
                }
                Screen::Summary => {
                    if let Some(s) = &summary {
                        terminal_ui::paint(
                            &mut painter,
                            s,
                            summary_scroll,
                            if record_warning.is_empty() {
                                provenance
                            } else {
                                &record_warning
                            },
                        );
                    }
                }
                Screen::ImportSave => painter.import_save(&notice),
                Screen::Start => {
                    painter.start(&seed, &mode, seed_focus, playtest_floor.as_deref().map(|s| (s, floor_focus)), &notice);
                },
                Screen::Manual => {
                    let manual_run=if previous==Screen::ReplayGame {replays.session.as_ref().map(|s|&s.engine.run)} else {run.as_ref().map(|e|&e.run)};
                    manual.set_compressed_depths(manual_run.map_or(true,|r|r.compressed_depths.value_or(false)));
                    manual.set_death_gas(!matches!(previous, Screen::Game | Screen::ReplayGame)
                        || manual_run.is_none_or(corpse_core::death_gas_rules::current));
                    manual.set_simulation_revision(manual_simulation_revision(previous,
                        run.as_ref().map(|engine| &engine.run),
                        replays.session.as_ref().map(|session| &session.engine.run)));
                    manual.paint_at(&mut painter, if reduced { 0. } else { now });
                    manual.mark_viewed(); manual.progress.report_error(&mut notice);
                },
                Screen::Replays => replays.library(&mut painter),
                Screen::ReplayGame => replays.paint(&mut painter, now, reduced),
                Screen::Scores => profiles.paint(&mut painter),
                Screen::Settings => { if let Some(form) = display_form.as_ref() { form.draw(&mut painter); } },
                Screen::Game => {
                    if let Some(staged) = effects.gameplay_pause.outcome.as_deref().or_else(||effects.sequence.view(now)) {
                        // Paint only the historical observation, including HUD
                        // and visibility. Final-state tooltips/modals cannot leak.
                        painter.tile_clip_enabled = false;
                        painter.firebolt_terrain = None;
                        painter.game_at(staged, "", false, if reduced { 0. } else { now });
                        if effects.gameplay_pause.outcome.is_some(){
                            let outgoing:Vec<_>=effects.cues.iter().filter(|c|c.event.depth==staged.depth).cloned().collect();
                            painter.effects(staged,&outgoing,now,action_reduced);
                        }else{painter.effects(staged,&effects.cues,now,action_reduced);}
                        tile_plan.invalidate(); tile_overlays.invalidate();
                        force_texture_upload = true;
                    } else if let Some(r) = &run {
                        {
                            if dirty || cached_view.is_none() {
                                let _observe_timing=corpse_core::timing::span("desktop.ui.observe_world");
                                let mut view=observe::observe_with_profile(&r.run,&painter.profile);
                                let discovered=bestiary_progress::observed_ids(&r.run,&view,manual.ids());
                                let added=manual.progress.discover(discovered.iter().map(String::as_str));
                                manual.progress.report_error(&mut notice);
                                bestiary_notices.observe(&r.run,added);
                                bestiary_notices.apply(&r.run,&mut view);
                                painter.bestiary_unread=manual.progress.has_unread();
                                if let Some(raw)=pending_hover_raw.take(){
                                    pending_hover_position=Some(resize_hover_position(&painter,raw,canvas.window().size(),resize_hover_blocked(Some(&r.run),inventory||inspecting||history.is_some()||ability_detail.is_some()||hazard_prompt.is_some()),tile_report.is_some()));
                                }
                                if let Some(position)=pending_hover_position.take(){
                                    hover_tile=position.and_then(|(x,y)|pointer_tile(&painter,&view,x,y));
                                }
                                // Re-resolve against fresh observations/geometry, even if the mouse stayed still.
                                let mouse=events.mouse_state();
                                let point=canvas.window().has_mouse_focus().then(||painter.logical_point_for_canvas(mouse.x(),mouse.y(),canvas.window().size(),true));
                                refresh_panel_hover(&mut painter,&view,point,r.run.offers.is_empty()&&r.run.status=="playing"
                                    &&!(inventory||inspecting||history.is_some()||ability_detail.is_some()||hazard_prompt.is_some()||tile_report.is_some()));
                                hover_report=None;hover_path.clear();
                                if let Some((x, y)) = hover_tile {
                                    hover_report = Some(hover_tile_report(&r.run,x,y,pending_item.as_ref(),view.terrain_snapshot.as_deref()));
                                    hover_path = if pending_item.is_none()
                                        && pending_ability.is_none()
                                        && tile_report.is_none()
                                        && r.run.status == "playing"
                                    {
                                        travel_ui::path_to(&r.run, x, y, true)
                                            .iter()
                                            .map(|s| (s.x, s.y))
                                            .collect()
                                    } else {
                                        Vec::new()
                                    };
                                }
                                let identity = format!("{}:{}", r.run.seed, r.run.depth);
                                if identity != meter_identity || r.run.tick < meter_tick {
                                    condition_meters.reset();
                                }
                                meter_identity = identity;
                                meter_tick = r.run.tick;
                                view.measure_conditions(&mut condition_meters);
                                view.player_status_offset = player_status_offset
                                    .min(view.player_meters.len().saturating_sub(2));
                                view.selected_actor = inspected_actor;
                                view.path = travel.as_ref().map(|(t, _)| t.path())
                                    .or_else(|| autoexplore.path())
                                    .map(|path| path.iter().map(|s| (s.x, s.y)).collect())
                                    .unwrap_or_else(|| hover_path.clone());
                                view.hover = hover_tile;
                                view.possible_exits = hover_tile.map_or_else(Vec::new, |(x, y)|
                                    view.terrain_snapshot.as_deref().map_or_else(||non_euclidean_ui::possible_exit_marks(&r.run,x,y),|o|non_euclidean_ui::possible_exit_marks_with_terrain(&r.run,x,y,o)));
                                sync_item_preview(&mut view,&r.run,pending_item.as_ref());
                                sync_ability_preview(&mut view,&r.run,pending_ability.as_deref(),if pending_ability.as_deref()==Some("etill-riptide"){riptide_target.enemy()}else{implant_attacker},pending_item.is_some());
                                view.provenance = provenance.into();
                                view.chronicle_offset = chronicle_offset;
                                view.abilities =
                                    view.abilities.into_iter().skip(ability_offset).collect();
                                view.state = view.state.replace(
                                    &format!("history {}", r.run.history.len()),
                                    &format!("history {}", r.history.frames.len()),
                                );
                                if tile_enabled{tile_plan.replace_view(cached_view.as_ref(),&view,&painter);}
                                cached_view = Some(view);
                            }
                            if dirty||hallucination_changed {
                                let view=cached_view.as_mut().unwrap();
                                if tile_enabled{hallucination_ui::mark_view(&painter,view,&mut tile_plan);}
                                hallucination_ui::apply_view(view,next_hallucination_phase);
                                if tile_enabled{hallucination_ui::mark_view(&painter,view,&mut tile_plan);}
                            }
                            if let Some(report)=hover_report.as_mut(){inspection_ui::apply_hallucination(report,next_hallucination_phase);}
                            if let Some(report)=tile_report.as_mut(){inspection_ui::apply_hallucination(report,next_hallucination_phase);}
                            hallucination_phase=next_hallucination_phase;
                            let _paint_timing=corpse_core::timing::span("desktop.paint.composition");
                            let current_view=cached_view.as_ref().unwrap();
                            if effects.firebolt_terrain.prepare(current_view,&effects.cues,now,action_reduced,&mut painter.firebolt_terrain) && tile_enabled {
                                // Staged terrain can change layer footprints and custom light
                                // reconstruction. Arrival frames conservatively redraw all.
                                tile_plan.invalidate();
                                tile_plan.replace_view(None,current_view,&painter);
                            }
                            let paint_notice=inspection_ui::footer_text(&notice, hover_report.as_ref());
                            let ability_detail_view=ability_detail.as_ref().map(|id|ability_ui::observe(&r.run,id));
                            let mut planned_overlay=None;
                            let mut full_pack_state=None;
                            if tile_enabled{
                                tile_frame=true;
                                let pack_state=if inventory{
                                    let pack=pack_view.get_or_insert_with(||pack::PackView::observe(&r.run));
                                    if dirty{pack.refresh(&r.run);pack.configure(&painter,painter.frame().map.tuple());}
                                    Some(pack.paint_state(&painter))
                                }else{None};
                                let hover_state=if !inventory&&!inspecting&&ability_detail.is_none()&&history.is_none(){
                                    hover_report.as_ref().zip(hover_tile).and_then(|(report,_)|tile_overlay::hover(&painter,report))
                                }else{None};
                                let shutter_state=history.as_ref().and_then(|_|chronicle_overlay::bounds_painter(&painter,&shutter,now,reduced).and_then(|(x,y,w,h)|painter.logical_damage_rect(x,y,w,h)).map(|bounds|tile_overlay::Shutter{bounds,offset:shutter.offset}));
                                let mut panels=Vec::new();
                                if effects.plane_transition.is_some(){let b=painter.frame().map;panels.extend(painter.logical_damage_rect(b.0,b.1,b.2,b.3));}
                                if !effects::inventory_marks(&effects.cues,now).is_empty(){panels.extend(painter.logical_damage_rect(0,painter.toolbar_y(),painter.logical_width(),64));panels.extend(painter.logical_damage_rect(painter.hud_left(),40,painter.logical_width()-painter.hud_left(),662));if let Some(pack)=pack_view.as_ref(){let b=pack.bounds();panels.extend(painter.logical_damage_rect(b.0,b.1,b.2,b.3));}}
                                if inspecting{panels.extend(painter.sheet_damage_rect(333,121,644,587));}
                                if tile_report.is_some(){let(x,y,w,h)=inspection_ui::bounds(&painter.profile);panels.extend(painter.sheet_damage_rect(x,y,w,h));}
                                // Unclipped hazard prose retains its horizontal band. Ability
                                // details share exact content-sized bounds with their painter.
                                if hazard_prompt.is_some(){panels.extend(painter.sheet_damage_rect(0,282,render::WIDTH as i32,render::HEIGHT as i32-282));}
                                if let Some(ability)=&ability_detail_view{panels.extend(painter.ability_detail_damage(ability));}
                                if debug_loadout.active(&r.run){panels.extend(painter.sheet_damage_rect(0,51,render::WIDTH as i32,718));}
                                let next=tile_overlay::Frame{pack:pack_state,hover:hover_state,effects:tile_overlay::effects(&painter,current_view,&effects.cues,now,action_reduced),shutter:shutter_state,
                                    full_modal:!current_view.offers.is_empty()&&r.run.status=="playing",panels,redraw_panels:dirty||hallucination_changed||effects.redraw_panels(now),choice:choice_selected,inspector:inspecting.then_some((inspected_actor.map(f64::to_bits),creature_offset)),save_error:autosave.error.clone()};
                                for rect in tile_overlays.changes(&painter,&next){tile_plan.mark_native_rect(rect);}
                                let keys=tile_overlay::PanelKeys::read(&painter);keys.mark_changes(tile_panels,&painter,current_view,&mut tile_plan);tile_panels=Some(keys);
                                if tile_notice!=paint_notice{if let Some(rect)=painter.footer_damage(){tile_plan.mark_native_rect(rect);}tile_notice=paint_notice.to_owned();}
                                if let Some(stage)=painter.firebolt_terrain.as_ref() {
                                    for rect in stage.damage(&painter,current_view) {tile_plan.mark_native_rect(rect);}
                                }
                                tile_plan.mark_animation(current_view,&painter,now,reduced);
                                tile_overlay::finish_dependencies(&mut painter,&mut tile_plan,&next);
                                tile_plan.paint_base(&mut painter,current_view,paint_notice,if reduced{0.}else{now});
                                planned_overlay=Some(next);
                            }else if inventory {
                                let pack=pack_view.get_or_insert_with(||pack::PackView::observe(&r.run));
                                if dirty {pack.refresh(&r.run);pack.configure(&painter,painter.frame().map.tuple());}
                                full_pack_state=Some(pack.paint_state(&painter));
                                painter.game_at_behind_pack(current_view,paint_notice,if reduced{0.}else{now},pack.bounds());
                            }else{painter.game_at(current_view,paint_notice,false,if reduced{0.}else{now});}
                            if !tile_enabled||!painter.tile_mask.is_empty(){
                            if let Some(transition)=effects.plane_transition.as_ref(){painter.scene_crossfade(&transition.before,now-transition.started,transition.ascend);}
                            painter.effects(
                                cached_view.as_ref().unwrap(),
                                &effects.cues,
                                now,
                                action_reduced,
                            );
                            if inventory {
                                let pack=pack_view.get_or_insert_with(||pack::PackView::observe(&r.run));
                                if let Some(state)=planned_overlay.as_ref().and_then(|s|s.pack.as_ref()).or(full_pack_state.as_ref()){
                                    pack.paint_prepared(&mut painter,state);
                                }else{pack.paint(&mut painter);}
                                pack.paint_action_marks(&mut painter,&effects.cues,now);
                            }
                            effects::paint_inventory_hud(&mut painter,current_view,&effects.cues,now,inventory);
                            if !inventory&&!inspecting&&ability_detail.is_none()&&history.is_none() {
                                if let Some((report,_))=hover_report.as_ref().zip(hover_tile) {
                                    inspection_ui::paint_hover(report,&mut painter);
                                }
                            }
                            if inspecting {
                                creature_offset = creature_offset.min(
                                    cached_view
                                        .as_ref()
                                        .unwrap()
                                        .creatures
                                        .len()
                                        .saturating_sub(1),
                                );
                                painter.creatures(
                                    cached_view.as_ref().unwrap(),
                                    inspected_actor,
                                    creature_offset,
                                );
                            }
                            if let Some(report) = &tile_report {
                                painter.centered_sheet(|p|inspection_ui::paint(report,p,tile_report_scroll));
                            }
                            if let Some((_, message)) = &hazard_prompt {
                                painter.hazard_prompt(message);
                            }
                            if let Some(ability) = &ability_detail_view {
                                painter.ability_detail(
                                    ability,
                                    ability_detail_scroll,
                                );
                            }
                            if r.run.status == "playing"
                                && !cached_view.as_ref().unwrap().offers.is_empty()
                            {
                                painter.blessing_choices(
                                    &cached_view.as_ref().unwrap().offers,
                                    choice_selected,
                                );
                                painter.centered_sheet(|p|debug_floor_ui::choice_banner(&r.run,p));
                            }
                            if debug_loadout.active(&r.run) { painter.centered_sheet(|p|debug_loadout.paint(&r.run,p)); }
                            if let Some(lines)=&history{chronicle_overlay::paint(&mut painter,lines,&shutter,now,reduced);}
                            }
                            if let Some(next)=planned_overlay{tile_overlays.commit(next);}
                        }
                    }
                }
            }
            help.paint(&mut painter);
            if screen==Screen::Game && !effects.sequence.pending(now) && !effects.gameplay_pause.fall_active(){if let Some(level)=health_warning.level(){painter.tile_clip_enabled=false;health_warning::paint(&mut painter,level);}}
            if screen==Screen::Game && !effects.presentation_pending(now) && effects.gameplay_pause.active() && (!health_warning.active()||effects.gameplay_pause.fall_active()){painter.tile_clip_enabled=false;gameplay_pause::paint(&mut painter,effects.gameplay_pause.text());}
            if screen==Screen::Game && !effects.presentation_pending(now) && !effects.gameplay_pause.active() && run.as_ref().is_some_and(|r|death_pause::terminal(&r.run.status)){painter.tile_clip_enabled=false;gameplay_pause::paint(&mut painter,gameplay_pause::CONTINUE);}
            if !tile_frame||!painter.tile_mask.is_empty(){if let Some(error)=autosave.error.as_ref(){paint_save_failure(&mut painter,error);}}
            if let Some(frame)=painter.gpu.as_ref(){
                let _gpu_timing=corpse_core::timing::span("desktop.gpu.draw");
                if let Err(error)=gpu_renderer.draw(&mut canvas,frame){
                    pending_gpu_failure=Some(error);continue 'running;
                }
                canvas.present();
            }else{
            let upload=tile_overlay::upload_plan(&painter,!tile_frame||force_texture_upload||health_warning.active()||effects.gameplay_pause.active());
            let full_upload=upload.full;
            let upload_rects=&upload.rectangles;
            {let _upload_timing=corpse_core::timing::span("desktop.texture.upload");
                if full_upload{texture.update(None,&painter.pixels,painter.width*4).map_err(|e|e.to_string())?;}
                for rect in upload_rects{let(bytes,pitch)=tile_overlay::upload_source(&painter,*rect)?;
                    texture.update(Some(sdl2::rect::Rect::new(rect.x as i32,rect.y as i32,rect.w as u32,rect.h as u32)),bytes,pitch).map_err(|e|e.to_string())?;
                }
            }
            if full_upload||!upload_rects.is_empty()||force_present{let _present_timing=corpse_core::timing::span("desktop.texture.present");canvas.clear();canvas.copy(&texture,None,None)?;canvas.present();}
            }
            if !effects.presentation_pending(now) { if effects.gameplay_pause.active()&&(!health_warning.active()||effects.gameplay_pause.fall_active()){effects.gameplay_pause.presented();}else{health_warning.presented();} }
            death_pause.presented(!effects.gameplay_pause.active()&&!effects.presentation_pending(now)&&screen==Screen::Game&&run.as_ref().is_some_and(|r|death_pause::terminal(&r.run.status)));
            painter.tile_clip_enabled=false;tile_plan.clear_frame();force_present=false;force_texture_upload=false;
            dirty = false;
            cosmetic_dirty = false;
            last_frame = frame;
        }
        #[cfg(test)] if input_loop_tests::observe(&mut canvas,&painter,input_loop_tests::State {
            screen,engine:run.as_ref(),cursor:keyboard_cursor.active(),tile:hover_tile,inventory,
            help:help.active(),resting:resting.is_some(),item:pending_item.is_some(),ability:pending_ability.is_some(),
        })? {break 'running;}
        std::thread::sleep(Duration::from_millis(8));
    }
    Ok(())
}
/// Paint-only persistence banner, drawn last over the complete frame.
fn paint_save_failure(painter:&mut Painter,error:&str){
    painter.rect(8,8,1264,76,[35,10,14]);
    painter.text("Save failed. Your window stays open so you can retry or export the run.",20,17,16,[255,180,160]);
    let lines=painter.wrap_text_role(typography_ui::Role::Primary,error,1228,13);
    for(i,line)in lines.iter().take(2).enumerate(){painter.text(line,20,43+i as i32*17,13,[245,210,204]);}
}
fn cancels_autoexplore(event: &Event) -> bool {
    match event {
        Event::KeyDown { keycode, .. } => *keycode != Some(Keycode::X),
        Event::MouseButtonDown { .. } | Event::MouseWheel { .. }
            | Event::Window { win_event: sdl2::event::WindowEvent::FocusLost, .. } => true,
        _ => false,
    }
}
fn key_command(key: Keycode) -> Option<Command> {
    let d = match key {
        Keycode::Left | Keycode::A | Keycode::H | Keycode::Kp4 => Some((-1., 0.)),
        Keycode::Right | Keycode::D | Keycode::L | Keycode::Kp6 => Some((1., 0.)),
        Keycode::Up | Keycode::W | Keycode::K | Keycode::Kp8 => Some((0., -1.)),
        Keycode::Down | Keycode::S | Keycode::J | Keycode::Kp2 => Some((0., 1.)),
        Keycode::Y | Keycode::Kp7 => Some((-1., -1.)),
        Keycode::U | Keycode::Kp9 => Some((1., -1.)),
        Keycode::B | Keycode::Kp1 => Some((-1., 1.)),
        Keycode::N | Keycode::Kp3 => Some((1., 1.)),
        _ => None,
    };
    if let Some((dx, dy)) = d {
        return Some(Command::Move { dx, dy });
    }
    match key {
        Keycode::Z | Keycode::Period | Keycode::Space | Keycode::Kp5 => Some(Command::Wait),

        Keycode::R => Some(Command::Ability {
            id: "numo-revert".into(),
            target: Field::Missing,
            x: Field::Missing,
            y: Field::Missing,
        }),
        Keycode::F => Some(Command::Ability {
            id: "morae-see-the-future".into(),
            target: Field::Missing,
            x: Field::Missing,
            y: Field::Missing,
        }),
        Keycode::V => Some(Command::Ability {
            id: "native-unsupported-boundary".into(),
            target: Field::Missing,
            x: Field::Missing,
            y: Field::Missing,
        }),
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn autoexplore_other_input_cancels_but_hover_and_x_do_not() {
        let key = |keycode| Event::KeyDown { timestamp: 0, window_id: 0,
            keycode, scancode: None, keymod: Mod::NOMOD, repeat: false };
        assert!(!cancels_autoexplore(&key(Some(Keycode::X))));
        for other in [Some(Keycode::Escape), Some(Keycode::Right), Some(Keycode::I), None] {
            assert!(cancels_autoexplore(&key(other)));
        }
        assert!(cancels_autoexplore(&Event::MouseButtonDown { timestamp: 0, window_id: 0,
            which: 0, mouse_btn: sdl2::mouse::MouseButton::Left, clicks: 1, x: 0, y: 0 }));
        assert!(!cancels_autoexplore(&Event::MouseMotion { timestamp: 0, window_id: 0,
            which: 0, mousestate: sdl2::mouse::MouseState::from_sdl_state(0), x: 0, y: 0, xrel: 1, yrel: 0 }));
        assert!(cancels_autoexplore(&Event::Window { timestamp: 0, window_id: 0,
            win_event: sdl2::event::WindowEvent::FocusLost }));
        assert!(key_command(Keycode::X).is_none(), "X is handled by the preview, not simulation command dispatch");
    }
    #[test]
    fn manual_render_test_matches_live_and_respects_fractional_visual_override() {
        let mut manual = manual::Manual::new();
        assert!(manual.select_id("wendigo"));
        let initial_depth = manual.depth;
        let initial_selected = manual.selected;
        let initial_roster = manual.ids().to_vec();
        let mut painter = Painter::new();
        manual.paint_at(&mut painter, 0.);
        let live = painter.pixels.clone();
        paint_manual_render_test(&mut manual, &mut painter, 0., None);
        assert_eq!(painter.pixels, live);
        manual.paint_at_depth(&mut painter, 0., 0);
        let explicit = painter.pixels.clone();
        assert_ne!(explicit, live, "explicit floor one differs from first appearance");
        paint_manual_render_test(&mut manual, &mut painter, 0., Some(0.));
        assert_eq!(painter.pixels, explicit);
        manual.paint_at_visual_depth(&mut painter, 0., 18.5);
        let fractional = painter.pixels.clone();
        assert_ne!(fractional, live);
        paint_manual_render_test(&mut manual, &mut painter, 0., Some(18.5));
        assert_eq!(painter.pixels, fractional);
        assert_eq!(manual.depth, initial_depth);
        assert_eq!(manual.selected, initial_selected);
        assert_eq!(manual.ids(), initial_roster);
    }
    fn fixture_engine() -> temporal::Engine {
        temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap()
    }
    /// O(one command + effect events), tests the actual main helper ownership route.
    #[test]
    fn main_dispatch_keeps_command_receipts_without_mutable_escape() {
        let mut engine=fixture_engine();assert!(engine.acknowledge_persisted(engine.token()));
        let mut effects=effects::Effects::default();dispatch(&mut engine,&mut effects,0.,&Command::Wait).unwrap();
        let batch=engine.pending_batch();assert!(!batch.checkpoint_required);
        assert_eq!(batch.receipts.len(),1);assert_eq!(batch.receipts[0].command,Command::Wait);
    }
    /// O(pack IDs), actual metadata changes remain appendable and exact no-ops
    /// leave the token unchanged, including Missing/Null/empty distinctions.
    #[test]
    fn pack_order_records_exact_metadata_without_mutable_escape() {
        let mut engine=fixture_engine();engine.run.inventory_display_order=Field::Missing;
        assert!(engine.acknowledge_persisted(engine.token()));let token=engine.token();
        assert!(!set_pack_order(&mut engine,Field::Missing).unwrap());assert_eq!(engine.token(),token);
        assert!(set_pack_order(&mut engine,Field::Value(vec![])).unwrap());assert!(!engine.pending_batch().checkpoint_required);
        assert!(engine.acknowledge_persisted(engine.token()));let token=engine.token();
        assert!(!set_pack_order(&mut engine,Field::Value(vec![])).unwrap());assert_eq!(engine.token(),token);
        assert!(set_pack_order(&mut engine,Field::Null).unwrap());assert!(!engine.pending_batch().checkpoint_required);
        assert!(matches!(engine.run.inventory_display_order, Field::Null));
    }
    #[test]
    fn invalid_pack_order_reports_error_without_mutating_or_advancing() {
        let mut engine=fixture_engine();assert!(engine.acknowledge_persisted(engine.token()));
        let token=engine.token();let before=engine.save().unwrap();
        let mut view=pack::PackView::observe(&engine.run);let mut effects=effects::Effects::default();
        let mut open=true;let mut target=None;let mut notice=String::new();
        apply_pack_action(pack::PackAction::Order(vec![f64::NAN]),&mut view,&mut engine,&mut effects,0.,&mut open,&mut target,&mut notice);
        assert!(!notice.is_empty());assert!(open);assert!(target.is_none());
        assert_eq!(engine.token(),token);assert_eq!(engine.save().unwrap(),before);
        assert!(!engine.pending_batch().checkpoint_required);
    }
    #[test]
    fn terminal_existing_id_does_not_poison_tracking_or_generate_another_id() {
        let mut engine=fixture_engine();
        let original=engine.ensure_run_id(||"ui-existing-id".into()).unwrap();
        assert!(engine.acknowledge_persisted(engine.token()));let token=engine.token();
        assert_eq!(engine.ensure_run_id(||panic!("existing ID must not generate")).unwrap(),original);
        assert_eq!(engine.token(),token);assert!(!engine.pending_batch().checkpoint_required);
    }
    #[test]
    fn debug_continue_resumes_only_preserved_normal_run() {
        let normal=fixture_engine();let canonical=normal.save().unwrap();
        let mut debug=fixture_engine();debug.run.seed="debug-marker".into();
        debug.run.playtest=corpse_core::playtest::Profile { version:1,start_floor:1,profile:corpse_core::playtest::PROFILE.into(),prior_hearts:0,prior_blessings:0.,sources:vec![],candidates:vec![],debug:None }.into();
        let mut current=Some(debug);let mut saved=Some((normal,session_store::Kind::Genuine));let mut kind=session_store::Kind::Fixture;
        assert!(can_continue(&current,&saved));assert!(continue_run(&mut current,&mut saved,&mut kind));
        assert_eq!(current.unwrap().save().unwrap(),canonical);assert!(saved.is_none());assert_eq!(kind,session_store::Kind::Genuine);
    }
    /// O(1), pointer hover selects without activation and gaps preserve keyboard-selected index.
    #[test]
    fn blessing_hover_uses_click_bounds_without_auto_choosing(){let mut selected=0;assert!(update_blessing_hover(&mut selected,3,389,350));assert_eq!(selected,1);assert!(!update_blessing_hover(&mut selected,3,389,350));assert!(!update_blessing_hover(&mut selected,3,389,338));assert_eq!(selected,1);assert!(!update_blessing_hover(&mut selected,1,389,520));assert_eq!(selected,1);selected=2;assert!(!update_blessing_hover(&mut selected,3,0,0));assert_eq!(selected,2);}
    #[test]
    fn menu_pointer_matches_primary_buttons_and_excludes_gaps() {
        for (i, item) in menu_ui::ITEMS.iter().enumerate() {
            assert_eq!(menu_item_at(item.x, item.y), Some(i));
            assert_eq!(menu_item_at(item.x + item.w - 1, item.y + item.h - 1), Some(i));
            assert_eq!(menu_item_at(item.x - 1, item.y), None);
            assert_eq!(menu_item_at(item.x + item.w, item.y), None);
        }
        assert_eq!(menu_item_at(0, 0), None);
    }
    #[test]
    fn continue_requires_live_run_and_never_revives_terminal_run() {
        let mut current = None;
        let mut saved = None;
        let mut kind = session_store::Kind::Genuine;
        assert!(!can_continue(&current, &saved));
        assert!(!continue_run(&mut current, &mut saved, &mut kind));
        saved = Some((fixture_engine(), session_store::Kind::Fixture));
        let canonical = saved.as_ref().unwrap().0.save().unwrap();
        assert!(continue_run(&mut current, &mut saved, &mut kind));
        assert!(saved.is_none());
        assert_eq!(kind, session_store::Kind::Fixture);
        assert_eq!(current.as_ref().unwrap().save().unwrap(), canonical);
        current.as_mut().unwrap().run.status = "dead".into();
        saved = Some((fixture_engine(), session_store::Kind::Genuine));
        assert!(!continue_run(&mut current, &mut saved, &mut kind));
        current.as_mut().unwrap().run.status = "victory".into();
        assert!(!can_continue(&current, &saved));
        current = None;
        saved.as_mut().unwrap().0.run.status = "dead".into();
        assert!(!can_continue(&current, &saved));
    }
    #[test]
    fn continue_keeps_active_progress_instead_of_loading_stale_save() {
        let mut current = Some(fixture_engine());
        current.as_mut().unwrap().run.tick = 99.;
        let mut saved = Some((fixture_engine(), session_store::Kind::Fixture));
        let mut kind = session_store::Kind::Genuine;
        assert!(continue_run(&mut current, &mut saved, &mut kind));
        assert_eq!(current.unwrap().run.tick, 99.);
        assert_eq!(kind, session_store::Kind::Genuine);
    }
    #[test]
    fn controls_dispatch_exact_direction() {
        assert_eq!(
            key_command(Keycode::Y),
            Some(Command::Move { dx: -1., dy: -1. })
        );
        assert_eq!(key_command(Keycode::Period), Some(Command::Wait));
        assert_eq!(key_command(Keycode::Escape), None);
    }
}

/// Core command + receipt/event processing, plus O(I) dagger lookup for I items.
/// Ordered playback retains <=128 COW boundary checkpoints and projects only
/// visible-action boundaries; detailed clone/visibility bounds: complexity-ledger.
/// Every R52 command while Dreamwalking, or one capable of entering/changing a
/// plane/floor, captures a full outgoing observation: existing observation cost
/// plus O(View bytes) retained memory, even if that command does not transition.
/// Keep the wrapper type here: coercing to mutable core would force every command to checkpoint.
fn dispatch(
    engine: &mut temporal::Engine,
    effects: &mut effects::Effects,
    now: f64,
    command: &Command,
) -> Result<corpse_core::engine::DispatchOutcome, corpse_core::engine::EngineError> {
    if engine.run.status=="dead" {return Err(corpse_core::engine::EngineError("Run has ended.".into()));}
    let acknowledged=effects.gameplay_pause.take_bypass();
    if effects.gameplay_pause.enabled && !acknowledged && gameplay_pause::needs_ack(&engine.run,command) {
        if engine.run.simulation_revision.value_or(29.)>=52. { effects.gameplay_pause.defer_jump(command); }
        else { effects.gameplay_pause.defer(command); }
        return Ok(Default::default());
    }

    let before_depth=engine.run.depth;
    let before_travel=staged_firebolt::needs_snapshot(&engine.run,command).then(||
        observe::observe_with_profile(&engine.run,effects.profile.as_ref().unwrap_or_else(||appearance_profile::Profile::defaults())));
    let profile = effects.profile.clone().unwrap_or_default();
    let sequence_before = engine.run.clone();
    let observer = corpse_core::events::PresentationObserver::install(|run| std::rc::Rc::new(run.clone()));
    let mut out = engine.dispatch(command)?;
    let frames = observer.take();
    let descents = observer.descents();
    drop(observer);
    session_store::changed();
    let rewind = matches!(command,Command::Ability{id,..} if id=="numo-revert" || id=="morae-see-the-future");
    effects.add(
        std::mem::take(&mut out.events),
        engine.run.tick,
        engine.run.depth,
        now,
        rewind,
    );
    if effects.gameplay_pause.enabled && !acknowledged && engine.run.depth>before_depth
        && engine.run.simulation_revision.value_or(29.)<52. && !descents.stairs_only() {
        effects.gameplay_pause.hold_outcome(action_sequence::observe(&sequence_before,&profile));
    }
    effects.sequence.capture_run(sequence_before, frames, &effects.cues, &profile);
    if let Some(before)=before_travel {effects.firebolt_terrain.capture(before,&effects.cues);}
    effects.defer_until_frame(now);
    Ok(out)
}

/// O(S) validation/seed preparation before one bounded worker is enqueued.
fn begin_menu_descent(loader:&mut startup_ui::Loader<temporal::Engine>,seed:&str,selection:&avatar_menu::Selection,notice:&mut String)->bool {
    begin_selected_descent(loader,seed,selection,None,notice)
}
fn begin_selected_descent(loader:&mut startup_ui::Loader<temporal::Engine>,seed:&str,selection:&avatar_menu::Selection,floor:Option<String>,notice:&mut String)->bool {
    begin_selected_descent_with_fast(loader,seed,selection,floor,false,notice)
}
fn begin_selected_descent_with_fast(loader:&mut startup_ui::Loader<temporal::Engine>,seed:&str,selection:&avatar_menu::Selection,floor:Option<String>,fast:bool,notice:&mut String)->bool {
    if loader.busy(){return false;}
    if fast {
        if let Some(input)=floor.as_deref() {
            if let Err(error)=menu_ui::parse_start_floor(input,true) { *notice=error; return false; }
        }
    }
    let seed=match selection.prepared_seed(seed,fresh_seed) {Ok(seed)=>seed,Err(error)=>{*notice=error;return false;}};
    let started=loader.start(move||new_selected_run_with_fast(&seed,avatar_menu::MODE,floor.as_deref(),fast));
    if started{*notice="Preparing descent... Esc cancels.".into();}started
}
fn validate_new_mode(mode:&str)->Result<(),String> {
    if mode!=avatar_menu::MODE {Err("New games are Avatar-only. Existing saves and replays retain their recorded modes.".into())} else {Ok(())}
}
fn new_selected_run(seed: &str, mode: &str, floor: Option<&str>) -> Result<temporal::Engine, String> {
    new_selected_run_with_fast(seed,mode,floor,false)
}
fn new_selected_run_with_fast(seed: &str, mode: &str, floor: Option<&str>, _fast: bool) -> Result<temporal::Engine, String> {
    validate_new_mode(mode)?;
    let seed = menu_ui::selected_seed(seed,fresh_seed);
    let run = if let Some(floor) = floor {
        let floor = menu_ui::parse_start_floor(floor,true)?;
        corpse_core::debug_floor::create_avatar_run(&seed, floor)?
    } else { corpse_core::startup::create_avatar_run(&seed)? };
    let engine=temporal::Engine::new(run).map_err(|e| e.to_string())?;
    session_store::changed();
    Ok(engine)
}
fn new_run(seed: &str, mode: &str) -> Result<temporal::Engine, String> {
    new_selected_run_with_fast(seed,mode,None,true)
}
fn fresh_seed() -> String {
    let mut bytes = [0u8; 24];
    getrandom::fill(&mut bytes).expect("OS random source for new descent seed");
    let values: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .collect();
    let mut gods: Vec<u32> = (0..12).collect();
    for i in 0..3 {
        gods.swap(i, i + values[i] as usize % (12 - i));
    }
    fn base36(mut n: u32) -> String {
        if n == 0 {
            return "0".into();
        }
        let mut s = vec![];
        while n > 0 {
            s.push(b"0123456789abcdefghijklmnopqrstuvwxyz"[(n % 36) as usize] as char);
            n /= 36;
        }
        s.into_iter().rev().collect()
    }
    format!(
        "{:02}-{:02}-{:02}:{}",
        gods[0],
        gods[1],
        gods[2],
        values[3..]
            .iter()
            .map(|n| base36(*n))
            .collect::<Vec<_>>()
            .join("-")
    )
}


/// O(I) exact ordered-ID comparison and changed metadata receipt. No world copy
/// or mutable escape. Missing, Null and empty remain distinct; errors never dispatch.
fn set_pack_order(engine: &mut temporal::Engine, desired: Field<Vec<f64>>) -> Result<bool,String> {
    engine.set_inventory_display_order(desired)
}

/// A committed item use closes the Pack even if no turn advanced. Preserve
/// prior behavior for other turn-taking commands and rejected/cancelled aim.
fn finish_pack_dispatch(open:&mut bool,status:&str,result:Result<corpse_core::engine::DispatchOutcome,corpse_core::engine::EngineError>)->String {
    *open=status=="playing" && !result.as_ref().is_ok_and(|out|out.took_turn||out.used_item);
    result.map(|_|String::new()).unwrap_or_else(|e|e.to_string())
}
/// Actual command cost and existing pack refresh; UI branches O(1) apart from order data.
fn apply_pack_action(
    action: pack::PackAction,
    view: &mut pack::PackView,
    r: &mut temporal::Engine,
    effects: &mut effects::Effects,
    now: f64,
    open: &mut bool,
    target: &mut Option<pack::PackTarget>,
    notice: &mut String,
) {
    match action {
        pack::PackAction::None => {}
        pack::PackAction::Close => *open = false,
        pack::PackAction::Order(ids) => { if let Err(error)=set_pack_order(r, ids.into()){*notice=error;} },
        pack::PackAction::Notice(message) => *notice = message,
        pack::PackAction::Target(t) => {
            *notice = directional_zap_ui::prompt(&r.run,&t).into();
            *target = Some(t);
            *open = false;
        }
        pack::PackAction::Command(command) => {
            if let Err(error)=set_pack_order(r, view.display_order().into()){*notice=error;return;}
            let outcome=dispatch(r,effects,now,&command);
            *notice=finish_pack_dispatch(open,&r.run.status,outcome);
            view.refresh(&r.run);
        }
    }
}

fn display_choices(window: &sdl2::video::Window, video: &sdl2::VideoSubsystem) -> (appearance_store::Resolution, Vec<appearance_store::Resolution>) {
    use appearance_store::Resolution;
    let index = window.display_index().unwrap_or(0);
    let desktop = video.desktop_display_mode(index).map(|m| Resolution { width: m.w as u32, height: m.h as u32 })
        .unwrap_or(Resolution { width: 1280, height: 800 });
    let mut modes = Vec::new();
    for i in 0..video.num_display_modes(index).unwrap_or(0) {
        if let Ok(m) = video.display_mode(index, i) {
            let r = Resolution { width: m.w as u32, height: m.h as u32 };
            if r.valid() { modes.push(r); }
        }
    }
    modes.push(desktop);
    let size = window.size();
    let current = Resolution { width: size.0, height: size.1 };
    if current.valid() { modes.push(current); }
    modes.push(Resolution { width: 1280, height: 800 });
    modes.push(Resolution { width: 1280, height: 720 });
    modes.push(Resolution { width: 1024, height: 768 });
    modes.sort(); modes.dedup();
    (desktop, modes)
}
fn change_window_display(window: &mut sdl2::video::Window, video: &sdl2::VideoSubsystem,
    mode: appearance_store::DisplayMode, resolution: Option<appearance_store::Resolution>) -> Result<(), String> {
    use appearance_store::DisplayMode;
    if resolution.is_some_and(|r| !r.valid()) { return Err("Unsupported resolution.".into()); }
    // Query first, before mutating any window state.
    let exclusive = if mode == DisplayMode::Fullscreen {
        let index = window.display_index()?;
        Some(if let Some(resolution) = resolution {
            (0..video.num_display_modes(index)?).filter_map(|i| video.display_mode(index, i).ok())
                .find(|m| m.w == resolution.width as i32 && m.h == resolution.height as i32)
                .ok_or("That fullscreen resolution is unavailable on this display.")?
        } else {
            video.desktop_display_mode(index)?
        })
    } else { None };
    window.set_fullscreen(sdl2::video::FullscreenType::Off)?;
    match mode {
        DisplayMode::Windowed => {
            if let Some(resolution) = resolution {
                window.set_size(resolution.width, resolution.height).map_err(|e| e.to_string())?;
                window.set_position(sdl2::video::WindowPos::Centered, sdl2::video::WindowPos::Centered);
            }
        }
        DisplayMode::Borderless => window.set_fullscreen(sdl2::video::FullscreenType::Desktop)?,
        DisplayMode::Fullscreen => {
            window.set_display_mode(exclusive)?;
            window.set_fullscreen(sdl2::video::FullscreenType::True)?;
        }
    }
    if appearance_store::DisplayMode::from_sdl(window.fullscreen_state()) != mode {
        return Err("The display could not enter that mode.".into());
    }
    Ok(())
}
fn apply_basic_display(action: display_ui::Action, form: &mut display_ui::DisplaySettings,
    settings: &mut appearance_ui::SettingsView, canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    video: &sdl2::VideoSubsystem) -> bool {
    use display_ui::Action;
    let Action::Apply { mode, resolution, action_animation_mode } = action else { return matches!(action, Action::Back); };
    let old_mode = canvas.window().fullscreen_state();
    let old_size = canvas.window().size();
    let old_display = canvas.window().display_mode().ok();
    let result = change_window_display(canvas.window_mut(), video, mode, resolution)
        .and_then(|_| settings.store.set_presentation_preferences(mode,
            if mode == appearance_store::DisplayMode::Borderless { None } else { resolution }, action_animation_mode));
    match result {
        Ok(()) => form.notice = "Applied.".into(),
        Err(error) => {
            // Keep actual mode and saved preferences aligned if the OS or disk rejects the change.
            let window = canvas.window_mut();
            let restored = window.set_fullscreen(sdl2::video::FullscreenType::Off)
                .and_then(|_| window.set_display_mode(old_display))
                .and_then(|_| window.set_size(old_size.0, old_size.1).map_err(|e| e.to_string()))
                .and_then(|_| window.set_fullscreen(old_mode));
            form.notice = match restored { Ok(()) => error, Err(e) => format!("{error} Restore failed: {e}") };
        }
    }
    settings.observe_display_mode(appearance_store::DisplayMode::from_sdl(canvas.window().fullscreen_state()));
    false
}

fn apply_settings(
    action: appearance_ui::Action,
    settings: &mut appearance_ui::SettingsView,
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    video: &sdl2::VideoSubsystem,
    painter: &mut Painter,
    reduced: &mut bool,
) -> Result<bool, String> {
    use appearance_ui::Action;
    let mut close = false;
    match action {
        Action::None => return Ok(false),
        Action::Changed => {}
        Action::ReducedMotion(value) => *reduced = value,
        Action::Fullscreen => {
            use appearance_store::DisplayMode;
            let previous = DisplayMode::from_sdl(canvas.window().fullscreen_state());
            let requested = previous.next();
            let mut applied = canvas.window_mut().set_fullscreen(requested.to_sdl());
            let mut actual = DisplayMode::from_sdl(canvas.window().fullscreen_state());
            // Unsupported exclusive mode must not trap the user in borderless mode.
            let fallback = requested == DisplayMode::Fullscreen && (applied.is_err() || actual != requested);
            let expected = if fallback { DisplayMode::Windowed } else { requested };
            if fallback {
                applied = canvas.window_mut().set_fullscreen(expected.to_sdl());
                actual = DisplayMode::from_sdl(canvas.window().fullscreen_state());
            }
            let error = match applied {
                Err(e) => Some(e),
                Ok(()) if actual != expected => Some("Display mode is not supported by this display.".into()),
                Ok(()) => settings.sync_display_mode(actual).err(),
            };
            if let Some(error) = error {
                settings.notice = match canvas.window_mut().set_fullscreen(previous.to_sdl()) {
                    Ok(()) => error,
                    Err(restore) => format!("{error}; cannot restore display mode: {restore}"),
                };
            } else if fallback {
                settings.notice = "Fullscreen unavailable; switched to Windowed.".into();
            }
            settings.observe_display_mode(DisplayMode::from_sdl(canvas.window().fullscreen_state()));
        }
        Action::Close => {
            settings.close();
            close = true;
        }
        Action::Copy(text) => {
            if let Err(e) = video.clipboard().set_clipboard_text(&text) {
                settings.notice = e;
            }
        }
        Action::Paste => match video.clipboard().clipboard_text() {
            Ok(text) => {
                settings.text_input(&text);
            }
            Err(e) => settings.notice = e,
        },
    }
    painter.profile = settings.profile().clone();
    Ok(close)
}

#[cfg(test)]
mod append_play_sequence_tests;

#[cfg(test)] mod pack_turn_completion_tests {
    use super::*;
    #[test] fn actual_eat_turn_closes_pack_and_failed_use_keeps_it_open() {
        use corpse_core::model::{Run,Item,PartialStats};
        let mut raw:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        raw.inventory=vec![Item{id:201.,kind:"heart".into(),name:"Rat heart".into(),glyph:";".into(),bonus:PartialStats{offense:1.0.into(),..Default::default()}.into(),exp:1.0.into(),energy:2.0.into(),..Default::default()}];
        raw.next_id=1000.;
        let mut engine=temporal::Engine::new(raw).unwrap();let mut view=pack::PackView::observe(&engine.run);
        let mut effects=effects::Effects::default();let mut open=true;let mut target=None;let mut notice=String::new();let tick=engine.run.tick;
        apply_pack_action(pack::PackAction::Command(Command::Use{id:201.}),&mut view,&mut engine,&mut effects,0.,&mut open,&mut target,&mut notice);
        assert_eq!(engine.run.tick,tick+1.);assert!(!open);assert!(notice.is_empty());
        open=true;
        apply_pack_action(pack::PackAction::Command(Command::Use{id:9999.}),&mut view,&mut engine,&mut effects,0.,&mut open,&mut target,&mut notice);
        assert!(open);assert_eq!(engine.run.tick,tick+1.);
    }
    #[test] fn tools_of_the_trade_free_consumable_closes_pack() {
        let mut run=staged_firebolt::tests::room();
        run.player.abilities.push("isheilah-tools-of-the-trade".into());
        run.inventory=vec![corpse_core::Item{id:900.,kind:"potion".into(),name:"Potion of Healing".into(),
            glyph:"!".into(),effect:"healing".to_string().into(),identified:true.into(),..Default::default()}];
        let mut engine=temporal::Engine::new(run).unwrap();
        let mut view=pack::PackView::observe(&engine.run);
        let mut effects=effects::Effects::default();
        let (mut open,mut target,mut notice)=(true,None,String::new());
        let tick=engine.run.tick;
        apply_pack_action(pack::PackAction::Command(Command::Use{id:900.}),&mut view,&mut engine,
            &mut effects,0.,&mut open,&mut target,&mut notice);
        assert!(engine.run.inventory.iter().all(|item|item.id!=900.));
        assert_eq!(engine.run.tick,tick);
        assert!(!open&&notice.is_empty());
    }

    #[test] fn spellslinger_free_targeted_activation_stays_out_of_pack_and_rejected_cast_reopens_it() {
        let mut run=staged_firebolt::tests::room();
        run.player.abilities.push("isheilah-spellslinger".into());
        let mut engine=temporal::Engine::new(run).unwrap();
        let mut pack=pack::PackView::observe(&engine.run);
        let mut effects=effects::Effects::default();
        let (mut open,mut pending,mut notice)=(true,None,String::new());
        let (tick,energy)=(engine.run.tick,engine.run.inventory[0].amulet_state.as_ref().unwrap().energy);
        apply_pack_action(pack::PackAction::Target(pack::PackTarget::UseAt(900.)),&mut pack,&mut engine,
            &mut effects,0.,&mut open,&mut pending,&mut notice);
        assert!(!open);
        let view=observe::observe(&engine.run);
        let command=directional_zap_ui::command(&engine.run,&view,&pending.take().unwrap(),(11,10)).unwrap();
        let outcome=dispatch(&mut engine,&mut effects,0.,&command).unwrap();
        assert!(outcome.used_item&&!outcome.took_turn);
        assert_eq!(engine.run.tick,tick);
        assert!(engine.run.inventory[0].amulet_state.as_ref().unwrap().energy<energy);
        assert!(finish_pack_dispatch(&mut open,&engine.run.status,Ok(outcome)).is_empty());assert!(!open);

        let rejected=Command::UseAt{id:900.,x:10.,y:10.};
        let outcome=dispatch(&mut engine,&mut effects,1000.,&rejected).unwrap();
        assert!(!outcome.used_item&&!outcome.took_turn);
        finish_pack_dispatch(&mut open,&engine.run.status,Ok(outcome));
        assert!(open);
    }

    #[test] fn throwing_an_item_during_a_haste_bonus_action_keeps_pack_closed() {
        let mut run=staged_firebolt::tests::room();
        run.phase="haste".to_string().into();
        run.player.conditions.insert("hasted".into(),5.);
        run.inventory=vec![corpse_core::Item{id:900.,kind:"potion".into(),name:"Potion of Healing".into(),
            glyph:"!".into(),effect:"healing".to_string().into(),identified:true.into(),..Default::default()}];
        let mut engine=temporal::Engine::new(run).unwrap();
        let target=pack::PackTarget::Throw(900.);
        let command=directional_zap_ui::command(&engine.run,&observe::observe(&engine.run),&target,(11,10)).unwrap();
        let outcome=dispatch(&mut engine,&mut effects::Effects::default(),0.,&command).unwrap();
        assert!(outcome.used_item&&!outcome.took_turn);
        let mut open=false;
        finish_pack_dispatch(&mut open,&engine.run.status,Ok(outcome));
        assert!(!open);
    }

    #[test] fn free_rejected_target_and_terminal_completion_policy() {
        use corpse_core::engine::{DispatchOutcome,EngineError};
        let mut open=true;
        assert!(finish_pack_dispatch(&mut open,"playing",Ok(DispatchOutcome::default())).is_empty());assert!(open);
        open=false;assert!(!finish_pack_dispatch(&mut open,"playing",Err(EngineError("target rejected".into()))).is_empty());assert!(open);
        open=false;finish_pack_dispatch(&mut open,"playing",Ok(DispatchOutcome{took_turn:true,..Default::default()}));assert!(!open);
        finish_pack_dispatch(&mut open,"dead",Ok(DispatchOutcome::default()));assert!(!open);
        finish_pack_dispatch(&mut open,"won",Err(EngineError("late failure".into())));assert!(!open);
    }
    #[test] fn choosing_a_target_hides_pack_without_spending_a_turn() {
        let mut engine=temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let before=engine.save().unwrap();let mut view=pack::PackView::observe(&engine.run);
        let mut effects=effects::Effects::default();let mut open=true;let mut target=None;let mut notice=String::new();
        apply_pack_action(pack::PackAction::Target(pack::PackTarget::Throw(201.)),&mut view,&mut engine,&mut effects,0.,&mut open,&mut target,&mut notice);
        assert!(!open);assert!(target.is_some());assert_eq!(engine.save().unwrap(),before);
        // Escape discards the pending selector and restores the pack only for a live run.
        assert!(target.take().is_some());open=engine.run.status=="playing";assert!(open);
    }
}

#[cfg(test)]
mod menu_seed_creation_tests {
    use super::*;
    #[test]
    fn fast_debug_rejection_does_not_enqueue_or_change_gods() {
        let selection=avatar_menu::Selection::default();
        let before=selection.clone();
        let mut loader=startup_ui::Loader::default();
        let mut notice=String::new();
        for floor in ["0","21","38"] {
            assert!(!begin_selected_descent_with_fast(&mut loader,"",&selection,Some(floor.into()),true,&mut notice));
            assert_eq!(notice,menu_ui::FAST_DEBUG_NOTICE);
            assert!(!loader.busy());
            assert_eq!(selection,before);
        }
        // A bad floor must win over even invalid seed syntax: no preparation has run.
        assert!(!begin_selected_descent_with_fast(&mut loader,"bad\nseed",&selection,Some("21".into()),true,&mut notice));
        assert_eq!(notice,menu_ui::FAST_DEBUG_NOTICE);
        assert!(!loader.busy());
        assert_eq!(selection,before);
    }
    #[test]
    fn menu_fast_start_is_explicit_and_continue_preserves_the_recorded_flag() {
        let seed="00-09-10:menu-fast-review";
        let normal=new_selected_run_with_fast(seed,"Avatar",None,false).unwrap();
        let fast=new_selected_run_with_fast(seed,"Avatar",None,true).unwrap();
        assert!(normal.run.fast_mode.value_or(false));
        assert!(normal.run.compressed_depths.value_or(false));
        assert!(fast.run.fast_mode.value_or(false));
        assert_eq!(fast.run.seed,seed);
        let encoded=fast.save().unwrap();
        let mut current=None;
        let mut saved=Some((temporal::Engine::load(&encoded).unwrap(),session_store::Kind::Genuine));
        let mut kind=session_store::Kind::Genuine;
        assert!(continue_run(&mut current,&mut saved,&mut kind));
        assert_eq!(current.unwrap().save().unwrap(),encoded);
        assert!(new_selected_run_with_fast(seed,"Avatar",Some("21"),true).is_err());
        let debug=new_selected_run_with_fast(seed,"Avatar",Some("1"),true).unwrap();
        assert!(debug.run.fast_mode.value_or(false));
        assert_eq!(debug.run.playtest.as_ref().unwrap().start_floor,1);
        assert_eq!(debug.run.seed,seed);
    }
    /// Four actual floor constructions plus O(serialized Run bytes) equality; no file IO.
    #[test]
    fn menu_new_run_uses_exact_fixed_seed_revision52_and_blank_os_random_seeds() {
        let seed="00-09-10:menu-seed-review";
        let a=new_selected_run(seed,"Avatar",None).unwrap();
        let b=new_selected_run(seed,"Avatar",None).unwrap();
        assert_eq!(a.run.seed,seed);assert_eq!(b.run.seed,seed);
        assert_eq!(a.run.simulation_revision.value_or(0.),52.);
        assert_eq!(b.run.simulation_revision.value_or(0.),52.);
        assert_eq!(serde_json::to_vec(&a.run).unwrap(),serde_json::to_vec(&b.run).unwrap());
        let blank=new_selected_run("","Avatar",None).unwrap();
        let spaces=new_selected_run("   ","Avatar",None).unwrap();
        assert!(!blank.run.seed.is_empty());assert!(!spaces.run.seed.is_empty());
        // This is a real OS-random integration check, not a mathematical uniqueness claim.
        assert_ne!(blank.run.seed,spaces.run.seed);
        assert_eq!(blank.run.simulation_revision.value_or(0.),52.);
        assert_eq!(spaces.run.simulation_revision.value_or(0.),52.);
    }
    #[test]
    fn menu_debug_start_uses_revision52_without_changing_the_selected_seed() {
        let seed = "0-5-11:menu-debug-r50";
        let debug = new_selected_run(seed, "Avatar", Some("1")).unwrap();
        assert_eq!(debug.run.seed, seed);
        assert_eq!(debug.run.simulation_revision.value_or(0.), 52.);
        assert_eq!(debug.run.playtest.as_ref().unwrap().start_floor, 1);
        let saved = debug.save().unwrap();
        assert_eq!(temporal::Engine::load(&saved).unwrap().save().unwrap(), saved);
    }
}

/// O(registered profile entries + cached cells) worst case; dream flag short-circuits O(1).
/// Only redraw eligibility; no observation, simulation state or RNG access.
fn cached_view_needs_idle_paint(v:&render::View,p:&appearance_profile::Profile)->bool {
    v.dreamwalking || appearance_light::animated(p) || appearance_material::animated(p)
        || v.cells.iter().any(|c|!c.animations.is_empty()||c.pulse_speed!=0.||c.fire_flicker)
        || (p.number("path.animationSpeed")>0.&&!v.path.is_empty())
}
#[cfg(test)]mod dream_idle_redraw_tests {
 use super::*;
 /// O(one fixture observation + cached cells), dream scheduling needs no incidental effect.
 #[test]fn dream_view_idle_eligibility_without_other_animations(){
  let r:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
  let before=serde_json::to_vec(&r).unwrap();let mut v=observe::observe(&r);
  for c in &mut v.cells {c.animations.clear();c.pulse_speed=0.;c.fire_flicker=false;}v.path.clear();v.dreamwalking=false;
  assert!(!cached_view_needs_idle_paint(&v,appearance_profile::Profile::defaults()));
  v.dreamwalking=true;assert!(cached_view_needs_idle_paint(&v,appearance_profile::Profile::defaults()));
  assert_eq!(serde_json::to_vec(&r).unwrap(),before);
 }
}

#[cfg(test)]
mod timeweave_action_tests;

include!("retained_overlay_tests.rs");

include!("retained_paint_bench.rs");

/// O(H) for copying the existing hover path; no Run/visibility/pathfinding query.
/// The right-click handler has already cancelled travel, so hover is its fallback.
fn sync_enemy_inspection_view(view:&mut render::View,actor:Option<f64>,hover_path:&[(usize,usize)]){
    view.selected_actor=actor;view.path=hover_path.to_vec();
}
#[cfg(test)]mod enemy_inspection_descriptor_tests{
    use super::*;
    #[test]fn cancelled_travel_uses_hover_fallback_and_updates_only_selected_actor(){
        let r:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let mut view=observe::observe(&r);view.path=vec![(1,1),(2,1)];view.hover=Some((4,5));view.target_path=vec![(8,9)];view.possible_exits=vec![(7,6)];
        sync_enemy_inspection_view(&mut view,Some(123.),&[(4,4),(4,5)]);
        assert_eq!(view.path,vec![(4,4),(4,5)]);assert_eq!(view.selected_actor,Some(123.));
        assert_eq!(view.hover,Some((4,5)));assert_eq!(view.target_path,vec![(8,9)]);assert_eq!(view.possible_exits,vec![(7,6)]);
        sync_enemy_inspection_view(&mut view,Some(456.),&[]);assert!(view.path.is_empty());assert_eq!(view.selected_actor,Some(456.));
        let before=serde_json::to_vec(&r).unwrap();let mut painter=Painter::new();let mut full=Painter::new();let mut retained=render::RetainedGame::default();
        // Detached visible actor establishes a real selection paint footprint.
        let mut actor=view.cells[view.cells.len()/2].clone();actor.actor_id=Some(123.);actor.current=true;actor.direct=true;actor.glyph="b".into();view.cells.push(actor);
        for selected in [Some(123.),Some(123.),Some(456.)]{
            sync_enemy_inspection_view(&mut view,selected,&[(4,4),(4,5)]);retained.invalidate();
            retained.update(&mut painter,&view,"",0.,false,false).unwrap();full.game_at(&view,"",false,0.);
            assert_eq!(painter.pixels,full.pixels,"same/new selection retained invalidation");
            assert_eq!(view.selected_actor,selected);assert_eq!(view.path,vec![(4,4),(4,5)]);
        }
        assert_eq!(serde_json::to_vec(&r).unwrap(),before,"presentation helper changes no Run/RNG/tick");
    }
}

#[cfg(test)]mod full_paint_production_contract_tests{
    #[test]fn production_redraw_has_full_upload_and_preserves_overlay_order(){
        let source=include_str!("main.rs");
        let begin=source.find("let mut last_frame =").unwrap();
        let end=source.find("fn paint_save_failure(").unwrap();
        let body=&source[begin..end];
        for forbidden in ["RetainedGame","overlay_tracker","retained_game","next_overlay","damage.rects()"]{assert!(!body.contains(forbidden),"production retained work: {forbidden}");}
        assert!(body.contains("CORPSE_TILE_PAINT")||source.contains("CORPSE_TILE_PAINT"));
        let game=&body[body.find("painter.game_at(current_view").unwrap()..];
        let mut after=0;
        for step in ["painter.game_at(","painter.effects(","pack.paint(","inspection_ui::paint_hover(","painter.creatures(","inspection_ui::paint(report", "painter.hazard_prompt(","painter.ability_detail(","painter.blessing_choices(","debug_loadout.paint(","chronicle_overlay::paint(","paint_save_failure(","texture.update(Some", "canvas.copy(&texture"]{
            after+=game[after..].find(step).unwrap_or_else(||panic!("missing/out-of-order layer {step}"))+step.len();
        }
    }
}

/// Synchronize only pointer presentation. Existing targeting/portal queries remain;
/// this deliberately does not call observe_with_profile or regenerate world cells.
fn sync_ability_preview(view:&mut render::View,run:&corpse_core::Run,ability:Option<&str>,attacker:Option<f64>,item_target:bool) {
    view.selected_ability=if item_target{None}else{ability.map(str::to_owned)};
    if item_target {return;}
    view.target_color=None;view.target_area.clear();view.target_impact=None;
    if run.simulation_revision.value_or(29.) < 50. { return; }
    if let Some(id)=ability {
        view.target_path=targeting_ui::ability_preview(run,id,attacker,view.hover);
        if matches!(id,"kative-cogworks"|"kative-clay-army") { view.target_color=Some(ability_ui::god_color(10)); }
        view.possible_exits.clear();
    } else {
        if !view.pursuit_path.is_empty() { view.path=view.pursuit_path.clone(); }
    }
}
/// Entering another interaction context cancels the selected action, not merely
/// its ink. Pack action -> aim is preserved because that transition closes Pack.
fn cancel_aim_for_context(blocked:bool,item:&mut Option<pack::PackTarget>,ability:&mut Option<String>,attacker:&mut Option<f64>,keyboard:&mut bool)->bool {
    if !blocked{return false;}
    let changed=item.is_some()||ability.is_some()||attacker.is_some()||*keyboard;
    *item=None;*ability=None;*attacker=None;*keyboard=false;changed
}
fn aim_modifiers_allowed(modifiers:Mod)->bool {
    !modifiers.intersects(Mod::LCTRLMOD|Mod::RCTRLMOD|Mod::LALTMOD|Mod::RALTMOD|Mod::LGUIMOD|Mod::RGUIMOD)
}
/// O(1) mode admission beyond the existing bounded debug-state lookup.
fn cursor_context_available(screen:Screen,modal:bool,run:Option<&corpse_core::Run>,debug_open:bool)->bool {
    screen==Screen::Game&&!modal&&!debug_open&&run.is_some_and(|run|run.status=="playing"&&run.offers.is_empty()
        && !corpse_core::debug_floor::preparing(run))
}
/// Match the existing pointer's displayed plane, without reading any cell or
/// actor. O(F) existing floor lookup; no knowledge or simulation is changed.
fn cursor_grid(run:&corpse_core::Run)->Option<cursor_ui::Grid> {
    let floor=if corpse_core::dreamwalking::active(run) {
        corpse_core::dreamwalking::floor_in_plane(run,true).or_else(||run.floor())
    } else {run.floor()}?;
    Some(cursor_ui::Grid{width:floor.width(),height:floor.height(),player:(run.player.x as usize,run.player.y as usize)})
}
/// O(1) synthetic map click, bounded to displayed coordinates. The normal
/// click handler owns knowledge, hazards, targeting, turn and resource checks.
fn cursor_click(painter:&Painter,grid:cursor_ui::Grid,tile:(usize,usize),timestamp:u32,window_id:u32)->Option<Event> {
    if !grid.contains(tile) {return None;}
    let(cw,ch,ox,oy)=painter.map_layout(grid.width,grid.height);
    if cw<=0||ch<=0 {return None;}
    Some(Event::MouseButtonDown{timestamp,window_id,which:0,mouse_btn:sdl2::mouse::MouseButton::Left,clicks:1,
        x:ox+tile.0 as i32*cw+cw/2,y:oy+tile.1 as i32*ch+ch/2})
}
/// Keyboard movement while targeting moves only the detached aim coordinate.
/// O(1), bounded by the current floor; world/equipment/turn state is untouched.
fn aim_keyboard_tile(run:&corpse_core::Run,hover:Option<(usize,usize)>,key:Keycode)->Option<(usize,usize)> {
    let Command::Move{dx,dy}=key_command(key)? else{return None};
    let grid=cursor_grid(run)?;if grid.width==0||grid.height==0{return None;}
    let(x,y)=hover.unwrap_or(grid.player);
    Some((x.saturating_add_signed(dx as isize).min(grid.width-1),
          y.saturating_add_signed(dy as isize).min(grid.height-1)))
}
/// One detached result for full observations, cached pointer updates and
/// keyboard aim changes. Clearing is unconditional, including cancel/reopen.
fn sync_item_preview(view:&mut render::View,run:&corpse_core::Run,target:Option<&pack::PackTarget>){
    view.target_path.clear();view.target_area.clear();view.target_impact=None;view.target_color=None;
    if let Some(target)=target {
        let aim=target.aim_preview(run,view.hover);
        view.target_path=aim.delivery;view.target_area=aim.area;view.target_impact=aim.impact;
        view.possible_exits=aim.possible_exits;
        view.target_color=Some([246,192,91]);
    }
}
fn sync_pointer_view(view:&mut render::View,run:&corpse_core::Run,hover:Option<(usize,usize)>,path:Vec<(usize,usize)>,target:Option<&pack::PackTarget>){
    view.path=path;view.hover=hover;
    view.possible_exits=hover.map_or_else(Vec::new,|(x,y)|view.terrain_snapshot.as_deref().map_or_else(||non_euclidean_ui::possible_exit_marks(run,x,y),|o|non_euclidean_ui::possible_exit_marks_with_terrain(run,x,y,o)));view.target_path.clear();
    sync_item_preview(view,run,target);

}
#[cfg(test)]mod pointer_cached_descriptor_tests{
 use super::*;
 #[test]fn pointer_targets_and_cancel_match_fresh_observation_presentation(){
  let r:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();let original=serde_json::to_vec(&r).unwrap();let mut cached=observe::observe(&r);
  for target in [None,Some(pack::PackTarget::UseAt(0.)),Some(pack::PackTarget::Shoot(0.)),None]{
   for hover in [Some((r.player.x as usize,r.player.y as usize)),None]{
    let path=hover.into_iter().collect::<Vec<_>>();sync_pointer_view(&mut cached,&r,hover,path.clone(),target.as_ref());
    // Original full-rebuild presentation assignments, intentionally independent
    // of the helper under test; gameplay observation itself remains unchanged.
    let mut fresh=observe::observe(&r);fresh.path=path;fresh.hover=hover;fresh.possible_exits=hover.map_or_else(Vec::new,|(x,y)|non_euclidean_ui::possible_exit_marks(&r,x,y));
    if let Some(target)=target.as_ref(){let aim=target.aim_preview(&r,hover);fresh.target_path=aim.delivery;fresh.target_area=aim.area;fresh.target_impact=aim.impact;fresh.possible_exits=aim.possible_exits;fresh.target_color=Some([246,192,91]);}
    assert_eq!(cached.target_area,fresh.target_area);assert_eq!(cached.target_impact,fresh.target_impact);assert_eq!(cached.target_color,fresh.target_color);
    assert_eq!(cached.path,fresh.path);assert_eq!(cached.hover,fresh.hover);assert_eq!(cached.target_path,fresh.target_path);assert_eq!(cached.possible_exits,fresh.possible_exits);
   }
  }
  assert_eq!(serde_json::to_vec(&r).unwrap(),original);
 }
}

/// O(1) screen-to-cell projection using the current View geometry.
fn pointer_tile(painter:&render::Painter,view:&render::View,x:i32,y:i32)->Option<(usize,usize)>{
 pointer_tile_dimensions(painter,view.width,view.height,x,y)
}
fn pointer_tile_dimensions(painter:&render::Painter,width:usize,height:usize,x:i32,y:i32)->Option<(usize,usize)>{
 let(cw,ch,ox,oy)=painter.map_layout(width,height);
 (cw>0&&ch>0&&x>=ox&&y>=oy&&x<ox+cw*width as i32&&y<oy+ch*height as i32)
  .then(||(((x-ox)/cw)as usize,((y-oy)/ch)as usize))
}
/// Supplied terrain must belong to the same current projected world. Main calls
/// after fresh observation or under !dirty; shape checks do not prove freshness.
fn hover_tile_report(run:&corpse_core::Run,x:usize,y:usize,target:Option<&pack::PackTarget>,terrain:Option<&corpse_core::vision::TerrainObservation>)->inspection_ui::Report{
 match(target,terrain){
  (Some(pack::PackTarget::Shoot(id)),Some(o))=>inspection_ui::observe_bow_tile_with_terrain(run,x,y,*id,o),
  (Some(pack::PackTarget::Shoot(id)),None)=>inspection_ui::observe_bow_tile(run,x,y,*id),
  (_,Some(o))=>inspection_ui::observe_tile_with_terrain(run,x,y,o),
  (_,None)=>inspection_ui::observe_tile(run,x,y),
 }
}
#[cfg(test)] mod hover_reuse_bench;

/// Cheap queued-event admission: O(F) floor lookup, no terrain/actor observation.
/// Update the immediate Inspect target, but defer report/path until fresh View.
fn defer_hover_position(painter:&render::Painter,run:&corpse_core::Run,position:Option<(i32,i32)>,pending:&mut Option<Option<(i32,i32)>>,tile:&mut Option<(usize,usize)>,path:&mut Vec<(usize,usize)>,report:&mut Option<inspection_ui::Report>){
 *pending=Some(position);
 let floor=if corpse_core::dreamwalking::active(run){corpse_core::dreamwalking::floor_in_plane(run,true).or_else(||run.floor())}else{run.floor()};
 *tile=position.and_then(|(x,y)|floor.and_then(|f|pointer_tile_dimensions(painter,f.width(),f.height(),x,y)));
 path.clear();*report=None;
}
#[cfg(test)]mod queued_hover_tests{
 use super::*;
 #[test]fn dirty_motion_then_inspect_uses_current_geometry_and_offmap_falls_back_to_player(){
  let run:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
  let painter=render::Painter::new();let view=observe::observe(&run);let before=serde_json::to_vec(&run).unwrap();
  let(cw,ch,ox,oy)=painter.map_layout(view.width,view.height);let point=(ox+cw*3+cw/2,oy+ch*4+ch/2);
  let(mut pending,mut tile,mut path,mut report)=(None,Some((20,20)),vec![(20,20)],Some(inspection_ui::Report::default()));
  defer_hover_position(&painter,&run,Some(point),&mut pending,&mut tile,&mut path,&mut report);
  assert_eq!(tile,Some((3,4)));assert_eq!(tile,pointer_tile(&painter,&view,point.0,point.1));
  assert_eq!(pending,Some(Some(point)));assert!(path.is_empty()&&report.is_none());
  // This is the actual toolbar Inspect target selection after queued motion.
  assert_eq!(tile.unwrap_or((run.player.x as usize,run.player.y as usize)),(3,4));
  defer_hover_position(&painter,&run,Some((-1,-1)),&mut pending,&mut tile,&mut path,&mut report);
  assert_eq!(tile,None);assert_eq!(tile.unwrap_or((run.player.x as usize,run.player.y as usize)),(run.player.x as usize,run.player.y as usize));
  // Modal/leave cancellation invalidates queued intent without any observation.
  defer_hover_position(&painter,&run,None,&mut pending,&mut tile,&mut path,&mut report);
  assert_eq!(pending,Some(None));assert!(tile.is_none()&&path.is_empty()&&report.is_none());
  assert_eq!(serde_json::to_vec(&run).unwrap(),before);
 }
}

/// Clear only this resize attempt's own message after successful acceptance.
/// A newer save/game notice must survive; accepting the unchanged committed size
/// is success too. O(message bytes), only on a pending refusal recovery.
fn clear_resize_refusal(notice:&mut String,owned:&mut Option<String>)->bool{
    if owned.take().is_some_and(|error|*notice==error){notice.clear();true}else{false}
}
#[cfg(test)]mod resize_refusal_notice_tests{
    use super::clear_resize_refusal;
    #[test]fn accepted_new_or_already_committed_size_clears_only_owned_refusal(){
        // Both Ok(true) and Ok(false) use the same successful-acceptance branch.
        for resized in [true,false]{
            let result:Result<bool,String>=Ok(resized);
            let mut notice="Window too large; resize smaller".to_owned();
            let mut owned=Some(notice.clone());
            if result.is_ok(){assert!(clear_resize_refusal(&mut notice,&mut owned));}
            assert!(notice.is_empty());assert!(owned.is_none());
        }
    }
    #[test]fn accepting_size_preserves_a_newer_save_failure(){
        let mut owned=Some("Window too large; resize smaller".to_owned());
        let mut notice="Save failed: file replaced".to_owned();
        assert!(!clear_resize_refusal(&mut notice,&mut owned));
        assert_eq!(notice,"Save failed: file replaced");assert!(owned.is_none());
        assert!(!clear_resize_refusal(&mut notice,&mut owned));
    }
}

/// O(1), no Run query. Keep window coordinates until the current drawable and
/// fresh-view handoff; authored inspector hits use their own centered space.
fn resize_hover_blocked(run:Option<&corpse_core::Run>,modal:bool)->bool{
    modal||run.is_some_and(|r| !r.offers.is_empty())
}
fn resize_hover_position(p:&Painter,raw:Option<(i32,i32)>,window:(u32,u32),blocked:bool,tile_inspector:bool)->Option<(i32,i32)>{
    let(x,y)=raw?;if blocked||x<0||y<0||x>=window.0 as i32||y>=window.1 as i32{return None}
    if tile_inspector{let(sx,sy)=p.logical_point_for_sheet(x,y,window);let(xx,yy,w,h)=inspection_ui::bounds(&p.profile);if sx>=xx&&sx<xx+w&&sy>=yy&&sy<yy+h{return None}}
    Some(p.logical_point_for_canvas(x,y,window,true))
}
#[cfg(test)]mod resize_hover_tests{
 use super::*;
 #[test]fn stationary_raw_pointer_uses_new_geometry_in_both_directions(){
  let run:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();let view=observe::observe(&run);let before=serde_json::to_vec(&run).unwrap();
  let mut p=Painter::new();p.set_responsive_canvas(true);let raw=Some((500,350));
  let old_logical=resize_hover_position(&p,raw,(1280,800),false,false).unwrap();
  let mut saw_remapping=false;
  for(w,h,window)in[(1920,1080,(960,540)),(960,1200,(960,1200)),(1280,800,(1280,800))]{
   p.try_resize(w,h).unwrap();let logical=resize_hover_position(&p,raw,window,false,false).unwrap();saw_remapping|=logical!=old_logical;
   let(mut pending,mut tile,mut path,mut report)=(None,Some((30,30)),vec![(30,30)],None);
   defer_hover_position(&p,&run,Some(logical),&mut pending,&mut tile,&mut path,&mut report);
   assert_eq!(tile,pointer_tile(&p,&view,logical.0,logical.1));assert!(path.is_empty()&&report.is_none());
   assert_eq!(pending,Some(Some(logical)));
  }
  assert!(saw_remapping);assert_eq!(serde_json::to_vec(&run).unwrap(),before);
 }
 #[test]fn blessing_offer_resize_cannot_synthesize_map_hover(){
  let mut run:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
  run.offers.clear();let p=Painter::new();
  assert!(!resize_hover_blocked(Some(&run),false));
  run.offers.push("test-offer".into());
  assert_eq!(resize_hover_position(&p,Some((500,350)),(1280,800),resize_hover_blocked(Some(&run),false),false),None);
  run.offers.clear();assert!(resize_hover_blocked(Some(&run),true));
 }
 #[test]fn leave_other_window_and_modal_do_not_create_hover(){
  let mut p=Painter::new();p.try_resize(1920,1080).unwrap();p.set_responsive_canvas(true);
  assert_eq!(resize_hover_position(&p,None,(960,540),false,false),None);
  assert_eq!(resize_hover_position(&p,Some((-1,5)),(960,540),false,false),None);
  assert_eq!(resize_hover_position(&p,Some((960,5)),(960,540),false,false),None);
  assert_eq!(resize_hover_position(&p,Some((500,350)),(960,540),true,false),None);
 }
}

#[cfg(test)]
mod native_drawable_contract_tests {
    use super::*;

    #[test]
    fn window_area_and_density_use_exact_drawable_pixels_without_changing_hit_targets() {
        let mut painter = Painter::new();
        painter.set_responsive_canvas(true);
        // Smaller occupied areas, fractional DPI, Retina and full-screen 4K.
        // SDL provides drawable dimensions: multiplying by monitor DPI again would double-scale.
        for (window, drawable) in [
            ((960, 600), (960, 600)),
            ((960, 600), (1200, 750)),
            ((960, 600), (1440, 900)),
            ((960, 600), (1920, 1200)),
            ((1280, 720), (2560, 1440)),
            ((1920, 1080), (3840, 2160)),
        ] {
            painter.try_resize(drawable.0 as usize, drawable.1 as usize).unwrap();
            assert_eq!((painter.width, painter.height), (drawable.0 as usize, drawable.1 as usize));
            assert_eq!(painter.pixels.len(), drawable.0 as usize * drawable.1 as usize * 4);
            for (nx, ny) in [(1, 1), (2, 2), (3, 3)] {
                let raw = ((window.0 * nx / 4) as i32, (window.1 * ny / 4) as i32);
                let device = ((drawable.0 * nx / 4) as i32, (drawable.1 * ny / 4) as i32);
                // Integer window/device samples round independently at fractional DPI
                // (e.g. 750 / 4). Their inverse hits may differ by one logical pixel.
                for (window_hit, device_hit) in [
                    (painter.logical_point_for_canvas(raw.0, raw.1, window, true),
                     painter.logical_point_for_canvas(device.0, device.1, drawable, true)),
                    (painter.logical_point_for_sheet(raw.0, raw.1, window),
                     painter.logical_point_for_sheet(device.0, device.1, drawable)),
                ] {
                    assert!((window_hit.0 - device_hit.0).abs() <= 1
                        && (window_hit.1 - device_hit.1).abs() <= 1,
                        "window {window:?}, drawable {drawable:?}: {window_hit:?} vs {device_hit:?}");
                }
            }
        }
    }
}

/// O(C+A+T²) for observed-only hit resolution and existing card text layout.
/// Reused after mouse input, world replacement and resizing, without world reads.
fn refresh_panel_hover(p:&mut Painter,v:&render::View,point:Option<(i32,i32)>,allowed:bool)->bool{
    let old=(p.enemy_hover,p.ground_hover,p.ability_hover,p.enemy_list_offset);
    let (enemy,ground,ability)=point.filter(|_|allowed).map_or((None,None,None),|(x,y)|{
        let(cw,ch,ox,oy)=p.map_layout(v.width,v.height);
        let layout=render::enemy_list::Layout::resolve_frame(&p.profile,ox,p.frame());
        let content=layout.content_view(p,v);
        let from_map=p.enemy_at_map_point(v,x,y);
        let map_ground=if from_map.is_none()&&x>=ox&&y>=oy&&x<ox+cw*v.width as i32&&y<oy+ch*v.height as i32 {
            let at=(((x-ox)/cw)as usize,((y-oy)/ch)as usize);
            v.ground_entities.iter().position(|g|(g.x,g.y)==at)
        }else{None};
        let (enemy,ground)=if layout.rows.contains(x,y){
            let index=content.hit(x,y,p.enemy_list_offset);
            (index.and_then(|i|render::enemy_list::Content::actor(v,i)),index.and_then(|i|render::enemy_list::Content::ground(v,i)))
        }else{(from_map,map_ground.map(|i|{let g=&v.ground_entities[i];(g.x,g.y)}))};
        let reveal=from_map.and_then(|id|v.creatures.iter().position(|a|a.id.to_bits()==id).map(|i|i+render::enemy_list::Content::stair_count(v)))
            .or_else(||map_ground.map(|i|render::enemy_list::Content::row_for_ground(v,i)));
        if let Some(index)=reveal{p.enemy_list_offset=content.reveal(index,p.enemy_list_offset);}
        (enemy,ground,p.ability_inspect_at(v.abilities.len(),x,y))
    });
    p.enemy_hover=enemy;p.ground_hover=ground;p.ability_hover=ability;
    old!=(p.enemy_hover,p.ground_hover,p.ability_hover,p.enemy_list_offset)
}
#[cfg(test)]
mod refreshed_hover_tests {
    use super::*;
    #[test]
    fn stationary_map_pointer_uses_new_actor_positions_and_resized_geometry(){
        let run:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let mut v=observe::observe(&run);let mut p=Painter::new();p.set_responsive_canvas(true);
        let id=v.creatures[0].id;let cell=v.cells.iter().find(|c|c.actor_id==Some(id)).unwrap().clone();
        let point=|p:&Painter,v:&render::View|{let(cw,ch,ox,oy)=p.map_layout(v.width,v.height);(ox+cell.x as i32*cw+cw/2,oy+cell.y as i32*ch+ch/2)};
        let at=point(&p,&v);refresh_panel_hover(&mut p,&v,Some(at),true);assert_eq!(p.enemy_hover,Some(id.to_bits()));
        v.cells.iter_mut().find(|c|c.actor_id==Some(id)).unwrap().x+=1;
        refresh_panel_hover(&mut p,&v,Some(at),true);assert_eq!(p.enemy_hover,None);
        v.cells.iter_mut().find(|c|c.actor_id==Some(id)).unwrap().x=cell.x;
        p.resize(1920,1080);let at=point(&p,&v);refresh_panel_hover(&mut p,&v,Some(at),true);assert_eq!(p.enemy_hover,Some(id.to_bits()));
        v.creatures.clear();refresh_panel_hover(&mut p,&v,Some(at),true);assert_eq!(p.enemy_hover,None);
        refresh_panel_hover(&mut p,&v,Some(at),false);assert_eq!(p.ability_hover,None);
    }
}

#[cfg(test)]
mod darkness_render_tests;

#[cfg(test)]
mod water_render_tests;

#[cfg(test)] mod action_animation_tests;

#[cfg(test)]
mod revision52_floods_ui_tests;

#[cfg(test)] mod death_dispatch_tests {
 use super::*;
 #[test] fn dead_run_commands_cannot_advance_or_modify_archive_input(){
  let run:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
  let mut engine=temporal::Engine::new(run).unwrap();engine.run.status="dead".into();engine.run.player.hp=0.;
  let before=engine.run.clone();let mut effects=effects::Effects::default();
  for command in [Command::Wait,Command::Move{dx:1.,dy:0.}] {assert!(dispatch(&mut engine,&mut effects,0.,&command).is_err());assert!(engine.run==before);}
  assert_eq!(serde_json::to_value(corpse_core::profile::summarize_run(&engine.run)).unwrap(),serde_json::to_value(corpse_core::profile::summarize_run(&before)).unwrap());
 }
}

#[cfg(test)] mod animation_video_capture;
