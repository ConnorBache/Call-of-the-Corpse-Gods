// Development-only source geometry oracle.
import fs from 'node:fs';
import {enemyHoverPlacement} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/ui-overlays.js';
const cases=[];
for(const viewport of [{width:981,height:800},{width:1280,height:800},{width:320,height:240},{width:80,height:40}])for(const point of [[0,0],[12,131],[400,400],[900,690],[1280,800]])for(const height of [20,300,620,900]){
 const tile={left:point[0],top:point[1],right:point[0]+12,bottom:point[1]+20};const panel={width:520,height};cases.push({tile,viewport,height,expected:enemyHoverPlacement(tile,panel,viewport)});
}
fs.writeFileSync(new URL('../fixtures/hover-ui-v48.json',import.meta.url),JSON.stringify(cases));console.log(`Exported ${cases.length} hover placements.`);
