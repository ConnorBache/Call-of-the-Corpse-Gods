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
}
impl Sequence {
    pub fn clear(&mut self) { *self = Self::default(); }
    pub fn capture_run(&mut self, before: corpse_core::Run, frames: Vec<PresentationFrame>, cues: &[Cue], profile: &crate::appearance_profile::Profile) {
        if cues.is_empty() { self.clear(); return; }
        self.capture(observe(&before,profile), frames, cues, profile);
    }
    pub fn capture(&mut self, before: View, frames: Vec<PresentationFrame>, cues: &[Cue], profile: &crate::appearance_profile::Profile) {
        self.clear();
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
        for (index, frame) in frames.into_iter().enumerate() {
            if !selected.contains(&index) { continue; }
            if let Some(run) = frame.observation.downcast_ref::<corpse_core::Run>() {
                self.frames.push((frame.action, Rc::new(observe(run,profile))));
            } else if let Ok(view) = frame.observation.downcast::<View>() {
                self.frames.push((frame.action,view));
            }
        }
        for cue in cues {
            let end = cue.started + duration(&cue.event);
            self.finish = self.finish.max(end);
            if let Some(a) = &cue.event.animation {
                if let Some((_, until)) = self.ends.iter_mut().find(|(id,_)| *id == a.action) { *until = until.max(end); }
                else { self.ends.push((a.action,end)); }
            }
        }
    }
    pub fn shift(&mut self, amount: f64) { self.finish += amount; for (_,end) in &mut self.ends { *end += amount; } }
    pub fn active(&self) -> bool { self.before.is_some() }
    pub fn pending(&self, now: f64) -> bool { self.before.is_some() && now < self.finish }
    /// O(F*A), bounded by 128 frames and 4096 receipts, no view clones per paint.
    /// A frame can contain quiet actions too, but never an unfinished action.
    pub fn view(&self, now: f64) -> Option<&View> {
        if !self.pending(now) { return None; }
        self.frames.iter().rev().find(|(id,_)| self.ends.iter().all(|(action,end)| action > id || now >= *end))
            .map(|(_,v)|v.as_ref()).or(self.before.as_deref())
    }
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
