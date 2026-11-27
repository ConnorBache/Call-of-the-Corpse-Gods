//! Read-only v48 compactLog/logEvent presentation. Provenance remains authoritative.
use crate::render::Rgb;
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub message: String,
    pub kind: String,
    pub count: usize,
    pub index: usize,
}
/// Hide routine recovery and descriptive-only lines in both Chronicle views.
pub fn visible(message: &str) -> bool {
    let text = message.trim().trim_end_matches('.');
    !text.ends_with(" waits silently beside the doorway")
        && !text.ends_with(" recovers after its ranged attack")
        && !["Ranged enemy recovers", "Mountainous area", "Corn Dimension",
            "This chamber belongs to the connected Saidribee Nest",
            "Permafrost spreads through these connected chambers",
            "A flower-rich grove fills this pocket of the Flesh"]
            .iter().any(|line| text.eq_ignore_ascii_case(line))
        && !text.strip_prefix("Floor ").and_then(|tail| tail.split_once(": "))
            .is_some_and(|(depth, biome)| !depth.is_empty()
                && depth.bytes().all(|b| b.is_ascii_digit()) && !biome.is_empty())
}

/// O(M*L + min(M,30)*C*V*L), M bounded to the recent log; inputs immutable.
pub fn compact(messages: &[String], kinds: &[String]) -> Vec<Group> {
    let mut groups: Vec<Group> = Vec::new();
    let start = messages.iter().enumerate().rev().filter(|(_, message)| visible(message))
        .take(30).last().map_or(messages.len(), |(index, _)| index);
    for (index, message) in messages
        .iter()
        .enumerate()
        .skip(start)
        .filter(|(_, message)| visible(message))
    {
        let kind = kinds.get(index).map(String::as_str).unwrap_or("unknown");
        if let Some(prior) = groups
            .last_mut()
            .filter(|p| p.message == *message && p.kind == kind)
        {
            prior.count += 1;
            prior.index = index;
        } else {
            groups.push(Group {
                message: message.clone(),
                kind: kind.into(),
                count: 1,
                index,
            });
        }
    }
    for group in &mut groups { group.message = present(&group.message); }
    groups
}
pub fn event(message: &str) -> &'static str {
    if message == "You hear an explosion!" {
        return "explosion";
    }
    let lower = message.to_ascii_lowercase();
    let words: Vec<_> = lower
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .filter(|s| !s.is_empty())
        .collect();
    let phrase = |p: &str| {
        lower.match_indices(p).any(|(i, _)| {
            let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
            (i == 0 || !word(lower.as_bytes()[i - 1]))
                && (i + p.len() == lower.len() || !word(lower.as_bytes()[i + p.len()]))
        })
    };
    let has = |set: &[&str]| words.iter().any(|w| set.contains(w));
    if has(&["die", "dies", "died", "kill", "kills", "killed"]) {
        "death"
    } else if has(&[
        "damage", "hit", "hits", "attack", "attacks", "strike", "strikes", "bite", "bites",
        "shoot", "shoots", "miss", "misses",
    ]) {
        "combat"
    } else if has(&[
        "burning",
        "paralyzed",
        "paralysed",
        "paralyzes",
        "paralyses",
        "paralysis",
        "paralyzis",
        "paralyz",
        "paralys",
        "poison",
        "poisoned",
        "poisoning",
        "bleeding",
        "venom",
    ]) {
        "condition"
    } else if has(&["blessing", "blessings", "acquired", "learned"])
        || phrase("picked up")
        || words.iter().any(|w| {
            w.strip_prefix("leve").is_some_and(|tail| {
                let tail = tail.trim_start_matches('l');
                tail.len() < w.len() - 4 && matches!(tail, "" | "ed" | "s")
            })
        })
    {
        "progress"
    } else if has(&["pause", "pauses", "wait", "waits", "wander", "wanders"])
        || phrase("it is slow")
        || phrase("nothing happens")
    {
        "quiet"
    } else {
        "event"
    }
}
pub fn color(message: &str) -> Rgb {
    if message=="Bestiary Pages Added!" {return [193,138,255];}
    match event(message) {
        "death" => [216, 66, 96],
        "explosion" | "progress" => [196, 174, 98],
        "quiet" => [160, 153, 171],
        _ => crate::render::INK,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn consecutive_and_provenance_only() {
        let m = ["wait", "wait", "wait", "hit", "wait"].map(String::from);
        let k = ["player", "player", "enemy", "enemy", "player"].map(String::from);
        let g = compact(&m, &k);
        assert_eq!(
            g.iter().map(|g| g.count).collect::<Vec<_>>(),
            vec![2, 1, 1, 1]
        );
        assert_eq!(g[0].index, 1);
        assert_eq!(g[2].message, "hit");
    }
    #[test]
    fn source_priority_order() {
        assert_eq!(event("You hear an explosion!"), "explosion");
        assert_eq!(event("Poison kills Blorb."), "death");
        assert_eq!(event("You hit the burning Blorb."), "combat");
        assert_eq!(event("You acquired a heart."), "progress");
        assert_eq!(event("You wait."), "quiet");
        assert_eq!(event("The waiter smiles."), "event");
    }
    #[test]
    fn bounded_recent_window() {
        let m = vec!["Wait".into(); 80];
        let g = compact(&m, &[]);
        assert_eq!(g[0].count, 30);
        assert_eq!(g[0].index, 79);
    }
    #[test]
    fn routine_recovery_and_area_labels_are_not_chronicle_events() {
        let raw = [
            "Pale Spitter recovers after its ranged attack.",
            "Ranged enemy recovers",
            "Mountainous Area",
            "Corn Dimension.",
            "Floor 12: Mountain.",
            "You descend.",
            "You ascend.",
            "You recover from paralysis.",
            "The Pale Spitter shoots at you.",
        ].map(String::from);
        let lines: Vec<_> = compact(&raw, &[]).into_iter().map(|group| group.message).collect();
        assert_eq!(lines, ["You descend.", "You ascend.", "You recover from paralysis.",
            "The Pale Spitter shoots at you."]);
    }
    #[test]
    fn doorway_waits_do_not_displace_visible_recent_entries() {
        let mut messages = vec!["You wait.".into(); 29];
        messages.extend(vec!["Tiger waits silently beside the doorway.".into(); 40]);
        messages.push("Tiger bites you for 3 damage.".into());
        let groups = compact(&messages, &[]);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].count, 29);
        assert_eq!(groups[1].index, 69);
        assert!(groups[1].message.ends_with("bites you for 3 damage."));
    }
}

/// Continuous screen-space fade: older lines soften toward the top. Smoothstep
/// avoids fixed per-message brightness bands and preserves chronology verbatim.
pub fn fade(position: f64) -> f64 {
    let t = position.clamp(0., 1.);
    0.32 + 0.68 * t * t * (3. - 2. * t)
}

/// Presentation-only articles for authored common-creature combat names.
/// O(C*V*L) worst-case text work, O(L) output; C catalog names, V fixed verbs,
/// L message bytes. First use adds O(C log C) catalog setup; later calls reuse it.
/// Never changes Run/log/RNG.
pub fn present(message: &str) -> String {
    static NAMES: std::sync::OnceLock<Vec<&'static str>> = std::sync::OnceLock::new();
    let names = NAMES.get_or_init(|| {
        let mut names: Vec<_> = corpse_core::population::catalog().actors.values()
            .map(|a| a.name.as_str()).collect();
        names.extend(corpse_core::new_creatures::templates().iter().map(|a| a.name.as_str()));
        names.sort_unstable_by_key(|name| std::cmp::Reverse(name.len()));
        names.dedup();
        names
    });
    // Longer projectile phrases precede their short legacy forms. A miss uses
    // "tries to <base verb>", so the same object grammar applies inside it.
    const VERBS: &[&str] = &["fire a magical bolt at", "fires a magical bolt at",
        "fire a moonbeam at", "fires a moonbeam at", "jet water at", "jets water at",
        "shoot at", "shoots at", "throw at", "throws at", "spit at", "spits at", "fire at", "fires at",
        "strike", "strikes", "hit", "hits", "miss", "misses", "shoot", "shoots",
        "bite", "bites", "slash", "slashes", "skewer", "skewers", "pierce", "pierces",
        "sting", "stings", "gore", "gores", "crush", "crushes", "claw", "claws",
        "kill", "kills", "attack", "attacks"];
    let mut output = crate::ability_ui::display_text(message).into_owned();
    if names.iter().any(|name| message.strip_prefix(name).is_some_and(|tail|
        VERBS.iter().any(|verb| tail.strip_prefix(" tries to ").or_else(|| tail.strip_prefix(' '))
            .and_then(|s|s.strip_prefix(verb)).is_some_and(|s|s.starts_with(' '))))) {
        output.insert_str(0, "The ");
    }
    for verb in VERBS {
        let pattern = format!(" {verb} ");
        if let Some(at) = output.find(&pattern) {
            let start = at + pattern.len();
            let object = &output[start..];
            if names.iter().any(|name| object.strip_prefix(name).is_some_and(|tail|
                tail.is_empty() || [".", "!", "?", ",", " for ", " with ", " but ", " and "]
                    .iter().any(|end|tail.starts_with(end)))) {
                output.insert_str(start, "the ");
            }
            break;
        }
    }
    crate::display_names::present(&output).into_owned()
}

#[cfg(test)]
mod article_tests {
    use super::*;
    #[test]
    fn common_combat_articles_are_presentational_and_idempotent() {
        for (raw, expected) in [
            ("You strike Giant Rat for 8 damage.", "You strike the Giant Rat for 8 damage."),
            ("Pale Biter bites Giant Rat for 3 damage.", "The Pale Biter bites the Giant Rat for 3 damage."),
            ("You miss Cave Bat.", "You miss the Cave Bat."),
            ("You throw at Bear for 2 damage.", "You throw at the Bear for 2 damage."),
            ("You strike the Giant Rat for 8 damage.", "You strike the Giant Rat for 8 damage."),
            ("You strike Epidite for 8 damage.", "You strike Epidite for 8 damage."),
            ("You strike something unseen for 8 damage.", "You strike something unseen for 8 damage."),
            ("You acquired Giant Rat heart.", "You acquired Giant Rat heart."),
            ("Night Crawler skewers Giant Rat for 8 damage.", "The Night Crawler skewers the Giant Rat for 8 damage."),
            ("Night Crawler tries to skewer you, but misses.", "The Night Crawler tries to skewer you, but misses."),
            ("You try to slash Night Crawler, but miss.", "You try to slash the Night Crawler, but miss."),
            ("You shoot at Giant Rat for 8 damage.", "You shoot at the Giant Rat for 8 damage."),
            ("You try to shoot at Cave Bat, but miss.", "You try to shoot at the Cave Bat, but miss."),
            ("Unicorn fires a magical bolt at Giant Rat for 8 damage.", "The Unicorn fires a magical bolt at the Giant Rat for 8 damage."),
            ("River Drake jets water at Giant Rat for 8 damage.", "The River Drake jets water at the Giant Rat for 8 damage."),
            ("Cancer crushes you for 8 damage.", "The Cancer crushes you for 8 damage."),
        ] { assert_eq!(present(raw), expected); assert_eq!(present(expected), expected); }
        let messages = vec!["You strike Giant Rat for 8 damage.".into(); 2];
        let before = messages.clone(); let groups = compact(&messages, &[]);
        assert_eq!(groups.len(), 1); assert_eq!(groups[0].count, 2);
        assert_eq!(messages, before);
        assert_eq!(groups[0].message, "You strike the Giant Rat for 8 damage.");
    }
}
