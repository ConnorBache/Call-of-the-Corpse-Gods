// Frozen observed-only ray + Bow tooltip oracle. Development only.
import fs from 'node:fs';
import {previewProjectileTrace} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/ui-targeting.js';
import {bowAttackPreview} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/bows.js';
import {huntingCombatView} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/combat-preview-view.js';
import {makeEquipment} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/equipment.js';
import {makeArrows} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/ammunition.js';
import {createBestiaryActor} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/bestiary.js';
import {ABILITIES} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/abilities.js';
const base=JSON.parse(fs.readFileSync(new URL('../fixtures/travel-ui-v48.json',import.meta.url),'utf8')).cases[0].run;
const id=name=>ABILITIES.find(a=>a.name===name).id;
function make(){const r=structuredClone(base);r.player.x=1;r.player.y=1;r.player.conditions={};r.player.abilities=[];r.player.active=[];r.inventory=[];r.floors[0].tiles.fill('floor');r.floors[0].rememberedTiles.fill('floor');return r}
const rays=[],bows=[];
for(const mode of ['clear','unknown','remembered','wall','tree','door','open-door','grate','creature','hidden-creature','corpse','zero-hp','target-creature'])for(let y=0;y<7;y++)for(const x of [0,3,6]){
 const r=make(),f=r.floors[0],target={x,y},mask=Array(49).fill(true),observed=[];
 const at=16;if(mode==='unknown'||mode==='remembered')mask[at]=false;
 if(mode==='wall'||mode==='tree')f.tiles[at]=mode;
 if(mode==='door'||mode==='open-door')f.doors[at]={open:mode==='open-door'};
 if(mode==='grate')f.grates=[at];
 if(['creature','hidden-creature','corpse','zero-hp','target-creature'].includes(mode)){const a=createBestiaryActor('rat',200,mode==='target-creature'?x:2,mode==='target-creature'?y:2);if(mode==='corpse')a.dead=true;if(mode==='zero-hp')a.hp=0;r.actors[r.depth]=[a];if(mode!=='hidden-creature')observed.push(200);}
 rays.push({name:`${mode}-${x}-${y}`,run:r,target,mask,observed,expected:previewProjectileTrace(r,target,{mask,observedActors:r.actors[r.depth].filter(a=>observed.includes(a.id))})});
}
for(const target of [null,{x:7,y:2}]){const r=make();rays.push({name:`invalid-${target?.x}`,run:r,target,mask:Array(49).fill(true),observed:[],expected:previewProjectileTrace(r,target,{mask:Array(49).fill(true),observedActors:[]})});}
const variants=['plain','unlucky','lucky','distracted','all-luck','blood','angel','missed-me','paralyzed','wheel','not-there','nightmare','surrender','adaptive','fury','shine','drown'];
for(const name of ['Bow','Composite bow'])for(const state of ['hunting','sleeping'])for(const range of ['near','far'])for(const variant of variants){
 const r=make();r.tick=4;r.player.energy=15;const bow={...makeEquipment(name,700),equipped:'hand1',enchantment:3};r.inventory=[bow,makeArrows(701)];const a=createBestiaryActor('rat',200,5,range==='near'?1:5);a.state=state;a.hp=60;a.maxHp=60;r.actors[r.depth]=[a];
 if(['unlucky','lucky','distracted'].includes(variant))r.player.conditions[variant]=1;
 if(variant==='all-luck')r.player.conditions={unlucky:1,lucky:1,distracted:1};
 if(variant==='blood'){r.player.abilities=[id('Blood Weapon')];r.player.conditions.bloodWeaponReady=1;}
 if(variant==='angel')r.player.abilities=[id('The Number of Blood')];
 if(variant==='paralyzed')a.conditions.paralyzed=3;
 const effect={'missed-me':'Missed Me',wheel:'Turning the Wheel','not-there':'Not There',nightmare:'Waking Nightmare',surrender:'Never Surrender',adaptive:'Adaptive Armor'}[variant];if(effect){a.abilities.push(id(effect));a.active=[id(effect)];a.reactions=3;a.nightmares=2;a.energy=100;a.stats.defense=10;}
 if(variant==='fury'){bow.enhancements=[{craft:'Fury',active:true,bonus:{}}];r.player.abilities=[id('Unyielding Assault')];r.player.active=[id('Unyielding Assault')];}
 if(variant==='shine')r.player.abilities=[id('Shine')];
 if(variant==='drown'){r.player.abilities=[id('Drown')];r.floors[0].tiles[8]='water';}
 const view=huntingCombatView(r,a),shot=bowAttackPreview(view.run,bow,view.actor);const expected=`Bow shot: ${(shot.hitChance*100).toFixed(1)}% hit · ${shot.minimum}–${shot.maximum} damage · uses 1 arrow.`;bows.push({name:`${name}-${state}-${range}-${variant}`,run:r,bow:700,actor:200,trace:{path:[],stopReason:'target'},expected});
}
for(const kind of ['unequipped','no-arrows','terrain','creature','unknown']){const r=make(),a=createBestiaryActor('rat',200,5,1),bow={...makeEquipment('Bow',700),equipped:'hand1'};r.inventory=[bow,makeArrows(701)];r.actors[r.depth]=[a];if(kind==='unequipped')delete bow.equipped;if(kind==='no-arrows')r.inventory.pop();const view=huntingCombatView(r,a),shot=bowAttackPreview(view.run,bow,view.actor);const expected=shot.reason?`Bow shot unavailable: ${shot.reason}`:kind==='unknown'?'Bow shot: the full projectile path is not currently visible.':'Bow shot: blocked by visible terrain or another visible creature. The arrow stops at the first obstruction.';bows.push({name:kind,run:r,bow:700,actor:200,trace:{path:[],stopReason:['terrain','creature','unknown'].includes(kind)?kind:'target'},expected});}
fs.writeFileSync(new URL('../fixtures/targeting-ui-v48.json',import.meta.url),JSON.stringify({rays,bows}));console.log(`Exported ${rays.length} observed rays and ${bows.length} Bow reports.`);
