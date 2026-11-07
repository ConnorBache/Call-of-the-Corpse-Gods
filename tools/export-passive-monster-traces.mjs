import fs from 'node:fs';
const root=new URL('../../release-v48/source/call-of-the-corpse-gods/dist/src/',import.meta.url);
const {dispatch}=await import(new URL('engine.js',root));
const {createBestiaryActor}=await import(new URL('bestiary.js',root));
const {consumePresentationEvents}=await import(new URL('action-events.js',root));
const {createHalfHere}=await import(new URL('half-here.js',root));
const {monsterApplyYetiAdjacency}=await import(new URL('monster-ai.js',root));
const base=JSON.parse(fs.readFileSync(new URL('../fixtures/travel-v48.json',import.meta.url))).find(c=>c.kind==='stairs').initial;
function initial(kind){const r=structuredClone(base);r.player.abilities=[];r.player.abilityOrder=[];r.player.active=[];r.player.baseMaxHp=1000;r.player.hp=1000;r.player.maxHp=1000;r.player.energy=200;r.player.stats.dr=20;r.player.conditions={};r.ground[1]=[];r.history=[];r.replay=[];r.metrics={commands:0,aiActors:0,sightTests:0};delete r.resourceHistory;delete r.phase;delete r.hasteSnapshot;r.nextId=30;r.floors[0].tiles[255]='floor';const a=createBestiaryActor(kind,10,6,5);a.state='hunting';a.stats.perception=300;a.lastSeen={x:5,y:5};a.conditions.targetId=0;r.actors[1]=[a];return r;}
const cases=[];
for(const kind of ['yeti','phoenix','cancer','ursa-major'])for(let v=0;v<3;v++){
 const r=initial(kind),a=r.actors[1][0];
 if(v===1){a.state='ally';a.perfect=true;a.order='a';const b=createBestiaryActor('hemogoblin',11,8,5);b.state='hunting';b.stats.perception=300;r.actors[1].push(b);}
 if(v===2){if(kind==='phoenix'){a.dead=true;a.heartless=true;a.hp=0;a.corpseAge=90;a.conditions.monsterPhoenixRolls=4;}else if(kind==='ursa-major'){a.state='sleeping';const b=createBestiaryActor('ursa-minor',11,6,5);b.hp=1;r.actors[1].push(b);a.x=9;}else if(kind==='yeti'){r.player.abilities=['etill-half-here'];createHalfHere(r,9,5);a.x=10;a.conditions.hasted=5;}else{a.conditions.lucky=1;r.player.conditions.hasted=3;}}
 consumePresentationEvents(r);const start=structuredClone(r);
 const commands=Array.from({length:8},(_,i)=>kind==='ursa-major'&&v===2&&i===0?{type:'move',dx:1,dy:0}:{type:'wait'});
 const steps=commands.map(command=>{dispatch(r,command,true);return{command,state:structuredClone(r),events:consumePresentationEvents(r)}});
 cases.push({name:`${kind}-${v}`,initial:start,steps});
}
const raw=[];for(let v=0;v<24;v++){const r=initial('yeti');const a=r.actors[1][0];a.x=5+(v%4);a.y=5+Math.floor(v/4)%3;if(v%5===0)a.dead=true;if(v%7===0)a.hp=0;if(v%2===0){r.player.abilities=['etill-half-here'];createHalfHere(r,9,5);}if(v%6===0){a.abilities=[];a.creatureKind='yeti';}consumePresentationEvents(r);const input=structuredClone(r);monsterApplyYetiAdjacency(r);raw.push({input,expected:r});}
for(const [name,data]of[['passive-monster-traces',cases],['monster-chill',raw]]){const p=new URL(`../fixtures/${name}-v48.json`,import.meta.url),t=new URL(p.href+'.tmp');fs.writeFileSync(t,JSON.stringify(data));fs.renameSync(t,p);}console.log(cases.length,raw.length);
const {catchUpFloor}=await import(new URL('offscreen.js',root));
const hidden=[];for(const kind of ['yeti','phoenix','cancer','ursa-major'])for(const dirty of [false,true]){
 const r=initial(kind);r.tick=5;const f=r.floors[1];f.lastTick=0;f.dirty=dirty;f.seen.fill(true);r.actors[2]=r.actors[1];r.actors[1]=[];r.ground[2]=[];const a=r.actors[2][0];a.state='wandering';delete a.lastSeen;delete a.conditions.targetId;if(kind==='phoenix'){a.dead=true;a.hp=0;a.corpseAge=87;a.conditions.monsterPhoenixRolls=2;}
 const input=structuredClone(r);catchUpFloor(r,2);hidden.push({input,expected:r});
}
const hp=new URL('../fixtures/passive-monster-offscreen-v48.json',import.meta.url);fs.writeFileSync(new URL(hp.href+'.tmp'),JSON.stringify(hidden));fs.renameSync(new URL(hp.href+'.tmp'),hp);console.log(hidden.length);
