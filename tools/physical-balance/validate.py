"""Independent-seeded before/after and collection-sensitivity of revision31 overlay.
Analytical/Python exchange model, not full native engine execution.
"""
import json,csv,copy,sys,time
from pathlib import Path
import numpy as np
import exchange as k
P=Path(__file__).resolve().parent
ROOT=P.parents[1]
BASE=copy.deepcopy(k.CAT)
PROFILES=np.load(ROOT/'fixtures/physical-reference-profiles-v31.npz')['entry_profiles'].astype(float)
meta=json.loads((k.SRC/'src/catalog/physical-balance-v31.json').read_text())
def profiles(item,rate,repeats):
 import math
 f=item['reference_depth'];lo=math.floor(f);hi=math.ceil(f);a=PROFILES[[50,75,100].index(rate)]
 x=a[:,lo-1] if lo==hi else np.concatenate([a[:,lo-1],a[:,hi-1]])
 return np.tile(x,(repeats,1))
rows=[];t=time.time()
for item in meta:
 if item.get('validation_model')=='native_dispatch': continue
 kind=item['kind'];old=BASE[kind];new=copy.deepcopy(old)
 for key in ['offense','defense','accuracy','dodge','dr']:new['stats'][key]=item[key]
 new['hp']=new['maxHp']=100+6*item['defense']
 for rate in [100,75,50]:
  repeats=20 if kind=='rat' and rate==100 else (10 if rate==100 else 1)
  x=profiles(item,rate,repeats)
  for mode,a in [('before',old),('after',new)]:
   k.CAT[kind]=a
   result=k.exchange(kind,x,True,seed_label='independent-validation-r31:'+kind+':'+str(rate))
   pbase=30+x[:,0]
   row=dict(kind=kind,reference_depth=item['reference_depth'],collection_pct=rate,mode=mode,
    enemy_hp=a['maxHp'],enemy_offense=a['stats']['offense'],enemy_accuracy=a['stats']['accuracy'],enemy_dodge=a['stats']['dodge'],enemy_dr=a['stats']['dr'],
    target_mean_loss=item['target_mean_loss'],qualification=item['qualification'],
    player_zero_damage_fraction=float((1.25*pbase<=a['stats']['dr']).mean()),**result)
   rows.append(row)
  r=rows[-1];print(kind,rate,'net',round(r['mean_total_fight_hp_loss_fraction']*100,3),'deaths',round(r['player_death_fraction']*100,3),'censored',r['censored'],'seconds',round(time.time()-t,1),flush=True)
with (ROOT/'docs/artifacts/physical-balance-v31-before-after.csv').open('w',newline='') as o:
 w=csv.DictWriter(o,fieldnames=rows[0]);w.writeheader();w.writerows(rows)
print('TRIALS',sum(r['n'] for r in rows),'SECONDS',time.time()-t,flush=True)
