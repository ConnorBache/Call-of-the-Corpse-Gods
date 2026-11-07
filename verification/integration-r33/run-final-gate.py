#!/usr/bin/env python3
"""Frozen integration gate. Only launch after the parent explicitly freezes owners."""
import datetime, hashlib, json, os, pathlib, re, subprocess, time
ROOT=pathlib.Path(__file__).resolve().parents[2]
OUT=ROOT/'verification/integration-r33'
def digest(path): return hashlib.sha256(path.read_bytes()).hexdigest()
def snapshot():
    paths=[]
    for folder in ['crates','fixtures','tools']:
        paths += [p for p in (ROOT/folder).rglob('*') if p.is_file() and '__pycache__' not in p.parts]
    paths += [ROOT/name for name in ['Cargo.toml','Cargo.lock','rust-toolchain.toml','AGENTS.md']]
    return {str(p.relative_to(ROOT)):digest(p) for p in sorted(set(paths))}
def write(name,obj): (OUT/name).write_text(json.dumps(obj,indent=2)+'\n')
def fingerprint(rows): return hashlib.sha256(json.dumps(rows,sort_keys=True,separators=(',',':')).encode()).hexdigest()
def now():return datetime.datetime.now(datetime.timezone.utc).isoformat()
env=dict(os.environ,CARGO_INCREMENTAL='0',CARGO_PROFILE_TEST_OPT_LEVEL='2',CARGO_PROFILE_TEST_DEBUG='0',CARGO_PROFILE_TEST_DEBUG_ASSERTIONS='true',CARGO_PROFILE_TEST_OVERFLOW_CHECKS='true')
before=snapshot();write('frozen-inputs-before.json',before)
receipt={'started_utc':now(),'input_scope':'All files under crates, fixtures, tools (excluding __pycache__), plus Cargo.toml, Cargo.lock, rust-toolchain.toml and AGENTS.md. Generated verification and release documentation are excluded.','input_files':len(before),'input_fingerprint_before':fingerprint(before),'environment':{k:env[k] for k in ['CARGO_INCREMENTAL','CARGO_PROFILE_TEST_OPT_LEVEL','CARGO_PROFILE_TEST_DEBUG','CARGO_PROFILE_TEST_DEBUG_ASSERTIONS','CARGO_PROFILE_TEST_OVERFLOW_CHECKS']},'checks':[], 'cargo_version':subprocess.check_output(['sh','tools/cargo.sh','--version'],cwd=ROOT,env=env,text=True).strip(), 'rust_toolchain':(ROOT/'rust-toolchain.toml').read_text().strip()}
for label,extra in [('workspace-all-targets',['--all-targets']),('workspace-doc',['--doc'])]:
    command=['sh','tools/cargo.sh','test','--workspace','--locked']+extra
    start=time.monotonic();log=OUT/f'final-{label}.log'
    with log.open('w') as handle: code=subprocess.call(command,cwd=ROOT,env=env,stdout=handle,stderr=subprocess.STDOUT)
    text=log.read_text();cases=re.findall(r'^test (.+) \.\.\. (ok|FAILED|ignored)',text,re.M)
    row={'label':label,'command':' '.join(command),'exit_code':code,'seconds':round(time.monotonic()-start,3),'passed':sum(v=='ok' for _,v in cases),'failed':sum(v=='FAILED' for _,v in cases),'ignored':sum(v=='ignored' for _,v in cases),'failed_tests':[name for name,status in cases if status=='FAILED'],'ignored_tests':[name for name,status in cases if status=='ignored'],'log':str(log.relative_to(ROOT)),'log_sha256':digest(log),'count_method':'Only actual test NAME ... STATUS lines; summary lines are not tests.'}
    receipt['checks'].append(row);write('final-gate-receipt.json',receipt);print(json.dumps(row),flush=True)
    if code: break
after=snapshot();write('frozen-inputs-after.json',after)
receipt.update(finished_utc=now(),input_fingerprint_after=fingerprint(after),changed_inputs=[p for p in sorted(before.keys()|after.keys()) if before.get(p)!=after.get(p)])
receipt['code_build_inputs_unchanged']=before==after
receipt['passed']=len(receipt['checks'])==2 and all(c['exit_code']==0 and c['failed']==0 for c in receipt['checks']) and before==after
write('final-gate-receipt.json',receipt);print(json.dumps({k:receipt[k] for k in ['finished_utc','passed','code_build_inputs_unchanged','changed_inputs']}),flush=True)
raise SystemExit(0 if receipt['passed'] else 1)
