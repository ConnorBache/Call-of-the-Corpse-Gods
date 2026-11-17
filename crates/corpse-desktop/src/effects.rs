//! Brogue-style detached feedback: tile illumination or a moving canonical glyph.
//! No simulation access or clocks live here; receipts own geometry and outcomes.
use crate::render::Rgb;
use corpse_core::events::{PresentationEvent, Delivery, Stage};
#[derive(Clone)]
pub struct Cue {
    pub event: PresentationEvent,
    pub started: f64,
    pub held: bool,
    pub static_marks: bool,
}
pub struct Tile {
    pub x: i32,
    pub y: i32,
    pub color: Rgb,
    pub light: f64,
    /// Presentation-only tile field intensity, before the existing lighting filter.
    pub field_strength: f64,
    pub glyph: Option<String>,
    pub mark: Option<crate::action_marks::Mark>,
}
fn explosion(e: &PresentationEvent) -> bool {
    e.kind == "burst" && e.glyph.as_deref() == Some("人")
}
fn color(e: &PresentationEvent) -> Rgb {
    if explosion(e) {
        return [255, 242, 58];
    }
    if let Some(c) = &e.color {
        return crate::color::hex(c);
    }
    match e.label.as_deref().unwrap_or("") {
        "Fire breath" => [255, 118, 46],
        "Moonbeam" | "Charging moonbeam" => [255, 248, 232],
        "Hornlight" => [255, 218, 53],
        "Water jet" | "Water Jet" | "Rushing Torrent" | "Riptide" | "Half Here" => [0, 229, 255],
        "Blood wrest" | "Blood Letting" | "Blood Weapon" => [255, 23, 76],
        "Ruin pulse" | "Ruin Pulse" | "Implant Thought" | "Mind Slave" | "Mind Read" => {
            [182, 77, 255]
        }
        "Bulwark of Mind" | "Healing" => [86, 255, 55],
        _ => match e.kind.as_str() {
            "projectile" | "summon" => [255, 218, 53],
            "impact" => [255, 23, 76],
            "heal" => [86, 255, 55],
            "status" => [0, 229, 255],
            _ => [182, 77, 255],
        },
    }
}
fn flash(age: f64, duration: f64) -> f64 {
    if age < 0. || age >= duration {
        0.
    } else if age <= 18. {
        1.
    } else {
        1. - (age - 18.) / (duration - 18.)
    }
}
pub fn tiles(cue: &Cue, now: f64, reduced: bool) -> Vec<Tile> {
    let age = now - cue.started;
    if cue.event.animation.is_some() { return action_tiles(cue, age, reduced || cue.static_marks); }
    let e = &cue.event;
    let held = cue.held && explosion(e);
    // Ordinary melee stays quiet. Its resulting state is already visible.
    if e.kind == "impact" { return vec![]; }
    if age < 0. || (!held && age >= 240.) {
        return vec![];
    }
    let color = color(e);
    let glyph = e.glyph.clone().unwrap_or_else(|| {
        match e.kind.as_str() {
            "pulse" | "heal" => "+",
            "impact" => "x",
            "summon" => "&",
            "status" => "~",
            _ => "*",
        }
        .into()
    });
    let x = e.to.x.round() as i32;
    let y = e.to.y.round() as i32;
    if reduced {
        return vec![Tile {
            x,
            y,
            color,
            light: 0.3,
            field_strength: 0.32,
            glyph: None,
            mark: None,
        }];
    }
    let mut out = vec![];
    let mut add = |x, y, light, glyph| {
        if light > 0. {
            out.push(Tile {
                x,
                y,
                color,
                light,
                field_strength: 0.32,
                glyph,
                mark: None,
            });
        }
    };
    if e.kind == "projectile" {
        if let Some(from) = &e.from {
            let steps = (e.to.x - from.x).abs().max((e.to.y - from.y).abs()) as usize;
            let head = steps.min((age / 140. * steps as f64).floor() as usize);
            for step in 0..=steps {
                let fraction = if steps > 0 {
                    step as f64 / steps as f64
                } else {
                    1.
                };
                let arrival = if steps > 0 { fraction * 140. } else { 0. };
                add(
                    (from.x + (e.to.x - from.x) * fraction).round() as i32,
                    (from.y + (e.to.y - from.y) * fraction).round() as i32,
                    if step == head { 0.9 } else { flash(age - arrival, 56.) * 0.35 },
                    (step == head).then(|| glyph.clone()),
                );
            }
            return out;
        }
    }
    if held {
        add(x, y, 1., None);
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            add(x + dx, y + dy, 1., None);
        }
        return out;
    }
    let strength = match e.kind.as_str() {
        "status" => 0.4,
        "impact" => 0.65,
        _ => 1.,
    };
    add(x, y, flash(age, 100.) * strength, None);
    if ["burst", "pulse", "heal", "summon"].contains(&e.kind.as_str()) {
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            add(
                x + dx,
                y + dy,
                flash(age - 36., 92.),
                None,
            );
        }
    }
    out
}
pub struct PlaneTransition {pub before:crate::render::View,pub started:f64,pub ascend:bool}
#[derive(Default)]
pub struct Effects {
    pub gameplay_pause:crate::gameplay_pause::Pause,
    pub sequence: crate::action_sequence::Sequence,
    pub plane_transition:Option<PlaneTransition>,
    pub profile:Option<crate::appearance_profile::Profile>,
    pub cues: Vec<Cue>,
    pub firebolt_terrain: crate::staged_firebolt::Staging,
    pending_frame: Option<f64>,
    key: Option<(u64, u64)>,
    reduced: bool,
    instant: bool,
}
impl Effects {
    pub fn configure_motion(&mut self,reduced:bool,instant:bool){self.reduced=reduced;self.instant=instant;for c in &mut self.cues{c.static_marks=reduced||instant;}if reduced||instant{self.plane_transition=None;}}
    pub fn plane_changed(&mut self,before:crate::render::View,now:f64){if !self.reduced&&!self.instant{self.plane_transition=Some(PlaneTransition{before,started:now,ascend:false});}}
    pub fn clear(&mut self) {
        self.gameplay_pause.reset();
        self.sequence.clear();
        self.plane_transition=None;
        self.cues.clear();
        self.firebolt_terrain.clear();
        self.pending_frame=None;
        self.key = None;
    }
    pub fn add(
        &mut self,
        events: Vec<PresentationEvent>,
        tick: f64,
        depth: f64,
        now: f64,
        rewind: bool,
    ) {
        self.sequence.clear();
        self.plane_transition=None;
        self.firebolt_terrain.clear();
        self.pending_frame=None;
        let key = (tick.to_bits(), depth.to_bits());
        if self.key != Some(key) || rewind {
            self.cues.clear();
        }
        self.key = Some(key);
        let mut events=events;
        // A shared strike gets feedback only when it actually traveled. Ability
        // procs have their own receipts; ordinary walking/melee adds no flourish.
        // O(E log E) time / O(E) scratch, bounded by the existing cue cap.
        let ranged: std::collections::BTreeSet<_> = events.iter().filter_map(|e| {
            let a = e.animation.as_ref()?;
            (a.stage == corpse_core::events::Stage::Travel && !a.cells.is_empty()
                && a.delivery!=Delivery::Local).then_some(a.action)
        }).collect();
        events.retain(|e| e.animation.as_ref().is_none_or(|a| {
            // Routine regeneration is not an ability proc and otherwise lights
            // a creature after every ordinary exchange of blows.
            !(a.proposal == "E70" && a.stage == corpse_core::events::Stage::Health && a.style == "droplet")
                && (a.proposal != "shared"
                    || !matches!(a.style.as_str(), "slash" | "puncture" | "miss" | "rejected")
                    || ranged.contains(&a.action))
        }));
        events.sort_by_key(|e|e.animation.as_ref().map(|a|(a.action,a.stage)));
        // O(E log E) schedule: each phase completes before the next phase.
        let mut phase_lengths=std::collections::BTreeMap::new();
        for e in &events {if let Some(a)=&e.animation {let d=phase_lengths.entry((a.action,a.stage)).or_insert(0f64);if a.stage==Stage::Travel {*d+=duration(e)}else{*d=d.max(duration(e));}}}
        let mut starts=std::collections::BTreeMap::new();let mut cursor=now;
        for (key,len) in phase_lengths {starts.insert(key,cursor);cursor+=len;}
        let mut travel_offsets=std::collections::BTreeMap::<u32,f64>::new();
        let mut action_end=now;
        for mut event in events {
            if self.instant {if let Some(a)=event.animation.as_mut(){if a.stage==corpse_core::events::Stage::Travel && !a.style.starts_with("wave") && !a.style.starts_with("fan"){if let Some(last)=a.cells.last().cloned(){a.cells=vec![last];}}}}
            let started=if self.instant {now} else if let Some(a)=&event.animation {let offset=if a.stage==Stage::Travel {let offset=travel_offsets.entry(a.action).or_default();let old=*offset;*offset+=duration(&event);old}else{0.};starts[&(a.action,a.stage)]+offset}else{action_end};
            action_end = action_end.max(started + duration(&event));
            let held = explosion(&event);
            if held {
                self.cues.retain(|c| !c.held || c.event.to != event.to);
            }
            self.cues.push(Cue {
                event,
                started,
                held,
                static_marks:self.reduced||self.instant,
            });
        }
        // Held lines remain visible through every later phase of their action.
        let mut ends=std::collections::BTreeMap::<u32,f64>::new();
        for c in &self.cues {if let Some(a)=&c.event.animation {let end=ends.entry(a.action).or_default();*end=end.max(c.started+duration(&c.event));}}
        for c in &mut self.cues {if let Some(a)=&mut c.event.animation {if a.style=="line-hold"||a.style=="moving-actor" {a.style=format!("{}:{}",a.style,ends[&a.action]-c.started);}}}
        if self.cues.len() > 4096 {
            self.cues.drain(..self.cues.len() - 4096);
        }
    }
    /// Dispatch may take longer than the animation. Start at the first rendered
    /// frame, after simulation finishes; this clock never feeds back into Run.
    pub fn defer_until_frame(&mut self, queued_at:f64){self.pending_frame=Some(queued_at);}
    pub fn begin_frame(&mut self, now:f64){
        if let Some(queued)=self.pending_frame.take(){
            let shift=(now-queued).max(0.);
            self.sequence.shift(shift);
            for cue in &mut self.cues{if cue.started>=queued{cue.started+=shift;}}
            if let Some(t)=&mut self.plane_transition{t.started+=shift;}
        }
    }
    pub fn presentation_pending(&self,now:f64)->bool { self.sequence.pending(now) || self.pending_frame.is_some() || self.animate(now) }
    pub fn travel_pending(&self,now:f64)->bool{ self.presentation_pending(now) }
    pub fn redraw_panels(&self,now:f64)->bool {self.plane_transition.as_ref().is_some_and(|p|now-p.started<250.)||!inventory_marks(&self.cues,now).is_empty()}
    pub fn animate(&self, now: f64) -> bool {
        self.plane_transition.as_ref().is_some_and(|p|now-p.started<250.) || self.cues.iter().any(|c| !c.held && now - c.started < duration(&c.event))
    }
    pub fn prune(&mut self, now: f64) {
        if self.pending_frame.is_none() && !self.sequence.pending(now) { self.sequence.clear(); }
        if self.plane_transition.as_ref().is_some_and(|p|now-p.started>=250.){self.plane_transition=None;}
        self.cues.retain(|c| c.held || now - c.started < duration(&c.event));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use corpse_core::events::EventPoint;
    fn event() -> PresentationEvent {
        PresentationEvent {
            kind: "burst".into(),
            from: None,
            to: EventPoint { x: 2., y: 3. },
            glyph: Some("人".into()),
            color: None,
            label: None,
            depth: 1.,
         animation: None, }
    }
    #[test]
    fn explosions_hold_until_tick_changes() {
        let mut f = Effects::default();
        f.add(vec![event()], 1., 1., 0., false);
        assert_eq!(tiles(&f.cues[0], 9999., false).len(), 5);
        f.add(vec![], 1., 1., 10000., false);
        assert_eq!(f.cues.len(), 1);
        f.add(vec![], 2., 1., 10001., false);
        assert!(f.cues.is_empty());
    }
    #[test]
    fn flash_boundaries() {
        assert_eq!(flash(0., 56.), 1.);
        assert_eq!(flash(18., 56.), 1.);
        assert_eq!(flash(56., 56.), 0.);
        assert_eq!(flash(-1., 56.), 0.);
    }
}

/// Brogue requests 16ms per visible bolt cell. These deliberately slower steps
/// keep physical shots visible for multiple display frames; route length is not
/// compressed into a fixed total. Only admitted cells affect visual duration.
pub const MAGIC_CELL_MS:f64=32.;
pub const PROJECTILE_CELL_MS:f64=40.;
const MAGIC_TAIL_MS:f64=128.;
pub fn arrival_ms(e:&PresentationEvent,index:usize)->f64{
    match e.animation.as_ref().map(|a|a.delivery){
        Some(Delivery::Magic)=>index as f64*MAGIC_CELL_MS,
        Some(Delivery::Projectile)=>index as f64*PROJECTILE_CELL_MS,
        _=>0.,
    }
}
fn travel_end(e:&PresentationEvent)->f64{
    e.animation.as_ref().map_or(140.,|a|if a.delivery==Delivery::Local{140.}else{arrival_ms(e,a.cells.len().max(1))})
}
pub(crate) fn duration(e:&PresentationEvent)->f64 {
    if let Some(d)=crate::divine_animation_shapes::duration(e){return d;}
    e.animation.as_ref().map_or(240.,|a|match a.stage {
        Stage::Travel=>travel_end(e)+if a.delivery==Delivery::Magic{MAGIC_TAIL_MS}else{0.},
        Stage::Impact=>100.,Stage::Special=>125.,Stage::Health=>100.,Stage::Death=>100.})
}
fn proposal_color(key:&str)->Rgb {
    let n=key.get(1..).and_then(|s|s.parse::<usize>().ok()).unwrap_or(0);
    if key.starts_with('P') {return match n {1..=6=>[237,40,72],7..=12=>[235,196,72],13..=18=>[179,112,244],19..=24=>if n==23{[47,206,185]}else{[86,207,117]},25..=30=>[66,216,234],31..=36=>[117,156,242],37..=42=>[242,103,137],43..=48=>[196,198,217],49..=54=>[172,218,68],55..=60=>[255,225,144],61..=66=>[234,152,71],67..=72=>[227,114,218],73=>[146,145,239],_=>[211,168,80]};}
    if key.starts_with('A'){return match n{1=>[193,230,255],2=>[255,169,74],3|5=>[167,219,91],4=>[171,236,243],6|7=>[183,142,232],8=>[233,125,219],9=>[186,170,139],10=>[136,175,96],11=>[222,180,225],_=>[111,204,243]};}
    match n {18=>[230,237,244],58=>[172,218,68],1|2|4|10|22|60|71=>[231,116,123],3|16|37|43=>[248,172,85],7|17|31|33..=35|42|82=>[149,211,105],13|14|45|47..=49=>[154,216,240],38..=40|66|73..=80=>[231,197,118],_=>[199,172,229]}
}
fn projectile_glyph(e: &PresentationEvent) -> String {
    e.glyph.clone().filter(|s|!s.is_empty()).unwrap_or_else(|| "↑".into())
}
/// O(C) projection and O(C) output for C exact receipt cells. No geometry query,
/// interpolation, animation-driven simulation, particles, or per-style strokes.
fn action_tiles(cue: &Cue, age: f64, reduced: bool) -> Vec<Tile> {
    let e = &cue.event;
    let a = e.animation.as_ref().unwrap();
    let duration = duration(e);
    if age < 0. || age >= duration || a.cells.is_empty() { return vec![]; }
    let color = e.color.as_ref().map_or_else(|| {
        if a.delivery==Delivery::Projectile && a.proposal=="shared" {return [195,199,208];}
        if a.proposal.starts_with('E') {
            for (word, color) in [("warm", [237,190,112]), ("green", [149,211,105]),
                ("red", [231,116,123]), ("crimson", [231,116,123]),
                ("ivory", [236,225,187]), ("blue", [154,216,240]),
                ("water", [154,216,240]), ("violet", [199,172,229]), ("ember", [248,172,85])] {
                if a.style.contains(word) { return color; }
            }
        }
        proposal_color(&a.proposal)
    }, |s| crate::color::hex(s));
    if let Some(tiles)=crate::divine_animation_shapes::tiles(e,age,reduced,color){return tiles;}
    let travel = a.stage == corpse_core::events::Stage::Travel;
    let projectile = travel && a.delivery==Delivery::Projectile;
    let magic = travel && a.delivery==Delivery::Magic;
    // O(1) palette tuning per cue: keep the exact receipt, fade and glyph.
    // Lightning gets the brightest cool-white field; other magic retains hue.
    let field_strength = if a.proposal == "A01" { 0.90 }
        else if a.delivery == Delivery::Magic || a.proposal.starts_with('A') { 0.62 }
        else { 0.32 };
    let tile = |p: &corpse_core::events::EventPoint, light, glyph| Tile {
        x: p.x as i32, y: p.y as i32, color, light, field_strength, glyph, mark: None,
    };
    if projectile || magic {
        if reduced {
            return vec![tile(a.cells.last().unwrap(), 0.55, None)];
        }
        if projectile {
            let head=(age/PROJECTILE_CELL_MS) as usize;
            return a.cells.get(head).map(|p|tile(p,0.95,Some(projectile_glyph(e)))).into_iter().collect();
        }
        // A bright advancing tile with a short tapered/retracting tail. The
        // terrain/entity glyph beneath is retained, never replaced by a flame.
        return a.cells.iter().enumerate().filter_map(|(i,p)|{
            let local=age-arrival_ms(e,i);
            (local>=0. && local<MAGIC_TAIL_MS).then(||tile(p,0.95*(1.-local/MAGIC_TAIL_MS),None))
        }).collect();
    }
    a.cells.iter().enumerate().filter_map(|(i, p)| {
        let offset = if travel && !reduced { i as f64 / a.cells.len() as f64 * 70. } else { 0. };
        let local = age-offset;
        if local < 0. { return None; }
        Some(tile(p, if reduced { 0.55 } else { 0.8 * flash(local, duration-offset) }, None))
    }).filter(|t| t.light > 0.).collect()
}

pub fn inventory_marks(cues:&[Cue],now:f64)->Vec<(Option<f64>,crate::action_marks::Mark,Rgb,f64)> {
    cues.iter().filter_map(|c|{let a=c.event.animation.as_ref()?;let age=now-c.started;
        if a.surface.as_deref()!=Some("inventory")||age<0.||age>=duration(&c.event){return None}
        let color=if let Some(n)=a.style.strip_prefix("spark-god-").and_then(|s|s.parse::<usize>().ok()){crate::ability_ui::god_color(n)}else{proposal_color(&a.proposal)};
        Some((a.item_id,crate::action_marks::Mark{style:if a.proposal=="P65"&&a.style=="gear"{"gear-three".into()}else{a.style.clone()},direction:None,progress:if c.static_marks{500}else{(age/duration(&c.event)*1000.).clamp(0.,1000.)as u16}},color,if c.static_marks{0.75}else{0.85*(1.-age/duration(&c.event))}))}).collect()
}

/// Logical anchors shared by painting and retained damage. Item-free resource
/// receipts stay on their real HUD even while the map's Pack overlay is open.
pub fn inventory_hud_anchor(p:&crate::render::Painter,mark:&crate::action_marks::Mark)->(i32,i32,i32,i32) {
    if mark.style=="health-slash" {(p.hud_left()+12,84,24,24)}else if mark.style=="pack-capacity" {
        let (x,w)=crate::render::toolbar_rect_for_width(0,p.logical_width());
        (x+w/2-12,p.toolbar_y()+4,24,28)
    } else if mark.style=="charge-spark"||mark.style=="unlock-diamond"||mark.style=="five-notch" {
        (p.hud_left()+12,crate::ability_ui::top(&p.profile)-26,24,24)
    } else {(p.hud_left()+12,110,24,24)}
}
pub fn paint_inventory_hud(p:&mut crate::render::Painter,view:&crate::render::View,cues:&[Cue],now:f64,pack_open:bool){
    for (id,mark,color,alpha) in inventory_marks(cues,now) {
        if id.is_some() && pack_open || mark.style=="pack-capacity"&&pack_open {continue}
        let anchor=if mark.style.starts_with("reaction-spent"){p.reaction_action_anchor(view,&mark.style)}else if id.is_some(){let (x,w)=crate::render::toolbar_rect_for_width(0,p.logical_width());(x+w/2-12,p.toolbar_y()+4,24,28)}else{inventory_hud_anchor(p,&mark)};
        crate::action_marks::paint(p,&mark,anchor.0,anchor.1,anchor.2,anchor.3,color,alpha);
    }
}
