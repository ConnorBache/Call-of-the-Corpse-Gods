import {writeFileSync} from 'node:fs';
import {makeIchorFish} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/mercuria-creatures.js';
const cases=[];
for(let depth=1;depth<=40;depth++)for(const random of [0,0.49,0.999999])cases.push({depth,random,expected:makeIchorFish(777,3,5,depth,()=>random)});
writeFileSync(new URL('../crates/corpse-core/tests/fixtures/ichor-v48.json',import.meta.url),JSON.stringify(cases));
