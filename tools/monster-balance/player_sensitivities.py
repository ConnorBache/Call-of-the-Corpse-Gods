#!/usr/bin/env python3
"""Conditional heart-collection sensitivities, not simulated play.

Inputs: native census JSONL; native `heart_progression_audit fish-ledger REV` JSON;
output JSON. Fish distribution convolves the actual uniform native species rewards
for ten fish/event, preserving exact discrete blessing thresholds. Summon scenarios
use explicitly stated four/five children per50-energy parent, no sleep/refill.
For S seeds,D40 floors,F<=80 fish,B<=400 total fish blessings, convolution O(S*F*B*K),
K<=3 eligible species; memory O(S*D+B). No gameplay code is edited or approximated.
"""
import json, math, statistics, sys
from collections import defaultdict
from pathlib import Path

def level(blessings): return math.floor(math.log2(blessings+2))
def avg(rows): return {k:statistics.mean(r[k] for r in rows) for k in rows[0]}
def main():
    source, ledger_path, out = map(Path,sys.argv[1:4])
    records=[json.loads(l) for l in source.read_text().splitlines() if l.strip()]
    worlds=[r for r in records if "error" not in r]
    ledger=json.loads(ledger_path.read_text())
    fish=[[] for _ in range(40)]
    summon={c:[[] for _ in range(40)] for c in (4,5)}
    combined={c:[[] for _ in range(40)] for c in (4,5)}
    for world in worlds:
        distribution={0:1.}
        fish_hearts=normal=greater=0
        for di,f in enumerate(world["floors"]):
            b=f["entry"]["all_initial_unblessed"]["blessings"]
            extra=sum(n*p for n,p in distribution.items())
            extra_levels=sum((level(b+n)-level(b))*p for n,p in distribution.items())
            fish[di].append(dict(extra_hearts=fish_hearts,extra_heart_offense=extra,extra_blessings=extra,
                extra_levels=extra_levels,extra_level_offense=2.5*extra_levels,
                extra_level_defense=2.5*extra_levels,extra_max_hp=15*extra_levels))
            for c in (4,5):
                # Greater parents create c ordinary Pale children. 1/5 of them
                # are basic Birthers; their c Jackals give no stats/blessings.
                xp=c*greater
                dl=level(b+xp)-level(b)
                summon[c][di].append(dict(natural_basic_birthers=normal,natural_greater_birthers=greater,
                    extra_hearts=c*normal+c*greater+c*c*greater/5,
                    extra_blessings=xp,extra_heart_offense=2*c*greater/5,
                    extra_heart_defense=2*c*greater/5,extra_heart_accuracy=2*c*greater/5,
                    extra_heart_max_energy=2*c*greater/5,extra_heart_dr=c*greater/5,
                    extra_levels=dl,extra_level_offense=2.5*dl,extra_level_defense=2.5*dl,
                    extra_max_hp=6*(2*c*greater/5+2.5*dl)))
                combined_dl=sum((level(b+xp+n)-level(b))*p for n,p in distribution.items())
                s=f["entry"]["all_initial_unblessed"]["stats"]
                combined[c][di].append(dict(extra_hearts=fish_hearts+c*normal+c*greater+c*c*greater/5,
                    extra_blessings=extra+xp,extra_levels=combined_dl,
                    extra_offense=extra+2*c*greater/5+2.5*combined_dl,
                    extra_defense=2*c*greater/5+2.5*combined_dl,
                    extra_accuracy=2*c*greater/5,extra_dr=c*greater/5,extra_max_energy=2*c*greater/5,
                    extra_max_hp=6*(2*c*greater/5+2.5*combined_dl),
                    conditional_mean_offense=s["offense"]+extra+2*c*greater/5+2.5*combined_dl,
                    conditional_mean_defense=s["defense"]+2*c*greater/5+2.5*combined_dl,
                    conditional_mean_accuracy=s["accuracy"]+2*c*greater/5,
                    conditional_mean_dr=s["dr"]+c*greater/5,
                    conditional_mean_dodge=s["dodge"],conditional_mean_perception=s["perception"],
                    conditional_mean_max_energy=s["maxEnergy"]+2*c*greater/5,
                    conditional_mean_max_hp=f["entry"]["all_initial_unblessed"]["max_hp"]+6*(2*c*greater/5+2.5*combined_dl)))
            normal+=f["species"].get("pale:Birther",0)+f["species"].get("Pale Birther",0)
            greater+=f["species"].get("pale:Greater Birther",0)+f["species"].get("Greater Pale Birther",0)
            if (f.get("godbeast_event") or {}).get("kind")=="Mercuria":
                hearts=ledger["floors"][di]["hearts"]
                assert all(h["bonus"]=={"offense":h["exp"]} for h in hearts)
                for _ in range(10):
                    nxt=defaultdict(float)
                    for n,p in distribution.items():
                        for h in hearts: nxt[n+int(h["exp"])]+=p/len(hearts)
                    distribution=nxt
                fish_hearts+=10
    result={"samples":len(worlds),"source":str(source),"revision":ledger["revision"],
        "scope":"Alternatives to the finite initial-population census; do not add both to primary without labeling. Conditional potential rewards, not simulated survival or guaranteed access. Every original heart once; heartless parents/children add none in revision32.",
        "fish_assumptions":"All ten fish from each prior Mercuria event emerge and are consumed. Native species probabilities are uniform, draws treated independently; heart rewards from native constructor ledger. Expected Unblessed extra level grants use exact discrete blessing threshold distribution, then2.5 Offense/Defense per extra level. Existing RNG order and surviving health not simulated.",
        "summon_assumptions":"All naturally heart-bearing Birthers spend one starting50-energy pool, with enough elapsed normal turns and no sleep/refill.4-child scenario:30-energy twins then two10-energy singles (three casts).5-child scenario:only one free adjacent cell at each cast, five10-energy singles. Greater children uniformly choose five basic Pale roles; basic children that are Birthers likewise exhaust once, producing zero-stat/zero-blessing Jackals. Expected grants, not actual seeded births.",
        "fish_entry":[{"depth":i+1,**avg(v)} for i,v in enumerate(fish)],
        "summon_entry":{str(c):[{"depth":i+1,**avg(v)} for i,v in enumerate(vs)] for c,vs in summon.items()},
        "combined_entry":{str(c):[{"depth":i+1,**avg(v)} for i,v in enumerate(vs)] for c,vs in combined.items()}}
    out.write_text(json.dumps(result,indent=2)+"\n")
    print(json.dumps({"output":str(out),"samples":len(worlds),"fish_depth40":result["fish_entry"][-1],"four_child_depth40":result["summon_entry"]["4"][-1]}))
if __name__=="__main__":main()
