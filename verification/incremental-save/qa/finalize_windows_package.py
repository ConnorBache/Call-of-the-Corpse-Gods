#!/usr/bin/env python3
"""Add release notes without changing binaries; O(total archive bytes)."""
import argparse,hashlib,json,zipfile
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--base',type=Path,required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--notes',type=Path,required=True);p.add_argument('--gate-note',type=Path);a=p.parse_args()
assert not a.output.exists(),'Refusing overwrite'
with zipfile.ZipFile(a.base) as z:
 assert z.testzip() is None
 files={n:z.read(n) for n in z.namelist()}
m=json.loads(files.pop('MANIFEST.json'));files['SAVES-AND-RECOVERY.txt']=a.notes.read_bytes()
if a.gate_note:files['VERIFICATION.txt']=a.gate_note.read_bytes()
m['files']=[{'path':n,'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()} for n,b in sorted(files.items())]
files['MANIFEST.json']=(json.dumps(m,indent=2,sort_keys=True)+'\n').encode()
with zipfile.ZipFile(a.output,'x',zipfile.ZIP_DEFLATED,compresslevel=9) as z:
 for n,b in sorted(files.items()):
  i=zipfile.ZipInfo(n,date_time=(1980,1,1,0,0,0));i.create_system=3;i.external_attr=0o100644<<16;i.compress_type=zipfile.ZIP_DEFLATED;z.writestr(i,b,compresslevel=9)
with zipfile.ZipFile(a.output) as z:
 assert z.testzip() is None and set(z.namelist())==set(files)
 for n,b in files.items():assert z.read(n)==b,n
assert a.output.stat().st_size<20_000_000,'Exceeds compact package budget'
r={'path':str(a.output),'bytes':a.output.stat().st_size,'sha256':hashlib.sha256(a.output.read_bytes()).hexdigest(),'archive_verified':True,'windows_runtime_verified':False,'base_archive_sha256':hashlib.sha256(a.base.read_bytes()).hexdigest(),'binary_and_asset_bytes_unchanged':True}
a.output.with_suffix('.verification.json').write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(r,indent=2))
