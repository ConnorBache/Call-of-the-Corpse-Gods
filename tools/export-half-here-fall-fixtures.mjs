import fs from 'node:fs';
const root=new URL('../../release-v48/source/call-of-the-corpse-gods/dist/src/',import.meta.url);
const {createHalfHere}=await import(new URL('half-here.js',root));
const {fallActorsOneLevel,collapseDescent}=await import(new URL('descent.js',root));
const base=JSON.parse(fs.readFileSync(new URL('../fixtures/travel-v48.json',import.meta.url))).find(c=>c.kind==='stairs').initial;
const cases=[];
for(const kind of ['endpoint','endpoint-water','both','endpoint-lethal','collapse-endpoint','collapse-both','offscreen-endpoint']){const r=structuredClone(base);r.player.abilities=['etill-half-here'];r.player.active=[];const clone=createHalfHere(r,7,5);if(!kind.startsWith('collapse'))r.floors[0].tiles[257]='chasm';if(kind==='both')r.floors[0].tiles[255]='chasm';if(kind==='endpoint-water')r.floors[1].tiles[257]='water';if(kind==='endpoint-lethal')r.player.hp=clone.hp=1;if(kind==='offscreen-endpoint')r.depth=3;const ids=kind==='both'?[r.player.id,clone.id]:[clone.id];const initial=structuredClone(r);if(kind.startsWith('collapse'))collapseDescent(r,kind==='collapse-both'?6:8,5,true);else fallActorsOneLevel(r,1,ids.map(id=>id===r.player.id?r.player:clone));cases.push({kind,initial,ids,expected:r});}
{const out=new URL('../fixtures/half-here-falls-v48.json',import.meta.url),tempOut=new URL(out.href+'.tmp');fs.writeFileSync(tempOut,JSON.stringify(cases));fs.renameSync(tempOut,out);}
