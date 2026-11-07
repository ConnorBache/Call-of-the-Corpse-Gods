#!/usr/bin/env python3
"""Losslessly encode UI resources; preserve every decoded RGBA byte, including hidden RGB."""
import hashlib
import json
from pathlib import Path
from PIL import Image, features, __version__

ASSETS = Path(__file__).resolve().parents[1] / 'crates/corpse-desktop/assets'
NAMES = ('menu-logo', 'bestiary-ink-splash-broad', 'BrogueCE-ASCII', 'window-icon')
SETTINGS = dict(lossless=True, exact=True, quality=100, method=6)

def sha(data):
    return hashlib.sha256(data).hexdigest()

def main():
    output = ASSETS / 'ui-runtime'
    output.mkdir(exist_ok=True)
    rows = []
    for name in NAMES:
        source = ASSETS / (name + '.png')
        target = output / (name + '.webp')
        original = Image.open(source).convert('RGBA')
        original.save(target, **SETTINGS)
        decoded = Image.open(target).convert('RGBA')
        if decoded.size != original.size or decoded.tobytes() != original.tobytes():
            target.unlink()
            raise RuntimeError('Lossless RGBA verification failed: ' + name)
        rows.append(dict(name=name, width=original.width, height=original.height,
                         source_bytes=source.stat().st_size, webp_bytes=target.stat().st_size,
                         source_sha256=sha(source.read_bytes()), webp_sha256=sha(target.read_bytes()),
                         rgba_sha256=sha(original.tobytes()), rgba_exact=True))
    (ASSETS / 'ui-runtime-manifest.json').write_text(json.dumps(dict(
        pillow=__version__, libwebp=features.version('webp'), settings=SETTINGS, assets=rows), indent=2) + '\n')

if __name__ == '__main__':
    main()
