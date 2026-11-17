//! First-four-floor ordinary drops. Special rewards and arrows are separate.
use crate::{Item, rng::GenerationRng};

pub(super) const KINDS: [&str; 3] = ["potion", "ring", "amulet"];

/// O(seed characters) hashing plus three fixed shuffle draws, O(seed bytes)
/// temporary key. Independent of population order, retries and simulation RNG.
pub(super) fn primary_kind(seed: &str, depth: u32) -> Option<&'static str> {
    if !(1..=4).contains(&depth) { return None; }
    let mut rng = crate::population::population_random(&format!("{seed}:early-floor-primary-loot"));
    let mut schedule = ["potion", "potion", "ring", "amulet"];
    for end in (1..schedule.len()).rev() {
        let chosen = (rng.random() * (end + 1) as f64).floor() as usize;
        schedule.swap(end, chosen);
    }
    Some(schedule[depth as usize - 1])
}

/// Two minor-count draws and one existing subtype draw per item. O(L + P) time
/// and returned memory for L=5..7 items and P payload bytes; no rejection loop.
/// Each instance gets a fresh ID; equal subtype rolls remain valid ordinary loot.
pub(super) fn generate(
    primary: &str, rng: &mut GenerationRng, next_id: &mut f64, revision:f64,
) -> Result<Vec<Item>, String> {
    generate_with_amulet_rules(primary,rng,next_id,revision,true)
}
pub(super) fn generate_with_amulet_rules(
    primary:&str,rng:&mut GenerationRng,next_id:&mut f64,revision:f64,current:bool,
)->Result<Vec<Item>,String> {
    generate_with_item_rules(primary,rng,next_id,revision,current,revision>=52.)
}
pub(super) fn generate_with_item_rules(primary:&str,rng:&mut GenerationRng,next_id:&mut f64,revision:f64,current:bool,current_potions:bool)->Result<Vec<Item>,String> {
    let counts = KINDS.map(|kind| if kind == primary { 3 } else { rng.integer(1, 2) });
    let mut items = Vec::with_capacity(counts.iter().sum::<i32>() as usize);
    for (kind, count) in KINDS.into_iter().zip(counts) {
        for _ in 0..count {
            let item = if kind == "potion" {
                Item {
                    id: *next_id,
                    name: "Unidentified potion".into(),
                    kind: kind.into(),
                    glyph: "!".into(),
                    effect: (*super::pick(crate::revision50_potions::roster(current_potions), rng)).to_string().into(),
                    identified: false.into(),
                    ..Default::default()
                }
            } else {
                super::revision50::create_with_amulet_rules(kind, rng, *next_id, revision,current)?
            };
            *next_id += 1.;
            items.push(item);
        }
    }
    Ok(items)
}
