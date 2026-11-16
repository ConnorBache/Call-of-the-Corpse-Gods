use crate::{avatar_menu::*, menu_ui, render::Painter};
use sdl2::keyboard::{Keycode,Mod};

#[test]
fn avatar_menu_exact_authored_god_catalog_and_colors() {
    let expected=[
        ("SHRAIL","the Godthorn","God of Fury"),
        ("PELIAN","the King of Desire","God of Zeal"),
        ("ISHEILAH","the Trickster","God of Guile"),
        ("NUMO","the Pregenitor","Goddess of Memory"),
        ("ETILL","the Ocean-Black","Goddess of Flow"),
        ("URSA","the Sleeping Mother","Goddess of Dreams"),
        ("LICHOR","the Gushing One","God of Blood"),
        ("DAWEL","the Mouthless","Goddess of Silence"),
        ("GRISHA","the Devourer","God of Ruin"),
        ("ANAVANI","of the Moon","Goddess of Ascendance"),
        ("KATIVE","the Craftsman","God of Artifice"),
        ("MORAE","the Sacrifice","Goddess of Fate"),
    ];
    let mut selection=Selection::default();selection.open(0);
    let mut p=Painter::new();selection.paint_picker(&mut p);
    for (id,god) in GODS.iter().enumerate() {
        assert_eq!((god.name,god.alias,god.domain),expected[id]);
        let(x,y,w,h)=card_rect(id);let at=((y+h-3) as usize*p.width as usize+(x+w-3) as usize)*4;
        assert_eq!(&p.pixels[at..at+3],&crate::ability_ui::god_color(id));
    }
}
#[test]
fn avatar_menu_pointer_regions_slots_cards_cancel_and_gaps() {
    for slot in 0..3 {
        let (x,y,w,h)=slot_rect(slot);
        assert_eq!(slot_at(x,y),Some(slot));assert_eq!(slot_at(x+w-1,y+h-1),Some(slot));
        assert_eq!(slot_at(x+w,y),None);assert_eq!(slot_at(x,y+h),None);
        assert_eq!(menu_ui::item_at(x,y),Some(GOD_START+slot));
    }
    for id in 0..12 {
        let(x,y,w,h)=card_rect(id);
        assert_eq!(card_at(x,y),Some(id));assert_eq!(card_at(x+w-1,y+h-1),Some(id));
        assert_eq!(card_at(x-1,y),None);assert_eq!(card_at(x+w,y),None);assert_eq!(card_at(x,y+h),None);
    }
    let mut s=Selection::default();let mut seed=String::new();s.open(0);
    s.click(0,0,&mut seed);assert!(s.active());s.click(CANCEL.0,CANCEL.1,&mut seed);assert!(!s.active());
}
#[test]
fn avatar_menu_distinct_choices_reject_duplicate_without_seed_mutation() {
    for slot in 0..3 {
        for id in 0..12 {
            let mut s=Selection::default();let mut seed="00-01-02:suffix:é".to_owned();s.open(slot);
            let old=seed.clone();let original=s.gods;
            let allowed=id==slot || id>=3;
            assert_eq!(s.choose(id,&mut seed),allowed);
            if allowed {
                assert!(!s.active());assert_eq!(s.gods[slot],id);
                assert!(seed.ends_with(":suffix:é"));
                let run=corpse_core::startup::new_run_shell(&seed,MODE,false).unwrap();
                assert_eq!(run.gods.iter().map(|g|*g as usize).collect::<Vec<_>>(),s.gods);
            } else {assert_eq!(s.gods,original);assert_eq!(seed,old);assert!(s.active());assert!(!s.picker.unwrap().notice.is_empty());}
        }
    }
}
#[test]
fn avatar_menu_keyboard_mouse_parity_cancel_reopen_and_no_clickthrough() {
    let mut keyboard=Selection::default();let mut mouse=keyboard.clone();
    let mut ks="00-01-02:fixed".to_owned();let mut ms=ks.clone();
    keyboard.open(0);mouse.open(0);
    keyboard.key(Keycode::Down,Mod::NOMOD,&mut ks); // NUMO, id 3
    keyboard.key(Keycode::Return,Mod::NOMOD,&mut ks);
    let(x,y,_,_)=card_rect(3);mouse.hover(x+3,y+3);mouse.click(x+3,y+3,&mut ms);
    assert_eq!(keyboard,mouse);assert_eq!(ks,ms);
    keyboard.open(1);keyboard.key(Keycode::End,Mod::NOMOD,&mut ks);
    keyboard.key(Keycode::Escape,Mod::NOMOD,&mut ks);assert_eq!(keyboard.gods,[3,1,2]);assert_eq!(ks,ms);
    keyboard.open(1);assert_eq!(keyboard.picker.as_ref().unwrap().selected,1);
    keyboard.key(Keycode::Tab,Mod::LSHIFTMOD,&mut ks);assert_eq!(keyboard.picker.as_ref().unwrap().selected,0);
    keyboard.key(Keycode::Tab,Mod::LSHIFTMOD,&mut ks);assert_eq!(keyboard.picker.as_ref().unwrap().selected,12);
    keyboard.key(Keycode::Space,Mod::NOMOD,&mut ks);assert!(!keyboard.active());
    // Repeated confirm after a committed choice cannot touch the closed picker.
    keyboard.key(Keycode::Return,Mod::NOMOD,&mut ks);assert_eq!(keyboard.gods,[3,1,2]);
    keyboard.open(2);keyboard.click(110,507,&mut ks);assert!(keyboard.active()); // modal gap, not underlying DESCEND
}
#[test]
fn avatar_menu_seed_sync_preserves_full_seed_and_free_form_suffix() {
    let mut s=Selection::default();s.sync_seed(" 11-09-03:Exact:Suffix-é ");assert_eq!(s.gods,[11,9,3]);
    assert_eq!(s.prepared_seed(" 11-09-03:Exact:Suffix-é ",||panic!()).unwrap(),"11-09-03:Exact:Suffix-é");
    s.sync_seed("just-a-seed");assert_eq!(s.gods,[11,9,3]);
    assert_eq!(s.prepared_seed("just-a-seed",||panic!()).unwrap(),"11-09-03:just-a-seed");
    assert_eq!(s.prepared_seed("word:other",||panic!()).unwrap(),"11-09-03:word:other");
    s.sync_seed("00-00-03:bad");assert_eq!(s.gods,[11,9,3]);
    assert!(s.prepared_seed("00-00-03:bad",||panic!()).is_err());
    assert!(s.prepared_seed("00-12-03:bad",||panic!()).is_err());
    let calls=std::cell::Cell::new(0);
    assert_eq!(s.prepared_seed("  ",||{calls.set(calls.get()+1);"01-02-04:random-suffix".into()}).unwrap(),"11-09-03:random-suffix");
    assert_eq!(calls.get(),1);
    let mut seed=String::new();s.open(0);s.choose(0,&mut seed);assert_eq!(seed,"");assert_eq!(s.gods,[0,9,3]);
}
#[test]
fn avatar_menu_seed_bounds_and_invalid_state_fail_before_launch() {
    let mut s=Selection::default();
    for seed in ["a\nb".into(),"x".repeat(181),"x".repeat(180)] {
        assert!(s.prepared_seed(&seed,||panic!()).is_err());
    }
    assert!(s.prepared_seed(&format!("00-01-02:{}","x".repeat(171)),||panic!()).is_ok());
    let mut oversized="x".repeat(180);let original=oversized.clone();s.open(0);
    assert!(!s.choose(3,&mut oversized));assert_eq!(oversized,original);assert_eq!(s.gods,[0,1,2]);
    s.gods=[0,0,2];assert!(s.prepared_seed("",||panic!()).is_err());
    s.gods=[0,1,12];assert!(s.prepared_seed("",||panic!()).is_err());
}
#[test]
fn avatar_menu_all_controls_fit_scaled_canvas_and_labels_fit_cards() {
    let mut p=Painter::new();
    for god in GODS.iter() {
        for (text,size) in [(god.name,24),(god.alias,18),(god.domain,15)] {
            assert!(p.measure_text_role(crate::typography_ui::Role::Primary,text,size)<=336.,"{}",text);
        }
    }
    for(width,height)in[(640,480),(960,540),(1280,800),(1600,900),(1920,1200)] {
        let scale=(width as f64/1280.).min(height as f64/800.);
        let ox=(width as f64-1280.*scale)/2.;let oy=(height as f64-800.*scale)/2.;
        for(x,y,w,h)in(0..3).map(slot_rect).chain((0..12).map(card_rect)).chain([CANCEL]) {
            assert!(ox+x as f64*scale>=0.&&ox+(x+w)as f64*scale<=width as f64);
            assert!(oy+y as f64*scale>=0.&&oy+(y+h)as f64*scale<=height as f64);
        }
        p.resize(width,height);
        for dpi in [1,2] {
            let window=((width/dpi) as u32,(height/dpi) as u32);
            let (x,y,w,h)=card_rect(4);
            let native=(((ox+(x+w/2) as f64*scale)/dpi as f64).round() as i32,
                        ((oy+(y+h/2) as f64*scale)/dpi as f64).round() as i32);
            let logical=p.logical_point_for_canvas(native.0,native.1,window,false);
            assert_eq!(card_at(logical.0,logical.1),Some(4));
            let mut choice=Selection::default();let mut seed="00-01-02:resize".to_owned();choice.open(2);
            choice.click(logical.0,logical.1,&mut seed);
            assert_eq!(choice.gods,[0,1,4]);assert_eq!(seed,"00-01-04:resize");assert!(!choice.active());
        }
    }
}
#[test]
fn avatar_menu_native_scaled_captures() {
    let Ok(dir)=std::env::var("CORPSE_AVATAR_MENU_CAPTURE") else{return;};
    std::fs::create_dir_all(&dir).unwrap();
    let mut p=Painter::new();let mut s=Selection::default();s.sync_seed("03-09-11:avatar-menu-review");
    for(width,height)in[(1280,800),(1600,900),(1920,1200)] {
        p.resize(width,height);
        for picker in [false,true] {
            s.close();p.menu(18,GOD_START,false,MODE);p.menu_seed("03-09-11:avatar-menu-review",false,false);s.paint_slots(&mut p,GOD_START);
            if picker {s.open(0);s.paint_picker(&mut p);}
            let path=std::path::Path::new(&dir).join(format!("{}-{width}x{height}.png",if picker{"god-picker"}else{"avatar-menu"}));
            let mut encoder=png::Encoder::new(std::fs::File::create(path).unwrap(),width as u32,height as u32);
            encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);
            encoder.write_header().unwrap().write_image_data(&p.pixels).unwrap();
        }
    }
}
#[test]
fn avatar_menu_removed_modes_fail_for_normal_and_debug_start() {
    for mode in ["Unblessed","Blessed","unknown",""] {
        for floor in [None,Some("1")] {
            let error=match crate::new_selected_run("00-01-02:avatar-admission",mode,floor) {Ok(_)=>panic!("removed mode admitted"),Err(error)=>error};
            assert!(error.contains("Avatar-only"));
        }
    }
}
