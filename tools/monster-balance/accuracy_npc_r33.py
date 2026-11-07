#!/usr/bin/env python3
"""Native same-depth species-vs-species sensitivity, including real ally phases.
Hostile packet mode is separately labeled: normal hostiles do not target allies
of their own side merely because this harness probes their combat matchup.
"""
import argparse,csv,hashlib,json,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
PAIRS=[('shellback-beetle','cave-bat',1),('giant-beetle','tiger',10),('bear','tiger',10),
 ('horn-1','tiger',12),('troll','wendigo',20),('dire-bear','sabretooth',25),
 ('horn-3','skin-walker',26),('minotaur','skin-walker',33),('vampire','skin-walker',30)]
def main():
 p=argparse.ArgumentParser();p.add_argument('--phase',choices=['proposal','shipping'],default='proposal');p.add_argument('--specs',type=Path);p.add_argument('--trials',type=int,default=64);args=p.parse_args()
 source=args.specs or ROOT/'tools/monster-balance/r33-accuracy-proposal-method.json';spec=json.loads(source.read_text())['specs'];binary=ROOT/'target/release/examples/accuracy_npc_arena';out=ROOT/f'tools/monster-balance/r33-accuracy-npc-{args.phase}.json'
 proc=subprocess.Popen([str(binary)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True);rows=[]
 try:
  for left,right,depth in PAIRS:
   for packet in [False,True]:
    for reverse in ([False,True] if not packet else [False]):
     for variant in ['before','after']:
      q=dict(left=left,right=right,reverse=reverse,packet_probe=packet,trials=args.trials)
      if variant=='before' or args.phase=='proposal':
       for side,kind in [('left',left),('right',right)]:q[side+'_patch']=spec[kind][variant]
      proc.stdin.write(json.dumps(q)+'\n');proc.stdin.flush();line=proc.stdout.readline()
      if not line:raise RuntimeError('Native NPC arena exited')
      r=json.loads(line);r.update(variant=variant,same_depth=depth,baseline='pre-specialization r33 stats; both variants use same current r33 mechanics',binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest());rows.append(r)
      out.write_text(json.dumps(rows,indent=2)+'\n');print(left,right,variant,'packet' if packet else 'ally','reverse',reverse,r.get('left_wins'),r.get('right_wins'),r.get('censored'),r.get('error',''),flush=True)
 finally:proc.stdin.close();proc.wait()
 keys=['left','right','same_depth','mode','reverse','variant','trials','left_wins','right_wins','censored','mean_commands','mean_left_gross_fraction','mean_right_gross_fraction','player_untouched','error']
 with out.with_suffix('.csv').open('w',newline='') as f:w=csv.DictWriter(f,keys,extrasaction='ignore');w.writeheader();w.writerows(rows)
if __name__=='__main__':main()
