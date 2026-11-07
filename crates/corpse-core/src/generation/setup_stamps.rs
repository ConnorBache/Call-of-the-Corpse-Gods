//! Authored setup geometry; bounded 13×13 mutations, no random draws.
use super::{GENERATED_VENT_GASES, setup_contains};
use crate::model::*;
struct Stamp<'a> {
    f: &'a mut Floor,
    x: i32,
    y: i32,
    w: i32,
}
impl Stamp<'_> {
    fn at(&self, x: i32, y: i32) -> usize {
        ((self.y + y) * self.w + self.x + x) as usize
    }
    fn tile(&mut self, x: i32, y: i32, t: &str) {
        let i = self.at(x, y);
        self.f.tiles[i] = t.into();
    }
    fn wall(&mut self, x: i32, y: i32) {
        self.tile(x, y, "wall")
    }
    fn rect(&mut self, l: i32, r: i32, t: i32, b: i32, tile: &str) {
        for y in t..=b {
            for x in l..=r {
                self.tile(x, y, tile)
            }
        }
    }
    fn vent(&mut self, x: i32, y: i32, gas: &str) {
        let at = self.at(x, y) as f64;
        self.f.vents.get_or_insert_with(Vec::new).push(GasVent {
            at,
            gas: gas.into(),
            phase: "waiting".into(),
            countdown: 30.,
            amount: Field::Value(1000.),
            ..Default::default()
        });
    }
    fn light(&mut self, x: i32, y: i32, power: f64, kind: &str) {
        self.f
            .light_sources
            .get_or_insert_with(Vec::new)
            .push(FloorLightSourcesEntry {
                x: (self.x + x) as f64,
                y: (self.y + y) as f64,
                power,
                falloff: 10.,
                kind: Field::Value(kind.into()),
            });
    }
    fn flame(&mut self, x: i32, y: i32) {
        let at = self.at(x, y) as f64;
        self.f.permanent_fire.get_or_insert_with(Vec::new).push(at);
        self.light(x, y, 70., "brazier");
    }
    fn door(&mut self, x: i32, y: i32) {
        let key = self.at(x, y).to_string();
        self.f
            .doors
            .get_or_insert_with(Default::default)
            .insert(key, FloorDoorsEntry::default());
    }
    fn field(&mut self, key: &str, l: i32, r: i32, t: i32, b: i32) {
        let n = self.f.tiles.len();
        let w = self.w;
        let sx = self.x;
        let sy = self.y;
        let a = if key == "mud" {
            &mut self.f.mud
        } else {
            &mut self.f.slime
        };
        let a = a.get_or_insert_with(|| vec![0.; n]);
        for y in t..=b {
            for x in l..=r {
                a[((sy + y) * w + sx + x) as usize] = 1.;
            }
        }
    }
    fn spawn(&mut self, kind: &str, x: i32, y: i32) {
        self.f
            .setup_spawns
            .get_or_insert_with(Vec::new)
            .push(FloorSetupSpawnsEntry {
                kind: kind.into(),
                x: (self.x + x) as f64,
                y: (self.y + y) as f64,
                ..Default::default()
            });
    }
    fn record(&mut self, id: u32) {
        self.f
            .setups
            .get_or_insert_with(Vec::new)
            .push(FloorSetupsEntry {
                id: id as f64,
                x: self.x as f64,
                y: self.y as f64,
                ..Default::default()
            });
    }
}
pub fn stamp_setup(f: &mut Floor, id: u32, x: i32, y: i32, kinds: &[String]) -> Result<(), String> {
    let w = f.width() as i32;
    let h = f.height() as i32;
    if x < 6 || y < 6 || x + 6 >= w || y + 6 >= h {
        return Err("Setup footprint outside grid".into());
    }
    if (101..=108).contains(&id) {
        return stamp_water_setup_with_kind(f, id, x, y, if id==108 { kinds.first().map(String::as_str) } else { None });
    }
    let mut s = Stamp { f, x, y, w };
    s.rect(-6, 6, -6, 6, "floor");
    if [1, 2, 4, 12, 19].contains(&id) {
        s.flame(-3, 0);
        s.vent(
            -2,
            0,
            if id == 2 {
                "paralysis"
            } else if id == 12 {
                "flammable"
            } else {
                "caustic"
            },
        );
    }
    if id == 4 {
        let at = s.at(-4, 0) as f64;
        s.f.vents.as_mut().unwrap().last_mut().unwrap().at = at;
        s.vent(4, 0, "caustic");
    }
    if id == 8 {
        s.rect(-2, 2, -2, 1, "wall");
        for (x, y) in [(1, -2), (1, -1), (1, 0), (0, 0), (-1, 0), (-2, 0)] {
            s.tile(x, y, "floor")
        }
        s.wall(1, -2);
        s.vent(
            1,
            -1,
            GENERATED_VENT_GASES[((x + y) as usize) % GENERATED_VENT_GASES.len()],
        );
    }
    if id == 10 {
        for yy in -3..=3 {
            if yy != 0 {
                s.rect(-4, 4, yy, yy, "chasm")
            }
        }
        s.field("mud", -4, 4, 0, 0);
    }
    if id == 12 {
        s.rect(1, 5, -3, 3, "water")
    }
    if [15, 44, 52].contains(&id) {
        for xx in [-2, 2] {
            for yy in [-2, 2] {
                s.wall(xx, yy)
            }
        }
    }
    if id == 18 {
        s.rect(0, 0, -4, 4, "wall");
        s.tile(0, 0, "floor");
        let at = s.at(0, 0) as f64;
        s.f.grates.get_or_insert_with(Vec::new).push(at);
        for yy in [-1, 1] {
            s.rect(-2, 2, yy, yy, "wall")
        }
        s.wall(2, 0);
        s.vent(1, 0, "paralysis");
    }
    if id == 19 {
        let at = s.at(1, 0) as f64;
        s.f.vents.as_mut().unwrap().last_mut().unwrap().at = at;
        let at = s.at(-2, 0) as f64;
        *s.f.permanent_fire.as_mut().unwrap().last_mut().unwrap() = at;
        if let Some(light) =
            s.f.light_sources
                .as_mut()
                .unwrap()
                .iter_mut()
                .find(|l| l.x == (x - 3) as f64 && l.y == y as f64)
        {
            light.x = (x - 2) as f64;
        }
        s.rect(0, 0, -4, 4, "wall");
        s.tile(0, 0, "floor");
        s.door(0, 0);
    }
    if id == 24 {
        s.spawn("fungus", 0, 0)
    }
    if [34, 35, 37, 38, 50].contains(&id) {
        s.field("slime", -4, 4, -1, 1)
    }
    if [37, 40].contains(&id) {
        s.rect(-1, 1, -2, 2, "wall")
    }
    if id == 36 {
        for xx in [-2, 2] {
            s.rect(xx, xx, -4, 2, "wall")
        }
    }
    if [39, 47].contains(&id) {
        s.rect(0, 0, -4, 4, "wall");
        s.tile(0, 0, "floor");
        s.door(0, 0);
    }
    if [41, 43, 59].contains(&id) {
        for xx in [-3, 0, 3] {
            s.wall(xx, -2);
            s.wall(xx, 2);
        }
    }
    if id == 42 {
        s.rect(2, 2, -3, 3, "wall");
        s.tile(2, 0, "floor");
    }
    if id == 45 {
        for yy in [-2, 2] {
            s.rect(-3, 3, yy, yy, "wall")
        }
    }
    if id == 46 {
        s.wall(-2, -1);
        s.wall(-2, 1);
    }
    if [48, 49, 50].contains(&id) {
        s.rect(1, 5, -3, 3, "water")
    }
    if id == 49 {
        s.rect(-5, -3, -2, 2, "chasm")
    }
    if id == 51 {
        s.rect(-5, -1, -3, 3, "water");
        s.rect(1, 5, -3, 3, "water");
    }
    if [53, 56].contains(&id) {
        s.rect(-1, 1, -4, 4, "chasm")
    }
    if [54, 55].contains(&id) {
        for yy in [-2, 2] {
            s.rect(-4, 4, yy, yy, "wall")
        }
        s.tile(0, -2, "floor");
    }
    if id == 57 {
        s.rect(1, 1, -2, 2, "wall")
    }
    if id == 67 {
        s.light(4, 0, 100., "beacon");
        for xx in -4..=1 {
            let at = s.at(xx, 0);
            s.f.lights[at] = (-99f64).max(-70. - 2. * (s.f.depth - 1.));
        }
    }
    for (n, kind) in kinds.iter().enumerate() {
        let mut dx = if n == 0 { 2 } else { -2 };
        let mut dy = if n == 2 { 2 } else { 0 };
        if kind == "river-drake" {
            dx = if id == 51 { -3 } else { 3 };
            dy = 0;
        }
        if kind == "glim" {
            dx = -3;
            dy = 0;
            s.field("slime", dx - 1, dx + 1, dy - 1, dy + 1);
        }
        if id == 54 {
            dx = if n == 0 { -2 } else { 3 };
        }
        if id == 57 {
            dx = -1;
            dy = n as i32 - 1;
        }
        if kind != "river-drake" {
            s.tile(dx, dy, "floor")
        }
        s.spawn(kind, dx, dy);
    }
    s.record(id);
    Ok(())
}
pub fn stamp_water_setup(f: &mut Floor, id: u32, x: i32, y: i32) -> Result<(), String> {
    stamp_water_setup_with_kind(f,id,x,y,None)
}
fn stamp_water_setup_with_kind(f: &mut Floor, id: u32, x: i32, y: i32, watcher:Option<&str>) -> Result<(), String> {
    if !(101..=108).contains(&id) {
        return Err("Unknown water setup".into());
    }
    let w = f.width() as i32;
    if x < 6 || y < 6 || x + 6 >= w || y + 6 >= f.height() as i32 {
        return Err("Setup footprint outside grid".into());
    }
    let mut s = Stamp { f, x, y, w };
    s.rect(-6, 6, -6, 6, "floor");
    s.rect(-5, -3, -1, 1, "water");
    s.rect(3, 5, -1, 1, "water");
    if id == 102 {
        for yy in [-1, 1] {
            s.rect(-3, 3, yy, yy, "water")
        }
    } else {
        s.rect(-3, 3, 0, 0, "water")
    }
    if id == 103 {
        s.vent(-1, -1, "paralysis")
    }
    if id == 104 {
        s.spawn("maulchild", -1, -1)
    }
    if id == 105 {
        s.spawn("pale:Spitter", -1, -2);
        s.rect(2, 2, -2, 2, "wall");
        s.rect(1, 4, 3, 3, "water");
        for (xx, yy) in [(1, 1), (1, 2), (4, 2)] {
            s.tile(xx, yy, "water")
        }
        s.wall(2, 0);
    }
    if id == 106 {
        for yy in [-3, 3] {
            s.rect(-3, 0, yy, yy, "water")
        }
        s.rect(0, 0, -3, 3, "water");
        s.rect(-5, -3, -4, -2, "water");
        s.rect(-5, -3, 2, 4, "water");
    }
    if id == 107 {
        s.rect(-2, 2, 1, 1, "water");
        s.spawn("river-drake", 2, 0);
    }
    if id == 108 {
        let (kind, aquatic) = if let Some(kind)=watcher {
            (kind.to_owned(),crate::population::catalog().aquatic.get(kind)==Some(&true))
        } else { crate::population::low_end_water_setup_kind(
            s.f.depth as u32,
            s.f.enemy_families.as_ref().map(Vec::as_slice),
        )? };
        if aquatic {
            s.tile(-4, 2, "water")
        }
        s.spawn(&kind, -4, if aquatic { 0 } else { -2 });
        s.wall(4, -2);
        s.light(4, -2, 100., "torch");
    }
    s.record(id);
    Ok(())
}
pub fn generate_room_doors(f: &mut Floor, r: &mut dyn FnMut() -> f64) {
    let w = f.width();
    let h = f.height();
    f.doors.get_or_insert_with(Default::default);
    let mut count = 0;
    for y in 7..h.saturating_sub(7) {
        for x in 7..w.saturating_sub(7) {
            if count >= 12 {
                return;
            }
            let i = y * w + x;
            if f.tiles[i] != "floor"
                || f.doors.as_ref().unwrap().contains_key(&i.to_string())
                || f.setups
                    .as_ref()
                    .is_some_and(|a| a.iter().any(|p| setup_contains(p, x as i32, y as i32)))
            {
                continue;
            }
            let solid = |j: usize| f.tiles[j] == "wall";
            if ((solid(i - 1) && solid(i + 1) && !solid(i - w) && !solid(i + w))
                || (solid(i - w) && solid(i + w) && !solid(i - 1) && !solid(i + 1)))
                && r() < 0.15
            {
                f.doors
                    .as_mut()
                    .unwrap()
                    .insert(i.to_string(), FloorDoorsEntry::default());
                count += 1;
            }
        }
    }
}
