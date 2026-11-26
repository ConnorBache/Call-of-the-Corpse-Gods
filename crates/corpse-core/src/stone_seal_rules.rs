//! Stone Seal expiry belongs to the floor and plane that owns the tile, not
//! its caster. Legacy command prefixes retain their actor-owned expiry.
use crate::{Field, Floor, Run};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Seal {
    pub cell: usize,
    pub expires_at: f64,
}

pub fn current(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 52. && r.stone_seal_from.as_ref().is_some()
}

pub(crate) fn validate(r: &Run) -> Result<(), String> {
    let revision = r.simulation_revision.value_or(29.);
    if !r.stone_seal_from.is_missing()
        && (revision < 52. || r.stone_seal_from.as_ref().is_some_and(|&n| n > r.replay.len()))
    {
        return Err("Invalid Stone Seal command boundary".into());
    }
    for f in r.floors.iter().chain(r.dreamwalking.as_ref().into_iter().flat_map(|s| &s.floors)) {
        if let Some(seals) = f.stone_seals.as_ref() {
            let mut cells = std::collections::HashSet::new();
            if revision < 52. || seals.is_empty() || seals.iter().any(|s|
                s.cell >= f.tiles.len() || !cells.insert(s.cell) || !s.expires_at.is_finite()
                || s.expires_at < 0. || s.expires_at.fract() != 0.)
            {
                return Err("Invalid Stone Seal floor state".into());
            }
        } else if matches!(f.stone_seals, Field::Null) {
            return Err("Invalid null Stone Seal floor state".into());
        }
    }
    Ok(())
}

pub fn admit_current(r: &mut Run) -> Result<(), String> {
    validate(r)?;
    if r.simulation_revision.value_or(29.) >= 52. && r.stone_seal_from.is_missing() {
        r.stone_seal_from = r.replay.len().into();
    }
    Ok(())
}

pub(crate) fn apply_replay_policy(r: &mut Run, policy: &Field<usize>, cursor: usize) {
    if r.simulation_revision.value_or(29.) < 52. { return; }
    if policy.as_ref() == Some(&cursor) { r.stone_seal_from = cursor.into(); }
    else if cursor == 0 { r.stone_seal_from = Field::Null; }
}

pub(crate) fn contains(f: &Floor, cell: usize) -> bool {
    f.stone_seals.as_ref().is_some_and(|seals| seals.iter().any(|seal| seal.cell == cell))
}

pub(crate) fn pending(r: &Run, depth: f64) -> bool {
    current(r) && r.floors.iter().chain(r.dreamwalking.as_ref().into_iter().flat_map(|s| &s.floors))
        .any(|f| f.depth == depth && f.stone_seals.as_ref().is_some_and(|s| !s.is_empty()))
}

/// The admission branch guarantees ordinary 'floor' and no overlapping lease.
pub(crate) fn record(r: &mut Run, cell: usize, expires_at: f64) {
    let f = r.floor_mut().unwrap();
    f.stone_seals.get_or_insert_with(Vec::new).push(Seal { cell, expires_at });
}

/// A concrete wall-destroying edit cancels that lease before replacing tiles.
pub(crate) fn forget_where(f: &mut Floor, erased: impl Fn(usize) -> bool) {
    let Some(seals) = f.stone_seals.as_mut() else { return; };
    seals.retain(|seal| !erased(seal.cell));
    if seals.is_empty() { f.stone_seals = Field::Missing; }
}

/// O(F + S) due check per available plane of this depth for F floors, S seals.
/// The ordered drain is O(S) space on an expiry; each actual open queues the
/// existing liquid/frontier work. No cross-depth lookup or actor dependency.
pub(crate) fn expire(r: &mut Run) {
    if !current(r) { return; }
    let revision = r.simulation_revision.value_or(29.);
    let tick = r.tick;
    for dream in [false, true] {
        if dream && !crate::dreamwalking::enabled(r) { continue; }
        if !crate::dreamwalking::floor_in_plane(r, dream)
            .and_then(|f| f.stone_seals.as_ref()).is_some_and(|seals| seals.iter().any(|s| s.expires_at <= tick))
        {
            continue;
        }
        crate::dreamwalking::with_plane(r, dream, |r| {
            let f = r.floor_mut().unwrap();
            let width = f.width();
            let mut retained = Vec::new();
            let mut openings = Vec::new();
            for seal in std::mem::take(f.stone_seals.as_mut().unwrap()) {
                if seal.expires_at > tick {
                    retained.push(seal);
                } else if f.tiles.get(seal.cell).is_some_and(|t| t == "wall") {
                    f.tiles[seal.cell] = "floor".into();
                    crate::ichor::enqueue_opening_revision(f, seal.cell, revision);
                    openings.push(seal.cell);
                }
            }
            f.stone_seals = if retained.is_empty() { Field::Missing } else { retained.into() };
            if !openings.is_empty() { f.dirty = true; }
            for cell in openings {
                crate::events::at(r, "E40", "fracture-stone", crate::events::Stage::Special,
                    &[crate::events::EventPoint { x: (cell % width) as f64, y: (cell / width) as f64 }]);
            }
        });
    }
}

/// A newly created dream layer copies the base tile, not waking Stone Seals.
pub(crate) fn strip_copied_seals(f: &mut Floor) {
    if let Some(seals) = f.stone_seals.as_ref() {
        for seal in seals {
            if let Some(tile) = f.tiles.get_mut(seal.cell).filter(|tile| *tile == "wall") {
                *tile = "floor".into();
            }
        }
        f.stone_seals = Field::Missing;
    }
}
