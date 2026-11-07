/** Read-only extraction from the frozen v48 source build. */
import fs from 'node:fs';
import { pathToFileURL } from 'node:url';
import path from 'node:path';
const root = path.resolve(process.argv[2] ?? '../release-v48/source/call-of-the-corpse-gods');
const { DEFAULT_APPEARANCE, APPEARANCE_CONTROLS, parseAppearance, setAppearanceValue, resetAppearanceValue } = await import(pathToFileURL(path.join(root, 'dist/src/appearance-config.js')));
const dest = 'crates/corpse-desktop/assets';
fs.writeFileSync(`${dest}/appearance-controls-v48.json`, JSON.stringify(APPEARANCE_CONTROLS, null, 2) + '\n');
// Keep the existing default fixture and detect accidental source drift.
if (JSON.stringify(JSON.parse(fs.readFileSync(`${dest}/appearance-defaults.json`, 'utf8'))) !== JSON.stringify(DEFAULT_APPEARANCE)) throw new Error('Appearance defaults drift');
const cases = [
  {}, { schemaVersion: 1, global: { exposure: 1.7 }, futureExtension: { x: [1, 'kept', true] } },
  { schemaVersion: 2 }, { global: { exposure: 5 } }, { global: { minimumBrightness: .8, maximumBrightness: .6 } },
  { terrain: { wall: { color: '#ab12ff', glyph: 'W' } } }, { terrain: { wall: { glyph: '☃' } } },
  { typography: { map: { fontFamily: 'IBM Plex Mono, monospace' } } },
  { typography: { map: { fontFamily: 'bad; font: 9px' } } },
  JSON.parse('{"__proto__": {"x": 1}}'), { ui: { layout: { leftSidebarWidth: 999 } } },
  { terrain: { 'gas:caustic': { glyph: 'inert legacy field' } } },
  { lighting: { blending: 'screen', falloffCurve: 'quadratic' } },
];
const fixtures = cases.map(input => {try {return {input, output:parseAppearance(input)}}catch(error){return {input, error: error.message}}});
const edits = [
 ['entities.objects.player.color', '#aabbcc'], ['entities.objects.player.brightness', 2],
 ['terrain.wall.glyph', 'W'], ['typography.map.fontWeight', 700],
];
for (const [property, value] of edits) {
 const output = setAppearanceValue(DEFAULT_APPEARANCE, property, value);
 fixtures.push({input: DEFAULT_APPEARANCE, edit: {path:property, value}, output, reset:resetAppearanceValue(output, property)});
}
fs.writeFileSync(`${dest}/appearance-parity-v48.json`, JSON.stringify(fixtures) + '\n');
console.log(`${APPEARANCE_CONTROLS.length} controls, ${fixtures.length} reference cases`);
const {AppearanceStore}=await import(pathToFileURL(path.join(root,'dist/src/appearance-store.js')));
const sequences=[
 [['openEditor'],['beginGesture'],['setValue','global.exposure',1.2],['setValue','global.exposure',1.4],['endGesture'],['undo'],['redo'],['setComparison','original'],['setComparison','edited'],['closeEditor']],
 [['setValue','entities.objects.player.color','#aabbcc'],['setValue','entities.objects.player.brightness',2],['resetProperty','entities.objects.player.color'],['resetGroup','entities.objects.player'],['undo'],['redo']],
 [['setValue','global.exposure',1.8],['savePreset','Warm'],['setDefaultPreset','preset-1'],['duplicatePreset','preset-1','Evening'],['renamePreset','preset-2','Dusk'],['setValue','global.exposure',2.2],['updatePreset','preset-1'],['loadPreset','preset-2'],['setComparison','preset-1'],['deletePreset','preset-1',true],['reload']],
 [['setValue','global.exposure',1.5],['previewSet','global.exposure',2],['setComparison','original'],['setComparison','edited'],['cancelPreview'],['previewSet','global.exposure',2.5],['commitPreview'],['undo'],['redo'],['closeEditor']],
 [['importJSON','{"presetName":"Future","future":{"value":[1,true,"kept"]},"global":{"exposure":2.5}}'],['resetAll'],['undo'],['redo']],
];
const storeFixtures=sequences.map(actions=>{
 let saved=null;const storage={getItem:()=>saved,setItem:(_,value)=>{saved=value;}};let store=new AppearanceStore(storage);
 const steps=actions.map(action=>{
  const [method,...args]=action;
  if(method==='reload')store=new AppearanceStore(storage);
  else if(method==='previewSet')store.preview(setAppearanceValue(store.getAppearance(),...args));
  else store[method](...args);
  const s=store.getStatus();
  return {action,appearance:store.getAppearance(),edited:store.getEditedAppearance(),status:{dirty:s.dirty,previewing:s.previewing,canUndo:s.canUndo,canRedo:s.canRedo,comparison:s.comparison,savedPresetId:s.savedPresetId,defaultPresetId:s.defaultPresetId}};
 });return {steps};
});
fs.writeFileSync(`${dest}/appearance-store-parity-v48.json`,JSON.stringify(storeFixtures)+'\n');
console.log(`${storeFixtures.reduce((n,c)=>n+c.steps.length,0)} store transition fixtures`);
