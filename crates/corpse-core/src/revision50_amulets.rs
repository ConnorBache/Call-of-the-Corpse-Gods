//! Revision 50 amulet energy/action accounting. Effect admission is caller-owned.
//! No cooldowns, passive recharge, level refill, effect defaults or guessed costs.
use serde::{Deserialize, Serialize};
use crate::{Field, Item, Run};
pub const BASE_ENERGY: f64 = 50.0;
fn zero(n: &u64) -> bool { *n == 0 }

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AmuletSpec {
    pub capacity: f64,
    pub activation_cost: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmuletState {
    pub instance_id: u64,
    pub energy: f64,
    /// Read-only draft compatibility. Never blocks activation or changes gameplay.
    #[serde(default, skip_serializing_if = "zero")]
    pub ready_at_half_tick: u64,
}
/// Shared by every amulet/target of one resource owner; captured with Actor state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivationLedger {
    pub last_free_normal_tick: Option<u64>,
    pub last_activation_normal_tick: Option<u64>,
    pub commit_sequence: u64,
}
/// Shared player-owner damage remainder, never a per-ring or per-turn credit.
/// Whole energy awards are spent immediately, including when every amulet is full.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReapingLedger { pub damage_carry: f64 }
fn valid_reaping_ledger(ledger: &ReapingLedger) -> bool {
    ledger.damage_carry.is_finite() && ledger.damage_carry >= 0.0 && ledger.damage_carry < 20.0
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Blessings { pub appropriate: bool, pub spellslinger: bool }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionError {
    MissingSpec, InvalidSpec, InvalidState, MissingItem, NotAmulet,
    InsufficientPlayerEnergy, TickOverflow, StalePlan, AlreadyInitialized,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ActivationPlan {
    spec: AmuletSpec,
    before: AmuletState,
    ledger_before: ActivationLedger,
    player_before: f64,
    tick: u64,
    blessings: Blessings,
    after: AmuletState,
    ledger_after: ActivationLedger,
    player_after: f64,
    consumes_action: bool,
    missing_state: bool,
}
impl ActivationPlan {
    pub fn consumes_action(&self) -> bool { self.consumes_action }
    pub fn amulet_energy_spent(&self) -> f64 { self.before.energy - self.after.energy }
    pub fn player_energy_spent(&self) -> f64 { self.player_before - self.player_after }
}
fn valid_ledger(ledger: &ActivationLedger, tick: u64) -> bool {
    ledger.last_free_normal_tick.is_none_or(|t| t <= tick)
        && ledger.last_activation_normal_tick.is_none_or(|t| t <= tick)
        && ledger.last_free_normal_tick.is_none_or(|t| ledger.last_activation_normal_tick.is_some_and(|a|t <= a))
}
/// O(1) read-only plan. Old draft cooldown deadlines are deliberately ignored.
pub fn plan_activation(spec: Option<&AmuletSpec>, state: &AmuletState, ledger: &ActivationLedger,
    player_energy: f64, normal_tick: u64, blessings: Blessings) -> Result<ActivationPlan, AdmissionError> {
    use AdmissionError::*;
    let spec = *spec.ok_or(MissingSpec)?;
    if spec.capacity != BASE_ENERGY || !spec.activation_cost.is_finite() || spec.activation_cost < 0.0 { return Err(InvalidSpec); }
    if !state.energy.is_finite() || state.energy < 0.0 || state.energy > spec.capacity
        || !player_energy.is_finite() || player_energy < 0.0 || !valid_ledger(ledger,normal_tick) { return Err(InvalidState); }
    let cost = (spec.activation_cost - if blessings.appropriate {10.0} else {0.0}).max(0.0);
    let mut after = *state;
    let mut player_after = player_energy;
    if cost == 0.0 {}
    else if state.energy >= cost {
        after.energy -= cost;
        if after.energy == state.energy { return Err(InvalidState); }
    } else {
        let remainder = cost - state.energy;
        if player_energy < remainder { return Err(InsufficientPlayerEnergy); }
        player_after -= remainder;
        if player_after == player_energy { return Err(InvalidState); }
        after.energy = 0.0;
    }
    let consumes_action = !blessings.spellslinger || ledger.last_free_normal_tick == Some(normal_tick);
    let mut ledger_after = *ledger;
    if !consumes_action { ledger_after.last_free_normal_tick = Some(normal_tick); }
    ledger_after.last_activation_normal_tick = Some(normal_tick);
    ledger_after.commit_sequence = ledger.commit_sequence.checked_add(1).ok_or(TickOverflow)?;
    Ok(ActivationPlan { spec, before:*state, ledger_before:*ledger, player_before:player_energy,
        tick:normal_tick, blessings, after, ledger_after, player_after, consumes_action, missing_state:false })
}
/// O(1); rechecks every resource/ledger/spec input before either write.
pub fn commit_activation(plan:&ActivationPlan,spec:Option<&AmuletSpec>,state:&mut AmuletState,
    ledger:&mut ActivationLedger,player_energy:&mut f64,normal_tick:u64,blessings:Blessings)->Result<(),AdmissionError>{
    if plan_activation(spec,state,ledger,*player_energy,normal_tick,blessings)? != *plan {return Err(AdmissionError::StalePlan);}
    *state=plan.after;*ledger=plan.ledger_after;*player_energy=plan.player_after;Ok(())
}
fn new_state(item:&Item)->Result<AmuletState,AdmissionError>{
    if item.kind!="amulet" || !item.id.is_finite() || item.id<0.0 || item.id.fract()!=0.0 || item.id>9007199254740991.0 {
        return Err(AdmissionError::InvalidState);
    }
    Ok(AmuletState {instance_id:item.id as u64,energy:BASE_ENERGY,ready_at_half_tick:0})
}
/// One-time acquisition/materialization, never an equip, rest or level refill.
pub fn initialize_item(item:&mut Item)->Result<(),AdmissionError>{
    if !item.amulet_state.is_missing(){return Err(AdmissionError::AlreadyInitialized);}
    item.amulet_state=new_state(item)?.into();Ok(())
}
pub fn valid_item_state(item:&Item,revision:f64)->bool{
    match &item.amulet_state {
        Field::Missing=>true,Field::Null=>false,
        Field::Value(s)=> revision>=50.0 && new_state(item).is_ok_and(|base|base.instance_id==s.instance_id)
            && s.energy.is_finite() && s.energy>=0.0 && s.energy<=BASE_ENERGY,
    }
}
fn normal_tick(r:&Run)->Result<u64,AdmissionError>{
    if !r.tick.is_finite() || r.tick<0.0 || r.tick.fract()!=0.0 || r.tick>9007199254740991.0 {return Err(AdmissionError::InvalidState);}
    Ok(r.tick as u64)
}
fn blessings(r:&Run)->Blessings{
    Blessings{appropriate:r.player.abilities.iter().any(|id|id=="kative-appropriate"),
        spellslinger:r.player.abilities.iter().any(|id|id=="isheilah-spellslinger")}
}
/// O(I+B), read-only. Missing state is projected as authored initial50; commit
/// materializes the spent state only after target/effect admission succeeds.
pub fn plan_item_activation(r:&Run,id:f64,spec:Option<&AmuletSpec>)->Result<ActivationPlan,AdmissionError>{
    if !crate::revision50::enabled(r){return Err(AdmissionError::InvalidState);}
    let item=r.inventory.iter().find(|i|i.id==id).ok_or(AdmissionError::MissingItem)?;
    if item.kind!="amulet" {return Err(AdmissionError::NotAmulet);}
    if !valid_item_state(item,50.0){return Err(AdmissionError::InvalidState);}
    let state=item.amulet_state.as_ref().copied().unwrap_or(new_state(item)?);
    let mut plan=plan_activation(spec,&state,&r.player.amulet_ledger.as_ref().copied().unwrap_or_default(),
        r.player.energy,normal_tick(r)?,blessings(r))?;
    plan.missing_state=item.amulet_state.is_missing();Ok(plan)
}
/// O(I+B) revalidation then existing resource-history and shared-owner sync costs.
/// Caller must combine this with the admitted effect in its atomic command.
pub fn commit_item_activation(r:&mut Run,id:f64,plan:&ActivationPlan,spec:Option<&AmuletSpec>)->Result<(),AdmissionError>{
    if plan_item_activation(r,id,spec)? != *plan {return Err(AdmissionError::StalePlan);}
    let index=r.inventory.iter().position(|i|i.id==id).ok_or(AdmissionError::MissingItem)?;
    let loss=plan.player_energy_spent();
    if loss>0.0 {crate::resources::record_player_energy_loss(r,loss);}
    r.inventory[index].amulet_state=plan.after.into();
    r.player.amulet_ledger=plan.ledger_after.into();r.player.energy=plan.player_after;
    crate::half_here::sync(r);Ok(())
}
/// O(I+A+G+J), O(1) scratch, plus bounded actor-ledger checks. Rejects malformed
/// explicit state before commands; old amulet fields remain absent in old saves.
pub fn validate_run(r:&Run)->Result<(),String>{
    let revision=r.simulation_revision.value_or(29.0);
    let valid_actor=|a:&crate::Actor| a.copied_inventory.as_ref().is_none_or(|items|items.iter().all(|i|valid_item_state(i,revision)))
        && match &a.amulet_ledger {Field::Missing=>true,Field::Null=>false,Field::Value(l)=>revision>=50.0 && normal_tick(r).is_ok_and(|t|valid_ledger(l,t))}
        && match &a.reaping_ledger {Field::Missing=>true,Field::Null=>false,Field::Value(l)=>revision>=50.0 && valid_reaping_ledger(l)};
    if !r.inventory.iter().all(|i|valid_item_state(i,revision)) || !valid_actor(&r.player)
        || !r.actors.values().all(|actors|actors.iter().all(valid_actor))
        || !r.ground.values().all(|items|items.iter().all(|g|valid_item_state(&g.item,revision))) {
        return Err("Invalid Revision 50 amulet energy state, activation ledger or Reaping ledger".into());
    }
    Ok(())
}

/// Explicit immediate impact only. Callers supply the pre-impact source/target
/// and gross HP removed, capped at the target owner's positive pre-hit HP, before
/// rescue. Source snapshots keep one multi-target impact independent of deaths.
/// O(I+C) time, O(1) scratch: inventory scans plus ownership/hostility conditions.
/// No per-20 loop, RNG, turn cap, player energy, ability reserve or passive refill.
pub fn reap_direct_hit(r:&mut Run,attacker:&crate::Actor,target:&crate::Actor,actual_hp_damage:f64) {
    if !crate::revision50::enabled(r) || !actual_hp_damage.is_finite() || actual_hp_damage<=0.0
        || attacker.dead.value_or(false) || attacker.hp<=0.0 || target.dead.value_or(false) || target.hp<=0.0
        || crate::half_here::owner_id(r,attacker)!=r.player.id
        || crate::half_here::owner_id(r,target)==r.player.id
        || attacker.conditions.get("miniMe")==Some(&1.0)
        || crate::revision50_terrain::is_soldier(attacker)
        || crate::revision50_nightmares::is_decoy(attacker)
        || crate::revision50_terrain::is_soldier(target)
        || crate::revision50_nightmares::is_decoy(target)
        || !crate::revision50_control_amulets::hostile(r,attacker,target)
        || (target.state=="neutral" && !crate::revision50_control_amulets::discord_hostility(r,attacker,target))
        || !r.inventory.iter().any(|i|crate::revision50_inventory::worn(i)
            && crate::revision50_rings::effect(i)==Some(crate::revision50_rings::REAPING)) {
        return;
    }
    let mut ledger=match &r.player.reaping_ledger {
        Field::Missing=>ReapingLedger::default(),Field::Value(ledger)=>*ledger,Field::Null=>return,
    };
    // Invalid saved state is rejected by admission, never repaired into a refill.
    if !valid_reaping_ledger(&ledger) {return;}
    // Split before addition to avoid overflow on an otherwise finite HP packet.
    let remainder=ledger.damage_carry+actual_hp_damage%20.0;
    let award=(actual_hp_damage/20.0).floor()+(remainder/20.0).floor();
    ledger.damage_carry=remainder%20.0;
    r.player.reaping_ledger=ledger.into();
    if award<=0.0 {return;}
    for item in &mut r.inventory {
        if !crate::revision50_amulet_effects::is_amulet(item) || !valid_item_state(item,50.0) {continue;}
        // Missing state already means the initial full 50, not a spent reserve.
        if let Some(state)=item.amulet_state.as_mut() {
            state.energy=(state.energy+award.min(BASE_ENERGY)).min(BASE_ENERGY);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn spec()->AmuletSpec{AmuletSpec{capacity:BASE_ENERGY,activation_cost:20.0}}
    fn state(energy:f64)->AmuletState{AmuletState{instance_id:1,energy,ready_at_half_tick:0}}
    #[test]
    fn own_energy_then_exhausted_player_fallback_without_cooldowns(){
        let a=spec();let l=ActivationLedger::default();let b=Blessings::default();
        let mut s=state(50.0);s.ready_at_half_tick=u64::MAX;
        let p=plan_activation(Some(&a),&s,&l,7.0,0,b).unwrap();
        assert_eq!((p.amulet_energy_spent(),p.player_energy_spent()),(20.0,0.0));
        assert!(plan_activation(Some(&a),&state(0.0),&l,20.0,0,b).is_ok());
        let split=plan_activation(Some(&a),&state(5.0),&l,30.0,0,b).unwrap();
        assert_eq!((split.amulet_energy_spent(),split.player_energy_spent()),(5.0,15.0));
        assert_eq!(plan_activation(Some(&a),&state(5.0),&l,14.0,0,b),Err(AdmissionError::InsufficientPlayerEnergy));
    }
    #[test]
    fn appropriate_reduces_cost_ten_and_can_make_zero_cost(){
        let b=Blessings{appropriate:true,spellslinger:false};let l=ActivationLedger::default();
        let p=plan_activation(Some(&spec()),&state(50.0),&l,0.0,0,b).unwrap();assert_eq!(p.amulet_energy_spent(),10.0);
        let free=AmuletSpec{capacity:BASE_ENERGY,activation_cost:5.0};
        assert_eq!(plan_activation(Some(&free),&state(0.0),&l,0.0,0,b).unwrap().player_energy_spent(),0.0);
    }
    #[test]
    fn global_free_credit_and_stale_commit_remain_atomic(){
        let a=spec();let mut s=state(50.0);let mut l=ActivationLedger::default();let mut e=10.0;
        let b=Blessings{appropriate:false,spellslinger:true};let p=plan_activation(Some(&a),&s,&l,e,7,b).unwrap();
        assert!(!p.consumes_action());commit_activation(&p,Some(&a),&mut s,&mut l,&mut e,7,b).unwrap();
        assert!(plan_activation(Some(&a),&s,&l,e,7,b).unwrap().consumes_action());
        let before=(s,l,e);assert!(commit_activation(&p,Some(&a),&mut s,&mut l,&mut e,7,b).is_err());assert_eq!((s,l,e),before);
    }
    #[test]
    fn initialization_is_once_only_and_unknown_or_invalid_specs_fail(){
        let mut i=Item{id:1.0,kind:"amulet".into(),..Default::default()};initialize_item(&mut i).unwrap();
        i.amulet_state.as_mut().unwrap().energy=2.0;assert_eq!(initialize_item(&mut i),Err(AdmissionError::AlreadyInitialized));
        assert_eq!(i.amulet_state.as_ref().unwrap().energy,2.0);
        let l=ActivationLedger::default();assert_eq!(plan_activation(None,&state(50.0),&l,0.0,0,Blessings::default()),Err(AdmissionError::MissingSpec));
        let bad=AmuletSpec{capacity:12.0,activation_cost:1.0};assert_eq!(plan_activation(Some(&bad),&state(12.0),&l,0.0,0,Blessings::default()),Err(AdmissionError::InvalidSpec));
    }
}
