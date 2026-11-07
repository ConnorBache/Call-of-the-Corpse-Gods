//! Exact budget arithmetic; horde recipes, feasibility and population live in population.rs.
use crate::{
    Floor,
    rng::{GenerationRng, seed_hash},
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
pub enum BudgetSeed<'a> {
    Number(u32),
    Text(&'a str),
}
pub fn ordinary_blessing_budget(seed: BudgetSeed<'_>, depth: u32) -> Result<u32, String> {
    if !(1..=40).contains(&depth) {
        return Err("Invalid ordinary heart-budget floor".into());
    }
    let normalized = match seed {
        BudgetSeed::Number(n) => {
            if n == 0 {
                1
            } else {
                n
            }
        }
        BudgetSeed::Text(s) => seed_hash(s),
    };
    let key = format!("{normalized}:floor:{depth}:ordinary-heart-budget");
    let mut h = 2166136261u32;
    for b in key.bytes() {
        h = (h ^ b as u32).wrapping_mul(16777619)
    }
    Ok(depth + 6 + (GenerationRng::new(h).random() * 3.0).floor() as u32)
}
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct EncounterBudget {
    pub low: i32,
    pub medium: i32,
    pub high: i32,
}
pub fn base_encounter_budget(depth: u32, cells: usize) -> EncounterBudget {
    let orig = if depth <= 3 { [10, 5, 0] } else { [5, 5, 5] };
    let mut q = orig;
    if cells == 2291 {
        let scaled = orig.map(|v| v as f64 * (2291.0 / 2500.0));
        q = scaled.map(|v| v.floor() as i32);
        let rotation = (depth as i32 - 1).rem_euclid(3);
        let mut order = [0, 1, 2];
        order.sort_by(|&a, &b| {
            (scaled[b] - q[b] as f64)
                .total_cmp(&(scaled[a] - q[a] as f64))
                .then_with(|| ((a as i32 - rotation + 3) % 3).cmp(&((b as i32 - rotation + 3) % 3)))
        });
        for n in 0..(14 - q.iter().sum::<i32>()) as usize {
            q[order[n]] += 1
        }
    }
    EncounterBudget {
        low: q[0],
        medium: q[1],
        high: q[2],
    }
}
pub fn nominal_retained_slots(depth: u32) -> f64 {
    if depth % 3 == 0 { 6.0 } else { 7.0 }
}
fn recorded(f: &Floor) -> Option<f64> {
    let details = f
        .generation
        .as_ref()?
        .stages
        .iter()
        .find(|s| s.id == "items")?
        .details
        .as_ref()?;
    let keys = [
        "ordinaryLoot",
        "retainedHeartSlots",
        "carriedItemSlots",
        "retainedSlots",
        "retainedBlockTotal",
        "retainedBlockTarget",
        "retainedBlockOvershoot",
    ];
    if !keys.iter().all(|k| {
        details
            .get(*k)
            .is_some_and(|v| v.is_finite() && v.fract() == 0.0 && *v >= 0.0 && *v <= 12288.0)
    }) {
        return None;
    }
    let d = |k: &str| details[k];
    if d("ordinaryLoot") < 2.0
        || d("ordinaryLoot") > 20.0
        || d("retainedSlots") > 4096.0
        || d("retainedSlots") != d("ordinaryLoot") + d("retainedHeartSlots") + d("carriedItemSlots")
        || d("retainedBlockTotal") < d("retainedSlots")
        || d("retainedBlockTarget")
            != if f.depth % 3.0 == 0.0 {
                20.0
            } else {
                7.0 * ((f.depth - 1.0) % 3.0 + 1.0)
            }
        || d("retainedBlockOvershoot")
            != (d("retainedBlockTotal") - d("retainedBlockTarget")).max(0.0)
    {
        None
    } else {
        Some(d("retainedSlots"))
    }
}
/// Uses immutable generation diagnostics only; scans at most two prior depths. O(F+S), O(1) extra state.
pub fn retained_loot_allocation(
    depth: u32,
    floors: &[std::sync::Arc<Floor>],
    hearts: f64,
    carried: f64,
) -> Result<IndexMap<String, f64>, String> {
    if !(1..=40).contains(&depth)
        || ![hearts, carried]
            .iter()
            .all(|v| v.is_finite() && v.fract() == 0.0 && *v >= 0.0 && *v <= 9007199254740991.0)
    {
        return Err("Invalid retained loot budget".into());
    }
    let first = depth - (depth - 1) % 3;
    let mut prior = 0.0;
    for d in first..depth {
        prior += floors
            .iter()
            .find(|f| f.depth == d as f64)
            .and_then(|f| recorded(f))
            .unwrap_or(nominal_retained_slots(d));
    }
    let target = if depth % 3 == 0 {
        20.0
    } else {
        7.0 * (depth - first + 1) as f64
    };
    let loot = (target - prior - hearts - carried).max(2.0);
    let slots = loot + hearts + carried;
    let total = prior + slots;
    Ok([
        ("ordinaryLoot", loot),
        ("retainedHeartSlots", hearts),
        ("carriedItemSlots", carried),
        ("retainedSlots", slots),
        ("retainedBlockTotal", total),
        ("retainedBlockTarget", target),
        ("retainedBlockOvershoot", (total - target).max(0.0)),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v))
    .collect())
}
