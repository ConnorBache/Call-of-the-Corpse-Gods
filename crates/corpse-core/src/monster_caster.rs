//! Source dedicated-caster bounded retreat and action ownership.
use crate::{
    Actor, Run,
    engine::{self, actor, condition},
    movement::{DIRECTIONS, Point},
};
pub fn dedicated(a: &Actor) -> bool {
    a.abilities.iter().any(|s| {
        [
            "bloodmancy",
            "witch",
            "saidribee-queen",
            "dryad",
            "horn-bolt",
            "beckon",
            "wall-seal",
            "moon-laser",
        ]
        .iter()
        .any(|t| s == &format!("enemy:beast:{t}"))
    })
}
fn controls(r: &Run, a: &Actor) -> bool {
    !a.dead.value_or(false)
        && a.hp > 0.0
        && a.id != r.player.id
        && !crate::half_here::is_clone(a)
        && dedicated(a)
        && !["sleeping", "neutral"].contains(&a.state.as_str())
        && condition(a, "paralyzed") <= 0.0
        && !(a.state == "ally"
            && a.perfect.value_or(false)
            && a.order.as_ref().is_some_and(|s| s == "p"))
        && !crate::monster_inventor::is_mech(a)
        && !["minotaur", "sphinx", "inventor"]
            .iter()
            .any(|t| a.abilities.iter().any(|s| s == &format!("enemy:beast:{t}")))
}
pub fn can_observe(r: &Run, a: &Actor, b: Option<&Actor>) -> bool {
    b.is_some_and(|b| {
        b.id != a.id
            && !b.dead.value_or(false)
            && b.hp > 0.0
            && crate::dreamwalking::same_plane(r, a, b)
            && crate::vision::line_of_sight(r, a, b)
            && crate::vision::detection_chance(r, a, b) > 0.0
    })
}
/// Eight stable candidates, occupied-cell set and hazard-first distance ranking.
pub fn retreat_step(r: &Run, a: &Actor, b: Option<&Actor>) -> Option<Point> {
    if a.dead.value_or(false) || a.hp <= 0.0 || !can_observe(r, a, b) {
        return None;
    }
    let b = b.unwrap();
    let f = r.floor().unwrap();
    let w = f.width();
    let occupied: std::collections::HashSet<_> = std::iter::once(&r.player)
        .chain(r.actors_at(r.depth))
        .filter(|other| other.id != a.id && !other.dead.value_or(false) && other.hp > 0.0
            && crate::dreamwalking::same_plane(r, a, other))
        .map(|o| o.y as usize * w + o.x as usize)
        .collect();
    let mut hazard = crate::environment::creature_hazard_at(r, a, a.y as usize * w + a.x as usize);
    let mut distance = crate::portals::actor_distance(r, &a, &b);
    let mut result = None;
    for step in DIRECTIONS {
        let mut candidate = a.clone();
        candidate.x += step.x as f64;
        candidate.y += step.y as f64;
        if !crate::travel::can_traverse(r, a, candidate.x, candidate.y)
            || occupied.iter().any(|&at| crate::portals::same_cell(f, (at % w) as f64, (at / w) as f64, candidate.x, candidate.y))
            || !crate::vision::line_of_sight(r, &candidate, b)
        {
            continue;
        }
        let arrivals=crate::portals::step_options(r,a,candidate.x,candidate.y);
        if !arrivals.iter().all(|&(x,y)|engine::actor_passable(r,a,x,y)){continue}
        // Rank the worst possible retreat, not an imaginary chosen branch.
        let h=arrivals.iter().any(|&(x,y)|crate::environment::creature_hazard_at(r,a,y as usize*w+x as usize));
        let d=arrivals.iter().map(|&(x,y)|{candidate.x=x;candidate.y=y;crate::portals::actor_distance(r,&candidate,b)}).fold(f64::INFINITY,f64::min);
        if h < hazard || (h == hazard && d > distance) {
            hazard = h;
            distance = d;
            result = Some(step)
        }
    }
    result
}
fn target(r: &Run, a: &Actor, b: Option<f64>) -> Option<f64> {
    b.or_else(|| {
        if a.state == "ally" {
            crate::ai::nearest_ally_enemy(r, a).map(|b| b.id)
        } else {
            None
        }
    })
}
pub fn retreat(r: &mut Run, id: f64, b: Option<f64>, movement_only: bool) -> Result<bool, String> {
    let a = actor(r, id).ok_or("Caster missing")?.clone();
    if !controls(r, &a) {
        return Ok(false);
    }
    let b = target(r, &a, b);
    if !can_observe(r, &a, b.and_then(|id| actor(r, id))) {
        return Ok(false);
    }
    if let Some(step) = retreat_step(r, &a, b.and_then(|id| actor(r, id))) {
        engine::move_actor(r, id, step.x as f64, step.y as f64, movement_only).map_err(|e| e.0)?;
    }
    Ok(true)
}
pub fn turn(r: &mut Run, id: f64, b: Option<f64>, movement_only: bool) -> Result<bool, String> {
    let a = actor(r, id).ok_or("Caster missing")?.clone();
    if a.dead.value_or(false)
        || a.hp <= 0.0
        || id == r.player.id
        || crate::half_here::is_clone(&a)
        || crate::ai::sleeping(&a)
        || (a.state == "ally"
            && a.perfect.value_or(false)
            && a.order.as_ref().is_some_and(|s| s == "p"))
    {
        return Ok(false);
    }
    let dryad = a.abilities.iter().any(|s| s == "enemy:beast:dryad");
    let b = target(r, &a, b);
    if !controls(r, &a) || !can_observe(r, &a, b.and_then(|id| actor(r, id))) {
        return if dryad && !movement_only {
            crate::monster_spells::heal_others(r, id)
        } else {
            Ok(false)
        };
    }
    let bid = b.unwrap();
    if !movement_only {
        if crate::ai::recover_ranged(r, id) {
            retreat(r, id, Some(bid), true)?;
            return Ok(true);
        }
        if dryad && crate::monster_spells::heal_others(r, id)? {
            return Ok(true);
        }
        if crate::ai::enemy_action_options(r, id, bid, false).map_err(|e| e.0)? {
            return Ok(true);
        }
    }
    if let Some(step) = retreat_step(r, actor(r, id).unwrap(), actor(r, bid)) {
        engine::move_actor(r, id, step.x as f64, step.y as f64, movement_only).map_err(|e| e.0)?;
    } else if !movement_only && !dryad && !a.abilities.iter().any(|s| s == "enemy:beast:witch") {
        let b = actor(r, bid).unwrap();
        if crate::portals::actor_distance(r, &a, &b) <= std::f64::consts::SQRT_2
            && !crate::monster_statuses::try_hemophilia_episode(r, id)?
        {
            engine::attack(r, id, bid, 1.0).map_err(|e| e.0)?;
        }
    }
    Ok(true)
}
