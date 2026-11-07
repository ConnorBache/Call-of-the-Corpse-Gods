//! Exact room-mask and passage primitives from frozen v48. Array order is semantic.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoomKind {
    Rectangle,
    Cavern,
    Lobed,
    Alcove,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SizeClass {
    Small,
    Medium,
    Large,
}
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomShape {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<usize>,
    pub center: Point,
    pub size_class: SizeClass,
}
fn integer(r: &mut impl FnMut() -> f64, lo: usize, hi: usize) -> usize {
    lo + (r() * (hi - lo + 1) as f64).floor() as usize
}
fn connected(mask: &[u8], w: usize, center: usize) -> Vec<usize> {
    let mut seen = vec![false; mask.len()];
    let mut components = Vec::new();
    for at in 0..mask.len() {
        if mask[at] == 0 || seen[at] {
            continue;
        }
        let mut q = vec![at];
        seen[at] = true;
        let mut head = 0;
        while head < q.len() {
            let cur = q[head];
            head += 1;
            let x = cur % w;
            for next in [
                if x > 0 { Some(cur - 1) } else { None },
                if x < w - 1 { Some(cur + 1) } else { None },
                cur.checked_sub(w),
                cur.checked_add(w),
            ]
            .into_iter()
            .flatten()
            {
                if next < mask.len() && mask[next] != 0 && !seen[next] {
                    seen[next] = true;
                    q.push(next)
                }
            }
        }
        components.push(q);
    }
    if let Some(i) = components.iter().position(|c| c.contains(&center)) {
        return components.remove(i);
    }
    components.sort_by_key(|c| std::cmp::Reverse(c.len()));
    components.into_iter().next().unwrap_or_default()
}
/// Two cellular passes plus monotone diagonal repair. O(V²) worst case, O(V) memory; V<=360.
pub fn make_room_shape(
    kind: RoomKind,
    size_class: SizeClass,
    r: &mut impl FnMut() -> f64,
) -> RoomShape {
    let (mut w, mut h) = match size_class {
        SizeClass::Large => (integer(r, 18, 24), integer(r, 11, 15)),
        SizeClass::Medium => (integer(r, 10, 15), integer(r, 7, 11)),
        SizeClass::Small => (integer(r, 5, 9), integer(r, 5, 7)),
    };
    if r() < 0.28 {
        std::mem::swap(&mut w, &mut h)
    }
    let mut mask = vec![0; w * h];
    let center = Point {
        x: (w / 2) as i32,
        y: (h / 2) as i32,
    };
    if kind == RoomKind::Rectangle {
        mask.fill(1)
    } else {
        let horizontal = w >= h;
        let side = if r() < 0.5 { -1.0 } else { 1.0 };
        let flip = if r() < 0.5 { -1.0 } else { 1.0 };
        let wf = w as f64;
        let hf = h as f64;
        let lobes = [
            (
                wf * (0.48 + 0.08 * side),
                hf * (0.48 - 0.07 * flip),
                wf * 0.36,
                hf * 0.38,
            ),
            (
                wf * if horizontal { 0.24 } else { 0.56 + 0.13 * side },
                hf * if horizontal { 0.5 + 0.17 * flip } else { 0.24 },
                wf * if horizontal { 0.24 } else { 0.34 },
                hf * if horizontal { 0.31 } else { 0.24 },
            ),
            (
                wf * if horizontal { 0.78 } else { 0.48 - 0.17 * side },
                hf * if horizontal { 0.5 - 0.18 * flip } else { 0.79 },
                wf * if horizontal { 0.23 } else { 0.31 },
                hf * if horizontal { 0.29 } else { 0.22 },
            ),
        ];
        let ix = wf * (0.36 + r() * 0.25);
        let iy = if flip < 0.0 { 0.0 } else { hf - 1.0 };
        let irx = wf * (0.11 + r() * 0.06);
        let iry = hf * if kind == RoomKind::Alcove { 0.4 } else { 0.29 };
        for y in 0..h {
            for x in 0..w {
                let open = lobes.iter().any(|&(lx, ly, rx, ry)| {
                    ((x as f64 - lx) / rx).powi(2) + ((y as f64 - ly) / ry).powi(2) <= 1.0
                });
                let intrusion = kind != RoomKind::Lobed
                    && ((x as f64 - ix) / irx).powi(2) + ((y as f64 - iy) / iry).powi(2) < 1.0;
                mask[y * w + x] = u8::from(open && !intrusion)
            }
        }
        for _ in 0..2 {
            let mut next = mask.clone();
            for y in 0..h as i32 {
                for x in 0..w as i32 {
                    let mut n = 0;
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            if (dx != 0 || dy != 0)
                                && x + dx >= 0
                                && x + dx < w as i32
                                && y + dy >= 0
                                && y + dy < h as i32
                            {
                                n += mask[((y + dy) * w as i32 + x + dx) as usize]
                            }
                        }
                    }
                    let at = y as usize * w + x as usize;
                    next[at] = if n >= 5 {
                        1
                    } else if n <= 2 {
                        0
                    } else {
                        mask[at]
                    }
                }
            }
            mask = next;
        }
        for y in center.y - 1..=center.y + 1 {
            for x in center.x - 1..=center.x + 1 {
                mask[y as usize * w + x as usize] = 1
            }
        }
        loop {
            let mut changed = false;
            for y in 0..h - 1 {
                for x in 0..w - 1 {
                    let a = y * w + x;
                    let b = a + 1;
                    let c = a + w;
                    let d = c + 1;
                    if mask[a] != 0 && mask[d] != 0 && mask[b] == 0 && mask[c] == 0 {
                        mask[b] = 1;
                        changed = true
                    } else if mask[b] != 0 && mask[c] != 0 && mask[a] == 0 && mask[d] == 0 {
                        mask[a] = 1;
                        changed = true
                    }
                }
            }
            if !changed {
                break;
            }
        }
    }
    RoomShape {
        width: w,
        height: h,
        cells: connected(&mask, w, center.y as usize * w + center.x as usize),
        center,
        size_class,
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PassageStyle {
    Constructed,
    Natural,
    Broad,
    Opening,
}
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct PassageShape {
    pub cells: Vec<usize>,
    pub path: Vec<usize>,
    pub style: PassageStyle,
}
/// Exact valid-coordinate slice. Invalid JS NaN footprints are rejected, never serialized as a valid passage.
/// O(P²) ordered uniqueness here (bounded passages), O(P) memory.
#[allow(clippy::too_many_arguments)]
pub fn make_room_passage(
    width: usize,
    from: Point,
    direction: Point,
    gap: i32,
    offset: i32,
    style: PassageStyle,
    constructed_start: bool,
    constructed_end: bool,
    bend: f64,
) -> Result<PassageShape, String> {
    if gap < 1
        || !matches!(
            (direction.x, direction.y),
            (1, 0) | (-1, 0) | (0, 1) | (0, -1)
        )
    {
        return Err("Unsupported passage parameters".into());
    }
    let side = Point {
        x: -direction.y,
        y: direction.x,
    };
    let mut path = Vec::new();
    let mut cells = Vec::new();
    let mut last_cross = 0;
    let at = |along: i32, cross: i32| -> Result<usize, String> {
        let x = from.x + direction.x * along + side.x * cross;
        let y = from.y + direction.y * along + side.y * cross;
        if x <= 0 || x >= width as i32 - 1 || y <= 0 {
            Err("Passage footprint outside supported interior".into())
        } else {
            Ok(y as usize * width + x as usize)
        }
    };
    for along in 1..=gap {
        let t = if gap <= 1 {
            0.0
        } else {
            (along - 1) as f64 / (gap - 1) as f64
        };
        let cross = if constructed_start && along <= 2 {
            0
        } else if constructed_end && along >= gap - 1 {
            offset
        } else {
            js_round(offset as f64 * t + bend * (std::f64::consts::PI * t).sin())
        };
        if along > 1 {
            path.push(at(along, last_cross)?);
            while last_cross != cross {
                last_cross += (cross - last_cross).signum();
                path.push(at(along, last_cross)?);
            }
        } else {
            last_cross = cross;
            path.push(at(along, cross)?)
        }
        let mut radius = match style {
            PassageStyle::Broad => 1,
            PassageStyle::Opening => 2,
            _ => 0,
        };
        if style == PassageStyle::Natural && gap >= 5 && along > 2 && along < gap - 1 {
            radius = 1
        }
        if style == PassageStyle::Opening && along > 2 && along < gap - 1 {
            radius = 1
        }
        if (constructed_start && along <= 2) || (constructed_end && along >= gap - 1) {
            radius = 0
        }
        for spread in -radius..=radius {
            let cell = at(along, cross + spread)?;
            if !cells.contains(&cell) {
                cells.push(cell)
            }
        }
    }
    for &cell in &path {
        if !cells.contains(&cell) {
            cells.push(cell)
        }
    }
    let first = path[0];
    let last = *path.last().unwrap();
    cells.retain(|&c| c != first && c != last);
    cells.insert(0, first);
    if last != first {
        cells.push(last)
    }
    Ok(PassageShape { cells, path, style })
}

fn js_round(n: f64) -> i32 {
    let floor = n.floor();
    (if n - floor < 0.5 { floor } else { floor + 1.0 }) as i32
}
