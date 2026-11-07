#!/usr/bin/env python3
"""Summarize native heart_progression_audit JSONL without inventing gameplay.

Usage: python3 tools/monster-balance/summarize_player_progression.py INPUT OUTPUT_PREFIX
Outputs wide mean CSV, full distribution JSON, and joint six-stat cohort JSON.
O(S*D*T*K) time/memory with S seed samples, D=40, T=4 tracks, K=13 metrics;
quantiles sort O(S log S) per cell. Analysis only; never loaded by the game.
"""
import csv
import json
import math
import statistics
import sys
from pathlib import Path

TRACKS = ("all_initial_unblessed", "all_initial", "ordinary_unblessed", "ordinary")
STATS = ("offense", "defense", "accuracy", "dodge", "dr", "perception", "maxEnergy")
METRICS = STATS + ("max_hp", "level", "hearts", "blessings", "unspent_blessings", "shortsword_base")

def quantile(vals, q):
    s = sorted(vals)
    i = (len(s)-1)*q
    lo, hi = math.floor(i), math.ceil(i)
    return s[lo] + (s[hi]-s[lo])*(i-lo)

def distribution(vals):
    n = len(vals)
    mean = statistics.mean(vals)
    sd = statistics.stdev(vals) if n > 1 else 0
    se = sd/math.sqrt(n)
    half = n//2
    return dict(n=n, mean=mean, sd=sd, standard_error=se, ci95_mean=[mean-1.96*se, mean+1.96*se],
                min=min(vals), p10=quantile(vals,.1), median=quantile(vals,.5), p90=quantile(vals,.9), max=max(vals),
                first_half_mean=statistics.mean(vals[:half]) if half else mean,
                second_half_mean=statistics.mean(vals[half:]),
                half_difference=abs(statistics.mean(vals[:half])-statistics.mean(vals[half:])) if half else 0)

def value(profile, metric):
    return profile["stats"][metric] if metric in STATS else profile[metric]

def main():
    source, prefix = map(Path, sys.argv[1:3])
    records = [json.loads(s) for s in source.read_text().splitlines() if s.strip()]
    failures = [s for s in records if "error" in s]
    samples = [s for s in records if "error" not in s]
    assert samples and all(len(s["floors"]) == 40 for s in samples)
    assert len({s["index"] for s in samples}) == len(samples), "Duplicate seed index"
    samples.sort(key=lambda s:s["index"])
    for sample in samples:
        for i,f in enumerate(sample["floors"]):
            assert abs(f["ordinary_blessings"]-f["ordinary_budget"]) < 1e-8, (sample["index"],i,"ordinary budget")
            for phase in ("entry","exit"):
                for track in TRACKS:
                    p=f[phase][track]
                    assert p["level"] == math.floor(math.log2(p["blessings"]+2)), (sample["index"],i,"level")
                    assert p["max_hp"] == 100+6*p["stats"]["defense"]+p["stats"].get("maxHp",0), (sample["index"],i,"HP")
                    if phase=="entry" and i:
                        assert p == sample["floors"][i-1]["exit"][track], (sample["index"],i,"continuity")
                for base in ("all_initial","ordinary"):
                    b=f[phase][base];u=f[phase][base+"_unblessed"]
                    delta=sum(u["stats"][k]-b["stats"][k] for k in ("offense","defense"))
                    assert delta == 5*(u["level"]-1), (sample["index"],i,"level grants")
    rows, floors = [], []
    for di in range(40):
        fs = [s["floors"][di] for s in samples]
        floor = {"depth":di+1,"floor_hearts":distribution([f["hearts"] for f in fs]),
                 "floor_blessings":distribution([f["blessings"] for f in fs]),
                 "ordinary_blessings":distribution([f["ordinary_blessings"] for f in fs]),
                 "profiles":{}}
        for track in TRACKS:
            floor["profiles"][track] = {}
            for phase in ("entry", "exit"):
                metrics = {m:distribution([value(f[phase][track],m) for f in fs]) for m in METRICS}
                floor["profiles"][track][phase] = metrics
                rows.append({"depth":di+1,"profile":track,"phase":phase,"samples":len(samples),
                             "floor_hearts":floor["floor_hearts"]["mean"],"floor_blessings":floor["floor_blessings"]["mean"],
                             **{m:d["mean"] for m,d in metrics.items()}})
        floors.append(floor)
    with Path(str(prefix)+"-means.csv").open("w",newline="") as out:
        w=csv.DictWriter(out,fieldnames=rows[0]);w.writeheader();w.writerows(rows)
    summary={"input":str(source),"samples":len(samples),"failed_seeds":failures,"revision":sorted({s["revision"] for s in samples}),
             "seed_indices":[s["index"] for s in samples],"floors":floors,
             "assumptions":"Actual native startup generation, first natural heart-bearing actors once; machines and initial Zillennia included in all_initial. No turns, survival, destruction, summons, Mercuria fish, equipment loot or selected divine abilities. Unblessed tracks execute seeded +5 Offense/Defense level grants. Heart-only tracks explicitly suppress grants after real inventory consumption."}
    Path(str(prefix)+"-summary.json").write_text(json.dumps(summary,indent=2)+"\n")
    cohorts={"samples":len(samples),"seed_indices":[s["index"] for s in samples],"seeds":[s["seed"] for s in samples],"failed_seeds":failures,"metrics":["offense","defense","accuracy","dodge","dr","max_hp"],
             "revision":summary["revision"],"profiles":{}}
    for track in TRACKS:
        cohorts["profiles"][track]={phase:[[[value(f[phase][track],m) for m in cohorts["metrics"]]
                                               for f in s["floors"]] for s in samples] for phase in ("entry","exit")}
    Path(str(prefix)+"-joint-profiles.json").write_text(json.dumps(cohorts,separators=(",",":"))+"\n")
    print(json.dumps({"samples":len(samples),"revision":summary["revision"],"outputs":str(prefix)+"-{means.csv,summary.json,joint-profiles.json}"}))

if __name__ == "__main__":
    main()
