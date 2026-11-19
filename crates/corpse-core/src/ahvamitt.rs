//! The stationary trickster's three-hit search and one carried teleport mask.
use crate::{Actor, Floor, GroundItem, Item, Run, engine::{self, EngineError}};
pub const MASK_NAME: &str = "Mask of Trickery";
pub const MASQUE_NAME: &str = "Masque of the Trickster";
pub const TELEPORT_ID: &str = "ahvamitt-teleport";
pub const DESCRIPTION: &str = "A Godbeast of trickery.";
pub const COOLDOWN: f64 = 100.;
pub const FIRST: &str = "You hear a voice in your head. 'Humor me won't you? Find me twice more.'";
pub const SECOND: &str = "Oh how I delight in this game! Find me again.";
pub const THIRD: &str = "Oh, what a wonderful show. Here, you may have my face.";
pub fn is_actor(a: &Actor) -> bool { a.conditions.get("ahvamitt") == Some(&1.) }
pub fn is_mask(i: &Item) -> bool {
    i.kind == "tool" && matches!(i.name.as_str(), MASK_NAME | "Mask of the Trickster" | MASQUE_NAME)
}
pub fn has_mask(r: &Run) -> bool { r.inventory.iter().any(is_mask) }
pub fn cooldown(r: &Run) -> u32 { (r.player.conditions.get("ahvamittTeleportReadyAt").copied().unwrap_or(0.) - r.tick).max(0.).ceil() as u32 }
pub fn mask(id: f64) -> Item {
    Item { id, name:MASK_NAME.into(),kind:"tool".into(),glyph:"]".into(),identified:true.into(),
        effect:"Grants Teleport while in your inventory.".to_string().into(),..Default::default() }
}
/// R50 creation uses the new name; identity accepts both historical spellings.
pub fn mask_for_revision(id: f64, revision: f64) -> Item {
    let mut item = mask(id);
    if revision >= 50.0 { item.name = MASQUE_NAME.into(); }
    item
}
pub fn actor(id:f64,x:f64,y:f64) -> Actor {
    let mut a=Actor {id,x,y,name:"Ahvamitt".into(),creature_kind:"ahvamitt".to_string().into(),glyph:"A".into(),
        color:"#d8bf75".into(),hp:100.,max_hp:100.,energy:100.,state:"neutral".into(),heartless:true.into(),
        stats:crate::Stats{perception:100.,max_energy:100.,..Default::default()},..Default::default()};
    a.conditions.insert("ahvamitt".into(),1.);a.conditions.insert("ahvamittFinds".into(),0.);
    a.conditions.insert("bestiaryTraitsInitialized".into(),1.);a
}
pub fn actor_revision(id:f64,x:f64,y:f64,revision:f64) -> Actor {
    let mut a=actor(id,x,y);
    a.stats.accuracy=crate::combat::creature_birth_accuracy(revision,a.stats.accuracy);
    a.stats.dodge=crate::combat::creature_birth_dodge(revision,a.stats.dodge);
    a
}
pub fn populate(f:&Floor,actors:&mut Vec<Actor>,next_id:&mut f64) {
    populate_revision(f,actors,next_id,34.);
}
pub fn populate_revision(f:&Floor,actors:&mut Vec<Actor>,next_id:&mut f64,revision:f64) {
    if let Some(e)=f.godbeast_event.as_ref().filter(|e|e.kind=="Ahvamitt" && !e.reward_created.value_or(false)) {
        actors.push(actor_revision(*next_id,e.x,e.y,revision));*next_id+=1.;
    }
}
/// O(V*(A+B)) filtering plus O(V) BFS, A current actors, B bounded active traits;
/// O(V) scratch. The RNG draw is made only after a legal destination is known.
/// Required reachability comes from the connected stair component; final checks
/// honor the teleported body's current plane, passability and standing habitat.
fn destination(r:&mut Run,a:&Actor)->Option<(f64,f64)> {
    let f=r.floor()?;let w=f.width();let reachable=crate::generation::reachable_without_braziers(f,None);
    let choices:Vec<_>=(0..f.tiles.len()).filter(|at|reachable.contains(at)
        && !["wall","tree","chasm","pit"].contains(&f.tiles[*at].as_str()))
        .map(|at|((at%w) as f64,(at/w) as f64))
        .filter(|&(x,y)|!crate::portals::same_position(r,a,x,y) && engine::actor_passable(r,a,x,y)
            && crate::travel::habitat_valid(f,a,x,y)
            && !std::iter::once(&r.player).chain(r.actors_at(r.depth)).any(|b|b.id!=a.id
                && !b.dead.value_or(false) && b.hp>0. && crate::portals::same_position(r,b,x,y) && crate::dreamwalking::same_plane(r,a,b)))
        .collect();
    if choices.is_empty() {return None}
    Some(choices[(crate::rng::random(r)*choices.len() as f64).floor() as usize])
}
pub fn attack(r:&mut Run,id:f64)->Result<bool,EngineError> {
    let Some(a)=engine::actor(r,id).filter(|a|is_actor(a)).cloned() else{return Ok(false)};
    if r.status!="playing" || r.player.hp<=0. || !crate::dreamwalking::same_plane(r,&r.player,&a) { return Ok(false); }
    let finds=a.conditions.get("ahvamittFinds").copied().unwrap_or(0.) as u32;
    if finds>=3 {return Ok(false)}
    let at=if finds<2 {destination(r,&a)} else {None};
    engine::actor_mut_trusted(r,id).unwrap().conditions.insert("ahvamittFinds".into(),(finds+1) as f64);
    engine::log(r,[FIRST,SECOND,THIRD][finds as usize],"environment");
    if finds==2 {
        crate::animation_enemy::mark(r, &a, "E76", "mask-fold", crate::events::Stage::Special);
        let item=mask_for_revision(r.next_id,r.simulation_revision.value_or(29.));r.next_id+=1.;
        crate::item_placement::place_ground_item(r,r.depth,GroundItem{x:a.x,y:a.y,item,..Default::default()},
            |r,d|crate::startup::ensure_floors(r,d as u32)).map_err(EngineError)?;
        r.actors_at_mut(r.depth).retain(|a|a.id!=id);
        crate::revision50::end_duels_with_target(r,id);
        if let Some(e)=r.floor_mut().and_then(|f|f.godbeast_event.as_mut()).filter(|e|e.kind=="Ahvamitt") { e.reward_created=true.into(); }
    }
    if let Some((x,y))=at {
        crate::animation_shapes::nova(r,&a,"E75","nova-rainbow-small",2);
        let a=engine::actor_mut_trusted(r,id).unwrap();a.x=x;a.y=y;
        let a=a.clone(); crate::revision50::duel_actor_changed(r,id); crate::animation_shapes::nova(r,&a,"E75","nova-rainbow-small",2);
    } else if finds<2 { crate::animation_enemy::mark(r, &a, "E75", "diamond-gold", crate::events::Stage::Special); }
    r.floor_mut().unwrap().dirty=true;
    Ok(true)
}
pub fn teleport(r:&mut Run)->Result<bool,EngineError> {
    if r.simulation_revision.value_or(29.)<34. || !has_mask(r) || cooldown(r)>0 || r.status!="playing" || r.player.hp<=0. || !r.offers.is_empty()
        || crate::debug_floor::preparing(r) || engine::condition(&r.player,"paralyzed")>0. {return Ok(false)}
    let id=r.player.id;
    crate::dreamwalking::with_actor(r,id,|r| {
        let a=r.player.clone();
        let Some((x,y))=destination(r,&a) else{return Ok(false)};
        r.player.x=x;r.player.y=y;
        crate::revision50::duel_actor_changed(r,id);
        r.player.conditions.insert("ahvamittTeleportReadyAt".into(),r.tick+COOLDOWN);
        crate::half_here::sync(r);
        crate::zillennia_ground::on_arrival(r,id);
        crate::environment::refresh_burning_from_fire(r,id).map_err(EngineError)?;
        engine::log(r,if crate::revision50::enabled(r) {"The Masque of the Trickster whisks you away."} else {"The Mask of Trickery whisks you away."},"player");
        if crate::events::enabled(r) {
            if (a.x,a.y)!=(r.player.x,r.player.y) {crate::animation_shapes::short_nova(r,&r.player,"P74",crate::animation_shapes::ISHEILAH);}
        } else {crate::events::actor_event(r,&r.player,"pulse","Teleport",None,None);}
        Ok(true)
    })
}
