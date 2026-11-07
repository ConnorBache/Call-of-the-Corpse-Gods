use super::*;
fn disjoint(d:&Damage){assert!(d.rects().len()<=8);for(i,a)in d.rects().iter().enumerate(){for b in &d.rects()[..i]{assert!(a.intersect(*b).is_none(),"overlap {a:?} {b:?}");}}}
#[test]
fn l_shape_and_corner_contact_do_not_fill_unchanged_pixels(){
    let mut d=Damage::empty(1280,800);d.add(Rect::new(0,0,990,128));d.add(Rect::new(990,0,290,800));
    assert_eq!(d.rects().len(),2);assert_eq!(d.area(),990*128+290*800);assert!(d.rects().iter().all(|r|!r.contains(500,400)));disjoint(&d);
    let mut d=Damage::empty(30,30);d.add(Rect::new(1,1,4,4));d.add(Rect::new(5,5,4,4));assert_eq!(d.area(),32);assert_eq!(d.rects().len(),2);
}
#[test]
fn aligned_edges_merge_without_added_area_and_containment_stays_one(){
    let mut d=Damage::empty(100,100);d.add(Rect::new(10,10,20,20));d.add(Rect::new(30,10,15,20));assert_eq!(d.rects(),&[Rect::new(10,10,35,20)]);
    d.add(Rect::new(12,12,3,3));assert_eq!(d.area(),700);assert_eq!(d.rects().len(),1);
    d.add(Rect::new(10,30,35,10));assert_eq!(d.rects(),&[Rect::new(10,10,35,30)]);
}
#[test]
fn actual_overlap_remains_conservative_and_fragment_budget_chooses_local_pair(){
    let mut d=Damage::empty(100,100);d.add(Rect::new(0,0,20,10));d.add(Rect::new(10,5,20,10));assert_eq!(d.rects(),&[Rect::new(0,0,30,15)]);
    let mut d=Damage::empty(1000,1000);for i in 0..8{d.add(Rect::new(20+i*100,20+i*90,5,5));}d.add(Rect::new(26,20,5,5));
    assert_eq!(d.rects().len(),8);assert!(d.coalesced_to_bounds);assert_eq!(d.area(),8*25+30);disjoint(&d);
    assert!(d.rects().iter().any(|r|*r==Rect::new(20,20,11,5)));
}
#[test]
fn deterministic_random_insertions_preserve_coverage_bounds_and_nonoverlap(){
    let mut seed=0x1234abcd_u64;let mut next=||{seed=seed.wrapping_mul(6364136223846793005).wrapping_add(1);(seed>>32)as usize};
    for _ in 0..20 {let mut d=Damage::empty(64,48);let mut covered=vec![false;64*48];
        for _ in 0..50 {let r=Rect::new(next()%75,next()%60,1+next()%17,1+next()%13);d.add(r);
            for y in r.y.min(48)..r.bottom().min(48){for x in r.x.min(64)..r.right().min(64){covered[y*64+x]=true;}}
            disjoint(&d);for r in d.rects(){assert!(r.right()<=64&&r.bottom()<=48);}
            for y in 0..48{for x in 0..64{if covered[y*64+x]{assert!(d.rects().iter().any(|r|r.contains(x,y)));}}}
        }
    }
}
#[test]
fn l_shaped_game_damage_and_translucent_overlay_match_full_pixels(){
    let r:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();
    let mut v=crate::observe::observe(&r);let mut p=Painter::new();let mut full=Painter::new();let mut retained=RetainedGame::default();
    retained.update(&mut p,&v,"",0.,true,true).unwrap();
    let overlay=Rect::new(910,40,220,140);
    p.blend_rect(910,40,220,140,[35,60,90],150);
    v.hp-=1.;v.log_groups.push(crate::chronicle::Group{message:"New Chronicle line".into(),kind:String::new(),count:1,index:0});
    let mut damage=retained.update(&mut p,&v,"",0.,true,false).unwrap();
    assert!(damage.area()<p.width*p.height,"touching Chronicle and HUD must not dirty the full frame");
    damage.add(overlay);retained.restore(&mut p,&damage).unwrap();
    for rect in damage.rects(){p.damage_clip=Some(*rect);p.blend_rect(910,40,220,140,[35,60,90],150);}p.damage_clip=None;
    full.game_at(&v,"",false,0.);full.blend_rect(910,40,220,140,[35,60,90],150);assert_eq!(p.pixels,full.pixels);
}
