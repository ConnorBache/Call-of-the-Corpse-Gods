import fs from 'node:fs';
import {dispatch} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/engine.js';
const r=JSON.parse(fs.readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url)));r.actors[1]=[];
const initial=structuredClone(r),steps=[];
for(const command of [{type:'equip',id:2,slot:'hand2'},{type:'unequip',id:2},{type:'equip',id:1,slot:'hand1'},{type:'ability',id:'shrail-unyielding-assault'},{type:'ability',id:'shrail-unyielding-assault'},{type:'ability',id:'kative-dilute-potion'},{type:'drop',id:3},{type:'move',dx:1,dy:0},{type:'move',dx:-1,dy:0},{type:'pickup'}]){dispatch(r,command,true);steps.push({command,state:structuredClone(r)});}
atomicWrite(new URL('../fixtures/inventory-trace-v48.json',import.meta.url),JSON.stringify({initial,steps}));

function atomicWrite(path,contents){const file=path instanceof URL?path:new URL(path,import.meta.url);const temp=new URL(file.href+".tmp");fs.writeFileSync(temp,contents);fs.renameSync(temp,file);}
