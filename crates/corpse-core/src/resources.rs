//! v48 resource history: observed baselines and net presentation loss.
use crate::model::*;
use indexmap::IndexMap;
fn observable(r: &Run, a: &Actor) -> bool {
    r.player.x >= 0.0
        && (a.id == r.player.id || !a.dead.value_or(false) && crate::vision::visible_actor(r, a))
}
fn meters(r: &Run, a: &Actor) -> IndexMap<String, ResourceMeterHistory> {
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
                max: 2.0f64.powf(r.level),
                loss: 0.0,
                display_reference: Field::Missing,
            },
        );
    }
    m
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
        .map(|a| (a.id.to_string(), meters(r, a)))
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
        .map(|a| (a.id.to_string(), meters(r, a)))
        .collect();
    let h = r.resource_history.as_mut().unwrap();
    if h.depth != r.depth {
        h.actors.retain(|id, _| id == &r.player.id.to_string());
        h.depth = r.depth;
    }
    h.actors
        .retain(|id, _| observed.iter().any(|(i, _)| i == id));
    for (id, m) in observed {
        h.actors.entry(id).or_insert(m);
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
    let m = meters(r, a);
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
