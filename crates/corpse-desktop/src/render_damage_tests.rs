//! Byte oracles are deliberately outside measured rendering intervals.
use super::*;
fn view()->View {
    let r:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    crate::observe::observe(&r)
}
fn assert_pixels(a:&Painter,b:&Painter,stage:&str){
    if let Some((i,(x,y)))=a.pixels.iter().zip(&b.pixels).enumerate().find(|(_, (a,b))|a!=b){panic!("{stage}: byte {i}, retained {x}, full {y}");}
    assert_eq!(a.pixels.len(),b.pixels.len());
}
#[test]
fn retained_base_matches_full_fixed_clock_hover_world_and_resize(){
    let mut v=view();let mut p=Painter::new();let mut full=Painter::new();let mut retained=RetainedGame::default();
    p.raster_stats.enabled=true;
    for step in 0..11 {
        let mut changed=false;let mut force=false;let mut now=0.;
        match step {
            1=>{p.chronicle_hover=true;full.chronicle_hover=true;},
            2=>{p.chronicle_hover=false;full.chronicle_hover=false;p.ability_hover=Some(0);full.ability_hover=Some(0);},
            3=>{p.ability_hover=None;full.ability_hover=None;v.hp-=1.;changed=true;},
            4=>{let c=v.cells.last_mut().unwrap();c.fg=[9,240,190];c.x=c.x.saturating_sub(1);changed=true;},
            5=>{v.hover=Some((v.cells[0].x,v.cells[0].y));changed=true;},
            6=>{v.path=vec![(v.cells[1].x,v.cells[1].y)];changed=true;},
            7=>{now=0.4;},
            8=>{p.resize(1920,1200);full.resize(1920,1200);force=true;},
            9=>{v.log_groups.push(crate::chronicle::Group{message:"A new visible event".into(),kind:"".into(),count:1,index:0});changed=true;},
            10=>{v.hover=None;v.path.clear();changed=true;},_=>{}
        }
        retained.update(&mut p,&v,"oracle",now,changed,force).unwrap();
        full.game_at(&v,"oracle",false,now);assert_pixels(&p,&full,&format!("step {step}"));
    }
    p.raster_stats=RasterStats{enabled:true,..Default::default()};
    let d=retained.update(&mut p,&v,"oracle",0.,false,false).unwrap();
    assert!(d.is_empty());assert_eq!(p.raster_stats.pixel_writes,0);assert_eq!(d.upload_bytes(),0);
}
#[test]
fn independent_damage_clip_survives_nested_clip_and_clear(){
    let mut p=Painter::new();p.clear([3,4,5]);let before=p.pixels.clone();
    let d=Rect::new(80,60,40,30);p.damage_clip=Some(d);
    p.clip=Some((0,0,1280,800));p.clear([7,8,9]);p.rect(0,0,1280,800,[30,40,50]);p.clip=None;
    p.text("Wide antialiased test",0,50,40,[200,150,100]);
    for y in 0..p.height{for x in 0..p.width{if !d.contains(x,y){let i=(y*p.width+x)*4;assert_eq!(&p.pixels[i..i+4],&before[i..i+4]);}}}
}
#[test]
fn transparent_overlay_move_and_close_restore_clean_base(){
    let v=view();let mut p=Painter::new();let mut full=Painter::new();let mut retained=RetainedGame::default();
    retained.update(&mut p,&v,"",0.,true,true).unwrap();
    let overlays=[Some(Rect::new(100,180,270,150)),Some(Rect::new(170,210,270,150)),None];let mut old=None;
    for next in overlays {
        let mut d=Damage::empty(p.width,p.height);if let Some(r)=old{d.add(r);}if let Some(r)=next{d.add(r);}
        retained.restore(&mut p,&d).unwrap();
        for r in d.rects(){p.damage_clip=Some(*r);if let Some(o)=next{p.blend_rect(o.x as i32,o.y as i32,o.w as i32,o.h as i32,[22,40,65],190);p.text("overlay",o.x as i32+10,o.y as i32+10,18,[240,240,240]);}}
        p.damage_clip=None;
        full.game_at(&v,"",false,0.);if let Some(o)=next{full.blend_rect(o.x as i32,o.y as i32,o.w as i32,o.h as i32,[22,40,65],190);full.text("overlay",o.x as i32+10,o.y as i32+10,18,[240,240,240]);}
        assert_pixels(&p,&full,"overlay composition");old=next;
    }
}
#[test]
fn cached_sparse_and_dense_fire_and_dream_are_pixel_exact(){
    for dense in [false,true] {
        let mut v=view();
        for(i,c)in v.cells.iter_mut().enumerate(){
            c.animations.clear();c.pulse_speed=0.;
            if dense||i==0{c.animations.push(super::super::Animation::Fire{phase:0.17+i as f64/100.,speed:1.,colors:[[255,40,0],[255,140,20],[255,220,80],[255,110,0]]});}
        }
        let mut p=Painter::new();let mut full=Painter::new();let mut retained=RetainedGame::default();
        for(t,dream)in [(0.,false),(180.,false),(420.,false),(0.,false),(0.,true),(230.,true)]{
            let changed=v.dreamwalking!=dream;v.dreamwalking=dream;
            retained.update(&mut p,&v,"",t,changed,false).unwrap();full.game_at(&v,"",false,t);
            assert_pixels(&p,&full,if dense{"dense fire/dream"}else{"sparse fire/dream"});
        }
    }
}
#[test]
fn overlapping_global_cell_order_and_passive_mode_invalidate(){
    let mut v=view();let mut p=Painter::new();let mut full=Painter::new();let mut retained=RetainedGame::default();
    // Explicit overhanging adjacent layers, unchanged per-coordinate stacks after swap.
    let mut a=v.cells[0].clone();a.x=12;a.y=14;a.emphasis=3.;a.glyph="X".into();a.fg=[240,20,20];
    let mut b=a.clone();b.x=13;b.glyph="O".into();b.fg=[20,240,20];v.cells.push(a);v.cells.push(b);
    retained.update(&mut p,&v,"",0.,true,false).unwrap();full.game_at(&v,"",false,0.);assert_pixels(&p,&full,"before order");
    let n=v.cells.len();v.cells.swap(n-1,n-2);
    if let Some(a)=v.abilities.first_mut(){a.active=false;a.supported=true;a.mode="passive".into();a.state="READY".into();}
    retained.update(&mut p,&v,"",0.,true,false).unwrap();full.game_at(&v,"",false,0.);assert_pixels(&p,&full,"order and passive mode");
}
