# Revision 34 verification receipt

**Release gate passed for the current source: 753 tests passed, 0 failed, 1 existing ignored test, across 136 target harnesses.**
No Windows runtime verification is claimed. Native Linux SDL offscreen test scenes
are actual application-renderer captures, not generated artwork or desktop-session
screenshots. The original canonical r33 source was not overwritten during work.

## Reproducibility

- Baseline: `corpse-gods-native-r33-features-source.zip`, canonical source version 4.
- Baseline SHA-256: `a3891f912f6646790d284e2396f2453fce7a31136bc3b5ef5133dbd892c36100`.
- Rust/Cargo: pinned official 1.99.1; Cargo.lock unchanged.
- Production build-input freeze: 911 files, SHA-256
  `6b38787754521be07384f5642690b7c1b4bd169846ca108d18ab2284762c7c85`.
  Digest is over `<sha256>  <relative path>\n` lines in the JSON manifest record order (Python pathlib component ordering), for all files under
  crates/tools plus Cargo.toml, Cargo.lock and rust-toolchain.toml. Public docs
  and the final receipt are outside that code digest.
- Final source/test manifest after two approved test-only corrections:
  `d808403c8c5d0d556a4b694e85d6ac738288abc1d3b47547509794875ba86b66`.
  Only `ordinary_roster.rs` and `witch_skinwalker_v33.rs` tests differ: obsolete
  default-33 assertions now expect 34; 34 additionally joins ordinary-roster
  save/lazy-generation/replay coverage. Legacy33 temporal expectations remain intact. All production code/assets match the built executable. See
  `test-only-delta-r34.json` and `final-code-manifest-r34.json`.
- Preservation audit: 336 protected image/catalog/toolchain files byte-identical
  to r33; no baseline files missing. No new monster illustrations were requested
  for these Godbeasts; original plates and natural-history strings remain intact.

## Exact commands

```sh
sh tools/cargo.sh test --workspace --all-targets --locked --no-fail-fast
sh tools/cargo.sh test --workspace --doc --locked
sh tools/cargo.sh test -p corpse-core --test phantasmii_integration_r34 --locked
sh tools/cargo.sh test -p corpse-core --lib dream_plane -- --nocapture
```

The Windows package's cross-build receipt separately identifies compiler/SDK,
locked build command, PE imports, executable/DLL hashes, source digest and ZIP CRC.
A successful cross-build is not a Windows execution test.

## Focused evidence before the aggregate gate

- 16 independent Phantasmii integration tests: actual dagger/altar/stacked shards,
  early wake, unlock, save and fixture replay, Revert/Future with both layers,
  creation-plane Half Here, waking fire provenance, dream-only spell geometry,
  no Nightmare rewards, normal/haste stair cancellation, exactly-once terminal
  sleeping-body death, and gated synthetic controls.
- Ten corrupt nested-save cases reject through both canonical and compatible
  loaders: duplicate/missing floors, geometry, unknown terrain, truncated light,
  seen, fire and ownership arrays, and both transient context flags.
- 16 focused plane/power/monster regressions; existing power/monster traces and
  explosion presentation parity retained. Environment probes have explicit plane
  identity; opposite-plane projection cannot consume the body's shared hit pool.
- 23 generation/item/Godbeast/DEBUG tests: save/load, temporal reward restoration,
  visited offscreen pulse clock, once-only Blessed, exact seven-event scheduling,
  all-water placement, ordinary budget preservation and mask anchor/cooldown.
- Five native-plane vision regressions plus existing 20-scene/entrance parity.
- Actual native-renderer screenshots: dreamwalking, flicker, Nightmare overlap,
  Epidite subscript, Ahvamitt inspection/Teleport. Controlled test scenes are
  deliberately not evidence of an entire naturally played 40-floor run.

## Final results

The full no-fail-fast sweep completed with 751 passes, two obsolete default33
assertion failures, and one existing ignored manual test. No other target failed.
After correcting those tests only, ordinary_roster passed 6/6 (including actual34
save/load, lazy generation and replay restart), and witch_skinwalker_v33 passed
11/11. Replacing those two obsolete outcomes yields **753 passed, 0 failed,
1 ignored** across **136 harnesses**, including zero-test example harnesses.
This is a consolidated gate, not a claim that the original raw sweep exited zero.
The raw exit was 101; both corrected reruns and the separate doctest gate exited 0.
The complete raw results, logs and exact two-file delta are retained.

Windows x64 locked release cross-build succeeded using official Rust1.99.1,
Debian GCC14 POSIX/MinGW-w64 and SDL2 2.32.10. All production inputs match the
build freeze. Delivery executable: 97,312,495 bytes, SHA-256
`c1fb3f610ca1edc8925f8b061f21f381f5a05dfbc5e157955949f18ea6be2d74`.
Static checks verify required SDL2.dll, all 84 RGBA plates, seven icon images and
manifest. The executable is unsigned. **Windows launch/input/audio/save runtime
was not tested.** See the package's cross-build and CRC/hash receipts.

Six images accompany the source: five controlled UI scenes and one real generated
Epidite floor10 shown through DEBUG with full reveal, repositioned player and no
simulation turns. They are native Linux/offscreen renderer captures, not a
Windows playthrough or an all-40-floor balance certification.
