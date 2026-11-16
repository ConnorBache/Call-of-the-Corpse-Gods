//! R52 Hanged replaces the stable Missed Me slot with passive damage redirection.
use crate::{Actor, Run};
pub const ID: &str = "isheilah-missed-me";
pub const DESCRIPTION: &str = "Passive. Damage that would reduce you below 1 HP uses your main energy bar instead, at 4 energy per HP. This includes excess damage from the same hit. If energy runs out, uncovered damage reduces HP and can kill you. Health costs and unpreventable deaths are not protected. Does not grant or spend reactions.";
pub fn enabled(revision: f64) -> bool { revision >= 52. }
/// Passive ownership is shared, including the sleeping Dreamwalk body.
pub fn protects(r: &Run, body: &Actor) -> bool {
    enabled(r.simulation_revision.value_or(29.))
        && crate::engine::actor(r, crate::half_here::owner_id(r, body)).unwrap_or(body)
            .abilities.iter().any(|id| id == ID)
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Loss { pub hp: f64, pub energy: f64 }
/// O(1), no RNG. Input is the already mitigated/rounded incoming packet.
/// Preserve fractional resources; multiplying/dividing by four adds no rounding policy.
/// A body already below 1 HP is protected at its actual HP, never healed up to 1.
pub fn split(hp: f64, energy: f64, damage: f64) -> Loss {
    let unprotected = Loss { hp: damage, energy: 0. };
    if hp <= 0. || damage <= 0. || energy <= 0.
        || !hp.is_finite() || !energy.is_finite() || !damage.is_finite() { return unprotected; }
    let below_one = (damage - (hp - 1.).max(0.)).max(0.);
    let absorbed = below_one.min(energy / 4.);
    Loss { hp: damage - absorbed, energy: absorbed * 4. }
}
/// Restore/import compatibility for earlier R52 states. No refill, ID remap or RNG.
/// Actor-local O(K), plus recursively stored pilot metadata. Missing reactions stay missing.
pub fn normalize_actor(revision: f64, a: &mut Actor) {
    if !enabled(revision) { return; }
    a.active.retain(|id| id != ID);
    let cap = crate::engine::max_reactions_for_revision(revision, a);
    if let Some(n) = a.reactions.as_mut() { *n = n.min(cap); }
    if let Some(pilot) = a.monster_state.as_mut().and_then(|s| s.pilot.as_mut()) {
        normalize_actor(revision, pilot);
    }
}
/// Load/new-engine boundary only: O(A*K), existing actor COW detach costs, no per-frame scan.
pub fn normalize_world(r: &mut Run) {
    let revision = r.simulation_revision.value_or(29.);
    if !enabled(revision) { return; }
    normalize_actor(revision, &mut r.player);
    if let Some(a) = r.initial_player.as_mut() { normalize_actor(revision, a); }
    for actors in r.actors.values_mut() {
        if actors.iter().any(|a| a.active.iter().any(|id|id==ID)
            || a.reactions.as_ref().is_some_and(|&n|n>crate::engine::max_reactions_for_revision(revision,a))
            || a.monster_state.as_ref().and_then(|s|s.pilot.as_ref()).is_some()) {
            for a in crate::mutation_journal::make_mut(actors) { normalize_actor(revision, a); }
        }
    }
}
