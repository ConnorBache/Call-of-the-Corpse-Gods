#!/usr/bin/env python3
"""O(C*N log N + evidence bytes) deterministic offline summaries; no timed work."""
from pathlib import Path
import json,statistics,math,hashlib
root=Path(__file__).resolve().parents[1];out=root/'reports'
merged={}
for implementation in ['baseline','candidate']:
    cases={}
    for run in [1,2]:
        report=json.loads((out/f'{implementation}-{run}.json').read_text())
        for row in report['cases']:
            dst=cases.setdefault(row['case'],{'bytes':row['bytes'],'starting_frames':row['starting_frames']})
            for key in ['command_inclusive','immediate_save_inclusive','command_plus_save_inclusive']:
                dst.setdefault(key,[]).extend(row[key]['samples_ms'])
    merged[implementation]=cases

def stats(values):
    """O(N log N) sorting; midpoint median and nearest-rank p95."""
    values=sorted(values);return {'n':len(values),'median_ms':statistics.median(values),'p95_ms':values[math.ceil(.95*len(values))-1],'min_ms':values[0],'max_ms':values[-1],'samples_ms':values}

result=[]
for name in merged['baseline']:
    a=merged['baseline'][name];b=merged['candidate'][name];assert a['bytes']==b['bytes']
    before=(out/'states-baseline'/f'{name}.json').read_bytes();after=(out/'states-candidate'/f'{name}.json').read_bytes();assert before==after,name
    row={'case':name,'bytes':len(before),'canonical_sha256':hashlib.sha256(before).hexdigest(),'exact_state_equal':True}
    for key in ['command_inclusive','immediate_save_inclusive','command_plus_save_inclusive']:
        row[key]={'baseline':stats(a[key]),'candidate':stats(b[key])}
    result.append(row)
profile_file=out/'candidate-profiled.json'
if profile_file.exists():
    profile=json.loads(profile_file.read_text())
    prof={row['case']:row['profile_reports'] for row in profile['cases']}
    for row in result:
        rows=prof[row['case']]
        for entry in rows:
            assert sum(v['exclusive_ns'] for v in entry.values())==entry['benchmark.command_plus_save']['inclusive_ns']
        row['profile_checks']=[{key:entry.get(key,{'calls':0,'inclusive_ns':0,'exclusive_ns':0}) for key in ['history.opening_identity_guard','history.opening_identity_reuse','history.export_cache_hit','history.journal_project','history.journal_seal']} for entry in rows]
(out/'summary.json').write_text(json.dumps(result,indent=2))
for row in result:
    total=row['command_plus_save_inclusive'];command=row['command_inclusive'];save=row['immediate_save_inclusive']
    print(row['case'],f"total {total['baseline']['median_ms']:.3f}->{total['candidate']['median_ms']:.3f} p95 {total['candidate']['p95_ms']:.3f}; command {command['baseline']['median_ms']:.3f}->{command['candidate']['median_ms']:.3f}; save {save['baseline']['median_ms']:.3f}->{save['candidate']['median_ms']:.3f}")
