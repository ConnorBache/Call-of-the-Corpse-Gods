"""Summarize final executable-target test results. O(test-log bytes)."""
import json,pathlib,re,hashlib
root=pathlib.Path(__file__).resolve().parents[1];s=json.loads((root/'reports/stage2/full-tests/status.json').read_text())
assert len(s['completed'])==143,(s['state'],len(s['completed']))
totals={'passed':0,'failed':0,'ignored':0,'measured':0,'filtered':0};targets=[]
for t in s['completed']:
 assert t['exit_code']==0,t
 p=pathlib.Path(t['log']);text=p.read_text();m=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',text)
 assert len(m)==1,(t['target'],m)
 for k,v in zip(totals,m[0]):totals[k]+=int(v)
 targets.append({'target':t['target'],'seconds':t['seconds'],'counts':dict(zip(totals,map(int,m[0]))),'log_sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
r={'all_targets_passed':True,'targets':143,'totals':totals,'profile':'opt-level1/debug0/debug assertions true/overflow checks true; not benchmark release timings','details':targets}
(root/'reports/stage2/full-test-summary.json').write_text(json.dumps(r,indent=2));print(totals)
