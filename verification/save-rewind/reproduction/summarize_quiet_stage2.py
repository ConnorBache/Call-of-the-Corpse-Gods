#!/usr/bin/env python3
"""O(samples log samples + evidence bytes), independent benchmark verification."""
import hashlib,json,math,pathlib,statistics
root=pathlib.Path(__file__).resolve().parents[1];q=root/'reports/quiet-stage2'
def stats(xs):
 s=sorted(xs);return {'n':len(s),'median_ms':s[len(s)//2],'p95_ms':s[math.ceil(len(s)*.95)-1],'samples_ms':xs}
report={'comparison':'published v17 runtime versus staged coarse journal; normal Linux release','component_only':True,'results':[],'evidence':[],'profiles':[],'cold_and_flush':[]}
for group in ['command','temporal']:
 runs={r:{c['case']:c for c in json.loads((q/f'{r}-{group}.json').read_text())['cases']} for r in ['a1','b1','b2','a2']}
 names=runs['a1'].keys()
 assert all(set(x)==set(names) for x in runs.values())
 for name in names:
  row={'group':group,'case':name,'bytes':runs['a1'][name]['bytes']}
  for field in ['command_inclusive','immediate_save_inclusive','command_plus_save_inclusive']:
   a=stats(runs['a1'][name][field]['samples_ms']+runs['a2'][name][field]['samples_ms'])
   b=stats(runs['b1'][name][field]['samples_ms']+runs['b2'][name][field]['samples_ms'])
   row[field]={'baseline':a,'candidate':b,'median_reduction_percent':100*(1-b['median_ms']/a['median_ms'])}
  report['results'].append(row)
  hashes=[]
  for r in ['a1','b1','b2','a2']:
   p=q/f'{r}-{group}-evidence'/f'{name}.json';hashes.append(hashlib.sha256(p.read_bytes()).hexdigest())
  assert len(set(hashes))==1,(name,hashes)
  report['evidence'].append({'case':name,'all_four_exact_save_bytes_equal':True,'sha256':hashes[0]})
for group in ['command','temporal']:
 data=json.loads((q/f'profile-on-{group}.json').read_text())
 for case in data['cases']:
  samples=case['profile_reports'];labels=set().union(*(x.keys() for x in samples))
  root_label='benchmark.command_plus_save' if group=='command' else 'benchmark.temporal_command_plus_save'
  for sample in samples:assert sample[root_label]['inclusive_ns']==sum(v['exclusive_ns'] for v in sample.values()),case['case']
  report['profiles'].append({'case':case['case'],'samples':len(samples),'exclusive_partition_exact':True,'labels':{label:{'calls':[x.get(label,{}).get('calls',0) for x in samples],'exclusive_ns':[x.get(label,{}).get('exclusive_ns',0) for x in samples],'inclusive_ns':[x.get(label,{}).get('inclusive_ns',0) for x in samples]} for label in sorted(labels)}})
on={c['case']:c for c in json.loads((q/'profile-on-command.json').read_text())['cases']}
off={c['case']:c for c in json.loads((q/'profile-off-command.json').read_text())['cases']}
report['instrumentation_overhead_small_sample']=[{'case':k,'off_median_ms':off[k]['command_plus_save_inclusive']['median_ms'],'on_median_ms':on[k]['command_plus_save_inclusive']['median_ms'],'n_each':3}for k in on]
a=json.loads((q/'baseline-cold-flush.json').read_text());b=json.loads((q/'candidate-cold-flush.json').read_text());assert len(a)==len(b)
for old,new in zip(a,b):
 assert(old['case'],old['starting_turns'],old['cache'])==(new['case'],new['starting_turns'],new['cache'])
 report['cold_and_flush'].append({'case':old['case'],'starting_frames':old['starting_turns'],'mode':old['cache'],'bytes':old['bytes'],'encode':{'baseline':old['candidate_export_and_envelope'],'candidate':new['candidate_export_and_envelope']},'durable_flush_inclusive':{'baseline':old['candidate_durable_flush_inclusive'],'candidate':new['candidate_durable_flush_inclusive']}})
(root/'reports/stage2/quiet-summary.json').write_text(json.dumps(report,indent=2))
for row in report['results']:
 m=row['command_plus_save_inclusive'];print(row['case'], 'median',round(m['baseline']['median_ms'],3),'->',round(m['candidate']['median_ms'],3),'p95',round(m['baseline']['p95_ms'],3),'->',round(m['candidate']['p95_ms'],3))
print('Exact cross-binary save cases',len(report['evidence']),'exact exclusive samples',sum(x['samples']for x in report['profiles']))
