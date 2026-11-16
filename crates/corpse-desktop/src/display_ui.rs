//! Display-only draft settings. No simulation, renderer profile, SDL, or disk mutation.
use crate::{
    appearance_store::{ActionAnimationMode, DisplayMode, Resolution},
    render::{Painter, Rgb},
};
use sdl2::keyboard::Keycode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    None,
    Apply {
        mode: DisplayMode,
        resolution: Option<Resolution>,
        action_animation_mode: ActionAnimationMode,
    },
    Back,
}
#[derive(Clone, Copy)]
struct Bounds {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}
impl Bounds {
    fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}
const ROWS: [Bounds; 5] = [
    Bounds {
        x: 272,
        y: 286,
        w: 736,
        h: 68,
    },
    Bounds {
        x: 272,
        y: 386,
        w: 736,
        h: 68,
    },
    Bounds {
        x: 272,
        y: 486,
        w: 736,
        h: 52,
    },
    Bounds {
        x: 650,
        y: 556,
        w: 358,
        h: 60,
    },
    Bounds {
        x: 272,
        y: 556,
        w: 350,
        h: 60,
    },
];
const BONE: Rgb = [222, 213, 191];
const GOLD: Rgb = [183, 145, 83];
const ASH: Rgb = [137, 128, 116];
pub struct DisplaySettings {
    mode: DisplayMode,
    desktop: Resolution,
    resolutions: Vec<Resolution>,
    selected_resolution: usize,
    action_animation_mode: ActionAnimationMode,
    selected: usize,
    pub notice: String,
}
impl DisplaySettings {
    /// O(min(n,256) log min(n,256)) on opening only; bounded copy, sort and dedup.
    /// Invalid inputs are discarded; the fixed fallback keeps selection safe.
    pub fn new(
        mode: DisplayMode,
        resolution: Option<Resolution>,
        desktop: Resolution,
        available: Vec<Resolution>,
    ) -> Self {
        let desktop = if desktop.valid() {
            desktop
        } else {
            Resolution {
                width: 1280,
                height: 800,
            }
        };
        let mut resolutions: Vec<_> = available
            .into_iter()
            .take(256)
            .filter(|r| r.valid())
            .collect();
        if resolutions.is_empty() {
            resolutions.push(desktop);
        }
        resolutions.sort_unstable();
        resolutions.dedup();
        // Keep a valid saved manual size even after changing monitors.
        if let Some(saved) = resolution.filter(|r| r.valid()) {
            if !resolutions.contains(&saved) {
                resolutions.push(saved);
                resolutions.sort_unstable();
            }
        }
        // Index zero is automatic native pixels; manual choices start at one.
        let selected_resolution = resolution
            .and_then(|r| resolutions.iter().position(|v| *v == r))
            .map(|index| index + 1)
            .unwrap_or(0);
        Self {
            mode,
            desktop,
            resolutions,
            selected_resolution,
            action_animation_mode: ActionAnimationMode::Normal,
            selected: 0,
            notice: String::new(),
        }
    }
    pub fn with_action_animation_mode(mut self, mode: ActionAnimationMode) -> Self {
        self.action_animation_mode = mode;
        self
    }
    fn resolution(&self) -> Option<Resolution> {
        if self.mode == DisplayMode::Borderless || self.selected_resolution == 0 {
            None
        } else {
            Some(self.resolutions[self.selected_resolution - 1])
        }
    }
    fn adjust(&mut self, forward: bool) {
        match self.selected {
            0 => {
                self.mode = if forward {
                    self.mode.next()
                } else {
                    self.mode.next().next()
                }
            }
            1 if self.mode != DisplayMode::Borderless => {
                let count = self.resolutions.len() + 1;
                self.selected_resolution =
                    (self.selected_resolution + if forward { 1 } else { count - 1 }) % count;
            }
            2 => {
                self.action_animation_mode = if forward {
                    self.action_animation_mode.next()
                } else {
                    self.action_animation_mode.next().next()
                };
            }
            _ => {}
        }
    }
    fn activate(&mut self) -> Action {
        match self.selected {
            0 | 1 | 2 => {
                self.adjust(true);
                Action::None
            }
            3 => Action::Apply {
                mode: self.mode,
                resolution: self.resolution(),
                action_animation_mode: self.action_animation_mode,
            },
            _ => Action::Back,
        }
    }
    pub fn key(&mut self, key: Keycode) -> Action {
        match key {
            Keycode::Escape => return Action::Back,
            Keycode::Up => self.selected = (self.selected + ROWS.len() - 1) % ROWS.len(),
            Keycode::Down | Keycode::Tab => self.selected = (self.selected + 1) % ROWS.len(),
            Keycode::Left => self.adjust(false),
            Keycode::Right => self.adjust(true),
            Keycode::Return | Keycode::KpEnter | Keycode::Space => return self.activate(),
            _ => {}
        }
        Action::None
    }
    pub fn click(&mut self, x: i32, y: i32) -> Action {
        if let Some(index) = ROWS.iter().position(|r| r.contains(x, y)) {
            self.selected = index;
            if index < 3 {
                self.adjust(x >= 560);
                Action::None
            } else {
                self.activate()
            }
        } else {
            Action::None
        }
    }
    pub fn draw(&self, p: &mut Painter) {
        p.clear([10, 9, 10]);
        p.rect(224, 128, 832, 548, [28, 23, 22]);
        p.rect(226, 130, 828, 544, [15, 13, 14]);
        p.rect(272, 163, 58, 2, GOLD);
        p.text("SETTINGS", 272, 194, 38, BONE);
        for (index, r) in ROWS.iter().enumerate() {
            let disabled = index == 1 && self.mode == DisplayMode::Borderless;
            let selected = self.selected == index && !disabled;
            p.rect(
                r.x,
                r.y,
                r.w,
                r.h,
                if selected { GOLD } else { [58, 47, 40] },
            );
            p.rect(
                r.x + 1,
                r.y + 1,
                r.w - 2,
                r.h - 2,
                if selected { [39, 30, 24] } else { [22, 18, 18] },
            );
        }
        p.text("DISPLAY MODE", 296, 300, 13, ASH);
        let mode = match self.mode {
            DisplayMode::Windowed => "Windowed",
            DisplayMode::Borderless => "Borderless fullscreen",
            DisplayMode::Fullscreen => "Fullscreen",
        };
        p.text(mode, 565, 307, 22, BONE);
        p.text("<", 535, 308, 20, GOLD);
        p.text(">", 969, 308, 20, GOLD);
        p.text("RESOLUTION", 296, 402, 13, ASH);
        let label = match self.resolution() {
            Some(resolution) => format!("{} x {}", resolution.width, resolution.height),
            None => "Auto (native pixels)".to_owned(),
        };
        p.text(
            &label,
            565,
            407,
            22,
            if self.mode == DisplayMode::Borderless {
                ASH
            } else {
                BONE
            },
        );
        if self.mode != DisplayMode::Borderless {
            p.text("<", 535, 408, 20, GOLD);
            p.text(">", 969, 408, 20, GOLD);
        }
        p.text("ACTION ANIMATIONS", 296, 504, 13, ASH);
        p.text(self.action_animation_mode.label(), 565, 500, 22, BONE);
        p.text("<", 535, 501, 20, GOLD);
        p.text(">", 969, 501, 20, GOLD);
        if !self.notice.is_empty() {
            p.text(
                &crate::render::truncate(&self.notice, 82),
                274,
                637,
                14,
                [208, 140, 110],
            );
        }
        p.text("BACK", 412, 576, 21, BONE);
        p.text("APPLY", 790, 576, 21, BONE);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn view() -> DisplaySettings {
        DisplaySettings::new(
            DisplayMode::Windowed,
            None,
            Resolution {
                width: 1920,
                height: 1080,
            },
            vec![
                Resolution {
                    width: 1280,
                    height: 800,
                },
                Resolution {
                    width: 1920,
                    height: 1080,
                },
            ],
        )
    }
    #[test]
    fn action_animation_draft_cycles_both_directions_and_cancel_has_no_commit() {
        let mut v = view().with_action_animation_mode(ActionAnimationMode::Reduced);
        assert_eq!(v.click(900, 510), Action::None);
        assert_eq!(v.action_animation_mode, ActionAnimationMode::Instant);
        v.key(Keycode::Right);
        assert_eq!(v.action_animation_mode, ActionAnimationMode::Normal);
        v.key(Keycode::Left);
        assert_eq!(v.action_animation_mode, ActionAnimationMode::Instant);
        v.click(300, 510);
        assert_eq!(v.action_animation_mode, ActionAnimationMode::Reduced);
        assert_eq!(v.key(Keycode::Escape), Action::Back);
        let mut reopened = view().with_action_animation_mode(ActionAnimationMode::Normal);
        assert_eq!(
            reopened.click(800, 580),
            Action::Apply {
                mode: DisplayMode::Windowed,
                resolution: None,
                action_animation_mode: ActionAnimationMode::Normal
            }
        );
        assert_eq!(
            v.click(800, 580),
            Action::Apply {
                mode: DisplayMode::Windowed,
                resolution: None,
                action_animation_mode: ActionAnimationMode::Reduced
            }
        );
    }
    #[test]
    fn action_animation_settings_native_frames_at_multiple_scales() {
        for (width, height) in [(1280, 800), (1600, 900), (1920, 1200)] {
            for mode in [
                ActionAnimationMode::Normal,
                ActionAnimationMode::Reduced,
                ActionAnimationMode::Instant,
            ] {
                let mut v = view().with_action_animation_mode(mode);
                v.selected = 2;
                v.notice = "Applied.".into();
                let mut p = Painter::new();
                p.resize(width, height);
                v.draw(&mut p);
                assert!(p.pixels.chunks_exact(4).any(|p| p[..3] == GOLD));
                if let Ok(dir) = std::env::var("CORPSE_R52_SETTINGS_OUTPUT") {
                    std::fs::create_dir_all(&dir).unwrap();
                    let file = std::fs::File::create(format!(
                        "{dir}/action-animations-{}-{width}x{height}.png",
                        mode.label().to_lowercase()
                    ))
                    .unwrap();
                    let mut encoder = png::Encoder::new(file, width as u32, height as u32);
                    encoder.set_color(png::ColorType::Rgba);
                    encoder.set_depth(png::BitDepth::Eight);
                    encoder
                        .write_header()
                        .unwrap()
                        .write_image_data(&p.pixels)
                        .unwrap();
                }
            }
        }
    }
    #[test]
    fn action_animation_keyboard_traverses_all_rows_and_wraps() {
        let mut v = view();
        v.key(Keycode::Up);
        assert_eq!(v.key(Keycode::Return), Action::Back);
        v.key(Keycode::Down);
        assert_eq!(v.selected, 0);
        v.key(Keycode::Tab);
        v.key(Keycode::Tab);
        v.key(Keycode::Right);
        assert_eq!(v.action_animation_mode, ActionAnimationMode::Reduced);
        v.key(Keycode::Down);
        assert_eq!(
            v.key(Keycode::Return),
            Action::Apply {
                mode: DisplayMode::Windowed,
                resolution: None,
                action_animation_mode: ActionAnimationMode::Reduced
            }
        );
    }
    #[test]
    fn automatic_is_default_and_can_be_restored_after_manual_selection() {
        let mut v = view();
        assert_eq!(
            v.click(800, 580),
            Action::Apply {
                mode: DisplayMode::Windowed,
                resolution: None,
                action_animation_mode: ActionAnimationMode::Normal
            }
        );
        v.click(900, 410);
        assert_eq!(
            v.resolution(),
            Some(Resolution {
                width: 1280,
                height: 800
            })
        );
        v.key(Keycode::Left);
        assert_eq!(v.resolution(), None);
        assert_eq!(
            v.click(800, 580),
            Action::Apply {
                mode: DisplayMode::Windowed,
                resolution: None,
                action_animation_mode: ActionAnimationMode::Normal
            }
        );
    }
    #[test]
    fn reopening_preserves_explicit_size_missing_from_new_monitor_modes() {
        let saved = Resolution {
            width: 1440,
            height: 900,
        };
        let mut v = DisplaySettings::new(
            DisplayMode::Windowed,
            Some(saved),
            Resolution {
                width: 1920,
                height: 1080,
            },
            vec![],
        );
        assert_eq!(
            v.click(800, 580),
            Action::Apply {
                mode: DisplayMode::Windowed,
                resolution: Some(saved),
                action_animation_mode: ActionAnimationMode::Normal
            }
        );
    }
    #[test]
    fn cancel_does_not_apply_and_keyboard_mouse_share_actions() {
        let mut v = view();
        assert_eq!(v.key(Keycode::Right), Action::None);
        assert_eq!(v.key(Keycode::Escape), Action::Back);
        v.key(Keycode::Down);
        v.key(Keycode::Down);
        v.key(Keycode::Down);
        assert_eq!(v.key(Keycode::Return), v.click(800, 580));
    }
    #[test]
    fn borderless_locks_to_desktop_and_preserves_window_selection() {
        let mut v = view();
        v.key(Keycode::Down);
        v.key(Keycode::Right);
        assert_eq!(v.resolution().unwrap().width, 1280);
        v.key(Keycode::Up);
        v.key(Keycode::Right);
        v.key(Keycode::Down);
        v.key(Keycode::Right);
        assert_eq!(v.resolution(), None);
        v.key(Keycode::Up);
        v.key(Keycode::Left);
        assert_eq!(v.resolution().unwrap().width, 1280);
    }
    #[test]
    fn input_bounds_are_exclusive_and_gaps_do_nothing() {
        let mut v = view();
        for r in ROWS {
            assert_eq!(v.click(r.x - 1, r.y), Action::None);
            assert_eq!(v.click(r.x + r.w, r.y), Action::None);
            assert_eq!(v.click(r.x, r.y + r.h), Action::None);
        }
        assert_eq!(v.click(800, 550), Action::None);
        assert_eq!(v.click(272, 556), Action::Back);
    }
    #[test]
    fn empty_invalid_duplicate_inputs_are_safe_and_bounded() {
        let r = Resolution {
            width: 1280,
            height: 800,
        };
        let mut v = DisplaySettings::new(DisplayMode::Windowed, None, r, vec![]);
        for _ in 0..20 {
            v.key(Keycode::Down);
            v.key(Keycode::Left);
        }
        assert!(v.resolution().is_none_or(|value| value == r));
        let v = DisplaySettings::new(DisplayMode::Windowed, None, r, vec![r; 1000]);
        assert_eq!(v.resolutions, vec![r]);
    }
}
