//! In-run HUD geometry and pack shortcuts. Draw, hit tests and damage share the tracks.
use crate::{modal_ui::Rect, render::{Painter, View, INK, MUTED, GOLD, game_color, resource_ui}};
use corpse_core::{Item, Run};

pub const HEARTS: usize = 18;
pub const POTIONS: usize = 19;
pub const RAIL_WIDTH: i32 = 166;
pub const HEADER: i32 = 82;
pub const FOOTER: i32 = 116;
pub const MAP_TILE: (i32, i32) = (12,18);
pub const DIVINE_SLOTS: usize = 15;
pub const ITEM_SLOTS: usize = 14; // ten amulets, bow, Dreamwalk, Masque, Borge shard
pub const ABILITY_INSET: i32 = 3;
const MIN_ABILITY_ROW: i32 = 18;
pub const YELLOW: [u8;3] = [255,218,74];
pub const RAIL_INSET: i32 = 8;
pub const STATUS_SLOTS: usize = 4;
pub const NAV: [(usize, &str); 3] = [(7, "Bestiary"), (crate::help_ui::ACTION, "?"), (9, "Main Menu")];
const NAV_WIDTH: i32 = 168;
const DEPTH_WIDTH: i32 = 80;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PackHud {
    pub hearts: u32,
    pub potions: u32,
    pub heart_capacity: Option<u32>,
    pub potion_capacity: Option<u32>,
    pub items: Vec<ItemAction>,
    pub rings: Vec<String>,
    pub shards: u32,
    pub ring_capacity: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ItemAction {
    pub source: QuickSource,
    pub label: String,
    pub arrows: Option<u32>,
    pub reserve: Option<(f64,f64)>,
    pub tier: Option<u8>,
    pub hint: Option<String>,
}

#[derive(Clone,Copy,Debug,PartialEq)]
pub enum QuickSource { Item(f64), Dreamwalk }
pub fn activate_quick(run:&Run,source:QuickSource)->crate::pack::PackAction {
    match source {
        QuickSource::Item(id)=>crate::pack::quick_item_action(run,id),
        QuickSource::Dreamwalk=>{
            use corpse_core::dreamwalking as dream;
            let id=if dream::unlocked(run){dream::DREAMWALK_ID}else if dream::active(run){dream::WAKE_UP_ID}else{
                return crate::pack::PackAction::Notice("Dreamwalk is not available.".into());
            };
            crate::pack::PackAction::Command(corpse_core::Command::Ability{id:id.into(),
                target:corpse_core::Field::Missing,x:corpse_core::Field::Missing,y:corpse_core::Field::Missing})
        },
    }
}
pub fn is_quick_item(run: &Run, i: &Item) -> bool {
    corpse_core::projectiles::is_bow(i) || corpse_core::ahvamitt::is_mask(i) || (corpse_core::borge::is_shard(i) && run.simulation_revision.value_or(29.)>=50.)
        || (run.simulation_revision.value_or(29.) >= 50.
            && corpse_core::revision50_amulet_effects::is_amulet(i)
            && !corpse_core::revision52_amulet_tiers::retired(run,i))
}

/// O(I+B) time and O(I) detached labels for I inventory entries and B abilities.
pub fn observe(run: &Run) -> PackHud {
    let mut out = PackHud::default();
    // Older replays have one shared capacity, not separate heart and potion limits.
    if corpse_core::revision50_inventory::enabled(run) {
        let capacity = corpse_core::revision50_inventory::capacity(run);
        out.heart_capacity = Some(capacity.hearts as u32);
        out.ring_capacity = Some(capacity.rings as u32);
        out.potion_capacity = Some(capacity.potions as u32);
    }
    let mut arrows = 0.;
    for i in &run.inventory {
        let n = corpse_core::inventory::quantity(i) as u32;
        match i.kind.as_str() {
            "heart" => out.hearts = out.hearts.saturating_add(n),
            "potion" => out.potions = out.potions.saturating_add(n),
            "ring" if corpse_core::revision50_inventory::active(run,i) => out.rings.push(crate::pack::ground_name(run,i)),
            _ if corpse_core::phantasmii_generation::is_shard(i) => out.shards = out.shards.saturating_add(n),
            _ => {}
        }
        if corpse_core::inventory::is_arrow(i) { arrows += n as f64; }
    }
    let revision=run.simulation_revision.value_or(29.);
    if revision>=34. {
        if let Some(mask)=run.inventory.iter().find(|i|corpse_core::ahvamitt::is_mask(i)) {
            let cooldown=corpse_core::ahvamitt::cooldown(run);
            out.items.push(ItemAction{source:QuickSource::Item(mask.id),label:"Ahvamitt Teleport".into(),
                arrows:None,reserve:None,tier:None,hint:Some(if cooldown==0{"Ready".into()}else{format!("{cooldown} turns")})});
        }
    }
    if corpse_core::dreamwalking::unlocked(run)||corpse_core::dreamwalking::active(run) {
        let active=corpse_core::dreamwalking::active(run);
        out.items.push(ItemAction{source:QuickSource::Dreamwalk,label:if active&&!corpse_core::dreamwalking::unlocked(run){"Wake Up".into()}else{"Dreamwalk".into()},
            arrows:None,reserve:None,tier:None,hint:Some(if active{"ON"}else{"OFF"}.into())});
    }
    for bow in [true,false] {
        for i in run.inventory.iter().filter(|i| corpse_core::projectiles::is_bow(i)==bow && is_quick_item(run,i)&&!corpse_core::ahvamitt::is_mask(i)) {
            let label=if bow {"Bow".into()} else {crate::pack::ground_name(run,i)};
            let tier=if !bow && corpse_core::revision52_amulet_tiers::enabled(run) {
                corpse_core::revision52_amulet_tiers::profile(run,i).map(|p|p.tier)
            } else {None};
            out.items.push(ItemAction{source:QuickSource::Item(i.id),label,arrows:bow.then_some(arrows as u32),
                reserve:resource_ui::amulet_reserve(i,revision),tier,
                hint:corpse_core::borge::is_shard(i).then(||"Throw".into())});
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
    pub depth: Rect,
    pub nav: Rect,
    pub inspector: Rect,
    pub stats: Rect,
    pub gods: Rect,
    pub rings: Rect,
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
        let width=p.logical_width();
        // The player bars begin at column 87.5 of the reference grid, before the side rails.
        let resources_x=map.2-248;
        let depth_x=resources_x-DEPTH_WIDTH;
        let cut=depth_x-NAV_WIDTH;
        let stats_width=192;
        let supply=126;
        let stats_x=map.2-stats_width-2*supply;
        let footer_height=h-foot;
        let footer_split=foot+(footer_height*43)/100;
        let usable=(h-header-4*ABILITY_INSET).max(0);
        let split=header+2*ABILITY_INSET+usable*DIVINE_SLOTS as i32/(DIVINE_SLOTS+ITEM_SLOTS) as i32;
        let gods_width=264.min(stats_x);
        Self {
            chronicle:Rect(0,0,cut,header),
            resources:Rect(resources_x,0,width-resources_x,header),
            status:Rect(resources_x+10,29,width-resources_x-20,21),
            nav:Rect(cut,0,NAV_WIDTH,header),
            depth:Rect(depth_x,0,DEPTH_WIDTH,header),
            inspector:Rect(0,foot,stats_x,footer_split-foot),
            gods:Rect(0,footer_split,gods_width,h-footer_split),
            stats:Rect(stats_x,foot,stats_width,footer_split-foot),
            rings:Rect(gods_width,footer_split,stats_x+stats_width-gods_width,h-footer_split),
            hearts:Rect(map.2-2*supply+4,foot+4,supply-8,footer_height-8),
            potions:Rect(map.2-supply+4,foot+4,supply-8,footer_height-8),
            divine:Rect(frame.sidebar,header,rail,split-header),
            items:Rect(frame.sidebar,split,rail,h-split),
        }
    }
    pub fn button(self,slot:usize)->Rect {
        let Rect(x,y,w,h)=self.nav;
        let split=(h+4)/2;
        match slot {
            2=>Rect(x+5,y+5,w-10,split-6),
            1=>Rect(x+5,y+split+3,w/2-7,h-split-8),
            _=>Rect(x+w/2+2,y+split+3,w/2-7,h-split-8),
        }
    }
    pub fn vital(self, slot: usize) -> Rect {
        let r = self.resources;
        let w = (r.2 - 28) / 2;
        Rect(r.0 + 10 + slot as i32 * (w + 8), r.1 + 4, w, 23)
    }
    pub fn divine_rows(self)->RailRows {RailRows::new(self.divine,DIVINE_SLOTS)}
    pub fn item_geometry(self)->RailRows {RailRows::new(self.items,ITEM_SLOTS)}
    pub fn item_rows(self)->usize {self.item_geometry().count}
    pub fn item_at(self,x:i32,y:i32,offset:usize,count:usize)->Option<usize> {
        let i=offset+self.item_geometry().index(x,y)?;
        (i<count).then_some(i)
    }
}
/// Shared O(1) geometry for paint and hit tests. Distribution by cumulative
/// edges gives each leftover pixel to a row instead of leaving a bottom gap.
#[derive(Clone,Copy)]
pub struct RailRows {
    pub area:Rect,
    pub count:usize,
}
impl RailRows {
    fn new(r:Rect,max:usize)->Self {
        let height=(r.3-2*ABILITY_INSET).max(0);
        Self{area:Rect(r.0+4,r.1+ABILITY_INSET,(r.2-8).max(0),height),
            count:max.min((height/MIN_ABILITY_ROW) as usize)}
    }
    pub fn row(self,i:usize)->Rect {
        let Rect(x,y,w,h)=self.area;
        let n=self.count.max(1) as i32;
        let top=h*i as i32/n;
        Rect(x,y+top,w,h*(i as i32+1)/n-top)
    }
    pub fn index(self,x:i32,y:i32)->Option<usize> {
        if self.count==0 || !self.area.contains(x,y){return None;}
        Some((((y-self.area.1+1)*self.count as i32-1)/self.area.3) as usize)
    }
}
pub fn panel(p:&mut Painter, r:Rect) {
    if !p.intersects_damage(r.0,r.1,r.2,r.3){return}
    p.rect(r.0,r.1,r.2,r.3,[14,13,22]);
    p.border(r.0,r.1,r.2,r.3,[70,63,81]);
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
        let size=12;
        let len=p.measure_text_role(crate::typography_ui::Role::Primary,text,size).ceil() as i32;
        p.text(text,r.0+(r.2-len)/2,r.1+(r.3-i32::from(size))/2,size,if active {GOLD}else{INK});
    }
}
/// Record a display class, not gameplay state. Untimed next-action effects
/// remain temporary even without a turn countdown.
pub fn status_tint(name:&str,duration:Option<f64>,now:f64)->[u8;3] {
    let name=crate::statuses_ui::display_name(name);
    if duration.is_none() && !matches!(name,"Lucky"|"Unlucky"|"Distracted"|"Stuck"|"Held"|"Silenced"|"Charging moonbeam"|"Riptide") {
        return [193,138,255];
    }
    match name {
        "Burning"=>burning_ink(now),
        "Hemophilia"|"Bleeding"=>[255,84,109],
        _ if name.starts_with("Envenomed")||name.starts_with("Poisoned")=>[113,240,91],
        "Confused"|"Hallucinating"|"Disoriented"=>[255,220,112],
        "Slowed"|"Paralyzed"|"Rooted"|"Held"|"Stuck"=>[90,222,244],
        "Haste"|"Flying"|"Lucky"|"Emboldened"=>[255,191,79],
        "Silenced"|"Unlucky"|"Corroded"|"Distracted"=>[243,139,188],
        _=>[122,194,255],
    }
}
pub fn burning_ink(now:f64)->[u8;3] {
    let pulse=(1.-(now * std::f64::consts::TAU / 840.).cos())*0.5;
    [255, (117.+77.*pulse).round() as u8, (41.+25.*pulse).round() as u8]
}
pub fn status_burning_visible(v:&View)->bool {
    v.player_meters.iter().skip(v.player_status_offset).take(STATUS_SLOTS)
        .any(|m|m.duration.is_some() && m.name=="Burning")
}
pub fn enemy_burning_visible(v:&View)->bool {
    v.creatures.iter().any(|a|a.statuses.iter().any(|s|s.duration.is_some()&&s.name=="Burning"))
}
pub fn enemy_status_animation_damage(p:&Painter,v:&View,previous:Option<f64>,now:f64)->Option<crate::render::damage::Rect> {
    if previous.map(burning_ink)==Some(burning_ink(now)) || !enemy_burning_visible(v) {return None;}
    let r=p.frame();
    p.logical_damage_rect(r.map.2,r.map.1,r.sidebar-r.map.2,r.rail_bottom-r.map.1)
}
pub fn status_animation_damage(p:&Painter,v:&View,previous:Option<f64>,now:f64)->Option<crate::render::damage::Rect> {
    if !status_burning_visible(v) || previous.map(burning_ink)==Some(burning_ink(now)) { return None; }
    let r=p.hud_tracks().status;
    p.logical_damage_rect(r.0,r.1,r.2,r.3)
}
pub fn paint_resources(p:&mut Painter,v:&View,t:Tracks,now:f64) {
    let r=t.resources;
    if !p.intersects_damage(r.0,r.1,r.2,r.3){return}
    panel(p,r);
    for (i,(name,current,max,color)) in [
        ("HP",v.hp,v.max_hp,game_color(&p.profile,"ui.colors.health")),
        ("NRG",v.energy,v.max_energy,game_color(&p.profile,"ui.colors.energy"))
    ].into_iter().enumerate(){
        let value=resource_ui::value_text(if i==0 {"Health"} else {"Energy"},current,max,&v.resource_changes[i]);
        let (fill,pending)=resource_ui::bar_fractions(current,max,if i==1 {v.pending_energy_loss}else{0.});
        let Rect(x,y,w,h) = t.vital(i);
        resource_ui::paint_bar_pending(p,name,&value,x,y,w,h,color,fill,13,pending);
    }
    let z=t.status;
    if v.player_meters.is_empty() {p.text("No status effects",z.0,z.1+4,11,MUTED);}
    for (i,m) in v.player_meters.iter().skip(v.player_status_offset).take(STATUS_SLOTS).enumerate(){
        let w=(z.2-8*(STATUS_SLOTS as i32-1))/STATUS_SLOTS as i32;
        p.condition_meter_tinted(m,z.0+i as i32*(w+8),z.1,w,status_tint(&m.name,m.duration,now));
    }
    let level = format!("LVL {:.0}",v.level);
    let value = format!("Blessings {:.0} / {:.0}",v.blessings,v.blessing_goal);
    let bar_x=r.0+96;
    p.text(&level,r.0+10,54,12,INK);
    let fraction = if v.blessing_goal > 0. { (v.blessings / v.blessing_goal).clamp(0.,1.) } else { 1. };
    resource_ui::paint_bar(p,"",&value,bar_x,52,r.2-106,19,YELLOW,fraction,11);
}
pub fn paint_depth(p:&mut Painter,v:&View,t:Tracks) {
    let r=t.depth;
    if !p.intersects_damage(r.0,r.1,r.2,r.3){return;}
    panel(p,r);
    p.text("Depth",r.0+10,r.1+13,11,MUTED);
    p.text(&format!("{:.0}/{}",v.display_depth,v.max_depth),r.0+10,r.1+30,16,INK);
}

fn supply_label(count: u32, max: Option<u32>) -> String {
    max.map_or_else(|| count.to_string(), |max| format!("{count} / {max}"))
}
fn paint_supply(p: &mut Painter, b: Rect, id: usize, name: &str, count: u32, max: Option<u32>) {
    let (accent, fill, border) = if id == HEARTS {
        ([255,154,164], [49,26,38], [151,77,94])
    } else {
        ([161,218,250], [23,38,58], [68,118,151])
    };
    let active = p.toolbar_focus == Some(id);
    p.rect(b.0,b.1,b.2,b.3,if active { [59,46,62] } else { fill });
    p.border(b.0,b.1,b.2,b.3,if active { GOLD } else { border });
    let top=b.1+(b.3-42)/2;
    p.text(name,b.0+9,top,13,INK);
    p.text(&supply_label(count,max),b.0+9,top+20,17,accent);
    // Right-facing chevron shows this opens an inventory view.
    let x=b.0+b.2-14;
    let y=b.1+b.3/2;
    let ink=if active { GOLD } else { accent };
    for d in 0..4 {
        p.rect(x+d,y-3+d,2,1,ink);
        p.rect(x+d,y+3-d,2,1,ink);
    }
}
pub fn paint_footer(p:&mut Painter,v:&View,t:Tracks,notice:&str) {
    let r=t.inspector;
    let bottom=t.gods.1+t.gods.3;
    if p.intersects_damage(r.0,r.1,p.frame().map.2,bottom-r.1) {
        let end=t.potions.0+t.potions.2+4;
        let start=t.hearts.0-4;
        panel(p,Rect(start,r.1,end-start,bottom-r.1));
        for b in [r,t.stats,t.rings]{panel(p,b);}
        let role=crate::typography_ui::Role::Primary;
        let text=p.fit_text_role(role,notice,r.2-16,13);
        p.text(&text,r.0+8,p.inspection_notice_y(),13,INK);
        for i in 0..3 {
            let z=t.gods;
            let left=z.0+z.2*i/3;
            let width=z.0+z.2*(i+1)/3-left;
            let rect=Rect(left,z.1,width,z.3);
            panel(p,rect);
            if let Some((name,color))=v.god_labels.get(i as usize) {
                p.rect(left+1,z.1+1,width-2,2,*color);
                let name=p.fit_text_role(role,name,width-16,14);
                p.text(&name,left+9,z.1+(z.3-14)/2,14,*color);
            }
        }
        let z=t.stats;
        let [damage,hp,dr,acc,dodge]=v.player_stats;
        p.text(&format!("DMG {damage:+}  HP {hp:+}  DR {dr}"),z.0+8,z.1+4,11,INK);
        p.text(&format!("ACC {acc}  DODGE {dodge}"),z.0+8,z.1+18,11,MUTED);
        let pack = &v.pack_hud;
        let shards=format!("Phantasmii {}",pack.shards);
        p.text(&shards,z.0+8,z.1+32,10,[194,161,236]);
        let z=t.rings;
        if pack.rings.is_empty() {
            p.text("No rings worn",z.0+8,z.1+8,11,MUTED);
        } else {
            let count=supply_label(pack.rings.len() as u32,pack.ring_capacity);
            p.text(&format!("{count} rings"),z.0+8,z.1+5,11,INK);
            paint_ring_boxes(p,z,&pack.rings);
        }
        paint_supply(p,t.hearts,HEARTS,"Hearts",pack.hearts,pack.heart_capacity);
        paint_supply(p,t.potions,POTIONS,"Potions",pack.potions,pack.potion_capacity);
    }
}
fn paint_ring_boxes(p:&mut Painter,r:Rect,names:&[String]) {
    let mut groups=Vec::<(&str,usize)>::new();
    for name in names {
        let name=name.strip_suffix(" Ring").unwrap_or(name);
        if let Some((_,count))=groups.iter_mut().find(|(key,_)|*key==name){*count+=1}else{groups.push((name,1))}
    }
    let labels=groups.iter().map(|(name,n)|if *n==1{name.to_string()}else{format!("{name} ×{n}")}).collect::<Vec<_>>();
    let role=crate::typography_ui::Role::Primary;
    let max=r.2-14;
    let widths=|p:&Painter,size:u16|labels.iter().map(|s|p.measure_text_role(role,s,size).ceil() as i32+10).collect::<Vec<_>>();
    let mut size=9;
    let mut cuts=widths(p,size);
    let mut split=labels.len().div_ceil(2);
    for n in (9..=11).rev() {
        let w=widths(p,n);
        if let Some(i)=(0..=w.len()).filter(|&i|w[..i].iter().sum::<i32>()<=max&&w[i..].iter().sum::<i32>()<=max)
            .min_by_key(|&i|(w[..i].iter().sum::<i32>()-w[i..].iter().sum::<i32>()).abs()) {
            size=n;cuts=w;split=i;break;
        }
    }
    for (line,range) in [0..split,split..labels.len()].into_iter().enumerate(){
        let y=r.1+21+line as i32*19;
        let mut x=r.0+7;
        let compressed=cuts[range.clone()].iter().sum::<i32>()>max;
        let n=range.len().max(1) as i32;
        for i in range {
            let width=if compressed{max/n}else{cuts[i]};
            p.border(x,y,width-3,16,[115,98,78]);
            let label=p.fit_text_role(role,&labels[i],width-10,size);
            p.text(&label,x+4,y+(16-size as i32)/2,size,[220,199,145]);
            x+=width;
        }
    }
}
pub fn scroll_mark(p:&mut Painter,r:Rect,count:usize,visible:usize,offset:usize) {
    if visible==0 || count<=visible { return; }
    let height=(r.3-12).max(1);
    let thumb=(height*visible as i32/count as i32).clamp(1,height);
    let top=(height-thumb)*offset.min(count-visible) as i32/(count-visible) as i32;
    p.rect(r.0+r.2-6,r.1+6,2,height,[50,44,64]);
    p.rect(r.0+r.2-6,r.1+6+top,2,thumb,[144,112,174]);
}
/// Prefer the regular label size, then shrink only this label. Reserve width
/// for the tier suffix so an unusually long item still shows its upgrade.
pub fn group_name_size(p:&Painter,role:crate::typography_ui::Role,names:&[(String,i32)],max:u16)->u16 {
    (9..=max).rev().find(|&size|names.iter().all(|(n,w)|p.measure_text_role(role,n,size)<=*w as f64)).unwrap_or(9)
}
pub fn fitted_name(p:&Painter,role:crate::typography_ui::Role,name:&str,suffix:&str,width:i32,size:u16)->String {
    let reserved=p.measure_text_role(role,suffix,size).ceil() as i32;
    format!("{}{suffix}",p.fit_text_role(role,name,(width-reserved).max(0),size))
}
pub fn divine_name_width(a:&crate::ability_ui::AbilityView,rail:i32)->i32 {
    let passive=a.mode.eq_ignore_ascii_case("passive");
    let ragelight=matches!(a.indicator,Some(crate::ability_ui::CustomIndicator::RagelightCost(_)));
    let reserve=a.reserve.is_some()&&(!passive||ragelight);
    if !passive||ragelight||a.indicator.is_some()||a.fortunes.is_some() {
        rail-69-if reserve&&a.id=="kative-cogworks"{40}else if reserve&&a.indicator.is_some(){28}else{0}
    }else{rail-17}
}
pub fn right_slot(row:Rect)->Rect {Rect(row.0+row.2-53,row.1+(row.3-13)/2,48,13)}
fn tier_suffix(tier:Option<u8>)->String{
    match tier {Some(n) if n>1=>format!(" +{}",n-1),_=>String::new()}
}
/// O(D+I) bounded measurements for D displayed divine powers and I item actions.
pub fn ability_label_size(p:&Painter,v:&View,t:Tracks)->u16 {
    let role=crate::typography_ui::Role::Primary;
    let mut names=v.abilities.iter().map(|a|(a.name.clone(),divine_name_width(a,t.divine.2))).collect::<Vec<_>>();
    names.extend(v.pack_hud.items.iter().map(|a|(format!("{}{}",a.label,tier_suffix(a.tier)),t.items.2-68)));
    group_name_size(p,role,&names,11)
}
pub fn paint_item_rail(p:&mut Painter,v:&View,t:Tracks,size:u16) {
    let r=t.items;
    if !p.intersects_damage(r.0,r.1,r.2,r.3){return}
    panel(p,r);
    let old=p.clip;
    p.clip=Some((r.0+4,r.1+ABILITY_INSET,r.2-8,r.3-2*ABILITY_INSET));
    let slots=t.item_geometry();
    let offset=p.item_rail_offset.min(v.pack_hud.items.len().saturating_sub(slots.count));
    let role=crate::typography_ui::Role::Primary;
    let width=r.2-68;
    for (i,a) in v.pack_hud.items.iter().skip(offset).take(t.item_rows()).enumerate(){
        let row=slots.row(i);
        let y=row.1;
        let Rect(x,my,w,h)=right_slot(row);
        let name=fitted_name(p,role,&a.label,&tier_suffix(a.tier),width,size);
        p.text(&name,r.0+7,y+(row.3-size as i32)/2,size,INK);
        if let Some((current,max))=a.reserve {
            resource_ui::paint_bar(p,"",&format!("{current:.0}/{max:.0}"),x,my,w,h,game_color(&p.profile,"ui.colors.energy"),current/max.max(1.),9);
        } else {
            let detail=a.arrows.map_or_else(||a.hint.clone().unwrap_or_else(||"Apply".to_owned()), |n|format!("{n} arrows"));
            let dx=x+w-resource_ui::text_width(p,&detail,9);
            p.text(&detail,dx,my+2,9,[147,210,246]);
        }
        p.rect(r.0+7,y+row.3-1,r.2-14,1,[43,37,55]);
    }
    p.clip=old;
    scroll_mark(p,r,v.pack_hud.items.len(),t.item_rows(),offset);
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
    fn mockup_item_rail_actions_use_canonical_commands() {
        use corpse_core::Command;
        let mut r=sample();
        r.dreamwalking=corpse_core::dreamwalking::State{unlocked:true,..Default::default()}.into();
        r.inventory.push(corpse_core::ahvamitt::mask_for_revision(40000.,52.));
        r.inventory.push(corpse_core::borge::shard(40001.));
        let actions=observe(&r).items;
        assert_eq!(actions[0].label,"Ahvamitt Teleport");
        assert!(matches!(activate_quick(&r,actions[0].source),crate::pack::PackAction::Command(Command::Ability{id,..}) if id==corpse_core::ahvamitt::TELEPORT_ID));
        assert!(matches!(activate_quick(&r,actions[1].source),crate::pack::PackAction::Command(Command::Ability{id,..}) if id==corpse_core::dreamwalking::DREAMWALK_ID));
        let shard=actions.iter().find(|a|a.label=="Shard of the Borge").unwrap();
        assert!(matches!(activate_quick(&r,shard.source),crate::pack::PackAction::Target(crate::pack::PackTarget::Throw(40001.))));
    }

    #[test]
    fn hud_font_audit() {
        use crate::typography_ui::Role;
        let mut r=sample();
        let p=Painter::new();
        let t=p.hud_tracks();
        r.player.abilities=corpse_core::progression::catalog_for_revision(52.).iter().map(|a|a.id.clone()).collect();
        corpse_core::revision50_ability_energy::reconcile_acquired(&mut r).unwrap();
        let mut w=crate::ability_ui::divine_abilities(&r).into_iter().map(|a| {
            let width=divine_name_width(&a,t.divine.2);
            (p.measure_text_role(Role::Primary,&a.name,11)/width as f64,a.name)
        }).collect::<Vec<_>>();
        w.sort_by(|a,b|b.0.total_cmp(&a.0)); eprintln!("Divine names past width at size 11: {:?}",w.iter().filter(|a|a.0>1.).collect::<Vec<_>>());
        let item_names=["Teleportation Amulet +8","Obstruction Amulet +8","Lightning Amulet +8","Beckoning Amulet +8","Ahvamitt Teleport","Shard of the Borge",
            "Firebolt Amulet +8","Poison Amulet +8","Iceblast Amulet +8","Blinking Amulet +8","Discord Amulet +8","Roots Amulet +8","Floods Amulet +8"];
        let mut w=item_names.into_iter().map(|a|(p.measure_text_role(Role::Primary,a,11)/(t.items.2-68)as f64,a)).collect::<Vec<_>>();
        w.sort_by(|a,b|b.0.total_cmp(&a.0));eprintln!("Item names past width at size 11: {:?}",w.iter().filter(|a|a.0>1.).collect::<Vec<_>>());
        let frame=p.frame();
        let mut names=corpse_core::population::catalog().actors.values().map(|a|{
            let name=corpse_core::revision50_creature_identity::display_name(&a.name);
            let label=format!("{} {name}",a.glyph);
            let width=frame.sidebar-frame.map.2-16-16-17;
            (p.measure_text_role(Role::Secondary,&label,12)/width as f64,label)
        }).collect::<Vec<_>>();
        names.sort_by(|a,b|b.0.total_cmp(&a.0));eprintln!("Monster names past width at size 12: {:?}",names.iter().filter(|a|a.0>1.).take(15).collect::<Vec<_>>());
    }

    #[test]
    fn mockup_status_colors_preserve_permanents_and_animate_timed_fire() {
        assert_eq!(status_tint("Flying",None,0.),[193,138,255]);
        assert_eq!(status_tint("Hemophilia",Some(8.),0.),[255,84,109]);
        assert_eq!(status_tint("Envenomed 4",Some(8.),0.),[113,240,91]);
        assert_ne!(status_tint("Burning",Some(8.),0.),status_tint("Burning",Some(8.),420.));
        assert_eq!(status_tint("Burning",None,420.),[193,138,255]);
    }

    #[test]
    fn mockup_tracks_and_shortcut_admission() {
        let r=sample();
        let summary=observe(&r);
        assert_eq!((summary.hearts,summary.potions),(2,3));
        assert_eq!(supply_label(summary.hearts,summary.heart_capacity),"2 / 10");
        assert_eq!(supply_label(summary.potions,summary.potion_capacity),"3 / 5");
        let mut boosted = r.clone();
        boosted.player.abilities.extend(["isheilah-tools-of-the-trade".into(),"kative-bottomless-pack".into()]);
        let boosted = observe(&boosted);
        assert_eq!(supply_label(boosted.potions,boosted.potion_capacity),"3 / 15");
        let mut legacy = r.clone();
        legacy.simulation_revision=49.0.into();
        assert_eq!(observe(&legacy).potion_capacity,None);
        assert_eq!(summary.items[0].label,"Bow");
        assert!(matches!(crate::pack::quick_item_action(&r,match summary.items[0].source {QuickSource::Item(id)=>id,_=>unreachable!()}),
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
        assert_eq!(p.frame().map,Rect(0,82,948,522));
        assert_eq!(p.frame().sidebar,1114);
        let (cw,ch,x,y)=p.map_layout(79,29);
        assert_eq!((cw,ch),MAP_TILE);
        assert_eq!(x,p.frame().map.0);
        assert_eq!(t.resources.0,700);
        assert_eq!(t.nav.0+t.nav.2,t.depth.0);
        assert_eq!(t.depth.0+t.depth.2,t.resources.0);
        assert!(t.button(2).1+t.button(2).3<t.button(1).1);
        assert!(t.button(1).0<t.button(0).0);
        assert_eq!(t.gods.1,t.inspector.1+t.inspector.3);
        assert_eq!(t.gods.0+t.gods.2,t.rings.0);
        assert_eq!(p.ability_capacity(),DIVINE_SLOTS);
        assert_eq!(t.item_rows(),ITEM_SLOTS);
        assert_eq!(tier_suffix(Some(9))," +8");
        assert_eq!(tier_suffix(Some(1)),"");
        assert_eq!(t.rings.1,t.stats.1+t.stats.3);
        assert!(t.nav.2<t.resources.2);
        assert!(NAV.iter().all(|&(id,_)|id!=4));
        assert_eq!(Rect(x,y,79*cw,29*ch),p.frame().map);
        for (slot,(id,_)) in NAV.into_iter().enumerate(){
            let Rect(x,y,w,h)=t.button(slot);
            assert_eq!(p.toolbar_action(x+w/2,y+h/2),Some(id));
        }
        assert_eq!(p.toolbar_action(t.hearts.0+4,t.hearts.1+4),Some(HEARTS));
        assert_eq!(p.toolbar_action(t.potions.0+4,t.potions.1+4),Some(POTIONS));
        assert_eq!(t.item_at(t.items.0+8,t.items.1+ABILITY_INSET+2,0,summary.items.len()),Some(0));
    }

    #[test]
    #[ignore = "Optional staged preview fixture; uses a real generated floor and engine save"]
    fn mockup_capture_fixture() {
        let Some(dir)=std::env::var_os("CORPSE_MOCKUP_CAPTURE") else{return};
        let dir=std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut r=sample();
        r.dev=false;
        for line in &mut r.log {
            if line=="Creature and loot tuning is provisional and documented." {
                *line="You enter the dungeon.".into();
            }
        }
        r.level=4.;
        r.exp=2.;
        // A staged late-build inventory on a compact generated floor. Do not
        // export this as a playable starting save or a scored run.
        for ability in ["isheilah-tools-of-the-trade","kative-bottomless-pack","kative-appropriate",
            "isheilah-make-slippery","isheilah-quick-hand","isheilah-are-you-sure-about-that",
            "isheilah-spellslinger","numo-implant-thought","numo-distract","numo-mind-read",
            "numo-mind-slave","numo-remember","anavani-shine","anavani-illuminate","anavani-commandment"] {
            r.player.abilities.push(ability.into());
        }
        r.player.abilities.drain(..3);
        assert_eq!(r.player.abilities.len(),15);
        corpse_core::revision50_ability_energy::reconcile_acquired(&mut r).unwrap();
        r.dreamwalking=corpse_core::dreamwalking::State{unlocked:true,..Default::default()}.into();
        for effect in ["light","flight","stealth","regeneration","vampirism","reaping",
            "willpower","protection","slaying","swimming","light","stealth","willpower"] {
            let mut ring=Item{id:r.next_id,kind:"ring".into(),glyph:"=".into(),effect:effect.to_owned().into(),..Default::default()};
            ring.name=corpse_core::revision50_rings::display_name(&ring).unwrap().into();
            r.next_id+=1.;r.inventory.push(ring);
        }
        // The sample already holds a Firebolt Amulet.
        for effect in ["lightning","poison","iceblast","blinking","beckoning","discord","obstruction","roots","teleportation"] {
            let mut item=Item{id:r.next_id,kind:"amulet".into(),glyph:"\"".into(),effect:effect.to_owned().into(),..Default::default()};
            item.name=corpse_core::revision50_amulet_effects::display_name(&item).unwrap().into();
            if effect=="iceblast"{item.amulet_tier=9.0.into();}
            corpse_core::revision50_amulets::initialize_item_for_revision(&mut item,52.).unwrap();
            r.next_id+=1.;r.inventory.push(item);
        }
        let id=r.next_id;r.next_id+=1.;
        r.inventory.push(corpse_core::phantasmii_generation::floor_shard(id));
        let id=r.next_id;r.next_id+=1.;
        r.inventory.push(corpse_core::borge::shard(id));
        let id=r.next_id;r.next_id+=1.;
        r.inventory.push(corpse_core::ahvamitt::mask_for_revision(id,52.));
        for line in ["You hear wings in the dark.","Something stirs beyond the stairs.","You grip your bow."] {
            r.log.push(line.into());
            if let Some(kinds)=r.log_kinds.as_mut(){kinds.push("player".into());}
        }
        assert!(corpse_core::revision50_inventory::finite_categories_fit_revision(&r.inventory,&r.player.abilities,52.));
        r.player.hp=82.;r.player.energy=78.;
        for (condition, turns) in [("burning",8.),("hemophilia",15.),("envenomed",12.),("venom:1",4.)] {
            r.player.conditions.insert(condition.into(),turns);
        }
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
        for (n,actor) in r.actors_at_mut(d).iter_mut().take(4).enumerate() {
            if let Some((x,y))=candidates.nth(1) {
                actor.x=x as f64;actor.y=y as f64;
                actor.heartless=(n!=0).into();
                if n==1 {
                    actor.hp=(actor.max_hp*0.55).ceil();
                    actor.conditions.insert("burning".into(),7.);
                    actor.conditions.insert("envenomed".into(),9.);
                    actor.conditions.insert("venom:1".into(),3.);
                }
                actor.state="sleeping".into();
            }
        }
        // Stage jewelry close to the vessel so the floor rail includes it.
        let occupied: std::collections::HashSet<_> = r.actors_at(d).iter()
            .map(|a|(a.x as usize,a.y as usize)).collect();
        let mut loot = open.iter().copied().filter(|at|!occupied.contains(at));
        let ring = Item {id:r.next_id,kind:"ring".into(),name:"Ring of Flight".into(),
            glyph:"=".into(),effect:"flight".to_owned().into(),..Default::default()};
        r.next_id+=1.;
        let amulet = corpse_core::revision50_amulet_effects::make_amulet_for_revision(
            corpse_core::revision50_amulet_effects::Effect::Lightning,r.next_id,52.).unwrap();
        r.next_id+=1.;
        for (i,item) in [ring,amulet].into_iter().enumerate() {
            let (x,y)=loot.next().unwrap();
            r.ground_at_mut(d).push(corpse_core::GroundItem{x:x as f64,y:y as f64,item,..Default::default()});
            if i==0 { std::fs::write(dir.join("jewelry-location.json"),format!("[{x},{y}]")).unwrap(); }
        }
        let f=r.floor_mut().unwrap();f.seen.fill(true);f.lights.fill(65.);
        let engine=corpse_core::temporal::Engine::new(r).unwrap();
        let save=crate::session_store::encode(&engine,crate::session_store::Kind::Genuine).unwrap();
        std::fs::write(dir.join("showcase-save.json"),save).unwrap();
        let run=&engine.run;
        let view=crate::observe::observe(run);
        let mut meter_model=crate::statuses_ui::ConditionMeters::default();
        let meters=meter_model.measure(run.player.id,&crate::statuses_ui::observe(run,&run.player));
        for name in ["Flying","Hemophilia","Burning","Envenomed"] {
            assert!(meters.iter().take(STATUS_SLOTS).any(|m|m.name.starts_with(name)),"missing preview status {name}: {meters:?}");
        }
        assert_eq!(view.total_abilities,15);
        assert_eq!(view.pack_hud.items[0].label,"Ahvamitt Teleport");
        assert_eq!(view.pack_hud.items[1].label,"Dreamwalk");
        assert_eq!(view.pack_hud.rings.len(),13);
        assert_eq!(view.pack_hud.items.len(),14);
        assert!(view.creatures.iter().any(|a|a.statuses.iter().any(|s|s.name=="Burning")&&a.statuses.iter().any(|s|s.name.starts_with("Envenomed"))));
        assert!(view.pack_hud.items.iter().any(|a|a.label=="Shard of the Borge"));
        assert!(view.creatures.iter().filter(|a|a.has_heart==Some(false)).count()>=2,
            "the walkthrough must show multiple visible heartless enemies");
        assert!(view.creatures.iter().any(|a|a.has_heart==Some(true)));
        assert!(view.ground_entities.iter().any(|row|row.name=="Flight Ring"));
        assert!(view.ground_entities.iter().any(|row|row.name=="Lightning Amulet"));
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
