//! JSON is confined to import/export and legacy inverse-history compatibility.
use crate::Run;
use serde_json::{Value, json};
use std::fmt;
#[derive(Debug, Clone)]
pub struct StorageError(pub String);
impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl std::error::Error for StorageError {}
impl From<serde_json::Error> for StorageError {
    fn from(e: serde_json::Error) -> Self {
        Self(e.to_string())
    }
}
pub type Result<T> = std::result::Result<T, StorageError>;
fn invalid(s: &str) -> StorageError {
    StorageError(s.into())
}
pub fn canonical_value(run: &Run) -> Result<Value> {
    Ok(serde_json::to_value(run)?)
}
pub fn serialize(run: &Run) -> Result<String> {
    Ok(serde_json::to_string(run)?)
}
pub fn deserialize(raw: &str) -> Result<Run> {
    let mut v: Value = serde_json::from_str(raw)?;
    unpack(&mut v)?;
    normalize_inventory_display_order(&mut v);
    let mut run: Run = serde_json::from_value(v)?;
    validate(&run)?;
    crate::revision52_reserves::normalize_run(&mut run).map_err(StorageError)?;
    crate::revision50::settle_current_duels(&mut run);
    Ok(run)
}
/// Source-ordered compatibility loader. Original timeline is captured before mutation.
/// Runtime simulation revision admission remains independent and is never upgraded here.
pub fn deserialize_compatible(raw: &str) -> Result<Run> {
    decode_compatible_with(raw, &mut crate::migration_pipeline::Context::default(), 0)
}
pub(crate) fn decode_compatible_with(
    raw: &str,
    context: &mut crate::migration_pipeline::Context,
    depth: usize,
) -> Result<Run> {
    let mut value: Value = serde_json::from_str(raw)?;
    unpack(&mut value)?;
    // Display order is presentation-only and may be malformed in otherwise valid saves.
    let order = value
        .as_object_mut()
        .and_then(|v| v.shift_remove("inventoryDisplayOrder"));
    let mut run: Run = serde_json::from_value(value)?;
    validate(&run)?;
    crate::migration_pipeline::migrate_timeline(&mut run, context, depth)?;
    crate::revision52_reserves::normalize_run(&mut run).map_err(StorageError)?;
    if let Some(order) = order {
        let mut value = canonical_value(&run)?;
        value["inventoryDisplayOrder"] = order;
        normalize_inventory_display_order(&mut value);
        run = serde_json::from_value(value)?;
    }
    validate(&run)?;
    Ok(run)
}
/// Source presentation-only normalization; does not alter authoritative inventory order.
/// O(I + D) expected time/space for owned inventory IDs and display order.
fn normalize_inventory_display_order(v: &mut Value) {
    let Some(order) = v.get("inventoryDisplayOrder") else {
        return;
    };
    // IEEE numeric identity, including -0 == 0, matches Set<number>.
    let key = |n: f64| if n == 0.0 { 0 } else { n.to_bits() };
    let ids: std::collections::HashSet<_> = v
        .get("inventory")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|i| i.get("id").and_then(Value::as_f64))
        .map(key)
        .collect();
    let mut seen = std::collections::HashSet::new();
    let normalized: Vec<_> = order
        .as_array()
        .into_iter()
        .flatten()
        .filter(|id| {
            id.as_f64()
                .is_some_and(|n| ids.contains(&key(n)) && seen.insert(key(n)))
        })
        .cloned()
        .collect();
    v["inventoryDisplayOrder"] = Value::Array(normalized);
}
/// Canonical fixture reader deliberately performs no migration or normalization.
pub fn from_canonical(raw: &str) -> Result<Run> {
    let run: Run = serde_json::from_str(raw)?;
    validate(&run)?;
    Ok(run)
}
pub fn validate(run: &Run) -> Result<()> {
    crate::revision52_enemy_followups::validate_run(run).map_err(StorageError)?;
    if !crate::fast_mode::valid_configuration(run) { return Err(invalid("Invalid Fast mode flag/revision")); }
    if run.version != 1.0
        || !["Unblessed", "Blessed", "Avatar"].contains(&run.mode.as_str())
        || !["playing", "dead", "won"].contains(&run.status.as_str())
    {
        return Err(invalid("Invalid run version/mode/status"));
    }
    if !run.rng.is_finite() || run.rng.fract() != 0.0 || run.rng < 0.0 || run.rng > u32::MAX as f64
    {
        return Err(invalid("Invalid RNG state"));
    }
    if run.depth < 1.0
        || run.depth > 40.0
        || run.depth.fract() != 0.0
        || run.floors.len() > 40
        || !run.floors.iter().any(|f| f.depth == run.depth)
    {
        return Err(invalid("Invalid floor count/depth"));
    }
    if run.history.len() > 5 {
        return Err(invalid("History exceeds five frames"));
    }
    for (i, f) in run.floors.iter().enumerate() {
        let n = f.tiles.len();
        let expected_depth = if crate::fast_mode::enabled(run) {
            if i == 0 { 1. } else { crate::fast_mode::below(run, run.floors[i - 1].depth) }
        } else { (i + 1) as f64 };
        if f.depth != expected_depth
            || !matches!(n, 2291 | 2500 | 3600)
            || f.lights.len() != n
            || f.seen.len() != n
            || f.width() * f.height() != n
        {
            return Err(invalid("Invalid floor geometry"));
        }
        if !run.actors.contains_key(&f.depth.to_string())
            || !run.ground.contains_key(&f.depth.to_string())
        {
            return Err(invalid("Missing floor actor/ground block"));
        }
        if !crate::pocket_biomes::validate(f) { return Err(invalid("Invalid pocket biome metadata")); }
        if f.tiles.iter().any(|t| !TILES.contains(&t.as_str())) {
            return Err(invalid("Unknown terrain"));
        }
    }
    let marked = |i: &crate::Item| !i.dream_ground.is_missing();
    let ground_and_memory = run.ground.values().flat_map(|v| v.iter().map(|g| &g.item))
        .chain(run.remembered_items.as_ref().into_iter().flat_map(|m| m.values().flatten().map(|g| &g.item)))
        .chain(run.detected_items.as_ref().into_iter().flat_map(|m| m.values().flatten().map(|g| &g.item)));
    fn carried_marker(a: &crate::Actor) -> bool {
        a.monster_state.as_ref().and_then(|m| m.carried_items.as_ref()).is_some_and(|v| v.iter().any(|i| !i.dream_ground.is_missing()))
            || a.monster_state.as_ref().and_then(|m| m.pilot.as_ref()).is_some_and(|p| carried_marker(p))
    }
    if run.inventory.iter().any(marked)
        || std::iter::once(&run.player).chain(run.actors.values().flat_map(|v| v.iter())).any(carried_marker)
        || ground_and_memory.filter(|i| marked(i)).any(|i| {
        !crate::borge::weapon_enabled(run) || !crate::borge::is_shard(i) || !i.dream_ground.value_or(false)
    }) { return Err(invalid("Invalid dream ground item")); }
    crate::firebolt_travel::validate(run).map_err(StorageError)?;
    crate::validation::validate(run)?;
    Ok(())
}
pub const TILES: [&str; 11] = [
    "wall",
    "floor",
    "water",
    "chasm",
    "grass",
    "tree",
    "up",
    "down",
    "pit",
    "shallow-water",
    "speed-up",
];
const EXPANSION: [&str; 22] = [
    "currents",
    "rememberedCurrents",
    "rememberedIchor",
    "mud",
    "roots",
    "substrates",
    "rememberedTiles",
    "rememberedSubstrates",
    "slime",
    "razorleaves",
    "blood",
    "waterBlood",
    "ice",
    "puddles",
    "hellfire",
    "rememberedIce",
    "rememberedPuddles",
    "rememberedHellfire",
    "snow",
    "rememberedSnow",
    "ichor",
    "unused",
];
fn rle(v: &mut Value) -> Result<()> {
    if v.is_array() || v.is_null() {
        return Ok(());
    }
    let a = v
        .get("rle")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("Invalid RLE"))?;
    *v = expand(a)?;
    Ok(())
}
fn expand(a: &[Value]) -> Result<Value> {
    if a.len() % 2 != 0 {
        return Err(invalid("Odd RLE"));
    }
    let mut out = Vec::new();
    for pair in a.chunks_exact(2) {
        let n = pair[0]
            .as_u64()
            .or_else(|| {
                pair[0]
                    .as_f64()
                    .filter(|n| n.fract() == 0.0 && *n >= 0.0)
                    .map(|n| n as u64)
            })
            .ok_or_else(|| invalid("Invalid RLE count"))? as usize;
        if n == 0 || n > 3600 || out.len() + n > 3600 {
            return Err(invalid("RLE bound exceeded"));
        }
        out.extend(std::iter::repeat_n(pair[1].clone(), n));
    }
    Ok(Value::Array(out))
}
fn unpack(v: &mut Value) -> Result<()> {
    let grid = v.get("gridEncoding").and_then(Value::as_f64) == Some(1.0);
    let floors = v
        .get_mut("floors")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| invalid("Missing floors"))?;
    for f in floors {
        let o = f.as_object_mut().ok_or_else(|| invalid("Invalid floor"))?;
        for key in EXPANSION {
            if let Some(v) = o.get_mut(key) {
                rle(v)?;
            }
        }
        if o.remove("lightEncoding").as_ref().and_then(Value::as_f64) == Some(1.0) {
            let lights = o
                .get("lights")
                .and_then(Value::as_array)
                .ok_or_else(|| invalid("Invalid light RLE"))?;
            let decoded = expand(lights)?;
            o.insert("lights".into(), decoded);
        }
        if grid {
            let tiles = o
                .get("tiles")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("Invalid encoded tiles"))?;
            let mut decoded = Vec::new();
            for c in tiles.chars() {
                let i = c
                    .to_digit(10)
                    .ok_or_else(|| invalid("Invalid tile digit"))? as usize;
                decoded.push(json!(TILES[i]));
            }
            o.insert("tiles".into(), Value::Array(decoded));
            let seen = o
                .get("seen")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("Invalid encoded seen"))?;
            if seen.chars().any(|c| c != '0' && c != '1') {
                return Err(invalid("Invalid seen digit"));
            }
            let decoded = seen.chars().map(|c| json!(c == '1')).collect();
            o.insert("seen".into(), Value::Array(decoded));
        }
        if let Some(env) = o.get_mut("environment").and_then(Value::as_object_mut) {
            for (key, val) in env {
                if key == "provenance" {
                    if let Some(entries) = val.as_object_mut() {
                        for (_, entry) in entries {
                            if let Some(pair) = entry.as_object_mut() {
                                for (_, v) in pair {
                                    rle(v)?;
                                }
                            }
                        }
                    }
                } else if key != "nextFireReaction" {
                    rle(val)?;
                }
            }
        }
    }
    let o = v.as_object_mut().ok_or_else(|| invalid("Invalid run"))?;
    o.shift_remove("gridEncoding");
    if let Some(anchors) = o.shift_remove("rewindAnchors") {
        let a = anchors
            .as_array()
            .ok_or_else(|| invalid("Invalid anchors"))?;
        let anchor = |v: &Value| -> Result<Value> {
            if v.as_f64() == Some(-1.0) {
                return Ok(Value::Null);
            }
            let i = v.as_u64().ok_or_else(|| invalid("Invalid anchor index"))? as usize;
            a.get(i)
                .filter(|v| v.is_string())
                .cloned()
                .ok_or_else(|| invalid("Anchor index outside table"))
        };
        let future = anchor(&o.shift_remove("futureAnchor").unwrap_or(Value::Null))?;
        o.insert("future".into(), future);
        if let Some(h) = o.get_mut("history").and_then(Value::as_array_mut) {
            for raw in h {
                let mut frame: Value = serde_json::from_str(
                    raw.as_str()
                        .ok_or_else(|| invalid("Invalid history frame"))?,
                )?;
                let f = frame
                    .as_object_mut()
                    .ok_or_else(|| invalid("Invalid history object"))?;
                let future = anchor(&f.shift_remove("futureAnchor").unwrap_or(json!(-1)))?;
                f.insert("future".into(), future);
                *raw = json!(serde_json::to_string(&frame)?);
            }
        }
    }
    Ok(())
}

/// Parsed canonical equality: JSON numeric values compare as ECMAScript doubles;
/// object property order is immaterial, array order and absent/null are significant.
pub fn canonical_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| canonical_equal(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(k, a)| b.get(k).is_some_and(|b| canonical_equal(a, b)))
        }
        _ => a == b,
    }
}

pub fn validate_current_revision(run: &Run) -> Result<()> {
    crate::playtest::validate_profile(run).map_err(StorageError)?;
    if run.gameplay_revision.as_ref() != Some(&3.0)
        || run.divine_revision.as_ref() != Some(&1.0)
        || run.enemy_health_revision.as_ref() != Some(&1.0)
        || !matches!(run.simulation_revision.as_ref(), Some(29.0 | 30.0 | 31.0 | 32.0 | 33.0 | 34.0 | 35.0 | 36.0 | 37.0 | 38.0 | 39.0 | 40.0 | 41.0 | 42.0 | 43.0 | 44.0 | 45.0 | 46.0 | 50.0 | 51.0 | 52.0))
    {
        return Err(StorageError("Unsupported legacy migration: native execution requires gameplay 3, divine 1, enemy health 1, simulation 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 50, 51 or 52".into()));
    }
    Ok(())
}
