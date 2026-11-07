//! Disabled-by-default measurement pilot, NOT persistence or complete mutation capture.
//! Actor marks are mutable-access candidates, not writes. Final values are read only
//! from the committed master; the retained opening Run supplies transient old values.
use crate::{Actor,Run};
use serde::{Serialize,Deserialize};
use std::{cell::RefCell,collections::BTreeSet};
#[derive(Default)]
struct Marks { groups:BTreeSet<String>,gas:BTreeSet<(u64,usize)>,unsupported:BTreeSet<String>,accesses:u64,gas_writes:u64 }
thread_local! { static ACTIVE:RefCell<Option<Marks>>=const {RefCell::new(None)}; }
#[derive(Debug,Default,Serialize)]
/// selected_changes counts changed four-field components, not changed individual fields.
/// Forward payload deliberately repeats unchanged siblings within each changed component.
pub struct Counters {pub actor_mutable_access_attempts:u64,pub gas_write_attempts:u64,pub actor_candidates:usize,pub gas_candidates:usize,pub selected_changes:usize}
#[derive(Debug,Clone,PartialEq,Serialize,Deserialize)]
#[serde(tag="kind")]
pub enum Change {Actor{floor:Option<String>,id_bits:u64,hp_bits:u64,energy_bits:u64,x_bits:u64,y_bits:u64},Gas{depth_bits:u64,index:usize,value:serde_json::Value}}
#[derive(Debug,Serialize)]
pub struct Report {pub coverage:Vec<&'static str>,pub counters:Counters,pub unsupported:Vec<String>,pub changes:Vec<Change>}
/// Lexical thread-local collection. Nested scopes are explicitly unsupported.
pub struct Scope {parent:Option<Marks>,_thread:std::marker::PhantomData<std::rc::Rc<()>>}
impl Scope {
    /// O(1), begins one command; player is always a candidate at collection.
    pub fn begin()->Self {let parent=ACTIVE.with(|a|a.borrow_mut().replace(Marks::default()));if parent.is_some(){mark_unsupported("nested_pilot_scope");}Self{parent,_thread:std::marker::PhantomData}}
    /// O(sum marked group sizes + gas candidates * gas-address lookup + output payload).
    /// Actor map lookup expected O(1); gas helper verifies direct depth-index lookup.
    /// No all-world scan; unsupported labels add O(log B + label bytes).
    /// Source snapshots are supplied by caller; this does not clone or mutate a Run.
    pub fn finish(self,opening:&Run,committed:&Run)->Report {
        let marks=ACTIVE.with(|a|a.borrow_mut().take()).unwrap_or_default();
        let mut unsupported=marks.unsupported;let mut changes=Vec::new();let mut actors=1;
        if opening.player.id.to_bits()!=committed.player.id.to_bits(){unsupported.insert("player_identity_changed".into());}
        compare_actor(None,&opening.player,&committed.player,&mut changes);
        if opening.depth.to_bits()!=committed.depth.to_bits(){unsupported.insert("active_depth_changed".into());}
        if opening.dreamwalking.as_ref().is_some()||committed.dreamwalking.as_ref().is_some(){unsupported.insert("dream_plane_present".into());}
        for key in &marks.groups {
            let old=opening.actors.get(key);let new=committed.actors.get(key);
            match(old,new){(Some(a),Some(b))=>{actors+=b.len();if a.len()!=b.len()||a.iter().zip(b.iter()).any(|(a,b)|a.id.to_bits()!=b.id.to_bits()){unsupported.insert(format!("actor_membership_or_order:{key}"));continue;}for(a,b)in a.iter().zip(b.iter()){compare_actor(Some(key.as_str()),a,b,&mut changes);}},_=>{unsupported.insert(format!("actor_group_presence:{key}"));}}
        }
        for &(depth_bits,index) in &marks.gas {
            match(crate::environment::capture_pilot_gas_borrowed(opening,depth_bits,index),crate::environment::capture_pilot_gas_borrowed(committed,depth_bits,index)){
                (Ok(old),Ok(value))=>if old!=value{changes.push(Change::Gas{depth_bits,index,value:value.compact()});},
                (a,b)=>{unsupported.insert(format!("gas_address_or_shape:{depth_bits}:{index}:{:?}:{:?}",a.err(),b.err()));}
            }
        }
        let counters=Counters{actor_mutable_access_attempts:marks.accesses,gas_write_attempts:marks.gas_writes,actor_candidates:actors,gas_candidates:marks.gas.len(),selected_changes:changes.len()};
        Report{coverage:vec!["actor hp/energy/x/y only; group access candidates, not write counts","gasType/gasVolume/gasOwner/gasPlayer only; channels/provenance/fire/RNG excluded"],counters,unsupported:unsupported.into_iter().collect(),changes}
    }
}
impl Drop for Scope {
    /// O(marked candidates) destruction; parent restored even during unwind.
    fn drop(&mut self){let mut parent=self.parent.take();if let Some(p)=parent.as_mut(){p.unsupported.insert("nested_pilot_scope".into());}ACTIVE.with(|a|*a.borrow_mut()=parent);}
}
/// O(1), permits bulk producer hooks to skip their extra candidate walk when disabled.
pub fn enabled()->bool{ACTIVE.with(|a|a.borrow().is_some())}
/// O(log marked groups + key bytes) enabled; O(1) disabled. Counts access, NOT write.
pub fn mark_actor_group(key:&str){ACTIVE.with(|a|if let Some(m)=a.borrow_mut().as_mut(){m.accesses+=1;m.groups.insert(key.to_owned());});}
/// O(1); player access is counted separately from its once-per-command candidate.
pub fn mark_player_access(){ACTIVE.with(|a|if let Some(m)=a.borrow_mut().as_mut(){m.accesses+=1;});}
/// O(log marked gas cells) enabled; O(1) disabled. Call only at instrumented writes.
pub fn mark_gas(depth_bits:u64,index:usize){mark_gas_write(depth_bits,index,true);}
/// O(log marked cells) for changed writes, O(1) unchanged/disabled; W counts all calls.
pub fn mark_gas_write(depth_bits:u64,index:usize,changed:bool){ACTIVE.with(|a|if let Some(m)=a.borrow_mut().as_mut(){m.gas_writes+=1;if changed{m.gas.insert((depth_bits,index));}});}

/// O(log flags + site bytes) enabled; O(1) disabled; never changes gameplay.
pub fn mark_unsupported(site:&str){ACTIVE.with(|a|if let Some(m)=a.borrow_mut().as_mut(){m.unsupported.insert(site.to_owned());});}
/// O(log flags); caller may label Revert/Future/other boundaries; no automatic command inference.
pub fn note_boundary(site:&str){mark_unsupported(site);}
/// O(1) selected scalar reads, O(floor-key bytes) only if a change is emitted.
fn compare_actor(floor:Option<&str>,old:&Actor,new:&Actor,out:&mut Vec<Change>){if [old.hp.to_bits(),old.energy.to_bits(),old.x.to_bits(),old.y.to_bits()]!=[new.hp.to_bits(),new.energy.to_bits(),new.x.to_bits(),new.y.to_bits()]{out.push(Change::Actor{floor:floor.map(str::to_owned),id_bits:new.id.to_bits(),hp_bits:new.hp.to_bits(),energy_bits:new.energy.to_bits(),x_bits:new.x.to_bits(),y_bits:new.y.to_bits()});}}
/// O(F log F + all actors + gas cells * gas-address lookup): independent oracle,
/// OUTSIDE timed collection; ordered floor-key construction retains O(F) keys.
/// Structural actors are unsupported; this oracle reports existing stable ordered actors.
pub fn selected_oracle(opening:&Run,committed:&Run)->Vec<Change>{let mut out=Vec::new();compare_actor(None,&opening.player,&committed.player,&mut out);let keys:BTreeSet<_>=opening.actors.keys().chain(committed.actors.keys()).cloned().collect();for key in keys{if let(Some(a),Some(b))=(opening.actors.get(&key),committed.actors.get(&key)){if a.len()==b.len()&&a.iter().zip(b.iter()).all(|(a,b)|a.id.to_bits()==b.id.to_bits()){for(a,b)in a.iter().zip(b.iter()){compare_actor(Some(key.as_str()),a,b,&mut out);}}}}out.extend(crate::environment::capture_pilot_gas_oracle(opening,committed));out}

#[cfg(test)]
mod tests {
    use super::*;
    /// O(A), tiny stable actor fixture without gameplay admission claims.
    fn fixture()->Run {let mut r=Run::default();r.player.id=0.;r.depth=1.;let mut a=Actor::default();a.id=1.;a.hp=10.;r.actors.insert("1".into(),std::sync::Arc::new(vec![a]));r}
    /// O(A), repeated access and restored writes produce no forward changes.
    #[test]fn no_op_repeated_and_cancellation(){let old=fixture();let mut new=old.clone();let s=Scope::begin();for _ in 0..3{new.actors.trusted_slice_mut("1").unwrap()[0].hp=9.;}new.actors.trusted_slice_mut("1").unwrap()[0].hp=10.;let r=s.finish(&old,&new);assert_eq!(r.counters.actor_mutable_access_attempts,4);assert_eq!(r.counters.actor_candidates,2);assert!(r.changes.is_empty());assert!(r.unsupported.is_empty());}
    /// O(A), scratch writes never become persisted values; final master alone decides.
    #[test]fn discarded_scratch_not_collected(){let old=fixture();let final_run=old.clone();let s=Scope::begin();let mut scratch=old.clone();scratch.actors.trusted_slice_mut("1").unwrap()[0].hp=-99.;drop(scratch);let r=s.finish(&old,&final_run);assert!(r.changes.is_empty());assert_eq!(r.counters.actor_candidates,2);}
    /// O(A), exact bit-pattern final scalars and matching independent selected oracle.
    #[test]fn final_master_values_only(){let old=fixture();let mut new=old.clone();let s=Scope::begin();new.actors.trusted_slice_mut("1").unwrap()[0].hp=8.;new.player.energy=-0.;let r=s.finish(&old,&new);assert_eq!(r.changes,selected_oracle(&old,&new));assert_eq!(r.changes.len(),2);}
    /// O(A), unrestricted/nested boundaries stay explicit even when final values match.
    #[test]fn boundaries_and_nested_discard(){let old=fixture();let mut new=old.clone();let s=Scope::begin();{let _nested=Scope::begin();mark_unsupported("discarded_proxy");}new.actors_at_mut(1.).push(Actor::default());let r=s.finish(&old,&new);assert!(r.unsupported.iter().any(|x|x=="nested_pilot_scope"));assert!(r.unsupported.iter().any(|x|x.starts_with("actor_membership_or_order")));assert!(r.unsupported.iter().any(|x|x=="unrestricted_actor_map"));}
    /// O(A), demonstrates a deliberate uninstrumented write is found by the oracle, not hidden.
    #[test]fn raw_unmarked_write_is_oracle_miss(){let old=fixture();let mut new=old.clone();let s=Scope::begin();let mut group=(*new.actors.get("1").unwrap()).as_ref().clone();group[0].hp=3.;drop(s); // Manufacture raw committed state outside hook scope, as public fields permit.
        new.actors.insert("1".into(),std::sync::Arc::new(group));let p=Scope::begin().finish(&old,&new);assert!(p.changes.is_empty());assert_eq!(selected_oracle(&old,&new).len(),1);}
    /// O(1), disabled hooks carry no observations into a later command.
    #[test]fn disabled_hooks_do_not_leak(){mark_actor_group("99");mark_unsupported("outside");let r=fixture();let p=Scope::begin().finish(&r,&r);assert_eq!(p.counters.actor_mutable_access_attempts,0);assert!(p.unsupported.is_empty());}
}

/// O(emitted payload bytes), OUTSIDE timing: expand compact gas diagnostics into
/// frozen verbose-oracle semantics. This never reads a Run or discovers changes.
pub fn normalize_changes(changes:&[Change])->Result<Vec<Change>,String>{changes.iter().map(|c|Ok(match c{Change::Gas{depth_bits,index,value}=>Change::Gas{depth_bits:*depth_bits,index:*index,value:normalize_gas(value)?},other=>other.clone()})).collect()}
/// O(gas-name bytes), strict compact tuple interpretation independent of typed reads.
fn normalize_gas(value:&serde_json::Value)->Result<serde_json::Value,String>{
    use serde_json::json;let a=value.as_array().ok_or("gas tuple must be an array")?;
    match a.first().and_then(|v|v.as_u64()){
        Some(0) if a.len()==1=>Ok(json!({"environment":"missing"})),
        Some(1) if a.len()==1=>Ok(json!({"environment":"null"})),
        Some(2) if a.len()==5=>Ok(json!({"gasType":normalize_slot(&a[1],false)?,"gasVolume":normalize_slot(&a[2],true)?,"gasOwner":normalize_slot(&a[3],true)?,"gasPlayer":normalize_slot(&a[4],true)?})),
        _=>Err("invalid compact gas environment tag/length".into()),
    }
}
/// O(string bytes) for kind, O(1) numeric; distinguish column null from a null kind cell.
fn normalize_slot(value:&serde_json::Value,numeric:bool)->Result<serde_json::Value,String>{
    use serde_json::json;let a=value.as_array().ok_or("gas slot must be an array")?;
    match a.first().and_then(|v|v.as_u64()){
        Some(0) if a.len()==1=>Ok(json!({"presence":"missing"})),
        Some(1) if a.len()==1=>Ok(json!({"presence":"null"})),
        Some(2) if a.len()==2=>if numeric{Ok(json!({"presence":"value","bits":a[1].as_u64().ok_or("numeric slot requires u64 bits")?}))}else if a[1].is_null()||a[1].is_string(){Ok(json!({"presence":"value","value":a[1]}))}else{Err("kind slot requires string or null".into())},
        _=>Err("invalid compact gas slot tag/length".into()),
    }
}
/// O(encoded + emitted payload bytes), observer-only parse/normalization after timed wall.
pub fn normalize_encoded_changes(bytes:&[u8])->Result<Vec<Change>,String>{let changes:Vec<Change>=serde_json::from_slice(bytes).map_err(|e|e.to_string())?;normalize_changes(&changes)}
