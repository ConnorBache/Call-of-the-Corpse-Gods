#!/usr/bin/env python3
"""Extract actual r33 joint profiles, recording additive Nest hearts separately.
The old r32 strict ordinary-budget summarizer remains unchanged. O(S*40*4).
"""
import argparse,hashlib,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];P=ROOT/'tools/monster-balance'
args=argparse.ArgumentParser();args.add_argument('--input',type=Path,default=P/'r33-refresh-eight-worlds.jsonl');args.add_argument('--prefix',type=Path,default=P/'r33-refresh-eight-worlds');opts=args.parse_args()
src=opts.input;records=[json.loads(x) for x in src.read_text().splitlines()];samples=[x for x in records if 'error' not in x];events=[]
for s in samples:
 assert len(s['floors'])==40
 for f in s['floors']:
  extra=f['ordinary_blessings']-f['ordinary_budget'];queen=f['species'].get('saidribee-queen',0)
  assert extra==13*queen,(s['index'],f['depth'],extra,queen)
  if queen:events.append(dict(seed=s['seed'],depth=f['depth'],ordinary_budget=f['ordinary_budget'],initial_non_godbeast_blessings=f['ordinary_blessings'],conditional_nest_blessings=extra))
keys=['offense','defense','accuracy','dodge','dr','max_hp'];tracks=['all_initial_unblessed','all_initial','ordinary_unblessed','ordinary']
out=dict(samples=len(samples),seed_indices=[s['index'] for s in samples],seeds=[s['seed'] for s in samples],failed_seeds=[s for s in records if 'error' in s],metrics=keys,revision=[33],profiles={})
for track in tracks:
 out['profiles'][track]={phase:[[[f[phase][track]['max_hp'] if k=='max_hp' else f[phase][track]['stats'][k] for k in keys] for f in s['floors']] for s in samples] for phase in ['entry','exit']}
Path(str(opts.prefix)+'-joint-profiles.json').write_text(json.dumps(out,separators=(',',':'))+'\n')
receipt=dict(scope=f'{len(samples)} fresh r33 generated worlds; heart census only, no survival simulation or full-run balance claim.',source_sha256=hashlib.sha256(src.read_bytes()).hexdigest(),binary_sha256=hashlib.sha256((ROOT/'target/release/examples/heart_progression_audit').read_bytes()).hexdigest(),samples=len(samples),failures=out['failed_seeds'],nest_accounting=events,track_caveat='The historical ordinary track means initial non-Godbeast actors, and now includes the new conditional Nest. all_initial also includes initial Zillennia; neither includes future summons or event fish.')
Path(str(opts.prefix)+'-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
print(json.dumps(receipt,indent=2))
