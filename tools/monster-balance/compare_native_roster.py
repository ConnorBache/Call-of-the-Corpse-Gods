#!/usr/bin/env python3
"""Reproducible native before/after roster audit, never a replacement combat model.

Pairs identical joint player profiles and combat seeds between revisions. Native
arena includes accuracy, dodge, DR, regen, energy, cooldowns, resistances, native
AI and statuses. A bounded room/player strategy is not an emergent-floor model.
Writes each result as it arrives; errors/censors remain visible in JSON and CSV.
"""
import argparse, csv, hashlib, json, math, subprocess, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CAT = ROOT / 'crates/corpse-core/src/catalog'

def read(path):
    return json.loads(Path(path).read_text())

def default_cases():
    catalog = read(CAT/'population-v48.json')
    roster = {v['id']: v for v in read(CAT/'ordinary-roster-v48.json')}
    overlay_path = CAT/'physical-balance-v32.json'
    if not overlay_path.exists():
        overlay_path = CAT/'physical-balance-v31.json'
    tuning = {v['kind']: v for v in read(overlay_path)}
    result = []
    for kind, a in catalog['actors'].items():
        abilities = a['abilities']
        leader = next((k for k, v in catalog['pairs'].items() if v == kind), kind)
        placement = roster.get(leader)
        depth = tuning.get(kind, {}).get('reference_depth')
        if depth is None and placement:
            depth = (placement['first'] + placement['last']) / 2
        if depth is None:
            # Explicit provisional references, not invented spawn bounds.
            depth = 1 if kind == 'bastidion' else 30 if kind == 'pale:Greater Jackal' else 20
        scenario = 'adjacent'
        if 'enemy:beast:ichor-bound' in abilities:
            scenario = 'ichor-shore'
        elif 'enemy:beast:aquatic' in abilities:
            scenario = 'water-shore'
        elif kind == 'hemogoblin-mech':
            scenario = 'piloted-mech'
        elif kind in ['vampire','wendigo'] or any(x in abilities for x in ['enemy:beast:horn-bolt','enemy:beast:witch','enemy:beast:dryad','enemy:beast:bloodmancy','enemy:beast:moon-laser','enemy:pale:spit']):
            scenario = 'chase'
        result.append(dict(kind=kind, role=tuning.get(kind,{}).get('role','unclassified'),reference_depth=depth,scenario=scenario))
    result.append(dict(kind='flesh-administrator',role='boss_exception',reference_depth=40,scenario='adjacent'))
    for kind in ['pale:Birther','pale:Greater Birther']:
        row = next(r for r in result if r['kind']==kind)
        result.append(dict(row,scenario='three-cast-children',qualification='Isolated three-cast payload; parent removed; includes child descendants.'))
    return result

def profiles_at(blob, track, depth, trials):
    profiles = blob['profiles'][track]['entry']
    lo = max(0, min(39, math.floor(depth)-1))
    hi = max(0, min(39, math.ceil(depth)-1))
    # Alternate adjacent layers for half-floor reference; each seed stays joint.
    return [profiles[i % len(profiles)][lo if i % 2 == 0 else hi] for i in range(trials)]

def save(rows, output):
    output.parent.mkdir(parents=True,exist_ok=True)
    output.write_text(json.dumps(rows,indent=2)+'\n')
    keys=['kind','role','scenario','settlement_strategy','partner_kind','without_support','reference_depth','layer_position','reference_policy','target_gross_fraction','status','revision','trials','profiles','mean_gross_combat_damage_fraction','gross_combat_standard_error','mean_gross_damage_fraction','mean_gross_postdeath_damage_fraction','mean_status_channel_postdeath_damage_fraction','mean_capped_actual_damage_fraction','mean_cost','mean_attacks_to_kill_conditional_on_kill','mean_world_turns_to_kill_conditional_on_kill','mean_enemy_attacks_over_killed_lifespan','mean_player_attack_attempts','mean_enemy_attack_attempts','death_rate','censor_rate','kills','deaths','combat_deaths','residual_deaths','censored','combat_censored','aftermath_censored','all_primary_lifespans_observed','all_settlements_observed','error']
    with output.with_suffix('.csv').open('w',newline='') as f:
        writer=csv.DictWriter(f,fieldnames=keys,extrasaction='ignore');writer.writeheader();writer.writerows(rows)

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--binary',type=Path,default=ROOT/'target/release/examples/physical_trade_arena')
    p.add_argument('--profiles',type=Path,required=True,help='Actual-generator joint-profiles JSON; same profiles used for both revisions.')
    p.add_argument('--track',default='all_initial_unblessed')
    p.add_argument('--before-profiles',type=Path,help='Optional revision31 cohort for an overall-placement comparison; default uses SAME after cohort both revisions.')
    p.add_argument('--placements',type=Path,help='Approved flavor placement proposal JSON; includes explicit N/A rows.')
    p.add_argument('--cases',type=Path,help='JSON [{kind,role,reference_depth,scenario,...}]; overrides derived references.')
    p.add_argument('--write-cases',type=Path,help='Write derived cases then exit, for explicit review before running.')
    p.add_argument('--output',type=Path,default=ROOT/'tools/monster-balance/native-roster-before-after.json')
    p.add_argument('--revisions',type=int,nargs='+',default=[31,32])
    p.add_argument('--trials',type=int,default=128)
    p.add_argument('--max-turns',type=int,default=256)
    p.add_argument('--settle-turns',type=int,default=128)
    p.add_argument('--kinds',nargs='*',help='Optional exact kind filter, not hidden in output.')
    args=p.parse_args()
    cases=read(args.cases) if args.cases else default_cases()
    if args.placements:
        derived={r['kind']:r for r in cases if r['scenario']!='three-cast-children'}
        cases=[]
        for item in read(args.placements)['entries']:
            kind=item['id']
            case=dict(derived.get(kind,dict(kind=kind,scenario='chase' if kind.startswith('zillennia:') else 'adjacent')))
            case.update(role=item.get('role'),reference_depth=item.get('reference_depth'),category=item.get('category'),target_gross_fraction=(item['target_lifetime_loss_pct']/100 if isinstance(item.get('target_lifetime_loss_pct'),(int,float)) else None),qualification=item.get('rationale',''))
            cases.append(case)
            if kind in ['pale:Birther','pale:Greater Birther']:
                cases.append(dict(case,scenario='three-cast-children',target_gross_fraction=item.get('three_cast_child_budget_pct',0)/100,qualification='Isolated three-cast payload; parent removed; includes child descendants.'))
    if args.kinds: cases=[r for r in cases if r['kind'] in args.kinds]
    if args.write_cases:
        args.write_cases.write_text(json.dumps(cases,indent=2)+'\n');return
    blob=read(args.profiles)
    before=read(args.before_profiles) if args.before_profiles else None
    if before is not None:
        common=[seed for seed in blob['seeds'] if seed in before['seeds']]
        if not common: raise ValueError('No shared player seed cohorts between before/after profiles')
        for source in [blob,before]:
            indices=[source['seeds'].index(seed) for seed in common]
            source['profiles'][args.track]['entry']=[source['profiles'][args.track]['entry'][i] for i in indices]
            source['seeds']=common
    metadata=dict(profile_file=str(args.profiles),profile_sha256=hashlib.sha256(args.profiles.read_bytes()).hexdigest(),track=args.track,profile_revision=blob.get('revision'),paired_same_profiles=before is None,paired_same_seeds=True,before_profile_file=str(args.before_profiles) if before else None,before_profile_sha256=hashlib.sha256(args.before_profiles.read_bytes()).hexdigest() if before else None)
    rows=[]
    proc=subprocess.Popen([str(args.binary.resolve())],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True,bufsize=1)
    try:
        for case in cases:
            if case.get('reference_depth') is None:
                for revision in args.revisions:
                    rows.append(dict(case,revision=revision,baseline=metadata,status='not_applicable',measurement='No finite one-to-one kill-life target: '+case.get('qualification','No approved reference depth')))
                save(rows,args.output)
                continue
            for revision in args.revisions:
                profiles=profiles_at(before if revision==31 and before is not None else blob,args.track,float(case['reference_depth']),args.trials)
                request={k:case[k] for k in ['kind','scenario','candidate','partner_candidate','partner_kind','without_support','summoned_candidates','settlement_strategy'] if k in case}
                request.update(revision=revision,profiles=profiles,trials=args.trials,max_turns=args.max_turns,settle_turns=args.settle_turns,seed_prefix=f"monster32-heldout:{case['kind']}:{case.get('scenario','adjacent')}")
                proc.stdin.write(json.dumps(request)+'\n');proc.stdin.flush()
                line=proc.stdout.readline()
                if not line: raise RuntimeError(f'Native arena ended during {case["kind"]} revision{revision}, code={proc.poll()}')
                result=json.loads(line)
                result.update({k:v for k,v in case.items() if k not in ['candidate','partner_candidate']})
                result.update(revision=revision,baseline=metadata)
                rows.append(result);save(rows,args.output)
                print(case['kind'],case.get('scenario'),revision,'gross',result.get('mean_gross_damage_fraction'),'net',result.get('mean_cost'),'deaths',result.get('deaths'),'censored',result.get('censored'),'error',result.get('error',''),flush=True)
    finally:
        proc.stdin.close();proc.wait()
    print(f'Wrote {len(rows)} observations to {args.output}',flush=True)

if __name__=='__main__': main()
