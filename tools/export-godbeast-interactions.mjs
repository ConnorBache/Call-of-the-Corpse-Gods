import fs from 'node:fs';
import {tapVeridian,activateMercuria} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/godbeasts.js';
const base=JSON.parse(fs.readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url)));
const taps=[],floods=[];
for(let n=0;n<24;n++){
 const r=structuredClone(base);r.floors[0].godbeastEvent={kind:n%6===0?'Mercuria':'Veridian',x:r.player.x+(n%4),y:r.player.y+(Math.floor(n/4)%2),room:{left:1,top:1,right:8,bottom:8}};
 if(n%7===0)r.veridianSapHarvested=true;if(n%8===0)r.floors[1].godbeastEvent={kind:'Veridian',x:1,y:1,tapped:true,room:{left:1,top:1,right:8,bottom:8}};
 const input=structuredClone(r),result=tapVeridian(r);taps.push({input,result,expected:r});
 const f=structuredClone(base.floors[0]);f.godbeastEvent={kind:n%3===0?'Veridian':'Mercuria',x:10,y:10,activated:n%4===0,room:{left:1,top:1,right:8,bottom:8}};if(n%2===0)f.ichor=Array(f.tiles.length).fill(2);const before=structuredClone(f),ok=activateMercuria(f);floods.push({input:before,ok,expected:f});
}
const p=new URL('../fixtures/godbeast-interactions-v48.json',import.meta.url),t=new URL(p.href+'.tmp');fs.writeFileSync(t,JSON.stringify({taps,floods}));fs.renameSync(t,p);console.log(taps.length,floods.length);
