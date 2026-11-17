//! Frozen authored catalog and deterministic blessing offers.
use crate::{model::*, resources, rng};
use serde::Deserialize;
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ability {
    pub id: String,
    pub god_id: f64,
    pub name: String,
    pub description: String,
    pub flags: Vec<String>,
}
pub fn catalog() -> &'static [Ability] {
    static C: std::sync::OnceLock<Vec<Ability>> = std::sync::OnceLock::new();
    C.get_or_init(|| {
        serde_json::from_str(include_str!("../catalog/abilities-v48.json")).expect("frozen catalog")
    })
}

pub const WEAVE_ID: &str = "numo-weave-of-time";
pub const WEAVE_NAME: &str = "Weave of Time";
pub const WEAVE_DESCRIPTION: &str = "You can freely travel along your time weave, which you leave behind as you move.";
/// Warm O(1); first revision41 use clones O(A + catalog text bytes), A=72.
/// Old revisions return the original frozen catalog without altering its order.
pub fn catalog_for_revision(revision: f64) -> &'static [Ability] {
    if crate::hanged::enabled(revision) {
        static R52: std::sync::OnceLock<Vec<Ability>> = std::sync::OnceLock::new();
        return R52.get_or_init(|| {
            let mut catalog = catalog_for_revision(50.).to_vec();
            let a = catalog.iter_mut().find(|a| a.id == crate::hanged::ID).expect("Missed Me slot");
            a.name = "Hanged".into();
            a.description = crate::hanged::DESCRIPTION.into();
            a.flags = vec!["passive".into()];
            for a in &mut catalog {
                if matches!(a.id.as_str(), "isheilah-quick-hand" | "isheilah-are-you-sure-about-that") {
                    a.description = a.description.replace("Reactions refresh when you wait.", "Waiting restores 1 reaction, up to your maximum.");
                }
            }
            catalog
        });
    }
    if revision >= 50.0 {
        static R50: std::sync::OnceLock<Vec<Ability>> = std::sync::OnceLock::new();
        return R50.get_or_init(|| serde_json::from_str(include_str!("../catalog/abilities-v50.json")).expect("revision 50 catalog"));
    }
    if revision < 41.0 { return catalog(); }
    static C: std::sync::OnceLock<Vec<Ability>> = std::sync::OnceLock::new();
    C.get_or_init(|| {
        let mut entries = catalog().to_vec();
        let entry = entries.iter_mut().find(|a| a.id == "numo-revert").expect("frozen Revert slot");
        *entry = Ability { id: WEAVE_ID.into(), god_id: entry.god_id,
            name: WEAVE_NAME.into(), description: WEAVE_DESCRIPTION.into(), flags: vec!["passive".into()] };
        entries
    })
}
/// O(A + ID bytes); metadata lookup only, never changes offer eligibility/order.
/// Legacy Revert metadata remains available for old runs and recordings.
pub fn definition(id: &str) -> Option<&'static Ability> {
    if id == WEAVE_ID { catalog_for_revision(41.0).iter().find(|a| a.id == id) }
    else { catalog().iter().find(|a| a.id == id).or_else(|| catalog_for_revision(50.0).iter().find(|a| a.id == id)) }
}
/// Version-aware UI/support metadata. Old saved runs retain the original text.
pub fn definition_for_revision(revision:f64,id:&str)->Option<&'static Ability> {
    catalog_for_revision(revision).iter().find(|a|a.id==id)
}
/// O(A*O + description bytes and tag overlap), A catalog entries and O owned IDs,
/// plus string-comparison bytes; same allocation/scan shape as legacy weighting.
/// The old branch invokes the unchanged legacy function, preserving exact arithmetic.
pub fn offer_weight_for_revision(revision: f64, candidate: &Ability, acquired: &[String]) -> f64 {
    if revision < 41.0 { return offer_weight(candidate, acquired); }
    let owned: Vec<_> = catalog_for_revision(revision).iter().filter(|a| acquired.contains(&a.id)).collect();
    let same = owned.iter().filter(|a| a.god_id == candidate.god_id).count();
    if same == 0 { return 1.0; }
    let ct = tags(candidate);
    let overlap: usize = owned.iter().map(|a| tags(a).iter().filter(|t| ct.contains(t)).count()).sum();
    1.0 + (same * overlap) as f64
}
/// O(pool length * weight cost), O(pool length) weights; identical legacy branch below41.
fn choose_revision<'a>(revision: f64, pool: &[&'a Ability], owned: &[String], sample: f64) -> &'a Ability {
    if revision < 41.0 { return choose(pool, owned, sample); }
    let weights: Vec<_> = pool.iter().map(|a| offer_weight_for_revision(revision,a,owned)).collect();
    let mut point = sample.clamp(0.0,0.999999999) * weights.iter().sum::<f64>();
    for (a,w) in pool.iter().zip(weights) { point -= w; if point < 0.0 { return a; } }
    pool.last().unwrap()
}

fn tags(a: &Ability) -> Vec<usize> {
    let s = a.description.to_lowercase();
    let patterns: [&[&str]; 9] = [
        &["offense", "damage", "attack"],
        &["defense", "health"],
        &["energy"],
        &["telepathy", "light", "illumin", "see", "sight"],
        &["move", "flying", "haste", "square", "chasm"],
        &["craft", "equipment", "potion"],
        &["die", "death", "blood", "nightmare"],
        &["reaction", "miss", "roll"],
        &["paraly", "ally", "neutral", "cannot"],
    ];
    patterns
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            (p.iter().any(|v| s.contains(v))
                || (i == 1
                    && s.match_indices("dr").any(|(j, _)| {
                        s[j + 2..]
                            .chars()
                            .next()
                            .is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_')
                    })))
            .then_some(i)
        })
        .collect()
}
pub fn offer_weight(candidate: &Ability, acquired: &[String]) -> f64 {
    let owned: Vec<_> = catalog()
        .iter()
        .filter(|a| acquired.contains(&a.id))
        .collect();
    let same = owned
        .iter()
        .filter(|a| a.god_id == candidate.god_id)
        .count();
    if same == 0 {
        return 1.0;
    }
    let ct = tags(candidate);
    let overlap: usize = owned
        .iter()
        .map(|a| tags(a).iter().filter(|t| ct.contains(t)).count())
        .sum();
    1.0 + (same * overlap) as f64
}
fn choose<'a>(pool: &[&'a Ability], owned: &[String], sample: f64) -> &'a Ability {
    let weights: Vec<_> = pool.iter().map(|a| offer_weight(a, owned)).collect();
    let mut point = sample.clamp(0.0, 0.999999999) * weights.iter().sum::<f64>();
    for (a, w) in pool.iter().zip(weights) {
        point -= w;
        if point < 0.0 {
            return a;
        }
    }
    pool.last().unwrap()
}
/// O(1); twenty-floor level costs begin at one and double for each subsequent level.
pub fn level_up_cost(r: &Run) -> f64 {
    2.0f64.powf(r.level - if r.compressed_depths.value_or(false) { 1. } else { 0. })
}
/// O(A²) with fixed authored A=72; deterministic god/pool and RNG order.
pub fn level_up(r: &mut Run) {
    let cost = level_up_cost(r);
    if r.exp < cost || !r.offers.is_empty() {
        return;
    }
    // Validate every persisted reserve before level/experience/RNG mutation.
    if crate::revision50::enabled(r)
        && crate::revision50_ability_energy::validate_reserves(&r.player).is_err() { return; }
    let p = r.player.clone();
    resources::record_loss(r, &p, "blessings", cost);
    r.exp -= cost;
    r.level += 1.0;
    if crate::revision50::enabled(r) {
        // Already validated above. Player energy is intentionally not refilled.
        crate::revision50_ability_energy::refill_on_level_up_for_revision(&mut r.player, r.simulation_revision.value_or(29.)).unwrap();
    }
    r.player.conditions.shift_remove("pelianUsed");
    r.player.conditions.insert("rageCost".into(), 0.0);
    r.offers.clear();
    if r.gods.is_empty() {
        r.offers.push("eclipsed".into());
        return;
    }
    let gods = r.gods.clone();
    if r.mode == "Avatar" {
        for god in gods {
            let pool: Vec<_> = catalog_for_revision(r.simulation_revision.value_or(29.0))
                .iter()
                .filter(|a| a.god_id == god && !r.player.abilities.contains(&a.id))
                .collect();
            let id = if pool.is_empty() {
                format!("eclipsed:{god}")
            } else {
                let sample = rng::random(r);
                choose_revision(r.simulation_revision.value_or(29.0), &pool, &r.player.abilities, sample).id.clone()
            };
            r.offers.push(id);
        }
    } else {
        let mut pool: Vec<_> = catalog_for_revision(r.simulation_revision.value_or(29.0))
            .iter()
            .filter(|a| a.god_id == gods[0] && !r.player.abilities.contains(&a.id))
            .collect();
        if pool.is_empty() {
            r.offers.push("eclipsed".into());
        }
        for _ in 0..3 {
            if pool.is_empty() {
                break;
            }
            let sample = rng::random(r);
            let id = choose_revision(r.simulation_revision.value_or(29.0), &pool, &r.player.abilities, sample).id.clone();
            pool.retain(|a| a.id != id);
            r.offers.push(id);
        }
    }
}
pub fn award(r: &mut Run, n: f64) {
    r.exp += n;
    let s = r.run_stats.get_or_insert_with(|| RunStatistics {
        version: 1.0,
        ..Default::default()
    });
    if n < 0.0
        || n.fract() != 0.0
        || !n.is_finite()
        || s.blessings_received + n > 9007199254740991.0
    {
        s.blessings_complete = false;
    } else {
        s.blessings_received += n;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_authored_weighted_choices() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/progression-v48.json")).unwrap();
        for c in cases.as_array().unwrap() {
            let owned: Vec<String> = serde_json::from_value(c["owned"].clone()).unwrap();
            for (i, a) in catalog().iter().enumerate() {
                assert_eq!(
                    offer_weight(a, &owned),
                    c["weights"][i].as_f64().unwrap(),
                    "{}",
                    a.id
                );
            }
            let pool: Vec<_> = catalog()
                .iter()
                .filter(|a| !owned.contains(&a.id))
                .collect();
            for choice in c["choices"].as_array().unwrap() {
                assert_eq!(
                    choose(&pool, &owned, choice["sample"].as_f64().unwrap()).id,
                    choice["id"].as_str().unwrap()
                );
            }
        }
    }
}
/// O(A+I), admitted ability learning preserves order and never initializes unknown hooks.
pub fn choose_offer(r: &mut Run, id: &str) -> Result<bool, crate::engine::EngineError> {
    if !r.offers.iter().any(|s| s == id) {
        return Ok(false);
    }
    if id.starts_with("eclipsed") {
        let offense = rng::random(r) < 0.5;
        if offense {
            r.player.stats.offense = crate::combat::integer_stat(r.player.stats.offense) + 5.0
        } else {
            r.player.stats.defense = crate::combat::integer_stat(r.player.stats.defense) + 5.0
        }
    } else {
        if !crate::abilities::supported(id) && !matches!(id, "numo-revert" | "morae-see-the-future")
        {
            return Err(crate::engine::EngineError(format!("learning ability {id}")));
        }
        let a = catalog_for_revision(r.simulation_revision.value_or(29.0))
            .iter()
            .find(|a| a.id == id)
            .ok_or_else(|| crate::engine::EngineError(format!("unknown ability {id}")))?;
        if crate::revision50::enabled(r) {
            crate::revision50_ability_energy::acquire_for_revision(&mut r.player, &[id.into()], r.simulation_revision.value_or(29.))
                .map_err(|e| crate::engine::EngineError(format!("Blessing acquisition: {e:?}")))?;
        }
        r.player
            .equipment_abilities
            .get_or_insert_with(Vec::new)
            .retain(|s| s != id);
        r.player
            .conditions
            .shift_remove(&format!("remembered:{id}"));
        if !r.player.abilities.iter().any(|s| s == id) {
            r.player.abilities.push(id.into());
        }
        let owned = r.player.abilities.clone();
        let order = r.player.ability_order.get_or_insert_with(|| owned);
        if !order.iter().any(|s| s == id) {
            order.push(id.into());
        }
        initialize_ability_state_for_revision(r.simulation_revision.value_or(29.), &mut r.player, id, r.tick).map_err(crate::engine::EngineError)?;
        crate::engine::log(r, format!("Learned {}.", a.name), "player");
    }
    r.offers.clear();
    crate::equipment::sync_health(r, false);
    level_up(r);
    Ok(true)
}
pub fn starter_abilities(god_id: u32) -> Result<Vec<String>, String> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Pool {
        god_id: u32,
        abilities: Vec<String>,
    }
    static POOLS: std::sync::OnceLock<Vec<Pool>> = std::sync::OnceLock::new();
    let pools = POOLS.get_or_init(|| {
        serde_json::from_str(include_str!("../catalog/starter-pools-v48.json")).unwrap()
    });
    pools
        .iter()
        .find(|p| p.god_id == god_id)
        .map(|p| p.abilities.clone())
        .ok_or_else(|| format!("No starter abilities for god {god_id}."))
}
/// O(P * L), O(P * L) returned strings for P starter entries of ID length L.
/// The designer's explicit replacements retain every pool position and weight;
/// no selection draw or new starter is added. Legacy pools remain byte-identical.
pub fn starter_abilities_for_revision(god_id: u32, revision: f64) -> Result<Vec<String>, String> {
    let mut pool = starter_abilities(god_id)?;
    if revision >= 50.0 {
        for id in &mut pool {
            let replacement = match id.as_str() {
                "shrail-unyielding-assault" => Some("shrail-execute"),
                "pelian-never-without-a-fight" => Some("pelian-fated-duel"),
                "kative-dilute-potion" => Some("kative-bottomless-pack"),
                _ => None,
            };
            if let Some(replacement) = replacement { *id = replacement.into(); }
        }
    }
    Ok(pool)
}
/// Exact one-time acquisition metadata; catalog initialization is distinct from execution support.
pub fn initialize_ability_state(actor: &mut Actor, id: &str, tick: f64) -> Result<(), String> {
    initialize_ability_state_for_revision(29., actor, id, tick)
}
pub fn initialize_ability_state_for_revision(revision: f64, actor: &mut Actor, id: &str, tick: f64) -> Result<(), String> {
    let a = definition_for_revision(revision, id).or_else(|| definition(id));
    let owned = actor.abilities.clone();
    let order = actor.ability_order.get_or_insert_with(|| owned);
    if !order.iter().any(|s| s == id) {
        order.push(id.into());
    }
    let key = format!("acquired:{id}");
    if actor.conditions.contains_key(&key) {
        return Ok(());
    }
    actor.conditions.insert(key, tick);
    match a.map(|a| a.name.as_str()).unwrap_or("") {
        "Fortune Teller" => {
            actor.conditions.entry("fortunes".into()).or_insert(3.0);
        }
        "Waking Nightmare" | "Waking Nightmares" => {
            actor
                .conditions
                .entry("nightmareAt".into())
                .or_insert(tick + 100.0);
        }
        "Missed Me" | "Are You Sure About That?" | "Quick Hand" => {
            let max = crate::engine::max_reactions_for_revision(revision, actor);
            actor.reactions = (actor.reactions.value_or(0.0) + 1.0).min(max).into();
        }
        _ => {}
    }
    Ok(())
}
