import fs from 'node:fs';
const root=new URL('../../release-v48/source/call-of-the-corpse-gods/dist/src/',import.meta.url);
const hh=await import(new URL('half-here.js',root));
const base=JSON.parse(fs.readFileSync(new URL('../fixtures/travel-v48.json',import.meta.url))).find(c=>c.kind==='stairs').initial;
const cases=[];
for(const kind of ['create','recreate','sync','inactive','unowned','dead','terminal','offfloor','dismiss','legacy-color','custom-color']){
 const r=structuredClone(base);r.player.abilities=['etill-half-here','numo-revert'];r.player.active=[];
 if(kind!=='create')hh.createHalfHere(r,8,8);
 if(kind==='sync'){Object.assign(r.player,{hp:37,energy:43,blood:6,damageBank:4,nightmares:2,reactions:7,adaptiveHearts:{Rat:2}});const a=r.actors[1][0];a.adaptiveDr=13;a.conditions.poisoned=4;r.player.stats.offense=18;r.player.abilityOrder=['numo-revert','etill-half-here'];r.player.equipmentAbilities=['etill-half-here','numo-revert'];}
 if(kind==='inactive')r.player.active=[];
 if(kind==='unowned')r.player.abilities=[];
 if(kind==='dead')r.player.dead=true;
 if(kind==='terminal')r.status='won';
 if(kind==='offfloor')r.depth=2;
 if(kind==='legacy-color')r.actors[1][0].color='#88D9CD';
 if(kind==='custom-color')r.actors[1][0].color='#abcdef';
 const initial=structuredClone(r);let result=null;
 if(kind==='create'||kind==='recreate')result=hh.createHalfHere(r,9,7).id;
 else if(kind==='dismiss')hh.dismissHalfHere(r);
 else if(kind.endsWith('color'))result=hh.normalizeHalfHerePresentation(r.actors[1][0]);
 else hh.syncHalfHere(r);
 cases.push({kind,initial,result,expected:r,origins:hh.halfHereOrigins(r).map(a=>a.id),ownerIds:r.actors[r.depth].map(a=>hh.halfHereOwner(r,a).id),ranges:[[8,8,0],[8,8,1],[4,5,1],[20,20,2]].map(([x,y,d])=>hh.withinHalfHereRange(r,{x,y},d))});
}
{const out=new URL('../fixtures/half-here-v48.json',import.meta.url),tempOut=new URL(out.href+'.tmp');fs.writeFileSync(tempOut,JSON.stringify(cases));fs.renameSync(tempOut,out);}
