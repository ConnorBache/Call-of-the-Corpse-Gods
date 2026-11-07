import fs from 'node:fs';
import {clearMaintainedLink,endMaintainedLinksToDeadTarget,normalizeDeadMaintainedLinks} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/maintained-links.js';
const base=JSON.parse(fs.readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url)));
const links=[['morae-bonded-by-unseen-threads','bondedTarget',['bondedTarget']],['pelian-i-came-for-you','cameTarget',['cameTarget','cameTurns','cameFree']],['isheilah-make-slippery','slipperyTarget',['slipperyTarget']],['etill-riptide','riptideTarget',['riptideTarget','riptideX','riptideY'],'riptide'],['dawel-unspeak','unspeakTarget',['unspeakTarget','unspeakCost'],'silenced']];const cases=[];
for(const op of['clear','end','normalize'])for(const link of links)for(const kind of['alive','dead','missing','offfloor','duplicate','zero','inactive','clone']){
 const r=structuredClone(base);r.player.abilities=[link[0]];r.player.active=kind==='inactive'?[]:[link[0]];r.player.conditions={};for(const key of link[2])r.player.conditions[key]=3;r.player.conditions[link[1]]=4;
 let target=r.actors[1][0];target.hp=kind==='zero'||kind==='dead'?0:1;target.dead=kind==='dead';if(link[3])target.conditions[link[3]]=5;
 if(kind==='missing')r.actors[1]=[];
 if(kind==='offfloor'){r.actors[2]=[target];r.actors[1]=[];}
 if(kind==='duplicate')r.actors[1].push(structuredClone(target));
 if(kind==='clone'){const clone=structuredClone(r.player);clone.id=7;clone.state='ally';clone.conditions.halfHereClone=1;clone.conditions.halfHereOwner=r.player.id;r.player.abilities.push('etill-half-here');r.player.active.push('etill-half-here');r.player.conditions.halfHereId=7;r.actors[1].push(clone);target.dead=true;}
 const initial={player:structuredClone(r.player),actors:structuredClone(r.actors)};let changed=null;
 if(op==='clear')changed=clearMaintainedLink(r,r.player,link[0]);else if(op==='end')endMaintainedLinksToDeadTarget(r,target);else changed=normalizeDeadMaintainedLinks(r);
 cases.push({op,kind,ability:link[0],target:4,initial,expected:{player:r.player,actors:r.actors},changed});
}
const p=new URL('../fixtures/maintained-links-v48.json',import.meta.url),t=new URL(p.href+'.tmp');fs.writeFileSync(t,JSON.stringify(cases));fs.renameSync(t,p);console.log(cases.length);
