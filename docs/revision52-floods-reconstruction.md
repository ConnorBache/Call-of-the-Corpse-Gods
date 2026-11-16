# R52 Floods source reconstruction, 2026-11-08

## Provenance and scope

Four missing or outdated core files were reconstructed in an isolated cloud
staging tree from the retained source and the latest packaged design records.
This is a behavioral reconstruction, not byte-exact recovery. None of its four
new source hashes matches the unavailable released hash. The release inventory
and old candidate remain untouched. This work neither contacted the user's PC
nor retried the unavailable source importer.

Authority: `revision52-floods.md`, `revision52-amulet-tiers.md`,
`r52-floods-native-ui-verification.md`, and `current-design-amendments.md`.
The restored boundary is exact simulation revision 52. Later requested reserve,
Hanged, and stat changes are separate work and are not included here.

## Restored behavior

- Each cardinal or diagonal wave lane includes its first preexisting shallow
  cell and stops there. That cell becomes deep water. Existing deep water is
  excluded and stops the lane. Closed doors remain excluded.
- Geometry and shallow membership are captured before damage, forced arrival,
  quenching, or terrain writes. A cast cannot consume its own new shallows.
- Natural shallow water is restored after 12 normal turns. Promoting an earlier
  temporary shallow record updates that same record in place, retains its first
  predecessor, and refreshes expiry to the current tick plus 12.
- Shared ground, door, grate, portal/alias, authored-terrain, Borge and Epidite
  protections remain. Only an existing shallow record can be promoted. No
  general terrain stacking is introduced. Later independent terrain replacement
  remains protected from expiry.
- The original damage, push, source/plane/Duel gates, quenching and payment paths
  are retained. The released 50 reserve capacity, 30 cost and frozen Tier 1–9
  damage values are unchanged by this patch. The optional follow-up reserve
  patch changes that separate capacity contract and must adjust the tests too.
- Deep-water temporary records are admitted only in exact R52 using the existing
  record type and original-ground whitelist. No model field or migration is
  added. Save/load, Snapshot, normal-turn and offscreen ownership are unchanged.
- Preview knowledge gates precede terrain reads. Animation comes from actual
  reached cells and retains independent visibility filtering and zero RNG use.

## Known uncertainty and differences

The previous runtime files were retained wherever unrelated to these behaviors.
The tier module needed only Floods description text; its numeric table and all
other profiles remain byte-for-byte unchanged. New helper names and comments,
the exact descriptive sentence, and the entire missing 14-test suite were
written anew. Their spelling and organization cannot be certified against the
unavailable source. Expected-versus-reconstructed bytes/hashes are recorded in
the accompanying machine-readable manifest.

The terminal shallow cell receives ordinary damage and up-to-two-tile push,
with no additional conversion packet. This preserves the explicit narrow
implementation inference in the released Floods design document; it is not a
new claim of a separately confirmed designer creature exception.

No discrepancy from the documented R52 behavior was found in the executed
regressions. This does not establish equivalence to every path in the released
Windows executable. Native desktop tests are a separate worker's gate, and the
full merged workspace and Windows build/runtime gates remain separate.

## Reconstructed verification

The fourteen deterministic regression groups cover mixed/diagonal independent
lanes, repeated casts and fallback, single-record refresh and original terrain,
natural water and independent replacements, raw storage and Snapshot,
offscreen catch-up, all nine damage tiers and terminal pushes, normal/haste and
free-cast clocks, hidden reachable terrain and invisible actors, animation
coordinates, protection gates, Duel and plane isolation, invalid records,
legacy stopping, and tier-aware positive-effect prose.

Cloud Rust 1.99.1, offline dependency cache:

- Runtime `cargo check -p corpse-core`: passed.
- Focused reconstructed suite: 14 passed, no failures.
- Full core-library suite: 662 passed, no failures, two existing manual
  benchmarks ignored. This includes the existing R50 wave/control and R52
  amulet-tier suites plus the reconstructed Floods suite.
- Nine relevant integration suites: 80 passed, no failures. They cover R50
  admission (8), amulet energy (7), inventory boundaries (4), preserved edges
  (12), runtime paths (23), R51 aiming (7), R52 admission/generated-world and
  replay equality (6), saved role coefficients (3), and Snapshot overwrite (10).
- Final core-library rerun through the repository Cargo wrapper: 662 passed,
  no failures, two ignored, with identical four-file before/after hashes.

Together the final library and selected integration runs passed 742 tests.
This is not a whole-workspace test result. Logs, exit codes and source manifests
are retained alongside the reconstruction patches.

The first test compile exposed an incomplete synthetic permanent-terrain struct
literal. Supplying its required predecessor field corrected the fixture; runtime
code was unchanged. No production validation or test assertion was weakened.
The final full-suite log includes a stronger hidden-terrain check than the first
focused pass. Existing unrelated compiler warnings were not broadened into
cleanup changes.

## Complexity

The runtime bounds remain those in `revision52-floods.md`. Promotion explicitly
rechecks the existing structural gates and scans the same terrain ledger before
an in-place merge. The bounded original-shallow list adds at most 15 cells and
225 membership comparisons. No search, retry, cache, rendering loop, gameplay
randomness or terrain stack is added. Test setup clones existing fixture worlds;
that is test overhead, not new runtime work. These are source-derived bounds,
not benchmark measurements.
