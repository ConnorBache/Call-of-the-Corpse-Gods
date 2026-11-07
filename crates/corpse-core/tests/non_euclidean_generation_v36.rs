use corpse_core::{Actor, Field, Floor, GroundItem, Run, generation, non_euclidean_generation as ne, portals_generation, startup, storage};
use ne::{NonEuclideanGroup as Group, NonEuclideanPocket as Pocket};
use std::{collections::{HashSet, VecDeque}, sync::Arc};
fn room(depth: u32) -> Floor {
    let mut f = Floor { depth: depth as f64, domain: "Dungeon".into(), tiles: vec!["floor".into(); 2291], lights: vec![0.;2291], seen: vec![false;2291], ..Default::default() };
    for at in 0..2291 { if at%79==0 || at%79==78 || at/79==0 || at/79==28 { f.tiles[at]="wall".into(); } }
    f.tiles[80]="up".into(); f.tiles[27*79+77]="down".into(); f
}
fn independent_walk_graph(f: &Floor, start: usize) -> HashSet<usize> { independent_walk_graph_with_portal_avoidance(f,start,false) }
fn independent_walk_graph_with_portal_avoidance(f: &Floor, start: usize, avoid_portals: bool) -> HashSet<usize> {
    let forbidden:HashSet<_>=f.portals.as_ref().into_iter().flatten().flat_map(|p|[p.a,p.b]).filter(|_|avoid_portals).collect();
    let w=f.width(); let mut seen=HashSet::from([start]); let mut q=VecDeque::from([start]);
    while let Some(at)=q.pop_front() {
        for (dx,dy) in [(-1,0),(1,0),(0,-1),(0,1)] {
            let (x,y)=((at%w) as i32+dx,(at/w) as i32+dy);
            if x<0 || y<0 || x>=w as i32 || y>=f.height() as i32 { continue; }
            let entry=y as usize*w+x as usize;
            if forbidden.contains(&entry) || ["wall","tree","chasm","pit"].contains(&f.tiles[entry].as_str()) { continue; }
            let mut arrivals=vec![entry];
            if let Some(p)=f.portals.as_ref().into_iter().flatten().find(|p|p.a==entry||p.b==entry) { arrivals=vec![if p.a==entry {p.b}else{p.a}]; }
            for g in f.non_euclidean.as_ref().into_iter().flatten().flat_map(|p|&p.groups) {
                if g.destination==entry {arrivals=g.sources.clone();break;}
                if g.sources.contains(&entry) {arrivals=vec![g.destination];break;}
            }
            for next in arrivals { if seen.insert(next) {q.push_back(next);} }
        }
    }
    seen
}
#[test]
fn depth_revision_determinism_idempotence_and_unchanged_terrain() {
    for depth in 1..=40 {
        let mut f=room(depth); let original=f.clone();
        ne::install("repeat",&mut f,&[],&[],35.); assert_eq!(f,original);
        ne::install("repeat",&mut f,&[],&[],36.);
        if depth<20 { assert_eq!(f,original); continue; }
        assert_eq!(f.non_euclidean.as_ref().unwrap().len(),1,"depth{depth}");
        assert!(ne::validate_floor(&f)); assert!(ne::traversable(&f));
        for p in f.non_euclidean.as_ref().unwrap() { assert!(ne::assignment_follows_rules(&f,p)); }
        let once=f.clone(); ne::install("different",&mut f,&[],&[],36.); assert_eq!(f,once);
        let mut repeated=original.clone();ne::install("repeat",&mut repeated,&[],&[],36.);assert_eq!(repeated,once);
        f.non_euclidean=Field::Missing; assert_eq!(f,original);
    }
}
#[test]
fn cell_partition_and_all_stochastic_outcomes_have_paths_out() {
    let mut triples=0;
    for seed in 0..64 {
        let mut f=room(20);portals_generation::install(&seed.to_string(),&mut f,&[],&[],35.);
        ne::install(&seed.to_string(),&mut f,&[],&[],36.);
        assert_eq!(f.non_euclidean.as_ref().unwrap().len(),1);
        let reachable=independent_walk_graph(&f,80);
        let avoiding=independent_walk_graph_with_portal_avoidance(&f,80,true);
        assert_eq!(avoiding.len(),77*27-2,"all ordinary walking cells except the optional pair remain reachable without portals");
        for pocket in f.non_euclidean.as_ref().unwrap() {
            assert!(ne::assignment_follows_rules(&f,pocket));
            let mut partition=HashSet::new();
            for group in &pocket.groups {
                triples+=usize::from(group.sources.len()==2);
                assert!(partition.insert(group.destination));
                for &source in &group.sources { assert!(partition.insert(source)); }
                for at in group.sources.iter().copied().chain([group.destination]) {
                    assert!(reachable.contains(&at),"seed{seed}, cannot enter{at}");
                    let optional=independent_walk_graph_with_portal_avoidance(&f,at,true);
                    assert!(optional.contains(&80)&&optional.contains(&(27*79+77)),"every stochastic exit must reach both stairs without using portals");
                    let back=independent_walk_graph(&f,at);
                    assert!(back.contains(&80),"seed{seed}, trapped at{at}");
                    assert!(back.contains(&(27*79+77)),"seed{seed}, cannot leave to downstairs{at}");
                }
            }
            assert_eq!(partition,pocket.cells.iter().copied().collect());
        }
    }
    assert!(triples>0,"two-source groups must survive safety rejection too");
}
#[test]
fn stairs_fixtures_portals_population_and_loot_are_never_rewritten() {
    let mut f=room(20); f.permanent_fire=vec![200.].into(); f.grates=vec![201.].into();
    let actors=vec![Actor{x:10.,y:10.,..Default::default()}];
    let ground=vec![GroundItem{x:11.,y:10.,..Default::default()}];
    portals_generation::install("protected",&mut f,&actors,&ground,35.);
    let original=f.clone();
    for seed in 0..32 {
        let mut next=f.clone();ne::install(&seed.to_string(),&mut next,&actors,&ground,36.);
        let cells:HashSet<_>=next.non_euclidean.as_ref().unwrap().iter().flat_map(|p|p.cells.iter().copied()).collect();
        for at in [80,27*79+77,200,201,10*79+10,10*79+11].into_iter().chain(f.portals.as_ref().unwrap().iter().flat_map(|p|[p.a,p.b])) {assert!(!cells.contains(&at));}
        next.non_euclidean=Field::Missing;assert_eq!(next,original);
    }
}
#[test]
fn all_six_domains_keep_geometry_and_every_accepted_pocket_is_traversable() {
    let mut installed=0;
    for domain in ["Dungeon","Cave","Meadow","Forest","Archipelago","Mountain"] {
        for seed in 0..6 {
            let mut f=generation::generate_floor_revision(20,seed,domain,true,79,None,36.).unwrap();
            let before=f.tiles.clone();portals_generation::install(&format!("{domain}{seed}"),&mut f,&[],&[],36.);
            ne::install(&format!("{domain}{seed}"),&mut f,&[],&[],36.);
            assert_eq!(f.tiles,before);assert!(ne::validate_floor(&f));
            if !f.non_euclidean.as_ref().unwrap().is_empty() {installed+=1;assert!(ne::traversable(&f),"{domain}:{seed}");}
        }
    }
    assert!(installed>=24,"unexpectedly sparse sample: {installed}/36");
}
#[test]
fn unsuitable_floor_omits_instead_of_altering_graph_or_terrain() {
    let mut f=room(20);for t in &mut f.tiles {if t=="floor" {*t="water".into();}}
    let old=f.clone(); ne::install("unsuitable",&mut f,&[],&[],36.);
    assert!(f.non_euclidean.as_ref().unwrap().is_empty());f.non_euclidean=Field::Missing;assert_eq!(f,old);
}
#[test]
fn physical_contiguity_and_one_good_stochastic_branch_do_not_prove_safety() {
    let mut f=room(20);f.tiles.fill("wall".into());
    let at=|x:usize|10*79+x;
    for x in 10..=19 {f.tiles[at(x)]="floor".into();}
    f.tiles[at(10)]="up".into(); f.tiles[at(19)]="down".into();
    f.non_euclidean=vec![Pocket{cells:(11..=18).map(at).collect(),groups:vec![
        Group{sources:vec![at(13),at(17)],destination:at(14)},
        Group{sources:vec![at(18),at(15)],destination:at(16)},
        Group{sources:vec![at(11)],destination:at(12)},
    ]}].into();
    assert!(ne::validate_floor(&f));
    assert!(!independent_walk_graph(&f,at(18)).contains(&at(10)));
    assert!(independent_walk_graph(&f,at(15)).contains(&at(10)));
    assert!(!ne::traversable(&f),"one random outcome is a closed trap");
}
#[test]
fn save_roundtrip_structural_boundaries_and_dream_validation() {
    let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=36.0.into();
    while r.floors.len()<20 {let d=r.floors.len()+1;r.floors.push(Arc::new(room(d as u32)));r.actors.insert(d.to_string(),Arc::new(vec![]));r.ground.insert(d.to_string(),Arc::new(vec![]));}
    ne::install("save",Arc::make_mut(&mut r.floors[19]),&[],&[],36.);
    let saved=storage::serialize(&r).unwrap();let loaded=storage::deserialize(&saved).unwrap();assert_eq!(loaded.floors[19].non_euclidean,r.floors[19].non_euclidean);
    let mut legacy=r.clone();legacy.simulation_revision=35.0.into();assert!(storage::validate(&legacy).is_err());
    for invalid in [serde_json::Value::Null,serde_json::json!([{"cells":[80,81],"groups":[{"sources":[80],"destination":80}]}]),serde_json::json!([{"cells":[80,999999],"groups":[{"sources":[80],"destination":999999}]}]),serde_json::json!([{"cells":[80,82],"groups":[{"sources":[80],"destination":82}]}])] {
        let mut v:serde_json::Value=serde_json::from_str(&saved).unwrap();v["floors"][19]["nonEuclidean"]=invalid;
        let raw=serde_json::to_string(&v).unwrap();assert!(storage::deserialize(&raw).is_err());assert!(storage::deserialize_compatible(&raw).is_err());
    }
    let mut dream=r.clone();dream.dreamwalking=corpse_core::dreamwalking::State{floors:vec![r.floors[19].clone()],..Default::default()}.into();
    Arc::make_mut(&mut dream.dreamwalking.as_mut().unwrap().floors[0]).non_euclidean.as_mut().unwrap()[0].groups[0].destination=999999;
    assert!(storage::validate(&dream).is_err());
    let floor=Arc::make_mut(&mut r.floors[19]);let at=floor.non_euclidean.as_ref().unwrap()[0].cells[0];floor.tiles[at]="wall".into();
    assert!(ne::validate_floor(floor),"legitimate changed terrain must not invalidate metadata");
}
#[test]
fn startup_adds_only_metadata_without_rebudgeting_or_spending_simulation_rng() {
    let f=generation::generate_floor_revision(20,7,"Dungeon",true,79,None,35.).unwrap();
    let mut old=startup::new_run_shell_revision("0-1-2:ne-budget","Unblessed",false,35.).unwrap();old.depth=20.;old.floors=vec![Arc::new(f)];
    let mut new=old.clone();new.simulation_revision=36.0.into();
    startup::spawn_floor(&mut old,20).unwrap();startup::spawn_floor(&mut new,20).unwrap();
    assert_eq!(old.actors,new.actors);assert_eq!(old.ground,new.ground);assert_eq!(old.rng,new.rng);assert_eq!(old.next_id,new.next_id);
    let mut stripped=new.floor().unwrap().clone();stripped.non_euclidean=Field::Missing;assert_eq!(&stripped,old.floor().unwrap());
}
#[test]
fn sampled_distribution_and_bounded_retry_receipt() {
    let began=std::time::Instant::now();let mut rows=vec![];
    for depth in [20,30,40] { for domain in ["Dungeon","Cave","Meadow","Forest","Archipelago","Mountain"] {
        let mut sums=ne::GenerationReport::default();
        for seed in 0..8 {
            let mut f=generation::generate_floor_revision(depth,seed,domain,true,79,None,36.).unwrap();
            let seed=format!("ne-r36-sweep:{depth}:{domain}:{seed}");
            portals_generation::install(&seed,&mut f,&[],&[],36.);
            let r=ne::install_with_report(&seed,&mut f,&[],&[],36.);
            assert!(r.shape_attempts<=ne::SHAPE_ATTEMPTS);
            assert!(r.assignment_attempts<=ne::SHAPE_ATTEMPTS*ne::ASSIGNMENT_ATTEMPTS);
            assert_eq!(r.assignment_attempts,r.assignment_rejections+r.safety_rejections+r.accepted_pockets);
            assert!(ne::validate_floor(&f));if r.accepted_pockets>0 {assert!(ne::traversable(&f));}
            sums.shape_attempts+=r.shape_attempts;sums.shape_candidates+=r.shape_candidates;
            sums.assignment_attempts+=r.assignment_attempts;sums.assignment_rejections+=r.assignment_rejections;sums.safety_rejections+=r.safety_rejections;
            sums.accepted_pockets+=r.accepted_pockets;sums.accepted_cells+=r.accepted_cells;sums.accepted_groups+=r.accepted_groups;sums.accepted_two_source_groups+=r.accepted_two_source_groups;
        }
        rows.push(serde_json::json!({"depth":depth,"domain":domain,"floors":8,"counts":sums}));
    }}
    println!("GENERATION_R36_RECEIPT {}",serde_json::json!({"profile":"optimized-tests","sample":"generated geometry; no initial actor or ground inputs","floors":144,"elapsedSeconds":began.elapsed().as_secs_f64(),"rows":rows}));
}
#[test]
fn populated_floor_sample_records_actual_exclusions_and_retries() {
    let began=std::time::Instant::now();let mut rows=vec![];let mut inventory=vec![];
    for seed in 0..4 {
        let mut r=startup::create_run_revision(&format!("0-1-2:ne-r36-populated:{seed}"),"Unblessed",true,36.).unwrap();
        startup::ensure_floors(&mut r,40).unwrap();
        assert_eq!(r.simulation_revision.value_or(0.),36.);
        for f in &r.floors {
            let pockets=f.non_euclidean.as_ref().map_or(0,Vec::len);
            if f.depth<20. {assert!(f.non_euclidean.is_missing());}
            else {assert!(f.non_euclidean.as_ref().is_some());assert!(ne::validate_floor(f));if pockets>0 {assert!(ne::traversable(f));}}
            inventory.push(serde_json::json!({"seed":r.seed,"depth":f.depth,"domain":f.domain,"pockets":pockets,"cells":f.non_euclidean.as_ref().into_iter().flatten().map(|p|p.cells.len()).sum::<usize>(),"actors":r.actors_at(f.depth).len(),"ground":r.ground.get(&f.depth.to_string()).map_or(0,|g|g.len())}));
        }
        for depth in [20.,30.,40.] {
            let original=r.floors.iter().find(|f|f.depth==depth).unwrap();
            let mut f=(**original).clone();f.non_euclidean=Field::Missing;
            let actors=r.actors_at(depth);
            let ground=r.ground.get(&depth.to_string()).unwrap();
            let report=ne::install_with_report(&r.seed,&mut f,actors,ground,36.);
            assert_eq!(f.non_euclidean,original.non_euclidean,"same populated floor and inputs must recreate the actual metadata");
            let cells:HashSet<_>=f.non_euclidean.as_ref().unwrap().iter().flat_map(|p|p.cells.iter().copied()).collect();
            for at in actors.iter().map(|a|a.y as usize*f.width()+a.x as usize).chain(ground.iter().map(|g|g.y as usize*f.width()+g.x as usize)) {assert!(!cells.contains(&at));}
            if report.accepted_pockets>0 {assert!(ne::traversable(&f));}
            rows.push(serde_json::json!({"seed":r.seed,"depth":depth,"domain":f.domain,"actors":actors.len(),"ground":ground.len(),"counts":report}));
        }
    }
    println!("POPULATED_GENERATION_R36_RECEIPT {}",serde_json::json!({"profile":"optimized-tests","sample":"fully generated and populated seed runs; selected floors20,30,40","floors":12,"elapsedSeconds":began.elapsed().as_secs_f64(),"rows":rows,"allGeneratedFloors":inventory}));
}
#[test]
fn a_pocket_must_not_turn_previously_optional_portals_into_the_only_escape() {
    let mut f=room(20);f.tiles.fill("wall".into());let at=|x:usize|10*79+x;
    for x in 10..=16 {f.tiles[at(x)]="floor".into();}
    f.tiles[at(10)]="up".into();f.tiles[at(16)]="down".into();
    let (a,b)=(11*79+14,11*79+10);f.tiles[a]="floor".into();f.tiles[b]="floor".into();
    f.portals=vec![portals_generation::PortalPair{a,b,color_index:0}].into();
    assert!(portals_generation::all_endpoints_optional(&f));
    f.non_euclidean=vec![Pocket{cells:(11..=15).map(at).collect(),groups:vec![
        Group{sources:vec![at(13),at(11)],destination:at(14)},
        Group{sources:vec![at(12)],destination:at(15)},
    ]}].into();
    assert!(ne::validate_floor(&f));
    for cell in (10..=16).map(at).chain([a,b]) {
        assert_eq!(independent_walk_graph(&f,cell).len(),9,"full graph is strongly connected through the portals");
    }
    assert!(!independent_walk_graph_with_portal_avoidance(&f,at(14),true).contains(&at(10)));
    assert!(!ne::traversable(&f),"reject the pocket rather than requiring a previously optional portal");
}
