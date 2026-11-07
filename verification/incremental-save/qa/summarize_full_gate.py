"""O(test log bytes + target count). Summarize completed assertion-enabled gate only."""
import pathlib,json,re,hashlib
r=pathlib.Path(__file__).resolve().parents[1];root=r/'reports/full-tests';s=json.loads((root/'status.json').read_text());assert s['state']=='passed',s['state']
keys=['passed','failed','ignored','measured','filtered'];totals=dict.fromkeys(keys,0);details=[]
for x in s['completed']:
 p=pathlib.Path(x['log']);data=p.read_bytes();matches=re.findall(r'test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out',data.decode(errors='replace'))
 assert x['exit_code']==0,x
 assert len(matches)==1,(x['target'],len(matches))
 counts=dict(zip(keys,map(int,matches[0])));
 for k,v in counts.items():totals[k]+=v
 details.append({'target':x['target'],'seconds':x['seconds'],'counts':counts,'log_sha256':hashlib.sha256(data).hexdigest()})
result={'all_targets_passed':True,'targets':len(details),'totals':totals,'profile':'opt-level1/debug0/debug assertions true/overflow checks true; all workspace targets, including example test harnesses; duplicate module tests across harnesses count as executions, not unique cases','details':details}
(r/'reports/full-test-summary.json').write_text(json.dumps(result,indent=2));print(json.dumps({k:v for k,v in result.items()if k!='details'},indent=2))
