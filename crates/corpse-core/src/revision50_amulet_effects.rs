//! Authored R50 Apply effects. Target admission is pure; launch/accounting lives
//! inside the engine command transaction. No cooldowns or hidden-target probes.
use crate::{Actor, Command, Item, Run, engine::{self, EngineError}, movement::Point};
use crate::revision50_amulets::{AmuletSpec, ActivationPlan};

pub const POISON_DAMAGE: &str = "r50PoisonDamage";
pub const ACID_TURNS: &str = "r50AcidTurns";
pub const ACID_PENALTY: &str = "r50AcidPenalty";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect { Lightning, Firebolt, Poison, Iceblast, Acid, Blinking }
pub fn effect(item: &Item) -> Option<Effect> {
    if item.kind != "amulet" { return None; }
    Some(match item.effect.as_ref()?.as_str() {
        "lightning" => Effect::Lightning, "firebolt" => Effect::Firebolt,
        "poison" => Effect::Poison, "iceblast" => Effect::Iceblast,
        "acid" => Effect::Acid, "blinking" => Effect::Blinking, _ => return None,
    })
}
pub fn is_amulet(item: &Item) -> bool { effect(item).is_some() || crate::revision50_control_amulets::effect(item).is_some() }
pub fn is_projectile(item: &Item) -> bool {
    matches!(effect(item), Some(Effect::Lightning | Effect::Firebolt | Effect::Poison | Effect::Acid))
}
pub fn display_name(item: &Item) -> Option<&'static str> {
    let Some(kind)=effect(item) else {return crate::revision50_control_amulets::display_name(item)};
    Some(match kind {
        Effect::Lightning => "Amulet of Lightning", Effect::Firebolt => "Amulet of Firebolt",
        Effect::Poison => "Amulet of Poison", Effect::Iceblast => "Amulet of Iceblast",
        Effect::Acid => "Amulet of Acid", Effect::Blinking => "Amulet of Blinking",
    })
}
pub fn description(item:&Item)->Option<&'static str> {
    Some(match effect(item) {
        Some(Effect::Lightning)=>"Fires a piercing bolt for 50 lightning damage.",
        Some(Effect::Firebolt)=>"Fires a bolt for 80 fire damage, igniting creatures and flammable terrain.",
        Some(Effect::Poison)=>"Fires a projectile that poisons its target for 6 damage per turn for 20 turns.",
        Some(Effect::Iceblast)=>"Deals 30 cold damage and slows creatures for 3 turns in a 3×3 area, leaving ice.",
        Some(Effect::Acid)=>"Fires a projectile that deals 40 damage and reduces the target’s DR by 10 for 6 turns.",
        Some(Effect::Blinking)=>"Blink along a 10 tile line.",
        None=>return crate::revision50_control_amulets::description(item),
    })
}
/// Essential targeting information supplements the short effect description.
pub fn details(item:&Item)->Option<&'static str> {
    Some(match effect(item) {
        Some(Effect::Lightning | Effect::Firebolt | Effect::Poison)=>"Unlimited range.",
        Some(Effect::Iceblast)=>"Targets a visible area.",
        Some(Effect::Acid)=>"Range: 8 tiles.",
        Some(Effect::Blinking)=>return None,
        None=>return crate::revision50_control_amulets::details(item),
    })
}
pub fn details_for_run(r:&Run,item:&Item)->Option<&'static str> {
    if r.simulation_revision.value_or(29.)>=52. {match effect(item) {
        Some(Effect::Iceblast)=>return Some("Detonates on the first creature in the aimed line, or at the last reachable square before terrain. Targets a visible square."),
        Some(Effect::Blinking)=>return Some("Aim within 10 tiles. Blocking creatures and terrain shorten the line; you land on its last accessible empty square."),
        _=>{},
    }}
    details(item)
}

pub fn item_spec(item: &Item) -> Option<AmuletSpec> {
    item_spec_for_revision(item,50.)
}
pub fn item_spec_for_revision(item: &Item, revision: f64) -> Option<AmuletSpec> {
    let capacity=crate::revision52_reserves::capacity(revision);
    if let Some(kind)=crate::revision50_control_amulets::effect(item) {
        return Some(AmuletSpec {capacity,activation_cost:crate::revision50_control_amulets::activation_cost(kind)});
    }
    let activation_cost = match effect(item)? {
        Effect::Lightning => 25., Effect::Firebolt => 20., Effect::Poison => 15.,
        Effect::Iceblast => 30., Effect::Acid | Effect::Blinking => 20.,
    };
    Some(AmuletSpec { capacity, activation_cost })
}
pub fn condition_key(key: &str) -> bool { matches!(key, POISON_DAMAGE | ACID_TURNS | ACID_PENALTY) }
pub fn validate_statuses(r:&Run)->Result<(),String> {
    let valid=|a:&Actor| a.conditions.get(POISON_DAMAGE).is_none_or(|v|crate::revision52_amulet_tiers::valid_condition(POISON_DAMAGE,*v,r.simulation_revision.value_or(29.)))
        && a.conditions.get(ACID_TURNS).is_none_or(|v|v.is_finite()&&*v>=0.&&*v<=6.&&v.fract()==0.)
        && a.conditions.get(ACID_PENALTY).is_none_or(|v|*v==5.||*v==10.);
    if !valid(&r.player)||!r.actors.values().all(|actors|actors.iter().all(valid)) {
        return Err("Invalid Revision 50 poison or corrosion state".into());
    }
    Ok(())
}
pub fn make_amulet(kind: Effect, id: f64) -> Option<Item> {
    make_amulet_for_revision(kind,id,50.)
}
pub fn make_amulet_for_revision(kind: Effect, id: f64, revision: f64) -> Option<Item> {
    let name = match kind { Effect::Lightning => "lightning", Effect::Firebolt => "firebolt",
        Effect::Poison => "poison", Effect::Iceblast => "iceblast", Effect::Acid => "acid", Effect::Blinking => "blinking" };
    let mut item = Item { id, kind: "amulet".into(), effect: name.to_string().into(),
        glyph: "\"".into(), identified: true.into(), ..Default::default() };
    item.name = display_name(&item)?.into();
    crate::revision50_amulets::initialize_item_for_revision(&mut item,revision).ok()?;
    Some(item)
}
fn coordinates(r: &Run, x: f64, y: f64) -> Option<usize> {
    crate::portals::cell(crate::dreamwalking::floor_for(r, &r.player)?, x, y)
}
fn ray_pass(f: &crate::Floor, p: Point) -> bool {
    crate::portals::cell(f, p.x as f64, p.y as f64).is_some_and(|at|
        !matches!(f.tiles[at].as_str(), "wall" | "tree") && !crate::environment::blocks_sight(f, at))
}
#[cfg(test)]
fn target_valid(r: &Run, kind: Effect, x: f64, y: f64) -> bool {target_valid_with_range(r,kind,x,y,10.)}
fn target_valid_with_range(r: &Run, kind: Effect, x: f64, y: f64, blink_range:f64) -> bool {
    let Some(at) = coordinates(r, x, y) else { return false; };
    if kind != Effect::Iceblast && crate::portals::same_position(r, &r.player, x, y) { return false; }
    if matches!(kind, Effect::Iceblast | Effect::Blinking) {
        if !crate::vision::visible(r, x as usize, y as usize) { return false; }
        let f = crate::dreamwalking::floor_for(r, &r.player).unwrap();
        if !ray_pass(f, crate::portals::point(f, at)) { return false; }
    }
    if kind == Effect::Blinking {
        // R52 selects a direction/endpoint, not a hidden-occupancy probe. Resolve
        // the last legal landing only after payment, just like a launched bolt.
        if r.simulation_revision.value_or(29.)>=52. {
            return (r.player.x-x).hypot(r.player.y-y)<=blink_range;
        }
        let f=crate::dreamwalking::floor_for(r,&r.player).unwrap();
        return (r.player.x - x).hypot(r.player.y - y) <= 10.
            && engine::actor_passable(r, &r.player, x, y)
            && crate::portals::occupied_at(r, &r.player, x, y, Some(r.player.id)).is_none()
            && (r.simulation_revision.value_or(29.)<51. || {
                let landing=crate::projectiles::trace(r,x,y);
                landing.actor_id.is_none() && crate::portals::same_cell(f,landing.x,landing.y,x,y)
            });
    }
    true
}
/// Target/resource admission never inspects actors along a ray. Empty directions
/// and hidden blockers are legal shots, preventing target selection from scouting.
pub fn admission(r: &Run, c: &Command) -> Option<Result<ActivationPlan, String>> {
    if !crate::revision50::enabled(r) { return None; }
    let (id, aim) = match c { Command::UseAt { id, x, y } => (*id, Some((*x,*y))),
        Command::Use { id } => (*id, None), _ => return None };
    let item = r.inventory.iter().find(|i| i.id == id)?;
    if !is_amulet(item) {return None}
    if crate::revision52_amulet_tiers::retired(r,item) {return Some(Err(crate::revision52_amulet_tiers::RETIRED_MESSAGE.into()));}
    let Some((x,y)) = aim else { return Some(Err("Choose a target for this amulet.".into())); };
    let Some(profile)=crate::revision52_amulet_tiers::profile(r,item) else {return Some(Err("Invalid amulet tier.".into()));};
    let valid=if let Some(kind)=effect(item) {target_valid_with_range(r,kind,x,y,profile.range.unwrap_or(10.))
        && (r.simulation_revision.value_or(29.)<52. || kind!=Effect::Blinking
            || !known_blink_stays_put(r,item,x,y))} else {
        crate::revision50_control_amulets::target_valid_with_profile(r,crate::revision50_control_amulets::effect(item).unwrap(),x,y,profile)
    };
    if !valid { return Some(Err("Choose a valid target for this amulet.".into())); }
    Some(crate::revision50_amulets::plan_item_activation(r, id, item_spec(item).as_ref())
        .map_err(admission_message))
}

#[derive(Debug)]
struct Ray { points: Vec<Point>, impact: Option<Point>, targets: Vec<f64>, rng: u32 }
/// Uses the existing spatial ray plan and one traversal per linked group. A
/// 2V-step endpoint extends the selected slope to the floor boundary without a
/// gameplay range cap. The shared traversal bounds cycles; only committed forks
/// consume serialized RNG. O(ray-plan + V*A*P + A²), O(V+A) extra memory,
/// plus existing plane/owner queries. Alias lookup scans bounded link groups.
fn trace(r: &Run, kind: Effect, x: f64, y: f64) -> Ray {
    let f = crate::dreamwalking::floor_for(r, &r.player).unwrap();
    let from = Point { x:r.player.x as i32, y:r.player.y as i32 };
    let aim = Point { x:x as i32, y:y as i32 };
    let pass=|p|crate::projectiles::terrain_pass(r.simulation_revision.value_or(29.),f,p);
    let plan = crate::portals::ray_plan(f, from, aim, pass);
    let mut state = crate::rng::to_u32(r.rng);
    let mut origin = plan.origin;
    if plan.origin_fork.len() > 1 {
        let n = (crate::rng::next_u32(&mut state) as f64 / 4294967296. * plan.origin_fork.len() as f64).floor() as usize;
        origin = crate::portals::point(f, plan.origin_fork[n]);
    }
    let dx = plan.aim.x - plan.origin.x;
    let dy = plan.aim.y - plan.origin.y;
    let scale = if matches!(kind,Effect::Iceblast|Effect::Blinking) {1}
        else {(f.tiles.len().saturating_mul(2).max(1)) as i32};
    let endpoint = Point { x:origin.x + dx*scale, y:origin.y + dy*scale };
    let blocked = std::cell::Cell::new(None);
    let range_origin = std::cell::Cell::new(origin);
    let traveled = std::cell::Cell::new(0.0);
    let prior = std::cell::Cell::new(origin);
    let range_stop = std::cell::Cell::new(false);
    let ray = crate::portals::directed_ray_trace(f, origin, endpoint, |p| {
        if !pass(p) {
            // Alternative portal exits are checked before the launch lottery.
            // They are not impact cells unless the ray physically entered them.
            let last=prior.get();
            let alternative=crate::portals::cell(f,last.x as f64,last.y as f64)
                .zip(crate::portals::cell(f,p.x as f64,p.y as f64))
                .is_some_and(|(a,b)|a!=b&&crate::portals::exits(f,a).contains(&b));
            if !alternative && (last.x-p.x).abs().max((last.y-p.y).abs())<=1 {blocked.set(Some(p));}
            return false;
        }
        true
    }, |options| {
        let selected = if options.len()==1 { options[0] } else {
            options[(crate::rng::next_u32(&mut state) as f64/4294967296.*options.len() as f64).floor() as usize]
        };
        traveled.set(traveled.get()+crate::vision::distance(range_origin.get(),prior.get()));
        range_origin.set(crate::portals::point(f,selected));
        Some(selected)
    }, |p| {
        prior.set(p);
        if kind == Effect::Acid && traveled.get()+crate::vision::distance(range_origin.get(),p) > 8. {
            range_stop.set(true); return true;
        }
        kind != Effect::Lightning && crate::portals::occupied_at(r,&r.player,p.x as f64,p.y as f64,Some(r.player.id)).is_some()
    });
    let mut points = ray.points;
    if range_stop.get() { points.pop(); }
    let mut targets = Vec::new();
    for p in points.iter().skip(usize::from(origin == from)) {
        for a in std::iter::once(&r.player).chain(r.actors_at(r.depth)) {
            if a.id != r.player.id && !a.dead.value_or(false) && a.hp > 0.
                && crate::dreamwalking::same_plane(r,&r.player,a)
                && crate::portals::same_position(r,a,p.x as f64,p.y as f64)
                && !targets.contains(&a.id) { targets.push(a.id); }
        }
        if kind != Effect::Lightning && !targets.is_empty() { targets.truncate(1); break; }
    }
    Ray { points, impact: blocked.get(), targets, rng: state }
}
/// Blink keeps the physical ray, then backs up from its first occupant or an
/// inaccessible landing. O(V*(A*P+M+G)), no RNG beyond the committed ray's forks.
#[cfg(test)]
fn blink_landing(r:&Run,ray:&Ray)->Point {blink_landing_with_range(r,ray,10.)}
fn blink_landing_with_range(r:&Run,ray:&Ray,range:f64)->Point {
    ray.points.iter().rev().copied().find(|p|
        (p.x as f64-r.player.x).hypot(p.y as f64-r.player.y)<=range
        && engine::actor_passable(r,&r.player,p.x as f64,p.y as f64)
        && crate::portals::occupied_at(r,&r.player,p.x as f64,p.y as f64,Some(r.player.id)).is_none())
        .unwrap_or(Point{x:r.player.x as i32,y:r.player.y as i32})
}
#[cfg(test)]
fn poison(r: &mut Run, id: f64, source: f64) {poison_with_damage(r,id,source,6.)}
fn poison_with_damage(r: &mut Run, id: f64, source: f64,damage:f64) {
    if engine::actor(r,id).zip(engine::actor_any(r,source))
        .is_some_and(|(target,source)|crate::revision50::duel_blocks(r,source,target)) {return;}
    let duration = engine::actor(r,id).map_or(20., |a| crate::monster_statuses::status_duration(r,a,20.));
    if crate::monster_statuses::apply_status(r,id,"poisoned",20.,Some(source)) {
        let a = engine::actor_mut_trusted(r,id).unwrap();
        a.conditions.insert("poisoned".into(),duration);
        a.conditions.insert(POISON_DAMAGE.into(),damage);
        let a=engine::actor(r,id).unwrap();crate::events::cue(r,a,"A03","split-ring",crate::events::Stage::Special,&[a.into()]);
    } else if let Some(a)=engine::actor(r,id){crate::events::cue(r,a,"A03","rejected-poison",crate::events::Stage::Impact,&[a.into()]);}
}
#[cfg(test)]
fn acid(r: &mut Run, id: f64, source: &Actor) -> Result<(),EngineError> {acid_with_damage(r,id,source,40.)}
fn acid_with_damage(r: &mut Run, id: f64, source: &Actor,damage:f64) -> Result<(),EngineError> {
    let Some(a) = engine::actor(r,id) else { return Ok(()); };
    if crate::revision50::duel_blocks(r,source,a) || crate::revision50_combat::oblivious_blocks(r,a,Some(source)) || crate::revision50_nightmares::is_decoy(a) {return Ok(());}
    let duration = crate::monster_statuses::status_duration(r,a,6.);
    let penalty = 10. * crate::revision50_rings::stack_multiplier(r,a,crate::revision50_rings::CORROSION_RESISTANCE,0.5);
    engine::damage_direct_with_source_snapshot(r,id,damage,source,"acid",true)?;
    if let Some(a) = engine::actor_mut_trusted(r,id).filter(|a|!a.dead.value_or(false)&&a.hp>0.) {
        a.conditions.insert(ACID_TURNS.into(),duration);
        a.conditions.insert(ACID_PENALTY.into(),penalty);
        let a=engine::actor(r,id).unwrap();crate::events::cue(r,a,"A05","cracked-shield",crate::events::Stage::Special,&[a.into()]);
    }
    Ok(())
}
fn ignite_target(r: &mut Run, id: f64, source: &Actor) {
    let Some(a) = engine::actor(r,id) else {return};
    if a.dead.value_or(false) || a.hp<=0. || crate::revision50_nightmares::is_decoy(a)
        || crate::revision51_ichor_fire::immune(r.simulation_revision.value_or(29.), a)
        || crate::revision50_potions::fire_immune(a)
        || a.abilities.iter().any(|s|matches!(s.as_str(),"enemy:beast:fire-immunity"|"anavani-star-power"))
        || engine::condition(a,"underwater")>0.
        || crate::revision50::duel_blocks(r,source,a) || crate::revision50_combat::oblivious_blocks(r,a,Some(source)) {return;}
    let random = crate::rng::random(r);
    crate::corpses::apply_burning_run(r,id,r.simulation_revision.value_or(29.),random,Some(-2.),Some(1.),None);
}
fn fire_path(r: &mut Run, ray: &Ray) -> Result<(),EngineError> {
    let frozen = crate::environment::frozen_environment(r);
    let width = r.floor().unwrap().width();
    let mut adapter = crate::fire_adapter::FireAdapter::new(r,frozen).map_err(EngineError)?;
    for p in ray.points.iter().skip(1).chain(ray.impact.iter()) {
        if p.x >= 0 && p.y >= 0 && coordinates(adapter.run,p.x as f64,p.y as f64).is_some() {
            adapter.ignite_creature(p.y as usize*width+p.x as usize,-2.,1.);
        }
    }
    let bursts = adapter.resolve().map_err(EngineError)?;
    drop(adapter);
    crate::environment::resolve_bursts(r,bursts,None,false).map_err(EngineError)
}
fn ice_reaches(f:&crate::Floor,center:Point,p:Point)->bool {
    if (p.x-center.x).abs()>1 || (p.y-center.y).abs()>1 || !ray_pass(f,p) {return false;}
    p.x==center.x || p.y==center.y || ray_pass(f,Point{x:p.x,y:center.y}) || ray_pass(f,Point{x:center.x,y:p.y})
}
#[cfg(test)]
fn iceblast(r: &mut Run, x: f64, y: f64, source: &Actor) -> Result<(),EngineError> {iceblast_with_damage(r,x,y,source,30.)}
fn iceblast_with_damage(r: &mut Run, x: f64, y: f64, source: &Actor,damage:f64) -> Result<(),EngineError> {
    let f = r.floor().unwrap();
    let center = Point {x:x as i32,y:y as i32};
    let cells:Vec<_>=(-1..=1).flat_map(|dy|(-1..=1).map(move |dx|Point{x:center.x+dx,y:center.y+dy})).filter(|&p|ice_reaches(f,center,p)).filter_map(|p|crate::portals::cell(f,p.x as f64,p.y as f64)).collect();
    // R52 physical occupancy is alias-aware, just like delivery. A creature on
    // the remote mate which intercepts this ray must receive the local impact.
    // At most nine alias checks per actor; each actor receives one packet.
    let targets: Vec<_> = std::iter::once(&r.player).chain(r.actors_at(r.depth)).filter(|a|
        !a.dead.value_or(false)&&a.hp>0.&&crate::dreamwalking::same_plane(r,source,a)
        && if r.simulation_revision.value_or(29.)>=52. {cells.iter().any(|&at| {
            let p=crate::portals::point(f,at);crate::portals::same_position(r,a,p.x as f64,p.y as f64)
        })} else {ice_reaches(f,center,Point{x:a.x as i32,y:a.y as i32})})
        .map(|a|a.id).collect();
    crate::events::cue(r,source,"A04","crystal",crate::events::Stage::Special,&crate::animation_amulets::cell_points(r,&cells));
    crate::revision50_potions::freeze_cells(r,&cells);
    for id in targets {
        let Some(a) = engine::actor(r,id) else {continue};
        if crate::revision50::duel_blocks(r,source,a) || crate::revision50_rings::has_effect(r,a,crate::revision50_rings::ICE_IMMUNITY) {continue;}
        engine::damage_direct_with_source_snapshot(r,id,damage,source,"cold",true)?;
        if crate::monster_statuses::apply_status(r,id,"slowed",3.,Some(source.id)){if let Some(a)=engine::actor(r,id){crate::events::cue(r,a,"A04","snowflake",crate::events::Stage::Special,&[a.into()]);}}
    }
    Ok(())
}
/// Executed only after pure target/resource admission. Outer engine dispatch
/// rolls back effects, payment, RNG, logs and clocks together on any failure.
pub fn command(r: &mut Run, c: &Command) -> Result<Option<bool>,EngineError> {
    let Some(admitted) = admission(r,c) else {return Ok(None)};
    let plan = admitted.map_err(EngineError)?;
    let Command::UseAt{id,x,y}=*c else {unreachable!()};
    let item = r.inventory.iter().find(|i|i.id==id).unwrap();
    let kind = effect(item);
    let control=crate::revision50_control_amulets::effect(item);
    let name=crate::revision52_amulet_tiers::display_name(r,item).unwrap_or_else(||"amulet".into());
    let profile=crate::revision52_amulet_tiers::profile(r,item).ok_or_else(||EngineError("Invalid amulet tier".into()))?;
    let spec = item_spec(item);
    crate::revision50_amulets::commit_item_activation(r,id,&plan,spec.as_ref())
        .map_err(|e|EngineError(format!("Amulet activation changed: {e:?}")))?;
    let source = r.player.clone();
    if let Some(kind)=control {
        crate::revision50_control_amulets::execute_with_profile(r,kind,x,y,profile)?;
        engine::log(r,format!("You activate {name}."),"player");
        return Ok(Some(plan.consumes_action()));
    }
    let kind=kind.unwrap();
    match kind {
        Effect::Blinking => {
            let (x,y)=if r.simulation_revision.value_or(29.)>=52. {
                let ray=trace(r,kind,x,y);r.rng=ray.rng as f64;
                let landing=blink_landing_with_range(r,&ray,profile.range.unwrap());
                let end=ray.points.iter().rposition(|p|*p==landing).map_or(0,|i|i+1);
                crate::animation_amulets::route(r,&source,kind,&ray.points[..end],None);
                (landing.x as f64,landing.y as f64)
            } else {(x,y)};
            crate::events::cue(r,&source,"A06","fold",crate::events::Stage::Travel,&[(&source).into()]);
            let old = coordinates(r,r.player.x,r.player.y).unwrap();
            if r.simulation_revision.value_or(29.)<52. || r.player.x!=x || r.player.y!=y {
                r.player.x=x; r.player.y=y;
                crate::half_here::sync(r);
                crate::zillennia_ground::on_arrival(r,r.player.id);
                crate::environment::refresh_burning_from_fire(r,r.player.id).map_err(EngineError)?;
                engine::finish_actor_arrival(r,r.player.id,old,false,None)?;
            }
            crate::events::cue(r,&r.player,"A06","unfold",crate::events::Stage::Special,&[(&r.player).into()]);
        },
        Effect::Iceblast => {
            let (x,y)=if r.simulation_revision.value_or(29.)>=52. {
                let ray=trace(r,kind,x,y);r.rng=ray.rng as f64;
                let center=ray.points.last().copied().unwrap_or(Point{x:source.x as i32,y:source.y as i32});
                crate::animation_amulets::route(r,&source,kind,&ray.points,None);
                (center.x as f64,center.y as f64)
            } else {(x,y)};
            iceblast_with_damage(r,x,y,&source,profile.direct_damage)?;
        },
        Effect::Firebolt if crate::firebolt_travel::current(r) => {
            let flight=crate::firebolt_travel::launch(r,&source,x,y)?;
            crate::animation_amulets::route(r,&source,kind,&flight.points,flight.impact);
            if let Some(id)=flight.target {
                let before=engine::actor(r,id).unwrap().clone();
                let packets_before=crate::events::damage_count(id);
                engine::damage_direct_with_source_snapshot(r,id,profile.direct_damage,&source,"fire",true)?;
                ignite_target(r,id,&source);
                crate::animation_amulets::outcome(r,&before,kind,packets_before);
            }
            crate::environment::resolve_bursts(r,flight.bursts,None,false).map_err(EngineError)?;
        },
        _ => {
            let ray = trace(r,kind,x,y); r.rng=ray.rng as f64;
            crate::animation_amulets::route(r,&source,kind,&ray.points,ray.impact);
            for &id in &ray.targets {
                if engine::actor(r,id).is_none_or(|a|a.dead.value_or(false)||a.hp<=0.) {continue;}
                let before=engine::actor(r,id).unwrap().clone();
                let packets_before=crate::events::damage_count(id);
                match kind {
                    Effect::Lightning => engine::damage_direct_with_source_snapshot(r,id,profile.direct_damage,&source,"lightning",true)?,
                    Effect::Firebolt => {engine::damage_direct_with_source_snapshot(r,id,profile.direct_damage,&source,"fire",true)?;ignite_target(r,id,&source);},
                    Effect::Poison => poison_with_damage(r,id,source.id,profile.periodic_damage),
                    Effect::Acid => acid_with_damage(r,id,&source,profile.direct_damage)?, _ => unreachable!(),
                }
                crate::animation_amulets::outcome(r,&before,kind,packets_before);
            }
            if kind==Effect::Firebolt {fire_path(r,&ray)?;}
        },
    }
    if crate::revision52_amulet_tiers::enabled(r) {engine::log(r,format!("You activate {name}."),"player");} else {
    engine::log(r,format!("You activate {}.",match kind { Effect::Lightning=>"Lightning",Effect::Firebolt=>"Firebolt",Effect::Poison=>"Poison",Effect::Iceblast=>"Iceblast",Effect::Acid=>"Acid",Effect::Blinking=>"Blinking" }),"player");}
    Ok(Some(plan.consumes_action()))
}
/// Normal-clock only, including inactive-floor simulation; never haste phases.
pub fn tick_statuses(a: &mut Actor) {
    engine::tick_condition(a,ACID_TURNS);
    if engine::condition(a,ACID_TURNS)<=0. {a.conditions.shift_remove(ACID_TURNS);a.conditions.shift_remove(ACID_PENALTY);}
    if engine::condition(a,"poisoned")<=0. {a.conditions.shift_remove(POISON_DAMAGE);}
}
pub fn corrosion_penalty(r:&Run,a:&Actor)->f64 {
    if crate::revision50::enabled(r) && engine::condition(a,ACID_TURNS)>0. {engine::condition(a,ACID_PENALTY)} else {0.}
}

/// UI-only admission. Never query a hidden occupant or unrevealed tile/door.
/// Definitive collision remains in atomic command admission.
pub fn target_blocker_ui(r:&Run,item:&Item,x:f64,y:f64)->Option<&'static str> {
    if crate::revision52_amulet_tiers::retired(r,item) {return Some(crate::revision52_amulet_tiers::RETIRED_MESSAGE);}
    target_blocker_ui_base(r,item,x,y).or_else(|| {
        (r.simulation_revision.value_or(29.)>=52.&&effect(item)==Some(Effect::Blinking)
            &&known_blink_stays_put(r,item,x,y)).then_some("There is no reachable empty square to blink to.")
    })
}
fn known_blink_stays_put(r:&Run,item:&Item,x:f64,y:f64)->bool {
    aim_preview(r,item,x,y).impact==Some((r.player.x as usize,r.player.y as usize))
}
fn target_blocker_ui_base(r:&Run,item:&Item,x:f64,y:f64)->Option<&'static str> {
    let Some(kind)=effect(item) else {return crate::revision50_control_amulets::target_blocker_ui(r,item,x,y)};
    let Some(at)=coordinates(r,x,y) else {return Some("Choose a target on this floor.")};
    if kind!=Effect::Iceblast && r.player.x==x && r.player.y==y {return Some("Choose a direction away from yourself.");}
    let Some(observation)=crate::vision::observe_terrain(r) else {return Some("No floor is visible.")};
    let cell=&observation.cells[at];
    if !cell.current && !cell.remembered {return Some("Choose a known target tile.")}
    if matches!(kind,Effect::Iceblast|Effect::Blinking) {
        if !cell.current {return Some("Choose a currently visible target tile.")}
        if matches!(cell.tile.as_deref(),Some("wall"|"tree")) || cell.door_open==Some(false) {
            return Some("Choose an open target tile.")
        }
    }
    if kind==Effect::Blinking {
        if (r.player.x-x).hypot(r.player.y-y)>crate::revision52_amulet_tiers::profile(r,item).and_then(|p|p.range).unwrap_or(10.) {return Some(if crate::revision52_amulet_tiers::enabled(r){"The target is beyond this amulet’s blink range."}else{"Blinking reaches at most 10 tiles."})}
        if r.simulation_revision.value_or(29.)<52. && std::iter::once(&r.player).chain(r.actors_at(r.depth)).any(|a|a.id!=r.player.id
            && !a.dead.value_or(false)&&a.hp>0.&&crate::vision::visible_actor(r,a)
            && crate::dreamwalking::same_plane(r,&r.player,a)&&crate::portals::same_position(r,a,x,y)) {
            return Some("Choose an empty visible tile.")
        }
    }
    None
}
/// Detached aiming result. Delivery and affected area stay distinct so an AoE
/// cannot masquerade as a route. No Run state or random choice is retained.
#[derive(Clone,Debug,Default,PartialEq,Eq)]
pub struct AimPreview {
    pub delivery:Vec<(usize,usize)>,
    pub area:Vec<(usize,usize)>,
    pub impact:Option<(usize,usize)>,
    pub possible_exits:Vec<(usize,usize)>,
}
/// Legacy footprint adapter for callers which do not render delivery separately.
pub fn preview(r:&Run,item:&Item,x:f64,y:f64)->Vec<(usize,usize)> {
    let p=aim_preview(r,item,x,y);p.delivery.into_iter().chain(p.area).collect()
}
fn ice_area(out:&mut AimPreview,o:&crate::vision::TerrainObservation,center:(usize,usize)) {
    let (x,y)=(center.0 as i32,center.1 as i32);let w=o.width;
    let open=|xx:i32,yy:i32|xx>=0&&yy>=0&&(xx as usize)<w&&(yy as usize)<o.height
        &&o.cells.get(yy as usize*w+xx as usize).is_some_and(|c|c.current&&c.tile.is_some()
            &&!matches!(c.tile.as_deref(),Some("wall"|"tree"))&&c.door_open!=Some(false));
    for yy in y-1..=y+1 {for xx in x-1..=x+1 {
        if open(xx,yy)&&(xx==x||yy==y||open(xx,y)||open(x,yy)) {out.area.push((xx as usize,yy as usize));}
    }}
}
/// Knowledge-filtered geometry, using the same portal ray planner/traversal and
/// physical terrain policy as launch. O(observation + floor clone + ray-plan +
/// V*A*P), O(F+V+A+P) scratch. Unknown cells terminate the route, even if remembered.
pub fn aim_preview(r:&Run,item:&Item,x:f64,y:f64)->AimPreview {
    if crate::revision52_amulet_tiers::retired(r,item) {return AimPreview::default();}
    let mut out=AimPreview::default();
    let Some(kind)=effect(item) else {
        out.area=crate::revision50_control_amulets::preview(r,item,x,y);
        let control=crate::revision50_control_amulets::effect(item);
        if r.simulation_revision.value_or(29.)>=52. && control.is_some_and(|k|k!=crate::revision50_control_amulets::ControlEffect::Floods) {
            // Reuse the bounded, knowledge-only non-piercing ray geometry.
            // The synthetic item is solely a geometry selector, not an activation.
            let geometry=Item{kind:"amulet".into(),effect:"iceblast".to_string().into(),..Default::default()};
            let path=aim_preview(r,&geometry,x,y);
            out.delivery=path.delivery;out.possible_exits=path.possible_exits;out.impact=path.impact;
            out.area.clear();
            if let Some((xx,yy))=out.impact {
                if control==Some(crate::revision50_control_amulets::ControlEffect::Obstruction) {
                    out.area=crate::revision50_control_amulets::preview(r,item,xx as f64,yy as f64);
                } else if r.actors_at(r.depth).iter().any(|a|a.hp>0.&&!a.dead.value_or(false)
                    &&crate::dreamwalking::same_plane(r,&r.player,a)&&crate::vision::visible_actor(r,a)
                    &&crate::portals::same_position(r,a,xx as f64,yy as f64)) {out.area.push((xx,yy));}
            }
            return out;
        }
        if control==Some(crate::revision50_control_amulets::ControlEffect::Beckoning) {
            out.delivery=crate::revision50_control_amulets::beckoning_preview_with_range(r,x,y,crate::revision52_amulet_tiers::profile(r,item).and_then(|p|p.range).unwrap_or(f64::INFINITY));
        }
        return out;
    };
    let Some(at)=coordinates(r,x,y) else {return out};
    let Some(o)=crate::vision::observe_terrain(r) else {return out};
    let known=|at:usize|o.cells.get(at).is_some_and(|c|c.current&&c.tile.is_some());
    let f=crate::dreamwalking::floor_for(r,&r.player).unwrap();let w=f.width();
    if kind==Effect::Iceblast && r.simulation_revision.value_or(29.)<52. {
        if target_blocker_ui_base(r,item,x,y).is_some(){return out;}
        let opaque:Vec<_>=o.cells.iter().map(|c|!c.current || matches!(c.tile.as_deref(),Some("wall"|"tree")) || c.door_open==Some(false)).collect();
        for yy in y as i32-1..=y as i32+1 {for xx in x as i32-1..=x as i32+1 {
            let Some(at)=crate::portals::cell(f,xx as f64,yy as f64) else {continue};
            let corner_open=xx==x as i32||yy==y as i32
                || !opaque[y as usize*w+xx as usize]||!opaque[yy as usize*w+x as usize];
            if known(at)&&!opaque[at]&&corner_open {out.area.push((xx as usize,yy as usize));}
        }}
        out.impact=Some((x as usize,y as usize));return out;
    }
    if matches!(kind,Effect::Iceblast|Effect::Blinking) && target_blocker_ui_base(r,item,x,y).is_some(){return out;}
    if kind==Effect::Blinking && r.simulation_revision.value_or(29.)<51. {
        out.area.push((x as usize,y as usize));return out;
    }
    if r.player.x==x&&r.player.y==y {
        if kind==Effect::Iceblast {out.impact=Some((x as usize,y as usize));ice_area(&mut out,&o,(x as usize,y as usize));}
        return out;
    }
    // A remembered aim is legal, but only its continuously observed prefix is shown.
    if !o.cells[at].current&&!o.cells[at].remembered {return out;}
    let mut floor=f.clone();
    for (at,tile) in floor.tiles.iter_mut().enumerate() {
        *tile=if known(at){o.cells[at].tile.clone().unwrap()}else{"wall".into()};
    }
    floor.doors=crate::Field::Missing;
    floor.environment=crate::Field::Missing;
    let mut unknown_entries=std::collections::HashSet::new();
    if let Some(pairs)=floor.portals.as_mut() {pairs.retain(|p| {
        let all=known(p.a)&&known(p.b);
        if !all&&r.simulation_revision.value_or(29.)>=52. {unknown_entries.extend([p.a,p.b].into_iter().filter(|&at|known(at)));}
        all
    });}
    for pockets in [&mut floor.non_euclidean,&mut floor.speed_up] {
        if let Some(pockets)=pockets.as_mut() {for pocket in pockets.iter_mut() {
            pocket.groups.retain(|g| {
                let all=g.sources.iter().copied().chain(std::iter::once(g.destination)).all(known);
                if !all {unknown_entries.extend(g.sources.iter().copied().chain(std::iter::once(g.destination)).filter(|&at|known(at)));}
                all
            });
            pocket.cells=pocket.groups.iter().flat_map(|g|g.sources.iter().copied().chain(std::iter::once(g.destination))).collect();
            pocket.cells.sort_unstable();pocket.cells.dedup();
        }pockets.retain(|p|!p.groups.is_empty());}
    }
    let current_firebolt=kind==Effect::Firebolt&&crate::firebolt_travel::current(r);
    // A detached, knowledge-only burned view. No live ignition, hidden fuel,
    // hidden freezing aura or gameplay RNG may affect the targeting preview.
    let visible_freezers:Vec<_>=if current_firebolt {std::iter::once(&r.player).chain(r.actors_at(r.depth))
        .filter(|a|!a.dead.value_or(false)&&crate::dreamwalking::same_plane(r,&r.player,a)
            && (a.id==r.player.id||crate::vision::visible_actor(r,a))
            && a.active.iter().any(|id|id=="dawel-reverse-entropy")).collect()} else {vec![]};
    let burnable:Vec<_>=o.cells.iter().map(|c|current_firebolt&&known(c.index)&&!c.snow
        && !c.hazards.as_ref().is_some_and(|h|crate::environment::fire_active(h.fire.as_ref().map(String::as_str)))
        && !visible_freezers.iter().any(|a|(a.x-(c.index%w)as f64).hypot(a.y-(c.index/w)as f64)<=10.)).collect();
    let burned=if current_firebolt {let mut view=floor.clone();
        for (at,tile) in view.tiles.iter_mut().enumerate(){if burnable[at]&&tile=="tree"{*tile="floor".into();}}
        Some(view)}else{None};
    let pass=|p:Point|crate::portals::cell(&floor,p.x as f64,p.y as f64).is_some_and(|at|known(at)
        && !unknown_entries.contains(&at)&&o.cells[at].door_open!=Some(false)
        &&crate::projectiles::terrain_pass(r.simulation_revision.value_or(29.),&floor,p));
    let entered_pass=|p:Point|pass(p)||burned.as_ref().is_some_and(|view|
        crate::portals::cell(view,p.x as f64,p.y as f64).is_some_and(|at|known(at)
            && !unknown_entries.contains(&at)&&burnable[at]
            &&crate::projectiles::terrain_pass(r.simulation_revision.value_or(29.),view,p)));
    let from=Point{x:r.player.x as i32,y:r.player.y as i32};let target=Point{x:x as i32,y:y as i32};
    let plan=crate::portals::ray_plan(&floor,from,target,pass);
    if plan.origin_fork.len()>1 {
        out.possible_exits=plan.origin_fork.into_iter().filter(|&at|known(at)).map(|at|(at%w,at/w)).collect();return out;
    }
    let scale=if matches!(kind,Effect::Blinking|Effect::Iceblast){1}else{(floor.tiles.len()*2) as i32};
    let aim=Point{x:plan.origin.x+(plan.aim.x-plan.origin.x)*scale,y:plan.origin.y+(plan.aim.y-plan.origin.y)*scale};
    let visible:Vec<_>=r.actors_at(r.depth).iter().filter(|a|!a.dead.value_or(false)&&a.hp>0.
        &&crate::dreamwalking::same_plane(r,&r.player,a)&&crate::vision::visible_actor(r,a)
        && (!current_firebolt||engine::condition(a,"underwater")<=0.)).collect();
    let occupied=|p:Point|visible.iter().any(|a|crate::portals::same_cell(&floor,a.x,a.y,p.x as f64,p.y as f64));
    let prior=std::cell::Cell::new(plan.origin);
    let impact=std::cell::Cell::new(None);
    let unknown_stop=std::cell::Cell::new(false);
    let forks=std::cell::RefCell::new(vec![]);
    let ray=crate::portals::directed_ray_trace(&floor,plan.origin,aim,|p| {
        let alternative=crate::portals::cell(&floor,prior.get().x as f64,prior.get().y as f64)
            .zip(crate::portals::cell(&floor,p.x as f64,p.y as f64))
            .is_some_and(|(a,b)|a!=b&&crate::portals::exits(&floor,a).contains(&b));
        let open=if alternative{pass(p)}else{entered_pass(p)};
        if !open && crate::portals::cell(&floor,p.x as f64,p.y as f64).is_some_and(|at|!known(at)||unknown_entries.contains(&at)) {
            unknown_stop.set(true);
        }
        if !open && (p.x-prior.get().x).abs().max((p.y-prior.get().y).abs())<=1
            && crate::portals::cell(&floor,p.x as f64,p.y as f64).is_some_and(|at|known(at)&&!unknown_entries.contains(&at)) {
            impact.set(Some((p.x as usize,p.y as usize)));
        }
        open
    },|options| {
        if options.len()==1 {Some(options[0])}else{*forks.borrow_mut()=options.to_vec();None}
    },|p|{prior.set(p);kind!=Effect::Lightning&&occupied(p)});
    let mut distance=0.;let mut segment=plan.origin;let mut prior=plan.origin;let mut range_stop=false;
    for (i,p) in ray.points.iter().enumerate() {
        if i==0 {if *p!=from {out.delivery.push((p.x as usize,p.y as usize));}continue}
        let transfer=crate::portals::cell(&floor,prior.x as f64,prior.y as f64).zip(crate::portals::cell(&floor,p.x as f64,p.y as f64))
            .is_some_and(|(a,b)|a!=b&&crate::portals::exits(&floor,a).contains(&b));
        if transfer {distance+=crate::vision::distance(segment,prior);segment=*p;}
        if kind==Effect::Acid&&distance+crate::vision::distance(segment,*p)>8. {range_stop=true;break}
        out.delivery.push((p.x as usize,p.y as usize));prior=*p;
    }
    if !range_stop {
        out.impact=impact.get().or_else(||ray.points.last().filter(|p|occupied(**p)).map(|p|(p.x as usize,p.y as usize)));
        out.possible_exits=forks.into_inner().into_iter().filter(|&at|known(at)).map(|at|(at%w,at/w)).collect();
    }
    if kind==Effect::Blinking&&out.delivery.last()==Some(&(x as usize,y as usize))&&!occupied(target) {out.impact=Some((x as usize,y as usize));}
    if r.simulation_revision.value_or(29.)>=52. && matches!(kind,Effect::Iceblast|Effect::Blinking) {
        // An unknown prefix/fork is not a promised detonation or landing. Never
        // draw an AoE around the end of incomplete knowledge.
        let resolved=out.possible_exits.is_empty()&&!unknown_stop.get();
        out.impact=None;
        if resolved {
            let endpoint=if kind==Effect::Blinking {
                let phase=r.player.active.iter().any(|s|s=="etill-phase-through");
                let at=out.delivery.iter().rposition(|&(xx,yy)|!occupied(Point{x:xx as i32,y:yy as i32})
                    &&(xx as f64-r.player.x).hypot(yy as f64-r.player.y)<=crate::revision52_amulet_tiers::profile(r,item).and_then(|p|p.range).unwrap_or(10.)
                    &&!crate::revision50_terrain::clay_blocks_run(r,&r.player,xx as f64,yy as f64)
                    &&(phase||!o.cells[yy*w+xx].grate));
                out.delivery.truncate(at.map_or(0,|i|i+1));
                out.delivery.last().copied().unwrap_or((from.x as usize,from.y as usize))
            } else {out.delivery.last().copied().unwrap_or((from.x as usize,from.y as usize))};
            out.impact=Some(endpoint);
            if kind==Effect::Iceblast {ice_area(&mut out,&o,endpoint);} else {out.area.push(endpoint);}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Field,Stats};
    fn room()->Run {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=50.0.into();r.player.x=10.;r.player.y=10.;
        r.player.hp=100.;r.player.max_hp=100.;r.player.base_max_hp=100.0.into();r.player.energy=100.;
        r.player.stats=Stats{perception:300.,max_energy:100.,..Default::default()};
        r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
        r.player.equipment_abilities=Field::Missing;r.player.ability_order=Field::Missing;
        r.player.ability_reserves=Field::Missing;r.player.amulet_ledger=Field::Missing;
        r.inventory.clear();r.actors=Default::default();r.ground.clear();r.history.clear();r.haste_snapshot=Field::Missing;
        r.future=None;r.next_id=900.;r.phase="normal".to_string().into();
        let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.seen.fill(true);
        f.remembered_tiles=vec!["floor".into();f.tiles.len()].into();f.doors=Field::Missing;
        f.portals=Field::Missing;f.non_euclidean=Field::Missing;f.speed_up=Field::Missing;
        r
    }
    fn add(r:&mut Run,kind:Effect,id:f64){let mut item=make_amulet(kind,id).unwrap();item.equipped="amulet".to_string().into();r.inventory.push(item);}
    fn enemy(r:&mut Run,id:f64,x:f64,y:f64){
        let mut a=Actor{id,x,y,name:"Amulet test actor".into(),glyph:"e".into(),color:"#ffffff".into(),
            hp:500.,max_hp:500.,base_max_hp:500.0.into(),energy:100.,state:"hunting".into(),
            stats:Stats{max_energy:100.,..Default::default()},..Default::default()};
        a.conditions.insert("paralyzed".into(),100.);r.actors_at_mut(r.depth).push(a);
    }
    fn cast(r:&mut Run,id:f64,x:f64,y:f64){assert_eq!(command(r,&Command::UseAt{id,x,y}).unwrap(),Some(true));}
    fn hp(r:&Run,id:f64)->f64{engine::actor(r,id).unwrap().hp}
    fn ring(id:f64,effect:&str)->Item{Item{id,kind:"ring".into(),effect:effect.to_string().into(),equipped:"ring1".to_string().into(),..Default::default()}}
    #[test] fn r52_telepathy_admits_dark_ability_target_without_bypassing_effect_blockers(){
        let mut r=room();r.simulation_revision=52.0.into();r.player.stats.perception=0.;
        r.floor_mut().unwrap().lights.fill(-100.);enemy(&mut r,20.,18.,10.);
        r.inventory.push(make_amulet_for_revision(Effect::Iceblast,1.,52.).unwrap());
        assert!(!target_valid(&r,Effect::Iceblast,18.,10.));
        r.player.conditions.insert("telepathic".into(),30.);
        assert!(!crate::vision::directly_visible_actor(&r,engine::actor(&r,20.).unwrap()));
        assert!(target_valid(&r,Effect::Iceblast,18.,10.));
        let mut direct=r.clone();cast(&mut direct,1.,18.,10.);assert!(hp(&direct,20.)<500.);
        let mut wall=r.clone();let w=wall.floor().unwrap().width();wall.floor_mut().unwrap().tiles[10*w+14]="wall".into();
        cast(&mut wall,1.,18.,10.);assert_eq!(hp(&wall,20.),500.);
        let mut blocker=r.clone();enemy(&mut blocker,21.,14.,10.);
        cast(&mut blocker,1.,18.,10.);assert_eq!(hp(&blocker,20.),500.);assert!(hp(&blocker,21.)<500.);
    }
    #[test] fn lightning_continues_beyond_aim_pierces_allies_and_stops_at_wall(){
        let mut r=room();add(&mut r,Effect::Lightning,1.);enemy(&mut r,20.,13.,10.);enemy(&mut r,21.,30.,10.);enemy(&mut r,22.,35.,10.);
        engine::actor_mut_trusted(&mut r,21.).unwrap().state="ally".into();
        let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[10*w+33]="wall".into();
        cast(&mut r,1.,11.,10.);assert_eq!((hp(&r,20.),hp(&r,21.),hp(&r,22.)),(450.,450.,500.));
        assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,25.);assert_eq!(r.player.energy,100.);
    }
    #[test] fn firebolt_is_eighty_not_twenty_and_ignites_path_and_first_target(){
        let mut r=room();add(&mut r,Effect::Firebolt,1.);enemy(&mut r,20.,25.,10.);enemy(&mut r,21.,28.,10.);
        let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[10*w+15]="grass".into();
        cast(&mut r,1.,11.,10.);assert_eq!((hp(&r,20.),hp(&r,21.)),(420.,500.));
        assert!(engine::condition(engine::actor(&r,20.).unwrap(),"burning")>=6.);
        assert!(r.floor().unwrap().environment.as_ref().unwrap().fire[10*w+15]>0.);
        assert_eq!(r.floor().unwrap().environment.as_ref().unwrap().fire[10*w+12],0.);
    }
    #[test] fn firebolt_ignites_blocking_tree_but_never_hits_beyond_it(){
        let mut r=room();add(&mut r,Effect::Firebolt,1.);enemy(&mut r,20.,25.,10.);
        let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[10*w+20]="tree".into();
        cast(&mut r,1.,11.,10.);assert_eq!(hp(&r,20.),500.);
        assert!(r.floor().unwrap().environment.as_ref().unwrap().fire[10*w+20]>0.);
    }
    #[test] fn poison_is_six_per_normal_tick_for_twenty_and_refreshes_without_rate_stacking(){
        let mut r=room();add(&mut r,Effect::Poison,1.);enemy(&mut r,20.,30.,10.);enemy(&mut r,21.,32.,10.);
        cast(&mut r,1.,11.,10.);assert_eq!(engine::condition(engine::actor(&r,20.).unwrap(),"poisoned"),20.);
        for _ in 0..5 {crate::monster_statuses::tick_poisoned(&mut r,20.).unwrap();let a=engine::actor_mut_trusted(&mut r,20.).unwrap();engine::tick_condition(a,"poisoned");tick_statuses(a);}
        cast(&mut r,1.,11.,10.);assert_eq!(engine::condition(engine::actor(&r,20.).unwrap(),"poisoned"),20.);
        for _ in 0..20 {crate::monster_statuses::tick_poisoned(&mut r,20.).unwrap();let a=engine::actor_mut_trusted(&mut r,20.).unwrap();engine::tick_condition(a,"poisoned");tick_statuses(a);}
        assert_eq!(hp(&r,20.),350.);assert_eq!(hp(&r,21.),500.);
        assert!(!engine::actor(&r,20.).unwrap().conditions.contains_key(POISON_DAMAGE));
        crate::monster_statuses::tick_poisoned(&mut r,20.).unwrap();assert_eq!(hp(&r,20.),350.);
    }
    #[test] fn named_resistances_apply_to_final_packets_and_acid_penalty(){
        let mut r=room();enemy(&mut r,20.,12.,10.);
        engine::actor_mut_trusted(&mut r,20.).unwrap().copied_inventory=vec![ring(30.,"poison-resistance"),ring(31.,"corrosion-resistance")].into();
        let source=r.player.clone();poison(&mut r,20.,source.id);crate::monster_statuses::tick_poisoned(&mut r,20.).unwrap();assert_eq!(hp(&r,20.),497.);
        acid(&mut r,20.,&source).unwrap();assert_eq!(hp(&r,20.),477.);
        assert_eq!(crate::combat::effective_stats(&r,engine::actor(&r,20.).unwrap()).dr,-5.);
        for _ in 0..6 {tick_statuses(engine::actor_mut_trusted(&mut r,20.).unwrap());}
        assert_eq!(crate::combat::effective_stats(&r,engine::actor(&r,20.).unwrap()).dr,0.);
    }
    #[test] fn acid_has_range_eight_and_refreshes_six_turn_penalty(){
        let mut r=room();add(&mut r,Effect::Acid,1.);enemy(&mut r,20.,19.,10.);
        cast(&mut r,1.,11.,10.);assert_eq!(hp(&r,20.),500.);
        engine::actor_mut_trusted(&mut r,20.).unwrap().x=18.;cast(&mut r,1.,11.,10.);
        assert_eq!(hp(&r,20.),460.);assert_eq!(corrosion_penalty(&r,engine::actor(&r,20.).unwrap()),10.);
        let source=r.player.clone();acid(&mut r,20.,&source).unwrap();assert_eq!(engine::condition(engine::actor(&r,20.).unwrap(),ACID_TURNS),6.);
        assert_eq!(corrosion_penalty(&r,engine::actor(&r,20.).unwrap()),10.);
    }
    #[test] fn iceblast_thirty_hits_player_and_allies_while_immunity_blocks_damage_and_slow(){
        let mut r=room();add(&mut r,Effect::Iceblast,1.);enemy(&mut r,20.,11.,10.);enemy(&mut r,21.,10.,11.);
        engine::actor_mut_trusted(&mut r,21.).unwrap().state="ally".into();
        engine::actor_mut_trusted(&mut r,20.).unwrap().copied_inventory=vec![ring(30.,"ice-immunity")].into();
        cast(&mut r,1.,10.,10.);assert_eq!(r.player.hp,70.);assert_eq!(hp(&r,20.),500.);assert_eq!(hp(&r,21.),470.);
        assert_eq!(engine::condition(&r.player,"slowed"),3.);assert_eq!(engine::condition(engine::actor(&r,20.).unwrap(),"slowed"),0.);
    }
    #[test] fn blinking_visible_empty_within_ten_and_arrival_fire(){
        let mut r=room();add(&mut r,Effect::Blinking,1.);
        assert!(!target_valid(&r,Effect::Blinking,21.,10.));enemy(&mut r,20.,15.,10.);assert!(!target_valid(&r,Effect::Blinking,15.,10.));
        crate::environment::ignite(&mut r,20.,10.,None,None).unwrap();cast(&mut r,1.,20.,10.);
        assert_eq!((r.player.x,r.player.y),(20.,10.));assert!(engine::condition(&r.player,"burning")>0.);
    }
    #[test] fn invalid_target_and_insufficient_combined_payment_leave_resources_untouched(){
        let mut r=room();add(&mut r,Effect::Lightning,1.);r.player.abilities.push("isheilah-spellslinger".into());
        for x in [f64::NAN,-1.,10.] {
            let before=serde_json::to_value(&r).unwrap();assert!(admission(&r,&Command::UseAt{id:1.,x,y:10.}).unwrap().is_err());assert_eq!(serde_json::to_value(&r).unwrap(),before);
        }
        // Lightning costs25: one item energy plus23 player energy is insufficient.
        r.inventory[0].amulet_state.as_mut().unwrap().energy=1.;r.player.energy=23.;
        let use_at=Command::UseAt{id:1.,x:11.,y:10.};
        let before=serde_json::to_value(&r).unwrap();assert!(admission(&r,&use_at).unwrap().is_err());assert_eq!(serde_json::to_value(&r).unwrap(),before);
        let resources=(r.inventory[0].amulet_state.clone(),r.player.energy,r.player.amulet_ledger.clone(),r.rng,r.tick);
        assert!(command(&mut r,&use_at).is_err());
        assert_eq!((r.inventory[0].amulet_state.clone(),r.player.energy,r.player.amulet_ledger.clone(),r.rng,r.tick),resources);
        // At exactly the combined cost, partial item funding is valid and the
        // read-only plan must not claim the free activation until actual commit.
        r.player.energy=24.;let before=serde_json::to_value(&r).unwrap();
        assert!(admission(&r,&use_at).unwrap().is_ok());assert_eq!(serde_json::to_value(&r).unwrap(),before);
        assert_eq!(command(&mut r,&use_at).unwrap(),Some(false));
        assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,0.);assert_eq!(r.player.energy,0.);
        assert_eq!(r.player.amulet_ledger.as_ref().unwrap().commit_sequence,1);
        assert_eq!((r.rng,r.tick),(resources.3,resources.4));
    }
    #[test] fn dispatch_free_activation_then_other_amulet_consumes_turn(){
        let mut r=room();add(&mut r,Effect::Lightning,1.);add(&mut r,Effect::Poison,2.);r.player.abilities.push("isheilah-spellslinger".into());
        let tick=r.tick;let rng=r.rng;
        let free=engine::dispatch(&mut r,&Command::UseAt{id:1.,x:11.,y:10.}).unwrap();
        assert!(free.used_item&&!free.took_turn);
        assert_eq!(r.tick,tick);assert_eq!(r.rng,rng);assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,25.);
        let paid=engine::dispatch(&mut r,&Command::UseAt{id:2.,x:11.,y:10.}).unwrap();
        assert!(paid.used_item&&paid.took_turn);assert_eq!(r.tick,tick+1.);
        assert_eq!(r.inventory[1].amulet_state.as_ref().unwrap().energy,35.);
    }
    #[test] fn saved_item_energy_and_poison_rate_replay_identically(){
        let mut r=room();add(&mut r,Effect::Poison,1.);enemy(&mut r,20.,30.,10.);cast(&mut r,1.,11.,10.);
        let mut restored:Run=serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        crate::monster_statuses::tick_poisoned(&mut r,20.).unwrap();crate::monster_statuses::tick_poisoned(&mut restored,20.).unwrap();
        assert_eq!(serde_json::to_value(r).unwrap(),serde_json::to_value(restored).unwrap());
    }
    #[test] fn spatial_ray_cycles_are_bounded_and_duplicate_actor_is_hit_once(){
        let mut r=room();r.player.x=12.;add(&mut r,Effect::Lightning,1.);enemy(&mut r,20.,13.,10.);
        let w=r.floor().unwrap().width();r.floor_mut().unwrap().portals=vec![crate::portals_generation::PortalPair{a:10*w+14,b:10*w+11,color_index:0}].into();
        let ray=trace(&r,Effect::Lightning,13.,10.);assert!(ray.points.len()<=r.floor().unwrap().tiles.len()*4);assert_eq!(ray.targets,vec![20.]);
    }
    #[test] fn iceblast_closed_corner_shields_and_corrosion_compounds_sustain(){
        let mut r=room();enemy(&mut r,20.,11.,11.);let w=r.floor().unwrap().width();
        r.floor_mut().unwrap().tiles[10*w+11]="wall".into();r.floor_mut().unwrap().tiles[11*w+10]="wall".into();
        let source=r.player.clone();iceblast(&mut r,10.,10.,&source).unwrap();assert_eq!(hp(&r,20.),500.);
        let a=engine::actor_mut_trusted(&mut r,20.).unwrap();a.copied_inventory=vec![ring(30.,"corrosion-resistance"),ring(31.,"sustain-strength")].into();a.stats.dr=-4.;
        acid(&mut r,20.,&source).unwrap();assert_eq!(crate::combat::effective_stats(&r,engine::actor(&r,20.).unwrap()).dr,-6.5);
    }
    #[test] fn preview_cannot_leak_unseen_live_terrain_or_consume_rng(){
        let mut r=room();add(&mut r,Effect::Lightning,1.);r.player.stats.perception=-1000.;
        let f=r.floor_mut().unwrap();f.lights.fill(-100.);f.light_sources=Field::Missing;f.environment=Field::Missing;let w=f.width();
        let item=r.inventory[0].clone();let before=r.rng;let first=preview(&r,&item,11.,10.);
        assert!(!crate::vision::visible(&r,20,10));
        r.floor_mut().unwrap().tiles[10*w+20]="wall".into();
        assert_eq!(preview(&r,&item,11.,10.),first);assert_eq!(r.rng,before);
    }
    #[test] fn invalid_dispatch_does_not_start_clocks_or_claim_free_credit(){
        let mut r=room();add(&mut r,Effect::Poison,1.);r.player.abilities.push("isheilah-spellslinger".into());
        let state=r.inventory[0].amulet_state.clone();let tick=r.tick;let rng=r.rng;
        assert!(!engine::dispatch(&mut r,&Command::UseAt{id:1.,x:10.,y:10.}).unwrap().took_turn);
        assert_eq!(r.inventory[0].amulet_state,state);assert_eq!(r.tick,tick);assert_eq!(r.rng,rng);assert!(r.player.amulet_ledger.is_missing());
    }

    #[test] fn cold_immunity_is_enforced_at_shared_damage_packet_boundary(){
        let mut r=room();enemy(&mut r,20.,12.,10.);
        engine::actor_mut_trusted(&mut r,20.).unwrap().copied_inventory=vec![ring(30.,"ice-immunity")].into();
        let source=r.player.clone();let before=serde_json::to_value(engine::actor(&r,20.).unwrap()).unwrap();
        engine::damage_direct_with_source_snapshot(&mut r,20.,30.,&source,"cold",true).unwrap();
        assert_eq!(serde_json::to_value(engine::actor(&r,20.).unwrap()).unwrap(),before);
        engine::actor_mut_trusted(&mut r,20.).unwrap().copied_inventory=Field::Missing;
        engine::damage_direct_with_source_snapshot(&mut r,20.,30.,&source,"cold",true).unwrap();assert_eq!(hp(&r,20.),470.);
    }

    fn duel_room()->Run {
        let mut r=room();enemy(&mut r,20.,12.,10.);
        r.player.abilities.push(crate::revision50::DUEL.into());r.player.active.push(crate::revision50::DUEL.into());
        for (key,value) in [("r50DuelTarget",21.),("r50DuelX",10.),("r50DuelY",10.),("r50DuelDepth",r.depth)] {r.player.conditions.insert(key.into(),value);}
        r
    }
    #[test] fn fated_duel_blocks_poison_source_without_curing_existing_poison(){
        let mut r=duel_room();poison(&mut r,0.,20.);assert_eq!(engine::condition(&r.player,"poisoned"),0.);
        r.player.conditions.insert("r50DuelTarget".into(),20.);poison(&mut r,0.,20.);assert_eq!(engine::condition(&r.player,"poisoned"),20.);
        r.player.conditions.insert("r50DuelTarget".into(),21.);crate::monster_statuses::tick_poisoned(&mut r,0.).unwrap();assert_eq!(r.player.hp,94.);
    }
    #[test] fn fated_duel_blocks_acid_damage_and_attached_dr_loss(){
        let mut r=duel_room();let source=engine::actor(&r,20.).unwrap().clone();
        acid(&mut r,0.,&source).unwrap();assert_eq!(r.player.hp,100.);assert_eq!(engine::condition(&r.player,ACID_TURNS),0.);
        r.player.conditions.insert("r50DuelTarget".into(),20.);acid(&mut r,0.,&source).unwrap();assert_eq!(r.player.hp,60.);assert_eq!(engine::condition(&r.player,ACID_PENALTY),10.);
    }
    #[test] fn fated_duel_blocks_iceblast_damage_and_attached_slow(){
        let mut r=duel_room();let source=engine::actor(&r,20.).unwrap().clone();
        iceblast(&mut r,10.,10.,&source).unwrap();assert_eq!(r.player.hp,100.);assert_eq!(engine::condition(&r.player,"slowed"),0.);
        r.player.conditions.insert("r50DuelTarget".into(),20.);iceblast(&mut r,10.,10.,&source).unwrap();assert_eq!(r.player.hp,70.);assert_eq!(engine::condition(&r.player,"slowed"),3.);
    }

    #[test] fn inactive_floor_uses_the_same_poison_and_corrosion_normal_clock(){
        let mut r=room();enemy(&mut r,20.,12.,10.);let source=r.player.clone();
        poison(&mut r,20.,source.id);acid(&mut r,20.,&source).unwrap();
        let actor=r.actors_at_mut(r.depth).remove(0);r.actors_at_mut(2.).push(actor);
        let f=crate::mutation_journal::make_mut(r.floors.iter_mut().find(|f|f.depth==2.).unwrap());
        f.tiles.fill("floor".into());f.environment=Field::Missing;f.last_tick=0.;f.dirty=true;f.seen.fill(true);
        r.tick=1.;crate::offscreen::tick_inactive_floors(&mut r).unwrap();
        let a=r.actors_at(2.).iter().find(|a|a.id==20.).unwrap();
        assert_eq!(engine::condition(a,"poisoned"),19.);assert_eq!(engine::condition(a,POISON_DAMAGE),6.);assert_eq!(engine::condition(a,ACID_TURNS),5.);
    }

    fn aimed_room()->Run {
        let mut r=room();r.simulation_revision=51.0.into();
        r.player.conditions.insert("allSeeingReveal".into(),1.);
        r.player.conditions.insert("allSeeingDepth".into(),r.depth);r
    }
    #[test] fn r51_blink_line_blockers_reject_before_any_payment_or_rng() {
        for obstruction in ["creature","wall","door","Borge","Epidite"] {
            let mut r=aimed_room();add(&mut r,Effect::Blinking,1.);let w=r.floor().unwrap().width();
            match obstruction {
                "creature"=>enemy(&mut r,20.,12.,10.),
                "wall"=>r.floor_mut().unwrap().tiles[10*w+12]="wall".into(),
                "door"=>{
                    let mut doors=indexmap::IndexMap::new();
                    doors.insert((10*w+12).to_string(),crate::FloorDoorsEntry{open:false,..Default::default()});
                    r.floor_mut().unwrap().doors=doors.into();
                },
                kind=>r.floor_mut().unwrap().godbeast_event=crate::GodbeastEvent{kind:kind.into(),x:14.,y:10.,..Default::default()}.into(),
            }
            let before=r.clone();let command=Command::UseAt{id:1.,x:18.,y:10.};
            assert!(admission(&r,&command).unwrap().is_err(),"{obstruction}");
            assert!(super::command(&mut r,&command).is_err(),"{obstruction}");
            assert_eq!(r,before,"blocked {obstruction} changed state");
        }
    }
    #[test] fn r51_directional_preview_matches_launch_and_keeps_piercing_and_range() {
        for kind in [Effect::Lightning,Effect::Firebolt,Effect::Poison,Effect::Acid] {
            let mut r=aimed_room();enemy(&mut r,20.,14.,10.);enemy(&mut r,21.,20.,10.);
            let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[10*w+25]="wall".into();
            let item=make_amulet(kind,1.).unwrap();let before=r.clone();
            let got=aim_preview(&r,&item,11.,10.);let actual=trace(&r,kind,11.,10.);
            assert_eq!(got.delivery,actual.points.iter().skip(1).map(|p|(p.x as usize,p.y as usize)).collect::<Vec<_>>(),"{kind:?}");
            assert!(got.area.is_empty());assert_eq!(r,before);
            if kind==Effect::Lightning {assert!(got.delivery.contains(&(20,10)));assert_eq!(got.impact,Some((25,10)));}
            else {assert_eq!(got.delivery.last(),Some(&(14,10)));assert_eq!(got.impact,Some((14,10)));}
        }
        let r=aimed_room();let item=make_amulet(Effect::Acid,1.).unwrap();
        assert_eq!(aim_preview(&r,&item,11.,10.).delivery.last(),Some(&(18,10)));
    }
    #[test] fn r51_fixed_fixtures_block_bow_and_amulet_but_old_replays_keep_collision() {
        for kind in ["Borge","Epidite"] {
            let mut r=aimed_room();r.floor_mut().unwrap().godbeast_event=crate::GodbeastEvent{kind:kind.into(),x:15.,y:10.,..Default::default()}.into();
            let hit=crate::projectiles::trace(&r,20.,10.);assert!(hit.x<15.,"{kind}");
            let item=make_amulet(Effect::Lightning,1.).unwrap();let p=aim_preview(&r,&item,11.,10.);
            assert!(!p.delivery.contains(&(15,10)),"{kind}");assert!(p.impact.is_some());
            r.simulation_revision=50.0.into();assert_eq!(crate::projectiles::trace(&r,20.,10.).x,20.);
        }
    }
    #[test] fn r51_hidden_creatures_and_remembered_blockers_do_not_change_aim() {
        let mut r=room();r.simulation_revision=51.0.into();r.player.stats.perception=-1000.;
        let f=r.floor_mut().unwrap();f.lights.fill(-100.);f.light_sources=Field::Missing;f.environment=Field::Missing;let w=f.width();
        let item=make_amulet(Effect::Lightning,1.).unwrap();let before=aim_preview(&r,&item,30.,10.);
        assert!(!before.delivery.contains(&(20,10)));assert!(!crate::vision::visible(&r,20,10));
        r.floor_mut().unwrap().tiles[10*w+20]="wall".into();enemy(&mut r,30.,19.,10.);
        assert_eq!(aim_preview(&r,&item,30.,10.),before);
        let mut r=aimed_room();let item=make_amulet(Effect::Blinking,1.).unwrap();
        // Actor invisibility is unobserved even while its terrain is visible.
        r.player.conditions.shift_remove("allSeeingReveal");r.player.conditions.shift_remove("allSeeingDepth");
        let clear=aim_preview(&r,&item,15.,10.);enemy(&mut r,30.,12.,10.);
        engine::actor_mut_trusted(&mut r,30.).unwrap().conditions.insert("invisible".into(),10.);
        assert!(!crate::vision::visible_actor(&r,engine::actor(&r,30.).unwrap()));
        assert_eq!(aim_preview(&r,&item,15.,10.),clear);
        assert!(!target_valid(&r,Effect::Blinking,15.,10.));
    }
    #[test] fn r51_ice_keeps_authored_area_without_invented_delivery() {
        let r=aimed_room();let item=make_amulet(Effect::Iceblast,1.).unwrap();let before=r.clone();
        let p=aim_preview(&r,&item,15.,10.);assert!(p.delivery.is_empty());assert_eq!(p.area.len(),9);assert_eq!(p.impact,Some((15,10)));assert_eq!(r,before);
    }
    #[test] fn r51_portal_delivery_range_and_blink_endpoint_remain_physical() {
        let mut r=aimed_room();let w=r.floor().unwrap().width();
        r.floor_mut().unwrap().portals=vec![crate::portals_generation::PortalPair{a:10*w+12,b:10*w+30,color_index:0}].into();
        let item=make_amulet(Effect::Acid,1.).unwrap();let p=aim_preview(&r,&item,11.,10.);
        assert!(p.delivery.contains(&(30,10)));assert!(p.delivery.contains(&(36,10)));assert!(!p.delivery.contains(&(37,10)));
        assert_eq!(p.delivery,trace(&r,Effect::Acid,11.,10.).points.iter().skip(1).map(|p|(p.x as usize,p.y as usize)).collect::<Vec<_>>());
        assert!(!target_valid(&r,Effect::Blinking,30.,10.),"Blink keeps ten Euclidean endpoint range even across portals");
    }

    fn r52_room()->Run {let mut r=aimed_room();r.simulation_revision=52.0.into();r}
    #[test] fn r52_iceblast_uses_first_creature_and_actual_animation_area() {
        let mut r=r52_room();add(&mut r,Effect::Iceblast,1.);enemy(&mut r,20.,14.,10.);enemy(&mut r,21.,18.,10.);
        let before=r.clone();let p=aim_preview(&r,&r.inventory[0],18.,10.);
        assert_eq!(p.delivery,vec![(11,10),(12,10),(13,10),(14,10)]);
        assert_eq!(p.impact,Some((14,10)));assert_eq!(p.area.len(),9);assert!(!p.area.contains(&(18,10)));assert_eq!(r,before);
        crate::events::begin();cast(&mut r,1.,18.,10.);let events=crate::events::finish();
        assert_eq!((hp(&r,20.),hp(&r,21.)),(470.,500.));
        let route=events.iter().filter_map(|e|e.animation.as_ref()).find(|a|a.proposal=="A04"&&a.stage==crate::events::Stage::Travel).unwrap();
        assert_eq!(route.cells.last(),Some(&crate::events::EventPoint{x:14.,y:10.}));
        let area=events.iter().filter_map(|e|e.animation.as_ref()).find(|a|a.proposal=="A04"&&a.style=="crystal").unwrap();
        assert_eq!(area.cells.iter().map(|p|(p.x as usize,p.y as usize)).collect::<Vec<_>>(),p.area);
        assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,0.);
        assert_eq!(r.player.energy,before.player.energy-20.);
    }
    #[test] fn r52_iceblast_and_blink_resolve_before_walls_doors_and_fixtures() {
        for obstruction in ["wall","door","Borge","Epidite"] {for kind in [Effect::Iceblast,Effect::Blinking] {
            let mut r=r52_room();add(&mut r,kind,1.);let w=r.floor().unwrap().width();
            match obstruction {
                "wall"=>r.floor_mut().unwrap().tiles[10*w+14]="wall".into(),
                "door"=>{let mut d=indexmap::IndexMap::new();d.insert((10*w+14).to_string(),crate::FloorDoorsEntry{open:false,..Default::default()});r.floor_mut().unwrap().doors=d.into();},
                kind=>r.floor_mut().unwrap().godbeast_event=crate::GodbeastEvent{kind:kind.into(),x:15.,y:10.,..Default::default()}.into(),
            }
            let p=aim_preview(&r,&r.inventory[0],18.,10.);let ray=trace(&r,kind,18.,10.);
            let endpoint=if kind==Effect::Blinking {blink_landing(&r,&ray)} else {*ray.points.last().unwrap()};
            assert!(endpoint.x<15,"{obstruction} {kind:?}");
            assert_eq!(p.impact,Some((endpoint.x as usize,endpoint.y as usize)),"{obstruction} {kind:?}");
            assert_eq!(p.delivery.last(),p.impact.as_ref());
            assert!(admission(&r,&Command::UseAt{id:1.,x:18.,y:10.}).unwrap().is_ok());
            cast(&mut r,1.,18.,10.);
            if kind==Effect::Blinking {assert_eq!((r.player.x,r.player.y),(endpoint.x as f64,endpoint.y as f64));}
        }}
    }
    #[test] fn r52_blink_backs_up_from_occupied_or_inaccessible_landing() {
        for target in [14.,18.] {
            let mut r=r52_room();add(&mut r,Effect::Blinking,1.);enemy(&mut r,20.,14.,10.);
            let w=r.floor().unwrap().width();r.floor_mut().unwrap().grates=vec![(10*w+13)as f64].into();
            let p=aim_preview(&r,&r.inventory[0],target,10.);
            assert!(target_blocker_ui(&r,&r.inventory[0],target,10.).is_none());
            assert_eq!(p.delivery.last(),Some(&(12,10)));assert_eq!(p.impact,Some((12,10)));
            crate::events::begin();cast(&mut r,1.,target,10.);let events=crate::events::finish();
            assert_eq!((r.player.x,r.player.y),(12.,10.));assert_eq!(hp(&r,20.),500.);
            assert!(events.iter().filter_map(|e|e.animation.as_ref()).any(|a|a.proposal=="A06"&&a.style=="unfold"&&a.cells.last()==Some(&crate::events::EventPoint{x:12.,y:10.})));
        }
        let mut r=r52_room();add(&mut r,Effect::Blinking,1.);enemy(&mut r,20.,11.,10.);
        let p=aim_preview(&r,&r.inventory[0],18.,10.);assert!(p.delivery.is_empty());assert_eq!(p.impact,Some((10,10)));
        let before=r.clone();let c=Command::UseAt{id:1.,x:18.,y:10.};
        assert!(target_blocker_ui(&r,&r.inventory[0],18.,10.).is_some());
        assert!(admission(&r,&c).unwrap().is_err());assert!(command(&mut r,&c).is_err());assert_eq!(r,before);
        assert!(!target_valid(&r,Effect::Blinking,21.,10.));
    }
    #[test] fn r52_hidden_adjacent_blink_blocker_does_not_become_a_free_probe() {
        let mut r=r52_room();add(&mut r,Effect::Blinking,1.);
        r.player.conditions.shift_remove("allSeeingReveal");r.player.conditions.shift_remove("allSeeingDepth");
        let clear=aim_preview(&r,&r.inventory[0],18.,10.);let c=Command::UseAt{id:1.,x:18.,y:10.};
        enemy(&mut r,20.,11.,10.);engine::actor_mut_trusted(&mut r,20.).unwrap().conditions.insert("invisible".into(),20.);
        assert!(!crate::vision::visible_actor(&r,engine::actor(&r,20.).unwrap()));
        assert_eq!(aim_preview(&r,&r.inventory[0],18.,10.),clear);
        assert!(target_blocker_ui(&r,&r.inventory[0],18.,10.).is_none());assert!(admission(&r,&c).unwrap().is_ok());
        let main_energy=r.player.energy;cast(&mut r,1.,18.,10.);
        assert_eq!((r.player.x,r.player.y),(10.,10.));assert_eq!(r.inventory[0].amulet_state.as_ref().unwrap().energy,0.);
        assert_eq!(r.player.energy,main_energy-10.);
    }
    #[test] fn r52_telepathic_aim_still_detonates_on_intervening_creature() {
        let mut r=r52_room();add(&mut r,Effect::Iceblast,1.);
        r.player.conditions.clear();r.player.conditions.insert("telepathic".into(),20.);
        r.player.stats.perception=-1000.;
        let f=r.floor_mut().unwrap();f.lights.fill(-100.);f.light_sources=Field::Missing;f.environment=Field::Missing;
        enemy(&mut r,20.,14.,10.);enemy(&mut r,21.,18.,10.);
        assert!(crate::ability_sight::actor(&r,engine::actor(&r,21.).unwrap()));
        let command=Command::UseAt{id:1.,x:18.,y:10.};
        assert!(admission(&r,&command).unwrap().is_ok());
        assert_eq!(trace(&r,Effect::Iceblast,18.,10.).points.last(),Some(&Point{x:14,y:10}));
        cast(&mut r,1.,18.,10.);
        assert_eq!(hp(&r,20.),470.);assert_eq!(hp(&r,21.),500.);
    }
    #[test] fn r52_hidden_interceptor_never_changes_preview_or_admission() {
        for kind in [Effect::Iceblast,Effect::Blinking,Effect::Firebolt,Effect::Poison] {
            let mut r=r52_room();add(&mut r,kind,1.);
            r.player.conditions.shift_remove("allSeeingReveal");r.player.conditions.shift_remove("allSeeingDepth");
            let p=aim_preview(&r,&r.inventory[0],18.,10.);let c=Command::UseAt{id:1.,x:18.,y:10.};
            enemy(&mut r,20.,14.,10.);engine::actor_mut_trusted(&mut r,20.).unwrap().conditions.insert("invisible".into(),20.);
            assert!(!crate::vision::visible_actor(&r,engine::actor(&r,20.).unwrap()));
            assert_eq!(aim_preview(&r,&r.inventory[0],18.,10.),p,"{kind:?}");
            assert!(admission(&r,&c).unwrap().is_ok());let ray=trace(&r,kind,18.,10.);assert_eq!(ray.points.last(),Some(&Point{x:14,y:10}));
            cast(&mut r,1.,18.,10.);
            if kind==Effect::Blinking {assert_eq!(r.player.x,13.);}else if kind==Effect::Iceblast {assert_eq!(hp(&r,20.),470.);}
        }
    }
    #[test] fn r52_unknown_terrain_stops_ink_without_promising_an_endpoint() {
        let mut r=r52_room();r.player.conditions.clear();r.player.stats.perception=-1000.;
        let f=r.floor_mut().unwrap();f.lights.fill(-100.);f.light_sources=Field::Missing;f.environment=Field::Missing;let w=f.width();
        for kind in [Effect::Iceblast,Effect::Blinking,Effect::Firebolt] {
            let item=make_amulet(kind,1.).unwrap();let p=aim_preview(&r,&item,18.,10.);
            let mut changed=r.clone();changed.floor_mut().unwrap().tiles[10*w+14]="wall".into();enemy(&mut changed,20.,13.,10.);
            assert_eq!(aim_preview(&changed,&item,18.,10.),p);assert!(p.area.is_empty());assert!(p.impact.is_none());
        }
    }
    #[test] fn r52_lightning_still_pierces_and_retired_acid_has_no_preview() {
        for kind in [Effect::Lightning,Effect::Firebolt,Effect::Poison] {
            let mut r=r52_room();add(&mut r,kind,1.);enemy(&mut r,20.,14.,10.);enemy(&mut r,21.,20.,10.);
            let p=aim_preview(&r,&r.inventory[0],18.,10.);let ray=trace(&r,kind,18.,10.);
            assert_eq!(p.delivery,ray.points.iter().skip(1).map(|p|(p.x as usize,p.y as usize)).collect::<Vec<_>>());
            if kind==Effect::Lightning {assert!(p.delivery.contains(&(20,10)));assert_eq!(ray.targets,vec![20.,21.]);}
            else {assert_eq!(p.impact,Some((14,10)));assert_eq!(ray.targets,vec![20.]);}
        }
        let r=r52_room();let item=make_amulet(Effect::Acid,1.).unwrap();assert!(aim_preview(&r,&item,20.,10.).delivery.is_empty());
    }
    #[test] fn r52_finite_portal_rays_and_save_restore_are_deterministic() {
        for kind in [Effect::Iceblast,Effect::Blinking] {
            let mut r=r52_room();add(&mut r,kind,1.);let w=r.floor().unwrap().width();
            r.floor_mut().unwrap().portals=vec![crate::portals_generation::PortalPair{a:10*w+12,b:10*w+16,color_index:0}].into();
            enemy(&mut r,20.,17.,10.);let p=aim_preview(&r,&r.inventory[0],18.,10.);let ray=trace(&r,kind,18.,10.);
            let endpoint=if kind==Effect::Blinking{blink_landing(&r,&ray)}else{*ray.points.last().unwrap()};
            assert_eq!(p.impact,Some((endpoint.x as usize,endpoint.y as usize)));
            let mut saved:Run=serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
            cast(&mut r,1.,18.,10.);cast(&mut saved,1.,18.,10.);assert_eq!(r,saved);
        }
    }
    #[test] fn r52_iceblast_hits_portal_alias_interceptor_once_and_hidden_aim_is_pure() {
        for hidden in [false,true] {
            let mut r=r52_room();add(&mut r,Effect::Iceblast,1.);let w=r.floor().unwrap().width();
            r.floor_mut().unwrap().portals=vec![crate::portals_generation::PortalPair{a:10*w+12,b:10*w+30,color_index:0}].into();
            if hidden {r.player.conditions.shift_remove("allSeeingReveal");r.player.conditions.shift_remove("allSeeingDepth");}
            let clear=aim_preview(&r,&r.inventory[0],18.,10.);enemy(&mut r,20.,30.,10.);
            if hidden {engine::actor_mut_trusted(&mut r,20.).unwrap().conditions.insert("invisible".into(),20.);}
            let before=r.clone();let p=aim_preview(&r,&r.inventory[0],18.,10.);assert_eq!(r,before);
            if hidden {assert_eq!(p,clear);}else {assert_eq!(p.impact,Some((12,10)));}
            assert_eq!(trace(&r,Effect::Iceblast,18.,10.).points.last(),Some(&Point{x:12,y:10}));
            cast(&mut r,1.,18.,10.);assert_eq!(hp(&r,20.),470.);assert_eq!(engine::condition(engine::actor(&r,20.).unwrap(),"slowed"),3.);
        }
    }

}

/// Shared player-facing admission copy; invalid-state diagnostics stay distinct.
pub fn admission_message(error:crate::revision50_amulets::AdmissionError)->String {
 if error==crate::revision50_amulets::AdmissionError::InsufficientPlayerEnergy {"Insufficient Energy".into()}else{format!("This amulet cannot activate: {error:?}.")}
}
#[cfg(test)]mod insufficient_energy_copy_tests {
 use super::*;use crate::revision50_amulets::AdmissionError;
 #[test]fn only_insufficient_energy_uses_short_message(){assert_eq!(admission_message(AdmissionError::InsufficientPlayerEnergy),"Insufficient Energy");assert_eq!(admission_message(AdmissionError::InvalidState),"This amulet cannot activate: InvalidState.");}
}
