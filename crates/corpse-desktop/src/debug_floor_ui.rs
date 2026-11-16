//! Debug-only stash/loadout screen; normal revision-specific Pack limits apply.
use corpse_core::{Run, debug_floor as debug};
use sdl2::keyboard::Keycode;
use crate::render::Painter;
const ROWS: usize = 13;
const INK: [u8;3] = [235,233,218];
const MUTED: [u8;3] = [164,169,178];
const GOLD: [u8;3] = [255,209,72];
fn carrying_summary(r: &Run) -> String {
    if corpse_core::revision50::enabled(r) {
        let c = corpse_core::revision50_inventory::capacity(r);
        // f64::sum starts from -0.0; empty category counters must display +0.
        let count = |kind: &str| r.inventory.iter().filter(|i|i.kind == kind)
            .map(corpse_core::inventory::quantity).fold(0.0, |total, units| total + units);
        format!("Rings {:.0}/{:.0} · Amulets {:.0}/{:.0} · Potions {:.0}/{:.0} · Hearts {:.0}/{:.0} · Trinkets unlimited",
            count("ring"),c.rings,count("amulet"),c.amulets,count("potion"),c.potions,count("heart"),c.hearts)
    } else { format!("Pack {:.0}/20",corpse_core::inventory::occupancy(r)) }
}
/// Canonical instance name and reserve, never a target-floor preview of old loot.
pub(crate) fn item_row(r: &Run, item: &corpse_core::Item, depth: Option<u32>) -> String {
    let source=depth.map(|d|if d==0{"starter".into()}else{format!("F{:.0}",corpse_core::fast_mode::display_depth(r,d as f64))}).unwrap_or_else(|| {
        if corpse_core::revision50::enabled(r) {
            if r.simulation_revision.value_or(29.) != 52. && matches!(item.kind.as_str(),"ring"|"amulet") { "active".into() } else { "carry".into() }
        } else { item.equipped.as_ref().map_or("carry".into(),Clone::clone) }
    });
    let reserve=if r.simulation_revision.value_or(29.)==52. {
        corpse_core::revision50_amulet_effects::item_spec_for_revision(item,r.simulation_revision.value_or(29.)).map(|spec|
            format!(" · {}/{} energy",item.amulet_state.as_ref().map_or(spec.capacity,|s|s.energy.min(spec.capacity)),spec.capacity)).unwrap_or_default()
    } else {String::new()};
    format!("{}{}{reserve} · {source}",corpse_core::revision50_item_knowledge::display_name(r,item),
        if corpse_core::inventory::quantity(item)>1.{format!(" ×{}",corpse_core::inventory::quantity(item))}else{String::new()})
}
#[derive(Default)]
pub struct View { pub open: bool, pub pack: bool, pub selected: usize, pub notice: String }
impl View {
    pub fn active(&self, r: &Run) -> bool { debug::choices_complete(r) && (self.open || debug::preparing(r)) }
    fn len(&self, r: &Run) -> usize { if self.pack { r.inventory.len() } else { debug::state(r).map_or(0, |s|s.stash.len()) } }
    fn id(&self, r: &Run) -> Option<f64> { if self.pack { r.inventory.get(self.selected).map(|i|i.id) } else { debug::state(r)?.stash.get(self.selected).map(|v|v.item.id) } }
    fn action(&mut self, r: &mut Run, equip: bool) {
        let Some(id) = self.id(r) else { return; };
        let result = if equip { debug::equip(r,id) } else if self.pack { debug::put(r,id) } else { debug::take(r,id) };
        self.notice = match result { Ok(true) => if equip { if r.simulation_revision.value_or(29.)==52. { "Jewelry carried.".into() } else if corpse_core::revision50::enabled(r) { "Carried jewelry is automatically active.".into() } else { "Equipped using normal slot rules.".into() } } else { "Moved; nothing discarded.".into() }, Ok(false) => "No item moved.".into(), Err(e) => e };
        self.selected = self.selected.min(self.len(r).saturating_sub(1));
    }
    fn close(&mut self, r: &mut Run) { match debug::enter(r) { Ok(()) => {self.open=false;self.notice.clear();}, Err(e) => self.notice=e } }
    pub fn key(&mut self, r: &mut Run, key: Keycode) {
        match key {
            Keycode::Tab | Keycode::Left | Keycode::Right => { self.pack=!self.pack; self.selected=0; },
            Keycode::Up => self.selected=self.selected.saturating_sub(1),
            Keycode::Down => self.selected=(self.selected+1).min(self.len(r).saturating_sub(1)),
            Keycode::PageDown => self.selected=(self.selected+ROWS).min(self.len(r).saturating_sub(1)),
            Keycode::PageUp => self.selected=self.selected.saturating_sub(ROWS),
            Keycode::Return | Keycode::T => self.action(r,false),
            Keycode::E => self.action(r,true),
            Keycode::Space | Keycode::Escape | Keycode::F4 => self.close(r),
            _ => {}
        }
    }
    pub fn scroll(&mut self, r: &Run, y: i32) { self.selected=(self.selected as i32-y).clamp(0,self.len(r).saturating_sub(1) as i32) as usize; }
    pub fn click(&mut self,r: &mut Run,x:i32,y:i32) {
        if (230..659).contains(&y) {
            let pane=if (105..630).contains(&x) {Some(false)} else if (650..1170).contains(&x) {Some(true)} else {None};
            if let Some(pack)=pane { let first=if self.pack==pack {self.selected/ROWS*ROWS}else{0}; self.pack=pack;self.selected=(first+(y-230) as usize/33).min(self.len(r).saturating_sub(1)); }
        } else if (704..748).contains(&y) {
            if (110..430).contains(&x) {self.action(r,false);} else if (450..710).contains(&x) {self.action(r,true);} else if (730..1168).contains(&x) {self.close(r);}
        }
    }
    pub fn paint(&self,r:&Run,p:&mut Painter) {
        let Some(s)=debug::state(r) else{return;};
        p.rect(80,55,1120,710,[10,12,18]);p.border(80,55,1120,710,[100,89,51]);
        p.text("DEBUG LOADOUT / RECOVERED LOOT",108,75,26,GOLD);
        let pack = carrying_summary(r);
        if corpse_core::revision50::enabled(r) {
            p.text(&format!("Floor {}   Level {}   HP {}   Energy {}",corpse_core::fast_mode::display_depth(r,r.depth),r.level,r.player.max_hp,corpse_core::combat::effective_stats(r,&r.player).max_energy),108,110,17,INK);
            p.text(&pack,108,133,14,INK);
        } else {
            p.text(&format!("Floor {}   Level {}   HP {}   Energy {}   {pack}",corpse_core::fast_mode::display_depth(r,r.depth),r.level,r.player.max_hp,corpse_core::combat::effective_stats(r,&r.player).max_energy),108,114,18,INK);
        }
        p.text(&format!("Exact seed: {}",r.seed),108,if corpse_core::revision50::enabled(r) {154} else {145},15,MUTED);
        let hint = if r.simulation_revision.value_or(29.)==52. { "Enter: carry or return the selected item. Overflow stays in the stash." }
            else if corpse_core::revision50::enabled(r) { "Carried rings and amulets are automatically active. Overflow stays in the stash." }
            else { "Nothing auto-equipped. All overflow stays here. No invented gear or combat consumption." };
        p.text(hint,108,if corpse_core::revision50::enabled(r) {177} else {172},15,MUTED);
        p.text(&format!("Recovered stash ({} item entries)",s.stash.len()),110,204,18,if !self.pack{GOLD}else{INK});
        p.text(if corpse_core::revision50::enabled(r) { "Carried items (normal Pack)" } else { "Carry / equipped (normal Pack)" },654,204,18,if self.pack{GOLD}else{INK});
        for pack in [false,true] {
            let first=if pack==self.pack {self.selected/ROWS*ROWS}else{0};
            let items:Vec<_>=if pack {r.inventory.iter().map(|i|(i,None)).collect()}else{s.stash.iter().map(|v|(&v.item,Some(v.depth))).collect()};
            for (row,(item,depth)) in items.iter().skip(first).take(ROWS).enumerate() {
                let (x,y)=(if pack{650}else{105},230+row as i32*33);
                if pack==self.pack&&first+row==self.selected {p.rect(x,y,520,31,[37,40,46]);}
                let text=item_row(r,item,*depth);
                p.text(&text.chars().take(52).collect::<String>(),x+7,y+7,15,INK);
            }
        }
        p.text(&self.notice.chars().take(118).collect::<String>(),110,666,14,GOLD);
        for (x,w,text) in [(110,320,"Enter: carry / return"),(450,260,if corpse_core::revision50::enabled(r) { "E: carry jewelry" } else { "E: equip selected" }),(730,438,"Space: enter target floor")] {p.border(x,704,w,44,[90,90,85]);p.text(text,x+12,716,17,INK);}
        p.text("Tab: switch list   Arrows / PgUp / PgDn: select   F4: reopen stash during DEBUG",110,749,12,MUTED);
    }
}
pub fn choice_banner(r:&Run,p:&mut Painter) {
    if let Some(s)=debug::state(r).filter(|_|debug::preparing(r)) {
        p.text(&format!("DEBUG RECONSTRUCTION · LEVEL {} · {}/{} PRIOR HEARTS",r.level,s.consumed,r.playtest.as_ref().unwrap().prior_hearts),345,20,17,GOLD);
        p.text(&format!("Target floor {} · {}",corpse_core::fast_mode::display_depth(r,r.depth),r.seed),345,46,13,INK);
        p.text("Real level-up offer; your choice affects later rewards. No target-floor rewards included.",300,749,14,MUTED);
    }
}
pub fn god_names(seed:&str)->String {
    let Ok(r)=corpse_core::startup::new_run_shell(seed,"Avatar",false) else{return "Use three distinct god IDs 0–11 before : in the seed.".into();};
    let names:Vec<_>=r.gods.iter().map(|id|corpse_core::progression::catalog().iter().find(|a|a.god_id==*id).map(|a|a.id.split('-').next().unwrap_or("?").to_string()).unwrap_or_default()).collect();
    format!("Gods: {} / {} / {}  [F3 / F4 / F8 to cycle]",names[0],names[1],names[2])
}
pub fn cycle_god(seed:&mut String,slot:usize) {
    let Ok(r)=corpse_core::startup::new_run_shell(seed,"Avatar",false)else{return;};
    let mut gods:Vec<_>=r.gods.iter().map(|v|*v as u32).collect();
    for _ in 0..12 {gods[slot]=(gods[slot]+1)%12;if gods.iter().enumerate().all(|(i,v)|i==slot||*v!=gods[slot]){break;}}
    let suffix=r.seed.split_once(':').unwrap().1;
    *seed=format!("{}-{}-{}:{suffix}",gods[0],gods[1],gods[2]);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn debug_god_picker_keeps_distinct_ids_and_exact_seed_suffix() {
        let mut seed="0-5-11:picker".to_string();
        for slot in 0..3 { for _ in 0..14 { cycle_god(&mut seed,slot);let r=corpse_core::startup::new_run_shell(&seed,"Avatar",false).unwrap();assert_eq!(r.gods.len(),3);assert_ne!(r.gods[0],r.gods[1]);assert_ne!(r.gods[1],r.gods[2]);assert_ne!(r.gods[0],r.gods[2]);assert!(seed.ends_with(":picker")); } }
    }
    #[test]
    fn no_choice_starter_loadout_can_enter_and_reopen_without_healing() {
        let mut r=debug::create_run("0-5-11:debug-floor-regression","Avatar",1).unwrap();let mut v=View::default();assert!(v.active(&r));
        v.key(&mut r,Keycode::Space);assert!(!v.active(&r));assert_eq!(r.inventory.len(),3);
        r.player.hp=10.;v.open=true;assert!(v.active(&r));v.key(&mut r,Keycode::Escape);assert!(!v.active(&r));assert_eq!(r.player.hp,10.);
    }
    #[test]
    fn r50_debug_empty_category_counts_display_positive_zero() {
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=50.0.into();r.inventory.clear();r.player.abilities.clear();
        let before=r.clone();
        assert_eq!(carrying_summary(&r),"Rings 0/5 · Amulets 0/5 · Potions 0/5 · Hearts 0/5 · Trinkets unlimited");
        assert_eq!(r,before);
        r.player.abilities=vec!["isheilah-tools-of-the-trade".into(),"kative-bottomless-pack".into(),"kative-appropriate".into()];
        assert_eq!(carrying_summary(&r),"Rings 0/13 · Amulets 0/10 · Potions 0/15 · Hearts 0/5 · Trinkets unlimited");
    }
    #[test]
    fn r50_debug_e_carries_active_jewelry_without_capacity_bypass() {
        use corpse_core::{Field, Item};
        let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
        r.simulation_revision=50.0.into();r.inventory.clear();r.player.abilities.clear();r.player.active.clear();
        r.player.conditions.clear();r.player.ability_order=Field::Missing;r.player.equipment_abilities=Field::Missing;
        r.player.ability_reserves=Field::Missing;r.offers.clear();
        let stash:Vec<_>=(0..6).map(|n|debug::Reward{depth:1,item:Item{id:900.+n as f64,kind:"ring".into(),name:"Ring of Light".into(),effect:"light".to_string().into(),..Default::default()}}).collect();
        r.playtest=corpse_core::playtest::Profile{version:2,start_floor:6,profile:debug::PROFILE.into(),prior_hearts:0,prior_blessings:0.,
            sources:stash.iter().map(|s|corpse_core::playtest::EquipmentSource{depth:s.depth,item_id:s.item.id}).collect(),candidates:vec![],
            debug:Some(debug::State{pending:Default::default(),consumed:0,stash,entered:false,loadout_revision:0})}.into();
        let mut v=View::default();
        for _ in 0..5 {v.key(&mut r,Keycode::E);assert_eq!(v.notice,"Carried jewelry is automatically active.");}
        assert!(r.inventory.iter().all(|i|i.equipped.is_missing()));
        assert_eq!(corpse_core::revision50_rings::count_effect(&r,&r.player,"light"),5);
        let before=r.clone();v.key(&mut r,Keycode::E);assert_eq!(r,before);assert!(v.notice.contains("category is full"));
        assert_eq!(carrying_summary(&r),"Rings 5/5 · Amulets 0/5 · Potions 0/5 · Hearts 0/5 · Trinkets unlimited");
        v.key(&mut r,Keycode::Tab);v.key(&mut r,Keycode::E);assert_eq!(r,before);
        v.key(&mut r,Keycode::Return);assert_eq!(r.inventory.len(),4);
        assert_eq!(corpse_core::revision50_rings::count_effect(&r,&r.player,"light"),4);
    }

}
