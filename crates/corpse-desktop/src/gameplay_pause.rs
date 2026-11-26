//! Desktop-only acknowledgments. No queued command or wait loop is serialized.
use corpse_core::{Command,Run};
use sdl2::{event::Event,keyboard::Keycode};
use std::rc::Rc;
pub const CONTINUE:&str="Press space to continue";
pub const DEATH:&str="You died.";
pub const PARALYZED:&str="You are paralyzed. Press space to wait until un-paralyzed";
#[derive(Default)]pub struct Pause {
 pub enabled:bool,
 pending:Option<Command>,
 confirm_jump:bool,
 pub outcome:Option<Rc<crate::render::View>>,
 paralysis:bool,
 pub waiting:bool,
 presented:bool,
 space_down:bool,
 bypass:bool,
}
pub enum Decision {Pass,Block,Continue(Option<Command>)}
impl Pause {
 pub fn fall_active(&self)->bool{self.pending.is_some()||self.outcome.is_some()}
 pub fn active(&self)->bool{self.pending.is_some()||self.outcome.is_some()||self.paralysis&&!self.waiting}
 pub fn text(&self)->&'static str {if self.pending.is_some()&&self.confirm_jump{crate::hazards_ui::CHASM_QUESTION}else if self.pending.is_some()||self.outcome.is_some(){CONTINUE}else{PARALYZED}}
 pub fn reset(&mut self){let enabled=self.enabled;*self=Self::default();self.enabled=enabled;}
 pub fn presented(&mut self){if self.active(){self.presented=true;}}
 pub fn observe(&mut self,r:Option<&Run>){
  let paralyzed=r.is_some_and(|r|r.status=="playing"&&r.offers.is_empty()&&corpse_core::engine::condition(&r.player,"paralyzed")>0.);
  if paralyzed!=self.paralysis {self.paralysis=paralyzed;self.presented=false;if !paralyzed{self.waiting=false;}}
  if r.is_none(){self.reset();}
 }
 pub fn stop_waiting(&mut self){self.waiting=false;self.presented=false;}
 pub fn defer(&mut self,c:&Command){self.pending=Some(c.clone());self.confirm_jump=false;self.presented=false;self.waiting=false;}
 pub fn defer_jump(&mut self,c:&Command){self.defer(c);self.confirm_jump=true;}
 pub fn hold_outcome(&mut self,view:crate::render::View){self.outcome=Some(Rc::new(view));self.presented=false;self.waiting=false;}
 pub fn take_bypass(&mut self)->bool{std::mem::take(&mut self.bypass)}
 pub fn release(&mut self,e:&Event){if matches!(e,Event::KeyUp{keycode:Some(Keycode::Space),..}){self.space_down=false;}}
 pub fn event(&mut self,e:&Event)->Decision{
  let fresh=matches!(e,Event::KeyDown{keycode:Some(Keycode::Space),repeat:false,..})&&!self.space_down;
  match e {Event::KeyDown{keycode:Some(Keycode::Space),..}=>self.space_down=true,Event::KeyUp{keycode:Some(Keycode::Space),..}=>self.space_down=false,_=>{}}
  if self.waiting && matches!(e,Event::KeyDown{repeat:false,..}|Event::MouseButtonDown{..}) {self.stop_waiting();return Decision::Block;}
  if !self.active(){return Decision::Pass;}
  if self.pending.is_some()&&self.confirm_jump&&matches!(e,Event::KeyDown{keycode:Some(Keycode::Escape|Keycode::N),repeat:false,..}) {
   self.pending=None;self.confirm_jump=false;self.presented=false;return Decision::Continue(None);
  }
  if fresh&&self.presented {
   self.presented=false;
   if let Some(c)=self.pending.take(){self.confirm_jump=false;self.bypass=true;return Decision::Continue(Some(c));}
   if self.outcome.take().is_some(){return Decision::Continue(None);}
   self.waiting=true;return Decision::Continue(None);
  }
  if matches!(e,Event::Quit{..}|Event::Window{..}|Event::RenderTargetsReset{..}|Event::RenderDeviceReset{..}){Decision::Pass}else{Decision::Block}
 }
}
pub fn needs_ack(r:&Run,c:&Command)->bool {
 if r.status!="playing"||!r.offers.is_empty(){return false;}
 let Some(f)=r.floor()else{return false;};
 let tile=|x:f64,y:f64|corpse_core::portals::cell(f,x,y).and_then(|i|f.tiles.get(i));
 let current=tile(r.player.x,r.player.y).is_some_and(|t|t=="chasm");
 if r.simulation_revision.value_or(29.)>=52. {
  return match c {
   Command::Move{..}=>crate::hazards_ui::intentional(r,c).as_deref()==Some(crate::hazards_ui::CHASM_QUESTION),
   Command::Descend=>current&&!corpse_core::dreamwalking::active(r)
    && ["paralyzed","deathResolutionBlocked"].iter().all(|k|corpse_core::engine::condition(&r.player,k)==0.),
   _=>false,
  };
 }
 match c {
  Command::Move{dx,dy}=>!corpse_core::travel::flying(r,&r.player)&&(
   tile(r.player.x+dx,r.player.y+dy).is_some_and(|t|t=="chasm")&&
   corpse_core::vision::visible(r,(r.player.x+dx)as usize,(r.player.y+dy)as usize)
   ||crate::hazards_ui::intentional(r,c).is_some_and(|s|s.to_ascii_lowercase().contains("chasm"))),
  Command::Descend=>current,
  Command::Ability{id,..}=>id=="anavani-ascend"&&current,
  _=>false,
 }
}
pub fn paint(p:&mut crate::render::Painter,text:&str){p.centered_sheet(|p|{
 p.rect(280,320,720,110,[12,10,20]);p.border(280,320,720,110,[255,217,53]);
 if text==PARALYZED {p.text("You are paralyzed.",304,340,20,[255,248,232]);p.text("Press space to wait until un-paralyzed",304,378,18,[255,248,232]);}
 else if text==DEATH {p.text(DEATH,304,340,20,[255,248,232]);p.text(CONTINUE,304,378,18,[255,248,232]);}
 else if text==crate::hazards_ui::CHASM_QUESTION {p.text(text,304,340,20,[255,248,232]);p.text("Space to jump · Esc to cancel",304,378,18,[255,248,232]);}
 else {p.text(text,304,359,20,[255,248,232]);}
});}
#[cfg(test)]mod tests{
 use super::*;use sdl2::keyboard::Mod;
 fn key(k:Keycode,repeat:bool)->Event{Event::KeyDown{timestamp:0,window_id:0,keycode:Some(k),scancode:None,keymod:Mod::NOMOD,repeat}}
 fn release()->Event{Event::KeyUp{timestamp:0,window_id:0,keycode:Some(Keycode::Space),scancode:None,keymod:Mod::NOMOD,repeat:false}}
 fn room()->Run{serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap()}
 #[test]fn queued_held_and_escape_do_not_acknowledge(){let mut p=Pause::default();p.defer(&Command::Descend);assert!(matches!(p.event(&key(Keycode::Space,false)),Decision::Block));p.presented();assert!(matches!(p.event(&key(Keycode::Escape,false)),Decision::Block));assert!(matches!(p.event(&key(Keycode::Space,true)),Decision::Block));p.event(&release());assert!(matches!(p.event(&key(Keycode::Space,false)),Decision::Continue(Some(Command::Descend))));assert!(p.take_bypass());assert!(!p.take_bypass());}
 #[test]fn paralysis_wait_ends_and_can_stop_for_danger(){let mut r=room();r.player.conditions.insert("paralyzed".into(),3.);let mut p=Pause::default();p.observe(Some(&r));assert_eq!(p.text(),PARALYZED);p.presented();assert!(matches!(p.event(&key(Keycode::Space,false)),Decision::Continue(None)));assert!(p.waiting);p.stop_waiting();assert!(p.active());r.player.conditions.remove("paralyzed");p.observe(Some(&r));assert!(!p.active());assert!(!p.waiting);}
 #[test]fn chasm_and_ascend_are_preflight_without_mutating_run(){let mut r=room();r.player.x=2.;r.player.y=2.;let f=r.floor_mut().unwrap();let w=f.width();f.tiles[2*w+3]="chasm".into();f.tiles[2*w+2]="chasm".into();f.lights.fill(100.);let before=r.clone();assert!(needs_ack(&r,&Command::Move{dx:1.,dy:0.}));assert!(needs_ack(&r,&Command::Descend));assert!(needs_ack(&r,&Command::Ability{id:"anavani-ascend".into(),target:corpse_core::Field::Missing,x:corpse_core::Field::Missing,y:corpse_core::Field::Missing}));assert_eq!(r,before);}
 #[test]fn dispatch_queues_before_chasm_changes_and_only_fresh_space_releases(){
  let mut r=room();r.player.x=2.;r.player.y=2.;let f=r.floor_mut().unwrap();let w=f.width();f.tiles[2*w+3]="chasm".into();f.lights.fill(100.);
  let mut e=crate::temporal::Engine::new(r).unwrap();let before=e.run.clone();let mut fx=crate::effects::Effects::default();fx.gameplay_pause.enabled=true;
  let out=crate::dispatch(&mut e,&mut fx,0.,&Command::Move{dx:1.,dy:0.}).unwrap();assert!(!out.took_turn);assert_eq!(e.run,before);assert!(fx.gameplay_pause.active());
  assert!(matches!(fx.gameplay_pause.event(&key(Keycode::Escape,false)),Decision::Block));fx.gameplay_pause.presented();assert!(matches!(fx.gameplay_pause.event(&key(Keycode::Space,false)),Decision::Continue(Some(Command::Move{..}))));
 }
 #[test]fn actual_waits_remove_paralysis_and_do_not_continue_afterward(){
  let mut r=room();r.player.conditions.clear();r.player.conditions.insert("paralyzed".into(),2.);r.player.abilities.clear();r.player.active.clear();r.actors_at_mut(r.depth).clear();
  let f=r.floor_mut().unwrap();f.tiles.fill("floor".into());f.environment=corpse_core::Field::Missing;
  let mut e=crate::temporal::Engine::new(r).unwrap();let mut fx=crate::effects::Effects::default();fx.gameplay_pause.enabled=true;fx.gameplay_pause.observe(Some(&e.run));fx.gameplay_pause.presented();fx.gameplay_pause.event(&key(Keycode::Space,false));
  let mut count=0;while fx.gameplay_pause.waiting&&count<10{crate::dispatch(&mut e,&mut fx,count as f64*1000.,&Command::Wait).unwrap();fx.gameplay_pause.observe(Some(&e.run));count+=1;}
  assert!(count>0&&count<10);assert_eq!(corpse_core::engine::condition(&e.run.player,"paralyzed"),0.);assert!(!fx.gameplay_pause.waiting);
 }

 #[test]fn paralysis_wait_is_not_rest_until_change(){let mut r=room();r.player.conditions.insert("paralyzed".into(),3.);let mut p=Pause::default();p.observe(Some(&r));p.presented();p.event(&key(Keycode::Space,false));r.player.hp-=1.;r.actors_at_mut(r.depth)[0].state="hunting".into();p.observe(Some(&r));assert!(p.waiting,"damage/new threat alone must not cancel the requested paralysis wait");r.status="dead".into();p.observe(Some(&r));assert!(!p.waiting);}

 fn stairs_room()->Run {
  let mut r=room();r.inventory.clear();
  for depth in r.floors.iter().map(|f|f.depth).collect::<Vec<_>>() {r.actors_at_mut(depth).clear();}
  r.player.abilities.clear();r.player.active.clear();r.player.conditions.clear();
  r.player.x=2.;r.player.y=2.;
  for floor in &mut r.floors {
   let f=std::sync::Arc::make_mut(floor);f.tiles.fill("floor".into());f.lights.fill(100.);
   f.environment=corpse_core::Field::Missing;f.portals=corpse_core::Field::Missing;
   f.doors=corpse_core::Field::Missing;f.vents=corpse_core::Field::Missing;
   let w=f.width();f.tiles[2*w+2]="up".into();
  }
  r
 }
 #[test]
 fn current_chasm_prompt_requires_a_voluntary_jump_and_can_cancel() {
  let mut r=stairs_room();r.simulation_revision=52.0.into();
  let w=r.floor().unwrap().width();
  r.floor_mut().unwrap().tiles[2*w+3]="chasm".into();
  let step=Command::Move{dx:1.,dy:0.};
  let f=std::sync::Arc::make_mut(&mut r.floors[1]);
  f.tiles[2*w+3]="water".into();f.seen[2*w+3]=true;
  assert!(needs_ack(&r,&step),"a known soft landing is still a jump");
  r.player.conditions.insert("flying".into(),2.);
  assert!(!needs_ack(&r,&step));
  r.player.conditions.shift_remove("flying");
  r.player.conditions.insert("paralyzed".into(),2.);
  assert!(!needs_ack(&r,&step));
  r.player.conditions.shift_remove("paralyzed");
  let mut enemy=corpse_core::population::create_actor_revision("rat",80.,3.,2.,false,52.).unwrap();
  enemy.conditions.insert("flying".into(),2.);
  r.actors_at_mut(r.depth).push(enemy);
  assert!(!needs_ack(&r,&step),"a bump-attack isn't a jump");
  r.actors_at_mut(r.depth).clear();
  let mut e=crate::temporal::Engine::new(r).unwrap();
  let mut fx=crate::effects::Effects::default();fx.gameplay_pause.enabled=true;
  let before=e.run.clone();
  crate::dispatch(&mut e,&mut fx,0.,&step).unwrap();
  assert_eq!(e.run,before);
  assert_eq!(fx.gameplay_pause.text(),"Are you sure you want to jump into the chasm?");
  fx.gameplay_pause.presented();
  assert!(matches!(fx.gameplay_pause.event(&key(Keycode::Escape,false)),Decision::Continue(None)));
  assert!(!fx.gameplay_pause.active());
  assert_eq!(e.run,before);
  crate::dispatch(&mut e,&mut fx,1.,&step).unwrap();fx.gameplay_pause.presented();
  assert!(matches!(fx.gameplay_pause.event(&key(Keycode::Space,false)),Decision::Continue(Some(Command::Move{..}))));
  crate::dispatch(&mut e,&mut fx,2.,&step).unwrap();
  assert_eq!(e.run.depth,2.);
  assert!(!fx.gameplay_pause.active());
 }
 #[test]
 fn current_involuntary_chasm_fall_never_asks_for_space() {
  let mut r=stairs_room();r.simulation_revision=52.0.into();
  let f=r.floor_mut().unwrap();let w=f.width();f.tiles[2*w+2]="chasm".into();
  r.player.conditions.insert("flying".into(),1.);
  let mut e=crate::temporal::Engine::new(r).unwrap();
  let mut fx=crate::effects::Effects::default();fx.gameplay_pause.enabled=true;
  crate::dispatch(&mut e,&mut fx,0.,&Command::Wait).unwrap();
  assert_eq!(e.run.depth,2.);
  assert!(!fx.gameplay_pause.fall_active());
 }
 #[test]fn stairs_pause_direct_keyboard_or_button_descent_needs_no_space(){
  let mut r=stairs_room();let f=r.floor_mut().unwrap();let w=f.width();f.tiles[2*w+2]="down".into();
  let mut e=crate::temporal::Engine::new(r).unwrap();let mut fx=crate::effects::Effects::default();fx.gameplay_pause.enabled=true;
  assert!(!needs_ack(&e.run,&Command::Descend));
  crate::dispatch(&mut e,&mut fx,0.,&Command::Descend).unwrap();
  assert_eq!(e.run.depth,2.);assert!(!fx.gameplay_pause.active());assert!(fx.gameplay_pause.outcome.is_none());
 }
 #[test]fn stairs_pause_move_keyboard_click_or_path_auto_descent_needs_no_space(){
  // Keyboard steps and click/path travel share this Move dispatch boundary.
  let mut r=stairs_room();let f=r.floor_mut().unwrap();let w=f.width();f.tiles[2*w+3]="down".into();
  let mut e=crate::temporal::Engine::new(r).unwrap();let mut fx=crate::effects::Effects::default();fx.gameplay_pause.enabled=true;
  crate::dispatch(&mut e,&mut fx,0.,&Command::Move{dx:1.,dy:0.}).unwrap();
  assert_eq!(e.run.depth,2.);assert!(!fx.gameplay_pause.active());assert!(fx.gameplay_pause.outcome.is_none());
 }
 #[test]fn stairs_pause_forced_flight_expiry_fall_still_holds_even_water_landing(){
  let mut r=stairs_room();let f=r.floor_mut().unwrap();let w=f.width();f.tiles[2*w+2]="chasm".into();
  r.player.conditions.insert("flying".into(),1.);
  let f=std::sync::Arc::make_mut(&mut r.floors[1]);f.tiles[2*w+2]="water".into();
  let mut e=crate::temporal::Engine::new(r).unwrap();let mut fx=crate::effects::Effects::default();fx.gameplay_pause.enabled=true;
  assert!(!needs_ack(&e.run,&Command::Wait));crate::dispatch(&mut e,&mut fx,0.,&Command::Wait).unwrap();
  assert_eq!(e.run.depth,2.);assert!(fx.gameplay_pause.fall_active());assert_eq!(fx.gameplay_pause.text(),CONTINUE);
 }
 #[test]fn stairs_pause_acknowledged_voluntary_fall_does_not_ask_twice(){
  let mut r=stairs_room();let f=r.floor_mut().unwrap();let w=f.width();f.tiles[2*w+3]="chasm".into();
  let mut e=crate::temporal::Engine::new(r).unwrap();let mut fx=crate::effects::Effects::default();fx.gameplay_pause.enabled=true;
  let command=Command::Move{dx:1.,dy:0.};crate::dispatch(&mut e,&mut fx,0.,&command).unwrap();
  assert_eq!(e.run.depth,1.);assert!(fx.gameplay_pause.active());fx.gameplay_pause.presented();
  assert!(matches!(fx.gameplay_pause.event(&key(Keycode::Space,false)),Decision::Continue(Some(_))));
  crate::dispatch(&mut e,&mut fx,1.,&command).unwrap();assert_eq!(e.run.depth,2.);assert!(!fx.gameplay_pause.active());
 }

 #[test]fn stairs_pause_click_and_generated_path_reach_downstairs_without_ack(){
  for x in [3,6] {
   let mut r=stairs_room();let f=r.floor_mut().unwrap();let w=f.width();f.tiles[2*w+x]="down".into();f.seen.fill(true);
   f.remembered_tiles=f.tiles.clone().into();
   let mut e=crate::temporal::Engine::new(r).unwrap();let mut fx=crate::effects::Effects::default();fx.gameplay_pause.enabled=true;
   match crate::travel_ui::start(&e.run,x,2) {
    crate::travel_ui::Start::Direct(command)=>{crate::dispatch(&mut e,&mut fx,0.,&command).unwrap();},
    crate::travel_ui::Start::Walk(mut path)=>{
     for step in 0..10 {
      let Some(command)=path.next_command(&e.run).unwrap()else{break};
      crate::dispatch(&mut e,&mut fx,step as f64*1000.,&command).unwrap();
      if !path.after_step(&e.run,false).unwrap(){break;}
     }
    },
    crate::travel_ui::Start::NoPath=>panic!("known clear staircase must be reachable"),
   }
   assert_eq!(e.run.depth,2.);assert!(!fx.gameplay_pause.active());
  }
 }

}
