//! Exact bounded authored Godbeast room placement, O(V) geometry scans.
use super::Point;
use crate::model::*;
use indexmap::IndexSet;
pub fn reserve_godbeast_room(
    tiles: &mut [String],
    kind: &str,
    r: &mut dyn FnMut() -> f64,
    avoid: &[Point],
    preferred: &[Point],
    core: &IndexSet<usize>,
) -> Result<GodbeastEvent, String> {
    reserve_godbeast_room_revision(tiles, kind, r, avoid, preferred, core, 29.0)
}

/// R38 reserves the complete stair footprint before choosing distant quest loot.
/// O(V*A), A<=2 stair cells, O(V) candidate storage; historical draws are unchanged.
pub fn reserve_godbeast_room_revision(
    tiles: &mut [String], kind: &str, r: &mut dyn FnMut() -> f64,
    avoid: &[Point], preferred: &[Point], core: &IndexSet<usize>, revision: f64,
) -> Result<GodbeastEvent, String> {
    if kind == "Borge" { return crate::borge::reserve_room(tiles, avoid); }
    if !["Mercuria", "Zillennia", "Veridian", "Phantasmii", "Epidite", "Ahvamitt"].contains(&kind) {
        return Err("Unknown Godbeast kind".into());
    }
    let w = if tiles.len() == 2291 {
        79
    } else {
        (tiles.len() as f64).sqrt() as usize
    };
    if w == 0 || tiles.len() % w != 0 {
        return Err("Invalid Godbeast grid".into());
    }
    let h = tiles.len() / w;
    let clear = |p: &Point, d: i32| {
        avoid
            .iter()
            .all(|a| (a.x - p.x).abs() > d || (a.y - p.y).abs() > d)
    };
    let mut candidates = vec![];
    for y in 7..h as i32 - 7 {
        for x in 7..w as i32 - 7 {
            let p = Point { x, y };
            if clear(&p, 7) {
                candidates.push(p)
            }
        }
    }
    let rooms: Vec<Point> = preferred
        .iter()
        .copied()
        .filter(|p| {
            p.x >= 5
                && p.y >= 5
                && p.x < w as i32 - 5
                && p.y < h as i32 - 5
                && clear(p, 4)
                && (p.y - 4..=p.y + 4).all(|y| {
                    (p.x - 4..=p.x + 4).all(|x| {
                        ["floor", "grass"].contains(&tiles[y as usize * w + x as usize].as_str())
                    })
                })
        })
        .collect();
    let pool = if rooms.is_empty() {
        &candidates
    } else {
        &rooms
    };
    let mut least = usize::MAX;
    let mut choices = vec![];
    for p in pool {
        let mut cost = 0;
        for y in p.y - 4..=p.y + 4 {
            for x in p.x - 4..=p.x + 4 {
                let at = y as usize * w + x as usize;
                let t = &tiles[at];
                if t == "floor" || t == "grass" {
                    continue;
                }
                cost += if core.contains(&at) { 1000 } else { 0 };
                cost += if t == "wall" { 10 } else { 100 };
            }
        }
        if cost < least {
            least = cost;
            choices.clear()
        }
        if cost == least {
            choices.push(*p)
        }
    }
    let roll = r();
    let p = choices
        .get((roll * choices.len() as f64).floor() as usize)
        .copied()
        .unwrap_or(Point {
            x: (w / 2) as i32,
            y: (h / 2) as i32,
        });
    if p.x < 4 || p.y < 4 || p.x + 4 >= w as i32 || p.y + 4 >= h as i32 {
        return Err("Grid too small for Godbeast room".into());
    }
    let Point { x, y } = p;
    for yy in y - 4..=y + 4 {
        for xx in x - 4..=x + 4 {
            tiles[yy as usize * w + xx as usize] = "floor".into();
        }
    }
    let mut e = GodbeastEvent {
        kind: kind.into(),
        x: x as f64,
        y: y as f64,
        room: GodbeastEventRoom {
            left: (x - 4) as f64,
            right: (x + 4) as f64,
            top: (y - 4) as f64,
            bottom: (y + 4) as f64,
        },
        ..Default::default()
    };
    if kind == "Epidite" {
        tiles[y as usize * w + x as usize] = "wall".into();
        e.epidite_age = 0.0.into();
    }
    if kind == "Phantasmii" {
        e.altar = Field::Value(GodbeastEventTap { x: x as f64, y: y as f64 });
    }
    if kind == "Mercuria" {
        e.well = Field::Value(GodbeastEventWell {
            x: x as f64,
            y: y as f64,
        });
        e.lever = Field::Value(GodbeastEventLever {
            x: (x - 3) as f64,
            y: (y + 2) as f64,
        });
    }
    if kind == "Veridian" {
        tiles[y as usize * w + x as usize] = "tree".into();
        let dry: Vec<usize> = (w + 1..tiles.len() - w - 1)
            .filter(|&i| {
                ["floor", "grass"].contains(&tiles[i].as_str())
                    && i % w > 0
                    && i % w < w - 1
                    && ((i % w) as f64 - x as f64).hypot((i / w) as f64 - y as f64) > 10.
                    && (revision < 38.0 || clear(&Point { x: (i % w) as i32, y: (i / w) as i32 }, 1))
            })
            .collect();
        let roll = r();
        let fallback = y as usize * w + x as usize - 3;
        let i = dry
            .get((roll * dry.len() as f64).floor() as usize)
            .copied()
            .or_else(|| {
                (revision < 38.0 || (clear(&Point { x: x - 3, y }, 1)
                    && ["floor", "grass"].contains(&tiles[fallback].as_str())))
                    .then_some(fallback)
            })
            .ok_or("No safe Godbeast tap location")?;
        e.tap = Field::Value(GodbeastEventTap {
            x: (i % w) as f64,
            y: (i / w) as f64,
        });
    }
    Ok(e)
}
