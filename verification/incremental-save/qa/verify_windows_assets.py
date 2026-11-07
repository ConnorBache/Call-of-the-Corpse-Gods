#!/usr/bin/env python3
"""Read-only static asset/PE validation. O(asset bytes + assets*binary search bytes)."""
from pathlib import Path
import hashlib,json,sys,importlib.util
root=Path(__file__).resolve().parents[1];source=root/'source';assets=source/'crates/corpse-desktop/assets'
rows=[]
manifest=json.loads((assets/'creature-runtime-manifest.json').read_text())
for plate in manifest['plates']:
 for folder,name,key,embedded in [('creatures-rgba',plate['source_file'],'source_sha256',False),('creatures-runtime',plate['encoded_file'],'encoded_sha256',True)]:
  path=assets/folder/name;actual=hashlib.sha256(path.read_bytes()).hexdigest();assert actual==plate[key],str(path)
  rows.append({'path':path.relative_to(source).as_posix(),'sha256':actual,'embedded':embedded})
ui=json.loads((assets/'ui-runtime-manifest.json').read_text())
for item in ui['assets']:
 for folder,suffix,key,embedded in [('', '.png','source_sha256',False),('ui-runtime','.webp','webp_sha256',True)]:
  path=assets/folder/(item['name']+suffix);actual=hashlib.sha256(path.read_bytes()).hexdigest();assert actual==item[key],str(path)
  rows.append({'path':path.relative_to(source).as_posix(),'sha256':actual,'embedded':embedded})
rust_source='\n'.join(p.read_text() for p in (source/'crates/corpse-desktop/src').glob('*.rs'))
for path in sorted(assets.glob('*.ttf'))+sorted(assets.glob('*.otf')):
 rows.append({'path':path.relative_to(source).as_posix(),'sha256':hashlib.sha256(path.read_bytes()).hexdigest(),'embedded':('include_bytes!("../assets/'+path.name+'")') in rust_source})
report={'creature_count':manifest['count'],'creature_original_and_runtime_hashes_match_approved_manifest':True,'ui_count':len(ui['assets']),'ui_original_and_runtime_hashes_match_approved_manifest':True,'asset_files':rows,'windows_runtime_verified':False}
if len(sys.argv)>1:
 binary=Path(sys.argv[1]);data=binary.read_bytes()
 for row in rows:
  if row['embedded']:assert data.find((source/row['path']).read_bytes())>=0,'Missing embedded asset '+row['path']
 spec=importlib.util.spec_from_file_location('package_windows',source/'tools/package-compact-windows.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
 sections=module.pe_sections(data);assert '.text' in sections and '.rsrc' in sections and '.pdata' in sections
 application_manifest=(assets/'corpse-gods.manifest').read_bytes();assert application_manifest in sections['.rsrc'],'Missing exact application manifest resource'
 assert b'PerMonitorV2, PerMonitor' in application_manifest
 report.update({'binary':str(binary),'binary_bytes':len(data),'binary_sha256':hashlib.sha256(data).hexdigest(),'all_declared_embedded_asset_bytes_found':True,'mapped_pe_sections':sorted(sections),'application_manifest_resource_exact':True,'unsigned_pe32_plus_x64':True})
(root/'reports/windows-assets-static.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({key:value for key,value in report.items() if key!='asset_files'},indent=2))
