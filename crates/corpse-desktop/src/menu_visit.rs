//! Main-menu visits are UI transitions, not paints or simulation commands.
use crate::{avatar_menu::{self, Selection}, Screen};

#[derive(Default)]
pub struct Visits { last_active: Option<bool> }

impl Visits {
    pub fn observe(&mut self, screen:Screen, preparing:bool, selection:&mut Selection,
                   seed:&mut String, notice:&mut String) -> bool {
        self.observe_with(screen,preparing,selection,seed,notice,random_values)
    }
    /// O(1) at every input/frame boundary, O(K + S) only on entry (K=12).
    /// Preparing on the menu temporarily leaves its editable state. Observe both
    /// before and after polling so immediate success/error cannot hide that exit.
    fn observe_with(&mut self, screen:Screen, preparing:bool, selection:&mut Selection,
                    seed:&mut String, notice:&mut String,
                    random:impl FnOnce()->Result<[u32;3],String>) -> bool {
        let active=screen==Screen::Menu && !preparing;
        let previous=self.last_active.replace(active);
        if !active || previous==Some(true) {return false;}
        // An explicitly supplied initial full seed is an exact startup request.
        // Returning from CLI DEBUG/replay/manual is a later entry, not this case.
        if previous.is_none() && avatar_menu::numeric_prefix(seed.trim()) {return false;}
        match random() {
            Ok(values)=>if let Err(error)=selection.randomize(seed,values) {
                *notice=format!("Gods randomized. {error} Shorten the seed before descending.");
            },
            Err(error)=>*notice=format!("Could not randomize menu gods: {error}"),
        }
        true
    }
}

fn random_values()->Result<[u32;3],String> {
    let mut bytes=[0u8;12];
    getrandom::fill(&mut bytes).map_err(|error|error.to_string())?;
    Ok(std::array::from_fn(|i|u32::from_le_bytes(bytes[i*4..i*4+4].try_into().unwrap())))
}

#[cfg(test)]
#[path="menu_visit_tests.rs"]
mod tests;
