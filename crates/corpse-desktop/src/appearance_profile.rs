//! Detached, versioned v48 appearance data. No simulation, clock, RNG, or saves.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::OnceLock;
pub const MAX_JSON_CHARACTERS: usize = 2_000_000;
pub const MAX_HISTORY_CHARACTERS: usize = 8_000_000;
pub const MAX_HISTORY: usize = 80;
pub const MAX_PRESETS: usize = 50;
#[path="appearance_compiled.rs"]
mod compiled;
pub use compiled::{GlobalTone,MaterialParent,MaterialView};
#[derive(Clone, Debug, Serialize)]
#[serde(transparent)]
pub struct Profile(Value, #[serde(skip)] std::sync::Arc<compiled::Compiled>);
impl PartialEq for Profile {fn eq(&self,other:&Self)->bool{self.0==other.0}}
#[derive(Clone, Debug, Deserialize)]
pub struct Control {
    pub path: String,
    pub category: String,
    pub group: Option<String>,
    pub label: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    #[serde(default)]
    pub options: Vec<String>,
    pub description: Option<String>,
}
pub fn controls() -> &'static [Control] {
    static CONTROLS: OnceLock<Vec<Control>> = OnceLock::new();
    CONTROLS.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/appearance-controls-v48.json"))
            .expect("frozen appearance controls")
    })
}
pub fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}
fn forbidden(key: &str) -> bool {
    matches!(key, "__proto__" | "constructor" | "prototype")
}
pub fn get<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
    let mut v = v;
    for key in path.split('.') {
        if forbidden(key) {
            return None;
        }
        v = v.get(key)?;
    }
    Some(v)
}
fn get_mut<'a>(v: &'a mut Value, path: &str) -> Option<&'a mut Value> {
    let mut v = v;
    for key in path.split('.') {
        if forbidden(key) {
            return None;
        }
        v = v.get_mut(key)?;
    }
    Some(v)
}
fn safe(v: &Value, depth: usize) -> Result<(), String> {
    if depth > 32 {
        return Err("Appearance nesting exceeds 32 levels.".into());
    }
    match v {
        Value::String(s) if utf16_len(s) > 100_000 => {
            return Err("Appearance text is too long.".into())
        }
        Value::Object(m) => {
            if m.len() > 10_000 {
                return Err("Too many appearance properties.".into());
            }
            for (k, child) in m {
                if forbidden(k) {
                    return Err(format!("{k}: unsafe property name."));
                }
                safe(child, depth + 1)?;
            }
        }
        Value::Array(a) => {
            if a.len() > 10_000 {
                return Err("Too many appearance entries.".into());
            }
            for child in a {
                safe(child, depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}
// JavaScript has one number type; serde distinguishes 1 and 1.0. Normalize
// exactly representable integral editor values so no-op edits and inheritance
// do not depend on whether a value arrived via JSON or a native number field.
fn normalize_numbers(value: &mut Value) {
    match value {
        Value::Number(number) => {
            if let Some(n) = number
                .as_f64()
                .filter(|n| n.fract() == 0. && n.abs() <= 9_007_199_254_740_991.)
            {
                *value = Value::from(n as i64);
            }
        }
        Value::Array(values) => {
            for child in values {
                normalize_numbers(child);
            }
        }
        Value::Object(values) => {
            for child in values.values_mut() {
                normalize_numbers(child);
            }
        }
        _ => {}
    }
}
fn merge(defaults: &Value, supplied: &mut Value, path: &str) -> Result<(), String> {
    match defaults {
        Value::Object(m) => {
            let target = supplied
                .as_object_mut()
                .ok_or_else(|| format!("{path}: expected an object."))?;
            for (k, v) in m {
                if let Some(child) = target.get_mut(k) {
                    merge(v, child, &format!("{path}.{k}"))?;
                } else {
                    target.insert(k.clone(), v.clone());
                }
            }
        }
        _ => {
            let same = match defaults {
                Value::String(_) => supplied.is_string(),
                Value::Number(_) => supplied.is_number(),
                Value::Bool(_) => supplied.is_boolean(),
                Value::Null => supplied.is_null(),
                _ => false,
            };
            if !same {
                return Err(format!("{path}: wrong appearance value type."));
            }
        }
    }
    Ok(())
}
impl Default for Profile {
    fn default() -> Self {
        Self::defaults().clone()
    }
}
impl Profile {
    pub fn defaults() -> &'static Self {
        static DEFAULT: OnceLock<Profile> = OnceLock::new();
        DEFAULT.get_or_init(|| {
            let value:Value=serde_json::from_str(include_str!("../assets/appearance-defaults.json")).expect("frozen appearance defaults");
            let compiled=compiled::Compiled::build(&value,&value);
            Profile(value,std::sync::Arc::new(compiled))
        })
    }
    pub fn parse(raw: &str) -> Result<Self, String> {
        if utf16_len(raw) > MAX_JSON_CHARACTERS {
            return Err("Appearance JSON exceeds 2 MB.".into());
        }
        let value =
            serde_json::from_str(raw).map_err(|_| "Appearance JSON is malformed.".to_string())?;
        Self::from_value(value)
    }
    // O(B + C*d): B JSON characters, C fixed controls, d <=32 field depth.
    pub fn from_value(mut value: Value) -> Result<Self, String> {
        safe(&value, 0)?;
        normalize_numbers(&mut value);
        if !value.is_object() {
            return Err("Appearance must be a JSON object.".into());
        }
        if value
            .get("schemaVersion")
            .is_some_and(|v| v.as_f64() != Some(1.))
        {
            return Err("Unsupported appearance schema version; expected 1.".into());
        }
        if utf16_len(&value.to_string()) > MAX_JSON_CHARACTERS {
            return Err("Appearance JSON exceeds 2 MB.".into());
        }
        merge(&Self::defaults().0, &mut value, "Appearance")?;
        if utf16_len(&value.to_string()) > MAX_JSON_CHARACTERS {
            return Err("Appearance JSON exceeds 2 MB after adding defaults.".into());
        }
        let compiled=compiled::Compiled::build(&value,&Self::defaults().0);
        let result = Self(value,std::sync::Arc::new(compiled));
        let name = result.text("presetName");
        if name.trim().is_empty() || utf16_len(name) > 120 {
            return Err("Preset name must contain 1–120 characters.".into());
        }
        for control in controls() {
            let v = result
                .value(&control.path)
                .ok_or_else(|| format!("Missing appearance value: {}", control.path))?;
            let valid = match control.kind.as_str() {
                "number" => v.as_f64().is_some_and(|n| {
                    n.is_finite() && n >= control.min.unwrap() && n <= control.max.unwrap()
                }),
                "color" => v.as_str().is_some_and(|s| {
                    s.len() == 7
                        && s.starts_with('#')
                        && s[1..].bytes().all(|b| b.is_ascii_hexdigit())
                }),
                "select" => v
                    .as_str()
                    .is_some_and(|s| control.options.iter().any(|o| o == s)),
                "glyph" => v.as_str().is_some_and(|s| {
                    (s.len() == 1 && (32..=126).contains(&s.as_bytes()[0]))
                        || Some(v) == Self::defaults().value(&control.path)
                        // Preserve the two historical multi-character defaults
                        // as explicit overrides when importing saved profiles.
                        || (s == "(≬)" && matches!(control.path.as_str(),
                            "entities.creatures.pale:Birther.glyph"
                            | "entities.creatures.pale:Greater Birther.glyph"))
                }),
                "text" => v.as_str().is_some_and(|s| {
                    !s.trim().is_empty()
                        && utf16_len(s) <= 500
                        && !s.chars().any(|c| ";{}<>\n\r".contains(c))
                }),
                "boolean" => v.is_boolean(),
                _ => false,
            };
            if !valid {
                return Err(format!(
                    "{}: invalid {} value{}.",
                    control.path,
                    control.label,
                    if control.kind == "number" {
                        format!(" ({}–{})", control.min.unwrap(), control.max.unwrap())
                    } else {
                        String::new()
                    }
                ));
            }
        }
        if result.number("global.minimumBrightness") > result.number("global.maximumBrightness") {
            return Err("Minimum terrain brightness cannot exceed maximum brightness.".into());
        }
        Ok(result)
    }
    pub fn value(&self, path: &str) -> Option<&Value> {
        corpse_core::timing::count(corpse_core::timing::Count::ProfileValue,1);
        get(&self.0, path)
    }
    pub fn data(&self) -> &Value {
        &self.0
    }
    pub fn number(&self, path: &str) -> f64 {
        self.1.number(path)
    }
    pub fn text(&self, path: &str) -> &str {
        self.1.text(path)
    }
    pub fn boolean(&self, path: &str) -> bool {
        self.1.boolean(path)
    }
    pub fn rgb(&self, path: &str) -> crate::render::Rgb {
        corpse_core::timing::count(corpse_core::timing::Count::ProfileRgb,1);
        self.1.rgb(path)
    }
    pub fn json(&self) -> String {
        let pretty = serde_json::to_string_pretty(&self.0).unwrap() + "\n";
        // A near-limit valid profile must remain importable after export. The
        // browser always pretty-prints, which can exceed its own import cap.
        if utf16_len(&pretty) <= MAX_JSON_CHARACTERS {
            pretty
        } else {
            self.0.to_string()
        }
    }
    pub fn characters(&self) -> usize {
        utf16_len(&self.0.to_string())
    }
    pub fn set(&self, path: &str, value: Value) -> Result<Self, String> {
        if !controls().iter().any(|c| c.path == path) {
            return Err(format!("Unknown appearance setting: {path}."));
        }
        let mut next = self.0.clone();
        *get_mut(&mut next, path).ok_or("Missing appearance field")? = value;
        let keys: Vec<_> = path.split('.').collect();
        if keys.first() == Some(&"entities")
            && keys.len() == 4
            && ["glyph", "color", "background"].contains(&keys[3])
        {
            *get_mut(&mut next, &format!("{}.inherited", keys[..3].join("."))).unwrap() =
                Value::Bool(false);
        }
        Self::from_value(next)
    }
    pub fn reset(&self, path: &str) -> Result<Self, String> {
        let default = Self::defaults()
            .value(path)
            .ok_or("Unknown appearance default")?
            .clone();
        let mut next = self.set(path, default)?;
        let keys: Vec<_> = path.split('.').collect();
        if keys.first() == Some(&"entities")
            && keys.len() == 4
            && ["glyph", "color", "background"].contains(&keys[3])
        {
            let base = keys[..3].join(".");
            let identity_default = ["glyph", "color", "background"].iter().all(|field| {
                let p = format!("{base}.{field}");
                if *field == "glyph" {
                    next.value(&p) == Self::defaults().value(&p)
                } else {
                    next.text(&p)
                        .eq_ignore_ascii_case(Self::defaults().text(&p))
                }
            });
            let p = format!("{base}.inherited");
            *get_mut(&mut next.0, &p).unwrap() = Value::Bool(if identity_default {
                Self::defaults().boolean(&p)
            } else {
                self.boolean(&p)
            });
        }
        next.recompile();
        Ok(next)
    }
    pub fn reset_group(&self, path: &str) -> Result<Self, String> {
        let default = Self::defaults()
            .value(path)
            .filter(|v| v.is_object())
            .ok_or("Not an appearance group")?;
        let mut next = self.0.clone();
        *get_mut(&mut next, path).ok_or("Unknown appearance group")? = default.clone();
        Self::from_value(next)
    }
    pub fn named(&self, name: &str) -> Self {
        let mut result = self.clone();
        result.0["presetName"] = Value::String(name.into());
        result.recompile();
        result
    }
    /// Rebuild only after private JSON mutation, never on view/clone/paint.
    fn recompile(&mut self){self.1=std::sync::Arc::new(compiled::Compiled::build(&self.0,&Self::defaults().0));}
    /// Expected O(key bytes) hash lookup, borrowed typed result; no clone/parse.
    pub fn resolved_material(&self,key:&str,parent:MaterialParent)->Option<MaterialView<'_>>{
        self.1.material(key,parent)
    }
    /// Exact raw JSON equality compiled once, including case-sensitive strings.
    pub fn is_default(&self,path:&str)->bool{self.1.is_default(path)}
    /// O(1) immutable borrow; no formatting, hashing, JSON or float conversion.
    pub fn global_tone(&self)->&GlobalTone{&self.1.global_tone}
    #[cfg(test)] pub fn compiled_storage_stats(&self)->Value{self.1.storage_stats()}
    /// Source material inheritance: only unchanged child fields inherit changed parent fields.
    pub fn material(&self, key: &str, parent: Option<&str>) -> Option<Value> {
        corpse_core::timing::count(corpse_core::timing::Count::ProfileMaterial,1);
        let own = self.value(&format!("terrain.{key}"))?;
        let Some(parent) = parent.filter(|p| *p != key) else {
            return Some(own.clone());
        };
        let (Some(base), Some(above), Some(parent_base)) = (
            Self::defaults().value(&format!("terrain.{key}")),
            self.value(&format!("terrain.{parent}")),
            Self::defaults().value(&format!("terrain.{parent}")),
        ) else {
            return Some(own.clone());
        };
        let mut result = own.clone();
        for (field, default) in base.as_object()? {
            if own.get(field) == Some(default) && above.get(field) != parent_base.get(field) {
                if let Some(v) = above.get(field) {
                    result[field] = v.clone();
                }
            }
        }
        Some(result)
    }
}

include!("appearance_compiled_tests.rs");
