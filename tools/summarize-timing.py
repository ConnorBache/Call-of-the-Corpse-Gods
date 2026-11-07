#!/usr/bin/env python3
"""Verify cross-feature exact state equality and summarize measured profiles.
Time O(samples*labels + JSON bytes + sort(labels)); memory O(report bytes).
No inference from theoretical bounds, no subtraction of overlapping inclusive totals.
"""
import json,sys,pathlib,statistics

def main():
    """O(input bytes + samples*labels + labels log labels); pure local diagnostics."""
    root=pathlib.Path(sys.argv[1])
    a=json.loads((root/'core-normal.json').read_text());b=json.loads((root/'core-profiled.json').read_text())
    assert not a['runtime_timing_feature'] and b['runtime_timing_feature'], 'Need separate feature-OFF and feature-ON binaries'
    assert len(a['scenarios'])==len(b['scenarios'])
    summaries=[]
    for x,y in zip(a['scenarios'],b['scenarios']):
        assert (x['scenario'],x['command'])==(y['scenario'],y['command'])
        filename=f"{x['scenario']}-{x['command']}.json"
        assert (root/'states-normal'/filename).read_bytes()==(root/'states-profiled'/filename).read_bytes(),filename
        totals={}
        for sample in y['counter_samples']:
            assert sample['sample.command']['calls']==1
            assert sum(v['exclusive_ns'] for v in sample.values())==sample['sample.command']['inclusive_ns']
            for label,row in sample.items():
                t=totals.setdefault(label,{'calls':0,'exclusive_ns':0,'inclusive_ns':0})
                for key in t:t[key]+=row[key]
        root_ns=totals.get('sample.command',{}).get('inclusive_ns',0)
        ranked=sorted(totals.items(),key=lambda p:p[1]['exclusive_ns'],reverse=True)
        n=len(y['counter_samples'])
        summaries.append({'scenario':x['scenario'],'command':x['command'],'ticks':x['ticks_advanced'],'active_actors':x['active_actors'],'normal_median_ms':x['collector_disabled']['median_ns']/1e6,'profiled_collector_off_median_ms':y['collector_disabled']['median_ns']/1e6,'profiled_collector_on_median_ms':y['collector_enabled']['median_ns']/1e6,'exact_cross_feature_save_equal':True,'exclusive_ranking':[{'label':k,'mean_exclusive_ms':v['exclusive_ns']/n/1e6,'mean_inclusive_ms':v['inclusive_ns']/n/1e6,'calls_per_sample':v['calls']/n,'fraction_profiled_root_exclusive':v['exclusive_ns']/root_ns if root_ns else None}for k,v in ranked]})
    output={'warning':'Exclusive rankings are from instrumented execution; normal/disabled/enabled timing differences quantify observer and build effects, not calibrated subtraction. Hardware/OS noise and coverage limits apply.','core':summaries}
    if (root/'desktop.json').exists():output['desktop_report']='desktop.json'
    (root/'summary.json').write_text(json.dumps(output,indent=2))
    for s in summaries:
        print(s['scenario'],s['command'],f"normal={s['normal_median_ms']:.3f}ms instrumented={s['profiled_collector_on_median_ms']:.3f}ms")
        for r in s['exclusive_ranking'][:5]:print(' ',r['label'],round(r['mean_exclusive_ms'],3),'ms',r['calls_per_sample'],'calls')
if __name__=='__main__':main()
