import fs from'node:fs';import{deserialize,serialize}from'../../release-v48/source/call-of-the-corpse-gods/dist/src/persistence.js';import{snapshot,recordInverse,recordFreeInverse}from'../../release-v48/source/call-of-the-corpse-gods/dist/src/rewind.js';import{appendLog}from'../../release-v48/source/call-of-the-corpse-gods/dist/src/log.js';
import{environment}from'../../release-v48/source/call-of-the-corpse-gods/dist/src/environment.js';
const initial=JSON.parse(fs.readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url),'utf8'));const cases=[];
function add(name,r,packed=false){const raw=packed?serialize(r):JSON.stringify(r);try{cases.push({name,raw,expected:deserialize(raw)});}catch(e){console.log('oracle rejected',name,e.message);throw e;}}
add('current raw',structuredClone(initial));add('current packed',structuredClone(initial),true);
function old(){const r=structuredClone(initial);delete r.divineRevision;delete r.enemyHealthRevision;delete r.gameplayRevision;delete r.generatedAbilityGrants;r.player.stats.defense=1.5;r.player.adaptiveDr=5;r.player.conditions.adaptiveAt=8;r.player.abilities.push('isheilah-quick-hand');const a=r.actors[1][0];a.name='Vent Fungus';a.maxHp=18;a.hp=8;a.glyph='f';a.stats.dr=0;const bow=r.inventory.find(i=>i.rangedWeapon==='bow');bow.hands=1;bow.equipped='hand2';r.inventory[0].bonus={defense:2.5};const h={...r.inventory[0],id:10001,kind:'heart',name:'Pale Spitter heart',glyph:'♥',bonus:{perception:1.5,accuracy:2},effect:'+1.5 Perception',equipped:undefined};r.inventory.push(h);r.inventoryDisplayOrder=[h.id,h.id,-1];return r;}
add('old world',old());
let r=old();r.player.equipmentAbilities=['isheilah-quick-hand'];r.inventory[1].grantedAbility='isheilah-quick-hand';r.player.active.push('isheilah-quick-hand');add('stale bow equipment grant',r);
r=old();r.inventory.push({...r.inventory.find(i=>i.kind==='ammo'),id:10002,quantity:3});r.actors[1][0].name='Pale Birther';add('stack consolidation',r);
r=old();const plate={...r.inventory[0],id:10003,kind:'armor',name:'Plate armor of +1 defense',tinkered:true,grantedAbility:'isheilah-quick-hand',equipped:undefined};r.inventory.push(plate);add('tinkered plate name',r);
r=old();const anchor=old();anchor.history=[];anchor.replay=[];anchor.future=null;r.future=serialize(anchor);add('recursive future',r);
r=old();for(let i=0;i<3;i++){const before=snapshot(r);r.tick++;r.player.x++;r.actors[1][0].hp--;appendLog(r,'old step '+i,'player');recordInverse(r,before);const free=snapshot(r);r.inventory[0].identified=true;recordFreeInverse(r,free);}add('history raw',structuredClone(r));add('history packed',structuredClone(r),true);r.hasteSnapshot=JSON.stringify(snapshot(r));r.player.x++;r.tick++;appendLog(r,'pending old phase','player');add('history pending haste',r);
r=old();environment(r);let env=r.floors[0].environment;for(const key of['gasType','gasVolume','gasOwner','gasPlayer','fireKind','fireFresh','fireReaction','nextFireReaction'])delete env[key];env.caustic[100]=4;env.steam[100]=3;env.fire[101]=2;add('legacy propagation',r);
r=old();r.ground[1]=[{x:12,y:14,item:{...r.inventory[0],id:20001,equipped:undefined}},{x:12,y:14,item:{...r.inventory[0],id:20002,equipped:undefined}}];add('duplicate ground placement',r);
r=old();let a=r.actors[1][0];a.dead=true;a.hp=0;r.actors[1].push({...structuredClone(a),id:20003});add('duplicate corpse placement',r);
r=old();r.player.abilities.push('da wel');r.player.abilities.pop();r.player.active.push('morae-bonded-by-unseen-threads');r.player.conditions.bondedTarget=r.actors[1][0].id;r.actors[1][0].dead=true;r.actors[1][0].hp=0;add('dead maintained target',r);
r=old();r.actors[1][0].creatureKind='zillennia';r.actors[1][0].name='Zillennia';add('legacy darkness trait',r);
r=old();r.runId='legacy-run-id';r.hasteSnapshot=JSON.stringify(snapshot(r));r.player.x++;add('pending with run identity',r);
r=old();r.hasteSnapshot='';add('empty pending marker',r);
r=old();r.inventory.at(-1).name='Pale Spitter\theart';add('legacy heart whitespace',r);
r=old();r.inventory.at(-1).name='Blorb\u0085 heart';add('non-JS whitespace identity',r);
const path=new URL('../fixtures/load-migrations-v48.json',import.meta.url).pathname;fs.writeFileSync(path+'.tmp',JSON.stringify(cases));fs.renameSync(path+'.tmp',path);console.log(cases.length,'full load cases');
