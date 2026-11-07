import json,pathlib,subprocess,time,re,statistics,hashlib
b=pathlib.Path(__file__).resolve().parents[1];o=b/'performance-small';repo=b/'source/corpse-gods-native';records=[]
profiles=[('baseline',b/'build/linux-target','build/small/perf-test-build-baseline.jsonl'),('z',b/'build/linux-small-target','build/small/perf-test-build-small.jsonl'),('s',b/'build/linux-small-s-target','build/small-s/perf-test-build.jsonl')]
for label,target,buildlog in profiles:
 exe=target/'release/examples/generated_commands_bench';t=time.monotonic();p=subprocess.run([str(exe)],cwd=repo,capture_output=True,text=True);(o/f'{label}-quiet-core.json').write_text(p.stdout);(o/f'{label}-quiet-core.stderr').write_text(p.stderr);assert p.returncode==0,p.stderr
 core=json.loads(p.stdout);print(label,'core pass',round(time.monotonic()-t,2),flush=True)
 messages=[json.loads(x) for x in (b/buildlog).read_text().splitlines() if x.startswith('{')];test=[x['executable'] for x in messages if x.get('reason')=='compiler-artifact' and x.get('executable') and x.get('profile',{}).get('test')][-1]
 p=subprocess.run([test,'benchmark_animated_manual_warm_frames','--ignored','--nocapture','--test-threads=1'],cwd=repo,capture_output=True,text=True);(o/f'{label}-quiet-warm.log').write_text(p.stdout+p.stderr);assert p.returncode==0,p.stdout+p.stderr
 warm=[{'resolution':m[0],'id':m[1],'depth':int(m[2]),'mean_ms':float(m[3])} for m in re.findall(r'MANUAL_WARM (\S+) (\S+) depth=(\d+) mean_ms=([\d.]+)',p.stderr)];assert len(warm)==8
 records.append({'profile':label,'core_binary_sha256':hashlib.sha256(exe.read_bytes()).hexdigest(),'core':core,'warm':warm});print(label,'warm pass',max(x['mean_ms'] for x in warm),flush=True)
base=records[0];summary=[]
for r in records:
 ratios=[x['timing']['median_us']/y['timing']['median_us'] for x,y in zip(r['core']['results'],base['core']['results'])];wr=[x['mean_ms']/y['mean_ms'] for x,y in zip(r['warm'],base['warm'])]
 summary.append({'profile':r['profile'],'median_core_ratio_vs_baseline':statistics.median(ratios),'core_ratio_range':[min(ratios),max(ratios)],'max_core_median_ms':max(x['timing']['median_us']/1000 for x in r['core']['results']),'median_warm_ratio_vs_baseline':statistics.median(wr),'warm_ms_range':[min(x['mean_ms'] for x in r['warm']),max(x['mean_ms'] for x in r['warm'])]})
out={'scope':'Sequential Linux release benchmarks after all builds and full gate completed; shared cloud host not controlled hardware. Six generated initial-floor traces (472commands) preserve canonical parity. Warm fullmanual pages include depths1/15/40 at1280x800 and1920x1080,24frames each after2warmups. Not Windows or end-to-end FPS; no claim about worstcase deep-floor simulation.','records':records,'summary':summary};(o/'quiet-comparison.json').write_text(json.dumps(out,indent=2)+'\n');print(json.dumps(summary),flush=True)
