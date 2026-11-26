//! Revision 34: one player resource pool, a stationary physical endpoint, and a dream plane.
//! State is ordinary serialized Run data, so saves and temporal snapshots are atomic.
use crate::{Actor, Command, Field, Floor, Run, engine::{self, EngineError}};
use serde::{Deserialize, Serialize};
pub const WAKE_UP_ID: &str = "phantasmii-wake-up";
pub const DREAMWALK_ID: &str = "phantasmii-dreamwalk";
pub const STAIRS_MESSAGE: &str = "You cannot go up or down while Dreamwalking.";
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct State {
    #[serde(default)]
    pub floors: Vec<std::sync::Arc<Floor>>,
    #[serde(default)]
    pub context_dream: bool,
    #[serde(default)]
    pub resolving_environment: bool,
    pub unlocked: bool,
    pub active: bool,
    pub depth: f64,
    pub body_id: Option<f64>,
    pub anchor_x: f64,
    pub anchor_y: f64,
}
pub fn enabled(r: &Run) -> bool { r.simulation_revision.value_or(29.) >= 34. }
pub fn active(r: &Run) -> bool { enabled(r) && r.dreamwalking.as_ref().is_some_and(|s| s.active && s.depth==r.depth) }
pub fn unlocked(r: &Run) -> bool { enabled(r) && r.dreamwalking.as_ref().is_some_and(|s| s.unlocked) }
pub fn is_nightmare(a: &Actor) -> bool { a.conditions.get("phantasmiiNightmare")==Some(&1.) }
pub fn is_body(a: &Actor) -> bool { a.conditions.get("phantasmiiBody")==Some(&1.) }
pub fn dream_actor(r: &Run,a: &Actor)->bool { is_nightmare(a) || a.conditions.get("phantasmiiPlane")==Some(&1.) || (active(r) && a.id==r.player.id) }
pub fn dream_view(r:&Run)->bool { active(r) || crate::half_here::clone_id(r).and_then(|id|engine::actor(r,id)).is_some_and(|a|dream_actor(r,a)) }
pub fn same_plane(r:&Run,a:&Actor,b:&Actor)->bool { !enabled(r) || dream_actor(r,a)==dream_actor(r,b) }
pub fn physical(r:&Run,a:&Actor)->bool { !dream_actor(r,a) }
pub fn inert(r:&Run,a:&Actor)->bool { crate::ahvamitt::is_actor(a) || is_body(a) || (is_nightmare(a) && !dream_view(r)) }
pub fn owner(r:&Run,a:&Actor)->Option<f64> { (active(r) && is_body(a)).then_some(r.player.id) }
pub fn body(r:&Run)->Option<&Actor> { let id=r.dreamwalking.as_ref()?.body_id?; r.actors_at(r.depth).iter().find(|a|a.id==id) }
pub fn validate(r:&Run)->Result<(),String> {
    let Some(s)=r.dreamwalking.as_ref() else {return Ok(())};
    if !enabled(r) || !s.depth.is_finite() || !s.anchor_x.is_finite() || !s.anchor_y.is_finite() {return Err("Invalid Dreamwalking state".into());}
    if s.context_dream || s.resolving_environment {return Err("Transient Dreamwalking context cannot be saved".into());}
    if s.floors.len()>r.floors.len() {return Err("Too many dream floors".into());}
    let mut depths=std::collections::HashSet::new();
    for f in &s.floors {
        if f.tiles.iter().any(|t|!crate::storage::TILES.contains(&t.as_str())) {return Err("Invalid dream terrain".into());}
        let Some(real)=r.floors.iter().find(|real|real.depth==f.depth) else {return Err("Dream floor has no waking counterpart".into())};
        if f.lights.len()!=f.tiles.len() || f.seen.len()!=f.tiles.len() || !crate::pocket_biomes::validate(f) {return Err("Invalid dream floor arrays or biome metadata".into());}
        if !depths.insert(f.depth as u32) || f.width()!=real.width() || f.height()!=real.height() || f.tiles.len()!=real.tiles.len() {return Err("Invalid dream floor geometry".into());}
    }
    if s.active && (s.depth!=r.depth || body(r).is_none() || s.body_id==Some(r.player.id)) {return Err("Dreamwalking body missing or on another floor".into());}
    if !s.active && s.body_id.is_some() {return Err("Inactive Dreamwalking has a body endpoint".into());}
    Ok(())
}
/// O(A + P), one endpoint clone, no gameplay RNG. The anchor reserves ordinary occupancy.
/// Birth admission adds O(d) clean, O(A*d + F + H) dirty registry work.
fn begin(r:&mut Run)->Result<bool,EngineError> {
    if active(r) {return Ok(false)}
    if crate::portals::enabled(r) {
        let occupied=r.actors_at(r.depth).iter().any(|a|!a.dead.value_or(false)&&a.hp>0.&&dream_actor(r,a)&&floor_in_plane(r,true).or_else(||r.floor()).is_some_and(|f|crate::portals::same_cell(f,a.x,a.y,r.player.x,r.player.y)));
        if occupied {engine::log(r,"Something occupies your place in the dream world.","player");return Ok(false)}
    }
    crate::creature_index::check_next_creature_id(r).map_err(EngineError)?;
    let id=r.next_id; r.next_id+=1.;
    let mut b=r.player.clone(); b.id=id; b.name="Your sleeping body".into();
    b.state="ally".into(); b.heartless=true.into(); b.perfect=false.into();
    b.conditions.clear();
    b.conditions.insert("phantasmiiBody".into(),1.);
    b.conditions.insert("halfHereOwner".into(),r.player.id);
    b.active.clear();
    let s=State {unlocked:unlocked(r),active:true,depth:r.depth,body_id:Some(id),anchor_x:b.x,anchor_y:b.y, ..r.dreamwalking.as_ref().cloned().unwrap_or_default()};
    r.actors_at_mut(r.depth).push(b); r.dreamwalking=s.into();
    ensure_layer(r);
    engine::log(r,if unlocked(r) {"You fall into a deep sleep."} else {"You prick yourself, and fall into a deep sleep."},"player");
    if !unlocked(r) {engine::log(r,"Find three shards of the Phantasmii to complete the ritual.","player");}
    Ok(true)
}
pub fn apply_dagger(r:&mut Run,id:f64)->Result<bool,EngineError> {
    if !enabled(r) || !r.inventory.iter().any(|i|i.id==id && crate::phantasmii_generation::is_dagger(i)) {return Ok(false)}
    if active(r) {return Ok(false)}
    let at=r.floor().and_then(|f|f.godbeast_event.as_ref()).is_some_and(|e|e.kind=="Phantasmii" && e.altar.as_ref().is_some_and(|a|a.x==r.player.x && a.y==r.player.y));
    if !at {engine::log(r,"Stand on the Altar of the Phantasmii to apply this dagger.","player");return Ok(false)}
    // The shared P73 transition is emitted once by the command observer.
    begin(r)
}
pub fn wake(r:&mut Run)->bool {
    if !active(r) {return false}
    let s=r.dreamwalking.as_ref().unwrap().clone();
    r.player.x=s.anchor_x; r.player.y=s.anchor_y;
    r.actors_at_mut(r.depth).retain(|a|Some(a.id)!=s.body_id);
    let s=r.dreamwalking.as_mut().unwrap();s.active=false;s.body_id=None;
    crate::revision50::duel_actor_changed(r,r.player.id);
    crate::revision50::settle_duel_targets(r);
    engine::log(r,"You wake up.","player"); true
}
pub fn ability(r:&mut Run,id:&str)->Result<Option<bool>,EngineError> {
    if !enabled(r) || ![WAKE_UP_ID,DREAMWALK_ID].contains(&id) {return Ok(None)}
    if r.status!="playing" || r.player.hp<=0. || !r.offers.is_empty() || crate::debug_floor::preparing(r) || engine::condition(&r.player,"paralyzed")>0. {return Ok(Some(false))}
    if id==WAKE_UP_ID {return Ok(Some(!unlocked(r) && wake(r)))}
    if !unlocked(r) {return Ok(Some(false))}
    if active(r) {Ok(Some(wake(r)))} else {begin(r).map(Some)}
}
pub fn shards_collected(r:&mut Run) {
    if !active(r) || unlocked(r) {return}
    let count:f64=r.inventory.iter().filter(|i|crate::phantasmii_generation::is_dream_shard(i)).map(crate::inventory::quantity).sum();
    if count>=3. {r.dreamwalking.as_mut().unwrap().unlocked=true;crate::events::inventory(r,"E83","unlock-diamond",None);engine::log(r,"The ritual is complete. You can now Dreamwalk whenever you wish.","player");}
}
/// O(A+P) at resource synchronization boundaries. Conditions remain body-local.
pub fn sync(r:&mut Run) {
    if !active(r) {return}
    let Some(id)=r.dreamwalking.as_ref().and_then(|s|s.body_id) else{return};
    let p=r.player.clone();
    if let Some(b)=engine::actor_mut_trusted(r,id) {
        b.hp=p.hp;b.max_hp=p.max_hp;b.energy=p.energy;b.stats=p.stats.clone();
        b.abilities=p.abilities.clone();b.active.clear();b.base_max_hp=p.base_max_hp.clone();
    }
}
pub fn damage_notice(r:&mut Run,a:&Actor,n:f64) {
    if active(r) && is_body(a) && n>0. {engine::log(r,format!("Your sleeping body takes {n} damage!"),"enemy");}
}
/// Explicit ground ownership for the unique weapon, not carried-item state.
pub fn dream_ground_item(item:&crate::Item)->bool {
    crate::borge::is_shard(item) && item.dream_ground.value_or(false)
}
/// Special ground items that waking-world terrain and item placement must ignore.
pub(crate) fn protected_ground(item: &crate::Item, dream_shards_only: bool) -> bool {
    dream_ground_item(item) || (dream_shards_only && crate::phantasmii_generation::is_shard(item))
}
/// Map, inspector and ground-list visibility use one rule. A dream clone or
/// memory cannot reveal a Phantasmii shard to a player who is currently awake.
pub fn item_visible(r: &Run, item: &crate::Item) -> bool {
    if crate::phantasmii_blessing_rules::dream_shards_only(r) && crate::phantasmii_generation::is_shard(item) {
        return active(r);
    }
    !(crate::phantasmii_generation::is_dream_shard(item) || dream_ground_item(item)) || dream_view(r)
}
pub fn dream_item(r: &Run, item: &crate::Item) -> bool {
    crate::phantasmii_generation::is_dream_shard(item)
        || protected_ground(item, crate::phantasmii_blessing_rules::dream_shards_only(r))
}
/// Whether the player can collect this item in their currently active plane.
pub fn item_in_plane(r:&Run,item:&crate::Item)->bool {
    let dream=crate::phantasmii_generation::is_dream_shard(item)
        || (crate::phantasmii_blessing_rules::dream_shards_only(r) && crate::phantasmii_generation::is_floor_shard(item))
        || (crate::borge::weapon_enabled(r) && dream_ground_item(item));
    dream==active(r)
}
pub fn blocker(r:&Run,c:&Command)->Option<&'static str> {
    if !active(r) {return None}
    match c {
        Command::Descend|Command::Ascend=>Some(STAIRS_MESSAGE),
        Command::Throw{id,..} if crate::borge::weapon_enabled(r)
            && r.inventory.iter().any(|i|i.id==*id && crate::borge::is_shard(i))=>None,
        Command::Use{..}|Command::UseAt{..}|Command::Throw{..}|Command::Craft{..}=>Some("You cannot use consumable items while Dreamwalking."),
        Command::Drop{..}|Command::Interact{..}|Command::Rally{..}|Command::Ally{..}=>Some("You cannot interact with the waking world while Dreamwalking."),
        _=>None,
    }
}

/// O(V) first visit; subsequent context switches are two Arc swaps, O(F) lookup.
/// Dream environment is separate, not a duplicated inventory/actor/reward world.
pub fn ensure_layer(r:&mut Run) {
    if !enabled(r) {return}
    let d=r.depth;
    if r.dreamwalking.as_ref().is_some_and(|s|s.floors.iter().any(|f|f.depth==d)) {return}
    let Some(mut f)=r.floor().cloned() else{return};
    f.environment=Field::Missing;f.vents=Field::Missing;f.permanent_fire=Field::Missing;
    f.fungal_roots=Field::Missing;f.roots=Field::Missing;f.razorleaves=Field::Missing;
    f.blood=Field::Missing;f.water_blood=Field::Missing;f.kelpie_blood_sources=Field::Missing;
    f.remembered_hazards=Field::Missing;
    r.dreamwalking.get_or_insert_with(State::default).floors.push(std::sync::Arc::new(f));
}
pub fn context_dream(r:&Run)->bool {enabled(r)&&r.dreamwalking.as_ref().is_some_and(|s|s.context_dream)}
pub fn environment_context(r:&Run)->bool {enabled(r)&&r.dreamwalking.as_ref().is_some_and(|s|s.resolving_environment)}
pub fn contact_plane(r:&Run,a:&Actor)->bool {!enabled(r)||dream_actor(r,a)==context_dream(r)}
pub fn floor_for<'a>(r:&'a Run,a:&Actor)->Option<&'a Floor> {floor_in_plane(r,dream_actor(r,a))}
pub fn floor_in_plane(r:&Run,dream:bool)->Option<&Floor> {
    if !enabled(r) || dream==context_dream(r) {return r.floor()}
    r.dreamwalking.as_ref()?.floors.iter().find(|f|f.depth==r.depth).map(|f|f.as_ref())
}
pub fn with_plane<T>(r:&mut Run,dream:bool,f:impl FnOnce(&mut Run)->T)->T {
    if !enabled(r) || context_dream(r)==dream {return f(r)}
    ensure_layer(r);
    let depth=r.depth;
    let i=r.floors.iter().position(|f|f.depth==depth).unwrap();
    let j=r.dreamwalking.as_ref().unwrap().floors.iter().position(|f|f.depth==depth).unwrap();
    {let s=r.dreamwalking.as_mut().unwrap();std::mem::swap(&mut r.floors[i],&mut s.floors[j]);s.context_dream=dream;}
    let result=f(r);
    {let s=r.dreamwalking.as_mut().unwrap();std::mem::swap(&mut r.floors[i],&mut s.floors[j]);s.context_dream=!dream;}
    result
}
pub fn with_actor<T>(r:&mut Run,id:f64,f:impl FnOnce(&mut Run)->T)->T {
    let dream=engine::actor(r,id).is_some_and(|a|dream_actor(r,a));with_plane(r,dream,f)
}
pub fn observation_run(r:&Run)->Run {
    let mut out=r.clone();
    if active(r) && !context_dream(r) {ensure_layer(&mut out);let d=out.depth;
        let i=out.floors.iter().position(|f|f.depth==d).unwrap();
        let j=out.dreamwalking.as_ref().unwrap().floors.iter().position(|f|f.depth==d).unwrap();
        let s=out.dreamwalking.as_mut().unwrap();std::mem::swap(&mut out.floors[i],&mut s.floors[j]);s.context_dream=true;
    }out
}

pub fn tick_environments(r:&mut Run,pending:&[f64])->Result<(),String> {
    if !enabled(r) {return crate::environment::tick_environment_options_with_falls(r,None,pending)}
    r.dreamwalking.get_or_insert_with(State::default).resolving_environment=true;
    let normal=with_plane(r,false,|r|crate::environment::tick_environment_options_with_falls(r,None,pending));
    if normal.is_err() {r.dreamwalking.as_mut().unwrap().resolving_environment=false;return normal;}
    let dream=r.dreamwalking.as_ref().unwrap().floors.iter().any(|f|f.depth==r.depth);
    let result=if dream {with_plane(r,true,|r|crate::environment::tick_environment_options_with_falls(r,None,&[]))} else {Ok(())};
    r.dreamwalking.as_mut().unwrap().resolving_environment=false;result
}
