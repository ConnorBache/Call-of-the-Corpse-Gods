# Controlled pre-rename R50 compatibility fixtures

These files were produced by the exact preserved PRE-RENAME R50 source, not the currently edited R50 code. This is a controlled compatibility fixture, not a player campaign, full-playthrough result, seed-only replay, or visual verification.

## Source identity

Preserved source: `../r50-pre-creature-names`.

`SOURCE-SNAPSHOT.json` SHA-256: `f19312126dd8d9cfac9eb449f9bf543b65ee6c41e53780b81ee4e9433e25b59c`.

All 785 preserved text inputs matched their bytes and SHA-256 entries in the 1,372-file source snapshot both before and after the build. Rust compiler dependency-info inputs were checked individually against that snapshot. See `source-verification.json` and `build-provenance.json`. Neither preserved source nor `r50-work` was edited. The separate build directory was `../r50-compat-target`; no command used `r50-optimized-target`.

The external helper compiled the backup `corpse-core` through a path dependency and included the backup desktop `session_store.rs` and its modules unchanged. All 17 registry dependencies have the same versions, registry sources and checksums as the original lockfile. The external helper owns its own manifest and lockfile. Only already cached dependencies were used, with offline locked Cargo execution, official existing Rust 1.99.1, one job and no installation. Toolchain details are in `toolchain.txt`.

## Construction

1. Generated a valid R50 start using the unmodified official `startup::create_run_revision`, seed `0-5-11:revision50-protocol-resume`, mode Unblessed, simulation revision 50.
2. Added 11 actors using the backup's `population::create_actor_revision`. Actors occupy unoccupied floor-1 floor cells at least 15 tiles from the player. Their IDs are 247–257. No actor template fields were altered. Existing generated geometry, actors and items were retained.
3. Explicitly granted See The Future, initialized its official state/reserve, assigned a fixture run ID, and marked seed-only replay ancestry unknown. This controlled setup is explicitly not a claim of normal campaign provenance.
4. Dispatched an actual See The Future command to capture the nested anchor.
5. Used unchanged desktop Autosave/Kind::Fixture to write the native incremental checkpoint.
6. Dispatched two actual Wait commands and used unchanged Autosave to append their real command receipts. The recording retained the exact 3,874,152-byte prefix and appended 537 bytes; tick advanced from 0 to 2. The player remained playing and all 11 inserted creatures remained alive.
7. Decoded the recording through the backup desktop loader and verified byte-exact equality with the original final engine save. Loaded that final save, dispatched the actual See The Future restore command, verified the anchor was consumed, and validated the restored run.

## Files

- `generated-start.json`: Unmodified generated R50 startup before controlled setup.
- `controlled-setup-run.json`: Controlled actors and granted Future before activation.
- `old-run-with-future.json`: Serialized old Run with authentic captured Future anchor.
- `old-native-checkpoint.json`: Desktop native save envelope with fixture session classification.
- `old-recording-checkpoint-prefix.bin`: Exact initial journal bytes before appended commands.
- `old-incremental-recording.json`: Native incremental recording with the Future-bearing checkpoint and two real Wait tail commands. Despite the `.json` extension this is the game's framed native recording format, not a standalone JSON document.
- `old-final-run.json`: Old-source reference result after checkpoint plus Wait tail replay.
- `old-restored-future-run.json`: Old-source reference after restoring Future from the final run.
- `fixture-report.json`: Exact roster, setup, ticks and completed checks.
- `SHA256SUMS`: Artifact checksums.

The old authored larger Pale names are `Greater Pale Biter` and `Greater Pale Breather`; their stable kinds are `pale:Greater Biter` and `pale:Greater Breather`. They must not be confused with expected new display names.

## Reproduce

From the workspace root, run:

```
./r50-prerename-fixture-helper/reproduce.sh /absolute/path/to/a-new-output-directory
```

The script re-verifies all preserved source hashes and cached dependency identities, then runs:

```
cargo +1.99.1 build --offline --locked -j1 --manifest-path r50-prerename-fixture-helper/Cargo.toml
r50-compat-target/debug/r50-prerename-fixture-helper OUTPUT_DIRECTORY
```

It requires a new output directory to avoid replacing existing evidence. The helper only writes its chosen output directory. No screenshot or graphical startup was performed. New-source acceptance must be tested separately against these immutable old-source bytes; cosmetic display changes should not rewrite canonical actor names or mechanics.
