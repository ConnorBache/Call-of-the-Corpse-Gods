//! Threshold warning is desktop state only, never saved simulation state.
use sdl2::{event::Event,keyboard::Keycode};
#[derive(Default)]pub struct Warning { previous:Option<(f64,f64)>,level:Option<u8>,presented:bool,space_down:bool }
#[derive(Debug,PartialEq)]pub enum Decision {Pass,Block,Dismiss}
impl Warning {
 pub fn level(&self)->Option<u8>{self.level}
 pub fn active(&self)->bool{self.level.is_some()}
 pub fn observe(&mut self,health:Option<(f64,f64)>)->bool{
  let old=self.level;
  let Some((hp,max))=health.filter(|(hp,max)|*hp>0.&&*max>0.) else{self.previous=None;self.level=None;self.presented=false;return old.is_some();};
  let ratio=hp/max;
  if let Some((prior_hp,prior_ratio))=self.previous {
   if hp<prior_hp {for threshold in [40u8,25u8]{let limit=threshold as f64/100.;if prior_ratio>limit&&ratio<=limit{self.level=Some(threshold);self.presented=false;}}}
  }
  self.previous=Some((hp,ratio));old!=self.level
 }
 pub fn presented(&mut self){if self.active(){self.presented=true;}}
 pub fn release(&mut self,event:&Event){if matches!(event,Event::KeyUp{keycode:Some(Keycode::Space),..}){self.space_down=false;}}
 pub fn event(&mut self,event:&Event)->Decision{
  let fresh=matches!(event,Event::KeyDown{keycode:Some(Keycode::Space),repeat:false,..})&&!self.space_down;
  match event{Event::KeyDown{keycode:Some(Keycode::Space),..}=>self.space_down=true,Event::KeyUp{keycode:Some(Keycode::Space),..}=>self.space_down=false,_=>{}}
  if !self.active(){return Decision::Pass;}
  if fresh&&self.presented{self.level=None;self.presented=false;return Decision::Dismiss;}
  if matches!(event,Event::Quit{..}|Event::Window{..}|Event::RenderTargetsReset{..}|Event::RenderDeviceReset{..}){Decision::Pass}else{Decision::Block}
 }
}
pub fn paint(p:&mut crate::render::Painter,level:u8){p.centered_sheet(|p|{p.rect(430,324,420,100,[12,10,20]);p.border(430,324,420,100,[255,75,85]);p.text(&format!("Health at or below {level}%"),452,342,20,[255,100,110]);p.text("Press space to continue",452,381,17,[255,248,232]);});}
#[cfg(test)]mod tests{
 use super::*;use sdl2::keyboard::Mod;
 fn key(k:Keycode,repeat:bool)->Event{Event::KeyDown{timestamp:0,window_id:0,keycode:Some(k),scancode:None,keymod:Mod::NOMOD,repeat}}
 fn release()->Event{Event::KeyUp{timestamp:0,window_id:0,keycode:Some(Keycode::Space),scancode:None,keymod:Mod::NOMOD,repeat:false}}
 #[test]fn thresholds_rearm_and_combined_crossing_uses_lower(){let mut w=Warning::default();w.observe(Some((100.,100.)));assert!(w.observe(Some((40.,100.))));assert_eq!(w.level(),Some(40));w.presented();assert_eq!(w.event(&key(Keycode::Space,false)),Decision::Dismiss);w.event(&release());assert!(!w.observe(Some((39.,100.))));assert!(w.observe(Some((25.,100.))));assert_eq!(w.level(),Some(25));w.presented();w.event(&key(Keycode::Space,false));w.event(&release());w.observe(Some((60.,100.)));w.observe(Some((20.,100.)));assert_eq!(w.level(),Some(25));}
 #[test]fn held_queued_space_and_other_inputs_cannot_skip(){let mut w=Warning::default();w.observe(Some((100.,100.)));w.event(&key(Keycode::Space,false));w.observe(Some((30.,100.)));w.presented();assert_eq!(w.event(&key(Keycode::Space,true)),Decision::Block);assert_eq!(w.event(&key(Keycode::Return,false)),Decision::Block);assert_eq!(w.event(&key(Keycode::Escape,false)),Decision::Block);w.event(&release());assert_eq!(w.event(&key(Keycode::Space,false)),Decision::Dismiss);w.event(&release());w.observe(Some((20.,100.)));assert_eq!(w.event(&key(Keycode::Space,false)),Decision::Block);w.presented();w.event(&release());assert_eq!(w.event(&key(Keycode::Space,false)),Decision::Dismiss);}
 #[test]fn death_initial_low_hp_and_max_hp_changes_do_not_warn(){let mut w=Warning::default();assert!(!w.observe(Some((20.,100.))));w.observe(Some((100.,100.)));assert!(!w.observe(Some((100.,500.))));w.observe(Some((100.,100.)));w.observe(Some((10.,100.)));assert!(w.active());w.observe(None);assert!(!w.active());w.observe(Some((0.,100.)));assert!(!w.active());}
 #[test]fn native_warning_screenshot(){if let Ok(dir)=std::env::var("CORPSE_HEALTH_REVIEW"){let mut r:corpse_core::Run=serde_json::from_str(include_str!("../../../fixtures/avatar-room-v1.json")).unwrap();r.player.hp=25.;r.player.max_hp=100.;let mut p=crate::render::Painter::new();p.resize(1920,1080);p.set_responsive_canvas(true);let v=crate::observe::observe_with_profile(&r,&p.profile);p.game_at(&v,"",false,0.);paint(&mut p,25);std::fs::create_dir_all(&dir).unwrap();let mut e=png::Encoder::new(std::fs::File::create(format!("{dir}/health-warning.png")).unwrap(),p.width as u32,p.height as u32);e.set_color(png::ColorType::Rgba);e.set_depth(png::BitDepth::Eight);e.write_header().unwrap().write_image_data(&p.pixels).unwrap();}}
}
