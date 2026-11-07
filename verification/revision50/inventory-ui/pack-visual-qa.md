# R50 Pack visual QA

Reviewed all 17 PNGs in `items/` by opening their actual pixels at 1280×800. This is deterministic native test-render evidence, not a live SDL interaction or Windows platform certification. No Rust source was edited and no Cargo command was run during this review.

## Confirmed

- `five-carried-categories.png`: five clearly separated sections in Trinkets, Amulets, Rings, Potions, Hearts order. Header and row counters agree: 1 unlimited trinket stack, 1/5 amulets, 1/5 rings, 2/5 potions, 1/5 hearts. Arrows ×500 consume one trinket stack. All five sections fit in the default viewport. No global capacity total or phantom twenty-slot grid.
- `amulet-full.png`, `amulet-spent-appropriate.png`: carried amulet is marked active and has Apply, Drop, Throw only. Energy/cost updates from 50/50 and 25 to 25/50 and 15. Appropriate raises the displayed amulet/ring limits from 5 to 8, while potions/hearts stay at 5. No Wear or Take off controls.
- `light-duplicates.png`, `ice-immunity.png`, `vampirism.png`: active jewelry labels, current names/effects, and disabled reorder arrows at category boundaries are legible. Two grouped Light rings still occupy two ring slots; all six carried rings are counted against the Tools of the Trade limit of ten.
- `long-ring-top.png`, `long-ring-bottom.png`: long descriptions remain inside the detail pane; scrolling reaches the final grant text and Drop/Throw controls. No text spills outside the modal.
- `freezing-pack-hover.png`, `freezing-ground-hover.png`, `granted-ring-pack-hover.png`: effect text is readable before selection. The long ring tooltip stays inside the modal and its final line is visible. See the copy defect below.
- `amulet-target-preview.png`, `amulet-cancel-restored.png`: Apply hides the Pack and draws the targeting line. Cancellation removes the line and restores the base view. The passing capture test also checks exact pixel restoration, repeated cancellation and unchanged Run serialization.
- Four `statuses-*` captures show the expected full/half meter lengths and remaining-turn labels on both pages. No new issue observed.

## Defect and staged fix

**Stale Tinker grant wording:** `long-ring-top.png`, `long-ring-bottom.png` and `granted-ring-pack-hover.png` still say “While equipped: grants …”. This contradicts their correct “Active while carried” label. The source also uses “When equipped” for the bonus-stat line when a jewelry item has stored bonuses.

`pack-carried-grants-copy.patch` is staged separately, not applied. It changes only R50 ring/amulet grant and bonus labels to “While carried”, preserves historical copy, and adds `carried_jewelry_grant_and_bonus_copy_preserves_legacy`. After applying, rerun the Pack tests and regenerate/review the three affected captures.

## Test evidence and limits

`focused-desktop.log` confirms all Pack and jewelry/item capture tests passed for the reviewed source. The complete desktop run ended with **587 passed, 1 failed, 32 ignored**. Its unrelated failing assertion is `debug_floor_ui::tests::r50_debug_e_carries_active_jewelry_without_capacity_bypass`: empty categories format as `-0/5` instead of `0/5`. The Pack captures show ordinary zero correctly. This failure is recorded in the complete desktop test log.

The staged copy fix has not been compiled or tested. These screenshots do not cover other resolutions, every typography override, or a live Windows session.
