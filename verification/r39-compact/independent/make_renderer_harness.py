from pathlib import Path
import hashlib,json
root=Path('compact-r39/source/corpse-gods-native/crates/corpse-desktop/src')
s=(root/'render.rs').read_text()
def method(name):
 start=s.index('    '+('pub ' if name=='picture' else '')+'fn '+name+'(')
 opening=s.index('{',start);level=1;end=opening+1
 while level:
  level += (s[end]=='{')-(s[end]=='}');end+=1
 return s[start:end]
methods='\n'.join(method(n) for n in ['native_x','native_y','inside_native_clip','native_pixel','picture'])
head='''type Rgb=[u8;3]; struct Picture {w:usize,h:usize,rgba:Vec<u8>} struct Painter {width:usize,height:usize,scale:f64,origin:(i32,i32),clip:Option<(i32,i32,i32,i32)>,pixels:Vec<u8>} impl Painter {\n'''
main=r'''}
fn main() {
 let a:Vec<String>=std::env::args().collect();let w:usize=a[3].parse().unwrap();let h:usize=a[4].parse().unwrap();
 let p=Picture{w,h,rgba:std::fs::read(&a[1]).unwrap()};let q=Picture{w,h,rgba:std::fs::read(&a[2]).unwrap()};
 for bg in [[120,0,0],[255,255,255],[128,128,128],[0,0,0]] {
 for (width,height,scale) in [(640,400,0.5),(1280,800,1.0),(1920,1200,1.5),(1600,900,1.125)] {
 let blank:Vec<u8>=(0..width*height).flat_map(|_|[bg[0],bg[1],bg[2],255]).collect();
 let mut r=Painter{width,height,scale,origin:(0,0),clip:None,pixels:blank.clone()};
 let mut t=Painter{width,height,scale,origin:(0,0),clip:None,pixels:blank};
 r.picture(&p,23,19,511,637);t.picture(&q,23,19,511,637);assert_eq!(r.pixels,t.pixels,"Scaled frame changed");
 }}
}
'''
out=Path('compact-r39/independent-art-review');(out/'renderer_harness.rs').write_text(head+methods+'\n'+main)
(out/'renderer-methods.json').write_text(json.dumps({'method_names':['native_x','native_y','inside_native_clip','native_pixel','picture'],'source_file':str(root/'render.rs'),'source_sha256':hashlib.sha256(s.encode()).hexdigest(),'note':'Verbatim production sampling/compositing methods compiled in isolated minimal state harness; not full SDL executable.'},indent=2))
