//! Authored integer rays and inventory projectile commands; no geometry RNG.
use crate::{
    Command, Field, GroundItem, Item, Run,
    engine::{EngineError, log},
};
#[derive(Clone, Copy, Debug)]
pub struct Landing {
    pub x: f64,
    pub y: f64,
    pub actor_id: Option<f64>,
    pub distance: Option<f64>,
}
/// Pure physical terrain policy shared by launch and observed aiming. Callers
/// must gate knowledge before passing a cell. Fixed fixtures are R51 blockers;
/// old replay revisions retain their original collision contract. O(1).
pub fn terrain_pass(revision:f64, f:&crate::Floor, p:crate::movement::Point)->bool {
    crate::portals::cell(f,p.x as f64,p.y as f64).is_some_and(|at|
        !matches!(f.tiles[at].as_str(),"wall"|"tree") && !crate::environment::blocks_sight(f,at)
        && (revision<51. || (!crate::borge::solid_at(f,p.x as f64,p.y as f64)
            && !crate::epidite::solid_at(f,p.x as f64,p.y as f64))))
}
fn bounds(r: &Run, x: f64, y: f64) -> bool {
    r.floor().is_some_and(|f| {
        x.is_finite()
            && y.is_finite()
            && x.fract() == 0.0
            && y.fract() == 0.0
            && x >= 0.0
            && y >= 0.0
            && x < (f.width() as f64)
            && y < (f.height() as f64)
    })
}
pub fn trace(r: &Run, tx: f64, ty: f64) -> Landing {
    trace_from_actor(r, &r.player, tx, ty)
}
pub fn trace_from(r: &Run, source_x: f64, source_y: f64, tx: f64, ty: f64) -> Landing {
    let origin=std::iter::once(&r.player).chain(r.actors_at(r.depth)).find(|a|a.x==source_x && a.y==source_y && !a.dead.value_or(false) && crate::dreamwalking::contact_plane(r,a));
    trace_plane(r,source_x,source_y,tx,ty,origin)
}
pub fn trace_from_actor(r:&Run,a:&crate::Actor,tx:f64,ty:f64)->Landing {trace_plane(r,a.x,a.y,tx,ty,Some(a))}
pub fn trace_committed(r:&mut Run,tx:f64,ty:f64)->Landing {let a=r.player.clone();trace_committed_from_actor(r,&a,tx,ty)}
/// Only launch sites call this after validation. Pure trace/hover/AI queries stop
/// at random forks; a launched ray consumes RNG only for forks it actually enters.
pub fn trace_committed_from_actor(r:&mut Run,a:&crate::Actor,tx:f64,ty:f64)->Landing {
 let mut points=Vec::new();
 let result=if r.simulation_revision.value_or(29.)<36.||!crate::dreamwalking::floor_for(r,a).is_some_and(|f|crate::non_euclidean_generation::has_links(f)) {
    trace_plane_impl(r,a.x,a.y,tx,ty,Some(a),None,Some(&mut points))
 } else {
    let mut state=crate::rng::to_u32(r.rng);
    let result=trace_plane_impl(r,a.x,a.y,tx,ty,Some(a),Some(&mut state),Some(&mut points));
    r.rng=state as f64;result
 };
 crate::events::committed_path(r,a,&points);result
}
fn trace_plane(r:&Run,source_x:f64,source_y:f64,tx:f64,ty:f64,origin:Option<&crate::Actor>)->Landing {
 trace_plane_impl(r,source_x,source_y,tx,ty,origin,None,None)
}
fn trace_plane_impl(r:&Run,source_x:f64,source_y:f64,tx:f64,ty:f64,origin:Option<&crate::Actor>,mut random:Option<&mut u32>,mut capture:Option<&mut Vec<crate::movement::Point>>)->Landing {
    let (mut x, mut y) = (source_x, source_y);
    if let Some(v)=capture.as_deref_mut(){v.push(crate::movement::Point{x:x as i32,y:y as i32});}
    let mut last = Landing {
        x,
        y,
        actor_id: None,
        distance: None,
    };
    if !bounds(r, x, y) || !bounds(r, tx, ty) {
        return last;
    }
    let f = origin.and_then(|a|crate::dreamwalking::floor_for(r,a)).or_else(||r.floor()).unwrap();
    if crate::portals::enabled(r) {
        return trace_spatial(r,f,source_x,source_y,tx,ty,origin,capture,|options|random.as_deref_mut().map(|state|options[(crate::rng::next_u32(state) as f64/4294967296.*options.len() as f64).floor()as usize]));
    }
    let (dx, dy) = ((tx - x).abs(), (ty - y).abs());
    let (sx, sy) = (
        if x < tx { 1.0 } else { -1.0 },
        if y < ty { 1.0 } else { -1.0 },
    );
    let mut error = dx - dy;
    while x != tx || y != ty {
        let twice = 2.0 * error;
        if twice > -dy {
            error -= dy;
            x += sx
        }
        if twice < dx {
            error += dx;
            y += sy
        }
        if !terrain_pass(r.simulation_revision.value_or(29.),f,crate::movement::Point{x:x as i32,y:y as i32})
        {
            return last;
        }
        if let Some(v)=capture.as_deref_mut(){v.push(crate::movement::Point{x:x as i32,y:y as i32});}
        last = Landing {
            x,
            y,
            actor_id: None,
        distance: None,
        };
        if let Some(a) = std::iter::once(&r.player)
            .chain(r.actors_at(r.depth))
            .find(|a| !a.dead.value_or(false) && a.hp > 0.0 && a.x == x && a.y == y && origin.is_none_or(|o|crate::dreamwalking::same_plane(r,o,a)))
        {
            last.actor_id = Some(a.id);
            return last;
        }
    }
    last
}

fn trace_spatial(r:&Run,f:&crate::Floor,source_x:f64,source_y:f64,tx:f64,ty:f64,origin:Option<&crate::Actor>,capture:Option<&mut Vec<crate::movement::Point>>,mut choose:impl FnMut(&[usize])->Option<usize>)->Landing {
 let mut last=Landing{x:source_x,y:source_y,actor_id:None,distance:Some(0.)};

        let from=crate::movement::Point{x:source_x as i32,y:source_y as i32};
        let target=crate::movement::Point{x:tx as i32,y:ty as i32};
        let pass=|p:crate::movement::Point| terrain_pass(r.simulation_revision.value_or(29.),f,p);
        let occupant=|p:crate::movement::Point| std::iter::once(&r.player).chain(r.actors_at(r.depth)).find(|a|!a.dead.value_or(false)&&a.hp>0.&&origin.is_none_or(|o|a.id!=o.id&&crate::dreamwalking::same_plane(r,o,a))&&crate::portals::same_position(r,a,p.x as f64,p.y as f64));
        let plan=crate::portals::ray_plan(f,from,target,pass);
        let (mut ray_from,mut aim)=(plan.origin,plan.aim);
        if plan.origin_fork.len()>1 {
            let Some(at)=choose(&plan.origin_fork)else{last.distance=Some(0.);return last};
            ray_from=crate::portals::point(f,at);aim.x+=ray_from.x-plan.origin.x;aim.y+=ray_from.y-plan.origin.y;
        }
        let ray=crate::portals::directed_ray_trace(f,ray_from,aim,pass,|options| {
            if options.len()==1{return Some(options[0])}
            choose(options)
        },|p|occupant(p).is_some());
        if let Some(v)=capture{v.clear();v.extend(ray.points.iter().copied());}
        if let Some(p)=ray.points.last(){last=Landing{x:p.x as f64,y:p.y as f64,actor_id:if ray.points.len()>1||ray_from!=from{occupant(*p).map(|a|a.id)}else{None},distance:Some(ray.distance)};}
        return last;
}
/// Pure bounded NPC admission search over the fixed aim later launched. Existing
/// optical LOS is mandatory; no pocket-origin/exit visibility is granted. A
/// failed budget search only declines this AI shot, never changes ray physics.
/// At most256 branch-prefix traces, O(B*(N²V + VA(P+C))), O(B*C+V+N) scratch.
pub const POSSIBLE_SHOT_BUDGET:usize=256;
pub fn possible_shot_range(r:&Run,a:&crate::Actor,b:&crate::Actor,max_range:f64)->Option<f64>{
 let f=crate::dreamwalking::floor_for(r,a)?;
 if r.simulation_revision.value_or(29.)<36.||!crate::non_euclidean_generation::has_links(f) {
  let p=trace_from_actor(r,a,b.x,b.y);let range=traveled_distance(r,a,b.x,b.y,&p);return (reaches_actor(r,&p,b)&&range<=max_range).then_some(range)
 }
 if !crate::dreamwalking::same_plane(r,a,b)||!crate::vision::line_of_sight(r,a,b){return None}
 let mut queue=std::collections::VecDeque::from([Vec::<usize>::new()]);
 for _ in 0..POSSIBLE_SHOT_BUDGET {
  let Some(prefix)=queue.pop_front()else{break};let mut at=0;let mut fork=None;
  let p=trace_spatial(r,f,a.x,a.y,b.x,b.y,Some(a),None,|options|{
   if let Some(&choice)=prefix.get(at){at+=1;options.get(choice).copied()}else{fork=Some(options.len());None}
  });
  if p.actor_id==Some(b.id)&&p.distance.is_some_and(|d|d<=max_range){return p.distance}
  if let Some(count)=fork{for choice in 0..count{let mut next=prefix.clone();next.push(choice);queue.push_back(next);}}
 }
 None
}
pub fn is_bow(i: &Item) -> bool {
    i.kind == "weapon"
        && i.ranged_weapon
            .as_ref()
            .is_some_and(|s| matches!(s.as_str(), "bow" | "composite-bow"))
}
/// Live R52 accuracy policy for both bow types; older saves stay unchanged.
pub fn normal_bow_accuracy(r: &Run, item: &Item) -> bool {
    r.simulation_revision.value_or(29.) >= 52. && is_bow(item)
}
/// Pure shared throw preview/commit calculation. One effective-stat query and
/// constant arithmetic; R51 reduces the fixed component, not bonuses or throw scaling.
pub fn player_throw_base(r: &Run, item: &Item) -> f64 {
    player_throw_base_for_revision(item, crate::combat::effective_stats(r, &r.player).offense,
        r.simulation_revision.value_or(29.0))
}
/// Frozen legacy arithmetic for callers without a simulation revision.
pub fn player_throw_base_with_offense(item: &Item, offense: f64) -> f64 {
    player_throw_base_for_revision(item, offense, 50.0)
}
/// O(1), fixed component only; improvised nonweapon throws have no fixed component.
pub fn player_throw_base_for_revision(item: &Item, offense: f64, revision: f64) -> f64 {
    let one = item.kind == "weapon" && item.hands.value_or(0.0) == 1.0;
    let n = if one {
        crate::revision50_player_attack::fixed_damage_for_revision(revision, item.damage.value_or(0.0))
            + item.enchantment.value_or(0.0) * if crate::combat::raw_attack_damage_enabled(revision) { 0.5 } else { 1.0 }
            + offense * item.offense_multiplier.value_or(1.0) * if crate::combat::raw_attack_damage_enabled(revision) { 0.5 } else { 1.0 }
    } else {
        offense * if crate::combat::raw_attack_damage_enabled(revision) { 0.5 } else { 1.0 }
    };
    if !item.thrown.value_or(false) && one { n / 2.0 } else { n }
}
pub fn blocker(r: &Run, c: &Command) -> Option<String> {
    let Command::Shoot { id, x, y } = c else {
        return None;
    };
    let Some(bow) = r.inventory.iter().find(|i| i.id == *id && is_bow(i)) else {
        return Some("Choose a bow in your pack.".into());
    };
    if !crate::revision50_inventory::enabled(r) && (!crate::crafting::is_equipped(bow)
        || r.inventory.iter().any(|i| {
            i.id != bow.id
                && i.equipped
                    .as_ref()
                    .is_some_and(|s| matches!(s.as_str(), "hand1" | "hand2"))
        }))
    {
        return Some("Equip this bow in both hands before firing.".into());
    }
    if !bounds(r, *x, *y) {
        return Some("Choose a target on this floor.".into());
    }
    if !r
        .inventory
        .iter()
        .any(|i| crate::inventory::is_arrow(i) && crate::inventory::quantity(i) > 0.0)
    {
        return Some("You have no arrows.".into());
    }
    None
}
fn glyph(i: &Item) -> &str {
    match i.kind.as_str() {
        "ring" => "o",
        "heart" => ";",
        "weapon" => {
            if is_bow(i) {
                ")"
            } else {
                "{"
            }
        }
        "ammo" if crate::inventory::is_arrow(i) => "→",
        _ => &i.glyph,
    }
}
fn place(
    r: &mut Run,
    depth: f64,
    p: Landing,
    item: Item,
    blocked: bool,
) -> Result<(), EngineError> {
    crate::item_placement::place_ground_item(
        r,
        depth,
        GroundItem {
            x: p.x,
            y: p.y,
            item,
            auto_pickup_blocked: if blocked { true.into() } else { Field::Missing },
        },
        |r, d| crate::startup::ensure_floors(r, d as u32),
    )
    .map_err(EngineError)?;
    Ok(())
}
pub fn command(r: &mut Run, c: &Command) -> Result<Option<bool>, EngineError> {
    let (id, x, y, shoot) = match c {
        Command::Shoot { id, x, y } => (*id, *x, *y, true),
        Command::Throw { id, x, y } => (*id, *x, *y, false),
        _ => return Ok(None),
    };
    if !shoot && crate::revision50::enabled(r) {
        if let Some(message) = crate::equipment::change_blocker(r, c) {
            log(r, message, "player");
            return Ok(Some(false));
        }
    }
    if shoot {
        if let Some(message) = blocker(r, c) {
            log(r, message, "player");
            return Ok(Some(false));
        }
    }
    let Some(index) = r.inventory.iter().position(|i| i.id == id) else {
        return Ok(Some(false));
    };
    if !bounds(r, x, y) {
        return Ok(Some(false));
    }
    let item = r.inventory[index].clone();
    if item.kind=="potion" {
        let effect=item.effect.value_or_default();
        if effect.is_empty(){return Ok(Some(false))}
        if !crate::potions::supported(&effect){return Err(EngineError("This potion effect is not yet supported natively.".into()))}
    }
    let depth = r.depth;
    let landing = trace_committed(r, x, y);
    if item.kind == "potion" {
        return Ok(Some(crate::potions::apply(
            r, index, landing.x, landing.y, true,
        )?));
    }
    let display_name = crate::revision50_item_knowledge::display_name(r, &item);
    let range = traveled_distance(r,&r.player,x,y,&landing);
    let player = r.player.id;
    if shoot {
        let ammo = r
            .inventory
            .iter()
            .position(crate::inventory::is_arrow)
            .unwrap();
        let arrow = crate::inventory::take_one(r, ammo);
        log(
            r,
            format!("You fire an arrow from {}.", item.name),
            "player",
        );
        crate::events::actor_event(
            r,
            &r.player,
            "projectile",
            "Arrow",
            Some(crate::events::EventPoint {
                x: landing.x,
                y: landing.y,
            }),
            Some(glyph(&arrow)),
        );
        crate::events::projectile_item(r,arrow.id);
        let target = r
            .actors_at(depth)
            .iter()
            .find(|a| Some(a.id) == landing.actor_id && !a.dead.value_or(false) && a.hp > 0.0)
            .map(|a| a.id);
        let hit = if let Some(victim) = target {
            crate::engine::attack_bow_resolved(r, victim, &item, range)?
        } else {
            false
        };
        if !hit {
            place(r, depth, landing, arrow, true)?
        }
    } else {
        log(r, format!("You throw {}.", display_name), "player");
        crate::events::actor_event(
            r,
            &r.player,
            "projectile",
            &display_name,
            Some(crate::events::EventPoint {
                x: landing.x,
                y: landing.y,
            }),
            Some(glyph(&item)),
        );
        let base = player_throw_base(r, &item);
        let shard=crate::borge::weapon_enabled(r) && crate::borge::is_shard(&item);
        let target = r.actors_at(depth).iter()
            .find(|a| reaches_actor(r,&landing,a) && !a.dead.value_or(false))
            .map(|a| a.id);
        let hit=if let Some(victim)=target {
            if shard { crate::engine::attack_shard_resolved(r,victim,range)? }
            else { crate::engine::attack_ranged_resolved(r, player, victim, base, range, 1.0, false)? }
        } else {false};
        if shard && hit {crate::borge::consume_weapon(r,id)?; return Ok(Some(true));}
        // A missed unique Shard remains recoverable on this floor, including
        // when the projectile terminates over a pit. O(V + G) placement scan.
        let recovery=if shard {
            let f=crate::dreamwalking::floor_for(r,&r.player).ok_or_else(||EngineError("missing projectile floor".into()))?;
            let ground:Vec<_>=r.ground.get(&depth.to_string()).into_iter().flat_map(|g|g.iter())
                .filter(|g|crate::dreamwalking::item_in_plane(r,&g.item)).cloned().collect();
            crate::item_placement::ground_destination(f,&ground,landing.x,landing.y)
                .filter(|(fall,_,_)|!*fall)
                .or_else(||crate::item_placement::ground_destination(f,&[],landing.x,landing.y).filter(|(fall,_,_)|!*fall))
        } else {None};
        // Pathological all-gap floors have no recoverable surface: retain it
        // rather than silently deleting the unique weapon after a miss.
        if shard && recovery.is_none() {return Ok(Some(true));}
        let cap = crate::equipment::capture_before(r);
        let index = r
            .inventory
            .iter()
            .position(|i| i.id == id)
            .ok_or_else(|| EngineError("thrown item disappeared during attack".into()))?;
        let mut thrown = crate::inventory::take_one(r, index);
        crate::events::projectile_item(r,thrown.id);
        thrown.equipped = Field::Missing;
        if shard && crate::dreamwalking::active(r) {thrown.dream_ground=true.into();}
        if let Some((_,x,y))=recovery {
            r.ground_at_mut(depth).push(GroundItem{x,y,item:thrown,auto_pickup_blocked:Field::Missing});
            if let Some(f)=r.floor_mut(){f.dirty=true;}
        } else {place(r, depth, landing, thrown, false)?;}
        crate::equipment::settle_removed(r, cap)?;
    }
    Ok(Some(true))
}

pub fn reaches_actor(r:&Run,p:&Landing,a:&crate::Actor)->bool{if crate::portals::enabled(r){p.actor_id==Some(a.id)}else{p.x==a.x&&p.y==a.y}}
pub fn traveled_distance(r:&Run,a:&crate::Actor,tx:f64,ty:f64,landing:&Landing)->f64{
 if let Some(distance)=landing.distance{return distance}
 if !crate::portals::enabled(r){return(a.x-landing.x).hypot(a.y-landing.y)}
 let Some(f)=crate::dreamwalking::floor_for(r,a)else{return 0.};let path=crate::portals::aimed_ray(f,crate::movement::Point{x:a.x as i32,y:a.y as i32},crate::movement::Point{x:tx as i32,y:ty as i32},|p|crate::portals::cell(f,p.x as f64,p.y as f64).is_some_and(|at|!matches!(f.tiles[at].as_str(),"wall"|"tree")&&!crate::environment::blocks_sight(f,at)));
 let end=path.iter().position(|p|crate::portals::same_cell(f,p.x as f64,p.y as f64,landing.x,landing.y)).unwrap_or(path.len().saturating_sub(1));crate::portals::ray_distance(f,&path[..=end])
}

#[cfg(test)]
mod throw_base_preview_tests {
    use super::*;
    #[test]
    fn lifted_throw_base_preserves_frozen_arithmetic_and_input_state() {
        let mut run:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        run.inventory.clear();run.player.abilities.clear();run.player.active.clear();run.player.conditions.clear();run.player.stats.offense=20.;
        for revision in [42.,43.] {
            run.simulation_revision=revision.into();
            let before=serde_json::to_vec(&run).unwrap();
            for kind in ["weapon","tool"] { for hands in [0.,1.,2.] { for thrown in [false,true] { for damage in [0.,30.,-30.,f64::INFINITY] {
                let item=Item{kind:kind.into(),hands:hands.into(),thrown:thrown.into(),damage:damage.into(),enchantment:2.0.into(),offense_multiplier:1.5.into(),..Default::default()};
                // Independent literal copy of the previously inline calculation.
                let offense=crate::combat::effective_stats(&run,&run.player).offense;
                let one=item.kind=="weapon" && item.hands.value_or(0.0)==1.0;
                let n=if one {item.damage.value_or(0.0)+item.enchantment.value_or(0.0)+offense*item.offense_multiplier.value_or(1.0)}else{offense};
                let expected=if !item.thrown.value_or(false)&&one{n/2.0}else{n};
                assert_eq!(player_throw_base(&run,&item).to_bits(),expected.to_bits());
            }}}}
            assert_eq!(serde_json::to_vec(&run).unwrap(),before);
        }
    }
}
