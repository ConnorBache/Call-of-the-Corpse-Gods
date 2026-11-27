//! In-run HUD geometry and pack shortcuts. Draw, hit tests and damage share the tracks.
use crate::{modal_ui::Rect, render::{Painter, View, INK, MUTED, GOLD, game_color, resource_ui}};
use corpse_core::{Item, Run};

pub const HEARTS: usize = 18;
pub const POTIONS: usize = 19;
pub const RAIL_WIDTH: i32 = 128;
pub const HEADER: i32 = 96;
pub const FOOTER: i32 = 40;
pub const ITEM_ROW: i32 = 34;
pub const NAV: [(usize, &str); 4] = [(4, "Pack"), (7, "Bestiary"), (crate::help_ui::ACTION, "?"), (9, "Menu")];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PackHud {
    pub hearts: u32,
    pub potions: u32,
    pub items: Vec<ItemAction>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ItemAction {
    pub id: f64,
    pub label: String,
    pub detail: String,
}

pub fn is_quick_item(run: &Run, i: &Item) -> bool {
    corpse_core::projectiles::is_bow(i) || corpse_core::ahvamitt::is_mask(i)
        || (run.simulation_revision.value_or(29.) >= 50.
            && corpse_core::revision50_amulet_effects::is_amulet(i)
            && !corpse_core::revision52_amulet_tiers::retired(run,i))
}

/// O(I) time and O(I) detached labels, one scan per refreshed gameplay view.
pub fn observe(run: &Run) -> PackHud {
    let mut out = PackHud::default();
    let mut arrows = 0.;
    for i in &run.inventory {
        let n = corpse_core::inventory::quantity(i) as u32;
        match i.kind.as_str() {
            "heart" => out.hearts = out.hearts.saturating_add(n),
            "potion" => out.potions = out.potions.saturating_add(n),
            _ => {}
        }
        if corpse_core::inventory::is_arrow(i) { arrows += n as f64; }
    }
    for bow in [true,false] {
        for i in run.inventory.iter().filter(|i| corpse_core::projectiles::is_bow(i)==bow && is_quick_item(run,i)) {
            let detail=if bow { format!("{arrows:.0} arrows") } else {
                resource_ui::amulet_reserve(i,run.simulation_revision.value_or(29.))
                    .map_or_else(||"Apply".to_owned(), |(n,m)|format!("{n:.0}/{m:.0} NRG"))
            };
            out.items.push(ItemAction { id: i.id, label: if bow {"Bow".into()} else {crate::pack::displayed_name(run,i)}, detail });
        }
    }
    out
}

/// Constant-time tracks: exact reference cuts plus extra horizontal space for the map.
#[derive(Clone, Copy)]
pub struct Tracks {
    pub chronicle: Rect,
    pub resources: Rect,
    pub status: Rect,
    pub nav: Rect,
    pub inspector: Rect,
    pub stats: Rect,
    pub hearts: Rect,
    pub potions: Rect,
    pub divine: Rect,
    pub items: Rect,
}
impl Tracks {
    pub fn resolve(p: &Painter) -> Self {
        let frame=p.frame();
        let map=frame.map;
        let rail=p.logical_width()-frame.sidebar;
        let h=p.logical_height();
        let header=map.1;
        let foot=map.1+map.3;
        let cut=map.2-472;
        let stats_width=216;
        let supply=104;
        let stats_x=map.2-stats_width-2*supply;
        let item_height=((h-header)/3).clamp(128,224);
        let split=h-item_height;
        Self {
            chronicle:Rect(0,0,cut,header),
            resources:Rect(cut,0,map.2-cut,header),
            status:Rect(cut+10,49,map.2-cut-20,24),
            nav:Rect(map.2,0,p.logical_width()-map.2,header),
            inspector:Rect(0,foot,stats_x,h-foot),
            stats:Rect(stats_x,foot,stats_width,h-foot),
            hearts:Rect(map.2-2*supply,foot,supply,h-foot),
            potions:Rect(map.2-supply,foot,supply,h-foot),
            divine:Rect(frame.sidebar,header,rail,split-header),
            items:Rect(frame.sidebar,split,rail,h-split),
        }
    }
    pub fn button(self,slot:usize)->Rect {
        let nav=self.nav;
        let w=(nav.2-24)/2;
        let h=(nav.3-24)/2;
        Rect(nav.0+8+w*(slot%2) as i32+8*(slot%2) as i32,
             nav.1+8+h*(slot/2) as i32+8*(slot/2) as i32,w,h)
    }
    pub fn item_rows(self)->usize { ((self.items.3-27)/ITEM_ROW).max(0) as usize }
    pub fn item_at(self,x:i32,y:i32,offset:usize,count:usize)->Option<usize> {
        let area=Rect(self.items.0+6,self.items.1+24,self.items.2-12,ITEM_ROW*self.item_rows() as i32);
        if !area.contains(x,y){return None}
        let i=offset+((y-area.1)/ITEM_ROW) as usize;
        (i<count).then_some(i)
    }
}
pub fn panel(p:&mut Painter, r:Rect) {
    if !p.intersects_damage(r.0,r.1,r.2,r.3){return}
    p.rect(r.0,r.1,r.2,r.3,[14,13,22]);
    p.border(r.0,r.1,r.2,r.3,[70,63,81]);
}
fn label(p:&mut Painter,r:Rect,t:&str,color:[u8;3]) {
    p.text(t,r.0+8,r.1+6,11,color);
}
pub fn paint_nav(p:&mut Painter,t:Tracks) {
    if !p.intersects_damage(t.nav.0,t.nav.1,t.nav.2,t.nav.3){return}
    panel(p,t.nav);
    for (i,(id,name)) in NAV.into_iter().enumerate() {
        let r=t.button(i);
        let active=p.toolbar_focus==Some(id);
        p.rect(r.0,r.1,r.2,r.3,if active {[47,40,63]} else {[24,21,35]});
        p.border(r.0,r.1,r.2,r.3,if active {GOLD} else {[67,59,78]});
        let text=if id==7&&p.bestiary_unread {"Bestiary !"} else {name};
        let len=p.measure_text_role(crate::typography_ui::Role::Primary,text,13).ceil() as i32;
        p.text(text,r.0+(r.2-len)/2,r.1+(r.3-13)/2,13,if active {GOLD}else{INK});
    }
}
pub fn paint_resources(p:&mut Painter,v:&View,t:Tracks) {
    let r=t.resources;
    if !p.intersects_damage(r.0,r.1,r.2,r.3){return}
    panel(p,r);
    let bw=(r.2-28)/2;
    for (i,(name,current,max,color)) in [
        ("HP",v.hp,v.max_hp,game_color(&p.profile,"ui.colors.health")),
        ("NRG",v.energy,v.max_energy,game_color(&p.profile,"ui.colors.energy"))
    ].into_iter().enumerate(){
        let value=resource_ui::value_text(if i==0 {"Health"} else {"Energy"},current,max,&v.resource_changes[i]);
        let (fill,pending)=resource_ui::bar_fractions(current,max,if i==1 {v.pending_energy_loss}else{0.});
        resource_ui::paint_bar_pending(p,name,&value,r.0+10+i as i32*(bw+8),r.1+10,bw,30,color,fill,13,pending);
    }
    let z=t.status;
    if v.player_meters.is_empty() {label(p,z,"No status effects",MUTED);}
    for (i,m) in v.player_meters.iter().skip(v.player_status_offset).take(3).enumerate(){
        let w=(z.2-16)/3;
        p.condition_meter(m,z.0+i as i32*(w+8),z.1,w);
    }
    let y=r.1+r.3-18;
    p.text(&format!("Depth {:.0}/{} · Lvl {:.0} · Blessings {:.0}/{:.0}",v.display_depth,v.max_depth,v.level,v.blessings,v.blessing_goal),r.0+10,y,11,MUTED);
    let mut x=r.0+r.2-10;
    for (name,color) in v.god_labels.iter().rev().take(3) {
        x-=p.measure_text_role(crate::typography_ui::Role::Secondary,name,10).ceil()as i32;
        p.text(name,x,y,10,*color);x-=9;
    }
}
pub fn paint_footer(p:&mut Painter,v:&View,t:Tracks,notice:&str) {
    let r=t.inspector;
    if p.intersects_damage(r.0,r.1,p.frame().map.2,r.3) {
        for b in [r,t.stats,t.hearts,t.potions]{panel(p,b);}
        label(p,r,"INSPECTOR",MUTED);
        let text=p.fit_text_role(crate::typography_ui::Role::Primary,notice,r.2-16,13);
        p.text(&text,r.0+8,r.1+20,13,INK);
        let z=t.stats;
        let [damage,hp,dr,acc,dodge]=v.player_stats;
        p.text(&format!("DMG {damage:+}  HP {hp:+}  DR {dr}"),z.0+8,z.1+5,12,INK);
        p.text(&format!("ACC {acc}  DODGE {dodge}"),z.0+8,z.1+21,12,MUTED);
        for (b,id,name,count) in [(t.hearts,HEARTS,"Hearts",v.pack_hud.hearts),(t.potions,POTIONS,"Potions",v.pack_hud.potions)]{
            label(p,b,name,MUTED);
            p.text(&count.to_string(),b.0+8,b.1+19,14,if id==HEARTS {[255,115,128]} else {[147,210,246]});
            if p.toolbar_focus==Some(id){p.border(b.0,b.1,b.2,b.3,GOLD);}
        }
    }
}
pub fn paint_item_rail(p:&mut Painter,v:&View,t:Tracks) {
    let r=t.items;
    if !p.intersects_damage(r.0,r.1,r.2,r.3){return}
    panel(p,r);
    label(p,r,"ITEM ABILITIES",MUTED);
    let old=p.clip;
    p.clip=Some((r.0+6,r.1+23,r.2-12,r.3-23));
    let offset=p.item_rail_offset.min(v.pack_hud.items.len().saturating_sub(t.item_rows()));
    if v.pack_hud.items.is_empty(){p.text("No usable items",r.0+8,r.1+26,11,MUTED);}
    for (i,a) in v.pack_hud.items.iter().skip(offset).take(t.item_rows()).enumerate(){
        let y=r.1+24+i as i32*ITEM_ROW;
        let name=p.fit_text_role(crate::typography_ui::Role::Primary,&a.label,r.2-16,12);
        p.text(&name,r.0+8,y,12,INK);
        p.text(&a.detail,r.0+8,y+15,10,[147,210,246]);
        p.rect(r.0+8,y+31,r.2-16,1,[43,37,55]);
    }
    p.clip=old;
}
pub fn paint_divine_header(p:&mut Painter,t:Tracks) {
    panel(p,t.divine);
    label(p,t.divine,"DIVINE ABILITIES",MUTED);
}

#[cfg(test)]
mod tests {
    use super::*;
    use corpse_core::Field;
    fn sample() -> Run {
        let mut r=corpse_core::startup::create_avatar_run("02-03-09:layout-review").unwrap();
        r.inventory.clear();
        for (kind,effect,name,glyph,quantity) in [
            ("heart","", "Rat heart","♥",2.),("potion","healing","Potion of Healing","!",2.),
            ("potion","energy","Potion of Energy","!",1.),
        ] {
            for _ in 0..quantity as usize {
                let id=r.next_id;r.next_id+=1.;
                r.inventory.push(Item {id,name:name.into(),kind:kind.into(),glyph:glyph.into(),
                    effect:if effect.is_empty(){Field::Missing}else{effect.to_owned().into()},identified:true.into(),..Default::default()});
            }
        }
        let id=r.next_id;r.next_id+=1.;r.inventory.push(corpse_core::equipment::make_equipment("Bow",id).unwrap());
        let id=r.next_id;r.next_id+=1.;r.inventory.push(corpse_core::equipment::make_arrows(id,8.));
        let id=r.next_id;r.next_id+=1.;
        r.inventory.push(corpse_core::revision50_amulet_effects::make_amulet_for_revision(
            corpse_core::revision50_amulet_effects::Effect::Firebolt,id,52.).unwrap());
        r
    }

    #[test]
    fn mockup_tracks_and_shortcut_admission() {
        let r=sample();
        let summary=observe(&r);
        assert_eq!((summary.hearts,summary.potions),(2,3));
        assert_eq!(summary.items[0].label,"Bow");
        assert!(matches!(crate::pack::quick_item_action(&r,summary.items[0].id),
            crate::pack::PackAction::Target(crate::pack::PackTarget::Shoot(_))));
        let start=serde_json::to_vec(&r).unwrap();
        let mut pack=crate::pack::PackView::observe(&r);
        let order=pack.display_order();
        for kind in ["heart","potion"] {
            pack.focus_kind(&r,Some(kind));
            assert_eq!(pack.display_order(),order,"must not reorder or discard omitted pack items");
        }
        assert_eq!(start,serde_json::to_vec(&r).unwrap());

        let mut p=Painter::new();
        p.resize(1920,1080);
        p.set_responsive_canvas(true);
        let t=Tracks::resolve(&p);
        assert_eq!(p.frame().map,Rect(0,96,1024,584));
        assert_eq!(p.frame().sidebar,1152);
        let (cw,ch,x,y)=p.map_layout(79,29);
        assert_eq!((cw,ch),(12,20));
        assert!(p.frame().map.contains(x+79*cw-1,y+29*ch-1));
        for (slot,(id,_)) in NAV.into_iter().enumerate(){
            let Rect(x,y,w,h)=t.button(slot);
            assert_eq!(p.toolbar_action(x+w/2,y+h/2),Some(id));
        }
        assert_eq!(p.toolbar_action(t.hearts.0+4,t.hearts.1+4),Some(HEARTS));
        assert_eq!(p.toolbar_action(t.potions.0+4,t.potions.1+4),Some(POTIONS));
        assert_eq!(t.item_at(t.items.0+8,t.items.1+26,0,summary.items.len()),Some(0));
    }

    #[test]
    #[ignore = "Optional staged preview fixture; uses a real generated floor and engine save"]
    fn mockup_capture_fixture() {
        let Some(dir)=std::env::var_os("CORPSE_MOCKUP_CAPTURE") else{return};
        let dir=std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut r=sample();
        r.player.hp=82.;r.player.energy=78.;
        r.player.stats.perception=220.;
        let d=r.depth;
        let (x,y,w,h) = {
            let f=r.floor().unwrap();
            let (w,h)=(f.width(),f.height());
            let mut best=(0,0,0);
            for y in 3..h-3 {for x in 3..w-3 {
                if f.tiles[y*w+x]!="floor" {continue;}
                let open=|x:usize,y:usize| f.tiles[y*w+x]=="floor";
                let orth=(open(x+1,y) as usize)+(open(x-1,y) as usize)+(open(x,y+1) as usize)+(open(x,y-1) as usize);
                if orth<3 {continue;}
                let score=(y-3..=y+3).flat_map(|j|(x-3..=x+3).map(move|i|(i,j))).filter(|&(i,j)|open(i,j)).count();
                if score>best.0{best=(score,x,y);}
            }}
            assert!(best.0>0);
            (best.1,best.2,w,h)
        };
        r.player.x=x as f64;r.player.y=y as f64;
        let mut open = Vec::new();
        let f=r.floor().unwrap();
        for j in y.saturating_sub(5)..=(y+5).min(h-2) {for i in x.saturating_sub(7)..=(x+7).min(w-2) {
            let dist=i.abs_diff(x).max(j.abs_diff(y));
            if dist>=2 && j.abs_diff(y)<=2 && f.tiles[j*w+i]=="floor"{open.push((i,j));}
        }}
        open.sort_by_key(|&(i,j)|(j.abs_diff(y),i.abs_diff(x)));
        let mut candidates = open.iter().copied();
        for (n,actor) in r.actors_at_mut(d).iter_mut().take(2).enumerate() {
            if let Some((x,y))=candidates.nth(1) {
                actor.x=x as f64;actor.y=y as f64;
                actor.heartless=(n==1).into();
                actor.state="sleeping".into();
            }
        }
        let f=r.floor_mut().unwrap();f.seen.fill(true);f.lights.fill(65.);
        let engine=corpse_core::temporal::Engine::new(r).unwrap();
        let save=crate::session_store::encode(&engine,crate::session_store::Kind::Genuine).unwrap();
        std::fs::write(dir.join("showcase-save.json"),save).unwrap();
        let run=&engine.run;
        let view=crate::observe::observe(run);
        let coords=view.creatures.iter().filter_map(|a|view.cells.iter().find(|c|c.actor_id==Some(a.id)).map(|c|(a.name.clone(),c.x,c.y))).collect::<Vec<_>>();
        let mut keys=Vec::new();
        for (dx,dy,key) in [(1,0,"KP_6"),(-1,0,"KP_4"),(0,1,"KP_2"),(0,-1,"KP_8")] {
            let f=run.floor().unwrap();
            if f.tiles[(y as i32+dy)as usize*w+(x as i32+dx)as usize]=="floor" {keys.push(key);}
        }
        std::fs::write(dir.join("scene.json"),serde_json::to_string_pretty(&serde_json::json!({
            "player":[x,y],"enemies":coords,"keys":keys
        })).unwrap()).unwrap();
    }
}
