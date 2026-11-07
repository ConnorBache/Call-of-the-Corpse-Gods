//! Revision51 reserves stair tiles for the player and existing friendly actors.
//! Queries are pure: no placement repair, random draws or new serialized state.
use crate::{Actor, Floor, Run, engine, movement::{self, Point}};

pub fn tile(tile: &str) -> bool { matches!(tile, "up" | "down") }
pub fn restricted(r: &Run, a: &Actor) -> bool {
    r.simulation_revision.value_or(29.) >= 51. && a.id != r.player.id
        && (a.state != "ally" || engine::condition(a,"temporaryAlly") > 0.)
        && !crate::half_here::is_clone(a) && !crate::dreamwalking::is_body(a)
}
/// Includes temporarily covered stairs so a later restoration cannot place a
/// stair under a creature. O(M*K+T), M modifications, K<=9 cells, T temporary edits.
pub fn stair_cell(f: &Floor, at: usize) -> bool {
    f.tiles.get(at).is_some_and(|t| tile(t))
        || f.revision50_terrain.as_ref().is_some_and(|mods| mods.iter().any(|m|
            m.cells.iter().zip(&m.previous_tiles).any(|(c,previous)|
                tile(previous) && c.y as usize*f.width()+c.x as usize==at)))
        || f.revision50_temporary_terrain.as_ref().is_some_and(|edits| edits.iter()
            .any(|edit| edit.cell==at && tile(&edit.previous)))
        || f.altered_chasms.as_ref().is_some_and(|edits| edits.get(&at.to_string())
            .is_some_and(|edit| tile(&edit.tile)))
}
pub fn blocked_cell(r: &Run, a: &Actor, f: &Floor, at: usize) -> bool {
    restricted(r, a) && stair_cell(f, at)
}
pub fn blocked(r: &Run, a: &Actor, x: f64, y: f64) -> bool {
    r.floor().is_some_and(|f| crate::portals::cell(f, x, y)
        .is_some_and(|at| blocked_cell(r, a, f, at)))
}
/// Revived corpses become hostile even if they were allies before death.
pub fn blocks_revival(r: &Run, f: &Floor, a: &Actor) -> bool {
    r.simulation_revision.value_or(29.) >= 51. && crate::portals::cell(f, a.x, a.y)
        .is_some_and(|at| stair_cell(f, at))
}
/// A stair chase ends at its accessible edge instead of standing on the stair.
/// Constant geometry work plus the existing actor-passability/terrain lookup;
/// the caller owns hold/status gates. See the stair complexity ledger.
pub(crate) fn at_chase_exit(r: &Run, a: &Actor, x: f64, y: f64) -> bool {
    let Some(f) = r.floor() else { return false; };
    let actual_stair = crate::portals::cell(f,x,y).is_some_and(|at|tile(&f.tiles[at]));
    if !restricted(r,a) || !actual_stair { return a.x == x && a.y == y; }
    let (dx, dy) = ((a.x-x).abs(), (a.y-y).abs());
    dx.max(dy) == 1. && movement::diagonal_step_allowed(&f.tiles, f.width(),
        Point{x:a.x as i32,y:a.y as i32}, Point{x:x as i32,y:y as i32},
        a.active.iter().any(|id| id == "etill-phase-through"))
        && engine::actor_passable(r, a, a.x, a.y)
}

#[cfg(test)]
#[path = "revision51_stairs_tests.rs"]
mod tests;
