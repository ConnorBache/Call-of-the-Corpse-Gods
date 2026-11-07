//! Revision32 flavor-first ordinary placement. These are FINAL bounds, not inputs
//! to revision30's two-floor expansion. Pure lookup: no RNG, no state mutation.
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
pub struct Placement {
    pub id: String,
    pub first: u32,
    pub last: u32,
    pub reference_depth: f64,
    pub role: String,
    pub target_lifetime_damage_pct: Option<f64>,
}
pub fn placements() -> &'static [Placement] {
    static DATA: OnceLock<Vec<Placement>> = OnceLock::new();
    DATA.get_or_init(|| serde_json::from_str(include_str!("catalog/flavor-placement-v32.json"))
        .expect("revision32 flavor placement"))
}
pub fn bounds(kind: &str) -> Option<(u32,u32)> {
    placements().iter().find(|s| s.id == kind).map(|s| (s.first,s.last))
}
pub fn reference_depth(kind: &str) -> Option<f64> {
    placements().iter().find(|s| s.id == kind).map(|s| s.reference_depth)
}
pub fn eligible(kind: &str, depth: u32) -> bool {
    bounds(kind).is_some_and(|(first,last)| (first..=last).contains(&depth))
}
/// A corpse may revive: corpse props must obey the same species placement. Fixed
/// 40-element lookup preserves draw count; optional setups still decide occurrence.
pub fn corpse_kind(depth: u32) -> &'static str {
    const KINDS: [&str;40] = [
        "blorb", "shellback-beetle", "hemogoblin", "cave-bat", "pale:Jackal",
        "short-saidri", "pale:Biter", "dryad", "hemogoblin-bloodmancer", "pale:Spitter",
        "pale:Screamer", "pale:Birther", "giant-beetle", "bear", "tiger",
        "horn-1", "horn-2", "tiger", "bombardier", "long-saidri",
        "wendigo", "wendigo", "pale:Greater Spitter", "troll", "pale:Greater Breather",
        "pale:Greater Screamer", "pale:Greater Birther", "horn-3", "horn-3", "killspawn",
        "horn-4", "vampire", "witch", "maulchild", "horn-5",
        "phoenix", "saidri-guardian", "moonspawn", "valta", "saidri-guardian",
    ];
    assert!((1..=40).contains(&depth), "corpse depth must be1..40");
    KINDS[depth as usize-1]
}
/// Existing ungenerated arrangements stay disabled. For authored active creature
/// arrangements, eligibility is the intersection of members' final windows; keep
/// fixed Samara terrain windows and all non-creature setup windows unchanged.
/// O(M*K) bounded metadata lookups, M<=5, K=57; no geometry/pathfinding or RNG.
pub fn setup_eligible(def: &crate::generation::SetupDefinition, depth: u32) -> bool {
    if !(1..=40).contains(&depth) || !def.eligible.iter().any(|v| *v) { return false; }
    if def.kinds.is_empty() { return def.eligible[depth as usize-1]; }
    def.kinds.iter().all(|kind| match bounds(kind) {
        Some((first,last)) => (first..=last).contains(&depth),
        None if kind == "samara" => (8..=40).contains(&depth),
        None => def.eligible[depth as usize-1],
    })
}
pub fn setup_introduced(def: &crate::generation::SetupDefinition, depth: u32) -> bool {
    if def.kinds.is_empty() { return def.introduced[depth as usize-1]; }
    def.kinds.iter().any(|kind| bounds(kind).is_some_and(|(first,_)| first==depth)
        || (kind == "samara" && depth==8))
}

/// Setup108's low-end watcher is a dynamic creature, absent from its frozen kinds
/// metadata. Choose the earliest-introduced eligible current species with the same
/// hazard exclusions, preserving roster order for ties and never orphaning a pair.
/// O(L*K + L*A) bounded metadata work, L=55, K=57, A max authored traits; no RNG.
pub fn water_watcher_kind(depth:u32,families:Option<&[String]>) -> Result<String,String> {
    if !(1..=40).contains(&depth) { return Err("Invalid single creature depth".into()); }
    let catalog=crate::population::catalog();
    let all=&crate::generation::generation_bestiary().families;
    let families=families.unwrap_or(all);
    crate::ordinary_roster::species().iter().filter(|s| eligible(&s.id,depth)
        && families.contains(&s.family) && s.id!="dryad" && !catalog.pairs.contains_key(&s.id)
        && !(s.id.starts_with("pale:") && s.id.contains("Breather"))
        && !catalog.actors.get(&s.id).is_some_and(|a|a.abilities.iter()
            .any(|id|id=="enemy:beast:slime-trail" || id=="enemy:beast:fungal-gas")))
        .min_by_key(|s|bounds(&s.id).unwrap().0).map(|s|s.id.clone())
        .ok_or_else(||"No eligible current single creature".into())
}
