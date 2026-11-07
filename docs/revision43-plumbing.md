# Revision 43 admission and persistence

New desktop games and debug starts explicitly select simulation revision 43. Core historical convenience constructors retain their prior defaults. Loaded runs keep their recorded revision; no actor retuning occurs during load.

Revision 43 inherits revision 42's fixed birth profiles, child-context overrides and early Ruin packets. These guard-only extensions do not change catalogs, RNG consumption, rewards or prior revisions. The new item/action mechanics are specified and tested separately.

Exact recording identities are `corpse-r43-item-action-v1` (command receipts) and `corpse-r43-item-action-metadata-v2` (ordered metadata/command operations). Existing identities for revisions 39–42 remain unchanged. Mismatched build/revision pairs fail before checkpoint execution. Clean metadata recordings for 41, 42 and 43 can resume in place; older/torn/plain records retain conflict guards and checkpoint migration behavior. Metadata has no seed-replay command index, turn or RNG effect. No wire grammar, byte limit, lock, synchronization or acknowledgement behavior changes.

## Complexity

Admission runs at construction/load/replay creation and performs a fixed finite whitelist check: O(1) time and memory. Header selection and exact pair matching are O(L) in identifier bytes, bounded by the existing frame parser; one new constant identifier pair does not change limits. Existing resume validation remains O(B + replay execution) on load and O(B) exact verification once under the writer lock, with bounded per-append checks afterward. Fixed birth-profile lookup remains O(R), R = 91 rows, once per actor birth; cached catalog parse O(catalog bytes) once. No new per-turn traversal or cache is introduced. No new timing or platform-verification claim is made by this plumbing change.

Tests cover exact pair rejection, old 41/42 continuation, 43 resume/metadata append, native load preservation, inherited birth/pulse values, seed replay/restart and explicit unsupported revisions. Run the native workspace suite through `tools/cargo.sh test --workspace`; mechanic-specific tests remain independently required.
