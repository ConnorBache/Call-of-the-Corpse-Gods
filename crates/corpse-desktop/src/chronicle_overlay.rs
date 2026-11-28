//! UI-only retractable Chronicle. Stored lines are observed once on opening.
use crate::render::{Painter,WIDTH,HEIGHT};
const DURATION:f64=220.;
#[derive(Default)]
pub struct Shutter{from:f64,target:f64,started:f64,pub offset:usize}
impl Shutter{
    /// O(1), continuous eased height; reversing never jumps to an endpoint.
    pub fn fraction(&self,now:f64,reduced:bool)->f64{if reduced{return self.target;}let t=((now-self.started)/DURATION).clamp(0.,1.);self.from+(self.target-self.from)*(t*t*(3.-2.*t))}
    /// O(1), opens from current height; scroll position intentionally survives close/reopen.
    pub fn open(&mut self,now:f64,reduced:bool){self.from=self.fraction(now,reduced);self.target=1.;self.started=now;}
    /// O(1), input remains captured until the closing animation finishes.
    pub fn close(&mut self,now:f64,reduced:bool){if self.target==0.{return;}self.from=self.fraction(now,reduced);self.target=0.;self.started=now;}
    /// O(1), animation-only redraw indication, no observation invalidation.
    pub fn animating(&self,now:f64,reduced:bool)->bool{!reduced&&(now-self.started)<DURATION&&(self.from-self.target).abs()>f64::EPSILON}
    /// O(1), release modal input only at the fully retracted endpoint.
    pub fn closed(&self,now:f64,reduced:bool)->bool{self.target==0.&&!self.animating(now,reduced)}
    /// O(1), bounded older/newer scrolling independent of simulation state.
    pub fn scroll(&mut self,older:i32,total:usize){self.scroll_for_height(older,total,HEIGHT as i32)}
    pub fn scroll_for_height(&mut self,older:i32,total:usize,height:i32){self.offset=if older>=0{self.offset.saturating_add(older as usize)}else{self.offset.saturating_sub(older.unsigned_abs()as usize)}.min(total.saturating_sub(capacity_height(height)));}
}
/// O(1), entire screen-width header retracts the panel; shared with input.
pub fn header_hit(x:i32,y:i32)->bool{(12..WIDTH as i32-12).contains(&x)&&(6..44).contains(&y)}
/// O(1), maximum visible rows at full height.
fn capacity()->usize{capacity_height(HEIGHT as i32)}
fn capacity_height(height:i32)->usize{((height-18-80)/24).max(1)as usize}
pub fn header_hit_painter(p:&Painter,x:i32,y:i32)->bool{(12..p.logical_width()-12).contains(&x)&&(6..44).contains(&y)}
pub fn bounds_painter(p:&Painter,shutter:&Shutter,now:f64,reduced:bool,rows:usize)->Option<(i32,i32,i32,i32)>{
    let measured=80+rows.min(capacity_height(p.logical_height())) as i32*24;
    let h=(measured.min(p.logical_height()-18)as f64*shutter.fraction(now,reduced)).round()as i32;
    (h>0).then_some((12,6,p.logical_width()-24,h))
}
/// Exact logical painted coverage, including the retracting shutter's current height.
pub fn bounds(shutter:&Shutter,now:f64,reduced:bool)->Option<(i32,i32,i32,i32)>{let h=((HEIGHT as i32-18)as f64*shutter.fraction(now,reduced)).round()as i32;(h>0).then_some((12,6,WIDTH as i32-24,h))}
/// O(visible glyphs + clipped panel pixels); no Run access, history observation, or layout search.
pub fn paint(p:&mut Painter,lines:&[String],shutter:&Shutter,now:f64,reduced:bool){
 let Some((_,_,_,h))=bounds_painter(p,shutter,now,reduced,lines.len())else{return;};
 let old_clip=p.clip;p.clip=Some((12,6,p.logical_width()-24,h));
 p.rect(12,6,p.logical_width()-24,h,[11,13,21]);p.border(12,6,p.logical_width()-24,h,[104,81,66]);
 p.text("CHRONICLE",28,15,22,[238,211,151]);p.rect(24,44,p.logical_width()-48,1,[83,68,59]);
 let footer=6+h-27;
 // The bottom rule travels with the shutter as it slides down over the map.
 let count=capacity_height(p.logical_height());let end=lines.len().saturating_sub(shutter.offset.min(lines.len().saturating_sub(count)));let begin=end.saturating_sub(count);
 for(i,line)in lines[begin..end].iter().enumerate(){let y=58+i as i32*24;if y+18>footer{break;}p.text(&crate::render::truncate(line,130),28,y,16,crate::chronicle::color(line));}
 if footer>=46 {
     p.rect(24,footer,p.logical_width()-48,1,[83,68,59]);
     p.close_hint(28,footer+6);
     if lines.len()>count { p.text("Wheel / ↑ / ↓ to scroll",142,footer+6,11,[167,164,181]); }
 }
 p.clip=old_clip;
}
#[cfg(test)]mod tests{
 use super::*;
 /// O(1), exact endpoints and smooth reversal; capture continues during retraction.
 #[test]fn endpoints_reversal_and_capture(){let mut s=Shutter::default();assert!(s.closed(0.,false));s.open(0.,false);assert!(!s.closed(0.,false));assert_eq!(s.fraction(0.,false),0.);assert_eq!(s.fraction(220.,false),1.);let mid=s.fraction(110.,false);assert_eq!(mid,0.5);s.close(110.,false);assert_eq!(s.fraction(110.,false),mid);assert!(!s.closed(220.,false));assert!(s.closed(330.,false));}
 /// O(1), reduced motion jumps and scroll remains stable across toggles.
 #[test]fn reduced_motion_and_scroll_retention(){let mut s=Shutter::default();s.open(0.,true);assert_eq!(s.fraction(0.,true),1.);s.scroll(7,100);s.close(1.,true);assert!(s.closed(1.,true));s.open(2.,true);assert_eq!(s.offset,7);s.scroll(999,100);assert_eq!(s.offset,100-capacity());s.scroll(-999,100);assert_eq!(s.offset,0);}
 /// O(panel pixels), shutter clipping keeps outside pixels intact and restores prior clip.
 #[test]fn mid_panel_clip_and_header_bounds(){let mut p=Painter::new();p.clear([1,2,3]);let before=p.pixels.clone();let mut s=Shutter::default();s.open(0.,false);paint(&mut p,&vec!["A remembered event.".into();40],&s,110.,false);assert_eq!(p.clip,None);let bottom=6+(HEIGHT as i32-18)/2;for y in bottom+1..HEIGHT as i32{let at=y as usize*WIDTH*4;assert_eq!(&before[at..at+WIDTH*4],&p.pixels[at..at+WIDTH*4]);}assert!(header_hit(12,6));assert!(!header_hit(12,44));}
}
