import argparse,io,json,hashlib,zipfile,subprocess,time
from pathlib import Path
from PIL import Image
import numpy as np
p=argparse.ArgumentParser();p.add_argument('--encoded-dir',required=True);p.add_argument('--output',required=True);a=p.parse_args()
z=zipfile.ZipFile('r39-size-audit/corpse-gods-native-r39-shard-source.zip');d=Path(a.encoded_dir)
names=sorted(n for n in z.namelist() if '/creatures-rgba/' in n and n.endswith('.png'));assert len(names)==84
assert {Path(n).stem for n in names}=={f.stem for f in d.glob('*.webp')}
rows=[];t=time.monotonic()
for n in names:
 f=d/(Path(n).stem+'.webp');src=z.read(n);im=Image.open(io.BytesIO(src)).convert('RGBA');base=np.array(im);dec=Image.open(f).convert('RGBA');out=np.array(dec)
 ff=subprocess.run(['ffmpeg','-v','error','-i',str(f),'-f','rawvideo','-pix_fmt','rgba','-threads','1','-'],capture_output=True,check=True).stdout
 assert im.size==dec.size and len(ff)==base.size,n
 ffarray=np.frombuffer(ff,np.uint8).reshape(base.shape)
 assert np.array_equal(base[:,:,3],out[:,:,3]) and np.array_equal(base[:,:,3],ffarray[:,:,3]),n
 production=subprocess.run(['compact-r39/independent-art-review/rust-decoder/target/release/independent-art-decode',str(f)],capture_output=True,check=True).stdout
 out=np.frombuffer(production,np.uint8).reshape(base.shape)
 alpha=np.array_equal(base[:,:,3],out[:,:,3]);assert alpha,n
 mask=base[:,:,3]>0;err=np.abs(base[:,:,:3].astype(np.int16)-out[:,:,:3].astype(np.int16));e=err[mask];dark=mask & (base[:,:,:3].max(axis=2)<=32);edge=(base[:,:,3]>0)&(base[:,:,3]<255)
 weighted=(err.astype(np.float64)*base[:,:,3:4]/255)[mask]
 row={'name':Path(n).stem,'dimensions':im.size,'alpha_exact':alpha,'all_three_decoder_alpha_exact':True,'runtime_decoder':'image-webp 0.2.4','bytes':f.stat().st_size,'visible_rgb_mae':float(e.mean()),'visible_rgb_rmse':float(np.sqrt(np.square(e.astype(float)).mean())),'visible_rgb_error_p95':float(np.percentile(e,95)),'visible_rgb_error_p99':float(np.percentile(e,99)),'visible_rgb_max_error':int(e.max()),'alpha_weighted_rgb_mae':float(weighted.mean()),'dark_pixels':int(dark.sum()),'dark_rgb_mae':float(err[dark].mean()) if dark.any() else None,'partial_alpha_pixels':int(edge.sum()),'partial_alpha_rgb_mae':float(err[edge].mean()) if edge.any() else None,'source_sha256':hashlib.sha256(src).hexdigest(),'encoded_sha256':hashlib.sha256(f.read_bytes()).hexdigest()};rows.append(row)
 print(row['name']+' passed alpha; RGB MAE '+str(round(row['visible_rgb_mae'],3)),flush=True)
r={'status':'passed_alpha_dimensions_coverage','count':len(rows),'elapsed_seconds':time.monotonic()-t,'encoded_bytes':sum(r['bytes'] for r in rows),'note':'All84 alpha bytes and dimensions exact; source RGBA PNG from frozen ZIP, WebP decoded by Pillow, FFmpeg and actual runtime image-webp0.2.4; alpha exact in all three. RGB metrics use actual runtime decoder; lossy decoders can differ in RGB conversion. This is quantitative QA, not alone visual acceptance.','rows':rows};Path(a.output).write_text(json.dumps(r,indent=2));print(json.dumps({k:v for k,v in r.items() if k!='rows'}))
