//! Inert, single-channel gas from an Overflowing rescue.
use crate::{Floor, Run, environment, engine};
/// Provisional volume tuning; each spent blood contributes equally.
pub const VOLUME_PER_USE:f64=100.;
/// O(1) candidate cells, plus existing O(N) environment hydration on first use.
/// Do not erase another gas during emission. If the origin is occupied, use
/// adjacent empty/mist cells, dividing the one dose without multiplying it.
pub fn emit(r:&mut Run,id:f64) {
    if r.simulation_revision.value_or(0.)<52. {return;}
    let Some(a)=engine::actor(r,id) else{return;};
    let (x,y,source)=(a.x as i32,a.y as i32,if a.id==r.player.id {-2.} else {a.id});
    let Some(f)=r.floor_mut() else{return;};
    if environment::initialize(f).is_err(){return;}
    let candidate=|dx:i32,dy:i32| {
        let (x,y)=(x+dx,y+dy);
        if x<0||y<0||x>=f.width() as i32||y>=f.height() as i32{return None;}
        let i=y as usize*f.width()+x as usize;
        let gas=f.environment.as_ref()?.gas_type.as_ref()?[i].as_deref();
        (environment::gas_passable(f,i)&&(gas.is_none()||gas==Some("bloodMist"))).then_some(i)
    };
    let cells:Vec<_>=if let Some(i)=candidate(0,0){vec![i]}else{
        [(-1,-1),(0,-1),(1,-1),(-1,0),(1,0),(-1,1),(0,1),(1,1)].into_iter().filter_map(|(x,y)|candidate(x,y)).collect()
    };
    if cells.is_empty(){return;}
    crate::capture_pilot::mark_unsupported("blood_mist_duration");
    let count=cells.len();
    for (ordinal,i) in cells.into_iter().enumerate(){
        let base=(VOLUME_PER_USE/count as f64).floor();
        let amount=base+if ordinal<(VOLUME_PER_USE as usize%count){1.}else{0.};
        let _=environment::add_gas(f,i,"bloodMist",amount,source);
    }
}
/// Once per world/environment update, not per creature/haste action.
/// O(N) time, O(1) extra memory. Diffusion carries remaining duration to new cells.
pub fn tick(f:&mut Floor)->Result<(),String>{
    let Some(e)=f.environment.as_mut() else{return Ok(());};
    let Some(turns)=e.blood_mist_turns.as_mut() else{return Ok(());};
    crate::capture_pilot::mark_unsupported("blood_mist_duration");
    let Some(types)=e.gas_type.as_mut() else{return Ok(());};
    for (i,t) in turns.iter_mut().enumerate(){
        if types[i].as_deref()!=Some("bloodMist"){*t=0.;continue;}
        *t=(*t-1.).max(0.);
        if *t==0. {
            types[i]=None;
            if let Some(v)=e.gas_volume.as_mut(){v[i]=0.;}
            if let Some(v)=e.gas_owner.as_mut(){v[i]=-1.;}
            if let Some(v)=e.gas_player.as_mut(){v[i]=0.;}
            if let Some(p)=e.provenance.as_mut().and_then(|p|p.get_mut("bloodMist")){p.owner[i]=-1.;p.player[i]=0.;}
            f.dirty=true;
        }
    }
    environment::refresh_gas_views(e);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn floor()->Floor{
        let mut f=Floor::default();f.width=7.0.into();f.height=7.0.into();f.depth=1.;f.tiles=vec!["floor".into();49];
        environment::initialize(&mut f).unwrap();f
    }
    #[test] fn mist_adds_volume_and_expires_after_five_updates(){
        let mut f=floor();
        environment::add_gas(&mut f,24,"bloodMist",100.,-2.).unwrap();
        environment::add_gas(&mut f,24,"bloodMist",100.,-2.).unwrap();
        assert_eq!(f.environment.as_ref().unwrap().gas_volume.as_ref().unwrap()[24],200.);
        for _ in 0..4 {tick(&mut f).unwrap();}
        assert_eq!(f.environment.as_ref().unwrap().gas_type.as_ref().unwrap()[24].as_deref(),Some("bloodMist"));
        tick(&mut f).unwrap();
        assert!(!environment::gas_present(f.environment.as_ref().unwrap(),24));
    }
    #[test] fn diffused_mist_keeps_lifetime_across_save_load(){
        let mut f=floor();
        environment::add_gas(&mut f,24,"bloodMist",100.,-2.).unwrap();
        tick(&mut f).unwrap();
        environment::propagate_gas(&mut f,&mut 1234,&[]).unwrap();
        let e=f.environment.as_ref().unwrap();
        assert!(e.gas_type.as_ref().unwrap().iter().filter(|g|g.as_deref()==Some("bloodMist")).count()>1);
        for (i,g) in e.gas_type.as_ref().unwrap().iter().enumerate(){if g.as_deref()==Some("bloodMist"){assert_eq!(e.blood_mist_turns.as_ref().unwrap()[i],4.);}}
        let data=serde_json::to_string(&f).unwrap();
        let mut f:Floor=serde_json::from_str(&data).unwrap();
        for _ in 0..4 {tick(&mut f).unwrap();environment::propagate_gas(&mut f,&mut 1234,&[]).unwrap();}
        assert!(!f.environment.as_ref().unwrap().gas_type.as_ref().unwrap().iter().any(|g|g.as_deref()==Some("bloodMist")));
    }
    #[test] fn actual_rescue_emits_and_repeated_uses_add_volume() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();
        r.player.abilities=vec![crate::divine_rescues::OVERFLOWING.into()];
        r.player.blood=2.0.into();r.player.hp=0.;
        let id=r.player.id;let i=r.player.y as usize*r.floor().unwrap().width()+r.player.x as usize;
        assert!(crate::divine_rescues::try_rescue(&mut r,id,false));
        assert_eq!(r.floor().unwrap().environment.as_ref().unwrap().gas_volume.as_ref().unwrap()[i],100.);
        r.player.hp=0.;
        assert!(crate::divine_rescues::try_rescue(&mut r,id,false));
        assert_eq!(r.floor().unwrap().environment.as_ref().unwrap().gas_volume.as_ref().unwrap()[i],200.);
        assert_eq!(r.player.blood.value_or(0.),0.);
        let restored:Run=serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!(restored.floor().unwrap().environment.as_ref().unwrap().blood_mist_turns.as_ref().unwrap()[i],5.);
    }
    #[test] fn emission_does_not_erase_existing_gas() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();
        let id=r.player.id;let i=r.player.y as usize*r.floor().unwrap().width()+r.player.x as usize;
        environment::add_gas(r.floor_mut().unwrap(),i,"paralysis",100.,-1.).unwrap();
        emit(&mut r,id);
        let e=r.floor().unwrap().environment.as_ref().unwrap();
        assert_eq!(e.gas_type.as_ref().unwrap()[i].as_deref(),Some("paralysis"));
        assert_eq!(e.gas_volume.as_ref().unwrap()[i],100.);
        assert_eq!(e.blood_mist.as_ref().unwrap().iter().sum::<f64>(),100.);
    }

    #[test] fn blood_mist_retains_normal_gas_flammability_without_explosion() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();
        let i=r.player.y as usize*r.floor().unwrap().width()+r.player.x as usize;
        environment::add_gas(r.floor_mut().unwrap(),i,"bloodMist",100.,-2.).unwrap();
        let mut fire=crate::fire_adapter::FireAdapter::new(&mut r,vec![]).unwrap();
        fire.ignite_creature(i,-1.,0.);
        assert!(fire.resolve().unwrap().is_empty());
        drop(fire);
        let e=r.floor().unwrap().environment.as_ref().unwrap();
        assert_eq!(e.gas_volume.as_ref().unwrap()[i],0.);
        assert!(environment::fire_active(e.fire_kind.as_ref().unwrap()[i].as_deref()));
    }

}
