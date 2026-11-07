import fs from 'node:fs';import {dispatch} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/engine.js';
const r=JSON.parse(fs.readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url)));r.actors[1]=[];r.player.hp=10;
r.inventory.push({id:100,name:'Hemogoblin heart',kind:'heart',glyph:'♥',heartCreatureType:'Hemogoblin',bonus:{offense:1,defense:1},energy:10,exp:1,quantity:2,enhancements:[]},{id:101,name:'Unidentified potion',kind:'potion',glyph:'!',effect:'healing',identified:false,enhancements:[]});r.nextId=102;
const initial=structuredClone(r),steps=[];for(const command of [{type:'use',id:101},{type:'use',id:101},{type:'use',id:100},{type:'use',id:100}]){dispatch(r,command,true);steps.push({command,state:structuredClone(r)});}
atomicWrite(new URL('../fixtures/consumable-trace-v48.json',import.meta.url),JSON.stringify({initial,steps}));

function atomicWrite(path,contents){const file=path instanceof URL?path:new URL(path,import.meta.url);const temp=new URL(file.href+".tmp");fs.writeFileSync(temp,contents);fs.renameSync(temp,file);}
