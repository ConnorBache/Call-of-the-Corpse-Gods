"""Reproduce recorded native controlled-trade cases with actual engine dispatch.
Requires NumPy and a built physical_trade_arena example. No shipping-game dependency.
Use --binary PATH when CARGO_TARGET_DIR is not the ordinary workspace target.
Censored means are partial observations, never completed encounter costs.
"""
from pathlib import Path
import argparse,json,math,subprocess,time
import numpy as np
ROOT=Path(__file__).resolve().parents[2]
p=argparse.ArgumentParser()
p.add_argument('--binary',type=Path,default=ROOT/'target/release/examples/physical_trade_arena')
p.add_argument('--cases',type=Path,default=ROOT/'docs/artifacts/physical-balance-v31-native-cases.json')
p.add_argument('--output',type=Path,default=ROOT/'docs/artifacts/physical-balance-v31-native-reproduced.json')
p.add_argument('--limit',type=int,default=0,help='Optional smoke subset; 0 reproduces every recorded case.')
args=p.parse_args()
profiles=np.load(ROOT/'fixtures/physical-reference-profiles-v31.npz')['entry_profiles'].astype(float)
order=np.random.default_rng(310043).permutation(10000)
cases=json.loads(args.cases.read_text());cases=cases[:args.limit]if args.limit else cases
proc=subprocess.Popen([str(args.binary)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True,bufsize=1)
rows=[];started=time.time()
try:
 for case in cases:
  n=case['trials'];depth=case['reference_depth'];lo=math.floor(depth)-1;hi=math.ceil(depth)-1
  indices=order[(np.arange(n)+4096)%len(order)];floors=np.where(np.arange(n)%2==0,lo,hi)
  x=profiles[[50,75,100].index(case['collection_pct']),indices,floors]
  request={k:v for k,v in case.items()if k not in ['reference_depth','collection_pct','mode']}
  request.update(revision=31,profiles=x.tolist())
  proc.stdin.write(json.dumps(request)+'\n');proc.stdin.flush()
  line=proc.stdout.readline()
  if not line:raise RuntimeError('Native arena process ended before returning a result')
  r=json.loads(line)
  if 'error'in r:raise RuntimeError(r['error'])
  r.update(reference_depth=depth,collection_pct=case['collection_pct'],mode=case['mode'])
  rows.append(r)
  print(case['kind'],case['scenario'],case['collection_pct'],case['mode'],'net',round(100*r['mean_cost'],3),'deaths',r['deaths'],'censored',r['censored'],flush=True)
  args.output.write_text(json.dumps(rows,indent=2)+'\n')
finally:
 proc.stdin.close();proc.wait()
print('Native cases',len(rows),'trials',sum(r['trials']for r in rows),'seconds',time.time()-started)
