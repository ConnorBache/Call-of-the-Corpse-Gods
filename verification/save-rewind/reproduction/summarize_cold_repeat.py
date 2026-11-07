import json, pathlib, re, math
p=pathlib.Path('/workspace/scratch/69f53637ba6f/save-journal-r39/reports/stage2/cold-repeat')
def stats(xs):
 s=sorted(xs);return {'n':len(s),'median_ms':s[len(s)//2],'p95_ms':s[math.ceil(.95*len(s))-1],'samples_ms':xs}
r={'comparison':'supplemental quiet cold5-frame A/B/B/A, normal Linux release; original outliers retained separately','order':['a1','b1','b2','a2'],'normal':{},'strace_diagnostic':{}}
for name,seq in [('baseline',['a1','a2']),('candidate',['b1','b2'])]:
 ds=[json.loads((p/f'{s}.json').read_text())[0]for s in seq]
 r['normal'][name]={k:stats(sum([d[k]['samples_ms']for d in ds],[]))for k in ['candidate_export_and_envelope','candidate_durable_flush_inclusive']}
 
 if not (p/f'{name}-traced.json').exists() or not (p/f'{name}-traced.json').stat().st_size:
  r['strace_diagnostic'][name]={'blocked':'PTRACE_TRACEME Operation not permitted; no syscall data collected'};continue
 lines=(p/f'{name}.strace').read_text().splitlines();calls=[]
 for line in lines:
  if 'corpse-save-envelope-bench-' not in line:continue
  m=re.match(r'([0-9.]+) (\w+)\(.*<([0-9.]+)>$',line)
  if m:calls.append({'timestamp':float(m[1]),'syscall':m[2],'ms':float(m[3])*1000,'line':line})
 groups={}
 for x in calls:groups.setdefault(x['syscall'],[]).append(x['ms'])
 r['strace_diagnostic'][name]={'warning':'Tracing changes latency; separate diagnostic, not mixed with normal samples. Captures syscalls only, not attribution of earlier untraced outlier.', 'syscalls':{k:{'count':len(v),'sum_ms':sum(v),'max_ms':max(v)}for k,v in groups.items()},'slowest_file_calls':sorted(calls,key=lambda c:c['ms'],reverse=True)[:10],'traced_benchmark':json.loads((p/f'{name}-traced.json').read_text())[0]}
(p/'summary.json').write_text(json.dumps(r,indent=2));print(json.dumps(r['normal'],indent=2));print(r['strace_diagnostic'])
