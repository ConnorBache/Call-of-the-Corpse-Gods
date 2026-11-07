# Native integrated preview verification — 25 October 2026

## Current verified checkpoint

This revision 31 integrated preview includes the seeded floor 1–40 playtester,
visual-only Flesh/blood treatment, persistent floor 1 blue Ω entrance, the
48-row physical-stat overlay, and the Administrator/collapse/escape finale.
Original authored bestiary content remains **79 pages with 81 embedded art
plates**. Current actor stats are shown without rewriting authored histories.
Normal page color follows first appearance, independently of combat reference
floors, with continuous floor 1 parchment → floor 25 red → floor 40 black anchors.
The last normal page first appears at floor 38 and is therefore near-black.

## Final validation

- Full workspace test gate: **484 passed, zero failed, one ignored timing-only
  benchmark**, across 107 test groups. No filtered tests. The run completed in
  795.97 seconds with optimization level 1, debug assertions enabled and overflow
  checks enabled. All 992 captured source files were unchanged during execution.
- The complete affected desktop suite separately passed 226 tests, zero failed,
  with the same one timing-only benchmark ignored. The native arena example's
  six tests also passed.
- The first aggregate exposed a stale test assumption that every tuning key
  has a separate bestiary page. The corrected test explicitly covers the legacy
  `ichor-fish` alias without adding a duplicate page. All real pages compare all
  eight visible stats. The alias compares its six overlay-owned combat values
  while preserving authored Perception 40 versus the canonical page's 45 and its
  original Energy. No production stats were normalized to satisfy the test.
- Portable stationary validation reproduced the checked-in CSV byte-for-byte:
  23 species, 138 cases and 7,640,000 seeded fights. A separate fresh native smoke
  run executed 36 scenarios and 1,152 trials, including every native-dispatch
  tuning row, the legacy alias and special terrain/paired scenarios. There were
  no execution errors. Phoenix grass scenarios and a Bombardier approach-grass
  trial remained explicitly censored, not completed encounter-cost estimates.
- All 79 pages were captured from the final native software renderer at 1280×800.
  Page order, solid visual-depth colors, current stats and exact authored/display
  text were retained in the separate QA artifact. Following the test-only fix,
  every recaptured PNG was byte-for-byte identical to its prior reviewed image.
  Source and executable hashes remained stable during both capture batches.
- This validation includes existing revision 29/30 differential traces and
  generated-state tests, plus new playtester origin/replay/high-score guards,
  entrance generation, actual boss death, collapse layers, offscreen/fall
  cohorts, full fixture staircase ascent, save/load, Revert and See the Future.
  Passing these cases is not proof of every possible build or seed combination.

The final test command was:

```sh
CARGO_INCREMENTAL=0 \
CARGO_PROFILE_TEST_OPT_LEVEL=1 \
CARGO_PROFILE_TEST_DEBUG=0 \
CARGO_PROFILE_TEST_DEBUG_ASSERTIONS=true \
CARGO_PROFILE_TEST_OVERFLOW_CHECKS=true \
sh tools/cargo.sh test --workspace --locked
```

`tools/cargo.sh` selects the workspace's isolated installed toolchain. In a
normal checkout with the pinned Rust toolchain installed, use `cargo` in place
of `sh tools/cargo.sh`; no bundled SDK is required by the source archive.

## Platform and distribution status

- Linux x86-64: native code and tests executed, including actual software-renderer
  screenshots. Earlier real SDL keyboard/mouse acceptance is preserved in the
  linked historical record; it is not silently represented as a new Windows test.
- Windows x86-64: the final release was cross-compiled successfully with pinned
  Rust 1.99.1 and the restored MinGW/SDL toolchain. **It has not been run on
  Windows and is unsigned.** Display-driver behavior is not certified by Linux
  testing. Packaging strips a copy, retaining the original build for diagnosis.
- The Windows ZIP includes SDL2.dll, asset/font notices, Brogue notices and Rust
  dependency notices. Extract the entire ZIP and keep SDL2.dll beside the EXE.
  All 81 art plates are embedded; no loose-art download or setup script is needed.
- The source ZIP preserves PNG masters, development fixtures, deterministic
  validators and existing third-party notices. Build targets, SDK/toolchain
  installations and separate QA screenshots are excluded.

## Important scope and limitations

The playtester simulates full collection of prior ordinary generated hearts and
actual seeded loot. It excludes target-floor and rare/Godbeast rewards, preserves
synthetic provenance through persistence/time travel, and cannot enter normal
high scores. It is not a played-through historical run.

The physical overlay is role-specific and does not declare all 79 creatures or
all powers globally balanced. Collection sensitivity, incomplete/censored
special encounters, terrain and group assumptions are documented in the
[physical encounter report](docs/physical-encounter-balance-v31.md). Cosmetics
add no blood-dependent gameplay mechanics.

The collapse advances one eight-neighbor layer every two normal world turns,
through walls, and propagates upward through captured stairs. Stair propagation
is a disclosed provisional design assumption. Flight is preserved; a consumed
route is not silently replaced with a fallback ending. Actual falls end the run
with the exact recorded cause; the intact floor 1 blue Ω entrance wins only after
boss defeat. See [finale rules](docs/native-flesh-finale.md).

Generated escape pacing is **not guaranteed on every seed/build**. A cheap
read-only check of 92 existing generated floor pairs found three Mountain floors
where an optimistic bare shortest walk exceeded local wave travel time, by 1–8
turns. Deadline-aware path checks found 13 of 92 needing some inherited head start,
at most 10 turns. This excludes revised floor 1, treats doors as open and ignores
enemies/hazards. It identifies local cutoff risk with insufficient lead, not a
proof that an entire ascent is impossible: lead from lower floors, boss death
position, movement powers and tactical decisions matter. The successful 40→1
fixture test is functional coverage, not universal generated-route proof.

Unsupported custom/import states remain explicit admission boundaries; see
[admission gaps](docs/native-engine-admission-gaps.md) and the subsystem coverage
documents. Benchmark numbers are not end-to-end FPS or Windows speed guarantees.
The owner has not selected a project-wide open-source license; existing
third-party licenses remain applicable. No public publication was performed.

## Historical evidence

Earlier accepted native baselines, real SDL interaction checks and older
presentation/regression counts are preserved separately in
[the historical verification record](docs/native-verification-history-20261020.md).
They are not the current integrated gate above.
