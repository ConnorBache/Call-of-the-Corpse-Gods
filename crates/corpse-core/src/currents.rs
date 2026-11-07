//! v48 deterministic water drainage and normal-turn current displacement.
use crate::{Floor, Run};
use std::collections::HashSet;
pub const DIRECTIONS: [(isize, isize); 9] = [
    (0, 0),
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];
fn adjacent(f: &Floor, i: usize) -> Vec<usize> {
    let w = f.width();
    let mut out = vec![];
    if i >= w {
        out.push(i - w)
    }
    if i % w < w - 1 {
        out.push(i + 1)
    }
    if i < f.tiles.len() - w {
        out.push(i + w)
    }
    if i % w > 0 {
        out.push(i - 1)
    }
    out
}
/// O(V) time/memory; four edges per cell, no simulation RNG.
pub fn generate_currents(f: &mut Floor) {
    let n = f.tiles.len();
    let w = f.width();
    let water: Vec<_> = (0..n)
        .map(|i| {
            f.tiles[i] == "water"
                && f.ichor
                    .as_ref()
                    .and_then(|v| v.get(i))
                    .copied()
                    .unwrap_or(0.)
                    == 0.
                && !crate::environment::blocks_movement(f, i)
        })
        .collect();
    let mut currents = vec![0.; n];
    let mut seen = vec![false; n];
    for start in 0..n {
        if seen[start] || !water[start] {
            continue;
        }
        let mut component = vec![start];
        seen[start] = true;
        let mut q = 0;
        while q < component.len() {
            for j in adjacent(f, component[q]) {
                if !seen[j] && water[j] {
                    seen[j] = true;
                    component.push(j)
                }
            }
            q += 1;
        }
        if component.len() < 6 {
            continue;
        }
        let cx = component.iter().map(|&i| (i % w) as f64).sum::<f64>() / component.len() as f64;
        let cy = component.iter().map(|&i| (i / w) as f64).sum::<f64>() / component.len() as f64;
        let mut center = component[0];
        let mut best = f64::NEG_INFINITY;
        for i in component {
            let score = adjacent(f, i).iter().filter(|&&j| water[j]).count() as f64 * 10000.
                - ((i % w) as f64 - cx).powi(2)
                - ((i / w) as f64 - cy).powi(2);
            if score > best {
                best = score;
                center = i
            }
        }
        let mut pool = vec![center];
        pool.extend(adjacent(f, center).into_iter().filter(|&j| water[j]));
        if pool.len() < 3 {
            continue;
        }
        let mut drained: HashSet<_> = pool.iter().copied().collect();
        let mut queue = pool;
        let mut q = 0;
        while q < queue.len() {
            let downstream = queue[q];
            q += 1;
            for i in adjacent(f, downstream) {
                if drained.contains(&i) || !water[i] {
                    continue;
                }
                drained.insert(i);
                queue.push(i);
                let dx = (downstream % w) as isize - (i % w) as isize;
                let dy = (downstream / w) as isize - (i / w) as isize;
                currents[i] = DIRECTIONS.iter().position(|&d| d == (dx, dy)).unwrap() as f64;
            }
        }
    }
    f.currents = currents.into();
}
/// Snapshot occupancy prevents following into a vacated square and swaps.
/// O(A log A + A*G + movement hooks), O(A) snapshot storage.
pub fn tick_currents(r: &mut Run) -> Result<(), String> {
    let f = r.floor().ok_or("Missing floor")?;
    if f.currents.as_ref().is_none() {
        return Ok(());
    }
    let w = f.width();
    let mut positions: Vec<_> = std::iter::once(&r.player)
        .chain(r.actors_at(r.depth))
        .filter(|a| !a.dead.value_or(false) && crate::dreamwalking::contact_plane(r,a))
        .map(|a| (a.id, a.y as usize * w + a.x as usize))
        .collect();
    positions.sort_by(|a, b| a.0.total_cmp(&b.0));
    let occupied: HashSet<_> = positions.iter().map(|p| p.1).collect();
    let mut reserved = HashSet::new();
    for (id, from) in positions {
        let a = crate::engine::actor(r, id).unwrap();
        // Keep its cell in occupancy, but a hallucination never drifts.
        if crate::revision50_nightmares::is_decoy(a) { continue; }
        if crate::environment::is_flying_in(r, a) {
            continue;
        }
        let Some(mut to) = crate::environment::current_destination_revision(r.floor().unwrap(), from, r.simulation_revision.value_or(29.0)) else {
            continue;
        };
        if occupied.contains(&to) || reserved.contains(&to) {
            if r.simulation_revision.value_or(29.0)>=40.0 && crate::shore_currents::bank(r.floor().unwrap(),to) {
                let Some(alternate)=crate::shore_currents::alternate_bank(r.floor().unwrap(),from,|i|!occupied.contains(&i)&&!reserved.contains(&i)) else {continue;};
                to=alternate;
            } else {continue;}
        }
        let (dx, dy) = ((to % w) as f64 - a.x, (to / w) as f64 - a.y);
        let prior = a.conditions.get("forcedMove").copied();
        crate::engine::actor_mut_trusted(r, id)
            .unwrap()
            .conditions
            .insert("forcedMove".into(), 1.);
        let result = crate::engine::move_actor(r, id, dx, dy, false).map_err(|e| e.to_string());
        let a = crate::engine::actor(r, id).unwrap();
        if a.y as usize * w + a.x as usize == to {
            reserved.insert(to);
            r.floor_mut().unwrap().dirty = true;
        }
        let a = crate::engine::actor_mut_trusted(r, id).unwrap();
        if let Some(prior) = prior {
            a.conditions.insert("forcedMove".into(), prior);
        } else {
            a.conditions.shift_remove("forcedMove");
        }
        result?;
    }
    Ok(())
}
