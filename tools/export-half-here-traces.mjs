import fs from 'node:fs';
const root=new URL('../../release-v48/source/call-of-the-corpse-gods/dist/src/',import.meta.url);
const {createHalfHere}=await import(new URL('half-here.js',root));
const {dispatch}=await import(new URL('engine.js',root));
const {createBestiaryActor}=await import(new URL('bestiary.js',root));
const {consumePresentationEvents}=await import(new URL('action-events.js',root));
const base=JSON.parse(fs.readFileSync(new URL('../fixtures/travel-v48.json',import.meta.url))).find(c=>c.kind==='stairs').initial;
const cases=[];
for(const name of ['linked-wait','linked-haste','linked-hit','linked-flight-expiry','linked-travel','linked-revert','linked-legacy-projection']){
 let r=structuredClone(base);r.player.abilities=['etill-half-here',...(name==='linked-revert'?['numo-revert']:[])];r.player.abilityOrder=[...r.player.abilities];r.player.active=[];r.player.energy=200;r.ground[1]=[];r.history=[];r.replay=[];r.metrics={commands:0,aiActors:0,sightTests:0};delete r.resourceHistory;delete r.phase;delete r.hasteSnapshot;if(name!=='linked-legacy-projection')r.player.baseMaxHp=r.player.maxHp;const clone=createHalfHere(r,8,5);
 let commands=[{type:'wait'},{type:'wait'}];
 if(name==='linked-haste')r.player.conditions.hasted=3;
 if(name==='linked-hit'){const rat=createBestiaryActor('rat',r.nextId++,9,5);rat.state='hunting';rat.stats.perception=300;rat.conditions.targetId=clone.id;r.actors[1].push(rat);}
 if(name==='linked-flight-expiry'){r.floors[0].tiles[258]='chasm';clone.conditions.flying=1;}
 if(name==='linked-travel')commands=[{type:'descend'},{type:'wait'}];
 if(name==='linked-revert')commands.push({type:'ability',id:'numo-revert'});
 r=JSON.parse(JSON.stringify(r));const initial=structuredClone(r);const steps=commands.map(command=>{dispatch(r,command,true);return{command,state:structuredClone(r),events:consumePresentationEvents(r)}});cases.push({name,initial,steps});
}
const file=new URL('../fixtures/half-here-traces-v48.json',import.meta.url),tmp=new URL(file.href+'.tmp');fs.writeFileSync(tmp,JSON.stringify(cases));fs.renameSync(tmp,file);
