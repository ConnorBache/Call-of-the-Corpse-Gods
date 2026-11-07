import fs from 'node:fs';
import {ABILITIES} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/abilities.js';
import {offerWeight,weightedChoice} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/progression.js';
const cases=[];for(const owned of [[],ABILITIES.slice(0,3).map(a=>a.id),ABILITIES.filter((_,i)=>i%5===0).map(a=>a.id)]){const pool=ABILITIES.filter(a=>!owned.includes(a.id));cases.push({owned,weights:ABILITIES.map(a=>offerWeight(a,owned)),choices:[0,0.1,0.5,0.999999999].map(sample=>({sample,id:weightedChoice(pool,owned,sample).id}))});}
atomicWrite(new URL('../fixtures/progression-v48.json',import.meta.url),JSON.stringify(cases));

function atomicWrite(path,contents){const file=path instanceof URL?path:new URL(path,import.meta.url);const temp=new URL(file.href+".tmp");fs.writeFileSync(temp,contents);fs.renameSync(temp,file);}
