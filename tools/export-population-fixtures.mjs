import fs from 'node:fs';
const root=new URL('../../release-v48/source/call-of-the-corpse-gods/dist/src/',import.meta.url);const mod=n=>import(new URL(n+'.js',root));
const {planBudgetedHordes,ordinaryBlessingBudget,ordinaryHordeBudgetFeasible}=await mod('horde-budget');
const {createPopulationActor}=await mod('population-actors');const {populationRandom}=await mod('encounters');
const {BESTIARY}=await mod('bestiary');
const plans=[];for(const seed of ['population-native','0-1-2:fixture','🌙'])for(let depth=1;depth<=40;depth++)for(const water of [false,true]){const total=ordinaryBlessingBudget(seed,depth);plans.push({seed,depth,total,water,expected:planBudgetedHordes(seed,depth,total,0,water)})}
const actors=[];for(const kind of [...BESTIARY.map(d=>d.id),'fungus','vent-fungus','pale:Biter','pale:Greater Screamer'])for(const sleeping of [false,true]){const f={depth:17,tiles:Array(2500).fill('floor')},r={nextId:10};let n=0;const rng=populationRandom('actor-native');const a=createPopulationActor(r,f,kind,1275,sleeping,()=>{n++;return rng()});actors.push({kind,sleeping,actor:a,nextId:r.nextId,slime:f.slime,draws:n});}
fs.writeFileSync(new URL('../fixtures/population-v48.json',import.meta.url),JSON.stringify({plans,actors}));
