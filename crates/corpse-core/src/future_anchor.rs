//! Immutable second-game Future anchor. Legacy/imported text remains exact until
//! explicit41 admission; newly captured finite41 anchors do not encode JSON.
use crate::{Run,Field,storage::{self,Result,StorageError}};
use std::sync::{Arc,OnceLock};
#[path="future_anchor_finite.rs"] mod finite;
include!("future_anchor_clone.rs");
#[derive(Clone,Debug)]
pub struct FutureAnchor(Arc<Authority>);
#[derive(Debug)]
enum Authority {Text(String), Native{state:Run,encoded:OnceLock<String>,imported:Option<ImportedValidation>}}
// Only successful promote41 can mint this process-local, non-serialized token.
#[derive(Debug)] struct ImportedValidation;
#[cfg(test)] thread_local! {static FRESH_RESTORE_VALIDATIONS:std::cell::Cell<usize>=const{std::cell::Cell::new(0)};}
/// Existing O(S) validation/Value allocation, retained for every fresh native restore.
fn validate_fresh_restore(run:&Run)->Result<()>{
    #[cfg(test)] FRESH_RESTORE_VALIDATIONS.with(|n|n.set(n.get()+1));
    storage::validate(run)
}
impl FutureAnchor {
    /// O(1) move/allocation. No eager validation or changed malformed-anchor timing.
    pub fn from_text(raw:String)->Self {Self(Arc::new(Authority::Text(raw)))}
    /// O(F+metadata/archive+canonical nodes); no output allocation for finite scan.
    /// Nonfinite exceptional states preserve original JSON capture/restore behavior.
    pub(crate) fn capture41(run:&Run)->Result<Self> {
        let state={let _span=crate::timing::span("future.capture.clone");clone_anchor_state(run)};
        let is_finite={let _span=crate::timing::span("future.capture.finite_scan");finite::all_finite(&state)};
        if is_finite {
            Ok(Self(Arc::new(Authority::Native{state,encoded:OnceLock::new(),imported:None})))
        } else {Ok(Self::from_text(storage::serialize(&state)?))}
    }
    /// O(1), native authority available for proof/measurement without serialization.
    pub fn native_state(&self)->Option<&Run>{match &*self.0{Authority::Native{state,..}=>Some(state),_=>None}}
    /// O(1), provenance diagnostic; never accepted from serialized input.
    pub fn is_certified_import(&self)->bool{matches!(&*self.0,Authority::Native{imported:Some(_),..})}
    /// O(1), legacy/import bytes without materializing a native image.
    pub fn text_if_encoded(&self)->Option<&str>{match &*self.0{Authority::Text(s)=>Some(s),Authority::Native{encoded,..}=>encoded.get().map(String::as_str)}}
    /// First native boundary O(S) encoding, later O(1) borrow. Outer serialization
    /// still escapes/copies O(S); cached text is derived, immutable and replace-local.
    pub fn wire_text(&self)->Result<&str>{match &*self.0{
        Authority::Text(s)=>Ok(s),
        Authority::Native{state,encoded,..}=>{if encoded.get().is_none(){let s={let _span=crate::timing::span("future.wire.first_encode");storage::serialize(state)?};let _=encoded.set(s);}Ok(encoded.get().unwrap())}
    }}
    /// O(S) parse/validation for text or O(F+metadata/archive+validation) for native.
    /// Fresh anchors retain full canonical Value validation. Certified immutable
    /// imports already paid that validation during load and skip only its duplicate.
    /// No anchor stringify/parse is needed. Caller retains revision/retired
    /// history preflight before publishing and reapplies live cumulative costs.
    pub(crate) fn prepare_restore(&self)->Result<Run>{match &*self.0{
        Authority::Text(s)=>{let _span=crate::timing::span("future.restore.text_parse_validate");storage::deserialize(s)},
        Authority::Native{state,imported,..}=>{
            let mut run={let _span=crate::timing::span("future.restore.clone_normalize");
                let mut run=clone_anchor_state(state);
                normalize_inventory_order(&mut run);
                reset_derived(&mut run);run
            };
            if imported.is_none(){let _span=crate::timing::span("future.restore.storage_validate");validate_fresh_restore(&run)?;}
            crate::revision52_reserves::normalize_run(&mut run).map_err(StorageError)?;
            crate::revision50::settle_current_duels(&mut run);
            Ok(run)
        }
    }}
    /// O(S parse + validation + finite nodes + raw bytes clone), admission only.
    /// Warm valid finite41 imported anchors so later restore avoids parsing. Failed
    /// preparation remains exact text; errors are deferred to actual activation.
    /// Original wire is cached intact, preserving checkpoint spelling on import.
    pub(crate) fn promote41(&mut self,revision:&Field<f64>){
        let Authority::Text(raw)=&*self.0 else{return};
        let _span=crate::timing::span("future.import.promotion");
        let parsed={let _span=crate::timing::span("future.import.parse_validate");storage::deserialize(raw)};
        let Ok(mut state)=parsed else{return};
        if &state.simulation_revision!=revision || crate::temporal::retains_revert_history(&state)
            || crate::temporal::History::import(&state).is_err()
            || crate::timeweave::validate_run(&state).is_err()
            || !{let _span=crate::timing::span("future.import.finite_scan");finite::all_finite(&state)} {return;}
        // Imported nested Future/replay are discarded by actual restore anyway.
        // No recursive native anchor graph is retained in the second game.
        state.future=None;state.history.clear();state.replay.clear();state.replay_snapshot_from=Field::Missing;state.firebolt_travel_from=Field::Missing;state.ragelight_rules_from=Field::Missing;state.balance_rules_from=Field::Missing;state.collapse_rules_from=Field::Missing;state.execute_rules_from=Field::Missing;state.beckoning_rules_from=Field::Missing;
        reset_derived(&mut state);
        let encoded=OnceLock::new();let _=encoded.set(raw.clone());
        // deserialize already validated the normalized image. The above checks
        // establish41/empty retired history; removing Future/replay/policy cannot
        // invalidate world schema, and normalized inventory order is a fixed point.
        *self=Self(Arc::new(Authority::Native{state,encoded,imported:Some(ImportedValidation)}));
    }
}
impl From<String> for FutureAnchor { /// O(1) move/allocation.
 fn from(raw:String)->Self{Self::from_text(raw)}}
impl PartialEq for FutureAnchor {
    /// O(1) shared identity; O(S) distinct legacy byte comparison. Distinct native
    /// authority conservatively differs; this is not world semantic equality and
    /// never triggers encoding merely to satisfy Snapshot cache comparisons.
    fn eq(&self,other:&Self)->bool{Arc::ptr_eq(&self.0,&other.0)||match(&*self.0,&*other.0){(Authority::Text(a),Authority::Text(b))=>a==b,_=>false}}
}
impl serde::Serialize for FutureAnchor {
    /// O(S) output/escaping, first native boundary additionally encodes its state.
    fn serialize<S:serde::Serializer>(&self,s:S)->std::result::Result<S::Ok,S::Error>{s.serialize_str(self.wire_text().map_err(serde::ser::Error::custom)?)}
}
impl<'de> serde::Deserialize<'de> for FutureAnchor {
    /// O(S) input allocation; no parse until explicit native admission/restore.
    fn deserialize<D:serde::Deserializer<'de>>(d:D)->std::result::Result<Self,D::Error>{<String as serde::Deserialize>::deserialize(d).map(Self::from_text)}
}
/// Expected O(I+D), O(I+D) scratch; exactly matches storage JSON normalization:
/// Missing stays absent, Null becomes empty Value, surviving IDs are first-only.
fn normalize_inventory_order(run:&mut Run){
    if run.inventory_display_order.is_missing(){return;}
    let key=|n:f64|if n==0.0{0}else{n.to_bits()};
    let ids:std::collections::HashSet<_>=run.inventory.iter().map(|i|key(i.id)).collect();
    let mut seen=std::collections::HashSet::new();
    let old=std::mem::take(&mut run.inventory_display_order);
    let next=old.as_ref().into_iter().flatten().copied().filter(|n|ids.contains(&key(*n))&&seen.insert(key(*n))).collect();
    run.inventory_display_order=Field::Value(next);
}
/// O(1) cache reset/invalidation; later typed admission/lookups pay rebuild costs.
fn reset_derived(run:&mut Run){run.optical_cache=Default::default();run.lighting_cache=Default::default();run.actors.invalidate();crate::health_dirty::invalidate(run);}
/// O(encoded anchor bytes), explicit legacy Frame boundary, never a command probe.
pub(crate) fn wire_option(anchor:&Option<FutureAnchor>)->Result<Option<String>>{anchor.as_ref().map(|a|a.wire_text().map(str::to_owned)).transpose()}
#[cfg(test)] mod tests {
 use super::*;
 /// O(fixture bytes), valid41 world without retired ability/history state.
 fn fixture()->Run {let mut r=crate::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap().run;r.simulation_revision=41.0.into();r.player.abilities.clear();r.player.active.clear();r.player.ability_order=Field::Missing;r.player.conditions.clear();r.history.clear();r.replay.clear();r.haste_snapshot=Field::Missing;r.future=None;r.offers.clear();r}
 /// O(world nodes), finite capture must not eagerly materialize a JSON string.
 #[test] fn native_capture_is_lazy_shared_and_boundary_compatible(){let r=fixture();let a=FutureAnchor::capture41(&r).unwrap();assert!(a.native_state().is_some());assert!(a.text_if_encoded().is_none());let b=a.clone();assert_eq!(a,b);assert!(b.text_if_encoded().is_none());let native=a.prepare_restore().unwrap();let wire=a.wire_text().unwrap();let old=storage::deserialize(wire).unwrap();assert_eq!(storage::serialize(&native).unwrap(),storage::serialize(&old).unwrap());assert_eq!(b.text_if_encoded(),Some(wire));}
 /// O(world nodes), every serialized numeric location is visited; textual NaN is harmless.
 #[test] fn finite_visitor_handles_bits_and_nested_nonfinite(){let mut r=fixture();r.player.stats.offense=-0.0;r.seed="NaN Infinity".into();assert!(finite::all_finite(&r));r.player.stats.offense=f64::from_bits(1);assert!(finite::all_finite(&r));r.player.stats.offense=f64::MAX;assert!(finite::all_finite(&r));r.metrics.insert("late".into(),f64::INFINITY);assert!(!finite::all_finite(&r));let a=FutureAnchor::capture41(&r).unwrap();assert!(a.native_state().is_none());assert_eq!(a.wire_text().unwrap(),storage::serialize(&clone_anchor_state(&r)).unwrap());assert!(a.prepare_restore().is_err());}
 /// O(cases*world bytes), exceptional optional numbers retain JSON null semantics.
 #[test] fn optional_nonfinite_uses_exact_legacy_fallback(){for n in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY]{let mut r=fixture();r.damage_since_last_turn=n.into();let a=FutureAnchor::capture41(&r).unwrap();assert!(a.native_state().is_none());let expected=storage::serialize(&clone_anchor_state(&r)).unwrap();assert_eq!(a.wire_text().unwrap(),expected);let old=storage::deserialize(&expected);let now=a.prepare_restore();assert_eq!(old.as_ref().map(|r|storage::serialize(r).unwrap()).map_err(ToString::to_string),now.as_ref().map(|r|storage::serialize(r).unwrap()).map_err(ToString::to_string));}}
 /// O(cases*(I+D+world bytes)), presentNull/stale/duplicate matches old normalization.
 #[test] fn typed_inventory_normalization_matches_json(){for order in [Field::Missing,Field::Null,Field::Value(vec![999999.,-0.,0.,1.,1.])]{let mut r=fixture();r.inventory_display_order=order;let a=FutureAnchor::capture41(&r).unwrap();let typed=a.prepare_restore().unwrap();let old=storage::deserialize(a.wire_text().unwrap()).unwrap();assert_eq!(typed.inventory_display_order,old.inventory_display_order);}}
 /// O(world parse/validation), warmed imported41 avoids subsequent parse; bad
 /// imported bytes remain intact and fail only on actual restore.
 #[test] fn imported_valid_promotes_invalid_defers_and_replacement_is_local(){let r=fixture();let raw=storage::serialize(&clone_anchor_state(&r)).unwrap();let mut a=FutureAnchor::from_text(raw.clone());a.promote41(&r.simulation_revision);assert!(a.native_state().is_some());assert_eq!(a.wire_text().unwrap(),raw);assert!(a.prepare_restore().is_ok());let mut bad=FutureAnchor::from_text("invalid".into());bad.promote41(&r.simulation_revision);assert!(bad.native_state().is_none());assert_eq!(bad.wire_text().unwrap(),"invalid");assert!(bad.prepare_restore().is_err());let mut next=r.clone();next.tick+=1.;let fresh=FutureAnchor::capture41(&next).unwrap();assert!(fresh.text_if_encoded().is_none());assert_ne!(fresh.wire_text().unwrap(),raw);}
 /// O(world metadata), no nested anchor/replay/history is cloned into native state.
 #[test] fn capture_omits_nested_authorities_without_touching_current(){let mut r=fixture();r.future=Some(FutureAnchor::from_text("opaque old anchor".into()));r.replay.push(crate::Command::Wait);let a=FutureAnchor::capture41(&r).unwrap();let state=a.native_state().unwrap();assert!(state.future.is_none());assert!(state.replay.is_empty());assert!(state.history.is_empty());assert!(state.replay_snapshot_from.is_missing());assert_eq!(r.future.as_ref().unwrap().wire_text().unwrap(),"opaque old anchor");assert_eq!(r.replay.len(),1);}
 /// O(world nodes + changed floor clone), current mutations cannot alter the
 /// immutable native game, and normal checkpoint/load restores the same state.
 #[test] fn checkpoint_reload_and_cow_divergence_match_native_restore(){
    let mut r=fixture();let anchor=FutureAnchor::capture41(&r).unwrap();
    assert!(Arc::ptr_eq(&r.floors[0],&anchor.native_state().unwrap().floors[0]));
    let original=anchor.native_state().unwrap().floors[0].tiles[0].clone();
    Arc::make_mut(&mut r.floors[0]).tiles[0]="floor".into();
    assert_eq!(anchor.native_state().unwrap().floors[0].tiles[0],original);
    r.future=Some(anchor.clone());let live=anchor.prepare_restore().unwrap();
    let encoded=storage::serialize(&r).unwrap();
    let loaded=crate::temporal::Engine::load(&encoded).unwrap();
    let restored=loaded.run.future.as_ref().unwrap().prepare_restore().unwrap();
    assert!(loaded.run.future.as_ref().unwrap().native_state().is_some());
    assert_eq!(storage::serialize(&live).unwrap(),storage::serialize(&restored).unwrap());
 }
 /// O(anchor bytes), invalid retired state and mismatched revisions remain text
 /// and are rejected by the original temporal preflight, never hidden by stripping.
 #[test] fn promotion_does_not_hide_invalid_history_or_revision(){
    let r=fixture();for mode in 0..2{let mut old=r.clone();if mode==0{old.history.push("invalid".into());}else{old.simulation_revision=40.0.into();}
      let raw=storage::serialize(&old).unwrap();let mut a=FutureAnchor::from_text(raw.clone());a.promote41(&r.simulation_revision);assert!(a.native_state().is_none());assert_eq!(a.wire_text().unwrap(),raw);
    }
 }

 /// O(engine metadata clone + first anchor encoding). Proof clones share the
 /// immutable lazy cache, so benchmark proofs must run after the cold checkpoint.
 #[test] fn engine_clone_save_warms_shared_cache_explicitly(){
    let mut r=fixture();r.player.abilities=vec!["morae-see-the-future".into()];
    let mut e=crate::temporal::Engine::new(r).unwrap();
    e.dispatch(&crate::Command::Ability{id:"morae-see-the-future".into(),target:Field::Missing,x:Field::Missing,y:Field::Missing}).unwrap();
    assert!(e.run.future.as_ref().unwrap().text_if_encoded().is_none());
    let proof=e.clone();assert!(proof.run.future.as_ref().unwrap().text_if_encoded().is_none());
    let _wire=proof.save().unwrap();
    assert!(e.run.future.as_ref().unwrap().text_if_encoded().is_some());
 }

 /// O(world validation and canonical comparisons), fresh remains validated on
 /// every restore; imported provenance skips only the duplicate existing check.
 #[test] fn imported_certificate_skips_only_duplicate_validation(){
    let mut r=fixture();r.inventory_display_order=Field::Null;
    let fresh=FutureAnchor::capture41(&r).unwrap();assert!(!fresh.is_certified_import());
    FRESH_RESTORE_VALIDATIONS.with(|n|n.set(0));
    let expected=fresh.prepare_restore().unwrap();fresh.prepare_restore().unwrap();
    assert_eq!(FRESH_RESTORE_VALIDATIONS.with(|n|n.get()),2);
    let raw=fresh.wire_text().unwrap().to_owned();let mut imported=FutureAnchor::from_text(raw.clone());
    assert!(!imported.is_certified_import());imported.promote41(&r.simulation_revision);assert!(imported.is_certified_import());
    FRESH_RESTORE_VALIDATIONS.with(|n|n.set(0));
    let mut first=imported.prepare_restore().unwrap();
    assert_eq!(storage::serialize(&first).unwrap(),storage::serialize(&expected).unwrap());
    first.rng=-1.;let second=imported.prepare_restore().unwrap();assert_eq!(second.rng,expected.rng);
    assert_eq!(FRESH_RESTORE_VALIDATIONS.with(|n|n.get()),0);
    assert_eq!(imported.wire_text().unwrap(),raw);
    let decoded:FutureAnchor=serde_json::from_str(&serde_json::to_string(&imported).unwrap()).unwrap();
    assert!(!decoded.is_certified_import(),"wire must never mint a certificate");
 }
 /// O(world capture/validation), invalid finite fresh inputs keep original
 /// deferred restore errors; fresh capture is not falsely certified by finiteness.
 #[test] fn finite_invalid_fresh_anchor_still_validates_on_restore(){
    let mut r=fixture();r.rng=-1.;let fresh=FutureAnchor::capture41(&r).unwrap();
    assert!(fresh.native_state().is_some());assert!(!fresh.is_certified_import());
    let expected=storage::deserialize(fresh.wire_text().unwrap()).unwrap_err().to_string();
    FRESH_RESTORE_VALIDATIONS.with(|n|n.set(0));assert_eq!(fresh.prepare_restore().unwrap_err().to_string(),expected);
    assert_eq!(FRESH_RESTORE_VALIDATIONS.with(|n|n.get()),1);
    let mut imported=FutureAnchor::from_text(fresh.wire_text().unwrap().to_owned());imported.promote41(&r.simulation_revision);assert!(!imported.is_certified_import());assert!(imported.native_state().is_none());
 }

}
