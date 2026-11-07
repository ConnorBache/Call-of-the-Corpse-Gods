import fs from 'node:fs';
import {useAbility} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/skills-runtime.js';
const base=JSON.parse(fs.readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url)));
const cases=[],id='kative-mini-me';
for(let n=0;n<48;n++){
 const r=structuredClone(base);r.player.abilities=[id];r.player.active=[];r.player.energy=100;r.player.conditions={};r.actors[r.depth]=[];
 if(n%3===0)delete r.initialPlayer;
 else {r.initialPlayer=structuredClone(base.player);r.initialPlayer.stats.defense=n%2?-2.5:1.5;r.initialPlayer.stats.maxHp=2.5;r.initialPlayer.blood=4;r.initialPlayer.abilities=['lichor-hardened-blood'];}
 const sword=structuredClone(r.inventory.find(i=>i.kind==='weapon'&&!i.rangedWeapon));r.inventory=[];
 if(n%4!==0){sword.equipped=n%4===1?'hand2':'hand1';sword.baseBonus={offense:-0.5,defense:2.5,maxEnergy:3.5,maxHp:4.5};sword.bonus={offense:100};r.inventory.push(sword);}
 if(n%4===3){const off=structuredClone(sword);off.id=900;off.equipped='hand2';off.damage=12;off.offenseMultiplier=2;delete off.baseBonus;off.bonus={offense:1.5};r.inventory.unshift(off);}
 if(n%6===0){const bow=structuredClone(base.inventory.find(i=>i.rangedWeapon));bow.equipped='hand1';bow.hands=2;bow.baseBonus={accuracy:2.5};r.inventory.push(bow);}
 const f=r.floors[r.depth-1],w=f.width??79;
 for(let k=0;k<n%9;k++){const dx=k%3-1,dy=Math.floor(k/3)-1;if(dx||dy)f.tiles[(r.player.y+dy)*w+r.player.x+dx]='wall';}
 if(n%7===0)r.player.active=['etill-phase-through'];
 const input=structuredClone(r);useAbility(r,{type:'ability',id});
 cases.push({input,expected:r.actors[r.depth].find(a=>a.conditions.miniMe)??null,nextId:r.nextId});
}
const p=new URL('../fixtures/mini-me-v48.json',import.meta.url),t=new URL(p.href+'.tmp');fs.writeFileSync(t,JSON.stringify({cases}));fs.renameSync(t,p);console.log(cases.length);
