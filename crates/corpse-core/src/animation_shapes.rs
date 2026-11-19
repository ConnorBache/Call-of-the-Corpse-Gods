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

fn nova_colored(r:&Run,a:&Actor,key:&str,style:&str,radius:i32,color:&str) {
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
            cells.push(EventPoint{x:a.x+f64::from(dx),y:a.y+f64::from(dy)});
        }
    }}
    let serial=events::emission_serial();
    events::cue(r,a,key,style,Stage::Special,&cells);
    events::decorate(serial,key,style,a.into(),color,None);
}
/// Straight visual connection, not an attack/LoE trace. O(max(dx,dy)) candidates,
/// bounded by the current floor dimensions; hidden cells are omitted by events::travel.
pub fn line(r:&Run,a:&Actor,to:EventPoint,key:&str,style:&str,color:&str) {
    if !a.x.is_finite()||!a.y.is_finite()||!to.x.is_finite()||!to.y.is_finite(){return;}
    let Some(f)=r.floor() else{return;};
    let bound=f.width().max(f.height()) as i32 +1;
    let(mut x,mut y)=(a.x as i32,a.y as i32);let(tx,ty)=(to.x as i32,to.y as i32);
    let dx=(tx-x).abs();let dy=-(ty-y).abs();if dx.max(-dy)>bound{return;}
    let sx=if x<tx{1}else{-1};let sy=if y<ty{1}else{-1};let mut error=dx+dy;let mut cells=vec![];
    loop {cells.push(EventPoint{x:f64::from(x),y:f64::from(y)});if x==tx&&y==ty{break;}
        let e=error*2;if e>=dy {error+=dy;x+=sx;}if e<=dx {error+=dx;y+=sy;}}
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
}
