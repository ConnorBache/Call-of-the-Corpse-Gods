# Enemy metadata and legacy trait initialization

The native catalog freezes 65 enemy ability definitions and 59 bestiary/event definitions from v48. It includes authored bleed flags, costs, names and conservative historical glyph/color aliases. No randomized or inferred replacement lore is introduced.

`enemy_traits` matches 602 source cases covering intrinsic inference, explicit erased-trait markers, cost queries, legacy presentation normalization and idempotent initialization. Initialized actors never reacquire erased powers. Detonation cost reads current energy. Half Here presentation delegates the shared-body helper.

Complexity: with D definitions, A actor traits, C catalog abilities and L name length, name lookup is O(D L); inference is O(D L + A), and cost queries O(A) expected with insertion-ordered hash metadata. Initialization allocates an O(A) trait list only where source does. Normalization adds bounded legacy palette/glyph scans. These are per initialization/query, not independent whole-world scans. Catalog JSON is parsed once; subsequent queries borrow static metadata. Hash maps do not decide simulation order.

Important semantic distinction: some source modules locally name a raw `abilities.includes` query `hasEnemyAbility`; other modules use enemy-data inference. These must not be globally interchanged. Initialization occurs only at authored special-action/environment boundaries. Capability admission checks inferred traits conservatively before accepting execution.

Verification: `node tools/export-enemy-trait-catalog.mjs`; `node tools/export-enemy-traits.mjs`; `tools/cargo.sh test -p corpse-core --test enemy_traits`. This metadata coverage does not assert every enemy power executes.
