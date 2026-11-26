//! Read-only v48 actor statuses and local condition-meter calibration.
//! Call `observe` only for the player or actors already admitted by perception.
use corpse_core::{
    ai, combat, crafting, engine::condition, half_here, progression, travel, vision, Actor, Run,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Status {
    pub name: String,
    pub duration: Option<f64>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Meter {
    pub name: String,
    pub duration: Option<f64>,
    pub reference: f64,
    pub reference_known: bool,
    pub percent: f64,
}
#[derive(Clone, Copy)]
struct Reference {
    duration: f64,
    known: bool,
}
/// Unlike JS objects, native actor addresses change during COW dispatch. The
/// owner resets this view-only history on load/restart/temporal replacement.
#[derive(Default)]
pub struct ConditionMeters {
    observations: HashMap<u64, HashMap<String, Reference>>,
}
impl ConditionMeters {
    pub fn reset(&mut self) {
        self.observations.clear();
    }
    /// O(S) expected work/storage per observed actor's S status labels. Empty
    /// observations are retained so a later new application has a known start.
    pub fn measure(&mut self, actor_id: f64, statuses: &[Status]) -> Vec<Meter> {
        let key = if actor_id == 0. {
            0
        } else {
            actor_id.to_bits()
        };
        let old = self.observations.get(&key);
        let observed_before = old.is_some();
        let mut next = HashMap::with_capacity(statuses.len());
        let result = statuses
            .iter()
            .map(|status| {
                let prior = old.and_then(|m| m.get(&status.name));
                let reference = status
                    .duration
                    .map_or(1., |d| d.max(prior.map_or(0., |p| p.duration)).max(1.));
                let known = status.duration.is_none()
                    || prior.map_or(observed_before, |p| {
                        p.known || status.duration.unwrap() > p.duration
                    });
                next.insert(
                    status.name.clone(),
                    Reference {
                        duration: reference,
                        known,
                    },
                );
                Meter {
                    name: status.name.clone(),
                    duration: status.duration,
                    reference,
                    reference_known: known,
                    percent: status
                        .duration
                        .map_or(100., |d| (100. * d / reference).clamp(0., 100.)),
                }
            })
            .collect();
        self.observations.insert(key, next);
        result
    }
}
pub fn hover_description(revision: f64, status: &Status) -> String {
    if revision >= 52. && status.name == "Fast" { return "Fast: can move two tiles per action, or move once then attack. Slowed suppresses the extra movement.".into(); }
    if revision >= 52. && status.name == "Haste" { return format!("Haste: takes two complete actions per world turn. Slowed suppresses Haste.{}",
        status.duration.map_or_else(String::new,|n|format!(" {} turns remaining.", crate::stat_presentation::whole_number(n)))); }
    format!(
        "{} {}.",
        status.name,
        status.duration.map_or_else(
            || "ongoing".into(),
            |d| format!(
                "{} {} remaining",
                crate::stat_presentation::whole_number(d),
                if d == 1. { "turn" } else { "turns" }
            )
        )
    )
}
/// Compact only authored R50 meter labels; observation and hover keep full text.
pub fn compact_meter_name(name: &str) -> Option<&'static str> {
    match name {
        "Discord: hostile to all" => Some("Discord"),
        "Rooted" => Some("Rooted"),
        "Corroded" => Some("Corroded"),
        "Fire Immunity" => Some("Fire immune"),
        _ => None,
    }
}
/// Short on-screen condition names, without explanatory prose. Remembered
/// ability names and named scream effects retain their distinguishing suffixes.
pub fn display_name(name:&str)->&str {
    match name {
        "Slowed: acts every other turn"=>"Slowed",
        "Slow: acts every other turn"=>"Slow",
        "Lucky: next attack"=>"Lucky",
        "Unlucky: next attack"=>"Unlucky",
        "Distracted: next attack"=>"Distracted",
        "Stuck: next move escapes"=>"Stuck",
        "Charging moonbeam: next normal action"=>"Charging moonbeam",
        "Riptide: rooted"=>"Riptide",
        "Discord: hostile to all"=>"Discord",
        "Squirming helplessly: outside habitat"=>"Squirming helplessly",
        _ if name.ends_with(" · active")=>name.strip_suffix(" · active").unwrap(),
        _ if name.ends_with(" · linked to you")=>name.strip_suffix(" · linked to you").unwrap(),
        _ if name.starts_with("Envenomed:")=>"Envenomed",
        _=>compact_meter_name(name).unwrap_or(name),
    }
}

/// One readable condition run; callers wrap the text to their available width.
/// O(T) output for T displayed characters. Duration values stay visible.
pub fn summary(statuses: &[Status]) -> String {
    statuses.iter().map(|s| {
        let name = display_name(&s.name);
        s.duration.map_or_else(|| name.to_string(), |n| format!("{name} {}", crate::stat_presentation::whole_number(n)))
    }).collect::<Vec<_>>().join(", ")
}

fn named(actor: &Actor, name: &str, active: bool) -> bool {
    let ids = if active {
        &actor.active
    } else {
        &actor.abilities
    };
    progression::catalog()
        .iter()
        .any(|a| a.name == name && ids.contains(&a.id))
}
fn has(actor: &Actor, id: &str) -> bool {
    actor.abilities.iter().any(|a| a == id)
}
fn dream_craft(run: &Run) -> bool {
    run.inventory
        .iter()
        .filter(|i| corpse_core::revision50_inventory::active(run,i))
        .any(|i| {
            i.enhancements
                .iter()
                .any(|e| crafting::craft_name(&e.craft) == Some("Dream"))
        })
}
/// Exact scheduler projection. Shadow haste checks recorded sight plus current
/// deterministic detection, not pursuit state alone. O(N*D+A), N observers and
/// D core detection-query cost (light/stat/LOS); no random draws or mutations.
pub fn has_ongoing_haste(run: &Run, actor: &Actor) -> bool {
    corpse_core::revision52_swimming::grants_haste(run, actor)
        || has_non_water_haste(run, actor)
}
/// Fixed Haste sources for knowledge-filtered route projections. Swimming is
/// evaluated separately on the projected departure tile, not hidden live water.
pub(crate) fn has_non_water_haste(run: &Run, actor: &Actor) -> bool {
    (run.simulation_revision.value_or(29.) < 52. && corpse_core::ai::is_fast(actor))
        || named(actor, "Rushing Torrent", true)
        || (named(actor, "Sleep Walk", false) && actor.energy == 0.)
        || (named(actor, "Step Through Shadows", false)
            && !run.actors_at(run.depth).iter().any(|other| {
                !other.dead.value_or(false)
                    && other.hp > 0.
                    && other.state != "ally"
                    && other.state != "neutral"
                    && condition(other, "seesPlayer") == 1.
                    && vision::detection_chance(run, other, actor) > 0.
            }))
}
pub fn is_hasted(run: &Run, actor: &Actor) -> bool {
    !(condition(actor, "slowed") > 0.)
        && (condition(actor, "hasted") > 0. || has_ongoing_haste(run, actor))
}
#[derive(Deserialize)]
struct TraitDefinition {
    id: String,
    name: String,
    slow: bool,
}
fn trait_definitions() -> &'static [TraitDefinition] {
    static DATA: std::sync::OnceLock<Vec<TraitDefinition>> = std::sync::OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/status-traits-v48.json"))
            .expect("frozen status trait metadata")
    })
}
fn slow_trait(actor: &Actor) -> bool {
    if has(actor, "enemy:pale:slow") || has(actor, "enemy:beast:slow") {
        return true;
    }
    if condition(actor, "paleTraitsInitialized") != 0.
        || condition(actor, "bestiaryTraitsInitialized") != 0.
    {
        return false;
    }
    let original = actor.creature_kind.as_ref().unwrap_or(&actor.name);
    let original = original
        .strip_prefix("Mercuria’s ")
        .or_else(|| original.strip_prefix("Mercuria's "))
        .unwrap_or(original);
    let name = if matches!(original, "vent-fungus" | "Vent Fungus") {
        "bastidion"
    } else {
        original
    };
    if let Some(d) = trait_definitions()
        .iter()
        .find(|d| d.id == name || d.name == name)
    {
        return d.slow;
    }
    // JS's /\bPale\b/ uses ASCII word boundaries, including underscore.
    let pale = actor.name.match_indices("Pale").any(|(i, _)| {
        let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
        (i == 0 || !word(actor.name.as_bytes()[i - 1]))
            && (i + 4 == actor.name.len() || !word(actor.name.as_bytes()[i + 4]))
    });
    pale && actor.name.ends_with("Birther")
}
const LINKS: [(&str, &str, &str, Option<&str>); 5] = [
    (
        "morae-bonded-by-unseen-threads",
        "Bonded by Unseen Threads",
        "bondedTarget",
        None,
    ),
    (
        "pelian-i-came-for-you",
        "I Came For You",
        "cameTarget",
        Some("cameTurns"),
    ),
    (
        "isheilah-make-slippery",
        "Make Slippery",
        "slipperyTarget",
        None,
    ),
    ("etill-riptide", "Riptide", "riptideTarget", None),
    ("dawel-unspeak", "Unspeak", "unspeakTarget", None),
];
/// Exact ordered status projection. O(A + C + I*E + N*D) including catalog
/// lookups, active equipment and Shadow haste detection; retained output O(S).
/// Unknown internal condition keys are deliberately never exposed.
pub fn observe(run: &Run, actor: &Actor) -> Vec<Status> {
    let mut out = vec![];
    let player = actor.id == run.player.id;
    let mut push = |name: &str, duration: Option<f64>| {
        out.push(Status {
            name: name.into(),
            duration,
        })
    };
    if player && corpse_core::dreamwalking::active(run) {
        push("Dreamwalking", None);
        push("Flying", None);
    }
    if run.status == "playing" && !run.player.dead.value_or(false) {
        for (id, name, target, timer) in LINKS {
            let duration = timer.map(|key| condition(&run.player, key));
            if duration.map_or(!run.player.active.iter().any(|a| a == id), |d| d <= 0.) {
                continue;
            }
            if player {
                push(
                    &format!(
                        "{name}{}",
                        if duration.is_none() { " · active" } else { "" }
                    ),
                    duration,
                );
            } else if !actor.dead.value_or(false)
                && actor.hp > 0.
                && run.player.conditions.get(target) == Some(&actor.id)
                && half_here::owner_id(run, actor) != run.player.id
                && run
                    .actors_at(run.depth)
                    .iter()
                    .any(|a| a.id == actor.id && !a.dead.value_or(false) && a.hp > 0.)
            {
                push(&format!("{name} · linked to you"), duration);
            }
        }
    }
    if !actor.dead.value_or(false) && (ai::sleeping(actor) || corpse_core::dreamwalking::is_body(actor)) {
        push("Sleeping", None);
    }
    if corpse_core::dreamwalking::floor_for(run, actor)
        .is_some_and(|f| !travel::habitat_valid(f, actor, actor.x, actor.y))
    {
        push("Squirming helplessly: outside habitat", None);
    }
    const TIMED: [(&str, &str); 22] = [
        ("paralyzed", "Paralyzed"),
        ("telepathic", "Telepathy"),
        ("invisible", "Invisible"),
        ("confused", "Confused"),
        ("magicSensitive", "Magic Sensitive"),
        ("bleeding", "Bleeding"),
        ("hemophilia", "Hemophilia"),
        ("poisoned", "Poisoned"),
        ("slowed", "Slowed: acts every other turn"),
        ("envenomed", "Envenomed"),
        ("grabbed", "Held"),
        ("weakened", "Weakened"),
        ("emboldened", "Emboldened"),
        ("screamWeakened", "Scream: weakened"),
        ("disoriented", "Disoriented"),
        ("hallucinating", "Hallucinating"),
        ("screamEmboldened", "Scream: emboldened"),
        ("gasVulnerableTicks", "Gas vulnerability"),
        ("temporaryAlly", "Temporary ally"),
        ("cameTurns", "I Came For You"),
        ("rangedRecovery", "Ranged recovery"),
        ("beastCooldown", "Special-action cooldown"),
    ];
    for (key, name) in TIMED {
        let value = if key == "bleeding" && corpse_core::revision52_enemy_followups::enabled(run) {
            condition(actor,key).max(condition(actor,corpse_core::revision52_enemy_followups::BLEED))
        } else { condition(actor, key) };
        if value > 0. && !(key == "cameTurns" && player) {
            push(name, Some(value));
        }
    }
    if run.simulation_revision.value_or(29.)>=50. {
        for (key,label) in [("r50Discord","Discord: hostile to all"),("r50Roots","Rooted"),("r50AcidTurns","Corroded"),("r50FireImmunity","Fire Immunity")] {
            let remaining=condition(actor,key);
            if remaining>0. {push(label,Some(remaining));}
        }
    }
    let passive_burning = corpse_core::star_power::burning(run, actor);
    let burning = condition(actor, "burning");
    if burning > 0. || passive_burning {
        push(
            "Burning",
            if passive_burning
                || (!actor.dead.value_or(false) && has(actor, "enemy:beast:everburn"))
            {
                None
            } else {
                Some(burning)
            },
        );
    }
    if travel::flying(run, actor) && !(player && corpse_core::dreamwalking::active(run)) {
        let ongoing = has(actor, "enemy:beast:flight")
            || named(actor, "Ascend", false)
            || named(actor, "Phase Through", true)
            || (player && dream_craft(run))
            || condition(actor, "flying") <= 0.;
        push(
            "Flying",
            if ongoing {
                None
            } else {
                Some(condition(actor, "flying"))
            },
        );
    }
    if run.simulation_revision.value_or(29.) >= 52. && corpse_core::ai::is_fast(actor) {
        push("Fast", None);
    }
    if is_hasted(run, actor) {
        push(
            "Haste",
            if has_ongoing_haste(run, actor) {
                None
            } else {
                Some(condition(actor, "hasted"))
            },
        );
    }
    if slow_trait(actor) {
        push("Slow: acts every other turn", None);
    }
    for (key, label) in [
        ("lucky", "Lucky: next attack"),
        ("unlucky", "Unlucky: next attack"),
        ("distracted", "Distracted: next attack"),
        ("silenced", "Silenced"),
        ("auraSilenced", "Silenced by aura"),
        ("underwater", "Underwater"),
        ("stuck", "Stuck: next move escapes"),
        ("laserCharge", "Charging moonbeam: next normal action"),
        ("riptide", "Riptide: rooted"),
    ] {
        if condition(actor, key) > 0. {
            push(label, None);
        }
    }
    if condition(actor, "envenomed") > 0. {
        if let Some(status) = out.iter_mut().find(|s| s.name == "Envenomed") {
            if run.simulation_revision.value_or(29.0) >= 52.0 {
                let severity: f64 = actor.conditions.iter().filter(|(key, _)| key.starts_with("venom:"))
                    .map(|(_, value)| value.max(0.0)).sum();
                status.name = format!("Envenomed {}", crate::stat_presentation::whole_number(severity));
            } else {
                let potency = actor.conditions.iter().filter(|(key, _)| key.starts_with("venom:"))
                    .map(|(_, value)| combat::damage_packet(run, actor, *value, "venom", false)).fold(0.0, |sum, n| sum + n);
                status.name = format!("Envenomed: {} damage/turn", crate::stat_presentation::whole_number(potency));
            }
        }
    }
    if has(actor, "enemy:beast:rooted") {
        out.push(Status {
            name: "Rooted".into(),
            duration: None,
        });
    }
    if has(actor, "enemy:beast:slime-bound") {
        out.push(Status {
            name: "Slime-bound".into(),
            duration: None,
        });
    }
    for (key, value) in &actor.conditions {
        if let Some(stat) = key.strip_prefix("buff:") {
            let n = combat::integer_stat(*value);
            if n != 0. {
                let ticks = condition(actor, &format!("buffTicks:{stat}"));
                out.push(Status {
                    name: format!("{stat} {}{n}", if *value > 0. { "+" } else { "" }),
                    duration: (ticks > 0.).then_some(ticks),
                });
            }
        }
        if let Some(id) = key.strip_prefix("remembered:").filter(|_| *value > 0.) {
            let name = progression::definition_for_revision(run.simulation_revision.value_or(29.), id)
                .map_or(id, |a| a.name.as_str());
            out.push(Status {
                name: format!("Remember: {name}"),
                duration: Some((*value - run.tick).max(0.)),
            });
        }
    }
    // O(S) bounded label projection, O(L) output for each changed label. Historical
    // observation fixtures retain their original units; simulation is never changed.
    if run.simulation_revision.value_or(29.) >= 50. {
        let player_units=half_here::owner_id(run,actor)==run.player.id
            || condition(actor,"miniMe")!=0.;
        let damage_factor=if player_units {1.} else {
            actor.conditions.get("weaponOffenseMultiplier").copied().unwrap_or(1.)*0.5
        };
        for status in &mut out {
            if let Some((key,n))=status.name.split_once(' ') {
                if let Ok(n)=n.parse::<f64>() {
                    let converted=match key {
                        "offense"=>Some(("Damage",crate::stat_presentation::damage_bonus(run,n*damage_factor))),
                        "defense"=>Some(("HP",n*6.)),
                        _=>None,
                    };
                    if let Some((label,value))=converted {
                        status.name=format!("{label} {value:+}");
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use corpse_core::Field;
    fn fixture() -> Run {
        serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap()
    }
    #[test]
    fn revision52_swimming_haste_is_ongoing_read_only_and_matches_scheduler() {
        let mut r=fixture();r.simulation_revision=52.0.into();
        r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
        r.inventory=vec![corpse_core::revision52_swimming::make_ring(900.)];
        let at=r.player.y as usize*r.floor().unwrap().width()+r.player.x as usize;
        for tile in ["floor","shallow-water","water"] {
            r.floor_mut().unwrap().tiles[at]=tile.into();r.floor_mut().unwrap().ichor=Field::Missing;
            for state in ["ordinary","slowed","flying"] {
                r.player.conditions.clear();
                if state!="ordinary" {r.player.conditions.insert(state.into(),3.);}
                let before=serde_json::to_value(&r).unwrap();
                let actual=observe(&r,&r.player);let hasted=tile!="floor"&&state=="ordinary";
                assert_eq!(is_hasted(&r,&r.player),hasted);
                assert_eq!(is_hasted(&r,&r.player),corpse_core::ai::is_hasted_in(&r,&r.player));
                assert_eq!(actual.iter().find(|s|s.name=="Haste"),hasted.then_some(&Status{name:"Haste".into(),duration:None}));
                assert_eq!(serde_json::to_value(&r).unwrap(),before);
            }
        }
    }
    #[test]
    fn revision52_bleeding_meter_merges_wounds_without_exposing_keys() {
        let mut r=fixture();r.simulation_revision=52.0.into();
        for player in [false,true] {
            let mut a=r.player.clone();a.id=if player {r.player.id}else{800.};a.conditions.clear();a.abilities.clear();a.active.clear();
            a.conditions.insert("bleeding".into(),2.);a.conditions.insert("killspawnBleeding".into(),3.);
            a.conditions.insert("killspawnBleed:3:801".into(),10.);
            let statuses=observe(&r,&a);let bleeding:Vec<_>=statuses.iter().filter(|s|s.name=="Bleeding").collect();
            assert_eq!(bleeding.len(),1);assert_eq!(bleeding[0].duration,Some(3.));
            assert!(statuses.iter().all(|s|!s.name.contains("killspawn")&&!s.name.contains("801")));
        }
    }
    #[test]
    fn source_185_observed_status_and_scheduler_cases() {
        let data: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/status-ui-v48.json")).unwrap();
        assert_eq!(data["cases"].as_array().unwrap().len(), 185);
        for case in data["cases"].as_array().unwrap() {
            let options = &case["options"];
            let mut raw = serde_json::to_value(fixture()).unwrap();
            raw["inventory"] = serde_json::json!([]);
            let depth = raw["depth"].as_f64().unwrap();
            raw["actors"][depth.to_string()] = serde_json::json!([]);
            if let Some(changes) = options["run"].as_object() {
                for (k, v) in changes {
                    raw[k] = v.clone();
                }
            }
            if let Some(changes) = options["playerChanges"].as_object() {
                for (k, v) in changes {
                    raw["player"][k] = v.clone();
                }
            }
            let player = options["player"] == true;
            if player {
                raw["player"] = case["actor"].clone();
            } else if options["absent"] != true {
                raw["actors"][depth.to_string()]
                    .as_array_mut()
                    .unwrap()
                    .push(case["actor"].clone());
            }
            if let Some(observers) = options["observers"].as_array() {
                raw["actors"][depth.to_string()]
                    .as_array_mut()
                    .unwrap()
                    .extend(observers.iter().cloned());
            }
            let mut run: Run = serde_json::from_value(raw).unwrap();
            let a: Actor = serde_json::from_value(case["actor"].clone()).unwrap();
            let f = run.floor_mut().unwrap();
            let at = a.y as usize * f.width() + a.x as usize;
            if let Some(tile) = options["tile"].as_str() {
                f.tiles[at] = tile.into();
            }
            let len = f.tiles.len();
            if let Some(n) = options["ichor"].as_f64() {
                f.ichor.get_or_insert_with(|| vec![0.; len])[at] = n;
            }
            if let Some(n) = options["slime"].as_f64() {
                f.slime.get_or_insert_with(|| vec![0.; len])[at] = n;
            }
            let before = run.clone();
            let actual = observe(&run, if player { &run.player } else { &a });
            assert!(
                corpse_core::storage::canonical_equal(
                    &serde_json::to_value(&actual).unwrap(),
                    &case["statuses"]
                ),
                "{}\nactual: {:?}\nexpected: {}",
                case["name"],
                actual,
                case["statuses"]
            );
            assert_eq!(
                is_hasted(&run, if player { &run.player } else { &a }),
                case["haste"].as_bool().unwrap(),
                "{} haste",
                case["name"]
            );
            assert_eq!(
                has_ongoing_haste(&run, if player { &run.player } else { &a }),
                case["ongoing"].as_bool().unwrap(),
                "{} ongoing",
                case["name"]
            );
            let hovers: Vec<_> = actual.iter().map(|s| hover_description(run.simulation_revision.value_or(29.), s)).collect();
            assert_eq!(
                serde_json::to_value(hovers).unwrap(),
                serde_json::to_value(case["hover"].as_array().unwrap().iter().map(|v|{
                    let s=v.as_str().unwrap();s.rsplit_once(" · ").map_or_else(||s.to_owned(),|(a,b)|format!("{a} {b}"))
                }).collect::<Vec<_>>()).unwrap(),
                "{} hover",
                case["name"]
            );
            assert_eq!(run, before, "{} mutates observation", case["name"]);
        }
    }
    #[test]
    fn source_meter_baselines_unknown_expiration_reapplication() {
        let data: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/status-ui-v48.json")).unwrap();
        for case in data["meterCases"].as_array().unwrap() {
            let mut meters = ConditionMeters::default();
            for step in case["rows"].as_array().unwrap() {
                let statuses: Vec<Status> =
                    serde_json::from_value(step["statuses"].clone()).unwrap();
                let actual=meters.measure(1.,&statuses).iter().map(|m|serde_json::json!({"name":m.name,"duration":m.duration,"reference":m.reference,"referenceKnown":m.reference_known,"percent":m.percent})).collect::<Vec<_>>();
                assert!(corpse_core::storage::canonical_equal(
                    &serde_json::json!(actual),
                    &step["expected"]
                ));
            }
        }
    }
    #[test]
    fn new_actor_or_replaced_session_does_not_inherit_meter_certainty() {
        let mut meters = ConditionMeters::default();
        meters.measure(1., &[]);
        let status = [Status {
            name: "Paralyzed".into(),
            duration: Some(4.),
        }];
        assert!(meters.measure(1., &status)[0].reference_known);
        assert!(!meters.measure(2., &status)[0].reference_known);
        meters.reset();
        assert!(!meters.measure(1., &status)[0].reference_known);
    }
    #[test]
    fn native_disoriented_and_hallucinating_labels_expire_without_internal_penalty_labels() {
        let mut r = fixture();
        r.simulation_revision = 33.0.into();
        r.player.conditions.clear(); r.player.abilities.clear(); r.player.active.clear();
        r.player.conditions.insert("disoriented".into(), 5.);
        r.player.conditions.insert("disorientedPenalty".into(), 5.);
        r.player.conditions.insert("hallucinating".into(), 2.);
        let before = r.clone();
        let labels = observe(&r, &r.player);
        assert!(labels.iter().any(|s| s.name == "Disoriented" && s.duration == Some(5.)));
        assert!(labels.iter().any(|s| s.name == "Hallucinating" && s.duration == Some(2.)));
        assert!(!labels.iter().any(|s| s.name.contains("weaken") || s.name.contains("Penalty")));
        assert_eq!(r, before);
        r.player.conditions.insert("disoriented".into(), 0.);
        r.player.conditions.insert("hallucinating".into(), 0.);
        assert!(observe(&r, &r.player).iter().all(|s| !["Disoriented", "Hallucinating"].contains(&s.name.as_str())));
    }
    #[test]
    fn unknown_internal_keys_never_become_conditions() {
        let mut r = fixture();
        r.player.conditions.clear();
        r.player.abilities.clear();
        r.player.active.clear();
        r.player.dead = Field::Missing;
        for key in [
            "targetId",
            "hastedFresh",
            "poisonSourceId",
            "halfHereOwner",
            "corpseClock",
            "surrenderTick",
            "bloodWeaponUsed",
        ] {
            r.player.conditions.insert(key.into(), 100.);
        }
        assert!(observe(&r, &r.player).is_empty());
    }
}

#[cfg(test)] mod revision50_amulet_status_tests {
    use super::*;
    #[test] fn revision50_fire_immunity_turns_exclude_fresh_metadata() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=50.0.into();r.player.conditions.clear();r.player.abilities.clear();r.player.active.clear();
        r.player.conditions.insert("r50FireImmunityFresh".into(),1.);
        assert!(observe(&r,&r.player).is_empty());
        for turns in [20.,10.] {
            r.player.conditions.insert("r50FireImmunity".into(),turns);
            let before=serde_json::to_vec(&r).unwrap();let statuses=observe(&r,&r.player);
            assert_eq!(statuses,vec![Status{name:"Fire Immunity".into(),duration:Some(turns)}]);
            assert_eq!(hover_description(r.simulation_revision.value_or(29.), &statuses[0]),format!("Fire Immunity {turns} turns remaining."));
            assert_eq!(serde_json::to_vec(&r).unwrap(),before);
        }
        r.player.conditions.insert("r50FireImmunity".into(),0.);assert!(observe(&r,&r.player).is_empty());
        r.player.conditions.insert("r50FireImmunity".into(),20.);r.simulation_revision=49.0.into();
        assert!(observe(&r,&r.player).is_empty());
    }
    #[test] fn compact_discord_retains_full_description_data() {
        let status=Status{name:"Discord: hostile to all".into(),duration:Some(6.)};
        assert_eq!(compact_meter_name(&status.name),Some("Discord"));
        assert_eq!(hover_description(50., &status),"Discord: hostile to all 6 turns remaining.");
        assert_eq!(compact_meter_name("Paralyzed"),None);
    }
    #[test] fn named_amulet_statuses_expose_only_names_and_remaining_turns() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=50.0.into();r.player.conditions.clear();r.player.abilities.clear();r.player.active.clear();
        for (key,n) in [("r50Discord",6.),("r50Roots",4.),("r50AcidTurns",5.),("r50RootsSource",999.),("r50RootsX",10.),("r50AcidPenalty",10.)] {
            r.player.conditions.insert(key.into(),n);
        }
        let before=serde_json::to_vec(&r).unwrap();let statuses=observe(&r,&r.player);
        for (name,turns) in [("Discord: hostile to all",6.),("Rooted",4.),("Corroded",5.)] {
            assert!(statuses.iter().any(|s|s.name==name && s.duration==Some(turns)));
        }
        assert!(!statuses.iter().any(|s|s.name.contains("r50")||s.name.contains("999")));
        assert_eq!(serde_json::to_vec(&r).unwrap(),before);
        r.simulation_revision=46.0.into();
        assert!(!observe(&r,&r.player).iter().any(|s|matches!(s.name.as_str(),"Discord: hostile to all"|"Rooted"|"Corroded")));
    }
}

#[cfg(test)] mod carried_jewelry_status_tests {
    use super::*;
    #[test] fn r50_carried_dream_craft_projection_ignores_historical_flags() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.inventory=vec![corpse_core::Item{kind:"ring".into(),enhancements:vec![corpse_core::ItemEnhancementsEntry{craft:"Dream".into(),..Default::default()}],..Default::default()}];
        r.simulation_revision=50.0.into();
        for flag in [corpse_core::Field::Missing,corpse_core::Field::Null,"".to_string().into(),"ring1".to_string().into()] {
            r.inventory[0].equipped=flag;let before=r.clone();assert!(dream_craft(&r));assert_eq!(r,before);
        }
        r.simulation_revision=46.0.into();r.inventory[0].equipped=corpse_core::Field::Missing;assert!(!dream_craft(&r));
        r.inventory[0].equipped="ring1".to_string().into();assert!(dream_craft(&r));
    }
}

#[cfg(test)]
mod revision51_display_unit_tests {
    use super::*;
    #[test]
    fn current_status_units_are_contextual_and_legacy_observations_unchanged() {
        let mut run:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        run.player.conditions.insert("buff:defense".into(),3.);
        run.player.conditions.insert("buffTicks:defense".into(),7.);
        run.player.conditions.insert("buff:offense".into(),5.);
        run.simulation_revision=48.0.into();
        let legacy=observe(&run,&run.player);
        assert!(legacy.iter().any(|s|s.name=="defense +3"));
        assert!(legacy.iter().any(|s|s.name=="offense +5"));
        run.simulation_revision=50.0.into();
        let before=run.clone();
        let player=observe(&run,&run.player);
        let hp=player.iter().find(|s|s.name=="HP +18").unwrap();
        assert_eq!(hover_description(run.simulation_revision.value_or(29.), hp),"HP +18 7 turns remaining.");
        assert!(player.iter().any(|s|s.name=="Damage +5"));
        let mut enemy=run.player.clone();enemy.id=98765.;
        enemy.conditions.insert("buff:defense".into(),-2.);
        enemy.conditions.shift_remove("weaponOffenseMultiplier");
        let shown=observe(&run,&enemy);
        assert!(shown.iter().any(|s|s.name=="HP -12"));
        assert!(shown.iter().any(|s|s.name=="Damage +2.5"));
        enemy.conditions.insert("weaponOffenseMultiplier".into(),3.);
        assert!(observe(&run,&enemy).iter().any(|s|s.name=="Damage +7.5"));
        assert_eq!(display_name("Slowed: acts every other turn"),"Slowed");
        assert_eq!(run,before);
        run.simulation_revision=51.0.into();
        assert!(observe(&run,&enemy).iter().any(|s|s.name=="Damage +8"));
        enemy.conditions.insert("buff:offense".into(),-5.);
        assert!(observe(&run,&enemy).iter().any(|s|s.name=="Damage -8"));
    }
}

#[cfg(test)]
mod revision51_venom_tests {
    use super::*;
    #[test]
    fn revision51_venom_preview_sums_each_rounded_resisted_packet() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=51.0.into();r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.inventory.clear();
        r.inventory.push(corpse_core::Item{id:800.,name:"Ring of Poison Resistance".into(),kind:"ring".into(),effect:corpse_core::revision50_rings::POISON_RESISTANCE.to_string().into(),..Default::default()});
        r.player.conditions.insert("envenomed".into(),2.);r.player.conditions.insert("venom:1".into(),2.9);r.player.conditions.insert("venom:2".into(),2.9);
        let before=r.clone();assert!(observe(&r,&r.player).iter().any(|s|s.name=="Envenomed: 2 damage/turn"));assert_eq!(r,before);
    }
}

#[cfg(test)]
mod revision52_speed_tests {
    use super::*;
    #[test]
    fn speed_hover_copy_is_revision_gated() {
        for revision in [29., 48., 51.] {
            for (duration, expected) in [
                (None, "Haste ongoing."),
                (Some(1.), "Haste 1 turn remaining."),
                (Some(5.), "Haste 5 turns remaining."),
            ] {
                assert_eq!(hover_description(revision, &Status { name: "Haste".into(), duration }), expected);
            }
            assert_eq!(hover_description(revision, &Status { name: "Fast".into(), duration: None }), "Fast ongoing.");
        }
        assert_eq!(hover_description(52., &Status { name: "Haste".into(), duration: None }),
            "Haste: takes two complete actions per world turn. Slowed suppresses Haste.");
        assert_eq!(hover_description(52., &Status { name: "Haste".into(), duration: Some(5.) }),
            "Haste: takes two complete actions per world turn. Slowed suppresses Haste. 5 turns remaining.");
        assert_eq!(hover_description(52., &Status { name: "Fast".into(), duration: None }),
            "Fast: can move two tiles per action, or move once then attack. Slowed suppresses the extra movement.");
    }
    #[test]
    fn fast_is_distinct_from_haste_without_mutating_observed_actor() {
        for revision in [51.,52.] {let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=revision.into();let mut a=corpse_core::population::create_actor_revision("cave-bat",800.,10.,10.,false,revision).unwrap();
            for hasted in [false,true] {a.conditions.insert("hasted".into(),if hasted{5.}else{0.});let before=r.clone();let old=a.clone();let statuses=observe(&r,&a);assert_eq!(statuses.iter().any(|s|s.name=="Fast"),revision>=52.);assert_eq!(statuses.iter().any(|s|s.name=="Haste"),revision<52.||hasted);assert_eq!(r,before);assert_eq!(a,old);}
        }
        assert!(hover_description(52., &Status{name:"Fast".into(),duration:None}).contains("then attack"));
        assert!(hover_description(52., &Status{name:"Haste".into(),duration:Some(5.)}).contains("two complete actions"));
    }
}

#[cfg(test)] mod compact_duration_format_tests {
    use super::*;
    #[test] fn plain_numbers_preserve_status_duration_and_severity(){
        let statuses=vec![Status{name:"Burning".into(),duration:Some(1.)},Status{name:"Envenomed 2".into(),duration:Some(7.)},Status{name:"Flying".into(),duration:None}];
        assert_eq!(summary(&statuses),"Burning 1, Envenomed 2 7, Flying");
        assert_eq!(hover_description(52.,&statuses[0]),"Burning 1 turn remaining.");
    }
}

#[cfg(test)]
mod revision52_venom_tests {
    use super::*;
    #[test]
    fn severity_label_preserves_shared_duration_and_does_not_apply_resistance() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();r.player.conditions.clear();r.player.conditions.insert("envenomed".into(),5.);
        r.player.conditions.insert("venom:20:7".into(),2.);
        r.inventory.push(corpse_core::Item{id:800.,name:"Ring of Poison Resistance".into(),kind:"ring".into(),effect:corpse_core::revision50_rings::POISON_RESISTANCE.to_string().into(),..Default::default()});
        let before=r.clone();
        assert!(observe(&r,&r.player).iter().any(|s|s.name=="Envenomed 2" && s.duration==Some(5.)));
        assert_eq!(r,before);
        r.player.conditions.insert("venom:20:7".into(),1.);
        assert!(observe(&r,&r.player).iter().any(|s|s.name=="Envenomed 1"));
    }
}

#[cfg(test)] mod integer_status_presentation_tests {
    #[test] fn fractional_duration_is_displayed_as_a_whole_turn_without_mutation() {
        let status=super::Status{name:"Burning".into(),duration:Some(2.6)};
        assert_eq!(super::summary(&[status.clone()]),"Burning 3");
        assert_eq!(super::hover_description(52.,&status),"Burning 3 turns remaining.");
        assert_eq!(status.duration,Some(2.6));
    }
}
