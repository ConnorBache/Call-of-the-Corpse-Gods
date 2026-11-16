//! Read-only, bounded custom counters. No simulation clocks or mutation.
use super::{Painter,resource_ui};
use crate::ability_ui::CustomIndicator;
const INK:[u8;3]=[255,248,232];
fn centered(p:&mut Painter,text:&str,x:i32,y:i32,width:i32,size:u16,color:[u8;3]) {
    let width_text=resource_ui::text_width(p,text,size);
    p.text(text,x+(width-width_text)/2,y,size,color);
}
pub fn paint(p:&mut Painter,value:CustomIndicator,x:i32,y:i32,width:i32) {
    match value {
        CustomIndicator::Nightmares{count,until}=>{
            let w=32;
            p.rect(x,y,w,19,[75,47,152]);
            p.border(x,y,w,19,[143,119,241]);
            centered(p,&count.to_string(),x,y+2,w,12,INK);
            centered(p,&until.to_string(),x,y+20,w,10,[180,164,242]);
        }
        CustomIndicator::Reactions(count)=>{
            p.rect(x,y,30,21,[218,173,53]);
            p.border(x,y,30,21,[255,219,113]);
            centered(p,&count.to_string(),x,y+3,30,12,[30,23,13]);
        }
        CustomIndicator::Rescue(ready)=>{
            if ready {p.rect(x,y,21,21,[59,150,255]);}
            p.border(x,y,21,21,[99,177,255]);
        }
        CustomIndicator::Blood{count,progress}=>{
            // Rasterize a pointed droplet with a rounded bottom, bounded 29x30.
            for dy in 0i32..30 {
                let half=if dy<14 {dy*13/14} else {((1.-((dy-17)as f64/13.).powi(2)).max(0.).sqrt()*13.)as i32};
                p.rect(x+14-half,y+dy,half*2+1,1,[177,33,66]);
            }
            centered(p,&count.to_string(),x,y+12,29,12,INK);
            let bx=x+35;let bw=(width-35).max(12);
            p.rect(bx,y+5,bw,7,[49,28,39]);
            p.rect(bx,y+5,(bw as f64*(progress as f64/50.).clamp(0.,1.)).round()as i32,7,[221,65,100]);
            p.border(bx,y+5,bw,7,[165,76,98]);
            centered(p,&format!("{progress}/50"),bx,y+16,bw,10,INK);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use corpse_core::Run;
    use crate::render::{RetainedGame,tile_damage::TilePlanner};
    const IDS:[&str;5]=["ursa-waking-nightmares","isheilah-quick-hand","isheilah-are-you-sure-about-that","pelian-you-shall-never-be-rid-of-me","lichor-overflowing"];
    fn room()->Run {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();r.player.abilities=IDS.iter().map(|s|s.to_string()).collect();
        r.player.active=vec![IDS[1].into(),IDS[2].into(),IDS[3].into()];
        r.player.conditions.clear();r.offers.clear();r.player.nightmares=3.0.into();r.player.reactions=2.0.into();
        r.player.blood=12.0.into();r.player.damage_bank=27.0.into();r.player.conditions.insert("nightmareAt".into(),r.tick+43.);
        r
    }
    fn save(p:&Painter,name:&str) {
        if let Ok(dir)=std::env::var("CORPSE_INDICATOR_REVIEW") {
            std::fs::create_dir_all(&dir).unwrap();
            let mut encoder=png::Encoder::new(std::fs::File::create(format!("{dir}/{name}.png")).unwrap(),p.width as u32,p.height as u32);
            encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
            // A detail capture of the same native frame, without resizing pixels.
            if p.width==1920 {
                let (x,y,w,h)=(1524usize,235usize,386usize,250usize);
                let mut pixels=Vec::with_capacity(w*h*4);
                for row in y..y+h {pixels.extend_from_slice(&p.pixels[(row*p.width+x)*4..(row*p.width+x+w)*4]);}
                let mut enc=png::Encoder::new(std::fs::File::create(format!("{dir}/{name}-detail.png")).unwrap(),w as u32,h as u32);
                enc.set_color(png::ColorType::Rgba);enc.set_depth(png::BitDepth::Eight);
                enc.write_header().unwrap().write_image_data(&pixels).unwrap();
            }

        }
    }
    #[test]
    fn custom_indicators_use_serialized_values_and_acquisition_order() {
        let mut r=room();let before=r.clone();let v=crate::ability_ui::visible_abilities(&r);
        assert_eq!(v[0].indicator,Some(CustomIndicator::Nightmares{count:3,until:43}));
        assert_eq!(v[1].indicator,Some(CustomIndicator::Reactions(2)));assert_eq!(v[2].indicator,None);
        assert_eq!(v[3].indicator,Some(CustomIndicator::Rescue(true)));
        assert_eq!(v[4].indicator,Some(CustomIndicator::Blood{count:12,progress:27}));assert!(r==before);
        r.player.abilities.swap(1,2);
        assert_eq!(crate::ability_ui::observe(&r,IDS[2]).indicator,Some(CustomIndicator::Reactions(2)));
        assert_eq!(crate::ability_ui::observe(&r,IDS[1]).indicator,None);
        r.player.conditions.insert("pelianUsed".into(),1.);
        assert_eq!(crate::ability_ui::observe(&r,IDS[3]).indicator,Some(CustomIndicator::Rescue(false)));
        r.player.abilities.insert(0,"isheilah-missed-me".into());
        assert_eq!(crate::ability_ui::observe(&r,"isheilah-missed-me").indicator,None);
    }
    #[test]
    fn custom_indicators_native_captures_and_incremental_repaint() {
        for(w,h)in[(1280,800),(1920,1080),(960,1200)] {
            let mut r=room();let mut old=crate::observe::observe(&r);
            let(mut full,mut retained_p,mut tile)=(Painter::new(),Painter::new(),Painter::new());
            for p in [&mut full,&mut retained_p,&mut tile]{p.try_resize(w,h).unwrap();p.set_responsive_canvas(true);}
            let mut retained=RetainedGame::default();let mut plan=TilePlanner::default();
            for step in 0..3 {
                if step==1 {r.player.nightmares=0.0.into();r.player.reactions=0.0.into();r.player.blood=0.0.into();r.player.damage_bank=0.0.into();r.player.conditions.insert("pelianUsed".into(),1.);r.player.active.clear();}
                if step==2 {r.player.nightmares=1.0.into();r.player.reactions=1.0.into();r.player.blood=55.0.into();r.player.damage_bank=49.0.into();r.player.conditions.remove("pelianUsed");r.player.conditions.insert("nightmareAt".into(),r.tick+1.);}
                let v=crate::observe::observe(&r);full.game_at(&v,"",false,0.);
                retained.update(&mut retained_p,&v,"",0.,true,false).unwrap();
                plan.replace_view(if step==0{None}else{Some(&old)},&v,&tile);plan.finish_damage(&mut tile);plan.paint_base(&mut tile,&v,"",0.);tile.tile_clip_enabled=false;plan.clear_frame();
                assert!(retained_p.pixels==full.pixels,"retained {w}x{h} {step}");assert!(tile.pixels==full.pixels,"tile {w}x{h} {step}");
                save(&full,&format!("custom-indicators-{step}-{w}x{h}"));old=v;
            }
        }
    }
}
