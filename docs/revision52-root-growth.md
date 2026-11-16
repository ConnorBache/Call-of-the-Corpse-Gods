# Revision 52 incremental corpse roots

New fungus/corpse connections retain the existing radius-10, line-of-sight,
shortest direct Bresenham route. They begin with material only at the fungus and
advance one cell along that route after each three elapsed normal environment
turns. A new link discovered during turn N first extends on N+3. New generated
feasts begin at turn zero and first extend on turn three. Each connection has
its own deterministic cadence; there is no two-target acquisition cap.

Extra haste movement, free actions, animation frames and observation do not
advance the timer. Exact offscreen normal-turn simulation uses the same routine.
A floor that has never been visited still follows the established offscreen
activation policy: it does not retroactively simulate a lifetime before play.

Only the grown prefix is physical root material, visible root geometry or
severable root. A target is not rooted until its prefix reaches the corpse.
Only a completed intact connection prevents revival and increases fungal gas
output. Rooted corpses cannot resurrect. The emitted gas and per-completed-link
volume formula are unchanged. A corpse can revive normally before contact,
which invalidates and clears its incomplete connection.

The original path is never rerouted. Endpoint death/movement, removal, a cut or
fire on existing material retain the R51 severed-pair history and cleanup.
Shared cells survive while another active prefix supports them. Fire on an
unbuilt next cell or frozen material through that next cell pauses progress
without consuming accumulated turns or burning a nonexistent root. Existing
collapse/terrain-destruction rules continue to invalidate an intersecting route.

## Persistence and compatibility

Each new R52 connection adds optional serialized state:

- `growth.grown`: number of materialized path entries, from 1 through path length.
- `growth.turns`: elapsed eligible normal turns since the last extension, 0–2.

The full original `path` remains stored. Missing `growth` means fully grown,
not zero progress. Existing saved roots therefore stay grown and are not erased
or retimed. Revisions before 52 still acquire and generate roots under their
historical rules. No new RNG draw, external clock or runtime cache is used.

The existing floor COW payload carries progress through raw saves, compatible
load normalization, replay, Snapshot and Future. Compatible normalization checks
only existing prefix material, never fills missing cells, and preserves elapsed
progress. Invalid serialized progress is rejected rather than clamped or silently
repaired. No replay/snapshot root list or top-level save version changes are
needed; the optional state is nested in an already-retained floor block.

## Hallucinations and observation

Root geometry remains terrain, not a hallucinated creature identity. Observation
indexes only grown prefixes, including their real tip orientation, so overlapping
material from another branch cannot reveal an unbuilt route or make a false
junction. Hidden remembered roots still keep their last observed appearance.

R52 growth animation uses the existing terrain-only, directly-visible-cell gate
rather than the actor-trait hallucination suppression. It contains newly grown
cells and the root style, not a fungus identity or a target silhouette. Other
hallucination identities, six-phase cosmetic cycling and trait-animation rules
are unchanged. Static root glyphs and configured root appearance remain intact.

## Complexity and portability

Let A be actors on the affected floor; E their inspected ability entries; F living
fungi; C corpses; K retained connection records, including severed history;
P total stored path entries; G total grown prefix entries; V<=3600 cells;
R<=11 cells in an admitted canonical radius-10 path; L the existing line-of-sight
query cost; B the total cloned actor payload; S a detached floor payload.

- Each normal tick keeps the R51 normalization cost, expected
  O(A+E+V+P+K*R) time and O(A+V+K+R) scratch. It includes canonical path creation,
  endpoint validation, duplicate-pair suppression and support-mask cleanup.
  Prefix material scans cost O(G), already bounded by P. First writes to a
  shared floor additionally copy O(S) bytes; retaining a Snapshot/Future can
  trigger that copy. There is no per-frame simulation work.
- Growth/classification/acquisition adds expected O(A+E+B+K*R+F*C*(R+L)) time,
  plus existing gas emission/initialization, source-owner lookup and per-cue
  visibility work.
  Existing gas helpers can scan/normalize O(V) environment arrays per fungus;
  they are not constant-time. Actor-index cold/fallback lookup and owner resolution
  retain their existing costs (up to O(A) per source without a warm index). New
  acquisition compares a pair before doing LOS.
  Growth advances at most one cell per active incomplete connection per normal
  turn and scans only the prefix through its next cell for freezing.
- Scratch is O(B+F+C+K+R+V), including cloned actor payload, hash sets/maps,
  at most K emitted growth points and the existing root-layer allocation when
  absent. Pair/count sets are transient; no new long-lived index/cache exists.
  Expected hash bounds assume ordinary hashing; worst collision membership can
  add O(K²+F*C*K+F*K) work. No result order depends on hash iteration.
- New persisted state is O(K) scalar metadata, two small integers for each new
  link. Full path storage remains O(P), not duplicated. JSON save/load is
  O(serialized world bytes); nested Future and retained temporal normalization
  add their existing state traversal costs. Copies of shared floor handles are
  O(1) per handle, but the first detached floor clone remains O(S).
- Orphan cleanup is O(V+K+G) time and O(V) scratch. Burn/sever scans retain their
  original path/connection and world traversal costs. Corpse-rooted checks scan
  connections, source actors and path material as before, with an O(1) completion
  gate that rejects incomplete links before scanning source actors/material.
- Observation builds the root index in expected O(K+G) time/space, bounded by
  the old O(K+P). Direct growth cues add up to K visibility checks, not O(1).
  Native glyph rasterization, font-cache misses, grading and framebuffer upload
  are unchanged. The new six-phase hallucination regression does not claim
  exhaustive renderer parity or native Windows validation.

These are source-derived bounds, not measured latency. Profile many-corpse and
many-severed-record saves, normal/offscreen ticks and large Future anchors before
making performance claims. The initial feast does not bound lifetime K.

## Verification

The focused `revision52_root_growth` suite covers new acquisition and generated
feast boundaries, direct paths, contact/revival/gas, normal versus haste commands,
command replay and interrupted save/resume, offscreen progress, Snapshot/Future,
legacy full links, cuts/shared-prefix cleanup, source death, freezing/fire ahead,
malformed save progress, and hallucination-visible growth cues. Existing R51
root/feast suites remain separate compatibility checks.

Set `CORPSE_ROOT_GROWTH_REVIEW=verification/revision52-root-growth` on the new
suite to export actual normal-turn 1/3/4/7/13 and hallucinating states. Render with
the coordinated desktop executable using `--fixture FILE --render-game IMAGE
--render-size 1920x1080`. Execution results and platform limits are recorded after
the coordinated verification, not inferred from authored assertions.


### Executed focused checkpoint

The optimized native core command passed 34/34 tests: 14 new R52 root-growth
regressions, 9 R51 root-cleanup regressions and 11 R51 fungus-feast regressions.
`verification/revision52-root-growth/core-focused-02.log` is the full output;
`core-focused-02.exit` records exit 0. Debug assertions and overflow checks were
enabled, with locked/offline dependencies and two test threads. The first attempt
compiled runtime successfully but rejected a test's use of private event helpers;
the corrected test now observes actual public Engine command outcomes.

The desktop six-phase root-appearance and ability-description regressions have
been added for the coordinated desktop batch. They are not included in the
34-test result. Full-workspace, rendering and Windows checks remain separate.
