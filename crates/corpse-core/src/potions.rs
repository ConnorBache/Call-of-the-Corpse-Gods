//! Potion application at an authored impact point. Body statuses remain local;
//! health, energy and capacity buffs resolve through the shared owner.
use crate::{
    Actor, Run, combat,
    engine::{EngineError, actor, actor_mut_trusted, log},
};
pub(crate) fn timed(a: &mut Actor, key: &str, duration: f64, fresh: bool) {
    let old = a.conditions.get(key).copied().unwrap_or(0.0);
    a.conditions
        .insert(key.into(), old.max(duration.ceil().max(0.0)));
    if fresh {
        a.conditions.insert(format!("{key}Fresh"), 1.0);
    } else {
        a.conditions.shift_remove(&format!("{key}Fresh"));
    }
}
pub fn supported(effect: &str) -> bool {
    crate::revision50_potions::new_effect(effect) || matches!(
        effect,
        "healing"
            | "energy"
            | "Veridian Sap"
            | "incineration"
            | "caustic gas"
            | "confusion"
            | "paralysis"
            | "offense"
            | "defense"
            | "accuracy"
            | "dodge"
            | "dr"
            | "perception"
            | "telepathy"
            | "levitation"
            | "speed"
            | "invisibility"
            | "detect magic"
            | "descent"
    )
}
pub fn apply(r: &mut Run, index: usize, x: f64, y: f64, thrown: bool) -> Result<bool, EngineError> {
    let item = r.inventory[index].clone();
    let effect = item.effect.value_or_default();
    if item.kind != "potion" || effect.is_empty() {
        return Ok(false);
    }
    if crate::revision50_potions::retired(r,&item) {
        return Err(EngineError(crate::revision50_potions::RETIRED_MESSAGE.into()));
    }
    if !supported(&effect) {
        return Err(EngineError(
            "This potion effect is not yet supported natively.".into(),
        ));
    }
    let target = if !thrown || crate::portals::same_position(r,&r.player,x,y) {
        Some(r.player.id)
    } else {
        r.actors_at(r.depth)
            .iter()
            .find(|a| !a.dead.value_or(false) && (!crate::portals::enabled(r)||crate::dreamwalking::same_plane(r,&r.player,a)) && crate::portals::same_position(r,a,x,y))
            .map(|a| a.id)
    };
    let potion_wave = crate::potion_animation::begin(r,&effect,x,y,target);
    let pool = target.map(|id| crate::half_here::owner_id(r, actor(r, id).unwrap()));
    let energy_before = pool.map(|id| actor(r, id).unwrap().energy).unwrap_or(0.0);
    let hp_before = pool.map(|id| actor(r,id).unwrap().hp).unwrap_or(0.0);
    let strength = if item.diluted.value_or(false) {
        0.5
    } else {
        1.0
    };
    let fresh = thrown || !r.player.abilities.iter().any(|a|a=="isheilah-tools-of-the-trade");
    if !crate::revision50_potions::apply(r,&effect,target,x,y,strength,fresh)? {
    match effect.as_str() {
        "Veridian Sap" => {
            if let Some(id) = pool {
                let cap = combat::effective_stats(r, actor(r, id).unwrap()).max_energy;
                let a = actor_mut_trusted(r, id).unwrap();
                a.hp += (a.max_hp - a.hp) * strength;
                a.energy += (cap - a.energy).max(0.0) * strength;
                if target == Some(r.player.id) {
                    crate::progression::award(
                        r,
                        ((crate::progression::level_up_cost(r) - r.exp).max(0.0) * strength).ceil(),
                    );
                }
            }
        }
        "healing" => {
            if let Some(id) = pool {
                let a = actor_mut_trusted(r, id).unwrap();
                a.hp = (a.hp + 50.0 * strength).min(a.max_hp);
            }
        }
        "energy" => {
            if let Some(id) = pool {
                actor_mut_trusted(r, id).unwrap().energy += 50.0 * strength;
            }
        }
        "detect magic" | "telepathy" | "levitation" | "speed" | "invisibility" => {
            if let Some(id) = target {
                let (key, turns) = match effect.as_str() {
                    "detect magic" => ("magicSensitive", 20.0),
                    "telepathy" => ("telepathic", 30.0),
                    "levitation" => ("flying", 20.0),
                    "speed" => ("hasted", 10.0),
                    _ => ("invisible", 20.0),
                };
                let fresh = thrown
                    || !r
                        .player
                        .abilities
                        .iter()
                        .any(|a| a == "isheilah-tools-of-the-trade");
                timed(actor_mut_trusted(r, id).unwrap(), key, turns * strength, fresh);
                if effect == "detect magic" && pool == Some(r.player.id) {
                    crate::inventory::remember_detected_items(r).map_err(EngineError)?;
                }
            }
        }
        "incineration" if crate::potion_rules::current(r) => {
            crate::revision50_potions::incinerate(r,x,y,strength)?;
        }
        "incineration" => {
            crate::environment::ignite(r, x, y, Some((8.0 * strength).ceil()), Some(r.player.id))
                .map_err(EngineError)?;
            if let Some(id) = target {
                let random = crate::rng::random(r);
                let revision = r.simulation_revision.value_or(29.);
                crate::corpses::apply_burning_revision(
                    actor_mut_trusted(r, id).unwrap(),
                    revision,
                    random,
                    Some(-2.0),
                    Some(1.0),
                    Some(strength),
                );
            }
        }
        "caustic gas" | "confusion" | "paralysis" => {
            let at = y as usize * r.floor().unwrap().width() + x as usize;
            crate::environment::add_gas(
                r.floor_mut().unwrap(),
                at,
                if effect == "caustic gas" {
                    "caustic"
                } else {
                    &effect
                },
                100.0 * strength,
                -2.0,
            )
            .map_err(EngineError)?;
            if effect == "paralysis" {
                if let Some(id) = target {
                    if !crate::enemy_traits::has_enemy_ability(
                        actor(r, id).unwrap(),
                        "enemy:beast:gas-immunity",
                    ) {
                        timed(
                            actor_mut_trusted(r, id).unwrap(),
                            "paralyzed",
                            5.0 * strength,
                            false,
                        );
                    }
                }
            }
        }
        "offense" | "defense" | "accuracy" | "dodge" | "dr" | "perception" => {
            if let Some(id) = target {
                let id = if effect == "defense" {
                    pool.unwrap()
                } else {
                    id
                };
                let a = actor_mut_trusted(r, id).unwrap();
                let key = format!("buff:{effect}");
                let timer = format!("buffTicks:{effect}");
                let old = if a.conditions.get(&timer).copied().unwrap_or(0.0) > 0.0 {
                    combat::integer_stat(a.conditions.get(&key).copied().unwrap_or(0.0))
                } else {
                    0.0
                };
                let bonus =
                    combat::integer_stat((if effect == "dr" { 2.0 } else { 5.0 }) * strength);
                let duration = a
                    .conditions
                    .get(&timer)
                    .copied()
                    .unwrap_or(0.0)
                    .max((20.0 * strength).ceil());
                a.conditions.insert(key, old.max(bonus));
                a.conditions.insert(timer, duration);
            }
        }
        _ => {}
    }
    }
    if let Some(id) = pool {
        if !(id == r.player.id
            && r.player
                .abilities
                .iter()
                .any(|a| a == "grisha-boundless-hunger"))
        {
            let a = actor(r, id).unwrap().clone();
            let energy = a.energy.min(combat::effective_stats(r, &a).max_energy);
            crate::resources::record_loss(r, &a, "energy", (energy_before - energy).max(0.0));
            actor_mut_trusted(r, id).unwrap().energy = energy;
        }
        crate::equipment::sync_actor_health(r, id)?;
    }
    crate::half_here::sync(r);
    let display_effect = if effect == "Veridian Sap" && r.simulation_revision.value_or(29.) >= 33. { "Verdania Sap" } else { &effect };
    let discovery_enabled = !crate::revision50::enabled(r);
    let discovered = discovery_enabled && !r.known_potions.contains(&effect);
    if discovered {
        r.known_potions.push(effect.clone());
    }
    let item_label = if crate::revision50::enabled(r) {
        crate::revision50_item_knowledge::display_name(r, &item)
    } else { format!("{} of {display_effect}", if item.diluted.value_or(false) { "Diluted potion" } else { "Potion" }) };
    log(
        r,
        format!(
            "You {} {item_label}.",
            if thrown { "throw" } else { "drink" }
        ),
        "player",
    );
    if discovered {
        log(r, format!("Identified: Potion of {display_effect}."), "player");
    }
    let healing_item=matches!(effect.as_str(),"healing"|"energy"|"Veridian Sap");
    let hp_increased=pool.and_then(|id|actor(r,id)).is_some_and(|a|a.hp>hp_before);
    let energy_increased=pool.and_then(|id|actor(r,id)).is_some_and(|a|a.energy>energy_before);
    if potion_wave.is_none() && (!crate::events::enabled(r) || thrown || !healing_item || hp_increased) || (potion_wave.is_some() && thrown) {
        crate::events::actor_event(
            r,&r.player,
            if thrown {"projectile"} else if healing_item {"heal"} else {"pulse"},
            &if crate::revision50::enabled(r) { item_label } else { format!("Potion of {display_effect}") },
            Some(crate::events::EventPoint{x,y}),if thrown {Some(&item.glyph)} else {None},
        );
    }
    if energy_increased && pool==Some(r.player.id) {
        crate::events::inventory(r,"shared","energy-rise",None);
    }
    for i in &mut r.inventory {
        if discovery_enabled && i.kind == "potion" && i.effect.as_ref() == Some(&effect) {
            i.identified = true.into();
            i.name = format!("Potion of {display_effect}");
        }
    }
    if !thrown
        && r.player
            .abilities
            .iter()
            .any(|a| a == crate::abilities::DILUTE || (crate::revision50::enabled(r) && a == "kative-bottomless-pack"))
        && !item.diluted.value_or(false)
    {
        r.inventory[index].diluted = true.into();
        r.inventory[index].name = format!("Diluted potion of {display_effect}");
        crate::events::inventory(r,"P66","split-potion",Some(item.id));
    } else {
        r.inventory.retain(|i| i.id != item.id);
        if item.diluted.value_or(false) { crate::events::inventory(r,"P66","fade-potion",Some(item.id)); }
    }
    if effect == "descent" {
        crate::travel::collapse_descent(r, x, y, strength < 1.0).map_err(EngineError)?;
    }
    crate::progression::level_up(r);
    if let Some(action)=potion_wave {crate::events::checkpoint_for_action(r,action);}
    Ok(true)
}
