//! v48 perception geometry and probability primitives; lighting sources are separate.
use crate::movement::Point;
pub fn distance(a: Point, b: Point) -> f64 {
    ((a.x - b.x) as f64).hypot((a.y - b.y) as f64)
}
/// Bresenham ray, O(max(dx,dy)) time and O(1) space; opaque destination is visible.
pub fn ray(width: usize, height: usize, opaque: &[bool], a: Point, b: Point) -> bool {
    let bounds =
        |p: Point| p.x >= 0 && p.y >= 0 && (p.x as usize) < width && (p.y as usize) < height;
    if !bounds(a) || !bounds(b) {
        return false;
    }
    let (mut x, mut y) = (a.x, a.y);
    let (dx, dy) = ((b.x - x).abs(), (b.y - y).abs());
    let (sx, sy) = (if x < b.x { 1 } else { -1 }, if y < b.y { 1 } else { -1 });
    let mut e = dx - dy;
    while x != b.x || y != b.y {
        let e2 = 2 * e;
        if e2 > -dy {
            e -= dy;
            x += sx;
        }
        if e2 < dx {
            e += dx;
            y += sy;
        }
        if x == b.x && y == b.y {
            return true;
        }
        if opaque[y as usize * width + x as usize] {
            return false;
        }
    }
    true
}
pub fn sight_chance(
    mut perception: f64,
    distance: f64,
    stealth: f64,
    submerged_stealth: f64,
    light: f64,
    both_underwater: bool,
    los: bool,
    invisible: bool,
) -> f64 {
    if invisible || !los || light <= -100.0 {
        return 0.0;
    }
    if both_underwater {
        perception *= 1.25;
    }
    let mut d = (distance
        + crate::combat::integer_stat(stealth)
        + crate::combat::integer_stat(submerged_stealth))
    .max(0.0);
    d = if light >= 0.0 {
        d * (1.0 - light / 100.0)
    } else {
        d / (1.0 + light / 100.0)
    };
    (perception * 0.9f64.powf(d)).clamp(0.0, 100.0)
}
/// R51 returns deterministic eligibility (0/100), preserving legacy query
/// thresholds and old revision probability arithmetic without duplicate rules.
fn optical_chance(r: &Run, perception: f64, distance: f64, stealth: f64,
    submerged: f64, light: f64, wet: bool, los: bool, invisible: bool, creature: bool) -> f64 {
    if !crate::revision51_perception::enabled(r) {
        return sight_chance(perception,distance,stealth,submerged,light,wet,los,invisible);
    }
    if !los || invisible { return 0.; }
    let eligible = if creature {
        crate::revision51_perception::creature_in_range(perception,distance,stealth,submerged,light)
    } else { crate::revision51_perception::tile_in_range(perception,distance,light) };
    if eligible {100.} else {0.}
}
fn observer_perception(r: &Run, observer: &Actor) -> f64 {
    if crate::revision51_perception::enabled(r) { crate::combat::effective_stats(r,observer).perception } else {300.}
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn destination_wall_visible() {
        let mut o = vec![false; 5];
        o[2] = true;
        assert!(ray(5, 1, &o, Point { x: 0, y: 0 }, Point { x: 2, y: 0 }));
        assert!(!ray(5, 1, &o, Point { x: 0, y: 0 }, Point { x: 3, y: 0 }));
    }
    #[test]
    fn dark_impossible() {
        assert_eq!(
            sight_chance(300.0, 0.0, 0.0, 0.0, -100.0, false, true, false),
            0.0
        );
    }
}
use crate::engine::condition;
use crate::model::{Actor, Floor, Run};
fn point(a: &Actor) -> Point {
    Point {
        x: a.x as i32,
        y: a.y as i32,
    }
}
fn has(a: &Actor, id: &str) -> bool {
    a.abilities.iter().any(|s| s == id)
}
fn active(a: &Actor, id: &str) -> bool {
    a.active.iter().any(|s| s == id)
}
/// O(B) ability/status scan, no allocation. R46-49 and R51+ suppress the whole
/// source; <=45 and R50 keep historical self-exclusion.
/// Body/dead/plane eligibility remains with each existing source collector.
pub(crate) fn emits_shine(r: &Run, a: &Actor) -> bool {
    let revision = r.simulation_revision.value_or(29.);
    has(a, "anavani-shine")
        && !(((46.0..50.0).contains(&revision) || revision >= 51.) && active(a, "anavani-shine"))
}
/// One equipment owner can have multiple physical endpoints (Half Here).
/// The same worn rings illuminate the union of their LOS once per tile.
pub(crate) struct RingLightGroup<'a> { pub origins: Vec<&'a Actor>, pub copies: usize }
/// O(A*I + A*C) expected membership/condition work, O(H) borrowed source handles.
/// No cache, RNG or world mutation; body and dead endpoints do not emit.
pub(crate) fn ring_light_groups(r: &Run, dream: bool) -> Vec<RingLightGroup<'_>> {
    if !crate::revision50::enabled(r) { return Vec::new(); }
    let mut owners = indexmap::IndexMap::<u64,RingLightGroup<'_>>::new();
    for a in std::iter::once(&r.player).chain(r.actors_at(r.depth)) {
        if a.dead.value_or(false) || a.hp <= 0. || crate::dreamwalking::is_body(a)
            || (crate::dreamwalking::enabled(r) && crate::dreamwalking::dream_actor(r,a) != dream) { continue; }
        let copies = crate::revision50_rings::count_effect(r,a,"light");
        if copies == 0 { continue; }
        let owner = crate::half_here::owner_id(r,a);
        let key = if owner == 0. { 0f64.to_bits() } else { owner.to_bits() };
        owners.entry(key).or_insert_with(|| RingLightGroup { origins: Vec::new(), copies }).origins.push(a);
    }
    owners.into_values().collect()
}
/// O(F+D+A*I + H*(F+K²R)) scalar optical query, O(H+K) scratch. D is the
/// R52 potion mask. Each copy contributes +10 outside that mask; shared-body
/// endpoints do not duplicate the same ring.
fn ring_light_bonus(r: &Run, target: Point, dream: bool) -> f64 {
    if crate::dreamwalking::floor_in_plane(r,dream).or_else(||r.floor()).is_some_and(|f|
        bounds(f,target)&&crate::revision50_potions::absolute_darkness_at(r,f,target.y as usize*f.width()+target.x as usize)) {return 0.;}
    ring_light_groups(r,dream).into_iter().filter(|g|
        !g.origins.iter().any(|a| point(a)==target)
            && g.origins.iter().any(|a| optical_route(r,a,target).is_some())
    ).map(|g|10. * g.copies as f64).sum()
}
fn sleeping(a: &Actor) -> bool {
    a.state == "sleeping"
        || (a.state == "ally" && condition(a, "monsterAllyRest") == 1.0)
        || (a.state == "neutral" && condition(a, "monsterRecoverySleep") == 1.0)
}
fn bounds(f: &Floor, p: Point) -> bool {
    p.x >= 0 && p.y >= 0 && (p.x as usize) < f.width() && (p.y as usize) < f.height()
}
/// Direct geometry query avoids an O(V) allocation for each ray.
pub(crate) fn floor_ray(f: &Floor, a: Point, b: Point) -> bool {
    if !bounds(f, a) || !bounds(f, b) {
        return false;
    }
    let (mut x, mut y) = (a.x, a.y);
    let (dx, dy) = ((b.x - x).abs(), (b.y - y).abs());
    let (sx, sy) = (if x < b.x { 1 } else { -1 }, if y < b.y { 1 } else { -1 });
    let mut e = dx - dy;
    while x != b.x || y != b.y {
        let e2 = e * 2;
        if e2 > -dy {
            e -= dy;
            x += sx;
        }
        if e2 < dx {
            e += dx;
            y += sy;
        }
        if x == b.x && y == b.y {
            return true;
        }
        let i = y as usize * f.width() + x as usize;
        if f.tiles[i] == "wall" || f.tiles[i] == "tree" || crate::environment::blocks_sight(f, i) {
            return false;
        }
    }
    true
}
/// Scalar query: O(F+R) without portals; O(F+K²R) with K endpoints, O(K) scratch.
/// Full routes retain exact point/tie order; bulk callers use OpticalProjection.
pub fn optical_route(r:&Run,a:&Actor,target:Point)->Option<crate::portals::Route> {
    let _timing_function = crate::timing::span("vision.optical_route");
 let f=crate::dreamwalking::floor_for(r,a).or_else(||r.floor())?;
 if crate::portals::enabled(r) && f.portals.as_ref().is_some_and(|p|!p.is_empty()){crate::portals::route_filtered(f,point(a),target,false,|x,y|floor_ray(f,x,y),|a,b|[a,b].into_iter().all(|p|crate::portals::cell(f,p.x as f64,p.y as f64).is_some_and(|i|!matches!(f.tiles[i].as_str(),"wall"|"tree")&&!crate::environment::blocks_sight(f,i))))}
 else{floor_ray(f,point(a),target).then(||crate::portals::Route{distance:distance(point(a),target),points:vec![point(a),target]})}
}
pub fn line_of_sight(r: &Run, a: &Actor, b: &Actor) -> bool { optical_route(r,a,point(b)).is_some() }
fn origins(r: &Run) -> Vec<&Actor> {
    let mut out = vec![&r.player];
    if has(&r.player, "etill-half-here") && active(&r.player, "etill-half-here") {
        if let Some(a) = r.actors_at(r.depth).iter().find(|a| {
            !a.dead.value_or(false)
                && condition(a, "halfHereClone") == 1.0
                && a.id == condition(&r.player, "halfHereId")
                && condition(a, "halfHereOwner") == r.player.id
        }) {
            out.push(a);
        }
    }
    out
}
fn viewpoints(r: &Run) -> Vec<&Actor> {
    let mut out = origins(r);
    out.extend(r.actors_at(r.depth).iter().filter(|a| {
        a.perfect.value_or(false) && !a.dead.value_or(false) && condition(a, "halfHereClone") != 1.0
    }));
    out
}
fn illumination(r: &Run, target: Point, exclude: Option<f64>) -> f64 {
    illumination_in_plane(r,target,exclude,crate::dreamwalking::context_dream(r))
}
/// O(F + D + fire-window*R + L*R + A + A_near*B + emitting_near*R), plus
/// ring_light_bonus's optical-query cost. D is the R52 potion mask; covered
/// cells return after O(F+D). Otherwise O(1) scratch apart from ring queries.
/// Distant actors skip ability/body/plane checks; old subtraction order stays.
pub(crate) fn illumination_in_plane(r:&Run,target:Point,exclude:Option<f64>,dream:bool)->f64 {
    let _timing_function = crate::timing::span("vision.lighting");
    let Some(f) = crate::dreamwalking::floor_in_plane(r,dream).or_else(||r.floor()) else { return -100.0 };
    if !bounds(f, target) {
        return -100.0;
    }
    // R52 applies a final effective-light override, including later/mobile lights.
    // Early return is equivalent without mutating or clamping any source values.
    if crate::revision50_potions::absolute_darkness_at(r,f,target.y as usize*f.width()+target.x as usize) {return -100.;}
    let mut result = f
        .lights
        .get(target.y as usize * f.width() + target.x as usize)
        .copied()
        .unwrap_or(0.0)
        .max(-100.0);
    if let Some(e) = f.environment.as_ref() {
        for y in (target.y - 7).max(0)..=(target.y + 7).min(f.height() as i32 - 1) {
            for x in (target.x - 7).max(0)..=(target.x + 7).min(f.width() as i32 - 1) {
                if e.fire
                    .get(y as usize * f.width() + x as usize)
                    .copied()
                    .unwrap_or(0.0)
                    <= 0.0
                {
                    continue;
                }
                let source = Point { x, y };
                let amount = 80.0 - 10.0 * distance(source, target);
                if amount > 0.0 && amount > result && floor_ray(f, source, target) {
                    result = amount;
                }
            }
        }
    }
    if let Some(sources) = f.light_sources.as_ref() {
        for s in sources {
            if !s.power.is_finite() || !s.falloff.is_finite() || s.falloff <= 0.0 {
                continue;
            }
            let source = Point {
                x: s.x as i32,
                y: s.y as i32,
            };
            let amount = s.power - 10.0 * distance(source, target);
            if amount > 0.0 && amount > result && floor_ray(f, source, target) {
                result = amount;
            }
        }
    }
    let dream_enabled = crate::dreamwalking::enabled(r);
    for a in std::iter::once(&r.player).chain(r.actors_at(r.depth)) {
        // Shine uses integer-cast positions. Raw fractional/NaN positions cannot
        // be used for this cull. i64 differences avoid extreme cast overflow.
        if result >= 100. { break; }
        let source = point(a);
        if (i64::from(source.x)-i64::from(target.x)).abs() >= 10
            || (i64::from(source.y)-i64::from(target.y)).abs() >= 10 { continue; }
        if (dream_enabled && crate::dreamwalking::dream_actor(r,a)!=dream) || crate::dreamwalking::is_body(a) || a.dead.value_or(false)
            || !emits_shine(r, a)
            || (exclude == Some(a.id) && active(a, "anavani-shine"))
        {
            continue;
        }
        let amount = 100.0 - 10.0 * distance(source, target);
        if amount > 0.0 && amount > result && floor_ray(f, source, target) {
            result = amount;
        }
    }
    result = result.clamp(-100.0, 100.0);
    for a in std::iter::once(&r.player).chain(r.actors_at(r.depth)) {
        // Keep the exact fractional Manhattan expression and max: NaN becomes
        // zero as in the original, while infinity contributes no darkness.
        let amount = (100.0
            - 10.0 * ((a.x - target.x as f64).abs() + (a.y - target.y as f64).abs()))
        .max(0.0);
        if !(amount > 0.0) { continue; }
        if (dream_enabled && crate::dreamwalking::dream_actor(r,a)!=dream) || a.dead.value_or(false) || a.hp <= 0.0 || !has(a, "enemy:beast:darkness-aura") {
            continue;
        }
        if floor_ray(f, point(a), target) { result -= amount; }
    }
    result = result.clamp(-100.0, 100.0);
    if crate::revision50::enabled(r) {result=(result-crate::revision50_potions::darkness_at(f,target.y as usize*f.width()+target.x as usize,r.tick)).clamp(-100.,100.);}
    // Ring powers add after ambient/fire/Shine/darkness have been resolved.
    result += ring_light_bonus(r,target,dream);
    result.clamp(-100.0, 100.0)
}

pub fn tile_light(r: &Run, x: usize, y: usize) -> f64 {
    illumination(
        r,
        Point {
            x: x as i32,
            y: y as i32,
        },
        None,
    )
}
/// Actor-only modifiers never alter the tile field or other occupants.
/// O(B+I) ability/equipped-ring checks; no allocation or RNG.
fn actor_light_modifiers(r: &Run, a: &Actor, mut l: f64, ring_bonus: f64) -> f64 {
    if has(a, "enemy:beast:darkness-aura") {
        if !crate::revision50::enabled(r) { return -100.0; }
        l = -100.0 + ring_bonus;
    } else if has(a, "dawel-shadow-in-the-dark") {
        if active(a, "dawel-shadow-in-the-dark") {
            l -= 20.0;
        }
        if l < 0.0 {
            l *= 2.0;
        }
    }
    if l < 0.0 { l *= crate::revision50_rings::stack_multiplier(r,a,"stealth",1.2); }
    l.clamp(-100.0, 100.0)
}
pub fn light(r: &Run, a: &Actor) -> f64 {
    if has(a,"enemy:beast:darkness-aura") {
        return actor_light_modifiers(r,a,-100.,ring_light_bonus(r,point(a),crate::dreamwalking::context_dream(r)));
    }
    actor_light_modifiers(r,a,illumination(r, point(a), Some(a.id)),0.)
}
/// O(F + optical route + scalar light + B) time, O(route scratch) memory.
/// Same-call sample only: immutable inputs, no persistent state or RNG changes.
/// Missing floor/out-of-bounds implies no legal optical route: every edge from
/// the source fails bounds, or every final target edge does. Preserve that result.
fn actor_chance_sample(r: &Run, observer: &Actor, target: &Actor, perception: f64) -> (f64, bool, Option<f64>) {
    let Some(f) = crate::dreamwalking::floor_for(r,observer).or_else(||r.floor()) else { return (0.0, false, None) };
    if !bounds(f, point(observer)) || !bounds(f, point(target)) {
        return (0.0, false, None);
    }
    let underwater = f.tiles[observer.y as usize * f.width() + observer.x as usize] == "water"
        && f.tiles[target.y as usize * f.width() + target.x as usize] == "water";
    let route = optical_route(r, observer, point(target));
    // No optical route forces probability zero before any lighting modifier.
    // This is same-call reuse, not a cache across mutable simulation.
    if route.is_none() { return (0.0, false, None); }
    let current_plane = !(crate::dreamwalking::enabled(r)
        && crate::dreamwalking::dream_actor(r,observer)!=crate::dreamwalking::context_dream(r));
    let l = if current_plane { light(r,target) } else {
        let raw = illumination_in_plane(r,point(target),Some(target.id),crate::dreamwalking::dream_actor(r,observer));
        if r.simulation_revision.value_or(29.) >= 50. { actor_light_modifiers(r,target,raw,if has(target,"enemy:beast:darkness-aura") {
            ring_light_bonus(r,point(target),crate::dreamwalking::dream_actor(r,observer))
        } else {0.}) } else { raw }
    };
    let los = route.is_some();
    let chance = optical_chance(
        r, perception,
        route.as_ref().map_or(f64::INFINITY,|p|p.distance),
        condition(target, "stealth"),
        condition(target, "submergedStealth"),
        l,
        underwater,
        los,
        condition(target, "invisible") > 0.0,
        true,
    );
    (chance, los, current_plane.then_some(l))
}
/// Same cost as actor_chance_sample; public scalar probability contract unchanged.
pub fn actor_chance(r: &Run, observer: &Actor, target: &Actor, perception: f64) -> f64 {
    actor_chance_sample(r,observer,target,perception).0
}
/// O(K²R + Ls) time, where K portals, R ray length, Ls sparse lighting
/// cost only for a cross-plane observer. Geometry is delegated unchanged.
fn tile_chance(r: &Run, observer: &Actor, target: Point, l: f64) -> f64 {
    let Some(f) = crate::dreamwalking::floor_for(r,observer).or_else(||r.floor()) else { return 0.0 };
    let l=if crate::dreamwalking::enabled(r) && crate::dreamwalking::dream_actor(r,observer)!=crate::dreamwalking::context_dream(r) {illumination_in_plane(r,target,None,crate::dreamwalking::dream_actor(r,observer))}else{l};
    tile_chance_lit(r, observer, target, l, f)
}
/// Immutable per-view geometry. For G distinct floors (at most two), O(G*(N+K))
/// exact guards, cold fields O(G*KNR), cold observers O(P*(K²+KR+N*(R+K))).
/// Expected O(P) index setup/handles, O(PN) observer outputs; no world mutation.
struct OpticalProjection<'a> {
    entries: std::collections::HashMap<usize, (usize, usize, std::sync::Arc<Vec<f64>>, f64)>,
    _borrow: std::marker::PhantomData<&'a Actor>,
}
impl<'a> OpticalProjection<'a> {
    /// Build exact fields for viewers, capturing each borrowed floor only once.
    /// See type-level complexity; hash lookup expected O(1), worst O(P).
    fn new(r: &Run, views: &[&'a Actor]) -> Self {
        let mut fields=std::collections::HashMap::new();
        let entries=views.iter().filter_map(|&a|{
            let f=crate::dreamwalking::floor_for(r,a).or_else(||r.floor())?;
            let field=fields.entry(f as *const Floor as usize)
                .or_insert_with(||r.optical_cache.field(f,crate::portals::enabled(r)));
            Some((a as *const Actor as usize,(f.width(),f.height(),field.distances(point(a)),observer_perception(r,a))))
        }).collect();
        Self { entries, _borrow:std::marker::PhantomData }
    }
    fn perception(&self, a: &Actor) -> f64 {
        self.entries.get(&(a as *const Actor as usize)).map_or(0.,|entry|entry.3)
    }
    /// Expected O(1) viewer/cell lookup (hash worst O(P)); no ray/graph search.
    fn distance(&self, a: &Actor, p: Point) -> f64 {
        self.entries.get(&(a as *const Actor as usize)).map_or(f64::INFINITY, |(w,h,d,_)| {
            if p.x<0||p.y<0||p.x as usize>=*w||p.y as usize>=*h{f64::INFINITY}
            else{d[p.y as usize * *w+p.x as usize]}
        })
    }
}
/// Cached geometry/light probability: expected O(F+B), worst O(F+B+P) for
/// viewer hash lookup; F floor lookup, B plane abilities. No rays or RNG.
fn tile_chance_projected(r: &Run, observer: &Actor, target: Point,
    projection: &crate::lighting_cache::LightingProjection, geometry: &OpticalProjection) -> f64 {
    let Some(f) = crate::dreamwalking::floor_for(r,observer).or_else(||r.floor()) else { return 0.0 };
    let plane = if crate::dreamwalking::enabled(r) {
        crate::dreamwalking::dream_actor(r, observer)
    } else { crate::dreamwalking::context_dream(r) };
    if !bounds(f, point(observer)) || !bounds(f, target) { return 0.; }
    let d = geometry.distance(observer, target);
    let wet = f.tiles[observer.y as usize * f.width() + observer.x as usize] == "water"
        && f.tiles[target.y as usize * f.width() + target.x as usize] == "water";
    optical_chance(r, geometry.perception(observer), d, 0., 0., projection.in_plane(target, plane), wet, d.is_finite(), false, false)
}
/// One optical-route call supplies both distance and existence; O(K²R)
/// on the portal graph and O(1) extra local storage apart from those route helpers.
fn tile_chance_lit(r: &Run, observer: &Actor, target: Point, l: f64, f: &Floor) -> f64 {
    if !bounds(f, point(observer)) || !bounds(f, target) {
        return 0.0;
    }
    let wet = f.tiles[observer.y as usize * f.width() + observer.x as usize] == "water"
        && f.tiles[target.y as usize * f.width() + target.x as usize] == "water";
    let route = optical_route(r, observer, target);
    optical_chance(
        r, observer_perception(r,observer),
        route.as_ref().map_or(f64::INFINITY,|p|p.distance),
        0.0,
        0.0,
        l,
        wet,
        route.is_some(),
        false,
        false,
    )
}
pub fn all_seeing(r: &Run) -> bool {
    condition(&r.player, "allSeeingReveal") == 1.0
        && condition(&r.player, "allSeeingDepth") == r.depth
}
pub fn telepathic(r: &Run, target: &Actor) -> bool {
    telepathic_from_origins(r, target, &origins(r))
}
fn telepathic_from_origins(r: &Run, target: &Actor, origins: &[&Actor]) -> bool {
    telepathic_for_owner(r, &r.player, target, origins)
}
fn telepathic_for_owner(r: &Run, p: &Actor, target: &Actor, origins: &[&Actor]) -> bool {
    if origins.iter().any(|a| crate::revision50_combat::witch_senses(r,a,target)) { return true; }
    if target.dead.value_or(false) {
        return false;
    }
    (p.id==r.player.id && target.perfect.value_or(false))
        || origins.iter().any(|o| condition(o, "telepathic") > 0.0)
        || (condition(p, "cameTurns") > 0.0 && condition(p, "cameTarget") == target.id)
        || (has(p, "numo-mind-read")
            && origins.iter().any(|o| {
                distance(point(o), point(target))
                    <= if active(p, "numo-mind-read") {
                        20.0
                    } else {
                        5.0
                    }
            }))
        || (has(p, "ursa-dream-see")
            && (sleeping(target)
                || (active(p, "ursa-dream-see")
                    && origins.iter().any(|o| line_of_sight(r, o, target)))))
}
pub fn telepathic_area(r: &Run, p: Point) -> bool {
    r.floor().is_some_and(|f| bounds(f, p))
        && r.actors_at(r.depth).iter().any(|a| {
            (a.x - p.x as f64).powi(2) + (a.y - p.y as f64).powi(2) <= 4.0 && telepathic(r, a)
        })
}
pub fn visible(r: &Run, x: usize, y: usize) -> bool {
    let p = Point {
        x: x as i32,
        y: y as i32,
    };
    if !r.floor().is_some_and(|f| bounds(f, p)) {
        return false;
    }
    if all_seeing(r) || telepathic_area(r, p) {
        return true;
    }
    let l = tile_light(r, x, y);
    viewpoints(r).iter().any(|a| tile_chance(r, a, p, l) > 5.0)
}
/// Direct optical cell admission for detached action paths, excluding sensed-only
/// area knowledge. O(P*(light/LOS)), preserving current visible() optical policy.
pub fn directly_visible_tile(r:&Run,x:usize,y:usize)->bool {
    let p=Point{x:x as i32,y:y as i32};
    if !r.floor().is_some_and(|f|bounds(f,p)){return false}
    if all_seeing(r){return true}
    let light=tile_light(r,x,y);
    viewpoints(r).iter().filter(|a|crate::dreamwalking::same_plane(r,&r.player,a))
        .any(|a|tile_chance(r,a,p,light)>5.)
}
/// Actor-local targeting visibility. Player calls preserve the existing shared
/// viewpoints. Independent copies use their own senses and plane, never the
/// player's reveal mask. O(A*sense/LOS cost + one tile-light/optical query).
pub fn visible_from(r: &Run, observer: &Actor, x: usize, y: usize) -> bool {
    if observer.id==r.player.id {return visible(r,x,y);}
    let p=Point{x:x as i32,y:y as i32};
    if !crate::dreamwalking::floor_for(r,observer).is_some_and(|f|bounds(f,p)) {return false;}
    if condition(observer,"allSeeingReveal")==1.0 && condition(observer,"allSeeingDepth")==r.depth {return true;}
    if r.actors_at(r.depth).iter().chain(std::iter::once(&r.player)).any(|a|
        crate::dreamwalking::same_plane(r,observer,a)
            && (a.x-x as f64).powi(2)+(a.y-y as f64).powi(2)<=4.0
            && telepathic_for_owner(r,observer,a,&[observer])) {return true;}
    tile_chance(r,observer,p,tile_light(r,x,y))>5.0
}
pub fn directly_visible_actor(r: &Run, a: &Actor) -> bool {
    if crate::dreamwalking::is_nightmare(a) {return false;}
    if !r.floor().is_some_and(|f| bounds(f, point(a))) {
        return false;
    }
    let l = tile_light(r, a.x as usize, a.y as usize);
    let views = viewpoints(r);
    if !views.iter().any(|o| tile_chance(r, o, point(a), l) > 5.0)
        || condition(a, "invisible") > 0.0
    {
        return false;
    }
    if a.perfect.value_or(false) && !a.dead.value_or(false) {
        return true;
    }
    actor_chance(r, &r.player, a, observer_perception(r,&r.player)) >= 50.0
        || views
            .iter()
            .any(|o| o.id != r.player.id && actor_chance(r, o, a, observer_perception(r,o)) >= 50.0)
}
pub fn visible_actor(r: &Run, a: &Actor) -> bool {
    if crate::dreamwalking::is_nightmare(a) {return crate::dreamwalking::dream_view(r);}
    (all_seeing(r) && r.floor().is_some_and(|f| bounds(f, point(a))))
        || telepathic(r, a)
        || telepathic_area(r, point(a))
        || directly_visible_actor(r, a)
}
/// Borrowed visibility batch: exact geometry/light snapshots cannot outlive an
/// immutable Run borrow. Setup includes O(N+A*B+L) dependency guards and dirty
/// field construction; queries avoid repeating source scans and route searches.
pub struct VisibilityProjection<'a> {
    run: &'a Run,
    lighting: std::cell::OnceCell<crate::lighting_cache::LightingProjection<'a>>,
    geometry: std::cell::OnceCell<OpticalProjection<'a>>,
    views: Vec<&'a Actor>,
}
impl<'a> VisibilityProjection<'a> {
    /// O(A*B) viewpoint collection, O(P) handles; no field build until needed.
    /// First optical query adds C_light+C_geometry; fields retain O(PN) data.
    pub fn new(run: &'a Run) -> Self {
        let views=viewpoints(run);
        Self {run, lighting:std::cell::OnceCell::new(), geometry:std::cell::OnceCell::new(),views}
    }
    /// O(1) warm; first actual optical query pays documented projection setup.
    fn lighting(&self) -> &crate::lighting_cache::LightingProjection<'a> {
        self.lighting.get_or_init(||crate::lighting_cache::LightingProjection::new(self.run))
    }
    /// O(1) warm; first actual optical query pays documented geometry setup.
    fn geometry(&self) -> &OpticalProjection<'a> {
        self.geometry.get_or_init(||OpticalProjection::new(self.run,&self.views))
    }
    /// O(B) target modifier checks and O(1) excluded-Shine light lookup.
    fn light(&self, target: &Actor) -> f64 {
        actor_light_modifiers(self.run,target,self.lighting().current_excluding(point(target),target.id),
            self.lighting().ring_bonus_in_plane(point(target),crate::dreamwalking::context_dream(self.run)))
    }
    /// O(P+B+F) query (view lookup, abilities and floor selection), no rays.
    fn actor_chance(&self, observer: &Actor, target: &Actor) -> f64 {
        let r=self.run;
        let Some(f)=crate::dreamwalking::floor_for(r,observer).or_else(||r.floor())else{return 0.};
        if !bounds(f,point(observer))||!bounds(f,point(target)){return 0.}
        let wet=f.tiles[observer.y as usize*f.width()+observer.x as usize]=="water"
            && f.tiles[target.y as usize*f.width()+target.x as usize]=="water";
        let d=self.geometry().distance(observer,point(target));
        let l=if crate::dreamwalking::enabled(r)&&crate::dreamwalking::dream_actor(r,observer)!=crate::dreamwalking::context_dream(r){
            let raw=self.lighting().in_plane_excluding(point(target),crate::dreamwalking::dream_actor(r,observer),target.id);
            if r.simulation_revision.value_or(29.) >= 50. { actor_light_modifiers(r,target,raw,self.lighting().ring_bonus_in_plane(point(target),crate::dreamwalking::dream_actor(r,observer))) } else { raw }
        }else{self.light(target)};
        optical_chance(r,self.geometry().perception(observer),d,condition(target,"stealth"),condition(target,"submergedStealth"),l,wet,d.is_finite(),condition(target,"invisible")>0.,true)
    }
    /// O(P*(P+B+F)) worst-case, no repeated light/source/ray traversal.
    pub fn directly_visible_actor(&self, a: &Actor) -> bool {
        let r=self.run;
        if crate::dreamwalking::is_nightmare(a)||!r.floor().is_some_and(|f|bounds(f,point(a))){return false}
        if !self.views.iter().any(|o|tile_chance_projected(r,o,point(a),self.lighting(),self.geometry())>5.)
            ||condition(a,"invisible")>0.{return false}
        if a.perfect.value_or(false)&&!a.dead.value_or(false){return true}
        self.actor_chance(&r.player,a)>=50.||self.views.iter().any(|o|o.id!=r.player.id&&self.actor_chance(o,a)>=50.)
    }
    /// O(telepathic + telepathic_area + P*(P+B+F)); preserves special reveal,
    /// nightmare, ally and plane handling. No gameplay mutation or RNG.
    pub fn visible_actor(&self,a:&Actor)->bool{
        crate::timing::count(crate::timing::Count::ActorQueries,1);
        let r=self.run;
        if crate::dreamwalking::is_nightmare(a){return crate::dreamwalking::dream_view(r)}
        (all_seeing(r)&&r.floor().is_some_and(|f|bounds(f,point(a))))||telepathic(r,a)
            ||telepathic_area(r,point(a))||self.directly_visible_actor(a)
    }
}
/// Actor-local eligibility, without an awareness lottery or the player's shared
/// viewpoint union. O(stats + optical route + scalar lighting + senses), no RNG.
pub fn plausibly_visible_actor(r: &Run, observer: &Actor, target: &Actor) -> bool {
    if !crate::dreamwalking::same_plane(r,observer,target) {return false;}
    let Some(f)=crate::dreamwalking::floor_for(r,observer).or_else(||r.floor()) else{return false};
    if !bounds(f,point(observer)) || !bounds(f,point(target)) {return false;}
    if condition(observer,"allSeeingReveal")==1. && condition(observer,"allSeeingDepth")==r.depth {
        return true;
    }
    if telepathic_for_owner(r,observer,target,&[observer]) {return true;}
    actor_chance(r,observer,target,observer_perception(r,observer))>0.
}
/// Apply only publicly known bearer modifiers to a detached cell's illumination.
/// O(B+I) status/equipment reads, no hidden geometry, light-source reads or RNG.
pub fn perception_actor_light(r: &Run, target: &Actor, raw_light: f64, ring_light_bonus: f64) -> f64 {
    actor_light_modifiers(r,target,raw_light,ring_light_bonus)
}
/// Full simulation optical mask, not a knowledge-safe UI overlay. Desktop must
/// construct its overlay from detached observations to avoid hidden blockers.
/// O(cold lighting/geometry + N*F), O(N) output plus existing projection fields.
pub fn perception_tile_mask(r: &Run, observer: &Actor) -> Vec<bool> {
    let Some(f)=crate::dreamwalking::floor_for(r,observer).or_else(||r.floor()) else{return vec![]};
    let lighting=crate::lighting_cache::LightingProjection::new(r);
    let geometry=OpticalProjection::new(r,&[observer]);
    (0..f.tiles.len()).map(|i|tile_chance_projected(r,observer,
        Point{x:(i%f.width())as i32,y:(i/f.width())as i32},&lighting,&geometry)>5.).collect()
}
/// O(stats + optical route + scalar light), plus a second light query only for
/// cross-context-plane zero-chance fallback. O(route scratch) memory; no RNG.
pub fn detection_chance(r: &Run, observer: &Actor, target: &Actor) -> f64 {
    let _timing_function = crate::timing::span("vision.detection");
    if crate::revision51_perception::enabled(r) {
        let plausible = plausibly_visible_actor(r,observer,target);
        let d = if sleeping(observer) {
            optical_route(r,observer,point(target)).map_or_else(
                ||distance(point(observer),point(target)),|route|route.distance)
        } else {0.};
        return crate::revision51_perception::awareness_chance(plausible,
            observer.id==r.player.id || observer.state=="ally", sleeping(observer),
            observer.state=="hunting" || (observer.state=="stalking" && observer.last_seen.as_ref().is_some()), d);
    }
    if !crate::dreamwalking::same_plane(r,observer,target) {return 0.;}
    if !target.dead.value_or(false) && condition(observer, "telepathic") > 0.0 {
        return 100.0;
    }
    if condition(target, "invisible") > 0.0 {
        return 0.0;
    }
    let mut p = crate::combat::effective_stats(r, observer).perception;
    if observer.state == "hunting" {
        p *= 2.0;
    }
    if sleeping(observer) {
        p *= 0.5;
    }
    let (base, los, current_light) = actor_chance_sample(r, observer, target, p);
    if observer.id == r.player.id || observer.state == "ally" {
        return base;
    }
    if base == 0.0 && (!los || current_light.unwrap_or_else(|| light(r, target)) <= -100.0) {
        return 0.0;
    }
    if observer.state == "hunting" {
        if condition(observer, "targetId") == target.id
            && distance(point(observer), point(target)) < 5.0
        {
            return 100.0;
        }
        100.0 - (100.0 - base) / 3.0
    } else {
        base / 2.0
    }
}
/// O(C + V*(T + P*(F+B))) expected time after field setup C, O(V+A+PN)
/// result/fields. T telepathic_area, P viewpoints, F floor lookup, B plane abilities.
/// C includes exact guards and cold light/optical fields; hash worst adds O(P) per query.
pub fn terrain_visibility(r: &Run) -> Vec<bool> {
    let _timing_function = crate::timing::span("vision.terrain_mask");
    let Some(f) = r.floor() else { return vec![] };
    // Reveal already decides every cell: preserve the original no-lighting shortcut.
    if all_seeing(r) { return vec![true; f.tiles.len()]; }
    let w = f.width();
    let lighting = crate::lighting_cache::LightingProjection::new(r);
    let views = viewpoints(r);
    let geometry = OpticalProjection::new(r, &views);
    (0..f.tiles.len())
        .map(|i| {
            let p = Point { x: (i % w) as i32, y: (i / w) as i32 };
            all_seeing(r) || telepathic_area(r, p)
                || views.iter().any(|a| tile_chance_projected(r, a, p, &lighting, &geometry) > 5.0)
        })
        .collect()
}
/// Memory admission for the current floor, not the union of both planes.
/// O(C + sensed-actor queries + V*(A+P*(F+B))) expected time, O(A+PN+V)
/// fields/result. C includes exact light/optical guards and cold field builds;
/// sleeping bodies are not eyes. Viewer hash worst adds O(P) per cell/view.
pub fn native_terrain_visibility(r: &Run) -> Vec<bool> {
    let _timing_function = crate::timing::span("vision.native_terrain_mask");
    if !crate::dreamwalking::enabled(r) {
        return terrain_visibility(r);
    }
    let Some(f) = r.floor() else { return vec![] };
    let native_origins: Vec<_> = origins(r).into_iter()
        .filter(|a| crate::dreamwalking::contact_plane(r, a) && !crate::dreamwalking::is_body(a)
            && !a.dead.value_or(false)).collect();
    let views: Vec<_> = viewpoints(r).into_iter()
        .filter(|a| crate::dreamwalking::contact_plane(r, a) && !crate::dreamwalking::is_body(a)
            && !a.dead.value_or(false)).collect();
    let reveal = !native_origins.is_empty() && all_seeing(r);
    if reveal { return vec![true; f.tiles.len()]; }
    let sensed: Vec<_> = r.actors_at(r.depth).iter()
        .filter(|a| crate::dreamwalking::contact_plane(r, a)
            && if native_origins.is_empty() {
                a.perfect.value_or(false) && !a.dead.value_or(false)
            } else {
                telepathic_from_origins(r, a, &native_origins)
            })
        .map(point).collect();
    let w = f.width();
    let lighting = crate::lighting_cache::LightingProjection::new(r);
    let geometry = OpticalProjection::new(r, &views);
    (0..f.tiles.len()).map(|i| {
        let p = Point { x: (i % w) as i32, y: (i / w) as i32 };
        reveal || sensed.iter().any(|a| (a.x - p.x).pow(2) + (a.y - p.y).pow(2) <= 4)
            || views.iter().any(|a| tile_chance_projected(r, a, p, &lighting, &geometry) > 5.0)
    }).collect()
}
/// O(C + V*(T+P*(F+B))) expected time, O(V+A+PN) fields/result; same
/// symbols and hash/cold-field caveats as terrain_visibility, without view short-circuit.
pub fn terrain_visibility_chance(r: &Run) -> Vec<f64> {
    let Some(f) = r.floor() else { return vec![] };
    if all_seeing(r) { return vec![100.; f.tiles.len()]; }
    let w = f.width();
    let views = viewpoints(r);
    let lighting = crate::lighting_cache::LightingProjection::new(r);
    let geometry = OpticalProjection::new(r, &views);
    (0..f.tiles.len())
        .map(|i| {
            let p = Point {
                x: (i % w) as i32,
                y: (i / w) as i32,
            };
            if all_seeing(r) || telepathic_area(r, p) {
                100.0
            } else {
                views
                    .iter()
                    .map(|a| tile_chance_projected(r, a, p, &lighting, &geometry))
                    .fold(0.0, f64::max)
            }
        })
        .collect()
}
#[derive(Debug, Clone)]
pub struct TerrainObservation {
    pub all_seeing: bool,
    pub width: usize,
    pub height: usize,
    pub cells: Vec<ObservedCell>,
    pub light_sources: Vec<ObservedLightSource>,
}
#[derive(Debug, Clone)]
pub struct ObservedCell {
    pub index: usize,
    pub current: bool,
    pub remembered: bool,
    pub knowledge: String,
    pub direct: bool,
    pub tile: Option<String>,
    pub substrate: Option<String>,
    pub door_open: Option<bool>,
    pub snow: bool,
    pub ice: bool,
    pub puddles: bool,
    pub hellfire: bool,
    /// Immutable biome metadata is exposed only with known terrain.
    pub biome: Option<String>,
    pub flowers: bool,
    pub grate: bool,
    pub bridge: bool,
    /// Persistent floor-one marker, exposed only for known entrance terrain.
    pub entrance: bool,
    pub chance: f64,
    pub direct_chance: f64,
    pub illumination: Option<f64>,
    pub ambient: Option<f64>,
    pub distance: f64,
    pub hazards: Option<crate::model::HazardSnapshot>,
}
#[derive(Debug, Clone)]
pub struct ObservedLightSource {
    pub origin_known: bool,
    pub emission_group: usize,
    pub x: f64,
    pub y: f64,
    pub kind: String,
    pub color_role: String,
    pub power: f64,
    pub falloff: f64,
    pub actor_id: Option<f64>,
    pub cells: Vec<usize>,
    pub strengths: Vec<f64>,
}
/// Detached knowledge-filtered observation. No hidden source origin, terrain,
/// hazard, or ambient value crosses this boundary. Current illumination can
/// legitimately be influenced by an unseen source without exposing its identity.
/// Expected O(C + V*(P*(F+B)+T+H+M) + A*B+L + S*(Q+V*R)) time;
/// O(V+H+P*V+S*V) fields/output. C cold/cache guards; F floor lookup; B abilities;
/// T telepathy; Q scalar emitter visibility; S sources; A actors; L placed lights.
/// Viewer-hash worst adds O(V*P²). Exact geometry/light fields, not whole observations.
pub fn observe_terrain(r: &Run) -> Option<TerrainObservation> {
    let _timing_function = crate::timing::span("vision.observe_terrain");
    let setup_span=crate::timing::span("vision.observe_setup");
    let context_floor = r.floor()?;
    let player_dream = crate::dreamwalking::enabled(r)
        && crate::dreamwalking::dream_actor(r, &r.player);
    let f = crate::dreamwalking::floor_for(r, &r.player).unwrap_or(context_floor);
    let w = f.width();
    let views = viewpoints(r);
    let lighting = crate::lighting_cache::LightingProjection::new(r);
    let geometry = OpticalProjection::new(r, &views);
    let all_seeing = all_seeing(r);
    // At most two floors. Build O(H) hazard indexes once per floor, not per cell;
    // per-cell selection adds O(P) work and O(V) plane tags to existing LOS work.
    let mut floors = vec![(f, player_dream)];
    if views.iter().any(|a| crate::dreamwalking::dream_actor(r, a) != player_dream) {
        if let Some(other) = crate::dreamwalking::floor_in_plane(r, !player_dream) {
            if !std::ptr::eq(f, other) && f.width() == other.width()
                && f.height() == other.height() && f.tiles.len() == other.tiles.len() {
                floors.push((other, !player_dream));
            }
        }
    }
    let hazard_contexts: Vec<_> = floors.iter()
        .map(|(f, _)| crate::hazard_memory::observation_context_revision(f, r.simulation_revision.value_or(29.0))).collect();
    let entrances: Vec<_> = floors.iter().map(|(f, _)| crate::entrance::metadata_cell(f)).collect();
    let view_planes: Vec<_> = views.iter().map(|a| {
        usize::from(floors.len() > 1 && crate::dreamwalking::dream_actor(r, a) != player_dream)
    }).collect();
    drop(setup_span);
    let cells_span=crate::timing::span("vision.observe_cell_projection");
    let mut cell_planes = Vec::with_capacity(f.tiles.len());
    let cells: Vec<ObservedCell> = (0..f.tiles.len())
        .map(|i| {
            let p = Point { x: (i % w) as i32, y: (i / w) as i32 };
            let mut direct_chances = [0.0_f64; 2];
            for (a, &plane) in views.iter().zip(&view_planes) {
                direct_chances[plane] = direct_chances[plane].max(tile_chance_projected(r, a, p, &lighting, &geometry));
            }
            let direct_chance = direct_chances[0].max(direct_chances[1]);
            let chance = if all_seeing || telepathic_area(r, p) { 100.0 } else { direct_chance };
            let current = chance > 5.0;
            // A stronger clone roll must not override terrain the player plane sees.
            let plane = usize::from(!all_seeing && direct_chances[0] <= 5.0 && direct_chances[1] > 5.0);
            cell_planes.push(plane);
            let (f, dream) = floors[plane];
            let hazard_context = &hazard_contexts[plane];
            let entrance = entrances[plane];
            let remembered = f.seen.get(i).copied().unwrap_or(false);
            let direct = all_seeing || direct_chance > 5.0;
            ObservedCell {
                index: i,
                current,
                remembered,
                direct,
                knowledge: if current {
                    if direct { "direct" } else { "telepathic" }
                } else if remembered {
                    "remembered"
                } else {
                    "layout"
                }
                .into(),
                tile: if current {
                    Some(f.tiles[i].clone())
                } else if remembered {
                    f.remembered_tiles.as_ref().and_then(|v| v.get(i)).cloned()
                } else {
                    None
                },
                substrate: if current {
                    f.substrates.as_ref().and_then(|v| v.get(i)).cloned()
                } else if remembered {
                    f.remembered_substrates
                        .as_ref()
                        .and_then(|v| v.get(i))
                        .cloned()
                } else {
                    None
                },
                door_open: if current {
                    f.doors
                        .as_ref()
                        .and_then(|m| m.get(&i.to_string()))
                        .map(|d| d.open)
                } else if remembered {
                    f.remembered_doors
                        .as_ref()
                        .and_then(|m| m.get(&i.to_string()))
                        .map(|d| d.open)
                } else {
                    None
                },
                biome: if current || remembered {crate::pocket_biomes::at(f,i).map(|b|b.theme.clone())} else {None},
                flowers: crate::pocket_biomes::flower_at(f,i) && if current {f.tiles[i]=="grass"} else if remembered {f.remembered_tiles.as_ref().and_then(|v|v.get(i)).is_some_and(|t|t=="grass")} else {false},
                snow: if current {
                    f.snow
                        .as_ref()
                        .and_then(|v| v.get(i))
                        .copied()
                        .unwrap_or(false)
                } else if remembered {
                    f.remembered_snow
                        .as_ref()
                        .and_then(|v| v.get(i))
                        .copied()
                        .unwrap_or(false)
                } else {
                    false
                },
                ice: if current {
                    f.ice.as_ref().and_then(|v|v.get(i)).copied().unwrap_or(false)
                } else if remembered {
                    f.remembered_ice.as_ref().and_then(|v|v.get(i)).copied().unwrap_or(false)
                } else { false },
                puddles: if current {
                    f.puddles.as_ref().and_then(|v|v.get(i)).copied().unwrap_or(false)
                } else if remembered {
                    f.remembered_puddles.as_ref().and_then(|v|v.get(i)).copied().unwrap_or(false)
                } else { false },
                hellfire: if current {
                    f.hellfire.as_ref().and_then(|v|v.get(i)).copied().unwrap_or(false)
                } else if remembered {
                    f.remembered_hellfire.as_ref().and_then(|v|v.get(i)).copied().unwrap_or(false)
                } else { false },
                grate: (current || remembered)
                    && f.grates.as_ref().is_some_and(|v| v.contains(&(i as f64))),
                bridge: (current || remembered)
                    && f.generation_bridges
                        .as_ref()
                        .is_some_and(|v| v.contains(&(i as f64))),
                entrance: entrance == Some(i) && if current {
                    f.tiles[i] == "up"
                } else {
                    remembered && f.remembered_tiles.as_ref().and_then(|v| v.get(i))
                        .is_some_and(|tile| tile == "up")
                },
                chance,
                direct_chance,
                illumination: current.then(|| lighting.in_plane(p, dream)),
                ambient: current
                    .then(|| f.lights.get(i).copied().unwrap_or(0.0).clamp(-100.0, 100.0)),
                distance: ((i % w) as f64 - r.player.x).hypot((i / w) as f64 - r.player.y),
                hazards: if current {
                    Some(crate::hazard_memory::observed_hazards_with_context(
                        f,
                        i,
                        r.tick,
                        hazard_context,
                    ))
                } else if remembered {
                    f.remembered_hazards
                        .as_ref()
                        .and_then(|m| m.get(&i.to_string()))
                        .cloned()
                } else {
                    None
                },
            }
        })
        .collect();
    drop(cells_span);
    let source_span=crate::timing::span("vision.observe_source_collection");
    let origin_known = |plane: usize, x: f64, y: f64| {
        let p = Point { x: x as i32, y: y as i32 };
        bounds(f, p) && {
            let i = p.y as usize * w + p.x as usize;
            cells[i].current && cell_planes[i] == plane
        }
    };
    let mut sources = vec![];
    for a in std::iter::once(&r.player).chain(r.actors_at(r.depth)) {
        if crate::dreamwalking::is_body(a) || a.dead.value_or(false) {
            continue;
        }
        let dream = crate::dreamwalking::enabled(r) && crate::dreamwalking::dream_actor(r, a);
        let Some(plane) = floors.iter().position(|(_, d)| *d == dream) else { continue };
        if emits_shine(r, a) {
            sources.push((plane, ObservedLightSource {
                origin_known: a.id == r.player.id || (origin_known(plane, a.x, a.y) && visible_actor(r, a)),
                emission_group: 0,
                x: a.x,
                y: a.y,
                kind: "shine".into(),
                color_role: "magic".into(),
                power: 100.0,
                falloff: 10.0,
                actor_id: Some(a.id),
                cells: vec![],
                strengths: vec![],
            }));
        }
        if a.hp > 0.0 && has(a, "enemy:beast:darkness-aura") {
            sources.push((plane, ObservedLightSource {
                origin_known: a.id == r.player.id || (origin_known(plane, a.x, a.y) && visible_actor(r, a)),
                emission_group: 0,
                x: a.x,
                y: a.y,
                kind: "darkness-aura".into(),
                color_role: "darkness".into(),
                power: 100.0,
                falloff: 10.0,
                actor_id: Some(a.id),
                cells: vec![],
                strengths: vec![],
            }));
        }
    }
    for (plane, &(f, _)) in floors.iter().enumerate() {
        if let Some(ss) = f.light_sources.as_ref() {
            for s in ss {
                if !s.power.is_finite() || !s.falloff.is_finite() || s.falloff <= 0.0 {
                    continue;
                }
                let kind = s.kind.value_or("fixture".into());
                let color = match kind.as_str() {
                    "torch" => "torch",
                    "brazier" => "fire",
                    "beacon" => "magic",
                    _ => "neutral",
                };
                sources.push((plane, ObservedLightSource {
                    origin_known: origin_known(plane, s.x, s.y),
                    emission_group: 0,
                    x: s.x,
                    y: s.y,
                    kind,
                    color_role: color.into(),
                    power: s.power,
                    falloff: 10.0,
                    actor_id: None,
                    cells: vec![],
                    strengths: vec![],
                }));
            }
        }
        if let Some(e) = f.environment.as_ref() {
            for (i, fire) in e.fire.iter().enumerate() {
                if *fire > 0.0 {
                    sources.push((plane, ObservedLightSource {
                        origin_known: origin_known(plane, (i % w) as f64, (i / w) as f64),
                        emission_group: 0,
                        x: (i % w) as f64,
                        y: (i / w) as f64,
                        kind: "fire".into(),
                        color_role: "fire".into(),
                        power: 80.0,
                        falloff: 10.0,
                        actor_id: None,
                        cells: vec![],
                        strengths: vec![],
                    }));
                }
            }
        }
    }
    drop(source_span);
    let _footprint_span=crate::timing::span("vision.observe_source_footprints");
    crate::timing::count(crate::timing::Count::LightSources,sources.len() as u64);
    let (mut candidates,mut rays,mut entries)=(0u64,0u64,0u64);
    let mut groups = std::collections::HashMap::new();
    for (plane, s) in &mut sources {
        let f = floors[*plane].0;
        let next = groups.len();
        s.emission_group = *groups.entry((*plane, s.x.to_bits(), s.y.to_bits())).or_insert(next);
        let origin = Point {
            x: s.x as i32,
            y: s.y as i32,
        };
        let radius = ((s.power / 10.0).ceil() as i32 - 1).min(w as i32);
        for y in (origin.y - radius).max(0)..=(origin.y + radius).min(f.height() as i32 - 1) {
            for x in (origin.x - radius).max(0)..=(origin.x + radius).min(w as i32 - 1) {
                let i = y as usize * w + x as usize;
                candidates+=1;
                if !cells[i].current || cell_planes[i] != *plane {
                    continue;
                }
                let d = if s.kind == "darkness-aura" {
                    (origin.x - x).abs() as f64 + (origin.y - y).abs() as f64
                } else {
                    distance(origin, Point { x, y })
                };
                let amount = (s.power - 10.0 * d).min(100.0);
                if amount > 0. {rays+=1;}
                if amount > 0.0 && floor_ray(f, origin, Point { x, y }) {
                    entries+=1;
                    s.cells.push(i);
                    s.strengths.push(amount);
                }
            }
        }
    }
    crate::timing::count(crate::timing::Count::LightCandidates,candidates);
    crate::timing::count(crate::timing::Count::LightRayTests,rays);
    crate::timing::count(crate::timing::Count::LightFootprintEntries,entries);
    sources.retain(|(_, s)| !s.cells.is_empty());
    for (_, s) in &mut sources {
        if !s.origin_known {
            s.x = -1.0;
            s.y = -1.0;
            s.actor_id = None;
        }
    }
    // Potion darkness is an authored cell mask, not a radial emitter. Append
    // after optical footprints so walls cannot reshape it. O(D) work/output,
    // D <= V per plane; only currently observed cells in their chosen plane pass.
    if crate::revision50::enabled(r) {
        for (plane, &(floor, _)) in floors.iter().enumerate() {
            let mut source = ObservedLightSource {
                origin_known: false, emission_group: 0, x: -1., y: -1.,
                kind: if crate::revision50_potions::absolute_darkness(r) {"potion-darkness-override"} else {"potion-darkness"}.into(), color_role: "darkness".into(),
                power: 50., falloff: 0., actor_id: None,
                cells: Vec::new(), strengths: Vec::new(),
            };
            for darkness in floor.revision50_darkness.as_ref().into_iter().flatten() {
                if darkness.expires_at > r.tick
                    && cells.get(darkness.cell).is_some_and(|c| c.current)
                    && cell_planes[darkness.cell] == plane
                {
                    source.cells.push(darkness.cell);
                    source.strengths.push(darkness.amount);
                }
            }
            if !source.cells.is_empty() {
                crate::timing::count(crate::timing::Count::LightSources, 1);
                crate::timing::count(crate::timing::Count::LightFootprintEntries, source.cells.len() as u64);
                sources.push((plane, source));
            }
        }
    }
    Some(TerrainObservation {
        all_seeing,
        width: w,
        height: f.height(),
        cells,
        light_sources: sources.into_iter().map(|(_, s)| s).collect(),
    })
}

#[cfg(test)]
mod native_plane_tests {
    use super::*;
    use crate::{dreamwalking, Field, FloorDoorsEntry, FloorLightSourcesEntry, GasVent};
    use std::sync::Arc;

    const W: usize = 9;
    const H: usize = 9;
    const AT: usize = 2 * W + 7;

    fn split_room(player_dream: bool) -> Run {
        let mut r = Run { depth: 1.0, simulation_revision: 34.0.into(), ..Default::default() };
        r.player.id = 1.0;
        r.player.hp = 100.0;
        r.player.x = 1.0;
        r.player.y = 2.0;
        r.player.abilities.push("etill-half-here".into());
        r.player.active.push("etill-half-here".into());
        r.player.conditions.insert("halfHereId".into(), 2.0);
        let mut clone = r.player.clone();
        clone.id = 2.0;
        clone.x = 6.0;
        clone.conditions.clear();
        clone.conditions.insert("halfHereClone".into(), 1.0);
        clone.conditions.insert("halfHereOwner".into(), r.player.id);
        clone.conditions.insert("phantasmiiPlane".into(), if player_dream { 0.0 } else { 1.0 });
        r.actors_at_mut(1.0).push(clone);
        let mut floor = Floor { depth: 1.0, width: (W as f64).into(), height: (H as f64).into(),
            tiles: vec!["floor".into(); W * H], lights: vec![-20.0; W * H],
            seen: vec![false; W * H], substrates: vec!["stone".into(); W * H].into(),
            ..Default::default() };
        for y in 0..H {
            for x in 0..W {
                if x == 0 || x == 3 || x == W - 1 || y == 0 || y == H - 1 {
                    floor.tiles[y * W + x] = "wall".into();
                }
            }
        }
        floor.tiles[AT] = "water".into();
        let mut slime = vec![0.0; W * H]; slime[AT] = 1.0; floor.slime = slime.into();
        floor.doors = [(AT.to_string(), FloorDoorsEntry { open: false, ..Default::default() })]
            .into_iter().collect::<indexmap::IndexMap<_, _>>().into();
        floor.light_sources = vec![FloorLightSourcesEntry { x: 7.0, y: 2.0, power: 80.0,
            falloff: 10.0, kind: "torch".to_string().into() }].into();
        floor.vents = vec![GasVent { at: AT as f64, gas: "poison".into(),
            phase: "active".into(), countdown: 3.0, ..Default::default() }].into();
        r.floors.push(Arc::new(floor));
        r.dreamwalking = dreamwalking::State { active: player_dream, depth: 1.0, ..Default::default() }.into();
        dreamwalking::ensure_layer(&mut r);
        let dream = crate::mutation_journal::make_mut(&mut r.dreamwalking.as_mut().unwrap().floors[0]);
        dream.tiles[AT] = "door".into();
        dream.substrates.as_mut().unwrap()[AT] = "wood".into();
        dream.lights.fill(20.0);
        dream.doors.as_mut().unwrap().get_mut(&AT.to_string()).unwrap().open = true;
        dream.slime = Field::Missing;
        let mut mud = vec![0.0; W * H]; mud[AT] = 1.0; dream.mud = mud.into();
        dream.light_sources.as_mut().unwrap()[0].kind = "beacon".to_string().into();
        dream.vents = vec![GasVent { at: AT as f64, gas: "confusion".into(),
            phase: "active".into(), countdown: 9.0, ..Default::default() }].into();
        r
    }

    #[test]
    fn potion_darkness_projection_is_current_plane_only_and_read_only() {
        for revision in [51.,52.] {for player_dream in [false, true] {
            let mut r = split_room(player_dream);
            r.simulation_revision = revision.into();
            r.player.stats.perception=100.;
            let kind=if revision>=52.{"potion-darkness-override"}else{"potion-darkness"};
            let expiry = r.tick + 20.;
            crate::mutation_journal::make_mut(&mut r.floors[0]).revision50_darkness = vec![crate::revision50_potions::Darkness { cell: AT, amount: 25., expires_at: expiry }].into();
            crate::mutation_journal::make_mut(&mut r.dreamwalking.as_mut().unwrap().floors[0]).revision50_darkness = vec![crate::revision50_potions::Darkness { cell: AT, amount: 50., expires_at: expiry }].into();
            for own_plane in [false, true] {
                if own_plane { r.player.x = 6.; }
                let frozen = serde_json::to_vec(&r).unwrap();
                let o = observe_terrain(&r).unwrap();
                let sources: Vec<_> = o.light_sources.iter().filter(|s| s.kind == kind).collect();
                assert_eq!(sources.len(), 1);
                assert_eq!(sources[0].cells, vec![AT]);
                let dream = if own_plane { player_dream } else { !player_dream };
                assert_eq!(sources[0].strengths, vec![if dream { 50. } else { 25. }]);
                if revision>=52. {assert_eq!(o.cells[AT].illumination,Some(-100.));}
                assert!(!sources[0].origin_known && sources[0].actor_id.is_none());
                assert_eq!((sources[0].x, sources[0].y), (-1., -1.));
                assert_eq!(serde_json::to_vec(&r).unwrap(), frozen);
            }
            r.player.x = 1.; r.player.active.clear();
            assert!(!observe_terrain(&r).unwrap().light_sources.iter().any(|s| s.kind == kind), "hidden source mask must not reveal unseen cells");
            r.player.x = 6.; r.tick = expiry;
            assert!(!observe_terrain(&r).unwrap().light_sources.iter().any(|s| s.kind == kind), "expired masks never reach rendering even before cleanup");
        }}
    }

    fn assert_native_cell(o: &TerrainObservation, dream: bool) {
        let c = &o.cells[AT];
        assert!(c.current && c.direct);
        assert_eq!(c.tile.as_deref(), Some(if dream { "door" } else { "water" }));
        assert_eq!(c.substrate.as_deref(), Some(if dream { "wood" } else { "stone" }));
        assert_eq!(c.door_open, Some(dream));
        assert_eq!(c.ambient, Some(if dream { 20.0 } else { -20.0 }));
        let hazards = c.hazards.as_ref().unwrap();
        assert_eq!(hazards.mud.value_or(false), dream);
        assert_eq!(hazards.slime.value_or(false), !dream);
        assert_eq!(hazards.vent.as_ref().unwrap().gas, if dream { "confusion" } else { "poison" });
        assert_eq!(hazards.light_source.as_ref().unwrap(), if dream { "beacon" } else { "torch" });
        assert!(o.light_sources.iter().any(|s| s.cells.contains(&AT)));
        assert!(o.light_sources.iter().filter(|s| s.cells.contains(&AT))
            .all(|s| s.kind == if dream { "beacon" } else { "torch" }));
    }

    #[test]
    fn opposite_half_here_observes_native_geometry_hazards_and_lights_without_mutation() {
        for player_dream in [false, true] {
            let r = split_room(player_dream);
            let before = r.clone();
            assert_native_cell(&observe_terrain(&r).unwrap(), !player_dream);
            let detached = dreamwalking::observation_run(&r);
            assert_native_cell(&observe_terrain(&detached).unwrap(), !player_dream);
            assert_eq!(r, before);
        }
    }

    #[test]
    fn player_plane_wins_when_both_half_here_views_see_the_tile() {
        for player_dream in [false, true] {
            let mut r = split_room(player_dream);
            r.player.x = 6.0;
            assert_native_cell(&observe_terrain(&r).unwrap(), player_dream);
        }
    }

    #[test]
    fn hidden_other_plane_cells_do_not_expose_current_terrain_hazards_or_sources() {
        let mut r = split_room(false);
        r.player.active.clear();
        let o = observe_terrain(&r).unwrap();
        let c = &o.cells[AT];
        assert!(!c.current && !c.remembered);
        assert!(c.tile.is_none() && c.substrate.is_none() && c.hazards.is_none());
        assert!(c.ambient.is_none() && c.illumination.is_none());
        assert!(o.light_sources.iter().all(|s| !s.cells.contains(&AT)));
    }

    #[test]
    fn native_memory_mask_excludes_opposite_clone_only_visibility() {
        let mut r = split_room(false);
        assert!(terrain_visibility(&r)[AT]);
        assert!(!native_terrain_visibility(&r)[AT]);
        dreamwalking::with_plane(&mut r, true, |r| {
            assert!(native_terrain_visibility(r)[AT]);
            assert!(!native_terrain_visibility(r)[2 * W + 1]);
        });
    }

    #[test]
    fn native_memory_all_seeing_and_senses_need_a_native_endpoint_not_a_sleeping_body() {
        let mut r = split_room(true);
        r.player.conditions.insert("allSeeingReveal".into(), 1.0);
        r.player.conditions.insert("allSeeingDepth".into(), 1.0);
        r.player.abilities.push("ursa-dream-see".into());
        r.player.conditions.insert("telepathic".into(), 1.0);
        r.player.active.retain(|a| a != "etill-half-here");
        let mut body = r.player.clone();
        body.id = 3.0; body.conditions.clear(); body.conditions.insert("phantasmiiBody".into(), 1.0);
        let mut sleeper = Actor { id: 4.0, x: 7.0, y: 2.0, state: "sleeping".into(), hp: 20.0,
            ..Default::default() };
        sleeper.conditions.insert("stealth".into(), 0.0);
        r.actors_at_mut(1.0).extend([body, sleeper]);
        assert!(terrain_visibility(&r).iter().all(|&v| v));
        assert!(native_terrain_visibility(&r).iter().all(|&v| !v));
        dreamwalking::with_plane(&mut r, true, |r| {
            assert!(native_terrain_visibility(r).iter().all(|&v| v));
        });
        r.player.active.push("etill-half-here".into());
        assert!(native_terrain_visibility(&r).iter().all(|&v| v), "native Half Here shares all-seeing");
    }
}

#[cfg(test)]
mod lighting_cache_equivalence_tests {
    use super::*;
    use crate::{FloorDoorsEntry, FloorLightSourcesEntry};
    use std::sync::Arc;

    /// O(V) revealed output; proves no dependency capture or light-field build.
    #[test]
    fn all_seeing_masks_preserve_no_lighting_shortcut() {
        let mut r = room();
        r.player.conditions.insert("allSeeingReveal".into(), 1.);
        r.player.conditions.insert("allSeeingDepth".into(), r.depth);
        assert!(r.lighting_cache.is_empty_for_test());
        assert_eq!(terrain_visibility(&r), vec![true; 121]);
        assert_eq!(native_terrain_visibility(&r), vec![true; 121]);
        assert_eq!(terrain_visibility_chance(&r), vec![100.; 121]);
        assert!(r.lighting_cache.is_empty_for_test(), "revealed masks must not build fields");
        r.player.conditions.shift_remove("allSeeingReveal");
        let _ = terrain_visibility(&r);
        assert!(!r.lighting_cache.is_empty_for_test());
    }

    /// O(V) fixed synthetic floor and O(1) actors; no RNG or runtime mutation.
    fn room() -> Run {
        let mut r = Run { depth: 1., simulation_revision: 39.0.into(), ..Default::default() };
        r.player = Actor { id: 1., x: 2., y: 2., hp: 100., ..Default::default() };
        let mut f = Floor { depth: 1., tiles: vec!["floor".into(); 121],
            lights: (0..121).map(|i| -55.5 + (i % 7) as f64).collect(),
            seen: vec![false; 121], ..Default::default() };
        for y in 0..11 { f.tiles[y * 11 + 6] = "wall".into(); }
        f.tiles[5 * 11 + 6] = "floor".into();
        f.doors = [(61usize.to_string(), FloorDoorsEntry { open: false, ..Default::default() })]
            .into_iter().collect::<indexmap::IndexMap<_, _>>().into();
        f.light_sources = vec![FloorLightSourcesEntry { x: 4., y: 4., power: 110.25,
            falloff: 10., kind: "torch".to_string().into() }].into();
        let mut e = crate::EnvironmentalState::default();
        e.fire = vec![0.; 121]; e.fire[3 * 11 + 3] = 1.;
        f.environment = e.into();
        r.floors.push(Arc::new(f));
        let shine = Actor { id: 2., x: 8.25, y: 8.5, hp: 100.,
            abilities: vec!["anavani-shine".into()], active: vec!["anavani-shine".into()],
            ..Default::default() };
        let dark = Actor { id: 3., x: 5.125, y: 4.75, hp: 1.,
            abilities: vec!["enemy:beast:darkness-aura".into()], ..Default::default() };
        let mut dark2 = dark.clone(); dark2.id = 4.; dark2.x = 9.875;
        r.actors_at_mut(1.).extend([shine, dark, dark2]);
        r
    }

    /// O(V * original-light-cost) reference comparison, each cell bit-for-bit.
    fn assert_fields(r: &Run) {
        for dream in [false, true] {
            let f = crate::dreamwalking::floor_in_plane(r, dream).or_else(|| r.floor()).unwrap();
            let field = r.lighting_cache.field(r, dream).unwrap();
            for i in 0..f.tiles.len() {
                let p = Point { x: (i % f.width()) as i32, y: (i / f.width()) as i32 };
                assert_eq!(field.at(p).to_bits(), illumination_in_plane(r, p, None, dream).to_bits(),
                    "plane={dream}, cell={i}");
            }
        }
    }

    /// O(edits*V*reference-light-cost), 14 fixed direct same-tick mutations. Proves
    /// invalidation does not depend on commands/tick/explicit setters being used.
    #[test]
    fn direct_mutations_and_planes_match_original_lighting_exactly() {
        let mut r = room(); assert_fields(&r);
        r.floor_mut().unwrap().doors.as_mut().unwrap().get_mut("61").unwrap().open = true;
        assert_fields(&r);
        r.floor_mut().unwrap().tiles[49] = "tree".into(); assert_fields(&r);
        r.floor_mut().unwrap().lights[50] = 99.875; assert_fields(&r);
        r.floor_mut().unwrap().environment.as_mut().unwrap().fire[5] = 10.; assert_fields(&r);
        r.floor_mut().unwrap().environment.as_mut().unwrap().fire[36] = 0.; assert_fields(&r);
        r.floor_mut().unwrap().light_sources.as_mut().unwrap()[0].power = 33.25; assert_fields(&r);
        r.floor_mut().unwrap().light_sources.as_mut().unwrap()[0].falloff = 0.; assert_fields(&r);
        r.actors_at_mut(1.)[0].x = 4.; assert_fields(&r);
        r.actors_at_mut(1.)[0].dead = true.into(); assert_fields(&r);
        r.actors_at_mut(1.)[1].hp = 0.; assert_fields(&r);
        r.actors_at_mut(1.)[1].hp = 100.; r.actors_at_mut(1.)[1].x = 2.75; assert_fields(&r);
        r.actors_at_mut(1.).swap(1, 2); assert_fields(&r);
        crate::dreamwalking::ensure_layer(&mut r);
        let f = crate::mutation_journal::make_mut(&mut r.dreamwalking.as_mut().unwrap().floors[0]);
        f.lights.fill(71.5); f.tiles[49] = "floor".into(); assert_fields(&r);
        r.actors_at_mut(1.)[2].conditions.insert("phantasmiiPlane".into(), 1.); assert_fields(&r);
    }

    /// O(V+A*B) key comparison; clean result must reuse the exact allocation.
    #[test]
    fn cache_reuses_clean_field_and_ignores_non_light_changes() {
        let mut r = room();
        let a = r.lighting_cache.field(&r, false).unwrap();
        r.tick += 1.; r.player.hp -= 1.; r.player.x += 1.;
        r.floor_mut().unwrap().seen[0] = true;
        let b = r.lighting_cache.field(&r, false).unwrap();
        assert!(Arc::ptr_eq(&a, &b));
        r.floor_mut().unwrap().lights[0] += 1.;
        let c = r.lighting_cache.field(&r, false).unwrap();
        assert!(!Arc::ptr_eq(&b, &c));
        assert_fields(&r);
    }

    /// O(V*reference-light-cost + serialized bytes); cache never enters state.
    #[test]
    fn clone_restore_and_sparse_exclusion_remain_exact() {
        let r = room(); let before = serde_json::to_string(&r).unwrap(); assert_fields(&r);
        assert_eq!(before, serde_json::to_string(&r).unwrap());
        let original_field = r.lighting_cache.field(&r, false).unwrap();
        let clone = r.clone(); assert_fields(&clone); assert_eq!(r, clone);
        assert!(!Arc::ptr_eq(&original_field, &clone.lighting_cache.field(&clone, false).unwrap()));
        let loaded: Run = serde_json::from_str(&before).unwrap(); assert_fields(&loaded);
        assert!(!Arc::ptr_eq(&original_field, &loaded.lighting_cache.field(&loaded, false).unwrap()));
        let a = &r.actors_at(1.)[0];
        // Tile field must include active Shine while the original actor-specific
        // query excludes that target's own active source. That path is untouched.
        assert_eq!(light(&r, a).to_bits(), illumination_in_plane(&r, point(a), Some(a.id), false).to_bits());
    }

    /// O(generation + V*visibility-cost), fixed generated scenario. Whole mask
    /// matches original per-cell `visible`, not merely selected light samples.
    #[test]
    fn generated_visibility_mask_matches_reference_queries() {
        let r = crate::startup::create_run("02-04-09:timing-r39-1", "Blessed", false).unwrap();
        let f = r.floor().unwrap(); let w = f.width();
        let expected: Vec<_> = (0..f.tiles.len()).map(|i| visible(&r, i % w, i / w)).collect();
        assert_eq!(terrain_visibility(&r), expected);
        assert_eq!(native_terrain_visibility(&r), expected);
        assert_fields(&r);
    }
    /// O(32*V*reference-light-cost), fixed non-gameplay arithmetic fixture sweep.
    #[test]
    fn varied_sources_and_opaque_edits_match_reference() {
        let mut r = room();
        for k in 0..32usize {
            let f = r.floor_mut().unwrap();
            f.tiles[(k * 13) % 121] = if k % 2 == 0 { "wall" } else { "floor" }.into();
            f.environment.as_mut().unwrap().fire[(k * 7) % 121] = if k % 3 == 0 { 1. } else { 0. };
            let s = &mut f.light_sources.as_mut().unwrap()[0];
            s.x = ((k * 3) % 11) as f64 + 0.375;
            s.y = ((k * 5) % 11) as f64;
            s.power = (k * 19 % 180) as f64 + 0.0625;
            r.actors_at_mut(1.)[0].x = (k % 11) as f64;
            r.actors_at_mut(1.)[1].y = ((k * 3) % 11) as f64 + 0.125;
            assert_fields(&r);
        }
    }

}

#[cfg(test)]
#[path="scalar_light_culling_tests.rs"]
mod scalar_light_culling_tests;
