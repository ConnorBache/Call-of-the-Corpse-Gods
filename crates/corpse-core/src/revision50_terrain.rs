//! Revision 50 Kative placement planning. No payment or mutation precedes
//! successful admission. These plans are shared by sight preview and commit.
use serde::{Deserialize, Serialize};
use crate::Actor;

pub const COGWORKS: &str = "kative-cogworks";
pub const CLAY_ARMY: &str = "kative-clay-army";
pub const COST: f64 = 20.0;
pub const WALL_HP: f64 = 200.0;
pub const MODE_KEY: &str = "r50CogworksMode";
pub const SOLDIER_KEY: &str = "r50ClaySoldier";

pub fn is_soldier(a:&Actor)->bool {a.conditions.get(SOLDIER_KEY).copied()==Some(1.)}
/// Hard collision overlay: call before any phase-through exception. Uses the
/// moving actor's physical/dream plane, not the currently rendered floor.
pub fn clay_blocks_run(r:&crate::Run,a:&Actor,x:f64,y:f64)->bool {
    if !crate::revision50::enabled(r) {return false;}
    let Some(cell)=selected_cell(x,y) else{return false;};
    crate::dreamwalking::floor_for(r,a).and_then(|f|f.revision50_terrain.as_ref())
        .is_some_and(|mods|clay_blocks(mods,cell))
}

/// One-tile ordinary movement admission, before payment or departure terrain.
/// Portals may be used only when every arrival is exactly the selected tile;
/// otherwise this is not the authored one-tile step.
pub fn clay_preview_for_run(r:&crate::Run,x:f64,y:f64)->Result<Placement,PlacementError> {
    clay_plan_for_run(r,x,y,true)
}
fn clay_plan_for_run(r:&crate::Run,x:f64,y:f64,known_only:bool)->Result<Placement,PlacementError> {
    let to=selected_cell(x,y).ok_or(PlacementError::IllegalDestination)?;
    let from=Cell{x:r.player.x as i32,y:r.player.y as i32};
    let f=crate::dreamwalking::floor_for(r,&r.player).ok_or(PlacementError::IllegalDestination)?;
    let phase=r.player.active.iter().any(|s|s=="etill-phase-through");
    let legal=crate::engine::actor_passable(r,&r.player,x,y)
        && !clay_blocks_run(r,&r.player,x,y)
        && !crate::engine::bulwark_blocks(r,&r.player,x,y)
        && crate::ichor::movement_allowed(r,&r.player,x,y)
        && crate::portals::step_options(r,&r.player,x,y).iter().all(|&p|p==(x,y))
        && !r.actors_at(r.depth).iter().any(|a|a.hp>0. && !a.dead.value_or(false)
            && crate::dreamwalking::same_plane(r,&r.player,a)
            && crate::portals::same_position(r,a,x,y)
            && (!known_only || crate::vision::directly_visible_actor(r,a)))
        && selected_cell(x,y).is_some_and(|c|(c.x as usize)<f.width() && (c.y as usize)<f.height())
        && crate::movement::diagonal_step_allowed(&f.tiles,f.width(),
            crate::movement::Point{x:from.x,y:from.y},crate::movement::Point{x:to.x,y:to.y},phase)
        && !f.revision50_terrain.as_ref().into_iter().flatten().any(|m|m.cells.contains(&from))
        && !raised_wall_at(r,from);
    clay_preview(r.simulation_revision.value_or(29.),from,to,legal)
}

/// Transactional Clay placement: movement and terrain/payment either all commit
/// or restore the opening state. Reuses ordinary movement/hazard rules. COW Run
/// clone cost is documented; this is not a per-frame or per-path-query clone.
pub fn activate_clay(r:&mut crate::Run,x:f64,y:f64)->Result<bool,crate::engine::EngineError> {
    if !crate::revision50::enabled(r) || !r.player.abilities.iter().any(|a|a==CLAY_ARMY)
        || crate::abilities::energy_activation_silenced(r,&r.player)
        || !crate::revision50_ability_energy::can_pay(r,r.player.id,CLAY_ARMY,COST) {return Ok(false);}
    let Ok(plan)=clay_plan_for_run(r,x,y,false) else{return Ok(false);};
    let before=r.clone();
    let presentation=crate::events::Probe::begin();
    let (dx,dy)=(x-r.player.x,y-r.player.y);
    let moved=match crate::engine::move_actor(r,r.player.id,dx,dy,true) {
        Ok(moved)=>moved,Err(error)=>{*r=before;return Err(error);}
    };
    if !moved || r.depth!=before.depth || r.player.x!=x || r.player.y!=y
        || r.player.dead.value_or(false) || r.player.hp<=0. {
        *r=before;return Ok(false);
    }
    let departure=plan.cells[0];
    if r.actors_at(r.depth).iter().any(|a|a.hp>0. && !a.dead.value_or(false)
        && crate::dreamwalking::same_plane(r,&r.player,a)
        && crate::portals::same_position(r,a,departure.x as f64,departure.y as f64)) {
        *r=before;return Ok(false);
    }
    let revision=r.simulation_revision.value_or(29.);
    let mut modifications=r.floor().unwrap().revision50_terrain.value_or_default();
    let mut energy=COST;
    if commit(revision,r.floor_mut().unwrap(),&mut energy,&mut modifications,&plan).is_err() {
        *r=before;return Ok(false);
    }
    r.floor_mut().unwrap().revision50_terrain=modifications.into();
    if !crate::revision50_ability_energy::spend(r,r.player.id,CLAY_ARMY,COST) {*r=before;return Ok(false);}
    crate::engine::log(r,"A Clay Soldier rises where you stood.","player");
    crate::events::at(r,"P63","rise",crate::events::Stage::Special,&plan.cells.iter().map(|c|crate::events::EventPoint{x:c.x as f64,y:c.y as f64}).collect::<Vec<_>>());
    presentation.commit();
    Ok(true)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell { pub x: i32, pub y: i32 }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind { Pit, Wall, ClaySoldier }
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Modification {
    pub kind: Kind,
    pub cells: Vec<Cell>,
    /// Preserve exact previous terrain for later authorized removal. Mode 3
    /// restores this terrain for free, per the designer clarification.
    pub previous_tiles: Vec<String>,
    pub hp: Option<f64>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlacementError {
    LegacyRevision, RemovalRequiresSelection, InvalidDirection, OutOfBounds,
    Occupied, IllegalDestination, InsufficientEnergy, Overlap, InvalidModification,
}

pub fn mode(r: &crate::Run) -> u8 {
    match r.player.conditions.get(MODE_KEY).copied() {Some(2.)=>2,Some(3.)=>3,_=>1}
}
pub fn select_mode(r: &mut crate::Run, selected: f64) -> bool {
    if !crate::revision50::enabled(r) || !r.player.abilities.iter().any(|a|a==COGWORKS)
        || !matches!(selected,1.0|2.0|3.0) {return false;}
    r.player.conditions.insert(MODE_KEY.into(),selected);
    true
}
fn selected_cell(x:f64,y:f64)->Option<Cell> {
    (x.is_finite() && y.is_finite() && x.fract()==0. && y.fract()==0.
        && x>=0. && y>=0. && x<=i32::MAX as f64 && y<=i32::MAX as f64)
        .then_some(Cell{x:x as i32,y:y as i32})
}
/// Knowledge-limited UI query. Hidden occupants must never change the preview.
/// Full occupancy is revalidated separately at the command boundary.
pub fn preview_for_run(r:&crate::Run,x:f64,y:f64)->Result<Placement,PlacementError> {
    if !crate::revision50::enabled(r) {return Err(PlacementError::LegacyRevision);}
    let center=selected_cell(x,y).ok_or(PlacementError::OutOfBounds)?;
    let f=crate::dreamwalking::floor_for(r,&r.player).ok_or(PlacementError::OutOfBounds)?;
    if mode(r)==3 {
        let m=f.revision50_terrain.as_ref().into_iter().flatten().find(|m|m.kind!=Kind::ClaySoldier && m.cells.contains(&center))
            .ok_or(PlacementError::InvalidModification)?;
        return Ok(Placement{kind:m.kind,cells:m.cells.clone()});
    }
    cogworks_preview(r.simulation_revision.value_or(29.),mode(r),
        Cell{x:r.player.x as i32,y:r.player.y as i32},center,f.width(),f.height(),|cell| {
            crate::portals::same_position(r,&r.player,cell.x as f64,cell.y as f64)
            || r.actors_at(r.depth).iter().any(|a|a.hp>0. && !a.dead.value_or(false)
                && crate::dreamwalking::same_plane(r,&r.player,a)
                && crate::vision::directly_visible_actor(r,a)
                && crate::portals::same_position(r,a,cell.x as f64,cell.y as f64))
        })
}

/// Run-level pit/wall/free-removal command. The abilities dispatcher already enters
/// the player's dream/physical plane. All failures precede terrain/payment edits.
/// floor_mut uses the mutation journal's COW access path. Optical and lighting
/// caches compare exact terrain dependencies on their next query, including
/// same-tick changes, so no tick-only or ad hoc cache reset is required.
pub fn activate_cogworks(r:&mut crate::Run,x:f64,y:f64)->Result<bool,crate::engine::EngineError> {
    if !crate::revision50::enabled(r) || !r.player.abilities.iter().any(|a|a==COGWORKS) {return Ok(false);}
    let Some(center)=selected_cell(x,y) else{return Ok(false);};
    let selected=mode(r);
    let revision=r.simulation_revision.value_or(29.);
    let Some(f)=r.floor() else{return Ok(false);};
    let mut modifications=f.revision50_terrain.value_or_default();
    if validate_modifications(f,&modifications).is_err() {return Ok(false);}
    if selected==3 {
        if !modifications.iter().any(|m|m.kind!=Kind::ClaySoldier && m.cells.contains(&center)) {return Ok(false);}
        let removed=modifications.iter().find(|m|m.kind!=Kind::ClaySoldier && m.cells.contains(&center)).unwrap();
        let removed_kind=removed.kind;
        let removed_cells=removed.cells.clone();
        // remove performs all saved-data and selection checks before changing a tile.
        if remove(revision,r.floor_mut().unwrap(),&mut modifications,center).is_err() {return Ok(false);}
        r.floor_mut().unwrap().revision50_terrain=modifications.into();
        crate::events::at(r,"P61",if removed_kind==Kind::Pit {"pit-rise"}else{"slab-sink"},crate::events::Stage::Special,&removed_cells.iter().map(|c|crate::events::EventPoint{x:c.x as f64,y:c.y as f64}).collect::<Vec<_>>());
        crate::engine::log(r,"You remove your Cogworks terrain without spending energy.","player");
        return Ok(true);
    }
    if crate::abilities::energy_activation_silenced(r,&r.player)
        || !crate::revision50_ability_energy::can_pay(r,r.player.id,COGWORKS,COST) {return Ok(false);}
    let plan=cogworks_preview(revision,selected,Cell{x:r.player.x as i32,y:r.player.y as i32},center,
        f.width(),f.height(),|cell| {
            crate::portals::same_position(r,&r.player,cell.x as f64,cell.y as f64)
            || r.actors_at(r.depth).iter().any(|a|a.hp>0. && !a.dead.value_or(false)
                && crate::dreamwalking::same_plane(r,&r.player,a)
                && crate::portals::same_position(r,a,cell.x as f64,cell.y as f64))
        });
    let Ok(plan)=plan else{return Ok(false);};
    // A horn's independent expiry must not later overwrite the persistent wall.
    if plan.cells.iter().any(|&c|raised_wall_at(r,c)) {return Ok(false);}
    let mut checked_energy=COST;
    if commit(revision,r.floor_mut().unwrap(),&mut checked_energy,&mut modifications,&plan).is_err() {return Ok(false);}
    r.floor_mut().unwrap().revision50_terrain=modifications.into();
    // Payment has no fallible admission left: the same finite energy was checked
    // by commit above, and no actor mutation occurs during that terrain operation.
    let paid=crate::revision50_ability_energy::spend(r,r.player.id,COGWORKS,COST);
    debug_assert!(paid);
    crate::events::at(r,"P61",if selected==2 {"slab"} else {"pit-sink"},crate::events::Stage::Special,&plan.cells.iter().map(|c|crate::events::EventPoint{x:c.x as f64,y:c.y as f64}).collect::<Vec<_>>());
    crate::engine::log(r,if selected==2 {"You create a Cogworks wall with 200 shared HP."} else {"You create a Cogworks pit."},"player");
    Ok(true)
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement { pub kind: Kind, pub cells: Vec<Cell> }

/// O(1) geometry: at most 9 cells. Full footprints must fit, never truncate at
/// map boundaries. Cardinal and exact diagonal throw directions have an exact
/// integer-grid perpendicular; other vectors require a designer rounding rule.
pub fn cogworks_preview(
    revision: f64, mode: u8, origin: Cell, center: Cell,
    width: usize, height: usize,
    mut occupied: impl FnMut(Cell) -> bool,
) -> Result<Placement, PlacementError> {
    if !revision.is_finite() || revision < 50. { return Err(PlacementError::LegacyRevision); }
    let mut cells=Vec::new();
    let kind=match mode {
        1 => {
            for dy in -1..=1 { for dx in -1..=1 {
                cells.push(Cell{x:center.x.checked_add(dx).ok_or(PlacementError::OutOfBounds)?,y:center.y.checked_add(dy).ok_or(PlacementError::OutOfBounds)?});
            }}
            Kind::Pit
        },
        2 => {
            let dx=i64::from(center.x)-i64::from(origin.x);
            let dy=i64::from(center.y)-i64::from(origin.y);
            if (dx==0 && dy==0) || (dx!=0 && dy!=0 && dx.abs()!=dy.abs()) {
                return Err(PlacementError::InvalidDirection);
            }
            let (px,py)=(-dy.signum() as i32,dx.signum() as i32);
            for k in -2..=2 {
                cells.push(Cell{x:center.x.checked_add(px*k).ok_or(PlacementError::OutOfBounds)?,y:center.y.checked_add(py*k).ok_or(PlacementError::OutOfBounds)?});
            }
            Kind::Wall
        },
        3 => return Err(PlacementError::RemovalRequiresSelection),
        _ => return Err(PlacementError::InvalidDirection),
    };
    for &c in &cells {
        if c.x<0 || c.y<0 || c.x as usize>=width || c.y as usize>=height {return Err(PlacementError::OutOfBounds);}
        // Both footprints are atomic and cannot entomb an occupied creature.
        if occupied(c) {return Err(PlacementError::Occupied);}
    }
    Ok(Placement{kind,cells})
}

/// Pure Clay Army movement admission. Caller supplies the SAME legality check as
/// ordinary player movement, including collision, plane and diagonal corners.
/// Never convert the departure cell until the voluntary step actually commits.
pub fn clay_preview(
    revision: f64, from: Cell, to: Cell, legal_step: bool,
) -> Result<Placement, PlacementError> {
    if !revision.is_finite() || revision<50. {return Err(PlacementError::LegacyRevision);}
    let dx=(i64::from(to.x)-i64::from(from.x)).abs();
    let dy=(i64::from(to.y)-i64::from(from.y)).abs();
    if dx.max(dy)!=1 || !legal_step {return Err(PlacementError::IllegalDestination);}
    Ok(Placement{kind:Kind::ClaySoldier,cells:vec![from]})
}

/// Clay walls are unbreakable even to phase-through; caller must apply this
/// collision overlay before ordinary terrain/phase checks and enemy pathfinding.
pub fn clay_blocks(modifications: &[Modification], cell: Cell) -> bool {
    modifications.iter().any(|m|m.kind==Kind::ClaySoldier && m.cells.contains(&cell))
}
/// Adjacent means the same eight-cell melee neighborhood as ordinary attacks.
/// The combat dispatcher selects creatures/planes and resolves each attack.
pub fn adjacent(soldier: Cell, creature: Cell) -> bool {
    let dx=(i64::from(soldier.x)-i64::from(creature.x)).abs();
    let dy=(i64::from(soldier.y)-i64::from(creature.y)).abs();
    dx.max(dy)==1
}

/// Detached live basic attacker. Numeric effective stats are current, but no
/// player ability, active power, equipment grant or attack-trigger condition is
/// copied. The shared greatsword helper matches the player's ordinary melee base.
pub fn clay_attacker(r:&crate::Run,cell:Cell,id:f64,dream:bool)->Actor {
    let stats=crate::combat::effective_stats(r,&r.player);
    clay_attacker_with_stats(r,cell,id,dream,stats)
}
fn clay_attacker_with_stats(r:&crate::Run,cell:Cell,id:f64,dream:bool,stats:crate::Stats)->Actor {
    let mut a=Actor{id,x:cell.x as f64,y:cell.y as f64,name:"Clay Soldier".into(),
        glyph:"▓".into(),color:r.player.color.clone(),hp:1.,max_hp:1.,energy:0.,
        state:"ally".into(),stats:stats.clone(),..Default::default()};
    a.conditions.insert(SOLDIER_KEY.into(),1.);
    a.conditions.insert("phantasmiiPlane".into(),if dream{1.}else{0.});
    // Nonplayer ordinary attacks add s.offense once; avoid double counting.
    a.conditions.insert("weaponDamage".into(),crate::revision50_player_attack::with_offense_for_run(r,stats.offense)-stats.offense);
    a.conditions.insert("weaponOffenseMultiplier".into(),1.);
    a
}

fn with_clay_source(
    r:&mut crate::Run,cell:Cell,dream:bool,player_stats:Option<&crate::Stats>,
    attack:impl FnOnce(&mut crate::Run,f64)->Result<(),crate::engine::EngineError>,
)->Result<(),crate::engine::EngineError> {
    crate::creature_index::check_next_creature_id(r).map_err(crate::engine::EngineError)?;
    let source_depth=r.depth;
    let id=r.next_id;r.next_id+=1.;
    let source=match player_stats {
        Some(stats)=>clay_attacker_with_stats(r,cell,id,dream,stats.clone()),
        None=>clay_attacker(r,cell,id,dream),
    };
    r.actors_at_mut(source_depth).push(source);
    let result=attack(r,id);
    // Reactions can change player depth. Cleanup follows source ownership.
    r.actors_at_mut(source_depth).retain(|a|a.id!=id);
    result
}

/// Stable floor/cell/actor ordering and normal combat RNG. Refresh live stats
/// before EVERY attack. Sources get fresh IDs and are removed even on error.
/// Call once per completed world turn, never per haste/movement-only phase.
pub fn clay_end_turn(r:&mut crate::Run)->Result<(),crate::engine::EngineError> {
    clay_end_turn_inner(r,None)
}
/// Offscreen proxy has no live player/inventory. Caller provides current actual
/// player effective stats before building that proxy, never its stripped stats.
pub fn clay_end_turn_with_player_stats(r:&mut crate::Run,stats:&crate::Stats)->Result<(),crate::engine::EngineError> {
    clay_end_turn_inner(r,Some(stats))
}
fn clay_end_turn_inner(r:&mut crate::Run,player_stats:Option<&crate::Stats>)->Result<(),crate::engine::EngineError> {
    if !crate::revision50::enabled(r) || r.status!="playing" {return Ok(());}
    let turn_depth=r.depth;
    for dream in [false,true] {
        if r.depth!=turn_depth || r.status!="playing" {break;}
        let cells:Vec<Cell>=crate::dreamwalking::floor_in_plane(r,dream)
            .and_then(|f|f.revision50_terrain.as_ref()).into_iter().flatten()
            .filter(|m|m.kind==Kind::ClaySoldier).flat_map(|m|m.cells.iter().copied()).collect();
        if cells.is_empty() {continue;}
        crate::dreamwalking::with_plane(r,dream,|r|->Result<(),crate::engine::EngineError>{
            for cell in cells {
                if r.depth!=turn_depth || r.status!="playing" {break;}
                let targets:Vec<f64>=std::iter::once(&r.player).chain(r.actors_at(r.depth))
                    .filter(|a|a.hp>0. && !a.dead.value_or(false) && !is_soldier(a)
                        && crate::dreamwalking::dream_actor(r,a)==dream
                        && adjacent(cell,Cell{x:a.x as i32,y:a.y as i32}))
                    .map(|a|a.id).collect();
                for target in targets {
                    if r.depth!=turn_depth || r.status!="playing" {break;}
                    let Some(victim)=crate::engine::actor(r,target) else{continue;};
                    if victim.dead.value_or(false) || victim.hp<=0.
                        || !adjacent(cell,Cell{x:victim.x as i32,y:victim.y as i32}) {continue;}
                    with_clay_source(r,cell,dream,player_stats,|r,id|crate::engine::attack(r,id,target,1.))?;
                }
            }
            Ok(())
        })?;
    }
    Ok(())
}

/// Commit a previously admitted footprint. Revalidate after movement/selection
/// before charging: O(M*K+K), with K<=9 cells and M stored modifications.
/// Caller checks ownership, plane, current occupancy and ability availability.
/// Paint from this same stored footprint using the catalog's Kative colors.
pub fn commit(
    revision: f64, floor: &mut crate::Floor, energy: &mut f64,
    modifications: &mut Vec<Modification>, placement: &Placement,
) -> Result<(), PlacementError> {
    if !revision.is_finite() || revision<50. {return Err(PlacementError::LegacyRevision);}
    if !energy.is_finite() || *energy<COST {return Err(PlacementError::InsufficientEnergy);}
    let expected=match placement.kind {Kind::Pit=>9,Kind::Wall=>5,Kind::ClaySoldier=>1};
    if placement.cells.len()!=expected {return Err(PlacementError::InvalidModification);}
    let mut indices=Vec::new();
    for &c in &placement.cells {
        if c.x<0 || c.y<0 || c.x as usize>=floor.width() || c.y as usize>=floor.height() {
            return Err(PlacementError::OutOfBounds);
        }
        let i=c.y as usize*floor.width()+c.x as usize;
        if i>=floor.tiles.len() || indices.contains(&i) {return Err(PlacementError::InvalidModification);}
        if modifications.iter().any(|m|m.cells.contains(&c)) {return Err(PlacementError::Overlap);}
        if floor.altered_chasms.as_ref().is_some_and(|m|m.contains_key(&i.to_string()))
            || floor.revision50_temporary_terrain.as_ref().is_some_and(|v|v.iter().any(|t|t.cell==i)) {
            return Err(PlacementError::Overlap);
        }
        indices.push(i);
    }
    let previous_tiles=indices.iter().map(|&i|floor.tiles[i].clone()).collect();
    let tile=if placement.kind==Kind::Pit {"chasm"} else {"wall"};
    for i in indices {floor.tiles[i]=tile.into();}
    modifications.push(Modification{kind:placement.kind,cells:placement.cells.clone(),previous_tiles,
        hp:if placement.kind==Kind::Wall {Some(WALL_HP)}else{None}});
    *energy-=COST;
    floor.dirty=true;
    Ok(())
}

/// Free Mode 3 removal. Select only a modification created by this ability.
/// Reject malformed saved data before mutation. No energy parameter by design.
/// O(M*K+K) search/validation, O(K) scratch, K<=9 authored cells.
pub fn remove(
    revision: f64, floor: &mut crate::Floor,
    modifications: &mut Vec<Modification>, selected: Cell,
) -> Result<(), PlacementError> {
    if !revision.is_finite() || revision<50. {return Err(PlacementError::LegacyRevision);}
    let Some(index)=modifications.iter().position(|m|m.cells.contains(&selected)) else {
        return Err(PlacementError::InvalidModification);
    };
    let m=&modifications[index];
    if m.cells.len()!=m.previous_tiles.len() {return Err(PlacementError::InvalidModification);}
    let mut indices=Vec::new();
    for c in &m.cells {
        if c.x<0 || c.y<0 || c.x as usize>=floor.width() || c.y as usize>=floor.height() {
            return Err(PlacementError::InvalidModification);
        }
        let i=c.y as usize*floor.width()+c.x as usize;
        if i>=floor.tiles.len() {return Err(PlacementError::InvalidModification);}
        indices.push(i);
    }
    for (i,tile) in indices.into_iter().zip(&m.previous_tiles) {
        floor.tiles[i]=tile.clone();
        crate::ichor::enqueue_opening_revision(floor,i,revision);
    }
    modifications.remove(index);
    floor.dirty=true;
    Ok(())
}

fn raised_wall_at(r:&crate::Run,cell:Cell)->bool {
    let Some(f)=crate::dreamwalking::floor_for(r,&r.player) else{return false};
    let Some(at)=crate::portals::cell(f,cell.x as f64,cell.y as f64) else{return false};
    r.actors_at(r.depth).iter().any(|a|crate::engine::condition(a,"raisedWallExpires")>0.
        && a.conditions.get("raisedWallIndex")==Some(&(at as f64))
        && a.conditions.get("raisedWallPlane").copied().map(|p|p==1.)
            .unwrap_or_else(||crate::dreamwalking::dream_actor(r,a))==crate::dreamwalking::context_dream(r))
}

/// Boundary validation covers physical and dream floors, including restored
/// temporal snapshots. O(F + M*K), O(M*K) cells; K<=9. Reject, never repair.
pub fn validate_run(r:&crate::Run)->Result<(),String> {
    for f in r.floors.iter().chain(r.dreamwalking.as_ref().into_iter().flat_map(|s|s.floors.iter())) {
        if matches!(f.revision50_terrain,crate::Field::Null) {return Err("Invalid null Cogworks terrain state".into());}
        if let Some(mods)=f.revision50_terrain.as_ref() {
            if !crate::revision50::enabled(r) {return Err("Cogworks terrain requires revision 50".into());}
            validate_modifications(f,mods).map_err(|_|"Invalid Cogworks terrain state".to_string())?;
        }
    }
    Ok(())
}
fn validate_modifications(f:&crate::Floor,mods:&[Modification])->Result<(),PlacementError> {
    let mut seen=std::collections::HashSet::new();
    for m in mods {
        let expected=match m.kind {Kind::Pit=>9,Kind::Wall=>5,Kind::ClaySoldier=>1};
        if m.cells.len()!=expected || m.previous_tiles.len()!=expected
            || if m.kind==Kind::Wall {m.hp.is_none_or(|hp|!hp.is_finite()||hp<=0.||hp>WALL_HP)} else {m.hp.is_some()} {
            return Err(PlacementError::InvalidModification);
        }
        for (c,previous) in m.cells.iter().zip(&m.previous_tiles) {
            let Some(at)=crate::portals::cell(f,c.x as f64,c.y as f64) else{return Err(PlacementError::InvalidModification)};
            let tile=if m.kind==Kind::Pit {"chasm"} else {"wall"};
            if !seen.insert(at) || !crate::storage::TILES.contains(&previous.as_str()) || f.tiles[at]!=tile
                || f.altered_chasms.as_ref().is_some_and(|v|v.contains_key(&at.to_string()))
                || f.revision50_temporary_terrain.as_ref().is_some_and(|v|v.iter().any(|t|t.cell==at)) {
                return Err(PlacementError::InvalidModification);
            }
        }
        let min_x=m.cells.iter().map(|c|c.x).min().unwrap();
        let min_y=m.cells.iter().map(|c|c.y).min().unwrap();
        let max_x=m.cells.iter().map(|c|c.x).max().unwrap();
        let max_y=m.cells.iter().map(|c|c.y).max().unwrap();
        let shape=match m.kind {
            Kind::Pit=>max_x-min_x==2 && max_y-min_y==2,
            Kind::ClaySoldier=>true,
            Kind::Wall=>[(0,1),(1,0),(1,1),(1,-1)].into_iter().any(|(dx,dy)| {
                let start=Cell{x:min_x,y:if dy<0 {max_y}else{min_y}};
                (0..5).all(|n|m.cells.contains(&Cell{x:start.x+n*dx,y:start.y+n*dy}))
            }),
        };
        if !shape {return Err(PlacementError::InvalidModification);}
    }
    Ok(())
}

pub fn breakable_at(r:&crate::Run,a:&Actor,x:f64,y:f64)->bool {
    if !crate::revision50::enabled(r) {return false;}
    let Some(cell)=selected_cell(x,y) else{return false;};
    crate::dreamwalking::floor_for(r,a).and_then(|f|f.revision50_terrain.as_ref())
        .is_some_and(|mods|mods.iter().any(|m|m.kind==Kind::Wall && m.cells.contains(&cell)))
}
fn wall_enemy(r:&crate::Run,a:&Actor)->bool {
    crate::revision50::enabled(r) && a.id!=r.player.id && a.hp>0. && !a.dead.value_or(false)
        && (!matches!(a.state.as_str(),"ally"|"neutral") || crate::revision50_control_amulets::discordant(r,a))
}
/// A normal hostile movement attempt becomes one terrain strike, never an
/// arrival or creature attack. Caller excludes haste-only/forced movement.
/// Automatic hit, no wall dodge/DR or creature-only effects; damage reuses the
/// ordinary enemy base, serialized variance and revision-aware raw attack profiles and damage rounding.
pub(crate) fn attack_wall(r:&mut crate::Run,a:&Actor,x:f64,y:f64)->Result<bool,crate::engine::EngineError> {
    if !wall_enemy(r,a) || crate::engine::condition(a,"forcedMove")!=0.
        || !breakable_at(r,a,x,y) || crate::engine::actor_passable(r,a,x,y) {return Ok(false);}
    let f=r.floor().unwrap();
    if (a.x-x).abs().max((a.y-y).abs())!=1.
        || !crate::movement::diagonal_step_allowed(&f.tiles,f.width(),
            crate::movement::Point{x:a.x as i32,y:a.y as i32},crate::movement::Point{x:x as i32,y:y as i32},false)
        || crate::engine::bulwark_blocks(r,a,x,y) {return Ok(false);}
    validate_modifications(f,f.revision50_terrain.as_ref().unwrap())
        .map_err(|_|crate::engine::EngineError("Invalid Cogworks wall state".into()))?;
    if crate::monster_statuses::try_hemophilia_episode(r,a.id).map_err(crate::engine::EngineError)? {return Ok(true);}
    let stats=crate::combat::effective_stats(r,a);
    let base=crate::combat::creature_attack_base(r.simulation_revision.value_or(29.), a, stats.offense);
    let variance=0.75+crate::rng::random(r)*0.5;
    let damage=crate::combat::ordinary_strike_damage_for_run(r,base,variance,0.,false,false);
    let revision = r.simulation_revision.value_or(29.);
    let damaged_wall=damage_wall_revision(r.floor_mut().unwrap(),Cell{x:x as i32,y:y as i32},damage,revision)
        .map_err(|_|crate::engine::EngineError("Invalid Cogworks wall state".into()))?;
    // A real positive hit chips only the observed struck material. No attacker
    // identity, neighboring wall footprint, speculative damage or extra RNG.
    if damaged_wall && damage>0. {
        crate::events::at(r,"P61","fracture-chip",crate::events::Stage::Impact,&[crate::events::EventPoint{x,y}]);
    }
    if crate::vision::visible_actor(r,a) {
        crate::engine::log(r,format!("{} strikes the Cogworks wall for {damage} damage.",crate::actor_presentation::name(r,a)),"enemy");
    }
    Ok(true)
}
/// Only after ordinary pursuit has no route, allow a hostile route through
/// breakable Cogworks cells. Other walls, Clay, habitat and hazard rules stay
/// unchanged. O(V*(M*K+P+B+H)+field(V,L)), O(V+L) scratch, no RNG/cache.
pub(crate) fn breach_step(r:&crate::Run,a:&Actor,target:crate::movement::Point)->Option<crate::movement::Point> {
    if !wall_enemy(r,a) {return None;}
    let f=crate::dreamwalking::floor_for(r,a)?;
    let mods=f.revision50_terrain.as_ref()?;
    if !mods.iter().any(|m|m.kind==Kind::Wall) {return None;}
    let mut tiles=f.tiles.clone();
    for m in mods.iter().filter(|m|m.kind==Kind::Wall) {for c in &m.cells {
        if let Some(at)=crate::portals::cell(f,c.x as f64,c.y as f64) {tiles[at]="floor".into();}
    }}
    let phase=a.active.iter().any(|id|id=="etill-phase-through");
    let allowed:Vec<_>=(0..tiles.len()).map(|at| {
        let (x,y)=((at%f.width()) as f64,(at/f.width()) as f64);
        !clay_blocks_run(r,a,x,y) && !crate::revision50_control_amulets::obstruction_blocks(r,a,x,y)
            && crate::ichor::habitat_path_allowed(f,a,at)
            && (phase||!f.grates.as_ref().is_some_and(|g|g.contains(&(at as f64))))
            && !crate::engine::bulwark_blocks(r,a,x,y)
    }).collect();
    let harmful:Vec<_>=(0..tiles.len()).map(|at|crate::environment::creature_hazard_at(r,a,at)).collect();
    let field=crate::movement::distance_field_ordered(&tiles,f.width(),target,phase,
        crate::engine::flying_in(r,a),Some(&allowed),Some(&harmful),
        &if crate::portals::enabled(r){crate::portals::links(f)}else{vec![]},
        &if r.simulation_revision.value_or(29.)>=37.{crate::portals::ordered_pairs(f)}else{vec![]});
    crate::movement::next_step(&field,&tiles,crate::movement::Point{x:a.x as i32,y:a.y as i32})
}

/// One HP pool belongs to the entire five-cell object. A damaging hit on any
/// cell updates that pool; destruction restores every previous tile atomically.
/// Caller selects the physical/dream floor. O(M*K), O(M*K) cloned records on hit.
pub fn damage_wall(f:&mut crate::Floor,cell:Cell,damage:f64)->Result<bool,PlacementError> { damage_wall_revision(f,cell,damage,50.) }
pub fn damage_wall_revision(f:&mut crate::Floor,cell:Cell,damage:f64,revision:f64)->Result<bool,PlacementError> {
    if !damage.is_finite() || damage<0. {return Err(PlacementError::InvalidModification);}
    let Some(mods)=f.revision50_terrain.as_ref() else{return Ok(false)};
    let Some(index)=mods.iter().position(|m|m.kind==Kind::Wall && m.cells.contains(&cell)) else{return Ok(false)};
    validate_modifications(f,mods)?;
    if damage==0. {return Ok(true);}
    let mut mods=mods.clone();
    let hp=mods[index].hp.unwrap();
    if damage>=hp {remove(revision,f,&mut mods,cell)?;} else {mods[index].hp=Some(hp-damage);}
    f.revision50_terrain=mods.into();f.dirty=true;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn c(x:i32,y:i32)->Cell{Cell{x,y}}
    #[test]
    fn pit_requires_all_nine_cells_unoccupied() {
        let p=cogworks_preview(50.,1,c(1,1),c(4,4),10,10,|_|false).unwrap();
        assert_eq!(p.cells.len(),9);
        assert_eq!(cogworks_preview(50.,1,c(1,1),c(4,4),10,10,|p|p==c(5,5)),Err(PlacementError::Occupied));
        assert_eq!(cogworks_preview(50.,1,c(1,1),c(0,0),10,10,|_|false),Err(PlacementError::OutOfBounds));
    }
    #[test]
    fn wall_is_five_cells_perpendicular_to_throw() {
        let p=cogworks_preview(50.,2,c(1,4),c(4,4),10,10,|_|false).unwrap();
        assert_eq!(p.cells,vec![c(4,2),c(4,3),c(4,4),c(4,5),c(4,6)]);
        let p=cogworks_preview(50.,2,c(4,1),c(4,4),10,10,|_|false).unwrap();
        assert!(p.cells.iter().all(|c|c.y==4));
        assert_eq!(p.cells.len(),5); assert_eq!(WALL_HP,200.);
    }
    #[test]
    fn legacy_and_removal_without_selection_are_rejected() {
        assert_eq!(cogworks_preview(49.,1,c(1,1),c(4,4),10,10,|_|false),Err(PlacementError::LegacyRevision));
        assert_eq!(cogworks_preview(50.,3,c(1,1),c(4,4),10,10,|_|false),Err(PlacementError::RemovalRequiresSelection));
    }
    #[test]
    fn clay_requires_committable_one_cell_move() {
        assert_eq!(clay_preview(50.,c(4,4),c(5,5),true).unwrap().cells,vec![c(4,4)]);
        assert!(clay_preview(50.,c(4,4),c(6,4),true).is_err());
        assert!(clay_preview(50.,c(4,4),c(5,4),false).is_err());
        assert!(adjacent(c(4,4),c(5,5))); assert!(!adjacent(c(4,4),c(4,4)));
    }
    #[test]
    fn clay_attacks_use_live_stats_without_abilities() {
        let mut r=run();r.player.abilities.push("anavani-nova".into());
        r.player.active.push("anavani-nova".into());
        r.player.conditions.insert("bloodWeaponReady".into(),1.);
        let first=clay_attacker(&r,c(13,14),88.,false);
        assert!(first.abilities.is_empty());assert!(first.active.is_empty());
        assert!(!first.conditions.contains_key("bloodWeaponReady"));
        assert_eq!(first.conditions["weaponDamage"]+first.stats.offense,crate::revision50_player_attack::base(&r));
        r.player.stats.offense+=5.;r.player.stats.accuracy+=7.;
        let next=clay_attacker(&r,c(13,14),89.,false);
        assert_eq!(next.stats.accuracy,first.stats.accuracy+7.);
        assert_eq!(next.conditions["weaponDamage"]+next.stats.offense,
            first.conditions["weaponDamage"]+first.stats.offense+10.);
    }

    #[test]
    fn paid_commit_and_free_removal_round_trip() {
        let mut r:crate::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let f=r.floor_mut().unwrap(); let before=f.tiles.clone();
        let plan=cogworks_preview(50.,1,c(1,1),c(3,3),f.width(),f.height(),|_|false).unwrap();
        let mut mods=Vec::new(); let mut energy=20.;
        commit(50.,f,&mut energy,&mut mods,&plan).unwrap();
        assert_eq!(energy,0.); assert_eq!(mods.len(),1);
        remove(50.,f,&mut mods,c(3,3)).unwrap();
        assert_eq!(f.tiles,before); assert!(mods.is_empty()); assert_eq!(energy,0.);
    }
    #[test]
    fn rejected_payment_does_not_mutate() {
        let mut r:crate::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let f=r.floor_mut().unwrap(); let before=f.tiles.clone();
        let plan=clay_preview(50.,c(3,3),c(4,3),true).unwrap();
        let mut mods=Vec::new(); let mut energy=19.;
        assert_eq!(commit(50.,f,&mut energy,&mut mods,&plan),Err(PlacementError::InsufficientEnergy));
        assert_eq!(f.tiles,before); assert_eq!(energy,19.); assert!(mods.is_empty());
    }
    fn run()->crate::Run {
        let mut r:crate::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=50.0.into();r.player.abilities=vec![COGWORKS.into()];
        r.player.active.clear();r.player.energy=20.;
        r.player.ability_reserves=indexmap::IndexMap::from([
            (COGWORKS.into(),crate::revision50_ability_energy::authored_reserve(20.,0.).unwrap()),
            (CLAY_ARMY.into(),crate::revision50_ability_energy::authored_reserve(20.,0.).unwrap())]).into();
        r.floor_mut().unwrap().tiles.fill("floor".into());
        r
    }
    #[test]
    fn runtime_pit_and_free_removal_preserve_save_state() {
        let mut r=run();let original=r.floor().unwrap().tiles.clone();
        assert!(activate_cogworks(&mut r,20.,20.).unwrap());assert_eq!(r.player.energy,0.);
        assert_eq!(r.floor().unwrap().revision50_terrain.as_ref().unwrap().len(),1);
        let mut restored:crate::Run=serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert!(select_mode(&mut restored,3.));
        assert!(activate_cogworks(&mut restored,20.,20.).unwrap());
        assert_eq!(restored.player.energy,0.);assert_eq!(restored.floor().unwrap().tiles,original);
    }
    #[test]
    fn runtime_occupied_pit_and_wall_fail_atomically() {
        let mut r=run();let original=r.clone();
        assert!(!activate_cogworks(&mut r,16.,14.).unwrap());assert_eq!(r,original);
        assert!(select_mode(&mut r,2.));let original=r.clone();
        assert!(!activate_cogworks(&mut r,16.,14.).unwrap());assert_eq!(r,original);
    }
    #[test]
    fn hidden_occupant_does_not_change_preview_but_blocks_commit() {
        let mut r=run();
        {let f=r.floor_mut().unwrap();let w=f.width();for y in 0..f.height(){f.tiles[y*w+16]="wall".into();}}
        r.actors_at_mut(r.depth)[0].x=20.;r.actors_at_mut(r.depth)[0].y=14.;
        assert!(!crate::vision::directly_visible_actor(&r,&r.actors_at(r.depth)[0]));
        let preview=preview_for_run(&r,20.,14.).unwrap();
        let mut empty=r.clone();empty.actors_at_mut(r.depth).clear();
        assert_eq!(preview_for_run(&empty,20.,14.).unwrap(),preview);
        let original=r.clone();assert!(!activate_cogworks(&mut r,20.,14.).unwrap());assert_eq!(r,original);
    }
    #[test]
    fn mode_selection_is_free_and_rejects_invalid_values() {
        let mut r=run();assert_eq!(mode(&r),1);
        assert!(select_mode(&mut r,3.));assert_eq!(mode(&r),3);assert_eq!(r.player.energy,20.);
        let original=r.clone();assert!(!select_mode(&mut r,2.5));assert_eq!(r,original);
    }
    #[test]
    fn clay_placement_moves_and_leaves_unbreakable_wall() {
        let mut r=run();r.player.abilities.push(CLAY_ARMY.into());
        let old=(r.player.x,r.player.y);
        assert!(activate_clay(&mut r,old.0+1.,old.1).unwrap());
        assert_eq!((r.player.x,r.player.y),(old.0+1.,old.1));assert_eq!(r.player.energy,0.);
        assert!(clay_blocks_run(&r,&r.player,old.0,old.1));
        r.player.active.push("etill-phase-through".into());
        assert!(clay_blocks_run(&r,&r.player,old.0,old.1));
    }
    #[test]
    fn clay_placement_uses_its_reserve_before_shared_energy() {
        let mut r=run();r.player.abilities.push(CLAY_ARMY.into());r.player.energy=0.;
        r.player.ability_reserves.as_mut().unwrap().get_mut(CLAY_ARMY).unwrap().energy=20.;
        assert!(activate_clay(&mut r,13.,14.).unwrap());assert_eq!(r.player.energy,0.);
        assert_eq!(r.player.ability_reserves.as_ref().unwrap()[CLAY_ARMY].energy,0.);
    }
    #[test]
    fn clay_rejected_move_is_atomic() {
        let mut r=run();r.player.abilities.push(CLAY_ARMY.into());
        let before=r.clone();assert!(!activate_clay(&mut r,16.,14.).unwrap());assert_eq!(r,before);
    }
    #[test]
    fn clay_end_turn_attacks_any_adjacent_creature_and_cleans_sources() {
        let mut r=run();r.player.abilities.push(CLAY_ARMY.into());r.player.stats.accuracy=100.;
        assert!(activate_clay(&mut r,13.,14.).unwrap());
        r.player.hp=1000.;r.player.max_hp=1000.;
        r.actors_at_mut(r.depth)[0].x=11.;r.actors_at_mut(r.depth)[0].y=14.;
        r.actors_at_mut(r.depth)[0].hp=1000.;r.actors_at_mut(r.depth)[0].max_hp=1000.;
        r.actors_at_mut(r.depth)[0].state="ally".into();
        let ids:Vec<_>=r.actors_at(r.depth).iter().map(|a|a.id).collect();
        clay_end_turn(&mut r).unwrap();
        assert!(r.player.hp<1000.);assert!(r.actors_at(r.depth)[0].hp<1000.);
        assert_eq!(r.actors_at(r.depth).iter().map(|a|a.id).collect::<Vec<_>>(),ids);
        assert!(!r.actors_at(r.depth).iter().any(is_soldier));
    }
    #[test]
    fn cogworks_cannot_remove_clay_soldiers() {
        let mut r=run();r.player.abilities.push(CLAY_ARMY.into());
        assert!(activate_clay(&mut r,13.,14.).unwrap());assert!(select_mode(&mut r,3.));
        let before=r.clone();assert!(!activate_cogworks(&mut r,12.,14.).unwrap());
        assert_eq!(r,before);assert!(preview_for_run(&r,12.,14.).is_err());
    }
    #[test]
    fn clay_transient_ids_obey_registry_and_terminal_turns_stop() {
        let mut r=run();r.player.abilities.push(CLAY_ARMY.into());
        assert!(activate_clay(&mut r,13.,14.).unwrap());
        r.next_id=r.actors_at(r.depth)[0].id;
        let before=r.clone();assert!(clay_end_turn(&mut r).is_err());assert_eq!(r,before);
        r.status="dead".into();let before=r.clone();clay_end_turn(&mut r).unwrap();assert_eq!(r,before);
    }
    #[test]
    fn clay_source_cleanup_tracks_original_floor_on_reaction_error() {
        let mut r=run();let depth=r.depth;let next=r.next_id;
        let result=with_clay_source(&mut r,c(12,14),false,None,|r,id| {
            assert_eq!(id,next);assert!(r.actors_at(depth).iter().any(|a|a.id==id));
            r.depth=depth+1.;Err(crate::engine::EngineError("test reaction error".into()))
        });
        assert!(result.is_err());assert_eq!(r.depth,depth+1.);
        assert!(!r.actors_at(depth).iter().any(is_soldier));assert_eq!(r.next_id,next+1.);
    }
    #[test]
    fn clay_preview_hides_invisible_occupants_but_activation_rejects() {
        let mut r=run();r.player.abilities.push(CLAY_ARMY.into());
        let a=&mut r.actors_at_mut(r.depth)[0];a.x=13.;a.y=14.;a.conditions.insert("invisible".into(),10.);
        let preview=clay_preview_for_run(&r,13.,14.).unwrap();
        let mut empty=r.clone();empty.actors_at_mut(r.depth).clear();
        assert_eq!(clay_preview_for_run(&empty,13.,14.).unwrap(),preview);
        let before=r.clone();assert!(!activate_clay(&mut r,13.,14.).unwrap());assert_eq!(r,before);
    }
    #[test]
    fn clay_rejected_runtime_move_preserves_outer_presentation_batch() {
        let mut r=run();r.player.abilities.push(CLAY_ARMY.into());
        r.player.conditions.insert("riptide".into(),1.);
        crate::events::begin();
        crate::events::actor_event(&r,&r.player,"status","outer",None,None);
        let expected=crate::events::finish();assert!(!expected.is_empty());
        crate::events::begin();
        crate::events::actor_event(&r,&r.player,"status","outer",None,None);
        let before=r.clone();assert!(!activate_clay(&mut r,13.,14.).unwrap());
        assert_eq!(r,before);assert_eq!(crate::events::finish(),expected);
    }
    #[test]
    fn clay_grappler_cannot_entomb_dragged_creature_in_departure_cell() {
        let mut r=run();r.player.abilities.extend([CLAY_ARMY.into(),"shrail-grappler".into()]);
        r.player.active.push("shrail-grappler".into());
        r.actors_at_mut(r.depth)[0].x=11.;r.actors_at_mut(r.depth)[0].y=14.;
        let before=r.clone();crate::events::begin();
        assert!(!activate_clay(&mut r,13.,14.).unwrap());assert_eq!(r,before);
        assert!(crate::events::finish().is_empty());
    }
    #[test]
    fn clay_black_pit_departure_overlap_rolls_back_move_and_events() {
        let mut r=run();r.player.abilities.extend([CLAY_ARMY.into(),"grisha-black-pit".into()]);
        r.player.active.push("grisha-black-pit".into());
        let before=r.clone();crate::events::begin();
        assert!(!activate_clay(&mut r,13.,14.).unwrap());assert_eq!(r,before);
        assert!(crate::events::finish().is_empty());
    }
}

#[cfg(test)]
#[path="revision50_wall_tests.rs"]
mod wall_tests;
