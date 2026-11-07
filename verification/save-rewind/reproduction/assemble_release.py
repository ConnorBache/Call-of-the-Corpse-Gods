#!/usr/bin/env python3
"""Stage verified save/rewind source. O(total bytes) I/O, O(file count) metadata.
Default is a read-only plan. --assemble requires final report and fresh output dir.
No Library operations, builds, source mutations, or deletion of existing outputs.
"""
from pathlib import Path
import argparse, hashlib, json, os, shutil, zipfile

ROOT=Path(__file__).resolve().parents[1]
BASE=ROOT.parent/'visibility-r39/source'
SOURCE=ROOT/'source'
CODE_OVERLAYS=[
 'crates/corpse-core/src/'+name+'.rs' for name in (
 'corpse_placement','creature_index','engine','equipment_abilities','finale',
 'fire_adapter','half_here','item_magic','item_placement','lib','migration_pipeline',
 'migrations','mini_me','model','offscreen','startup','temporal','travel','vision',
 'water_blood','zillennia_ground','mutation_journal')
]+[
 'crates/corpse-core/tests/journal_projection.rs',
 'crates/corpse-desktop/src/session_store.rs',
 'crates/corpse-desktop/examples/journal_command_bench.rs',
 'crates/corpse-desktop/examples/journal_temporal_bench.rs',
 'crates/corpse-desktop/examples/save_envelope_bench.rs',
 'crates/corpse-desktop/examples/save_envelope_bench/frozen_history.rs',
]
DOC_OVERLAYS=['docs/'+name for name in (
 'mutation-journal-foundation.md','save-envelope-complexity.md','save-rewind-export-complexity.md')]
HISTORICAL=['SOURCE-MANIFEST.json','COMPACT-VERIFICATION.json','INTEGRATED-VERIFICATION.json','NATIVE-VERIFICATION.md','CREATURE-ALPHA-VERIFICATION.txt']
EXCLUDED={'target','.git','__pycache__','.pytest_cache'}
LEDGER_MARK='### Save and rewind export reuse (October 2026)'
READ_EXT={'.rs','.md','.toml','.lock','.py','.sh','.json','.txt','.html','.css','.js','.ts',''}


def sha(path):
    """O(file bytes), O(1 MiB) scratch."""
    h=hashlib.sha256()
    with path.open('rb') as stream:
        while block:=stream.read(1048576):h.update(block)
    return h.hexdigest()


def files(root):
    """O(files+directories); follow asset/fixture links, reject symlink cycles."""
    def walk(directory,rel,ancestors):
        real=directory.resolve()
        if real in ancestors:raise RuntimeError(f'Symlink directory cycle: {directory}')
        for path in sorted(directory.iterdir()):
            if path.name in EXCLUDED:continue
            relative=rel/path.name
            if path.is_dir():yield from walk(path,relative,ancestors|{real})
            elif path.is_file():yield relative,path
            else:raise RuntimeError(f'Unsupported or broken source entry: {path}')
    yield from walk(root,Path(),set())


def code_diff():
    """O(code/config bytes), ignoring immutable heavy payload and verification."""
    def index(root):
        return {rel.as_posix():sha(path) for rel,path in files(root)
                if rel.parts[0]!='verification' and 'fixtures' not in rel.parts and 'assets' not in rel.parts}
    a,b=index(BASE),index(SOURCE)
    changed=sorted(k for k in a.keys()&b.keys() if a[k]!=b[k])
    return {'changed':changed,'only_baseline':sorted(a.keys()-b.keys()),'only_candidate':sorted(b.keys()-a.keys())}


def manifest(root,description,selected=None):
    """O(selected bytes), O(file count) manifest; excludes itself explicitly."""
    rows=[]
    for rel,path in files(root):
        name=rel.as_posix()
        if name=='SOURCE-MANIFEST.json' or (selected is not None and name not in selected):continue
        rows.append({'path':name,'bytes':path.stat().st_size,'sha256':sha(path)})
    return {'revision':39,'description':description,'files':rows}



def code_fingerprint(root):
    """O(Rust/Cargo input bytes), excluding assets/fixtures; stable path+hash digest."""
    rows=[{'path':rel.as_posix(),'sha256':sha(path)} for rel,path in files(root)
          if 'fixtures' not in rel.parts and 'assets' not in rel.parts
          and (path.suffix=='.rs' or path.name in {'Cargo.toml','Cargo.lock','rust-toolchain.toml'})]
    rows.sort(key=lambda row:row['path'])
    digest=hashlib.sha256(''.join(row['path']+'\0'+row['sha256']+'\n' for row in rows).encode()).hexdigest()
    return {'algorithm':'sha256(sorted path + NUL + per-file sha256 + newline)','sha256':digest,'files':rows}


def main():
    """O(source bytes) staging/hash/archive; no build or executable invocation."""
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assemble',action='store_true')
    parser.add_argument('--output',type=Path,default=ROOT/'package-final')
    parser.add_argument('--extra-root',type=Path,help='Final report/receipts overlay, docs/ and verification/ only')
    parser.add_argument('--report-path',default='docs/SAVE-REWIND-PERFORMANCE.md')
    parser.add_argument('--expected-code-sha256',required=False,help='Final build input fingerprint, using code_fingerprint() format')
    args=parser.parse_args()
    diff=code_diff()
    expected=set(CODE_OVERLAYS+DOC_OVERLAYS+['SOURCE-MANIFEST.json','docs/complexity-ledger.md','docs/painting-bulk-audit.md'])
    unexpected=[p for p in diff['changed']+diff['only_candidate'] if p not in expected]
    if unexpected:raise RuntimeError('Unreviewed candidate changes: '+repr(unexpected))
    for name in CODE_OVERLAYS+DOC_OVERLAYS:
        if not (SOURCE/name).is_file():raise RuntimeError('Missing approved overlay: '+name)
    plan={'baseline':str(BASE),'candidate':str(SOURCE),'diff':diff,'code_overlays':CODE_OVERLAYS,
          'document_overlays':DOC_OVERLAYS,'ledger_policy':'preserve published ledger; append save suffix only',
          'baseline_docs_preserved':['docs/painting-bulk-audit.md','docs/VISIBILITY-PAINT-PERFORMANCE.md','docs/painting-bulk-verification.md','verification/visibility-paint/'],
          'historical_top_level':HISTORICAL,'output':str(args.output),'assemble':args.assemble}
    print(json.dumps(plan,indent=2))
    if not args.assemble:return
    if args.output.exists():raise RuntimeError('Output already exists; use a fresh directory (nothing deleted)')
    if args.extra_root is None:raise RuntimeError('--extra-root containing final report/receipts is required')
    if not args.expected_code_sha256:raise RuntimeError('--expected-code-sha256 from final build provenance is required')
    extras=list(files(args.extra_root))
    if any(rel.parts[0] not in {'docs','verification'} for rel,_ in extras):raise RuntimeError('Extra overlay may contain docs/ or verification/ only')
    if not (args.extra_root/args.report_path).is_file():raise RuntimeError('Final report is missing from extra overlay')
    stage=args.output/'stage/corpse-gods-native'
    stage.mkdir(parents=True)
    # copy2 dereferences source file symlinks; directory traversal above materializes links.
    for rel,path in files(BASE):
        destination=stage/rel;destination.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(path,destination)
    for name in CODE_OVERLAYS+DOC_OVERLAYS:
        destination=stage/name;destination.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(SOURCE/name,destination)
    ledger=(SOURCE/'docs/complexity-ledger.md').read_text()
    if LEDGER_MARK not in ledger:raise RuntimeError('Expected appended save complexity section missing')
    baseline_ledger=(BASE/'docs/complexity-ledger.md').read_text()
    if LEDGER_MARK in baseline_ledger:raise RuntimeError('Baseline unexpectedly already contains save section; review merge')
    suffix=ledger[ledger.index(LEDGER_MARK):]
    (stage/'docs/complexity-ledger.md').write_text(baseline_ledger.rstrip()+'\n\n'+suffix)
    for rel,path in extras:
        destination=stage/rel;destination.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(path,destination)
    historical=stage/'verification/historical-pre-save-journal'
    historical.mkdir(parents=True,exist_ok=True)
    for name in HISTORICAL:
        source=stage/name
        if source.exists():source.rename(historical/name)
    (historical/'README.md').write_text('# Historical receipts\n\nThese top-level files describe earlier builds, not the current save/rewind candidate. They are preserved unchanged for provenance. Their relative paths refer to the earlier source root.\n')
    (stage/'NATIVE-VERIFICATION.md').write_text('# Current native save/rewind verification\n\nSee ['+args.report_path+']('+args.report_path+') for the actual final gates, measured scope and limits. Older build receipts are preserved in `verification/historical-pre-save-journal/`; they are not current pass counts. Windows cross-compilation/static inspection is not Windows runtime verification.\n')
    # Keep historical document links live without presenting old counts as current.
    tests=json.loads((stage/'verification/save-rewind/full-test-summary.json').read_text())
    (stage/'INTEGRATED-VERIFICATION.json').write_text(json.dumps({'revision':39,'scope':'save/rewind coarse journal final native executable test gate','current_report':args.report_path,'test_summary':'verification/save-rewind/full-test-summary.json','totals':tests['totals'],'targets':tests['targets'],'windows_runtime_verified':False,'historical_receipt':'verification/historical-pre-save-journal/INTEGRATED-VERIFICATION.json'},indent=2)+'\n')
    compact=json.loads((stage/'verification/save-rewind/windows-package.log').read_text())
    compact['scope']='Current save/rewind compact Windows cross-build and archive/static validation only'
    compact['historical_receipt']='verification/historical-pre-save-journal/COMPACT-VERIFICATION.json'
    (stage/'COMPACT-VERIFICATION.json').write_text(json.dumps(compact,indent=2)+'\n')
    provenance=code_fingerprint(stage)
    candidate_provenance=code_fingerprint(SOURCE)
    if provenance!=candidate_provenance:raise RuntimeError('Staged Rust/Cargo inputs do not match tested candidate')
    if provenance['sha256']!=args.expected_code_sha256:raise RuntimeError('Staged Rust/Cargo inputs do not match final build fingerprint')
    provenance_path=stage/'verification/save-rewind/source-code-provenance.json'
    provenance_path.parent.mkdir(parents=True,exist_ok=True)
    provenance_path.write_text(json.dumps(provenance,indent=2)+'\n')
    # Keep gameplay revision39 identity; optimization does not invent a new save version.
    data=manifest(stage,'Revision39 save/rewind first-write COW evidence, dirty-block sealing and exact inverse reuse. Current gates/limits: '+args.report_path+'. Historical reports retained. This self-referential manifest excluded.')
    (stage/'SOURCE-MANIFEST.json').write_text(json.dumps(data,indent=2)+'\n')
    results=[]
    for reading in [False,True]:
        destination=args.output/('corpse-gods-native-r39-save-rewind-reading.zip' if reading else 'corpse-gods-native-r39-save-rewind-source.zip')
        selected=[(rel,path) for rel,path in files(stage) if not reading or
                  (rel.parts[0] not in {'fixtures','verification','third-party-notices'} and 'fixtures' not in rel.parts and 'assets' not in rel.parts and path.suffix.lower() in READ_EXT)]
        with zipfile.ZipFile(destination,'w',zipfile.ZIP_DEFLATED,compresslevel=6) as archive:
            for rel,path in selected:
                if reading and rel.as_posix()=='SOURCE-MANIFEST.json':continue
                archive.write(path,'corpse-gods-native/'+rel.as_posix())
            if reading:
                names={rel.as_posix() for rel,_ in selected}
                reading_manifest=manifest(stage,'READING ONLY: assets, fixtures and verification payloads omitted; NOT buildable. Manifest lists only this reading companion.',names)
                archive.writestr('corpse-gods-native/SOURCE-MANIFEST.json',json.dumps(reading_manifest,indent=2)+'\n')
                archive.writestr('READING-ONLY.txt','Code-reading companion only. Assets and test fixtures are omitted; this archive is NOT buildable. Use the full source archive to build and run tests.\n')
        with zipfile.ZipFile(destination) as archive:
            if archive.testzip() is not None:raise RuntimeError('Archive CRC verification failed')
            if any((info.external_attr>>16)&0o170000==0o120000 for info in archive.infolist()):raise RuntimeError('Archive contains a symlink')
        results.append({'path':str(destination),'bytes':destination.stat().st_size,'sha256':sha(destination),'archive_verified':True,'reading_only':reading})
    (args.output/'assembly-plan.json').write_text(json.dumps(plan,indent=2)+'\n')
    (args.output/'source-packages.json').write_text(json.dumps(results,indent=2)+'\n')
    print(json.dumps(results,indent=2))

if __name__=='__main__':main()
