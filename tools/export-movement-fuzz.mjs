import fs from 'node:fs';
import { dispatch } from '../../release-v48/source/call-of-the-corpse-gods/dist/src/engine.js';
import { createBestiaryActor } from '../../release-v48/source/call-of-the-corpse-gods/dist/src/bestiary.js';
import { rememberVisibleTerrain } from '../../release-v48/source/call-of-the-corpse-gods/dist/src/perception.js';
import { resetPresentationEpoch } from '../../release-v48/source/call-of-the-corpse-gods/dist/src/presentation-context.js';
const base=JSON.parse(fs.readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url)));
const strip=r=>{const x=structuredClone(r);delete x.history;delete x.hasteSnapshot;return x;};
function diff(a,b,path=[],result=[]){
 if(JSON.stringify(a)===JSON.stringify(b))return result;
 if(a && b && typeof a==='object' && typeof b==='object' && Array.isArray(a)===Array.isArray(b) && (!Array.isArray(a)||a.length===b.length)){
  for(const k of Object.keys(a))if(!(k in b))result.push({path:[...path,k],remove:true});
  for(const k of Object.keys(b)){if(!(k in a))result.push({path:[...path,k],value:b[k]});else diff(a[k],b[k],[...path,k],result);}return result;
 }
 result.push({path,value:b});return result;
}
const cases=[];
for(let c=0;c<20;c++){
 const r=structuredClone(base);r.rng=(1234567+c*89173)>>>0;r.player.stats.defense=150;r.player.maxHp=1000;r.player.hp=1000;r.nextId=7;
 r.actors[1]=[[16,14],[20,16],[11,10]].map(([x,y],i)=>{const a=createBestiaryActor('hemogoblin',4+i,x,y,false);a.state=['hunting','wandering','sleeping','ally'][(c+i)%4];if(a.state==='hunting')a.lastSeen={x:12,y:14};return a;});
 const f=r.floors[0];for(let y=9;y<21;y++)if(y!==13 && y!==18)f.tiles[y*79+18]='wall';
 f.seen.fill(false);for(const k of Object.keys(f))if(k.startsWith('remembered'))delete f[k];resetPresentationEpoch(r);rememberVisibleTerrain(r);
 const initial=structuredClone(r);let previous=strip(r),input=876543+c*1753;const steps=[];
 const dirs=[[0,-1],[1,0],[0,1],[-1,0],[-1,-1],[1,-1],[-1,1],[1,1]];
 for(let n=0;n<40 && r.status==='playing';n++){
  input^=input<<13;input^=input>>>17;input^=input<<5;input>>>=0;let command={type:'wait'};
  if(input%5){const [dx,dy]=dirs[input%8];const tile=f.tiles[(r.player.y+dy)*79+r.player.x+dx];if(tile!=='up'&&tile!=='down')command={type:'move',dx,dy};}
  dispatch(r,command,true);const after=strip(r);steps.push({command,patch:diff(previous,after)});previous=after;
 }
 cases.push({name:`movement-${c}`,initial,steps});
}
const dest=new URL('../fixtures/movement-fuzz-v48.json',import.meta.url);const tmp=new URL(dest.href+'.tmp');fs.writeFileSync(tmp,JSON.stringify(cases));fs.renameSync(tmp,dest);
console.log(JSON.stringify({cases:cases.length,commands:cases.reduce((n,c)=>n+c.steps.length,0),bytes:fs.statSync(dest).size}));
