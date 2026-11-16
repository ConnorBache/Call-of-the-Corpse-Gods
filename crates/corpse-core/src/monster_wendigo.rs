//! Frozen monster-ai.ts corpse feeding and injury retreat, using stable actor IDs.
use crate::{
    ActorMonsterState, Run,
    engine::{self, EngineError, actor, actor_mut_trusted},
    movement::Point,
};
pub fn turn(
    r: &mut Run,
    id: f64,
    target: Option<f64>,
    movement_only: bool,
) -> Result<bool, EngineError> {
    let a = actor(r, id).unwrap().clone();
    let threat = target
        .and_then(|id| actor(r, id))
        .map(|a| Point {
            x: a.x as i32,
            y: a.y as i32,
        })
        .or_else(|| {
            a.last_seen.as_ref().map(|a| Point {
                x: a.x as i32,
                y: a.y as i32,
            })
        });
    if let Some(threat) = threat {
        if a.hp <= a.max_hp * 0.35 {
            return crate::monster_vampire::flee(r, id, threat, movement_only);
        }
    }
    let level = a
        .conditions
        .get("monsterWendigoLevel")
        .copied()
        .unwrap_or(1.0);
    if level >= 5.0 {
        return Ok(false);
    }
    let consumed = a
        .monster_state
        .as_ref()
        .and_then(|m| m.consumed_corpse_ids.as_ref());
    let mut nearest: Option<(f64, f64)> = None;
    for b in r.actors_at(r.depth) {
        let d = crate::portals::actor_distance(r, &a, b);
        if b.id != id
            && b.dead.value_or(false)
            && !consumed.is_some_and(|v| v.contains(&b.id))
            && d <= 10.0
            && crate::vision::line_of_sight(r, &a, b)
        {
            if nearest.is_none_or(|(prior, other)| d < prior || (d == prior && b.id < other)) {
                nearest = Some((d, b.id));
            }
        }
    }
    let Some((distance, corpse)) = nearest else {
        return Ok(false);
    };
    if distance <= std::f64::consts::SQRT_2 {
        if movement_only {
            return Ok(true);
        }
        let a = actor_mut_trusted(r, id).unwrap();
        a.monster_state
            .get_or_insert_with(ActorMonsterState::default)
            .consumed_corpse_ids
            .get_or_insert_with(Vec::new)
            .push(corpse);
        a.stats.offense = crate::combat::integer_stat(a.stats.offense) + 5.0;
        a.stats.defense = crate::combat::integer_stat(a.stats.defense) + 5.0;
        a.conditions
            .insert("monsterWendigoLevel".into(), level + 1.0);
        crate::corpses::reset_corpse_revival(actor_mut_trusted(r, corpse).unwrap());
        crate::lifecycle::sync_enemy_health(r, id).map_err(EngineError)?;
        let a = actor(r, id).unwrap().clone();
        if id == r.player.id || crate::vision::visible_actor(r, &a) {
            engine::log(
                r,
                format!(
                    "{} feeds on a corpse and grows to level {}.",
                    if id == r.player.id { "You" } else { &a.name },
                    level + 1.0
                ),
                if id == r.player.id { "player" } else { "enemy" },
            );
        }
        if crate::events::enabled(r) {
            if let Some(remains)=actor(r,corpse) {crate::events::link(r,remains,&a,"E10","droplet-feast",crate::events::Stage::Special);}
            crate::animation_enemy::mark(r,&a,"E10","unfold-pale-red",crate::events::Stage::Special);
        } else {crate::events::actor_event(
            r,
            &a,
            "pulse",
            &format!("Wendigo level {}", level + 1.0),
            None,
            None,
        ); }
        r.floor_mut().unwrap().dirty = true;
        return Ok(true);
    }
    if target.is_none() {
        let b = actor(r, corpse).unwrap();
        let goal = Point {
            x: b.x as i32,
            y: b.y as i32,
        };
        if let Some(d) = engine::creature_path_step(r, id, goal) {
            engine::move_actor(r, id, d.x as f64, d.y as f64, movement_only)?;
        }
        return Ok(true);
    }
    Ok(false)
}
