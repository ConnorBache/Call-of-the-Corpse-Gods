// Development-only differential oracle; not linked or invoked by native runtime.
import fs from 'node:fs';
import {snapshot,recordInverse,recordFreeInverse,revert,rememberFuture,restoreFuture} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/rewind.js';
import {appendLog} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/log.js';
import {serialize} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/persistence.js';
const run=JSON.parse(fs.readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url),'utf8'));
run.player.conditions.revertCharges=5;run.player.energy=100;run.history=[];
const initial=structuredClone(run),turns=[];
for(let i=0;i<7;i++){
 const before=snapshot(run);run.tick++;run.player.x++;run.rng=(run.rng+137)>>>0;run.floors[0].seen[100+i]=true;run.actors[1][0].hp-=0.5;run.player.energy-=1;appendLog(run,`Turn ${i}`,'environment');recordInverse(run,before);
 const free=snapshot(run);run.player.conditions.extra=i;run.inventory[0].identified=true;appendLog(run,`Free ${i}`,'player');recordFreeInverse(run,free);turns.push(structuredClone(run));
}
const pending=snapshot(run);run.hasteSnapshot=JSON.stringify(pending);run.phase='normal';run.player.x+=2;run.floors[0].tiles[800]='grass';appendLog(run,'Pending haste','player');
const beforeRevert=structuredClone(run),reverted=[];while(revert(run))reverted.push(structuredClone(run));
const futureRun=structuredClone(initial);rememberFuture(futureRun);const anchor=structuredClone(futureRun);futureRun.player.x+=4;futureRun.player.energy=50;futureRun.timeCosts.energy+=9;futureRun.timeCosts.charges+=1;futureRun.player.conditions.revertCharges=4;futureRun.rng=765;appendLog(futureRun,'Erased branch','player');const branch=structuredClone(futureRun);restoreFuture(futureRun);
const vectors=[];for(const seed of ['','hello','😀','a𐀀b']){let h=2166136261;for(const c of seed)h=Math.imul(h^c.charCodeAt(0),16777619);h=h>>>0||1;let s=h,draws=[];for(let i=0;i<128;i++){s^=s<<13;s^=s>>>17;s^=s<<5;s>>>=0;draws.push(s);}vectors.push({seed,hash:h,draws});}
fs.writeFileSync(new URL('../fixtures/foundation.json',import.meta.url),JSON.stringify({initial,turns,beforeRevert,reverted,future:{initial,anchor,branch,restored:futureRun},vectors,packed:serialize(initial)}));
console.log('exported temporal and RNG fixtures');
