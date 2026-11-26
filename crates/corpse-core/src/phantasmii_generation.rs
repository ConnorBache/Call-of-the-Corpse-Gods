//! Revision34 dream-only population and Phantasmii quest items.
//! The four persistent Nightmares and quest loot are added after ordinary budgets.
use crate::{Actor, Floor, GroundItem, Item};
use std::collections::HashSet;

pub const DAGGER_NAME: &str = "Dagger of the Phantasmii";
pub const SHARD_NAME: &str = "Shards of the Phantasmii";
pub const FLOOR_SHARD_NAME: &str = "Shard of the Phantasmii";
pub fn blessing_discount(r: &crate::Run) -> f64 {
    if crate::phantasmii_blessing_rules::uses_three_blessing_discount(r) { 3. } else { 10. }
}
pub fn blessing_description(r: &crate::Run) -> String {
    format!("Each carried shard lowers the Blessings needed for the next level by {}, to a minimum of zero.", blessing_discount(r))
}
// Stable item payload for save/replay parity. Display copy follows the live rules.
const FLOOR_SHARD_EFFECT: &str = "Each carried shard lowers the Blessings needed for the next level by 10, to a minimum of zero.";
pub const DAGGER_DESCRIPTION: &str = "Apply while standing on the Altar of the Phantasmii to enter Dreamwalking.";
pub const NIGHTMARE_COUNT: usize = 4;
pub const SHARD_COUNT: usize = 3;
pub const NIGHTMARE_DESCRIPTORS: [&str; 8] = ["Small", "Lesser", "Minor", "", "Major", "Greater", "Gargantuan", "Terrible"];
/// Fixed ordinary bodies nearest the following five-floor band's start. The final
/// band uses the strongest ordinary late-floor body, not the final boss. Traits,
/// rewards, disguises and special attacks are deliberately not copied.
pub const NIGHTMARE_BASELINES: [&str; 8] = ["bear", "long-saidri", "pale:Greater Biter", "dire-bear", "killspawn", "phoenix", "ursa-minor", "dragon"];

pub fn is_dagger(i: &Item) -> bool { i.kind == "tool" && i.name == DAGGER_NAME }
pub fn is_dream_shard(i: &Item) -> bool { i.kind == "quest" && i.name == SHARD_NAME }
pub fn is_floor_shard(i: &Item) -> bool { i.kind == "quest" && i.name == FLOOR_SHARD_NAME }
pub fn is_shard(i: &Item) -> bool { is_dream_shard(i) || is_floor_shard(i) }
pub fn floor_shard(id: f64) -> Item {
    Item { id, name: FLOOR_SHARD_NAME.into(), kind: "quest".into(), glyph: "*".into(),
        identified: true.into(), quantity: 1.0.into(), effect: FLOOR_SHARD_EFFECT.to_string().into(), ..Default::default() }
}
pub fn dagger(id: f64) -> Item {
    Item { id, name: DAGGER_NAME.into(), kind: "tool".into(), glyph: "/".into(),
        identified: true.into(), effect: DAGGER_DESCRIPTION.to_string().into(), ..Default::default() }
}
pub fn shard(id: f64) -> Item {
    Item { id, name: SHARD_NAME.into(), kind: "quest".into(), glyph: "*".into(),
        identified: true.into(), quantity: 1.0.into(),
        effect: "Three shards reveal the power of Dreamwalking.".to_string().into(), ..Default::default() }
}
pub fn tier(depth: u32) -> usize { (depth.saturating_sub(1) / 5).min(7) as usize }
pub fn nightmare(depth: u32, id: f64, x: f64, y: f64) -> Actor {
    let n = tier(depth);
    let baseline = crate::population::create_actor_revision(NIGHTMARE_BASELINES[n], id, x, y, false, 33.0)
        .expect("fixed ordinary Nightmare body baseline");
    let mut a = Actor { id, x, y, creature_kind: "phantasmii-nightmare".to_string().into(),
        name: if NIGHTMARE_DESCRIPTORS[n].is_empty() { "Nightmare".into() }
            else { format!("{} Nightmare", NIGHTMARE_DESCRIPTORS[n]) },
        glyph: "N".into(), color: "#d8a3f0".into(), hp: baseline.max_hp, max_hp: baseline.max_hp,
        energy: baseline.stats.max_energy, stats: baseline.stats, state: "wandering".into(),
        heartless: true.into(), abilities: vec!["enemy:beast:flight".into(), "enemy:beast:no-revival".into()],
        ..Default::default() };
    a.conditions.insert("phantasmiiNightmare".into(), 1.0);
    a.conditions.insert("nightmareTier".into(), (n + 1) as f64);
    a.conditions.insert("bestiaryTraitsInitialized".into(), 1.0);
    a
}
pub fn nightmare_revision(depth:u32,id:f64,x:f64,y:f64,revision:f64) -> Actor {
    let mut a=nightmare(depth,id,x,y);
    a.stats.accuracy=crate::combat::creature_birth_accuracy(revision,a.stats.accuracy);
    a.stats.dodge=crate::combat::creature_birth_dodge(revision,a.stats.dodge);
    a
}
/// The dagger's adjacent tile is reserved by startup's ordinary-loot sampler.
pub fn dagger_cell(f: &Floor) -> Option<usize> {
    let e = f.godbeast_event.as_ref().filter(|e| e.kind == "Phantasmii")?;
    let p = e.altar.as_ref()?;
    Some(p.y as usize * f.width() + p.x as usize + 1)
}
fn position(f: &Floor, at: usize) -> (f64, f64) { ((at % f.width()) as f64, (at / f.width()) as f64) }
/// Once per new floor: O(V + A + I + F + P) time and scratch for the fixed-degree
/// reachability pass and V cells, A actors, I loot, F fixture cells, P portal endpoints.
/// Select only clear reachable dry cells; the private stream shifts no existing draw.
pub(crate) fn floor_shard_cell(seed: &str, f: &Floor, actors: &[Actor], ground: &[GroundItem]) -> Result<usize, String> {
    let w = f.width();
    let mut occupied: HashSet<_> = actors.iter().map(|a| a.y as usize * w + a.x as usize)
        .chain(ground.iter().map(|g| g.y as usize * w + g.x as usize))
        .chain(crate::spawn_heatmap::fixture_cells(f))
        .chain(f.portals.as_ref().into_iter().flatten().flat_map(|p| [p.a, p.b]))
        .collect();
    if let Some(p) = crate::entrance::arrival(f) { occupied.insert(p.y as usize * w + p.x as usize); }
    let cells: Vec<_> = crate::generation::reachable_without_braziers(f, None).into_iter()
        .filter(|&at| matches!(f.tiles[at].as_str(), "floor" | "grass") && !occupied.contains(&at))
        .collect();
    if cells.is_empty() { return Err("No reachable dry cell for a Shard of the Phantasmii".into()); }
    let mut rng = crate::population::population_random(&format!("{seed}:floor:{}:floor-shard", f.depth));
    Ok(cells[(rng.random() * cells.len() as f64).floor() as usize])
}
/// O(V + A + I + K*K*V), K<=7 selections, O(V+A+I) scratch. Candidates include water
/// so water-heavy floors cannot silently omit required actors/shards. No terrain
/// mutation, ordinary budget/RNG consumption, repeated activation or respawning.
pub fn populate(seed: &str, f: &mut Floor, actors: &mut Vec<Actor>, ground: &mut Vec<GroundItem>, next_id: &mut f64) -> Result<(), String> {
    populate_revision(seed,f,actors,ground,next_id,34.)
}
pub fn populate_revision(seed: &str, f: &mut Floor, actors: &mut Vec<Actor>, ground: &mut Vec<GroundItem>, next_id: &mut f64, revision:f64) -> Result<(), String> {
    if actors.iter().any(|a| a.conditions.get("phantasmiiNightmare") == Some(&1.0)) {
        return Err("Dream population already generated".into());
    }
    let w = f.width();
    let reachable = crate::generation::reachable_without_braziers(f, None);
    let mut occupied: HashSet<usize> = actors.iter().map(|a| a.y as usize * w + a.x as usize)
        .chain(ground.iter().map(|g| g.y as usize * w + g.x as usize))
        .chain(crate::spawn_heatmap::fixture_cells(f)).collect();
    let dagger_at = dagger_cell(f);
    if let Some(at) = dagger_at {
        if at >= f.tiles.len() || !["floor", "grass"].contains(&f.tiles[at].as_str()) || occupied.contains(&at) {
            return Err("Phantasmii dagger has no clear adjacent altar tile".into());
        }
        occupied.insert(at);
    }
    let candidates: Vec<usize> = (0..f.tiles.len()).filter(|at| reachable.contains(at)
        && ["floor", "grass", "water", "shallow-water"].contains(&f.tiles[*at].as_str())
        && !occupied.contains(at)).collect();
    let count = NIGHTMARE_COUNT + if dagger_at.is_some() { SHARD_COUNT } else { 0 };
    if candidates.len() < count { return Err("Insufficient reachable cells for required dream population".into()); }
    let mut rng = crate::population::population_random(&format!("{seed}:floor:{}:phantasmii", f.depth));
    let mut selected = Vec::with_capacity(count);
    // One random anchor and maximum-minimum squared distance give scattered but
    // deterministic placements. Stable cell-order ties use the private stream.
    while selected.len() < count {
        let mut best = 0usize;
        let mut choices = Vec::new();
        for &at in &candidates {
            if selected.contains(&at) { continue; }
            let score = selected.iter().map(|&other| {
                let dx = at as i32 % w as i32 - other as i32 % w as i32;
                let dy = at as i32 / w as i32 - other as i32 / w as i32;
                (dx * dx + dy * dy) as usize
            }).min().unwrap_or(0);
            if choices.is_empty() || score > best { best = score; choices.clear(); }
            if score == best { choices.push(at); }
        }
        selected.push(choices[(rng.random() * choices.len() as f64).floor() as usize]);
    }
    if let Some(at) = dagger_at {
        let (x, y) = position(f, at);
        ground.push(GroundItem { x, y, item: dagger(*next_id), ..Default::default() });
        *next_id += 1.0;
        for &at in &selected[..SHARD_COUNT] {
            let (x, y) = position(f, at);
            ground.push(GroundItem { x, y, item: shard(*next_id), ..Default::default() });
            *next_id += 1.0;
        }
    }
    for &at in &selected[count-NIGHTMARE_COUNT..] {
        let (x, y) = position(f, at);
        actors.push(nightmare_revision(f.depth as u32, *next_id, x, y, revision));
        *next_id += 1.0;
    }
    Ok(())
}
