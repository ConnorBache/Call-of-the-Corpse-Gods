"""O(total input bytes): assemble immutable receipts, hash files and validate ZIPs.
Compression memory is bounded per file; manifests consume O(file count) memory.
Run only after final report, full tests and Windows receipt are frozen.
"""
import hashlib,json,pathlib,shutil,zipfile,stat
R=pathlib.Path(__file__).resolve().parents[1]; S=R/'source'; P=R/'package'
def sha(p):
    """O(file bytes) time, O(1) streaming memory."""
    h=hashlib.sha256()
    with p.open('rb') as f:
        for b in iter(lambda:f.read(1024*1024),b''):h.update(b)
    return h.hexdigest()
def dump(p,v):
    """O(serialized bytes) time and memory."""
    p.parent.mkdir(parents=True,exist_ok=True);p.write_text(json.dumps(v,indent=2)+'\n')
def copytree(a,b):
    """O(copied bytes); omit only report-render PNGs and Python bytecode."""
    shutil.copytree(a,b,dirs_exist_ok=True,ignore=shutil.ignore_patterns('*.png','__pycache__','*.pyc'))
def paths(root):
    """O(n log n) path sorting; O(n) retained paths."""
    return sorted(p for p in root.rglob('*') if p.is_file() and '__pycache__' not in p.parts and p.suffix!='.pyc')
def archive(name,items):
    """O(input bytes) compression, CRC reread and hash; O(member count) metadata."""
    out=P/f'corpse-gods-native-r39-incremental-save-{name}.zip'
    with zipfile.ZipFile(out,'w',zipfile.ZIP_DEFLATED,compresslevel=6,allowZip64=True) as z:
        for p,rel in items:z.write(p,str(rel))
    with zipfile.ZipFile(out) as z:
        assert z.testzip() is None
        for p,rel in items:
            info=z.getinfo(str(rel));assert ((info.external_attr>>16)&0o777)==stat.S_IMODE(p.stat().st_mode),(rel,'mode')
    return {'path':str(out.relative_to(R)),'bytes':out.stat().st_size,'sha256':sha(out),'members':len(items),'crc_verified':True,'mode_bits_verified':True}
summary=json.loads((R/'reports/full-test-summary.json').read_text());assert summary['all_targets_passed'] and summary['targets']==164
win=json.loads((R/'reports/windows-final-gate.json').read_text())
assert (P/'corpse-gods-native-r39-incremental-save-windows-final.verification.json').exists()
profile=json.loads((R/'reports/windows-build-profile.json').read_text())
for e in profile['files'] + profile['compiled_data_inputs']:
    assert sha(S/e['path'])==e['sha256'],e['path']
history=S/'verification/historical-pre-incremental-save';history.mkdir(parents=True,exist_ok=True)
for name in ['SOURCE-MANIFEST.json','INTEGRATED-VERIFICATION.json','COMPACT-VERIFICATION.json','NATIVE-VERIFICATION.md']:
    if not (history/name).exists():shutil.copy2(S/name,history/name)
V=S/'verification/incremental-save';V.mkdir(parents=True,exist_ok=True)
copytree(R/'reports',V/'reports');copytree(R/'cache-work/reports',V/'cache/reports')
copytree(R/'qa',V/'qa');copytree(R/'cache-work/qa',V/'cache/qa')
E=V/'cache/examples';E.mkdir(parents=True,exist_ok=True)
for name in ['cache_opening_bench.rs','journal_command_bench.rs']:
    shutil.copy2(R/'cache-work/source/crates/corpse-desktop/examples'/name,E/name)
shutil.copy2(P/'corpse-gods-native-r39-incremental-save-windows-final.verification.json',V/'windows-package-verification.json')
report=S/'docs/verification/incremental-save-engineering-report.md'
assert report.exists();shutil.copy2(report,V/'incremental-save-engineering-report.md')
dump(S/'INTEGRATED-VERIFICATION.json',{'revision':39,'scope':'Incremental durable saves, tracked commands, exact closing-edge cache guard','current_report':'docs/verification/incremental-save-engineering-report.md','test_summary':'verification/incremental-save/reports/full-test-summary.json','targets':summary['targets'],'totals':summary['totals'],'profile':summary['profile'],'windows_runtime_verified':False,'power_loss_verified':False,'historical_receipt':'verification/historical-pre-incremental-save/INTEGRATED-VERIFICATION.json'})
windows=P/'corpse-gods-native-r39-incremental-save-windows-final.zip'
dump(S/'COMPACT-VERIFICATION.json',{'path':windows.name,'bytes':windows.stat().st_size,'sha256':sha(windows),'archive_verified':True,'windows_runtime_verified':False,'power_loss_verified':False,'scope':'Current incremental-save cross-build and static/archive checks; no Windows runtime claim','receipt':'verification/incremental-save/windows-package-verification.json','historical_receipt':'verification/historical-pre-incremental-save/COMPACT-VERIFICATION.json'})
(S/'NATIVE-VERIFICATION.md').write_text('# Current incremental-save verification\n\nSee [the engineering report](docs/verification/incremental-save-engineering-report.md) and `verification/incremental-save/`. The assertion-enabled optimized native gate passed '+str(summary['targets'])+' targets: '+str(summary['totals']['passed'])+' passed, '+str(summary['totals']['failed'])+' failed, '+str(summary['totals']['ignored'])+' ignored test executions (not unique tests). Older v18 receipts are preserved unchanged under `verification/historical-pre-incremental-save/`. Windows cross-compilation and static archive verification are not Windows runtime verification. Process-kill tests are not physical power-loss certification.\n')
files=paths(S)
dump(S/'SOURCE-MANIFEST.json',{'revision':39,'description':'Current incremental-save source; every packaged file except this self-referential manifest is hashed. Python bytecode/cache directories excluded. Full assets and fixtures retained. Historical receipts are labeled separately.','files':[{'path':str(p.relative_to(S)),'bytes':p.stat().st_size,'sha256':sha(p),'mode':oct(stat.S_IMODE(p.stat().st_mode))} for p in files if p!=S/'SOURCE-MANIFEST.json']})
manifest=json.loads((S/'SOURCE-MANIFEST.json').read_text())
for e in manifest['files']:assert sha(S/e['path'])==e['sha256']
receipts=[archive('source',[(p,pathlib.Path('corpse-gods-native')/p.relative_to(S)) for p in paths(S)])]
# Reading archive: all source/config/docs scripts, no binary art, fixtures or raw verification.
allowed={'.rs','.toml','.lock','.md','.txt','.py','.sh','.ps1','.bat','.cmd','.h','.c','.html','.css','.js','.mjs','.cjs','.ts','.yml','.yaml','.xml','.rc','.manifest'}
readme=P/'READING-CODE-README.txt';readme.write_text('Incremental-save code reading copy\n\nNOT STANDALONE BUILDABLE: binary artwork/assets, fixtures, raw benchmark evidence, and third-party binary inputs are intentionally omitted. Use the full source ZIP for building and testing. All Rust code, documentation and text tooling are included; the full-source manifest/receipts refer to the full source package, not this reduced copy.\n')
reading=[]
for p in paths(S):
    rel=p.relative_to(S)
    if rel.parts[0] in {'fixtures','verification'}:continue
    if p.suffix in allowed or (p.suffix=='.json' and p.stat().st_size<300000 and 'tests' not in rel.parts):reading.append((p,pathlib.Path('corpse-gods-native')/rel))
reading.append((readme,pathlib.Path('corpse-gods-native/READING-CODE-README.txt')))
receipts.append(archive('code',reading))
receipts.append(archive('evidence',[(p,pathlib.Path('incremental-save-evidence')/p.relative_to(V)) for p in paths(V)]))
dump(P/'source-code-evidence-verification.json',{'artifacts':receipts,'source_manifest_files':len(manifest['files']),'source_manifest_all_hashes_verified':True,'windows_production_profile_entries_verified':len(profile['files'])+len(profile['compiled_data_inputs']),'historical_receipts_preserved':True,'report_sha256':sha(report),'excluded':'Toolchains, targets, input downloads, upload helpers/auth metadata, report-render PNGs and Python bytecode are not packaged.'})
print(json.dumps(receipts,indent=2),flush=True)
