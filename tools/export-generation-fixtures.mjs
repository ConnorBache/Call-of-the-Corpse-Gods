// Development-only frozen-reference oracle. Never invoked by shipped Rust gameplay.
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
import { writeFileSync, readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
const root=resolve(process.argv[2]??'../release-v48/source/call-of-the-corpse-gods');
const load=name=>import(pathToFileURL(`${root}/dist/src/${name}.js`));
const {makeRoomShape}=await load('room-shapes');const {makeRoomPassage}=await load('room-passages');const {floorStairs}=await load('stair-plan');const {domainPlan}=await load('world');
function rng(seed){let state=seed>>>0;const random=()=>{state=(state+0x6d2b79f5)>>>0;let t=Math.imul(state^(state>>>15),1|state);t^=t+Math.imul(t^(t>>>7),61|t);return ((t^(t>>>14))>>>0)/4294967296};return {random,state:()=>state}}
const seeds=[0,1,2,42,12345,0xffffffff,0x80000000];const rooms=[];for(const seed of seeds)for(const kind of ['rectangle','cavern','lobed','alcove'])for(const sizeClass of ['small','medium','large']){const r=rng(seed);rooms.push({seed,kind,sizeClass,result:makeRoomShape(kind,sizeClass,r.random),rngState:r.state()})}
const stairs=[];for(const seed of seeds)for(const size of [79,50,60])for(const depth of [1,4,5,6,20,40])for(const actualUp of [null,{x:100,y:-10},{x:18,y:15}])stairs.push({seed,size,depth,actualUp,result:floorStairs(depth,seed,size,actualUp??undefined)});
const passages=[];for(const style of ['constructed','natural','broad','opening'])for(const direction of [{x:1,y:0},{x:-1,y:0},{x:0,y:1},{x:0,y:-1}])for(const offset of [-3,0,4])for(const bend of [-2,0,2])for(const start of [false,true])for(const end of [false,true]){const input={width:79,from:{x:30,y:30},direction,gap:8,offset,style,start,end,bend};passages.push({input,result:makeRoomPassage(input.width,input.from,direction,8,offset,style,start,end,bend)})}
const geology=readFileSync(`${root}/dist/src/geology.js`,'utf8');
const fieldSource=geology.slice(geology.indexOf('function valueField('),geology.indexOf('/** Landforms exist'));
const valueField=new Function('const smooth = n => n*n*(3-2*n);'+fieldSource+';return valueField')();
const fields=[];for(const seed of seeds)for(const [width,height] of [[79,29],[50,50],[60,60]])for(const wavelength of [5,13]){const r=rng(seed);const values=valueField(width,height,wavelength,r.random);fields.push({seed,width,height,wavelength,bits:[...new Uint32Array(values.buffer)],rngState:r.state()})}
const {ordinaryBlessingBudget}=await load('horde-budget');const {baseEncounterBudget}=await load('encounters');const {retainedLootAllocation}=await load('loot-budget');
const budgets=[];for(const seed of [...seeds,'native-port','🐉',''])for(let depth=1;depth<=40;depth++)budgets.push({seed,depth,result:ordinaryBlessingBudget(seed,depth)});
const encounters=[];const loot=[];for(let depth=1;depth<=40;depth++){for(const cells of [2291,2500,3600])encounters.push({depth,cells,result:baseEncounterBudget(depth,cells)});for(const hearts of [0,2,9,48])for(const carried of [0,1,8])loot.push({depth,hearts,carried,result:retainedLootAllocation(depth,[],hearts,carried)})}
const recordedLoot=[];
for(const depth of [2,3,5,6,39,40])for(const malformed of [false,true]){const priorDepth=depth-1;const details=retainedLootAllocation(priorDepth,[],9,2);if(malformed)details.retainedBlockTotal=-1;const floors=[{depth:priorDepth,generation:{stages:[{id:'items',details}]}}];recordedLoot.push({depth,floors,result:retainedLootAllocation(depth,floors,1,1)})}
const {generateDungeonLayout}=await load('dungeon-layout');const layouts=[];
for(const seed of seeds)for(const size of [79,50,60])for(const spreadAcrossMap of [false,true]){const stairs=floorStairs(1,seed,size);const constraints={spreadAcrossMap};const r=rng(seed);const stages=[];let result,error;try{result=generateDungeonLayout(size,stairs.up,stairs.down,r.random,(id,layout)=>stages.push({id,layout:structuredClone(layout)}),constraints)}catch(e){error=e.message}layouts.push({seed,size,entrance:stairs.up,exit:stairs.down,constraints,result,error,stages,rngState:r.state()})}
const {addRoomLoops}=await load('room-loops');const loops=[];
for(let index=0;index<layouts.length;index++){if(!layouts[index].result)continue;for(const mode of ['straight','bent','gated']){const layout=structuredClone(layouts[index].result),r=rng(layouts[index].seed^0x713974);const options={allowBent:mode!=='straight',...(mode==='gated'?{maximumCandidateChecks:5,acceptCandidate:(_l,c)=>c.routeBefore>=40}:{})};const added=addRoomLoops(layout,r.random,options);loops.push({index,mode,added,result:layout,rngState:r.state()})}}
const output={loops,layouts,recordedLoot,budgets,encounters,loot,fields,reference:'frozen-v48',rooms,stairs,passages,domains:seeds.map(seed=>({seed,result:domainPlan(seed)}))};const text=JSON.stringify(output);const out=resolve('crates/corpse-core/tests/fixtures/generation-v48.json');writeFileSync(out,text+'\n');console.log(`${out}: sha256 ${createHash('sha256').update(text+'\n').digest('hex')}`);
