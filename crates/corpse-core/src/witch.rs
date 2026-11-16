//! Revision33 Witch summons use the same successful-cast-only world clock as Birthers.
use crate::{Run, engine::{actor,actor_mut_trusted,condition}};
/// Eight adjacent candidates, O(A) occupancy preprocessing and O(8) lookup. No
/// random draw, energy spend or cooldown mutation occurs until a cast can succeed.
/// Birth admission adds O(d) clean, O(A*d + F + H) dirty registry work.
/// d is ID digits; A actors, F floors, H retained slot history.
pub fn summon(r:&mut Run,id:f64,target:f64)->Result<bool,String> {
    if r.simulation_revision.value_or(29.)<33. { return Ok(false); }
    let a=actor(r,id).ok_or("Witch missing")?.clone();
    let b=actor(r,target).ok_or("Witch target missing")?.clone();
    if !a.abilities.iter().any(|s|s=="enemy:beast:witch")
        || r.tick<condition(&a,"monsterSummonReadyAt") || a.energy<10.
        || !crate::monster_horn::permitted(r,&a,&b) { return Ok(false); }
    let f=r.floor().ok_or("Summoning floor missing")?;
    let occupied:std::collections::HashSet<_>=std::iter::once(&r.player).chain(r.actors_at(r.depth))
        .filter(|b|!b.dead.value_or(false)&&b.hp>0. && crate::dreamwalking::same_plane(r,&a,b)).map(|b|(b.x as i32,b.y as i32)).collect();
    let mut cells=Vec::new();
    for dy in -1..=1 { for dx in -1..=1 {
        if dx==0 && dy==0 {continue;}
        let (x,y)=(a.x+dx as f64,a.y+dy as f64);
        if x<0. || y<0. || x>=f.width() as f64 || y>=f.height() as f64
            || occupied.iter().any(|&(ox, oy)| crate::portals::same_cell(f, ox as f64, oy as f64, x, y)) || crate::engine::bulwark_blocks(r,&a,x,y)
            || (r.simulation_revision.value_or(29.)>=51. && a.state!="ally"
                && crate::revision51_stairs::stair_cell(f,y as usize*f.width()+x as usize))
            || !matches!(f.tiles[y as usize*f.width()+x as usize].as_str(),"floor"|"grass"|"up"|"down") {continue;}
        cells.push((x,y));
    }}
    if cells.is_empty() {return Ok(false);}
    crate::creature_index::check_next_creature_id(r)?;
    let player_energy=r.player.energy;
    if !crate::monster_horn::spell_spend(r,id,10.) {
        // Uncast pays to cancel this attempted spell; it consumes this action,
        // not a successful-cast cooldown, and must not provoke a second curse.
        return Ok(r.player.energy<player_energy);
    }
    let (x,y)=cells[(crate::rng::random(r)*cells.len() as f64).floor() as usize];
    let kind=["night-crawler","skin-walker"][(crate::rng::random(r)*2.).floor() as usize];
    let mut child=crate::population::create_actor_revision(kind,r.next_id,x,y,false,33.)?;
    crate::revision50_combat::mark_new_caster(&mut child,kind,r.simulation_revision.value_or(29.));
    r.next_id+=1.;
    child.heartless=a.heartless.clone();
    child.state=if a.state=="ally" {"ally"} else {"wandering"}.into();
    child.conditions.insert("witchSummoned".into(),1.);
    // Birth-only payload is lighter than a naturally encountered adult; never retune saved actors.
    child.stats.offense=200.;child.stats.defense=24.;child.stats.accuracy=140.;child.stats.dodge=30.;
    child.hp=244.;child.max_hp=244.;
    crate::accuracy_dodge::apply_witch_newborn(&mut child, kind, 33.);
    if matches!(r.simulation_revision.value_or(29.), 42. | 43. | 44. | 45. | 46. | 50. | 51. | 52.) {
        crate::balance42::apply_new_actor_kind(&mut child, &format!("witch:{kind}"), 42.);
    }
    crate::balance52::apply_new_actor_kind(&mut child, &format!("witch:{kind}"), r.depth, r.simulation_revision.value_or(29.));
    let child_id = child.id;
    r.actors_at_mut(r.depth).push(child);
    let ready=r.tick+10.;
    actor_mut_trusted(r,id).unwrap().conditions.insert("monsterSummonReadyAt".into(),ready);
    r.floor_mut().unwrap().dirty=true;
    // Preserve visibility/disguise admission: a witnessed summon is named by
    // its current appearance, never by a hidden true Skin Walker identity.
    if crate::events::enabled(r) {
        if let Some(child)=actor(r,child_id) {crate::animation_enemy::mark(r,child,"E08","summon-violet",crate::events::Stage::Special);}
    } else {crate::events::actor_event(r,&a,"summon","Summon",Some(crate::events::EventPoint{x,y}),None);}
    if crate::vision::visible_actor(r,&a) {
        let message = if r.simulation_revision.value_or(29.) >= 43. {
            let caster = definite_name(crate::actor_presentation::name(r,&a));
            let newborn = actor(r,child_id).filter(|b|crate::vision::visible_actor(r,b));
            let target = newborn.map(|b|indefinite_name(crate::actor_presentation::name(r,b)))
                .unwrap_or_else(||"a creature".into());
            format!("{caster} summons {target}.")
        } else { format!("{} summons a creature.",crate::actor_presentation::name(r,&a)) };
        crate::engine::log(r,message,"enemy");
    }
    Ok(true)
}

/// O(name bytes), presentation only; do not duplicate an existing article.
fn definite_name(name:&str)->String {
    if name.get(..4).is_some_and(|s|s.eq_ignore_ascii_case("the ")) { name.into() }
    else {format!("The {name}")}
}
fn indefinite_name(name:&str)->String {
    if ["a ","an ","the "].iter().any(|p|name.get(..p.len()).is_some_and(|s|s.eq_ignore_ascii_case(p))) {return name.into();}
    let article=if name.chars().next().is_some_and(|c|"aeiouAEIOU".contains(c)){"an"}else{"a"};
    format!("{article} {name}")
}

#[cfg(test)] mod wording_tests {
    #[test] fn names_keep_one_article() {
        assert_eq!(super::definite_name("Witch"),"The Witch");
        assert_eq!(super::definite_name("The Witch"),"The Witch");
        assert_eq!(super::indefinite_name("Night Crawler"),"a Night Crawler");
        assert_eq!(super::indefinite_name("an Echo"),"an Echo");
        assert_eq!(super::indefinite_name("The Echo"),"The Echo");
    }
}
