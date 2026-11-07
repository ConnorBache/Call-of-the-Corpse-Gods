//! v48 stationary clean-floor catch-up and exact admitted dirty-floor phases.
//! Unsupported trait/ally/transfer paths fail before mutation.
use crate::{Actor, Field, Floor, Run};
use std::collections::HashSet;
fn c(a: &Actor, k: &str) -> f64 {
    a.conditions.get(k).copied().unwrap_or(0.)
}
/// Clay attacks require exact turns even if all adjacent actors are stationary.
/// O(F+M) floor/modification lookup, including an inactive dream-plane floor.
fn has_clay(r:&Run,depth:f64)->bool {
    crate::revision50::enabled(r) && r.floors.iter()
        .chain(r.dreamwalking.as_ref().into_iter().flat_map(|s|s.floors.iter()))
        .filter(|f|f.depth==depth).any(|f|f.revision50_terrain.as_ref().into_iter().flatten()
            .any(|m|m.kind==crate::revision50_terrain::Kind::ClaySoldier))
}
fn dynamic(r: &Run, f: &Floor) -> bool {
    let _timing_function = crate::timing::span("offscreen.dynamic_guard");
    if has_clay(r,f.depth) {return true;}
    if crate::revision50_control_amulets::has_temporary_terrain(r,f.depth) {return true;}
    if crate::terrain_runtime::enabled(r) && f.hellfire.as_ref().is_some_and(|v|v.iter().any(|v|*v)) {return true;}
    if r.simulation_revision.value_or(29.)>=34. && f.godbeast_event.as_ref().is_some_and(|e|e.kind=="Epidite") {return true;}
    // Ecology needs exact normal turns: no stationary catch-up may skip a meal.
    if crate::creature_feeding::enabled(r) && r.actors_at(f.depth).iter().any(|a| !a.dead.value_or(false) && a.hp > 0.0) {
        return true;
    }
    if r.actors_at(f.depth).iter().any(|a| {
        crate::corpses::has_permanent_burning(a)
            && (!a.dead.value_or(false)
                || (crate::corpses::can_revive_corpse(a)
                    && !crate::environment::corpse_rooted(f, r.actors_at(f.depth), a)))
    }) {
        return true;
    }
    if r.actors_at(f.depth).iter().any(|a| {
        !a.dead.value_or(false) && a.abilities.iter().any(|id| id == "enemy:beast:fungal-gas")
    }) {
        return true;
    }
    if crate::water_blood::enabled(r) && crate::water_blood::active(f) {
        return true;
    }
    if f.bulwarks
        .as_ref()
        .is_some_and(|b| b.values().any(|&v| v > f.last_tick))
        || f.vents.as_ref().is_some_and(|v| !v.is_empty())
        || f.permanent_fire.as_ref().is_some_and(|v| !v.is_empty())
        || f.altered_chasms.as_ref().is_some_and(|v| !v.is_empty())
    {
        return true;
    }
    if let Some(e) = f.environment.as_ref() {
        if e.fire_kind
            .as_ref()
            .is_some_and(|v| v.iter().any(Option::is_some))
            || e.gas_type
                .as_ref()
                .is_some_and(|v| v.iter().any(Option::is_some))
            || e.fire.iter().any(|&v| v > 0.)
            || crate::environment::GAS_TYPES.iter().any(|g| {
                crate::environment::legacy_channel(e, g)
                    .iter()
                    .any(|&v| v > 0.)
            })
        {
            return true;
        }
    }
    if f.godbeast_event.as_ref().is_some_and(|e| {
        e.kind == "Mercuria"
            && e.activated.value_or(false)
            && (e.flood_frontier.as_ref().map_or(1, |v| v.len()) > 0
                || e.fish_spawned.value_or(0.) < 10.)
    }) {
        return true;
    }
    let w = f.width();
    if r.actors_at(f.depth).iter().any(|a| {
        !a.dead.value_or(false)
            && f.ichor
                .as_ref()
                .is_some_and(|v| v[a.y as usize * w + a.x as usize] > 0.)
    }) {
        return true;
    }
    if r.ground.get(&f.depth.to_string()).is_some_and(|v| {
        v.iter().filter(|e| !crate::dreamwalking::dream_ground_item(&e.item)).any(|e| {
            let i = e.y as usize * w + e.x as usize;
            f.tiles[i] == "water" || f.ichor.as_ref().is_some_and(|v| v[i] > 0.)
        })
    }) {
        return true;
    }
    r.actors_at(f.depth).iter().any(|a| {
        !a.dead.value_or(false)
            && (a.state == "hunting"
                || c(a, "monsterInventorFleeing") > 0.
                || !a.active.is_empty()
                || a.abilities.iter().any(|id| {
                    matches!(
                        id.as_str(),
                        "enemy:beast:everburn"
                            | "enemy:beast:razorleaves"
                            | "grisha-entropy"
                            | "anavani-star-power"
                            | "enemy:pale:breath"
                    )
                })
                || (!crate::environment::is_flying_in(r, a)
                    && crate::environment::current_destination_revision(f, a.y as usize * w + a.x as usize, r.simulation_revision.value_or(29.0))
                        .is_some())
                || a.conditions.iter().any(|(k, &v)| {
                    v > 0.
                        && ([
                            "bleeding",
                            "burning",
                            "paralyzed",
                            "telepathic",
                            "invisible",
                            "confused",
                            "magicSensitive",
                            "flying",
                            "hasted",
                            "temporaryAlly",
                            "screamWeakened",
                            "disoriented",
                            "hallucinating",
                            "screamEmboldened",
                            "gasVulnerableTicks",
                            "weakened",
                            "emboldened",
                            "envenomed",
                            "grabbed",
                            "beastCooldown",
                            "hemophilia",
                            "poisoned",
                            crate::revision50_amulet_effects::ACID_TURNS,
                            crate::revision50_control_amulets::DISCORD,
                            crate::revision50_potions::FIRE_IMMUNITY,
                            crate::revision50_control_amulets::ROOTS,
                            "slowed",
                            "raisedWallExpires",
                        ]
                        .contains(&k.as_str())
                            || k.starts_with("buffTicks:")
                            || k.starts_with("remembered:"))
                }))
    })
}
fn involved(r: &Run, f: &Floor) -> bool {
    f.depth != r.depth && (f.dirty || f.last_tick > 0. || f.seen.iter().any(|&v| v) || has_clay(r,f.depth) || crate::revision50_control_amulets::has_temporary_terrain(r,f.depth))
}
pub fn validate_supported(r: &Run) -> Result<(), String> {
    let _timing_function = crate::timing::span("offscreen.validate");
    for f in &r.floors {
        if !involved(r, f) {
            continue;
        }
        let actors = r.actors_at(f.depth);

        if actors.iter().any(|a| {
            crate::half_here::is_clone(a)
                || a.abilities.iter().any(|id| {
                    if crate::revision50::enabled(r) && id==crate::revision50_combat::MAGICKAL_CASTER {return false;}
                    if c(a, "miniMe") == 1.0 && crate::abilities::mini_me_ability_supported(id) {
                        return false;
                    }
                    ![
                        "enemy:beast:stalk",
                        "enemy:beast:sphinx",
                        "enemy:beast:slime-bound",
                        "enemy:beast:slime-trail",
                        "enemy:beast:yeti",
                        "enemy:beast:phoenix",
                        "enemy:beast:armor-piercing",
                        "enemy:beast:ursa-major",
                        "enemy:beast:detonate",
                        "enemy:beast:ruin-pulse",
                        "enemy:beast:silence-aura",
                        "enemy:beast:darkness-aura",
                        "enemy:beast:explosion-immunity",
                        "enemy:beast:aquatic",
                        "enemy:beast:submerge",
                        "enemy:beast:grab",
                        "enemy:beast:hold-only",
                        "enemy:beast:deep-water",
                        "enemy:beast:water-jet",
                        "enemy:beast:blink",
                        "enemy:beast:moon-laser",
                        "enemy:beast:horn-bolt",
                        "enemy:beast:beckon",
                        "enemy:beast:wall-seal",
                        "enemy:beast:bloodmancy",
                        "enemy:beast:witch",
                        "enemy:beast:saidribee-queen",
                        "enemy:beast:dryad",
                        "enemy:beast:slow",
                        "enemy:pale:slow",
                        "enemy:pale:gas-immunity",
                        "enemy:pale:bite-repeat",
                        "enemy:pale:lifesteal",
                        "enemy:pale:scream",
                        "enemy:pale:spit",
                        "enemy:pale:gas-shot",
                        "enemy:pale:birth",
                        "enemy:pale:double-birth",
                        "enemy:pale:breath",
                        "enemy:pale:breath-boost",
                        "enemy:pale:gas-vulnerability",
                        "enemy:beast:slime-walker",
                        "enemy:beast:venom-weak",
                        "enemy:beast:venom",
                        "enemy:beast:venom-strong",
                        "enemy:beast:ursa-minor",
                        "enemy:beast:ursa-major",
                        "enemy:beast:fast",
                        "enemy:beast:flight",
                        "enemy:beast:minotaur",
                        "enemy:beast:vampire",
                        "enemy:beast:wendigo",
                        "enemy:beast:inventor",
                        "enemy:beast:mech",
                        "enemy:beast:fire-breath",
                        "enemy:beast:lifesteal",
                        "enemy:beast:razorleaves",
                        "enemy:beast:everburn",
                        "enemy:beast:fire-immunity",
                        "enemy:beast:gas-immunity",
                        "enemy:beast:fungal-gas",
                        "enemy:beast:no-revival",
                        "enemy:beast:rooted",
                        "etill-phase-through",
                        "etill-rushing-torrent",
                        "etill-riptide",
                        "grisha-black-pit",
                        "anavani-shine",
                        "isheilah-make-slippery",
                        "dawel-reverse-entropy",
                        "grisha-entropy",
                        "anavani-star-power",
                        "anavani-ascend",
                        "etill-drown",
                        "numo-mind-read",
                        "ursa-dream-see",
                        "dawel-shadow-in-the-dark",
                        "dawel-not-there",
                        "enemy:beast:regeneration",
                        "enemy:beast:banshee",
                        "enemy:beast:ichor-bound",
                        "enemy:beast:ichor-siphon",
                    ]
                    .contains(&id.as_str())
                })
                || a.active.iter().any(|id| {
                    ![
                        "etill-phase-through",
                        "etill-rushing-torrent",
                        "etill-riptide",
                        "grisha-black-pit",
                        "anavani-shine",
                        "isheilah-make-slippery",
                        "dawel-reverse-entropy",
                        "grisha-entropy",
                        "anavani-star-power",
                        "numo-mind-read",
                        "ursa-dream-see",
                        "dawel-shadow-in-the-dark",
                        "dawel-not-there",
                    ]
                    .contains(&id.as_str())
                })
        }) {
            return Err(format!(
                "Unsupported offscreen actor traits/ally transfer on floor {}",
                f.depth
            ));
        }
        if dynamic(r, f) {
            let head = proxy(r, f.depth, r.tick);
            crate::environment::validate_supported(&head)?;
        }
    }
    Ok(())
}
fn proxy(r: &Run, depth: f64, tick: f64) -> Run {
    let _timing_function = crate::timing::span("offscreen.proxy_clone");
    let mut head = r.clone();
    head.depth = depth;
    head.tick = tick;
    head.player.id = -1.;
    head.player.x = -999.;
    head.player.y = -999.;
    head.player.hp = 0.;
    head.player.dead = true.into();
    head.player.abilities.clear();
    head.player.active.clear();
    head.player.conditions.clear();
    head.inventory.clear();
    head.log.clear();
    head.log_kinds = Vec::new().into();
    head.log_archive = Field::Missing;
    head.history.clear();
    head.replay.clear();
    head
}
fn rest(a: &mut Actor, elapsed: f64, cap: f64, suppressed: bool, revision: f64) -> bool {
    if crate::dreamwalking::is_nightmare(a) || a.dead.value_or(false)
        || a.hp <= 0.
        || crate::half_here::is_clone(a)
        || (crate::monster_inventor::is_mech(a)
            && !a
                .monster_state
                .as_ref()
                .is_some_and(|s| s.pilot.as_ref().is_some()))
    {
        return false;
    }
    let elapsed = elapsed.floor().max(0.);
    if elapsed == 0. {
        return false;
    }
    if a.state != "ally" {
        a.conditions.shift_remove("monsterAllyRest");
    }
    if a.state != "sleeping" && a.state != "neutral" {
        a.conditions.shift_remove("monsterRecoverySleep");
    }
    let old = c(a, "monsterUnseenTurns").max(0.).min(20.);
    if a.state != "ally" {
        a.conditions
            .insert("monsterUnseenTurns".into(), (old + elapsed).min(20.));
    }
    let mut resting = elapsed;
    let sleeping = crate::ai::sleeping(a);
    if !sleeping {
        if a.state == "ally" || a.energy >= cap || c(a, "monsterUnseenTurns") < 20.
            || (revision >= 51. && matches!(a.state.as_str(),"hunting"|"stalking")
                && a.last_seen.as_ref().is_some()) {
            return false;
        }
        resting = (elapsed - (20. - old).max(1.) + 1.).max(0.);
        if a.state != "neutral" {
            a.state = "sleeping".into();
        }
        a.conditions.insert("monsterRecoverySleep".into(), 1.);
        a.last_seen = Field::Missing;
        for key in [
            "targetId",
            "chaseDepth",
            "chaseExitX",
            "chaseExitY",
            "chaseLandingX",
            "chaseLandingY",
            "stalkGoalX",
            "stalkGoalY",
            "stalkObservedX",
            "stalkObservedY",
            "stalkDx",
            "stalkDy",
            "stalkWait",
            "stalkPeekTick",
        ] {
            a.conditions.shift_remove(key);
        }
        a.conditions.insert("seesPlayer".into(), 0.);
    }
    if !suppressed && a.energy < cap {
        a.energy = (a.energy + resting).min(cap)
    }
    if c(a, "monsterRecoverySleep") == 1. && a.energy >= cap {
        if a.state == "sleeping" {
            a.state = "wandering".into();
        }
        a.conditions.shift_remove("monsterRecoverySleep");
        a.conditions.shift_remove("monsterAllyRest");
    }
    true
}
/// Same intentionally stationary approximation as v48; at most 298 corpse rolls.
fn approximate(r: &mut Run, depth: f64, elapsed: f64) -> Result<(), String> {
    let _timing_function = crate::timing::span("offscreen.approximate");
    let f = r.floors.iter().find(|f| f.depth == depth).unwrap();
    let width = f.width();
    let last = f.last_tick;
    let mut actors = r.actors_at(depth).to_vec();
    let mut occupied: HashSet<_> = actors
        .iter()
        .filter(|a| !a.dead.value_or(false))
        .map(|a| a.y as usize * width + a.x as usize)
        .collect();
    for a in &mut actors {
        if crate::revision50_nightmares::is_decoy(a) { continue; }
        if c(a, "monsterAudibleUntil") <= r.tick {
            for key in ["monsterAudibleUntil", "monsterAudibleX", "monsterAudibleY"] {
                a.conditions.shift_remove(key);
            }
        }
        let cap = crate::combat::effective_stats(r, a).max_energy;
        if !a.dead.value_or(false) {
            let f = r.floors.iter().find(|f| f.depth == depth).unwrap();
            regen(r, f, a, elapsed, frozen(r.actors_at(depth), a));
            prepare_ally_rest(&proxy(r, depth, r.tick), a);
            rest(a, elapsed, cap, frozen(r.actors_at(depth), a), r.simulation_revision.value_or(29.));
            continue;
        }
        crate::corpses::advance_corpse_burning(a, elapsed);
        if frozen(r.actors_at(depth), a) {
            continue;
        }
        let age = a.corpse_age.value_or(0.);
        let at = a.y as usize * width + a.x as usize;
        if !crate::corpses::can_revive_corpse(a)
            || crate::revision51_stairs::blocks_revival(r,r.floors.iter().find(|f|f.depth==depth).unwrap(),a)
            || occupied_cell(r.floors.iter().find(|f|f.depth==depth).unwrap(),crate::portals::enabled(r),&occupied,at)
            || crate::environment::corpse_rooted(
                r.floors.iter().find(|f| f.depth == depth).unwrap(),
                r.actors_at(depth),
                a,
            )
        {
            a.corpse_age = (age + elapsed).into();
            continue;
        }
        let mut revived = None;
        if age >= 378. {
            revived = Some(1.);
        } else {
            let first = (81. - age).max(1.) as usize;
            let last_step = elapsed.min(378. - age) as usize;
            for step in first..=last_step {
                a.corpse_age = (age + step as f64).into();
                let yes = crate::corpses::roll_corpse_revival(a, || crate::rng::random(r));
                if yes {
                    revived = Some(step as f64);
                    break;
                }
            }
        }
        if let Some(step) = revived {
            crate::corpses::revive_corpse(a, last + step);
            if r.floors
                .iter()
                .find(|f| f.depth == depth)
                .unwrap()
                .ichor
                .as_ref()
                .is_some_and(|v| v[at] != 0.)
            {
                crate::ichor::mercurialize_actor(a);
            }
            a.conditions.shift_remove("underwater");
            a.conditions.shift_remove("submergedStealth");
            let remaining = (elapsed - step).max(0.);
            regen(
                r,
                r.floors.iter().find(|f| f.depth == depth).unwrap(),
                a,
                remaining,
                frozen(r.actors_at(depth), a),
            );
            let cap = crate::combat::effective_stats(r, a).max_energy;
            prepare_ally_rest(&proxy(r, depth, r.tick), a);
            rest(a, remaining, cap, frozen(r.actors_at(depth), a), r.simulation_revision.value_or(29.));
            occupied.insert(at);
        } else {
            a.corpse_age = (age + elapsed).into();
        }
    }
    if !actors.is_empty() {
        *r.actors_at_mut(depth) = actors;
    }
    Ok(())
}
pub fn catch_up_floor(r: &mut Run, depth: f64) -> Result<(), String> {
    catch_up(r, depth, None)
}
fn catch_up(
    r: &mut Run,
    depth: f64,
    mut deferred: Option<&mut Vec<Transfer>>,
) -> Result<(), String> {
    let _timing_function = crate::timing::span("offscreen.catch_up");
    let Some(index) = r.floors.iter().position(|f| f.depth == depth) else {
        return Ok(());
    };
    let f = &r.floors[index];
    if !f.dirty
        && f.last_tick == 0.
        && !f.seen.iter().any(|&v| v)
        && !has_clay(r,depth)
        && !crate::revision50_control_amulets::has_temporary_terrain(r,depth)
        && !f
            .godbeast_event
            .as_ref()
            .is_some_and(|e| e.activated.value_or(false))
    {
        if f.last_tick != r.tick {
            crate::mutation_journal::make_mut(&mut r.floors[index]).last_tick = r.tick;
        }
        return Ok(());
    }
    let mut elapsed = (r.tick - f.last_tick).max(0.);
    while elapsed > 0. {
        if !dynamic(r, &r.floors[index]) {
            approximate(r, depth, elapsed)?;
            let tick = r.tick;
            let f = crate::mutation_journal::make_mut(&mut r.floors[index]);
            f.last_tick = tick;
            f.dirty = false;
            break;
        }
        let tick = r.floors[index].last_tick + 1.;
        let mut head = proxy(r, depth, tick);
        if crate::creature_feeding::enabled(&head) { head.resolving_tick = true.into(); }
        let living_before: Vec<_> = head
            .actors_at(depth)
            .iter()
            .filter(|a| !a.dead.value_or(false))
            .map(|a| a.id)
            .collect();
        let expiring: Vec<_> = head
            .actors_at(depth)
            .iter()
            .filter(|a| c(a, "flying") > 0. && crate::travel::flying(&head, a))
            .map(|a| a.id)
            .collect();
        let mut pending = vec![];
        precise_actor_phase(&mut head, &mut pending)?;
        crate::monster_horn::world_turn(&mut head);
        crate::epidite::world_turn(&mut head).map_err(|e|e.to_string())?;
        crate::currents::tick_currents(&mut head)?;
        crate::environment::tick_environment_options_with_falls(
            &mut head,
            Some(r.player.active.iter().any(|id| id == "grisha-black-pit")),
            &expiring,
        )?;
        crate::creature_feeding::end_turn(&mut head);
        let width = head.floor().unwrap().width();
        let mut occupied: Vec<_> = head
            .ground
            .get(&depth.to_string())
            .into_iter()
            .flat_map(|g| g.iter())
            .map(|e| e.y as usize * width + e.x as usize)
            .collect();
        occupied.extend(
            head.actors_at(depth)
                .iter()
                .map(|a| a.y as usize * width + a.x as usize),
        );
        crate::environment::settle_doors(head.floor_mut().unwrap(), occupied);
        if has_clay(r,depth) {
            let live_stats=crate::combat::effective_stats(r,&r.player);
            // Keep the absent player's real identity reserved during births;
            // the dead/off-map proxy is still excluded from attack targets.
            let sentinel_id=head.player.id;head.player.id=r.player.id;
            let clay_result=crate::revision50_terrain::clay_end_turn_with_player_stats(&mut head,&live_stats);
            head.player.id=sentinel_id;
            clay_result.map_err(|e|e.to_string())?;
            // Clay attack reactions can edit the inactive dream terrain. Keep
            // the actual player's dream state/context while retaining floor edits.
            if let (Some(real),Some(projected))=(r.dreamwalking.as_mut(),head.dreamwalking.as_ref()) {
                real.floors=projected.floors.clone();
            }
        }
        if crate::revision50_control_amulets::has_temporary_terrain(&head,depth)
            && head.dreamwalking.as_ref().is_some_and(|s|s.floors.iter().any(|f|f.depth==depth)) {
            crate::dreamwalking::with_plane(&mut head,true,crate::revision50_control_amulets::expire_terrain);
        }
        if crate::revision50::enabled(r) {
            if let (Some(real),Some(projected))=(r.dreamwalking.as_mut(),head.dreamwalking.as_ref()) {real.floors=projected.floors.clone();}
        }
        r.rng = head.rng;
        r.next_id = head.next_id;
        r.metrics = head.metrics;
        r.run_stats = head.run_stats;
        r.collapse = head.collapse;
        // JS proxies share these whole collections: death/item callbacks can mutate a deeper floor.
        // Moving Arc collections preserves all untouched block identities without deep copying.
        r.actors.replace_from(head.actors);
        r.floors = head.floors;
        r.ground = head.ground;
        r.generated_ability_grants = head.generated_ability_grants;
        let surviving_ids: HashSet<u64> = r
            .actors
            .values()
            .flat_map(|v| v.iter())
            .filter(|a| !a.dead.value_or(false))
            .map(|a| a.id.to_bits())
            .collect();
        for id in living_before {
            if !surviving_ids.contains(&id.to_bits()) {
                crate::maintained_links::end_maintained_links_to_dead_target(r, id);
            }
        }
        for message in head.log {
            if message == "You hear an explosion!" {
                crate::engine::log(r, message, "environment");
            }
        }
        let expired: Vec<_> = expiring
            .into_iter()
            .filter(|id| {
                r.actors_at(depth)
                    .iter()
                    .find(|a| a.id == *id)
                    .is_some_and(|a| !crate::travel::flying(&proxy(r, depth, tick), a))
            })
            .collect();
        crate::travel::fall_actors_one_level(r, depth, &expired, None)?;
        let dirty = dynamic(r, &r.floors[index]);
        let f = crate::mutation_journal::make_mut(&mut r.floors[index]);
        f.last_tick = tick;
        f.dirty = dirty;
        elapsed -= 1.;
        for transfer in pending {
            if r.actors_at(transfer.from)
                .iter()
                .any(|a| a.id == transfer.id && !a.dead.value_or(false))
            {
                if let Some(deferred) = deferred.as_deref_mut() {
                    deferred.push(transfer)
                } else {
                    apply_transfer(r, transfer)?;
                }
            }
        }
        if deferred.is_none() {
            let f = crate::mutation_journal::make_mut(&mut r.floors[index]);
            if let Some(b) = f.bulwarks.as_mut() {
                // Scheduled expiry: warm no-due check O(1); a dirty map rebuilds
                // O(B log B), due work O(K log B + B), preserving retain order.
                b.expire(tick);
            }
        }
        let dirty = dynamic(r, &r.floors[index]);
        crate::mutation_journal::make_mut(&mut r.floors[index]).dirty = dirty;
    }
    Ok(())
}
pub fn tick_inactive_floors(r: &mut Run) -> Result<(), String> {
    let _timing_function = crate::timing::span("offscreen.total");
    validate_supported(r)?;
    let depths: Vec<_> = r
        .floors
        .iter()
        .filter(|f| involved(r, f))
        .map(|f| f.depth)
        .collect();
    let mut pending = vec![];
    for depth in depths {
        catch_up(r, depth, Some(&mut pending))?;
    }
    for transfer in pending {
        apply_transfer(r, transfer)?;
    }
    crate::travel::tick_ally_stair_followers(r)?;
    let tick = r.tick;
    for f in &mut r.floors {
        if f.depth == r.depth {
            continue;
        }
        // O(F) floor admission remains; unchanged deadline maps have O(1)
        // due checks. Only a due floor needs COW and actual ordered expiry.
        if f.bulwarks.as_ref().is_some_and(|v| v.has_due(tick)) {
            crate::mutation_journal::make_mut(f)
                .bulwarks
                .as_mut()
                .unwrap()
                .expire(tick);
        }
    }
    Ok(())
}
/// Ordinary hostile/neutral offscreen actors only. No wandering or proxy pursuit.
/// Source phase ordering and status/RNG order are retained; transfer traits gated.
fn precise_actor_phase(head: &mut Run, pending: &mut Vec<Transfer>) -> Result<(), String> {
    let _timing_function = crate::timing::span("offscreen.actor_phase");
    use crate::engine::{actor, actor_mut_trusted};
    let depth = head.depth;
    let tick = head.tick;
    let width = head.floor().unwrap().width();
    crate::creature_feeding::begin_turn(head);
    let ids = crate::ai::phase_order(head.actors_at(depth), false);
    crate::monster_chill::apply_adjacency(head);
    let mut occupied: HashSet<_> = head
        .actors_at(depth)
        .iter()
        .filter(|a| !a.dead.value_or(false))
        .map(|a| a.y as usize * width + a.x as usize)
        .collect();
    let mut cache: std::collections::HashMap<String, crate::movement::DistanceField> =
        std::collections::HashMap::new();
    for &id in &ids {
        let a = actor_mut_trusted(head, id).unwrap();
        if c(a, "monsterAudibleUntil") <= tick {
            for k in ["monsterAudibleUntil", "monsterAudibleX", "monsterAudibleY"] {
                a.conditions.shift_remove(k);
            }
        }
    }
    crate::monster_chill::apply_adjacency(head);
    'actors: for &id in &ids {
        let Some(a) = actor(head, id).cloned() else {
            continue;
        };
        if crate::dreamwalking::is_nightmare(&a) {continue;}
        if c(&a, "offscreenActionTick") == tick && a.conditions.contains_key("offscreenActionTick")
        {
            continue;
        }
        let at = a.y as usize * width + a.x as usize;
        if a.dead.value_or(false) {
            if frozen(head.actors_at(depth), &a) {
                continue;
            }
            let age = a.corpse_age.value_or(0.) + 1.;
            actor_mut_trusted(head, id).unwrap().corpse_age = age.into();
            let blocked = head
                .floor()
                .unwrap()
                .bulwarks
                .as_ref()
                .and_then(|v| v.get(&at.to_string()))
                .is_some_and(|&expiry| expiry >= tick);
            let chance = crate::corpses::corpse_roll_chance(actor(head, id).unwrap());
            if crate::corpses::can_revive_corpse(&a)
                && !crate::revision51_stairs::blocks_revival(head,head.floor().unwrap(),&a)
                && age > 80.
                && !blocked
                && !occupied_cell(head.floor().unwrap(),crate::portals::enabled(head),&occupied,at)
                && !crate::environment::corpse_rooted(
                    head.floor().unwrap(),
                    head.actors_at(depth),
                    &a,
                )
                && chance > 0.
            {
                let random = crate::rng::random(head);
                let a = actor_mut_trusted(head, id).unwrap();
                if crate::corpses::roll_corpse_revival(a, || random) {
                    crate::corpses::revive_corpse(a, tick);
                    crate::ichor::mercurialize_on_revival(head, id, depth);
                    let a = actor_mut_trusted(head, id).unwrap();
                    a.conditions.shift_remove("underwater");
                    a.conditions.shift_remove("submergedStealth");
                    occupied.insert(at);
                    crate::travel::enemy_moved(head, id)?;
                }
            }
            continue;
        }
        crate::ai::enemy_turn_effects(head, id).map_err(|e| e.to_string())?;
        let Some(a) = actor(head, id).cloned() else {
            continue;
        };
        if a.dead.value_or(false) {
            continue;
        }
        let was_paralyzed = c(&a, "paralyzed") > 0.;
        if was_paralyzed && crate::travel::has_ally_stair_follow(&a) {
            actor_mut_trusted(head, id)
                .unwrap()
                .conditions
                .insert("allyStairHeldTick".into(), tick);
        }
        let suppressed = frozen(head.actors_at(depth), &a);
        let mut changed = actor(head, id).unwrap().clone();
        regen(head, head.floor().unwrap(), &mut changed, 1., suppressed);
        *actor_mut_trusted(head, id).unwrap() = changed;
        let keys: Vec<_> = actor(head, id)
            .unwrap()
            .conditions
            .keys()
            .cloned()
            .collect();
        let mut stat_change = false;
        for key in keys {
            if key.starts_with("buffTicks:") || key.starts_with("remembered:") {
                let a = actor_mut_trusted(head, id).unwrap();
                let value = a.conditions.get_mut(&key).unwrap();
                *value -= 1.;
                if *value <= 0. {
                    if let Some(id) = key.strip_prefix("remembered:") {
                        a.abilities.retain(|v| v != id);
                        a.active.retain(|v| v != id);
                    } else {
                        a.conditions
                            .shift_remove(&key.replace("buffTicks:", "buff:"));
                    }
                    a.conditions.shift_remove(&key);
                    stat_change = true;
                }
            }
        }
        {
            let a = actor_mut_trusted(head, id).unwrap();
            for key in ["weakened", "emboldened"] {
                if c(a, key) > 0. {
                    *a.conditions.get_mut(key).unwrap() -= 1.;
                    stat_change = true;
                }
            }
        }
        if stat_change {
            crate::lifecycle::sync_enemy_health(head, id)?;
            crate::engine::death_if_nonpositive(head, id).map_err(|e| e.to_string())?;
        }
        if actor(head, id).is_none_or(|a| a.dead.value_or(false)) {
            continue;
        }
        let a = actor(head, id).unwrap().clone();
        if c(&a, "bleeding") > 0. {
            let source_id = a.conditions.get("bleedSourceId").copied();
            let source = source_id.and_then(|s| {
                if s == 0. {
                    Some(head.player.id)
                } else {
                    actor(head, s).map(|b| b.id)
                }
            });
            let (amount, exact_scale) = crate::divine_reactions::bleeding_packet(head.simulation_revision.value_or(29.0), &a);
            crate::engine::damage_environment(head, id, amount, source, "bleeding", exact_scale)
                .map_err(|e| e.to_string())?;
            if let Some(a) = actor_mut_trusted(head, id) {
                if let Some(v) = a.conditions.get_mut("bleeding") {
                    *v -= 1.
                }
            }
        }
        if actor(head, id).is_none_or(|a| a.dead.value_or(false)) {
            continue;
        }
        let a = actor_mut_trusted(head, id).unwrap();
        for key in [
            "paralyzed",
            "flying",
            "hasted",
            "temporaryAlly",
            "magicSensitive",
        ] {
            if c(a, key) > 0. {
                crate::engine::tick_condition(a, key);
                if key == "temporaryAlly" && c(a, key) == 0. {
                    a.state = "wandering".into();
                    a.perfect = false.into();
                    a.order = Field::Missing;
                }
            }
        }
        a.conditions.shift_remove("silenced");
        a.conditions.shift_remove("riptide");
        let active = actor(head, id).unwrap().active.clone();
        for ability in active {
            let cost = if crate::revision50::enabled(head) {
                let a = actor(head,id).unwrap();
                (crate::abilities::upkeep_cost_in(head,a,&ability)
                    - crate::abilities::prepaid_upkeep(head,a,&ability)).max(0.0)
            } else {crate::abilities::upkeep_cost(&ability)};
            let paid=if crate::revision50::enabled(head) {
                crate::revision50_ability_energy::spend(head,id,&ability,cost)
            } else {
                let a=actor_mut_trusted(head,id).unwrap();
                if a.energy>=cost {a.energy-=cost;true} else {false}
            };
            if !paid {
                let a=actor_mut_trusted(head,id).unwrap();
                a.active.retain(|v| v != &ability);
                if ability == "etill-phase-through" {
                    let at = a.y as usize * width + a.x as usize;
                    if head.floor().unwrap().tiles[at] == "wall" {
                        actor_mut_trusted(head, id).unwrap().hp = 0.;
                        crate::engine::resolve_death(head, id, None, None, true)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
        }
        if actor(head, id).is_none_or(|a| a.dead.value_or(false)) {
            continue;
        }
        if !was_paralyzed && crate::revision50_control_amulets::discord_turn(head,id,false).map_err(|e|e.to_string())? {
            occupied=head.actors_at(depth).iter().filter(|a|!a.dead.value_or(false)).map(|a|a.y as usize*width+a.x as usize).collect();
            continue;
        }
        let mut a = actor(head, id).unwrap().clone();
        prepare_ally_rest(head, &mut a);
        let cap = crate::combat::effective_stats(head, &a).max_energy;
        let suppressed = frozen(head.actors_at(depth), &a);
        let resting = rest(&mut a, 1., cap, suppressed, head.simulation_revision.value_or(29.));
        *actor_mut_trusted(head, id).unwrap() = a;
        if resting {
            continue;
        }
        if crate::ichor::squirm_if_stranded(head, id) {
            continue;
        }
        let a = actor(head, id).unwrap().clone();
        let may_move = !was_paralyzed && !((c(&a, "slowed") > 0.
            || a.abilities.iter().any(|s| s == "enemy:pale:slow" || s == "enemy:beast:slow")) && tick % 2. == 0.);
        if !was_paralyzed && crate::creature_feeding::take_turn(head, id, may_move, false).map_err(|e| e.to_string())? {
            let a = actor(head, id).unwrap();
            occupied.remove(&at);
            occupied.insert(a.y as usize * width + a.x as usize);
            continue;
        }
        if (a.state != "hunting" && c(&a, "monsterInventorFleeing") <= 0.)
            || was_paralyzed
            || ((c(&a, "slowed") > 0.
                || a.abilities
                    .iter()
                    .any(|id| id == "enemy:pale:slow" || id == "enemy:beast:slow"))
                && tick % 2. == 0.)
        {
            continue;
        }
        if c(&a, "rangedRecovery") > 0. {
            actor_mut_trusted(head, id)
                .unwrap()
                .conditions
                .shift_remove("rangedRecovery");
            continue;
        }
        if crate::engine::monster_ai_turn(head, id, false).map_err(|e| e.to_string())? {
            occupied = head
                .actors_at(depth)
                .iter()
                .filter(|a| !a.dead.value_or(false))
                .map(|a| a.y as usize * width + a.x as usize)
                .collect();
            continue;
        }
        let Some(a) = crate::engine::actor(head, id).cloned() else {
            continue;
        };
        let chasing = a
            .conditions
            .get("chaseDepth")
            .is_some_and(|&v| v.is_finite() && v.fract() == 0. && v != depth);
        let chase_destination = crate::ActorLastSeen {
            x: a.conditions.get("chaseExitX").copied().unwrap_or(f64::NAN),
            y: a.conditions.get("chaseExitY").copied().unwrap_or(f64::NAN),
        };
        let destination = if chasing {
            Some(&chase_destination)
        } else {
            a.last_seen.as_ref()
        };
        let Some(destination) = destination else {
            continue;
        };
        if destination.x.fract() != 0. || destination.y.fract() != 0. {
            continue;
        }
        let target = crate::movement::Point {
            x: destination.x as i32,
            y: destination.y as i32,
        };
        let at_exit = crate::revision51_stairs::at_chase_exit(head,&a,destination.x,destination.y);
        if !at_exit {
            let f = head.floor().unwrap();
            let flight = crate::environment::is_flying_in(head, &a);
            let phase = a.active.iter().any(|id| id == "etill-phase-through");
            let mut abilities = a.abilities.clone();
            abilities.sort();
            let key = format!(
                "{},{},{},{},{},{},{}",
                target.x,
                target.y,
                phase,
                flight,
                if head.simulation_revision.value_or(29.) >= 51. { !crate::revision51_stairs::restricted(head,&a) } else { a.state == "ally" || a.state == "neutral" },
                crate::corpses::is_bastidion(&a),
                abilities.join(";")
            );
            let harmful: Vec<_> = (0..f.tiles.len())
                .map(|i| crate::environment::creature_hazard_at(head, &a, i))
                .collect();
            let allowed: Vec<_> = (0..f.tiles.len())
                .map(|i| {
                    !crate::revision50_terrain::clay_blocks_run(head,&a,(i%width)as f64,(i/width)as f64)
                        && !crate::revision50_control_amulets::obstruction_blocks(head,&a,(i%width)as f64,(i/width)as f64)
                        && !crate::revision51_stairs::blocked_cell(head,&a,f,i)
                        && crate::ichor::habitat_path_allowed(f, &a, i)
                        && (phase || !f.grates.as_ref().is_some_and(|v| v.contains(&(i as f64))))
                        && !crate::engine::bulwark_blocks(
                            head,
                            &a,
                            (i % width) as f64,
                            (i / width) as f64,
                        )
                })
                .collect();
            let mut built = false;
            if !cache.contains_key(&key) {
                cache.insert(
                    key.clone(),
                    crate::movement::distance_field_ordered(
                        &f.tiles,
                        width,
                        target,
                        phase,
                        flight,
                        Some(&allowed),
                        Some(&harmful),
                        &if crate::portals::enabled(head) {crate::portals::links(f)} else {vec![]},
                        &if head.simulation_revision.value_or(29.)>=37. {crate::portals::ordered_pairs(f)} else {vec![]},
                    ),
                );
                built = true
            }
            let from = crate::movement::Point {
                x: a.x as i32,
                y: a.y as i32,
            };
            let mut step = crate::movement::next_step(&cache[&key], &f.tiles, from);
            if let Some(d) = step {
                let i = (a.y as i32 + d.y) as usize * width + (a.x as i32 + d.x) as usize;
                if !cache[&key].harmful.as_ref().is_some_and(|h| h[i]) && harmful[i] {
                    cache.insert(
                        key.clone(),
                        crate::movement::distance_field_ordered(
                            &f.tiles,
                            width,
                            target,
                            phase,
                            flight,
                            Some(&allowed),
                            Some(&harmful),
                            &if crate::portals::enabled(head) {crate::portals::links(f)} else {vec![]},
                            &if head.simulation_revision.value_or(29.)>=37. {crate::portals::ordered_pairs(f)} else {vec![]},
                        ),
                    );
                    step = crate::movement::next_step(&cache[&key], &f.tiles, from);
                    built = true
                }
            }
            if step.is_none() {step=crate::revision50_terrain::breach_step(head,&a,target);}
            if built {
                *head.metrics.entry("pathFields".into()).or_default() += 1.;
            }
            if let Some(d) = step {
                if crate::revision50_terrain::breakable_at(head,&a,a.x+d.x as f64,a.y+d.y as f64) {
                    // This is a normal hostile turn, not the movement-only haste
                    // opportunity. Reuse confusion/hemophilia and wall admission.
                    crate::engine::move_actor(head,id,d.x as f64,d.y as f64,false).map_err(|e|e.to_string())?;
                    occupied=head.actors_at(depth).iter().filter(|a|!a.dead.value_or(false))
                        .map(|a|a.y as usize*width+a.x as usize).collect();
                } else if crate::portals::enabled(head) {
                    crate::engine::move_actor(head,id,d.x as f64,d.y as f64,c(&a,"confused")<=0.)
                        .map_err(|e|e.to_string())?;
                    occupied=head.actors_at(depth).iter().filter(|a|!a.dead.value_or(false))
                        .map(|a|a.y as usize*width+a.x as usize).collect();
                } else if c(&a, "confused") > 0. {
                    occupied.remove(&at);
                    crate::engine::move_actor(head, id, d.x as f64, d.y as f64, false)
                        .map_err(|e| e.to_string())?;
                    let a = actor(head, id).unwrap();
                    occupied.insert(a.y as usize * width + a.x as usize);
                } else {
                    let x = a.x + d.x as f64;
                    let y = a.y + d.y as f64;
                    let to = y as usize * width + x as usize;
                    if !occupied.contains(&to)
                        && crate::ichor::movement_allowed(head, &a, x, y)
                        && crate::travel::can_traverse(head, &a, x, y)
                    {
                        if !crate::engine::movement_held(head, id) {
                            if crate::monster_statuses::try_hemophilia_episode(head, id)? {
                                continue 'actors;
                            }
                            occupied.remove(&at);
                            crate::environment::open_door(head.floor_mut().unwrap(), to);
                            let a = actor_mut_trusted(head, id).unwrap();
                            a.x = x;
                            a.y = y;
                            a.conditions.shift_remove("underwater");
                            a.conditions.shift_remove("submergedStealth");
                            occupied.insert(to);
                            crate::travel::enemy_moved(head, id)?;
                        }
                    }
                }
            }
        }
        let a = actor(head, id).unwrap();
        if (chasing && crate::revision51_stairs::at_chase_exit(head,a,target.x as f64,target.y as f64)
            && (!crate::revision51_stairs::blocked(head,a,target.x as f64,target.y as f64) || at_exit))
            || (!chasing && crate::revision51_stairs::at_chase_exit(head,a,target.x as f64,target.y as f64)) {
            if chasing {
                if crate::ichor::movement_allowed(head, a, a.x, a.y)
                    && (!at_exit || !crate::engine::movement_held(head, id))
                {
                    let a = actor(head, id).unwrap();
                    let fall =
                        head.floor().unwrap().tiles[a.y as usize * width + a.x as usize] == "chasm";
                    pending.push(Transfer {
                        id,
                        from: depth,
                        to: c(a, "chaseDepth"),
                        tick,
                        fall,
                    });
                }
            } else {
                let a = actor_mut_trusted(head, id).unwrap();
                a.state = "wandering".into();
                a.last_seen = Field::Missing;
                crate::travel::clear_floor_chase(a);
            }
        }
    }
    for id in ids {
        if actor(head, id)
            .is_some_and(|a| !a.dead.value_or(false) && c(a, "offscreenActionTick") != tick)
        {
            crate::revision50_control_amulets::tick_actor(head,id).map_err(|e|e.to_string())?;
            crate::monster_statuses::tick_poisoned(head, id)?;
            if let Some(a) = actor_mut_trusted(head, id) {
                if !a.dead.value_or(false) {
                    for key in [
                        "telepathic",
                        "invisible",
                        "confused",
                        "hemophilia",
                        "poisoned",
                        "slowed",
                    ] {
                        crate::engine::tick_condition(a, key);
                    }
                    crate::revision50_amulet_effects::tick_statuses(a);
                }
            }
        }
    }
    Ok(())
}

fn prepare_ally_rest(r: &Run, a: &mut Actor) {
    if a.state != "ally" {
        return;
    }
    if crate::travel::has_ally_stair_follow(a) || crate::ai::nearest_ally_enemy(r, a).is_some() {
        if crate::ai::sleeping(a) {
            crate::ai::wake(a, true);
        }
    } else if !crate::creature_feeding::hungry(r, a) {
        a.conditions.insert("monsterAllyRest".into(), 1.);
    }
}
#[derive(Clone, Copy)]
struct Transfer {
    id: f64,
    from: f64,
    to: f64,
    tick: f64,
    fall: bool,
}
fn apply_transfer(r: &mut Run, t: Transfer) -> Result<(), String> {
    let Some(target) = r.floors.iter().find(|f| f.depth == t.to) else {
        return Ok(());
    };
    if !r.floors.iter().any(|f| f.depth == t.from) {
        return Ok(());
    }
    let Some(a) = r.actors_at(t.from).iter().find(|a| a.id == t.id).cloned() else {
        return Ok(());
    };
    let source = proxy(r, t.from, t.tick);
    if a.dead.value_or(false)
        || a.state != "hunting"
        || !crate::revision51_stairs::at_chase_exit(&source,&a,c(&a,"chaseExitX"),c(&a,"chaseExitY"))
        || !crate::travel::passable(&source, &a, a.x, a.y)
        || !crate::ichor::movement_allowed(&source, &a, a.x, a.y)
    {
        return Ok(());
    }
    r.actors
        .entry(t.to.to_string())
        .or_insert_with(|| std::sync::Arc::new(vec![]));
    let w = target.width();
    let mut occupied: HashSet<_> = r
        .actors_at(t.to)
        .iter()
        .filter(|a| !a.dead.value_or(false))
        .map(|a| a.y as usize * w + a.x as usize)
        .collect();
    if t.to == r.depth {
        occupied.insert(r.player.y as usize * w + r.player.x as usize);
    }
    let recorded =
        a.conditions.contains_key("chaseLandingX") || a.conditions.contains_key("chaseLandingY");
    let origin = if recorded {
        (
            a.conditions
                .get("chaseLandingX")
                .copied()
                .unwrap_or(f64::NAN),
            a.conditions
                .get("chaseLandingY")
                .copied()
                .unwrap_or(f64::NAN),
        )
    } else if let Some(i) = target
        .tiles
        .iter()
        .position(|v| v == if t.to > t.from { "up" } else { "down" })
    {
        ((i % w) as f64, (i / w) as f64)
    } else {
        (5., 5.)
    };
    if !origin.0.is_finite()
        || !origin.1.is_finite()
        || origin.0.fract() != 0.
        || origin.1.fract() != 0.
        || origin.0 < 0.
        || origin.1 < 0.
        || origin.0 >= w as f64
        || origin.1 >= target.height() as f64
    {
        return Ok(());
    }
    let mut head = proxy(r, t.to, t.tick);
    let mut landing_actor = a.clone();
    landing_actor.x = origin.0;
    landing_actor.y = origin.1;
    let adjacent_only = crate::revision51_stairs::restricted(&head,&landing_actor)
        && crate::revision51_stairs::tile(&target.tiles[origin.1 as usize*w+origin.0 as usize]);
    if !adjacent_only && !crate::travel::can_traverse(&head, &landing_actor, origin.0, origin.1) {
        return Ok(());
    }
    let mut candidates = vec![];
    for (order, d) in std::iter::once(crate::movement::Point { x: 0, y: 0 })
        .chain(crate::movement::DIRECTIONS)
        .enumerate()
    {
        if adjacent_only && d.x == 0 && d.y == 0 { continue; }
        let x = origin.0 + d.x as f64;
        let y = origin.1 + d.y as f64;
        if !crate::travel::can_traverse(&head, &landing_actor, x, y) {
            continue;
        }
        let i = y as usize * w + x as usize;
        if target.tiles[i] == "pit" || occupied_cell(&target,crate::portals::enabled(r),&occupied,i) {
            continue;
        }
        candidates.push((
            crate::environment::creature_hazard_at(&head, &landing_actor, i),
            order,
            x,
            y,
            i,
        ));
    }
    candidates.sort_by_key(|v| (v.0, v.1));
    let Some((_, _, x, y, i)) = candidates.first().copied() else {
        return Ok(());
    };
    head.actors_at_mut(t.from).retain(|a| a.id != t.id);
    let mut a = a;
    a.x = x;
    a.y = y;
    head.actors_at_mut(t.to).push(a);
    crate::environment::open_door(head.floor_mut().unwrap(), i);
    crate::travel::enemy_moved(&mut head, t.id)?;
    let a = crate::engine::actor_mut_trusted(&mut head, t.id).unwrap();
    if recorded {
        a.last_seen = crate::ActorLastSeen {
            x: origin.0,
            y: origin.1,
        }
        .into();
    }
    crate::travel::clear_floor_chase(a);
    a.conditions.insert("offscreenActionTick".into(), t.tick);
    a.conditions.insert("seesPlayer".into(), 0.);
    a.state = "hunting".into();
    for f in &mut head.floors {
        if f.depth == t.from || f.depth == t.to {
            crate::mutation_journal::make_mut(f).dirty = true;
        }
    }
    if t.fall && head.floor().unwrap().tiles[i] != "water" {
        let hp = crate::engine::actor(&head, t.id).unwrap().max_hp;
        crate::engine::damage_environment(&mut head, t.id, hp * 0.25, None, "fall", false)
            .map_err(|e| e.to_string())?;
    }
    r.rng = head.rng;
    r.next_id = head.next_id;
    r.metrics = head.metrics;
    r.run_stats = head.run_stats;
    r.collapse = head.collapse;
    r.floors = head.floors;
    r.actors.replace_from(head.actors);
    r.ground = head.ground;
    for message in head.log {
        if message == "You hear an explosion!" {
            crate::engine::log(r, message, "environment");
        }
    }
    Ok(())
}

fn frozen(actors: &[Actor], a: &Actor) -> bool {
    actors.iter().any(|other| {
        other.id != a.id
            && !other.dead.value_or(false)
            && other.active.iter().any(|id| id == "dawel-reverse-entropy")
            && ((a.x - other.x).powi(2) + (a.y - other.y).powi(2)).sqrt() <= 10.
    })
}
fn regen(r: &Run, f: &Floor, a: &mut Actor, ticks: f64, suppressed: bool) {
    // Shared endpoints must not gain a second natural-regeneration tick.
    // Preserve historical offscreen arithmetic for legacy revisions.
    if suppressed || (crate::revision50::enabled(r) && crate::half_here::is_clone(a)) {
        return;
    }
    let drowned = a.abilities.iter().any(|id| id == "etill-drown")
        && matches!(
            f.tiles[a.y as usize * f.width() + a.x as usize].as_str(),
            "water" | "shallow-water"
        );
    a.hp = (a.hp
        + a.max_hp
            * crate::revision50_rings::natural_hp_rate(r, a)
            * ticks
            * crate::lifecycle::regen_multiplier(a)
            * if drowned { 3. } else { 1. })
    .min(a.max_hp);
}

// One shared logical portal cell reserves both physical endpoint representations.
fn occupied_cell(f:&Floor, portals:bool, occupied:&HashSet<usize>, at:usize)->bool {
    occupied.contains(&at) || portals && crate::portals::aliases(f,at).iter().any(|other|occupied.contains(other))
}

#[cfg(test)]
mod revision50_ring_tests {
    use super::*;
    fn room(revision: f64) -> Run {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = revision.into(); r.tick = 5.; r.actors = Default::default();
        r.inventory.clear(); r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
        let mut a = Actor { id: 900., x: 10., y: 10., hp: 50., max_hp: 100.,
            energy: 100., state: "neutral".into(),
            stats: crate::Stats { max_energy: 100., ..Default::default() }, ..Default::default() };
        a.copied_inventory = vec![crate::Item { id: 901., kind: "ring".into(),
            effect: "regeneration".to_string().into(), equipped: "ring1".to_string().into(),
            ..Default::default() }].into();
        let mut f = r.floor().unwrap().clone(); f.depth = 2.; f.last_tick = 0.; f.dirty = true;
        f.tiles.fill("floor".into());
        let index = r.floors.iter().position(|f| f.depth == 2.).unwrap();
        r.floors[index] = std::sync::Arc::new(f); r.actors_at_mut(2.).push(a); r
    }
    fn close(a: f64, b: f64) { assert!((a - b).abs() < 1e-9, "{a} != {b}"); }

    #[test]
    fn stationary_approximation_uses_elapsed_ticks_and_preserves_legacy_rate() {
        for (revision, hp) in [(49., 51.), (50., 52.)] {
            let mut r = room(revision); let player_hp = r.player.hp;
            // Directly cover the retained aggregate branch. Current living R50
            // actors use exact catch-up because creature-feeding is dynamic.
            approximate(&mut r, 2., 5.).unwrap();
            close(r.actors_at(2.)[0].hp, hp); close(r.player.hp, player_hp);
            close(r.actors_at(2.)[0].energy, 100.);
        }
    }
    #[test]
    fn aggregate_regen_keeps_suppression_water_creature_and_endpoint_rules() {
        let mut r = room(50.);
        let index = r.floors.iter().position(|f| f.depth == 2.).unwrap();
        let width = r.floors[index].width();
        crate::mutation_journal::make_mut(&mut r.floors[index]).tiles[10 * width + 10] = "shallow-water".into();
        let mut a = r.actors_at(2.)[0].clone();
        a.abilities = vec!["etill-drown".into(), "enemy:beast:regeneration".into()];
        regen(&r, &r.floors[index], &mut a, 2., false); close(a.hp, 74.);
        regen(&r, &r.floors[index], &mut a, 2., true); close(a.hp, 74.);
        a.conditions.insert("halfHereClone".into(), 1.);
        a.conditions.insert("halfHereOwner".into(), r.player.id);
        regen(&r, &r.floors[index], &mut a, 2., false); close(a.hp, 74.);
    }
}

#[cfg(test)]
mod clay_tests {
    use super::*;
    use crate::revision50_terrain::{Cell,Kind,Modification};
    fn run()->Run {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=50.0.into();r.player.stats.accuracy=100.;r.player.stats.offense=20.;
        r.player.abilities.clear();r.player.active.clear();r.inventory.clear();r.tick=1.;
        let mut f=r.floor().unwrap().clone();f.depth=2.;f.last_tick=0.;f.dirty=true;
        f.tiles.fill("floor".into());
        f.revision50_terrain=vec![Modification{kind:Kind::ClaySoldier,cells:vec![Cell{x:12,y:14}],previous_tiles:vec!["floor".into()],hp:None}].into();
        let w=f.width();f.tiles[14*w+12]="wall".into();
        let i=r.floors.iter().position(|f|f.depth==2.).unwrap();r.floors[i]=std::sync::Arc::new(f);
        let mut a=r.actors_at(1.)[0].clone();a.id=800.;a.x=13.;a.y=14.;a.hp=1000.;a.max_hp=1000.;
        a.abilities.clear();a.active.clear();a.conditions.clear();a.creature_kind=Field::Missing;
        a.state="neutral".into();a.stats.dr=0.;a.stats.dodge=0.;a.stats.max_hp=0.0.into();
        r.actors_at_mut(2.).push(a);r.next_id=900.;r
    }
    #[test]
    fn clay_forces_exact_inactive_tick_without_active_floor_duplicates() {
        let mut r=run();assert!(has_clay(&r,2.));assert!(dynamic(&r,&r.floors[1]));
        let player_hp=r.player.hp;
        catch_up_floor(&mut r,2.).unwrap();
        let hp=r.actors_at(2.).iter().find(|a|a.id==800.).unwrap().hp;
        assert!(hp<1000.);assert_eq!(r.player.hp,player_hp);
        assert!(!r.actors_at(2.).iter().any(crate::revision50_terrain::is_soldier));
        catch_up_floor(&mut r,2.).unwrap();
        assert_eq!(r.actors_at(2.).iter().find(|a|a.id==800.).unwrap().hp,hp);
    }
    #[test]
    fn offscreen_clay_receives_actual_player_stats_not_stripped_proxy() {
        let r=run();let stats=crate::combat::effective_stats(&r,&r.player);
        let mut head=proxy(&r,2.,1.);head.player.stats.offense=-100.;head.player.stats.accuracy=-100.;
        crate::revision50_terrain::clay_end_turn_with_player_stats(&mut head,&stats).unwrap();
        assert!(head.actors_at(2.).iter().find(|a|a.id==800.).unwrap().hp<1000.);
        assert!(head.actors_at(1.).iter().all(|a|!crate::revision50_terrain::is_soldier(a)));
    }
}

#[cfg(test)]
mod portal_traversal_tests {
    use super::*;
    fn room(blocked:bool)->(Run,f64) {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=35.0.into();r.tick=1.;
        r.player.x=-999.;r.player.y=-999.;
        let mut a=r.actors_at(1.)[0].clone();
        a.id=700.;a.x=10.;a.y=10.;a.name="Portal route test".into();a.creature_kind=Field::Missing;
        a.hp=100.;a.max_hp=100.;a.energy=100.;a.stats.max_hp=100.0.into();a.stats.max_energy=100.;
        a.abilities.clear();a.active.clear();a.conditions.clear();a.state="hunting".into();a.dead=false.into();a.monster_state=Field::Missing;
        a.last_seen=crate::ActorLastSeen{x:43.,y:10.}.into();
        a.conditions.insert("bestiaryTraitsInitialized".into(),1.);a.conditions.insert("paleTraitsInitialized".into(),1.);
        r.actors_at_mut(1.).clear();r.actors_at_mut(1.).push(a.clone());
        if blocked {a.id=701.;a.x=40.;a.state="wandering".into();a.last_seen=Field::Missing;r.actors_at_mut(1.).push(a);}
        let f=r.floor_mut().unwrap();let w=f.width();let h=f.height();
        for y in 1..h-1 {for x in 1..w-1 {f.tiles[y*w+x]="floor".into();}}
        f.environment=Field::Missing;f.roots=Field::Missing;f.fungal_roots=Field::Missing;f.ichor=Field::Missing;f.slime=Field::Missing;f.mud=Field::Missing;f.razorleaves=Field::Missing;f.vents=Field::Missing;f.permanent_fire=Field::Missing;f.currents=Field::Missing;
        f.portals=vec![crate::portals_generation::PortalPair{a:10*w+11,b:10*w+40,color_index:0}].into();
        (r,700.)
    }
    #[test]
    fn offscreen_pursuit_traverses_once_and_does_not_overlap_remote_occupant() {
        let (mut r,id)=room(false);precise_actor_phase(&mut r,&mut vec![]).unwrap();
        let a=crate::engine::actor(&r,id).unwrap();assert_eq!((a.x,a.y),(40.,10.));
        let (mut r,id)=room(true);precise_actor_phase(&mut r,&mut vec![]).unwrap();
        let a=crate::engine::actor(&r,id).unwrap();assert_ne!((a.x,a.y),(40.,10.));assert_ne!((a.x,a.y),(11.,10.));
    }
    #[test]
    fn credited_returning_ally_traverses_portal_once_with_remote_occupancy() {
        for blocked in [false,true] {
            let (mut r,id)=room(blocked);
            let a=crate::engine::actor_mut_trusted(&mut r,id).unwrap();a.state="ally".into();a.last_seen=Field::Missing;
            for (k,v) in [("allyStairSource",1.),("allyStairDepth",2.),("allyStairExitX",43.),("allyStairExitY",10.),("allyStairLandingX",10.),("allyStairLandingY",10.),("allyStairDistance",10.),("allyStairSpeed",100.),("allyStairRemaining",9.),("allyStairLastTick",1.)] {a.conditions.insert(k.into(),v);}
            crate::travel::restore_returning_ally_stair_followers(&mut r).unwrap();
            let a=crate::engine::actor(&r,id).unwrap();
            assert_eq!((a.x,a.y),if blocked {(10.,10.)} else {(40.,10.)});
        }
    }

    #[test]
    fn offscreen_stochastic_pursuit_replans_both_returns_without_telefrag() {
        for seed in [1u32,8192] {
            let (mut r,id)=room(false);r.simulation_revision=36.0.into();r.rng=seed as f64;
            let a=crate::engine::actor_mut_trusted(&mut r,id).unwrap();a.x=14.;a.last_seen=crate::ActorLastSeen{x:8.,y:10.}.into();
            let f=r.floor_mut().unwrap();let w=f.width();for t in &mut f.tiles{*t="wall".into()}for x in 8..=14{f.tiles[10*w+x]="floor".into()}f.portals=Field::Missing;
            f.non_euclidean=vec![crate::non_euclidean_generation::NonEuclideanPocket{cells:vec![10*w+11,10*w+12,10*w+13],groups:vec![crate::non_euclidean_generation::NonEuclideanGroup{sources:vec![10*w+11,10*w+12],destination:10*w+13}]}].into();
            precise_actor_phase(&mut r,&mut vec![]).unwrap();let a=crate::engine::actor(&r,id).unwrap();assert!([11.,12.].contains(&a.x),"actual offscreen arrival: {}",a.x);assert_eq!(a.y,10.);
        }
        let (mut r,id)=room(false);r.simulation_revision=36.0.into();
        let a=crate::engine::actor_mut_trusted(&mut r,id).unwrap();a.x=14.;a.last_seen=crate::ActorLastSeen{x:8.,y:10.}.into();
        let mut blocker=a.clone();blocker.id=701.;blocker.x=11.;blocker.state="sleeping".into();blocker.last_seen=Field::Missing;r.actors_at_mut(1.).push(blocker);
        let f=r.floor_mut().unwrap();let w=f.width();for t in &mut f.tiles{*t="wall".into()}for x in 8..=14{f.tiles[10*w+x]="floor".into()}f.portals=Field::Missing;
        f.non_euclidean=vec![crate::non_euclidean_generation::NonEuclideanPocket{cells:vec![10*w+11,10*w+12,10*w+13],groups:vec![crate::non_euclidean_generation::NonEuclideanGroup{sources:vec![10*w+11,10*w+12],destination:10*w+13}]}].into();
        precise_actor_phase(&mut r,&mut vec![]).unwrap();assert_eq!(crate::engine::actor(&r,id).unwrap().x,14.);assert_eq!(crate::engine::actor(&r,701.).unwrap().x,11.);
    }

}

#[cfg(test)]
#[path = "revision51_stair_arrival_tests.rs"]
mod revision51_stair_arrival_tests;
