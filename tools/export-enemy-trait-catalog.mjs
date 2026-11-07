import fs from 'node:fs';import vm from 'node:vm';
import {ENEMY_ABILITIES} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/enemy-data.js';
import {BESTIARY,EVENT_CREATURES,ZILLENNIA_COLOR} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/bestiary.js';
import {ZILLENNIA_QUALITIES} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/zillennia-qualities.js';
import {PALE_GLYPHS,PALE_COLORS} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/creature-presentation-catalog.js';
const source=fs.readFileSync(new URL('../../release-v48/source/call-of-the-corpse-gods/src/enemy-data.ts',import.meta.url),'utf8');
const literal=name=>vm.runInNewContext('('+source.match(new RegExp(`const ${name}[^=]+=\\s*Object.freeze\\((\\{[\\s\\S]*?\\})\\)`))[1]+')');
const data={abilities:ENEMY_ABILITIES,definitions:[...BESTIARY,...EVENT_CREATURES].map(({id,name,glyph,color,abilities,bleeds})=>({id,name,glyph,color,abilities,bleeds})),zillenniaNames:ZILLENNIA_QUALITIES.map(q=>q.name),zillenniaColor:ZILLENNIA_COLOR,paleGlyphs:PALE_GLYPHS,paleColors:PALE_COLORS,legacyColors:literal('LEGACY_ENEMY_COLORS'),legacyGlyphs:literal('LEGACY_ENEMY_GLYPHS')};
const path=new URL('../crates/corpse-core/catalog/enemy-traits-v48.json',import.meta.url);fs.writeFileSync(path,JSON.stringify(data));console.log(data.definitions.length,Object.keys(data.abilities).length);
