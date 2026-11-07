"""Whole-encounter cost diagnostic. NO 20%-per-hit target is used.

Restricted source-ordered stand-trade model: awake/hunting adjacent opponent,
player Shortsword attack, then normal-turn automatic regeneration and opponent
attack. Fast is extra movement only and gives no extra attack. Slow skips even
normal ticks, starting with tick one. No terrain, surprise, activated abilities,
retreat, corpse growth, groups, player equipment upgrades, or navigation.
Healing toggle includes base .2% regen, Troll x10, and innate Vampire lifesteal.
Not a native engine execution or a simulation of full ability-equipped players.
"""
from pathlib import Path
import csv,hashlib,json,math,time
import numpy as np
P=Path(__file__).resolve().parent
SRC=P.parents[1]/'crates/corpse-core'
CAT=json.loads((SRC/'src/catalog/population-v48.json').read_text())['actors']
ROSTER=json.loads((SRC/'src/catalog/ordinary-roster-v48.json').read_text())


def wilson(count,n):
 z=1.959963984540054;p=count/n;den=1+z*z/n
 center=(p+z*z/(2*n))/den
 half=z*np.sqrt(p*(1-p)/n+z*z/(4*n*n))/den
 return center-half,center+half

def chance(a,d):return np.exp(np.minimum(0,math.log(.65)+(d-a)*math.log(.9)))

def exchange(kind,x,healing=True,raw_override=None,accuracy_override=None,seed_label='reference-v2',horizon=1000):
 a=CAT[kind];s=a['stats'];ab=a['abilities'];c=a['conditions'];n=len(x)
 B=c.get('weaponDamage',10)+s['offense']*c.get('weaponOffenseMultiplier',1) if raw_override is None else raw_override
 A=s['accuracy'] if accuracy_override is None else accuracy_override
 slow='enemy:beast:slow' in ab;pierce='enemy:beast:armor-piercing' in ab
 preg=.002 if healing else 0;ereg=preg*(10 if 'enemy:beast:regeneration' in ab else 1)
 life=healing and 'enemy:beast:lifesteal' in ab
 pbase=30+x[:,0];phit=chance(x[:,2],s['dodge']);ehit=chance(A,x[:,3]);hp=x[:,5].copy();pmax=hp.copy();ehp=np.full(n,a['maxHp'],float)
 seed=int.from_bytes(hashlib.sha256(f'{seed_label}:{kind}:{healing}'.encode()).digest()[:16],'little');rng=np.random.default_rng(seed)
 pat=np.zeros(n,int);eat=np.zeros(n,int);rounds=np.zeros(n,int);gross=np.zeros(n);rawgross=np.zeros(n);planded=np.zeros(n,int);elanded=np.zeros(n,int)
 for turn in range(1,horizon+1):
  live=(hp>0)&(ehp>0)
  if not live.any():break
  rounds[live]+=1;pat[live]+=1
  hit=(rng.random(n)<phit)&live;planded+=hit
  dmg=np.maximum(0,np.ceil((pbase*(.75+.5*rng.random(n))-s['dr'])/2))*hit;ehp-=dmg
  # turn() still runs after the player kills an enemy; living player regenerates.
  hp[live]=np.minimum(pmax[live],hp[live]+pmax[live]*preg)
  remains=live&(ehp>0);ehp[remains]=np.minimum(a['maxHp'],ehp[remains]+a['maxHp']*ereg)
  if slow and turn%2==0:continue
  eat[remains]+=1
  hit=(rng.random(n)<ehit)&remains;elanded+=hit
  raw=np.maximum(0,np.ceil((B*(.75+.5*rng.random(n))-(0 if pierce else x[:,4]))/2))*hit
  actual=np.minimum(np.maximum(hp,0),raw);gross+=actual;rawgross+=raw;hp-=raw
  if life:ehp[remains]=np.minimum(a['maxHp'],ehp[remains]+actual[remains]/2)
 wins=(hp>0)&(ehp<=0);dead=hp<=0;censored=(hp>0)&(ehp>0);loss=1-np.maximum(hp,0)/pmax;lo,hi=wilson(int(dead.sum()),n)
 return dict(n=n,raw_attack_base=float(B),accuracy=float(A),healing_mode='automatic_regeneration_and_intrinsic_lifesteal' if healing else 'no_healing',
  mean_start_hp=float(pmax.mean()),mean_player_offense=float(x[:,0].mean()),mean_player_accuracy=float(x[:,2].mean()),mean_player_dr=float(x[:,4].mean()),mean_player_dodge=float(x[:,3].mean()),
  mean_enemy_hit_probability=float(ehit.mean()),mean_player_hit_probability=float(phit.mean()),
  mean_total_fight_hp_loss_fraction=float(loss.mean()),mean_loss_standard_error=float(loss.std()/np.sqrt(n)),
  median_total_fight_hp_loss_fraction=float(np.median(loss)),p90_total_fight_hp_loss_fraction=float(np.quantile(loss,.9)),p99_total_fight_hp_loss_fraction=float(np.quantile(loss,.99)),
  mean_gross_hp_damage_fraction=float((gross/pmax).mean()),mean_uncapped_incoming_packet_sum_fraction=float((rawgross/pmax).mean()),
  player_deaths=int(dead.sum()),player_death_fraction=float(dead.mean()),death_fraction_wilson95_low=lo,death_fraction_wilson95_high=hi,
  player_wins=int(wins.sum()),censored=int(censored.sum()),mean_rounds=float(rounds.mean()),max_rounds=int(rounds.max()),
  mean_player_attempts=float(pat.mean()),mean_enemy_attempts=float(eat.mean()),no_enemy_attack_fraction=float((eat==0).mean()),
  mean_player_landed_strikes=float(planded.mean()),mean_enemy_landed_strikes=float(elanded.mean()),round_horizon=horizon)
