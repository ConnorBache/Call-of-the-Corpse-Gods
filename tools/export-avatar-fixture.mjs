import { writeFileSync as writeRaw, renameSync, mkdirSync } from 'node:fs';
const writeFileSync=(path,data)=>{const temp=new URL(path.href+'.tmp');writeRaw(temp,data);renameSync(temp,path);};
import { createRun, dispatch } from '../../release-v48/source/call-of-the-corpse-gods/dist/src/engine.js';
import { createBestiaryActor } from '../../release-v48/source/call-of-the-corpse-gods/dist/src/bestiary.js';
import { rememberVisibleTerrain } from '../../release-v48/source/call-of-the-corpse-gods/dist/src/perception.js';
import { resetPresentationEpoch } from '../../release-v48/source/call-of-the-corpse-gods/dist/src/presentation-context.js';
const out = new URL('../fixtures/', import.meta.url); mkdirSync(out,{recursive:true});
const run = createRun('00-09-10:native-parity-v1', 'Avatar');
// Explicit mechanics fixture: actual starting Avatar/items/abilities, but authored
// static rooms isolate the first native slice from unported world generation.
run.floors = Array.from({length:10},(_,i)=>{
 const tiles=Array(79*29).fill('wall');
 for(let y=6;y<=22;y++) for(let x=5;x<=35;x++) tiles[y*79+x]='floor';
 tiles[8*79+7]='up'; tiles[20*79+33]='down';
 return {depth:i+1,domain:'Dungeon',godbeast:false,dirty:false,lastTick:0,width:79,height:29,tiles,lights:Array(79*29).fill(0),seen:Array(79*29).fill(false)};
});
run.actors=Object.fromEntries(run.floors.map(f=>[f.depth,[]]));
run.ground=Object.fromEntries(run.floors.map(f=>[f.depth,[]]));
run.player.x=12;run.player.y=14;run.initialPlayer=structuredClone(run.player);
run.history=[];run.replay=[];run.tick=0;run.nextId=5;
const goblin=createBestiaryActor('hemogoblin',4,16,14,false);
goblin.state='hunting';goblin.lastSeen={x:12,y:14};run.actors[1]=[goblin];
resetPresentationEpoch(run);rememberVisibleTerrain(run);
writeFileSync(new URL('avatar-room-v1.json',out),JSON.stringify(run));
const initial=structuredClone(run);
const commands=[{type:'wait'},{type:'move',dx:1,dy:0},{type:'move',dx:1,dy:0},{type:'move',dx:1,dy:0},{type:'move',dx:1,dy:0},{type:'wait'},{type:'move',dx:0,dy:1},{type:'move',dx:1,dy:0}];
const steps=[];
for(const command of commands){dispatch(run,command,true);steps.push({command,state:structuredClone(run)});}
// Extend the same source run through a kill, corpse/heart pickup and eating.
function step(command){dispatch(run,command,true);steps.push({command,state:structuredClone(run)});}
for(let i=0;i<12 && !run.actors[1][0].dead && run.status==='playing';i++){
 const enemy=run.actors[1][0];step({type:'move',dx:Math.sign(enemy.x-run.player.x),dy:Math.sign(enemy.y-run.player.y)});
}
if(run.actors[1][0].dead){
 const corpse=run.actors[1][0];
 if(run.player.x!==corpse.x || run.player.y!==corpse.y) step({type:'move',dx:Math.sign(corpse.x-run.player.x),dy:Math.sign(corpse.y-run.player.y)});
 const heart=run.inventory.find(i=>i.kind==='heart');if(heart)step({type:'use',id:heart.id});
}
const injured=structuredClone(initial);injured.player.hp=1;injured.actors[1][0].x=13;
const deathInitial=structuredClone(injured),deathSteps=[];
for(let i=0;i<12 && injured.status==='playing';i++){const command={type:'wait'};dispatch(injured,command,true);deathSteps.push({command,state:structuredClone(injured)});}
writeFileSync(new URL('avatar-death-v1-trace.json',out),JSON.stringify({scope:'Controlled injured Avatar terminal-death mechanics fixture.',initial:deathInitial,steps:deathSteps}));
writeFileSync(new URL('avatar-room-v1-trace.json',out),JSON.stringify({scope:'Controlled static-room Avatar mechanics fixture. Not native procedural generation or a complete gameplay acceptance test.',initial,steps}));
console.log(JSON.stringify({abilities:initial.player.abilities,inventory:initial.inventory,enemy:initial.actors[1][0],steps:steps.map(s=>({command:s.command,tick:s.state.tick,rng:s.state.rng,hp:s.state.player.hp,enemyHp:s.state.actors[1][0].hp,enemyPosition:[s.state.actors[1][0].x,s.state.actors[1][0].y]}))},null,2));
