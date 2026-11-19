//! Detached persistent ability geometry. Range edges may occupy unknown space;
//! they carry no terrain, actor, light, or item information.
use corpse_core::{Run, vision};
use crate::render::{Painter, View};
use crate::perception_ui::Edge;
#[derive(Clone,Debug,Default,PartialEq,Eq)]
pub struct Overlays { pub entropy:Vec<Edge>, pub mind_read:Vec<Edge>, pub riptide:Vec<(usize,usize)>, pub threads:Vec<Thread> }
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct Thread { pub cells:Vec<(usize,usize)>, pub color:[u8;3] }
/// Portal metric setup O(H*K^3), then O(V*H*K + L*movement_probe) per
/// observation; O(V+K+L) storage. V floor area, H<=2 origins, L<=V.
/// Mind Read independently adds O(A+V*H) time, O(V) scratch/edges, no portals.
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
        out.entropy=perimeter(&inside,w,h);
    }
    if run.player.abilities.iter().any(|s|s=="numo-mind-read") {
        let radius=if run.player.active.iter().any(|s|s=="numo-mind-read"){20.}else{5.};
        // Match telepathic_for_owner exactly: Euclidean radius from sensing
        // origins, not portal distance or optical terrain visibility.
        let origins=vision::origins(run);
        let inside=(0..w*h).map(|i|origins.iter().any(|a|
            vision::distance(corpse_core::movement::Point{x:a.x as i32,y:a.y as i32},corpse_core::movement::Point{x:(i%w)as i32,y:(i/w)as i32})<=radius)).collect::<Vec<_>>();
        out.mind_read=perimeter(&inside,w,h);
    }
    // Two maintained player links; recompute endpoints after every actor move.
    // O(A + L) per link plus visible_actor, with L <= max(width,height).
    for (id,key,timer,color) in [
        ("morae-bonded-by-unseen-threads","bondedTarget",None,[255,48,64]),
        ("pelian-i-came-for-you","cameTarget",Some("cameTurns"),[255,211,64]),
    ] {
        if timer.map_or_else(||!run.player.active.iter().any(|s|s==id),|k|corpse_core::engine::condition(&run.player,k)<=0.) {continue;}
        let Some(a)=run.actors_at(run.depth).iter().find(|a|a.id==corpse_core::engine::condition(&run.player,key)&&a.hp>0.&&!a.dead.value_or(false)&&corpse_core::dreamwalking::same_plane(run,&run.player,a)) else {continue;};
        if !vision::visible_actor(run,a){continue;}
        let cells=line_cells((run.player.x as i32,run.player.y as i32),(a.x as i32,a.y as i32),w.max(h)+1).into_iter().filter(|&(x,y)|
            x<w&&y<h&&o.and_then(|o|o.cells.get(y*w+x)).is_some_and(|c|c.current&&c.tile.is_some())).collect();
        out.threads.push(Thread{cells,color});
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
/// Boundary of the union, with no internal seams. O(V) time and O(V) output.
fn perimeter(inside:&[bool],w:usize,h:usize)->Vec<Edge> {
    let mut out=Vec::new();
    for y in 0..h {for x in 0..w {if !inside[y*w+x]{continue;}
        if x==0||!inside[y*w+x-1]{out.push(Edge{x,y,vertical:true});}
        if x+1==w||!inside[y*w+x+1]{out.push(Edge{x:x+1,y,vertical:true});}
        if y==0||!inside[(y-1)*w+x]{out.push(Edge{x,y,vertical:false});}
        if y+1==h||!inside[(y+1)*w+x]{out.push(Edge{x,y:y+1,vertical:false});}
    }}
    out
}
/// Ordered Bresenham connection, never a gameplay path or visibility grant.
fn line_cells((mut x,mut y):(i32,i32),(tx,ty):(i32,i32),limit:usize)->Vec<(usize,usize)> {
    let dx=(tx-x).abs();let dy=-(ty-y).abs();
    if dx.max(-dy) as usize>limit{return vec![];}
    let sx=if x<tx{1}else{-1};let sy=if y<ty{1}else{-1};let mut error=dx+dy;let mut cells=Vec::new();
    loop {if x>=0&&y>=0{cells.push((x as usize,y as usize));}if x==tx&&y==ty{break;}
        let e=2*error;if e>=dy{error+=dy;x+=sx;}if e<=dx{error+=dx;y+=sy;}}
    cells
}
/// Draw only neighboring admitted cells, never bridge a hidden gap.
pub fn paint_thread(p:&mut Painter,layout:(i32,i32,i32,i32),cells:&[(usize,usize)],color:[u8;3],alpha:u8){
    let(cw,ch,ox,oy)=layout;
    for pair in cells.windows(2){let((ax,ay),(bx,by))=(pair[0],pair[1]);
        if ax.abs_diff(bx)>1||ay.abs_diff(by)>1{continue;}
        let(x0,y0)=(ox+ax as i32*cw+cw/2,oy+ay as i32*ch+ch/2);
        let(x1,y1)=(ox+bx as i32*cw+cw/2,oy+by as i32*ch+ch/2);
        p.faint_line(x0,y0,x1,y1,color,alpha);
    }
}
pub fn paint(p:&mut Painter,v:&View){
    let(cw,ch,ox,oy)=p.map_layout(v.width,v.height);let old=p.clip;
    p.clip=Some((ox,oy,cw*v.width as i32,ch*v.height as i32));
    for (edges,color) in [(&v.ability_overlays.entropy,[255,255,255]),
        (&v.ability_overlays.mind_read,crate::ability_ui::god_color(3))] {for e in edges {
        let x=ox+e.x as i32*cw-i32::from(e.x==v.width);
        let y=oy+e.y as i32*ch-i32::from(e.y==v.height);
        p.rect(x-1,y-1,if e.vertical{3}else{cw+3},if e.vertical{ch+3}else{3},[15,15,22]);
        p.rect(x,y,if e.vertical{2}else{cw+1},if e.vertical{ch+1}else{2},color);
    }}
    for pair in v.ability_overlays.riptide.windows(2){
        let((ax,ay),(bx,by))=(pair[0],pair[1]);
        let(x0,y0)=(ox+ax as i32*cw+cw/2,oy+ay as i32*ch+ch/2);
        let(x1,y1)=(ox+bx as i32*cw+cw/2,oy+by as i32*ch+ch/2);
        let n=(x1-x0).abs().max((y1-y0).abs()).max(1);
        for i in 0..=n {p.rect(x0+(x1-x0)*i/n,y0+(y1-y0)*i/n,2,2,[55,140,255]);}
    }
    for thread in &v.ability_overlays.threads {paint_thread(p,(cw,ch,ox,oy),&thread.cells,thread.color,85);}
    p.clip=old;
}
#[cfg(test)] mod tests {
    use super::*;
    fn run()->Run{serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap()}
    #[test] fn faint_thread_survives_small_window_and_does_not_bridge_gap(){
        let mut p=Painter::new();p.resize(960,600);p.set_responsive_canvas(true);
        let before=p.pixels.clone();
        paint_thread(&mut p,(16,16,100,100),&[(1,1),(2,1),(3,1)],[255,48,64],85);
        assert_ne!(p.pixels,before,"one-native-pixel thread must survive downscaling");
        p.pixels.copy_from_slice(&before);
        paint_thread(&mut p,(16,16,100,100),&[(1,1),(3,1)],[255,48,64],85);
        assert_eq!(p.pixels,before,"hidden middle tile must not be bridged");
    }
    #[test] fn links_track_motion_expire_and_do_not_bridge_unknown_cells(){
        let mut r=crate::non_euclidean_ui::tests::scene();r.simulation_revision=52.0.into();
        r.floor_mut().unwrap().non_euclidean=corpse_core::Field::Missing;
        r.player.x=10.;r.player.y=10.;
        let a=corpse_core::population::create_actor_revision("bear",901.,16.,10.,false,52.).unwrap();
        r.actors_at_mut(r.depth).push(a);
        r.player.active.push("morae-bonded-by-unseen-threads".into());
        r.player.conditions.insert("bondedTarget".into(),901.);
        let mut o=vision::observe_terrain(&r).unwrap();for c in &mut o.cells{c.current=true;c.tile=Some("floor".into());}
        let before=r.clone();let first=observe(&r,Some(&o));assert_eq!(first.threads[0].color,[255,48,64]);
        assert_eq!(first.threads[0].cells.first(),Some(&(10,10)));assert_eq!(first.threads[0].cells.last(),Some(&(16,10)));assert_eq!(r,before);
        r.player.x=11.;r.actors_at_mut(r.depth).last_mut().unwrap().y=11.;
        let moved=observe(&r,Some(&o));assert_eq!(moved.threads[0].cells.first(),Some(&(11,10)));assert_eq!(moved.threads[0].cells.last(),Some(&(16,11)));
        assert!(observe(&r,None).threads[0].cells.is_empty());
        let w=r.floor().unwrap().width();o.cells[10*w+12].current=false;
        assert!(!observe(&r,Some(&o)).threads[0].cells.contains(&(12,10)));
        r.player.active.clear();assert!(observe(&r,Some(&o)).threads.is_empty());
        r.player.conditions.insert("cameTarget".into(),901.);r.player.conditions.insert("cameTurns".into(),1.);
        assert_eq!(observe(&r,Some(&o)).threads[0].color,[255,211,64]);
        r.player.conditions.insert("cameTurns".into(),0.);assert!(observe(&r,Some(&o)).threads.is_empty());
        r.player.active.push("morae-bonded-by-unseen-threads".into());r.actors_at_mut(r.depth).last_mut().unwrap().hp=0.;assert!(observe(&r,Some(&o)).threads.is_empty());
    }
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

    fn mind_run()->Run {
        let mut r=run();r.simulation_revision=52.0.into();
        r.player.abilities=vec!["numo-mind-read".into()];r.player.active.clear();r.player.conditions.clear();
        r.actors.clear();r
    }
    #[test] fn mind_read_outline_matches_actual_telepathy_in_both_states_and_half_here() {
        let mut r=mind_run();
        for clone in [false,true] {for active in [false,true] {
            r.player.active.clear();if active {r.player.active.push("numo-mind-read".into());}
            if clone {
                r.player.abilities.push("etill-half-here".into());r.player.active.push("etill-half-here".into());
                r.player.conditions.insert("halfHereId".into(),900.);
                let mut a=r.player.clone();a.id=900.;a.x+=12.;
                a.conditions.insert("halfHereClone".into(),1.);a.conditions.insert("halfHereOwner".into(),r.player.id);
                r.actors_at_mut(r.depth).clear();r.actors_at_mut(r.depth).push(a);
            }
            let f=r.floor().unwrap();let(w,h)=(f.width(),f.height());
            let inside=(0..w*h).map(|i|{
                let target=corpse_core::Actor{id:999.,x:(i%w)as f64,y:(i/w)as f64,hp:1.,..Default::default()};
                vision::telepathic(&r,&target)
            }).collect::<Vec<_>>();
            let before=r.clone();
            assert_eq!(observe(&r,None).mind_read,perimeter(&inside,w,h));assert_eq!(r,before);
        }}
        r.player.abilities.retain(|s|s!="numo-mind-read");
        assert!(observe(&r,None).mind_read.is_empty(),"active-list residue alone does not grant Mind Read");
    }
    #[test] fn mind_read_outline_does_not_leak_hidden_terrain_or_portals() {
        let mut r=mind_run();let before=observe(&r,None).mind_read;
        let f=r.floor_mut().unwrap();let w=f.width();f.tiles.fill("wall".into());f.lights.fill(-100.);f.seen.fill(false);
        f.portals=vec![corpse_core::portals_generation::PortalPair{a:14*w+13,b:14*w+40,color_index:0}].into();
        assert_eq!(observe(&r,None).mind_read,before);
        let o=vision::observe_terrain(&r).unwrap();assert_eq!(observe(&r,Some(&o)).mind_read,before);
        r.player.x+=1.;assert_ne!(observe(&r,None).mind_read,before);
        r.player.active.push("numo-mind-read".into());assert_ne!(observe(&r,None).mind_read,before);
    }
    #[test] fn mind_read_outline_uses_numo_ink_and_same_entropy_stroke() {
        let r=mind_run();let mut v=crate::observe::observe(&r);
        let mut p=Painter::new();p.resize(1280,800);paint(&mut p,&v);
        let green=crate::ability_ui::god_color(3);
        let green_mask=p.pixels.chunks_exact(4).map(|c|c[..3]==green).collect::<Vec<_>>();
        assert!(green_mask.iter().any(|&v|v));
        v.ability_overlays.entropy=std::mem::take(&mut v.ability_overlays.mind_read);
        let mut white=Painter::new();white.resize(1280,800);paint(&mut white,&v);
        assert_eq!(green_mask,white.pixels.chunks_exact(4).map(|c|c[..3]==[255,255,255]).collect::<Vec<_>>());
    }

}
