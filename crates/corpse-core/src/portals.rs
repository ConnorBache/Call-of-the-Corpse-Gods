//! Revision35 same-floor spatial links. Pure queries never consume gameplay RNG.
use crate::{Actor, Floor, Run};
use crate::movement::Point;
use crate::portals_generation::PortalPair;
pub fn enabled(r:&Run)->bool {r.simulation_revision.value_or(29.)>=35.}
pub fn pair_at(f:&Floor,cell:usize)->Option<&PortalPair> {f.portals.as_ref()?.iter().find(|p|p.a==cell||p.b==cell)}
pub fn mate(f:&Floor,cell:usize)->Option<usize> {let p=pair_at(f,cell)?;Some(if p.a==cell {p.b}else{p.a})}
pub fn cell(f:&Floor,x:f64,y:f64)->Option<usize> {(x.is_finite()&&y.is_finite()&&x.fract()==0.&&y.fract()==0.&&x>=0.&&y>=0.&&x<f.width()as f64&&y<f.height()as f64).then(||y as usize*f.width()+x as usize)}
pub fn point(f:&Floor,at:usize)->Point {Point{x:(at%f.width())as i32,y:(at/f.width())as i32}}
/// Logical occupancy is shared by every representation, including both sources
/// of a stochastic link. This query is not an optical viewpoint or a transition.
pub fn aliases(f:&Floor,at:usize)->Vec<usize> {
 if let Some(g)=crate::non_euclidean_generation::group_at(f,at) {let mut out=g.sources.clone();out.push(g.destination);return out}
 let mut out=vec![at];if let Some(other)=mate(f,at){out.push(other)}out
}
pub fn same_cell(f:&Floor,x1:f64,y1:f64,x2:f64,y2:f64)->bool {x1==x2&&y1==y2 || cell(f,x1,y1).zip(cell(f,x2,y2)).is_some_and(|(a,b)|aliases(f,a).contains(&b))}
pub fn same_position(r:&Run,a:&Actor,x:f64,y:f64)->bool {if !enabled(r){return a.x==x&&a.y==y}crate::dreamwalking::floor_for(r,a).is_some_and(|f|same_cell(f,a.x,a.y,x,y))}
pub fn occupied_at<'a>(r:&'a Run,origin:&Actor,x:f64,y:f64,exclude_id:Option<f64>)->Option<&'a Actor> {std::iter::once(&r.player).chain(r.actors_at(r.depth)).find(|b|Some(b.id)!=exclude_id&&!b.dead.value_or(false)&&b.hp>0.&&crate::dreamwalking::same_plane(r,origin,b)&&same_position(r,b,x,y))}
/// O(G*S), G<=384 and S<=2; terrain-sensitive Speed-up links are rebuilt
/// from current floor state, with no stale topology cache to invalidate.
pub fn links(f:&Floor)->Vec<(usize,usize)> {
 let mut out:Vec<_>=f.portals.as_ref().map_or_else(Vec::new,|p|p.iter().map(|p|(p.a,p.b)).collect());
 for (pocket,speed) in f.non_euclidean.as_ref().into_iter().flatten().map(|p|(p,false))
    .chain(f.speed_up.as_ref().into_iter().flatten().map(|p|(p,true))) {
    for group in &pocket.groups {
        if speed && !crate::non_euclidean_generation::speedup_group_active(f,group) {continue}
        for &source in &group.sources {out.push((source,group.destination));}
    }
 }
 out
}
pub fn exits(f:&Floor,at:usize)->Vec<usize> {let v=crate::non_euclidean_generation::exits(f,at);if !v.is_empty(){v}else{vec![arrival(f,at)]}}
/// Ordered-pair movement has one local exception: reversing or entering from
/// inside a pair walks onto the paired adjacent square instead of returning to
/// the square just occupied. This does not change shared occupancy or rays.
pub fn ordered_pairs(f:&Floor)->Vec<(usize,usize)> {
 f.speed_up.as_ref().into_iter().flatten().flat_map(|p|p.groups.iter())
  .filter_map(|g|(g.sources.len()==1 && crate::non_euclidean_generation::speedup_group_active(f,g)).then(||(g.sources[0],g.destination))).collect()
}
pub fn walking_exits(f:&Floor,from:usize,entry:usize)->Vec<usize> {
 if f.speed_up.as_ref().into_iter().flatten().flat_map(|p|p.groups.iter()).any(|g|g.sources.len()==1 && crate::non_euclidean_generation::speedup_group_active(f,g) && ((g.sources[0]==from&&g.destination==entry)||(g.destination==from&&g.sources[0]==entry))){vec![entry]}else{exits(f,entry)}
}
pub fn step_options(r:&Run,a:&Actor,x:f64,y:f64)->Vec<(f64,f64)> {
 if !enabled(r){return vec![(x,y)]}let Some(f)=crate::dreamwalking::floor_for(r,a)else{return vec![(x,y)]};let Some(at)=cell(f,x,y)else{return vec![(x,y)]};
 let options=if r.simulation_revision.value_or(29.)>=37. {cell(f,a.x,a.y).map_or_else(||exits(f,at),|from|walking_exits(f,from,at))}else{exits(f,at)};
 options.into_iter().map(|at|{let p=point(f,at);(p.x as f64,p.y as f64)}).collect()
}
/// Call only after all possible outcomes have passed action validation. Exactly
/// one gameplay draw is consumed for an actual two-way transition, none otherwise.
pub fn commit_step(r:&mut Run,a:&Actor,x:f64,y:f64)->(f64,f64) {let options=step_options(r,a,x,y);let index=if options.len()>1{(crate::rng::random(r)*options.len() as f64).floor()as usize}else{0};options[index]}
pub fn arrival(f:&Floor,at:usize)->usize {mate(f,at).unwrap_or(at)}
#[derive(Clone,Debug)] pub struct Route {pub distance:f64,pub points:Vec<Point>}
fn dist(a:Point,b:Point,chebyshev:bool)->f64 {let(dx,dy)=((a.x-b.x).abs()as f64,(a.y-b.y).abs()as f64);if chebyshev{dx.max(dy)}else{dx.hypot(dy)}}
/// Physical metric graph: O(K²*C) time, O(K) scratch, K<=2+2P+2S,
/// P<=7 portal pairs, S<=512 source links across <=16 bounded48-cell areas. C is the
/// supplied clearance-check cost (1 for metrics, up to V for raster checks).
/// Optical route_filtered deliberately constructs portal-only nodes (K<=16).
pub fn route(f:&Floor,from:Point,to:Point,chebyshev:bool,clear:impl Fn(Point,Point)->bool)->Option<Route> {
 route_links(f,from,to,chebyshev,clear,|_,_|true,&links(f))
}
pub fn route_filtered(f:&Floor,from:Point,to:Point,chebyshev:bool,clear:impl Fn(Point,Point)->bool,transfer:impl Fn(Point,Point)->bool)->Option<Route> {
 let links:Vec<_>=f.portals.as_ref().into_iter().flatten().map(|p|(p.a,p.b)).collect();
 route_links(f,from,to,chebyshev,clear,transfer,&links)
}
fn route_links(f:&Floor,from:Point,to:Point,chebyshev:bool,clear:impl Fn(Point,Point)->bool,transfer:impl Fn(Point,Point)->bool,links:&[(usize,usize)])->Option<Route> {
 let mut nodes=vec![from,to];for &(a,b) in links{nodes.push(point(f,a));nodes.push(point(f,b));}
 let n=nodes.len();let mut costs=vec![f64::INFINITY;n];let mut previous=vec![usize::MAX;n];let mut done=vec![false;n];costs[0]=0.;
 for _ in 0..n {let Some(i)=(0..n).filter(|&i|!done[i]&&costs[i].is_finite()).min_by(|&a,&b|costs[a].total_cmp(&costs[b])) else{break};if i==1{break}done[i]=true;
  for j in 0..n{if i==j||done[j]{continue}let linked=i>=2&&j>=2&&(i-2)/2==(j-2)/2&&i!=j;let length=if linked{if !transfer(nodes[i],nodes[j]){continue}0.}else{if !clear(nodes[i],nodes[j]){continue}dist(nodes[i],nodes[j],chebyshev)};let v=costs[i]+length;if v<costs[j]{costs[j]=v;previous[j]=i;}}
 }
 if !costs[1].is_finite(){return None}let mut ids=vec![1];let mut i=1;while i!=0 {i=previous[i];if i==usize::MAX||ids.len()>n{return None}ids.push(i)}ids.reverse();Some(Route{distance:costs[1],points:ids.into_iter().map(|i|nodes[i]).collect()})
}
pub fn point_distance(r:&Run,origin:&Actor,x:f64,y:f64)->f64 {metric(r,origin,x,y,false)}
pub fn point_chebyshev(r:&Run,origin:&Actor,x:f64,y:f64)->f64 {metric(r,origin,x,y,true)}
fn metric(r:&Run,a:&Actor,x:f64,y:f64,cheb:bool)->f64 {let from=Point{x:a.x as i32,y:a.y as i32};let to=Point{x:x as i32,y:y as i32};if !enabled(r){return if cheb{(a.x-x).abs().max((a.y-y).abs())}else{(a.x-x).hypot(a.y-y)}}crate::dreamwalking::floor_for(r,a).and_then(|f|route(f,from,to,cheb,|_,_|true)).map_or(f64::INFINITY,|p|p.distance)}
pub fn actor_distance(r:&Run,a:&Actor,b:&Actor)->f64 {if !enabled(r){return(a.x-b.x).hypot(a.y-b.y)}if !crate::dreamwalking::same_plane(r,a,b){f64::INFINITY}else{point_distance(r,a,b.x,b.y)}}
pub fn actor_chebyshev(r:&Run,a:&Actor,b:&Actor)->f64 {if !enabled(r){return(a.x-b.x).abs().max((a.y-b.y).abs())}if !crate::dreamwalking::same_plane(r,a,b){f64::INFINITY}else{point_chebyshev(r,a,b.x,b.y)}}
pub fn direction_toward(r:&Run,a:&Actor,x:f64,y:f64)->(f64,f64) {
 if same_position(r,a,x,y){return(0.,0.)}
 let start=Point{x:a.x as i32,y:a.y as i32};let goal=Point{x:x as i32,y:y as i32};
 if enabled(r){if let Some(f)=crate::dreamwalking::floor_for(r,a){if let Some(p)=route(f,start,goal,false,|_,_|true){for w in p.points.windows(2){if w[0]!=w[1]&&!same_cell(f,w[0].x as f64,w[0].y as f64,w[1].x as f64,w[1].y as f64){return((w[1].x-w[0].x).signum()as f64,(w[1].y-w[0].y).signum()as f64)}}}}}
 // f64::signum maps positive zero to +1; legacy grid movement requires zero.
 let component=|delta:f64|if delta==0.{0.}else{delta.signum()};
 (component(x-a.x),component(y-a.y))
}
/// Pure representative for legacy callers. Stochastic-aware callers must inspect
/// step_options; only commit_step may choose an actual random outcome.
pub fn resolve_step(r:&Run,a:&Actor,x:f64,y:f64)->(f64,f64) {step_options(r,a,x,y)[0]}
/// A directed integer ray. Entering a portal translates its remaining endpoint;
/// direction/error and remaining range are unchanged. No portal can be traversed
/// twice in one ray, so cycles terminate without consuming RNG.
pub fn directed_ray(f:&Floor,from:Point,aim:Point,pass:impl Fn(Point)->bool)->Vec<Point>{
 directed_ray_choose(f,from,aim,pass,|options|if options.len()==1{Some(options[0])}else{None},|_|false)
}
/// Physical-only ray. Preview returns at the first random fork; committed callers
/// provide one draw per encountered fork. Collision is checked BEFORE drawing,
/// and each linked group may be traversed once, bounding cycles by map size.
pub fn directed_ray_choose(f:&Floor,from:Point,aim:Point,pass:impl Fn(Point)->bool,choose:impl FnMut(&[usize])->Option<usize>,stop:impl Fn(Point)->bool)->Vec<Point>{directed_ray_trace(f,from,aim,pass,choose,stop).points}
#[derive(Clone,Debug)]pub struct PhysicalRay{pub points:Vec<Point>,pub distance:f64}
pub fn directed_ray_trace(f:&Floor,from:Point,aim:Point,pass:impl Fn(Point)->bool,mut choose:impl FnMut(&[usize])->Option<usize>,stop:impl Fn(Point)->bool)->PhysicalRay{
 let mut segment=from;let mut traveled=0.;
 let(mut x,mut y)=(from.x,from.y);let(mut tx,mut ty)=(aim.x,aim.y);let(dx,dy)=((tx-x).abs(),(ty-y).abs());let(sx,sy)=((tx-x).signum(),(ty-y).signum());let mut error=dx-dy;let mut out=vec![from];let mut crossed=Vec::new();
 for _ in 0..f.tiles.len().saturating_mul(2){if x==tx&&y==ty{break}let twice=error*2;if twice > -dy {error-=dy;x+=sx}if twice < dx{error+=dx;y+=sy}let p=Point{x,y};let Some(at)=cell(f,x as f64,y as f64)else{break};if !pass(p){break}out.push(p);if stop(p){break}
  let options=exits(f,at);if options.len()!=1||options[0]!=at {
   let key=*aliases(f,at).iter().min().unwrap();if crossed.contains(&key){break}crossed.push(key);
   // A blocked alternative is not removed or rerolled. Stop before the lottery.
   if options.iter().any(|&other|!pass(point(f,other))){break}
   let Some(other)=choose(&options).map(|at|point(f,at))else{break};
   traveled+=dist(segment,p,false);segment=other;
   tx+=other.x-x;ty+=other.y-y;x=other.x;y=other.y;out.push(other);if stop(other){break}
  }
 }
 traveled+=dist(segment,*out.last().unwrap(),false);
 if !crate::non_euclidean_generation::has_links(f){traveled=ray_distance(f,&out)}
 PhysicalRay{points:out,distance:traveled}
}
/// Pick a straight translated aim that reaches the selected physical target.
/// O(K²*V + K*V*(K+1)) worst-case bounded search over direct, individual
/// and shortest physical transforms; no branch enumeration or gameplay RNG.
/// A preview ray terminates at its first stochastic entry.
/// Live occupancy is deliberately absent: UI can supply its own known opacity.
pub fn aimed_ray(f:&Floor,from:Point,to:Point,pass:impl Fn(Point)->bool+Copy)->Vec<Point>{
 let plan=ray_plan(f,from,to,pass);if plan.origin_fork.len()>1{return vec![from]}
 let mut out=directed_ray(f,plan.origin,plan.aim,pass);if plan.origin!=from{out.insert(0,from)}out
}
/// Read-only aim selection. A stochastic exit is never sampled or predicted.
#[derive(Clone,Debug)]pub struct RayPlan{pub origin:Point,pub aim:Point,pub origin_fork:Vec<usize>}
pub fn ray_aim(f:&Floor,from:Point,to:Point,pass:impl Fn(Point)->bool+Copy)->(Point,Point){let p=ray_plan(f,from,to,pass);(p.origin,p.aim)}
pub fn ray_plan(f:&Floor,from:Point,to:Point,pass:impl Fn(Point)->bool+Copy)->RayPlan{
 let mut origins=vec![from];let start=cell(f,from.x as f64,from.y as f64);let alternatives=start.map_or_else(Vec::new,|at|exits(f,at));
 let pocket_origin=start.is_some_and(|at|crate::non_euclidean_generation::group_at(f,at).is_some());
 if pocket_origin {
  if !alternatives.is_empty()&&alternatives.iter().all(|&at|pass(point(f,at))){for &at in &alternatives{let p=point(f,at);if p!=from{origins.push(p)}}}
 } else if let Some(at)=start.and_then(|at|mate(f,at)){origins.push(point(f,at));}
 let mut aims=vec![to];for (a,b) in links(f){let a=point(f,a);let b=point(f,b);aims.push(Point{x:to.x-(b.x-a.x),y:to.y-(b.y-a.y)});aims.push(Point{x:to.x-(a.x-b.x),y:to.y-(a.y-b.y)});}
 if let Some(path)=route(f,from,to,false,|a,b|linear_cells(a,b).into_iter().skip(1).all(pass)){let(mut dx,mut dy)=(0,0);for w in path.points.windows(2){if cell(f,w[0].x as f64,w[0].y as f64).is_some_and(|a|exits(f,a).contains(&cell(f,w[1].x as f64,w[1].y as f64).unwrap_or(usize::MAX))){dx+=w[1].x-w[0].x;dy+=w[1].y-w[0].y;}}aims.push(Point{x:to.x-dx,y:to.y-dy});}
 let mut best:Option<(f64,Point,Point)>=None;for origin in origins{for &aim in &aims{let ray=directed_ray_trace(f,origin,aim,pass,|options|if options.len()==1{Some(options[0])}else{None},|_|false);if !ray.points.last().is_some_and(|p|same_cell(f,p.x as f64,p.y as f64,to.x as f64,to.y as f64)){continue}let cost=if crate::non_euclidean_generation::has_links(f){ray.distance}else{ray.points.windows(2).filter(|w|!same_cell(f,w[0].x as f64,w[0].y as f64,w[1].x as f64,w[1].y as f64)).count()as f64};if best.as_ref().is_none_or(|(n,_,_)|cost<*n){best=Some((cost,origin,aim));}}}
 let (origin,aim)=best.map(|(_,o,a)|(o,a)).unwrap_or((from,to));
 RayPlan{origin,aim,origin_fork:if origin!=from&&alternatives.len()>1{alternatives}else{vec![]}}
}
pub fn ray_distance(f:&Floor,path:&[Point])->f64{
 let Some(&first)=path.first()else{return 0.};
 if !crate::non_euclidean_generation::has_links(f) {
  // Retain exact revision35 segment interpretation for legacy portal-only rays.
  let mut start=first;let mut total=0.;for w in path.windows(2){if same_cell(f,w[0].x as f64,w[0].y as f64,w[1].x as f64,w[1].y as f64)&&w[0]!=w[1]{total+=dist(start,w[0],false);start=w[1];}}return total+dist(start,*path.last().unwrap(),false)
 }
 // Directed rays emit a physical step, THEN its optional transfer. Occupancy
 // aliases alone do not identify jumps: adjacent sibling sources cost a step.
 let mut i=0;let mut start=first;let mut total=0.;
 if path.len()>1 && cell(f,first.x as f64,first.y as f64).and_then(|at|mate(f,at))==cell(f,path[1].x as f64,path[1].y as f64){i=1;start=path[1];}
 while i+1<path.len(){i+=1;let entry=path[i];if i+1<path.len(){let at=cell(f,entry.x as f64,entry.y as f64);let next=cell(f,path[i+1].x as f64,path[i+1].y as f64);if at.zip(next).is_some_and(|(a,b)|a!=b&&exits(f,a).contains(&b)){total+=dist(start,entry,false);i+=1;start=path[i];}}}
 total+dist(start,*path.last().unwrap(),false)
}
pub fn linear_cells(from:Point,to:Point)->Vec<Point>{let(mut x,mut y)=(from.x,from.y);let(dx,dy)=((to.x-x).abs(),(to.y-y).abs());let(sx,sy)=((to.x-x).signum(),(to.y-y).signum());let mut e=dx-dy;let mut out=vec![from];for _ in 0..7200{if x==to.x&&y==to.y{break}let t=e*2;if t > -dy{e-=dy;x+=sx}if t < dx{e+=dx;y+=sy}out.push(Point{x,y});}out}
