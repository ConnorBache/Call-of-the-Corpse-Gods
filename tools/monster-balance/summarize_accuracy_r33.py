#!/usr/bin/env python3
"""Compact approved-profile matrix; detailed native rows remain the evidence."""
import csv,json
from pathlib import Path
P=Path(__file__).resolve().parent;rows=json.loads((P/'r33-accuracy-shipping.json').read_text());spec=json.loads((P/'r33-accuracy-shipping-method.json').read_text())['specs'];out=[]
def get(kind,variant,cohort='joint_mean',track='all_initial_unblessed'):
 return next(x for x in rows if x['kind']==kind and x['variant']==variant and x['cohort']==cohort and x['track']==track)
for kind,s in spec.items():
 b=get(kind,'before');a=get(kind,'after');low=get(kind,'after','accuracy_p10');high=get(kind,'after','accuracy_p90');ld=get(kind,'after','dodge_p10');hd=get(kind,'after','dodge_p90');ordinary=get(kind,'after',track='ordinary_unblessed')
 out.append(dict(kind=kind,depth=s['depth'],role=s['role'],accuracy_before=s['before']['accuracy'],accuracy_after=s['after']['accuracy'],dodge_before=s['before']['dodge'],dodge_after=s['after']['dodge'],hp=s['after']['max_hp'],offense=s['after']['offense'],primary_worlds=127,
  gross_before=b['mean_gross_combat_damage_fraction'],gross_after=a['mean_gross_combat_damage_fraction'],lifetime_attempts_before=b['mean_attacks_to_kill_conditional_on_kill'],lifetime_attempts_after=a['mean_attacks_to_kill_conditional_on_kill'],lifetime_world_turns_after=a['mean_world_turns_to_kill_conditional_on_kill'],
  player_hit_mean=a['mean_player_to_enemy_probability'],player_hit_p10_accuracy=low['mean_player_to_enemy_probability'],player_hit_p90_accuracy=high['mean_player_to_enemy_probability'],enemy_hit_mean=a['mean_enemy_to_player_probability'],enemy_hit_p10_dodge=ld['mean_enemy_to_player_probability'],enemy_hit_p90_dodge=hd['mean_enemy_to_player_probability'],primary_deaths=a['deaths'],primary_combat_censored=a['combat_censored'],primary_aftermath_censored=a['aftermath_censored'],low_accuracy_deaths_of32=low['deaths'],high_accuracy_deaths_of32=high['deaths'],ordinary_only_gross=ordinary['mean_gross_combat_damage_fraction'],ordinary_only_deaths_of127=ordinary['deaths'],max_primary_damage_packet_fraction=a['max_damage_packet_fraction']))
(P/'r33-accuracy-summary.json').write_text(json.dumps(dict(reference='Frozen r32 all_initial_unblessed, native revision33 mechanics; ordinary-only explicitly sensitivity',lifetime_warning='Kill conditional, not unconditional when deaths/censors exist.',rows=out),indent=2)+'\n')
with (P/'r33-accuracy-summary.csv').open('w',newline='') as f:w=csv.DictWriter(f,out[0].keys());w.writeheader();w.writerows(out)
