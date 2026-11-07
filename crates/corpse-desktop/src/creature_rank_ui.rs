//! Stable identity ranks, not live combat power; presentation only.
use corpse_core::Actor;
/// O(1): bounded matches (at most ten qualities); no simulation mutation.
pub fn rank(a: &Actor) -> Option<u8> {
    match a.creature_kind.as_ref().map(String::as_str).unwrap_or("") {
        "lesser-ichor-fish" | "lesser-ichor-shark" | "lesser-ichor-leviathan" => Some(1),
        "ichor-fish" | "ichor-fish-standard" | "ichor-shark" | "ichor-leviathan" => Some(2),
        "greater-ichor-fish" | "greater-ichor-shark" | "greater-ichor-leviathan" => Some(3),
        "zillennia" => match a.zillennia_quality.as_ref().map(String::as_str)? {
            "infant" => Some(1), "child" => Some(2), "lesser" => Some(3),
            "unqualified" => Some(4), "mature" => Some(5), "greater" => Some(6),
            "hulking" => Some(7), "ancient" => Some(8), "timeless" => Some(9),
            "transcendent" => Some(10), _ => None,
        },
        "phantasmii-nightmare" => a.conditions.get("nightmareTier").copied()
            .filter(|n| n.is_finite() && n.fract() == 0. && (1. ..=8.).contains(n)).map(|n| n as u8),
        _ => None,
    }
}
/// Rank owns the superscript; existing ally/clone markers move to the subscript.
pub fn markers(a: &Actor, revision: f64, dead_or_body: bool, player: bool, old: String) -> (String, String) {
    if revision >= 50. && !dead_or_body && !player {
        if let Some(rank) = rank(a) { return (rank.to_string(), old); }
    }
    (old, String::new())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn approved_creature_ranks_cover_all_families_and_tiers() {
        for family in ["fish", "shark", "leviathan"] {
            for (prefix, expected) in [("lesser-",1),("",2),("greater-",3)] {
                let kind = if family=="fish" && prefix.is_empty() {"ichor-fish-standard".into()} else {format!("{prefix}ichor-{family}")};
                let a = Actor {creature_kind:kind.into(), ..Default::default()};
                assert_eq!(rank(&a),Some(expected));
            }
        }
        for (i,q) in ["infant","child","lesser","unqualified","mature","greater","hulking","ancient","timeless","transcendent"].iter().enumerate() {
            let a = Actor {creature_kind:"zillennia".to_string().into(),zillennia_quality:q.to_string().into(),..Default::default()};
            assert_eq!(rank(&a),Some(i as u8+1));
        }
        for n in 1..=8 { assert_eq!(rank(&corpse_core::phantasmii_generation::nightmare((n-1)*5+1,1.,1.,1.)),Some(n as u8)); }
    }
    #[test]
    fn ranks_preserve_orders_and_do_not_mutate_or_reveal_legacy_corpses() {
        let a=Actor {creature_kind:"zillennia".to_string().into(),zillennia_quality:"transcendent".to_string().into(),..Default::default()};
        let before=serde_json::to_vec(&a).unwrap();
        assert_eq!(markers(&a,50.,false,false,"a".into()),("10".into(),"a".into()));
        assert_eq!(markers(&a,49.,false,false,"a".into()),("a".into(),"".into()));
        assert_eq!(markers(&a,50.,true,false,"".into()),("".into(),"".into()));
        assert_eq!(markers(&a,50.,false,true,"1".into()),("1".into(),"".into()));
        assert_eq!(serde_json::to_vec(&a).unwrap(),before);
    }
}
