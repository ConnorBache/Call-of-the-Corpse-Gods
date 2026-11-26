//! Detached observed scenes, revealed only after their causal animation ends.
//! Simulation still commits atomically; no renderer time or RNG enters Run.
use crate::{effects::{Cue, duration}, render::View};
use corpse_core::events::PresentationFrame;
use std::rc::Rc;
#[derive(Default)]
pub struct Sequence {
    before: Option<Rc<View>>,
    frames: Vec<(u32, Rc<View>)>,
    ends: Vec<(u32, f64)>,
    finish: f64,
    reveals: Vec<(f64, f64, Rc<View>)>,
}
impl Sequence {
    pub fn clear(&mut self) { *self = Self::default(); }
    pub fn capture_run(&mut self, before: corpse_core::Run, frames: Vec<PresentationFrame>, cues: &[Cue], profile: &crate::appearance_profile::Profile) {
        if !cues.iter().any(|c| crate::effects::blocks(&c.event)) { self.clear(); return; }
        self.capture(observe(&before,profile), frames, cues, profile);
    }
    pub fn capture(&mut self, before: View, frames: Vec<PresentationFrame>, cues: &[Cue], profile: &crate::appearance_profile::Profile) {
        self.clear();
        let cues: Vec<_> = cues.iter().filter(|c| crate::effects::blocks(&c.event)).collect();
        if cues.is_empty() { return; }
        self.before = Some(Rc::new(before));
        // Select the last completed boundary preceding each visible action.
        // Quiet scheduler work needs no separately rendered scene. Raw detached
        // simulation snapshots are immediately dropped after this projection.
        let mut actions: Vec<_> = cues.iter().filter_map(|c| c.event.animation.as_ref().map(|a| a.action)).collect();
        actions.sort_unstable(); actions.dedup();
        let mut selected = std::collections::BTreeSet::new();
        for action in actions {
            if let Some(index) = frames.iter().rposition(|f| f.action < action) { selected.insert(index); }
        }
        for cue in &cues {if cue.event.animation.as_ref().is_some_and(|a|a.style=="map-reveal"||a.style.starts_with("wave-reveal")||a.arrival_steps.is_some()||a.hide_actor.is_some()||a.style==corpse_core::future_floor::FADE) {
            let action=cue.event.animation.as_ref().unwrap().action;
            if let Some(index)=frames.iter().position(|f|f.action==action){selected.insert(index);}
        }}
        for (index, frame) in frames.into_iter().enumerate() {
            if !selected.contains(&index) { continue; }
            if let Some(run) = frame.observation.downcast_ref::<corpse_core::Run>() {
                self.frames.push((frame.action, Rc::new(observe(run,profile))));
            } else if let Ok(view) = frame.observation.downcast::<View>() {
                self.frames.push((frame.action,view));
            }
        }
        for cue in &cues {
            if let Some(a)=cue.event.animation.as_ref().filter(|a|a.style=="map-reveal"||a.style.starts_with("wave-reveal")) {
                if let Some((_,after))=self.frames.iter().find(|(id,_)|*id==a.action) {
                    let base=self.frames.iter().rev().find(|(id,_)|*id<a.action).map(|(_,v)|v).or(self.before.as_ref()).unwrap();
                    let center=(cue.event.to.x,cue.event.to.y);
                    let allowed:std::collections::BTreeSet<_>=a.cells.iter().map(|p|(p.x as usize,p.y as usize)).collect();
                    let distance=|c:&crate::render::Cell|((c.x as f64-center.0).powi(2)+(c.y as f64-center.1).powi(2)).sqrt();
                    let max=a.cells.iter().map(|p|((p.x-center.0).powi(2)+(p.y-center.1).powi(2)).sqrt()).fold(1f64,f64::max);
                    for step in 0..=10 {
                        let radius=max*step as f64/10.;let mut view=(**base).clone();
                        let arrived=|c:&crate::render::Cell|allowed.contains(&(c.x,c.y))&&distance(c)<=radius+1e-9;
                        // Preserve all z-layers, including terrain and actor at the
                        // same coordinate. Never rebuild from the final terrain snapshot.
                        view.cells.retain(|c|!arrived(c));
                        view.cells.extend(after.cells.iter().filter(|c|arrived(c)).cloned());
                        view.terrain_snapshot=None;view.terrain_count=0;
                        view.hallucination_cells=view.cells.iter().enumerate().filter_map(|(i,c)|c.hallucination.is_some().then_some(i)).collect();
                        if a.style.starts_with("wave-reveal") {
                            let reached=|id:f64|base.cells.iter().chain(after.cells.iter()).any(|c|c.actor_id==Some(id)&&allowed.contains(&(c.x,c.y))&&distance(c)<=radius+1e-9);
                            view.creatures.retain(|c|!reached(c.id)||after.creatures.iter().any(|n|n.id==c.id));
                            for creature in &mut view.creatures {if reached(creature.id){if let Some(new)=after.creatures.iter().find(|c|c.id==creature.id){*creature=new.clone();}}}
                            for creature in &after.creatures {if reached(creature.id)&&!view.creatures.iter().any(|c|c.id==creature.id){view.creatures.push(creature.clone());}}
                            view.creatures.sort_by(|a,b|a.distance.total_cmp(&b.distance));
                            if reached(after.player_id){view.hp=after.hp;view.dead=after.dead;view.player_statuses=after.player_statuses.clone();view.player_meters=after.player_meters.clone();}

                        }
                        self.reveals.push((cue.started+step as f64*65.+if a.style=="wave-reveal-shattering"{120.}else{0.},cue.started+duration(&cue.event),Rc::new(view)));
                    }
                }
            }
        }
        for cue in &cues {
            let Some(a)=cue.event.animation.as_ref().filter(|a|a.proposal=="E03" && !cue.static_marks) else{continue};
            let Some(bands)=a.arrival_steps.as_ref() else{continue};
            let Some((_,after))=self.frames.iter().find(|(id,_)|*id==a.action) else{continue};
            let base=self.frames.iter().rev().find(|(id,_)|*id<a.action)
                .map(|(_,v)|v).or(self.before.as_ref()).unwrap();
            for band in 0..=bands.iter().copied().max().unwrap_or(0).min(5) {
                let reached:std::collections::BTreeSet<_>=a.cells.iter().zip(bands).filter(|(_,i)|**i<=band)
                    .map(|(p,_)|(p.x as usize,p.y as usize)).collect();
                if let Some(view)=breath_ignitions(base,after,&reached) {
                    let start=cue.started+(band as f64+1.)*crate::effects::MAGIC_CELL_MS;
                    self.reveals.push((start,cue.started+duration(&cue.event),Rc::new(view)));
                }
            }
        }
        for cue in &cues {
            let end = cue.started + duration(&cue.event);
            self.finish = self.finish.max(end);
            if let Some(a) = &cue.event.animation {
                if let Some((_, until)) = self.ends.iter_mut().find(|(id,_)| *id == a.action) { *until = until.max(end); }
                else { self.ends.push((a.action,end)); }
            }
        }
        for cue in &cues {
            let Some(a)=cue.event.animation.as_ref().filter(|a|a.style==corpse_core::future_floor::FADE) else {continue};
            // The scene switches only under opaque purple; its restored map
            // and HUD are both present throughout the fade back in.
            if let Some((_,after))=self.frames.iter().find(|(id,_)|*id==a.action) {
                self.reveals.push((cue.started+crate::effects::REWIND_MIDPOINT_MS,
                    cue.started+duration(&cue.event),after.clone()));
            }
        }
        for cue in &cues {
            let Some(a)=cue.event.animation.as_ref().filter(|_| !cue.static_marks) else { continue };
            let Some(actor)=a.hide_actor else { continue };
            let base=self.frames.iter().rev().find(|(id,_)|*id<a.action)
                .map(|(_,v)|v).or(self.before.as_ref()).unwrap();
            // Paint the original terrain and everyone's original positions
            // while the return beam travels, hiding only this actor layer.
            if base.cells.iter().any(|c|c.actor_id==Some(actor)) {
                let mut hidden=(**base).clone();
                hidden.cells.retain(|c|c.actor_id!=Some(actor));
                hidden.hallucination_cells=hidden.cells.iter().enumerate()
                    .filter_map(|(idx,c)|c.hallucination.is_some().then_some(idx)).collect();
                let end=cue.started+duration(&cue.event);
                self.reveals.push((cue.started,end,Rc::new(hidden)));
                // If a secondary HUD cue is still running, the landed actor
                // is already visible at the exact end of the return trip.
                let action_end=self.ends.iter().find(|(id,_)|*id==a.action).map_or(end,|(_,end)|*end);
                if action_end>end {
                    if let Some((_,after))=self.frames.iter().find(|(id,_)|*id==a.action) {
                        self.reveals.push((end,action_end,after.clone()));
                    }
                }
            }
        }
        self.reveals.sort_by(|a,b|a.0.total_cmp(&b.0));
    }
    pub fn shift(&mut self, amount: f64) { self.finish += amount; for (start,end,_) in &mut self.reveals {*start+=amount;*end+=amount;} for (_,end) in &mut self.ends { *end += amount; } }
    pub fn active(&self) -> bool { self.before.is_some() }
    pub fn pending(&self, now: f64) -> bool { self.before.is_some() && now < self.finish }
    /// O(F*A), bounded by 128 frames and 4096 receipts, no view clones per paint.
    /// A frame can contain quiet actions too, but never an unfinished action.
    pub fn view(&self, now: f64) -> Option<&View> {
        if !self.pending(now) { return None; }
        if let Some((_,_,view))=self.reveals.iter().rev().find(|(start,end,_)|now>=*start&&now<*end){return Some(view);}
        self.frames.iter().rev().find(|(id,_)| self.ends.iter().all(|(action,end)| action > id || now >= *end))
            .map(|(_,v)|v.as_ref()).or(self.before.as_deref())
    }
}

/// O(V log R) per displayed band for V view layers and R reached coordinates.
/// Only swap terrain/fire at tiles observed by both boundaries; actor state and
/// the HUD wait for the normal action boundary.
fn breath_ignitions(base:&View,after:&View,reached:&std::collections::BTreeSet<(usize,usize)>)->Option<View> {
    if base.depth!=after.depth||base.dreamwalking!=after.dreamwalking||base.width!=after.width||base.height!=after.height {return None;}
    let pre=base.terrain_snapshot.as_ref()?;
    let post=after.terrain_snapshot.as_ref()?;
    let allowed:std::collections::BTreeSet<_>=reached.iter().copied().filter(|&(x,y)| {
        let at=y*base.width+x;
        let (Some(was),Some(is))=(pre.cells.get(at),post.cells.get(at)) else{return false;};
        if !(was.current&&was.direct&&is.current&&is.direct) {return false;}
        let old=was.hazards.as_ref().and_then(|h|h.fire.as_ref());
        let new=is.hazards.as_ref().and_then(|h|h.fire.as_ref());
        new.is_some() && (old!=new || was.tile!=is.tile)
    }).collect();
    if allowed.is_empty(){return None;}
    let mut view=base.clone();
    view.cells.clear();
    let selected=|c:&crate::render::Cell|allowed.contains(&(c.x,c.y));
    view.cells.extend(base.cells.iter().take(base.terrain_count).filter(|c|!selected(c)).cloned());
    view.cells.extend(after.cells.iter().take(after.terrain_count).filter(|c|selected(c)).cloned());
    view.terrain_count=view.cells.len();
    view.cells.extend(after.cells.iter().skip(after.terrain_count).filter(|c|c.fire&&selected(c)).cloned());
    view.cells.extend(base.cells.iter().skip(base.terrain_count).filter(|c|!c.fire||!selected(c)).cloned());
    // Retain the original scene's knowledge rather than exposing the outcome's
    // light sources; the precomputed cells above already contain the new fire.
    view.terrain_snapshot=None;
    view.hallucination_cells=view.cells.iter().enumerate().filter_map(|(i,c)|c.hallucination.is_some().then_some(i)).collect();
    Some(view)
}

/// An enemy may be resolving in the other plane. Never paint that working
/// context as the player's knowledge; normalize only a detached Run clone.
pub fn observe(run: &corpse_core::Run, profile: &crate::appearance_profile::Profile) -> View {
    if corpse_core::dreamwalking::context_dream(run) != corpse_core::dreamwalking::active(run) {
        let mut detached = run.clone();
        let player = detached.player.id;
        corpse_core::dreamwalking::with_actor(&mut detached, player, |r| crate::observe::observe_with_profile(r,profile))
    } else { crate::observe::observe_with_profile(run,profile) }
}

#[cfg(test)]mod wave_tests{
 use super::*;use corpse_core::events::{AnimationCue,Delivery,EventPoint,PresentationEvent,Stage};
 #[test]fn revelation_advances_only_through_receipt_cells(){
  let run:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();let profile=crate::appearance_profile::Profile::defaults();
  let mut before=observe(&run,profile);before.cells.clear();let mut after=before.clone();
  for x in [0,5,10] {after.cells.push(crate::render::Cell{x,y:0,glyph:".".into(),label:"revealed".into(),..Default::default()});}
  let event=PresentationEvent{kind:"animation".into(),from:None,to:EventPoint{x:0.,y:0.},glyph:None,color:None,label:None,depth:1.,animation:Some(AnimationCue{proposal:"P72".into(),style:"map-reveal".into(),stage:Stage::Special,action:1,cells:vec![EventPoint{x:0.,y:0.},EventPoint{x:10.,y:0.}],delivery:Delivery::Local,surface:None,item_id:None,hide_actor:None,arrival_steps:None})};
  let cue=Cue{event,started:0.,held:false,static_marks:false};let mut sequence=Sequence::default();sequence.capture(before,vec![PresentationFrame{action:1,observation:Rc::new(after)}],&[cue],profile);
  assert_eq!(sequence.view(0.).unwrap().cells.len(),1);assert_eq!(sequence.view(400.).unwrap().cells.len(),1);assert_eq!(sequence.view(700.).unwrap().cells.len(),2);assert!(!sequence.view(700.).unwrap().cells.iter().any(|c|c.x==5));assert!(sequence.view(900.).is_none());
 }
}

#[cfg(test)] mod damage_wave_tests {
 use super::*;
 use corpse_core::events::{AnimationCue,Delivery,EventPoint,PresentationEvent,Stage};
 #[test] fn damage_status_and_death_wait_for_wave_arrival() {
  let run:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
  let profile=crate::appearance_profile::Profile::defaults();
  let mut before=observe(&run,profile);before.player_id=1.;before.hp=50.;before.dead=false;before.player_statuses.clear();
  before.creatures=vec![crate::render::Creature { perception_range:None,identity_outline:None,hallucination:None,shared_resources:false,invulnerable:false,id:2.,name:"Victim".into(),glyph:"v".into(),color:[255,255,255],state:"Hunting".into(),distance:5.,hp:10.,max_hp:10.,energy:0.,max_energy:0.,uses_energy:false,statuses:vec![],meters:vec![] }];
  before.cells=vec![crate::render::Cell {x:5,y:0,actor_id:Some(2.),glyph:"v".into(),..Default::default()},crate::render::Cell {x:10,y:0,actor_id:Some(1.),glyph:"@".into(),..Default::default()}];
  let mut after=before.clone();after.creatures.clear();after.cells[0].actor_id=None;after.cells[0].glyph="x".into();after.hp=0.;after.dead=true;after.player_statuses.push(crate::statuses_ui::Status{name:"Stunned".into(),duration:Some(2.)});
  let event=PresentationEvent{kind:"animation".into(),from:None,to:EventPoint{x:0.,y:0.},glyph:None,color:None,label:None,depth:1.,animation:Some(AnimationCue{proposal:"E18".into(),style:"wave-reveal-purple".into(),stage:Stage::Special,action:1,cells:(0..=10).map(|x|EventPoint{x:x as f64,y:0.}).collect(),delivery:Delivery::Local,surface:None,item_id:None,hide_actor:None,arrival_steps:None})};
  let cue=Cue{event:event.clone(),started:0.,held:false,static_marks:false};let mut sequence=Sequence::default();sequence.capture(before,vec![PresentationFrame{action:1,observation:Rc::new(after)}],&[cue],profile);
  let early=sequence.view(324.).unwrap();assert_eq!(early.creatures[0].hp,10.);assert_eq!(early.cells.iter().find(|c|c.x==5).unwrap().glyph,"v");assert_eq!(early.hp,50.);assert!(!early.dead);assert!(early.player_statuses.is_empty());
  let middle=sequence.view(325.).unwrap();assert!(middle.creatures.is_empty());assert_eq!(middle.cells.iter().find(|c|c.x==5).unwrap().glyph,"x");assert_eq!(middle.hp,50.);assert!(!middle.dead);assert!(middle.player_statuses.is_empty());
  assert_eq!(sequence.view(649.).unwrap().hp,50.);
  let hit=sequence.view(650.).unwrap();assert_eq!(hit.hp,0.);assert!(hit.dead);assert_eq!(hit.player_statuses[0].name,"Stunned");
  // Confirm the renderer's wave actually lights each recipient at these boundaries.
  assert!(!crate::divine_animation_shapes::tiles(&event,324.,false,[0,0,0]).unwrap().iter().any(|t|t.x==5));
  assert!(crate::divine_animation_shapes::tiles(&event,325.,false,[0,0,0]).unwrap().iter().any(|t|t.x==5));
  assert!(crate::divine_animation_shapes::tiles(&event,650.,false,[0,0,0]).unwrap().iter().any(|t|t.x==10));
 }
}

#[cfg(test)] mod telepathy_wave_tests {
 use super::*;
 use corpse_core::events::{AnimationCue,Delivery,EventPoint,PresentationEvent,Stage};
 #[test] fn hidden_identity_appears_on_blue_wave_without_final_ui_leak_or_layer_loss(){
  let run:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
  let profile=crate::appearance_profile::Profile::defaults();let mut before=observe(&run,profile);
  before.cells.clear();before.creatures.clear();before.log=vec!["Before".into()];
  let mut after=before.clone();after.log=vec!["Future".into()];
  after.cells=vec![crate::render::Cell{x:5,y:0,glyph:".".into(),..Default::default()},crate::render::Cell{x:5,y:0,glyph:"B".into(),actor_id:Some(2.),..Default::default()}];
  after.creatures=vec![crate::render::Creature { perception_range:None,identity_outline:None,hallucination:None,shared_resources:false,invulnerable:false,id:2.,name:"Bear".into(),glyph:"B".into(),color:[255,255,255],state:"Sleeping".into(),distance:5.,hp:10.,max_hp:10.,energy:0.,max_energy:0.,uses_energy:false,statuses:vec![],meters:vec![] }];
  let event=PresentationEvent{kind:"animation".into(),from:None,to:EventPoint{x:0.,y:0.},glyph:None,color:None,label:None,depth:1.,animation:Some(AnimationCue{proposal:"potion-wave".into(),style:"wave-reveal-telepathy".into(),stage:Stage::Special,action:1,cells:(0..=10).map(|x|EventPoint{x:x as f64,y:0.}).collect(),delivery:Delivery::Local,surface:None,item_id:None,hide_actor:None,arrival_steps:None})};
  let mut sequence=Sequence::default();sequence.capture(before,vec![PresentationFrame{action:1,observation:Rc::new(after)}],&[Cue{event:event.clone(),started:0.,held:false,static_marks:false}],profile);
  assert!(sequence.view(324.).unwrap().creatures.is_empty());assert!(sequence.view(324.).unwrap().cells.is_empty());
  let reached=sequence.view(325.).unwrap();assert_eq!(reached.creatures[0].name,"Bear");assert_eq!(reached.cells.len(),2);assert_eq!(reached.log,vec!["Before"]);assert!(reached.terrain_snapshot.is_none());
  assert!(crate::divine_animation_shapes::tiles(&event,325.,false,[0,0,0]).unwrap().iter().any(|t|t.x==5&&t.color==[45,140,255]));
  assert!(sequence.pending(899.));assert!(!sequence.pending(900.));
 }
 #[test] fn shattering_wall_remains_during_crystal_flash(){
  let run:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();let profile=crate::appearance_profile::Profile::defaults();
  let mut before=observe(&run,profile);before.cells=vec![crate::render::Cell{x:0,y:0,glyph:"#".into(),..Default::default()}];let mut after=before.clone();after.cells[0].glyph=".".into();
  let event=PresentationEvent{kind:"animation".into(),from:None,to:EventPoint{x:0.,y:0.},glyph:None,color:None,label:None,depth:1.,animation:Some(AnimationCue{proposal:"potion-wave".into(),style:"wave-reveal-shattering".into(),stage:Stage::Special,action:1,cells:vec![EventPoint{x:0.,y:0.}],delivery:Delivery::Local,surface:None,item_id:None,hide_actor:None,arrival_steps:None})};
  let mut sequence=Sequence::default();sequence.capture(before,vec![PresentationFrame{action:1,observation:Rc::new(after)}],&[Cue{event,started:0.,held:false,static_marks:false}],profile);
  assert_eq!(sequence.view(119.).unwrap().cells[0].glyph,"#");assert_eq!(sequence.view(120.).unwrap().cells[0].glyph,".");
 }
}

#[cfg(test)]
mod beckon_tests {
    use super::*;
    use corpse_core::{
        animation_shapes::BECKON_BEAM,
        events::{AnimationCue, Delivery, EventPoint, PresentationEvent, Stage},
    };

    #[test]
    fn beckon_hides_one_layer_without_hiding_the_terrain_or_nearby_actors() {
        let run: corpse_core::Run =
            serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let profile = crate::appearance_profile::Profile::defaults();
        let mut before = observe(&run, profile);
        before.cells = vec![
            crate::render::Cell { x:5, y:1, glyph:".".into(), ..Default::default() },
            crate::render::Cell { x:5, y:1, glyph:"r".into(), actor_id:Some(9.), ..Default::default() },
            crate::render::Cell { x:5, y:1, glyph:"B".into(), actor_id:Some(8.), ..Default::default() },
        ];
        before.terrain_count = 1;
        before.hallucination_cells.clear();
        let mut after = before.clone();
        after.cells[1].x = 7;
        let event = PresentationEvent {
            kind:"animation".into(), from:None, to:EventPoint { x:8., y:1. },
            glyph:None, color:None, label:None, depth:1.,
            animation:Some(AnimationCue {
                delivery:Delivery::Magic, proposal:"A07".into(), style:BECKON_BEAM.into(),
                stage:Stage::Travel, action:1, cells:vec![EventPoint {x:5.,y:1.},EventPoint {x:8.,y:1.}],
                hide_actor:Some(9.), surface:None, item_id:None, arrival_steps:None,
            }),
        };
        let end = 100. + duration(&event);
        let cue = Cue {event:event.clone(), started:100., held:false, static_marks:false};
        let mut secondary = cue.clone();
        secondary.started = end;
        let hud = secondary.event.animation.as_mut().unwrap();
        hud.stage = Stage::Health;
        hud.hide_actor = None;
        let mut sequence = Sequence::default();
        sequence.capture(before.clone(), vec![PresentationFrame {action:1, observation:Rc::new(after.clone())}],
            &[cue.clone(),secondary], profile);
        let ids = |view:&View| view.cells.iter().map(|c|c.actor_id).collect::<Vec<_>>();
        assert_eq!(ids(sequence.view(99.).unwrap()), vec![None,Some(9.),Some(8.)]);
        assert_eq!(ids(sequence.view(100.).unwrap()), vec![None,Some(8.)]);
        assert_eq!(ids(sequence.view(end-1.).unwrap()), vec![None,Some(8.)]);
        assert_eq!(sequence.view(end).unwrap().cells[1].x, 7);
        sequence.shift(75.);
        assert_eq!(ids(sequence.view(174.).unwrap()), vec![None,Some(9.),Some(8.)]);
        assert_eq!(ids(sequence.view(175.).unwrap()), vec![None,Some(8.)]);

        let reduced = Cue {static_marks:true,..cue};
        sequence.capture(before, vec![PresentationFrame {action:1, observation:Rc::new(after)}], &[reduced], profile);
        assert_eq!(ids(sequence.view(100.).unwrap()), vec![None,Some(9.),Some(8.)]);
    }
}
