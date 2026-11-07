//! Developer-only, seeded full-generation heart census; no survival simulation.
//! Usage: heart_progression_audit [count=128] [first_index=0] [seed_prefix=heart-audit] [revision=31]
//! One JSON line per seed, with all forty entrance/exit profiles. Uses real generation,
//! heart creation, inventory consumption, and progression. Primary reference suppresses
//! chosen level rewards explicitly; separate Unblessed profiles execute their actual +5 roll.
//! No turns, environmental spoilage, summons, revivals, loot or survival selection.
//! For S seeds, D<=40 floors and H hearts: S full-generation costs plus O(S*H*(I+L))
//! collection overhead, I<=4 inventory, L growing resource/log bookkeeping. Four runs
//! share generated Arc floors; output O(S*D) plus species census. Nothing ships in-game.
use corpse_core::{Actor, Command, Run, combat, inventory, progression, startup};
use serde_json::{Value, json};
use std::{collections::BTreeMap, io::{self, Write}};

struct Track { run: Run, hearts: usize, blessings: f64, choose: bool }
impl Track {
    fn new(run: &Run, choose: bool) -> Self {
        Self { run: run.clone(), hearts: 0, blessings: 0., choose }
    }
    fn snapshot(&self) -> Value {
        let stats = combat::effective_stats(&self.run, &self.run.player);
        let hp = corpse_core::divine_combat::health_from_stats(&self.run, &self.run.player, &stats);
        json!({"stats":stats,"max_hp":hp,"level":self.run.level,"unspent_blessings":self.run.exp,
            "hearts":self.hearts,"blessings":self.blessings,
            "shortsword_base":30.+stats.offense})
    }
    fn consume(&mut self, a: &Actor) -> Result<(), String> {
        let heart = inventory::heart_for(a, self.run.next_id).map_err(|e|e.0)?;
        self.run.next_id += 1.;
        let id = heart.id;
        self.blessings += heart.exp.value_or(0.);
        self.hearts += 1;
        self.run.inventory.push(heart);
        if inventory::command(&mut self.run, &Command::Use{id}).map_err(|e|e.0)? != Some(true) {
            return Err("Real inventory heart consumption failed".into());
        }
        while !self.run.offers.is_empty() {
            if self.choose {
                let id = self.run.offers[0].clone();
                if !id.starts_with("eclipsed") { return Err("Expected Unblessed offer".into()); }
                progression::choose_offer(&mut self.run, &id).map_err(|e|e.0)?;
            } else {
                // Explicit analytical intervention: account for level costs, but
                // no arbitrary ability choice or Unblessed +5 is in the primary.
                self.run.offers.clear();
                progression::level_up(&mut self.run);
            }
        }
        Ok(())
    }
}
fn ordinary(a: &Actor) -> bool {
    a.creature_kind.as_ref().is_none_or(|k| k != "zillennia") && !a.conditions.contains_key("godbeastDepth")
}
fn seed(index: usize, prefix: &str, revision: f64) -> Result<Value, String> {
    let mut generated = startup::new_run_shell_revision(&format!("{prefix}-{index:06}"), "Unblessed", false, revision)?;
    startup::ensure_floors(&mut generated, 40)?;
    let mut all = Track::new(&generated, false);
    let mut ord = Track::new(&generated, false);
    let mut all_unblessed = Track::new(&generated, true);
    let mut ord_unblessed = Track::new(&generated, true);
    let mut floors = Vec::new();
    for depth in 1..=40 {
        for t in [&mut all, &mut ord, &mut all_unblessed, &mut ord_unblessed] { t.run.depth = depth as f64; }
        let entry = json!({"all_initial":all.snapshot(),"ordinary":ord.snapshot(),
            "all_initial_unblessed":all_unblessed.snapshot(),"ordinary_unblessed":ord_unblessed.snapshot()});
        let mut species: BTreeMap<String, usize> = BTreeMap::new();
        let mut floor_hearts = 0;
        let mut zero_blessing_hearts = 0;
        let mut floor_blessings = 0.;
        let mut ordinary_blessings = 0.;
        for actor in generated.actors_at(depth as f64) {
            if actor.dead.value_or(false) || actor.heartless.value_or(false) { continue; }
            let kind = actor.creature_kind.as_ref().unwrap_or(&actor.name).clone();
            *species.entry(kind).or_default() += 1;
            let heart = inventory::heart_for(actor, 0.).map_err(|e| e.0)?;
            let exp = heart.exp.value_or(0.);
            floor_hearts += 1;
            floor_blessings += exp;
            if exp == 0. { zero_blessing_hearts += 1; }
            all.consume(actor)?;
            all_unblessed.consume(actor)?;
            if ordinary(actor) {
                ordinary_blessings += exp;
                ord.consume(actor)?;
                ord_unblessed.consume(actor)?;
            }
        }
        let floor = &generated.floors[depth-1];
        let exit = json!({"all_initial":all.snapshot(),"ordinary":ord.snapshot(),
            "all_initial_unblessed":all_unblessed.snapshot(),"ordinary_unblessed":ord_unblessed.snapshot()});
        floors.push(json!({"depth":depth,"domain":floor.domain,"entry":entry,"exit":exit,
            "initial_actors":generated.actors_at(depth as f64).len(),"hearts":floor_hearts,
            "zero_blessing_hearts":zero_blessing_hearts,"blessings":floor_blessings,
            "ordinary_blessings":ordinary_blessings,"ordinary_budget":floor.enemy_blessing_budget,
            "species":species,"godbeast_event":floor.godbeast_event}));
    }
    Ok(json!({"index":index,"seed":generated.seed,"revision":generated.simulation_revision,"floors":floors}))
}
fn main() {
    let args:Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|s|s == "fish-ledger") {
        let revision:f64 = args.get(2).map_or(Ok(31.), |s|s.parse()).expect("revision");
        let floors:Vec<_> = (1..=40).map(|depth| {
            let kinds = corpse_core::ichor::eligible_fish_kinds(depth, revision);
            let hearts:Vec<_> = (0..kinds.len()).map(|i| {
                let actor = corpse_core::ichor::make_fish_revision(1., 0., 0., depth as f64,
                    (i as f64+0.5)/kinds.len() as f64, revision);
                inventory::heart_for(&actor,0.).expect("event fish heart")
            }).collect();
            json!({"depth":depth,"kinds":kinds,"hearts":hearts,"hard_cap":10})
        }).collect();
        println!("{}",json!({"revision":revision,"floors":floors}));
        return;
    }
    let count:usize = args.get(1).map_or(Ok(128), |s|s.parse()).expect("count");
    let first:usize = args.get(2).map_or(Ok(0), |s|s.parse()).expect("first index");
    let prefix = args.get(3).map(String::as_str).unwrap_or("heart-audit");
    let revision:f64 = args.get(4).map_or(Ok(31.), |s|s.parse()).expect("revision");
    let start = std::time::Instant::now();
    let mut out = io::BufWriter::new(io::stdout().lock());
    for index in first..first+count {
        let result = seed(index, prefix, revision).unwrap_or_else(|e| {
            eprintln!("heart audit FAILED seed index {index}: {e}");
            json!({"index":index,"requested_seed":format!("{prefix}-{index:06}"),"revision":revision,"error":e})
        });
        serde_json::to_writer(&mut out, &result).unwrap();
        writeln!(&mut out).unwrap();
        out.flush().unwrap();
        eprintln!("heart audit completed index {index}: {:.2}s cumulative", start.elapsed().as_secs_f64());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn heart_only_and_unblessed_follow_real_rewards_and_level_thresholds() {
        let run = startup::new_run_shell_revision("0-5-11:heart-audit-unit", "Unblessed", false, 31.).unwrap();
        let actor = corpse_core::population::create_actor_revision("pale:Biter",4.,3.,3.,false,31.).unwrap();
        let mut bare = Track::new(&run,false);
        let mut unblessed = Track::new(&run,true);
        for _ in 0..6 { bare.consume(&actor).unwrap(); unblessed.consume(&actor).unwrap(); }
        assert_eq!(bare.run.player.stats.offense,12.);
        assert_eq!(bare.run.player.stats.defense,0.);
        assert_eq!((bare.run.level,bare.run.exp),(3.,0.));
        assert_eq!(bare.blessings,6.);
        assert_eq!(unblessed.run.player.stats.offense+unblessed.run.player.stats.defense,22.);
        assert_eq!((unblessed.run.level,unblessed.run.exp),(3.,0.));
        assert_eq!(bare.hearts,6);
        assert_eq!(bare.run.inventory.len(),3);
    }
}
