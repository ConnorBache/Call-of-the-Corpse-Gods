import fs from 'node:fs';
import {createRun,dispatch} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/engine.js';
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
for(let c=0;c<6;c++){
 const r=createRun(`00-09-10:independent-native-walk-${c}`,'Avatar');const initial=structuredClone(r);let previous=strip(r);const steps=[],visits=new Map();
 const dirs=[[0,-1],[1,0],[0,1],[-1,0]];
 for(let n=0;n<80&&r.status==='playing';n++){
  const f=r.floors[r.depth-1],w=f.width??79,x=r.player.x,y=r.player.y;visits.set(`${x},${y}`,(visits.get(`${x},${y}`)??0)+1);
  const options=dirs.map(([dx,dy],i)=>({dx,dy,i,t:f.tiles[(y+dy)*w+x+dx],count:visits.get(`${x+dx},${y+dy}`)??0})).filter(o=>['floor','grass','rubble','door','shallowWater','bridge','ash','sand','stone','mud'].includes(o.t));
  options.sort((a,b)=>a.count-b.count||((a.i+c+n)%4)-((b.i+c+n)%4));
  const o=options[0];const command=o?{type:'move',dx:o.dx,dy:o.dy}:{type:'wait'};
  dispatch(r,command,true);const after=strip(r);steps.push({command,patch:diff(previous,after)});previous=after;
 }
 cases.push({name:`generated-walk-${c}`,initial,steps});
}
const dest=new URL('../fixtures/generated-walks-v48.json',import.meta.url);const tmp=new URL(dest.href+'.tmp');fs.writeFileSync(tmp,JSON.stringify(cases));fs.renameSync(tmp,dest);
console.log(JSON.stringify({cases:cases.length,commands:cases.map(c=>c.steps.length),bytes:fs.statSync(dest).size}));
