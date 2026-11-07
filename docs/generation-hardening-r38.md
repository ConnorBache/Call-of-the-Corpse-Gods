# Revision 38: generation hardening and authored copy

## Scope

New descents and DEBUG starts use simulation revision 38. Explicit historical
revisions 29–37 retain their generation and replay semantics, including their
historical failing seeds. This does not retroactively change an old run's map.

The release corrects four generation defects:

1. Verdania's tap candidate and fallback now exclude both planned stair
   footprints before stairs/recesses are painted. This addresses the verified
   floor-20 collision in `r37-event-count-0047` directly.
2. Repairable candidate connectivity failures (including inaccessible protected
   rooms) reject that candidate and advance the existing bounded geometry search
   instead of aborting the whole run on its first unsuitable geometry.
3. Machine/setup heart-budget preflight now uses the current revision's eligible
   ordinary roster, rather than the obsolete revision-29 feasibility pool.
4. Late sub-biome trees and chasms preserve the generator’s hazard-free cardinal reachability, not merely
   a raw route through permanent fire. A final populated-floor transition check
   rejects any remaining unsafe candidate atomically. Independent stress exposed
   five such route failures in the initial r38 candidate; these are retained as
   regressions, not counted as successful verification. This is a stricter
   generation-contract violation, not a proven gameplay softlock: legal diagonal
   movement can still provide an alternate route on the diagnosed floor.

The built-in startup also treats complete floor generation as an atomic
transaction. Classified geometry/placement failures can try up to eight complete
floor candidates, each using a distinct window of at most 32 quality candidates.
Rejected attempts cannot consume IDs, unique item grants, metrics or DEBUG
rewards. Original seed, domain/Godbeast/Nest/stair plans and the exact ordinary
heart budget stay fixed; required actors/items and safety validators are not
removed. Different accepted geometry can contain different legal machine
arrangements, as before. There is no unbounded retry or emergency empty map.

Unsupported inputs, corrupt catalogs, malformed fixtures and internal invariant
errors still fail explicitly. Exhausting all bounded safe attempts also returns
an error with the last reason. Finite stress tests are not a proof that every
possible seed can never fail. No known valid-input revision-38 failure is
accepted as a successful test result or silently substituted by another seed.

Public custom-generation callbacks remain single-attempt and atomic. They are
not silently invoked repeatedly, and `spawn_floor` does not replace an explicitly
supplied caller map.

See [geometry audit](generation-geometry-r38.md) and
[startup/population audit](generation-startup-r38.md) for error classifications,
complexity bounds, direct fixes and regression coverage. The final measured
results and platform limitations are in [verification](verification-r38.md).

## Designer-authored replacements

The designer supplied replacements for batch entries 1, 2, 3, 4 and 10. They are
applied to every located active duplicate:

- Queen Natural History in both the manual and runtime description.
- Dreamwalk and Wake Up descriptions, with the Realm of Dreams wording.
- Mask of Trickery item description separated from Teleport's ability text.
- Borge's first-strike instruction, preserving the designer's exact “it's”.

Teleport's actual standable-destination selection and 100-turn cooldown do not
change. Its normal separate cost/readiness state remains displayed. Queen royal
jelly wording is prose, not an item rename or a change to Royal Honey mechanics.
Batch entries 5–9 are still pending and were not rewritten. This release does not
claim all assistant-written game text has been removed.

New generation diagnostics are inventoried in
`verification/integration-r38/new-player-text-provenance.json`; existing unknown
origins remain unknown. Art and all non-Queen authored catalog content are
preserved. The Queen manual asset changes only the requested Natural History.
