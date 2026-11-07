//! Frozen ui-targeting.ts: physical ray geometry with observation gates before
//! every live terrain/door read. Never use the simulation collision trace here.
use corpse_core::{vision, Actor, Run};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point {
    pub x: usize,
    pub y: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StopReason {
    Target,
    Unknown,
    Terrain,
    Creature,
    Invalid,
    /// A physical ray can choose either exit; no selected outcome is previewed.
    Stochastic,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trace {
    pub path: Vec<Point>,
    pub stop_reason: StopReason,
    #[serde(default)]
    pub possible_exits: Vec<Point>,
}
impl Trace {
    fn stopped(path: Vec<Point>, stop_reason: StopReason) -> Self {
        Self { path, stop_reason, possible_exits: vec![] }
    }
}
fn observe_with(
    r: &Run,
    target: Option<Point>,
    mask: &[bool],
    observable: impl Fn(&Actor) -> bool,
) -> Trace {
    let Some(target) = target else {
        return Trace::stopped(vec![], StopReason::Invalid);
    };
    let Some(f) = r.floor() else {
        return Trace::stopped(vec![], StopReason::Invalid);
    };
    let width = f.width();
    let height = f.height();
    let p = &r.player;
    if !p.x.is_finite()
        || !p.y.is_finite()
        || p.x.fract() != 0.
        || p.y.fract() != 0.
        || p.x < 0.
        || p.y < 0.
        || p.x >= width as f64
        || p.y >= height as f64
        || target.x >= width
        || target.y >= height
    {
        return Trace::stopped(vec![], StopReason::Invalid);
    }
    let occupied: HashSet<_> = r
        .actors_at(r.depth)
        .iter()
        .filter(|a| !a.dead.value_or(false) && a.hp > 0. && observable(a)
            && corpse_core::dreamwalking::same_plane(r, p, a))
        .map(|a| (a.x as usize, a.y as usize))
        .collect();
    if corpse_core::portals::enabled(r) {
        // O(F + A + L²V) worst case, O(F + A + L) scratch: F is the full
        // stored floor, V terrain cells, L physical link endpoints. Unknown
        // links are removed before pure ray search; per-cell gates are O(1).
        let mut known = f.clone();
        for (at, tile) in known.tiles.iter_mut().enumerate() {
            if mask.get(at) != Some(&true) { *tile = "wall".into(); }
        }
        known.portals = f.portals.as_ref().map(|pairs| pairs.iter()
            .filter(|p| mask.get(p.a) == Some(&true) && mask.get(p.b) == Some(&true))
            .cloned().collect::<Vec<_>>()).map_or(corpse_core::Field::Missing, Into::into);
        let mut unknown_pocket_entries=vec![false;width*height];
        if crate::non_euclidean_ui::enabled(r) {
            known.non_euclidean = f.non_euclidean.as_ref().map(|pockets| pockets.iter().map(|pocket| {
                let groups: Vec<_> = pocket.groups.iter().filter(|g| {
                    let all_known=g.sources.iter().copied().chain(std::iter::once(g.destination))
                        .all(|at|mask.get(at)==Some(&true));
                    if !all_known {for at in g.sources.iter().copied().chain(std::iter::once(g.destination)) {
                        if let Some(blocked)=unknown_pocket_entries.get_mut(at) {*blocked=true;}
                    }}
                    all_known
                }).cloned().collect();
                let mut cells: Vec<_> = groups.iter().flat_map(|g|g.sources.iter().copied().chain(std::iter::once(g.destination))).collect();
                cells.sort_unstable(); cells.dedup();
                corpse_core::non_euclidean_generation::NonEuclideanPocket { cells, groups }
            }).filter(|p|!p.groups.is_empty()).collect::<Vec<_>>()).map_or(corpse_core::Field::Missing,Into::into);
            known.speed_up = f.speed_up.as_ref().map(|pockets| pockets.iter().map(|pocket| {
                let groups: Vec<_> = pocket.groups.iter().filter(|g| {
                    let all_known=g.sources.iter().copied().chain(std::iter::once(g.destination))
                        .all(|at|mask.get(at)==Some(&true));
                    if !all_known {for at in g.sources.iter().copied().chain(std::iter::once(g.destination)) {
                        if let Some(blocked)=unknown_pocket_entries.get_mut(at) {*blocked=true;}
                    }}
                    all_known
                }).cloned().collect();
                let mut cells: Vec<_> = groups.iter().flat_map(|g|g.sources.iter().copied().chain(std::iter::once(g.destination))).collect();
                cells.sort_unstable(); cells.dedup();
                corpse_core::non_euclidean_generation::NonEuclideanPocket { cells, groups }
            }).filter(|p|!p.groups.is_empty()).collect::<Vec<_>>()).map_or(corpse_core::Field::Missing,Into::into);
        }
        let stop = std::cell::Cell::new(StopReason::Target);
        let from=corpse_core::movement::Point{x:p.x as i32,y:p.y as i32};
        let to=corpse_core::movement::Point{x:target.x as i32,y:target.y as i32};
        let pass=|point: corpse_core::movement::Point| {
                if point.x < 0 || point.y < 0 || point.x >= width as i32 || point.y >= height as i32 {
                    stop.set(StopReason::Terrain); return false;
                }
                let at=point.y as usize*width+point.x as usize;
                if mask.get(at) != Some(&true) {stop.set(StopReason::Unknown);return false;}
                if unknown_pocket_entries[at] {
                    stop.set(StopReason::Unknown); return false;
                }
                if !corpse_core::projectiles::terrain_pass(r.simulation_revision.value_or(29.),&known,point) {
                    stop.set(StopReason::Terrain);return false;
                }
                true
            };
        let plan=corpse_core::portals::ray_plan(&known,from,to,pass);
        if plan.origin_fork.len()>1 {
            return Trace {path:vec![Point{x:p.x as usize,y:p.y as usize}],stop_reason:StopReason::Stochastic,
                possible_exits:plan.origin_fork.into_iter().filter(|&at|mask.get(at)==Some(&true))
                    .map(|at|Point{x:at%width,y:at/width}).collect()};
        }
        let mut points=corpse_core::portals::directed_ray(&known,plan.origin,plan.aim,pass);
        if plan.origin!=from {points.insert(0,from);}
        let mut path=Vec::new();
        for point in points.into_iter().skip(1) {
            let here=Point{x:point.x as usize,y:point.y as usize};
            path.push(here);
            if crate::non_euclidean_ui::enabled(r) {
                let at=here.y*width+here.x;
                let exits=corpse_core::non_euclidean_generation::exits(f,at);
                if exits.len()>1 {
                    return Trace {path,stop_reason:StopReason::Stochastic,
                        possible_exits:exits.into_iter().filter(|&at|mask.get(at)==Some(&true))
                            .map(|at|Point{x:at%width,y:at/width}).collect()};
                }
            }
            let target_here=corpse_core::portals::same_cell(&known,point.x as f64,point.y as f64,target.x as f64,target.y as f64);
            if occupied.iter().any(|&(x,y)|corpse_core::portals::same_cell(&known,x as f64,y as f64,point.x as f64,point.y as f64)) && !target_here {
                return Trace::stopped(path,StopReason::Creature);
            }
        }
        let reached=path.last().is_some_and(|at|corpse_core::portals::same_cell(&known,at.x as f64,at.y as f64,target.x as f64,target.y as f64));
        return Trace::stopped(path,if reached {StopReason::Target}else{stop.get()});
    }
    let (mut x, mut y) = (p.x as i32, p.y as i32);
    let (tx, ty) = (target.x as i32, target.y as i32);
    let (dx, dy) = ((tx - x).abs(), (ty - y).abs());
    let (sx, sy) = (if x < tx { 1 } else { -1 }, if y < ty { 1 } else { -1 });
    let mut error = dx - dy;
    let mut path = vec![];
    while x != tx || y != ty {
        let twice = 2 * error;
        if twice > -dy {
            error -= dy;
            x += sx
        }
        if twice < dx {
            error += dx;
            y += sy
        }
        let at = y as usize * width + x as usize;
        if mask.get(at) != Some(&true) {
            return Trace::stopped(path, StopReason::Unknown);
        }
        if !corpse_core::projectiles::terrain_pass(r.simulation_revision.value_or(29.),f,corpse_core::movement::Point{x,y})
        {
            return Trace::stopped(path, StopReason::Terrain);
        }
        path.push(Point {
            x: x as usize,
            y: y as usize,
        });
        if occupied.contains(&(x as usize, y as usize)) && (x != tx || y != ty) {
            return Trace::stopped(path, StopReason::Creature);
        }
    }
    Trace::stopped(path, StopReason::Target)
}
/// Observe once per input/observation invalidation, then paint the detached path.
pub fn observe(r: &Run, target: Option<Point>) -> Trace {
    let dream_view = crate::dream_ui::observation_run(r);
    let r = dream_view.as_ref();
    let mask = vision::terrain_visibility(r);
    observe_with(r, target, &mask, |a| vision::visible_actor(r, a))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> serde_json::Value {
        serde_json::from_str(include_str!("../../../fixtures/targeting-ui-v48.json")).unwrap()
    }
    #[test]
    fn source_275_knowledge_limited_projectile_rays() {
        for c in fixture()["rays"].as_array().unwrap() {
            let r: Run = serde_json::from_value(c["run"].clone()).unwrap();
            let before = r.clone();
            let target: Option<Point> = serde_json::from_value(c["target"].clone()).unwrap();
            let mask: Vec<bool> = serde_json::from_value(c["mask"].clone()).unwrap();
            let observed: Vec<f64> = serde_json::from_value(c["observed"].clone()).unwrap();
            let got = observe_with(&r, target, &mask, |a| observed.contains(&a.id));
            let expected: Trace = serde_json::from_value(c["expected"].clone()).unwrap();
            assert_eq!(got, expected, "{}", c["name"]);
            assert_eq!(r, before);
        }
    }
    #[test]
    fn hidden_terrain_and_interceptors_cannot_change_preview() {
        let c = &fixture()["rays"][0];
        let mut r: Run = serde_json::from_value(c["run"].clone()).unwrap();
        let mut mask = vec![true; 49];
        mask[16] = false;
        let target = Some(Point { x: 5, y: 5 });
        let before = observe_with(&r, target, &mask, |_| false);
        assert_eq!(before.stop_reason, StopReason::Unknown);
        r.floor_mut().unwrap().tiles[16] = "wall".into();
        let mut a = r.player.clone();
        a.id = 99.;
        a.x = 2.;
        a.y = 2.;
        r.actors_at_mut(r.depth).push(a);
        assert_eq!(observe_with(&r, target, &mask, |_| false), before);
        r.floor_mut().unwrap().tiles[16] = "floor".into();
        mask[16] = true;
        assert_eq!(
            observe_with(&r, target, &mask, |_| false).stop_reason,
            StopReason::Target
        );
        assert_eq!(
            observe_with(&r, target, &mask, |_| true).stop_reason,
            StopReason::Creature
        );
    }
    #[test]
    fn portal_ui_target_preview_crosses_and_preserves_hidden_state() {
        let c=&fixture()["rays"][0];let mut r:Run=serde_json::from_value(c["run"].clone()).unwrap();
        r.simulation_revision=35.0.into();r.player.x=1.;r.player.y=3.;r.actors.clear();
        let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.environment=corpse_core::Field::Missing;f.doors=corpse_core::Field::Missing;
        f.portals=vec![corpse_core::portals_generation::PortalPair{a:23,b:25,color_index:0}].into();
        let target=Some(Point{x:5,y:3});let mask=vec![true;49];
        let got=observe_with(&r,target,&mask, |_|false);
        assert_eq!(got.stop_reason,StopReason::Target);
        assert_eq!(got.path,vec![Point{x:2,y:3},Point{x:4,y:3},Point{x:5,y:3}]);
        let mut mask=mask;mask[25]=false;
        let before=observe_with(&r,target,&mask, |_|false);
        r.floor_mut().unwrap().tiles[25]="wall".into();
        let mut hidden=r.player.clone();hidden.id=808.;hidden.x=4.;r.actors_at_mut(r.depth).push(hidden);
        assert_eq!(observe_with(&r,target,&mask, |_|false),before);
    }

    #[test]
    fn non_euclidean_ui_projectile_preview_stops_at_stochastic_entry() {
        let mut r=crate::non_euclidean_ui::tests::scene();r.player.x=13.;r.player.y=9.;
        let before=r.clone();let mask=vec![true;r.floor().unwrap().tiles.len()];
        let trace=observe_with(&r,Some(Point{x:13,y:12}),&mask, |_|false);
        assert_eq!(trace.stop_reason,StopReason::Stochastic);
        assert_eq!(trace.path,vec![Point{x:13,y:10}]);
        assert_eq!(trace.possible_exits,vec![Point{x:11,y:10},Point{x:12,y:10}]);
        for _ in 0..5 {assert_eq!(observe_with(&r,Some(Point{x:13,y:12}),&mask, |_|false),trace);}
        assert_eq!(r,before);
    }
    #[test]
    fn non_euclidean_ui_projectile_unknown_exit_cannot_leak_or_change_preview() {
        let r=crate::non_euclidean_ui::tests::scene();let w=r.floor().unwrap().width();
        let mut mask=vec![true;r.floor().unwrap().tiles.len()];mask[10*w+13]=false;
        let target=Some(Point{x:20,y:10});let before=observe_with(&r,target,&mask, |_|false);
        assert_eq!(before.stop_reason,StopReason::Unknown);assert!(before.possible_exits.is_empty());
        let mut changed=r.clone();changed.floor_mut().unwrap().tiles[10*w+13]="wall".into();
        assert_eq!(observe_with(&changed,target,&mask, |_|false),before);
    }

    #[test]
    fn non_euclidean_ui_stochastic_firing_origin_is_not_a_promised_branch() {
        let mut r=crate::non_euclidean_ui::tests::scene();r.player.x=13.;r.player.y=10.;
        let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[9*w+12]="wall".into();
        let before=r.clone();let mask=vec![true;r.floor().unwrap().tiles.len()];
        let trace=observe_with(&r,Some(Point{x:11,y:8}),&mask, |_|false);
        assert_eq!(trace.stop_reason,StopReason::Stochastic);
        assert_eq!(trace.path,vec![Point{x:13,y:10}]);
        assert_eq!(trace.possible_exits,vec![Point{x:11,y:10},Point{x:12,y:10}]);
        assert_eq!(r,before);
    }

    #[test]
    fn r37_speed_up_projectile_preview_never_reads_unknown_exit() {
        let mut r=crate::non_euclidean_ui::tests::scene();r.simulation_revision=37.0.into();
        let f=r.floor_mut().unwrap();let w=f.width();
        f.speed_up=vec![corpse_core::non_euclidean_generation::NonEuclideanPocket {
            cells:vec![10*w+11,10*w+18],groups:vec![corpse_core::non_euclidean_generation::NonEuclideanGroup{sources:vec![10*w+11],destination:10*w+18}]
        }].into();f.non_euclidean=corpse_core::Field::Missing;
        let mut mask=vec![true;r.floor().unwrap().tiles.len()];mask[10*w+18]=false;
        let target=Some(Point{x:20,y:10});let before=observe_with(&r,target,&mask, |_|false);
        r.floor_mut().unwrap().speed_up.as_mut().unwrap()[0].groups[0].destination=12*w+18;
        mask[12*w+18]=false;
        assert_eq!(observe_with(&r,target,&mask, |_|false),before);
    }

}

/// O(A + visibility): validate observed targets without leaking hidden occupants.
pub fn visible_enemy(run: &Run, id: f64) -> bool {
    corpse_core::engine::actor(run, id).is_some_and(|a| a.id != run.player.id
        && !matches!(a.state.as_str(), "ally" | "neutral") && a.hp > 0.
        && !a.dead.value_or(false) && vision::visible_actor(run, a)
        && corpse_core::dreamwalking::same_plane(run, &run.player, a))
}
pub fn adjacent_enemy(run: &Run, id: f64) -> bool {
    visible_enemy(run, id) && corpse_core::engine::actor(run, id)
        .is_some_and(|a| corpse_core::portals::actor_chebyshev(run, &run.player, a) <= 1.)
}
/// A target-kind hint is presentation only; core admission still owns payment.
pub fn ability_prompt(id: &str) -> &'static str {
    match id {
        "numo-implant-thought" => "Select a visible enemy, then an adjacent creature. Escape cancels.",
        "pelian-fated-duel" => "Select a visible adjacent enemy. Escape cancels.",
        "pelian-relentless-pursuit" => "Select a visible enemy to pursue. Escape cancels.",
        "ursa-waking-nightmares" => "Select a visible unoccupied square. Escape cancels.",
        "kative-clay-army" => "Select an adjacent empty square to step into. The marked departure tile becomes a Clay Soldier. Escape cancels.",
        "kative-cogworks" => "Cogworks: 1 Pit · 2 Wall · 3 Remove. Choose a mode, then a square. Escape cancels.",
        _ => "Select a target. Escape cancels.",
    }
}
/// Reject invisible terrain before inspecting occupancy. The generic failure
/// never identifies an unobserved occupant on an otherwise visible square.
pub fn nightmare_blocker(run: &Run, x: usize, y: usize) -> Option<String> {
    if !vision::visible(run, x, y) { return Some("Select a visible unoccupied square.".into()); }
    corpse_core::revision50_nightmares::preflight(run, x as f64, y as f64).err()
}
/// O(visibility + actors) for one hovered candidate. No RNG or Run mutation.
pub fn ability_preview(run: &Run, id: &str, attacker: Option<f64>, hover: Option<(usize,usize)>) -> Vec<(usize,usize)> {
    let Some((x,y))=hover else { return Vec::new(); };
    if id==corpse_core::revision50_terrain::CLAY_ARMY {
        if clay_blocker(run,x,y).is_some() { return Vec::new(); }
        return corpse_core::revision50_terrain::clay_preview_for_run(run,x as f64,y as f64)
            .map(|plan|plan.cells.into_iter().map(|c|(c.x as usize,c.y as usize)).collect()).unwrap_or_default();
    }
    if id=="kative-cogworks" {
        if !vision::visible(run,x,y) { return Vec::new(); }
        let Ok(plan)=corpse_core::revision50_terrain::preview_for_run(run,x as f64,y as f64) else {return Vec::new();};
        let mask=vision::terrain_visibility(run);
        let Some(f)=run.floor() else {return Vec::new();};
        return plan.cells.into_iter().filter(|c| c.x>=0 && c.y>=0
            && mask.get(c.y as usize*f.width()+c.x as usize)==Some(&true))
            .map(|c|(c.x as usize,c.y as usize)).collect();
    }
    let enemy = || run.actors_at(run.depth).iter().find(|a|
        visible_enemy(run,a.id) && corpse_core::portals::same_position(run,a,x as f64,y as f64));
    let valid=match id {
        "ursa-waking-nightmares" => nightmare_blocker(run,x,y).is_none(),
        "pelian-relentless-pursuit" => enemy().is_some(),
        "pelian-fated-duel" => enemy().is_some_and(|a| adjacent_enemy(run,a.id)),
        "numo-implant-thought" => attacker.map_or_else(||enemy().is_some(),|id|implant_victim_blocker(run,id,x,y).is_none()),
        _ => false,
    };
    if valid { vec![(x,y)] } else { Vec::new() }
}
pub fn clay_blocker(run: &Run, x: usize, y: usize) -> Option<String> {
    if (x as f64-run.player.x).abs().max((y as f64-run.player.y).abs())!=1.
        || !vision::visible(run,x,y) {
        return Some("Select a visible adjacent empty square to step into.".into());
    }
    if let Some(reason)=crate::ability_ui::payment_blocker(run,corpse_core::revision50_terrain::CLAY_ARMY,20.) {
        return Some(reason);
    }
    corpse_core::revision50_terrain::clay_preview_for_run(run,x as f64,y as f64).err()
        .map(|_|"Choose a legal adjacent empty square; your departure tile must be available for a Clay Soldier.".into())
}
pub fn cogworks_prompt(run: &Run) -> String {
    let mode=corpse_core::revision50_terrain::mode(run);
    let selected=match mode {2=>"Wall: 20 energy, 200 shared HP",3=>"Remove: free",_=>"Pit: 20 energy"};
    format!("Cogworks mode {mode}: {selected}. [1 Pit] [2 Wall] [3 Remove]. Select a square; Escape cancels.")
}
/// The paid power authors the route. Display its contiguous visible prefix only,
/// never paint a hidden location or a disconnected glimpse past unseen terrain.
pub fn pursuit_path(run: &Run) -> Vec<(usize,usize)> {
    if run.simulation_revision.value_or(29.) < 50.
        || !run.player.conditions.contains_key(corpse_core::revision50_pursuit::TARGET_KEY) { return Vec::new(); }
    let mask=vision::terrain_visibility(run);
    let Some(f)=run.floor() else {return Vec::new();};
    corpse_core::revision50_pursuit::route_for_run(run).into_iter()
        .take_while(|&(x,y)|x>=0. && y>=0. && (x as usize)<f.width() && (y as usize)<f.height()
            && mask.get(y as usize*f.width()+x as usize)==Some(&true))
        .map(|(x,y)|(x as usize,y as usize)).collect()
}
pub fn implant_victim_blocker(run: &Run, attacker: f64, x: usize, y: usize) -> Option<&'static str> {
    let Some(source) = corpse_core::engine::actor(run, attacker).filter(|_| visible_enemy(run, attacker)) else {
        return Some("The selected enemy is no longer visible.");
    };
    let valid = std::iter::once(&run.player).chain(run.actors_at(run.depth)).any(|a|
        a.id != attacker && a.hp > 0. && !a.dead.value_or(false)
        && (a.id == run.player.id || vision::visible_actor(run, a))
        && corpse_core::dreamwalking::same_plane(run, source, a)
        && corpse_core::portals::same_position(run, a, x as f64, y as f64)
        && corpse_core::portals::actor_chebyshev(run, source, a) <= 1.);
    (!valid).then_some("Choose a visible creature adjacent to the selected enemy.")
}

#[cfg(test)]
mod revision50_target_tests {
    use super::*;
    #[test] fn invalid_implant_selection_is_read_only() {
        let mut run: Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        run.simulation_revision=50.0.into();
        let before=run.clone();
        assert!(!visible_enemy(&run,run.player.id));
        assert!(!adjacent_enemy(&run,-12345.));
        assert!(implant_victim_blocker(&run,-12345.,run.player.x as usize,run.player.y as usize).is_some());
        assert_eq!(run,before);
    }
    #[test] fn empty_and_invisible_nightmare_targets_do_not_mutate() {
        let mut run: Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        run.simulation_revision=50.0.into();
        run.player.abilities.push("ursa-waking-nightmares".into());
        run.player.nightmares=1.0.into();
        let before=run.clone();
        assert!(nightmare_blocker(&run,run.player.x as usize,run.player.y as usize).is_some());
        assert!(ability_preview(&run,"ursa-waking-nightmares",None,None).is_empty());
        assert!(pursuit_path(&run).is_empty());
        assert_eq!(run,before);
    }
    #[test] fn cogworks_modes_report_wall_limit_and_legacy_has_no_pursuit() {
        let mut run: Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        run.simulation_revision=50.0.into();
        run.player.conditions.insert(corpse_core::revision50_terrain::MODE_KEY.into(),2.);
        let before=run.clone();
        assert!(cogworks_prompt(&run).contains("Wall: 20 energy, 200 shared HP"));
        assert!(!cogworks_prompt(&run).contains("preview only"));
        assert!(ability_preview(&run,"kative-cogworks",None,None).is_empty());
        assert_eq!(run,before);
        run.simulation_revision=46.0.into();
        assert!(pursuit_path(&run).is_empty());
    }
    #[test] fn clay_rejects_same_and_distant_targets_before_payment() {
        let mut run: Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        run.simulation_revision=50.0.into();run.player.abilities.push("kative-clay-army".into());
        let before=run.clone();let (x,y)=(run.player.x as usize,run.player.y as usize);
        assert!(clay_blocker(&run,x,y).is_some());assert!(clay_blocker(&run,x+2,y).is_some());
        assert!(ability_preview(&run,"kative-clay-army",None,Some((x,y))).is_empty());
        assert_eq!(run,before);
    }
}
