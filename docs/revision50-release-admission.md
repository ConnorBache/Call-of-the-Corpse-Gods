# Revision 50 coordinated release admission

This patch opens runtime admission only after the separately implemented R50
mechanics have been reviewed. It is not a migration of active games, an assertion
that all mechanics have exhaustive coverage, or a Windows verification result.

## Exact gates and defaults

- `startup::new_run_shell_revision`, `storage::validate_current_revision`, both
  `ReplaySession` constructors, debug profile validation/debug creation and
  playtest creation add exact `50` to their existing lists. Revisions 47–49,
  unknown/fractional/nonfinite revisions and mismatched fixture/replay revisions
  remain rejected. Generation, setup and population already support exact 50.
- Desktop `new_run` and `new_selected_run` select 50 for new normal/debug runs.
  Continue/load still uses the saved revision. No imported actor, item, blessing,
  reserve or active saved timeline is upgraded or relabeled by this patch.
- Historical core convenience APIs retain their existing 39/33 defaults.
  `Header::current()` retains its historical revision-39 receipt identity.
- Recording uses `corpse-r50-ability-roster-v1` and
  `corpse-r50-ability-roster-metadata-v2`; clean R50 metadata resume was already
  implemented. No framing, source-conflict, file-identity or locking rule changes.
- Debug version-2 loadout reconstruction remains non-seed-replayable; synthetic
  version-1 playtest seed replay retains its existing explicit profile contract.

## Authored acceptance coverage

`revision50_admission.rs` exercises all 12 Blessed starter pools with bounded
fixed-seed sampling and matching final legacy RNG states; actual generated
Avatar/Blessed/Unblessed starts; command progression; Engine save/load; seed
replay and restart; debug/playtest creation; unchanged revision-46 save and
shared-energy semantics; invalid revision rejection; and a controlled admitted
Engine trace with movement, reverse Timeweave, flooding/fire-immunity potions,
All Seeing Eyes, Sparkle, Iceblast, Future capture/return and checkpoint/replay
agreement. The trace checks restored blessing/item pools, terrain, statuses and
Timeweave, rather than only checking that a serialized field exists.

The deep public-constructor checks generate actual floors 6 and 40 through both
`debug_floor::create_run_revision` and `playtest::create_run_revision`, admit the
results through Engine, and round-trip them before and after an initial world
command. Debug coverage also reloads pending rewards, completes generated choices,
takes and equips actual generated jewelry, and enters the target floor. Synthetic
loadouts retain starter Bow/arrows, exclude weapon/armor equip, satisfy category
limits and preserve selected-item provenance. These complement the item subsystem's
helper-level deeper-floor tests; they are not injected geometry fixtures.

The existing real-start reserve, preserved-edge Future/replay and recording
resume integration tests are enabled. Recording resume now starts from actual
R50 generation rather than relabeling a legacy fixture. Existing desktop menu
creation tests assert 50 for explicit and OS-random new starts and debug starts.
The old closed-gate assertions in identity/runtime-admission tests are updated.

All tests above are authored, not run during patch preparation. Required
required checks after prerequisite integration:

1. `tools/cargo.sh test -p corpse-core --test revision50_admission`
2. `tools/cargo.sh test -p corpse-core --test revision50_energy_lifecycle`
3. `tools/cargo.sh test -p corpse-core --test revision50_preserved_edges`
4. `tools/cargo.sh test -p corpse-desktop revision50_protocol`
5. `tools/cargo.sh test -p corpse-desktop menu_seed_creation_tests`
6. Compatibility filters for revision46 admission/protocol, recording resume,
   metadata operations and incremental log, followed by the full workspace suite.
7. Existing Windows build/package workflow and a real launch/Continue/new-game,
   inventory/apply/targeting and save/resume smoke test. This source patch does
   not perform those platform/UI checks or package a game.

The all-god seed test verifies every authored starter entry but not every possible
seed or starter combination. Generated-start tests cover one fixed world seed
per mode and first-floor progression, plus full public reconstruction through
floor 40 for one fixed seed. Reconstruction does not establish campaign completion
or simulated survival through all 40 floors.
The controlled command trace is fixture replay, distinct from actual seed replay.
Interactive debug choice/loadout UI, long campaigns and platform behavior remain
separate checks. Acceptance tests must fail honestly if a prerequisite is absent.

## Complexity

The version checks add one entry to fixed-size lists: O(1) time and O(1) extra
memory. Default selection changes a scalar only. No cache, per-turn traversal,
additional gameplay RNG draw, renderer operation or save migration is added.

Startup retains its existing bounded candidate generation, population and
acquisition costs. Engine/load, replay and resume retain their transitive world
validation/serialization/simulation costs, documented in the startup, temporal,
recording and subsystem entries; this patch does not make those operations O(1).
For S starter samples and P entries per pool, the starter test uses O(S times
shell construction/acquisition) time and O(P) observed-ID storage. S is capped at
12 × 256 and currently P is at most two. Generated tests have a fixed number of
world constructions, each retaining generation/population bounds. For C commands,
state size W and command-specific costs T_i, checkpoint/replay tests cost
O(CW + sum(T_i)) plus explicit repeated generation and retain O(W) comparison
states and the engine's existing temporal storage. These are source-based bounds,
not measured timings. No Cargo or Windows command ran during preparation.
