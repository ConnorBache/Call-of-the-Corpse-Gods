import fs from 'node:fs';
import {useAbility} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/skills-runtime.js';
import {learnAbility} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/equipment-abilities.js';
import {consumePresentationEvents} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/action-events.js';
const base=JSON.parse(fs.readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url)));base.floors=base.floors.slice(0,1);base.ground={1:[]};
const catalog=JSON.parse(fs.readFileSync(new URL('../crates/corpse-core/catalog/abilities-v48.json',import.meta.url)));const cases=[];
for(const ability of catalog.filter(a=>a.flags.includes('passive'))){const r=structuredClone(base);learnAbility(r,'numo-remember');r.player.energy=100;const enemy=r.actors[1][0];enemy.abilities=[ability.id];const initial=structuredClone(r);useAbility(r,{type:'ability',id:'numo-remember',target:enemy.id});cases.push({name:ability.id,initial,state:r,events:consumePresentationEvents(r)});}
const file=new URL('../fixtures/remember-passives-v48.json',import.meta.url),tmp=new URL(file.href+'.tmp');fs.writeFileSync(tmp,JSON.stringify(cases));fs.renameSync(tmp,file);
