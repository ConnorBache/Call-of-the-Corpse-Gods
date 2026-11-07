// Pure frozen v48 overlay geometry oracle for authored widths and canvas sizes.
import fs from 'node:fs';import path from 'node:path';import{pathToFileURL}from'node:url';
const root=path.resolve(process.argv[2]??'../release-v48/source/call-of-the-corpse-gods');
const{packOverlayPlacement,enemyHoverPlacement}=await import(pathToFileURL(path.join(root,'dist/src/ui-overlays.js')));
const packs=[];for(const [x,y,w,h]of[[0,119,985,603],[0,271,768,451],[0,51,1080,671],[20,100,600,400],[0,300,700,350],[12,12,1200,900]]){const result=packOverlayPlacement({left:x,top:y,right:x+w,bottom:y+h});packs.push({map:[x,y,w,h],expected:[result.left,result.top,result.width,result.height]});}
const hovers=[];for(const width of[240,520,800])for(const height of[80,400,1000])for(const tile of[[10,10,30,30],[700,680,720,700],[300,350,320,370],[900,200,920,220]]){const viewport=[768,800];const result=enemyHoverPlacement({left:tile[0],top:tile[1],right:tile[2],bottom:tile[3]},{width,height},{width:viewport[0],height:viewport[1]});hovers.push({tile,width,height,viewport,expected:[result.left,result.top,result.width,Math.min(height,result.maxHeight)]});}
fs.writeFileSync('crates/corpse-desktop/assets/appearance-modal-parity-v48.json',JSON.stringify({packs,hovers})+'\n');console.log(`${packs.length} source Pack placements, ${hovers.length} authored hover placements`);
