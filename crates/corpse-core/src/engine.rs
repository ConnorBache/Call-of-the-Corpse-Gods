//! Capture pilot note: disabled hooks O(1); enabled actor-group marks O(log G + key bytes),
//! unrestricted-boundary flags O(log B + label bytes). These are access counts, not writes.
//! Command boundary. Unsupported branches fail transactionally, never become no-ops.
use crate::{
    ai, combat,
    model::*,
    movement::{self, Point},
    resources, rng, vision,
};
use std::fmt;
#[derive(Debug, Clone, PartialEq)]
pub struct EngineError(pub String);
impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unsupported native behavior: {}", self.0)
    }
}
impl std::error::Error for EngineError {}
#[derive(Debug, Clone, Default)]
pub struct DispatchOutcome {
    pub took_turn: bool,
    pub changed: bool,
    pub events: Vec<crate::events::PresentationEvent>,
    pub pending_snapshot: Option<crate::temporal::Snapshot>,
}
fn unsupported<T>(s: impl Into<String>) -> Result<T, EngineError> {
    Err(EngineError(s.into()))
}
pub fn condition(a: &Actor, k: &str) -> f64 {
    a.conditions.get(k).copied().unwrap_or(0.0)
}
/// O(d) indexed lookup; first lookup after structural mutation O((A+P+B)*d+F+V).
/// Invalid identity worlds yield None and are rejected by dispatch admission.
pub fn actor(r: &Run, id: f64) -> Option<&Actor> {
    if id == r.player.id { Some(&r.player) } else {
        let (floor,at)=r.actors.position(id,r.player.id,r.depth,true).ok()??;
        r.actors.get_index(floor)?.1.get(at)
    }
}
/// O(d) global lookup; structural reconciliation O((A+P+B)*d+F+V), no floor scan.
pub(crate) fn actor_any(r: &Run,id:f64)->Option<&Actor> {
    if id==r.player.id {return Some(&r.player)}
    let (floor,at)=r.actors.position(id,r.player.id,r.depth,false).ok()??;r.actors.get_index(floor)?.1.get(at)
}
/// O(d) indexed lookup plus O(floor actor payload) on first Arc COW write.
/// Internal callers must preserve identity/nested pilot/order invariants.
pub(crate) fn actor_any_mut(r:&mut Run,id:f64)->Option<&mut Actor> {
    if id==r.player.id{crate::capture_pilot::mark_player_access();return Some(&mut r.player)}
    let (floor,at)=r.actors.position(id,r.player.id,r.depth,false).ok()??;
    r.actors.trusted_index_mut(floor)?.get_mut(at)
}
/// O(d) indexed lookup plus first-write Arc COW; preserves identity internally.
pub(crate) fn actor_mut_trusted(r: &mut Run,id:f64)->Option<&mut Actor> {
    if id==r.player.id{crate::capture_pilot::mark_player_access();return Some(&mut r.player)}
    let (floor,at)=r.actors.position(id,r.player.id,r.depth,true).ok()??;
    r.actors.trusted_index_mut(floor)?.get_mut(at)
}
/// Public mutable access: O(d) lookup plus COW; invalidates before exposing ID.
/// Next lookup reconciles O((A+P+B)*d+F+V), including same-length replacement/ID edits.
pub fn actor_mut(r:&mut Run,id:f64)->Option<&mut Actor> {
    if id==r.player.id {crate::capture_pilot::mark_player_access();crate::capture_pilot::mark_unsupported("unrestricted_actor_identity");r.actors.invalidate();return Some(&mut r.player)}
    let (floor,at)=r.actors.position(id,r.player.id,r.depth,true).ok()??;
    r.actors.invalidate();r.actors.trusted_index_mut(floor)?.get_mut(at)
}
pub fn log(r: &mut Run, message: impl Into<String>, kind: &str) {
    let message = revision43_message(message.into(), r.simulation_revision.value_or(29.));
    let kinds = r.log_kinds.get_or_insert_with(Vec::new);
    kinds.truncate(r.log.len());
    while kinds.len() < r.log.len() {
        kinds.push("unknown".into());
    }
    if r.log_archive.as_ref().is_none() {
        let mut a = LogArchive {
            version: 1.0,
            messages: vec![],
            events: vec![],
            incomplete: true,
        };
        for (m, k) in r.log.iter().zip(kinds.iter()) {
            archive_event(&mut a, m, k);
        }
        r.log_archive = Field::Value(a);
    }
    archive_event(r.log_archive.as_mut().unwrap(), &message, kind);
    r.log.push(message);
    r.log_kinds.as_mut().unwrap().push(kind.into());
    if r.log.len() > 80 {
        r.log.remove(0);
        r.log_kinds.as_mut().unwrap().remove(0);
    }
}

/// O(message bytes) for the fixed exact comparisons/replacement; no state, RNG,
/// or cause-of-death changes. Legacy recording revisions retain their text.
fn revision43_message(message: String, revision: f64) -> String {
    if revision < 43. { return message; }
    match message.as_str() {
        "Silence prevents this energy-paid activation." => "Your ability is silenced and does not work.".into(),
        "No eligible outside-god abilities remain for Tinker." => "You can't think of any new ideas to tinker with...".into(),
        "You died unpreventably." => "You died.".into(),
        _ => message,
    }
}
fn archive_event(a: &mut LogArchive, m: &str, k: &str) {
    let id = a.messages.iter().position(|s| s == m).unwrap_or_else(|| {
        a.messages.push(m.into());
        a.messages.len() - 1
    });
    let kind = ["player", "enemy", "environment", "unknown"]
        .iter()
        .position(|v| *v == k)
        .unwrap_or(3);
    a.events.push((id * 4 + kind) as f64);
}
fn bump(r: &mut Run, k: &str, n: f64) {
    *r.metrics.entry(k.into()).or_default() += n;
}
/// Preflight is conservative: each admitted mechanic must have an executable branch.
pub fn validate_supported(r: &Run) -> Result<(), EngineError> {
    validate_supported_for(r, &[])
}
fn validate_supported_for(r: &Run, pending_falls: &[f64]) -> Result<(), EngineError> {
    let _timing_function = crate::timing::span("command.validate");
    if !crate::fast_mode::valid_configuration(r) {
        return unsupported("invalid Fast mode flag/revision");
    }
    let Some(f) = r.floor() else {
        return unsupported("active floor missing");
    };
    if f.width() == 0 || f.tiles.len() != f.width() * f.height() {
        return unsupported("invalid floor geometry");
    }
    crate::revision50_amulets::validate_run(r).map_err(EngineError)?;
    crate::revision50_amulet_effects::validate_statuses(r).map_err(EngineError)?;
    crate::revision52_enemy_followups::validate_run(r).map_err(EngineError)?;
    crate::revision50_control_amulets::validate_terrain(r).map_err(EngineError)?;
    crate::offscreen::validate_supported(r).map_err(EngineError)?;
    for a in std::iter::once(&r.player).chain(r.actors_at(r.depth)) {
        if a.active.iter().any(|id| {
            // R50 player-owned/copy powers must belong to this revision's
            // executable catalog, even when a stale active ID lacks ownership.
            // O(72 * ID bytes) per checked power; NPC trait admission stays below.
            (crate::revision50::enabled(r)
                && (crate::half_here::owner_id(r,a)==r.player.id || condition(a,"miniMe")==1.0)
                && !crate::abilities::supported_for_run(r,id))
                || (crate::half_here::owner_id(r, a) != r.player.id && !(crate::revision50::enabled(r) && condition(a,"miniMe")==1.0))
                || !(crate::revision50::enabled(r) && crate::revision50::toggle(id)) && !matches!(
                    id.as_str(),
                    "shrail-unyielding-assault"
                        | "isheilah-missed-me"
                        | "pelian-never-without-a-fight"
                        | "etill-float-away"
                        | "numo-mind-read"
                        | "ursa-dream-see"
                        | "dawel-shadow-in-the-dark"
                        | "dawel-not-there"
                        | "grisha-entropy"
                        | "anavani-star-power"
                        | "etill-half-here"
                        | "anavani-shine"
                        | "anavani-nova"
                        | "pelian-never-surrender"
                        | "isheilah-quick-hand"
                        | "isheilah-are-you-sure-about-that"
                        | "ursa-waking-nightmare"
                        | "ursa-dream-of-time-gone-by"
                        | "isheilah-make-slippery"
                        | "etill-phase-through"
                        | "etill-rushing-torrent"
                        | "etill-riptide"
                        | "morae-bonded-by-unseen-threads"
                        | "dawel-unspeak"
                        | "grisha-erase-metaphysical"
                        | "grisha-uncast"
                        | "kative-mini-me"
                        | "morae-turning-the-wheel"
                        | "grisha-black-pit"
                        | "dawel-reverse-entropy"
                        | "shrail-rage-at-the-dying-of-the-light"
                        | "pelian-you-shall-never-be-rid-of-me"
                )
        }) {
            return unsupported(format!("active abilities on {}", a.name));
        }
        let inferred_traits = crate::enemy_traits::enemy_traits(a);
        if a.abilities.iter().chain(inferred_traits.iter()).any(|id| {
            if condition(a, "miniMe") == 1.0 {
                if crate::revision50::enabled(r) { !crate::abilities::supported_for_run(r,id) && !crate::abilities::copied_trait_supported(id) && id!="morae-see-the-future" }
                else { !crate::abilities::mini_me_ability_supported(id) }
            } else if crate::half_here::owner_id(r, a) != r.player.id {
                !(crate::revision50::enabled(r) && id==crate::revision50_combat::MAGICKAL_CASTER) && !matches!(
                    id.as_str(),
                    "enemy:beast:slow"
                        | "enemy:pale:slow"
                        | "enemy:beast:fast"
                        | "enemy:beast:flight"
                        | "enemy:beast:ichor-bound"
                        | "enemy:beast:ichor-siphon"
                        | "enemy:beast:venom-weak"
                        | "enemy:beast:venom"
                        | "enemy:beast:venom-strong"
                        | "enemy:beast:slime-walker"
                        | "enemy:beast:rooted"
                        | "enemy:beast:fungal-gas"
                        | "enemy:beast:gas-immunity"
                        | "enemy:beast:no-revival"
                        | "enemy:beast:wendigo"
                        | "enemy:beast:stalk"
                        | "enemy:beast:horn-bolt"
                        | "enemy:beast:blink"
                        | "enemy:beast:moon-laser"
                        | "enemy:beast:beckon"
                        | "enemy:beast:wall-seal"
                        | "enemy:beast:bloodmancy"
                        | "enemy:beast:witch"
                        | "enemy:beast:saidribee-queen"
                        | "enemy:beast:dryad"
                        | "enemy:beast:inventor"
                        | "enemy:beast:mech"
                        | "enemy:beast:grab"
                        | "enemy:beast:hold-only"
                        | "enemy:beast:deep-water"
                        | "enemy:beast:water-jet"
                        | "enemy:beast:aquatic"
                        | "enemy:beast:submerge"
                        | "enemy:beast:yeti"
                        | "enemy:beast:phoenix"
                        | "enemy:beast:armor-piercing"
                        | "enemy:beast:ursa-major"
                        | "enemy:beast:detonate"
                        | "enemy:beast:explosion-immunity"
                        | "enemy:beast:slime-bound"
                        | "enemy:beast:slime-trail"
                        | "enemy:beast:razorleaves"
                        | "enemy:beast:sphinx"
                        | "enemy:beast:ruin-pulse"
                        | "enemy:beast:silence-aura"
                        | "enemy:beast:darkness-aura"
                        | "enemy:beast:fire-breath"
                        | "enemy:beast:everburn"
                        | "enemy:beast:fire-immunity"
                        | "enemy:beast:vampire"
                        | "enemy:beast:lifesteal"
                        | "enemy:beast:regeneration"
                        | "enemy:beast:banshee"
                        | "enemy:beast:ursa-minor"
                        | "enemy:beast:minotaur"
                        | "enemy:pale:gas-immunity"
                        | "enemy:pale:bite-repeat"
                        | "enemy:pale:lifesteal"
                        | "enemy:pale:scream"
                        | "enemy:pale:breath"
                        | "enemy:pale:breath-boost"
                        | "enemy:pale:gas-vulnerability"
                        | "enemy:pale:spit"
                        | "enemy:pale:gas-shot"
                        | "enemy:pale:birth"
                        | "enemy:pale:double-birth"
                )
            } else {
                !crate::abilities::supported_for_run(r,id)
                    && !crate::abilities::copied_trait_supported(id)
                    && !(id == "numo-revert" && !crate::revision50::enabled(r))
                    // Future is owned by temporal dispatch, not ordinary activation.
                    && id != "morae-see-the-future"
            }
        }) {
            return unsupported(format!("ability hooks on {}: {:?}", a.name, a.abilities));
        }
        for k in a.conditions.keys() {
            if crate::balance52::condition_key(k) && (r.simulation_revision.value_or(29.) != 52.
                || !crate::balance52::valid_condition(k, a.conditions[k])) {
                return unsupported(format!("invalid R52 birth coefficient on {}: {k}", a.name));
            }
            if !(r.simulation_revision.value_or(29.) == 52. && crate::balance52::condition_key(k))
                && !(crate::revision52_enemy_followups::enabled(r) && crate::revision52_enemy_followups::condition_key(k))
                && !(crate::revision50::enabled(r) && crate::revision50::condition_key(k))
                && !k.starts_with("acquired:")
                && !k.strip_prefix("unequipped:").is_some_and(|id| {
                    crate::abilities::supported(id)
                        || ["numo-revert", "morae-see-the-future"].contains(&id)
                })
                && !k.strip_prefix("remembered:").is_some_and(|id| {
                    crate::abilities::supported(id) || crate::abilities::copied_trait_supported(id)
                })
                && !k.starts_with("buff:")
                && !k.starts_with("buffTicks:")
                && !k.starts_with("venom:")
                && ![
                    "telepathic",
                    "telepathicFresh",
                    "invisible",
                    "invisibleFresh",
                    "hasted",
                    "hastedFresh",
                    "paralyzed",
                    "paralyzedFresh",
                    "flying",
                    "flyingFresh",
                    "bleeding",
                    "bleedingFresh",
                    "bleedSourceId",
                    "nightmareAt",
                    "surrenderTick",
                    "dreamX",
                    "dreamY",
                    "dreamDepth",
                    "slipperyTarget",
                    "riptide",
                    "riptideTarget",
                    "bondedTarget",
                    "riptideX",
                    "riptideY",
                    "silenced",
                    "auraSilenced",
                    "pelianUsed",
                    "bloodWeaponReady",
                    "beatToDeathHits",
                    "ahvamitt",
                    "ahvamittFinds",
                    "ahvamittTeleportReadyAt",
                    "phantasmiiNightmare",
                    "nightmareTier",
                    "phantasmiiBody",
                    "phantasmiiPlane",
                    "halfHereClone",
                    "halfHereOwner",
                    "halfHereId",
                    "fungalGasType",
                    "stuck",
                    "weakened",
                    "emboldened",
                    "seesPlayer",
                    "targetId",
                    "creatureHungerTurns",
                    "creatureHungerClock",
                    "creatureFeedingTick",
                    "creatureFeedingTurns",
                    "creatureFeedingSource",
                    "creatureFeedingKind",
                    "creatureFeedingX",
                    "creatureFeedingY",
                    "creatureFeedingInterruptedAt",
                    "monsterUnseenTurns",
                    "monsterSurprisedThrough",
                    "monsterRecoverySleep",
                    "monsterAllyRest",
                    "bestiaryTraitsInitialized",
                    "paleTraitsInitialized",
                    "paleTraitsInitialized",
                    "stealth",
                    "weaponDamage",
                    "weaponOffenseMultiplier",
                    "offenseMultiplier",
                    "tier",
                    "enemyKind",
                    "hemophilia",
                    "hemophiliaFresh",
                    "hemophiliaSourceId",
                    "poisoned",
                    "poisonedFresh",
                    "poisonSourceId",
                    "slowed",
                    "slowedFresh",
                    "slowSourceId",
                    "stalkDx",
                    "stalkDy",
                    "stalkGoalX",
                    "stalkGoalY",
                    "stalkObservedX",
                    "stalkObservedY",
                    "stalkPeekTick",
                    "stalkWait",
                    "monsterWendigoLevel",
                    "monsterLinkedMechId",
                    "monsterLinkedInventorId",
                    "monsterMountedTick",
                    "monsterInventorFleeing",
                    "monsterFireReadyAt",
                    "monsterVampireBat",
                    "monsterRetreating",
                    "monsterAudibleUntil",
                    "monsterAudibleX",
                    "monsterAudibleY",
                    "monsterBansheeLastShriek",
                    "monsterBansheeRevivedAt",
                    "monsterUrsaPartnerId",
                    "offscreenActionTick",
                    "temporaryAlly",
                    "temporaryAllyFresh",
                    "magicSensitive",
                    "magicSensitiveFresh",
                    "envenomed",
                    "maxAbilityCost",
                    "godbeastDepth",
                    "monsterSphinxHuntAt",
                    "monsterSphinxProvokedAt",
                    "monsterSphinxProvokerId",
                    "monsterSphinxLostSight",
                    "grabbed",
                    "grabbedBy",
                    "miniMe",
                    "miniMeId",
                    "weaponDamage",
                    "weaponOffenseMultiplier",
                    "offhandDamage",
                    "offhandOffenseMultiplier",
                    "rangedRecovery",
                    "beastCooldown",
                    "laserCharge",
                    "raisedWallPlane",
                    "raisedWallIndex",
                    "raisedWallExpires",
                    "monsterSpellPhase",
                    "monsterSpellReadyAt",
                    "monsterHealReadyAt",
                    "monsterSummonReadyAt",
                    "skinWalkerDisguise",
                    "witchSummoned",
                    "queenSummoned",
                    "nestResident",
                    "cornDimensionResident",
                    "monsterGasPassiveVolume",
                    "monsterGasBoostVolume",
                    "monsterGasShotVolume",
                    "unspeakTarget",
                    "unspeakCost",
                    "monsterChargeDx",
                    "monsterChargeDy",
                    "monsterChargeRemaining",
                    "reacted",
                    "hitBack",
                    "angelUsed",
                    "angelUsedRound",
                    "fortunes",
                    "cameTarget",
                    "cameTurns",
                    "cameFree",
                    "lucky",
                    "unlucky",
                    "distracted",
                    "rageCost",
                    "revertCharges",
                    "allSeeingReveal",
                    "allSeeingDepth",
                    "confused",
                    "confusedFresh",
                    "burning",
                    "burningOwner",
                    "burningPlayer",
                    "gasVulnerable",
                    "gasVulnerableTicks",
                    "submergedStealth",
                    "underwater",
                    "forcedMove",
                    "monsterPhoenixRolls",
                    "screamWeakened",
                    "disoriented",
                    "disorientedPenalty",
                    "hallucinating",
                    "screamEmboldened",
                    "screamPenalty",
                    "screamBonus",
                    "screamPerception",
                ]
                .contains(&k.as_str())
            {
                return unsupported(format!("condition {k} on {}", a.name));
            }
        }
    }
    for i in &r.inventory {
        crate::equipment::validate(i)?;
    }
    crate::environment::validate_supported_with_falls(r, pending_falls).map_err(EngineError)?;
    Ok(())
}
/// Inclusive O(T_validation + T_command + T_clone + T_index) time. Arc-backed
/// block snapshot is O(F); other metadata/COW costs follow their payloads.
/// T_index=O((A+P+B)*d+F+V) when dirty, O(1) at clean admission.
/// Command cost retains the existing turn/action abstraction (see complexity ledger).
/// Failure drops the candidate without touching the caller, RNG, logs or temporal ring.
pub fn dispatch(r: &mut Run, c: &Command) -> Result<DispatchOutcome, EngineError> {
    let _timing_function = crate::timing::span("command.core_dispatch");
    if !crate::future_floor::enabled(r) && r.status == "playing" && r.future.is_none()
        && matches!(c, Command::Ability { id, .. }
            if id == "morae-see-the-future" && r.player.abilities.contains(id))
    {
        crate::temporal::validate_future_capture(r).map_err(|e| EngineError(e.to_string()))?;
    }
    if r.player.monster_state.as_ref().and_then(|m|m.pilot.as_ref()).is_some() { return Err(EngineError("player cannot contain a mounted pilot".into())); } // O(1), before mutation.
    r.actors.validate_allocator(r.player.id,r.depth,r.next_id).map_err(EngineError)?;
    if r.status == "playing" {
        if let Some(message) = crate::projectiles::blocker(r, c) {
            bump(r, "commands", 1.0);
            r.replay.push(c.clone());
            log(r, message, "player");
            return Ok(DispatchOutcome {
                changed: true,
                ..Default::default()
            });
        }
    }
    if r.status != "playing" {
        return Ok(DispatchOutcome::default());
    }
    validate_supported_for(
        r,
        if matches!(c, Command::Descend) {
            std::slice::from_ref(&r.player.id)
        } else {
            &[]
        },
    )?;
    if crate::revision50::enabled(r) && matches!(c,Command::Craft{..}) {
        return unsupported("Revision 50 removed general god-specific Craft blessings");
    }
    if !matches!(
        c,
        Command::Move { .. }
            | Command::Interact { .. }
            | Command::Wait
            | Command::Pickup
            | Command::Rally { .. }
            | Command::Ally { .. }
            | Command::Use { .. }
            | Command::UseAt { .. }
            | Command::Drop { .. }
            | Command::Equip { .. }
            | Command::Unequip { .. }
            | Command::Ability { .. }
            | Command::Choose { .. }
            | Command::Craft { .. }
            | Command::Shoot { .. }
            | Command::Throw { .. }
            | Command::Descend
            | Command::Ascend
    ) {
        return unsupported(format!("command {c:?}"));
    }
    let replay = std::mem::take(&mut r.replay);
    let history = std::mem::take(&mut r.history);
    let archive = std::mem::take(&mut r.log_archive);
    let haste = std::mem::take(&mut r.haste_snapshot);
    let future = std::mem::take(&mut r.future);
    let archive_null = matches!(archive, Field::Null);
    let replay_len = replay.len();
    let archive_cursor = archive.as_ref().map(|a| (a.messages.len(), a.events.len()));
    let mut candidate = r.clone();
    // Public Run/inventory can be edited between commands. Admit those edits
    // conservatively once; repeated in-command health flushes stay O(1) clean.
    crate::health_dirty::invalidate(&mut candidate);
    candidate.replay = replay;
    candidate.history = history;
    candidate.log_archive = archive;
    crate::events::begin();
    let animation_snapshot=crate::animation_player::Snapshot::capture(&candidate);
    let observation_animation=crate::animation_observation::Snapshot::capture(&candidate);
    match dispatch_inner(&mut candidate, c).and_then(|out| {
        candidate.actors.validate_allocator(candidate.player.id,candidate.depth,candidate.next_id).map_err(EngineError)?;
        Ok(out)
    }) {
        Ok(mut out) => {
            if let Some(snapshot)=animation_snapshot {let player=candidate.player.id;crate::dreamwalking::with_actor(&mut candidate,player,|r|snapshot.finish(r));}
            if let Some(snapshot)=observation_animation {let player=candidate.player.id;crate::dreamwalking::with_actor(&mut candidate,player,|r|snapshot.finish(r));}
            crate::events::checkpoint(&candidate);
            out.events = crate::events::finish();
            if let Some(snapshot) = out.pending_snapshot.as_mut() {
                crate::mutation_journal::make_mut(&mut snapshot.state).future = future.clone();
            }
            candidate.future = future;
            if !out.took_turn {
                candidate.haste_snapshot = haste;
            }
            candidate.actors.adopt_epoch_from(&r.actors);
            *r = candidate;
            Ok(out)
        }
        Err(e) => {
            crate::events::finish();
            candidate.replay.truncate(replay_len);
            r.replay = candidate.replay;
            r.history = candidate.history;
            r.haste_snapshot = haste;
            r.future = future;
            if let Some((m, n)) = archive_cursor {
                let a = candidate.log_archive.as_mut().unwrap();
                a.messages.truncate(m);
                a.events.truncate(n);
                r.log_archive = candidate.log_archive;
            } else if archive_null {
                r.log_archive = Field::Null;
            }
            Err(e)
        }
    }
}
fn dispatch_inner(r: &mut Run, c: &Command) -> Result<DispatchOutcome, EngineError> {
    let _timing_function = crate::timing::span("command.action_dispatch");
    bump(r, "commands", 1.0);
    r.replay.push(c.clone());
    resources::observe_resource_history(r);
    crate::lifecycle::settle_nonpositive_health(r)?;
    crate::corpse_currents::settle_falls(r).map_err(EngineError)?;
    if r.status != "playing" { return finish(r, false); }
    if let Some(message)=crate::dreamwalking::blocker(r,c) {log(r,message,"player");return finish(r,false)}
    if r.simulation_revision.value_or(29.)<52. { if let Command::Ability{id,..}=c {if let Some(took)=crate::dreamwalking::ability(r,id)? {if took {turn(r)?;} return finish(r,took);}} }
    if r.simulation_revision.value_or(29.)<52. { if let Command::Ability{id,..}=c {if id==crate::ahvamitt::TELEPORT_ID {let took=crate::ahvamitt::teleport(r)?;if took {turn(r)?;}else {log(r,"Teleport is unavailable or still cooling down.","player");}return finish(r,took);}} }
    if let Command::Choose { id } = c {
        if crate::progression::choose_offer(r, id)? {
            crate::debug_floor::advance(r).map_err(EngineError)?;
        }
        if crate::debug_floor::preparing(r) { return Ok(DispatchOutcome { changed: true, ..Default::default() }); }
        return finish(r, false);
    }
    if crate::debug_floor::preparing(r) {
        log(r, "DEBUG: finish earned choices and confirm the loadout before entering.", "player");
        return Ok(DispatchOutcome { changed: true, ..Default::default() });
    }
    if !r.offers.is_empty() {
        log(r, "Choose your level-up ability first.", "player");
        return finish(r, false);
    }
    if let Command::Rally { x, y } = c {
        if in_bounds(r, *x, *y) {
            r.rally = Some(RunRally { x: *x, y: *y });
        }
        return finish(r, false);
    }
    if let Command::Ally { id } = c {
        if let Some(a) = r
            .actors_slice_mut(r.depth)
            .iter_mut()
            .find(|a| a.id == *id && a.perfect.value_or(false) && !a.dead.value_or(false))
        {
            a.order = Field::Value(
                if a.order.as_ref().is_some_and(|s| s == "p") {
                    "a"
                } else {
                    "p"
                }
                .into(),
            );
        }
        return finish(r, false);
    }
    if condition(&r.player, "paralyzed") > 0.0 && !matches!(c, Command::Wait) {
        log(r, "You are paralyzed. Wait to advance time.", "player");
        return finish(r, false);
    }
    if let Some(message) = crate::equipment::change_blocker(r, c) {
        log(r, &message, "player");
        return finish(r, false);
    }
    if let Some(Err(message)) = crate::revision50_amulet_effects::admission(r,c) {
        log(r,message,"player");
        return finish(r,false);
    }
    // Habitat stranding consumes even otherwise free commands, before damage counters open.
    // Source dispatchCore resolves this before the ordinary haste/action phase.
    if crate::ichor::stranded(r, &r.player) {
        let normal = r.phase.as_ref().is_some_and(|p| p == "normal");
        let tools_free = matches!(c, Command::Use { id } if r.player.abilities.iter().any(|a| a == "isheilah-tools-of-the-trade") && r.inventory.iter().any(|i| i.id == *id && matches!(i.kind.as_str(), "heart" | "potion")));
        let ability_free =
            matches!(c, Command::Ability { id, .. } if !crate::abilities::is_action(id));
        if !normal && !tools_free && !ability_free && !matches!(c, Command::Pickup) {
            crate::monster_chill::apply_adjacency(r);
        }
        if !normal {
            resources::begin_resource_turn(r);
        }
        crate::ichor::squirm_if_stranded(r, r.player.id);
        if !normal {
            advance_haste(r)?;
        }
        r.phase = Field::Value("haste".into());
        r.haste_snapshot = Field::Missing;
        turn(r)?;
        return finish(r, true);
    }
    if matches!(c, Command::Pickup) {
        return finish(r, false);
    }
    if let Command::Use { id } = c {
        if r.player
            .abilities
            .iter()
            .any(|id| id == "isheilah-tools-of-the-trade")
            && r.inventory
                .iter()
                .any(|i| i.id == *id && matches!(i.kind.as_str(), "heart" | "potion"))
        {
            crate::inventory::command(r, c)?;
            return finish(r, false);
        }
    }
    if crate::revision50_amulet_effects::admission(r,c).is_some_and(|result|
        result.is_ok_and(|plan| !plan.consumes_action())) {
        crate::dreamwalking::with_actor(r,r.player.id,|r|crate::inventory::command(r,c))?;
        return finish(r,false);
    }
    if let Command::Ability { id, target, x, y } = c {
        if !crate::abilities::is_action(id) && !(r.simulation_revision.value_or(29.)>=52.
            && ([crate::dreamwalking::DREAMWALK_ID,crate::dreamwalking::WAKE_UP_ID,crate::ahvamitt::TELEPORT_ID].contains(&id.as_str())
                || (id == crate::half_here::ID && !r.player.active.iter().any(|a| a == id)))) {
            crate::dreamwalking::with_actor(r,r.player.id,|r|crate::abilities::activate(r, id, target, x, y))?;
            return finish(r, false);
        }
    }
    // An earned Pursuit credit is exactly one movement-only action. It cannot
    // become an attack, and the bonus branch cannot mint another credit.
    if crate::revision50::enabled(r) && condition(&r.player,"r50PursuitCredit")>0.0 {
        if let Command::Move{dx,dy}=c {
            if dx.abs()<=1.0 && dy.abs()<=1.0 && (*dx!=0.0 || *dy!=0.0) {
                let before=r.clone();
                if move_player_command(r,*dx,*dy,true)?.0 {
                    r.player.conditions.shift_remove("r50PursuitCredit");
                    let mut out=finish_action(r,false)?;
                    if crate::temporal::retains_revert_history(r) {out.pending_snapshot=Some(crate::temporal::Snapshot::capture(&before));}
                    return Ok(out);
                }
                *r=before;
            }
        }
    }
    // Revision41 backward walking is tested before haste/resource/world clocks.
    // A relevant-edge candidate probe isolates RNG/state/events; failed probes fall through.
    if let Command::Move{dx,dy}=c {
        if let Some(outcome)=try_free_timeweave_move(r,*dx,*dy){return Ok(outcome);}
    }
    let replay = std::mem::take(&mut r.replay);
    let history = std::mem::take(&mut r.history);
    let archive = std::mem::take(&mut r.log_archive);
    let haste = std::mem::take(&mut r.haste_snapshot);
    let phase_cursor = archive.as_ref().map(|a| crate::temporal::ArchiveCursor {
        messages: a.messages.len(),
        events: a.events.len(),
    });
    let phase_before = r.clone();
    r.replay = replay;
    r.history = history;
    r.log_archive = archive;
    r.haste_snapshot = haste;
    let fast_continuation = if r.simulation_revision.value_or(29.)>=52. { r.fast_action_phase.as_ref().copied() } else { None };
    let normal = r.phase.as_ref().is_some_and(|p| p == "normal");
    if !normal && fast_continuation.is_none() {
        crate::monster_chill::apply_adjacency(r);
    }
    let bonus_player_action = r.simulation_revision.value_or(29.)>=52.
        && (fast_continuation == Some(crate::model::FastActionPhase::Haste)
            || (fast_continuation.is_none() && !normal && ai::is_hasted_in(r,&r.player)));
    let player_was_slowed = condition(&r.player, "slowed") > 0.0;
    if !normal && fast_continuation.is_none() {
        resources::begin_resource_turn(r);
        r.damage_since_last_turn = Field::Value(0.0);
        r.damage_since_last_turn_max_hp = Field::Value(r.player.max_hp);
        if let Command::Move { dx, dy } = c {
            let foe = r.actors_at(r.depth).iter().any(|a| {
                !a.dead.value_or(false)
                    && a.state != "ally"
                    && a.x == r.player.x + dx
                    && a.y == r.player.y + dy
            });
            if r.simulation_revision.value_or(29.) < 52. && !foe
                && !crate::godbeast_interactions::lever_pending(r, r.player.x + dx, r.player.y + dy)
                && ai::is_hasted_in(r, &r.player)
                && condition(&r.player, "paralyzed") == 0.0
                && move_player_command(r, *dx, *dy, true)?.0
            {
                let cap = crate::inventory::carrying_capacity(r)?;
                crate::inventory::collect_underfoot(r, cap)?;
                advance_haste(r)?;
                r.phase = Field::Value("normal".into());
                let mut out = finish_action(r, false)?;
                if crate::temporal::retains_revert_history(r) {
                    let mut snapshot = crate::temporal::Snapshot::capture(&phase_before);
                    snapshot.cursor = phase_cursor;
                    out.pending_snapshot = Some(snapshot);
                }
                return Ok(out);
            }
        }
        advance_haste(r)?;
        if r.simulation_revision.value_or(29.)>=52. && (r.status!="playing" || r.depth!=phase_before.depth) {
            return finish_action(r,false);
        }
    }
    crate::events::begin_player_action(r);
    let mut player_move_was_attack=false;
    let mut player_moved=false;
    let took = crate::dreamwalking::with_actor(r,r.player.id,|r| -> Result<bool,EngineError> {Ok(match c {
        Command::Interact { x, y } => {
            if crate::portals::point_chebyshev(r,&r.player,*x,*y) <= 1.0 {
                crate::borge::interact(r, *x, *y)?
                    || crate::godbeast_interactions::pull_lever(r, *x, *y).map_err(EngineError)?
            } else {
                false
            }
        }
        Command::Move { dx, dy } => {
            if dx.abs() <= 1.0 && dy.abs() <= 1.0 && (*dx != 0.0 || *dy != 0.0) {
                let (took,moved,attacked)=move_player_command(r,*dx,*dy,false)?;
                player_moved=moved;
                player_move_was_attack=attacked;
                took
            } else {false}
        }
        Command::Wait => {
            crate::events::actor_event(r, &r.player, "status", "Wait", None, None);
            stomp(r, false);
            r.player.reactions = Field::Value(if r.simulation_revision.value_or(29.) >= 52. {
                (r.player.reactions.value_or(0.) + 1.).min(max_reactions_for_revision(r.simulation_revision.value_or(29.), &r.player))
            } else { max_reactions_for_revision(r.simulation_revision.value_or(29.), &r.player) });
            if r.simulation_revision.value_or(29.) < 46. {
                log(r, "You wait.", "player");
            }
            true
        }
        Command::Descend => {
            let f = r.floor().unwrap();
            let tile = f.tiles[r.player.y as usize * f.width() + r.player.x as usize].clone();
            match tile.as_str() {
                "pit" => {
                    r.status = "won".into();
                    log(r, "You jump into the bottomless pit. Victory.", "player");
                    true
                }
                "chasm" => crate::travel::jump_chasm(r).map_err(EngineError)?,
                "down" => crate::travel::travel(r, 1.0, false, false).map_err(EngineError)?,
                _ => false,
            }
        }
        Command::Ascend => {
            if crate::finale::try_escape(r) { return Ok(true); }
            let f = r.floor().unwrap();
            if f.tiles[r.player.y as usize * f.width() + r.player.x as usize] == "up" {
                crate::travel::travel(r, -1.0, false, false).map_err(EngineError)?
            } else {
                false
            }
        }
        Command::Ability { id, target, x, y } => {
            if r.simulation_revision.value_or(29.)>=52. && id==crate::ahvamitt::TELEPORT_ID {
                crate::ahvamitt::teleport(r)?
            } else if r.simulation_revision.value_or(29.)>=52.
                && [crate::dreamwalking::DREAMWALK_ID,crate::dreamwalking::WAKE_UP_ID].contains(&id.as_str()) {
                crate::dreamwalking::ability(r,id)?.unwrap_or(false)
            } else {crate::abilities::activate(r,id,target,x,y)?}
        },
        Command::Craft { heart, item, craft } => crate::crafting::craft(r, *heart, *item, craft)?,
        _ => crate::inventory::command(r, c)?.unwrap_or(false),
    })})?;
    if took && r.simulation_revision.value_or(29.)>=52. {
        let fast_first_move = fast_continuation.is_none() && player_moved && !player_move_was_attack
            && ai::fast_movement_enabled(&phase_before,&phase_before.player)
            && ai::fast_movement_enabled(r,&r.player) && r.status=="playing"
            && r.depth==phase_before.depth && r.player.hp>0. && !r.player.dead.value_or(false);
        if fast_first_move {
            r.fast_action_phase=if bonus_player_action {crate::model::FastActionPhase::Haste}
                else {crate::model::FastActionPhase::Normal}.into();
            r.phase="normal".to_string().into();
            let cap=crate::inventory::carrying_capacity(r)?;
            crate::inventory::collect_underfoot(r,cap)?;
            return finish_action(r,false);
        }
        r.fast_action_phase=Field::Missing;
        if bonus_player_action {
            r.phase="normal".to_string().into();
            let cap=crate::inventory::carrying_capacity(r)?;
            crate::inventory::collect_underfoot(r,cap)?;
            return finish_action(r,false);
        }
    }
    let pursuit_bonus = took && !player_was_slowed && !player_move_was_attack && match c {
        Command::Move{dx,dy} => crate::revision50_pursuit::grant_for_move(&phase_before,r,*dx,*dy),
        _ => false,
    };
    if took
        && !player_was_slowed
        && matches!(c, Command::Move { .. })
        && condition(&r.player, "cameTurns") > 0.0
        && condition(&r.player, "cameFree") == 0.0
    {
        let target = condition(&r.player, "cameTarget");
        if actor(r, target).is_some_and(|a| {
            !a.dead.value_or(false)
                && (r.player.x - a.x).hypot(r.player.y - a.y)
                    < (phase_before.player.x - a.x).hypot(phase_before.player.y - a.y)
        }) {
            let cap = crate::inventory::carrying_capacity(r)?;
            crate::inventory::collect_underfoot(r, cap)?;
            r.player.conditions.insert("cameFree".into(), 1.0);
            crate::animation_player::local(r,&r.player,"P08","speed",crate::events::Stage::Travel);
            r.phase = Field::Value("normal".into());
            let mut out = finish_action(r, false)?;
            if crate::temporal::retains_revert_history(r) {
                let mut snapshot = crate::temporal::Snapshot::capture(&phase_before);
                snapshot.cursor = phase_cursor;
                out.pending_snapshot = Some(snapshot);
            }
            return Ok(out);
        }
    }
    if took {
        r.player.conditions.shift_remove("r50PursuitCredit");
        r.player.conditions.shift_remove("cameFree");
        let cap = crate::inventory::carrying_capacity(r)?;
        crate::inventory::collect_underfoot(r, cap)?;
        r.phase = Field::Value("haste".into());
        r.haste_snapshot = Field::Missing;
        turn(r)?;
        if player_was_slowed && r.status == "playing" {
            if r.simulation_revision.value_or(29.) < 46. {
                log(r, "Slowness delays your next action.", "player");
            }
            advance_haste(r)?;
            turn(r)?;
        }
        if pursuit_bonus && r.status=="playing" && crate::revision50_pursuit::selected_target(r).is_some() {
            r.player.conditions.insert("r50PursuitCredit".into(),1.0);
        }
    } else {
        let messages = std::mem::take(&mut r.log);
        let kinds = std::mem::take(&mut r.log_kinds);
        let archive = std::mem::take(&mut r.log_archive);
        let replay = std::mem::take(&mut r.replay);
        let history = std::mem::take(&mut r.history);
        let haste = std::mem::take(&mut r.haste_snapshot);
        // Source Object.assign restores present snapshot keys but retains newly introduced optional counters.
        let damage = r.damage_since_last_turn.clone();
        let damage_max = r.damage_since_last_turn_max_hp.clone();
        *r = phase_before;
        if matches!(r.damage_since_last_turn, Field::Missing) {
            r.damage_since_last_turn = damage;
        }
        if matches!(r.damage_since_last_turn_max_hp, Field::Missing) {
            r.damage_since_last_turn_max_hp = damage_max;
        }
        r.log = messages;
        r.log_kinds = kinds;
        r.log_archive = archive;
        r.replay = replay;
        r.history = history;
        r.haste_snapshot = haste;
    }
    finish(r, took)
}
fn finish(r: &mut Run, took: bool) -> Result<DispatchOutcome, EngineError> {
    finish_inner(r, took, took)
}
/// A successful bonus movement finishes an action without advancing the turn.
/// O(1) routing; expiry removes two IndexMap entries in O(C) conditions.
/// Existing final-observation costs and ordering remain unchanged.
fn finish_action(r: &mut Run, took: bool) -> Result<DispatchOutcome, EngineError> {
    finish_inner(r, took, true)
}
fn finish_inner(r: &mut Run, took: bool, completed_action: bool) -> Result<DispatchOutcome, EngineError> {
    let _timing_function = crate::timing::span("command.finish");
    crate::corpse_currents::settle_falls(r).map_err(EngineError)?;
    crate::equipment::sync_ownership(r)?;
    crate::abilities::disable_unfunded(r)?;
    crate::revision50_pursuit::settle(r);
    crate::half_here::sync(r);
    crate::monster_grab::release_invalid(r);
    settle_doors(r);
    if crate::dreamwalking::active(r) {crate::dreamwalking::with_plane(r,true,settle_doors);}
    let cap = crate::inventory::carrying_capacity(r)?;
    crate::inventory::collect_underfoot(r, cap)?;
    settle_doors(r);
    remember(r);
    if crate::dreamwalking::active(r) {crate::dreamwalking::with_plane(r,true,remember);}
    crate::inventory::remember_detected_items(r).map_err(EngineError)?;
    let wake: Vec<_> = r
        .actors_at(r.depth)
        .iter()
        .filter(|a| {
            !a.dead.value_or(false)
                && a.hp > 0.0
                && a.state == "ally"
                && ai::sleeping(a)
                && ai::should_wake_ally(r, a)
        })
        .map(|a| a.id)
        .collect();
    for id in wake {
        ai::wake(actor_mut_trusted(r, id).unwrap(), true);
    }
    if took || ((50.0..51.0).contains(&r.simulation_revision.value_or(29.)) && completed_action) {
        r.player.conditions.shift_remove("allSeeingReveal");
        r.player.conditions.shift_remove("allSeeingDepth");
    }
    resources::observe_resource_history(r);
    Ok(DispatchOutcome {
        events: vec![],
        pending_snapshot: None,
        took_turn: took,
        changed: true,
    })
}
fn in_bounds(r: &Run, x: f64, y: f64) -> bool {
    r.floor().is_some_and(|f| {
        x.is_finite()
            && y.is_finite()
            && x.fract() == 0.0
            && y.fract() == 0.0
            && x >= 0.0
            && y >= 0.0
            && x < f.width() as f64
            && y < f.height() as f64
    })
}
pub fn passable(r: &Run, x: f64, y: f64) -> bool {
    if !in_bounds(r, x, y) {
        return false;
    }
    let f = r.floor().unwrap();
    let i = y as usize * f.width() + x as usize;
    f.tiles[i] != "wall"
        && f.tiles[i] != "tree"
        && !f.grates.as_ref().is_some_and(|g| g.contains(&(i as f64)))
}
pub fn actor_passable(r: &Run, a: &Actor, x: f64, y: f64) -> bool {
    in_bounds(r, x, y)
        && !crate::revision51_stairs::blocked(r,a,x,y)
        && !crate::revision50_terrain::clay_blocks_run(r,a,x,y)
        && !crate::revision50_control_amulets::obstruction_blocks(r,a,x,y)
        && !r.floor().is_some_and(|f|crate::borge::solid_at(f,x,y) || crate::epidite::solid_at(f,x,y))
        && (passable(r, x, y) || a.active.iter().any(|id| id == "etill-phase-through"))
}
fn point(a: &Actor) -> Point {
    Point {
        x: a.x as i32,
        y: a.y as i32,
    }
}
fn stomp(r: &mut Run, announce: bool) {
    let (x, y) = (r.player.x, r.player.y);
    let mut count = 0;
    for a in r.actors_slice_mut(r.depth) {
        if a.dead.value_or(false) && a.x == x && a.y == y {
            a.corpse_age = Field::Value(0.0);
            a.conditions.shift_remove("monsterPhoenixRolls");
            count += 1;
        }
    }
    if announce {
        for _ in 0..count {
            log(r, "You stomp on the heartless corpse.", "player");
        }
    }
}
pub fn move_actor(r:&mut Run,id:f64,dx:f64,dy:f64,movement_only:bool)->Result<bool,EngineError> {
    let before=actor(r,id).map(|a|(a.x,a.y,condition(a,"forcedMove")));
    let result=crate::dreamwalking::with_actor(r,id,|r|move_actor_inner(r,id,dx,dy,movement_only,None));
    if r.simulation_revision.value_or(29.)>=52. && before.is_some_and(|(x,y,forced)|forced==0.
        && actor(r,id).is_some_and(|a|a.x!=x || a.y!=y)) {crate::revision52_speed::record_move(id);}
    result
}
fn move_actor_inner(
    r: &mut Run,
    id: f64,
    mut dx: f64,
    mut dy: f64,
    movement_only: bool,
    trace: Option<&mut CommandMoveTrace>,
) -> Result<bool, EngineError> {
    let _timing_function = crate::timing::span("movement.actor");
    let a = actor(r, id)
        .ok_or_else(|| EngineError("actor missing".into()))?
        .clone();
    if crate::revision50_terrain::is_soldier(&a) || crate::revision50_nightmares::is_decoy(&a) || crate::ahvamitt::is_actor(&a) || crate::dreamwalking::is_body(&a) || (crate::half_here::is_clone(&a) && condition(&a, "forcedMove") == 0.0) {
        return Ok(false);
    }
    let mut confused = false;
    if condition(&a, "confused") > 0.0 && condition(&a, "forcedMove") == 0.0 {
        let intentional = id == r.player.id
            && !movement_only
            && r.actors_at(r.depth).iter().any(|b| {
                !b.dead.value_or(false)
                    && b.state != "ally"
                    && b.x == a.x + dx
                    && b.y == a.y + dy
                    && vision::visible_actor(r, b)
            });
        if !intentional {
            let f = r.floor().unwrap();
            let choices: Vec<_> = movement::DIRECTIONS
                .iter()
                .filter(|d| {
                    let (x, y) = (a.x + d.x as f64, a.y + d.y as f64);
                    actor_passable(r, &a, x, y)
                        && crate::ichor::movement_allowed(r, &a, x, y)
                        && !bulwark_blocks(r, &a, x, y)
                        && movement::diagonal_step_allowed(
                            &f.tiles,
                            f.width(),
                            point(&a),
                            Point {
                                x: x as i32,
                                y: y as i32,
                            },
                            a.active.iter().any(|id| id == "etill-phase-through"),
                        )
                        && (id == r.player.id
                            || f.tiles[y as usize * f.width() + x as usize] != "chasm"
                            || flying_in(r, &a))
                })
                .copied()
                .collect();
            if choices.is_empty() {
                return Ok(true);
            }
            let d = choices[(rng::random(r) * choices.len() as f64).floor() as usize];
            dx = d.x as f64;
            dy = d.y as f64;
            confused = true;
        }
    }
    Ok(move_actor_step(r, id, dx, dy, movement_only, confused, trace)? || confused)
}
fn move_actor_step(
    r: &mut Run,
    id: f64,
    dx: f64,
    dy: f64,
    movement_only: bool,
    confused: bool,
    mut trace: Option<&mut CommandMoveTrace>,
) -> Result<bool, EngineError> {
    let a = actor(r, id)
        .ok_or_else(|| EngineError("actor missing".into()))?
        .clone();
    let r50_grapple=crate::revision50_grappler::snapshot(r);
    // Only an explicit player command carries this trace; forced/current moves
    // retain ordinary fall scaling even when they enter the same chasm tile.
    let (entry_x, entry_y) = (a.x + dx, a.y + dy);
    let options=crate::portals::step_options(r,&a,entry_x,entry_y);
    let (mut x,mut y)=options[0];
    let voluntary_chasm = trace.is_some() && condition(&a,"forcedMove")==0.0
        && options.len()==1 && (x,y)==(entry_x,entry_y);
    let ordinary_edge=if trace.is_some()&&options.len()==1&&(x,y)==(entry_x,entry_y){ordinary_timeweave_edge(r,&a,x,y)}else{None};
    // Probe refuses transitions before their side effects; normal movement unchanged.
    if trace.as_ref().is_some_and(|t|t.probe_only)&&ordinary_edge.is_none(){return Ok(false);}

    // Validate every stochastic outcome without drawing. A blocked sibling is
    // not discarded from the lottery, and failed movement cannot reroll it.
    if options.len()>1 && options.iter().any(|&(ox,oy)| {
        !in_bounds(r,ox,oy) || !actor_passable(r,&a,ox,oy)
            || bulwark_blocks(r,&a,ox,oy) || !crate::ichor::movement_allowed(r,&a,ox,oy)
            || r.floor().is_some_and(|f| {
                let at=oy as usize*f.width()+ox as usize;
                (id!=r.player.id&&f.tiles[at]=="chasm"&&!flying_in(r,&a))
                || (id==r.player.id&&crate::dreamwalking::active(r)&&matches!(f.tiles[at].as_str(),"up"|"down"|"pit"))
                || (condition(&a,"forcedMove")!=0.&&f.doors.as_ref().and_then(|d|d.get(&at.to_string())).is_some_and(|d|!d.open))
            })
    }) {return Ok(false)}
    if !in_bounds(r, x, y) {
        return Ok(false);
    }
    if id==r.player.id && crate::dreamwalking::active(r) && r.floor().is_some_and(|f|matches!(f.tiles[y as usize*f.width()+x as usize].as_str(),"up"|"down"|"pit")) {log(r,crate::dreamwalking::STAIRS_MESSAGE,"player");return Ok(false)}
    if condition(&a, "forcedMove") == 0.0 && crate::ichor::squirm_if_stranded(r, id) {
        return Ok(true);
    }
    let f = r.floor().unwrap();
    if condition(&a, "forcedMove") != 0.0
        && f.doors
            .as_ref()
            .and_then(|d| d.get(&(y as usize * f.width() + x as usize).to_string()))
            .is_some_and(|d| !d.open)
    {
        return Ok(false);
    }
    if id != r.player.id && crate::portals::same_position(r,&r.player,x,y) && crate::dreamwalking::same_plane(r,&a,&r.player) {
        if confused && !movement_only && a.state != "ally" {
            if !crate::monster_statuses::try_hemophilia_episode(r, id).map_err(EngineError)? {
                attack(r, id, r.player.id, 1.0)?;
            }
            return Ok(true);
        }
        return Ok(false);
    }
    if condition(&a, "riptide") != 0.0 && condition(&a, "forcedMove") == 0.0 {
        return Ok(false);
    }
    // Solid obelisk cells are interactions, never ordinary attack targets or movement.
    if id == r.player.id && !crate::dreamwalking::active(r) && crate::borge::interaction_at(r, x, y) {
        if movement_only { return Ok(false); }
        return crate::borge::interact(r, x, y);
    }
    if !movement_only && options.len()==1 && (x,y)==(entry_x,entry_y)
        && crate::revision50_terrain::attack_wall(r,&a,x,y)? {return Ok(true);}
    if !actor_passable(r, &a, entry_x, entry_y) || !actor_passable(r, &a, x, y)
        || bulwark_blocks(r, &a, x, y)
        || !crate::ichor::movement_allowed(r, &a, x, y)
    {
        return Ok(false);
    }
    let f = r.floor().unwrap();
    let tile = &f.tiles[y as usize * f.width() + x as usize];
    if id != r.player.id && tile == "chasm" && !flying_in(r, &a) {
        return Ok(false);
    }
    if id != r.player.id
        && condition(&a, "forcedMove") == 0.0
        && r.player
            .active
            .iter()
            .any(|id| id == "isheilah-make-slippery")
        && condition(&r.player, "slipperyTarget") == id
        && crate::dreamwalking::same_plane(r,&r.player,&a)
        && crate::revision50_ability_energy::spend(r, r.player.id, "isheilah-make-slippery", 8.0)
    {
        crate::animation_shapes::area(r,&a,"P15","box-contract",2,"#a040ff",crate::events::Stage::Special);
        return Ok(false);
    }
    let f = r.floor().unwrap();
    if let Some(other) = r
        .actors_at(r.depth)
        .iter()
        .find(|b| !b.dead.value_or(false) && b.id != id && crate::portals::same_position(r,b,x,y) && crate::dreamwalking::same_plane(r,&a,b))
    {
        let oid = other.id;
        if id == r.player.id {
            if other.state != "ally" || crate::revision50_control_amulets::discord_hostility(r,&a,other) {
                if movement_only {
                    return Ok(false);
                }
                if let Some(trace)=trace.as_deref_mut(){trace.attacked=true;}
                if !crate::monster_statuses::try_hemophilia_episode(r, id).map_err(EngineError)? {
                    player_attack(r, oid)?;
                }
                return Ok(true);
            }
            if !can_traverse(r, other, a.x, a.y) {
                return Ok(false);
            }
        } else {
            if !movement_only
                && condition(&a, "forcedMove") == 0.0
                && (crate::revision50_control_amulets::discord_hostility(r,&a,other)
                    || (a.state == "ally" && other.state != "ally" && other.state != "neutral")
                    || (confused && a.state != "ally" && other.state == "ally"))
            {
                if !crate::monster_statuses::try_hemophilia_episode(r, id).map_err(EngineError)? {
                    attack(r, id, oid, 1.0)?;
                }
                return Ok(true);
            }
            return Ok(false);
        }
    }
    if condition(&a, "forcedMove") == 0.0
        && !movement::diagonal_step_allowed(
            &f.tiles,
            f.width(),
            point(&a),
            Point {
                x: entry_x as i32,
                y: entry_y as i32,
            },
            a.active.iter().any(|id| id == "etill-phase-through"),
        )
    {
        return Ok(false);
    }
    if id == r.player.id && !crate::dreamwalking::active(r) {
        if movement_only && crate::godbeast_interactions::lever_pending(r, x, y) {
            return Ok(false);
        }
        if !movement_only
            && crate::godbeast_interactions::pull_lever(r, x, y).map_err(EngineError)?
        {
            return Ok(true);
        }
    }
    if movement_held(r, id) || crate::revision50_grappler::locked(r,&a) {
        return Ok(true);
    }
    if condition(&a, "forcedMove") == 0.0
        && crate::monster_statuses::try_hemophilia_episode(r, id).map_err(EngineError)?
    {
        return Ok(true);
    }
    if (dx != 0. || dy != 0.) && crate::terrain_runtime::slip(r, id) { return Ok(true); }
    if options.len()>1 {(x,y)=crate::portals::commit_step(r,&a,entry_x,entry_y);}
    let f = r.floor().unwrap();
    let cell = y as usize * f.width() + x as usize;
    if crate::environment::open_door(r.floor_mut().unwrap(), cell) {
        crate::events::actor_event(
            r,
            &a,
            "status",
            "Open door",
            Some(crate::events::EventPoint { x, y }),
            None,
        );
        if id == r.player.id || vision::visible_actor(r, &a) {
            log(
                r,
                format!(
                    "{} the door.",
                    if id == r.player.id {
                        "You open".into()
                    } else {
                        format!("{} opens", crate::actor_presentation::name(r, &a))
                    }
                ),
                if id == r.player.id { "player" } else { "enemy" },
            );
        }
    }
    if id == r.player.id {
        if let Some(other) = r
            .actors_at(r.depth)
            .iter()
            .find(|b| !b.dead.value_or(false) && b.state == "ally" && crate::dreamwalking::same_plane(r,&a,b) && crate::portals::same_position(r,b,x,y))
        {
            let oid = other.id;
            let swapped_from = other.y as usize * r.floor().unwrap().width() + other.x as usize;
            let other = actor_mut_trusted(r, oid).unwrap();
            other.x = a.x;
            other.y = a.y;
            crate::travel::enemy_moved(r, oid).map_err(EngineError)?;
            if actor(r, oid).is_some_and(crate::half_here::is_clone) {
                crate::divine_environment::leave_black_pit(r, oid, swapped_from);
            }
        }
    }
    let before_move=a.clone();
    let animation_depth=r.depth;
    let phase_wall_before=r.floor().is_some_and(|f|f.tiles[a.y as usize*f.width()+a.x as usize]=="wall");
    let old = a.y as usize * r.floor().unwrap().width() + a.x as usize;
    let a = actor_mut_trusted(r, id).unwrap();
    a.x = x;
    a.y = y;
    if let Some(trace)=trace {
        trace.edge=ordinary_edge;
        // Fast uses the committed command movement, not the narrower local
        // Timeweave edge. Portals and special tiles may move without an edge.
        trace.voluntary_move=condition(&before_move,"forcedMove")==0.
            && (x!=before_move.x || y!=before_move.y);
    }
    finish_actor_arrival(r,id,old,voluntary_chasm,r50_grapple)?;
    if let Some(after)=actor(r,id){
        crate::animation_enemy::moved(r,&before_move,after);
        if animation_depth==r.depth {
            if after.active.iter().any(|s|s=="etill-phase-through")&&(phase_wall_before||r.floor().is_some_and(|f|f.tiles[after.y as usize*f.width()+after.x as usize]=="wall")){crate::animation_player::moved(r,&before_move,after,"P25","ripple");}
            if after.active.iter().any(|s|s=="etill-rushing-torrent"){crate::animation_player::moved(r,&before_move,after,"P26","speed");}
            if after.abilities.iter().any(|s|s=="ursa-sleep-walk")&&after.energy==0.{crate::animation_player::moved(r,&before_move,after,"P34","speed");}
            if crate::animation_player::shadow_active(r,after){crate::animation_player::moved(r,&before_move,after,"P43","speed");}
            if after.abilities.iter().any(|s|s=="etill-drown")&&crate::divine_environment::water_at(r,after)&&!crate::divine_environment::water_at(r,&before_move){crate::animation_player::local(r,after,"P27","ripple",crate::events::Stage::Special);}
        }
    }
    Ok(true)
}
/// Shared arrival for committed ordinary movement and authored R50 displacement.
/// Uses the existing contact, door, departure and travel rules without duplicating
/// a partial movement pipeline. Caller has already admitted and set the position.
pub(crate) fn finish_actor_arrival(
    r: &mut Run, id: f64, old: usize, voluntary_chasm: bool,
    grapple: Option<crate::revision50_grappler::DragSnapshot>,
) -> Result<(), EngineError> {
    if crate::revision50::enabled(r) {
        let w=r.floor().unwrap().width();
        if actor(r,id).is_some_and(|a|a.x!=(old%w)as f64||a.y!=(old/w)as f64) {crate::revision50_control_amulets::break_roots(r,id);}
    }
    // End a Duel at the first committed departure, including forced multi-step
    // movement, so later reentry within this action cannot restore protection.
    if crate::revision50::enabled(r) {
        let depth=r.depth;
        if let Some(a)=actor_mut_trusted(r,id) {
            if !crate::half_here::is_clone(a) {crate::revision50::settle_actor_duel(a,depth);}
        }
    }
    crate::travel::enemy_moved(r, id).map_err(EngineError)?;
    settle_doors(r);
    if id == r.player.id {
        stomp(r, true);
    }
    crate::divine_environment::leave_black_pit(r, id, old);
    crate::revision50_grappler::finish(r,grapple)?;
    if id == r.player.id {
        let f = r.floor().unwrap();
        let entered = f.tiles[r.player.y as usize * f.width() + r.player.x as usize].clone();
        if entered == "chasm" && !flying_in(r, &r.player) {
            if voluntary_chasm { crate::travel::jump_chasm(r).map_err(EngineError)?; }
            else { crate::travel::travel(r, 1.0, true, false).map_err(EngineError)?; }
        } else if entered == "down" {
            crate::travel::travel(r, 1.0, false, false).map_err(EngineError)?;
        } else if entered == "up" && !crate::finale::try_escape(r) {
            crate::travel::travel(r, -1.0, false, false).map_err(EngineError)?;
        }
        let f = r.floor().unwrap();
        if f.tiles[r.player.y as usize * f.width() + r.player.x as usize] == "pit"
            && !flying_in(r, &r.player)
        {
            r.status = "won".into();
            log(r, "You leap into the bottomless pit. Victory.", "player");
        }
    }
    Ok(())
}
pub(crate) type PathCache = std::collections::HashMap<String, movement::DistanceField>;
fn pursue(
    r: &mut Run,
    id: f64,
    target: Point,
    cache: &mut PathCache,
    movement_only: bool,
) -> Result<(), EngineError> {
    let step=crate::dreamwalking::with_actor(r,id,|r|path_step_cached(r,id,target,cache));
    if let Some(s) = step {
        move_actor(r, id, s.x as f64, s.y as f64, movement_only)?;
    }
    Ok(())
}
pub fn creature_path_step(r: &mut Run, id: f64, target: Point) -> Option<Point> {
    crate::dreamwalking::with_actor(r,id,|r|path_step_cached(r, id, target, &mut PathCache::new()))
}
/// Cached hit: actor lookup/clone + O(B log B) ability key + next-step checks.
/// Miss/rebuild adds O(V * (allowed-cell + hazard-cell helper)) masks and the
/// BFS/Dijkstra field cost, with O(V + links) scratch; no mask scan on clean hit.
fn path_step_cached(r: &mut Run, id: f64, target: Point, cache: &mut PathCache) -> Option<Point> {
    let _timing_function = crate::timing::span("path.request");
    let _timing_prepare = crate::timing::span("path.masks_and_key");
    let a = actor(r, id).unwrap().clone();
    let phase = a.active.iter().any(|id| id == "etill-phase-through");
    let f = r.floor().unwrap();
    let mut abilities = a.abilities.clone();
    abilities.sort();
    let key = format!(
        "{},{},{},{},{},{},{},{}",
        crate::dreamwalking::context_dream(r),
        target.x,
        target.y,
        phase,
        flying_in(r, &a),
        if r.simulation_revision.value_or(29.) >= 51. { !crate::revision51_stairs::restricted(r,&a) } else { a.state == "ally" || a.state == "neutral" },
        crate::corpses::is_bastidion(&a),
        abilities.join(";")
    );
    drop(_timing_prepare);
    // Masks are consumed only by a new/rebuilt field. A validated hit reuses
    // the field, while its next-step hazard check below still observes current
    // state exactly as before. Avoid O(V * cell-helper) work on ordinary hits.
    let build_masks = || {
        let _timing_masks = crate::timing::span("path.mask_build");
    let allowed: Vec<_> = (0..f.tiles.len())
        .map(|i| {
            !crate::revision50_terrain::clay_blocks_run(r,&a,(i%f.width())as f64,(i/f.width())as f64)
                && !crate::revision50_control_amulets::obstruction_blocks(r,&a,(i%f.width())as f64,(i/f.width())as f64)
                && !crate::revision51_stairs::blocked_cell(r,&a,f,i)
                && crate::ichor::habitat_path_allowed(f, &a, i)
                && (phase || !f.grates.as_ref().is_some_and(|g| g.contains(&(i as f64))))
                && !bulwark_blocks(r, &a, (i % f.width()) as f64, (i / f.width()) as f64)
        })
        .collect();
    let harmful: Vec<_> = f
        .tiles
        .iter()
        .enumerate()
        .map(|(i, _)| crate::environment::creature_hazard_at(r, &a, i))
        .collect();
        (allowed, harmful)
    };
    let mut built = false;
    if !cache.contains_key(&key) {
        let (allowed, harmful) = build_masks();
        let field = movement::distance_field_ordered(
            &f.tiles,
            f.width(),
            target,
            phase,
            flying_in(r, &a),
            Some(&allowed),
            Some(&harmful),
            &if crate::portals::enabled(r){crate::portals::links(f)}else{vec![]},
            &if r.simulation_revision.value_or(29.)>=37. {crate::portals::ordered_pairs(f)} else {vec![]},
        );
        cache.insert(key.clone(), field);
        built = true;
    }
    let field = cache.get(&key).unwrap();
    let mut step = movement::next_step(field, &f.tiles, point(&a));
    if let Some(d) = step {
        let at = (a.y as i32 + d.y) as usize * f.width() + (a.x as i32 + d.x) as usize;
        if !field.harmful.as_ref().is_some_and(|h| h[at])
            && crate::environment::creature_hazard_at(r, &a, at)
        {
            let (allowed, harmful) = build_masks();
            let field = movement::distance_field_ordered(
                &f.tiles,
                f.width(),
                target,
                phase,
                flying_in(r, &a),
                Some(&allowed),
                Some(&harmful),
                &if crate::portals::enabled(r){crate::portals::links(f)}else{vec![]},
                &if r.simulation_revision.value_or(29.)>=37. {crate::portals::ordered_pairs(f)} else {vec![]},
            );
            cache.insert(key.clone(), field);
            step = movement::next_step(cache.get(&key).unwrap(), &f.tiles, point(&a));
            built = true;
        }
    }
    if step.is_none() {step=crate::revision50_terrain::breach_step(r,&a,target);}
    if built {
        bump(r, "pathFields", 1.0);
    }
    step.map(|chosen| crate::revision51_enemy_reroute::replacement_step(r, id, target, chosen)
        .unwrap_or(chosen))
}
/// One true-Haste action per eligible NPC. Legacy revisions preserve the old
/// movement-only pass. Fast is resolved within each R52 complete action.
fn advance_haste(r: &mut Run) -> Result<(), EngineError> {
    let _timing_function = crate::timing::span("tick.haste");
    let mut cache = PathCache::new();
    let depth=r.depth;
    for id in ai::phase_order(r.actors_at(r.depth), true) {
        if r.simulation_revision.value_or(29.)>=52. && (r.status!="playing"||r.depth!=depth) {break;}
        let Some(a) = actor(r,id).cloned() else {continue;};
        crate::events::next_action(r);
        if a.dead.value_or(false)
            || crate::dreamwalking::inert(r,&a)
            || crate::half_here::is_clone(&a)
            || condition(&a, "paralyzed") > 0.0
            || crate::ichor::stranded(r, &a)
            || !ai::is_hasted_in(r, &a)
            || (a.abilities.iter().any(|id| id == "enemy:beast:slow")
                || a.abilities.iter().any(|id| id == "enemy:pale:slow"))
            || (a.state == "sleeping" && a.state != "ally")
        {
            continue;
        }
        if r.simulation_revision.value_or(29.) >= 52. {
            complete_actor_action(r, id, true, &[], &mut cache)?;
            continue;
        }
        if crate::revision50_control_amulets::discord_turn(r,id,true)? {continue;}
        if crate::creature_feeding::take_turn(r, id, false, true)? { continue; }
        if a.state == "ally" {
            ally_turn(r, id, true, &mut cache)?;
            continue;
        }
        let known_target = std::iter::once(&r.player)
            .chain(r.actors_at(r.depth))
            .find(|b| {
                !b.dead.value_or(false)
                    && crate::dreamwalking::same_plane(r,&a,b)
                    && a.conditions.get("targetId") == Some(&b.id)
                    && a.last_seen
                        .as_ref()
                        .is_some_and(|p| p.x == b.x && p.y == b.y)
            })
            .map(|b| b.id);
        if monster_ai_turn_target(r, id, known_target, true)? {
            continue;
        }
        if a.state == "stalking" && a.conditions.contains_key("stalkGoalX") {
            pursue(
                r,
                id,
                Point {
                    x: condition(&a, "stalkGoalX") as i32,
                    y: condition(&a, "stalkGoalY") as i32,
                },
                &mut cache,
                true,
            )?;
        } else if a.state == "hunting" {
            if let Some(t) = a.last_seen.as_ref() {
                pursue(
                    r,
                    id,
                    Point {
                        x: t.x as i32,
                        y: t.y as i32,
                    },
                    &mut cache,
                    true,
                )?;
            }
        }
    }
    Ok(())
}

/// Bounded Fast continuation: one extra path request or ordinary attack, with
/// the same movement/contact restrictions. No clocks, recovery, spells or RNG
/// detection pass are repeated. Path-cache misses retain their existing cost.
pub(crate) fn fast_followup(r: &mut Run, id: f64, before: &Actor, cache: &mut PathCache) -> Result<(), EngineError> {
    let Some(a) = actor(r,id).cloned() else { return Ok(()); };
    if !ai::fast_movement_enabled(r,before) || !ai::fast_movement_enabled(r,&a)
        || (a.x == before.x && a.y == before.y) || a.dead.value_or(false) || a.hp <= 0.
        || r.status != "playing" || condition(&a,"paralyzed") > 0.
        || crate::dreamwalking::inert(r,&a) || crate::ichor::stranded(r,&a) {
        return Ok(());
    }
    // Already-paid Dragon landing precedes rest on both ordinary and Fast
    // movement. This exception cannot activate flight, wake or attack.
    if ai::sleeping(&a) {
        crate::revision52_dragon_flight::landing_turn(r,id)?;
        return Ok(());
    }
    if crate::creature_feeding::hungry(r,&a) && a.last_seen.as_ref().is_none() { return Ok(()); }
    if crate::revision50_control_amulets::discord_fast_followup(r,id)? { return Ok(()); }
    let quarry = if a.state == "ally" { ai::nearest_ally_enemy(r,&a).cloned() }
        else { std::iter::once(&r.player).chain(r.actors_at(r.depth)).find(|b| {
            !b.dead.value_or(false) && b.hp>0. && condition(&a,"targetId")==b.id
                && crate::dreamwalking::same_plane(r,&a,b)
                && a.last_seen.as_ref().is_some_and(|p|p.x==b.x && p.y==b.y)
        }).cloned() };
    // Committed charge owns its direction, including an attack after the first
    // successful step. Other specialist AI gets movement only on this follow-up.
    let charge = a.abilities.iter().any(|s|s=="enemy:beast:minotaur");
    if monster_ai_turn_target(r,id,quarry.as_ref().map(|a|a.id),!charge)? { return Ok(()); }
    let a=actor(r,id).unwrap().clone();
    if a.state=="ally" && a.perfect.value_or(false) && a.order.as_ref().is_some_and(|s|s=="p") {
        if let Some(p)=r.rally.clone() {pursue(r,id,Point{x:p.x as i32,y:p.y as i32},cache,true)?;}
        return Ok(());
    }
    if let Some(b)=quarry.as_ref().filter(|b|crate::portals::actor_distance(r,&a,b)<=std::f64::consts::SQRT_2) {
        if !crate::monster_statuses::try_hemophilia_episode(r,id).map_err(EngineError)? {attack(r,id,b.id,1.)?;}
        return Ok(());
    }
    let target=if a.state=="stalking" && a.conditions.contains_key("stalkGoalX") {
        Some(Point{x:condition(&a,"stalkGoalX") as i32,y:condition(&a,"stalkGoalY") as i32})
    } else if a.conditions.get("chaseDepth").is_some_and(|&d|d!=r.depth) {
        Some(Point{x:condition(&a,"chaseExitX") as i32,y:condition(&a,"chaseExitY") as i32})
    } else if a.state=="ally" {
        quarry.as_ref().map(point).or_else(||if r.player.x>=0. {idle_ally_target(r,&a)}else{None})
    } else if a.state=="hunting" {a.last_seen.as_ref().map(|p|Point{x:p.x as i32,y:p.y as i32})}
    else {None};
    if let Some(target)=target {pursue(r,id,target,cache,true)?;}
    else if a.state=="wandering" {
        let (dx,dy)=((a.x-before.x).signum(),(a.y-before.y).signum());
        if can_traverse(r,&a,a.x+dx,a.y+dy)
            && !crate::environment::creature_hazard_at(r,&a,(a.y+dy)as usize*r.floor().unwrap().width()+(a.x+dx)as usize) {
            move_actor(r,id,dx,dy,true)?;
        }
    }
    Ok(())
}

/// One complete NPC action. Normal-only upkeep/rest stays outside bonus actions;
/// Fast adds at most one move-or-melee follow-up, never another full turn.
fn surprise_response_blocked(r:&Run,a:&Actor)->bool {
    r.simulation_revision.value_or(29.)>=52. && !a.dead.value_or(false) && a.hp>0. && a.conditions.get("monsterSurprisedThrough").is_some_and(|until|*until>=r.tick)
}
fn mark_player_surprise(r:&mut Run,source:Option<&Actor>,before:&Actor){
    if r.simulation_revision.value_or(29.)<52. || before.id==r.player.id || !matches!(before.state.as_str(),"sleeping"|"wandering")
        || !source.is_some_and(|a|crate::half_here::owner_id(r,a)==r.player.id){return;}
    let through=r.tick+if r.resolving_tick.value_or(false){0.}else{1.};
    if let Some(a)=actor_mut_trusted(r,before.id){a.conditions.insert("monsterSurprisedThrough".into(),through);}
}
fn complete_actor_action(r: &mut Run, id: f64, bonus: bool, paralyzed: &[f64], cache: &mut PathCache) -> Result<(), EngineError> {
    if actor(r,id).is_some_and(|a|surprise_response_blocked(r,a)){return Ok(());}
    let before = actor(r, id).cloned();
    let (result,receipt)=crate::revision52_speed::measure(id,bonus,||actor_action_inner(r,id,bonus,paralyzed,cache));
    result?;
    if receipt.moved && !receipt.attacked {
        if let Some(before)=before {fast_followup(r,id,&before,cache)?;}
    }
    Ok(())
}
fn actor_action_inner(r: &mut Run, id: f64, bonus: bool, paralyzed: &[f64], cache: &mut PathCache) -> Result<(), EngineError> {
        bump(r, "aiActors", 1.0);
        let mut a = actor(r, id).unwrap().clone();
        if a.dead.value_or(false) {
            if bonus { return Ok(()); }
            if crate::divine_environment::corpse_frozen(r, &a) {
                return Ok(());
            }
            a.corpse_age = Field::Value(a.corpse_age.value_or(0.0) + 1.0);
            if crate::corpses::can_revive_corpse(&a)
                && !crate::revision51_stairs::blocks_revival(r,r.floor().unwrap(),&a)
                && a.corpse_age.value_or(0.0) > 80.0
                && {
                    let mut revived = a.clone();
                    revived.state = "wandering".into();
                    !bulwark_blocks(r, &revived, a.x, a.y)
                }
                && !crate::environment::corpse_rooted(r.floor().unwrap(), r.actors_at(r.depth), &a)
                && !std::iter::once(&r.player)
                    .chain(r.actors_at(r.depth))
                    .any(|b| !b.dead.value_or(false) && b.x == a.x && b.y == a.y)
                && crate::corpses::roll_corpse_revival(&mut a, || rng::random(r))
            {
                crate::corpses::revive_corpse(&mut a, r.tick);
                *actor_mut_trusted(r, id).unwrap() = a;
                crate::ichor::mercurialize_on_revival(r, id, r.depth);
                crate::environment::refresh_burning_from_fire(r, id).map_err(EngineError)?;
                a = actor(r, id).unwrap().clone();
                crate::animation_enemy::revived(r,&a);
                if vision::visible_actor(r, &a) {
                    log(r, format!("{} rises heartless.", crate::actor_presentation::name(r, &a)), "enemy");
                }
            }
            *actor_mut_trusted(r, id).unwrap() = a;
            return Ok(());
        }
        if crate::ichor::squirm_if_stranded(r, id) {
            if !bonus { ai::advance_rest(r, &mut a, false); }
            *actor_mut_trusted(r, id).unwrap() = a;
            return Ok(());
        }
        if paralyzed.contains(&id) {
            if !bonus { ai::advance_rest(r, &mut a, false); }
            *actor_mut_trusted(r, id).unwrap() = a.clone();
            if r.simulation_revision.value_or(29.) < 46. && vision::visible_actor(r, &a) {
                log(
                    r,
                    format!("{} is paralyzed and cannot act.", crate::actor_presentation::name(r, &a)),
                    "enemy",
                );
            }
            return Ok(());
        }
        if crate::revision50_control_amulets::discord_turn(r,id,bonus)? {return Ok(());}
        if a.state == "ally" {
            ally_turn(r, id, bonus, cache)?;
            return Ok(());
        }
        if a.state == "neutral" {
            let ready = a.abilities.iter().any(|s| s == "enemy:beast:sphinx")
                && a.conditions
                    .get("monsterSphinxHuntAt")
                    .is_some_and(|t| *t <= r.tick);
            if !ready {
                let resting = !bonus && ai::advance_rest(r, &mut a, false);
                *actor_mut_trusted(r, id).unwrap() = a;
                if !resting {
                    monster_ai_turn(r, id, false)?;
                }
                return Ok(());
            }
            ai::wake(&mut a, true);
            a.state = "wandering".into();
        }
        let hostiles: Vec<_> = std::iter::once(&r.player)
            .chain(
                r.actors_at(r.depth)
                    .iter()
                    .filter(|b| !b.dead.value_or(false) && b.state == "ally"),
            )
            .filter(|b| b.x >= 0. && b.y >= 0. && crate::dreamwalking::same_plane(r,&a,b))
            .cloned()
            .collect();
        let mut detected_target: Option<Actor> = None;
        let mut detected_player = false;
        a.conditions.insert("seesPlayer".into(), 0.0);
        for p in &hostiles {
            bump(r, "sightTests", 1.0);
            let locked = !crate::revision51_perception::enabled(r) && a.state == "hunting"
                && condition(p, "invisible") <= 0.0
                && condition(&a, "targetId") == p.id
                && crate::portals::actor_distance(r,&a,p) < 5.0
                && vision::line_of_sight(r, &a, p)
                && vision::light(r, p) > -100.0;
            let detected = if crate::revision51_perception::enabled(r) {
                let chance=vision::detection_chance(r,&a,p);
                chance>=100. || (chance>0. && rng::random(r)*100.<chance)
            } else { locked || rng::random(r) * 100.0 < vision::detection_chance(r, &a, p) };
            if p.id == r.player.id {
                a.conditions
                    .insert("seesPlayer".into(), if detected { 1.0 } else { 0.0 });
                if detected {
                    detected_player = true;
                }
            }
            if detected && (crate::half_here::is_clone(p) || crate::revision50_nightmares::is_decoy(p)) {
                detected_player = true;
            }
            if detected
                && detected_target.as_ref().is_none_or(|b| {
                    crate::portals::actor_distance(r,&a,p) < crate::portals::actor_distance(r,&a,b)
                })
            {
                detected_target = Some(p.clone());
            }
        }
        if a.abilities.iter().any(|s| s == "enemy:beast:sphinx")
            && condition(&a, "monsterSphinxLostSight") == 0.0
            && a.conditions
                .get("monsterSphinxHuntAt")
                .is_some_and(|t| *t <= r.tick)
            && crate::half_here::origins(r).iter().any(|b| {
                b.id == condition(&a, "monsterSphinxProvokerId") && vision::line_of_sight(r, &a, b)
            })
        {
            detected_player = true;
        }
        if detected_player {
            a.conditions.insert("monsterUnseenTurns".into(), 0.0);
        }
        if let Some(p) = &detected_target {
            if a.state == "sleeping" {
                ai::wake(&mut a, true);
                if crate::revision51_perception::enabled(r) {
                    a.state="hunting".into();
                    a.conditions.insert("targetId".into(),p.id);
                    a.last_seen=ActorLastSeen{x:p.x,y:p.y}.into();
                }
                *actor_mut_trusted(r, id).unwrap() = a;
                return Ok(());
            }
            a.state = "hunting".into();
            for k in [
                "chaseDepth",
                "chaseExitX",
                "chaseExitY",
                "chaseLandingX",
                "chaseLandingY",
                "chaseChasm",
            ] {
                a.conditions.shift_remove(k);
            }
            a.conditions.insert("targetId".into(), p.id);
            a.last_seen = Field::Value(ActorLastSeen { x: p.x, y: p.y });
        }
        let resting = !bonus && ai::advance_rest(r, &mut a, detected_player);
        *actor_mut_trusted(r, id).unwrap() = a.clone();
        if resting {
            return Ok(());
        }
        if crate::creature_feeding::take_turn(r, id, !slow_rest(r, &a), bonus)? {
            return Ok(());
        }
        if !bonus && slow_rest(r, &a) {
            if r.simulation_revision.value_or(29.) < 46. && vision::visible_actor(r, &a) {
                log(r, format!("{} pauses; it is Slow.", crate::actor_presentation::name(r, &a)), "enemy");
            }
            return Ok(());
        }
        if ai::recover_ranged(r, id) {
            crate::monster_caster::retreat(r, id, detected_target.as_ref().map(|a| a.id), true)
                .map_err(EngineError)?;
            return Ok(());
        }
        if monster_ai_turn_target(r, id, detected_target.as_ref().map(|a| a.id), false)? {
            return Ok(());
        }
        if crate::monster_stalking::turn(r, id, detected_target.as_ref().map(|a| a.id))
            .map_err(EngineError)?
        {
            return Ok(());
        }
        a = actor(r, id).unwrap().clone();
        let quarry = detected_target.clone().or_else(|| {
            hostiles
                .iter()
                .find(|b| b.id == condition(&a, "targetId"))
                .cloned()
        });
        let detected = detected_target.is_some();
        if a.state == "hunting" {
            if let Some(target) = detected_target.as_ref() {
                if ai::enemy_action(r, id, target.id)? {
                    return Ok(());
                }
            }
            if detected {}
            let knows = quarry.as_ref().is_some_and(|p| {
                detected_target.as_ref().is_some_and(|b| b.id == p.id)
                    || a.last_seen
                        .as_ref()
                        .is_some_and(|t| t.x == p.x && t.y == p.y)
            });
            if knows
                && quarry.as_ref().is_some_and(|p| {
                    crate::portals::actor_distance(r,&a,p) <= std::f64::consts::SQRT_2
                })
            {
                if !crate::monster_statuses::try_hemophilia_episode(r, id).map_err(EngineError)? {
                    attack(r, id, quarry.as_ref().unwrap().id, 1.0)?;
                }
            } else if let Some(t) = a.last_seen.as_ref() {
                if crate::revision51_stairs::at_chase_exit(r,&a,t.x,t.y) {
                    let a = actor_mut_trusted(r, id).unwrap();
                    a.state = "wandering".into();
                    a.last_seen = Field::Missing;
                    a.conditions.shift_remove("targetId");
                } else {
                    pursue(
                        r,
                        id,
                        Point {
                            x: t.x as i32,
                            y: t.y as i32,
                        },
                        cache,
                        false,
                    )?;
                }
            }
        } else if a.state == "wandering" && rng::random(r) < 0.5 {
            let dx = (rng::random(r) * 3.0).floor() - 1.0;
            let dy = (rng::random(r) * 3.0).floor() - 1.0;
            if condition(&a, "confused") > 0.0 {
                move_actor(r, id, dx, dy, false)?;
                return Ok(());
            }
            let safe = |d: Point| {
                let x = a.x + d.x as f64;
                let y = a.y + d.y as f64;
                can_traverse(r, &a, x, y)
                    && !crate::environment::creature_hazard_at(
                        r,
                        &a,
                        y as usize * r.floor().unwrap().width() + x as usize,
                    )
                    && !(r.player.x == x && r.player.y == y && crate::dreamwalking::same_plane(r,&a,&r.player))
                    && !r
                        .actors_at(r.depth)
                        .iter()
                        .any(|b| !b.dead.value_or(false) && b.id != id && b.x == x && b.y == y && crate::dreamwalking::same_plane(r,&a,b))
            };
            let desired = Point {
                x: dx as i32,
                y: dy as i32,
            };
            let step = if safe(desired) {
                Some(desired)
            } else if crate::environment::creature_hazard_at(
                r,
                &a,
                a.y as usize * r.floor().unwrap().width() + a.x as usize,
            ) {
                movement::DIRECTIONS.into_iter().find(|d| safe(*d))
            } else {
                None
            };
            if let Some(d) = step {
                move_actor(r, id, d.x as f64, d.y as f64, false)?;
            }
        }
    Ok(())
}
/// Inactive floors retain their own transfer and rest rules. The bonus performs
/// one full known-target action without perceiving the off-map proxy player or
/// changing a pending stair chase into wandering. No world clock advances.
pub(crate) fn offscreen_haste_actions(r:&mut Run)->Result<(),EngineError> {
    if r.simulation_revision.value_or(29.)<52. {return Ok(());}
    let mut cache=PathCache::new();
    for id in ai::phase_order(r.actors_at(r.depth),true) {
        let Some(before)=actor(r,id).cloned() else {continue};
        if before.dead.value_or(false) || before.hp<=0. || ai::sleeping(&before)
            || crate::dreamwalking::inert(r,&before) || crate::half_here::is_clone(&before)
            || condition(&before,"paralyzed")>0. || crate::ichor::stranded(r,&before)
            || !ai::is_hasted_in(r,&before)
            || before.abilities.iter().any(|s|s=="enemy:beast:slow"||s=="enemy:pale:slow") {continue;}
        let (result,receipt)=crate::revision52_speed::measure(id,true,|| -> Result<(),EngineError> {
            if crate::revision50_control_amulets::discord_turn(r,id,true)? {return Ok(());}
            if crate::creature_feeding::take_turn(r,id,false,true)? {return Ok(());}
            if before.state!="hunting" && condition(&before,"monsterInventorFleeing")<=0. {return Ok(());}
            if ai::recover_ranged(r,id) {return Ok(());}
            let quarry=r.actors_at(r.depth).iter().find(|b| !b.dead.value_or(false)&&b.hp>0.
                && condition(&before,"targetId")==b.id && crate::dreamwalking::same_plane(r,&before,b)
                && before.last_seen.as_ref().is_some_and(|p|p.x==b.x&&p.y==b.y)).cloned();
            if monster_ai_turn_target(r,id,quarry.as_ref().map(|b|b.id),false)? {return Ok(());}
            let Some(a)=actor(r,id).cloned() else {return Ok(());};
            if let Some(b)=quarry {
                if ai::enemy_action(r,id,b.id)? {return Ok(());}
                if crate::portals::actor_distance(r,&a,&b)<=std::f64::consts::SQRT_2 {
                    if !crate::monster_statuses::try_hemophilia_episode(r,id).map_err(EngineError)? {attack(r,id,b.id,1.)?;}
                    return Ok(());
                }
            }
            let target=if a.conditions.get("chaseDepth").is_some_and(|&d|d!=r.depth) {
                Some(Point{x:condition(&a,"chaseExitX") as i32,y:condition(&a,"chaseExitY") as i32})
            } else {a.last_seen.as_ref().map(|p|Point{x:p.x as i32,y:p.y as i32})};
            if let Some(t)=target {
                if !crate::revision51_stairs::at_chase_exit(r,&a,t.x as f64,t.y as f64) {pursue(r,id,t,&mut cache,false)?;}
            }
            Ok(())
        });
        result?;
        if receipt.moved&&!receipt.attacked {fast_followup(r,id,&before,&mut cache)?;}
    }
    Ok(())
}
pub fn tick_condition(a: &mut Actor, k: &str) {
    if condition(a, k) > 0.0 {
        let fresh = format!("{k}Fresh");
        if condition(a, &fresh) != 0.0 {
            a.conditions.shift_remove(&fresh);
        } else {
            *a.conditions.get_mut(k).unwrap() -= 1.0;
        }
    }
}
fn turn(r: &mut Run) -> Result<(), EngineError> {
    if r.simulation_revision.value_or(29.)>=52. { r.fast_action_phase=Field::Missing; }
    let _timing_function = crate::timing::span("tick.total");
    if r.status != "playing" {
        return Ok(());
    }
    let _timing_stage = crate::timing::span("tick.clocks_and_setup");
    crate::corpse_currents::settle_falls(r).map_err(EngineError)?;
    // Leaving the Duel area ends it before the next upkeep charge.
    crate::revision50::settle_duel(r);
    r.tick += 1.0;
    r.resolving_tick = Field::Value(true);
    crate::creature_feeding::begin_turn(r);
    let mut deferred_phase_fall = false;
    let had_phase = r.player.active.iter().any(|id| id == "etill-phase-through");
    let expired: Vec<_> = r
        .player
        .conditions
        .iter()
        .filter(|(k, v)| k.starts_with("remembered:") && r.tick >= **v)
        .map(|(k, _)| k.clone())
        .collect();
    for key in expired {
        let id = &key[11..];
        r.player.abilities.retain(|a| a != id);
        r.player.active.retain(|a| a != id);
        r.player.conditions.shift_remove(&key);
    }
    crate::equipment::sync_ownership(r)?;
    if had_phase && !r.player.active.iter().any(|id| id == "etill-phase-through") {
        crate::divine_environment::settle_phase_loss(r, false).map_err(EngineError)?;
    }
    if r.status != "playing" {
        return Ok(());
    }
    if !crate::revision50::enabled(r) && condition(&r.player, "cameTurns") > 0.0 {
        *r.player.conditions.get_mut("cameTurns").unwrap() -= 1.0;
        if condition(&r.player, "cameTurns") == 0.0 {
            r.player.conditions.shift_remove("cameTarget");
        }
    }
    crate::abilities::refresh_unspeak(r)?;
    crate::equipment::flush_health(r);
    crate::half_here::sync(r);
    crate::events::next_action(r);
    crate::monster_horn::world_turn(r);
    crate::epidite::world_turn(r)?;
    crate::lifecycle::world_turn(r).map_err(EngineError)?;
    if r.status != "playing" {
        r.resolving_tick = Field::Missing;
        return Ok(());
    }
    let mut cache = PathCache::new();
    let ids: Vec<_> = std::iter::once(r.player.id)
        .chain(r.actors_at(r.depth).iter().filter(|a|!crate::revision50_nightmares::is_decoy(a)).map(|a| a.id))
        .collect();
    let active_depth = r.depth;
    let expiring_flight: Vec<_> = ids
        .iter()
        .copied()
        .filter(|id| {
            let a = actor(r, *id).unwrap();
            (condition(a,"flying")>0.0 || (crate::revision50::enabled(r) && condition(a,"miniMe")==1.0 && a.active.iter().any(|id|id=="etill-phase-through"))) && flying_in(r,a)
        })
        .collect();
    let paralyzed: Vec<_> = ids
        .iter()
        .copied()
        .filter(|id| condition(actor(r, *id).unwrap(), "paralyzed") > 0.0)
        .collect();
    let mut expired_allies = Vec::new();
    let current_tick = r.tick;
    let player_id = r.player.id;
    drop(_timing_stage);
    let _timing_stage = crate::timing::span("tick.actor_upkeep");
    for id in ids {
        crate::events::next_action(r);
        if actor(r, id).is_none_or(|a| a.dead.value_or(false) || (crate::dreamwalking::is_nightmare(a) && !crate::dreamwalking::dream_view(r))) {
            continue;
        }
        let before_status=crate::events::enabled(r).then(||actor(r,id).unwrap().clone());
        ai::enemy_turn_effects(r, id)?;
        crate::revision52_enemy_followups::tick_bleed(r, id)?;
        // Status damage may remove transient actors (for example a lethal
        // venom tick on Mini-Me). The upkeep ID snapshot is then stale.
        if r.status != "playing" || actor(r, id).is_none_or(|a| a.dead.value_or(false)) {
            continue;
        }
        let suppressed = crate::divine_environment::healing_suppressed(r, actor(r, id).unwrap());
        let water_regen =
            crate::divine_environment::drown_regen_multiplier(r, actor(r, id).unwrap());
        let hp_regen_rate = crate::revision50_rings::natural_hp_rate(r, actor(r, id).unwrap());
        let before_regen=actor(r,id).unwrap().clone();
        let can_regenerate = before_regen.hp > 0. || r.simulation_revision.value_or(29.) < 52.;
        let a = actor_mut_trusted(r, id).unwrap();
        if a.dead.value_or(false) {
            continue;
        }
        let endpoint = crate::half_here::is_clone(a);
        if !endpoint && !suppressed && can_regenerate {
            a.hp = (a.hp + a.max_hp * hp_regen_rate * crate::lifecycle::regen_multiplier(a) * water_regen)
                .min(a.max_hp);
        }
        let after_regen=a.clone();
        let regen_key=if before_regen.abilities.iter().any(|s|s=="enemy:beast:regeneration"){"E07"}else{"E70"};
        crate::animation_enemy::healed(r,&before_regen,&after_regen,regen_key);
        if water_regen>1. && after_regen.hp>before_regen.hp {crate::animation_player::local(r,&after_regen,"P27","stitch",crate::events::Stage::Health);}
        let a=actor_mut_trusted(r,id).unwrap();
        if !endpoint && a.abilities.iter().any(|id| id == "numo-revert") {
            let charges = condition(a, "revertCharges");
            a.conditions
                .insert("revertCharges".into(), (charges + 1.0).min(5.0));
        }
        let mut expired = false;
        let keys: Vec<_> = a
            .conditions
            .keys()
            .filter(|k| k.starts_with("buffTicks:"))
            .cloned()
            .collect();
        for k in keys {
            let n = a.conditions.get_mut(&k).unwrap();
            *n -= 1.0;
            if *n <= 0.0 {
                a.conditions.shift_remove(&k.replace("buffTicks:", "buff:"));
                a.conditions.shift_remove(&k);
                expired = true;
            }
        }
        if expired {
            if id == player_id || endpoint {
                crate::equipment::sync_health(r, false);
            } else {
                crate::equipment::sync_actor_health(r,id)?;
            }
        }
        let Some(a) = actor_mut_trusted(r, id) else { continue };
        for k in ["weakened", "emboldened"] {
            if condition(a, k) > 0.0 {
                *a.conditions.get_mut(k).unwrap() -= 1.0;
            }
        }
        for k in ["reacted", "hitBack", "angelUsed"] {
            a.conditions.shift_remove(k);
        }
        let timed: Vec<String> = a
            .conditions
            .iter()
            .filter(|(k, v)| {
                [
                    "paralyzed",
                    "bleeding",
                    "flying",
                    "hasted",
                    "temporaryAlly",
                    "dragonFlight",
                    "magicSensitive",
                ]
                .contains(&k.as_str())
                    && **v > 0.0
            })
            .map(|(k, _)| k.clone())
            .collect();
        for k in timed {
            if k == "bleeding" {
                let Some(body) = actor(r, id).cloned() else {
                    break;
                };
                let source = body
                    .conditions
                    .get("bleedSourceId")
                    .copied()
                    .filter(|sid| actor(r, *sid).is_some());
                let (amount, exact_scale) = crate::divine_reactions::bleeding_packet(r.simulation_revision.value_or(29.0), &body);
                damage_environment(r, id, amount, source, "bleeding", exact_scale)?;
            }
            let Some(body) = actor_mut_trusted(r, id) else { break };
            tick_condition(body, &k);
            if k == "temporaryAlly" && condition(body, &k) == 0.0 {
                expired_allies.push(id);
            }
        }
        if actor(r, id).is_none() {
            continue;
        }
        crate::abilities::tick_nightmare(r, id);
        crate::revision50_nightmares::tick_actor(r,id);
        let revision=r.simulation_revision.value_or(29.);
        let a = actor_mut_trusted(r, id).unwrap();
        if !endpoint && a.abilities.iter().any(|id| id == "morae-fortune-teller") {
            if revision >= 52. {
                crate::fortune_schedule::upkeep(a,current_tick);
            } else if current_tick % 20.0 == 0.0 {
                let fortunes=a.conditions.get("fortunes").copied().unwrap_or(3.0);
                a.conditions.insert("fortunes".into(),(fortunes+1.0).min(3.0));
            }
        }
        if !endpoint {
            deferred_phase_fall |= crate::dreamwalking::with_actor(r,id,|r|crate::abilities::pay_upkeep(r, id))?;
        }
        if let Some(before)=before_status{crate::animation_enemy::expired_statuses(r,&before);}
    }
    drop(_timing_stage);
    let _timing_stage = crate::timing::span("tick.pre_ai_sync_riptide");
    crate::half_here::sync(r);
    for a in r.actors_slice_mut(r.depth) {
        a.conditions.shift_remove("riptide");
        a.conditions.shift_remove("silenced");
    }
    if r.player.active.iter().any(|s| s == "dawel-unspeak") {
        let target = condition(&r.player, "unspeakTarget");
        if let Some(a) = r
            .actors_slice_mut(r.depth)
            .iter_mut()
             .find(|a| a.id == target && !a.dead.value_or(false))
        {
            a.conditions.insert("silenced".into(), 1.0);
        }
    }
    crate::divine_environment::apply_riptide(r).map_err(EngineError)?;
    drop(_timing_stage);
    let _timing_stage = crate::timing::span("tick.ai");
    for id in ai::phase_order(r.actors_at(r.depth), false) {
        crate::events::next_action(r);
        if r.status != "playing" {
            break;
        }
        if actor(r, id).is_none_or(|a|crate::half_here::is_clone(a) || crate::dreamwalking::inert(r,a)) {
            continue;
        }
        complete_actor_action(r, id, false, &paralyzed, &mut cache)?;
    }
    drop(_timing_stage);
    let _timing_stage = crate::timing::span("tick.late_statuses");
    for id in expired_allies {
        if let Some(a) = actor_mut_trusted(r, id) {
            a.state = "wandering".into();
            a.perfect = Field::Value(false);
            a.order = Field::Missing;
        }
    }
    if r.status == "playing" {
        let ids: Vec<_> = std::iter::once(r.player.id)
            .chain(r.actors_at(r.depth).iter().filter(|a|!crate::revision50_nightmares::is_decoy(a)).map(|a| a.id))
            .collect();
        for id in ids {
        crate::events::next_action(r);
            if actor(r, id).is_some_and(|a| !a.dead.value_or(false)) {
                crate::revision50_control_amulets::tick_actor(r,id)?;
                crate::monster_statuses::tick_poisoned(r, id).map_err(EngineError)?;
            }
            let status_before=actor(r,id).cloned();
            let Some(a) = actor_mut_trusted(r, id) else { continue };
            if !a.dead.value_or(false) {
                for k in [
                    "telepathic",
                    "invisible",
                    "confused",
                    "hemophilia",
                    "poisoned",
                    "slowed",
                ] {
                    tick_condition(a, k);
                }
                crate::revision50_amulet_effects::tick_statuses(a);
            }
            if let Some(before)=status_before {crate::animation_amulets::expired_statuses(r,&before);crate::animation_enemy::expired_statuses(r,&before);}
        }
        crate::half_here::sync(r);
        crate::events::next_action(r);
        {
            let _timing_current = crate::timing::span("tick.currents");
            crate::currents::tick_currents(r).map_err(EngineError)?;
        }
        let mut pending_falls = expiring_flight.clone();
        if deferred_phase_fall {
            pending_falls.extend(crate::half_here::origins(r).iter().map(|a| a.id));
        }
        {
            let _timing_environment = crate::timing::span("tick.environments");
            let before_status:Vec<_>=if crate::events::enabled(r){std::iter::once(&r.player).chain(r.actors_at(r.depth)).filter(|a|crate::events::actor_observed(r,a)).cloned().collect()}else{Vec::new()};
            crate::dreamwalking::tick_environments(r,&pending_falls).map_err(EngineError)?;
            for before in before_status{crate::animation_enemy::expired_statuses(r,&before);}
        }
        let tick = r.tick;
        r.floor_mut().unwrap().last_tick = tick;
        let collapse_before = r.collapse.as_ref().is_some();
        {
            let _timing_offscreen = crate::timing::span("tick.inactive_floors");
            crate::offscreen::tick_inactive_floors(r).map_err(EngineError)?;
        }
        if !collapse_before && r.collapse.as_ref().is_some() { crate::finale::announce_start(r); }
    }
    drop(_timing_stage);
    let _timing_stage = crate::timing::span("tick.final_settlement");
    crate::finale::world_turn(r).map_err(EngineError)?;
    let expired: Vec<_> = expiring_flight
        .into_iter()
        .filter(|id| actor(r, *id).is_some_and(|a| !flying_in(r, a)))
        .collect();
    crate::travel::fall_actors_one_level(r, active_depth, &expired, None).map_err(EngineError)?;
    let tick = r.tick;
    let expired_bulwarks:Vec<_>=r.floor().and_then(|f|f.bulwarks.as_ref()).map(|b|b.iter().filter(|(_,deadline)|**deadline<=tick).filter_map(|(key,_)|key.parse::<usize>().ok()).collect()).unwrap_or_default();
    for f in &mut r.floors {
        if f.bulwarks
            .as_ref()
            .is_some_and(|b| b.has_due(tick))
        {
            if let Some(b) = crate::mutation_journal::make_mut(f).bulwarks.as_mut() {
                b.expire(tick);
            }
        }
    }
    crate::events::at(r,"P11","dissolve",crate::events::Stage::Special,&crate::animation_amulets::cell_points(r,&expired_bulwarks));
    crate::equipment::flush_health(r);
    if r.status=="playing" {crate::revision50_terrain::clay_end_turn(r)?;}
    monster_prepare_player_turn(r)?;
    crate::creature_feeding::end_turn(r);
    r.resolving_tick = Field::Missing;
    if deferred_phase_fall {
        crate::divine_environment::settle_phase_loss(r, false).map_err(EngineError)?;
    }
    Ok(())
}
pub fn remember(r: &mut Run) {
    let _timing_function = crate::timing::span("finish.remember");
    let Some(f) = r.floor() else { return };
    let w = f.width();
    let mask = if crate::dreamwalking::enabled(r) {vision::native_terrain_visibility(r)} else {vision::terrain_visibility(r)};
    {
        let f = r.floor_mut().unwrap();
        if f.remembered_tiles.as_ref().is_none() {
            f.remembered_tiles = Field::Value(f.tiles.clone());
        }
        if f.remembered_doors.as_ref().is_none() {
            f.remembered_doors = Field::Value(
                f.doors
                    .as_ref()
                    .map(|doors| {
                        doors
                            .iter()
                            .map(|(k, d)| {
                                (
                                    k.clone(),
                                    FloorRememberedDoorsEntry {
                                        open: d.open,
                                        burning: d.burning.clone(),
                                    },
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            );
        }
        if let Some(v) = f.substrates.as_ref() {
            f.remembered_substrates.get_or_insert_with(|| v.clone());
        }
        if f.snow.as_ref().is_some() {
            f.remembered_snow
                .get_or_insert_with(|| vec![false; mask.len()]);
        }
        if f.ice.as_ref().is_some() {
            f.remembered_ice.get_or_insert_with(|| vec![false; mask.len()]);
        }
        if f.puddles.as_ref().is_some() {
            f.remembered_puddles.get_or_insert_with(|| vec![false; mask.len()]);
        }
        if f.hellfire.as_ref().is_some() {
            f.remembered_hellfire.get_or_insert_with(|| vec![false; mask.len()]);
        }
        if f.currents.as_ref().is_some() {
            f.remembered_currents
                .get_or_insert_with(|| vec![0.0; mask.len()]);
        }
        if f.ichor.as_ref().is_some() {
            f.remembered_ichor
                .get_or_insert_with(|| vec![0.0; mask.len()]);
        }
        for (i, vis) in mask.iter().copied().enumerate() {
            if vis {
                f.seen[i] = true;
                if let Some(v) = f.substrates.as_ref() {
                    f.remembered_substrates.as_mut().unwrap()[i] = v[i].clone();
                }
                if let Some(v) = f.snow.as_ref() {
                    f.remembered_snow.as_mut().unwrap()[i] = v[i];
                }
                if let Some(v) = f.ice.as_ref() {
                    f.remembered_ice.as_mut().unwrap()[i] = v[i];
                }
                if let Some(v) = f.puddles.as_ref() {
                    f.remembered_puddles.as_mut().unwrap()[i] = v[i];
                }
                if let Some(v) = f.hellfire.as_ref() {
                    f.remembered_hellfire.as_mut().unwrap()[i] = v[i];
                }
                if let Some(v) = f.currents.as_ref() {
                    f.remembered_currents.as_mut().unwrap()[i] = v[i];
                }
                if let Some(v) = f.ichor.as_ref() {
                    f.remembered_ichor.as_mut().unwrap()[i] = v[i];
                }
                f.remembered_tiles.as_mut().unwrap()[i] = f.tiles[i].clone();
                if let Some(d) = f.doors.as_ref().and_then(|m| m.get(&i.to_string())) {
                    f.remembered_doors.as_mut().unwrap().insert(
                        i.to_string(),
                        FloorRememberedDoorsEntry {
                            open: d.open,
                            burning: d.burning.clone(),
                        },
                    );
                } else {
                    f.remembered_doors
                        .as_mut()
                        .unwrap()
                        .shift_remove(&i.to_string());
                }
            }
        }
    }
    let tick = r.tick;
    let revision = r.simulation_revision.value_or(29.0);
    crate::hazard_memory::remember_observed_hazards_revision(r.floor_mut().unwrap(), &mask, tick, revision);
    let observed = |x: f64, y: f64| {
        mask.get(y as usize * w + x as usize)
            .copied()
            .unwrap_or(false)
    };
    let items: Vec<_> = r
        .ground
        .get(&r.depth.to_string())
        .into_iter()
        .flat_map(|g| g.iter())
        .filter(|e| observed(e.x, e.y) && (!(crate::phantasmii_generation::is_shard(&e.item) || crate::dreamwalking::dream_ground_item(&e.item)) || crate::dreamwalking::dream_view(r)))
        .map(|e| RunRememberedItemsEntryEntry {
            x: e.x,
            y: e.y,
            item: e.item.clone(),
        })
        .collect();
    let dream_view=crate::dreamwalking::dream_view(r);
    let remembered = r
        .remembered_items
        .get_or_insert_with(Default::default)
        .entry(r.depth.to_string())
        .or_default();
    remembered.retain(|e| (dream_view || !crate::dreamwalking::dream_ground_item(&e.item)) && !observed(e.x, e.y) && !items.iter().any(|i| i.item.id == e.item.id));
    remembered.extend(items);
}

pub fn attack_blocker(
    r: &Run,
    a: &Actor,
    b: &Actor,
    range: f64,
    secondary: bool,
) -> Option<&'static str> { attack_blocker_impl(r,a,b,range,secondary,false) }
fn attack_blocker_impl(r:&Run,a:&Actor,b:&Actor,range:f64,secondary:bool,projectile_resolved:bool)->Option<&'static str>{
    if crate::revision50_terrain::is_soldier(b) {return Some("unbreakable-clay-soldier");}
    if crate::revision50::duel_blocks(r,a,b) { return Some("fated-duel"); }
    if crate::revision50_combat::oblivious_blocks(r,b,Some(a)) { return Some("oblivious"); }
    if !crate::dreamwalking::same_plane(r,a,b) {return Some("different-plane");}
    if crate::ahvamitt::is_actor(a) {return Some("nonattacking-godbeast");}
    if crate::revision50_nightmares::is_decoy(a) || a.dead.value_or(false) || a.hp <= 0.0 {
        return Some("attacker-dead");
    }
    if crate::half_here::is_clone(a) && !secondary {
        return Some("nonattacking-clone");
    }
    if (crate::half_here::is_clone(a) && actor(r, a.id).is_none())
        || (crate::half_here::is_clone(b) && actor(r, b.id).is_none())
    {
        return Some("removed-clone");
    }
    if crate::ichor::stranded(r, a) {
        return Some("stranded");
    }
    if b.dead.value_or(false) || b.hp <= 0.0 {
        return Some("defender-dead");
    }
    if crate::enemy_traits::definition(a).is_some_and(|d| d.id == "hemogoblin-mech")
        && !a
            .monster_state
            .as_ref()
            .is_some_and(|s| s.pilot.as_ref().is_some())
    {
        return Some("inactive-mech");
    }
    if a.abilities.iter().any(|id| id == "enemy:beast:hold-only") {
        return Some("harmless");
    }
    if a.abilities.iter().any(|id| id == "enemy:beast:rooted") {
        return Some("rooted");
    }
    let hostile = crate::revision50_control_amulets::hostile(r,a,b);
    if hostile
        && (crate::abilities::endpoint_active(r,b,"morae-turning-the-wheel")
            || (range > 0.0
                && crate::portals::actor_distance(r,a,b) > 5.0
                && crate::abilities::endpoint_active(r,b,"dawel-not-there")))
    {
        return Some("protected");
    }
    if range > 0.0 && !projectile_resolved {
        let land = crate::projectiles::trace_from_actor(r, a, b.x, b.y);
        if land.actor_id != Some(b.id) && (land.x != b.x || land.y != b.y) {
            return Some("blocked-projectile");
        }
    }
    None
}

pub fn attack(r: &mut Run, aid: f64, bid: f64, multiplier: f64) -> Result<(), EngineError> {
    if let (Some(a),Some(b))=(actor(r,aid),actor(r,bid)) {if !crate::dreamwalking::same_plane(r,a,b) {return Ok(())}}
    crate::dreamwalking::with_actor(r,aid,|r|attack_base(r, aid, bid, multiplier, None, false, 0.0, 1.0, false)).map(|_| ())
}
fn attack_base(
    r: &mut Run,
    aid: f64,
    bid: f64,
    multiplier: f64,
    explicit_base: Option<f64>,
    secondary: bool,
    range: f64,
    miss_chance_multiplier: f64,
    arrow: bool,
) -> Result<bool, EngineError> {
    // Implicit player main-hand strikes include Never Without a Fight counters.
    let shard_id = if !crate::revision50::enabled(r) && crate::borge::weapon_enabled(r) && aid==r.player.id
        && range==0.0 && explicit_base.is_none() {
        r.inventory.iter().find(|i|crate::borge::is_shard(i)
            && i.equipped.as_ref().is_some_and(|s|s=="hand1")).map(|i|i.id)
    } else {None};
    let landed=std::cell::Cell::new(false);
    let wording = (aid == r.player.id && range == 0.0 && explicit_base.is_none()).then(||
        crate::attack_text::weapon(r.inventory.iter().find(|i| i.kind == "weapon"
            && i.equipped.as_ref().is_some_and(|s| s == "hand1") && i.ranged_weapon.as_ref().is_none())));
    let result=attack_base_with_fury(
        r, aid, bid, multiplier, explicit_base, secondary, range,
        miss_chance_multiplier, arrow, None, &landed, range==0.0, false, shard_id.is_some(), wording,
    )?;
    if landed.get() {if let Some(id)=shard_id {crate::borge::consume_weapon(r,id)?;}}
    Ok(result)
}
fn attack_base_with_fury(
    r: &mut Run,
    aid: f64,
    bid: f64,
    multiplier: f64,
    explicit_base: Option<f64>,
    secondary: bool,
    range: f64,
    miss_chance_multiplier: f64,
    arrow: bool,
    fury: Option<f64>,
    landed: &std::cell::Cell<bool>,
    melee: bool,
    projectile_resolved: bool,
    shard: bool,
    wording: Option<crate::attack_text::Verb>,
) -> Result<bool, EngineError> {
    attack_base_with_accuracy(r, aid, bid, multiplier, explicit_base, secondary, range, miss_chance_multiplier, arrow, fury, landed, melee, projectile_resolved, shard, wording, false, 1.0)
}
fn attack_base_with_accuracy(
    r: &mut Run,
    aid: f64,
    bid: f64,
    multiplier: f64,
    explicit_base: Option<f64>,
    secondary: bool,
    range: f64,
    miss_chance_multiplier: f64,
    arrow: bool,
    fury: Option<f64>,
    landed: &std::cell::Cell<bool>,
    melee: bool,
    projectile_resolved: bool,
    shard: bool,
    wording: Option<crate::attack_text::Verb>,
    normal_accuracy: bool,
    damage_multiplier: f64,
) -> Result<bool, EngineError> {
    if r.simulation_revision.value_or(29.)>=52. {crate::revision52_speed::record_attack(aid);}

    let _timing_function = crate::timing::span("combat.attack");
    let (Some(a), Some(b)) = (actor(r, aid).cloned(), actor(r, bid).cloned()) else {
        return Ok(false);
    };
    if surprise_response_blocked(r,&a) && crate::half_here::owner_id(r,&b)==r.player.id {return Ok(false);}
    if attack_blocker_impl(r, &a, &b, range, secondary, projectile_resolved).is_some() {
        return Ok(false);
    }
    if crate::ahvamitt::is_actor(&b) {return if aid==r.player.id {crate::ahvamitt::attack(r,bid)} else {Ok(false)};}
    if aid != r.player.id && range > 0.0 {
        actor_mut_trusted(r, aid)
            .unwrap()
            .conditions
            .insert("rangedRecovery".into(), 1.0);
    }
    let _animation_attack=crate::events::attack_scope(r);
    if crate::revision50_combat::missed_me(r, &a, &b, melee)? { return Ok(false); }
    let special_provocation = crate::monster_sphinx::provoked_with_source(r, bid, Some(&a));
    let s = combat::effective_stats(r, &a);
    let bs = combat::effective_stats(r, &b);
    let surprise = (aid == r.player.id || a.state == "ally")
        && (b.state == "sleeping" || b.state == "wandering");
    let unseen = !vision::line_of_sight(r,&b,&a) || vision::detection_chance(r,&b,&a)<=0.
        || (aid==r.player.id && condition(&b,"seesPlayer")==0.0);
    let silent = crate::revision50::enabled(r) && a.abilities.iter().any(|id|id=="dawel-silent-strike") && unseen;
    let surprise = surprise || silent;
    let angel = angel_ready(r, &a);
    let blood = crate::divine_combat::blood_weapon_ready(r, &a);
    let guaranteed = (surprise && !(aid == r.player.id && range > 0.0)) || angel || blood;
    let mut chance = if guaranteed {
        1.0
    } else {
        combat::hit_chance_for_run(
            r,
            s.accuracy
                + if a.abilities.iter().any(|id| id == "anavani-shine") {
                    combat::integer_stat(vision::light(r, &b))
                } else {
                    0.0
                },
            bs.dodge,
            if normal_accuracy { 0.0 } else { range },
        ) * multiplier
    };
    let owner_b = crate::half_here::owner_id(r, &b);
    let pool_b = actor(r, owner_b).unwrap();
    let reaction = !crate::revision50::enabled(r) && crate::abilities::endpoint_active(r,&b,"isheilah-missed-me")
        && pool_b.reactions.value_or(0.0) > 0.0
        && condition(pool_b, "reacted") == 0.0;
    if reaction {
        chance *= 0.5;
    }
    if miss_chance_multiplier != 1.0 {
        chance = 1.0 - (1.0 - chance.clamp(0.0, 1.0)) * miss_chance_multiplier;
    }
    if condition(&a, "unlucky") != 0.0 && !guaranteed {
        chance = 0.0;
    }
    if condition(&a, "lucky") != 0.0 {
        chance = 1.0;
    }
    if condition(&a, "distracted") != 0.0 {
        chance = 0.0;
    }
    if condition(&b, "paralyzed") > 0.0 {
        chance = 1.0;
    }
    for key in ["lucky", "unlucky", "distracted"] {
        if condition(&a,key)!=0. {crate::animation_player::local(r,&a,if key=="distracted"{"P20"}else{"P67"},"dissolve",crate::events::Stage::Special);}
        actor_mut_trusted(r, aid).unwrap().conditions.shift_remove(key);
    }
    if reaction {
        let b = actor_mut_trusted(r, owner_b).unwrap();
        b.reactions = Field::Value(b.reactions.value_or(0.0) - 1.0);
        b.conditions.insert("reacted".into(), 1.0);
    }

    let extra = fury.unwrap_or_else(|| {
        if !crate::revision50::enabled(r) && aid == r.player.id && !secondary {
            crate::crafting::fury_bonus(r)
        } else {
            0.0
        }
    });
    let hit = rng::random(r) < chance;
    let observed = aid == r.player.id
        || bid == r.player.id
        || vision::visible_actor(r, &a)
        || vision::visible_actor(r, &b);
    let subject = if aid == r.player.id {
        "You"
    } else if vision::visible_actor(r, &a) {
        crate::actor_presentation::name(r, &a)
    } else {
        "Something unseen"
    };
    let object = if bid == r.player.id {
        "you"
    } else if vision::visible_actor(r, &b) {
        crate::actor_presentation::name(r, &b)
    } else {
        "something unseen"
    };
    let biting = ["Biter", "Jackal", "Screamer", "Rat", "Bear", "Tiger"]
        .iter()
        .any(|x| a.name.ends_with(x));
    let verb = if range > 0.0 || arrow {
        if aid == r.player.id {
            if arrow { "shoot" } else { "throw at" }
        } else if a.name.contains("Spitter") {
            "spits at"
        } else {
            "fires at"
        }
    } else if biting {
        "bites"
    } else if aid == r.player.id {
        "strike"
    } else {
        "strikes"
    };
    let action = (r.simulation_revision.value_or(29.) >= 46.).then(||
        wording.unwrap_or_else(|| crate::attack_text::select(r, &a, range > 0.0, arrow)));
    let kind = if aid == r.player.id {
        "player"
    } else {
        "enemy"
    };
    if !hit {
        if blood {
            crate::divine_combat::consume_blood_weapon_at(r, &a, &b);
        }
        if angel {
            mark_angel(r, aid);
        }
        crate::events::strike_ranged(r, &a, &b, false, 0.0, range);
        if observed {
            log(
                r,
                action.map_or_else(|| format!(
                    "{subject} {verb} {object}, but {}.",
                    if aid == r.player.id { "miss" } else { "misses" }
                ), |action| crate::attack_text::miss(subject, object, action, aid == r.player.id)),
                kind,
            );
        }
        if let Some(target) = crate::divine_reactions::select_deflection_target(r, &b, secondary) {
            attack_base_with_accuracy(
                r,
                aid,
                target,
                multiplier,
                explicit_base,
                true,
                0.0,
                miss_chance_multiplier,
                arrow,
                None,
                landed,
                melee,
                false,
                shard,
                action,
                normal_accuracy,
                damage_multiplier,
            )?;
        }
        nova_after_attack(r, &a, &b)?;
        counter_attack(r, aid, bid, secondary)?;
        mini_me_offhand(r, &a, bid, s.offense, secondary)?;
        crate::revision50_combat::after_attack(r, &a, &b)?;
        return Ok(false);
    }
    landed.set(true);
    mark_player_surprise(r,Some(&a),&b);
    if crate::revision50_nightmares::strike(r,bid) {
        crate::events::strike_ranged(r,&a,&b,true,0.,range);
        log(r,"The waking nightmare dissolves.","player");return Ok(true);
    }
    let r50_lethal = crate::revision50_combat::lethal_hit(r, &a, &b);
    let variance = 0.75 + rng::random(r) * 0.5;
    let base = if let Some(base) = explicit_base {
        base
    } else if crate::revision50::enabled(r) && crate::half_here::owner_id(r,&a)==r.player.id {
        crate::revision50_player_attack::with_offense_for_run(r, s.offense)
    } else if aid == r.player.id {
        let weapon = r.inventory.iter().find(|i| {
            i.kind == "weapon"
                && i.equipped.as_ref().is_some_and(|s| s == "hand1")
                && i.ranged_weapon.as_ref().is_none()
        });
        weapon.map_or(30.0, |i| {
            i.damage.value_or(30.0) + i.enchantment.value_or(0.0)
        }) + s.offense * weapon.map_or(1.0, |i| i.offense_multiplier.value_or(1.0))
    } else if crate::half_here::owner_id(r, &a) == r.player.id {
        30.0 + s.offense
    } else {
        combat::creature_attack_base(r.simulation_revision.value_or(29.0), &a, s.offense)
    };
    let ordinary = if b.abilities.iter().any(|id| id == "enemy:beast:sphinx") {
        0.0
    } else {
        combat::ordinary_strike_damage_for_run(
            r,
            (base + extra) * if silent {2.0} else {1.0}
                * if angel { 2.0 } else { 1.0 }
                * if blood { 2.0 } else { 1.0 }
                * if a.abilities.iter().any(|id| id == "etill-drown")
                    && crate::divine_environment::water_at(r, &a)
                {
                    3.0
                } else {
                    1.0
                },
            variance,
            if a.abilities
                .iter()
                .any(|id| id == "enemy:beast:armor-piercing")
            {
                0.0
            } else {
                bs.dr + if crate::revision50::enabled(r) { 0.0 } else { combat::adaptive_armor_dr(actor(r, owner_b).unwrap(), &a) }
            },
            surprise,
            condition(&b, "paralyzed") > 0.0,
        )
    };
    let mut n = combat::ordinary_strike_damage_for_run(
            r,
        (base + extra) * if silent {2.0} else {1.0}
            * if angel { 2.0 } else { 1.0 }
            * if blood { 2.0 } else { 1.0 }
            * if a.abilities.iter().any(|id| id == "etill-drown")
                && crate::divine_environment::water_at(r, &a)
            {
                3.0
            } else {
                1.0
            },
        variance,
        if a.abilities
            .iter()
            .any(|id| id == "enemy:beast:armor-piercing")
        {
            0.0
        } else {
            bs.dr
                + if crate::revision50::enabled(r) { 0.0 } else { combat::adaptive_armor_dr(actor(r, owner_b).unwrap(), &a) }
                + crate::divine_reactions::surrender_bonus(r, &b, bs.defense, true)
        },
        surprise,
        condition(&b, "paralyzed") > 0.0,
    );
    // Composite doubles the rounded ordinary packet after DR, preserving zero.
    let ordinary = ordinary * damage_multiplier;
    n *= damage_multiplier;
    if shard || r50_lethal {
        // A landed Shard or Revision 50 terminal strike is terminal, independent of DR, damage immunity,
        // absorption or death rescue. Preserve the ordinary landed-hit aftermath.
        n = actor(r,owner_b).unwrap().hp.max(0.0);
    } else {
        if b.abilities.iter().any(|id| id == "enemy:beast:sphinx") { n=0.0; }
        crate::divine_reactions::pay_surrender_from(r, &b, ordinary, Some(&a));
        if n>0.0 && crate::divine_reactions::absorb_nightmare(r,&b,Some(&a)) { n=0.0; }
    }
    let mut actual_hp_damage = n.min(actor(r, owner_b).unwrap().hp.max(0.0));
    let displayed_damage = if combat::nearest_damage_enabled(r) { combat::nearest_damage(n) } else { n };
    crate::events::strike_ranged(r, &a, &b, true, displayed_damage, range);
    crate::animation_enemy::strike(r,&a,&b,true,n);
    if silent {crate::animation_player::local(r,&a,"P44","fold-outline",crate::events::Stage::Special);crate::animation_player::local(r,&b,"P44","slash",crate::events::Stage::Special);}
    if a.abilities.iter().any(|s|s=="etill-drown")&&crate::divine_environment::water_at(r,&a){crate::animation_player::local(r,&b,"P27","wave",crate::events::Stage::Special);}
    if angel {crate::animation_player::local(r,&b,"P42","slash-double",crate::events::Stage::Special);}
    if observed {
        log(
            r,
            action.map_or_else(|| format!("{subject} {verb} {object} for {displayed_damage} damage."),
                |action| crate::attack_text::hit(subject, object, action, aid == r.player.id, displayed_damage)),
            kind,
        );
    }
    if angel {
        mark_angel(r, aid);
    }
    if blood {
        crate::divine_combat::consume_blood_weapon_at(r, &a, &b);
    }
    actor_mut_trusted(r, bid)
        .unwrap()
        .conditions
        .shift_remove("paralyzed");
    if n > 0.0 {
        // Capture before a lethal packet removes the observing body's living state.
        let source_display = (r.simulation_revision.value_or(29.) >= 33.
            && owner_b == r.player.id && n >= actor(r, owner_b).unwrap().hp)
            .then(|| crate::actor_presentation::source_name(r, &a).to_string());
        if shard { crate::borge::kill_on_hit(r,bid,aid)?; }
        else {
            actual_hp_damage = apply_hp_loss(r, bid, n, !r50_lethal);
            crate::revision50_amulets::reap_direct_hit(r, &a, &b, actual_hp_damage);
            share_bonded_packet(r, bid, n)?;
            if actor(r, bid).is_some_and(|a| a.hp <= 0.0) {
                death_core_with_source(r, bid, Some(aid), surprise, None, r50_lethal, None, source_display.as_deref())?;
            }
        }
        crate::half_here::sync(r);
        after_damage(r, bid, Some(aid));
        crate::divine_combat::beat_after_landed_hit(r, &a, bid);
        if let Some(body) = actor(r, bid).cloned() {
            crate::divine_reactions::quick_hand_after_hit(r, &body);
        }
        if bid != r.player.id && crate::dreamwalking::physical(r,&b) && combat::creature_bleeds(&b) {
            splatter_blood(r, b.x, b.y);
        }
    }
    if n == 0.0 {
        crate::divine_combat::beat_after_landed_hit(r, &a, bid);
        if let Some(body) = actor(r, bid).cloned() {
            crate::divine_reactions::quick_hand_after_hit(r, &body);
        }
    }
    if actor(r, bid).is_some() && actor(r, aid).is_some() {
        ai::enemy_hit_effects(r, aid, bid);
    }
    crate::divine_reactions::apply_bleeding(r, &a, bid, n, s.offense);
    crate::revision52_enemy_followups::after_melee(r, &a, bid, actual_hp_damage, melee)?;
    let current_b = actor_any(r, bid).cloned().unwrap_or_else(|| b.clone());
    if let Some(current_a) = actor(r, aid).cloned() {
        let dr = combat::effective_stats(r, &current_a).dr;
        let adaptive = combat::adaptive_armor_dr(
            actor(r, crate::half_here::owner_id(r, &current_a)).unwrap(),
            &current_b,
        );
        if let Some(packet) = crate::divine_reactions::prepare_hit_back(
            r, &current_a, &current_b, bs.offense, dr, adaptive, secondary,
        ) {
            damage_environment_core(
                r,
                aid,
                packet,
                Some(current_b.id),
                "attack",
                false,
                Some(&current_b),
                true,
                true,
            )?;
        }
    }
    let attacker_owner = crate::half_here::owner_id(r, &a);
    let defender_owner = crate::half_here::owner_id(r, &current_b);
    if actor_any(r, attacker_owner)
        .unwrap_or(&a)
        .active
        .iter()
        .any(|id| id == "grisha-erase-metaphysical")
        && actor_any(r, defender_owner)
            .unwrap_or(&current_b).abilities.iter()
            .any(|id| !shard || id!="enemy:beast:no-revival")
    {
        if actor(r, attacker_owner).is_some() && crate::revision50_ability_energy::spend(r, attacker_owner, "grisha-erase-metaphysical", 10.0) {
            // The Shard's terminal lifecycle marker is not an erasable power.
            let choices:Vec<usize>=actor_any(r,defender_owner).unwrap_or(&current_b)
                .abilities.iter().enumerate().filter_map(|(i,id)|
                    (!shard || id!="enemy:beast:no-revival").then_some(i)).collect();
            let index = choices[(rng::random(r) * choices.len() as f64).floor() as usize];
            let removed =
                actor_any(r, defender_owner).unwrap_or(&current_b).abilities[index].clone();
            if let Some(owner) = actor_any_mut(r, defender_owner) {
                owner.abilities.remove(index);
                owner.active.retain(|id| id != &removed);
            }
            crate::half_here::sync(r);
            crate::animation_shapes::nova(r,&current_b,"P51","nova-black-small",2);
            if defender_owner == r.player.id && removed == "etill-phase-through" {
                crate::divine_environment::settle_phase_loss(r, true).map_err(EngineError)?;
            }
        } else if let Some(owner) = actor_any_mut(r, attacker_owner) {
            owner.active.retain(|id| id != "grisha-erase-metaphysical");
        }
    }
    if actor_any(r, bid).is_some() {
        let b = actor_any_mut(r, bid).unwrap();
        ai::wake(b, true);
        if b.state != "ally" && !special_provocation {
            b.state = "hunting".into();
            if n > 0.0 {
                b.last_seen = Field::Value(ActorLastSeen { x: a.x, y: a.y });
                b.conditions.insert("targetId".into(), aid);
            }
        }
        if n > 0.0
            && a.abilities
                .iter()
                .any(|id| id == "enemy:pale:gas-vulnerability")
        {
            let duration = crate::monster_statuses::status_duration(r,actor_any(r,bid).unwrap(),5.0);
            let b = actor_any_mut(r, bid).unwrap();
            b.conditions.insert("gasVulnerable".into(), 1.0);
            b.conditions.insert("gasVulnerableTicks".into(), duration);
            if let Some(b)=actor_any(r,bid){crate::animation_enemy::mark(r,b,"E58","cracked-shield",crate::events::Stage::Special);}
        }
    }
    if n > 0.0
        && a.abilities.iter().any(|id| id == "enemy:beast:lifesteal")
        && !crate::divine_combat::healing_suppressed(r, &a)
    {
        let owner = crate::half_here::owner_id(r, &a);
        let before_heal=actor(r,owner).cloned();
        if let Some(healer) = actor_mut_trusted(r, owner) {
            if healer.hp > 0.0 && !healer.dead.value_or(false) {
                healer.hp = (healer.hp + actual_hp_damage / 2.0).min(healer.max_hp);
            }
        }
        if let Some((before,after))=before_heal.as_ref().zip(actor(r,owner)){crate::animation_enemy::healed(r,before,after,"E02");if after.hp>before.hp {crate::events::link(r,&b,after,"E02","droplet-return",crate::events::Stage::Health);}}
        crate::half_here::sync(r);
    }
    crate::divine_combat::heal_melee_damage_from(r, &a, Some(&b), actual_hp_damage, melee);
    if n > 0.0
        && actor(r, aid).is_some_and(|a| a.hp > 0.0)
        && a.abilities.iter().any(|id| id == "enemy:pale:lifesteal")
        && !crate::divine_combat::healing_suppressed(r, &a)
        && enemy_spend(r, aid, 8.0)
    {
        let before=actor(r,aid).unwrap().clone();
        let a = actor_mut_trusted(r, aid).unwrap();
        a.hp = (a.hp + (n / 2.0).ceil()).min(a.max_hp);
        let after=actor(r,aid).unwrap();crate::animation_enemy::healed(r,&before,after,"E60");if after.hp>before.hp {crate::events::link(r,&b,after,"E60","droplet-return",crate::events::Stage::Health);}
    }
    nova_after_attack(r, &a, &b)?;
    counter_attack(r, aid, bid, secondary)?;
    mini_me_offhand(r, &a, bid, s.offense, secondary)?;
    crate::revision50_combat::after_attack(r, &a, &b)?;
    if !secondary
        && actor(r, aid).is_some_and(|a| a.hp > 0.0)
        && actor(r, bid).is_some_and(|a| !a.dead.value_or(false))
        && a.abilities.iter().any(|id| id == "enemy:pale:bite-repeat")
        && enemy_spend(r, aid, 8.0)
    {
        let _bite_action=crate::events::action_scope(r);
        crate::animation_enemy::mark(r,&a,"E59","bracket",crate::events::Stage::Travel);
        attack_base(
            r,
            aid,
            bid,
            multiplier,
            Some(base),
            true,
            range,
            miss_chance_multiplier,
            arrow,
        )?;
    }
    Ok(true)
}
fn mini_me_offhand(
    r: &mut Run,
    a: &Actor,
    target: f64,
    offense: f64,
    secondary: bool,
) -> Result<(), EngineError> {
    if !crate::revision50::enabled(r) && !secondary
        && condition(a, "miniMe") != 0.0
        && condition(a, "offhandDamage") > 0.0
        && actor(r, a.id).is_some_and(|a| a.hp > 0.0)
        && actor_any(r, target).is_some_and(|b| !b.dead.value_or(false))
    {
        let base = condition(a, "offhandDamage")
            + offense
                * a.conditions
                    .get("offhandOffenseMultiplier")
                    .copied()
                    .unwrap_or(1.0);
        attack_base(r, a.id, target, 0.5, Some(base), true, 0.0, 1.0, false)?;
    }
    Ok(())
}
fn nova_after_attack(r: &mut Run, attacker: &Actor, impact: &Actor) -> Result<(), EngineError> {
    let owner = crate::half_here::owner_id(r, attacker);
    if crate::abilities::endpoint_active(r,attacker,"anavani-nova") {
        let cost = crate::abilities::nova_attack_cost(r.simulation_revision.value_or(29.0));
        if crate::revision50_ability_energy::spend(r, owner, "anavani-nova", cost) {
            crate::environment::explode_with_source_snapshot(
                r,
                impact.x,
                impact.y,
                crate::environment::ExplosionOptions {
                    source: Some(attacker.id),
                    direct_impact: true,
                    ..Default::default()
                },
                None,
                attacker,
            )
            .map_err(EngineError)?;
        } else if let Some(a) = actor_mut_trusted(r, owner) {
            a.active.retain(|id| id != "anavani-nova");
        }
    }
    Ok(())
}

fn splatter_blood(r: &mut Run, x: f64, y: f64) {
    let f = r.floor().unwrap();
    let w = f.width();
    let mut cells = Vec::new();
    for dy in -1..=1 {
        for dx in -1..=1 {
            let xx = x as i32 + dx;
            let yy = y as i32 + dy;
            if xx >= 0
                && yy >= 0
                && (xx as usize) < w
                && (yy as usize) < f.height()
                && f.tiles[yy as usize * w + xx as usize] == "floor"
                && (dx != 0 || dy != 0)
            {
                cells.push(yy as usize * w + xx as usize);
            }
        }
    }
    let center = y as usize * w + x as usize;
    let droplets = cells.len().min(2 + (rng::random(r) * 4.0).floor() as usize);
    let mut selected = vec![];
    for _ in 0..droplets {
        let pick = (rng::random(r) * cells.len() as f64).floor() as usize;
        selected.push(cells.remove(pick));
    }
    let f = r.floor_mut().unwrap();
    let n = f.tiles.len();
    let blood = f.blood.get_or_insert_with(|| vec![false; n]);
    if f.tiles[center] == "floor" {
        blood[center] = true;
    }
    for i in selected {
        blood[i] = true;
    }
    f.dirty = true;
}
/// Ordinary terminal death only. Ability rescue and authored species death hooks
/// remain rejected by preflight rather than disappearing into this implementation.
fn death(
    r: &mut Run,
    id: f64,
    source: Option<f64>,
    surprise: bool,
    cause: Option<&str>,
) -> Result<(), EngineError> {
    death_core(r, id, source, surprise, cause, false)
}
fn death_core(
    r: &mut Run,
    id: f64,
    source: Option<f64>,
    surprise: bool,
    cause: Option<&str>,
    bypass: bool,
) -> Result<(), EngineError> {
    death_core_with_source(r, id, source, surprise, cause, bypass, None, None)
}
fn death_core_with_source(
    r: &mut Run,
    id: f64,
    source: Option<f64>,
    surprise: bool,
    cause: Option<&str>,
    bypass: bool,
    source_snapshot: Option<&Actor>,
    source_display: Option<&str>,
) -> Result<(), EngineError> {
    let _timing_function = crate::timing::span("combat.death");
    let a = actor(r, id).unwrap().clone();
    // Hallucinations dissolve only at the landed-strike boundary, never through
    // environmental/lifecycle death, resurrection, corpse or reward processing.
    if crate::revision50_nightmares::is_decoy(&a) {
        if let Some(a)=actor_mut_trusted(r,id) {a.hp=a.max_hp;a.dead=false.into();}
        return Ok(());
    }
    if crate::ahvamitt::is_actor(&a) {if let Some(a)=actor_mut_trusted(r,id) {a.hp=a.max_hp;a.dead=false.into();}return Ok(());}
    if crate::dreamwalking::is_nightmare(&a) {
        if a.hp<=0. && !a.dead.value_or(false) {r.actors_at_mut(r.depth).retain(|b|b.id!=id);crate::revision50::end_duels_with_target(r,id);log(r,format!("{} dissolves.",a.name),"enemy");}
        return Ok(());
    }
    let owner = crate::half_here::owner_id(r, &a);
    if owner != id {
        let active=crate::dreamwalking::is_body(&a).then(||std::mem::take(&mut r.player.active));
        let result=death_core_with_source(r, owner, source, surprise, cause, bypass, source_snapshot, source_display);
        if let Some(mut active)=active {if r.status!="playing" {active.retain(|s|s!=crate::half_here::ID);}r.player.active=active;}
        result?;
        crate::half_here::sync(r);
        return Ok(());
    }
    if a.hp > 0.0 || a.dead.value_or(false) {
        return Ok(());
    }
    actor_mut_trusted(r, id)
        .unwrap()
        .conditions
        .shift_remove("deathResolutionBlocked");
    if crate::divine_rescues::try_rescue(r, id, bypass) {
        return Ok(());
    }
    crate::animation_enemy::death(r,&a);
    actor_mut_trusted(r, id)
        .unwrap()
        .conditions
        .shift_remove("beatToDeathHits");
    let src = source
        .and_then(|s| actor_any(r, s))
        .cloned()
        .or_else(|| source_snapshot.cloned());
    let kind = if source == Some(r.player.id) {
        "player"
    } else if source.is_some() {
        "enemy"
    } else {
        "environment"
    };
    if id == r.player.id {
        r.player.hp = 0.0;
        r.status = "dead".into();
        let tick = r.tick;
        let depth = r.depth;
        let source_name = source_display.map(str::to_string).or_else(||
            src.as_ref().map(|a| crate::actor_presentation::source_name(r, a).to_string()));
        let stats = r.run_stats.get_or_insert_with(|| RunStatistics {
            version: 1.0,
            ..Default::default()
        });
        stats.death.get_or_insert_with(|| RunDeath {
            reason: source_name.as_ref().map_or("Cause not recorded".into(), |name| {
                format!("Killed by {name}")
            }),
            // Stable true provenance is not a player-facing appearance label.
            source_name: src.as_ref().map(|a| Field::Value(a.name.clone())).unwrap_or_default(),
            cause: Field::Missing,
            tick,
            depth,
            unpreventable: bypass,
        });
        crate::maintained_links::end_maintained_links_to_dead_target(r, id);
        crate::half_here::dismiss(r);
        log(
            r,
            if bypass {
                "You died unpreventably."
            } else {
                "You died."
            },
            kind,
        );
        return Ok(());
    }
    let visible = vision::visible_actor(r, &a);
    let victim = if visible {
        crate::actor_presentation::name(r, &a).to_string()
    } else {
        "something unseen".into()
    };
    // Revision 46 omits ordinary actor-kill follow-ups. Keep unowned causes,
    // surprise context and the Shard's permanent-corpse effect; death mechanics
    // continue below regardless of whether there is anything to narrate.
    let narrate_death = r.simulation_revision.value_or(29.) < 46.
        || source.is_none()
        || (source == Some(r.player.id)
            && (surprise || cause == Some("Shard of the Borge")));
    if narrate_death
        && (source == Some(r.player.id)
            || visible
            || src.as_ref().is_some_and(|s| vision::visible_actor(r, s)))
    {
        let message = if source.is_none() {
            let mut chars = victim.chars();
            let capital = chars
                .next()
                .map(|c| c.to_uppercase().to_string() + chars.as_str())
                .unwrap_or_default();
            format!(
                "{capital} {}.",
                cause
                    .map(|c| format!("is killed by {}", c.to_lowercase()))
                    .unwrap_or(if bypass { "dies unpreventably" } else { "dies" }.into())
            )
        } else if source == Some(r.player.id) && surprise {
            format!(
                "You kill {}{} while it is {}.",
                if visible { "the " } else { "" },
                victim,
                if a.state == "sleeping" {
                    "sleeping"
                } else {
                    "unaware"
                }
            )
        } else {
            let killer = src
                .as_ref()
                .map(|s| {
                    if s.id == r.player.id {
                        "You".into()
                    } else if vision::visible_actor(r, s) {
                        crate::actor_presentation::name(r, s).to_string()
                    } else {
                        "Something unseen".into()
                    }
                })
                .unwrap_or("Something unseen".into());
            format!(
                "{killer} {} {victim}.",
                if source == Some(r.player.id) {
                    "kill"
                } else {
                    "kills"
                }
            )
        };
        let message = if cause == Some("Shard of the Borge") && source == Some(r.player.id)
            && message.starts_with("You kill ") {
            format!("{message} The corpse blackens and turns to stone.")
        } else { message };
        log(r, message, kind);
    }
    {
        let a = actor_mut_trusted(r, id).unwrap();
        a.hp = 0.0;
        a.dead = Field::Value(true);
        crate::corpses::reset_corpse_revival(a);
    }
    crate::maintained_links::end_maintained_links_to_dead_target(r, id);
    if a.abilities.iter().any(|ability| ability == "enemy:beast:fungal-gas") {
        crate::fungi::sever_source(r, id);
    }
    if crate::finale::actual_death(r, &a).map_err(EngineError)? { return Ok(()); }
    if crate::monster_inventor::actual_death(r, id, point(&a), source, cause)
        .map_err(EngineError)?
    {
        return Ok(());
    }
    let carried = actor_mut_trusted(r, id)
        .unwrap()
        .monster_state
        .as_mut()
        .and_then(|s| {
            if s.carried_items
                .as_ref()
                .is_some_and(|items| !items.is_empty())
            {
                std::mem::take(&mut s.carried_items).as_ref().cloned()
            } else {
                None
            }
        });
    if let Some(items) = carried {
        for item in items {
            crate::item_placement::place_ground_item(
                r,
                r.depth,
                GroundItem {
                    x: a.x,
                    y: a.y,
                    item,
                    auto_pickup_blocked: Field::Missing,
                },
                |r, depth| crate::startup::ensure_floors(r, depth as u32),
            )
            .map_err(EngineError)?;
        }
    }
    crate::lifecycle::ursa_death_cry(r, id, point(&a));
    if a.name == "Mini-Me" {
        r.actors_at_mut(r.depth).retain(|b| b.id != id);
        r.floor_mut().unwrap().dirty = true;
        return Ok(());
    }
    if !a.heartless.value_or(false) {
        let mut item = crate::inventory::heart_for_revision(&a, r.next_id, r.simulation_revision.value_or(29.))?;
        crate::borge::mark_heart(r, &a, &mut item);
        r.next_id += 1.0;
        place_item(r, a.x, a.y, item)?;
        actor_mut_trusted(r, id).unwrap().heartless = Field::Value(true);
    }
    if a.born_from_birther.value_or(false) {
        r.actors_at_mut(r.depth).retain(|b| b.id != id);
        if visible
            || source == Some(r.player.id)
            || src.as_ref().is_some_and(|s| vision::visible_actor(r, s))
        {
            let mut chars = victim.chars();
            let capital = chars
                .next()
                .map(|c| c.to_uppercase().to_string() + chars.as_str())
                .unwrap_or_default();
            log(
                r,
                if r.simulation_revision.value_or(29.) >= 52. && a.heartless.value_or(false) { format!("{capital} collapses and vanishes.") }
                else { format!("{capital} collapses and vanishes, leaving its heart.") },
                kind,
            );
        }
    } else {
        let corpse = actor(r, id).unwrap().clone();
        crate::corpse_placement::place_corpse(r, r.depth, corpse, |r, d| {
            crate::startup::ensure_floors(r, d as u32)
        })
        .map_err(EngineError)?;
    }
    r.floor_mut().unwrap().dirty = true;
    Ok(())
}
pub fn ground_destination(
    r: &Run,
    occupied: &[(f64, f64)],
    x: f64,
    y: f64,
) -> Result<(f64, f64), EngineError> {
    let f = r.floor().unwrap();
    let w = f.width();
    let mut best = None;
    let mut best_d = f64::INFINITY;
    for (i, t) in f.tiles.iter().enumerate() {
        if ["wall", "tree", "chasm", "pit"].contains(&t.as_str()) {
            continue;
        }
        let p = ((i % w) as f64, (i / w) as f64);
        if occupied.contains(&p) {
            continue;
        }
        let d = (p.0 - x).powi(2) + (p.1 - y).powi(2);
        if d < best_d {
            best_d = d;
            best = Some(p);
        }
    }
    best.ok_or_else(|| EngineError("full-floor placement overflow".into()))
}
pub fn place_item(r: &mut Run, x: f64, y: f64, item: Item) -> Result<(), EngineError> {
    crate::item_placement::place_ground_item(
        r,
        r.depth,
        GroundItem {
            x,
            y,
            item,
            ..Default::default()
        },
        |r, d| crate::startup::ensure_floors(r, d as u32),
    )
    .map_err(EngineError)?;
    Ok(())
}

pub fn damage_environment(
    r: &mut Run,
    victim: f64,
    amount: f64,
    source: Option<f64>,
    kind: &str,
    exact_scale: bool,
) -> Result<(), EngineError> {
    damage_environment_core(r, victim, amount, source, kind, exact_scale, None, false, false)
}
/// Explicit immediate attack damage, never used by status/environment ticks.
pub fn damage_direct(r: &mut Run, victim: f64, amount: f64, source: f64, kind: &str, exact_scale: bool) -> Result<(), EngineError> {
    damage_environment_core(r, victim, amount, Some(source), kind, exact_scale, None, false, true)
}
/// Direct impact retaining the source even when the first blast victim changes it.
pub fn damage_direct_with_source_snapshot(r: &mut Run, victim: f64, amount: f64, source: &Actor, kind: &str, exact_scale: bool) -> Result<(), EngineError> {
    damage_environment_core(r, victim, amount, Some(source.id), kind, exact_scale, Some(source), false, true)
}
pub fn damage_environment_with_source_snapshot(
    r: &mut Run,
    victim: f64,
    amount: f64,
    source: &Actor,
    kind: &str,
    exact_scale: bool,
) -> Result<(), EngineError> {
    damage_environment_core(
        r,
        victim,
        amount,
        Some(source.id),
        kind,
        exact_scale,
        Some(source),
        false,
        false,
    )
}
fn damage_environment_core(
    r: &mut Run,
    victim: f64,
    amount: f64,
    source: Option<f64>,
    kind: &str,
    exact_scale: bool,
    retained_source: Option<&Actor>,
    secondary: bool,
    direct: bool,
) -> Result<(), EngineError> {
    if (direct || kind=="banshee") && victim==r.player.id && source.and_then(|id|actor_any(r,id)).or(retained_source).is_some_and(|a|surprise_response_blocked(r,a)){return Ok(());}
    let _timing_function = crate::timing::span("combat.damage_packet");
    let a = actor(r, victim)
        .ok_or_else(|| EngineError("damage victim missing".into()))?
        .clone();
    if crate::revision50_terrain::is_soldier(&a) || crate::revision50_nightmares::is_decoy(&a) || crate::ahvamitt::is_actor(&a) {return Ok(());}
    if crate::revision50_combat::oblivious_blocks(r,&a,source.and_then(|id|actor_any(r,id)).or(retained_source)) {return Ok(());}
    // Direct spells obey the same Fated Duel attacker restriction as attacks.
    // Ordinary hazards and periodic status packets retain their existing rules.
    if direct && source.and_then(|id|actor_any(r,id)).or(retained_source)
        .is_some_and(|b|crate::revision50::duel_blocks(r,b,&a)) {return Ok(());}
    let environmental=crate::dreamwalking::environment_context(r);
    if environmental && !crate::dreamwalking::contact_plane(r,&a) {return Ok(())}
    if !environmental && !matches!(kind,"poison"|"venom"|"bleeding"|"killspawnBleed"|"hemophilia"|"roots") && source.and_then(|id|actor_any(r,id)).or(retained_source).is_some_and(|b|!crate::dreamwalking::same_plane(r,&a,b)) {return Ok(())}
    if a.abilities.iter().any(|id| id == "enemy:beast:sphinx") {
        let retained = source
            .and_then(|id| actor_any(r, id))
            .cloned()
            .or_else(|| retained_source.cloned());
        crate::monster_sphinx::provoked_with_source(r, victim, retained.as_ref());
        return Ok(());
    }
    if a.dead.value_or(false)
        || !amount.is_finite()
        || (crate::half_here::owner_id(r, &a) == r.player.id && r.status != "playing")
    {
        return Ok(());
    }
    let n = combat::damage_packet(r, &a, amount, kind, exact_scale);
    if n <= 0.0 {
        return Ok(());
    }
    let has = |id: &str| a.abilities.iter().any(|v| v == id);
    if (kind == "fire" && (crate::revision51_ichor_fire::immune(r.simulation_revision.value_or(29.), &a) || has("enemy:beast:fire-immunity") || has("anavani-star-power") || (crate::revision50::enabled(r) && crate::revision50_potions::fire_immune(&a))))
        || (kind == "explosion" && (has("enemy:beast:explosion-immunity") || has("anavani-nova")))
        || (["caustic", "ichor", "breatherGas"].contains(&kind)
            && (has("grisha-entropy")
                || (kind != "ichor" && has("enemy:beast:gas-immunity"))
                || (["caustic", "breatherGas"].contains(&kind) && has("enemy:pale:gas-immunity"))))
    {
        return Ok(());
    }
    let source_snapshot = if direct && crate::revision50::enabled(r) {
        // One immediate multi-target impact keeps its original living source even
        // if an earlier victim triggers death, revival or removal of that source.
        retained_source.cloned().or_else(||source.and_then(|id|actor_any(r,id)).cloned())
    } else {
        source.and_then(|id|actor_any(r,id)).cloned().or_else(||retained_source.cloned())
    };
    if crate::divine_reactions::absorb_nightmare(r, &a, source_snapshot.as_ref()) {
        return Ok(());
    }
    if a.state == "neutral" && source == Some(r.player.id) {
        actor_mut_trusted(r, victim).unwrap().state = "hunting".into();
    }
    let cause = match kind {
        "" | "attack" => "",
        "fire" => "Fire",
        "lightning" => "Lightning",
        "cold" => "Iceblast",
        "acid" => "Acid",
        "water" if crate::revision50::enabled(r) => "Floods",
        "roots" if crate::revision50::enabled(r) => "Roots",
        "caustic" => "Caustic gas",
        "ichor" => "Caustic ichor",
        "breatherGas" if crate::revision50::enabled(r) => "Gasser gas",
        "breatherGas" => "Breather gas",
        "bleeding" | "killspawnBleed" => "Bleeding",
        "razorleaves" => "Razorleaves",
        "epidite" => "Epidite cleansing light",
        "fall" => "A fall",
        "blood-weapon" => "Blood Weapon health cost",
        "blood-letting" => "Blood Letting health cost",
        "venom" => "Venom",
        "poison" => "Poison",
        "hemophilia" => "Blood loss",
        "banshee" => "Banshee shriek",
        "explosion" => "An explosion",
        _ => return unsupported(format!("damage kind {kind}")),
    };
    if !matches!(
        kind,
        "fall" | "blood-weapon" | "blood-letting" | "attack" | ""
    ) && (victim == r.player.id || vision::visible_actor(r, &a))
    {
        log(
            r,
            if kind=="caustic" && victim==r.player.id && source_snapshot.as_ref().is_some_and(|a|crate::revision52_enemy_followups::species(a,"blorb")) {
                format!("You take {n:.0} damage from the Blorb's caustic slime.")
            } else {format!(
                "{cause} deals {n} damage to {}.",
                if victim == r.player.id {
                    "you"
                } else {
                    crate::actor_presentation::name(r, &a)
                }
            )},
            if source == Some(r.player.id) {
                "player"
            } else if source.is_some() {
                "enemy"
            } else {
                "environment"
            },
        );
    }
    if !matches!(
        kind,
        "fall" | "blood-weapon" | "blood-letting" | "attack" | ""
    ) && (victim == r.player.id || vision::visible_actor(r, &a))
    {
        if !crate::events::enabled(r) {crate::events::emit(
            r,
            crate::events::PresentationEvent {
                kind: if kind == "explosion" {
                    "burst"
                } else {
                    "impact"
                }
                .into(),
                from: None,
                to: (&a).into(),
                glyph: if kind == "explosion" {
                    Some("人".into())
                } else {
                    None
                },
                color: Some(
                    if kind == "explosion" {
                        "#fff23a"
                    } else if kind == "fire" {
                        "#ef9c4a"
                    } else {
                        "#a9be70"
                    }
                    .into(),
                ),
                label: Some(cause.into()),
                depth: r.depth,
             animation: None, },
        );}
        else {
            if kind!="hemophilia"{crate::animation_enemy::damage(r,&a,kind);}
            crate::events::cue(r,&a,"shared","puncture",crate::events::Stage::Health,&[(&a).into()]);
        }
    }
    let pool_id = crate::half_here::owner_id(r, &a);
    let source_display = (r.simulation_revision.value_or(29.) >= 33.
        && pool_id == r.player.id && n >= actor(r, pool_id).unwrap().hp)
        .then(|| source_snapshot.as_ref().map(|a| crate::actor_presentation::source_name(r, a).to_string())).flatten();
    if direct {mark_player_surprise(r,source_snapshot.as_ref(),&a);}
    let actual_hp_damage = apply_hp_loss(r, victim, n, !matches!(kind, "blood-weapon" | "blood-letting"));
    if direct {
        if let Some(attacker) = source_snapshot.as_ref() {
            crate::revision50_amulets::reap_direct_hit(r, attacker, &a, actual_hp_damage);
        }
    }
    if !secondary {
        share_bonded_packet(r, victim, n)?;
    }
    if actor(r, victim).is_some_and(|a| a.hp <= 0.0) {
        death_core_with_source(
            r,
            victim,
            source,
            false,
            if kind == "attack" || kind.is_empty() {
                None
            } else {
                Some(cause)
            },
            false,
            source_snapshot.as_ref(),
            source_display.as_deref(),
        )?;
        if pool_id == r.player.id {
            let source_name = source_display.or_else(|| source_snapshot.as_ref()
                .map(|a| crate::actor_presentation::source_name(r, a).to_string()));
            if let Some(d) = r.run_stats.as_mut().and_then(|s| s.death.as_mut()) {
                if kind != "attack" && !kind.is_empty() {
                    d.reason =
                        source_name.map_or(cause.into(), |name| format!("{cause} from {name}"));
                }
                d.cause = if kind.is_empty() {
                    Field::Missing
                } else {
                    Field::Value(kind.into())
                };
            }
        }
    }
    crate::half_here::sync(r);
    after_damage_with_source(r, victim, source, source_snapshot.as_ref(), direct || !matches!(kind, "caustic" | "breatherGas" | "fire" | "poison" | "venom" | "bleeding" | "killspawnBleed" | "hemophilia" | "roots"));
    Ok(())
}

pub fn player_attack(r: &mut Run, bid: f64) -> Result<(), EngineError> {
    if actor(r,bid).is_some_and(crate::ahvamitt::is_actor) {crate::ahvamitt::attack(r,bid)?;return Ok(());}
    if !crate::revision50::enabled(r) && crate::borge::equipped_shard_id(r).is_some() {
        if crate::borge::weapon_enabled(r) { player_shard_weapon_attack(r, bid)?; }
        else { crate::borge::equipped_attack(r, bid)?; }
        return Ok(());
    }
    let mut count = 1;
    if !crate::revision50::enabled(r) && r.player
        .active
        .iter()
        .any(|s| s == crate::abilities::ASSAULT)
    {
        if crate::revision50_ability_energy::spend(r, r.player.id, crate::abilities::ASSAULT, 10.0) {
            count = 2;
        } else {
            r.player.active.retain(|s| s != crate::abilities::ASSAULT);
        }
    }
    let offense = combat::effective_stats(r, &r.player).offense;
    let base = |item: Option<&Item>| {
        item.map_or(30.0, |i| {
            i.damage.value_or(30.0) + i.enchantment.value_or(0.0)
        }) + offense * item.map_or(1.0, |i| i.offense_multiplier.value_or(1.0))
    };
    let melee = |i: &&Item| i.kind == "weapon" && i.ranged_weapon.as_ref().is_none();
    let main = r
        .inventory
        .iter()
        .filter(melee)
        .find(|i| i.equipped.as_ref().is_some_and(|s| s == "hand1"));
    let off = r
        .inventory
        .iter()
        .filter(melee)
        .find(|i| i.equipped.as_ref().is_some_and(|s| s == "hand2"));
    let main_wording = crate::attack_text::weapon(main);
    let off_wording = crate::attack_text::weapon(off);
    let main = if crate::revision50::enabled(r) {crate::revision50_player_attack::with_offense_for_run(r, offense)} else {base(main)};
    let off = if crate::revision50::enabled(r) {None} else {off.map(|i| base(Some(i)))};
    let fury = if crate::revision50::enabled(r) {0.0} else {crate::crafting::fury_bonus(r)};
    for strike in 0..count {
        if r.player.hp <= 0.0 || actor(r, bid).is_none_or(|a| a.dead.value_or(false)) {
            break;
        }
        attack_base_with_fury(
            r,
            r.player.id,
            bid,
            1.0,
            Some(main),
            false,
            0.0,
            1.0,
            false,
            Some(if strike == 0 { fury } else { 0.0 }),
            &std::cell::Cell::new(false),
            true,
        false,
    false,
    Some(main_wording),
    )?;
        if let Some(off) = off {
            if r.player.hp > 0.0 && actor(r, bid).is_some_and(|a| !a.dead.value_or(false)) {
                attack_base_with_fury(
                    r,
                    r.player.id,
                    bid,
                    0.5,
                    Some(off),
                    false,
                    0.0,
                    1.0,
                    false,
                    Some(0.0),
                    &std::cell::Cell::new(false),
                    true,
                false,
    false,
    Some(off_wording),
    )?;
            }
        }
    }
    Ok(())
}

/// Revision39: each equipped weapon owns its own strike. Refresh slots between
/// Assault repeats because a landed Shard strike can remove its equipped item.
/// O(S*I) inventory lookup for S<=4 strikes and I carried item entries.
fn player_shard_weapon_attack(r: &mut Run, bid: f64) -> Result<(), EngineError> {
    let mut count = 1;
    if !crate::revision50::enabled(r) && r.player.active.iter().any(|s| s == crate::abilities::ASSAULT) {
        if crate::revision50_ability_energy::spend(r, r.player.id, crate::abilities::ASSAULT, 10.0) {
            count = 2;
        } else { r.player.active.retain(|s| s != crate::abilities::ASSAULT); }
    }
    let fury = crate::crafting::fury_bonus(r);
    for strike in 0..count {
        for (slot, multiplier) in [("hand1", 1.0), ("hand2", 0.5)] {
            if r.player.hp <= 0.0 || actor(r,bid).is_none_or(|a|a.dead.value_or(false)||a.hp<=0.0) { return Ok(()); }
            let item = r.inventory.iter().find(|i|i.kind=="weapon" && i.ranged_weapon.as_ref().is_none()
                && i.equipped.as_ref().is_some_and(|s|s==slot)).cloned();
            if slot=="hand2" && item.is_none() { continue; }
            let shard = item.as_ref().is_some_and(crate::borge::is_shard);
            let offense = combat::effective_stats(r,&r.player).offense;
            let base = item.as_ref().map_or(30.0+offense, |i| player_weapon_strike_base(r,i));
            let landed = std::cell::Cell::new(false);
            attack_base_with_fury(r,r.player.id,bid,multiplier,Some(base),false,0.0,1.0,false,
                Some(if strike==0 && slot=="hand1" {fury} else {0.0}),&landed,true,false,shard,
                Some(crate::attack_text::weapon(item.as_ref())))?;
            if shard && landed.get() { crate::borge::consume_weapon(r,item.unwrap().id)?; }
        }
    }
    Ok(())
}
/// Hit-resolved projectile route; collision and portal/pocket RNG are already
/// committed by the launch path. The shared combat roll decides whether it hits.
pub fn attack_shard_resolved(r:&mut Run, victim:f64, range:f64)->Result<bool,EngineError> {
    let landed=std::cell::Cell::new(false);
    attack_base_with_fury(r,r.player.id,victim,1.0,Some(0.0),false,range,1.0,false,None,
        &landed,false,true,true,None)?;
    Ok(landed.get())
}

pub fn settle_equipment_death(r: &mut Run) -> Result<(), EngineError> {
    if r.player.hp > 0.0 || r.status != "playing" {
        return Ok(());
    }
    death(
        r,
        r.player.id,
        None,
        false,
        Some("Health lost when equipment was removed"),
    )?;
    if let Some(d) = r.run_stats.as_mut().and_then(|s| s.death.as_mut()) {
        d.reason = "Health lost when equipment was removed".into();
        d.cause = Field::Value("equipment-loss".into());
    }
    Ok(())
}
pub fn can_traverse(r: &Run, a: &Actor, x: f64, y: f64) -> bool {
    if r.simulation_revision.value_or(29.)>=36. && crate::portals::step_options(r,a,x,y).iter().any(|&(ox,oy)|!actor_passable(r,a,ox,oy)||bulwark_blocks(r,a,ox,oy)||!crate::ichor::movement_allowed(r,a,ox,oy)||r.floor().is_some_and(|f|f.tiles[oy as usize*f.width()+ox as usize]=="chasm"&&!flying_in(r,a))){return false}
    if !actor_passable(r, a, x, y)
        || bulwark_blocks(r, a, x, y)
        || !crate::ichor::movement_allowed(r, a, x, y)
    {
        return false;
    }
    let f = r.floor().unwrap();
    movement::diagonal_step_allowed(
        &f.tiles,
        f.width(),
        point(a),
        Point {
            x: x as i32,
            y: y as i32,
        },
        a.active.iter().any(|id| id == "etill-phase-through"),
    ) && (f.tiles[y as usize * f.width() + x as usize] != "chasm" || flying_in(r, a))
}
fn idle_ally_target(r: &mut Run, a: &Actor) -> Option<Point> {
    if (a.x - r.player.x).abs().max((a.y - r.player.y).abs()) >= 3.0
        || !vision::line_of_sight(r, a, &r.player)
    {
        return Some(point(&r.player));
    }
    if rng::random(r) >= 0.3 {
        return None;
    }
    let choices: Vec<_> = movement::DIRECTIONS
        .iter()
        .map(|d| Point {
            x: a.x as i32 + d.x,
            y: a.y as i32 + d.y,
        })
        .filter(|p| {
            !(p.x as f64 == r.player.x && p.y as f64 == r.player.y)
                && can_traverse(r, a, p.x as f64, p.y as f64)
                && !r.actors_at(r.depth).iter().any(|b| {
                    !b.dead.value_or(false)
                        && b.id != a.id
                        && b.x == p.x as f64
                        && b.y == p.y as f64
                })
        })
        .collect();
    let w = r.floor().unwrap().width();
    let safe: Vec<_> = choices
        .iter()
        .copied()
        .filter(|p| !crate::environment::creature_hazard_at(r, a, p.y as usize * w + p.x as usize))
        .collect();
    let preferred = if !safe.is_empty() {
        safe
    } else if crate::environment::creature_hazard_at(r, a, a.y as usize * w + a.x as usize) {
        choices
    } else {
        vec![]
    };
    if preferred.is_empty() {
        None
    } else {
        Some(preferred[(rng::random(r) * preferred.len() as f64).floor() as usize])
    }
}
fn ally_turn(r: &mut Run, id: f64, haste: bool, cache: &mut PathCache) -> Result<(), EngineError> {
    crate::dreamwalking::with_actor(r,id,|r|ally_turn_inner(r,id,haste,cache))
}
fn ally_turn_inner(r: &mut Run, id: f64, haste: bool, cache: &mut PathCache) -> Result<(), EngineError> {
    let movement_only = haste && r.simulation_revision.value_or(29.) < 52.;
    let mut a = actor(r, id).unwrap().clone();
    if ai::sleeping(&a) && (haste || !slow_rest(r,&a))
        && crate::revision52_dragon_flight::landing_turn(r,id)? { return Ok(()); }
    a = actor(r,id).unwrap().clone();
    if ai::sleeping(&a) {
        if ai::should_wake_ally(r, &a) {
            ai::wake(&mut a, true);
            *actor_mut_trusted(r, id).unwrap() = a.clone();
        } else {
            if !haste {
                ai::advance_rest(r, &mut a, false);
                *actor_mut_trusted(r, id).unwrap() = a;
            }
            return Ok(());
        }
    }
    if crate::creature_feeding::take_turn(r, id, !slow_rest(r, &a), haste)? {
        return Ok(());
    }
    if !haste && slow_rest(r, &a) {
        return Ok(());
    }
    if !movement_only && ai::recover_ranged(r, id) {
        crate::monster_caster::retreat(r, id, None, true).map_err(EngineError)?;
        return Ok(());
    }
    if monster_ai_turn(r, id, movement_only)? {
        return Ok(());
    }
    let target = r
        .actors_at(r.depth)
        .iter()
        .find(|b| {
            !b.dead.value_or(false)
                && b.state != "ally"
                && b.state != "neutral"
                && (if crate::revision51_perception::enabled(r) {vision::plausibly_visible_actor(r,&a,b)}
                    else {condition(b, "invisible") <= 0.0 || condition(&a, "telepathic") > 0.0})
                && crate::portals::actor_distance(r,&a,b) <= std::f64::consts::SQRT_2
        })
        .map(|b| b.id);
    if a.perfect.value_or(false) && a.order.as_ref().is_some_and(|s| s == "p") && r.rally.is_some()
    {
        let p = r.rally.clone().unwrap();
        if a.x != p.x || a.y != p.y {
            pursue(
                r,
                id,
                Point {
                    x: p.x as i32,
                    y: p.y as i32,
                },
                cache,
                movement_only,
            )?;
        } else if !ai::should_wake_ally(r, &a) {
            actor_mut_trusted(r, id)
                .unwrap()
                .conditions
                .insert("monsterAllyRest".into(), 1.0);
            if !haste {
                let mut a = actor(r, id).unwrap().clone();
                ai::advance_rest(r, &mut a, false);
                *actor_mut_trusted(r, id).unwrap() = a;
            }
        }
    } else if !movement_only && target.is_some() {
        if !ai::enemy_action(r, id, target.unwrap())?
            && !crate::monster_statuses::try_hemophilia_episode(r, id).map_err(EngineError)?
        {
            attack(r, id, target.unwrap(), 1.0)?;
        }
    } else {
        let target_id = ai::nearest_ally_enemy(r, &a).map(|b| b.id);
        if !movement_only {
            if let Some(target) = target_id {
                if ai::enemy_action(r, id, target)? {
                    return Ok(());
                }
            }
        }
        let target = ai::nearest_ally_enemy(r, &a).map(point);
        if let Some(t) = target {
            if !haste {}
            pursue(r, id, t, cache, movement_only)?;
        } else if let Some(t) = idle_ally_target(r, &a) {
            pursue(r, id, t, cache, movement_only)?;
        } else if !ai::should_wake_ally(r, &a) {
            let mut a = actor(r, id).unwrap().clone();
            a.conditions.insert("monsterAllyRest".into(), 1.0);
            if !haste {
                ai::advance_rest(r, &mut a, false);
            }
            *actor_mut_trusted(r, id).unwrap() = a;
        }
    }
    Ok(())
}

pub fn settle_doors(r: &mut Run) {
    let w = r.floor().unwrap().width();
    let occupied: Vec<_> = std::iter::once(&r.player)
        .chain(r.actors_at(r.depth))
        .filter(|a|crate::dreamwalking::contact_plane(r,a))
        .map(|a|(a.x,a.y))
        .chain(
            r.ground
                .get(&r.depth.to_string())
                .into_iter()
                .flat_map(|g| g.iter().filter(|e|(crate::phantasmii_generation::is_shard(&e.item)||crate::dreamwalking::dream_ground_item(&e.item))==crate::dreamwalking::context_dream(r)).map(|e| (e.x, e.y))),
        )
        .map(|(x, y)| y as usize * w + x as usize)
        .collect();
    crate::environment::settle_doors(r.floor_mut().unwrap(), occupied);
}

pub fn bulwark_blocks(r: &Run, a: &Actor, x: f64, y: f64) -> bool {
    if a.id == r.player.id || a.state == "ally" || a.state == "neutral" || !in_bounds(r, x, y) {
        return false;
    }
    let f = r.floor().unwrap();
    f.bulwarks
        .as_ref()
        .and_then(|b| b.get(&(y as usize * f.width() + x as usize).to_string()))
        .is_some_and(|expiry| *expiry >= r.tick)
}

pub fn flying_in(r: &Run, a: &Actor) -> bool {
    if r.simulation_revision.value_or(29.) >= 52. && a.dead.value_or(false) {return false;}
    if crate::dreamwalking::dream_actor(r,a) {return true;}
    flying(a)
        || crate::revision50_rings::has_effect(r, a, crate::revision50_rings::FLIGHT)
        || (crate::half_here::owner_id(r, a) == r.player.id && crate::crafting::has_dream_craft(r))
}
pub fn flying(a: &Actor) -> bool {
    condition(a, "flying") > 0.0
        || a.active.iter().any(|id| id == "etill-phase-through")
        || (a.abilities.iter().any(|id| id == "enemy:beast:flight")
            || a.abilities.iter().any(|id| id == "anavani-ascend"))
        || crate::monster_vampire::flying(a)
}
fn slow_rest(r: &Run, a: &Actor) -> bool {
    r.tick % 2.0 == 0.0
        && (condition(a, "slowed") > 0.0
            || (a.abilities.iter().any(|id| id == "enemy:beast:slow")
                || a.abilities.iter().any(|id| id == "enemy:pale:slow")))
}

pub fn movement_held(r: &mut Run, id: f64) -> bool {
    let a = actor(r, id).unwrap().clone();
    if condition(&a, "forcedMove") != 0.0 {
        return false;
    }
    if crate::revision50_control_amulets::rooted(r,&a) {return true;}
    if crate::monster_grab::movement_held(r, id) {
        return true;
    }
    let f = r.floor().unwrap();
    let i = a.y as usize * f.width() + a.x as usize;
    let slimed = f
        .slime
        .as_ref()
        .and_then(|v| v.get(i))
        .is_some_and(|v| *v > 0.0);
    let muddy = f
        .mud
        .as_ref()
        .and_then(|v| v.get(i))
        .is_some_and(|v| *v > 0.0);
    let rooted = f
        .roots
        .as_ref()
        .and_then(|v| v.get(i))
        .is_some_and(|v| *v > 0.0);
    let sticky = muddy
        || rooted
        || (slimed
            && !a
                .abilities
                .iter()
                .any(|id| id == "enemy:beast:slime-walker"));
    if !sticky || flying_in(r, &a) || condition(&a, "stuck") > 0.0 {
        if crate::water_blood::enabled(r) && rooted && !flying_in(r, &a)
            && condition(&a, "stuck") > 0.0
        {
            // Only links crossing the escaped binding tile are severed. Inactive
            // links remain recorded, so fungi cannot immediately reacquire them.
            let revision = r.simulation_revision.value_or(29.);
            crate::environment::burn_roots_revision(r.floor_mut().unwrap(), i, revision);
            r.floor_mut().unwrap().dirty = true;
        }
        actor_mut_trusted(r, id).unwrap().conditions.shift_remove("stuck");
        return false;
    }
    actor_mut_trusted(r, id)
        .unwrap()
        .conditions
        .insert("stuck".into(), 1.0);
    if id == r.player.id || vision::visible_actor(r, &a) {
        log(
            r,
            format!(
                "{} against the {} and cannot move.",
                if id == r.player.id {
                    "You struggle".into()
                } else {
                    format!("{} struggles", crate::actor_presentation::name(r, &a))
                },
                if rooted {
                    "roots"
                } else if muddy {
                    "mud"
                } else {
                    "slime"
                }
            ),
            "player",
        );
    }
    true
}

pub fn monster_floor_entered(r: &mut Run) {
    let tick = r.tick;
    r.floor_mut()
        .unwrap()
        .monster_entry_tick
        .get_or_insert_with(|| tick);
}
pub fn monster_prepare_player_turn(r: &mut Run) -> Result<(), EngineError> {
    let ids: Vec<_> = r
        .actors_at(r.depth)
        .iter()
        .filter(|a| {
            !a.dead.value_or(false)
                && a.hp > 0.0
                && a.abilities.iter().any(|id| id == "enemy:beast:minotaur")
                && condition(a, "monsterChargeRemaining") <= 0.0
        })
        .map(|a| a.id)
        .collect();
    for id in ids {
        crate::events::next_action(r);
        choose_commitment(r, id)?;
    }
    Ok(())
}
fn choose_commitment(r: &mut Run, id: f64) -> Result<(), EngineError> {
    let a = actor(r, id).unwrap().clone();
    let goal = if a.state == "ally" {
        Some(
            ai::nearest_ally_enemy(r, &a)
                .map(point)
                .or_else(|| {
                    r.rally.as_ref().map(|p| Point {
                        x: p.x as i32,
                        y: p.y as i32,
                    })
                })
                .unwrap_or_else(|| point(&r.player)),
        )
    } else if a.state == "hunting" {
        a.last_seen.as_ref().map(|p| Point {
            x: p.x as i32,
            y: p.y as i32,
        })
    } else {
        None
    };
    let toward = if let Some(goal) = goal {
        let f = r.floor().unwrap();
        let allowed: Vec<_> = (0..f.tiles.len())
            .map(|i| {
                !f.grates.as_ref().is_some_and(|g| g.contains(&(i as f64)))
                    && !crate::revision51_stairs::blocked_cell(r,&a,f,i)
                && crate::ichor::habitat_path_allowed(f, &a, i)
                    && !bulwark_blocks(r, &a, (i % f.width()) as f64, (i / f.width()) as f64)
            })
            .collect();
        let harmful: Vec<_> = (0..f.tiles.len())
            .map(|i| crate::environment::creature_hazard_at(r, &a, i))
            .collect();
        let field = movement::distance_field_ordered(
            &f.tiles,
            f.width(),
            goal,
            false,
            flying_in(r, &a),
            Some(&allowed),
            Some(&harmful),
            &if crate::portals::enabled(r){crate::portals::links(f)}else{vec![]},
            &if r.simulation_revision.value_or(29.)>=37. {crate::portals::ordered_pairs(f)} else {vec![]},
        );
        let step = movement::next_step(&field, &f.tiles, point(&a));
        bump(r, "pathFields", 1.0);
        step
    } else {
        None
    };
    let options: Vec<_> = movement::DIRECTIONS
        .into_iter()
        .filter(|d| can_traverse(r, &a, a.x + d.x as f64, a.y + d.y as f64))
        .collect();
    let d = toward.unwrap_or_else(|| {
        if options.is_empty() {
            movement::DIRECTIONS[0]
        } else {
            options[(rng::random(r) * options.len() as f64).floor() as usize]
        }
    });
    let a = actor_mut_trusted(r, id).unwrap();
    a.conditions.insert("monsterChargeDx".into(), d.x as f64);
    a.conditions.insert("monsterChargeDy".into(), d.y as f64);
    a.conditions.insert("monsterChargeRemaining".into(), 2.0);
    Ok(())
}

pub fn enemy_spend(r: &mut Run, id: f64, cost: f64) -> bool {
    let id = crate::half_here::owner_id(r, actor(r, id).unwrap());
    let a = actor(r, id).unwrap().clone();
    if crate::abilities::energy_activation_silenced(r, &a) || a.energy < cost {
        return false;
    }
    resources::record_loss(r, &a, "energy", cost.min(a.energy.max(0.0)));
    actor_mut_trusted(r, id).unwrap().energy -= cost;
    crate::half_here::sync(r);
    true
}

pub fn monster_ai_turn(r: &mut Run, id: f64, movement_only: bool) -> Result<bool, EngineError> {
    monster_ai_turn_target(r, id, None, movement_only)
}
fn monster_ai_turn_target(r:&mut Run,id:f64,target:Option<f64>,movement_only:bool)->Result<bool,EngineError> {
    crate::dreamwalking::with_actor(r,id,|r|monster_ai_turn_target_inner(r,id,target,movement_only))
}
fn monster_ai_turn_target_inner(
    r: &mut Run,
    id: f64,
    target: Option<f64>,
    movement_only: bool,
) -> Result<bool, EngineError> {
    let a = actor(r, id).unwrap().clone();
    if target.and_then(|id|actor(r,id)).is_some_and(|b|!crate::dreamwalking::same_plane(r,&a,b)) {return Ok(false);}
    if crate::ahvamitt::is_actor(&a) || a.dead.value_or(false) || a.hp <= 0.0 || id == r.player.id {
        return Ok(false);
    }
    if (ai::sleeping(&a) || a.state=="neutral"
        || (a.state=="ally" && a.perfect.value_or(false) && a.order.as_ref().is_some_and(|s|s=="p")))
        && crate::revision52_dragon_flight::landing_turn(r,id)? { return Ok(true); }
    if crate::monster_inventor::turn(r, id, target, movement_only).map_err(EngineError)? {
        return Ok(true);
    }
    if ai::sleeping(&a) {
        return Ok(true);
    }
    if a.abilities.iter().any(|id| id == "enemy:beast:sphinx") {
        return Ok(crate::monster_sphinx::turn(r, id, target, movement_only));
    }
    if a.state == "ally" && a.perfect.value_or(false) && a.order.as_ref().is_some_and(|s| s == "p")
    {
        return Ok(false);
    }
    let target = target.or_else(|| {
        if a.state == "ally" {
            ai::nearest_ally_enemy(r, &a).map(|a| a.id)
        } else {
            None
        }
    });
    if crate::revision52_dragon_flight::turn(r, id, target, movement_only)? { return Ok(true); }
    if a.abilities.iter().any(|id| id == "enemy:beast:vampire")
        && crate::monster_vampire::turn(r, id, target, movement_only)?
    {
        return Ok(true);
    }
    if a.abilities.iter().any(|id| id == "enemy:beast:wendigo")
        && crate::monster_wendigo::turn(r, id, target, movement_only)?
    {
        return Ok(true);
    }
    if !a.abilities.iter().any(|id| id == "enemy:beast:minotaur") {
        return crate::monster_caster::turn(r, id, target, movement_only).map_err(EngineError);
    }
    if condition(&a, "monsterChargeRemaining") <= 0.0 {
        choose_commitment(r, id)?;
    }
    let a = actor(r, id).unwrap().clone();
    let (dx, dy) = (
        condition(&a, "monsterChargeDx"),
        condition(&a, "monsterChargeDy"),
    );
    let allied = a.state == "ally";
    let victim = std::iter::once(&r.player)
        .chain(r.actors_at(r.depth))
        .find(|b| {
            b.id != id
                && !b.dead.value_or(false)
                && b.hp > 0.0
                && crate::dreamwalking::same_plane(r,&a,b)
                && ((b.id == r.player.id || b.state == "ally") != allied)
                && crate::portals::same_position(r,b,a.x+dx,a.y+dy)
        })
        .map(|b| b.id);
    if let Some(v) = victim {
        if !movement_only
            && !crate::monster_statuses::try_hemophilia_episode(r, id).map_err(EngineError)?
        {
            attack(r, id, v, 1.0)?;
        }
    } else {
        let accepted=move_actor(r, id, dx, dy, movement_only)?;
        crate::animation_enemy::blocked_charge(r,&a,dx,dy,accepted);
    }
    let a = actor_mut_trusted(r, id).unwrap();
    *a.conditions.get_mut("monsterChargeRemaining").unwrap() -= 1.0;
    if condition(a, "monsterChargeRemaining") <= 0.0 && !a.dead.value_or(false) && a.hp > 0.0 {
        choose_commitment(r, id)?;
    }
    Ok(true)
}

pub fn attack_ranged(
    r: &mut Run,
    attacker: f64,
    victim: f64,
    base: f64,
    range: f64,
    miss_chance_multiplier: f64,
    arrow: bool,
) -> Result<bool, EngineError> {
    if r.simulation_revision.value_or(29.)>=36. && r.floor().is_some_and(crate::non_euclidean_generation::has_links) {
        let (Some(a),Some(b))=(actor(r,attacker).cloned(),actor(r,victim).cloned())else{return Ok(false)};
        if attack_blocker_impl(r,&a,&b,range,false,true).is_some(){return Ok(false)}
        if attacker!=r.player.id{actor_mut_trusted(r,attacker).unwrap().conditions.insert("rangedRecovery".into(),1.);}
        let landing=crate::projectiles::trace_committed_from_actor(r,&a,b.x,b.y);
        let Some(actual)=landing.actor_id else{return Ok(false)};
        let traveled=crate::projectiles::traveled_distance(r,&a,b.x,b.y,&landing);
        return attack_ranged_resolved(r,attacker,actual,base,traveled,miss_chance_multiplier,arrow)
    }
    let landed = std::cell::Cell::new(false);
    attack_base_with_fury(
        r,
        attacker,
        victim,
        1.0,
        Some(base),
        false,
        range,
        miss_chance_multiplier,
        arrow,
        None,
        &landed,
        range == 0.0 && !crate::revision50::enabled(r),
    false,
    false,
    None,
    )?;
    Ok(landed.get())
}
/// A projectile whose collision and stochastic path have already been resolved.
/// Re-tracing here would inspect a different branch and could reject a real hit.
pub fn attack_ranged_resolved(
    r: &mut Run,
    attacker: f64,
    victim: f64,
    base: f64,
    range: f64,
    miss_chance_multiplier: f64,
    arrow: bool,
) -> Result<bool, EngineError> {
    // Preserve the legacy second trajectory check, including its historical
    // raster re-aim quirks, on revisions1–35 and floors without pocket links.
    if r.simulation_revision.value_or(29.)<36.||!r.floor().is_some_and(crate::non_euclidean_generation::has_links) {
        return attack_ranged(r,attacker,victim,base,range,miss_chance_multiplier,arrow)
    }
    let landed = std::cell::Cell::new(false);
    attack_base_with_fury(
        r,
        attacker,
        victim,
        1.0,
        Some(base),
        false,
        range,
        miss_chance_multiplier,
        arrow,
        None,
        &landed,
        range == 0.0 && !crate::revision50::enabled(r),
    true,
    false,
    None,
    )?;
    Ok(landed.get())
}
/// Bow accuracy is distance-independent without changing projectile semantics.
/// Collision is already resolved; retain the existing legacy retrace on plain floors.
pub fn attack_bow_normal_accuracy_resolved(r: &mut Run, victim: f64, base: f64, range: f64) -> Result<bool, EngineError> {
    attack_bow_scaled_accuracy_resolved(r, victim, base, range, 1.0)
}
fn attack_bow_scaled_accuracy_resolved(r: &mut Run, victim: f64, base: f64, range: f64, scale: f64) -> Result<bool, EngineError> {
    let landed = std::cell::Cell::new(false);
    let resolved = r.floor().is_some_and(crate::non_euclidean_generation::has_links);
    attack_base_with_accuracy(r, r.player.id, victim, 1.0, Some(base), false,
        range, 1.0, true, None, &landed, false, resolved, false, None, true, scale)?;
    Ok(landed.get())
}
pub fn bow_damage_multiplier(revision: f64, item: &Item) -> f64 {
    if revision >= 52. && crate::projectiles::is_bow(item) && item.ranged_weapon.as_ref().is_some_and(|kind| kind == "composite-bow") { 2. } else { 1. }
}
pub fn attack_bow_resolved(r: &mut Run, victim: f64, item: &Item, range: f64) -> Result<bool, EngineError> {
    let base = player_weapon_strike_base(r, item);
    if crate::projectiles::normal_bow_accuracy(r, item) {
        let scale = bow_damage_multiplier(r.simulation_revision.value_or(29.), item);
        attack_bow_scaled_accuracy_resolved(r, victim, base, range, scale)
    } else { attack_ranged_resolved(r, r.player.id, victim, base, range, 0.5, true) }
}
pub fn player_weapon_strike_base(r: &Run, item: &Item) -> f64 {
    player_weapon_strike_base_for_revision(item, combat::effective_stats(r, &r.player).offense,
        r.simulation_revision.value_or(29.0))
}
/// O(1), same revision-aware fixed-component arithmetic for commits and previews.
pub fn player_weapon_strike_base_for_revision(item: &Item, offense: f64, revision: f64) -> f64 {
    if revision >= 52.0 && crate::projectiles::is_bow(item) {
        return crate::revision50_player_attack::with_offense_for_revision(revision, offense);
    }
    crate::revision50_player_attack::fixed_damage_for_revision(revision, item.damage.value_or(30.0))
        + item.enchantment.value_or(0.0) * if combat::raw_attack_damage_enabled(revision) { 0.5 } else { 1.0 }
        + offense * item.offense_multiplier.value_or(1.0) * if combat::raw_attack_damage_enabled(revision) { 0.5 } else { 1.0 }
}
/// Frozen legacy arithmetic for callers without a simulation revision.
pub fn player_weapon_strike_base_with_offense(item: &Item, offense: f64) -> f64 {
    item.damage.value_or(30.0)
        + item.enchantment.value_or(0.0)
        + offense * item.offense_multiplier.value_or(1.0)
}

pub fn max_reactions(a: &Actor) -> f64 { max_reactions_for_revision(29., a) }
/// O(K), with the frozen legacy capacity preserved for pre-R52 callers.
pub fn max_reactions_for_revision(revision: f64, a: &Actor) -> f64 {
    [
        "isheilah-missed-me",
        "isheilah-are-you-sure-about-that",
        "isheilah-quick-hand",
    ]
    .iter()
    .filter(|id| (!crate::hanged::enabled(revision) || **id != crate::hanged::ID)
        && a.abilities.iter().any(|s| s == **id))
    .count() as f64
}

pub(crate) fn angel_ready(r: &Run, a: &Actor) -> bool {
    let a = actor(r, crate::half_here::owner_id(r, a)).unwrap_or(a);
    let round = r.tick
        + if r.resolving_tick.value_or(false) {
            0.0
        } else {
            1.0
        };
    a.abilities
        .iter()
        .any(|id| id == "lichor-the-number-of-blood")
        && round % 5.0 == 0.0
        && a.conditions
            .get("angelUsedRound")
            .is_none_or(|n| *n != round)
}
fn mark_angel(r: &mut Run, id: f64) {
    let id = crate::half_here::owner_id(r, actor(r, id).unwrap());
    let round = r.tick
        + if r.resolving_tick.value_or(false) {
            0.0
        } else {
            1.0
        };
    let a = actor_mut_trusted(r, id).unwrap();
    a.conditions.insert("angelUsed".into(), 1.0);
    a.conditions.insert("angelUsedRound".into(), round);
}
fn counter_attack(r: &mut Run, aid: f64, bid: f64, secondary: bool) -> Result<(), EngineError> {
    if !secondary
        && actor(r, aid).is_some_and(|a| a.hp > 0.0)
        && actor(r, bid).is_some_and(|a| a.hp > 0.0)
        && actor(r, bid)
            .unwrap()
            .active
            .iter()
            .any(|id| id == "pelian-never-without-a-fight")
    {
        if crate::revision50_ability_energy::spend(r, bid, "pelian-never-without-a-fight", 10.0) {
            attack_base(r, bid, aid, 1.0, None, true, 0.0, 1.0, false)?;
        } else {
            let owner = crate::half_here::owner_id(r, actor(r, bid).unwrap());
            actor_mut_trusted(r, owner)
                .unwrap()
                .active
                .retain(|id| id != "pelian-never-without-a-fight");
        }
    }
    Ok(())
}
fn after_damage(r: &mut Run, victim: f64, source: Option<f64>) {
    after_damage_with_source(r, victim, source, None, true);
}
fn after_damage_with_source(
    r: &mut Run,
    victim: f64,
    source: Option<f64>,
    source_snapshot: Option<&Actor>,
    float_away_eligible: bool,
) {
    let Some(id) = source else { return };
    let Some(a) = actor_any(r, victim).cloned() else {
        return;
    };
    let Some(s) = actor_any(r, id)
        .cloned()
        .or_else(|| source_snapshot.cloned())
    else {
        return;
    };
    if a.dead.value_or(false) && s.abilities.iter().any(|id| id == "grisha-death-swift") {
        let resolving = r.resolving_tick.value_or(false);
        let owner_id = crate::half_here::owner_id(r, &s);
        let duration = actor_any(r,owner_id).map(|a|crate::monster_statuses::status_duration(r,a,3.0)).unwrap_or(3.0);
        let Some(owner) = actor_any_mut(r, owner_id) else {
            return;
        };
        let haste = condition(owner, "hasted").max(duration);
        owner.conditions.insert("hasted".into(), haste);
        if resolving {
            owner.conditions.shift_remove("hastedFresh");
        } else {
            owner.conditions.insert("hastedFresh".into(), 1.0);
        }
        // The killing action is complete before Death Swift's celebratory nova.
        // Capture the corpse/removed victim in a separate presentation boundary;
        // this does not advance the simulation clock or consume an extra turn.
        crate::events::next_action(r);
        crate::animation_shapes::nova(r,&s,"P52","nova-green-small",2);
    }
    if a.hp > 0.0
        && (float_away_eligible || r.simulation_revision.value_or(29.) < 52.)
        && !s.dead.value_or(false)
        && in_bounds(r, s.x, s.y)
        && a.active.iter().any(|id| id == "etill-float-away")
    {
        let duration = crate::monster_statuses::status_duration(r,&a,2.0);
        actor_mut_trusted(r, victim)
            .unwrap()
            .conditions
            .insert("flying".into(), condition(&a, "flying").max(duration));
        crate::animation_player::local(r,&a,"P28","rise",crate::events::Stage::Special);
        if crate::portals::enabled(r) {
            let (dx,dy)=crate::portals::direction_toward(r,&a,s.x,s.y);
            let options=crate::portals::step_options(r,&a,a.x-dx,a.y-dy);
            if actor_passable(r,&a,a.x-dx,a.y-dy) && options.iter().all(|&(x,y)|actor_passable(r,&a,x,y) && !bulwark_blocks(r,&a,x,y) && crate::portals::occupied_at(r,&a,x,y,Some(victim)).is_none()) {
                let (x,y)=crate::portals::commit_step(r,&a,a.x-dx,a.y-dy);
                let v=actor_mut_trusted(r,victim).unwrap();v.x=x;v.y=y;crate::revision50::duel_actor_changed(r,victim);crate::zillennia_ground::on_arrival(r,victim);
                if let Some(after)=actor(r,victim){crate::animation_player::moved(r,&a,after,"P28","speed");}
            }
            return;
        }
        let (x, y) = (
            a.x + if a.x == s.x {
                0.0
            } else {
                (a.x - s.x).signum()
            },
            a.y + if a.y == s.y {
                0.0
            } else {
                (a.y - s.y).signum()
            },
        );
        if passable(r, x, y)
            && !r
                .floor()
                .unwrap()
                .doors
                .as_ref()
                .and_then(|d| {
                    d.get(&(y as usize * r.floor().unwrap().width() + x as usize).to_string())
                })
                .is_some_and(|d| !d.open)
            && !bulwark_blocks(r, &a, x, y)
            && !(victim != r.player.id && r.player.x == x && r.player.y == y)
            && !r
                .actors_at(r.depth)
                .iter()
                .any(|b| b.id != victim && !b.dead.value_or(false) && b.x == x && b.y == y)
        {
            let target = actor_mut_trusted(r, victim).unwrap();
            target.x = x;
            target.y = y;
            crate::revision50::duel_actor_changed(r,victim);
            crate::zillennia_ground::on_arrival(r, victim);
            if let Some(after)=actor(r,victim){crate::animation_player::moved(r,&a,after,"P28","speed");}
        }
    }
}

/// Health resynchronization may resolve death without inventing damage or a source.
pub fn death_if_nonpositive(r: &mut Run, id: f64) -> Result<(), EngineError> {
    death(r, id, None, false, None)
}

fn share_bonded_packet(r: &mut Run, victim: f64, n: f64) -> Result<(), EngineError> {
    let Some(body) = actor(r, victim).cloned() else {
        return Ok(());
    };
    let owner = crate::half_here::owner_id(r, &body);
    let Some(pool) = actor(r, owner) else {
        return Ok(());
    };
    if !crate::abilities::endpoint_active(r,&body,"morae-bonded-by-unseen-threads")
    {
        return Ok(());
    }
    let target = condition(pool, "bondedTarget");
    let linked = r
        .actors_at(r.depth)
        .iter()
        .find(|a| {
            !a.dead.value_or(false) && a.id == target && crate::half_here::owner_id(r, a) != owner && crate::dreamwalking::same_plane(r,&body,a)
        })
        .map(|a| a.id);
    if let Some(target) = linked {
        let packet_before=crate::events::damage_count(target);
        let recipient=actor(r,target).cloned();
        damage_environment_core(
            r,
            target,
            n / 2.0,
            Some(victim),
            "",
            false,
            Some(&body),
            true,
            false,
        )?;
        if crate::events::damage_count(target)>packet_before {if let Some(a)=recipient {crate::events::cue(r,&a,"P68","stitch",crate::events::Stage::Health,&[(&a).into()]);crate::events::link(r,&body,&a,"P68","stitch-bond",crate::events::Stage::Health);}}
    }
    Ok(())
}

/// Shared incoming-damage boundary. Redirection follows mitigation, precedes HP
/// telemetry/rescues, and never admits health costs or terminal strikes. O(A+K)
/// lookup plus existing history/visibility, body synchronization and COW costs.
fn apply_hp_loss(r: &mut Run, victim: f64, n: f64, redirect: bool) -> f64 {
    let body = actor(r, victim).unwrap().clone();
    let id = crate::half_here::owner_id(r, &body);
    let pool = actor(r, id).unwrap().clone();
    let loss = if redirect && crate::hanged::protects(r, &body) {
        crate::hanged::split(pool.hp, pool.energy, n)
    } else { crate::hanged::Loss { hp: n, energy: 0. } };
    let actual = loss.hp.min(pool.hp.max(0.));
    // Packet-based on-hit/Bonded cues remain distinct from actual HP loss.
    crate::events::record_damage(r, victim, n.min(pool.hp.max(0.)));
    crate::combat_audit::record(r.tick, id, n, actual);
    resources::record_loss(r, &pool, "hp", actual);
    if loss.energy > 0. {
        resources::record_loss(r, &pool, "energy", loss.energy);
        actor_mut_trusted(r, id).unwrap().energy -= loss.energy;
        crate::animation_player::local(r, &body, "P13", "shield", crate::events::Stage::Health);
    }
    if id == r.player.id && (actual > 0. || loss.energy == 0.) {
        r.damage_since_last_turn_max_hp.get_or_insert_with(|| pool.max_hp);
        r.damage_since_last_turn = Field::Value(r.damage_since_last_turn.value_or(0.) + actual);
    }
    // Deposit only lost HP at the physical endpoint, not redirected energy.
    crate::dreamwalking::damage_notice(r, &body, n);
    if crate::dreamwalking::physical(r, &body) { crate::water_blood::deposit_damage(r, &body, actual); }
    if n > 0. { crate::creature_feeding::damage_interrupt(r, victim); }
    actor_mut_trusted(r, id).unwrap().hp -= loss.hp;
    if victim != id {
        let endpoint = actor_mut_trusted(r, victim).unwrap();
        endpoint.hp = pool.hp - loss.hp;
        endpoint.energy = pool.energy - loss.energy;
    }
    crate::revision50_combat::masochism_after_hp_loss(r, id, pool.hp);
    crate::half_here::sync(r);
    // Overflowing counts the resolved packet, including overkill, as before.
    crate::divine_rescues::accrue_damage(r, victim, n);
    actual
}

/// Combat and maintenance budget payment, without enemy casting eligibility.
pub fn spend_shared(r: &mut Run, id: f64, cost: f64) -> bool {
    let owner = crate::half_here::owner_id(r, actor(r, id).unwrap());
    let a = actor(r, owner).unwrap().clone();
    if a.energy < cost {
        return false;
    }
    resources::record_loss(r, &a, "energy", cost.min(a.energy.max(0.0)));
    actor_mut_trusted(r, owner).unwrap().energy -= cost;
    if id != owner {
        actor_mut_trusted(r, id).unwrap().energy = a.energy - cost;
    }
    true
}

/// Physical lifecycle death; callers have already applied the source health loss.
pub fn resolve_death(
    r: &mut Run,
    id: f64,
    source: Option<f64>,
    cause: Option<&str>,
    bypass: bool,
) -> Result<(), EngineError> {
    let label = match cause {
        Some("flesh-collapse") => Some(crate::finale::FALL_MESSAGE),
        Some("epidite") => Some("Epidite cleansing light"),
        Some("borge-shard") => Some("Shard of the Borge"),
        Some("phase-crushing") => Some("Crushed inside a wall after Phase Through ended"),
        Some("mech-destruction") => Some("Mech destruction"),
        Some("fall") => Some("A fall"),
        Some("equipment-loss") => Some("Health lost when equipment was removed"),
        None => None,
        Some("fire") => Some("Fire"),
        Some("caustic") => Some("Caustic gas"),
        Some("ichor") => Some("Caustic ichor"),
        Some("explosion") => Some("An explosion"),
        Some("poison") => Some("Poison"),
        Some("venom") => Some("Venom"),
        Some("hemophilia") => Some("Blood loss"),
        Some("banshee") => Some("Banshee shriek"),
        Some("razorleaves") => Some("Razorleaves"),
        Some(
            label @ ("Fire"
            | "Caustic gas"
            | "Caustic ichor"
            | "An explosion"
            | "Poison"
            | "Venom"
            | "Blood loss"
            | "Banshee shriek"
            | "Razorleaves"
            | "A fall"
            | "Breather gas"
            | "Gasser gas"
            | "Bleeding"
            | "Blood Weapon health cost"
            | "Blood Letting health cost"
            | "Shard of the Borge"),
        ) => Some(label),
        Some(other) => return unsupported(format!("direct death cause {other}")),
    };
    let owner = crate::half_here::owner_id(
        r,
        actor(r, id).ok_or_else(|| EngineError("death actor missing".into()))?,
    );
    let prior = r
        .run_stats
        .as_ref()
        .and_then(|s| s.death.as_ref())
        .is_some();
    let source_name = source.and_then(|id| actor(r, id)).map(|a| crate::actor_presentation::source_name(r, a).to_string());
    death_core(r, id, source, false, label, bypass)?;
    if !prior && owner == r.player.id && r.status == "dead" {
        if let Some(d) = r.run_stats.as_mut().and_then(|s| s.death.as_mut()) {
            d.reason = match (source_name, label) {
                (Some(name), Some(label)) => format!("{label} from {name}"),
                (Some(name), None) => format!("Killed by {name}"),
                (None, Some(label)) => label.into(),
                _ => if bypass {
                    "Unpreventable death"
                } else {
                    "Cause not recorded"
                }
                .into(),
            };
            d.cause = cause.map(|s| Field::Value(s.into())).unwrap_or_default();
        }
    }
    Ok(())
}
pub fn unpreventable_death(r: &mut Run, id: f64, kind: &str) -> Result<(), EngineError> {
    resolve_death(r, id, None, Some(kind), true)
}


#[derive(Default)]
struct CommandMoveTrace {
    edge:Option<(crate::timeweave::Cell,crate::timeweave::Cell)>,
    voluntary_move:bool,
    attacked:bool,
    probe_only:bool,
}
/// O(player ability count), revision/passive admission; no clocks or RNG.
fn timeweave_enabled(r:&Run)->bool {
    r.simulation_revision.value_or(39.)>=41. && r.player.abilities.iter().any(|s|s==crate::progression::WEAVE_ID)
}
/// O(F) existing floor lookup, O(1) geometry. Uses the actor's canonical plane,
/// not whichever floor is temporarily projected for a callback.
fn timeweave_player_cell(r:&Run)->Option<crate::timeweave::Cell>{
    use crate::timeweave::{Cell,Plane};
    let a=&r.player;let f=crate::dreamwalking::floor_for(r,a)?;let w=f.width();
    if w==0||!r.depth.is_finite()||r.depth.fract()!=0.||r.depth<1.||r.depth>u32::MAX as f64
        || !a.x.is_finite()||!a.y.is_finite()||a.x.fract()!=0.||a.y.fract()!=0.
        || a.x<0.||a.y<0.||a.x>=w as f64||a.y>=f.height() as f64{return None;}
    let tile=a.y as usize*w+a.x as usize;if tile>u32::MAX as usize{return None;}
    Some(Cell{floor:r.depth as u32,plane:if crate::dreamwalking::dream_actor(r,a){Plane::Dream}else{Plane::Waking},tile:tile as u32})
}
/// O(F+P) floor/portal lookups, no movement legality duplication. This only
/// classifies a shared movement kernel's resolved local edge; statuses/doors/
/// occupancy/ice still execute in that kernel. Forced/transitional edges pending.
fn ordinary_timeweave_edge(r:&Run,a:&Actor,x:f64,y:f64)->Option<(crate::timeweave::Cell,crate::timeweave::Cell)>{
    use crate::timeweave::{Cell,Plane};
    if a.id!=r.player.id||condition(a,"forcedMove")!=0. {return None;}
    let f=r.floor()?;let w=f.width();
    if w==0||!r.depth.is_finite()||r.depth.fract()!=0.||r.depth<1.||r.depth>u32::MAX as f64
        || !a.x.is_finite()||!a.y.is_finite()||!x.is_finite()||!y.is_finite()
        || a.x.fract()!=0.||a.y.fract()!=0.||x.fract()!=0.||y.fract()!=0.
        || !in_bounds(r,a.x,a.y)||!in_bounds(r,x,y){return None;}
    let(dx,dy)=((a.x-x).abs(),(a.y-y).abs());if dx>1.||dy>1.||(dx==0.&&dy==0.){return None;}
    let from=a.y as usize*w+a.x as usize;let to=y as usize*w+x as usize;
    if from>u32::MAX as usize||to>u32::MAX as usize||matches!(f.tiles.get(to)?.as_str(),"up"|"down"|"pit"|"chasm")
        || crate::portals::pair_at(f,from).is_some()||crate::portals::pair_at(f,to).is_some(){return None;}
    let plane=if crate::dreamwalking::context_dream(r){Plane::Dream}else{Plane::Waking};
    Some((Cell{floor:r.depth as u32,plane,tile:from as u32},Cell{floor:r.depth as u32,plane,tile:to as u32}))
}
/// Expected O(1) trail lookup; callers own boundary and action admission costs.
fn follows_timeweave(r:&Run,from:crate::timeweave::Cell,to:crate::timeweave::Cell)->bool {
    r.timeweave.as_ref().and_then(|t|t.get(to)).is_some_and(|s|s.from==to&&s.to==from)
}
/// O(F+ability count+8 expected lookups), no extra game RNG. Confusion can resolve
/// any adjacent reverse edge; an ordinary command only probes its intended edge.
fn timeweave_probe_possible(r:&Run,dx:f64,dy:f64)->bool {
    if !timeweave_enabled(r)||condition(&r.player,"forcedMove")!=0.||r.timeweave.as_ref().is_none_or(|t|t.is_empty())
        || !dx.is_finite()||!dy.is_finite()||dx.fract()!=0.||dy.fract()!=0.||dx.abs()>1.||dy.abs()>1.||(dx==0.&&dy==0.){return false;}
    let Some(from)=timeweave_player_cell(r) else{return false;};
    let f=crate::dreamwalking::floor_for(r,&r.player).unwrap();let w=f.width();
    let test=|dx:i32,dy:i32|{
        let x=r.player.x as i64+dx as i64;let y=r.player.y as i64+dy as i64;
        if x<0||y<0||x>=w as i64||y>=f.height() as i64{return false;}
        let to=crate::timeweave::Cell{tile:(y as usize*w+x as usize) as u32,..from};
        follows_timeweave(r,from,to)
    };
    if condition(&r.player,"confused")>0. {movement::DIRECTIONS.iter().any(|d|test(d.x,d.y))}
    else {test(dx as i32,dy as i32)}
}
/// Shared movement execution and exact action trace; no second movement algorithm.
/// Cost is ordinary movement, plus O(1) trace writes at the actual coordinate commit.
fn player_move_attempt(r:&mut Run,dx:f64,dy:f64,movement_only:bool,probe_only:bool)->Result<(bool,CommandMoveTrace),EngineError>{
    let id=r.player.id;let mut trace=CommandMoveTrace{probe_only,..Default::default()};
    let took=crate::dreamwalking::with_actor(r,id,|r|move_actor_inner(r,id,dx,dy,movement_only,Some(&mut trace)))?;
    Ok((took,trace))
}
/// Normal movement transitive cost plus expected O(1+k) entry-prefix removal,
/// or O(L) middle/source replacement; shared trail first-write detaches O(H+L).
fn move_player_command(r:&mut Run,dx:f64,dy:f64,movement_only:bool)->Result<(bool,bool,bool),EngineError>{
    if r.simulation_revision.value_or(29.)<43. && !timeweave_enabled(r){return move_actor(r,r.player.id,dx,dy,movement_only).map(|took|(took,false,false));}
    let (took,trace)=player_move_attempt(r,dx,dy,movement_only,false)?;
    let edge=trace.edge.filter(|(_,to)|!trace.attacked&&timeweave_player_cell(r)==Some(*to));
    if let Some((from,to))=edge.filter(|_|timeweave_enabled(r)) {
        let follows=follows_timeweave(r,from,to);
        if r.simulation_revision.value_or(29.)>=52. {
            consume_entered_timeweave(r,to,follows);
        }
        if !follows {
            let consumes_on_entry=r.simulation_revision.value_or(29.)>=52.;
            let trail=std::sync::Arc::make_mut(r.timeweave.get_or_insert_with(||std::sync::Arc::new(crate::timeweave::Trail::default())));
            // A pre-amendment save may already have a segment under the player.
            // Replace only that source, never reapply the old departure-prefix rule.
            if consumes_on_entry {trail.remove(from);}
            trail.append(crate::timeweave::Segment{cell:from,from,to});
        }
    }
    Ok((took,trace.voluntary_move,trace.attacked))
}
/// R52 arrival rule: a correct reverse entry consumes only that segment;
/// a wrong-angle entry consumes it and every older segment. Expected O(1+k)
/// for prefix/end consumption, O(L) for a middle entry, plus O(H+L) Arc detachment.
fn consume_entered_timeweave(r:&mut Run,to:crate::timeweave::Cell,follows:bool) {
    let Some(shared)=r.timeweave.as_mut() else{return;};
    if shared.get(to).is_none(){return;}
    let trail=std::sync::Arc::make_mut(shared);
    if follows {trail.remove(to);} else {trail.remove_through(to);}
}
/// Only potential reverse moves pay a candidate clone: O(R) owned Run metadata,
/// archive/replay/player/inventory bytes, plus COW writes and O(E+B) event clone.
/// Failed/held/slipped/attacking probes discard all candidate RNG/log/state/events;
/// charged dispatch then executes from the unchanged original. No time prelude.
fn try_free_timeweave_move(r:&mut Run,dx:f64,dy:f64)->Option<DispatchOutcome>{
    if !timeweave_probe_possible(r,dx,dy){return None;}
    let mut candidate=r.clone();let events=crate::events::Probe::begin();
    let Ok((accepted,trace))=player_move_attempt(&mut candidate,dx,dy,false,true) else{return None;};
    let Some((from,to))=trace.edge else{return None;};
    if !accepted||trace.attacked||timeweave_player_cell(&candidate)!=Some(to)||!follows_timeweave(r,from,to){return None;}
    if candidate.simulation_revision.value_or(29.)>=52. {
        consume_entered_timeweave(&mut candidate,to,true);
    }
    let Ok(outcome)=finish_action(&mut candidate,false) else{return None;};
    let w=candidate.floor().unwrap().width();
    crate::events::cue(&candidate,&candidate.player,"P23","speed",crate::events::Stage::Travel,&[
        crate::events::EventPoint{x:(from.tile as usize%w)as f64,y:(from.tile as usize/w)as f64},
        crate::events::EventPoint{x:(to.tile as usize%w)as f64,y:(to.tile as usize/w)as f64}]);
    *r=candidate;events.commit();Some(outcome)
}

#[cfg(test)]
mod timeweave_movement_tests {
    use super::*;
    /// O(fixture cells/actors) deterministic setup, ordinary admitted waking floor.
    fn room()->Run {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=41.0.into();r.player.x=10.;r.player.y=10.;
        r.player.abilities=vec![crate::progression::WEAVE_ID.into()];r.player.active.clear();r.player.conditions.clear();
        r.player.hp=10000.;r.player.max_hp=10000.;r.offers.clear();
        for a in r.actors.values_mut(){*a=std::sync::Arc::new(Vec::new());}
        for g in r.ground.values_mut(){*g=std::sync::Arc::new(Vec::new());}
        let f=r.floor_mut().unwrap();for tile in &mut f.tiles {*tile="floor".into();}
        f.environment=Field::Missing;f.doors=Field::Missing;f.vents=Field::Missing;
        f.currents=Field::Missing;f.mud=Field::Missing;f.slime=Field::Missing;f.roots=Field::Missing;
        r
    }
    /// O(1) fixture address, canonical depth1 and physical79-wide floor.
    fn cell(x:u32,y:u32)->crate::timeweave::Cell {crate::timeweave::Cell{floor:1,plane:crate::timeweave::Plane::Waking,tile:y*79+x}}
    /// O(normal command transitive work); assertions distinguish clock from position.
    #[test]
    fn forward_walk_lays_source_arrow_backward_walk_is_free_without_append(){
        let mut r=room();let before=r.tick;
        let first=dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap();assert!(first.took_turn);assert!(r.tick>before);
        let trail=r.timeweave.clone();let s=trail.as_ref().unwrap().get(cell(10,10)).unwrap();
        assert_eq!((s.from,s.to),(cell(10,10),cell(11,10)));
        let tick=r.tick;let rng=r.rng;let phase=r.phase.clone();let damage=r.damage_since_last_turn.clone();
        let history=r.resource_history.clone();let back=dispatch(&mut r,&Command::Move{dx:-1.,dy:0.}).unwrap();
        assert!(!back.took_turn);assert!(back.changed);assert!(back.pending_snapshot.is_none());
        assert_eq!((r.player.x,r.player.y),(10.,10.));assert_eq!(r.tick,tick);assert_eq!(r.rng,rng);
        assert_eq!(r.phase,phase);assert_eq!(r.damage_since_last_turn,damage);assert_eq!(r.resource_history,history);
        assert_eq!(r.timeweave,trail);
    }
    /// O(five normal commands); collision is SOURCE placement, never destination entry.
    #[test]
    fn wrong_side_entry_preserves_old_arrow_until_departure_source_collision(){
        let mut r=room();
        for (dx,dy) in [(1.,0.),(1.,0.),(0.,1.),(-1.,-1.)] {assert!(dispatch(&mut r,&Command::Move{dx,dy}).unwrap().took_turn);}
        let trail=r.timeweave.as_ref().unwrap();
        assert_eq!(trail.iter().map(|s|s.cell).collect::<Vec<_>>(),vec![cell(10,10),cell(11,10),cell(12,10),cell(12,11)]);
        assert_eq!((r.player.x,r.player.y),(11.,10.));
        assert!(dispatch(&mut r,&Command::Move{dx:0.,dy:1.}).unwrap().took_turn);
        let trail=r.timeweave.as_ref().unwrap();
        assert_eq!(trail.iter().map(|s|s.cell).collect::<Vec<_>>(),vec![cell(12,10),cell(12,11),cell(11,10)]);
        assert_eq!(trail.get(cell(11,10)).unwrap().to,cell(11,11));
    }
    /// O(command combat cost), hostile occupancy never becomes a free movement attempt.
    #[test]
    fn attacking_backwards_is_not_free_and_does_not_append(){
        let mut r=room();dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap();
        let mut a=crate::population::create_actor_revision("rat",r.next_id,10.,10.,false,41.).unwrap();r.next_id+=1.;a.hp=10000.;a.max_hp=10000.;
        r.actors_at_mut(1.).push(a);let trail=r.timeweave.clone();let tick=r.tick;
        assert!(dispatch(&mut r,&Command::Move{dx:-1.,dy:0.}).unwrap().took_turn);
        assert!(r.tick>tick);assert_eq!(r.timeweave,trail);
    }
    /// O(haste movement/finish), revision41 does not construct a pending Revert snapshot.
    #[test]
    fn hasted_forward_and_free_backtrack_preserve_phase_without_pending_snapshot(){
        let mut r=room();r.player.conditions.insert("hasted".into(),5.);
        let first=dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap();
        assert!(!first.took_turn);assert!(first.pending_snapshot.is_none());assert_eq!(r.phase.as_ref().map(String::as_str),Some("normal"));
        let tick=r.tick;let phase=r.phase.clone();let hasted=condition(&r.player,"hasted");
        let back=dispatch(&mut r,&Command::Move{dx:-1.,dy:0.}).unwrap();assert!(!back.took_turn);assert!(back.pending_snapshot.is_none());
        assert_eq!(r.tick,tick);assert_eq!(r.phase,phase);assert_eq!(condition(&r.player,"hasted"),hasted);
    }
    /// O(preflight + direct movement), unsupported special cases are named, not claimed covered.
    #[test]
    fn forced_and_transition_policy_remains_outside_local_scope(){
        let mut r=room();
        assert!(ordinary_timeweave_edge(&r,&r.player,11.,10.).is_some());
        r.floor_mut().unwrap().tiles[10*79+11]="down".into();assert!(ordinary_timeweave_edge(&r,&r.player,11.,10.).is_none());
        r.floor_mut().unwrap().tiles[10*79+11]="floor".into();r.player.conditions.insert("forcedMove".into(),1.);
        assert!(ordinary_timeweave_edge(&r,&r.player,11.,10.).is_none());
        assert!(move_actor(&mut r,0.,1.,0.,false).unwrap());assert!(r.timeweave.is_missing());
    }
    /// O(fixture preparation), creates a valid reverse edge without consuming a turn.
    fn reverse_room()->Run {
        let mut r=room();r.player.x=11.;r.phase="normal".to_string().into();
        let mut trail=crate::timeweave::Trail::default();trail.append(crate::timeweave::Segment{cell:cell(10,10),from:cell(10,10),to:cell(11,10)});
        r.timeweave=std::sync::Arc::new(trail).into();r
    }
    /// O(64) bounded fixture RNG selection; never used by gameplay/preflight.
    fn ice_seed(slips:bool)->f64 {
        for i in 1..=64u32 {let seed=i.wrapping_mul(0x9e3779b9);let mut value=seed;let n=crate::rng::next_u32(&mut value) as f64/4294967296.;if (n<1./3.)==slips{return seed as f64;}}
        panic!("fixture seed not found")
    }
    /// O(two movements + state bytes), failed slip probe leaves every canonical byte/RNG untouched.
    #[test]fn ice_success_free_and_failed_probe_does_not_consume_rng_or_log(){
        let mut r=reverse_room();let n=r.floor().unwrap().tiles.len();let mut ice=vec![false;n];ice[10*79+11]=true;r.floor_mut().unwrap().ice=ice.into();
        r.rng=ice_seed(true);let raw=serde_json::to_vec(&r).unwrap();crate::events::begin();
        assert!(try_free_timeweave_move(&mut r,-1.,0.).is_none());assert_eq!(serde_json::to_vec(&r).unwrap(),raw);assert!(crate::events::finish().is_empty());
        let tick=r.tick;let out=dispatch(&mut r,&Command::Move{dx:-1.,dy:0.}).unwrap();assert!(out.took_turn);assert!(r.tick>tick);assert_eq!((r.player.x,r.player.y),(11.,10.));
        assert_eq!(r.log.iter().filter(|s|s.as_str()=="You slip on the ice.").count(),1);
        let mut r=reverse_room();let n=r.floor().unwrap().tiles.len();let mut ice=vec![false;n];ice[10*79+11]=true;r.floor_mut().unwrap().ice=ice.into();r.rng=ice_seed(false);
        let tick=r.tick;let prior_rng=r.rng;let out=dispatch(&mut r,&Command::Move{dx:-1.,dy:0.}).unwrap();assert!(!out.took_turn);assert_eq!(r.tick,tick);assert_ne!(r.rng,prior_rng);assert_eq!((r.player.x,r.player.y),(10.,10.));
    }
    /// O(movement + cloned bytes), sticky hold is charged; a successful escape preserves its side effect.
    #[test]fn sticky_hold_probe_discards_then_successful_escape_is_free(){
        let mut r=reverse_room();let n=r.floor().unwrap().tiles.len();let mut mud=vec![0.;n];mud[10*79+11]=1.;r.floor_mut().unwrap().mud=mud.into();
        let raw=serde_json::to_vec(&r).unwrap();assert!(try_free_timeweave_move(&mut r,-1.,0.).is_none());assert_eq!(serde_json::to_vec(&r).unwrap(),raw);
        r.player.conditions.insert("stuck".into(),1.);let tick=r.tick;
        let out=dispatch(&mut r,&Command::Move{dx:-1.,dy:0.}).unwrap();assert!(!out.took_turn);assert_eq!(r.tick,tick);assert_eq!(condition(&r.player,"stuck"),0.);assert_eq!((r.player.x,r.player.y),(10.,10.));
    }
    /// O(movement/finish), door opening and arrival remain real during a free step.
    #[test]fn backward_door_opening_preserves_arrival_without_time(){
        let mut r=reverse_room();let mut doors=indexmap::IndexMap::new();doors.insert((10*79+10).to_string(),crate::model::FloorDoorsEntry{open:false,..Default::default()});r.floor_mut().unwrap().doors=doors.into();let tick=r.tick;
        let out=dispatch(&mut r,&Command::Move{dx:-1.,dy:0.}).unwrap();assert!(!out.took_turn);assert_eq!(r.tick,tick);
        assert!(r.floor().unwrap().doors.as_ref().unwrap()[&(10*79+10).to_string()].open);assert_eq!((r.player.x,r.player.y),(10.,10.));
    }
    /// O(64 bounded shared-kernel probes + state bytes), resolved edge rather than requested key controls freedom.
    #[test]fn confusion_resolved_reverse_is_free_and_missed_probe_is_exactly_discarded(){
        let mut original=reverse_room();original.player.conditions.insert("confused".into(),3.);
        let mut success=None;let mut failure=None;
        for i in 1..=64u32 {
            let seed=i.wrapping_mul(0x9e3779b9) as f64;let mut probe=original.clone();probe.rng=seed;
            let _events=crate::events::Probe::begin();let (_,trace)=player_move_attempt(&mut probe,0.,1.,false,true).unwrap();
            if trace.edge.is_some_and(|(a,b)|follows_timeweave(&original,a,b)){success.get_or_insert((seed,probe.rng));}else{failure.get_or_insert(seed);}
            if success.is_some()&&failure.is_some(){break;}
        }
        let (seed,expected_rng)=success.expect("bounded success fixture");let mut r=original.clone();r.rng=seed;let tick=r.tick;
        let out=dispatch(&mut r,&Command::Move{dx:0.,dy:1.}).unwrap();assert!(!out.took_turn);assert_eq!(r.tick,tick);assert_eq!(r.rng,expected_rng);assert_eq!((r.player.x,r.player.y),(10.,10.));
        let mut r=original;r.rng=failure.expect("bounded failed fixture");let before=serde_json::to_vec(&r).unwrap();
        assert!(try_free_timeweave_move(&mut r,0.,1.).is_none());assert_eq!(serde_json::to_vec(&r).unwrap(),before);
        let mut control=r.clone();control.timeweave=Field::Missing;
        let actual=dispatch(&mut r,&Command::Move{dx:0.,dy:1.}).unwrap();
        let expected=dispatch(&mut control,&Command::Move{dx:0.,dy:1.}).unwrap();
        assert_eq!(actual.took_turn,expected.took_turn);assert!(actual.took_turn);
        assert_eq!(actual.events,expected.events);assert_eq!(r.rng,control.rng);
        // Reference has the same ability/rules but no relevant trail, hence no probe.
        // Only trail state differs; compare all other canonical fields exactly.
        r.timeweave=Field::Missing;control.timeweave=Field::Missing;
        assert_eq!(serde_json::to_vec(&r).unwrap(),serde_json::to_vec(&control).unwrap());
    }
    /// O(dream setup + two commands), plane identity remains distinct and body is not moved.
    #[test]fn dream_walking_records_dream_edges_and_backtracks_without_time(){
        let mut r=room();r.dreamwalking=crate::dreamwalking::State{unlocked:true,..Default::default()}.into();
        assert_eq!(crate::dreamwalking::ability(&mut r,crate::dreamwalking::DREAMWALK_ID).unwrap(),Some(true));
        let body=crate::dreamwalking::body(&r).unwrap().clone();
        assert!(dispatch(&mut r,&Command::Move{dx:1.,dy:0.}).unwrap().took_turn);
        let from=crate::timeweave::Cell{plane:crate::timeweave::Plane::Dream,..cell(10,10)};
        assert!(r.timeweave.as_ref().unwrap().get(from).is_some());assert!(r.timeweave.as_ref().unwrap().get(cell(10,10)).is_none());
        let tick=r.tick;let out=dispatch(&mut r,&Command::Move{dx:-1.,dy:0.}).unwrap();assert!(!out.took_turn);assert_eq!(r.tick,tick);
        assert!(!r.dreamwalking.as_ref().unwrap().context_dream);let after=crate::dreamwalking::body(&r).unwrap();assert_eq!((after.x,after.y),(body.x,body.y));
        assert_eq!((r.player.x,r.player.y),(10.,10.));assert!(crate::timeweave::validate_run(&r).is_ok());
    }

}

#[cfg(test)]
#[path = "revision52_weave_tests.rs"]
mod revision52_weave_tests;

#[cfg(test)]
mod revision52_speed_receipt_tests {
    use super::*;
    #[test]
    fn forced_motion_after_adjacent_attack_does_not_grant_fast_attack() {
        // Exercise the action kernel directly: ordinary NPC save admission still
        // rejects borrowed divine powers and must not be widened for this test.
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();r.player.x=10.;r.player.y=10.;r.player.hp=1000.;r.player.max_hp=1000.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.stats.dodge=-1000.;r.inventory.clear();r.actors=Default::default();
        let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.environment=Field::Missing;f.doors=Field::Missing;f.currents=Field::Missing;f.mud=Field::Missing;f.roots=Field::Missing;f.slime=Field::Missing;f.ichor=Field::Missing;
        let mut a=crate::population::create_actor_revision("cave-bat",800.,9.,10.,false,52.).unwrap();a.state="hunting".into();a.stats.accuracy=1000.;a.stats.perception=1000.;a.abilities.push(crate::revision50_combat::FORCE.into());a.active.push(crate::revision50_combat::FORCE.into());a.conditions.insert("targetId".into(),r.player.id);a.last_seen=ActorLastSeen{x:10.,y:10.}.into();r.actors_at_mut(r.depth).push(a);
        complete_actor_action(&mut r,800.,false,&[],&mut PathCache::new()).unwrap();
        assert_eq!(actor(&r,800.).unwrap().x,10.);assert_eq!(r.player.x,11.);
    }
}

#[cfg(test)] mod float_away_direct_only_tests {
 use super::*;
 fn room()->Run {
  let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
  r.simulation_revision=52.0.into();r.player.x=4.;r.player.y=4.;r.player.hp=100.;r.player.max_hp=100.;r.player.conditions.clear();r.player.abilities=vec!["etill-float-away".into()];r.player.active=r.player.abilities.clone();
  r.actors_at_mut(r.depth).clear();let mut a=r.player.clone();a.id=9999.;a.x=3.;a.state="hunting".into();a.abilities.clear();a.active.clear();r.actors_at_mut(r.depth).push(a);
  let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.portals=Field::Missing;f.non_euclidean=Field::Missing;r
 }
 #[test] fn gas_and_status_damage_do_not_fly_or_move_but_direct_spells_still_do(){
  for kind in ["caustic","breatherGas","fire","poison","venom","bleeding","killspawnBleed","hemophilia","roots"] {
   let mut r=room();let id=r.player.id;damage_environment(&mut r,id,5.,Some(9999.),kind,true).unwrap();
   assert!(r.player.hp<100.,"{kind} still damages");assert_eq!((r.player.x,r.player.y),(4.,4.),"{kind}");assert_eq!(condition(&r.player,"flying"),0.,"{kind}");
  }
  for kind in ["attack","fire","lightning","cold","acid"] {
   let mut r=room();let id=r.player.id;damage_direct(&mut r,id,5.,9999.,kind,true).unwrap();
   assert_eq!((r.player.x,r.player.y),(5.,4.),"{kind}");assert_eq!(condition(&r.player,"flying"),2.,"{kind}");
  }
 }
 #[test] fn immediate_banshee_shriek_keeps_proc(){let mut r=room();let id=r.player.id;damage_environment(&mut r,id,5.,Some(9999.),"banshee",true).unwrap();assert_eq!(r.player.x,5.);}
 #[test] fn legacy_environment_proc_is_preserved(){let mut r=room();r.simulation_revision=51.0.into();let id=r.player.id;damage_environment(&mut r,id,5.,Some(9999.),"poison",true).unwrap();assert_eq!(r.player.x,5.);}
}

#[cfg(test)] mod surprise_response_tests {
 use super::*;
 fn room(state:&str)->Run{
  let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();r.player.x=4.;r.player.y=4.;r.player.hp=1000.;r.player.max_hp=1000.;r.player.conditions.clear();r.player.abilities.clear();r.player.active.clear();r.player.stats.accuracy=10000.;
  let mut b=r.player.clone();b.id=991.;b.x=5.;b.state=state.into();b.hp=10000.;b.max_hp=10000.;b.stats.accuracy=10000.;b.conditions.insert("hasted".into(),3.);r.actors_at_mut(r.depth).clear();r.actors_at_mut(r.depth).push(b);let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);r
 }
 #[test]fn successful_hit_delays_only_surprised_target_normal_and_bonus_actions(){
  for state in ["sleeping","wandering"] {
   let mut r=room(state);let player=r.player.id;assert!(attack_base(&mut r,player,991.,1.,Some(1.),false,0.,1.,false).unwrap());
   assert!(surprise_response_blocked(&r,actor(&r,991.).unwrap()));let hp=r.player.hp;r.tick=1.;
   let mut cache=PathCache::new();complete_actor_action(&mut r,991.,false,&[],&mut cache).unwrap();complete_actor_action(&mut r,991.,true,&[],&mut cache).unwrap();assert_eq!(r.player.hp,hp);
   assert!(!attack_base(&mut r,991.,player,1.,Some(5.),false,0.,1.,false).unwrap());
   damage_direct(&mut r,player,5.,991.,"lightning",true).unwrap();assert_eq!(r.player.hp,hp);
   r.tick=2.;assert!(attack_base(&mut r,991.,player,1.,Some(5.),false,0.,1.,false).unwrap());assert!(r.player.hp<hp);
  }
 }
 #[test]fn direct_spell_marks_but_status_and_aware_hits_do_not(){
  for (state,direct,expected) in [("sleeping",true,true),("wandering",true,true),("hunting",true,false),("sleeping",false,false)] {
   let mut r=room(state);let player=r.player.id;if direct{damage_direct(&mut r,991.,1.,player,"lightning",true).unwrap();}else{damage_environment(&mut r,991.,1.,Some(player),"poison",true).unwrap();}assert_eq!(surprise_response_blocked(&r,actor(&r,991.).unwrap()),expected);
  }
 }
 #[test]fn ranged_miss_does_not_mark_and_legacy_stays_unchanged(){
  let mut r=room("wandering");r.player.conditions.insert("unlucky".into(),1.);let player=r.player.id;assert!(!attack_base(&mut r,player,991.,1.,Some(1.),false,4.,1.,true).unwrap());assert!(!surprise_response_blocked(&r,actor(&r,991.).unwrap()));
  let mut r=room("sleeping");r.simulation_revision=51.0.into();let player=r.player.id;attack_base(&mut r,player,991.,1.,Some(1.),false,0.,1.,false).unwrap();assert!(!actor(&r,991.).unwrap().conditions.contains_key("monsterSurprisedThrough"));
 }
}
