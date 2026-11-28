//! Physical corpse space, independent of living actors.
use crate::{
    Actor, Run,
    item_placement::{PlacementResult, cell_key, nearest_free_ground},
};
use std::collections::HashSet;
fn metric(r: &mut Run, k: &str, n: f64) {
    *r.metrics.entry(k.into()).or_default() += n
}
pub(crate) fn sever(r: &mut Run, id: f64) {
    let direct_roots = r.simulation_revision.value_or(29.) >= 51.;
    for f in &mut r.floors {
        if f.fungal_roots
            .as_ref()
            .is_some_and(|roots| roots.iter().any(|root| root.corpse_id == id))
        {
            let f = crate::mutation_journal::make_mut(f);
            for root in f.fungal_roots.as_mut().unwrap() {
                if root.corpse_id == id {
                    root.active = false
                }
            }
            if direct_roots { crate::fungi::clear_orphaned_root_cells(f); }
        }
    }
}
/// O(sum(V+A)) over <=40 floors, plus native generation requested by callback.
pub fn place_corpse(
    r: &mut Run,
    mut depth: f64,
    mut corpse: Actor,
    ensure: impl Fn(&mut Run, f64) -> Result<(), String>,
) -> Result<PlacementResult, String> {
    if !corpse.dead.value_or(false)
        || !depth.is_finite()
        || depth.fract() != 0.0
        || depth < 1.0
        || !corpse.x.is_finite()
        || !corpse.y.is_finite()
    {
        return Err("Invalid corpse placement coordinates/depth/state".into());
    }
    let from = depth;
    let origin = (corpse.x, corpse.y);
    let (mut x, mut y) = origin;
    metric(r, "corpsePlacementCalls", 1.0);
    while depth <= 40.0 {
        ensure(r, depth)?;
        let fi = r
            .floors
            .iter()
            .position(|f| f.depth == depth)
            .ok_or("Corpse placement floor missing")?;
        let f = &r.floors[fi];
        let w = f.width();
        let shared=crate::remains_rules::current(r);
        let dream=crate::dreamwalking::dream_actor(r,&corpse);
        let mut occupied:HashSet<_>=r.actors_at(depth).iter()
            .filter(|a| a.dead.value_or(false) && a.id!=corpse.id
                && (!shared || crate::dreamwalking::dream_actor(r,a)==dream))
            .map(|a| cell_key(f,a.x,a.y)).collect();
        if shared {
            occupied.extend(r.ground.get(&depth.to_string()).into_iter().flat_map(|g|g.iter())
                .filter(|g|crate::dreamwalking::dream_item(r,&g.item)==dream)
                .map(|g|cell_key(f,g.x,g.y)));
        }
        let direct = x.fract() == 0.0
            && y.fract() == 0.0
            && x >= 0.0
            && y >= 0.0
            && x < w as f64
            && y < f.height() as f64
            && ["chasm", "pit"].contains(&f.tiles[y as usize * w + x as usize].as_str());
        if direct && crate::finale::collapsed(r, depth, y as usize*w+x as usize) { break; }
        let destination = if direct {
            Some((true, x, y))
        } else {
            nearest_free_ground(f, &occupied, x, y)
        };
        let cells = f.tiles.len();
        if !direct {
            metric(r, "corpsePlacementCells", cells as f64);
            let count=if shared {occupied.len()} else {
                r.actors_at(depth).iter().filter(|a|a.dead.value_or(false)&&a.id!=corpse.id).count()
            };
            metric(r, "corpsePlacementOccupancyEntries", count as f64);
        }
        crate::mutation_journal::make_mut(&mut r.floors[fi]).dirty = true;
        let Some((fall, nx, ny)) = destination else {
            break;
        };
        if !fall {
            corpse.x = nx;
            corpse.y = ny;
            if depth != from {
                r.actors_at_mut(from).retain(|a| a.id != corpse.id);
            }
            if let Some(existing) = r
                .actors_at_mut(depth)
                .iter_mut()
                .find(|a| a.id == corpse.id)
            {
                *existing = corpse.clone();
            } else {
                r.actors_at_mut(depth).push(corpse.clone());
            }
            if depth != from || corpse.x != origin.0 || corpse.y != origin.1 {
                sever(r, corpse.id)
            }
            return Ok(PlacementResult::Placed {
                depth,
                x: nx,
                y: ny,
            });
        }
        if crate::finale::collapsed(r, depth, ny as usize*w+nx as usize) { break; }
        x = nx;
        y = ny;
        metric(r, "corpsePlacementFalls", 1.0);
        depth = crate::fast_mode::below(r, depth);
    }
    r.actors_at_mut(from).retain(|a| a.id != corpse.id);
    sever(r, corpse.id);
    metric(r, "corpsePlacementDiscarded", 1.0);
    Ok(PlacementResult::Discarded { depth })
}
pub fn normalize_corpses(
    r: &mut Run,
    ensure: impl Fn(&mut Run, f64) -> Result<(), String>,
) -> Result<(usize, usize), String> {
    let mut depths: Vec<_> = r.floors.iter().map(|f| (f.depth, f.width())).collect();
    depths.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut pending = Vec::new();
    for (depth, w) in depths {
        let mut occupied = HashSet::new();
        r.actors_at_mut(depth).retain(|a| {
            if a.dead.value_or(false) && !occupied.insert((a.y * w as f64 + a.x).to_bits()) {
                pending.push((depth, a.clone()));
                false
            } else {
                true
            }
        });
    }
    let (mut relocated, mut discarded) = (0, 0);
    for (d, a) in pending {
        match place_corpse(r, d, a, &ensure)? {
            PlacementResult::Placed { .. } => relocated += 1,
            PlacementResult::Discarded { .. } => discarded += 1,
        }
    }
    Ok((relocated, discarded))
}

/// Fixed-position offerings and death hearts take precedence over corpses, not
/// over living creatures. Same-plane corpses displaced by the item spread normally.
pub(crate) fn vacate_for_item(r:&mut Run,depth:f64,x:f64,y:f64,dream:bool,except:Option<f64>)->Result<(),String>{
    let mut bodies:Vec<_>=r.actors_at(depth).iter().filter(|a|a.dead.value_or(false)
        && a.x==x && a.y==y && Some(a.id)!=except
        && crate::dreamwalking::dream_actor(r,a)==dream).cloned().collect();
    bodies.sort_by(|a,b|a.id.total_cmp(&b.id));
    for corpse in bodies {
        place_corpse(r,depth,corpse,|r,d|crate::startup::ensure_floors(r,d as u32))?;
    }
    Ok(())
}

/// One-time legacy adoption (or pre-amendment snapshot restore). Items stay
/// fixed. Stable depth and actor iteration breaks ties in overlapping corpses.
pub(crate) fn separate_overlaps(r:&mut Run)->Result<(),String>{
    if !crate::remains_rules::current(r){return Ok(());}
    let mut floors:Vec<_>=r.floors.iter().collect();
    floors.sort_by(|a,b|a.depth.total_cmp(&b.depth));
    let mut pending=Vec::new();
    for f in floors {
        let mut taken:HashSet<_>=r.ground.get(&f.depth.to_string()).into_iter().flat_map(|g|g.iter())
            .map(|g|(crate::dreamwalking::dream_item(r,&g.item),cell_key(f,g.x,g.y))).collect();
        for a in r.actors_at(f.depth).iter().filter(|a|a.dead.value_or(false)) {
            if !taken.insert((crate::dreamwalking::dream_actor(r,a),cell_key(f,a.x,a.y))){
                pending.push((f.depth,a.clone()));
            }
        }
    }
    for (depth,corpse) in pending {
        place_corpse(r,depth,corpse,|r,d|crate::startup::ensure_floors(r,d as u32))?;
    }
    Ok(())
}
