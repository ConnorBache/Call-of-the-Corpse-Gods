# Revision 44 admission and persistence

Desktop new games and debug starts explicitly choose simulation revision 44; loaded runs retain their recorded version. Core historical convenience constructors keep their old defaults. Revision 44 inherits the fixed revision 42 birth/pulse profiles and revision 43 item, gas, Fury and jewelry behavior; this plumbing does not alter their data or retune existing actors. Bleed Them changes are a separate mechanics change and test suite.

New exact header pairs are `corpse-r44-bleed-v1` (command receipts) and `corpse-r44-bleed-metadata-v2` (ordered operations). Every older identifier remains unchanged. Clean metadata recordings for revisions 41–44 may resume without checkpoint migration or mechanics upgrade. Unknown pairs still fail before execution. Framing, limits, locks, synchronization, operation ordering and ACK semantics are unchanged.

## Complexity and validation

The admission whitelist is a fixed O(1) check per creation/load/replay boundary. Exact header matching is O(identifier bytes), bounded by existing parser limits; no world or inventory traversal is introduced. Inherited fixed profile lookup remains O(R), R=91, once per new actor after the one-time catalog parse. Resume still costs O(B + replay) at load and O(B) exact verification once at attachment, then bounded attachment checks per append. No per-turn allocation or new retained cache is introduced; there are no new measured-performance claims.

Focused tests cover old/current load preservation, birth/pulse inheritance, native seed replay/restart, exact header/revision rejection, and clean 41–44 resume with ordered metadata append. Existing unsupported-future fixtures move only their rejected revision from 44 to 45. Run `tools/cargo.sh test --workspace` for the native gate; independent mechanics and platform checks remain required.
