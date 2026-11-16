# Revision 52 amulet tier presentation

## Authority and boundary

The approved naming format is `Tier X Amulet of Y`. New item births acquire the
floor's tier; an existing item's tier and energy are never recalculated from
the viewed or loaded floor. Tier 1 preserves the base behavior. Revisions 50
and 51 retain their names, effects and resource text.

The desktop reads `revision52_amulet_tiers::profile`, `description` and `details`.
Canonical names flow through `revision50_item_knowledge::display_name` to
the same tier module. Cost and reserve capacity come from
`revision50_amulet_effects::item_spec`. There are no desktop damage,
range, duration or tier schedules.

Pack descriptions, item tooltips, visible/remembered ground inspection, and
map item labels use these shared projections. DEBUG rows use the actual Item
and show its current energy. DEBUG reconstructs prior-floor loot rather than
creating a selected-floor item template. Changing its target floor does not
upgrade the recovered items. The desktop's Manual is creature-only, so this
change does not add an unsolicited amulet catalog or world text. The selected
jewelry pane omits an initial description line only when it exactly equals
the title already painted above it. Standalone tooltip and ground descriptions
retain their complete title and effect text; this presentation cleanup applies
to old-revision jewelry as well.

## Instance ownership and order

R52 amulet grouping retains the physical item ID as well as typed tier and
reserve fields. Even identical same-tier items or old missing-state items
remain independent rows. The selected ID, Apply/Throw target ID, manual display
order and the action receipt continue to identify the same physical item.
There is no forced tier sort that would undo the user's existing row ordering.
Refreshing spent energy, canceling targeting and changing canvas size retain
the selected instance. Only removal of the selected item clears it.

## Targeting and observation

`PackTarget` continues to call the core's known-only `target_blocker_ui` and
`aim_preview`; the desktop never recreates collision or tier range rules.
Thus tier ranges use the same profile as actual activation. Iceblast's first
blocker, Blink's last legal empty line cell, Lightning piercing, free known
no-forward rejection and hidden-blocker boundaries remain core-owned.
Painting consumes detached results, including actual animation receipt IDs.

Both retained render paths use a shared interaction footprint. It expands a
logical tile by two logical pixels per side, then by one native pixel per side
after conversion and clipping. This covers rounded corner strokes and the
rasterizer's minimum one-native-pixel rectangles at fractional scales. Removing
a target restores the whole old footprint rather than leaving a boundary
stroke. This changes redraw bounds only, not targeting or simulation.

## Complexity

Let I be physical inventory records, G displayed groups, S the total item-field
payload and T projected text bytes. A shared profile lookup is O(E) in effect
name bytes with constant additional storage; the effect and tier catalogs are
fixed. Description and name construction add O(T) output work/storage.

Pack grouping remains pairwise typed comparison: O(I² times compared item
payload) plus item cloning O(S), existing descriptions/actions and manual-order
rank scans. Ranking scans each group's members against the order/inventory
lists, O(I²); final group sorting is O(G log G). Retaining amulet IDs often
rejects equality earlier, but no speedup is claimed. Existing effective-stat,
ability, tooltip wrapping and render work remains additional as documented in
the Pack ledger. There is no new persistent or frame-polled tier cache.

DEBUG text projection is O(item/name text bytes) per displayed row; at most 13
rows per pane are painted, although its existing item-reference collection
still scans the complete stash/inventory. No RNG, generated IDs, pathfinding,
visibility or charge mutation occurs in this helper.

For K old/new interaction cells, the shared footprint calculation takes O(1)
work per cell, O(K) total work and at most O(K) queued rectangle storage on an
interaction change. The fixed logical/native padding is clipped to the canvas.
Subsequent span construction, overlap merging, clipped rasterization and uploads
still have the renderer's existing area/candidate-dependent costs; O(K) is not
a claim about the whole frame. No full-frame repaint, pixel-diff discovery or
new persistent cache was introduced. Performance has not been benchmarked.

## Verified scope

The eight `r52_amulet_ui_` tests passed in
`verification/r52-tier-ui-03` with unchanged source hashes before and after the
run. They cover all 12 effects at tiers 1, 5 and 9 against shared copy, unchanged
cost/capacity, Appropriate, physical-instance grouping/manual ordering,
selected-ID persistence, current and legacy save/load, missing-tier deep-floor
presentation, DEBUG recovered-loot birth tiers, visible and remembered ground
knowledge, utility range admission, mouse/keyboard equivalence, first collision,
Lightning piercing, hidden blockers, actual reserve/animation identity and free
no-forward rejection. The fixture retains empty actor/ground blocks for every
stored floor to satisfy production save admission.

Native CPU full, retained and tile paths are compared pixel-for-pixel at
1280×800 and 960×1200, including targeting and cancellation. Eight captured
images in `verification/r52-tier-ui-03/captures` were visually inspected: Pack,
Lightning aiming, Blink aiming and cancellation in both orientations. The item
title appears once in the selected pane, tier/reserve/cost text fits, and the
canceled target leaves no trail. Screenshots use `CORPSE_R52_AMULET_REVIEW`.

The first two focused attempts retained their failing evidence. They exposed
eight stale portrait target pixels after cancellation. The shared interaction
footprint fixes the invalidation gap without relaxing the pixel oracle. A
separate regression verifies repeated aim/cancel at 1280×800, 960×1200,
719×1201, 320×200 and 1537×911. It also paints actual rounded marks at every
quarter-pixel phase at scale 0.75, proves that overhang occurs on both axes and
asserts that every such pixel lies inside the shared footprint. The local
redraw remains smaller than the canvas after each interaction change.

Warm regressions in `verification/r52-tier-ui-legacy-01` passed with unchanged
source hashes: legacy jewelry 5, item knowledge 7, aiming 6, tile rendering 15,
and retained rendering 17. The retained filter also repeats the tier
full/retained/tile oracle. Three existing opt-in tests were ignored: the software
paint/upload benchmark, SDL texture readback oracle and Future timing test.
All checks used pinned Rust 1.99.1, test opt-level 1, debug info off, debug
assertions and overflow checks on, incremental compilation off and two jobs.

These are native UI handler/render tests, not a live SDL event-loop,
GPU-driver, Windows or realistic-gameplay claim. They do not constitute a new
full-workspace pass. Platform smoke tests, explicit ignored tests and later
integrated-workspace gates remain separate verification steps.
