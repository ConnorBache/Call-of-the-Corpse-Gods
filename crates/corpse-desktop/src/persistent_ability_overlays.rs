//! Detached ability geometry. White entropy edges are explicitly allowed in
//! unknown space; they carry no terrain, actor, light, or item information.
use corpse_core::{Run, vision};
use crate::render::{Painter, View};
use crate::perception_ui::Edge;
#[derive(Clone,Debug,Default,PartialEq,Eq)]
pub struct Overlays { pub entropy:Vec<Edge>, pub riptide:Vec<(usize,usize)> }
/// Portal metric setup O(H*K^3), then O(V*H*K + L*movement_probe) per
/// observation; O(V+K+L) storage. V floor area, H<=2 origins, L<=V.
pub fn observe(run:&Run,o:Option<&vision::TerrainObservation>)->Overlays {
    let Some(f)=corpse_core::dreamwalking::floor_for(run,&run.player) else{return Overlays::default()};
    let(w,h)=(f.width(),f.height());
    let mut out=Overlays::default();
    if run.player.active.iter().any(|s|s=="dawel-reverse-entropy") {
        let origins=corpse_core::half_here::origins(run);
        // Any shortest unobstructed portal-metric route ends in a straight
        // segment from either the origin or a portal endpoint. Compute endpoint
        // distances once, not a complete graph search separately for every tile.
        let mut anchors=Vec::new();
        for a in origins.iter().filter(|a|corpse_core::dreamwalking::same_plane(run,&run.player,a)) {
            anchors.push((a.x,a.y,0.));
            if corpse_core::portals::enabled(run) {
                let mut endpoints=std::collections::HashSet::new();
                for (p,q) in corpse_core::portals::links(f){endpoints.insert(p);endpoints.insert(q);}
                for p in endpoints {let(x,y)=((p%w)as f64,(p/w)as f64);
                    let d=corpse_core::portals::point_distance(run,a,x,y);
                    if d<=10.{anchors.push((x,y,d));}
                }
            }
        }
        let inside=(0..w*h).map(|i|anchors.iter().any(|&(x,y,d)|
            d+((i%w)as f64-x).hypot((i/w)as f64-y)<=10.)).collect::<Vec<_>>();
        for y in 0..h {for x in 0..w {if !inside[y*w+x]{continue;}
            if x==0||!inside[y*w+x-1]{out.entropy.push(Edge{x,y,vertical:true});}
            if x+1==w||!inside[y*w+x+1]{out.entropy.push(Edge{x:x+1,y,vertical:true});}
            if y==0||!inside[(y-1)*w+x]{out.entropy.push(Edge{x,y,vertical:false});}
            if y+1==h||!inside[(y+1)*w+x]{out.entropy.push(Edge{x,y:y+1,vertical:false});}
        }}
    }
    if !run.player.active.iter().any(|s|s=="etill-riptide"){return out;}
    let c=corpse_core::engine::condition;
    let Some(mut a)=run.actors_at(run.depth).iter().find(|a|a.id==c(&run.player,"riptideTarget")&&!a.dead.value_or(false)&&a.hp>0.&&corpse_core::dreamwalking::same_plane(run,&run.player,a)).cloned() else{return out;};
    let known=|x:f64,y:f64|corpse_core::portals::cell(f,x,y).is_some_and(|i|o.and_then(|o|o.cells.get(i)).is_some_and(|c|c.current&&c.tile.is_some()));
    if !known(a.x,a.y)||!vision::visible_actor(run,&a){return out;}
    if corpse_core::revision50_terrain::is_soldier(&a)||corpse_core::revision50_nightmares::is_decoy(&a)||corpse_core::ahvamitt::is_actor(&a)||corpse_core::dreamwalking::is_body(&a){return out;}
    let(goal_x,goal_y)=(c(&run.player,"riptideX"),c(&run.player,"riptideY"));
    let mut seen=std::collections::HashSet::new();
    for _ in 0..w*h {
        if !seen.insert((a.x as usize,a.y as usize)){break;}
        out.riptide.push((a.x as usize,a.y as usize));
        let(dx,dy)=corpse_core::portals::direction_toward(run,&a,goal_x,goal_y);
        if dx==0.&&dy==0.{break;}
        let(ex,ey)=(a.x+dx,a.y+dy);
        if !known(ex,ey)||!corpse_core::engine::actor_passable(run,&a,ex,ey){break;}
        let options=corpse_core::portals::step_options(run,&a,ex,ey);
        // Never predict a random portal destination as certain, or consume RNG.
        if options.len()!=1{break;}
        let(x,y)=options[0];
        let Some(at)=corpse_core::portals::cell(f,x,y) else{break;};
        if !known(x,y)||!corpse_core::engine::actor_passable(run,&a,x,y)
            ||corpse_core::engine::bulwark_blocks(run,&a,x,y)
            ||!corpse_core::ichor::movement_allowed(run,&a,x,y)
            ||corpse_core::portals::occupied_at(run,&a,x,y,Some(a.id)).is_some()
            ||f.doors.as_ref().and_then(|d|d.get(&at.to_string())).is_some_and(|d|!d.open)
            ||(f.tiles[at]=="chasm"&&!corpse_core::engine::flying_in(run,&a)){break;}
        // A portal is a discontinuity, not a fictitious line across its gap.
        if (x-ex).abs()>1.||(y-ey).abs()>1.{break;}
        a.x=x;a.y=y;
    }
    out
}
pub fn paint(p:&mut Painter,v:&View){
    let(cw,ch,ox,oy)=p.map_layout(v.width,v.height);let old=p.clip;
    p.clip=Some((ox,oy,cw*v.width as i32,ch*v.height as i32));
    for e in &v.ability_overlays.entropy {
        let x=ox+e.x as i32*cw-i32::from(e.x==v.width);
        let y=oy+e.y as i32*ch-i32::from(e.y==v.height);
        p.rect(x-1,y-1,if e.vertical{3}else{cw+3},if e.vertical{ch+3}else{3},[15,15,22]);
        p.rect(x,y,if e.vertical{2}else{cw+1},if e.vertical{ch+1}else{2},[255,255,255]);
    }
    for pair in v.ability_overlays.riptide.windows(2){
        let((ax,ay),(bx,by))=(pair[0],pair[1]);
        let(x0,y0)=(ox+ax as i32*cw+cw/2,oy+ay as i32*ch+ch/2);
        let(x1,y1)=(ox+bx as i32*cw+cw/2,oy+by as i32*ch+ch/2);
        let n=(x1-x0).abs().max((y1-y0).abs()).max(1);
        for i in 0..=n {p.rect(x0+(x1-x0)*i/n,y0+(y1-y0)*i/n,2,2,[55,140,255]);}
    }
    p.clip=old;
}
#[cfg(test)] mod tests {
    use super::*;
    fn run()->Run{serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap()}
    #[test] fn entropy_visible_without_observation_and_removed_on_toggle(){
        let mut r=run();r.player.active.push("dawel-reverse-entropy".into());
        assert!(!observe(&r,None).entropy.is_empty());
        let before=observe(&r,None);r.floor_mut().unwrap().tiles[0]="wall".into();
        assert_eq!(before,observe(&r,None));
        r.player.active.clear();assert_eq!(observe(&r,None),Overlays::default());
    }
    #[test] fn riptide_never_discloses_unobserved_target(){
        let mut r=run();r.player.active.push("etill-riptide".into());
        assert!(observe(&r,None).riptide.is_empty());
    }
    #[test] fn riptide_tracks_forced_steps_stops_at_wall_and_clears(){
        let mut r=crate::non_euclidean_ui::tests::scene();r.simulation_revision=52.0.into();
        r.floor_mut().unwrap().non_euclidean=corpse_core::Field::Missing;
        let mut a=corpse_core::population::create_actor_revision("bear",901.,12.,10.,false,52.).unwrap();
        a.abilities.clear();a.active.clear();r.actors_at_mut(r.depth).push(a);
        r.player.active.push("etill-riptide".into());
        for(k,v)in [("riptideTarget",901.),("riptideX",16.),("riptideY",10.)]{r.player.conditions.insert(k.into(),v);}
        let mut o=vision::observe_terrain(&r).unwrap();for c in &mut o.cells {c.current=true;c.tile=Some("floor".into());}
        let original=r.clone();assert_eq!(observe(&r,Some(&o)).riptide,vec![(12,10),(13,10),(14,10),(15,10),(16,10)]);
        assert_eq!(r,original,"projection must not advance RNG, resources or simulation");
        let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[10*w+15]="wall".into();
        assert_eq!(observe(&r,Some(&o)).riptide,vec![(12,10),(13,10),(14,10)]);
        corpse_core::divine_environment::apply_riptide(&mut r).unwrap();
        assert_eq!(observe(&r,Some(&o)).riptide,vec![(13,10),(14,10)]);
        r.player.active.clear();assert!(observe(&r,Some(&o)).riptide.is_empty());
    }

}
