// Frozen-source read-only ability UI oracle; development only.
import fs from'node:fs';import{resolve}from'node:path';import{pathToFileURL}from'node:url';
const root=resolve(process.argv[2]),base=JSON.parse(fs.readFileSync(process.argv[3]));const mod=n=>import(pathToFileURL(resolve(root,'dist/src',n+'.js')).href);const{ABILITIES}=await mod('abilities');const{abilityPresentation}=await mod('ability-presentation');const{compactAbilitySummary}=await mod('ui-ability-list');
const cases=[];for(const a of ABILITIES){for(const active of [false,true]){const r=structuredClone(base);r.player.abilities=[a.id];r.player.active=active?[a.id]:[];const p=abilityPresentation(r,a.id);cases.push({id:a.id,active,summary:compactAbilitySummary(p,a.flags.includes('toggle')),cost:p.costLabel,mode:p.mode});}}
fs.writeFileSync(process.argv[4],JSON.stringify(cases,null,2));console.log(cases.length);
