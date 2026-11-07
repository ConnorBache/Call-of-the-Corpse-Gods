//! Native frozen-v48 geological planning and materialization.
use super::{Bounds, LayoutConstraints, Point, value_field};
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;
#[derive(Clone, Debug, Deserialize)]
struct Archetype {
    id: String,
    weight: f64,
    kind: String,
    tile: String,
    radius: f64,
    satellites: usize,
    hydrology: String,
    shape: Option<String>,
}
fn archetypes() -> &'static BTreeMap<String, Vec<Archetype>> {
    static C: OnceLock<BTreeMap<String, Vec<Archetype>>> = OnceLock::new();
    C.get_or_init(|| {
        serde_json::from_str(include_str!("../../data/geology-archetypes-v48.json"))
            .expect("frozen archetype data")
    })
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Landmark {
    pub id: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<usize>,
    pub kind: String,
    pub scale: String,
    pub center: Point,
    pub bounds: Bounds,
    pub cells: Vec<usize>,
    pub room_ids: Vec<usize>,
    pub connector_ids: Vec<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeologicalPlan {
    pub width: usize,
    pub height: usize,
    pub domain: String,
    pub archetype: String,
    pub orientation: String,
    pub phase: u32,
    pub spread_across_map: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_floor_cells: Option<usize>,
    pub reserved_cells: IndexSet<usize>,
    pub route_cells: IndexSet<usize>,
    pub tiles: Vec<Option<String>>,
    pub owners: Vec<i16>,
    pub landmarks: Vec<Landmark>,
    pub terrain_spine: Vec<usize>,
    pub field: Vec<f32>,
    pub forest_density: Vec<f32>,
}
impl GeologicalPlan {
    pub fn constraints(&self) -> LayoutConstraints {
        LayoutConstraints {
            reserved_cells: self.reserved_cells.iter().copied().collect(),
            route_cells: self.route_cells.iter().copied().collect(),
            spread_across_map: true,
            maximum_floor_cells: self.maximum_floor_cells,
        }
    }
}
#[derive(Clone, Copy)]
struct FPoint {
    x: f64,
    y: f64,
}
fn round(n: f64) -> i32 {
    let f = n.floor();
    if n - f < 0.5 {
        f as i32
    } else {
        (f + 1.0) as i32
    }
}
fn dry(t: Option<&str>) -> bool {
    matches!(t, Some("floor" | "grass"))
}
fn obstruction(t: Option<&str>) -> bool {
    matches!(t, Some("tree" | "chasm" | "water"))
}
fn ground(domain: &str) -> &str {
    if ["Meadow", "Forest", "Archipelago"].contains(&domain) {
        "grass"
    } else {
        "floor"
    }
}
fn nearby(at: usize, w: usize, h: usize) -> Vec<usize> {
    [
        at.checked_sub(1),
        at.checked_add(1),
        at.checked_sub(w),
        at.checked_add(w),
    ]
    .into_iter()
    .flatten()
    .filter(|&n| n >= w && n < w * (h - 1) && n % w > 0 && n % w < w - 1)
    .collect()
}
struct Builder {
    w: usize,
    h: usize,
    protected: BTreeSet<usize>,
    rough: Vec<f32>,
    tiles: Vec<Option<String>>,
    owners: Vec<i16>,
    landmarks: Vec<Landmark>,
    spine: IndexSet<usize>,
    routes: IndexSet<usize>,
}
impl Builder {
    fn bounded(&self, p: FPoint) -> FPoint {
        let margin = 5.min(self.w.min(self.h) / 4) as f64;
        FPoint {
            x: p.x.clamp(margin, self.w as f64 - margin - 1.0),
            y: p.y.clamp(margin, self.h as f64 - margin - 1.0),
        }
    }
    fn landmark(
        &mut self,
        kind: &str,
        scale: &str,
        p: FPoint,
        parent_id: Option<usize>,
    ) -> Result<usize, String> {
        let id = self.landmarks.len();
        if id >= 24 {
            return Err("Geological landmark budget exceeded.".into());
        }
        self.landmarks.push(Landmark {
            id,
            parent_id,
            kind: kind.into(),
            scale: scale.into(),
            center: Point {
                x: round(p.x),
                y: round(p.y),
            },
            bounds: Bounds {
                x: round(p.x),
                y: round(p.y),
                width: 1,
                height: 1,
            },
            cells: vec![],
            room_ids: vec![],
            connector_ids: vec![],
        });
        Ok(id)
    }
    fn ellipse(&mut self, p: FPoint, rx: f64, ry: f64, tile: &str, id: usize) {
        for y in 2.max((p.y - ry * 1.12).floor() as i32)
            ..=(self.h as i32 - 3).min((p.y + ry * 1.12).ceil() as i32)
        {
            for x in 2.max((p.x - rx * 1.12).floor() as i32)
                ..=(self.w as i32 - 3).min((p.x + rx * 1.12).ceil() as i32)
            {
                let at = y as usize * self.w + x as usize;
                let radial = ((x as f64 - p.x) / rx).powi(2) + ((y as f64 - p.y) / ry).powi(2);
                if radial <= 0.86 + self.rough[at] as f64 * 0.28 && !self.protected.contains(&at) {
                    self.tiles[at] = Some(tile.into());
                    self.owners[at] = id as i16
                }
            }
        }
    }
    fn line(&self, a: FPoint, b: FPoint) -> Vec<usize> {
        let (mut x, mut y) = (round(a.x), round(a.y));
        let (tx, ty) = (round(b.x), round(b.y));
        let mut result = vec![y as usize * self.w + x as usize];
        while x != tx || y != ty {
            let ex = (tx - x).abs();
            let ey = (ty - y).abs();
            if ex != 0
                && (ey == 0
                    || ex as f64 / (tx as f64 - a.x).abs().max(1.0)
                        >= ey as f64 / (ty as f64 - a.y).abs().max(1.0))
            {
                x += (tx - x).signum()
            } else {
                y += (ty - y).signum()
            }
            result.push(y as usize * self.w + x as usize)
        }
        result
    }
    fn stroke(&mut self, points: &[FPoint], radius: f64, tile: &str, id: usize, record: bool) {
        for p in points.windows(2) {
            let a = p[0];
            let b = p[1];
            let steps = ((b.x - a.x).hypot(b.y - a.y) * 2.0).ceil().max(1.0) as usize;
            for s in 0..=steps {
                let t = s as f64 / steps as f64;
                self.ellipse(
                    FPoint {
                        x: a.x + (b.x - a.x) * t,
                        y: a.y + (b.y - a.y) * t,
                    },
                    radius,
                    radius,
                    tile,
                    id,
                )
            }
            if record {
                for at in self.line(a, b) {
                    if !self.protected.contains(&at) {
                        self.spine.insert(at);
                    }
                }
            }
        }
    }
}
/// Bounded L<=24 regions; O(LV) geometry, O(V) component/BFS work and storage.
/// IndexSet preserves JavaScript insertion order for crossing and terrain-core provenance.
pub fn plan_geology(
    w: usize,
    h: usize,
    domain: &str,
    r: &mut impl FnMut() -> f64,
    protection: &BTreeSet<usize>,
) -> Result<GeologicalPlan, String> {
    if w < 12 || h < 12 {
        return Err("Geology requires an interior grid at least 12 by 12.".into());
    }
    let mut protected = protection.clone();
    if domain == "Archipelago" {
        let mut entrance = None;
        'search: for y in 1..=h - 8 {
            for x in 1..=w - 8 {
                if (0..7).all(|dy| (0..7).all(|dx| protection.contains(&((y + dy) * w + x + dx)))) {
                    entrance = Some(Point {
                        x: x as i32 + 3,
                        y: y as i32 + 3,
                    });
                    break 'search;
                }
            }
        }
        if let Some(p) = entrance {
            let direction = if (p.x as f64) < w as f64 / 2.0 { 1 } else { -1 };
            for step in 2..=18 {
                for side in -5..=5 {
                    let x = p.x + direction * step;
                    let y = p.y + side;
                    if x > 0 && x < w as i32 - 1 && y > 0 && y < h as i32 - 1 {
                        protected.insert(y as usize * w + x as usize);
                    }
                }
            }
        }
    }
    let choices = archetypes().get(domain).unwrap_or(&archetypes()["Cave"]);
    let mut draw = r() * choices.iter().map(|a| a.weight).sum::<f64>();
    let mut archetype = choices.last().unwrap();
    for a in choices {
        draw -= a.weight;
        if draw < 0.0 {
            archetype = a;
            break;
        }
    }
    let orientation = if r() < 0.5 { "horizontal" } else { "vertical" };
    let phase = (r() * 65536.0).floor() as u32;
    let phase_angle = phase as f64 / 65536.0 * std::f64::consts::PI * 2.0;
    let field = value_field(w, h, 13, r)?;
    let rough = value_field(w, h, 5, r)?;
    let mut forest_density: Vec<f32> = field
        .iter()
        .enumerate()
        .map(|(i, &n)| (0.16 + 0.70 * n as f64 + 0.12 * rough[i] as f64) as f32)
        .collect();
    let mut b = Builder {
        w,
        h,
        protected,
        rough,
        tiles: vec![None; w * h],
        owners: vec![-1; w * h],
        landmarks: vec![],
        spine: IndexSet::new(),
        routes: IndexSet::new(),
    };
    let hub = b.bounded(FPoint {
        x: w as f64 * (0.28 + r() * 0.44),
        y: h as f64 * (0.31 + r() * 0.38),
    });
    let primary = archetype.tile.as_str();
    let major = b.landmark(&archetype.kind, "major", hub, None)?;
    let base_angle = if orientation == "horizontal" {
        0.0
    } else {
        std::f64::consts::PI / 2.0
    } + phase_angle.sin() * 0.45;
    let branches = archetype.satellites + 1;
    let mut endpoints: Vec<FPoint> = vec![];
    let mut bends: Vec<FPoint> = vec![];
    let mut origins: Vec<FPoint> = vec![];
    let chained = [
        "flooded-cavern",
        "underground-river",
        "meandering-river",
        "braided-straits",
        "island-chain",
        "high-ridge",
    ]
    .contains(&archetype.id.as_str());
    let radius = (archetype.radius
        * if obstruction(Some(primary)) {
            0.53
        } else {
            0.69
        })
    .clamp(1.3, 3.1);
    let span_x = (w as f64 * 0.24).min(19.0);
    let span_y = (h as f64 * 0.29).min(14.0);
    let basin = archetype.shape.as_deref() == Some("basin");
    b.ellipse(
        hub,
        radius * if basin { 2.25 } else { 1.6 },
        radius * if basin { 1.65 } else { 1.4 },
        primary,
        major,
    );
    for n in 0..branches {
        let origin = if n < 2 {
            hub
        } else if chained {
            endpoints[n - 2]
        } else {
            bends[n - 2]
        };
        let angle = base_angle
            + n as f64 * std::f64::consts::PI * 2.0 / branches as f64
            + (r() - 0.5) * 0.5;
        let end = b.bounded(FPoint {
            x: origin.x + angle.cos() * span_x * (0.75 + r() * 0.35),
            y: origin.y + angle.sin() * span_y * (0.78 + r() * 0.25),
        });
        let bend = b.bounded(FPoint {
            x: (origin.x + end.x) / 2.0 - angle.sin() * (1.0 + r() * 2.0),
            y: (origin.y + end.y) / 2.0 + angle.cos() * (0.8 + r() * 1.6),
        });
        let arm_kind = if archetype.kind == "river" {
            "lake"
        } else {
            archetype.kind.as_str()
        };
        let medium = b.landmark(arm_kind, "medium", end, Some(major))?;
        endpoints.push(end);
        bends.push(bend);
        origins.push(origin);
        b.stroke(
            &[origin, bend, end],
            if basin {
                (radius * 0.56).max(1.25)
            } else {
                radius * 0.8
            },
            primary,
            medium,
            true,
        );
        let rx = 3.0 + r() * 2.2;
        let ry = 2.0 + r() * 1.6;
        b.ellipse(end, rx, ry, primary, medium);
        let twig = b.bounded(FPoint {
            x: end.x + (angle + 0.7).cos() * (3.0 + r() * 3.0),
            y: end.y + (angle + 0.7).sin() * (2.0 + r() * 2.0),
        });
        let minor = b.landmark(arm_kind, "minor", twig, Some(medium))?;
        b.stroke(&[end, twig], 1.15 + r() * 0.4, primary, minor, false);
    }
    if archetype.hydrology != "none" {
        let stream = archetype.hydrology == "stream";
        let water = b.landmark(
            if stream { "river" } else { "lake" },
            "medium",
            hub,
            Some(major),
        )?;
        if stream {
            b.stroke(
                &[endpoints[0], bends[0], hub, bends[1], endpoints[1]],
                1.05,
                "water",
                water,
                false,
            )
        } else {
            b.ellipse(hub, 3.5 + r() * 1.5, 2.5 + r(), "water", water)
        }
        for n in 0..2 {
            let p = if stream {
                bends[n]
            } else {
                FPoint {
                    x: hub.x + if n != 0 { 1.0 } else { -1.0 } * 2.5,
                    y: hub.y + if n != 0 { 1.0 } else { -1.0 } * 1.4,
                }
            };
            let pool = b.landmark("lake", "minor", p, Some(water))?;
            b.ellipse(p, 2.1 + r(), 1.6 + r() * 0.7, "water", pool);
        }
    }
    if (domain == "Mountain" || domain == "Archipelago") && dry(Some(primary)) {
        let (edge_tile, edge_kind) = if domain == "Mountain" {
            ("chasm", "fissure")
        } else {
            ("water", "river")
        };
        for n in 0..2 {
            let a = origins[n];
            let e = endpoints[n];
            let angle = (e.y - a.y).atan2(e.x - a.x);
            let sign = if n != 0 { -1.0 } else { 1.0 };
            let offset = radius + 2.5;
            let start = b.bounded(FPoint {
                x: a.x - angle.sin() * offset * sign,
                y: a.y + angle.cos() * offset * sign,
            });
            let finish = b.bounded(FPoint {
                x: e.x - angle.sin() * offset * sign,
                y: e.y + angle.cos() * offset * sign,
            });
            let edge = b.landmark(edge_kind, "medium", start, Some(major))?;
            let end = b.landmark(edge_kind, "minor", finish, Some(edge))?;
            b.stroke(
                &[start, finish],
                if domain == "Mountain" { 1.9 } else { 1.5 },
                edge_tile,
                edge,
                false,
            );
            b.ellipse(finish, 2.8, 2.1, edge_tile, end);
        }
    }
    if domain == "Archipelago" {
        let sea_center = b.bounded(FPoint {
            x: w as f64 - hub.x,
            y: h as f64 - hub.y,
        });
        let sea = b.landmark("lake", "major", sea_center, None)?;
        let rx = (w as f64 * 0.15).min(11.5);
        let ry = (h as f64 * 0.22).min(6.4);
        b.ellipse(sea_center, rx, ry, "water", sea);
        for y in round(sea_center.y) - 1..=round(sea_center.y) + 1 {
            for x in round(sea_center.x - rx * 0.62)..=round(sea_center.x + rx * 0.62) {
                let at = y as usize * w + x as usize;
                if b.owners[at] == sea as i16 && !b.protected.contains(&at) {
                    b.spine.insert(at);
                }
            }
        }
        let bay_center = b.bounded(FPoint {
            x: sea_center.x
                + if sea_center.x < w as f64 / 2.0 {
                    1.0
                } else {
                    -1.0
                } * rx
                    * 0.75,
            y: sea_center.y
                + if sea_center.y < h as f64 / 2.0 {
                    1.0
                } else {
                    -1.0
                } * ry
                    * 0.45,
        });
        let bay = b.landmark("lake", "medium", bay_center, Some(sea))?;
        let inlet_center = b.bounded(FPoint {
            x: bay_center.x + (r() - 0.5) * 4.0,
            y: bay_center.y + if r() < 0.5 { -1.0 } else { 1.0 } * 3.0,
        });
        let inlet = b.landmark("river", "minor", inlet_center, Some(bay))?;
        b.ellipse(bay_center, rx * 0.64, ry * 0.72, "water", bay);
        b.stroke(&[bay_center, inlet_center], 1.6, "water", inlet, false);
    }
    if domain == "Forest" {
        for n in 0..2 {
            let p = b.bounded(FPoint {
                x: w as f64 * if n != 0 { 0.73 } else { 0.25 } + (r() - 0.5) * 5.0,
                y: h as f64 * if n != 0 { 0.32 } else { 0.67 } + (r() - 0.5) * 3.0,
            });
            let grove = b.landmark("grove", "major", p, None)?;
            let stand = b.landmark("grove", "medium", p, Some(grove))?;
            let rx = 6.0 + r() * 2.5;
            let ry = 3.4 + r() * 1.5;
            for y in 1..h - 1 {
                for x in 1..w - 1 {
                    let at = y * w + x;
                    let radial = ((x as f64 - p.x) / (rx * 1.7)).powi(2)
                        + ((y as f64 - p.y) / (ry * 1.8)).powi(2);
                    if radial <= 1.6 {
                        forest_density[at] = (forest_density[at] as f64)
                            .max(0.34 + 0.57 * (1.0 - radial / 1.6).max(0.0))
                            as f32
                    }
                }
            }
            b.ellipse(p, rx, ry, "tree", grove);
            b.ellipse(
                FPoint {
                    x: p.x - 1.0,
                    y: p.y + 0.5,
                },
                rx * 0.62,
                ry * 0.72,
                "tree",
                stand,
            );
            let glade_center = b.bounded(FPoint {
                x: p.x + rx * 0.35,
                y: p.y - ry * 0.15,
            });
            let glade = b.landmark("clearing", "minor", glade_center, Some(stand))?;
            b.ellipse(glade_center, 2.5 + r(), 1.8 + r() * 0.5, "grass", glade);
            let mouth = b.bounded(FPoint {
                x: p.x + rx + 2.0,
                y: glade_center.y,
            });
            b.stroke(&[glade_center, mouth], 1.25, "grass", glade, false);
            for at in b.line(glade_center, mouth) {
                b.routes.insert(at);
            }
        }
    }
    if obstruction(Some(primary)) {
        let p = bends[branches - 1];
        let e = endpoints[branches - 1];
        let origin = origins[branches - 1];
        let angle = (e.y - origin.y).atan2(e.x - origin.x) + std::f64::consts::PI / 2.0;
        let reach = radius + 3.0;
        let a = b.bounded(FPoint {
            x: p.x + angle.cos() * reach,
            y: p.y + angle.sin() * reach,
        });
        let z = b.bounded(FPoint {
            x: p.x - angle.cos() * reach,
            y: p.y - angle.sin() * reach,
        });
        b.stroke(&[a, z], 1.25, ground(domain), major, false);
        for at in b.line(a, z) {
            b.routes.insert(at);
        }
    }
    for at in b.routes.clone() {
        for n in [at, at - 1, at + 1, at - w, at + w] {
            if n >= w && n < w * (h - 1) && n % w > 0 && n % w < w - 1 && !b.protected.contains(&n)
            {
                b.tiles[n] = Some(ground(domain).into());
                b.routes.insert(n);
            }
        }
    }
    let mut reserved = IndexSet::new();
    for at in 0..b.tiles.len() {
        if b.protected.contains(&at) {
            b.tiles[at] = None;
            b.owners[at] = -1;
            continue;
        }
        if obstruction(b.tiles[at].as_deref()) {
            reserved.insert(at);
        }
    }
    let interior = |at: usize| at >= w && at < w * (h - 1) && at % w > 0 && at % w < w - 1;
    let mut component = vec![-1i16; w * h];
    let mut groups: Vec<Vec<usize>> = vec![];
    for at in 0..b.tiles.len() {
        if !interior(at) || reserved.contains(&at) || component[at] >= 0 {
            continue;
        }
        let id = groups.len();
        let mut cells = vec![at];
        component[at] = id as i16;
        let mut head = 0;
        while head < cells.len() {
            let cur = cells[head];
            head += 1;
            for n in nearby(cur, w, h) {
                if !reserved.contains(&n) && component[n] < 0 {
                    component[n] = id as i16;
                    cells.push(n)
                }
            }
        }
        groups.push(cells);
    }
    if groups.len() > 1 {
        let mut main = 0;
        for id in 0..groups.len() {
            if groups[id].len() > groups[main].len() {
                main = id
            }
        }
        let mut prev = vec![usize::MAX; w * h];
        let mut distance = vec![-1i16; w * h];
        let mut search = groups[main].clone();
        let mut joined = vec![false; w * h];
        for &at in &search {
            prev[at] = at;
            distance[at] = 0;
            joined[at] = true
        }
        let mut head = 0;
        while head < search.len() {
            let cur = search[head];
            head += 1;
            for n in nearby(cur, w, h) {
                if prev[n] == usize::MAX {
                    prev[n] = cur;
                    distance[n] = distance[cur] + 1;
                    search.push(n)
                }
            }
        }
        for id in 0..groups.len() {
            if id == main {
                continue;
            }
            let mut at = groups[id][0];
            for &n in &groups[id] {
                if distance[n] < distance[at] {
                    at = n
                }
            }
            let mut connected_now = vec![];
            while !joined[at] && distance[at] > 0 {
                for n in std::iter::once(at).chain(nearby(at, w, h)) {
                    if !b.protected.contains(&n) {
                        b.tiles[n] = Some(ground(domain).into())
                    }
                    reserved.shift_remove(&n);
                    b.routes.insert(n);
                    connected_now.push(n)
                }
                at = prev[at]
            }
            for n in connected_now {
                joined[n] = true
            }
        }
    }
    let major_of: Vec<usize> = b
        .landmarks
        .iter()
        .map(|entry| {
            let mut id = entry.id;
            while let Some(parent) = b.landmarks[id].parent_id {
                id = parent
            }
            id
        })
        .collect();
    for y in 2..h - 2 {
        for x in 2..w - 2 {
            let at = y * w + x;
            let owner = b.owners[at];
            if owner < 0 || b.protected.contains(&at) {
                continue;
            }
            let family = major_of[owner as usize];
            if [
                -(w as i64) - 1,
                -(w as i64),
                -(w as i64) + 1,
                -1,
                1,
                w as i64 - 1,
                w as i64,
                w as i64 + 1,
            ]
            .iter()
            .all(|d| {
                let owner = b.owners[(at as i64 + d) as usize];
                owner >= 0 && major_of[owner as usize] == family
            }) {
                b.spine.insert(at);
            }
        }
    }
    let mut core_seen = vec![false; w * h];
    let mut core_parents = vec![usize::MAX; w * h];
    let mut core_joined = vec![false; w * h];
    let material = |at: usize| {
        if b.tiles[at].as_deref() == Some("grass") {
            Some("floor")
        } else {
            b.tiles[at].as_deref()
        }
    };
    for start in 0..b.tiles.len() {
        if b.owners[start] < 0 || core_seen[start] {
            continue;
        }
        let family = major_of[b.owners[start] as usize];
        let tile = material(start);
        let compatible = |at: usize| {
            b.owners[at] >= 0 && major_of[b.owners[at] as usize] == family && material(at) == tile
        };
        let mut cells = vec![start];
        core_seen[start] = true;
        let mut head = 0;
        while head < cells.len() {
            let cur = cells[head];
            head += 1;
            for n in nearby(cur, w, h) {
                if !core_seen[n] && compatible(n) {
                    core_seen[n] = true;
                    cells.push(n)
                }
            }
        }
        let seeds: Vec<usize> = cells
            .iter()
            .copied()
            .filter(|at| b.spine.contains(at))
            .collect();
        let root = seeds.first().copied().unwrap_or(cells[cells.len() / 2]);
        let mut walk = vec![root];
        core_parents[root] = root;
        let mut head = 0;
        while head < walk.len() {
            let cur = walk[head];
            head += 1;
            for n in nearby(cur, w, h) {
                if core_parents[n] == usize::MAX && compatible(n) {
                    core_parents[n] = cur;
                    walk.push(n)
                }
            }
        }
        b.spine.insert(root);
        core_joined[root] = true;
        for seed in seeds {
            let mut at = seed;
            while !core_joined[at] {
                b.spine.insert(at);
                core_joined[at] = true;
                at = core_parents[at]
            }
        }
        let mut frontier: Vec<usize> = cells
            .iter()
            .copied()
            .filter(|at| b.spine.contains(at))
            .collect();
        let fraction = if domain == "Archipelago" && tile == Some("water") {
            0.68
        } else {
            0.60
        };
        let target = (cells.len() as f64 * fraction).ceil() as usize;
        let mut head = 0;
        while head < frontier.len() && frontier.len() < target {
            let cur = frontier[head];
            head += 1;
            for n in nearby(cur, w, h) {
                if frontier.len() >= target {
                    break;
                }
                if !core_joined[n] && compatible(n) {
                    b.spine.insert(n);
                    core_joined[n] = true;
                    frontier.push(n)
                }
            }
        }
    }
    for at in b.routes.clone() {
        for next in [at - 1, at + 1, at - w, at + w] {
            if dry(b.tiles.get(next).and_then(|t| t.as_deref())) {
                b.routes.insert(next);
            }
        }
    }
    let terrain_spine = b
        .spine
        .iter()
        .copied()
        .filter(|&at| b.owners[at] >= 0)
        .collect();
    Ok(GeologicalPlan {
        width: w,
        height: h,
        domain: domain.into(),
        archetype: archetype.id.clone(),
        orientation: orientation.into(),
        phase,
        spread_across_map: true,
        maximum_floor_cells: if domain == "Archipelago" {
            Some(round(((w - 2) * (h - 2)) as f64 * 0.345) as usize)
        } else {
            None
        },
        reserved_cells: reserved,
        route_cells: b.routes,
        tiles: b.tiles,
        owners: b.owners,
        landmarks: b.landmarks,
        terrain_spine,
        field,
        forest_density,
    })
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeologicalProvenance {
    pub version: u32,
    pub orientation: String,
    pub phase: u32,
    pub changed_cells: usize,
    pub archetype: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeologyResult {
    pub landmarks: Vec<Landmark>,
    pub terrain: GeologicalProvenance,
    pub natural_walls: Vec<usize>,
    pub terrain_spine: Vec<usize>,
    pub details: indexmap::IndexMap<String, usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeologyTransition {
    pub domain: String,
    pub top: bool,
}
/// O(LV + V log V) time, O(V+L) working storage. Stable scoring, cardinal flood order,
/// route margins and island bank tie order are semantic. Does not run final topology cleanup.
pub fn generate_geology(
    tiles: &mut [String],
    domain: &str,
    r: &mut impl FnMut() -> f64,
    graph: &super::DungeonLayout,
    protected: &BTreeSet<usize>,
    transitions: &[GeologyTransition],
    supplied: Option<&GeologicalPlan>,
) -> Result<GeologyResult, String> {
    let n = tiles.len();
    let w = if n == 2291 {
        79
    } else {
        let w = (n as f64).sqrt() as usize;
        if w * w != n {
            return Err("Invalid floor grid geometry.".into());
        }
        w
    };
    let h = n / w;
    let before = tiles.to_vec();
    let owned_plan;
    let plan = if let Some(plan) = supplied {
        plan
    } else {
        owned_plan = plan_geology(w, h, domain, r, protected)?;
        &owned_plan
    };
    if plan.width != w || plan.height != h || plan.domain != domain {
        return Err("Geological plan does not match its floor.".into());
    }
    let mut owners = plan.owners.clone();
    let mut landmarks = plan.landmarks.clone();
    for entry in &mut landmarks {
        entry.cells.clear();
        entry.room_ids.clear();
        entry.connector_ids.clear()
    }
    let ground = ground(domain);
    let natural = domain != "Dungeon";
    let mut network = vec![false; n];
    let mut distance = vec![-1i16; n];
    let mut queue = vec![];
    let inside = |at: usize| at >= w && at < n - w && at % w > 0 && at % w < w - 1;
    let target = match domain {
        "Dungeon" => 0.36,
        "Cave" => 0.46,
        "Meadow" => 0.58,
        "Forest" => 0.56,
        "Archipelago" => 0.36,
        "Mountain" => 0.47,
        _ => 0.46,
    };
    let max_distance = match domain {
        "Dungeon" => 1,
        "Cave" => 2,
        "Meadow" => 6,
        "Forest" => 7,
        "Archipelago" => 4,
        "Mountain" => 3,
        _ => 2,
    };
    let mut route_margins = BTreeSet::new();
    for connector in &graph.connectors {
        if connector.purpose != "loop" && connector.style != Some(super::PassageStyle::Constructed)
        {
            continue;
        }
        for &at in &connector.cells {
            for near in nearby(at, w, h) {
                if before[near] == "wall" && plan.owners[near] < 0 {
                    route_margins.insert(near);
                }
            }
        }
    }
    let mut island_banks = BTreeSet::new();
    if domain == "Archipelago" {
        for connector in &graph.connectors {
            let path = connector.path.as_ref().unwrap_or(&connector.cells);
            if connector.purpose == "corner" || path.len() < 4 {
                continue;
            }
            for &at in path {
                let mut candidates: Vec<Vec<usize>> = vec![];
                for a in [at, at - 1, at - w, at - w - 1] {
                    let quadrant = [a, a + 1, a + w, a + w + 1];
                    let available = |n: usize| {
                        inside(n)
                            && (dry(Some(&before[n]))
                                || (!protected.contains(&n)
                                    && !plan.reserved_cells.contains(&n)
                                    && !obstruction(plan.tiles[n].as_deref())))
                    };
                    if quadrant.into_iter().all(available) {
                        let missing = quadrant
                            .into_iter()
                            .filter(|n| {
                                !dry(Some(&before[*n]))
                                    && !dry(plan.tiles[*n].as_deref())
                                    && !island_banks.contains(n)
                            })
                            .collect();
                        candidates.push(missing);
                    }
                }
                candidates.sort_by(|a, b| {
                    a.len().cmp(&b.len()).then(
                        a.first()
                            .copied()
                            .unwrap_or(0)
                            .cmp(&b.first().copied().unwrap_or(0)),
                    )
                });
                if let Some(c) = candidates.first() {
                    island_banks.extend(c.iter().copied());
                }
            }
        }
    }
    let (mut preserved, mut obstacles, mut conflicts, mut expanded) = (0, 0, 0, 0);
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let at = y * w + x;
            let feature = plan.tiles[at].as_deref();
            if dry(Some(&before[at])) {
                network[at] = true;
                preserved += 1
            }
            if protected.contains(&at) {
                owners[at] = -1;
                continue;
            }
            if network[at] {
                if obstruction(feature) {
                    conflicts += 1;
                    owners[at] = -1
                }
                tiles[at] = if natural {
                    ground.into()
                } else {
                    before[at].clone()
                };
            } else if island_banks.contains(&at) {
                tiles[at] = ground.into()
            } else if let Some(feature) = feature {
                tiles[at] = feature.into();
                if obstruction(Some(feature)) {
                    obstacles += 1
                }
            } else if natural {
                tiles[at] = if domain == "Forest" || domain == "Meadow" {
                    "tree"
                } else if domain == "Archipelago" {
                    "water"
                } else {
                    "wall"
                }
                .into()
            }
        }
    }
    for at in 0..n {
        if inside(at) && dry(Some(&tiles[at])) {
            distance[at] = 0;
            queue.push(at)
        }
    }
    let initial_dry = queue.len();
    let mut head = 0;
    while head < queue.len() {
        let at = queue[head];
        head += 1;
        if distance[at] >= max_distance {
            continue;
        }
        for near in nearby(at, w, h) {
            if distance[near] >= 0
                || protected.contains(&near)
                || plan.reserved_cells.contains(&near)
                || route_margins.contains(&near)
            {
                continue;
            }
            distance[near] = distance[at] + 1;
            queue.push(near)
        }
    }
    let mut candidates: Vec<usize> = queue.into_iter().filter(|&at| distance[at] > 0).collect();
    let budget = (round(((w - 2) * (h - 2)) as f64 * target) as usize).saturating_sub(initial_dry);
    let score = |at: usize| {
        let density = if domain == "Forest" {
            plan.forest_density[at] as f64
        } else {
            1.0 - plan.field[at] as f64
        };
        distance[at] as f64 * if domain == "Forest" { 0.50 } else { 0.67 }
            + density * if domain == "Forest" { 2.1 } else { 0.65 }
    };
    candidates.sort_by(|&a, &b| score(a).total_cmp(&score(b)).then(a.cmp(&b)));
    for _ in 0..max_distance {
        if expanded >= budget {
            break;
        }
        for &at in &candidates {
            if expanded >= budget {
                break;
            }
            if dry(Some(&tiles[at])) || !nearby(at, w, h).iter().any(|&n| dry(Some(&tiles[n]))) {
                continue;
            }
            if domain == "Forest" && distance[at] > 1 && plan.forest_density[at] as f64 > 0.73 {
                continue;
            }
            tiles[at] = ground.into();
            expanded += 1
        }
    }
    let mut sparse_trees = 0;
    if domain == "Forest" {
        let stand_draw = |at: usize| {
            let gx = (at % w) / 3;
            let gy = (at / w) / 2;
            let mut hash = (gx as u32 + 137 * gy as u32 + plan.phase).wrapping_mul(0x45d9f3b);
            hash = (hash ^ (hash >> 16)).wrapping_mul(0x45d9f3b);
            (hash ^ (hash >> 16)) as f64 / 4294967296.0
        };
        for &at in &candidates {
            if !dry(Some(&tiles[at]))
                || network[at]
                || owners[at] >= 0
                || plan.route_cells.contains(&at)
            {
                continue;
            }
            let density = plan.forest_density[at] as f64;
            if density < 0.25 || density > 0.76 || stand_draw(at) >= 0.08 + density * 0.25 {
                continue;
            }
            if nearby(at, w, h).iter().all(|&n| dry(Some(&tiles[n]))) {
                tiles[at] = "tree".into();
                sparse_trees += 1
            }
        }
    }
    for transition in transitions {
        let from = if transition.top { 1 } else { h - 5 };
        let to = if transition.top { 5 } else { h - 1 };
        for y in from..to {
            for x in 1..w - 1 {
                let at = y * w + x;
                if protected.contains(&at) || owners[at] >= 0 {
                    continue;
                }
                if dry(Some(&tiles[at])) {
                    tiles[at] = self::ground(&transition.domain).into()
                } else if tiles[at] == "wall"
                    && ["Meadow", "Forest"].contains(&transition.domain.as_str())
                {
                    tiles[at] = "tree".into()
                } else if tiles[at] == "tree"
                    && ["Cave", "Dungeon", "Mountain"].contains(&transition.domain.as_str())
                {
                    tiles[at] = "wall".into()
                }
            }
        }
    }
    let mut room_at = vec![-1i32; n];
    let mut connector_at = vec![-1i32; n];
    for room in &graph.rooms {
        for &at in &room.cells {
            room_at[at] = room.id as i32
        }
    }
    for connector in &graph.connectors {
        for &at in &connector.cells {
            connector_at[at] = connector.id as i32
        }
    }
    for at in 0..n {
        if owners[at] >= 0 {
            landmarks[owners[at] as usize].cells.push(at)
        }
    }
    for index in 0..landmarks.len() {
        let mut family = BTreeSet::from([landmarks[index].id]);
        for child in &landmarks {
            if child.parent_id.is_some_and(|id| family.contains(&id)) {
                family.insert(child.id);
            }
        }
        let (mut left, mut right, mut top, mut bottom) = (w - 1, 0, h - 1, 0);
        let mut room_ids = BTreeSet::new();
        let mut connector_ids = BTreeSet::new();
        for child in &landmarks {
            if !family.contains(&child.id) {
                continue;
            }
            for &at in &child.cells {
                left = left.min(at % w);
                right = right.max(at % w);
                top = top.min(at / w);
                bottom = bottom.max(at / w);
                for near in std::iter::once(at).chain(nearby(at, w, h)) {
                    if room_at[near] >= 0 {
                        room_ids.insert(room_at[near] as usize);
                    }
                    if connector_at[near] >= 0 {
                        connector_ids.insert(connector_at[near] as usize);
                    }
                }
            }
        }
        if right >= left {
            landmarks[index].bounds = Bounds {
                x: left as i32,
                y: top as i32,
                width: right - left + 1,
                height: bottom - top + 1,
            }
        }
        landmarks[index].room_ids = room_ids.into_iter().collect();
        landmarks[index].connector_ids = connector_ids.into_iter().collect();
    }
    let (mut changed, mut breached, mut dry_cells) = (0, 0, 0);
    let mut natural_walls = vec![];
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let at = y * w + x;
            if tiles[at] != before[at] {
                changed += 1
            }
            if before[at] == "wall" && dry(Some(&tiles[at])) {
                breached += 1
            }
            if dry(Some(&tiles[at])) {
                dry_cells += 1
            }
            if tiles[at] == "wall" && (natural || nearby(at, w, h).iter().any(|&n| owners[n] >= 0))
            {
                natural_walls.push(at)
            }
        }
    }
    let details = [
        ("passes", 0),
        ("changedCells", changed),
        ("landmarks", landmarks.len()),
        ("breachedWalls", breached),
        (
            "majorRooms",
            landmarks.first().map_or(0, |e| e.room_ids.len()),
        ),
        (
            "majorConnectors",
            landmarks.first().map_or(0, |e| e.connector_ids.len()),
        ),
        ("plannedObstacles", obstacles),
        ("obstacleConflicts", conflicts),
        ("preservedFloorCells", preserved),
        ("expandedCells", expanded),
        ("sparseTrees", sparse_trees),
        ("dryCells", dry_cells),
        ("routeMargins", route_margins.len()),
        ("islandBankCells", island_banks.len()),
        ("plannedBeforeAccretion", usize::from(supplied.is_some())),
    ]
    .into_iter()
    .map(|(k, v)| (k.into(), v))
    .collect();
    Ok(GeologyResult {
        landmarks,
        terrain: GeologicalProvenance {
            version: 1,
            orientation: plan.orientation.clone(),
            phase: plan.phase,
            changed_cells: changed,
            archetype: plan.archetype.clone(),
        },
        natural_walls,
        terrain_spine: plan
            .terrain_spine
            .iter()
            .copied()
            .filter(|&at| owners[at] >= 0)
            .collect(),
        details,
    })
}
pub(crate) fn known_geology_archetype(domain: &str, id: &str) -> bool {
    archetypes()
        .get(domain)
        .is_some_and(|entries| entries.iter().any(|a| a.id == id))
}
