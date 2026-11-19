//! Bounded, detached hover differential benchmark. No SDL, disk writes to profiles,
//! command dispatch, upload, present, or frame-rate claim. Lead enables this module.
use crate::{inspection_ui, non_euclidean_ui, observe, render::{Painter, View}, travel_ui};
use corpse_core::Run;
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug)]
enum Arm { Fresh, Cached }
impl Arm { fn name(self) -> &'static str { match self { Self::Fresh => "fresh", Self::Cached => "cached" } } }

/// Exactly one inspection, one unchanged route, and pointer-only projection.
/// Existing Report/route disposal is included, but no global observation is added.
fn changed_hover(run: &Run, view: &mut View, report: &mut Option<inspection_ui::Report>, xy: (usize,usize), arm: Arm) {
    let (x,y)=xy;
    *report=Some(match arm {
        Arm::Fresh => inspection_ui::observe_tile(run,x,y),
        Arm::Cached => inspection_ui::observe_tile_with_terrain(run,x,y,view.terrain_snapshot.as_deref().expect("same-generation terrain")),
    });
    let path=travel_ui::path_to(run,x,y,true).iter().map(|s|(s.x,s.y)).collect();
    match arm {
        Arm::Fresh => { view.path=path;view.hover=Some(xy);view.possible_exits=non_euclidean_ui::possible_exit_marks(run,x,y);view.target_path.clear(); }
        Arm::Cached => crate::sync_pointer_view(view,run,Some(xy),path,None),
    }
}

fn assert_projection(a: &View,b: &View) {
    assert_eq!(a.path,b.path);assert_eq!(a.hover,b.hover);
    assert_eq!(a.target_path,b.target_path);assert_eq!(a.possible_exits,b.possible_exits);
}
fn prove(run: &Run, xy: (usize,usize)) {
    let before=serde_json::to_vec(run).unwrap();
    let (mut a,mut b)=(observe::observe(run),observe::observe(run));
    let (mut ar,mut br)=(None,None);
    changed_hover(run,&mut a,&mut ar,xy,Arm::Fresh);
    changed_hover(run,&mut b,&mut br,xy,Arm::Cached);
    assert_eq!(ar,br,"complete Report including creature/hallucination");assert_projection(&a,&b);
    let mut painter=Painter::new();painter.resize(1280,800);
    painter.game_at(&a,"",false,1000.);
    inspection_ui::paint_hover(ar.as_ref().unwrap(),&mut painter);
    let pixels=painter.pixels.clone();
    painter.game_at(&b,"",false,1000.);
    inspection_ui::paint_hover(br.as_ref().unwrap(),&mut painter);
    assert_eq!(pixels,painter.pixels,"fixed-clock game and actual hover paint");
    assert_eq!(before,serde_json::to_vec(run).unwrap(),"no gameplay/RNG mutation");
}

fn summary(raw: Vec<u64>) -> Value {
    assert!(!raw.is_empty());let mut sorted=raw.clone();sorted.sort_unstable();let n=sorted.len();
    let rank=|p:usize|sorted[(n*p).div_ceil(100)-1];
    let median=if n%2==0 {(sorted[n/2-1] as f64+sorted[n/2] as f64)/2.} else {sorted[n/2] as f64};
    json!({"n":n,"median_ns":median,"p95_ns":rank(95),"p99_ns":rank(99),"min_ns":sorted[0],"max_ns":sorted[n-1],"raw_ns":raw})
}

#[test]
fn hover_summary_nearest_rank_preserves_raw() {
    let s=summary(vec![4,1,3,2]);assert_eq!(s["median_ns"],2.5);assert_eq!(s["p95_ns"],4);assert_eq!(s["p99_ns"],4);assert_eq!(s["raw_ns"],json!([4,1,3,2]));
}

/// Small authored topology proof, no dependence on external live save.
#[test]
fn cached_hover_non_euclidean_projection_report_and_pixels() {
    let r=non_euclidean_ui::tests::scene();prove(&r,(11,10));
}

#[cfg(feature="runtime-timing")]
mod measured {
    use super::*;
    use corpse_core::timing;
    use std::{hint::black_box,path::PathBuf,time::Instant};

    /// Alternating paired operation timings. Setup, proof, warmup and collectors
    /// are outside headline intervals. Both arms keep live prior output to replace.
    fn case(run: &Run,label: &str,xy: (usize,usize),n:usize) -> Value {
        timing::set_enabled(false);prove(run,xy);
        let before=serde_json::to_vec(run).unwrap();
        let (mut a,mut b)=(observe::observe(run),observe::observe(run));let(mut ar,mut br)=(None,None);
        for _ in 0..3 {changed_hover(run,&mut a,&mut ar,xy,Arm::Fresh);changed_hover(run,&mut b,&mut br,xy,Arm::Cached);}
        let(mut fresh,mut cached)=(Vec::with_capacity(n),Vec::with_capacity(n));
        for i in 0..n {
            for arm in if i%2==0 {[Arm::Fresh,Arm::Cached]}else{[Arm::Cached,Arm::Fresh]} {
                let(view,report)=match arm {Arm::Fresh=>(&mut a,&mut ar),Arm::Cached=>(&mut b,&mut br)};
                // Simulate arrival from another cell; this assignment is not timed.
                view.hover=None;
                let start=Instant::now();changed_hover(run,view,report,xy,arm);black_box((&*view,&*report));
                let ns=start.elapsed().as_nanos().min(u64::MAX as u128) as u64;
                match arm {Arm::Fresh=>fresh.push(ns),Arm::Cached=>cached.push(ns)}
            }
        }
        assert_eq!(ar,br);assert_projection(&a,&b);assert_eq!(before,serde_json::to_vec(run).unwrap());
        let mut diagnostics=Vec::new();
        for arm in [Arm::Fresh,Arm::Cached] {
            let(view,report)=match arm {Arm::Fresh=>(&mut a,&mut ar),Arm::Cached=>(&mut b,&mut br)};
            timing::reset();timing::set_enabled(true);changed_hover(run,view,report,xy,arm);timing::set_enabled(false);
            let spans=timing::take_report();let counts=timing::take_counts();
            assert_eq!(spans.get("desktop.inspect").map(|s|s.calls),Some(1));
            assert_eq!(spans.get("desktop.route").map(|s|s.calls),Some(1));
            assert!(!spans.contains_key("desktop.observe"),"isolated hover must not rebuild world");
            diagnostics.push(json!({"arm":arm.name(),"calls":1,"spans":spans,"counts":counts}));
        }
        json!({"case":label,"target":xy,"fresh":summary(fresh),"cached":summary(cached),"diagnostic_collector_on_separate_from_headline":diagnostics,"report_projection_pixels_state_equal":true})
    }

    /// The same-target branch performs no projection; report separately, no claim
    /// of cached speedup on a branch that already did no work in shipped source.
    fn same_target(run:&Run,xy:(usize,usize)) -> Value {
        let mut view=observe::observe(run);let mut report=None;
        changed_hover(run,&mut view,&mut report,xy,Arm::Cached);
        timing::reset();timing::set_enabled(true);
        if view.hover!=Some(xy) {changed_hover(run,&mut view,&mut report,xy,Arm::Cached);}
        timing::set_enabled(false);let spans=timing::take_report();assert!(spans.is_empty());
        json!({"case":"same_target_noop","target":xy,"spans":spans,"note":"Existing guard, not a speedup; no full SDL event pump measured"})
    }

    /// Explicit dirty-world fallback: discard stale observation generation before
    /// reusing terrain. This is a correctness scenario, not an isolated-hover time.
    fn dirty_world(run:&Run,xy:(usize,usize)) -> Value {
        let stale=observe::observe(run);let mut changed=run.clone();changed.tick+=1.;
        changed.player.conditions.insert("allSeeingReveal".into(),1.);
        changed.player.conditions.insert("allSeeingDepth".into(),changed.depth);
        let before=serde_json::to_vec(&changed).unwrap();
        let fresh_report=inspection_ui::observe_tile(&changed,xy.0,xy.1);
        let mut fresh_view=observe::observe(&changed);let mut cached_report=None;
        assert!(!std::sync::Arc::ptr_eq(stale.terrain_snapshot.as_ref().unwrap(),fresh_view.terrain_snapshot.as_ref().unwrap()));
        changed_hover(&changed,&mut fresh_view,&mut cached_report,xy,Arm::Cached);
        assert_eq!(Some(fresh_report),cached_report);assert_eq!(before,serde_json::to_vec(&changed).unwrap());prove(&changed,xy);
        json!({"case":"dirty_world_coincident_correctness_only","stale_snapshot_not_reused":true,"note":"Models dirty fallback plus refreshed world; production event-loop guard tested separately; excluded from headline hover timing"})
    }

    #[test]
    #[ignore="lead-controlled release benchmark; requires normalized real tick-48 input and explicit new output path"]
    fn real_tick48_alternating_hover_reuse_benchmark() {
        let input=PathBuf::from(std::env::var_os("CORPSE_HOVER_INPUT").expect("normalized roundtrip.json path"));
        let output=PathBuf::from(std::env::var_os("CORPSE_HOVER_OUTPUT").expect("new output JSON path"));assert!(!output.exists(),"preserve prior evidence");
        let n:usize=std::env::var("CORPSE_HOVER_SAMPLES").unwrap_or_else(|_|"30".into()).parse().unwrap();assert!((30..=1000).contains(&n));
        timing::set_enabled(false);
        let raw=std::fs::read_to_string(&input).unwrap();let(engine,_)=crate::session_store::decode(&raw).unwrap();let run=&engine.run;
        assert_eq!(run.tick,48.,"Use real normalized tick-48 fixture, not generated startup");
        let saved=engine.save().unwrap();let view=observe::observe(run);let terrain=view.terrain_snapshot.as_deref().unwrap();let w=terrain.width;
        let player=(run.player.x as usize,run.player.y as usize);
        let mut targets=Vec::new();let mut unavailable=Vec::new();
        for (label,cell) in [
            ("ordinary_known",terrain.cells.iter().find(|o|o.tile.as_deref()==Some("floor")&&(o.index%w,o.index/w)!=player)),
            ("unknown",terrain.cells.iter().find(|o|!o.current&&!o.remembered&&o.tile.is_none())),
            ("impassable_known_wall",terrain.cells.iter().find(|o|o.tile.as_deref()==Some("wall"))),
        ] {if let Some(o)=cell {targets.push((label,(o.index%w,o.index/w)));}else{unavailable.push(label);}}
        if let Some(actor)=run.actors_at(run.depth).iter().find(|a|!a.dead.value_or(false)&&corpse_core::vision::visible_actor(run,a)) {targets.push(("visible_monster",(actor.x as usize,actor.y as usize)));}else{unavailable.push("visible_monster");}
        assert!(!targets.is_empty());let mut rows=Vec::new();
        for (label,xy) in &targets {rows.push(case(run,label,*xy,n));}
        let topology=non_euclidean_ui::tests::scene();rows.push(case(&topology,"authored_non_euclidean_endpoint",(11,10),n));
        if unavailable.contains(&"visible_monster") {
            let mut monster:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
            monster.player.conditions.insert("allSeeingReveal".into(),1.);
            monster.player.conditions.insert("allSeeingDepth".into(),monster.depth);
            let actor=monster.actors_at(monster.depth).iter().find(|a|!a.dead.value_or(false)&&corpse_core::vision::visible_actor(&monster,a)).expect("authored visible actor");
            let xy=(actor.x as usize,actor.y as usize);
            assert!(inspection_ui::observe_tile(&monster,xy.0,xy.1).creature.is_some());
            rows.push(case(&monster,"authored_visible_monster_fallback",xy,n));
        }
        let noop=same_target(run,targets[0].1);let dirty=dirty_world(run,targets[0].1);assert_eq!(saved,engine.save().unwrap());
        let report=json!({"schema":"corpse-hover-reuse-v1","input":input,"tick":run.tick,"seed":run.seed,"samples_per_arm_per_case":n,"warmup_per_arm":3,"collector_enabled_headline":false,"order":"AB, BA alternating pairs", "coverage":"One inspection + one unchanged route + pointer-only projection. Report, state and fixed-clock 1280x800 game/hover pixels proven outside timings. No SDL event loop/upload/present/vsync/full-frame claim.","baseline":"Current fresh wrapper versus supplied same-generation terrain. Ordinary no-portal fresh wrapper equals shipped flow; endpoint fresh wrapper already contains local alias observation reuse, so endpoint result isolates cache reuse only, not full original before/after.","legacy_warning":"Older ordinary_hover_event_plus_dirty_refresh row performed two inspections, two routes and full observe; it is not isolated shipped hover and is not a baseline for this comparison.","fixture_note":"Caller must supply normalized roundtrip.json from live-probe; original v2 import/replay ordering issue is separate and normalization is outside timing.","unavailable_real_fixture_targets":unavailable,"rows":rows,"same_target":noop,"dirty_world":dirty,"game_state_unchanged":true});
        std::fs::write(&output,serde_json::to_vec_pretty(&report).unwrap()).unwrap();println!("HOVER_REUSE_REPORT {}",output.display());
    }
}
