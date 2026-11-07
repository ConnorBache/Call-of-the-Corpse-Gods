import fs from 'node:fs';
import {STARTER_POOLS} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/starter-abilities.js';
import {ABILITIES} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/abilities.js';
import {EQUIPMENT} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/equipment-catalog.js';
import {BESTIARY,EVENT_CREATURES,bestiaryHeart} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/bestiary.js';
import {ZILLENNIA_QUALITIES} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/zillennia-qualities.js';
const names={};for(const d of [...BESTIARY,...EVENT_CREATURES]){names[d.id]=d.name;names[d.name]=d.name;}names['vent-fungus']=names['Vent Fungus']='Bastidion';for(const q of ZILLENNIA_QUALITIES)names[q.name]='Zillennia';
const hearts={};for(const d of [...BESTIARY,...EVENT_CREATURES]){const h=bestiaryHeart({name:d.name,creatureKind:d.id,conditions:{}},0);if(h){hearts[d.id]=h;hearts[d.name]=h;}}for(const q of ZILLENNIA_QUALITIES){hearts['quality:'+q.id]=bestiaryHeart({name:q.name,creatureKind:'zillennia',zillenniaQuality:q.id,conditions:{}},0);hearts['zillennia-name:'+q.name]=hearts['quality:'+q.id];}
for(const [name,value]of [['heart-items-v48',hearts],['starter-pools-v48',STARTER_POOLS.map(p=>({godId:p.godId,abilities:p.abilities.map(a=>a.id)}))],['abilities-v48',ABILITIES],['heart-types-v48',names],['equipment-v48',EQUIPMENT]]){const path=new URL(`../crates/corpse-core/catalog/${name}.json`,import.meta.url);const tmp=new URL(path.href+'.tmp');fs.writeFileSync(tmp,JSON.stringify(value,null,2)+'\n');fs.renameSync(tmp,path);}
