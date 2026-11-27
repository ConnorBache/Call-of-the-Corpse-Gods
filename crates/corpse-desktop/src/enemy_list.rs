//! Observation-only full creature previews. Geometry and content are shared by
//! paint, hit testing and scrolling; no live/hidden actor lookup occurs here.
use super::{Creature, Painter, View, INK, MUTED};
use crate::{modal_ui::Rect, typography_ui::{Role, RoleStyle}};

/// Reserved logical width includes the left outer gutter and map separation.
/// Unlike an incidental spare margin this survives wide maps and short windows.
/// Knowledge-filtered ground evidence; no live world references survive observation.
#[derive(Clone, Debug, PartialEq)]
pub struct GroundEntity {
    pub hallucination: Option<std::sync::Arc<[(String,String,super::Rgb);6]>>,
    pub kind: GroundKind, pub x: usize, pub y: usize, pub id: u64,
    pub name: String, pub glyph: String, pub color: super::Rgb, pub distance: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum GroundKind { Stair, Item, Vent, Corpse }
/// O(G log G), deterministic distance/tile/identity tie-breaks within each group.
pub fn sort_ground(rows: &mut [GroundEntity]) {
    rows.sort_by(|a,b|a.kind.cmp(&b.kind).then(a.distance.total_cmp(&b.distance))
        .then((a.y,a.x,a.id).cmp(&(b.y,b.x,b.id))).then(a.name.cmp(&b.name)));
}
pub const RESERVED_WIDTH: i32 = crate::game_hud::RAIL_WIDTH;
#[derive(Clone, Copy, Debug)]
pub struct Layout { pub bounds: Rect, pub rows: Rect }
impl Layout {
    pub fn resolve(p: &crate::appearance_profile::Profile, map_left: i32) -> Self {
        Self::resolve_frame(p, map_left, crate::modal_ui::Frame::from_profile(p))
    }
    /// O(1), uses the same responsive playfield as map/input geometry.
    pub fn resolve_frame(p:&crate::appearance_profile::Profile,_map_left:i32,frame:crate::modal_ui::Frame)->Self {
        let _=p;
        let top=frame.map.1;
        let bottom=frame.rail_bottom;
        let bounds=Rect(frame.map.2,top,RESERVED_WIDTH,(bottom-top).max(0));
        Self { bounds, rows:Rect(bounds.0+8,bounds.1+8,bounds.2-16,bounds.3-16) }
    }
    pub fn content(self,p:&Painter,actors:&[Creature])->Content { Content::new(p,self,actors) }
    pub fn content_view(self,p:&Painter,v:&View)->Content {
        let mut content=self.content(p,&v.creatures);
        for row in &v.ground_entities {
            let mut card=Card{heart:None,top:content.height,height:0,lines:Vec::new(),bars:Vec::new()};
            let style=crate::typography_ui::resolve(&p.profile,Role::Secondary,RoleStyle::baseline(13.));
            let step=(style.font_size*style.line_height).ceil().max(15.) as i32;
            let mut y=5;
            for line in p.wrap_text_role(Role::Secondary,&format!("{} {}",row.glyph,row.name),(self.rows.2-12).max(1),13) {
                card.lines.push(Line{hover_only:false,text:line,y,size:13,color:row.color,outline:None,black_initial:false});y+=step;
            }
            card.height=y+6;content.height+=card.height;content.cards.push(card);
        }
        content.cards.rotate_right(v.ground_entities.len());
        let ground = v.ground_entities.len();
        let mut top=0;
        let mut divider=None;
        for (i,card) in content.cards.iter_mut().enumerate(){
            if i==ground && ground>0 {
                divider=Some(top+4);
                top+=10;
            }
            card.top=top;top+=card.height;
        }
        content.divider=divider;
        content.height=top;
        content
    }
}
#[derive(Clone,Debug)]
struct Line { hover_only:bool, text:String, y:i32, size:u16, color:super::Rgb, outline:Option<super::Rgb>, black_initial:bool }
#[derive(Clone,Debug)]
struct Bar { y:i32, label:String, value:String, fraction:f64, color:super::Rgb }
#[derive(Clone,Debug)]
struct Card { heart:Option<bool>, top:i32, height:i32, lines:Vec<Line>, bars:Vec<Bar> }
#[derive(Clone,Debug)]
pub struct Content { cards:Vec<Card>, height:i32, layout:Layout, divider:Option<i32> }
impl Content {
    pub fn stair_count(v:&View)->usize {v.ground_entities.partition_point(|g|g.kind==GroundKind::Stair)}
    pub fn creature_index(v:&View,index:usize)->Option<usize>{index.checked_sub(v.ground_entities.len()).filter(|&i|i<v.creatures.len())}
    pub fn actor(v:&View,index:usize)->Option<u64>{Self::creature_index(v,index).map(|i|v.creatures[i].id.to_bits())}
    pub fn ground_index(v:&View,index:usize)->Option<usize>{(index<v.ground_entities.len()).then_some(index)}
    pub fn ground(v:&View,index:usize)->Option<(usize,usize)>{Self::ground_index(v,index).map(|i|{let g=&v.ground_entities[i];(g.x,g.y)})}
    pub fn row_for_ground(_v:&View,index:usize)->usize{index}

    /// O(T²) worst-case text measurement via measured word wrapping, O(T+C)
    /// temporary storage for C admitted creatures and T displayed characters.
    /// Work is bounded by detached observation text, never world population.
    fn new(p:&Painter,layout:Layout,actors:&[Creature])->Self {
        let width=(layout.rows.2-16).max(1);
        let mut cards=Vec::with_capacity(actors.len());let mut top=0;
        for a in actors {
            let mut card=Card{heart:a.has_heart,top,height:0,lines:Vec::new(),bars:Vec::new()};let mut y=5;
            fn text(p:&Painter,card:&mut Card,y:&mut i32,width:i32,s:&str,size:u16,color:super::Rgb) {
                let style=crate::typography_ui::resolve(&p.profile,Role::Secondary,RoleStyle::baseline(size as f64));
                let line=(style.font_size*style.line_height).ceil().max(size as f64+2.) as i32;
                for s in p.wrap_text_role(Role::Secondary,s,width,size) {
                    card.lines.push(Line{hover_only:false,text:s,y:*y,size,color,outline:None,black_initial:false});*y+=line;
                }
            }
            let black_b=a.glyph=="B" && a.color==[0;3] && a.identity_outline.is_none();
            let color=if black_b {p.profile.rgb("ui.colors.primary")} else {a.color};
            text(p,&mut card,&mut y,(width-if a.has_heart.is_some(){17}else{0}).max(1),&format!("{} {}",a.glyph,a.name),12,color);
            if black_b {if let Some(line)=card.lines.first_mut(){line.black_initial=true;}}
            for line in &mut card.lines {line.outline=a.identity_outline;}
            y+=1;
            if !a.shared_resources && layout.bounds.2>=180 {
                for (name,value,max,color,invulnerable) in [
                    ("Health",a.hp,a.max_hp,[255,48,85],a.invulnerable),
                    ("Energy",a.energy,a.max_energy,[36,223,255],false),
                ].into_iter().take(if a.uses_energy {2} else {1}) {
                    let display=if invulnerable {"Invulnerable".into()} else {
                        format!("{:.0}/{}",if name=="Health" {crate::stat_presentation::current_health(value)} else {value},super::resource_max_label(max))
                    };
                    card.bars.push(Bar{y,label:name.into(),value:display,
                        fraction:if invulnerable {1.} else {(value/if max.is_infinite(){100.}else{max.max(1.)}).clamp(0.,1.)},color});
                    y+=26;
                }
            }
            if !a.state.is_empty() && !a.statuses.iter().any(|s| crate::statuses_ui::display_name(&s.name).eq_ignore_ascii_case(&a.state)) {
                let mut chars=a.state.chars();
                let state=chars.next().map(|c|c.to_uppercase().collect::<String>()+chars.as_str()).unwrap_or_default();
                text(p,&mut card,&mut y,width,&state,11,MUTED);
            }
            let conditions_top=y;
            let conditions=crate::statuses_ui::summary(&a.statuses);
            if !conditions.is_empty() {
                text(p,&mut card,&mut y,width,&conditions,11,[193,138,255]);
            }
            if let Some(label)=&a.perception_range {
                // Reserve stable card geometry, but reveal senses only on hover.
                let start=card.lines.len();text(p,&mut card,&mut y,width,label,11,MUTED);
                for line in &mut card.lines[start..] {line.hover_only=true;}
            }
            card.height=y.max(conditions_top+16)+3;top+=card.height;cards.push(card);
        }
        Self{cards,height:top,layout,divider:None}
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
    let content=layout.content_view(p,v);
    p.enemy_list_offset=content.offset(p.enemy_list_offset);
    let old_clip=p.clip;p.clip=Some(layout.bounds.tuple());
    p.clip=Some(layout.rows.tuple());
    if let Some(top)=content.divider {
        let y=layout.rows.1+top-p.enemy_list_offset as i32;
        p.rect(layout.rows.0+4,y,layout.rows.2-8,1,[70,63,81]);
    }
    for (index,card) in content.cards.iter().enumerate() {
        let selected=Content::actor(v,index).is_some_and(|id|p.enemy_hover==Some(id)) || Content::ground(v,index).is_some_and(|at|p.ground_hover==Some(at));
        let y=layout.rows.1+card.top-p.enemy_list_offset as i32;
        if y+card.height<=layout.rows.1 || y>=layout.rows.1+layout.rows.3 {continue;}
        if selected {p.rect(layout.rows.0,y,layout.rows.2-4,card.height-2,[39,44,61]);}
        if let Some(heart)=card.heart {
            let x=layout.rows.0+layout.rows.2-16;
            p.text("♥",x,y+4,13,if heart {[255,116,133]} else {[150,128,151]});
            if !heart{p.text("×",x,y+3,13,[255,56,72]);}
        }
        for line in &card.lines {
            if line.hover_only&&!selected{continue;}
            if let Some(outline)=line.outline {for (dx,dy) in [(-1,0),(1,0),(0,-1),(0,1)] {p.text_role(Role::Secondary,&line.text,layout.rows.0+4+dx,y+line.y+dy,line.size,outline);}}
            let x=layout.rows.0+4;
            if let Some(name)=line.black_initial.then(||line.text.strip_prefix("B")).flatten() {
                let advance=p.measure_text_role(Role::Secondary,"B",line.size).round() as i32;
                p.text_role(Role::Secondary,"B",x,y+line.y,line.size,[0;3]);
                p.text_role(Role::Secondary,name,x+advance,y+line.y,line.size,line.color);
            } else {
                p.text_role(Role::Secondary,&line.text,x,y+line.y,line.size,line.color);
            }
        }
        for bar in &card.bars {
            let w=(layout.rows.2-16).max(1);let yy=y+bar.y;
            super::resource_ui::paint_bar(p,&bar.label,&bar.value,layout.rows.0+4,yy,w,24,bar.color,bar.fraction,11);
        }
        p.rect(layout.rows.0+4,y+card.height-3,(layout.rows.2-16).max(1),1,[36,30,46]);
    }
    p.clip=old_clip;
}
#[cfg(test)] mod tests {
    use super::*;
    fn view()->View {crate::observe::observe(&corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap().run)}
    #[test]
    #[ignore = "opt-in generated mid-depth screenshot"]
    fn generated_mid_depth_screenshot() {
        let path=std::env::var("MID_DEPTH_SCREENSHOT").expect("isolated screenshot path");
        let mut run=corpse_core::debug_floor::create_avatar_run("0-1-2:scrollbar-review-20261113",21).unwrap();
        while !run.offers.is_empty() {
            let id=run.offers[0].clone();corpse_core::progression::choose_offer(&mut run,&id).unwrap();
            corpse_core::debug_floor::advance(&mut run).unwrap();
        }
        corpse_core::debug_floor::enter(&mut run).unwrap();
        if std::env::var_os("SHOW_UPKEEP").is_some() {
            let id="morae-turning-the-wheel";
            if !run.player.abilities.iter().any(|a|a==id){run.player.abilities.push(id.into());}
            if !run.player.ability_reserves.as_ref().is_some_and(|m|m.contains_key(id)) {
                corpse_core::revision50_ability_energy::initialize_reserve(&mut run.player,id,10.,0.).unwrap();
            }
            let reserve=if std::env::var_os("SHOW_RESERVE").is_some(){5.}else{0.};
            run.player.ability_reserves.as_mut().unwrap().get_mut(id).unwrap().energy=reserve;
            run.player.active.push(id.into());
            assert_eq!(corpse_core::abilities::projected_player_upkeep(&run),20.-reserve);
        }
        if std::env::var_os("SHOW_FORTUNES").is_some(){run.player.abilities.push("morae-fortune-teller".into());run.player.conditions.insert("fortunes".into(),1.);run.tick=13.;}
        let mut p=Painter::new();p.resize(1920,1080);p.set_responsive_canvas(true);
        let v=crate::observe::observe_with_profile(&run,&p.profile);
        if std::env::var_os("SHOW_STATUS").is_some(){if let Some(a)=v.creatures.first(){let id=a.id;if let Some(actor)=run.actors.get_mut(&run.depth.to_string()).and_then(|actors|std::sync::Arc::make_mut(actors).iter_mut().find(|a|a.id==id)){actor.conditions.insert("burning".into(),1.);}}}
        let v=crate::observe::observe_with_profile(&run,&p.profile);p.game_at(&v,&std::env::var("INSPECT_SCREENSHOT_TEXT").unwrap_or_default(),false,0.);
        let file=std::fs::File::create(path).unwrap();let mut encoder=png::Encoder::new(file,p.width as u32,p.height as u32);
        encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
        assert_eq!(run.depth,21.);
    }
    #[test] fn overflowing_preview_has_no_scrollbar_and_still_scrolls() {
        let mut v=view();let template=v.creatures[0].clone();
        v.creatures=(0..40).map(|i|{let mut a=template.clone();a.id=1000.+i as f64;a}).collect();
        let mut p=Painter::new();
        let layout=Layout{bounds:Rect(8,100,240,180),rows:Rect(8,100,240,180)};
        let content=layout.content(&p,&v.creatures);
        assert!(content.height>layout.rows.3);
        let bottom=content.scroll(0,-i32::MAX);
        assert!(bottom>0);assert_eq!(content.scroll(bottom,i32::MAX),0);
        for offset in [0,bottom/2,bottom] {
            p.clear([7,11,19]);p.enemy_list_offset=offset;
            let before=p.pixels.clone();paint(&mut p,&v,layout);
            assert_eq!(p.enemy_list_offset,offset);
            let x=p.native_x((layout.rows.0+layout.rows.2-2)as f64) as usize;
            for y in layout.rows.1..layout.rows.1+layout.rows.3 {
                let y=p.native_y(y as f64) as usize;let at=(y*p.width as usize+x)*4;
                assert_eq!(&p.pixels[at..at+4],&before[at..at+4],"scrollbar gutter changed");
            }
        }
    }
    #[test] fn preview_column_survives_supported_sizes_and_right_aligns_map() {
        let mut p=Painter::new();
        for(w,h)in[(1280,800),(1920,1080),(2560,1600),(1537,911),(960,720),(960,1200)] {
            p.resize(w,h);p.set_responsive_canvas(true);
            for(mw,mh)in[(79,29),(40,30),(96,60)] {
                let(cw,ch,ox,oy)=p.map_layout(mw,mh);let frame=p.frame();
                let l=Layout::resolve_frame(&p.profile,ox,frame);
                assert!(l.bounds.2>=RESERVED_WIDTH-16);assert!(l.rows.3>0);
                assert_eq!(ox+cw*mw as i32,frame.map.2-12);assert_eq!(l.bounds.0,frame.map.2);assert_eq!(l.bounds.0+l.bounds.2,frame.sidebar);assert!(oy>=frame.map.1);
                assert!(oy+ch*mh as i32<=frame.map.1+frame.map.3-20);
                let px=p.native_x((ox+cw*(mw as i32-1)+cw/2)as f64);let py=p.native_y((oy+ch*(mh as i32-1)+ch/2)as f64);
                let(x,y)=p.logical_point(px,py,(w as u32,h as u32));
                assert_eq!(((x-ox)/cw,(y-oy)/ch),(mw as i32-1,mh as i32-1));
            }
        }
    }
    #[test] fn zero_health_enemy_cards_preserve_living_fractions_and_exact_bar() {
        let mut v=view();let p=Painter::new();let(_,_,ox,_)=p.map_layout(v.width,v.height);
        let l=Layout::resolve_frame(&p.profile,ox,p.frame());
        v.creatures.truncate(1);v.creatures[0].shared_resources=false;v.creatures[0].invulnerable=false;
        v.creatures[0].max_hp=100.;v.creatures[0].hp=0.25;
        let c=l.content(&p,&v.creatures);
        assert!(c.cards[0].bars.iter().any(|bar|bar.label=="Health"&&bar.value=="1/100"));
        assert_eq!(c.cards[0].bars[0].fraction,0.0025);
        v.creatures[0].hp=0.;let c=l.content(&p,&v.creatures);
        assert!(c.cards[0].bars.iter().any(|bar|bar.label=="Health"&&bar.value=="0/100"));
    }
    #[test] fn every_dense_preview_and_every_long_condition_remains_reachable() {
        let mut v=view();let mut template=v.creatures[0].clone();template.uses_energy=true;
        v.creatures=(0..200).map(|i|{let mut a=template.clone();a.id=1000.+i as f64;a.name=format!("Observed creature {i}");a.statuses=(0..30).map(|j|crate::statuses_ui::Status{name:format!("Condition {j} with a long description"),duration:Some(j as f64+1.)}).collect();a}).collect();
        let p=Painter::new();let(_,_,ox,_)=p.map_layout(v.width,v.height);let l=Layout::resolve_frame(&p.profile,ox,p.frame());let c=l.content(&p,&v.creatures);
        assert!(c.cards[0].height>l.rows.3);
        for(i,card)in c.cards.iter().enumerate(){
            assert!(card.bars.iter().any(|b|b.label=="Health"));assert!(card.bars.iter().any(|b|b.label=="Energy"));
            for line in &card.lines {let at=(card.top+line.y)as usize;let off=c.offset(at);let y=l.rows.1+at as i32-off as i32;assert_eq!(c.hit(l.rows.0+4,y,off),Some(i));}
        }
        assert_eq!(c.hit(l.rows.0-1,l.rows.1,0),None);assert_eq!(c.hit(l.rows.0,l.rows.1+l.rows.3,0),None);
        assert_eq!(c.scroll(0,1),0);assert_eq!(c.scroll(0,-i32::MAX),c.offset(usize::MAX));
        assert_eq!(l.content(&p,&[]).offset(usize::MAX),0);
    }
    #[test] fn previews_preserve_special_resources_and_world() {
        let engine=corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();let before=engine.save().unwrap();let mut v=crate::observe::observe(&engine.run);
        let mut a=v.creatures[0].clone();a.max_energy=f64::INFINITY;a.uses_energy=true;a.invulnerable=true;a.shared_resources=false;v.creatures=vec![a.clone()];
        let mut p=Painter::new();let(_,_,ox,_)=p.map_layout(v.width,v.height);let l=Layout::resolve_frame(&p.profile,ox,p.frame());let c=l.content(&p,&v.creatures);
        assert!(c.cards[0].bars.iter().any(|b|b.value=="Invulnerable"));assert!(c.cards[0].bars.iter().any(|b|b.value.contains('∞')));
        v.creatures[0].shared_resources=true;let c=l.content(&p,&v.creatures);assert!(!c.cards[0].lines.iter().any(|l|l.text.contains("Shares")));assert!(c.cards[0].bars.is_empty());
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
    #[test]
    fn capability_only_change_repaints_both_retained_paths() {
        let engine=corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let mut old=crate::observe::observe(&engine.run);let mut new=crate::observe::observe(&engine.run);
        old.creatures[0].uses_energy=true;new.creatures[0].uses_energy=false;
        let mut p=Painter::new();let mut full=Painter::new();
        let mut retained=super::super::RetainedGame::default();
        retained.update(&mut p,&old,"",0.,true,true).unwrap();
        let before=p.pixels.clone();retained.update(&mut p,&new,"",0.,true,false).unwrap();
        full.game_at(&new,"",false,0.);assert_ne!(p.pixels,before);assert_eq!(p.pixels,full.pixels);
        let mut plan=super::super::tile_damage::TilePlanner::default();let mut p=Painter::new();
        plan.replace_view(None,&old,&p);plan.finish_damage(&mut p);plan.paint_base(&mut p,&old,"",0.);p.tile_clip_enabled=false;plan.clear_frame();
        plan.replace_view(Some(&old),&new,&p);plan.finish_damage(&mut p);plan.paint_base(&mut p,&new,"",0.);p.tile_clip_enabled=false;
        assert_eq!(p.pixels,full.pixels);
    }
}

#[cfg(test)] mod comma_condition_tests {
    use super::*;
    #[test]
    fn comma_rows_wrap_without_duplicate_sleeping_or_empty_rows() {
        let r=corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap().run;
        let mut v=crate::observe::observe(&r);v.creatures.truncate(1);
        let a=&mut v.creatures[0];a.state="sleeping".into();a.uses_energy=false;
        a.statuses=vec![crate::statuses_ui::Status{name:"Sleeping".into(),duration:None},crate::statuses_ui::Status{name:"Flying".into(),duration:None}];
        let p=Painter::new();let(_,_,ox,_)=p.map_layout(v.width,v.height);let l=Layout::resolve_frame(&p.profile,ox,p.frame());
        let c=l.content(&p,&v.creatures);let lines=&c.cards[0].lines;
        assert!(lines.iter().any(|l|l.text=="Sleeping, Flying"));
        assert!(!lines.iter().any(|l|l.text=="Sleeping"||l.text.contains("Energy")));
        v.creatures[0].statuses=(0..30).map(|i|crate::statuses_ui::Status{name:format!("Condition {i} with a long name"),duration:Some(i as f64)}).collect();
        let c=l.content(&p,&v.creatures);
        let text=c.cards[0].lines.iter().filter(|line|line.color==[193,138,255]).map(|line|line.text.as_str()).collect::<Vec<_>>().join(" ");
        assert_eq!(text,crate::statuses_ui::summary(&v.creatures[0].statuses));
        for line in &c.cards[0].lines {assert!(p.measure_text_role(Role::Secondary,&line.text,line.size)<=((l.rows.2-16).max(1))as f64);}
        v.creatures[0].statuses.clear();let c=l.content(&p,&v.creatures);
        assert!(!c.cards[0].lines.iter().any(|line|line.text.is_empty()||line.color==[193,138,255]));
    }
}

#[cfg(test)] mod ground_tests {
    use super::*;
    fn scene()->corpse_core::Run {
        let mut r:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=52.0.into();r.player.x=10.;r.player.y=10.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
        let mut corpse=corpse_core::population::create_actor_revision("rat",900.,11.,10.,false,52.).unwrap();corpse.dead=true.into();
        let mut living=corpse.clone();living.dead=false.into();living.id=901.;living.x=13.;living.state="hunting".into();
        r.actors_at_mut(r.depth).clear();let mut farther=living.clone();farther.id=902.;farther.x=17.;
        r.actors_at_mut(r.depth).push(corpse);r.actors_at_mut(r.depth).push(living);r.actors_at_mut(r.depth).push(farther);
        let f=r.floor_mut().unwrap();let w=f.width();let n=f.tiles.len();
        for y in 1..f.height()-1 {for x in 1..w-1 {f.tiles[y*w+x]="floor".into();}}
        for y in 1..f.height()-1 {f.tiles[y*w+20]="wall".into();}
        f.lights=vec![100.;n];f.environment=corpse_core::Field::Missing;f.doors=corpse_core::Field::Missing;
        f.tiles[10*w+14]="up".into();f.tiles[10*w+15]="down".into();
        f.vents=vec![corpse_core::model::GasVent{at:(10*w+12)as f64,gas:"steam".into(),countdown:5.,phase:"waiting".into(),..Default::default()}].into();
        let item=|id,x|corpse_core::model::GroundItem{x,y:10.,item:corpse_core::model::Item{id,name:format!("Item {id}"),kind:"food".into(),glyph:"%".into(),..Default::default()},..Default::default()};
        r.ground.insert(r.depth.to_string(),vec![item(3.,11.),item(2.,11.),item(1.,65.)].into());
        r
    }
    #[test] fn relocated_two_way_hover_multiple_and_wrapped_names() {
        let mut v=crate::observe::observe(&scene());
        assert_eq!(v.creatures.len(),2);
        v.creatures[1].name="Greater Pale Long-Legged Saidribee Guardian".into();
        for (w,h) in [(1280,800),(1920,1080),(960,1200)] {
            let mut p=Painter::new();p.resize(w,h);p.set_responsive_canvas(true);
            for i in 0..v.creatures.len() {
                let id=v.creatures[i].id;
                let cell=v.cells.iter().find(|c|c.actor_id==Some(id)).unwrap();
                let(cw,ch,ox,oy)=p.map_layout(v.width,v.height);
                crate::refresh_panel_hover(&mut p,&v,Some((ox+cell.x as i32*cw+cw/2,oy+cell.y as i32*ch+ch/2)),true);
                assert_eq!(p.enemy_hover,Some(id.to_bits()));p.game_at(&v,"",false,0.);let map_pixels=p.pixels.clone();
                let layout=Layout::resolve_frame(&p.profile,ox,p.frame());let content=layout.content_view(&p,&v);
                let row=Content::stair_count(&v)+i;let card=&content.cards[row];
                assert_eq!(Content::actor(&v,row),Some(id.to_bits()));
                if i==1 {assert!(card.lines.iter().filter(|l|l.size==13).count()>1);}
                let point=(layout.rows.0+10,layout.rows.1+card.top-p.enemy_list_offset as i32+card.bars[0].y+5);
                crate::refresh_panel_hover(&mut p,&v,Some(point),true);
                assert_eq!(p.enemy_hover,Some(id.to_bits()));p.game_at(&v,"",false,0.);
                assert_eq!(p.pixels,map_pixels,"map and list selection must render identical highlighted actor/card");
                crate::refresh_panel_hover(&mut p,&v,None,false);p.game_at(&v,"",false,0.);
                assert_ne!(p.pixels,map_pixels,"selection must visibly highlight");
            }
        }
    }
    #[test] fn direct_ground_evidence_includes_stacks_and_excludes_hidden_and_memory() {
        let r=scene();let before=serde_json::to_string(&r).unwrap();let v=crate::observe::observe(&r);
        let rows=&v.ground_entities;
        assert_eq!(rows.iter().filter(|g|g.kind==GroundKind::Item).count(),2);
        assert!(rows.iter().all(|g|g.x<20));
        assert!(rows.iter().any(|g|g.kind==GroundKind::Vent));
        assert_eq!(rows.iter().filter(|g|g.kind==GroundKind::Stair).count(),2);
        assert_eq!(Content::stair_count(&v),2);assert_eq!(Content::actor(&v,0),None);
        assert_eq!(Content::actor(&v,2),Some(v.creatures[0].id.to_bits()));
        assert_eq!(Content::ground(&v,2),None);assert_eq!(Content::creature_index(&v,2),Some(0));
        assert_eq!(Content::actor(&v,3),Some(v.creatures[1].id.to_bits()));assert_eq!(Content::creature_index(&v,3),Some(1));
        assert_eq!(Content::ground_index(&v,4),Some(2));assert_eq!(Content::row_for_ground(&v,2),4);
        assert_eq!(Content::ground_index(&v,0),Some(0));assert_eq!(Content::row_for_ground(&v,0),0);
        assert!(rows.iter().any(|g|g.kind==GroundKind::Corpse));
        assert!(rows.windows(2).all(|w|w[0].kind<=w[1].kind));
        assert_eq!(serde_json::to_string(&r).unwrap(),before);
        let mut remembered=r.clone();let item=remembered.ground[&remembered.depth.to_string()][2].item.clone();
        remembered.detected_items=corpse_core::Field::Value(std::iter::once((remembered.depth.to_string(),vec![corpse_core::model::RunDetectedItemsEntryEntry{x:65.,y:10.,item:item.clone()}])).collect());
        remembered.remembered_items=corpse_core::Field::Value(std::iter::once((remembered.depth.to_string(),vec![corpse_core::model::RunRememberedItemsEntryEntry{x:64.,y:10.,item}])).collect());
        assert_eq!(crate::observe::observe(&remembered).ground_entities,*rows);
        let mut hidden=r.clone();hidden.player.x=65.;
        let hidden=crate::observe::observe(&hidden);assert!(hidden.ground_entities.iter().all(|g|g.x>20));
    }
    #[test] fn restores_detached_behavior_without_live_lookup() {
        let engine=corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        let mut v=crate::observe::observe(&engine.run);let p=Painter::new();let(_,_,ox,_)=p.map_layout(v.width,v.height);let l=Layout::resolve_frame(&p.profile,ox,p.frame());
        for (state,label) in [("hunting","Hunting"),("Sleeping","Sleeping"),("sensed","Sensed")] {
            v.creatures[0].state=state.into();let c=l.content_view(&p,&v);
            assert!(c.cards[Content::stair_count(&v)].lines.iter().any(|line|line.text==label));
        }
    }
    #[test] fn hallucinated_corpse_under_item_uses_detached_phase_palette() {
        let mut r=scene();r.actors_at_mut(r.depth).retain(|a|a.dead.value_or(false));r.player.conditions.insert("hallucinating".into(),10.);
        let before=serde_json::to_string(&r).unwrap();let mut v=crate::observe::observe(&r);
        assert!(v.ground_entities.iter().all(|g|g.name.ends_with('?')));
        let index=v.ground_entities.iter().position(|g|g.kind==GroundKind::Corpse).unwrap();
        let palette=v.ground_entities[index].hallucination.clone().expect("corpse presentation must be uncertain");
        for phase in 0..6 {crate::hallucination_ui::apply_view(&mut v,phase);let row=&v.ground_entities[index];assert_eq!((&row.name,&row.glyph,row.color),(&palette[phase].0,&palette[phase].1,palette[phase].2));}
        assert_eq!(serde_json::to_string(&r).unwrap(),before);
        r.ground.clear();let mut exposed=crate::observe::observe(&r);
        for phase in 0..6 {
            crate::hallucination_ui::apply_view(&mut exposed,phase);
            let row=exposed.ground_entities.iter().find(|g|g.kind==GroundKind::Corpse).unwrap();
            let map=exposed.cells.iter().find(|c|c.actor_id==Some(900.)).unwrap();
            assert_eq!(row.glyph,map.glyph);assert_eq!(row.color,map.entity_ink.unwrap().color);
            assert_eq!(row.name,format!("{} corpse?",map.label.trim_end_matches('?')));
        }
    }
    #[test] fn deterministic_group_distance_and_ties() {
        let row=|kind,distance,id|GroundEntity{hallucination:None,kind,distance,id,x:2,y:3,name:"Thing".into(),glyph:"!".into(),color:INK};
        let mut rows=vec![row(GroundKind::Corpse,0.,1),row(GroundKind::Vent,0.,1),row(GroundKind::Item,9.,1),row(GroundKind::Item,1.,2),row(GroundKind::Item,1.,1)];
        sort_ground(&mut rows);assert_eq!(rows.iter().map(|g|(g.kind,g.distance,g.id)).collect::<Vec<_>>(),vec![(GroundKind::Item,1.,1),(GroundKind::Item,1.,2),(GroundKind::Item,9.,1),(GroundKind::Vent,0.,1),(GroundKind::Corpse,0.,1)]);
        let expected=rows.clone();rows.reverse();sort_ground(&mut rows);assert_eq!(rows,expected);
    }
    #[test] fn ground_rows_hit_scroll_and_highlight_repaint_without_creature_indexing() {
        let v=crate::observe::observe(&scene());let mut p=Painter::new();let mut full=Painter::new();
        let(_,_,ox,_)=p.map_layout(v.width,v.height);let l=Layout::resolve_frame(&p.profile,ox,p.frame());let c=l.content_view(&p,&v);
        for(i,card)in c.cards.iter().enumerate().filter(|(i,_)|Content::ground(&v,*i).is_some()){
            let offset=c.reveal(i,0);let y=l.rows.1+card.top-offset as i32+5;
            assert_eq!(c.hit(l.rows.0+5,y,offset),Some(i));assert_eq!(Content::actor(&v,i),None);assert!(Content::ground(&v,i).is_some());
        }
        let mut retained=super::super::RetainedGame::default();
        retained.update(&mut p,&v,"",0.,true,true).unwrap();
        for at in [Some((11,10)),Some((12,10)),None] {
            p.ground_hover=at;full.ground_hover=at;
            retained.update(&mut p,&v,"",0.,false,false).unwrap();full.game_at(&v,"",false,0.);
            assert_eq!(p.pixels,full.pixels,"ground highlight retained parity");
        }
        let mut tile=Painter::new();let mut plan=super::super::tile_damage::TilePlanner::default();let mut keys=None;
        plan.replace_view(None,&v,&tile);
        for at in [None,Some((11,10)),Some((12,10)),None] {
            tile.ground_hover=at;full.ground_hover=at;
            let next=crate::tile_overlay::PanelKeys::read(&tile);next.mark_changes(keys,&tile,&v,&mut plan);keys=Some(next);
            plan.finish_damage(&mut tile);plan.paint_base(&mut tile,&v,"",0.);tile.tile_clip_enabled=false;plan.clear_frame();full.game_at(&v,"",false,0.);
            assert_eq!(tile.pixels,full.pixels,"ground highlight tile parity");
        }
        if let Ok(dir)=std::env::var("GROUND_LIST_SCREENSHOT") {
            p.ground_hover=Some((11,10));p.game_at(&v,"",false,0.);
            let file=std::fs::File::create(dir).unwrap();let mut encoder=png::Encoder::new(file,p.width as u32,p.height as u32);encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
        }
    }
}

#[cfg(test)] mod flush_layout_tests {
    use super::*;
    #[test] fn mockup_columns_are_adjacent_and_names_wrap_inside_their_column() {
        for (w,h) in [(1280,800),(1920,1080),(1537,911),(960,1200)] {
            let mut p=Painter::new();p.resize(w,h);p.set_responsive_canvas(true);
            let frame=p.frame();let l=Layout::resolve_frame(&p.profile,0,frame);
            assert_eq!(l.bounds.0,frame.map.2);
            assert_eq!(l.bounds.0+l.bounds.2,p.hud_left());
            assert_eq!(l.bounds.1,frame.map.1);
            assert_eq!(l.bounds.1+l.bounds.3,frame.rail_bottom);
            assert!(p.chronicle_hit_rect().0+p.chronicle_hit_rect().2<l.bounds.0);
            let text="B Greater Extremely Long Monster Name That Must Remain Inside Its Panel";
            let width=l.rows.2-16;
            for line in p.wrap_text_role(Role::Secondary,text,width,13) {
                assert!(p.measure_text_role(Role::Secondary,&line,13)<=width as f64);
            }
            let ability_bottom=p.game_ability_top()+p.game_ability_row()*p.ability_capacity() as i32;
            assert!(ability_bottom<=p.hud_tracks().items.1);
        }
    }
}

#[cfg(test)] mod resource_grouping_tests {
    use super::*;
    #[test] fn creature_name_resources_then_state_form_one_close_group() {
        let mut v=crate::observe::observe(&corpse_core::temporal::Engine::load(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap().run);
        let a=&mut v.creatures[0];a.state="hunting".into();a.statuses.clear();a.perception_range=None;a.shared_resources=false;a.uses_energy=true;
        let p=Painter::new();let layout=Layout::resolve_frame(&p.profile,0,p.frame());let c=layout.content(&p,&v.creatures);let card=&c.cards[0];
        let state=card.lines.iter().find(|l|l.text=="Hunting").unwrap();
        assert_eq!(card.bars.len(),2);
        assert!(card.bars[0].y<state.y);
        assert_eq!(state.y,card.bars[1].y+26);
        assert!(card.bars[0].y-card.lines[0].y<=22);
        assert!(card.height>=state.y+11+3);
    }
}
