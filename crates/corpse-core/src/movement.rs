//! Exact grid algorithms ported from v48 pathfinding.ts. No world mutation.
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}
pub const DIRECTIONS: [Point; 8] = [
    Point { x: 0, y: -1 },
    Point { x: 1, y: 0 },
    Point { x: 0, y: 1 },
    Point { x: -1, y: 0 },
    Point { x: 1, y: -1 },
    Point { x: 1, y: 1 },
    Point { x: -1, y: 1 },
    Point { x: -1, y: -1 },
];
pub fn diagonal_step_allowed(
    tiles: &[String],
    width: usize,
    from: Point,
    to: Point,
    phase: bool,
) -> bool {
    phase
        || from.x == to.x
        || from.y == to.y
        || (tiles[(from.y as usize) * width + to.x as usize] != "wall"
            && tiles[(to.y as usize) * width + from.x as usize] != "wall")
}
#[derive(Debug, Clone)]
pub struct DistanceField {
    pub steps: Vec<i16>,
    pub diagonals: Vec<u16>,
    pub hazards: Option<Vec<u16>>,
    pub harmful: Option<Vec<bool>>,
    pub width: usize,
    pub phase: bool,
    pub portal_mates: Option<Vec<usize>>,
    pub spatial_exits: Option<Vec<Vec<usize>>>,
    pub ordered_mates: Option<Vec<usize>>,
}
/// O(V+E) without hazards, O((V+E) log V) with hazards, O(V+E) heap/scratch.
/// Supported maps have <=3600 cells, matching reference scalar cost bounds.
pub fn distance_field(
    tiles: &[String],
    width: usize,
    target: Point,
    phase: bool,
    flight: bool,
    allowed: Option<&[bool]>,
    harmful: Option<&[bool]>,
) -> DistanceField {
    distance_field_portals(tiles,width,target,phase,flight,allowed,harmful,&[])
}
pub fn distance_field_portals(
    tiles:&[String],width:usize,target:Point,phase:bool,flight:bool,
    allowed:Option<&[bool]>,harmful:Option<&[bool]>,links:&[(usize,usize)],
)->DistanceField {
    distance_field_ordered(tiles,width,target,phase,flight,allowed,harmful,links,&[])
}
/// Existing spatial path costs plus the speed-up-only local within-pair step.
/// Predecessors depend on both origin and entry; no physical/projectile changes.
pub fn distance_field_ordered(
    tiles:&[String],width:usize,target:Point,phase:bool,flight:bool,
    allowed:Option<&[bool]>,harmful:Option<&[bool]>,links:&[(usize,usize)],ordered:&[(usize,usize)],
)->DistanceField {
    let _timing_function = crate::timing::span("path.field_build");
    let n = tiles.len();
    let mut ordered_mates:Vec<_>=(0..n).collect();
    for &(a,b) in ordered {if a<n&&b<n{ordered_mates[a]=b;ordered_mates[b]=a;}}
    let mut mates: Vec<usize>=(0..n).collect();
    let usable=|at:usize|at<n && allowed.is_none_or(|v|v[at]) && (phase || (tiles[at]!="wall"&&tiles[at]!="tree"&&(tiles[at]!="chasm"||flight)));
    let mut exits=vec![Vec::new();n];
    for &(a,b) in links {if a<n&&b<n {exits[a].push(b);exits[b].push(a);}}
    let multi:Vec<_>=exits.iter().map(|v|v.len()>1).collect();
    // A stochastic step is admitted only if every outcome is traversable. Do not
    // silently filter one exit, which would change the authored fair lottery.
    for at in 0..n {if !usable(at)||exits[at].iter().any(|&other|!usable(other)){exits[at].clear();}else{exits[at].sort_unstable();exits[at].dedup();}}
    let stochastic=multi.iter().any(|v|*v);
    for at in 0..n {if let Some(&other)=exits[at].first(){mates[at]=other;}else if !multi[at]{exits[at].push(at);}}
    let mut incoming=vec![Vec::new();n];for (entry,arrivals) in exits.iter().enumerate(){for &arrival in arrivals{incoming[arrival].push(entry);}}
    let mut out = DistanceField {
        steps: vec![-1; n],
        diagonals: vec![u16::MAX; n],
        hazards: None,
        harmful: None,
        width,
        phase,
        portal_mates: (!links.is_empty()).then(||mates.clone()),
        spatial_exits: stochastic.then(||exits.clone()),
        ordered_mates: (!ordered.is_empty()).then(||ordered_mates.clone()),
    };
    if width == 0
        || n % width != 0
        || target.x < 0
        || target.y < 0
        || target.x as usize >= width
        || target.y as usize >= n / width
    {
        return out;
    }
    assert!(n <= 3600, "v48 pathfinding supports at most 3600 cells");
    let start = target.y as usize * width + target.x as usize;
    let adjacent = |i: usize| -> Vec<(usize, u16)> {
        let mut out:Vec<_> = incoming[i].iter().flat_map(|&entry| {
        let ordered_mates=&ordered_mates;
        let from = Point { x: (entry % width) as i32, y: (entry / width) as i32 };
        DIRECTIONS.iter().filter_map(move |d| {
            let to=Point{x:from.x+d.x,y:from.y+d.y};
            if to.x<0||to.y<0||to.x as usize>=width||to.y as usize>=n/width{return None}
            let ni=to.y as usize*width+to.x as usize;
            if !ordered.is_empty() && ordered_mates[entry]==ni && entry!=i {return None;}
            if allowed.is_some_and(|a|!a[ni]) || (!phase&&(tiles[ni]=="wall"||tiles[ni]=="tree"||(tiles[ni]=="chasm"&&!flight))) || !diagonal_step_allowed(tiles,width,from,to,phase){return None}
            Some((ni,u16::from(d.x!=0&&d.y!=0)))
        })}).collect();
        // The paired origin has a one-cell edge to i rather than a self-loop.
        let previous=ordered_mates[i];
        if previous!=i && usable(i) && usable(previous) {
            let from=Point{x:(previous%width)as i32,y:(previous/width)as i32};
            let to=Point{x:(i%width)as i32,y:(i/width)as i32};
            if (from.x-to.x).abs().max((from.y-to.y).abs())==1 && diagonal_step_allowed(tiles,width,from,to,phase) {
                out.push((previous,u16::from(from.x!=to.x&&from.y!=to.y)));
            }
        }
        out
    };
    out.steps[start] = 0;
    out.diagonals[start] = 0;
    if !harmful.is_some_and(|h| h.iter().any(|x| *x)) {
        let mut queue = VecDeque::from([start]);
        while let Some(i) = queue.pop_front() {
            for (ni, diagonal) in adjacent(i) {
                let length = out.steps[i] + 1;
                let turns = out.diagonals[i] + diagonal;
                if out.steps[ni] >= 0 {
                    if out.steps[ni] == length && turns < out.diagonals[ni] {
                        out.diagonals[ni] = turns;
                    }
                    continue;
                }
                out.steps[ni] = length;
                out.diagonals[ni] = turns;
                queue.push_back(ni);
            }
        }
    } else {
        let h = harmful.unwrap();
        let mut hazards = vec![u16::MAX; n];
        let mut costs = vec![u64::MAX; n];
        let scale = n as u64 + 1;
        let mut heap = BinaryHeap::new();
        hazards[start] = 0;
        costs[start] = 0;
        heap.push(Reverse((0u64, start)));
        while let Some(Reverse((cost, i))) = heap.pop() {
            if cost != costs[i] {
                continue;
            }
            for (ni, diagonal) in adjacent(i) {
                let proposed = cost + u64::from(h[i]) * scale * scale + scale + diagonal as u64;
                if proposed >= costs[ni] {
                    continue;
                }
                costs[ni] = proposed;
                hazards[ni] = hazards[i] + u16::from(h[i]);
                out.steps[ni] = out.steps[i] + 1;
                out.diagonals[ni] = out.diagonals[i] + diagonal;
                heap.push(Reverse((proposed, ni)));
            }
        }
        out.hazards = Some(hazards);
        out.harmful = Some(h.to_vec());
    }
    out
}
pub fn next_step(field: &DistanceField, tiles: &[String], from: Point) -> Option<Point> {
    let w = field.width;
    if w == 0
        || from.x < 0
        || from.y < 0
        || from.x as usize >= w
        || from.y as usize >= field.steps.len() / w
    {
        return None;
    }
    let at = from.y as usize * w + from.x as usize;
    if let Some(exits)=field.spatial_exits.as_ref() {
        if field.steps[at]==0{return None}
        let mut best=None;
        for d in DIRECTIONS {
            let to=Point{x:from.x+d.x,y:from.y+d.y};
            if to.x<0||to.y<0||to.x as usize>=w||to.y as usize>=field.steps.len()/w||!diagonal_step_allowed(tiles,w,from,to,field.phase){continue}
            let local=to.y as usize*w+to.x as usize;
            let direct=[local];
            let arrivals=if field.ordered_mates.as_ref().is_some_and(|m|m[local]==at){direct.as_slice()}else{exits[local].as_slice()};
            if arrivals.is_empty()||arrivals.iter().any(|&out|field.steps[out]<0){continue}
            let near=arrivals.iter().map(|&out|field.steps[out]).min().unwrap();
            if field.steps[at]>=0&&near>=field.steps[at]{continue}
            let far=arrivals.iter().map(|&out|field.steps[out]).max().unwrap();
            let harm=arrivals.iter().map(|&out|field.hazards.as_ref().map_or(0,|h|h[out]as u32)+field.harmful.as_ref().map_or(0,|h|u32::from(h[out]))).max().unwrap();
            let diagonal=arrivals.iter().map(|&out|field.diagonals[out]as u32).max().unwrap()+u32::from(d.x!=0&&d.y!=0);
            let score=(harm,near,far,diagonal);
            if best.as_ref().is_none_or(|&(old,_)|score<old){best=Some((score,d));}
        }
        return best.map(|(_,d)|d)
    }
    let mut best = field.steps[at];
    let mut harm = if best < 0 {
        u32::MAX
    } else {
        field.hazards.as_ref().map_or(0, |h| h[at] as u32)
    };
    let mut fewest = u32::MAX;
    let mut step = None;
    for d in DIRECTIONS {
        let to = Point {
            x: from.x + d.x,
            y: from.y + d.y,
        };
        if to.x < 0
            || to.y < 0
            || to.x as usize >= w
            || to.y as usize >= field.steps.len() / w
            || !diagonal_step_allowed(tiles, w, from, to, field.phase)
        {
            continue;
        }
        let local = to.y as usize * w + to.x as usize;
        let ni=if field.ordered_mates.as_ref().is_some_and(|m|m[local]==at){local}else{field.portal_mates.as_ref().map_or(local,|m|m[local])};
        let value = field.steps[ni];
        let h = field.hazards.as_ref().map_or(0, |h| h[ni] as u32)
            + field.harmful.as_ref().map_or(0, |h| u32::from(h[ni]));
        let diagonal = field.diagonals[ni] as u32 + u32::from(d.x != 0 && d.y != 0);
        if value >= 0
            && (h < harm
                || (h == harm
                    && (best < 0
                        || value < best
                        || (step.is_some() && value == best && diagonal < fewest))))
        {
            best = value;
            harm = h;
            fewest = diagonal;
            step = Some(d);
        }
    }
    step
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corner_wall_but_not_tree() {
        let mut t = vec!["floor".into(); 9];
        t[1] = "wall".into();
        assert!(!diagonal_step_allowed(
            &t,
            3,
            Point { x: 0, y: 0 },
            Point { x: 1, y: 1 },
            false
        ));
        t[1] = "tree".into();
        assert!(diagonal_step_allowed(
            &t,
            3,
            Point { x: 0, y: 0 },
            Point { x: 1, y: 1 },
            false
        ));
    }
    #[test]
    fn safe_route_beats_short_hazard() {
        let t = vec!["floor".into(); 25];
        let mut h = vec![false; 25];
        h[12] = true;
        let f = distance_field(&t, 5, Point { x: 4, y: 2 }, false, false, None, Some(&h));
        assert_ne!(
            next_step(&f, &t, Point { x: 1, y: 2 }),
            Some(Point { x: 1, y: 0 })
        );
    }
}
