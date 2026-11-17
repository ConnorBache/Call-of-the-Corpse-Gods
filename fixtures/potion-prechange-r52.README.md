# Pre-amendment potion replay provenance

`potion-prechange-r52.json` was generated before the potion changes were edited,
from the post-amulet-amendment source tree (the replay explicitly records
`amuletRulesFrom: 0`). The exact pre-potion source hashes were:

- `crates/corpse-core/src/revision50_potions.rs`: `9f8dc56b45e41a524cb418db2cb4c3a2c229369ea9f51141c9a6ee619bd11726`
- `crates/corpse-core/src/equipment_generation.rs`: `51ff084d1e7d123811918ce19a514e18651973b70d56e134ad2926e937f32c7b`

The producer is retained in `tools/generate-potion-baseline.rs`; copy it into
that historical checkout's core examples and run it there. Do not regenerate
with amended code. The fixture records full canonical initial/final state
FNV-1a64 regression fingerprints, a seed, and three wait commands. Tests omit
only the newly introduced top-level `potionRulesFrom` field before comparing.
The original amulet fixture remains unchanged and tests both legacy policies.
