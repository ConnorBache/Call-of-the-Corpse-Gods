//! Read-only resource capability of an already admitted, apparent actor.
//! Never look up a live actor by ID here: disguises and hallucinations pass
//! detached bodies, and the Bestiary passes public birth templates.
use corpse_core::{Actor, Run, abilities, enemy_traits};

/// Capability, not present affordability, cooldown, silence, sleep or HP.
/// O((K+D)*L) expected time and O(K*L) storage for K powers, fixed definition
/// rows D and maximum name/ID length L through legacy trait inference.
/// No Run, RNG or writes.
pub fn energy_capable(revision: f64, a: &Actor) -> bool {
    // Inert hallucinations have no AI phase. These never inherit real powers.
    if corpse_core::revision50_nightmares::is_decoy(a) { return false; }
    let traits = enemy_traits::enemy_traits(a);
    let hold_only = traits.iter().any(|s| s == "enemy:beast:hold-only");
    if traits.iter().any(|id| match id.as_str() {
        // ai::enemy_action_options and engine's post-strike callbacks.
        "enemy:pale:breath-boost" | "enemy:pale:scream" | "enemy:pale:gas-shot"
        | "enemy:pale:birth" | "enemy:pale:bite-repeat" | "enemy:pale:lifesteal" => true,
        // monster_spells, witch, saidribee_queen (Queen begins at R33).
        "enemy:beast:bloodmancy" | "enemy:beast:witch" | "enemy:beast:dryad" => true,
        "enemy:beast:saidribee-queen" => revision >= 33.,
        // Remaining paid specials are skipped by hold-only in the AI dispatcher.
        "enemy:beast:detonate" | "enemy:beast:blink" | "enemy:beast:moon-laser"
        | "enemy:beast:water-jet" | "enemy:beast:ruin-pulse" | "enemy:beast:beckon"
        | "enemy:beast:wall-seal" | "enemy:beast:horn-bolt" => !hold_only,
        // Double Birth alone cannot execute: the branch requires Birth.
        _ => false,
    }) { return true; }
    // Fire breath and Vampire bat form are free. R52 Dragon takeoff is paid
    // by species, even when legacy trait initialization has already occurred.
    if revision == 52. && corpse_core::revision52_enemy_followups::species(a, "dragon") {
        return true;
    }
    // Acquired passive protection spends main energy without an active toggle.
    if corpse_core::hanged::enabled(revision) && a.abilities.iter().any(|id| id == corpse_core::hanged::ID) {
        return true;
    }
    // Monsters do not run the player's activation command. Only copied active
    // upkeep/combat powers execute on their own bodies; ownership of an unused
    // player-only action or an energy-capacity passive is not sufficient.
    a.active.iter().any(|id| abilities::upkeep_cost_for(a, id) > 0.
        || matches!(id.as_str(), "dawel-unspeak" | "anavani-nova"
            | "grisha-erase-metaphysical" | "pelian-never-without-a-fight"
            | "pelian-never-surrender" | "shrail-rage-at-the-dying-of-the-light")
        || (revision >= 50. && matches!(id.as_str(), "shrail-execute" | "anavani-commandment")))
}

#[cfg(test)]
#[path = "monster_status_ui_tests.rs"]
mod tests;

/// Visible ecology only: core `Eating` means an active kind-1 Bloody-wall meal,
/// never proximity, hunger or corpse eating. Keep the canonical clock untouched.
/// The caller reuses its observation/projection decisions; no extra LOS pass.
/// Cost is the existing feeding-description priority check (see ecology ledger).
pub fn feeding_description(r: &Run, a: &Actor, direct: bool, projected: bool) -> Option<&'static str> {
    if !direct || projected { return None; }
    corpse_core::creature_feeding::description(r, a).map(|activity| {
        if activity == "Eating" { "Feeding" } else { activity }
    })
}

#[cfg(test)]
#[path = "feeding_state_ui_tests.rs"]
mod feeding_state_tests;
