//! Frozen-v48 room accretion. No post-hoc replacement geometry.
use super::{
    PassageStyle, Point, RoomKind, RoomShape, SizeClass, make_room_passage, make_room_shape,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    pub x: i32,
    pub y: i32,
    pub width: usize,
    pub height: usize,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutRoom {
    pub id: usize,
    pub kind: RoomKind,
    pub cells: Vec<usize>,
    pub size_class: SizeClass,
    pub center: Point,
    pub bounds: Bounds,
    pub parent_room_id: Option<usize>,
    pub connector_id: Option<usize>,
    pub doors: Vec<usize>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutConnector {
    pub id: usize,
    pub kind: String,
    pub cells: Vec<usize>,
    pub room_ids: Vec<usize>,
    pub doors: Vec<usize>,
    pub parent_room_id: usize,
    pub purpose: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<PassageStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Vec<usize>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_connector_id: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_before: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_after: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_endpoints: Option<[usize; 2]>,
}
impl LayoutConnector {
    pub fn new(
        id: usize,
        cells: Vec<usize>,
        room_ids: Vec<usize>,
        parent_room_id: usize,
        purpose: &str,
    ) -> Self {
        Self {
            id,
            kind: "hallway".into(),
            cells,
            room_ids,
            doors: vec![],
            parent_room_id,
            purpose: purpose.into(),
            style: None,
            path: None,
            parent_connector_id: None,
            route_before: None,
            route_after: None,
            route_endpoints: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayoutDoor {
    pub open: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DungeonLayout {
    pub tiles: Vec<String>,
    pub doors: BTreeMap<usize, LayoutDoor>,
    pub rooms: Vec<LayoutRoom>,
    pub connectors: Vec<LayoutConnector>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutConstraints {
    #[serde(default)]
    pub reserved_cells: BTreeSet<usize>,
    #[serde(default)]
    pub route_cells: BTreeSet<usize>,
    #[serde(default)]
    pub spread_across_map: bool,
    pub maximum_floor_cells: Option<usize>,
}
impl LayoutConstraints {
    pub fn room_allowed(&self, at: usize) -> bool {
        !self.reserved_cells.contains(&at)
    }
    pub fn route_allowed(&self, at: usize) -> bool {
        self.room_allowed(at) || self.route_cells.contains(&at)
    }
}
pub const DIRECTIONS: [Point; 4] = [
    Point { x: 1, y: 0 },
    Point { x: 0, y: 1 },
    Point { x: -1, y: 0 },
    Point { x: 0, y: -1 },
];
fn integer(r: &mut impl FnMut() -> f64, lo: i32, hi: i32) -> i32 {
    lo + (r() * (hi - lo + 1) as f64).floor() as i32
}
fn inside(w: usize, h: usize, x: i32, y: i32) -> bool {
    x > 0 && y > 0 && x < w as i32 - 1 && y < h as i32 - 1
}
fn adjacent(w: usize, h: usize, at: usize) -> Vec<usize> {
    let x = (at % w) as i32;
    let y = (at / w) as i32;
    DIRECTIONS
        .into_iter()
        .filter(|d| inside(w, h, x + d.x, y + d.y))
        .map(|d| (at as i32 + d.x + d.y * w as i32) as usize)
        .collect()
}
fn surrounding(w: usize, h: usize, at: usize) -> Vec<usize> {
    let mut v = vec![];
    let x = (at % w) as i32;
    let y = (at / w) as i32;
    for dy in -1..=1 {
        for dx in -1..=1 {
            if (dx != 0 || dy != 0) && inside(w, h, x + dx, y + dy) {
                v.push((at as i32 + dx + dy * w as i32) as usize)
            }
        }
    }
    v
}
fn add_room(
    l: &mut DungeonLayout,
    kind: RoomKind,
    bounds: Bounds,
    cells: Vec<usize>,
    parent_room_id: Option<usize>,
    connector_id: Option<usize>,
    size_class: SizeClass,
) -> usize {
    let id = l.rooms.len();
    for &at in &cells {
        l.tiles[at] = "floor".into()
    }
    l.rooms.push(LayoutRoom {
        id,
        kind,
        cells,
        size_class,
        center: Point {
            x: bounds.x + (bounds.width / 2) as i32,
            y: bounds.y + (bounds.height / 2) as i32,
        },
        bounds,
        parent_room_id,
        connector_id,
        doors: vec![],
    });
    id
}
fn add_door(
    l: &mut DungeonLayout,
    w: usize,
    h: usize,
    at: usize,
    room: usize,
    connector: usize,
    entrance: usize,
    exit: usize,
) {
    if l.rooms[room].kind != RoomKind::Rectangle || at == entrance || at == exit {
        return;
    }
    let adj: Vec<usize> = adjacent(w, h, at)
        .into_iter()
        .filter(|&n| l.tiles[n] != "wall")
        .collect();
    if adj.len() != 2 || ![2, 2 * w].contains(&adj[0].abs_diff(adj[1])) {
        return;
    }
    l.doors.insert(at, LayoutDoor { open: false });
    l.rooms[room].doors.push(at);
    l.connectors[connector].doors.push(at);
}
/// O(V² log V) worst case with sector targeting and bounded 180 attempts/slot;
/// cached local neighborhoods and graph data use O(V) memory. Callback sees reference stage boundaries.
pub fn generate_dungeon_layout(
    size: usize,
    entrance: Point,
    exit: Point,
    r: &mut impl FnMut() -> f64,
    constraints: &LayoutConstraints,
    mut stage: Option<&mut dyn FnMut(&str, &DungeonLayout)>,
) -> Result<DungeonLayout, String> {
    let rows = if size == 79 { 29 } else { size };
    if size < 12 || rows < 12 {
        return Err("Dungeon layout requires an integer width of at least 12.".into());
    }
    if !inside(size, rows, entrance.x, entrance.y) || !inside(size, rows, exit.x, exit.y) {
        return Err("Dungeon stairs must be inside the uncarved boundary.".into());
    }
    let mut l = DungeonLayout {
        tiles: vec!["wall".into(); size * rows],
        doors: BTreeMap::new(),
        rooms: vec![],
        connectors: vec![],
    };
    let entrance_at = entrance.y as usize * size + entrance.x as usize;
    let exit_at = exit.y as usize * size + exit.x as usize;
    let max = constraints.maximum_floor_cells.unwrap_or(l.tiles.len());
    if max < 49 || max > l.tiles.len() {
        return Err("Planned floor budget must fit the entrance room and grid.".into());
    }
    let budget = if constraints.maximum_floor_cells.is_none() {
        max
    } else {
        49.max(max.saturating_sub((size + rows).div_ceil(2)))
    };
    let mut carved = 0usize;
    let neighbors: Vec<Vec<usize>> = (0..l.tiles.len())
        .map(|at| adjacent(size, rows, at))
        .collect();
    let surrounds: Vec<Vec<usize>> = (0..l.tiles.len())
        .map(|at| surrounding(size, rows, at))
        .collect();
    let mut first_x = (entrance.x - 3).clamp(1, size as i32 - 8);
    let mut first_y = (entrance.y - 3).clamp(1, rows as i32 - 8);
    let first_fits = |x: i32, y: i32| {
        (0..7).all(|dy| {
            (0..7).all(|dx| constraints.room_allowed((y + dy) as usize * size + (x + dx) as usize))
        })
    };
    if !first_fits(first_x, first_y) {
        let mut c = vec![];
        for y in 1.max(entrance.y - 6)..=(rows as i32 - 8).min(entrance.y) {
            for x in 1.max(entrance.x - 6)..=(size as i32 - 8).min(entrance.x) {
                if first_fits(x, y) {
                    c.push(((x - first_x).abs() + (y - first_y).abs(), y, x))
                }
            }
        }
        c.sort();
        let Some(&(_, y, x)) = c.first() else {
            return Err("Planned terrain leaves no entrance room.".into());
        };
        first_x = x;
        first_y = y;
    }
    add_room(
        &mut l,
        RoomKind::Rectangle,
        Bounds {
            x: first_x,
            y: first_y,
            width: 7,
            height: 7,
        },
        (0..49)
            .map(|at| (first_y as usize + at / 7) * size + first_x as usize + at % 7)
            .collect(),
        None,
        None,
        SizeClass::Medium,
    );
    carved += 49;
    if let Some(ref mut cb) = stage {
        cb("entrance-room", &l)
    }
    let kinds = [
        RoomKind::Alcove,
        RoomKind::Cavern,
        RoomKind::Lobed,
        RoomKind::Alcove,
        RoomKind::Cavern,
        RoomKind::Rectangle,
        RoomKind::Lobed,
    ];
    let sizes = [
        SizeClass::Small,
        SizeClass::Large,
        SizeClass::Medium,
        SizeClass::Small,
        SizeClass::Large,
        SizeClass::Medium,
        SizeClass::Medium,
    ];
    for slot in 1..(size * rows).div_ceil(65) {
        if l.rooms.len() >= (size * rows).div_ceil(115) {
            break;
        }
        let kind = kinds[slot % kinds.len()];
        let mut target = None;
        if constraints.spread_across_map {
            let mut sectors = vec![];
            for id in 0..12 {
                let col = id % 4;
                let row = id / 4;
                let x = ((col as f64 + 0.5) * size as f64 / 4.0).floor() as i32;
                let y = ((row as f64 + 0.5) * rows as f64 / 3.0).floor() as i32;
                let (mut capacity, mut occupied) = (0, 0);
                for yy in 1.max(row * rows / 3)..(rows - 1).min((row + 1) * rows / 3) {
                    for xx in 1.max(col * size / 4)..(size - 1).min((col + 1) * size / 4) {
                        let at = yy * size + xx;
                        if constraints.room_allowed(at) {
                            capacity += 1
                        }
                        if l.tiles[at] == "floor" {
                            occupied += 1
                        }
                    }
                }
                let distance = l
                    .rooms
                    .iter()
                    .map(|room| (room.center.x - x).abs() + (room.center.y - y).abs())
                    .min()
                    .unwrap();
                if capacity >= 24 {
                    sectors.push((occupied as f64 / capacity as f64, distance, y, x));
                }
            }
            sectors.sort_by(|a, b| {
                a.0.total_cmp(&b.0)
                    .then(a.1.cmp(&b.1))
                    .then(a.2.cmp(&b.2))
                    .then(a.3.cmp(&b.3))
            });
            if let Some(&(_, _, y, x)) = sectors.first() {
                target = Some(Point { x, y })
            }
        }
        let mut directed_parents: Vec<usize> = vec![];
        if let Some(t) = target {
            directed_parents = (0..l.rooms.len()).collect();
            directed_parents.sort_by_key(|&id| {
                let p = l.rooms[id].center;
                ((p.x - t.x).abs() + (p.y - t.y).abs(), id)
            });
            directed_parents.truncate(3)
        }
        let mut shape: Option<RoomShape> = None;
        for attempt in 0..180 {
            if attempt % 60 == 0 {
                let scheduled = sizes[slot % sizes.len()];
                let remaining = budget as i64 - carved as i64;
                let wanted = if constraints.maximum_floor_cells.is_none() {
                    scheduled
                } else if remaining < 85
                    || (kind == RoomKind::Rectangle && carved as f64 > budget as f64 * 0.45)
                {
                    SizeClass::Small
                } else if remaining < 200 && scheduled == SizeClass::Large {
                    SizeClass::Medium
                } else {
                    scheduled
                };
                let size_class = if attempt < 120 {
                    wanted
                } else if wanted == SizeClass::Large {
                    SizeClass::Medium
                } else {
                    SizeClass::Small
                };
                shape = Some(make_room_shape(kind, size_class, r));
            }
            let shape = shape.as_ref().unwrap();
            let width = shape.width;
            let height = shape.height;
            let random_parent =
                (integer(r, 0, l.rooms.len() as i32 - 1) as usize + attempt / 4) % l.rooms.len();
            let directed = target.is_some() && attempt % 3 != 2;
            let parent_id = if directed {
                directed_parents[(attempt / 12) % directed_parents.len()]
            } else {
                random_parent
            };
            let random_direction = DIRECTIONS[(integer(r, 0, 3) as usize + attempt) % 4];
            let parent = &l.rooms[parent_id];
            let direction = if directed && attempt % 4 < 2 {
                let t = target.unwrap();
                if (t.x - parent.center.x).abs() >= (t.y - parent.center.y).abs() {
                    DIRECTIONS[if t.x >= parent.center.x { 0 } else { 2 }]
                } else {
                    DIRECTIONS[if t.y >= parent.center.y { 1 } else { 3 }]
                }
            } else {
                random_direction
            };
            let side = Point {
                x: -direction.y,
                y: direction.x,
            };
            let cross = if direction.x != 0 {
                parent.center.y
            } else {
                parent.center.x
            } + integer(r, -2, 2);
            let edge:Vec<usize>=parent.cells.iter().copied().filter(|&at|if direction.x!=0{at/size}else{at%size}==cross as usize).collect();
            if edge.is_empty() {
                continue;
            }
            let from = if direction.x + direction.y > 0 {
                *edge.iter().max().unwrap()
            } else {
                *edge.iter().min().unwrap()
            };
            let sx = (from % size) as i32;
            let sy = (from / size) as i32;
            let local_cross = if direction.x != 0 {
                shape.center.y
            } else {
                shape.center.x
            };
            let entry_edge:Vec<usize>=shape.cells.iter().copied().filter(|&at|if direction.x!=0{at/width}else{at%width}==local_cross as usize).collect();
            let entry = if direction.x + direction.y > 0 {
                *entry_edge.iter().min().unwrap()
            } else {
                *entry_edge.iter().max().unwrap()
            };
            let both_natural = parent.kind != RoomKind::Rectangle && kind != RoomKind::Rectangle;
            let style = if !both_natural {
                PassageStyle::Constructed
            } else {
                [
                    PassageStyle::Opening,
                    PassageStyle::Broad,
                    PassageStyle::Natural,
                ][(slot + attempt) % 3]
            };
            let gap = if style == PassageStyle::Opening {
                integer(r, 2, 4)
            } else {
                integer(r, 3, 7)
            };
            let offset = if style == PassageStyle::Constructed {
                0
            } else {
                integer(r, -2, 2)
            };
            let x = sx + direction.x * (gap + 1) + side.x * offset - (entry % width) as i32;
            let y = sy + direction.y * (gap + 1) + side.y * offset - (entry / width) as i32;
            if !inside(size, rows, x, y)
                || !inside(size, rows, x + width as i32 - 1, y + height as i32 - 1)
            {
                continue;
            }
            let cells: Vec<usize> = shape
                .cells
                .iter()
                .map(|&at| (y as usize + at / width) * size + x as usize + at % width)
                .collect();
            if cells.iter().any(|&at| {
                !constraints.room_allowed(at)
                    || l.tiles[at] != "wall"
                    || surrounds[at].iter().any(|&n| l.tiles[n] != "wall")
            }) {
                continue;
            }
            let room_set: BTreeSet<usize> = cells.iter().copied().collect();
            let parent_set: BTreeSet<usize> = parent.cells.iter().copied().collect();
            let Ok(passage) = make_room_passage(
                size,
                Point { x: sx, y: sy },
                direction,
                gap,
                offset,
                style,
                parent.kind == RoomKind::Rectangle,
                kind == RoomKind::Rectangle,
                if style == PassageStyle::Natural && gap >= 5 {
                    if slot % 2 != 0 { 1.0 } else { -1.0 }
                } else {
                    0.0
                },
            ) else {
                continue;
            };
            let hall: Vec<usize> = passage
                .cells
                .into_iter()
                .filter(|at| !room_set.contains(at) && !parent_set.contains(at))
                .collect();
            let path: Vec<usize> = passage
                .path
                .into_iter()
                .filter(|at| !room_set.contains(at) && !parent_set.contains(at))
                .collect();
            let reserve = if constraints.maximum_floor_cells.is_none() {
                0
            } else {
                8usize.saturating_sub(l.rooms.len() + 1) * 20
            };
            if hall.is_empty()
                || path.is_empty()
                || carved + cells.len() + hall.len() + reserve > budget
                || hall.iter().any(|&at| {
                    !inside(size, rows, (at % size) as i32, (at / size) as i32)
                        || !constraints.route_allowed(at)
                        || l.tiles[at] != "wall"
                        || surrounds[at]
                            .iter()
                            .any(|&n| l.tiles[n] != "wall" && !parent_set.contains(&n))
                })
            {
                continue;
            }
            if path.windows(2).any(|p| {
                (p[0] % size).abs_diff(p[1] % size) + (p[0] / size).abs_diff(p[1] / size) != 1
            }) {
                continue;
            }
            let near_exit = |at: usize| {
                ((at % size) as i32 - exit.x).abs() + ((at / size) as i32 - exit.y).abs() <= 1
            };
            if (parent.kind == RoomKind::Rectangle && near_exit(path[0]))
                || (kind == RoomKind::Rectangle && near_exit(*path.last().unwrap()))
            {
                continue;
            }
            carved += cells.len() + hall.len();
            let connector_id = l.connectors.len();
            let room_id = add_room(
                &mut l,
                kind,
                Bounds {
                    x,
                    y,
                    width,
                    height,
                },
                cells,
                Some(parent_id),
                Some(connector_id),
                shape.size_class,
            );
            let mut c = LayoutConnector::new(
                connector_id,
                hall,
                vec![parent_id, room_id],
                parent_id,
                "growth",
            );
            c.style = Some(style);
            if c.cells.len() != path.len() || style != PassageStyle::Constructed {
                c.path = Some(path.clone())
            }
            for &at in &c.cells {
                l.tiles[at] = "floor".into()
            }
            l.connectors.push(c);
            add_door(
                &mut l,
                size,
                rows,
                path[0],
                parent_id,
                connector_id,
                entrance_at,
                exit_at,
            );
            add_door(
                &mut l,
                size,
                rows,
                *path.last().unwrap(),
                room_id,
                connector_id,
                entrance_at,
                exit_at,
            );
            break;
        }
    }
    if let Some(ref mut cb) = stage {
        cb("room-accretion", &l)
    }
    if l.tiles[exit_at] == "wall" {
        let mut prev = vec![usize::MAX; l.tiles.len()];
        let mut queue = vec![exit_at];
        let mut jambs = BTreeSet::new();
        for &at in l.doors.keys() {
            for &side in &neighbors[at] {
                if l.tiles[side] == "wall" {
                    jambs.insert(side);
                }
            }
        }
        prev[exit_at] = exit_at;
        let mut join = None;
        let mut head = 0;
        while head < queue.len() && join.is_none() {
            let cur = queue[head];
            head += 1;
            for &at in &neighbors[cur] {
                if prev[at] != usize::MAX
                    || l.doors.contains_key(&at)
                    || jambs.contains(&at)
                    || !constraints.route_allowed(at)
                {
                    continue;
                }
                prev[at] = cur;
                if l.tiles[at] == "floor" {
                    join = Some(at);
                    break;
                }
                queue.push(at)
            }
        }
        let Some(join) = join else {
            return Err("Dungeon exit cannot reach its entrance room.".into());
        };
        let mut cells = vec![];
        let mut at = prev[join];
        loop {
            cells.push(at);
            if at == exit_at {
                break;
            }
            at = prev[at]
        }
        let parent_connector = l
            .connectors
            .iter()
            .find(|c| c.cells.contains(&join))
            .map(|c| c.id);
        let parent_id = l
            .rooms
            .iter()
            .find(|room| room.cells.contains(&join))
            .map(|room| room.id)
            .or_else(|| parent_connector.map(|id| l.connectors[id].parent_room_id))
            .ok_or("Unowned exit route")?;
        let adjoining: BTreeSet<usize> = cells
            .iter()
            .flat_map(|&at| neighbors[at].iter().copied())
            .collect();
        let room_ids: Vec<usize> = l
            .rooms
            .iter()
            .filter(|room| room.cells.iter().any(|at| adjoining.contains(at)))
            .map(|room| room.id)
            .collect();
        let connector_id = l.connectors.len();
        let door_at = cells[0];
        let side = if door_at.abs_diff(join) == size {
            1
        } else {
            size
        };
        let mut c = LayoutConnector::new(
            connector_id,
            cells,
            if room_ids.is_empty() {
                vec![parent_id]
            } else {
                room_ids
            },
            parent_id,
            "exit",
        );
        c.parent_connector_id = parent_connector;
        for &at in &c.cells {
            l.tiles[at] = "floor".into()
        }
        carved += c.cells.len();
        l.connectors.push(c);
        if parent_connector.is_none()
            && l.tiles[door_at - side] == "wall"
            && l.tiles[door_at + side] == "wall"
        {
            add_door(
                &mut l,
                size,
                rows,
                door_at,
                parent_id,
                connector_id,
                entrance_at,
                exit_at,
            )
        }
    }
    let mut jambs = BTreeSet::new();
    for &at in l.doors.keys() {
        if l.tiles[at - 1] == "wall" && l.tiles[at + 1] == "wall" {
            jambs.insert(at - 1);
            jambs.insert(at + 1);
        }
        if l.tiles[at - size] == "wall" && l.tiles[at + size] == "wall" {
            jambs.insert(at - size);
            jambs.insert(at + size);
        }
    }
    loop {
        let mut changed = false;
        for y in 1..rows - 2 {
            for x in 1..size - 2 {
                let a = y * size + x;
                let b = a + 1;
                let c = a + size;
                let d = c + 1;
                let sides = if l.tiles[a] != "wall"
                    && l.tiles[d] != "wall"
                    && l.tiles[b] == "wall"
                    && l.tiles[c] == "wall"
                {
                    Some([b, c])
                } else if l.tiles[b] != "wall"
                    && l.tiles[c] != "wall"
                    && l.tiles[a] == "wall"
                    && l.tiles[d] == "wall"
                {
                    Some([a, d])
                } else {
                    None
                };
                let Some(sides) = sides else { continue };
                let bridge = sides
                    .iter()
                    .copied()
                    .find(|at| !jambs.contains(at) && constraints.route_allowed(*at));
                let Some(bridge) = bridge else {
                    if sides.iter().any(|&at| !constraints.route_allowed(at)) {
                        continue;
                    }
                    return Err("Diagonal doorway has no cardinal approach.".into());
                };
                l.tiles[bridge] = "floor".into();
                carved += 1;
                let adjoining = &neighbors[bridge];
                let parent_connector = l
                    .connectors
                    .iter()
                    .find(|c| c.cells.iter().any(|at| adjoining.contains(at)))
                    .map(|c| c.id);
                let room_ids: Vec<usize> = l
                    .rooms
                    .iter()
                    .filter(|room| room.cells.iter().any(|at| adjoining.contains(at)))
                    .map(|room| room.id)
                    .collect();
                let parent_id = parent_connector
                    .map(|id| l.connectors[id].parent_room_id)
                    .or_else(|| room_ids.first().copied())
                    .ok_or("Unowned cardinal room bridge.")?;
                let mut c = LayoutConnector::new(
                    l.connectors.len(),
                    vec![bridge],
                    if room_ids.is_empty() {
                        vec![parent_id]
                    } else {
                        room_ids
                    },
                    parent_id,
                    "corner",
                );
                c.parent_connector_id = parent_connector;
                l.connectors.push(c);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    if carved > max {
        return Err("Floor has insufficient space within the planned scaffold budget.".into());
    }
    Ok(l)
}
