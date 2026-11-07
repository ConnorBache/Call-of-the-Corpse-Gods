import fs from 'node:fs';
const root=new URL('../../release-v48/source/call-of-the-corpse-gods/dist/src/',import.meta.url);
const {enemyMoved}=await import(new URL('enemy-expansion.js',root));
const {createBestiaryActor}=await import(new URL('bestiary.js',root));
const base=JSON.parse(fs.readFileSync(new URL('../fixtures/travel-v48.json',import.meta.url))).find(c=>c.kind==='stairs').initial;
const cases=[];
for(const kind of ['glim','glim-forced','glim-obstacles','submerged','ichor','emerged','yeti','yeti-chasm']){
 const r=structuredClone(base), f=r.floors[0], a=createBestiaryActor(kind.startsWith('glim')?'glim':kind.startsWith('yeti')?'yeti':'river-drake',10,7,7);r.actors[1]=[a];
 if(kind==='glim-forced')a.conditions.forcedMove=1;
 if(kind==='glim-obstacles'){for(const [i,t] of ['wall','tree','water','shallow-water','chasm','pit'].entries())f.tiles[306+i%3+Math.floor(i/3)*50]=t;}
 if(kind==='submerged'||kind==='ichor')f.tiles[357]='water';
 if(kind==='yeti-chasm')f.tiles[357]='chasm';
 if(kind==='ichor'){f.ichor=Array(2500).fill(0);f.ichor[357]=1;}
 const initial=structuredClone(r);enemyMoved(r,a);cases.push({kind,initial,expected:r});
}
fs.writeFileSync(new URL('../fixtures/movement-hooks-v48.json',import.meta.url),JSON.stringify(cases));
