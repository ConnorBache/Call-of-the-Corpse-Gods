import {readFileSync,writeFileSync} from 'node:fs';
import {createHalfHere,dismissHalfHere} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/half-here.js';
import {explode} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/environment.js';
import {consumePresentationEvents} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/action-events.js';
const scenes=JSON.parse(readFileSync(new URL('../crates/corpse-core/tests/fixtures/environment-active-v48.json',import.meta.url),'utf8'));const cases=[];
for(const name of ['explosive','hydrogen','multi-target'])for(const igniteFirst of [false,true])for(const igniteArea of [false,true]){
 const initial=structuredClone(scenes.find(s=>s.name===name).initial);const x=initial.player.x,y=initial.player.y;const expected=structuredClone(initial);explode(expected,x,y,2,expected.player,igniteFirst,false,igniteArea,undefined,true);
 cases.push({name,igniteFirst,igniteArea,initial,expected,events:consumePresentationEvents(expected)});
}
for(const hp of [1000,1])for(const igniteFirst of [false,true])for(const igniteArea of [false,true]){
 const initial=structuredClone(scenes.find(s=>s.name==='explosive').initial);
 initial.player.abilities.push('etill-half-here');initial.player.hp=initial.player.maxHp=initial.player.baseMaxHp=hp;
 createHalfHere(initial,initial.player.x+1,initial.player.y);
 const expected=structuredClone(initial);explode(expected,expected.player.x,expected.player.y,2,expected.player,igniteFirst,false,igniteArea,undefined,true);
 cases.push({name:`half-here-${hp}`,igniteFirst,igniteArea,initial,expected,events:consumePresentationEvents(expected)});
}
for(const active of [false,true]){const initial=structuredClone(scenes.find(s=>s.name==='explosive').initial);initial.player.abilities.push('anavani-nova');if(active)initial.player.active.push('anavani-nova');const expected=structuredClone(initial);explode(expected,expected.player.x,expected.player.y,2,expected.player,false,false,true,undefined,true);cases.push({name:`nova-immunity-${active}`,igniteFirst:false,igniteArea:true,initial,expected,events:consumePresentationEvents(expected)});}
{const initial=structuredClone(scenes.find(s=>s.name==='explosive').initial);initial.player.abilities.push('etill-half-here');const sourceSnapshot=createHalfHere(initial,initial.player.x+1,initial.player.y);dismissHalfHere(initial);const expected=structuredClone(initial);explode(expected,expected.player.x,expected.player.y,2,sourceSnapshot,false,false,true,undefined,true);cases.push({name:'retained-clone-source',igniteFirst:false,igniteArea:true,sourceSnapshot,initial,expected,events:consumePresentationEvents(expected)});}
writeFileSync(new URL('../crates/corpse-core/tests/fixtures/explosion-v48.json',import.meta.url),JSON.stringify(cases));
