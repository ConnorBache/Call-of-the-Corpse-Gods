//! Revision 33 creature ecology. Hunger is not an AI allegiance or combat state.
//! Normal-world clocks only; feeding grants no health, energy, stats or depletion.
use crate::{Actor, Floor, Run, engine::{self, EngineError, actor, actor_mut_trusted, condition}, movement::{self, Point}};

pub const HUNGER_TURNS: f64 = 100.0;
pub const FEEDING_TURNS: f64 = 10.0;
pub const AGE: &str = "creatureHungerTurns";
const CLOCK: &str = "creatureHungerClock";
const LAST: &str = "creatureFeedingTick";
const COUNT: &str = "creatureFeedingTurns";
const SOURCE: &str = "creatureFeedingSource";
const KIND: &str = "creatureFeedingKind";
const X: &str = "creatureFeedingX";
const Y: &str = "creatureFeedingY";
const INTERRUPTED: &str = "creatureFeedingInterruptedAt";
pub const CONDITIONS: &[&str] = &[AGE, CLOCK, LAST, COUNT, SOURCE, KIND, X, Y, INTERRUPTED];
pub fn enabled(r: &Run) -> bool { r.simulation_revision.value_or(29.0) >= 33.0 }
fn living_npc(r: &Run, a: &Actor) -> bool {
    a.id != r.player.id && !crate::ahvamitt::is_actor(a) && !crate::dreamwalking::is_nightmare(a) && !a.dead.value_or(false) && a.hp > 0.0
        && !crate::half_here::is_clone(a) && condition(a, "miniMe") != 1.0
        && !(crate::revision50::enabled(r) && crate::revision50_nightmares::is_decoy(a))
}
pub fn hungry(r: &Run, a: &Actor) -> bool {
    enabled(r) && living_npc(r, a) && condition(a, AGE) >= HUNGER_TURNS
}
pub fn bloody_wall(f: &Floor, at: usize) -> bool {
    f.tiles.get(at).is_some_and(|t| t == "wall")
        && f.bloody_walls.as_ref().and_then(|v| v.get(at)).copied().unwrap_or(false)
}
/// Cosmetic crimson water and ground blood do not qualify. No healing is added.
pub fn drinkable(f: &Floor, at: usize) -> bool {
    crate::water_blood::water_cell(f, at)
        && f.water_blood.as_ref().and_then(|v| v.get(at)).copied().unwrap_or(0.0) >= crate::water_blood::CUTOFF
}
pub fn interrupt(a: &mut Actor) {
    for key in [LAST, COUNT, SOURCE, KIND, X, Y] { a.conditions.shift_remove(key); }
}
pub fn damage_interrupt(r: &mut Run, id: f64) {
    if !enabled(r) || actor(r,id).is_none_or(|a| !living_npc(r,a)) { return; }
    let in_turn = r.resolving_tick.value_or(false);
    let tick = r.tick + if in_turn { 0.0 } else { 1.0 };
    if let Some(a) = actor_mut_trusted(r, id) {
        interrupt(a);
        a.conditions.insert(INTERRUPTED.into(), tick);
    }
}
/// One O(A + W*S) clock pass per simulated floor/world turn, not per haste
/// action; W sleepers needing a wake check use the existing stat helper cost S.
/// Old revision saves never acquire ecology keys. Missing r33 keys begin at zero.
pub fn begin_turn(r: &mut Run) {
    if !enabled(r) { return; }
    let tick = r.tick;
    let updates: Vec<_> = r.actors_at(r.depth).iter().map(|a| {
        if !living_npc(r,a) || a.conditions.get(CLOCK)==Some(&tick) { return None; }
        let age=(condition(a,AGE)+1.0).min(HUNGER_TURNS);
        let wake=age>=HUNGER_TURNS && crate::ai::sleeping(a)
            && a.energy>=crate::combat::effective_stats(r,a).max_energy;
        Some((age,wake))
    }).collect();
    for (a,update) in r.actors_slice_mut(r.depth).iter_mut().zip(updates) {
        let Some((age,wake))=update else {continue};
        a.conditions.insert(AGE.into(),age);
        a.conditions.insert(CLOCK.into(),tick);
        if wake {crate::ai::wake(a,false);}
        if a.conditions.get(INTERRUPTED).is_some_and(|t| *t < tick) {a.conditions.shift_remove(INTERRUPTED);}
    }
}

fn autonomous(r: &Run, a: &Actor) -> bool {
    if !hungry(r,a) || crate::ai::sleeping(a) || condition(a,"paralyzed") > 0.0
        || condition(a,"monsterInventorFleeing") > 0.0 || condition(a,"monsterRetreating") > 0.0 || condition(a,"rangedRecovery") > 0.0
        || a.last_seen.as_ref().is_some() || crate::travel::has_ally_stair_follow(a)
        || a.conditions.get(INTERRUPTED).is_some_and(|t| *t >= r.tick) { return false; }
    if a.state == "ally" {
        !(a.perfect.value_or(false) && a.order.as_ref().is_some_and(|s|s=="p"))
            && !crate::ai::should_wake_ally(r,a)
    } else { a.state == "wandering" }
}
fn point(f: &Floor, at: usize) -> Point { Point{x:(at%f.width()) as i32,y:(at/f.width()) as i32} }
fn source_at(f: &Floor, at: usize) -> Option<f64> {
    if bloody_wall(f,at) {Some(1.0)} else if drinkable(f,at) {Some(2.0)} else {None}
}
fn accessible_source(f: &Floor, from: usize, source: usize) -> bool {
    if source >= f.tiles.len() { return false; }
    let a=point(f,from); let b=point(f,source);
    (a.x-b.x).abs().max((a.y-b.y).abs()) <= 1
        && (from != source || drinkable(f,source))
        && movement::diagonal_step_allowed(&f.tiles,f.width(),a,b,false)
}
fn adjacent_source(f: &Floor, at: usize) -> Option<(usize,f64)> {
    if drinkable(f,at) { return Some((at,2.0)); }
    let p=point(f,at);
    for d in movement::DIRECTIONS {
        let (x,y)=(p.x+d.x,p.y+d.y);
        if x<0 || y<0 || x>=f.width() as i32 || y>=f.height() as i32 {continue;}
        let i=y as usize*f.width()+x as usize;
        if accessible_source(f,at,i) { if let Some(kind)=source_at(f,i) {return Some((i,kind));} }
    }
    None
}
fn safe_cell(r: &Run, a: &Actor, i: usize) -> bool {
    let f=r.floor().unwrap(); let p=point(f,i);
    engine::actor_passable(r,a,p.x as f64,p.y as f64)
        && crate::ichor::habitat_path_allowed(f,a,i)
        && !engine::bulwark_blocks(r,a,p.x as f64,p.y as f64)
        && (f.tiles[i] != "chasm" || crate::environment::is_flying_in(r,a))
        && !crate::environment::creature_hazard_at(r,a,i)
}
/// At most two ordinary distance fields, never one path search per food cell.
/// O(N*H + A + N) time/O(N) scratch: H is existing safe-cell helper cost; the
/// hazard-free field is BFS. Occupancy is indexed once, not scanned per cell.
fn food_step(r: &mut Run, a: &Actor) -> Option<Point> {
    if a.abilities.iter().any(|s|s=="enemy:beast:rooted") || crate::ichor::stranded(r,a) {return None;}
    let f=r.floor()?; let n=f.tiles.len(); let w=f.width();
    let from=Point{x:a.x as i32,y:a.y as i32};
    let mut allowed:Vec<_>=(0..n).map(|i|safe_cell(r,a,i) && !crate::revision50_terrain::clay_blocks_run(r,a,(i%w)as f64,(i/w)as f64) && !crate::revision50_control_amulets::obstruction_blocks(r,a,(i%w)as f64,(i/w)as f64)).collect();
    for b in std::iter::once(&r.player).chain(r.actors_at(r.depth)) {
        if b.id!=a.id && !b.dead.value_or(false) && (!crate::portals::enabled(r) || crate::dreamwalking::same_plane(r,a,b)) && b.x>=0.0 && b.y>=0.0 && b.x<w as f64 && b.y<f.height() as f64 {
            let at=b.y as usize*w+b.x as usize;allowed[at]=false;
            if crate::portals::enabled(r) {for alias in crate::portals::aliases(f,at) {allowed[alias]=false;}}
        }
    }
    let phase=a.active.iter().any(|s|s=="etill-phase-through");
    let flight=crate::environment::is_flying_in(r,a);
    let links=if crate::portals::enabled(r) {crate::portals::links(f)} else {vec![]};
    let ordered=if r.simulation_revision.value_or(29.)>=37. {crate::portals::ordered_pairs(f)} else {vec![]};
    let from_field=movement::distance_field_ordered(&f.tiles,w,from,phase,flight,Some(&allowed),None,&links,&ordered);
    let goal=(0..n).filter(|&i|allowed[i] && from_field.steps[i]>0 && adjacent_source(f,i).is_some())
        .min_by_key(|&i|(from_field.steps[i],from_field.diagonals[i],i));
    let result=goal.and_then(|goal| {
        let to_field=movement::distance_field_ordered(&f.tiles,w,point(f,goal),phase,flight,Some(&allowed),None,&links,&ordered);
        movement::next_step(&to_field,&f.tiles,from)
    });
    *r.metrics.entry("pathFields".into()).or_default() += if goal.is_some() {2.0} else {1.0};
    result
}
/// Invoked only after perception, before idle wandering. Combat/flee/rally wins.
/// A Slow creature may eat every normal turn while its travel remains Slow.
pub fn take_turn(r: &mut Run, id: f64, may_move: bool, haste: bool) -> Result<bool,EngineError> {
    if !enabled(r) {return Ok(false);}
    let Some(a)=actor(r,id).cloned() else {return Ok(false)};
    if crate::revision50::enabled(r) && crate::revision50_nightmares::is_decoy(&a) {return Ok(false);}
    if !autonomous(r,&a) { if let Some(a)=actor_mut_trusted(r,id) {interrupt(a);} return Ok(false); }
    let f=r.floor().unwrap(); let at=a.y as usize*f.width()+a.x as usize;
    if !safe_cell(r,&a,at) {interrupt(actor_mut_trusted(r,id).unwrap());return Ok(false);}
    let prior=a.conditions.get(SOURCE).copied().map(|v|v as usize).filter(|&i|accessible_source(f,at,i))
        .and_then(|i|source_at(f,i).map(|k|(i,k)));
    let source=prior.or_else(||adjacent_source(f,at));
    if let Some((source,kind))=source {
        if haste {
            let holding=condition(&a,COUNT)>0.0 && a.conditions.get(X)==Some(&a.x) && a.conditions.get(Y)==Some(&a.y)
                && a.conditions.get(SOURCE)==Some(&(source as f64));
            if !holding {interrupt(actor_mut_trusted(r,id).unwrap());}
            return Ok(holding);
        }
        if a.conditions.get(LAST)==Some(&r.tick) {return Ok(true);}
        let consecutive=a.conditions.get(LAST)==Some(&(r.tick-1.0))
            && a.conditions.get(SOURCE)==Some(&(source as f64)) && a.conditions.get(KIND)==Some(&kind)
            && a.conditions.get(X)==Some(&a.x) && a.conditions.get(Y)==Some(&a.y);
        let count=if consecutive {condition(&a,COUNT)+1.0} else {1.0};
        let tick=r.tick; let a=actor_mut_trusted(r,id).unwrap();
        if count>=FEEDING_TURNS {interrupt(a);a.conditions.insert(AGE.into(),0.0);}
        else {for (k,v) in [(LAST,tick),(COUNT,count),(SOURCE,source as f64),(KIND,kind),(X,a.x),(Y,a.y)] {a.conditions.insert(k.into(),v);}}
        return Ok(true);
    }
    interrupt(actor_mut_trusted(r,id).unwrap());
    if haste || !may_move {return Ok(false);}
    if let Some(d)=food_step(r,&a) {
        engine::move_actor(r,id,d.x as f64,d.y as f64,true)?;
        return Ok(true);
    }
    Ok(false)
}
/// Clear stale activity after special actions, forced movement, paralysis or damage.
pub fn end_turn(r: &mut Run) {
    if !enabled(r) {return;}
    let ids:Vec<_>=r.actors_at(r.depth).iter().filter(|a|condition(a,COUNT)>0.0).map(|a|a.id).collect();
    for id in ids {
        let a=actor(r,id).unwrap();
        let f=r.floor().unwrap();let at=a.y as usize*f.width()+a.x as usize;
        let source_valid=a.conditions.get(SOURCE).is_some_and(|v| accessible_source(f,at,*v as usize)
            && source_at(f,*v as usize)==a.conditions.get(KIND).copied());
        let invalid=!autonomous(r,a) || a.conditions.get(LAST)!=Some(&r.tick) || !source_valid || !safe_cell(r,a,at)
            || a.conditions.get(X)!=Some(&a.x) || a.conditions.get(Y)!=Some(&a.y);
        if invalid {interrupt(actor_mut_trusted(r,id).unwrap());}
    }
}
pub fn description(r: &Run,a:&Actor)->Option<&'static str> {
    if !hungry(r,a) {return None;}
    if condition(a,COUNT)>0.0 && autonomous(r,a) && a.conditions.get(X)==Some(&a.x) && a.conditions.get(Y)==Some(&a.y) {
        Some(if condition(a,KIND)==1.0 {"Eating"} else {"Drinking"})
    } else {Some("Hungering")}
}
/// A separate deterministic decoration stream, O(N), no geometry/RNG changes.
/// Roughly one in twelve accessible interior walls; guarantee one if any exist.
pub fn decorate_floor(f:&mut Floor, seed:u32) {
    let w=f.width(); let h=f.height(); let mut marked=vec![false;f.tiles.len()];let mut first=None;
    for y in 1..h.saturating_sub(1) {for x in 1..w.saturating_sub(1) {
        let i=y*w+x;
        if f.tiles[i]!="wall" || ![i-1,i+1,i-w,i+w].into_iter().any(|j|matches!(f.tiles[j].as_str(),"floor"|"grass"|"shallow-water"|"water")) {continue;}
        first.get_or_insert(i);
        let mut hash=(seed ^ (i as u32).wrapping_mul(0x9e3779b9)).wrapping_mul(0x85ebca6b);
        hash^=hash>>16; marked[i]=hash%12==0;
    }}
    if !marked.iter().any(|v|*v) {if let Some(i)=first {marked[i]=true;}}
    if marked.iter().any(|v|*v) {f.bloody_walls=marked.into();}
}
