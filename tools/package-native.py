#!/usr/bin/env python3
"""Package an explicitly named native checkpoint, with no compile or runtime claims."""
import argparse, hashlib, json, pathlib, zipfile, subprocess, sys
p=argparse.ArgumentParser();p.add_argument('--platform',choices=['linux','windows'],required=True);p.add_argument('--binary',type=pathlib.Path,required=True);p.add_argument('--output',type=pathlib.Path,required=True);p.add_argument('--label',required=True);a=p.parse_args()
root=pathlib.Path(__file__).resolve().parents[1];sdk=root.parent/'native-windows-sdk'/'SDL2-2.32.10';assets=root/'crates/corpse-desktop/assets'
files=[(a.binary,'corpse-desktop.exe' if a.platform=='windows' else 'corpse-desktop'),(root/'NATIVE-VERIFICATION.md','NATIVE-VERIFICATION.md'),(root/'docs/native-performance-report.md','PERFORMANCE.md')]
# Every current distribution plate is embedded; validate the same ID set as build.rs.
files += [(f,'notices/'+f.name) for f in sorted(assets.glob('*OFL.txt'))]
files += [(assets/'NOTICE.md','notices/assets-NOTICE.md'), (assets/'art-distribution.json','notices/legacy-art-distribution.json'), (assets/'creature-alpha-manifest.json','notices/creature-alpha-manifest.json'), (root/'docs/creature-alpha.md','CREATURE-ALPHA.md'), (root/'docs/native-embedded-art.md','ARTWORK.md')]
files += [(f,'notices/brogue/'+str(f.relative_to(assets/'brogue'))) for f in (assets/'brogue').rglob('*') if f.is_file()]
files += [(f,'notices/crates/'+str(f.relative_to(root/'third-party-notices'))) for f in (root/'third-party-notices').rglob('*') if f.is_file()]
if a.platform=='windows':files += [(sdk/'x86_64-w64-mingw32/bin/SDL2.dll','SDL2.dll'),(sdk/'LICENSE.txt','notices/SDL2-LICENSE.txt')]
source_ids={f.stem for f in (assets/'creatures').glob('*.png')}
runtime_ids={f.stem for f in (assets/'creatures-rgba').glob('*.png')}
if not source_ids or source_ids!=runtime_ids:
 raise RuntimeError(f'Source/runtime plate mismatch: missing={sorted(source_ids-runtime_ids)}, extra={sorted(runtime_ids-source_ids)}')
plate_count=len(runtime_ids)
subprocess.run([sys.executable,str(root/'tools/creature-alpha/pipeline.py'),'verify'],check=True)
manifest={'label':a.label,'platform':a.platform,'windows_runtime_verified':False,'migration_complete':False,'embedded_creature_plates':plate_count,'embedded_creature_ids':sorted(runtime_ids),'artwork_encoding':'RGBA PNG; reviewed segmentation or labelled frozen legacy reference; original masters preserved','files':[]}
a.output.parent.mkdir(parents=True,exist_ok=True)
with zipfile.ZipFile(a.output,'w',zipfile.ZIP_DEFLATED,compresslevel=6) as z:
 for src,dst in files:
  b=src.read_bytes();info=zipfile.ZipInfo(dst);info.create_system=3;info.external_attr=((0o100755 if src==a.binary and a.platform=='linux' else 0o100644)<<16);info.compress_type=zipfile.ZIP_DEFLATED;z.writestr(info,b);manifest['files'].append({'path':dst,'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()})
 z.writestr('CHECKPOINT.json',json.dumps(manifest,indent=2))
 z.writestr('READ-ME.txt',f'''The Call of the Corpse Gods — {a.label}

This is a playable native preview. Unsupported custom/legacy behavior is reported explicitly. Keep the verified browser release as a fallback. Windows builds are cross-compiled, unsigned, and have not been run on Windows. Cross-build and PE inspection are not Windows launch, rendering, input, or save verification.

All {plate_count} creature illustrations are inside the executable. No loose-art setup or extra download is required. The source repository coverage documents describe verified and unported behavior.

WINDOWS (64-bit, Windows 10 or newer)
Extract the whole ZIP into a folder, keep SDL2.dll beside corpse-desktop.exe, then open corpse-desktop.exe. No installer, compiler, or SDK is required. Do not run directly from the ZIP.

LINUX
Extract, install your distribution SDL2 runtime if absent, then run ./corpse-desktop. Exact Linux build/test and native-renderer coverage is recorded in NATIVE-VERIFICATION.md; headless rendering is not interactive-window verification.

GENERAL CONTROLS
Arrows or roguelike movement keys; click known ground to walk; I opens Pack; M opens Bestiary; ? opens the menu and basic display settings; Alt+Enter cycles display modes. F5 saves and F9 resumes normal sessions. New runs use a fresh seed.
''')
print(json.dumps({'path':str(a.output),'bytes':a.output.stat().st_size,'sha256':hashlib.sha256(a.output.read_bytes()).hexdigest()},indent=2))
