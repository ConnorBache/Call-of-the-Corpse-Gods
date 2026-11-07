import {readFileSync,writeFileSync} from 'node:fs';
import {tickEnvironment} from '../../release-v48/source/call-of-the-corpse-gods/dist/src/environment.js';
const initial=JSON.parse(readFileSync(new URL('../fixtures/avatar-room-v1.json',import.meta.url),'utf8'));
const expected=structuredClone(initial);tickEnvironment(expected);
writeFileSync(new URL('../crates/corpse-core/tests/fixtures/environment-static-v48.json',import.meta.url),JSON.stringify({initial,expected}));
