//! v48 detached visual cue timing. No simulation access or clocks live here.
use crate::render::Rgb;
use corpse_core::events::PresentationEvent;
#[derive(Clone)]
pub struct Cue {
    pub event: PresentationEvent,
    pub started: f64,
    pub held: bool,
}
pub struct Tile {
    pub x: i32,
    pub y: i32,
    pub color: Rgb,
    pub light: f64,
    pub glyph: Option<String>,
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
    let e = &cue.event;
    let held = cue.held && explosion(e);
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
            glyph: Some(glyph),
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
                glyph,
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
                    flash(age - arrival, if step == steps { 86. } else { 56. }),
                    (step == head).then(|| glyph.clone()),
                );
            }
            return out;
        }
    }
    if held {
        add(x, y, 1., Some(glyph));
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            add(x + dx, y + dy, 1., Some("人".into()));
        }
        return out;
    }
    let strength = match e.kind.as_str() {
        "status" => 0.4,
        "impact" => 0.65,
        _ => 1.,
    };
    add(x, y, flash(age, 100.) * strength, Some(glyph));
    if ["burst", "pulse", "heal", "summon"].contains(&e.kind.as_str()) {
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            add(
                x + dx,
                y + dy,
                flash(age - 36., 92.),
                (e.kind == "burst").then(|| "*".into()),
            );
        }
    }
    out
}
#[derive(Default)]
pub struct Effects {
    pub cues: Vec<Cue>,
    key: Option<(u64, u64)>,
}
impl Effects {
    pub fn clear(&mut self) {
        self.cues.clear();
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
        let key = (tick.to_bits(), depth.to_bits());
        if self.key != Some(key) || rewind {
            self.cues.clear();
        }
        self.key = Some(key);
        for event in events {
            let held = explosion(&event);
            if held {
                self.cues.retain(|c| !c.held || c.event.to != event.to);
            }
            self.cues.push(Cue {
                event,
                started: now,
                held,
            });
        }
        if self.cues.len() > 256 {
            self.cues.drain(..self.cues.len() - 256);
        }
    }
    pub fn animate(&self, now: f64) -> bool {
        self.cues.iter().any(|c| !c.held && now - c.started < 240.)
    }
    pub fn prune(&mut self, now: f64) {
        self.cues.retain(|c| c.held || now - c.started < 240.);
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
        }
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
