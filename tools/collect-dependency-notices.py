#!/usr/bin/env python3
import pathlib,tomllib,json,shutil
root=pathlib.Path(__file__).resolve().parents[1];cargo=root.parent/'.cargo/registry/src';dest=root/'third-party-notices';dest.mkdir(exist_ok=True)
manifest=[]
for package in tomllib.loads((root/'Cargo.lock').read_text())['package']:
 if 'source' not in package:continue
 name,version=package['name'],package['version'];dirs=list(cargo.glob(f'*/{name}-{version}'))
 if not dirs:manifest.append({'name':name,'version':version,'status':'source not downloaded'});continue
 src=dirs[0];meta=tomllib.loads((src/'Cargo.toml').read_text())['package'];licenses=[]
 for f in src.iterdir():
  if f.is_file() and f.name.upper().startswith(('LICENSE','COPYING','NOTICE')):
   out=dest/f'{name}-{version}'/f.name;out.parent.mkdir(exist_ok=True);shutil.copy2(f,out);licenses.append(str(out.relative_to(dest)))
 if name=='sdl2-sys':
  for f in [src/'SDL/LICENSE.txt', src.parent/f'sdl2-{version}'/'LICENSE']:
   out=dest/f'{name}-{version}'/('SDL-LICENSE.txt' if f.name=='LICENSE.txt' else 'rust-sdl2-LICENSE');out.parent.mkdir(exist_ok=True);shutil.copy2(f,out);licenses.append(str(out.relative_to(dest)))
 manifest.append({'name':name,'version':version,'license':meta.get('license'),'repository':meta.get('repository'),'notices':licenses})
(dest/'manifest.json').write_text(json.dumps(manifest,indent=2))
print(len(manifest),'registry dependencies');print([m for m in manifest if not m.get('notices')])
