//! Revision-52 outcome-only blessing presentation. Detached snapshots never enter saves.
use crate::{Actor, Run, engine, events::{self, EventPoint, Stage}};

pub fn local(r:&Run,a:&Actor,p:&str,style:&str,stage:Stage) {
    events::cue(r,a,p,style,stage,&[a.into()]);
}
/// Two independent observed endpoints, never an interpolated or guessed route.
pub fn moved(r:&Run,before:&Actor,after:&Actor,p:&str,style:&str) {
    if (before.x,before.y)==(after.x,after.y) {return}
    if visible(r,before) {events::cue(r,before,p,style,Stage::Travel,&[before.into()]);}
    events::cue(r,after,p,style,Stage::Travel,&[after.into()]);
}
pub fn classify(_a:&Actor,kind:&str,label:&str)->Option<(&'static str,&'static str,Stage)> {
    if kind!="pulse" {return None}
    match label {
        "See The Future"=>Some(("P71","ripple",Stage::Special)),
        _=>None,
    }
}
fn has(a:&Actor,id:&str)->bool {a.abilities.iter().any(|s|s==id)}
fn active(a:&Actor,id:&str)->bool {a.active.iter().any(|s|s==id)}
/// Match Hardened Blood's actual stat formula rather than a stale max_hp field.
fn protection_band(r:&Run,a:&Actor)->f64 {
 if !has(a,crate::divine_combat::HARDENED_BLOOD) {return 0.;}
 let owner=engine::actor(r,crate::half_here::owner_id(r,a)).unwrap_or(a);
 ((crate::divine_combat::health_from_stats(r,a,&crate::combat::effective_stats(r,a))-owner.hp).max(0.)/10.).ceil()
}
/// Outline only the observed portion of the exact portal-aware radius. O(V*(Q+R))
/// for visibility Q/range R, O(V) temporary mask; four neighbors per known cell.
fn entropy_boundary(r:&Run)->Vec<EventPoint> {
 let Some(f)=r.floor() else{return Vec::new()};let w=f.width();if w==0{return Vec::new()}
 let known:Vec<_>=(0..f.tiles.len()).map(|i|crate::vision::directly_visible_tile(r,i%w,i/w)
  && crate::half_here::within_range(r,(i%w)as f64,(i/w)as f64,10.)).collect();
 known.iter().enumerate().filter_map(|(i,&inside)|{
  if !inside{return None}let(x,y)=(i%w,i/w);
  let edge=x==0||x+1==w||y==0||i+w>=known.len()
   || !known[i-1]||!known[i+1]||!known[i-w]||!known[i+w];
  edge.then_some(EventPoint{x:x as f64,y:y as f64})
 }).collect()
}
fn visible(r:&Run,a:&Actor)->bool {crate::dreamwalking::same_plane(r,&r.player,a) && (a.id==r.player.id || crate::vision::all_seeing(r) || crate::vision::directly_visible_actor(r,a))}
/// Transition styles only. Acquisition-only inventory feedback is handled separately.
const TOGGLES:&[(&str,&str,&str)]=&[
 ("shrail-rage-at-the-dying-of-the-light","P01","bracket"),
 ("shrail-execute","P02","bracket"),("shrail-grappler","P03","bracket"),
 ("shrail-unstoppable-force","P05","wave"),
 ("pelian-you-shall-never-be-rid-of-me","P07","bracket"),
 ("pelian-never-surrender","P10","shield"),
 ("isheilah-missed-me","P13","unfold"),("isheilah-make-slippery","P15","dip"),
 ("isheilah-quick-hand","P16","speed"),("isheilah-are-you-sure-about-that","P17","fold"),
 ("numo-mind-read","P21","eye"),("etill-phase-through","P25","membrane"),
 ("etill-rushing-torrent","P26","spiral"),("etill-float-away","P28","unfold"),
 ("etill-riptide","P30","bracket"),
 ("ursa-dream-see","P35","eye"),("dawel-unspeak","P45","sound"),
 ("dawel-reverse-entropy","P46","diamond"),("dawel-shadow-in-the-dark","P47","collapse"),
 ("dawel-not-there","P48","split-ring"),("grisha-entropy","P50","spiral"),
 ("grisha-erase-metaphysical","P51","fracture"),("grisha-black-pit","P53","collapse"),
 ("anavani-shine","P55","rise"),("anavani-nova","P57","diamond"),
 ("anavani-star-power","P59","ember"),("anavani-commandment","P60","wave"),
 ("morae-bonded-by-unseen-threads","P68","stitch"),("morae-turning-the-wheel","P69","gear")];

/// O(A*(K+visibility+S)) capture; O(A*(A+K+visibility+S)) finish.
/// S is existing effective-stat/health derivation for Hardened Blood owners only;
/// K is bounded actor metadata. No floor clone, RNG, clock, or saved field.
/// Only actors actually observed before AND after can generate delta effects.
pub struct Snapshot { depth:f64, actors:Vec<(Actor,f64)>, dream:bool,number_ready:bool }
impl Snapshot {
 pub fn capture(r:&Run)->Option<Self> {
  events::enabled(r).then(||Self{depth:r.depth,dream:crate::dreamwalking::active(r),number_ready:engine::angel_ready(r,&r.player),actors:std::iter::once(&r.player)
   .chain(r.actors_at(r.depth)).filter(|a|visible(r,a)).map(|a|(a.clone(),protection_band(r,a))).collect()})
 }
 pub fn finish(self,r:&Run) {
  if !events::enabled(r) {return}
  if self.dream!=crate::dreamwalking::active(r) {local(r,&r.player,"P73","wake",Stage::Special);}
  if self.depth!=r.depth {return}
  if !self.number_ready&&engine::angel_ready(r,&r.player){events::inventory(r,"P42","five-notch",None);}
  for (before,protection_before) in &self.actors {
   let Some(a)=engine::actor(r,before.id).filter(|a|visible(r,a)) else {continue};
   let _command_action=(a.id==r.player.id).then(||events::at_action(r,events::player_action()));
   if a.id==r.player.id {
    for (id,key,style) in [("grisha-boundless-hunger","P49","energy-capacity"),("kative-appropriate","P64","pack-capacity")] {
     if !has(before,id)&&has(a,id) {events::inventory(r,key,style,None);}
    }
   }
   for &(id,p,style) in TOGGLES {
    if id==crate::hanged::ID && crate::hanged::enabled(r.simulation_revision.value_or(29.)) {continue;}
    if active(before,id)!=active(a,id) {
     let on=active(a,id);
     let style=match id {
      "grisha-entropy"=>if on {"collapse"}else{"spiral"},
      "anavani-shine"=>if on {"collapse"}else{"rise"},
      "anavani-star-power"=>if on {"ember-out"}else{"ember-in"},
      _=>if on {style}else{"dissolve"},
     };
     if id=="dawel-reverse-entropy" && on && a.id==r.player.id {
      events::cue(r,a,p,"diamond-boundary",Stage::Special,&entropy_boundary(r));
     } else if id=="shrail-grappler" && on && a.id==r.player.id {
      for target in r.actors_at(r.depth).iter().filter(|target|crate::revision50_grappler::locked(r,target)) {local(r,target,p,"bracket",Stage::Special);}
      local(r,a,p,"bracket",Stage::Special);
     } else if id=="etill-riptide" && on {
      if let Some(target)=a.conditions.get("riptideTarget").and_then(|&id|engine::actor(r,id)){local(r,target,p,"root",Stage::Special);}
      if a.conditions.contains_key("riptideTarget"){events::at(r,p,"bracket",Stage::Special,&[EventPoint{x:engine::condition(a,"riptideX"),y:engine::condition(a,"riptideY")}]);}
      if !a.conditions.contains_key("riptideTarget"){local(r,a,p,style,Stage::Special);}
     } else if id=="isheilah-make-slippery" && on {
      if let Some(target)=a.conditions.get("slipperyTarget").and_then(|&id|engine::actor(r,id)) {
       crate::animation_shapes::area(r,target,p,"box-contract",2,"#a040ff",Stage::Special);
      } else {local(r,a,p,style,Stage::Special);}
     } else if id=="dawel-unspeak" && on {
      if let Some(target)=a.conditions.get("unspeakTarget").and_then(|&id|engine::actor(r,id)) {local(r,target,p,"sound-barred",Stage::Special);}else{local(r,a,p,style,Stage::Special);}
     } else if id=="morae-bonded-by-unseen-threads" && on {
      if let Some(target)=a.conditions.get("bondedTarget").and_then(|&id|engine::actor(r,id)).filter(|b|visible(r,b)) {
       crate::animation_shapes::line(r,a,target.into(),p,"thread-grow","#ff3040");
      } else {local(r,a,p,style,Stage::Special);}
     } else {local(r,a,p,style,Stage::Special);}
    }
   }
   if !active(before,"pelian-fated-duel") && active(a,"pelian-fated-duel") {
    let (x,y)=(engine::condition(a,"r50DuelX"),engine::condition(a,"r50DuelY"));
    let mut center=a.clone();center.x=x;center.y=y;
    crate::animation_shapes::area(r,&center,"P09","box-expand",2,"#ffd340",Stage::Special);
   }
   for (key,p,style) in [("cameTarget","P08","bracket"),("bloodWeaponReady","P41","droplet"),("distracted","P20","bracket")] {
    if engine::condition(a,key)>0. && before.conditions.get(key)!=a.conditions.get(key) {
     if key=="cameTarget" {if let Some(target)=engine::actor(r,engine::condition(a,key)) {crate::animation_shapes::line(r,a,target.into(),p,"thread-grow","#ffd340");}}
     else {local(r,a,p,style,Stage::Special);}
    }
   }
   if !active(before,"ursa-dream-of-time-gone-by") && active(a,"ursa-dream-of-time-gone-by") {local(r,a,"P33","fold",Stage::Special);}
   let protection_after=protection_band(r,a);
   if has(a,"lichor-hardened-blood") && *protection_before!=protection_after {
    local(r,a,"P40",if protection_after>*protection_before {"shield-harden"}else{"shield-soften"},Stage::Special);
   }
   if has(a,"ursa-sleep-walk") && before.energy>0. && a.energy<=0. {local(r,a,"P34","speed",Stage::Special);}
   if a.id==r.player.id && before.conditions.get("r50PursuitTarget")!=a.conditions.get("r50PursuitTarget") && a.conditions.contains_key("r50PursuitTarget") {
    let cells:Vec<_>=crate::revision50_pursuit::route_for_run(r).into_iter().map(|(x,y)|EventPoint{x,y}).collect();
    events::cue(r,a,"P12","speed",Stage::Travel,&cells);
   }
  }
 }
}

#[cfg(test)] mod tests {
 use super::*;
 fn room()->Run {let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.x=2.;r.player.y=2.;let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);r}
 #[test] fn every_toggle_has_one_on_one_off_and_no_repeat(){
  for &(id,key,_) in TOGGLES.iter().filter(|(id,_,_)| *id != crate::hanged::ID) {
   let mut r=room();let snap=Snapshot::capture(&r).unwrap();r.player.active.push(id.into());
   events::begin();snap.finish(&r);let cues=events::finish();assert_eq!(cues.len(),1,"on {id}");assert_eq!(cues[0].animation.as_ref().unwrap().proposal,key);
   let snap=Snapshot::capture(&r).unwrap();events::begin();snap.finish(&r);assert!(events::finish().is_empty(),"repeat {id}");
   let snap=Snapshot::capture(&r).unwrap();r.player.active.clear();events::begin();snap.finish(&r);let cues=events::finish();assert_eq!(cues.len(),1,"off {id}");assert_eq!(cues[0].animation.as_ref().unwrap().proposal,key);
  }
 }
 #[test] fn hanged_stale_toggle_is_silent_and_damage_has_a_shield_cue(){
  let mut r=room();r.player.abilities=vec![crate::hanged::ID.into()];r.player.hp=1.;r.player.energy=4.;
  let snap=Snapshot::capture(&r).unwrap();r.player.active.push(crate::hanged::ID.into());events::begin();snap.finish(&r);assert!(events::finish().is_empty());
  events::begin();let id=r.player.id;crate::engine::damage_environment(&mut r,id,1.,None,"",true).unwrap();let cues=events::finish();
  assert!(cues.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="P13"&&a.style=="shield")));
  assert!(!cues.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.style=="reaction-spent-missed-me")));
 }
 #[test] fn changed_toggle_once_and_no_upkeep_replay(){let mut r=room();let snap=Snapshot::capture(&r).unwrap();r.player.active.push("numo-mind-read".into());events::begin();snap.finish(&r);let e=events::finish();assert_eq!(e.len(),1);assert_eq!(e[0].animation.as_ref().unwrap().proposal,"P21");let snap=Snapshot::capture(&r).unwrap();events::begin();snap.finish(&r);assert!(events::finish().is_empty());}
 #[test] fn old_revision_and_unchanged_movement_are_silent(){let mut r=room();r.simulation_revision=51.0.into();assert!(Snapshot::capture(&r).is_none());r.simulation_revision=52.0.into();events::begin();moved(&r,&r.player,&r.player,"P13","unfold");assert!(events::finish().is_empty());}
 #[test] fn passive_acquisition_has_inventory_only_once_and_no_map_loop(){let mut r=room();let snap=Snapshot::capture(&r).unwrap();r.player.abilities.extend(["ursa-oblivious","grisha-boundless-hunger","kative-appropriate","isheilah-tools-of-the-trade"].map(str::to_owned));events::begin();snap.finish(&r);let cues=events::finish();assert_eq!(cues.len(),2);assert!(cues.iter().all(|e|e.animation.as_ref().unwrap().surface.as_deref()==Some("inventory")));assert_eq!(cues.iter().map(|e|e.animation.as_ref().unwrap().proposal.as_str()).collect::<Vec<_>>(),vec!["P49","P64"]);let snap=Snapshot::capture(&r).unwrap();events::begin();snap.finish(&r);assert!(events::finish().is_empty());}
}

#[cfg(test)] mod outcome_tests {
 use super::*;
 fn room()->Run {let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.ability_reserves=crate::Field::Missing;r.player.x=2.;r.player.y=2.;r.player.max_hp=100.;r.player.hp=100.;let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);r}
 fn proposals()->Vec<String>{events::finish().into_iter().filter_map(|e|e.animation.map(|a|a.proposal)).collect()}
 #[test] fn rescue_is_real_and_bypass_is_quiet(){
  let mut r=room();let id=r.player.id;r.player.abilities=vec![crate::divine_rescues::OVERFLOWING.into()];r.player.blood=1.0.into();r.player.hp=-10.;
  events::begin();assert!(crate::divine_rescues::try_rescue(&mut r,id,false));assert_eq!(r.player.hp,1.);assert_eq!(proposals(),vec!["P37"]);
  r.player.hp=-10.;r.player.blood=1.0.into();events::begin();assert!(!crate::divine_rescues::try_rescue(&mut r,id,true));assert!(proposals().is_empty());
 }
 #[test] fn distinct_rescues_and_spent_rescue_failure(){
  for (ability,key,hp) in [(crate::divine_rescues::RAGE,"P01",1.),(crate::divine_rescues::PELIAN,"P07",100.)] {
   let mut r=room();let id=r.player.id;r.player.abilities=vec![ability.into()];r.player.active=vec![ability.into()];r.player.hp=-1.;
   events::begin();assert!(crate::divine_rescues::try_rescue(&mut r,id,false));assert_eq!(r.player.hp,hp);assert_eq!(proposals(),vec![key]);
   r.player.hp=-1.;r.player.energy=0.;events::begin();assert!(!crate::divine_rescues::try_rescue(&mut r,id,false));assert!(proposals().is_empty());
  }
 }
 #[test] fn execute_needs_paid_below_quarter_landed_hit(){
  let mut r=room();r.player.abilities=vec![crate::revision50_combat::EXECUTE.into()];r.player.active=r.player.abilities.clone();
  crate::revision50_ability_energy::initialize_reserve(&mut r.player,crate::revision50_combat::EXECUTE,40.,40.).unwrap();
  let a=r.player.clone();let mut b=r.actors_at(r.depth)[0].clone();b.x=3.;b.y=2.;b.state="hunting".into();b.conditions.clear();b.max_hp=100.;b.hp=25.;
  *crate::engine::actor_mut_trusted(&mut r,b.id).unwrap()=b.clone();events::begin();assert!(!crate::revision50_combat::lethal_hit(&mut r,&a,&b));assert!(proposals().is_empty());
  b.hp=24.;*crate::engine::actor_mut_trusted(&mut r,b.id).unwrap()=b.clone();events::begin();assert!(crate::revision50_combat::lethal_hit(&mut r,&a,&b));assert_eq!(proposals(),vec!["P02"]);
 }
 #[test] fn full_health_heal_and_empty_reaction_are_quiet(){
  let mut r=room();r.player.abilities=vec![crate::divine_combat::FIGHT_LONG.into()];let a=r.player.clone();
  events::begin();crate::divine_combat::heal_melee_damage(&mut r,&a,20.,true);assert!(proposals().is_empty());
  r.player.hp=50.;let a=r.player.clone();events::begin();crate::divine_combat::heal_melee_damage(&mut r,&a,20.,true);assert_eq!(proposals(),vec!["P04"]);
  r.player.active=vec![crate::divine_reactions::QUICK_HAND.into()];r.player.reactions=0.0.into();let a=r.player.clone();events::begin();crate::divine_reactions::quick_hand_after_hit(&mut r,&a);assert!(proposals().is_empty());
  r.player.reactions=1.0.into();let a=r.player.clone();events::begin();crate::divine_reactions::quick_hand_after_hit(&mut r,&a);let cues=events::finish();assert_eq!(cues.len(),2);assert_eq!(cues.iter().filter(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="P16"&&a.style=="speed"&&a.surface.is_none())).count(),1);assert_eq!(cues.iter().filter(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="P16"&&a.style=="reaction-spent"&&a.surface.as_deref()==Some("inventory"))).count(),1);assert_eq!(r.player.reactions.value_or(0.),0.);
 }
 #[test] fn bleeding_requires_real_positive_damage(){
  let mut r=room();r.player.abilities=vec![crate::divine_reactions::BLEED.into()];let a=r.player.clone();let target=r.actors_at_mut(r.depth).first_mut().unwrap();target.x=3.;target.y=2.;target.conditions.clear();let id=target.id;
  events::begin();crate::divine_reactions::apply_bleeding(&mut r,&a,id,0.,10.);assert!(proposals().is_empty());
  events::begin();crate::divine_reactions::apply_bleeding(&mut r,&a,id,10.,10.);assert_eq!(proposals(),vec!["P38"]);
 }
 #[test] fn snapshot_does_not_disclose_newly_seen_actors(){
  let mut r=room();let target=r.actors_at_mut(r.depth).first_mut().unwrap();target.x=3.;target.y=2.;target.conditions.insert("invisible".into(),10.);let id=target.id;
  let snap=Snapshot::capture(&r).unwrap();let target=engine::actor_mut_trusted(&mut r,id).unwrap();target.conditions.clear();target.active.push("numo-mind-read".into());events::begin();snap.finish(&r);assert!(proposals().is_empty());
 }
}

/// Presentation query matching the authored shadow-haste condition, without
/// exposing which (possibly unseen) enemy changed awareness.
pub fn shadow_active(r:&Run,a:&Actor)->bool {
    has(a,"dawel-step-through-shadows") && engine::condition(a,"slowed")<=0.
        && !r.actors_at(r.depth).iter().any(|b|!b.dead.value_or(false)&&b.hp>0.&&b.state!="ally"&&b.state!="neutral"&&b.conditions.get("seesPlayer")==Some(&1.)&&crate::vision::detection_chance(r,b,a)>0.)
}


#[cfg(test)] mod final_style_tests {
 use super::*;
 fn room()->Run {let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();r.player.x=12.;r.player.y=12.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.inventory.clear();r.actors_at_mut(r.depth).clear();let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.portals=crate::Field::Missing;f.non_euclidean=crate::Field::Missing;r}
 fn styles()->Vec<String>{events::finish().into_iter().filter_map(|e|e.animation.map(|a|a.style)).collect()}
 #[test] fn protection_direction_and_same_band_silence_use_actual_stats(){
  let mut r=room();r.player.abilities.push(crate::divine_combat::HARDENED_BLOOD.into());
  let maximum=crate::divine_combat::health_from_stats(&r,&r.player,&crate::combat::effective_stats(&r,&r.player));
  r.player.max_hp=maximum+10000.;r.player.hp=maximum-1.;
  let snap=Snapshot::capture(&r).unwrap();r.player.hp-=1.;events::begin();snap.finish(&r);assert!(styles().is_empty());
  let snap=Snapshot::capture(&r).unwrap();r.player.hp-=20.;events::begin();snap.finish(&r);assert_eq!(styles(),vec!["shield-harden"]);
  let snap=Snapshot::capture(&r).unwrap();r.player.hp+=20.;events::begin();snap.finish(&r);assert_eq!(styles(),vec!["shield-soften"]);
 }
 #[test] fn fortune_snapshot_does_not_duplicate_explicit_cast_animation(){
  for key in ["lucky","unlucky"] {
   let mut r=room();let snap=Snapshot::capture(&r).unwrap();r.player.conditions.insert(key.into(),1.);events::begin();snap.finish(&r);assert!(styles().is_empty());
   let snap=Snapshot::capture(&r).unwrap();events::begin();snap.finish(&r);assert!(styles().is_empty());
  }
 }
 #[test] fn entropy_uses_known_boundary_not_interior_and_no_upkeep_replay(){
  let mut r=room();let cells=entropy_boundary(&r);assert!(!cells.is_empty());assert!(!cells.contains(&EventPoint::from(&r.player)));
  assert!(cells.iter().all(|p|crate::vision::directly_visible_tile(&r,p.x as usize,p.y as usize)&&crate::half_here::within_range(&r,p.x,p.y,10.)));
  let snap=Snapshot::capture(&r).unwrap();r.player.active.push("dawel-reverse-entropy".into());events::begin();snap.finish(&r);let cues=events::finish();assert_eq!(cues.len(),1);assert_eq!(cues[0].animation.as_ref().unwrap().cells,cells);
  let snap=Snapshot::capture(&r).unwrap();events::begin();snap.finish(&r);assert!(styles().is_empty());
 }
 #[test] fn bond_activation_links_only_observed_target(){
  for hidden in [false,true] {
   let mut r=room();let mut target=r.player.clone();target.id=900.;target.x+=1.;target.state="hunting".into();if hidden{target.conditions.insert("invisible".into(),1.);}
   r.actors_at_mut(r.depth).push(target);let snap=Snapshot::capture(&r).unwrap();r.player.active.push("morae-bonded-by-unseen-threads".into());r.player.conditions.insert("bondedTarget".into(),900.);
   events::begin();snap.finish(&r);let cues=events::finish();assert_eq!(cues.len(),1);assert_eq!(cues[0].animation.as_ref().unwrap().style,if hidden{"stitch"}else{"thread-grow"});assert_eq!(cues[0].from.is_some(),!hidden);
  }
 }
 #[test] fn real_rescues_blood_gain_and_bleed_have_distinct_styles(){
  for (ability,style) in [(crate::divine_rescues::RAGE,"nova-red-small"),(crate::divine_rescues::PELIAN,"nova-blue-small"),(crate::divine_rescues::OVERFLOWING,"droplet-empty")] {
   let mut r=room();r.player.abilities=vec![ability.into()];r.player.active=r.player.abilities.clone();r.player.blood=1.0.into();r.player.hp=-1.;let id=r.player.id;events::begin();assert!(crate::divine_rescues::try_rescue(&mut r,id,false));assert_eq!(styles(),vec![style]);
  }
  let mut r=room();r.player.abilities=vec![crate::divine_rescues::OVERFLOWING.into()];r.player.blood=0.0.into();r.player.damage_bank=0.0.into();let id=r.player.id;events::begin();crate::divine_rescues::accrue_damage(&mut r,id,50.);assert_eq!(styles(),vec!["droplet-fill"]);
  r.player.abilities=vec![crate::divine_reactions::BLEED.into()];let a=r.player.clone();let mut b=a.clone();b.id=900.;b.x+=1.;b.state="hunting".into();r.actors_at_mut(r.depth).push(b);
  events::begin();crate::divine_reactions::apply_bleeding(&mut r,&a,900.,10.,10.);assert_eq!(styles(),vec!["droplet-two"]);
 }
}
