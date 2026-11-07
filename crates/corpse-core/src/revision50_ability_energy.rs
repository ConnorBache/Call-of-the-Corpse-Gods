//! Authored Revision 50 blessing reserves and atomic own-first split payment.
use crate::{Actor, Run};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
pub const BASE_ENERGY: f64 = 50.0;
pub const FUTURE_ID: &str = "morae-see-the-future";

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbilityReserve {
    pub capacity: f64,
    pub energy: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnergyError { MissingReserve, InvalidReserve, InvalidCost, InvalidActorEnergy, InsufficientEnergy, StalePlan, AlreadyInitialized }
#[derive(Clone, Debug, PartialEq)]
pub struct PaymentPlan {
    ability_id: String,
    reserve_before: AbilityReserve,
    reserve_after: AbilityReserve,
    actor_before: f64,
    actor_after: f64,
}
impl PaymentPlan {
    pub fn actor_energy_spent(&self) -> f64 { self.actor_before - self.actor_after }
    pub fn reserve_energy_spent(&self) -> f64 { self.reserve_before.energy - self.reserve_after.energy }
}
pub fn valid_reserve(s: &AbilityReserve) -> bool {
    s.capacity.is_finite() && s.capacity >= 0.0 && s.energy.is_finite()
        && s.energy >= 0.0 && s.energy <= s.capacity
}
/// Explicit authored capacity and initial charge. Acquisition policy is not inferred.
pub fn authored_reserve(capacity: f64, initial_energy: f64) -> Result<AbilityReserve, EnergyError> {
    let s = AbilityReserve { capacity, energy: initial_energy };
    if valid_reserve(&s) { Ok(s) } else { Err(EnergyError::InvalidReserve) }
}
/// Explicit acquisition boundary; refuses replacement so toggles/equipment refresh
/// cannot refill an already initialized reserve. Caller supplies authored values.
pub fn initialize_reserve(actor: &mut Actor, ability_id: &str, capacity: f64, initial_energy: f64) -> Result<(), EnergyError> {
    if ability_id.is_empty() { return Err(EnergyError::InvalidReserve); }
    let reserve = authored_reserve(capacity, initial_energy)?;
    if actor.ability_reserves.as_ref().is_some_and(|m| m.contains_key(ability_id)) {
        return Err(EnergyError::AlreadyInitialized);
    }
    actor.ability_reserves.get_or_insert_with(IndexMap::new).insert(ability_id.into(), reserve);
    Ok(())
}
fn valid_acquisition_ids(ids: &[String]) -> bool {
    ids.iter().all(|id| crate::progression::definition_for_revision(50.0, id).is_some()
        || crate::abilities::copied_trait_supported(id))
}
/// One-time acquisition. Existing (including dormant) charge is never replaced.
/// Validate all entries and IDs before inserting any of the new authored reserves.
/// O(B + G*A*max ID bytes) metadata validation, expected O(ID bytes) inserts;
/// O(new ID bytes) retained allocation, A=72 catalog entries and G supplied IDs.
pub fn acquire(actor: &mut Actor, ids: &[String]) -> Result<(), EnergyError> {
    validate_reserves(actor)?;
    if !valid_acquisition_ids(ids) { return Err(EnergyError::InvalidReserve); }
    for id in ids {
        if !actor.ability_reserves.as_ref().is_some_and(|m| m.contains_key(id)) {
            initialize_reserve(actor, id, BASE_ENERGY, BASE_ENERGY)?;
        }
    }
    Ok(())
}
/// Reconciliation can discover a newly carried grant, but never tops up one that
/// was already acquired. Dropped grants remain dormant in the owner's map.
/// Adds O(I+G+ID bytes) enumeration to acquire's metadata validation/lookup;
/// O(G+owned ID bytes) temporary IDs.
pub fn reconcile_acquired(r: &mut Run) -> Result<(), EnergyError> {
    if !crate::revision50::enabled(r) { return Ok(()); }
    let ids: Vec<String> = r.player.abilities.iter().cloned()
        .chain(r.inventory.iter().flat_map(crate::tinker::grants).cloned()).collect();
    acquire(&mut r.player, &ids)
}
/// Acquisition preflight and materialization on a detached item, before pickup.
/// Item identity and existing charge are preserved when dropped and reacquired.
pub fn acquire_item(actor: &mut Actor, item: &mut crate::Item) -> Result<(), String> {
    let ids: Vec<String> = crate::tinker::grants(item).cloned().collect();
    validate_reserves(actor).map_err(|e| format!("Invalid blessing reserve: {e:?}"))?;
    if !valid_acquisition_ids(&ids) { return Err("Unknown Revision 50 blessing grant".into()); }
    if !crate::revision50_amulets::valid_item_state(item, 50.0) {
        return Err("Invalid amulet energy state".into());
    }
    if item.kind == "amulet" && item.amulet_state.is_missing() {
        crate::revision50_amulets::initialize_item(item).map_err(|e| format!("Invalid amulet acquisition: {e:?}"))?;
    }
    acquire(actor, &ids).map_err(|e| format!("Invalid blessing acquisition: {e:?}"))
}
/// Expected O(L) lookup/copy for ability ID bytes L; O(B*L) hash-collision worst
/// case for B reserves. No scan, mutation, regeneration or random draw.
pub fn plan_payment(
    reserves: Option<&IndexMap<String, AbilityReserve>>, ability_id: &str,
    actor_energy: f64, cost: f64,
) -> Result<PaymentPlan, EnergyError> {
    use EnergyError::*;
    if !cost.is_finite() || cost <= 0.0 { return Err(InvalidCost); }
    if !actor_energy.is_finite() || actor_energy < 0.0 { return Err(InvalidActorEnergy); }
    let before = *reserves.and_then(|m| m.get(ability_id)).ok_or(MissingReserve)?;
    if ability_id.is_empty() || !valid_reserve(&before) { return Err(InvalidReserve); }
    let mut after = before;
    let mut actor_after = actor_energy;
    if before.energy >= cost {
        after.energy -= cost;
        if after.energy == before.energy { return Err(InvalidReserve); }
    } else {
        let remainder = cost - before.energy;
        if actor_energy < remainder { return Err(InsufficientEnergy); }
        actor_after -= remainder;
        if actor_after == actor_energy { return Err(InvalidActorEnergy); }
        after.energy = 0.0;
    }
    Ok(PaymentPlan { ability_id: ability_id.into(), reserve_before: before,
        reserve_after: after, actor_before: actor_energy, actor_after })
}
/// See The Future drains exactly its own reserve and player energy, including
/// the valid zero/zero case. Other reserves never enter this payment plan.
pub fn plan_future_payment(actor: &Actor) -> Result<PaymentPlan, EnergyError> {
    if !actor.energy.is_finite() || actor.energy < 0.0 { return Err(EnergyError::InvalidActorEnergy); }
    let before = *actor.ability_reserves.as_ref().and_then(|m| m.get(FUTURE_ID)).ok_or(EnergyError::MissingReserve)?;
    if !valid_reserve(&before) { return Err(EnergyError::InvalidReserve); }
    Ok(PaymentPlan { ability_id: FUTURE_ID.into(), reserve_before: before,
        reserve_after: AbilityReserve { energy: 0.0, ..before },
        actor_before: actor.energy, actor_after: 0.0 })
}
/// Revalidate before either mutation; rejected or stale plans leave both unchanged.
pub fn commit_payment(
    plan: &PaymentPlan, reserves: &mut IndexMap<String, AbilityReserve>, actor_energy: &mut f64,
) -> Result<(), EnergyError> {
    if reserves.get(&plan.ability_id) != Some(&plan.reserve_before) || *actor_energy != plan.actor_before {
        return Err(EnergyError::StalePlan);
    }
    *reserves.get_mut(&plan.ability_id).unwrap() = plan.reserve_after;
    *actor_energy = plan.actor_after;
    Ok(())
}
/// Validate all entries first: a malformed later entry cannot cause partial refill.
/// O(B + L) time for B entries with L total key bytes; O(1) extra memory.
pub fn validate_reserves(actor: &Actor) -> Result<(), EnergyError> {
    if matches!(actor.ability_reserves, crate::Field::Null) { return Err(EnergyError::InvalidReserve); }
    if let Some(reserves) = actor.ability_reserves.as_ref() {
        for (id, reserve) in reserves {
            if id.is_empty() || !valid_reserve(reserve) { return Err(EnergyError::InvalidReserve); }
        }
    }
    Ok(())
}
pub fn refill_on_level_up(actor: &mut Actor) -> Result<(), EnergyError> {
    validate_reserves(actor)?;
    if let Some(reserves) = actor.ability_reserves.as_mut() {
        for reserve in reserves.values_mut() { reserve.capacity = BASE_ENERGY; reserve.energy = BASE_ENERGY; }
    }
    Ok(())
}
/// Read-only admission for gates which formerly compared only player energy.
/// Zero-cost effects do not require an invented reserve; pre-R50 behavior is kept.
pub fn can_pay(r: &Run, endpoint: f64, ability_id: &str, cost: f64) -> bool {
    let Some(body) = crate::engine::actor(r, endpoint) else { return false; };
    let owner = crate::half_here::owner_id(r, body);
    let Some(actor) = crate::engine::actor(r, owner) else { return false; };
    if !crate::revision50::enabled(r) { return actor.energy >= cost; }
    if !cost.is_finite() || cost < 0.0 { return false; }
    cost == 0.0 || plan_payment(actor.ability_reserves.as_ref(), ability_id, actor.energy, cost).is_ok()
}
/// Ability-aware replacement for spend_shared. Silence/target/effect eligibility
/// stays at call sites. Half Here and Dream Walk use the owner's reserve, not copies.
pub fn spend(r: &mut Run, endpoint: f64, ability_id: &str, cost: f64) -> bool {
    if !crate::revision50::enabled(r) { return crate::engine::spend_shared(r, endpoint, cost); }
    if !cost.is_finite() || cost < 0.0 { return false; }
    let Some(body) = crate::engine::actor(r, endpoint) else { return false; };
    let owner = crate::half_here::owner_id(r, body);
    let Some(actor) = crate::engine::actor(r, owner) else { return false; };
    if cost == 0.0 { return true; }
    let Ok(plan) = plan_payment(actor.ability_reserves.as_ref(), ability_id, actor.energy, cost) else { return false; };
    let actor_before = actor.clone();
    let a = crate::engine::actor_mut_trusted(r, owner).unwrap();
    let Some(reserves) = a.ability_reserves.as_mut() else { return false; };
    if commit_payment(&plan, reserves, &mut a.energy).is_err() { return false; }
    if endpoint != owner {
        // Preserve the existing shared-energy presentation. All future deductions
        // still resolve the canonical owner, regardless of stale copied reserves.
        crate::engine::actor_mut_trusted(r, endpoint).unwrap().energy = plan.actor_after;
    }
    if plan.actor_energy_spent() > 0.0 {
        crate::resources::record_loss(r, &actor_before, "energy", plan.actor_energy_spent());
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    fn reserves() -> IndexMap<String, AbilityReserve> {
        IndexMap::from([("test:a".into(), authored_reserve(12., 4.).unwrap()),
            ("test:b".into(), authored_reserve(9., 0.).unwrap())])
    }
    #[test] fn own_first_and_exhausted_fallback() {
        let mut m = reserves(); let mut e = 8.;
        let p = plan_payment(Some(&m), "test:a", e, 4.).unwrap();
        assert_eq!((p.reserve_energy_spent(), p.actor_energy_spent()), (4., 0.));
        commit_payment(&p, &mut m, &mut e).unwrap(); assert_eq!(e, 8.);
        let p = plan_payment(Some(&m), "test:a", e, 3.).unwrap();
        commit_payment(&p, &mut m, &mut e).unwrap(); assert_eq!(e, 5.);
        assert_eq!(m["test:b"].energy, 0.);
    }
    #[test] fn invalid_missing_insufficient_and_stale_leave_state_unchanged() {
        let mut m = reserves(); let mut e = 8.; let before = (m.clone(), e);
        assert_eq!(plan_payment(None, "test:a", e, 1.), Err(EnergyError::MissingReserve));
        assert_eq!(plan_payment(Some(&m), "test:a", e, 13.), Err(EnergyError::InsufficientEnergy));
        assert_eq!(plan_payment(Some(&m), "test:a", e, f64::NAN), Err(EnergyError::InvalidCost));
        assert_eq!(plan_payment(Some(&m), "test:b", 1., 2.), Err(EnergyError::InsufficientEnergy));
        assert_eq!((m.clone(), e), before);
        let p = plan_payment(Some(&m), "test:a", e, 4.).unwrap();
        commit_payment(&p, &mut m, &mut e).unwrap(); let after = (m.clone(), e);
        assert_eq!(commit_payment(&p, &mut m, &mut e), Err(EnergyError::StalePlan));
        assert_eq!((m, e), after);
    }
    #[test] fn level_refill_serializes_and_does_not_refill_player_energy() {
        let mut a = Actor { energy: 7., ability_reserves: reserves().into(), ..Default::default() };
        let saved = serde_json::to_string(&a).unwrap();
        let mut restored: Actor = serde_json::from_str(&saved).unwrap();
        refill_on_level_up(&mut a).unwrap(); refill_on_level_up(&mut restored).unwrap();
        assert_eq!(a, restored); assert_eq!(a.energy, 7.);
        assert_eq!(a.ability_reserves.as_ref().unwrap()["test:a"].energy, 50.);
        assert_eq!(a.ability_reserves.as_ref().unwrap()["test:b"].energy, 50.);
        let rewind: Actor = serde_json::from_str(&saved).unwrap();
        assert_eq!(rewind.ability_reserves.as_ref().unwrap()["test:a"].energy, 4.);
    }
    #[test] fn malformed_later_reserve_prevents_entire_refill() {
        let mut m = reserves(); m["test:b"].energy = -1.;
        let mut a = Actor { energy: 7., ability_reserves: m.into(), ..Default::default() };
        let before = a.clone();
        assert_eq!(refill_on_level_up(&mut a), Err(EnergyError::InvalidReserve));
        assert_eq!(a, before);
    }
    #[test] fn revision_gating_and_real_level_up() {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.player.energy = 8.; r.player.ability_reserves = reserves().into();
        r.simulation_revision = 50.0.into(); r.exp = 2.0f64.powf(r.level); r.offers.clear(); r.gods.clear();
        crate::progression::level_up(&mut r);
        assert_eq!(r.player.energy, 8.);
        assert_eq!(r.player.ability_reserves.as_ref().unwrap()["test:a"].energy, 50.);
        let id = r.player.id;
        assert!(!spend(&mut r, id, "test:missing", 3.)); assert_eq!(r.player.energy, 8.);
        r.simulation_revision = 49.0.into();
        assert!(spend(&mut r, id, "test:missing", 3.)); assert_eq!(r.player.energy, 5.);
    }
}
