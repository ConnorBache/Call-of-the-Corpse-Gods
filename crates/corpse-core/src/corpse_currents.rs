//! R52 loose corpses follow water currents, independently of living occupancy.
use crate::{Actor, Floor, Run};
use std::collections::HashSet;

fn cell(f: &Floor, a: &Actor) -> usize {
    a.y as usize * f.width() + a.x as usize
}

fn loose(r: &Run, f: &Floor, a: &Actor) -> bool {
    a.dead.value_or(false)
        && !crate::dreamwalking::dream_actor(r, a)
        && !crate::environment::corpse_rooted(f, r.actors_at(f.depth), a)
}

fn gap(f: &Floor, a: &Actor) -> bool {
    matches!(f.tiles.get(cell(f, a)).map(String::as_str), Some("chasm" | "pit"))
}

/// Settle gravity at command/normal-phase/environment boundaries, without a
/// clock advance or new physics RNG. O(A + G*K + G log G) collection plus established
/// placement/generation costs for G unsupported dead bodies, O(G*K) scratch.
/// Running before corpse upkeep prevents a mature saved corpse reviving in air.
pub(crate) fn settle_falls(r: &mut Run) -> Result<(), String> {
    if r.simulation_revision.value_or(29.) < 52. || crate::dreamwalking::context_dream(r) {
        return Ok(());
    }
    let Some(f) = r.floor() else { return Ok(()) };
    let depth = r.depth;
    let mut corpses: Vec<_> = r.actors_at(depth).iter()
        .filter(|a| a.dead.value_or(false) && !crate::dreamwalking::dream_actor(r, a) && gap(f, a))
        .cloned().collect();
    corpses.sort_by(|a, b| a.id.total_cmp(&b.id));
    for corpse in corpses {
        crate::corpse_placement::place_corpse(r, depth, corpse,
            |r, d| crate::startup::ensure_floors(r, d as u32))?;
    }
    Ok(())
}

/// Conservative activity test: occupied-but-legal destinations need exact
/// ticks because revival can free them. Illegal arrows do not. O(A*(B+R*(A+L))), O(1) extra memory;
/// B is current eligibility, R root links and L their path length.
pub(crate) fn has_activity(r: &Run, f: &Floor) -> bool {
    r.simulation_revision.value_or(29.) >= 52.
        && r.actors_at(f.depth).iter().any(|a| {
            a.dead.value_or(false) && !crate::dreamwalking::dream_actor(r, a)
                && (gap(f, a) || (crate::environment::current_destination_revision(f, cell(f, a), 52.).is_some()
                    && loose(r, f, a)))
        })
}

/// Once per normal environment tick, in stable ID order. O(A+C log C+C*(B+
/// R*(A+L)+d)) plus root sever/COW costs; O(C) snapshots, C dead actors.
/// Snapshot corpse occupancy forbids swaps/following and one reservation owns
/// each destination. Under the current rules, waking items also reserve their cells.
pub(crate) fn drift(r: &mut Run) {
    if r.simulation_revision.value_or(29.) < 52. || crate::dreamwalking::context_dream(r) {
        return;
    }
    let Some(f) = r.floor() else { return };
    if f.currents.as_ref().is_none() { return; }
    let w = f.width();
    let mut corpses: Vec<_> = r.actors_at(r.depth).iter()
        .filter(|a| a.dead.value_or(false) && !crate::dreamwalking::dream_actor(r, a))
        .map(|a| (a.id, cell(f, a))).collect();
    corpses.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut occupied: HashSet<_> = corpses.iter().map(|&(_, at)| at).collect();
    if crate::remains_rules::current(r) {
        occupied.extend(r.ground.get(&r.depth.to_string()).into_iter().flat_map(|g|g.iter())
            .filter(|g|!crate::dreamwalking::dream_item(r,&g.item))
            .map(|g|g.y as usize*w+g.x as usize));
    }
    let mut reserved = HashSet::new();
    for (id, from) in corpses {
        let f = r.floor().unwrap();
        let Some(mut to) = crate::environment::current_destination_revision(f, from, 52.) else { continue };
        let a = crate::engine::actor(r, id).unwrap();
        if !loose(r, f, a) { continue; }
        if occupied.contains(&to) || reserved.contains(&to) {
            if !crate::shore_currents::bank(f, to) { continue; }
            let Some(alternate) = crate::shore_currents::alternate_bank(f, from,
                |at| !occupied.contains(&at) && !reserved.contains(&at)) else { continue };
            to = alternate;
        }
        let a = crate::engine::actor_mut_trusted(r, id).unwrap();
        a.x = (to % w) as f64;
        a.y = (to / w) as f64;
        reserved.insert(to);
        // Incomplete root growth does not anchor the corpse. Preserve severed
        // pair history but clear its disconnected material at the old location.
        crate::corpse_placement::sever(r, id);
        r.floor_mut().unwrap().dirty = true;
    }
}

#[cfg(test)]
#[path = "revision52_corpse_currents_tests.rs"]
mod tests;
