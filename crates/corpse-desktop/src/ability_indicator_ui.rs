//! Read-only, bounded custom counters. No simulation clocks or mutation.
use super::{Painter,resource_ui};
use crate::ability_ui::CustomIndicator;
const INK:[u8;3]=[255,248,232];
fn centered(p:&mut Painter,text:&str,x:i32,y:i32,width:i32,size:u16,color:[u8;3]) {
    let width_text=resource_ui::text_width(p,text,size);
    p.text(text,x+(width-width_text)/2,y,size,color);
}
pub(super) const RAGELIGHT_FILL: [u8; 3] = [150, 26, 47];
pub(super) const RAGELIGHT_OUTLINE: [u8; 3] = [255, 92, 105];

fn ragelight_label(cost_bits:u64)->String {
    let cost=f64::from_bits(cost_bits);
    if cost>=1e9 {format!("{cost:.2e}")} else {crate::stat_presentation::whole_number(cost)}
}
pub(super) fn ragelight_width(p:&Painter,cost_bits:u64)->i32 {
    (resource_ui::text_width(p,&ragelight_label(cost_bits),12)+12).max(25)
}

fn paint_nightmare_boxes(p:&mut Painter,count:u64,until:u64,x:i32,y:i32,w:i32,h:i32) {
    use crate::game_hud::{nightmare_box_width,NIGHTMARE_COUNTER_GAP as GAP};
    let count_w=nightmare_box_width(count,2);
    let timer_w=nightmare_box_width(until,3);
    let x=x+(w-count_w-GAP-timer_w).max(0);
    for (x,w,text,fill,border,ink) in [
        (x,count_w,count.to_string(),[0,0,0],[132,130,148],INK),
        (x+count_w+GAP,timer_w,until.to_string(),[32,139,72],[113,240,91],[255,255,255]),
    ] {
        p.rect(x,y,w,h,fill);
        p.border(x,y,w,h,border);
        centered(p,&text,x,y+(h-8)/2,w,8,ink);
    }
}

fn paint_blood_drop(p: &mut Painter, count: u64, x: i32, y: i32) {
    // Rasterize a pointed droplet with a rounded bottom, bounded 29x30.
    for dy in 0i32..30 {
        let half = if dy < 14 {
            dy * 13 / 14
        } else {
            ((1. - ((dy - 17) as f64 / 13.).powi(2)).max(0.).sqrt() * 13.) as i32
        };
        p.rect(x + 14 - half, y + dy, half * 2 + 1, 1, [177, 33, 66]);
    }
    centered(p, &count.to_string(), x, y + 12, 29, 12, INK);
}

pub fn paint(p:&mut Painter,value:CustomIndicator,x:i32,y:i32,width:i32) {
    match value {
        CustomIndicator::Nightmares{count,until}=>{
            paint_nightmare_boxes(p,count,until,x,y,width,19);
        }
        CustomIndicator::Reactions(count)=>{
            p.rect(x,y,30,21,[218,173,53]);
            p.border(x,y,30,21,[255,219,113]);
            centered(p,&count.to_string(),x,y+3,30,12,[30,23,13]);
        }
        CustomIndicator::FutureFloors{used}=>{
            // Fixed 20-floor tracker: bounded O(20) work, no simulation changes.
            let cell=width/10;
            for floor in 1..=20 {
                let col=(floor-1)%10;let row=(floor-1)/10;
                let bx=x+col*cell;let by=y+row*15;
                centered(p,&floor.to_string(),bx,by,cell,8,INK);
                if used&(1 << (floor-1))!=0 {
                    // Deliberate X over the number, not a replacement glyph.
                    for step in 0..10 {
                        p.rect(bx+(cell-10)/2+step,by+step,1,1,[243,108,255]);
                        p.rect(bx+(cell-10)/2+9-step,by+step,1,1,[243,108,255]);
                    }
                }
            }
        }
        CustomIndicator::Future(ready)=>{
            p.border(x,y,23,23,[174,142,241]);
            if ready {
                // Sample a two-turn spiral directly, avoiding font fallback glyphs.
                for step in 0..160 {
                    let t=step as f64/159.;
                    let angle=t*std::f64::consts::TAU*2.;
                    let radius=1.+7.*t;
                    let px=x+11+(radius*angle.cos()).round() as i32;
                    let py=y+11+(radius*angle.sin()).round() as i32;
                    p.rect(px,py,2,2,[192,168,255]);
                }
            }
        }
        CustomIndicator::RagelightCost(cost_bits)=>{
            p.rect(x,y,width,19,RAGELIGHT_FILL);
            p.border(x,y,width,19,RAGELIGHT_OUTLINE);
            centered(p,&ragelight_label(cost_bits),x,y+2,width,12,INK);
        }
        CustomIndicator::Rescue(ready)=>{
            if ready {p.rect(x,y,21,21,[59,150,255]);}
            p.border(x,y,21,21,[99,177,255]);
        }
        CustomIndicator::OverflowingSaves(count) => paint_blood_drop(p, count, x, y),
        CustomIndicator::Blood{count,progress}=>{
            paint_blood_drop(p, count, x, y);
            let bx=x+35;let bw=(width-35).max(12);
            p.rect(bx,y+5,bw,7,[49,28,39]);
            p.rect(bx,y+5,(bw as f64*(progress as f64/50.).clamp(0.,1.)).round()as i32,7,[221,65,100]);
            p.border(bx,y+5,bw,7,[165,76,98]);
            centered(p,&format!("{progress}/50"),bx,y+16,bw,10,INK);
        }
    }
}

/// Same detached counters as the full detail sheet in a single HUD row.
pub fn paint_compact(p:&mut Painter,value:CustomIndicator,r:crate::modal_ui::Rect) {
    let crate::modal_ui::Rect(x,y,w,h)=r;
    let (text,fill,border)=match value {
        CustomIndicator::Nightmares{count,until}=>{
            paint_nightmare_boxes(p,count,until,x,y,w,h);
            return;
        },
        CustomIndicator::Reactions(n)=>(n.to_string(),[77,65,33],[255,219,113]),
        CustomIndicator::Rescue(ready)|CustomIndicator::Future(ready)=>(if ready{"Ready"}else{"Spent"}.into(),[40,64,105],[126,171,255]),
        CustomIndicator::RagelightCost(bits)=>(ragelight_label(bits),RAGELIGHT_FILL,RAGELIGHT_OUTLINE),
        CustomIndicator::FutureFloors{used}=>{
            // Keep all 20 floor states, using ten dots per row.
            for n in 0..20 {
                let ink=if used&(1<<n)==0{[160,121,231]}else{[66,49,85]};
                p.rect(x+n%10*(w/10),y+2+n/10*6,3,3,ink);
            }
            return;
        },
        CustomIndicator::OverflowingSaves(n)|CustomIndicator::Blood{count:n,..}=>{
            // Retain the blood drop glyph; progress stays in the right slot.
            for dy in 0i32..11 {
                let dx=if dy<6{dy/2}else{(11-dy).min(4)};
                p.rect(x+5-dx,y+dy,2*dx+1,1,[218,51,86]);
            }
            let text=if let CustomIndicator::Blood{progress,..}=value{format!("{n} · {progress}")}else{n.to_string()};
            let text=p.fit_text_role(crate::typography_ui::Role::Primary,&text,w-13,8);
            p.text(&text,x+13,y+1,8,INK);
            return;
        },
    };
    p.rect(x,y,w,h,fill);
    p.border(x,y,w,h,border);
    let text=p.fit_text_role(crate::typography_ui::Role::Primary,&text,w-4,9);
    centered(p,&text,x,y+1,w,9,INK);
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
        assert_eq!(v[0].name,"Nightmare");
        assert_eq!(v[0].id,IDS[0]);
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
    fn compact_nightmare_boxes_and_cogworks_full_reserve_fit() {
        use crate::game_hud::{COGWORKS_RESERVE_WIDTH,nightmare_box_width,nightmare_extra_width};
        let mut p=Painter::new();
        assert_eq!(resource_ui::fit_value(&p,"10/10",6,COGWORKS_RESERVE_WIDTH-6).0,"10/10");
        let count=1000;
        let until=100;
        let indicator=CustomIndicator::Nightmares{count,until};
        let width=48+nightmare_extra_width(Some(indicator));
        let count_w=nightmare_box_width(count,2);
        let timer_w=nightmare_box_width(until,3);
        assert_eq!(width,count_w+3+timer_w);
        assert!(resource_ui::text_width(&p,&count.to_string(),8)<=count_w-6);
        assert!(resource_ui::text_width(&p,&until.to_string(),8)<=timer_w-6);
        p.clear([24,32,40]);
        paint_compact(&mut p,indicator,crate::modal_ui::Rect(10,10,width,13));
        let pixel=|x,y|{let i=(y*p.width+x)*4;&p.pixels[i..i+3]};
        assert_eq!(pixel(11,11),[0,0,0]);
        assert_eq!(pixel(10+count_w as usize+4,11),[32,139,72]);
        assert_eq!(pixel(10+count_w as usize+1,11),[24,32,40]);
    }

    #[test]
    fn overflowing_uses_the_classic_drop_without_the_legacy_progress_bar() {
        let (mut current, mut legacy) = (Painter::new(), Painter::new());
        let untouched = current.pixels.clone();
        for (p, indicator) in [
            (&mut current, CustomIndicator::OverflowingSaves(3)),
            (&mut legacy, CustomIndicator::Blood { count: 3, progress: 27 }),
        ] {
            paint(p, indicator, 0, 0, 80);
        }
        for y in 0..30 {
            let row = y * current.width * 4;
            let drop = row..row + 29 * 4;
            assert_eq!(&current.pixels[drop.clone()], &legacy.pixels[drop]);
            let beside_drop = row + 29 * 4..row + 80 * 4;
            assert_eq!(&current.pixels[beside_drop.clone()], &untouched[beside_drop]);
        }
        assert_ne!(&current.pixels[..29 * 4], &untouched[..29 * 4]);
        let mut empty = Painter::new();
        paint(&mut empty, CustomIndicator::OverflowingSaves(0), 0, 0, 80);
        assert!(empty.pixels != current.pixels);
    }

    #[test]
    fn custom_indicators_native_captures_and_incremental_repaint() {
        for(w,h)in[(1280,800),(1920,1080),(960,1200)] {
            let mut r=room();let mut old=crate::observe::observe(&r);
            let(mut full,mut retained_p,mut tile)=(Painter::new(),Painter::new(),Painter::new());
            for p in [&mut full,&mut retained_p,&mut tile]{p.try_resize(w,h).unwrap();p.set_responsive_canvas(true);}
            let mut retained=RetainedGame::default();let mut plan=TilePlanner::default();
            for step in 0..5 {
                if step==1 {r.player.nightmares=0.0.into();r.player.reactions=0.0.into();r.player.blood=0.0.into();r.player.damage_bank=0.0.into();r.player.conditions.insert("pelianUsed".into(),1.);r.player.active.clear();}
                if step==2 {r.player.nightmares=1.0.into();r.player.reactions=1.0.into();r.player.blood=55.0.into();r.player.damage_bank=49.0.into();r.player.conditions.remove("pelianUsed");r.player.conditions.insert("nightmareAt".into(),r.tick+1.);}
                if step==3 {r.balance_rules_from=0usize.into();r.player.conditions.insert("overflowingSaves".into(),0.);}
                if step==4 {corpse_core::divine_rescues::refill_on_level_up(&mut r);}
                let v=crate::observe::observe(&r);full.game_at(&v,"",false,0.);
                retained.update(&mut retained_p,&v,"",0.,true,false).unwrap();
                plan.replace_view(if step==0{None}else{Some(&old)},&v,&tile);plan.finish_damage(&mut tile);plan.paint_base(&mut tile,&v,"",0.);tile.tile_clip_enabled=false;plan.clear_frame();
                assert!(retained_p.pixels==full.pixels,"retained {w}x{h} {step}");assert!(tile.pixels==full.pixels,"tile {w}x{h} {step}");
                save(&full,&format!("custom-indicators-{step}-{w}x{h}"));old=v;
            }
        }
    }
}

#[cfg(test)]
mod future_indicator_tests {
    use super::*;
    #[test]
    fn future_box_spiral_tracks_snapshot_without_mutation() {
        let id="morae-see-the-future";
        let mut r:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();r.future_floor_rules_from=corpse_core::Field::Null;r.player.abilities=vec![id.into()];r.offers.clear();r.future=None;
        corpse_core::revision50_ability_energy::acquire_for_revision(&mut r.player,&[id.into()],52.).unwrap();
        let before=r.clone();
        let row=crate::ability_ui::observe(&r,id);
        assert_eq!(row.indicator,Some(CustomIndicator::Future(false)));assert_eq!(row.reserve,None);
        assert!(!row.cost.contains("Reserve"));assert!(!row.state.contains("Unavailable"));assert_eq!(r,before);
        r.future=Some(corpse_core::future_anchor::FutureAnchor::from_text(corpse_core::storage::serialize(&r).unwrap()));
        assert_eq!(crate::ability_ui::observe(&r,id).indicator,Some(CustomIndicator::Future(true)));
        let mut empty=Painter::new();empty.try_resize(1280,800).unwrap();empty.set_responsive_canvas(true);
        paint(&mut empty,CustomIndicator::Future(false),4,4,25);
        let mut full=Painter::new();full.try_resize(1280,800).unwrap();full.set_responsive_canvas(true);
        paint(&mut full,CustomIndicator::Future(true),4,4,25);
        assert_ne!(empty.pixels,full.pixels);
        for y in 0..800 { for x in 0..1280 {if !(5..26).contains(&x)||!(5..26).contains(&y) {let i=(y*1280+x)*4;assert_eq!(&empty.pixels[i..i+4],&full.pixels[i..i+4]);}}}
    }
}

#[cfg(test)]
mod floor_future_tests {
    use super::*;
    use corpse_core::{Run,future_floor};
    #[test]
    fn floor_future_native_tracker_screenshots() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();r.future_floor_rules_from=0usize.into();r.fast_mode=true.into();r.compressed_depths=true.into();
        r.player.abilities=vec!["morae-fortune-teller".into(),future_floor::ID.into(),"morae-all-seeing-eyes".into()];
        r.offers.clear();r.player.conditions.clear();r.player.active.clear();
        r.future_floor=Some(future_floor::State{depth:r.depth,anchor:None,used_floors:vec![5.,11.,15.]});
        let before=r.clone();let row=crate::ability_ui::observe(&r,future_floor::ID);
        assert!(row.active);assert_eq!(row.reserve,None);assert!(row.cost.is_empty());
        assert_eq!(row.description,crate::ability_ui::FUTURE_FLOOR_DESCRIPTION);
        assert_eq!(row.indicator,Some(CustomIndicator::FutureFloors{used:(1<<2)|(1<<5)|(1<<7)}));
        assert_eq!(r,before);
        for (width,height) in [(1920,1080),(1280,800),(960,1200)] {
            for spent in [false,true] {
                if spent {r.future_floor.as_mut().unwrap().used_floors.push(r.depth);}
                let mut p=Painter::new();p.try_resize(width,height).unwrap();p.set_responsive_canvas(true);
                let view=crate::observe::observe(&r);p.game_at(&view,"",false,0.);
                let ready=crate::ability_ui::observe(&r,future_floor::ID);assert_eq!(ready.active,!spent);
                if let Ok(dir)=std::env::var("CORPSE_FUTURE_REVIEW") {
                    std::fs::create_dir_all(&dir).unwrap();
                    let mut enc=png::Encoder::new(std::fs::File::create(format!("{dir}/futuresight-{}-{width}x{height}.png",if spent{"used"}else{"ready"})).unwrap(),width as u32,height as u32);
                    enc.set_color(png::ColorType::Rgba);enc.set_depth(png::BitDepth::Eight);enc.write_header().unwrap().write_image_data(&p.pixels).unwrap();
                    if width==1920 {
                        let(x,y,w,h)=(1530usize,235usize,380usize,150usize);
                        let mut detail=Vec::with_capacity(w*h*4);
                        for row in y..y+h {detail.extend_from_slice(&p.pixels[(row*width+x)*4..(row*width+x+w)*4]);}
                        let mut enc=png::Encoder::new(std::fs::File::create(format!("{dir}/futuresight-{}-detail.png",if spent{"used"}else{"ready"})).unwrap(),w as u32,h as u32);
                        enc.set_color(png::ColorType::Rgba);enc.set_depth(png::BitDepth::Eight);enc.write_header().unwrap().write_image_data(&detail).unwrap();
                    }
                }
                if spent {r.future_floor.as_mut().unwrap().used_floors.retain(|d|*d!=r.depth);}
            }
        }
    }
}

#[cfg(test)]
mod ragelight_tests {
    use super::*;
    use crate::render::{RetainedGame,tile_damage::TilePlanner};

    #[test]
    fn ragelight_red_cost_box_and_reserve_render_without_a_toggle_at_each_size() {
        use corpse_core::{Run, divine_rescues::{self, RAGE}};
        for (w,h) in [(960,1200),(1280,800),(1920,1080)] {
            let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
            r.simulation_revision=52.0.into();
            r.player.abilities=vec![RAGE.into()];
            // Even stale selection and old enabled bits cannot bring back its toggle.
            r.player.active=vec![RAGE.into()];
            r.player.conditions.shift_remove("rageCost");
            corpse_core::revision50_ability_energy::reconcile_acquired(&mut r).unwrap();
            let (mut full,mut retained_p,mut tile)=(Painter::new(),Painter::new(),Painter::new());
            for p in [&mut full,&mut retained_p,&mut tile] {p.try_resize(w,h).unwrap();p.set_responsive_canvas(true);}
            let mut retained=RetainedGame::default();
            let mut plan=TilePlanner::default();
            let mut old=None;
            let mut zero_pixels=None;
            for expected in [0.0f64,1.,2.,0.] {
                if expected==0. && old.is_some() {
                    r.exp=corpse_core::progression::level_up_cost(&r);
                    corpse_core::progression::level_up(&mut r);
                    r.offers.clear();
                }
                r.player.hp=1.;
                let mut v=crate::observe::observe(&r);
                v.selected_ability=Some(RAGE.into());
                let cost=v.abilities[0].indicator.unwrap();
                assert_eq!(cost,CustomIndicator::RagelightCost(expected.to_bits()));
                assert!(v.abilities[0].reserve.is_some());
                full.game_at(&v,"",false,0.);
                retained.update(&mut retained_p,&v,"",0.,true,false).unwrap();
                plan.replace_view(old.as_ref(),&v,&tile);plan.finish_damage(&mut tile);
                plan.paint_base(&mut tile,&v,"",0.);tile.tile_clip_enabled=false;plan.clear_frame();
                assert_eq!(full.pixels,retained_p.pixels);
                assert_eq!(full.pixels,tile.pixels);

                let row=full.game_ability_rect(0);
                let crate::modal_ui::Rect(bar_x,y,_,h)=crate::game_hud::right_slot(row);
                let (box_x,box_w)=(bar_x-28,26);
                let row_color=|x:i32,y:i32| {
                    let at=(full.native_y(y as f64)as usize*full.width+full.native_x(x as f64)as usize)*4;
                    &full.pixels[at..at+3]
                };
                assert_eq!(row_color(box_x+2,y+2),RAGELIGHT_FILL);
                assert_eq!(row_color(bar_x+2,y+2),super::super::game_color(&full.profile,"ui.colors.energy"));
                for dy in 0..h {for dx in 4..row.2-4 {
                    assert!(![[100,255,114],[255,155,56],[142,142,158]].iter()
                        .any(|rgb|row_color(row.0+dx,y+dy)==rgb));
                }}
                let glyph:Vec<_>=(1..h-1).flat_map(|dy|(3..box_w-3).map(move |dx|(dx,dy)))
                    .map(|(dx,dy)|row_color(box_x+dx,y+dy)!=RAGELIGHT_FILL).collect();
                if let Some(zero)=&zero_pixels {assert_eq!(&glyph==zero,expected==0.);}
                else {zero_pixels=Some(glyph);}
                old=Some(v);
                let player=r.player.id;
                r.player.hp=0.;
                assert!(divine_rescues::try_rescue(&mut r,player,false));
            }
        }
    }
}
