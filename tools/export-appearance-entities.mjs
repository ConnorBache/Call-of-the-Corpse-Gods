// Frozen source oracle for entity inheritance and material feature authoring.
import fs from 'node:fs';import path from 'node:path';import {pathToFileURL} from 'node:url';
const root=path.resolve(process.argv[2]??'../release-v48/source/call-of-the-corpse-gods');const mod=n=>import(pathToFileURL(path.join(root,`dist/src/${n}.js`)));
const {parseAppearance}=await mod('appearance-config');const {setAppearance}=await mod('appearance-runtime');
const {entityAppearance,materialAppearance,featureColors,featureColor,materialPulse,entityPulse,worldAppearanceAnimated}=await mod('ui-world-appearance');const {colorChannels}=await mod('ui-color');
const entities=[];const base={glyph:'&',color:'#b64793'};
function entity(name,appearance,group,key,parent){setAppearance(parseAppearance(appearance));const e=entityAppearance(group,key,base,parent);entities.push({name,appearance,group,key,parent:parent??null,base:{glyph:base.glyph,color:colorChannels(base.color)},expected:{glyph:e.glyph,color:colorChannels(e.color),background:e.background==='transparent'?null:colorChannels(e.background),emphasis:e.emphasis,speed:e.animationSpeed}});}
for(const[group,key,parent]of[['objects','player'],['objects','corpse'],['creatures','rat'],['creatures','future'],['items','Shortsword','kind:weapon'],['items','future','kind:weapon']]){
 entity(`${group}-${key}-defaults`,{},group,key,parent);
 for(const[field,value]of Object.entries({glyph:'$',color:'#1234ee',background:'#341222',brightness:2,saturation:.2,emphasis:2,animationSpeed:2})){entity(`${group}-${key}-shared-${field}`,{entities:{defaults:{[field]:value}}},group,key,parent);}
 if(key!=='future'){
  entity(`${group}-${key}-override`,{entities:{[group]:{[key]:{inherited:false,glyph:'X',color:'#1234ee',background:'#123456',brightness:1.7,saturation:.3,emphasis:1.5,animationSpeed:2.3}}}},group,key,parent);
  entity(`${group}-${key}-inherited-tuning`,{entities:{[group]:{[key]:{inherited:true,brightness:1.5,saturation:.2,emphasis:2,animationSpeed:3}}}},group,key,parent);
 }
}
entity('explicit-parent',{entities:{items:{'kind:weapon':{inherited:false,glyph:'W',color:'#123456',brightness:1.4,saturation:.5,emphasis:2,animationSpeed:1}}}},'items','Shortsword','kind:weapon');
entity('parent-child-numeric-precedence',{entities:{defaults:{brightness:3},items:{'kind:weapon':{brightness:1.4,saturation:.5,emphasis:2,animationSpeed:1},Shortsword:{brightness:1,saturation:2,emphasis:1,animationSpeed:0}}}},'items','Shortsword','kind:weapon');
entity('future-ignored',{entities:{creatures:{future:{inherited:false,color:'#ff0000',glyph:'F',brightness:2}}}},'creatures','future');
const materials=[];
for(const[key,parent]of[['wall:natural','wall'],['ground:sand','floor'],['fixture:door-open','fixture:door'],['water',null]])for(const appearance of [{},{terrain:{[parent??key]:{color:'#123456',brightness:1.5,lightingResponse:2,glyph:'*'}}},{terrain:{[parent??key]:{color:'#123456',brightness:1.5},[key]:{color:'#abcdef',brightness:1}}}]){setAppearance(parseAppearance(appearance));materials.push({appearance,key,parent,expected:materialAppearance(key,parent??undefined)});}
const features=[];
for(const key of ['feature:slime','feature:ichor','feature:fire','gas:caustic','missing'])for(const appearance of [{},{terrain:{[key]:{color:'#1234ee'}}},{terrain:{[key]:{color:'#1234ee',background:'#543210',brightness:1.4,saturation:.3,contrast:1.4,colorVariation:2,brightnessVariation:3}}}])for(const useFallback of [false,true]){
 if(key==='missing'&&Object.keys(appearance).length)continue;
 setAppearance(parseAppearance(appearance));const fallback=useFallback?{color:'#987654',background:'#123456'}:undefined;const colors=featureColors(key,11,19,123456,fallback);features.push({appearance,key,x:11,y:19,seed:123456,fallback:useFallback?[colorChannels(fallback.color),colorChannels(fallback.background)]:null,expected:[colorChannels(colors.color),colorChannels(colors.background)],single:colorChannels(featureColor(key,'#987654'))});
}
const pulses=[];for(const speed of [0,1,2.3])for(const now of [0,450,1234])pulses.push({speed,now,x:3,y:4,material:materialPulse({animationSpeed:speed},now,3,4),entity:entityPulse({animationSpeed:speed},now,3,4)});
const animated=[];for(const appearance of [{},{path:{animationSpeed:1}},{terrain:{wall:{animationSpeed:2}}},{entities:{defaults:{animationSpeed:2}}},{entities:{creatures:{rat:{animationSpeed:1}}}},{entities:{creatures:{future:{animationSpeed:1}}}}]){setAppearance(parseAppearance(appearance));animated.push({appearance,expected:worldAppearanceAnimated()});}
fs.writeFileSync('crates/corpse-desktop/assets/appearance-entities-parity-v48.json',JSON.stringify({entities,materials,features,pulses,animated})+'\n');console.log(`${entities.length} entity, ${materials.length} material, ${features.length} feature, ${pulses.length} pulse, ${animated.length} animation cases`);
const {mapColors}=await mod('ui-tiles');
const fog=[];
for(const knowledge of ['direct','telepathic','remembered','layout'])for(const allSeeing of [false,true])for(const chance of [10,50,100])for(const appearance of [{},{fog:{transition:.8,allSeeingColor:'#ff5511',allSeeingIntensity:1.5,revealColor:'#22bb11',revealIntensity:.7,telepathyColor:'#cc44ee',telepathyIntensity:1.3,visibleContrast:1.2,rememberedBrightness:1.4,rememberedSaturation:.8}}]){
 setAppearance(parseAppearance(appearance));
 const context={material:'floor',distance:8,illumination:0,index:0,visualLight:{energy:new Float32Array([.1,.2,.3]),wallEnergy:new Float32Array([.3,.2,.1]),ambient:new Float32Array([0]),illumination:new Float64Array([0])},chance,allSeeing,lightingResponse:1};
 const result=mapColors({color:'#515b6b',background:'#151923'},{knowledge,...context});
 fog.push({appearance,knowledge,allSeeing,chance,expected:[colorChannels(result.color),colorChannels(result.background)]});
}
const output=JSON.parse(fs.readFileSync('crates/corpse-desktop/assets/appearance-entities-parity-v48.json'));output.fog=fog;
fs.writeFileSync('crates/corpse-desktop/assets/appearance-entities-parity-v48.json',JSON.stringify(output)+'\n');console.log(`${fog.length} observed fog-stage cases`);
