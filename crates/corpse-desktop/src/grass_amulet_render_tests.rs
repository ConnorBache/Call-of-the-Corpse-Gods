//! Native ink position and repaint oracles for grass, independent of game rules.
use super::*;
use crate::appearance_profile::Profile;
use serde_json::json;
fn ink_pixels(p:&Painter,bg:Rgb)->Vec<(i32,i32,[u8;4])> {
    p.pixels.chunks_exact(4).enumerate().filter(|(_,v)|v[..3]!=bg)
        .map(|(i,v)|((i%p.width)as i32,(i/p.width)as i32,[v[0],v[1],v[2],v[3]])).collect()
}
#[test]
fn grass_amulet_bottom_alignment_moves_ink_only_and_reuses_masks() {
    for (width,height) in [(1280,800),(1920,1200),(1537,911)] {
        for profile in [Profile::default(),Profile::default().set("typography.map.fontFamily",json!("IBM Plex Mono")).unwrap()] {
            let mut p=Painter::new();p.resize(width,height);p.profile=profile;
            let bg=[13,38,28];let fg=[120,199,107];let(x,y,w,h)=(50,60,15,24);
            p.clear(bg);p.map_identity("\"",x,y,w,h,fg);let old=ink_pixels(&p,bg);
            let caches=(p.map_masks.len(),p.identity_masks.len());
            p.clear(bg);p.map_identity_aligned("\"",x,y,w,h,fg,true);let new=ink_pixels(&p,bg);
            assert!(!new.is_empty());assert_eq!(old.len(),new.len());assert_eq!(caches,(p.map_masks.len(),p.identity_masks.len()));
            let shift=new[0].1-old[0].1;assert!(shift>0,"grass must move downward");
            assert!(old.iter().zip(&new).all(|(a,b)|a.0==b.0&&a.1+shift==b.1&&a.2==b.2),"unchanged glyph shape, ink and horizontal placement");
            assert_eq!(new.iter().map(|v|v.1).max().unwrap(),p.native_y((y+h-1)as f64)-1);
            assert!(new.iter().all(|&(xx,yy,_)|xx>=p.native_x(x as f64)&&xx<p.native_x((x+w)as f64)&&yy>=p.native_y(y as f64)&&yy<p.native_y((y+h)as f64)));
            p.clear(bg);p.map_identity("?",x,y,w,h,fg);let normal=p.pixels.clone();
            p.clear(bg);p.map_identity_aligned("?",x,y,w,h,fg,false);assert_eq!(p.pixels,normal);
            p.gpu=Some(crate::gpu_render::Frame::default());p.clear(bg);let frozen=p.pixels.clone();
            p.map_identity_aligned("\"",x,y,w,h,fg,true);
            assert_eq!(p.pixels,frozen);assert!(!p.gpu.as_ref().unwrap().quads.is_empty());
        }
    }
}
fn view()->View {
    let r=corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let mut v=crate::observe::observe(&r);v.terrain_snapshot=None;v.cells=vec![
        Cell{x:13,y:12,glyph:"\"".into(),label:"grass".into(),fg:[120,199,107],bg:[13,38,28],current:true,direct:true,bottom_aligned:true,..Default::default()},
        Cell{x:14,y:12,glyph:".".into(),fg:[81,91,107],bg:[21,25,35],current:true,direct:true,..Default::default()},
        Cell{x:15,y:12,glyph:"?".into(),fg:[255,248,232],bg:[21,25,35],current:true,direct:true,..Default::default()},
    ];v.terrain_count=v.cells.len();v
}
#[test]
fn grass_amulet_alignment_repaint_restores_full_retained_and_tile_frames() {
    for (width,height) in [(1280,800),(1920,1200),(1537,911)] {
        let mut v=view();let mut old=view();old.cells[0].bottom_aligned=false;
        let(mut p,mut full,mut tile)=(Painter::new(),Painter::new(),Painter::new());
        for p in [&mut p,&mut full,&mut tile] {p.resize(width,height);}
        let mut retained=RetainedGame::default();
        retained.update(&mut p,&old,"",0.,true,true).unwrap();
        let mut plan=tile_damage::TilePlanner::default();plan.replace_view(None,&old,&tile);plan.finish_damage(&mut tile);plan.paint_base(&mut tile,&old,"",0.);tile.tile_clip_enabled=false;plan.clear_frame();
        for bottom in [true,false,true] {
            v.cells[0].bottom_aligned=bottom;retained.update(&mut p,&v,"",0.,true,false).unwrap();
            full.game_at(&v,"",false,0.);assert!(p.pixels==full.pixels,"retained {width}x{height}");
            plan.replace_view(Some(&old),&v,&tile);plan.finish_damage(&mut tile);plan.paint_base(&mut tile,&v,"",0.);tile.tile_clip_enabled=false;
            assert!(tile.pixels==full.pixels,"tile {width}x{height}");plan.clear_frame();old=view();old.cells[0].bottom_aligned=bottom;
        }
        if let Ok(dir)=std::env::var("CORPSE_R51_VISUAL_OUTPUT") {
            std::fs::create_dir_all(&dir).unwrap();let file=std::fs::File::create(format!("{dir}/grass-amulet-{width}x{height}.png")).unwrap();
            let mut encoder=png::Encoder::new(file,width as u32,height as u32);encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&full.pixels).unwrap();
        }
    }
}
