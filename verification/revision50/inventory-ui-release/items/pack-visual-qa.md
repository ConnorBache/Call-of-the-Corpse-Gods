# Final R50 Pack targeted visual QA

Opened the actual regenerated 1280×800 pixels for `long-ring-top.png`, `long-ring-bottom.png`, and `granted-ring-pack-hover.png` in this release directory.

**Result: the previously reported grant-copy defect is resolved.**

- The selected ring's Execute grant now says “While carried: grants Execute.”
- Scrolling to the bottom shows “While carried” for Appropriate and Implant Thought, with the Drop/Throw buttons fully accessible.
- The unselected ring hover uses “While carried” for every grant. Its final line remains visible inside the modal. No tooltip spill or new clipping defect was observed.
- Active jewelry labels, five category headings, per-category counts/limits and category-bound reorder arrows remain intact in the reviewed frames. No Wear/Take off action appears.

The earlier full 17-image review is recorded in `../../inventory-ui/pack-visual-qa.md`. This final pass specifically rechecks the three captures affected by the wording fix; it does not claim a new live SDL or Windows interaction test.

## Regression correction

The workspace run's sole failure was the newly added `carried_jewelry_grant_and_bonus_copy_preserves_legacy` test expecting the R50 name Execute at revision 49. The actual legacy catalog contains `shrail-unyielding-assault` / “Unyielding Assault”; `shrail-execute` / “Execute” exists only in the R50 catalog.

Corrected only the test's legacy fixture identity and exact expected name in `crates/corpse-desktop/src/pack.rs`. All R50 carried behavior assertions remain unchanged. No production source changes or Cargo commands were made during this review. Source is frozen; rerunning the corrected test is the release verification's next step.

Frozen `pack.rs` SHA-256 after the test-only correction:
`3a99d1022f150e0f2fdd08643cf2caebd31d70bb8c4ef0c217bf135be0005ba8`
