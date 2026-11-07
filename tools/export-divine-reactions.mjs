import fs from 'node:fs';
import {nightmareAbsorbsDamage,strikeDamage,effectiveStats} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/combat.js';
const base=JSON.parse(fs.readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url))),cases=[];
for(let n=0;n<96;n++){
 const r=structuredClone(base),a=n%2?r.player:r.actors[1][0],b=n%2?r.actors[1][0]:r.player;
 a.state=n%3===0?'ally':'hunting';b.state=n%4===0?'ally':'hunting';a.active=n%5===0?[]:['ursa-waking-nightmare'];a.nightmares=n%7===0?0:2;a.abilities=['ursa-waking-nightmare'];
 cases.push({player:r.player,actors:r.actors,victim:a.id,source:n%6===0?null:b.id,expected:nightmareAbsorbsDamage(r,a,n%6===0?undefined:b)});
}
const surrender=[];
for(let n=0;n<72;n++){
 const r=structuredClone(base);r.player.active=n%5===0?[]:['pelian-never-surrender'];r.player.abilities=['pelian-never-surrender'];r.player.energy=[0,7,8,20][n%4];r.player.stats.defense=[-2,0,2,6][Math.floor(n/4)%4];r.player.conditions={};r.tick=n%3;if(n%2===0)r.player.conditions.surrenderTick=r.tick;r.player.stats.dr=2;
 const foe=r.actors[1][0];foe.state='hunting';r.player.state='hunting';
 surrender.push({player:r.player,actors:r.actors,tick:r.tick,withBonus:strikeDamage(r,foe,r.player,100,1,0,true),withoutBonus:strikeDamage(r,foe,r.player,100,1,0,false),stats:effectiveStats(r,r.player)});
}
const p=new URL('../fixtures/divine-reactions-v48.json',import.meta.url),t=new URL(p.href+'.tmp');fs.writeFileSync(t,JSON.stringify({cases,surrender}));fs.renameSync(t,p);console.log(cases.length,surrender.length);
