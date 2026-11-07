# Revision 36 native desktop evidence

These captures are produced by the actual native `Painter::game_at` rasterizer. They are headless renderer evidence, **not** evidence of a running interactive window or of Windows runtime behavior. No original creature art or lore has been replaced.

## Captures

- `non-euclidean-possible-exits-controlled.png`: controlled eight-cell topology fixture on a floor labeled 20. Cyan insets preserve original terrain glyphs; hollow cyan corners show both known possible exits. The player/geometry are synthetic. The serialized fixture is `non-euclidean-controlled-run.json`.
- `non-euclidean-generated-floor20-debug.png`: seed `non-euclidean-review-r36`, real revision-36 generation and population through `startup::ensure_floors(..., 20)`, with **controlled player placement and full reveal** for inspection. No turns simulated. This does not claim to exercise the interactive DEBUG reconstruction/level-choice workflow. Serialized capture input: `non-euclidean-generated-debug-run.json`.
- `epidite-9-turns-controlled.png` and `epidite-10-turns-controlled.png`: controlled existing Epidite fixture at respective serialized ages 21 and 20. The former shows subscript 9; the latter hides the subscript. Hover text begins with the exact remaining timer, `9 turns until judgement` or `10 turns until judgement`. No timer/damage mechanics changed.

The renderer was visually inspected for all four captures. Original portal italic ASCII `0` and portal pair colors remain unchanged.

## Reproduce

From the source root with the pinned toolchain:

```sh
CARGO_INCREMENTAL=0 \
CARGO_PROFILE_TEST_OPT_LEVEL=2 CARGO_PROFILE_TEST_DEBUG=0 \
CARGO_PROFILE_TEST_DEBUG_ASSERTIONS=true CARGO_PROFILE_TEST_OVERFLOW_CHECKS=true \
CORPSE_NON_EUCLIDEAN_UI_REVIEW="$PWD/verification/integration-r36/desktop-ui" \
CORPSE_NON_EUCLIDEAN_GENERATED_REVIEW="$PWD/verification/integration-r36/desktop-ui" \
tools/cargo.sh test -p corpse-desktop --locked
```

Relevant test filters: `non_euclidean_ui`, `epidite_ui`, `portal_ui`.

## Checked behavior

- Pure observation/hover/route/targeting does not mutate Run or consume simulation RNG.
- Known deterministic links route to their actual arrival. Stochastic entry may be an explicit final route step; automatic travel halts afterward rather than promising or rerolling a chosen branch.
- Projectile previews stop at the first stochastic entry and expose possible known exits separately. Unknown group endpoints stop previews; changes to hidden terrain cannot change the preview.
- Same-plane aliases preserve one actor list identity and one ground-item record; never reveal a hidden actor through a pocket. Normal and dream views keep aliases in the appropriate plane.
- Local vision remains local, even across a linked group behind an opaque barrier.
- Both known stochastic exits participate in hazard warnings; unknown hazards stay hidden.
- Epidite counts 30/10 hide the map subscript; 9/1 show it. A real `epidite::world_turn` rollover from 1 returns to hidden 30. Hover and subscript read the same core countdown; non-direct observation never exposes the live clock.

An initial optional generated-capture test inspected a normal startup before generating floors 20+: normal startup creates only floors 1–10. Its archived failure is a capture setup error, corrected by calling `ensure_floors(...,20)`, not evidence of a pocket generator rejection. The corrected full desktop unit run passed 230 tests with the one existing ignored test. A later added same-plane regression also passed; see the final frozen desktop log/receipt for the complete current count.

`desktop-source-hashes.json` records the owned desktop source paths at handoff. The final workspace-wide gate is owned by the integration lead and recorded separately.

## Final desktop result

The final run including the linked-origin firing preview regression passed **285 tests**, failed **0**, and retained **1 pre-existing ignored test**. `desktop-final.log` is the complete log; `receipt.json` records current artifact hashes. All four PNGs were freshly rendered in this run. Stochastic linked firing origins stop the preview at the player’s actual tile and show both known possible sources instead of promising one branch.
