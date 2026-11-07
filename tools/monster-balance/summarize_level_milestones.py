#!/usr/bin/env python3
"""First-earned level depths from executed heart_progression_audit JSONL.
No missing seed replacement; all_initial_unblessed includes initial Nest/Godbeast.
This does not apply the old ordinary-budget invariant to conditional population.
"""
import argparse,json,statistics
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('input',type=Path);p.add_argument('output',type=Path);a=p.parse_args()
records=[json.loads(line)for line in a.input.read_text().splitlines()if line.strip()]
assert len({r['index']for r in records})==len(records),'duplicate seed index'
worlds=[r for r in records if 'floors'in r]
failures=[r for r in records if 'error'in r]
assert len(worlds)+len(failures)==len(records)
for r in worlds:
 assert [f['depth']for f in r['floors']]==list(range(1,41))
 for f in r['floors']:
  x=f['exit']['all_initial_unblessed'];assert 2**x['level']-2<=x['blessings']<2**(x['level']+1)-2
maximum=max(int(f['exit']['all_initial_unblessed']['level'])for r in worlds for f in r['floors'])
rows=[]
for level in range(2,maximum+2):
 values=[next((f['depth']for f in r['floors']if f['exit']['all_initial_unblessed']['level']>=level),None)for r in worlds]
 reached=[d for d in values if d is not None]
 rows.append({'level':level,'median_floor':statistics.median(reached)if reached else None,'min_floor':min(reached)if reached else None,'max_floor':max(reached)if reached else None,'worlds_reached':len(reached),'worlds_not_reached':len(worlds)-len(reached)})
result={'input':str(a.input),'revisions':sorted({r['revision']for r in records}),'requested_worlds':len(records),'successful_worlds':len(worlds),'indices':[r['index']for r in records], 'failed_worlds':failures,'definition':'Level first earned DURING each floor, after collecting every initial natural heart once; not floor-entry or survival. Includes initial Nest/Godbeast hearts. Excludes finite summoned/event extras. Level 1 is the start.','milestones':rows}
a.output.write_text(json.dumps(result,indent=2)+'\n')
print(f"{len(worlds)}/{len(records)} generated worlds; {len(failures)} failures retained")
for r in rows:print(f"L{r['level']}: median {r['median_floor']}, range {r['min_floor']}–{r['max_floor']}, reached {r['worlds_reached']}/{len(worlds)}")
