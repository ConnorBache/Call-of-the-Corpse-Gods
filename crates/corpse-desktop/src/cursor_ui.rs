//! Keyboard-owned inspection coordinates. This module never reads world state,
//! dispatches a command, changes a Run, advances time, or consumes gameplay RNG.
use sdl2::keyboard::{Keycode, Mod};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Grid {
    pub width: usize,
    pub height: usize,
    pub player: (usize, usize),
}
impl Grid {
    pub fn contains(self, (x, y): (usize, usize)) -> bool { x < self.width && y < self.height }
    fn anchor(self, hover: Option<(usize, usize)>) -> Option<(usize, usize)> {
        if self.width == 0 || self.height == 0 { return None; }
        Some(hover.filter(|tile| self.contains(*tile)).unwrap_or((
            self.player.0.min(self.width - 1), self.player.1.min(self.height - 1))))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Pass,
    Consumed,
    Inspect((usize, usize)),
    Confirm((usize, usize)),
    Exit,
}
#[derive(Default)]
pub struct Cursor { active: bool }
impl Cursor {
    pub fn active(&self) -> bool { self.active }
    pub fn clear(&mut self) -> bool { std::mem::replace(&mut self.active, false) }
    /// O(1), O(1) state. Uses the same direction aliases as normal movement;
    /// only the detached coordinate changes. View observation/admission stays
    /// with the existing pointer pipeline in main, never this controller.
    pub fn key(&mut self, key: Keycode, modifiers: Mod, repeat: bool,
        grid: Grid, hover: Option<(usize, usize)>) -> Action {
        if !crate::aim_modifiers_allowed(modifiers) { return Action::Pass; }
        // Holding Escape after cursor exit is still the same physical press.
        if key == Keycode::Escape && repeat { return Action::Consumed; }
        if matches!(key, Keycode::Return | Keycode::KpEnter) {
            if repeat { return Action::Consumed; }
            let Some(tile) = grid.anchor(hover) else { self.clear(); return Action::Consumed; };
            if self.clear() { return Action::Confirm(tile); }
            self.active = true;
            return Action::Inspect(tile);
        }
        if !self.active { return Action::Pass; }
        if key == Keycode::Escape { self.clear(); return Action::Exit; }
        if let Some(corpse_core::Command::Move { dx, dy }) = crate::key_command(key) {
            let Some((x, y)) = grid.anchor(hover) else { self.clear(); return Action::Exit; };
            let tile = (x.saturating_add_signed(dx as isize).min(grid.width - 1),
                y.saturating_add_signed(dy as isize).min(grid.height - 1));
            return Action::Inspect(tile);
        }
        // An ordinary command or another panel takes ownership before it runs.
        // In particular reload/revert/new targeting cannot retain stale mode.
        self.clear();
        Action::Pass
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestKey { None, Once, UntilChange }
/// Selection is modal to the new Z binding: it cannot accidentally rest while
/// aiming an item or divine ability. Existing wait aliases are unchanged.
pub fn rest_key(key: Keycode, modifiers: Mod, selecting: bool) -> RestKey {
    if key != Keycode::Z || selecting || !crate::aim_modifiers_allowed(modifiers) {
        RestKey::None
    } else if modifiers.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD) {
        RestKey::UntilChange
    } else { RestKey::Once }
}

#[cfg(test)]
#[path = "cursor_ui_tests.rs"]
mod tests;
