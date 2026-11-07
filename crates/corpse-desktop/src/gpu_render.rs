//! Observation-only GPU drawing: CPU emits ordered quads; SDL's accelerated
//! renderer rasterizes, tints and blends them. There is no painted-frame upload.
//! Software Painter remains the reference and fallback, including headless export.
use std::collections::HashMap;
use sdl2::{pixels::{Color, PixelFormatEnum}, rect::{FPoint, Rect}, render::{Canvas, Texture, TextureCreator, Vertex}, video::{Window, WindowContext}};

/// Prefer an accelerated renderer. A retained Window clone lets a failed driver
/// creation retry on SDL's software driver without losing the user's window.
pub fn create_canvas(window:Window,force_software:bool)->Result<(Canvas<Window>,bool),String>{
    if !force_software {
        match window.clone().into_canvas().accelerated().build(){
            Ok(canvas)=>return Ok((canvas,true)),
            Err(error)=>eprintln!("Accelerated renderer unavailable; using software: {error}"),
        }
    }
    let index=sdl2::render::drivers().position(|driver|driver.flags & sdl2::sys::SDL_RendererFlags::SDL_RENDERER_SOFTWARE as u32 !=0).ok_or("SDL software renderer unavailable")?;
    let canvas=window.into_canvas().index(index as u32).software().build().map_err(|error|error.to_string())?;
    Ok((canvas,false))
}

// The sdl2 convenience setters panic on native errors. Keep GPU operations in
// the Result boundary so main can recreate the renderer in software instead.
fn checked_sdl(status:i32,operation:&str)->Result<(),String>{
    if status==0 {Ok(())}else{Err(format!("{operation}: {}",sdl2::get_error()))}
}

pub type Bounds = (i32,i32,i32,i32);
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum MaskKey {
    Text(char,u16), Styled(char,u16,usize,u16), Prose(char,u32,bool),
    Ascii(char,i32,i32,u16,bool),
    Identity(String,i32,i32,u32,bool,u64,u64),
}
pub struct Mask { pub width:usize, pub height:usize, pub coverage:Vec<u8> }
#[derive(Clone, Debug)]
pub struct Quad { pub rect:Bounds, pub mask:Option<(usize,Bounds)>, pub color:[u8;4] }
#[derive(Default)]
pub struct Frame {
    pub epoch:u64,
    pub quads:Vec<Quad>,
    pub masks:Vec<Mask>,
    keys:HashMap<MaskKey,usize>,
    bytes:usize,
    pub clear:Option<[u8;3]>,
    pub unsupported:Option<&'static str>,
}
impl Frame {
    pub fn begin(&mut self) {
        self.quads.clear(); self.clear=None;self.unsupported=None;
        // Bound persistent CPU mask residency; invalidate only at frame boundaries.
        if self.masks.len()>4096 || self.bytes>32*1024*1024 { self.invalidate(); }
    }
    pub fn invalidate(&mut self) { self.quads.clear();self.clear=None;self.keys.clear();self.masks.clear();self.bytes=0;self.epoch=self.epoch.wrapping_add(1); }
    pub fn fill(&mut self,rect:Bounds,color:[u8;3],alpha:u8) {
        if rect.0<rect.2 && rect.1<rect.3 && alpha>0 { self.quads.push(Quad{rect,mask:None,color:[color[0],color[1],color[2],alpha]}); }
    }
    /// Warm expected O(key bytes), one clipped quad. Cold O(mask area) copy;
    /// italic masks are sheared once at cache insertion, not every frame.
    pub fn mask(&mut self,key:MaskKey,bounds:Bounds,x:i32,y:i32,w:usize,h:usize,mask:&[u8],color:[u8;3],alpha:f64,italic:bool) {
        if w==0 || h==0 || alpha<=0. {return;}
        let shear=if italic {(w as f64/8.).ceil() as i32} else {0};
        let x=x-shear; let width=w+2*shear as usize;
        let rect=(x.max(bounds.0),y.max(bounds.1),(x+width as i32).min(bounds.2),(y+h as i32).min(bounds.3));
        if rect.0>=rect.2 || rect.1>=rect.3 {return;}
        let id=if let Some(&id)=self.keys.get(&key) {id} else {
            let coverage=if italic {
                let mut result=vec![0;width*h];
                for yy in 0..h {
                    let dx=((h as i32-1-2*yy as i32) as f64*w as f64/(8.*h.max(1) as f64)).round() as i32;
                    let start=yy*width+(shear+dx) as usize;
                    result[start..start+w].copy_from_slice(&mask[yy*w..(yy+1)*w]);
                } result
            } else {mask.to_vec()};
            let id=self.masks.len();self.bytes+=coverage.len();
            self.masks.push(Mask{width,height:h,coverage});self.keys.insert(key,id);id
        };
        self.quads.push(Quad{rect,mask:Some((id,(rect.0-x,rect.1-y,rect.2-x,rect.3-y))),color:[color[0],color[1],color[2],(alpha*255.).round().clamp(0.,255.)as u8]});
    }
}
#[derive(Clone,Copy)]
struct Slot {page:usize,x:u32,y:u32}
struct Page<'a> {texture:Texture<'a>, x:u32,y:u32,row:u32}
#[derive(Default,Debug,Clone,Copy)]
pub struct Stats {pub quads:usize,pub batches:usize,pub uploaded_mask_bytes:usize,pub atlas_pages:usize}
pub struct Renderer<'a> {
    creator:&'a TextureCreator<WindowContext>, pages:Vec<Page<'a>>, slots:Vec<Slot>,
    epoch:Option<u64>,size:u32,vertices:Vec<Vertex>,indices:Vec<u32>,
    pub stats:Stats,
}
impl<'a> Renderer<'a> {
    pub fn new(creator:&'a TextureCreator<WindowContext>,max_texture:u32)->Self {
        Self{creator,pages:Vec::new(),slots:Vec::new(),epoch:None,size:2048.min(if max_texture==0{2048}else{max_texture}),vertices:Vec::new(),indices:Vec::new(),stats:Stats::default()}
    }
    pub fn reset(&mut self) { self.pages.clear();self.slots.clear();self.epoch=None;self.vertices.clear();self.indices.clear(); }
    fn page(&mut self)->Result<(),String> {
        if self.pages.len()>=4 {return Err("GPU glyph atlas exceeds its 64 MiB budget".into());}
        let mut texture=self.creator.create_texture_static(PixelFormatEnum::RGBA32,self.size,self.size).map_err(|e|e.to_string())?;
        // SAFETY: texture is live and owned here; these calls retain no pointers.
        checked_sdl(unsafe{sdl2::sys::SDL_SetTextureBlendMode(texture.raw(),sdl2::sys::SDL_BlendMode::SDL_BLENDMODE_BLEND)},"GPU texture blend mode")?;
        checked_sdl(unsafe{sdl2::sys::SDL_SetTextureScaleMode(texture.raw(),sdl2::sys::SDL_ScaleMode::SDL_ScaleModeNearest)},"GPU texture sampling")?;
        // Dedicated opaque white sample for all untextured quads.
        texture.update(Rect::new(0,0,1,1),&[255,255,255,255],4).map_err(|e|e.to_string())?;
        self.pages.push(Page{texture,x:2,y:2,row:0});Ok(())
    }
    fn upload(&mut self,mask:&Mask)->Result<(),String> {
        let (w,h)=(mask.width as u32,mask.height as u32);
        if w+2>self.size || h+2>self.size {return Err("Glyph exceeds GPU atlas dimensions".into());}
        if self.pages.is_empty(){self.page()?;}
        let last=self.pages.last_mut().unwrap();
        if last.x+w+1>self.size {last.x=1;last.y+=last.row+1;last.row=0;}
        if last.y+h+1>self.size {self.page()?;}
        let page=self.pages.len()-1;let last=&mut self.pages[page];
        let slot=Slot{page,x:last.x,y:last.y};
        let mut rgba=Vec::with_capacity(mask.coverage.len()*4);
        for &a in &mask.coverage {rgba.extend_from_slice(&[255,255,255,a]);}
        last.texture.update(Rect::new(slot.x as i32,slot.y as i32,w,h),&rgba,mask.width*4).map_err(|e|e.to_string())?;
        self.stats.uploaded_mask_bytes+=rgba.len();
        last.x+=w+1;last.row=last.row.max(h);self.slots.push(slot);Ok(())
    }
    fn flush(&mut self,canvas:&mut Canvas<Window>,page:usize)->Result<(),String> {
        if self.vertices.is_empty(){return Ok(());}
        canvas.render_geometry(&self.vertices,Some(&self.pages[page].texture),&self.indices[..])?;
        self.stats.batches+=1;self.vertices.clear();self.indices.clear();Ok(())
    }
    /// Warm O(Q) CPU command/geometry work, GPU O(covered samples + overdraw).
    /// Cold adds O(new mask area) conversion/upload and driver allocation. Batches
    /// preserve order; only adjacent commands using the same atlas are combined.
    pub fn draw(&mut self,canvas:&mut Canvas<Window>,frame:&Frame)->Result<(),String> {
        if let Some(reason)=frame.unsupported{return Err(reason.into());}
        self.stats=Stats::default();self.vertices.clear();self.indices.clear();
        if self.epoch!=Some(frame.epoch){self.reset();self.epoch=Some(frame.epoch);}
        while self.slots.len()<frame.masks.len(){self.upload(&frame.masks[self.slots.len()])?;}
        if self.pages.is_empty(){self.page()?;}
        // SAFETY: canvas owns this live renderer; null clip means disable clipping.
        checked_sdl(unsafe{sdl2::sys::SDL_RenderSetClipRect(canvas.raw(),std::ptr::null())},"GPU clip reset")?;
        checked_sdl(unsafe{sdl2::sys::SDL_SetRenderDrawBlendMode(canvas.raw(),sdl2::sys::SDL_BlendMode::SDL_BLENDMODE_BLEND)},"GPU draw blend mode")?;
        if let Some(c)=frame.clear {
            checked_sdl(unsafe{sdl2::sys::SDL_SetRenderDrawColor(canvas.raw(),c[0],c[1],c[2],255)},"GPU clear color")?;
            checked_sdl(unsafe{sdl2::sys::SDL_RenderClear(canvas.raw())},"GPU clear")?;
        }
        let mut active=0;
        for quad in &frame.quads {
            let (page,uv)=if let Some((id,src))=quad.mask {
                let s=self.slots[id];(s.page,((s.x as f32+src.0 as f32)/self.size as f32,(s.y as f32+src.1 as f32)/self.size as f32,(s.x as f32+src.2 as f32)/self.size as f32,(s.y as f32+src.3 as f32)/self.size as f32))
            } else {(active,(0.5/self.size as f32,0.5/self.size as f32,0.5/self.size as f32,0.5/self.size as f32))};
            if page!=active || self.vertices.len()>=65532 {self.flush(canvas,active)?;active=page;}
            let (l,t,r,b)=quad.rect;let (u,v,uu,vv)=uv;
            let c=quad.color;let color=Color::RGBA(c[0],c[1],c[2],c[3]);let at=self.vertices.len()as u32;
            for (x,y,u,v) in [(l,t,u,v),(r,t,uu,v),(r,b,uu,vv),(l,b,u,vv)] {
                self.vertices.push(Vertex{position:FPoint::new(x as f32,y as f32),color,tex_coord:FPoint::new(u,v)});
            }
            self.indices.extend_from_slice(&[at,at+1,at+2,at,at+2,at+3]);
        }
        self.flush(canvas,active)?;self.stats.quads=frame.quads.len();self.stats.atlas_pages=self.pages.len();Ok(())
    }
}

#[cfg(test)]
#[path="gpu_render_tests.rs"]
mod tests;
