import fs from 'node:fs';
import {effectiveStats,maximumHealth,bloodWeaponHpCost,bloodWeaponReady,healingSuppressed} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/combat.js';
const base=JSON.parse(fs.readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url)));
const cases=[];
for(let n=0;n<192;n++){
 const r=structuredClone(base),kind=['player','enemy','mini','clone'][n%4];
 r.player.abilities=['lichor-blood-weapon'];r.player.active=[];r.player.conditions={bloodWeaponReady:n%3===0?1:0};r.player.hp=[0,1,49,90,100,106][Math.floor(n/4)%6];
 let a=kind==='player'?r.player:structuredClone(r.actors[1][0]);a.conditions={};a.abilities=['lichor-hardened-blood','lichor-blood-weapon'];a.stats.defense=[-16,-2,0,4][Math.floor(n/24)%4];a.stats.maxHp=[0,3,6,-3][n%4];a.stats.dr=2;a.stats.dodge=-1;
 a.hp=[0,1,49,90,100,106][Math.floor(n/4)%6];a.maxHp=100+6*a.stats.defense;a.conditions.beatToDeathHits=[-1,0,1,1.5][Math.floor(n/48)%4];a.conditions.bloodWeaponReady=n%3===0?1:0;a.x=r.player.x+(n%2?11:3);a.y=r.player.y;a.state=n%5===0?'ally':'hunting';
 if(kind==='mini'){a.conditions.miniMe=1;a.baseMaxHp=100;}
 if(kind==='clone'){a.conditions.halfHereClone=1;a.conditions.halfHereOwner=r.player.id;a.state='ally';r.player.abilities.push('etill-half-here');r.player.active.push('etill-half-here');r.player.conditions.halfHereId=a.id;}
 if(n%7===0)r.player.active.push('dawel-reverse-entropy');
 if(kind!=='player')r.actors[1]=[a];
 const expectedStats=effectiveStats(r,a),health=maximumHealth(r,a),cost=bloodWeaponHpCost(a),ready=bloodWeaponReady(a,r),suppressed=healingSuppressed(r,a);
 const abilities=a.abilities,hits=a.conditions.beatToDeathHits;a.abilities=abilities.filter(id=>id!=='lichor-hardened-blood');delete a.conditions.beatToDeathHits;const inputStats=effectiveStats(r,a);a.abilities=abilities;a.conditions.beatToDeathHits=hits;
 cases.push({kind,player:r.player,actor:a,inventory:r.inventory,inputStats,expectedStats,health,cost,ready,suppressed});
}
const p=new URL('../fixtures/divine-combat-v48.json',import.meta.url),t=new URL(p.href+'.tmp');fs.writeFileSync(t,JSON.stringify(cases));fs.renameSync(t,p);console.log(cases.length);
