//! Firebolt travel amendment, deliberately independent of the simulation revision.
//! Existing command prefixes retain the original trace-then-burn execution.
use crate::{Actor, Field, Run, engine::{self, EngineError}, movement::Point};

pub fn current(r: &Run) -> bool {
    r.simulation_revision.value_or(29.) >= 52. && !matches!(r.firebolt_travel_from, Field::Null)
}
/// O(1). Missing old checkpoints adopt the amendment at their next command.
/// Explicit Null is reserved for replaying a legacy prefix without upgrading it.
pub fn admit_current(r: &mut Run) -> Result<(), String> {
    validate(r)?;
    if r.simulation_revision.value_or(29.) >= 52. && r.firebolt_travel_from.is_missing() {
        r.firebolt_travel_from = r.replay.len().into();
    }
    Ok(())
}
pub fn validate(r: &Run) -> Result<(), String> {
    if !r.firebolt_travel_from.is_missing() && (r.simulation_revision.value_or(29.) < 52.
        || r.firebolt_travel_from.as_ref().is_some_and(|&n| n > r.replay.len())) {
        return Err("Invalid Firebolt travel command boundary".into());
    }
    Ok(())
}
/// O(1), called before Engine admission and at the single command boundary.
pub fn apply_replay_policy(r: &mut Run, from: &Field<usize>, cursor: usize) {
    if r.simulation_revision.value_or(29.) < 52. { return; }
    if from.as_ref() == Some(&cursor) { r.firebolt_travel_from = cursor.into(); }
    else if cursor == 0 { r.firebolt_travel_from = Field::Null; }
}

pub(crate) struct Flight {
    pub points: Vec<Point>,
    pub impact: Option<Point>,
    pub target: Option<f64>,
    pub bursts: Vec<crate::fire_adapter::Burst>,
}
/// A collision is decided before damage, so immunity or death cannot make the
/// bolt pierce. Underwater occupants do not receive the direct packet or stop it.
fn occupant(r: &Run, source: &Actor, p: Point) -> Option<f64> {
    std::iter::once(&r.player).chain(r.actors_at(r.depth)).find(|a|
        a.id != source.id && !a.dead.value_or(false) && a.hp > 0.
        && engine::condition(a,"underwater") <= 0.
        && crate::dreamwalking::same_plane(r,source,a)
        && crate::portals::same_position(r,a,p.x as f64,p.y as f64)).map(|a|a.id)
}
fn pass(r: &Run, p: Point) -> bool {
    r.floor().is_some_and(|f|crate::projectiles::terrain_pass(r.simulation_revision.value_or(29.),f,p))
}
fn enter(adapter: &mut crate::fire_adapter::FireAdapter<'_>, flight: &mut Flight, source: &Actor, p: Point) -> bool {
    let Some(at)=adapter.run.floor().and_then(|f|crate::portals::cell(f,p.x as f64,p.y as f64)) else {return false;};
    // Ignite the entered cell before inspecting ALL surviving blocking layers.
    // This is the existing forced terrain ignition, not a new damage packet.
    adapter.ignite_creature(at,-2.,1.);
    // Phasing creatures can occupy a still-solid tile. They intercept the bolt
    // before the terrain stop, just as any other non-submerged occupant does.
    flight.target=occupant(adapter.run,source,p);
    if !pass(adapter.run,p) {flight.impact=Some(p);return false;}
    flight.points.push(p);
    flight.target.is_none()
}
/// One committed, bounded integer ray with live ignition before obstruction.
/// Keep the existing pure aim selection and portal exit preflight: speculative
/// alternatives never ignite and never consume RNG. Only the selected exit is
/// entered; each linked group is crossed once. O(ray-plan + V*(A*P+G) + fire
/// closure), O(V) extra memory. P/G are alias/link query costs; fire closure is
/// FireAdapter::resolve, performed once rather than once per traversed tile.
pub(crate) fn launch(r: &mut Run, source: &Actor, x: f64, y: f64) -> Result<Flight,EngineError> {
    let from=Point{x:source.x as i32,y:source.y as i32};
    let f=crate::dreamwalking::floor_for(r,source).unwrap();
    let revision=r.simulation_revision.value_or(29.);
    let plan=crate::portals::ray_plan(f,from,Point{x:x as i32,y:y as i32},|p|crate::projectiles::terrain_pass(revision,f,p));
    let limit=f.tiles.len().saturating_mul(2);
    let mut origin=plan.origin;
    if plan.origin_fork.len()>1 {
        let selected=(crate::rng::random(r)*plan.origin_fork.len() as f64).floor() as usize;
        origin=crate::portals::point(r.floor().unwrap(),plan.origin_fork[selected]);
    }
    let (dx,dy)=(plan.aim.x-plan.origin.x,plan.aim.y-plan.origin.y);
    let (mut x,mut y)=(origin.x,origin.y);
    let (mut tx,mut ty)=(x+dx*limit as i32,y+dy*limit as i32);
    let (dx,dy)=((tx-x).abs(),(ty-y).abs());
    let (sx,sy)=((tx-x).signum(),(ty-y).signum());
    let mut error=dx-dy;
    let frozen=crate::environment::frozen_environment(r);
    let mut adapter=crate::fire_adapter::FireAdapter::new(r,frozen).map_err(EngineError)?;
    let mut flight=Flight{points:vec![from],impact:None,target:None,bursts:vec![]};
    let mut crossed=std::collections::HashSet::new();
    let start_open=origin==from || enter(&mut adapter,&mut flight,source,origin);
    if start_open {for _ in 0..limit {
        if x==tx && y==ty {break;}
        let twice=error*2;
        if twice > -dy {error-=dy;x+=sx;}
        if twice < dx {error+=dx;y+=sy;}
        let p=Point{x,y};
        if !enter(&mut adapter,&mut flight,source,p) {break;}
        let f=adapter.run.floor().unwrap();
        let at=crate::portals::cell(f,x as f64,y as f64).unwrap();
        let options=crate::portals::exits(f,at);
        if options.len()!=1 || options[0]!=at {
            let key=*crate::portals::aliases(f,at).iter().min().unwrap();
            if !crossed.insert(key) {break;}
            // Retain the spatial-link contract: all exits must be accessible
            // before the lottery, and an unchosen exit is never a visited tile.
            if options.iter().any(|&at|!pass(adapter.run,crate::portals::point(f,at))) {break;}
            let selected=if options.len()==1 {0} else {(crate::rng::random(adapter.run)*options.len() as f64).floor() as usize};
            let other=crate::portals::point(adapter.run.floor().unwrap(),options[selected]);
            tx+=other.x-x;ty+=other.y-y;x=other.x;y=other.y;
            if !enter(&mut adapter,&mut flight,source,other) {break;}
        }
    }}
    flight.bursts=adapter.resolve().map_err(EngineError)?;
    Ok(flight)
}
