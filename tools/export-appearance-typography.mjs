// Frozen browser CSS override oracle, no DOM or simulation required.
import fs from 'node:fs';import path from 'node:path';import {pathToFileURL} from 'node:url';
const root=path.resolve(process.argv[2]??'../release-v48/source/call-of-the-corpse-gods');const mod=n=>import(pathToFileURL(path.join(root,`dist/src/${n}.js`)));
const {parseAppearance}=await mod('appearance-config');const {appearanceStyles}=await mod('ui-interface-appearance');
const roles=['primary','secondary','headings','chronicle','ability','tooltips','inventory','hotkeys'];
const selectors={primary:'body',secondary:'.ability-summary',headings:'h2',chronicle:'#log',ability:'.ability-row .ability-action',tooltips:'#hover-tooltip',inventory:'#inventory',hotkeys:'kbd'};
const cases=[];
function add(name,appearance){const config=parseAppearance(appearance),css=appearanceStyles(config);const expected={};for(const role of roles){const declarations={};for(const line of css.split('\n')){const brace=line.indexOf('{');if(line.slice(0,brace).split(',').includes(selectors[role]+':not(.appearance-editor):not(.appearance-editor *)')){const [property,value]=line.slice(brace+1,-1).replace(/ !important$/,'').split(':');if(['font-family','font-size','font-weight','letter-spacing','line-height'].includes(property))declarations[property]=value;}}expected[role]=declarations;}const variables={};for(const match of css.matchAll(/\{(--chronicle-(?:line|height)):([^}]+) !important\}/g))variables[match[1]]=Number(match[2].replace(/px$/,''));cases.push({name,appearance,expected,variables,css});}
add('default',{});
for(const role of roles){for(const [field,value]of Object.entries({fontFamily:'"Inconsolata", monospace',fontSize:23,fontWeight:700,letterSpacing:2,lineHeight:1.8})){add(`${role}-${field}`,{typography:{[role]:{[field]:value}}});}add(`${role}-combined`,{typography:{[role]:{fontFamily:'"Inconsolata", monospace',fontSize:23,fontWeight:700,letterSpacing:2,lineHeight:1.8}}});}
add('chronicle-explicit-track',{typography:{chronicle:{fontSize:23,lineHeight:1.8}},ui:{layout:{chronicleHeight:200}}});
add('chronicle-weight-only',{typography:{chronicle:{fontWeight:700}}});add('chronicle-track-only',{ui:{layout:{chronicleHeight:200}}});add('legacy-inert',{typography:{primary:{glyphScale:4,alignX:2,alignY:2,tileWidth:40,tileHeight:40}},ui:{layout:{leftSidebarWidth:400}}});
fs.writeFileSync('crates/corpse-desktop/assets/appearance-typography-parity-v48.json',JSON.stringify({cases})+'\n');console.log(`${cases.length} source UI override cases`);
