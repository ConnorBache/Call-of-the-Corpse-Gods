use corpse_core::{flavor_placement as f, ordinary_roster as o, population as p, generation as g};
use std::collections::BTreeSet;
#[test]
fn final_windows_cover_every_leader_and_pair_without_legacy_changes() {
    assert_eq!(f::placements().len(),57);
    let mut ids=BTreeSet::new();
    for s in f::placements() {
        assert!(ids.insert(&s.id));
        assert!(s.first>=1 && s.last<=40 && s.first<=s.last);
        assert!((s.first as f64..=s.last as f64).contains(&s.reference_depth));
        assert!(p::catalog().actors.contains_key(&s.id));
    }
    for s in o::species() { assert!(f::bounds(&s.id).is_some()); }
    for (leader,follower) in &p::catalog().pairs { assert_eq!(f::bounds(leader),f::bounds(follower)); }
    assert_eq!(f::bounds("witch"),Some((27,36)));
    assert_eq!(f::bounds("minotaur"),Some((33,40)));
    assert_eq!(f::bounds("pale:Greater Jackal"),None);
    assert_eq!(f::bounds("flesh-administrator"),None);
    let mut seen=BTreeSet::new();
    for d in 1..=40 { for water in [false,true] {
        for rev in [30.0,31.0] {
            assert_eq!(serde_json::to_value(o::candidates_revision(d,water,rev).unwrap()).unwrap(),
                serde_json::to_value(o::candidates(d,water).unwrap()).unwrap());
        }
        for c in o::candidates_revision(d,water,32.0).unwrap() {
            assert!(c.weight>0.0 && c.weight<=1.0);
            assert_eq!(c.blessings,p::heart_blessings(&c.kinds).unwrap());
            assert!(c.kinds.iter().filter(|k|k.as_str()=="banshee").count()<=1);
            if c.kinds.len()==2 { assert_eq!(p::catalog().pairs.get(&c.kinds[0]),Some(&c.kinds[1])); }
            if d==1 { assert!(["low","medium"].contains(&c.danger.as_str())); }
            for k in &c.kinds {
                assert!(f::eligible(k,d),"{k} at {d}");
                if !water { assert_ne!(p::catalog().aquatic.get(k),Some(&true)); }
                seen.insert(k.clone());
            }
        }
    }}
    assert_eq!(seen,ids.into_iter().cloned().collect());
    for (d,r) in [(0,32.0),(41,32.0),(1,47.0)] { assert!(o::candidates_revision(d,true,r).is_err()); }
    assert!(o::candidates_revision(1,true,36.0).is_ok());
    assert!(o::candidates_revision(1,true,37.0).is_ok());
    assert!(o::candidates_revision(1,true,38.0).is_ok());
    assert!(o::candidates_revision(1,true,39.0).is_ok());
    assert!(o::candidates_revision(1,true,40.0).is_ok());
}
#[test]
fn corpse_revival_cannot_introduce_out_of_band_witches() {
    for d in 1..=40 { assert!(f::eligible(f::corpse_kind(d),d),"corpse{} at{d}",f::corpse_kind(d)); }
    assert_ne!(f::corpse_kind(15),"witch");
    assert_eq!(f::corpse_kind(33),"witch");
    assert_eq!(p::catalog().corpses[14],"witch");
}
#[test]
fn authored_setup_windows_follow_members_and_keep_disabled_recipes_disabled() {
    for d in g::setup_definitions() {
        for depth in 1..=40 {
            assert_eq!(g::setup_eligible_at_depth_revision(d,depth,31.0).unwrap(),d.eligible[depth as usize-1]);
            if g::setup_eligible_at_depth_revision(d,depth,32.0).unwrap() {
                assert!(d.eligible.iter().any(|v|*v),"enabled unauthored setup{}",d.id);
                for k in &d.kinds { if f::bounds(k).is_some() { assert!(f::eligible(k,depth),"setup{} {k} depth{depth}",d.id); } }
            }
        }
    }
}
#[test]
fn all_depths_keep_deterministic_exact_reward_feasibility() {
    for d in 1..=40 { for water in [false,true] { for sample in 0..4 {
        let seed=format!("flavor-r32-{sample}");
        let budget=g::ordinary_blessing_budget(g::BudgetSeed::Text(&seed),d).unwrap() as i32;
        let plan=p::plan_budgeted_hordes_revision(&seed,d,budget,0,water,&[],32.0).unwrap();
        let again=p::plan_budgeted_hordes_revision(&seed,d,budget,0,water,&[],32.0).unwrap();
        assert_eq!(serde_json::to_value(&plan).unwrap(),serde_json::to_value(&again).unwrap());
        let kinds:Vec<_>=plan.hordes.iter().flat_map(|h|h.kinds.clone()).collect();
        assert_eq!(p::heart_blessings(&kinds).unwrap(),budget);
        assert!((1.5..=2.5).contains(&(kinds.len() as f64/plan.hordes.len() as f64)));
        for k in kinds { assert!(f::eligible(&k,d)); }
    }}}
}
#[test]
fn dynamic_water_watcher_stencil_never_leaks_frozen_depth_species() {
    for depth in 1..=40 {
        let kind=f::water_watcher_kind(depth,None).unwrap();
        assert!(f::eligible(&kind,depth));
        assert!(!p::catalog().pairs.contains_key(&kind));
        let mut floor=corpse_core::Floor {depth:depth as f64,domain:"Cave".into(),tiles:vec!["floor".into();169],lights:vec![0.;169],seen:vec![false;169],..Default::default()};
        g::stamp_setup(&mut floor,108,6,6,std::slice::from_ref(&kind)).unwrap();
        let spawns=floor.setup_spawns.as_ref().unwrap();
        assert_eq!(spawns.len(),1);
        assert_eq!(spawns[0].kind,kind);
    }
    assert_ne!(f::water_watcher_kind(15,None).unwrap(),"witch");
    assert_ne!(f::water_watcher_kind(37,None).unwrap(),"yeti");
    assert_eq!(p::low_end_water_setup_kind(15,None).unwrap().0,"witch");
    assert_eq!(p::low_end_water_setup_kind(37,None).unwrap().0,"yeti");
}
#[test]
fn generated_live_sources_obey_relocated_windows_regression_seed_zero() {
    let mut run=corpse_core::startup::new_run_shell_revision("heart-audit-000000","Unblessed",false,32.).unwrap();
    corpse_core::startup::ensure_floors(&mut run,40).unwrap();
    for depth in 1..=40 {
        for a in run.actors_at(depth as f64) {
            let kind=a.creature_kind.as_ref().cloned().unwrap_or_else(|| {
                p::catalog().actors.iter().find(|(_,old)|old.name==a.name).map(|(k,_)|k.clone()).unwrap_or_default()
            });
            if f::bounds(&kind).is_some() { assert!(f::eligible(&kind,depth),"{kind} at{depth}; dead{}",a.dead.value_or(false)); }
        }
    }
}
