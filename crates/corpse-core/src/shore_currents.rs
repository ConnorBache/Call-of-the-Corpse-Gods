//! Revision40 shore-directed currents. Old revisions keep their frozen inward flow.
use crate::Floor;
/// O(1) neighbor indices, cardinal stable N/E/S/W order; no heap allocation.
fn neighbors(f:&Floor,i:usize)->[Option<usize>;4] {
    let w=f.width();let n=f.tiles.len();
    [i.checked_sub(w), (i%w+1<w).then_some(i+1), (i+w<n).then_some(i+w), (i%w>0).then(||i-1)]
}
/// O(G+D) existing grate/door predicate cost, G=grates, D=decimal key bytes.
fn wet(f:&Floor,i:usize)->bool {
    f.tiles.get(i).is_some_and(|t|t=="water")
        && f.ichor.as_ref().and_then(|v|v.get(i)).copied().unwrap_or(0.)==0.
        && !crate::environment::blocks_movement(f,i)
}
/// O(G+P+F+D) eligibility. P<=7 portal pairs; F=permanent fire cells.
/// No automatic stair/trap/pit traversal is introduced as a bank landing.
pub fn bank(f:&Floor,i:usize)->bool {
    f.tiles.get(i).is_some_and(|t|matches!(t.as_str(),"shallow-water"|"floor"|"grass"))
        && f.ichor.as_ref().and_then(|v|v.get(i)).copied().unwrap_or(0.)==0.
        && !crate::environment::blocks_movement(f,i)
        && !f.permanent_fire.as_ref().is_some_and(|v|v.contains(&(i as f64)))
        && !f.portals.as_ref().is_some_and(|v|v.iter().any(|p|p.a==i||p.b==i))
        && !crate::borge::solid_at(f,(i%f.width()) as f64,(i/f.width()) as f64)
        && !crate::epidite::solid_at(f,(i%f.width()) as f64,(i/f.width()) as f64)
}
/// O(1) index arithmetic and bounded direction decode.
fn arrow_target(f:&Floor,i:usize,code:f64)->Option<usize> {
    if code.fract()!=0. || !(1.0..=8.).contains(&code){return None;}
    let (dx,dy)=crate::currents::DIRECTIONS[code as usize];
    let x=(i%f.width()) as isize+dx;let y=(i/f.width()) as isize+dy;
    (x>=0&&y>=0&&x<f.width() as isize&&y<f.height() as isize).then(||y as usize*f.width()+x as usize)
}
/// O(passability) time/O(1) memory. Current-origin must remain water; only final
/// destination may be an admitted bank. No search, RNG, or mutations.
pub fn destination(f:&Floor,i:usize)->Option<usize> {
    if !wet(f,i){return None;}
    let to=arrow_target(f,i,*f.currents.as_ref()?.get(i)?)?;
    (wet(f,to)||bank(f,to)).then_some(to)
}
/// O(4 × bank eligibility), O(1) space. Only substitute an adjacent safe bank
/// when a shoreline destination is occupied; never search/rewrite the flow.
pub fn alternate_bank(f:&Floor,from:usize,mut available:impl FnMut(usize)->bool)->Option<usize> {
    neighbors(f,from).into_iter().flatten().find(|&i|bank(f,i)&&available(i))
}
/// O(V × eligibility + V), O(V) memory. Preserve every existing route that
/// already terminates at a bank; repair only trapping/no-flow paths. No RNG.
/// Enclosed components get zero arrows, never terrain carving.
pub fn repair(f:&mut Floor) {
    let n=f.tiles.len();if n==0||f.width()==0{return;}
    let water:Vec<_>=(0..n).map(|i|wet(f,i)).collect();
    let banks:Vec<_>=(0..n).map(|i|bank(f,i)).collect();
    let old=f.currents.as_ref().cloned().unwrap_or_else(||vec![0.;n]);
    let mut fallback=vec![0.;n];let mut distance=vec![usize::MAX;n];let mut queue=Vec::new();
    for i in 0..n {if water[i] {if let Some(to)=neighbors(f,i).into_iter().flatten().find(|&j|banks[j]) {
        distance[i]=1;fallback[i]=code(f,i,to);queue.push(i);
    }}}
    let mut head=0;while head<queue.len(){let to=queue[head];head+=1;
        for i in neighbors(f,to).into_iter().flatten(){if water[i]&&distance[i]==usize::MAX{
            distance[i]=distance[to]+1;fallback[i]=code(f,i,to);queue.push(i);
        }}
    }
    // Functional-graph classification is linear: each water node enters one
    // trail and is permanently marked good/bad after that trail terminates.
    let mut state=vec![0u8;n];
    for start in 0..n {if !water[start]||state[start]!=0 {continue;}
        let mut trail=Vec::new();let mut at=start;
        let good=loop {
            if banks[at]{break true;}
            if !water[at]{break false;}
            if state[at]!=0{break state[at]==2;}
            state[at]=1;trail.push(at);
            match arrow_target(f,at,old.get(at).copied().unwrap_or(0.)){Some(to)=>at=to,None=>break false}
        };
        for i in trail {state[i]=if good {2}else{3};}
    }
    let currents=(0..n).map(|i|if water[i]&&state[i]==2 {old[i]} else {fallback[i]}).collect();
    f.currents=crate::Field::Value(currents);
}
/// O(1) cardinal delta encoding, preserving the existing nine-code wire shape.
fn code(f:&Floor,from:usize,to:usize)->f64 {
    let delta=((to%f.width()) as isize-(from%f.width()) as isize,(to/f.width()) as isize-(from/f.width()) as isize);
    crate::currents::DIRECTIONS.iter().position(|&d|d==delta).unwrap() as f64
}

#[cfg(test)] mod tests {
    use super::*;
    /// O(V) synthetic construction with no external actors/obstacles.
    fn basin()->Floor {let mut f=Floor::default();f.tiles=vec!["floor".into();81];
        for y in 2..7 {for x in 2..7 {f.tiles[y*9+x]="water".into();}}f}
    /// O(V²) exhaustive test oracle, not runtime: every origin must terminate.
    fn all_reach_bank(f:&Floor) {
        for start in 0..f.tiles.len(){if !wet(f,start){continue;}let mut at=start;let mut seen=std::collections::HashSet::new();
            for _ in 0..=f.tiles.len(){if bank(f,at){break;}assert!(seen.insert(at),"cycle at {at} from {start}");at=destination(f,at).expect("water route reaches bank");}
            assert!(bank(f,at),"no bank from {start}");
        }
    }
    #[test] fn inward_basin_repairs_to_finite_bank_routes_without_rng_or_tile_changes(){let mut f=basin();crate::currents::generate_currents(&mut f);let tiles=f.tiles.clone();repair(&mut f);all_reach_bank(&f);assert_eq!(f.tiles,tiles);let once=f.currents.clone();repair(&mut f);assert_eq!(f.currents,once);}
    #[test] fn good_authored_route_is_preserved_bad_cycle_repaired(){let mut f=basin();repair(&mut f);let good=f.currents.as_ref().unwrap()[20];f.currents.as_mut().unwrap()[40]=3.;f.currents.as_mut().unwrap()[41]=7.;repair(&mut f);assert_eq!(f.currents.as_ref().unwrap()[20],good);all_reach_bank(&f);}
    #[test] fn enclosed_pool_disables_flow_without_carving(){let mut f=basin();for t in &mut f.tiles {if t=="floor"{*t="wall".into();}}crate::currents::generate_currents(&mut f);let before=f.tiles.clone();repair(&mut f);assert!(f.currents.as_ref().unwrap().iter().all(|&c|c==0.));assert_eq!(f.tiles,before);}
    #[test] fn occupied_final_bank_uses_only_available_adjacent_bank(){let mut f=basin();repair(&mut f);let from=20;let to=destination(&f,from).unwrap();assert!(bank(&f,to));let alternate=alternate_bank(&f,from,|i|i!=to).unwrap();assert_ne!(alternate,to);assert!(bank(&f,alternate));assert_eq!(alternate_bank(&f,from,|_|false),None);}
    #[test] fn legacy_destination_does_not_gain_bank_landings(){let mut f=basin();repair(&mut f);let from=20;assert!(destination(&f,from).is_some());assert_eq!(crate::environment::current_destination_revision(&f,from,39.),None);assert!(crate::environment::current_destination_revision(&f,from,40.).is_some());}
    #[test] fn every_origin_in_deterministic_generated_floor_reaches_bank_when_reachable(){
        for seed in [1,7,39] {let f=crate::generation::generate_floor_full_revision(1,seed,None,40.).unwrap();
            for start in 0..f.tiles.len(){if !wet(&f,start)||f.currents.as_ref().unwrap()[start]==0. {continue;}let mut at=start;let mut seen=std::collections::HashSet::new();
                while !bank(&f,at){assert!(seen.insert(at));at=destination(&f,at).unwrap();assert!(seen.len()<=f.tiles.len());}
            }
        }
    }
    #[test] fn actual_wait_washes_player_to_bank_and_reloads_exactly(){
        let mut r:crate::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=40.0.into();for v in r.actors.values_mut(){*v=std::sync::Arc::new(vec![]);}for v in r.ground.values_mut(){*v=std::sync::Arc::new(vec![]);}r.player.x=12.;r.player.y=14.;r.player.hp=10000.;r.player.max_hp=10000.;
        let f=r.floor_mut().unwrap();for t in &mut f.tiles {*t="floor".into();}for y in 12..17 {for x in 10..15 {f.tiles[y*79+x]="water".into();}}repair(f);
        let mut e=crate::temporal::Engine::new(r).unwrap();
        for _ in 0..26 {let i=e.run.player.y as usize*79+e.run.player.x as usize;if bank(e.run.floor().unwrap(),i){break;}e.dispatch(&crate::Command::Wait).unwrap();}
        let i=e.run.player.y as usize*79+e.run.player.x as usize;assert!(bank(e.run.floor().unwrap(),i));
        let raw=e.save().unwrap();assert_eq!(crate::temporal::Engine::load(&raw).unwrap().save().unwrap(),raw);
    }
    #[test] fn all_eight_authored_water_setups_have_nontrapping_repaired_routes() {
        for id in 101..=108 {
            let mut f=Floor::default();f.tiles=vec!["floor".into();625];f.lights=vec![0.;625];
            for y in 5..20 {for x in 5..20 {f.tiles[y*25+x]="water".into();}}
            f.setups=vec![crate::model::FloorSetupsEntry{id:id as f64,x:12.,y:12.,..Default::default()}].into();
            repair(&mut f);crate::generation::apply_water_setup_currents_revision(&mut f,40.);repair(&mut f);all_reach_bank(&f);
        }
    }
    /// O(fixture payload) test setup; retained floor keys satisfy admission.
    fn runtime_scene()->crate::Run {
        let mut r:crate::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=40.0.into();r.player.x=5.;r.player.y=5.;
        let f=r.floor_mut().unwrap();for t in &mut f.tiles {*t="floor".into();}
        for y in 12..17 {for x in 10..15 {f.tiles[y*79+x]="water".into();}}repair(f);
        for v in r.ground.values_mut(){*v=std::sync::Arc::new(vec![]);}
        r
    }
    #[test] fn npc_is_washed_to_bank_without_following_player_controls() {
        let mut r=runtime_scene();let a=std::sync::Arc::make_mut(r.actors.get_mut("1").unwrap()).first_mut().unwrap();a.x=12.;a.y=14.;a.abilities.clear();a.conditions.clear();let id=a.id;
        for _ in 0..26 {let a=crate::engine::actor(&r,id).unwrap();let at=a.y as usize*79+a.x as usize;if bank(r.floor().unwrap(),at){break;}crate::currents::tick_currents(&mut r).unwrap();}
        let a=crate::engine::actor(&r,id).unwrap();assert!(bank(r.floor().unwrap(),a.y as usize*79+a.x as usize));
    }
    #[test] fn floating_item_reaches_bank_via_environment_tick() {
        let mut r=runtime_scene();for v in r.actors.values_mut(){*v=std::sync::Arc::new(vec![]);}
        let item=crate::Item{id:500.,kind:"heart".into(),name:"Rat heart".into(),glyph:";".into(),..Default::default()};r.next_id=501.;
        std::sync::Arc::make_mut(r.ground.get_mut("1").unwrap()).push(crate::GroundItem{x:12.,y:14.,item,..Default::default()});
        for _ in 0..26 {let a=&r.ground["1"][0];if bank(r.floor().unwrap(),a.y as usize*79+a.x as usize){break;}crate::environment::tick_environment(&mut r).unwrap();}
        let a=&r.ground["1"][0];assert!(bank(r.floor().unwrap(),a.y as usize*79+a.x as usize));
    }
    #[test] fn inactive_floor_current_activity_catches_up_to_bank() {
        let mut r=runtime_scene();let a=std::sync::Arc::make_mut(r.actors.get_mut("1").unwrap()).first_mut().unwrap();a.x=12.;a.y=14.;a.state="sleeping".into();a.abilities.clear();a.conditions.clear();let id=a.id;
        let f=r.floor_mut().unwrap();f.dirty=true;f.last_tick=1.;
        let mut other=(**r.floors.first().unwrap()).clone();other.depth=2.;other.currents=crate::Field::Missing;
        r.floors[1]=std::sync::Arc::new(other);r.actors.insert("2".into(),std::sync::Arc::new(vec![]));r.ground.insert("2".into(),std::sync::Arc::new(vec![]));r.depth=2.;r.tick=27.;
        crate::offscreen::catch_up_floor(&mut r,1.).unwrap();
        let a=r.actors["1"].iter().find(|a|a.id==id).unwrap();assert!(bank(&r.floors[0],a.y as usize*79+a.x as usize));
    }

}
