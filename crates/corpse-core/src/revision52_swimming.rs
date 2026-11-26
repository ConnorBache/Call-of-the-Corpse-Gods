//! R52 conditional Swimming ring and deterministic isolated-island safeguards.
//! Existing floors are never repaired on load. Generation alone calls placement.
use crate::{Actor, Field, Floor, GroundItem, Item, Run, movement::{self, Point}};
use std::collections::VecDeque;

pub fn make_ring(id: f64) -> Item {
    Item { id, name: "Ring of Swimming".into(), kind: "ring".into(), glyph: "=".into(),
        effect: crate::revision50_rings::SWIMMING.to_string().into(), identified: true.into(),
        ..Default::default() }
}

/// O(I+C+B) owner/equipment/flight queries, O(1) scratch. Presence is boolean:
/// no stored duration, RNG draw, extra stacking multiplier, or per-turn mutation.
pub fn grants_haste(r: &Run, a: &Actor) -> bool {
    if r.simulation_revision.value_or(29.) < 52.
        || !crate::revision50_rings::has_effect(r,a,crate::revision50_rings::SWIMMING)
        || crate::travel::flying(r,a) { return false; }
    let Some(f) = crate::dreamwalking::floor_for(r,a) else { return false; };
    if a.x < 0. || a.y < 0. || a.x >= f.width() as f64 || a.y >= f.height() as f64
        || a.x.fract() != 0. || a.y.fract() != 0. { return false; }
    water(f, a.y as usize*f.width()+a.x as usize)
}

fn nonzero(layer: &Field<Vec<f64>>, at: usize) -> bool {
    layer.as_ref().and_then(|v|v.get(at)).is_some_and(|v|*v != 0.)
}
fn water(f: &Floor, at: usize) -> bool {
    matches!(f.tiles.get(at).map(String::as_str),Some("water"|"shallow-water"))
        && !nonzero(&f.ichor,at)
}

fn neighbors(f: &Floor, from: usize) -> impl Iterator<Item=(usize,i32,i32)> + '_ {
    let p=Point{x:(from%f.width()) as i32,y:(from/f.width()) as i32};
    movement::DIRECTIONS.into_iter().filter_map(move |d| {
        let q=Point{x:p.x+d.x,y:p.y+d.y};
        (q.x>=0 && q.y>=0 && q.x<f.width() as i32 && q.y<f.height() as i32
            && movement::diagonal_step_allowed(&f.tiles,f.width(),p,q,false))
            .then(||(q.y as usize*f.width()+q.x as usize,d.x,d.y))
    })
}

/// Physical land/water identity precedes transport exclusions. A portal or
/// spatial pocket cannot make an otherwise real island disappear from discovery.
/// Dynamic actors never split land components; they exclude placement cells.
fn physical_terrain(f: &Floor, at: usize) -> bool {
    !matches!(f.tiles[at].as_str(),"wall"|"tree"|"chasm"|"pit"|"up"|"down")
        && !f.grates.as_ref().is_some_and(|g|g.contains(&(at as f64)))
        && !crate::borge::solid_at(f,(at%f.width()) as f64,(at/f.width()) as f64)
        && !crate::epidite::solid_at(f,(at%f.width()) as f64,(at/f.width()) as f64)
        && !nonzero(&f.ichor,at)
}
/// Transport endpoints are not ordinary walking/placement shortcuts. Their
/// terrain still participates in physical island identity above.
fn ordinary_cell(f: &Floor, at: usize) -> bool {
    !f.portals.as_ref().is_some_and(|p|p.iter().any(|p|p.a==at||p.b==at))
        && crate::non_euclidean_generation::pocket_at(f,at).is_none()
        && crate::non_euclidean_generation::speedup_at(f,at).is_none()
}

fn safe_item_cell(f: &Floor, at: usize, fixture: &[bool]) -> bool {
    matches!(f.tiles[at].as_str(),"floor"|"grass") && !fixture[at] && safe_contact(f,at)
}
fn safe_contact(f:&Floor,at:usize)->bool {
    !f.permanent_fire.as_ref().is_some_and(|v|v.contains(&(at as f64)))
        && [&f.ichor,&f.mud,&f.slime,&f.roots,&f.razorleaves].iter().all(|l|!nonzero(l,at))
        && !crate::terrain_runtime::ice_at(f,at)
        && !crate::terrain_runtime::hellfire_at(f,at)
        && !f.bulwarks.as_ref().is_some_and(|b|b.contains_key(&at.to_string()))
        && f.environment.as_ref().is_none_or(|e|
            [&e.fire,&e.caustic,&e.breather,&e.paralysis].iter().all(|l|l.get(at).is_none_or(|v|*v==0.))
            && [&e.gas_volume,&e.confusion,&e.steam,&e.flammable,&e.explosive,&e.hydrogen,&e.hallucinogenic,&e.death_gas]
                .iter().all(|l|!nonzero(l,at)))
}

struct Geometry {
    geometric: Vec<bool>,
    passable: Vec<bool>,
    component: Vec<usize>,
    ordinary_component: Vec<usize>,
    land_return: Vec<bool>,
    components: Vec<Vec<usize>>,
}
fn land_components(f:&Floor,allowed:&[bool])->(Vec<usize>,Vec<Vec<usize>>) {
    let mut labels=vec![usize::MAX;f.tiles.len()];let mut components=Vec::new();
    for start in 0..f.tiles.len() {
        if !allowed[start]||f.tiles[start]=="water"||labels[start]!=usize::MAX {continue;}
        let label=components.len();let mut cells=vec![start];labels[start]=label;let mut head=0;
        while head<cells.len() {
            for (to,_,_) in neighbors(f,cells[head]) {
                if allowed[to]&&f.tiles[to]!="water"&&labels[to]==usize::MAX {labels[to]=label;cells.push(to);}
            }
            head+=1;
        }
        components.push(cells);
    }
    (labels,components)
}
impl Geometry {
    // Each of at most 8V edges is visited a constant number of times. Predicate
    // scans are precomputed once per cell; metadata lookup costs remain explicit.
    fn new(f: &Floor) -> Result<Self,String> {
        let n=f.tiles.len();
        if f.width()==0 || n==0 { return Err("Isolated island safeguard requires a nonempty floor".into()); }
        let geometric:Vec<_>=(0..n).map(|at|physical_terrain(f,at)).collect();
        let passable:Vec<_>=(0..n).map(|at|geometric[at]&&ordinary_cell(f,at)).collect();
        let (component,components)=land_components(f,&geometric);
        let (ordinary_component,_)=land_components(f,&passable);
        let land_return=prove_land_return(f,&component,&passable);
        Ok(Self{geometric,passable,component,ordinary_component,land_return,components})
    }
    fn meaningful(&self,f:&Floor)->Vec<bool> {
        self.components.iter().map(|cells|cells.iter().any(|&at|f.tiles[at]!="shallow-water")).collect()
    }
    fn islands<'a>(&'a self,f:&'a Floor) -> impl Iterator<Item=&'a Vec<usize>> + 'a {
        let meaningful=self.meaningful(f);
        let separate_land=meaningful.iter().filter(|&&dry|dry).count()>1;
        self.components.iter().enumerate().filter_map(move |(id,cells)| {
            // One shore surrounding a lake is not a stranded island. Shallow
            // specks alone are not a meaningful destination or separate island.
            (separate_land && meaningful[id]
                && cells.iter().any(|&at|neighbors(f,at).any(|(to,_,_)|
                    self.geometric[to]&&f.tiles[to]=="water"))).then_some(cells)
        })
    }
    fn ring_accessible_across_island(&self,_f:&Floor,cells:&[usize])->bool {
        let Some(label)=cells.iter().map(|&at|self.ordinary_component[at]).find(|&c|c!=usize::MAX) else{return false;};
        cells.iter().all(|&at|if self.passable[at] {self.ordinary_component[at]==label}
            else {self.land_return[at]})
    }
}

/// Prove finite land-only access from every special square to ordinary ground.
/// One action succeeds only when ALL actual spatial outcomes already have a
/// proof. Each action has <=2 outcomes; counters process <=8V actions/16V
/// outcomes once. O(V*M+V) time, O(V) scratch, including walking_exits metadata.
/// Same-island dry/shallow endpoints exclude currents, cross-island portals,
/// hazards and lucky stochastic branches. No RNG is sampled and no state changes.
fn prove_land_return(f:&Floor,component:&[usize],passable:&[bool])->Vec<bool> {
    let n=f.tiles.len();
    let admitted:Vec<_>=(0..n).map(|at|component[at]!=usize::MAX&&safe_contact(f,at)).collect();
    let mut proven:Vec<_>=(0..n).map(|at|passable[at]&&component[at]!=usize::MAX).collect();
    let mut incoming=vec![Vec::new();n];let mut owners=Vec::new();let mut pending=Vec::new();
    for from in 0..n {
        if component[from]==usize::MAX||proven[from] {continue;}
        for (entry,_,_) in neighbors(f,from) {
            if !admitted[entry]||component[entry]!=component[from] {continue;}
            let outcomes=crate::portals::walking_exits(f,from,entry);
            if outcomes.is_empty()||outcomes.iter().any(|&to|to>=n||!admitted[to]||component[to]!=component[from]) {continue;}
            let action=owners.len();owners.push(from);pending.push(outcomes.len());
            for to in outcomes {incoming[to].push(action);}
        }
    }
    let mut queue:VecDeque<_>=proven.iter().enumerate().filter_map(|(at,&yes)|yes.then_some(at)).collect();
    while let Some(to)=queue.pop_front() {
        for &action in &incoming[to] {
            pending[action]-=1;
            if pending[action]==0&&!proven[owners[action]] {
                proven[owners[action]]=true;queue.push_back(owners[action]);
            }
        }
    }
    proven
}

#[derive(Clone,Copy)]
struct Step { to:usize, dx:i32, dy:i32 }
// Baseline Swimming-only Move/Wait transition, without innate Fast or other
// statuses. True Haste is checked at action start and includes Wait: the bonus
// action has no world/current tick, even when movement leaves the water.
fn baseline_transition(f:&Floor,drift:&[usize],at:usize,entry:usize,owned:bool,haste:bool)->(usize,bool) {
    let bonus=owned&&haste&&water(f,at);
    (if bonus {entry} else {drift[entry]},!bonus)
}
// State 2*cell is normal; 2*cell+1 is the full-action Haste opportunity.
// Every successful normal Move or Wait ends in the Haste opportunity after
// exactly one existing current displacement; a bonus Move or Wait ends normal.
fn mobility(f:&Floor,g:&Geometry,owned:bool) -> Vec<Vec<Step>> {
    let n=f.tiles.len();let mut edges=vec![Vec::new();2*n];
    let drift:Vec<_>=(0..n).map(|at|crate::environment::current_destination_revision(f,at,52.).unwrap_or(at)).collect();
    for at in 0..n {
        if !g.passable[at] {continue;}
        for haste in [false,true] {
            let from=at*2+usize::from(haste);
            for (entry,dx,dy) in neighbors(f,at).chain(std::iter::once((at,0,0))) {
                if !g.passable[entry] {continue;}
                let (end,next_haste)=baseline_transition(f,&drift,at,entry,owned,haste);
                // A special endpoint/current beyond the admitted ordinary map
                // is not silently treated as standing still or safe transport.
                if !g.passable[end] {continue;}
                edges[from].push(Step{to:end*2+usize::from(next_haste),dx,dy});
            }
        }
    }
    edges
}

/// Propagate at most two distinct reachable dry-land labels per state. Two
/// suffice to answer "can I reach a component other than my own?" Every edge
/// is visited at most twice: O(V+E) time/scratch, independent of island count.
fn escape_labels(f:&Floor,g:&Geometry,edges:&[Vec<Step>])->Vec<[usize;2]> {
    let mut incoming=vec![Vec::new();edges.len()];
    for (from,steps) in edges.iter().enumerate() {for step in steps {incoming[step.to].push(from);}}
    let meaningful=g.meaningful(f);
    let mut labels=vec![[usize::MAX;2];edges.len()];let mut queue=VecDeque::new();
    for at in 0..f.tiles.len() {
        let label=g.component[at];
        if g.passable[at]&&label!=usize::MAX&&meaningful[label] {
            for state in [at*2,at*2+1] {labels[state][0]=label;queue.push_back((state,label));}
        }
    }
    while let Some((to,label))=queue.pop_front() {
        for &from in &incoming[to] {
            if !labels[from].contains(&label) {
                if let Some(slot)=labels[from].iter_mut().find(|v|**v==usize::MAX) {
                    *slot=label;queue.push_back((from,label));
                }
            }
        }
    }
    labels
}
fn reaches_other(labels:[usize;2],own:usize)->bool {
    labels.into_iter().any(|label|label!=usize::MAX&&label!=own)
}

/// Directed entrance proof starts without a Swimming ring. State is
/// 4*cell + 2*has_ring + haste_phase. On a safe dry ring square, optional normal
/// Drops, the existing free Pickup, and an optional normal Wait reach the owned
/// haste-phase state without moving. Full capacity never becomes a free slot.
/// Normal Move/Wait applies current displacement; true-Haste Move/Wait does not.
/// This baseline proof assumes no innate Fast or other status/ability effects.
/// One forward and reverse BFS over <=4V states/36V edges: O(V) time/scratch
/// after the precomputed terrain predicates. Dynamic hazards remain out of scope.
fn validate_entrance_progress(f:&Floor,g:&Geometry,rings:&[bool])->Result<(),String> {
    let exits:Vec<_>=f.tiles.iter().enumerate().filter_map(|(at,t)|matches!(t.as_str(),"down"|"pit").then_some(at)).collect();
    // Pure placement probes may describe only a lake. Complete generated floors
    // always contain the authored down stair (or final pit), and take this gate.
    if exits.is_empty() {return Ok(());}
    let n=f.tiles.len();let mut edges=vec![Vec::new();4*n];
    let drift:Vec<_>=(0..n).map(|at|crate::environment::current_destination_revision(f,at,52.).unwrap_or(at)).collect();
    for at in 0..n {if g.passable[at] {for owned in [false,true] {for haste in [false,true] {
        let from=4*at+2*usize::from(owned)+usize::from(haste);
        if !owned&&rings[at] {
            // Acquisition is required before leaving this proof node. If auto
            // pickup already succeeded, Wait reaches the same static state.
            edges[from].push(4*at+3);continue;
        }
        for (entry,_,_) in neighbors(f,at).chain(std::iter::once((at,0,0))) {
            if !g.passable[entry] {continue;}
            let (end,next_haste)=baseline_transition(f,&drift,at,entry,owned,haste);
            if g.passable[end] {edges[from].push(4*end+2*usize::from(owned)+usize::from(next_haste));}
        }
    }}}}
    let mut incoming=vec![Vec::new();4*n];
    for (from,targets) in edges.iter().enumerate() {for &to in targets {incoming[to].push(from);}}
    let mut reaches_exit=vec![false;4*n];let mut queue=VecDeque::new();
    for exit in exits {for (at,_,_) in neighbors(f,exit) {if g.passable[at] {
        for offset in 0..4 {let state=4*at+offset;if !reaches_exit[state] {reaches_exit[state]=true;queue.push_back(state);}}
    }}}
    while let Some(to)=queue.pop_front() {for &from in &incoming[to] {if !reaches_exit[from] {reaches_exit[from]=true;queue.push_back(from);}}}
    let up=f.tiles.iter().position(|t|t=="up").ok_or("Isolated island entrance has no up stair")?;
    let starts:Vec<usize>=if let Some(arrival)=crate::entrance::arrival(f) {
        let at=arrival.y as usize*f.width()+arrival.x as usize;
        if g.passable[at] {vec![4*at,4*at+1]} else {vec![]}
    } else {
        // Deeper player stair travel lands on up. It is an initial source only,
        // never a bridge. Conservatively validate every legal first departure,
        // including its normal current displacement, rather than a lucky side.
        neighbors(f,up).filter_map(|(entry,_,_)|(g.passable[entry]&&g.passable[drift[entry]])
            .then_some(4*drift[entry]+1)).collect()
    };
    if starts.is_empty()||starts.iter().any(|&state|!reaches_exit[state]) {
        return Err(format!("Isolated island entrance cannot reach the downstairs approach without a preowned Swimming ring on floor {}",f.depth));
    }
    let mut reachable=vec![false;4*n];let mut queue=VecDeque::new();
    for start in starts {if !reachable[start] {reachable[start]=true;queue.push_back(start);}}
    while let Some(from)=queue.pop_front() {for &to in &edges[from] {if !reachable[to] {reachable[to]=true;queue.push_back(to);}}}
    if rings.iter().enumerate().any(|(at,&ring)|ring&&!(0..4).any(|offset|reachable[at*4+offset])) {
        return Err(format!("Isolated island Swimming ring cannot be reached from the entrance on floor {}",f.depth));
    }
    Ok(())
}

/// One safe, unoccupied ring cell only on a water-separated dry component that
/// has no ordinary swimming escape to another meaningful land component. Both
/// phases must then have a ring-assisted escape. A single shore is not an island.
/// O(V*(M+S)+A+G) time and O(V+A+G) scratch, M per-cell fixture/topology metadata
/// scans, S per-cell safety scans; 8 neighbors, a 2-phase return graph and a
/// 4-state ownership/phase acquisition graph, V<=3600. No RNG.
/// Population conflicts are candidate errors, never loot relocation or removal.
pub fn placement_cells(f:&Floor,actors:&[Actor],ground:&[GroundItem]) -> Result<Vec<usize>,String> {
    let g=Geometry::new(f)?;
    let ordinary=escape_labels(f,&g,&mobility(f,&g,false));
    let assisted=escape_labels(f,&g,&mobility(f,&g,true));
    let mut fixture=vec![false;f.tiles.len()];
    for at in crate::spawn_heatmap::fixture_cells(f) {if at<fixture.len() {fixture[at]=true;}}
    let safe:Vec<_>=(0..f.tiles.len()).map(|at|g.passable[at]&&safe_item_cell(f,at,&fixture)
        &&reaches_other(assisted[at*2],g.component[at])&&reaches_other(assisted[at*2+1],g.component[at])).collect();
    let mut occupied=vec![false;f.tiles.len()];
    let mut ring_cells=vec![false;f.tiles.len()];
    let mut supplied=vec![false;g.components.len()];
    for a in actors.iter().filter(|a|!a.dead.value_or(false)) {
        if a.x>=0.&&a.y>=0.&&a.x<f.width() as f64&&a.y<f.height() as f64 {occupied[a.y as usize*f.width()+a.x as usize]=true;}
    }
    for item in ground {
        if item.x>=0.&&item.y>=0.&&item.x<f.width() as f64&&item.y<f.height() as f64 {
            let at=item.y as usize*f.width()+item.x as usize;
            occupied[at]=true;
            if safe[at] && g.component[at]!=usize::MAX
                && crate::revision50_rings::effect(&item.item)==Some(crate::revision50_rings::SWIMMING) {
                supplied[g.component[at]]=true;
                ring_cells[at]=true;
            }
        }
    }
    let mut out=Vec::new();
    for cells in g.islands(f) {
        let label=g.component[cells[0]];
        if !g.ring_accessible_across_island(f,cells) {
            return Err(format!("Isolated island is split or covered by special transport on floor {}",f.depth));
        }
        // A safe connected ordinary fragment makes this component-wide query
        // sound; an escaping fragment must never conceal another trapped one.
        if cells.iter().any(|&at|reaches_other(ordinary[at*2],label)) {continue;}
        if supplied[label] {continue;}
        let at=cells.iter().copied().filter(|&at|!occupied[at]&&safe[at]).min()
            .ok_or_else(||format!("Isolated island has no safe obtainable Swimming ring and escape route on floor {} ({} land cells)",f.depth,cells.len()))?;
        out.push(at);occupied[at]=true;ring_cells[at]=true;
    }
    validate_entrance_progress(f,&g,&ring_cells)?;
    Ok(out)
}

#[cfg(test)]
#[path="revision52_swimming_tests.rs"]
mod tests;
