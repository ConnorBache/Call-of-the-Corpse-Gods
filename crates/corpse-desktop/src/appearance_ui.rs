//! Fixed-geometry native appearance studio. Reads cosmetic data only.
//! Source controls and validation are retained; renderer coverage is disclosed.
pub use crate::appearance_profile::Profile;
use crate::{
    appearance_profile::{controls, Control},
    appearance_store::{DisplayMode, Store},
    render::{Painter, GOLD, INK, MUTED},
};
use sdl2::keyboard::{Keycode, Mod};
use serde_json::Value;
use std::path::Path;
pub const CATEGORIES: [&str; 10] = [
    "Global Rendering",
    "Terrain",
    "Lighting",
    "Fog of War",
    "Creatures and Items",
    "Glyphs and Fonts",
    "Interface Colors",
    "UI Layout",
    "Path Highlighting",
    "Presets and Export",
];
#[derive(Debug, PartialEq)]
pub enum Action {
    None,
    Changed,
    ReducedMotion(bool),
    Fullscreen,
    Close,
    Copy(String),
    Paste,
}
#[derive(Clone, PartialEq)]
enum Focus {
    Search,
    Value(usize),
    PresetName,
    FilePath,
    Json,
}
#[derive(Clone)]
enum Confirmation {
    Delete(String),
    Overwrite(String),
}
pub struct SettingsView {
    pub store: Store,
    pub reduced_motion: bool,
    pub display_mode: DisplayMode,
    pub notice: String,
    category: usize,
    group: usize,
    selected: usize,
    offset: usize,
    query: String,
    focus: Option<Focus>,
    input: String,
    preset_name: String,
    preset: usize,
    file_path: String,
    json: String,
    confirm: Option<Confirmation>,
}
/// Keep UI disclosure aligned with actual renderer integrations, not schema presence.
fn integrated_layout(path: &str) -> bool {
    [
        "rightSidebarWidth",
        "sidebarWidthPercent",
        "resourceRegionHeight",
        "abilityRowHeight",
        "chronicleHeight",
        "mapSpacing",
        "inventoryRowHeight",
        "tooltipMaxWidth",
        "buttonHeight",
        "buttonPadding",
        "borderWidth",
    ]
    .iter()
    .any(|key| path == format!("ui.layout.{key}"))
}
fn integrated_role(path: &str) -> bool {
    [
        "primary",
        "secondary",
        "headings",
        "chronicle",
        "ability",
        "tooltips",
        "inventory",
    ]
    .iter()
    .any(|key| path.starts_with(&format!("typography.{key}.")))
}
const FIXED_PLAYER_GLYPH: &str = "Player glyph is fixed to @";
fn fixed_player_glyph(path: &str) -> bool {
    path == "entities.objects.player.glyph"
}
fn retained_only(path: &str) -> bool {
    if (path.starts_with("ui.layout.") && !integrated_layout(path))
        || (path.starts_with("typography.")
            && !path.starts_with("typography.map.")
            && !integrated_role(path))
    {
        return true;
    }
    if path.starts_with("ui.colors.") {
        return ![
            "primary",
            "secondary",
            "headings",
            "health",
            "energy",
            "blessing",
            "background",
            "chronicle",
            "panel",
            "border",
            "hover",
            "selected",
            "disabled",
            "button",
            "tooltip",
        ]
        .iter()
        .any(|key| path == format!("ui.colors.{key}"));
    }
    if path.starts_with("terrain.gas:")
        && (path.ends_with(".animationSpeed") || path.ends_with(".glyph"))
    {
        return true;
    }
    if path.starts_with("entities.objects.") {
        return [
            "player", "corpse", "vent", "light", "torch", "brazier", "beacon",
        ]
        .iter()
        .all(|key| !path.starts_with(&format!("entities.objects.{key}.")));
    }
    false
}
fn partially_applied(path: &str) -> bool {
    path.starts_with("terrain.feature:")
        || path.starts_with("terrain.gas:")
        || path == "fog.transition"
        || path.starts_with("fog.allSeeing")
        || integrated_role(path)
        || integrated_layout(path)
        || [
            "panel", "border", "hover", "selected", "disabled", "button", "tooltip",
        ]
        .iter()
        .any(|key| path == format!("ui.colors.{key}"))
}
fn changed_notice(path: &str) -> &'static str {
    if partially_applied(path) {
        "Appearance updated in supported views."
    } else if retained_only(path) {
        "Saved in working appearance and export. This setting is not yet applied by the native renderer."
    } else {
        "Appearance updated."
    }
}

fn display(v: Option<&Value>) -> String {
    match v {
        Some(Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
        None => String::new(),
    }
}
fn button(p: &mut Painter, label: &str, x: i32, y: i32, w: i32, active: bool) {
    p.rect(
        x,
        y,
        w,
        30,
        if active { [53, 51, 68] } else { [21, 25, 35] },
    );
    p.border(x, y, w, 30, [53, 55, 68]);
    p.text(label, x + 8, y + 5, 14, if active { GOLD } else { INK });
}
fn field(p: &mut Painter, label: &str, value: &str, x: i32, y: i32, w: i32, active: bool) {
    p.text(label, x, y - 19, 12, MUTED);
    p.rect(x, y, w, 30, [5, 4, 12]);
    p.border(x, y, w, 30, if active { GOLD } else { [53, 55, 68] });
    p.text(
        &crate::render::truncate(value, (w / 8 - 2).max(1) as usize),
        x + 7,
        y + 5,
        14,
        INK,
    );
}
impl SettingsView {
    pub fn load(path: &Path) -> Self {
        let store = Store::load(path);
        let notice = store.error.clone();
        let reduced_motion = store.reduced_motion;
        let display_mode = store.display_mode;
        Self {
            store,
            reduced_motion,
            display_mode,
            notice,
            category: 0,
            group: 0,
            selected: 0,
            offset: 0,
            query: String::new(),
            focus: None,
            input: String::new(),
            preset_name: String::new(),
            preset: 0,
            file_path: "corpse-gods-appearance.json".into(),
            json: String::new(),
            confirm: None,
        }
    }
    pub fn profile(&self) -> &Profile {
        self.store.profile()
    }
    pub fn open(&mut self) {
        self.store.open();
        self.focus = None;
        self.confirm = None;
    }
    /// Hide keeps committed edits, discards temporary preview, and clears comparison.
    pub fn close(&mut self) {
        self.store.close();
        self.focus = None;
        self.confirm = None;
    }
    pub fn import_file(&mut self, path: &Path) -> Action {
        if self.confirm.is_some() {
            self.notice = "Finish or cancel the confirmation before importing.".into();
            return Action::None;
        }
        let result = self.store.import_file(path);
        self.finish(
            result,
            "Appearance imported. Save a preset to keep it for later.",
        )
    }
    /// Commit only after the window confirms the requested mode.
    pub fn sync_display_mode(&mut self, value: DisplayMode) -> Result<(), String> {
        self.store.display_preferences(self.reduced_motion, value)?;
        self.display_mode = value;
        Ok(())
    }
    /// Reflect actual window state without overwriting the last saved preference.
    pub fn observe_display_mode(&mut self, value: DisplayMode) {
        self.display_mode = value;
    }
    fn finish(&mut self, result: Result<(), String>, success: &str) -> Action {
        match result {
            Ok(()) => {
                self.notice = success.into();
                Action::Changed
            }
            Err(e) => {
                self.notice = e;
                Action::None
            }
        }
    }
    fn groups(&self) -> Vec<&str> {
        let mut result = vec!["All entries"];
        if self.category >= 9 {
            return result;
        }
        for c in controls()
            .iter()
            .filter(|c| self.category == 10 || c.category == CATEGORIES[self.category])
        {
            if let Some(group) = c.group.as_deref() {
                if !result.contains(&group) {
                    result.push(group);
                }
            }
        }
        result
    }
    fn filtered(&self) -> Vec<usize> {
        if self.category == 9 {
            return Vec::new();
        }
        let groups = self.groups();
        let group = groups.get(self.group).copied().unwrap_or("All entries");
        let query = self.query.to_lowercase();
        controls()
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                (self.category == 10 || c.category == CATEGORIES[self.category])
                    && (self.group == 0 || c.group.as_deref() == Some(group))
                    && (query.is_empty()
                        || format!(
                            "{} {} {}",
                            c.path,
                            c.label,
                            c.group.as_deref().unwrap_or("")
                        )
                        .to_lowercase()
                        .contains(&query))
            })
            .map(|(i, _)| i)
            .collect()
    }
    fn change_category(&mut self, n: usize) {
        self.category = n.min(10);
        self.group = if matches!(self.category, 1 | 4 | 5) {
            1
        } else {
            0
        };
        self.selected = 0;
        self.offset = 0;
        self.focus = None;
    }
    fn focus(&mut self, focus: Focus) {
        if matches!(&focus, Focus::Value(i) if fixed_player_glyph(&controls()[*i].path)) {
            self.focus = None;
            self.notice = FIXED_PLAYER_GLYPH.into();
            return;
        }
        self.input = match &focus {
            Focus::Search => self.query.clone(),
            Focus::Value(i) => display(self.store.working().value(&controls()[*i].path)),
            Focus::PresetName => self.preset_name.clone(),
            Focus::FilePath => self.file_path.clone(),
            Focus::Json => self.json.clone(),
        };
        self.focus = Some(focus);
    }
    fn select_control(&mut self, relative: usize) {
        let list = self.filtered();
        if let Some(i) = list.get(relative) {
            self.selected = relative;
            self.focus(Focus::Value(*i));
        }
    }
    fn selected_control(&self) -> Option<&'static Control> {
        self.filtered().get(self.selected).map(|i| &controls()[*i])
    }
    fn edit_selected(&mut self, direction: i32) -> Action {
        let Some(c) = self.selected_control().cloned() else {
            return Action::None;
        };
        if fixed_player_glyph(&c.path) {
            self.focus = None;
            self.notice = FIXED_PLAYER_GLYPH.into();
            return Action::None;
        }
        let value = match c.kind.as_str() {
            "number" => {
                let n = (self.store.working().number(&c.path)
                    + direction as f64 * c.step.unwrap_or(0.01))
                .clamp(c.min.unwrap(), c.max.unwrap());
                Value::from((n * 1e10).round() / 1e10)
            }
            "boolean" => Value::Bool(!self.store.working().boolean(&c.path)),
            "select" => {
                let current = c
                    .options
                    .iter()
                    .position(|v| v == self.store.working().text(&c.path))
                    .unwrap_or(0);
                let i = (current as i32 + direction).rem_euclid(c.options.len() as i32) as usize;
                Value::String(c.options[i].clone())
            }
            _ => return Action::None,
        };
        let result = self.store.set(&c.path, value);
        self.finish(result, changed_notice(&c.path))
    }
    fn commit_input(&mut self) -> Action {
        let Some(focus) = self.focus.clone() else {
            return Action::None;
        };
        match focus {
            Focus::Value(i) => {
                let c = &controls()[i];
                if fixed_player_glyph(&c.path) {
                    self.focus = None;
                    self.notice = FIXED_PLAYER_GLYPH.into();
                    return Action::None;
                }
                let value = match c.kind.as_str() {
                    "number" => match self.input.parse::<f64>().ok().filter(|n| n.is_finite()) {
                        Some(n) => Value::from(n),
                        None => {
                            self.notice = "Enter a finite number.".into();
                            return Action::None;
                        }
                    },
                    "boolean" => match self.input.to_lowercase().as_str() {
                        "true" | "on" | "1" => Value::Bool(true),
                        "false" | "off" | "0" => Value::Bool(false),
                        _ => {
                            self.notice = "Enter true or false.".into();
                            return Action::None;
                        }
                    },
                    _ => Value::String(self.input.clone()),
                };
                let result = self.store.set(&c.path, value);
                if result.is_ok() {
                    self.focus = None;
                }
                self.finish(result, changed_notice(&c.path))
            }
            _ => {
                self.sync_input();
                self.focus = None;
                Action::None
            }
        }
    }
    fn sync_input(&mut self) {
        match &self.focus {
            Some(Focus::Search) => {
                self.query = self.input.clone();
                self.selected = 0;
                self.offset = 0;
            }
            Some(Focus::PresetName) => self.preset_name = self.input.clone(),
            Some(Focus::FilePath) => self.file_path = self.input.clone(),
            Some(Focus::Json) => self.json = self.input.clone(),
            _ => {}
        }
    }
    pub fn text_input(&mut self, text: &str) -> Action {
        if self.confirm.is_some() || self.focus.is_none() {
            return Action::None;
        }
        let limit = if self.focus == Some(Focus::Json) {
            crate::appearance_profile::MAX_JSON_CHARACTERS
        } else {
            1000
        };
        if crate::appearance_profile::utf16_len(&self.input)
            + crate::appearance_profile::utf16_len(text)
            > limit
        {
            self.notice = "Input is too long.".into();
            return Action::None;
        }
        self.input.push_str(text);
        self.sync_input();
        Action::None
    }
    fn page_rows(&self) -> usize {
        if self.category == 5 {
            5
        } else {
            6
        }
    }
    pub fn scroll(&mut self, delta: i32) {
        let len = self.filtered().len();
        self.offset = (self.offset as i32 - delta * 3)
            .clamp(0, len.saturating_sub(self.page_rows()) as i32) as usize;
    }
    pub fn key(&mut self, key: Keycode, mods: Mod) -> Action {
        if self.confirm.is_some() {
            return match key {
                Keycode::Return | Keycode::Y => self.confirm_action(),
                Keycode::Escape | Keycode::N => {
                    self.confirm = None;
                    Action::None
                }
                _ => Action::None,
            };
        }
        if key == Keycode::F8 || key == Keycode::Escape {
            self.close();
            return Action::Close;
        }
        let ctrl = mods.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD | Mod::LGUIMOD | Mod::RGUIMOD);
        let shift = mods.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD);
        if ctrl {
            match key {
                Keycode::Z => {
                    self.focus = None;
                    if shift {
                        self.store.redo();
                    } else {
                        self.store.undo();
                    }
                    return Action::Changed;
                }
                Keycode::Y => {
                    self.focus = None;
                    self.store.redo();
                    return Action::Changed;
                }
                Keycode::V if self.focus.is_some() => return Action::Paste,
                Keycode::A if self.focus.is_some() => {
                    self.input.clear();
                    self.sync_input();
                    return Action::None;
                }
                Keycode::C if self.focus == Some(Focus::Json) => {
                    return Action::Copy(self.json.clone())
                }
                _ => {}
            }
        }
        if self.focus.is_some() {
            return match key {
                Keycode::Escape => {
                    self.focus = None;
                    Action::None
                }
                Keycode::Return | Keycode::KpEnter => self.commit_input(),
                Keycode::Backspace => {
                    self.input.pop();
                    self.sync_input();
                    Action::None
                }
                _ => Action::None,
            };
        }
        match key {
            Keycode::Escape | Keycode::F8 => {
                self.close();
                Action::Close
            }
            Keycode::Tab => {
                self.change_category((self.category + if shift { 10 } else { 1 }) % 11);
                Action::None
            }
            Keycode::Slash => {
                self.focus(Focus::Search);
                Action::None
            }
            Keycode::Up | Keycode::Down => {
                let len = self.filtered().len();
                self.selected = (self.selected as i32 + if key == Keycode::Up { -1 } else { 1 })
                    .clamp(0, len.saturating_sub(1) as i32)
                    as usize;
                if self.selected < self.offset {
                    self.offset = self.selected;
                }
                if self.selected >= self.offset + self.page_rows() {
                    self.offset = self.selected.saturating_sub(self.page_rows() - 1);
                }
                Action::None
            }
            Keycode::PageUp => {
                self.scroll(2);
                Action::None
            }
            Keycode::PageDown => {
                self.scroll(-2);
                Action::None
            }
            Keycode::Left => self.edit_selected(-1),
            Keycode::Right => self.edit_selected(1),
            Keycode::Return => {
                self.select_control(self.selected);
                Action::None
            }
            Keycode::R => self.toggle_reduced(),
            Keycode::F => Action::Fullscreen,
            _ => Action::None,
        }
    }
    fn toggle_reduced(&mut self) -> Action {
        let value = !self.reduced_motion;
        match self
            .store
            .display_preferences(value, self.store.display_mode)
        {
            Ok(()) => {
                self.reduced_motion = value;
                Action::ReducedMotion(value)
            }
            Err(e) => {
                self.notice = e;
                Action::None
            }
        }
    }
    fn selected_preset(&self) -> Result<String, String> {
        self.store
            .presets()
            .get(self.preset)
            .map(|p| p.id.clone())
            .ok_or("Select a saved preset first.".into())
    }
    fn preset_action(&mut self, n: usize) -> Action {
        let id = self.selected_preset();
        let result: Result<(), String> = match n {
            0 => {
                let result = self.store.save_preset(&self.preset_name);
                match result {
                    Ok(id) => {
                        self.preset = self
                            .store
                            .presets()
                            .iter()
                            .position(|p| p.id == id)
                            .unwrap();
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            }
            1 => id.and_then(|id| self.store.update_preset(&id)),
            2 => id.and_then(|id| self.store.load_preset(&id)),
            3 => id
                .and_then(|id| self.store.duplicate_preset(&id, &self.preset_name))
                .map(|id| {
                    self.preset = self
                        .store
                        .presets()
                        .iter()
                        .position(|p| p.id == id)
                        .unwrap();
                }),
            4 => id.and_then(|id| self.store.rename_preset(&id, &self.preset_name)),
            5 => match id {
                Ok(id) => {
                    self.confirm = Some(Confirmation::Delete(id));
                    return Action::None;
                }
                Err(e) => Err(e),
            },
            6 => id.and_then(|id| self.store.set_default(Some(&id))),
            7 => self.store.set_default(None),
            8 => id.and_then(|id| self.store.compare(&id)),
            _ => Ok(()),
        };
        self.finish(result, "Appearance preset updated.")
    }
    fn confirm_action(&mut self) -> Action {
        let Some(action) = self.confirm.take() else {
            return Action::None;
        };
        let result = match action {
            Confirmation::Delete(id) => {
                let result = self.store.delete_preset(&id, true);
                self.preset = self
                    .preset
                    .min(self.store.presets().len().saturating_sub(1));
                result
            }
            Confirmation::Overwrite(path) => self.store.export_file(Path::new(&path)),
        };
        self.finish(result, "Done.")
    }
    pub fn click(&mut self, x: i32, y: i32) -> Action {
        if self.confirm.is_some() {
            if (420..560).contains(&x) && (420..450).contains(&y) {
                return self.confirm_action();
            }
            if (580..720).contains(&x) && (420..450).contains(&y) {
                self.confirm = None;
            }
            return Action::None;
        }
        if (80..310).contains(&x) && (108..140).contains(&y) {
            return self.toggle_reduced();
        }
        if (330..640).contains(&x) && (108..140).contains(&y) {
            return Action::Fullscreen;
        }
        if (1040..1210).contains(&x) && (108..140).contains(&y) {
            self.close();
            return Action::Close;
        }
        if (60..280).contains(&x) && (208..238).contains(&y) {
            self.change_category(10);
            return Action::None;
        }
        if (60..280).contains(&x) && (248..568).contains(&y) {
            self.change_category(((y - 248) / 32) as usize);
            return Action::None;
        }
        if (310..1180).contains(&x) && (208..238).contains(&y) {
            self.focus(Focus::Search);
            return Action::None;
        }
        if (310..1190).contains(&x) && (158..188).contains(&y) {
            self.focus = None;
            let n = ((x - 310) / 110) as usize;
            match n {
                0 => self.store.undo(),
                1 => self.store.redo(),
                2 => {
                    let _ = self.store.compare("original");
                }
                3 => {
                    let _ = self.store.compare("edited");
                }
                4 => self.store.begin_preview(),
                5 => self.store.commit_preview(),
                6 => self.store.cancel_preview(),
                7 => self.store.reset_all(),
                _ => {}
            }
            return Action::Changed;
        }
        if self.category == 9 {
            return self.preset_click(x, y);
        }
        if (310..1190).contains(&x) && (256..286).contains(&y) {
            let len = self.groups().len();
            self.group = (self.group + if x < 350 { len - 1 } else { 1 }) % len;
            self.offset = 0;
            self.selected = 0;
            self.focus = None;
            return Action::None;
        }
        if (310..1190).contains(&x) && (306..306 + self.page_rows() as i32 * 60).contains(&y) {
            let row = ((y - 306) / 60) as usize + self.offset;
            let list = self.filtered();
            if let Some(i) = list.get(row) {
                self.selected = row;
                let c = &controls()[*i];
                if fixed_player_glyph(&c.path) {
                    self.focus = None;
                    self.notice = FIXED_PLAYER_GLYPH.into();
                    return Action::None;
                }
                if x >= 1110 {
                    let result = self.store.reset_property(&c.path);
                    return self.finish(result, "Setting reset to project default.");
                }
                if (1000..1040).contains(&x) {
                    self.focus = None;
                    return self.edit_selected(-1);
                }
                if (1050..1090).contains(&x) {
                    self.focus = None;
                    return self.edit_selected(1);
                }
                self.focus(Focus::Value(*i));
            }
            return Action::None;
        }
        self.focus = None;
        Action::None
    }
    fn preset_click(&mut self, x: i32, y: i32) -> Action {
        if (310..1190).contains(&x) && (265..295).contains(&y) {
            let len = self.store.presets().len();
            if len > 0 {
                self.preset = (self.preset + if x < 350 { len - 1 } else { 1 }) % len;
            }
            return Action::None;
        }
        if (310..1190).contains(&x) && (325..355).contains(&y) {
            self.focus(Focus::PresetName);
            return Action::None;
        }
        if (310..1190).contains(&x) && (370..400).contains(&y) {
            self.focus = None;
            return self.preset_action(((x - 310) / 145) as usize);
        }
        if (310..1110).contains(&x) && (410..440).contains(&y) {
            self.focus = None;
            return self.preset_action(6 + ((x - 310) / 265) as usize);
        }
        if (310..1190).contains(&x) && (480..510).contains(&y) {
            self.focus(Focus::FilePath);
            return Action::None;
        }
        if (310..1190).contains(&x) && (520..550).contains(&y) {
            self.focus = None;
            match ((x - 310) / 215) as usize {
                0 => {
                    if Path::new(&self.file_path).exists() {
                        self.confirm = Some(Confirmation::Overwrite(self.file_path.clone()));
                        return Action::None;
                    }
                    let result = self.store.export_file(Path::new(&self.file_path));
                    return self.finish(result, "Appearance JSON exported.");
                }
                1 => {
                    let path = self.file_path.clone();
                    return self.import_file(Path::new(&path));
                }
                2 => {
                    self.json = self.store.export_json();
                    return Action::Copy(self.json.clone());
                }
                3 => {
                    self.focus(Focus::Json);
                    return Action::Paste;
                }
                _ => {}
            }
        }
        if (310..1190).contains(&x) && (585..615).contains(&y) {
            self.focus(Focus::Json);
            return Action::None;
        }
        if (310..550).contains(&x) && (628..658).contains(&y) {
            let text = self.json.clone();
            let result = self.store.import_json(&text);
            self.focus = None;
            return self.finish(
                result,
                "Appearance JSON imported. Save a preset to keep it.",
            );
        }
        self.focus = None;
        Action::None
    }
    pub fn paint(&self, p: &mut Painter) {
        // Intentionally fixed coordinates and editor colors: authoring its own font,
        // margins or background cannot resize, obscure, or strand these controls.
        p.clear([5, 4, 12]);
        p.text("Settings", 60, 38, 32, GOLD);
        p.text(
            "Appearance studio · edits affect appearance only",
            310,
            51,
            18,
            INK,
        );
        button(
            p,
            if self.reduced_motion {
                "Motion: reduced [R]"
            } else {
                "Motion: animated [R]"
            },
            80,
            108,
            230,
            self.reduced_motion,
        );
        button(
            p,
            self.display_mode.label(),
            330,
            108,
            310,
            self.display_mode != DisplayMode::Windowed,
        );
        button(p, "Back · Esc / F8", 1040, 108, 170, false);
        p.text(
            "Edits stay in this session. Save a preset and choose startup default to keep them.",
            60,
            78,
            14,
            MUTED,
        );
        let status = self.store.status();
        let top = [
            ("Undo", status.can_undo),
            ("Redo", status.can_redo),
            ("Original", status.comparison == "original"),
            ("Edited", status.comparison == "edited"),
            ("Preview", status.previewing),
            ("Commit", false),
            ("Discard", false),
            ("Reset all", false),
        ];
        for (i, (label, active)) in top.iter().enumerate() {
            button(p, label, 310 + i as i32 * 110, 158, 104, *active);
        }
        field(
            p,
            "Search settings",
            if self.focus == Some(Focus::Search) {
                &self.input
            } else {
                &self.query
            },
            310,
            208,
            880,
            self.focus == Some(Focus::Search),
        );
        button(p, "All categories", 60, 208, 220, self.category == 10);
        for (i, name) in CATEGORIES.iter().enumerate() {
            button(p, name, 60, 248 + i as i32 * 32, 220, i == self.category);
        }
        if self.category == 9 {
            self.paint_presets(p);
        } else {
            self.paint_controls(p);
        }
        p.text(
            if status.dirty {
                "Unsaved edits"
            } else {
                "Saved appearance"
            },
            60,
            596,
            14,
            if status.dirty { GOLD } else { MUTED },
        );
        p.text("[Tab] Category", 60, 628, 13, MUTED);
        p.text("[↑↓] Setting  [←→] Adjust", 60, 650, 13, MUTED);
        p.text("[Enter] Edit  [Ctrl+A] Replace", 60, 672, 12, MUTED);
        p.text("[Ctrl+Z / Shift+Z] Undo / redo", 60, 694, 12, MUTED);
        for (i, line) in crate::manual::wrap(&self.notice, 148)
            .iter()
            .take(2)
            .enumerate()
        {
            p.text(line, 60, 731 + i as i32 * 16, 13, [237, 175, 130]);
        }
        p.text("Native coverage: bundled fonts; other families fall back. Controls marked [stored only] are preserved for export.",60,766,12,MUTED);
        if let Some(confirm) = &self.confirm {
            p.rect(370, 275, 540, 190, [11, 13, 21]);
            p.border(370, 275, 540, 190, GOLD);
            p.text("Confirm", 400, 300, 24, GOLD);
            let text = match confirm {
                Confirmation::Delete(id) => format!(
                    "Delete {}? This cannot be undone.",
                    self.store
                        .presets()
                        .iter()
                        .find(|p| &p.id == id)
                        .map(|p| p.name.as_str())
                        .unwrap_or(id)
                ),
                Confirmation::Overwrite(path) => format!("Replace existing export {}?", path),
            };
            for (i, line) in crate::manual::wrap(&text, 50).iter().take(3).enumerate() {
                p.text(line, 400, 340 + i as i32 * 22, 15, INK);
            }
            button(p, "Confirm [Y]", 420, 420, 140, false);
            button(p, "Cancel [Esc]", 580, 420, 140, false);
        }
    }
    fn paint_controls(&self, p: &mut Painter) {
        let groups = self.groups();
        button(
            p,
            &format!(
                "<  {}  >",
                groups.get(self.group).copied().unwrap_or("All entries")
            ),
            310,
            256,
            880,
            false,
        );
        let list = self.filtered();
        if list.is_empty() {
            p.text("No settings match this search.", 330, 330, 18, MUTED);
        }
        for (row, i) in list
            .iter()
            .skip(self.offset)
            .take(self.page_rows())
            .enumerate()
        {
            let c = &controls()[*i];
            let y = 306 + row as i32 * 60;
            if self.selected == row + self.offset {
                p.rect(306, y - 3, 888, 57, [25, 27, 37]);
            }
            let group = c
                .group
                .as_deref()
                .map(|s| format!("{s} · "))
                .unwrap_or_default();
            p.text(
                &crate::render::truncate(
                    &format!(
                        "{group}{}{}",
                        c.label,
                        if partially_applied(&c.path) {
                            " [partly applied]"
                        } else if retained_only(&c.path) {
                            " [stored only]"
                        } else {
                            ""
                        }
                    ),
                    83,
                ),
                318,
                y,
                14,
                INK,
            );
            if fixed_player_glyph(&c.path) {
                p.text(FIXED_PLAYER_GLYPH, 325, y + 27, 14, MUTED);
                continue;
            }
            let editing = self.focus == Some(Focus::Value(*i));
            let value = if editing {
                self.input.clone()
            } else {
                display(self.store.working().value(&c.path))
            };
            p.rect(318, y + 22, 660, 28, [5, 4, 12]);
            p.border(
                318,
                y + 22,
                660,
                28,
                if editing { GOLD } else { [53, 55, 68] },
            );
            if c.kind == "color" {
                p.rect(325, y + 27, 18, 18, self.store.working().rgb(&c.path));
            }
            p.text(
                &crate::render::truncate(&value, 75),
                if c.kind == "color" { 352 } else { 325 },
                y + 27,
                14,
                INK,
            );
            button(p, "−", 1000, y + 20, 40, false);
            button(p, "+", 1050, y + 20, 40, false);
            button(p, "Reset", 1110, y + 20, 78, false);
        }
        if self.category == 5 {
            p.rect(310, 606, 880, 62, [5, 4, 12]);
            for (row, line) in [
                "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
                "abcdefghijklmnopqrstuvwxyz",
                "0123456789 @ # . ~ + > < ! ? %",
            ]
            .iter()
            .enumerate()
            {
                for (col, glyph) in line.chars().enumerate() {
                    p.map_identity(
                        &glyph.to_string(),
                        320 + col as i32 * 28,
                        607 + row as i32 * 20,
                        16,
                        18,
                        [255, 242, 211],
                    );
                }
            }
        }
        p.text(
            &format!(
                "{}–{} of {} settings · scroll for more",
                if list.is_empty() { 0 } else { self.offset + 1 },
                (self.offset + self.page_rows()).min(list.len()),
                list.len()
            ),
            315,
            682,
            13,
            MUTED,
        );
        if let Some(c) = self.selected_control() {
            let hint = if fixed_player_glyph(&c.path) {
                "Player glyph is fixed to @."
            } else if partially_applied(&c.path) {
                "Applied in supported views; remaining source consumers and modal reflow are incomplete."
            } else if retained_only(&c.path) {
                "Stored/exportable: native rendering for this setting is not implemented yet."
            } else {
                c.description.as_deref().unwrap_or(match c.kind.as_str() {
                    "glyph" => "One printable ASCII character, or this entry's original built-in glyph.",
                    "color" => "Six hexadecimal digits, such as #8ca0a8.",
                    "text" => "Font family list; bundled IBM Plex Mono, Inconsolata and Brogue CE ASCII are available.",
                    _ => "Enter an exact value, or use the minus and plus controls.",
                })
            };
            p.text(&crate::render::truncate(hint, 106), 315, 707, 12, MUTED);
        }
    }
    fn paint_presets(&self, p: &mut Painter) {
        let current = self.store.presets().get(self.preset);
        let selected = current
            .map(|p| p.name.as_str())
            .unwrap_or("No saved presets");
        button(p, &format!("<  {selected}  >"), 310, 265, 880, false);
        field(
            p,
            "Name for save / duplicate / rename",
            if self.focus == Some(Focus::PresetName) {
                &self.input
            } else {
                &self.preset_name
            },
            310,
            325,
            880,
            self.focus == Some(Focus::PresetName),
        );
        for (i, label) in [
            "Save new",
            "Save changes",
            "Load",
            "Duplicate",
            "Rename",
            "Delete",
        ]
        .iter()
        .enumerate()
        {
            button(p, label, 310 + i as i32 * 145, 370, 138, false);
        }
        for (i, label) in [
            "Set startup default",
            "Project startup default",
            "Compare saved preset",
        ]
        .iter()
        .enumerate()
        {
            button(p, label, 310 + i as i32 * 265, 410, 255, false);
        }
        field(
            p,
            "JSON file path · drag and drop also imports",
            if self.focus == Some(Focus::FilePath) {
                &self.input
            } else {
                &self.file_path
            },
            310,
            480,
            880,
            self.focus == Some(Focus::FilePath),
        );
        for (i, label) in [
            "Export JSON file",
            "Import JSON file",
            "Copy edited JSON",
            "Paste clipboard",
        ]
        .iter()
        .enumerate()
        {
            button(p, label, 310 + i as i32 * 215, 520, 207, false);
        }
        field(
            p,
            "Pasted appearance JSON",
            if self.focus == Some(Focus::Json) {
                &self.input
            } else {
                &self.json
            },
            310,
            585,
            880,
            self.focus == Some(Focus::Json),
        );
        button(p, "Import pasted JSON", 310, 628, 240, false);
        let default = self
            .store
            .status()
            .default_preset_id
            .and_then(|id| {
                self.store
                    .presets()
                    .iter()
                    .find(|p| p.id == id)
                    .map(|p| p.name.clone())
            })
            .unwrap_or("Project defaults".into());
        p.text(
            &crate::render::truncate(&format!("Startup: {default}"), 104),
            315,
            682,
            13,
            MUTED,
        );
        p.text(
            "Export uses committed edits even while comparing or previewing. Import is undoable.",
            315,
            707,
            12,
            MUTED,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn view() -> SettingsView {
        let mut v = SettingsView::load(Path::new(
            "/nonexistent-corpse-appearance-tests/settings.json",
        ));
        v.store = Store::memory();
        v.open();
        v
    }
    #[test]
    fn player_glyph_setting_is_read_only_for_mouse_keyboard_and_stale_focus() {
        let mut v = view();
        v.store.set("entities.objects.player.glyph", Value::from("P")).unwrap();
        v.change_category(10);
        v.query = "entities.objects.player.glyph".into();
        let list = v.filtered();
        assert_eq!(list.len(), 1);
        let before = v.store.export_json();
        for _ in 0..2 {
            for x in [500, 1020, 1070, 1140] {
                assert_eq!(v.click(x, 340), Action::None);
                assert!(v.focus.is_none());
                assert_eq!(v.notice, FIXED_PLAYER_GLYPH);
                v.text_input("X");
            }
            for key in [Keycode::Return, Keycode::Left, Keycode::Right] {
                assert_eq!(v.key(key, Mod::NOMOD), Action::None);
                assert!(v.focus.is_none());
            }
        }
        v.focus = Some(Focus::Value(list[0]));
        v.input = "X".into();
        assert_eq!(v.commit_input(), Action::None);
        assert!(v.focus.is_none());
        assert_eq!(v.store.export_json(), before);
        let imported = Profile::parse(&before).unwrap();
        assert_eq!(imported.text("entities.objects.player.glyph"), "P");
        if let Ok(directory) = std::env::var("CORPSE_PLAYER_GLYPH_REVIEW") {
            let mut p = Painter::new();
            v.paint(&mut p);
            std::fs::create_dir_all(&directory).unwrap();
            let file = std::fs::File::create(format!("{directory}/player-glyph-fixed-setting-1280x800.png")).unwrap();
            let mut encoder = png::Encoder::new(file, p.width as u32, p.height as u32);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
        }
    }
    #[test]
    fn player_glyph_lock_preserves_other_player_style_and_map_font_controls() {
        let mut v = view();
        for (path, value) in [
            ("entities.objects.player.color", "#123456"),
            ("entities.objects.player.background", "#654321"),
            ("entities.objects.player.emphasis", "1.5"),
            ("typography.map.fontFamily", "monospace"),
        ] {
            v.change_category(10);
            v.query = path.into();
            assert_eq!(v.filtered().len(), 1, "{path}");
            v.select_control(0);
            assert!(matches!(v.focus, Some(Focus::Value(_))), "{path}");
            v.input = value.into();
            assert_eq!(v.commit_input(), Action::Changed, "{path}: {}", v.notice);
            assert_eq!(display(v.store.working().value(path)), value);
        }
    }
    #[test]
    fn exact_edit_cancel_undo_and_repeat_do_not_move_panel() {
        let mut v = view();
        v.click(500, 340);
        assert!(matches!(v.focus, Some(Focus::Value(_))));
        v.key(Keycode::A, Mod::LCTRLMOD);
        v.text_input("1.7");
        assert_eq!(v.key(Keycode::Return, Mod::NOMOD), Action::Changed);
        assert_eq!(v.profile().number("global.exposure"), 1.7);
        v.click(500, 340);
        v.key(Keycode::A, Mod::LCTRLMOD);
        v.text_input("3.5");
        v.key(Keycode::Escape, Mod::NOMOD);
        assert_eq!(v.profile().number("global.exposure"), 1.7);
        v.key(Keycode::Z, Mod::LCTRLMOD);
        assert_eq!(v.profile().number("global.exposure"), 1.);
        v.key(Keycode::Z, Mod::LCTRLMOD | Mod::LSHIFTMOD);
        assert_eq!(v.profile().number("global.exposure"), 1.7);
        v.click(1120, 340);
        assert_eq!(v.profile().number("global.exposure"), 1.);
        assert_eq!(v.category, 0);
        assert_eq!(v.offset, 0);
    }
    #[test]
    fn invalid_value_stays_editable_and_does_not_change_profile() {
        let mut v = view();
        v.click(500, 340);
        v.key(Keycode::A, Mod::LCTRLMOD);
        v.text_input("500");
        assert_eq!(v.key(Keycode::Return, Mod::NOMOD), Action::None);
        assert_eq!(v.profile().number("global.exposure"), 1.);
        assert!(v.focus.is_some());
        assert!(!v.notice.is_empty());
    }
    #[test]
    fn search_and_group_navigation_remain_bounded() {
        let mut v = view();
        v.change_category(1);
        assert!(!v.filtered().is_empty());
        v.click(400, 220);
        v.text_input("xyz absent");
        assert!(v.filtered().is_empty());
        v.scroll(-100_000);
        assert_eq!(v.offset, 0);
        v.key(Keycode::A, Mod::LCTRLMOD);
        v.text_input("color");
        assert!(!v.filtered().is_empty());
        v.key(Keycode::Escape, Mod::NOMOD);
        for _ in 0..500 {
            v.click(400, 270);
        }
        assert!(v.group < v.groups().len());
        v.scroll(-100_000);
        assert!(v.offset <= v.filtered().len().saturating_sub(6));
    }
    #[test]
    fn hide_discards_preview_and_exits_comparison() {
        let mut v = view();
        v.store.set("global.exposure", Value::from(1.8)).unwrap();
        v.click(755, 170);
        v.store.set("global.exposure", Value::from(2.7)).unwrap();
        assert!(v.store.status().previewing);
        assert_eq!(v.key(Keycode::F8, Mod::NOMOD), Action::Close);
        assert_eq!(v.profile().number("global.exposure"), 1.8);
        assert!(!v.store.status().previewing);
        v.open();
        assert_eq!(v.profile().number("global.exposure"), 1.8);
    }
    #[test]
    fn deleting_preset_requires_confirmation_and_cancel_is_safe() {
        let mut v = view();
        v.preset_name = "Test preset".into();
        v.preset_action(0);
        assert_eq!(v.store.presets().len(), 1);
        v.preset_action(5);
        v.key(Keycode::Escape, Mod::NOMOD);
        assert_eq!(v.store.presets().len(), 1);
        v.preset_action(5);
        v.key(Keycode::Y, Mod::NOMOD);
        assert!(v.store.presets().is_empty());
    }
    #[test]
    fn export_copy_and_pasted_import_use_edited_snapshot() {
        let mut v = view();
        v.change_category(9);
        v.store.set("global.exposure", Value::from(1.6)).unwrap();
        v.store.begin_preview();
        v.store.set("global.exposure", Value::from(2.7)).unwrap();
        let Action::Copy(exported) = v.click(800, 535) else {
            panic!("copy action")
        };
        assert_eq!(
            Profile::parse(&exported).unwrap().number("global.exposure"),
            1.6
        );
        v.click(600, 600);
        v.key(Keycode::A, Mod::LCTRLMOD);
        v.text_input("{\"global\":{\"exposure\":2.1}}");
        v.click(400, 640);
        assert_eq!(v.profile().number("global.exposure"), 2.1);
        v.store.undo();
        assert_eq!(v.profile().number("global.exposure"), 1.6);
    }
    #[test]
    fn comparing_original_keeps_editor_values_at_edited_snapshot() {
        let mut v = view();
        v.store.set("global.exposure", Value::from(1.8)).unwrap();
        v.store.compare("original").unwrap();
        v.click(500, 340);
        assert_eq!(v.input, "1.8");
        assert_eq!(v.profile().number("global.exposure"), 1.);
    }
    #[test]
    fn fixed_editor_paints_all_pages_and_extreme_settings_without_moving_controls() {
        let mut v = view();
        let mut painter = Painter::new();
        let output =
            std::env::var_os("CORPSE_APPEARANCE_SCREENSHOTS").map(std::path::PathBuf::from);
        if let Some(dir) = &output {
            std::fs::create_dir_all(dir).unwrap();
        }
        for (category, name) in [
            (0, "global"),
            (1, "terrain"),
            (4, "entities"),
            (5, "fonts"),
            (9, "presets"),
        ] {
            v.change_category(category);
            v.paint(&mut painter);
            assert_eq!(
                &painter.pixels[(248 * crate::render::WIDTH + 60) * 4..][..3],
                &[53, 55, 68]
            );
            if let Some(dir) = &output {
                let file =
                    std::fs::File::create(dir.join(format!("appearance-{name}.png"))).unwrap();
                let mut encoder = png::Encoder::new(
                    file,
                    crate::render::WIDTH as u32,
                    crate::render::HEIGHT as u32,
                );
                encoder.set_color(png::ColorType::Rgba);
                encoder.set_depth(png::BitDepth::Eight);
                encoder
                    .write_header()
                    .unwrap()
                    .write_image_data(&painter.pixels)
                    .unwrap();
            }
        }
        v.store
            .set("typography.primary.fontSize", Value::from(40))
            .unwrap();
        v.store
            .set("ui.layout.panelPadding", Value::from(24))
            .unwrap();
        v.store
            .set("ui.layout.rightSidebarWidth", Value::from(600))
            .unwrap();
        painter.profile = v.profile().clone();
        v.paint(&mut painter);
        assert_eq!(
            &painter.pixels[(248 * crate::render::WIDTH + 60) * 4..][..3],
            &[53, 55, 68]
        );
        assert_eq!(v.click(1100, 120), Action::Close);
    }
    #[test]
    fn borderless_display_control_uses_full_label_hit_area_and_persists() {
        let mut v = view();
        assert_eq!(v.click(610, 120), Action::Fullscreen);
        assert_eq!(v.key(Keycode::F, Mod::NOMOD), Action::Fullscreen);
        for mode in [
            DisplayMode::Borderless,
            DisplayMode::Fullscreen,
            DisplayMode::Windowed,
        ] {
            assert_eq!(v.display_mode.next(), mode);
            v.sync_display_mode(mode).unwrap();
            assert_eq!(v.display_mode, mode);
            assert_eq!(v.store.display_mode, mode);
        }
        v.observe_display_mode(DisplayMode::Fullscreen);
        assert_eq!(v.display_mode, DisplayMode::Fullscreen);
        assert_eq!(v.store.display_mode, DisplayMode::Windowed);
    }
    #[test]
    fn failed_display_preference_save_does_not_claim_success() {
        let mut v = SettingsView::load(Path::new(
            "/nonexistent-corpse-appearance-tests/settings.json",
        ));
        assert!(v.sync_display_mode(DisplayMode::Fullscreen).is_err());
        assert_eq!(v.display_mode, DisplayMode::Windowed);
        assert_eq!(v.store.display_mode, DisplayMode::Windowed);
        // If the OS rollback fails, the displayed mode must reflect reality,
        // while the saved preference remains unchanged.
        v.observe_display_mode(DisplayMode::Fullscreen);
        assert_eq!(v.display_mode, DisplayMode::Fullscreen);
        assert_eq!(v.store.display_mode, DisplayMode::Windowed);
    }
}
