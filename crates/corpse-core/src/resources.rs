//! Observed turn baselines. R52 also retains player blessing/amulet reserves.
//! Presentation reads signed net changes without altering simulation or clocks.
use crate::model::*;
use indexmap::IndexMap;
fn observable(r: &Run, a: &Actor) -> bool {
    r.player.x >= 0.0
        && (a.id == r.player.id || !a.dead.value_or(false) && crate::vision::visible_actor(r, a))
}
fn meters(r: &Run, a: &Actor, include_reserves: bool) -> IndexMap<String, ResourceMeterHistory> {
    let mut m = IndexMap::new();
    let boundless = a.abilities.iter().any(|id| id == "grisha-boundless-hunger");
    for (k, v, max) in [
        ("hp", a.hp.max(0.0), a.max_hp),
        (
            "energy",
            a.energy.max(0.0),
            if boundless {
                100.0
            } else {
                crate::combat::effective_stats(r, a).max_energy
            },
        ),
    ] {
        m.insert(
            k.into(),
            ResourceMeterHistory {
                start: v,
                max: if max.is_finite() && max >= 0.0 {
                    max
                } else {
                    0.0
                },
                loss: 0.0,
                display_reference: if k == "energy" && boundless {
                    Field::Value(true)
                } else {
                    Field::Missing
                },
            },
        );
    }
    if a.id == r.player.id {
        m.insert(
            "blessings".into(),
            ResourceMeterHistory {
                start: r.exp,
                max: crate::progression::level_up_cost(r),
                loss: 0.0,
                display_reference: Field::Missing,
            },
        );
    }
    if include_reserves && a.id == r.player.id && r.simulation_revision.value_or(29.) >= 52. {
        for (id, reserve) in a.ability_reserves.as_ref().into_iter().flatten() {
            if crate::revision50_ability_energy::valid_reserve(reserve) {
                let reserve = crate::revision52_reserves::project_reserve(*reserve, 52.);
                m.insert(ability_key(id), meter(reserve.energy, reserve.capacity));
            }
        }
        for item in &r.inventory {
            if let Some(spec) = crate::revision50_amulet_effects::item_spec_for_revision(item, 52.) {
                let energy = item.amulet_state.as_ref().map_or(spec.capacity, |s| s.energy.min(spec.capacity));
                m.insert(amulet_key(item.id), meter(energy, spec.capacity));
            }
        }
    }
    m
}
fn meter(start: f64, max: f64) -> ResourceMeterHistory {
    ResourceMeterHistory { start, max, loss: 0., display_reference: Field::Missing }
}
pub fn ability_key(id: &str) -> String { format!("ability:{id}") }
pub fn amulet_key(id: f64) -> String { format!("amulet:{id}") }
/// Pure signed net change / current maximum. Unbounded main energy retains the
/// existing 100-energy display reference; absent/invalid baselines stay unknown.
/// O(ID bytes) expected hash work, O(H * ID bytes) collision worst case, O(1) space.
pub fn player_change_percent(r: &Run, key: &str, current: f64, maximum: f64) -> Option<f64> {
    if r.simulation_revision.value_or(29.) < 52. { return None; }
    let baseline = r.resource_history.as_ref()?.actors.get(&r.player.id.to_string())?.get(key)?;
    let reference = if maximum == f64::INFINITY && key == "energy" { 100. } else { maximum };
    if !current.is_finite() || !baseline.start.is_finite() || !reference.is_finite() || reference <= 0. { return None; }
    let change = (current.max(0.) - baseline.start.max(0.)) / reference * 100.;
    change.is_finite().then_some(change)
}
/// One immutable visibility projection per batch: O(C + A*(Q+M)) time, O(A*M+P)
/// output/scratch, C projection setup, Q visibility query, M meter/stat projection,
/// P projection storage. Retains actor iteration order and scalar record_loss path.
pub fn begin_resource_turn(r: &mut Run) {
    if r.player.x < 0.0 {
        return;
    }
    let visibility = crate::vision::VisibilityProjection::new(r);
    let actors = std::iter::once(&r.player)
        .chain(r.actors_at(r.depth))
        .filter(|a| a.id == r.player.id || !a.dead.value_or(false) && visibility.visible_actor(a))
        .map(|a| (a.id.to_string(), meters(r, a, true)))
        .collect();
    r.resource_history = Field::Value(ResourceTurnHistory {
        tick: r.tick,
        depth: r.depth,
        actors,
    });
}
/// O(C + A*(Q+M) + H*A) time and O(A*M+P) scratch. Visibility is batched;
/// existing history retain/matching remains unchanged (H previous entries).
pub fn observe_resource_history(r: &mut Run) {
    let _timing_function = crate::timing::span("history.resources");
    if r.player.x < 0.0 {
        return;
    }
    if r.resource_history.as_ref().is_none() {
        begin_resource_turn(r);
    }
    let visibility = crate::vision::VisibilityProjection::new(r);
    let observed: Vec<_> = std::iter::once(&r.player)
        .chain(r.actors_at(r.depth))
        .filter(|a| a.id == r.player.id || !a.dead.value_or(false) && visibility.visible_actor(a))
        .map(|a| (a.id.to_string(), meters(r, a, true)))
        .collect();
    let current = r.simulation_revision.value_or(29.) >= 52.;
    let player = r.player.id.to_string();
    let h = r.resource_history.as_mut().unwrap();
    if h.depth != r.depth {
        h.actors.retain(|id, _| id == &r.player.id.to_string());
        h.depth = r.depth;
    }
    h.actors
        .retain(|id, _| observed.iter().any(|(i, _)| i == id));
    for (id, m) in observed {
        if current && id == player {
            // Old saves/newly acquired reserves start at their first observed
            // value. Existing turn baselines must survive free commands.
            let previous = h.actors.entry(id).or_default();
            for (key, value) in m { previous.entry(key).or_insert(value); }
        } else { h.actors.entry(id).or_insert(m); }
    }
}
pub fn record_loss(r: &mut Run, a: &Actor, key: &str, amount: f64) {
    if r.player.x < 0.0 {
        return;
    }
    if r.resource_history.as_ref().is_none() {
        begin_resource_turn(r);
    }
    if !amount.is_finite() || amount <= 0.0 {
        return;
    }
    let observed = observable(r, a);
    let m = meters(r, a, false);
    let h = r.resource_history.as_mut().unwrap();
    if !observed {
        h.actors.shift_remove(&a.id.to_string());
        return;
    }
    if h.depth != r.depth && a.id != r.player.id {
        return;
    }
    if let Some(entry) = h.actors.entry(a.id.to_string()).or_insert(m).get_mut(key) {
        entry.loss += amount;
    }
}
pub fn record_player_energy_loss(r: &mut Run, amount: f64) {
    let p = r.player.clone();
    record_loss(r, &p, "energy", amount);
}

#[cfg(test)]
#[path = "resource_turn_tests.rs"]
mod tests;
