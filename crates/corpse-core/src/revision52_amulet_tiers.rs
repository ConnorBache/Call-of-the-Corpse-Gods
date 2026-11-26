//! R52 immutable birth tiers. Mechanics and presentation share these profiles.
//! No RNG, energy initialization, actor scaling, or loaded-item depth inference.
use crate::{Actor, Field, Item, Run};

pub const ROOTS_DAMAGE: &str = "r52RootsDamage";
pub const MAX_TIER: u8 = 9;
// Separately fitted packets are centralized here; tier one is authored R50.
// Columns: Lightning, Firebolt, Acid, Iceblast, Floods, Poison/tick, Roots/tick.
pub const DAMAGE: [[f64; 7]; 9] = [
    [50.,80.,40.,30.,20.,6.,3.],
    [54.,81.,51.,45.,35.,14.,11.],
    [56.,82.,54.,46.,42.,15.,12.],
    [57.,83.,55.,49.,43.,17.,17.],
    [71.,84.,67.,67.,61.,24.,22.],
    [72.,85.,68.,68.,62.,25.,23.],
    [73.,86.,69.,69.,63.,26.,24.],
    [87.,92.,87.,82.,77.,27.,25.],
    [93.,93.,93.,88.,84.,28.,26.],
];
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Profile {
    pub tier: u8,
    pub direct_damage: f64,
    pub periodic_damage: f64,
    pub range: Option<f64>,
    pub duration: Option<f64>,
}
pub fn enabled(r: &Run) -> bool { r.simulation_revision.value_or(29.) == 52. }
/// Retired powers remain readable in old saves, never usable in current runs.
pub fn retired(r: &Run, item: &Item) -> bool {
    crate::amulet_rules::current(r) && item.kind == "amulet" && matches!(item.effect.as_ref().map(String::as_str), Some("acid" | "floods"))
}
pub const RETIRED_MESSAGE: &str = "This amulet has been retired.";
pub fn tier_for_depth(depth: u32) -> u8 { (depth.clamp(1,40) / 5 + 1) as u8 }
pub fn valid_item(item: &Item, revision: f64) -> bool {
    match item.amulet_tier {
        Field::Missing => true,
        Field::Null => false,
        Field::Value(n) => revision == 52. && crate::revision50_amulet_effects::is_amulet(item)
            && n.is_finite() && n.fract() == 0. && (1. ..=9.).contains(&n),
    }
}
/// Birth-only O(effect/name bytes). Repeated calls cannot upgrade/refill an item.
/// Unknown-depth legacy constructors deliberately leave tier missing (base one).
pub fn initialize_birth(item: &mut Item, revision: f64, depth: u32) -> Result<(), String> {
    if !valid_item(item, revision) { return Err("Invalid amulet tier".into()); }
    if revision != 52. || !crate::revision50_amulet_effects::is_amulet(item) { return Ok(()); }
    if item.amulet_tier.is_missing() {
        if !(1..=40).contains(&depth) { return Err("Invalid amulet birth floor".into()); }
        assign_birth(item, tier_for_depth(depth));
    }
    Ok(())
}
/// Authored special reward. Never retier existing inventory or floor items.
pub(crate) fn initialize_max_birth(item: &mut Item, revision: f64) -> Result<(), String> {
    if revision != 52. || !valid_item(item, revision) || !crate::revision50_amulet_effects::is_amulet(item) {
        return Err("Maximum tier reward requires an R52 amulet".into());
    }
    if item.amulet_tier.is_missing() { assign_birth(item, MAX_TIER); }
    Ok(())
}
fn assign_birth(item: &mut Item, tier: u8) {
    item.amulet_tier = (tier as f64).into();
    item.name = format!("Tier {tier} {}", crate::revision50_amulet_effects::display_name(item).unwrap());
}
/// O(effect bytes), no allocation. Missing tier is always authored base one.
pub fn profile(r: &Run, item: &Item) -> Option<Profile> {
    if !valid_item(item, r.simulation_revision.value_or(29.)) || !crate::revision50_amulet_effects::is_amulet(item) { return None; }
    let tier = if enabled(r) { item.amulet_tier.value_or(1.) as u8 } else { 1 };
    let d = DAMAGE[(tier-1) as usize]; let extra = (tier-1) as f64;
    let mut p = Profile { tier, direct_damage:0., periodic_damage:0., range:None, duration:None };
    match item.effect.as_ref()?.as_str() {
        "lightning" => p.direct_damage=d[0],
        "firebolt" => p.direct_damage=d[1],
        "acid" => { p.direct_damage=d[2];p.range=Some(8.);p.duration=Some(6.); },
        "iceblast" => { p.direct_damage=d[3];p.duration=Some(3.); },
        "floods" => { p.direct_damage=d[4];p.duration=Some(12.); },
        "poison" => { p.periodic_damage=d[5];p.duration=Some(20.); },
        "roots" => { p.periodic_damage=d[6];p.range=Some(6.);p.duration=Some(4.); },
        "blinking" => p.range=Some(10.+extra),
        "beckoning" | "teleportation" => p.range=Some(8.+extra),
        "discord" => { p.range=Some(8.);p.duration=Some(6.+extra); },
        "obstruction" => p.duration=Some(8.+extra),
        _ => return None,
    }
    if crate::amulet_rules::current(r) {
        match item.effect.as_ref()?.as_str() {
            "beckoning" | "discord" => p.range=None,
            "roots" => {p.periodic_damage=0.;p.duration=Some(8.);},
            _ => {},
        }
    }
    Some(p)
}
pub fn display_name(r:&Run,item:&Item)->Option<String> {
    let name=crate::revision50_amulet_effects::display_name(item)?;
    if retired(r,item) {Some(format!("Retired Tier {} {name}",profile(r,item)?.tier))}
    else if enabled(r) {Some(format!("Tier {} {name}",profile(r,item)?.tier))} else {Some(name.into())}
}
pub fn description(r:&Run,item:&Item)->Option<String> {
    if retired(r,item) {return Some(RETIRED_MESSAGE.into());}
    if !enabled(r) {return crate::revision50_amulet_effects::description(item).map(str::to_owned);}
    let p=profile(r,item)?;
    Some(match item.effect.as_ref()?.as_str() {
        "lightning"=>format!("Fires a piercing bolt for {} lightning damage.",p.direct_damage),
        "firebolt"=>format!("Fires a bolt for {} fire damage, igniting creatures and flammable terrain.",p.direct_damage),
        "poison"=>format!("Fires a projectile that poisons its target for {} damage per normal turn for 20 turns.",p.periodic_damage),
        "iceblast"=>format!("Deals {} cold damage and slows creatures for 3 turns in a 3×3 area, leaving ice.",p.direct_damage),
        "acid"=>format!("Fires a projectile that deals {} damage and reduces the target’s DR by 10 for 6 turns.",p.direct_damage),
        "blinking"=>format!("Blink along a {} tile line.",p.range?),
        "discord"=>format!("Makes a creature and all other creatures hostile to each other for {} turns.",p.duration?),
        "obstruction"=>format!("Creates a three-tile wall perpendicular to your aim for {} turns.",p.duration?),
        "roots"=>if crate::amulet_rules::current(r) {"Roots a creature in place for 8 turns.".into()} else {format!("Roots a creature in place for 4 turns, dealing {} damage each normal turn.",p.periodic_damage)},
        "floods"=>format!("Sends a 3×5 wave that deals {} damage, pushes creatures up to 2 tiles, extinguishes fire, and leaves shallow water for 12 turns. Existing shallow water consumes each lane and becomes deep water for 12 turns.",p.direct_damage),
        _=>crate::revision50_amulet_effects::description(item)?.into(),
    })
}
pub fn details(r:&Run,item:&Item)->Option<String> {
    if retired(r,item) {return None;}
    if !enabled(r) {return crate::revision50_amulet_effects::details_for_run(r,item).map(str::to_owned);}
    let p=profile(r,item)?;
    Some(match item.effect.as_ref()?.as_str() {
        "blinking"=>return None,
        "beckoning"=>if crate::amulet_rules::current(r) {"Targets a visible creature at unlimited range with a clear path to you.".into()} else {format!("Targets a visible creature within {} tiles with a clear path to you.",p.range?)},
        "discord" if crate::amulet_rules::current(r)=>"Targets a visible creature at unlimited range.".into(),
        "teleportation"=>format!("Targets a visible creature within {} tiles.",p.range?),
        _=>crate::revision50_amulet_effects::details_for_run(r,item)?.into(),
    })
}
pub fn valid_condition(key:&str,value:f64,revision:f64)->bool {
    if key==ROOTS_DAMAGE {return revision==52.&&DAMAGE.iter().any(|d|d[6]==value);}
    if revision==52. && key==crate::revision50_control_amulets::DISCORD {return value.is_finite()&&value.fract()==0.&&(0. ..=14.).contains(&value);}
    if key==crate::revision50_amulet_effects::POISON_DAMAGE {
        return if revision==52. {DAMAGE.iter().any(|d|d[5]==value)} else {value==6.};
    }
    true
}
fn actor_valid(a:&Actor,revision:f64)->bool {
    a.copied_inventory.as_ref().is_none_or(|v|v.iter().all(|i|valid_item(i,revision)))
        && a.conditions.iter().all(|(k,&v)|valid_condition(k,v,revision))
        && a.conditions.get(ROOTS_DAMAGE).is_none_or(|_|crate::engine::condition(a,crate::revision50_control_amulets::ROOTS)>0.)
        && a.monster_state.as_ref().is_none_or(|m|
            m.carried_items.as_ref().is_none_or(|v|v.iter().all(|i|valid_item(i,revision)))
            && m.pilot.as_ref().is_none_or(|p|actor_valid(p,revision)))
}
/// O(I+A+C+G+M+J) admission, O(pilot nesting) stack. Visit persisted item homes
/// and raw captured rates, not current floor or source-item identity. Cold save
/// validation additionally traverses serialized temporal payloads on restoration.
pub fn validate_run(r:&Run)->Result<(),String> {
    let rev=r.simulation_revision.value_or(29.);
    if !r.inventory.iter().all(|i|valid_item(i,rev)) || !actor_valid(&r.player,rev)
        || !r.initial_player.as_ref().is_none_or(|a|actor_valid(a,rev))
        || !r.actors.values().all(|v|v.iter().all(|a|actor_valid(a,rev)))
        || !r.ground.values().all(|v|v.iter().all(|g|valid_item(&g.item,rev)))
        || !r.remembered_items.as_ref().is_none_or(|m|m.values().all(|v|v.iter().all(|g|valid_item(&g.item,rev))))
        || !r.detected_items.as_ref().is_none_or(|m|m.values().all(|v|v.iter().all(|g|valid_item(&g.item,rev))))
        || !crate::debug_floor::state(r).is_none_or(|s|s.pending.iter().chain(&s.stash).all(|g|valid_item(&g.item,rev))) {
        return Err("Invalid Revision 52 amulet tier or captured periodic rate".into());
    }
    Ok(())
}

#[cfg(test)]
#[path="revision52_amulet_tiers_tests.rs"]
mod tests;
