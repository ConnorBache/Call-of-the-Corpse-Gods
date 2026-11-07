import { exposeFire,promoteTimedLayers,spreadExistingFire } from '../../release-v48/source/call-of-the-corpse-gods/dist/src/brogue-propagation.js';
import {writeFileSync} from 'node:fs';
const out=[];
for(let seed=1;seed<=64;seed++){
 let rng=seed;const randomInt=(a,b)=>{rng^=rng<<13;rng^=rng>>>17;rng^=rng<<5;rng>>>=0;return a+Math.floor(rng/4294967296*(b-a+1));};
 const width=5,height=4,n=width*height;
 const layers=Array.from({length:n},()=>Array.from({length:3},(_,j)=>({id:String(j),flammable:randomInt(0,2)>0,ignitionChance:randomInt(0,100),drawPriority:randomInt(1,100),extinguishes:randomInt(0,6)===0,isGas:j===1,explosive:j===1&&randomInt(0,1)===1})));
 const exposure=Array.from({length:n},()=>randomInt(0,13)),qualifies=Array.from({length:n},()=>randomInt(0,4)>0);
 const initial={width,height,layers:structuredClone(layers),exposure:[...exposure],qualifies,rng};const events=[],results=[];
 const grid={width,height,layers:i=>layers[i],exposure,randomInt,qualifiesForConfinement:i=>qualifies[i],promote:(i,layer,explosive)=>{events.push({i,id:layer.id,explosive});layers[i][Number(layer.id)].flammable=false;if(layer.id==='0')layers[i][2].flammable=false;}};
 for(let x=0;x<width;x++)for(let y=0;y<height;y++){const i=y*width+x;results.push(exposeFire(grid,i,i%7===0));}
 out.push({initial,expected:{layers,exposure,rng,events,results}});
}
writeFileSync(new URL('../crates/corpse-core/tests/fixtures/fire-v48.json',import.meta.url),JSON.stringify(out));
