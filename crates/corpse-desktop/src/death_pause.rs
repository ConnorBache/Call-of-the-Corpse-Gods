//! Presentation/input gate only: the terminal run remains available for archiving.
use sdl2::{event::Event,keyboard::Keycode};
#[derive(Default)]
pub struct DeathPause { active:bool, presented:bool, space_down:bool }
#[derive(Debug,PartialEq)] pub enum Decision {Pass,Block,Advance}
impl DeathPause {
 pub fn observe(&mut self,dead:bool){if dead!=self.active{self.active=dead;self.presented=false;}}
 pub fn presented(&mut self,dead:bool){self.observe(dead);if dead{self.presented=true;}}
 pub fn event(&mut self,dead:bool,event:&Event)->Decision {
  self.observe(dead);
  let fresh=matches!(event,Event::KeyDown{keycode:Some(Keycode::Space),repeat:false,..})&&!self.space_down;
  match event {Event::KeyDown{keycode:Some(Keycode::Space),..}=>self.space_down=true,Event::KeyUp{keycode:Some(Keycode::Space),..}=>self.space_down=false,_=>{}}
  if !dead{return Decision::Pass;}
  if fresh&&self.presented{return Decision::Advance;}
  if matches!(event,Event::Quit{..}|Event::Window{..}|Event::RenderTargetsReset{..}|Event::RenderDeviceReset{..}){Decision::Pass}else{Decision::Block}
 }
}
#[cfg(test)]mod tests {
 use super::*;use sdl2::keyboard::Mod;
 fn key(k:Keycode,repeat:bool)->Event{Event::KeyDown{timestamp:0,window_id:0,keycode:Some(k),scancode:None,keymod:Mod::NOMOD,repeat}}
 fn release()->Event{Event::KeyUp{timestamp:0,window_id:0,keycode:Some(Keycode::Space),scancode:None,keymod:Mod::NOMOD,repeat:false}}
 #[test]fn only_fresh_space_after_death_frame_advances(){let mut g=DeathPause::default();assert_eq!(g.event(false,&key(Keycode::Space,false)),Decision::Pass);g.presented(true);assert_eq!(g.event(true,&key(Keycode::Space,true)),Decision::Block);assert_eq!(g.event(true,&key(Keycode::Space,false)),Decision::Block);assert_eq!(g.event(true,&release()),Decision::Block);assert_eq!(g.event(true,&key(Keycode::Space,false)),Decision::Advance);}
 #[test]fn queued_space_cannot_skip_unseen_death_frame(){let mut g=DeathPause::default();assert_eq!(g.event(true,&key(Keycode::Space,false)),Decision::Block);g.presented(true);assert_eq!(g.event(true,&key(Keycode::Space,true)),Decision::Block);g.event(true,&release());assert_eq!(g.event(true,&key(Keycode::Space,false)),Decision::Advance);}
 #[test]fn incidental_inputs_block_but_quit_and_new_run_work(){let mut g=DeathPause::default();g.presented(true);for k in [Keycode::Return,Keycode::Escape,Keycode::I,Keycode::M,Keycode::Kp2]{assert_eq!(g.event(true,&key(k,false)),Decision::Block);}assert_eq!(g.event(true,&Event::MouseButtonDown{timestamp:0,window_id:0,which:0,mouse_btn:sdl2::mouse::MouseButton::Left,clicks:1,x:10,y:10}),Decision::Block);assert_eq!(g.event(true,&Event::Quit{timestamp:0}),Decision::Pass);assert_eq!(g.event(false,&key(Keycode::Return,false)),Decision::Pass);}
}
