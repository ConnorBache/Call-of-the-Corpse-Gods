// Development-only differential oracle. Native runtime never invokes JavaScript.
import { diffuseGasPass } from '../../release-v48/source/call-of-the-corpse-gods/dist/src/brogue-propagation.js';
import {writeFileSync} from 'node:fs';
const cases=[];
for(let seed=1;seed<=64;seed++){
 const width=3+(seed%5),height=3+((seed>>2)%4),n=width*height;let rng=seed;
 const randomInt=(min,max)=>{if(min===max)return min;rng^=rng<<13;rng^=rng>>>17;rng^=rng<<5;rng>>>=0;return min+Math.floor(rng/4294967296*(max-min+1));};
 const volume=Array.from({length:n},()=>randomInt(0,65535));const gas=volume.map(()=>[null,'caustic','confusion','explosive'][randomInt(0,3)]);const blocked=volume.map(()=>randomInt(0,7)===0),frozen=volume.map(()=>randomInt(0,12)===0),descent=volume.map(()=>randomInt(0,8)===0);
 const initial={width,height,volume:[...volume],gas:[...gas],blocked,frozen,descent,rng};const events=[];
 diffuseGasPass({width,height,volume,gas,blocksGas:i=>blocked[i],frozen:i=>frozen[i],autoDescent:i=>descent[i],dissipation:g=>g===null||g==='explosive'?'none':g==='confusion'?'quick':'normal',randomInt,onTypeChange:(i,previous,next,source)=>events.push({i,previous,next,source}),onCellResolved:(i,result)=>events.push({i,...result})});
 cases.push({initial,expected:{volume,gas,rng,events}});
}
writeFileSync(new URL('../crates/corpse-core/tests/fixtures/propagation-v48.json',import.meta.url),JSON.stringify(cases));
