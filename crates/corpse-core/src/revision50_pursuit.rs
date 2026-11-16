//! Revision 50 Relentless Pursuit. Planning is pure; the caller owns legal
//! movement edges, action scheduling, and payment after successful admission.
use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

pub const ID: &str = "pelian-relentless-pursuit";
pub const COST: f64 = 10.0;

/// Serialized target identity, not a stale destination. Rebuild the route using
/// the target's current cell before each command and sight-display preview.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Pursuit {
    pub target_id: f64,
    pub depth: f64,
    pub last_bonus_turn: Option<f64>,
}
impl Pursuit {
    pub fn new(target_id: f64, depth: f64) -> Self {
        Self { target_id, depth, last_bonus_turn: None }
    }
    /// Call only after a voluntary move commits along the route's first edge.
    /// A bonus movement cannot earn another bonus within the same world turn.
    /// Pass the scheduling turn captured before movement, not an animation tick.
    pub fn claim_bonus(&mut self, turn: f64, followed_first_edge: bool) -> bool {
        if !turn.is_finite() || !followed_first_edge || self.last_bonus_turn == Some(turn) {
            return false;
        }
        self.last_bonus_turn = Some(turn);
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    /// Includes source and target. No route means no movement bonus.
    pub cells: Vec<usize>,
    pub crosses_other_enemy: bool,
}

/// Two BFS passes, O(V+E) time and O(V) scratch for a supplied movement graph.
/// Neighbors must already honor walls, pits/flight, corner restrictions, event
/// solids, portal semantics, Clay Soldiers, and non-enemy occupied cells.
/// Never choose a shorter enemy-crossing route when ANY enemy-free route exists.
/// Enemy passage is a route permission only; actual combat/collision remains the
/// engine's responsibility, not an implicit teleport or movement-through actor.
pub fn route(
    revision: f64,
    neighbors: &[Vec<usize>],
    occupied_by_other_enemy: &[bool],
    source: usize,
    target: usize,
) -> Option<Route> {
    let n = neighbors.len();
    if revision < 50.0 || !revision.is_finite() || n == 0
        || occupied_by_other_enemy.len() != n || source >= n || target >= n {
        return None;
    }
    for allow_enemies in [false, true] {
        let mut previous = vec![usize::MAX; n];
        previous[source] = source;
        let mut queue = VecDeque::from([source]);
        while let Some(at) = queue.pop_front() {
            if at == target { break; }
            for &next in &neighbors[at] {
                if next >= n || previous[next] != usize::MAX
                    || (!allow_enemies && next != target && occupied_by_other_enemy[next]) {
                    continue;
                }
                previous[next] = at;
                queue.push_back(next);
            }
        }
        if previous[target] != usize::MAX {
            let mut cells = vec![target];
            let mut at = target;
            while at != source { at = previous[at]; cells.push(at); }
            cells.reverse();
            let crosses_other_enemy = cells.iter().any(|&i| i != source && i != target && occupied_by_other_enemy[i]);
            return Some(Route { cells, crosses_other_enemy });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn long_clear_route_beats_short_enemy_route() {
        let g = vec![vec![1,2],vec![4],vec![3],vec![4],vec![]];
        let r = route(50., &g, &[false,true,false,false,false],0,4).unwrap();
        assert_eq!(r.cells,vec![0,2,3,4]); assert!(!r.crosses_other_enemy);
    }
    #[test]
    fn enemy_route_only_as_fallback() {
        let g=vec![vec![1],vec![2],vec![]];
        let r=route(50.,&g,&[false,true,false],0,2).unwrap();
        assert_eq!(r.cells,vec![0,1,2]); assert!(r.crosses_other_enemy);
        assert!(route(49.,&g,&[false,true,false],0,2).is_none());
    }
    #[test]
    fn target_motion_and_obstacle_changes_are_replanned() {
        let mut g=vec![vec![1],vec![0,2],vec![1,3],vec![2]];
        assert_eq!(route(50.,&g,&[false;4],0,3).unwrap().cells,vec![0,1,2,3]);
        assert_eq!(route(50.,&g,&[false;4],0,2).unwrap().cells,vec![0,1,2]);
        g[1].clear(); assert!(route(50.,&g,&[false;4],0,3).is_none());
    }
    #[test]
    fn bonus_is_once_per_turn_and_survives_serialization() {
        let mut p=Pursuit::new(9.,2.);
        assert!(!p.claim_bonus(1.,false)); assert!(p.claim_bonus(1.,true));
        let mut q: Pursuit=serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert!(!q.claim_bonus(1.,true)); assert!(q.claim_bonus(2.,true));
    }
}

pub const TARGET_KEY: &str = "r50PursuitTarget";
pub const DEPTH_KEY: &str = "r50PursuitDepth";
pub const BONUS_KEY: &str = "r50PursuitBonusTick";
pub fn condition_key(key: &str) -> bool { matches!(key,TARGET_KEY|DEPTH_KEY|BONUS_KEY) }

pub(crate) fn selected_target(r: &crate::Run) -> Option<&crate::Actor> {
    if !crate::revision50::enabled(r) || !r.player.abilities.iter().any(|s|s==ID)
        || r.player.conditions.get(DEPTH_KEY).copied()!=Some(r.depth) {return None;}
    let id=*r.player.conditions.get(TARGET_KEY)?;
    r.actors_at(r.depth).iter().find(|a|a.id==id && a.hp>0. && !a.dead.value_or(false)
        && a.state!="ally" && a.state!="neutral" && crate::dreamwalking::same_plane(r,&r.player,a))
}

/// Admission and payment are atomic. An inaccessible target may still be marked;
/// a later obstacle change can make the route available without a second charge.
pub fn activate(r: &mut crate::Run, target_id: f64) -> bool {
    if !crate::revision50::enabled(r) || !r.player.abilities.iter().any(|s|s==ID)
        || !target_id.is_finite() || crate::abilities::energy_activation_silenced(r,&r.player) {
        return false;
    }
    let Some(a)=r.actors_at(r.depth).iter().find(|a|a.id==target_id) else{return false;};
    if a.id==r.player.id || a.state=="ally" || a.state=="neutral" || a.dead.value_or(false)
        || a.hp<=0. || !crate::dreamwalking::same_plane(r,&r.player,a)
        || !crate::vision::visible_actor(r,a) {return false;}
    if !crate::revision50_ability_energy::spend(r,r.player.id,ID,COST) {return false;}
    r.player.conditions.insert(TARGET_KEY.into(),target_id);
    r.player.conditions.insert(DEPTH_KEY.into(),r.depth);
    // Preserve an already-used tick when retargeting, preventing extra credits.
    crate::engine::log(r,"You draw a path of Relentless Pursuit.","player");
    true
}

/// Remove expired target identity without clearing the once-per-turn credit.
pub fn settle(r: &mut crate::Run) {
    if crate::revision50::enabled(r) && selected_target(r).is_none() {
        r.player.conditions.shift_remove(TARGET_KEY);
        r.player.conditions.shift_remove(DEPTH_KEY);
    }
}

struct RunRoute {
    field: crate::movement::DistanceField,
    source: crate::movement::Point,
}

/// Uses the core's ordered/stochastic portal-aware distance field. Enemy masks
/// are tried strictly before fallback masks, never mixed into a shorter route.
/// O(V*(A*P+C)+field(V,L)) time and O(V+L) scratch, where P is alias lookup,
/// C terrain/contact checks and L portal links. Field cost is documented in
/// movement.rs; stochastic link construction can be larger than the base grid.
fn run_field(r: &crate::Run) -> Option<RunRoute> {
    use crate::movement::Point;
    let target=selected_target(r)?;
    let f=r.floor()?;
    let w=f.width();
    let source=Point{x:r.player.x as i32,y:r.player.y as i32};
    let source_cell=crate::portals::cell(f,r.player.x,r.player.y)?;
    let target_cell=crate::portals::cell(f,target.x,target.y)?;
    if w==0 || f.tiles.len()>3600 {return None;}
    let phase=r.player.active.iter().any(|s|s=="etill-phase-through");
    let flight=crate::travel::flying(r,&r.player);
    let mut terrain=vec![false;f.tiles.len()];
    let mut enemy=vec![false;f.tiles.len()];
    for at in 0..f.tiles.len() {
        let (x,y)=((at%w)as f64,(at/w)as f64);
        terrain[at]=crate::engine::actor_passable(r,&r.player,x,y)
            && !crate::engine::bulwark_blocks(r,&r.player,x,y)
            && crate::ichor::movement_allowed(r,&r.player,x,y)
            && (!crate::dreamwalking::active(r) || !matches!(f.tiles[at].as_str(),"up"|"down"|"pit"));
        for a in r.actors_at(r.depth).iter().filter(|a|a.hp>0. && !a.dead.value_or(false)
            && crate::dreamwalking::same_plane(r,&r.player,a)) {
            if a.id!=target.id && crate::portals::same_position(r,a,x,y) {
                if a.state=="ally" || a.state=="neutral" {terrain[at]=false;}
                else {enemy[at]=true;}
            }
        }
    }
    terrain[source_cell]=true;
    let links=crate::portals::links(f);
    let ordered=crate::portals::ordered_pairs(f);
    for allow_enemies in [false,true] {
        let mut allowed=terrain.clone();
        if !allow_enemies {for (allow,occupied) in allowed.iter_mut().zip(&enemy) {*allow &= !occupied;}}
        allowed[source_cell]=true;
        if !allowed[target_cell] {continue;}
        let field=crate::movement::distance_field_ordered(&f.tiles,w,
            Point{x:target.x as i32,y:target.y as i32},phase,flight,Some(&allowed),None,&links,&ordered);
        if field.steps[source_cell]>=0 {return Some(RunRoute{field,source});}
    }
    None
}

/// Current visible route coordinates include source, movement entry cells and
/// deterministic portal arrivals. Stop preview at a stochastic fork rather than
/// pretend one random exit is guaranteed. Replanning resumes from the actual exit.
pub fn route_for_run(r: &crate::Run) -> Vec<(f64,f64)> {
    let Some(plan)=run_field(r) else{return Vec::new();};
    let Some(f)=r.floor() else{return Vec::new();};
    let mut from=plan.source;
    let mut result=vec![(from.x as f64,from.y as f64)];
    let mut visited=vec![false;f.tiles.len()];
    for _ in 0..f.tiles.len() {
        let at=from.y as usize*f.width()+from.x as usize;
        if visited[at] || plan.field.steps[at]==0 {break;}
        visited[at]=true;
        let Some(d)=crate::movement::next_step(&plan.field,&f.tiles,from) else{break;};
        let entry=crate::movement::Point{x:from.x+d.x,y:from.y+d.y};
        result.push((entry.x as f64,entry.y as f64));
        let entry_at=entry.y as usize*f.width()+entry.x as usize;
        let exits=crate::portals::walking_exits(f,at,entry_at);
        if exits.len()!=1 {break;}
        from=crate::portals::point(f,exits[0]);
        if from!=entry {result.push((from.x as f64,from.y as f64));}
    }
    result
}

/// Scheduler hook, called only for a successful explicit player Move command,
/// before advancing the world tick. No clone of the Run is made by this helper.
/// Return true to schedule ONE movement-only action. Attacks, failed steps,
/// forced movement, falls to another floor and confused deviations earn none.
pub fn grant_for_move(before: &crate::Run, after: &mut crate::Run, dx: f64, dy: f64) -> bool {
    if !crate::revision50::enabled(before) || !crate::revision50::enabled(after)
        || before.depth!=after.depth || after.player.dead.value_or(false) || after.player.hp<=0.
        || !before.tick.is_finite() || after.player.conditions.get(BONUS_KEY).copied()==Some(before.tick)
        || crate::engine::condition(&before.player,"forcedMove")!=0.
        || (before.player.x==after.player.x && before.player.y==after.player.y)
        || !dx.is_finite() || !dy.is_finite() || dx.fract()!=0. || dy.fract()!=0.
        || dx.abs().max(dy.abs())!=1. {return false;}
    let Some(plan)=run_field(before) else{return false;};
    let Some(f)=before.floor() else{return false;};
    let Some(step)=crate::movement::next_step(&plan.field,&f.tiles,plan.source) else{return false;};
    if (dx,dy)!=(step.x as f64,step.y as f64) {return false;}
    let options=crate::portals::step_options(before,&before.player,before.player.x+dx,before.player.y+dy);
    if !options.contains(&(after.player.x,after.player.y)) {return false;}
    after.player.conditions.insert(BONUS_KEY.into(),before.tick);
    crate::animation_player::moved(after,&before.player,&after.player,"P12","speed");
    true
}

#[cfg(test)]
mod runtime_tests {
    use super::*;
    fn run()->crate::Run {
        let mut r:crate::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=50.0.into();r.player.abilities=vec![ID.into()];r.player.active.clear();
        r.player.ability_reserves=indexmap::IndexMap::from([(ID.into(),crate::revision50_ability_energy::authored_reserve(10.,0.).unwrap())]).into();
        r.player.x=12.;r.player.y=14.;r.player.energy=100.;
        r.player.conditions.insert(TARGET_KEY.into(),4.);
        r.player.conditions.insert(DEPTH_KEY.into(),r.depth);
        {let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.portals=crate::Field::Missing;
        f.non_euclidean=crate::Field::Missing;f.speed_up=crate::Field::Missing;}
        r
    }
    #[test]
    fn runtime_target_follows_movement_and_death() {
        let mut r=run();
        assert_eq!(route_for_run(&r).last(),Some(&(16.,14.)));
        r.actors_at_mut(r.depth)[0].x=18.;
        assert_eq!(route_for_run(&r).last(),Some(&(18.,14.)));
        r.actors_at_mut(r.depth)[0].dead=true.into();
        assert!(route_for_run(&r).is_empty()); settle(&mut r);
        assert!(!r.player.conditions.contains_key(TARGET_KEY));
    }
    #[test]
    fn runtime_move_bonus_is_once_and_not_for_attack_or_wrong_direction() {
        let before=run();let mut after=before.clone();
        assert!(!grant_for_move(&before,&mut after,1.,0.));
        after.player.x+=1.;assert!(grant_for_move(&before,&mut after,1.,0.));
        assert!(!grant_for_move(&before,&mut after,1.,0.));
        let mut wrong=before.clone();wrong.player.y+=1.;
        assert!(!grant_for_move(&before,&mut wrong,0.,1.));
    }
    #[test]
    fn runtime_failed_activation_is_atomic_and_legacy_is_unchanged() {
        let mut r=run();let before=r.clone();assert!(!activate(&mut r,-99.));assert_eq!(r,before);
        r.player.energy=9.;let before=r.clone();assert!(!activate(&mut r,4.));assert_eq!(r,before);
        r.simulation_revision=49.0.into();assert!(route_for_run(&r).is_empty());
    }
    #[test]
    fn runtime_enemy_free_detour_precedes_occupied_shortcut() {
        let mut r=run();let mut enemy=r.actors_at(r.depth)[0].clone();enemy.id=55.;enemy.x=14.;enemy.y=14.;
        r.actors_at_mut(r.depth).push(enemy);
        let path=route_for_run(&r);assert_eq!(path.last(),Some(&(16.,14.)));
        assert!(!path.contains(&(14.,14.)));
    }
    #[test]
    fn runtime_portal_entry_and_arrival_both_appear_and_earn_credit() {
        let mut r=run();
        {let f=r.floor_mut().unwrap();let w=f.width();
        f.portals=vec![crate::portals_generation::PortalPair{a:14*w+13,b:14*w+15,color_index:0}].into();}
        let path=route_for_run(&r);
        assert_eq!(path,vec![(12.,14.),(13.,14.),(15.,14.),(16.,14.)]);
        let mut after=r.clone();after.player.x=15.;
        assert!(grant_for_move(&r,&mut after,1.,0.));
    }
    #[test]
    fn runtime_fallback_crosses_enemy_only_when_clear_route_is_impossible() {
        let mut r=run();
        {let f=r.floor_mut().unwrap();let w=f.width();f.tiles.fill("wall".into());
        for x in 12..=16 {f.tiles[14*w+x]="floor".into();}}
        let mut blocker=r.actors_at(r.depth)[0].clone();blocker.id=55.;blocker.x=14.;blocker.y=14.;
        r.actors_at_mut(r.depth).push(blocker);
        assert!(route_for_run(&r).contains(&(14.,14.)));
    }
}
