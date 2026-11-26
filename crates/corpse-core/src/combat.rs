//! Numeric combat primitives with explicit legacy and current rounding policies.
/// v48 integerStat rounds signed contributions halfway away from zero.
pub fn integer_stat(n: f64) -> f64 {
    n.signum() * n.abs().round() + 0.0
}
/// Frozen pre-R52 probability for legacy callers and replay compatibility.
pub fn hit_chance(accuracy: f64, dodge: f64, range: f64) -> f64 {
    ((0.65 * 0.9f64.powf(dodge - accuracy)).clamp(0.0, 1.0) * 0.9f64.powf(range)).clamp(0.0, 1.0)
}
/// R52 ordinary chance: Accuracy / (2 * Dodge), then the existing range loss.
/// Dodge is clamped to a minimum of one only within this calculation.
/// Nonpositive Accuracy cannot hit. Stored/effective stats are not clamped.
/// Divide before multiplying by 0.5 so large finite Dodge cannot overflow.
/// O(1) time and memory; no RNG or actor/stat mutation.
pub fn ratio_hit_chance(accuracy: f64, dodge: f64, range: f64) -> f64 {
    if !accuracy.is_finite() || !dodge.is_finite() || !range.is_finite() || range < 0.0 || accuracy <= 0.0 {
        return 0.0;
    }
    let base = (accuracy / dodge.max(1.0) * 0.5).clamp(0.0, 1.0);
    (base * 0.9f64.powf(range)).clamp(0.0, 1.0)
}
/// Actual non-player birth Accuracy uses the designer's provisional R52
/// fifteen-point baseline plus its authored species/floor offset. O(1).
/// Like birth Dodge, this is never applied to an existing or copied body.
pub fn creature_birth_accuracy(revision: f64, offset: f64) -> f64 {
    offset + if revision >= 52.0 { 15.0 } else { 0.0 }
}
/// Actual non-player birth Dodge: retain the authored offset above R52's
/// ten-point baseline, with a minimum starting value of ten. This is not an
/// effective-stat floor: later penalties and recorded/copy/revival stats stay exact.
/// Call only when assigning a raw NEW body stat. O(1), no mutation or RNG.
pub fn creature_birth_dodge(revision: f64, offset: f64) -> f64 {
    if revision >= 52.0 { (offset + 10.0).max(10.0) } else { offset }
}
/// One authority for actual attacks and detached melee/projectile previews.
pub fn hit_chance_for_run(r: &crate::Run, accuracy: f64, dodge: f64, range: f64) -> f64 {
    if r.simulation_revision.value_or(29.0) >= 52.0 {
        ratio_hit_chance(accuracy, dodge, range)
    } else {
        hit_chance(accuracy, dodge, range)
    }
}
/// Current R52 amendment: attack values are authored in HP units before DR.
/// Other revisions retain their original packet scaling and replay arithmetic.
pub fn raw_attack_damage_enabled(revision: f64) -> bool { revision == 52.0 }
/// O(1), actual non-player attack profile. Explicit copy/weapon fields already
/// contain raw HP coefficients and must not be scaled a second time.
pub fn creature_attack_base(revision: f64, actor: &crate::Actor, offense: f64) -> f64 {
    let (base, coefficient) = if raw_attack_damage_enabled(revision) { (5.0, 0.5) } else { (10.0, 1.0) };
    actor.conditions.get("weaponDamage").copied().unwrap_or(base)
        + offense * actor.conditions.get("weaponOffenseMultiplier").copied().unwrap_or(coefficient)
}
fn damage_scale_for_run(r: &crate::Run, kind: Option<&str>) -> f64 {
    if raw_attack_damage_enabled(r.simulation_revision.value_or(29.0)) && kind == Some("attack") { 1.0 }
    else { damage_scale(kind) }
}
fn damage_scale(kind: Option<&str>) -> f64 {
    match kind {
        Some("fire" | "caustic" | "ichor" | "breatherGas" | crate::death_gas_rules::GAS | "fall") => 0.25,
        Some("attack" | "bleeding") => 0.5,
        _ => 1.0,
    }
}
/// Frozen pre-51 packet rounding, retained for replay and public legacy callers.
pub fn tuned_damage(amount: f64, kind: Option<&str>) -> f64 {
    (amount * damage_scale(kind)).ceil().max(0.0)
}
/// Damage is nonnegative; exact halves round upward. Healing/stat rounding is separate.
pub fn nearest_damage(amount: f64) -> f64 {
    amount.max(0.0).round()
}
pub fn nearest_damage_enabled(r: &crate::Run) -> bool {
    r.simulation_revision.value_or(29.0) >= 51.0
}
pub fn tuned_damage_for_run(r: &crate::Run, amount: f64, kind: Option<&str>) -> f64 {
    if nearest_damage_enabled(r) { nearest_damage(amount * damage_scale_for_run(r, kind)) }
    else { tuned_damage(amount, kind) }
}
pub fn explosion_damage_for_run(r: &crate::Run, max_hp: f64) -> f64 {
    if nearest_damage_enabled(r) { nearest_damage(max_hp * 0.5) }
    else { explosion_damage(max_hp) }
}
pub fn ordinary_strike_damage_for_run(
    r: &crate::Run, base: f64, variance: f64, dr: f64, surprise: bool, paralyzed: bool,
) -> f64 {
    tuned_damage_for_run(r,
        base * if surprise || paralyzed { 3.0 } else { 1.0 } * variance - dr,
        Some("attack"))
}
/// O(I+C) resistance lookup, O(1) scratch. Round once, after all packet scaling.
/// Does not decide source/plane admission, immunity, absorption or lifecycle effects.
pub fn damage_packet(r: &crate::Run, victim: &crate::Actor, amount: f64, kind: &str, exact_scale: bool) -> f64 {
    let scaled = if nearest_damage_enabled(r) {
        (if kind == "explosion" { victim.max_hp * 0.5 }
        else { amount * damage_scale_for_run(r, if exact_scale { None } else { Some(kind) }) }).max(0.0)
    } else if kind == "explosion" { explosion_damage(victim.max_hp) }
    else { tuned_damage(amount, if exact_scale { None } else { Some(kind) }) };
    let resisted = crate::revision50_rings::resist_damage(r, victim, kind, scaled);
    if nearest_damage_enabled(r) { nearest_damage(resisted) } else { resisted }
}
pub fn explosion_damage(max_hp: f64) -> f64 {
    (max_hp * 0.5).ceil().max(0.0)
}
pub fn ordinary_strike_damage(
    base: f64,
    variance: f64,
    dr: f64,
    surprise: bool,
    paralyzed: bool,
) -> f64 {
    tuned_damage(
        base * if surprise || paralyzed { 3.0 } else { 1.0 } * variance - dr,
        Some("attack"),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mitigation_before_scale_rounding() {
        assert_eq!(ordinary_strike_damage(30.0, 0.75, 10.0, false, false), 7.0);
        assert_eq!(ordinary_strike_damage(30.0, 1.0, 100.0, false, false), 0.0);
    }
    #[test]
    fn negative_rounding() {
        assert_eq!(integer_stat(-1.5), -2.0);
    }
    #[test]
    fn chance_clamps_before_range() {
        assert_eq!(hit_chance(100.0, 0.0, 1.0), 0.9);
    }
    #[test]
    fn packets_round_up() {
        assert_eq!(tuned_damage(1.0, Some("attack")), 1.0);
        assert_eq!(tuned_damage(4.1, Some("fire")), 2.0);
    }
}
use crate::model::*;
pub fn effective_stats(r: &Run, a: &Actor) -> Stats {
    let _timing_function = crate::timing::span("stats.effective");
    let owner = crate::engine::actor(r, crate::half_here::owner_id(r, a)).unwrap_or(a);
    let sustain_factor = crate::revision50_rings::stack_multiplier(r, a, crate::revision50_rings::SUSTAIN_STRENGTH, 0.5);
    let sustain = sustain_factor < 1.0;
    // Scale each temporary negative contribution after its existing integer
    // normalization. Do not halve base/equipment penalties or round halves away.
    let temporary = |n: f64| if sustain && n < 0.0 { n * sustain_factor } else { n };
    let mut s = owner.stats.clone();
    // R50 Mini-Me stores already-derived numeric stats, including fractional
    // Sustain relief. Normalize ordinary/legacy base stats, not that snapshot.
    let stats_snapshot = crate::revision50::enabled(r) && a.id != r.player.id
        && owner.id == a.id && crate::engine::condition(a, "miniMe") == 1.0;
    if !stats_snapshot {
        s.offense = integer_stat(s.offense);
        s.defense = integer_stat(s.defense);
        s.accuracy = integer_stat(s.accuracy);
        s.dodge = integer_stat(s.dodge);
        s.dr = integer_stat(s.dr);
        s.perception = integer_stat(s.perception);
        s.max_energy = integer_stat(s.max_energy);
    }
    s.max_hp = Field::Value(if stats_snapshot { s.max_hp.value_or(0.0) }
        else { integer_stat(s.max_hp.value_or(0.0)) });
    for (k, v) in &a.conditions {
        if let Some(k) = k.strip_prefix("buff:") {
            if !matches!(k, "defense" | "maxHp" | "maxEnergy") {
                add_stat(&mut s, k, integer_stat(*v));
            }
        }
    }
    for k in ["defense", "maxHp", "maxEnergy"] {
        add_stat(
            &mut s,
            k,
            integer_stat(owner.conditions.get(&format!("buff:{k}")).copied().unwrap_or(0.0)),
        );
    }
    {
        for i in crate::actor_equipment::inventory(r, owner) {
            if if crate::revision50::enabled(r) { !crate::revision50_inventory::worn(i) } else { i.equipped.as_ref().is_none() } {
                continue;
            }
            if let Some(b) = i.bonus.as_ref() {
                for (k, v, base) in [
                    (
                        "offense",
                        &b.offense,
                        i.base_bonus.as_ref().map(|b| &b.offense),
                    ),
                    (
                        "defense",
                        &b.defense,
                        i.base_bonus.as_ref().map(|b| &b.defense),
                    ),
                    (
                        "accuracy",
                        &b.accuracy,
                        i.base_bonus.as_ref().map(|b| &b.accuracy),
                    ),
                    ("dodge", &b.dodge, i.base_bonus.as_ref().map(|b| &b.dodge)),
                    ("dr", &b.dr, i.base_bonus.as_ref().map(|b| &b.dr)),
                    (
                        "perception",
                        &b.perception,
                        i.base_bonus.as_ref().map(|b| &b.perception),
                    ),
                    (
                        "maxEnergy",
                        &b.max_energy,
                        i.base_bonus.as_ref().map(|b| &b.max_energy),
                    ),
                    ("maxHp", &b.max_hp, i.base_bonus.as_ref().map(|b| &b.max_hp)),
                ] {
                    if let Some(v) = v.as_ref() {
                        add_stat(
                            &mut s,
                            k,
                            if let Some(base) = base.and_then(|b| b.as_ref()) {
                                integer_stat(*base) + integer_stat(v - base)
                            } else {
                                integer_stat(*v)
                            },
                        );
                    }
                }
            }
            for e in &i.enhancements {
                crate::equipment::add_bonus(&mut s, &e.bonus);
            }
        }
    }
    if sustain {
        // Restore half the timed negative buff after equipment: legacy item
        // enhancement arithmetic rounds its accumulated stat, so applying the
        // fractional relief earlier would incorrectly round it away.
        for key in ["offense", "defense", "accuracy", "dodge", "dr", "perception", "maxHp", "maxEnergy"] {
            let source = if matches!(key, "defense" | "maxHp" | "maxEnergy") { owner } else { a };
            let n = integer_stat(crate::engine::condition(source, &format!("buff:{key}")));
            if n < 0.0 && crate::engine::condition(source, &format!("buffTicks:{key}")) > 0.0 {
                add_stat(&mut s, key, -n * (1.0 - sustain_factor));
            }
        }
    }
    // Each carried copy contributes before normal combat modifiers.
    s.dr += 4.0 * crate::revision50_rings::stacking_count(r, a, crate::revision50_rings::PROTECTION) as f64;
    s.offense += 4.0 * crate::revision50_rings::stacking_count(r, a, crate::revision50_rings::SLAYING) as f64;
    s.dr += temporary(-crate::revision50_amulet_effects::corrosion_penalty(r,a));
    if a.conditions.get("weakened").copied().unwrap_or(0.0) > 0.0 {
        s.offense += temporary(-5.0);
    }
    if a.conditions.get("emboldened").copied().unwrap_or(0.0) > 0.0 {
        s.offense += 5.0;
    }
    if crate::water_blood::enabled(r) && crate::engine::condition(a, "disoriented") > 0.0 {
        s.accuracy += temporary(-integer_stat(crate::engine::condition(a, "disorientedPenalty")));
    }
    if crate::engine::condition(a, "screamWeakened") > 0.0 {
        s.offense += temporary(-integer_stat(crate::engine::condition(a, "screamPenalty")));
    }
    if crate::engine::condition(a, "screamEmboldened") > 0.0 {
        s.offense += temporary(integer_stat(crate::engine::condition(a, "screamBonus")));
        s.perception += temporary(integer_stat(crate::engine::condition(a, "screamPerception")));
    }
    if crate::divine_combat::uses_enemy_health(r, owner) {
        s.defense += integer_stat(s.max_hp.value_or(0.0) / 6.0);
        s.max_hp = Field::Value(0.0);
    }
    crate::divine_combat::apply_stat_modifiers(r, a, &mut s);
    if r.simulation_revision.value_or(29.) < 50. && owner.id == r.player.id && crate::crafting::has_dream_craft(r) {
        s.max_energy = integer_stat(s.max_energy * 1.5);
    }
    s.max_energy = s.max_energy.max(0.0);
    s
}
fn add_stat(s: &mut Stats, k: &str, n: f64) {
    match k {
        "offense" => s.offense += n,
        "defense" => s.defense += n,
        "accuracy" => s.accuracy += n,
        "dodge" => s.dodge += n,
        "dr" => s.dr += n,
        "perception" => s.perception += n,
        "maxEnergy" => s.max_energy += n,
        "maxHp" => s.max_hp = Field::Value(s.max_hp.value_or(0.0) + n),
        _ => {}
    }
}

/// Frozen bestiary `bleeds !== false`, including legacy names and Mercurial identities.
pub fn creature_bleeds(a: &crate::model::Actor) -> bool {
    crate::enemy_traits::definition(a).is_none_or(|d| d.bleeds != Some(false))
}

/// Source-specific mitigation; dormant heart counts survive loss of the ability.
pub fn adaptive_armor_dr(defender: &crate::model::Actor, source: &crate::model::Actor) -> f64 {
    if !defender
        .abilities
        .iter()
        .any(|id| id == "kative-adaptive-armor")
    {
        return 0.0;
    }
    static TYPES: std::sync::OnceLock<std::collections::BTreeMap<String, String>> =
        std::sync::OnceLock::new();
    let types = TYPES.get_or_init(|| {
        serde_json::from_str(include_str!("../catalog/heart-types-v48.json")).unwrap()
    });
    let raw = source
        .name
        .strip_prefix("Mercuria’s ")
        .or_else(|| source.name.strip_prefix("Mercuria's "))
        .unwrap_or(&source.name);
    let kind = types.get(raw).map(String::as_str).unwrap_or(raw);
    let n = defender
        .adaptive_hearts
        .as_ref()
        .and_then(|m| m.get(kind))
        .copied()
        .unwrap_or(0.0)
        .floor()
        .clamp(0.0, 3.0);
    n * (7.0 - n) / 2.0
}

#[cfg(test)]
#[path = "revision52_accuracy_tests.rs"]
mod revision52_accuracy_tests;
