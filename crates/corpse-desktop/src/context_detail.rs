//! Read-only contextual projections for HUD right-click, with carried-item
//! actions admitted through the same command and targeting paths as Pack.
use crate::{inspection_ui, modal_ui::{DetailAnchor,Rect}, pack, render::{Painter,View,INK,GOLD}, typography_ui::Role};
use corpse_core::{Command, Run};

pub const SHEET: Rect=Rect(370, 228, 540, 344);
const PAD: i32 = 20;
const ACTION_WIDTH: i32 = 150;
const ACTION_BOTTOM: i32 = 144;
struct TextLayout { bounds: Rect, body_width: i32, top: i32, line: i32, visible: usize, rows: Vec<String> }
impl TextLayout {
    fn actions(&self) -> [(Rect,&'static str);2] {
        let Rect(x,y,w,_) = self.bounds;
        let left = x + w - 24 - ACTION_WIDTH;
        [(Rect(left,y+62,ACTION_WIDTH,37),"Drop"),(Rect(left,y+107,ACTION_WIDTH,37),"Throw")]
    }
}
fn layout(p: &Painter, title: &str, copy: &str, actions: bool) -> TextLayout {
    let copy = copy.strip_prefix(title).and_then(|s| s.strip_prefix('\n')).unwrap_or(copy);
    let actions_space = if actions { ACTION_WIDTH + 18 } else { 0 };
    let natural_width = (p.widest_text_line(Role::Tooltips, copy, 15) + actions_space)
        .max(p.widest_text_line(Role::Headings, title, 23));
    let width = (natural_width + PAD * 2).max(if actions { 352 } else { 132 }).min(SHEET.2);
    let body_width = (width - 2*PAD - actions_space).max(1);
    let rows = p.compact_text_rows(Role::Tooltips, copy, body_width, 15);
    let style = crate::typography_ui::resolve(&p.profile, Role::Tooltips, crate::typography_ui::RoleStyle::baseline(15.));
    let line = ((style.font_size * style.line_height).ceil() as i32).max(20);
    let heading = crate::typography_ui::resolve(&p.profile, Role::Headings, crate::typography_ui::RoleStyle::baseline(23.));
    let top = if title.trim().is_empty() { PAD } else { PAD + heading.font_size.ceil() as i32 + 12 };
    let height = (top + rows.len() as i32 * line).max(if actions { ACTION_BOTTOM } else { 0 }).saturating_add(32).min(SHEET.3);
    let visible = ((height-top-32)/line).max(0) as usize;
    TextLayout { bounds: Rect(SHEET.0,SHEET.1,width,height), body_width, top, line, visible, rows }
}
pub enum Detail {
    Enemy(inspection_ui::Report),
    Text { title:String, copy:String, ring:Option<f64>, scroll:usize },
}
impl Detail {
    pub fn text(title:String,copy:String,ring:Option<f64>)->Self {
        Self::Text{title,copy,ring,scroll:0}
    }
    pub fn carried(run:&Run,item:&corpse_core::Item,actions:bool)->Self {
        Self::text(pack::ground_name(run,item),pack::describe_item(run,item),actions.then_some(item.id))
    }
    pub fn ground(run:&Run,row:&crate::render::enemy_list::GroundEntity)->Option<Self> {
        if row.kind!=crate::render::enemy_list::GroundKind::Item{return None;}
        let r=crate::dream_ui::observation_run(run);
        let g=r.ground.get(&r.depth.to_string())?.iter().find(|g|
            g.item.id.to_bits()==row.id && (g.x,g.y)==(row.x as f64,row.y as f64)
                && crate::dream_ui::item_visible(&r,&g.item))?;
        Some(Self::carried(&r,&g.item,false))
    }
    pub fn enemy(run:&Run,v:&View,index:usize)->Option<Self> {
        let actor=v.creatures.get(index)?;
        let cell=v.cells.iter().find(|c|c.actor_id==Some(actor.id))?;
        let mut report=v.terrain_snapshot.as_deref().map_or_else(
            || inspection_ui::observe_tile(run,cell.x,cell.y),
            |terrain|inspection_ui::observe_tile_with_terrain(run,cell.x,cell.y,terrain));
        report.item_hover.clear();
        report.creature.as_ref().is_some_and(|c|c.hover_available).then_some(Self::Enemy(report))
    }
    pub fn bounds(&self,p:&Painter)->Rect {
        match self { Self::Text{title,copy,ring,..} => layout(p,title,copy,ring.is_some()).bounds, Self::Enemy(_) => SHEET }
    }
    pub fn click(&self,p:&Painter,x:i32,y:i32)->pack::PackAction {
        let Self::Text{title,copy,ring:Some(id),..}=self else{return pack::PackAction::None;};
        let [(drop,_),(throw,_)]=layout(p,title,copy,true).actions();
        if drop.contains(x,y){pack::PackAction::Command(Command::Drop{id:*id})}
        else if throw.contains(x,y){pack::PackAction::Target(pack::PackTarget::Throw(*id))}
        else{pack::PackAction::None}
    }
    pub fn scroll(&mut self,delta:i32) {
        if let Self::Text{scroll,..}=self { *scroll=if delta<0{scroll.saturating_sub(delta.unsigned_abs() as usize)}else{scroll.saturating_add(delta as usize)}; }
    }
    pub fn damage(&self,p:&Painter,anchor:DetailAnchor)->Option<crate::render::damage::Rect> {
        match self {
            Self::Enemy(_)=>{
                // Text height determines whether the card flips above the row.
                // Cover both possible orientations without wrapping the card twice.
                let upper=anchor.enemy_bounds(p,620);
                let lower=anchor.enemy_bounds(p,1);
                let y=upper.1.min(lower.1);
                let bottom=p.logical_height().min((upper.1+upper.3).max(lower.1+upper.3));
                p.logical_damage_rect(upper.0,y,upper.2,bottom-y)
            }
            Self::Text{..}=>p.detail_damage_rect(self.bounds(p),anchor),
        }
    }
    pub fn paint(&self,p:&mut Painter,anchor:DetailAnchor) {
        match self {
            Self::Enemy(report)=>inspection_ui::paint_pinned_hover(report,p,anchor),
            Self::Text{title,copy,ring,scroll} => {
                let l = layout(p,title,copy,ring.is_some());
                p.anchored_sheet(l.bounds,anchor,|p|{
                    let Rect(x,y,w,h)=l.bounds;
                    p.rect(x,y,w,h,[20,17,29]);p.border(x,y,w,h,[111,88,131]);
                    let head = p.fit_text_role(Role::Headings,title,w-2*PAD,23);
                    p.text_role(Role::Headings,&head,x+PAD,y+18,23,GOLD);
                    let clip=p.clip;
                    p.clip=Some((x+PAD,y+l.top,l.body_width,l.visible as i32*l.line));
                    let first=(*scroll).min(l.rows.len().saturating_sub(l.visible));
                    for (index,line) in l.rows.iter().skip(first).take(l.visible).enumerate() {
                        p.text_role(Role::Tooltips,line,x+PAD,y+l.top+index as i32*l.line,15,INK);
                    }
                    p.clip=clip;
                    if ring.is_some() {
                        for (Rect(xx,yy,ww,hh),name) in l.actions() {
                            p.rect(xx,yy,ww,hh,[45,36,55]);p.border(xx,yy,ww,hh,[125,99,143]);
                            p.text_role(Role::Secondary,name,xx+12,yy+8,15,INK);
                        }
                    }
                    p.close_hint(x+PAD,y+h-24);
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::enemy_list::{GroundEntity,GroundKind};

    fn row(kind:GroundKind,id:f64,x:usize,y:usize)->GroundEntity {
        GroundEntity{hallucination:None,kind,x,y,id:id.to_bits(),name:"Floor item".into(),glyph:"!".into(),color:INK,distance:1.}
    }
    #[test]
    fn ground_detail_uses_exact_clicked_item_without_revealing_potions() {
        let mut r=corpse_core::startup::create_avatar_run("02-03-09:right-click-tests").unwrap();
        let (x,y)=(r.player.x,r.player.y);
        let potion=corpse_core::Item{id:90_001.,name:"Unknown potion".into(),kind:"potion".into(),glyph:"!".into(),effect:"invisibility".to_owned().into(),..Default::default()};
        let ring=corpse_core::Item{id:90_002.,name:"Ring of Flight".into(),kind:"ring".into(),glyph:"=".into(),effect:"flight".to_owned().into(),..Default::default()};
        // Keep both on the same tile so selection must use item identity, not only coordinates.
        let d=r.depth;
        r.ground_at_mut(d).push(corpse_core::GroundItem{x,y,item:potion.clone(),..Default::default()});
        r.ground_at_mut(d).push(corpse_core::GroundItem{x,y,item:ring.clone(),..Default::default()});
        let Detail::Text{copy,ring:actions,..}=Detail::ground(&r,&row(GroundKind::Item,potion.id,x as usize,y as usize)).unwrap() else {panic!("not an item detail");};
        assert_eq!(copy,pack::describe_item(&r,&potion));
        assert!(copy.to_lowercase().contains("invisibility"),"R52 makes ordinary potions known");
        assert!(actions.is_none());
        let mut legacy=r.clone();
        legacy.simulation_revision=49.0.into();
        legacy.known_potions.clear();
        let Detail::Text{copy,..}=Detail::ground(&legacy,&row(GroundKind::Item,potion.id,x as usize,y as usize)).unwrap() else {panic!("not a legacy item detail");};
        assert!(copy.contains("Might be:") && !copy.to_lowercase().contains("potion of invisibility"),
            "legacy tooltip lists possibilities without identifying the effect: {copy}");
        for kind in [GroundKind::Stair,GroundKind::Corpse,GroundKind::Vent] {
            assert!(Detail::ground(&r,&row(kind,ring.id,x as usize,y as usize)).is_none());
        }
        let detail=Detail::ground(&r,&row(GroundKind::Item,ring.id,x as usize,y as usize)).unwrap();
        let p=Painter::new();
        let carried=Detail::carried(&r,&ring,true);
        let Detail::Text{title,copy,..}=&carried else { panic!("not a ring"); };
        let [(drop,_),(throw,_)]=layout(&p,title,copy,true).actions();
        assert_eq!(detail.click(&p,drop.0+2,drop.1+2),pack::PackAction::None);
        assert_eq!(carried.click(&p,drop.0+2,drop.1+2),pack::PackAction::Command(Command::Drop{id:ring.id}));
        assert_eq!(carried.click(&p,throw.0+2,throw.1+2),pack::PackAction::Target(pack::PackTarget::Throw(ring.id)));
    }
}
