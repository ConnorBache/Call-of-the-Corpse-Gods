#!/usr/bin/env python3
"""Auditable native test runner. O(targets + output bytes); pause only between targets."""
import argparse,json,pathlib,subprocess,time,os
p=argparse.ArgumentParser();p.add_argument('artifacts');p.add_argument('destination');p.add_argument('--pause-file',required=True);p.add_argument('--cancel-file');args=p.parse_args()
out=pathlib.Path(args.destination);out.mkdir(parents=True,exist_ok=True)
records=[];manifest=pathlib.Path(args.artifacts)
for line in manifest.read_text().splitlines():
 try:x=json.loads(line)
 except json.JSONDecodeError:continue
 if x.get('reason')=='compiler-artifact' and x.get('executable') and x.get('profile',{}).get('test'):
  key=x['manifest_path']+'::'+x['target']['name'];
  if not any(y['key']==key for y in records):records.append({'key':key,'executable':x['executable'],'cwd':str(pathlib.Path(x['manifest_path']).parent),'target':x['target']['name']})
(out/'targets.json').write_text(json.dumps(records,indent=2));done=[];status=out/'status.json'
for i,x in enumerate(records):
 while pathlib.Path(args.pause_file).exists():
  if args.cancel_file and pathlib.Path(args.cancel_file).exists():
   status.write_text(json.dumps({'state':'canceled_between_targets','completed':done},indent=2));raise SystemExit(0)
  status.write_text(json.dumps({'state':'paused_between_targets','next':x,'completed':done},indent=2));time.sleep(1)
 status.write_text(json.dumps({'state':'running','current':x,'completed':done},indent=2));start=time.time()
 path=out/f'{i:03d}-{x["target"]}.log'
 with path.open('w') as log:r=subprocess.run([x['executable']],cwd=x['cwd'],stdout=log,stderr=subprocess.STDOUT)
 result={**x,'exit_code':r.returncode,'seconds':time.time()-start,'log':str(path)};done.append(result)
 print(json.dumps(result),flush=True)
 if r.returncode:
  status.write_text(json.dumps({'state':'failed','completed':done},indent=2));raise SystemExit(r.returncode)
status.write_text(json.dumps({'state':'passed','completed':done},indent=2))
