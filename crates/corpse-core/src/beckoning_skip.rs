//! One pending action receipt, not a timed status or paralysis animation.
use crate::{Run,engine};
pub const TAG:&str="beckoningSkipOwner";
/// O(indexed actor lookup + existing hostility checks), no actor/status scans.
/// Repeated pulls before the next opportunity refresh one skip, never stack.
pub(crate) fn after_pull(r:&mut Run,source:f64,target:f64) {
 if !crate::beckoning_rules::current(r){return;}
 let (Some(a),Some(b))=(engine::actor(r,source),engine::actor(r,target))else{return};
 if target==r.player.id || b.dead.value_or(false)||b.hp<=0.
  || crate::half_here::owner_id(r,b)==r.player.id || b.state=="ally"
  || !(crate::half_here::owner_id(r,a)==r.player.id||a.state=="ally")
  || !crate::revision50_control_amulets::hostile(r,&r.player,b){return;}
 if let Some(b)=engine::actor_mut_trusted(r,target){b.conditions.insert(TAG.into(),target);}
}
/// Consume only at a scheduled full action boundary. A Haste action can consume
/// it; Fast cannot bypass that skipped action with a move/attack follow-up.
/// The owner receipt prevents clones inheriting another creature's lost action.
pub(crate) fn consume(r:&mut Run,id:f64)->bool {
 if !crate::beckoning_rules::current(r){return false;}
 let Some(a)=engine::actor(r,id)else{return false};
 if a.conditions.get(TAG)!=Some(&id)||a.dead.value_or(false)||a.hp<=0. {return false;}
 let enemy=a.state!="ally"&&crate::half_here::owner_id(r,a)!=r.player.id
  &&crate::revision50_control_amulets::hostile(r,&r.player,a);
 engine::actor_mut_trusted(r,id).unwrap().conditions.shift_remove(TAG);
 enemy
}
