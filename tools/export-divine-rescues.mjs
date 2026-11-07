import fs from 'node:fs';
import {death,damage} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/combat.js';
const base=JSON.parse(fs.readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url)));
const R='shrail-rage-at-the-dying-of-the-light',O='lichor-overflowing',P='pelian-you-shall-never-be-rid-of-me';
const orders=[[R,O,P],[R,P,O],[O,R,P],[O,P,R],[P,R,O],[P,O,R]],cases=[];
for(let n=0;n<144;n++){
 const r=structuredClone(base);r.player.abilities=orders[n%6];r.player.active=[R,P];r.player.hp=-7;r.player.energy=[0,1,8][Math.floor(n/6)%3];r.player.blood=[0,1,55][Math.floor(n/18)%3];r.player.conditions={rageCost:[0,1,8][Math.floor(n/54)%3],pelianUsed:n%4===0?1:0,deathResolutionBlocked:1,beatToDeathHits:3};
 const bypass=n%11===0,initial=structuredClone(r.player);death(r,r.player,bypass);cases.push({initial,bypass,expected:r.player,rescued:r.player.hp>0&&!r.player.dead,resourceHistory:r.resourceHistory});
}
const banks=[];
for(const blood of[0,1,54,55])for(const bank of[0,0.25,49,50])for(const amount of[1,49,50,101]){
 const r=structuredClone(base);r.player.abilities=[O];r.player.blood=blood;r.player.damageBank=bank;r.player.hp=r.player.maxHp=r.player.baseMaxHp=10000;damage(r,r.player,amount,undefined,{exactScale:true});banks.push({blood,bank,amount,expectedBlood:r.player.blood,expectedBank:r.player.damageBank});
}
const p=new URL('../fixtures/divine-rescues-v48.json',import.meta.url),t=new URL(p.href+'.tmp');fs.writeFileSync(t,JSON.stringify({cases,banks}));fs.renameSync(t,p);console.log(cases.length,banks.length);
