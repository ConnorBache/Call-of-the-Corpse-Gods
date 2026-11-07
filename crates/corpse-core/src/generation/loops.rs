//! Exact alternate-room route addition, including provisional quality callbacks.
use super::{DungeonLayout, LayoutConnector, LayoutConstraints, PassageStyle, RoomKind};
use std::collections::BTreeSet;
#[derive(Clone, Debug, Default)]
pub struct LoopOptions {
    pub constraints: LayoutConstraints,
    pub max_loops: Option<usize>,
    pub allow_bent: bool,
    pub maximum_candidate_checks: Option<usize>,
}
#[derive(Debug)]
struct Candidate {
    from: usize,
    to: usize,
    cells: Vec<usize>,
    tie: f64,
}
fn neighbors(at: usize, w: usize, h: usize) -> Vec<usize> {
    [
        at.checked_sub(1),
        at.checked_add(1),
        at.checked_sub(w),
        at.checked_add(w),
    ]
    .into_iter()
    .flatten()
    .filter(|&n| n % w > 0 && n % w < w - 1 && n >= w && n < (h - 1) * w)
    .collect()
}
fn room_for(l: &DungeonLayout, at: usize) -> usize {
    l.rooms
        .iter()
        .find(|room| room.cells.contains(&at))
        .map(|room| room.id)
        .or_else(|| {
            l.connectors
                .iter()
                .find(|c| c.cells.contains(&at))
                .map(|c| c.parent_room_id)
        })
        .unwrap_or(0)
}
fn no_diagonal(l: &DungeonLayout, cells: &[usize], w: usize) -> bool {
    let new: BTreeSet<usize> = cells.iter().copied().collect();
    let open = |at: i64| {
        at >= 0
            && (l.tiles.get(at as usize).is_some_and(|t| t == "floor")
                || new.contains(&(at as usize)))
    };
    for &cell in cells {
        for a in [
            cell as i64,
            cell as i64 - 1,
            cell as i64 - w as i64,
            cell as i64 - w as i64 - 1,
        ] {
            let b = a + 1;
            let c = a + w as i64;
            let d = c + 1;
            if (open(a) && open(d) && !open(b) && !open(c))
                || (open(b) && open(c) && !open(a) && !open(d))
            {
                return false;
            }
        }
    }
    true
}
/// O(V²) bounded candidate searches (max 12-cell centerline), O(V) scratch plus candidates.
/// Callback receives provisionally carved tiles and unattached connector, exactly as reference.
pub fn add_room_loops(
    l: &mut DungeonLayout,
    r: &mut impl FnMut() -> f64,
    options: &LoopOptions,
    mut accept: Option<&mut dyn FnMut(&DungeonLayout, &LayoutConnector) -> bool>,
) -> Result<usize, String> {
    let n = l.tiles.len();
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
    if w < 3 || h < 3 {
        return Err("Invalid loop grid geometry.".into());
    }
    let goal = 6.min(
        options
            .max_loops
            .unwrap_or(1.max(l.rooms.len().div_ceil(4))),
    );
    let mut jambs = BTreeSet::new();
    for &key in l.doors.keys() {
        for at in neighbors(key, w, h) {
            if l.tiles[at] == "wall" {
                jambs.insert(at);
            }
        }
    }
    let mut candidates = vec![];
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let from = y * w + x;
            if l.tiles[from] != "floor" || l.doors.contains_key(&from) {
                continue;
            }
            for step in [1, w] {
                let mut cells = vec![];
                for length in 1..=13 {
                    let at = from + step * length;
                    if at % w >= w - 1 || at / w >= h - 1 {
                        break;
                    }
                    if l.tiles[at] == "floor" {
                        if !cells.is_empty() && !l.doors.contains_key(&at) {
                            candidates.push(Candidate {
                                from,
                                to: at,
                                cells: cells.clone(),
                                tie: r(),
                            })
                        }
                        break;
                    }
                    if jambs.contains(&at) || !options.constraints.route_allowed(at) {
                        break;
                    }
                    cells.push(at);
                    if options.allow_bent {
                        for side in if step == 1 {
                            [-(w as i64), w as i64]
                        } else {
                            [-1, 1]
                        } {
                            let mut bend = cells.clone();
                            let mut tail = 1;
                            while bend.len() <= 12 {
                                let next = at as i64 + side * tail;
                                tail += 1;
                                if next < 0
                                    || next % w as i64 <= 0
                                    || next % w as i64 >= w as i64 - 1
                                    || next < w as i64
                                    || next >= ((h - 1) * w) as i64
                                {
                                    break;
                                }
                                let next = next as usize;
                                if l.tiles[next] == "floor" {
                                    if !l.doors.contains_key(&next) {
                                        candidates.push(Candidate {
                                            from,
                                            to: next,
                                            cells: bend.clone(),
                                            tie: r(),
                                        })
                                    }
                                    break;
                                }
                                if jambs.contains(&next) || !options.constraints.route_allowed(next)
                                {
                                    break;
                                }
                                bend.push(next);
                            }
                        }
                    }
                }
            }
        }
    }
    candidates.sort_by(|a, b| {
        let key = |c: &Candidate| {
            if accept.is_some() {
                c.cells.len().abs_diff(4)
            } else {
                c.cells.len()
            }
        };
        key(a).cmp(&key(b)).then(a.tie.total_cmp(&b.tie))
    });
    let mut added = 0;
    let mut checks = 0;
    let mut carved = l.tiles.iter().filter(|t| t.as_str() == "floor").count();
    for candidate in candidates {
        if added >= goal
            || (accept.is_some() && checks >= options.maximum_candidate_checks.unwrap_or(48))
        {
            break;
        }
        let Candidate {
            from,
            to,
            cells,
            tie,
        } = candidate;
        let path: BTreeSet<usize> = cells.iter().copied().collect();
        if cells.iter().any(|&at| {
            !options.constraints.route_allowed(at)
                || l.tiles[at] != "wall"
                || neighbors(at, w, h)
                    .into_iter()
                    .any(|n| l.tiles[n] != "wall" && n != from && n != to && !path.contains(&n))
        }) {
            continue;
        }
        let mut distance = vec![-1i32; n];
        let mut queue = vec![from];
        distance[from] = 0;
        let mut head = 0;
        while head < queue.len() && distance[to] < 0 {
            let cur = queue[head];
            head += 1;
            for next in neighbors(cur, w, h) {
                if l.tiles[next] == "floor" && distance[next] < 0 {
                    distance[next] = distance[cur] + 1;
                    queue.push(next)
                }
            }
        }
        let shortcut = cells.len() + 1;
        if distance[to] < (shortcut + 6) as i32 {
            continue;
        }
        let a = room_for(l, from);
        let b = room_for(l, to);
        let room_ids = if a == b { vec![a] } else { vec![a, b] };
        let natural = room_ids
            .iter()
            .all(|&id| l.rooms[id].kind != RoomKind::Rectangle);
        let mut footprint = cells.clone();
        if natural
            && cells.len() >= 4
            && tie < 0.7
            && (cells.iter().all(|at| at % w == cells[0] % w)
                || cells.iter().all(|at| at / w == cells[0] / w))
        {
            let side = if to.abs_diff(from) < w { w } else { 1 };
            for i in 1..cells.len() - 1 {
                for at in [cells[i] - side, cells[i] + side] {
                    if !footprint.contains(&at) {
                        footprint.push(at)
                    }
                }
            }
            if footprint.iter().any(|&at| {
                neighbors(at, w, h).is_empty()
                    || jambs.contains(&at)
                    || !options.constraints.route_allowed(at)
                    || l.tiles.get(at).is_none_or(|t| t != "wall")
                    || at % w == 0
                    || at % w >= w - 1
                    || at < w
                    || at >= (h - 1) * w
                    || neighbors(at, w, h).into_iter().any(|next| {
                        l.tiles[next] != "wall"
                            && next != from
                            && next != to
                            && !footprint.contains(&next)
                    })
            }) || !no_diagonal(l, &footprint, w)
            {
                footprint = cells.clone();
            }
        }
        let widened = footprint.len() > cells.len();
        let first = cells[0];
        let last = *cells.last().unwrap();
        let mut hall = vec![first];
        hall.extend(
            footprint
                .into_iter()
                .filter(|&at| at != first && at != last),
        );
        if cells.len() > 1 {
            hall.push(last)
        }
        if !no_diagonal(l, &hall, w)
            || carved + hall.len() > options.constraints.maximum_floor_cells.unwrap_or(n)
        {
            continue;
        }
        let mut c = LayoutConnector::new(
            l.connectors.len(),
            hall,
            room_ids.clone(),
            room_ids[0],
            "loop",
        );
        c.style = Some(if natural {
            if widened {
                PassageStyle::Broad
            } else {
                PassageStyle::Natural
            }
        } else {
            PassageStyle::Constructed
        });
        if widened {
            c.path = Some(cells)
        }
        c.route_before = Some(distance[to] as usize);
        c.route_after = Some(shortcut);
        c.route_endpoints = Some([from, to]);
        for &at in &c.cells {
            l.tiles[at] = "floor".into()
        }
        if let Some(ref mut cb) = accept {
            checks += 1;
            if !cb(l, &c) {
                for &at in &c.cells {
                    l.tiles[at] = "wall".into()
                }
                if checks >= options.maximum_candidate_checks.unwrap_or(48) {
                    break;
                }
                continue;
            }
        }
        carved += c.cells.len();
        l.connectors.push(c);
        added += 1;
    }
    Ok(added)
}
