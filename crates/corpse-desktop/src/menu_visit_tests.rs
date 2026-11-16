use super::*;
use std::cell::Cell;

fn visit(visits:&mut Visits, screen:Screen, busy:bool, selection:&mut Selection,
         seed:&mut String, notice:&mut String, calls:&Cell<u32>) -> bool {
    visits.observe_with(screen,busy,selection,seed,notice,||{
        let n=calls.get();calls.set(n+1);Ok([n+3,n+5,n+7])
    })
}

#[test]
fn menu_visits_sample_every_ordered_distinct_triple_without_game_rng() {
    let mut triples=std::collections::HashSet::new();
    for a in 0..12 {for b in 0..11 {for c in 0..10 {
        let mut selection=Selection::default();let mut seed=String::new();
        selection.randomize(&mut seed,[a,b,c]).unwrap();
        let [x,y,z]=selection.gods;
        assert!(x<12 && y<12 && z<12 && x!=y && x!=z && y!=z);
        assert!(triples.insert(selection.gods));assert!(seed.is_empty());
    }}}
    assert_eq!(triples.len(),12*11*10);
    let mut selection=Selection::default();
    selection.randomize(&mut String::new(),[u32::MAX;3]).unwrap();
    assert_eq!(selection.gods.len(),3);
}

#[test]
fn menu_visits_reroll_every_return_path_once_before_next_input_or_paint() {
    let screens=[Screen::Start,Screen::ImportSave,Screen::Summary,Screen::Manual,
        Screen::Game,Screen::Replays,Screen::ReplayGame,Screen::Scores,Screen::Settings];
    for away in screens {
        let mut visits=Visits::default();let mut selection=Selection::default();
        let mut seed=String::new();let mut notice=String::new();let calls=Cell::new(0);
        assert!(visit(&mut visits,Screen::Menu,false,&mut selection,&mut seed,&mut notice,&calls));
        assert_eq!(calls.get(),1);
        for expected in 2..=5 {
            assert!(!visit(&mut visits,away,false,&mut selection,&mut seed,&mut notice,&calls));
            assert!(!visit(&mut visits,away,false,&mut selection,&mut seed,&mut notice,&calls));
            assert!(visit(&mut visits,Screen::Menu,false,&mut selection,&mut seed,&mut notice,&calls));
            let chosen=selection.gods;
            for _ in 0..12 {assert!(!visit(&mut visits,Screen::Menu,false,&mut selection,&mut seed,&mut notice,&calls));}
            assert_eq!(calls.get(),expected);assert_eq!(selection.gods,chosen);
        }
    }
}

#[test]
fn menu_visits_preserve_edits_picker_cancellation_and_repaints_until_leaving() {
    let mut visits=Visits::default();let mut selection=Selection::default();
    let mut seed="world:é".to_owned();let mut notice=String::new();let calls=Cell::new(0);
    visit(&mut visits,Screen::Menu,false,&mut selection,&mut seed,&mut notice,&calls);
    let replacement=(0..12).find(|id|!selection.gods.contains(id)).unwrap();
    selection.open(0);assert!(selection.choose(replacement,&mut seed));
    let chosen=selection.gods;let edited=seed.clone();
    selection.open(1);
    for _ in 0..20 {
        selection.sync_seed(&seed);
        assert!(!visit(&mut visits,Screen::Menu,false,&mut selection,&mut seed,&mut notice,&calls));
    }
    assert!(selection.active());selection.close();
    assert!(!visit(&mut visits,Screen::Menu,false,&mut selection,&mut seed,&mut notice,&calls));
    assert_eq!(selection.gods,chosen);assert_eq!(seed,edited);assert_eq!(calls.get(),1);
    assert_eq!(selection.prepared_seed(&seed,||panic!()).unwrap(),edited);
}

#[test]
fn menu_visits_preserve_initial_explicit_seed_but_rewrite_later_entry_prefix() {
    let mut visits=Visits::default();let mut selection=Selection::default();
    let mut seed=" 11-09-03:Exact:Suffix-é ".to_owned();selection.sync_seed(&seed);
    let original=seed.clone();let mut notice=String::new();let calls=Cell::new(0);
    assert!(!visit(&mut visits,Screen::Menu,false,&mut selection,&mut seed,&mut notice,&calls));
    assert_eq!(seed,original);assert_eq!(selection.gods,[11,9,3]);assert_eq!(calls.get(),0);
    assert_eq!(selection.prepared_seed(&seed,||panic!()).unwrap(),original.trim());
    visit(&mut visits,Screen::Settings,false,&mut selection,&mut seed,&mut notice,&calls);
    assert!(visit(&mut visits,Screen::Menu,false,&mut selection,&mut seed,&mut notice,&calls));
    assert!(seed.ends_with(":Exact:Suffix-é"));assert_ne!(selection.gods,[11,9,3]);
    let sampled=selection.gods;selection.sync_seed(&seed);assert_eq!(selection.gods,sampled);
    // A CLI startscreen/replay/manual first observation does not get the exact
    // initial Menu exception when it later returns to the menu.
    for start in [Screen::Start,Screen::ReplayGame,Screen::Manual] {
        let mut visits=Visits::default();let calls=Cell::new(0);
        visit(&mut visits,start,false,&mut selection,&mut seed,&mut notice,&calls);
        assert!(visit(&mut visits,Screen::Menu,false,&mut selection,&mut seed,&mut notice,&calls));
        assert_eq!(calls.get(),1);
    }
}

#[test]
fn menu_visits_preparation_cancel_error_and_instant_poll_are_single_entries() {
    let mut visits=Visits::default();let mut selection=Selection::default();
    let mut seed=String::new();let mut notice=String::new();let calls=Cell::new(0);
    visit(&mut visits,Screen::Menu,false,&mut selection,&mut seed,&mut notice,&calls);
    // Both cancellation and errors release the same busy gate. Repeated polls
    // while the cancelled worker drains do not reroll or admit another launch.
    for busy_polls in [0,1,5] {
        assert!(!visit(&mut visits,Screen::Menu,true,&mut selection,&mut seed,&mut notice,&calls));
        let before=calls.get();
        for _ in 0..busy_polls {visit(&mut visits,Screen::Menu,true,&mut selection,&mut seed,&mut notice,&calls);}
        assert_eq!(calls.get(),before);
        assert!(visit(&mut visits,Screen::Menu,false,&mut selection,&mut seed,&mut notice,&calls));
        assert_eq!(calls.get(),before+1);
        assert!(!visit(&mut visits,Screen::Menu,false,&mut selection,&mut seed,&mut notice,&calls));
    }
    // Success goes to Game instead of a spurious menu visit.
    visit(&mut visits,Screen::Menu,true,&mut selection,&mut seed,&mut notice,&calls);
    let before=calls.get();
    visit(&mut visits,Screen::Game,false,&mut selection,&mut seed,&mut notice,&calls);
    assert_eq!(calls.get(),before);
    // Summary's direct new-descent hook observes Menu before consuming choices.
    visit(&mut visits,Screen::Summary,false,&mut selection,&mut seed,&mut notice,&calls);
    assert!(visit(&mut visits,Screen::Menu,false,&mut selection,&mut seed,&mut notice,&calls));
    let launch=selection.prepared_seed("",||"00-01-02:fresh".into()).unwrap();
    let shell=corpse_core::startup::new_run_shell(&launch,avatar_menu::MODE,false).unwrap();
    assert_eq!(shell.gods.iter().map(|id|*id as usize).collect::<Vec<_>>(),selection.gods);
    visit(&mut visits,Screen::Menu,true,&mut selection,&mut seed,&mut notice,&calls);
    assert_eq!(calls.get(),before+1);
}

#[test]
fn menu_visits_random_failure_is_visible_and_never_retries_each_frame() {
    let mut visits=Visits::default();let mut selection=Selection::default();
    let original=selection.clone();let mut seed=String::new();let mut notice=String::new();
    assert!(visits.observe_with(Screen::Menu,false,&mut selection,&mut seed,&mut notice,||Err("test entropy unavailable".into())));
    assert_eq!(selection,original);assert!(notice.contains("test entropy unavailable"));
    assert!(!visits.observe_with(Screen::Menu,false,&mut selection,&mut seed,&mut notice,||panic!("no per-frame entropy retry")));
}

#[test]
fn menu_visits_maximum_seed_keeps_suffix_and_blocks_oversized_launch() {
    let suffix="x".repeat(174);let mut seed=format!("0-1-2:{suffix}");
    assert_eq!(seed.len(),180);
    let mut visits=Visits::default();let mut selection=Selection::default();
    selection.sync_seed(&seed);let mut notice=String::new();let calls=Cell::new(0);
    visit(&mut visits,Screen::Menu,false,&mut selection,&mut seed,&mut notice,&calls);
    visit(&mut visits,Screen::Settings,false,&mut selection,&mut seed,&mut notice,&calls);
    assert!(visit(&mut visits,Screen::Menu,false,&mut selection,&mut seed,&mut notice,&calls));
    assert_eq!(seed.len(),183);assert_eq!(seed.split_once(':').unwrap().1,suffix);
    assert!(notice.contains("Shorten the seed"));
    assert!(selection.prepared_seed(&seed,||panic!()).is_err());
    for _ in 0..3 {seed.pop();}
    selection.sync_seed(&seed);
    assert!(selection.prepared_seed(&seed,||panic!()).is_ok());
}

#[test]
fn menu_visits_leave_saved_runs_replay_commands_and_simulation_rng_independent() {
    use corpse_core::{model::Command,temporal::Engine,replay::{ReplayRecord,ReplaySession}};
    let initial:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let mut baseline=Engine::new(initial.clone()).unwrap();
    let mut actual=Engine::load(&baseline.save().unwrap()).unwrap();
    let mut visits=Visits::default();let mut selection=Selection::default();
    let mut seed=String::new();let mut notice=String::new();let calls=Cell::new(0);
    visit(&mut visits,Screen::Menu,false,&mut selection,&mut seed,&mut notice,&calls);
    for _ in 0..3 {
        visit(&mut visits,Screen::Game,false,&mut selection,&mut seed,&mut notice,&calls);
        baseline.dispatch(&Command::Wait).unwrap();actual.dispatch(&Command::Wait).unwrap();
        let saved=actual.save().unwrap();
        visit(&mut visits,Screen::Menu,false,&mut selection,&mut seed,&mut notice,&calls);
        visit(&mut visits,Screen::ReplayGame,false,&mut selection,&mut seed,&mut notice,&calls);
        visit(&mut visits,Screen::Menu,false,&mut selection,&mut seed,&mut notice,&calls);
        assert_eq!(actual.save().unwrap(),saved);
        assert_eq!(actual.save().unwrap(),baseline.save().unwrap());
        assert_eq!(actual.run.rng,baseline.run.rng);
        assert_eq!(actual.run.seed,initial.seed);assert_eq!(actual.run.gods,initial.gods);
        let record=ReplayRecord::try_from_run(&actual.run).unwrap();
        let mut replay=ReplaySession::from_initial(initial.clone(),record).unwrap();
        while replay.step().unwrap() {}
        assert_eq!(replay.engine.save().unwrap(),actual.save().unwrap());
        replay.restart().unwrap();while replay.step().unwrap() {}
        assert_eq!(replay.engine.save().unwrap(),actual.save().unwrap());
    }
}
