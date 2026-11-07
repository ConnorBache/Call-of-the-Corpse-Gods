# Revision 39 Shard weapon UI

Revision 39 removes the Shard's separate `Use on…` target action. It retains the
ordinary main-hand, offhand and unequip commands and exposes ordinary `Throw`.
Throwing follows the same keyboard/mouse targeting, cancel, ray-preview and
command-dispatch path as other thrown inventory items. The preview is an
observation of geometry, not a promise to hit or a simulation-RNG roll.

Explicit revisions 33–38 retain their historical Shard target action and lack
of Throw. The core rejects a stale revision-39 `UseAt` request; desktop action
availability is not the sole enforcement boundary.

The revision-39 description contains only the item name, equipment state and
the designer-authored Shard description. The old appended instruction promising
an accuracy-free targeted use is not shown for revision 39. No new player-facing
wording is added by this UI change. Historical-only instruction text retains its
original provenance and is not falsely marked as rewritten by the designer.

Revision 39 permits the Shard's weapon Throw while Dreamwalking. The UI uses
the core's narrow Shard exception; ordinary consumable throws remain blocked.
Normal equipment management remains available, and targeting follows the active
dreamwalking self.

## Complexity

The revision gate and fixed action selection are O(1) time and space per item
projection. Description formatting is O(L) time and memory for L output bytes.
These changes do not alter Pack grouping's existing O(I² + I*A + G log G + T)
bound or its O(I + T) projection memory (I inventory records, A ability catalog,
G display groups, T tooltip text). Throw targeting continues to use the shared
knowledge-limited ray preview, including its bounded linked-space search;
rendering/cache-miss costs are unchanged from `targeting-ui-complexity.md`.
No new per-frame simulation mutation, lookup cache, sort or gameplay RNG call
is introduced.

Tests cover revision gating, exact authored description, click and keyboard
throw dispatch, repeated cancel/retarget purity, either-hand commands, refresh
after equipment/removal, read-only projectile preview, dream-plane weapon targeting, and preserved
ordinary-consumable restrictions. These are native UI model tests, not a claim of Windows runtime
verification.

Tagged dropped Shards are hidden from waking item render/inspection, including
remembered and detected snapshots. The shared item-visibility predicate preserves
the quest-shard and dream Half Here exceptions. Dream views render tagged weapons
as dream entities (not blue waking-world ink); ordinary waking items remain
visible with the authored blue override. The extra item tag checks are O(1) per
existing item projection; no new scans or simulation mutation are introduced.
