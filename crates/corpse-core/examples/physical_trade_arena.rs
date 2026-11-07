//! Controlled native-dispatch encounter analysis; no gameplay mechanics are replaced.
//!
//! Input is newline-delimited JSON on stdin, or one request / array of requests in a
//! file named by argv[1]. Each output line is one result (including explicit errors).
//! Example: {"kind":"yeti","revision":31,"candidate":{"offense":25,"defense":10},
//! "profiles":[[70,40,8,7,10,340]],"trials":256,"scenario":"adjacent"}
//! Profile order: offense, defense, accuracy, dodge, DR, maxHP. Profiles cycle over
//! trials; explicit seeds define the trial count. Explicit Defense candidates
//! derive HP=100+6*Defense unless max_hp is supplied; otherwise authored HP stays.
//! Scenarios: adjacent (also yeti/biter/venom/fire-breath/moonspawn/awake-major),
//! water-shore, water-wet, water-approach, ichor-shore, approach, ursa-pair,
//! piloted-mech, adjacent-grass, approach-grass. Grass scenarios have ordinary
//! combustible grass, never permanent fire. Ursa pair uses kind=ursa-major and candidate for Major;
//! partner_candidate controls Minor. Pair links the native death cry and hides the
//! sleeping Major behind a wall, with an open approach route. HP is never reset.
//! Player keeps the baseline Shortsword, no abilities, and attacks adjacent living
//! targets by Move, otherwise Wait. Forced movement, reactions, regen, energy,
//! cooldowns, all enemy traits, corpses, and environment remain engine-controlled.
//! Cost at enemy death is measured at the END of its native dispatch command;
//! that command may also run status/regen ticks. Postdeath Wait settlement is
//! reported separately. No waiting for future revival after finite effects settle.
//! Pending effects plus a revival/summon or a horizon produce explicit censoring.
//! Added r32 audit scenarios: chase/water-chase/ichor-chase start four tiles apart
//! and follow a stable shortest legal path; they do not optimize tactics. Three-cast-
//! children prepares three real Birther casts at ticks0/10/20 without simulating
//! intervening turns, removes the parent, then fights all descendants. It measures
//! isolated payload only, not twenty turns of exposure. All damage metrics are
//! per-trial fractions before averaging. An optional nonauthoritative native
//! packet observer captures raw and actual-HP loss, including unlogged spell
//! damage; channel logs and net HP loss remain separate diagnostics.
//! ATK/turns-to-kill are explicitly conditional on a kill; a player death does not
//! magically complete a monster lifespan or prove its eventual damage is capped.
//!
//! Harness overhead: T trials * C capped dispatch commands, fixed V=81 arena cells.
//! Recording costs O(A^2+S+V+E) per command for A actors, S copied actor-state
//! fields, and E emitted log events (the actor-ID reconciliation is quadratic). Optional
//! traces use O(T*C*(A+status keys)); otherwise outcomes use O(T), scratch O(A+V).
//! Native dispatch retains its full validation/pathfinding/visibility/environment
//! cost (not claimed O(1)); actor growth from summons is not hidden or stripped.
#![recursion_limit="256"]
use corpse_core::{Actor, ActorLastSeen, Command, Field, Floor, Run, engine, population, startup};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeMap, io::{self, BufRead, Write}, sync::Arc};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Candidate {
    offense: Option<f64>, defense: Option<f64>, accuracy: Option<f64>, dodge: Option<f64>, dr: Option<f64>,
    gas_passive: Option<f64>, gas_boost: Option<f64>, gas_shot: Option<f64>,
    #[serde(alias="maxHp")] max_hp: Option<f64>,
}
fn default_revision() -> f64 { 31. }
fn default_trials() -> usize { 256 }
fn default_max_turns() -> usize { 512 }
fn default_settle_turns() -> usize { 256 }
fn adjacent() -> String { "adjacent".into() }
fn wait_strategy()->String{"wait".into()}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    kind: String,
    #[serde(default="default_revision")] revision: f64,
    #[serde(default)] candidate: Candidate,
    #[serde(default)] partner_candidate: Candidate,
    #[serde(default)] partner_kind: Option<String>,
    #[serde(default)] without_support: bool,
    #[serde(default)] whole_family: bool,
    #[serde(default)] summoned_candidates: BTreeMap<String,Candidate>,
    #[serde(default="wait_strategy")] settlement_strategy: String,
    profiles: Vec<[f64;6]>,
    #[serde(default)] seeds: Vec<u32>,
    #[serde(default="default_trials")] trials: usize,
    #[serde(default="adjacent")] scenario: String,
    #[serde(default="default_max_turns")] max_turns: usize,
    #[serde(default="default_settle_turns")] settle_turns: usize,
    #[serde(default)] include_trials: bool,
    #[serde(default)] include_trace: bool,
    #[serde(default)] seed_prefix: Option<String>,
}
fn live(a: &Actor) -> bool { !a.dead.value_or(false) && a.hp > 0. }
fn patch(a: &mut Actor, c: &Candidate) -> Result<(), String> {
    for (slot, v) in [(&mut a.stats.offense,c.offense),(&mut a.stats.defense,c.defense),
        (&mut a.stats.accuracy,c.accuracy),(&mut a.stats.dodge,c.dodge),(&mut a.stats.dr,c.dr)] {
        if let Some(v) = v { if !v.is_finite() { return Err("Nonfinite candidate stat".into()); } *slot=v; }
    }
    for (key,value) in [("monsterGasPassiveVolume",c.gas_passive),("monsterGasBoostVolume",c.gas_boost),("monsterGasShotVolume",c.gas_shot)] {
        if let Some(value)=value {if !value.is_finite()||value<0.{return Err("Gas volume candidate must be finite and nonnegative".into());}a.conditions.insert(key.into(),value);}
    }
    // Preserve authored HP (e.g. Queen430/Defense48) when no HP/Defense
    // candidate was requested. An Accuracy/Dodge probe must not retune its body.
    if c.defense.is_some() { a.max_hp=100.+6.*a.stats.defense; }
    if let Some(hp)=c.max_hp { a.max_hp=hp; }
    if !a.max_hp.is_finite() || a.max_hp<=0. {
        return Err("Candidate maxHP must be positive and finite".into());
    }
    a.hp=a.max_hp;
    Ok(())
}
/// Analysis-only newborn candidates, applied after the native creation command.
/// Newborns are absent from that phase's actor snapshot; preserve any incidental
/// HP loss as a fraction. O(A log K), A current actor records, K explicit candidates.
fn patch_newborns(r:&mut Run,q:&Request,first_new_id:f64)->Result<(),String> {
    if q.summoned_candidates.is_empty(){return Ok(());}
    for a in r.actors_at_mut(1.).iter_mut().filter(|a|a.id>=first_new_id&&(a.born_from_birther.value_or(false)||a.conditions.get("witchSummoned")==Some(&1.)||a.conditions.get("queenSummoned")==Some(&1.))&&live(a)) {
        let key=a.creature_kind.as_ref().cloned().unwrap_or_else(||format!("pale:{}",a.name.strip_prefix("Pale ").unwrap_or(&a.name)));
        if let Some(candidate)=q.summoned_candidates.get(&key) {
            let fraction=if a.max_hp>0.{a.hp/a.max_hp}else{1.};
            patch(a,candidate)?;a.hp=a.max_hp*fraction;
        }
    }
    Ok(())
}
fn make_actor(kind: &str, id: f64, x: f64, y: f64, revision: f64) -> Result<Actor,String> {
    if kind==corpse_core::finale::BOSS_KIND { return Ok(corpse_core::finale::administrator_revision(id,x,y,revision)); }
    if let Some(quality)=kind.strip_prefix("zillennia:") {
        let mut a=population::catalog().zillennia.iter().find(|a|a.zillennia_quality.as_ref().is_some_and(|q|q==quality))
            .ok_or_else(||format!("Unknown Zillennia quality {quality}"))?.clone();
        a.id=id;a.x=x;a.y=y;a.state="hunting".into();
        corpse_core::physical_balance::apply_new_actor_kind(&mut a,kind,revision);
        return Ok(a);
    }
    if population::catalog().actors.contains_key(kind) || (revision>=33. && corpse_core::new_creatures::template(kind).is_some()) {
        return population::create_actor_revision(kind,id,x,y,false,revision);
    }
    // Resolve any noncatalog fish through the native event constructor;
    // existing catalog Ichor templates use the ordinary factory above.
    for depth in 1..=40 {
        let kinds=corpse_core::ichor::eligible_fish_kinds(depth,revision);
        if let Some(i)=kinds.iter().position(|k| *k==kind) {
            return Ok(corpse_core::ichor::make_fish_revision(id,x,y,depth as f64,
                (i as f64+0.5)/kinds.len() as f64,revision));
        }
    }
    Err(format!("Unknown creature {kind}"))
}
fn scene(q: &Request, profile: &[f64;6], seed: u32) -> Result<Run,String> {
    if ![31.,32.,33.,41.,42.].contains(&q.revision) { return Err("Harness expects explicit revision31,32,33,41 or42".into()); }
    let mut r=startup::new_run_shell_revision("0-5-11:physical-native-trade", "Unblessed",false,q.revision)?;
    r.player.abilities.clear(); r.player.active.clear(); r.player.conditions.clear();
    r.player.ability_order=Field::Missing;
    r.player.x=2.; r.player.y=2.;
    r.player.stats.offense=profile[0]; r.player.stats.defense=profile[1];
    r.player.stats.accuracy=profile[2]; r.player.stats.dodge=profile[3]; r.player.stats.dr=profile[4];
    r.player.hp=profile[5]; r.player.max_hp=profile[5];
    // Preserve an explicit reference profile's HP even if it contains a recorded
    // non-defense HP contribution. Effective defense uses the engine's rounding.
    r.player.base_max_hp=(profile[5]-6.*corpse_core::combat::integer_stat(profile[1])).into();
    r.rng=seed as f64;
    let w=9; let h=9;
    let mut tiles=vec!["wall".to_string();w*h];
    for y in 1..h-1 { for x in 1..w-1 { tiles[y*w+x]="floor".into(); } }
    if matches!(q.scenario.as_str(),"adjacent-grass"|"approach-grass") {
        for tile in &mut tiles {if tile=="floor" {*tile="grass".into();}}
    }
    let approach=matches!(q.scenario.as_str(),"approach"|"water-approach"|"approach-grass"|"chase"|"water-chase"|"ichor-chase");
    let x=if approach {6.} else {3.};
    let mut a=make_actor(&q.kind,4.,x,2.,q.revision)?;
    patch(&mut a,&q.candidate)?;
    a.state="hunting".into(); a.last_seen=ActorLastSeen{x:2.,y:2.}.into();
    let mut actors=vec![];
    match q.scenario.as_str() {
        "adjacent"|"yeti"|"biter"|"venom"|"fire-breath"|"moonspawn"|"awake-major"|"approach"|"adjacent-grass"|"approach-grass"|"chase"|"three-cast-children" => {},
        "water-shore"|"water-wet"|"water-approach"|"water-chase" => {
            for y in 1..h-1 { for x in 3..w-1 {tiles[y*w+x]="water".into();} }
            if q.scenario=="water-wet" { tiles[2*w+2]="water".into(); }
        },
        "ichor-shore"|"ichor-chase" => {},
        "ursa-pair" => {
            if q.kind!="ursa-major" { return Err("ursa-pair requires kind=ursa-major; partner_candidate controls Minor".into()); }
            a.x=5.; a.y=2.; a.state="sleeping".into(); a.last_seen=Field::Missing;
            tiles[2*w+4]="wall".into(); tiles[w+4]="wall".into();
            let mut minor=make_actor("ursa-minor",5.,3.,2.,q.revision)?;
            patch(&mut minor,&q.partner_candidate)?;
            minor.state="hunting".into(); minor.last_seen=ActorLastSeen{x:2.,y:2.}.into();
            minor.conditions.insert("monsterUrsaPartnerId".into(),4.);
            actors.push(minor);
        },
        "support-with-fighter" => {
            let kind=q.partner_kind.as_deref().ok_or("support-with-fighter requires partner_kind")?;
            a.x=4.;a.y=2.;
            let mut fighter=make_actor(kind,5.,3.,2.,q.revision)?;
            patch(&mut fighter,&q.partner_candidate)?;
            fighter.state="hunting".into();fighter.last_seen=ActorLastSeen{x:2.,y:2.}.into();
            actors.push(fighter);
        },
        "piloted-mech" => {
            if q.kind!="hemogoblin-mech" { return Err("piloted-mech requires kind=hemogoblin-mech".into()); }
            let mut pilot=make_actor("hemogoblin-inventor",5.,a.x,a.y,q.revision)?;
            pilot.state="hunting".into();
            a.monster_state.get_or_insert_with(Default::default).pilot=Box::new(pilot).into();
            a.conditions.insert("monsterLinkedInventorId".into(),5.);
        },
        s=>return Err(format!("Unknown scenario {s}")),
    }
    if q.scenario=="three-cast-children" { a.x=4.; a.y=4.; }
    if q.scenario!="support-with-fighter" || !q.without_support {actors.push(a);}
    let mut floor=Floor{width:(w as f64).into(),height:(h as f64).into(),depth:1.,domain:"Dungeon".into(),
        tiles,lights:vec![100.;w*h],seen:vec![true;w*h],..Default::default()};
    if matches!(q.scenario.as_str(),"ichor-shore"|"ichor-chase") {
        let mut ichor=vec![0.;w*h];
        for y in 1..h-1 { for x in 3..w-1 {ichor[y*w+x]=100.;} }
        floor.ichor=ichor.into();
    }
    // Other habitat-bound actors require their authored substrate too.
    for a in &actors {
        if a.abilities.iter().any(|s|s=="enemy:beast:slime-bound") {
            let n=floor.tiles.len(); floor.slime.get_or_insert_with(||vec![0.;n])[a.y as usize*w+a.x as usize]=1.;
        }
    }
    corpse_core::environment::initialize(&mut floor)?;
    r.floors=vec![Arc::new(floor)];
    r.actors.clear(); r.actors.insert("1".into(),Arc::new(actors));
    r.ground.clear(); r.ground.insert("1".into(),Arc::new(vec![])); r.next_id=6.;
    for a in r.actors_at(1.) {
        if corpse_core::ichor::stranded(&r,a) { return Err(format!("{} is stranded in scenario {}; choose water-shore, water-wet, or ichor-shore",a.name,q.scenario)); }
    }
    engine::validate_supported(&r).map_err(|e|e.to_string())?;
    if q.scenario=="three-cast-children" {
        if !r.actors_at(1.)[0].abilities.iter().any(|a|a=="enemy:pale:birth" || (q.revision>=33. && a=="enemy:beast:witch")) {
            return Err("three-cast-children requires a Birther or revision33 Witch".into());
        }
        let player=r.player.id;
        // Setup only: exactly three native successful casts, no skipped world-turn
        // simulation claimed. Parent is excluded from the isolated payload value.
        for cast in 0..3 {
            r.tick=cast as f64*10.;
            let count=r.actors_at(1.).len();
            let first_new_id=r.next_id;
            if !corpse_core::ai::enemy_action(&mut r,4.,player).map_err(|e|e.to_string())?
                || r.actors_at(1.).len()<=count {
                return Err(format!("Native summon setup cast {} did not create children",cast+1));
            }
            patch_newborns(&mut r,q,first_new_id)?;
        }
        r.actors_at_mut(1.).retain(|a|a.id!=4.);
        for a in r.actors_at_mut(1.) {
            a.state="hunting".into(); a.last_seen=ActorLastSeen{x:2.,y:2.}.into();
        }
        r.log_archive=Field::Missing; r.log.clear(); r.log_kinds=Field::Missing;
    }
    Ok(r)
}
// Capture only state needed by instrumentation, not the growing replay/log archive.
struct Before {
    hp: f64,
    x: f64,
    y: f64,
    tick: f64,
    actors: Vec<Actor>,
}
impl Before {
    fn capture(r:&Run)->Self {Self{hp:r.player.hp,x:r.player.x,y:r.player.y,tick:r.tick,actors:r.actors_at(1.).to_vec()}}
}
#[derive(Debug,Default,Serialize)]
struct Observation {
    markers: BTreeMap<String,usize>,
    damage_packets: BTreeMap<String,f64>,
    postdeath_damage_packets: BTreeMap<String,f64>,
    attack_attempts: usize,
    player_attacks: usize,
    player_hits: usize,
    capped_actual_damage: f64,
    authoritative_damage: f64,
    largest_authoritative_packet: f64,
    authoritative_postdeath_damage: f64,
    enemy_energy_min: f64,
    enemy_energy_consumed: f64,
    trace: Vec<Value>,
}
fn inc(m:&mut BTreeMap<String,usize>,k:&str) {*m.entry(k.into()).or_default()+=1;}
fn packet(m:&mut BTreeMap<String,f64>,k:&str,n:f64) {*m.entry(k.into()).or_default()+=n;}
fn archive_len(r:&Run)->usize {r.log_archive.as_ref().map_or(0,|a|a.events.len())}
fn new_logs(r:&Run,start:usize)->Vec<String> {
    r.log_archive.as_ref().map_or_else(Vec::new,|a|a.events.iter().skip(start)
        .filter_map(|e|a.messages.get(*e as usize/4).cloned()).collect())
}
fn observe(r:&Run,old:&Before,logs:&[String],out:&engine::DispatchOutcome,o:&mut Observation,settling:bool,trace:bool) {
    let recorded=corpse_core::combat_audit::take();
    for packet in recorded.iter().filter(|p|p.victim==r.player.id) {
        o.authoritative_damage+=packet.amount;
        o.largest_authoritative_packet=o.largest_authoritative_packet.max(packet.amount);
        o.capped_actual_damage+=packet.actual;
        if settling{o.authoritative_postdeath_damage+=packet.amount;}
    }
    let mut attempts=0;
    for line in logs {
        // Both "You strike ... for ..." and "You strike ..., but miss.".
        if line.starts_with("You strike ") {o.player_attacks+=1;if !line.ends_with(", but miss."){o.player_hits+=1;}}
        if line.contains(" you for ") || line.contains(" you, but misses.") {attempts+=1;}
        let damage=if let Some((channel,rest))=line.split_once(" deals ") {
            rest.strip_suffix(" damage to you.").and_then(|n|n.parse().ok()).map(|n|(channel.to_string(),n))
        } else {
            line.split_once(" you for ").and_then(|(_,rest)|rest.strip_suffix(" damage.").and_then(|n|n.parse().ok()))
                .map(|n|("physical_attack".into(),n))
        };
        if let Some((channel,n))=damage {
            packet(&mut o.damage_packets,&channel,n);
            if settling {packet(&mut o.postdeath_damage_packets,&channel,n);}
        }
        for (needle,key) in [
            ("accumulating venom","venom_applied"),("Slowness delays","yeti_slow_extra_turn"),
            ("cone of fire","fire_breath"),("draws strength from the ichor","ichor_siphon"),
            ("terrible scream","ursa_death_cry"),("rises heartless","corpse_revival"),
            ("folds through moonlight","moon_blink"),("grabs its prey in the deep water","deep_water_grab"),
            ("holds you in place","grab"),("jet of water","water_jet"),
            ("climbs into the Hemogoblin Mech","mech_mount"),
        ] {if line.contains(needle){inc(&mut o.markers,key);}}
    }
    // Water jet has a presentation event but no textual combat report.
    for event in &out.events {
        if event.label.as_deref()==Some("Water jet") {inc(&mut o.markers,"water_jet");}
        if event.label.as_deref()==Some("Fire breath") {inc(&mut o.markers,"event_fire_breath");}
        if event.label.as_deref()==Some("Vanish") {inc(&mut o.markers,"event_blink_vanish");}
        if event.label.as_deref()==Some("Held") {inc(&mut o.markers,"event_held");}
    }
    o.attack_attempts+=attempts;
    if attempts>1 && old.actors.iter().any(|a|a.abilities.iter().any(|s|s=="enemy:pale:bite-repeat")) {
        inc(&mut o.markers,"biter_multiple_attacks_in_command");
    }
    for a in r.actors_at(1.) {
        o.enemy_energy_min=o.enemy_energy_min.min(a.energy);
        if let Some(b)=old.actors.iter().find(|b|b.id==a.id) {
            let spent=(b.energy-a.energy).max(0.); o.enemy_energy_consumed+=spent;
            if spent>0. {inc(&mut o.markers,"enemy_energy_consumed");}
            // A sole Biter has exactly two energy-spending branches: 8 per
            // repeated attack and 8 per lifesteal. Attribute the observed spend
            // after counting its actual attack reports; do not infer from HP.
            if old.actors.iter().filter(|b|live(b)).count()==1
                && r.tick-old.tick==1.
                && a.abilities.iter().any(|s|s=="enemy:pale:lifesteal") {
                let prefix=format!("{} bites you",a.name);
                let source_attacks=logs.iter().filter(|l|l.starts_with(&prefix)).count();
                let repeat_cost=source_attacks.saturating_sub(1) as f64*8.;
                let lifesteals=((spent-repeat_cost).max(0.)/8.).round() as usize;
                if lifesteals>0 {*o.markers.entry("biter_lifesteal_activations".into()).or_default()+=lifesteals;}
            }
        }
        if a.abilities.iter().any(|s|s=="enemy:beast:everburn") && engine::condition(a,"burning")>0. && live(a) {
            inc(&mut o.markers,"everburn_alive_and_burning");
        }
    }
    if engine::condition(&r.player,"burning")>0. {inc(&mut o.markers,"player_burning");}
    if engine::condition(&r.player,"grabbed")>0. {inc(&mut o.markers,"player_grabbed");}
    if r.floor().unwrap().environment.as_ref().is_some_and(|e|e.fire.iter().any(|f|*f>0.)) {inc(&mut o.markers,"active_fire_grid");}
    if (r.player.x,r.player.y)!=(old.x,old.y) {inc(&mut o.markers,"forced_player_displacement");}
    if trace {o.trace.push(json!({"tick":r.tick,"phase":if settling{"settlement"}else{"combat"},
        "player_hp":r.player.hp,"player_xy":[r.player.x,r.player.y],"player_conditions":r.player.conditions,
        "actors":r.actors_at(1.).iter().map(|a|json!({"id":a.id,"kind":a.creature_kind,"name":a.name,
            "hp":a.hp,"energy":a.energy,"xy":[a.x,a.y],"dead":a.dead,"state":a.state,"conditions":a.conditions})).collect::<Vec<_>>(),
        "logs":logs,"events":out.events}));}
}
fn pending(r:&Run)->Vec<String> {
    let mut v=vec![];
    for (k,n) in &r.player.conditions {
        if *n>0. && (k.starts_with("venom:") || ["burning","poisoned","bleeding","hemophilia"].contains(&k.as_str())) {v.push(format!("player:{k}={n}"));}
    }
    let f=r.floor().unwrap();
    if let Some(e)=f.environment.as_ref() {
        if e.fire_kind.as_ref().is_some_and(|v|v.iter().any(|k|corpse_core::environment::fire_active(k.as_deref()))) {v.push("active_fire_grid".into());}
        if e.gas_volume.as_ref().is_some_and(|v|v.iter().any(|n|*n>0.)) {v.push("active_gas_grid".into());}
    }
    if f.permanent_fire.as_ref().is_some_and(|v|v.iter().any(|n|*n>0.)) {v.push("permanent_fire_terrain".into());}
    v
}
/// Post-kill exposure policy, not a world-rule change. Stable BFS O((V+E)*H+A),
/// H is native terrain/ability hazard helper cost (fixed starter loadout here).
/// row-neighbor order inherited from movement::DIRECTIONS. Replans each command
/// because native gas/fire propagation can make the previously clear cell unsafe.
fn escape_command(r:&Run)->Result<Option<Command>,String> {
    use corpse_core::movement::{Point,DIRECTIONS};
    use std::collections::VecDeque;
    let f=r.floor().unwrap();let w=f.width();let n=f.tiles.len();
    let at=r.player.y as usize*w+r.player.x as usize;
    let attached=pending(r).iter().any(|s|s.starts_with("player:"));
    if !corpse_core::environment::creature_hazard_at(r,&r.player,at) {
        return Ok(if attached{Some(Command::Wait)}else{None});
    }
    let mut occupied=vec![false;n];for a in r.actors_at(1.).iter().filter(|a|live(a)){occupied[a.y as usize*w+a.x as usize]=true;}
    let mut seen=vec![false;n];let mut first=vec![None;n];let mut queue=VecDeque::from([at]);seen[at]=true;
    while let Some(i)=queue.pop_front() {
        let from=Point{x:(i%w)as i32,y:(i/w)as i32};
        for d in DIRECTIONS {
            let x=from.x+d.x;let y=from.y+d.y;
            if x<0||y<0||x as usize>=w||y as usize>=f.height(){continue;}
            let j=y as usize*w+x as usize;
            if seen[j]||!corpse_core::travel::can_traverse_from(r,&r.player,x as f64,y as f64,from)
                ||occupied[j]{continue;}
            seen[j]=true;first[j]=if i==at{Some(d)}else{first[i]};
            if !corpse_core::environment::creature_hazard_at(r,&r.player,j) {
                let step=first[j].unwrap();
                return Ok(Some(if engine::condition(&r.player,"paralyzed")>0.{Command::Wait}else{Command::Move{dx:step.x as f64,dy:step.y as f64}}));
            }
            queue.push_back(j);
        }
    }
    Err("trapped_no_reachable_clear_cell_after_primary_death".into())
}
fn cost(hp:f64,max:f64)->f64 {(1.-hp.max(0.)/max).max(0.)}
/// Stable shortest-path chase in the bounded arena. O(V*H+E+A) per command;
/// H includes native passability helpers (fixed catalog/starter loadout here).
/// real dispatch still validates movement, hazards, displacement and combat.
fn player_command(r:&Run,target:&Actor,chase:bool)->Command {
    if engine::condition(&r.player,"paralyzed")>0. {return Command::Wait;}
    let dx=target.x-r.player.x; let dy=target.y-r.player.y;
    if dx.abs().max(dy.abs())<=1. && (dx!=0.||dy!=0.) {return Command::Move{dx,dy};}
    if !chase {return Command::Wait;}
    let f=r.floor().unwrap();let w=f.width();
    let mut allowed:Vec<_>=(0..f.tiles.len()).map(|i|corpse_core::travel::passable(r,&r.player,(i%w)as f64,(i/w)as f64)&&corpse_core::travel::habitat_valid(f,&r.player,(i%w)as f64,(i/w)as f64)&&f.tiles[i]!="chasm").collect();
    for a in r.actors_at(r.depth) {if live(a)&&a.id!=target.id {allowed[a.y as usize*w+a.x as usize]=false;}}
    allowed[target.y as usize*w+target.x as usize]=true;
    let field=corpse_core::movement::distance_field(&f.tiles,w,corpse_core::movement::Point{x:target.x as i32,y:target.y as i32},false,false,Some(&allowed),None);
    corpse_core::movement::next_step(&field,&f.tiles,corpse_core::movement::Point{x:r.player.x as i32,y:r.player.y as i32})
        .map_or(Command::Wait,|d|Command::Move{dx:d.x as f64,dy:d.y as f64})
}
fn run_trial(q:&Request,index:usize,seed:u32)->Result<Value,String> {
    let profile=&q.profiles[index%q.profiles.len()];
    let mut r=scene(q,profile,seed)?;
    let start_tick=r.tick;
    let chase=matches!(q.scenario.as_str(),"chase"|"water-chase"|"ichor-chase"|"three-cast-children"|"support-with-fighter");
    let whole_family=q.whole_family||q.scenario=="three-cast-children";
    let targets:Vec<f64>=r.actors_at(1.).iter().map(|a|a.id).collect();
    let initial_abilities:Vec<_>=r.actors_at(1.).iter().map(|a|json!({"id":a.id,"name":a.name,"abilities":a.abilities,"stats":a.stats,"effective_stats":corpse_core::combat::effective_stats(&r,a),"conditions":a.conditions,"max_hp":a.max_hp})).collect();
    let mut o=Observation{enemy_energy_min:r.actors_at(1.).iter().map(|a|a.energy).fold(f64::INFINITY,f64::min),..Default::default()};
    let mut commands=0; let mut death_cost=None; let mut death_command_start_hp=None;
    let mut minor_cost=None; let mut reason=None; let mut combat_done=false;
    while commands<q.max_turns && live(&r.player) && r.status=="playing" {
        let target=if whole_family {
            r.actors_at(1.).iter().filter(|a|live(a)).min_by(|a,b|(a.x-r.player.x).hypot(a.y-r.player.y).total_cmp(&(b.x-r.player.x).hypot(b.y-r.player.y)).then(a.id.total_cmp(&b.id)))
        } else {targets.iter().find_map(|id|r.actors_at(1.).iter().find(|a|a.id==*id&&live(a)))};
        let Some(target)=target else {combat_done=true;break;};
        let command=player_command(&r,target,chase);
        let old=Before::capture(&r); let cursor=archive_len(&r);let first_new_id=r.next_id;
        corpse_core::combat_audit::begin();
        let out=engine::dispatch(&mut r,&command).map_err(|e|format!("trial {index}, command {commands}: {e}"))?;
        patch_newborns(&mut r,q,first_new_id)?;
        let logs=new_logs(&r,cursor); observe(&r,&old,&logs,&out,&mut o,false,q.include_trace); commands+=1;
        if !out.took_turn {reason=Some("command_did_not_take_turn".to_string());break;}
        if q.scenario=="ursa-pair" && minor_cost.is_none() && r.actors_at(1.).iter().any(|a|a.id==5.&&!live(a)) {
            minor_cost=Some(cost(r.player.hp,profile[5]));
        }
        if (whole_family && !r.actors_at(1.).iter().any(live))
            || (!whole_family && targets.iter().all(|id|r.actors_at(1.).iter().find(|a|a.id==*id).is_none_or(|a|!live(a)))) {
            combat_done=true; death_cost=Some(cost(r.player.hp,profile[5])); death_command_start_hp=Some(old.hp);break;
        }
    }
    let combat_commands=commands; let combat_ticks=r.tick-start_tick; let combat_cost=cost(r.player.hp,profile[5]);
    let gross_combat=o.authoritative_damage/profile[5];
    let hp_at_death=death_cost.map(|_|r.player.hp); let mut settlement_commands=0;
    if !combat_done && live(&r.player) && reason.is_none() {reason=Some(format!("combat_command_horizon:{}",q.max_turns));}
    let combat_censored=reason.is_some();let combat_dead=!live(&r.player);
    if combat_done && live(&r.player) {
        loop {
            let others:Vec<_>=r.actors_at(1.).iter().filter(|a|live(a)).map(|a|format!("{}#{}",a.name,a.id)).collect();
            if !others.is_empty() {reason=Some(format!("living_actor_after_target_death:{}",others.join(",")));break;}
            let settle_command=if q.settlement_strategy=="escape" {
                match escape_command(&r) {
                    Ok(Some(command))=>command,
                    Ok(None)=>{if !pending(&r).is_empty(){inc(&mut o.markers,"escaped_residual_field_exposure");}break;},
                    Err(message)=>{reason=Some(message);break;}
                }
            } else {if pending(&r).is_empty(){break;}Command::Wait};
            if settlement_commands>=q.settle_turns {reason=Some(format!("residual_effect_horizon:{}:{}",q.settle_turns,pending(&r).join(",")));break;}
            let old=Before::capture(&r); let cursor=archive_len(&r);let first_new_id=r.next_id;
            corpse_core::combat_audit::begin();
            let out=engine::dispatch(&mut r,&settle_command).map_err(|e|format!("trial {index}, settlement {settlement_commands}: {e}"))?;
            patch_newborns(&mut r,q,first_new_id)?;
        let logs=new_logs(&r,cursor); observe(&r,&old,&logs,&out,&mut o,true,q.include_trace);
            settlement_commands+=1; commands+=1;
            if !live(&r.player) || r.status!="playing" {break;}
        }
    }
    let total_cost=cost(r.player.hp,profile[5]);
    let dead=!live(&r.player);
    Ok(json!({"trial":index,"profile_index":index%q.profiles.len(),"seed":seed,"profile":profile,
        "initial_actors":initial_abilities,"killed":combat_done,"died":dead,"died_during_settlement":dead&&combat_done&&combat_cost<1.,
        "censored":reason.is_some(),"combat_censored":combat_censored,"aftermath_censored":reason.is_some()&&!combat_censored,"died_during_combat":combat_dead,"censor_reason":reason,"combat_cost":combat_cost,"cost_at_enemy_death":death_cost,
        "total_cost":total_cost,"residual_net_cost":total_cost-combat_cost,"hp_at_enemy_death":hp_at_death,
        "hp_before_kill_command":death_command_start_hp,"terminal_hp":r.player.hp,"minor_death_cost":minor_cost,
        "combat_commands":combat_commands,"combat_world_turns":combat_ticks,"settlement_commands":settlement_commands,
        "commands":commands,"world_turns":r.tick-start_tick,"pending_effects":pending(&r),"terminal_player_conditions":r.player.conditions,
        "markers":o.markers,"damage_packets":o.damage_packets,"postdeath_damage_packets":o.postdeath_damage_packets,
        "enemy_attack_attempts":o.attack_attempts,"player_attack_attempts":o.player_attacks,"player_landed_attacks":o.player_hits,
        "gross_damage_fraction":o.authoritative_damage/profile[5],
        "largest_damage_packet_fraction":o.largest_authoritative_packet/profile[5],
        "gross_combat_damage_fraction":gross_combat,
        "gross_postdeath_damage_fraction":o.authoritative_postdeath_damage/profile[5],
        "status_channel_postdeath_damage_fraction":o.postdeath_damage_packets.iter().filter(|(k,_)|["Venom","Poison","Fire","Bleeding","Blood loss"].contains(&k.as_str())).map(|(_,v)|v).sum::<f64>()/profile[5],
        "logged_damage_fraction":o.damage_packets.values().sum::<f64>()/profile[5],
        "capped_actual_damage_fraction":o.capped_actual_damage/profile[5],
        "initial_child_count":if whole_family{Some(targets.len())}else{None},
        "enemy_energy_min":o.enemy_energy_min,"enemy_energy_consumed":o.enemy_energy_consumed,
        "trace":if q.include_trace{Some(o.trace)}else{None}}))
}
fn summarize(q:Request)->Result<Value,String> {
    if !["wait","escape"].contains(&q.settlement_strategy.as_str()) {return Err("settlement_strategy must be wait or escape".into());}
    if q.profiles.is_empty() || q.profiles.iter().any(|p|p.iter().any(|n|!n.is_finite())||p[5]<=0.) {return Err("profiles must be nonempty finite [offense,defense,accuracy,dodge,dr,positiveHP] arrays".into());}
    let n=if q.seeds.is_empty(){q.trials}else{q.seeds.len()};
    if n==0 || n>1_000_000 || q.max_turns==0 || q.max_turns>100_000 || q.settle_turns>100_000 {return Err("Trial/command bounds: 1..1000000 trials, 1..100000 combat commands, 0..100000 settlement commands".into());}
    let mut rows=Vec::with_capacity(n);
    for i in 0..n {
        let seed=q.seeds.get(i).copied().unwrap_or_else(||corpse_core::rng::seed_hash(&format!("{}:{}",q.seed_prefix.clone().unwrap_or_else(||format!("trade31:{}:{}",q.kind,q.scenario)),i)));
        rows.push(run_trial(&q,i,seed)?);
    }
    let mean=|key:&str|rows.iter().map(|r|r[key].as_f64().unwrap_or(0.)).sum::<f64>()/n as f64;
    let count=|key:&str|rows.iter().filter(|r|r[key]==true).count();
    let standard_error=|key:&str|if n>1{let m=mean(key);Some((rows.iter().map(|r|(r[key].as_f64().unwrap_or(0.)-m).powi(2)).sum::<f64>()/(n*(n-1))as f64).sqrt())}else{None};
    let conditional_mean=|key:&str,predicate:&str| {
        let rows:Vec<_>=rows.iter().filter(|r|r[predicate]==true).collect();
        if rows.is_empty(){None}else{Some(rows.iter().map(|r|r[key].as_f64().unwrap_or(0.)).sum::<f64>()/rows.len()as f64)}
    };
    let killed=count("killed"); let deaths=count("died"); let censored=count("censored");
    let mut costs:Vec<_>=rows.iter().map(|r|r["total_cost"].as_f64().unwrap()).collect(); costs.sort_by(f64::total_cmp);
    let quantile=|p:f64|costs[((n-1)as f64*p).round()as usize];
    let mut markers=BTreeMap::<String,usize>::new(); let mut marker_trials=BTreeMap::<String,usize>::new();
    let mut reasons=BTreeMap::<String,usize>::new(); let mut damage=BTreeMap::<String,f64>::new(); let mut postdamage=BTreeMap::<String,f64>::new();
    for r in &rows {
        for (k,v) in r["markers"].as_object().unwrap(){*markers.entry(k.clone()).or_default()+=v.as_u64().unwrap()as usize;inc(&mut marker_trials,k);}
        if let Some(reason)=r["censor_reason"].as_str(){inc(&mut reasons,reason);}
        for (k,v) in r["damage_packets"].as_object().unwrap(){packet(&mut damage,k,v.as_f64().unwrap()/n as f64);}
        for (k,v) in r["postdeath_damage_packets"].as_object().unwrap(){packet(&mut postdamage,k,v.as_f64().unwrap()/n as f64);}
    }
    Ok(json!({"kind":q.kind,"revision":q.revision,"scenario":q.scenario,"candidate":q.candidate,
        "initial_actors":rows.first().map(|r|r["initial_actors"].clone()),
        "summoned_candidates":q.summoned_candidates,"settlement_strategy":q.settlement_strategy,
        "partner_candidate":q.partner_candidate,"partner_kind":q.partner_kind,"without_support":q.without_support,"trials":n,"profiles":q.profiles.len(),
        "measurement":"Player attack attempts include misses. Kill-conditional lifetime metrics are not unconditional expectations if deaths/censors occur. Gross damage uses authoritative pre-HP-cap native packets (can overkill), including unlogged spell packets; net HP loss includes native passive regen; death boundary is end of native dispatch containing death. Damage packets are raw logged packets and may overkill. Censored net costs are partial observations, not bounds or completed encounters; exclude them from calibration.",
        "mean_cost":mean("total_cost"),"mean_completed_cost":if n>censored{Some(rows.iter().filter(|r|r["censored"]==false).map(|r|r["total_cost"].as_f64().unwrap()).sum::<f64>()/(n-censored)as f64)}else{None},"mean_combat_cost":mean("combat_cost"),"mean_residual_net_cost":mean("residual_net_cost"),
        "mean_cost_at_enemy_death":if killed>0{Some(rows.iter().filter_map(|r|r["cost_at_enemy_death"].as_f64()).sum::<f64>()/killed as f64)}else{None},
        "mean_minor_death_cost":if rows.iter().any(|r|r["minor_death_cost"].is_number()){Some(rows.iter().filter_map(|r|r["minor_death_cost"].as_f64()).sum::<f64>()/rows.iter().filter(|r|r["minor_death_cost"].is_number()).count()as f64)}else{None},
        "minor_kills":rows.iter().filter(|r|r["minor_death_cost"].is_number()).count(),
        "mean_commands":mean("commands"),"mean_combat_commands":mean("combat_commands"),"mean_world_turns":mean("world_turns"),
        "mean_settlement_commands":mean("settlement_commands"),"mean_enemy_attack_attempts":mean("enemy_attack_attempts"),
        "mean_player_attack_attempts":mean("player_attack_attempts"),
        "mean_player_landed_attacks":mean("player_landed_attacks"),
        "mean_attacks_to_kill_conditional_on_kill":conditional_mean("player_attack_attempts","killed"),
        "mean_world_turns_to_kill_conditional_on_kill":conditional_mean("combat_world_turns","killed"),
        "mean_enemy_attacks_over_killed_lifespan":conditional_mean("enemy_attack_attempts","killed"),
        "mean_gross_damage_fraction":mean("gross_damage_fraction"),
        "mean_largest_damage_packet_fraction":mean("largest_damage_packet_fraction"),
        "max_damage_packet_fraction":rows.iter().filter_map(|r|r["largest_damage_packet_fraction"].as_f64()).fold(0.,f64::max),
        "mean_gross_combat_damage_fraction":mean("gross_combat_damage_fraction"),
        "gross_combat_standard_error":standard_error("gross_combat_damage_fraction"),
        "gross_total_standard_error":standard_error("gross_damage_fraction"),
        "mean_gross_postdeath_damage_fraction":mean("gross_postdeath_damage_fraction"),
        "mean_status_channel_postdeath_damage_fraction":mean("status_channel_postdeath_damage_fraction"),
        "mean_logged_damage_fraction":mean("logged_damage_fraction"),
        "mean_gross_damage_fraction_conditional_on_kill":conditional_mean("gross_damage_fraction","killed"),
        "all_primary_lifespans_observed":killed==n,
        "all_settlements_observed":killed==n&&censored==0,
        "mean_gross_combat_damage_fraction_conditional_on_kill":conditional_mean("gross_combat_damage_fraction","killed"),
        "mean_capped_actual_damage_fraction":mean("capped_actual_damage_fraction"),"mean_enemy_energy_consumed":mean("enemy_energy_consumed"),
        "kills":killed,"deaths":deaths,"combat_deaths":count("died_during_combat"),"residual_deaths":count("died_during_settlement"),"censored":censored,
        "combat_censored":count("combat_censored"),"aftermath_censored":count("aftermath_censored"),
        "kill_rate":killed as f64/n as f64,"death_rate":deaths as f64/n as f64,"censor_rate":censored as f64/n as f64,
        "p50_cost":quantile(0.5),"p90_cost":quantile(0.9),"p95_cost":quantile(0.95),"max_cost":quantile(1.),
        "markers":markers,"marker_trials":marker_trials,"censor_reasons":reasons,
        "mean_damage_packets":damage,"mean_postdeath_damage_packets":postdamage,
        "outcomes":if q.include_trials||q.include_trace{Some(rows)}else{None}}))
}
fn process(value:Value)->Value {match serde_json::from_value::<Request>(value).map_err(|e|e.to_string()).and_then(summarize){Ok(v)=>v,Err(e)=>json!({"error":e})}}
fn emit(value:Value) {
    if let Value::Array(values)=value {for v in values {println!("{}",process(v));}} else {println!("{}",process(value));}
    io::stdout().flush().unwrap();
}
fn main() {
    if let Some(path)=std::env::args().nth(1) {
        match std::fs::read_to_string(&path).map_err(|e|e.to_string()).and_then(|s|serde_json::from_str::<Value>(&s).map_err(|e|e.to_string())) {
            Ok(v)=>emit(v),Err(e)=>println!("{}",json!({"error":e})),
        }
    } else {
        for line in io::stdin().lock().lines() {
            match line.map_err(|e|e.to_string()).and_then(|s|serde_json::from_str::<Value>(&s).map_err(|e|e.to_string())) {
                Ok(v)=>emit(v),Err(e)=>println!("{}",json!({"error":e})),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(kind:&str,scenario:&str)->Request {serde_json::from_value(json!({"kind":kind,"scenario":scenario,"profiles":[[50,30,5,5,5,280]],"seeds":[1178],"include_trials":true})).unwrap()}
    #[test] fn preserves_traits_and_short_sword() {
        let q=request("yeti","adjacent"); let r=scene(&q,&q.profiles[0],1178).unwrap();
        assert!(r.player.abilities.is_empty()); assert_eq!(r.inventory[0].name,"Shortsword");
        assert!(r.actors_at(1.)[0].abilities.contains(&"enemy:beast:yeti".into()));
        assert_eq!(r.player.max_hp,280.);
    }
    #[test] fn rejects_stranded_aquatics() {
        let q=request("kelpie","adjacent"); assert!(scene(&q,&q.profiles[0],1).unwrap_err().contains("stranded"));
        let q=request("kelpie","water-shore"); let r=scene(&q,&q.profiles[0],1).unwrap();
        assert_eq!(r.floor().unwrap().tiles[2*9+2],"floor"); assert!(!corpse_core::ichor::stranded(&r,&r.actors_at(1.)[0]));
    }
    #[test] fn dispatch_executes_yeti_slow() {
        let v=run_trial(&request("yeti","adjacent"),0,1178).unwrap();
        assert!(v["markers"]["yeti_slow_extra_turn"].as_u64().unwrap_or(0)>0);
        assert!(v["world_turns"].as_f64().unwrap()>v["commands"].as_f64().unwrap());
    }
    #[test] fn linked_pair_keeps_minor_first_and_major_sleeping() {
        let q=request("ursa-major","ursa-pair"); let r=scene(&q,&q.profiles[0],1).unwrap();
        assert_eq!(r.actors_at(1.)[0].creature_kind.as_ref().unwrap(),"ursa-minor");
        let major=&r.actors_at(1.)[1]; assert_eq!(major.state,"sleeping");
        assert!(!corpse_core::vision::line_of_sight(&r,major,&r.player));
    }
    #[test] fn grass_sensitivity_uses_ordinary_fuel_not_permanent_fire() {
        let q=request("moonspawn","adjacent-grass"); let r=scene(&q,&q.profiles[0],1).unwrap();
        assert_eq!(r.floor().unwrap().tiles[2*9+2],"grass");
        assert_eq!(r.floor().unwrap().tiles[2*9+3],"grass");
        assert!(r.floor().unwrap().permanent_fire.as_ref().is_none_or(|v|v.iter().all(|n|*n==0.)));
    }
    #[test] fn deterministic_real_dispatch() {
        let q=request("pale:Biter","adjacent"); assert_eq!(run_trial(&q,0,1178).unwrap(),run_trial(&q,0,1178).unwrap());
    }
    #[test] fn empty_or_accuracy_only_candidates_preserve_authored_queen_hp() {
        let mut a=population::create_actor_revision("saidribee-queen",4.,3.,2.,false,33.).unwrap();
        assert_eq!((a.max_hp,a.stats.defense),(430.,48.));
        patch(&mut a,&Candidate::default()).unwrap();assert_eq!(a.max_hp,430.);
        patch(&mut a,&Candidate{accuracy:Some(149.),..Default::default()}).unwrap();
        assert_eq!((a.max_hp,a.hp,a.stats.accuracy),(430.,430.,149.));
    }
    #[test] fn miss_attempts_count_and_capped_damage_counter_is_separate() {
        let q=request("rat","adjacent");let mut r=scene(&q,&q.profiles[0],1).unwrap();
        let old=Before::capture(&r);
        corpse_core::combat_audit::begin();
        engine::damage_environment(&mut r,0.,12.,None,"attack",true).unwrap();
        let mut o=Observation::default();
        let out=engine::DispatchOutcome::default();
        observe(&r,&old,&["You strike Giant Rat, but miss.".into(),"You strike Giant Rat for 7 damage.".into()],&out,&mut o,false,false);
        assert_eq!(o.player_attacks,2);assert_eq!(o.player_hits,1);assert_eq!(o.capped_actual_damage,12.);
    }
    #[test] fn chase_moves_toward_distant_caster() {
        let q=request("horn-1","chase");let r=scene(&q,&q.profiles[0],1).unwrap();
        let command=player_command(&r,&r.actors_at(1.)[0],true);
        assert!(matches!(command,Command::Move{dx:1.,dy:0.}));
        assert!(matches!(player_command(&r,&r.actors_at(1.)[0],false),Command::Wait));
    }
    #[test] fn three_cast_payload_uses_native_twins_and_excludes_parent() {
        let mut q=request("pale:Birther","three-cast-children");q.revision=32.;
        let r=scene(&q,&q.profiles[0],1).unwrap();
        assert_eq!(r.tick,20.);assert_eq!(r.actors_at(1.).len(),4); // Native energy50: twins30, then singles10+10.
        assert!(r.actors_at(1.).iter().all(|a|a.born_from_birther.value_or(false)&&a.id!=4.));
        assert_eq!(r.player.hp,q.profiles[0][5]);
    }
    #[test] fn lifetime_expectation_is_explicitly_kill_conditional() {
        let mut q=request("sphinx","adjacent");q.max_turns=2;
        let result=summarize(q).unwrap();
        assert!(result["mean_attacks_to_kill_conditional_on_kill"].is_null());
        assert_eq!(result["censored"],1);
        assert!(result["measurement"].as_str().unwrap().contains("not unconditional"));
    }

    #[test] fn audit_records_unlogged_ruin_after_mitigation_before_hp_cap() {
        let q=request("zillennia:child","adjacent");let mut r=scene(&q,&q.profiles[0],1).unwrap();
        r.player.hp=5.;let player=r.player.id;
        corpse_core::combat_audit::begin();
        assert!(corpse_core::monster_ruin::special_action(&mut r,4.,player).unwrap());
        let packets=corpse_core::combat_audit::take();
        let player_packets:Vec<_>=packets.iter().filter(|p|p.victim==player).collect();
        assert_eq!(player_packets.len(),1);
        assert_eq!(player_packets[0].amount,18.);assert_eq!(player_packets[0].actual,5.);
    }
    #[test] fn audit_does_not_change_state_rng_or_events() {
        let q=request("rat","adjacent");let mut a=scene(&q,&q.profiles[0],1178).unwrap();let mut b=a.clone();
        let command=Command::Move{dx:1.,dy:0.};
        corpse_core::combat_audit::take();let x=engine::dispatch(&mut a,&command).unwrap();
        corpse_core::combat_audit::begin();let y=engine::dispatch(&mut b,&command).unwrap();corpse_core::combat_audit::take();
        assert_eq!(serde_json::to_value(a).unwrap(),serde_json::to_value(b).unwrap());assert_eq!(x.events,y.events);
    }
    #[test] fn support_counterpart_preserves_frontliner() {
        let mut q=request("dryad","support-with-fighter");q.partner_kind=Some("bear".into());
        let support=scene(&q,&q.profiles[0],1).unwrap();q.without_support=true;
        let alone=scene(&q,&q.profiles[0],1).unwrap();
        assert_eq!(support.actors_at(1.).len(),2);assert_eq!(alone.actors_at(1.).len(),1);
        assert_eq!(support.actors_at(1.)[0],alone.actors_at(1.)[0]);assert_eq!(support.rng,alone.rng);
    }

    #[test] fn explicit_summoned_candidates_change_only_newborn_numbers() {
        let mut q=request("pale:Birther","three-cast-children");q.revision=32.;
        q.summoned_candidates.insert("pale:Jackal".into(),Candidate{offense:Some(99.),defense:Some(5.),..Default::default()});
        let mut r=scene(&q,&q.profiles[0],1).unwrap();
        assert!(r.actors_at(1.).iter().all(|a|a.stats.offense==99.&&a.hp==130.&&a.max_hp==130.));
        let rng=r.rng;let id=r.actors_at(1.)[0].id;r.actors_at_mut(1.)[0].hp=65.;
        q.summoned_candidates.get_mut("pale:Jackal").unwrap().defense=Some(20.);
        patch_newborns(&mut r,&q,id).unwrap();
        assert_eq!(r.actors_at(1.)[0].hp,110.);assert_eq!(r.actors_at(1.)[0].max_hp,220.);assert_eq!(r.rng,rng);
    }

    #[test] fn escape_moves_out_of_hazard_and_does_not_wait_for_remote_gas() {
        let q=request("rat","adjacent");let mut r=scene(&q,&q.profiles[0],1).unwrap();let w=r.floor().unwrap().width();
        corpse_core::environment::add_gas(r.floor_mut().unwrap(),2*w+2,"caustic",80.,-1.).unwrap();
        assert!(matches!(escape_command(&r).unwrap(),Some(Command::Move{dx:0.,dy:-1.})));
        r.player.x=2.;r.player.y=1.;
        assert!(escape_command(&r).unwrap().is_none());assert!(!pending(&r).is_empty());
        r.player.conditions.insert("poisoned".into(),2.);
        assert!(matches!(escape_command(&r).unwrap(),Some(Command::Wait)));
    }
    #[test] fn escape_reports_trapped_instead_of_claiming_settled_lifetime() {
        let q=request("rat","adjacent");let mut r=scene(&q,&q.profiles[0],1).unwrap();let w=r.floor().unwrap().width();
        for y in 1..8 {for x in 1..8 {corpse_core::environment::add_gas(r.floor_mut().unwrap(),y*w+x,"caustic",80.,-1.).unwrap();}}
        assert!(escape_command(&r).unwrap_err().contains("trapped_no_reachable_clear_cell"));
    }

    #[test] fn whole_family_is_opt_in_and_queen_override_is_explicit() {
        let mut q=request("rat","adjacent"); assert!(!q.whole_family);
        let mut r=scene(&q,&q.profiles[0],1).unwrap();
        q.summoned_candidates.insert("rat".into(),Candidate{offense:Some(3.),..Default::default()});
        let old=r.actors_at(1.)[0].stats.offense;
        patch_newborns(&mut r,&q,0.).unwrap(); assert_eq!(r.actors_at(1.)[0].stats.offense,old);
        r.actors_at_mut(1.)[0].conditions.insert("queenSummoned".into(),1.);
        patch_newborns(&mut r,&q,100.).unwrap(); assert_eq!(r.actors_at(1.)[0].stats.offense,old);
        patch_newborns(&mut r,&q,0.).unwrap(); assert_eq!(r.actors_at(1.)[0].stats.offense,3.);
        q.whole_family=true; assert!(q.whole_family);
    }

}
