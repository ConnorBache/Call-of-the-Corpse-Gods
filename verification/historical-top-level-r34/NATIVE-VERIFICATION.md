> Current revision 34 verification is recorded in [docs/verification-r34.md](docs/verification-r34.md). The revision 33 material below is retained as historical evidence, not a revision 34 result.

# Native revision 33 — integrated verification

Verified 26 October 2026. New descents use **simulation revision 33**. This is a
playable native preview, not a claim that every seed, build or encounter is balanced.
The Bestiary contains **82 pages and 84 embedded RGBA creature plates**.

## Accepted final gate

- **659 passed, zero failed, 1 ignored**: locked workspace all-targets
  **659 passed / 1 ignored**; the documentation-test command
  also passed (**0 doctests defined**).
  Counts include only individual test-result lines, never summary lines. The sole
  ignored test is the explicitly manual animation timing benchmark.
- Pinned Rust **1.99.1**; optimization **2**, debug info **0**, debug assertions
  **on**, overflow checks **on**, incremental compilation **off**.
- All **1124** captured code, assets, catalogs, fixtures, build/test tools
  and profile inputs were unchanged throughout the accepted gate. Canonical
  before/after manifest SHA256:
  `7bfb2c344ed064026e9b7d17fe0703a2dac4680118e75d2afe2df691b54e66af`.
- [Machine-readable gate receipt](verification/integration-r33/final-gate-receipt.json),
  [before manifest](verification/integration-r33/frozen-inputs-before.json),
  [after manifest](verification/integration-r33/frozen-inputs-after.json),
  [all-targets log](verification/integration-r33/final-workspace-all-targets.log),
  [doc-test log](verification/integration-r33/final-workspace-doc.log).
- The first aggregate stopped with two stale revision-32 test expectations in
  `ordinary_roster.rs` (289 passed, two failed). The corrected test compares the
  current default/manual with revision 33 and explicitly checks save/load, lazy
  generation and replay for revisions 29–33. Its six focused tests passed before
  the entire gate was rerun. **No runtime or manual data changed for this fix.**
  [The test-only refreeze receipt](verification/integration-r33/test-only-refreeze.json)
  records the first single-file delta. A second checkpoint reached 598 passed,
  one failed and one ignored before exposing the same stale revision-32 comparison
  in the manual stat test; current pages were already correctly showing revision 33.
  Both failed checkpoints remain separately retained, not counted as final passes.

Reproduce the accepted profile:

```sh
export CARGO_INCREMENTAL=0 CARGO_PROFILE_TEST_OPT_LEVEL=2 CARGO_PROFILE_TEST_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG_ASSERTIONS=true CARGO_PROFILE_TEST_OVERFLOW_CHECKS=true
sh tools/cargo.sh test --workspace --locked --all-targets
sh tools/cargo.sh test --workspace --locked --doc
```

Release documentation is finalized **after** tests and is outside the compiled-input
freeze. [Document hashes](verification/integration-r33/release-document-hashes.json)
and the final package manifest record that separate step; this is not a claim that
all files remained unchanged after documentation and packaging.

## Integrated feature and regression coverage

The gate includes the versioned legacy fixtures and the new Witch/Night Crawler/
Skin Walker, dissolved blood and permanent mild water sources, Greater Breather
hallucinations, Screamer Accuracy penalties, root severing, NPC feeding, Nest/Queen/
Royal Honey, Borge/Shards, pocket habitats, flowers and deeper-Zillennia suites.
Cross-feature tests exercise actual combat/environment dispatch, visibility and
Half Here, source provenance, invalid actions, save/load, travel, Revert and future
branches. These are selected deterministic cases, not exhaustive combinations.

- Independent tests generate four complete 40-floor worlds, including a searched
  all-water Nest-window seed. They check reachable stairs/fixtures, the seven
  Godbeast floors 5–35, Administrator at 40, exactly one Nest/Queen/Honey,
  population/source invariants and serialized state. The separate fixed 32-world
  revision-33 census completed **32/32 with no substitutions or discarded failures**.
  Generation/heart collection is not a survival playthrough.
- DEBUG floor starts reconstruct actual eligible prior-floor initial hearts and
  ordinary loose/carried loot, preserve legal manual ability choices, and keep the
  normal **20-slot Pack** with a separate overflow stash. World Tree taps remain
  on their source floors; initial Royal Honey is available but not auto-consumed;
  unearned Borge/event prizes are not fabricated. The target floor is untouched.
- Independent review reproduced live stash/Revert item loss and a history-path
  error. The accepted revision synchronizes explicit DEBUG loadout changes with
  temporal history. Regressions cover take/put/equip, save/reload, pending haste,
  Revert/Future, and all twelve gods' legal offer queues. Preparation cannot be
  rewound into play. Normal Continue, autosave and high scores remain separate;
  explicit DEBUG exports use their own file. See [DEBUG v2](docs/debug-floor-testing-v33.md).
- A late source review and actual native bow-miss reproduction found that a
  Skin Walker at distance four still appeared disguised on the map but leaked its
  true name in combat narration. The accepted presentation guard shares the
  deterministic apparent identity without changing actual identity, stats or RNG;
  its regressions and the entire aggregate were rerun after the fix. The exact
  far-bow before/after state differs only in its live/archived narration. A separate
  12-case Night/Skin/Vampire native combat smoke preserved all non-narration state,
  RNG and resource outcomes. [The equivalence receipt](verification/integration-r33/presentation-combat-smoke-receipt.json)
  states the synthetic inputs; it is not additional balance evidence.
- Native-renderer captures cover Borge target/heart/altar/Shard details, feeding,
  Nest/permafrost/flowers, new creature pages and DEBUG setup/choices/stash/entry.
  They use the application's actual painters and generated states, not mockups.
  [Capture hashes](verification/integration-r33/screens-manifest.json) identify
  the independent Borge/world set. These checks do **not** establish new physical
  SDL input or Windows window behavior.

Subsystem contracts: [Witch and disguises](docs/witch-creatures-v33.md),
[environment/status hooks](docs/native-environment-hooks-r33.md),
[feeding](docs/native-creature-feeding-r33.md),
[Nest and pocket biomes](docs/nest-and-pocket-biomes-r33.md),
[Borge](docs/borge-event-r33.md), [Zillennia](docs/native-zillennia-ground-r33.md).
No declined full Dryad/Birther physiology, general guarding or food-chain ecology
is implied by these hooks.

## Art, lore and numerical provenance

[The art/lore audit](verification/integration-r33/art-lore-audit.json) confirms all
**79 original Natural History strings** and all **162 original master/runtime plate
files** match the frozen r32 baseline. Three new pages and three authored plates
bring the totals to 82/84; selected native captures were visually reviewed, including
the approved adult Skin Walker blur. This is not a claim that historical alpha-edge
review debt on every older plate has been resolved.

Accuracy/Dodge specializes 17 natural and two Witch-born profiles without changing
the global hit formula. The primary reference is **all_initial_unblessed**;
ordinary-only is a sensitivity track, not an equivalent baseline. The old Witch
Night/Skin checkpoint used ordinary-only, 128 trials cycling 127 profiles and a
different seed family; the accuracy comparison used 127 primary joint profiles.
[The reconciliation](verification/integration-r33/reference-provenance-reconciliation.json)
records depth, approach, seeds and combat-versus-settlement accounting.

[Final numerical evidence](docs/accuracy-dodge-r33.md) includes 12,988 main native
player trials and 3,456 NPC trials with no combat-horizon timeouts, plus separately
labeled fresh-r33 and Witch/sentinel cohorts. Fresh 32-world final primary means
are Night 26.25%, Skin 32.32% and Vampire 45.59% gross incoming damage; deaths are
0, 5 and 9 respectively out of 128. These are controlled encounters, not survival
rates. Severe counter-build tails remain; Queen/Witch surviving-brood aftermath
censors prevent a complete summoner encounter-cost claim. Rough design guidelines
were not turned into exact-percent conformance rules. Numerical matrices retain
their actual pre-presentation-guard binary/source hashes; they were not falsely
relabeled as measurements from the final executable. The approved stat catalogs,
analysis drivers and numerical artifacts remain byte-identical; the narrow
[post-analysis delta receipt](verification/integration-r33/post-analysis-presentation-delta.json)
and native equivalence checks cover the later narration-only change.

## Windows cross-build and limits

A locked **Windows x64 release cross-build** passed with Rust 1.99.1. Static checks
confirm the PE architecture, seven embedded icon images and DPI manifest, and all
84 runtime PNG plates embedded byte-for-byte. SDL2.dll is the only required bundled
non-system DLL. The unsigned stripped executable is **97,131,201 bytes**, SHA256
`52d966bbff805a48a2f3b956f46e8b17c11323de87a50b5bc8600ab70cf64624`.
[Build evidence](verification/integration-r33/windows-crossbuild-receipt.json)
records the final cross-build and its input hash comparison.

**Windows runtime was not tested; Wine was unavailable.** No Windows launch,
rendering, input, audio, save behavior or performance certification is claimed.
The executable was rebuilt after the Skin Walker presentation fix; earlier
pre-fix executables are not the release artifact.

Unsupported custom/import states remain explicit admission boundaries; see
[admission gaps](docs/native-engine-admission-gaps.md). DEBUG validation covers
actual generated loot and supported equipment, not exhaustive fuzz certification
of arbitrary hand-edited stash values. The existing generated
collapse/escape-route risks are not certified away by these tests. The selected
screenshots do not replace interactive platform testing. A repeated heart suffix
in one Pack description remains a deferred cosmetic cleanup, not a missing item
provenance field. No public publication or Library upload was performed.

## Historical records

The former top-level [revision-31 narrative](docs/native-verification-r31-historical.md)
and [revision-31 JSON receipt](docs/integrated-verification-r31-historical.json)
are preserved unchanged and **are not current r33 counts or balance claims**.
Earlier actual SDL acceptance remains in the
[historical verification record](docs/native-verification-history-20261020.md).
The separate frozen r32 delivery was not edited by this integration.
