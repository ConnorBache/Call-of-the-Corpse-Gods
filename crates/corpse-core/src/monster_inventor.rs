//! Hemogoblin inventor's physical shell, mounting, and pilot destruction.
use crate::{
    Actor, Field, GroundItem, Run,
    engine::{self, actor, actor_mut_trusted, condition},
    movement::{DIRECTIONS, Point},
};
fn living(a: &Actor) -> bool {
    !a.dead.value_or(false) && a.hp > 0.0
}
fn trait_(a: &Actor, s: &str) -> bool {
    a.abilities.iter().any(|v| v == &format!("enemy:beast:{s}"))
}
pub fn is_mech(a: &Actor) -> bool {
    a.creature_kind
        .as_ref()
        .is_some_and(|s| s == "hemogoblin-mech")
        || trait_(a, "mech")
}
fn p(a: &Actor) -> Point {
    Point {
        x: a.x as i32,
        y: a.y as i32,
    }
}
fn report(r: &mut Run, a: &Actor, message: &str, target: Option<&Actor>) {
    if r.player.x < 0.0 {
        return;
    }
    let visible = crate::vision::visible_actor(r, a);
    if a.id == r.player.id
        || visible
        || target.is_some_and(|b| b.id == r.player.id || crate::vision::visible_actor(r, b))
    {
        engine::log(
            r,
            format!(
                "{} {}",
                if a.id == r.player.id {
                    "You"
                } else if visible {
                    &a.name
                } else {
                    "Something unseen"
                },
                message
            ),
            if a.id == r.player.id {
                "player"
            } else {
                "enemy"
            },
        );
    }
}
/// One actor path query or a constant-neighborhood mount, preserving actor order.
pub fn turn(
    r: &mut Run,
    id: f64,
    mut target: Option<f64>,
    movement_only: bool,
) -> Result<bool, String> {
    let a = actor(r, id).ok_or("Inventor actor missing")?.clone();
    if !living(&a) || id == r.player.id || crate::half_here::is_clone(&a) {
        return Ok(false);
    }
    if is_mech(&a) {
        if a.monster_state
            .as_ref()
            .and_then(|s| s.pilot.as_ref())
            .is_none()
        {
            actor_mut_trusted(r, id).unwrap().state = "neutral".into();
            return Ok(true);
        }
        if a.conditions.get("monsterMountedTick") == Some(&r.tick) {
            return Ok(true);
        }
    }
    if crate::ai::sleeping(&a) {
        return Ok(true);
    }
    if a.state == "ally" && a.perfect.value_or(false) && a.order.as_ref().is_some_and(|s| s == "p")
    {
        return Ok(false);
    }
    if target.is_none() && a.state == "ally" {
        target = crate::ai::nearest_ally_enemy(r, &a).map(|a| a.id)
    }
    if !trait_(&a, "inventor") {
        return Ok(false);
    }
    if target == Some(r.player.id) {
        actor_mut_trusted(r, id)
            .unwrap()
            .conditions
            .insert("monsterInventorFleeing".into(), 1.0);
    }
    let a = actor(r, id).unwrap().clone();
    if condition(&a, "monsterInventorFleeing") == 0.0 {
        return Ok(false);
    }
    let mech = r
        .actors_at(r.depth)
        .iter()
        .find(|b| {
            living(b)
                && Some(&b.id) == a.conditions.get("monsterLinkedMechId")
                && is_mech(b)
                && b.monster_state
                    .as_ref()
                    .and_then(|s| s.pilot.as_ref())
                    .is_none()
        })
        .cloned();
    let Some(mech) = mech else {
        return if let Some(threat) = target.and_then(|id| actor(r, id)).map(p) {
            crate::monster_vampire::flee(r, id, threat, movement_only).map_err(|e| e.0)
        } else {
            Ok(true)
        };
    };
    if crate::portals::actor_distance(r, &a, &mech) <= std::f64::consts::SQRT_2 {
        if movement_only {
            return Ok(true);
        }
        let tick = r.tick;
        let m = actor_mut_trusted(r, mech.id).unwrap();
        m.monster_state.get_or_insert_with(Default::default).pilot =
            Field::Value(Box::new(a.clone()));
        m.conditions.insert("monsterLinkedInventorId".into(), id);
        m.conditions.insert("monsterMountedTick".into(), tick);
        m.state = if a.state == "ally" { "ally" } else { "hunting" }.into();
        if a.last_seen.as_ref().is_some() {
            m.last_seen = a.last_seen.clone()
        }
        if let Some(v) = a.conditions.get("targetId") {
            m.conditions.insert("targetId".into(), *v);
        }
        crate::animation_enemy::mark(r,&a,"E05","fold",crate::events::Stage::Special);
        r.actors_at_mut(r.depth).retain(|b| b.id != id);
        crate::revision50::end_duels_with_target(r,id);
        r.floor_mut().unwrap().dirty = true;
        report(
            r,
            &a,
            "climbs into the Hemogoblin Mech. The machine wakes.",
            None,
        );
        crate::events::actor_event(
            r,
            actor(r, mech.id).unwrap(),
            "pulse",
            "Mech activated",
            None,
            None,
        );
        return Ok(true);
    }
    if let Some(step) = engine::creature_path_step(r, id, p(&mech)) {
        engine::move_actor(r, id, step.x as f64, step.y as f64, movement_only).map_err(|e| e.0)?;
    }
    Ok(true)
}
fn landing(r: &Run, pilot: &Actor, origin: Point) -> Point {
    let f = r.floor().unwrap();
    let candidates: Vec<_> = DIRECTIONS
        .iter()
        .map(|d| Point {
            x: origin.x + d.x,
            y: origin.y + d.y,
        })
        .filter(|p| {
            crate::travel::can_traverse_from(r, pilot, p.x as f64, p.y as f64, origin)
                && !["pit", "chasm"]
                    .contains(&f.tiles[p.y as usize * f.width() + p.x as usize].as_str())
                && !r
                    .actors_at(r.depth)
                    .iter()
                    .any(|a| a.dead.value_or(false) && crate::portals::same_position(r, a, p.x as f64, p.y as f64))
        })
        .collect();
    candidates
        .iter()
        .find(|p| {
            !(crate::portals::same_position(r, &r.player, p.x as f64, p.y as f64)
                && !r.player.dead.value_or(false))
                && !r
                    .actors_at(r.depth)
                    .iter()
                    .any(|a| !a.dead.value_or(false) && crate::portals::same_position(r, a, p.x as f64, p.y as f64))
        })
        .copied()
        .or_else(|| candidates.first().copied())
        .unwrap_or(origin)
}
/// Destroyed shells are removed before the detached pilot's unavoidable ordinary death.
pub fn actual_death(
    r: &mut Run,
    id: f64,
    origin: Point,
    source: Option<f64>,
    cause: Option<&str>,
) -> Result<bool, String> {
    let a = actor(r, id).ok_or("Mech death actor missing")?.clone();
    if !a.dead.value_or(false) {
        return Ok(false);
    }
    let physical = a
        .creature_kind
        .as_ref()
        .filter(|s| !s.is_empty())
        .map(|s| s == "hemogoblin-mech")
        .unwrap_or_else(|| {
            a.name.strip_prefix("Mercurial ").unwrap_or(&a.name) == "Hemogoblin Mech"
        });
    if !physical {
        return Ok(false);
    }
    if let Some(pilot) = a.monster_state.as_ref().and_then(|s| s.pilot.as_ref()) {
        if pilot.id == r.player.id || r.actors_at(r.depth).iter().any(|b| b.id == pilot.id) {
            return Err("Unsupported ambiguous duplicate inventor pilot identity".into());
        }
    }
    let carried = actor_mut_trusted(r, id)
        .unwrap()
        .monster_state
        .as_mut()
        .and_then(|s| {
            if s.carried_items.as_ref().is_some_and(|v| !v.is_empty()) {
                std::mem::take(&mut s.carried_items).into_option()
            } else {
                None
            }
        });
    for item in carried.into_iter().flatten() {
        crate::item_placement::place_ground_item(
            r,
            r.depth,
            GroundItem {
                x: origin.x as f64,
                y: origin.y as f64,
                item,
                auto_pickup_blocked: Field::Missing,
            },
            |r, d| crate::startup::ensure_floors(r, d as u32),
        )?;
    }
    let pilot = actor_mut_trusted(r, id)
        .unwrap()
        .monster_state
        .as_mut()
        .and_then(|s| std::mem::take(&mut s.pilot).into_option());
    actor_mut_trusted(r, id).unwrap().heartless = true.into();
    r.actors_at_mut(r.depth).retain(|a| a.id != id);
    r.floor_mut().unwrap().dirty = true;
    if let Some(pilot) = pilot {
        let mut pilot = *pilot;
        let p = landing(r, &pilot, origin);
        pilot.x = p.x as f64;
        pilot.y = p.y as f64;
        pilot.hp = 0.0;
        pilot.dead = false.into();
        pilot.conditions.shift_remove("monsterLinkedMechId");
        let id = pilot.id;
        if !r.actors_at(r.depth).iter().any(|a| a.id == id) {
            r.actors_at_mut(r.depth).push(pilot);
        }
        engine::resolve_death(
            r,
            id,
            source,
            Some(cause.unwrap_or("mech-destruction")),
            true,
        )
        .map_err(|e| e.0)?;
    }
    Ok(true)
}
/// Directed portal ray geometry: O(P²*R) bounded route/ray work, no RNG or mutation.
/// Return total physical ray length and its first non-portal segment heading.
fn portal_cone_vector(r: &Run, source: &Actor, x: f64, y: f64) -> Option<(f64, f64, f64)> {
    let f = crate::dreamwalking::floor_for(r, source)?;
    let target = Point { x: x as i32, y: y as i32 };
    if crate::non_euclidean_generation::has_links(f) {
        let pass=|q:Point|crate::portals::cell(f,q.x as f64,q.y as f64).is_some_and(|at|!matches!(f.tiles[at].as_str(),"wall"|"tree")&&!crate::environment::blocks_sight(f,at));
        let plan=crate::portals::ray_plan(f,p(source),target,pass);
        if plan.origin_fork.len()>1{return None}
        let ray=crate::portals::directed_ray_trace(f,plan.origin,plan.aim,pass,|options|if options.len()==1{Some(options[0])}else{None},|_|false);
        if !ray.points.last().is_some_and(|q|crate::portals::same_cell(f,q.x as f64,q.y as f64,x,y)){return None}
        return Some((ray.distance,(plan.aim.x-plan.origin.x)as f64,(plan.aim.y-plan.origin.y)as f64))
    }
    let path = crate::portals::aimed_ray(f, p(source), target, |q| {
        crate::portals::cell(f, q.x as f64, q.y as f64).is_some_and(|at|
            !matches!(f.tiles[at].as_str(), "wall" | "tree") && !crate::environment::blocks_sight(f, at))
    });
    if !path.last().is_some_and(|q| crate::portals::same_cell(f, q.x as f64, q.y as f64, x, y)) { return None; }
    let length = crate::portals::ray_distance(f, &path);
    let mut start = *path.first()?;
    let mut end = start;
    for pair in path.windows(2) {
        if crate::portals::same_cell(f, pair[0].x as f64, pair[0].y as f64, pair[1].x as f64, pair[1].y as f64) {
            if end != start { break; }
            start = pair[1]; end = start;
        } else { end = pair[1]; }
    }
    Some((length, (end.x-start.x) as f64, (end.y-start.y) as f64))
}
/// Linked cones scan V<=3600 cells; legacy cones retain the local13x13 scan.
/// With N<=80 physical route nodes, C<=48 pocket cells, P<=7 portal pairs and
/// A actors: O(V*(N²*V + V*A*(P+C))) geometry/collision plus ignition/damage;
/// O(V+A+N) scratch including deduplicated emitted aims. No preview RNG.
/// Fixed six-cell-radius cone: row-major ignitions precede actor-order direct hits.
pub fn breathe_fire(r: &mut Run, id: f64, target: f64) -> Result<bool, String> {
    crate::dreamwalking::with_actor(r, id, |r| breathe_fire_inner(r, id, target))
}
fn breathe_fire_inner(r: &mut Run, id: f64, target: f64) -> Result<bool, String> {
    let source = actor(r, id).ok_or("Fire source missing")?.clone();
    let target = actor(r, target).ok_or("Fire target missing")?.clone();
    if !crate::dreamwalking::same_plane(r, &source, &target)
        || (crate::dreamwalking::enabled(r) && crate::dreamwalking::is_body(&source))
    {
        return Ok(false);
    }
    let pockets=r.simulation_revision.value_or(29.)>=36.&&r.floor().is_some_and(|f|crate::non_euclidean_generation::has_links(f));
    let linked = crate::portals::enabled(r) && r.floor().is_some_and(|f| f.portals.as_ref().is_some_and(|p| !p.is_empty())||pockets);
    let (length, dx, dy) = if linked {
        let Some(v) = portal_cone_vector(r, &source, target.x, target.y) else { return Ok(false); };
        v
    } else {
        let (dx,dy) = (target.x-source.x, target.y-source.y);
        (dx.hypot(dy),dx,dy)
    };
    let heading_length = dx.hypot(dy);
    if r.tick < condition(&source, "monsterFireReadyAt") || length > 6.0 || length == 0.0 {
        return Ok(false);
    }
    let w = r.floor().unwrap().width();
    let h = r.floor().unwrap().height();
    let mut affected = std::collections::HashSet::new();
    let mut emitted=std::collections::HashSet::new();
    let (min_x,max_x,min_y,max_y) = if linked { (0,w as i32-1,0,h as i32-1) }
        else { (source.x as i32-6,source.x as i32+6,source.y as i32-6,source.y as i32+6) };
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
                continue;
            }
            let (range,vx,vy) = if linked {
                let Some(v) = portal_cone_vector(r, &source, x as f64, y as f64) else { continue; };
                v
            } else {
                let (vx,vy) = (x as f64-source.x,y as f64-source.y);
                (vx.hypot(vy),vx,vy)
            };
            if range == 0.0
                || range > 6.0
                || (vx * dx + vy * dy) / (vx.hypot(vy) * heading_length) < (std::f64::consts::PI / 6.0).cos()
            {
                continue;
            }
            let point = Actor {
                x: x as f64,
                y: y as f64,
                ..Default::default()
            };
            if !crate::vision::line_of_sight(r, &source, &point) {
                continue;
            }
            if pockets {
                // All geometry/angle/LOS decisions above are read-only. Each
                // admitted ray is emitted once; only its actual encountered
                // random entry advances RNG, and its actual landing is ignited.
                let f=r.floor().unwrap();let (origin,aim)=crate::portals::ray_aim(f,p(&source),Point{x,y},|q|crate::portals::cell(f,q.x as f64,q.y as f64).is_some_and(|at|!matches!(f.tiles[at].as_str(),"wall"|"tree")&&!crate::environment::blocks_sight(f,at)));
                if !emitted.insert((origin.x,origin.y,aim.x,aim.y)){continue}
                let land=crate::projectiles::trace_committed_from_actor(r,&source,x as f64,y as f64);
                let at=land.y as usize*w+land.x as usize;
                for alias in crate::portals::aliases(r.floor().unwrap(),at){affected.insert(alias);}
                crate::environment::ignite(r,land.x,land.y,None,Some(id))?;
                continue;
            }
            let land = crate::projectiles::trace_from_actor(r, &source, x as f64, y as f64);
            let reaches = if linked {
                crate::portals::same_cell(r.floor().unwrap(), land.x, land.y, x as f64, y as f64)
            } else { land.x == x as f64 && land.y == y as f64 };
            if !reaches { continue; }
            let cell = y as usize * w + x as usize;
            affected.insert(cell);
            if linked { if let Some(mate) = crate::portals::mate(r.floor().unwrap(), cell) { affected.insert(mate); } }
            crate::environment::ignite(r, x as f64, y as f64, None, Some(id))?;
        }
    }
    let breath_action=crate::events::current_action();
    let breath_receipt=crate::events::emission_serial();
    if crate::events::enabled(r) && crate::events::actor_observed(r,&source) {
        // Sight and distance are evaluated at the committed footprint only.
        // Portal/pocket jumps never interpolate an unobserved stretch.
        let band=|r:&Run,cell:&crate::events::EventPoint| {
            (crate::portals::point_distance(r,&source,cell.x,cell.y).ceil() as u8)
                .clamp(1,6)-1
        };
        let mut cells:Vec<_>=affected.iter().copied().collect();
        cells.sort_unstable();
        let cells=crate::animation_amulets::cell_points(r,&cells);
        crate::events::travel_bands(r,&source,"E03","ember",&cells,|cell|band(r,cell));
    }
    let breath_visible=crate::events::emission_serial()!=breath_receipt;
    let victims: Vec<_> = std::iter::once(&r.player)
        .chain(r.actors_at(r.depth))
        .filter(|a| {
            a.id != id && living(a) && crate::dreamwalking::same_plane(r, &source, a)
                && affected.contains(&(a.y as usize * w + a.x as usize))
        })
        .map(|a| a.id)
        .collect();
    for v in victims {
        if actor(r, v).is_none() {
            continue;
        }
        if actor(r,v).is_some_and(|target|crate::revision50_combat::oblivious_blocks(r,target,Some(&source))) {continue;}
        // This is the resolved breath footprint, not lingering fire exposure.
        if crate::revision50_nightmares::strike(r, v) { continue; }
        engine::damage_direct_with_source_snapshot(r, v, crate::balance52::local_amount(r, &source, "monsterSpecialDamage", 24.), &source, "fire", true).map_err(|e| e.0)?;
        if actor(r, v).is_some_and(living) {
            let random = crate::rng::random(r);
            let player = if id == r.player.id { 1.0 } else { 0.0 };
            let revision = r.simulation_revision.value_or(29.);
            crate::corpses::apply_burning_run(r, v,
                revision,
                random,
                Some(id),
                Some(player),
                None,
            );
        }
    }
    let tick = r.tick;
    if let Some(a) = actor_mut_trusted(r, id) {
        a.conditions.insert("monsterFireReadyAt".into(), tick + 4.0);
        a.conditions.insert("rangedRecovery".into(), 1.0);
    }
    let updated = actor(r, id).cloned().unwrap_or(source);
    report(
        r,
        &updated,
        "breathes a spreading cone of fire!",
        Some(&target),
    );
    if breath_visible {
        crate::events::checkpoint_for_action(r,breath_action);
    }
    if !crate::events::enabled(r) { crate::events::actor_event(
        r,
        &updated,
        "burst",
        "Fire breath",
        Some((&target).into()),
        None,
    ); }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn enemy_breath_beam_has_no_cells_in_portal_gap() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();
        r.player.x=33.;r.player.y=10.;r.player.stats.perception=300.;
        r.player.hp=5000.;r.player.max_hp=5000.;
        r.player.conditions.clear();r.player.active.clear();r.actors_at_mut(r.depth).clear();
        let f=r.floor_mut().unwrap();let w=f.width();
        f.tiles.fill("floor".into());f.lights.fill(100.);
        f.doors=crate::Field::Missing;f.environment=crate::Field::Missing;
        f.non_euclidean=crate::Field::Missing;f.speed_up=crate::Field::Missing;
        for y in 0..f.height() {f.tiles[y*w+20]="wall".into();}
        f.portals=vec![crate::portals_generation::PortalPair{a:10*w+12,b:10*w+30,color_index:0}].into();
        let a=crate::population::create_actor_revision("dragon",900.,10.,10.,false,52.).unwrap();
        r.actors_at_mut(r.depth).push(a);
        let p=r.player.id;
        crate::events::begin();
        assert!(breathe_fire(&mut r,900.,p).unwrap());
        let events=crate::events::finish();
        let cue=events.iter().filter_map(|e|e.animation.as_ref()).find(|a|a.proposal=="E03").unwrap();
        assert_eq!(cue.delivery,crate::events::Delivery::Magic);
        assert!(cue.cells.iter().any(|p|p.x<20.));
        assert!(cue.cells.iter().any(|p|p.x>30.));
        assert!(!cue.cells.iter().any(|p|p.x>16.&&p.x<28.));
        let bands=cue.arrival_steps.as_ref().unwrap();
        let step=|x:f64|bands[cue.cells.iter().position(|p|p.x==x&&p.y==10.).unwrap()];
        assert_eq!(step(12.),step(30.));
        assert!(step(31.)>step(30.));
    }
    #[test]
    fn dream_plane_fire_cone_never_applies_burning_or_rng_to_other_plane_victims() {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 34.0.into();
        r.dreamwalking = crate::dreamwalking::State { active: true, depth: 1., ..Default::default() }.into();
        r.player.conditions.clear(); r.player.active.clear();
        let source_id = r.actors_at(1.)[0].id;
        let player = r.player.id;
        let before = r.clone();
        assert!(!breathe_fire(&mut r, source_id, player).unwrap());
        assert_eq!(r, before, "cross-plane target must not ignite terrain or consume RNG");
        let mut native = r.actors_at(1.)[0].clone();
        native.id = 90.; native.x = r.player.x; native.y = r.player.y;
        native.hp = 100.; native.max_hp = 100.; native.conditions.clear();
        let mut dream = native.clone(); dream.id = 91.;
        dream.conditions.insert("phantasmiiPlane".into(), 1.);
        r.actors_at_mut(1.).extend([native, dream.clone()]);
        let player_before = r.player.clone();
        assert!(breathe_fire(&mut r, source_id, 90.).unwrap());
        assert_eq!(r.player, player_before);
        assert_eq!(actor(&r, 91.), Some(&dream));
        assert!(actor(&r, 90.).unwrap().hp < 100.);
        assert!(condition(actor(&r, 90.).unwrap(), "burning") > 0.);
    }
    use serde_json::Value;
    #[test]
    fn inventor_matches_frozen_reference() {
        let cases: Value =
            serde_json::from_str(include_str!("../../../fixtures/inventor-v48.json")).unwrap();
        for c in cases.as_array().unwrap() {
            let mut r: Run = serde_json::from_value(c["initial"].clone()).unwrap();
            crate::events::begin();
            let id = c["id"].as_f64().unwrap();
            let result = match c["operation"].as_str().unwrap() {
                "turn" => turn(
                    &mut r,
                    id,
                    c["target"].as_f64(),
                    c["movementOnly"].as_bool().unwrap(),
                ),
                "fire" => {
                    let p = r.player.id;
                    breathe_fire(&mut r, id, p)
                }
                _ => {
                    let p = r.player.id;
                    actual_death(&mut r, id, Point { x: 10, y: 5 }, Some(p), None)
                }
            }
            .unwrap();
            assert_eq!(result, c["result"].as_bool().unwrap(), "{}", c["kind"]);
            let events = crate::events::finish();
            let actual = serde_json::to_value(&r).unwrap();
            if !crate::storage::canonical_equal(&actual, &c["expected"]) {
                std::fs::write(
                    "/tmp/inventor-native.json",
                    serde_json::to_vec(&actual).unwrap(),
                )
                .unwrap();
                std::fs::write(
                    "/tmp/inventor-source.json",
                    serde_json::to_vec(&c["expected"]).unwrap(),
                )
                .unwrap();
                panic!("{}", c["kind"]);
            }
            assert!(
                crate::storage::canonical_equal(
                    &serde_json::to_value(events).unwrap(),
                    &c["events"]
                ),
                "{} events",
                c["kind"]
            );
        }
    }
    #[test]
    fn duplicate_pilot_identity_rejects_without_mutation() {
        let cases: Value =
            serde_json::from_str(include_str!("../../../fixtures/inventor-v48.json")).unwrap();
        let c = cases
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["kind"] == "death-pilot")
            .unwrap();
        let mut r: Run = serde_json::from_value(c["initial"].clone()).unwrap();
        let pilot = r.actors_at(1.0)[0]
            .monster_state
            .as_ref()
            .unwrap()
            .pilot
            .as_ref()
            .unwrap()
            .as_ref()
            .clone();
        r.actors_at_mut(1.0).push(pilot);
        let before = serde_json::to_value(&r).unwrap();
        assert!(actual_death(&mut r, 11.0, Point { x: 10, y: 5 }, None, None).is_err());
        assert!(crate::storage::canonical_equal(
            &before,
            &serde_json::to_value(&r).unwrap()
        ));
    }
}
