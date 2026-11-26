//! Deterministic static suitability and bounded roulette placement.
use crate::{Floor, FloorSetupsEntry, rng::GenerationRng};
use std::collections::HashSet;
pub fn setup_contains(s: &FloorSetupsEntry, x: f64, y: f64) -> bool {
    if let Some(fp) = s.footprint.as_ref() {
        let (dx, dy) = (x - s.x + 6.0, y - s.y + 6.0);
        if dx < 0.0 || dx >= 13.0 || dy < 0.0 || dy >= 13.0 {
            return false;
        }
        return fp
            .binary_search_by(|v| v.total_cmp(&(dy * 13.0 + dx)))
            .is_ok();
    }
    if let Some(b) = s.bounds.as_ref() {
        x >= b.left && x <= b.right && y >= b.top && y <= b.bottom
    } else {
        (x - s.x).abs() <= 6.0 && (y - s.y).abs() <= 6.0
    }
}
#[derive(Debug)]
pub struct SpawnHeatmap {
    pub width: usize,
    pub height: usize,
    pub weights: Vec<f64>,
    pub flags: Vec<u16>,
    pub land: Vec<usize>,
    pub water: Vec<usize>,
}
pub fn fixture_cells(f: &Floor) -> Vec<usize> {
    let w = f.width();
    let mut v: Vec<_> = f
        .permanent_fire
        .as_ref()
        .into_iter()
        .flatten()
        .chain(f.grates.as_ref().into_iter().flatten())
        .map(|x| *x as usize)
        .collect();
    if let Some(d) = f.doors.as_ref() {
        v.extend(d.keys().filter_map(|k| k.parse::<usize>().ok()));
    }
    if let Some(vents) = f.vents.as_ref() {
        v.extend(vents.iter().map(|v| v.at as usize));
    }
    if let Some(l) = f.light_sources.as_ref() {
        v.extend(
            l.iter()
                .filter(|s| s.kind.as_ref().is_some_and(|k| k == "brazier"))
                .map(|s| s.y as usize * w + s.x as usize),
        );
    }
    if let Some(e) = f.godbeast_event.as_ref() {
        v.push(e.y as usize * w + e.x as usize);
        if let Some(p) = e.well.as_ref() {
            v.push(p.y as usize * w + p.x as usize)
        }
        if let Some(p) = e.lever.as_ref() {
            v.push(p.y as usize * w + p.x as usize)
        }
        if let Some(p) = e.altar.as_ref() { v.push(p.y as usize * w + p.x as usize); }
        if let Some(p) = e.tap.as_ref() {
            v.push(p.y as usize * w + p.x as usize)
        }
    }
    v
}
fn flag(flags: &mut [u16], at: usize, reason: u16) {
    if let Some(f) = flags.get_mut(at) {
        *f |= reason;
    }
}
fn around(flags: &mut [u16], w: usize, h: usize, at: usize, r: i32, reason: u16) {
    if at >= flags.len() {
        return;
    }
    let (x, y) = ((at % w) as i32, (at / w) as i32);
    for dy in -r..=r {
        for dx in -r..=r {
            let (xx, yy) = (x + dx, y + dy);
            if xx >= 0 && yy >= 0 && (xx as usize) < w && (yy as usize) < h {
                flag(flags, yy as usize * w + xx as usize, reason)
            }
        }
    }
}
/// O(V + metadata footprint + fixture neighborhoods), O(V) scratch.
pub fn build_spawn_heatmap(f: &Floor) -> SpawnHeatmap {
    let (w, h, n) = (f.width(), f.height(), f.tiles.len());
    let mut flags = vec![0; n];
    let mut weights = vec![0.0; n];
    let mut land = Vec::new();
    let mut water = Vec::new();
    let rooms = f
        .room_graph
        .as_ref()
        .map(|g| &g.rooms)
        .or(f.dungeon_rooms.as_ref());
    if let Some(rooms) = rooms {
        for r in rooms {
            for at in &r.cells {
                flag(&mut flags, *at as usize, 2)
            }
        }
    }
    let connectors = f
        .room_graph
        .as_ref()
        .map(|g| &g.connectors)
        .or(f.dungeon_connectors.as_ref());
    if let Some(cs) = connectors {
        for c in cs {
            for at in &c.cells {
                flag(&mut flags, *at as usize, 4)
            }
        }
    }
    if let Some(ss) = f.setups.as_ref() {
        for s in ss {
            let (l, r, t, b) = s
                .bounds
                .as_ref()
                .map(|b| (b.left, b.right, b.top, b.bottom))
                .unwrap_or((s.x - 6.0, s.x + 6.0, s.y - 6.0, s.y + 6.0));
            for y in t.max(0.0) as usize..=b.min(h as f64 - 1.0) as usize {
                for x in l.max(0.0) as usize..=r.min(w as f64 - 1.0) as usize {
                    if setup_contains(s, x as f64, y as f64) {
                        flag(&mut flags, y * w + x, 16)
                    }
                }
            }
        }
    }
    for at in fixture_cells(f) {
        flag(&mut flags, at, 1)
    }
    let mut hazards: Vec<_> = f
        .permanent_fire
        .as_ref()
        .into_iter()
        .flatten()
        .map(|v| *v as usize)
        .collect();
    if let Some(v) = f.vents.as_ref() {
        hazards.extend(v.iter().map(|v| v.at as usize))
    }
    if let Some(l) = f.light_sources.as_ref() {
        hazards.extend(
            l.iter()
                .filter(|s| s.kind.as_ref().is_some_and(|k| k == "brazier"))
                .map(|s| s.y as usize * w + s.x as usize),
        )
    }
    for at in hazards {
        around(&mut flags, w, h, at, 2, 32)
    }
    for at in 0..n {
        if !["floor", "grass", "water", "shallow-water"].contains(&f.tiles[at].as_str()) {
            flag(&mut flags, at, 1)
        }
        if ["chasm", "pit"].contains(&f.tiles[at].as_str()) {
            around(&mut flags, w, h, at, 1, 32)
        }
        let layer = |a: &crate::Field<Vec<f64>>| {
            a.as_ref().and_then(|a| a.get(at)).copied().unwrap_or(0.0) != 0.0
        };
        let hazard = layer(&f.mud)
            || layer(&f.slime)
            || layer(&f.razorleaves)
            || f.environment.as_ref().is_some_and(|e| {
                e.fire[at] != 0.0
                    || e.caustic[at] != 0.0
                    || e.breather[at] != 0.0
                    || e.paralysis[at] != 0.0
                    || layer(&e.confusion) || layer(&e.death_gas)
            });
        if hazard {
            around(&mut flags, w, h, at, 1, 32)
        }
    }
    for at in 0..n {
        let dry = ["floor", "grass"].contains(&f.tiles[at].as_str());
        let wet = ["water", "shallow-water"].contains(&f.tiles[at].as_str());
        if (!dry && !wet) || flags[at] & 1 != 0 {
            flag(&mut flags, at, 1);
            continue;
        }
        let (x, y) = ((at % w) as i32, (at / w) as i32);
        let (mut neighbors, mut cardinal) = (0, 0);
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (xx, yy) = (x + dx, y + dy);
                if (dx != 0 || dy != 0)
                    && xx >= 0
                    && yy >= 0
                    && (xx as usize) < w
                    && (yy as usize) < h
                    && flags[yy as usize * w + xx as usize] & 1 == 0
                {
                    neighbors += 1;
                    if dx == 0 || dy == 0 {
                        cardinal += 1
                    }
                }
            }
        }
        if cardinal <= 2 && neighbors <= 4 {
            flag(&mut flags, at, 8)
        }
        let mut weight = (1.max(neighbors) as f64).powi(2);
        for (bit, mul) in [(2, 1.25), (4, 0.08), (8, 0.3), (16, 0.12), (32, 0.18)] {
            if flags[at] & bit != 0 {
                weight *= mul
            }
        }
        weights[at] = weight;
        if dry { land.push(at) } else { water.push(at) }
    }
    SpawnHeatmap {
        width: w,
        height: h,
        weights,
        flags,
        land,
        water,
    }
}
pub struct SpawnSampler {
    pub map: SpawnHeatmap,
    pub spacing: Vec<f64>,
    pub occupied: HashSet<usize>,
    allowed: Vec<bool>,
    cells: Vec<usize>,
    pub rng: GenerationRng,
}
impl SpawnSampler {
    pub fn new(
        map: SpawnHeatmap,
        candidates: Vec<usize>,
        rng: GenerationRng,
        occupied: HashSet<usize>,
    ) -> Self {
        let mut allowed = vec![false; map.weights.len()];
        let mut cells = Vec::new();
        for at in candidates {
            if at < allowed.len() && map.weights[at] > 0.0 && !allowed[at] {
                allowed[at] = true;
                cells.push(at)
            }
        }
        Self {
            spacing: vec![1.0; allowed.len()],
            map,
            occupied,
            allowed,
            cells,
            rng,
        }
    }
    pub fn weight_at(&self, at: usize) -> f64 {
        if self.allowed.get(at) == Some(&true) && !self.occupied.contains(&at) {
            self.map.weights[at] * self.spacing[at]
        } else {
            0.0
        }
    }
    pub fn pick(&mut self, subset: Option<&[usize]>) -> Option<usize> {
        let cells = subset.unwrap_or(&self.cells);
        let mut total = 0.0;
        let mut last = None;
        for &at in cells {
            let w = self.weight_at(at);
            if w > 0.0 {
                total += w;
                last = Some(at)
            }
        }
        last?;
        let mut threshold = self.rng.random() * total;
        for &at in cells {
            let w = self.weight_at(at);
            if w <= 0.0 {
                continue;
            }
            if threshold < w {
                return Some(at);
            }
            threshold -= w;
        }
        last
    }
    pub fn add_spacing_anchor(&mut self, at: usize) {
        let (x, y) = ((at % self.map.width) as i32, (at / self.map.width) as i32);
        for dy in -8..=8 {
            for dx in -8..=8 {
                let ds = dx * dx + dy * dy;
                let (xx, yy) = (x + dx, y + dy);
                if ds >= 64
                    || xx < 0
                    || yy < 0
                    || xx as usize >= self.map.width
                    || yy as usize >= self.map.height
                {
                    continue;
                }
                let ratio = ds as f64 / 64.0;
                let penalty = 0.015 + (1.0 - 0.015) * ratio * ratio;
                let i = yy as usize * self.map.width + xx as usize;
                self.spacing[i] = self.spacing[i].min(penalty)
            }
        }
    }
    pub fn reserve(&mut self, at: usize) {
        self.add_spacing_anchor(at);
        self.occupied.insert(at);
    }
}
