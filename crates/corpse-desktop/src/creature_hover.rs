//! Passive creature-card copy. Only use the already-observed apparent actor.
//! The full clicked Inspector and Bestiary keep their existing descriptions.
use super::{applies, behavior, caster, condition as c, has, meta, piloted, traits};
use corpse_core::{Actor, Run, balance52, combat, monster_statuses};
use std::collections::HashSet;

pub const ADMIN_FLAVOR: &str =
    "The creature responsible for this chaos. Kill it and bring an end to this madness!";

pub(crate) fn percent(amount: f64, hp: f64) -> String {
    let number = amount.max(0.) * 100. / hp.max(1.);
    if number != 0. && number < 1. {
        "<1%".into()
    } else {
        format!("{:.0}%", number)
    }
}

fn packet(r: &Run, n: f64, kind: &str, exact: bool) -> f64 {
    combat::damage_packet(r, &r.player, n, kind, exact)
}

fn fireproof(r: &Run) -> bool {
    let p = &r.player;
    corpse_core::revision51_ichor_fire::immune(r.simulation_revision.value_or(29.), p)
        || has(p, "fire-immunity")
        || p.abilities.iter().any(|s| s == "anavani-star-power")
        || (corpse_core::revision50::enabled(r) && corpse_core::revision50_potions::fire_immune(p))
}

fn gas(r: &Run, kind: &str) -> String {
    let hp = r.player.max_hp;
    let base = if combat::nearest_damage_enabled(r) { hp / 15. } else { (hp / 15.).ceil() };
    let immune = r.player.abilities.iter().any(|s| matches!(s.as_str(),
        "grisha-entropy" | "enemy:pale:gas-immunity" | "enemy:beast:gas-immunity"));
    let damage = |v: f64| percent(if immune { 0. } else { packet(r, v, kind, true) }, hp);
    let active = c(&r.player, "gasVulnerable").max(0.);
    format!("{} max HP/turn", damage(base * (1. + active)))
}

fn ready(r: &Run, a: &Actor, key: &str) -> String {
    let t = c(a, key) - r.tick;
    if t > 0. { format!(" · ready in {t}t") } else { String::new() }
}

pub fn describe(r: &Run, a: &Actor, melee: [f64; 2],
    ranged: impl Fn(f64) -> (f64, f64)) -> Vec<String> {
    let ids = traits(a);
    let set: HashSet<_> = ids.iter().map(String::as_str).collect();
    let revision = r.simulation_revision.value_or(29.);
    let mhp = r.player.max_hp;
    let hp = r.player.hp;
    let pct = |n| percent(n, mhp);
    let duration = |n| monster_statuses::status_duration(r, &r.player, n);
    let offense = || combat::effective_stats(r, a).offense;
    let shot = |base| {
        let (lo, hi) = ranged(base);
        let (lo, hi) = (pct(lo), pct(hi));
        if lo == hi { format!("{hi} HP/hit") }
        else { format!("{}–{hi} HP/hit", lo.trim_end_matches('%')) }
    };
    let raw = combat::raw_attack_damage_enabled(revision);
    let recovering = c(a, "rangedRecovery") > 0.;
    let recovery = if recovering { "" } else { " · 1 action recovery" };
    let init_spell = |name: &str| if recovering {
        format!("{name} (recovering this action)")
    } else { name.into() };
    let greater = a.name.starts_with("Greater ");
    let new_followups = corpse_core::revision52_enemy_followups::enabled(r);
    let is = |s| corpse_core::revision52_enemy_followups::species(a, s);
    let mut rows = Vec::new();

    if corpse_core::finale::is_boss(a) {
        rows.push("Killing her starts the collapse.".into());
    }
    if corpse_core::ahvamitt::is_actor(a) {
        let found = c(a, "ahvamittFinds");
        rows.push(format!("Strike him three times for the Mask of Trickery ({found}/3). Teleports after the first two strikes."));
    }
    if is("skin-walker") {
        rows.push("Mimics weaker creatures; revealed within 2 tiles in sight.".into());
    }
    // Keep the ability sequence, but let high-stakes attacks lead the card.
    let priority = |id: &str| match id {
        "enemy:beast:banshee" | "enemy:beast:moon-laser" | "enemy:beast:detonate"
        | "enemy:pale:spit" | "enemy:beast:ruin-pulse" | "enemy:beast:fire-breath"
        | "enemy:beast:horn-bolt" | "enemy:beast:water-jet" => 0,
        _ => 1,
    };
    let mut sorted: Vec<_> = ids.iter().map(String::as_str).collect();
    sorted.sort_by_key(|id| priority(id));
    for id in sorted {
        if !applies(a, id, &set) { continue; }
        let text = match id {
            // Shared traits are already in the condition row.
            "enemy:beast:flight" | "enemy:beast:fast" | "enemy:beast:slow" | "enemy:pale:slow"
            | "enemy:beast:slime-bound" | "enemy:beast:rooted"
            | "enemy:pale:gas-shot" | "enemy:pale:breath-boost" => continue,

            "enemy:beast:banshee" => {
                let when = corpse_core::lifecycle::banshee_turns_until(r, a)
                    .map(|t| format!("in {t}t · every 30t")).unwrap_or_else(|| "every 30t".into());
                let now = corpse_core::banshee_rules::current(r);
                let damage = if now { corpse_core::banshee_rules::damage(hp) }
                    else { balance52::local_amount(r, a, "monsterSpecialDamage", 100.) };
                let scope = if now { "anyone in sight" } else { "you, in sight" };
                format!("Shriek {when}: {} of your current HP through armor · hits {scope}; heard floorwide.",
                    percent(packet(r, damage, "banshee", true), hp))
            }
            "enemy:beast:moon-laser" => {
                let base = balance52::local_amount(r, a, "monsterSpecialDamage", if raw { 60. } else { 120. });
                let start = if c(a, "laserCharge") > 0. {
                    "Moonbeam charged: fires on its next clear action"
                } else { "Moonbeam: charges for 20 NRG, then fires along a clear line" };
                format!("{start} · 16 tiles · {}{recovery}.", shot(base))
            }
            "enemy:pale:spit" => {
                let base = (if raw { 5. + 0.5 * offense() } else { 10. + offense() }) * if greater { 2. } else { 1. };
                let cloud = if set.contains("enemy:pale:gas-shot") {
                    format!(" Adds Caustic Gas at impact (10 NRG; {}).", gas(r, "caustic"))
                } else { String::new() };
                format!("{}: clear shot, 12 tiles · {}{recovery}.{cloud}", init_spell("Spit"), shot(base))
            }
            "enemy:beast:horn-bolt" => {
                let base = if raw { 10. + 0.75 * offense() } else { 20. + 1.5 * offense() };
                format!("Hornlight: clear shot, 10 tiles · {} · 8 NRG{recovery}.", shot(base))
            }
            "enemy:beast:water-jet" => {
                let base = balance52::local_amount(r, a, "monsterSpecialDamage", if raw { 11. } else { 22. });
                format!("Water Jet: clear shot, 6 tiles · {} · pushes 1 tile on hit · 8 NRG{recovery}.", shot(base))
            }
            "enemy:beast:detonate" => {
                let resist = r.player.abilities.iter().any(|s|
                    matches!(s.as_str(), "enemy:beast:explosion-immunity" | "anavani-nova"));
                let effect = if resist { " (0% to you)" } else { "" };
                format!("Detonates within 5 tiles: 50% max HP{effect} · ignites the area · all NRG.")
            }
            "enemy:beast:ruin-pulse" => {
                let dmg = corpse_core::monster_ruin::ruin_damage_revision(a, revision);
                format!("Ruin Pulse: {} HP to anyone in sight within 3 tiles · 20 NRG · 3t cooldown.",
                    pct(packet(r, dmg, "attack", false)))
            }
            "enemy:beast:silence-aura" => "Silences new paid abilities within 3 tiles in sight.".into(),
            "enemy:beast:darkness-aura" => "Darkness aura fades over 10 tiles; walls block it.".into(),
            "enemy:beast:fire-breath" => {
                let direct = balance52::local_amount(r, a, "monsterSpecialDamage", 24.);
                let (hit, burn) = if fireproof(r) { (0., 0.) } else { (packet(r, direct, "fire", true), packet(r, 4., "fire", true)) };
                let (lo, hi) = (duration(6.), duration(10.));
                let amount = if lo == hi { pct(burn * hi) } else {
                    format!("{}–{}", pct(burn * lo).trim_end_matches('%'), pct(burn * hi))
                };
                let length = if lo == hi { format!("{hi}") } else { format!("{lo}–{hi}") };
                format!("Fire breath: 6-tile cone · {} HP + {amount} Burning over {length}t · ignites tiles · 4t cooldown{recovery}{}.",
                    pct(hit), ready(r, a, "monsterFireReadyAt"))
            }
            "enemy:beast:witch" => {
                let phase = (c(a, "monsterSpellPhase") as usize) % 3;
                let n = duration(20.);
                let poison_amount = if c(&r.player, "poisoned") > 0. { r.player.conditions.get(corpse_core::revision50_amulet_effects::POISON_DAMAGE).copied().unwrap_or(2.) } else { 2. };
                let poison_damage = packet(r, poison_amount, "poison", true);
                rows.push(format!("Curses: Slowed {}t → Hemophilia {n}t → Poison {} HP/{n}t (refreshes).",
                    duration(5.), pct(poison_damage * n)));
                rows.push(format!("Next: {} · clear shot, 7 tiles · 8 NRG · 3t cooldown{recovery}{}.",
                    ["Slowed", "Hemophilia", "Poison"][phase], ready(r, a, "monsterSpellReadyAt")));
                if revision >= 33. {
                    rows.push(format!("Summons a Night Crawler or Skin Walker (50/50) on adjacent land · 10 NRG · 10t cooldown{}{}.",
                        ready(r, a, "monsterSummonReadyAt"),
                        if a.heartless.value_or(false) { " · heartless offspring" } else { "" }));
                }
                continue;
            }
            "enemy:beast:bloodmancy" => {
                let next = if c(a, "monsterSpellPhase") % 2. == 0. { "Pull" } else { "Hemophilia" };
                format!("Pulls 1 tile ↔ Hemophilia {}t · next: {next} · clear shot, 7 tiles · 8 NRG · 3t cooldown{recovery}{}.",
                    duration(20.), ready(r, a, "monsterSpellReadyAt"))
            }
            "enemy:beast:fungal-gas" if revision >= 50. => {
                let (kind, _) = corpse_core::fungi::emission(r, a);
                let speed = if revision >= 52. { " (1 tile/3t)" } else { "" };
                format!("Emits {kind} gas. Roots seek corpses in sight within 10 tiles{speed}; linked corpses feed the cloud and stay dead.")
            }
            "enemy:beast:gas-immunity" => if set.contains("enemy:beast:fungal-gas") {
                "Immune to gas and gas blasts; fire burns it.".into()
            } else { "Immune to gas and gas blasts.".into() },
            "enemy:pale:gas-immunity" => if greater && set.contains("enemy:pale:breath")
                && corpse_core::environment::breather_gas(r, a) == corpse_core::death_gas_rules::GAS {
                "Immune to caustic gas and Death Gas.".into()
            } else { "Immune to caustic gas.".into() },
            "enemy:pale:breath" => {
                let kind = corpse_core::environment::breather_gas(r, a);
                let label = match kind {
                    corpse_core::death_gas_rules::GAS => "Death Gas",
                    "hallucinogenic" => "Hallucinogenic Gas",
                    _ => "Caustic Gas",
                };
                let mut effects = vec![];
                if kind != "hallucinogenic" { effects.push(gas(r, kind)); }
                if matches!(kind, "hallucinogenic" | corpse_core::death_gas_rules::GAS) {
                    let n = monster_statuses::willpower_duration(r, &r.player, if corpse_core::gas_rules::current(r) {5.} else {2.});
                    effects.push(format!("Hallucinating {n}t (refreshes)"));
                }
                let extra = if set.contains("enemy:pale:breath-boost") { " Exhales an extra cloud for 10 NRG." } else { "" };
                format!("Emits {label}: {}.{extra}", effects.join(" + "))
            }
            "enemy:pale:gas-vulnerability" => format!("Damaging hits double caustic gas damage taken for {}t.", duration(5.)),
            "enemy:pale:scream" => {
                let (aim, perception) = if greater { (10,40) } else { (5,20) };
                let who = if corpse_core::screamer_rules::range_required(r) { " while you’re in range" } else { "" };
                let wait = if corpse_core::screamer_schedule::blocked(r, a) {
                    format!(" · ready in {}t", (c(a, "lastScreamUntil") - r.tick).max(0.))
                } else if corpse_core::screamer_rules::current(r) { " · repeats after effects expire".into() } else { String::new() };
                format!("Scream{who}: 10 tiles · 15 NRG{wait}. Disorients foes (−{aim} Accuracy, {}t); allies gain +{aim} Damage and +{perception} Perception for 5t. Rallies allies across the floor.",
                    duration(5.))
            }
            id @ ("enemy:beast:venom-weak" | "enemy:beast:venom" | "enemy:beast:venom-strong") => {
                let strength = match id { "enemy:beast:venom-weak"=>1., "enemy:beast:venom"=>2., _=>4. };
                let n = duration(5.);
                format!("Hits add {} HP over {n}t from Envenomed (stacks{}).",
                    pct(packet(r, strength, "venom", false) * n), if revision >= 52. { " and refreshes" } else { "" })
            }
            "enemy:beast:razorleaves" => format!("Razorleaves: 5 tiles in sight · {} max HP at turn end.", pct(packet(r, mhp * 0.02, "razorleaves", true))),
            "enemy:beast:aquatic" if set.contains("enemy:beast:ichor-bound") => continue,
            "enemy:beast:aquatic" => "Water-bound.".into(),
            "enemy:beast:ichor-bound" => "Ichor-bound.".into(),
            "enemy:beast:dryad" => format!("Heals the ally at lowest HP% for {} of the Dryad’s max HP · clear shot, 6 tiles · 5 NRG · 2t cooldown{}.",
                percent(balance52::local_amount(r, a, "monsterHealAmount", 25.), a.max_hp), ready(r, a, "monsterHealReadyAt")),
            "enemy:beast:beckon" => "Beckons prey past 4 tiles onto a clear tile 2 away · 12 NRG · 3t shared cooldown.".into(),
            "enemy:beast:wall-seal" => if c(a, "raisedWallExpires") > r.tick {
                format!("Walls off prey within 5 tiles · 15 NRG · 3t shared cooldown · wall lasts {}t more.", c(a, "raisedWallExpires") - r.tick)
            } else { "Walls off prey within 5 tiles (3 free adjacent tiles) · wall lasts 5t · 15 NRG · 3t shared cooldown.".into() },
            "enemy:beast:blink" if caster(a) => continue, // The caster AI never admits the approach blink.
            "enemy:beast:blink" => "Blinks up to 5 tiles to get within 3 of prey · 10 NRG · 3t shared cooldown.".into(),
            "enemy:beast:slime-trail" => "Spreads slime around each step.".into(),
            "enemy:beast:slime-walker" if set.contains("enemy:beast:slime-bound") => continue,
            "enemy:beast:armor-piercing" => "Melee ignores armor.".into(),
            "enemy:beast:life-steal" | "enemy:beast:lifesteal" => "Heals for half the damage dealt.".into(),
            "enemy:beast:ichor-siphon" => "Hits heal 2% of its max HP while in ichor.".into(),
            "enemy:pale:bite-repeat" => "Landed melee can bite again for 8 NRG.".into(),
            "enemy:pale:lifesteal" => "Heals for half melee damage · 8 NRG.".into(),
            "enemy:pale:birth" | "enemy:pale:double-birth" => {
                if id.ends_with(":double-birth") { continue; }
                let twin = if set.contains("enemy:pale:double-birth") { " · twins preferred (30 NRG, 2 tiles)" } else { "" };
                let what = if greater { "a Pale Chomper, Gasser, Spitter, Birther or Screamer" } else { "a Pale Jackal" };
                format!("Birth: {what} on adjacent land · 10 NRG{twin}{}{}.",
                    if revision >= 32. { " · 10t shared cooldown" } else { "" },
                    ready(r,a,"monsterSummonReadyAt"))
            }
            "enemy:beast:saidribee-queen" => format!("Summons a Short Saidribee or Guardian (2:1) on adjacent land · 15 NRG · 10t cooldown{}.",
                ready(r, a, "monsterSummonReadyAt")),
            "enemy:beast:fire-immunity" => "Fire immune.".into(),
            "enemy:beast:everburn" => if has(a,"fire-immunity") { "Always Burning.".into() } else { "Any Burning lasts indefinitely.".into() },
            "enemy:beast:no-revival" => "Never revives.".into(),
            "enemy:beast:submerge" => "Hides in deep water.".into(),
            "enemy:beast:regeneration" => format!("Regenerates {} of its max HP per turn.",
                percent(a.max_hp * corpse_core::revision50_rings::natural_hp_rate(r,a) * corpse_core::lifecycle::regen_multiplier(a), a.max_hp)),
            "enemy:beast:vampire" => {
                let retreating = c(a,"monsterRetreating") > 0.;
                if retreating { "Retreating until 65% HP; flies over gaps.".into() }
                else { "Flees in bat form at 35% HP until 65%; flies over gaps.".into() }
            }
            "enemy:beast:wendigo" => {
                let growth = c(a,"monsterWendigoLevel").max(1.);
                if growth >= 5. { "Fully grown. Flees while injured.".into() }
                else { format!("Feeds once per corpse: grows ({growth}/5) and delays its revival. Flees while injured.") }
            }
            "enemy:beast:mech" => if piloted(a) { "Piloted; destroying the shell kills the pilot.".into() } else { "Dormant until piloted.".into() },
            "enemy:beast:inventor" => if a.conditions.contains_key("monsterLinkedMechId") {
                "Flees to pilot its linked dormant mech.".into()
            } else { "Flees when threatened.".into() },
            "enemy:beast:minotaur" => {
                let heading = match (c(a,"monsterChargeDx") as i32, c(a,"monsterChargeDy") as i32) {
                    (0,-1)=>"↑", (1,-1)=>"↗", (1,0)=>"→", (1,1)=>"↘", (0,1)=>"↓",
                    (-1,1)=>"↙", (-1,0)=>"←", (-1,-1)=>"↖", _=>"",
                };
                let n = c(a,"monsterChargeRemaining");
                format!("Straight charge; hits only by bumping. Runs into fire and chasms{}.",
                    if n > 0. && !heading.is_empty() { format!(" · {heading} {n} steps left") } else { String::new() })
            }
            "enemy:beast:sphinx" => {
                let until = c(a,"monsterSphinxHuntAt") - r.tick;
                if until > 0. { format!("Hunts its attacker in {until}t.") }
                else if a.conditions.contains_key("monsterSphinxHuntAt") { "Hunts its attacker until it loses sight.".into() }
                else { "Hunts its attacker 3t after provocation.".into() }
            }
            "enemy:beast:phoenix" => "After 80t dead: 1% chance to revive at full HP; doubles each failed roll.".into(),
            "enemy:beast:yeti" => format!("Slows adjacent player bodies for {}t at turn start.", duration(1.)),
            "enemy:beast:deep-water" => format!("Holds adjacent prey in deep water for {}t.", duration(4.)),
            "enemy:beast:grab" => format!("Holds adjacent prey for {}t.", duration(3.)),
            "enemy:beast:hold-only" => format!("Holds adjacent prey for {}t (refreshes).", duration(3.)),
            "enemy:beast:stalk" => "Circles behind prey; waits up to 3t at doorways. Hunts when it spots prey within 4 tiles.".into(),
            _ => {
                let Some(entry) = meta(id) else { continue; };
                super::compact_enemy(a, entry)
            }
        };
        rows.push(text);
    }
    if corpse_core::revision51_ichor_fire::immune(revision, a) && !set.contains("enemy:beast:fire-immunity") {
        rows.push("Fire immune.".into());
    }
    if recovering && !set.contains("enemy:pale:spit") {
        rows.insert(0, "Recovering: spends its next action.".into());
    }
    if corpse_core::zillennia_ground::clears_ground(r, a) {
        rows.push("Erases loose items and grass, snow, slime and blood on its tile.".into());
    }
    if caster(a) && !super::mech(a) && !["minotaur","sphinx","inventor"].iter().any(|s|has(a,s)) {
        rows.push("Retreats from threats between casts.".into());
    }
    if a.heartless.value_or(false) && (set.contains("enemy:pale:birth") || set.contains("enemy:beast:saidribee-queen")) {
        rows.push("Heartless offspring.".into());
    }
    if new_followups && is("dragon") {
        rows.push("Takes flight for a faster safe route · up to 6t · 8 NRG.".into());
    }
    if new_followups && is("blorb") {
        rows.push(format!("Damaging melee adds {} HP as caustic damage.", pct(packet(r, 1., "caustic",true))));
    }
    if new_followups && is("killspawn") {
        let bleed = |n: f64| {
            let hp_damage = n.min(hp);
            pct(combat::nearest_damage(corpse_core::revision50_rings::resist_damage(r, &r.player, "bleeding", hp_damage * 0.5)))
        };
        let (lo, hi) = (bleed(melee[0]), bleed(melee[1]));
        let addition = if melee[1] == 0. { "half the hit's damage".into() }
            else if lo == hi { format!("{hi} HP") } else { format!("{}–{hi} HP", lo.trim_end_matches('%')) };
        rows.push(format!("Damaging hits add {addition} as Bleeding over {}t (stacks).", duration(3.)));
    }
    if a.perfect.value_or(false) && a.order.as_ref().is_some_and(|s| s=="p") {
        rows.push("Rallied: special actions paused.".into());
    }
    rows.dedup();
    // Only acquired divine powers and the caster marker use the remaining
    // shared adapter; native monster copy above must never be repeated.
    rows.extend(behavior(r, a));
    rows
}
