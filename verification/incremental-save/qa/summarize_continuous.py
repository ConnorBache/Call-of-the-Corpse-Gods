"""O(sample count log sample count + evidence bytes); never used during timing."""
import pathlib,json,math,hashlib,sys
root=pathlib.Path(__file__).resolve().parents[1];q=root/'reports'/('continuous-clean'if '--clean'in sys.argv else'continuous-quiet')
def stats(x):
 if not x:return None
 s=sorted(x);return{'n':len(x),'median_ms':s[len(s)//2],'p95_ms':s[math.ceil(.95*len(s))-1],'max_ms':s[-1],'samples_ms':x}
runs={r:{c['case']:c for c in json.load(open(q/(r+'.json')))['cases']}for r in ['a1','b1','b2','a2']};out={'comparison':'published-v18 versus incremental candidate; Linux normal release; controlled persistence fixtures','cases':[],'evidence':[]}
for case in runs['a1']:
 row={'case':case}
 for f in ['command','flush_inclusive','command_plus_flush','periodic_checkpoint_flush','routine_append_flush']:
  row[f]={name:stats(sum([runs[r][case][f]['samples_ms']if runs[r][case][f]else[]for r in seq],[]))for name,seq in [('baseline',['a1','a2']),('candidate',['b1','b2'])]}
 row['initial_checkpoint_ms']={name:[runs[r][case]['initial_checkpoint_ms']for r in seq]for name,seq in [('baseline',['a1','a2']),('candidate',['b1','b2'])]}
 row['load_samples']={name:sum([runs[r][case]['load_samples']for r in seq],[])for name,seq in [('baseline',['a1','a2']),('candidate',['b1','b2'])]}
 row['total_bytes_written']={name:sum(sum(runs[r][case]['bytes_written_per_command'])for r in seq)for name,seq in [('baseline',['a1','a2']),('candidate',['b1','b2'])]};out['cases'].append(row)
for p in sorted((q/'a1-evidence').glob('*.json')):
 hashes=[hashlib.sha256((q/(r+'-evidence')/p.name).read_bytes()).hexdigest()for r in ['a1','b1','b2','a2']];assert len(set(hashes))==1,(p.name,hashes);out['evidence'].append({'case':p.name,'all_four_exact_live_state_export_bytes_equal':True,'sha256':hashes[0]})
(q/'summary.json').write_text(json.dumps(out,indent=2))
for r in out['cases']:
 f=r['command_plus_flush'];c=r['periodic_checkpoint_flush']['candidate'];a=r['routine_append_flush']['candidate'];print(r['case'],'total median/p95',*(round(f[n][k],3)for n in ['baseline','candidate']for k in ['median_ms','p95_ms']),'append',round(a['median_ms'],3),'checkpoint max',round(c['max_ms'],3),'bytes',r['total_bytes_written'])
print('Exactcrossbinary states',len(out['evidence']))
