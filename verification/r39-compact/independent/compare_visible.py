import io,json,hashlib,zipfile,subprocess,time,tempfile
from pathlib import Path
from PIL import Image
import numpy as np
root=Path('compact-r39/independent-art-review');d=Path('compact-r39/art-benchmark/zero-hidden');z=zipfile.ZipFile('r39-size-audit/corpse-gods-native-r39-shard-source.zip')
names=sorted(n for n in z.namelist() if '/creatures-rgba/' in n and n.endswith('.png')); assert len(names)==84
rows=[];t=time.monotonic()
with tempfile.TemporaryDirectory(dir=root) as tmp:
 for n in names:
  f=d/(Path(n).stem+'.webp');src=z.read(n);im=Image.open(io.BytesIO(src)).convert('RGBA');base=np.array(im);dec=Image.open(f).convert('RGBA');out=np.array(dec)
  ff=subprocess.run(['ffmpeg','-v','error','-i',str(f),'-f','rawvideo','-pix_fmt','rgba','-threads','1','-'],capture_output=True,check=True).stdout
  assert im.size==dec.size and out.tobytes()==ff
  mask=base[:,:,3]>0;alpha=np.array_equal(base[:,:,3],out[:,:,3]);visible=np.array_equal(base[mask],out[mask]);assert alpha and visible,n
  changed=int(np.count_nonzero(np.any(base!=out,axis=2)));assert not np.any(out[~mask,:3])
  for bg in [[120,0,0],[255,255,255],[128,128,128],[0,0,0]]:
   def comp(a):
    a=a.astype(np.uint32);return (a[:,:,:3]*a[:,:,3:4]+np.array(bg,dtype=np.uint32)*(255-a[:,:,3:4]))//255
   assert np.array_equal(comp(base),comp(out)),n
  b=Path(tmp)/'base.rgba';o=Path(tmp)/'out.rgba';b.write_bytes(base.tobytes());o.write_bytes(out.tobytes())
  subprocess.run([str(root/'renderer_harness'),str(b),str(o),str(im.width),str(im.height)],check=True)
  rows.append({'name':Path(n).stem,'dimensions':im.size,'alpha_exact':alpha,'visible_rgba_exact':visible,'pillow_ffmpeg_exact':True,'hidden_pixels_changed':changed,'four_background_composites_exact':True,'production_method_scaled_frames_exact':16,'source_sha256':hashlib.sha256(src).hexdigest(),'encoded_sha256':hashlib.sha256(f.read_bytes()).hexdigest()})
  print(Path(n).stem+' passed',flush=True)
report={'status':'passed','count':len(rows),'elapsed_seconds':time.monotonic()-t,'total_hidden_pixels_changed':sum(r['hidden_pixels_changed'] for r in rows),'note':'Baseline frozen r39 ZIP. Full-size composites red white gray black. Compiled verbatim actual Painter sampling/compositing methods at 4 framebuffer scales x4 backgrounds, not full executable/SDL run. All alpha and nonzero-alpha RGBA identical by Pillow and FFmpeg.','rows':rows}
(root/'zero-hidden-review.json').write_text(json.dumps(report,indent=2));print(json.dumps({k:v for k,v in report.items() if k!='rows'}))
