#!/usr/bin/env python3
"""Read-only O(total source bytes) verification of the logical imported R51 tree."""
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PROVENANCE = json.loads((ROOT / 'docs/git-import-provenance.json').read_text())
FIXTURE = 'fixtures/mini-me-starter-combinations-v48.json'
APPENDED = {'README.md', 'docs/complexity-ledger.md'}

def digest(paths, limit=None):
    value = hashlib.sha256()
    count = 0
    for path in paths:
        with path.open('rb') as stream:
            while limit is None or count < limit:
                block = stream.read(1048576 if limit is None else min(1048576, limit - count))
                if not block:
                    break
                value.update(block)
                count += len(block)
    return count, value.hexdigest()

for row in PROVENANCE['original_files']:
    relative = row['path']
    path = ROOT / relative
    if relative == FIXTURE:
        paths = [ROOT / (FIXTURE + '.part1'), ROOT / (FIXTURE + '.part2')]
        size, actual = digest(paths)
        if path.exists():
            assert digest([path]) == (row['bytes'], row['sha256']), 'restored fixture differs'
    elif relative in APPENDED:
        size, actual = digest([path], row['bytes'])
        assert path.stat().st_size >= row['bytes'], relative
    else:
        assert path.is_file() and not path.is_symlink(), relative
        size, actual = digest([path])
    assert (size, actual) == (row['bytes'], row['sha256']), relative

manifest = json.loads((ROOT / 'fixtures/mini-me-starter-combinations-v48.manifest.json').read_text())
for part in manifest['parts']:
    assert digest([ROOT / 'fixtures' / part['path']]) == (part['bytes'], part['sha256']), part['path']
print(json.dumps({'status': 'verified', 'original_files': len(PROVENANCE['original_files']),
                  'original_bytes': PROVENANCE['original_byte_count'],
                  'fixture_bytes': 116885985, 'documentation_prefixes_preserved': sorted(APPENDED),
                  'scope': 'Byte preservation; not a new full game or Windows runtime test.'}, indent=2))
