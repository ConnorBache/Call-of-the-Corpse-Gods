//! Authored presentation geometry only: no gameplay RNG, terrain writes or visibility grants.
use crate::{Actor,Run,events::{self,EventPoint,Stage,Delivery}};
/// O(r²) candidates, plus ordinary visibility queries for each admitted cell.
/// Small authored radii are bounded to 12 cells; renderer never adds unknown cells.
pub fn area(r:&Run,a:&Actor,key:&str,style:&str,radius:i32,color:&str,stage:Stage) {
    let radius=radius.clamp(0,12);
    let mut cells=Vec::new();
    for dy in -radius..=radius {for dx in -radius..=radius {
        if dx!=0||dy!=0 {cells.push(EventPoint{x:a.x+f64::from(dx),y:a.y+f64::from(dy)});}
    }}
    cells.push(a.into());
    let serial=events::emission_serial();
    events::cue(r,a,key,style,stage,&cells);
    events::decorate(serial,key,style,a.into(),color,None);
}
pub fn nova(r:&Run,a:&Actor,key:&str,style:&str,radius:i32) {
    let color=if style.contains("red"){"#ff3030"}else if style.contains("blue"){"#4080ff"}else if style.contains("green"){"#40ff60"}else if style.contains("black"){"#000000"}else if style.contains("orange"){"#ff9a20"}else if style.contains("yellow"){"#ffff40"}else{"#ff60c8"};
    nova_colored(r,a,key,style,radius,color);
}

/// Shared authored blessing waveform; color comes from the chosen god, not a
/// renderer style heuristic. Keep actor/plane/tile admission at the common cue.
pub fn short_nova(r:&Run,a:&Actor,key:&str,color:&str) {
    if events::enabled(r) {nova_colored(r,a,key,SHORT_NOVA,2,color);}
}
/// Authored colors match the native god palette in ability_ui::god_color.
pub const SHORT_NOVA:&str="nova-authored-short";
pub const ISHEILAH:&str="#b877ff";
pub const NUMO:&str="#64ff72";
pub const URSA:&str="#72a0ff";
pub const LICHOR:&str="#ff3f8e";
pub const ANAVANI:&str="#fff0a3";
pub const MORAE:&str="#f36cff";

/// Presentation-only cue centered on the placed ward, not the caster.
pub const BULWARK_NOVA: &str = "nova-bulwark-green";
pub fn bulwark_nova(r: &Run, center: EventPoint) {
    if !events::enabled(r) || !crate::vision::directly_visible_tile(r, center.x as usize, center.y as usize) { return; }
    let serial = events::emission_serial();
    events::at(r, "P11", BULWARK_NOVA, Stage::Special, &nova_cells(&center, 2));
    events::decorate(serial, "P11", BULWARK_NOVA, center, "#56ff37", None);
}
fn nova_cells(center: &EventPoint, radius: i32) -> Vec<EventPoint> {
    // Lobed radial envelope: the old square was an unintended visible boundary.
    // The extra two cells leave room for wisps beyond the nominal radius, but
    // admission still goes through the ordinary visibility-filtered cue path.
    let radius=radius.clamp(1,10);
    let reach=radius+2;
    let mut cells=Vec::new();
    for dy in -reach..=reach {for dx in -reach..=reach {
        let angle=(dy as f64).atan2(dx as f64);
        let edge=radius as f64+0.7+0.65*(3.*angle+0.4).sin()+0.55*(5.*angle-0.8).cos();
        if ((dx*dx+dy*dy) as f64).sqrt()<=edge.max(1.) {
            cells.push(EventPoint{x:center.x+f64::from(dx),y:center.y+f64::from(dy)});
        }
    }}
    cells
}
fn nova_colored(r:&Run,a:&Actor,key:&str,style:&str,radius:i32,color:&str) {
    let cells = nova_cells(&a.into(), radius);
    let serial=events::emission_serial();
    events::cue(r,a,key,style,Stage::Special,&cells);
    events::decorate(serial,key,style,a.into(),color,None);
}
use crate::movement::Point;

/// Rasterize each real sight segment. A portal hop contributes its two ends,
/// never the screen-space line between them. No projectile/RNG state is used.
/// The caller still admits only observed cells; this is world geometry.
pub fn connection_cells(r: &Run, a: &Actor, to: EventPoint, effect: bool) -> Vec<Point> {
    let Some(f) = crate::dreamwalking::floor_for(r, a) else { return vec![] };
    let Some(start) = crate::portals::cell(f, a.x, a.y) else { return vec![] };
    let Some(end) = crate::portals::cell(f, to.x, to.y) else { return vec![] };
    let (from, target) = (crate::portals::point(f, start), crate::portals::point(f, end));
    let route = if crate::portals::enabled(r) && f.portals.as_ref().is_some_and(|p| !p.is_empty()) {
        if effect { crate::ability_sight::effect_route(r, a, to.x, to.y) }
        else { crate::vision::optical_route(r, a, target) }
    } else { None };
    let Some(route) = route else { return crate::portals::linear_cells(from, target) };
    let mut cells = vec![from];
    for pair in route.points.windows(2) {
        let (p, q) = (pair[0], pair[1]);
        let index = |p: Point| p.y as usize * f.width() + p.x as usize;
        if crate::portals::mate(f, index(p)) == Some(index(q)) {
            cells.push(q);
        } else {
            cells.extend(crate::portals::linear_cells(p, q).into_iter().skip(1));
        }
    }
    cells
}

/// Two consecutive beams on the exact same route, first cast then pull.
pub const BECKON_OUTBOUND: &str = "beckon-outbound";
pub const BECKON_BEAM: &str = "beckon-return";
pub fn beckon_round_trip(r: &Run, caster: &Actor, target: &Actor, key: &str, inward: &[EventPoint], color: &str) {
    let outward: Vec<_> = inward.iter().rev().cloned().collect();
    let serial = events::emission_serial();
    events::travel(r, caster, key, BECKON_OUTBOUND, &outward, Delivery::Magic, None);
    events::decorate(serial, key, BECKON_OUTBOUND, caster.into(), color, None);
    let serial = events::emission_serial();
    events::travel(r, caster, key, BECKON_BEAM, inward, Delivery::Magic, None);
    events::decorate(serial, key, BECKON_BEAM, caster.into(), color, None);
    if events::actor_observed(r,target) {events::hide_actor_during(serial,target);}
}

pub fn line(r:&Run,a:&Actor,to:EventPoint,key:&str,style:&str,color:&str) {
    emit_line(r,a,to,key,style,color,false);
}
pub fn effect_line(r:&Run,a:&Actor,to:EventPoint,key:&str,style:&str,color:&str) {
    emit_line(r,a,to,key,style,color,true);
}
fn emit_line(r:&Run,a:&Actor,to:EventPoint,key:&str,style:&str,color:&str,effect:bool) {
    if !events::enabled(r) { return; }
    let cells: Vec<_> = connection_cells(r,a,to.clone(),effect).into_iter()
        .map(|p|EventPoint{x:p.x as f64,y:p.y as f64}).collect();
    let serial=events::emission_serial();
    events::travel(r,a,key,style,&cells,Delivery::Magic,None);
    events::decorate(serial,key,style,to,color,Some(a.into()));
}

#[cfg(test)] mod tests {
 use super::*;
 fn room()->Run {let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();r.player.x=4.;r.player.y=4.;r.player.conditions.clear();let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);r}
 #[test] fn nova_and_box_have_centered_explicit_footprints_without_mutation(){
  let r=room();let before=serde_json::to_value(&r).unwrap();events::begin();nova(&r,&r.player,"P01","nova-red-small",2);area(&r,&r.player,"P09","box-expand",2,"#ffd340",Stage::Special);let e=events::finish();assert_eq!(e.len(),2);
  assert_eq!(e[0].to,EventPoint{x:4.,y:4.});
  let cells=&e[0].animation.as_ref().unwrap().cells;
  assert!(cells.iter().any(|p|(p.x-4.).abs()>2.||(p.y-4.).abs()>2.));
  assert!(!cells.iter().any(|p|p.x==8.&&p.y==8.));
  assert_eq!(e[1].animation.as_ref().unwrap().cells.len(),25);
  assert_eq!(before,serde_json::to_value(&r).unwrap());
 }
 #[test] fn line_is_ordered_and_held_and_does_not_grant_visibility(){
  let r=room();let before=serde_json::to_value(&r).unwrap();events::begin();line(&r,&r.player,EventPoint{x:7.,y:4.},"P19","line-hold","#a040ff");let e=events::finish().remove(0);assert_eq!(e.from,Some((&r.player).into()));assert_eq!(e.to,EventPoint{x:7.,y:4.});let a=e.animation.unwrap();assert_eq!(a.delivery,Delivery::Magic);assert_eq!(a.cells.iter().map(|p|p.x).collect::<Vec<_>>(),vec![4.,5.,6.,7.]);assert_eq!(before,serde_json::to_value(&r).unwrap());
 }
 #[test] fn filtered_geometry_cannot_redecorate_a_prior_cue(){
  let r=room();events::begin();nova(&r,&r.player,"P01","nova-red-small",2);let mut hidden=r.player.clone();hidden.id=9999.;hidden.x=-20.;hidden.y=-20.;nova(&r,&hidden,"P01","nova-red-small",2);let e=events::finish();assert_eq!(e.len(),1);assert_eq!(e[0].to,(&r.player).into());
 }
 #[test]
 fn portal_animation_routes_toward_entrance_and_resumes_at_exit() {
    let mut r = room();
    r.player.conditions.insert("allSeeingReveal".into(), 1.);
    r.player.conditions.insert("allSeeingDepth".into(), r.depth);
    let f = r.floor_mut().unwrap();
    let w = f.width();
    for y in 0..f.height() { f.tiles[y*w+9] = "wall".into(); }
    f.portals = vec![crate::portals_generation::PortalPair{a:4*w+6,b:4*w+20,color_index:0}].into();
    let to = EventPoint{x:23.,y:4.};
    let expected = [4,5,6,20,21,22,23];
    for effect in [false, true] {
        assert_eq!(connection_cells(&r,&r.player,to.clone(),effect).iter().map(|p|p.x).collect::<Vec<_>>(),expected);
        events::begin();
        emit_line(&r,&r.player,to.clone(),"P19","line-hold","#ffffff",effect);
        let out = events::finish();
        let cells = &out[0].animation.as_ref().unwrap().cells;
        assert_eq!(cells.iter().map(|p|p.x as i32).collect::<Vec<_>>(),expected);
    }
 }
}
