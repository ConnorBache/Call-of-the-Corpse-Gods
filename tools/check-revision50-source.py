#!/usr/bin/env python3
"""Source/data checks only. Does not parse/compile Rust or prove gameplay behavior."""
import json,pathlib,re
root=pathlib.Path(__file__).resolve().parents[1]
def read(path): return (root/path).read_text()
def data(path): return json.loads(read(path))
entries=data('crates/corpse-core/catalog/abilities-v50.json')
amend=data('crates/corpse-core/catalog/ability-description-amendments-v50.json')
ident=data('crates/corpse-core/catalog/ability-identities-v50.json')
source=read('docs/revision50-request.txt').split('# FULL ABILITY ROSTER\n')[1].split('# IMPORTANT UNRESOLVED')[0]
expected=[]
for line in source.splitlines():
 m=re.match(r'^\d+\\\. (.+)$',line)
 if m:expected.append([m.group(1),''])
 elif not line.startswith('## ') and expected:expected[-1][1]+=line+'\n'
assert len(entries)==len(ident)==len(expected)==72
assert len({e['id'] for e in entries})==72
assert len(amend)==13
for i,(e,(name,description),mapping) in enumerate(zip(entries,expected,ident)):
 assert e['name']==name and e['godId']==i//6
 assert e['description']==amend.get(e['id'],description.strip()),name
 assert mapping['revision50Id']==e['id'] and mapping['name']==name
 assert mapping['slot']==i%6+1 and 'craft' not in e['flags']
assert next(e['name'] for e in entries if e['godId']==10 and ident[entries.index(e)]['slot']==4)=='Appropriate'
lib=read('crates/corpse-core/src/lib.rs')
modules=['revision50','revision50_inventory','revision50_combat','revision50_terrain','revision50_pursuit','revision50_nightmares','revision50_grappler','revision50_ability_energy','revision50_amulets','revision50_player_attack','actor_equipment','revision50_rings','revision50_amulet_effects','revision50_control_amulets','revision50_wave_teleport','revision50_potions','revision50_item_knowledge']
for name in modules:
 assert f'pub mod {name};' in lib and (root/f'crates/corpse-core/src/{name}.rs').is_file(),name
model=read('crates/corpse-core/src/model_generated.rs')
for key in ['abilityReserves','copiedInventory','additionalGrantedAbilities','revision50Terrain']:
 assert f'rename = "{key}"' in model,key
storage=read('crates/corpse-core/src/storage.rs').split('pub fn validate_current_revision',1)[1]
assert 'Some(29.0' in storage and '| 50.0' in storage,'Coordinated revision-50 admission must be present'
assert any(f'create_run_revision(&seed, mode, false, {v}.0)' in read('crates/corpse-desktop/src/main.rs') for v in [50,51])
inventory=read('crates/corpse-core/src/revision50_inventory.rs')
assert 'pub shared_jewelry' not in inventory,'R50 jewelry categories must have independent budgets'
for category in ['rings','amulets','potions']:
 assert re.search(r'\b'+category+r': 5\.0 \+',inventory),category+' must start at five'
assert 'hearts: 5.0' in inventory and 'trinkets: None' in inventory
unlimited=inventory.split('pub fn unlimited',1)[1].split('\n}',1)[0].split('pub fn units',1)[0]
assert 'worn(' not in unlimited and 'Category::Trinket' in unlimited,'Only trinkets are unlimited'
assert 'all carried rings and amulets are automatically active' in amend['kative-tinker']
pack=read('crates/corpse-desktop/src/pack.rs')
assert 'fn category_headers' in pack and all('format!("'+kind in pack for kind in ['Trinkets','Amulets','Rings','Potions','Hearts'])
print(json.dumps({'status':'source_contract_checks_passed','scope':'source-and-data-only','catalog_entries':72,'authorized_description_amendments':13,'new_modules_registered':len(modules),'independent_inventory_categories':5,'carried_jewelry_counted':True,'checks_native_compilation':False,'checks_runtime_tests':False,'revision50_release_enabled':True,'release_runtime_verified_by_this_check':False},indent=2))
