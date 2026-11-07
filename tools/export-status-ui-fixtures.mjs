// Frozen source display oracle. Development only, never loaded as executable code.
import fs from 'node:fs';
import {actorStatuses} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/statuses.js';
import {isHasted,hasOngoingHaste} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/scheduler.js';
import {ConditionMeters,conditionHoverDescription} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/ui-statuses.js';
import {BESTIARY,EVENT_CREATURES,createBestiaryActor} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/bestiary.js';
import {ABILITIES} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/abilities.js';
import {floorWidth} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/dimensions.js';
import {makeEquipment} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/equipment.js';
const base=JSON.parse(fs.readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url),'utf8'));
const id=name=>ABILITIES.find(a=>a.name===name).id;
const clean=()=>({...structuredClone(base.player),conditions:{},abilities:[],active:[],state:'wandering',energy:50});
const cases=[];
function add(name,actor,options={}) {
 const r=structuredClone(base);r.inventory=[];r.actors[r.depth]=[];
 Object.assign(r,options.run??{});
 if(options.playerChanges) Object.assign(r.player,structuredClone(options.playerChanges));
 if(options.player) {r.player=structuredClone(actor);actor=r.player;}
 else {actor={...structuredClone(actor),id:200};if(!options.absent)r.actors[r.depth].push(actor);}
 if(options.observers) r.actors[r.depth].push(...structuredClone(options.observers));
 const at=actor.y*floorWidth(r.floors[r.depth-1])+actor.x;
 if(options.tile)r.floors[r.depth-1].tiles[at]=options.tile;
 for(const field of ['ichor','slime'])if(options[field]!==undefined){r.floors[r.depth-1][field]??=Array(r.floors[r.depth-1].tiles.length).fill(0);r.floors[r.depth-1][field][at]=options[field];}
 const statuses=actorStatuses(r,actor);
 cases.push({name,actor,options,statuses,haste:isHasted(r,actor),ongoing:hasOngoingHaste(r,actor),hover:statuses.map(conditionHoverDescription)});
}
add('clean-player',clean(),{player:true});add('clean-actor',clean());
const timed=['paralyzed','telepathic','invisible','confused','magicSensitive','bleeding','hemophilia','poisoned','slowed','envenomed','grabbed','weakened','emboldened','screamWeakened','screamEmboldened','gasVulnerableTicks','temporaryAlly','cameTurns','rangedRecovery','beastCooldown'];
for(const player of [false,true]) {
 add(`all-timed-${player}`,{...clean(),conditions:Object.fromEntries(timed.map((k,i)=>[k,i+1]))},{player});
 add(`no-countdown-${player}`,{...clean(),conditions:Object.fromEntries(timed.map((k,i)=>[k,i%2?-1:0]))},{player});
}
add('internal-values-never-statuses',{...clean(),conditions:{seesPlayer:1,targetId:99,poisonSourceId:40,damageSinceLastTurn:3,corpseClock:100,halfHereOwner:1,halfHereClone:1,monsterVampireBat:1,flyingFresh:1}});
add('flags-venom-buffs-remembered',{...clean(),conditions:{lucky:1,unlucky:1,distracted:1,silenced:1,auraSilenced:1,underwater:1,stuck:1,laserCharge:1,riptide:1,envenomed:3,'venom:a':1.2,'venom:b':3.3,'venom:c':-1,'buff:offense':2.5,'buffTicks:offense':6,'buff:dr':-0.5,'buff:maxEnergy':0.4,'remembered:kative-dilute-potion':12,'remembered:unknown':1}},{run:{tick:5}});
for(const state of ['sleeping','ally','neutral','wandering'])for(const dead of [false,true])add(`sleep-${state}-${dead}`,{...clean(),state,dead,conditions:{monsterAllyRest:1,monsterRecoverySleep:1}});
for(const ability of [null,'Star Power','enemy:beast:everburn'])for(const burning of [0,5])for(const dead of [false,true]) {
 const abilities=ability?[ability.startsWith('enemy:')?ability:id(ability)]:[];
 add(`burn-${ability}-${burning}-${dead}`,{...clean(),abilities,dead,conditions:{burning}});
 if(ability==='Star Power')add(`burn-suppressed-${burning}-${dead}`,{...clean(),abilities,active:abilities,dead,conditions:{burning}});
}
for(const mode of ['timed','flight','Ascend','Phase Through','vampire','Dream'])for(const timer of [0,7]) {
 const a={...clean(),conditions:{flying:timer},abilities:[],active:[]}; const options={player:true};
 if(mode==='flight')a.abilities.push('enemy:beast:flight');
 if(mode==='Ascend')a.abilities.push(id(mode));
 if(mode==='Phase Through')a.active.push(id(mode));
 if(mode==='vampire'){a.abilities.push('enemy:beast:vampire');a.conditions.monsterVampireBat=1;}
 if(mode==='Dream'){const item=makeEquipment('Ring',100);item.equipped='ring1';item.enhancements=[{craft:'Dream',bonus:{}}];options.run={inventory:[item]};}
 add(`flight-${mode}-${timer}`,a,options);
}
for(const mode of ['timed','fast','Rushing Torrent','Sleep Walk','Step Through Shadows','vampire'])for(const slowed of [0,2]) {
 const a=clean();a.conditions.slowed=slowed;
 if(mode==='timed')a.conditions.hasted=5;
 if(mode==='fast')a.abilities.push('enemy:beast:fast');
 if(mode==='Rushing Torrent')a.active.push(id(mode));
 if(mode==='Sleep Walk'){a.abilities.push(id(mode));a.energy=0;}
 if(mode==='Step Through Shadows')a.abilities.push(id(mode));
 if(mode==='vampire'){a.abilities.push('enemy:beast:vampire');a.conditions.monsterVampireBat=1;}
 add(`haste-${mode}-${slowed}`,a,{player:true});
}
for(const sees of [0,1])for(const state of ['wandering','hunting','ally','neutral'])for(const hidden of [false,true]) {
 const a={...clean(),abilities:[id('Step Through Shadows')],conditions:hidden?{invisible:3}:{}};
 const observer={...clean(),id:300,x:a.x+1,y:a.y,state,conditions:{seesPlayer:sees,telepathic:hidden?0:2}};
 add(`shadow-${sees}-${state}-${hidden}`,a,{player:true,observers:[observer]});
}
for(const trait of ['aquatic','ichor-bound','slime-bound'])for(const tile of ['floor','water','shallow-water'])for(const ichor of [0,2]) {
 add(`habitat-${trait}-${tile}-${ichor}`,{...clean(),abilities:[`enemy:beast:${trait}`]},{tile,ichor,slime:trait==='slime-bound'?1:0});
}
for(const def of [...BESTIARY,...EVENT_CREATURES]) {
 const actor=createBestiaryActor(def.id,200,base.player.x+1,base.player.y);
 actor.abilities=[];actor.conditions={};
 add(`legacy-${def.id}`,actor);
 if(def.abilities.includes('enemy:beast:slow')) {
  add(`erased-${def.id}`,{...actor,conditions:{bestiaryTraitsInitialized:1}});
  add(`legacy-name-${def.id}`,{...actor,creatureKind:undefined,name:`Mercuria’s ${actor.name}`});
 }
}
for(const name of ['Pale Birther','Greater Pale Birther','Mercuria’s Pale Birther','Pale_Birther','SomethingPale Birther'])add(`legacy-${name}`,{...clean(),name});
const links=[['morae-bonded-by-unseen-threads','bondedTarget'],['pelian-i-came-for-you','cameTarget','cameTurns'],['isheilah-make-slippery','slipperyTarget'],['etill-riptide','riptideTarget'],['dawel-unspeak','unspeakTarget']];
for(const [ability,key,timer] of links)for(const player of [false,true]) {
 const a=clean();const playerChanges={conditions:{[key]:200,...(timer?{[timer]:4}:{})},active:[ability]};
 if(player)Object.assign(a,playerChanges);
 add(`link-${ability}-${player}`,a,{player,playerChanges});
 if(!player) {
  add(`absent-link-${ability}`,a,{absent:true,playerChanges});
  add(`dead-link-${ability}`,{...a,dead:true},{playerChanges});
  add(`own-clone-link-${ability}`,{...a,conditions:{halfHereClone:1,halfHereOwner:base.player.id}},{playerChanges});
 }
}
const meterCases=[];
const episodes=[[],[{name:'Poisoned',duration:5}],[{name:'Poisoned',duration:3}],[{name:'Poisoned',duration:8}],[{name:'Flying',duration:null}],[],[{name:'Poisoned',duration:2}],[{name:'Remember: A',duration:0}]];
for(const start of [0,1]) {
 const meters=new ConditionMeters(),actor={};const rows=[];
 for(const statuses of episodes.slice(start))rows.push({statuses,expected:meters.measure(actor,statuses)});
 meterCases.push({rows});
}
fs.writeFileSync(new URL('../fixtures/status-ui-v48.json',import.meta.url),JSON.stringify({cases,meterCases}));
fs.writeFileSync(new URL('../crates/corpse-desktop/assets/status-traits-v48.json',import.meta.url),JSON.stringify([...BESTIARY,...EVENT_CREATURES].map(d=>({id:d.id,name:d.name,slow:d.abilities.includes('enemy:beast:slow')}))));
console.log(`Exported ${cases.length} status cases and ${meterCases.length} meter sequences.`);
