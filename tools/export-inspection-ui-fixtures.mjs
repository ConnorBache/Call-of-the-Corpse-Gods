// Frozen authored copy and read-only inspection oracle. Development only.
import fs from 'node:fs';
import {BESTIARY,EVENT_CREATURES,createBestiaryActor} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/bestiary.js';
import {creatureDescription} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/creature-descriptions.js';
import {creatureInspection} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/ui-inspection.js';
import {terrainInspectionDescription} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/ui-terrain-inspection.js';
import {gasExposure,gasExposureDescription} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/environment.js';
import {heartFor} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/engine.js';
import {makeEquipment} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/equipment.js';
import {ABILITIES} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/abilities.js';
import {floorWidth} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/dimensions.js';
const base=JSON.parse(fs.readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url),'utf8'));
const defs=[...BESTIARY,...EVENT_CREATURES];
const authored=defs.map(d=>({id:d.id,name:d.name,description:creatureDescription(d.id),ichorFamily:'ichorFamily'in d,heartOrder:Object.keys(heartFor(structuredClone(base),createBestiaryActor(d.id,200,13,14),false).bonus??{})}));
const names=['Pale Biter','Greater Pale Biter','Pale Breather','Greater Pale Breather','Pale Spitter','Greater Pale Spitter','Pale Birther','Greater Pale Birther','Pale Screamer','Greater Pale Screamer','Pale Jackal','Greater Pale Jackal','Mini-Me','Half Here','Zillennia'];
const special=names.map(name=>({name,description:creatureDescription(name),zero:creatureDescription({...base.player,name,energy:0})}));
const wendigo=Array.from({length:5},(_,i)=>creatureDescription({...createBestiaryActor('wendigo',200,13,14),conditions:{monsterWendigoLevel:i+1}}));
fs.writeFileSync(new URL('../crates/corpse-desktop/assets/inspection-copy-v48.json',import.meta.url),JSON.stringify({authored,special,wendigo}));
const terrain=[]; const at=14*floorWidth(base.floors[0])+13;
function terrainCase(name,patch,current=true){let f=structuredClone(base.floors[0]);Object.assign(f,patch);terrain.push({name,patch,current,at,expected:terrainInspectionDescription(f,at,current,base.tick)});}
const sparse=(value)=>{const a=Array(base.floors[0].tiles.length).fill('');a[at]=value;return a;};
for(const tile of ['wall','floor','grass','tree','water','shallow-water','chasm','up','down','pit'])for(const current of [true,false]){
 const tiles=[...base.floors[0].tiles];tiles[at]=tile;
 terrainCase(`tile-${tile}-${current}`,current?{tiles,substrates:undefined}:{rememberedTiles:sparse(tile),rememberedSubstrates:[]},current);
}
for(const substrate of ['tiled','rock','earth','sand','soot','ash'])for(const current of [true,false]){
 const tiles=[...base.floors[0].tiles];tiles[at]='floor';
 terrainCase(`substrate-${substrate}-${current}`,current?{tiles,substrates:sparse(substrate)}:{rememberedTiles:sparse('floor'),rememberedSubstrates:sparse(substrate)},current);
}
for(const current of [true,false])for(const fixture of ['door-open','door-closed','grate','bridge','snow','natural-wall','constructed-wall']){
 const tiles=[...base.floors[0].tiles];tiles[at]=fixture.endsWith('wall')?'wall':'floor';
 const patch=current?{tiles}:{rememberedTiles:sparse(tiles[at]),rememberedSubstrates:[]};
 if(fixture.startsWith('door'))patch[current?'doors':'rememberedDoors']={[at]:{open:fixture==='door-open'}};
 if(fixture==='grate')patch.grates=[at];if(fixture==='bridge')patch.generationBridges=[at];
 if(fixture==='snow')patch[current?'snow':'rememberedSnow']=sparse(true).map(n=>!!n);
 if(fixture.endsWith('wall'))patch.generation={version:1,stages:[],...base.floors[0].generation,naturalWalls:fixture==='natural-wall'?[at]:[]};
 terrainCase(`fixture-${fixture}-${current}`,patch,current);
}
const details={blood:true,slime:true,mud:true,roots:true,ichor:true,razorleaves:true,bulwark:4,permanentFire:true,gas:{type:'caustic',concentration:0},vent:{gas:'paralysis',phase:'waiting',countdown:7},feature:{glyph:'/',color:'#fff',name:'Mercuria’s lever',description:'Walk into the lever to open the ichor well.'},lightSource:'beacon'};
terrainCase('remembered-all-details',{rememberedTiles:sparse('floor'),rememberedSubstrates:[],rememberedHazards:{[at]:details}},false);
for(let code=1;code<=8;code++)terrainCase(`remembered-current-${code}`,{rememberedTiles:sparse('water'),rememberedHazards:{[at]:{current:code}}},false);
terrainCase('seen-no-snapshot',{rememberedTiles:[],rememberedSubstrates:[],rememberedHazards:{}},false);
terrainCase('remembered-embers',{rememberedTiles:sparse('floor'),rememberedHazards:{[at]:{fire:'embers'}}},false);
terrainCase('remembered-feature-tree',{rememberedTiles:sparse('tree'),rememberedHazards:{[at]:{feature:{glyph:'T',color:'#fff',name:'Veridian',description:'An ancient tree. Apply a tree tap while adjacent to collect Veridian Sap.'},lightSource:'torch',vent:{gas:'steam',phase:'emitting',countdown:5}}}},false);
const creatures=[];
function addCreature(name,actor,patch={},tileObserved=true){const r=structuredClone(base);r.actors[r.depth]=[actor];r.inventory=[];Object.assign(r,patch);const expected=creatureInspection(r,actor,{reachMode:'adjacent',includeConditions:true,includeHeart:true,tileObserved,illumination:21.6});creatures.push({name,actor,patch,tileObserved,expected});}
for(const d of defs){const a=createBestiaryActor(d.id,200,base.player.x+1,base.player.y);addCreature(d.id,a);addCreature(`${d.id}-unobserved-ground`,a,{},false);}
for(const name of names){const a={...structuredClone(base.player),id:200,name,heartless:true,abilities:[],active:[],state:'wandering'};delete a.creatureKind;addCreature(name,a);if(name.includes('Birther'))addCreature(`${name}-empty`,{...a,energy:0});}
for(let level=1;level<=5;level++)addCreature(`wendigo-${level}`,{...createBestiaryActor('wendigo',200,13,14),conditions:{monsterWendigoLevel:level}});
for(const state of ['sleeping','hunting','ally','neutral'])addCreature(`state-${state}`,{...createBestiaryActor('rat',200,13,14),state});
addCreature('buffs',{...createBestiaryActor('rat',200,13,14),conditions:{'buff:offense':3,'buffTicks:offense':7,'buff:dodge':-2,poisoned:8}});
addCreature('born-from-birther',{...createBestiaryActor('rat',200,13,14),bornFromBirther:true});

const abilityId=name=>ABILITIES.find(a=>a.name===name).id;
const enemy=()=>({...createBestiaryActor('rat',200,13,14),hp:120,maxHp:120});
const gear=(name,id,slot)=>({...makeEquipment(name,id),equipped:slot});
for(const loadout of ['two','offhand-only','bow'])for(const posture of ['sleeping','wandering','hunting']) {
 const inventory=loadout==='two'?[gear('Shortsword',700,'hand1'),gear('Hatchet',701,'hand2')]:loadout==='offhand-only'?[gear('Hatchet',701,'hand2')]:[gear('Bow',702,'hand1')];
 addCreature(`weapon-${loadout}-${posture}`,{...enemy(),state:posture},{inventory});
}
for(const name of ['Missed Me','Never Surrender','Hardened Blood','Waking Nightmare','Turning the Wheel','Never Without a Fight','Are You Sure About That?','Float Away','I Hit Back','Overflowing']) {
 const a=enemy();a.abilities=[abilityId(name)];a.active=[abilityId(name)];a.reactions=3;a.nightmares=2;a.hp=60;a.stats.defense=5;
 addCreature(`offhand-target-${name}`,a,{inventory:[gear('Shortsword',700,'hand1'),gear('Hatchet',701,'hand2')]});
}
for(const name of ['Unyielding Assault','The Number of Blood','Blood Weapon','Beat to Death','Shine','Silent Strike','Drown','Erase Metaphysical','Nova','Bonded By Unseen Threads']) {
 const player={...structuredClone(base.player),abilities:[abilityId(name)],active:[abilityId(name)],conditions:{bloodWeaponReady:1},energy:60};
 addCreature(`offhand-attacker-${name}`,enemy(),{tick:4,player,inventory:[gear('Shortsword',700,'hand1'),gear('Hatchet',701,'hand2')]});
}
for(const flags of [{lucky:1},{unlucky:1},{distracted:1},{lucky:1,unlucky:1},{paralyzed:3},{riptide:1}])addCreature(`player-flags-${Object.keys(flags)}`,enemy(),{player:{...structuredClone(base.player),conditions:flags},inventory:[gear('Hatchet',701,'hand2')]});
for(const hp of [0,1,15,40,120])addCreature(`offhand-hp-${hp}`,{...enemy(),hp,dead:hp===0},{inventory:[gear('Shortsword',700,'hand1'),gear('Hatchet',701,'hand2')]});
addCreature('mini-me-offhand',{...enemy(),name:'Mini-Me',heartless:true,conditions:{miniMe:1,weaponDamage:30,offhandDamage:20,offhandOffenseMultiplier:2}});
const gas=[];
for(const type of ['caustic','breather','paralysis','confusion','flammable','explosive','steam','hydrogen'])for(const concentration of [0,20])for(const immunity of [null,'enemy:beast:gas-immunity','grisha-entropy','enemy:pale:gas-immunity']) {const actor={...structuredClone(base.player),abilities:immunity?[immunity]:[],conditions:{gasVulnerable:0.5}};gas.push({actor,presence:{type,concentration},expected:gasExposureDescription(gasExposure(actor,type,concentration))});}
fs.writeFileSync(new URL('../fixtures/inspection-ui-v48.json' ,import.meta.url),JSON.stringify({terrain,creatures,gas}));
console.log(`Exported ${terrain.length} terrain and ${creatures.length} creature reports.`);
