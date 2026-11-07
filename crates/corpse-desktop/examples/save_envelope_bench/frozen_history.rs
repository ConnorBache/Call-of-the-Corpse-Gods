//! Frozen pre-save-pass inverse walker and stringifier from sight/paint v17 candidate.
//! This benchmark-only reference is not linked into normal gameplay.
use corpse_core::{Field, temporal::Snapshot, storage::StorageError};
use serde::{Serialize, Deserialize};
use serde_json::{Value, json};
type Result<T> = std::result::Result<T, StorageError>;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    pub path: Vec<Value>,
    #[serde(default, skip_serializing_if = "Field::is_missing")]
    pub value: Field<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remove: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length: Option<usize>,
}
impl Patch {
    /// O(1) ownership transfer; allocations already belong to arguments.
    fn value(path: Vec<Value>, value: Value) -> Self {
        Self {
            path,
            value: Field::Value(value),
            remove: None,
            length: None,
        }
    }
    /// O(1) ownership transfer.
    fn remove(path: Vec<Value>) -> Self {
        Self {
            path,
            value: Field::Missing,
            remove: Some(true),
            length: None,
        }
    }
    /// O(1) ownership transfer.
    fn length(path: Vec<Value>, n: usize) -> Self {
        Self {
            path,
            value: Field::Missing,
            remove: None,
            length: Some(n),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub groups: Vec<Vec<Patch>>,
    pub future: Option<String>,
}
/// Recursive structural comparison plus path cloning and optional sparse-patch serialization;
/// O(S*D + Q) conservative time, O(P + D) memory: S nodes, D depth, Q repeated comparison/encoding work, P patches.
fn walk(a: &Value, b: Option<&Value>, path: Vec<Value>, patches: &mut Vec<Patch>) {
    if Some(a) == b {
        return;
    }
    if let (Value::Array(aa), Some(Value::Array(bb))) = (a, b) {
        let start = patches.len();
        if aa.len() != bb.len() {
            patches.push(Patch::length(path.clone(), aa.len()));
        }
        for (i, av) in aa.iter().enumerate() {
            let mut p = path.clone();
            p.push(json!(i));
            walk(av, bb.get(i), p, patches);
        }
        if patches.len() > start + 1 && aa.iter().all(|v| !v.is_array() && !v.is_object()) {
            let whole = Patch::value(path, a.clone());
            let sparse: usize = patches[start..]
                .iter()
                .map(|p| {
                    js_stringify(&serde_json::to_value(p).unwrap())
                        .encode_utf16()
                        .count()
                        + 1
                })
                .sum();
            if js_stringify(&serde_json::to_value(&whole).unwrap())
                .encode_utf16()
                .count()
                < sparse
            {
                patches.truncate(start);
                patches.push(whole);
            }
        }
        return;
    }
    if let (Value::Object(aa), Some(Value::Object(bb))) = (a, b) {
        for (k, av) in aa {
            let mut p = path.clone();
            p.push(json!(k));
            walk(av, bb.get(k), p, patches);
        }
        for k in bb.keys().filter(|k| !aa.contains_key(*k)) {
            let mut p = path.clone();
            p.push(json!(k));
            patches.push(Patch::remove(p));
        }
        return;
    }
    patches.push(Patch::value(path, a.clone()));
}
/// O(two snapshot serializations + walk + patch bytes), O(two Values + patches) memory.
pub fn inverse(before: &Snapshot, after: &Snapshot, retain_log: bool) -> Result<Vec<Patch>> {
    let mut a = before.value()?;
    let mut b = after.value()?;
    a.as_object_mut().unwrap().shift_remove("logArchiveCursor");
    b.as_object_mut().unwrap().shift_remove("logArchiveCursor");
    let mut p = Vec::new();
    walk(&a, Some(&b), vec![], &mut p);
    if retain_log {
        p.retain(|p| {
            !matches!(
                p.path.first().and_then(Value::as_str),
                Some("log" | "logKinds")
            )
        });
        p.push(Patch::value(vec![json!("log")], a["log"].clone()));
        p.push(if let Some(k) = a.get("logKinds") {
            Patch::value(vec![json!("logKinds")], k.clone())
        } else {
            Patch::remove(vec![json!("logKinds")])
        });
    }
    match (before.cursor, after.cursor) {
        (Some(a), Some(b)) => {
            for (k, x, y) in [
                ("events", a.events, b.events),
                ("messages", a.messages, b.messages),
            ] {
                if retain_log || x != y {
                    p.push(Patch::length(vec![json!("logArchive"), json!(k)], x));
                }
            }
        }
        (None, b) if b.is_some() || retain_log => p.push(Patch::remove(vec![json!("logArchive")])),
        _ => {}
    }
    Ok(p)
}
/// O(B*D) conservative time/transient allocation across recursive stringify, B bytes and D depth.
pub fn js_stringify(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => {
            let s = n.to_string();
            s.strip_suffix(".0").unwrap_or(&s).to_string()
        }
        Value::String(s) => serde_json::to_string(s).unwrap(),
        Value::Array(a) => format!(
            "[{}]",
            a.iter().map(js_stringify).collect::<Vec<_>>().join(",")
        ),
        Value::Object(o) => format!(
            "{{{}}}",
            o.iter()
                .map(|(k, v)| format!("{}:{}", serde_json::to_string(k).unwrap(), js_stringify(v)))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}

/// Frozen normal-run export: O(H*S + diff work) time, O(S + history bytes) memory.
/// Debug loadout branch is deliberately excluded; benchmark rejects debug runs.
pub fn export(engine: &corpse_core::temporal::Engine) -> corpse_core::Run {
    assert!(corpse_core::debug_floor::state(&engine.run).is_none());
    let history = &engine.history;
    let mut out = engine.run.clone();
    let mut newer = history.pending_haste.clone().or_else(|| history.closing.clone())
        .unwrap_or_else(|| Snapshot::capture(&engine.run));
    let mut frames = Vec::new();
    for older in history.frames.iter().rev() {
        let patches = inverse(older, &newer, true).unwrap();
        frames.push(js_stringify(&serde_json::to_value(Frame {
            groups: vec![patches], future: older.state.future.as_ref().map(|a| a.wire_text().unwrap().to_owned()),
        }).unwrap()));
        newer = older.clone();
    }
    frames.reverse(); out.history = frames;
    if let Some(p) = &history.pending_haste {out.haste_snapshot = Field::Value(js_stringify(&p.value().unwrap()));}
    out
}
