//! Exact stair departure/arrival and actor movement hooks.
use crate::{
    Actor, ActorLastSeen, Field, Floor, Run,
    engine::{self, condition},
    movement::{self, Point},
    vision,
};
use std::collections::HashSet;
fn has(a: &Actor, kind: &str) -> bool {
    a.abilities
        .iter()
        .any(|id| id == &format!("enemy:beast:{kind}"))
}
fn ability(a: &Actor, name: &str, active: bool) -> bool {
    let ids = if active { &a.active } else { &a.abilities };
    crate::progression::catalog()
        .iter()
        .any(|d| d.name == name && ids.contains(&d.id))
}
pub fn flying(r: &Run, a: &Actor) -> bool {
    if r.simulation_revision.value_or(29.) >= 52. && a.dead.value_or(false) {return false;}
    if crate::dreamwalking::dream_actor(r,a) {return true;}
    engine::flying(a)
        || crate::revision50_rings::has_effect(r, a, crate::revision50_rings::FLIGHT)
        || (has(a, "vampire") && condition(a, "monsterVampireBat") > 0.0)
        || ability(a, "Ascend", false)
        || ability(a, "Phase Through", true)
        || ((a.id == r.player.id
            || (condition(a, "halfHereClone") == 1.0
                && condition(a, "halfHereOwner") == r.player.id))
            && r.inventory.iter().any(|i| {
                (if crate::revision50::enabled(r) { crate::revision50_inventory::worn(i) }
                    else { i.equipped.as_ref().is_some_and(|s| !s.is_empty()) })
                    && i.enhancements
                        .iter()
                        .any(|e| crate::crafting::craft_name(&e.craft) == Some("Dream"))
            }))
}
pub fn habitat_valid(f: &Floor, a: &Actor, x: f64, y: f64) -> bool {
    if !in_bounds(f, x, y) {
        return false;
    }
    let at = y as usize * f.width() + x as usize;
    let layer = |v: &Field<Vec<f64>>| v.as_ref().map_or(0.0, |v| v[at]);
    if has(a, "ichor-bound") {
        return layer(&f.ichor) > 0.0;
    }
    if has(a, "slime-bound") && layer(&f.slime) == 0.0 {
        return false;
    }
    if has(a, "aquatic")
        && (!["water", "shallow-water"].contains(&f.tiles[at].as_str()) || layer(&f.ichor) > 0.0)
    {
        return false;
    }
    true
}
pub fn habitat_path_allowed(f: &Floor, a: &Actor, at: usize) -> bool {
    (has(a, "slime-bound") && has(a, "slime-trail") && !has(a, "ichor-bound"))
        || habitat_valid(f, a, (at % f.width()) as f64, (at / f.width()) as f64)
}
fn in_bounds(f: &Floor, x: f64, y: f64) -> bool {
    x.is_finite()
        && y.is_finite()
        && x.fract() == 0.0
        && y.fract() == 0.0
        && x >= 0.0
        && y >= 0.0
        && x < f.width() as f64
        && y < f.height() as f64
}
pub fn passable(r: &Run, a: &Actor, x: f64, y: f64) -> bool {
    let Some(f) = r.floor() else { return false };
    if !in_bounds(f, x, y) || crate::revision51_stairs::blocked(r,a,x,y) || (crate::borge::solid_at(f,x,y) || crate::epidite::solid_at(f,x,y)) || engine::bulwark_blocks(r, a, x, y) {
        return false;
    }
    let at = y as usize * f.width() + x as usize;
    let phase = ability(a, "Phase Through", true);
    !crate::revision50_terrain::clay_blocks_run(r,a,x,y)
        && !crate::revision50_control_amulets::obstruction_blocks(r,a,x,y)
        && (!["wall", "tree"].contains(&f.tiles[at].as_str()) || phase)
        && (!f.grates.as_ref().is_some_and(|v| v.contains(&(at as f64))) || phase)
}
pub fn can_traverse(r: &Run, a: &Actor, x: f64, y: f64) -> bool {
    can_traverse_from(
        r,
        a,
        x,
        y,
        Point {
            x: a.x as i32,
            y: a.y as i32,
        },
    )
}
pub fn can_traverse_from(r: &Run, a: &Actor, x: f64, y: f64, from: Point) -> bool {
    let mut origin;
    let source=if r.simulation_revision.value_or(29.)>=37. && (a.x!=from.x as f64 || a.y!=from.y as f64) {
        origin=a.clone();origin.x=from.x as f64;origin.y=from.y as f64;&origin
    }else{a};
    if r.simulation_revision.value_or(29.)>=36. && crate::portals::step_options(r,source,x,y).iter().any(|&(ox,oy)|!passable(r,a,ox,oy)||r.floor().is_some_and(|f|(condition(a,"forcedMove")==0.&&!habitat_valid(f,a,ox,oy))||(f.tiles[oy as usize*f.width()+ox as usize]=="chasm"&&!flying(r,a)))){return false}
    if !passable(r, a, x, y) {
        return false;
    }
    let f = r.floor().unwrap();
    let forced = condition(a, "forcedMove") != 0.0;
    let to = Point {
        x: x as i32,
        y: y as i32,
    };
    (forced
        || movement::diagonal_step_allowed(
            &f.tiles,
            f.width(),
            from,
            to,
            ability(a, "Phase Through", true),
        ))
        && (forced
            || (!has(a, "rooted") && habitat_valid(f, a, a.x, a.y) && habitat_valid(f, a, x, y)))
        && (f.tiles[y as usize * f.width() + x as usize] != "chasm" || flying(r, a))
}
pub fn enemy_moved(r: &mut Run, id: f64) -> Result<(), String> {
    crate::revision50::duel_actor_changed(r,id);
    let a = engine::actor(r, id).ok_or("Moved actor missing")?.clone();
    if crate::revision50_nightmares::is_decoy(&a) { return Ok(()); }
    // Revision31 cosmetic footprints share the existing save/remember blood layer.
    // O(1) per move after at most one O(V) allocation; no RNG, dirty flag or rules.
    let blood_trail = r.simulation_revision.value_or(29.0) >= 31.0
        && !a.dead.value_or(false)
        && crate::enemy_traits::definition(&a).is_some_and(|d| matches!(d.id.as_str(),
            "hemogoblin" | "hemogoblin-bloodmancer" | "hemogoblin-inventor"));
    let animate=crate::events::enabled(r);
    let mut slimed=Vec::new();
    let f = r.floor_mut().ok_or("Moved floor missing")?;
    let w = f.width();
    let at = a.y as usize * w + a.x as usize;
    if blood_trail && f.tiles[at] == "floor" {
        let n = f.tiles.len();
        f.blood.get_or_insert_with(|| vec![false; n])[at] = true;
    }
    if has(&a, "yeti")
        && !a.dead.value_or(false)
        && !["wall", "tree", "chasm", "pit"].contains(&f.tiles[at].as_str())
    {
        let n = f.tiles.len();
        f.snow.get_or_insert_with(|| vec![false; n])[at] = true;
        f.dirty = true;
    }
    if has(&a, "slime-trail") && condition(&a, "forcedMove") == 0.0 {
        let n = f.tiles.len();
        let mut slime = f.slime.value_or(vec![0.0; n]);
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (x, y) = (a.x + dx as f64, a.y + dy as f64);
                if in_bounds(f, x, y) {
                    let i = y as usize * w + x as usize;
                    if !["wall", "tree", "chasm", "pit", "water", "shallow-water"]
                        .contains(&f.tiles[i].as_str())
                    {
                        if animate && slime[i]!=1.0 {slimed.push(crate::events::EventPoint{x,y});}
                        slime[i] = 1.0
                    }
                }
            }
        }
        f.slime = slime.into();
        f.dirty = true;
    }
    let submerged = has(&a, "submerge")
        && f.tiles[at] == "water"
        && !f.ichor.as_ref().is_some_and(|g| g[at] != 0.0);
    crate::events::cue(r,&a,"E31","wave-green",crate::events::Stage::Special,&slimed);
    let a = engine::actor_mut_trusted(r, id).unwrap();
    if submerged {
        a.conditions.insert("underwater".into(), 1.0);
        a.conditions.insert("submergedStealth".into(), 6.0);
    } else {
        a.conditions.shift_remove("underwater");
        a.conditions.shift_remove("submergedStealth");
    }
    crate::terrain_runtime::snow_contact(r, id);
    crate::zillennia_ground::on_arrival(r, id);
    crate::environment::refresh_burning_from_fire(r, id)
}
pub fn clear_floor_chase(a: &mut Actor) {
    for k in [
        "chaseDepth",
        "chaseExitX",
        "chaseExitY",
        "chaseLandingX",
        "chaseLandingY",
        "chaseChasm",
    ] {
        a.conditions.shift_remove(k);
    }
}
/// A deliberate downward chase is distinct from losing flight over a chasm.
/// A saved marker distinguishes a closed/restored chasm from a stair departure.
/// No timer or RNG draw; the condition map remains the serialized boundary.
pub(crate) fn flying_chasm_chase(r: &Run, a: &Actor) -> bool {
    r.simulation_revision.value_or(29.) >= 52.
        && condition(a,"chaseChasm") == 1.
        && a.state == "hunting" && !a.dead.value_or(false) && a.hp > 0.
        && a.conditions.get("chaseDepth") == Some(&crate::fast_mode::below(r, r.depth))
        && a.conditions.contains_key("chaseLandingX")
        && a.conditions.contains_key("chaseLandingY")
        && r.floor().is_some_and(|f| a.conditions.get("chaseExitX")
            .zip(a.conditions.get("chaseExitY"))
            .and_then(|(&x,&y)| crate::portals::cell(f,x,y))
            .is_some_and(|at| f.tiles[at] == "chasm"))
        && flying(r,a)
}
/// Enroll only existing player hunters that can perceive this chasm departure.
/// O(A) candidate storage, O(A*(K+D+d+F)) time plus first index reconciliation
/// and COW writes. D includes cold visibility/lighting fields; see ledger.
fn queue_flying_chasm_followers(r: &mut Run, next: f64, landing: Point) {
    if r.simulation_revision.value_or(29.) < 52. || next != crate::fast_mode::below(r, r.depth)
        || r.floor().and_then(|f| crate::portals::cell(f,r.player.x,r.player.y)
            .map(|at| &f.tiles[at])).is_none_or(|tile| tile != "chasm") { return; }
    let ids: Vec<_> = r.actors_at(r.depth).iter().filter(|a|
        a.state == "hunting" && !a.dead.value_or(false) && a.hp > 0.
        && a.conditions.get("targetId") == Some(&r.player.id) && flying(r,a)
        && !crate::revision50_nightmares::is_decoy(a)).map(|a| a.id).collect();
    for id in ids {
        let a = engine::actor(r,id).unwrap();
        let known = a.last_seen.as_ref().is_some_and(|p|p.x == r.player.x && p.y == r.player.y);
        let chance = vision::detection_chance(r,a,&r.player);
        if chance <= 0. || (!known && chance < 100. && crate::rng::random(r)*100. >= chance) { continue; }
        let (x,y) = (r.player.x,r.player.y);
        let a = engine::actor_mut_trusted(r,id).unwrap();
        clear_floor_chase(a);
        a.last_seen = ActorLastSeen{x,y}.into();
        for (key,value) in [("chaseExitX",x),("chaseExitY",y),("chaseDepth",next),
            ("chaseLandingX",landing.x as f64),("chaseLandingY",landing.y as f64),
            ("chaseChasm",1.),("seesPlayer",0.),("monsterUnseenTurns",0.)] {
            a.conditions.insert(key.into(),value);
        }
        r.floor_mut().unwrap().dirty = true;
    }
}
pub fn dismiss_half_here(r: &mut Run) {
    crate::half_here::dismiss(r)
}
const ALLY_FIELDS: [&str; 10] = [
    "allyStairSource",
    "allyStairDepth",
    "allyStairExitX",
    "allyStairExitY",
    "allyStairLandingX",
    "allyStairLandingY",
    "allyStairDistance",
    "allyStairSpeed",
    "allyStairRemaining",
    "allyStairLastTick",
];
pub fn clear_ally_stair_follow(a: &mut Actor) {
    for k in ALLY_FIELDS {
        a.conditions.shift_remove(k);
    }
    a.conditions.shift_remove("allyStairHeldTick");
}
pub fn has_ally_stair_follow(a: &Actor) -> bool {
    condition(a, "allyStairRemaining") > 0.0
}
pub fn ally_stair_movement_speed(r: &Run, a: &Actor) -> f64 {
    if condition(a, "slowed") > 0.0
        || a.abilities
            .iter()
            .any(|id| id == "enemy:pale:slow" || id == "enemy:beast:slow")
    {
        200.0
    } else if condition(a, "hasted") > 0.0
        || has(a, "fast")
        || (has(a, "vampire") && condition(a, "monsterVampireBat") > 0.0)
        || ability(a, "Rushing Torrent", true)
        || (ability(a, "Sleep Walk", false) && a.energy == 0.0)
        || (ability(a, "Step Through Shadows", false)
            && !r.actors_at(r.depth).iter().any(|b| {
                !b.dead.value_or(false)
                    && b.hp > 0.0
                    && b.state != "ally"
                    && b.state != "neutral"
                    && condition(b, "seesPlayer") == 1.0
                    && vision::detection_chance(r, b, a) > 0.0
            }))
    {
        50.0
    } else {
        100.0
    }
}
fn route_field(r: &Run, a: &Actor, target: Point) -> movement::DistanceField {
    let f = r.floor().unwrap();
    let phase = ability(a, "Phase Through", true);
    let allowed: Vec<_> = (0..f.tiles.len())
        .map(|at| {
            !crate::revision50_terrain::clay_blocks_run(r,a,(at%f.width())as f64,(at/f.width())as f64)
                && !crate::revision50_control_amulets::obstruction_blocks(r,a,(at%f.width())as f64,(at/f.width())as f64)
                && !crate::revision51_stairs::blocked_cell(r,a,f,at)
                && habitat_path_allowed(f, a, at)
                && (phase || !f.grates.as_ref().is_some_and(|g| g.contains(&(at as f64))))
                && !engine::bulwark_blocks(r, a, (at % f.width()) as f64, (at / f.width()) as f64)
        })
        .collect();
    let harmful: Vec<_> = (0..f.tiles.len())
        .map(|at| crate::environment::creature_hazard_at(r, a, at))
        .collect();
    movement::distance_field_ordered(
        &f.tiles,
        f.width(),
        target,
        phase,
        flying(r, a),
        Some(&allowed),
        Some(&harmful),
        &if crate::portals::enabled(r) {crate::portals::links(f)} else {vec![]},
        &if r.simulation_revision.value_or(29.)>=37. {crate::portals::ordered_pairs(f)} else {vec![]},
    )
}
fn eligible_departure(r: &Run, a: &Actor) -> bool {
    if crate::revision50_nightmares::is_decoy(a) { return false; }
    let f = r.floor().unwrap();
    let at = a.y as usize * f.width() + a.x as usize;
    a.state == "ally"
        && !a.dead.value_or(false)
        && a.hp > 0.0
        && condition(a, "halfHereClone") != 1.0
        && condition(a, "paralyzed") <= 0.0
        && !has(a, "rooted")
        && !has(a, "aquatic")
        && habitat_valid(f, a, a.x, a.y)
        && !["wall", "tree"].contains(&f.tiles[at].as_str())
        && !f.grates.as_ref().is_some_and(|g| g.contains(&(at as f64)))
}
pub fn queue_ally_stair_followers(r: &mut Run, destination: f64, landing: Point) {
    let source = r.depth;
    let exit = Point {
        x: r.player.x as i32,
        y: r.player.y as i32,
    };
    let ids: Vec<_> = r
        .actors_at(source)
        .iter()
        .filter(|a| eligible_departure(r, a))
        .map(|a| a.id)
        .collect();
    for id in ids {
        let mut a = engine::actor(r, id).unwrap().clone();
        clear_ally_stair_follow(&mut a);
        let field = route_field(r, &a, exit);
        let at = a.y as usize * r.floor().unwrap().width() + a.x as usize;
        let distance = if field.steps[at] < 0 {
            30000.0
        } else {
            field.steps[at] as f64
        };
        let speed = ally_stair_movement_speed(r, &a);
        for (k, v) in [
            ("allyStairSource", source),
            ("allyStairDepth", destination),
            ("allyStairExitX", exit.x as f64),
            ("allyStairExitY", exit.y as f64),
            ("allyStairLandingX", landing.x as f64),
            ("allyStairLandingY", landing.y as f64),
            ("allyStairDistance", distance),
            ("allyStairSpeed", speed),
            (
                "allyStairRemaining",
                ((distance * speed / 100.0).floor() + 1.0).clamp(1.0, 150.0),
            ),
            ("allyStairLastTick", r.tick),
        ] {
            a.conditions.insert(k.into(), v);
        }
        crate::ai::wake(&mut a, true);
        *engine::actor_mut_trusted(r, id).unwrap() = a;
        r.floor_mut().unwrap().dirty = true;
    }
}
fn valid_queue(r: &Run, source: f64, a: &Actor) -> bool {
    let Some(from) = r.floors.iter().find(|f| f.depth == source) else {
        return false;
    };
    let Some(target) = r
        .floors
        .iter()
        .find(|f| f.depth == condition(a, "allyStairDepth"))
    else {
        return false;
    };
    condition(a, "allyStairSource") == source
        && crate::fast_mode::adjacent(r, source, condition(a, "allyStairDepth"))
        && ALLY_FIELDS.iter().all(|k| {
            a.conditions
                .get(*k)
                .is_some_and(|v| v.is_finite() && v.fract() == 0.0)
        })
        && [50.0, 100.0, 200.0].contains(&condition(a, "allyStairSpeed"))
        && condition(a, "allyStairDistance") >= 0.0
        && (1.0..=150.0).contains(&condition(a, "allyStairRemaining"))
        && condition(a, "allyStairLastTick") <= r.tick
        && in_bounds(
            from,
            condition(a, "allyStairExitX"),
            condition(a, "allyStairExitY"),
        )
        && in_bounds(
            target,
            condition(a, "allyStairLandingX"),
            condition(a, "allyStairLandingY"),
        )
}
pub fn arrive_ally_at_stair(
    r: &mut Run,
    id: f64,
    source: f64,
    origin: Point,
) -> Result<bool, String> {
    let Some(a) = r.actors_at(source).iter().find(|a| a.id == id).cloned() else {
        return Ok(false);
    };
    let f = r.floor().unwrap();
    let w = f.width();
    let mut incoming = a.clone();
    incoming.x = origin.x as f64;
    incoming.y = origin.y as f64;
    let at = origin.y as usize * w + origin.x as usize;
    if crate::revision50_nightmares::is_decoy(&a) || a.dead.value_or(false)
        || a.hp <= 0.0
        || a.state != "ally"
        || has(&a, "rooted")
        || has(&a, "aquatic")
        || condition(&a, "paralyzed") > 0.0
        || ["chasm", "pit"].contains(&f.tiles[at].as_str())
        || !passable(r, &incoming, incoming.x, incoming.y)
    {
        return Ok(false);
    }
    let occupied: HashSet<_> = std::iter::once(&r.player)
        .chain(r.actors_at(r.depth))
        .filter(|a| !a.dead.value_or(false))
        .map(|a| a.y as usize * w + a.x as usize)
        .collect();
    let landing = std::iter::once(Point { x: 0, y: 0 })
        .chain(movement::DIRECTIONS)
        .map(|d| Point {
            x: origin.x + d.x,
            y: origin.y + d.y,
        })
        .find(|p| {
            if !in_bounds(f, p.x as f64, p.y as f64) {
                return false;
            }
            let at = p.y as usize * w + p.x as usize;
            let mut candidate = a.clone();
            candidate.x = p.x as f64;
            candidate.y = p.y as f64;
            !occupied_cell(f,crate::portals::enabled(r),&occupied,at)
                && passable(r, &candidate, candidate.x, candidate.y)
                && movement::diagonal_step_allowed(
                    &f.tiles,
                    w,
                    origin,
                    *p,
                    ability(&a, "Phase Through", true),
                )
                && habitat_path_allowed(f, &candidate, at)
                && f.tiles[at] != "pit"
                && (f.tiles[at] != "chasm" || flying(r, &candidate))
                && !crate::environment::creature_hazard_at(r, &candidate, at)
        });
    let Some(p) = landing else { return Ok(false) };
    r.actors_at_mut(source).retain(|a| a.id != id);
    let mut a = a;
    a.x = p.x as f64;
    a.y = p.y as f64;
    r.actors_at_mut(r.depth).push(a);
    crate::environment::open_door(r.floor_mut().unwrap(), p.y as usize * w + p.x as usize);
    enemy_moved(r, id)?;
    let tick = r.tick;
    let a = engine::actor_mut_trusted(r, id).unwrap();
    crate::ai::wake(a, true);
    a.conditions.insert("offscreenActionTick".into(), tick);
    for f in &mut r.floors {
        if f.depth == source || f.depth == r.depth {
            crate::mutation_journal::make_mut(f).dirty = true
        }
    }
    Ok(true)
}
pub fn tick_ally_stair_followers(r: &mut Run) -> Result<(), String> {
    let mut depths: Vec<_> = r
        .actors
        .keys()
        .filter_map(|k| k.parse::<u32>().ok())
        .collect();
    depths.sort();
    for d in depths {
        let source = d as f64;
        let mut ids: Vec<_> = r
            .actors_at(source)
            .iter()
            .filter(|a| has_ally_stair_follow(a))
            .map(|a| a.id)
            .collect();
        ids.sort_by(f64::total_cmp);
        for id in ids {
            let a = r
                .actors_at(source)
                .iter()
                .find(|a| a.id == id)
                .unwrap()
                .clone();
            if crate::revision50_nightmares::is_decoy(&a) || !valid_queue(r, source, &a)
                || a.dead.value_or(false)
                || a.hp <= 0.0
                || a.state != "ally"
            {
                clear_ally_stair_follow(
                    r.actors_slice_mut(source)
                        .iter_mut()
                        .find(|a| a.id == id)
                        .unwrap(),
                );
                continue;
            }
            if condition(&a, "allyStairDepth") != r.depth
                || source == r.depth
                || condition(&a, "allyStairLastTick") == r.tick
            {
                continue;
            }
            let tick = r.tick;
            let actor = r
                .actors_slice_mut(source)
                .iter_mut()
                .find(|a| a.id == id)
                .unwrap();
            actor.conditions.insert("allyStairLastTick".into(), tick);
            if condition(&a, "paralyzed") > 0.0 || condition(&a, "allyStairHeldTick") == tick {
                continue;
            }
            if condition(&a, "allyStairRemaining") > 1.0 {
                *actor.conditions.get_mut("allyStairRemaining").unwrap() -= 1.0;
            } else if arrive_ally_at_stair(
                r,
                id,
                source,
                Point {
                    x: condition(&a, "allyStairLandingX") as i32,
                    y: condition(&a, "allyStairLandingY") as i32,
                },
            )? {
                clear_ally_stair_follow(engine::actor_mut_trusted(r, id).unwrap());
            }
        }
    }
    Ok(())
}
pub fn restore_returning_ally_stair_followers(r: &mut Run) -> Result<(), String> {
    let w = r.floor().unwrap().width();
    let mut occupied: HashSet<_> = std::iter::once(&r.player)
        .chain(r.actors_at(r.depth))
        .filter(|a| !a.dead.value_or(false))
        .map(|a| a.y as usize * w + a.x as usize)
        .collect();
    let mut ids: Vec<_> = r
        .actors_at(r.depth)
        .iter()
        .filter(|a| has_ally_stair_follow(a))
        .map(|a| a.id)
        .collect();
    ids.sort_by(f64::total_cmp);
    for id in ids {
        let a = engine::actor(r, id).unwrap().clone();
        if valid_queue(r, r.depth, &a) && !a.dead.value_or(false) && a.hp > 0.0 && a.state == "ally"
        {
            let field = route_field(
                r,
                &a,
                Point {
                    x: condition(&a, "allyStairExitX") as i32,
                    y: condition(&a, "allyStairExitY") as i32,
                },
            );
            let distance = field.steps[a.y as usize * w + a.x as usize] as f64;
            let speed = condition(&a, "allyStairSpeed");
            let initial = ((condition(&a, "allyStairDistance") * speed / 100.0).floor() + 1.0)
                .clamp(1.0, 150.0);
            let credited =
                condition(&a, "allyStairDistance").min((initial * 100.0 / speed).floor());
            let progress = if condition(&a, "paralyzed") > 0.0
                || condition(&a, "allyStairHeldTick") == r.tick
            {
                0.0
            } else {
                distance
                    .min(credited - (condition(&a, "allyStairRemaining") * 100.0 / speed).floor())
                    .max(0.0)
            };
            occupied.remove(&(a.y as usize * w + a.x as usize));
            for _ in 0..progress as usize {
                let a = engine::actor(r, id).unwrap();
                let Some(step) = movement::next_step(
                    &field,
                    &r.floor().unwrap().tiles,
                    Point {
                        x: a.x as i32,
                        y: a.y as i32,
                    },
                ) else {
                    break;
                };
                if crate::portals::enabled(r) {
                    // Portal entry is an ordinary local step. Central movement resolves
                    // its single arrival, alias occupancy, diagonal and contact hooks.
                    if !engine::move_actor(r,id,step.x as f64,step.y as f64,true).map_err(|e|e.to_string())? {break;}
                    continue;
                }
                let (x, y) = (a.x + step.x as f64, a.y + step.y as f64);
                if !can_traverse(r, a, x, y) || occupied.contains(&(y as usize * w + x as usize)) {
                    break;
                }
                let a = engine::actor_mut_trusted(r, id).unwrap();
                a.x = x;
                a.y = y;
                crate::environment::open_door(r.floor_mut().unwrap(), y as usize * w + x as usize);
                enemy_moved(r, id)?;
            }
            let a = engine::actor_mut_trusted(r, id).unwrap();
            occupied.insert(a.y as usize * w + a.x as usize);
            crate::ai::wake(a, true);
            r.floor_mut().unwrap().dirty = true;
        }
        clear_ally_stair_follow(engine::actor_mut_trusted(r, id).unwrap());
    }
    Ok(())
}
/// Existing-state travel API. Call through transactional engine dispatch for rollback on unsupported hooks.
pub fn travel(r: &mut Run, delta: f64, falling: bool, vertical: bool) -> Result<bool, String> {
    let next = crate::fast_mode::destination(r, r.depth, delta);
    travel_inner(r, next, falling, vertical, false)
}
/// Deliberate player chasm entry; revision43 charges 25% maximum HP once.
/// Uses the existing travel/landing pipeline and water exception. O(1) extra
/// bookkeeping; total time/space remain those of travel (generation, actors,
/// placement and arrival hooks). Forced/equipment/potion falls use `travel`.
pub fn jump_chasm(r: &mut Run) -> Result<bool, String> {
    if r.floor().is_none_or(|f| f.tiles.get(r.player.y as usize*f.width()+r.player.x as usize).is_none_or(|tile|tile!="chasm")) {
        return Ok(false);
    }
    let next = crate::fast_mode::below(r, r.depth);
    travel_inner(r, next, true, false, true)
}
/// Explicit effective-depth teleport; unlike physical travel it never skips again.
pub(crate) fn travel_to(r: &mut Run, next: f64, falling: bool, vertical: bool) -> Result<bool, String> {
    if !crate::fast_mode::playable(r, next) { return Ok(false); }
    travel_inner(r, next, falling, vertical, false)
}
fn travel_inner(r: &mut Run, next: f64, falling: bool, vertical: bool, voluntary_chasm: bool) -> Result<bool, String> {
    if crate::dreamwalking::active(r) {engine::log(r,crate::dreamwalking::STAIRS_MESSAGE,"player");return Ok(false)}
    if falling && crate::finale::intercept_fall(r)? { return Ok(true); }
    let delta = next - r.depth;
    if !crate::fast_mode::playable(r, next) {
        return Ok(false);
    }
    let to = crate::fast_mode::arrival_target(r, next as u32);
    crate::startup::ensure_floors(r, to)?;
    crate::offscreen::catch_up_floor(r, next)?;
    let destination = r.floors.iter().find(|f| f.depth == next).ok_or("Travel destination was not generated")?;
    let w = destination.width();
    let mut desired = if falling || vertical {
        if in_bounds(destination, r.player.x, r.player.y) {
            Some(r.player.y as usize * w + r.player.x as usize)
        } else {
            None
        }
    } else {
        destination
            .tiles
            .iter()
            .position(|t| t == if delta > 0.0 { "up" } else { "down" })
    };
    if desired.is_none_or(|at| {
        ["wall", "tree"].contains(&destination.tiles[at].as_str())
            || (destination.tiles[at] == "chasm" && !(vertical && !falling && flying(r, &r.player)))
    }) {
        desired = destination.tiles.iter().position(|t| t == "floor");
    }
    let Some(pos) = desired else {
        engine::log(r, "Destination has no safe landing.", "player");
        return Ok(false);
    };
    if r.actors_at(next)
        .iter()
        .any(|a| !a.dead.value_or(false) && (a.y as usize * w + a.x as usize == pos
            || (crate::portals::enabled(r) && crate::portals::aliases(destination,pos).contains(&(a.y as usize*w+a.x as usize)))))
    {
        engine::log(
            r,
            "Destination is occupied; transition blocked pending a free landing.",
            "player",
        );
        return Ok(false);
    }
    let source = r.depth;
    let f = r.floor().unwrap();
    let stair_exit = !falling
        && !vertical
        && crate::fast_mode::adjacent(r, r.depth, next)
        && f.tiles[r.player.y as usize * f.width() + r.player.x as usize]
            == if delta > 0.0 { "down" } else { "up" };
    let flying_chasm_exit = r.simulation_revision.value_or(29.) >= 52. && falling
        && next == crate::fast_mode::below(r, r.depth) && f.tiles[r.player.y as usize*f.width()+r.player.x as usize] == "chasm";
    if flying_chasm_exit {
        queue_flying_chasm_followers(r,next,Point{x:(pos%w) as i32,y:(pos/w) as i32});
    }
    let ids: Vec<_> = r
        .actors_at(source)
        .iter()
        .filter(|a| !a.dead.value_or(false) && a.state == "hunting")
        .map(|a| a.id)
        .collect();
    for id in ids {
        let a = engine::actor(r, id).unwrap().clone();
        // The R52 flying cohort above owns its stricter target/awareness gate
        // and exact landing. Keep legacy/nonflying chase enrollment unchanged.
        if flying_chasm_exit && flying(r,&a) { continue; }
        let known = a
            .last_seen
            .as_ref()
            .is_some_and(|p| p.x == r.player.x && p.y == r.player.y);
        if stair_exit {
            if condition(&a, "targetId") != r.player.id {
                continue;
            }
            let chance = vision::detection_chance(r, &a, &r.player);
            if chance <= 0.0
                || (!known && chance < 100.0 && crate::rng::random(r) * 100.0 >= chance)
            {
                continue;
            }
        } else if !known {
            continue;
        }
        let (x, y) = (r.player.x, r.player.y);
        let a = engine::actor_mut_trusted(r, id).unwrap();
        if stair_exit {
            a.last_seen = ActorLastSeen { x, y }.into();
            a.conditions.insert("monsterUnseenTurns".into(), 0.0);
        }
        clear_floor_chase(a);
        for (k, v) in [
            ("chaseExitX", x),
            ("chaseExitY", y),
            ("chaseDepth", next),
            ("seesPlayer", 0.0),
        ] {
            a.conditions.insert(k.into(), v);
        }
        if stair_exit {
            a.conditions
                .insert("chaseLandingX".into(), (pos % w) as f64);
            a.conditions
                .insert("chaseLandingY".into(), (pos / w) as f64);
        }
        r.floor_mut().unwrap().dirty = true;
    }
    dismiss_half_here(r);
    if stair_exit {
        queue_ally_stair_followers(
            r,
            next,
            Point {
                x: (pos % w) as i32,
                y: (pos / w) as i32,
            },
        );
    }
    let allies: Vec<_> = if stair_exit {
        vec![]
    } else {
        r.actors_at(source)
            .iter()
            .filter(|a| a.state == "ally" && !a.dead.value_or(false) && !crate::revision50_nightmares::is_decoy(a))
            .cloned()
            .collect()
    };
    let travelling: HashSet<_> = allies.iter().map(|a| a.id.to_bits()).collect();
    r.actors_at_mut(source)
        .retain(|a| !travelling.contains(&a.id.to_bits()));
    for g in r.ground_at_mut(source) {
        g.auto_pickup_blocked = Field::Missing;
    }
    r.depth = next;
    if next>source {crate::events::player_descent(stair_exit,falling);}
    crate::startup::ensure_floors(r, to)?;
    r.player.x = (pos % w) as f64;
    r.player.y = (pos / w) as f64;
    crate::revision50::duel_actor_changed(r,r.player.id);
    crate::environment::refresh_burning_from_fire(r, r.player.id)?;
    let landing_water = r.floor().unwrap().tiles[pos] == "water";
    if falling && !landing_water && !crate::future_floor::enabled(r) {
        let exact_chasm_cost = voluntary_chasm && r.simulation_revision.value_or(29.0)>=43.0;
        engine::damage_environment(r, r.player.id, r.player.max_hp * 0.25, None, "fall", exact_chasm_cost)
            .map_err(|e| e.0)?;
    }
    for mut a in allies {
        if falling
            && !landing_water
            && !flying(r, &a)
            && a.hp <= crate::combat::tuned_damage_for_run(r, a.max_hp * 0.25, Some("fall"))
        {
            r.actors_at_mut(source).push(a);
            continue;
        }
        let f = r.floor().unwrap();
        let mut landing = None;
        'rings: for radius in 1..w.max(f.height()) as i32 {
            for dy in -radius..=radius {
                let mut dx = -radius;
                while dx <= radius {
                    let (x, y) = (r.player.x + dx as f64, r.player.y + dy as f64);
                    dx += if dy.abs() == radius { 1 } else { 2 * radius };
                    if !in_bounds(f, x, y) {
                        continue;
                    }
                    let at = y as usize * w + x as usize;
                    if ["wall", "tree", "chasm"].contains(&f.tiles[at].as_str())
                        || ((!has(&a, "slime-trail") || has(&a, "ichor-bound"))
                            && !habitat_valid(f, &a, x, y))
                        || f.grates.as_ref().is_some_and(|g| g.contains(&(at as f64)))
                        || r.actors_at(next)
                            .iter()
                            .any(|b| !b.dead.value_or(false) && (b.x == x && b.y == y
                                || (crate::portals::enabled(r) && crate::portals::same_cell(f,b.x,b.y,x,y))))
                    {
                        continue;
                    }
                    landing = Some(at);
                    break 'rings;
                }
            }
        }
        let Some(at) = landing else {
            let name = crate::actor_presentation::source_name(r, &a).to_string();
            r.actors_at_mut(source).push(a);
            engine::log(r, format!("{name} cannot find a safe landing."), "enemy");
            continue;
        };
        a.x = (at % w) as f64;
        a.y = (at / w) as f64;
        let id = a.id;
        let flight = flying(r, &a);
        r.actors_at_mut(next).push(a);
        enemy_moved(r, id)?;
        if falling && !flight && !landing_water {
            let hp = engine::actor(r, id).unwrap().max_hp;
            engine::damage_environment(r, id, hp * 0.25, None, "fall", false).map_err(|e| e.0)?;
            if let Some(history) = r.resource_history.as_mut() {
                history.actors.shift_remove(&id.to_string());
            }
        }
    }
    r.rally = None;
    restore_returning_ally_stair_followers(r)?;
    crate::future_floor::capture_entry(r)?;
    if falling && !landing_water && crate::future_floor::enabled(r) {
        let exact_chasm_cost = voluntary_chasm && r.simulation_revision.value_or(29.0)>=43.0;
        engine::damage_environment(r, r.player.id, r.player.max_hp * 0.25, None, "fall", exact_chasm_cost).map_err(|e| e.0)?;
    }
    engine::monster_floor_entered(r);
    engine::monster_prepare_player_turn(r).map_err(|e| e.0)?;
    let domain = r.floor().unwrap().domain.clone();
    engine::log(r, format!("Floor {}: {domain}.", crate::fast_mode::display_depth(r, next)), "environment");
    Ok(true)
}

pub fn nearest_fall_landing(
    f: &Floor,
    x: f64,
    y: f64,
    occupied: &HashSet<usize>,
    eligible: impl Fn(usize) -> bool,
) -> Option<Point> {
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    let mut selected = None;
    let mut nearest = f64::INFINITY;
    for at in 0..f.tiles.len() {
        if occupied.contains(&at) || !eligible(at) {
            continue;
        }
        let distance =
            ((at % f.width()) as f64 - x).powi(2) + ((at / f.width()) as f64 - y).powi(2);
        if distance < nearest {
            selected = Some(at);
            nearest = distance
        }
    }
    selected.map(|at| Point {
        x: (at % f.width()) as i32,
        y: (at / f.width()) as i32,
    })
}
fn depth_view(r: &Run, depth: f64) -> Run {
    let mut head = r.clone();
    head.depth = depth;
    if r.depth != depth {
        head.player.id = -1.0;
        head.player.x = -999.0;
        head.player.y = -999.0;
        head.player.hp = 0.0;
        head.player.dead = true.into();
        head.player.abilities.clear();
        head.player.active.clear();
        head.player.conditions.clear();
        head.inventory.clear();
        head.log.clear();
        head.log_kinds = vec![].into();
        head.log_archive = Field::Missing;
        head.history.clear();
        head.replay.clear();
    }
    head
}
fn copy_depth_changes(r: &mut Run, head: Run) {
    r.floors = head.floors;
    r.actors.replace_from(head.actors);
    r.ground = head.ground;
    r.rng = head.rng;
    r.next_id = head.next_id;
    r.metrics = head.metrics;
    r.exp = head.exp;
    r.run_stats = head.run_stats;
    r.collapse = head.collapse;
    crate::revision50::settle_duel_targets(r);
}
/// Forced cohorts reserve all landings before transfer; IDs order the cohort.
/// Shared endpoints take impact on the source owner before being dismissed.
pub fn fall_actors_one_level(
    r: &mut Run,
    source_depth: f64,
    candidates: &[f64],
    instigator: Option<f64>,
) -> Result<(), String> {
    crate::finale::intercept_actor_falls(r, source_depth, candidates)?;
    if r.depth == source_depth && candidates.contains(&r.player.id)
        && !flying(r, &r.player) && crate::finale::intercept_fall(r)? { return Ok(()); }
    let Some(source) = r.floors.iter().find(|f| f.depth == source_depth) else {
        return Ok(());
    };
    let source_view = depth_view(r, source_depth);
    let w = source.width();
    let mut seen = HashSet::new();
    let mut cohort = Vec::new();
    for id in candidates {
        if !seen.insert(id.to_bits()) {
            continue;
        }
        let a = if *id == r.player.id && r.depth == source_depth {
            Some(&r.player)
        } else {
            r.actors_at(source_depth).iter().find(|a| a.id == *id)
        };
        if let Some(a) = a {
            if !crate::revision50_nightmares::is_decoy(a) && !a.dead.value_or(false)
                && a.hp > 0.0
                && !flying(&source_view, a)
                && in_bounds(source, a.x, a.y)
            {
                cohort.push(a.clone());
            }
        }
    }
    let player = cohort.iter().any(|a| a.id == r.player.id);
    let player_pit = cohort
        .iter()
        .any(|a| a.id == r.player.id && source.tiles[a.y as usize * w + a.x as usize] == "pit");
    let endpoint_pit = cohort.iter().any(|a| {
        crate::half_here::is_clone(a) && source.tiles[a.y as usize * w + a.x as usize] == "pit"
    });
    let mut falling: Vec<_> = cohort
        .into_iter()
        .filter(|a| source.tiles[a.y as usize * w + a.x as usize] == "chasm")
        .collect();
    falling.sort_by(|a, b| a.id.total_cmp(&b.id));
    if player_pit && r.status == "playing" {
        r.status = "won".into();
        engine::log(
            r,
            "Without flight, you fall into the bottomless pit. Victory.",
            "player",
        );
    }
    if endpoint_pit {
        crate::half_here::dismiss(r);
    }
    let next = crate::fast_mode::below(r, source_depth);
    if falling.is_empty() || !crate::fast_mode::playable(r, next) {
        return Ok(());
    }
    // A nonplayer fall only needs its physical destination, not a new player
    // prefetch horizon. Successful player transfer extends the horizon below.
    let to = if r.simulation_revision.value_or(29.) >= 52. { next as u32 }
        else { crate::startup::arrival_generation_target(r.simulation_revision.value_or(29.), next as u32) };
    crate::startup::ensure_floors(r, to)?;
    crate::offscreen::catch_up_floor(r, next)?;
    let destination = r.floors.iter().find(|f| f.depth == next).unwrap();
    let w = destination.width();
    let destination_view = depth_view(r, next);
    let mut occupied: HashSet<_> = r
        .actors_at(next)
        .iter()
        .filter(|a| !a.dead.value_or(false))
        .map(|a| a.y as usize * w + a.x as usize)
        .collect();
    if r.depth == next && !r.player.dead.value_or(false) {
        occupied.insert(r.player.y as usize * w + r.player.x as usize);
    }
    let mut plans = Vec::new();
    let mut failed = Vec::new();
    for a in falling {
        let landing = nearest_fall_landing(destination, a.x, a.y, &occupied, |at| {
            !occupied_cell(destination,crate::portals::enabled(r),&occupied,at)
                && !crate::revision51_stairs::blocked_cell(r,&a,destination,at)
                && !["wall", "tree", "chasm"].contains(&destination.tiles[at].as_str())
                && (destination.tiles[at] != "pit" || a.id == r.player.id)
                && !destination
                    .grates
                    .as_ref()
                    .is_some_and(|g| g.contains(&(at as f64)))
                && !destination
                    .doors
                    .as_ref()
                    .and_then(|d| d.get(&at.to_string()))
                    .is_some_and(|d| !d.open)
                && !engine::bulwark_blocks(&destination_view, &a, (at % w) as f64, (at / w) as f64)
        });
        if let Some(p) = landing {
            if !crate::half_here::is_clone(&a) {
                occupied.insert(p.y as usize * w + p.x as usize);
            }
            plans.push((a, p));
        } else {
            failed.push(a)
        }
    }
    for a in failed {
        if a.id == r.player.id || (r.depth == source_depth && vision::visible_actor(r, &a)) {
            engine::log(
                r,
                format!(
                    "{} fall: no free landing exists below.",
                    if a.id == r.player.id {
                        "You cannot".into()
                    } else {
                        format!("{} cannot", crate::actor_presentation::name(r, &a))
                    }
                ),
                "player",
            );
        }
    }
    // Endpoint exposure injures its stable source-floor owner before dismissal.
    for index in (0..plans.len()).rev() {
        let (a, p) = &plans[index];
        if !crate::half_here::is_clone(a) {
            continue;
        }
        if r.depth == source_depth
            && r.floors.iter().find(|f| f.depth == next).unwrap().tiles
                [p.y as usize * w + p.x as usize]
                != "water"
        {
            engine::damage_environment(r, a.id, a.max_hp * 0.25, instigator, "fall", false)
                .map_err(|e| e.0)?;
        }
        crate::half_here::dismiss(r);
        plans.remove(index);
    }
    if plans.is_empty() || (player && r.status != "playing") {
        return Ok(());
    }
    let player_falls = plans.iter().any(|(a, _)| a.id == r.player.id);
    if player_falls {
        if r.simulation_revision.value_or(29.) >= 52. {
            let to = crate::fast_mode::arrival_target(r, next as u32);
            crate::startup::ensure_floors(r, to)?;
        }
        let landing = plans.iter().find(|(a,_)|a.id == r.player.id).unwrap().1;
        queue_flying_chasm_followers(r,next,landing);
        dismiss_half_here(r)
    }
    for (a, _) in &plans {
        if a.id == r.player.id || (r.depth == source_depth && vision::visible_actor(r, a)) {
            engine::log(
                r,
                format!(
                    "{} to the floor below.",
                    if a.id == r.player.id {
                        "You fall".into()
                    } else {
                        format!("{} falls", crate::actor_presentation::name(r, a))
                    }
                ),
                if instigator == Some(r.player.id) {
                    "player"
                } else if instigator.is_some() {
                    "enemy"
                } else {
                    "environment"
                },
            );
        }
    }
    let departed: HashSet<_> = plans.iter().map(|(a, _)| a.id.to_bits()).collect();
    r.actors_at_mut(source_depth)
        .retain(|a| !departed.contains(&a.id.to_bits()));
    for (a, p) in &plans {
        let mut actor = if a.id == r.player.id {
            r.player.clone()
        } else {
            a.clone()
        };
        actor.x = p.x as f64;
        actor.y = p.y as f64;
        clear_floor_chase(&mut actor);
        actor.last_seen = Field::Missing;
        for key in ["grabbed", "grabbedBy", "stuck", "targetId"] {
            actor.conditions.shift_remove(key);
        }
        actor.conditions.insert("seesPlayer".into(), 0.0);
        actor
            .conditions
            .insert("offscreenActionTick".into(), r.tick);
        if actor.id == r.player.id {
            for entry in r.ground_at_mut(source_depth) {
                entry.auto_pickup_blocked = Field::Missing;
            }
            r.player = actor;
            r.depth = next;
            r.rally = None;
        } else {
            r.actors_at_mut(next).push(actor)
        }
    }
    for (a,_) in &plans {crate::revision50::duel_actor_changed(r,a.id);}
    for f in &mut r.floors {
        if f.depth == source_depth || f.depth == next {
            crate::mutation_journal::make_mut(f).dirty = true
        }
    }
    let active = r.depth == next;
    let mut head = depth_view(r, next);
    if !active && instigator == Some(r.player.id) {
        head.player.abilities = r.player.abilities.clone();
        head.player.conditions = r.player.conditions.clone();
    }
    let cause = if instigator == Some(r.player.id) {
        Some(head.player.id)
    } else {
        instigator
    };
    if player_falls { crate::future_floor::capture_entry(&mut head)?; }
    for (a, p) in &plans {
        let id = a.id;
        let prior = engine::actor(&head, id)
            .unwrap()
            .conditions
            .get("forcedMove")
            .copied();
        engine::actor_mut_trusted(&mut head, id)
            .unwrap()
            .conditions
            .insert("forcedMove".into(), 1.0);
        enemy_moved(&mut head, id)?;
        if let Some(v) = prior {
            engine::actor_mut_trusted(&mut head, id)
                .unwrap()
                .conditions
                .insert("forcedMove".into(), v);
        } else {
            engine::actor_mut_trusted(&mut head, id)
                .unwrap()
                .conditions
                .shift_remove("forcedMove");
        }
        if head.floor().unwrap().tiles[p.y as usize * w + p.x as usize] != "water" {
            let hp = engine::actor(&head, id).unwrap().max_hp;
            engine::damage_environment(&mut head, id, hp * 0.25, cause, "fall", false)
                .map_err(|e| e.0)?;
        }
    }
    if player_falls && next>source_depth {crate::events::player_descent(false,true);}
    if active {
        head.actors.adopt_epoch_from(&r.actors);
        *r = head;
    } else {
        if instigator == Some(r.player.id) {
            r.player.conditions = head.player.conditions.clone();
        }
        copy_depth_changes(r, head);
    }
    for depth in [source_depth, next] {
        let actors = r.actors_at(depth).to_vec();
        if let Some(f) = r.floors.iter_mut().find(|f| f.depth == depth) {
            crate::startup::refresh_razorleaves(crate::mutation_journal::make_mut(f), &actors);
        }
    }
    if player_falls {
        let f = r.floor().unwrap();
        let domain = f.domain.clone();
        let pit = f.tiles[r.player.y as usize * f.width() + r.player.x as usize] == "pit";
        engine::log(r, format!("Floor {}: {domain}.", crate::fast_mode::display_depth(r, next)), "environment");
        if r.status == "playing" && pit {
            r.status = "won".into();
            engine::log(r, "You fall into the bottomless pit. Victory.", "player");
        }
    }
    Ok(())
}
pub fn descent_footprint(f: &Floor, x: f64, y: f64, radius: i32) -> Vec<usize> {
    if !in_bounds(f, x, y) || radius < 0 {
        return vec![];
    }
    let mut cells = Vec::new();
    for yy in (y as i32 - radius).max(0)..=(y as i32 + radius).min(f.height() as i32 - 1) {
        for xx in (x as i32 - radius).max(0)..=(x as i32 + radius).min(f.width() as i32 - 1) {
            if (xx as f64 - x).powi(2) + (yy as f64 - y).powi(2) <= (radius * radius) as f64 {
                cells.push(yy as usize * f.width() + xx as usize)
            }
        }
    }
    cells
}
pub fn descent_cells(f: &Floor, x: f64, y: f64, diluted: bool) -> Vec<usize> {
    if f.depth >= 40.0 {
        return vec![];
    }
    let w = f.width();
    let mut fixed: HashSet<_> = f
        .doors
        .as_ref()
        .into_iter()
        .flat_map(|d| d.keys())
        .filter_map(|k| k.parse::<usize>().ok())
        .chain(
            f.grates
                .as_ref()
                .into_iter()
                .flatten()
                .chain(f.permanent_fire.as_ref().into_iter().flatten())
                .map(|i| *i as usize),
        )
        .chain(
            f.vents
                .as_ref()
                .into_iter()
                .flatten()
                .map(|v| v.at as usize),
        )
        .chain(
            f.light_sources
                .as_ref()
                .into_iter()
                .flatten()
                .map(|s| s.y as usize * w + s.x as usize),
        )
        .collect();
    if let Some(e) = f.godbeast_event.as_ref() {
        if let Some(p) = e.altar.as_ref() {
            fixed.insert(p.y as usize * w + p.x as usize);
        }
        if let Some(p) = e.well.as_ref() {
            fixed.insert(p.y as usize * w + p.x as usize);
        }
        if let Some(p) = e.lever.as_ref() {
            fixed.insert(p.y as usize * w + p.x as usize);
        }
        if e.kind == "Veridian" {
            fixed.insert(e.y as usize * w + e.x as usize);
        }
    }
    descent_footprint(f, x, y, if diluted { 1 } else { 2 })
        .into_iter()
        .filter(|at| {
            let tile = if f.tiles[*at] == "chasm" {
                f.altered_chasms
                    .as_ref()
                    .and_then(|m| m.get(&at.to_string()))
                    .map(|s| s.tile.as_str())
            } else {
                Some(f.tiles[*at].as_str())
            };
            !fixed.contains(at)
                && tile.is_some_and(|t| ["floor", "grass", "water", "shallow-water", "speed-up"].contains(&t))
        })
        .collect()
}
pub fn collapse_descent(r: &mut Run, x: f64, y: f64, diluted: bool) -> Result<bool, String> {
    let direct_roots = r.simulation_revision.value_or(29.) >= 51.;
    let source_depth = r.depth;
    if !crate::fast_mode::playable(r, crate::fast_mode::below(r, source_depth)) {
        engine::log(
            r,
            "The floor does not collapse: there is no floor below.",
            "player",
        );
        return Ok(false);
    }
    let cells = descent_cells(r.floor().ok_or("Missing descent source")?, x, y, diluted);
    if cells.is_empty() {
        engine::log(r, "There is no collapsible floor here.", "player");
        return Ok(false);
    }
    let next = crate::fast_mode::below(r, source_depth);
    let to = if r.simulation_revision.value_or(29.) >= 52. { next as u32 }
        else { crate::startup::arrival_generation_target(r.simulation_revision.value_or(29.), next as u32) };
    crate::startup::ensure_floors(r, to)?;
    crate::offscreen::catch_up_floor(r, next)?;
    let w = r.floor().unwrap().width();
    let changed: HashSet<_> = cells.iter().copied().collect();
    let victims: Vec<_> = std::iter::once(&r.player)
        .chain(r.actors_at(source_depth))
        .filter(|a| !a.dead.value_or(false) && changed.contains(&(a.y as usize * w + a.x as usize)))
        .map(|a| a.id)
        .collect();
    let corpses: Vec<_> = r
        .actors_at(source_depth)
        .iter()
        .filter(|a| a.dead.value_or(false) && changed.contains(&(a.y as usize * w + a.x as usize)))
        .cloned()
        .collect();
    let items: Vec<_> = r
        .ground
        .get(&source_depth.to_string())
        .into_iter()
        .flat_map(|g| g.iter())
        .filter(|g| !crate::dreamwalking::dream_ground_item(&g.item) && changed.contains(&(g.y as usize * w + g.x as usize)))
        .cloned()
        .collect();
    crate::events::actor_event(
        r,
        &r.player,
        "burst",
        "Collapsing floor",
        Some(crate::events::EventPoint { x, y }),
        None,
    );
    let f = r.floor_mut().unwrap();
    for at in &cells {
        f.tiles[*at] = "chasm".into();
        if let Some(m) = f.altered_chasms.as_mut() {
            m.shift_remove(&at.to_string());
        }
        for layer in [
            &mut f.ichor,
            &mut f.slime,
            &mut f.mud,
            &mut f.roots,
            &mut f.currents,
        ] {
            if let Some(v) = layer.as_mut() {
                v[*at] = 0.0
            }
        }
        if let Some(b) = f.blood.as_mut() {
            b[*at] = false
        }
        for layer in [&mut f.ice, &mut f.snow, &mut f.puddles, &mut f.hellfire] {
            if let Some(v)=layer.as_mut(){v[*at]=false;}
        }
        if let Some(e) = f.environment.as_mut() {
            e.fire[*at] = 0.0;
            if let Some(v) = e.fire_kind.as_mut() {
                v[*at] = None
            }
            if let Some(v) = e.fire_fresh.as_mut() {
                v[*at] = false
            }
            if let Some(p) = e.provenance.as_mut().and_then(|m| m.get_mut("fire")) {
                p.owner[*at] = -1.0;
                p.player[*at] = 0.0;
            }
        }
    }
    for root in f.fungal_roots.as_mut().into_iter().flatten() {
        if root.path.iter().any(|at| changed.contains(&(*at as usize))) {
            root.active = false
        }
    }
    if direct_roots { crate::fungi::clear_orphaned_root_cells(f); }
    f.dirty = true;
    let dropped: HashSet<_> = items.iter().map(|e| e.item.id.to_bits()).collect();
    r.ground_at_mut(source_depth)
        .retain(|g| !dropped.contains(&g.item.id.to_bits()));
    for entry in items {
        crate::item_placement::place_ground_item(r, source_depth, entry, |r, d| {
            crate::startup::ensure_floors(r, d as u32)
        })?;
    }
    for corpse in corpses {
        crate::corpse_placement::place_corpse(r, source_depth, corpse, |r, d| {
            crate::startup::ensure_floors(r, d as u32)
        })?;
    }
    engine::remember(r);
    crate::inventory::remember_detected_items(r)?;
    fall_actors_one_level(r, source_depth, &victims, Some(r.player.id))?;
    Ok(true)
}

// O(P) alias lookup, P<=7; landing placement does not retrigger entry transport.
fn occupied_cell(f:&Floor, portals:bool, occupied:&HashSet<usize>, at:usize)->bool {
    occupied.contains(&at) || portals && crate::portals::aliases(f,at).iter().any(|other|occupied.contains(other))
}
