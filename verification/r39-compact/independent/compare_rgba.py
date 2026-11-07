#!/usr/bin/env python3
"""Independent frozen-PNG versus distribution-art decoded RGBA verification."""
import argparse, hashlib, io, json, subprocess, time, zipfile
from pathlib import Path
from PIL import Image
p=argparse.ArgumentParser();p.add_argument('--base-zip',required=True);p.add_argument('--encoded-dir',required=True);p.add_argument('--output',required=True);a=p.parse_args()
z=zipfile.ZipFile(a.base_zip)
names=sorted(n for n in z.namelist() if '/creatures-rgba/' in n and n.endswith('.png'))
assert len(names)==84,len(names)
d=Path(a.encoded_dir);rows=[];beg=time.monotonic()
for n in names:
    stem=Path(n).stem;candidates=[q for q in d.glob(stem+'.*') if q.suffix.lower() in ('.webp','.png')]
    if len(candidates)!=1: raise RuntimeError(f'{stem}: expected one encoded file, got {candidates}')
    f=candidates[0];src=z.read(n);im=Image.open(io.BytesIO(src)).convert('RGBA');w,h=im.size;base=im.tobytes()
    dec=Image.open(f).convert('RGBA');pillow=dec.tobytes()
    ff=subprocess.run(['ffmpeg','-v','error','-i',str(f),'-f','rawvideo','-pix_fmt','rgba','-threads','1','-'],capture_output=True,check=True).stdout
    row={'name':stem,'dimensions':[w,h],'baseline_file_sha256':hashlib.sha256(src).hexdigest(),'baseline_rgba_sha256':hashlib.sha256(base).hexdigest(),'encoded_file':str(f),'encoded_file_bytes':f.stat().st_size,'encoded_file_sha256':hashlib.sha256(f.read_bytes()).hexdigest(),'pillow_dimensions_match':dec.size==im.size,'pillow_rgba_exact':pillow==base,'ffmpeg_rgba_exact':ff==base,'ffmpeg_rgba_bytes':len(ff),'alpha_exact':len(ff)==len(base) and ff[3::4]==base[3::4]}
    rows.append(row)
    print(f'{stem}: PIL={row["pillow_rgba_exact"]} FFmpeg={row["ffmpeg_rgba_exact"]}',flush=True)
report={'status':'passed' if all(r['pillow_dimensions_match'] and r['pillow_rgba_exact'] and r['ffmpeg_rgba_exact'] for r in rows) else 'failed','count':len(rows),'elapsed_seconds':time.monotonic()-beg,'comparison':'Every decoded RGBA byte, including transparent RGB; baseline read from frozen r39 archive, encoded art decoded independently by Pillow and FFmpeg.','baseline_archive_sha256':hashlib.sha256(Path(a.base_zip).read_bytes()).hexdigest(),'rows':rows}
Path(a.output).write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({k:v for k,v in report.items() if k!='rows'}))
raise SystemExit(0 if report['status']=='passed' else 1)
