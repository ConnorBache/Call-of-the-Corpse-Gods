//! R52 deep-water eddies. Square overlays use a private seed/floor stream;
//! existing saved arrows and the gameplay/population RNG are never migrated.
use crate::{Floor, rng::{GenerationRng, mix}};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Flow {
    // Cardinal N/E/S/W. The persisted layer retains the existing 1/3/5/7 codes.
    direction: u8,
    clockwise: bool,
    edge_distance: usize,
}

#[derive(Clone, Copy, Debug)]
struct Square { x: usize, y: usize, side: usize, clockwise: bool }

impl Square {
    fn flow(self, x: usize, y: usize, rng: &mut GenerationRng) -> Flow {
        let distances = [y - self.y, self.x + self.side - 1 - x,
            self.y + self.side - 1 - y, x - self.x]; // top/right/bottom/left
        let distance = *distances.iter().min().unwrap();
        let mut edges = [0u8; 4];
        let mut count = 0;
        for (edge, &d) in distances.iter().enumerate() {
            if d == distance { edges[count] = edge as u8; count += 1; }
        }
        let edge = edges[if count == 1 { 0 } else { rng.integer(0, count as i32 - 1) as usize }];
        Flow { direction: (edge + if self.clockwise { 1 } else { 3 }) % 4,
            clockwise: self.clockwise, edge_distance: distance }
    }
}

fn stronger(a: Flow, b: Flow, rng: &mut GenerationRng) -> Flow {
    if a.edge_distance < b.edge_distance { a }
    else if b.edge_distance < a.edge_distance { b }
    else if rng.random() < 0.5 { a } else { b }
}

/// Pair resolution is symmetric. Multiway overlaps fold in generation order;
/// surviving direction retains the stronger source's rotation/edge distance.
/// Cancellation is empty, so a later square can supply a new current.
fn collide(a: Option<Flow>, b: Flow, rng: &mut GenerationRng) -> Option<Flow> {
    let Some(a) = a else { return Some(b); };
    if (a.direction + 2) % 4 == b.direction {
        if a.clockwise == b.clockwise { return None; }
        // The designer's explicit collision sequence: clockwise up -> left.
        let direction = (a.direction + if a.clockwise { 3 } else { 1 }) % 4;
        Some(Flow { direction, ..stronger(a, b, rng) })
    } else {
        // Equal directions remain equal; metadata still has to survive for
        // the next square. Perpendicular directions use this same strength rule.
        Some(stronger(a, b, rng))
    }
}

fn neighbors(at: usize, w: usize, n: usize) -> [Option<usize>; 4] {
    [at.checked_sub(w), (at % w + 1 < w).then_some(at + 1),
        (at + w < n).then_some(at + w), (at % w > 0).then(|| at - 1)]
}

/// O(V + G + D) time/memory, D=total door-key bytes. Deep water covered with
/// ichor or blocked by a grate/closed door is not an active current surface.
fn water_mask(f: &Floor) -> Vec<bool> {
    let n = f.tiles.len();
    let mut blocked = vec![false; n];
    for &at in f.grates.as_ref().into_iter().flatten() {
        if let Some(v) = blocked.get_mut(at as usize) { *v = true; }
    }
    for (key, door) in f.doors.as_ref().into_iter().flatten() {
        if !door.open {
            if let Ok(at) = key.parse::<usize>() {
                if let Some(v) = blocked.get_mut(at) { *v = true; }
            }
        }
    }
    (0..n).map(|at| f.tiles[at] == "water" && !blocked[at]
        && f.ichor.as_ref().and_then(|v|v.get(at)).copied().unwrap_or(0.) == 0.).collect()
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct Coverage {
    pub lakes: usize,
    pub squares: usize,
    pub covered_tiles: usize,
    pub square_cell_visits: usize,
}

/// O(V + G + D + sum(Q*S²)) time, O(V + max L) extra memory. Each lake has
/// L tiles, Q<=L+1 squares, side S<=min(width,height)<=60. The indexed uncovered
/// set guarantees progress without rejection retries; each overlay affects
/// only its own lake. No hash iteration, pathfinding, rendering or gameplay RNG.
pub fn generate_for_attempt(f: &mut Floor, seed: u32, attempt: u32) -> Coverage {
    let n = f.tiles.len();
    let w = f.width();
    let h = f.height();
    let mut coverage = Coverage::default();
    if w < 2 || h < 2 { f.currents = vec![0.; n].into(); return coverage; }
    let generation_seed = if attempt == 0 { seed } else { mix(seed, 730000u32.wrapping_add(attempt)) };
    let mut rng = GenerationRng::new(mix(generation_seed, (f.depth as u32).wrapping_add(96000)));
    let water = water_mask(f);
    let mut lake = vec![usize::MAX; n];
    let mut slot = vec![usize::MAX; n];
    let mut flow = vec![None; n];
    for start in 0..n {
        if !water[start] || lake[start] != usize::MAX { continue; }
        coverage.lakes += 1;
        let mut cells = vec![start];
        lake[start] = start;
        let (mut min_x, mut max_x, mut min_y, mut max_y) = (w, 0, h, 0);
        let mut head = 0;
        while head < cells.len() {
            let at = cells[head]; head += 1;
            min_x = min_x.min(at % w); max_x = max_x.max(at % w);
            min_y = min_y.min(at / w); max_y = max_y.max(at / w);
            for to in neighbors(at, w, n).into_iter().flatten() {
                if water[to] && lake[to] == usize::MAX {
                    lake[to] = start; cells.push(to);
                }
            }
        }
        let max_side = w.min(h).min(2.max(max_x - min_x + 1).max(max_y - min_y + 1));
        let mut uncovered = cells.clone();
        for (i, &at) in uncovered.iter().enumerate() { slot[at] = i; }
        let mut count = 0;
        while !uncovered.is_empty() || (count == 1 && cells.len() > 1) {
            let anchors = if uncovered.is_empty() { &cells } else { &uncovered };
            let at = anchors[rng.integer(0, anchors.len() as i32 - 1) as usize];
            let side = rng.integer(2, max_side as i32) as usize;
            let x = rng.integer((at % w).saturating_sub(side - 1) as i32,
                (at % w).min(w - side) as i32) as usize;
            let y = rng.integer((at / w).saturating_sub(side - 1) as i32,
                (at / w).min(h - side) as i32) as usize;
            let square = Square { x, y, side, clockwise: rng.random() < 0.5 };
            coverage.squares += 1; count += 1;
            for yy in y..y + side { for xx in x..x + side {
                coverage.square_cell_visits += 1;
                let at = yy * w + xx;
                if lake[at] != start { continue; }
                if slot[at] != usize::MAX {
                    let index = slot[at];
                    uncovered.swap_remove(index);
                    if let Some(&moved) = uncovered.get(index) { slot[moved] = index; }
                    slot[at] = usize::MAX;
                    coverage.covered_tiles += 1;
                }
                flow[at] = collide(flow[at], square.flow(xx, yy, &mut rng), &mut rng);
            }}
        }
    }
    f.currents = flow.into_iter().map(|v|v.map_or(0., |v| (1 + 2 * v.direction) as f64)).collect::<Vec<_>>().into();
    coverage
}

/// Reapply after the population decorators, using the accepted geometry attempt
/// already recorded on the floor. This changes arrows only, never terrain.
pub fn generate(f: &mut Floor, seed: u32) -> Coverage {
    let attempt = f.generation.as_ref().and_then(|g|g.stages.iter().find(|s|s.id == "cleanup"))
        .and_then(|s|s.details.as_ref()).and_then(|d|d.get("generationAttempt"))
        .copied().unwrap_or(0.) as u32;
    generate_for_attempt(f, seed, attempt)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn f(direction: u8, clockwise: bool, edge_distance: usize) -> Flow { Flow { direction, clockwise, edge_distance } }
    #[test]
    fn nearest_manhattan_edge_and_both_rotations_match_all_four_rules() {
        let mut rng = GenerationRng::new(1);
        for clockwise in [true, false] {
            let s = Square {x:2,y:2,side:7,clockwise};
            for (x,y,cw) in [(5,2,1),(8,5,2),(5,8,3),(2,5,0)] {
                assert_eq!(s.flow(x,y,&mut rng),f(if clockwise {cw} else {(cw+2)%4},clockwise,0));
            }
            let a=s.flow(5,3,&mut rng);assert_eq!(a.edge_distance,1);
        }
    }
    #[test]
    fn corner_and_center_ties_are_seeded_random_among_only_nearest_edges() {
        let mut corner=std::collections::BTreeSet::new();let mut center=std::collections::BTreeSet::new();
        let s=Square{x:1,y:1,side:5,clockwise:true};
        for seed in 0..128 {let mut rng=GenerationRng::new(seed);
            corner.insert(s.flow(1,1,&mut rng).direction);center.insert(s.flow(3,3,&mut rng).direction);}
        assert_eq!(corner,[0,1].into());assert_eq!(center,[0,1,2,3].into());
    }
    #[test]
    fn every_pair_collision_branch_and_explicit_clockwise_up_example() {
        for direction in 0..4 {for clockwise in [false,true] {
            let a=f(direction,clockwise,2);let mut rng=GenerationRng::new(4);
            assert_eq!(collide(Some(a),f(direction, !clockwise,1),&mut rng).unwrap().direction,direction);
            assert_eq!(collide(Some(a),f((direction+1)%4,!clockwise,0),&mut rng).unwrap().direction,(direction+1)%4);
            assert_eq!(collide(Some(a),f((direction+1)%4,!clockwise,3),&mut rng),Some(a));
            assert_eq!(collide(Some(a),f((direction+2)%4,clockwise,0),&mut rng),None);
            let b=f((direction+2)%4,!clockwise,0);
            let out=collide(Some(a),b,&mut rng).unwrap();
            assert_eq!(out.direction,(direction+if clockwise{3}else{1})%4);
            assert_eq!(out.clockwise,b.clockwise);assert_eq!(out.edge_distance,0);
            assert_eq!(collide(Some(b),a,&mut rng),Some(out));
        }}
        let mut rng=GenerationRng::new(2);
        assert_eq!(collide(Some(f(0,true,0)),f(2,false,1),&mut rng).unwrap().direction,3);
    }
    #[test]
    fn strength_ties_use_seed_and_multioverlap_keeps_provenance_or_restarts_after_cancel() {
        let a=f(0,true,0);let b=f(1,false,0);let mut chosen=std::collections::BTreeSet::new();
        for seed in 0..128 {let mut rng=GenerationRng::new(seed);chosen.insert(collide(Some(a),b,&mut rng).unwrap().direction);}
        assert_eq!(chosen,[0,1].into());
        let mut rng=GenerationRng::new(1);
        let c=collide(Some(f(0,true,1)),f(2,false,0),&mut rng).unwrap();
        assert_eq!(c,f(3,false,0));
        assert_eq!(collide(Some(c),f(1,false,3),&mut rng),None);
        assert_eq!(collide(None,a,&mut rng),Some(a));
    }
    fn basin() -> Floor {
        let mut f=Floor::default();f.depth=7.;f.tiles=vec!["floor".into();225];
        for y in 1..14 {for x in 1..14 {f.tiles[y*15+x]="water".into();}}
        f
    }
    #[test]
    fn seeded_cover_is_total_bounded_and_does_not_change_any_noncurrent_field() {
        for seed in [0,1,7,27,99,u32::MAX] {
            let original=basin();let mut a=original.clone();let mut b=original.clone();
            let report=generate_for_attempt(&mut a,seed,3);
            assert_eq!(report,generate_for_attempt(&mut b,seed,3));assert_eq!(a,b);
            assert_eq!(report.covered_tiles,169);assert_eq!(report.lakes,1);
            assert!(report.squares>=2&&report.squares<=170);
            assert!(report.square_cell_visits<=report.squares*13*13);
            for (i,&v) in a.currents.as_ref().unwrap().iter().enumerate(){
                assert!([0.,1.,3.,5.,7.].contains(&v));if original.tiles[i]!="water"{assert_eq!(v,0.);}
            }
            a.currents=original.currents.clone();assert_eq!(a,original);
        }
    }
    #[test]
    fn skinny_tiny_separate_and_blocked_lakes_remain_covered_without_land_arrows() {
        let mut f=Floor::default();f.tiles=vec!["floor".into();81];
        for i in [0,1,2,3,4,5,6,7,8,20,40,41,49,50,80] {f.tiles[i]="water".into();}
        f.tiles[20]="shallow-water".into();f.ichor=vec![0.;81].into();f.ichor.as_mut().unwrap()[1]=1.;
        f.grates=vec![41.].into();
        let mask=water_mask(&f);let stats=generate(&mut f,4);
        assert_eq!(stats.covered_tiles,mask.iter().filter(|v|**v).count());
        assert!(stats.squares<=stats.covered_tiles+stats.lakes);
        for (i,&wet) in mask.iter().enumerate(){if !wet{assert_eq!(f.currents.as_ref().unwrap()[i],0.);}}
    }
    #[test]
    fn two_by_two_minimum_and_invalid_tiny_geometry_terminate() {
        let mut f=Floor::default();f.tiles=vec!["water".into();4];
        let stats=generate(&mut f,0);assert_eq!(stats.covered_tiles,4);assert_eq!(stats.squares,2);
        assert_eq!(stats.square_cell_visits,8);
        f.tiles=vec!["water".into()];assert_eq!(generate(&mut f,0),Coverage::default());
    }
}
