# Revision 51 perception and awareness

## Designer-approved rules

This revision replaces ordinary exponential sight with deterministic range gates.
Earlier saved revisions retain their formulas, RNG ordering, starting Perception
and heart rewards. New R51 players begin at **0 Perception**, not 300.

For observer effective Perception P and target illumination L:

- Tile range is `max(0, 20 + L/5 + P/5)`; an optically reachable tile within that
  inclusive distance is visible. A zero range reveals no tile.
- L >= +1 bypasses distance, but not LOS or invisibility/plane rules.
- A creature uses `2 * max(0, optical_distance + stealth + submerged_stealth)`
  against that tile range. Creature concealment uses its existing actor-local
  illumination modifiers; terrain visibility uses environmental illumination.
- Effective Perception retains the existing stat rules. Stealth, submerged
  stealth, range and lighting fractions are not rounded away. No legacy
  underwater Perception multiplier remains.
- The designer explicitly confirmed **trust the formula** at L=-100: P=100
  gives tile range 20 and creature range 10 (zero stealth), rather than a special
  complete-darkness cutoff. The explicit pure policy helper also tests the
  rejected opaque interpretation to guard against accidentally restoring it.
- Players and allies perceive eligible creatures deterministically. Wandering
  enemies acquire an eligible target with 25% chance per normal actor turn.
  Sleeping enemies have 10%, only at actual optical distance <= 10. Special senses
  can establish eligibility through blocked optics; their sleep cap falls back
  to direct Euclidean distance when no optical route exists.
- Hunters refresh last_seen deterministically whenever the target is plausible.
  Otherwise they pursue their stored location and forget on reaching it without
  a plausible target. Low energy and 20 unseen turns cannot cancel active pursuit
  into recovery sleep, including inactive-floor pursuit. A last-known stair
  location is reached at a legal adjacent square for creatures forbidden to
  occupy stairs; visible targets are reacquired before this arrival check.
  Witnessed cross-floor chase keeps the existing adjacent arrival/defer rules.
  A successful sleeping detection records hunting/target
  memory immediately and spends the turn waking, without another 25% acquisition.
- Haste does not add a second general acquisition lottery. Ordinary nearby-target
  hunter locks and doubled hunting/halved sleeping Perception are no longer used.

Existing optical blockers, portals, special senses, map memory, invisible-target
handling and explicit specialist behaviors are retained. The API separates
plausible actor-local eligibility from probabilistic awareness. R51 allied target
acquisition now checks that same eligibility rather than ignoring concealment.

## Approved hearts

New R51 drops add Perception while preserving their other rewards:

- Cave Bat: +1.
- Tiger: +2.
- Sabretooth Tiger: +3.
- Dragon: +4.

Banshee +3, Skin Walker +2 and existing Zillennia Perception are unchanged.
`inventory::heart_for_revision` is used by live death drops and generated debug/
playtest starting profiles. `heart_for` remains the frozen legacy constructor;
serialized existing items are not rewritten. The desktop inspector uses the
versioned constructor for truthful reward previews.

## Query and presentation boundary

`revision51_perception` contains pure range, functional-distance and awareness
helpers. `vision::plausibly_visible_actor` is actor-local, not the player's shared
viewpoint union. `vision::perception_actor_light` applies actor-only modifiers to
an already-known scalar light sample. `vision::perception_tile_mask` is a full
simulation query and must NOT be treated as a knowledge-safe desktop overlay.
The desktop contour is owned separately in `revision51-perception-ui.md` and must
use detached known terrain/light, omit uncertain boundaries, and never derive a
visible edge from hidden walls, portal connections or light sources.

The existing 0/100 query return type remains compatible with terrain>5 and
actor>=50 admission thresholds, but R51 values are eligibility rather than odds.
Only `detection_chance` returns awareness odds 25/10/100.

## Complexity and ownership

Let N be floor tiles, A actors, I owner inventory entries, B actor ability/status
entries, K portal endpoints, R ray length and Pn observer viewpoints.

- Range arithmetic is O(1) time/space. `effective_stats` retains its existing
  O(I+B+equipment-enhancement) cost. Each immutable optical projection captures
  the effective Perception once per observer, not once per tile.
- Scalar actor eligibility retains the existing optical-route and scalar-light
  costs, including O(K²R) route work and actor/source light scans. The added
  special-sense checks scan only the supplied observer's statuses/abilities.
- Sleeping awareness may perform a second optical-route query to enforce its
  actual-distance cap; awake/hunting awareness does not. No gameplay RNG is used
  by visibility queries. The engine draws once per eligible uncertain candidate;
  R51 zero and 100% outcomes consume no acquisition draw.
- Batched terrain/actor projection retains the lighting and optical caches and
  their invalidation boundaries. It adds O(Pn*(I+B)) observer-stat capture and
  O(Pn) stored scalars. Full tile masks retain O(N) output plus those caches.
- The normal enemy phase remains O(A * hostile_candidates * eligibility_cost),
  with no per-frame rolls or persistent Perception cache. Same-tick stat, item,
  light, portal and obstacle changes are observed on fresh immutable queries.
- Versioned heart rewards add O(1) fixed identity matching to the existing item
  construction. They do not rebalance monsters, change spawn rates or add RNG.

## Verification

Focused suites: `tests/revision51_perception.rs` and
`tests/revision51_perception_lifecycle.rs`, plus unit tests in the new module.
They cover inclusive limits, the +1 light threshold, L=-100/P>0, stealth ordering,
actual Perception, invisibility/telepathy, sleeping distance/awareness, ally
acquisition, retained hunting memory, successful waking, unchanged legacy sight,
projected/scalar agreement, zero-Perception startup, approved heart consumption,
portal/plane behavior, inactive-floor low-energy pursuit, legal stair-edge
arrival/reacquisition, cross-floor following and read-only query/RNG invariance.

Verification status is recorded by the integrating worker after running the
final source. No Windows runtime claim is made by these simulation tests.

### Cloud verification, 2026-11-07

The optimized, locked/offline core build passed 49 integration tests: 12 sight,
8 lifecycle, 9 retained legacy vision/ring/cache checks, 11 fungus feasts,
7 speed-up tiles and 2 floor-40 DEBUG/save-resume checks. The three new
stair/offscreen pursuit cases failed against a saved pre-fix library and passed
against the final library. Five pure formula tests also passed, including
fractional Stealth and the confirmed -100-light formula. Logs are under
`verification/revision51-perception/`. This is focused cloud evidence, not a
full-workspace or Windows-runtime certification.
