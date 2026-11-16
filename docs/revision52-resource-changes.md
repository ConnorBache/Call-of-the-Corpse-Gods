# Revision 52 player resource turn changes

Health, main Energy, Blessings, displayed ability reserves, and each carried physical
amulet reserve show their signed net actual change since the existing resource
turn opening, as a percentage of their current maximum. For example, healing
from 191.9 to 202 of 202 HP shows `202/202 - +5%`. Overhealing is excluded.
Damage followed by healing is netted; this is not gross damage or gross healing.
Zero change or an unknown baseline has no suffix. Changes below one percent
keep a decimal; positive/negative changes below 0.1% show their sign and `<0.1%`.
The Boundless Hunger main energy bar retains its existing 100-energy reference.

No gameplay, RNG, turn, payment, or time-travel rules change. Normal/Haste/Fast
actions retain the existing resource-turn opening. Free actions update within
that baseline; rejected actions preserve it. Future anchors and snapshots retain
the saved history including reserve entries. Legacy saves without reserve keys
adopt the first observed amount, so acquiring a new meter does not invent a gain.
R52 adds `ability:<canonical ID>` and `amulet:<physical item ID>` entries to the
existing serialized player meter map, without new required model fields. Save
validation is extended only for the R52 player: ability IDs must be canonical
catalog/copied-trait identities of at most 128 ASCII bytes; physical amulet IDs
must be canonical nonnegative safe-integer decimal strings (at most 16 digits).
Arbitrary labels, malformed IDs, non-player reserve entries, and these new keys
in pre-R52 saves are rejected. Removed/dormant meters remain valid until the
next turn; historical keys need not still belong to a carried item. Earlier
revisions keep their old history and have no resource-change suffix.

Detached view strings participate in both retained and tile HUD invalidation;
pack refresh includes the player meter history. Thus reaching full/zero and
resetting percentages without a numeric value change repaint correctly. Values
are right-aligned in the existing HUD row and shrink from 13 to 10 logical pixels
only if needed. Extreme totals use a leading ellipsis while preserving the
signed change; changes at or above 100,000% use scientific notation. No additional row, gameplay input, or hidden-world query is added.

## Complexity

Let A be observed actors, C visibility-projection setup, Q an actor visibility
query, M ordinary effective-stat meter projection (including inventory-derived
stats), H retained actor histories, B player reserve keys, I carried items, and
L total reserve/item-key text bytes. Existing resource observations cost
O(C + A*(Q+M) + H*A), with visibility projection storage P. New R52 player-meter
construction adds O(B + I + L) expected hash work, O((B+I)^2 + L*(B+I)) worst-case
hash collision work, and O(B+I+L) entries/key bytes. It runs at existing begin/
observe calls, not for individual record-loss calls or at paint time. Existing
map retention still scans histories against observed IDs. Maps reset each turn;
within a turn they additionally retain baselines for meters acquired then removed.
No new fixed gameplay cap is introduced; the admitted inventory/reserve sizes and
existing save budget bound stored data. Serialization, snapshot cloning and pack
stamp comparison include these additional entries and key bytes. Amulet item
prose checks ownership in O(I); ground/remembered items do not claim a current
player change. Existing pack projection already includes item scans.

Percentage projection is O(key bytes) expected hash work per visible meter,
O(H*key bytes) worst-case collisions, plus existing effective max-energy stats.
HUD paints three bounded strings with at most four O(G) font-metric scans, G text
characters. Extra transient/output text is O(G). Cold native glyph rasterization,
clipped pixel blending, framebuffer uploads and existing ability/pack layout
costs remain as documented in native rendering. Retained/tile equality compares
three detached strings; ability costs use their existing string equality. Pack
stamps add O(B+I+L) copy/compare/storage. The new boundary-only key validation adds O(K*A*L) for K reserve keys, A catalog
entries, L bounded ability-ID bytes; amulet ID parsing is bounded to 16 bytes.
Existing base serialization/validation costs still apply in addition to this
key-admission work.
These are asymptotic bounds, not measured
frame timings.

## Verification

Deterministic regressions cover capped healing, mixed net changes, death/zero,
current-capacity denominator, bounded and unbounded energy, reserve split costs
and refill, independent amulet IDs, missing legacy entries, free commands, Haste,
rejected movement, raw and Engine save/load, and Future restoration. Native CPU
render regressions compare full, retained and tile frames at 1280x800 and portrait
960x1200, including baseline-only updates at full/zero. Set
`CORPSE_RESOURCE_REVIEW` to export the native captures. This does not claim
physical Windows display verification.
