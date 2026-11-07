//! Revision 37 contact terrain. Queries never draw; only committed walk attempts
//! roll ice. Boolean-layer lookup is O(1); first puddle allocation is O(V).
use crate::{Actor, Field, Floor, Run};

pub fn enabled(r: &Run) -> bool { r.simulation_revision.value_or(29.) >= 37. }
fn layer(v: &Field<Vec<bool>>, at: usize) -> bool { v.as_ref().and_then(|v|v.get(at)).copied().unwrap_or(false) }
pub fn ice_at(f: &Floor, at: usize) -> bool { layer(&f.ice,at) }
pub fn melt_ice(f:&mut Floor,at:usize)->bool {
    if !ice_at(f,at){return false}
    f.ice.as_mut().unwrap()[at]=false;f.dirty=true;true
}
pub fn snow_at(f: &Floor, at: usize) -> bool { layer(&f.snow,at) }
pub fn hellfire_at(f: &Floor, at: usize) -> bool { layer(&f.hellfire,at) }
pub fn ice_creature(a: &Actor) -> bool {
    crate::enemy_traits::definition(a).is_some_and(|d|matches!(d.id.as_str(),"wendigo"|"yeti"))
}
/// Called only after walk validity, occupancy, roots and other action costs have
/// been resolved. Forced displacement and flying actors do not attempt footing.
pub fn slip(r: &mut Run, id: f64) -> bool {
    if !enabled(r) { return false; }
    let Some(a)=crate::engine::actor(r,id) else{return false};
    if crate::engine::condition(a,"forcedMove") != 0. || crate::travel::flying(r,a) || ice_creature(a) {return false;}
    let Some(f)=r.floor() else{return false};
    let at=a.y as usize*f.width()+a.x as usize;
    if !ice_at(f,at) {return false;}
    // The worn power prevents footing failure before any gameplay RNG draw.
    // It changes neither the terrain nor snow's ordinary fire-quenching contact.
    if crate::revision50_rings::has_effect(r,a,crate::revision50_rings::ICE_IMMUNITY) {return false;}
    if crate::rng::random(r) >= 1./3. {return false;}
    if id==r.player.id {crate::engine::log(r,"You slip on the ice.","player");}
    true
}
/// A fire contact consumes the snow before burning any underlying fuel. It does
/// not turn the cell into traversable water or add swimming/current mechanics.
pub fn melt_snow(f: &mut Floor, at: usize) -> bool {
    if !snow_at(f,at) {return false;}
    f.snow.as_mut().unwrap()[at]=false;
    let n=f.tiles.len();
    f.puddles.get_or_insert_with(||vec![false;n])[at]=true;
    if !hellfire_at(f,at) {
        if let Some(e)=f.environment.as_mut() {
            e.fire[at]=0.;
            if let Some(v)=e.fire_kind.as_mut(){v[at]=None;}
            if let Some(v)=e.fire_fresh.as_mut(){v[at]=false;}
            if let Some(v)=e.fire_reaction.as_mut(){v[at]=0.;}
            if let Some(p)=e.provenance.as_mut().and_then(|p|p.get_mut("fire")){p.owner[at]=-1.;p.player[at]=0.;}
        }
    }
    f.dirty=true;
    true
}
/// Snow follows ordinary water's contact exemption for flight and permanent or
/// fire-immune bodies: it melts, but cannot remove intrinsic eternal burning.
pub fn snow_contact(r: &mut Run, id: f64) -> bool {
    if !enabled(r) {return false;}
    let Some(a)=crate::engine::actor(r,id) else{return false};
    let revision = r.simulation_revision.value_or(29.);
    if crate::revision51_ichor_fire::immune(revision, a) {
        crate::revision51_ichor_fire::extinguish(revision,
            crate::engine::actor_mut_trusted(r, id).unwrap());
        return false;
    }
    if a.dead.value_or(false) || !crate::dreamwalking::contact_plane(r,a)
        || crate::travel::flying(r,a)
        || (crate::engine::condition(a,"burning")<=0. && !crate::corpses::has_permanent_burning(a)) {return false;}
    let Some(f)=r.floor() else{return false};
    let at=a.y as usize*f.width()+a.x as usize;
    if !snow_at(f,at){return false;}
    let permanent=crate::corpses::has_permanent_burning(a)
        || a.abilities.iter().any(|s|s=="enemy:beast:fire-immunity"||s=="enemy:beast:everburn"||s=="anavani-star-power");
    melt_snow(r.floor_mut().unwrap(),at);
    if !permanent {
        let a=crate::engine::actor_mut_trusted(r,id).unwrap();
        a.conditions.insert("burning".into(),0.);
        a.conditions.shift_remove("burningOwner");
        a.conditions.shift_remove("burningPlayer");
    }
    true
}
/// O(V) once per normal environment tick, no RNG. Direct ignition and spread
/// handle contacts immediately; this covers restored/generated overlapping fire.
pub fn melt_burning_snow(r: &mut Run) {
    if !enabled(r){return;}
    if crate::revision50::enabled(r) {
        let f=r.floor().unwrap();let w=f.width();
        let cells:Vec<_>=f.ice.as_ref().into_iter().flatten().enumerate().filter_map(|(at,&ice)|
            (ice&&(hellfire_at(f,at)||f.environment.as_ref().is_some_and(|e|e.fire[at]>0.)
                ||std::iter::once(&r.player).chain(r.actors_at(r.depth)).any(|a|!a.dead.value_or(false)&&a.hp>0.&&crate::dreamwalking::contact_plane(r,a)&&!crate::travel::flying(r,a)&&a.y as usize*w+a.x as usize==at&&(crate::engine::condition(a,"burning")>0.||crate::corpses::has_permanent_burning(a))))).then_some(at)).collect();
        for at in cells {melt_ice(r.floor_mut().unwrap(),at);}
    }
    let Some(f)=r.floor() else{return};
    let cells:Vec<_>=f.snow.as_ref().into_iter().flatten().enumerate()
        .filter_map(|(i,&snow)|(snow && (hellfire_at(f,i) || f.environment.as_ref().is_some_and(|e|e.fire[i]>0.))).then_some(i)).collect();
    for at in cells {melt_snow(r.floor_mut().unwrap(),at);}
}
