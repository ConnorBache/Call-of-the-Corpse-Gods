// Development-only frozen source oracle: no JS runtime is shipped.
import fs from 'node:fs';
import {planPlayerTravel} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/player-pathing.js';
import {planDisplacedTravel,changedTravelThreat} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/ui-travel.js';
import {ABILITIES} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/abilities.js';
const base=JSON.parse(fs.readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url),'utf8'));
const id=name=>ABILITIES.find(a=>a.name===name).id;
const cases=[];const graphs=[];const threats=[];
function make(){const r=structuredClone(base);r.player.x=1;r.player.y=3;r.player.abilities=[];r.player.active=[];r.player.conditions={};r.inventory=[];r.actors[r.depth]=[];r.phase='normal';r.tick=5;const n=49;const f=r.floors[0];r.floors=[f];for(const key of Object.keys(f))if(Array.isArray(f[key]))delete f[key];Object.assign(f,{tiles:Array.from({length:n},(_,i)=>i<7||i>41||i%7===0||i%7===6?'wall':'floor'),seen:Array(n).fill(true),lights:Array(n).fill(30),ichor:Array(n).fill(0),slime:Array(n).fill(0),currents:Array(n).fill(0),doors:{},rememberedDoors:{},rememberedHazards:{}});f.environment={caustic:Array(n).fill(0),breather:Array(n).fill(0),paralysis:Array(n).fill(0),fire:Array(n).fill(0),gasType:Array(n).fill(null),gasVolume:Array(n).fill(0),fireKind:Array(n).fill(null)};f.rememberedTiles=[...f.tiles];f.rememberedIchor=[...f.ichor];f.rememberedCurrents=[...f.currents];return r}
function add(name,r,mask=Array(49).fill(true),direct=true,target={x:5,y:3},observable=[]){cases.push({name,run:r,mask,direct,target,observable,expected:planPlayerTravel(r,target,mask,a=>observable.includes(a.id),direct)});}
for(const direct of [false,true])for(const phase of ['haste','normal'])for(const terrain of ['floor','wall','tree','shallow-water','water','chasm','pit','up','down','door','grate','fire','embers','permanent','gas-zero','gas','razorleaves','ichor-water']) {
 const r=make(),f=r.floors[0];r.phase=phase;r.player.conditions.hasted=3;
 for(const at of [16,23,30]) {
 if(['fire','embers'].includes(terrain)){f.environment.fire[at]=terrain==='fire'?4:0;f.environment.fireKind[at]=terrain==='fire'?'creature':'embers';}
 else if(terrain==='permanent')f.permanentFire=[...(f.permanentFire??[]),at];
 else if(terrain.startsWith('gas')){f.environment.gasType[at]='caustic';f.environment.gasVolume[at]=terrain==='gas'?40:0;}
 else if(terrain==='door')f.doors[at]={open:false};
 else if(terrain==='grate')f.grates=[...(f.grates??[]),at];
 else if(terrain==='razorleaves'){f.razorleaves??=Array(49).fill(0);f.razorleaves[at]=10;}
 else if(terrain==='ichor-water'){f.tiles[at]='water';f.ichor[at]=1;}
 else f.tiles[at]=terrain;
 }
 add(`${terrain}-${phase}-${direct}`,r,undefined,direct);
}
for(const mobility of ['Ascend','Drown','Phase Through','aquatic','rooted','ichor-bound','slime-bound'])for(const surface of ['shallow-water','water','chasm','wall']) {
 const r=make();r.player.abilities=[mobility.includes('-')||['aquatic','rooted'].includes(mobility)?`enemy:beast:${mobility}`:id(mobility)];if(mobility==='Phase Through')r.player.active=[id(mobility)];r.floors[0].tiles[23]=surface;add(`mobility-${mobility}-${surface}`,r);
}
for(const direct of [false,true])for(const target of [{x:2,y:3},{x:2,y:2},{x:1,y:3},{x:7,y:3},{x:5,y:3}]) {const r=make();r.floors[0].tiles[16]='wall';r.floors[0].tiles[23]='wall';add(`adjacent-${direct}-${target.x}-${target.y}`,r,undefined,direct,target);}
for(const remembered of ['wall','floor','water','shallow-water','chasm'])for(const hazard of [false,true]) {
 const r=make(),f=r.floors[0];f.tiles[23]='wall';f.rememberedTiles[23]=remembered;if(hazard)f.rememberedHazards[23]={gas:{type:'caustic',concentration:0}};add(`memory-${remembered}-${hazard}`,r,Array(49).fill(false));
}
for(const state of ['sleeping','hunting','ally','neutral'])for(const visible of [false,true]) {const r=make();r.player.conditions={cameTurns:3,cameTarget:200};r.actors[r.depth]=[{...structuredClone(r.player),id:200,x:5,y:3,state}];add(`quarry-${state}-${visible}`,r,undefined,false,undefined,visible?[200]:[]);}
let rng=3189;const random=()=>{rng=(Math.imul(rng,1664525)+1013904223)>>>0;return rng/4294967296};
for(let k=0;k<128;k++) {const r=make(),f=r.floors[0];for(let i=0;i<49;i++)if(f.tiles[i]==='floor'&&i!==22&&i!==26) {const n=random();if(n<0.16)f.tiles[i]='wall';else if(n<0.24)f.tiles[i]='water';else if(n<0.36)f.tiles[i]='shallow-water';else if(n<0.46)f.environment.fire[i]=4;else if(n<0.56)f.environment.gasType[i]='paralysis';}f.rememberedTiles=[...f.tiles];add(`random-${k}`,r,undefined,k%2===0);}
for(let k=0;k<96;k++) {
 const width=5,height=5,cells=25,phases=4,blocked=Array(cells).fill(false),push=Array.from({length:cells},(_,i)=>i),wet=Array(cells).fill(false),hazard=Array(cells).fill(false);
 for(let i=0;i<cells;i++){blocked[i]=random()<0.12;wet[i]=random()<0.15;hazard[i]=random()<0.1;if(random()<0.2)push[i]=Math.floor(random()*cells);}blocked[0]=false;blocked[24]=false;
 const transition=(from,entered,phase)=>blocked[entered]?null:phase===0?{at:entered,phase:1}:{at:push[entered],phase:0};
 const mode=k%4;const expected=planDisplacedTravel(0,24,k%phases,phases,transition,width,height,mode&1?(a,b)=>wet[a]||wet[b]:undefined,mode&2?(a,b)=>hazard[a]||hazard[b]:undefined);
 graphs.push({name:`graph-${k}`,width,height,phases,start:0,target:24,initialPhase:k%phases,blocked,push,wet,hazard,mode,expected});
}
for(const old of [null,'sleeping','wandering','stalking','hunting','ally','neutral','dead'])for(const now of ['sleeping','wandering','stalking','hunting','ally','neutral','dead']) {const before=old?[[2,{state:old==='dead'?'wandering':old,dead:old==='dead'}]]:[];const after=[[2,{state:now==='dead'?'wandering':now,dead:now==='dead'}]];threats.push({before,after,expected:changedTravelThreat(new Map(before),new Map(after))});}
fs.writeFileSync(new URL('../fixtures/travel-ui-v48.json',import.meta.url),JSON.stringify({cases,graphs,threats}));console.log(`Exported ${cases.length} player routes, ${graphs.length} displaced graphs, ${threats.length} threat changes.`);
