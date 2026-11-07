// Frozen v48 detached visual-light oracle. Never changes reference files.
import fs from 'node:fs';import path from 'node:path';import {pathToFileURL} from 'node:url';
const root=path.resolve(process.argv[2]??'../release-v48/source/call-of-the-corpse-gods');
const mod=n=>import(pathToFileURL(path.join(root,`dist/src/${n}.js`)));
const {parseAppearance}=await mod('appearance-config');const {setAppearance}=await mod('appearance-runtime');
const {createVisualLightField,visualLightFlicker,visualLightingAnimated}=await mod('ui-lighting');
const cases=[];
const base=()=>({width:8,height:1,time:450,cells:Array.from({length:8},(_,i)=>({current:true,tile:'floor',ambient:0,illumination:Math.max(0,30-i*10),doorOpen:null,burning:false})),sources:[{kind:'torch',colorRole:'torch',x:0,y:0,power:30,falloff:10,cells:[0,1,2],strengths:[30,20,10]}]});
function record(name,appearance={},scene=base()){
 setAppearance(parseAppearance(appearance));
 const snapshot={sources:scene.sources.filter(s=>s.colorRole!=='darkness'),darkness:scene.sources.filter(s=>s.colorRole==='darkness'),ambient:scene.cells.map(c=>c.ambient),illumination:Float64Array.from(scene.cells.map(c=>c.illumination))};
 const context={width:scene.width,height:scene.height,time:scene.time,observable:Uint8Array.from(scene.cells.map(c=>+c.current)),materials:new Map(scene.cells.flatMap((c,i)=>c.tile?[[i,c.tile]]:[])),blockedCells:new Set(scene.cells.flatMap((c,i)=>c.doorOpen===false?[i]:[])),burningLiquidCells:new Set(scene.cells.flatMap((c,i)=>c.burning?[i]:[]))};
 const field=createVisualLightField(snapshot,context);
 cases.push({name,appearance,scene,animated:visualLightingAnimated(),expected:{energy:Array.from(field.energy),wallEnergy:Array.from(field.wallEnergy),ambient:Array.from(field.ambient),illumination:Array.from(field.illumination)}});
}
record('defaults');
for(const [name,value]of Object.entries({ambient:1.5,intensity:2,visualRadius:2,falloff:2,falloffCurve:'linear',wallResponse:2,floorResponse:.2,blending:'screen',maximumIllumination:.2,flickerStrength:.6,glow:2}))record(`global-${name}`,{lighting:{[name]:value}});
for(const [name,value]of Object.entries({color:'#1234ee',intensity:2,visualRadius:2,falloff:2,curve:'quadratic',wallResponse:2,floorResponse:.2,flickerStrength:.6,glow:2}))record(`torch-${name}`,{lighting:{sources:{torch:{[name]:value}}}});
for(const scope of ['global','source'])for(const name of ['flickerFrequency','flickerSmoothness'])record(`${scope}-${name}`,{lighting:{flickerStrength:.8,...(scope==='global'?{[name]:.2}:{sources:{torch:{[name]:.2}}})}});
for(const obstacle of ['unknown','wall','tree','door','remembered']){const scene=base();if(obstacle==='unknown'){scene.cells[3].current=false;scene.cells[3].tile=null;}else if(obstacle==='remembered'){scene.cells[3].current=false;}else if(obstacle==='door'){scene.cells[3].doorOpen=false;}else{scene.cells[3].tile=obstacle;}record(`radius-${obstacle}`,{lighting:{visualRadius:2}},scene);}
for(const blend of ['soft','screen','additive']){const scene=base();scene.sources.push({kind:'shine',colorRole:'magic',x:2,y:0,power:70,falloff:10,cells:[0,1,2,3,4,5,6,7],strengths:[50,60,70,60,50,40,30,20]});scene.sources.push({kind:'darkness-aura',colorRole:'darkness',x:4,y:0,power:30,falloff:10,cells:[2,3,4,5,6],strengths:[10,20,30,20,10]});record(`overlap-${blend}`,{lighting:{blending:blend}},scene);}
for(const kind of ['fire','brazier','shine','neutral','burning-liquid']){const scene=base();Object.assign(scene.sources[0],{kind:kind==='burning-liquid'?'fire':kind,colorRole:kind==='shine'?'magic':kind==='neutral'?'neutral':'fire'});if(kind==='burning-liquid')scene.cells[0].burning=true;record(`category-${kind}`,{},scene);}
{const scene=base();scene.sources.push({...scene.sources[0],kind:'fire',colorRole:'fire'});record('dedup-fire-fixture',{},scene);}
for(const block of [false,true]){const scene={width:5,height:5,time:450,cells:Array.from({length:25},()=>({current:true,tile:'floor',ambient:-10,illumination:0,doorOpen:null,burning:false})),sources:[{kind:'torch',colorRole:'torch',x:2,y:2,power:10,falloff:10,cells:[12],strengths:[10]}]};if(block){scene.cells[13].tile='wall';scene.cells[17].doorOpen=false;}record(`diagonal-${block}`,{lighting:{visualRadius:3}},scene);}
const flicker=[];for(const time of [-1,0,450,1234,2234,1e7])for(const frequency of [0,1,2])for(const smoothness of [0,.8,1])flicker.push({x:3,y:4,time,frequency,smoothness,value:visualLightFlicker(3,4,time,frequency,smoothness)});
fs.writeFileSync('crates/corpse-desktop/assets/appearance-light-parity-v48.json',JSON.stringify({cases,flicker})+'\n');console.log(`${cases.length} light-field cases; ${flicker.length} flicker samples`);
