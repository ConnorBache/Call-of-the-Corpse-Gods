// Build-time immutable catalog export only. The native executable never invokes Node.
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
import { writeFileSync } from 'node:fs';
const root=resolve(process.argv[2]);
const mod=async name=>import(pathToFileURL(resolve(root,'dist/src',name+'.js')).href);
const {createBestiaryIndex}=await mod('bestiary-index');
const {BESTIARY_COMPANIONS}=await mod('bestiary-companions');
const {bestiaryAlternateForms}=await mod('bestiary-forms');
const {bestiaryIllustration}=await mod('bestiary-illustrations');
// Owner-directed bestiary exclusions only; summoned abilities and artwork remain intact.
const excluded=new Set(['half-here','mini-me']);
const companions=BESTIARY_COMPANIONS.filter(e=>!excluded.has(e.id));
const depths=createBestiaryIndex().map(d=>({...d,ordinary:d.ordinary.filter(e=>!excluded.has(e.id)),conditional:d.conditional.filter(e=>!excluded.has(e.id))}));const entries=new Map();
for(const depth of depths)for(const e of [...depth.ordinary,...depth.conditional])entries.set(e.id,e);
for(const e of companions)entries.set(e.id,e);
const output={version:1,source:'Frozen v48 immutable bestiary catalog; no runtime simulation dependency',entries:[...entries.values()].map(e=>({...e,asset:bestiaryIllustration(e).assetId,family:bestiaryIllustration(e).family,forms:bestiaryAlternateForms(e.kind)})),depths:depths.map(d=>({depth:d.depth,ordinary:d.ordinary.map(e=>e.id),conditional:d.conditional.map(e=>e.id)})),companions:companions.map(e=>e.id)};
writeFileSync(process.argv[3],JSON.stringify(output,null,2)+'\n');console.log(`Exported ${output.entries.length} entries, ${depths.length} depth rosters.`);
