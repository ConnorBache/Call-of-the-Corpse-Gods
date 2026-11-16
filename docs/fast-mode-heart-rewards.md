# Fast mode heart rewards

Fast mode doubles the blessing amount and the stats a heart would actually award under existing rounding. For each of the eight bonus fields, the effective Fast reward is `2 * combat::integer_stat(base bonus)`; normal mode keeps the original base bonus. Both modes use the existing `equipment::add_bonus`, including its existing rounding of the player's current stat totals. For example, a hypothetical +0.625 bonus awards +1 normally and +2 in Fast mode, not +1.25. Blessings remain `2 * base exp` with no added rounding. Missing/null bonus fields retain their original shape.

The eight fields are Offense, Defense, Accuracy, Dodge, DR, Perception, maximum Energy, and maximum HP. A source audit of all 138 templates in `crates/corpse-core/catalog/heart-items-v48.json` found zero fractional bonus values. Source review also verified integer new-creature sensory rewards, depth/ceiling heart rewards, and Epidite's existing doubling. The hypothetical fraction tests protect the existing rounding contract without inventing fractional-add mechanics.

Items and stacks retain their original serialized rewards through save/load. Effective conversion is a pure read used by consumption and display. Epidite still doubles the stored base stats, blessings, and energy once; Fast mode then doubles its awarded stats and blessings once. For authored integer rewards this gives four times the original stats/blessings and twice the original energy. That energy increase comes only from Epidite, never from Fast mode.

Energy restoration is otherwise unchanged. Hearts eaten count once per consumed unit. Adaptive Armor's species tracking and DR gain are unchanged. Equipment enhancements, enemy stats, drops, and other abilities receive no compensation. Pack and ground-item inspection use the shared heart description, showing effective awarded stat and blessing rewards with unchanged energy. Observe's ground entities show names/glyphs rather than numerical rewards.

Direct reward paths audited: ordinary consumption is the authoritative stat/blessing application. Legacy playtest reconstruction and debug-floor preparation consume via that same path, but their constructors currently have no Fast selector, so their normal-only reward accounting is unchanged. Heart generation and storage continue to author base amounts, not mode-scaled amounts.

## Complexity

Effective reward conversion visits a fixed eight-field schema: O(1) worst-case time and extra space per consumption or numerical preview, with no RNG, actor, inventory, or world traversal. It clones only the eight optional numerical fields, not the Item or strings. Existing stat addition visits those same eight fields. Formatting allocates output proportional to its string length; existing pack/inspection enumeration and rendering costs are unchanged. No caches are introduced.

## Verification

Inline tests are registered through existing heart_items and pack modules; no lib.rs hook is needed. Coverage includes all eight fractional base bonuses rounded before doubling, ordinary rounding of fractional player totals, missing/null fields, normal mode compatibility, unchanged energy, unchanged Adaptive Armor species count, one heart count per use, stacked consumption after serialization, Epidite composition, and truthful numerical descriptions. Save roundtrip here exercises typed serialization/deserialization; strict save admission is covered by the owning Fast mode suite. Workspace/test compilation and all four focused heart-reward tests passed. The owning core Fast mode suite passed 13 cases, including strict sparse save and temporal admission. Desktop numerical-preview tests compile but have not yet been executed; native interaction and measured timings remain unverified.
