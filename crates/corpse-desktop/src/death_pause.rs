//! Presentation/input gate only: the terminal run remains available for archiving.
use sdl2::{event::Event,keyboard::Keycode};
pub fn terminal(status: &str) -> bool { matches!(status, "dead" | "won") }
const END_FADE_MS: f64 = 960.;
const RESULTS_FADE_MS: f64 = 560.;

fn eased_alpha(t: f64) -> u8 {
    let t = t.clamp(0., 1.);
    (255. * t * t * (3. - 2. * t)).floor() as u8
}

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

/// Presentation time only. Death waits on the visible prompt; escape starts
/// fading on the first ready terminal frame. All clocks start on drawable frames.
#[derive(Default)]
pub struct Fade {
    started: Option<(Ending, f64)>,
    death_acknowledged: bool,
}
impl Fade {
    pub fn acknowledge_death(&mut self) {
        self.death_acknowledged = true;
    }
    pub fn sample(&mut self, status: &str, now: f64, skip_motion: bool) -> Option<([u8; 3], u8)> {
        let ending = Ending::from_status(status);
        if ending.is_none() {
            *self = Self::default();
            return None;
        }
        let Some(ending) = ending.filter(|ending| !skip_motion
            && (*ending != Ending::Death || self.death_acknowledged)) else { return None; };
        if !self.started.is_some_and(|(current, _)| current == ending) {
            self.started = Some((ending, now));
        }
        let elapsed = (now - self.started.unwrap().1) / END_FADE_MS;
        Some((ending.color(), eased_alpha(elapsed)))
    }
    pub fn prompt(&self, status: &str, fade: Option<([u8; 3], u8)>) -> bool {
        match Ending::from_status(status) {
            Some(Ending::Death) => !self.death_acknowledged,
            Some(Ending::Escape) => fade.is_none_or(|(_, alpha)| alpha == 255),
            None => false,
        }
    }
    pub fn death_complete(&self, status: &str, fade: Option<([u8; 3], u8)>) -> bool {
        status == "dead" && self.death_acknowledged && fade.is_none_or(|(_, alpha)| alpha == 255)
    }
}

/// Fade the already-laid-out results out of the same solid color as the last
/// gameplay frame. Start the clock on the first drawable frame, not the key
/// event, so autosaving or a display resize cannot skip the opaque frame.
pub struct ResultsFade {
    ending: Ending,
    started: Option<f64>,
}
impl ResultsFade {
    pub fn new(status: &str, skip_motion: bool) -> Option<Self> {
        if skip_motion { return None; }
        Ending::from_status(status).map(|ending| Self { ending, started: None })
    }
    pub fn sample(&mut self, now: f64) -> ([u8; 3], u8) {
        let started = *self.started.get_or_insert(now);
        (self.ending.color(), 255 - eased_alpha((now - started) / RESULTS_FADE_MS))
    }
    pub fn blocks_input(event: &Event) -> bool {
        !matches!(event, Event::Quit { .. } | Event::Window { .. }
            | Event::RenderTargetsReset { .. } | Event::RenderDeviceReset { .. })
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
   if status == "dead" {
    assert!(fade.prompt(status, None));
    assert_eq!(fade.sample(status,20.,false),None);
    assert_eq!(fade.sample(status,1900.,false),None);
    assert!(!fade.death_complete(status,None));
    fade.acknowledge_death();
    assert!(!fade.prompt(status,None));
   } else {
    assert!(!fade.prompt(status,Some(([255;3],0))));
   }
   assert_eq!(fade.sample(status,100.,false),Some((color,0)));
   assert_eq!(fade.sample(status,580.,false),Some((color,127)));
   assert_eq!(fade.sample(status,1059.,false),Some((color,254)));
   assert_eq!(fade.sample(status,1060.,false),Some((color,255)));
   assert_eq!(fade.death_complete(status, Some((color,255))), status=="dead");
   assert_eq!(fade.prompt(status, Some((color,255))), status=="won");
   assert_eq!(fade.sample(status,1500.,false),Some((color,255)));
   assert_eq!(fade.sample(status,1500.,true),None);
   assert_eq!(fade.sample("playing",1600.,false),None);
  }
 }
 #[test] fn results_reveal_starts_opaque_and_finishes_clear_for_both_endings() {
  for (status, color) in [("dead", [0;3]), ("won", [255;3])] {
   let mut fade = ResultsFade::new(status, false).unwrap();
   assert_eq!(fade.sample(40000.), (color,255));
   assert_eq!(fade.sample(40280.), (color,128));
   assert_eq!(fade.sample(40559.), (color,1));
   assert_eq!(fade.sample(40560.), (color,0));
   assert_eq!(fade.sample(41000.), (color,0));
   assert!(ResultsFade::new(status, true).is_none());
  }
  assert!(ResultsFade::new("playing", false).is_none());
  assert!(ResultsFade::blocks_input(&key(Keycode::Return,false)));
  assert!(ResultsFade::blocks_input(&key(Keycode::Down,false)));
  assert!(!ResultsFade::blocks_input(&Event::Quit {timestamp:0}));
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
