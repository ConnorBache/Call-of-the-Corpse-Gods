// Frozen renderer probes for known semantic overlays. Build/test only.
import fs from 'node:fs';import {resolve} from 'node:path';import {pathToFileURL} from 'node:url';
const root=resolve(process.argv[2]),mod=n=>import(pathToFileURL(resolve(root,'dist/src',n+'.js')).href);
const p=await mod('perception'),{renderDungeon}=await mod('ui-map'),{createVisualLightField}=await mod('ui-lighting'),{colorChannels}=await mod('ui-color');
const base=JSON.parse(fs.readFileSync(process.argv[3])),cases=[];
for(const key of ['slime','mud','ichor','razorleaves','blood','bulwarks','torch','brazier','beacon','fire']){
 const r=structuredClone(base),f=r.floors[0],x=r.player.x,y=r.player.y+1,at=y*79+x;r.actors['1']=[];
 if(key==='fire'){f.permanentFire=[at];}else if(key==='blood'){f.blood=Array(79*29).fill(false);f.blood[at]=true;}else if(key==='bulwarks')f.bulwarks={[at]:r.tick+2};else if(['torch','brazier','beacon'].includes(key))f.lightSources=[{x,y,power:50,falloff:5,kind:key}];else {f[key]=Array(79*29).fill(0);f[key][at]=1;}
 const snap=p.terrainLightingObservation(r),lighting=snap.illumination,direct=p.directTerrainVisibilityChance(r,lighting),sight=p.terrainVisibilityChance(r,direct),mask=p.terrainVisibility(r),drawn=[];
 const ctx=new Proxy({font:'17px monospace',fillStyle:'#fff',globalAlpha:1,save(){},restore(){},fillText(g,x,y){drawn.push({glyph:g,x,y,color:this.fillStyle,alpha:this.globalAlpha});},measureText(g){return{width:g.length*9,actualBoundingBoxLeft:g.length*4.5,actualBoundingBoxRight:g.length*4.5,actualBoundingBoxAscent:7,actualBoundingBoxDescent:7};}},{get(o,k){return k in o?o[k]:()=>{};}});
 const result=renderDungeon(ctx,{run:r,mask,sightChances:sight,directChances:direct,lighting,visualLight:createVisualLightField(snap),presentationTime:0,playerIllumination:p.light(r,r.player),camera:{x:0,y:0,scale:1},width:79*15,height:29*15,observedActors:[],telepathicOnlyIds:new Set(),shownItems:[],hover:null,targeting:false,travelPath:[],hoverPath:[]});
 const ink=drawn.findLast(d=>Math.floor(d.x/15)===x&&Math.floor(d.y/15)===y);cases.push({key,x,y,bg:colorChannels(result.tileColors.get(at).background),glyph:ink?.glyph,color:ink?colorChannels(ink.color):null,alpha:ink?.alpha});
}
fs.writeFileSync(process.argv[4],JSON.stringify(cases,null,2));console.log(cases.length+' semantic feature cases');
