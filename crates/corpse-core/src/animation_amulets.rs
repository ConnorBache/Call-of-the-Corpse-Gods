//! Presentation-only receipts for actual R50 amulet outcomes under R52.
use crate::{Actor,Run,events::{self,EventPoint,Stage},revision50_amulet_effects::Effect};
pub fn key(kind:Effect)->&'static str {match kind{Effect::Lightning=>"A01",Effect::Firebolt=>"A02",Effect::Poison=>"A03",Effect::Iceblast=>"A04",Effect::Acid=>"A05",Effect::Blinking=>"A06"}}
pub fn route(r:&Run,source:&Actor,kind:Effect,points:&[crate::movement::Point],impact:Option<crate::movement::Point>){
    let mut cells:Vec<_>=points.iter().map(|p|EventPoint{x:p.x as f64,y:p.y as f64}).collect();
    if let Some(p)=impact{cells.push(EventPoint{x:p.x as f64,y:p.y as f64});}
    let style=match kind{Effect::Lightning=>"bolt",Effect::Firebolt=>"ember",Effect::Poison=>"dart",Effect::Acid=>"glob",_=>"ripple"};
    if matches!(kind,Effect::Lightning|Effect::Firebolt|Effect::Poison|Effect::Acid){
        events::travel(r,source,key(kind),style,&cells,events::Delivery::Magic,None);
    }else{events::cue(r,source,key(kind),style,Stage::Travel,&cells);}
    if kind!=Effect::Firebolt {if let Some(p)=cells.last(){events::cue(r,source,key(kind),"spark",Stage::Impact,std::slice::from_ref(p));}}
}
pub fn outcome(r:&Run,before:&Actor,kind:Effect,packets_before:u64){
    if matches!(kind,Effect::Lightning|Effect::Firebolt) && events::damage_count(before.id)==packets_before {
        events::cue(r,before,key(kind),"rejected",Stage::Impact,&[before.into()]);
    }
}
pub fn cell_points(r:&Run,cells:&[usize])->Vec<EventPoint>{let Some(f)=r.floor()else{return vec![]};let w=f.width();cells.iter().map(|i|EventPoint{x:(i%w)as f64,y:(i/w)as f64}).collect()}

/// Called after the actual normal-clock status decrements, not an animation timer.
pub fn expired_statuses(r:&Run,before:&Actor){
 let Some(after)=crate::engine::actor(r,before.id) else{return};if after.dead.value_or(false){return}
 for (condition,key,style) in [(crate::revision50_amulet_effects::ACID_TURNS,"A05","shield-close"),(crate::revision50_amulet_effects::POISON_DAMAGE,"A03","dissolve-ring")] {
  if crate::engine::condition(before,condition)>0.&&crate::engine::condition(after,condition)<=0. {events::cue(r,after,key,style,Stage::Special,&[after.into()]);}
 }
}
