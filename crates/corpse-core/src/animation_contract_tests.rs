use crate::{Actor,Run,Field,events::{self,EventPoint,Stage}};
fn room()->Run{let mut r:Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.simulation_revision=52.0.into();r.player.x=10.;r.player.y=10.;r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();r.player.conditions.insert("allSeeingReveal".into(),1.);r.player.conditions.insert("allSeeingDepth".into(),r.depth);r.actors.clear();r.inventory.clear();r.ground.clear();let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.lights.fill(100.);f.environment=Field::Missing;f.doors=Field::Missing;f.portals=Field::Missing;f.non_euclidean=Field::Missing;r}
#[test]fn animation_metadata_is_detached_ordered_and_revision_gated(){let mut r=room();let before=serde_json::to_vec(&r).unwrap();events::begin();for stage in [Stage::Travel,Stage::Impact,Stage::Special,Stage::Health,Stage::Death]{events::cue(&r,&r.player,"P01","collapse",stage,&[(&r.player).into()]);}let list=events::finish();assert_eq!(list.len(),5);assert_eq!(serde_json::to_vec(&r).unwrap(),before);assert!(list.iter().all(|e|e.animation.is_some()));r.simulation_revision=51.0.into();events::begin();events::cue(&r,&r.player,"P01","collapse",Stage::Death,&[(&r.player).into()]);assert!(events::finish().is_empty());}
#[test]fn actor_guard_never_reveals_hidden_or_other_plane(){let mut r=room();r.player.conditions.clear();let a=Actor{id:123.,x:30.,y:30.,hp:20.,max_hp:20.,..Default::default()};events::begin();events::cue(&r,&a,"E01","fold",Stage::Special,&[(&r.player).into()]);assert!(events::finish().is_empty());let p=EventPoint{x:-1.,y:0.};events::begin();events::at(&r,"A01","bolt",Stage::Travel,&[p]);assert!(events::finish().is_empty());}
#[test]fn exact_route_retains_portal_gaps_and_does_not_touch_rng(){let r=room();let before=serde_json::to_vec(&r).unwrap();let cells=vec![EventPoint{x:10.,y:10.},EventPoint{x:11.,y:10.},EventPoint{x:20.,y:20.}];events::begin();events::cue(&r,&r.player,"A01","bolt",Stage::Travel,&cells);let out=events::finish();assert_eq!(out[0].animation.as_ref().unwrap().cells,cells);assert_eq!(serde_json::to_vec(&r).unwrap(),before);}
#[test]fn amulet_direct_outcomes_do_not_create_a_poison_damage_packet(){let mut r=room();let item=crate::revision50_amulet_effects::make_amulet(crate::revision50_amulet_effects::Effect::Poison,900.).unwrap();r.inventory.push(item);r.actors_at_mut(r.depth).push(Actor{id:123.,x:13.,y:10.,hp:100.,max_hp:100.,..Default::default()});events::begin();crate::revision50_amulet_effects::command(&mut r,&crate::Command::UseAt{id:900.,x:13.,y:10.}).unwrap();let cues=events::finish();assert_eq!(crate::engine::actor(&r,123.).unwrap().hp,100.);assert!(cues.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="A03"&&a.stage==Stage::Travel)));assert!(cues.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="A03"&&a.stage==Stage::Special)));}

#[test]fn integrated_commands_have_identical_state_rng_with_and_without_animation_recording(){
 use crate::revision50_amulet_effects::{self as amulets,Effect};
 for effect in [Effect::Lightning,Effect::Firebolt,Effect::Poison,Effect::Iceblast,Effect::Acid,Effect::Blinking]{
  let mut recorded=room();recorded.next_id=2000.;recorded.player.energy=1000.;recorded.player.hp=1000.;recorded.player.max_hp=1000.;recorded.player.abilities.push("isheilah-spellslinger".into());recorded.inventory.push(amulets::make_amulet(effect,900.).unwrap());
  for (id,x,y)in [(123.,11.,10.),(124.,14.,10.)]{let mut a=crate::population::create_actor_revision("rat",id,x,y,false,52.).unwrap();a.hp=100.;a.max_hp=100.;recorded.actors_at_mut(recorded.depth).push(a);}
  let mut silent=recorded.clone();let (x,y)=if effect==Effect::Blinking{(12.,12.)}else{(14.,10.)};
  let commands=[crate::Command::UseAt{id:900.,x,y},crate::Command::Move{dx:1.,dy:0.},crate::Command::Wait,crate::Command::Move{dx:0.,dy:1.},crate::Command::Wait];
  let mut event_count=0;
  for command in commands {
   let a=crate::engine::dispatch(&mut recorded,&command).unwrap();
   let b=events::without_recording(||crate::engine::dispatch(&mut silent,&command)).unwrap();
   assert_eq!(recorded.rng,silent.rng,"{effect:?} {command:?}");
   assert_eq!(serde_json::to_value(&recorded).unwrap(),serde_json::to_value(&silent).unwrap(),"{effect:?} {command:?}");
   assert_eq!((a.took_turn,a.changed),(b.took_turn,b.changed));assert!(b.events.is_empty());event_count+=a.events.len();
  }
  assert!(event_count>0,"no exercised animation for {effect:?}");
 }
}

#[test]fn audible_shriek_contains_only_the_already_disclosed_sound_location(){
 let mut r=room();r.player.conditions.clear();let mut a=crate::population::create_actor_revision("banshee",123.,30.,25.,false,52.).unwrap();
 for (key,value) in [("monsterAudibleUntil",r.tick+1.),("monsterAudibleX",30.),("monsterAudibleY",25.)]{a.conditions.insert(key.into(),value);}
 events::begin();events::audible_banshee(&r,&a);let e=events::finish();assert_eq!(e.len(),1);assert!(e[0].from.is_none()&&e[0].glyph.is_none()&&e[0].label.is_none());assert_eq!(e[0].animation.as_ref().unwrap().surface.as_deref(),Some("audible"));assert_eq!(e[0].animation.as_ref().unwrap().cells,vec![EventPoint{x:30.,y:25.}]);
 a.conditions.shift_remove("monsterAudibleUntil");events::begin();events::audible_banshee(&r,&a);assert!(events::finish().is_empty());
}

#[test]fn actual_lightning_delivery_uses_portal_route_not_straight_gap(){
 use crate::revision50_amulet_effects::{self as a,Effect};
 let mut r=room();let w=r.floor().unwrap().width();r.floor_mut().unwrap().portals=vec![crate::portals_generation::PortalPair{a:10*w+12,b:10*w+30,color_index:0}].into();
 r.inventory.push(a::make_amulet(Effect::Lightning,900.).unwrap());events::begin();a::command(&mut r,&crate::Command::UseAt{id:900.,x:11.,y:10.}).unwrap();let cues=events::finish();
 let cells=&cues.iter().find_map(|e|e.animation.as_ref().filter(|c|c.proposal=="A01"&&c.stage==Stage::Travel)).unwrap().cells;
 assert!(cells.contains(&EventPoint{x:12.,y:10.})&&cells.contains(&EventPoint{x:30.,y:10.}));
 assert!(cells.iter().all(|p|p.y!=10.||p.x<=12.||p.x>=30.),"invented untraveled portal gap");
}
#[test]fn flood_wave_receipt_matches_actual_independently_stopped_lanes(){
 let mut r=room();let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[10*w+13]="water".into();
 let expected=crate::revision50_wave_teleport::preview(&r,false,15.,10.);assert!(expected.len()<15);
 events::begin();crate::revision50_wave_teleport::execute(&mut r,false,15.,10.).unwrap();let cues=events::finish();
 let flood:Vec<_>=cues.iter().filter_map(|e|e.animation.as_ref().filter(|c|c.proposal=="A12").map(|c|(e,c))).collect();
 assert_eq!(flood.len(),1,"only the cast animates");
 let (event,cue)=flood[0];assert_eq!(cue.stage,Stage::Travel);assert_eq!(cue.style,"nova-authored-short");
 assert_eq!(event.to,(&r.player).into());assert_eq!(event.color.as_deref(),Some("#42d8ea"));
 let cells=&cue.cells;
 assert_eq!(cells, &expected.iter().map(|&(x,y)|EventPoint{x:x as f64,y:y as f64}).collect::<Vec<_>>());
 assert!(!cells.contains(&EventPoint{x:13.,y:10.}));assert_eq!(r.floor().unwrap().tiles[10*w+13],"water");
}
#[test]fn wrong_plane_and_hallucinated_trait_sources_do_not_emit_identity_styles(){
 let mut r=room();let mut a=crate::population::create_actor_revision("rat",123.,11.,10.,false,52.).unwrap();a.conditions.insert("phantasmiiPlane".into(),1.);
 events::begin();events::cue(&r,&a,"E18","puncture",Stage::Special,&[(&a).into()]);assert!(events::finish().is_empty());
 a.conditions.clear();r.player.conditions.insert("hallucinating".into(),3.);events::begin();events::cue(&r,&a,"E18","puncture",Stage::Special,&[(&a).into()]);assert!(events::finish().is_empty());
}

fn rat(r:&mut Run,id:f64,x:f64,y:f64)->Actor {let mut a=crate::population::create_actor_revision("rat",id,x,y,false,52.).unwrap();a.hp=100.;a.max_hp=100.;a.energy=100.;a.conditions.clear();r.actors_at_mut(r.depth).push(a.clone());a}
#[test]fn animation_recipient_heal_requires_actual_visible_recipient(){
 let mut r=room();r.player.conditions.clear();let mut dryad=crate::population::create_actor_revision("dryad",123.,11.,10.,false,52.).unwrap();dryad.energy=100.;r.actors_at_mut(r.depth).push(dryad);
 let b=rat(&mut r,124.,12.,10.);let a=crate::engine::actor_mut_trusted(&mut r,b.id).unwrap();a.hp=20.;a.conditions.insert("invisible".into(),10.);
 events::begin();assert!(crate::monster_spells::heal_others(&mut r,123.).unwrap());let out=events::finish();assert!(crate::engine::actor(&r,124.).unwrap().hp>20.);// A visible caster's spell path is observable, but the invisible recipient's
 // identity and healing outcome are not.
 assert_eq!(r.log.last().map(String::as_str), Some("Dryad casts a healing spell."));
 let grove:Vec<_>=out.iter().filter(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="E17")).collect();
 assert!(!grove.is_empty());
 assert!(grove.iter().all(|e|{let cue=e.animation.as_ref().unwrap();cue.stage==Stage::Travel&&cue.style=="line-grow"&&e.label.is_none()&&e.glyph.is_none()&&cue.cells.iter().all(|p|crate::vision::directly_visible_tile(&r,p.x as usize,p.y as usize))}));
 crate::engine::actor_mut_trusted(&mut r,123.).unwrap().conditions.shift_remove("monsterHealReadyAt");crate::engine::actor_mut_trusted(&mut r,124.).unwrap().conditions.clear();
 events::begin();assert!(crate::monster_spells::heal_others(&mut r,123.).unwrap());assert!(events::finish().iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="E17"&&a.stage==Stage::Health)));
 assert_eq!(r.log.last().map(String::as_str), Some("Dryad healed the Giant Rat."));
 // An unobserved caster may not disclose its spell or recipient.
 crate::engine::actor_mut_trusted(&mut r,123.).unwrap().conditions.shift_remove("monsterHealReadyAt");
 crate::engine::actor_mut_trusted(&mut r,123.).unwrap().conditions.insert("invisible".into(),10.);
 crate::engine::actor_mut_trusted(&mut r,124.).unwrap().conditions.insert("invisible".into(),10.);
 let log = r.log.clone();
 events::begin();assert!(crate::monster_spells::heal_others(&mut r,123.).unwrap());
 assert!(!events::finish().iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="E17")));
 assert_eq!(r.log,log);
}
#[test]fn animation_bond_damage_pulses_only_observed_real_recipient(){
 for hidden in [false,true] {
  let mut r=room();r.player.conditions.clear();r.player.hp=100.;r.player.max_hp=100.;r.player.abilities=vec!["morae-bonded-by-unseen-threads".into()];r.player.active=r.player.abilities.clone();r.player.conditions.insert("bondedTarget".into(),123.);
  rat(&mut r,123.,11.,10.);if hidden{crate::engine::actor_mut_trusted(&mut r,123.).unwrap().conditions.insert("invisible".into(),10.);}
  let id=r.player.id;events::begin();crate::engine::damage_environment(&mut r,id,10.,None,"fire",true).unwrap();let out=events::finish();assert_eq!(crate::engine::actor(&r,123.).unwrap().hp,95.);
  assert_eq!(out.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="P68")),!hidden);
 }
}
#[test]fn animation_status_expiry_and_roots_final_tick_are_actual_outcomes(){
 let mut r=room();r.player.hp=100.;r.player.max_hp=100.;r.player.conditions.insert("poisoned".into(),1.);r.player.conditions.insert(crate::revision50_amulet_effects::POISON_DAMAGE.into(),6.);r.player.conditions.insert(crate::revision50_amulet_effects::ACID_TURNS.into(),1.);r.player.conditions.insert(crate::revision50_amulet_effects::ACID_PENALTY.into(),10.);
 let out=crate::engine::dispatch(&mut r,&crate::Command::Wait).unwrap();for key in ["A03","A05"]{assert!(out.events.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal==key)),"expiry {key}");}
 let mut r=room();let mut a=rat(&mut r,123.,11.,10.);for (key,value)in [(crate::revision50_control_amulets::ROOTS,1.),("r50RootsX",a.x),("r50RootsY",a.y),("r50RootsDepth",r.depth)]{a.conditions.insert(key.into(),value);}*crate::engine::actor_mut_trusted(&mut r,123.).unwrap()=a;
 events::begin();crate::revision50_control_amulets::tick_actor(&mut r,123.).unwrap();let out=events::finish();assert!(!out.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="A10"&&a.style=="root-tighten")));assert_eq!(crate::engine::actor(&r,123.).unwrap().hp,100.);assert!(out.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="A10"&&a.style=="dissolve-root")));assert!(!crate::engine::actor(&r,123.).unwrap().conditions.contains_key(crate::revision50_control_amulets::ROOTS));
 events::begin();crate::revision50_control_amulets::tick_actor(&mut r,123.).unwrap();assert!(events::finish().is_empty());
}
#[test]fn animation_actual_nova_blast_has_exact_footprint_without_legacy_hold(){
 let mut r=room();let w=r.floor().unwrap().width();r.floor_mut().unwrap().tiles[10*w+13]="wall".into();r.player.abilities.push("anavani-nova".into());r.player.active=r.player.abilities.clone();let source=r.player.clone();
 events::begin();crate::environment::explode_with_source_snapshot(&mut r,12.,10.,crate::environment::ExplosionOptions{direct_impact:true,source:Some(source.id),ignite_area:false,..Default::default()},None,&source).unwrap();let out=events::finish();assert!(out.iter().all(|e|e.animation.is_some()));let cells=&out.iter().find_map(|e|e.animation.as_ref().filter(|a|a.proposal=="P57")).unwrap().cells;
 assert!(!cells.contains(&EventPoint{x:14.,y:10.}));assert!(cells.iter().all(|p|(p.x-12.).hypot(p.y-10.)<=2.));assert!(cells.contains(&EventPoint{x:12.,y:8.}));
}
#[test]fn animation_nested_attack_ids_preserve_trigger_parent_and_second_bite(){
 let r=room();events::begin();events::next_action(&r);
 {let _first=events::attack_scope(&r);events::cue(&r,&r.player,"P41","slash",Stage::Impact,&[(&r.player).into()]);
  {let _second=events::action_scope(&r);events::cue(&r,&r.player,"E59","bracket",Stage::Travel,&[(&r.player).into()]);let _attack=events::attack_scope(&r);events::cue(&r,&r.player,"shared","slash",Stage::Impact,&[(&r.player).into()]);}
  events::cue(&r,&r.player,"P57","burst-ember",Stage::Special,&[(&r.player).into()]);}
 events::next_action(&r);events::cue(&r,&r.player,"E03","fan",Stage::Travel,&[(&r.player).into()]);let out=events::finish();let ids:Vec<_>=out.iter().map(|e|e.animation.as_ref().unwrap().action).collect();assert_eq!(ids[0],ids[3]);assert_eq!(ids[1],ids[2]);assert_ne!(ids[0],ids[1]);assert!(ids[4]>ids[2]);
}
#[test]fn animation_landed_lightning_rescue_is_not_mislabeled_rejection(){
 let mut r=room();let mut b=rat(&mut r,123.,11.,10.);b.hp=1.;b.abilities=vec![crate::divine_rescues::PELIAN.into()];b.active=b.abilities.clone();*crate::engine::actor_mut_trusted(&mut r,b.id).unwrap()=b.clone();let source=r.player.clone();
 events::begin();let packets=events::damage_count(b.id);crate::engine::damage_direct_with_source_snapshot(&mut r,b.id,50.,&source,"lightning",true).unwrap();crate::animation_amulets::outcome(&r,&b,crate::revision50_amulet_effects::Effect::Lightning,packets);let out=events::finish();assert_eq!(crate::engine::actor(&r,b.id).unwrap().hp,100.);assert!(out.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="P07")));assert!(!out.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="A01"&&a.style=="rejected")));
}
#[test]fn animation_real_consecutive_waits_do_not_replay_yeti_chill(){
 let mut r=room();r.next_id=2000.;r.player.hp=1000.;r.player.max_hp=1000.;let mut yeti=crate::population::create_actor_revision("yeti",123.,11.,10.,false,52.).unwrap();yeti.conditions.insert("paralyzed".into(),100.);r.actors_at_mut(r.depth).push(yeti);
 let first=crate::engine::dispatch(&mut r,&crate::Command::Wait).unwrap();assert!(first.events.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="E14")));
 let second=crate::engine::dispatch(&mut r,&crate::Command::Wait).unwrap();assert!(!second.events.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="E14")));
}
#[test]fn animation_real_free_toggle_keeps_command_action_without_enemy_turn(){
 let mut r=room();r.next_id=2000.;r.player.abilities=vec!["numo-mind-read".into()];r.player.energy=1000.;r.player.hp=1000.;r.player.max_hp=1000.;rat(&mut r,123.,11.,10.);
 let command=crate::Command::Ability{id:"numo-mind-read".into(),target:Field::Missing,x:Field::Missing,y:Field::Missing};
 // An R52 owned paid ability requires its authored reserve, not merely actor energy.
 let mut unfunded=r.clone();let rejected=crate::engine::dispatch(&mut unfunded,&command).unwrap();assert!(!rejected.took_turn);assert!(!unfunded.player.active.iter().any(|s|s=="numo-mind-read"));assert!(!rejected.events.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="P21")));
 crate::revision50_ability_energy::initialize_reserve(&mut r.player,"numo-mind-read",40.,40.).unwrap();
 let tick=r.tick;let out=crate::engine::dispatch(&mut r,&command).unwrap();assert!(!out.took_turn);assert_eq!(r.tick,tick);assert!(r.player.active.iter().any(|s|s=="numo-mind-read"));assert!(out.events.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="P21"&&a.action==0)));assert!(!out.events.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.action>0)));
}

#[test]fn animation_paid_command_precedes_reacting_enemy_actions(){
 let mut r=room();r.next_id=2000.;r.player.hp=1000.;r.player.max_hp=1000.;r.player.energy=1000.;rat(&mut r,123.,11.,10.);rat(&mut r,124.,10.,11.);
 // The unstruck, already-aware enemy can respond; surprise correctly suppresses newly alerted ones.
 crate::engine::actor_mut(&mut r,124.).unwrap().state="hunting".into();
 r.inventory.push(crate::revision50_amulet_effects::make_amulet(crate::revision50_amulet_effects::Effect::Lightning,900.).unwrap());
 let tick=r.tick;let out=crate::engine::dispatch(&mut r,&crate::Command::UseAt{id:900.,x:11.,y:10.}).unwrap();assert!(out.took_turn);assert!(r.tick>tick);let player_ids:Vec<_>=out.events.iter().filter_map(|e|e.animation.as_ref()).filter(|a|a.proposal=="A01").map(|a|a.action).collect();assert!(!player_ids.is_empty());assert!(player_ids.iter().all(|a|*a==player_ids[0]));assert!(out.events.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal=="shared"&&a.action>player_ids[0])),"fixture requires a genuinely reacting enemy after player {player_ids:?}");
}

#[test]fn brogue_unseen_ranged_source_keeps_observed_impact_without_a_hidden_route(){
 let mut r=room();r.player.conditions.clear();let mut source=crate::population::create_actor_revision("rat",123.,30.,20.,false,52.).unwrap();source.conditions.insert("invisible".into(),10.);
 let before=serde_json::to_vec(&r).unwrap();events::begin();events::strike_ranged(&r,&source,&r.player,true,5.,20.);let events=events::finish();assert_eq!(events.len(),2);assert!(events.iter().all(|e|e.from.is_none()&&e.animation.as_ref().is_some_and(|a|a.proposal=="shared-ranged"&&a.cells==vec![(&r.player).into()])));assert_eq!(serde_json::to_vec(&r).unwrap(),before);
}

#[test]fn animation_magic_and_physical_delivery_are_explicit_and_keep_item_glyphs(){
 use events::Delivery;
 let r=room();let points=[crate::movement::Point{x:10,y:10},crate::movement::Point{x:11,y:10},crate::movement::Point{x:13,y:10}];
 for (label,glyph,delivery,expected) in [("Arrow",Some("↑"),Delivery::Projectile,Some("↑")),("Thrown dagger",Some("/"),Delivery::Projectile,Some("/")),("Potion of Healing",Some("!"),Delivery::Projectile,Some("!")),("Spell dissipates",None,Delivery::Magic,None)] {
  events::begin();events::committed_path(&r,&r.player,&points);events::actor_event(&r,&r.player,"projectile",label,Some(EventPoint{x:13.,y:10.}),glyph);
  events::projectile_item(&r,900.);
  let out=events::finish();let e=&out[0];assert_eq!(e.animation.as_ref().unwrap().delivery,delivery);assert_eq!(e.glyph.as_deref(),expected);assert_eq!(e.animation.as_ref().unwrap().item_id,if delivery==Delivery::Projectile{Some(900.)}else{None});
 }
}
#[test]fn animation_actual_ally_and_enemy_spells_are_lit_routes(){
 use events::Delivery;
 for ally in [false,true] {for kind in ["witch","hemogoblin-bloodmancer"] {
  let mut r=room();let source=crate::population::create_actor_revision(kind,123.,11.,10.,ally,52.).unwrap();
  let mut target=crate::population::create_actor_revision("rat",124.,15.,10.,!ally,52.).unwrap();target.hp=1000.;target.max_hp=1000.;
  r.actors_at_mut(r.depth).extend([source,target]);events::begin();
  assert!(crate::monster_spells::special_action(&mut r,123.,124.).unwrap(),"{kind} ally={ally}");
  let out=events::finish();let routes:Vec<_>=out.iter().filter_map(|e|e.animation.as_ref().filter(|a|a.stage==Stage::Travel)).collect();
  assert!(!routes.is_empty(),"missing {kind} ally={ally}");assert!(routes.iter().all(|a|a.delivery==Delivery::Magic));
 }}
}
#[test]fn animation_all_bolt_amulets_light_tiles_and_fire_has_no_generic_spark(){
 use crate::revision50_amulet_effects::{self as amulets,Effect};
 for effect in [Effect::Lightning,Effect::Firebolt,Effect::Poison]{
  let mut r=room();r.player.abilities.push("isheilah-spellslinger".into());r.player.energy=1000.;r.inventory.push(amulets::make_amulet(effect,900.).unwrap());rat(&mut r,123.,14.,10.);
  events::begin();amulets::command(&mut r,&crate::Command::UseAt{id:900.,x:14.,y:10.}).unwrap();let out=events::finish();
  let key=crate::animation_amulets::key(effect);let route=out.iter().find(|e|e.animation.as_ref().is_some_and(|a|a.proposal==key&&a.stage==Stage::Travel)).unwrap();
  assert_eq!(route.animation.as_ref().unwrap().delivery,events::Delivery::Magic);assert!(route.glyph.is_none());
  if effect==Effect::Firebolt{assert!(!out.iter().any(|e|e.animation.as_ref().is_some_and(|a|a.proposal==key&&a.style=="spark")));}
 }
}
