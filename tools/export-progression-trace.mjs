import fs from 'node:fs';import {dispatch} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/engine.js';
const r=JSON.parse(fs.readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url)));r.actors[1]=[];r.offers=['anavani-make-kneel'];const initial=structuredClone(r),steps=[];for(const command of [{type:'wait'},{type:'choose',id:'bad'},{type:'choose',id:'anavani-make-kneel'},{type:'ability',id:'anavani-make-kneel',target:0}]){dispatch(r,command,true);steps.push({command,state:structuredClone(r)});}
atomicWrite(new URL('../fixtures/progression-trace-v48.json',import.meta.url),JSON.stringify({initial,steps}));
function atomicWrite(file,contents){const temp=new URL(file.href+'.tmp');fs.writeFileSync(temp,contents);fs.renameSync(temp,file);}
