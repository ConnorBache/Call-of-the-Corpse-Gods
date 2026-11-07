//! Authored plate composition only. No decorative geometry or effects.
//! Cosmetic only: no simulation state, random source or unbounded elapsed-time work.
use super::Entry;

#[derive(Clone, Copy, Debug)]
pub(super) struct Direction {
    pub(super) slot: usize,
    pub(super) lift: i32,
    pub(super) scale: f64,
    pub(super) title_reverse: bool,
    variant: u8,
}
pub(super) fn direction(e: &Entry) -> Direction {
    // Explicit placements privilege each silhouette: tall figures puncture the title,
    // broad figures bridge the lower rule, and paired/cosmic figures take the center.
    let (slot, lift, scale, title_reverse, variant): (usize, i32, f64, bool, u8) = match e.id.as_str() {
        "blorb" => (0, 5, 1.02, false, 0),
        "rat" => (2, 0, 1.08, false, 1),
        "shellback-beetle" => (1, 0, 1.04, true, 2),
        "hemogoblin" => (0, -8, 1.08, true, 3),
        "cave-bat" => (1, -25, 1.16, false, 4),
        "pale:Jackal" => (2, -6, 1.10, true, 5),
        "needleling" => (0, -32, 1.10, false, 6),
        "bastidion" => (1, -18, 1.12, true, 7),
        "short-saidri" => (2, -14, 1.08, false, 8),
        "pale:Biter" => (0, -8, 1.12, true, 9),
        "dryad" => (2, -30, 1.14, false, 10),
        "hemogoblin-bloodmancer" => (1, -20, 1.10, true, 11),
        "zillennia:infant" => (0, 5, 1.00, false, 12),
        "lesser-ichor-fish" => (2, 4, 1.06, false, 13),
        "ichor-fish-standard" => (0, 0, 1.10, true, 14),
        "pale:Breather" => (2, -8, 1.10, false, 15),
        "zillennia:child" => (2, -8, 1.04, true, 16),
        "pale:Spitter" => (0, -10, 1.08, false, 17),
        "pale:Screamer" => (1, -24, 1.12, true, 18),
        "pale:Birther" => (2, 2, 1.08, false, 19),
        "greater-ichor-fish" => (1, -6, 1.14, true, 20),
        "giant-beetle" => (0, -5, 1.16, true, 21),
        "samara" => (2, -34, 1.16, false, 22),
        "glim" => (1, -7, 1.16, true, 23),
        "zillennia:lesser" => (1, -15, 1.08, true, 24),
        "bear" => (0, -5, 1.12, false, 25),
        "witch" => (2, -34, 1.10, true, 26),
        "horn-1" => (0, -28, 1.12, false, 27),
        "horn-2" => (2, -24, 1.12, true, 28),
        "sphinx" => (1, -16, 1.12, false, 29),
        "lesser-ichor-shark" => (0, -4, 1.12, true, 30),
        "hemogoblin-inventor" => (2, -18, 1.08, false, 31),
        "hemogoblin-mech" => (1, -18, 1.16, true, 32),
        "zillennia:unqualified" => (0, -22, 1.10, false, 33),
        "tiger" => (2, -2, 1.14, true, 34),
        "bombardier" => (0, -8, 1.10, false, 35),
        "long-saidri" => (2, -35, 1.18, true, 36),
        "kelpie" => (1, -26, 1.14, false, 37),
        "wendigo" => (0, -38, 1.18, true, 38),
        "zillennia:mature" => (2, -26, 1.12, true, 39),
        "ichor-shark" => (1, -5, 1.16, false, 40),
        "pale:Greater Biter" => (2, -16, 1.18, true, 41),
        "pale:Greater Spitter" => (0, -22, 1.14, false, 42),
        "troll" => (1, -24, 1.18, true, 43),
        "pale:Greater Breather" => (0, -18, 1.16, false, 44),
        "pale:Greater Screamer" => (2, -32, 1.16, true, 45),
        "zillennia:greater" => (1, -30, 1.14, false, 46),
        "pale:Greater Birther" => (1, -4, 1.18, true, 47),
        "greater-ichor-shark" => (2, -10, 1.18, true, 48),
        "dire-bear" => (0, -20, 1.18, true, 49),
        "river-drake" => (1, -18, 1.16, false, 50),
        "horn-3" => (1, -30, 1.14, true, 51),
        "killspawn" => (2, -35, 1.18, false, 52),
        "zillennia:hulking" => (0, -32, 1.18, true, 53),
        "horn-4" => (0, -32, 1.16, false, 54),
        "sabretooth" => (2, -16, 1.16, true, 55),
        "lesser-ichor-leviathan" => (0, -14, 1.18, false, 56),
        "vampire" => (2, -32, 1.14, true, 57),
        "banshee" => (1, -38, 1.16, false, 58),
        "maulchild" => (0, -18, 1.14, true, 59),
        "zillennia:ancient" => (2, -34, 1.16, true, 60),
        "horn-5" => (2, -34, 1.18, true, 61),
        "horn-6" => (1, -36, 1.18, false, 62),
        "phoenix" => (1, -30, 1.18, true, 63),
        "yeti" => (0, -24, 1.18, false, 64),
        "ichor-leviathan" => (1, -18, 1.18, true, 65),
        "moonspawn" => (2, -18, 1.10, false, 66),
        "zillennia:timeless" => (1, -36, 1.18, true, 67),
        "valta" => (0, -28, 1.14, true, 68),
        "saidri-guardian" => (2, -30, 1.18, false, 69),
        "chimera" => (0, -22, 1.18, true, 70),
        "cancer" => (1, -8, 1.18, false, 71),
        "horn-7" => (0, -38, 1.18, true, 72),
        "zillennia:transcendent" => (1, -38, 1.18, false, 73),
        "greater-ichor-leviathan" => (2, -24, 1.18, true, 74),
        "ursa-minor" => (0, -22, 1.14, false, 75),
        "ursa-major" => (1, -32, 1.18, true, 76),
        "minotaur" => (2, -30, 1.16, false, 77),
        "dragon" => (1, -36, 1.18, true, 78),
        "mini-me" => (0, 5, 1.00, false, 79),
        "half-here" => (2, -18, 1.12, true, 80),
        "night-crawler" => (1, -10, 1.08, false, 81),
        "skin-walker" => (2, -30, 1.10, true, 82),
        "saidribee-queen" => (1, -20, 1.12, true, 83),
        id => panic!("Missing creature page direction: {id}"),
    };
    // The left plate shares the title origin; reserve its text-safe top edge.
    Direction { slot, lift: if slot == 0 { lift.max(-5) } else { lift },
        scale: if slot == 0 { scale.min(1.04_f64) } else { scale },
        title_reverse, variant }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_creatures_have_bounded_composition_without_decorative_renderer() {
        let m=super::super::Manual::new();
        let mut variants=std::collections::HashSet::new();
        for e in &m.catalog.entries { let d=direction(e);
            assert!(d.slot<3); assert!((-38..=5).contains(&d.lift));
            assert!((1.0..=1.18).contains(&d.scale)); assert!(variants.insert(d.variant));
        }
        assert_eq!(variants.len(),m.catalog.entries.len());
    }
}
