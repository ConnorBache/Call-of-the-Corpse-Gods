# Visible wall-feeding state

## Request and scope

Designer request: “Feeding should be a specific state you can see if something is
licking the walls.” This is presentation only. It does not add an AI state,
condition key, timer, action, RNG draw, nutrition effect, or save-format field.

The existing revision-33 ecology is authoritative. Its source kind **1** is a
Bloody wall and its existing `Eating` description requires positive partial-meal
progress, an autonomous hungry creature, and the recorded stationary position.
Source kind **2** is actual blood water and remains `Drinking`. Hunger alone is
`Hungering`, not Feeding. Seeking a wall, standing beside a wall, corpse eating,
and merely being hungry do not create the new label.

## Presentation

- A directly observed, unprojected real creature with that active wall meal gets
  **Feeding** in its ordinary comma-separated status summary, alongside states
  such as **Flying**. Both the left creature rail and full creature list consume
  the same detached status list.
- Inspection uses the same visibility-gated feeding helper and adds **Feeding**
  to the comma-separated conditions. The old `Eating` behavior suffix is omitted
  there, so the activity is not duplicated. `Drinking` and `Hungering` keep their
  existing inspection behavior descriptions.
- `Actor.state` and detached `Creature.state` remain exactly unchanged. Existing
  wandering/ally/hunting/sensed/sleeping identity, color, Bestiary admission,
  sorting, and awareness checks keep their established inputs. Existing status
  fingerprints already repaint the rail/list when Feeding appears or disappears.
- Hidden, remembered-only, telepathically sensed, disguised, and hallucinated
  bodies do not expose Feeding. Six hallucination variants retain the projection
  boundary. The change adds no new terrain/food visibility permission.

## Lifecycle and compatibility

The label lasts during the existing first nine completed feeding turns. The
existing tenth-turn completion clears partial progress and hunger, so Feeding
ends. Existing damage, threat, displacement, source loss, paralysis, and missed
feeding-turn cleanup also remove it. Haste, offscreen processing, save/load,
Revert, and future restoration continue using the canonical ecology data. No
simulation code was changed. Legacy revisions without ecology cannot show it.

## Verification

`monster_status_ui::feeding_state_tests` contains seven desktop regressions:

1. Actual canonical wall feeding, hunger/proximity negatives, all nine active
   turns, tenth-turn completion, a repeated meal, and comma-separated Flying /
   Feeding without duplicated inspection text.
2. Damage, hunting, displacement, source loss, paralysis, and missed-turn cleanup.
3. Blood-water Drinking and non-feeding Hungering remain distinct.
4. Hidden and sensed actors, including unchanged unseen inspection memory.
5. A masked and then revealed Skin Walker, plus all six hallucination variants.
6. Validated storage round-trip, temporal snapshot restoration, and legacy gating.
7. Native rail, full list, and inspection rendering at 1280×800 and 960×720,
   including repeated idle painting and unchanged canonical state/RNG.

Focused execution: `tools/cargo.sh test -p corpse-desktop feeding_state_tests`.
Set `CORPSE_FEEDING_STATE_REVIEW` to a review directory to emit six native-software
PNGs and the controlled input save. The fixture starts a real canonical feeding
turn; it is not a naturally reached playthrough. The existing core
`creature_feeding_v33` suite remains the broader normal/haste/offscreen/temporal
mechanics regression. A passing desktop render is not Windows GUI verification.

The coordinated **compile-20** native test binary executed all seven cases:
**7 passed, 0 failed**. Evidence and exact binary/source hashes are in
`verification/feeding-state-r52-compile20/`. Every completed room fixture now
passes `storage::validate`, has the required empty depth-1 ground block, and
clears stale player ability-order/equipment-ability metadata when abilities are
removed. Template log entries and their log-kind metadata stay paired. The
validated save round-trip and temporal snapshot regression passed.

All six compile-20 native captures were individually opened and visually
inspected. **Flying, Fast, Feeding** is readable in rail, list and inspection at
1280×800 and 960×720, with no duplicate Feeding label or clipping. These remain
controlled software-render fixtures, not naturally reached gameplay or Windows
GUI validation. Original compile-18 failures and captures remain untouched in
`verification/feeding-state-r52-compile18/`: its stale status-order assertion and
missing-ground fixture were corrected in tests, without changing production
feeding behavior. No production or test source changed during compile-20 QA.

## Complexity

Let **V** be admitted visible creature rows, **A** floor actors, **C** per-actor
condition entries, **K** ability entries, **P** the existing perception/LOS/portal
query cost including cache misses, **S** status-summary characters, and **Q** the
existing ecology autonomy/priority-query cost. Observation runs when the view is
rebuilt, never on cosmetic idle paint.

- The new helper reuses caller-established direct-observation and projection
  decisions. It makes no additional visibility pass for the rail and retains
  the prior single direct-visibility check in inspection. It calls the existing
  core ecology description once and maps one fixed label. For non-allies, Q is
  a fixed set of condition lookups plus existing actor classification; expected
  hash-map lookup is O(1), with O(C) worst-case map lookup. For an autonomous
  ally, existing priority helpers can scan A actors, abilities, and perception
  paths: conservatively Q is O(C + K + A*P), with nested perception work included
  in P rather than assumed constant. The helper itself adds O(1) scratch.
- Added observation work is O(V*Q) and O(V) retained fixed-size labels. Existing
  status-vector append can reallocate/copy that row's statuses, which is O(S)
  for the row in the worst case; amortized append is O(1). Inspection adds one
  fixed label to its pre-existing condition collection and comma join, O(S).
- Native/software text wrapping and rasterization keep their existing costs:
  measured word wrapping can be O(S²), followed by work proportional to rasterized
  glyph pixels; GPU/software commands consume the same detached strings. The
  fixed seven-character label adds no new loop or cache. Status/dirty-region
  fingerprints include it and use the existing repaint/scroll behavior.
- Canonical retained state, replay/save size, and simulation execution costs are
  unchanged. These are asymptotic bounds, not measured timing claims.
