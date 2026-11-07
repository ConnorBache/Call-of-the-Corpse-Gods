import argparse,hashlib,json,zipfile,struct
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--zip',required=True);p.add_argument('--binary',required=True);p.add_argument('--output',required=True);a=p.parse_args();path=Path(a.zip);rawpath=Path(a.binary)
def sha(b):return hashlib.sha256(b).hexdigest()
z=zipfile.ZipFile(path);assert z.testzip() is None;names=z.namelist();assert len(names)==len(set(names));assert all(not n.startswith('/') and '..' not in Path(n).parts for n in names)
assert 'corpse-desktop.exe' in names and 'SDL2.dll' in names
assert all(n in ['corpse-desktop.exe','SDL2.dll','START-HERE.txt','MANIFEST.json'] or n.startswith('notices/') for n in names),names
manifest=json.loads(z.read('MANIFEST.json'))
for e in manifest['files']:
 b=z.read(e['path']);assert len(b)==e['bytes'] and sha(b)==e['sha256'],e
exe=z.read('corpse-desktop.exe');raw=rawpath.read_bytes();assert sha(raw)==manifest['original_binary']['sha256'];assert sha(exe)==manifest['stripped_binary']['sha256']
def sections(data):
 assert data[:2]==b'MZ';off=struct.unpack_from('<I',data,60)[0];assert data[off:off+4]==b'PE\0\0';machine,count=struct.unpack_from('<HH',data,off+4);assert machine==0x8664;opt=struct.unpack_from('<H',data,off+20)[0];sym,nsym=struct.unpack_from('<II',data,off+12);s={}
 for i in range(count):
  p=off+24+opt+40*i;name=data[p:p+8].rstrip(b'\0').decode();size,start=struct.unpack_from('<II',data,p+16);flags=struct.unpack_from('<I',data,p+36)[0]
  if name.startswith('/') and name[1:].isdigit():
   q=sym+nsym*18+int(name[1:]);name=data[q:data.index(b'\0',q)].decode()
  if flags&0xe0000000 and not name.startswith(('.debug','.zdebug')):s[name]=sha(data[start:start+size]) if start else sha(b'')
 return s
assert sections(raw)==sections(exe)
assets=Path('compact-r39/source/corpse-gods-native/crates/corpse-desktop/assets');art=[]
for f in sorted((assets/'creatures-runtime').glob('*.webp')):
 b=f.read_bytes();count=exe.count(b);assert count>=1,f;assert raw.count(b)>=1,f;art.append({'name':f.stem,'bytes':len(b),'sha256':sha(b),'occurrences_in_packaged_exe':count})
assert len(art)==84
old_present=[]
for f in (assets/'creatures-rgba').glob('*.png'):
 if f.read_bytes() in exe:old_present.append(f.name)
assert not old_present,old_present
r={'status':'passed','zip':str(path),'zip_bytes':path.stat().st_size,'zip_sha256':sha(path.read_bytes()),'archive_member_count':len(names),'zip_crc_verified':True,'manifest_files_hash_verified':len(manifest['files']),'original_binary_sha256':sha(raw),'packaged_binary_sha256':sha(exe),'loadable_pe_sections_hashes_unchanged':sections(exe),'runtime_art_embedded_count':len(art),'original_canonical_pngs_not_embedded':True,'members':names,'art':art,'windows_runtime_tested':False}
Path(a.output).write_text(json.dumps(r,indent=2));print(json.dumps({k:v for k,v in r.items() if k not in ['art','members','loadable_pe_sections_hashes_unchanged']}))
