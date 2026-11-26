//! Presentation/input gate only: the terminal run remains available for archiving.
use sdl2::{event::Event,keyboard::Keycode};
pub fn terminal(status: &str) -> bool { matches!(status, "dead" | "won") }
const END_FADE_MS: f64 = 960.;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Ending { Death, Escape }
impl Ending {
    fn from_status(status: &str) -> Option<Self> {
        match status { "dead" => Some(Self::Death), "won" => Some(Self::Escape), _ => None }
    }
    fn color(self) -> [u8; 3] {
        match self { Self::Death => [0; 3], Self::Escape => [255; 3] }
    }
}

/// Presentation time only. Start once the last gameplay effect is on screen,
/// and gate the existing continuation prompt until the full fade is presented.
#[derive(Default)]
pub struct Fade { started: Option<(Ending, f64)> }
impl Fade {
    pub fn sample(&mut self, status: &str, now: f64, skip_motion: bool) -> Option<([u8; 3], u8)> {
        let Some(ending) = Ending::from_status(status).filter(|_| !skip_motion) else {
            self.started = None;
            return None;
        };
        if !self.started.is_some_and(|(current, _)| current == ending) {
            self.started = Some((ending, now));
        }
        let elapsed = (now - self.started.unwrap().1) / END_FADE_MS;
        let t = elapsed.clamp(0., 1.);
        // Smoothstep with a guaranteed fully opaque last frame.
        let alpha = if t == 1. { 255 } else { (255. * t * t * (3. - 2. * t)).floor() as u8 };
        Some((ending.color(), alpha))
    }
}
#[derive(Default)]
pub struct DeathPause { active:bool, presented:bool, space_down:bool }
#[derive(Debug,PartialEq)] pub enum Decision {Pass,Block,Advance}
impl DeathPause {
 pub fn event_for_status(&mut self,status:&str,event:&Event)->Decision {
  let decision=self.event(terminal(status),event);
  // Preserve victory's existing menu/movement/click controls, but never let a
  // queued input skip the first terminal frame or bypass Space freshness.
  if status=="won" && self.presented && decision==Decision::Block
   && !matches!(event,Event::KeyDown{keycode:Some(Keycode::Space),..}|Event::KeyUp{keycode:Some(Keycode::Space),..}) {Decision::Pass}else{decision}
 }
 pub fn observe(&mut self,dead:bool){if dead!=self.active{self.active=dead;self.presented=false;}}
 pub fn is_presented(&self)->bool{self.presented}
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
 #[test] fn terminal_fades_reach_their_colors_once_and_reduced_motion_skips(){
  let mut fade=Fade::default();
  assert_eq!(fade.sample("playing",10.,false),None);
  for (status,color) in [("dead",[0;3]),("won",[255;3])] {
   assert_eq!(fade.sample(status,100.,false),Some((color,0)));
   assert_eq!(fade.sample(status,580.,false),Some((color,127)));
   assert_eq!(fade.sample(status,1059.,false),Some((color,254)));
   assert_eq!(fade.sample(status,1060.,false),Some((color,255)));
   assert_eq!(fade.sample(status,1500.,false),Some((color,255)));
   assert_eq!(fade.sample(status,1500.,true),None);
   assert_eq!(fade.sample("playing",1600.,false),None);
  }
 }
 #[test] fn the_ending_tint_covers_the_drawable_even_with_clips(){
  let mut p=crate::render::Painter::new();
  p.clip=Some((2,2,4,4));
  p.ending_fade([255;3],255);
  assert!(p.pixels.iter().all(|v|*v==255));
  assert_eq!(p.clip,Some((2,2,4,4)));
  p.ending_fade([0;3],255);
  assert!(p.pixels.chunks_exact(4).all(|pixel|pixel==[0,0,0,255]));
 }
 #[test] fn victory_preserves_prior_controls_after_presentation(){
  let mut g=DeathPause::default();assert_eq!(g.event_for_status("won",&key(Keycode::Kp2,false)),Decision::Block);
  g.presented(true);assert_eq!(g.event_for_status("won",&key(Keycode::Kp2,false)),Decision::Pass);
  let click=Event::MouseButtonDown{timestamp:0,window_id:0,which:0,mouse_btn:sdl2::mouse::MouseButton::Left,clicks:1,x:10,y:10};
  assert_eq!(g.event_for_status("won",&click),Decision::Pass);
  assert_eq!(g.event_for_status("won",&key(Keycode::Space,true)),Decision::Block);
  g.event_for_status("won",&release());assert_eq!(g.event_for_status("won",&key(Keycode::Space,false)),Decision::Advance);
 }
 #[test] fn victory_uses_same_fresh_space_gate_as_death(){
  for status in ["won","dead"] {
   assert!(terminal(status));let mut g=DeathPause::default();
   assert_eq!(g.event(terminal(status),&key(Keycode::Space,false)),Decision::Block);
   g.presented(terminal(status));
   assert_eq!(g.event(terminal(status),&key(Keycode::Space,true)),Decision::Block);
   assert_eq!(g.event(terminal(status),&key(Keycode::Kp2,false)),Decision::Block);
   g.event(terminal(status),&release());
   assert_eq!(g.event(terminal(status),&key(Keycode::Space,false)),Decision::Advance);
  }
  assert!(!terminal("playing"));
 }

 fn key(k:Keycode,repeat:bool)->Event{Event::KeyDown{timestamp:0,window_id:0,keycode:Some(k),scancode:None,keymod:Mod::NOMOD,repeat}}
 fn release()->Event{Event::KeyUp{timestamp:0,window_id:0,keycode:Some(Keycode::Space),scancode:None,keymod:Mod::NOMOD,repeat:false}}
 #[test]fn only_fresh_space_after_death_frame_advances(){let mut g=DeathPause::default();assert_eq!(g.event(false,&key(Keycode::Space,false)),Decision::Pass);g.presented(true);assert_eq!(g.event(true,&key(Keycode::Space,true)),Decision::Block);assert_eq!(g.event(true,&key(Keycode::Space,false)),Decision::Block);assert_eq!(g.event(true,&release()),Decision::Block);assert_eq!(g.event(true,&key(Keycode::Space,false)),Decision::Advance);}
 #[test]fn queued_space_cannot_skip_unseen_death_frame(){let mut g=DeathPause::default();assert_eq!(g.event(true,&key(Keycode::Space,false)),Decision::Block);g.presented(true);assert_eq!(g.event(true,&key(Keycode::Space,true)),Decision::Block);g.event(true,&release());assert_eq!(g.event(true,&key(Keycode::Space,false)),Decision::Advance);}
 #[test]fn incidental_inputs_block_but_quit_and_new_run_work(){let mut g=DeathPause::default();g.presented(true);for k in [Keycode::Return,Keycode::Escape,Keycode::I,Keycode::M,Keycode::Kp2]{assert_eq!(g.event(true,&key(k,false)),Decision::Block);}assert_eq!(g.event(true,&Event::MouseButtonDown{timestamp:0,window_id:0,which:0,mouse_btn:sdl2::mouse::MouseButton::Left,clicks:1,x:10,y:10}),Decision::Block);assert_eq!(g.event(true,&Event::Quit{timestamp:0}),Decision::Pass);assert_eq!(g.event(false,&key(Keycode::Return,false)),Decision::Pass);}
}
