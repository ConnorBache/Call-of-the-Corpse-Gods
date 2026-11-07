//! Knowledge-filtered Timeweave arrows. No simulation mutation, RNG or visibility queries.
use corpse_core::{Run, timeweave::{Cell, Plane}};

/// Expected O(1) indexed lookup (worst O(L) hash lookup), O(1) scratch.
/// Called only while rebuilding existing observed terrain, never from paint.
/// Invalid/non-adjacent edges are not visualized as invented straight connections.
pub fn direction(run:&Run, width:usize, height:usize, x:usize, y:usize, current:bool, direct:bool)->Option<(i8,i8)> {
    if !current || !direct || run.simulation_revision.value_or(39.)<41. || width==0 || x>=width || y>=height {return None;}
    if !run.depth.is_finite() || run.depth.fract()!=0. || run.depth<1. || run.depth>u32::MAX as f64 {return None;}
    let plane=if corpse_core::dreamwalking::context_dream(run){Plane::Dream}else{Plane::Waking};
    let tile=u32::try_from(y.checked_mul(width)?.checked_add(x)?).ok()?;
    let cell=Cell{floor:run.depth as u32,plane,tile};
    let segment=run.timeweave.as_ref()?.get(cell)?;
    if segment.cell!=segment.from || segment.from!=cell || segment.to.floor!=cell.floor || segment.to.plane!=cell.plane {return None;}
    let destination=segment.to.tile as usize;
    if destination>=width.checked_mul(height)? {return None;}
    let dx=(destination%width) as i64-x as i64;
    let dy=(destination/width) as i64-y as i64;
    if !(-1..=1).contains(&dx) || !(-1..=1).contains(&dy) || (dx==0 && dy==0) {return None;}
    Some((dx as i8,dy as i8))
}

#[cfg(test)]
mod tests {
    use super::*;
    use corpse_core::{Field, timeweave::{Segment,Trail}};
    use std::sync::Arc;
    fn run()->Run {let mut r=Run::default();r.simulation_revision=41.0.into();r.depth=1.;r}
    fn install(r:&mut Run,plane:Plane, floor:u32, from:u32,to:u32){
        let cell=Cell{floor,plane,tile:from};let mut trail=Trail::default();
        trail.append(Segment{cell,from:cell,to:Cell{tile:to,..cell}});r.timeweave=Field::Value(Arc::new(trail));
    }
    #[test] fn all_adjacent_arrows_and_no_hidden_or_remembered_trail(){
        let mut r=run();for (dx,dy) in [(-1,-1),(0,-1),(1,-1),(-1,0),(1,0),(-1,1),(0,1),(1,1)] {
            install(&mut r,Plane::Waking,1,55,(55+dx+dy*10) as u32);
            assert_eq!(direction(&r,10,10,5,5,true,true),Some((dx as i8,dy as i8)));
            assert_eq!(direction(&r,10,10,5,5,false,true),None);
            assert_eq!(direction(&r,10,10,5,5,true,false),None);
        }
    }
    #[test] fn floor_plane_legacy_and_bad_edge_are_not_drawn(){
        let mut r=run();install(&mut r,Plane::Waking,2,55,56);assert_eq!(direction(&r,10,10,5,5,true,true),None);
        install(&mut r,Plane::Dream,1,55,56);assert_eq!(direction(&r,10,10,5,5,true,true),None);
        r.dreamwalking=Field::Value(corpse_core::dreamwalking::State{context_dream:true,..Default::default()});
        assert_eq!(direction(&r,10,10,5,5,true,true),Some((1,0)));
        r.dreamwalking=Field::Missing;
        install(&mut r,Plane::Waking,1,55,57);assert_eq!(direction(&r,10,10,5,5,true,true),None);
        install(&mut r,Plane::Waking,1,55,56);r.simulation_revision=40.0.into();assert_eq!(direction(&r,10,10,5,5,true,true),None);
        r.simulation_revision=41.0.into();assert_eq!(direction(&r,10,10,5,5,true,true),Some((1,0)));
        assert_eq!(direction(&r,0,10,5,5,true,true),None);
    }
}

#[cfg(test)]
#[path="timeweave_scene_qa_tests.rs"]
mod full_scene_qa;
