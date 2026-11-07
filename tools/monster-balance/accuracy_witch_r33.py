#!/usr/bin/env python3
"""Paired native Witch brood audit; shipping after rows have no candidates."""
import csv,hashlib,json,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2];P=ROOT/'tools/monster-balance';path=P/'relocated-player-progressions-r32-joint-profiles.json';b=json.loads(path.read_text());binary=ROOT/'target/release/examples/physical_trade_arena'
proc=subprocess.Popen([str(binary)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True);rows=[]
try:
 for track in ['all_initial_unblessed','ordinary_unblessed']:
  profiles=[x[26] for x in b['profiles'][track]['entry']]
  for scenario in ['three-cast-children','chase']:
   for variant in ['before','after']:
    q=dict(kind='witch',revision=33,profiles=profiles,trials=127,scenario=scenario,max_turns=256,settle_turns=64,settlement_strategy='escape',seed_prefix='accuracy33:witch:'+track+':'+scenario)
    if variant=='before':q['summoned_candidates']={k:dict(accuracy=140,dodge=30) for k in ['night-crawler','skin-walker']}
    proc.stdin.write(json.dumps(q)+'\n');proc.stdin.flush();r=json.loads(proc.stdout.readline());r.update(track=track,variant=variant,profile_revision=32,reference_depth=27,profile_sha256=hashlib.sha256(path.read_bytes()).hexdigest(),binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest());rows.append(r)
    print(track,scenario,variant,r.get('mean_gross_combat_damage_fraction'),r.get('deaths'),r.get('censored'),r.get('error'),flush=True)
    (P/'r33-accuracy-witch-shipping.json').write_text(json.dumps(rows,indent=2)+'\n')
finally:proc.stdin.close();proc.wait()
keys=['track','scenario','variant','trials','profiles','mean_gross_combat_damage_fraction','mean_gross_damage_fraction','mean_cost','mean_attacks_to_kill_conditional_on_kill','mean_world_turns_to_kill_conditional_on_kill','max_damage_packet_fraction','kills','deaths','censored','combat_censored','aftermath_censored','error']
with (P/'r33-accuracy-witch-shipping.csv').open('w',newline='') as f:w=csv.DictWriter(f,keys,extrasaction='ignore');w.writeheader();w.writerows(rows)
