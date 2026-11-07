// Source-only differential probe. Build/test tool, never a native runtime dependency.
import fs from 'node:fs';import {resolve} from 'node:path';import {pathToFileURL} from 'node:url';
const root=resolve(process.argv[2]);const mod=n=>import(pathToFileURL(resolve(root,'dist/src',n+'.js')).href);
const r=JSON.parse(fs.readFileSync(process.argv[3]));const p=await mod('perception');const {renderDungeon}=await mod('ui-map');const {createVisualLightField}=await mod('ui-lighting');const {colorChannels}=await mod('ui-color');
const f=r.floors.find(f=>f.depth===r.depth),width=f.width??79,height=f.height??29;
const snapshot=p.terrainLightingObservation(r),lighting=snapshot.illumination,directChances=p.directTerrainVisibilityChance(r,lighting),sightChances=p.terrainVisibilityChance(r,directChances),mask=p.terrainVisibility(r);
const actors=r.actors[r.depth].filter(a=>p.visibleActor(r,a));const drawn=[];
const ctx=new Proxy({font:'17px monospace',fillStyle:'#fff',globalAlpha:1,textAlign:'center',textBaseline:'middle',save(){},restore(){},fillText(g,x,y){drawn.push({glyph:g,x,y,color:this.fillStyle,alpha:this.globalAlpha,font:this.font});},measureText(g){return{width:g.length*9,actualBoundingBoxLeft:g.length*4.5,actualBoundingBoxRight:g.length*4.5,actualBoundingBoxAscent:7,actualBoundingBoxDescent:7};}}, {get(o,k){if(k in o)return o[k];return ()=>{};}});
const result=renderDungeon(ctx,{run:r,mask,sightChances,directChances,lighting,visualLight:createVisualLightField(snapshot),presentationTime:0,playerIllumination:p.light(r,r.player),camera:{x:0,y:0,scale:1},width:width*15,height:height*15,observedActors:actors,telepathicOnlyIds:new Set(actors.filter(a=>!p.directlyVisibleActor(r,a)).map(a=>a.id)),shownItems:p.knownFloorItems(r,mask),hover:null,targeting:false,travelPath:[],hoverPath:[]});
fs.writeFileSync(process.argv[4],JSON.stringify({width,height,tiles:[...result.tileColors].map(([index,t])=>({index,...t,fg:colorChannels(t.color),bg:colorChannels(t.background)})),drawn},null,2));
console.log(`Source renderer recorded ${result.tileColors.size} material cells and ${drawn.length} glyph draws.`);
