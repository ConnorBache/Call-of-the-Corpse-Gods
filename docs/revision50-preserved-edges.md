# Revision 50 preserved blessing edges

Current release integration supersedes checkpoint-era gate/toolchain statements below.
R50 runtime admission, item/resource rules and generated acceptance are integrated;
see [implementation status](revision50-implementation-status.md) and
[native verification](revision50-native-verification.md) for current results and limits.
Earlier audit rows and unexecuted-test remarks are development history, not current
unsupported-feature claims.

Authority: the Sparkle and All-Seeing Eyes entries in [the Revision 50 request](revision50-request.txt). These are revision-gated changes to existing execution paths, not new ability identities or changes to historical saves.

## Executable changes

- **Sparkle** keeps the stable `anavani-illuminate` identity and its 20-energy, no-action activation. Revision >=50 accepts every visible in-bounds integer square, including occupied squares, walls, water, chasms and pits. Invalid, unseen and unfunded targets still cannot place a source. Historical revisions keep their old terrain filter.
- Placing the permanent source also ignites it immediately. Ordinary `ignite` still rejects noncombustible terrain, so `place_sparkle` alone ensures the new permanent brazier and player provenance after ordinary ignition. It preserves the underlying terrain except the pre-existing grass/tree burning and snow-melting effects. Existing permanent-source upkeep and light strength/falloff are reused. Ordinary fire rules are not relaxed.
- **All-Seeing Eyes** still costs 30 energy, reveals the current floor and is itself a no-action activation. For >=50, its lifetime now ends after the next successful action finishes, including a haste move, I Came For You move, Relentless Pursuit credit move or backward Weave of Time move. `DispatchOutcome.took_turn` continues to mean world-time advancement; it is not repurposed as action completion.
- The action boundary follows the established dispatcher: Tools of the Trade potions/hearts and ordinary no-action ability activations/toggles preserve the reveal. Rally, Ally orders, Pickup, invalid commands and blocked actions also preserve it. A regular consumable, Wait, attack, successful ordinary move or action-taking ability ends it. Finishing observation/remembering runs before expiry as before. Historical revisions retain their world-turn expiry.
- Spellslinger's eventual free amulet activation must follow the same no-action branch, and its ordinary paid action must follow action completion. The existing isolated amulet accounting module exposes that distinction, but authored amulet effects and playable integration remain unresolved. This change does not claim live amulet execution or test it.

## Temporal, plane and rendering boundaries

No new persisted state is introduced. Existing `allSeeingReveal` and `allSeeingDepth` conditions remain the authority. Snapshot capture, raw JSON save/restore and See The Future capture/restore retain those exact values, so returning to a revealed anchor restores the original pending expiry. Future activation/return is a no-action temporal command; it does not immediately consume the restored reveal. Revision >=41 still does not retain obsolete Revert history.

Sparkle runs inside the existing player-plane context, so a dream-plane source cannot change the waking floor's fire or lights. Existing renderer/visibility projection observes the same fire, light and reveal fields; there are no new per-frame queries, animation clocks, random draws or off-screen global scans. Fire on the new terrain types uses existing permanent-source simulation on active and off-screen floors. Off-screen return and platform rendering remain unverified combinations, not inferred test results.

## Deterministic verification status

`crates/corpse-core/tests/revision50_preserved_edges.rs` adds focused cases for:

- Every terrain type, occupied targeting, immediate fire/light/provenance, repeat-placement deduplication and upkeep persistence.
- Legacy target rejection, unchanged ordinary ignition, invalid/unseen/unfunded targets and dream-plane isolation.
- No-action consumption/toggle/administrative commands, failed moves, normal consumption and action abilities.
- All four successful bonus-movement branches, historical haste behavior, visible creature reveal, final-observation item/terrain memory and serialized Snapshot restoration.
- Raw canonical save plus Future anchor capture/restore followed by one-shot action expiry.

The full Engine load/replay/Future integration case is explicitly ignored while Revision 50 release admission remains disabled. It must be enabled and run when the integration gate is lifted. The other tests exercise explicit >=50 fixtures without changing that gate. Focused and workspace invocations via `sh tools/cargo.sh` both returned `cargo: not found`; the script itself is not executable in the recovered archive. No test has been run successfully in this environment, and no installation was attempted. Native compilation, reference parity and Windows behavior are not claimed.

## Complexity

Let V be active-floor cells, P permanent-fire entries, L explicit light sources, C player condition entries, and T_visibility the existing visibility/optical/lighting query cost including cache misses. These variables describe stored input sizes; admitted floor geometry currently has 2291, 2500 or 3600 cells. Valid permanent-fire indices and placed light coordinates are deduplicated, so this operation contributes at most V of each. Imported list multiplicity remains an independent input bound.

Sparkle target admission is O(T_visibility) time plus O(1) bounds/revision checks, with the visibility subsystem's existing scratch. Placement scans O(P+L), normalizes/initializes the fixed number of environment/provenance columns in O(V), and writes one cell in O(1). The >=50 completion performs another environment admission, also O(V), because ordinary ignition may have returned before initialization. Overall placement is O(P+L+V) time plus the existing visibility query, floor COW detachment and any existing root-burning helper. Environment initialization adds O(V) memory on first use and O(1) retained source records per new square; initialized state may still perform O(V) normalization work. A shared floor's first write copies its whole payload, not merely the touched cell. Existing environment propagation, gas reaction closure and off-screen scheduling keep their documented costs; none is counted as constant here.

The new action-completion routing costs O(1) time/scratch per dispatched command. Removing two ordered IndexMap conditions costs O(C) worst-case time and O(1) extra memory per successful expiry. Existing `finish` still pays its equipment, actor, pickup, visibility and memory costs; the extra route invokes it once, not a second time. No new cache, snapshot, floor scan, rendering work or serialized bytes are added. Temporal capture/export/restore retains the existing whole-state and COW costs, with only the already-existing two condition entries.

These are source-derived, unmeasured bounds. Run `tools/cargo.sh test -p corpse-core --test revision50_preserved_edges`, then `tools/cargo.sh test --workspace` when the pinned toolchain is available. After release admission, run the ignored replay case explicitly. Profile repeated Sparkle on maximum-sized floors with cold/warm lighting and environment state before claiming speed or memory improvements.
