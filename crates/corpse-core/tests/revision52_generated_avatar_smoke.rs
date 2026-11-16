//! Generated-world, headless simulation smoke, not a human or Windows playthrough.
//! No fixture import, actor relocation, artificial loot, RNG edits, or runtime edits.
//! A bounded read-only planner uses full floor knowledge to seek real generated loot,
//! then return toward and explore near the entrance. Only public Engine commands change the run.
//! Optional COTCG_GENERATED_AVATAR_SMOKE_EXPORT_DIR writes authentic final saves,
//! seed replays and command traces for a separate later native-renderer capture.
use corpse_core::{
    Command, Run, engine, environment, movement::{self, Point}, progression,
    replay::{ReplayRecord, ReplaySession}, revision50_inventory, startup,
    temporal::Engine,
};
use serde_json::{Value, json};
use std::{cmp::Reverse, collections::{BTreeMap, BTreeSet, BinaryHeap}, path::Path,
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind}};

const COMMANDS: usize = 64;
const CHECKPOINT: usize = 32;

fn position(run: &Run) -> (i32, i32) {
    (run.player.x as i32, run.player.y as i32)
}

fn assert_birth(run: &Run, seed: &str, gods: [f64; 3]) {
    assert_eq!(run.seed, seed);
    assert_eq!(run.mode, "Avatar");
    assert!(!run.dev);
    assert!(run.playtest.is_missing());
    assert_eq!(run.simulation_revision.as_ref(), Some(&52.));
    assert_eq!(run.gods, gods);
    assert_eq!(run.player.glyph, "@");
    assert_eq!((run.player.stats.accuracy, run.player.stats.dodge), (15., 15.));
    assert_eq!(run.player.abilities.len(), 3);
    let reserves = run.player.ability_reserves.as_ref().expect("starter reserves");
    assert_eq!(reserves.len(), 3);
    for (god, ability) in gods.into_iter().zip(&run.player.abilities) {
        assert!(progression::starter_abilities_for_revision(god as u32, 52.)
            .unwrap().contains(ability));
        assert_eq!(reserves[ability].capacity, 10.);
        assert_eq!(reserves[ability].energy, 10.);
    }
    assert_eq!(run.floors.len(), 2);
    assert_eq!(run.actors.len(), 2);
    assert_eq!(run.ground.len(), 2);
    assert_eq!(run.depth, 1.);
    assert_eq!(run.tick, 0.);
    assert_eq!(run.status, "playing");
    assert!(run.replay.is_empty());
}

// Planner-only work, never a simulation write. O(V*(A+H)+V log V+G*(I+B))
// conservative time and O(V+A+G) scratch per selection: V cells, A active actors,
// H existing terrain/admission helper cost, G ground entries, I items, B blessings.
// Cardinal Dijkstra has O(V) edges; these generated floors have 2,291 cells.
struct Paths { cost: Vec<usize>, steps: Vec<usize>, previous: Vec<usize>, threat: Vec<usize> }
fn hostiles(run: &Run) -> impl Iterator<Item=&corpse_core::Actor> {
    run.actors_at(run.depth).iter().filter(|a| !a.dead.value_or(false) && a.hp > 0.
        && !["ally", "neutral"].contains(&a.state.as_str())
        && corpse_core::dreamwalking::same_plane(run, a, &run.player))
}
fn threat_distance(run: &Run) -> usize {
    hostiles(run).map(|a| (a.x-run.player.x).abs().max((a.y-run.player.y).abs()) as usize)
        .min().unwrap_or(usize::MAX)
}
fn paths(run: &Run) -> Paths {
    let floor = run.floor().unwrap();
    let width = floor.width();
    let n = floor.tiles.len();
    let start = run.player.y as usize * width + run.player.x as usize;
    let occupied: BTreeSet<usize> = run.actors_at(run.depth).iter()
        .filter(|a| !a.dead.value_or(false) && a.hp > 0.
            && corpse_core::dreamwalking::same_plane(run, a, &run.player))
        .map(|a| a.y as usize * width + a.x as usize).collect();
    let allowed: Vec<bool> = (0..n).map(|at| {
        let (x, y) = ((at % width) as f64, (at / width) as f64);
        ["floor", "grass", "up", "down"].contains(&floor.tiles[at].as_str())
            && !occupied.contains(&at)
            && engine::actor_passable(run, &run.player, x, y)
            && !environment::creature_hazard_at(run, &run.player, at)
            && corpse_core::portals::step_options(run, &run.player, x, y) == vec![(x, y)]
    }).collect();
    // Conservative proximity estimate, not a simulation forecast. It includes
    // sleeping/wandering foes and sums packs; walls can only overstate the risk.
    let threat: Vec<usize> = (0..n).map(|at| hostiles(run).map(|a| {
        let d = ((at % width) as f64-a.x).abs().max(((at / width) as f64-a.y).abs()) as usize;
        match d { 0|1 => 64, 2 => 24, 3 => 8, 4 => 3, 5 => 1, _ => 0 }
    }).sum()).collect();
    let mut cost = vec![usize::MAX; n];
    let mut steps = vec![usize::MAX; n];
    let mut previous = vec![usize::MAX; n];
    cost[start] = 0;
    steps[start] = 0;
    let mut queue = BinaryHeap::from([Reverse((0usize, 0usize, start))]);
    while let Some(Reverse((weight, length, at))) = queue.pop() {
        if (weight, length) != (cost[at], steps[at]) { continue; }
        let from = Point { x: (at % width) as i32, y: (at / width) as i32 };
        for delta in &movement::DIRECTIONS[..4] {
            let to = Point { x: from.x + delta.x, y: from.y + delta.y };
            if to.x < 0 || to.y < 0 || to.x as usize >= width || to.y as usize >= floor.height() {
                continue;
            }
            let next = to.y as usize * width + to.x as usize;
            if !allowed[next] { continue; }
            let candidate = (weight + 1 + threat[next], length + 1);
            if candidate >= (cost[next], steps[next]) { continue; }
            (cost[next], steps[next]) = candidate;
            previous[next] = at;
            queue.push(Reverse((cost[next], steps[next], next)));
        }
    }
    Paths { cost, steps, previous, threat }
}

fn next_command(run: &Run, step: usize, origin: (i32, i32), collected: bool,
    visits: &BTreeMap<(i32, i32), usize>, waits: usize) -> Command {
    if let Some(id) = run.offers.first() { return Command::Choose { id: id.clone() }; }
    if step % 9 == 0 { return Command::Pickup; }
    // Exercise four real waits while safe instead of waiting next to a hunter
    // because an arbitrary command index says so. The assertion stays strict.
    let nearest = threat_distance(run);
    if waits < 4 && nearest >= 4 { return Command::Wait; }
    let floor = run.floor().unwrap();
    let width = floor.width();
    let start = run.player.y as usize * width + run.player.x as usize;
    let Paths { cost, steps, previous, threat } = paths(run);
    let movement = |at: usize| Command::Move {
        dx: (at % width) as f64 - run.player.x,
        dy: (at / width) as f64 - run.player.y,
    };
    // Immediate retreat outranks a loot/home goal when a safer adjacent square
    // exists. No look-ahead dispatches, actor relocation, or state edits occur.
    if nearest <= 2 {
        if let Some(at) = (0..steps.len()).filter(|&at| steps[at] == 1 && threat[at] < threat[start])
            .min_by_key(|&at| {
                let p = ((at % width) as i32, (at / width) as i32);
                (threat[at], visits.get(&p).copied().unwrap_or(0), at)
            }) { return movement(at); }
    }
    let target = if !collected {
        run.ground.get(&run.depth.to_string()).into_iter().flat_map(|v| v.iter())
            .filter(|g| revision50_inventory::space_for(run, &g.item) > 0.
                && !g.auto_pickup_blocked.value_or(false))
            .map(|g| g.y as usize * width + g.x as usize)
            .filter(|&at| cost[at] != usize::MAX)
            .min_by_key(|&at| (cost[at], steps[at], at))
    } else {
        let home = origin.1 as usize * width + origin.0 as usize;
        let (x, y) = position(run);
        if (x-origin.0).abs() + (y-origin.1).abs() > 6 {
            (home != start && cost[home] != usize::MAX).then_some(home)
        } else {
            (0..cost.len()).filter(|&at| steps[at] > 0 && cost[at] != usize::MAX)
                .filter(|&at| ((at % width) as i32-origin.0).abs()
                    + ((at / width) as i32-origin.1).abs() <= 6)
                .min_by_key(|&at| {
                    let p = ((at % width) as i32, (at / width) as i32);
                    (threat[at], visits.get(&p).copied().unwrap_or(0), cost[at], at)
                })
        }
    };
    if let Some(mut target) = target {
        if target == start { return Command::Pickup; }
        while previous[target] != start {
            assert_ne!(previous[target], usize::MAX, "Dijkstra predecessor must reach player");
            target = previous[target];
        }
        return movement(target);
    }
    let nearby = (0..steps.len()).filter(|&at| steps[at] == 1)
        .min_by_key(|&at| {
            let p = ((at % width) as i32, (at / width) as i32);
            (threat[at], visits.get(&p).copied().unwrap_or(0), at)
        });
    nearby.map_or(Command::Wait, movement)
}

fn exact_save(actual: &Engine, expected: &Engine, label: &str) {
    let a = actual.save().unwrap();
    let b = expected.save().unwrap();
    if a != b {
        let av: Value = serde_json::from_str(&a).unwrap();
        let bv: Value = serde_json::from_str(&b).unwrap();
        let differing: Vec<_> = av.as_object().unwrap().keys()
            .filter(|key| av.get(*key) != bv.get(*key)).collect();
        panic!("{label}: full serialized save differs (no fields excluded): {differing:?}");
    }
}

fn write_json(dir: &Path, name: &str, value: &impl serde::Serialize) {
    std::fs::write(dir.join(name), serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

// Diagnostic capture preserves failed gameplay instead of concealing it behind
// a survival assertion. This reads/serializes only and never changes the run.
fn nearby_actors(run: &Run) -> Vec<Value> {
    run.actors_at(run.depth).iter()
        .filter(|a| !a.dead.value_or(false) && (a.x-run.player.x).abs().max((a.y-run.player.y).abs()) <= 8.)
        .map(|a| json!({"id":a.id,"name":a.name,"kind":a.creature_kind,
            "position":[a.x,a.y],"state":a.state,"hp":a.hp,
            "abilities":a.abilities,"conditions":a.conditions})).collect()
}
fn export_failure(name: &str, engine: &Engine, trace: &[Value],
    initial_save: &str, checkpoint_save: &str) {
    let run = &engine.run;
    let record = ReplayRecord::try_from_run(run).unwrap();
    let summary = json!({"case":name,"seed":run.seed,"commandsCompleted":run.replay.len(),
        "status":run.status,"tick":run.tick,"depth":run.depth,"position":position(run),
        "hp":run.player.hp,"energy":run.player.energy,"playerConditions":run.player.conditions,
        "death":run.run_stats.as_ref().and_then(|s| s.death.as_ref()),
        "nearbyActors":nearby_actors(run),"logTail":run.log.iter().rev().take(12).collect::<Vec<_>>(),
        "verification":"smoke assertion/dispatch failure; planner, runtime and seed are not rewritten"});
    println!("GENERATED_AVATAR_SMOKE_FAILURE {}", serde_json::to_string(&summary).unwrap());
    if let Some(root) = std::env::var_os("COTCG_GENERATED_AVATAR_SMOKE_EXPORT_DIR") {
        let dir = Path::new(&root).join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("failed-run.json"), engine.save().unwrap()).unwrap();
        std::fs::write(dir.join("failed-initial-run.json"), initial_save).unwrap();
        std::fs::write(dir.join("failed-checkpoint-run.json"), checkpoint_save).unwrap();
        write_json(&dir, "failed-replay.json", &record);
        write_json(&dir, "failed-trajectory.json", &trace);
        write_json(&dir, "failure-summary.json", &summary);
    }
}

fn smoke(name: &str, seed: &str, gods: [f64; 3], expect_generated_loot: bool) {
    let run = startup::create_avatar_run(seed).unwrap_or_else(|e| panic!("{seed}: startup: {e}"));
    let birth_run = run.clone();
    let initial_abilities = run.player.abilities.clone();
    let initial_inventory = run.inventory.clone();
    let initial_ground_ids: BTreeSet<u64> = run.ground.values().flat_map(|v| v.iter())
        .map(|g| g.item.id as u64).collect();
    let origin = position(&run);
    let mut engine = Engine::new(run).unwrap();
    let initial_save = engine.save().unwrap();
    let mut resumed: Option<Engine> = None;
    let mut checkpoint_save = String::new();
    let mut visits = BTreeMap::from([(origin, 1usize)]);
    let mut acquired = BTreeSet::new();
    let mut trace = vec![];
    let mut outcomes = vec![];
    let mut move_commands = 0;
    let mut moves_completed = 0;
    let mut waits = 0;
    let mut pickups = 0;
    let mut turn_commands = 0;
    let mut resumed_commands = 0;
    // Export the last genuine state and trace before propagating any assertion
    // or dispatch panic. Failed checks never erase the evidence they inspected.
    let result = catch_unwind(AssertUnwindSafe(|| {
    assert_birth(&birth_run, seed, gods);
    let initial_load = Engine::load(&initial_save).unwrap();
    exact_save(&initial_load, &engine, "startup save/load");
    for step in 0..COMMANDS {
        assert_eq!(engine.run.status, "playing", "{seed}: never send commands after a terminal outcome");
        let command = next_command(&engine.run, step, origin, !acquired.is_empty(), &visits, waits);
        match command {
            Command::Move { .. } => move_commands += 1,
            Command::Wait => waits += 1,
            Command::Pickup => pickups += 1,
            _ => {},
        }
        let before = position(&engine.run);
        let actors_before = nearby_actors(&engine.run);
        let tick_before = engine.run.tick;
        let ledger_before = engine.run.replay.len();
        let outcome = engine.dispatch(&command)
            .unwrap_or_else(|e| panic!("{seed}, command {step}, {command:?}: {e}"));
        assert_eq!(engine.run.replay.len(), ledger_before + 1, "every attempted command is recorded");
        turn_commands += usize::from(outcome.took_turn);
        moves_completed += usize::from(matches!(command, Command::Move { .. }) && before != position(&engine.run));
        *visits.entry(position(&engine.run)).or_default() += 1;
        acquired.extend(engine.run.inventory.iter().map(|i| i.id as u64)
            .filter(|id| initial_ground_ids.contains(id)));
        if let Some(other) = &mut resumed {
            let after_load = other.dispatch(&command).unwrap();
            assert_eq!((after_load.took_turn, after_load.changed), (outcome.took_turn, outcome.changed));
            assert_eq!(after_load.events, outcome.events, "{seed}: resumed presentation events at {step}");
            exact_save(other, &engine, &format!("{seed}: resumed command {step}"));
            resumed_commands += 1;
        }
        trace.push(json!({"commandIndex":step,"command":command,"tookTurn":outcome.took_turn,
            "changed":outcome.changed,"tickBefore":tick_before,"tickAfter":engine.run.tick,
            "positionBefore":before,"positionAfter":position(&engine.run),
            "hp":engine.run.player.hp,"energy":engine.run.player.energy,
            "depth":engine.run.depth,"status":engine.run.status,"events":outcome.events,
            "nearbyActorsBefore":actors_before,
            "logTailAfter":engine.run.log.iter().rev().take(8).collect::<Vec<_>>()}));
        outcomes.push(outcome);
        if step + 1 == CHECKPOINT {
            checkpoint_save = engine.save().unwrap();
            let loaded = Engine::load(&checkpoint_save).unwrap();
            exact_save(&loaded, &engine, "mid-trajectory save/load");
            resumed = Some(loaded);
        }
        // The retreat case records the real terminal command, then stops. No
        // rejected post-death inputs are counted as gameplay or replay coverage.
        if engine.run.status != "playing" { break; }
    }
    let commands = engine.run.replay.len();
    assert!(commands > CHECKPOINT, "{seed}: must exercise a real resumed suffix");
    assert_eq!(commands, trace.len());
    assert_eq!(commands, outcomes.len());
    assert!(trace[..trace.len()-1].iter().all(|s| s["status"] == "playing"),
        "{seed}: the only possible terminal state is the last accepted command");
    if expect_generated_loot || engine.run.status == "playing" {
        assert_eq!(engine.run.status, "playing", "{seed}: loot trajectory must survive");
        assert_eq!(commands, COMMANDS);
    } else {
        assert_eq!(engine.run.status, "dead", "{seed}: unsupported terminal outcome");
        assert_eq!(engine.run.player.hp, 0.);
        assert_eq!(trace.last().unwrap()["status"], "dead");
        let death = engine.run.run_stats.as_ref().and_then(|s| s.death.as_ref())
            .expect("terminal death must retain its actual cause");
        assert!(!death.reason.is_empty());
        assert_eq!((death.depth, death.tick), (engine.run.depth, engine.run.tick));
        assert!(commands <= COMMANDS);
    }
    assert_eq!(engine.run.depth, 1., "this smoke does not claim floor traversal");
    assert_eq!(engine.run.floors.len(), 2);
    assert_eq!(resumed_commands, commands - CHECKPOINT);
    assert!(moves_completed >= 24, "{seed}: only {moves_completed} actual moves");
    assert!(visits.len() >= 6, "{seed}: only {} distinct player cells", visits.len());
    assert!(turn_commands >= 40, "{seed}: only {turn_commands} turn-taking commands");
    assert!(engine.run.tick >= 40., "{seed}: only {} real world ticks", engine.run.tick);
    assert!(waits >= 4 && pickups >= 4);
    if expect_generated_loot {
        assert!(!acquired.is_empty(), "{seed}: no real generated ground item acquired");
    } else {
        // This seed is a retreat/terminal-outcome scenario, not loot coverage.
        // Its real pursuit and possible death remain; the bot cannot manufacture an item to
        // meet a goal. The other two generated seeds exercise actual pickups.
        assert!(acquired.is_empty(), "{seed}: retreat coverage expects no acquired ground item");
        assert_eq!(engine.run.inventory, initial_inventory,
            "{seed}: retreat must retain exactly the genuine starting inventory");
    }
    let record = ReplayRecord::try_from_run(&engine.run).unwrap();
    let mut replay = ReplaySession::create(record.clone(), true).unwrap();
    for (index, expected) in outcomes.iter().enumerate() {
        let replayed = replay.step_with_outcome().unwrap().expect("recorded command");
        assert_eq!((replayed.took_turn, replayed.changed), (expected.took_turn, expected.changed));
        assert_eq!(replayed.events, expected.events, "{seed}: seed replay events at {index}");
    }
    assert!(replay.finished());
    assert_eq!(replay.cursor, commands, "replay ends at the actual final command");
    exact_save(&replay.engine, &engine, "seed replay final outcome");
    let final_save = engine.save().unwrap();
    exact_save(&Engine::load(&final_save).unwrap(), &engine, "final save/load");
    let summary = json!({"case":name,"seed":seed,"gods":gods,"starterAbilities":initial_abilities,
        "verification":"headless generated-world simulation; not Windows or live SDL gameplay",
        "scenario":if expect_generated_loot { "generated loot acquisition" } else { "retreat through its actual terminal outcome without loot acquisition" },
        "planner":"read-only full-world threat-weighted cardinal Dijkstra, safe waits and immediate retreat; real public dispatch only",
        "commands":commands,"commandBudget":COMMANDS,"checkpointAfterCommand":CHECKPOINT,
        "postTerminalCommands":0,"death":engine.run.run_stats.as_ref().and_then(|s| s.death.as_ref()),
        "turnTakingCommands":turn_commands,"worldTicks":engine.run.tick,
        "moveCommands":move_commands,"completedMoves":moves_completed,"distinctPlayerCells":visits.len(),
        "waitCommands":waits,"pickupCommands":pickups,"generatedItemIdsAcquired":acquired,
        "pickupSemantics":"generated items collected by normal automatic underfoot pickup during dispatch",
        "saveResumeCommands":resumed_commands,"seedReplayCommands":replay.cursor,
        "exactSaveComparisons":"all serialized fields; startup, checkpoint after command 32, each resumed step, final, replay final",
        "finalDepth":engine.run.depth,"generatedFloors":engine.run.floors.len(),
        "finalHp":engine.run.player.hp,"finalEnergy":engine.run.player.energy,"finalStatus":engine.run.status,
        "limits":format!("{commands} actual commands (budget {COMMANDS}) on floor 1; stops at a terminal outcome; no descent, broad ability-mechanic or renderer certification")});
    println!("GENERATED_AVATAR_SMOKE {}", serde_json::to_string(&summary).unwrap());
    if let Some(root) = std::env::var_os("COTCG_GENERATED_AVATAR_SMOKE_EXPORT_DIR") {
        let dir = Path::new(&root).join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("initial-run.json"), &initial_save).unwrap();
        std::fs::write(dir.join("checkpoint-run.json"), &checkpoint_save).unwrap();
        std::fs::write(dir.join("final-run.json"), final_save).unwrap();
        write_json(&dir, "replay.json", &record);
        write_json(&dir, "trajectory.json", &trace);
        write_json(&dir, "summary.json", &summary);
    }
    }));
    if let Err(payload) = result {
        export_failure(name, &engine, &trace, &initial_save, &checkpoint_save);
        resume_unwind(payload);
    }
}

#[test]
fn avatar_0_5_11_generated_gameplay_save_resume_and_seed_replay() {
    smoke("gods-0-5-11", "0-5-11:avatar-smoke-r52", [0., 5., 11.], true);
}
#[test]
fn avatar_1_4_8_generated_retreat_terminal_outcome_save_resume_and_seed_replay() {
    smoke("gods-1-4-8", "1-4-8:avatar-smoke-r52", [1., 4., 8.], false);
}
#[test]
fn avatar_2_6_10_generated_gameplay_save_resume_and_seed_replay() {
    smoke("gods-2-6-10", "2-6-10:avatar-smoke-r52", [2., 6., 10.], true);
}
