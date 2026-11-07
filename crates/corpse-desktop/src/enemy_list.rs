//! Observation-only full creature previews. Geometry and content are shared by
//! paint, hit testing and scrolling; no live/hidden actor lookup occurs here.
use super::{Creature, Painter, View, INK, MUTED};
use crate::{modal_ui::Rect, typography_ui::{Role, RoleStyle}};

/// Reserved logical width includes the left outer gutter and map separation.
/// Unlike an incidental spare margin this survives wide maps and short windows.
pub const RESERVED_WIDTH: i32 = 252;
#[derive(Clone, Copy, Debug)]
pub struct Layout { pub bounds: Rect, pub rows: Rect }
impl Layout {
    pub fn resolve(p: &crate::appearance_profile::Profile, map_left: i32) -> Self {
        Self::resolve_frame(p, map_left, crate::modal_ui::Frame::from_profile(p))
    }
    /// O(1), uses the same responsive playfield as map/input geometry.
    pub fn resolve_frame(p:&crate::appearance_profile::Profile,map_left:i32,frame:crate::modal_ui::Frame)->Self {
        let top=super::chronicle_height(p)+20;
        let bottom=frame.map.1+frame.map.3-20;
        let bounds=Rect(8,top,(map_left-16).clamp(0,300),(bottom-top).max(0));
        Self { bounds, rows:Rect(bounds.0,bounds.1,bounds.2,bounds.3) }
    }
    pub fn content(self,p:&Painter,actors:&[Creature])->Content { Content::new(p,self,actors) }
}
#[derive(Clone,Debug)]
struct Line { hover_only:bool, text:String, y:i32, size:u16, color:super::Rgb, outline:Option<super::Rgb> }
#[derive(Clone,Debug)]
struct Bar { y:i32, fraction:f64, color:super::Rgb, uncertain:bool }
#[derive(Clone,Debug)]
struct Card { top:i32, height:i32, lines:Vec<Line>, bars:Vec<Bar> }
#[derive(Clone,Debug)]
pub struct Content { cards:Vec<Card>, height:i32, layout:Layout }
impl Content {
    /// O(T²) worst-case text measurement via measured word wrapping, O(T+C)
    /// temporary storage for C admitted creatures and T displayed characters.
    /// Work is bounded by detached observation text, never world population.
    fn new(p:&Painter,layout:Layout,actors:&[Creature])->Self {
        let width=(layout.rows.2-16).max(1);
        let mut cards=Vec::with_capacity(actors.len());let mut top=0;
        for a in actors {
            let mut card=Card{top,height:0,lines:Vec::new(),bars:Vec::new()};let mut y=5;
            fn text(p:&Painter,card:&mut Card,y:&mut i32,width:i32,s:&str,size:u16,color:super::Rgb) {
                let style=crate::typography_ui::resolve(&p.profile,Role::Secondary,RoleStyle::baseline(size as f64));
                let line=(style.font_size*style.line_height).ceil().max(size as f64+2.) as i32;
                for s in p.wrap_text_role(Role::Secondary,s,width,size) {
                    card.lines.push(Line{hover_only:false,text:s,y:*y,size,color,outline:None});*y+=line;
                }
            }
            text(p,&mut card,&mut y,width,&format!("{} {}",a.glyph,a.name),13,a.color);
            for line in &mut card.lines {line.outline=a.identity_outline;}
            y+=3;
            if let Some(label)=&a.perception_range {
                // Reserve stable card geometry, but reveal senses only on hover.
                let start=card.lines.len();text(p,&mut card,&mut y,width,label,11,MUTED);
                for line in &mut card.lines[start..] {line.hover_only=true;}
            }
            if a.shared_resources {
                text(p,&mut card,&mut y,width,"Shares your Health and Energy",11,INK);
            } else {
                for (name,value,max,color,invulnerable) in [
                    ("Health",a.hp,a.max_hp,[255,48,85],a.invulnerable),
                    ("Energy",a.energy,a.max_energy,[36,223,255],false),
                ] {
                    if invulnerable {
                        text(p,&mut card,&mut y,width,"Health · Invulnerable",11,INK);
                    } else {
                        text(p,&mut card,&mut y,width,&format!("{name} {value:.0}/{}",super::resource_max_label(max)),11,INK);
                        card.bars.push(Bar{y,fraction:(value/if max.is_infinite(){100.}else{max.max(1.)}).clamp(0.,1.),color,uncertain:false});y+=8;
                    }
                }
            }
            {
                for status in &a.statuses {
                    let name=crate::statuses_ui::display_name(&status.name);
                    let label=status.duration.map_or_else(||name.to_string(),|n|format!("{name} · {n}"));
                    text(p,&mut card,&mut y,width,&label,11,[193,138,255]);
                    if let Some(m)=a.meters.iter().find(|m|m.name==status.name).filter(|m|m.duration.is_some()) {
                        card.bars.push(Bar{y,fraction:(m.percent/100.).clamp(0.,1.),color:[193,138,255],uncertain:!m.reference_known});y+=6;
                    }
                }
            }
            card.height=y+9;top+=card.height;cards.push(card);
        }
        Self{cards,height:top,layout}
    }
    /// Pixel scrolling keeps every condition reachable even for a card taller
    /// than the viewport. Clamping after removals cannot strand the final card.
    pub fn offset(&self,requested:usize)->usize {requested.min((self.height-self.layout.rows.3).max(0) as usize)}
    /// O(1) card lookup after content layout; keep the selected name visible.
    pub fn reveal(&self,index:usize,requested:usize)->usize {
        let Some(card)=self.cards.get(index) else{return self.offset(requested);};
        let offset=self.offset(requested)as i32;
        let next=if card.top<offset||card.height>self.layout.rows.3 {card.top}
            else if card.top+card.height>offset+self.layout.rows.3 {card.top+card.height-self.layout.rows.3}
            else{offset};
        self.offset(next.max(0)as usize)
    }
    pub fn scroll(&self,offset:usize,delta:i32)->usize {
        let offset=self.offset(offset);let step=(delta.unsigned_abs() as usize).saturating_mul(48);
        self.offset(if delta<0 {offset.saturating_add(step)}else{offset.saturating_sub(step)})
    }
    pub fn hit(&self,x:i32,y:i32,offset:usize)->Option<usize> {
        if self.layout.rows.2<24 || !self.layout.rows.contains(x,y) {return None;}
        let at=y-self.layout.rows.1+self.offset(offset) as i32;
        self.cards.iter().position(|c|at>=c.top&&at<c.top+c.height)
    }
}
/// Same ordered text/rectangle commands for GPU and software. Only intersecting
/// cards rasterize; the clip excludes the heading/footer and map.
pub fn paint(p:&mut Painter,v:&View,layout:Layout) {
    if layout.bounds.2<24 || layout.rows.3<=0 {return;}
    let content=layout.content(p,&v.creatures);
    p.enemy_list_offset=content.offset(p.enemy_list_offset);
    let old_clip=p.clip;p.clip=Some(layout.bounds.tuple());
    p.clip=Some(layout.rows.tuple());
    for (a,card) in v.creatures.iter().zip(&content.cards) {
        let y=layout.rows.1+card.top-p.enemy_list_offset as i32;
        if y+card.height<=layout.rows.1 || y>=layout.rows.1+layout.rows.3 {continue;}
        if p.enemy_hover==Some(a.id.to_bits()) {p.rect(layout.rows.0,y,layout.rows.2-4,card.height-2,[39,44,61]);}
        for line in &card.lines {
            if line.hover_only&&p.enemy_hover!=Some(a.id.to_bits()){continue;}
            if let Some(outline)=line.outline {for (dx,dy) in [(-1,0),(1,0),(0,-1),(0,1)] {p.text_role(Role::Secondary,&line.text,layout.rows.0+4+dx,y+line.y+dy,line.size,outline);}}
            p.text_role(Role::Secondary,&line.text,layout.rows.0+4,y+line.y,line.size,line.color);
        }
        for bar in &card.bars {
            let w=(layout.rows.2-16).max(1);let yy=y+bar.y;
            p.rect(layout.rows.0+4,yy,w,3,[36,41,53]);
            p.rect(layout.rows.0+4,yy,(w as f64*bar.fraction) as i32,3,bar.color);
            if bar.uncertain {for dx in (0..w).step_by(5){p.rect(layout.rows.0+4+dx,yy-1,2,1,bar.color);}}
        }
        p.rect(layout.rows.0+4,y+card.height-3,(layout.rows.2-16).max(1),1,[36,30,46]);
    }
    p.clip=Some(layout.bounds.tuple());
    if content.height>layout.rows.3 {
        let track=layout.rows.3;let thumb=((track as f64*track as f64/content.height as f64).round()as i32).clamp(12.min(track),track);
        let sy=(track-thumb) as f64*p.enemy_list_offset as f64/(content.height-track) as f64;
        p.rect(layout.rows.0+layout.rows.2-3,layout.rows.1,2,track,[36,30,46]);
        p.rect(layout.rows.0+layout.rows.2-3,layout.rows.1+sy as i32,2,thumb,MUTED);
    }
    p.clip=old_clip;
}
#[cfg(test)] mod tests {
    use super::*;
    fn view()->View {crate::observe::observe(&corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap().run)}
    #[test] fn preview_column_survives_supported_sizes_and_right_aligns_map() {
        let mut p=Painter::new();
        for(w,h)in[(1280,800),(1920,1080),(2560,1600),(1537,911),(960,720),(960,1200)] {
            p.resize(w,h);p.set_responsive_canvas(true);
            for(mw,mh)in[(79,29),(40,30),(96,60)] {
                let(cw,ch,ox,oy)=p.map_layout(mw,mh);let frame=p.frame();
                let l=Layout::resolve_frame(&p.profile,ox,frame);
                assert!(l.bounds.2>=RESERVED_WIDTH-16);assert!(l.rows.3>0);
                assert_eq!(ox+cw*mw as i32,frame.map.2);assert!(oy>=frame.map.1);
                assert!(oy+ch*mh as i32<=frame.map.1+frame.map.3-20);
                let px=p.native_x((ox+cw*(mw as i32-1)+cw/2)as f64);let py=p.native_y((oy+ch*(mh as i32-1)+ch/2)as f64);
                let(x,y)=p.logical_point(px,py,(w as u32,h as u32));
                assert_eq!(((x-ox)/cw,(y-oy)/ch),(mw as i32-1,mh as i32-1));
            }
        }
    }
    #[test] fn every_dense_preview_and_every_long_condition_remains_reachable() {
        let mut v=view();let template=v.creatures[0].clone();
        v.creatures=(0..200).map(|i|{let mut a=template.clone();a.id=1000.+i as f64;a.name=format!("Observed creature {i}");a.statuses=(0..30).map(|j|crate::statuses_ui::Status{name:format!("Condition {j} with a long description"),duration:Some(j as f64+1.)}).collect();a}).collect();
        let p=Painter::new();let(_,_,ox,_)=p.map_layout(v.width,v.height);let l=Layout::resolve_frame(&p.profile,ox,p.frame());let c=l.content(&p,&v.creatures);
        assert!(c.cards[0].height>l.rows.3);
        for(i,card)in c.cards.iter().enumerate(){
            assert!(card.lines.iter().any(|l|l.text.contains("Health")));assert!(card.lines.iter().any(|l|l.text.contains("Energy")));
            for line in &card.lines {let at=(card.top+line.y)as usize;let off=c.offset(at);let y=l.rows.1+at as i32-off as i32;assert_eq!(c.hit(l.rows.0+4,y,off),Some(i));}
        }
        assert_eq!(c.hit(l.rows.0-1,l.rows.1,0),None);assert_eq!(c.hit(l.rows.0,l.rows.1+l.rows.3,0),None);
        assert_eq!(c.scroll(0,1),0);assert_eq!(c.scroll(0,-i32::MAX),c.offset(usize::MAX));
        assert_eq!(l.content(&p,&[]).offset(usize::MAX),0);
    }
    #[test] fn previews_preserve_special_resources_and_world() {
        let engine=corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();let before=engine.save().unwrap();let mut v=crate::observe::observe(&engine.run);
        let mut a=v.creatures[0].clone();a.max_energy=f64::INFINITY;a.invulnerable=true;a.shared_resources=false;v.creatures=vec![a.clone()];
        let mut p=Painter::new();let(_,_,ox,_)=p.map_layout(v.width,v.height);let l=Layout::resolve_frame(&p.profile,ox,p.frame());let c=l.content(&p,&v.creatures);
        assert!(c.cards[0].lines.iter().any(|l|l.text.contains("Invulnerable")));assert!(c.cards[0].lines.iter().any(|l|l.text.contains('∞')));
        v.creatures[0].shared_resources=true;let c=l.content(&p,&v.creatures);assert!(c.cards[0].lines.iter().any(|l|l.text.contains("Shares")));assert!(c.cards[0].bars.is_empty());
        v.creatures[0]=a;p.game_at(&v,"",false,0.);assert_eq!(engine.save().unwrap(),before);
        if let Ok(path)=std::env::var("ENEMY_LIST_SCREENSHOT"){let file=std::fs::File::create(path).unwrap();let mut encoder=png::Encoder::new(file,p.width as u32,p.height as u32);encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();}
    }
}

#[cfg(test)] mod damage_tests {
    use super::*;
    #[test]
    fn resource_and_condition_only_changes_repaint_full_retained_and_tile_views() {
        let engine=corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        for(w,h)in[(1280,800),(1537,911),(960,1200)] {
            let old=crate::observe::observe(&engine.run);let mut new=crate::observe::observe(&engine.run);
            let actor=&mut new.creatures[0];actor.hp=1.;actor.energy=2.;actor.statuses=vec![crate::statuses_ui::Status{name:"Poisoned".into(),duration:Some(7.)}];actor.meters=vec![crate::statuses_ui::Meter{name:"Poisoned".into(),duration:Some(7.),reference:10.,reference_known:true,percent:70.}];
            let mut retained=super::super::RetainedGame::default();let mut p=Painter::new();let mut full=Painter::new();
            for p in [&mut p,&mut full]{p.resize(w,h);p.set_responsive_canvas(true);}
            retained.update(&mut p,&old,"",0.,true,true).unwrap();retained.update(&mut p,&new,"",0.,true,false).unwrap();full.game_at(&new,"",false,0.);
            assert!(p.pixels==full.pixels,"retained preview resources/statuses at {w}x{h}");
            let mut plan=super::super::tile_damage::TilePlanner::default();let mut p=Painter::new();p.resize(w,h);p.set_responsive_canvas(true);
            plan.replace_view(None,&old,&p);plan.finish_damage(&mut p);plan.paint_base(&mut p,&old,"",0.);p.tile_clip_enabled=false;plan.clear_frame();
            plan.replace_view(Some(&old),&new,&p);plan.finish_damage(&mut p);plan.paint_base(&mut p,&new,"",0.);p.tile_clip_enabled=false;
            assert!(p.pixels==full.pixels,"tile preview resources/statuses at {w}x{h}");
        }
    }
}
