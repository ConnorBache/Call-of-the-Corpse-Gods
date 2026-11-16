//! Read-only revision33 actor appearance identity shared by UI and message producers.
//! Callers gate visibility before asking for a visible name. No actor, AI, stats,
//! serialized RNG or simulation targeting is changed by this projection.
use crate::{Actor, Run};

/// Current runs use target-specific telepathy, not nearby sensed terrain.
/// Older revisions retain their recorded presentation behavior.
pub fn identity_known(r: &Run, a: &Actor) -> bool {
    r.simulation_revision.value_or(29.) >= 52. && crate::vision::telepathic(r, a)
}

/// O(A + B*R + L), with B<=2 body origins, ray length R and seed bytes L.
/// Uses the same fixed actor disguise and deterministic hallucination hash everywhere.
pub fn apparent_kind(r: &Run, a: &Actor) -> Option<&'static str> {
    if r.simulation_revision.value_or(29.) < 33.
        || a.id == r.player.id || crate::half_here::is_clone(a) || identity_known(r, a) { return None; }
    let mut kind = crate::new_creatures::disguise_kind(r, a);
    if crate::half_here::origins(r).iter().any(|body|
        !body.dead.value_or(false) && crate::engine::condition(body, "hallucinating") > 0.) {
        let mut hash = 2166136261u32;
        for b in r.seed.bytes() { hash = (hash ^ b as u32).wrapping_mul(16777619); }
        hash = hash.wrapping_add((r.tick as u32).wrapping_mul(0x9e3779b9))
            ^ (a.id as u32).wrapping_mul(0x85ebca6b);
        kind = Some(crate::new_creatures::DISGUISES[
            hash as usize % crate::new_creatures::DISGUISES.len()]);
    }
    kind
}

/// Actor-aware visible name, not a replacement applied to a completed log string.
/// The catalog owns apparent names permanently, so no RNG or allocation is needed.
pub fn name<'a>(r: &Run, a: &'a Actor) -> &'a str {
    apparent_kind(r, a).and_then(|kind| crate::population::catalog().actors.get(kind))
        .map_or(a.name.as_str(), |template| template.name.as_str())
}

/// Death-summary narration can reference an unseen or offscreen source. Preserve
/// legacy messages exactly, but never turn a revision33 hidden source into a reveal.
pub fn source_name<'a>(r: &Run, a: &'a Actor) -> &'a str {
    if r.simulation_revision.value_or(29.) >= 33. && a.id != r.player.id
        && !crate::vision::visible_actor(r, a) { "Something unseen" } else { name(r, a) }
}
