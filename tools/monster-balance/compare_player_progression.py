#!/usr/bin/env python3
"""Paired-seed comparison of two frozen actual-generator heart audits.
O(S*D*T*K) arithmetic; O(S*D*T*K) raw input memory; D40,T4,K13 fixed.
Never compares unmatched worlds or silently replaces failed generation seeds.
"""
import csv, hashlib, json, statistics, math, sys
from pathlib import Path
from summarize_player_progression import TRACKS,METRICS,value

def read(p):
    rows=[json.loads(l) for l in p.read_text().splitlines() if l.strip()]
    return {r['index']:r for r in rows if 'error' not in r},[r for r in rows if 'error' in r]
def main():
    before_path,after_path,prefix=map(Path,sys.argv[1:4])
    before,bad_before=read(before_path);after,bad_after=read(after_path)
    common=sorted(before.keys()&after.keys());assert common
    rows=[]
    for d in range(40):
        for track in TRACKS:
            for phase in ('entry','exit'):
                for metric in METRICS:
                    b=[value(before[i]['floors'][d][phase][track],metric) for i in common]
                    a=[value(after[i]['floors'][d][phase][track],metric) for i in common]
                    delta=[y-x for x,y in zip(b,a)]
                    mean=statistics.mean(delta);sd=statistics.stdev(delta) if len(delta)>1 else 0
                    width=1.96*sd/math.sqrt(len(delta))
                    rows.append(dict(depth=d+1,profile=track,phase=phase,metric=metric,n=len(common),
                        before_mean=statistics.mean(b),after_mean=statistics.mean(a),delta_mean=mean,
                        delta_sd=sd,delta_ci95_low=mean-width,delta_ci95_high=mean+width))
    with Path(str(prefix)+'.csv').open('w',newline='') as f:
        w=csv.DictWriter(f,fieldnames=rows[0]);w.writeheader();w.writerows(rows)
    manifest={'paired_n':len(common),'paired_seed_indices':common,'paired_seeds':[before[i]['seed'] for i in common],
        'before_only':sorted(before.keys()-after.keys()),'after_only':sorted(after.keys()-before.keys()),
        'before_failures':bad_before,'after_failures':bad_after,
        'inputs':{str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in (before_path,after_path)},
        'rows':rows,'scope':'Whole-world successful pairs only. No replacement seeds. Means conditional on generation success; 95% normal-approximation sampling intervals on paired differences, not player survival intervals.'}
    assert all(before[i]['seed']==after[i]['seed'] for i in common)
    Path(str(prefix)+'.json').write_text(json.dumps(manifest,indent=2)+'\n')
    print(json.dumps({k:manifest[k] for k in ('paired_n','before_only','after_only','before_failures','after_failures')}))
if __name__=='__main__':main()
