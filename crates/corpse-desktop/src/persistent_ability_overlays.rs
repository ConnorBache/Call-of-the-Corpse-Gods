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
    // O(A + L) per link plus visible_actor and the optical route when portals exist.
    for (id,key,timer,color) in [
        ("morae-bonded-by-unseen-threads","bondedTarget",None,[255,48,64]),
        ("pelian-i-came-for-you","cameTarget",Some("cameTurns"),[255,211,64]),
    ] {
        if timer.map_or_else(||!run.player.active.iter().any(|s|s==id),|k|corpse_core::engine::condition(&run.player,k)<=0.) {continue;}
        let Some(a)=run.actors_at(run.depth).iter().find(|a|a.id==corpse_core::engine::condition(&run.player,key)&&a.hp>0.&&!a.dead.value_or(false)&&corpse_core::dreamwalking::same_plane(run,&run.player,a)) else {continue;};
        if !vision::visible_actor(run,a){continue;}
        let cells=corpse_core::animation_shapes::connection_cells(run,&run.player,a.into(),false)
            .into_iter().map(|p|(p.x as usize,p.y as usize)).filter(|&(x,y)|
            x<w&&y<h&&o.and_then(|o|o.cells.get(y*w+x)).is_some_and(|c|c.current&&c.tile.is_some())).collect();
        out.threads.push(Thread{cells,color});
    }
    if !run.player.active.iter().any(|s|s=="etill-riptide"){return out;}
    let c=corpse_core::engine::condition;
    let Some(mut a)=run.actors_at(run.depth).iter().find(|a|a.id==c(&run.player,"riptideTarget")&&!a.dead.value_or(false)&&a.hp>0.&&corpse_core::dreamwalking::same_plane(run,&run.player,a)).cloned() else{return out;};
    let known=|x:f64,y:f64|corpse_core::portals::cell(f,x,y).is_some_and(|i|o.and_then(|o|o.cells.get(i)).is_some_and(|c|c.current&&c.tile.is_some()));
    if !known(a.x,a.y)||!vision::visible_actor(run,&a){return out;}
    if corpse_core::revision50_terrain::is_soldier(&a)||corpse_core::revision50_nightmares::is_decoy(&a)||corpse_core::ahvamitt::is_actor(&a)||corpse_core::dreamwalking::is_body(&a){return out;}
    let Some((goal_x, goal_y)) = corpse_core::riptide_rally::goal(run, &run.player) else { return out; };
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
const MIND_READ_PERIOD_MS: f64 = 2400.;

/// Preserve Numo's green hue. Average stroke brightness is 35% with a
/// 24%–46% breathing range; the dark rim stays still for legibility.
pub fn mind_read_ink(now: f64) -> [u8; 3] {
    let phase = now.rem_euclid(MIND_READ_PERIOD_MS) / MIND_READ_PERIOD_MS;
    let gain = 0.35 + 0.11 * (std::f64::consts::TAU * phase).sin();
    crate::ability_ui::god_color(3).map(|channel| (f64::from(channel) * gain).round() as u8)
}

fn edge_rect(e: &Edge, (cw, ch, ox, oy): (i32, i32, i32, i32), w: usize, h: usize) -> (i32, i32, i32, i32) {
    let x = ox + e.x as i32 * cw - i32::from(e.x == w);
    let y = oy + e.y as i32 * ch - i32::from(e.y == h);
    (x, y, if e.vertical { 2 } else { cw + 1 }, if e.vertical { ch + 1 } else { 2 })
}

/// Only the inner stroke changes with time. This leaves the retained idle
/// redraw proportional to outline length instead of invalidating the map.
pub fn mark_mind_read(p: &Painter, v: &View, mut mark: impl FnMut(crate::render::damage::Rect)) {
    if v.ability_overlays.mind_read.is_empty() { return; }
    let layout = p.map_layout(v.width, v.height);
    for edge in &v.ability_overlays.mind_read {
        let (x, y, w, h) = edge_rect(edge, layout, v.width, v.height);
        if let Some(rect) = p.logical_damage_rect(x, y, w, h) { mark(rect); }
    }
}

pub fn paint(p:&mut Painter,v:&View,now:f64){
    let layout = p.map_layout(v.width, v.height);
    let(cw,ch,ox,oy)=layout;let old=p.clip;
    p.clip=Some((ox,oy,cw*v.width as i32,ch*v.height as i32));
    for (edges,color) in [(&v.ability_overlays.entropy,[255,255,255]),
        (&v.ability_overlays.mind_read,mind_read_ink(now))] {for e in edges {
        let (x,y,w,h)=edge_rect(e,layout,v.width,v.height);
        p.rect(x-1,y-1,w+1+i32::from(!e.vertical),h+1+i32::from(e.vertical),[15,15,22]);
        p.rect(x,y,w,h,color);
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

    #[test]
    fn riptide_overlay_switches_direction_with_the_current_flag() {
        let mut r = crate::non_euclidean_ui::tests::scene();
        r.simulation_revision = 52.0.into();
        r.riptide_rally_from = 0usize.into();
        r.floor_mut().unwrap().non_euclidean = corpse_core::Field::Missing;
        let a = corpse_core::population::create_actor_revision("bear", 901., 12., 10., false, 52.).unwrap();
        r.actors_at_mut(r.depth).push(a);
        r.player.active.push("etill-riptide".into());
        r.player.conditions.insert("riptideTarget".into(), 901.);
        let mut o = vision::observe_terrain(&r).unwrap();
        for c in &mut o.cells { c.current = true; c.tile = Some("floor".into()); }
        assert!(observe(&r, Some(&o)).riptide.is_empty());
        r.rally = Some(corpse_core::RunRally { x: 16., y: 10. });
        assert_eq!(observe(&r, Some(&o)).riptide.last(), Some(&(16,10)));
        r.rally = Some(corpse_core::RunRally { x: 12., y: 14. });
        assert_eq!(observe(&r, Some(&o)).riptide.last(), Some(&(12,14)));
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
        let mut p=Painter::new();p.resize(1280,800);paint(&mut p,&v,0.);
        let green=mind_read_ink(0.);
        let green_mask=p.pixels.chunks_exact(4).map(|c|c[..3]==green).collect::<Vec<_>>();
        assert!(green_mask.iter().any(|&v|v));
        v.ability_overlays.entropy=std::mem::take(&mut v.ability_overlays.mind_read);
        let mut white=Painter::new();white.resize(1280,800);paint(&mut white,&v,0.);
        assert_eq!(green_mask,white.pixels.chunks_exact(4).map(|c|c[..3]==[255,255,255]).collect::<Vec<_>>());
    }

    #[test]
    fn mind_read_pulse_is_dim_continuous_and_periodic() {
        let green = crate::ability_ui::god_color(3);
        let frames: Vec<_> = (0..240).map(|i| mind_read_ink(f64::from(i) * 10.)).collect();
        for channel in 0..3 {
            assert!(frames.iter().all(|ink| ink[channel] <= (f64::from(green[channel]) * 0.46).round() as u8));
            let mean = frames.iter().map(|ink| f64::from(ink[channel])).sum::<f64>() / frames.len() as f64;
            assert!((mean / f64::from(green[channel]) - 0.35).abs() < 0.01);
            assert!(frames.windows(2).all(|pair| pair[0][channel].abs_diff(pair[1][channel]) <= 1));
        }
        assert_ne!(mind_read_ink(600.), mind_read_ink(1800.));
        assert_eq!(mind_read_ink(0.), mind_read_ink(MIND_READ_PERIOD_MS));
    }

    #[test]
    fn mind_read_idle_updates_both_retained_painters_without_world_refresh() {
        let r = mind_run();
        let mut view = crate::observe::observe(&r);
        for cell in &mut view.cells {
            cell.animations.clear();
            cell.pulse_speed = 0.;
            cell.fire_flicker = false;
        }
        for (width,height) in [(960,600), (1280,800)] {
            let painter = || {
                let mut p = Painter::new();
                p.resize(width,height);
                p.set_responsive_canvas(true);
                p
            };
            let mut tiled = painter();
            let mut tile = crate::render::tile_damage::TilePlanner::default();
            tile.replace_view(None,&view,&tiled);
            let mut retained = painter();
            let mut regions = crate::render::damage::RetainedGame::default();
            let mut full = painter();
            let mut before = Vec::new();
            for (i, now) in [0.,600.,1800.,0.].into_iter().enumerate() {
                tile.mark_animation(&view,&tiled,now,false);
                tile.finish_damage(&mut tiled);
                assert!(!tiled.tile_mask.is_empty());
                if i > 0 { assert!(tiled.tile_mask.area() < width*height/10, "do not repaint the full dungeon"); }
                tile.paint_base(&mut tiled,&view,"",now);
                tiled.tile_clip_enabled=false;
                tile.clear_frame();
                regions.update(&mut retained,&view,"",now,i==0,i==0).unwrap();
                full.game_at(&view,"",false,now);
                assert!(full.pixels == tiled.pixels, "tile frame differs at {width}x{height}, {now}ms");
                assert!(full.pixels == retained.pixels, "region frame differs at {width}x{height}, {now}ms");
                if i > 0 { assert!(full.pixels != before, "idle paint should change the stroke"); }
                before.clone_from(&full.pixels);
            }
            tile.mark_animation(&view,&tiled,2000.,true);
            tile.finish_damage(&mut tiled);
            assert!(tiled.tile_mask.is_empty(), "reduced motion stays at the dim steady color");
        }
    }

    #[test]
    #[ignore = "Native in-game painter capture requires ffmpeg and output path"]
    fn capture_mind_read_range_pulse() {
        use std::{io::Write, process::{Command as Process, Stdio}};
        let output = std::env::var("CORPSE_MIND_READ_VIDEO").expect("video output");
        let mut r = mind_run();
        r.inventory.clear();
        r.ground.clear();
        for depth in r.floors.iter().map(|f| f.depth).collect::<Vec<_>>() {
            r.actors_at_mut(depth);
            r.ground_at_mut(depth);
        }
        r.offers.clear();
        r.history.clear();
        r.replay.clear();
        corpse_core::revision50_ability_energy::initialize_reserve(
            &mut r.player, "numo-mind-read", 10., 10.,
        ).unwrap();
        r.player.x = 36.;
        r.player.y = 14.;
        let rat = corpse_core::population::create_actor_revision("rat", 900., 41., 14., true, 52.).unwrap();
        r.actors_at_mut(1.).push(rat);
        r.next_id = r.next_id.max(901.);
        let f = r.floor_mut().unwrap();
        f.tiles.fill("floor".into());
        f.lights.fill(50.);
        f.seen.fill(true);
        f.non_euclidean = corpse_core::Field::Missing;
        f.portals = corpse_core::Field::Missing;
        f.doors = corpse_core::Field::Missing;
        f.environment = corpse_core::Field::Missing;
        let mut engine = corpse_core::temporal::Engine::new(r).unwrap();
        let mut p = Painter::new();
        p.resize(960,600);
        p.set_responsive_canvas(true);
        let mut v = crate::observe::observe_with_profile(&engine.run,&p.profile);
        let mut fx = crate::effects::Effects::default();
        let mut encoder = Process::new("ffmpeg").args([
            "-hide_banner","-loglevel","error","-y","-f","rawvideo","-pix_fmt","rgba","-s","960x600",
            "-r","30","-i","-","-an","-c:v","libx264","-preset","fast","-crf","16","-pix_fmt","yuv420p",
            "-movflags","+faststart",&output,
        ]).stdin(Stdio::piped()).spawn().unwrap();
        let mut sink = encoder.stdin.take().unwrap();
        for i in 0..198 {
            let now = f64::from(i)*1000./30.;
            if i==90 {
                let mut after = engine.dispatch(&corpse_core::Command::Ability {
                    id:"numo-mind-read".into(), target:corpse_core::Field::Missing,
                    x:corpse_core::Field::Missing, y:corpse_core::Field::Missing,
                }).unwrap();
                assert!(engine.run.player.active.iter().any(|s|s=="numo-mind-read"), "{}", engine.run.log.join("\n"));
                fx.add(std::mem::take(&mut after.events),engine.run.tick,engine.run.depth,now,false);
                let active = crate::observe::observe_with_profile(&engine.run,&p.profile);
                assert_ne!(active.ability_overlays.mind_read,v.ability_overlays.mind_read);
                v = active;
            }
            fx.prune(now);
            p.game_at(&v,"",false,now);
            p.effects(&v,&fx.cues,now,false);
            sink.write_all(&p.pixels).unwrap();
        }
        drop(sink);
        assert!(encoder.wait().unwrap().success());
    }

}
