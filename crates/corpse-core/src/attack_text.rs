//! Evidence-backed Chronicle attack wording; never participates in combat rules.
use crate::{Actor, Item, Run};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Verb {
    pub base: &'static str,
    pub third: &'static str,
}
const fn verb(base: &'static str, third: &'static str) -> Verb { Verb { base, third } }
const STRIKE: Verb = verb("strike", "strikes");
const BITE: Verb = verb("bite", "bites");
const SLASH: Verb = verb("slash", "slashes");

/// Fixed authored weapon names and supported suffixes, not arbitrary substrings.
/// O(L) name comparisons, O(1) scratch. Unknown/custom equipment stays neutral.
pub(crate) fn weapon(item: Option<&Item>) -> Verb {
    let Some(item) = item.filter(|i| i.kind == "weapon" && i.ranged_weapon.as_ref().is_none()) else {
        return STRIKE;
    };
    if ["Shortsword", "Bastard sword", "Greatsword", "Hatchet"].iter().any(|name|
        item.name.strip_prefix(name).is_some_and(|tail|
            tail.is_empty() || tail.starts_with(" of ") || tail == ", Cursed" || tail.starts_with(", Cursed "))) {
        SLASH
    } else { STRIKE }
}

/// O(C*L) cold/warm catalog identity lookup, O(1) scratch after catalog loading.
/// Only explicit anatomy/lore or established bite behavior is specialized.
/// No random synonyms, display-name suffix guesses, or unsupported damage types.
pub(crate) fn creature(a: &Actor) -> Verb {
    if let Some(definition) = crate::enemy_traits::definition(a) {
        return match definition.id.as_str() {
            "rat" | "bear" | "dire-bear" | "tiger" | "sabretooth"
            | "lesser-ichor-fish" | "ichor-fish-standard" | "greater-ichor-fish" | "ichor-fish"
            | "lesser-ichor-shark" | "ichor-shark" | "greater-ichor-shark" => BITE,
            "night-crawler" => verb("skewer", "skewers"),
            "needleling" => verb("pierce", "pierces"),
            "short-saidri" | "long-saidri" | "saidri-guardian" => verb("sting", "stings"),
            "horn-2" | "horn-4" | "horn-6" => verb("gore", "gores"),
            "cancer" => verb("crush", "crushes"),
            "killspawn" => SLASH,
            "ursa-major" => verb("claw", "claws"),
            _ => STRIKE,
        };
    }
    // Pale identities have no definition rows. Match only catalog-authored names,
    // preserving their existing bite actions without classifying e.g. "Toy Bear".
    match crate::revision50_creature_identity::legacy_name(crate::enemy_traits::base_name(&a.name)) {
        "Pale Biter" | "Greater Pale Biter" | "Pale Jackal"
        | "Pale Screamer" | "Greater Pale Screamer" => BITE,
        _ => STRIKE,
    }
}

/// Current ranged callers each have one authored projectile family. A known
/// species must also retain the matching live ability; unknown actors stay neutral.
pub(crate) fn ranged(a: &Actor, player: bool, arrow: bool) -> Verb {
    if arrow { return verb("shoot at", "shoots at"); }
    if player { return verb("throw at", "throws at"); }
    let has = |id: &str| a.abilities.iter().any(|ability| ability == id);
    if has("enemy:pale:spit") { return verb("spit at", "spits at"); }
    if let Some(definition) = crate::enemy_traits::definition(a) {
        match definition.id.as_str() {
            "river-drake" if has("enemy:beast:water-jet") => return verb("jet water at", "jets water at"),
            "valta" if has("enemy:beast:moon-laser") => return verb("fire a moonbeam at", "fires a moonbeam at"),
            "horn-1" | "horn-3" | "horn-5" | "horn-7" if has("enemy:beast:horn-bolt") =>
                return verb("fire a magical bolt at", "fires a magical bolt at"),
            _ => (),
        }
    }
    verb("fire at", "fires at")
}

pub(crate) fn select(r: &Run, a: &Actor, ranged_attack: bool, arrow: bool) -> Verb {
    if ranged_attack || arrow { return ranged(a, a.id == r.player.id, arrow); }
    if a.id == r.player.id { STRIKE } else { creature(a) }
}

pub(crate) fn hit(subject: &str, object: &str, action: Verb, player: bool, damage: f64) -> String {
    let verb = if player { action.base } else { action.third };
    format!("{subject} {verb} {object} for {damage} damage.")
}

pub(crate) fn miss(subject: &str, object: &str, action: Verb, player: bool) -> String {
    // A miss must not claim contact: "skewers ... but misses" is contradictory.
    let (attempt, miss) = if player { ("try", "miss") } else { ("tries", "misses") };
    format!("{subject} {attempt} to {} {object}, but {miss}.", action.base)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_sources_and_neutral_unknowns() {
        for (kind, expected) in [
            ("night-crawler", "skewers"), ("needleling", "pierces"),
            ("short-saidri", "stings"), ("long-saidri", "stings"), ("saidri-guardian", "stings"),
            ("horn-2", "gores"), ("horn-4", "gores"), ("horn-6", "gores"),
            ("cancer", "crushes"), ("killspawn", "slashes"), ("ursa-major", "claws"),
            ("sabretooth", "bites"), ("ichor-shark", "bites"), ("witch", "strikes"),
            ("troll", "strikes"), ("yeti", "strikes"), ("moonspawn", "strikes"),
        ] {
            let actor = crate::population::create_actor_revision(kind, 10., 6., 5., false, 46.).unwrap();
            assert_eq!(creature(&actor).third, expected, "{kind}");
        }
        for name in ["Toy Bear", "Clockwork Tiger", "Infant Zillennia", "Unspecified fighter"] {
            assert_eq!(creature(&Actor { name: name.into(), ..Default::default() }), STRIKE);
        }
        let mut actor = crate::population::create_actor_revision("night-crawler", 10., 6., 5., false, 46.).unwrap();
        actor.name = "Renamed companion".into();
        assert_eq!(creature(&actor).third, "skewers");
    }
    #[test]
    fn exact_weapon_identity_and_conjugation() {
        for name in ["Shortsword", "Bastard sword of Fire", "Greatsword, Cursed", "Hatchet"] {
            let item = Item { name: name.into(), kind: "weapon".into(), ..Default::default() };
            assert_eq!(weapon(Some(&item)), SLASH);
        }
        for name in ["Shard of the Borge", "Shortswordfish", "Shortsword unknown", "Shortsword, Cursedfish", "Unspecified club"] {
            let item = Item { name: name.into(), kind: "weapon".into(), ..Default::default() };
            assert_eq!(weapon(Some(&item)), STRIKE);
        }
        assert_eq!(weapon(None), STRIKE);
        assert_eq!(hit("You", "Giant Rat", SLASH, true, 8.), "You slash Giant Rat for 8 damage.");
        assert_eq!(hit("Killspawn", "you", SLASH, false, 0.), "Killspawn slashes you for 0 damage.");
        assert_eq!(miss("You", "Giant Rat", SLASH, true), "You try to slash Giant Rat, but miss.");
        assert_eq!(miss("Night Crawler", "you", verb("skewer", "skewers"), false),
            "Night Crawler tries to skewer you, but misses.");
    }
    #[test]
    fn projectiles_are_not_melee_attacks() {
        for (kind, expected) in [("river-drake", "jets water at"), ("valta", "fires a moonbeam at"), ("horn-3", "fires a magical bolt at")] {
            let mut actor = crate::population::create_actor_revision(kind, 10., 6., 5., false, 46.).unwrap();
            assert_eq!(ranged(&actor, false, false).third, expected);
            actor.abilities.clear();
            assert_eq!(ranged(&actor, false, false).third, "fires at");
        }
        assert_eq!(ranged(&Actor::default(), true, true).base, "shoot at");
        assert_eq!(ranged(&Actor::default(), true, false).base, "throw at");
    }
}
