//! Guard the tap with a whole authored pack, never inflated to fill space.
use crate::{Actor, Floor, Run, Field, GodbeastEventTap};
use std::collections::{HashSet, HashMap, VecDeque};

fn neighbors(f:&Floor, at:usize)->Vec<usize> {
    let w=f.width(); let a=crate::movement::Point{x:(at%w) as i32,y:(at/w) as i32};
    crate::movement::DIRECTIONS.iter().filter_map(|d| {
        let b=crate::movement::Point{x:a.x+d.x,y:a.y+d.y};
        if b.x<0 || b.y<0 || b.x>=w as i32 || b.y>=f.height() as i32 { return None; }
        let i=b.y as usize*w+b.x as usize;
        (!matches!(f.tiles[i].as_str(),"wall"|"tree"|"chasm"|"pit")
            && !f.grates.as_ref().is_some_and(|g|g.contains(&(i as f64)))
            && crate::movement::diagonal_step_allowed(&f.tiles,w,a,b,false)).then_some(i)
    }).collect()
}
fn distances(f:&Floor,start:usize)->Vec<usize> {
    let mut dist=vec![usize::MAX;f.tiles.len()]; let mut q=VecDeque::from([start]); dist[start]=0;
    while let Some(i)=q.pop_front() { for j in neighbors(f,i) { if dist[j]==usize::MAX {dist[j]=dist[i]+1;q.push_back(j);} } }
    dist
}
fn guard_depth(r:&Run,depth:f64)->u32 { if crate::fast_mode::compressed(r) {
        crate::fast_mode::content_depth(((depth/2.).ceil() as u32+2).min(20))
    } else {(depth as u32+2).min(40)} }
/// O(K*V + V log V + V*C*M) with cached per-species fuel scans (M<=5);
/// O(K*V + V + C) scratch. See complexity ledger for helper assumptions.
/// Every approach is occupied by one member of a single unmodified catalog pack.
pub(crate) fn place(r:&mut Run,f:&mut Floor,actors:&mut Vec<Actor>,reserved:&HashSet<usize>)->Result<(),String>{
    let revision=r.simulation_revision.value_or(29.);
    if revision<52. || r.veridian_sap_harvested.value_or(false) || !f.godbeast_event.as_ref().is_some_and(|e|e.kind=="Veridian") {return Ok(());}
    let start=f.tiles.iter().position(|t|t=="up").ok_or("Verdania guard has no entrance")?;
    let source_depth=guard_depth(r,f.depth);
    let candidates=crate::ordinary_roster::candidates_revision(source_depth,false,revision)?;
    let mut fuel=HashMap::new();
    for kind in candidates.iter().flat_map(|c|&c.kinds) { fuel.entry(kind.clone()).or_insert_with(||crate::generation::initial_fuel_spawn_exclusions(f,kind)); }
    let mut blocked=crate::population::blocked_cells(f);
    blocked.extend(reserved); blocked.extend(actors.iter().map(|a|a.y as usize*f.width()+a.x as usize));
    // The old random tap is allowed to become the new guarded destination.
    if let Some(t)=f.godbeast_event.as_ref().and_then(|e|e.tap.as_ref()) {
        let i=t.y as usize*f.width()+t.x as usize;
        if !reserved.contains(&i) && !actors.iter().any(|a|a.x==t.x&&a.y==t.y) { blocked.remove(&i); }
    }
    let dist=distances(f,start);
    let mut cells:Vec<_>=(0..f.tiles.len()).filter(|i|dist[*i]!=usize::MAX&&dist[*i]>8
        && matches!(f.tiles[*i].as_str(),"floor"|"grass")&&!blocked.contains(i)).collect();
    cells.sort_by_key(|i|(std::cmp::Reverse(dist[*i]),*i));
    let mut rng=crate::population::population_random(&format!("{}:floor:{}:verdania-guard",r.seed,f.depth));
    for tap in cells {
        let approach=neighbors(f,tap);
        if approach.is_empty() || approach.iter().any(|i|blocked.contains(i)||!matches!(f.tiles[*i].as_str(),"floor"|"grass")) {continue;}
        let packs:Vec<_>=candidates.iter().filter(|c|c.kinds.len()==approach.len()
            && c.kinds.iter().all(|k|crate::population::catalog().awake.get(k)!=Some(&true)
                && !matches!(k.as_str(),"sphinx"|"hemogoblin-mech"|"ursa-minor"|"ursa-major")
                && approach.iter().all(|i|!fuel[k].contains(i)))) .collect();
        if packs.is_empty(){continue;}
        let pack=packs[(rng.random()*packs.len() as f64).floor() as usize];
        let first=actors.len();
        for (kind,at) in pack.kinds.iter().zip(&approach) {
            let mut a=crate::population::create_population_actor_revision(&mut r.next_id,f,kind,*at,true,&mut rng,revision)?;
            a.state="sleeping".into(); actors.push(a);
        }
        let ids:Vec<_>=actors[first..].iter().map(|a|a.id).collect();
        let score=crate::population::encounter_score(f.depth,&pack.kinds)?;
        let records=f.encounters.get_or_insert_with(Vec::new);
        records.push(crate::EncounterRecord{id:records.len() as f64+1.,danger:crate::population::encounter_danger(f.depth,score).into(),score,floor_threat:crate::population::floor_threat(f.depth),sleeping:true,leader_id:ids[0].into(),follower_ids:ids[1..].to_vec().into(),actor_ids:ids,setup_indices:vec![],kind:"verdania-guard".to_string().into()});
        let w=f.width();
        f.godbeast_event.as_mut().unwrap().tap=Field::Value(GodbeastEventTap{x:(tap%w) as f64,y:(tap/w) as f64});
        return Ok(());
    }
    Err("No guarded Verdania tap location".into())
}

/// Recheck after all terrain decorators and special spawns; never accept a bypass.
pub(crate) fn validate(f:&Floor,actors:&[Actor])->Result<(),String>{
    let Some(record)=f.encounters.as_ref().and_then(|es|es.iter().find(|e|e.kind.as_ref().map(String::as_str)==Some("verdania-guard"))) else {return Ok(());};
    let Some(t)=f.godbeast_event.as_ref().and_then(|e|e.tap.as_ref()) else{return Err("No guarded Verdania tap location".into());};
    let tap=t.y as usize*f.width()+t.x as usize;
    let guard:HashSet<_>=actors.iter().filter(|a|record.actor_ids.contains(&a.id)&&a.state=="sleeping"&&!a.dead.value_or(false)).map(|a|a.y as usize*f.width()+a.x as usize).collect();
    if guard.len()!=record.actor_ids.len() || !neighbors(f,tap).iter().all(|i|guard.contains(i)) {return Err("No guarded Verdania tap location".into());}
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn corridor()->Floor {
        let mut f=Floor{depth:15.,tiles:vec!["wall".into();225],..Default::default()};
        for x in 1..14 {f.tiles[7*15+x]="floor".into();}
        f.tiles[7*15+1]="up".into();
        f.godbeast_event=crate::GodbeastEvent{kind:"Veridian".into(),..Default::default()}.into();f
    }
    #[test] fn seeded_guard_is_sleeping_authored_pack_and_blocks_all_approaches(){
        for seed in 0..12 {
            let mut f=corridor(); let mut r=Run{seed:seed.to_string(),next_id:100.,simulation_revision:52.0.into(),compressed_depths:true.into(),..Default::default()};
            let mut actors=vec![];place(&mut r,&mut f,&mut actors,&HashSet::new()).unwrap();
            let t=f.godbeast_event.as_ref().unwrap().tap.as_ref().unwrap();let tap=t.y as usize*f.width()+t.x as usize;
            assert_eq!(tap,7*15+13);assert!(actors.iter().all(|a|a.state=="sleeping"));
            let guard:HashSet<_>=actors.iter().map(|a|a.y as usize*f.width()+a.x as usize).collect();
            assert!(neighbors(&f,tap).iter().all(|i|guard.contains(i)));
            assert_eq!(actors.len(),1);
            // Killing guards restores an ordinary path; no map walls were added.
            assert_ne!(distances(&f,7*15+1)[tap],usize::MAX);
        }
    }
    #[test] fn impossible_placement_is_explicit_and_no_partial_mutation(){
        let mut f=corridor();let mut r=Run{next_id:100.,simulation_revision:52.0.into(),..Default::default()};let mut a=vec![];
        let reserved=(0..225).collect(); assert_eq!(place(&mut r,&mut f,&mut a,&reserved).unwrap_err(),"No guarded Verdania tap location");assert_eq!(r.next_id,100.);assert!(a.is_empty());assert!(f.godbeast_event.as_ref().unwrap().tap.is_missing());
    }
    #[test] fn generated_maps_have_guarded_reachable_taps() {
        for seed in 1..=4 {
            let mut f=crate::generation::generate_floor_revision(15,seed,"Forest",false,79,None,52.).unwrap();
            f.godbeast_event=crate::GodbeastEvent{kind:"Veridian".into(),..Default::default()}.into();
            let mut r=Run{seed:format!("guard-{seed}"),next_id:100.,simulation_revision:52.0.into(),compressed_depths:true.into(),..Default::default()};
            let mut a=vec![];place(&mut r,&mut f,&mut a,&HashSet::new()).unwrap();
            let t=f.godbeast_event.as_ref().unwrap().tap.as_ref().unwrap();let tap=t.y as usize*f.width()+t.x as usize;
            assert!(neighbors(&f,tap).iter().all(|i|a.iter().any(|a|a.x as usize==i%f.width()&&a.y as usize==i/f.width())));
            assert!(a.iter().all(|a|a.state=="sleeping"));
        }
    }

    #[test] fn compressed_guard_depth_caps_at_last_roster(){
        let r=Run{compressed_depths:true.into(),..Default::default()};
        assert_eq!(guard_depth(&r,25.),30); // displayed13 -> displayed15
        assert_eq!(guard_depth(&r,35.),40); // latest regular boss displayed18 ->20
        assert_eq!(guard_depth(&r,37.),40); // displayed19 -> capped20
        assert_eq!(guard_depth(&r,40.),40); // displayed20 -> capped20
    }

}
