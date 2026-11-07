import fs from 'node:fs';
import {equipItem,unequipItem} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/equipment.js';
import {collectUnderfoot} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/inventory.js';
import {usePotion} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/potions.js';
const base=JSON.parse(fs.readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url)));const cases=[];
function add(name,setup,apply){const r=structuredClone(base);setup(r);const before=structuredClone(r);apply(r);delete r.visualEvents;cases.push({name,before,after:r});}
add('two-handed bow clears shortsword',()=>{},r=>equipItem(r,r.inventory[1],'hand2'));
add('one-handed weapon replaces two-handed',r=>equipItem(r,r.inventory[1],'hand2'),r=>equipItem(r,r.inventory[0],'hand2'));
add('unequip bow',r=>equipItem(r,r.inventory[1],'hand1'),r=>unequipItem(r,r.inventory[1]));
add('pickup arrow stack at full capacity',r=>{for(let i=0;i<17;i++)r.inventory.push({id:100+i,name:'Stone',kind:'tool',glyph:'*',enhancements:[]});r.ground[1]=[{x:r.player.x,y:r.player.y,item:{id:200,name:'Arrows',kind:'ammo',glyph:'/',effect:'arrows',quantity:5,enhancements:[]}}];},r=>collectUnderfoot(r,20));
add('pickup clears departed block',r=>{r.ground[1]=[{x:20,y:20,item:r.inventory.pop(),autoPickupBlocked:true}]},r=>collectUnderfoot(r,20));
add('dilute healing first use',r=>{r.player.hp=10;r.inventory.push({id:201,name:'Unidentified potion',kind:'potion',glyph:'!',effect:'healing',identified:false,enhancements:[]})},r=>usePotion(r,r.inventory[3]));
add('dilute healing second use',r=>{r.player.hp=10;r.inventory.push({id:201,name:'Diluted potion of healing',kind:'potion',glyph:'!',effect:'healing',identified:true,diluted:true,enhancements:[]})},r=>usePotion(r,r.inventory[3]));
add('equip capacity armor',r=>{r.inventory.push({id:300,name:'Capacity armor',kind:'armor',glyph:'[',bonus:{defense:10,maxEnergy:20},baseBonus:{defense:10,maxEnergy:20},enhancements:[]});},r=>equipItem(r,r.inventory[3],'armor'));
add('unequip capacity armor fatal',r=>{r.inventory.push({id:300,name:'Capacity armor',kind:'armor',glyph:'[',bonus:{defense:10,maxEnergy:20},baseBonus:{defense:10,maxEnergy:20},enhancements:[]});equipItem(r,r.inventory[3],'armor');r.player.hp=10;r.player.energy=120;},r=>unequipItem(r,r.inventory[3]));
atomicWrite(new URL('../fixtures/inventory-v48.json',import.meta.url),JSON.stringify(cases));

function atomicWrite(path,contents){const file=path instanceof URL?path:new URL(path,import.meta.url);const temp=new URL(file.href+".tmp");fs.writeFileSync(temp,contents);fs.renameSync(temp,file);}
