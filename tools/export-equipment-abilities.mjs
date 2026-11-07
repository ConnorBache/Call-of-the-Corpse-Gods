import fs from 'node:fs';
import {equipmentAbilityState,syncEquipmentAbilities,learnedAbilities} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/equipment-abilities.js';
const base=JSON.parse(fs.readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url))),cases=[];
const ids=['ursa-waking-nightmare','morae-fortune-teller','isheilah-missed-me','isheilah-quick-hand','etill-half-here','kative-mini-me','lichor-blood-weapon','etill-phase-through'];
for(let n=0;n<192;n++){
 const r=structuredClone(base);r.tick=50;r.player.abilities=[ids[n%8],ids[(n+1)%8],'shrail-unyielding-assault'];r.player.equipmentAbilities=[ids[n%8]];r.player.abilityOrder=[ids[(n+2)%8],...r.player.abilities];r.player.active=[ids[n%8]];r.player.reactions=n%4;r.player.conditions={nightmareAt:100,miniMeId:4,bloodWeaponReady:1};
 if(n%3===0)r.player.conditions[`remembered:${ids[n%8]}`]=60;
 if(n%5===0)r.player.conditions[`unequipped:${ids[(n+1)%8]}`]=10;
 if(n%7===0)r.player.conditions[`acquired:${ids[(n+1)%8]}`]=0;
 r.inventory=[];for(let j=0;j<3;j++){const item=structuredClone(base.inventory[0]);item.id=10+j;item.kind=j===2&&n%4===0?'potion':'ring';item.name='Fixture ring';item.equipped=n%6===0?undefined:j?'ring2':'ring1';item.grantedAbility=j===2&&n%11===0?'unknown-id':ids[(n+j+1)%8];r.inventory.push(item);}
 if(n%9===0){const bow=structuredClone(base.inventory[1]);bow.equipped=n%18===0?'hand2':'hand1';bow.grantedAbility=ids[(n+4)%8];r.inventory.push(bow);}
 const initial=structuredClone(r),projection=equipmentAbilityState(r,r.inventory),learned=learnedAbilities(r);syncEquipmentAbilities(r,undefined,{settleEnvironment:false});
 cases.push({initial:{player:initial.player,inventory:initial.inventory,actors:initial.actors,tick:initial.tick},projection,learned,expected:{player:r.player,actors:r.actors}});
}
const p=new URL('../fixtures/equipment-abilities-v48.json',import.meta.url),t=new URL(p.href+'.tmp');fs.writeFileSync(t,JSON.stringify(cases));fs.renameSync(t,p);console.log(cases.length);
