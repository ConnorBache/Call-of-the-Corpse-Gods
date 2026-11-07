//! Ordered authored death prevention and Overflowing's resolved-damage counter.
//! No RNG or attack recursion. See docs/divine-rescue-coverage.md for ordering/cost.
use crate::{Run, engine, half_here};
pub const RAGE: &str = "shrail-rage-at-the-dying-of-the-light";
pub const OVERFLOWING: &str = "lichor-overflowing";
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
    for ability in abilities {
        if ability == RAGE && engine::actor(r, id).unwrap().active.contains(&ability) {
            let cost = engine::condition(engine::actor(r, id).unwrap(), "rageCost");
            if crate::revision50_ability_energy::spend(r, id, RAGE, cost) {
                let a = engine::actor_mut_trusted(r, id).unwrap();
                a.hp = 1.0;
                a.conditions.insert(
                    "rageCost".into(),
                    if cost == 0.0 { 1.0 } else { cost * 2.0 },
                );
                half_here::sync(r);
                return true;
            }
            engine::actor_mut_trusted(r, id)
                .unwrap()
                .active
                .retain(|s| s != &ability);
        }
        if ability == OVERFLOWING && engine::actor(r, id).unwrap().blood.value_or(0.0) > 0.0 {
            let a = engine::actor_mut_trusted(r, id).unwrap();
            a.blood = (a.blood.value_or(0.0) - 1.0).into();
            a.hp = 1.0;
            half_here::sync(r);
            return true;
        }
        if ability == PELIAN
            && engine::actor(r, id).unwrap().active.contains(&ability)
            && engine::condition(engine::actor(r, id).unwrap(), "pelianUsed") == 0.0
        {
            let a = engine::actor_mut_trusted(r, id).unwrap();
            a.hp = a.max_hp;
            a.conditions.insert("pelianUsed".into(), 1.0);
            a.active.retain(|s| s != &ability);
            half_here::sync(r);
            return true;
        }
    }
    false
}
/// Accrues the resolved packet, including overkill, after HP loss and before rescue.
/// Repeated subtraction preserves source floating-point behavior for legacy banks.
pub fn accrue_damage(r: &mut Run, victim: f64, n: f64) {
    if n <= 0.0 {
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
