use super::*;
use crate::render::{TOOLBAR_ACTIONS, TOOLBAR_VISIBLE_ACTIONS};

fn key(key: Keycode, keymod: Mod, repeat: bool) -> Event {
    Event::KeyDown { timestamp: 0, window_id: 0, keycode: Some(key), scancode: None, keymod, repeat }
}
fn click(x: i32, y: i32) -> Event {
    Event::MouseButtonDown { timestamp: 0, window_id: 0, which: 0,
        mouse_btn: MouseButton::Left, clicks: 1, x, y }
}
fn wheel(y: i32, direction: MouseWheelDirection) -> Event {
    Event::MouseWheel { timestamp: 0, window_id: 0, which: 0, x: 0, y,
        direction, precise_x: 0., precise_y: y as f32, mouse_x: 0, mouse_y: 0 }
}
fn large_painter() -> Painter {
    let mut p = Painter::new();
    p.profile = p.profile.set("typography.primary.fontSize", serde_json::json!(28)).unwrap()
        .set("typography.primary.lineHeight", serde_json::json!(1.8)).unwrap();
    p
}

#[test]
fn help_preserves_requested_columns_casing_and_colors() {
    assert_eq!(COMMANDS.iter().map(|e|e.label).collect::<Vec<_>>(),
        ["mouse", "click", "enter", "esc", "numpad", "I", "Z", "Shift Z", "x"]);
    assert_eq!(COMMANDS[0].description, "move cursor to examine terrain and monsters or activate divine abilities");
    let escape=COMMANDS.iter().find(|entry|entry.label=="esc").unwrap();
    assert_eq!(escape.description,"exit keyboard cursor control, close panels, or cancel targeting");
    assert_eq!(INFORMATION, [
        Entry { label: "Divine Abilities", description: "Eat hearts to gain stats and blessings to level up and gain new abilities" },
        Entry { label: "Energy", description: "Used for abilities and amulets" },
        Entry { label: "Reserve Energy", description: "Free energy provided by the ability or amulet that is used on that ability or amulet" },
    ]);
    assert_eq!(LABEL_COLOR, [255, 217, 53]);
    assert_eq!(DESCRIPTION_COLOR, [255, 255, 255]);
    let layout = Layout::new(&Painter::new());
    for entry in COMMANDS.iter().chain(INFORMATION.iter()) {
        assert!(layout.lines.iter().any(|l| l.text == entry.label && l.color == LABEL_COLOR));
    }
    assert!(layout.lines.iter().filter(|l| l.x == layout.geometry.description_x)
        .all(|l| l.color == DESCRIPTION_COLOR));
    assert!(layout.lines.iter().any(|l| l.text == "Information"));
}

#[test]
fn help_toolbar_and_menu_question_mark_have_exact_hit_regions() {
    let slot = TOOLBAR_VISIBLE_ACTIONS.iter().position(|a| *a == ACTION).unwrap();
    assert_eq!(TOOLBAR_ACTIONS[slot], "?");
    assert_eq!(TOOLBAR_ACTIONS.last(), Some(&"Menu"));
    let mut p = Painter::new();
    for (width, height) in [(1280, 800), (1920, 1080), (900, 1200)] {
        p.resize(width, height); p.set_responsive_canvas(true);
        let (x, w) = crate::render::toolbar_rect_for_width(slot, p.logical_width());
        assert_eq!(p.toolbar_action(x, p.toolbar_y()), Some(ACTION));
        assert_eq!(p.toolbar_action(x + w - 1, p.toolbar_y() + 39), Some(ACTION));
        assert_ne!(p.toolbar_action(x + w, p.toolbar_y()), Some(ACTION));
        assert_ne!(p.toolbar_action(x, p.toolbar_y() + 40), Some(ACTION));
    }
    assert!(menu_button_at(MENU_BUTTON.0, MENU_BUTTON.1));
    assert!(menu_button_at(MENU_BUTTON.0 + MENU_BUTTON.2 - 1, MENU_BUTTON.1 + MENU_BUTTON.3 - 1));
    assert!(!menu_button_at(MENU_BUTTON.0 - 1, MENU_BUTTON.1));
    assert!(!menu_button_at(MENU_BUTTON.0 + MENU_BUTTON.2, MENU_BUTTON.1));
}

#[test]
fn help_open_close_shortcuts_repeat_modifiers_and_no_clickthrough() {
    let p = Painter::new();
    let mut h = Help::default();
    assert_eq!(h.event(&key(Keycode::F1, Mod::NOMOD, false), &p, false, false), Response::Ignored);
    assert!(!h.active());
    for modifier in [Mod::LCTRLMOD, Mod::LALTMOD, Mod::LGUIMOD] {
        assert_eq!(h.event(&key(Keycode::F1, modifier, false), &p, true, false), Response::Ignored);
    }
    assert_eq!(h.event(&key(Keycode::F1, Mod::NOMOD, true), &p, true, false), Response::Ignored);
    assert_eq!(h.event(&key(Keycode::F1, Mod::NOMOD, false), &p, true, false), Response::Opened);
    assert!(h.active());
    assert_eq!(h.event(&key(Keycode::F1, Mod::NOMOD, true), &p, true, false), Response::Consumed);
    let inside = h.layout.as_ref().unwrap().geometry.content;
    for input in [click(inside.0, inside.1), key(Keycode::I, Mod::NOMOD, false),
        key(Keycode::Z, Mod::NOMOD, false), key(Keycode::Kp6, Mod::NOMOD, true),
        Event::TextInput { timestamp: 0, window_id: 0, text: "x".into() }] {
        assert_eq!(h.event(&input, &p, true, false), Response::Consumed);
        assert!(h.active());
    }
    // The outside click dismisses but does not propagate to map travel.
    assert_eq!(h.event(&click(0, 0), &p, true, false), Response::Closed);
    assert!(!h.active());
    assert_eq!(h.event(&click(0, 0), &p, true, false), Response::Ignored);
    assert_eq!(h.event(&click(0, 0), &p, true, true), Response::Opened);
    let close = h.layout.as_ref().unwrap().geometry.close;
    assert_eq!(h.event(&click(close.0 + 1, close.1 + 1), &p, true, false), Response::Closed);
    assert_eq!(h.event(&key(Keycode::Slash, Mod::LSHIFTMOD, false), &p, true, false), Response::Opened);
    assert_eq!(h.event(&key(Keycode::Escape, Mod::NOMOD, false), &p, true, false), Response::Closed);
    assert_eq!(h.event(&key(Keycode::F1, Mod::NOMOD, false), &p, true, false), Response::Opened);
    assert_eq!(h.event(&key(Keycode::Return, Mod::NOMOD, false), &p, true, false), Response::Closed);
}

#[test]
fn help_preserves_system_events_and_fullscreen_shortcut() {
    let p = Painter::new(); let mut h = Help::default(); h.open(&p);
    for input in [Event::Quit { timestamp: 0 },
        key(Keycode::Return, Mod::LALTMOD, false),
        Event::Window { timestamp: 0, window_id: 0, win_event: sdl2::event::WindowEvent::FocusLost },
        Event::Window { timestamp: 0, window_id: 0, win_event: sdl2::event::WindowEvent::Resized(900, 600) }] {
        assert_eq!(h.event(&input, &p, true, false), Response::Ignored);
        assert!(h.active());
    }
}

#[test]
fn help_keyboard_wheel_and_visible_scroll_buttons_clamp_and_reopen_at_top() {
    let p = large_painter(); let mut h = Help::default(); h.open(&p);
    let g = h.layout.as_ref().unwrap().geometry;
    let max = h.layout.as_ref().unwrap().max_scroll();
    assert!(max > 0);
    for input in [key(Keycode::Down, Mod::NOMOD, false), key(Keycode::PageDown, Mod::NOMOD, false),
        wheel(-1, MouseWheelDirection::Normal), wheel(1, MouseWheelDirection::Flipped),
        click(g.down.0 + 1, g.down.1 + 1)] {
        let old = h.offset;
        assert_eq!(h.event(&input, &p, true, false), Response::Consumed);
        assert!(h.offset > old || old == max);
    }
    h.event(&wheel(i32::MIN, MouseWheelDirection::Normal), &p, true, false);
    assert_eq!(h.offset, max);
    h.event(&key(Keycode::End, Mod::NOMOD, false), &p, true, false); assert_eq!(h.offset, max);
    h.event(&click(g.up.0 + 1, g.up.1 + 1), &p, true, false); assert!(h.offset < max);
    h.event(&wheel(i32::MAX, MouseWheelDirection::Normal), &p, true, false); assert_eq!(h.offset, 0);
    h.event(&key(Keycode::Home, Mod::NOMOD, false), &p, true, false); assert_eq!(h.offset, 0);
    h.event(&key(Keycode::End, Mod::NOMOD, false), &p, true, false);
    h.event(&key(Keycode::Escape, Mod::NOMOD, false), &p, true, false);
    h.event(&key(Keycode::F1, Mod::NOMOD, false), &p, true, false);
    assert_eq!(h.offset, 0);
}

#[test]
fn help_layout_wraps_without_losing_text_and_keeps_columns_disjoint() {
    let mut extreme = Painter::new();
    extreme.profile = extreme.profile.set("typography.primary.fontSize", serde_json::json!(40)).unwrap()
        .set("typography.primary.lineHeight", serde_json::json!(0.8)).unwrap()
        .set("typography.primary.letterSpacing", serde_json::json!(6)).unwrap()
        .set("typography.headings.fontSize", serde_json::json!(40)).unwrap();
    for mut p in [Painter::new(), large_painter(), extreme] {
        for (width, height) in [(1280, 800), (1920, 1080), (900, 1200)] {
            p.resize(width, height); p.set_responsive_canvas(true);
            let l = Layout::new(&p); let g = l.geometry;
            let metrics = TextMetrics::new(&p, Role::Primary, BODY_SIZE, 24);
            assert!(g.bounds.0 >= 0 && g.bounds.1 >= 0);
            assert!(g.bounds.0 + g.bounds.2 <= p.logical_width());
            assert!(g.bounds.1 + g.bounds.3 <= p.logical_height());
            assert!(g.content.0 + g.label_width < g.description_x);
            assert!(g.description_x + g.description_width <= g.content.0 + g.content.2);
            assert!(g.content.1 + g.content.3 <= g.up.1);
            for line in &l.lines {
                let width = if line.x == g.description_x { g.description_width }
                    else if line.color == DESCRIPTION_COLOR { g.content.2 } else { g.label_width };
                assert!(metrics.width(&line.text) <= width as f64 + 0.01, "{}", line.text);
            }
            let descriptions = l.lines.iter().filter(|line| line.x == g.description_x)
                .map(|line| line.text.as_str()).collect::<Vec<_>>().join(" ");
            let expected = COMMANDS.iter().chain(INFORMATION.iter())
                .map(|entry| entry.description).collect::<Vec<_>>().join(" ");
            assert_eq!(descriptions, expected);
            // The final line remains fully visible at the exact scroll limit.
            let last = l.lines.last().unwrap();
            assert!(last.y - l.max_scroll() + l.line_height <= g.content.3);
        }
    }
}

#[test]
fn help_reconfigures_for_profile_and_viewport_changes_and_restores_clip() {
    let mut p = large_painter(); let mut h = Help::default(); h.open(&p);
    h.event(&key(Keycode::End, Mod::NOMOD, false), &p, true, false);
    assert!(h.offset > 0);
    p.profile = Profile::default(); p.resize(1920, 1080); p.set_responsive_canvas(true);
    p.clip = Some((3, 5, 100, 80)); let clip = p.clip;
    h.paint(&mut p);
    assert_eq!(p.clip, clip);
    let l = h.layout.as_ref().unwrap();
    assert_eq!((l.key.0, l.key.1), (p.logical_width(), p.logical_height()));
    assert_eq!(l.key.2, p.profile);
    assert!(h.offset <= l.max_scroll());
}

#[test]
fn help_does_not_change_engine_save_history_or_rng() {
    let engine = crate::session_store::tracked_engine::Engine::load(
        include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let before = engine.save().unwrap();
    let mut p = Painter::new(); let mut h = Help::default();
    for _ in 0..3 {
        h.event(&key(Keycode::F1, Mod::NOMOD, false), &p, true, false);
        h.event(&key(Keycode::Z, Mod::NOMOD, false), &p, true, false);
        h.paint(&mut p);
        h.event(&key(Keycode::Escape, Mod::NOMOD, false), &p, true, false);
    }
    assert_eq!(engine.save().unwrap(), before);
}

#[test]
fn help_native_scaled_captures() {
    let Ok(dir) = std::env::var("CORPSE_HELP_CAPTURE") else { return; };
    std::fs::create_dir_all(&dir).unwrap();
    for (name, width, height, large, bottom) in [
        ("default", 1280, 800, false, false),
        ("small", 800, 500, false, false),
        ("wide", 1920, 1080, false, false),
        ("portrait", 900, 1200, false, false),
        ("large-top", 1280, 800, true, false),
        ("large-bottom", 1280, 800, true, true),
    ] {
        let mut p = if large { large_painter() } else { Painter::new() };
        p.resize(width, height); p.set_responsive_canvas(true); p.clear([8, 8, 12]);
        let mut h = Help::default(); h.open(&p);
        if bottom { h.event(&key(Keycode::End, Mod::NOMOD, false), &p, true, false); }
        h.paint(&mut p);
        let file = std::fs::File::create(std::path::Path::new(&dir).join(format!("help-{name}.png"))).unwrap();
        let mut encoder = png::Encoder::new(file, p.width as u32, p.height as u32);
        encoder.set_color(png::ColorType::Rgba); encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
    }
}

#[test]
fn title_help_hover_changes_pixels_only_inside_the_click_target() {
    let mut p = Painter::new();
    paint_menu_button(&mut p, false);
    let before = p.pixels.clone();
    paint_menu_button(&mut p, true);
    let mut changed = 0;
    for (index, (a, b)) in before.chunks_exact(4).zip(p.pixels.chunks_exact(4)).enumerate() {
        if a != b {
            assert!(menu_button_at((index % p.width) as i32, (index / p.width) as i32));
            changed += 1;
        }
    }
    assert!(changed > 100);
}
