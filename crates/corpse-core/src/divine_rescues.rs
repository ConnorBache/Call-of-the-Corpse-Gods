//! Ordered authored death prevention and Overflowing's resolved-damage counter.
//! No RNG or attack recursion. See docs/divine-rescue-coverage.md for ordering/cost.
use crate::{Run, engine, half_here};
pub const RAGE: &str = "shrail-rage-at-the-dying-of-the-light";
pub const OVERFLOWING: &str = "lichor-overflowing";
pub const MAX_SAVES: f64 = 5.0;
const SAVES: &str = "overflowingSaves";

/// Default stock on acquisition, including when a temporary ability is granted.
pub fn saves(actor: &crate::Actor) -> f64 {
    actor.conditions.get(SAVES).copied().unwrap_or(MAX_SAVES)
}

fn refill(actor: &mut crate::Actor) {
    // Missing is the canonical full value; never write on an untouched actor.
    actor.conditions.shift_remove(SAVES);
}

pub fn refill_on_level_up(r: &mut Run) {
    if crate::balance_rules::current(r) && r.player.abilities.iter().any(|s| s == OVERFLOWING) {
        refill(&mut r.player);
    }
}

/// Full HP may be reached before an enemy retaliates in the same command.
/// The >1 guard prevents the 1 HP rescue itself refilling a 1-max-HP actor.
pub fn refresh_if_full(r: &mut Run, id: f64) {
    if !crate::balance_rules::current(r) { return; }
    if engine::actor(r, id).is_some_and(can_refill) {
        refill(engine::actor_mut_trusted(r, id).unwrap());
    }
}

fn can_refill(a: &crate::Actor) -> bool {
    a.hp >= a.max_hp && a.hp > 1.0 && a.conditions.contains_key(SAVES)
        && a.abilities.iter().any(|s| s == OVERFLOWING)
}

/// O(K) only at health settlement, with a fixed-key O(1) expected fast path.
pub fn refresh_player_if_full(r: &mut Run) {
    if crate::balance_rules::current(r) && can_refill(&r.player) {
        refill(&mut r.player);
    }
}

pub const PELIAN: &str = "pelian-you-shall-never-be-rid-of-me";
/// Called before terminal cleanup. Ability collection order is acquisition order.
pub fn try_rescue(r: &mut Run, id: f64, bypass: bool) -> bool {
    let Some(body) = engine::actor(r, id) else {
        return false;
    };
    let id = half_here::owner_id(r, body);
    let Some(a) = engine::actor(r, id) else {
        return false;
    };
    if a.hp > 0.0 || a.dead.value_or(false) {
        return false;
    }
    let abilities = a.abilities.clone();
    engine::actor_mut_trusted(r, id)
        .unwrap()
        .conditions
        .shift_remove("deathResolutionBlocked");
    if bypass {
        return false;
    }
    let passive_ragelight = crate::ragelight_rules::current(r);
    let balanced = crate::balance_rules::current(r);
    for ability in abilities {
        if ability == RAGE && (passive_ragelight || engine::actor(r, id).unwrap().active.contains(&ability)) {
            let cost = engine::condition(engine::actor(r, id).unwrap(), "rageCost");
            if crate::revision50_ability_energy::spend(r, id, RAGE, cost) {
                let a = engine::actor_mut_trusted(r, id).unwrap();
                a.hp = 1.0;
                a.conditions.insert(
                    "rageCost".into(),
                    if cost == 0.0 { 1.0 } else { cost * 2.0 },
                );
                half_here::sync(r);
                crate::animation_shapes::nova(r, engine::actor(r,id).unwrap(), "P01", "nova-red-small", 2);
                return true;
            }
            if !passive_ragelight {
                engine::actor_mut_trusted(r, id)
                    .unwrap()
                    .active
                    .retain(|s| s != &ability);
            }
        }
        if ability == OVERFLOWING {
            let a = engine::actor(r, id).unwrap();
            let stock = if balanced { saves(a) } else { a.blood.value_or(0.0) };
            if stock <= 0.0 { continue; }
            let a = engine::actor_mut_trusted(r, id).unwrap();
            if balanced {
                a.conditions.insert(SAVES.into(), stock - 1.0);
            } else {
                a.blood = (stock - 1.0).into();
            }
            a.hp = 1.0;
            crate::blood_mist::emit(r, id);
            half_here::sync(r);
            crate::animation_shapes::short_nova(r,engine::actor(r,id).unwrap(),"P37",crate::animation_shapes::LICHOR);
            return true;
        }
        if ability == PELIAN
            && engine::actor(r, id).unwrap().active.contains(&ability)
            && engine::condition(engine::actor(r, id).unwrap(), "pelianUsed") == 0.0
        {
            let a = engine::actor_mut_trusted(r, id).unwrap();
            a.hp = a.max_hp;
            if balanced && a.abilities.iter().any(|s| s == OVERFLOWING) { refill(a); }
            a.conditions.insert("pelianUsed".into(), 1.0);
            a.active.retain(|s| s != &ability);
            half_here::sync(r);
            crate::animation_shapes::nova(r, engine::actor(r,id).unwrap(), "P07", "nova-blue-small", 2);
            return true;
        }
    }
    false
}
/// Accrues the resolved packet, including overkill, after HP loss and before rescue.
/// Repeated subtraction preserves source floating-point behavior for legacy banks.
pub fn accrue_damage(r: &mut Run, victim: f64, n: f64) {
    if n <= 0.0 || crate::balance_rules::current(r) {
        return;
    }
    let Some(body) = engine::actor(r, victim) else {
        return;
    };
    if !body.abilities.iter().any(|s| s == OVERFLOWING) {
        return;
    }
    let owner = half_here::owner_id(r, body);
    let Some(a) = engine::actor_mut_trusted(r, owner) else {
        return;
    };
    let mut bank = a.damage_bank.value_or(0.0) + n;
    while bank >= 50.0 {
        bank -= 50.0;
        a.blood = (a.blood.value_or(0.0) + 1.0).min(55.0).into();
    }
    a.damage_bank = bank.into();
}
