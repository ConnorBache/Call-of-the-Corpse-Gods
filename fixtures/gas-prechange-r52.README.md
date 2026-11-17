# Pre-amendment gas replay provenance

`gas-prechange-r52.json` was captured from the compiled post-ring/pre-gas source
before five-turn gas refresh changes were edited. Its replay records the three
prior amendment markers at zero, independently of the new gas policy.
Pre-change `crates/corpse-core/src/environment.rs` SHA-256:
`2e10e1a7d45aaf7819fc01c41ee8e577625ea4ac28193e8b07e78be7695f0004`.

The producer is retained in `tools/generate-gas-baseline.rs`; compile it against
that historical core library. Do not regenerate using amended source. The
fixture records full canonical initial/final state FNV-1a64 regression
fingerprints, a seed and three wait commands. Tests omit only the newly
introduced top-level `gasRulesFrom` field before comparing. Earlier amulet,
potion and ring fixtures retain their original provenance and continue testing
their own legacy policies independently.
