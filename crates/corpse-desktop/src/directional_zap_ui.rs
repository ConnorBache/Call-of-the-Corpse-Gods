//! Empty-space ray aiming uses the same command and observation gates as clicks.
//! No new simulation command, target lookup, random choice or saved input state.
use crate::{pack::PackTarget, render::View};
use corpse_core::{Command, Run, revision50_amulet_effects as amulets};
use sdl2::keyboard::Keycode;

/// O(I) inventory lookup, O(1) scratch. Endpoint and creature-only amulets keep
/// their existing cursor semantics; only the four continuing bolts use a ray.
pub fn directional(run: &Run, target: &PackTarget) -> bool {
    let PackTarget::UseAt(id) = target else { return false; };
    run.simulation_revision.value_or(29.) >= 52.
        && run.inventory.iter().any(|item| item.id == *id && amulets::is_projectile(item))
}

/// The first direction key means a ray from the player, not a stale mouse tile.
/// Later keys adjust the detached cursor normally. O(I), no visibility queries.
pub fn keyboard_tile(run: &Run, target: Option<&PackTarget>, hover: Option<(usize, usize)>,
    keyboard_aim: bool, key: Keycode) -> Option<(usize, usize)> {
    let anchor = !keyboard_aim && target.is_some_and(|target| directional(run, target));
    crate::aim_keyboard_tile(run, if anchor { None } else { hover }, key)
}

pub fn prompt(run: &Run, target: &PackTarget) -> &'static str {
    if directional(run, target) {
        "Aim in a direction or click an empty tile. Direction keys aim; Enter zaps. Escape cancels."
    } else if target.is_amulet(run) {
        "Select an amulet target. Escape cancels without spending energy or an action."
    } else if matches!(target, PackTarget::UseAt(_)) {
        "Select a visible adjacent living creature. Escape cancels."
    } else {
        "Select a visible target. Escape cancels."
    }
}

/// The production mouse and Enter handlers share this admission. Checking a
/// projected cell does not require a creature. O(existing target admission + V)
/// for V detached view cells, O(1) extra memory. Never inspect hidden occupants.
pub fn command(run: &Run, view: &View, target: &PackTarget,
    (x, y): (usize, usize)) -> Result<Command, &'static str> {
    if let Some(reason) = target.target_blocker(run, x as f64, y as f64) { return Err(reason); }
    if !view.cells.iter().any(|cell| cell.x == x && cell.y == y) {
        return Err("Select a known map location.");
    }
    Ok(target.command(x as f64, y as f64))
}

#[cfg(test)]
#[path = "directional_zap_ui_tests.rs"]
mod tests;
