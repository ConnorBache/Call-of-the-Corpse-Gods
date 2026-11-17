//! Native divine activation and explicit copied-power admission boundaries.
use crate::{
    engine::{EngineError, log},
    model::*,
    resources, vision,
};
pub const ASSAULT: &str = "shrail-unyielding-assault";
pub const SPARKLE: &str = "anavani-illuminate";
pub const DILUTE: &str = "kative-dilute-potion";
pub const KNEEL: &str = "anavani-make-kneel";
pub const SLEEP: &str = "ursa-sudden-sleep";
pub const ALL_SEEING: &str = "morae-all-seeing-eyes";
pub const BULWARK: &str = "pelian-bulwark-of-mind";
/// Nova is paid for each resolved attack, never for toggling or turn upkeep.
/// Historical revisions retain their original per-blast funding threshold.
pub fn nova_attack_cost(revision: f64) -> f64 {
    if revision >= 45.0 { 40.0 } else { 30.0 }
}
/// Resource ownership does not grant a sleeping physical body the projection's toggles.
/// O(A + K): shared-owner lookup followed by the bounded active-power list.
pub fn endpoint_active(r: &Run, endpoint: &Actor, id: &str) -> bool {
    if crate::dreamwalking::enabled(r) && crate::dreamwalking::is_body(endpoint) {
        return false;
    }
    crate::engine::actor(r, crate::half_here::owner_id(r, endpoint))
        .unwrap_or(endpoint)
        .active.iter().any(|s| s == id)
}
fn targets_actor(id: &str) -> bool {
    matches!(id,
        "pelian-i-came-for-you" | "numo-implant-thought" | "numo-distract"
        | "numo-mind-slave" | "numo-remember" | "isheilah-make-slippery"
        | "etill-riptide" | "dawel-unspeak" | "morae-bonded-by-unseen-threads"
        | "morae-sever-fate" | "morae-fortune-teller" | KNEEL)
}
pub fn is_action(id: &str) -> bool {
    matches!(id, SLEEP | "numo-implant-thought")
}
pub fn copied_trait_supported(id: &str) -> bool {
    crate::enemy_traits::catalog()
        .abilities
        .get(id)
        .is_some_and(|ability| ability.passive)
}
pub fn mini_me_ability_supported(id: &str) -> bool {
    matches!(
        id,
        ASSAULT
            | SPARKLE
            | DILUTE
            | "pelian-i-came-for-you"
            | "pelian-never-without-a-fight"
            | "isheilah-missed-me"
            | "numo-distract"
            | "numo-mind-read"
            | "etill-float-away"
            | "ursa-dream-see"
            | "lichor-the-number-of-blood"
            | "dawel-shadow-in-the-dark"
            | "dawel-not-there"
            | "grisha-death-swift"
            | "morae-fortune-teller"
    )
}
pub fn supported(id: &str) -> bool {
    crate::revision50::supported(id) || matches!(
        id,
        crate::progression::WEAVE_ID
            | ASSAULT
            | SPARKLE
            | DILUTE
            | KNEEL
            | SLEEP
            | ALL_SEEING
            | BULWARK
            | "isheilah-guile-craft"
            | "kative-artifice-craft"
            | "pelian-zeal-craft"
            | "numo-implant-thought"
            | "numo-mind-slave"
            | "morae-sever-fate"
            | "isheilah-tools-of-the-trade"
            | "grisha-boundless-hunger"
            | "kative-adaptive-armor"
            | "grisha-entropy"
            | "anavani-star-power"
            | "anavani-ascend"
            | "kative-mini-me"
            | "numo-remember"
            | "grisha-erase-metaphysical"
            | "grisha-uncast"
            | "morae-turning-the-wheel"
            | "dawel-unspeak"
            | "kative-tinker"
            | "morae-bonded-by-unseen-threads"
            | "dawel-silent-strike"
            | "ursa-sleep-walk"
            | "dawel-step-through-shadows"
            | "ursa-dream-craft"
            | "shrail-fury-crafting"
            | "shrail-i-hit-back"
            | "lichor-bleed-them"
            | "pelian-never-surrender"
            | "isheilah-quick-hand"
            | "isheilah-are-you-sure-about-that"
            | "ursa-waking-nightmare"
            | "anavani-nova"
            | "lichor-blood-letting"
            | "ursa-dream-of-time-gone-by"
            | "etill-phase-through"
            | "etill-rushing-torrent"
            | "etill-drown"
            | "etill-riptide"
            | "grisha-black-pit"
            | "dawel-reverse-entropy"
            | "shrail-rage-at-the-dying-of-the-light"
            | "lichor-overflowing"
            | "pelian-you-shall-never-be-rid-of-me"
            | "anavani-shine"
            | "isheilah-make-slippery"
            | "kative-divine-craftsman"
            | "numo-mind-read"
            | "ursa-dream-see"
            | "dawel-shadow-in-the-dark"
            | "dawel-not-there"
            | "pelian-i-came-for-you"
            | "isheilah-missed-me"
            | "pelian-never-without-a-fight"
            | "grisha-death-swift"
            | "lichor-the-number-of-blood"
            | "shrail-fight-long"
            | "shrail-beat-to-death"
            | "lichor-hardened-blood"
            | "lichor-blood-weapon"
            | "etill-float-away"
            | "etill-half-here"
            | "morae-fortune-teller"
            | "numo-distract"
    )
}
/// Per-revision execution registry. Catalog data alone never admits a mechanic.
pub fn supported_for_run(r:&Run,id:&str)->bool {
    if !crate::revision50::enabled(r) {return supported(id);}
    crate::progression::definition_for_revision(50.,id).is_some()
        && supported(id)
}
pub fn activate(
    r: &mut Run,
    id: &str,
    target: &Field<f64>,
    x: &Field<f64>,
    y: &Field<f64>,
) -> Result<bool, EngineError> {
    crate::dreamwalking::with_actor(r, r.player.id, |r| activate_inner(r, id, target, x, y))
}
/// Creature-creating branches add O(d) clean admission or O(A*d + F + H) dirty
/// registry reconciliation before birth; existing ability-specific costs remain.
fn activate_inner(
    r: &mut Run,
    id: &str,
    target: &Field<f64>,
    x: &Field<f64>,
    y: &Field<f64>,
) -> Result<bool, EngineError> {
    // Target admission precedes revision-specific dispatch and all mutation.
    // Maintained links can always be switched off after losing their target.
    if r.simulation_revision.value_or(29.) >= 52.
        && !r.player.active.iter().any(|s| s == id)
        && matches!(id, "numo-implant-thought" | "pelian-fated-duel" | "pelian-relentless-pursuit")
        && target.as_ref().and_then(|tid| crate::engine::actor(r, *tid))
            .is_some_and(|a| !crate::ability_sight::actor(r, a))
    { return Ok(false); }
    if crate::revision50::enabled(r) {
        if let Some(took) = crate::revision50::activate(r, id, target, x, y)? { return Ok(took); }
    }
    if id.starts_with("craft:") {
        if crate::crafting::activate(r, id)? {
            log(r, "You use Equipment craft.", "player");
            emit_activation(r, "Equipment craft", target, x, y);
        }
        return Ok(false);
    }
    if !supported(id) {
        return Err(EngineError(format!("ability {id}")));
    }
    if !r.player.abilities.iter().any(|a| a == id) {
        return Ok(false);
    }
    // R52 Hanged is permanently passive, even when an old raw save has a stale toggle.
    if crate::hanged::enabled(r.simulation_revision.value_or(29.)) && id == crate::hanged::ID { return Ok(false); }
    // Passive Weave has no activation effect or resource cost. UI binds no command.
    if id == crate::progression::WEAVE_ID { return Ok(false); }
    // Validate actor targets before initialization, resource costs, or target mutation.
    // Tinker uses equipment IDs; active maintained links must still be switchable off.
    let switching_off = matches!(id, "dawel-unspeak" | "morae-bonded-by-unseen-threads"
        | "isheilah-make-slippery" | "etill-riptide")
        && r.player.active.iter().any(|s| s == id);
    if targets_actor(id) && !switching_off {
        if target.as_ref().and_then(|tid| crate::engine::actor(r, *tid))
            .is_some_and(|a| !crate::dreamwalking::same_plane(r, &r.player, a))
        {
            log(r, "That target is on another plane.", "player");
            return Ok(false);
        }
    }
    if matches!(id, "numo-implant-thought" | "numo-mind-slave" | "morae-sever-fate")
        && target.as_ref().and_then(|tid| crate::engine::actor(r, *tid))
            .is_some_and(crate::ahvamitt::is_actor)
    {
        log(r, "The Godbeast will not be swayed.", "player");
        return Ok(false);
    }
    if let Some(tid) = target.as_ref() {
        if crate::engine::actor(r, *tid).is_some_and(crate::half_here::is_clone)
            && !r.player.active.iter().any(|s| s == id)
            && matches!(
                id,
                "pelian-i-came-for-you"
                    | "numo-implant-thought"
                    | "numo-distract"
                    | "numo-mind-slave"
                    | "numo-remember"
                    | "isheilah-make-slippery"
                    | "etill-riptide"
                    | "dawel-unspeak"
                    | "morae-bonded-by-unseen-threads"
                    | "morae-sever-fate"
            )
        {
            log(
                r,
                "Half Here is your own body, not an enemy target.",
                "player",
            );
            return Ok(false);
        }
    }
    match id {
        "kative-mini-me" => {
            let active = r.player.active.iter().any(|a| a == id);
            if active {
                crate::mini_me::dismiss(r);
                r.player.active.retain(|s| s != id);
            } else {
                crate::mini_me::preflight(r).map_err(EngineError)?;
                let Some((x, y)) = crate::mini_me::destination(r) else {
                    log(
                        r,
                        "There is no free neighboring square for Mini-Me.",
                        "player",
                    );
                    return Ok(false);
                };
                if !crate::revision50_ability_energy::can_pay(r, r.player.id, id, 30.0) || energy_activation_silenced(r, &r.player) {
                    pay(r, id, 30.0);
                    return Ok(false);
                }
                // O(d) clean admission, O(A*d + F + H) dirty; reject before paying.
                crate::creature_index::check_next_creature_id(r).map_err(EngineError)?;
                let clone = crate::mini_me::preview(r, x, y);
                if let Some(id) = clone
                    .abilities
                    .iter()
                    .find(|id| if crate::revision50::enabled(r) { !supported(id) && !copied_trait_supported(id) && !["morae-see-the-future"].contains(&id.as_str()) } else { !mini_me_ability_supported(id) })
                {
                    return Err(EngineError(format!("Mini-Me copied ability {id}")));
                }
                if !pay(r, id, 30.0) {
                    return Ok(false);
                }
                crate::mini_me::commit(r, clone).map_err(EngineError)?;
                r.player.active.push(id.into());
            }
            crate::half_here::sync(r);
            log(
                r,
                format!(
                    "You {} Mini-Me.",
                    if active { "deactivate" } else { "activate" }
                ),
                "player",
            );
        }
        "numo-remember" => {
            let Some(&tid) = target.as_ref() else {
                return Ok(false);
            };
            let Some(enemy) = r
                .actors_at(r.depth)
                .iter()
                .find(|a| a.id == tid && !a.dead.value_or(false))
            else {
                return Ok(false);
            };
            let mut initialized = enemy.clone();
            if !crate::revision50_ability_energy::can_pay(r, r.player.id, id, 5.0) || energy_activation_silenced(r, &r.player) {
                pay(r, id, 5.0);
                return Ok(false);
            }
            crate::enemy_traits::initialize_enemy_traits_revision(&mut initialized,r.simulation_revision.value_or(29.));
            let copied: Vec<String> = initialized
                .abilities
                .iter()
                .filter(|id| {
                    !r.player.abilities.contains(id)
                        && (crate::progression::catalog_for_revision(r.simulation_revision.value_or(29.0))
                            .iter()
                            .find(|a| a.id == **id)
                            .is_some_and(|a| a.flags.iter().any(|f| f == "passive"))
                            || crate::enemy_traits::catalog()
                                .abilities
                                .get(*id)
                                .is_some_and(|a| a.passive))
                })
                .cloned()
                .collect();
            if let Some(id) = copied
                .iter()
                .find(|id| !supported(id) && !copied_trait_supported(id))
            {
                return Err(EngineError(format!("Remember copied trait {id}")));
            }
            if crate::revision50::enabled(r) {
                crate::revision50_ability_energy::validate_reserves(&r.player)
                    .map_err(|e| EngineError(format!("Invalid blessing reserve: {e:?}")))?;
            }
            if !pay(r, id, 5.0) {
                return Ok(false);
            }
            if crate::revision50::enabled(r) {
                crate::revision50_ability_energy::acquire_for_revision(&mut r.player, &copied, r.simulation_revision.value_or(29.))
                    .map_err(|e| EngineError(format!("Remember acquisition: {e:?}")))?;
            }
            *crate::engine::actor_mut_trusted(r, tid).unwrap() = initialized;
            let acquired=!copied.is_empty();
            let copied_animation=if crate::events::enabled(r) {copied.clone()}else{Vec::new()};
            for id in copied {
                r.player.abilities.push(id.clone());
                crate::progression::initialize_ability_state_for_revision(r.simulation_revision.value_or(29.), &mut r.player, &id, r.tick)
                    .map_err(EngineError)?;
                r.player
                    .conditions
                    .insert(format!("remembered:{id}"), r.tick + 5.0);
            }
            crate::equipment::sync_health(r, false);
            crate::half_here::sync(r);
            if acquired {
                if let Some(target)=crate::engine::actor(r,tid).filter(|a|crate::events::actor_observed(r,a)) {
                    for trait_id in &copied_animation {
                        crate::events::link(r,target,&r.player,"P24",&format!("fracture-trait:{trait_id}"),crate::events::Stage::Special);
                    }
                    crate::animation_player::local(r,target,"P24","fold-outline",crate::events::Stage::Special);
                } else {crate::animation_player::local(r,&r.player,"P24","fracture",crate::events::Stage::Special);}
            }
            log(r, "You use Remember.", "player");
        }
        "dawel-unspeak" => {
            let before_player = r.player.clone();
            let before_target = target
                .as_ref()
                .and_then(|id| crate::engine::actor(r, *id))
                .cloned();
            let active = r.player.active.iter().any(|s| s == id);
            if active {
                r.player.active.retain(|s| s != id);
                r.player.conditions.shift_remove("unspeakTarget");
                r.player.conditions.shift_remove("unspeakCost");
            } else {
                let Some(&tid) = target.as_ref() else {
                    return Ok(false);
                };
                if tid == r.player.id
                    || !r
                        .actors_at(r.depth)
                        .iter()
                        .any(|a| a.id == tid && !a.dead.value_or(false))
                {
                    return Ok(false);
                }
                let cost = if crate::revision50::enabled(r) {
                    let mut preview = crate::engine::actor(r, tid).unwrap().clone();
                    let cost = highest_energy_cost_for(&mut preview, r.simulation_revision.value_or(29.));
                    if (cost > 0.0 && energy_activation_silenced(r, &r.player))
                        || !crate::revision50_ability_energy::can_pay(r, r.player.id, id, (cost-prepaid_upkeep(r,&r.player,id)).max(0.0)) {
                        return Ok(false);
                    }
                    if !prepay_activation_upkeep(r, id, cost) { return Ok(false); }
                    *crate::engine::actor_mut_trusted(r, tid).unwrap() = preview;
                    cost
                } else { highest_energy_cost(r, tid)? };
                r.player.conditions.insert("unspeakTarget".into(), tid);
                r.player.conditions.insert("unspeakCost".into(), cost);
                if cost > 0.0 && energy_activation_silenced(r, &r.player) {
                    log(r, "Silence prevents this energy-paid activation.", "player");
                } else if !crate::revision50_ability_energy::can_pay(r, r.player.id, id, (cost - prepaid_upkeep(r,&r.player,id)).max(0.0)) {
                    log(r, "Not enough energy to maintain this ability.", "player");
                } else {
                    r.player.active.push(id.into());
                }
            }
            if r.player == before_player
                && target.as_ref().and_then(|id| crate::engine::actor(r, *id))
                    == before_target.as_ref()
            {
                return Ok(false);
            }
            log(
                r,
                format!(
                    "You {} Unspeak.",
                    if active {
                        "deactivate"
                    } else if r.player.active.iter().any(|s| s == id) {
                        "activate"
                    } else {
                        "use"
                    }
                ),
                "player",
            );
        }
        "kative-tinker" => {
            if !crate::tinker::activate(r, target.as_ref().copied())? {
                return Ok(false);
            }
            crate::half_here::sync(r);
            log(r, "You use Tinker.", "player");
        }
        "morae-bonded-by-unseen-threads" => {
            let active = r.player.active.iter().any(|s| s == id);
            if active {
                r.player.active.retain(|s| s != id);
                r.player.conditions.shift_remove("bondedTarget");
            } else {
                let Some(&tid) = target.as_ref() else {
                    return Ok(false);
                };
                if tid == r.player.id
                    || !crate::engine::actor(r, tid).is_some_and(|a| !a.dead.value_or(false))
                {
                    return Ok(false);
                }
                if crate::revision50::enabled(r) {
                    let cost = upkeep_cost_in(r, &r.player, id);
                    let paid = prepaid_upkeep(r, &r.player, id);
                    // Admit activation and first-turn maintenance together; never
                    // spend the activation fee if the maintained benefit is unfunded.
                    if !pay(r, id, 15.0 + (cost - paid).max(0.0)) { return Ok(false); }
                    record_activation_upkeep(r, id, cost, paid);
                } else if !pay(r, id, 15.0) {
                    return Ok(false);
                }
                r.player.conditions.insert("bondedTarget".into(), tid);
                if !crate::revision50_ability_energy::can_pay(r, r.player.id, id, activation_upkeep_due(r,id)) {
                    log(r, "Not enough energy to maintain this ability.", "player");
                } else {
                    r.player.active.push(id.into());
                }
            }
            log(
                r,
                format!(
                    "You {} Bonded By Unseen Threads.",
                    if active {
                        "deactivate"
                    } else if r.player.active.iter().any(|s| s == id) {
                        "activate"
                    } else {
                        "use"
                    }
                ),
                "player",
            );
        }
        "ursa-dream-of-time-gone-by" => {
            let active = r.player.active.iter().any(|s| s == id);
            if active {
                let (Some(&x), Some(&y), Some(&depth)) = (
                    r.player.conditions.get("dreamX"),
                    r.player.conditions.get("dreamY"),
                    r.player.conditions.get("dreamDepth"),
                ) else {
                    log(
                        r,
                        "Dream return blocked: the destination is not free.",
                        "player",
                    );
                    return Ok(false);
                };
                let phase = r.player.active.iter().any(|s| s == "etill-phase-through");
                let valid = crate::fast_mode::playable(r, depth) && r.floors.iter().find(|f| f.depth == depth).is_some_and(|f| {
                    valid_point(f, x, y)
                        && (phase
                            || !matches!(
                                f.tiles[y as usize * f.width() + x as usize].as_str(),
                                "wall" | "tree"
                            ))
                }) && !r
                    .actors_at(depth)
                    .iter()
                    .any(|a| !a.dead.value_or(false)
                        && r.floors.iter().find(|f| f.depth == depth).is_some_and(|f| crate::portals::same_cell(f, a.x, a.y, x, y))
                        && crate::dreamwalking::same_plane(r, &r.player, a));
                if !valid {
                    log(
                        r,
                        "Dream return blocked: the destination is not free.",
                        "player",
                    );
                    return Ok(false);
                }
                if depth != r.depth {
                    let (old_x, old_y) = (r.player.x, r.player.y);
                    r.player.x = x;
                    r.player.y = y;
                    if !crate::travel::travel_to(r, depth, false, true)
                        .map_err(EngineError)?
                    {
                        r.player.x = old_x;
                        r.player.y = old_y;
                        return Ok(false);
                    }
                }
                crate::animation_player::local(r,&r.player,"P33","fold",crate::events::Stage::Travel);
                r.player.x = x;
                r.player.y = y;
                crate::revision50::duel_actor_changed(r,r.player.id);
                crate::animation_player::local(r,&r.player,"P33","unfold",crate::events::Stage::Travel);
                r.player.active.retain(|s| s != id);
                for k in ["dreamX", "dreamY", "dreamDepth"] {
                    r.player.conditions.shift_remove(k);
                }
            } else {
                if !pay(r, id, 50.0) {
                    return Ok(false);
                }
                r.player.conditions.insert("dreamX".into(), r.player.x);
                r.player.conditions.insert("dreamY".into(), r.player.y);
                r.player.conditions.insert("dreamDepth".into(), r.depth);
                r.player.active.push(id.into());
            }
            crate::half_here::sync(r);
            log(
                r,
                format!(
                    "You {} Dream of Time Gone By.",
                    if active { "deactivate" } else { "activate" }
                ),
                "player",
            );
        }
        "lichor-blood-letting" => {
            if !pay(r, id, 0.0) {
                return Ok(false);
            }
            crate::engine::damage_environment(r, r.player.id, 10.0, None, "blood-letting", false)?;
            // Bounded player-known status names only: never source IDs, hidden
            // actors, serialized bookkeeping keys or arbitrary condition strings.
            const PEEL_STATUSES:&[&str]=&["hemophilia","slowed","paralyzed","magicSensitive","bleeding","burning","poisoned","weakened","emboldened","lucky","unlucky","distracted","flying","telepathic","invisible","confused","hasted","silenced","riptide","gasVulnerable","disoriented","hallucinating","stuck","grabbed","envenomed"];
            let peel:Vec<_>=if crate::events::enabled(r) {PEEL_STATUSES.iter().copied().filter(|key|crate::engine::condition(&r.player,key)>0.).collect()}else{Vec::new()};
            clear_status_conditions(&mut r.player);
            crate::equipment::sync_health(r, false);
            crate::half_here::sync(r);
            crate::animation_player::local(r,&r.player,"P39","ripple",crate::events::Stage::Death);
            let removed:Vec<_>=peel.into_iter().filter(|status|!r.player.conditions.contains_key(*status)).collect();
            if !removed.is_empty() {
                crate::animation_player::local(r,&r.player,"P39",&format!("fracture-status:{}",removed.join("+")),crate::events::Stage::Death);
            }
            log(r, "You use Blood Letting.", "player");
        }
        "isheilah-make-slippery" => {
            let active = r.player.active.iter().any(|s| s == id);
            if active {
                r.player.active.retain(|s| s != id);
            } else {
                let Some(&tid) = target.as_ref() else {
                    return Ok(false);
                };
                if !crate::engine::actor(r, tid).is_some_and(|a| !a.dead.value_or(false)) {
                    return Ok(false);
                }
                r.player.conditions.insert("slipperyTarget".into(), tid);
                r.player.active.push(id.into());
            }
            log(
                r,
                format!(
                    "You {} Make Slippery.",
                    if active { "deactivate" } else { "activate" }
                ),
                "player",
            );
        }
        "etill-riptide" => {
            let active = r.player.active.iter().any(|s| s == id);
            if active {
                r.player.active.retain(|s| s != id);
                r.player.conditions.shift_remove("riptideTarget");
            } else {
                let (Some(&tid), Some(&x), Some(&y)) = (target.as_ref(), x.as_ref(), y.as_ref())
                else {
                    return Ok(false);
                };
                if tid == r.player.id
                    || !crate::engine::actor(r, tid).is_some_and(|a| !a.dead.value_or(false))
                    || !r.floor().is_some_and(|f| {
                        x >= 0.0 && y >= 0.0 && x < f.width() as f64 && y < f.height() as f64
                    })
                {
                    return Ok(false);
                }
                if !pay(r, id, 15.0) {
                    return Ok(false);
                }
                r.player.conditions.insert("riptideTarget".into(), tid);
                r.player.conditions.insert("riptideX".into(), x);
                r.player.conditions.insert("riptideY".into(), y);
                if !crate::revision50_ability_energy::can_pay(r, r.player.id, id, upkeep_cost_in(r,&r.player,id)) {
                    log(r, "Not enough energy to maintain this ability.", "player");
                } else {
                    r.player.active.push(id.into());
                }
            }
            log(
                r,
                format!(
                    "You {} Riptide.",
                    if active {
                        "deactivate"
                    } else if r.player.active.iter().any(|s| s == id) {
                        "activate"
                    } else {
                        "use"
                    }
                ),
                "player",
            );
        }
        "anavani-ascend" => {
            if energy_activation_silenced(r, &r.player) {
                log(r, "Silence prevents this energy-paid activation.", "player");
                return Ok(false);
            }
            let valid = r.depth > 1.0
                && r.floors
                    .iter()
                    .find(|f| f.depth == crate::fast_mode::above(r, r.depth))
                    .is_some_and(|f| {
                        valid_point(f, r.player.x, r.player.y)
                            && f.tiles[r.player.y as usize * f.width() + r.player.x as usize]
                                == "chasm"
                    });
            if !valid || !crate::revision50_ability_energy::can_pay(r, r.player.id, id, 50.0) {
                return Ok(false);
            }
            if crate::revision50::enabled(r) {
                // Travel may mutate floors: keep reserve debit and travel atomic.
                let before = r.clone();
                if !pay(r, id, 50.0) { return Ok(false); }
                match crate::travel::travel(r, -1.0, false, true) {
                    Ok(true) => {},
                    Ok(false) => { *r = before; return Ok(false); },
                    Err(e) => { *r = before; return Err(EngineError(e)); },
                }
            } else {
                if !crate::travel::travel(r, -1.0, false, true).map_err(EngineError)? {
                    return Ok(false);
                }
                let p = r.player.clone();
                resources::record_loss(r, &p, "energy", 50.0);
                r.player.energy -= 50.0;
            }
            crate::animation_player::local(r,&r.player,"P56","rise",crate::events::Stage::Travel);
            log(r, "You use Ascend.", "player");
        }
        "lichor-blood-weapon" => {
            if crate::divine_combat::blood_weapon_ready(r, &r.player) {
                log(r, "Your Blood Weapon is already prepared.", "player");
                return Ok(false);
            }
            let cost = crate::divine_combat::blood_weapon_hp_cost_for_run(r, &r.player);
            crate::engine::damage_environment(r, r.player.id, cost, None, "blood-weapon", true)?;
            if r.player.hp > 0.0 && r.status == "playing" {
                r.player.conditions.insert("bloodWeaponReady".into(), 1.0);
            }
            crate::half_here::sync(r);
            log(r, "You use Blood Weapon.", "player");
        }
        "etill-half-here" => {
            let active = r.player.active.iter().any(|s| s == id);
            if active {
                crate::half_here::dismiss(r);
            } else {
                let valid = match (x.as_ref(), y.as_ref()) {
                    (Some(&x), Some(&y)) => {
                        r.floor().is_some_and(|f| {
                            valid_point(f, x, y)
                                && !matches!(
                                    f.tiles[y as usize * f.width() + x as usize].as_str(),
                                    "chasm" | "pit"
                                )
                        }) && crate::ability_sight::tile(r, x as usize, y as usize)
                            && crate::engine::passable(r, x, y)
                            && !occupied_cell(r, x, y)
                    }
                    _ => false,
                };
                if !valid {
                    log(
                        r,
                        "Choose a visible, unoccupied position for Half Here.",
                        "player",
                    );
                    return Ok(false);
                }
                crate::half_here::create(r, *x.as_ref().unwrap(), *y.as_ref().unwrap()).map_err(EngineError)?;
            }
            log(
                r,
                format!(
                    "You {} Half Here.",
                    if active { "deactivate" } else { "activate" }
                ),
                "player",
            );
        }
        "grisha-entropy" => {
            let active = r.player.active.iter().any(|s| s == id);
            if active {
                r.player.active.retain(|s| s != id);
            } else {
                r.player.active.push(id.into());
            }
            log(
                r,
                format!(
                    "You {} Entropy’s caustic gas emission.",
                    if active { "resume" } else { "suppress" }
                ),
                "player",
            );
        }
        "numo-implant-thought" | "numo-mind-slave" | "morae-sever-fate" => {
            let Some(&tid) = target.as_ref() else {
                return Ok(false);
            };
            let Some(a) = crate::engine::actor(r, tid) else {
                return Ok(false);
            };
            if tid != r.player.id && a.dead.value_or(false) {
                return Ok(false);
            }
            if id == "numo-mind-slave" && a.hp > a.max_hp * 0.2 {
                return Ok(false);
            }
            if id == "morae-sever-fate" && a.hp >= a.max_hp * 0.5 {
                return Ok(false);
            }
            if !pay(r, id, if id == "numo-mind-slave" { 100.0 } else { 30.0 }) {
                return Ok(false);
            }
            let a = crate::engine::actor_mut_trusted(r, tid).unwrap();
            crate::ai::wake(a, true);
            if id == "morae-sever-fate" {
                a.state = "neutral".into();
            } else {
                a.state = "ally".into();
                a.perfect = true.into();
                a.order = "a".to_string().into();
                if id == "numo-implant-thought" {
                    a.conditions.insert("temporaryAlly".into(), 5.0);
                }
            }
            if let Some(a)=crate::engine::actor(r,tid) {
                if id=="numo-mind-slave" {
                    crate::animation_shapes::line(r,&r.player,a.into(),"P22","line-hold","#a040ff");
                    crate::animation_shapes::area(r,a,"P22","box-contract",2,"#a040ff",crate::events::Stage::Special);
                } else {crate::animation_player::local(r,a,"P70","slash",crate::events::Stage::Special);}
            }
            let name = &crate::progression::catalog_for_revision(r.simulation_revision.value_or(29.0))
                .iter()
                .find(|a| a.id == id)
                .unwrap()
                .name;
            log(r, format!("You use {name}."), "player");
            emit_activation(r, name, target, x, y);
            return Ok(id == "numo-implant-thought");
        }
        "anavani-star-power"
        | "grisha-erase-metaphysical"
        | "grisha-uncast"
        | "morae-turning-the-wheel"
        | "pelian-never-surrender"
        | "isheilah-quick-hand"
        | "isheilah-are-you-sure-about-that"
        | "ursa-waking-nightmare"
        | "anavani-nova"
        | "etill-phase-through"
        | "etill-rushing-torrent"
        | "grisha-black-pit"
        | "dawel-reverse-entropy"
        | "shrail-rage-at-the-dying-of-the-light"
        | "pelian-you-shall-never-be-rid-of-me"
        | "anavani-shine"
        | "numo-mind-read"
        | "ursa-dream-see"
        | "dawel-shadow-in-the-dark"
        | "dawel-not-there"
        | "etill-float-away"
        | "isheilah-missed-me"
        | "pelian-never-without-a-fight" => {
            let active = r.player.active.iter().any(|s| s == id);
            if !active
                && upkeep_cost_in(r,&r.player, id) > 0.0
                && energy_activation_silenced(r, &r.player)
            {
                log(r, "Silence prevents this energy-paid activation.", "player");
                return Ok(false);
            }
            if !active && !crate::revision50_ability_energy::can_pay(r, r.player.id, id,
                (upkeep_cost_in(r,&r.player, id) - prepaid_upkeep(r,&r.player,id)).max(0.0)) {
                log(r, "Not enough energy to maintain this ability.", "player");
                return Ok(false);
            }
            if active {
                r.player.active.retain(|s| s != id);
                if id == "etill-phase-through" {
                    crate::divine_environment::settle_phase_loss(r, false).map_err(EngineError)?;
                }
            } else {
                if !prepay_activation_upkeep(r, id, upkeep_cost_in(r, &r.player, id)) { return Ok(false); }
                r.player.active.push(id.into());
            }
            let name = &crate::progression::catalog_for_revision(r.simulation_revision.value_or(29.0))
                .iter()
                .find(|a| a.id == id)
                .unwrap()
                .name;
            log(
                r,
                format!(
                    "You {} {name}.",
                    if active { "deactivate" } else { "activate" }
                ),
                "player",
            );
        }
        "numo-distract" | "pelian-i-came-for-you" | "morae-fortune-teller" => {
            let Some(&tid) = target.as_ref() else {
                return Ok(false);
            };
            let Some(a) = crate::engine::actor(r, tid) else {
                return Ok(false);
            };
            if tid != r.player.id && a.dead.value_or(false) {
                return Ok(false);
            }
            if id == "pelian-i-came-for-you" && tid == r.player.id {
                return Ok(false);
            }
            if id == "morae-fortune-teller" {
                let fortunes = r.player.conditions.get("fortunes").copied().unwrap_or(3.0);
                if fortunes <= 0.0 {
                    return Ok(false);
                }
                r.player
                    .conditions
                    .insert("fortunes".into(), fortunes - 1.0);
                if r.simulation_revision.value_or(29.) >= 52. {
                    crate::fortune_schedule::ensure(&mut r.player, r.tick);
                }
                let key = if tid == r.player.id {
                    "lucky"
                } else {
                    "unlucky"
                };
                // The visible casting cue also plays on same-value reapplication.
                if tid == r.player.id {
                    crate::animation_shapes::nova(r,&r.player,"P67","nova-blue-small",2);
                } else if let Some(recipient)=crate::engine::actor(r,tid) {
                    crate::animation_shapes::line(r,&r.player,recipient.into(),"P67","line-grow","#a050ff");
                }
                crate::engine::actor_mut_trusted(r, tid)
                    .unwrap()
                    .conditions
                    .insert(key.into(), 1.0);
            } else {
                if !pay(r, id, 10.0) {
                    return Ok(false);
                }
                if id == "numo-distract" {
                    crate::engine::actor_mut_trusted(r, tid)
                        .unwrap()
                        .conditions
                        .insert("distracted".into(), 1.0);
                } else {
                    r.player.conditions.insert("cameTarget".into(), tid);
                    // R50+ stores a stable enabled sentinel, not a countdown.
                    // Only historical <50 runs decrement cameTurns.
                    let turns=if crate::revision50::enabled(r) {1.0} else {20.0};
                    r.player.conditions.insert("cameTurns".into(), turns);
                }
            }
            let name = &crate::progression::catalog_for_revision(r.simulation_revision.value_or(29.0))
                .iter()
                .find(|a| a.id == id)
                .unwrap()
                .name;
            log(r, format!("You use {name}."), "player");
        }
        ASSAULT => {
            let active = r.player.active.iter().any(|a| a == id);
            if active {
                r.player.active.retain(|a| a != id);
            } else {
                r.player.active.push(id.into());
            }
            log(
                r,
                format!(
                    "You {} Unyielding Assault.",
                    if active { "deactivate" } else { "activate" }
                ),
                "player",
            );
        }
        ALL_SEEING => {
            if !pay(r, id, 30.0) {
                return Ok(false);
            }
            r.player.conditions.insert("allSeeingReveal".into(), 1.0);
            r.player.conditions.insert("allSeeingDepth".into(), r.depth);
            if let Some(f)=r.floor() {
                let cells:Vec<_>=(0..f.tiles.len()).map(|i|crate::events::EventPoint{x:(i%f.width()) as f64,y:(i/f.width()) as f64}).collect();
                let serial=crate::events::emission_serial();
                crate::events::cue(r,&r.player,"P72","map-reveal",crate::events::Stage::Special,&cells);
                crate::events::decorate(serial,"P72","map-reveal",(&r.player).into(),"#ffb0ee",None);
            }
            log(r, "You use All-Seeing Eyes.", "player");
        }
        BULWARK => {
            let (Some(&x), Some(&y)) = (x.as_ref(), y.as_ref()) else {
                return Ok(false);
            };
            let f = r
                .floor()
                .ok_or_else(|| EngineError("missing floor".into()))?;
            if !valid_point(f, x, y) || !crate::ability_sight::tile(r, x as usize, y as usize) {
                return Ok(false);
            }
            let at = y as usize * f.width() + x as usize;
            if matches!(f.tiles[at].as_str(), "wall" | "tree")
                || f.grates.as_ref().is_some_and(|g| g.contains(&(at as f64)))
                || occupied_cell(r, x, y)
            {
                return Ok(false);
            }
            if !pay(r, id, 20.0) {
                return Ok(false);
            }
            let expires = r.tick + 10.0;
            r.floor_mut()
                .unwrap()
                .bulwarks
                .get_or_insert_with(Default::default)
                .insert(at.to_string(), expires);
            r.floor_mut().unwrap().dirty = true;
            crate::events::at(r,"P11","bracket",crate::events::Stage::Special,&[crate::events::EventPoint{x,y}]);
            log(r, "You use Bulwark of Mind.", "player");
        }
        SLEEP => {
            if !pay(r, id, 20.0) {
                return Ok(false);
            }
            r.player.hp = r.player.max_hp;
            let duration = crate::monster_statuses::status_duration(r,&r.player,51.0);
            r.player.conditions.insert("paralyzed".into(), duration);
            crate::animation_player::local(r,&r.player,"P32","fold",crate::events::Stage::Special);
            log(r, "You use Sudden Sleep.", "player");
            emit_activation(r, "Sudden Sleep", target, x, y);
            return Ok(true);
        }
        KNEEL => {
            let Some(&tid) = target.as_ref() else {
                return Ok(false);
            };
            let Some(a) = crate::engine::actor(r, tid) else {
                return Ok(false);
            };
            if tid != r.player.id && a.dead.value_or(false) {
                return Ok(false);
            }
            if !pay(r, id, 20.0) {
                return Ok(false);
            }
            let duration = crate::monster_statuses::status_duration(r,crate::engine::actor(r,tid).unwrap(),10.0);
            crate::engine::actor_mut_trusted(r, tid)
                .unwrap()
                .conditions
                .insert("paralyzed".into(), duration);
            log(r, "You use Make Kneel.", "player");
        }
        "dawel-silent-strike"
        | "ursa-sleep-walk"
        | "dawel-step-through-shadows"
        | "ursa-dream-craft"
        | "shrail-fury-crafting"
        | "shrail-i-hit-back"
        | "lichor-bleed-them"
        | "etill-drown"
        | "lichor-overflowing"
        | "shrail-fight-long"
        | "shrail-beat-to-death"
        | "lichor-hardened-blood"
        | "isheilah-tools-of-the-trade"
        | "grisha-boundless-hunger"
        | "kative-adaptive-armor"
        | "grisha-death-swift"
        | "lichor-the-number-of-blood"
        | "isheilah-guile-craft"
        | "kative-artifice-craft"
        | "pelian-zeal-craft"
        | "kative-divine-craftsman" => {
            let name = &crate::progression::catalog_for_revision(r.simulation_revision.value_or(29.0))
                .iter()
                .find(|a| a.id == id)
                .unwrap()
                .name;
            log(r, format!("{name} is passive."), "player");
        }
        DILUTE => log(r, "Dilute Potion is passive.", "player"),
        SPARKLE => {
            let (Some(&x), Some(&y)) = (x.as_ref(), y.as_ref()) else {
                return Ok(false);
            };
            let f = r
                .floor()
                .ok_or_else(|| EngineError("floor missing".into()))?;
            if !x.is_finite()
                || !y.is_finite()
                || x.fract() != 0.0
                || y.fract() != 0.0
                || x < 0.0
                || y < 0.0
                || x >= f.width() as f64
                || y >= f.height() as f64
            {
                return Ok(false);
            }
            let i = y as usize * f.width() + x as usize;
            let tile = &f.tiles[i];
            let legacy_terrain_blocked = matches!(tile.as_str(), "wall" | "chasm" | "pit")
                || (matches!(tile.as_str(), "water" | "shallow-water")
                    && f.ichor
                        .as_ref()
                        .and_then(|v| v.get(i))
                        .copied()
                        .unwrap_or(0.0)
                        == 0.0);
            if !crate::ability_sight::tile(r, x as usize, y as usize)
                || (!crate::revision50::enabled(r) && legacy_terrain_blocked)
            {
                return Ok(false);
            }
            if !pay(r, id, 20.0) {
                return Ok(false);
            }
            crate::animation_shapes::line(r,&r.player,crate::events::EventPoint{x,y},"P58","light100","#ffffff");
            crate::environment::place_sparkle(r, x, y).map_err(EngineError)?;
            let wet=r.floor().is_some_and(|f|if r.simulation_revision.value_or(29.)>=52. {
                crate::revision51_fire_water::quenches(f,y as usize*f.width()+x as usize,r.simulation_revision.value_or(29.))
            }else{matches!(f.tiles[y as usize*f.width()+x as usize].as_str(),"water"|"shallow-water")});
            crate::events::at(r,"P58",if wet {"puff"}else{"ember"},crate::events::Stage::Special,&[crate::events::EventPoint{x,y}]);
            log(r, "You use Sparkle.", "player");
        }
        _ => unreachable!(),
    }
    if !matches!(
        id,
        "dawel-silent-strike"
            | "ursa-sleep-walk"
            | "dawel-step-through-shadows"
            | "ursa-dream-craft"
            | "shrail-fury-crafting"
            | "shrail-i-hit-back"
            | "lichor-bleed-them"
            | "etill-drown"
            | "lichor-overflowing"
            | "shrail-fight-long"
            | "shrail-beat-to-death"
            | "lichor-hardened-blood"
            | "isheilah-tools-of-the-trade"
            | "grisha-boundless-hunger"
            | "kative-adaptive-armor"
            | "grisha-death-swift"
            | "lichor-the-number-of-blood"
            | DILUTE
            | "isheilah-guile-craft"
            | "kative-artifice-craft"
            | "pelian-zeal-craft"
            | "kative-divine-craftsman"
    ) {
        let label = crate::progression::catalog_for_revision(r.simulation_revision.value_or(29.0))
            .iter()
            .find(|a| a.id == id)
            .unwrap()
            .name
            .as_str();
        emit_activation(r, label, target, x, y);
    }
    // Placement is a normal action in R52; dismissal remains a free toggle.
    Ok(r.simulation_revision.value_or(29.) >= 52.
        && id == crate::half_here::ID && r.player.active.iter().any(|a| a == id))
}

/// Stable endpoint occupancy; old revisions preserve pending-death blockers.
fn occupied_cell(r: &Run, x: f64, y: f64) -> bool {
    crate::portals::same_position(r, &r.player, x, y)
        || r.actors_at(r.depth).iter().any(|a| !a.dead.value_or(false)
            && crate::dreamwalking::same_plane(r, &r.player, a)
            && crate::portals::same_position(r, a, x, y))
}
fn valid_point(f: &Floor, x: f64, y: f64) -> bool {
    x.is_finite()
        && y.is_finite()
        && x.fract() == 0.0
        && y.fract() == 0.0
        && x >= 0.0
        && y >= 0.0
        && x < f.width() as f64
        && y < f.height() as f64
}
pub fn energy_activation_silenced(r: &Run, target: &Actor) -> bool {
    crate::engine::condition(target, "silenced") > 0.0
        || std::iter::once(&r.player)
            .chain(r.actors_at(r.depth))
            .any(|source| {
                source.id != target.id
                    && !crate::revision50_combat::oblivious_blocks(r,target,Some(source))
                    && !source.dead.value_or(false)
                    && source.hp > 0.0
                    && crate::dreamwalking::same_plane(r, source, target)
                    && crate::enemy_traits::has_enemy_ability(source, "enemy:beast:silence-aura")
                    && (source.id == r.player.id || source.state == "ally")
                        != (target.id == r.player.id || target.state == "ally")
                    && crate::portals::actor_distance(r, source, target) <= 3.0
                    && crate::vision::line_of_sight(r, source, target)
            })
}
fn pay(r: &mut Run, id: &str, n: f64) -> bool {
    if n > 0.0 && energy_activation_silenced(r, &r.player) {
        log(r, "Silence prevents this energy-paid activation.", "player");
        return false;
    }
    if crate::revision50::enabled(r) {
        if !crate::revision50_ability_energy::spend(r, r.player.id, id, n) {
            log(r, "This ability has no usable authored energy reserve or cannot fund its cost.", "player");
            return false;
        }
        return true;
    }
    if r.player.energy < n {
        log(r, "Not enough energy.", "player");
        return false;
    }
    let p = r.player.clone();
    resources::record_loss(r, &p, "energy", n);
    r.player.energy -= n;
    true
}

fn emit_activation(r: &Run, label: &str, target: &Field<f64>, x: &Field<f64>, y: &Field<f64>) {
    if crate::events::enabled(r) {return;} // R52 emits only actual outcome hooks / changed transitions.
    let to = target
        .as_ref()
        .and_then(|id| r.actors_at(r.depth).iter().find(|a| a.id == *id))
        .map(crate::events::EventPoint::from)
        .or_else(|| {
            x.as_ref()
                .zip(y.as_ref())
                .map(|(x, y)| crate::events::EventPoint { x: *x, y: *y })
        });
    crate::events::actor_event(r, &r.player, "pulse", label, to, None);
}

/// Exact authored decimal-energy mentions; catalog text is immutable ASCII prose.
fn declared_energy_cost(description: &str) -> f64 {
    let bytes = description.as_bytes();
    let mut best: f64 = 0.0;
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let end = i;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i > end && description[i..].starts_with("energy") {
            best = best.max(description[start..end].parse::<f64>().unwrap());
        }
    }
    best
}
pub fn highest_energy_cost(r: &mut Run, target_id: f64) -> Result<f64, EngineError> {
    let revision=r.simulation_revision.value_or(29.);
    let target = crate::engine::actor_mut_trusted(r, target_id)
        .ok_or_else(|| EngineError("Unspeak target missing".into()))?;
    Ok(highest_energy_cost_for(target, revision))
}
fn highest_energy_cost_for(target: &mut Actor, revision: f64) -> f64 {
    crate::enemy_traits::initialize_enemy_traits_revision(target,revision);
    let mut cost = crate::engine::condition(target, "maxAbilityCost").max(0.0);
    if revision>=33. && target.abilities.iter().any(|s|s=="enemy:beast:witch") { cost=cost.max(10.); }
    for n in crate::enemy_traits::energy_ability_costs(target) {
        cost = cost.max(n);
    }
    for id in &target.abilities {
        if let Some(ability) = crate::progression::catalog_for_revision(revision).iter().find(|a| &a.id == id) {
            let declared = if revision >= 45.0 && id == "anavani-nova" {
                nova_attack_cost(revision)
            } else {
                declared_energy_cost(&ability.description)
            };
            cost = cost.max(declared);
        }
    }
    cost
}
pub fn refresh_unspeak(r: &mut Run) -> Result<(), EngineError> {
    if !r.player.active.iter().any(|s| s == "dawel-unspeak") {
        return Ok(());
    }
    let target = r
        .player
        .conditions
        .get("unspeakTarget")
        .copied()
        .filter(|id| {
            r.actors_at(r.depth)
                .iter()
                .any(|a| a.id == *id && !a.dead.value_or(false)
                    && crate::dreamwalking::same_plane(r, &r.player, a))
        });
    let cost = match target {
        Some(id) => highest_energy_cost(r, id)?,
        None => 0.0,
    };
    r.player.conditions.insert("unspeakCost".into(), cost);
    if target.is_none() {
        r.player.active.retain(|s| s != "dawel-unspeak");
    }
    Ok(())
}
/// Activation prepayment survives save/rewind in actor conditions. It covers the
/// next normal upkeep boundary; haste and free toggles do not advance that boundary.
/// Two keyed lookups, plus the existing reserve payment/owner journal costs.
pub fn prepaid_upkeep(r: &Run, actor: &Actor, id: &str) -> f64 {
    if !crate::revision50::enabled(r) { return 0.0; }
    let until = actor.conditions.get(&format!("upkeepPaidThrough:{id}"));
    if until.is_some_and(|t| *t >= r.tick && *t <= r.tick + 1.0) {
        actor.conditions.get(&format!("upkeepPaidAmount:{id}")).copied().unwrap_or(0.0).max(0.0)
    } else { 0.0 }
}
/// Remaining upkeep for an immediate-benefit activation, after this turn's credit.
pub fn activation_upkeep_due(r: &Run, id: &str) -> f64 {
    (upkeep_cost_in(r, &r.player, id) - prepaid_upkeep(r, &r.player, id)).max(0.0)
}
pub fn prepay_activation_upkeep(r: &mut Run, id: &str, cost: f64) -> bool {
    if !crate::revision50::enabled(r) { return true; }
    let paid = prepaid_upkeep(r, &r.player, id);
    let due = (cost - paid).max(0.0);
    if due == 0.0 { return true; }
    if energy_activation_silenced(r, &r.player)
        || !crate::revision50_ability_energy::spend(r, r.player.id, id, due) { return false; }
    record_activation_upkeep(r, id, cost, paid);
    true
}
fn record_activation_upkeep(r: &mut Run, id: &str, cost: f64, paid: f64) {
    if paid >= cost { return; }
    if paid == 0.0 {
        r.player.conditions.insert(format!("upkeepPaidThrough:{id}"), r.tick + 1.0);
    }
    r.player.conditions.insert(format!("upkeepPaidAmount:{id}"), cost);
}
pub fn upkeep_payment_condition(k: &str) -> bool {
    ["upkeepPaidThrough:", "upkeepPaidAmount:"].iter().any(|prefix|
        k.strip_prefix(prefix).is_some_and(|id| upkeep_cost(id) > 0.0 || id == "dawel-unspeak"))
}
/// Authored maintenance costs; admitted modes have no terrain-loss settlement.
pub fn upkeep_cost(id: &str) -> f64 {
    match id {
        "numo-mind-read" | "ursa-dream-see" => 1.0,
        "dawel-shadow-in-the-dark" => 2.0,
        "dawel-not-there" | "pelian-fated-duel" => 3.0,
        "etill-phase-through" | "grisha-black-pit" => 5.0,
        "morae-bonded-by-unseen-threads" => 8.0,
        "etill-riptide" => 10.0,
        "etill-rushing-torrent" => 15.0,
        "morae-turning-the-wheel" => 20.0,
        _ => 0.0,
    }
}
/// Unspeak's maintained cost is refreshed from its live target before upkeep.
pub fn upkeep_cost_for(actor: &Actor, id: &str) -> f64 {
    if id == "dawel-unspeak" {
        actor.conditions.get("unspeakCost").copied().unwrap_or(0.0)
    } else {
        upkeep_cost(id)
    }
}
pub fn upkeep_cost_in(_r:&Run,actor:&Actor,id:&str)->f64 {
    upkeep_cost_for(actor,id)
}
/// Read-only forecast for the next normal upkeep boundary, not free/haste
/// actions or contingent procs. Uses canonical atomic own-reserve-first plans
/// in active order. Cannot spend or reveal any new world state.
/// O(B+A*L) expected reserve/active metadata work, plus optional current-floor
/// Unspeak target scan and copied-target trait projection. O(B*L) scratch.
#[derive(Default,Debug,PartialEq)]
pub struct UpkeepProjection {
    pub player_energy:f64,
    pub reserve_energy:indexmap::IndexMap<String,f64>,
}
pub fn projected_player_upkeep(r:&Run)->f64 {projected_upkeep(r).player_energy}
pub fn projected_upkeep(r:&Run)->UpkeepProjection {
    let mut result=UpkeepProjection::default();
    if r.status!="playing" || r.player.hp<=0. {return result;}
    let actor=&r.player;let next_tick=r.tick+1.;let revision=r.simulation_revision.value_or(29.);
    let mut energy=actor.energy;let mut reserves=actor.ability_reserves.as_ref().cloned();
    for id in &actor.active {
        if actor.conditions.get(&format!("remembered:{id}")).is_some_and(|until|next_tick>=*until) {continue;}
        if id=="pelian-fated-duel" && !crate::revision50::duel_upkeep_valid(r,actor) {continue;}
        let cost=if id=="dawel-unspeak" {
            actor.conditions.get("unspeakTarget").and_then(|id|r.actors_at(r.depth).iter().find(|a|a.id==*id&&!a.dead.value_or(false)&&crate::dreamwalking::same_plane(r,actor,a)))
                .map(|a|highest_energy_cost_for(&mut a.clone(),revision)).unwrap_or(0.)
        } else {upkeep_cost_in(r,actor,id)};
        let prepaid=if crate::revision50::enabled(r)&&actor.conditions.get(&format!("upkeepPaidThrough:{id}")).is_some_and(|until|*until>=next_tick&&*until<=next_tick+1.) {
            actor.conditions.get(&format!("upkeepPaidAmount:{id}")).copied().unwrap_or(0.).max(0.)
        } else {0.};
        let due=(cost-prepaid).max(0.);if due==0. {continue;}
        if crate::revision50::enabled(r) {
            if let Ok(plan)=crate::revision50_ability_energy::plan_payment_for_revision(reserves.as_ref(),id,energy,due,revision) {
                if let Some(reserves)=reserves.as_mut(){
                    if crate::revision50_ability_energy::commit_payment(&plan,reserves,&mut energy).is_ok() {
                        *result.reserve_energy.entry(id.clone()).or_default()+=plan.reserve_energy_spent();
                    }
                }
            }
        } else if energy>=due {energy-=due;}
    }
    result.player_energy=(actor.energy-energy).max(0.);
    result
}
/// O(active abilities), paid in active-array order at the source upkeep point.
pub fn pay_upkeep(r: &mut Run, actor_id: f64) -> Result<bool, EngineError> {
    crate::revision50::settle_duel_owner(r, actor_id);
    let mut lost_phase = false;
    let Some(a) = crate::engine::actor(r, actor_id) else {
        return Ok(false);
    };
    if crate::half_here::is_clone(a) {
        return Ok(false);
    }
    let active = a.active.clone();
    for id in active {
        let actor = crate::engine::actor(r, actor_id).unwrap();
        let cost = (upkeep_cost_in(r, actor, &id) - prepaid_upkeep(r, actor, &id)).max(0.0);
        if cost == 0.0 {
            continue;
        }
        let a = crate::engine::actor(r, actor_id).unwrap().clone();
        let paid = if crate::revision50::enabled(r) {
            crate::revision50_ability_energy::spend(r, actor_id, &id, cost)
        } else if a.energy >= cost {
            resources::record_loss(r, &a, "energy", cost);
            crate::engine::actor_mut_trusted(r, actor_id).unwrap().energy -= cost;
            true
        } else { false };
        if paid && crate::revision50::enabled(r) {
            let tick = r.tick;
            let actor = crate::engine::actor_mut_trusted(r, actor_id).unwrap();
            let amount_key = format!("upkeepPaidAmount:{id}");
            let amount = actor.conditions.get(&amount_key).copied().unwrap_or(0.0);
            let through_key = format!("upkeepPaidThrough:{id}");
            let same_turn = actor.conditions.get(&through_key).is_some_and(|t| *t == tick);
            actor.conditions.insert(amount_key, cost + if same_turn { amount } else { 0.0 });
            actor.conditions.insert(through_key, tick);
        }
        if !paid {
            crate::engine::actor_mut_trusted(r, actor_id)
                .unwrap()
                .active
                .retain(|s| s != &id);
            if actor_id == r.player.id || vision::visible_actor(r, &a) {
                let name = &crate::progression::catalog_for_revision(r.simulation_revision.value_or(29.0))
                    .iter()
                    .find(|a| a.id == id)
                    .unwrap()
                    .name;
                log(
                    r,
                    format!("{name} deactivated: insufficient energy."),
                    if actor_id == r.player.id {
                        "player"
                    } else {
                        "world"
                    },
                );
            }
            if id == "etill-phase-through" && actor_id == r.player.id {
                crate::divine_environment::settle_phase_loss(r, true).map_err(EngineError)?;
                lost_phase = true;
            } else if id == "etill-phase-through" && crate::revision50::enabled(r) {
                crate::mini_me::settle_phase_loss(r, actor_id)?;
                if crate::engine::actor(r,actor_id).is_none_or(|a|a.dead.value_or(false) || a.hp<=0.0) {
                    return Ok(lost_phase);
                }
            }
        }
    }
    Ok(lost_phase)
}
/// Free-command boundary funding check, after costs but before observation.
pub fn remove_unfunded(r: &mut Run) -> bool {
    let mut lost_phase = false;
    if r.status != "playing" {
        return false;
    }
    let active = r.player.active.clone();
    for id in active {
        if !crate::revision50_ability_energy::can_pay(r,r.player.id,&id,
            (upkeep_cost_in(r,&r.player,&id) - prepaid_upkeep(r,&r.player,&id)).max(0.0)) {
            lost_phase |= id == "etill-phase-through";
            r.player.active.retain(|s| s != &id);
            let name = &crate::progression::catalog_for_revision(r.simulation_revision.value_or(29.0))
                .iter()
                .find(|a| a.id == id)
                .unwrap()
                .name;
            log(
                r,
                format!("{name} deactivated: insufficient energy."),
                "player",
            );
        }
    }
    lost_phase
}
pub fn disable_unfunded(r: &mut Run) -> Result<(), EngineError> {
    crate::revision50::settle_duel(r);
    if remove_unfunded(r) {
        crate::divine_environment::settle_phase_loss(r, false).map_err(EngineError)?;
    }
    Ok(())
}

/// Authored status removal, O(C + R*K): C conditions, R remembered grants, K abilities.
pub fn clear_status_conditions(actor: &mut Actor) {
    crate::revision52_enemy_followups::clear_bleed(actor);
    actor.conditions.shift_remove(crate::revision52_enemy_followups::FLIGHT);
    const KEYS: &[&str] = &[
        "hemophilia",
        "hemophiliaSourceId",
        "poisonSourceId",
        "slowed",
        "slowSourceId",
        "paralyzed",
        "magicSensitive",
        "magicSensitiveFresh",
        "bleeding",
        "bleedSourceId",
        "burning",
        "poisoned",
        "weakened",
        "emboldened",
        "lucky",
        "unlucky",
        "distracted",
        "flying",
        "flyingFresh",
        "telepathic",
        "telepathicFresh",
        "invisible",
        "invisibleFresh",
        "confused",
        "confusedFresh",
        "hasted",
        "hastedFresh",
        "silenced",
        "riptide",
        "gasVulnerable",
        "gasVulnerableTicks",
        "screamWeakened",
        "disoriented",
        "hallucinating",
        "screamPenalty",
        "screamEmboldened",
        "screamBonus",
        "screamPerception",
        "temporaryAlly",
        "stuck",
        "grabbed",
        "grabbedBy",
        "envenomed",
        "venomTurns",
        "venomSourceId",
    ];
    let keys: Vec<_> = actor
        .conditions
        .keys()
        .filter(|k| {
            KEYS.contains(&k.as_str())
                || ["venom:", "buff:", "buffTicks:", "remembered:"]
                    .iter()
                    .any(|p| k.starts_with(p))
        })
        .cloned()
        .collect();
    for key in keys {
        if key == "burning" && actor.abilities.iter().any(|s| s == "enemy:beast:everburn") {
            continue;
        }
        if let Some(id) = key.strip_prefix("remembered:") {
            actor.abilities.retain(|s| s != id);
            actor.active.retain(|s| s != id);
        }
        if key == "temporaryAlly" && crate::engine::condition(actor, &key) > 0.0 {
            actor.state = "wandering".into();
            actor.perfect = false.into();
        }
        actor.conditions.shift_remove(&key);
    }
}

/// Owner-only normal-turn counter; a skipped tick grants at most one recharge.
pub fn tick_nightmare(r: &mut Run, id: f64) {
    let tick = r.tick;
    let Some(a) = crate::engine::actor_mut_trusted(r, id) else {
        return;
    };
    if crate::half_here::is_clone(a) || !a.abilities.iter().any(|s| s == "ursa-waking-nightmare") {
        return;
    }
    let at = *a
        .conditions
        .entry("nightmareAt".into())
        .or_insert(tick + 99.0);
    if tick >= at {
        a.nightmares = (a.nightmares.value_or(0.0) + 1.0).into();
        a.conditions.insert("nightmareAt".into(), at + 100.0);
    }
}

#[cfg(test)]
mod dream_plane_tests {
    use super::*;
    fn room() -> Run {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 34.0.into();
        r.player.energy = 1000.;
        r.player.conditions.clear();
        r.player.active.clear();
        r.dreamwalking = crate::dreamwalking::State {
            active: true, depth: r.depth, ..Default::default()
        }.into();
        r
    }
    #[test]
    fn ahvamitt_allegiance_is_protected_before_cost_without_disabling_other_powers() {
        let mut r = room(); r.dreamwalking.as_mut().unwrap().active = false;
        let mut godbeast = crate::ahvamitt::actor(90., 16., 14.); godbeast.hp = 10.;
        r.actors_at_mut(1.).clear(); r.actors_at_mut(1.).push(godbeast.clone());
        for id in ["numo-implant-thought", "numo-mind-slave", "morae-sever-fate"] {
            r.player.abilities.push(id.into());
            activate(&mut r, id, &90.0.into(), &Field::Missing, &Field::Missing).unwrap();
            assert_eq!(r.player.energy, 1000.);
            assert_eq!(crate::engine::actor(&r, 90.), Some(&godbeast));
        }
        r.player.abilities.push("numo-distract".into());
        activate(&mut r, "numo-distract", &90.0.into(), &Field::Missing, &Field::Missing).unwrap();
        assert_eq!(r.player.energy, 990.);
        assert_eq!(crate::engine::actor(&r, 90.).unwrap().conditions.get("distracted"), Some(&1.));
    }
    #[test]
    fn every_actor_target_rejects_another_plane_before_cost_or_mutation() {
        for id in ["pelian-i-came-for-you", "numo-implant-thought", "numo-distract",
            "numo-mind-slave", "numo-remember", "isheilah-make-slippery", "etill-riptide",
            "dawel-unspeak", "morae-bonded-by-unseen-threads", "morae-sever-fate",
            "morae-fortune-teller", KNEEL]
        {
            let mut r = room();
            r.player.abilities.push(id.into());
            let enemy = r.actors_at_mut(1.).first_mut().unwrap();
            enemy.hp = 1.;
            let before_target = enemy.clone();
            let before_player = r.player.clone();
            let before_rng = r.rng;
            assert!(!activate(&mut r, id, &before_target.id.into(), &13.0.into(), &14.0.into()).unwrap(), "{id}");
            assert_eq!(r.player, before_player, "player was mutated by {id}");
            assert_eq!(crate::engine::actor(&r, before_target.id), Some(&before_target), "target was mutated by {id}");
            assert_eq!(r.rng, before_rng, "{id}");
        }
        assert!(!targets_actor("kative-tinker"));
        assert!(!targets_actor("craft:zeal"));
    }
    #[test]
    fn matching_plane_and_legacy_targeting_still_work_and_cross_plane_link_can_stop() {
        for (revision, dream_target) in [(34., true), (33., false)] {
            let mut r = room();
            r.simulation_revision = revision.into();
            r.player.abilities.push(KNEEL.into());
            let enemy = r.actors_at_mut(1.).first_mut().unwrap();
            let id = enemy.id;
            if dream_target { enemy.conditions.insert("phantasmiiPlane".into(), 1.); }
            activate(&mut r, KNEEL, &id.into(), &Field::Missing, &Field::Missing).unwrap();
            assert_eq!(r.player.energy, 980.);
            assert_eq!(crate::engine::actor(&r, id).unwrap().conditions.get("paralyzed"), Some(&10.));
        }
        let mut r = room();
        r.player.abilities.push("etill-riptide".into());
        r.player.active.push("etill-riptide".into());
        let target = r.actors_at(1.)[0].id;
        activate(&mut r, "etill-riptide", &target.into(), &Field::Missing, &Field::Missing).unwrap();
        assert!(!r.player.active.iter().any(|id| id == "etill-riptide"));
        assert_eq!(r.player.energy, 1000.);
    }
    #[test]
    fn sleeping_body_has_no_owner_toggles_but_half_here_keeps_them() {
        let mut r = room();
        r.player.active.push("morae-turning-the-wheel".into());
        let mut body = r.player.clone(); body.id = 90.; body.active.clear();
        body.conditions.insert("phantasmiiBody".into(), 1.);
        body.conditions.insert("halfHereOwner".into(), r.player.id);
        assert!(!endpoint_active(&r, &body, "morae-turning-the-wheel"));
        assert!(endpoint_active(&r, &r.player, "morae-turning-the-wheel"));
        body.conditions.shift_remove("phantasmiiBody");
        body.conditions.insert("halfHereClone".into(), 1.);
        assert!(endpoint_active(&r, &body, "morae-turning-the-wheel"));
    }
    #[test]
    fn silence_aura_and_maintained_unspeak_do_not_cross_planes() {
        let mut r = room();
        let (px, py) = (r.player.x, r.player.y);
        let enemy = r.actors_at_mut(1.).first_mut().unwrap();
        enemy.x = px + 1.; enemy.y = py;
        enemy.abilities.push("enemy:beast:silence-aura".into());
        let target = enemy.id;
        assert!(!energy_activation_silenced(&r, &r.player));
        crate::engine::actor_mut_trusted(&mut r, target).unwrap().conditions.insert("phantasmiiPlane".into(), 1.);
        assert!(energy_activation_silenced(&r, &r.player));
        crate::engine::actor_mut_trusted(&mut r, target).unwrap().conditions.shift_remove("phantasmiiPlane");
        r.player.active.push("dawel-unspeak".into());
        r.player.conditions.insert("unspeakTarget".into(), target);
        refresh_unspeak(&mut r).unwrap();
        assert!(!r.player.active.iter().any(|id| id == "dawel-unspeak"));
        assert_eq!(r.player.conditions.get("unspeakCost"), Some(&0.));
    }
    #[test]
    fn dream_plane_sparkle_and_bulwark_mutate_only_the_dream_floor() {
        let mut r = room();
        let target = r.actors_at(1.)[0].clone();
        let at = target.y as usize * r.floor().unwrap().width() + target.x as usize;
        r.player.abilities.extend([SPARKLE.into(), BULWARK.into()]);
        r.floor_mut().unwrap().tiles[at] = "grass".into();
        activate(&mut r, SPARKLE, &Field::Missing, &target.x.into(), &target.y.into()).unwrap();
        assert_eq!(r.player.energy, 980.);
        assert_eq!(r.floor().unwrap().tiles[at], "grass");
        assert!(!r.floor().unwrap().permanent_fire.as_ref().is_some_and(|v| v.contains(&(at as f64))));
        assert!(crate::dreamwalking::floor_for(&r, &r.player).unwrap().permanent_fire.as_ref().unwrap().contains(&(at as f64)));
        // The physical monster on this cell is not an occupant of the dream layer.
        activate(&mut r, BULWARK, &Field::Missing, &target.x.into(), &target.y.into()).unwrap();
        assert_eq!(r.player.energy, 960.);
        assert!(!r.floor().unwrap().bulwarks.as_ref().is_some_and(|v| v.contains_key(&at.to_string())));
        assert!(crate::dreamwalking::floor_for(&r, &r.player).unwrap().bulwarks.as_ref().unwrap().contains_key(&at.to_string()));
        assert!(!crate::dreamwalking::context_dream(&r));
    }
}

#[cfg(test)]
mod reaction_attack_reference {
    use super::*;
    #[test]
    fn retained_reaction_sources_and_offhand() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/reaction-attacks-v48.json");
        let cases: serde_json::Value =
            serde_json::from_reader(std::fs::File::open(path).unwrap()).unwrap();
        for case in cases.as_array().unwrap() {
            let mut r: Run = serde_json::from_value(case["initial"].clone()).unwrap();
            crate::events::begin();
            if case["ranged"] == true {
                let hit = crate::engine::attack_ranged(
                    &mut r,
                    case["aid"].as_f64().unwrap(),
                    case["bid"].as_f64().unwrap(),
                    30.0,
                    2.0,
                    1.0,
                    case["arrow"] == true,
                )
                .unwrap();
                assert_eq!(
                    hit,
                    case["hit"].as_bool().unwrap(),
                    "{} onHit",
                    case["mode"]
                );
            } else if case["playerAction"] == true {
                crate::engine::player_attack(&mut r, case["bid"].as_f64().unwrap()).unwrap();
            } else {
                crate::engine::attack(
                    &mut r,
                    case["aid"].as_f64().unwrap(),
                    case["bid"].as_f64().unwrap(),
                    1.0,
                )
                .unwrap();
            }
            let events = crate::events::finish();
            assert!(
                crate::storage::canonical_equal(
                    &serde_json::to_value(events).unwrap(),
                    &case["events"]
                ),
                "{} events",
                case["mode"]
            );
            let actual = serde_json::to_value(&r).unwrap();
            let expected: Run = serde_json::from_value(case["state"].clone()).unwrap();
            let expected = serde_json::to_value(expected).unwrap();
            for (key, value) in actual.as_object().unwrap() {
                assert!(
                    crate::storage::canonical_equal(value, &expected[key]),
                    "{} field {key}: {value} != {}",
                    case["mode"],
                    expected[key]
                );
            }
            assert!(
                crate::storage::canonical_equal(&actual, &expected),
                "{} missing key",
                case["mode"]
            );
        }
    }
}

#[cfg(test)]
mod remember_passive_reference {
    #[test]
    fn copies_authored_divine_passives() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/remember-passives-v48.json");
        let cases: serde_json::Value =
            serde_json::from_reader(std::fs::File::open(path).unwrap()).unwrap();
        for case in cases.as_array().unwrap() {
            let mut run: crate::Run = serde_json::from_value(case["initial"].clone()).unwrap();
            crate::events::begin();
            super::activate(
                &mut run,
                "numo-remember",
                &crate::Field::Value(4.0),
                &crate::Field::Missing,
                &crate::Field::Missing,
            )
            .unwrap();
            let events = crate::events::finish();
            assert!(
                crate::storage::canonical_equal(
                    &serde_json::to_value(events).unwrap(),
                    &case["events"]
                ),
                "{} events",
                case["name"]
            );
            let expected: crate::Run = serde_json::from_value(case["state"].clone()).unwrap();
            assert!(
                crate::storage::canonical_equal(
                    &serde_json::to_value(run).unwrap(),
                    &serde_json::to_value(expected).unwrap()
                ),
                "{} state",
                case["name"]
            );
        }
    }
}

#[cfg(test)] mod remember_animation_fidelity_tests {
 use super::*;
 #[test] fn remember_encodes_only_real_new_traits_and_visible_apparent_outline(){
  let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();
  r.player.x=2.;r.player.y=2.;r.player.conditions.clear();r.player.active.clear();r.player.abilities=vec!["numo-remember".into()];r.player.ability_reserves=Field::Missing;r.inventory.clear();
  crate::revision50_ability_energy::initialize_reserve(&mut r.player,"numo-remember",20.,20.).unwrap();
  let mut enemy=r.player.clone();enemy.id=900.;enemy.x=3.;enemy.state="hunting".into();enemy.abilities=vec!["shrail-fight-long".into()];enemy.ability_reserves=Field::Missing;enemy.glyph="g".into();
  r.actors_at_mut(r.depth).clear();r.actors_at_mut(r.depth).push(enemy);let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.portals=Field::Missing;f.non_euclidean=Field::Missing;
  crate::events::begin();activate(&mut r,"numo-remember",&900.0.into(),&Field::Missing,&Field::Missing).unwrap();let events=crate::events::finish();
  assert!(r.player.abilities.iter().any(|s|s=="shrail-fight-long"));
  assert!(events.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.style=="fracture-trait:shrail-fight-long")));
  assert!(events.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.style=="fold-outline")&&e.glyph.as_deref()==Some("g")));
  crate::events::begin();activate(&mut r,"numo-remember",&900.0.into(),&Field::Missing,&Field::Missing).unwrap();assert!(!crate::events::finish().iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="P24")));
 }
}

#[cfg(test)] mod animation_blood_letting_peel_tests {
 use super::*;
 #[test] fn only_actual_removed_player_statuses_peel_without_source_metadata(){
  let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();r.player.abilities=vec!["lichor-blood-letting".into()];r.player.active.clear();r.player.conditions.clear();r.player.hp=100.;r.inventory.clear();
  for (key,value) in [("poisoned",3.),("paralyzed",4.),("poisonSourceId",999.),("unrelatedMetadata",7.)] {r.player.conditions.insert(key.into(),value);}
  crate::events::begin();activate(&mut r,"lichor-blood-letting",&Field::Missing,&Field::Missing,&Field::Missing).unwrap();let e=crate::events::finish();let peels:Vec<_>=e.iter().filter_map(|e|e.animation.as_ref()).map(|a|a.style.as_str()).filter(|s|s.starts_with("fracture-status:")).collect();
  assert_eq!(peels,vec!["fracture-status:paralyzed+poisoned"]);assert_eq!(r.player.conditions.get("unrelatedMetadata"),Some(&7.));
  crate::events::begin();activate(&mut r,"lichor-blood-letting",&Field::Missing,&Field::Missing,&Field::Missing).unwrap();assert!(!crate::events::finish().iter().filter_map(|e|e.animation.as_ref()).any(|a|a.style.starts_with("fracture-status:")));
 }
}

#[cfg(test)] mod upkeep_forecast_tests {
    use super::*;
    fn room()->Run {
        let mut r=crate::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();r.offers.clear();r.player.active.clear();r.player.ability_reserves=crate::Field::Missing;
        r.player.energy=50.;r
    }
    fn add(r:&mut Run,id:&str,reserve:f64){
        r.player.abilities.push(id.into());r.player.active.push(id.into());
        crate::revision50_ability_energy::initialize_reserve(&mut r.player,id,10.,reserve).unwrap();
    }
    fn actual(r:&Run)->f64{let mut next=r.clone();next.tick+=1.;let id=next.player.id;let before=next.player.energy;pay_upkeep(&mut next,id).unwrap();before-next.player.energy}
    #[test] fn forecast_matches_next_boundary_and_own_reserve_first_without_mutation(){
        let mut r=room();add(&mut r,"morae-turning-the-wheel",10.);add(&mut r,"etill-riptide",4.);
        let before=r.clone();
        let forecast=projected_upkeep(&r);
        assert_eq!(forecast.reserve_energy.get("morae-turning-the-wheel"),Some(&10.));
        assert_eq!(forecast.reserve_energy.get("etill-riptide"),Some(&4.));
        assert_eq!(forecast.player_energy,16.);
        assert_eq!(projected_player_upkeep(&r),16.);assert_eq!(projected_player_upkeep(&r),actual(&r));assert_eq!(r,before);
        r.player.active.clear();assert_eq!(projected_player_upkeep(&r),0.);
    }
    #[test] fn forecast_honors_prepaid_next_turn_but_not_previous_turn_credit(){
        let mut r=room();let id="morae-turning-the-wheel";add(&mut r,id,0.);
        r.player.conditions.insert(format!("upkeepPaidThrough:{id}"),r.tick+1.);
        r.player.conditions.insert(format!("upkeepPaidAmount:{id}"),20.);
        assert_eq!(projected_player_upkeep(&r),0.);assert_eq!(actual(&r),0.);
        r.player.conditions.insert(format!("upkeepPaidThrough:{id}"),r.tick);
        assert_eq!(projected_player_upkeep(&r),20.);assert_eq!(actual(&r),20.);
    }
    #[test] fn forecast_does_not_invent_payment_for_unaffordable_or_expired_modes(){
        let mut r=room();add(&mut r,"morae-turning-the-wheel",0.);add(&mut r,"etill-phase-through",0.);r.player.energy=15.;
        assert_eq!(projected_player_upkeep(&r),5.);assert_eq!(actual(&r),5.);
        r.player.conditions.insert("remembered:etill-phase-through".into(),r.tick+1.);
        assert_eq!(projected_player_upkeep(&r),0.);
    }
}

#[cfg(test)] mod authored_animation_cast_tests {
 use super::*;
 fn export(name:&str,before:&Run,after:&Run,events:&[crate::events::PresentationEvent]) {
  if let Ok(dir)=std::env::var("CORPSE_ANIMATION_EXPORT_DIR") {
   std::fs::create_dir_all(&dir).unwrap();std::fs::write(std::path::Path::new(&dir).join(format!("{name}.json")),serde_json::to_vec(&serde_json::json!({"before":before,"after":after,"events":events})).unwrap()).unwrap();
  }
 }
 fn room()->Run {
  let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
  r.simulation_revision=52.0.into();r.player.conditions.clear();r.player.active.clear();r.player.abilities.clear();r.player.energy=1000.;r.player.x=8.;r.player.y=8.;
  r.player.ability_reserves=Field::Missing; r.actors_at_mut(r.depth).clear();
  let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);r
 }
 #[test] fn fortune_self_nova_and_repeated_enemy_cast_have_real_receipts(){
  let mut r=room();let id="morae-fortune-teller";r.player.abilities.push(id.into());
  let mut target=r.player.clone();target.id=900.;target.x=10.;target.state="hunting".into();target.abilities.clear();r.actors_at_mut(r.depth).push(target);
  for (tid,style) in [(r.player.id,"nova-blue-small"),(900.,"line-grow"),(900.,"line-grow")] {
   let before=r.clone();crate::events::begin();activate(&mut r,id,&tid.into(),&Field::Missing,&Field::Missing).unwrap();
   let out=crate::events::finish();export(if tid==r.player.id {"fortune-self"}else{"fortune-enemy"},&before,&r,&out);assert!(out.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="P67" && a.style==style)),"{style}");
  }
 }
 #[test] fn sparkle_travels_as_bright_light_before_local_impact(){
  let mut r=room();r.player.abilities.push(SPARKLE.into());crate::revision50_ability_energy::initialize_reserve(&mut r.player,SPARKLE,10.,10.).unwrap();let before=r.clone();crate::events::begin();
  activate(&mut r,SPARKLE,&Field::Missing,&10.0.into(),&8.0.into()).unwrap();
  let out=crate::events::finish();export("sparkle",&before,&r,&out);let cues:Vec<_>=out.iter().filter_map(|e|e.animation.as_ref()).filter(|a|a.proposal=="P58").collect();
  assert_eq!(cues[0].style,"light100");assert_eq!(cues[0].delivery,crate::events::Delivery::Magic);assert!(cues.len()>=2);
 }
 #[test] fn all_seeing_has_radial_reveal_centered_on_player(){
  let mut r=room();r.player.abilities.push(ALL_SEEING.into());crate::revision50_ability_energy::initialize_reserve(&mut r.player,ALL_SEEING,10.,10.).unwrap();let before=r.clone();crate::events::begin();
  activate(&mut r,ALL_SEEING,&Field::Missing,&Field::Missing,&Field::Missing).unwrap();
  let out=crate::events::finish();export("all-seeing-eyes",&before,&r,&out);let e=out.iter().find(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="P72")).unwrap();
  assert_eq!(e.animation.as_ref().unwrap().style,"map-reveal");assert_eq!(e.to,(&r.player).into());assert!(e.animation.as_ref().unwrap().cells.len()>1);
 }
}
