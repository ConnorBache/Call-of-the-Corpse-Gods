import {readFileSync,writeFileSync} from 'node:fs';
import {createHalfHere} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/half-here.js';
import {tickEnvironment,ignite,addGas} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/environment.js';
import {rememberObservedHazards} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/hazard-memory.js';
import {generateCurrents} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/currents.js';
import {placeSparkle} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/world-abilities.js';
const base=JSON.parse(readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url),'utf8'));
base.floors=base.floors.slice(0,1);base.actors={'1':base.actors['1']};base.ground={'1':[]};
const cases=[];
for(const name of ['burning','sparkle','caustic','explosive','hydrogen','vent','water','fungi','frozen','entropy','star-power','multi-target','ichor','razorleaves','vent-hydrogen','vent-steam','vent-confusion','restore-chasm','breather','half-fire','half-gas','stationary-chasm-item']){
 const initial=structuredClone(base);initial.player.hp=initial.player.maxHp=1000;initial.actors['1']=[];initial.rng=1973;
 const f=initial.floors[0],w=f.width??79,x=initial.player.x,y=initial.player.y;
 if(name.startsWith('half-')){initial.player.abilities.push('etill-half-here');const clone=createHalfHere(initial,x+1,y);if(name==='half-fire'){ignite(initial,x,y,undefined,clone);ignite(initial,x+1,y,undefined,clone);}else{addGas(initial,x,y,'explosive',1000,clone);addGas(initial,x+1,y,'explosive',1000,clone);ignite(initial,x,y,undefined,clone);}}
 if(name==='stationary-chasm-item'){const i=(y+2)*w+x+3;f.tiles[i]='chasm';initial.ground['1']=[{x:x+3,y:y+2,item:{id:999,kind:'gold',name:'Gold',glyph:'$',enhancements:[],quantity:1}}];}
 if(name==='razorleaves'){const a=structuredClone(base.actors['1'][0]);Object.assign(a,{id:100,x:x+3,y,abilities:['enemy:beast:razorleaves','enemy:beast:rooted'],conditions:{bestiaryTraitsInitialized:1}});initial.actors['1']=[a];}
 if(name==='restore-chasm'){const i=(y+2)*w+x+3;f.tiles[i]='chasm';f.alteredChasms={[i]:{tile:'floor',age:0}};}
 if(name==='breather'){const a=structuredClone(base.actors['1'][0]);Object.assign(a,{id:100,x:x+1,y,abilities:['enemy:pale:breath','enemy:pale:breath-boost','enemy:pale:gas-vulnerability','enemy:pale:gas-immunity'],conditions:{paleTraitsInitialized:1}});initial.actors['1']=[a];initial.player.conditions.gasVulnerable=5;initial.player.conditions.gasVulnerableTicks=5;}
 if(name==='ichor')f.godbeastEvent={kind:'Mercuria',x:x+3,y,room:{left:1,top:1,right:w-2,bottom:20},activated:true,floodAge:6,fishSpawned:0,nextFishAge:8,well:{x:x+3,y},lever:{x:x+4,y}};
 if(name==='multi-target'){for(let n=0;n<3;n++){const a=structuredClone(base.actors['1'][0]);Object.assign(a,{id:100+n,x:x+n,y:y+1,hp:1,state:'wandering'});initial.actors['1'].push(a);}for(let dy=-2;dy<=2;dy++)for(let dx=-2;dx<=2;dx++)addGas(initial,x+dx,y+dy,'explosive',1000);ignite(initial,x,y,undefined,initial.player);}
 if(name==='frozen'){initial.player.abilities.push('dawel-reverse-entropy');initial.player.active.push('dawel-reverse-entropy');addGas(initial,x,y,'caustic',1000);ignite(initial,x+1,y);}
 if(name==='entropy')initial.player.abilities.push('grisha-entropy');
 if(name==='star-power')initial.player.abilities.push('anavani-star-power');
 if(name==='water'){for(let dy=-1;dy<=1;dy++)for(let dx=-1;dx<=1;dx++)f.tiles[(y+dy)*w+x+dx]='water';initial.player.conditions.burning=7;}
 if(name==='fungi'){const fungus=structuredClone(base.actors['1'][0]);Object.assign(fungus,{id:100,name:'Bastidion',x:x+3,y,abilities:['enemy:beast:fungal-gas','enemy:beast:gas-immunity','enemy:beast:no-revival','enemy:beast:rooted'],conditions:{bestiaryTraitsInitialized:1,fungalGasType:0}});const corpse=structuredClone(fungus);Object.assign(corpse,{id:101,name:'Corpse',x:x+5,dead:true,abilities:[],conditions:{burning:2,burningOwner:0}});initial.actors['1']=[fungus,corpse];}
 if(name.startsWith('vent-'))f.vents=[{at:y*w+x,gas:name.slice(5),countdown:1,phase:'waiting'}];
 if(name==='vent')f.vents=[{at:y*w+x,gas:'caustic',countdown:1,phase:'waiting'}];
 if(name==='burning')ignite(initial,x,y,undefined,initial.player);
 if(name==='sparkle')placeSparkle(initial,x,y);
 if(name==='caustic')addGas(initial,x,y,'caustic',1000,initial.player);
 if(name==='explosive'||name==='hydrogen'){
   for(let dy=-2;dy<=2;dy++)for(let dx=-2;dx<=2;dx++)addGas(initial,x+dx,y+dy,name,1000);
   ignite(initial,x,y,undefined,initial.player);
 }
 const expected=structuredClone(initial);for(let i=0;i<3;i++)tickEnvironment(expected);
 const observed=structuredClone(expected.floors[0]);const mask=observed.tiles.map((_,i)=>i%3!==1);rememberObservedHazards(observed,mask,123);
 const currentFloor=structuredClone(initial.floors[0]);generateCurrents(currentFloor);
 cases.push({name,initial,expected,mask,hazardExpected:observed.rememberedHazards,currentsExpected:currentFloor.currents});
}
writeFileSync(new URL('../crates/corpse-core/tests/fixtures/environment-active-v48.json',import.meta.url),JSON.stringify(cases));
