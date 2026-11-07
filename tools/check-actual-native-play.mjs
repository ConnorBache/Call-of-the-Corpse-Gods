import fs from 'node:fs';
import assert from 'node:assert/strict';
import {createRun,dispatch} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/engine.js';
const saved=JSON.parse(fs.readFileSync(process.argv[2] ?? new URL('../native-generated-qa-save.json',import.meta.url),'utf8'));
const native=saved.nativeSaveVersion===1 ? (typeof saved.run==='string'?JSON.parse(saved.run):saved.run) : saved;
const reference=createRun(native.seed,native.mode,native.dev);
for(const command of native.replay) dispatch(reference,command,true);
const expected=JSON.parse(JSON.stringify(reference));
for(const key of ['history','hasteSnapshot','runId','inventoryDisplayOrder']) {delete native[key];delete expected[key];}
function diff(a,b,p='$'){if(a===b)return null;if(typeof a!==typeof b||a===null||b===null||typeof a!=='object')return{path:p,actual:a,expected:b};if(Array.isArray(a)!==Array.isArray(b))return{path:p,type:'array mismatch'};if(Array.isArray(a)&&a.length!==b.length)return{path:p,actualLength:a.length,expectedLength:b.length};const keys=new Set([...Object.keys(a),...Object.keys(b)]);for(const k of keys){const d=diff(a[k],b[k],p+'.'+k);if(d)return d;}return null;}
const mismatch=diff(native,expected);if(mismatch){const differences=[...new Set([...Object.keys(expected),...Object.keys(native)])].map(k=>diff(native[k],expected[k],'$.'+k)).filter(Boolean);console.error(JSON.stringify(differences,null,2));fs.writeFileSync(new URL('../validation/actual-ui/native-play-expected.json',import.meta.url),JSON.stringify(expected));process.exit(1);}
const report={verified:true,seed:native.seed,mode:native.mode,commands:native.replay.length,tick:native.tick,health:native.player.hp,blessings:native.exp,scope:'Authoritative gameplay fields from actual SDL input play; excludes separately validated temporal representation, nondeterministic run ID and UI-only inventory display ordering.'};
fs.writeFileSync(new URL('../validation/actual-ui/native-play-state-check.json',import.meta.url),JSON.stringify(report,null,2));console.log(report);
