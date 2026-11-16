//! Avatar-only new-game choices. This state is presentation, never a saved Run.
use crate::{ability_ui::god_color, render::{Painter, Rgb}};
use sdl2::keyboard::{Keycode, Mod};

pub const MODE: &str = "Avatar";
pub const GOD_START: usize = 8;
const INK: Rgb = [15, 10, 18];
const PAPER: Rgb = [238, 229, 207];

pub struct God {
    pub name: &'static str,
    pub alias: &'static str,
    pub domain: &'static str,
}
pub const GODS: [God; 12] = [
    God { name: "SHRAIL", alias: "the Godthorn", domain: "God of Fury" },
    God { name: "PELIAN", alias: "the King of Desire", domain: "God of Zeal" },
    God { name: "ISHEILAH", alias: "the Trickster", domain: "God of Guile" },
    God { name: "NUMO", alias: "the Pregenitor", domain: "Goddess of Memory" },
    God { name: "ETILL", alias: "the Ocean-Black", domain: "Goddess of Flow" },
    God { name: "URSA", alias: "the Sleeping Mother", domain: "Goddess of Dreams" },
    God { name: "LICHOR", alias: "the Gushing One", domain: "God of Blood" },
    God { name: "DAWEL", alias: "the Mouthless", domain: "Goddess of Silence" },
    God { name: "GRISHA", alias: "the Devourer", domain: "God of Ruin" },
    God { name: "ANAVANI", alias: "of the Moon", domain: "Goddess of Ascendance" },
    God { name: "KATIVE", alias: "the Craftsman", domain: "God of Artifice" },
    God { name: "MORAE", alias: "the Sacrifice", domain: "Goddess of Fate" },
];

pub type Rect = (i32, i32, i32, i32);
pub fn contains((x,y,w,h): Rect, px:i32, py:i32)->bool { px>=x && px<x+w && py>=y && py<y+h }
pub fn slot_rect(slot:usize)->Rect { (710, 491 + slot as i32 * 91, 470, 80) }
pub fn card_rect(id:usize)->Rect { (74 + (id%3) as i32*384, 166 + (id/3) as i32*116, 364, 102) }
pub const CANCEL: Rect = (940, 681, 266, 54);
pub fn slot_at(x:i32,y:i32)->Option<usize> { (0..3).find(|i|contains(slot_rect(*i),x,y)) }
pub fn card_at(x:i32,y:i32)->Option<usize> { (0..12).find(|i|contains(card_rect(*i),x,y)) }

#[derive(Clone, Debug, PartialEq)]
pub struct Picker { pub slot:usize, pub selected:usize, pub notice:String }
#[derive(Clone, Debug, PartialEq)]
pub struct Selection { pub gods:[usize;3], pub picker:Option<Picker> }
impl Default for Selection {
    fn default()->Self { Self { gods:[0,1,2], picker:None } }
}

/// O(S) bounded seed scan. Match the canonical parser's numeric-prefix grammar;
/// a free-form seed remains a suffix rather than silently selecting new gods.
pub(crate) fn numeric_prefix(seed:&str)->bool {
    seed.split_once(':').is_some_and(|(prefix,_)| {
        let parts:Vec<_>=prefix.split('-').collect();
        parts.len()==3 && parts.iter().all(|p| !p.is_empty() && p.len()<=2 && p.bytes().all(|b|b.is_ascii_digit()))
    })
}
fn validate_seed(seed:&str)->Result<(),String> {
    if seed.len()>180 || seed.chars().any(char::is_control) {
        return Err("Seed must fit 180 bytes and contain no control characters.".into());
    }
    Ok(())
}
impl Selection {
    /// O(K + S), fixed K=12 partial shuffle and bounded seed copy. Presentation
    /// entropy is supplied by the caller, never taken from a live Run's RNG.
    pub fn randomize(&mut self, seed:&mut String, values:[u32;3]) -> Result<(),String> {
        let mut available:[usize;12]=std::array::from_fn(|i|i);
        for (slot,value) in values.iter().enumerate() {
            available.swap(slot,slot+*value as usize%(GODS.len()-slot));
        }
        self.gods=available[..3].try_into().unwrap();
        self.close();
        let trimmed=seed.trim();
        if numeric_prefix(trimmed) {
            // Preserve the entire suffix, including internal colons/UTF-8. A
            // longer padded prefix can exceed the input limit; never truncate it.
            *seed=format!("{}:{}",self.prefix(),trimmed.split_once(':').unwrap().1);
        }
        validate_seed(seed)
    }
    /// O(S + A) on seed edits, including the bounded starter catalog A in
    /// the canonical shell parser; invalid numeric prefixes preserve choices.
    pub fn sync_seed(&mut self,seed:&str) {
        let seed=seed.trim();
        if numeric_prefix(seed) {
            if let Ok(run)=corpse_core::startup::new_run_shell(seed,MODE,false) {
                self.gods=std::array::from_fn(|i|run.gods[i] as usize);
            }
        }
    }
    fn prefix(&self)->String { format!("{:02}-{:02}-{:02}",self.gods[0],self.gods[1],self.gods[2]) }
    /// O(S + A) including the bounded shell parser; one OS random seed only
    /// for a blank suffix. Full valid seeds retain
    /// every byte and their authored god order; the picker never draws randomness.
    pub fn prepared_seed(&self,seed:&str,random:impl FnOnce()->String)->Result<String,String> {
        validate_seed(seed)?;
        if self.gods.iter().any(|id|*id>=GODS.len()) || self.gods[0]==self.gods[1] || self.gods[0]==self.gods[2] || self.gods[1]==self.gods[2] {
            return Err("Choose three distinct gods.".into());
        }
        let seed=seed.trim();
        let result=if numeric_prefix(seed) {
            corpse_core::startup::new_run_shell(seed,MODE,false)?;
            seed.to_string()
        } else if seed.is_empty() {
            let fresh=random();
            let suffix=fresh.split_once(':').map_or(fresh.as_str(),|(_,suffix)|suffix);
            format!("{}:{suffix}",self.prefix())
        } else { format!("{}:{seed}",self.prefix()) };
        validate_seed(&result)?;
        Ok(result)
    }
    pub fn open(&mut self,slot:usize) {
        if slot<3 { self.picker=Some(Picker {slot,selected:self.gods[slot],notice:String::new()}); }
    }
    pub fn close(&mut self) { self.picker=None; }
    pub fn active(&self)->bool { self.picker.is_some() }
    /// O(S + 3), transactional: a used god or oversized seed changes neither draft.
    pub fn choose(&mut self,id:usize,seed:&mut String)->bool {
        let Some(picker)=self.picker.as_mut() else {return false;};
        if id>=GODS.len() {return false;}
        if let Some(other)=self.gods.iter().enumerate().find(|(slot,god)|*slot!=picker.slot && **god==id).map(|(i,_)|i) {
            picker.notice=format!("{} is already chosen in slot {}.",GODS[id].name,other+1);
            return false;
        }
        let slot=picker.slot;
        if self.gods[slot]==id {self.close();return true;}
        let mut next=self.clone();
        next.gods[slot]=id;
        let trimmed=seed.trim();
        let updated=if trimmed.is_empty() {String::new()} else {
            let suffix=if numeric_prefix(trimmed) {trimmed.split_once(':').unwrap().1} else {trimmed};
            format!("{}:{suffix}",next.prefix())
        };
        if let Err(error)=validate_seed(&updated) {self.picker.as_mut().unwrap().notice=error;return false;}
        next.picker=None;
        *self=next; *seed=updated;
        true
    }
    pub fn key(&mut self,key:Keycode,mods:Mod,seed:&mut String) {
        let Some(picker)=self.picker.as_mut() else {return;};
        let next=match key {
            Keycode::Escape=>{self.close();return;},
            Keycode::Return|Keycode::KpEnter|Keycode::Space=>{
                if picker.selected==12 {self.close();} else {let id=picker.selected;self.choose(id,seed);} return;
            },
            Keycode::Tab=>Some((picker.selected+if mods.intersects(Mod::LSHIFTMOD|Mod::RSHIFTMOD){12}else{1})%13),
            Keycode::Left=>Some(if picker.selected==12{11}else{(picker.selected+11)%12}),
            Keycode::Right=>Some(if picker.selected==12{0}else{(picker.selected+1)%12}),
            Keycode::Up=>Some(if picker.selected==12{9}else{(picker.selected+9)%12}),
            Keycode::Down=>Some(if picker.selected==12{0}else{(picker.selected+3)%12}),
            Keycode::Home=>Some(0),Keycode::End=>Some(11),_=>None,
        };
        if let Some(next)=next {picker.selected=next;picker.notice.clear();}
    }
    pub fn click(&mut self,x:i32,y:i32,seed:&mut String) {
        if contains(CANCEL,x,y) {self.close();} else if let Some(id)=card_at(x,y) {self.choose(id,seed);}
    }
    pub fn hover(&mut self,x:i32,y:i32)->bool {
        let Some(picker)=self.picker.as_mut() else{return false;};
        let selected=card_at(x,y).or_else(||contains(CANCEL,x,y).then_some(12));
        if let Some(selected)=selected {if picker.selected!=selected {picker.selected=selected;return true;}}
        false
    }
    pub fn paint_slots(&self,p:&mut Painter,selected:usize) {
        p.text("YOUR AVATAR'S GODS",710,451,21,PAPER);
        for (slot,id) in self.gods.iter().enumerate() {
            let (x,y,w,h)=slot_rect(slot);
            let god=&GODS[*id]; let color=god_color(*id);
            p.rect(x,y,w,h,color);
            if selected==GOD_START+slot {p.border(x-3,y-3,w+6,h+6,PAPER);}
            p.text(&format!("{}  {}",slot+1,god.name),x+14,y+9,23,INK);
            p.text(god.alias,x+14,y+35,17,INK);
            p.text(god.domain,x+14,y+58,13,INK);
            p.text("CHANGE >",x+w-101,y+34,14,INK);
        }
    }
    /// O(12 + G + P): fixed catalog, glyph raster/cache costs and painted pixels.
    /// No Run/visibility queries, random calls, file IO or persisted UI state.
    pub fn paint_picker(&self,p:&mut Painter) {
        let Some(picker)=&self.picker else{return;};
        p.clear([12,8,14]);
        p.text(&format!("CHOOSE GOD {}",picker.slot+1),74,56,36,PAPER);
        for (id,god) in GODS.iter().enumerate() {
            let (x,y,w,h)=card_rect(id); let color=god_color(id);
            p.rect(x,y,w,h,color);
            if picker.selected==id {p.border(x-4,y-4,w+8,h+8,PAPER);p.border(x-6,y-6,w+12,h+12,PAPER);}
            p.text(god.name,x+14,y+8,24,INK);
            p.text(god.alias,x+14,y+37,18,INK);
            p.text(god.domain,x+14,y+62,15,INK);
            if let Some(slot)=self.gods.iter().position(|g|*g==id) {
                p.text(if slot==picker.slot {"CURRENT CHOICE"} else {"CHOSEN IN ANOTHER SLOT"},x+14,y+84,10,INK);
            }
        }
        p.text(&picker.notice,74,643,14,[255,173,153]);
        p.text("Arrows / Tab: move    Enter: choose",74,690,17,PAPER);
        p.text("Esc: cancel.",74,720,14,[197,183,176]);
        let(x,y,w,h)=CANCEL;
        p.rect(x,y,w,h,if picker.selected==12 {PAPER}else{[45,32,40]});
        p.border(x,y,w,h,PAPER);
        p.text("CANCEL",x+80,y+16,21,if picker.selected==12 {INK}else{PAPER});
    }
}
