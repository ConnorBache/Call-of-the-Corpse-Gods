# Avatar-only new playable runs

The current new-game boundary creates an Avatar with three distinct gods. The
desktop menu and CLI use `startup::create_avatar_run(seed)` for normal starts and
`debug_floor::create_avatar_run(seed, floor)` for DEBUG/playtest starts. Both use
simulation revision 52 and `dev = false`; neither accepts a mode parameter.

This is a new-game policy, not a save migration. Existing saves, loaded sessions,
fixtures and seed replays retain their recorded Avatar, Blessed or Unblessed mode.
The low-level `new_run_shell`, `create_run`, `create_run_with` and explicit-revision
constructors remain compatible. Their historical default revisions are unchanged.
`storage` admission and `replay::create_initial`/replay restart are not redirected
through the new-game wrappers. Existing DEBUG reconstructions remain loadable as
saved sessions; their unsupported command-ledger replay export remains unsupported.

## Route audit

- Desktop normal Play, generated CLI checks/renders, end-of-run new-seed starts,
  and menu-triggered starts converge at `new_selected_run`/`new_run`.
- Desktop DEBUG and `--playtest-floor`/`--debug-floor` creation converge at
  `new_selected_run`; the same Avatar boundary applies.
- Continue and imported saves use the session loader, not a new-game constructor.
- Replay create and restart call the recorded-revision normal or historical
  playtest constructor. They must not replace the recorded mode with Avatar.
- Developer examples/tests may still deliberately call compatibility constructors
  with old modes. These are not alternate desktop playable menu options.

## Determinism and seed identity

The wrappers delegate without preprocessing the seed or making RNG draws. Existing
seed parsing enforces three distinct god IDs in 0–11 for a numeric prefix. An
already valid prefixed seed is retained byte-for-byte, including leading zeroes
and its complete suffix. Bare seeds retain the existing deterministic prefix
derivation. The desktop owns selecting a fresh suffix for a blank new-game seed.

The complete seed, including the ordered god prefix, participates in world
generation and starter RNG. Choosing different gods while retaining the suffix
therefore does not promise identical geometry or loot. It retains the existing
single complete-seed replay contract rather than introducing another seed stream.

Historical modes retain their original god count and starter RNG draw count:
Avatar three, Blessed one, Unblessed zero. They are not normalized on load.

## Complexity and bounds

The two wrappers add O(1) time and O(1) extra memory per requested new game. They
do not copy or parse the seed themselves, cache data, access storage, render,
perform native-library work or run per turn/frame.

The delegated construction is not O(1). For S seed bytes and D generated floors
(normal D=10; DEBUG D=max(10,target), target in 1–40), it retains the existing
O(S) seed parsing/hash allocation plus full startup/population costs. Each floor
has up to eight complete population attempts, each with up to 32 geometry
candidates. Existing clone/COW ownership, generation, population, creature-index
and prior-floor DEBUG reward costs are documented in
[population and startup](population-startup-coverage.md),
[generation](generation-complexity.md),
[startup recovery](generation-startup-r38.md), and
[DEBUG reconstruction](debug-floor-testing-v33.md). No new bound or caching
assumption is added; failed generation still reports the existing error.

## Verification

`crates/corpse-core/tests/avatar_new_run_boundary.rs` covers:

- Exact current normal and DEBUG equality with the existing revision52 pipelines.
- Duplicate/out-of-range god rejection before generation.
- Historical mode, seed, god and RNG semantics at revisions 29, 39, 51 and 52.
- Both original non-Avatar save fixtures loading without mode conversion.
- Revision52 Blessed/Unblessed seed replay creation, restart and saved-session load.

These deterministic tests add no claims about unexecuted platform UI behavior or
measured startup timings. Desktop selector, input, CLI rejection and presentation
coverage are separate from this core compatibility boundary.

On November 8, 2026, all six targeted tests passed in the cloud native environment
with pinned Rust 1.99.1, unchanged Cargo.lock, `--locked --offline`, test opt-level
1, debug assertions and overflow checks enabled, and two test threads. The test
phase took 3.22 seconds; this is a single suite run, not a startup benchmark. Full
workspace and desktop verification are reported separately. Command:

```sh
source ../cloud-build-env.sh native
CARGO_TARGET_DIR=../avatar-menu-target \
CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 \
CARGO_PROFILE_TEST_OPT_LEVEL=1 CARGO_INCREMENTAL=0 \
CARGO_PROFILE_TEST_DEBUG_ASSERTIONS=true \
CARGO_PROFILE_TEST_OVERFLOW_CHECKS=true CARGO_BUILD_JOBS=2 \
bash tools/cargo.sh test --locked --offline -p corpse-core \
  --test avatar_new_run_boundary -- --test-threads=2
```
