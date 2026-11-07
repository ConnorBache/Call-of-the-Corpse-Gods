//! Revision 37 population-driven habitats and ordered spatial shortcuts.
//! All randomness is an independent seed/floor stream. Ordinary actors are
//! planned first; habitats are fitted around their actual initial positions.
use crate::{Actor, Floor, GroundItem, rng::GenerationRng};
use crate::non_euclidean_generation::{self as ne, NonEuclideanPocket as Pocket, NonEuclideanGroup as Group};
use crate::pocket_biomes::PocketBiome;
use std::collections::{BTreeSet, HashSet};

pub const CORN_RESIDENTS: usize = 8;
pub const CORN_RADIUS: i32 = 5;
pub const CORN_CHANCE: f64 = 1.0 / 3.0;
const SHAPE_ATTEMPTS: usize = 64;
fn cardinal(at:usize,w:usize,n:usize)->impl Iterator<Item=usize> {
    [if at%w>0{Some(at-1)}else{None},if at%w+1<w{Some(at+1)}else{None},if at>=w{Some(at-w)}else{None},if at+w<n{Some(at+w)}else{None}].into_iter().flatten()
}
fn disk(f:&Floor,at:usize,r:i32)->Vec<usize> {
    let (w,h)=(f.width(),f.height());let (x,y)=((at%w)as i32,(at/w)as i32);let mut v=vec![];
    for yy in y-r..=y+r {for xx in x-r..=x+r {if xx>0&&yy>0&&xx<(w-1)as i32&&yy<(h-1)as i32&&(xx-x).pow(2)+(yy-y).pow(2)<=r*r {v.push(yy as usize*w+xx as usize);}}}v
}
fn shuffle<T>(v:&mut [T],rng:&mut GenerationRng) {for i in (1..v.len()).rev(){let j=(rng.random()*(i+1)as f64).floor()as usize;v.swap(i,j);}}
fn walkable(t:&str)->bool {!matches!(t,"wall"|"tree"|"chasm"|"pit")}
fn reachable(f:&Floor)->Vec<bool> {
    let mut seen=vec![false;f.tiles.len()];let Some(start)=f.tiles.iter().position(|t|t=="up")else{return seen};
    let mut q=vec![start];seen[start]=true;let mut head=0;
    while head<q.len(){let at=q[head];head+=1;for next in cardinal(at,f.width(),f.tiles.len()){if !seen[next]&&walkable(&f.tiles[next]){seen[next]=true;q.push(next);}}}seen
}
/// One bounded deletion with a complete before/after route proof, O(V+E).
fn safe_block(f:&mut Floor,at:usize,kind:&str,revision:f64)->bool {
    let before=reachable(f);
    // R38 preserves the generation contract's hazard-free graph as well as bare
    // terrain connectivity. Permanent fire, grates and auto-transitions cannot
    // be used as replacement walking routes. Two bounded BFS scans remain O(V+E).
    let safe_before=(revision>=38.0).then(||crate::generation::reachable_without_braziers(f,None));
    let old=std::mem::replace(&mut f.tiles[at],kind.into());let after=reachable(f);
    let terrain_ok=before.iter().enumerate().all(|(i,&v)|!v||i==at||after[i]);
    let safe_ok=safe_before.is_none_or(|before| {
        let after=crate::generation::reachable_without_braziers(f,None);
        before.iter().all(|&i|i==at||after.contains(&i))
            &&crate::generation::transitions_accessible(f,&after)
    });
    if terrain_ok&&safe_ok{true}else{f.tiles[at]=old;false}
}
fn protected(f:&Floor,ground:&[GroundItem])->HashSet<usize> {
    let w=f.width();let mut set:HashSet<_>=crate::generation::protected_generation_cells(f).into_iter().chain(crate::spawn_heatmap::fixture_cells(f)).chain(ground.iter().map(|g|g.y as usize*w+g.x as usize)).collect();
    if let Some(room)=f.sphinx_room.as_ref(){set.extend(room.cells.iter().map(|v|*v as usize));set.extend(room.reward_cells.iter().map(|v|*v as usize));set.insert(room.y as usize*w+room.x as usize);}
    for (at,t)in f.tiles.iter().enumerate(){if matches!(t.as_str(),"up"|"down"|"pit"){set.extend(disk(f,at,3));}}
    for (at,t)in f.tiles.iter().enumerate(){if !matches!(t.as_str(),"wall"|"floor"|"grass"|"tree"|"water"|"shallow-water"|"chasm"){set.insert(at);}}
    for (at,_)in f.tiles.iter().enumerate(){if f.environment.as_ref().is_some_and(|e|e.fire[at]>0.0||e.gas_volume.as_ref().is_some_and(|v|v[at]>0.0))||[&f.ichor,&f.slime,&f.roots,&f.razorleaves].iter().any(|v|v.as_ref().is_some_and(|v|v[at]>0.0)){set.insert(at);}}
    set
}
fn union_theme(f:&mut Floor,theme:&str,cells:BTreeSet<usize>,flowers:Vec<usize>){if !cells.is_empty(){f.pocket_biomes.get_or_insert_with(Vec::new).push(PocketBiome{theme:theme.into(),cells:cells.into_iter().collect(),flowers,..Default::default()});}}
/// Three Moore-neighbor CA rounds, then only the cardinal component containing
/// the actual ice creature is retained. The anchor is alive in each round.
fn ice_component(f:&Floor,center:usize,allowed:&HashSet<usize>,rng:&mut GenerationRng)->Vec<usize>{
    let candidates=disk(f,center,4);let mut live:HashSet<_>=candidates.iter().copied().filter(|i|allowed.contains(i)&&rng.random()<0.62).collect();live.insert(center);
    let w=f.width();for _ in 0..3{let mut next=HashSet::new();for &at in &candidates{if !allowed.contains(&at){continue}let x=at%w;let y=at/w;let mut count=0;for dy in -1..=1{for dx in -1..=1{if dx==0&&dy==0{continue}let xx=x as i32+dx;let yy=y as i32+dy;if xx>=0&&yy>=0&&xx<w as i32&&yy<f.height()as i32&&live.contains(&(yy as usize*w+xx as usize)){count+=1;}}}if count>=if live.contains(&at){3}else{4}{next.insert(at);}}next.insert(center);live=next;}
    let mut q=vec![center];let mut seen=HashSet::from([center]);let mut head=0;while head<q.len(){let at=q[head];head+=1;for next in cardinal(at,w,f.tiles.len()){if live.contains(&next)&&seen.insert(next){q.push(next);}}}q.sort_unstable();q
}
/// O(A*V + B*(V+E) + 64*V) worst-case, O(V+A+I) scratch; B<=64
/// terrain removals across the floor. Generation runs once per populated floor.
pub fn decorate(seed:&str,f:&mut Floor,actors:&mut Vec<Actor>,ground:&[GroundItem],next_id:&mut f64,revision:f64)->Result<(),String>{
    if revision<37.0||!f.ice.is_missing(){return Ok(())}let n=f.tiles.len();let w=f.width();let original_tiles=f.tiles.clone();
    f.ice=vec![false;n].into();f.puddles=vec![false;n].into();f.hellfire=vec![false;n].into();
    let mut rng=crate::population::population_random(&format!("{seed}:floor:{}:sub-biomes-r37",f.depth));
    let mut blocked=protected(f,ground);let occupied:HashSet<_>=actors.iter().map(|a|a.y as usize*w+a.x as usize).collect();
    let existing:HashSet<_>=f.pocket_biomes.as_ref().into_iter().flatten().flat_map(|b|b.cells.iter().copied()).collect();blocked.extend(existing);
    let ice_anchors:Vec<_>=actors.iter().filter(|a|a.creature_kind.as_ref().is_some_and(|k|matches!(k.as_str(),"wendigo"|"yeti"))).map(|a|a.y as usize*w+a.x as usize).collect();
    let mut mountain=BTreeSet::new();let mut changes=0;
    for center in ice_anchors {
        let region=disk(f,center,5);mountain.extend(region.iter().copied().filter(|i|!blocked.contains(i)));mountain.insert(center);
        let mut holes:Vec<_>=region.iter().copied().filter(|i|!blocked.contains(i)&&!occupied.contains(i)&&matches!(f.tiles[*i].as_str(),"floor"|"grass")).collect();shuffle(&mut holes,&mut rng);
        let wanted=(holes.len()/8).clamp(1,6);let mut made=0;for at in holes.into_iter().take(18){if changes>=64||made>=wanted{break}changes+=1;if safe_block(f,at,"chasm",revision){made+=1;}}
        let allowed:HashSet<_>=region.iter().copied().filter(|i|!blocked.contains(i)&&matches!(f.tiles[*i].as_str(),"floor"|"grass"|"shallow-water")).chain([center]).collect();
        let ice=ice_component(f,center,&allowed,&mut rng);for at in ice{f.ice.as_mut().unwrap()[at]=true;}
        let snow=f.snow.get_or_insert_with(||vec![false;n]);for at in allowed{snow[at]=true;}
    }
    blocked.extend(mountain.iter().copied());union_theme(f,"mountainous",mountain,vec![]);
    let forest_anchors:Vec<_>=actors.iter().filter(|a|a.creature_kind.as_ref().is_some_and(|k|matches!(k.as_str(),"dryad"|"bear"|"tiger"|"short-saidri"|"long-saidri"))).map(|a|a.y as usize*w+a.x as usize).collect();
    let mut forest=BTreeSet::new();let mut flowers=vec![];
    // New fuel remains outside existing vent/fire gas components, matching r33.
    let gas=crate::generation::build_generation_gas_components(f);let unsafe_components:HashSet<_>=f.vents.as_ref().into_iter().flatten().map(|v|v.at as usize).chain(crate::generation::fire_cells(f)).filter_map(|i|gas.component_at.get(i).copied()).collect();
    for center in forest_anchors{for at in disk(f,center,3){if blocked.contains(&at)||!matches!(f.tiles[at].as_str(),"floor"|"grass"|"tree"){continue}forest.insert(at);if f.tiles[at]=="floor"{if unsafe_components.contains(&gas.component_at[at]){continue}f.tiles[at]="grass".into();}if !occupied.contains(&at)&&changes<64&&rng.random()<0.12&&{changes+=1;safe_block(f,at,"tree",revision)}{}else if f.tiles[at]=="grass"&&rng.random()<0.45{flowers.push(at);}}}
    flowers.sort_unstable();flowers.dedup();forest.retain(|i|matches!(f.tiles[*i].as_str(),"grass"|"tree"));blocked.extend(forest.iter().copied());union_theme(f,"forest",forest,flowers);
    let corn_proposed=f.depth>=crate::flavor_placement::bounds("horn-5").unwrap().0 as f64&&rng.random()<CORN_CHANCE;
    if f.depth>=25.0 {
        let blocked=protected(f,ground);let mut walls:Vec<_>=(0..n).filter(|&i|f.tiles[i]=="wall"&&!blocked.contains(&i)&&i%w>1&&i%w+2<w&&i/w>1&&i/w+2<f.height()&&cardinal(i,w,n).any(|j|matches!(f.tiles[j].as_str(),"floor"|"grass"))).collect();shuffle(&mut walls,&mut rng);
        // Six isolated wall openings per floor; replacement adds routes, never
        // removes an existing walking line. Fixtures and outer walls are excluded.
        for at in walls.into_iter().take(6){f.tiles[at]="floor".into();f.hellfire.as_mut().unwrap()[at]=true;}
        crate::environment::initialize(f)?;
    }
    if corn_proposed { blocked.extend(protected(f,ground));install_corn(seed,f,actors,ground,next_id,revision,&blocked,&mut rng)?; }
    for (at,old) in original_tiles.iter().enumerate(){if *old!=f.tiles[at] { if let Some(light)=f.lights.get_mut(at){*light=(-99f64).max(-2.0*(f.depth-1.0)+if f.tiles[at]=="wall"{-35.0}else if f.tiles[at]=="chasm"{-45.0}else{0.0});}}}
    Ok(())
}
fn install_corn(seed:&str,f:&mut Floor,actors:&mut Vec<Actor>,ground:&[GroundItem],next_id:&mut f64,revision:f64,blocked:&HashSet<usize>,rng:&mut GenerationRng)->Result<(),String>{
    let w=f.width();let n=f.tiles.len();let mut centers:Vec<_>=(0..n).filter(|&i|i%w>CORN_RADIUS as usize&&i%w+CORN_RADIUS as usize+1<w&&i/w>CORN_RADIUS as usize&&i/w+CORN_RADIUS as usize+1<f.height()&&matches!(f.tiles[i].as_str(),"floor"|"grass")).collect();shuffle(&mut centers,rng);
    let occupied_unsafe:HashSet<_>=actors.iter().map(|a|a.y as usize*w+a.x as usize).filter(|i|!matches!(f.tiles[*i].as_str(),"floor"|"grass")).collect();
    let reached=reachable(f);let candidates:Vec<_>=centers.into_iter().take(SHAPE_ATTEMPTS).map(|c|disk(f,c,CORN_RADIUS)).filter(|cells|cells.iter().all(|i|!blocked.contains(i)&&!occupied_unsafe.contains(i))&&cells.iter().any(|i|reached[*i])&&cells.iter().filter(|i|matches!(f.tiles[**i].as_str(),"floor"|"grass")).count()>=16).take(4).collect();
    for cells in candidates {
        let mut draft=f.clone();let mut residents=actors.clone();let mut id=*next_id;
        // A circular planar overlay never changes fixtures or occupied aquatic
        // terrain. Existing walk routes can only gain open cells at this stage.
        for &at in &cells{draft.tiles[at]="floor".into();if let Some(v)=draft.currents.as_mut(){v[at]=0.0;}}
        let occupied:HashSet<_>=residents.iter().map(|a|a.y as usize*w+a.x as usize).chain(ground.iter().map(|g|g.y as usize*w+g.x as usize)).collect();
        let mut slots:Vec<_>=cells.iter().copied().filter(|i|!occupied.contains(i)).collect();shuffle(&mut slots,rng);
        let strongest=if f.depth>=crate::flavor_placement::bounds("horn-7").unwrap().0 as f64{"horn-7"}else{"horn-5"};
        let mut count=residents.iter().filter(|a|cells.contains(&(a.y as usize*w+a.x as usize))&&a.creature_kind.as_ref().is_some_and(|k|k.starts_with("horn-"))).count();
        let mut has_strong=residents.iter().any(|a|cells.contains(&(a.y as usize*w+a.x as usize))&&a.creature_kind.as_ref().is_some_and(|k|k==strongest));
        // Existing Corns are not detached from mixed hordes. Explicit extras
        // fill the chamber to eight and retain their ordinary authored hearts.
        let eligible:Vec<_>=(1..=7).map(|n|format!("horn-{n}")).filter(|k|crate::flavor_placement::eligible(k,f.depth as u32)).collect();
        while count<CORN_RESIDENTS||!has_strong {let Some(at)=slots.pop()else{break};let kind=if !has_strong {strongest.to_string()}else{eligible[(rng.random()*eligible.len()as f64).floor()as usize].clone()};
            let mut a=crate::population::create_actor_revision(&kind,id,(at%w)as f64,(at/w)as f64,false,revision)?;id+=1.0;a.conditions.insert("cornDimensionResident".into(),1.0);has_strong|=kind==strongest;residents.push(a);count+=1;}
        if count<CORN_RESIDENTS||!has_strong{continue}
        draft.pocket_biomes.get_or_insert_with(Vec::new).push(PocketBiome{theme:"corn-dimension".into(),cells:cells.clone(),..Default::default()});
        crate::portals_generation::install(seed,&mut draft,&residents,ground,revision);
        ne::install(seed,&mut draft,&residents,ground,revision);
        install_links(seed,&mut draft,&residents,ground,revision);
        let pairs=draft.portals.as_ref().into_iter().flatten().filter(|p|cells.contains(&p.a)&&cells.contains(&p.b)).count();
        let speed=draft.speed_up.as_ref().into_iter().flatten().filter(|p|p.cells.iter().all(|i|cells.contains(i))).count();
        let random=draft.non_euclidean.as_ref().into_iter().flatten().filter(|p|p.cells.iter().all(|i|cells.contains(i))).count();
        if pairs>=3&&speed>=1&&random>=1&&ne::traversable(&draft){*f=draft;*actors=residents;*next_id=id;return Ok(())}
    }
    // Reject the whole optional biome if its characteristic density or walking
    // proof cannot be met. No discarded residents, rewards or geometry survive.
    Ok(())
}
fn blocked_links(f:&Floor,actors:&[Actor],ground:&[GroundItem])->HashSet<usize>{let w=f.width();protected(f,ground).into_iter().chain(actors.iter().map(|a|a.y as usize*w+a.x as usize)).chain(f.portals.as_ref().into_iter().flatten().flat_map(|p|[p.a,p.b])).chain(f.non_euclidean.as_ref().into_iter().flatten().flat_map(|p|p.cells.iter().copied())).chain(f.speed_up.as_ref().into_iter().flatten().flat_map(|p|p.cells.iter().copied())).collect()}
/// Consecutive pairs along a straight run preserve forward order. Each step
/// skips one cell in either direction, doubling ordinary corridor traversal.
fn ordered_line(cells:Vec<usize>)->Pocket{let mut sorted=cells.clone();sorted.sort_unstable();let groups=cells.chunks_exact(2).map(|pair|Group{sources:vec![pair[0]],destination:pair[1]}).collect();Pocket{cells:sorted,groups}}
fn add_speedup(f:&mut Floor,allowed:&HashSet<usize>,rng:&mut GenerationRng,wanted:usize){
    let w=f.width();let n=f.tiles.len();let mut lines=vec![];let mut starts:Vec<_>=allowed.iter().copied().collect();starts.sort_unstable();for start in starts{for step in [1,w]{for len in [10,8,6,4]{let cells:Vec<_>=(0..len).map(|j|start+j*step).collect();if cells.iter().any(|i|*i>=n||!allowed.contains(i))||step==1&&cells.last().unwrap()/w!=start/w{continue}let hall=cells.iter().filter(|&&i|cardinal(i,w,n).filter(|&j|walkable(&f.tiles[j])).count()<=2).count();lines.push((hall,cells));break;}}}shuffle(&mut lines,rng);lines.sort_by_key(|(hall,_)|std::cmp::Reverse(*hall));
    let mut accepted=0;for (_,cells)in lines.into_iter().take(96){if accepted>=wanted{break}if cells.iter().any(|&i|ne::group_at(f,i).is_some()){continue}f.speed_up.get_or_insert_with(Vec::new).push(ordered_line(cells));if ne::traversable(f){accepted+=1;}else{f.speed_up.as_mut().unwrap().pop();}}
}
pub fn install_links(seed:&str,f:&mut Floor,actors:&[Actor],ground:&[GroundItem],revision:f64){
    if revision<37.0||f.depth<20.0||!f.speed_up.is_missing(){return}f.speed_up=vec![].into();let mut rng=crate::population::population_random(&format!("{seed}:floor:{}:sub-biome-links-r37",f.depth));
    let corn=f.pocket_biomes.as_ref().into_iter().flatten().find(|b|b.theme=="corn-dimension").map(|b|b.cells.clone());
    if let Some(cells)=corn.as_ref(){crate::non_euclidean_generation::append_in_cells(f,actors,ground,cells,3,&mut rng);crate::portals_generation::append_in_cells(f,actors,ground,cells,4,&mut rng);}
    let blocked=blocked_links(f,actors,ground);let eligible:HashSet<_>=(0..f.tiles.len()).filter(|i|!blocked.contains(i)&&matches!(f.tiles[*i].as_str(),"floor"|"grass"|"shallow-water")).collect();
    if let Some(cells)=corn.as_ref(){let local:HashSet<_>=cells.iter().copied().filter(|i|eligible.contains(i)).collect();add_speedup(f,&local,&mut rng,2);}
    let blocked=blocked_links(f,actors,ground);let outside:HashSet<_>=eligible.into_iter().filter(|i|!blocked.contains(i)&&corn.as_ref().is_none_or(|c|c.binary_search(i).is_err())).collect();add_speedup(f,&outside,&mut rng,2);
    if revision >= 51.0 { ne::install_speedup_terrain(f); }
}
