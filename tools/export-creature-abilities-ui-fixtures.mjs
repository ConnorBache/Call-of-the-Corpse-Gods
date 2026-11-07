// Source-authored metadata and dynamic read-only oracle; development only.
import fs from 'node:fs';
import {ENEMY_ABILITIES} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/enemy-data.js';
import {BESTIARY,EVENT_CREATURES,createBestiaryActor} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/bestiary.js';
import {ABILITIES} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/abilities.js';
import {PLAYER_CONTROLLED_ABILITY_EFFECTS} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/ability-effect-summaries.js';
import {ABILITY_ACTIVATION_COSTS} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/ability-costs.js';
import {upkeepCost} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/upkeep.js';
import {ZILLENNIA_QUALITIES} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/zillennia-qualities.js';
import {creatureAbilityDescription,creatureAbilityDetails} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/creature-ability-description.js';
const src=fs.readFileSync(new URL('../../release-v48/source/call-of-the-corpse-gods/src/creature-ability-description.ts',import.meta.url),'utf8');
const block=src.match(/const COMPACT_ENEMY_EFFECTS[^=]+=[\s\S]*?\n};/)[0];
const compact=Object.fromEntries([...block.matchAll(/"([^"]+)": "([^"]+)"/g)].map(m=>[m[1],m[2]]));
const base=JSON.parse(fs.readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url),'utf8'));
const defs=[...BESTIARY,...EVENT_CREATURES];
const metadata={enemy:Object.entries(ENEMY_ABILITIES).map(([id,d])=>({id,...d,compact:compact[id]??null})),definitions:defs.map(d=>({id:d.id,name:d.name,abilities:d.abilities})),zillenniaNames:ZILLENNIA_QUALITIES.map(q=>q.name),divine:ABILITIES.map(a=>({id:a.id,name:a.name,flags:a.flags,effect:a.effectSummary,controlled:PLAYER_CONTROLLED_ABILITY_EFFECTS.has(a.name),activation:ABILITY_ACTIVATION_COSTS[a.name]??0,upkeep:upkeepCost(base.player,a.name)}))};
fs.writeFileSync(new URL('../crates/corpse-desktop/assets/creature-abilities-v48.json',import.meta.url),JSON.stringify(metadata));
const cases=[];
function add(name,actor,patch={},player=false,shared=false){const run=structuredClone(base);run.inventory=[];run.actors[run.depth]=[];Object.assign(run,structuredClone(patch));actor=structuredClone(actor);if(player)run.player=actor;else if(shared){run.player={...structuredClone(actor),id:base.player.id};actor={...actor,id:200,name:'Half Here',conditions:{...actor.conditions,halfHereClone:1,halfHereOwner:run.player.id}};run.actors[run.depth]=[actor];}else run.actors[run.depth]=[actor];cases.push({name,actor,patch,player,shared,owner:shared?run.player:null,compact:creatureAbilityDescription(run,actor),details:creatureAbilityDetails(run,actor)});}
const generic=()=>({...structuredClone(base.player),id:200,name:'Unknown creature',abilities:[],active:[],conditions:{bestiaryTraitsInitialized:1},state:'hunting'});
for(const d of defs){const a=createBestiaryActor(d.id,200,13,14);add(`catalog-${d.id}`,a);add(`legacy-${d.id}`,{...a,abilities:[],conditions:{}});add(`erased-${d.id}`,{...a,abilities:[],conditions:{bestiaryTraitsInitialized:1}});}
for(const [id]of Object.entries(ENEMY_ABILITIES)) {
 const d=defs.find(d=>d.abilities.includes(id));const a=d?createBestiaryActor(d.id,200,13,14):generic();a.abilities=[id];a.conditions={bestiaryTraitsInitialized:1};
 for(const energy of [0,100])add(`${id}-${energy}`,{...a,energy});
 add(`${id}-timers`,{...a,energy:50,conditions:{...a.conditions,silenced:3,monsterFireReadyAt:10,monsterSpellReadyAt:11,monsterHealReadyAt:12,beastCooldown:3,monsterSpellPhase:1,raisedWallExpires:9,laserCharge:1,monsterVampireBat:1,monsterRetreating:1,monsterWendigoLevel:5,monsterSphinxHuntAt:12,monsterChargeDx:1,monsterChargeDy:0,monsterChargeRemaining:2,[`remembered:${id}`]:15}},{tick:5});
 add(`${id}-clone`,{...a,abilities:['etill-half-here',id]}, {},false,true);
}
for(const a of ABILITIES)for(const active of [false,true])for(const energy of [0,60])for(const player of [false,true]){
 const actor={...generic(),id:player?base.player.id:200,abilities:[a.id],active:active?[a.id]:[],energy,conditions:{bestiaryTraitsInitialized:1,unspeakCost:17,rageCost:22,revertCharges:2,fortunes:1,nightmareAt:90,bloodWeaponReady:1},blood:3,nightmares:2,reactions:2};
 actor.conditions[`remembered:${a.id}`]=20;add(`divine-${a.id}-${active}-${energy}-${player}`,actor,{tick:5},player);
}
for(const name of ['Pale Biter','Greater Pale Birther','Mercuria’s Greater Pale Birther','Pale Spitter','Unknown Pale Biter'])add(`pale-${name}`,{...generic(),name,conditions:{}});
for(const variant of ['rallied','paralyzed','recovery','ally','born','mini']){const a=createBestiaryActor('phoenix',200,13,14);a.abilities.push('enemy:pale:birth','enemy:pale:double-birth','enemy:pale:gas-shot','enemy:beast:stalk');if(variant==='rallied'){a.perfect=true;a.order='p';}if(variant==='paralyzed')a.conditions.paralyzed=3;if(variant==='recovery')a.conditions.rangedRecovery=1;if(variant==='ally')a.state='ally';if(variant==='born')a.bornFromBirther=true;if(variant==='mini')a.conditions.miniMe=1;add(variant,a);}
fs.writeFileSync(new URL('../fixtures/creature-abilities-ui-v48.json',import.meta.url),JSON.stringify(cases));
console.log(`Exported ${cases.length} dynamic ability reports.`);
