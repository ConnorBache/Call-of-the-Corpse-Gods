//! Native coverage regressions and optional before/after capture board.
use super::*;
#[test]
fn native_glyph_coverage_boards() {
 let dir=match std::env::var("CORPSE_GLYPH_COVERAGE_OUTPUT") { Ok(dir)=>dir, Err(_)=>return };std::fs::create_dir_all(&dir).unwrap();
 let samples=[("Tree / Veridian","♣"),("Chasm","⛶"),("Pit","○"),("Fire","人"),("Embers (fixed)","·"),("Bulwark ward","∷"),("Surface entrance","Ω"),("Fungal root vertical","│"),("Fungal root horizontal","─"),("Fungal root diagonal","╲"),("Fungal root diagonal","╱"),("Fungal root junction","┼"),("Borge centre","┃"),("Borge side","█"),("Borge altar","π"),("Wall-self actor","▓"),("Tree tap item","¬"),("Arrows item","→"),("Legacy Epidite burst","✧"),("Potion control","!"),("Melee weapon control","{"),("Bow control",")"),("Armour control","["),("Ring control","o"),("Amulet control","?"),("Rat control","r")];
 for (width,height) in [(1280,800),(1920,1200),(2560,1600)] {
  let mut p=Painter::new();p.resize(width,height);p.clear([12,17,25]); println!("MAP_LAYOUT {width}x{height} {:?}",p.map_layout(79,29));
  p.text("CURRENT GLYPH RENDERER - actual native Painter, default appearance",24,12,20,[242,235,220]);
  p.text("Each row: 9x14, 15x24, 30x48 logical cells. White ink isolates edge coverage.",24,38,14,[170,180,196]);
  for (i,(label,glyph)) in samples.iter().enumerate(){let ox=24+(i/13)as i32*630;let y=83+(i%13)as i32*53;
   p.text(label,ox,y+10,14,[190,199,213]);
   for(j,(w,h))in [(9,14),(15,24),(30,48)].into_iter().enumerate(){let x=ox+265+j as i32*100;p.border(x-2,y-2,w+4,h+4,[49,62,78]);p.map_identity(glyph,x,y,w,h,[255,255,255]);
    let code=glyph.chars().map(|c|format!("{:04X}",c as u32)).collect::<Vec<_>>().join("_");
    for(k,(dx,dy,mw,mh,mask,_))in p.identity_masks.iter().filter(|(k,_)|&k.0==glyph){
     if k.1==(w as f64*p.scale).round()as i32&&k.2==(h as f64*p.scale).round()as i32{
      let path=format!("{dir}/native-{code}_{w}x{h}_{}x{}.pgm",width,height);let mut b=format!("P5\n{mw} {mh}\n255\n").into_bytes();b.extend_from_slice(mask);std::fs::write(path,b).unwrap();
      println!("MASK {width}x{height} {label} U+{code} cell {w}x{h} dest {mw}x{mh} at {dx},{dy}");
     }
    }
   }
  }
  let f=std::fs::File::create(format!("{dir}/native-glyph-board-{width}x{height}.png")).unwrap();let mut encoder=png::Encoder::new(f,width as u32,height as u32);encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
 }

}

const SYMBOLS: &[&str] = &["¬","○","→","♣","∷","π","人","⛶","Ω","│","─","╲","╱","┼","┃","█","▓","(≬)","10"];

#[test]
fn native_coverage_masks_fit_fractional_sizes_and_stay_cached() {
    let mut p=Painter::new();
    for (width,height) in [(960,600),(1280,800),(1600,1000),(1920,1200),(2560,1600),(3840,2400)] {
        p.resize(width,height);
        for ibm in [false,true] {
            p.profile=if ibm {crate::appearance_profile::Profile::default().set("typography.map.fontFamily",serde_json::json!("IBM Plex Mono")).unwrap()}else{crate::appearance_profile::Profile::default()};
            p.identity_masks.clear();p.gpu=Some(crate::gpu_render::Frame::default());
            for glyph in SYMBOLS { for (w,h) in [(9,14),(15,24),(30,48)] {
                p.map_identity(glyph,100,100,w,h,[255;3]);
                let (nw,nh)=(p.native_x((100+w)as f64)-p.native_x(100.),p.native_y((100+h)as f64)-p.native_y(100.));
                let (key,value)=p.identity_masks.iter().find(|(key,_)|key.0==*glyph&&key.1==nw&&key.2==nh).unwrap();
                let (key,value)=(key.clone(),value.clone());let(dx,dy,mw,mh,mask,_)=&value;
                assert!(*mw>0&&*mh>0&&mask.iter().any(|&a|a>0),"{glyph}: nonempty coverage");
                assert!(*dx>=0&&*dy>=0&&*dx+*mw as i32<=nw&&*dy+*mh as i32<=nh,"{glyph}: cell fit");
                assert_eq!(mask.len(),mw*mh);
                // The corrected mask itself, not a separate low-resolution GPU
                // texture, is handed to the command recorder.
                let recorded=p.gpu.as_ref().unwrap().masks.last().unwrap();
                assert_eq!((recorded.width,recorded.height),(*mw,*mh));assert_eq!(&recorded.coverage,mask);
                let count=p.identity_masks.len();p.map_identity(glyph,100,100,w,h,[200,150,100]);
                assert_eq!(p.identity_masks.len(),count);assert_eq!(p.identity_masks.get(&key),Some(&value));
            }}
        }
    }
}

#[test]
fn native_coverage_clip_and_tint_are_the_same_mask() {
    let mut p=Painter::new();p.resize(1920,1200);
    for glyph in SYMBOLS {
        p.gpu=None;p.clip=None;p.clear([0,0,0]);p.map_identity(glyph,100,100,30,48,[255;3]);let full=p.pixels.clone();
        p.clear([0,0,0]);p.clip=Some((115,100,15,48));p.map_identity(glyph,100,100,30,48,[255;3]);
        let (left,top,right,bottom)=p.native_bounds();
        for y in 140..230 {for x in 140..205 {let at=(y*p.width+x)*4;let expected=if (left..right).contains(&(x as i32))&&(top..bottom).contains(&(y as i32)){&full[at..at+3]}else{&[0,0,0]};assert_eq!(&p.pixels[at..at+3],expected,"{glyph} clip");}}
    }
}

#[test]
fn native_coverage_retained_and_tile_renderers_match_full_frames() {
    let run:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let before=run.clone();let mut view=crate::observe::observe(&run);view.terrain_snapshot=None;
    view.cells=SYMBOLS.iter().enumerate().map(|(i,g)|Cell{x:8+i,y:10,glyph:(*g).into(),fg:[220,210,190],bg:[11,13,21],..Default::default()}).collect();view.terrain_count=view.cells.len();
    for(w,h)in [(1280,800),(1537,911),(2560,1600)] {
        let mut p=Painter::new();let mut full=Painter::new();let mut tiled=Painter::new();p.resize(w,h);full.resize(w,h);tiled.resize(w,h);
        let mut retained=RetainedGame::default();let mut planner=tile_damage::TilePlanner::default();let mut old=None;
        for step in 0..3 {
            if step==1{view.cells[0].fg=[40,210,160];view.cells[1].glyph="♣".into();}
            if step==2{view.cells[0].x+=1;view.cells[0].glyph="→".into();}
            retained.update(&mut p,&view,"",0.,true,step==0).unwrap();full.game_at(&view,"",false,0.);
            planner.replace_view(old.as_ref(),&view,&tiled);planner.finish_damage(&mut tiled);planner.paint_base(&mut tiled,&view,"",0.);tiled.tile_clip_enabled=false;planner.clear_frame();
            assert!(p.pixels==full.pixels,"retained {w}x{h} step {step}");assert!(tiled.pixels==full.pixels,"tile {w}x{h} step {step}");let mut next=crate::observe::observe(&run);next.terrain_snapshot=None;next.cells=view.cells.clone();next.terrain_count=view.terrain_count;old=Some(std::mem::replace(&mut view,next));
        }
    }
    assert_eq!(run,before);
}
