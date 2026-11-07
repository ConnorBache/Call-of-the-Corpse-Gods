#!/usr/bin/env python3
"""Prepare reviewed RGBA distribution and platform icon formats; never overwrite PNG masters.
Build-time maintainer tool only. Requires the pinned offline pipeline dependencies; players need none.
"""
import hashlib, io, json, pathlib, struct
from PIL import Image, features
root = pathlib.Path(__file__).resolve().parents[1]
assets = root / 'crates/corpse-desktop/assets'
# This entry point must never re-encode opaque masters as lossy runtime artwork.
# Build uses explicit review approvals and exact frozen native legacy references.
import subprocess, sys
subprocess.run([sys.executable, str(root/'tools/creature-alpha/pipeline.py'), 'build'], check=True)
subprocess.run([sys.executable, str(root/'tools/creature-alpha/pipeline.py'), 'verify'], check=True)
# ICO is a platform representation of the existing logo, not newly authored art.
# Keep its full aspect ratio inside a transparent square; nearest preserves pixel edges.
logo = Image.open(assets / 'menu-logo.png').convert('RGBA')
frames = []
for size in [16, 24, 32, 48, 64, 128, 256]:
    frame = logo.copy()
    frame.thumbnail((size, size), Image.Resampling.NEAREST)
    square = Image.new('RGBA', (size, size))
    square.alpha_composite(frame, ((size-frame.width)//2, (size-frame.height)//2))
    if size == 256: square.save(assets / 'window-icon.png')
    encoded = io.BytesIO(); square.save(encoded, 'PNG')
    frames.append((size, encoded.getvalue()))
offset = 6 + 16 * len(frames)
header = struct.pack('<HHH', 0, 1, len(frames))
for size, data in frames:
    header += struct.pack('<BBBBHHII', size % 256, size % 256, 0, 0, 1, 32, len(data), offset)
    offset += len(data)
(assets / 'corpse-gods.ico').write_bytes(header + b''.join(data for _, data in frames))
print(sum(p.stat().st_size for p in (assets/'creatures-rgba').glob('*.png')), 'embedded RGBA creature bytes')
