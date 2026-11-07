import fs from 'node:fs';
import crypto from 'node:crypto';
const root=new URL('../../release-v48/source/call-of-the-corpse-gods/dist/src/',import.meta.url);
const {dispatch}=await import(new URL('engine.js',root));
const {consumePresentationEvents}=await import(new URL('action-events.js',root));
const base=JSON.parse(fs.readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url)));
const cases=[];
for(const name of ['perfect','ordinary','dead','paralyzed']) {
 const r=structuredClone(base); const a=r.actors[1][0];a.state='ally';a.perfect=name!=='ordinary';if(name==='dead'){a.dead=true;a.hp=0;}if(name==='paralyzed')r.player.conditions.paralyzed=3;
 const initial=structuredClone(r);const commands=[{type:'ally',id:a.id},{type:'rally',x:r.player.x+2,y:r.player.y},{type:'ally',id:a.id},{type:'ally',id:999999},{type:'wait'}];
 const steps=commands.map(command=>{dispatch(r,command,true);return{command,state:structuredClone(r),events:consumePresentationEvents(r)}});cases.push({name,initial,steps});
}
const out=new URL('../fixtures/ally-command-traces-v48.json',import.meta.url);const data=JSON.stringify(cases);fs.writeFileSync(new URL(out.href+'.tmp'),data);fs.renameSync(new URL(out.href+'.tmp'),out);
fs.writeFileSync(new URL(out.href+'.manifest.json'),JSON.stringify({exporterVersion:1,cases:4,commands:20,sha256:crypto.createHash('sha256').update(data).digest('hex'),source:'frozen release-v48',sourceSha256:crypto.createHash('sha256').update(fs.readFileSync(new URL('engine.js',root))).digest('hex'),exporterSha256:crypto.createHash('sha256').update(fs.readFileSync(new URL(import.meta.url))).digest('hex')},null,2));
