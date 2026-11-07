//! Revision 35 portal metadata and optional-route-preserving placement.
use crate::{Actor, Field, Floor, GroundItem, Run};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const MAX_PAIRS: usize = 7;
pub const COLOR_NAMES: [&str; MAX_PAIRS] = ["blue", "orange", "red", "yellow", "green", "indigo", "violet"];
/// Cell indices belong to this floor only. Order is stable and determines color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct PortalPair {
    pub a: usize,
    pub b: usize,
    pub color_index: u8,
}

/// Initial numerical default, not designer-authored density scaling.
pub fn requested_pairs(depth: u32) -> usize { usize::from(depth >= 10) }

pub fn validate_floor(f: &Floor) -> bool {
    let Some(pairs) = f.portals.as_ref() else { return f.portals.is_missing(); };
    if pairs.len() > MAX_PAIRS || (!pairs.is_empty() && !(10.0..=40.0).contains(&f.depth)) { return false; }
    let mut cells = HashSet::new();
    pairs.iter().enumerate().all(|(index,p)| p.color_index as usize == index
        && p.a < f.tiles.len() && p.b < f.tiles.len() && p.a != p.b
        && cells.insert(p.a) && cells.insert(p.b))
}
pub fn validate_run(r: &Run) -> bool {
    let floors = r.floors.iter().chain(r.dreamwalking.as_ref().into_iter().flat_map(|s|s.floors.iter()));
    floors.into_iter().all(|f| validate_floor(f)
        && (r.simulation_revision.value_or(29.0) >= 35.0 || f.portals.is_missing()))
}

fn neighbors(at: usize, w: usize, n: usize) -> [Option<usize>;4] {
    [if at % w > 0 {Some(at-1)} else {None}, if at % w+1 < w {Some(at+1)} else {None},
        if at >= w {Some(at-w)} else {None}, if at+w < n {Some(at+w)} else {None}]
}
fn walking(f: &Floor) -> Vec<bool> {
    let mut result: Vec<_> = f.tiles.iter().enumerate().map(|(at,t)|
        !["wall","tree","chasm","pit"].contains(&t.as_str())
        && !crate::borge::solid_at(f,(at%f.width()) as f64,(at/f.width()) as f64)
        && !crate::epidite::solid_at(f,(at%f.width()) as f64,(at/f.width()) as f64)).collect();
    for at in f.grates.as_ref().into_iter().flatten().chain(f.permanent_fire.as_ref().into_iter().flatten()) {
        if let Some(v)=result.get_mut(*at as usize) {*v=false;}
    }
    result
}

/// Iterative Tarjan articulation test: O(V+E) time, O(V) scratch, E<=4V.
/// Recomputed after each accepted endpoint, so ALL endpoints can be avoided together.
fn articulation(open: &[bool], w: usize) -> Vec<bool> {
    let n=open.len(); let mut discovered=vec![0usize;n]; let mut low=vec![0usize;n];
    let mut parent=vec![usize::MAX;n]; let mut children=vec![0usize;n]; let mut cut=vec![false;n]; let mut time=0;
    for root in 0..n {
        if !open[root] || discovered[root]!=0 {continue;}
        time+=1; discovered[root]=time;low[root]=time;
        let mut stack=vec![(root,0usize)];
        while let Some(&(at,next))=stack.last() {
            if next==4 {
                stack.pop();
                if parent[at]!=usize::MAX {
                    let p=parent[at];low[p]=low[p].min(low[at]);
                    if parent[p]!=usize::MAX && low[at]>=discovered[p] {cut[p]=true;}
                } else {cut[at]=children[at]>1;}
                continue;
            }
            stack.last_mut().unwrap().1+=1;
            let Some(other)=neighbors(at,w,n)[next] else {continue;};
            if !open[other] {continue;}
            if discovered[other]==0 {
                parent[other]=at;children[at]+=1;time+=1;discovered[other]=time;low[other]=time;
                stack.push((other,0));
            } else if other!=parent[at] {low[at]=low[at].min(discovered[other]);}
        }
    }
    cut
}

/// Test the original walking components with every portal endpoint excluded.
/// O(P(V+E)) time, O(V) scratch; no portal edge is used to prove optionality.
pub fn all_endpoints_optional(f: &Floor) -> bool {
    let mut open=walking(f);
    for p in f.portals.as_ref().into_iter().flatten() {
        for at in [p.a,p.b] {
            if at>=open.len() || !open[at] || articulation(&open,f.width())[at] {return false;}
            open[at]=false;
        }
    }
    true
}

/// O(P(V+E)+A+I+F) time and O(V+A+I+F) scratch, P<=7, E<=4V;
/// F counts fixture/connector entries. No gameplay RNG, terrain edits or actor budgets.
pub fn install(seed: &str, f: &mut Floor, actors: &[Actor], ground: &[GroundItem], revision: f64) {
    if revision<35.0 || !f.portals.is_missing() {return;}
    let wanted=requested_pairs(f.depth as u32).min(MAX_PAIRS);
    if wanted==0 {return;}
    install_count(seed,f,actors,ground,wanted);
}
pub fn install_count(seed: &str, f: &mut Floor, actors: &[Actor], ground: &[GroundItem], wanted: usize) {
    if f.depth<10.0 || !f.portals.is_missing() {return;}
    let w=f.width(); let n=f.tiles.len(); if w==0 || n%w!=0 {return;}
    let mut blocked: HashSet<usize>=crate::spawn_heatmap::fixture_cells(f).into_iter()
        .chain(actors.iter().map(|a|a.y as usize*w+a.x as usize))
        .chain(ground.iter().map(|g|g.y as usize*w+g.x as usize)).collect();
    if let Some(connectors)=f.room_graph.as_ref().map(|g|&g.connectors).or(f.dungeon_connectors.as_ref()) {
        blocked.extend(connectors.iter().flat_map(|c|c.cells.iter().map(|v|*v as usize)));
    }
    if let Some(s)=f.sphinx_room.as_ref() {blocked.extend(s.cells.iter().map(|v|*v as usize));}
    let mut open=walking(f); let mut chosen=Vec::new();
    // Both endpoints remain reachable by an ordinary walker from the upstairs.
    // A disconnected decorative island cannot receive an unusable pair.
    let mut reachable=vec![false;n];
    if let Some(start)=f.tiles.iter().position(|t|t=="up").filter(|&at|open[at]) {
        let mut queue=vec![start];reachable[start]=true;let mut head=0;
        while head<queue.len() {
            let at=queue[head];head+=1;
            for next in neighbors(at,w,n).into_iter().flatten() {
                if open[next] && !reachable[next] {reachable[next]=true;queue.push(next);}
            }
        }
    }
    let mut rng=crate::population::population_random(&format!("{seed}:floor:{}:portals-r35",f.depth));
    for _ in 0..wanted.min(MAX_PAIRS)*2 {
        let cuts=articulation(&open,w);
        let candidates: Vec<_>=(0..n).filter(|&at|open[at] && reachable[at] && !cuts[at] && !blocked.contains(&at)
            && ["floor","grass","shallow-water"].contains(&f.tiles[at].as_str())
            && neighbors(at,w,n).into_iter().flatten().filter(|&i|open[i]).count()>=3).collect();
        if candidates.is_empty() {break;}
        let at=candidates[(rng.random()*candidates.len() as f64).floor() as usize];
        chosen.push(at);open[at]=false;
    }
    // An unmatched candidate is not a portal and never modifies terrain.
    chosen.truncate(chosen.len()/2*2);
    f.portals=Field::Value(chosen.chunks_exact(2).enumerate().map(|(i,p)|PortalPair{a:p[0],b:p[1],color_index:i as u8}).collect());
    debug_assert!(validate_floor(f) && all_endpoints_optional(f));
}

/// Revision37 Corn chamber pairs. Each accepted pair preserves both ordinary
/// optional walking lines and the complete existing non-Euclidean graph.
/// At most 4*32 proposals, O(P*32*(V+E)) time and O(V+E) scratch.
pub fn append_in_cells(f:&mut Floor,actors:&[Actor],ground:&[GroundItem],cells:&[usize],wanted:usize,rng:&mut crate::rng::GenerationRng){
    let w=f.width();let blocked:HashSet<_>=crate::spawn_heatmap::fixture_cells(f).into_iter().chain(actors.iter().map(|a|a.y as usize*w+a.x as usize)).chain(ground.iter().map(|g|g.y as usize*w+g.x as usize)).chain(f.non_euclidean.as_ref().into_iter().flatten().flat_map(|p|p.cells.iter().copied())).chain(f.speed_up.as_ref().into_iter().flatten().flat_map(|p|p.cells.iter().copied())).collect();
    for _ in 0..wanted.min(MAX_PAIRS){if f.portals.as_ref().map_or(0,Vec::len)>=MAX_PAIRS{break}let mut accepted=false;
        for _ in 0..32{let used:HashSet<_>=f.portals.as_ref().into_iter().flatten().flat_map(|p|[p.a,p.b]).collect();let choices:Vec<_>=cells.iter().copied().filter(|i|!blocked.contains(i)&&!used.contains(i)&&matches!(f.tiles[*i].as_str(),"floor"|"grass")).collect();if choices.len()<2{break}let a=choices[(rng.random()*choices.len()as f64).floor()as usize];let others:Vec<_>=choices.into_iter().filter(|&b|b!=a).collect();let b=others[(rng.random()*others.len()as f64).floor()as usize];let pairs=f.portals.get_or_insert_with(Vec::new);pairs.push(PortalPair{a,b,color_index:pairs.len()as u8});
            if all_endpoints_optional(f)&&crate::non_euclidean_generation::traversable(f){accepted=true;break}f.portals.as_mut().unwrap().pop();}
        if !accepted{break}
    }
}
