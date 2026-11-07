// Development-only immutable catalog extraction. Never used by native play.
import fs from 'node:fs';
const root=new URL('../../release-v48/source/call-of-the-corpse-gods/dist/src/',import.meta.url);
const mod=n=>import(new URL(n+'.js',root));
const {BESTIARY,EVENT_CREATURES,createBestiaryActor,createZillennia}=await mod('bestiary');
const {paleActor}=await mod('population-actors');
const {makeFungus}=await mod('mercuria-creatures');
const {budgetedHordeCandidates,creatureHeartBlessings}=await mod('horde-budget');
const {fungalCorpseKindAtDepth,PAIRED_CREATURES,ordinaryRosterKinds,firstRosterDepth}=await mod('monster-catalog');
const {requiresAquaticSpawn}=await mod('population-habitat');
const {CREATURE_DANGER}=await mod('encounter-danger');
const {POTION_TYPES}=await mod('potions');
const actors={},hearts={},awake={},aquatic={};
for(const d of [...BESTIARY,...EVENT_CREATURES]) {actors[d.id]=createBestiaryActor(d.id,0,0,0);awake[d.id]=!!d.alwaysAwake;aquatic[d.id]=requiresAquaticSpawn(d);try{hearts[d.id]=creatureHeartBlessings(d.id)}catch{}}
for(const p of ['','Greater '])for(const role of ['Biter','Breather','Spitter','Birther','Screamer','Jackal']){const k='pale:'+p+role;actors[k]=paleActor(p+role,0,0,0,false);hearts[k]=creatureHeartBlessings(k)}
for(const k of ['fungus','vent-fungus','bastidion'])hearts[k]=1;
const singles={};for(let d=1;d<=40;d++){const eligible=ordinaryRosterKinds(d);singles[d]=BESTIARY.filter(x=>eligible.includes(x.id)&&!PAIRED_CREATURES[x.id]).sort((a,b)=>eligible.indexOf(a.id)-eligible.indexOf(b.id)).map(x=>({id:x.id,family:x.family,weight:Math.min(1,0.2+(d-x.depth)*0.2)}));for(const id of eligible.filter(k=>k.startsWith("pale:")))singles[d].push({id,family:"pale",weight:id.startsWith("pale:Greater ")?Math.min(0.35,0.07+(d-(firstRosterDepth(id)??1))*0.07):1});}const candidates={},fungi=[],zillennia=[],corpses=[];
for(let d=1;d<=40;d++){for(const wet of [false,true])candidates[`${d}:${wet}`]=budgetedHordeCandidates(d,wet);fungi.push(makeFungus(0,0,0,0,d));zillennia.push(createZillennia(0,0,0,d));corpses.push(fungalCorpseKindAtDepth(d));}
fs.writeFileSync(new URL('../crates/corpse-core/src/catalog/population-v48.json',import.meta.url),JSON.stringify({singles,actors,hearts,awake,aquatic,candidates,fungi,zillennia,corpses,pairs:PAIRED_CREATURES,danger:CREATURE_DANGER,potions:POTION_TYPES}));
