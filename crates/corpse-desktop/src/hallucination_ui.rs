//! Revision43 cosmetic identities. An 800ms sample is shared by all presentation
//! surfaces; no Run, RNG, visibility, save token or canonical message is mutated.
use corpse_core::{Actor,Run};
pub const VARIANTS:usize=6;
pub const SAMPLE_MS:f64=800.;
/// O(A) existing shared-origin lookup, once when rebuilding observed data.
pub fn active(r:&Run)->bool {r.simulation_revision.value_or(29.)>=43. && corpse_core::half_here::origins(r).iter().any(|a|!a.dead.value_or(false)&&corpse_core::engine::condition(a,"hallucinating")>0.)}
pub fn eligible(a:&Actor)->bool {!corpse_core::half_here::is_clone(a)&&!corpse_core::dreamwalking::is_nightmare(a)&&!corpse_core::dreamwalking::is_body(a)}
pub fn sample(now:f64,reduced:bool)->usize {if reduced||!now.is_finite()||now<=0. {0}else{((now/SAMPLE_MS).floor()as u64%VARIANTS as u64)as usize}}
/// O(seed bytes), only while preparing a fixed six-entry display palette.
/// Cycling the fixed palette guarantees adjacent samples differ.
pub fn kind(r:&Run,a:&Actor,phase:usize)->&'static str {let mut h=2166136261u32;for b in r.seed.bytes(){h=(h^b as u32).wrapping_mul(16777619);}let at=(h as usize).wrapping_add(a.id.to_bits()as usize).wrapping_add(phase)%VARIANTS;corpse_core::new_creatures::DISGUISES[at]}
pub fn question(s:&mut String){if !s.is_empty()&&!s.ends_with('?'){s.push('?');}}
/// Preserve the uncertainty suffix after a detached portal/pocket qualifier.
pub fn append_alias(c:&mut crate::render::Cell,suffix:&str){let uncertain=c.hallucination.is_some();if uncertain&&c.label.ends_with('?'){c.label.pop();}c.label.push_str(suffix);if uncertain{question(&mut c.label);}}
pub fn view_active(v:&crate::render::View)->bool {!v.hallucination_cells.is_empty()}
/// O(observed layers + six-entry payload selection). No world reads or raster work.
pub fn apply_view(v:&mut crate::render::View,phase:usize){
 if v.hallucination_phase==phase{return;}v.hallucination_phase=phase;if !view_active(v){return;}
 for &index in &v.hallucination_cells {let Some(c)=v.cells.get_mut(index)else{continue;};if let Some(cache)=c.hallucination.clone(){let x=c.x;let y=c.y;let bg=c.bg;let alias=if c.label.contains(" (through portal)"){" (through portal)"}else if c.label.contains(" (linked space)"){" (linked space)"}else{""};*c=cache[phase].clone();c.x=x;c.y=y;c.bg=bg;if !alias.is_empty(){if c.label.ends_with('?'){c.label.pop();}c.label.push_str(alias);}question(&mut c.label);c.hallucination=Some(cache);}}
 for c in &mut v.creatures {if let Some(cache)=c.hallucination.clone(){*c=cache[phase].clone();c.hallucination=Some(cache);}}
}
/// Paint-only damage for opt-in tile mode; full-paint default has no planner work.
pub fn mark_view(p:&crate::render::Painter,v:&crate::render::View,plan:&mut crate::render::tile_damage::TilePlanner){for &index in &v.hallucination_cells{if let Some(c)=v.cells.get(index){if let Some(r)=p.cell_damage_rect(c,v.width,v.height){plan.mark_native_rect(r);}}}if view_active(v){let(_,_,ox,_)=p.map_layout(v.width,v.height);let crate::modal_ui::Rect(x,y,w,h)=crate::render::enemy_list::Layout::resolve_frame(&p.profile,ox,p.frame()).bounds;if let Some(r)=p.logical_damage_rect(x,y,w,h){plan.mark_native_rect(r);}}crate::tile_overlay::mark_presentation(p,v,plan);}

/// Apply the existing lighting/Dream transform to base and cached actor entries.
pub fn map_cell(c:&mut crate::render::Cell,mut f:impl FnMut(&mut crate::render::Cell)){f(c);if let Some(v)=c.hallucination.as_mut(){for entry in std::sync::Arc::make_mut(v).iter_mut(){f(entry);}}}

#[cfg(test)]
mod tests {
 use super::*;
 fn room(revision:f64,count:usize)->Run {
  let mut r=corpse_core::startup::new_run_shell_revision("0-5-11:appearance43","Unblessed",false,revision).unwrap();
  r.player.x=2.;r.player.y=2.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
  r.player.conditions.insert("hallucinating".into(),100.);r.player.conditions.insert("telepathic".into(),100.);
  let(w,h)=(79,29);let mut tiles=vec!["wall".into();w*h];for y in 1..h-1{for x in 1..w-1{tiles[y*w+x]="floor".into();}}
  r.floors=vec![std::sync::Arc::new(corpse_core::Floor{width:(w as f64).into(),height:(h as f64).into(),depth:1.,domain:"Dungeon".into(),tiles,lights:vec![100.;w*h],seen:vec![true;w*h],..Default::default()})];
  let actors=(0..count).map(|i|corpse_core::population::create_actor_revision("rat",100.+i as f64,5.+(i%8)as f64,5.+(i/8)as f64,false,revision).unwrap()).collect();
  r.actors.clear();r.actors.insert("1".into(),std::sync::Arc::new(actors));r.ground.clear();r.ground.insert("1".into(),std::sync::Arc::new(Vec::new()));r.next_id=200.+count as f64;r
 }
 #[test]
 fn revision43_shared_phase_keeps_map_list_and_report_coherent_without_canonical_writes(){
  let engine=crate::session_store::tracked_engine::Engine::from_core(corpse_core::temporal::Engine::new(room(43.,1)).unwrap());
  let token=engine.token();let before=serde_json::to_string(&engine.run).unwrap();
  let mut view=crate::observe::observe(&engine.run);view.measure_conditions(&mut crate::statuses_ui::ConditionMeters::default());let mut report=crate::inspection_ui::observe_tile(&engine.run,5,5);
  let self_cell=view.cells.iter().find(|c|c.actor_id==Some(engine.run.player.id)).cloned();
  assert!(self_cell.as_ref().is_none_or(|c|c.hallucination.is_none()));
  assert_eq!(view.creatures.len(),1);let mut previous=String::new();
  #[cfg(feature="runtime-timing")] {corpse_core::timing::reset();corpse_core::timing::set_enabled(true);}
  for i in 0..100 {
   let phase=sample(i as f64*SAMPLE_MS,false);apply_view(&mut view,phase);crate::inspection_ui::apply_hallucination(&mut report,phase);
   let c=view.cells.iter().find(|c|c.actor_id==Some(100.)).unwrap();let row=&view.creatures[0];
   assert_eq!(c.label,row.name);assert_eq!(c.label,report.title);assert!(c.label.ends_with('?'));assert!(c.superscript.ends_with('?'));
   assert_eq!(row.statuses.iter().map(|s|s.name.as_str()).collect::<Vec<_>>(),row.meters.iter().map(|m|m.name.as_str()).collect::<Vec<_>>());
   assert_eq!(row.glyph,format!("{}{}",c.glyph,c.arrow));assert_ne!(previous,c.label);previous=c.label.clone();
   let expected=crate::creature_presentation::project_kind(&engine.run,&engine.run.actors_at(1.)[0],kind(&engine.run,&engine.run.actors_at(1.)[0],phase)).unwrap();
   assert_eq!(row.max_energy,expected.stats.max_energy);assert_eq!(row.max_hp,expected.max_hp);assert!(report.creature.as_ref().unwrap().resources.contains(&format!("/{}",expected.max_hp)));
  }
  #[cfg(feature="runtime-timing")] {let times=corpse_core::timing::take_report();corpse_core::timing::set_enabled(false);assert!(!times.contains_key("vision.observe_terrain"),"cached samples must not observe the world: {times:?}");}
  assert_eq!(engine.token(),token);assert_eq!(serde_json::to_string(&engine.run).unwrap(),before);
  assert_eq!(sample(123456.,true),0);assert_eq!(sample(f64::NAN,false),0);
  apply_view(&mut view,sample(SAMPLE_MS,true));crate::inspection_ui::apply_hallucination(&mut report,0);assert_eq!(view.creatures[0].name,report.title);
 }
 #[test]
 fn revision43_keeps_self_and_owned_body_presentation(){
  let r=room(43.,1);assert!(crate::creature_presentation::project(&r,&r.player).is_none());
  let mut a=r.actors_at(1.)[0].clone();a.conditions.insert("halfHereClone".into(),1.);assert!(!eligible(&a));
  assert_eq!(sample(399.,false),0);assert_eq!(sample(799.,false),0);assert_eq!(sample(800.,false),1);assert_eq!(sample(4800.,false),0);
 }
 #[test]
 fn revision42_remains_tick_based_and_hidden_actors_are_not_cached(){
  let r=room(42.,1);let mut view=crate::observe::observe(&r);let mut report=crate::inspection_ui::observe_tile(&r,5,5);
  assert!(!view_active(&view));assert!(report.hallucination.is_none());let title=report.title.clone();
  apply_view(&mut view,5);crate::inspection_ui::apply_hallucination(&mut report,5);assert_eq!(report.title,title);assert!(!title.ends_with('?'));
  let mut hidden=room(43.,1);hidden.player.conditions.remove("telepathic");hidden.player.stats.perception=0.;hidden.actors_at_mut(1.)[0].x=60.;hidden.actors_at_mut(1.)[0].y=20.;for y in 1..28{hidden.floor_mut().unwrap().tiles[y*79+20]="wall".into();}
  let view=crate::observe::observe(&hidden);assert!(!view.creatures.iter().any(|c|c.id==100.));assert!(!view.cells.iter().any(|c|c.actor_id==Some(100.)));
  let report=crate::inspection_ui::observe_tile(&hidden,60,20);assert!(report.creature.is_none());assert!(report.title.ends_with('?'));assert!(report.hallucination.is_none());
 }
 #[test]
 fn revision43_cosmetic_actor_damage_matches_full_pixels_at_native_scales(){
  for scale in [1.,1.5,2.] {
   let r=room(43.,1);let mut view=crate::observe::observe(&r);let mut painter=crate::render::Painter::new();let mut full=crate::render::Painter::new();painter.resize((1280.*scale)as usize,(800.*scale)as usize);full.resize(painter.width,painter.height);
   let mut plan=crate::render::tile_damage::TilePlanner::default();plan.replace_view(None,&view,&painter);plan.finish_damage(&mut painter);plan.paint_base(&mut painter,&view,"",0.);painter.tile_clip_enabled=false;plan.clear_frame();
   for phase in 1..6 {mark_view(&painter,&view,&mut plan);apply_view(&mut view,phase);mark_view(&painter,&view,&mut plan);plan.finish_damage(&mut painter);plan.paint_base(&mut painter,&view,"",0.);painter.tile_clip_enabled=false;full.game_at(&view,"",false,0.);assert_eq!(painter.pixels.len(),full.pixels.len());if let Some(i)=painter.pixels.iter().zip(&full.pixels).position(|(a,b)|a!=b){panic!("phase {phase}, scale {scale}, differing pixel byte {i}: {} vs {}",painter.pixels[i],full.pixels[i]);}plan.clear_frame();}
  }
 }
 /// Lead-run only. Initial report formatting and steady selection are separate;
 /// no paint/upload cost is claimed. Each row contains all raw independent samples.
 #[test]
 #[ignore]
 fn revision43_hallucination_cache_benchmark(){
  #[cfg(feature="runtime-timing")] corpse_core::timing::set_enabled(false);
  // Shared profile/catalog initialization only; target-specific caches below stay cold.
  let warm=room(42.,1);std::hint::black_box(crate::observe::observe(&warm));std::hint::black_box(crate::inspection_ui::observe_tile(&warm,5,5));
  for count in [1,32]{for revision in [42.,43.]{
   let mut rows=Vec::new();
   for sample_index in 0..20 {
    let r=room(revision,count);let before=serde_json::to_string(&r).unwrap();
    let start=std::time::Instant::now();let mut view=crate::observe::observe(&r);let observe_ms=start.elapsed().as_secs_f64()*1000.;
    assert_eq!(view.creatures.len(),count,"actual visible/sensed fixture count");
    let start=std::time::Instant::now();let mut report=crate::inspection_ui::observe_tile(&r,5,5);let open_ms=start.elapsed().as_secs_f64()*1000.;
    let start=std::time::Instant::now();for phase in 0..100{apply_view(&mut view,phase%6);crate::inspection_ui::apply_hallucination(&mut report,phase%6);std::hint::black_box(&view);std::hint::black_box(&report);}let cached_100_ms=start.elapsed().as_secs_f64()*1000.;
    assert_eq!(serde_json::to_string(&r).unwrap(),before);
    rows.push(serde_json::json!({"sample":sample_index,"observe_ms":observe_ms,"hover_open_ms":open_ms,"cached_100_ms":cached_100_ms,"actual_actors":view.creatures.len(),"cells":view.cells.len()}));
   }
   println!("HALLUCINATION_CACHE {}",serde_json::json!({"revision":revision,"actors":count,"n":20,"rows":rows}));
  }}
 }
}
