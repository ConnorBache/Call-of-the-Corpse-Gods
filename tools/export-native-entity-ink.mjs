// Frozen source oracle for actual observed actor tone; build-time only.
import fs from 'node:fs';import {resolve} from 'node:path';import {pathToFileURL} from 'node:url';
const root=resolve(process.argv[2]);const mod=n=>import(pathToFileURL(resolve(root,'dist/src',n+'.js')).href);
const p=await mod('perception'),{renderDungeon}=await mod('ui-map'),{createVisualLightField}=await mod('ui-lighting'),{colorChannels}=await mod('ui-color');
const base=JSON.parse(fs.readFileSync(process.argv[3]));const cases=[];
for(const ambient of [-90,-40,0,50,100])for(const distance of [1,4]){
const r=structuredClone(base), f=r.floors[0],a=r.actors['1'][0];f.lights.fill(ambient);a.x=r.player.x+distance;a.y=r.player.y;
const snap=p.terrainLightingObservation(r),lighting=snap.illumination,direct=p.directTerrainVisibilityChance(r,lighting),sight=p.terrainVisibilityChance(r,direct),mask=p.terrainVisibility(r),actors=r.actors[1].filter(a=>p.visibleActor(r,a)),drawn=[];
const ctx=new Proxy({font:'17px monospace',fillStyle:'#fff',globalAlpha:1,save(){},restore(){},fillText(g,x,y){drawn.push({glyph:g,x,y,color:this.fillStyle,alpha:this.globalAlpha});},measureText(g){return{width:g.length*9,actualBoundingBoxLeft:g.length*4.5,actualBoundingBoxRight:g.length*4.5,actualBoundingBoxAscent:7,actualBoundingBoxDescent:7};}}, {get(o,k){return k in o?o[k]:()=>{};}});
renderDungeon(ctx,{run:r,mask,sightChances:sight,directChances:direct,lighting,visualLight:createVisualLightField(snap),presentationTime:0,playerIllumination:p.light(r,r.player),camera:{x:0,y:0,scale:1},width:79*15,height:29*15,observedActors:actors,telepathicOnlyIds:new Set(actors.filter(a=>!p.directlyVisibleActor(r,a)).map(a=>a.id)),shownItems:[],hover:null,targeting:false,travelPath:[],hoverPath:[]});
const ink=drawn.findLast(d=>d.glyph===a.glyph&&Math.floor(d.x/15)===a.x&&Math.floor(d.y/15)===a.y);
cases.push({ambient,distance,visible:!!ink,color:ink?colorChannels(ink.color):null});}
fs.writeFileSync(process.argv[4],JSON.stringify(cases,null,2));console.log(cases.length+' actor ink cases');
