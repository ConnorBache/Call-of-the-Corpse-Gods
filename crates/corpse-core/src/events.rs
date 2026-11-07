//! Non-authoritative command-scoped presentation output. Exact source filtering and bounds.
use crate::{Actor, Run, vision};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EventPoint {
    pub x: f64,
    pub y: f64,
}
impl From<&Actor> for EventPoint {
    fn from(a: &Actor) -> Self {
        Self { x: a.x, y: a.y }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PresentationEvent {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<EventPoint>,
    pub to: EventPoint,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glyph: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub depth: f64,
}
#[derive(Default, Clone)]
struct Batch {
    events: Vec<PresentationEvent>,
    explosions: indexmap::IndexMap<String, PresentationEvent>,
}
thread_local! {static PENDING:std::cell::RefCell<Option<Batch>>=const{std::cell::RefCell::new(None)};}
pub(crate) fn begin() {
    PENDING.with(|p| *p.borrow_mut() = Some(Batch::default()));
}
pub(crate) fn finish() -> Vec<PresentationEvent> {
    PENDING.with(|p| {
        p.borrow_mut()
            .take()
            .map(|mut b| {
                b.events.extend(b.explosions.into_values());
                b.events
            })
            .unwrap_or_default()
    })
}
pub fn emit(r: &Run, mut event: PresentationEvent) {
    if r.player.x < 0.0 || !vision::visible(r, event.to.x as usize, event.to.y as usize) {
        return;
    }
    if event
        .from
        .as_ref()
        .is_some_and(|p| !vision::visible(r, p.x as usize, p.y as usize))
    {
        event.from = None;
    }
    PENDING.with(|p| {
        let mut p = p.borrow_mut();
        let Some(b) = p.as_mut() else { return };
        if event.kind == "burst" && event.glyph.as_deref() == Some("人") {
            b.explosions.insert(
                format!("{}:{},{}", event.depth, event.to.x, event.to.y),
                event,
            );
        } else {
            b.events.push(event);
            if b.events.len() > 128 {
                b.events.remove(0);
            }
        }
    });
}
pub fn actor_event(
    r: &Run,
    a: &Actor,
    kind: &str,
    label: &str,
    target: Option<EventPoint>,
    glyph: Option<&str>,
) {
    if r.player.x < 0.0 || (a.id != r.player.id && !vision::visible_actor(r, a)) {
        return;
    }
    emit(
        r,
        PresentationEvent {
            kind: kind.into(),
            from: Some(a.into()),
            to: target.unwrap_or_else(|| a.into()),
            glyph: Some(
                glyph
                    .unwrap_or(if kind == "projectile" { "·" } else { "*" })
                    .into(),
            ),
            color: None,
            label: Some(if label == a.name {
                crate::actor_presentation::name(r, a).into()
            } else { label.into() }),
            depth: r.depth,
        },
    );
}
pub fn strike_ranged(r: &Run, a: &Actor, b: &Actor, hit: bool, n: f64, range: f64) {
    let source = a.id == r.player.id || vision::visible_actor(r, a);
    let target = b.id == r.player.id || vision::visible_actor(r, b);
    if !source && !target {
        return;
    }
    emit(
        r,
        PresentationEvent {
            kind: if range > 0.0 && source {
                "projectile"
            } else {
                "impact"
            }
            .into(),
            from: source.then(|| a.into()),
            to: if target { b.into() } else { a.into() },
            glyph: Some(
                if range > 0.0 {
                    "·"
                } else if hit {
                    "*"
                } else {
                    "×"
                }
                .into(),
            ),
            color: None,
            label: Some(if hit { n.to_string() } else { "Miss".into() }),
            depth: r.depth,
        },
    );
}

#[cfg(test)]
mod presentation_identity_tests {
    use super::*;
    #[test]
    fn exact_actor_labels_keep_distinct_disguises_without_changing_event_geometry() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=33.0.into();r.player.x=2.;r.player.y=2.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
        let f=r.floor_mut().unwrap();let w=f.width();let h=f.height();
        f.tiles=vec!["floor".into();w*h];f.lights=vec![100.;w*h];
        let a=crate::population::create_actor_revision("skin-walker",4.,6.,2.,false,33.).unwrap();
        let b=crate::population::create_actor_revision("skin-walker",5.,6.,4.,false,33.).unwrap();
        let n1=crate::actor_presentation::name(&r,&a).to_string();let n2=crate::actor_presentation::name(&r,&b).to_string();assert_ne!(n1,n2);
        let before=r.clone();begin();actor_event(&r,&a,"summon",&a.name,None,None);actor_event(&r,&b,"summon",&b.name,None,None);let events=finish();
        assert_eq!(events.len(),2);assert_eq!(events[0].label.as_deref(),Some(n1.as_str()));assert_eq!(events[1].label.as_deref(),Some(n2.as_str()));
        assert_eq!(events[0].to,EventPoint{x:6.,y:2.});assert_eq!(events[1].to,EventPoint{x:6.,y:4.});assert_eq!(events[0].glyph.as_deref(),Some("*"));assert_eq!(r,before);
        r.simulation_revision=32.0.into();begin();actor_event(&r,&a,"summon",&a.name,None,None);assert_eq!(finish()[0].label.as_deref(),Some("Skin Walker"));
    }
}

/// Nested presentation transaction. Clones the actual bounded event list and
/// ordered explosion map, so accepted emissions retain the exact original
/// append/cap/dedup semantics; no flattened merge. Not Send across TLS owners.
pub(crate) struct Probe {
    saved: Option<Option<Batch>>,
    _thread: std::marker::PhantomData<std::rc::Rc<()>>,
}
impl Probe {
    /// O(E+B) clone for E ordinary events (<=128), B pending explosion entries.
    /// Clone precedes mutation, so allocation panic leaves the outer batch intact.
    pub(crate) fn begin()->Self {
        let saved=PENDING.with(|p| {let mut slot=p.borrow_mut();let fork=slot.clone();std::mem::replace(&mut *slot,fork)});
        Self{saved:Some(saved),_thread:std::marker::PhantomData}
    }
    /// O(E+B) dropping saved prefix; accepted TLS batch already has exact order.
    pub(crate) fn commit(mut self){self.saved.take();}
}
impl Drop for Probe {
    /// O(E+B) discarded batch drop; restores original even during error/unwind.
    fn drop(&mut self){if let Some(saved)=self.saved.take(){PENDING.with(|p|*p.borrow_mut()=saved);}}
}
#[cfg(test)]mod probe_tests {
 use super::*;
 /// O(1) event fixture, no visibility/game state involved in transaction test.
 fn event(label:&str)->PresentationEvent{PresentationEvent{kind:"status".into(),from:None,to:EventPoint{x:1.,y:1.},glyph:None,color:None,label:Some(label.into()),depth:1.}}
 /// O(1) test injection into the actual pending presentation representation.
 fn inject(label:&str,key:Option<&str>){PENDING.with(|p|{let mut p=p.borrow_mut();let b=p.as_mut().unwrap();if let Some(key)=key{b.explosions.insert(key.into(),event(label));}else{b.events.push(event(label));}});}
 /// O(E+B), nested discard and unwind preserve the exact outer prefix/dedup state.
 #[test]fn probe_drop_nested_and_unwind_restore_outer(){
  begin();inject("outer",None);inject("old blast",Some("x"));
  {let _probe=Probe::begin();inject("discard",None);{let nested=Probe::begin();inject("nested accepted",Some("x"));nested.commit();}}
  let _=std::panic::catch_unwind(||{let _probe=Probe::begin();inject("panic",None);panic!("test unwind");});
  assert_eq!(finish(),vec![event("outer"),event("old blast")]);
 }
 /// O(E+B), accepted overwrite retains first explosion-key position, not flattened duplicates.
 #[test]fn accepted_probe_keeps_ordered_explosion_replacement(){
  begin();inject("outer",None);inject("old x",Some("x"));inject("old y",Some("y"));
  let probe=Probe::begin();inject("new ordinary",None);inject("new x",Some("x"));inject("new z",Some("z"));probe.commit();
  assert_eq!(finish(),vec![event("outer"),event("new ordinary"),event("new x"),event("old y"),event("new z")]);
 }
 /// O(1), absent outer recording stays absent after commit or discard.
 #[test]fn no_outer_batch_is_not_fabricated(){finish();{let probe=Probe::begin();probe.commit();}assert!(finish().is_empty());{let _probe=Probe::begin();}assert!(finish().is_empty());}
 /// O(128+visibility), accepted probe preserves the real emit ordinary-event cap.
 #[test]fn accepted_probe_uses_existing_emit_cap(){
  let r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
  begin();for i in 0..128{inject(&i.to_string(),None);}
  let probe=Probe::begin();let mut e=event("last");e.to=EventPoint::from(&r.player);emit(&r,e.clone());probe.commit();
  let events=finish();assert_eq!(events.len(),128);assert_eq!(events.first().unwrap().label.as_deref(),Some("1"));assert_eq!(events.last(),Some(&e));
 }

}
