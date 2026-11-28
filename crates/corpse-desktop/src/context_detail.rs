//! Read-only contextual projections for HUD right-click, with carried-item
//! actions admitted through the same command and targeting paths as Pack.
use crate::{inspection_ui, modal_ui::Rect, pack, render::{Painter,View,INK,GOLD}, typography_ui::Role};
use corpse_core::{Command, Run};

pub const SHEET: Rect=Rect(370, 228, 540, 344);
const DROP: Rect=Rect(736, 290, 150, 37);
const THROW: Rect=Rect(736, 335, 150, 37);
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
    pub fn click(&self,x:i32,y:i32)->pack::PackAction {
        let Self::Text{ring:Some(id),..}=self else{return pack::PackAction::None;};
        if DROP.contains(x,y){pack::PackAction::Command(Command::Drop{id:*id})}
        else if THROW.contains(x,y){pack::PackAction::Target(pack::PackTarget::Throw(*id))}
        else{pack::PackAction::None}
    }
    pub fn scroll(&mut self,delta:i32) {
        if let Self::Text{scroll,..}=self { *scroll=if delta<0{scroll.saturating_sub(delta.unsigned_abs() as usize)}else{scroll.saturating_add(delta as usize)}; }
    }
    pub fn damage(&self,p:&Painter)->Option<crate::render::damage::Rect> {
        match self {
            Self::Enemy(_)=>{let b=crate::modal_ui::pinned_hover_bounds(p,620);p.logical_damage_rect(b.0,b.1,b.2,b.3)}
            Self::Text{..}=>p.dock_damage_rect(SHEET),
        }
    }
    pub fn paint(&self,p:&mut Painter) {
        match self {
            Self::Enemy(report)=>inspection_ui::paint_pinned_hover(report,p),
            Self::Text{title,copy,ring,scroll}=>p.docked_sheet(SHEET,|p|{
                let Rect(x,y,w,h)=SHEET;
                p.rect(x,y,w,h,[20,17,29]);p.border(x,y,w,h,[111,88,131]);
                p.text_role(Role::Headings,title,x+20,y+18,23,GOLD);
                let clip=p.clip;
                let body_w=if ring.is_some(){340}else{w-40};
                let body_y=y+68;
                let body_h=h-112;
                p.clip=Some((x+20,body_y,body_w,body_h));
                let copy=copy.strip_prefix(title).and_then(|s|s.strip_prefix('\n')).unwrap_or(copy);
                let lines=p.wrap_text_role(Role::Tooltips,copy,body_w-12,15);
                let max=(body_h/22)as usize;
                for (index,line) in lines.iter().skip((*scroll).min(lines.len().saturating_sub(max))).take(max).enumerate() {
                    p.text_role(Role::Tooltips,line,x+20,body_y+index as i32*22,15,INK);
                }
                p.clip=clip;
                if ring.is_some() {
                    for (Rect(xx,yy,ww,hh),name) in [(DROP,"Drop"),(THROW,"Throw")] {
                        p.rect(xx,yy,ww,hh,[45,36,55]);p.border(xx,yy,ww,hh,[125,99,143]);
                        p.text_role(Role::Secondary,name,xx+12,yy+8,15,INK);
                    }
                }
                p.close_hint(x+20,y+h-28);
            }),
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
        assert_eq!(detail.click(DROP.0+2,DROP.1+2),pack::PackAction::None);
        let carried=Detail::carried(&r,&ring,true);
        assert_eq!(carried.click(DROP.0+2,DROP.1+2),pack::PackAction::Command(Command::Drop{id:ring.id}));
        assert_eq!(carried.click(THROW.0+2,THROW.1+2),pack::PackAction::Target(pack::PackTarget::Throw(ring.id)));
    }
}
