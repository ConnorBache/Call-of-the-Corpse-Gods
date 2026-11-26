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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation: Option<AnimationCue>,
}
#[derive(Default, Clone)]
struct Batch {
    descents: PlayerDescents,
    action: u32,
    player_action:u32,
    next_action: u32,
    serial:u64,
    frames: Vec<PresentationFrame>,
    chilled_applied:Vec<f64>,
    attacks: Vec<u32>,
    damage: std::collections::HashMap<u64,u64>,
    route: Option<(f64, Vec<EventPoint>, Option<(String,String)>)>,
    events: Vec<PresentationEvent>,
    explosions: indexmap::IndexMap<String, PresentationEvent>,
}
thread_local! {static PENDING:std::cell::RefCell<Option<Batch>>=const{std::cell::RefCell::new(None)};}
/// Command-local committed travel causes; never serialized or emitted as cues.
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
pub struct PlayerDescents { pub stairs:bool, pub fall:bool, pub other:bool }
impl PlayerDescents {
    pub fn stairs_only(self)->bool {self.stairs&&!self.fall&&!self.other}
}
/// O(1), opt-in presentation bookkeeping. Speculative probes clone/restore this
/// with their existing Batch, so rejected routes cannot suppress a real fall.
pub(crate) fn player_descent(stairs:bool,fall:bool) {
    if !OBSERVER.with(|o|o.borrow().is_some()){return;}
    PENDING.with(|p|if let Some(b)=p.borrow_mut().as_mut(){
        b.descents.stairs|=stairs&&!fall;b.descents.fall|=fall;b.descents.other|=!stairs&&!fall;
    });
}
/// Opaque, detached presentation checkpoints supplied by the host, never serialized in Run.
/// The core does not depend on a renderer or delay simulation for an animation.
#[derive(Clone)]
pub struct PresentationFrame { pub action: u32, pub observation: std::rc::Rc<dyn std::any::Any> }
type Observer = std::rc::Rc<dyn Fn(&Run) -> std::rc::Rc<dyn std::any::Any>>;
thread_local! {
    static OBSERVER: std::cell::RefCell<Option<Observer>> = const { std::cell::RefCell::new(None) };
    static DESCENTS: std::cell::Cell<PlayerDescents> = const { std::cell::Cell::new(PlayerDescents{stairs:false,fall:false,other:false}) };
    static COMPLETED: std::cell::RefCell<Vec<PresentationFrame>> = const { std::cell::RefCell::new(Vec::new()) };
}
/// Scoped opt-in: headless simulation/replays have zero observation work unless
/// their host installs this callback. Probes roll frames back with their receipts.
pub struct PresentationObserver { previous: Option<Observer> }
impl PresentationObserver {
    pub fn install(f: impl Fn(&Run) -> std::rc::Rc<dyn std::any::Any> + 'static) -> Self {
        COMPLETED.with(|c| c.borrow_mut().clear());
        DESCENTS.with(|c|c.set(PlayerDescents::default()));
        Self { previous: OBSERVER.with(|o| o.replace(Some(std::rc::Rc::new(f)))) }
    }
    pub fn descents(&self)->PlayerDescents {DESCENTS.with(|c|c.get())}
    pub fn take(&self) -> Vec<PresentationFrame> { COMPLETED.with(|c| std::mem::take(&mut *c.borrow_mut())) }
}
impl Drop for PresentationObserver {
    fn drop(&mut self) { OBSERVER.with(|o| *o.borrow_mut() = self.previous.take()); COMPLETED.with(|c| c.borrow_mut().clear()); DESCENTS.with(|c|c.set(PlayerDescents::default())); }
}
/// Capture action boundaries while a host explicitly requests presentation. O(host callback) per action, <=128
/// frames per command; beyond the cap the host holds the last safe observation
/// until the entire command completes. Hosts must knowledge-filter before painting.
pub(crate) fn checkpoint(r: &Run) {
    let action=PENDING.with(|p|p.borrow().as_ref().map_or(0,|b|b.next_action));
    checkpoint_for_action(r,action);
}
/// Capture a wave's completed state under its original action even when nested
/// reactions emitted later actions. Ordered insertion preserves boundary lookup.
pub(crate) fn checkpoint_for_action(r:&Run,action:u32) {
    let wanted = PENDING.with(|p| p.borrow().as_ref().is_some_and(|b| b.frames.len() < 128 || b.frames.iter().any(|f|f.action==action)));
    if !wanted { return; }
    let observer = OBSERVER.with(|o| o.borrow().clone());
    let Some(observer) = observer else { return; };
    let observation = observer(r);
    PENDING.with(|p| if let Some(b) = p.borrow_mut().as_mut() {
        let frame=PresentationFrame{action,observation};
        if let Some(at)=b.frames.iter().position(|f|f.action==action){b.frames[at]=frame;}
        else {let at=b.frames.partition_point(|f|f.action<action);b.frames.insert(at,frame);}
    });
}
pub(crate) fn current_action()->u32 {PENDING.with(|p|p.borrow().as_ref().map_or(0,|b|b.action))}

pub(crate) fn begin() {
    #[cfg(test)] if SUPPRESS_RECORDING.with(|s|s.get()){PENDING.with(|p|*p.borrow_mut()=None);return}
    PENDING.with(|p| *p.borrow_mut() = Some(Batch::default()));
}
pub(crate) fn finish() -> Vec<PresentationEvent> {
    PENDING.with(|p| {
        p.borrow_mut()
            .take()
            .map(|mut b| {
                COMPLETED.with(|c| *c.borrow_mut() = b.frames);
                DESCENTS.with(|c|c.set(b.descents));
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
            b.serial=b.serial.wrapping_add(1);
            b.events.push(event);
            let cap=if enabled(r){4096}else{128};
            if b.events.len() > cap {
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
    // Door state/log updates remain authoritative; opening has no visual cue.
    if label == "Open door" { return; }
    if r.player.x < 0.0 || (a.id != r.player.id && !vision::visible_actor(r, a)) {
        return;
    }
    if enabled(r) {
        if matches!(label,"Wait"|"Recovering"|"Stalking: waiting") {return}
        let classification=crate::animation_enemy::classify(a,kind,label).or_else(||crate::animation_player::classify(a,kind,label));
        let (key,style,stage)=classification.unwrap_or(("shared",if kind=="projectile"{"bolt"}else if kind=="heal"{"stitch"}else{"ripple"},if kind=="projectile"{Stage::Travel}else if kind=="heal"{Stage::Health}else{Stage::Special}));
        let endpoint=target.unwrap_or_else(||a.into());
        if kind!="projectile" && (kind=="heal"||matches!(label,"Held"|"Envenomed"|"Slowed"|"Poisoned"|"Hemophilia")) && (endpoint.x!=a.x||endpoint.y!=a.y) {
            let recipient=std::iter::once(&r.player).chain(r.actors_at(r.depth)).find(|b|b.x==endpoint.x&&b.y==endpoint.y&&crate::dreamwalking::same_plane(r,a,b));
            if recipient.is_none_or(|b|!actor_observed(r,b)){return}
        }
        let cells=if kind=="projectile"{take_path(a)}else{vec![endpoint.clone()]};
        if !cells.is_empty(){let serial=emission_serial();
            if kind=="projectile" {
                // Physical item callers supply the actual visible item glyph;
                // monster spell callers deliberately do not. Spit is physical.
                let physical=glyph.is_some()||key=="E61";
                travel(r,a,key,style,&cells,if physical{Delivery::Projectile}else{Delivery::Magic},
                    glyph.or(if key=="E61"{Some("o")}else{None}));
            } else {cue(r,a,key,style,stage,&cells);}
            if emission_serial()!=serial&&key=="E01"{set_glyph(key,stage,cells.last(),if label=="Vampire Bat"{"V"}else{"^"}.into());}
        }
        else if kind!="projectile" {cue(r,a,key,style,stage,&[endpoint]);}
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
         animation: None, },
    );
}
pub fn strike_ranged(r: &Run, a: &Actor, b: &Actor, hit: bool, n: f64, range: f64) {
    if enabled(r) {
        let tag=path_tag(a);
        let path=take_path(a);if range>0. && !path.is_empty(){
            let (key,style)=tag.as_ref().map_or(("shared","bolt"),|(k,s)|(k.as_str(),s.as_str()));
            let physical=tag.is_none()||key=="E61";
            travel(r,a,key,style,&path,if physical{Delivery::Projectile}else{Delivery::Magic},
                if physical{Some(if key=="E61"{"o"}else{"↑"})}else{None});
        }
        // Range is already resolved gameplay input, not inferred from visibility.
        // Retain target feedback when an unseen source correctly has no route.
        let impact_key=if range>0.{"shared-ranged"}else{"shared"};
        cue(r,b,impact_key,if !hit{"miss"}else if n<=0.{"rejected"}else{"slash"},Stage::Impact,&[b.into()]);
        if hit&&n>0.{cue(r,b,impact_key,"puncture",Stage::Health,&[b.into()]);}
        return;
    }
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
         animation: None, },
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
    /// O(E+B+F) clone for E ordinary events, B explosion entries, F<=128 frame references.
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
 fn event(label:&str)->PresentationEvent{PresentationEvent{kind:"status".into(),from:None,to:EventPoint{x:1.,y:1.},glyph:None,color:None,label:Some(label.into()),depth:1., animation: None, }}
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

/// Detached visual ordering. No field is stored in Run, snapshots or replays.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage { Travel, Impact, Special, Health, Death }
/// What travels, independent of its cosmetic color/style. Local effects and
/// symbolic links never become interpolated projectiles.
#[derive(Default, Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Delivery { #[default] Local, Magic, Projectile }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AnimationCue {
    #[serde(default)]
    pub delivery: Delivery,
    pub proposal: String,
    pub style: String,
    pub stage: Stage,
    pub action: u32,
    /// Exact admitted cells. Never interpolate between them: gaps can be unseen
    /// geometry, a portal jump, or a clipped area. Abstract sound may include fog.
    pub cells: Vec<EventPoint>,
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub surface: Option<String>,
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub item_id: Option<f64>,
    /// Detached actor layer to hide for this cue, never a simulation change.
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub hide_actor: Option<f64>,
    /// Parallel beam or wave arrival bands, indexed by the exact admitted cells.
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub arrival_steps: Option<Vec<u8>>,
}
pub fn enabled(r: &Run) -> bool { r.simulation_revision.value_or(29.) >= 52. }
pub fn next_action(r: &Run) {
    checkpoint(r);
    if enabled(r) { PENDING.with(|p| { if let Some(b)=p.borrow_mut().as_mut() { b.next_action=b.next_action.saturating_add(1); b.action=b.next_action; } }); }
}
pub fn begin_player_action(r:&Run){next_action(r);if enabled(r){PENDING.with(|p|if let Some(b)=p.borrow_mut().as_mut(){b.player_action=b.action;});}}
pub fn player_action()->u32{PENDING.with(|p|p.borrow().as_ref().map_or(0,|b|b.player_action))}
/// Nested physical attacks get distinct IDs, but their triggered effects retain
/// their parent's ID afterward. A primary strike shares its actor-action ID.
pub struct AttackScope { previous:u32, pushed:bool }
impl Drop for AttackScope {fn drop(&mut self){if self.pushed{PENDING.with(|p|if let Some(b)=p.borrow_mut().as_mut(){b.attacks.pop();b.action=self.previous;});}}}
pub fn attack_scope(r:&Run)->AttackScope {
 let mut scope=AttackScope{previous:0,pushed:false};
 if enabled(r){PENDING.with(|p|if let Some(b)=p.borrow_mut().as_mut(){scope.previous=b.action;scope.pushed=true;if b.attacks.contains(&b.action){b.next_action=b.next_action.saturating_add(1);b.action=b.next_action;}b.attacks.push(b.action);});}scope
}
/// Separate second-action prelude (for Bite Again) belongs to the second strike.
pub struct ActionScope { previous:u32, enabled:bool }
impl Drop for ActionScope {fn drop(&mut self){if self.enabled{PENDING.with(|p|if let Some(b)=p.borrow_mut().as_mut(){b.action=self.previous;});}}}
pub fn at_action(r:&Run,action:u32)->ActionScope {let previous=PENDING.with(|p|p.borrow().as_ref().map_or(0,|b|b.action));if enabled(r){PENDING.with(|p|if let Some(b)=p.borrow_mut().as_mut(){b.action=action;});}ActionScope{previous,enabled:enabled(r)}}
pub fn action_scope(r:&Run)->ActionScope {let previous=PENDING.with(|p|p.borrow().as_ref().map_or(0,|b|b.action));next_action(r);ActionScope{previous,enabled:enabled(r)}}
pub fn record_damage(r:&Run,id:f64,n:f64){if enabled(r)&&n>0.{PENDING.with(|p|if let Some(b)=p.borrow_mut().as_mut(){*b.damage.entry(id.to_bits()).or_default()+=1;});}}
pub fn damage_count(id:f64)->u64{PENDING.with(|p|p.borrow().as_ref().and_then(|b|b.damage.get(&id.to_bits()).copied()).unwrap_or(0))}
/// Target-local events require an independently observed recipient. Terrain and
/// committed projectiles use at()/cue() separately and never query occupants.
pub fn actor_observed(r:&Run,a:&Actor)->bool {crate::dreamwalking::same_plane(r,&r.player,a)&&(a.id==r.player.id||vision::all_seeing(r)||vision::directly_visible_actor(r,a))}
fn observed_cell(r: &Run, p: &EventPoint) -> bool {
    p.x.is_finite() && p.y.is_finite() && p.x>=0. && p.y>=0.
        && p.x.fract()==0. && p.y.fract()==0.
        && vision::visible(r,p.x as usize,p.y as usize)
}
/// O(C * visibility-query) time and O(C) detached scratch for C admitted cells.
/// The route is captured before simulation advances, not reconstructed afterward.
pub fn at(r:&Run, proposal:&str, style:&str, stage:Stage, cells:&[EventPoint]) {
    if !enabled(r) || crate::dreamwalking::context_dream(r)!=crate::dreamwalking::dream_actor(r,&r.player) { return; }
    let cells:Vec<_>=cells.iter().filter(|p|observed_cell(r,p) && vision::directly_visible_tile(r,p.x as usize,p.y as usize)).cloned().collect();
    let Some(to)=cells.last().cloned() else{return};
    let action=PENDING.with(|p|p.borrow().as_ref().map_or(0,|b|b.action));
    emit(r, PresentationEvent { kind:"animation".into(),from:None,to,glyph:None,color:None,
        label:None,depth:r.depth,animation:Some(AnimationCue{delivery:Delivery::Local,proposal:proposal.into(),style:style.into(),stage,action,cells,surface:None,item_id:None,hide_actor:None,arrival_steps:None}) });
}
/// Actor identity and plane admission precede all actor-related visual output.
/// Terrain-only at() must not be used to bypass this guard for hidden actors.
pub fn cue(r:&Run,a:&Actor,proposal:&str,style:&str,stage:Stage,cells:&[EventPoint]) {
    if !enabled(r) || !crate::dreamwalking::same_plane(r,&r.player,a)
        || (a.id!=r.player.id && !(vision::all_seeing(r)||vision::directly_visible_actor(r,a))) {return}
    if a.id!=r.player.id && proposal.starts_with('E') && !matches!(proposal,"E70"|"E71"|"E72")
        && crate::half_here::origins(r).iter().any(|b|crate::engine::condition(b,"hallucinating")>0.) {return}
    let serial=emission_serial();
    at(r,proposal,style,stage,cells);
    if emission_serial()==serial{return}
    // Only the ordinary corpse collapse is cosmetic. Player/owned endpoint
    // death and separately authored death abilities keep their own timing.
    if stage==Stage::Death && style=="collapse" && crate::half_here::owner_id(r,a)!=r.player.id {
        PENDING.with(|p|if let Some(e)=p.borrow_mut().as_mut().and_then(|b|b.events.last_mut()){e.kind="cosmetic-death".into();});
    }
    if (stage==Stage::Death&&style!="ripple") || style.contains("fold") || style=="collapse" {
        let glyph=crate::actor_presentation::apparent_kind(r,a).and_then(|kind|crate::population::catalog().actors.get(kind)).map(|t|t.glyph.clone())
            .unwrap_or_else(||if crate::monster_vampire::flying(a){"b".into()}else{a.glyph.clone()});
        set_glyph(proposal,stage,cells.last(),glyph);
    }
}
/// Explicit travel semantics, recorded only after the normal visibility/identity
/// guard succeeds. Receipts remain detached from gameplay and are never saved.
pub fn travel(r:&Run,a:&Actor,proposal:&str,style:&str,cells:&[EventPoint],delivery:Delivery,glyph:Option<&str>){
    let serial=emission_serial();
    cue(r,a,proposal,style,Stage::Travel,cells);
    if emission_serial()==serial{return}
    PENDING.with(|p|if let Some(e)=p.borrow_mut().as_mut().and_then(|b|b.events.last_mut()){
        if let Some(cue)=e.animation.as_mut(){cue.delivery=delivery;}
        if delivery==Delivery::Projectile{e.glyph=glyph.map(str::to_owned);}
    });
}
/// Assign travel bands only to the receipt just admitted, using its filtered
/// cells. Same-band cells advance in parallel rather than in row order.
pub(crate) fn travel_bands(r:&Run,a:&Actor,proposal:&str,style:&str,cells:&[EventPoint],band:impl Fn(&EventPoint)->u8) {
    let serial=emission_serial();
    travel(r,a,proposal,style,cells,Delivery::Magic,None);
    if emission_serial()==serial {return;}
    PENDING.with(|p|if let Some(cue)=p.borrow_mut().as_mut().and_then(|b|b.events.last_mut()).and_then(|e|e.animation.as_mut()){
        cue.arrival_steps=Some(cue.cells.iter().map(band).collect());
    });
}
/// Decorate an admitted cue without inventing cells or changing simulation state.
/// Exact receipt identity prevents a filtered cue from decorating an earlier event.
pub fn decorate(since:u64,proposal:&str,style:&str,center:EventPoint,color:&str,from:Option<EventPoint>) {
    if emission_serial()==since{return;}
    PENDING.with(|p|if let Some(e)=p.borrow_mut().as_mut().and_then(|b|b.events.last_mut()) {
        if e.animation.as_ref().is_some_and(|a|a.proposal==proposal&&a.style==style) {
            e.to=center;e.color=Some(color.into());e.from=from;
        }
    });
}
/// Attach an exact actor mask only to a newly admitted receipt.
pub(crate) fn hide_actor_during(since:u64, actor:&Actor) {
    if emission_serial()==since{return;}
    PENDING.with(|p|if let Some(cue)=p.borrow_mut().as_mut()
        .and_then(|b|b.events.last_mut()).and_then(|e|e.animation.as_mut()){
        cue.hide_actor=Some(actor.id);
    });
}
/// Known launched-item identity for detached landing presentation. This annotates
/// the current action's admitted physical route only; no world or RNG mutation.
pub(crate) fn projectile_item(r:&Run,item_id:f64){
    if !enabled(r){return}
    PENDING.with(|p|if let Some(b)=p.borrow_mut().as_mut(){
        let action=b.action;
        if let Some(a)=b.events.iter_mut().rev().filter_map(|e|e.animation.as_mut())
            .find(|a|a.action==action&&a.stage==Stage::Travel&&a.delivery==Delivery::Projectile){a.item_id=Some(item_id);}
    });
}
pub fn emission_serial()->u64{PENDING.with(|p|p.borrow().as_ref().map_or(0,|b|b.serial))}
fn set_glyph(proposal:&str,stage:Stage,last:Option<&EventPoint>,glyph:String){PENDING.with(|p|{if let Some(e)=p.borrow_mut().as_mut().and_then(|b|b.events.last_mut()){
    if e.animation.as_ref().is_some_and(|a|a.proposal==proposal&&a.stage==stage)&&last==Some(&e.to){e.glyph=Some(glyph);}
}});}
/// Preserve a real committed ray for the next corresponding projectile event.
/// No simulation RNG is consulted here. Pure preview rays never call this.
pub(crate) fn committed_path(r:&Run,a:&Actor,points:&[crate::movement::Point]) {
    if !enabled(r) {return}
    let cells=if a.id==r.player.id || vision::visible_actor(r,a) {
        points.iter().map(|p|EventPoint{x:p.x as f64,y:p.y as f64}).filter(|p|observed_cell(r,p)).collect()
    } else {Vec::new()};
    PENDING.with(|p|if let Some(b)=p.borrow_mut().as_mut(){b.route=Some((a.id,cells,None));});
}
pub(crate) fn take_path(a:&Actor)->Vec<EventPoint> {
    PENDING.with(|p|{let mut p=p.borrow_mut();let Some(b)=p.as_mut()else{return vec![]};
        if b.route.as_ref().is_some_and(|(id,_,_)|*id==a.id){b.route.take().unwrap().1}else{vec![]}})
}

/// Label the already committed geometry, without retracing or consulting RNG.
pub fn tag_path(a:&Actor,key:&str,style:&str){PENDING.with(|p|{if let Some(b)=p.borrow_mut().as_mut(){if let Some((id,_,tag))=&mut b.route{if *id==a.id{*tag=Some((key.into(),style.into()));}}}});}
fn path_tag(a:&Actor)->Option<(String,String)>{PENDING.with(|p|p.borrow().as_ref().and_then(|b|b.route.as_ref()).filter(|(id,_,_)|*id==a.id).and_then(|(_,_,tag)|tag.clone()))}

/// Player-known inventory feedback. Never paints a world cell or reveals an item
/// outside the player's inventory; the caller owns successful outcome admission.
pub fn inventory(r:&Run,proposal:&str,style:&str,item_id:Option<f64>){
    if !enabled(r)||r.player.x<0.{return}
    let action=PENDING.with(|p|p.borrow().as_ref().map_or(0,|b|b.action));
    let event=PresentationEvent{kind:"animation".into(),from:None,to:(&r.player).into(),glyph:None,color:None,label:None,depth:r.depth,
        animation:Some(AnimationCue{delivery:Delivery::Local,proposal:proposal.into(),style:style.into(),stage:if proposal=="P65"&&style=="gear"{Stage::Travel}else{Stage::Special},action,cells:vec![],surface:Some("inventory".into()),item_id,hide_actor:None,arrival_steps:None})};
    PENDING.with(|p|{if let Some(b)=p.borrow_mut().as_mut(){b.serial=b.serial.wrapping_add(1);b.events.push(event);if b.events.len()>4096{b.events.remove(0);}}});
}

#[cfg(test)]
thread_local! {static SUPPRESS_RECORDING:std::cell::Cell<bool>=const{std::cell::Cell::new(false)};}
/// Test-only recording suppression. It changes no Run or physics branch.
#[cfg(test)]
pub(crate) fn without_recording<T>(f:impl FnOnce()->T)->T{
    struct Guard(bool);impl Drop for Guard{fn drop(&mut self){SUPPRESS_RECORDING.with(|s|s.set(self.0));}}
    let guard=Guard(SUPPRESS_RECORDING.with(|s|s.replace(true)));let result=f();drop(guard);result
}

/// Already-disclosed sound location only: no source identity, terrain, path or
/// hidden target enters this receipt. The Banshee mechanic owns audible admission.
pub fn audible_banshee(r:&Run,a:&Actor){
    if !enabled(r)||r.player.x<0.||!crate::dreamwalking::same_plane(r,&r.player,a)||!crate::lifecycle::audible_banshee(r,a){return}
    let to=EventPoint{x:crate::engine::condition(a,"monsterAudibleX"),y:crate::engine::condition(a,"monsterAudibleY")};
    let action=PENDING.with(|p|p.borrow().as_ref().map_or(0,|b|b.action));
    let event=PresentationEvent{kind:"animation".into(),from:None,to:to.clone(),glyph:None,color:None,label:None,depth:r.depth,animation:Some(AnimationCue{delivery:Delivery::Local,proposal:"E11".into(),style:"sound-violet".into(),stage:Stage::Special,action,cells:vec![to],surface:Some("audible".into()),item_id:None,hide_actor:None,arrival_steps:None})};
    PENDING.with(|p|if let Some(b)=p.borrow_mut().as_mut(){b.serial=b.serial.wrapping_add(1);b.events.push(event);if b.events.len()>4096{b.events.remove(0);}});
}

/// One terrain-agnostic sound front reaches all tiles, even in fog. Its purple
/// overlay is limited to known optical cells, never emitting hidden terrain or
/// witness identities. O(V * (visibility + LOS)) time and O(V) detached space.
pub fn banshee_wave(r: &Run, a: &Actor) {
    use crate::banshee_rules::{PURPLE_WAVE, WHITE_WAVE};
    if !enabled(r) || r.player.x < 0. || !crate::dreamwalking::same_plane(r, &r.player, a)
        || !PENDING.with(|p| p.borrow().is_some()) { return; }
    let Some(f) = crate::dreamwalking::floor_for(r, a) else { return };
    let w = f.width();
    if w == 0 || f.height() == 0 { return; }
    let center = EventPoint::from(a);
    let far = a.x.max(w as f64 - 1. - a.x).hypot(a.y.max(f.height() as f64 - 1. - a.y)).max(1.);
    let band = |x: f64, y: f64| (((x-a.x).hypot(y-a.y)/far*10. - 1e-9).ceil().clamp(0.,10.)) as u8;
    let mut white = Vec::with_capacity(f.tiles.len());
    let mut white_bands = Vec::with_capacity(f.tiles.len());
    let mut purple = Vec::new();
    let mut purple_bands = Vec::new();
    let mut point = a.clone();
    for i in 0..f.tiles.len() {
        (point.x, point.y) = ((i % w) as f64, (i / w) as f64);
        let wave_band = band(point.x, point.y);
        white.push(EventPoint::from(&point));
        white_bands.push(wave_band);
        if vision::directly_visible_tile(r, i % w, i / w) && vision::line_of_sight(r, a, &point) {
            purple.push(EventPoint::from(&point));
            purple_bands.push(wave_band);
        }
    }
    PENDING.with(|p| if let Some(b) = p.borrow_mut().as_mut() {
        for (style, cells, bands, surface) in [
            (WHITE_WAVE, white, white_bands, Some("audible".into())),
            (PURPLE_WAVE, purple, purple_bands, None),
        ] {
            if cells.is_empty() { continue; }
            b.serial = b.serial.wrapping_add(1);
            b.events.push(PresentationEvent {
                kind: "animation".into(), from: None, to: center.clone(), glyph: None, color: None,
                label: None, depth: r.depth, animation: Some(AnimationCue {
                    delivery: Delivery::Local, proposal: "E11".into(), style: style.into(),
                    stage: Stage::Special, action: b.action, cells, surface, item_id: None,
                    hide_actor: None,
                    arrival_steps: Some(bands),
                }),
            });
            if b.events.len() > 4096 { b.events.remove(0); }
        }
    });
}

/// A symbolic transfer records only its two already observed endpoints. It is
/// not a projectile path: the renderer must never interpolate intervening cells.
pub fn link(r:&Run,source:&Actor,target:&Actor,key:&str,style:&str,stage:Stage){
 if !actor_observed(r,source)||!actor_observed(r,target){return}
 if crate::half_here::origins(r).iter().any(|a|crate::engine::condition(a,"hallucinating")>0.)&&source.id!=r.player.id{return}
 let serial=emission_serial();
 cue(r,source,key,style,stage,&[source.into(),target.into()]);
 if emission_serial()==serial{return}
 PENDING.with(|p|if let Some(e)=p.borrow_mut().as_mut().and_then(|b|b.events.last_mut()){
  if e.animation.as_ref().is_some_and(|a|a.proposal==key&&a.style==style)&&e.to==EventPoint::from(target){e.from=Some(source.into());}
 });
}
/// A defender-facing mark never paints the attacker cell. Direction is supplied
/// only for an independently observed source; an unseen hit stays directionless.
pub fn facing(r:&Run,defender:&Actor,attacker:&Actor,key:&str,style:&str,stage:Stage){
 let observed=actor_observed(r,attacker);let serial=emission_serial();cue(r,defender,key,if observed{style}else{"shield"},stage,&[defender.into()]);if emission_serial()==serial||!observed{return}
 PENDING.with(|p|if let Some(e)=p.borrow_mut().as_mut().and_then(|b|b.events.last_mut()){e.from=Some(attacker.into());});
}
/// Old glyph comes from an already-observed apparent-form snapshot, not true kind.
pub fn morph(r:&Run,a:&Actor,key:&str,old_glyph:String){let serial=emission_serial();cue(r,a,key,"fold",Stage::Special,&[a.into()]);if emission_serial()!=serial{set_glyph(key,Stage::Special,Some(&EventPoint::from(a)),old_glyph);}}

/// Detached receipt of a successful real adjacency status application.
pub fn note_chill(r:&Run,id:f64){if enabled(r){PENDING.with(|p|if let Some(b)=p.borrow_mut().as_mut(){if !b.chilled_applied.contains(&id){b.chilled_applied.push(id);}});}}
pub fn chill_applied(id:f64)->bool{PENDING.with(|p|p.borrow().as_ref().is_some_and(|b|b.chilled_applied.contains(&id)))}

#[cfg(test)] mod presentation_observer_tests {
 use super::*;
 #[test] fn wave_checkpoint_retains_original_action_after_nested_reaction(){
  let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();
  let observer=PresentationObserver::install(|r|std::rc::Rc::new(r.player.hp));begin();next_action(&r);let wave=current_action();
  next_action(&r);r.player.hp=42.;checkpoint_for_action(&r,wave);checkpoint(&r);finish();let frames=observer.take();
  assert_eq!(*frames.iter().find(|f|f.action==wave).unwrap().observation.downcast_ref::<f64>().unwrap(),42.);assert!(frames.windows(2).all(|p|p[0].action<p[1].action));
 }
 #[test] fn presentation_observer_probe_rollback_and_headless_cleanup() {
  let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();
  let observer=PresentationObserver::install(|r|std::rc::Rc::new(r.player.hp));
  begin(); next_action(&r); r.player.hp=17.;
  {let _probe=Probe::begin();next_action(&r);r.player.hp=3.;next_action(&r);}
  r.player.hp=11.;checkpoint(&r);finish();
  let frames=observer.take();assert_eq!(frames.len(),2);
  assert_eq!(*frames.last().unwrap().observation.downcast_ref::<f64>().unwrap(),11.);
  drop(observer);begin();next_action(&r);checkpoint(&r);assert!(PENDING.with(|p|p.borrow().as_ref().unwrap().frames.is_empty()));finish();
 }
}

#[cfg(test)] mod quiet_door_and_death_tests {
 use super::*;
 #[test] fn door_is_quiet_without_suppressing_other_same_action_feedback() {
  let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();
  let a=r.player.clone();begin();actor_event(&r,&a,"status","Open door",None,None);
  actor_event(&r,&a,"status","Other effect",None,None);let e=finish();assert_eq!(e.len(),1);assert_eq!(e[0].animation.as_ref().unwrap().style,"ripple");
 }
 #[test] fn death_admission_distinguishes_player_from_other_actors() {
  let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();
  r.player.conditions.insert("allSeeingReveal".into(),1.);r.player.conditions.insert("allSeeingDepth".into(),r.depth);
  r.floor_mut().unwrap().lights.fill(100.);let a=r.player.clone();let mut b=a.clone();b.id=999.;b.state="hunting".into();
  begin();crate::animation_enemy::death(&r,&a);crate::animation_enemy::death(&r,&b);let e=finish();
  assert_eq!(e.len(),2);assert_eq!(e[0].kind,"animation");assert_eq!(e[1].kind,"cosmetic-death");assert_eq!(e[1].animation.as_ref().unwrap().style,"collapse");
 }
}

#[cfg(test)] mod quiet_door_mechanics_test {
 use super::*;
 #[test] fn opening_door_still_moves_and_logs_without_feedback() {
  let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();
  r.player.x=10.;r.player.y=10.;r.player.conditions.clear();r.player.abilities.clear();r.actors_at_mut(r.depth).clear();
  let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.environment=crate::Field::Missing;f.portals=crate::Field::Missing;f.non_euclidean=crate::Field::Missing;
  let cell=10*f.width()+11;let mut doors=indexmap::IndexMap::new();doors.insert(cell.to_string(),crate::model::FloorDoorsEntry{open:false,..Default::default()});f.doors=doors.into();
  let id=r.player.id;begin();assert!(crate::engine::move_actor(&mut r,id,1.,0.,false).unwrap());let e=finish();
  assert!(r.floor().unwrap().doors.as_ref().unwrap()[&cell.to_string()].open);assert_eq!((r.player.x,r.player.y),(11.,10.));
  assert!(e.is_empty(),"door-only movement emitted {e:?}");
 }
}

#[cfg(test)]mod descent_receipt_tests {
 use super::*;
 #[test]fn stairs_pause_receipt_is_opt_in_nonvisual_and_probe_transactional(){
  let observer=PresentationObserver::install(|_|std::rc::Rc::new(()));begin();player_descent(true,false);
  {let _probe=Probe::begin();player_descent(false,true);}
  assert!(finish().is_empty());assert!(observer.descents().stairs_only());
  begin();player_descent(true,false);player_descent(false,true);assert!(finish().is_empty());
  assert!(!observer.descents().stairs_only());assert!(observer.descents().fall);
  begin();player_descent(false,false);finish();assert!(!observer.descents().stairs_only());
  drop(observer);begin();player_descent(true,false);finish();assert_eq!(DESCENTS.with(|c|c.get()),PlayerDescents::default());
 }
}

pub const COMMANDMENT_SLIDE: &str = "moving-actor-commandment";

/// Collect independently admitted travel receipts while forced movement resolves.
/// No final-state visibility recheck: each cell was observed at its actual step.
/// One ordered route, no interpolation across invisible gaps/portal jumps.
#[derive(Default)]
pub(crate) struct TravelRoute { event: Option<PresentationEvent> }
impl TravelRoute {
    pub(crate) fn capture(&mut self,r:&Run,a:&Actor,cells:&[EventPoint],glyph:&str) {
        let serial=emission_serial();
        travel(r,a,"P60",COMMANDMENT_SLIDE,cells,Delivery::Projectile,Some(glyph));
        hide_actor_during(serial,a);
        if emission_serial()==serial{return;}
        let event=PENDING.with(|p|p.borrow_mut().as_mut().and_then(|b|b.events.pop()));
        let Some(mut event)=event else{return};
        // Use the admitted apparent identity, not the true color of a disguise.
        let apparent=crate::actor_presentation::apparent_kind(r,a)
            .and_then(|kind|crate::population::catalog().actors.get(kind));
        event.color=Some(apparent.map_or(&a.color, |body| &body.color).clone());
        if let Some(route)=self.event.as_mut() {
            let cue=route.animation.as_mut().unwrap();
            for cell in &event.animation.as_ref().unwrap().cells {
                if cue.cells.last()!=Some(cell){cue.cells.push(cell.clone());}
            }
            route.to=event.to;
        } else {self.event=Some(event);}
    }
    pub(crate) fn finish(self) {
        if let Some(event)=self.event.filter(|e|e.animation.as_ref().is_some_and(|a|a.cells.len()>1)) {PENDING.with(|p|if let Some(b)=p.borrow_mut().as_mut(){b.events.push(event)});}
    }
}
