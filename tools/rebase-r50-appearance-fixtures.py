#!/usr/bin/env python3
"""Derive R50/R51 expectation fixtures from frozen v48 cases, not runtime outputs.

Only the enumerated designer-approved default fields change. Explicit old profile input
is preserved, so import/customization regression coverage is not weakened.
"""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'crates/corpse-desktop/assets'
CHANGES = [
    ('vampire', 'glyph', 'v', 'V'),
    ('bear', 'glyph', 'u', 'B'),
    ('bastidion', 'glyph', '&', 'F'),
    ('short-saidri', 'color', '#e3c55b', '#f8d100'),
    ('pale:Birther', 'glyph', '(≬)', 'b'),
    ('long-saidri', 'color', '#75cb9d', '#8a5a32'),
    ('chimera', 'color', '#D99270', '#ddb435'),
    ('sphinx', 'color', '#E9CF78', '#ffd34a'),
    ('yeti', 'glyph', 'A', 'Y'),
    ('samara', 'color', '#85b382', '#8a5a32'),
    ('samara', 'glyph', 'Y', 'T'),
    ('banshee', 'color', '#A2E0DA', '#d600ff'),
    ('banshee', 'glyph', 'b', 'B'),
    ('hemogoblin-inventor', 'color', '#EAB567', '#d600ff'),
    ('hemogoblin-inventor', 'glyph', 'i', 'h'),
    ('saidri-guardian', 'color', '#ed9de0', '#f8d100'),
    ('pale:Greater Birther', 'glyph', '(≬)', 'B'),
    ('dire-bear', 'color', '#e0a368', '#000000'),
    ('dire-bear', 'glyph', 'U', 'B'),
    ('blorb', 'color', '#b1a286', '#28703c'),
    ('blorb', 'glyph', 'a', 'b'),
]

def rebase(profile, explicit=None):
    for kind, field, old, new in CHANGES:
        if explicit and field in explicit.get('entities', {}).get('creatures', {}).get(kind, {}):
            continue
        entry = profile['entities']['creatures'][kind]
        assert entry[field] == old, (kind, field, entry[field])
        entry[field] = new
    # R51 presentation-only amulet glyph. Do not replace imported identities.
    for kind in ['Amulet', 'kind:amulet']:
        if explicit and 'glyph' in explicit.get('entities', {}).get('items', {}).get(kind, {}):
            continue
        entry = profile['entities']['items'][kind]
        assert entry['glyph'] == '"', (kind, entry['glyph'])
        entry['glyph'] = '?'

cases = json.loads((ASSETS / 'appearance-parity-v48.json').read_text())
for case in cases:
    if 'error' in case:
        continue
    rebase(case['output'], case['input'])
    if 'reset' in case:
        rebase(case['reset'], case['input'])
(ASSETS / 'appearance-parity-v50.json').write_text(json.dumps(cases, ensure_ascii=False, separators=(',', ':')) + '\n')

sequences = json.loads((ASSETS / 'appearance-store-parity-v48.json').read_text())
for sequence in sequences:
    for case in sequence['steps']:
        action = case['action']
        # Original operations touch player/global/terrain only. Reject future
        # fixture drift rather than accidentally overwriting creature edits.
        assert not (len(action) > 1 and isinstance(action[1], str) and action[1].startswith(('entities.creatures.', 'entities.items.')))
        if action[0] == 'importJSON':
            assert 'entities' not in json.loads(action[1])
        rebase(case['appearance'])
        rebase(case['edited'])
(ASSETS / 'appearance-store-parity-v50.json').write_text(json.dumps(sequences, ensure_ascii=False, separators=(',', ':')) + '\n')
print(f'Derived R50/R51 fixtures with {len(CHANGES) + 2} explicit default changes; v48 fixtures preserved.')
