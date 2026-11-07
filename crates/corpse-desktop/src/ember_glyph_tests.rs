//! Middle-dot geometry and observation-only ember captures.
use super::*;
use crate::appearance_profile::Profile;

fn capture(name: &str, p: &Painter) {
    let Ok(dir) = std::env::var("CORPSE_R51_EMBER_OUTPUT") else { return; };
    std::fs::create_dir_all(&dir).unwrap();
    let file = std::fs::File::create(format!("{dir}/{name}.png")).unwrap();
    let mut encoder = png::Encoder::new(file, p.width as u32, p.height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
}

#[test]
fn ember_middle_dot_keeps_font_geometry_at_native_fractional_and_hidpi_sizes() {
    let mut p = Painter::new();
    for (width, height) in [(960,600),(1280,800),(1600,1000),(1920,1200),(2560,1600),(3840,2400)] {
        p.resize(width,height);
        for ibm in [false,true] {
            p.profile = if ibm {
                Profile::default().set("typography.map.fontFamily",serde_json::json!("IBM Plex Mono")).unwrap()
            } else { Profile::default() };
            p.identity_masks.clear();
            for (w,h) in [(8,13),(12,19),(15,24),(30,48),(60,96)] {
                p.clear([0,0,0]);
                p.map_identity_alpha("·",100,100,w,h,[255,255,255],1.);
                let nw=p.native_x((100+w) as f64)-p.native_x(100.);
                let nh=p.native_y((100+h) as f64)-p.native_y(100.);
                let (_,actual)=p.identity_masks.iter().find(|(key,_)|key.0=="·"&&key.1==nw&&key.2==nh).unwrap();
                let (dx,dy,mw,mh,mask)=actual;
                let font=if ibm {&p.font} else {&p.fallback[0]};
                let (metrics,expected)=font.rasterize('·',(17.*(nw as f64/15.).min(nh as f64/24.)) as f32);
                assert_eq!((*mw,*mh),(metrics.width,metrics.height));
                assert_eq!(*mask,expected,"must retain original font coverage, never nearest-enlarge the ink bounds");
                assert!(*mw>0&&*mh>0&&mask.iter().any(|&a|a>0));
                assert!(*mw as i32<=(nw/3).max(3),"the ember must remain a small dot, not a full-cell block");
                assert!((*dx*2+*mw as i32-nw).abs()<=1);
                assert!((*dy*2+*mh as i32-nh).abs()<=1);
                assert!(mask.iter().any(|&a|a>0&&a<255),"retain native antialiasing");
            }
        }
    }
}

#[test]
fn ember_dot_cache_clip_and_gpu_use_the_same_native_mask() {
    let mut p=Painter::new();p.resize(1920,1200);p.clear([0,0,0]);
    p.map_identity_alpha("·",100,100,30,48,[255,255,255],1.);
    let full=p.pixels.clone();let cache=p.identity_masks.clone();
    p.clear([0,0,0]);p.map_identity_alpha("·",100,100,30,48,[255,255,255],1.);
    assert_eq!(p.identity_masks,cache);assert_eq!(p.pixels,full);
    p.clear([0,0,0]);p.clip=Some((115,100,15,48));
    p.map_identity_alpha("·",100,100,30,48,[255,255,255],1.);
    let clip_x=p.native_x(115.) as usize;
    for y in 0..p.height { for x in 0..p.width {
        let at=(y*p.width+x)*4;
        assert_eq!(&p.pixels[at..at+3],if x<clip_x {&[0,0,0][..]} else {&full[at..at+3]});
    }}
    p.clip=None;p.gpu=Some(crate::gpu_render::Frame::default());
    p.map_identity_alpha("·",100,100,30,48,[255,255,255],1.);
    let frame=p.gpu.as_ref().unwrap();assert_eq!(frame.masks.len(),1);assert_eq!(frame.quads.len(),1);
    let (_,(_,_,w,h,mask))=cache.iter().next().unwrap();
    assert_eq!((frame.masks[0].width,frame.masks[0].height),(*w,*h));
    assert_eq!(&frame.masks[0].coverage,mask);
}

#[test]
fn ember_scene_preserves_authored_symbol_preferences_and_detached_knowledge() {
    let run=corpse_core::storage::deserialize(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let frozen=run.clone();let mut view=crate::observe::observe(&run);
    let mut observation=(**view.terrain_snapshot.as_ref().unwrap()).clone();
    let indexes:Vec<_>=observation.cells.iter().filter(|c|c.current&&c.tile.as_deref()==Some("floor")
        &&(13..=16).contains(&(c.index%observation.width))&&c.index/observation.width==13)
        .map(|c|c.index).collect();
    assert!(!indexes.is_empty());
    for c in observation.cells.iter_mut().filter(|c|indexes.contains(&c.index)) {
        c.hazards=Some(corpse_core::HazardSnapshot {fire:"embers".to_owned().into(),..Default::default()});
    }
    let p=Profile::default();
    for remembered in [false,true] {
        let mut known=observation.clone();
        if remembered { for c in known.cells.iter_mut().filter(|c|indexes.contains(&c.index)) {
            c.current=false;c.direct=false;c.knowledge="remembered".into();
        }}
        let (cells,_,_)=crate::observe::terrain_frame(&known,&p,17,0.);
        for index in &indexes {
            let c=cells.iter().find(|c|c.y*known.width+c.x==*index).unwrap();
            assert_eq!(c.glyph,"·");
        }
        let custom=p.set("terrain.feature:embers.glyph",serde_json::json!("*")).unwrap();
        let (cells,_,_)=crate::observe::terrain_frame(&known,&custom,17,0.);
        for index in &indexes {
            assert_eq!(cells.iter().find(|c|c.y*known.width+c.x==*index).unwrap().glyph,"*");
        }
    }
    let (cells,_,_)=crate::observe::terrain_frame(&observation,&p,view.cosmetic_seed,0.);
    view.cells.splice(..view.terrain_count,cells);
    view.terrain_snapshot=Some(std::sync::Arc::new(observation));
    let mut painter=Painter::new();
    for (w,h) in [(1280,800),(1600,1000),(1920,1080),(2560,1600)] {
        painter.resize(w,h);painter.game_at(&view,"",false,0.);
        capture(&format!("ember-scene-{w}x{h}"),&painter);
        painter.clear([15,18,27]);
        painter.text("EMBER GLYPH: authored middle dot at native size",30,20,20,INK);
        for (i,(cw,ch)) in [(8,13),(12,19),(15,24),(30,48),(60,96)].into_iter().enumerate() {
            let x=50+i as i32*160;
            painter.text(&format!("{cw} x {ch}"),x,60,16,INK);
            painter.border(x-1,99,cw+2,ch+2,[60,65,77]);
            painter.map_identity("·",x,100,cw,ch,[227,38,54]);
        }
        capture(&format!("ember-detail-{w}x{h}"),&painter);
    }
    assert_eq!(run,frozen,"presentation must not alter simulation");
}
