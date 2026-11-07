//! Authored Tiger/Sabretooth observed-position stalking and doorway peeks.
use crate::{
    Actor, Field, Run,
    engine::{self, actor, actor_mut_trusted},
    movement::{DIRECTIONS, Point},
};
fn distance(x: f64, y: f64, a: f64, b: f64) -> f64 {
    (x - a).hypot(y - b)
}
fn doorway(r: &Run, x: i32, y: i32) -> bool {
    let f = r.floor().unwrap();
    let solid = |x: i32, y: i32| {
        let i = y as i64 * f.width() as i64 + x as i64;
        i >= 0
            && f.tiles
                .get(i as usize)
                .is_some_and(|t| t == "wall" || t == "tree")
    };
    (solid(x - 1, y) && solid(x + 1, y)) || (solid(x, y - 1) && solid(x, y + 1))
}
fn free(r: &Run, a: &Actor, x: f64, y: f64) -> bool {
    let f = r.floor().unwrap();
    let i = y as usize * f.width() + x as usize;
    !["wall", "tree", "chasm", "pit"].contains(&f.tiles[i].as_str())
        && !crate::environment::blocks_movement(f, i)
        && !engine::bulwark_blocks(r, a, x, y)
        && !(r.player.id != a.id && crate::portals::same_position(r, &r.player, x, y)
            && crate::dreamwalking::same_plane(r, a, &r.player))
        && !r
            .actors_at(r.depth)
            .iter()
            .any(|b| b.id != a.id && !b.dead.value_or(false) && crate::portals::same_position(r, b, x, y)
                && crate::dreamwalking::same_plane(r, a, b))
        && crate::ichor::movement_allowed(r, a, x, y)
}
/// At most 25 occupancy/visibility probes followed by one hazard-aware path query.
pub fn turn(r: &mut Run, id: f64, detected: Option<f64>) -> Result<bool, String> {
    let a = actor(r, id).ok_or("Stalking actor missing")?.clone();
    if !a.abilities.iter().any(|s| s == "enemy:beast:stalk")
        || ["sleeping", "ally", "neutral"].contains(&a.state.as_str())
    {
        return Ok(false);
    }
    let detected = detected.and_then(|id| actor(r, id)).cloned();
    if detected
        .as_ref()
        .is_some_and(|b| crate::portals::actor_distance(r, &a, b) <= 4.0)
    {
        let a = actor_mut_trusted(r, id).unwrap();
        a.state = "hunting".into();
        a.conditions.shift_remove("stalkWait");
        return Ok(false);
    }
    if let Some(b) = detected {
        let a = actor_mut_trusted(r, id).unwrap();
        if let Some(x) = a.conditions.get("stalkObservedX") {
            let dx = (b.x - x).signum();
            let dy = a
                .conditions
                .get("stalkObservedY")
                .map_or(f64::NAN, |y| (b.y - y).signum());
            let dx = if b.x == *x { 0.0 } else { dx };
            let dy = if a.conditions.get("stalkObservedY") == Some(&b.y) {
                0.0
            } else {
                dy
            };
            if dx != 0.0 || (!dy.is_nan() && dy != 0.0) {
                a.conditions.insert("stalkDx".into(), dx);
                a.conditions.insert("stalkDy".into(), dy);
            }
        }
        a.conditions.insert("stalkObservedX".into(), b.x);
        a.conditions.insert("stalkObservedY".into(), b.y);
        a.last_seen = Field::Value(crate::model::ActorLastSeen { x: b.x, y: b.y });
    }
    let a = actor(r, id).unwrap().clone();
    let Some(last) = a.last_seen.as_ref() else {
        return Ok(false);
    };
    actor_mut_trusted(r, id).unwrap().state = "stalking".into();
    if DIRECTIONS
        .iter()
        .any(|d| doorway(r, a.x as i32 + d.x, a.y as i32 + d.y))
        && a.conditions.get("stalkPeekTick") != Some(&(r.tick - 1.0))
    {
        let wait = a.conditions.get("stalkWait").copied().unwrap_or(0.0) + 1.0;
        actor_mut_trusted(r, id)
            .unwrap()
            .conditions
            .insert("stalkWait".into(), wait);
        if wait <= 3.0 {
            let a = actor(r, id).unwrap().clone();
            if r.player.x >= 0.0 && (id == r.player.id || crate::vision::visible_actor(r, &a)) {
                engine::log(
                    r,
                    format!(
                        "{} waits silently beside the doorway.",
                        if id == r.player.id { "You" } else { &a.name }
                    ),
                    if id == r.player.id { "player" } else { "enemy" },
                );
            }
            crate::events::actor_event(r, &a, "status", "Stalking: waiting", None, None);
            return Ok(true);
        }
        let tick = r.tick;
        let b = actor_mut_trusted(r, id).unwrap();
        b.conditions.shift_remove("stalkWait");
        b.conditions.insert("stalkPeekTick".into(), tick);
        if let Some(step) = engine::creature_path_step(
            r,
            id,
            Point {
                x: last.x as i32,
                y: last.y as i32,
            },
        ) {
            engine::move_actor(r, id, step.x as f64, step.y as f64, false).map_err(|e| e.0)?;
        }
        return Ok(true);
    }
    let sign = |v: f64| if v == 0.0 { 0.0 } else { v.signum() };
    let dx = a
        .conditions
        .get("stalkDx")
        .copied()
        .unwrap_or(sign(last.x - a.x));
    let dy = a
        .conditions
        .get("stalkDy")
        .copied()
        .unwrap_or(sign(last.y - a.y));
    let f = r.floor().unwrap();
    let w = f.width();
    let h = f.height();
    let gx = (last.x - dx * 4.0).clamp(1.0, (w - 2) as f64);
    let gy = (last.y - dy * 4.0).clamp(1.0, (h - 2) as f64);
    let mut choice: Option<(f64, f64, bool, f64, usize)> = None;
    for y in (gy as i32 - 2).max(1)..=(gy as i32 + 2).min(h as i32 - 2) {
        for x in (gx as i32 - 2).max(1)..=(gx as i32 + 2).min(w as i32 - 2) {
            let (x, y) = (x as f64, y as f64);
            let d = distance(x, y, gx, gy);
            if d > 2.0 || !free(r, &a, x, y) {
                continue;
            }
            let mut probe = a.clone();
            probe.x = x;
            probe.y = y;
            if crate::vision::visible_actor(r, &probe) {
                continue;
            }
            let i = y as usize * w + x as usize;
            let hazard = crate::environment::creature_hazard_at(r, &a, i);
            if choice
                .as_ref()
                .is_none_or(|c| (hazard, d, i) < (c.2, c.3, c.4))
            {
                choice = Some((x, y, hazard, d, i));
            }
        }
    }
    if let Some((x, y, ..)) = choice {
        let b = actor_mut_trusted(r, id).unwrap();
        b.conditions.insert("stalkGoalX".into(), x);
        b.conditions.insert("stalkGoalY".into(), y);
        if let Some(step) = engine::creature_path_step(
            r,
            id,
            Point {
                x: x as i32,
                y: y as i32,
            },
        ) {
            engine::move_actor(r, id, step.x as f64, step.y as f64, false).map_err(|e| e.0)?;
        }
    }
    Ok(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stalking_matches_frozen_reference() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/stalking-v48.json")).unwrap();
        for c in cases.as_array().unwrap() {
            let mut r: Run = serde_json::from_value(c["initial"].clone()).unwrap();
            crate::events::begin();
            let result = if c["operation"] == "offscreen" {
                crate::offscreen::tick_inactive_floors(&mut r).unwrap();
                true
            } else {
                let target = c["detected"].as_bool().unwrap().then_some(r.player.id);
                turn(&mut r, 10.0, target).unwrap()
            };
            assert_eq!(result, c["result"].as_bool().unwrap(), "{}", c["kind"]);
            let actual = serde_json::to_value(r).unwrap();
            if !crate::storage::canonical_equal(&actual, &c["expected"]) {
                std::fs::write(
                    "/tmp/stalking-native.json",
                    serde_json::to_vec(&actual).unwrap(),
                )
                .unwrap();
                std::fs::write(
                    "/tmp/stalking-source.json",
                    serde_json::to_vec(&c["expected"]).unwrap(),
                )
                .unwrap();
                panic!("{}", c["kind"]);
            }
            assert!(
                crate::storage::canonical_equal(
                    &serde_json::to_value(crate::events::finish()).unwrap(),
                    &c["events"]
                ),
                "{} events",
                c["kind"]
            );
        }
    }
}
