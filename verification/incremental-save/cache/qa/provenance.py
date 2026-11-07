#!/usr/bin/env python3
"""O(source and binary bytes) provenance hashing outside benchmark windows."""
from pathlib import Path
import hashlib,json
root=Path(__file__).resolve().parents[1]
files=[root/'baseline/crates/corpse-core/src/temporal.rs',root/'source/crates/corpse-core/src/temporal.rs',root/'source/crates/corpse-desktop/examples/cache_opening_bench.rs',root/'baseline/crates/corpse-desktop/examples/cache_opening_bench.rs']+sorted((root/'bin').glob('*'))
rows=[{'path':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size} for p in files]
(root/'reports/provenance.json').write_text(json.dumps(rows,indent=2))
