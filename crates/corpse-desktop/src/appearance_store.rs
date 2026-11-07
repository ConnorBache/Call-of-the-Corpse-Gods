//! Transactional appearance library and undo/preview session, translated from v48.
//! Working edits are session-local; only explicitly saved presets/startup choices persist.
use crate::appearance_profile::{
    utf16_len, Profile, MAX_HISTORY, MAX_HISTORY_CHARACTERS, MAX_JSON_CHARACTERS, MAX_PRESETS,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
/// Native display preference. Legacy `fullscreen: true` means borderless.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DisplayMode {
    #[default]
    Windowed,
    Borderless,
    Fullscreen,
}
impl DisplayMode {
    pub fn next(self) -> Self {
        match self {
            Self::Windowed => Self::Borderless,
            Self::Borderless => Self::Fullscreen,
            Self::Fullscreen => Self::Windowed,
        }
    }
    pub fn to_sdl(self) -> sdl2::video::FullscreenType {
        match self {
            Self::Windowed => sdl2::video::FullscreenType::Off,
            Self::Borderless => sdl2::video::FullscreenType::Desktop,
            Self::Fullscreen => sdl2::video::FullscreenType::True,
        }
    }
    pub fn from_sdl(value: sdl2::video::FullscreenType) -> Self {
        match value {
            sdl2::video::FullscreenType::Off => Self::Windowed,
            sdl2::video::FullscreenType::Desktop => Self::Borderless,
            sdl2::video::FullscreenType::True => Self::Fullscreen,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Windowed => "Windowed [F]",
            Self::Borderless => "Borderless fullscreen [F]",
            Self::Fullscreen => "Fullscreen [F]",
        }
    }
}
impl From<bool> for DisplayMode {
    fn from(fullscreen: bool) -> Self {
        if fullscreen {
            Self::Borderless
        } else {
            Self::Windowed
        }
    }
}
/// Requested SDL window size / exclusive display dimensions, independent of UI layout.
/// A missing stored resolution selects automatic native drawable pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, serde::Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}
impl Resolution {
    pub fn valid(self) -> bool {
        (640..=8192).contains(&self.width)
            && (480..=8192).contains(&self.height)
            && u64::from(self.width) * u64::from(self.height) <= 33_554_432
    }
}
#[derive(Clone, Serialize)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub appearance: Profile,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Library {
    version: u32,
    next_id: u64,
    default_preset_id: Option<String>,
    presets: Vec<Preset>,
}
impl Default for Library {
    fn default() -> Self {
        Self {
            version: 1,
            next_id: 1,
            default_preset_id: None,
            presets: Vec::new(),
        }
    }
}
#[derive(Clone)]
struct History {
    profile: Profile,
    characters: usize,
}
#[derive(Clone, Debug)]
pub struct Status {
    pub dirty: bool,
    pub previewing: bool,
    pub can_undo: bool,
    pub can_redo: bool,
    pub comparison: String,
    pub saved_preset_id: Option<String>,
    pub default_preset_id: Option<String>,
}
pub struct Store {
    path: Option<PathBuf>,
    library: Library,
    edited: Profile,
    original: Profile,
    preview: Option<Profile>,
    comparison: String,
    saved_preset_id: Option<String>,
    past: VecDeque<History>,
    future: VecDeque<History>,
    history_characters: usize,
    gesture: Option<Profile>,
    pub editor_open: bool,
    pub reduced_motion: bool,
    pub fullscreen: bool,
    pub display_mode: DisplayMode,
    pub resolution: Option<Resolution>,
    pub error: String,
}
fn same(a: &Profile, b: &Profile) -> bool {
    a.data().to_string() == b.data().to_string()
}
fn name_of(name: &str) -> Result<String, String> {
    let clean = name.trim();
    if clean.is_empty() || utf16_len(clean) > 80 {
        Err("Preset name must contain 1–80 characters.".into())
    } else {
        Ok(clean.into())
    }
}
/// O(1), compile-time app identity; direct executable launch needs no wrapper.
const fn app_directory_name()->&'static str {if cfg!(feature="append-recording-trial"){"corpse-gods-append-trial"}else{"corpse-gods"}}
/// O(path bytes), pure path construction allows tests without process environment races.
fn default_path_from_base(base:Option<PathBuf>)->PathBuf {base.map(|p|p.join(app_directory_name()).join("appearance.json")).unwrap_or_else(||PathBuf::from(if cfg!(feature="append-recording-trial"){"corpse-gods-append-trial/appearance.json"}else{"native-appearance.json"}))}
/// Per-user settings location; independent of the launch working directory.
/// O(environment/path bytes), no filesystem writes.
/// This function computes a path only. `prepare_default_directory` creates the
/// app-owned parent when the caller chooses the default instead of --appearance.
pub fn default_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA").map(PathBuf::from).or_else(|| {
        std::env::var_os("USERPROFILE").map(|p| PathBuf::from(p).join("AppData/Roaming"))
    });
    #[cfg(target_os = "macos")]
    let base =
        std::env::var_os("HOME").map(|p| PathBuf::from(p).join("Library/Application Support"));
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")));
    default_path_from_base(base)
}
pub fn prepare_default_directory(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| {
            format!(
                "Cannot create appearance settings directory {}: {e}",
                parent.display()
            )
        })?;
    }
    Ok(())
}
/// Replace in the same directory only after all bytes have been synced. A failed
/// write/rename leaves the previous file and in-memory library unchanged.
pub fn atomic_write(path: &Path, text: &str) -> Result<(), String> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .ok_or("Appearance path needs a file name")?
        .to_string_lossy();
    let temp = parent.join(format!(
        ".{name}.{}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        file.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        std::fs::rename(&temp, path).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        if let Ok(dir) = std::fs::File::open(parent) {
            let _ = dir.sync_all();
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}
pub fn read_limited(path: &Path) -> Result<String, String> {
    let file =
        std::fs::File::open(path).map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    // UTF-16 source-character budget can occupy at most 4x as many UTF-8 bytes.
    let mut raw = String::new();
    use std::io::Read;
    file.take((MAX_JSON_CHARACTERS * 4 + 1) as u64)
        .read_to_string(&mut raw)
        .map_err(|e| e.to_string())?;
    if raw.len() > MAX_JSON_CHARACTERS * 4 || utf16_len(&raw) > MAX_JSON_CHARACTERS {
        return Err("Appearance JSON exceeds 2 MB.".into());
    }
    Ok(raw)
}
impl Store {
    pub fn load(path: &Path) -> Self {
        let mut store = Self::memory();
        store.path = Some(path.into());
        if path.exists() {
            if let Err(error) = store.read(path) {
                store.error = format!(
                    "Could not load appearance presets: {error}. Original file left untouched."
                );
            }
        }
        store
    }
    pub fn memory() -> Self {
        Self {
            path: None,
            library: Library::default(),
            edited: Profile::default(),
            original: Profile::default(),
            preview: None,
            comparison: "edited".into(),
            saved_preset_id: None,
            past: VecDeque::new(),
            future: VecDeque::new(),
            history_characters: 0,
            gesture: None,
            editor_open: false,
            reduced_motion: false,
            fullscreen: false,
            display_mode: DisplayMode::Windowed,
            resolution: None,
            error: String::new(),
        }
    }
    fn read(&mut self, path: &Path) -> Result<(), String> {
        let raw = read_limited(path)?;
        let v: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        // Native envelope; an unwrapped browser preset library also imports unchanged.
        let envelope = v.get("library").is_some();
        if envelope && v["version"].as_u64() != Some(1) {
            return Err("Unsupported native appearance settings version".into());
        }
        let lib = if envelope { &v["library"] } else { &v };
        let library = Self::parse_library(lib)?;
        let reduced = if envelope {
            v["reducedMotion"]
                .as_bool()
                .ok_or("Invalid reduced-motion setting")?
        } else {
            false
        };
        let display_mode = if envelope {
            if let Some(mode) = v.get("displayMode") {
                serde_json::from_value::<DisplayMode>(mode.clone())
                    .map_err(|_| "Invalid display mode setting")?
            } else {
                DisplayMode::from(
                    v["fullscreen"]
                        .as_bool()
                        .ok_or("Invalid fullscreen setting")?,
                )
            }
        } else {
            DisplayMode::Windowed
        };
        let resolution = if envelope {
            match v.get("resolution").filter(|value| !value.is_null()) {
                Some(value) => {
                    let resolution: Resolution = serde_json::from_value(value.clone())
                        .map_err(|_| "Invalid display resolution")?;
                    if !resolution.valid() {
                        return Err("Invalid display resolution".into());
                    }
                    Some(resolution)
                }
                None => None,
            }
        } else {
            None
        };
        self.resolution = resolution;
        self.reduced_motion = reduced;
        self.fullscreen = display_mode != DisplayMode::Windowed;
        self.display_mode = display_mode;
        self.library = library;
        if let Some(p) = self
            .library
            .presets
            .iter()
            .find(|p| Some(&p.id) == self.library.default_preset_id.as_ref())
        {
            self.edited = p.appearance.clone();
            self.saved_preset_id = Some(p.id.clone());
        }
        self.original = self.edited.clone();
        Ok(())
    }
    fn parse_library(v: &Value) -> Result<Library, String> {
        if v["version"].as_f64() != Some(1.) {
            return Err("Unsupported or damaged appearance preset library".into());
        }
        let entries = v["presets"]
            .as_array()
            .filter(|a| a.len() <= MAX_PRESETS)
            .ok_or("Invalid preset count")?;
        let mut presets = Vec::<Preset>::new();
        let mut next_id = 1;
        for entry in entries {
            let id = entry["id"].as_str().ok_or("Invalid preset ID")?;
            let suffix = id
                .strip_prefix("preset-")
                .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
                .ok_or("Invalid preset ID")?;
            let n: u64 = suffix.parse().map_err(|_| "Invalid preset ID")?;
            if n >= 9_007_199_254_740_991 || presets.iter().any(|p| p.id == id) {
                return Err("Invalid or duplicate preset ID".into());
            }
            let name = name_of(entry["name"].as_str().ok_or("Missing preset name")?)?;
            if presets
                .iter()
                .any(|p| p.name.to_lowercase() == name.to_lowercase())
            {
                return Err("Duplicate appearance preset names".into());
            }
            presets.push(Preset {
                id: id.into(),
                name,
                appearance: Profile::from_value(entry["appearance"].clone())?,
            });
            next_id = next_id.max(n + 1);
        }
        if v.get("defaultPresetId").is_none() {
            return Err("Default appearance preset is missing".into());
        }
        let default = if v["defaultPresetId"].is_null() {
            None
        } else {
            Some(
                v["defaultPresetId"]
                    .as_str()
                    .ok_or("Invalid startup preset")?
                    .to_string(),
            )
        };
        if default
            .as_ref()
            .is_some_and(|id| !presets.iter().any(|p| &p.id == id))
        {
            return Err("Default appearance preset is missing".into());
        }
        Ok(Library {
            version: 1,
            next_id,
            default_preset_id: default,
            presets,
        })
    }
    pub fn profile(&self) -> &Profile {
        if self.comparison == "original" {
            return &self.original;
        }
        if let Some(p) = self
            .library
            .presets
            .iter()
            .find(|p| p.id == self.comparison)
        {
            return &p.appearance;
        }
        self.preview.as_ref().unwrap_or(&self.edited)
    }
    pub fn edited(&self) -> &Profile {
        &self.edited
    }
    pub fn working(&self) -> &Profile {
        if self.preview.is_some() {
            self.profile()
        } else {
            &self.edited
        }
    }
    pub fn presets(&self) -> &[Preset] {
        &self.library.presets
    }
    pub fn status(&self) -> Status {
        let saved = self
            .library
            .presets
            .iter()
            .find(|p| Some(&p.id) == self.saved_preset_id.as_ref())
            .map(|p| &p.appearance)
            .unwrap_or(Profile::defaults());
        Status {
            dirty: !same(&self.edited, saved),
            previewing: self.preview.is_some(),
            can_undo: !self.past.is_empty()
                || self
                    .gesture
                    .as_ref()
                    .is_some_and(|g| !same(g, &self.edited)),
            can_redo: !self.future.is_empty(),
            comparison: self.comparison.clone(),
            saved_preset_id: self.saved_preset_id.clone(),
            default_preset_id: self.library.default_preset_id.clone(),
        }
    }
    pub fn open(&mut self) {
        if !self.editor_open {
            self.original = self.profile().clone();
            self.editor_open = true;
            self.comparison = "edited".into();
        }
    }
    pub fn close(&mut self) {
        self.end_gesture();
        self.editor_open = false;
        self.preview = None;
        self.comparison = "edited".into();
    }
    pub fn begin_gesture(&mut self) {
        if self.gesture.is_none() {
            self.gesture = Some(self.edited.clone());
        }
    }
    pub fn end_gesture(&mut self) {
        if let Some(before) = self.gesture.take() {
            if !same(&before, &self.edited) {
                self.push(before, false);
            }
        }
    }
    fn push(&mut self, profile: Profile, future: bool) {
        let characters = profile.characters();
        self.history_characters += characters;
        let target = if future {
            &mut self.future
        } else {
            &mut self.past
        };
        target.push_back(History {
            profile,
            characters,
        });
        while self.history_characters > MAX_HISTORY_CHARACTERS
            || self.past.len() + self.future.len() > MAX_HISTORY
        {
            let distant = if self.past.len() >= self.future.len() {
                &mut self.past
            } else {
                &mut self.future
            };
            if let Some(removed) = distant.pop_front() {
                self.history_characters -= removed.characters;
            } else {
                break;
            }
        }
    }
    fn edit(&mut self, next: Profile) {
        if !same(&next, &self.edited) {
            if self.gesture.is_none() {
                self.push(self.edited.clone(), false);
            }
            self.history_characters -= self.future.iter().map(|h| h.characters).sum::<usize>();
            self.future.clear();
            self.edited = next;
        }
        self.preview = None;
        self.comparison = "edited".into();
    }
    pub fn set(&mut self, path: &str, value: Value) -> Result<(), String> {
        if self.preview.is_some() {
            let next = self.profile().set(path, value)?;
            self.preview(next);
        } else {
            self.edit(self.edited.set(path, value)?);
        }
        Ok(())
    }
    pub fn reset_property(&mut self, path: &str) -> Result<(), String> {
        if self.preview.is_some() {
            self.preview(self.profile().reset(path)?);
        } else {
            self.edit(self.edited.reset(path)?);
        }
        Ok(())
    }
    pub fn reset_group(&mut self, path: &str) -> Result<(), String> {
        let next = if self.preview.is_some() {
            self.profile()
        } else {
            &self.edited
        }
        .reset_group(path)?;
        self.end_gesture();
        if self.preview.is_some() {
            self.preview(next);
        } else {
            self.edit(next);
        }
        Ok(())
    }
    pub fn reset_all(&mut self) {
        self.end_gesture();
        if self.preview.is_some() {
            self.preview(Profile::default());
        } else {
            self.edit(Profile::default());
        }
    }
    pub fn undo(&mut self) {
        self.end_gesture();
        if let Some(h) = self.past.pop_back() {
            self.history_characters -= h.characters;
            self.push(self.edited.clone(), true);
            self.edited = h.profile;
            self.preview = None;
            self.comparison = "edited".into();
        }
    }
    pub fn redo(&mut self) {
        self.end_gesture();
        if let Some(h) = self.future.pop_back() {
            self.history_characters -= h.characters;
            self.push(self.edited.clone(), false);
            self.edited = h.profile;
            self.preview = None;
            self.comparison = "edited".into();
        }
    }
    pub fn preview(&mut self, value: Profile) {
        self.end_gesture();
        self.preview = Some(value);
        self.comparison = "edited".into();
    }
    pub fn begin_preview(&mut self) {
        self.preview(self.edited.clone());
    }
    pub fn commit_preview(&mut self) {
        if let Some(value) = self.preview.take() {
            self.edit(value);
        }
    }
    pub fn cancel_preview(&mut self) {
        self.preview = None;
    }
    pub fn compare(&mut self, id: &str) -> Result<(), String> {
        if id != "original" && id != "edited" {
            self.require(id)?;
        }
        self.end_gesture();
        self.comparison = id.into();
        Ok(())
    }
    fn require(&self, id: &str) -> Result<&Preset, String> {
        self.library
            .presets
            .iter()
            .find(|p| p.id == id)
            .ok_or("Appearance preset no longer exists.".into())
    }
    fn unique(&self, name: &str, except: Option<&str>) -> Result<String, String> {
        let clean = name_of(name)?;
        if self
            .library
            .presets
            .iter()
            .any(|p| Some(p.id.as_str()) != except && p.name.to_lowercase() == clean.to_lowercase())
        {
            return Err("A preset with that name already exists.".into());
        }
        Ok(clean)
    }
    fn persist(
        &mut self,
        next: Library,
        reduced: bool,
        display_mode: DisplayMode,
    ) -> Result<(), String> {
        self.persist_display(next, reduced, display_mode, self.resolution)
    }
    fn persist_display(
        &mut self,
        next: Library,
        reduced: bool,
        display_mode: DisplayMode,
        resolution: Option<Resolution>,
    ) -> Result<(), String> {
        if resolution.is_some_and(|value| !value.valid()) {
            return Err("Invalid display resolution".into());
        }
        let lib = serde_json::to_value(&next).map_err(|e| e.to_string())?;
        if utf16_len(&lib.to_string()) > MAX_JSON_CHARACTERS {
            return Err("Appearance library is full. Export and remove older presets.".into());
        }
        let text =
            json!({"version":1,"reducedMotion":reduced,"fullscreen":display_mode != DisplayMode::Windowed,"displayMode":display_mode,"resolution":resolution,"library":lib})
                .to_string();
        if utf16_len(&text) > MAX_JSON_CHARACTERS {
            return Err("Appearance library is full.".into());
        }
        if let Some(path) = &self.path {
            if let Err(error) = atomic_write(path, &text) {
                self.error=format!("Could not save appearance settings: {error}. Your working appearance is still available to export.");
                return Err(self.error.clone());
            }
        }
        self.library = next;
        self.reduced_motion = reduced;
        self.fullscreen = display_mode != DisplayMode::Windowed;
        self.display_mode = display_mode;
        self.resolution = resolution;
        self.error.clear();
        Ok(())
    }
    /// One atomic disk transaction; memory changes only after the write succeeds.
    pub fn set_display_preferences(
        &mut self,
        mode: DisplayMode,
        resolution: Option<Resolution>,
    ) -> Result<(), String> {
        self.persist_display(self.library.clone(), self.reduced_motion, mode, resolution)
    }
    pub fn preferences(&mut self, reduced: bool, fullscreen: bool) -> Result<(), String> {
        self.display_preferences(reduced, fullscreen.into())
    }
    pub fn display_preferences(&mut self, reduced: bool, mode: DisplayMode) -> Result<(), String> {
        self.persist(self.library.clone(), reduced, mode)
    }
    pub fn save_preset(&mut self, name: &str) -> Result<String, String> {
        self.end_gesture();
        if self.library.presets.len() >= MAX_PRESETS {
            return Err("At most 50 appearance presets can be stored.".into());
        }
        let clean = self.unique(name, None)?;
        let id = format!("preset-{}", self.library.next_id);
        let appearance = self.edited.named(&clean);
        let mut next = self.library.clone();
        next.next_id += 1;
        next.presets.push(Preset {
            id: id.clone(),
            name: clean,
            appearance: appearance.clone(),
        });
        self.persist(next, self.reduced_motion, self.display_mode)?;
        self.edited = appearance;
        self.saved_preset_id = Some(id.clone());
        Ok(id)
    }
    pub fn update_preset(&mut self, id: &str) -> Result<(), String> {
        let name = self.require(id)?.name.clone();
        self.end_gesture();
        let appearance = self.edited.named(&name);
        let mut next = self.library.clone();
        next.presets
            .iter_mut()
            .find(|p| p.id == id)
            .unwrap()
            .appearance = appearance.clone();
        self.persist(next, self.reduced_motion, self.display_mode)?;
        self.edited = appearance;
        self.saved_preset_id = Some(id.into());
        Ok(())
    }
    pub fn load_preset(&mut self, id: &str) -> Result<(), String> {
        let appearance = self.require(id)?.appearance.clone();
        self.end_gesture();
        self.saved_preset_id = Some(id.into());
        self.edit(appearance);
        Ok(())
    }
    pub fn duplicate_preset(&mut self, id: &str, name: &str) -> Result<String, String> {
        if self.library.presets.len() >= MAX_PRESETS {
            return Err("At most 50 appearance presets can be stored.".into());
        }
        let source = self.require(id)?.appearance.clone();
        let clean = self.unique(name, None)?;
        let id = format!("preset-{}", self.library.next_id);
        let mut next = self.library.clone();
        next.next_id += 1;
        next.presets.push(Preset {
            id: id.clone(),
            name: clean.clone(),
            appearance: source.named(&clean),
        });
        self.persist(next, self.reduced_motion, self.display_mode)?;
        Ok(id)
    }
    pub fn rename_preset(&mut self, id: &str, name: &str) -> Result<(), String> {
        self.require(id)?;
        let clean = self.unique(name, Some(id))?;
        let mut next = self.library.clone();
        let p = next.presets.iter_mut().find(|p| p.id == id).unwrap();
        p.name = clean.clone();
        p.appearance = p.appearance.named(&clean);
        self.persist(next, self.reduced_motion, self.display_mode)?;
        if self.saved_preset_id.as_deref() == Some(id) {
            self.edited = self.edited.named(&clean);
        }
        Ok(())
    }
    pub fn delete_preset(&mut self, id: &str, confirmed: bool) -> Result<(), String> {
        if !confirmed {
            return Err("Confirm deletion before removing an appearance preset.".into());
        }
        self.require(id)?;
        let mut next = self.library.clone();
        next.presets.retain(|p| p.id != id);
        if next.default_preset_id.as_deref() == Some(id) {
            next.default_preset_id = None;
        }
        self.persist(next, self.reduced_motion, self.display_mode)?;
        if self.saved_preset_id.as_deref() == Some(id) {
            self.saved_preset_id = None;
        }
        if self.comparison == id {
            self.comparison = "edited".into();
        }
        Ok(())
    }
    pub fn set_default(&mut self, id: Option<&str>) -> Result<(), String> {
        if let Some(id) = id {
            self.require(id)?;
        }
        let mut next = self.library.clone();
        next.default_preset_id = id.map(str::to_owned);
        self.persist(next, self.reduced_motion, self.display_mode)
    }
    pub fn import_json(&mut self, text: &str) -> Result<(), String> {
        let next = Profile::parse(text)?;
        self.end_gesture();
        self.edit(next);
        Ok(())
    }
    pub fn export_json(&self) -> String {
        self.edited.json()
    }
    pub fn import_file(&mut self, path: &Path) -> Result<(), String> {
        self.import_json(&read_limited(path)?)
    }
    pub fn export_file(&self, path: &Path) -> Result<(), String> {
        atomic_write(path, &self.export_json())
    }
}

#[cfg(test)]
mod display_mode_tests {
    use super::*;
    fn path(name: &str) -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        std::env::temp_dir().join(format!(
            "corpse-display-{name}-{}-{}.json",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }
    fn envelope() -> Value {
        json!({"version":1,"reducedMotion":true,"fullscreen":true,"library":{"version":1,"nextId":1,"defaultPresetId":null,"presets":[]}})
    }
    #[test]
    fn resolution_bounds_limit_framebuffer_allocation() {
        assert!(Resolution {
            width: 8192,
            height: 4096
        }
        .valid());
        assert!(!Resolution {
            width: 8192,
            height: 8192
        }
        .valid());
        assert!(!Resolution {
            width: 8193,
            height: 480
        }
        .valid());
        assert!(!Resolution {
            width: 640,
            height: 479
        }
        .valid());
        assert!(!Resolution {
            width: u32::MAX,
            height: u32::MAX
        }
        .valid());
    }
    #[test]
    fn resolution_roundtrip_and_old_preferences_preserve_it() {
        let path = path("resolution");
        let resolution = Resolution {
            width: 1920,
            height: 1080,
        };
        let mut store = Store::load(&path);
        store
            .set_display_preferences(DisplayMode::Fullscreen, Some(resolution))
            .unwrap();
        store
            .display_preferences(true, DisplayMode::Windowed)
            .unwrap();
        let loaded = Store::load(&path);
        assert_eq!(loaded.resolution, Some(resolution));
        assert_eq!(loaded.display_mode, DisplayMode::Windowed);
        assert!(loaded.reduced_motion);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn missing_resolution_is_compatible_and_invalid_is_transactional() {
        let path = path("invalid-resolution");
        let mut value = envelope();
        std::fs::write(&path, value.to_string()).unwrap();
        let loaded = Store::load(&path);
        assert!(loaded.error.is_empty());
        assert_eq!(loaded.resolution, None);
        for invalid in [
            json!({"width":0,"height":800}),
            json!({"width":1920,"height":999999}),
            json!("1920x1080"),
            json!({"width":1280}),
        ] {
            value["resolution"] = invalid;
            std::fs::write(&path, value.to_string()).unwrap();
            let loaded = Store::load(&path);
            assert!(loaded.error.contains("Invalid display resolution"));
            assert_eq!(loaded.display_mode, DisplayMode::Windowed);
            assert!(!loaded.reduced_motion);
        }
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn combined_display_write_failure_keeps_both_previous_values() {
        let path = path("combined-failure");
        let mut store = Store::load(&path);
        let resolution = Resolution {
            width: 1280,
            height: 800,
        };
        store
            .set_display_preferences(DisplayMode::Windowed, Some(resolution))
            .unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        store.path = Some(path.join("impossible.json"));
        assert!(store
            .set_display_preferences(
                DisplayMode::Fullscreen,
                Some(Resolution {
                    width: 1920,
                    height: 1080
                })
            )
            .is_err());
        assert_eq!(store.display_mode, DisplayMode::Windowed);
        assert_eq!(store.resolution, Some(resolution));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), raw);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn every_mode_round_trips_and_survives_preset_and_motion_writes() {
        let path = path("roundtrip");
        for mode in [
            DisplayMode::Windowed,
            DisplayMode::Borderless,
            DisplayMode::Fullscreen,
        ] {
            assert_eq!(DisplayMode::from_sdl(mode.to_sdl()), mode);
            let mut store = Store::load(&path);
            store.display_preferences(true, mode).unwrap();
            let id = store.save_preset("Display test").unwrap();
            store.display_preferences(false, mode).unwrap();
            let mut loaded = Store::load(&path);
            assert!(loaded.error.is_empty());
            assert_eq!(loaded.display_mode, mode);
            assert_eq!(loaded.fullscreen, mode != DisplayMode::Windowed);
            assert!(!loaded.reduced_motion);
            loaded.delete_preset(&id, true).unwrap();
        }
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn legacy_boolean_migrates_and_explicit_mode_takes_precedence() {
        let path = path("legacy");
        let mut value = envelope();
        for fullscreen in [false, true] {
            value["fullscreen"] = json!(fullscreen);
            std::fs::write(&path, value.to_string()).unwrap();
            let mut loaded = Store::load(&path);
            assert!(loaded.error.is_empty());
            assert_eq!(loaded.display_mode, DisplayMode::from(fullscreen));
            loaded
                .display_preferences(false, loaded.display_mode)
                .unwrap();
            assert_eq!(
                Store::load(&path).display_mode,
                DisplayMode::from(fullscreen)
            );
        }
        value["fullscreen"] = json!(false);
        value["displayMode"] = json!("fullscreen");
        std::fs::write(&path, value.to_string()).unwrap();
        assert_eq!(Store::load(&path).display_mode, DisplayMode::Fullscreen);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn invalid_explicit_mode_rejects_entire_load_without_mutation() {
        let path = path("invalid");
        for invalid in [
            json!("exclusive"),
            json!("Borderless"),
            json!(null),
            json!(true),
            json!(7),
        ] {
            let mut value = envelope();
            value["displayMode"] = invalid;
            let raw = value.to_string();
            std::fs::write(&path, &raw).unwrap();
            let loaded = Store::load(&path);
            assert!(loaded.error.contains("Invalid display mode"));
            assert_eq!(loaded.display_mode, DisplayMode::Windowed);
            assert!(!loaded.reduced_motion);
            assert_eq!(std::fs::read_to_string(&path).unwrap(), raw);
        }
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn failed_persistence_keeps_previous_mode_and_file() {
        let path = path("failure");
        let mut store = Store::load(&path);
        store
            .display_preferences(false, DisplayMode::Borderless)
            .unwrap();
        let previous = std::fs::read_to_string(&path).unwrap();
        // A child of a regular file is guaranteed to be an invalid write target.
        store.path = Some(path.join("settings.json"));
        assert!(store
            .display_preferences(true, DisplayMode::Fullscreen)
            .is_err());
        assert_eq!(store.display_mode, DisplayMode::Borderless);
        assert!(!store.reduced_motion);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), previous);
        std::fs::remove_file(path).unwrap();
    }
}

#[cfg(test)]
mod trial_default_path_tests {
    use super::*;
    /// O(path bytes), compile-feature identity checked without mutating process environment.
    #[test]
    fn configured_base_uses_expected_application_directory(){
        let base=PathBuf::from("test-user-config");
        let expected=if cfg!(feature="append-recording-trial"){"corpse-gods-append-trial"}else{"corpse-gods"};
        assert_eq!(app_directory_name(),expected);
        assert_eq!(default_path_from_base(Some(base.clone())),base.join(expected).join("appearance.json"));
    }
    /// O(path bytes), save/profile/replay siblings stay isolated even without an environment base.
    #[test]
    fn derived_default_siblings_share_only_the_selected_app_directory(){
        for base in [None,Some(PathBuf::from("test-user-config"))]{
            let appearance=default_path_from_base(base.clone());
            for name in ["native-save.json","fixture-save.json","profile.json","replays"]{
                let sibling=appearance.with_file_name(name);
                if cfg!(feature="append-recording-trial"){
                    let parent=base.as_ref().map_or_else(||PathBuf::from("corpse-gods-append-trial"),|p|p.join("corpse-gods-append-trial"));
                    assert_eq!(sibling,parent.join(name));
                    assert_ne!(sibling,base.as_ref().map_or_else(||PathBuf::from(name),|p|p.join("corpse-gods").join(name)));
                }else{assert_eq!(sibling,base.as_ref().map_or_else(||PathBuf::from(name),|p|p.join("corpse-gods").join(name)));}
            }
        }
    }
    /// O(path bytes), missing-environment appearance fallback remains feature-specific.
    #[test]
    fn absent_base_uses_expected_appearance_fallback(){
        let expected=if cfg!(feature="append-recording-trial"){"corpse-gods-append-trial/appearance.json"}else{"native-appearance.json"};
        assert_eq!(default_path_from_base(None),PathBuf::from(expected));
    }
}
