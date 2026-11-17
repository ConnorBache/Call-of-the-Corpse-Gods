//! Authored Hornlight, Beckon and temporary Stone Seal, in source priority.
use crate::{
    Actor, Run,
    engine::{self, actor, actor_mut_trusted, condition},
};
fn has(a: &Actor, s: &str) -> bool {
    a.abilities.iter().any(|v| v == &format!("enemy:beast:{s}"))
}
fn distance(r: &Run, a: &Actor, b: &Actor) -> f64 {
    crate::portals::actor_distance(r, a, b)
}
fn sensed_from(r: &Run, origin: &Actor, target: &Actor) -> bool {
    let has = |id: &str| r.player.abilities.iter().any(|s| s == id);
    let active = |id: &str| r.player.active.iter().any(|s| s == id);
    !target.dead.value_or(false)
        && (target.perfect.value_or(false)
            || condition(origin, "telepathic") > 0.
            || (condition(&r.player, "cameTurns") > 0.
                && condition(&r.player, "cameTarget") == target.id)
            || (has("numo-mind-read") && distance(r, origin, target)
                <= if active("numo-mind-read") { 20. } else { 5. })
            || (has("ursa-dream-see") && (crate::ai::sleeping(target)
                || (active("ursa-dream-see") && crate::vision::line_of_sight(r, origin, target)))))
}
/// With a provisioned plane: O(F + B*(A*(P+R) + (S+C)*R)), O(B) references;
/// F floors, A actors, P powers/actor, R ray length, S lights, C<=225 fire cells,
/// B<=2 origins. First-use layer creation adds ensure_layer's cloning cost.
/// Opposite-plane presentation visibility cannot supply an Uncast witness.
fn uncast_observes(r: &mut Run, target: &Actor) -> bool {
    if !crate::dreamwalking::enabled(r) { return crate::vision::visible_actor(r, target); }
    let dream = crate::dreamwalking::dream_actor(r, target);
    crate::dreamwalking::with_plane(r, dream, |r| {
        if !r.floor().is_some_and(|f| target.x >= 0. && target.y >= 0.
            && target.x < f.width() as f64 && target.y < f.height() as f64)
        { return false; }
        crate::half_here::origins(r).into_iter().any(|origin| {
            !origin.dead.value_or(false) && origin.hp > 0.
                && !crate::dreamwalking::is_body(origin)
                && crate::dreamwalking::same_plane(r, origin, target)
                && (crate::vision::all_seeing(r) || crate::dreamwalking::is_nightmare(target)
                    || sensed_from(r, origin, target)
                    || r.actors_at(r.depth).iter().any(|nearby| {
                        crate::dreamwalking::same_plane(r, origin, nearby)
                            && distance(r, nearby, target) <= 2. && sensed_from(r, origin, nearby)
                    })
                    || crate::vision::actor_chance(r, origin, target, if crate::revision51_perception::enabled(r) {crate::combat::effective_stats(r,origin).perception} else {300.}) >= 50.)
        })
    })
}
pub fn permitted(r: &Run, a: &Actor, b: &Actor) -> bool {
    !crate::revision50_combat::oblivious_blocks(r,b,Some(a))
        && !a.dead.value_or(false)
        && !b.dead.value_or(false)
        && crate::dreamwalking::same_plane(r, a, b)
        && !(crate::dreamwalking::enabled(r) && crate::dreamwalking::is_body(a))
        && ((a.id == r.player.id || a.state == "ally")
            == (b.id == r.player.id || b.state == "ally")
            || ((condition(b, "invisible") <= 0.0 || condition(a, "telepathic") > 0.0)
                && (distance(r, a, b) <= 5.0 || !b.active.iter().any(|s| s == "dawel-not-there"))))
}
pub fn spell_spend(r: &mut Run, id: f64, cost: f64) -> bool {
    let a = actor(r, id).unwrap().clone();
    if condition(&a, "silenced") > 0.0 || a.energy < cost
        || (crate::dreamwalking::enabled(r) && crate::dreamwalking::is_body(&a))
    {
        return false;
    }
    if std::iter::once(&r.player)
        .chain(r.actors_at(r.depth))
        .any(|s| {
            s.id != id
                && !crate::revision50_combat::oblivious_blocks(r,&a,Some(s))
                && !s.dead.value_or(false)
                && s.hp > 0.0
                && crate::dreamwalking::same_plane(r, s, &a)
                && has(s, "silence-aura")
                && (s.id == r.player.id || s.state == "ally")
                    != (id == r.player.id || a.state == "ally")
                && distance(r, s, &a) <= 3.0
                && crate::vision::line_of_sight(r, s, &a)
        })
    {
        return false;
    }
    if !crate::revision50::enabled(r) && id != r.player.id
        && a.state != "ally"
        && r.player.active.iter().any(|s| s == "grisha-uncast")
        && uncast_observes(r, &a)
    {
        if r.player.energy >= 10.0 {
            let p = r.player.clone();
            crate::resources::record_loss(r, &p, "energy", 10.0);
            r.player.energy -= 10.0;
            return false;
        }
        r.player.active.retain(|s| s != "grisha-uncast");
    }
    engine::enemy_spend(r, id, cost)
}
pub(crate) fn free(r: &Run, x: f64, y: f64, a: Option<&Actor>) -> bool {
    let f = r.floor().unwrap();
    if x < 1.0 || y < 1.0 || x > (f.width() - 2) as f64 || y > (f.height() - 2) as f64 {
        return false;
    }
    let i = y as usize * f.width() + x as usize;
    !["wall", "tree", "chasm", "pit"].contains(&f.tiles[i].as_str())
        && !crate::environment::blocks_movement(f, i)
        && !(r.simulation_revision.value_or(29.)>=52. && a.is_none() && crate::borge::reserved_cell(f,i))
        && a.is_none_or(|a| !engine::bulwark_blocks(r, a, x, y)
            && !crate::revision51_stairs::blocked(r,a,x,y))
        && !std::iter::once(&r.player).chain(r.actors_at(r.depth)).any(|b| {
            a.is_none_or(|a| a.id != b.id) && !b.dead.value_or(false)
                && crate::portals::same_cell(f, b.x, b.y, x, y)
                && a.map_or_else(|| crate::dreamwalking::contact_plane(r, b),
                    |a| crate::dreamwalking::same_plane(r, a, b))
        })
}
pub(crate) fn report(r: &mut Run, a: &Actor, b: &Actor, message: &str) {
    let av = crate::vision::visible_actor(r, a);
    if r.player.x >= 0.0
        && (a.id == r.player.id || b.id == r.player.id || av || crate::vision::visible_actor(r, b))
    {
        engine::log(
            r,
            format!(
                "{} {}",
                if a.id == r.player.id {
                    "You"
                } else if av {
                    crate::actor_presentation::name(r, a)
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
pub fn special_action(r: &mut Run, id: f64, target: f64) -> Result<bool, String> {
    let a = actor(r, id).ok_or("Horn actor missing")?.clone();
    let b = actor(r, target).ok_or("Horn target missing")?.clone();
    let range = distance(r, &a, &b);
    let allowed = permitted(r, &a, &b);
    if has(&a, "beckon") && range > 4.0 && condition(&a, "beastCooldown") == 0.0 && allowed {
        let mut choices = Vec::new();
        let f = r.floor().unwrap();
        for y in (a.y as i32 - 2).max(1)..=(a.y as i32 + 2).min(f.height() as i32 - 2) {
            for x in (a.x as i32 - 2).max(1)..=(a.x as i32 + 2).min(f.width() as i32 - 2) {
                let mut q = b.clone();
                q.x = x as f64;
                q.y = y as f64;
                let d = distance(r, &q, &a);
                if d <= 2.0
                    && d > std::f64::consts::SQRT_2
                    && d < range
                    && free(r, q.x, q.y, Some(&b))
                    && crate::vision::line_of_sight(r, &b, &q)
                {
                    choices.push(q);
                }
            }
        }
        if !choices.is_empty() && spell_spend(r, id, 12.0) {
            let q = &choices[(crate::rng::random(r) * choices.len() as f64).floor() as usize];
            let relocated=!crate::revision50_nightmares::is_decoy(&b) && (b.x!=q.x || b.y!=q.y);
            if crate::events::enabled(r) {if relocated {crate::animation_enemy::mark(r,&b,"E39","bracket-fold-gold",crate::events::Stage::Travel);}}
            else {crate::events::actor_event(r, &b, "pulse", "Vanish", None, None);}
            let t = actor_mut_trusted(r, target).unwrap();
            // The caster still targets/spends its action on the apparent player,
            // but a stationary hallucination cannot be teleported by Beckon.
            if !crate::revision50_nightmares::is_decoy(t) {
                t.x = q.x;
                t.y = q.y;
            }
            t.conditions.insert("forcedMove".into(), 1.0);
            crate::travel::enemy_moved(r, target)?;
            actor_mut_trusted(r, target)
                .unwrap()
                .conditions
                .shift_remove("forcedMove");
            let b = actor(r, target).unwrap().clone();
            if crate::events::enabled(r) {if relocated {crate::animation_enemy::mark(r,&b,"E39","bracket-unfold-gold",crate::events::Stage::Special);}}
            else {crate::events::actor_event(r, &b, "pulse", "Appear", None, None);}
            r.floor_mut().unwrap().dirty = true;
            actor_mut_trusted(r, id)
                .unwrap()
                .conditions
                .insert("beastCooldown".into(), 3.0);
            report(
                r,
                &a,
                &b,
                if target == r.player.id {
                    "beckons you closer."
                } else {
                    "beckons its prey closer."
                },
            );
            return Ok(true);
        }
    }
    if has(&a, "wall-seal")
        && range <= 5.0
        && condition(&a, "raisedWallExpires") == 0.0
        && condition(&a, "beastCooldown") == 0.0
        && allowed
    {
        let mut choices = Vec::new();
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let x = b.x + dx as f64;
                let y = b.y + dy as f64;
                if free(r, x, y, None)
                    && r.floor().unwrap().tiles
                        [y as usize * r.floor().unwrap().width() + x as usize]
                        == "floor"
                {
                    choices.push((x, y));
                }
            }
        }
        if choices.len() >= 3 && spell_spend(r, id, 15.0) {
            let (x, y) = choices[(crate::rng::random(r) * choices.len() as f64).floor() as usize];
            let i = y as usize * r.floor().unwrap().width() + x as usize;
            r.floor_mut().unwrap().tiles[i] = "wall".into();
            r.floor_mut().unwrap().dirty = true;
            let tick = r.tick;
            let wall_plane = crate::dreamwalking::enabled(r)
                .then(|| if crate::dreamwalking::context_dream(r) { 1. } else { 0. });
            let a = actor_mut_trusted(r, id).unwrap();
            a.conditions.insert("raisedWallIndex".into(), i as f64);
            a.conditions.insert("raisedWallExpires".into(), tick + 5.0);
            if let Some(plane) = wall_plane { a.conditions.insert("raisedWallPlane".into(), plane); }
            a.conditions.insert("beastCooldown".into(), 3.0);
            let a = a.clone();
            crate::events::actor_event(
                r,
                &a,
                "summon",
                "Stone seal",
                Some(crate::events::EventPoint { x, y }),
                None,
            );
            report(r, &a, &b, "raises a temporary wall beside its prey.");
            return Ok(true);
        }
    }
    if has(&a, "horn-bolt") && range > std::f64::consts::SQRT_2 && range <= 10.0 && allowed {
        let possible=crate::projectiles::possible_shot_range(r,&a,&b,10.);
        if let Some(range)=possible.filter(|_|spell_spend(r,id,8.)) {
            let (fixed, coefficient) = if crate::combat::raw_attack_damage_enabled(r.simulation_revision.value_or(29.)) { (10., 0.75) } else { (20., 1.5) };
            let base = fixed + crate::combat::effective_stats(r, &a).offense * coefficient;
            if r.simulation_revision.value_or(29.)>=36. {
                let actual=crate::projectiles::trace_committed_from_actor(r,&a,b.x,b.y);
    crate::events::tag_path(&a,"E38","bolt-ivory");
                let traveled=crate::projectiles::traveled_distance(r,&a,b.x,b.y,&actual);
                actor_mut_trusted(r,id).unwrap().conditions.insert("rangedRecovery".into(),1.);
                if let Some(victim)=actual.actor_id.filter(|_|traveled<=10.){engine::attack_ranged_resolved(r,id,victim,base,traveled,1.,false).map_err(|e|e.0)?;}
                crate::events::actor_event(r,&a,"projectile","Hornlight",Some(crate::events::EventPoint{x:actual.x,y:actual.y}),None);
            }else{
                engine::attack_ranged(r, id, target, base, range, 1.0, false).map_err(|e| e.0)?;
                crate::events::actor_event(r, &a, "projectile", "Hornlight", Some((&b).into()), None);
            }
            return Ok(true);
        }
    }
    Ok(false)
}
pub fn tick_actor(r: &mut Run, id: f64) {
    if let Some(a) = engine::actor_any_mut(r, id) {
        if condition(a, "beastCooldown") > 0.0 {
            *a.conditions.get_mut("beastCooldown").unwrap() -= 1.0;
        }
    }
}
pub fn world_turn(r: &mut Run) {
    let ids: Vec<_> = std::iter::once(r.player.id)
        .chain(r.actors_at(r.depth).iter().map(|a| a.id))
        .collect();
    let previously_silenced:std::collections::HashSet<u64>=ids.iter().filter(|&&id|actor(r,id).is_some_and(|a|condition(a,"auraSilenced")>0.)).map(|id|id.to_bits()).collect();
    for id in ids {
        actor_mut_trusted(r, id)
            .unwrap()
            .conditions
            .shift_remove("auraSilenced");
        let a = actor(r, id).unwrap();
        let expires = condition(a, "raisedWallExpires");
        if expires != 0.0 && expires <= r.tick {
            let i = a.conditions.get("raisedWallIndex").copied();
            let plane = a.conditions.get("raisedWallPlane").copied()
                .map(|v| v == 1.).unwrap_or_else(|| crate::dreamwalking::dream_actor(r, a));
            crate::dreamwalking::with_plane(r, plane, |r| {
                if let Some(i) = i {
                    let revision = r.simulation_revision.value_or(29.);
                    let f = r.floor_mut().unwrap();
                    if f.tiles.get(i as usize).is_some_and(|s| s == "wall") {
                        f.tiles[i as usize] = "floor".into();
                        crate::ichor::enqueue_opening_revision(f, i as usize, revision);
                        let w=f.width();
                        crate::events::at(r,"E40","fracture-stone",crate::events::Stage::Special,&[crate::events::EventPoint{x:(i as usize%w) as f64,y:(i as usize/w) as f64}]);
                    }
                }
                r.floor_mut().unwrap().dirty = true;
            });
            let a = actor_mut_trusted(r, id).unwrap();
            a.conditions.shift_remove("raisedWallExpires");
            a.conditions.shift_remove("raisedWallIndex");
            a.conditions.shift_remove("raisedWallPlane");
        }
    }
    // Ordered aura refresh: O(A*F + A²*(A+R+P)), including linear actor_mut_trusted
    // lookups, rays R, powers P and plane lookup F. Snapshots cost O(A*actor-size).
    // First-use layer provisioning and shared-block detach add their COW costs.
    let bodies: Vec<_> = std::iter::once(&r.player)
        .chain(r.actors_at(r.depth))
        .cloned()
        .collect();
    for source in &bodies {
        if !source.dead.value_or(false) && source.hp > 0.0 && has(source, "silence-aura") {
            let plane = crate::dreamwalking::dream_actor(r, source);
            crate::dreamwalking::with_plane(r, plane, |r| {
                for target in &bodies {
                    if source.id != target.id
                        && !crate::revision50_combat::oblivious_blocks(r,target,Some(source))
                        && !target.dead.value_or(false)
                        && crate::dreamwalking::same_plane(r, source, target)
                        && (source.id == r.player.id || source.state == "ally")
                            != (target.id == r.player.id || target.state == "ally")
                        && distance(r, source, target) <= 3.0
                        && crate::vision::line_of_sight(r, source, target)
                    {
                        let fresh=actor(r,target.id).is_some_and(|a|condition(a,"auraSilenced")<=0.) && !previously_silenced.contains(&target.id.to_bits());
                        actor_mut_trusted(r, target.id)
                            .unwrap()
                            .conditions
                            .insert("auraSilenced".into(), 1.0);
                        if fresh { crate::animation_enemy::mark(r,target,"E52","crossed-ring-violet",crate::events::Stage::Special); }
                    }
                }
            });
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn horn_caster_matches_frozen_reference() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/horn-caster-v48.json")).unwrap();
        for c in cases.as_array().unwrap() {
            let mut r: Run = serde_json::from_value(c["initial"].clone()).unwrap();
            crate::events::begin();
            let target = c["target"].as_f64();
            let result = match c["operation"].as_str().unwrap() {
                "special" => special_action(&mut r, 10.0, target.unwrap()).unwrap(),
                "retreat" => crate::monster_caster::retreat(
                    &mut r,
                    10.0,
                    target,
                    c["movementOnly"].as_bool().unwrap(),
                )
                .unwrap(),
                "offscreen" => {
                    crate::offscreen::tick_inactive_floors(&mut r).unwrap();
                    true
                }
                "world" => {
                    world_turn(&mut r);
                    true
                }
                _ => {
                    tick_actor(&mut r, 10.0);
                    true
                }
            };
            assert_eq!(result, c["result"].as_bool().unwrap(), "{}", c["kind"]);
            let actual = serde_json::to_value(r).unwrap();
            if !crate::storage::canonical_equal(&actual, &c["expected"]) {
                std::fs::write(
                    "/tmp/horn-native.json",
                    serde_json::to_vec(&actual).unwrap(),
                )
                .unwrap();
                std::fs::write(
                    "/tmp/horn-source.json",
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

#[cfg(test)]
mod plane_tests {
    use super::*;
    use crate::dreamwalking as dream;

    fn room() -> Run {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 34.0.into();
        r.actors_at_mut(r.depth).clear();
        r.player.x = 8.; r.player.y = 8.; r.player.energy = 100.;
        r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
        r.next_id = 1000.;
        let f = r.floor_mut().unwrap();
        let w = f.width(); let h = f.height();
        for y in 1..h-1 { for x in 1..w-1 { f.tiles[y*w+x] = "floor".into(); } }
        f.environment = crate::Field::Missing;
        f.lights.fill(0.);
        r
    }
    fn sleep(r: &mut Run) {
        r.dreamwalking = dream::State { unlocked: true, ..Default::default() }.into();
        assert_eq!(dream::ability(r, dream::DREAMWALK_ID).unwrap(), Some(true));
    }
    fn enemy(r: &mut Run, id: f64, dream_plane: bool) {
        let mut a = r.player.clone();
        a.id = id; a.x = 10.; a.y = 8.; a.state = "hunting".into();
        a.abilities.clear(); a.active.clear(); a.conditions.clear();
        if dream_plane { a.conditions.insert("phantasmiiPlane".into(), 1.); }
        r.actors_at_mut(r.depth).push(a);
    }

    #[test]
    fn r34_target_permission_silence_and_sleeping_body_are_plane_local() {
        let mut r = room(); sleep(&mut r); enemy(&mut r, 20., false);
        actor_mut_trusted(&mut r, 20.).unwrap().abilities.push("enemy:beast:silence-aura".into());
        let body = dream::body(&r).unwrap().id;
        assert!(!permitted(&r, actor(&r, 20.).unwrap(), &r.player));
        assert!(permitted(&r, actor(&r, 20.).unwrap(), actor(&r, body).unwrap()));
        assert!(!permitted(&r, actor(&r, body).unwrap(), actor(&r, 20.).unwrap()));
        world_turn(&mut r);
        assert_eq!(condition(&r.player, "auraSilenced"), 0.);
        assert_eq!(condition(actor(&r, body).unwrap(), "auraSilenced"), 1.);
        assert!(spell_spend(&mut r, 0., 8.));
        assert!(!spell_spend(&mut r, body, 8.));
        actor_mut_trusted(&mut r, 20.).unwrap().conditions.insert("phantasmiiPlane".into(), 1.);
        world_turn(&mut r);
        assert_eq!(condition(&r.player, "auraSilenced"), 1.);
        assert_eq!(condition(actor(&r, body).unwrap(), "auraSilenced"), 0.);
        assert!(!spell_spend(&mut r, 0., 8.));
    }

    #[test]
    fn r34_uncast_requires_a_same_plane_observing_endpoint_not_union_visibility() {
        let mut r = room(); sleep(&mut r); enemy(&mut r, 20., false);
        r.player.active.push("grisha-uncast".into());
        assert!(crate::vision::visible_actor(&r, actor(&r, 20.).unwrap()));
        assert!(spell_spend(&mut r, 20., 8.));
        assert_eq!(r.player.energy, 100., "the visible sleeping body cannot cast Uncast");
        r.player.abilities.push(crate::half_here::ID.into());
        let clone = crate::half_here::create(&mut r, 9., 8.).unwrap();
        actor_mut_trusted(&mut r, clone).unwrap().conditions.insert("phantasmiiPlane".into(), 0.);
        assert!(!spell_spend(&mut r, 20., 8.));
        assert_eq!(r.player.energy, 90.);
        actor_mut_trusted(&mut r, clone).unwrap().x = 60.;
        assert!(crate::vision::visible_actor(&r, actor(&r, 20.).unwrap()));
        assert!(spell_spend(&mut r, 20., 8.));
        assert_eq!(r.player.energy, 90., "opposite-plane sight cannot borrow a distant eligible clone");
        actor_mut_trusted(&mut r, clone).unwrap().x = 9.;
        actor_mut_trusted(&mut r, clone).unwrap().hp = 0.;
        assert!(spell_spend(&mut r, 20., 8.));
        assert_eq!(r.player.energy, 90.);
    }

    #[test]
    fn r34_dream_projection_uncast_and_aura_use_dream_geometry() {
        let mut r = room(); sleep(&mut r); enemy(&mut r, 20., true);
        r.player.active.push("grisha-uncast".into());
        assert!(!spell_spend(&mut r, 20., 8.));
        assert_eq!(r.player.energy, 90.);
        actor_mut_trusted(&mut r, 20.).unwrap().abilities.push("enemy:beast:silence-aura".into());
        let w = r.floor().unwrap().width();
        r.floor_mut().unwrap().tiles[8*w+9] = "wall".into();
        world_turn(&mut r);
        assert_eq!(condition(&r.player, "auraSilenced"), 1., "a waking wall cannot block a dream aura");
        assert!(!dream::context_dream(&r));
    }

    #[test]
    fn r34_horn_destination_occupancy_uses_actor_or_context_plane() {
        let mut r = room(); sleep(&mut r); enemy(&mut r, 20., false);
        let a = r.player.clone();
        assert!(free(&r, 10., 8., Some(&a)));
        assert!(!free(&r, 10., 8., None));
        assert!(dream::with_plane(&mut r, true, |r| free(r, 10., 8., None)));
        actor_mut_trusted(&mut r, 20.).unwrap().conditions.insert("phantasmiiPlane".into(), 1.);
        assert!(!free(&r, 10., 8., Some(&a)));
    }

    #[test]
    fn r34_stone_seal_expires_on_creation_plane_after_caster_changes_plane() {
        let mut r = room(); enemy(&mut r, 20., false);
        actor_mut_trusted(&mut r, 20.).unwrap().abilities.push("enemy:beast:wall-seal".into());
        assert!(special_action(&mut r, 20., 0.).unwrap());
        let i = condition(actor(&r, 20.).unwrap(), "raisedWallIndex") as usize;
        assert_eq!(condition(actor(&r, 20.).unwrap(), "raisedWallPlane"), 0.);
        sleep(&mut r);
        actor_mut_trusted(&mut r, 20.).unwrap().conditions.insert("phantasmiiPlane".into(), 1.);
        dream::with_plane(&mut r, true, |r| r.floor_mut().unwrap().tiles[i] = "wall".into());
        r.tick += 5.; world_turn(&mut r);
        assert_eq!(r.floor().unwrap().tiles[i], "floor");
        assert_eq!(dream::floor_for(&r, &r.player).unwrap().tiles[i], "wall");
        assert!(!actor(&r, 20.).unwrap().conditions.contains_key("raisedWallPlane"));
        assert!(!dream::context_dream(&r));
    }
}
