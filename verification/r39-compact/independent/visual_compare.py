import argparse,io,subprocess,tempfile,zipfile
from pathlib import Path
from PIL import Image,ImageDraw
import numpy as np
p=argparse.ArgumentParser();p.add_argument('--encoded-dir',required=True);p.add_argument('--names',default='night-crawler,phoenix,vampire,zillennia-transcendent,witch,half-here,kelpie,skin-walker');a=p.parse_args();root=Path('compact-r39/independent-art-review');out=root/(Path(a.encoded_dir).name+'-visual');out.mkdir(exist_ok=True)
z=zipfile.ZipFile('r39-size-audit/corpse-gods-native-r39-shard-source.zip');lookup={Path(n).stem:n for n in z.namelist() if '/creatures-rgba/' in n and n.endswith('.png')}
for name in a.names.split(','):
 src=Image.open(io.BytesIO(z.read(lookup[name]))).convert('RGBA');raw=subprocess.run([str(root/'rust-decoder/target/release/independent-art-decode'),str(Path(a.encoded_dir)/(name+'.webp'))],capture_output=True,check=True).stdout;dst=Image.frombytes('RGBA',src.size,raw);box=src.getchannel('A').getbbox();src=src.crop(box);dst=dst.crop(box)
 w,h=src.size;fw,fh=320,320;scale=min(fw/w,fh/h);dw,dh=max(1,int(w*scale)),max(1,int(h*scale));sheet=Image.new('RGB',(fw*2,350*4),(25,25,25));draw=ImageDraw.Draw(sheet)
 with tempfile.TemporaryDirectory(dir=root) as tmp:
  for row,(label,bg) in enumerate([('red',[120,0,0]),('white',[255,255,255]),('gray',[128,128,128]),('black',[0,0,0])]):
   for col,(tag,im) in enumerate([('original PNG',src),('encoded candidate',dst)]):
    path=Path(tmp)/(tag+'.rgba');path.write_bytes(im.tobytes());raw=subprocess.run([str(root/'render_only'),str(path),str(w),str(h),str(dw),str(dh),*map(str,bg)],capture_output=True,check=True).stdout;pic=Image.frombytes('RGBA',(dw,dh),raw).convert('RGB');tile=Image.new('RGB',(fw,fh),tuple(bg));tile.paste(pic,((fw-dw)//2,(fh-dh)//2));sheet.paste(tile,(col*fw,row*350+30));draw.text((col*fw+4,row*350+8),name+' | '+tag+' | '+label,fill='white')
 sheet.save(out/(name+'.png'))
print(out)
