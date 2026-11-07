import fs from 'node:fs';
import {BESTIARY,EVENT_CREATURES,createBestiaryActor} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/bestiary.js';
import {initializeEnemyTraits,enemyTraits,energyAbilityCosts,normalizeEnemyPresentation,hasEnemyAbility} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/enemy-data.js';
const data=JSON.parse(fs.readFileSync(new URL('../crates/corpse-core/catalog/enemy-traits-v48.json',import.meta.url))),base=createBestiaryActor('hemogoblin',4,15,14,false),actors=[...BESTIARY,...EVENT_CREATURES].map(d=>createBestiaryActor(d.id,4,15,14,false));
for(const name of[...data.zillenniaNames,...['Biter','Breather','Spitter','Birther','Screamer','Jackal'].flatMap(role=>['Pale '+role,'Greater Pale '+role]),'Vent Fungus','Unknown','Pales Biter','_Pale Biter','漢Pale Biter']){const a=structuredClone(base);a.name=name;delete a.creatureKind;a.abilities=[];a.conditions={};actors.push(a);}
const cases=[];for(const base of actors)for(let variant=0;variant<7;variant++){
 const a=structuredClone(base);a.energy=variant*7;
 if(variant===0){delete a.conditions.bestiaryTraitsInitialized;delete a.conditions.paleTraitsInitialized;a.abilities=[];}
 if(variant===1){a.conditions.bestiaryTraitsInitialized=1;a.abilities=[];}
 if(variant===2){a.glyph=(data.legacyGlyphs[a.name]??['B'])[0];a.color=(data.legacyColors[a.name]??['#D2C9C4'])[0].toUpperCase();}
 if(variant===3){a.name='Mercuria’s '+a.name;a.glyph=(data.legacyGlyphs[base.name]??['B'])[0];a.color='#e5bb4c';}
 if(variant===4){a.name='Renamed '+a.name;a.glyph='?';a.color='#123456';}
 if(variant===5){a.conditions.bestiaryTraitsInitialized=-1;a.abilities=['enemy:beast:detonate','enemy:beast:detonate','unknown','enemy:pale:birth'];}
 if(variant===6){a.conditions.halfHereClone=1;a.conditions.halfHereOwner=0;a.color='#88D9CD';}
 const initial=structuredClone(a),traits=enemyTraits(a),costs=energyAbilityCosts(a),has=Object.fromEntries(Object.keys(data.abilities).map(id=>[id,hasEnemyAbility(a,id)]));const presentation=structuredClone(a);const changed=normalizeEnemyPresentation(presentation);initializeEnemyTraits(a);cases.push({initial,traits,costs,has,presentation,changed,expected:a});
}
const p=new URL('../fixtures/enemy-traits-v48.json',import.meta.url),t=new URL(p.href+'.tmp');fs.writeFileSync(t,JSON.stringify(cases));fs.renameSync(t,p);console.log(cases.length);
