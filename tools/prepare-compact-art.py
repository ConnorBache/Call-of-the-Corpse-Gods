#!/usr/bin/env python3
"""Build approved compact runtime art without changing authored PNGs.

Requires Pillow with WebP support and NumPy. RGB is intentionally lossy; alpha
and dimensions must remain exact. Run from anywhere after placing in tools/.
Validation takes O(sum(P)) work and O(W*max(P)) temporary memory for image
pixel counts P and W workers. Codec search complexity is not assumed linear;
libwebp method=6 encoding costs are measured separately.
"""
import argparse, concurrent.futures, hashlib, json, math, os, platform
from pathlib import Path
import numpy as np
import PIL
from PIL import Image, features

def sha(data): return hashlib.sha256(data).hexdigest()
def main():
 p=argparse.ArgumentParser(description=__doc__)
 p.add_argument('--root',type=Path,default=Path(__file__).resolve().parents[1])
 p.add_argument('--quality',type=int,required=True,choices=(90,95,100))
 p.add_argument('--workers',type=int,default=4)
 a=p.parse_args(); assets=a.root/'crates/corpse-desktop/assets';src=assets/'creatures-rgba';out=assets/'creatures-runtime'
 files=sorted(src.glob('*.png'));assert files, f'No source PNGs in {src}'
 out.mkdir(exist_ok=True)
 settings=dict(lossless=False,exact=True,quality=a.quality,alpha_quality=100,method=6)
 def encode(path):
  source=path.read_bytes();im=Image.open(path).convert('RGBA');raw=im.tobytes();dest=out/path.with_suffix('.webp').name;tmp=dest.with_suffix('.webp.tmp')
  try:
   im.save(tmp,'WEBP',**settings);back=Image.open(tmp).convert('RGBA');decoded=back.tobytes()
   if back.size!=im.size or decoded[3::4]!=raw[3::4]:raise ValueError(f'Alpha/dimension mismatch: {path.name}')
   ar=np.asarray(im);br=np.asarray(back);err=np.abs(ar[:,:,:3].astype(np.int16)-br[:,:,:3].astype(np.int16))[ar[:,:,3]>0]
   count=int(err.size);absolute=int(err.sum());squared=int(np.square(err.astype(np.int64)).sum());mse=squared/count if count else 0
   encoded=tmp.read_bytes();os.replace(tmp,dest)
   return dict(id=path.stem,width=im.width,height=im.height,source_file=path.name,source_sha256=sha(source),source_rgba_sha256=sha(raw),encoded_file=dest.name,encoded_sha256=sha(encoded),encoded_rgba_sha256=sha(decoded),alpha_sha256=sha(raw[3::4]),decoded_alpha_sha256=sha(decoded[3::4]),source_bytes=len(source),encoded_bytes=len(encoded),alpha_exact=True,dimensions_exact=True,visible_rgb_channels=count,visible_rgb_absolute_error_sum=absolute,visible_rgb_squared_error_sum=squared,visible_rgb_mae=absolute/count if count else 0,visible_rgb_rmse=math.sqrt(mse),visible_rgb_max_error=int(err.max()) if count else 0,visible_rgb_psnr_db=10*math.log10(255**2/mse) if mse else None)
  finally:
   if tmp.exists():tmp.unlink()
 with concurrent.futures.ThreadPoolExecutor(max_workers=a.workers) as pool: rows=list(pool.map(encode,files))
 expected={r['encoded_file'] for r in rows};stale={p.name for p in out.glob('*.webp')}-expected
 if stale:raise ValueError(f'Unexpected stale runtime assets; review manually: {sorted(stale)}')
 manifest=dict(schema=1,format='WebP',settings=settings,resized=False,color_lossy=True,alpha_lossless_verified=True,source_directory='creatures-rgba',output_directory='creatures-runtime',versions=dict(python=platform.python_version(),pillow=PIL.__version__,libwebp=features.version('webp'),numpy=np.__version__),count=len(rows),total_source_bytes=sum(r['source_bytes'] for r in rows),total_encoded_bytes=sum(r['encoded_bytes'] for r in rows),plates=rows)
 target=assets/'creature-runtime-manifest.json';tmp=target.with_suffix('.json.tmp');tmp.write_text(json.dumps(manifest,indent=2)+'\n');os.replace(tmp,target)
 print(f'{len(rows)} plates; {manifest["total_encoded_bytes"]:,} bytes; alpha/dimensions verified; {target}')
if __name__=='__main__':main()
