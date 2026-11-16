//! Nest-only Queen: existing Saidribee species, paid casts, world-clock cooldown.
use crate::{Run, engine::{actor, actor_mut_trusted, condition}};
pub const COST: f64 = 15.0;
pub const COOLDOWN: f64 = 10.0;
pub const TRAIT: &str = "enemy:beast:saidribee-queen";
/// O(A + 8*H) time/O(A) occupancy scratch; H is existing creature hazard lookup.
/// No random draw, energy spend or cooldown mutation until a legal cast exists.
/// Birth admission adds O(d) clean, O(A*d + F + H) dirty registry work.
/// d is ID digits; A actors, F floors, H retained slot history.
pub fn summon(r: &mut Run, id: f64, target: f64) -> Result<bool, String> {
    if r.simulation_revision.value_or(29.0) < 33.0 { return Ok(false); }
    let a = actor(r,id).ok_or("Queen missing")?.clone();
    let b = actor(r,target).ok_or("Queen target missing")?.clone();
    if !a.abilities.iter().any(|s|s==TRAIT) || a.dead.value_or(false) || a.hp <= 0.0
        || condition(&a,"paralyzed") > 0.0 || crate::ai::sleeping(&a)
        || r.tick < condition(&a,"monsterSummonReadyAt") || a.energy < COST
        || !crate::monster_horn::permitted(r,&a,&b) { return Ok(false); }
    let f = r.floor().ok_or("Summoning floor missing")?;
    let occupied: std::collections::HashSet<_> = std::iter::once(&r.player).chain(r.actors_at(r.depth))
        .filter(|b| !b.dead.value_or(false) && b.hp > 0.0 && crate::dreamwalking::same_plane(r,&a,b))
        .map(|b| (b.x as i32,b.y as i32)).collect();
    let mut cells = vec![];
    for dy in -1..=1 { for dx in -1..=1 {
        if dx==0 && dy==0 {continue;}
        let (x,y)=(a.x+dx as f64,a.y+dy as f64);
        if x<0.0 || y<0.0 || x>=f.width() as f64 || y>=f.height() as f64
            || occupied.iter().any(|&(ox, oy)| crate::portals::same_cell(f, ox as f64, oy as f64, x, y)) || crate::engine::bulwark_blocks(r,&a,x,y)
            || !matches!(f.tiles[y as usize*f.width()+x as usize].as_str(),"floor"|"grass")
            || crate::environment::creature_hazard_at(r,&a,y as usize*f.width()+x as usize)
            {continue;}
        cells.push((x,y));
    }}
    if cells.is_empty() {return Ok(false);}
    crate::creature_index::check_next_creature_id(r)?;
    let player_energy=r.player.energy;
    if !crate::monster_horn::spell_spend(r,id,COST) {return Ok(r.player.energy<player_energy);}
    let (x,y)=cells[(crate::rng::random(r)*cells.len() as f64).floor() as usize];
    let kind=if crate::rng::random(r)<2.0/3.0 {"short-saidri"} else {"saidri-guardian"};
    let mut child=crate::population::create_actor_revision(kind,r.next_id,x,y,false,33.0)?;
    crate::revision50_combat::mark_new_caster(&mut child,kind,r.simulation_revision.value_or(29.));
    if r.simulation_revision.value_or(29.)>=50. {crate::revision50_creature_identity::apply_actor(&mut child);}
    r.next_id+=1.0;
    child.heartless=a.heartless.clone();
    child.state=if a.state=="ally" {"ally"} else {"wandering"}.into();
    child.conditions.insert("queenSummoned".into(),1.0);
    if matches!(r.simulation_revision.value_or(29.), 42. | 43. | 44. | 45. | 46. | 50. | 51. | 52.) {
        crate::balance42::apply_new_actor_kind(&mut child, kind, 42.);
        crate::balance42::apply_new_actor_kind(&mut child, &format!("queen:{kind}"), 42.);
    }
    crate::balance52::apply_new_actor_kind(&mut child, &format!("queen:{kind}"), r.depth, r.simulation_revision.value_or(29.));
    let child_id=child.id;
    r.actors_at_mut(r.depth).push(child);
    let ready=r.tick+COOLDOWN;
    actor_mut_trusted(r,id).unwrap().conditions.insert("monsterSummonReadyAt".into(),ready);
    r.floor_mut().unwrap().dirty=true;
    if crate::events::enabled(r) {
        if let Some(child)=actor(r,child_id) {crate::animation_enemy::mark(r,child,"E66","summon-honey",crate::events::Stage::Special);}
    } else {crate::events::actor_event(r,&a,"summon","Summon Saidribee",Some(crate::events::EventPoint{x,y}),None);}
    if crate::vision::visible_actor(r,&a) {
        crate::engine::log(r,format!("{} summons a Saidribee.",a.name),"enemy");
    }
    Ok(true)
}
