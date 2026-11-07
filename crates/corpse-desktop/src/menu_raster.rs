//! One-size, exact destination sampling cache for the independently animated emblem.
use super::{Picture, menu_heartbeat};
#[derive(Clone, Copy)]
struct Sample { x: i32, y: i32, rgba: [u8; 4] }
pub(super) struct Raster {
    pub size: (i32,i32),
    veins: Vec<(Sample,f64)>,
    foreground: Vec<Sample>,
    heart_bounds: Option<(usize,usize,usize,usize)>,
    heart_columns: Vec<(i32,usize)>,
}
/// O(1) time/space. Caller proves framebuffer bounds and skips transparent samples.
fn blend(pixels: &mut [u8], width: usize, x: i32, y: i32, rgba: [u8;4]) {
    let at=(y as usize*width+x as usize)*4;
    let pixel=&mut pixels[at..at+4];
    let a=rgba[3] as u32;
    if a==255 {pixel.copy_from_slice(&rgba);return;}
    for k in 0..3 {pixel[k]=((rgba[k] as u32*a+pixel[k] as u32*(255-a))/255) as u8;}
    pixel[3]=255;
}
impl Raster {
    /// Cold O(source heart pixels + destination area), O(occupied samples + width)
    /// retained memory. Only one native size is kept; no time/phase/clip/origin key.
    pub fn new(layers: &[Picture;3], w: i32, h: i32) -> Self {
        let mut veins=Vec::new();let mut foreground=Vec::new();
        for layer in [0,2] {
            let p=&layers[layer];
            for y in 0..h {for x in 0..w {
                let at=((y as usize*p.h/h as usize)*p.w+x as usize*p.w/w as usize)*4;
                if p.rgba[at+3]==0 {continue;}
                let sample=Sample{x,y,rgba:p.rgba[at..at+4].try_into().unwrap()};
                if layer==0 {
                    // Destination coordinates are intentional: repeated source texels can
                    // have different wave distance. Keep the scalar operation order.
                    let (u,v)=(x as f64/w as f64,y as f64/h as f64);
                    let dx=(u-0.505)*p.w as f64/p.h as f64;let dy=v-0.633;
                    veins.push((sample,(dx*dx+dy*dy).sqrt()));
                } else {foreground.push(sample);}
            }}
        }
        let heart=&layers[1];let mut bounds=None;
        for (i,pixel) in heart.rgba.chunks_exact(4).enumerate() {
            if pixel[3]==0 {continue;}
            let (x,y)=(i%heart.w,i/heart.w);
            let (l,t,r,b)=bounds.unwrap_or((x,y,x+1,y+1));
            bounds=Some((l.min(x),t.min(y),r.max(x+1),b.max(y+1)));
        }
        Self{size:(w,h),veins,foreground,heart_bounds:bounds,heart_columns:Vec::with_capacity(w as usize)}
    }
    /// Warm O(vein samples + foreground samples + width + height + transformed
    /// heart bounding-box area), O(width) reusable scratch; no full-canvas heart scan.
    /// Clipping/origin are resolved at draw time, never cached. Layer order is unchanged.
    pub fn paint(&mut self, pixels: &mut [u8], width: usize, origin: (i32,i32),
        clip: (i32,i32,i32,i32), layers: &[Picture;3], frame: u32) {
        let (nx,ny)=origin;let (l,t,r,b)=clip;let (w,h)=self.size;
        for (sample,distance) in &self.veins {
            let (x,y)=(nx+sample.x,ny+sample.y);
            if x<l||x>=r||y<t||y>=b {continue;}
            let wave=menu_heartbeat::vein_pulse(frame,*distance);
            let mut rgba=sample.rgba;
            for channel in &mut rgba[..3] {*channel=((*channel as f64*(1.+0.48*wave)).round().min(255.)) as u8;}
            blend(pixels,width,x,y,rgba);
        }
        if let Some((hl,ht,hr,hb))=self.heart_bounds {
            let p=&layers[1];let beat=menu_heartbeat::pulse(frame);
            self.heart_columns.clear();
            for x in (l-nx).max(0)..(r-nx).min(w) {
                let px=if beat!=0. {
                    let u=x as f64/w as f64;
                    let sx=0.505+(u-0.505)/(1.+0.06*beat);
                    if !(0. ..1.).contains(&sx) {continue;}
                    (sx*p.w as f64) as usize
                } else {x as usize*p.w/w as usize};
                if px>=hl&&px<hr {self.heart_columns.push((x,px));}
            }
            for y in (t-ny).max(0)..(b-ny).min(h) {
                let py=if beat!=0. {
                    let v=y as f64/h as f64;
                    let sy=0.633+(v-0.633)/(1.+0.045*beat);
                    if !(0. ..1.).contains(&sy) {continue;}
                    (sy*p.h as f64) as usize
                } else {y as usize*p.h/h as usize};
                if py<ht||py>=hb {continue;}
                for &(x,px) in &self.heart_columns {
                    let at=(py*p.w+px)*4;
                    if p.rgba[at+3]==0 {continue;}
                    blend(pixels,width,nx+x,ny+y,p.rgba[at..at+4].try_into().unwrap());
                }
            }
        }
        for sample in &self.foreground {
            let (x,y)=(nx+sample.x,ny+sample.y);
            if x<l||x>=r||y<t||y>=b {continue;}
            blend(pixels,width,x,y,sample.rgba);
        }
    }
    /// O(1) diagnostic accounting of retained Vec capacities, excluding allocator metadata.
    #[cfg(test)]
    pub fn storage(&self) -> (usize,usize,usize) {
        (self.veins.len(),self.foreground.len(),
            self.veins.capacity()*std::mem::size_of::<(Sample,f64)>()+
            self.foreground.capacity()*std::mem::size_of::<Sample>()+
            self.heart_columns.capacity()*std::mem::size_of::<(i32,usize)>())
    }
}
