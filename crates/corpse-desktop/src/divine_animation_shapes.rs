//! Detached tile-only animation patterns. Geometry is restricted to admitted receipt cells.
//! No gameplay RNG or simulation mutation. Nova is a small deterministic cellular wave.
use crate::{effects::{Tile, MAGIC_CELL_MS},render::Rgb};
use corpse_core::events::PresentationEvent;
use std::collections::{BTreeMap,BTreeSet};
pub fn duration(e:&PresentationEvent)->Option<f64>{
 let a=e.animation.as_ref()?;let s=a.style.as_str();
 if let Some(d)=s.strip_prefix("line-hold:").or_else(||s.strip_prefix("moving-actor:")).and_then(|d|d.parse().ok()){return Some(d)}
 if s=="moving-actor-commandment"{Some(240.)}
 else if s==corpse_core::animation_shapes::SHORT_NOVA {Some(360.)}
 else if s.starts_with("nova-"){Some(if s.ends_with("large"){1800.}else{1200.})}
 else if s.starts_with("box-"){Some(500.)}
 else if s=="line-grow"||s=="line-hold"{Some(a.cells.len().max(1) as f64*MAGIC_CELL_MS+50.)}
 else if s=="thread-grow"{Some(a.cells.len().max(1) as f64*32.+100.)}
 else if s=="map-reveal"||s.starts_with("wave-reveal")||s==corpse_core::banshee_rules::PURPLE_WAVE{Some(900.)}
 else if s=="wave-gasser"{Some(230.)}
 else if s=="wave-charge-inward"{Some(340.)}
 else if s=="light100"{Some(a.cells.len().max(1) as f64*30.+50.)}
 else if s.starts_with("moving-actor"){Some(a.cells.len().max(1) as f64*60.+100.)}
 else {None}
}
fn nova_color(s:&str,default:Rgb)->Rgb{if s.contains("redorange"){return [255,75,30]}for(word,c)in[("red",[255,35,55]),("blue",[50,130,255]),("green",[65,255,90]),("black",[0,0,0]),("orange",[255,145,30]),("yellow",[255,240,45]),("pink",[255,105,210])]{if s.contains(word){return c}}default}
pub fn tiles(e:&PresentationEvent,age:f64,reduced:bool,default:Rgb)->Option<Vec<Tile>>{
 let a=e.animation.as_ref()?;let total=duration(e)?;let s=a.style.as_str();
 if age<0.||age>=total{return Some(vec![])}
 let white=s==corpse_core::banshee_rules::WHITE_WAVE;
 if white || s==corpse_core::banshee_rules::PURPLE_WAVE {
  let color=if white{[255,255,255]}else{[185,75,255]};
  let bands=a.arrival_steps.as_deref()?;
  // Same outward bands as the Screamer, with the purple optical cells
  // riding the exact same front. Full-strength ink allows sound in fog.
  return Some(a.cells.iter().zip(bands).filter_map(|(p,&band)|{
   let t=age-f64::from(band)*65.;
   let light=if reduced{if white{0.16}else{0.5}}
       else if (0. ..250.).contains(&t){(if white{0.68}else{0.95})*(1.-t/250.)}
       else{return None};
   Some(Tile{x:p.x as i32,y:p.y as i32,color,light,field_strength:1.,glyph:None,mark:None})
  }).collect())
 }
 let center=(e.to.x.round() as i32,e.to.y.round() as i32);
 let mut color=if s=="wave-reveal-telepathy"{[45,140,255]}else if s=="wave-reveal-fire"{[255,90,20]}else if s=="wave-reveal-darkness"{[0,0,0]}else if s=="wave-reveal-shattering"{[90,210,255]}else if s=="wave-reveal-yellow"{[255,235,45]}else if s=="wave-reveal-white"{[255,255,255]}else if s=="wave-reveal-purple"{[185,75,255]}else if s.starts_with("nova-") && s!=corpse_core::animation_shapes::SHORT_NOVA {nova_color(s,default)}else{default};
 if s=="light100"||s.starts_with("moving-actor"){color=[255,255,255];}
 let make=|x,y,light,glyph|Tile{x,y,color,light,field_strength:if s=="light100"||s.starts_with("moving-actor"){1.}else{0.85},glyph,mark:None};
 if s=="thread-grow"{return Some(vec![])}
 if reduced{return Some(a.cells.iter().map(|p|make(p.x as i32,p.y as i32,0.55,None)).collect())}
 let phase=(age/total).clamp(0.,1.);
 let allowed:BTreeSet<_>=a.cells.iter().map(|p|(p.x as i32,p.y as i32)).collect();
 if s=="wave-charge-inward" {
  return Some(allowed.into_iter().filter_map(|(x,y)|{
   let d=f64::from(x-center.0).hypot(f64::from(y-center.1));
   let t=age-(4.-d).max(0.)*55.;
   (0. ..120.).contains(&t).then(||make(x,y,0.92*(1.-t/120.),None))
  }).collect());
 }
 if s=="wave-gasser" {
  return Some(allowed.into_iter().filter_map(|(x,y)| {
   let offset=if (x,y)==center {0.}else{70.};
   let t=age-offset;
   (t>=0.&&t<150.).then(||make(x,y,0.85*(1.-t/150.),None))
  }).collect());
 }
 if s.starts_with("nova-"){
  // Deterministic cellular arrival field, no simulation RNG. Weighted neighbor
  // growth avoids the old eight-neighbor Chebyshev (perfect square) front.
  // O(C² + 8C log C), bounded by the admitted nova envelope; O(C) scratch.
  let seed=allowed.iter().min_by_key(|(x,y)|(x-center.0).abs()+(y-center.1).abs()).copied();
  let mut born=BTreeMap::<(i32,i32),u32>::new();
  let mut settled=BTreeSet::new();
  if let Some(p)=seed{born.insert(p,0);}
  loop {
   let next=born.iter().filter(|(p,_)|!settled.contains(*p)).min_by_key(|(p,t)|(**t,**p)).map(|(p,t)|(*p,*t));
   let Some(((x,y),at))=next else{break};settled.insert((x,y));
   for dy in -1i32..=1 {for dx in -1i32..=1 {
    let p=(x+dx,y+dy);if (dx==0&&dy==0)||!allowed.contains(&p)||settled.contains(&p){continue}
    let rx=p.0-center.0;let ry=p.1-center.1;
    let hash=(rx as u32).wrapping_mul(73856093)^(ry as u32).wrapping_mul(19349663);
    let step=if dx!=0&&dy!=0{140}else{100};
    let arrival=at+step+hash%91;
    if born.get(&p).is_none_or(|old|arrival<*old){born.insert(p,arrival);}
   }}
  }
  if s==corpse_core::animation_shapes::SHORT_NOVA {
   // Clipped footprints can have disconnected islands. Illuminate only the
   // admitted cells, using distance for cells that had no visible connection.
   for &p in &allowed {
    born.entry(p).or_insert_with(|| {
     let dx=f64::from(p.0)-f64::from(center.0);let dy=f64::from(p.1)-f64::from(center.1);
     (dx.hypot(dy)*150.).round() as u32
    });
   }
  }
  let max=born.values().copied().max().unwrap_or(1).max(1)as f64;
  let propagation=total*0.45;
  let fade=total*0.55;
  return Some(born.into_iter().filter_map(|((x,y),arrival)|{
   let t=age-arrival as f64/max*propagation;
   (t>=0.&&t<fade).then(||{let mut tile=make(x,y,0.95*(1.-t/fade),None);
    if s.contains("rainbow"){let colors=[[255,45,55],[255,150,30],[255,240,45],[60,255,100],[50,160,255],[170,70,255]];tile.color=colors[((arrival/100)as usize+(x-center.0).unsigned_abs()as usize)%colors.len()];}tile
   })
  }).collect())
 }
 if s.starts_with("box-"){
  let radius=allowed.iter().map(|&(x,y)|(x-center.0).abs().max((y-center.1).abs())).max().unwrap_or(0).max(1);
  let count=1+radius*2;let n=((phase*count as f64).floor() as i32).min(count-1);
  let n=if s=="box-contract"{count-1-n}else{n};let r=(n+1)/2;let square=n%2==0;
  return Some(allowed.into_iter().filter(|&(x,y)|{let dx=(x-center.0).abs();let dy=(y-center.1).abs();if r==0{dx==0&&dy==0}else if square{dx.max(dy)==r}else{((dx*dx+dy*dy)as f64).sqrt()>=r as f64-0.6&&((dx*dx+dy*dy)as f64).sqrt()<=r as f64+0.35}}).map(|(x,y)|make(x,y,0.95,None)).collect())
 }
 if s=="line-grow"||s.starts_with("line-hold"){
  let count=(age/MAGIC_CELL_MS).floor() as usize+1;return Some(a.cells.iter().take(count).map(|p|make(p.x as i32,p.y as i32,0.9,None)).collect())
 }
 if (s=="map-reveal"||s.starts_with("wave-reveal")){
  let max=allowed.iter().map(|&(x,y)|((x-center.0).pow(2)+(y-center.1).pow(2))as f64).fold(1.,f64::max).sqrt();
  return Some(allowed.into_iter().filter_map(|(x,y)|{let dist=(((x-center.0).pow(2)+(y-center.1).pow(2))as f64).sqrt();let t=age-(dist/max*10.-1e-9).ceil().max(0.)*65.;(t>=0.&&t<250.).then(||make(x,y,0.95*(1.-t/250.),None))}).collect())
 }
 let step=if s=="moving-actor-commandment"{200./a.cells.len().max(1) as f64}else if s=="light100"{30.}else{60.};
 let head=((age/step).floor() as usize).min(a.cells.len().saturating_sub(1));
 Some(a.cells.iter().enumerate().filter_map(|(i,p)|{if i>head||head-i>2{return None}Some(make(p.x as i32,p.y as i32,if i==head{1.}else{0.5/(head-i)as f64},if i==head&&s.starts_with("moving-actor"){e.glyph.clone()}else{None}))}).collect())
}

#[cfg(test)]mod tests{
 use super::*;use corpse_core::events::{AnimationCue,Delivery,EventPoint,Stage};
 fn event(style:&str)->PresentationEvent{let cells=(-2..=2).flat_map(|y|(-2..=2).map(move|x|EventPoint{x:x as f64,y:y as f64})).collect();PresentationEvent{kind:"animation".into(),from:None,to:EventPoint{x:0.,y:0.},glyph:None,color:None,label:None,depth:1.,animation:Some(AnimationCue{proposal:"P01".into(),style:style.into(),stage:Stage::Special,action:1,cells,delivery:Delivery::Local,surface:None,item_id:None,arrival_steps:None})}}
 #[test]fn minotaur_windup_runs_from_the_outer_ring_to_the_actor(){
  let mut e=event("wave-charge-inward");
  e.animation.as_mut().unwrap().cells=vec![EventPoint{x:4.,y:0.},EventPoint{x:2.,y:0.},EventPoint{x:0.,y:0.}];
  let at=|age|tiles(&e,age,false,[255,255,255]).unwrap().iter().map(|t|t.x).collect::<Vec<_>>();
  assert_eq!(duration(&e),Some(340.));
  assert_eq!(at(0.),vec![4]);
  assert_eq!(at(135.),vec![2]);
  assert_eq!(at(240.),vec![0]);
  assert!(at(340.).is_empty());
 }
 #[test]fn banshee_white_and_purple_use_the_same_outward_bands(){
  use corpse_core::banshee_rules::{PURPLE_WAVE,WHITE_WAVE};
  let mut white=event(WHITE_WAVE);
  let cue=white.animation.as_mut().unwrap();cue.cells=vec![EventPoint{x:2.,y:0.},EventPoint{x:8.,y:0.}];
  cue.arrival_steps=Some(vec![3,10]);cue.surface=Some("audible".into());
  let mut purple=white.clone();let cue=purple.animation.as_mut().unwrap();
  cue.cells.pop();cue.arrival_steps.as_mut().unwrap().pop();cue.style=PURPLE_WAVE.into();cue.surface=None;
  assert!(tiles(&white,194.,false,[0;3]).unwrap().is_empty());
  assert!(tiles(&purple,194.,false,[0;3]).unwrap().is_empty());
  let shared_white=tiles(&white,195.,false,[0;3]).unwrap();
  let shared_purple=tiles(&purple,195.,false,[0;3]).unwrap();
  assert_eq!((shared_white[0].x,shared_white[0].color),(2,[255,255,255]));
  assert_eq!((shared_purple[0].x,shared_purple[0].color),(2,[185,75,255]));
  assert!(shared_purple[0].light>shared_white[0].light);
  assert!(tiles(&white,650.,false,[0;3]).unwrap().iter().any(|t|t.x==8));
  assert!(tiles(&purple,650.,false,[0;3]).unwrap().is_empty());
 }
 #[test]fn commandment_group_is_short_and_never_interpolates_portal_gap(){
  let mut e=event("moving-actor-commandment");e.glyph=Some("g".into());
  e.animation.as_mut().unwrap().cells=vec![EventPoint{x:1.,y:1.},EventPoint{x:2.,y:1.},EventPoint{x:20.,y:8.}];
  assert_eq!(duration(&e),Some(240.));
  for age in [0.,80.,160.,220.] {let frame=tiles(&e,age,false,[0,0,0]).unwrap();assert!(frame.iter().all(|t|[(1,1),(2,1),(20,8)].contains(&(t.x,t.y))));assert_eq!(frame.iter().filter(|t|t.glyph.is_some()).count(),1);}
  let last=tiles(&e,220.,false,[0,0,0]).unwrap();assert_eq!(last.iter().find(|t|t.glyph.is_some()).map(|t|(t.x,t.y)),Some((20,8)));
  assert!(tiles(&e,240.,false,[0,0,0]).unwrap().is_empty());
 }
 #[test]fn nova_is_deterministic_neighbor_growth_and_never_invents_cells(){let e=event("nova-red-small");let first=tiles(&e,0.,false,[0,0,0]).unwrap();assert_eq!(first.len(),1);assert_eq!((first[0].x,first[0].y),(0,0));let later=tiles(&e,500.,false,[0,0,0]).unwrap();assert!(later.len()>1);assert!(later.iter().all(|t|(-2..=2).contains(&t.x)&&(-2..=2).contains(&t.y)&&t.color==[255,35,55]));assert!(tiles(&e,1200.,false,[0,0,0]).unwrap().is_empty());}
 #[test]fn tier_nova_is_short_nonblocking_and_paints_every_admitted_island(){
  let mut e=event(corpse_core::animation_shapes::SHORT_NOVA);
  e.color=Some("#64ff72".into());
  e.animation.as_mut().unwrap().cells=[(0,0),(1,0),(4,0),(4,1)].into_iter().map(|(x,y)|EventPoint{x:x as f64,y:y as f64}).collect();
  assert_eq!(duration(&e),Some(360.));
  let all=tiles(&e,162.,false,[100,255,114]).unwrap();
  assert_eq!(all.iter().map(|t|(t.x,t.y)).collect::<Vec<_>>(),vec![(0,0),(1,0),(4,0),(4,1)]);
  assert!(all.iter().all(|t|t.color==[100,255,114]));
  let mut effects=crate::effects::Effects::default();
  effects.add(vec![e],1.,1.,0.,false);effects.defer_until_frame(0.);
  assert!(!effects.presentation_pending(0.));assert!(!effects.presentation_pending(180.));
 }
 #[test]fn box_expands_point_circle_square_and_contracts(){let e=event("box-expand");assert_eq!(tiles(&e,0.,false,[1,2,3]).unwrap().len(),1);assert_eq!(tiles(&e,100.,false,[1,2,3]).unwrap().len(),4);assert_eq!(tiles(&e,200.,false,[1,2,3]).unwrap().len(),8);assert_eq!(tiles(&e,400.,false,[1,2,3]).unwrap().len(),16);let e=event("box-contract");assert_eq!(tiles(&e,0.,false,[1,2,3]).unwrap().len(),16);assert_eq!(tiles(&e,400.,false,[1,2,3]).unwrap().len(),1);}
 #[test]fn growing_line_keeps_completed_prefix(){let mut e=event("line-grow");e.animation.as_mut().unwrap().cells=(0..5).map(|x|EventPoint{x:x as f64,y:0.}).collect();assert_eq!(tiles(&e,0.,false,[1,2,3]).unwrap().len(),1);assert_eq!(tiles(&e,32.,false,[1,2,3]).unwrap().len(),3);assert_eq!(tiles(&e,80.,false,[1,2,3]).unwrap().len(),5);}
 #[test]
 fn portal_animation_growing_line_shows_the_exit_after_the_entrance_without_a_bridge() {
    let mut e=event("line-grow");
    e.animation.as_mut().unwrap().cells=[4,5,6,20,21,22].into_iter()
        .map(|x|EventPoint{x:x as f64,y:4.}).collect();
    let frame=|age|tiles(&e,age,false,[1,2,3]).unwrap().iter().map(|t|t.x).collect::<Vec<_>>();
    assert_eq!(frame(32.),[4,5,6]);
    assert_eq!(frame(48.),[4,5,6,20]);
    assert_eq!(frame(80.),[4,5,6,20,21,22]);
 }
 #[test]fn authored_beams_double_speed_without_accelerating_waves_or_actors(){
  for (style,step,tail) in [("line-hold",16.,50.),("line-grow",16.,50.),("light100",30.,50.),("moving-actor",60.,100.)] {
   let mut e=event(style);
   e.animation.as_mut().unwrap().cells=(0..5).map(|x|EventPoint{x:x as f64,y:0.}).collect();
   assert_eq!(duration(&e),Some(5.*step+tail));
   assert_eq!(tiles(&e,step-1.,false,[1,2,3]).unwrap().last().unwrap().x,0);
   assert_eq!(tiles(&e,step,false,[1,2,3]).unwrap().last().unwrap().x,1);
  }
  assert_eq!(duration(&event("wave-reveal-fire")),Some(900.));
 }
 #[test]fn local_effects_block_input_without_scene_capture(){let mut fx=crate::effects::Effects::default();fx.add(vec![event("nova-red-small")],1.,1.,0.,false);assert!(fx.presentation_pending(0.));assert!(fx.presentation_pending(1199.));assert!(!fx.presentation_pending(1200.));}
 #[test]fn cast_travel_finishes_before_same_action_displacement(){
  let mut cast=event("line-grow");
  let a=cast.animation.as_mut().unwrap();a.proposal="A07".into();a.stage=Stage::Travel;a.delivery=Delivery::Magic;a.cells=(0..5).map(|x|EventPoint{x:x as f64,y:0.}).collect();
  let mut displacement=event("root");
  let a=displacement.animation.as_mut().unwrap();a.proposal="A07".into();a.stage=Stage::Travel;a.delivery=Delivery::Local;a.cells=vec![EventPoint{x:4.,y:0.},EventPoint{x:3.,y:0.}];
  let mut arrival=event("unfold");arrival.animation.as_mut().unwrap().proposal="A07".into();
  let cast_duration=crate::effects::duration(&cast);
  let mut fx=crate::effects::Effects::default();fx.add(vec![cast,displacement,arrival],1.,1.,20.,false);
  assert_eq!(fx.cues[0].started,20.);
  assert_eq!(fx.cues.len(),1);
  assert!(!fx.presentation_pending(20.+cast_duration));
 }
 #[test]fn stages_finish_before_next_phase_starts(){let a=event("nova-red-small");let mut b=event("pulse");b.animation.as_mut().unwrap().stage=Stage::Health;let mut fx=crate::effects::Effects::default();fx.add(vec![a,b],1.,1.,0.,false);assert_eq!(fx.cues.len(),1);assert!(fx.presentation_pending(1199.));assert!(!fx.presentation_pending(1200.));}
}
