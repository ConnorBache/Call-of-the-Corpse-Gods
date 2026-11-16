//! Command-boundary visible lighting and apparent-disguise transitions only.
use crate::{Run,events::{self,EventPoint,Stage}};
pub struct Snapshot {depth:f64,dream:bool,disguises:Vec<(f64,Option<&'static str>,String)>,light:Vec<(usize,f64)>,shadow:bool,chilled:Vec<f64>}
impl Snapshot {
 pub fn capture(r:&Run)->Option<Self>{
  if !events::enabled(r){return None}
  let hallucinates=crate::half_here::origins(r).iter().any(|a|crate::engine::condition(a,"hallucinating")>0.);
  let disguises=if hallucinates{vec![]}else{r.actors_at(r.depth).iter().filter(|a|crate::dreamwalking::same_plane(r,&r.player,a)&&crate::vision::directly_visible_actor(r,a)).map(|a|(a.id,crate::new_creatures::disguise_kind(r,a),crate::actor_presentation::apparent_kind(r,a).and_then(|kind|crate::population::catalog().actors.get(kind)).map_or_else(||a.glyph.clone(),|t|t.glyph.clone()))).filter(|(_,kind,_)|kind.is_some()).collect()};
  let has_darkness=r.actors_at(r.depth).iter().any(|a|a.abilities.iter().any(|id|id.contains("darkness")));
  let light=if has_darkness{crate::vision::observe_terrain(r).map(|o|o.cells.into_iter().filter(|c|c.current&&c.direct).filter_map(|c|c.illumination.map(|l|(c.index,l))).collect()).unwrap_or_default()}else{vec![]};
  Some(Self{depth:r.depth,dream:crate::dreamwalking::dream_actor(r,&r.player),disguises,light,shadow:crate::animation_player::shadow_active(r,&r.player),chilled:crate::monster_chill::observed_chilled_ids(r)})
 }
 pub fn finish(self,r:&Run){
  if self.depth!=r.depth||self.dream!=crate::dreamwalking::dream_actor(r,&r.player){return}
  for id in crate::monster_chill::observed_chilled_ids(r){if !self.chilled.contains(&id)&&events::chill_applied(id){if let Some(a)=crate::engine::actor(r,id){events::cue(r,a,"E14","bracket-frost",Stage::Special,&[a.into()]);}}}
  if !self.shadow&&crate::animation_player::shadow_active(r,&r.player){crate::animation_player::local(r,&r.player,"P43","crescent",Stage::Special);}
  let hallucinating=crate::half_here::origins(r).iter().any(|a|crate::engine::condition(a,"hallucinating")>0.);
  if !hallucinating{for (id,kind,old_glyph)in self.disguises{if let Some(a)=crate::engine::actor(r,id){if kind.is_some()&&crate::new_creatures::disguise_kind(r,a).is_none(){events::morph(r,a,"E67",old_glyph);}}}}
  if !self.light.is_empty(){if let Some(o)=crate::vision::observe_terrain(r){let cells:Vec<_>=self.light.into_iter().filter_map(|(i,old)|o.cells.get(i).filter(|c|c.current&&c.direct&&c.illumination.is_some_and(|n|n<old)).map(|_|EventPoint{x:(i%o.width)as f64,y:(i/o.width)as f64})).collect();events::at(r,"E53","dissolve",Stage::Special,&cells);}}
 }
}
