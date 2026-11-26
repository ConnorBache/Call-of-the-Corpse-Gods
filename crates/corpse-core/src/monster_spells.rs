//! Authored Bloodmancer, Witch and Dryad spell phases and target ranking.
use crate::{
    Actor, Run,
    engine::{self, actor, actor_mut_trusted, condition},
    movement::DIRECTIONS,
};
fn has(a: &Actor, t: &str) -> bool {
    a.abilities.iter().any(|s| s == &format!("enemy:beast:{t}"))
}
fn distance(r: &Run, a: &Actor, b: &Actor) -> f64 {
    crate::portals::actor_distance(r, a, b)
}
fn report(r: &mut Run, a: &Actor, b: &Actor, message: &str) {
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

/// Attribute a real heal without giving away an endpoint the player cannot see.
fn report_heal(r: &mut Run, caster: &Actor, target: &Actor) {
    if r.player.x < 0. { return; }
    let caster_is_player = caster.id == r.player.id;
    let target_is_player = target.id == r.player.id;
    let sees_caster = caster_is_player || crate::vision::visible_actor(r, caster);
    let sees_target = target_is_player || crate::vision::visible_actor(r, target);
    if !sees_caster && !sees_target { return; }
    let source = if caster_is_player { "You" } else if sees_caster {
        crate::actor_presentation::name(r, caster)
    } else { "Something unseen" };
    let message = if target_is_player {
        format!("{source} healed you.")
    } else if sees_target {
        let name = crate::actor_presentation::name(r, target);
        if crate::half_here::is_clone(target) {
            format!("{source} healed {name}.")
        } else {
            format!("{source} healed the {name}.")
        }
    } else {
        format!("{source} {} a healing spell.", if caster_is_player { "cast" } else { "casts" })
    };
    engine::log(r, message, if caster_is_player { "player" } else { "enemy" });
}
fn pull_one(r: &mut Run, a: &Actor, b: &Actor) -> Result<bool, String> {
    if crate::revision50_combat::oblivious_blocks(r,b,Some(a)) {return Ok(false);}
    let mut chosen = None;
    let mut best = f64::INFINITY;
    for d in DIRECTIONS {
        let x = b.x + d.x as f64;
        let y = b.y + d.y as f64;
        if !crate::travel::passable(r, b, x, y)
            || std::iter::once(&r.player)
                .chain(r.actors_at(r.depth))
                .any(|other| {
                    other.id != b.id
                        && !other.dead.value_or(false)
                        && other.hp > 0.0
                        && crate::dreamwalking::same_plane(r, b, other)
                        && crate::portals::same_position(r, other, x, y)
                })
        {
            continue;
        }
        let arrivals=crate::portals::step_options(r,b,x,y);
        if !arrivals.iter().all(|&(rx,ry)|crate::travel::passable(r,b,rx,ry)){continue}
        let dist=arrivals.iter().map(|&(rx,ry)|crate::portals::point_distance(r,a,rx,ry)).fold(0.,f64::max);
        if dist < distance(r, a, b) && dist < best {
            best = dist;
            chosen = Some(d)
        }
    }
    let Some(d) = chosen else { return Ok(false) };
    let previous = b.conditions.get("forcedMove").copied();
    actor_mut_trusted(r, b.id)
        .unwrap()
        .conditions
        .insert("forcedMove".into(), 1.0);
    let result = engine::move_actor(r, b.id, d.x as f64, d.y as f64, false).map_err(|e| e.0);
    if let Some(target) = engine::actor_any_mut(r, b.id) {
        if let Some(previous) = previous {
            target.conditions.insert("forcedMove".into(), previous);
        } else {
            target.conditions.shift_remove("forcedMove");
        }
    }
    result
}
pub fn special_action(r: &mut Run, id: f64, target: f64) -> Result<bool, String> {
    let a = actor(r, id).ok_or("Spell actor missing")?.clone();
    let b = actor(r, target).ok_or("Spell target missing")?.clone();
    if (!has(&a, "bloodmancy") && !has(&a, "witch"))
        || a.dead.value_or(false)
        || a.hp <= 0.0
        || b.dead.value_or(false)
        || b.hp <= 0.0
        || !crate::monster_horn::permitted(r, &a, &b)
        || !crate::vision::line_of_sight(r, &a, &b)
    {
        return Ok(false);
    }
    if crate::projectiles::possible_shot_range(r,&a,&b,7.).is_none()
        || r.tick < condition(&a, "monsterSpellReadyAt")
    {
        return Ok(false);
    }
    if !crate::monster_horn::spell_spend(r, id, 8.0) {
        return Ok(false);
    }
    let actual=crate::projectiles::trace_committed_from_actor(r,&a,b.x,b.y);
    let phase = condition(&a, "monsterSpellPhase");
    let actual_range=crate::projectiles::traveled_distance(r,&a,b.x,b.y,&actual);
    let Some((target,b))=actual.actor_id.filter(|_|actual_range<=7.).and_then(|id|actor(r,id).cloned().map(|b|(id,b)))else{
        let tick=r.tick;let caster=actor_mut_trusted(r,id).unwrap();
        caster.conditions.insert("monsterSpellPhase".into(),if has(&a,"bloodmancy"){phase+1.}else{phase%3.+1.});
        caster.conditions.insert("monsterSpellReadyAt".into(),tick+3.);
        caster.conditions.insert("rangedRecovery".into(),1.);
        crate::events::actor_event(r,&a,"projectile","Spell dissipates",Some(crate::events::EventPoint{x:actual.x,y:actual.y}),None);
        return Ok(true)
    };
    // A spell has reached this actual actor after projectile routing. Preserve
    // payment, phase advancement and recovery without applying biological state.
    if crate::revision50_nightmares::strike(r, target) {
        let tick = r.tick;
        let caster = actor_mut_trusted(r,id).unwrap();
        caster.conditions.insert("monsterSpellPhase".into(), if has(&a,"bloodmancy") { phase + 1. } else { phase % 3. + 1. });
        caster.conditions.insert("monsterSpellReadyAt".into(), tick + 3.);
        caster.conditions.insert("rangedRecovery".into(), 1.);
        crate::events::actor_event(r,&a,"projectile","Hallucination dissolves",Some((&b).into()),None);
        return Ok(true);
    }
    if has(&a, "bloodmancy") {
        if phase % 2.0 == 0.0 {
            let cast_depth = r.depth;
            let source_seen = crate::vision::visible_actor(r, &a);
            let pulled = pull_one(r, &a, &b)?;
            let message = if pulled {
                "wrests its prey one square closer with blood magic."
            } else {
                "casts Blood Wrest, but there is no room to pull its prey closer."
            };
            if r.depth == cast_depth {
                if let Some(after)=actor(r,target) {
                    if after.x!=b.x || after.y!=b.y {crate::events::cue(r,after,"E04","root-crimson",crate::events::Stage::Special,&[(&b).into(),after.into()]);}
                    else if !pulled {crate::animation_enemy::mark(r,after,"E04","rejected-crimson",crate::events::Stage::Special);}
                }
                let b = actor(r, target).unwrap_or(&b).clone();
                report(r, &a, &b, message);
                crate::events::actor_event(r, &a, "projectile", "Blood wrest", Some((&b).into()), None);
            } else if a.id == r.player.id || b.id == r.player.id || source_seen {
                // The forced fall was already presented by travel. Use the cast's
                // origin-floor knowledge for the log, never its map coordinates.
                let name = if a.id == r.player.id { "You" } else if source_seen { &a.name } else { "Something unseen" };
                engine::log(r,format!("{name} {message}"),if a.id==r.player.id {"player"} else {"enemy"});
            }
        } else {
            crate::monster_statuses::apply_curse(r, target, "hemophilia", 20.0, id);
            report(r, &a, &b, "curses its prey with hemophilia.");
            if crate::events::enabled(r) {crate::events::actor_event(r,&a,"projectile","Hemophilia",Some((&b).into()),None);}
        }
        // Arrival may change the active floor before this paid spell settles.
        // Use the caster's indexed identity even if it remained on the origin floor.
        let tick = r.tick;
        if let Some(caster) = engine::actor_any_mut(r,id) {
            caster.conditions.insert("monsterSpellPhase".into(),phase+1.);
            caster.conditions.insert("monsterSpellReadyAt".into(),tick+3.);
            caster.conditions.insert("rangedRecovery".into(),1.);
        }
        return Ok(true);
    } else {
        let phase = phase % 3.0;
        let (key, label, duration) = if phase == 0.0 {
            ("slowed", "Slowed", 5.0)
        } else if phase == 1.0 {
            ("hemophilia", "Hemophilia", 20.0)
        } else {
            ("poisoned", "Poisoned", 20.0)
        };
        crate::monster_statuses::apply_curse(r, target, key, duration, id);
        actor_mut_trusted(r, id)
            .unwrap()
            .conditions
            .insert("monsterSpellPhase".into(), phase + 1.0);
        report(
            r,
            &a,
            &b,
            &format!(
                "casts {} on its prey.",
                if phase == 0.0 {
                    "slowness"
                } else if phase == 1.0 {
                    "hemophilia"
                } else {
                    "poison"
                }
            ),
        );
        crate::events::actor_event(r, &a, "projectile", label, Some((&b).into()), None);
    }
    let tick = r.tick;
    let a = actor_mut_trusted(r, id).unwrap();
    a.conditions
        .insert("monsterSpellReadyAt".into(), tick + 3.0);
    a.conditions.insert("rangedRecovery".into(), 1.0);
    Ok(true)
}
/// One stable lowest-health-ratio scan with source actor-ID tie ordering.
pub fn heal_others(r: &mut Run, id: f64) -> Result<bool, String> {
    let a = actor(r, id).ok_or("Dryad missing")?.clone();
    if r.tick < condition(&a, "monsterHealReadyAt")
        || (crate::dreamwalking::enabled(r) && crate::dreamwalking::is_body(&a))
    {
        return Ok(false);
    }
    let aligned = |b: &Actor| {
        !crate::revision50_control_amulets::hostile(r,&a,b)
    };
    let mut target: Option<Actor> = None;
    for b in std::iter::once(&r.player).chain(r.actors_at(r.depth)) {
        if b.id == id
            || b.dead.value_or(false)
            || b.hp <= 0.0
            || !crate::dreamwalking::same_plane(r, &a, b)
            || !aligned(b)
            || b.hp >= b.max_hp
            || crate::divine_environment::healing_suppressed(r, b)
            || distance(r, &a, b) > 6.0
            || !crate::vision::line_of_sight(r, &a, b)
            || (crate::events::enabled(r) && crate::projectiles::possible_shot_range(r,&a,b,6.).is_none())
        {
            continue;
        }
        if target.as_ref().is_none_or(|c| {
            b.hp / b.max_hp < c.hp / c.max_hp || (b.hp / b.max_hp == c.hp / c.max_hp && b.id < c.id)
        }) {
            target = Some(b.clone());
        }
    }
    let Some(b) = target else { return Ok(false) };
    if !crate::monster_horn::spell_spend(r, id, 5.0) {
        return Ok(false);
    }
    if crate::events::enabled(r) {
        let landed=crate::projectiles::trace_committed_from_actor(r,&a,b.x,b.y);
        let cells=crate::events::take_path(&a);let serial=crate::events::emission_serial();
        crate::events::travel(r,&a,"E17","line-grow",&cells,crate::events::Delivery::Magic,None);
        // Keep travel metadata clipped to observable terrain as well as its cells.
        if let Some(end)=cells.iter().rev().find(|p|crate::vision::directly_visible_tile(r,p.x as usize,p.y as usize)) {
            crate::events::decorate(serial,"E17","line-grow",end.clone(),"#40ff60",Some((&a).into()));
        }
        if landed.actor_id!=Some(b.id) {
            let ready=r.tick+2.;actor_mut_trusted(r,id).unwrap().conditions.insert("monsterHealReadyAt".into(),ready);
            return Ok(true);
        }
    }
    let owner = crate::half_here::owner_id(r, &b);
    let heal_amount = crate::balance52::local_amount(r, &a, "monsterHealAmount", 25.);
    let recipient = actor_mut_trusted(r, owner).unwrap();
    let before_hp=recipient.hp;
    recipient.hp = (recipient.hp + heal_amount).min(recipient.max_hp);
    let healed=recipient.hp>before_hp;
    crate::half_here::sync(r);
    let tick = r.tick;
    actor_mut_trusted(r, id)
        .unwrap()
        .conditions
        .insert("monsterHealReadyAt".into(), tick + 2.0);
    let b = actor(r, b.id).unwrap_or(&b).clone();
    if r.simulation_revision.value_or(29.) < 52. {
        report(r, &a, &b, "restores an injured companion.");
    } else if healed {
        report_heal(r, &a, &b);
    }
    if crate::events::enabled(r) {
        if healed {
            crate::animation_enemy::mark(r,&b,"E17","stitch-green",crate::events::Stage::Health);
        }
    } else {crate::events::actor_event(r, &a, "heal", "Healing", Some((&b).into()), None);}
    Ok(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dream_plane_dryad_heals_only_a_native_endpoint() {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 34.0.into();
        r.player.hp = 50.; r.player.max_hp = 100.; r.player.conditions.clear();
        r.dreamwalking = crate::dreamwalking::State { active: true, depth: 1., ..Default::default() }.into();
        let (x, y) = (r.player.x, r.player.y);
        let healer = r.actors_at_mut(1.).first_mut().unwrap();
        healer.x = x + 1.; healer.y = y; healer.state = "ally".into();
        healer.hp = healer.max_hp; healer.energy = 100.; healer.conditions.clear();
        healer.abilities = vec!["enemy:beast:dryad".into()];
        let id = healer.id;
        assert!(!heal_others(&mut r, id).unwrap());
        assert_eq!(r.player.hp, 50.);
        assert_eq!(actor(&r, id).unwrap().energy, 100.);
        let mut body = r.player.clone(); body.id = 90.; body.state = "ally".into();
        body.conditions.insert("phantasmiiBody".into(), 1.);
        body.conditions.insert("halfHereOwner".into(), r.player.id);
        r.dreamwalking.as_mut().unwrap().body_id = Some(body.id);
        r.actors_at_mut(1.).push(body);
        assert!(heal_others(&mut r, id).unwrap());
        assert_eq!(r.player.hp, 75.);
        assert_eq!(actor(&r, 90.).unwrap().hp, 75.);
        assert_eq!(actor(&r, id).unwrap().energy, 95.);
    }
    #[test]
    fn caster_spells_match_frozen_reference() {
        let cases: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/caster-spells-v48.json")).unwrap();
        for c in cases.as_array().unwrap() {
            let mut r: Run = serde_json::from_value(c["initial"].clone()).unwrap();
            crate::events::begin();
            let result = if c["operation"] == "heal" {
                crate::monster_caster::turn(&mut r, 10.0, None, false).unwrap()
            } else {
                let p = r.player.id;
                special_action(&mut r, 10.0, p).unwrap()
            };
            assert_eq!(result, c["result"].as_bool().unwrap(), "{}", c["kind"]);
            let actual = serde_json::to_value(r).unwrap();
            if !crate::storage::canonical_equal(&actual, &c["expected"]) {
                std::fs::write(
                    "/tmp/spells-native.json",
                    serde_json::to_vec(&actual).unwrap(),
                )
                .unwrap();
                std::fs::write(
                    "/tmp/spells-source.json",
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
