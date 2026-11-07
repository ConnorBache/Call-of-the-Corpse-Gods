import {readFileSync,writeFileSync} from 'node:fs';
import {settlePhaseLoss,phaseCrushingEndpoint} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/upkeep.js';
import {createHalfHere} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/half-here.js';
import {consumePresentationEvents} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/action-events.js';
const base=JSON.parse(readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url)));const cases=[];
for(const name of ['floor','active-wall','wall','clone-wall','pit','flying-pit','deferred-pit','clone-pit']){
const r=structuredClone(base);r.actors={'1':[]};r.player.abilities=['etill-phase-through'];r.player.active=[];r.player.conditions={};r.player.hp=r.player.maxHp=100;r.history=[];r.replay=[];
const f=r.floors[0],w=f.width??79,at=r.player.y*w+r.player.x;
if(name.includes('wall'))f.tiles[at]='wall';if(name.includes('pit'))f.tiles[at]='pit';
if(name==='active-wall')r.player.active=['etill-phase-through'];if(name==='flying-pit')r.player.conditions.flying=3;
if(name.startsWith('clone-')){r.player.abilities.push('etill-half-here');const a=createHalfHere(r,r.player.x+1,r.player.y);f.tiles[a.y*w+a.x]=name==='clone-wall'?'wall':'pit';f.tiles[at]='floor';}
const initial=structuredClone(r),endpoint=phaseCrushingEndpoint(r)?.id??null;settlePhaseLoss(r,name==='deferred-pit');cases.push({name,initial,endpoint,defer:name==='deferred-pit',expected:r,events:consumePresentationEvents(r)});
}
writeFileSync(new URL('../crates/corpse-core/tests/fixtures/divine-environment-v48.json',import.meta.url),JSON.stringify(cases));
