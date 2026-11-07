// Development-only frozen v48 oracle. The native executable never runs JS.
import fs from 'node:fs';
import { inventoryDisplayGroups } from '../../release-v48/source/call-of-the-corpse-gods/dist/src/ui-inventory-order.js';
import { displayedItemName, describeItem } from '../../release-v48/source/call-of-the-corpse-gods/dist/src/ui-items.js';
import { makeEquipment, EQUIPMENT } from '../../release-v48/source/call-of-the-corpse-gods/dist/src/equipment.js';
import { inventoryOccupancy } from '../../release-v48/source/call-of-the-corpse-gods/dist/src/inventory.js';
import { carryingCapacity } from '../../release-v48/source/call-of-the-corpse-gods/dist/src/crafting.js';
const base=JSON.parse(fs.readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url),'utf8'));
const cases=[];
function add(name,inventory,changes={}) {
  const run={...structuredClone(base),...changes,inventory};
  const groups=inventoryDisplayGroups(run).map(g=>({ids:g.items.map(i=>i.id),count:g.count,name:displayedItemName(run,g.item),description:describeItem(run,g.item)}));
  cases.push({name,inventory,changes,groups,slots:inventoryOccupancy(run),capacity:carryingCapacity(run)});
}
add('empty',[]);
let id=200;
const equipment=Object.keys(EQUIPMENT).map(name=>makeEquipment(name,id++));
add('equipment-catalog',equipment);
add('acquisition-and-visual-stacks',[equipment[0],{...equipment[0],id:id++},equipment[6],{...equipment[6],id:id++},equipment[10]]);
add('saved-order-with-physical-members',[equipment[0],{...equipment[0],id:999},equipment[6],equipment[10]],{inventoryDisplayOrder:[equipment[10].id,999,equipment[6].id,equipment[0].id]});
add('equipped-leads',[equipment[4],{...equipment[0],equipped:'hand1'},{...equipment[2],equipped:'hand2'},{...equipment[6],equipped:'armor'},equipment[10]],{inventoryDisplayOrder:[equipment[10].id,equipment[4].id,equipment[6].id,equipment[2].id,equipment[0].id]});
add('equipped-bow',[{...equipment[4],equipped:'hand1'},{id:900,name:'Arrows',kind:'ammo',glyph:'→',effect:'arrows',quantity:100,enhancements:[]}]);
const hearts=[{id:300,name:'Rat heart',kind:'heart',glyph:';',bonus:{offense:1},exp:2,energy:3,enhancements:[]},{id:301,name:'Rat heart',kind:'heart',glyph:';',bonus:{offense:1},exp:2,energy:3,quantity:2,enhancements:[]}];
add('heart-and-arrow-slot-accounting',[...hearts,{id:900,name:'Arrows',kind:'ammo',glyph:'→',effect:'arrows',quantity:1000,enhancements:[]}]);
add('distinct-enhancements-and-magic',[equipment[0],{...equipment[0],id:501,magicNature:'good'},{...equipment[0],id:502,bonus:{offense:2}},{...equipment[0],id:503,grantedAbility:'kative-dilute-potion',tinkered:true},{...equipment[0],id:504,enhancements:[{craft:'isheilah-guile-craft',bonus:{offense:1}}]}]);
for(const effect of ['healing','energy','incineration','caustic gas','paralysis','telepathy','levitation','speed','confusion','invisibility','descent','dr','perception','Veridian Sap']) {
 for(const known of [false,true]) for(const diluted of [false,true]) {
   const potion={id:800,name:'Unrevealed potion',kind:'potion',glyph:'!',effect,enhancements:[],...(diluted?{diluted:true}:{})};
   add(`potion-${effect}-${known}-${diluted}`,[potion,{...potion,id:801,quantity:2}],{knownPotions:known?[effect]:[]});
 }
}
fs.writeFileSync(new URL('../fixtures/pack-ui-v48.json',import.meta.url),JSON.stringify({cases}));
console.log(`Exported ${cases.length} Pack cases.`);
