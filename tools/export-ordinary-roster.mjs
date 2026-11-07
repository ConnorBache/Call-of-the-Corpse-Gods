// Read-only extraction of baseline eligibility and family order from the frozen reference.
import fs from 'node:fs';
const root = new URL('../../release-v48/source/call-of-the-corpse-gods/dist/src/', import.meta.url);
const { BESTIARY } = await import(new URL('bestiary.js', root));
const { ORDINARY_ROSTER_ORDER, ordinaryRosterKinds } = await import(new URL('monster-catalog.js', root));
const order = [...BESTIARY.map(d => d.id), ...ORDINARY_ROSTER_ORDER.filter(k => k.startsWith('pale:'))];
const rows = order.filter(id => ORDINARY_ROSTER_ORDER.includes(id)).map(id => {
  const depths = Array.from({length:40}, (_, i) => i + 1).filter(d => ordinaryRosterKinds(d).includes(id));
  return { id, family: BESTIARY.find(d => d.id === id)?.family ?? 'pale', first: depths[0], last: depths.at(-1) };
});
fs.writeFileSync(new URL('../crates/corpse-core/src/catalog/ordinary-roster-v48.json', import.meta.url), JSON.stringify(rows, null, 2) + '\n');
