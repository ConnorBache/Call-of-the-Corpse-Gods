import fs from 'node:fs';
const root=new URL('../../release-v48/source/call-of-the-corpse-gods/dist/src/',import.meta.url);
const {dispatch}=await import(new URL('engine.js',root));const {consumePresentationEvents}=await import(new URL('action-events.js',root));
const base=JSON.parse(fs.readFileSync(new URL('../fixtures/travel-v48.json',import.meta.url))).find(c=>c.kind==='stairs').initial,cases=[];
for(let n=0;n<9;n++){
 const r=structuredClone(base);r.player.abilities=[];r.player.active=[];r.player.conditions={};r.actors[1]=[];r.ground[1]=[];r.history=[];r.replay=[];r.metrics={commands:0,aiActors:0,sightTests:0};delete r.phase;delete r.hasteSnapshot;delete r.resourceHistory;
 const f=r.floors[0];f.tiles[255]='floor';f.godbeastEvent={kind:'Mercuria',x:8,y:5,well:{x:8,y:5},lever:{x:6,y:5},room:{left:3,top:3,right:12,bottom:9}};
 if(n===1)r.player.conditions.hasted=4;
 if(n===2){f.mud=Array(f.tiles.length).fill(0);f.mud[r.player.y*50+r.player.x]=1;}
 if(n===3)r.player.conditions.hemophilia=5;
 if(n===4)f.godbeastEvent.rewardCreated=true;
 if(n===5){f.godbeastEvent.activated=true;f.ichor=Array(f.tiles.length).fill(0);f.ichor[258]=100;}
 const initial=structuredClone(r),commands=[n>=6?{type:'interact',x:n===7?12:6,y:5}:{type:'move',dx:1,dy:0},...Array.from({length:7},(_,i)=>n===8&&i===0?{type:'interact',x:6,y:5}:{type:'wait'})];
 const steps=commands.map(command=>{dispatch(r,command,true);return{command,state:structuredClone(r),events:consumePresentationEvents(r)}});cases.push({name:`lever-${n}`,initial,steps});
}
const p=new URL('../fixtures/mercuria-lever-traces-v48.json',import.meta.url),t=new URL(p.href+'.tmp');fs.writeFileSync(t,JSON.stringify(cases));fs.renameSync(t,p);console.log(cases.length);
