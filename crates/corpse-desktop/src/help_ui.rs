//! Detached, read-only command reference. No Run, dispatch, RNG or storage access.
use crate::{
    appearance_profile::Profile,
    modal_ui::{Rect, TextMetrics},
    render::Painter,
    typography_ui::Role,
};
use sdl2::{event::Event, keyboard::{Keycode, Mod}, mouse::{MouseButton, MouseWheelDirection}};

pub const ACTION: usize = 10;
pub const LABEL_COLOR: [u8; 3] = [255, 217, 53];
pub const DESCRIPTION_COLOR: [u8; 3] = [255, 255, 255];
const BACKGROUND: [u8; 3] = [20, 20, 26];
const BORDER: [u8; 3] = [126, 117, 95];
const BODY_SIZE: u16 = 16;
const MENU_BUTTON: Rect = Rect(1212, 18, 48, 42);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry { pub label: &'static str, pub description: &'static str }
pub const COMMANDS: [Entry; 10] = [
    Entry { label: "mouse", description: "move cursor to examine terrain and monsters or activate divine abilities" },
    Entry { label: "click", description: "travel" },
    Entry { label: "right click", description: "place or move the rally point on the map" },
    Entry { label: "enter", description: "enable keyboard cursor control" },
    Entry { label: "esc", description: "exit keyboard cursor control, close panels, or cancel targeting" },
    Entry { label: "numpad", description: "move or attack" },
    Entry { label: "I", description: "open inventory" },
    Entry { label: "Z", description: "rest" },
    Entry { label: "Shift Z", description: "rest until change" },
    Entry { label: "x", description: "autoexplore" },
];
pub const INFORMATION: [Entry; 3] = [
    Entry { label: "Divine Abilities", description: "Eat hearts to gain stats and blessings to level up and gain new abilities" },
    Entry { label: "Energy", description: "Used for abilities and amulets" },
    Entry { label: "Reserve Energy", description: "Free energy provided by the ability or amulet that is used on that ability or amulet" },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Geometry {
    pub bounds: Rect,
    pub content: Rect,
    pub close: Rect,
    pub up: Rect,
    pub down: Rect,
    pub label_width: i32,
    pub description_x: i32,
    pub description_width: i32,
}
impl Geometry {
    pub fn resolve(width: i32, height: i32) -> Self {
        // Live logical canvases are at least 1280 x 800. The bounded smaller
        // cases support deterministic layout tests, not new window policy.
        let width = width.max(320);
        let height = height.max(240);
        let w = 960.min(width - 24);
        let h = 716.min(height - 24);
        let x = (width - w) / 2;
        let y = (height - h) / 2;
        let content = Rect(x + 24, y + 70, w - 72, h - 144);
        let label_width = 220.min((content.2 - 24) / 3);
        let description_x = content.0 + label_width + 24;
        Self {
            bounds: Rect(x, y, w, h), content,
            close: Rect(x + w - 124, y + 14, 100, 40),
            up: Rect(x + w - 136, y + h - 54, 48, 36),
            down: Rect(x + w - 76, y + h - 54, 48, 36),
            label_width, description_x,
            description_width: content.2 - label_width - 24,
        }
    }
}
#[derive(Clone, Debug)]
struct Line { text: String, x: i32, y: i32, color: [u8; 3] }
#[derive(Clone)]
struct Layout {
    key: (i32, i32, Profile),
    geometry: Geometry,
    lines: Vec<Line>,
    line_height: i32,
    content_height: i32,
}
impl Layout {
    /// Cache miss only: O(T² + U*F) measured wrap/metrics work, O(T + U)
    /// storage for fixed authored text T, distinct glyphs U, font lookup F.
    fn new(p: &Painter) -> Self {
        let mut geometry = Geometry::resolve(p.logical_width(), p.logical_height());
        let size = |role, baseline| crate::typography_ui::resolve(&p.profile, role,
            crate::typography_ui::RoleStyle::baseline(baseline)).font_size.ceil() as i32;
        let body_size = size(Role::Primary, BODY_SIZE as f64);
        let header = 70.max(size(Role::Headings, 26.) + 44).max(body_size + 32);
        let footer = 74.max(body_size + 44).max(size(Role::Secondary, 13.) + 42);
        geometry.content.1 = geometry.bounds.1 + header;
        geometry.content.3 = (geometry.bounds.3 - header - footer).max(1);
        geometry.close.3 = 40.max(body_size + 16);
        for button in [&mut geometry.up, &mut geometry.down] {
            button.3 = 36.max(body_size + 16);
            button.1 = geometry.bounds.1 + geometry.bounds.3 - 18 - button.3;
        }
        let mut metrics = TextMetrics::new(p, Role::Primary, BODY_SIZE, 24);
        // Even an authored line-height below 1 cannot overlap adjacent rows.
        metrics.line = metrics.line.max(body_size + 4);
        for entry in COMMANDS.iter().chain(INFORMATION.iter()) {
            metrics.ensure(p, entry.label); metrics.ensure(p, entry.description);
        }
        let mut lines = Vec::new();
        let mut y = 0;
        for (section, entries) in [&COMMANDS[..], &INFORMATION[..]].into_iter().enumerate() {
            if section == 1 {
                y += 10;
                for text in metrics.wrap("Information", geometry.content.2) {
                    lines.push(Line { text, x: geometry.content.0, y, color: DESCRIPTION_COLOR });
                    y += metrics.line;
                }
                y += 12;
            }
            for entry in entries {
                let left = metrics.wrap(entry.label, geometry.label_width);
                let right = metrics.wrap(entry.description, geometry.description_width);
                let height = left.len().max(right.len()) as i32 * metrics.line;
                for (column, x, color) in [(&left, geometry.content.0, LABEL_COLOR),
                    (&right, geometry.description_x, DESCRIPTION_COLOR)] {
                    for (i, text) in column.iter().enumerate() {
                        lines.push(Line { text: text.clone(), x, y: y + i as i32 * metrics.line, color });
                    }
                }
                y += height + 10;
            }
        }
        Self { key: (p.logical_width(), p.logical_height(), p.profile.clone()),
            geometry, lines, line_height: metrics.line, content_height: y - 10 }
    }
    fn max_scroll(&self) -> i32 { (self.content_height - self.geometry.content.3).max(0) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Response { Ignored, Consumed, Opened, Closed }
#[derive(Default)]
pub struct Help {
    active: bool,
    offset: i32,
    layout: Option<Layout>,
}
impl Help {
    pub fn active(&self) -> bool { self.active }
    fn configure(&mut self, p: &Painter) {
        if self.layout.as_ref().is_none_or(|l| l.key.0 != p.logical_width()
            || l.key.1 != p.logical_height() || l.key.2 != p.profile) {
            self.layout = Some(Layout::new(p));
        }
        self.offset = self.offset.clamp(0, self.layout.as_ref().unwrap().max_scroll());
    }
    pub fn open(&mut self, p: &Painter) {
        self.active = true; self.offset = 0; self.configure(p);
    }
    fn scroll(&mut self, pixels: i32) {
        let max = self.layout.as_ref().unwrap().max_scroll();
        self.offset = self.offset.saturating_add(pixels).clamp(0, max);
    }
    /// O(1) routing plus O(profile-size) cache-key comparison per help event;
    /// cache misses use Layout::new. All dismissed/blocked input is consumed.
    pub fn event(&mut self, event: &Event, p: &Painter,
        allow_open: bool, help_button: bool) -> Response {
        let shortcut = match event {
            Event::KeyDown { keycode: Some(key), keymod, repeat: false, .. } =>
                !keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD | Mod::LALTMOD | Mod::RALTMOD | Mod::LGUIMOD | Mod::RGUIMOD)
                && (*key == Keycode::F1 || (*key == Keycode::Slash
                    && keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD))),
            _ => false,
        };
        let click_open = help_button && matches!(event,
            Event::MouseButtonDown { mouse_btn: MouseButton::Left, .. });
        if !self.active {
            if allow_open && (shortcut || click_open) { self.open(p); return Response::Opened; }
            return Response::Ignored;
        }
        self.configure(p);
        let layout = self.layout.as_ref().unwrap();
        let geometry = layout.geometry;
        let line = layout.line_height;
        let page = (geometry.content.3 - line).max(line);
        let max = layout.max_scroll();
        let mut close = shortcut;
        match event {
            Event::KeyDown { keycode: Some(Keycode::Return | Keycode::KpEnter), keymod, .. }
                if keymod.intersects(Mod::LALTMOD | Mod::RALTMOD) => return Response::Ignored,
            Event::KeyDown { keycode: Some(key), repeat, .. } => match *key {
                Keycode::Escape | Keycode::Return | Keycode::KpEnter | Keycode::Space if !*repeat => close = true,
                Keycode::Up | Keycode::Kp8 => self.scroll(-line),
                Keycode::Down | Keycode::Kp2 => self.scroll(line),
                Keycode::PageUp => self.scroll(-page),
                Keycode::PageDown => self.scroll(page),
                Keycode::Home => self.offset = 0,
                Keycode::End => self.offset = max,
                _ => {},
            },
            Event::MouseButtonDown { x, y, mouse_btn: MouseButton::Left, .. } => {
                if geometry.close.contains(*x, *y) || !geometry.bounds.contains(*x, *y) { close = true; }
                else if geometry.up.contains(*x, *y) { self.scroll(-page); }
                else if geometry.down.contains(*x, *y) { self.scroll(page); }
            },
            Event::MouseWheel { y, direction, .. } => {
                let rows = if *direction == MouseWheelDirection::Flipped { *y } else { y.saturating_neg() };
                self.scroll(rows.saturating_mul(line).saturating_mul(3));
            },
            Event::MouseMotion { .. } | Event::MouseButtonDown { .. } | Event::MouseButtonUp { .. }
                | Event::KeyDown { .. } | Event::KeyUp { .. } | Event::TextInput { .. }
                | Event::TextEditing { .. } => {},
            // Preserve close-window, focus, resize, display and system events.
            _ => return Response::Ignored,
        }
        if close { self.active = false; Response::Closed } else { Response::Consumed }
    }
    /// O(L + G + A) paint for retained lines L, visible glyph/raster work G and
    /// native filled pixels A. No measuring/wrapping after an unchanged key.
    pub fn paint(&mut self, p: &mut Painter) {
        if !self.active { return; }
        self.configure(p);
        let layout = self.layout.as_ref().unwrap();
        let g = layout.geometry;
        let prior_clip = p.clip;
        p.clip = Some(g.bounds.tuple());
        p.rect(g.bounds.0, g.bounds.1, g.bounds.2, g.bounds.3, BACKGROUND);
        p.border(g.bounds.0, g.bounds.1, g.bounds.2, g.bounds.3, BORDER);
        let title = p.fit_text_role(Role::Headings, "Commands", g.close.0 - g.bounds.0 - 48, 26);
        p.text_role(Role::Headings, &title, g.bounds.0 + 24, g.bounds.1 + 22, 26, DESCRIPTION_COLOR);
        button(p, g.close, "Close", true);
        p.clip = Some(g.content.tuple());
        for line in &layout.lines {
            let y = g.content.1 + line.y - self.offset;
            if y + layout.line_height > g.content.1 && y < g.content.1 + g.content.3 {
                p.text_role(Role::Primary, &line.text, line.x, y, BODY_SIZE, line.color);
            }
        }
        p.clip = Some(g.bounds.tuple());
        let max = layout.max_scroll();
        if max > 0 {
            let track = Rect(g.bounds.0 + g.bounds.2 - 32, g.content.1, 8, g.content.3);
            let thumb = (track.3 as i64 * track.3 as i64 / layout.content_height as i64) as i32;
            let thumb = thumb.max(20).min(track.3);
            let top = track.1 + ((track.3 - thumb) as i64 * self.offset as i64 / max as i64) as i32;
            p.rect(track.0, track.1, track.2, track.3, [42, 42, 50]);
            p.rect(track.0, top, track.2, thumb, LABEL_COLOR);
        }
        button(p, g.up, "↑", self.offset > 0);
        button(p, g.down, "↓", self.offset < max);
        let footer = p.fit_text_role(Role::Secondary, "Esc to close   ↑/↓ or wheel to scroll", g.up.0 - g.bounds.0 - 42, 13);
        p.text_role(Role::Secondary, &footer, g.bounds.0 + 24, g.up.1 + 11, 13, DESCRIPTION_COLOR);
        p.clip = prior_clip;
    }
}
fn button(p: &mut Painter, rect: Rect, text: &str, enabled: bool) {
    p.rect(rect.0, rect.1, rect.2, rect.3, [35, 35, 43]);
    p.border(rect.0, rect.1, rect.2, rect.3, BORDER);
    let label = p.fit_text_role(Role::Primary, text, rect.2 - 12, 16);
    let width = p.measure_text_role(Role::Primary, &label, 16).ceil() as i32;
    p.text_role(Role::Primary, &label, rect.0 + (rect.2 - width) / 2, rect.1 + 9, 16,
        if enabled { DESCRIPTION_COLOR } else { [126, 126, 136] });
}
pub fn menu_button_at(x: i32, y: i32) -> bool { MENU_BUTTON.contains(x, y) }
pub fn paint_menu_button(p: &mut Painter, hovered: bool) {
    if !hovered { button(p, MENU_BUTTON, "?", true); return; }
    let Rect(x, y, w, h) = MENU_BUTTON;
    p.rect(x, y, w, h, crate::render::game_color(&p.profile, "ui.colors.hover"));
    p.border(x, y, w, h, BORDER);
    p.rect(x + 7, y + h - 4, w - 14, 2, crate::render::game_color(&p.profile, "ui.colors.blessing"));
    let width = p.measure_text_role(Role::Primary, "?", 16).ceil() as i32;
    p.text_role(Role::Primary, "?", x + (w - width) / 2, y + 9, 16,
        crate::render::game_color(&p.profile, "ui.colors.primary"));
}

#[cfg(test)]
#[path = "help_ui_tests.rs"]
mod tests;
