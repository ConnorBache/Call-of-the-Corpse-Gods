//! Source ui-travel waitKnowledge/changedWaitState. All threat records are observed.
use corpse_core::{vision, Run};
pub struct Knowledge {
    threats: Vec<(u64, (String, bool))>,
    conditions: Vec<String>,
    hp: f64,
    damage: f64,
    tick: f64,
    depth: f64,
    x: f64,
    y: f64,
    status: String,
    choosing: bool,
    hazard: Option<String>,
}
pub fn observe(r: &Run) -> Knowledge {
    let mut conditions = crate::statuses_ui::observe(r, &r.player)
        .into_iter()
        .map(|s| s.name)
        .collect::<Vec<_>>();
    conditions.sort();
    Knowledge {
        threats: r
            .actors_at(r.depth)
            .iter()
            .filter(|a| vision::visible_actor(r, a))
            .map(|a| (a.id.to_bits(), (a.state.clone(), a.dead.value_or(false))))
            .collect(),
        conditions,
        hp: r.player.hp,
        damage: r.damage_since_last_turn.value_or(0.),
        tick: r.tick,
        depth: r.depth,
        x: r.player.x,
        y: r.player.y,
        status: r.status.clone(),
        choosing: !r.offers.is_empty(),
        hazard: crate::hazards_ui::waiting(r),
    }
}
pub fn changed(before: &Knowledge, after: &Knowledge) -> Option<String> {
    if after.status != "playing" {
        return Some("The run has ended.".into());
    }
    if after.choosing {
        return Some("Choose your level-up ability.".into());
    }
    if after.damage > 0. || after.hp < before.hp {
        return Some("You took damage.".into());
    }
    for (id, (state, dead)) in &after.threats {
        if *dead || matches!(state.as_str(), "ally" | "neutral") {
            continue;
        }
        let old = before
            .threats
            .iter()
            .find(|(old_id, _)| old_id == id)
            .map(|(_, threat)| threat);
        if old.is_none_or(|(_, dead)| *dead) {
            return Some("A creature has come into view.".into());
        }
        let old = &old.unwrap().0;
        if old == "sleeping" && state != "sleeping" {
            return Some("A creature has awakened.".into());
        }
        if old != "stalking" && state == "stalking" {
            return Some("A creature has begun stalking.".into());
        }
        if old != "hunting" && state == "hunting" {
            return Some("A creature has begun hunting.".into());
        }
    }
    if before.depth != after.depth || before.x != after.x || before.y != after.y {
        return Some("You were moved.".into());
    }
    if let Some(name) = before
        .conditions
        .iter()
        .find(|n| !after.conditions.contains(n))
    {
        return Some(format!("{name} ended."));
    }
    if let Some(name) = after
        .conditions
        .iter()
        .find(|n| !before.conditions.contains(n))
    {
        return Some(format!("{name} began."));
    }
    if after.hazard.is_some() && after.hazard != before.hazard {
        return Some("The terrain has become hazardous.".into());
    }
    if after.tick <= before.tick {
        return Some("Time did not advance.".into());
    }
    None
}
pub struct Rest {
    pub knowledge: Knowledge,
    pub next: f64,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn threat_reasons_keep_source_actor_order() {
        let r = corpse_core::storage::deserialize(include_str!(
            "../../../fixtures/avatar-room-v1.json"
        ))
        .unwrap();
        let mut before = observe(&r);
        let mut after = observe(&r);
        before.threats = vec![(2, ("sleeping".into(), false)), (1, ("idle".into(), false))];
        after.threats = vec![(2, ("idle".into(), false)), (1, ("hunting".into(), false))];
        after.tick += 1.;
        assert_eq!(
            changed(&before, &after).as_deref(),
            Some("A creature has awakened.")
        );
    }
    #[test]
    fn damage_precedes_threats_and_time() {
        let mut r = corpse_core::storage::deserialize(include_str!(
            "../../../fixtures/avatar-room-v1.json"
        ))
        .unwrap();
        let before = observe(&r);
        assert_eq!(
            changed(&before, &observe(&r)),
            Some("Time did not advance.".into())
        );
        r.tick += 1.;
        r.player.hp -= 1.;
        assert_eq!(
            changed(&before, &observe(&r)),
            Some("You took damage.".into())
        );
    }
}
