//! Fixed logical title-page composition. The renderer and input use identical bounds.
use crate::render::{Painter, Rgb};

pub struct Item {
    pub label: &'static str,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub size: u16,
}
pub const FAST_INDEX: usize = 11;
pub const FAST_RECT: (i32, i32, i32, i32) = (103, 8, 559, 44);
pub const FAST_DEBUG_NOTICE: &str = "Choose a floor from 1 to 20.";
/// O(input bytes), input is always the sequential player-facing floor number.
pub fn parse_start_floor(input: &str, _fast: bool) -> Result<u32, String> {
    let floor = input.trim().parse::<u32>().map_err(|_| FAST_DEBUG_NOTICE.to_string())?;
    if !(1..=20).contains(&floor) { return Err(FAST_DEBUG_NOTICE.into()); }
    Ok(floor)
}
/// O(1), keyboard steps through all twenty physical floors.
pub fn step_start_floor(input: &str, forward: bool, _fast: bool) -> String {
    let n = input.trim().parse::<i32>().unwrap_or(1);
    (n + if forward { 1 } else { -1 }).clamp(1, 20).to_string()
}
pub fn fast_at(x: i32, y: i32) -> bool {
    let (a, b, w, h) = FAST_RECT;
    x >= a && x < a + w && y >= b && y < b + h
}
pub fn fast_label(enabled: bool) -> &'static str {
    if enabled { "Fast mode: ON" } else { "Fast mode: OFF" }
}
pub const SEED_INDEX: usize = 7;
pub const SEED_RECT:(i32,i32,i32,i32)=(490,443,172,66);
/// O(1), shared logical input bounds; separate from existing button/art layout.
pub fn seed_at(x:i32,y:i32)->bool{let(a,b,w,h)=SEED_RECT;x>=a&&x<a+w&&y>=b&&y<b+h}
/// O(text bytes), bounded UTF-8 edit; validation of seed syntax remains in startup.
pub fn edit_seed(seed:&mut String,text:&str,replace:bool)->bool{if text.chars().any(char::is_control)||text.len()+(if replace{0}else{seed.len()})>180{return false;}if replace{seed.clear();}seed.push_str(text);true}
/// O(seed bytes), unchanged trimming/blank-random semantics, one random call only for blank.
pub fn selected_seed(seed:&str,random:impl FnOnce()->String)->String{if seed.trim().is_empty(){random()}else{seed.trim().into()}}
pub const ITEMS: [Item; 7] = [
    Item { label: "DESCEND", x: 103, y: 443, w: 375, h: 66, size: 32 },
    Item { label: "CONTINUE", x: 174, y: 515, w: 434, h: 54, size: 31 },
    Item { label: "VIEW REPLAY", x: 266, y: 581, w: 342, h: 39, size: 24 },
    Item { label: "VIEW HIGH SCORES", x: 266, y: 624, w: 342, h: 39, size: 24 },
    Item { label: "VIEW BESTIARY", x: 266, y: 667, w: 342, h: 39, size: 24 },
    Item { label: "SETTINGS", x: 266, y: 710, w: 342, h: 39, size: 24 },
    Item { label: "PLAY FROM X'TH FLOOR", x: 174, y: 753, w: 434, h: 39, size: 22 },
];

/// O(1), fixed twelve-control traversal; unavailable Continue is skipped.
pub fn next_item(selected: usize, forward: bool, can_continue: bool) -> usize {
    const ORDER: [usize; 11] = [0, 8, 9, 10, SEED_INDEX, 1, 2, 3, 4, 5, 6];
    let mut i = ORDER.iter().position(|v| *v == selected).unwrap_or(0);
    loop {
        i = (i + if forward { 1 } else { ORDER.len() - 1 }) % ORDER.len();
        if ORDER[i] != 1 || can_continue { return ORDER[i]; }
    }
}

pub fn item_at(x: i32, y: i32) -> Option<usize> {
    if seed_at(x,y){return Some(SEED_INDEX);}
    if let Some(slot)=crate::avatar_menu::slot_at(x,y){return Some(crate::avatar_menu::GOD_START+slot);}
    ITEMS.iter().position(|r| x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h)
}

// Original continuous woodcut contours, normalized to a 100-unit em. Heavy
// compressed stems and diagonal chamfers are deliberately unlike a bitmap face.
fn letter(c: char) -> &'static [&'static [(i32, i32)]] {
    match c {
        'A' => &[&[(19,0),(81,0),(100,100),(67,100),(63,76),(37,76),(33,100),(0,100)], &[(45,25),(55,25),(60,54),(40,54)]],
        'C' => &[&[(17,0),(84,0),(100,16),(100,32),(69,32),(69,24),(32,24),(32,76),(69,76),(69,67),(100,67),(100,84),(84,100),(16,100),(0,84),(0,17)]],
        'D' => &[&[(0,0),(76,0),(100,21),(100,79),(77,100),(0,100)], &[(33,24),(63,24),(68,30),(68,70),(61,76),(33,76)]],
        'E' => &[&[(0,0),(100,0),(100,24),(34,24),(34,38),(87,38),(87,62),(34,62),(34,76),(100,76),(100,100),(0,100)]],
        'F' => &[&[(0,0),(100,0),(100,24),(34,24),(34,40),(87,40),(87,64),(34,64),(34,100),(0,100)]],
        'G' => &[&[(17,0),(84,0),(100,16),(100,31),(68,31),(68,24),(32,24),(32,76),(69,76),(69,60),(53,60),(53,40),(100,40),(100,84),(84,100),(16,100),(0,84),(0,17)]],
        'H' => &[&[(0,0),(33,0),(33,37),(67,37),(67,0),(100,0),(100,100),(67,100),(67,63),(33,63),(33,100),(0,100)]],
        'L' => &[&[(0,0),(34,0),(34,75),(100,75),(100,100),(0,100)]],
        'O' => &[&[(17,0),(83,0),(100,17),(100,83),(83,100),(17,100),(0,83),(0,17)], &[(33,24),(67,24),(67,76),(33,76)]],
        'P' => &[&[(0,0),(81,0),(100,17),(100,49),(82,67),(34,67),(34,100),(0,100)], &[(34,23),(66,23),(66,44),(34,44)]],
        'R' => &[&[(0,0),(81,0),(100,17),(100,46),(82,63),(100,100),(63,100),(46,65),(34,65),(34,100),(0,100)], &[(34,23),(66,23),(66,43),(34,43)]],
        'S' => &[&[(16,0),(84,0),(100,16),(100,31),(68,31),(68,23),(32,23),(32,36),(85,45),(100,59),(100,84),(84,100),(16,100),(0,84),(0,68),(32,68),(32,77),(68,77),(68,63),(15,54),(0,40),(0,16)]],
        'T' => &[&[(0,0),(100,0),(100,25),(67,25),(67,100),(33,100),(33,25),(0,25)]],
        _ => &[],
    }
}
/// Constant-count contour scan conversion at drawable resolution. Static wear
/// never changes selection geometry or introduces animation-driven title jitter.
pub fn lettering(p: &mut Painter, text: &str, x: i32, y: i32, width: i32, height: i32, color: Rgb, worn: bool) {
    let units: i32 = text.chars().map(|c| if c == ' ' { 42 } else { 112 }).sum::<i32>() - 12;
    let mut cursor = 0;
    for (index, ch) in text.chars().enumerate() {
        if ch == ' ' { cursor += 42; continue; }
        let left = x + cursor * width / units;
        let right = x + (cursor + 100) * width / units;
        p.menu_shape(letter(ch), left, y, right - left, height, color, worn.then_some(index));
        cursor += 112;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compressed_debug_floor_validation_and_keyboard_follow_twenty_floors() {
        for floor in 1..=20 {
            assert_eq!(parse_start_floor(&floor.to_string(),true),Ok(floor));
        }
        for bad in ["0","21","40","2.5","bad",""] { assert!(parse_start_floor(bad,true).is_err()); }
        assert_eq!(step_start_floor("1",true,true),"2");
        assert_eq!(step_start_floor("20",true,true),"20");
        assert_eq!(step_start_floor("1",false,true),"1");
    }
    #[test]
    fn fast_mode_is_not_an_interactive_menu_control() {
        assert_eq!(item_at(103,8),None);
        for can_continue in [false,true] {
            let mut selected=0;let mut visited=Vec::new();
            loop {assert!(!visited.contains(&selected));visited.push(selected);
                selected=next_item(selected,true,can_continue);if selected==0 {break;}}
            assert!(!visited.contains(&FAST_INDEX));
            assert_eq!(visited.len(),if can_continue {11}else{10});
        }
        let mut p=Painter::new();p.menu(18,0,false,"Avatar");let before=p.pixels.clone();
        p.menu_fast_mode(true,true);assert_eq!(before,p.pixels);
    }
    #[test]
    fn complete_large_hit_regions_and_no_gap_activation() {
        for (i, r) in ITEMS.iter().enumerate() {
            assert_eq!(item_at(r.x, r.y), Some(i));
            assert_eq!(item_at(r.x + r.w - 1, r.y + r.h - 1), Some(i));
            assert_eq!(item_at(r.x - 1, r.y), None);
            assert_eq!(item_at(r.x + r.w, r.y), None);
            assert_eq!(item_at(r.x, r.y + r.h), None);
            assert!(r.h >= 39 && r.y + r.h <= 800);
        }
    }
    #[test]
    fn settings_is_accessible_from_the_main_menu() {
        assert_eq!(ITEMS.len(), 7);
        assert_eq!(item_at(300, 730), Some(5));
    }
    #[test]
    fn selection_never_moves_composition_and_disabled_continue_is_gray() {
        let mut p = Painter::new();
        p.menu(18, 0, false, "Avatar");
        let before = p.pixels.clone();
        p.menu(18, 5, false, "Avatar");
        for y in 0..800 {
            for x in 0..1280 {
                if !matches!(item_at(x, y), Some(0 | 5)) {
                    let at = (y as usize * 1280 + x as usize) * 4;
                    assert_eq!(&before[at..at + 4], &p.pixels[at..at + 4]);
                }
            }
        }
        p.menu(18, 1, false, "Avatar");
        let at = ((ITEMS[1].y + 2) as usize * 1280 + (ITEMS[1].x + 2) as usize) * 4;
        assert_eq!(&p.pixels[at..at + 3], &[74, 70, 70]);
    }
    #[test]
    fn common_aspect_ratios_keep_every_hit_region_on_screen() {
        for (width, height) in [(960, 540), (1280, 720), (1280, 800), (1920, 1080), (2560, 1600)] {
            let scale = (width as f64 / 1280.).min(height as f64 / 800.);
            let ox = (width as f64 - 1280. * scale) / 2.;
            let oy = (height as f64 - 800. * scale) / 2.;
            for r in &ITEMS {
                assert!(ox + r.x as f64 * scale >= 0.);
                assert!(oy + r.y as f64 * scale >= 0.);
                assert!(ox + (r.x + r.w) as f64 * scale <= width as f64);
                assert!(oy + (r.y + r.h) as f64 * scale <= height as f64);
            }
        }
    }
    #[test]
    fn keyboard_traversal_includes_three_gods_and_skips_disabled_continue() {
        assert_eq!(next_item(0, true, false), 8);
        assert_eq!(next_item(8, true, false), 9);
        assert_eq!(next_item(9, true, false), 10);
        assert_eq!(next_item(10, true, false), SEED_INDEX);
        assert_eq!(next_item(SEED_INDEX, true, false), 2);
        assert_eq!(next_item(SEED_INDEX, true, true), 1);
        assert_eq!(next_item(2, false, false), SEED_INDEX);
        assert_eq!(next_item(0, false, false), 6);
        assert_eq!(next_item(6, true, true), 0);
    }
    /// O(control count), input and first row never overlap each other or anatomical art.
    #[test]
    fn seed_input_is_beside_descend_and_outside_art(){
        let(x,y,w,h)=SEED_RECT;assert_eq!(y,ITEMS[0].y);assert!(x>ITEMS[0].x+ITEMS[0].w);assert!(x+w<=662);assert!(x+w<676);
        assert_eq!(item_at(x,y),Some(SEED_INDEX));assert_eq!(item_at(x+w-1,y+h-1),Some(SEED_INDEX));assert_eq!(item_at(x+w,y),None);
        for(width,height)in[(640,480),(960,540),(1280,800),(1920,1080)]{let scale=(width as f64/1280.).min(height as f64/800.);let ox=(width as f64-1280.*scale)/2.;let oy=(height as f64-800.*scale)/2.;assert!(ox+x as f64*scale>=0.&&ox+(x+w)as f64*scale<=width as f64);assert!(oy+y as f64*scale>=0.&&oy+(y+h)as f64*scale<=height as f64);}
    }
    /// O(text bytes), typing/paste never alters mode and preserves validation input exactly.
    #[test]
    fn seed_edits_are_bounded_and_blank_random_is_selected_once(){
        let mut s=String::new();assert!(edit_seed(&mut s,"02-04-09:fixed",false));assert_eq!(selected_seed(&s,||panic!("entered seed must not randomize")),s);
        assert!(edit_seed(&mut s," 02-04-09:pasted ",true));assert_eq!(selected_seed(&s,||panic!()),"02-04-09:pasted");let old=s.clone();assert!(!edit_seed(&mut s,"bad\nseed",true));assert_eq!(s,old);assert!(!edit_seed(&mut s,&"x".repeat(181),true));
        let calls=std::cell::Cell::new(0);assert_eq!(selected_seed("  ",||{calls.set(calls.get()+1);"random-once".into()}),"random-once");assert_eq!(calls.get(),1);
        s="é".into();s.pop();assert!(s.is_empty());
    }
    /// O(menu pixels + glyphs), seed overlay never paints over heart/ribs and long text is clipped by measured tail.
    #[test]
    fn seed_overlay_changes_only_its_rectangle(){let mut p=Painter::new();p.menu(18,0,false,"Avatar");let before=p.pixels.clone();let seed="W".repeat(180);let tail=p.menu_seed_tail(&seed);assert!(tail.len()<seed.len());assert!(seed.ends_with(&tail));p.menu_seed(&seed,true,false);for y in 0..800{for x in 0..1280{if !seed_at(x,y){let at=(y as usize*1280+x as usize)*4;assert_eq!(&before[at..at+4],&p.pixels[at..at+4]);}}}}
    #[test]
    fn title_has_every_authored_letter() {
        for c in "CALL OF THE CORPSE GODS".chars().filter(|c| *c != ' ') {
            assert!(!letter(c).is_empty());
        }
    }
}
