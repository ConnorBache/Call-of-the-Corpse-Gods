//! Capture pilot note: disabled hooks O(1); enabled actor-group marks O(log G + key bytes),
//! unrestricted-boundary flags O(log B + label bytes). These are access counts, not writes.
//! Derived creature identity and floor-local slot indexes. Actor vectors retain
//! canonical simulation order; mutable public access invalidates these indexes.
//! Complexity: A=current actors, P=pilots, B=previous indexed actors, F=retained
//! floor maps, V=vacant-slot entries copied after snapshot, H=slot high-water sum.
//! d=decimal digits. Retained memory O((A+P+H)*d+F); snapshots share trie nodes.
use crate::{Actor, creature_trie::{DecimalTrie, SlotMap, SlotHandle}};
use indexmap::IndexMap;
use serde::{Serialize, Deserialize};
use std::{ops::{Deref,DerefMut}, sync::{Arc,Mutex,atomic::{AtomicU64,Ordering}}};
type Map = IndexMap<String, Arc<Vec<Actor>>>;
static EPOCH: AtomicU64 = AtomicU64::new(1);
/// Expected O(1); explicit exhaustion rather than silently reusing a handle epoch.
fn next_epoch()->u64 { EPOCH.try_update(Ordering::Relaxed,Ordering::Relaxed,|v|v.checked_add(1)).expect("Creature handle epoch exhausted") }
/// A process-local handle, never serialized. Reacquire after clone/load/rewind.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct FloorCreatureId { pub epoch:u64, pub slot:u32, pub generation:u64 }
#[derive(Debug,Clone)]
struct Location { floor:usize, map_floor:usize, at:usize, handle:SlotHandle }
#[derive(Debug,Clone,Default)]
struct Registry { reserved:DecimalTrie<()>, positions:DecimalTrie<Location>, entries:Vec<(u64,Location)>, floors:IndexMap<String,SlotMap<u64>>, player:(u64,u64), active_map_floor:Option<usize>, duel_owners:Arc<[f64]> }
#[derive(Debug)]
struct Cache { registry:Arc<Registry>, dirty:bool, error:Option<String>, epoch:u64 }
impl Default for Cache {
    /// O(1) time/space; no trie branches or floor slots allocated.
    fn default()->Self { Self {registry:Arc::new(Registry::default()),dirty:true,error:None,epoch:next_epoch()} }
}
#[derive(Debug,Serialize,Deserialize)]
#[serde(transparent)]
pub struct ActorFloors { map:Map, #[serde(skip)] cache:Mutex<Cache> }
impl Default for ActorFloors {
    /// O(1) time/space.
    fn default()->Self { Self {map:Map::new(),cache:Mutex::new(Cache::default())} }
}
impl Clone for ActorFloors {
    /// O(F) floor-map/Arc snapshot; shared persistent index, fresh handle epoch.
    fn clone(&self)->Self { let c=self.cache.lock().unwrap_or_else(|e|e.into_inner()); Self {map:self.map.clone(),cache:Mutex::new(Cache {registry:c.registry.clone(),dirty:c.dirty,error:c.error.clone(),epoch:next_epoch()})} }
}
impl PartialEq for ActorFloors {
    /// O(total actor payload); derived indexes do not affect state equality.
    fn eq(&self,other:&Self)->bool {self.map==other.map}
}
impl Deref for ActorFloors {
    type Target=Map;
    /// O(1), read-only canonical map access.
    fn deref(&self)->&Map {&self.map}
}
impl DerefMut for ActorFloors {
    /// O(1) invalidation before any public mutable map/vector access.
    fn deref_mut(&mut self)->&mut Map {crate::capture_pilot::mark_unsupported("unrestricted_actor_map");self.invalidate(); &mut self.map}
}
impl<'a> IntoIterator for &'a ActorFloors {
    type Item=(&'a String,&'a Arc<Vec<Actor>>); type IntoIter=indexmap::map::Iter<'a,String,Arc<Vec<Actor>>>;
    /// O(1) iterator construction; iteration O(F).
    fn into_iter(self)->Self::IntoIter {self.map.iter()}
}
impl<'a> IntoIterator for &'a mut ActorFloors {
    type Item=(&'a String,&'a mut Arc<Vec<Actor>>); type IntoIter=indexmap::map::IterMut<'a,String,Arc<Vec<Actor>>>;
    /// O(1) invalidation/iterator construction; iteration O(F).
    fn into_iter(self)->Self::IntoIter {crate::capture_pilot::mark_unsupported("unrestricted_actor_map_iterator");self.invalidate();self.map.iter_mut()}
}
/// O(1): wire identities are exact nonnegative IEEE-754 safe integers.
pub(crate) fn key(id:f64)->Result<u64,String> {if id.is_finite() && id>=0. && id.fract()==0. && id<=9_007_199_254_740_991. {Ok(id as u64)} else {Err(format!("Invalid creature ID {id}: expected a nonnegative safe integer"))}}
impl ActorFloors {
    /// O(1), preserving old slots until the next structural reconciliation.
    pub fn invalidate(&mut self) {let c=self.cache.get_mut().unwrap_or_else(|e|e.into_inner());c.dirty=true;c.error=None;}
    /// O(1): commit a non-rewind transactional clone in the same handle lifetime.
    /// Only used by atomic simulation commits, never snapshot/time-travel restore.
    pub(crate) fn adopt_epoch_from(&mut self,source:&Self) {
        let epoch=source.cache.lock().unwrap_or_else(|e|e.into_inner()).epoch;
        self.cache.get_mut().unwrap_or_else(|e|e.into_inner()).epoch=epoch;
    }
    /// O(F) old map drop plus actor payload destruction if uniquely held; O(1)
    /// move of the new map. Keep current slot history across offscreen proxies.
    pub(crate) fn replace_from(&mut self,other:Self) {crate::capture_pilot::mark_unsupported("actor_map_copyback");self.map=other.map;self.invalidate();}
    /// Expected O(1) map lookup, plus O(actor payload) on first COW write.
    /// Crate-private: caller must preserve IDs, nested pilots, vector order/length,
    /// and must not introduce an independent NPC Duel (use public mutation).
    pub(crate) fn trusted_slice_mut(&mut self,depth:&str)->Option<&mut [Actor]> {crate::capture_pilot::mark_actor_group(depth);self.map.get_mut(depth).map(|v|crate::mutation_journal::make_mut(v).as_mut_slice())}
    /// O(1) indexed floor access plus first-write COW; internal property-only writes.
    /// Must not introduce an independent NPC Duel; use public mutation for that.
    pub(crate) fn trusted_index_mut(&mut self,index:usize)->Option<&mut [Actor]> {if let Some((key,_))=self.map.get_index(index){crate::capture_pilot::mark_actor_group(key);}self.map.get_index_mut(index).map(|(_,v)|crate::mutation_journal::make_mut(v).as_mut_slice())}
    /// Expected O(1) map lookup plus first-write COW; preserves legacy empty-floor insertion.
    /// Must not introduce an independent NPC Duel; use public mutation for that.
    pub(crate) fn trusted_slice_or_default(&mut self,depth:&str)->&mut [Actor] {
        crate::capture_pilot::mark_actor_group(depth);
        if !self.map.contains_key(depth) {self.invalidate();}
        crate::mutation_journal::make_mut(self.map.entry(depth.to_owned()).or_default()).as_mut_slice()
    }
    /// O((A+P+B)*d+F+V) after structural mutation; O(1) when clean.
    /// Extra O((A+P)*d+F); existing O(H*d) slot history remains retained.
    /// Duplicate/invalid IDs fail before installing a replacement index.
    fn ensure(&self,player:f64,depth:f64)->Result<(),String> {
        let mut c=self.cache.lock().unwrap_or_else(|e|e.into_inner());
        let signature=(player.to_bits(),depth.to_bits());
        if !c.dirty && c.registry.player==signature {return c.error.clone().map_or(Ok(()),Err)}
        let result=self.rebuild(&c.registry,player,depth);
        match result {Ok(registry)=>{c.registry=Arc::new(registry);c.dirty=false;c.error=None;Ok(())},Err(e)=>{c.error=Some(e.clone());Err(e)}}
    }
    /// O((A+P+B)*d+F+V) reconciliation, O((A+P)*d+F) new storage.
    /// Departed slots freed before arrivals; historical slot memory O(H*d).
    fn rebuild(&self,old:&Registry,player:f64,depth:f64)->Result<Registry,String> {
        let mut identities=DecimalTrie::default();
        let mut present=DecimalTrie::default();
        let mut desired=Vec::new();
        let mut duel_owners=Vec::new();
        let mut floors=old.floors.clone();
        if player!=-1. {
            let id=key(player)?;identities.insert_new(id,())?;
            let floor=floors.entry(depth.to_string()).or_default();let _=floor;
            let floor=floors.get_index_of(&depth.to_string()).unwrap();
            present.insert_new(id,floor)?;desired.push((id,floor,usize::MAX,usize::MAX));
        }
        for (map_floor,(name,actors)) in self.map.iter().enumerate() {
            floors.entry(name.clone()).or_default();let floor=floors.get_index_of(name).unwrap();
            for (at,a) in actors.iter().enumerate() {
                reserve(a,&mut identities)?;
                // Independent NPC Duel activation is birth/load/public-edit only.
                // Trusted in-place writers may remove it, never introduce it.
                if !crate::half_here::is_clone(a) && (a.conditions.contains_key("r50DuelTarget")
                    || a.active.iter().any(|s|s==crate::revision50::DUEL)
                    || a.abilities.iter().any(|s|s==crate::revision50::DUEL)) {duel_owners.push(a.id);}
                let id=key(a.id)?;present.insert_new(id,floor)?;desired.push((id,floor,map_floor,at));
            }
        }
        for (id,loc) in &old.entries {if present.get(*id)!=Some(&loc.floor) {
            floors.get_index_mut(loc.floor).unwrap().1.remove(loc.handle);
        }}
        let mut positions=DecimalTrie::default();let mut entries=Vec::with_capacity(desired.len());
        for (id,floor,map_floor,at) in desired {
            let handle=if let Some(loc)=old.positions.get(id).filter(|loc|loc.floor==floor) {loc.handle} else {floors.get_index_mut(floor).unwrap().1.allocate(id)?};
            let loc=Location{floor,map_floor,at,handle};positions.insert_new(id,loc.clone())?;entries.push((id,loc));
        }
        Ok(Registry {reserved:identities,positions,entries,floors,player:(player.to_bits(),depth.to_bits()),active_map_floor:self.map.get_index_of(&depth.to_string()),duel_owners:duel_owners.into()})
    }
    /// O(1) shared candidate list when clean; structural reconciliation also scans
    /// K active/owned abilities per actor, O(sum K), and stores O(D) candidates.
    /// A conservative superset: inactive/expired owners are filtered at use.
    /// Native independent activations are inserted actors; player activations and
    /// shared endpoint projections are deliberately outside this derived list.
    pub(crate) fn duel_owner_ids(&self,player:f64,depth:f64)->Result<Arc<[f64]>,String> {
        self.ensure(player,depth)?;
        Ok(self.cache.lock().unwrap_or_else(|e|e.into_inner()).registry.duel_owners.clone())
    }
    /// O(d) clean lookup; dirty-index rebuild O((A+P+B)*d+F+V). Invalid worlds return Err.
    pub(crate) fn position(&self,id:f64,player:f64,depth:f64,current_only:bool)->Result<Option<(usize,usize)>,String> {
        self.ensure(player,depth)?;let id=key(id)?;let c=self.cache.lock().unwrap_or_else(|e|e.into_inner());
        Ok(c.registry.positions.get(id).filter(|l|l.at!=usize::MAX && (!current_only || Some(l.map_floor)==c.registry.active_map_floor)).map(|l|(l.map_floor,l.at)))
    }
    /// O(d) clean lookup; O((A+P+B)*d+F+V) if structural changes need reconciliation.
    pub fn floor_id(&self,id:f64,player:f64,depth:f64)->Result<Option<(String,FloorCreatureId)>,String> {
        self.ensure(player,depth)?;let c=self.cache.lock().unwrap_or_else(|e|e.into_inner());
        Ok(c.registry.positions.get(key(id)?).map(|l|(c.registry.floors.get_index(l.floor).unwrap().0.clone(),FloorCreatureId{epoch:c.epoch,slot:l.handle.slot,generation:l.handle.generation})))
    }
    /// O(d) slot lookup, or O((A+P+B)*d+F+V) pending reconciliation. Rejects stale handles.
    pub fn resolve_floor_id(&self,floor:&str,id:FloorCreatureId,player:f64,depth:f64)->Result<Option<f64>,String> {
        self.ensure(player,depth)?;let c=self.cache.lock().unwrap_or_else(|e|e.into_inner());if id.epoch!=c.epoch{return Ok(None)}
        Ok(c.registry.floors.get(floor).and_then(|s|s.get(SlotHandle{slot:id.slot,generation:id.generation})).map(|id|*id as f64))
    }
    /// O(count*d) clean; dirty reconciliation O((A+P+B)*d+F+V). Reject birth
    /// collision before the creator changes RNG/resources or appends its actor.
    fn check_births(&self,player:f64,depth:f64,next:f64,count:usize)->Result<(),String> {
        self.ensure(player,depth)?;let first=key(next)?;let end=first.checked_add(count as u64).ok_or("Creature ID allocation exhausted")?;key(end as f64)?;if end>9_007_199_254_740_991{return Err("Creature ID allocation exhausted".into())}
        let c=self.cache.lock().unwrap_or_else(|e|e.into_inner());
        for id in first..end {if c.registry.reserved.get(id).is_some(){return Err(format!("Creature ID {id} is already reserved; allocation rejected"))}} Ok(())
    }
    /// O(1) clean, O((A+P+B)*d+F+V) dirty: validate identities and safe numeric cursor.
    pub(crate) fn validate_allocator(&self,player:f64,depth:f64,next:f64)->Result<(),String> {
        self.ensure(player,depth)?;key(next)?;Ok(())
    }
    /// O(1) clean, O((A+P+B)*d+F+V) after mutation; checks player and nested identity uniqueness.
    pub(crate) fn validate_ids(&self,player:f64,depth:f64)->Result<(),String> {self.ensure(player,depth)}
}
/// O(1): mounted-pilot storage belongs to NPC mechs, never the player.
pub(crate) fn check_player(run:&crate::Run)->Result<(),String> {
    if run.player.monster_state.as_ref().and_then(|m|m.pilot.as_ref()).is_some(){Err("Player cannot contain a mounted pilot".into())}else{Ok(())}
}
/// O(d) clean, O((A+P+B)*d+F+V) dirty; does not consume RNG or an ID.
/// Legacy low cursors are preserved, but an actual birth collision is an error.
pub(crate) fn check_next_creature_id(run:&crate::Run)->Result<(),String> {
    check_next_creature_ids(run,1)
}
/// O(count*d) clean, plus dirty rebuild; atomically preflight a birth batch.
pub(crate) fn check_next_creature_ids(run:&crate::Run,count:usize)->Result<(),String> {
    check_player(run)?;run.actors.check_births(run.player.id,run.depth,run.next_id,count)
}
/// O(P*d) for nested pilot chain P; ordinary actor O(d). Mounted pilots reserve
/// identities but intentionally receive no active lookup location or floor slot.
fn reserve(actor:&Actor,ids:&mut DecimalTrie<()>)->Result<(),String> {
    ids.insert_new(key(actor.id)?,())?;
    if let Some(pilot)=actor.monster_state.as_ref().and_then(|m|m.pilot.as_ref()) {reserve(pilot,ids)?;}
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Run,Field};
    /// O(1), empty fixed-size test actor.
    fn a(id:f64)->Actor {Actor{id,..Default::default()}}
    /// O(A), constructing small canonical vectors; no gameplay execution.
    fn run()->Run {let mut r=Run{player:a(0.),depth:1.,next_id:10.,..Default::default()};r.actors.insert("1".into(),Arc::new(vec![a(1.),a(2.)]));r}
    /// O(A*d): public same-length mutation, reordering, and malformed duplicates.
    #[test] fn public_mutations_cannot_leave_stale_locations() {
        let mut r=run();assert_eq!(crate::engine::actor(&r,2.).unwrap().id,2.);
        crate::mutation_journal::make_mut(r.actors.get_mut("1").unwrap()).swap(0,1);
        assert_eq!(crate::engine::actor(&r,2.).unwrap().id,2.);
        r.actors_at_mut(1.)[0]=a(3.);assert!(crate::engine::actor(&r,2.).is_none());assert_eq!(crate::engine::actor(&r,3.).unwrap().id,3.);
        crate::engine::actor_mut(&mut r,3.).unwrap().id=4.;assert!(crate::engine::actor(&r,3.).is_none());assert!(crate::engine::actor(&r,4.).is_some());
        r.actors_at_mut(1.)[0].id=1.;assert!(r.actors.validate_ids(0.,1.).is_err());
        r.actors_at_mut(1.)[0].id=3.5;assert!(r.actors.validate_ids(0.,1.).is_err());
    }
    /// O(A*d): departure frees a slot, new occupant increments generation.
    #[test] fn floor_transfer_reuse_player_and_stale_handles() {
        let mut r=run();let (floor,old)=r.creature_floor_id(1.).unwrap().unwrap();
        let moved=r.actors_at_mut(1.).remove(0);r.actors_at_mut(2.).push(moved);
        let (newfloor,new)=r.creature_floor_id(1.).unwrap().unwrap();assert_eq!(newfloor,"2");assert!(r.creature_from_floor_id(&floor,old).unwrap().is_none());assert_eq!(r.creature_from_floor_id("2",new).unwrap().unwrap().id,1.);
        r.actors_at_mut(1.).push(a(3.));let (_,reused)=r.creature_floor_id(3.).unwrap().unwrap();assert_eq!(reused.slot,old.slot);assert_ne!(reused.generation,old.generation);
        let (_,player)=r.creature_floor_id(0.).unwrap().unwrap();r.depth=2.;assert!(r.creature_from_floor_id("1",player).unwrap().is_none());assert_eq!(r.creature_floor_id(0.).unwrap().unwrap().0,"2");
    }
    /// O(A*d+serialized payload): runtime slots never change save shape/order.
    #[test] fn snapshots_load_and_nested_pilots_preserve_identity_contract() {
        let mut r=run();let (_,h)=r.creature_floor_id(1.).unwrap().unwrap();let clone=r.clone();assert!(clone.creature_from_floor_id("1",h).unwrap().is_none());
        let value=serde_json::to_value(&r).unwrap();let loaded:Run=serde_json::from_value(value.clone()).unwrap();assert_eq!(serde_json::to_value(&loaded).unwrap(),value);assert!(loaded.creature_from_floor_id("1",h).unwrap().is_none());
        r.actors_at_mut(1.)[0].monster_state=Field::Value(crate::ActorMonsterState{pilot:Box::new(a(7.)).into(),..Default::default()});
        assert!(r.actors.validate_ids(0.,1.).is_ok());assert!(crate::engine::actor_any(&r,7.).is_none());r.actors_at_mut(2.).push(a(7.));assert!(r.actors.validate_ids(0.,1.).is_err());
    }
}
