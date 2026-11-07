#!/usr/bin/env python3
"""Paired r33 native trials against frozen r32 joint profiles, not a full-run fit.

Primary = all_initial_unblessed; ordinary_unblessed is explicitly a sensitivity.
Percentile cases select one real joint profile, never splice independent stats.
Probability diagnostics use the unchanged native formula; outcomes run the real
physical_trade_arena dispatcher. Candidate mode does not change shipping data.
"""
import argparse, csv, hashlib, json, statistics, subprocess
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
CAT=ROOT/'crates/corpse-core/src/catalog'
PROFILES=ROOT/'tools/monster-balance/relocated-player-progressions-r32-joint-profiles.json'
# kind, actual earliest reference depth, candidate Accuracy, candidate Dodge, role
PROPOSAL=[
 ('rat',1,-1,4,'nimble vermin'),('cave-bat',1,-2,7,'small flying skirmisher'),
 ('shellback-beetle',1,-2,-7,'slow armored body'),('bear',6,6,-9,'heavy mauler'),
 ('giant-beetle',8,8,8,'large armored predator'),('tiger',10,25,38,'precise mobile hunter'),
 ('horn-1',12,41,43,'precise ranged caster'),('wendigo',17,100,50,'mobile starving hunter'),
 ('troll',19,111,16,'huge regenerating brute'),('dire-bear',20,124,9,'giant mauler'),
 ('horn-3',22,136,75,'precise ranged caster'),('night-crawler',24,144,74,'moderately mobile tentacled hunter'),
 ('sabretooth',25,142,78,'precise prehistoric hunter'),('skin-walker',26,148,92,'evasive shape-changing hunter'),
 ('vampire',30,158,92,'evasive undead skirmisher'),('minotaur',33,147,29,'committed heavy charger'),
 ('saidribee-queen',28,145,27,'large dedicated summoner'),
]
def load(p): return json.loads(Path(p).read_text())
def chance(a,d,r=0): return min(1.,.65*.9**(d-a))*.9**r
def snapshot():
 rows={x['kind']:x for x in load(CAT/'physical-balance-v31.json')}
 rows.update({x['kind']:x for x in load(CAT/'physical-balance-v32.json')})
 result={}
 for k,d,a,e,role in PROPOSAL:
  if k in rows:
   t=rows[k];s={n:t[n] for n in ['offense','defense','accuracy','dodge','dr']};s['max_hp']=100+6*s['defense']
  else:
   t=next(x for x in load(CAT/'new-creatures-v33.json') if x['creatureKind']==k)
   s={n:t['stats'][n] for n in ['offense','defense','accuracy','dodge','dr']};s['max_hp']=t['maxHp']
  result[k]=dict(depth=d,role=role,before=s,after=dict(s,accuracy=a,dodge=e))
 return result
def save(rows,path):
 path.write_text(json.dumps(rows,indent=2)+'\n')
 keys=['kind','role','depth','track','reference_role','cohort','profile_seed','variant','trials',
  'mean_player_to_enemy_probability','mean_enemy_to_player_probability','mean_player_landed_attacks',
  'mean_player_to_enemy_range4_probability','mean_enemy_to_player_range4_probability','max_damage_packet_fraction','rare_hit_large_packet_flag',
  'mean_player_attack_attempts','mean_enemy_attack_attempts','mean_gross_combat_damage_fraction',
  'mean_gross_damage_fraction','mean_cost','mean_attacks_to_kill_conditional_on_kill',
  'mean_world_turns_to_kill_conditional_on_kill','kills','deaths','censored','combat_censored','aftermath_censored','error']
 with path.with_suffix('.csv').open('w',newline='') as f:
  w=csv.DictWriter(f,fieldnames=keys,extrasaction='ignore');w.writeheader();w.writerows(rows)
def main():
 p=argparse.ArgumentParser();p.add_argument('--phase',choices=['proposal','shipping'],default='proposal')
 p.add_argument('--output',type=Path);p.add_argument('--kinds',nargs='*');p.add_argument('--mean-only',action='store_true')
 p.add_argument('--profiles',type=Path,default=PROFILES);p.add_argument('--mean-repeats',type=int,default=1)
 p.add_argument('--binary',type=Path,default=ROOT/'target/release/examples/physical_trade_arena')
 args=p.parse_args();out=args.output or ROOT/f'tools/monster-balance/r33-accuracy-{args.phase}.json';out.parent.mkdir(exist_ok=True,parents=True)
 specs=snapshot();blob=load(args.profiles);rows=[]
 provenance=dict(profile_revision=blob['revision'],profile_file=str(args.profiles),profile_sha256=hashlib.sha256(args.profiles.read_bytes()).hexdigest(),worlds=blob['samples'],binary_sha256=hashlib.sha256(args.binary.read_bytes()).hexdigest(),mechanics_revision=33,phase=args.phase,not_full_r33_progression=True)
 out.with_name(out.stem+'-method.json').write_text(json.dumps(dict(provenance=provenance,specs=specs),indent=2)+'\n')
 proc=subprocess.Popen([str(args.binary)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True,bufsize=1)
 try:
  for kind,spec in specs.items():
   if args.kinds and kind not in args.kinds:continue
   for track in ['all_initial_unblessed','ordinary_unblessed']:
    cohort=[x[spec['depth']-1] for x in blob['profiles'][track]['entry']]
    cases=[('joint_mean',cohort,None,len(cohort)*args.mean_repeats)]
    if not args.mean_only and track=='all_initial_unblessed':
     for metric,slot in [('accuracy',2),('dodge',3)]:
      for quantile in [10,90]:
       i=sorted(range(len(cohort)),key=lambda i:(cohort[i][slot],i))[round((len(cohort)-1)*quantile/100)]
       cases.append((f'{metric}_p{quantile}',[cohort[i]],blob['seeds'][i],32))
    for name,profiles,seed,trials in cases:
     for variant in ['before','after']:
      stats=spec[variant];scenario='chase' if kind in ['horn-1','horn-3','wendigo','vampire','saidribee-queen'] else 'adjacent'
      request=dict(kind=kind,revision=33,profiles=profiles,trials=trials,scenario=scenario,max_turns=256,settle_turns=64,settlement_strategy='escape',seed_prefix=f'accuracy33:{kind}:{track}:{name}')
      if variant=='before' or args.phase=='proposal':request['candidate']=stats
      proc.stdin.write(json.dumps(request)+'\n');proc.stdin.flush();line=proc.stdout.readline()
      if not line:raise RuntimeError(f'Arena ended at {kind}/{variant}: {proc.poll()}')
      result=json.loads(line);result.update(kind=kind,role=spec['role'],depth=spec['depth'],track=track,reference_role='primary' if track=='all_initial_unblessed' else 'ordinary_only_sensitivity',cohort=name,profile_seed=seed,variant=variant,provenance=provenance,
       mean_player_to_enemy_probability=statistics.mean(chance(x[2],stats['dodge']) for x in profiles),mean_enemy_to_player_probability=statistics.mean(chance(stats['accuracy'],x[3]) for x in profiles),
       mean_player_to_enemy_range4_probability=statistics.mean(chance(x[2],stats['dodge'],4) for x in profiles),mean_enemy_to_player_range4_probability=statistics.mean(chance(stats['accuracy'],x[3],4) for x in profiles))
      if 'error' not in result:
       actual=result['initial_actors'][0]
       for slot in ['offense','defense','accuracy','dodge','dr']:
        if actual['stats'][slot]!=stats[slot]:raise ValueError(f'Constructor mismatch {kind} {variant} {slot}: {actual["stats"][slot]} != {stats[slot]}')
       if actual['max_hp']!=stats['max_hp']:raise ValueError(f'Constructor HP mismatch {kind}')
      result['rare_hit_large_packet_flag']=result['mean_enemy_to_player_probability']<.1 and result.get('max_damage_packet_fraction',0)>.35
      rows.append(result);save(rows,out)
      print(kind,track,name,variant,'gross',result.get('mean_gross_combat_damage_fraction'),'kills',result.get('kills'),'deaths',result.get('deaths'),'censor',result.get('censored'),'error',result.get('error',''),flush=True)
 finally:
  proc.stdin.close();proc.wait()
if __name__=='__main__':main()
