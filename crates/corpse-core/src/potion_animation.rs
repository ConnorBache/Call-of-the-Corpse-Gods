//! Presentation-only radial potion receipts; never changes simulation or RNG.
use crate::{Run, events::{self, AnimationCue, Delivery, EventPoint, PresentationEvent, Stage}};
/// O(W*H) bounded floor geometry, O(C) receipt memory. Telepathy carries only
/// geometry, not hidden identities; the desktop reveals its filtered observation.
pub(crate) fn begin(r:&Run,effect:&str,x:f64,y:f64,target:Option<f64>)->Option<u32>{
 if !crate::potion_rules::current(r)||!events::enabled(r){return None}
 let (style,radius)=match effect {
  "incineration"=>("wave-reveal-fire",Some(2.)),
  "darkness"=>("wave-reveal-darkness",Some(3.)),
  "shattering"=>("wave-reveal-shattering",Some(6.)),
  "telepathy" if target==Some(r.player.id)=>("wave-reveal-telepathy",None),
  _=>return None,
 };
 if crate::dreamwalking::context_dream(r)!=crate::dreamwalking::dream_actor(r,&r.player){return None}
 let floor=r.floor()?;let mut cells=Vec::new();
 for cy in 0..floor.height(){for cx in 0..floor.width(){
  if radius.is_some_and(|radius|(cx as f64-x).powi(2)+(cy as f64-y).powi(2)>radius*radius){continue}
  if radius.is_some()&&!crate::vision::directly_visible_tile(r,cx,cy){continue}
  cells.push(EventPoint{x:cx as f64,y:cy as f64});
 }}
 if cells.is_empty(){return None}
 let action=events::current_action();
 events::emit(r,PresentationEvent{kind:"animation".into(),from:None,to:EventPoint{x,y},glyph:None,color:None,label:None,depth:r.depth,
  animation:Some(AnimationCue{proposal:"potion-wave".into(),style:style.into(),stage:Stage::Special,action,cells,delivery:Delivery::Local,surface:None,item_id:None,arrival_steps:None})});
 Some(action)
}

#[cfg(test)] mod tests {
 use super::*;
 #[test] fn telepathy_wave_is_geometry_only_and_legacy_has_no_new_receipt(){
  let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();r.potion_rules_from=0usize.into();
  events::begin();events::next_action(&r);let id=r.player.id;let x=r.player.x;let y=r.player.y;
  assert!(begin(&r,"telepathy",x,y,Some(id)).is_some());let receipts=events::finish();
  let e=receipts.iter().find(|e|e.animation.as_ref().is_some_and(|a|a.style=="wave-reveal-telepathy")).unwrap();
  assert_eq!(e.animation.as_ref().unwrap().cells.len(),r.floor().unwrap().width()*r.floor().unwrap().height());assert!(e.label.is_none());assert!(e.glyph.is_none());
  r.potion_rules_from=crate::Field::Null;events::begin();events::next_action(&r);assert!(begin(&r,"telepathy",x,y,Some(id)).is_none());assert!(events::finish().is_empty());
 }
}
