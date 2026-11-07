//! Exact monster-ai.ts vampire retreat, flight and gap-route decisions.
use crate::{
    Actor, Run,
    engine::{self, EngineError, actor, actor_mut_trusted, condition},
    movement::{DIRECTIONS, Point},
};
fn p(a: &Actor) -> Point {
    Point {
        x: a.x as i32,
        y: a.y as i32,
    }
}
fn distance(r: &Run, actor: &Actor, a: Point, b: Point) -> f64 {
    let mut origin = actor.clone(); origin.x = a.x as f64; origin.y = a.y as f64;
    crate::portals::point_distance(r, &origin, b.x as f64, b.y as f64)
}
pub fn flying(a: &Actor) -> bool {
    a.abilities.iter().any(|s| s == "enemy:beast:vampire")
        && condition(a, "monsterVampireBat") > 0.0
}
fn bat_form(r: &mut Run, id: f64, enabled: bool) {
    let previous = condition(actor(r, id).unwrap(), "monsterVampireBat") > 0.0;
    let a = actor_mut_trusted(r, id).unwrap();
    if enabled {
        a.conditions.insert("monsterVampireBat".into(), 1.0);
    } else {
        a.conditions.shift_remove("monsterVampireBat");
    }
    if previous != enabled {
        let a = actor(r, id).unwrap().clone();
        if id == r.player.id || crate::vision::visible_actor(r, &a) {
            engine::log(
                r,
                format!(
                    "{} {}",
                    if id == r.player.id { "You" } else { &a.name },
                    if enabled {
                        "becomes a red Vampire Bat."
                    } else {
                        "returns to vampire form."
                    }
                ),
                if id == r.player.id { "player" } else { "enemy" },
            );
        }
        crate::events::actor_event(
            r,
            &a,
            "pulse",
            if enabled { "Vampire Bat" } else { "Vampire" },
            None,
            None,
        );
    }
}
pub fn flee(r: &mut Run, id: f64, threat: Point, movement_only: bool) -> Result<bool, EngineError> {
    let a = actor(r, id).unwrap().clone();
    let w = r.floor().unwrap().width();
    let options: Vec<_> = DIRECTIONS
        .iter()
        .copied()
        .enumerate()
        .filter_map(|(order, d)| {
            let q = Point {
                x: a.x as i32 + d.x,
                y: a.y as i32 + d.y,
            };
            if !engine::can_traverse(r, &a, q.x as f64, q.y as f64)
                || std::iter::once(&r.player)
                    .chain(r.actors_at(r.depth))
                    .any(|b| {
                        b.id != id
                            && !b.dead.value_or(false)
                            && b.hp > 0.0
                            && crate::dreamwalking::same_plane(r, &a, b)
                            && crate::portals::same_position(r, b, q.x as f64, q.y as f64)
                    })
            {
                None
            } else {
                let arrivals=crate::portals::step_options(r,&a,q.x as f64,q.y as f64);
                let q=arrivals.iter().map(|&(x,y)|Point{x:x as i32,y:y as i32}).min_by(|a1,a2|distance(r,&a,*a1,threat).total_cmp(&distance(r,&a,*a2,threat))).unwrap();
                let hazard=arrivals.iter().any(|&(x,y)|crate::environment::creature_hazard_at(r,&a,y as usize*w+x as usize));
                Some((order,d,q,hazard))
            }
        })
        .collect();
    let mut choice = options.first();
    for candidate in options.iter().skip(1) {
        let old = choice.unwrap();
        let compare = (candidate.3 as i8 - old.3 as i8) as f64;
        let compare = if compare != 0.0 {
            compare
        } else {
            distance(r, &a, old.2, threat) - distance(r, &a, candidate.2, threat)
        };
        if compare < 0.0 {
            choice = Some(candidate);
        }
    }
    if let Some((_, d, q, _)) = choice {
        if distance(r, &a, *q, threat) >= distance(r, &a, p(&a), threat)
            || crate::environment::creature_hazard_at(r, &a, a.y as usize * w + a.x as usize)
        {
            engine::move_actor(r, id, d.x as f64, d.y as f64, movement_only)?;
        }
    }
    Ok(true)
}
pub fn turn(
    r: &mut Run,
    id: f64,
    target: Option<f64>,
    movement_only: bool,
) -> Result<bool, EngineError> {
    let a = actor(r, id).unwrap().clone();
    let goal = target.and_then(|id| actor(r, id)).map(p).or_else(|| {
        a.last_seen.as_ref().map(|p| Point {
            x: p.x as i32,
            y: p.y as i32,
        })
    });
    if a.hp <= a.max_hp * 0.35 {
        actor_mut_trusted(r, id)
            .unwrap()
            .conditions
            .insert("monsterRetreating".into(), 1.0);
    }
    if a.hp >= a.max_hp * 0.65 {
        actor_mut_trusted(r, id)
            .unwrap()
            .conditions
            .shift_remove("monsterRetreating");
    }
    if condition(actor(r, id).unwrap(), "monsterRetreating") > 0.0 {
        if let Some(threat) = goal {
            bat_form(r, id, true);
            return flee(r, id, threat, movement_only);
        }
    }
    let f = r.floor().unwrap();
    let tile = &f.tiles[a.y as usize * f.width() + a.x as usize];
    if flying(actor(r, id).unwrap()) && !matches!(tile.as_str(), "chasm" | "pit") {
        actor_mut_trusted(r, id)
            .unwrap()
            .conditions
            .shift_remove("monsterVampireBat");
        let grounded = goal.is_none_or(|g| {
            distance(r, &a, p(&a), g) <= std::f64::consts::SQRT_2
                || engine::creature_path_step(r, id, g).is_some()
        });
        actor_mut_trusted(r, id)
            .unwrap()
            .conditions
            .insert("monsterVampireBat".into(), 1.0);
        if grounded {
            bat_form(r, id, false);
        }
    }
    let Some(goal) = goal else { return Ok(false) };
    if distance(r, &a, p(&a), goal) <= std::f64::consts::SQRT_2 {
        return Ok(false);
    }
    if flying(actor(r, id).unwrap()) {
        return if let Some(d) = engine::creature_path_step(r, id, goal) {
            engine::move_actor(r, id, d.x as f64, d.y as f64, movement_only)
        } else {
            Ok(false)
        };
    }
    if engine::creature_path_step(r, id, goal).is_some() {
        return Ok(false);
    }
    actor_mut_trusted(r, id)
        .unwrap()
        .conditions
        .insert("monsterVampireBat".into(), 1.0);
    let route = engine::creature_path_step(r, id, goal);
    actor_mut_trusted(r, id)
        .unwrap()
        .conditions
        .shift_remove("monsterVampireBat");
    let Some(d) = route else { return Ok(false) };
    bat_form(r, id, true);
    engine::move_actor(r, id, d.x as f64, d.y as f64, movement_only)?;
    Ok(true)
}
