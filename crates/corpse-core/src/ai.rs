//! Stable actor phase snapshots. Normal order preserves floor order; haste sorts IDs first.
use crate::model::Actor;
pub fn phase_order(actors: &[Actor], haste: bool) -> Vec<f64> {
    let mut indices: Vec<_> = (0..actors.len()).collect();
    if haste {
        indices.sort_by(|a, b| actors[*a].id.total_cmp(&actors[*b].id));
    }
    indices
        .iter()
        .filter(|i| actors[**i].state == "ally")
        .chain(indices.iter().filter(|i| actors[**i].state != "ally"))
        .filter(|i| !crate::revision50_nightmares::is_decoy(&actors[**i]))
        .map(|i| actors[*i].id)
        .collect()
}
pub fn is_hasted(a: &Actor) -> bool {
    a.conditions.get("slowed").copied().unwrap_or(0.0) <= 0.0
        && (a.conditions.get("hasted").copied().unwrap_or(0.0) > 0.0
            || a.abilities.iter().any(|x| x == "enemy:beast:fast")
            || crate::monster_vampire::flying(a)
            || a.active.iter().any(|id| id == "etill-rushing-torrent"))
}
pub fn is_hasted_in(r: &crate::Run, a: &Actor) -> bool {
    if a.conditions.get("slowed").copied().unwrap_or(0.0) > 0.0 {
        return false;
    }
    (if r.simulation_revision.value_or(29.) >= 52. {
        condition(a, "hasted") > 0. || a.active.iter().any(|id| id == "etill-rushing-torrent")
    } else { is_hasted(a) })
        || crate::revision52_swimming::grants_haste(r, a)
        || crate::balance_rules::sleep_walk_active(r, a)
        || (a
            .abilities
            .iter()
            .any(|id| id == "dawel-step-through-shadows")
            && !r.actors_at(r.depth).iter().any(|b| {
                !b.dead.value_or(false)
                    && b.hp > 0.0
                    && b.state != "ally"
                    && b.state != "neutral"
                    && b.conditions.get("seesPlayer") == Some(&1.0)
                    && crate::vision::detection_chance(r, b, a) > 0.0
            }))
}
/// Innate movement speed is not a second complete action. Vampire Bat form
/// retains its existing movement benefit without being reported as Haste.
pub fn is_fast(a: &Actor) -> bool {
    a.abilities.iter().any(|id| id == "enemy:beast:fast") || crate::monster_vampire::flying(a)
}
pub fn fast_movement_enabled(r: &crate::Run, a: &Actor) -> bool {
    r.simulation_revision.value_or(29.) >= 52. && is_fast(a)
        && condition(a, "slowed") <= 0.
        && !a.abilities.iter().any(|id| id == "enemy:beast:slow" || id == "enemy:pale:slow")
}
use crate::{Field, Run, combat, engine::condition, vision};
pub fn sleeping(a: &Actor) -> bool {
    a.state == "sleeping"
        || (a.state == "ally" && condition(a, "monsterAllyRest") == 1.0)
        || (a.state == "neutral" && condition(a, "monsterRecoverySleep") == 1.0)
}
pub fn wake(a: &mut Actor, restart: bool) {
    if a.state == "sleeping" {
        a.state = "wandering".into();
    }
    a.conditions.shift_remove("monsterRecoverySleep");
    a.conditions.shift_remove("monsterAllyRest");
    if restart && a.state != "ally" && condition(a, "halfHereClone") != 1.0 {
        a.conditions.insert("monsterUnseenTurns".into(), 0.0);
    }
}
pub fn advance_rest(r: &Run, a: &mut Actor, detected: bool) -> bool {
    if a.id == r.player.id
        || a.dead.value_or(false)
        || a.hp <= 0.0
        || (crate::monster_inventor::is_mech(a)
            && !a
                .monster_state
                .as_ref()
                .is_some_and(|s| s.pilot.as_ref().is_some()))
    {
        return false;
    }
    if a.state != "ally" {
        a.conditions.shift_remove("monsterAllyRest");
    }
    if a.state != "sleeping" && a.state != "neutral" {
        a.conditions.shift_remove("monsterRecoverySleep");
    }
    let old = condition(a, "monsterUnseenTurns").clamp(0.0, 20.0);
    if a.state != "ally" {
        a.conditions.insert(
            "monsterUnseenTurns".into(),
            if detected { 0.0 } else { (old + 1.0).min(20.0) },
        );
    }
    let cap = combat::effective_stats(r, a).max_energy;
    if !sleeping(a) {
        if a.state == "ally"
            || (crate::revision51_perception::enabled(r)
                && matches!(a.state.as_str(),"hunting"|"stalking") && a.last_seen.as_ref().is_some())
            || detected
            || a.energy >= cap
            || condition(a, "monsterUnseenTurns") < 20.0
        {
            return false;
        }
        if a.state != "neutral" {
            a.state = "sleeping".into();
        }
        a.conditions.insert("monsterRecoverySleep".into(), 1.0);
        a.last_seen = Field::Missing;
        for k in [
            "targetId",
            "chaseDepth",
            "chaseExitX",
            "chaseExitY",
            "chaseLandingX",
            "chaseLandingY",
            "chaseChasm",
            "stalkGoalX",
            "stalkGoalY",
            "stalkObservedX",
            "stalkObservedY",
            "stalkDx",
            "stalkDy",
            "stalkWait",
            "stalkPeekTick",
        ] {
            a.conditions.shift_remove(k);
        }
        a.conditions.insert("seesPlayer".into(), 0.0);
    }
    if a.energy < cap && !crate::divine_environment::healing_suppressed(r, a) {
        a.energy = (a.energy + 1.0).min(cap);
    }
    if condition(a, "monsterRecoverySleep") == 1.0 && a.energy >= cap {
        wake(a, false);
    }
    true
}
pub fn nearest_ally_enemy<'a>(r: &'a Run, a: &Actor) -> Option<&'a Actor> {
    r.actors_at(r.depth)
        .iter()
        .filter(|b| {
            !b.dead.value_or(false)
                && crate::dreamwalking::same_plane(r,a,b)
                && b.state != "ally"
                && b.state != "neutral"
                && if crate::revision51_perception::enabled(r) {
                    vision::plausibly_visible_actor(r,a,b)
                } else { condition(a, "telepathic") > 0.0
                    || (condition(b, "invisible") <= 0.0 && vision::line_of_sight(r, a, b)) }
        })
        .min_by(|x, y| {
            let dx = crate::portals::actor_distance(r, a, x);
            let dy = crate::portals::actor_distance(r, a, y);
            dx.total_cmp(&dy).then(x.id.total_cmp(&y.id))
        })
}
pub fn should_wake_ally(r: &Run, a: &Actor) -> bool {
    if a.state != "ally" {
        return false;
    }
    if nearest_ally_enemy(r, a).is_some()
        || r.actors_at(r.depth).iter().any(|b| {
            !b.dead.value_or(false)
                && b.hp > 0.0
                && b.state != "ally"
                && b.state != "neutral"
                && vision::visible_actor(r, b)
        })
    {
        return true;
    }
    if a.perfect.value_or(false) && a.order.as_ref().is_some_and(|s| s == "p") {
        return r.rally.as_ref().is_some_and(|p| p.x != a.x || p.y != a.y);
    }
    if !r.player.dead.value_or(false)
        && (crate::portals::actor_chebyshev(r, a, &r.player) >= 3.0
            || !vision::line_of_sight(r, a, &r.player))
    {
        return true;
    }
    a.abilities.iter().any(|s| s == "enemy:beast:dryad")
        && a.energy >= 5.0
        && r.tick >= condition(a, "monsterHealReadyAt")
        && std::iter::once(&r.player)
            .chain(r.actors_at(r.depth))
            .any(|b| {
                b.id != a.id
                    && !b.dead.value_or(false)
                    && b.hp > 0.0
                    && (b.id == r.player.id || b.state == "ally")
                    && b.hp < b.max_hp
                    && !crate::divine_environment::healing_suppressed(r, b)
                    && crate::portals::actor_distance(r, &a, &b) <= 6.0
                    && vision::line_of_sight(r, a, b)
            })
}

pub fn maximum_ability_cost(a: &Actor) -> f64 {
    crate::enemy_traits::maximum_cost(a)
}
pub fn enemy_action(r: &mut Run, id: f64, target: f64) -> Result<bool, crate::engine::EngineError> {
    enemy_action_options(r, id, target, true)
}
/// Birth branch admission adds O(d) clean or O(A*d + F + H) dirty registry work
/// for the birth batch (at most two); successful action/RNG order is unchanged.
pub fn enemy_action_options(
    r: &mut Run,
    id: f64,
    target: f64,
    allow_approach_blink: bool,
) -> Result<bool, crate::engine::EngineError> {
    use crate::engine::{actor, actor_mut_trusted, enemy_spend, log};
    let a = actor(r, id).unwrap().clone();
    let b = actor(r, target).unwrap().clone();
    if crate::revision50_nightmares::is_decoy(&a) { return Ok(false); }
    let discord = crate::revision50_control_amulets::discord_hostility(r,&a,&b);
    if !crate::dreamwalking::same_plane(r,&a,&b) || crate::dreamwalking::inert(r,&a) || r.status != "playing"
        || condition(&r.player, "deathResolutionBlocked") != 0.0
        || a.dead.value_or(false)
        || b.dead.value_or(false)
        || a.hp <= 0.0
        || b.hp <= 0.0
        || condition(&a, "paralyzed") > 0.0
        || sleeping(&a)
        || (!discord && (a.state == "wandering" || a.state == "neutral"
            || (a.perfect.value_or(false) && a.order.as_ref().is_some_and(|s| s == "p"))))
        || id == target
        || !crate::revision50_control_amulets::hostile(r,&a,&b)
        || (condition(&b, "invisible") > 0.0 && condition(&a, "telepathic") <= 0.0)
        || !vision::line_of_sight(r, &a, &b)
    {
        return Ok(false);
    }
    let revision=r.simulation_revision.value_or(29.);
    crate::enemy_traits::initialize_enemy_traits_revision(actor_mut_trusted(r, id).unwrap(),revision);
    let a = actor(r, id).unwrap().clone();
    if crate::monster_grab::deep_water(r, id, target) {
        return Ok(true);
    }
    if a.abilities.iter().any(|id| id == "enemy:beast:fire-breath")
        && (!crate::monster_inventor::is_mech(&a)
            || a.monster_state
                .as_ref()
                .is_some_and(|s| s.pilot.as_ref().is_some()))
    {
        let landing = crate::projectiles::trace_from_actor(r, &a, b.x, b.y);
        if crate::projectiles::reaches_actor(r, &landing, &b)
            && crate::monster_inventor::breathe_fire(r, id, target)
                .map_err(crate::engine::EngineError)?
        {
            return Ok(true);
        }
    }
    if crate::witch::summon(r, id, target).map_err(crate::engine::EngineError)? {
        return Ok(true);
    }
    if crate::saidribee_queen::summon(r, id, target).map_err(crate::engine::EngineError)? {
        return Ok(true);
    }
    if crate::monster_spells::special_action(r, id, target).map_err(crate::engine::EngineError)?
        || crate::monster_grab::hold_action(r, id, target)
    {
        return Ok(true);
    }
    if !a.abilities.iter().any(|s| s == "enemy:beast:hold-only")
        && (crate::monster_bombardier::special_action(r, id, target)
            .map_err(crate::engine::EngineError)?
            || crate::monster_moon::special_action(r, id, target, allow_approach_blink)
                .map_err(crate::engine::EngineError)?
            || crate::monster_grab::water_jet(r, id, target).map_err(crate::engine::EngineError)?
            || crate::monster_ruin::special_action(r, id, target)
                .map_err(crate::engine::EngineError)?
            || crate::monster_horn::special_action(r, id, target)
                .map_err(crate::engine::EngineError)?)
    {
        return Ok(true);
    }
    if a.abilities.iter().any(|id| id == "enemy:pale:breath-boost") && enemy_spend(r, id, 10.0) {
        let i = a.y as usize * r.floor().unwrap().width() + a.x as usize;
        let gas = crate::environment::breather_gas(r, &a);
        // The paid cast deposits in one cell; propagation is a later world step.
        // Observe that committed delta, never invent a radius around the caster.
        let before=crate::animation_enemy::gas_cell(r, i);
        let before=(before.0==Some(gas),before.1);
        let deposited=crate::environment::add_gas(
            r.floor_mut().unwrap(),
            i,
            gas,
            a.conditions.get("monsterGasBoostVolume").copied().unwrap_or_else(||
                if a.name.starts_with("Greater ") { 160.0 } else { 80.0 }),
            id,
        )
        .map_err(crate::engine::EngineError)?;
        if a.id == r.player.id || vision::visible_actor(r, &a) {
            log(
                r,
                format!("{} exhales a billowing cloud of {gas} gas.", a.name),
                "enemy",
            );
        }
        if crate::events::enabled(r) {
            if deposited {crate::animation_enemy::gas_deposited(r,&a,i,before,gas,"E57");}
        } else {
            crate::events::actor_event(r, &a, "burst", if gas == "hallucinogenic" { "Hallucinogenic gas" } else { "Caustic gas" }, None, None);
        }
        return Ok(true);
    }
    let scream_radius = crate::screamer_rules::RADIUS;
    if a.abilities.iter().any(|id| id == "enemy:pale:scream")
        && !crate::screamer_schedule::blocked(r, &a)
        && crate::screamer_rules::can_reach_player(r, &a)
        && enemy_spend(r, id, 15.0) {
        crate::animation_enemy::wave(r,&a,"E65",true,Some(scream_radius),false,false);
        let wave_action=crate::events::current_action();
        let multiplier = if a.name.starts_with("Greater ") {
            2.0
        } else {
            1.0
        };

        let ids: Vec<_> = std::iter::once(&r.player)
            .chain(r.actors_at(r.depth))
            .filter(|b| !b.dead.value_or(false) && b.hp > 0.0 && crate::dreamwalking::same_plane(r,&a,b))
            .map(|b| b.id)
            .collect();
        let player = r.player.id;
        let disorient = crate::water_blood::enabled(r);
        let mut scream_duration: f64 = 0.;
        for other in ids {
            let recipient=actor(r,other).unwrap();
            if crate::revision50_nightmares::is_decoy(recipient)
                || crate::revision50_combat::oblivious_blocks(r,recipient,Some(&a)) {continue;}
            let nearby = crate::portals::actor_distance(r, &a, recipient) <= scream_radius;
            let same = if crate::revision50::enabled(r) {!crate::revision50_control_amulets::hostile(r,&a,recipient)}
                else {(other==player||recipient.state=="ally")== (a.state=="ally")};
            let duration = crate::monster_statuses::status_duration(r,recipient,5.0);
            let b = actor_mut_trusted(r, other).unwrap();
            if nearby {
                scream_duration = scream_duration.max(duration);
                if same {
                    b.conditions.insert("screamEmboldened".into(), duration);
                    b.conditions.insert("screamBonus".into(), 5.0 * multiplier);
                    b.conditions
                        .insert("screamPerception".into(), 20.0 * multiplier);
                } else {
                    if disorient {
                        b.conditions.shift_remove("screamWeakened");
                        b.conditions.shift_remove("screamPenalty");
                        b.conditions.insert("disoriented".into(), duration);
                        b.conditions.insert("disorientedPenalty".into(), 5.0 * multiplier);
                    } else {
                        b.conditions.insert("screamWeakened".into(), duration);
                        b.conditions.insert("screamPenalty".into(), 5.0 * multiplier);
                    }
                }
            }
            let show=nearby;
            if same && other != player && b.state != "ally" {
                wake(b, true);
                b.state = "hunting".into();
                b.last_seen = Field::Value(crate::ActorLastSeen { x: a.x, y: a.y });
            }
            let b=b.clone(); if show { crate::animation_enemy::mark(r,&b,"E65",if same {"diamond-warm"}else{"crossed-ring-violet"},crate::events::Stage::Special); }
        }
        crate::screamer_schedule::after_scream(r,id,scream_duration);
        if !crate::events::enabled(r) {crate::events::actor_event(r, &a, "pulse", "Scream", None, None);}
        crate::events::checkpoint_for_action(r,wave_action);
        if vision::visible_actor(r, &a) {
            log(
                r,
                format!(
                    "{} screams; nearby foes {} and its allies converge.",
                    a.name, if disorient { "become disoriented" } else { "weaken" }
                ),
                "enemy",
            );
        } else if crate::portals::actor_distance(r, &a, &r.player) <= scream_radius {
            log(r, if disorient { "An unseen creature screams; you feel disoriented." } else { "An unseen creature screams; you feel weakened." }, "enemy");
        }
        return Ok(true);
    }
    if a.abilities.iter().any(|id| id == "enemy:pale:spit") {
        let possible=crate::projectiles::possible_shot_range(r,&a,&b,12.);
        if possible.is_none_or(|range|range<=std::f64::consts::SQRT_2) {
            return Ok(false);
        }
        if !crate::monster_horn::permitted(r, &a, &b)
            || b.active.iter().any(|id| id == "morae-turning-the-wheel")
        {
            return Ok(true);
        }
        let multiplier = if a.name.starts_with("Greater ") {
            2.0
        } else {
            1.0
        };
        let (fixed, coefficient) = if combat::raw_attack_damage_enabled(r.simulation_revision.value_or(29.)) { (5., 0.5) } else { (10., 1.) };
        let base = (fixed + coefficient * combat::effective_stats(r, &a).offense) * multiplier;
        let landing=crate::projectiles::trace_committed_from_actor(r,&a,b.x,b.y);
    crate::events::tag_path(&a,"E61","glob");
        let range=crate::projectiles::traveled_distance(r,&a,b.x,b.y,&landing);
        if landing.actor_id.is_none() && crate::events::enabled(r) {crate::events::actor_event(r,&a,"projectile","Distant spit",Some(crate::events::EventPoint{x:landing.x,y:landing.y}),None);}
        if let Some(actual)=landing.actor_id.filter(|_|range<=12.){crate::engine::attack_ranged_resolved(r,id,actual,base,range,1.,false)?;}
        actor_mut_trusted(r, id)
            .unwrap()
            .conditions
            .insert("rangedRecovery".into(), 1.0);
        if a.abilities.iter().any(|id| id == "enemy:pale:gas-shot") && enemy_spend(r, id, 10.0) {
            if range>12.{return Ok(true)}
            let i = if r.simulation_revision.value_or(29.)>=36.{landing.y as usize*r.floor().unwrap().width()+landing.x as usize}else{b.y as usize*r.floor().unwrap().width()+b.x as usize};
            crate::environment::add_gas(
                r.floor_mut().unwrap(),
                i,
                "caustic",
                a.conditions.get("monsterGasShotVolume").copied().unwrap_or(100.0 * multiplier),
                id,
            )
            .map_err(crate::engine::EngineError)?;
            let w=r.floor().unwrap().width();
            crate::events::cue(r,&a,"E62","puff-green",crate::events::Stage::Special,&[crate::events::EventPoint{x:(i%w) as f64,y:(i/w) as f64}]);
        }
        return Ok(true);
    }
    if a.abilities.iter().any(|id| id == "enemy:pale:birth") {
        // Absolute world tick: haste opportunities never shorten a ten-turn wait.
        // Missing state is immediately ready; legacy replay revisions stay unchanged.
        if r.simulation_revision.value_or(29.0) >= 32.0
            && r.tick < condition(&a, "monsterSummonReadyAt")
        {
            return Ok(false);
        }
        let f = r.floor().unwrap();
        let w = f.width();
        let mut cells = Vec::new();
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let (x, y) = (a.x + dx as f64, a.y + dy as f64);
                if x < 0.0
                    || y < 0.0
                    || x >= w as f64
                    || y >= f.height() as f64
                    || crate::engine::bulwark_blocks(r, &a, x, y)
                    || (r.simulation_revision.value_or(29.)>=51. && a.state!="ally"
                        && (crate::revision51_stairs::stair_cell(f,y as usize*w+x as usize) || (r.simulation_revision.value_or(29.)>=52. && crate::borge::reserved_cell(f,y as usize*w+x as usize))))
                    || !matches!(
                        f.tiles[y as usize * w + x as usize].as_str(),
                        "floor" | "grass" | "up" | "down"
                    )
                    || (crate::portals::same_position(r, &r.player, x, y) && crate::dreamwalking::same_plane(r,&a,&r.player))
                    || r.actors_at(r.depth)
                        .iter()
                        .any(|b| !b.dead.value_or(false) && b.hp > 0.0 && crate::portals::same_position(r, b, x, y) && crate::dreamwalking::same_plane(r,&a,b))
                {
                    continue;
                }
                if !crate::portals::enabled(r) || !cells.iter().any(|&(cx, cy)| crate::portals::same_cell(f, cx, cy, x, y)) {
                    cells.push((x, y));
                }
            }
        }
        if cells.is_empty() {
            return Ok(false);
        }
        let twin_candidate = a.abilities.iter().any(|id| id == "enemy:pale:double-birth")
            && cells.len() >= 2;
        // Mirror enemy_spend's pure eligibility checks before any birth payment.
        let owner = actor(r, crate::half_here::owner_id(r, actor(r, id).unwrap())).unwrap();
        let birth_count = if twin_candidate && owner.energy >= 30.0
            && !crate::abilities::energy_activation_silenced(r, owner) { 2 } else { 1 };
        crate::creature_index::check_next_creature_ids(r, birth_count)
            .map_err(crate::engine::EngineError)?;
        let twins = twin_candidate && enemy_spend(r, id, 30.0);
        if !twins && !enemy_spend(r, id, 10.0) {
            return Ok(false);
        }
        let greater = a.name.starts_with("Greater ");
        let count = if twins { 2 } else { 1 };
        for _ in 0..count {
            let pick = (crate::rng::random(r) * cells.len() as f64).floor() as usize;
            let (x, y) = cells.remove(pick);
            if crate::portals::enabled(r) {
                let f = r.floor().unwrap();
                cells.retain(|&(cx, cy)| !crate::portals::same_cell(f, x, y, cx, cy));
            }
            let role = if greater {
                ["Biter", "Breather", "Spitter", "Birther", "Screamer"]
                    [(crate::rng::random(r) * 5.0).floor() as usize]
            } else {
                "Jackal"
            };
            let mut child =
                crate::population::create_actor_revision(&format!("pale:{role}"), r.next_id, x, y, false, r.simulation_revision.value_or(29.0))
                    .map_err(crate::engine::EngineError)?;
            r.next_id += 1.0;
            child.born_from_birther = true.into();
            if r.simulation_revision.value_or(29.0) >= 32.0 && a.heartless.value_or(false) {
                child.heartless = true.into();
            }
            child.state = if a.state == "ally" {
                "ally"
            } else {
                "wandering"
            }
            .into();
            if !greater {
                child.energy = 100.0;
                child.stats.offense = 4.0;
                child.stats.perception = 25.0;
                child.stats.max_energy = 100.0;
            }
            crate::physical_balance::apply_summoned_actor(
                &mut child, greater, &format!("pale:{role}"), r.simulation_revision.value_or(29.0),
            );
            let context = if greater { "greater-birther" } else { "birther" };
            crate::balance52::apply_new_actor_kind(&mut child, &format!("{context}:pale:{role}"), r.depth, r.simulation_revision.value_or(29.));
            let label = format!(
                "Birth {}",
                child.name.strip_prefix("Pale ").unwrap_or(&child.name)
            );
            if crate::events::enabled(r) {
                crate::animation_enemy::mark(r,&child,if twins {"E64"}else{"E63"},"membrane-ivory",crate::events::Stage::Special);
            }
            r.actors_at_mut(r.depth).push(child);
            if !crate::events::enabled(r) { crate::events::actor_event(
                r,
                &a,
                "summon",
                &label,
                Some(crate::events::EventPoint { x, y }),
                None,
            ); }
        }
        if r.simulation_revision.value_or(29.0) >= 32.0 {
            let ready_at = r.tick + 10.0;
            actor_mut_trusted(r, id).unwrap().conditions.insert("monsterSummonReadyAt".into(), ready_at);
        }
        r.floor_mut().unwrap().dirty = true;
        if vision::visible_actor(r, &a) {
            log(
                r,
                format!(
                    "{} births {}.",
                    a.name,
                    if greater {
                        if twins { "two Pales" } else { "a Pale" }
                    } else {
                        if twins { "two Jackals" } else { "a Jackal" }
                    }
                ),
                "enemy",
            );
        }
        return Ok(true);
    }
    Ok(a.abilities.iter().any(|id| id == "enemy:beast:rooted"))
}

pub fn recover_ranged(r: &mut Run, id: f64) -> bool {
    let a = crate::engine::actor(r, id).unwrap().clone();
    if condition(&a, "rangedRecovery") <= 0.0 {
        return false;
    }
    crate::engine::actor_mut_trusted(r, id)
        .unwrap()
        .conditions
        .shift_remove("rangedRecovery");
    if id == r.player.id || vision::visible_actor(r, &a) {
        crate::engine::log(
            r,
            format!("{} recovers after its ranged attack.", crate::actor_presentation::name(r, &a)),
            "enemy",
        );
    }
    crate::events::actor_event(r, &a, "status", "Recovering", None, None);
    true
}

pub fn enemy_hit_effects(r: &mut Run, id: f64, target: f64) {
    use crate::engine::{actor, actor_mut_trusted, log};
    let a = actor(r, id).unwrap().clone();
    let b = actor(r, target).unwrap().clone();
    if b.dead.value_or(false) || b.hp <= 0.0 {
        return;
    }
    crate::ichor::siphon_hit(r, id, target);
    let has = |kind: &str| a.abilities.iter().any(|id| id == kind);
    let strength = if has("enemy:beast:venom-strong") {
        4.0
    } else if has("enemy:beast:venom") {
        2.0
    } else if has("enemy:beast:venom-weak") {
        1.0
    } else {
        0.0
    };
    if strength > 0.0 {
        let duration = crate::monster_statuses::status_duration(r, &b, 5.0);
        let key = format!("venom:{}:{}", r.tick + duration, id);
        let stacked = r.simulation_revision.value_or(29.0) >= 52.0;
        let now = r.tick;
        let b = actor_mut_trusted(r, target).unwrap();
        if stacked {
            // O(C) time and O(1) scratch: one additive severity, one refreshed clock.
            let severity = b.conditions.iter().filter(|(k, _)| venom_expiry(k).is_some_and(|expiry| expiry >= now))
                .map(|(_, value)| value.max(0.0)).sum::<f64>();
            b.conditions.retain(|k, _| !k.starts_with("venom:"));
            b.conditions.insert(key, severity + strength);
        } else {
            *b.conditions.entry(key).or_insert(0.0) += strength;
        }
        b.conditions.insert("envenomed".into(), duration);
        let b = b.clone();
        if id == r.player.id
            || target == r.player.id
            || vision::visible_actor(r, &a)
            || vision::visible_actor(r, &b)
        {
            let subject = if id == r.player.id {
                "You"
            } else if vision::visible_actor(r, &a) {
                crate::actor_presentation::name(r, &a)
            } else {
                "Something unseen"
            };
            log(
                r,
                format!(
                    "{subject} injects {} with accumulating venom.",
                    if target == r.player.id {
                        "you"
                    } else {
                        "its prey"
                    }
                ),
                if id == r.player.id { "player" } else { "enemy" },
            );
        }
        crate::events::actor_event(r, &a, "status", "Envenomed", Some((&b).into()), None);
    }
}
fn venom_expiry(key: &str) -> Option<f64> {
    key.strip_prefix("venom:")?.split(':').next()?.parse::<f64>().ok().filter(|n| n.is_finite())
}

/// R52 saves retain the existing key format. Older multi-bucket R52 states migrate
/// lazily to one severity and the latest expiry, without resetting their clock.
/// O(C) time, O(1) scratch besides the replacement key.
fn consolidate_venom(r: &mut Run, id: f64) {
    let now = r.tick;
    let a = crate::engine::actor_mut_trusted(r, id).unwrap();
    let mut severity = 0.0;
    let mut latest: Option<(f64, String)> = None;
    for (key, value) in &a.conditions {
        if let Some(expiry) = venom_expiry(key).filter(|expiry| *expiry >= now) {
            severity += value.max(0.0);
            if latest.as_ref().is_none_or(|(old, _)| expiry > *old) {
                latest = Some((expiry, key.clone()));
            }
        }
    }
    a.conditions.retain(|key, _| !key.starts_with("venom:"));
    if let Some((_, key)) = latest.filter(|_| severity > 0.0) {
        a.conditions.insert(key, severity);
    }
}

pub fn enemy_turn_effects(r: &mut Run, id: f64) -> Result<(), crate::engine::EngineError> {
    if r.simulation_revision.value_or(29.0) >= 52.0 { consolidate_venom(r, id); }
    let keys: Vec<_> = crate::engine::actor(r, id)
        .unwrap()
        .conditions
        .iter()
        .filter(|(k, _)| k.starts_with("venom:"))
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    let mut remaining: f64 = 0.0;
    for (key, value) in keys {
        let parts: Vec<_> = key.split(':').collect();
        let expiry = parts
            .get(1)
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(f64::NAN);
        let source = parts
            .get(2)
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(f64::NAN);
        if expiry < r.tick {
            crate::engine::actor_any_mut(r, id)
                .unwrap()
                .conditions
                .shift_remove(&key);
            continue;
        }
        let source = if source == r.player.id || r.actors_at(r.depth).iter().any(|a| a.id == source)
        {
            Some(source)
        } else {
            None
        };
        if crate::engine::actor_any(r, id).is_some_and(|a| !a.dead.value_or(false)) {
            crate::engine::damage_environment(r, id, value, source, "venom", false)?;
        }
        if crate::engine::actor_any(r, id).is_none() {
            return Ok(());
        }
        remaining = remaining.max(expiry - r.tick);
        if expiry <= r.tick {
            crate::engine::actor_any_mut(r, id)
                .unwrap()
                .conditions
                .shift_remove(&key);
        }
    }
    let a = crate::engine::actor_any_mut(r, id).unwrap();
    if remaining != 0.0 {
        a.conditions.insert("envenomed".into(), remaining);
    } else {
        a.conditions.shift_remove("envenomed");
    }
    crate::monster_grab::tick_actor(r, id);
    crate::monster_horn::tick_actor(r, id);
    Ok(())
}
