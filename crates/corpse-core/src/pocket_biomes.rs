//! Localized, connected monster habitats. No portals or new terrain rules.
use crate::{Actor, Floor, GroundItem};
use serde::{Deserialize,Serialize};
use std::collections::{BTreeSet,HashMap,HashSet,VecDeque};
#[derive(Debug,Clone,Default,PartialEq,Serialize,Deserialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct PocketBiome {
    pub theme: String,
    pub cells: Vec<usize>,
    pub rooms: Vec<usize>,
    #[serde(default,skip_serializing_if="Vec::is_empty")]
    pub flowers: Vec<usize>,
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub queen_cell: Option<usize>,
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub honey_cell: Option<usize>,
    #[serde(default,skip_serializing_if="Vec::is_empty")]
    pub inhabitants: Vec<usize>,
}
pub fn nest_depth(seed:u32)->u32 {
    let plan=crate::generation::domain_plan_revision(seed,33.0);
    let all=[28,29,31,32,33,34];
    let dry:Vec<_>=all.into_iter().filter(|d|plan[(*d as usize-1)/5].domain!="Archipelago").collect();
    let choices=if dry.is_empty(){&all[..]}else{&dry[..]};
    choices[(crate::rng::mix(seed,333_029)%choices.len() as u32)as usize]
}
pub fn at(f:&Floor,cell:usize)->Option<&PocketBiome> {
    f.pocket_biomes.as_ref()?.iter().find(|b| b.cells.binary_search(&cell).is_ok())
}
pub fn flower_at(f:&Floor,cell:usize)->bool {at(f,cell).is_some_and(|b|b.flowers.binary_search(&cell).is_ok())}
pub fn nest(f:&Floor)->Option<&PocketBiome> {f.pocket_biomes.as_ref()?.iter().find(|b|b.theme=="saidribee-nest")}
pub fn reserved(f:&Floor)->impl Iterator<Item=usize>+'_ {nest(f).into_iter().flat_map(|b|b.cells.iter().copied())}
fn room_list(f:&Floor)->Vec<crate::RoomMetadata> { f.room_graph.as_ref().map(|g|g.rooms.clone()).or_else(||f.dungeon_rooms.as_ref().cloned()).unwrap_or_default() }
fn neighbors(f:&Floor,cell:usize)->impl Iterator<Item=usize>+'_ {
    let w=f.width();let h=f.height();let x=cell%w;let y=cell/w;
    [(0,-1),(1,0),(0,1),(-1,0)].into_iter().filter_map(move |(dx,dy)| {
        let xx=x as i32+dx;let yy=y as i32+dy;
        (xx>=0 && yy>=0 && xx<w as i32 && yy<h as i32).then_some((yy as usize).wrapping_mul(w).wrapping_add(xx as usize))
    })
}
/// Linear BFS on already passable land; never carves cells or changes stairs.
fn field(f:&Floor,start:usize,allowed:&[bool])->(Vec<usize>,Vec<usize>) {
    let n=f.tiles.len();let mut dist=vec![usize::MAX;n];let mut prev=vec![usize::MAX;n];
    if start>=n || !allowed[start] {return (dist,prev);}
    let mut q=VecDeque::from([start]);dist[start]=0;
    while let Some(i)=q.pop_front() { for j in neighbors(f,i) { if allowed[j] && dist[j]==usize::MAX {dist[j]=dist[i]+1;prev[j]=i;q.push_back(j);} } }
    (dist,prev)
}
fn safe(f:&Floor)->Vec<bool> {
    let protected=crate::generation::protected_generation_cells(f);
    let stairs:Vec<_>=f.tiles.iter().enumerate().filter(|(_,t)|matches!(t.as_str(),"up"|"down"|"pit")).map(|(i,_)|i).collect();
    let passable:Vec<_>=f.tiles.iter().map(|t|crate::generation::terrain_passable(t)).collect();
    let reachable=field(f,f.tiles.iter().position(|t|t=="up").unwrap_or(0),&passable).0;
    f.tiles.iter().enumerate().map(|(i,t)| {
        matches!(t.as_str(),"floor"|"grass") && reachable[i]!=usize::MAX && !protected.contains(&i)
            && !f.environment.as_ref().is_some_and(|e| e.fire[i]>0.0 || e.gas_volume.as_ref().is_some_and(|v|v[i]>0.0))
            && [&f.slime,&f.ichor,&f.razorleaves,&f.roots].iter().all(|v|v.as_ref().is_none_or(|v|v[i]==0.0))
            && stairs.iter().copied().all(|s| (s%f.width()).abs_diff(i%f.width()).max((s/f.width()).abs_diff(i/f.width()))>3)
    }).collect()
}
/// Up to R start BFS/candidate sorts and ordered cell sets:
/// O(R*(N log N+sum room cells+R log R)),
/// O(N+sum room cells) scratch. Three-room Nest is bounded to existing geometry.
fn region(f:&Floor,allowed:&[bool],rng:&mut crate::rng::GenerationRng,count:usize,bridge:bool,raised:bool)->Option<PocketBiome> {
    let mut links=allowed.to_vec();
    if bridge {
        let protected=crate::generation::protected_generation_cells(f);
        for (i,t) in f.tiles.iter().enumerate() {
            if matches!(t.as_str(),"water"|"shallow-water") && !protected.contains(&i)
                && f.ichor.as_ref().is_none_or(|v|v[i]==0.0) {links[i]=true;}
        }
    }
    let rooms=room_list(f);let mut choices:Vec<_>=rooms.iter().filter_map(|r| {
        let mut cells:Vec<_>=r.cells.iter().map(|v|*v as usize).filter(|&i|i<allowed.len()&&allowed[i]).collect();
        if raised && !cells.is_empty() {
            // The all-water fallback raises a compact part of each existing
            // footprint, not its whole island/room or the surrounding domain.
            let permitted:HashSet<_>=cells.iter().copied().collect();
            let start=*cells.iter().min_by_key(|&&i|((i%f.width())as i32-r.center.x as i32).abs()+((i/f.width())as i32-r.center.y as i32).abs()).unwrap();
            let mut chosen=vec![start];let mut seen=HashSet::from([start]);let mut head=0;
            while head<chosen.len() && chosen.len()<16 {
                let i=chosen[head];head+=1;
                for j in neighbors(f,i) {if chosen.len()<16 && permitted.contains(&j)&&seen.insert(j){chosen.push(j);}}
            }
            cells=chosen;
        }
        (cells.len()>=8).then_some((r.id as usize,cells))
    }).collect();
    if choices.len()<count {return None;}
    let offset=(rng.random()*choices.len() as f64).floor() as usize;choices.rotate_left(offset);
    for (rid,root) in &choices {
        let start=root[root.len()/2];let (dist,prev)=field(f,start,&links);
        let mut candidates:Vec<_>=choices.iter().filter_map(|(id,cells)| {
            let rep=cells.iter().copied().min_by_key(|i|dist[*i])?;
            (dist[rep]!=usize::MAX).then_some((*id,rep,cells))
        }).collect();
        candidates.sort_by_key(|(id,rep,_)|(*id!=*rid,dist[*rep],*id));
        if candidates.len()<count {continue;}
        candidates.truncate(count);
        let mut cells=BTreeSet::new();let mut selected=vec![];
        for (id,rep,rcells) in candidates {
            selected.push(id);
            cells.extend(rcells.iter().copied().filter(|i|dist[*i]!=usize::MAX));
            let mut i=rep;cells.insert(i);
            while i!=start {i=prev[i];if i==usize::MAX {break;}cells.insert(i);}
        }
        let connected:Vec<_>=(0..allowed.len()).map(|i|cells.contains(&i)).collect();
        let (dist,_)=field(f,start,&connected);
        cells.retain(|i|dist[*i]!=usize::MAX);
        let chamber:Vec<_>=root.iter().copied().filter(|i|cells.contains(i)).collect();
        if chamber.len()<8 || selected.iter().any(|id| !rooms.iter().find(|r|r.id as usize==*id).unwrap().cells.iter().any(|i|cells.contains(&(*i as usize)))) {continue;}
        if cells.iter().filter(|&&i|matches!(f.tiles[i].as_str(),"water"|"shallow-water")).count()>if raised {160}else{96} {continue;}
        let residents:Vec<_>=cells.iter().copied().filter(|i|allowed[*i]&&*i!=chamber[0]&&*i!=chamber[1]).collect();
        let inhabitants=(0..6).map(|i|residents[i*residents.len()/6]).collect();
        return Some(PocketBiome{cells:cells.into_iter().collect(),rooms:selected,queen_cell:Some(chamber[0]),honey_cell:Some(chamber[1]),inhabitants,..Default::default()});
    }
    None
}
/// Independent seed stream. Snow/grass use existing terrain mechanics; flowers
/// themselves are decorative. Caller runs final fire/vent safety afterward.
pub fn decorate(f:&mut Floor,seed:u32)->Result<(),String> { decorate_revision(f,seed,33.0) }
pub fn decorate_revision(f:&mut Floor,seed:u32,revision:f64)->Result<(),String> {
    if !f.pocket_biomes.is_missing() {return if validate(f) {Ok(())} else {Err("Invalid existing pocket biome metadata".into())};}
    if f.godbeast || f.depth==40.0 {return Ok(());}
    let mut allowed=safe(f);
    let mut rng=crate::rng::GenerationRng::new(crate::rng::mix(seed,333_000+f.depth as u32));
    let mut biomes=vec![];
    if f.depth as u32==nest_depth(seed) {
        let planned=region(f,&allowed,&mut rng,3,true,false).or_else(|| {
            if f.domain!="Archipelago" {return None;}
            let mut platforms=allowed.clone();let protected=crate::generation::protected_generation_cells(f);
            for (i,t) in f.tiles.iter().enumerate() {
                if matches!(t.as_str(),"water"|"shallow-water") && !protected.contains(&i)
                    && f.ichor.as_ref().is_none_or(|v|v[i]==0.0) {platforms[i]=true;}
            }
            region(f,&platforms,&mut rng,3,true,true)
        });
        let mut b=planned.ok_or("No connected Saidribee Nest")?;
        b.theme="saidribee-nest".into();
        for &i in &b.cells {
            if matches!(f.tiles[i].as_str(),"water"|"shallow-water") {
                f.tiles[i]="floor".into();f.generation_bridges.get_or_insert_with(Vec::new).push(i as f64);
                if let Some(v)=f.currents.as_mut() {v[i]=0.0;}
            }
        }
        if let Some(v)=f.generation_bridges.as_mut() {v.sort_by(f64::total_cmp);v.dedup();}
        for &i in &b.cells {allowed[i]=false;}
        biomes.push(b);
    }
    // Preserve gas/furnace containment: never introduce new fuel into a source
    // component. Existing vegetation keeps its already-validated fire behavior.
    let gas=crate::generation::build_generation_gas_components(f);
    let mut unsafe_components=HashSet::new();let mut no_new_fuel=false;
    for source in f.vents.as_ref().into_iter().flatten().map(|v|v.at as usize)
        .chain(crate::generation::fire_cells(f).into_iter()) {
        if let Some(&c)=gas.component_at.get(source) {if c>=0 {unsafe_components.insert(c);}}
    }
    for setup in f.setups.as_ref().into_iter().flatten() {
        for source in crate::generation::furnace_origins(setup.id as i32,setup.x as i32,setup.y as i32,f.width()) {
            if source<0 || gas.component_at.get(source as usize).is_none_or(|c|*c<0) {no_new_fuel=true;}
            else {unsafe_components.insert(gas.component_at[source as usize]);}
        }
    }
    let chance=(0.10+f.depth*0.018).min(0.80);
    let themes:Vec<_>=if (17.0..=36.0).contains(&f.depth) {vec!["permafrost","flower-grove"]} else {vec!["flower-grove"]};
    if revision<37.0 && f.depth>=6.0 {for theme in themes {
        if rng.random()>=chance {continue;}
        let count=if f.depth>=24.0 {2} else {1};
        if let Some(mut b)=region(f,&allowed,&mut rng,count,false,false) {
            b.theme=theme.into();b.queen_cell=None;b.honey_cell=None;b.inhabitants.clear();
            for &i in &b.cells {allowed[i]=false;}
            if theme=="permafrost" {let n=f.tiles.len();let snow=f.snow.get_or_insert_with(||vec![false;n]);for &i in &b.cells {snow[i]=true;}}
            else {
                for &i in &b.cells {if f.tiles[i]=="floor" && !no_new_fuel && !unsafe_components.contains(&gas.component_at[i]) {f.tiles[i]="grass".into();}}
                b.flowers=b.cells.iter().copied().filter(|i|f.tiles[*i]=="grass" && crate::rng::mix(seed,*i as u32+f.depth as u32*4000)%3!=0).collect();
                if b.flowers.is_empty() {continue;}
            }
            biomes.push(b);
        }
    }}
    if !biomes.is_empty() {f.pocket_biomes=biomes.into();}
    Ok(())
}
/// Relocate only matching existing individuals; ordinary counts/rewards unchanged.
/// Nest has a bounded authored population: Queen, four workers, two Guardians.
pub fn populate(f:&mut Floor,actors:&mut Vec<Actor>,ground:&mut Vec<GroundItem>,next_id:&mut f64)->Result<(),String> {
    populate_revision(f,actors,ground,next_id,33.)
}
/// Preserve pocket-specific birth stats while applying current explicit traits.
pub fn populate_revision(f:&mut Floor,actors:&mut Vec<Actor>,ground:&mut Vec<GroundItem>,next_id:&mut f64,revision:f64)->Result<(),String> {
    let Some(biomes)=f.pocket_biomes.as_ref().cloned() else {return Ok(())};
    let w=f.width();let mut occupied:HashSet<_>=actors.iter().filter(|a|!a.dead.value_or(false)).map(|a|a.y as usize*w+a.x as usize)
        .chain(ground.iter().map(|g|g.y as usize*w+g.x as usize)).collect();
    let indices_by_id:HashMap<_,_>=actors.iter().enumerate().map(|(i,a)|(a.id.to_bits(),i)).collect();
    for b in &biomes {
        if b.theme=="saidribee-nest" {
            let q=b.queen_cell.ok_or("Nest lacks Queen chamber")?;let h=b.honey_cell.ok_or("Nest lacks Royal Honey")?;
            for (kind,at) in std::iter::once(("saidribee-queen",q)).chain(b.inhabitants.iter().copied().enumerate().map(|(i,c)|(if i<4 {"short-saidri"} else {"saidri-guardian"},c))) {
                if occupied.contains(&at) {return Err("Nest reserved creature cell occupied".into());}
                let mut a=crate::population::create_actor_revision(kind,*next_id,(at%w) as f64,(at/w) as f64,false,33.0)?;
                crate::revision50_combat::mark_new_caster(&mut a,kind,revision);
                if revision>=50.0 {crate::revision50_creature_identity::apply_actor(&mut a);}
                *next_id+=1.0;a.conditions.insert("nestResident".into(),1.0);actors.push(a);occupied.insert(at);
            }
            if occupied.contains(&h) {return Err("Nest honey cell occupied".into());}
            ground.push(GroundItem{x:(h%w) as f64,y:(h/w) as f64,item:crate::royal_honey::item(*next_id),..Default::default()});*next_id+=1.0;occupied.insert(h);
        } else if b.theme=="permafrost" || b.theme=="flower-grove" {
            let kinds:&[&str]=if b.theme=="permafrost" {if f.depth<=26.0 {&["wendigo"]} else {&["yeti"]}} else {&["dryad","bear","tiger","short-saidri","long-saidri"]};
            let mut slots=b.cells.iter().copied().filter(|i|!occupied.contains(i)).collect::<Vec<_>>().into_iter();
            // Never split a horde or relocate a machine/special encounter. Synthetic
            // ungrouped floors use singleton groups, preserving their existing IDs.
            let groups:Vec<Vec<f64>>=if let Some(records)=f.encounters.as_ref() {
                records.iter().filter(|e|e.kind.as_ref().is_some_and(|k|k=="horde")).map(|e|e.actor_ids.clone()).collect()
            } else { actors.iter().map(|a|vec![a.id]).collect() };
            let mut remaining=3;
            for group in groups {
                if group.is_empty() || group.len()>remaining || slots.len()<group.len() {continue;}
                let indices:Vec<_>=group.iter().filter_map(|id|indices_by_id.get(&id.to_bits()).copied()).collect();
                if indices.len()!=group.len() || !indices.iter().all(|&i| {
                    let a=&actors[i];!a.dead.value_or(false) && !a.conditions.contains_key("nestResident")
                        && a.creature_kind.as_ref().is_some_and(|k|kinds.contains(&k.as_str()))
                }) {continue;}
                for i in indices {let at=slots.next().unwrap();let a=&mut actors[i];occupied.remove(&(a.y as usize*w+a.x as usize));a.x=(at%w) as f64;a.y=(at/w) as f64;occupied.insert(at);}
                remaining-=group.len();
            }
        }
    }
    Ok(())
}
pub fn validate(f:&Floor)->bool {
    let Some(v)=f.pocket_biomes.as_ref() else {return f.pocket_biomes.is_missing()};
    if v.len()>6 {return false;}
    let n=f.tiles.len();let mut used=HashSet::new();
    v.iter().all(|b| ["saidribee-nest","permafrost","flower-grove","mountainous","forest","corn-dimension"].contains(&b.theme.as_str())
        && (if ["mountainous","forest","corn-dimension"].contains(&b.theme.as_str()) { (1.0..=40.0).contains(&f.depth) } else { !f.godbeast && (6.0..40.0).contains(&f.depth) })
        && (b.theme!="permafrost" || (17.0..=36.0).contains(&f.depth))
        && b.rooms.iter().copied().collect::<HashSet<_>>().len()==b.rooms.len()
        && !b.cells.is_empty() && b.cells.len()<=n && b.cells.windows(2).all(|p|p[0]<p[1])
        && b.cells.iter().all(|i|*i<n&&used.insert(*i)) && (b.rooms.is_empty() == ["mountainous","forest","corn-dimension"].contains(&b.theme.as_str()) || !b.rooms.is_empty()) && b.rooms.len()<=3
        && b.flowers.windows(2).all(|p|p[0]<p[1]) && b.flowers.iter().all(|i|b.cells.binary_search(i).is_ok())
        && b.queen_cell.into_iter().chain(b.honey_cell).chain(b.inhabitants.iter().copied()).all(|i|b.cells.binary_search(&i).is_ok())
        && if b.theme=="saidribee-nest" {b.rooms.len()==3&&b.queen_cell.is_some()&&b.honey_cell.is_some()&&b.queen_cell!=b.honey_cell&&b.inhabitants.len()==6&&b.queen_cell.into_iter().chain(b.honey_cell).chain(b.inhabitants.iter().copied()).collect::<HashSet<_>>().len()==8&&(28.0..=34.0).contains(&f.depth)&&!f.godbeast}
            else {b.queen_cell.is_none()&&b.honey_cell.is_none()&&b.inhabitants.is_empty()})
}
