//! Authored two-draw outside-god blessing selection. O(A + I + G) scans.
use crate::{
    Item, Run,
    engine::{EngineError, log},
    progression::{self, Ability},
};
pub fn target_eligible(i: &Item) -> bool {
    crate::equipment::equippable(i)
        && !i.tinkered.value_or(false)
        && !i.granted_ability.as_ref().is_some_and(|s| !s.is_empty())
}
pub fn target_eligible_for_revision(i: &Item, revision: f64) -> bool {
    if revision >= 50.0 {
        matches!(i.kind.as_str(), "ring" | "amulet") && !i.tinkered.value_or(false)
    } else { target_eligible(i) }
}
/// Stable primary-then-additional grants; no duplicate grants are introduced.
pub fn grants(i: &Item) -> impl Iterator<Item = &String> {
    i.granted_ability.as_ref().into_iter().filter(|s| !s.is_empty())
        .chain(i.additional_granted_abilities.iter().filter(|s| !s.is_empty()))
}
/// Query-only canonical identity for generation-history exclusion. This never
/// changes learned powers, item grants, provenance, weights or random draws.
/// Warm O(A*L) for A=72 mapping rows and maximum ID length L; first use parses
/// O(mapping bytes) into one retained table. Unknown IDs have no replacement.
fn revision50_history_identity(id: &str) -> Option<&'static str> {
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Identity { legacy_id: String, revision50_id: String }
    static IDENTITIES: std::sync::OnceLock<Vec<Identity>> = std::sync::OnceLock::new();
    IDENTITIES.get_or_init(|| {
        serde_json::from_str(include_str!("../catalog/ability-identities-v50.json"))
            .expect("authored revision 50 identities")
    }).iter().find(|row| row.legacy_id == id).map(|row| row.revision50_id.as_str())
}
pub fn god_pools(r: &Run) -> Vec<Vec<&'static Ability>> {
    let mut excluded: std::collections::HashSet<String> = r
        .generated_ability_grants
        .value_or_default()
        .into_iter()
        .collect();
    if crate::revision50::enabled(r) {
        // Historical generation can name the replaced blessing. Exclude its
        // authored current identity as well, without rewriting that history or
        // treating a stale active item grant as a migration request.
        // O(H*A*L) mapping work for H historical entries, plus set insertion.
        for id in r.generated_ability_grants.as_ref().into_iter().flatten() {
            if let Some(current) = revision50_history_identity(id) {
                excluded.insert(current.into());
            }
        }
    }
    for i in r
        .inventory
        .iter()
        .chain(r.ground.values().flat_map(|v| v.iter().map(|g| &g.item)))
    {
        for id in grants(i) { excluded.insert(id.clone()); }
    }
    let mut pools = Vec::new();
    for god in 0..12 {
        if r.gods.contains(&(god as f64)) {
            continue;
        }
        let pool: Vec<_> = progression::catalog_for_revision(r.simulation_revision.value_or(29.0))
            .iter()
            .filter(|a| a.god_id == god as f64 && !excluded.contains(&a.id))
            .collect();
        if !pool.is_empty() {
            pools.push(pool)
        }
    }
    pools
}
pub fn name_equipment(i: &mut Item, perk: &str) {
    if i.kind == "armor" && i.name.starts_with("Plate armor") {
        let tail = &i.name[11..];
        if tail.is_empty()
            || tail.starts_with(" of ")
            || tail == ", Cursed"
            || tail.starts_with(", Cursed ")
        {
            i.name = format!("Plate Armor{tail}");
        }
    }
    if i.name.ends_with(&format!(" of {perk}")) || i.name.ends_with(&format!(" and {perk}")) {
        return;
    }
    i.name = format!(
        "{}{}{perk}",
        i.name,
        if i.name.contains(" of ") {
            " and "
        } else {
            " of "
        }
    );
}
pub fn activate(r: &mut Run, target: Option<f64>) -> Result<bool, EngineError> {
    let Some(index) = r
        .inventory
        .iter()
        .position(|i| Some(i.id) == target && target_eligible_for_revision(i, r.simulation_revision.value_or(29.0)))
    else {
        log(
            r,
            if r.simulation_revision.value_or(29.0) >= 50.0 {
                "Select a ring or amulet in your inventory. Each item can be tinkered once; its existing powers are preserved."
            } else { "Select an unblessed weapon, armor, ring or amulet in your inventory. Each item can be tinkered once." },
            "player",
        );
        return Ok(false);
    };
    let pools = god_pools(r);
    if pools.is_empty() {
        log(
            r,
            "No eligible outside-god abilities remain for Tinker.",
            "player",
        );
        return Ok(false);
    }
    if crate::abilities::energy_activation_silenced(r, &r.player) {
        log(r, "Silence prevents this energy-paid activation.", "player");
        return Ok(false);
    }
    if !crate::revision50_ability_energy::can_pay(r, r.player.id, "kative-tinker", 100.0) {
        log(r, "Not enough energy.", "player");
        return Ok(false);
    }
    let mut state = crate::rng::to_u32(r.rng);
    let g = crate::rng::next_u32(&mut state) as f64 / 4294967296.0;
    let pool = &pools[(g * pools.len() as f64).floor() as usize];
    let n = crate::rng::next_u32(&mut state) as f64 / 4294967296.0;
    let ability = pool[(n * pool.len() as f64).floor() as usize];
    if !crate::abilities::supported(&ability.id)
        && !matches!(ability.id.as_str(), "numo-revert" | "morae-see-the-future")
    {
        return Err(EngineError(format!(
            "Tinker selected unsupported ability {}",
            ability.id
        )));
    }
    if crate::revision50::enabled(r) {
        crate::revision50_ability_energy::validate_reserves(&r.player)
            .map_err(|e| EngineError(format!("Invalid blessing reserve: {e:?}")))?;
    }
    if !crate::revision50_ability_energy::spend(r, r.player.id, "kative-tinker", 100.0) {
        return Ok(false);
    }
    if crate::revision50::enabled(r) {
        crate::revision50_ability_energy::acquire_for_revision(&mut r.player, &[ability.id.clone()], r.simulation_revision.value_or(29.))
            .map_err(|e| EngineError(format!("Blessing acquisition: {e:?}")))?;
    }
    let before = crate::equipment::capture_before(r);
    r.rng = state as f64;
    let item = &mut r.inventory[index];
    if item.granted_ability.as_ref().is_some_and(|id| !id.is_empty()) {
        item.additional_granted_abilities.push(ability.id.clone());
    } else {
        item.granted_ability = ability.id.clone().into();
    }
    item.tinkered = true.into();
    name_equipment(item, &ability.name);
    crate::equipment::refresh_known_magic(item);
    let name = if crate::revision52_amulet_tiers::enabled(r) {crate::revision50_item_knowledge::display_name(r,&r.inventory[index])} else {r.inventory[index].name.clone()};
    r.generated_ability_grants
        .get_or_insert_with(Vec::new)
        .push(ability.id.clone());
    crate::equipment::settle_removed(r, before)?;
    let success = if r.simulation_revision.value_or(29.) >= 43. {
        format!("{name} is blessed with {}.", ability.name)
    } else {
        format!("{name} is blessed with {}. It grants this ability while equipped.", ability.name)
    };
    log(
        r,
        success,
        "player",
    );
    crate::events::inventory(r,"P65","gear",target);
    crate::events::inventory(r,"P65",&format!("spark-god-{}",ability.god_id as u32),target);
    Ok(true)
}

#[cfg(test)]
mod revision50_tests {
    use super::*;
    #[test]
    fn existing_jewelry_powers_do_not_block_tinker_and_legacy_stays_unchanged() {
        let mut item=Item {kind:"ring".into(),granted_ability:"etill-drown".to_string().into(),..Default::default()};
        assert!(target_eligible_for_revision(&item,50.0));
        assert!(!target_eligible_for_revision(&item,49.0));
        item.tinkered=true.into(); assert!(!target_eligible_for_revision(&item,50.0));
        item.tinkered=false.into(); item.kind="weapon".into();
        assert!(!target_eligible_for_revision(&item,50.0));
    }
    #[test]
    fn all_existing_grants_are_excluded_from_generation() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=50.0.into();r.gods.clear();r.inventory.clear();r.ground.clear();
        r.inventory.push(Item {kind:"amulet".into(),granted_ability:"etill-drown".to_string().into(),additional_granted_abilities:vec!["grisha-boundless-hunger".into()],..Default::default()});
        assert_eq!(grants(&r.inventory[0]).cloned().collect::<Vec<_>>(),vec!["etill-drown","grisha-boundless-hunger"]);
        for pool in god_pools(&r) { for a in pool {
            assert_ne!(a.id,"etill-drown");assert_ne!(a.id,"grisha-boundless-hunger");
        }}
    }
    #[test]
    fn tinker_preserves_original_power_and_costs_exactly_one_payment() {
        let mut base:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        base.simulation_revision=50.0.into();base.inventory.clear();base.ground.clear();
        base.player.abilities.clear();base.player.active.clear();base.player.energy=150.0;
        // Explicit test-only exhausted reserve exercises one player-bar payment.
        // This does not supply a production reserve capacity or acquisition rule.
        base.player.ability_reserves = crate::Field::Missing;
        crate::revision50_ability_energy::initialize_reserve(
            &mut base.player, "kative-tinker", 0., 0.).unwrap();
        base.gods=(0..12).filter(|g| *g!=8).map(|g|g as f64).collect();
        base.inventory.push(Item {id:900.0,name:"Ring of Drown".into(),kind:"ring".into(),granted_ability:"etill-drown".to_string().into(),bonus:crate::PartialStats{offense:7.0.into(),..Default::default()}.into(),..Default::default()});
        // Explicit deterministic seeds: unsupported generated powers reject
        // before RNG/payment; the first supported choice exercises completion.
        let mut success=None;
        for seed in 1..=1000 {
            let mut r=base.clone();r.rng=seed as f64;
            if activate(&mut r,Some(900.0)).is_ok_and(|ok|ok) {success=Some(r);break;}
            assert_eq!(r.player.energy,150.0);
        }
        let mut r=success.expect("at least one supported outside-god grant");
        assert_eq!(r.player.energy,50.0);
        let i=&r.inventory[0];
        assert_eq!(i.granted_ability.as_ref().map(String::as_str),Some("etill-drown"));
        assert_eq!(i.additional_granted_abilities.len(),1);
        assert_eq!(i.bonus,base.inventory[0].bonus);
        assert!(i.tinkered.value_or(false));
        let id=i.additional_granted_abilities[0].clone();
        assert!(r.generated_ability_grants.as_ref().unwrap().contains(&id));
        assert!(god_pools(&r).iter().flatten().all(|a|a.id!=id));
        assert!(!activate(&mut r,Some(900.0)).unwrap());
        assert_eq!(r.player.energy,50.0);
    }

}
