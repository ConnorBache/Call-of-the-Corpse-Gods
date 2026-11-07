//! Runtime-only player maximum-health invalidation.
//!
//! This cache is not a general effective-stat cache. It guards the repeated
//! settlement calls at normal-turn boundaries. Mutation-boundary callers may
//! continue to force `equipment::sync_health`; those calls record the new clean
//! state. Item bonus/enhancement/equipped changes must invalidate or force-settle.
use crate::{Field, Run};

/// Fixed-size derived bookkeeping, intentionally absent from canonical saves.
/// Clone/default/equality are O(1) time and space; equality ignores cache warmth
/// because it is not gameplay state. Deserialization must use the dirty default.
#[derive(Debug, Clone, Default)]
pub struct HealthDirty {
    stamp: Option<[u64; 14]>,
}

impl PartialEq for HealthDirty {
    /// O(1): different cache warmth never makes otherwise equal Runs unequal.
    fn eq(&self, _other: &Self) -> bool { true }
}

/// O(1) time/space: preserves Missing/Null distinctions for canonical field
/// normalization. Float bits avoid NaN-dependent cache equality behavior.
fn field_bits(value: &Field<f64>) -> (u64, u64) {
    match value {
        Field::Missing => (0, 0),
        Field::Null => (1, 0),
        Field::Value(n) => (2, n.to_bits()),
    }
}

/// O(1) expected time and O(1) space: a fixed number of scalar reads and four
/// fixed-key condition-map lookups. No actor, inventory, ability or map scan.
/// Equipment mutations are covered by explicit invalidation/forced settlement.
fn stamp(r: &Run) -> [u64; 14] {
    let p = &r.player;
    let base = field_bits(&p.base_max_hp);
    let extra = field_bits(&p.stats.max_hp);
    [
        p.id.to_bits(), base.0, base.1, p.stats.defense.to_bits(),
        extra.0, extra.1,
        p.conditions.get("buff:defense").copied().unwrap_or(0.).to_bits(),
        p.conditions.get("buff:maxHp").copied().unwrap_or(0.).to_bits(),
        p.max_hp.to_bits(), r.enemy_health_revision.value_or(0.).to_bits(),
        r.divine_revision.value_or(0.).to_bits(), r.inventory.len() as u64,
        // Sustain Strength distinguishes timed reductions from persistent ones.
        (crate::engine::condition(p, "buffTicks:defense") > 0.) as u64,
        (crate::engine::condition(p, "buffTicks:maxHp") > 0.) as u64,
    ]
}

/// O(1) expected time/space. Scalar fallback also notices callers that directly
/// mutate the public player's defense/buffs/base HP/output max HP between calls.
/// It cannot detect arbitrary direct inventory writes: call `invalidate` there.
pub fn needs_sync(r: &Run) -> bool { r.health_dirty.stamp != Some(stamp(r)) }

/// O(1) time/space. Use at item health-bonus/equipped changes, full-state restore
/// or any mutation not represented by the fixed scalar stamp.
pub fn invalidate(r: &mut Run) { r.health_dirty.stamp = None; }

/// O(1) expected time/space; called only after original health settlement.
pub(crate) fn mark_clean(r: &mut Run) { r.health_dirty.stamp = Some(stamp(r)); }

#[cfg(test)]
mod tests {
    use super::*;

    /// O(1) fixture construction, no generated world or collection-sized data.
    fn run() -> Run {
        let mut r = Run::default();
        r.player.id = 1.;
        r.player.hp = 50.;
        r.player.max_hp = 100.;
        r.status = "playing".into();
        r
    }

    /// O(1) expected cache work; forced health settlement inherits Stats cost,
    /// which is constant for this empty fixture.
    #[test]
    fn default_dirty_then_clean_and_scalar_changes_detected() {
        let mut r = run();
        assert!(needs_sync(&r));
        crate::equipment::sync_health(&mut r, false);
        assert!(!needs_sync(&r));
        assert!(!crate::equipment::flush_health(&mut r));
        r.player.stats.defense = 2.;
        assert!(needs_sync(&r));
        assert!(crate::equipment::flush_health(&mut r));
        assert_eq!(r.player.max_hp, 112.);
        assert_eq!(r.player.hp, 62.);
        r.player.conditions.insert("buff:defense".into(), 1.);
        assert!(crate::equipment::flush_health(&mut r));
        assert_eq!(r.player.max_hp, 118.);
        r.player.conditions.shift_remove("buff:defense");
        assert!(crate::equipment::flush_health(&mut r));
        assert_eq!(r.player.max_hp, 112.);
    }

    /// O(1) expected cache work. HP changes alone are not a maximum-HP change.
    #[test]
    fn damage_and_regeneration_do_not_dirty_maximum_health() {
        let mut r = run();
        crate::equipment::sync_health(&mut r, false);
        r.player.hp -= 4.;
        assert!(!needs_sync(&r));
        r.player.hp += 0.2;
        assert!(!needs_sync(&r));
        invalidate(&mut r);
        assert!(needs_sync(&r));
    }

    /// O(S) serialize/deserialize for fixture bytes S; runtime cache is omitted.
    #[test]
    fn cache_warmth_is_not_canonical_state_or_semantic_equality() {
        let mut r = run();
        crate::equipment::sync_health(&mut r, false);
        let warm = r.clone();
        invalidate(&mut r);
        assert_eq!(r, warm);
        let json = serde_json::to_string(&warm).unwrap();
        assert!(!json.contains("health_dirty"));
        let restored: Run = serde_json::from_str(&json).unwrap();
        assert!(needs_sync(&restored));
    }
    /// O(S) comparison for serialized fixture size S; excludes cache by design.
    fn same_gameplay(a: &Run, b: &Run) {
        assert_eq!(serde_json::to_value(a).unwrap(), serde_json::to_value(b).unwrap());
    }

    /// O(T*(I+E+C+B+S)) fixture comparison over T mutations; exercises exact
    /// forced equipment loss versus non-equipment clamp and rounding behavior.
    #[test]
    fn dirty_settlement_matches_forced_for_buffs_equipment_and_crafts() {
        let mut a = run();
        crate::equipment::sync_health(&mut a, false);
        let mut b = a.clone();
        for defense in [0.49, 0.5, 2.2, -1.5, 0.0] {
            a.player.conditions.insert("buff:defense".into(), defense);
            b.player.conditions.insert("buff:defense".into(), defense);
            crate::equipment::sync_health(&mut a, false);
            crate::equipment::flush_health(&mut b);
            same_gameplay(&a, &b);
        }
        let mut item = crate::Item::default();
        item.id = 2.;
        item.kind = "armor".into();
        item.equipped = "armor".to_string().into();
        item.bonus = crate::PartialStats { defense: 2.5.into(), max_hp: 3.5.into(), ..Default::default() }.into();
        a.inventory.push(item.clone());
        b.inventory.push(item);
        // Public inventory mutation requires the dispatch admission invalidation.
        invalidate(&mut b);
        crate::equipment::sync_health(&mut a, false);
        crate::equipment::flush_health(&mut b);
        same_gameplay(&a, &b);
        let enhancement = crate::ItemEnhancementsEntry {
            bonus: crate::PartialStats { max_hp: 8.5.into(), ..Default::default() },
            ..Default::default()
        };
        a.inventory[0].enhancements.push(enhancement.clone());
        b.inventory[0].enhancements.push(enhancement);
        // Crafting's existing forced settlement remains a mutation boundary.
        crate::equipment::sync_health(&mut a, false);
        crate::equipment::sync_health(&mut b, false);
        assert!(!crate::equipment::flush_health(&mut b));
        same_gameplay(&a, &b);
        a.inventory.clear(); b.inventory.clear();
        crate::equipment::sync_health(&mut a, true);
        crate::equipment::sync_health(&mut b, true);
        assert!(!crate::equipment::flush_health(&mut b));
        same_gameplay(&a, &b);
    }

    /// O(B+H+S), abilities B, achievements H, canonical fixture bytes S.
    #[test]
    fn clean_health_still_unlocks_blood_and_fury_exactly_once() {
        let mut a = run();
        crate::equipment::sync_health(&mut a, false);
        a.player.abilities = vec![crate::divine_combat::FIGHT_LONG.into(), crate::divine_combat::BLOOD_WEAPON.into()];
        let mut b = a.clone();
        crate::equipment::sync_health(&mut a, false);
        assert!(!crate::equipment::flush_health(&mut b));
        same_gameplay(&a, &b);
        crate::equipment::sync_health(&mut a, false);
        assert!(!crate::equipment::flush_health(&mut b));
        same_gameplay(&a, &b);
    }

    /// O(I+S) fixture mutation/settlement and canonical comparison; verifies
    /// same-length removal/replacement cannot hide an equipped health change.
    #[test]
    fn equipped_take_then_replace_invalidates_even_at_same_length() {
        let mut a = run();
        let mut item = crate::Item::default();
        item.id = 2.;
        item.kind = "heart".into();
        item.equipped = "hand1".to_string().into();
        item.bonus = crate::PartialStats { defense: 2.0.into(), ..Default::default() }.into();
        a.inventory.push(item);
        crate::equipment::sync_health(&mut a, false);
        let mut b = a.clone();
        crate::inventory::take_one(&mut a, 0);
        crate::inventory::take_one(&mut b, 0);
        a.inventory.push(crate::Item::default());
        b.inventory.push(crate::Item::default());
        assert!(needs_sync(&b));
        crate::equipment::sync_health(&mut a, false);
        assert!(crate::equipment::flush_health(&mut b));
        same_gameplay(&a, &b);
    }

}
