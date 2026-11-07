use super::*;
fn view()->View{let r:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();crate::observe::observe(&r)}
fn equal(p:&Painter,f:&Painter,label:&str){if let Some((i,(a,b)))=p.pixels.iter().zip(&f.pixels).enumerate().find(|(_, (a,b))|a!=b){panic!("{label} pixelbyte{i}: tile{a}, full{b}");}assert_eq!(p.pixels.len(),f.pixels.len());}
#[test]fn exact_spans_preserve_l_shapes_and_binary_row_intersections(){let mut s=RowSpans::default();s.reset(1280,800);s.add(Rect::new(0,0,990,128));s.add(Rect::new(990,0,290,800));s.finish();assert_eq!(s.area(),990*128+290*800);assert!(!s.contains(500,400));assert!(!s.intersects(Rect::new(200,300,10,10)));assert!(s.intersects(Rect::new(989,300,3,10)));assert_eq!(s.upload_rectangles().iter().map(|r|r.area()).sum::<usize>(),s.area());let spans:Vec<_>=row_segments(Some(&s),300,500,1100).collect();assert_eq!(spans,vec![(990,1100)]);}
#[test]fn borrowed_transitions_match_full_at_three_native_scales(){
 for scale in [1.,1.5,2.]{let mut p=Painter::new();let mut f=Painter::new();p.resize((1280.*scale)as usize,(800.*scale)as usize);f.resize(p.width,p.height);let mut plan=TilePlanner::default();let mut old=None;
  for step in 0..11{let mut next=view();match step{
   2=>{next.hp-=1.;next.log_groups.push(crate::chronicle::Group{message:"Changed Chronicle".into(),kind:String::new(),count:1,index:0});},
   3=>{let c=next.cells.last_mut().unwrap();c.x=c.x.saturating_sub(1);c.fg=[20,240,180];c.superscript="LONG".into();c.timeweave=Some((-1,0));},
   4=>{next.cells.pop();},
   5=>{let n=next.cells.len();next.cells.swap(n-1,n-2);},
   6=>{for c in next.cells.iter_mut().take(8){c.current=false;c.direct=false;c.bg=[22,20,25];}},
   7=>{next.cells.clear();},
   8=>{next.path=next.cells.iter().take(5).map(|c|(c.x,c.y)).collect();next.hover=Some((next.cells[0].x,next.cells[0].y));},
   9=>{next.cells[0].emphasis=3.;next.cells[0].subscript="12345".into();next.cells[1].emphasis=3.;},_=>{}}
   plan.replace_view(old.as_ref(),&next,&p);plan.mark_animation(&next,&p,0.,false);plan.finish_damage(&mut p);
   if step==1{assert!(p.tile_mask.is_empty(),"identical borrowed paint should skip");}
   plan.paint_base(&mut p,&next,"",0.);p.tile_clip_enabled=false;f.game_at(&next,"",false,0.);equal(&p,&f,&format!("scale{scale} step{step}"));plan.clear_frame();old=Some(next);
  }
 }
}
#[test]fn clock_candidates_and_reduced_time_reset_are_exact(){let mut v=view();for(i,c)in v.cells.iter_mut().enumerate(){c.animations.clear();c.pulse_speed=0.;if i%7==0{c.animations.push(Animation::Fire{phase:i as f64/9.,speed:1.,colors:[[230,30,0],[255,100,0],[255,240,80],[255,130,0]]});}if i%13==0{c.animations.push(Animation::Gas{phase:i as f64,color:[90,200,70]});}}
 let mut p=Painter::new();let mut f=Painter::new();let mut plan=TilePlanner::default();plan.replace_view(None,&v,&p);
 for(t,reduced)in [(0.,false),(200.,false),(550.,false),(550.,true),(550.,true)]{plan.mark_animation(&v,&p,t,reduced);plan.finish_damage(&mut p);let time=if reduced{0.}else{t};plan.paint_base(&mut p,&v,"",time);p.tile_clip_enabled=false;f.game_at(&v,"",false,time);equal(&p,&f,"clock");plan.clear_frame();}
}
#[test]fn overlay_close_over_unknown_cells_and_sparse_writers_are_exact(){let mut v=view();v.cells.clear();let mut p=Painter::new();let mut f=Painter::new();let mut plan=TilePlanner::default();plan.replace_view(None,&v,&p);plan.finish_damage(&mut p);plan.paint_base(&mut p,&v,"",0.);p.tile_clip_enabled=false;plan.clear_frame();let r=Rect::new(100,200,250,160);p.blend_rect(100,200,250,160,[80,20,90],150);plan.mark_native_rect(r);plan.finish_damage(&mut p);plan.paint_base(&mut p,&v,"",0.);p.tile_clip_enabled=false;f.game_at(&v,"",false,0.);equal(&p,&f,"unknown overlay close");
 let before=p.pixels.clone();p.tile_mask.reset(p.width,p.height);p.tile_mask.add(Rect::new(30,30,15,20));p.tile_mask.add(Rect::new(90,30,15,20));p.tile_mask.finish();p.tile_clip_enabled=true;p.clear([20,40,60]);p.text("Wide clipping",0,25,30,[240,200,100]);
 for y in 0..p.height{for x in 0..p.width{if !p.tile_mask.contains(x,y){let i=(y*p.width+x)*4;assert_eq!(&p.pixels[i..i+4],&before[i..i+4]);}}}
}
#[test]fn ieee_paint_comparison_is_bit_exact(){let mut a=Cell::default();let mut b=Cell::default();a.emphasis=f64::from_bits(0x7ff8000000000001);b.emphasis=a.emphasis;assert!(paint_eq(&a,&b));b.emphasis=f64::from_bits(0x7ff8000000000002);assert!(!paint_eq(&a,&b));a.emphasis=0.;b.emphasis=-0.;assert!(!paint_eq(&a,&b));}
#[test]fn selected_actor_only_marks_old_new_footprints(){let mut old=view();let id=old.cells.iter().find_map(|c|c.actor_id).expect("fixture actor");old.selected_actor=Some(id);let mut next=view();next.selected_actor=None;let mut p=Painter::new();let mut f=Painter::new();let mut plan=TilePlanner::default();plan.replace_view(None,&old,&p);plan.finish_damage(&mut p);plan.paint_base(&mut p,&old,"",0.);p.tile_clip_enabled=false;plan.clear_frame();plan.replace_view(Some(&old),&next,&p);plan.finish_damage(&mut p);assert!(!p.tile_mask.is_empty());plan.paint_base(&mut p,&next,"",0.);p.tile_clip_enabled=false;f.game_at(&next,"",false,0.);equal(&p,&f,"selection only");}
#[test]fn meter_signed_zero_label_and_profile_invalidation_match_full(){let mut old=view();old.player_meters=vec![crate::statuses_ui::Meter{name:"Time".into(),duration:Some(0.),reference:1.,reference_known:true,percent:0.}];let mut next=view();next.player_meters=old.player_meters.clone();next.player_meters[0].duration=Some(-0.);assert!(!hud_eq(&old,&next));let mut p=Painter::new();let mut f=Painter::new();let mut plan=TilePlanner::default();plan.replace_view(None,&old,&p);plan.finish_damage(&mut p);plan.paint_base(&mut p,&old,"",0.);p.tile_clip_enabled=false;plan.clear_frame();plan.replace_view(Some(&old),&next,&p);plan.finish_damage(&mut p);assert!(!p.tile_mask.is_empty());plan.paint_base(&mut p,&next,"",0.);p.tile_clip_enabled=false;f.game_at(&next,"",false,0.);equal(&p,&f,"signed zero meter");plan.clear_frame();p.profile=p.profile.set("ui.colors.background",serde_json::json!("#203040")).unwrap();f.profile=p.profile.clone();plan.invalidate();plan.replace_view(Some(&next),&next,&p);plan.finish_damage(&mut p);assert_eq!(p.tile_mask.area(),p.width*p.height);plan.paint_base(&mut p,&next,"",0.);p.tile_clip_enabled=false;f.game_at(&next,"",false,0.);equal(&p,&f,"profile invalidate");}
#[test]fn raw_and_merged_span_diagnostics_show_duplicate_work(){let v=view();let mut p=Painter::new();let mut plan=TilePlanner::default();plan.replace_view(None,&v,&p);plan.finish_damage(&mut p);plan.clear_frame();plan.eligibility.enabled=true;for _ in 0..10{plan.mark_native_rect(Rect::new(10,10,20,5));}plan.finish_damage(&mut p);assert_eq!(plan.eligibility.raw_span_entries,50);assert_eq!(plan.eligibility.merged_span_entries,5);assert_eq!(p.tile_mask.area(),100);}

// Independent per-pixel union oracle: it does not use RowSpans or word masks.
fn assert_bitmap_union(mask:&RowSpans, bitmap:&[bool]) {
 assert_eq!(mask.area(),bitmap.iter().filter(|&&v|v).count());
 for y in 0..mask.height(){for x in 0..mask.width(){assert_eq!(mask.contains(x,y),bitmap[y*mask.width()+x],"pixel {x},{y}");}
  let row=&mask.rows()[y];for pair in row.windows(2){assert!(pair[0].1<pair[1].0,"spans must be separated and maximal");}
  for &(a,b)in row{assert!(a<b&&b<=mask.width());}
 }
 assert_eq!(mask.upload_rectangles().iter().map(|r|r.area()).sum::<usize>(),mask.area());
}
fn mark_bitmap(bitmap:&mut[bool],w:usize,h:usize,r:Rect){for y in 0..h{for x in 0..w{if x>=r.x&&x<r.right()&&y>=r.y&&y<r.bottom(){bitmap[y*w+x]=true;}}}}
#[test]
fn bitset_word_boundaries_clipping_empty_full_and_reuse_match_bitmap(){
 let mut mask=RowSpans::default();
 for w in [0,1,63,64,65,127,128,129]{for h in [0,1,5]{
  mask.reset(w,h);let mut bitmap=vec![false;w*h];mask.finish();assert_bitmap_union(&mask,&bitmap);
  let rectangles=[Rect::new(0,0,0,3),Rect::new(0,0,w,0),Rect::new(w,0,1,2),Rect::new(0,h,3,1),Rect::new(usize::MAX,usize::MAX,10,10),Rect::new(0,0,1,1),Rect::new(62,0,4,3),Rect::new(63,1,65,2),Rect::new(64,0,64,1),Rect::new(126,2,4,3),Rect::new(1,0,1,5),Rect::new(w.saturating_sub(1),0,usize::MAX,5)];
  for r in rectangles{mask.add(r);mark_bitmap(&mut bitmap,w,h,r);mask.finish();assert_bitmap_union(&mask,&bitmap);mask.finish();assert_bitmap_union(&mask,&bitmap);}
  mask.add(Rect::new(0,0,w,h));bitmap.fill(true);mask.finish();assert_bitmap_union(&mask,&bitmap);
  // Reset must clear old word padding and output spans even across width changes.
  mask.reset(w,h);mask.finish();bitmap.fill(false);assert_bitmap_union(&mask,&bitmap);
 }}
}
#[test]
fn bitset_random_union_and_duplicate_rows_match_bitmap(){
 let mut mask=RowSpans::default();mask.diagnostics=true;let mut seed=0x1947u64;
 for w in [1,63,64,65,127,128,129]{let h=9;mask.reset(w,h);let mut bitmap=vec![false;w*h];let mut raw=0;let mut words=0;
  for i in 0..300{let mut next=||{seed=seed.wrapping_mul(6364136223846793005).wrapping_add(1);(seed>>32)as usize};let r=Rect::new(next()%(w+8),next()%12,next()%140,next()%12);
   if let Some(c)=r.intersect(Rect::new(0,0,w,h)){raw+=c.h;words+=c.h*((c.right()-1)/64-c.x/64+1);}
   mask.add(r);mark_bitmap(&mut bitmap,w,h,r);
   if i%13==0{mask.finish();assert_bitmap_union(&mask,&bitmap);}
  }
  mask.finish();assert_bitmap_union(&mask,&bitmap);assert_eq!(mask.raw_entries,raw);assert_eq!(mask.words_touched,words);assert_eq!(mask.words_scanned,h*(w/64+usize::from(w%64!=0)));
 }
 mask.reset(129,5);for _ in 0..1000{mask.add(Rect::new(63,0,2,5));}mask.finish();assert_eq!(mask.raw_entries,5000);assert_eq!(mask.words_touched,10000);assert_eq!(mask.area(),10);assert_eq!(mask.rows().iter().map(Vec::len).sum::<usize>(),5);
}

#[test]
fn same_clock_global_light_snapshot_identity_invalidates_equal_base_cells(){
 use std::sync::Arc;
 let mut p=Painter::new();let mut f=Painter::new();
 // Equal authored source settings except frequency: at time zero the two
 // immutable observations produce identical Cells, but not at positive time.
 let torch=p.profile.value("lighting.sources.torch").unwrap().as_object().unwrap().clone();
 for (key,value) in torch{p.profile=p.profile.set(&format!("lighting.sources.brazier.{key}"),value).unwrap();}
 // Pin an explicit equal hue for both roles; a copied historical default is
 // otherwise unedited torch hue but an explicit override for the brazier.
 for category in ["torch","brazier"]{p.profile=p.profile.set(&format!("lighting.sources.{category}.color"),serde_json::json!("#ffaa66")).unwrap();p.profile=p.profile.set(&format!("lighting.sources.{category}.flickerStrength"),serde_json::json!(0.8)).unwrap();}
 p.profile=p.profile.set("lighting.sources.torch.flickerFrequency",serde_json::json!(0.2)).unwrap();
 p.profile=p.profile.set("lighting.sources.brazier.flickerFrequency",serde_json::json!(3.)).unwrap();
 f.profile=p.profile.clone();assert!(crate::appearance_light::animated(&p.profile));
 let mut old=view();let mut observation=old.terrain_snapshot.as_ref().unwrap().as_ref().clone();
 let i=observation.cells.iter().position(|c|c.current&&c.direct&&c.tile.as_deref()==Some("floor")).expect("visible floor");
 observation.light_sources=vec![corpse_core::vision::ObservedLightSource{origin_known:true,emission_group:0,x:(i%observation.width)as f64,y:(i/observation.width)as f64,kind:"torch".into(),color_role:"torch".into(),power:40.,falloff:1.,actor_id:None,cells:vec![i],strengths:vec![40.]}];
 let mut changed=observation.clone();changed.light_sources[0].kind="brazier".into();
 old.cells=crate::observe::terrain_frame(&observation,&p.profile,old.cosmetic_seed,0.).0;old.terrain_count=old.cells.len();old.terrain_snapshot=Some(Arc::new(observation));
 let mut next=view();next.cells=crate::observe::terrain_frame(&changed,&p.profile,next.cosmetic_seed,0.).0;next.terrain_count=next.cells.len();next.terrain_snapshot=Some(Arc::new(changed));
 assert_eq!(old.cells.len(),next.cells.len());assert!(old.cells.iter().zip(&next.cells).all(|(a,b)|paint_eq(a,b)),"fixture must isolate snapshot dependency, not base paint differences");
 let now=731.;f.game_at(&old,"",false,now);let before=f.pixels.clone();f.game_at(&next,"",false,now);assert!(before!=f.pixels,"fixture must visibly differ at the same positive clock");
 let mut plan=TilePlanner::default();plan.replace_view(None,&old,&p);plan.mark_animation(&old,&p,now,false);plan.finish_damage(&mut p);plan.paint_base(&mut p,&old,"",now);p.tile_clip_enabled=false;plan.clear_frame();
 // Identical Arc, equal paint and equal clock must not spuriously dirty.
 plan.replace_view(Some(&old),&old,&p);plan.mark_animation(&old,&p,now,false);plan.finish_damage(&mut p);assert!(p.tile_mask.is_empty());plan.clear_frame();
 plan.replace_view(Some(&old),&next,&p);plan.mark_animation(&next,&p,now,false);plan.finish_damage(&mut p);assert!(!p.tile_mask.is_empty());plan.paint_base(&mut p,&next,"",now);p.tile_clip_enabled=false;equal(&p,&f,"same clock changed immutable light snapshot");
}
#[test]
fn snapshot_dependency_is_global_only_and_covers_presence_and_count(){
 let old=view();let mut next=view();let mut p=Painter::new();let mut plan=TilePlanner::default();
 assert!(!crate::appearance_light::animated(&p.profile)&&!crate::appearance_material::animated(&p.profile));
 plan.replace_view(None,&old,&p);plan.finish_damage(&mut p);plan.clear_frame();
 plan.replace_view(Some(&old),&next,&p);plan.finish_damage(&mut p);assert!(p.tile_mask.is_empty(),"default profile must not react to observation allocation identity");plan.clear_frame();
 p.profile=p.profile.set("lighting.flickerStrength",serde_json::json!(0.2)).unwrap();
 // Share the same Arc so only terrain_count changes.
 next.terrain_snapshot=old.terrain_snapshot.clone();next.terrain_count=old.terrain_count+1;
 plan.replace_view(Some(&old),&next,&p);plan.finish_damage(&mut p);assert!(!p.tile_mask.is_empty());plan.clear_frame();
 next.terrain_count=old.terrain_count;next.terrain_snapshot=None;
 plan.replace_view(Some(&old),&next,&p);plan.finish_damage(&mut p);assert!(!p.tile_mask.is_empty());plan.clear_frame();
 plan.replace_view(Some(&next),&old,&p);plan.finish_damage(&mut p);assert!(!p.tile_mask.is_empty());
}
