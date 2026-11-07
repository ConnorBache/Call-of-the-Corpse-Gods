use crate::{ConnectorMetadata, Field, Floor, GenerationStageRecord, RoomMetadata};
use indexmap::{IndexMap, IndexSet};
pub const GENERATION_STAGES: [&str; 10] = [
    "entrance-room",
    "room-accretion",
    "loops",
    "natural-terrain",
    "environment",
    "trap-setups",
    "cleanup",
    "hordes",
    "items",
    "machine-enemies",
];
pub fn generation_stage_record(
    id: &str,
    tiles: &[String],
    rooms: &[RoomMetadata],
    connectors: &[ConnectorMetadata],
    details: Option<IndexMap<String, f64>>,
) -> GenerationStageRecord {
    let mut hash = 2166136261u32;
    let mut open = 0;
    for t in tiles {
        let code = match t.as_str() {
            "wall" => 1,
            "floor" => 2,
            "water" => 3,
            "shallow-water" => 4,
            "chasm" => 5,
            "grass" => 6,
            "tree" => 7,
            "up" => 8,
            "down" => 9,
            "pit" => 10,
            _ => 0,
        };
        hash = (hash ^ code).wrapping_mul(16777619);
        if !["wall", "tree", "chasm"].contains(&t.as_str()) {
            open += 1
        }
    }
    GenerationStageRecord {
        id: id.into(),
        tile_hash: hash as f64,
        open_cells: open as f64,
        rooms: rooms.len() as f64,
        connectors: connectors.len() as f64,
        loops: connectors.iter().filter(|c| c.purpose == "loop").count() as f64,
        details: details.map(Field::Value).unwrap_or(Field::Missing),
    }
}
pub fn record_generation_stage(
    f: &mut Floor,
    id: &str,
    details: Option<IndexMap<String, f64>>,
) -> Result<(), String> {
    let Some(g) = f.generation.as_ref() else {
        return Ok(());
    };
    if let Some(previous) = g.stages.last() {
        let index = |s: &str| {
            GENERATION_STAGES
                .iter()
                .position(|&v| v == s)
                .map(|i| i as i32)
                .unwrap_or(-1)
        };
        if index(id) <= index(&previous.id) {
            return Err(format!(
                "Generation stage out of order: {} -> {id}",
                previous.id
            ));
        }
    }
    let (rooms, connectors) = if let Some(graph) = f.room_graph.as_ref() {
        (graph.rooms.as_slice(), graph.connectors.as_slice())
    } else if let (Some(rooms), Some(connectors)) =
        (f.dungeon_rooms.as_ref(), f.dungeon_connectors.as_ref())
    {
        (rooms.as_slice(), connectors.as_slice())
    } else {
        (&[][..], &[][..])
    };
    let record = generation_stage_record(id, &f.tiles, rooms, connectors, details);
    f.generation.as_mut().unwrap().stages.push(record);
    Ok(())
}
fn integer(n: f64, max: f64) -> bool {
    n.is_finite() && n.fract() == 0.0 && n >= 0.0 && n <= max && n <= 9007199254740991.0
}
fn optional<T>(f: &Field<T>, check: impl FnOnce(&T) -> bool) -> bool {
    match f {
        Field::Missing => true,
        Field::Null => false,
        Field::Value(v) => check(v),
    }
}
fn cell_list(v: &[f64], cells: usize) -> bool {
    v.len() <= cells && v.iter().all(|&at| integer(at, cells as f64 - 1.0))
}
fn unique(v: &[f64]) -> bool {
    let mut set = IndexSet::new();
    v.iter().all(|&n| set.insert(n as u64))
}
fn bounds(x: f64, y: f64, width: f64, height: f64, w: usize, h: usize) -> bool {
    integer(x, w as f64 - 1.0)
        && integer(y, h as f64 - 1.0)
        && integer(width, w as f64)
        && integer(height, h as f64)
        && width >= 1.0
        && height >= 1.0
        && x + width <= w as f64
        && y + height <= h as f64
}
/// Optional generation diagnostics validation. Typed schema handles array/object shape;
/// this enforces finite indices, ordered stages, provenance ownership and graph invariants.
pub fn validate_floor_generation(f: &Floor) -> bool {
    let cells = f.tiles.len();
    let w = f.width();
    let h = f.height();
    if w == 0 || h == 0 {
        return false;
    }
    if !optional(&f.enemy_blessing_budget, |&v| {
        integer(v, 48.0) && v >= f.depth + 6.0 && v <= f.depth + 8.0
    }) {
        return false;
    }
    for list in [&f.basic_vent_cells, &f.generation_bridges] {
        if !optional(list, |v| cell_list(v, cells)) {
            return false;
        }
    }
    if !optional(&f.generation, |g| {
        if g.version != 1.0 || g.stages.len() > 10 {
            return false;
        }
        if !optional(&g.stair_recesses, |v| {
            v.len() <= 2
                && unique(&v.iter().map(|r| r.at).collect::<Vec<_>>())
                && v.iter().all(|r| {
                    integer(r.at, cells as f64 - 1.0)
                        && r.at % w as f64 > 0.0
                        && r.at % (w as f64) < (w - 1) as f64
                        && r.at >= w as f64
                        && r.at < ((h - 1) * w) as f64
                        && ["north", "east", "south", "west"].contains(&r.facing.as_str())
                })
        }) {
            return false;
        }
        for list in [&g.terrain_spine, &g.natural_walls] {
            if !optional(list, |v| cell_list(v, cells) && unique(v)) {
                return false;
            }
        }
        if !optional(&g.terrain, |t| {
            t.version == 1.0
                && ["horizontal", "vertical"].contains(&t.orientation.as_str())
                && integer(t.phase, 65535.0)
                && integer(t.changed_cells, cells as f64)
                && optional(&t.archetype, |id| {
                    super::geology::known_geology_archetype(&f.domain, id)
                })
        }) {
            return false;
        }
        if !optional(&g.landmarks, |landmarks| {
            if landmarks.len() > 24 {
                return false;
            }
            let (rooms, connectors) = if let Some(graph) = f.room_graph.as_ref() {
                (graph.rooms.len(), graph.connectors.len())
            } else {
                (
                    f.dungeon_rooms.as_ref().map_or(0, Vec::len),
                    f.dungeon_connectors.as_ref().map_or(0, Vec::len),
                )
            };
            let mut claimed = IndexSet::new();
            for (id, l) in landmarks.iter().enumerate() {
                if l.id != id as f64
                    || !optional(&l.parent_id, |&v| integer(v, id as f64 - 1.0))
                    || ![
                        "cavern", "lake", "river", "fissure", "grove", "island", "clearing",
                    ]
                    .contains(&l.kind.as_str())
                    || !["major", "medium", "minor"].contains(&l.scale.as_str())
                    || !cell_list(&l.cells, cells)
                    || !integer(l.center.x, w as f64 - 1.0)
                    || !integer(l.center.y, h as f64 - 1.0)
                    || !bounds(
                        l.bounds.x,
                        l.bounds.y,
                        l.bounds.width,
                        l.bounds.height,
                        w,
                        h,
                    )
                    || l.room_ids.len() > rooms
                    || !l.room_ids.iter().all(|&v| integer(v, rooms as f64 - 1.0))
                    || l.connector_ids.len() > connectors
                    || !l
                        .connector_ids
                        .iter()
                        .all(|&v| integer(v, connectors as f64 - 1.0))
                {
                    return false;
                }
                for &at in &l.cells {
                    if !claimed.insert(at as usize) {
                        return false;
                    }
                }
            }
            true
        }) {
            return false;
        }
        let mut previous = -1;
        for stage in &g.stages {
            let position = GENERATION_STAGES
                .iter()
                .position(|&s| s == stage.id)
                .map(|p| p as i32)
                .unwrap_or(-1);
            if position <= previous
                || !integer(stage.tile_hash, 4294967295.0)
                || ![stage.open_cells, stage.rooms, stage.connectors, stage.loops]
                    .iter()
                    .all(|&v| integer(v, cells as f64))
            {
                return false;
            }
            previous = position;
            if !optional(&stage.details, |d| {
                d.len() <= 64 && d.values().all(|v| v.is_finite() && *v >= 0.0)
            }) {
                return false;
            }
        }
        true
    }) {
        return false;
    }
    // roomGraph null is nullish and falls back in source; partial legacy graph remains optional.
    let graph = if let Some(g) = f.room_graph.as_ref() {
        Some((g.rooms.as_slice(), g.connectors.as_slice()))
    } else if let (Some(r), Some(c)) = (f.dungeon_rooms.as_ref(), f.dungeon_connectors.as_ref()) {
        Some((r.as_slice(), c.as_slice()))
    } else {
        None
    };
    if let Some((rooms, connectors)) = graph {
        if rooms.len() > cells || connectors.len() > cells {
            return false;
        }
        for (id, r) in rooms.iter().enumerate() {
            if r.id != id as f64
                || ![
                    "rectangle",
                    "circle",
                    "eye",
                    "open",
                    "cavern",
                    "lobed",
                    "alcove",
                ]
                .contains(&r.kind.as_str())
                || !cell_list(&r.cells, cells)
                || !cell_list(&r.doors, cells)
                || !optional(&r.size_class, |s| {
                    ["small", "medium", "large"].contains(&s.as_str())
                })
                || !r.parent_room_id.is_none_or(|v| integer(v, id as f64 - 1.0))
                || !r
                    .connector_id
                    .is_none_or(|v| integer(v, connectors.len() as f64 - 1.0))
                || !integer(r.center.x, w as f64 - 1.0)
                || !integer(r.center.y, h as f64 - 1.0)
                || !bounds(
                    r.bounds.x,
                    r.bounds.y,
                    r.bounds.width,
                    r.bounds.height,
                    w,
                    h,
                )
            {
                return false;
            }
        }
        for (id, c) in connectors.iter().enumerate() {
            if c.id != id as f64
                || c.kind != "hallway"
                || !["growth", "exit", "corner", "loop"].contains(&c.purpose.as_str())
                || !cell_list(&c.cells, cells)
                || !cell_list(&c.doors, cells)
                || !optional(&c.route_endpoints, |&(a, b)| {
                    cell_list(&[a, b], cells) && a != b
                })
                || !optional(&c.style, |s| {
                    ["constructed", "natural", "broad", "opening"].contains(&s.as_str())
                })
                || !optional(&c.path, |p| {
                    cell_list(p, cells) && {
                        let set: IndexSet<usize> = c.cells.iter().map(|&v| v as usize).collect();
                        p.iter().all(|&v| set.contains(&(v as usize)))
                    }
                })
                || !integer(c.parent_room_id, rooms.len() as f64 - 1.0)
                || !c
                    .room_ids
                    .iter()
                    .all(|&v| integer(v, rooms.len() as f64 - 1.0))
                || !optional(&c.parent_connector_id, |&v| integer(v, id as f64 - 1.0))
                || !optional(&c.route_before, |&v| integer(v, cells as f64))
                || !optional(&c.route_after, |&v| integer(v, cells as f64))
            {
                return false;
            }
            if c.purpose == "loop" {
                let (Some(before), Some(after)) = (c.route_before.as_ref(), c.route_after.as_ref())
                else {
                    return false;
                };
                if before <= after {
                    return false;
                }
            }
        }
    }
    true
}
