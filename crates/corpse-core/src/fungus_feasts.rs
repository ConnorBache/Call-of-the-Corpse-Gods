//! Revision51 generated fungus feasts. Legacy populations keep their frozen props.
use crate::{Actor, Floor, movement::Point};
use std::collections::HashSet;

pub fn is_fungus(kind: &str) -> bool {
    matches!(kind, "fungus" | "bastidion" | "vent-fungus")
}
#[derive(Clone, Debug, PartialEq)]
pub struct Feast {
    pub source: usize,
    /// Current-depth victim first, out-of-depth victim second.
    pub cells: [usize; 2],
}

/// Use actual ordinary recipes, not bestiary display/reference depths. Dead
/// specimens need no living-horde partner or living habitat; follower depth
/// follows its leader. Aquatic corpses retain normal stranded behavior if revived.
/// O(C*M*(Q+K+T)*J) time, O(K*J) memory: C recipes across three depths,
/// M<=5 members, Q=2 pairs, K<=57 species, T<=57 window rows, J kind-ID bytes.
/// Includes linear species/window/dedup scans and string comparisons; a cold
/// candidate cache pays its existing recipe-construction costs separately.
/// No RNG or mutation. See docs/revision51-fungus-feasts.md for full bounds.
pub fn victim_pools(depth: u32, revision: f64) -> Result<[Vec<String>; 2], String> {
    let mut pools: [Vec<String>; 2] = Default::default();
    let catalog = crate::population::catalog();
    let species = crate::ordinary_roster::species_revision(revision);
    for offset in [0, 2, 3] {
        let target = depth + offset;
        if target > 40 { continue; }
        let candidates = crate::ordinary_roster::candidates_revision(target, true, revision)?;
        for kind in candidates.iter().flat_map(|candidate| &candidate.kinds) {
            let leader = catalog.pairs.iter().find(|(_, follower)| *follower == kind)
                .map_or(kind, |(leader, _)| leader);
            if offset != 0 && !species.iter().any(|s| &s.id == leader
                && crate::ordinary_roster::bounds(s, revision).0 == target) {
                continue;
            }
            let pool = &mut pools[usize::from(offset != 0)];
            if !pool.contains(kind) { pool.push(kind.clone()); }
        }
    }
    Ok(pools)
}

fn clear_path(f: &Floor, source: &Actor, corpse: &Actor) -> bool {
    crate::vision::floor_ray(f,
        Point { x: source.x as i32, y: source.y as i32 },
        Point { x: corpse.x as i32, y: corpse.y as i32 })
        && crate::fungi::root_path(f.width(), source, corpse).iter().all(|&at| {
            let at = at as usize;
            !f.permanent_fire.as_ref().is_some_and(|fire| fire.contains(&(at as f64)))
                && f.environment.as_ref().and_then(|e| e.fire.get(at)).copied().unwrap_or(0.) <= 0.
        })
}

/// For F setup fungi, O(F*B*(R*(1+P)+log B)) time plus victim-pool work:
/// B<=441 candidate cells, R<=11 LOS/root cells, P permanent-fire entries scanned
/// per root cell. Includes distance sorting and existing door-map lookup costs;
/// blocked/reserved membership uses expected hash bounds. O(B+F+K*J) scratch.
/// No RNG. Reserve both victims together; omit an impossible fungus entirely.
/// See docs/revision51-fungus-feasts.md for cache and transitive helper costs.
pub(crate) fn plan(
    f: &Floor, blocked: &HashSet<usize>, reserved: &mut HashSet<usize>, revision: f64,
) -> Result<Vec<Feast>, String> {
    let fungi: Vec<_> = f.setup_spawns.as_ref().into_iter().flatten()
        .filter(|intent| is_fungus(&intent.kind)).collect();
    if fungi.is_empty() { return Ok(vec![]); }
    let pools = victim_pools(f.depth as u32, revision)?;
    if pools.iter().any(Vec::is_empty) { return Ok(vec![]); }
    let w = f.width();
    let mut result = Vec::new();
    for intent in fungi {
        let source = Actor { x: intent.x, y: intent.y, ..Default::default() };
        let (x, y) = (intent.x as i32, intent.y as i32);
        let mut choices = Vec::new();
        for cy in (y - 10).max(0)..=(y + 10).min(f.height() as i32 - 1) {
            for cx in (x - 10).max(0)..=(x + 10).min(w as i32 - 1) {
                let distance = (cx - x).pow(2) + (cy - y).pow(2);
                let at = cy as usize * w + cx as usize;
                if distance == 0 || distance > 100 || f.tiles[at] != "floor"
                    || blocked.contains(&at) || reserved.contains(&at) { continue; }
                let corpse = Actor { x: cx as f64, y: cy as f64, ..Default::default() };
                if clear_path(f, &source, &corpse) { choices.push((distance, at)); }
            }
        }
        choices.sort_unstable();
        if choices.len() < 2 { continue; }
        let cells = [choices[0].1, choices[1].1];
        reserved.extend(cells);
        result.push(Feast { source: y as usize * w + x as usize, cells });
    }
    Ok(result)
}

/// Two selection draws per feast from an isolated seed/floor stream; existing
/// actor/loot streams are unchanged. R52 starts each root at its source and
/// records its normal-turn growth state; earlier revisions keep full paths.
/// In addition to pool
/// work, O(F*(A+L+W+R)+V) time: F fungi, A actors scanned per source lookup,
/// L cloned actor payload, W existing actor construction/trait work, R<=11 root
/// cells, V floor cells. O(K*J+F*(L+R)+V) scratch/output, with root-layer O(V)
/// allocation only when absent. Existing COW/transaction costs remain additive.
/// See docs/revision51-fungus-feasts.md for bounds and cache assumptions.
pub(crate) fn populate(
    seed: &str, next_id: &mut f64, f: &mut Floor, actors: &mut Vec<Actor>,
    feasts: &[Feast], revision: f64,
) -> Result<(), String> {
    if feasts.is_empty() { return Ok(()); }
    let pools = victim_pools(f.depth as u32, revision)?;
    if pools.iter().any(Vec::is_empty) { return Err("Fungus feast lost eligible victims".into()); }
    let mut rng = crate::population::population_random(
        &format!("{seed}:floor:{}:fungus-feasts", f.depth));
    let w = f.width();
    for feast in feasts {
        let source = actors.iter().find(|a| a.y as usize * w + a.x as usize == feast.source
            && a.abilities.iter().any(|id| id == "enemy:beast:fungal-gas"))
            .cloned().ok_or("Fungus feast lost source")?;
        for (slot, at) in feast.cells.iter().copied().enumerate() {
            let pool = &pools[slot];
            let kind = &pool[(rng.random() * pool.len() as f64).floor() as usize];
            let mut corpse = crate::population::create_actor_revision(kind, *next_id,
                (at % w) as f64, (at / w) as f64, false, revision)?;
            *next_id += 1.;
            crate::balance52::apply_new_actor_kind(&mut corpse, kind, f.depth, revision);
            corpse.dead = true.into();
            corpse.hp = 0.;
            corpse.heartless = true.into();
            corpse.corpse_age = 0.0.into();
            let path = crate::fungi::root_path(w, &source, &corpse);
            let root = crate::fungi::new_link(&source, &corpse, path, revision, crate::fungi::ROOT_COLOR);
            let roots = f.roots.get_or_insert_with(|| vec![0.; f.tiles.len()]);
            for &cell in root.grown_path() { roots[cell as usize] = 1.; }
            f.fungal_roots.get_or_insert_with(Vec::new).push(root);
            actors.push(corpse);
        }
    }
    Ok(())
}
