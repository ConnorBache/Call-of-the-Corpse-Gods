//! Read-only source ability-presentation / ui-ability-list projection.
use crate::render::Rgb;
use corpse_core::{model::Run, progression};
pub const TOP: i32 = 182;
pub const ROW: i32 = 35;
pub const CAPACITY: usize = 9;
pub fn top(p: &crate::appearance_profile::Profile) -> i32 {
    p.number("ui.layout.resourceRegionHeight").round() as i32 - 56
}
pub fn row(p: &crate::appearance_profile::Profile) -> i32 {
    let action = if p.number("typography.ability.fontSize") == 16. {
        p.number("typography.primary.fontSize")
    } else {
        p.number("typography.ability.fontSize")
    };
    let minimum = (action * p.number("typography.ability.lineHeight"))
        .max(p.number("typography.secondary.fontSize") * p.number("typography.secondary.lineHeight"))
        .ceil() + 6.;
    let previous = p.number("ui.layout.abilityRowHeight").max(50.).max(minimum);
    // A compact reserve meter is 19 logical pixels; halve only the spare gap.
    // Keep the typography minimum so larger authored fonts are never crushed.
    (19. + (previous - 19.) / 2.).max(minimum).round() as i32
}
pub fn capacity(p: &crate::appearance_profile::Profile) -> usize {
    ((634 - top(p)).max(0) / row(p).max(1)).max(1) as usize
}
pub fn god_color(id: usize) -> Rgb {
    const C: [Rgb; 12] = [
        [255, 51, 77],
        [255, 209, 57],
        [184, 119, 255],
        [100, 255, 114],
        [0, 229, 255],
        [114, 160, 255],
        [255, 63, 142],
        [224, 232, 255],
        [189, 255, 69],
        [255, 240, 163],
        [255, 155, 56],
        [243, 108, 255],
    ];
    C.get(id).copied().unwrap_or([255, 248, 232])
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustomIndicator {
    Nightmares { count: u64, until: u64 },
    Reactions(u64),
    Rescue(bool),
    /// Raw cost bits retain equality for HUD damage tracking without truncating energy.
    RagelightCost(u64),
    Future(bool),
    FutureFloors { used: u32 },
    Blood { count: u64, progress: u64 },
    OverflowingSaves(u64),
}
#[derive(Clone)]
#[cfg_attr(test, derive(Debug))]
pub struct AbilityView {
    pub id: String,
    pub name: String,
    pub description: String,
    pub color: Rgb,
    pub cost: String,
    pub state: String,
    pub active: bool,
    pub mode: String,
    pub supported: bool,
    pub reserve: Option<(f64,f64)>,
    pub pending_reserve_loss: f64,
    pub fortunes: Option<(u8,u8)>,
    pub indicator: Option<CustomIndicator>,
}
/// Presentation-only alias: canonical names remain stable for proc matching,
/// catalog identity, replays and saved state. O(text bytes), borrowed if absent.
pub fn display_text(text:&str)->std::borrow::Cow<'_,str> {
    let mut result = std::borrow::Cow::Borrowed(text);
    for (old,new) in [("Rage at the Dying of the Light","Ragelight"),("You Shall Never Be Rid of Me","Never Be Rid of Me"),("See The Future","Futuresight"),("See the Future","Futuresight")] {
        if result.contains(old) { result = std::borrow::Cow::Owned(result.replace(old,new)); }
    }
    result
}
/// O(1) presentation-only override: never changes catalog text used by offer tags.
pub const EXECUTE_DESCRIPTION:&str="When active, if a weapon hit would leave an enemy alive but drop it from at least 25% of its maximum HP to below 25%, spend 10 energy to kill it.";
const LEGACY_EXECUTE_DESCRIPTION:&str="When active, you kill enemies in one hit if they have less than 25% of their HP remaining. Every time you do so, spend 10 energy.";
pub const RAGELIGHT_DESCRIPTION:&str="Whenever you would die, if you can pay the energy cost, you survive with 1 HP. The first rescue is free, the second costs 1 energy, and the cost doubles after each paid rescue. Leveling up resets the cost to 0.";
const LEGACY_RAGELIGHT_DESCRIPTION:&str="When active, whenever you would die, instead be reduced to 1 HP and spend X energy and double the cost. X starts at 0 and goes to 1 on first use, and when you level up, reduce the cost back to 0.";
pub const FUTURE_FLOOR_DESCRIPTION:&str="Whenever you activate this ability or die, you are reset to when you last entered the floor. This can only be used once per floor.";
pub fn display_description<'a>(run: &Run, id: &str, original: &'a str) -> &'a str {
    if id == "etill-riptide" && corpse_core::riptide_rally::current(run) {
        return "Select a visible enemy. While active, pull it toward your current rally point and prevent its voluntary movement. Right-click a map tile to move the rally point and redirect the pull.";
    }
    if id == corpse_core::half_here::ID && corpse_core::half_here::paid_placement(run) {
        return "Spend 5 energy to place a stationary half clone on a visible, unoccupied tile. It shares your health and grants sight from its tile. Activate again at any time to dismiss it for free.";
    }
    if id == corpse_core::half_here::ID && corpse_core::half_here::lock_enabled(run) {
        return "Place a stationary half clone on a visible, unoccupied tile. It shares your health and grants sight from its tile. It must stay there for at least 5 turns before you can dismiss it.";
    }
    if corpse_core::balance_rules::current(run) {
        match id {
            "lichor-overflowing" => return "You have five saves. Each time you would die, spend one and survive at 1 HP. Reaching full HP or leveling up restores all five.",
            "shrail-fight-long" => return "Heal for 3% of actual HP damage dealt by your melee attacks.",
            "ursa-sudden-sleep" => return "Spend 20 energy to fully heal, then become paralyzed for 25 turns.",
            "ursa-dream-of-time-gone-by" => return "Spend 20 energy to set an anchor at your current location. Activate again to return to that location for free.",
            "ursa-sleep-walk" => return "You have Haste below 10% of maximum energy. With no energy cap, Haste still requires zero energy.",
            _ => {}
        }
    }
    if id=="morae-see-the-future" && corpse_core::future_floor::enabled(run) {return FUTURE_FLOOR_DESCRIPTION;}
    if id=="shrail-execute" && run.simulation_revision.value_or(29.)>=52. {
        return if corpse_core::execute_rules::current(run) {EXECUTE_DESCRIPTION} else {LEGACY_EXECUTE_DESCRIPTION};
    }
    if id==corpse_core::divine_rescues::RAGE {
        return if corpse_core::ragelight_rules::current(run) {RAGELIGHT_DESCRIPTION} else {LEGACY_RAGELIGHT_DESCRIPTION};
    }
    if run.simulation_revision.value_or(29.) >= 52. && id == "pelian-never-surrender" {
        return "Increase your DR for the first source of damage every turn.";
    }
    if run.simulation_revision.value_or(29.)>=52. && id=="anavani-illuminate" {
        return "Spend 20 energy and select any visible square. Place a fire and light source that lasts until extinguished. Ordinary water puts it out immediately; Ichor-covered water does not.";
    }
    if run.simulation_revision.value_or(29.) >= 51. {
        match id {
            "pelian-i-came-for-you" => return "Spend 10 energy and select an enemy. You gain telepathy on that enemy and if a move would bring you closer to the enemy, you can take 1 free turn afterward. There is no turn limit.",
            "anavani-illuminate" => return "Spend 20 energy and select any visible square. Place a permanent fire and light source there. The fire and its light go out immediately on contact with water.",
            "morae-all-seeing-eyes" => return "Spend 30 energy to reveal the entire current floor, including enemies and contents, until your next turn.",
            "anavani-shine" => return "You emit light 100 which decreases by 10 every square away from you. You gain additional accuracy equal to the illumination of enemies. When active, you stop emitting this light.",
            _ => {},
        }
    }
    if corpse_core::combat::nearest_damage_enabled(run) {
        match id {
            "lichor-blood-weapon" => return "Activate by paying 10% of your maximum HP. Your next attack is guaranteed to hit and deals double damage.",
            "anavani-nova" => return "You are immune to explosions. When active, when you attack, create an explosion and spend 40 energy. Every explosion deals 50% of each affected creature's maximum HP.",
            _ => {},
        }
    }
    if r50(run) {
        return match id {
            "pelian-never-surrender" => "Spend 8 energy and add one sixth of your HP bonus to your DR the first time you take damage every turn.",
            "lichor-bleed-them" => "Your attacks inflict bleed proportional to your Damage bonus.",
            _ => original,
        };
    }
    // Fixed authored table: no catalog mutation, allocation, or simulation work.
    if run.simulation_revision.value_or(29.) >= 46. {
        match id {
            "anavani-illuminate" => return "Spend 20 energy and select a visible square. Place a permanent fire and light source there.",
            "anavani-star-power" => return "You are immune to fire and always burning. You can activate this ability to stop burning.",
            "kative-adaptive-armor" => return "Eating a creature's heart grants 3 DR against that creature type. The next two hearts of that type grant 2 and 1 DR, for a maximum of +6 DR.",
            "kative-artifice-craft" => return "You can use hearts to Artifice Craft. Multiply the heart's permanent stat bonuses by 5 on the enhanced item while equipped.",
            "morae-all-seeing-eyes" => return "Spend 30 energy to reveal the entire current floor, including enemies and contents, until your next turn.",
            _ => {}
        }
    }
    if run.simulation_revision.value_or(29.) >= 46. && id == "anavani-shine" {
        return "You emit light 100 which decreases by 10 every square away from you. You gain additional accuracy equal to the illumination of enemies. When active, this ability does not illuminate you.";
    }
    if run.simulation_revision.value_or(29.) >= 45. {
        match id {
            "lichor-blood-weapon" => return "Activate by paying 10% of your maximum HP. Your next attack is guaranteed to hit and deals double damage.",
            "grisha-entropy" => return "You are immune to the effects of caustic gas, and have a cloud of caustic gas originating from you. When this is active, caustic gas does not originate from you.",
            "anavani-nova" => return "You are immune to explosions. When active, when you attack, create an explosion and spend 40 energy. Every explosion deals 50% of each affected creature's maximum HP.",
            _ => {}
        }
    }
    if run.simulation_revision.value_or(29.) >= 44. {
        match id {
            "shrail-fight-long" => return "Heal 1/5 the damage you deal in melee.",
            "shrail-beat-to-death" => return "Reduce target dodge by 1 and DR 5 when you hit an enemy until they die.",
            "pelian-bulwark-of-mind" => return "Spend 20 energy and select a visible, unoccupied square. Enemies cannot enter it for 10 turns.",
            "lichor-the-number-of-blood" => return "Every 5 turns, automatically hit and do double damage.",
            "grisha-boundless-hunger" => return "You have no max energy.",
            "dawel-reverse-entropy" => return "While active, enemies do not heal, regain energy, or revive, plants do not grow and neither gas nor fire spreads within 10.",
            "morae-bonded-by-unseen-threads" => return "On activation, select an enemy; when you take damage, they take half.",
            "morae-see-the-future" => return "Spend all your energy to save your current state. Deactivating will return you there, undoing all that has been done.",
            "numo-mind-read" => return "While active, you have telepathy within 20. While inactive, you have telepathy within 5.",
            "etill-float-away" => return "When active, if an enemy damages you, fly until your next turn and move 1 tile away.",
            "ursa-dream-see" => return "You have telepathy with sleeping creatures. When active, you have telepathy with all creatures that are not obstructed.",
            "kative-dilute-potion" => return "When you use a potion, you can use it again at half efficacy.",
            "pelian-i-came-for-you" => return "Select an enemy. You gain telepathy on that enemy and if a move would bring you closer to the enemy, you can take 1 free turn afterward.",
            "isheilah-make-slippery" => return "Select an enemy; when they try to move, they don't and you spend 8 energy.",
            "lichor-bleed-them" => return "Your attacks inflict bleed proportional to your Damage bonus.",
            "morae-fortune-teller" => return "You can have up to 3 fortunes. Spend a fortune to give yourself lucky, ensuring you hit with your next attack, or an enemy creature unlucky, ensuring they miss.",
            "etill-half-here" => return "Activate to put down a half clone within line of sight that shares your health pool, doesn't move, but provides you sight from that location. Deactivate to remove it.",
            "pelian-never-surrender" => return "Spend 8 energy and add one sixth of your HP bonus to your DR the first time you take damage every turn.",
            _ => {}
        }
    }
    if run.simulation_revision.value_or(29.)>=43. && id=="shrail-fury-crafting" {
        "You may Fury Craft with hearts. Active Fury Crafts empower your first strike for 10 energy."
    } else { original }
}
/// Canonical displayed/command order, matching main.ts + crafting.ts. Recomputed
/// per refresh: See docs/ability-command-projection.md for catalog scans, aura/LOS and text costs.
pub fn visible_abilities(r: &Run) -> Vec<AbilityView> {
    let forecast=corpse_core::abilities::projected_upkeep(r);
    let mut rows: Vec<_> = dream_row(r).into_iter().collect();
    rows.extend(teleport_row(r));
    rows.extend(r.player.abilities.iter()
        .filter(|id| !matches!(id.as_str(), corpse_core::dreamwalking::WAKE_UP_ID | corpse_core::dreamwalking::DREAMWALK_ID | corpse_core::ahvamitt::TELEPORT_ID))
        .map(|id| observe_with_forecast(r, id, &forecast)));
    rows.extend(craft_rows(r));
    rows
}

/// A single canonical row also supplies numeric keys, pointer input and details.
fn dream_row(r: &Run) -> Option<AbilityView> {
    let active = corpse_core::dreamwalking::active(r);
    let permanent = corpse_core::dreamwalking::unlocked(r);
    if !active && !permanent { return None; }
    let unavailable = r.status != "playing" || !r.offers.is_empty()
        || r.player.conditions.get("deathResolutionBlocked").is_some_and(|v| *v > 0.);
    Some(AbilityView {
        reserve: None, indicator:None, fortunes:None,pending_reserve_loss: 0.,
        id: if permanent { corpse_core::dreamwalking::DREAMWALK_ID } else { corpse_core::dreamwalking::WAKE_UP_ID }.into(),
        name: if permanent { "Dreamwalk" } else { "Wake Up" }.into(),
        description: if permanent {
            "Leave your sleeping body behind to enter the Realm of Dreams. Toggle again to awake."
        } else {
            "Awake from the realm of dreams."
        }.into(),
        color: god_color(5),
        cost: "Free".into(),
        state: format!("{}{}", if unavailable { "Unavailable " } else { "" },
            if permanent { if active { "ON" } else { "OFF" } } else { "" }),
        active,
        mode: if permanent { "Toggle" } else { "Active" }.into(),
        supported: true,
    })
}

fn teleport_row(r: &Run) -> Option<AbilityView> {
    if r.simulation_revision.value_or(29.) < 34. || !corpse_core::ahvamitt::has_mask(r) { return None; }
    let cooldown = corpse_core::ahvamitt::cooldown(r);
    let unavailable = cooldown > 0 || r.status != "playing" || !r.offers.is_empty()
        || ["paralyzed", "deathResolutionBlocked"].iter()
            .any(|key| r.player.conditions.get(*key).is_some_and(|v| *v > 0.));
    Some(AbilityView {
        reserve: None, indicator:None, fortunes:None,pending_reserve_loss: 0.,
        id: corpse_core::ahvamitt::TELEPORT_ID.into(), name: "Teleport".into(),
        description: "Teleport to a random location on the floor.".into(),
        color: [216, 191, 117], cost: if cooldown > 0 { format!("{cooldown}t cooldown") } else { "100t cooldown · Free".into() },
        state: if unavailable { "Unavailable" } else { "Ready" }.into(),
        active: false, mode: "Carried item".into(), supported: true,
    })
}

fn craft_rows(r: &Run) -> Vec<AbilityView> {
    if r50(r) { return vec![]; }
    let mut rows = Vec::new();
    for item in &r.inventory {
        if !corpse_core::crafting::is_equipped(item) {
            continue;
        }
        for (slot, enhancement) in item.enhancements.iter().enumerate() {
            let Some(kind @ ("Fury" | "Zeal")) =
                corpse_core::crafting::craft_name(&enhancement.craft)
            else {
                continue;
            };
            let fury = kind == "Fury";
            let active = fury && enhancement.active.value_or(false);
            let c = |key: &str| r.player.conditions.get(key).copied().unwrap_or(0.);
            let reason = if r.status != "playing" {
                "This run has ended"
            } else if c("deathResolutionBlocked") > 0. {
                "Death resolution is blocked"
            } else if !r.offers.is_empty() {
                "Choose your level-up ability first"
            } else if c("paralyzed") > 0. {
                "You are paralyzed. Wait to advance time."
            } else if !fury && r.player.energy < 15. {
                "Needs 15 energy"
            } else if !fury && corpse_core::abilities::energy_activation_silenced(r, &r.player) {
                if r.simulation_revision.value_or(29.)>=43. { "Your ability is silenced and does not work." } else { "Energy-paid activation is silenced" }
            } else {
                ""
            };
            let mut description = if fury && r.simulation_revision.value_or(29.)>=43. {
                "Empowers your first strike for 10 energy."
            } else if fury {
                "Toggle is free. While equipped and enabled, spend 10 energy to add +10 base damage to an attack's first strike, even a miss. Enhancements stack and each pays once, after Unyielding. Includes thrown attacks; excludes reaction attacks. Unfunded enhancements switch off. Heart stat bonuses remain while disabled."
            } else {
                "Spend 15 energy to restore 25% of maximum HP, capped at maximum, without spending an action. Requires the item equipped; can be used at full HP."
            }.to_string();
            if !reason.is_empty() {
                description.push_str(&format!(" Unavailable: {reason}{}", if reason.ends_with('.') { "" } else { "." }));
            }
            rows.push(AbilityView {
        reserve: None, indicator:None, fortunes:None,pending_reserve_loss: 0.,
                id: format!("craft:{}:{slot}", item.id),
                name: format!("{kind} · {}", item.name),
                description,
                color: god_color(if fury { 0 } else { 1 }),
                cost: if fury {
                    "10 NRG/attack · free toggle"
                } else {
                    "15 NRG · free action"
                }
                .into(),
                state: format!(
                    "{}{}",
                    if reason.is_empty() {
                        ""
                    } else {
                        "Unavailable "
                    },
                    if fury {
                        if active {
                            "ON"
                        } else {
                            "OFF"
                        }
                    } else {
                        ""
                    }
                ),
                active,
                mode: if fury {
                    "Equipment toggle"
                } else {
                    "Equipment active"
                }
                .into(),
                supported: true,
            });
        }
    }
    rows
}

/// Active targeted toggles must dispatch OFF before requesting a fresh target.
pub fn deactivates_without_target(r: &Run, id: &str) -> bool {
    (id == corpse_core::dreamwalking::DREAMWALK_ID && corpse_core::dreamwalking::active(r))
        || r.player.active.iter().any(|active| active == id)
        || (id == "morae-see-the-future" && r.future.as_ref().is_some())
}

pub fn observe(r: &Run, id: &str) -> AbilityView {
    observe_with_forecast(r,id,&corpse_core::abilities::projected_upkeep(r))
}
fn observe_with_forecast(r:&Run,id:&str,forecast:&corpse_core::abilities::UpkeepProjection)->AbilityView {
    let mut row=observe_inner(r,id);
    if id=="morae-fortune-teller" {
        let count=r.player.conditions.get("fortunes").copied().unwrap_or(3.).clamp(0.,3.) as u8;
        let until=if r.simulation_revision.value_or(29.)>=52. {corpse_core::fortune_schedule::turns_until(&r.player,r.tick)} else {(20.-r.tick.rem_euclid(20.)).ceil() as u8};
        row.fortunes=Some((count,until));
    }
    if id=="morae-all-seeing-eyes" {row.description="Spend 30 energy to reveal the entire current floor, including enemies and contents, until your next turn.".into();}
    // Serialized ability order is acquisition order; only the first reaction
    // power owns the shared pool display. Never confuse modern Hanged with it.
    let first_reaction = r.player.abilities.iter().find(|a| {
        matches!(a.as_str(), "isheilah-quick-hand" | "isheilah-are-you-sure-about-that")
            || (a.as_str()=="isheilah-missed-me" && !corpse_core::hanged::enabled(r.simulation_revision.value_or(29.)))
    });
    row.indicator = match id {
        corpse_core::divine_rescues::RAGE if corpse_core::ragelight_rules::current(r) =>
            Some(CustomIndicator::RagelightCost(corpse_core::engine::condition(&r.player, "rageCost").to_bits())),
        "morae-see-the-future" => Some(CustomIndicator::Future(r.future.is_some())),
        "ursa-waking-nightmares" => Some(CustomIndicator::Nightmares {
            count:r.player.nightmares.value_or(0.).max(0.) as u64,
            until:(r.player.conditions.get("nightmareAt").copied().unwrap_or(r.tick+100.)-r.tick).max(0.).ceil() as u64,
        }),
        "pelian-you-shall-never-be-rid-of-me" => Some(CustomIndicator::Rescue(
            r.player.conditions.get("pelianUsed").copied().unwrap_or(0.)==0.)),
        "lichor-overflowing" if corpse_core::balance_rules::current(r) =>
            Some(CustomIndicator::OverflowingSaves(corpse_core::divine_rescues::saves(&r.player) as u64)),
        "lichor-overflowing" => Some(CustomIndicator::Blood {
            count:r.player.blood.value_or(0.).max(0.) as u64,
            progress:r.player.damage_bank.value_or(0.).max(0.).round() as u64,
        }),
        _ if first_reaction.is_some_and(|first| first==id) => Some(CustomIndicator::Reactions(r.player.reactions.value_or(0.).max(0.) as u64)),
        _ => None,
    };
    row.name=display_text(&row.name).into_owned();
    row.description=display_text(&row.description).into_owned();
    if r50(r) && uses_energy_in(r, id) && id != "morae-see-the-future" {
        row.reserve=r.player.ability_reserves.as_ref().and_then(|m|m.get(id))
            .filter(|s|corpse_core::revision50_ability_energy::valid_reserve(s))
            .map(|s|{let s=corpse_core::revision52_reserves::project_reserve(*s,r.simulation_revision.value_or(29.));(s.energy,s.capacity)});
        if row.reserve.is_some() {
            row.pending_reserve_loss=forecast.reserve_energy.get(id).copied().unwrap_or(0.);
        }
    }
    if id=="morae-see-the-future" && corpse_core::future_floor::enabled(r) {
        row.description=FUTURE_FLOOR_DESCRIPTION.into();
        row.name="Futuresight".into();
        row.active=corpse_core::future_floor::active(r);
        row.mode="Active".into();
        row.cost.clear();
        row.state.clear();
        row.reserve=None;
        let mut used=0u32;
        if let Some(state)=&r.future_floor {
            for &depth in &state.used_floors {
                let floor=corpse_core::fast_mode::display_depth(r,depth) as u32;
                if (1..=20).contains(&floor) {used|=1 << (floor-1);}
            }
        }
        row.indicator=Some(CustomIndicator::FutureFloors{used});
    }
    row
}
fn observe_inner(r: &Run, id: &str) -> AbilityView {
    let _timing = corpse_core::timing::span("desktop.ability");
    if matches!(id, corpse_core::dreamwalking::WAKE_UP_ID | corpse_core::dreamwalking::DREAMWALK_ID) {
        if let Some(row) = dream_row(r).filter(|row| row.id == id) { return row; }
    }
    if id == corpse_core::ahvamitt::TELEPORT_ID {
        if let Some(row) = teleport_row(r) { return row; }
    }
    if id.starts_with("craft:") {
        if let Some(row) = craft_rows(r).into_iter().find(|row| row.id == id) {
            return row;
        }
    }
    let Some(a) = progression::definition_for_revision(r.simulation_revision.value_or(29.), id) else {
        return AbilityView {
        reserve: None, indicator:None, fortunes:None,pending_reserve_loss: 0.,
            id: id.into(),
            name: id.into(),
            description: "Unknown ability".into(),
            color: [255, 248, 232],
            cost: "Unknown cost".into(),
            state: "Unavailable".into(),
            active: false,
            mode: "Unknown".into(),
            supported: false,
        };
    };
    let p = &r.player;
    let n = a.name.as_str();
    let c = |k: &str| p.conditions.get(k).copied().unwrap_or(0.);
    let flag = |f: &str| a.flags.iter().any(|v| v == f);
    let passive_ragelight = id == corpse_core::divine_rescues::RAGE && corpse_core::ragelight_rules::current(r);
    let toggle = !passive_ragelight && flag("toggle");
    let craft = flag("craft") && !toggle && !flag("activate");
    let callable = !passive_ragelight && (toggle
        || flag("activate")
        || flag("target")
        || flag("action")
        || matches!(n, "Revert" | "Blood Letting"));
    let active = if passive_ragelight || n == "Hanged" { false } else if n == "See The Future" {
        r.future.as_ref().is_some()
    } else {
        p.active.iter().any(|i| i == id)
    };
    let upkeep = if r50(r) { corpse_core::abilities::upkeep_cost_in(r,p,id) } else {match n {
        "Mind Read" | "Dream-See" if !r50(r) => 1.,
        "Fated Duel" => 3.,
        "Rushing Torrent" => 15.,
        "Phase Through" | "Black Pit" => 5.,
        "Riptide" => 10.,
        "Shadow in the Dark" => 2.,
        "Not There" => 3.,
        "Bonded By Unseen Threads" if !r50(r) => 8.,
        "Turning the Wheel" => 20.,
        "Unspeak" if active => c("unspeakCost"),
        _ => 0.,
    }};
    let activation = match n {
        "Half Here" if corpse_core::half_here::paid_placement(r) => corpse_core::half_here::PLACEMENT_ENERGY,
        "Implant Thought" if r50(r) => 15.,
        "Relentless Pursuit" => 10.,
        "Cogworks" if corpse_core::revision50_terrain::mode(r)==3 => 0.,
        "Cogworks" | "Clay Army" => 20.,
        "I Came For You" | "Distract" => 10.,
        "Implant Thought" | "All-Seeing Eyes" | "Mini-Me" | "Sever Fate" => 30.,
        "Mind Slave" | "Tinker" => 100.,
        "Revert" => 2.,
        "Remember" => 5.,
        "Sudden Sleep" | "Sparkle" | "Bulwark of Mind" | "Make Kneel" => 20.,
        "Dream of Time Gone By" => corpse_core::balance_rules::dream_anchor_cost(r),
        "Ascend" => 50.,
        "Riptide" | "Bonded By Unseen Threads" => 15.,
        "See The Future" => p.energy,
        _ => 0.,
    };
    let suppression = matches!(n, "Entropy" | "Shine" | "Star Power");
    let mut cost = match n {
        "Execute" => format!("{} NRG/execute",corpse_core::revision50_combat::execute_cost(r.simulation_revision.value_or(29.))),
        "Commandment" => "20 NRG/hit".into(),
        "Waking Nightmares" => "1 nightmare".into(),
        "Cogworks" => match corpse_core::revision50_terrain::mode(r) {
            2=>"Mode 2 · 20 NRG · 200 shared HP".into(),3=>"Mode 3 · free removal".into(),_=>"Mode 1 · 20 NRG".into(),
        },
        "Unyielding Assault" => "10 NRG/attack".into(),
        "Never Without a Fight" => "10 NRG/counter".into(),
        "Never Surrender" if r.simulation_revision.value_or(29.) >= 52. => "8 NRG/first source".into(),
        "Never Surrender" => "8 NRG/first hit".into(),
        "Hanged" => "4 main NRG/HP".into(),
        "Missed Me" if r50(r) => "1 reaction/retreat".into(),
        "Missed Me" => "1 reaction/first attack".into(),
        "Make Slippery" => "8 NRG/move".into(),
        "Quick Hand" => "1 reaction/hit".into(),
        "Are You Sure About That?" => "1 reaction/miss".into(),
        "Waking Nightmare" => "1 nightmare/hit".into(),
        "Overflowing" if corpse_core::balance_rules::current(r) => "1 save/rescue".into(),
        "Overflowing" => "1 blood/rescue".into(),
        "Erase Metaphysical" => "10 NRG/hit".into(),
        "Nova" => format!("{} NRG/blast", corpse_core::abilities::nova_attack_cost(r.simulation_revision.value_or(29.))),
        "Uncast" => "10 NRG/spell".into(),
        "Rage at the Dying of the Light" => format!("{} NRG/rescue", crate::stat_presentation::whole_number(c("rageCost"))),
        "You Shall Never Be Rid of Me" => if c("pelianUsed") > 0. {
            "Spent/level"
        } else {
            "Free/level"
        }
        .into(),
        "Revert" => "2 NRG + 1 charge".into(),
        "Fortune Teller" => "1 fortune".into(),
        "Blood Weapon" => format!("{} HP", crate::stat_presentation::whole_number(corpse_core::divine_combat::blood_weapon_hp_cost_for_run(r, p))),
        "Blood Letting" => "10 HP".into(),
        "See The Future" => {
            if active {
                "Free return".into()
            } else if r50(r) {
                format!("All own reserve + {} player NRG", crate::stat_presentation::whole_number(p.energy))
            } else {
                format!("{} NRG", crate::stat_presentation::whole_number(p.energy))
            }
        }
        "Unspeak" => {
            if active {
                format!("{upkeep:.0} NRG/t")
            } else {
                "Target NRG/t".into()
            }
        }
        _ if craft => "Craft · Pack".into(),
        _ if suppression => "Free".into(),
        _ if activation > 0. && upkeep > 0. => format!("{activation:.0} NRG + {upkeep:.0}/t"),
        _ if upkeep > 0. => format!("{upkeep:.0} NRG/t"),
        _ if activation > 0. => format!(
            "{activation:.0} NRG{}",
            if flag("action") { " + action" } else { "" }
        ),
        _ => "Free".into(),
    };
    if active && n == "Dream of Time Gone By" {
        cost = "Free return".into();
    }
    if active && n == "Half Here" && corpse_core::half_here::paid_placement(r) {
        cost = "Free dismissal".into();
    }
    let passive = !callable && !craft;
    if passive {
        cost = if n == "Hanged" { "Passive · 4 main NRG/HP" } else { "Passive" }.into();
    }
    // All counters are player-owned; none consult a hidden linked target.
    let counter = match n {
        "Overflowing" if corpse_core::balance_rules::current(r) =>
            Some(format!("{}/5 saves", corpse_core::divine_rescues::saves(p))),
        "Overflowing" => Some(format!(
            "{}/55 blood · {}/50 dmg",
            p.blood.value_or(0.),
            p.damage_bank.value_or(0.)
        )),
        "Waking Nightmare" | "Waking Nightmares" => Some(format!(
            "{} dreams · +1/{}t",
            p.nightmares.value_or(0.),
            (p.conditions
                .get("nightmareAt")
                .copied()
                .unwrap_or(r.tick + 100.)
                - r.tick)
                .max(1.)
        )),
        "The Number of Blood" => {
            let round = r.tick
                + if r.resolving_tick.value_or(false) {
                    0.
                } else {
                    1.
                };
            let count = if round % 5. == 0. { 5. } else { round % 5. };
            let used = p.conditions.get("angelUsedRound") == Some(&round);
            Some(format!(
                "{count}/5{}",
                if count == 5. && !used {
                    " READY"
                } else if count == 5. && used {
                    " used"
                } else {
                    " phase"
                }
            ))
        }
        "Blood Weapon" => Some(
            if c("bloodWeaponReady") > 0. {
                "READY"
            } else {
                "Unprimed"
            }
            .into(),
        ),
        "Adaptive Armor" => {
            let rows = p
                .adaptive_hearts
                .as_ref()
                .map(|m| {
                    m.iter()
                        .filter(|(_, count)| **count > 0.)
                        .map(|(name, count)| {
                            let name = corpse_core::revision50_creature_identity::name_for_revision(name, r.simulation_revision.value_or(29.));
                            let n = count.floor().clamp(0., 3.);
                            format!("{name} +{} DR", n * (7. - n) / 2.)
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            Some(format!("{} types", rows.len()))
        }
        "Revert" => Some(format!("{}/5 charges", c("revertCharges"))),
        "Fortune Teller" => Some(format!(
            "{}/3 fortunes",
            p.conditions.get("fortunes").copied().unwrap_or(3.)
        )),
        "Missed Me" | "Are You Sure About That?" | "Quick Hand" => Some(format!(
            "{}/{} reactions",
            p.reactions.value_or(0.),
            corpse_core::engine::max_reactions_for_revision(r.simulation_revision.value_or(29.), p)
        )),
        "Rage at the Dying of the Light" => Some(format!("{} NRG/rescue", crate::stat_presentation::whole_number(c("rageCost")))),
        "You Shall Never Be Rid of Me" => Some(format!(
            "{}/1 rescues",
            if c("pelianUsed") > 0. { 0 } else { 1 }
        )),
        _ => None,
    };
    if let Some(counter) = counter {
        cost = if !passive
            && !craft
            && cost != "Free"
            && !matches!(
                n,
                "Rage at the Dying of the Light" | "You Shall Never Be Rid of Me"
            ) {
            format!("{counter} · {cost}")
        } else {
            counter
        };
    }
    let paid = activation > 0. || upkeep > 0. || matches!(n,
        "Execute" | "Commandment" | "Never Surrender" | "Make Slippery" |
        "Erase Metaphysical" | "Nova" | "Unspeak" | "Rage at the Dying of the Light" | "See The Future");
    let reserve = (id != "morae-see-the-future" && (paid || r50_uses_energy(id))).then(||reserve_summary(r,id)
        .or_else(||r50(r).then(||"Reserve not configured".into()))).flatten();
    if let Some(summary)=&reserve { cost=format!("{summary} · {cost}"); }
    let upkeep_due=(upkeep-corpse_core::abilities::prepaid_upkeep(r,&r.player,id)).max(0.);
    let activation_blocker=payment_blocker(r,id,activation+upkeep_due);
    let supported = runtime_supported(r, id);
    let unavailable = !supported || callable
        && (r.status != "playing"
            || !r.offers.is_empty()
            || c("paralyzed") > 0.
            || c("deathResolutionBlocked") > 0.
            || (n == "Blood Weapon" && c("bloodWeaponReady") > 0.)
            || (!active
                && (activation_blocker.is_some()
                    || (n == "Revert" && (r.history.is_empty() || c("revertCharges") < 1.))
                    || (n == "Fortune Teller"
                        && p.conditions.get("fortunes").copied().unwrap_or(3.) < 1.)
                    || (n == "Waking Nightmares" && p.nightmares.value_or(0.) < 1.)
                    || (n == "Ascend"
                        && !r
                            .floors
                            .iter()
                            .find(|f| f.depth == r.depth - 1.)
                            .is_some_and(|f| {
                                p.x >= 0.
                                    && p.y >= 0.
                                    && p.x < f.width() as f64
                                    && p.y < f.height() as f64
                                    && f.tiles[p.y as usize * f.width() + p.x as usize] == "chasm"
                            })))));
    let state = format!(
        "{}{}",
        if unavailable { "Unavailable " } else { "" },
        if toggle {
            if active {
                "ON"
            } else {
                "OFF"
            }
        } else {
            ""
        }
    );
    let mut row = AbilityView {
        reserve: None, indicator:None, fortunes:None,pending_reserve_loss: 0.,
        id: id.into(),
        name: a.name.clone(),
        description: if n == "Adaptive Armor" {
            let details = p
                .adaptive_hearts
                .as_ref()
                .map(|types| {
                    types
                        .iter()
                        .filter(|(_, count)| **count > 0.)
                        .map(|(name, count)| {
                            let name = corpse_core::revision50_creature_identity::name_for_revision(name, r.simulation_revision.value_or(29.));
                            let hearts = count.floor().clamp(0., 3.);
                            format!(
                                "{name}: {count} hearts, +{} DR.",
                                hearts * (7. - hearts) / 2.
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default();
            format!(
                "{} Current adaptations: {}",
                display_description(r, &a.id, &a.description),
                if details.is_empty() {
                    "None."
                } else {
                    &details
                }
            )
        } else {
            display_description(r, &a.id, &a.description).into()
        },
        color: god_color(a.god_id as usize),
        cost,
        state,
        active,
        mode: if craft {
            "Craft"
        } else if passive {
            "Passive"
        } else if toggle {
            "Toggle"
        } else {
            "Active"
        }
        .into(),
        supported,
    };
    if r.simulation_revision.value_or(29.) >= 52. && id == "pelian-never-surrender" {
        row.description.push_str("\nDamage that ignores DR spends no energy and leaves this activation available.");
    }
    if r50(r) && id=="kative-clay-army" {
        row.description.push_str("\nIt is an unbreakable wall.");
    }
    if id == corpse_core::revision50_ability_energy::FUTURE_ID {
        row.description.push_str("\nSpends all your energy.");
    }
    if let Some(reason)=activation_blocker.filter(|_|r50(r) && !active) {
        row.description.push_str(&format!("\nUnavailable: {reason}."));
    }
    row
}

/// Fixed authored R50 consumers. Owning an unused reserve is not a UI cost.
/// Includes free return/removal modes of abilities that spend energy in other modes.
pub fn r50_uses_energy(id:&str)->bool {
    matches!(id,
        "shrail-rage-at-the-dying-of-the-light" | "shrail-execute" |
        "pelian-fated-duel" | "pelian-never-surrender" | "pelian-bulwark-of-mind" | "pelian-relentless-pursuit" |
        "isheilah-make-slippery" | "numo-implant-thought" | "numo-distract" | "numo-mind-slave" | "numo-remember" |
        "etill-phase-through" | "etill-rushing-torrent" | "etill-riptide" |
        "ursa-dream-see" | "numo-mind-read" | "morae-bonded-by-unseen-threads" | "pelian-i-came-for-you" |
        "ursa-sudden-sleep" | "ursa-dream-of-time-gone-by" | "dawel-unspeak" | "dawel-shadow-in-the-dark" | "dawel-not-there" |
        "grisha-erase-metaphysical" | "grisha-black-pit" | "anavani-ascend" | "anavani-nova" | "anavani-illuminate" | "anavani-commandment" |
        "kative-cogworks" | "kative-mini-me" | "kative-clay-army" | "kative-tinker" |
        "morae-turning-the-wheel" | "morae-sever-fate" | "morae-see-the-future" | "morae-all-seeing-eyes")
}
fn uses_energy_in(r: &Run, id: &str) -> bool {
    r50_uses_energy(id) || (id == corpse_core::half_here::ID && corpse_core::half_here::paid_placement(r))
}
/// Expected O(ID bytes), O(text bytes) output; no resource mutation or defaults.
pub fn reserve_summary(r: &Run, id: &str) -> Option<String> {
    if !r50(r) || !uses_energy_in(r, id) {return None;}
    let mut summary = format_reserve(&r.player,id,r.simulation_revision.value_or(29.))?;
    if let Some(reserve) = r.player.ability_reserves.as_ref().and_then(|m|m.get(id)) {
        if corpse_core::revision50_ability_energy::valid_reserve(reserve) {
            let reserve = corpse_core::revision52_reserves::project_reserve(*reserve,r.simulation_revision.value_or(29.));
            summary.push_str(&crate::render::resource_ui::change_suffix(r,
                &corpse_core::resources::ability_key(id),reserve.energy,reserve.capacity));
        }
    }
    Some(summary)
}
pub fn actor_reserve_summary(actor: &corpse_core::Actor, id: &str) -> Option<String> {
    actor_reserve_summary_for_revision(actor,id,50.)
}
pub fn actor_reserve_summary_for_revision(actor: &corpse_core::Actor, id: &str, revision:f64) -> Option<String> {
    if !r50_uses_energy(id) {return None;}
    format_reserve(actor, id, revision)
}
fn format_reserve(actor: &corpse_core::Actor, id: &str, revision:f64) -> Option<String> {
    let reserve=actor.ability_reserves.as_ref()?.get(id)?;
    Some(if corpse_core::revision50_ability_energy::valid_reserve(reserve) {
        let reserve=corpse_core::revision52_reserves::project_reserve(*reserve,revision);
        format!("Reserve {}/{}",reserve.energy,reserve.capacity)
    } else {"Reserve invalid".into()})
}
/// Mirrors atomic core split payment and Future's own-reserve/player drain.
pub fn payment_blocker(r: &Run, id: &str, cost: f64) -> Option<String> {
    if !r50(r) {return (r.player.energy<cost).then(||format!("Needs {cost:.0} energy"));}
    use corpse_core::revision50_ability_energy::{plan_payment_for_revision, plan_future_payment_for_revision, EnergyError, FUTURE_ID};
    let plan = if id == FUTURE_ID {
        if r.future.is_some() { return None; }
        plan_future_payment_for_revision(&r.player,r.simulation_revision.value_or(29.))
    } else {
        if cost == 0. { return None; }
        plan_payment_for_revision(r.player.ability_reserves.as_ref(),id,r.player.energy,cost,r.simulation_revision.value_or(29.))
    };
    if id == FUTURE_ID && plan.as_ref().is_ok_and(|p| p.actor_energy_spent() > 0. || p.reserve_energy_spent() > 0.)
        && corpse_core::abilities::energy_activation_silenced(r, &r.player) {
        return Some("Silence prevents this energy-paid activation".into());
    }
    plan.err().map(|error|match error {
        EnergyError::MissingReserve if id == FUTURE_ID=>"Cannot save this state".into(),
        EnergyError::MissingReserve=>"Reserve not configured".into(),
        EnergyError::InsufficientEnergy=>format!("Reserve and player energy cannot cover {cost:.0} energy"),
        _=>"Ability energy configuration is invalid".into(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Run {
        corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json"))
            .unwrap()
    }
    // Frozen crafting.ts: equipped inventory order, enhancement slot IDs, Fury
    // toggle and Zeal free-action dispatch; non-activation crafts are omitted.
    #[test]
    fn crafted_rows_share_command_order_and_native_execution() {
        use corpse_core::model::{Item, ItemEnhancementsEntry};
        let mut r = fixture();
        r.player.abilities = vec!["numo-revert".into()];
        r.offers.clear();
        r.inventory = vec![Item {
            id: 731.,
            name: "Crafted blade".into(),
            kind: "weapon".into(),
            equipped: "hand1".to_string().into(),
            enhancements: ["Fury", "Guile", "Zeal"]
                .iter()
                .map(|name| ItemEnhancementsEntry {
                    craft: (*name).into(),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }];
        let rows = visible_abilities(&r);
        assert_eq!(
            rows.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(),
            ["numo-revert", "craft:731:0", "craft:731:2"]
        );
        assert_eq!(rows[1].state, "OFF");
        let tick = r.tick;
        assert!(corpse_core::crafting::activate(&mut r, &rows[1].id).unwrap());
        assert!(observe(&r, &rows[1].id).active);
        r.player.energy = 30.;
        r.player.hp = r.player.max_hp / 2.;
        let hp = r.player.hp;
        assert!(corpse_core::crafting::activate(&mut r, &rows[2].id).unwrap());
        assert_eq!(r.player.energy, 15.);
        assert_eq!(r.player.hp, hp + r.player.max_hp * 0.25);
        assert_eq!(r.tick, tick);
        r.player.energy = 14.;
        assert!(observe(&r, &rows[2].id).state.contains("Unavailable"));
        r.player.energy = 30.;
        r.player.conditions.insert("silenced".into(), 1.);
        assert!(observe(&r, &rows[2].id).description.contains("silenced"));
        assert!(!observe(&r, &rows[1].id).state.contains("Unavailable"));
        r.inventory[0].equipped = "".to_string().into();
        assert_eq!(visible_abilities(&r).len(), 1);
    }
    // Frozen ability-presentation.ts uses one resource per creature type. Keep
    // every entry in scrollable prose, rather than truncating the cost line.
    #[test]
    fn adaptive_armor_all_types_survive_detail_projection() {
        let mut r = fixture();
        let mut types = r
            .player
            .adaptive_hearts
            .as_ref()
            .cloned()
            .unwrap_or_default();
        types.clear();
        for i in 0..20 {
            types.insert(format!("creature-{i}"), 3.);
        }
        r.player.adaptive_hearts = types.into();
        let view = observe(&r, "kative-adaptive-armor");
        assert_eq!(view.cost, "20 types");
        for i in 0..20 {
            assert!(view
                .description
                .contains(&format!("creature-{i}: 3 hearts, +6 DR.")));
        }
    }
    // Frozen main.ts/skills-runtime.ts: active targeted toggles can be turned
    // off without choosing another tile/creature.
    #[test]
    fn active_targeted_toggle_deactivates_before_target_prompt() {
        let mut r = fixture();
        for name in ["Unspeak", "Riptide", "Bonded By Unseen Threads"] {
            let id = &progression::catalog()
                .iter()
                .find(|a| a.name == name)
                .unwrap()
                .id;
            r.player.active.clear();
            assert!(!deactivates_without_target(&r, id));
            r.player.active.push(id.clone());
            assert!(deactivates_without_target(&r, id));
        }
    }
    #[test]
    fn frozen_ability_rows() {
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("../assets/ability-ui-parity.json")).unwrap();
        let mut differences = Vec::new();
        for row in rows {
            let mut r = corpse_core::storage::deserialize(include_str!(
                "../../../fixtures/avatar-room-v1.json"
            ))
            .unwrap();
            let id = row["id"].as_str().unwrap();
            r.player.abilities = vec![id.into()];
            r.player.active = if row["active"] == true {
                vec![id.into()]
            } else {
                vec![]
            };
            let v = observe(&r, id);
            let mut summary = v
                .state
                .split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>();
            summary.push(v.cost);
            let summary = summary.join(" · ");
            if summary != row["summary"] {
                differences.push(format!(
                    "{id} active{}: {summary:?} != {}",
                    row["active"], row["summary"]
                ));
            }
        }
        assert!(differences.is_empty(), "{}", differences.join("\n"));
    }
}

#[cfg(test)]
mod ahvamitt_ui_tests {
    use super::*;
    fn scene() -> Run {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=34.0.into();r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
        r.offers.clear();r.inventory=vec![corpse_core::ahvamitt::mask(201.)];r.actors.clear();
        let (x,y)=(r.player.x+1.,r.player.y);let w=r.floor().unwrap().width();
        r.floor_mut().unwrap().tiles[y as usize*w+x as usize]="floor".into();
        r.actors_at_mut(r.depth).push(corpse_core::ahvamitt::actor(30.,x,y));r
    }
    #[test]
    fn ahvamitt_ui_carried_mask_grants_one_teleport_row_and_preserves_cooldown_when_dropped() {
        let mut r=scene();let rows=visible_abilities(&r);assert_eq!(rows.len(),1);
        assert_eq!(rows[0].name,"Teleport");assert_eq!(rows[0].id,corpse_core::ahvamitt::TELEPORT_ID);
        assert_eq!(rows[0].state,"Ready");assert!(!crate::observe::ability_needs_target(&rows[0].id));
        assert_eq!(observe(&r,&rows[0].id).description,rows[0].description);
        r.player.abilities.push(corpse_core::ahvamitt::TELEPORT_ID.into());assert_eq!(visible_abilities(&r).len(),1);
        r.player.conditions.insert("ahvamittTeleportReadyAt".into(),r.tick+100.);
        assert_eq!(visible_abilities(&r)[0].state,"Unavailable");assert!(visible_abilities(&r)[0].cost.contains("100t"));
        r.tick+=99.;assert!(visible_abilities(&r)[0].cost.contains("1t"));
        let mask=r.inventory.pop().unwrap();assert!(visible_abilities(&r).is_empty());
        r.inventory.push(mask);assert_eq!(visible_abilities(&r)[0].state,"Unavailable");
        r.tick+=1.;assert_eq!(visible_abilities(&r)[0].state,"Ready");
    }
    #[test]
    fn ahvamitt_ui_exact_flavor_and_interaction_preview_without_bestiary_entry() {
        let r=scene();let report=crate::inspection_ui::observe_tile(&r,(r.player.x+1.) as usize,r.player.y as usize);
        let actor=report.creature.unwrap();assert_eq!(actor.description,"A Godbeast of trickery.");
        assert!(actor.your_attack.contains("continue his game"));assert!(actor.their_attack.contains("does not move or attack"));
        assert!(crate::manual::Manual::new().catalog.entries.iter().all(|e|e.name!="Ahvamitt"));
    }
    #[test]
    fn ahvamitt_ui_actual_renderer_review() {
        let r=scene();let view=crate::observe::observe(&r);let mut p=crate::render::Painter::new();
        p.game_at(&view,"",false,0.);
        if let Ok(directory)=std::env::var("CORPSE_DREAM_UI_REVIEW") {
            let report=crate::inspection_ui::observe_tile(&r,(r.player.x+1.) as usize,r.player.y as usize);
            crate::inspection_ui::paint(&report,&mut p,0);
            let file=std::fs::File::create(format!("{directory}/ahvamitt-teleport.png")).unwrap();
            let mut encoder=png::Encoder::new(file,p.width as u32,p.height as u32);
            encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
        }
    }
}

#[cfg(test)]
#[path = "authored_copy_r38_tests.rs"]
mod authored_copy_r38_tests;

/// O(ID bytes); only the new passive has no command binding. Older passive rows
/// retain their historical click/key behavior; offer selection/details are separate.
pub fn command_binding_allowed(r: &Run, id: &str) -> bool {
    if id == corpse_core::divine_rescues::RAGE && corpse_core::ragelight_rules::current(r) { return false; }
    if r50(r) {
        if !runtime_supported(r, id) { return false; }
        if progression::definition_for_revision(r.simulation_revision.value_or(29.), id).is_some_and(|a| a.flags.iter().all(|flag| flag == "passive") && a.name != "Blood Letting") { return false; }
    }
    !(r.simulation_revision.value_or(29.0) >= 41.0 && id == progression::WEAVE_ID)
}

#[cfg(test)]
mod weave_presentation_tests {
    use super::*;
    /// O(fixture bytes + ability metadata projection); no simulation or renderer timing.
    #[test]
    fn weave_is_passive_exact_description_without_activation_binding() {
        let mut r=corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=41.0.into();r.player.abilities=vec![progression::WEAVE_ID.into()];r.player.active.clear();r.player.conditions.clear();
        let row=observe(&r,progression::WEAVE_ID);assert_eq!(row.name,progression::WEAVE_NAME);assert_eq!(row.description,progression::WEAVE_DESCRIPTION);
        assert_eq!(row.mode,"Passive");assert_eq!(row.cost,"Passive");assert!(row.supported);assert!(!row.active);assert!(!command_binding_allowed(&r,&row.id));assert!(!crate::observe::ability_needs_target(&row.id));
        r.simulation_revision=40.0.into();r.player.abilities=vec!["numo-revert".into()];let old=observe(&r,"numo-revert");assert_eq!(old.name,"Revert");assert_eq!(old.mode,"Active");assert_eq!(old.cost,"0/5 charges · 2 NRG + 1 charge");assert!(command_binding_allowed(&r,&old.id));
    }
}

#[cfg(test)]
mod revision43_description_tests {
    use super::*;
    #[test]
    fn zeals_silence_reason_has_exact_sentence_without_double_punctuation() {
        let mut run:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        run.simulation_revision=43.0.into();run.offers.clear();run.player.energy=30.;run.player.conditions.clear();run.player.conditions.insert("silenced".into(),1.);
        run.inventory=vec![corpse_core::Item{id:6002.,name:"Crafted blade".into(),kind:"weapon".into(),equipped:"hand1".to_string().into(),enhancements:vec![corpse_core::ItemEnhancementsEntry{craft:"Zeal".into(),..Default::default()}],..Default::default()}];
        let rows=craft_rows(&run);assert_eq!(rows.len(),1);
        assert!(rows[0].description.ends_with("Your ability is silenced and does not work."));assert!(!rows[0].description.ends_with(".."));
    }
    #[test]
    fn fury_copy_is_display_only_for_rows_offers_and_granted_items() {
        let mut run:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let id="shrail-fury-crafting";let original=progression::definition(id).unwrap().description.clone();
        let requested="You may Fury Craft with hearts. Active Fury Crafts empower your first strike for 10 energy.";
        for revision in [42.,43.] {
            run.simulation_revision=revision.into();
            let before=serde_json::to_vec(&run).unwrap();
            assert_eq!(observe(&run,id).description,if revision>=43.{requested}else{original.as_str()});
            let item=corpse_core::Item{id:6001.,name:"Blessed ring".into(),kind:"ring".into(),granted_ability:id.to_string().into(),..Default::default()};
            let detail=crate::pack::describe_item(&run,&item);
            assert!(!detail.contains(if revision>=43.{requested}else{original.as_str()}));
            assert_eq!(serde_json::to_vec(&run).unwrap(),before);
        }
        assert_eq!(progression::definition(id).unwrap().description,original);
        assert!(original.contains("10 extra damage"));
    }
}

#[cfg(test)]
mod revision44_description_tests {
    use super::*;
    const REQUESTED: &[(&str, &str)] = &[
        ("shrail-fight-long", "Heal 1/5 the damage you deal in melee."),
        ("shrail-beat-to-death", "Reduce target dodge by 1 and DR 5 when you hit an enemy until they die."),
        ("pelian-bulwark-of-mind", "Spend 20 energy and select a visible, unoccupied square. Enemies cannot enter it for 10 turns."),
        ("lichor-the-number-of-blood", "Every 5 turns, automatically hit and do double damage."),
        ("grisha-boundless-hunger", "You have no max energy."),
        ("dawel-reverse-entropy", "While active, enemies do not heal, regain energy, or revive, plants do not grow and neither gas nor fire spreads within 10."),
        ("morae-bonded-by-unseen-threads", "On activation, select an enemy; when you take damage, they take half."),
        ("morae-see-the-future", "Spend all your energy to save your current state. Deactivating will return you there, undoing all that has been done."),
        ("numo-mind-read", "While active, you have telepathy within 20. While inactive, you have telepathy within 5."),
        ("etill-float-away", "When active, if an enemy damages you, fly until your next turn and move 1 tile away."),
        ("ursa-dream-see", "You have telepathy with sleeping creatures. When active, you have telepathy with all creatures that are not obstructed."),
        ("kative-dilute-potion", "When you use a potion, you can use it again at half efficacy."),
        ("pelian-i-came-for-you", "Select an enemy. You gain telepathy on that enemy and if a move would bring you closer to the enemy, you can take 1 free turn afterward."),
        ("isheilah-make-slippery", "Select an enemy; when they try to move, they don't and you spend 8 energy."),
        ("lichor-bleed-them", "Your attacks inflict bleed proportional to your Damage bonus."),
        ("morae-fortune-teller", "You can have up to 3 fortunes. Spend a fortune to give yourself lucky, ensuring you hit with your next attack, or an enemy creature unlucky, ensuring they miss."),
        ("etill-half-here", "Activate to put down a half clone within line of sight that shares your health pool, doesn't move, but provides you sight from that location. Deactivate to remove it."),
        ("pelian-never-surrender", "Spend 8 energy and add one sixth of your HP bonus to your DR the first time you take damage every turn."),
    ];
    #[test]
    fn exact_authored_copy_reaches_rows_grants_and_offers_without_catalog_mutation() {
        let mut run: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        for revision in [42., 43., 44.] {
            run.simulation_revision = revision.into();
            run.offers = REQUESTED.iter().map(|(id, _)| (*id).into()).collect();
            run.player.abilities = REQUESTED.iter().map(|(id, _)| (*id).into()).collect();
            let before = serde_json::to_vec(&run).unwrap();
            let view = crate::observe::observe(&run);
            for ((id, requested), offer) in REQUESTED.iter().zip(&view.offers) {
                let original = progression::definition(id).unwrap().description.clone();
                let expected = if revision >= 44. { *requested } else { original.as_str() };
                assert_eq!(display_description(&run, id, &original), expected, "{revision}: {id}");
                let detail_expected = if *id == "morae-see-the-future" { format!("{expected}\nSpends all your energy.") } else { expected.to_string() };
                assert_eq!(observe(&run, id).description, detail_expected, "ability detail: {id}");
                assert_eq!(offer.description, detail_expected, "blessing choice: {id}");
                let item = corpse_core::Item { id: 6001., name: "Blessed ring".into(), kind: "ring".into(), granted_ability: (*id).to_string().into(), ..Default::default() };
                assert!(!crate::pack::describe_item(&run, &item).lines().any(|line| line == expected), "item grant must omit full ability copy: {id}");
                assert_eq!(progression::definition(id).unwrap().description, original);
            }
            assert_eq!(view.offers.len(), REQUESTED.len());
            let selected_ids: Vec<_> = visible_abilities(&run).into_iter()
                .filter(|row| REQUESTED.iter().any(|(id, _)| *id == row.id))
                .map(|row| row.id).collect();
            assert_eq!(selected_ids, REQUESTED.iter().map(|(id, _)| (*id).to_string()).collect::<Vec<_>>());
            assert_eq!(serde_json::to_vec(&run).unwrap(), before);
        }
    }
    #[test]
    fn approved_unchanged_copy_and_prior_fury_override_are_preserved() {
        let mut run: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        run.simulation_revision = 44.0.into();
        for id in ["dawel-shadow-in-the-dark", "kative-mini-me", "numo-implant-thought", "grisha-uncast"] {
            let original = &progression::definition(id).unwrap().description;
            assert_eq!(observe(&run, id).description, *original);
        }
        assert_eq!(observe(&run, "shrail-fury-crafting").description,
            "You may Fury Craft with hearts. Active Fury Crafts empower your first strike for 10 energy.");
        assert_eq!(display_description(&run, "unknown", "Original."), "Original.");
    }
}

#[cfg(test)]
mod revision45_description_tests {
    use super::*;
    const REQUESTED: &[(&str, &str)] = &[
        ("lichor-blood-weapon", "Activate by paying 10% of your maximum HP. Your next attack is guaranteed to hit and deals double damage."),
        ("grisha-entropy", "You are immune to the effects of caustic gas, and have a cloud of caustic gas originating from you. When this is active, caustic gas does not originate from you."),
        ("anavani-nova", "You are immune to explosions. When active, when you attack, create an explosion and spend 40 energy. Every explosion deals 50% of each affected creature's maximum HP."),
    ];
    #[test]
    fn exact_authored_copy_reaches_rows_grants_and_offers_without_catalog_mutation() {
        let mut run: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        for revision in [43., 44., 45.] {
            run.simulation_revision = revision.into();
            run.offers = REQUESTED.iter().map(|(id, _)| (*id).into()).collect();
            run.player.abilities = REQUESTED.iter().map(|(id, _)| (*id).into()).collect();
            let before = serde_json::to_vec(&run).unwrap();
            let view = crate::observe::observe(&run);
            for ((id, requested), offer) in REQUESTED.iter().zip(&view.offers) {
                let original = progression::definition(id).unwrap().description.clone();
                let expected = if revision >= 45. { *requested } else { original.as_str() };
                assert_eq!(display_description(&run, id, &original), expected, "{revision}: {id}");
                let detail_expected = if *id == "morae-see-the-future" { format!("{expected}\nSpends all your energy.") } else { expected.to_string() };
                assert_eq!(observe(&run, id).description, detail_expected, "ability detail: {id}");
                if *id == "anavani-nova" {
                    assert_eq!(observe(&run, id).cost, if revision >= 45. { "40 NRG/blast" } else { "30 NRG/blast" });
                }
                assert_eq!(offer.description, detail_expected, "blessing choice: {id}");
                let item = corpse_core::Item { id: 6001., name: "Blessed ring".into(), kind: "ring".into(), granted_ability: (*id).to_string().into(), ..Default::default() };
                assert!(!crate::pack::describe_item(&run, &item).lines().any(|line| line == expected), "item grant must omit full ability copy: {id}");
                assert_eq!(progression::definition(id).unwrap().description, original);
            }
            assert_eq!(view.offers.len(), REQUESTED.len());
            let selected_ids: Vec<_> = visible_abilities(&run).into_iter()
                .filter(|row| REQUESTED.iter().any(|(id, _)| *id == row.id))
                .map(|row| row.id).collect();
            assert_eq!(selected_ids, REQUESTED.iter().map(|(id, _)| (*id).to_string()).collect::<Vec<_>>());
            assert_eq!(serde_json::to_vec(&run).unwrap(), before);
        }
    }
}

#[cfg(test)]
mod revision46_description_tests {
    use super::*;
    const REQUESTED: &[(&str, &str)] = &[
        ("anavani-shine", "You emit light 100 which decreases by 10 every square away from you. You gain additional accuracy equal to the illumination of enemies. When active, this ability does not illuminate you."),
    ];
    #[test]
    fn exact_authored_copy_reaches_rows_grants_and_offers_without_catalog_mutation() {
        let mut run: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        for revision in [44., 45., 46.] {
            run.simulation_revision = revision.into();
            run.offers = REQUESTED.iter().map(|(id, _)| (*id).into()).collect();
            run.player.abilities = REQUESTED.iter().map(|(id, _)| (*id).into()).collect();
            let before = serde_json::to_vec(&run).unwrap();
            let view = crate::observe::observe(&run);
            for ((id, requested), offer) in REQUESTED.iter().zip(&view.offers) {
                let original = progression::definition(id).unwrap().description.clone();
                let expected = if revision >= 46. { *requested } else { original.as_str() };
                assert_eq!(display_description(&run, id, &original), expected, "{revision}: {id}");
                let detail_expected = if *id == "morae-see-the-future" { format!("{expected}\nSpends all your energy.") } else { expected.to_string() };
                assert_eq!(observe(&run, id).description, detail_expected, "ability detail: {id}");
                assert_eq!(offer.description, detail_expected, "blessing choice: {id}");
                let item = corpse_core::Item { id: 6001., name: "Blessed ring".into(), kind: "ring".into(), granted_ability: (*id).to_string().into(), ..Default::default() };
                assert!(!crate::pack::describe_item(&run, &item).lines().any(|line| line == expected), "item grant must omit full ability copy: {id}");
                assert_eq!(progression::definition(id).unwrap().description, original);
            }
            assert_eq!(view.offers.len(), REQUESTED.len());
            let selected_ids: Vec<_> = visible_abilities(&run).into_iter()
                .filter(|row| REQUESTED.iter().any(|(id, _)| *id == row.id))
                .map(|row| row.id).collect();
            assert_eq!(selected_ids, REQUESTED.iter().map(|(id, _)| (*id).to_string()).collect::<Vec<_>>());
            assert_eq!(serde_json::to_vec(&run).unwrap(), before);
        }
    }
}

#[cfg(test)]
mod revision46_wording_refresh_tests {
    use super::*;
    const REQUESTED: &[(&str, &str)] = &[
        ("anavani-illuminate", "Spend 20 energy and select a visible square. Place a permanent fire and light source there."),
        ("anavani-star-power", "You are immune to fire and always burning. You can activate this ability to stop burning."),
        ("kative-adaptive-armor", "Eating a creature's heart grants 3 DR against that creature type. The next two hearts of that type grant 2 and 1 DR, for a maximum of +6 DR."),
        ("kative-artifice-craft", "You can use hearts to Artifice Craft. Multiply the heart's permanent stat bonuses by 5 on the enhanced item while equipped."),
        ("morae-all-seeing-eyes", "Spend 30 energy to reveal the entire current floor, including enemies and contents, until your next turn."),
    ];
    #[test]
    fn exact_authored_copy_reaches_rows_grants_and_offers_without_catalog_mutation() {
        let mut run: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        for revision in [44., 45., 46.] {
            run.simulation_revision = revision.into();
            run.offers = REQUESTED.iter().map(|(id, _)| (*id).into()).collect();
            run.player.abilities = REQUESTED.iter().map(|(id, _)| (*id).into()).collect();
            run.player.adaptive_hearts = corpse_core::Field::Missing;
            let before = serde_json::to_vec(&run).unwrap();
            let view = crate::observe::observe(&run);
            for ((id, requested), offer) in REQUESTED.iter().zip(&view.offers) {
                let original = progression::definition(id).unwrap().description.clone();
                let expected = if revision >= 46. { *requested } else { original.as_str() };
                assert_eq!(display_description(&run, id, &original), expected, "{revision}: {id}");
                let detail = observe(&run, id).description;
                if *id == "kative-adaptive-armor" {
                    assert_eq!(detail, format!("{expected} Current adaptations: None."));
                } else {
                    assert_eq!(detail, expected, "ability detail: {id}");
                }
                assert_eq!(offer.description, detail, "blessing choice: {id}");
                let item = corpse_core::Item { id: 6001., name: "Blessed ring".into(), kind: "ring".into(), granted_ability: (*id).to_string().into(), ..Default::default() };
                assert!(!crate::pack::describe_item(&run, &item).lines().any(|line| line == expected), "item grant must omit full ability copy: {id}");
                assert_eq!(progression::definition(id).unwrap().description, original);
            }
            assert_eq!(view.offers.len(), REQUESTED.len());
            let selected_ids: Vec<_> = visible_abilities(&run).into_iter()
                .filter(|row| REQUESTED.iter().any(|(id, _)| *id == row.id))
                .map(|row| row.id).collect();
            assert_eq!(selected_ids, REQUESTED.iter().map(|(id, _)| (*id).to_string()).collect::<Vec<_>>());
            assert_eq!(serde_json::to_vec(&run).unwrap(), before);
        }
    }
    #[test]
    fn adaptive_suffix_and_approved_unchanged_descriptions_remain_live() {
        let mut run: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        for revision in [45., 46.] {
            run.simulation_revision = revision.into();
            run.player.adaptive_hearts = serde_json::from_value(serde_json::json!({"rat": 2., "pale:rat": 1., "bat": 0.})).unwrap();
            let before = serde_json::to_vec(&run).unwrap();
            let original = &progression::definition("kative-adaptive-armor").unwrap().description;
            let prefix = display_description(&run, "kative-adaptive-armor", original);
            assert_eq!(observe(&run, "kative-adaptive-armor").description,
                format!("{prefix} Current adaptations: rat: 2 hearts, +5 DR. pale:rat: 1 hearts, +3 DR."));
            for id in ["anavani-make-kneel", "kative-divine-craftsman", "morae-turning-the-wheel", "morae-sever-fate"] {
                let original = &progression::definition(id).unwrap().description;
                assert_eq!(observe(&run, id).description, *original);
            }
            assert_eq!(serde_json::to_vec(&run).unwrap(), before);
        }
    }
}

fn r50(r: &Run) -> bool { r.simulation_revision.value_or(29.) >= 50. }
/// Runtime registry, not catalog presence, controls executable UI paths.
pub fn runtime_supported(r: &Run, id: &str) -> bool {
    if matches!(id, corpse_core::dreamwalking::WAKE_UP_ID | corpse_core::dreamwalking::DREAMWALK_ID | corpse_core::ahvamitt::TELEPORT_ID) { return true; }
    corpse_core::abilities::supported_for_run(r, id)
        || id == "morae-see-the-future" || (!r50(r) && id == "numo-revert")
}

#[cfg(test)]
mod revision50_ui_tests {
    use super::*;
    #[test] fn revision50_catalog_copy_and_costs_do_not_leak_into_legacy() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=50.0.into();r.offers.clear();
        for a in progression::catalog_for_revision(50.) {
            let row=observe(&r,&a.id);
            let expected_name=match a.name.as_str() { "Rage at the Dying of the Light"=>"Ragelight", "You Shall Never Be Rid of Me"=>"Never Be Rid of Me", _=>a.name.as_str() };
            assert_eq!(row.name,expected_name);
            // Keep the frozen catalog intact while pinning the two authorized
            // display-unit amendments independently of display_description.
            let expected=match a.id.as_str() {
                "shrail-rage-at-the-dying-of-the-light"=>LEGACY_RAGELIGHT_DESCRIPTION,
                "pelian-never-surrender"=>"Spend 8 energy and add one sixth of your HP bonus to your DR the first time you take damage every turn.",
                "lichor-bleed-them"=>"Your attacks inflict bleed proportional to your Damage bonus.",
                _=>a.description.as_str(),
            };
            assert!(row.description.starts_with(expected),"{}: {}",a.id,row.description);
        }
        assert_eq!(observe(&r,"numo-implant-thought").cost,"Reserve not configured · 15 NRG + action");
        assert!(crate::observe::ability_needs_target_for_run(&r,"pelian-fated-duel"));
        assert!(craft_rows(&r).is_empty());
        r.simulation_revision=46.0.into();assert_eq!(observe(&r,"numo-implant-thought").cost,"30 NRG + action");
    }
    #[test] fn reserve_projection_uses_actual_capacity_and_preserves_state() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=50.0.into();r.offers.clear();r.player.energy=0.;
        let id="numo-implant-thought";
        assert_eq!(payment_blocker(&r,id,15.).as_deref(),Some("Reserve not configured"));
        corpse_core::revision50_ability_energy::initialize_reserve(&mut r.player,id,31.,16.).unwrap();
        let before=r.clone();let row=observe(&r,id);
        assert_eq!(row.cost,"Reserve 16/31 · 15 NRG + action");
        assert!(!row.description.contains("Reserve 16/31"));
        assert!(!row.state.contains("Unavailable"));
        assert!(!row.description.contains("player energy does not"));assert_eq!(r,before);
        r.player.ability_reserves.as_mut().unwrap().get_mut(id).unwrap().energy=0.;r.player.energy=15.;
        assert!(payment_blocker(&r,id,15.).is_none());
        r.player.energy=14.;assert!(payment_blocker(&r,id,15.).unwrap().contains("player energy"));
        r.player.ability_reserves.as_mut().unwrap().get_mut(id).unwrap().energy=1.;r.player.energy=100.;
        assert!(payment_blocker(&r,id,15.).is_none());
        r.simulation_revision=46.0.into();assert!(reserve_summary(&r,id).is_none());
        assert!(!observe(&r,id).description.contains("Reserve"));
    }
}

#[cfg(test)]
mod revision50_future_ui_tests {
    use super::*;
    #[test]
    fn future_shows_own_and_shared_drain_and_reserve_only_silence() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=50.0.into();r.offers.clear();r.player.energy=0.;r.player.conditions.clear();
        let id=corpse_core::revision50_ability_energy::FUTURE_ID;
        r.player.abilities=vec![id.into()];
        assert!(runtime_supported(&r,id));assert!(!runtime_supported(&r,"numo-revert"));
        assert_eq!(payment_blocker(&r,id,0.).as_deref(),Some("Cannot save this state"));
        corpse_core::revision50_ability_energy::acquire(&mut r.player,&[id.into()]).unwrap();
        let row=observe(&r,id);assert!(row.supported);assert!(row.cost.contains("All own reserve + 0 player NRG"));
        assert!(!row.state.contains("Unavailable"));assert!(!row.description.contains("leaving other reserves untouched"));
        r.player.conditions.insert("silenced".into(),1.);assert!(observe(&r,id).state.contains("Unavailable"));
        r.player.ability_reserves.as_mut().unwrap()[id].energy=0.;assert!(!observe(&r,id).state.contains("Unavailable"));
        r.future=Some("not decoded by UI".to_string().into());r.player.energy=20.;
        let row=observe(&r,id);assert!(row.cost.contains("Free return"));assert!(!row.state.contains("Unavailable"));
    }
}

#[cfg(test)]
mod revision52_hanged_ui_tests {
    use super::*;
    #[test]
    fn passive_hanged_has_no_toggle_binding_reaction_counter_or_reserve_cost() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.offers.clear();r.player.abilities=vec![corpse_core::hanged::ID.into(),"isheilah-quick-hand".into()];
        r.player.active=vec![corpse_core::hanged::ID.into()];r.player.reactions=1.0.into();
        for revision in [51.,52.] {
            r.simulation_revision=revision.into();let row=observe(&r,corpse_core::hanged::ID);
            assert_eq!(row.name,if revision==52.{"Hanged"}else{"Missed Me"});
            assert_eq!(command_binding_allowed(&r,corpse_core::hanged::ID),revision<52.);
            if revision==52. {
                assert!(!row.active);assert!(row.cost.contains("4 main NRG/HP"));
                assert!(!row.cost.to_lowercase().contains("reaction"));assert!(!row.cost.contains("Reserve"));
                assert!(!row.state.contains("ON") && !row.state.contains("OFF"));
                let quick=observe(&r,"isheilah-quick-hand");assert!(quick.cost.contains("1/1 reactions"));
                assert!(quick.description.contains("Waiting restores 1 reaction"));
            }
        }
    }
}

#[cfg(test)]
mod never_surrender_description_tests {
    use super::*;
    #[test]
    fn exact_requested_r52_description_preserves_cost_and_legacy_copy() {
        let mut run: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        run.simulation_revision = 52.0.into();
        assert_eq!(display_description(&run, "pelian-never-surrender", "legacy"),
            "Increase your DR for the first source of damage every turn.");
        let row = observe(&run, "pelian-never-surrender");
        assert!(row.description.starts_with("Increase your DR for the first source of damage every turn."));
        assert!(row.cost.contains("8 NRG"), "{}", row.cost);
        run.player.abilities = vec!["pelian-never-surrender".into()];
        run.offers = vec!["pelian-never-surrender".into()];
        let requested = "Increase your DR for the first source of damage every turn.";
        assert!(crate::observe::observe(&run).offers[0].description.starts_with(requested));
        let item = corpse_core::Item { id: 6001., name: "Blessed ring".into(), kind: "ring".into(),
            granted_ability: "pelian-never-surrender".to_string().into(), ..Default::default() };
        assert!(!crate::pack::describe_item(&run, &item).contains(requested));
        assert!(crate::creature_abilities_ui::describe(&run, &run.player).1.contains(requested));
        run.simulation_revision = 51.0.into();
        assert_eq!(display_description(&run, "pelian-never-surrender", "legacy"),
            "Spend 8 energy and add one sixth of your HP bonus to your DR the first time you take damage every turn.");
    }
}

#[cfg(test)]
mod never_surrender_detail_followup_tests {
    use super::*;
    const ID: &str = "pelian-never-surrender";
    const OPENING: &str = "Increase your DR for the first source of damage every turn.";
    const EXCEPTION: &str = "Damage that ignores DR spends no energy and leaves this activation available.";

    fn room(revision: f64) -> Run {
        let mut run: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        run.simulation_revision = revision.into();
        run.player.abilities = vec![ID.into()]; run.player.active = vec![ID.into()];
        run.player.conditions.clear(); run.offers = vec![ID.into()];
        run
    }

    fn review_actual_detail_dialog() {
        let mut run = room(52.); run.offers.clear();
        run.player.ability_reserves = corpse_core::Field::Missing;
        corpse_core::revision50_ability_energy::reconcile_acquired(&mut run).unwrap();
        corpse_core::resources::begin_resource_turn(&mut run);
        let player_id = run.player.id;
        assert!(corpse_core::revision50_ability_energy::spend(&mut run, player_id, ID, 8.));
        let before = serde_json::to_vec(&run).unwrap();
        let row = observe(&run, ID);
        assert!(row.cost.contains("Reserve 2/10 - -80%"), "{}", row.cost);
        assert!(!row.description.contains("Ability energy refills to"));
        let view = crate::observe::observe(&run);
        for (width, height) in [(1280, 800), (960, 1200)] {
            let mut painter = crate::render::Painter::new();
            painter.try_resize(width, height).unwrap(); painter.set_responsive_canvas(true);
            painter.game_at(&view, "", false, 0.);
            let background = painter.pixels.clone();
            let bounds = painter.ability_detail_damage(&row).unwrap();
            let rail = painter.logical_damage_rect(painter.hud_left(), 0, 2, 2).unwrap().x;
            let top = painter.logical_damage_rect(0, (super::top(&painter.profile) - 28).max(8), 2, 2).unwrap().y;
            let footer = painter.logical_damage_rect(0, painter.inspection_notice_y(), 2, 2).unwrap().y;
            assert_eq!(bounds.right(), rail, "dialog right edge must touch the ability rail");
            assert_eq!(bounds.y, top);
            assert!(bounds.x > 0 && bounds.bottom() < footer);
            let mut short = row.clone(); short.description = OPENING.into();
            assert!(bounds.h > painter.ability_detail_damage(&short).unwrap().h,
                "dialog height must reflect the complete detail body");
            let prior_clip = painter.clip;
            painter.ability_detail(&row, 0);
            assert_eq!(painter.clip, prior_clip);
            for (i, pixel) in painter.pixels.chunks_exact(4).enumerate() {
                if !bounds.contains(i % painter.width, i / painter.width) {
                    assert_eq!(pixel, &background[i*4..i*4+4], "paint escaped exact dialog bounds");
                }
            }
            let first_page = painter.pixels.clone();
            painter.game_at(&view, "", false, 0.); painter.ability_detail(&row, usize::MAX);
            assert_eq!(painter.pixels, first_page, "complete detail must fit without clipped or scroll-hidden text");
            let geometry = serde_json::json!({"width": width, "height": height,
                "dialog": {"x":bounds.x,"y":bounds.y,"width":bounds.w,"height":bounds.h},
                "rail_x":rail,"footer_y":footer,"all_text_fits":true,"outside_bounds_unchanged":true,
                "opening":OPENING,"exception":EXCEPTION,"reserve":"Reserve 2/10 - -80%","refill":10});
            println!("Never Surrender detail: {geometry}");
            if let Ok(directory) = std::env::var("CORPSE_NEVER_SURRENDER_DETAIL_OUTPUT") {
                assert!(std::path::Path::new(&directory).is_absolute(), "capture destination must be absolute");
                std::fs::create_dir_all(&directory).unwrap();
                let stem = format!("{directory}/never-surrender-detail-{width}x{height}");
                let mut encoder = png::Encoder::new(std::fs::File::create(format!("{stem}.png")).unwrap(), width as u32, height as u32);
                encoder.set_color(png::ColorType::Rgba); encoder.set_depth(png::BitDepth::Eight);
                encoder.write_header().unwrap().write_image_data(&painter.pixels).unwrap();
                std::fs::write(format!("{stem}.json"), serde_json::to_vec_pretty(&geometry).unwrap()).unwrap();
            }
        }
        assert_eq!(serde_json::to_vec(&run).unwrap(), before);
    }

    #[test]
    fn full_detail_explains_bypass_after_the_exact_unmodified_short_description() {
        let mut run = room(52.);
        let before = serde_json::to_vec(&run).unwrap();
        assert_eq!(display_description(&run, ID, "legacy"), OPENING);
        let row = observe(&run, ID);
        let mut lines = row.description.lines();
        assert_eq!(lines.next(), Some(OPENING));
        assert_eq!(lines.next(), Some(EXCEPTION));
        assert!(row.cost.contains("8 NRG/first source"));
        assert!(crate::observe::observe(&run).offers[0].description.contains(EXCEPTION));
        let item = corpse_core::Item { id: 6001., name: "Blessed ring".into(), kind: "ring".into(),
            granted_ability: ID.to_string().into(), ..Default::default() };
        assert!(!crate::pack::describe_item(&run, &item).contains(EXCEPTION));
        assert!(!observe(&run, "numo-distract").description.contains(EXCEPTION));
        assert_eq!(serde_json::to_vec(&run).unwrap(), before);
        run.simulation_revision = 51.0.into();
        assert!(!observe(&run, ID).description.contains(EXCEPTION));
        review_actual_detail_dialog();
    }

    #[test]
    fn reserve_refill_detail_uses_revision_capacity_for_all_paid_ability_kinds() {
        for revision in [50., 51., 52.] {
            let run = room(revision);
            let before = serde_json::to_vec(&run).unwrap();
            for id in [ID, "numo-distract", corpse_core::revision50_ability_energy::FUTURE_ID] {
                let description = observe(&run, id).description;
                assert!(!description.contains("Ability energy refills to"),
                    "revision={revision}, id={id}: {description}");
                if revision >= 52. { assert!(!description.contains("refills to 50")); }
            }
            assert_eq!(serde_json::to_vec(&run).unwrap(), before);
        }
        let run = room(49.);
        assert!(!observe(&run, ID).description.contains("Ability energy refills to"));
    }
}

#[cfg(test)] mod reserve_bar_projection_tests {
    use super::*;
    #[test] fn reserve_bar_forecast_uses_canonical_debit_and_leaves_run_unchanged(){
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();r.offers.clear();r.player.active.clear();r.player.ability_reserves=corpse_core::Field::Missing;
        let id="morae-turning-the-wheel";r.player.abilities.push(id.into());r.player.active.push(id.into());r.player.energy=50.;
        corpse_core::revision50_ability_energy::initialize_reserve(&mut r.player,id,10.,7.).unwrap();
        let before=r.clone();let row=observe(&r,id);
        assert_eq!(row.reserve,Some((7.,10.)));assert_eq!(row.pending_reserve_loss,7.);
        assert_eq!(corpse_core::abilities::projected_player_upkeep(&r),13.);assert_eq!(r,before);
        r.player.active.clear();assert_eq!(observe(&r,id).pending_reserve_loss,0.);
        r.player.ability_reserves.as_mut().unwrap()[id].energy=11.;assert_eq!(observe(&r,id).reserve,None);
        assert_eq!(observe(&r,"shrail-masochism").reserve,None);
    }
}

#[cfg(test)] mod ragelight_presentation_tests {
    use super::*;
    #[test] fn rename_is_display_only_and_preserves_proc_cost_and_save(){
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();let id="shrail-rage-at-the-dying-of-the-light";
        r.player.abilities=vec![id.into()];r.player.conditions.insert("rageCost".into(),17.);
        let before=serde_json::to_string(&r).unwrap();let row=observe(&r,id);
        assert_eq!(row.name,"Ragelight");assert_eq!(row.id,id);assert!(row.cost.contains("17 NRG/rescue"));
        assert_eq!(corpse_core::progression::definition_for_revision(52.,id).unwrap().name,"Rage at the Dying of the Light");
        assert_eq!(crate::chronicle::present("Rage at the Dying of the Light activates."),"Ragelight activates.");
        let (short,details)=crate::creature_abilities_ui::describe(&r,&r.player);
        assert!(!short.contains("Rage at the Dying of the Light"));assert!(!details.contains("Rage at the Dying of the Light"));
        assert!(details.contains("Ragelight"));
        for copy in [short, details] {
            assert!(!copy.contains("When active") && !copy.contains("(inactive)") && copy.contains("Next rescue: 17 NRG."));
        }
        let item=corpse_core::Item{kind:"ring".into(),granted_ability:id.to_string().into(),..Default::default()};
        assert!(crate::pack::describe_item(&r,&item).contains("Grants Ragelight."));
        assert_eq!(serde_json::to_string(&r).unwrap(),before);
        assert!(matches!(display_text("@"),std::borrow::Cow::Borrowed(_)));
    }
}

#[cfg(test)] mod compact_spacing_tests {
    use super::*;
    #[test] fn halve_spare_ability_margin_and_keep_hit_testing_aligned() {
        let profile=crate::appearance_profile::Profile::defaults();
        assert_eq!(row(&profile),35);
        for(w,h)in[(1280,800),(1920,1080),(960,1200)] {
            let mut p=crate::render::Painter::new();p.resize(w,h);p.set_responsive_canvas(true);
            let count=capacity(&p.profile);
            for i in 0..count {
                let y=top(&p.profile)+i as i32*row(&p.profile);
                assert_eq!(p.ability_inspect_at(count,p.hud_left()+20,y+10),Some(i));
            }
        }
    }
}

#[cfg(test)] mod ragelight_final_copy_tests {
    use super::*;
    #[test] fn passive_ragelight_keeps_cost_and_reserve_but_no_toggle() {
        let mut r: Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();
        let id=corpse_core::divine_rescues::RAGE;
        r.player.abilities=vec![id.into()];
        r.player.active=vec![id.into()];
        r.player.conditions.insert("rageCost".into(),17.);
        corpse_core::revision50_ability_energy::reconcile_acquired(&mut r).unwrap();
        let row=observe(&r,id);
        assert_eq!(row.mode,"Passive");
        assert!(row.state.is_empty());
        assert!(!row.active);
        assert!(!command_binding_allowed(&r,id));
        assert!(row.cost.contains("17 NRG/rescue"));
        assert!(row.reserve.is_some());
        assert_eq!(row.indicator,Some(CustomIndicator::RagelightCost(17.0f64.to_bits())));
        assert_eq!(row.description,RAGELIGHT_DESCRIPTION);

        r.ragelight_rules_from=corpse_core::Field::Null;
        let row=observe(&r,id);
        assert_eq!(row.mode,"Toggle");
        assert!(row.active);
        assert!(command_binding_allowed(&r,id));
        assert_eq!(row.description,LEGACY_RAGELIGHT_DESCRIPTION);
    }

    #[test] fn exact_copy_has_no_appended_reserve_explanation_or_mechanics_mutation(){
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let id="shrail-rage-at-the-dying-of-the-light";r.simulation_revision=52.0.into();r.player.abilities=vec![id.into()];
        corpse_core::revision50_ability_energy::reconcile_acquired(&mut r).unwrap();
        for energy in [0.,100.] {r.player.energy=energy;let before=r.clone();assert_eq!(observe(&r,id).description,RAGELIGHT_DESCRIPTION);assert!(r==before);}
    }
}

#[cfg(test)] mod execute_final_copy_tests {
    use super::*;
    #[test] fn current_execute_cost_and_exact_description_match_payment(){
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();
        let id="shrail-execute";r.player.abilities=vec![id.into()];corpse_core::revision50_ability_energy::reconcile_acquired(&mut r).unwrap();
        let row=observe(&r,id);assert_eq!(row.description,EXECUTE_DESCRIPTION);assert!(row.cost.contains("10 NRG/execute"));assert!(!row.cost.contains("20 NRG/execute"));
        r.execute_rules_from=corpse_core::Field::Null;
        assert_eq!(observe(&r,id).description,LEGACY_EXECUTE_DESCRIPTION);
    }
}

#[cfg(test)]mod audit_batch_two_tests {
 use super::*;
 #[test]fn requested_short_copy_has_no_stale_overrides(){
  let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();
  for id in ["isheilah-quick-hand","lichor-blood-weapon","anavani-nova","morae-all-seeing-eyes"] {r.player.abilities.push(id.into());}
  r.player.ability_reserves=corpse_core::Field::Missing;corpse_core::revision50_ability_energy::initialize_reserve(&mut r.player,"morae-all-seeing-eyes",10.,10.).unwrap();
  assert!(!observe(&r,"isheilah-quick-hand").description.contains("No energy cost."));
  assert!(observe(&r,"lichor-blood-weapon").description.starts_with("Activate by paying 10% of your maximum HP. Your next attack"));
  let nova=observe(&r,"anavani-nova").description;assert!(nova.contains("Every explosion deals 50% of each affected creature's maximum HP."));assert!(!nova.contains("unless it is immune"));
  assert_eq!(observe(&r,"morae-all-seeing-eyes").description,"Spend 30 energy to reveal the entire current floor, including enemies and contents, until your next turn.");
  for diluted in [false,true]{let n=if diluted{25}else{50};assert_eq!(corpse_core::revision50_item_knowledge::potion_effect_description("healing",diluted).unwrap(),format!("Restores {n} HP."));assert_eq!(corpse_core::revision50_item_knowledge::potion_effect_description("energy",diluted).unwrap(),format!("Restores {n} energy."));}
  assert_eq!(corpse_core::revision50_item_knowledge::potion_effect_description("cleansing",false).unwrap(),"Removes harmful temporary effects from the affected creature.");
 }
}

#[cfg(test)] mod audit_batch_three_tests {
 use super::*;
 #[test] fn future_unique_paragraph_and_appropriate_copy(){
  let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();
  let id="morae-see-the-future";r.player.abilities=vec![id.into(),"kative-appropriate".into()];
  r.player.ability_reserves=corpse_core::Field::Missing;
  corpse_core::revision50_ability_energy::initialize_reserve(&mut r.player,id,10.,10.).unwrap();
  for energy in [0.,100.] {r.player.energy=energy;let before=r.clone();let d=observe(&r,id).description;
   assert_eq!(d,FUTURE_FLOOR_DESCRIPTION);
   for stale in ["other reserves","after payment","Ability energy is spent first"] {assert!(!d.contains(stale));}
   assert!(!observe(&r,"kative-appropriate").description.contains("minimum of zero"));assert!(r==before);
  }
 }
}

#[cfg(test)] mod approved_copy_cuts_tests {
    use super::*;
    #[test] fn clay_and_tinker_keep_rules_without_repeated_tutorial() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();
        r.player.abilities=vec!["kative-clay-army".into(),"kative-tinker".into()];
        let clay=observe(&r,"kative-clay-army").description;
        assert_eq!(clay.lines().filter(|line| *line == "It is an unbreakable wall.").count(), 1);
        assert!(!clay.contains("Each Clay Soldier uses"));
        assert!(clay.contains("spend 20 energy"));
        let tinker=observe(&r,"kative-tinker").description;
        assert!(!tinker.contains("automatically active"));
        assert!(tinker.contains("Spend 100 energy"));
        assert!(tinker.contains("Each item can be tinkered once"));
    }
}


#[cfg(test)]
mod never_be_rid_name_tests {
    #[test]
    fn rename_keeps_internal_identity() {
        let mut r: corpse_core::Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 52.0.into();
        let id = "pelian-you-shall-never-be-rid-of-me";
        let before = serde_json::to_string(&r).unwrap();
        assert_eq!(super::observe(&r,id).name,"Never Be Rid of Me");
        assert_eq!(super::display_text("You Shall Never Be Rid of Me activates."),"Never Be Rid of Me activates.");
        assert_eq!(serde_json::to_string(&r).unwrap(),before);
    }
}

#[cfg(test)]
mod divine_balance_presentation_tests {
    use super::*;

    #[test]
    fn half_here_description_explains_the_five_turn_lock() {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 52.0.into();
        let id = corpse_core::half_here::ID;
        r.player.abilities.push(id.into());
        assert!(!observe(&r, id).description.contains("5 turns"));
        r.half_here_lock_from = 0usize.into();
        assert!(observe(&r, id).description.contains("at least 5 turns before you can dismiss it"));
    }

    #[test]
    fn half_here_paid_placement_updates_copy_and_reserve_but_dismissal_is_free() {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision = 52.0.into();
        let id = corpse_core::half_here::ID;
        r.player.abilities.push(id.into());
        corpse_core::half_here::admit_current(&mut r).unwrap();
        corpse_core::revision50_ability_energy::initialize_reserve(&mut r.player, id, 10., 10.).unwrap();
        let row = observe(&r, id);
        assert!(row.description.contains("Spend 5 energy"));
        assert!(row.description.contains("at any time"));
        assert!(row.cost.contains("5 NRG"));
        assert_eq!(row.reserve, Some((10., 10.)));
        r.player.active.push(id.into());
        assert!(observe(&r, id).cost.contains("Free dismissal"));
    }

    #[test]
    fn shows_five_saves_instead_of_legacy_blood_and_the_new_ursa_prices() {
        let mut r: Run = serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();
        r.balance_rules_from=0usize.into();
        r.player.abilities=["lichor-overflowing","shrail-fight-long","ursa-sudden-sleep",
            "ursa-dream-of-time-gone-by","ursa-sleep-walk"].map(str::to_owned).to_vec();
        let row=observe(&r,"lichor-overflowing");
        assert_eq!(row.indicator,Some(CustomIndicator::OverflowingSaves(5)));
        assert!(row.description.contains("full HP or leveling up"));
        r.player.conditions.insert("overflowingSaves".into(), 2.);
        assert_eq!(observe(&r,"lichor-overflowing").indicator,Some(CustomIndicator::OverflowingSaves(2)));
        r.player.conditions.insert("overflowingSaves".into(), 0.);
        assert_eq!(observe(&r,"lichor-overflowing").indicator,Some(CustomIndicator::OverflowingSaves(0)));
        corpse_core::divine_rescues::refill_on_level_up(&mut r);
        assert_eq!(observe(&r,"lichor-overflowing").indicator,Some(CustomIndicator::OverflowingSaves(5)));
        assert!(observe(&r,"shrail-fight-long").description.contains("3%"));
        assert!(observe(&r,"ursa-sudden-sleep").description.contains("25 turns"));
        let dream=observe(&r,"ursa-dream-of-time-gone-by");
        assert!(dream.description.contains("20 energy"));
        assert!(!dream.cost.contains("50"));
        assert!(observe(&r,"ursa-sleep-walk").description.contains("below 10%"));
        r.balance_rules_from=corpse_core::Field::Missing;
        assert!(matches!(observe(&r,"lichor-overflowing").indicator,Some(CustomIndicator::Blood{..})));
    }
}
