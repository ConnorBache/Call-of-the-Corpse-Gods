//! Frozen dynamic creature ability prose. Only observed actors may enter this
//! read-only adapter. Linked-partner state is never inspected for flavor.
use corpse_core::{engine, half_here, lifecycle, Actor, Run};
use serde::Deserialize;
use std::{collections::HashSet, sync::OnceLock};
#[derive(Deserialize)]
struct Enemy {
    id: String,
    description: String,
    cost: f64,
    passive: bool,
    compact: Option<String>,
}
#[derive(Deserialize)]
struct Definition {
    id: String,
    name: String,
    abilities: Vec<String>,
}
#[derive(Deserialize)]
struct Divine {
    id: String,
    name: String,
    flags: Vec<String>,
    effect: String,
    controlled: bool,
    activation: f64,
    upkeep: f64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Data {
    enemy: Vec<Enemy>,
    definitions: Vec<Definition>,
    zillennia_names: Vec<String>,
    divine: Vec<Divine>,
}
fn data() -> &'static Data {
    static D: OnceLock<Data> = OnceLock::new();
    D.get_or_init(|| {
        let mut data:Data=serde_json::from_str(include_str!("../assets/creature-abilities-v48.json")).unwrap();
        data.enemy.push(Enemy {id:"enemy:beast:saidribee-queen".into(),description:"summons a Short Saidribee or Saidribee Guardian into a free adjacent land square in a 2:1 ratio".into(),cost:15.,passive:false,compact:Some("Summons a Saidribee".into())});
        data
    })
}
fn condition(a: &Actor, k: &str) -> f64 {
    engine::condition(a, k)
}
fn has(a: &Actor, kind: &str) -> bool {
    a.abilities
        .iter()
        .any(|s| s == &format!("enemy:beast:{kind}"))
}
fn owner<'a>(r: &'a Run, a: &'a Actor) -> &'a Actor {
    engine::actor(r, half_here::owner_id(r, a)).unwrap_or(a)
}
fn turns(n: f64) -> String {
    format!("{n} {}", if n == 1. { "turn" } else { "turns" })
}
fn capital(s: &str) -> String {
    let mut chars = s.chars();
    chars.next().map_or(String::new(), |c| {
        c.to_uppercase().collect::<String>() + chars.as_str()
    })
}
fn trim(s: &str) -> &str {
    s.trim_end_matches(|c: char| c == '.' || c.is_whitespace())
}
fn fragment(effect: &str) -> String {
    let text = if let Some(rest) = effect.strip_prefix("can ") {
        let (verb, tail) = rest.split_once(' ').unwrap_or((rest, ""));
        let verb = if verb == "be" {
            "is".into()
        } else if verb.ends_with('y')
            && verb
                .chars()
                .rev()
                .nth(1)
                .is_some_and(|c| !"aeiou".contains(c))
        {
            format!("{}ies", &verb[..verb.len() - 1])
        } else if ["s", "x", "z", "ch", "sh", "o"]
            .iter()
            .any(|end| verb.ends_with(end))
        {
            format!("{verb}es")
        } else {
            format!("{verb}s")
        };
        format!("{verb}{}{}", if tail.is_empty() { "" } else { " " }, tail)
    } else {
        effect.into()
    };
    format!("{}.", capital(trim(&text)))
}
fn uninflect(effect: &str) -> String {
    let text = effect.strip_prefix("can ").unwrap_or(effect);
    let (word, rest) = text.split_once(' ').unwrap_or((text, ""));
    let word = if [
        "flees", "commits", "feeds", "senses", "consumes", "gains", "inflicts", "holds", "heals",
        "stores", "spends",
    ]
    .contains(&word)
    {
        &word[..word.len() - 1]
    } else if word == "is" {
        "be"
    } else {
        word
    };
    format!("{word}{}{}", if rest.is_empty() { "" } else { " " }, rest)
}
fn meta(id: &str) -> Option<&'static Enemy> {
    data().enemy.iter().find(|e| e.id == id)
}
fn original(name: &str) -> &str {
    name.strip_prefix("Mercuria’s ")
        .or_else(|| name.strip_prefix("Mercuria's "))
        .unwrap_or(name)
}
fn intrinsic(a: &Actor) -> Vec<String> {
    let name = corpse_core::revision50_creature_identity::legacy_name(original(a.creature_kind.as_ref().unwrap_or(&a.name)));
    let name = if matches!(name, "vent-fungus" | "Vent Fungus") {
        "bastidion"
    } else {
        name
    };
    if let Some(d) = data()
        .definitions
        .iter()
        .find(|d| d.id == name || d.name == name)
    {
        return d.abilities.clone();
    }
    if a.creature_kind.as_ref().is_some_and(|s| s == "zillennia")
        || data()
            .zillennia_names
            .iter()
            .any(|n| n == original(&a.name))
    {
        return ["ruin-pulse", "silence-aura", "darkness-aura"]
            .map(|s| format!("enemy:beast:{s}"))
            .to_vec();
    }
    if !a
        .name
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .any(|s| s == "Pale")
    {
        return vec![];
    }
    let mut ids = vec!["enemy:pale:gas-immunity".into()];
    for (suffix, traits) in [
        (
            "Breather",
            &["breath", "breath-boost", "gas-vulnerability"][..],
        ),
        ("Biter", &["bite-repeat", "lifesteal"][..]),
        ("Spitter", &["spit", "gas-shot"][..]),
        ("Birther", &["birth", "double-birth", "slow"][..]),
        ("Screamer", &["scream"][..]),
    ] {
        if corpse_core::revision50_creature_identity::legacy_name(original(&a.name)).ends_with(suffix) {
            ids.extend(traits.iter().map(|s| format!("enemy:pale:{s}")));
        }
    }
    ids
}
fn traits(a: &Actor) -> Vec<String> {
    let mut ids = a.abilities.clone();
    if condition(a, "paleTraitsInitialized") == 0.
        && condition(a, "bestiaryTraitsInitialized") == 0.
    {
        ids.extend(intrinsic(a));
    }
    let mut seen = HashSet::new();
    ids.into_iter()
        .filter(|id| meta(id).is_some() && seen.insert(id.clone()))
        .collect()
}
fn mech(a: &Actor) -> bool {
    a.creature_kind
        .as_ref()
        .is_some_and(|s| s == "hemogoblin-mech")
        || has(a, "mech")
}
fn piloted(a: &Actor) -> bool {
    a.monster_state
        .as_ref()
        .is_some_and(|s| s.pilot.as_ref().is_some())
}
fn caster(a: &Actor) -> bool {
    [
        "bloodmancy",
        "witch",
        "dryad",
        "horn-bolt",
        "beckon",
        "wall-seal",
        "moon-laser",
    ]
    .iter()
    .any(|s| has(a, s))
}
fn applies(a: &Actor, id: &str, ids: &HashSet<&str>) -> bool {
    match id {
        "enemy:pale:gas-shot" => ids.contains("enemy:pale:spit"),
        "enemy:pale:double-birth" => ids.contains("enemy:pale:birth"),
        "enemy:beast:banshee" => lifecycle::has_banshee_shriek(a),
        "enemy:beast:ursa-minor" => lifecycle::has_ursa_death_cry(a),
        "enemy:beast:ursa-major" => lifecycle::is_ursa_companion(a),
        "enemy:beast:stalk" => !matches!(a.state.as_str(), "ally" | "neutral"),
        "enemy:beast:yeti" => !half_here::is_clone(a),
        "enemy:beast:phoenix" => {
            !half_here::is_clone(a)
                && condition(a, "miniMe") == 0.
                && !a.born_from_birther.value_or(false)
        }
        "enemy:beast:grab" => {
            !ids.contains("enemy:beast:deep-water") && !ids.contains("enemy:beast:hold-only")
        }
        "enemy:beast:aquatic" | "enemy:beast:slime-bound" => {
            !ids.contains("enemy:beast:ichor-bound")
        }
        "enemy:beast:venom-weak" => {
            !ids.contains("enemy:beast:venom") && !ids.contains("enemy:beast:venom-strong")
        }
        "enemy:beast:venom" => !ids.contains("enemy:beast:venom-strong"),
        _ => true,
    }
}
fn birth(a: &Actor, id: &str, detailed: bool) -> String {
    let one = id == "enemy:pale:birth";
    if a.name.starts_with("Greater ") {
        format!(
            "{}{}",
            if one {
                "an ordinary Pale"
            } else {
                "two ordinary Pales"
            },
            if detailed {
                ", chosen from Biter, Breather, Spitter, Birther, and Screamer"
            } else {
                ""
            }
        )
    } else {
        if one { "a Jackal" } else { "two Jackals" }.into()
    }
}
fn compact_enemy(a: &Actor, e: &Enemy) -> String {
    let id = e.id.as_str();
    let mut effect = e.compact.as_ref().unwrap_or(&e.description).clone();
    match id {
        "enemy:pale:birth" | "enemy:pale:double-birth" => {
            effect = format!("Summons {}", birth(a, id, false))
        }
        "enemy:beast:inventor" => {
            effect = if a.conditions.contains_key("monsterLinkedMechId") {
                "Flees toward its linked mech and pilots it if dormant"
            } else {
                "Flees when threatened. Has no assigned mech"
            }
            .into()
        }
        "enemy:beast:mech" => {
            effect = if piloted(a) {
                "Moves and attacks while piloted"
            } else {
                "Needs a pilot to move or attack"
            }
            .into()
        }
        "enemy:beast:moon-laser" if condition(a, "laserCharge") > 0. => {
            return "Fires its charged moonbeam (already paid).".into()
        }
        "enemy:beast:everburn" if has(a, "fire-immunity") => effect = "Burns continuously".into(),
        "enemy:beast:sphinx" if half_here::is_clone(a) => effect = "Ignores damage".into(),
        "enemy:beast:wendigo"
            if a.conditions
                .get("monsterWendigoLevel")
                .copied()
                .unwrap_or(1.)
                >= 5. =>
        {
            effect = "Has finished growing. Flees while injured".into()
        }
        _ => {}
    }
    let cost = if id == "enemy:beast:detonate" {
        a.energy.max(0.)
    } else {
        e.cost
    };
    if cost > 0. || id == "enemy:beast:detonate" {
        effect += &format!(
            " ({cost} NRG{})",
            if id == "enemy:beast:detonate" {
                ", all remaining"
            } else {
                ""
            }
        );
    }
    fragment(&effect)
}
fn cooldown(r: &Run, a: &Actor, id: &str) -> f64 {
    match id {
        "enemy:beast:fire-breath" => (condition(a, "monsterFireReadyAt") - r.tick).max(0.),
        "enemy:beast:bloodmancy" | "enemy:beast:witch" => {
            (condition(a, "monsterSpellReadyAt") - r.tick).max(0.)
        }
        "enemy:beast:dryad" => (condition(a, "monsterHealReadyAt") - r.tick).max(0.),
        "enemy:beast:blink"
        | "enemy:beast:beckon"
        | "enemy:beast:wall-seal"
        | "enemy:beast:ruin-pulse" => condition(a, "beastCooldown").max(0.),
        _ => 0.,
    }
}
fn detailed_enemy(r: &Run, a: &Actor, e: &Enemy) -> String {
    let id = e.id.as_str();
    let mut effect = e.description.clone();
    let mut limits = Vec::<String>::new();
    match id {
        "enemy:beast:bloodmancy" => limits.extend([
            "within 7 tiles along a clear projectile path".into(),
            "3-turn cooldown".into(),
            format!(
                "alternating casts; next is {}",
                if condition(a, "monsterSpellPhase") % 2. == 0. {
                    "a pull"
                } else {
                    "Hemophilia"
                }
            ),
        ]),
        "enemy:beast:witch" => limits.extend([
            "within 7 tiles along a clear projectile path".into(),
            "3-turn cooldown".into(),
            format!(
                "next is {}",
                ["Slowed", "Hemophilia", "Poisoned"]
                    .get((condition(a, "monsterSpellPhase") % 3.) as usize)
                    .unwrap_or(&"undefined")
            ),
        ]),
        "enemy:beast:saidribee-queen" => {
            limits.push("successful casts wait 10 world turns; haste does not shorten the cooldown".into());
        }
        "enemy:beast:fire-breath" => {
            limits.extend(["within 6 tiles".into(), "4-turn cooldown".into()])
        }
        "enemy:beast:dryad" => limits.extend([
            "within 6 tiles, excluding itself".into(),
            "2-turn cooldown".into(),
        ]),
        "enemy:beast:horn-bolt" => limits.push("within 10 tiles".into()),
        "enemy:beast:water-jet" => {
            limits.push("within 6 tiles along a clear projectile path".into())
        }
        "enemy:beast:moon-laser" => {
            limits.push("within 16 tiles".into());
            if condition(a, "laserCharge") != 0. {
                effect =
                    "has a charged moonbeam ready to fire along a clear projectile path".into();
                limits.push("already paid; releasing it costs no further energy".into())
            }
        }
        "enemy:beast:blink" => {
            limits.push("up to 5 tiles".into());
            if caster(a) {
                limits.push("its casting-and-retreat behavior does not use this approach teleport against observed threats".into())
            }
        }
        "enemy:beast:wall-seal" => {
            limits.push("the wall lasts 5 turns".into());
            if condition(a, "raisedWallExpires") > r.tick {
                limits.push(format!(
                    "its existing wall lasts {} longer; it cannot raise another yet",
                    turns(condition(a, "raisedWallExpires") - r.tick)
                ))
            }
        }
        "enemy:pale:spit" => limits.push("within 12 tiles".into()),
        "enemy:pale:scream" => limits.push("nearby effects reach 10 tiles".into()),
        "enemy:pale:birth" | "enemy:pale:double-birth" => {
            effect = effect.replace("{offspring}", &birth(a, id, true));
            if r.simulation_revision.value_or(29.0) >= 32.0 {
                limits.push("a successful birth has a shared 10-world-turn cooldown; haste does not shorten it".into());
                let remaining = condition(a, "monsterSummonReadyAt") - r.tick;
                if remaining > 0.0 {
                    limits.push(format!("next birth in {}", turns(remaining)));
                }
                if a.heartless.value_or(false) {
                    limits.push("this heartless creature's minions are also heartless".into());
                }
            }
            limits.push(
                if id == "enemy:pale:birth" {
                    "requires an empty adjacent land tile"
                } else {
                    "requires two empty adjacent land tiles"
                }
                .into(),
            );
        }
        "enemy:beast:vampire" => {
            if corpse_core::monster_vampire::flying(a) {
                limits.push("currently in bat form".into());
            }
            if condition(a, "monsterRetreating") != 0. {
                limits.push("currently retreating until it recovers".into());
            }
        }
        "enemy:beast:inventor" => {
            effect = if a.conditions.contains_key("monsterLinkedMechId") {
                "flees toward its linked mech and can enter it if it is still dormant"
            } else {
                "flees when threatened and can enter a linked dormant mech, but has none assigned"
            }
            .into()
        }
        "enemy:beast:ursa-minor" => {
            effect = "calls its linked companion to its death site if that companion is still alive"
                .into()
        }
        "enemy:beast:ursa-major" => {
            effect = "can be awakened by its linked companion’s death cry".into()
        }
        "enemy:beast:mech" if piloted(a) => {
            effect = "is currently piloted and able to move and attack".into()
        }
        "enemy:beast:minotaur" => {
            let arrow = if a.conditions.contains_key("monsterChargeDx")
                && a.conditions.contains_key("monsterChargeDy")
            {
                match (
                    condition(a, "monsterChargeDx"),
                    condition(a, "monsterChargeDy"),
                ) {
                    (0., -1.) => "↑",
                    (1., -1.) => "↗",
                    (1., 0.) => "→",
                    (1., 1.) => "↘",
                    (0., 1.) => "↓",
                    (-1., 1.) => "↙",
                    (-1., 0.) => "←",
                    (-1., -1.) => "↖",
                    _ => "",
                }
            } else {
                ""
            };
            if !arrow.is_empty() {
                limits.push(format!("currently committed {arrow}"));
            }
            if condition(a, "monsterChargeRemaining") > 0. {
                limits.push(format!(
                    "{} movement attempts remain",
                    condition(a, "monsterChargeRemaining")
                ));
            }
        }
        "enemy:beast:wendigo" => {
            let level = a
                .conditions
                .get("monsterWendigoLevel")
                .copied()
                .unwrap_or(1.);
            if level >= 5. {
                effect = "has finished growing from corpses and still flees while injured".into();
            } else {
                limits.push(format!("currently at level {level}"));
            }
        }
        "enemy:beast:banshee" => {
            limits.push("every 30 turns".into());
            if let Some(n) = lifecycle::banshee_turns_until(r, a) {
                limits.push(format!("next shriek in {}", turns(n)));
            }
        }
        "enemy:beast:sphinx" => {
            if half_here::is_clone(a) {
                effect = "is invulnerable".into();
            } else {
                let remaining = a
                    .conditions
                    .get("monsterSphinxHuntAt")
                    .copied()
                    .unwrap_or(r.tick)
                    - r.tick;
                if remaining > 0. {
                    limits.push(format!("begins hunting in {}", turns(remaining)));
                } else if !a.conditions.contains_key("monsterSphinxHuntAt") {
                    limits.push("waits 3 turns after provocation".into());
                }
            }
        }
        "enemy:beast:everburn" if has(a, "fire-immunity") => effect = "is always Burning".into(),
        "enemy:beast:gas-immunity" if matches!(original(&a.name), "Bastidion" | "Vent Fungus" | "Corpse Fungus") => {
            limits.push("flames can still ignite its body".into())
        }
        "enemy:beast:detonate" => limits.push(
            if corpse_core::combat::nearest_damage_enabled(r) {
                "within 5 tiles; deals half of each affected creature’s maximum health, rounded to the nearest whole number"
            } else { "within 5 tiles; deals half of each affected creature’s maximum health, rounded up" }.into(),
        ),
        "enemy:beast:ruin-pulse" | "enemy:beast:silence-aura" => {
            limits.push("within 3 tiles".into())
        }
        _ => {}
    }
    if [
        "enemy:beast:blink",
        "enemy:beast:beckon",
        "enemy:beast:wall-seal",
        "enemy:beast:ruin-pulse",
    ]
    .contains(&id)
    {
        limits.push("3-turn shared cooldown".into());
    }
    let charged = id == "enemy:beast:moon-laser" && condition(a, "laserCharge") > 0.;
    let cost = if id == "enemy:beast:detonate" {
        a.energy.max(0.)
    } else {
        e.cost
    };
    if !charged && (cost > 0. || id == "enemy:beast:detonate") {
        effect += &format!(" ({cost} NRG)");
        if id == "enemy:beast:detonate" {
            limits.push("spends all current energy".into());
        }
        if a.energy < cost || (id == "enemy:beast:detonate" && a.energy <= 0.) {
            limits.push("currently lacks enough energy".into());
        } else if condition(a, "silenced") > 0. {
            limits.push("currently blocked by Silence".into());
        }
    }
    let waiting = cooldown(r, a, id);
    if waiting > 0. {
        limits.push(format!("ready again in {}", turns(waiting)));
    }
    if has(a, "rooted")
        && [
            "enemy:pale:spit",
            "enemy:beast:horn-bolt",
            "enemy:beast:water-jet",
            "enemy:beast:moon-laser",
        ]
        .contains(&id)
    {
        limits.push("its Rooted trait currently prevents the direct strike".into());
    }
    let expiry = condition(owner(r, a), &format!("remembered:{id}"));
    if expiry > r.tick {
        limits.push(format!("copied for {} more", turns(expiry - r.tick)));
    }
    let needs_ai = ([
        "enemy:beast:vampire",
        "enemy:beast:inventor",
        "enemy:beast:minotaur",
        "enemy:beast:wendigo",
        "enemy:beast:stalk",
        "enemy:beast:deep-water",
    ]
    .contains(&id))
        || (!e.passive
            && ![
                "enemy:pale:bite-repeat",
                "enemy:pale:lifesteal",
                "enemy:pale:gas-shot",
            ]
            .contains(&id));
    let limit = if limits.is_empty() {
        String::new()
    } else {
        format!("; {}", limits.join("; "))
    };
    if half_here::is_clone(a) && needs_ai {
        format!("A creature acting independently could {}{limit}.",uninflect(&effect))
    } else {
        format!("It {}.", trim(&(effect + &limit)))
    }
}
fn flag(d: &Divine, f: &str) -> bool {
    d.flags.iter().any(|s| s == f)
}
fn price(r: &Run, a: &Actor, d: &Divine) -> (String, bool) {
    if d.id==corpse_core::future_floor::ID && a.id==r.player.id && corpse_core::future_floor::enabled(r) {
        return ("No energy cost".into(),false);
    }
    let name = d.name.as_str();
    let toggle = flag(d, "toggle");
    let craft = flag(d, "craft") && !toggle && !flag(d, "activate");
    let enabled = if name == "See The Future" {
        a.id == r.player.id && r.future.as_ref().is_some()
    } else {
        a.active.contains(&d.id)
    };
    let upkeep = if name == "Unspeak" {
        if enabled {
            condition(a, "unspeakCost")
        } else {
            0.
        }
    } else {
        d.upkeep
    };
    let activation = match name {
        "See The Future" => a.energy,
        "Dream of Time Gone By" => corpse_core::balance_rules::dream_anchor_cost(r),
        _ => d.activation,
    };
    let suppression = ["Entropy", "Shine", "Star Power"].contains(&name);
    let mut price = match name {
        "Rage at the Dying of the Light" => format!(
            "{} energy for the next rescue; cost escalates; level-up resets",
            condition(a, "rageCost")
        ),
        "You Shall Never Be Rid of Me" => if condition(a, "pelianUsed") != 0. {
            "Rescue spent until next level"
        } else {
            "One free rescue per level"
        }
        .into(),
        "Revert" => "2 energy + 1 charge".into(),
        "Fortune Teller" => "1 fortune per use · +1 every 20 turns".into(),
        "Blood Weapon" => format!(
            "{} HP (10% maximum, {}); free action; no energy cost",
            corpse_core::divine_combat::blood_weapon_hp_cost_for_run(r, a),
            if corpse_core::combat::nearest_damage_enabled(r) { "rounded to nearest whole number" } else { "rounded up" }
        ),
        "Blood Letting" => "10 self-damage; no energy cost".into(),
        "See The Future" => {
            if enabled {
                "Return is free; spent energy stays spent".into()
            } else {
                format!("All current energy ({}); return is free", a.energy)
            }
        }
        "Unspeak" => {
            if enabled {
                format!("{upkeep} energy/turn (target's highest ability cost)")
            } else {
                "Target's highest ability cost in energy/turn".into()
            }
        }
        "Unyielding Assault" => "10 energy per attack action".into(),
        "Never Without a Fight" => "10 energy per counterattack".into(),
        "Never Surrender" => "8 energy for the first damaging hit each round".into(),
        "Missed Me" => "1 reaction on the first incoming attack each round".into(),
        "Make Slippery" => "8 energy per blocked move".into(),
        "Quick Hand" => {
            "+1 maximum reaction; 1 reaction when hit for 1 turn of haste; no energy cost".into()
        }
        "Are You Sure About That?" => "1 reaction per redirected miss".into(),
        "Waking Nightmare" => "1 nightmare per prevented damage event".into(),
        "Overflowing" if corpse_core::balance_rules::current(r) => "1 save per rescue".into(),
        "Overflowing" => "1 blood per rescue".into(),
        "Erase Metaphysical" => "10 energy per hit with a removable ability".into(),
        "Nova" => format!("{} energy per attack explosion",
            corpse_core::abilities::nova_attack_cost(r.simulation_revision.value_or(29.))),
        "Uncast" => "10 energy per canceled spell".into(),
        "Fury Crafting" => "Heart + equipment; equipped craft pays 10 energy per attack".into(),
        "Zeal Craft" => "Heart + equipment; equipped craft heals for 15 energy".into(),
        _ if craft => if name == "Divine Craftsman" {
            "No energy cost; permits two crafts per item"
        } else {
            "Heart + equipment; no energy upkeep"
        }
        .into(),
        _ if suppression => "No energy cost to suppress or resume the automatic effect".into(),
        _ if activation != 0. && upkeep != 0. => {
            format!("{activation} energy to activate + {upkeep}/turn")
        }
        _ if upkeep != 0. => format!("{upkeep} energy/turn while enabled"),
        _ if activation != 0. => format!(
            "{activation} energy{}",
            if flag(d, "action") {
                " + one action"
            } else {
                " per use"
            }
        ),
        _ => "No energy cost".into(),
    };
    if toggle && name != "See The Future" && !suppression {
        price.push_str("; turning off is free");
    }
    let callable = toggle
        || flag(d, "activate")
        || flag(d, "target")
        || flag(d, "action")
        || ["Revert", "Blood Letting"].contains(&name);
    let earlier_block = !a.abilities.contains(&d.id)
        || craft
        || !callable
        || r.status != "playing"
        || condition(a, "deathResolutionBlocked") != 0.
        || (a.id == r.player.id && !r.offers.is_empty())
        || (name == "Blood Weapon" && corpse_core::divine_combat::blood_weapon_ready(r, a))
        || condition(a, "paralyzed") > 0.
        || enabled
        || (name == "Revert"
            && ((a.id != r.player.id || r.history.is_empty())
                || condition(a, "revertCharges") < 1.))
        || (name == "Fortune Teller" && a.conditions.get("fortunes").copied().unwrap_or(3.) < 1.);
    (price, !earlier_block && a.energy < activation + upkeep)
}
fn divine(r: &Run, a: &Actor, d: &Divine, compact: bool) -> String {
    let own = owner(r, a);
    if d.id==corpse_core::future_floor::ID && own.id==r.player.id && corpse_core::future_floor::enabled(r) {
        return if compact {"Futuresight.".into()} else {crate::ability_ui::FUTURE_FLOOR_DESCRIPTION.into()};
    }
    let enabled = own.active.contains(&d.id);
    let (price, lacks) = price(r, own, d);
    let mut notes = Vec::new();
    let suppressed = match d.name.as_str() {
        "Entropy" => Some("gas emission"),
        "Shine" => Some(if r.simulation_revision.value_or(29.) >= 46. { "light emission" } else { "self-illumination" }),
        "Star Power" => Some("flames"),
        _ => None,
    };
    if let Some(what) = suppressed {
        notes.push(format!(
            "{what} currently {}",
            if enabled { "suppressed" } else { "present" }
        ));
    } else if flag(d, "toggle") {
        notes.push(
            if enabled {
                "currently enabled"
            } else {
                "currently disabled"
            }
            .into(),
        );
    }
    let priced = !price.starts_with("No energy cost")
        && !price.starts_with("Heart + equipment; no energy upkeep");
    if priced {
        notes.push(
            price
                .replace("energy", "NRG")
                .trim_end_matches("; turning off is free")
                .into(),
        );
    }
    if own.abilities.contains(&d.id) {
        let resource = match d.name.as_str() {
            "Overflowing" if corpse_core::balance_rules::current(r) =>
                Some(format!("saves: {} / 5", corpse_core::divine_rescues::saves(own))),
            "Overflowing" => Some(format!("blood: {} / 55", own.blood.value_or(0.))),
            "Waking Nightmare" => Some(format!("nightmares: {}", own.nightmares.value_or(0.))),
            "Revert" => Some(format!(
                "rewind charges: {} / 5",
                condition(own, "revertCharges")
            )),
            "Fortune Teller" => Some(format!(
                "fortunes: {} / 3",
                own.conditions.get("fortunes").copied().unwrap_or(3.)
            )),
            "Missed Me" | "Are You Sure About That?" | "Quick Hand" => Some(format!(
                "reactions: {} / {}",
                own.reactions.value_or(0.),
                own.abilities
                    .iter()
                    .filter(|id| data().divine.iter().any(|d| d.id == **id
                        && ["Missed Me", "Are You Sure About That?", "Quick Hand"]
                            .contains(&d.name.as_str())))
                    .count()
            )),
            "Blood Weapon" => Some(format!(
                "prepared attack: {}",
                if corpse_core::divine_combat::blood_weapon_ready(r, own) {
                    "Ready"
                } else {
                    "Not prepared"
                }
            )),
            "Rage at the Dying of the Light" => Some(format!(
                "next rescue: {} energy",
                condition(own, "rageCost")
            )),
            "You Shall Never Be Rid of Me" => Some(format!(
                "rescues: {} / 1",
                if condition(own, "pelianUsed") != 0. {
                    0
                } else {
                    1
                }
            )),
            _ => None,
        };
        if let Some(s) = resource {
            notes.push(s);
        }
        let expiry = condition(own, &format!("remembered:{}", d.id));
        if expiry > r.tick {
            notes.push(format!("copied for {} remaining", turns(expiry - r.tick)));
        }
    }
    if lacks {
        notes.push("currently lacks enough energy".into());
    }
    if !enabled
        && condition(own, "silenced") > 0.
        && notes.iter().any(|n| {
            n.as_bytes()
                .windows(5)
                .any(|w| w[0].is_ascii_digit() && &w[1..] == b" NRG")
        })
    {
        notes.push("new energy-paid activation is blocked by Silence".into());
    }
    let independent = own.id != r.player.id;
    let player_only = independent && (d.controlled || (!flag(d, "passive") && !flag(d, "toggle")));
    let mut effect = match (corpse_core::balance_rules::current(r), d.id.as_str()) {
        (true, "lichor-overflowing") => "has five death saves that replenish at full HP or upon leveling up".into(),
        (true, "shrail-fight-long") => "heals for 3% of actual melee damage dealt".into(),
        (true, "ursa-sudden-sleep") => "can restore full health at the cost of 25 turns of Paralysis".into(),
        (true, "ursa-sleep-walk") => "gains Haste below 10% of maximum energy, or at zero energy if uncapped".into(),
        _ => d.effect.clone(),
    };
    if independent && d.name == "Reverse Entropy" {
        effect = "can halt nearby environmental growth and spread".into();
    }
    if compact {
        let mut short = Vec::new();
        if priced {
            let part = if d.name == "Quick Hand" {
                price.split(';').nth(1).unwrap_or("").into()
            } else if ["Fury Crafting", "Zeal Craft"].contains(&d.name.as_str()) {
                price.replace("; equipped craft", ", craft")
            } else {
                price.clone()
            };
            let part = part.replace("energy", "NRG");
            let mut part = part
                .split([';', '·'])
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            for (from, to) in [
                (" per attack action", "/attack"),
                (" per use", "/use"),
                (" while enabled", ""),
                (" for the first damaging hit each round", "/round"),
                (" on the first incoming attack each round", "/round"),
                (" when hit for 1 turn of haste", "/hit"),
            ] {
                if part.ends_with(from) {
                    part.truncate(part.len() - from.len());
                    part.push_str(to);
                }
            }
            short.push(part);
        }
        if player_only {
            return fragment(&format!(
                "Carries {}{}, requiring player control",
                d.name,
                if short.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", short.join(", "))
                }
            ));
        }
        if suppressed.is_some() {
            effect = match d.name.as_str() {
                "Entropy" => format!(
                    "Ignores caustic gas. {}",
                    if enabled {
                        "Suppresses gas emission"
                    } else {
                        "Emits caustic gas"
                    }
                ),
                "Shine" => format!(
                    "Aims better at illuminated foes. {}",
                    if enabled {
                        if r.simulation_revision.value_or(29.) >= 46. { "Suppresses light emission" } else { "Suppresses self-illumination" }
                    } else {
                        "Illuminates nearby ground"
                    }
                ),
                _ => format!(
                    "Ignores fire damage. {}",
                    if enabled {
                        "Suppresses its flames"
                    } else {
                        "Burns continuously"
                    }
                ),
            };
        } else if flag(d, "toggle") {
            short.push(if enabled { "enabled" } else { "disabled" }.into());
        }
        if d.name == "Missed Me" {
            effect = "Halves the first attack’s hit chance each round".into();
        }
        return fragment(
            &(effect
                + &if short.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", short.join(", "))
                }),
        );
    }
    if player_only {
        return format!(
            "Its carried {} power requires player control ({}{}).",
            d.name,
            effect,
            if notes.is_empty() {
                String::new()
            } else {
                format!("; {}", notes.join("; "))
            }
        );
    }
    format!(
        "It {}.",
        trim(
            &(effect
                + &if notes.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", notes.join("; "))
                })
        )
    )
}
fn text(r: &Run, a: &Actor, compact: bool) -> String {
    let own = owner(r, a);
    let copied;
    let source = if own.id == a.id {
        a
    } else {
        copied = Actor {
            abilities: own
                .abilities
                .iter()
                .filter(|id| id.as_str() != "etill-half-here")
                .cloned()
                .collect(),
            active: own.active.clone(),
            energy: own.energy,
            ..a.clone()
        };
        &copied
    };
    let mut ids = traits(source);
    if corpse_core::revision51_ichor_fire::immune(r.simulation_revision.value_or(29.), source) {
        ids.push("enemy:beast:fire-immunity".into());
    }
    ids.extend(source.abilities.clone());
    let mut seen = HashSet::new();
    ids.retain(|id| seen.insert(id.clone()));
    let idset = ids.iter().map(String::as_str).collect();
    let mut parts = Vec::new();
    let mut actions = false;
    let mut divine_actions = false;
    for id in &ids {
        if r.simulation_revision.value_or(29.)>=50. && id==corpse_core::revision50_combat::MAGICKAL_CASTER {
            parts.push(if compact {"Magickal Caster."} else {"Counts as a Magickal Caster for Witch-Hatred."}.into());
        } else if let Some(d)=corpse_core::progression::definition_for_revision(r.simulation_revision.value_or(29.),id)
            .filter(|_|r.simulation_revision.value_or(29.)>=50.) {
            let floor_future=id==corpse_core::future_floor::ID && own.id==r.player.id && corpse_core::future_floor::enabled(r);
            let passive_ragelight=id==corpse_core::divine_rescues::RAGE && corpse_core::ragelight_rules::current(r);
            let toggle=!passive_ragelight && d.flags.iter().any(|flag|flag=="toggle");
            let state=if floor_future {if corpse_core::future_floor::active(r) {" (active)"} else {" (used)"}} else if toggle {if source.active.contains(id) {" (active)"} else {" (inactive)"}} else {""};
            let reserve=if d.name == "Hanged" || floor_future { String::new() } else { crate::ability_ui::actor_reserve_summary_for_revision(source,id,r.simulation_revision.value_or(29.))
                .map(|s|format!(" {s}." )).unwrap_or_default() };
            let description = if floor_future || passive_ragelight || id == corpse_core::revision50_combat::EXECUTE || (r.simulation_revision.value_or(29.) >= 52. && id == "pelian-never-surrender")
                || (corpse_core::balance_rules::current(r) && matches!(id.as_str(),
                    "lichor-overflowing" | "shrail-fight-long" | "ursa-sudden-sleep"
                    | "ursa-dream-of-time-gone-by" | "ursa-sleep-walk")) {
                crate::ability_ui::display_description(r, id, &d.description)
            } else { d.description.as_str() };
            let name=if floor_future {"Futuresight"}else{d.name.as_str()};
            let rescue_cost=if passive_ragelight {
                let cost=crate::stat_presentation::whole_number(condition(own,"rageCost"));
                format!(" Next rescue: {cost} NRG.")
            } else {String::new()};
            parts.push(if compact {format!("{name}{state}.{rescue_cost}{reserve}")} else {format!("{name}{state}: {description}{rescue_cost}{reserve}")});
            divine_actions |= !passive_ragelight && d.flags.iter().any(|flag|matches!(flag.as_str(),"toggle"|"activate"|"target"|"action"));
        } else if let Some(e) = meta(id) {
            if !applies(source, id, &idset) {
                continue;
            }
            actions |= !e.passive;
            parts.push(if id == "enemy:beast:fast" && r.simulation_revision.value_or(29.)>=52. {
                "Fast: can move two tiles per action, or move once then attack. This is not Haste.".into()
            } else if id == "enemy:beast:fungal-gas" && r.simulation_revision.value_or(29.) >= 52. {
                format!("Continually emits {} gas. Roots grow along a direct route toward visible corpses within 10 tiles, one tile every 3 normal turns. Once roots reach a corpse, they feed on it, preventing its revival and increasing gas output.", corpse_core::fungi::emission(r, source).0)
            } else if corpse_core::fungi::hallucination_emitter(r, source) && id == "enemy:beast:fungal-gas" {
                "Continually emits hallucinogenic gas. Roots feed on visible corpses within 10 tiles, preventing their revival. Each connected corpse increases its gas output.".into()
            } else if r.simulation_revision.value_or(29.) >= 33. && source.name.starts_with("Greater ") && matches!(id.as_str(),"enemy:pale:breath"|"enemy:pale:breath-boost") {
                if id=="enemy:pale:breath" {"Continually emits hallucinogenic gas.".into()} else {"Exhales an additional hallucinogenic cloud for 10 Energy.".into()}
            } else if r.simulation_revision.value_or(29.) >= 33. && id == "enemy:pale:scream" {
                let gate = if corpse_core::screamer_rules::range_required(r) {
                    " Only screams when you are in range."
                } else { "" };
                format!("Disorients foes within 10 tiles for 5 turns: -{} Accuracy. Costs 15 Energy.{gate}", if source.name.starts_with("Greater ") {10} else {5})
            } else if compact {
                compact_enemy(source, e)
            } else {
                detailed_enemy(r, source, e)
            });
        } else if let Some(d) = data().divine.iter().find(|d| d.id == *id) {
            parts.push(divine(r, source, d, compact));
            divine_actions |= if compact {
                flag(d, "toggle") && !d.controlled
            } else {
                !flag(d, "passive") || flag(d, "toggle")
            };
        } else {
            parts.push(
                if compact {
                    "Has an undescribed ability."
                } else {
                    "It has an ability whose effect description is unavailable."
                }
                .into(),
            );
        }
    }
    if r.simulation_revision.value_or(29.) >= 33. {
        if has(source,"witch") {
            parts.push("Summons one Night Crawler or Skin Walker for 10 Energy, with equal probability. Successful summons share a 10-world-turn cooldown; curses remain available between summons. Heartless parents create heartless offspring.".into());
        }
        if source.creature_kind.as_ref().is_some_and(|k|k=="skin-walker") {
            parts.push("Disguises itself as a much shallower creature beyond two grid squares of an observing player body. Nearer bodies reveal its true form; combat statistics remain unchanged.".into());
        }
        if source.name.starts_with("Greater ") && source.abilities.iter().any(|s|s=="enemy:pale:breath") {
            parts.push("Exposure refreshes Hallucinating to 2 turns while exposed, changing only perceived creature identities.".into());
        }
    }
    if parts.is_empty() {
        return if mech(a) && !piloted(a) {
            if compact {
                "Needs a pilot to move or act."
            } else {
                "It cannot move or use special actions until a pilot enters it."
            }
        } else {
            ""
        }
        .into();
    }
    if half_here::is_clone(a) {
        parts.push(
            if compact {
                "Shares your powers and Energy. Cannot act independently."
            } else {
                "This body shares its powers and energy with you and does not act independently."
            }
            .into(),
        );
    } else {
        if caster(source)
            && !mech(source)
            && !["minotaur", "sphinx", "inventor"]
                .iter()
                .any(|s| has(source, s))
        {
            parts.push(
                if compact {
                    "Flees when threatened."
                } else {
                    "It retreats from observed threats when it is not casting."
                }
                .into(),
            );
        }
        if divine_actions && a.id != r.player.id {
            parts.push(
                if compact {
                    "Cannot activate divine powers independently."
                } else {
                    "It cannot activate its carried divine powers independently."
                }
                .into(),
            );
        }
        if mech(a) && !piloted(a) && (!compact || !idset.contains("enemy:beast:mech")) {
            parts.push(
                if compact {
                    "Needs a pilot to move or act."
                } else {
                    "It cannot move or use special actions until a pilot enters it."
                }
                .into(),
            );
        }
        if a.perfect.value_or(false) && a.order.as_ref().is_some_and(|s| s == "p") && actions {
            parts.push(
                if compact {
                    "Cannot use special actions while rallied."
                } else {
                    "Its current rally order prevents it from using special actions."
                }
                .into(),
            );
        } else if actions && condition(a, "paralyzed") > 0. {
            parts.push(
                if compact {
                    "Cannot act while Paralyzed."
                } else {
                    "It cannot take special actions while Paralyzed."
                }
                .into(),
            );
        } else if actions && condition(a, "rangedRecovery") > 0. {
            parts.push(
                if compact {
                    "Must recover next action."
                } else {
                    "It must spend its next action recovering before using another special action."
                }
                .into(),
            );
        }
    }
    if corpse_core::zillennia_ground::clears_ground(r, a) {
        parts.push(if compact {
            "Clears loose items and ordinary cover on its tile."
        } else {
            "It erases loose ground items, including hearts, and ordinary land cover only on the tile it occupies. Corpses, creatures, carried items, water, Ichor, stairs and structures are spared."
        }.into());
    }
    let mut unique = HashSet::new();
    parts
        .into_iter()
        .filter(|p| unique.insert(p.clone()))
        .collect::<Vec<_>>()
        .join(" ")
}
/// Caller must establish current actor observation before reading its powers.
pub fn describe(r: &Run, a: &Actor) -> (String, String) {
    let current = |value:String| if r.simulation_revision.value_or(29.)>=50. {
        value.replace("Bastidion","Corpse Fungus").replace("Breather","Gasser").replace("Biter","Chomper")
    } else {value};
    let mut compact = current(text(r, a, true));
    let mut details = current(text(r, a, false));

    if corpse_core::revision52_enemy_followups::enabled(r) {
        let species = |kind| corpse_core::revision52_enemy_followups::species(a,kind);
        let extra = if species("dragon") { Some((
            "Takes flight for a faster safe route (8 NRG).",
            "Takes flight for up to 6 turns when it offers a faster route to safe ground (8 NRG).")) }
            else if species("blorb") { Some(("Melee hits add 1 caustic damage.","Damaging melee hits add 1 caustic damage before protection.")) }
            else if species("killspawn") { Some(("Melee hits cause deep bleeding.","Damaging melee hits add half their damage as bleeding over 3 turns. Wounds add together.")) }
            else {None};
        if let Some((short,long)) = extra {
            if !compact.is_empty() {compact.push(' ');} compact.push_str(short);
            if !details.is_empty() {details.push(' ');} details.push_str(long);
        }
    }
    (crate::ability_ui::display_text(&compact).into_owned(), crate::ability_ui::display_text(&details).into_owned())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scream_copy_omits_approved_ally_recap_without_mutation() {
        for revision in [33., 52.] {
            let r = corpse_core::startup::new_run_shell_revision("0-5-11:scream-copy", "Unblessed", false, revision).unwrap();
            for (name, accuracy) in [("Screamer", 5), ("Greater Screamer", 10)] {
                let a = Actor { id: 999., name: name.into(), abilities: vec!["enemy:pale:scream".into()], ..Default::default() };
                let before_run = r.clone(); let before_actor = a.clone();
                let (compact, details) = describe(&r, &a);
                let expected = format!("Disorients foes within 10 tiles for 5 turns: -{accuracy} Accuracy. Costs 15 Energy.");
                for text in [compact, details] {
                    assert!(text.contains(&expected), "{text}");
                    assert!(!text.contains("Allies retain the existing"), "{text}");
                }
                assert_eq!(r, before_run); assert_eq!(a, before_actor);
            }
        }
    }
    #[test]
    fn revision52_followup_copy_is_revision_scoped() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        for (kind,needle) in [("dragon","8 NRG"),("blorb","1 caustic"),("killspawn","3 turns")] {
            let a=corpse_core::population::create_actor_revision(kind,800.,1.,1.,false,51.).unwrap();
            r.simulation_revision=51.0.into();assert!(!describe(&r,&a).1.contains(needle));
            r.simulation_revision=52.0.into();assert!(describe(&r,&a).1.contains(needle));
        }
    }
    #[test]
    fn shine_suppression_status_preserves_legacy_wording_without_mutation() {
        for revision in [45., 46.] { for enabled in [false, true] {
            let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
            r.simulation_revision = revision.into();
            r.player.abilities = vec!["anavani-shine".into()];
            r.player.active = if enabled { r.player.abilities.clone() } else { vec![] };
            r.player.conditions.clear();
            let before = serde_json::to_value(&r).unwrap();
            let (compact, details) = describe(&r, &r.player);
            let label = if revision >= 46. { "light emission" } else { "self-illumination" };
            assert!(details.contains(&format!("{label} currently {}", if enabled { "suppressed" } else { "present" })), "{details}");
            assert!(compact.contains(if enabled {
                if revision >= 46. { "Suppresses light emission" } else { "Suppresses self-illumination" }
            } else { "Illuminates nearby ground" }), "{compact}");
            assert_eq!(serde_json::to_value(&r).unwrap(), before);
        }}
    }
    #[test]
    fn nova_cost_projection_uses_recorded_revision_without_mutation() {
        for (revision, cost) in [(44., 30), (45., 40)] { for player in [false, true] {
            let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
            r.simulation_revision = revision.into();
            let mut a = r.player.clone();
            a.id = if player { r.player.id } else { 1000. };
            a.abilities = vec!["anavani-nova".into()]; a.active = a.abilities.clone(); a.conditions.clear();
            if player { r.player = a.clone(); } else { *r.actors_at_mut(r.depth) = vec![a.clone()]; }
            let before = serde_json::to_value(&r).unwrap();
            let (compact, details) = describe(&r, &a);
            for text in [compact, details] {
                assert!(text.contains(&format!("{cost} NRG per attack explosion")), "{text}");
                assert!(!text.contains(&format!("{} NRG per attack explosion", if cost == 40 { 30 } else { 40 })));
            }
            assert_eq!(serde_json::to_value(&r).unwrap(), before);
        }}
    }
    #[test]
    fn revision33_queen_summon_metadata_reports_actual_cost_and_world_cooldown() {
        let r=corpse_core::startup::new_run_shell_revision("0-5-11:queen-inspect","Unblessed",false,33.).unwrap();
        let a=corpse_core::population::create_actor_revision("saidribee-queen",4.,1.,1.,false,33.).unwrap();
        let before=r.clone();let (compact,details)=describe(&r,&a);
        assert!(compact.contains("Summons a Saidribee"));assert!(details.contains("15 NRG"),"{details}");
        assert!(details.contains("2:1"));assert!(details.contains("10 world turns"));assert_eq!(r,before);
    }
    #[test]
    fn revision33_deeper_zillennia_ground_erasure_is_observation_only_prose() {
        for revision in [29.0, 32.0, 33.0] {
            let r = corpse_core::startup::new_run_shell_revision("0-5-11:zillennia-inspect", "Unblessed", false, revision).unwrap();
            for depth in [1, 20, 21, 40] {
                let a = corpse_core::population::zillennia_revision(depth, revision).unwrap();
                let before = r.clone();
                let (compact, details) = describe(&r, &a);
                let expected = revision >= 33.0 && depth >= 21;
                assert_eq!(compact.contains("Clears loose items"), expected);
                assert_eq!(details.contains("Corpses, creatures, carried items, water"), expected);
                assert_eq!(r, before);
            }
        }
    }
    #[test]
    fn source_1024_dynamic_creature_ability_reports() {
        let cases: serde_json::Value = serde_json::from_str(include_str!(
            "../../../fixtures/creature-abilities-ui-v48.json"
        ))
        .unwrap();
        for c in cases.as_array().unwrap() {
            let mut value: serde_json::Value =
                serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json"))
                    .unwrap();
            value["inventory"] = serde_json::json!([]);
            for (k, v) in c["patch"].as_object().unwrap() {
                value[k] = v.clone();
            }
            let mut r: Run = serde_json::from_value(value).unwrap();
            let a: Actor = serde_json::from_value(c["actor"].clone()).unwrap();
            if c["player"] == true {
                r.player = a.clone();
                r.actors_at_mut(r.depth).clear();
            } else {
                if c["shared"] == true {
                    r.player = serde_json::from_value(c["owner"].clone()).unwrap();
                }
                *r.actors_at_mut(r.depth) = vec![a.clone()];
            }
            let before = r.clone();
            let (compact, details) = describe(&r, &a);
            assert_eq!(
                compact,
                c["compact"].as_str().unwrap(),
                "{} compact",
                c["name"]
            );
            assert_eq!(
                details,
                // Later approved copy cut removes this duplicate shared-body limitation.
                c["details"].as_str().unwrap().replace(" This body cannot use that power independently.", ""),
                "{} details",
                c["name"]
            );
            assert_eq!(r, before);
        }
    }
    #[test]
    fn revision52_fungal_copy_states_slow_contact_dependent_roots() {
        let mut r: Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();
        let a=corpse_core::population::create_actor_revision("bastidion",1000.,1.,1.,false,52.).unwrap();
        for copy in [describe(&r,&a).0,describe(&r,&a).1] {
            assert!(copy.contains("one tile every 3 normal turns"));assert!(copy.contains("Once roots reach a corpse"));
            assert!(copy.contains("hallucinogenic gas"));
        }
        r.simulation_revision=51.0.into();assert!(!describe(&r,&a).1.contains("3 normal turns"));
    }

    #[test]
    fn revision50_bastidion_copy_names_fixed_gas_without_removing_enemy_cooldowns() {
        let mut r: Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=50.0.into();
        let a=corpse_core::population::create_actor_revision("bastidion",1000.,1.,1.,false,50.).unwrap();
        let before=r.clone();let (compact,details)=describe(&r,&a);
        for copy in [compact,details] {assert!(copy.contains("hallucinogenic gas"));assert!(copy.contains("preventing their revival"));}
        assert_eq!(r,before);
        let queen=corpse_core::population::create_actor_revision("saidribee-queen",1001.,1.,1.,false,50.).unwrap();
        assert!(describe(&r,&queen).1.contains("10 world turns"));
        r.simulation_revision=46.0.into();
        assert!(!describe(&r,&a).1.contains("hallucinogenic gas"));
    }

    #[test] fn revision50_caster_marker_is_legible_in_both_creature_descriptions() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=50.0.into();
        for kind in ["witch","hemogoblin-bloodmancer","dryad","horn-1","horn-2","horn-3","horn-4","horn-5","horn-6","horn-7","saidribee-queen","valta"] {
            let a=corpse_core::population::create_actor_revision(kind,1000.,1.,1.,false,50.).unwrap();
            let before=r.clone();let (compact,details)=describe(&r,&a);
            assert!(compact.contains("Magickal Caster."),"{kind}: {compact}");
            assert!(details.contains("Counts as a Magickal Caster for Witch-Hatred."),"{kind}: {details}");
            assert!(!compact.contains("undescribed ability"));assert!(!details.contains("description is unavailable"));
            assert!(!compact.contains(corpse_core::revision50_combat::MAGICKAL_CASTER));assert_eq!(r,before);
            let old=corpse_core::population::create_actor_revision(kind,1000.,1.,1.,false,46.).unwrap();
            assert!(!describe(&r,&old).0.contains("Magickal Caster"));
        }
    }

}

#[cfg(test)]
mod revision51_ichor_fire_tests {
    use super::*;
    #[test]
    fn revision51_ichor_fire_prose_covers_all_tiers_and_preserves_legacy() {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let manual = crate::manual::Manual::new();
        for kind in ["lesser-ichor-fish", "ichor-fish-standard", "greater-ichor-fish",
            "lesser-ichor-shark", "ichor-shark", "greater-ichor-shark",
            "lesser-ichor-leviathan", "ichor-leviathan", "greater-ichor-leviathan"] {
            let a = corpse_core::population::create_actor_revision(kind, 901., 12., 10., false, 51.).unwrap();
            for revision in [50., 51.] {
                r.simulation_revision = revision.into(); let before = r.clone();
                for copy in [describe(&r, &a).0, describe(&r, &a).1] {
                    assert_eq!(copy.contains("immune to fire damage"), revision >= 51., "{kind}: {copy}");
                }
                assert_eq!(r, before);
            }
            let entry = manual.catalog.entries.iter().find(|e| e.id == kind).unwrap();
            assert!(entry.mechanics.iter().any(|m| m.name == "Fire Immunity"
                && m.text == "Immune to fire and burning."));
        }
    }
}

#[cfg(test)]
mod revision52_fast_copy_tests {
    use super::*;
    #[test]
    fn observed_fast_trait_explains_move_then_attack_and_preserves_legacy_copy() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let a=corpse_core::population::create_actor_revision("cave-bat",800.,10.,10.,false,52.).unwrap();
        for revision in [51.,52.] {r.simulation_revision=revision.into();let before=r.clone();let (short,long)=describe(&r,&a);for text in [short,long] {assert_eq!(text.contains("move once then attack"),revision>=52.);}assert_eq!(r,before);}
    }
}

#[cfg(test)] mod floor_future_description_tests {
    use super::*;
    #[test] fn floor_rewind_player_description_has_current_state_without_energy_tutorial() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();r.future_floor_rules_from=0usize.into();
        r.player.abilities=vec![corpse_core::future_floor::ID.into()];r.player.active.clear();
        let before=r.clone();let (compact,detail)=describe(&r,&r.player);
        let both=format!("{compact} {detail}");
        assert!(both.contains(crate::ability_ui::FUTURE_FLOOR_DESCRIPTION));assert!(both.contains("(active)"));
        assert!(!both.contains("reserve"));assert!(!both.contains("Spends all"));assert_eq!(r,before);
        r.future_floor=Some(corpse_core::future_floor::State{depth:r.depth,anchor:None,used_floors:vec![r.depth]});
        let (compact,detail)=describe(&r,&r.player);assert!(format!("{compact} {detail}").contains("(used)"));
    }
}
